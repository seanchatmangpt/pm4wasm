// Drift Server – Real-time Process Drift Detection SaaS
// Copyright (C) 2024 Process Intelligence Solutions
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! Real-time process drift detection server with WebSocket ingestion.
//!
//! ## Features
//!
//! - **WebSocket Ingestion**: 10K concurrent connections with backpressure handling
//! - **EWMA Smoothing**: Configurable exponential weighted moving averages
//! - **SPC Drift Detection**: Western Electric rules for statistical process control
//! - **Alert Integrations**: Slack, Jira, PagerDuty, email webhooks
//! - **Prometheus Metrics**: Built-in observability
//!
//! ## Quick Start
//!
//! ```bash
//! cargo run --release
//! ```
//!
//! ## WebSocket Protocol
//!
//! ### Connect
//! ```text
//! ws://localhost:8080/ws/{tenant_id}/{process_id}
//! ```
//!
//! ### Message Format (Client → Server)
//! ```json
//! {
//!   "type": "trace",
//!   "case_id": "case-123",
//!   "events": [
//!     {"name": "A", "timestamp": "2024-01-01T10:00:00Z"},
//!     {"name": "B", "timestamp": "2024-01-01T10:01:00Z"}
//!   ]
//! }
//! ```
//!
//! ### Message Format (Server → Client)
//! ```json
//! {
//!   "type": "snapshot",
//!   "fitness": 0.95,
//!   "perfect_rate": 0.8,
//!   "alerts": [...]
//! }
//! ```

use anyhow::Result;

use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        Path, State,
    },
    response::{IntoResponse, Response},
    routing::get,
    Router,
    Json,
};
use axum::http::StatusCode;
use chrono::Utc;
use prometheus::{Counter, Histogram, IntGauge, Registry, TextEncoder};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    sync::Arc,
    time::Instant,
};
use tokio::sync::{mpsc, RwLock};
use tracing::{debug, error, info, instrument, warn};

mod alerting;
mod drift;
mod metrics;
mod telemetry;

use alerting::AlertBroadcaster;
use drift::{DriftDetector, DriftSnapshot, TraceEvent};
use metrics::ServerMetrics;
use telemetry::setup_telemetry;

/// Global server state.
#[derive(Clone)]
struct ServerState {
    metrics: ServerMetrics,
    alert_broadcaster: Arc<AlertBroadcaster>,
    detector_store: Arc<RwLock<HashMap<String, Arc<RwLock<DriftDetector>>>>>,
}

/// Incoming WebSocket message from client.
#[derive(Debug, Deserialize)]
#[serde(tag = "type")]
enum ClientMessage {
    /// Submit a trace for drift detection.
    Trace { case_id: String, events: Vec<TraceEvent> },
    /// Configure drift detection thresholds.
    Configure { config: drift::DetectorConfig },
    /// Heartbeat to keep connection alive.
    Ping,
}

/// Outgoing WebSocket message to client.
#[derive(Debug, Serialize)]
#[serde(tag = "type")]
enum ServerMessage {
    /// Current drift snapshot.
    Snapshot { data: DriftSnapshot },
    /// Alert fired.
    Alert { alert: alerting::DriftAlert },
    /// Error message.
    Error { message: String },
    /// Pong response.
    Pong,
}

/// Build the application router.
fn router() -> Router {
    let state = ServerState {
        metrics: ServerMetrics::new(),
        alert_broadcaster: Arc::new(AlertBroadcaster::new()),
        detector_store: Arc::new(RwLock::new(HashMap::new())),
    };

    Router::new()
        .route("/health", get(health_check))
        .route("/metrics", get(metrics_export))
        .route("/ws/:tenant_id/:process_id", get(websocket_handler))
        .route("/api/v1/configure/:process_id", get(configure_detector))
        .with_state(state)
}

/// Health check endpoint.
async fn health_check() -> &'static str {
    "OK"
}

/// Prometheus metrics export endpoint.
async fn metrics_export(State(state): State<ServerState>) -> Response {
    let metrics = state.metrics.registry();
    let encoder = prometheus::TextEncoder::new();
    let metric_families = metrics.gather();
    let mut buffer = String::new();
    encoder.encode_utf8(&metric_families, &mut buffer).unwrap();
    Response::builder()
        .header("content-type", "application/openmetrics-text")
        .body(axum::body::Body::from(buffer))
        .unwrap()
}

/// Configure drift detection for a process.
#[instrument(skip(state))]
async fn configure_detector(
    State(state): State<ServerState>,
    Path(process_id): Path<String>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    // Return current or default configuration
    let store = state.detector_store.read().await;
    if let Some(detector) = store.get(&process_id) {
        let detector = detector.read().await;
        Ok(Json(serde_json::json!({
            "process_id": process_id,
            "config": detector.config(),
        })))
    } else {
        Ok(Json(serde_json::json!({
            "process_id": process_id,
            "config": drift::DetectorConfig::default(),
        })))
    }
}

