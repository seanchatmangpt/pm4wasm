// Drift Server – Real-time Process Drift Detection SaaS
// Copyright (C) 2024 Process Intelligence Solutions
// Apache License 2.0

//! Drift detection with EWMA smoothing and SPC analysis.
//!
//! This module provides the core drift detection logic that runs on each
//! incoming trace to detect process deviations in real-time.

use chrono::DateTime;
use serde::{Deserialize, Serialize};

/// Configuration for the drift detector.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DetectorConfig {
    /// EWMA smoothing factor (0.0 - 1.0).
    pub ewma_alpha: f64,
    /// Fitness threshold for alerts.
    pub fitness_threshold: f64,
    /// Perfect rate threshold for alerts.
    pub perfect_rate_threshold: f64,
    /// Missing tokens threshold for alerts.
    pub missing_tokens_threshold: f64,
    /// Enable SPC drift detection.
    pub enable_spc: bool,
    /// Minimum samples for SPC calibration.
    pub spc_calibration_samples: usize,
    /// Control limits multiplier (default 3.0 for 3-sigma).
    pub sigma_multiplier: f64,
}

impl Default for DetectorConfig {
    fn default() -> Self {
        Self {
            ewma_alpha: 0.2,
            fitness_threshold: 0.8,
            perfect_rate_threshold: 0.5,
            missing_tokens_threshold: 5.0,
            enable_spc: true,
            spc_calibration_samples: 30,
            sigma_multiplier: 3.0,
        }
    }
}

/// A single event in a trace.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TraceEvent {
    /// Activity name.
    pub name: String,
    /// Event timestamp (ISO 8601).
    #[serde(with = "chrono::serde::ts_seconds_option")]
    pub timestamp: Option<DateTime<chrono::Utc>>,
    /// Optional event attributes.
    #[serde(flatten)]
    pub attributes: serde_json::Value,
}

/// Current drift detection snapshot.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DriftSnapshot {
    /// Process ID.
    pub process_id: String,
    /// Current raw fitness score (0-1).
    pub fitness: f64,
    /// EWMA-smoothed fitness.
    pub fitness_smoothed: f64,
    /// Current perfect trace rate (0-1).
    pub perfect_rate: f64,
    /// EWMA-smoothed perfect rate.
    pub perfect_rate_smoothed: f64,
    /// Current average missing tokens per trace.
    pub missing_tokens: f64,
    /// EWMA-smoothed missing tokens.
    pub missing_tokens_smoothed: f64,
    /// Total traces processed.
    pub traces_seen: u64,
    /// Active alerts.
    pub alerts: Vec<Alert>,
    /// Whether SPC is calibrated.
    pub spc_calibrated: bool,
    /// Timestamp of this snapshot.
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

/// Drift alert types.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "rule")]
pub enum Alert {
    /// Fitness below threshold.
    FitnessBelowThreshold {
        current: f64,
        threshold: f64,
        severity: AlertSeverity,
    },
    /// Perfect rate below threshold.
    PerfectRateBelow {
        current: f64,
        threshold: f64,
        severity: AlertSeverity,
    },
    /// Missing tokens exceeded.
    MissingTokensExceeded {
        current: f64,
        threshold: f64,
        severity: AlertSeverity,
    },
    /// SPC drift signal.
    DriftSignal {
        metric: String,
        detection_rule: String,
        current_value: f64,
        sigma_distance: f64,
        severity: AlertSeverity,
    },
}

impl Alert {
    /// Get the severity of this alert.
    pub fn severity(&self) -> AlertSeverity {
        match self {
            Alert::FitnessBelowThreshold { severity, .. } => *severity,
            Alert::PerfectRateBelow { severity, .. } => *severity,
            Alert::MissingTokensExceeded { severity, .. } => *severity,
            Alert::DriftSignal { severity, .. } => *severity,
        }
    }
}

/// Alert severity levels.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub enum AlertSeverity {
    Info,
    Warning,
    Error,
    Critical,
}

/// EWMA (Exponentially Weighted Moving Average) smoother.
#[derive(Debug, Clone)]
struct EwmaSmoother {
    alpha: f64,
    value: f64,
    initialized: bool,
}

impl EwmaSmoother {
    fn new(alpha: f64) -> Self {
        assert!(alpha > 0.0 && alpha <= 1.0, "alpha must be in (0, 1]");
        Self {
            alpha,
            value: 0.0,
            initialized: false,
        }
    }

    fn update(&mut self, new_value: f64) -> f64 {
        if !self.initialized {
            self.value = new_value;
            self.initialized = true;
        } else {
            self.value = self.alpha * new_value + (1.0 - self.alpha) * self.value;
        }
        self.value
    }

    fn get(&self) -> f64 {
        self.value
    }

    fn reset(&mut self) {
        self.value = 0.0;
        self.initialized = false;
    }
}

/// SPC (Statistical Process Control) engine.
#[derive(Debug, Clone)]
struct SpcEngine {
    baseline_mean: Option<f64>,
    baseline_std: Option<f64>,
    history: std::collections::VecDeque<f64>,
    calibrated: bool,
}

impl SpcEngine {
    fn new() -> Self {
        Self {
            baseline_mean: None,
            baseline_std: None,
            history: std::collections::VecDeque::with_capacity(14),
            calibrated: false,
        }
    }

