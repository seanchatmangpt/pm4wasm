// Drift Server – Real-time Process Drift Detection SaaS
// Copyright (C) 2024 Process Intelligence Solutions
// Apache License 2.0

//! Alert broadcasting to external integrations.
//!
//! Supports Slack webhooks, Jira API, PagerDuty, and email notifications.

use prometheus::Counter;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, warn};

use crate::drift::{Alert, AlertSeverity};

/// Alert broadcaster that forwards alerts to external integrations.
#[derive(Clone)]
pub struct AlertBroadcaster {
    integrations: Arc<RwLock<Vec<Box<dyn AlertIntegration>>>>,
    metrics: BroadcasterMetrics,
}

/// Metrics for alert broadcasting.
#[derive(Clone)]
pub struct BroadcasterMetrics {
    alerts_sent: Counter,
    alerts_failed: Counter,
}

impl BroadcasterMetrics {
    pub fn new() -> Self {
        Self {
            alerts_sent: Counter::new(
                "drift_alerts_sent_total",
                "Total alerts sent to integrations"
            ).unwrap(),
            alerts_failed: Counter::new(
                "drift_alerts_failed_total",
                "Total alerts that failed to send"
            ).unwrap(),
        }
    }
}

/// Alert integration trait.
#[async_trait::async_trait]
pub trait AlertIntegration: Send + Sync {
    async fn send_alert(&self, tenant_id: &str, process_id: &str, alert: &Alert) -> Result<(), anyhow::Error>;
    fn name(&self) -> &str;
}

/// Slack webhook integration.
pub struct SlackIntegration {
    client: reqwest::Client,
    webhook_url: String,
}

impl SlackIntegration {
    pub fn new(webhook_url: String) -> Self {
        Self {
            client: reqwest::Client::new(),
            webhook_url,
        }
    }
}

#[async_trait::async_trait]
impl AlertIntegration for SlackIntegration {
    async fn send_alert(&self, tenant_id: &str, process_id: &str, alert: &Alert) -> Result<(), anyhow::Error> {
        let payload = serde_json::json!({
            "text": format!("Drift Alert for {}/{}", tenant_id, process_id),
            "attachments": [{
                "color": color_for_severity(alert.severity()),
                "title": format!("Drift Detected: {}", format_alert_rule(alert)),
                "text": format_alert_details(alert),
                "ts": chrono::Utc::now().timestamp(),
            }]
        });

        self.client
            .post(&self.webhook_url)
            .json(&payload)
            .send()
            .await?;

        Ok(())
    }

    fn name(&self) -> &str {
        "slack"
    }
}

/// Jira API integration.
pub struct JiraIntegration {
    client: reqwest::Client,
    base_url: String,
    auth_token: String,
    project_key: String,
}

impl JiraIntegration {
    pub fn new(base_url: String, auth_token: String, project_key: String) -> Self {
        Self {
            client: reqwest::Client::new(),
            base_url,
            auth_token,
            project_key,
        }
    }
}

#[async_trait::async_trait]
impl AlertIntegration for JiraIntegration {
    async fn send_alert(&self, tenant_id: &str, process_id: &str, alert: &Alert) -> Result<(), anyhow::Error> {
        let payload = serde_json::json!({
            "fields": {
                "project": { "key": self.project_key },
                "summary": format!("Drift Alert: {}/{}", tenant_id, process_id),
                "description": format_alert_details(alert),
                "issuetype": { "name": "Bug" }
            }
        });

        self.client
            .post(format!("{}/rest/api/3/issue", self.base_url))
            .header("Authorization", format!("Bearer {}", self.auth_token))
            .json(&payload)
            .send()
            .await?;

        Ok(())
    }

    fn name(&self) -> &str {
        "jira"
    }
}

/// PagerDuty webhook integration.
pub struct PagerDutyIntegration {
    client: reqwest::Client,
    integration_key: String,
}

impl PagerDutyIntegration {
    pub fn new(integration_key: String) -> Self {
        Self {
            client: reqwest::Client::new(),
            integration_key,
        }
    }
}

#[async_trait::async_trait]
impl AlertIntegration for PagerDutyIntegration {
    async fn send_alert(&self, tenant_id: &str, process_id: &str, alert: &Alert) -> Result<(), anyhow::Error> {
        let payload = serde_json::json!({
            "routing_key": self.integration_key,
            "event_action": "trigger",
            "payload": {
                "summary": format!("Drift Alert: {}/{}", tenant_id, process_id),
                "severity": severity_to_pagerduty(alert.severity()),
                "source": "drift-server",
                "custom_details": format_alert_details(alert),
            }
        });

        self.client
            .post("https://events.pagerduty.com/v2/enqueue")
            .json(&payload)
            .send()
            .await?;

        Ok(())
    }

    fn name(&self) -> &str {
        "pagerduty"
    }
}

impl AlertBroadcaster {
    pub fn new() -> Self {
        Self {
            integrations: Arc::new(RwLock::new(Vec::new())),
            metrics: BroadcasterMetrics::new(),
        }
    }