/// WebSocket handler for event stream ingestion.
#[instrument(skip_all)]
async fn websocket_handler(
    ws: WebSocketUpgrade,
    State(state): State<ServerState>,
    Path((tenant_id, process_id)): Path<(String, String)>,
) -> Response {
    ws.on_upgrade(move |socket| handle_socket(socket, state, tenant_id, process_id))
}

/// Handle a WebSocket connection.
#[instrument(skip_all, fields(%tenant_id, %process_id))]
async fn handle_socket(
    mut socket: WebSocket,
    state: ServerState,
    tenant_id: String,
    process_id: String,
) {
    info!("WebSocket connection established");
    state.metrics.connections_active().inc();

    // Get or create drift detector for this process
    let key = format!("{}:{}", tenant_id, process_id);
    let detector = {
        let mut store = state.detector_store.write().await;
        if !store.contains_key(&key) {
            store.insert(
                key.clone(),
                Arc::new(RwLock::new(DriftDetector::new(process_id.clone()))),
            );
            state.metrics.detectors_created().inc();
        }
        store.get(&key).cloned().unwrap()
    };

    // Create channel for sending alerts back to client
    let (tx, mut rx) = mpsc::channel::<ServerMessage>(100);

    // Handle both sending and receiving in a single loop
    loop {
        tokio::select! {
            // Handle incoming messages from WebSocket
            result = socket.recv() => {
                match result {
                    Some(Ok(Message::Text(text))) => {
                        let start = Instant::now();
                        match parse_and_process(&text, &detector, &state, &tenant_id, &process_id, &tx).await {
                            Ok(_) => {
                                state.metrics.messages_processed().inc();
                                state.metrics
                                    .message_latency()
                                    .observe(start.elapsed().as_secs_f64());
                            }
                            Err(e) => {
                                warn!("Message processing error: {}", e);
                                state.metrics.messages_failed().inc();
                                let _ = tx
                                    .send(ServerMessage::Error {
                                        message: e.to_string(),
                                    })
                                    .await;
                            }
                        }
                    }
                    Some(Ok(Message::Close(_))) => {
                        info!("Client initiated close");
                        break;
                    }
                    Some(Err(e)) => {
                        error!("WebSocket error: {}", e);
                        break;
                    }
                    _ => {}
                }
            }
            // Handle outgoing messages to WebSocket
            msg = rx.recv() => {
                match msg {
                    Some(msg) => {
                        let json = serde_json::to_string(&msg).unwrap();
                        if socket.send(Message::Text(json.into())).await.is_err() {
                            break;
                        }
                    }
                    None => break,
                }
            }
        }
    }

    // Cleanup
    state.metrics.connections_active().dec();
    info!("WebSocket connection closed");
}

/// Parse and process a client message.
#[instrument(skip_all)]
async fn parse_and_process(
    text: &str,
    detector: &Arc<RwLock<DriftDetector>>,
    state: &ServerState,
    tenant_id: &str,
    process_id: &str,
    tx: &mpsc::Sender<ServerMessage>,
) -> Result<()> {
    let msg: ClientMessage = serde_json::from_str(text)
        .map_err(|e| anyhow::anyhow!("Failed to parse message: {}", e))?;

    match msg {
        ClientMessage::Trace { case_id, events } => {
            let snapshot = {
                let mut det = detector.write().await;
                det.process_trace(&case_id, &events)?
            };

            // Check for alerts
            if !snapshot.alerts.is_empty() {
                for alert in &snapshot.alerts {
                    // Convert to DriftAlert for client
                    let drift_alert: alerting::DriftAlert = alert.into();

                    // Send to client
                    let _ = tx
                        .send(ServerMessage::Alert {
                            alert: drift_alert.clone(),
                        })
                        .await;

                    // Broadcast to external integrations
                    state.alert_broadcaster.broadcast(
                        tenant_id,
                        process_id,
                        alert.clone(),
                    ).await;
                }
            }

            // Send snapshot
            let _ = tx
                .send(ServerMessage::Snapshot { data: snapshot })
                .await;
        }
        ClientMessage::Configure { config } => {
            let mut det = detector.write().await;
            det.update_config(config);
            info!("Detector configured");
        }
        ClientMessage::Ping => {
            let _ = tx.send(ServerMessage::Pong).await;
        }
    }

    Ok(())
}

/// Main entry point.
#[tokio::main]
async fn main() -> Result<()> {
    setup_telemetry();

    let app = router();

    let listener = tokio::net::TcpListener::bind("0.0.0.0:8080").await?;
    info!("Drift server listening on ws://0.0.0.0:8080");

    axum::serve(listener, app).await?;

    Ok(())
}
