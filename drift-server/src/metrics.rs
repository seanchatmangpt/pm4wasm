// Drift Server – Real-time Process Drift Detection SaaS
// Copyright (C) 2024 Process Intelligence Solutions
// Apache License 2.0

//! Prometheus metrics for the drift detection server.

use prometheus::{Counter, Histogram, IntGauge, Registry};

/// Server metrics exposed to Prometheus.
#[derive(Clone)]
pub struct ServerMetrics {
    registry: Registry,
    connections_active: IntGauge,
    connections_total: Counter,
    messages_processed: Counter,
    messages_failed: Counter,
    detectors_created: Counter,
    message_latency: Histogram,
}

impl ServerMetrics {
    pub fn new() -> Self {
        let registry = Registry::new();

        let connections_active = IntGauge::new(
            "drift_connections_active",
            "Currently active WebSocket connections"
        ).unwrap();
        let connections_total = Counter::new(
            "drift_connections_total",
            "Total WebSocket connections accepted"
        ).unwrap();
        let messages_processed = Counter::new(
            "drift_messages_processed_total",
            "Total messages processed"
        ).unwrap();
        let messages_failed = Counter::new(
            "drift_messages_failed_total",
            "Total messages that failed to process"
        ).unwrap();
        let detectors_created = Counter::new(
            "drift_detectors_created_total",
            "Total drift detectors created"
        ).unwrap();
        let message_latency = Histogram::with_opts(
            prometheus::HistogramOpts::new(
                "drift_message_latency_seconds",
                "Message processing latency"
            )
        ).unwrap();

        registry.register(Box::new(connections_active.clone())).unwrap();
        registry.register(Box::new(connections_total.clone())).unwrap();
        registry.register(Box::new(messages_processed.clone())).unwrap();
        registry.register(Box::new(messages_failed.clone())).unwrap();
        registry.register(Box::new(detectors_created.clone())).unwrap();
        registry.register(Box::new(message_latency.clone())).unwrap();

        Self {
            registry,
            connections_active,
            connections_total,
            messages_processed,
            messages_failed,
            detectors_created,
            message_latency,
        }
    }

    pub fn registry(&self) -> &Registry {
        &self.registry
    }

    pub fn connections_active(&self) -> &IntGauge {
        &self.connections_active
    }

    pub fn connections_total(&self) -> &Counter {
        &self.connections_total
    }

    pub fn messages_processed(&self) -> &Counter {
        &self.messages_processed
    }

    pub fn messages_failed(&self) -> &Counter {
        &self.messages_failed
    }

    pub fn detectors_created(&self) -> &Counter {
        &self.detectors_created
    }

    pub fn message_latency(&self) -> &Histogram {
        &self.message_latency
    }
}