    pub async fn add_integration(&self, integration: Box<dyn AlertIntegration>) {
        let mut integrations = self.integrations.write().await;
        integrations.push(integration);
    }

    pub async fn broadcast(&self, tenant_id: &str, process_id: &str, alert: Alert) {
        let integrations = self.integrations.read().await;

        for integration in integrations.iter() {
            match integration.send_alert(tenant_id, process_id, &alert).await {
                Ok(_) => {
                    self.metrics.alerts_sent.inc();
                    debug!("Alert sent to {}", integration.name());
                }
                Err(e) => {
                    self.metrics.alerts_failed.inc();
                    warn!("Failed to send alert to {}: {}", integration.name(), e);
                }
            }
        }
    }
}

impl Default for AlertBroadcaster {
    fn default() -> Self {
        Self::new()
    }
}

/// Convert alert severity to color for Slack.
fn color_for_severity(severity: crate::drift::AlertSeverity) -> &'static str {
    match severity {
        crate::drift::AlertSeverity::Info => "#36a64f",    // blue
        crate::drift::AlertSeverity::Warning => "#ff9900", // orange
        crate::drift::AlertSeverity::Error => "#ff0000",    // red
        crate::drift::AlertSeverity::Critical => "#990000", // dark red
    }
}

/// Convert alert severity to PagerDuty severity.
fn severity_to_pagerduty(severity: crate::drift::AlertSeverity) -> &'static str {
    match severity {
        crate::drift::AlertSeverity::Info => "info",
        crate::drift::AlertSeverity::Warning => "warning",
        crate::drift::AlertSeverity::Error => "error",
        crate::drift::AlertSeverity::Critical => "critical",
    }
}

/// Format alert rule for display.
fn format_alert_rule(alert: &Alert) -> String {
    match alert {
        Alert::FitnessBelowThreshold { .. } => "Fitness Below Threshold".to_string(),
        Alert::PerfectRateBelow { .. } => "Perfect Rate Below Threshold".to_string(),
        Alert::MissingTokensExceeded { .. } => "Missing Tokens Exceeded".to_string(),
        Alert::DriftSignal { detection_rule, .. } => detection_rule.clone(),
    }
}

/// Format alert details for display.
fn format_alert_details(alert: &Alert) -> String {
    match alert {
        Alert::FitnessBelowThreshold { current, threshold, .. } => {
            format!("Current fitness: {:.2}, Threshold: {:.2}", current, threshold)
        }
        Alert::PerfectRateBelow { current, threshold, .. } => {
            format!("Current perfect rate: {:.2}, Threshold: {:.2}", current, threshold)
        }
        Alert::MissingTokensExceeded { current, threshold, .. } => {
            format!("Current missing tokens: {:.2}, Threshold: {:.2}", current, threshold)
        }
        Alert::DriftSignal { metric, current_value, sigma_distance, .. } => {
            format!("Metric: {}, Value: {:.2}, Sigma: {:.2}", metric, current_value, sigma_distance)
        }
    }
}

/// Exported alert type for external use.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DriftAlert {
    pub rule: String,
    pub metric: Option<String>,
    pub current_value: Option<f64>,
    pub threshold: Option<f64>,
    pub sigma_distance: Option<f64>,
    pub severity: crate::drift::AlertSeverity,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

impl From<&crate::drift::Alert> for DriftAlert {
    fn from(alert: &crate::drift::Alert) -> Self {
        match alert {
            crate::drift::Alert::FitnessBelowThreshold { current, threshold, severity } => DriftAlert {
                rule: "fitness_below_threshold".to_string(),
                metric: Some("fitness".to_string()),
                current_value: Some(*current),
                threshold: Some(*threshold),
                sigma_distance: None,
                severity: *severity,
                timestamp: chrono::Utc::now(),
            },
            crate::drift::Alert::PerfectRateBelow { current, threshold, severity } => DriftAlert {
                rule: "perfect_rate_below".to_string(),
                metric: Some("perfect_rate".to_string()),
                current_value: Some(*current),
                threshold: Some(*threshold),
                sigma_distance: None,
                severity: *severity,
                timestamp: chrono::Utc::now(),
            },
            crate::drift::Alert::MissingTokensExceeded { current, threshold, severity } => DriftAlert {
                rule: "missing_tokens_exceeded".to_string(),
                metric: Some("missing_tokens".to_string()),
                current_value: Some(*current),
                threshold: Some(*threshold),
                sigma_distance: None,
                severity: *severity,
                timestamp: chrono::Utc::now(),
            },
            crate::drift::Alert::DriftSignal { metric, detection_rule: rule, current_value, sigma_distance, severity } => DriftAlert {
                rule: rule.clone(),
                metric: Some(metric.clone()),
                current_value: Some(*current_value),
                threshold: None,
                sigma_distance: Some(*sigma_distance),
                severity: *severity,
                timestamp: chrono::Utc::now(),
            },
        }
    }
}