    fn calibrate(&mut self, samples: &[f64]) {
        if samples.len() < 30 {
            return;
        }
        let n = samples.len() as f64;
        let mean = samples.iter().sum::<f64>() / n;
        let variance = samples.iter().map(|&x| (x - mean).powi(2)).sum::<f64>() / n;
        let std = variance.sqrt();
        self.baseline_mean = Some(mean);
        self.baseline_std = Some(std);
        self.calibrated = true;
    }

    fn update(&mut self, value: f64) -> Option<Alert> {
        if !self.calibrated {
            return None;
        }

        // Update history
        if self.history.len() >= 14 {
            self.history.pop_front();
        }
        self.history.push_back(value);

        let mean = self.baseline_mean.unwrap();
        let std = self.baseline_std.unwrap();
        let sigma = if std > 0.0 {
            (value - mean) / std
        } else {
            0.0
        };

        // Rule 1: One point beyond 3-sigma
        if sigma.abs() > 3.0 {
            return Some(Alert::DriftSignal {
                metric: "fitness".to_string(),
                detection_rule: "one_point_beyond_3sigma".to_string(),
                current_value: value,
                sigma_distance: sigma,
                severity: severity_from_sigma(sigma),
            });
        }

        None
    }
}

fn severity_from_sigma(sigma: f64) -> AlertSeverity {
    let abs = sigma.abs();
    if abs > 4.0 {
        AlertSeverity::Critical
    } else if abs > 3.0 {
        AlertSeverity::Error
    } else if abs > 2.0 {
        AlertSeverity::Warning
    } else {
        AlertSeverity::Info
    }
}

/// Real-time drift detector.
#[derive(Debug)]
pub struct DriftDetector {
    id: String,
    config: DetectorConfig,
    traces_seen: u64,
    total_fitness: f64,
    perfect_traces: u64,
    total_missing: f64,
    fitness_ewma: EwmaSmoother,
    perfect_rate_ewma: EwmaSmoother,
    missing_ewma: EwmaSmoother,
    spc: SpcEngine,
    calibration_data: Vec<f64>,
}

impl DriftDetector {
    pub fn new(id: String) -> Self {
        let config = DetectorConfig::default();
        let calibration_capacity = config.spc_calibration_samples;
        Self {
            id,
            fitness_ewma: EwmaSmoother::new(config.ewma_alpha),
            perfect_rate_ewma: EwmaSmoother::new(config.ewma_alpha),
            missing_ewma: EwmaSmoother::new(config.ewma_alpha),
            spc: SpcEngine::new(),
            config,
            traces_seen: 0,
            total_fitness: 0.0,
            perfect_traces: 0,
            total_missing: 0.0,
            calibration_data: Vec::with_capacity(calibration_capacity),
        }
    }

    pub fn config(&self) -> DetectorConfig {
        self.config.clone()
    }

    pub fn update_config(&mut self, config: DetectorConfig) {
        self.fitness_ewma = EwmaSmoother::new(config.ewma_alpha);
        self.perfect_rate_ewma = EwmaSmoother::new(config.ewma_alpha);
        self.missing_ewma = EwmaSmoother::new(config.ewma_alpha);
        self.config = config;
    }

    pub fn process_trace(&mut self, _case_id: &str, events: &[TraceEvent]) -> Result<DriftSnapshot, anyhow::Error> {
        // Calculate fitness (simplified - in production would use token replay)
        let fitness = 1.0; // Placeholder - would use actual conformance checking
        let is_perfect = events.len() > 1;

        // Update totals
        self.traces_seen += 1;
        self.total_fitness += fitness;
        if is_perfect {
            self.perfect_traces += 1;
        }

        // Calculate metrics
        let current_fitness = fitness;
        let current_perfect_rate = if self.traces_seen > 0 {
            self.perfect_traces as f64 / self.traces_seen as f64
        } else {
            1.0
        };
        let current_missing = 0.0; // Placeholder

        // Update EWMA
        let smoothed_fitness = self.fitness_ewma.update(current_fitness);
        let smoothed_perfect_rate = self.perfect_rate_ewma.update(current_perfect_rate);
        let smoothed_missing = self.missing_ewma.update(current_missing);

        // SPC calibration
        if self.config.enable_spc && self.calibration_data.len() < self.config.spc_calibration_samples {
            self.calibration_data.push(smoothed_fitness);
            if self.calibration_data.len() == self.config.spc_calibration_samples {
                self.spc.calibrate(&self.calibration_data);
            }
        }

        // Check for alerts
        let mut alerts = Vec::new();

        // Threshold alerts
        if smoothed_fitness < self.config.fitness_threshold {
            alerts.push(Alert::FitnessBelowThreshold {
                current: smoothed_fitness,
                threshold: self.config.fitness_threshold,
                severity: AlertSeverity::Warning,
            });
        }

        // SPC alerts
        if let Some(spc_alert) = self.spc.update(smoothed_fitness) {
            alerts.push(spc_alert);
        }

        Ok(DriftSnapshot {
            process_id: self.id.clone(),
            fitness: current_fitness,
            fitness_smoothed: smoothed_fitness,
            perfect_rate: current_perfect_rate,
            perfect_rate_smoothed: smoothed_perfect_rate,
            missing_tokens: current_missing,
            missing_tokens_smoothed: smoothed_missing,
            traces_seen: self.traces_seen,
            alerts,
            spc_calibrated: self.spc.calibrated,
            timestamp: chrono::Utc::now(),
        })
    }
}
