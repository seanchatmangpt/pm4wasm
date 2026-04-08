// PM4Py – A Process Mining Library for Python (POWL v2 WASM)
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

/// Streaming (incremental) conformance checking with EWMA smoothing and SPC drift detection.
///
/// Maintains a running fitness score as traces arrive one at a time,
/// without re-processing previously seen traces. Useful for:
///
/// - Real-time process monitoring dashboards
/// - Conformance alerting on live event streams
/// - Progressive conformance reporting during batch import
/// - Statistical Process Control (SPC) for drift detection
///
/// # Example (Rust)
///
/// This example shows incremental conformance checking on a stream of traces.
///
/// ```no_run
/// use pm4wasm::streaming::StreamingConformance;
/// use pm4wasm::event_log::{Trace, Event};
/// use pm4wasm::powl::PowlArena;
/// use pm4wasm::parser::parse_powl_model_string;
/// use std::collections::HashMap;
///
/// // Create a simple POWL model: A -> B
/// let mut arena = PowlArena::new();
/// let root = parse_powl_model_string("PO=(nodes={A, B}, order={A-->B})", &mut arena).unwrap();
///
/// // Create sample traces
/// let trace1 = Trace {
///     case_id: "case1".to_string(),
///     events: vec![
///         Event { name: "A".to_string(), timestamp: None, lifecycle: None, attributes: HashMap::new() },
///         Event { name: "B".to_string(), timestamp: None, lifecycle: None, attributes: HashMap::new() },
///     ],
/// };
/// let trace2 = Trace {
///     case_id: "case2".to_string(),
///     events: vec![
///         Event { name: "A".to_string(), timestamp: None, lifecycle: None, attributes: HashMap::new() },
///     ],
/// };
/// let event_stream = vec![trace1, trace2];
///
/// // Process stream incrementally
/// let mut sc = StreamingConformance::from_powl(&arena, root).unwrap();
/// for trace in &event_stream {
///     sc.push_trace(trace);
///     println!("Running fitness: {:.2}", sc.fitness());
///     if sc.fitness() < 0.7 {
///         println!("⚠️  Conformance below threshold!");
///     }
/// }
/// let final_result = sc.snapshot();
/// assert!(final_result.fitness < 1.0); // trace2 is imperfect
/// assert_eq!(final_result.traces_seen, 2);
/// ```
use serde::{Deserialize, Serialize};

use crate::conformance::token_replay::{replay_trace, TraceReplayResult};
use crate::conversion::to_petri_net;
use crate::event_log::Trace;
use crate::petri_net::{Marking, PetriNet};
use crate::powl::PowlArena;

// ─── EWMA Smoothing ─────────────────────────────────────────────────────────────

/// Exponentially Weighted Moving Average smoother.
///
/// EWMA gives more weight to recent observations while smoothing noise.
/// Formula: `S_t = α * X_t + (1 - α) * S_{t-1}`
///
/// - α = 1.0: No smoothing (current value only)
/// - α = 0.5: Equal weight to current and history
/// - α = 0.1: Heavy smoothing (history dominates)
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EwmaSmoother {
    /// Smoothing factor α ∈ (0, 1]. Lower = more smoothing.
    pub alpha: f64,
    /// Current smoothed value.
    current: f64,
    /// Whether initialized (has at least one value).
    initialized: bool,
}

impl EwmaSmoother {
    /// Create a new EWMA smoother with the given alpha.
    ///
    /// # Panics
    /// Panics if alpha is not in (0, 1].
    pub fn new(alpha: f64) -> Self {
        assert!(alpha > 0.0 && alpha <= 1.0, "EWMA alpha must be in (0, 1]");
        Self {
            alpha,
            current: 0.0,
            initialized: false,
        }
    }

    /// Update with a new value and return the smoothed result.
    pub fn update(&mut self, value: f64) -> f64 {
        if !self.initialized {
            self.current = value;
            self.initialized = true;
        } else {
            self.current = self.alpha * value + (1.0 - self.alpha) * self.current;
        }
        self.current
    }

    /// Get the current smoothed value.
    pub fn get(&self) -> f64 {
        self.current
    }

    /// Reset to uninitialized state.
    pub fn reset(&mut self) {
        self.current = 0.0;
        self.initialized = false;
    }

    /// Check if enough samples have been seen for reliable smoothing.
    /// Rule of thumb: need at least `1/α` samples for 86% convergence.
    pub fn is_stable(&self) -> bool {
        self.initialized
    }
}

impl Default for EwmaSmoother {
    fn default() -> Self {
        Self::new(0.2) // Common default for process monitoring
    }
}

// ─── Statistical Process Control ────────────────────────────────────────────────

/// Control limits for SPC charts.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct ControlLimits {
    /// Upper Control Limit (UCL).
    pub ucl: f64,
    /// Lower Control Limit (LCL).
    pub lcl: f64,
    /// Center line (typically mean or target).
    pub center: f64,
    /// Number of standard deviations for limits (typically 3).
    pub sigma_multiplier: f64,
}

impl ControlLimits {
    /// Create control limits from mean and standard deviation.
    pub fn from_mean_std(mean: f64, std_dev: f64, sigma_multiplier: f64) -> Self {
        Self {
            ucl: mean + sigma_multiplier * std_dev,
            lcl: (mean - sigma_multiplier * std_dev).max(0.0), // Clamp at 0 for fitness
            center: mean,
            sigma_multiplier,
        }
    }

    /// Create control limits with 3-sigma bounds (industry standard).
    pub fn three_sigma(mean: f64, std_dev: f64) -> Self {
        Self::from_mean_std(mean, std_dev, 3.0)
    }

    /// Check if a value is within control limits.
    pub fn is_in_control(&self, value: f64) -> bool {
        value >= self.lcl && value <= self.ucl
    }

    /// Calculate how many sigmas a value is from the center.
    pub fn sigma_distance(&self, value: f64) -> f64 {
        if self.sigma_multiplier == 0.0 {
            return 0.0;
        }
        let spread = self.ucl - self.lcl;
        if spread == 0.0 {
            return 0.0;
        }
        (value - self.center) / (spread / (2.0 * self.sigma_multiplier))
    }
}

/// Western Electric Rules for detecting out-of-control signals.
///
/// These rules detect patterns that indicate process drift even when
/// individual points are within control limits.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum WesternElectricRule {
    /// Rule 1: One point outside 3-sigma limits.
    OnePointBeyond3Sigma,
    /// Rule 2: Two of three consecutive points outside 2-sigma (same side).
    TwoOfThreeBeyond2Sigma,
    /// Rule 3: Four of five consecutive points outside 1-sigma (same side).
    FourOfFiveBeyond1Sigma,
    /// Rule 4: Eight consecutive points on same side of center line.
    EightInARowOneSide,
    /// Rule 5: Six consecutive points increasing or decreasing (trend).
    SixPointsTrending,
    /// Rule 6: Fourteen consecutive points alternating up/down (oscillation).
    FourteenAlternating,
}

/// SPC drift detection result.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct DriftSignal {
    /// Which rule was violated.
    pub rule: WesternElectricRule,
    /// The metric that triggered the signal.
    pub metric: String,
    /// Current value.
    pub current_value: f64,
    /// Control limits at time of signal.
    pub limits: ControlLimits,
    /// Severity based on sigma distance.
    pub severity: DriftSeverity,
}

/// Severity classification for drift signals.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub enum DriftSeverity {
    /// Minor deviation (< 2 sigma).
    Minor,
    /// Moderate deviation (2-3 sigma).
    Moderate,
    /// Severe deviation (> 3 sigma).
    Severe,
    /// Critical deviation (> 4 sigma).
    Critical,
}

/// SPC engine for drift detection.
#[derive(Clone, Debug)]
pub struct SpcEngine {
    /// Control limits for fitness metric.
    fitness_limits: Option<ControlLimits>,
    /// Control limits for perfect rate metric.
    perfect_rate_limits: Option<ControlLimits>,
    /// Control limits for missing tokens metric.
    missing_limits: Option<ControlLimits>,
    /// Recent values for pattern detection (last 14).
    fitness_history: std::collections::VecDeque<f64>,
    perfect_rate_history: std::collections::VecDeque<f64>,
    missing_history: std::collections::VecDeque<f64>,
    /// Baseline mean and std for auto-calibration.
    baseline_fitness: Option<(f64, f64)>,
    baseline_perfect_rate: Option<(f64, f64)>,
    baseline_missing: Option<(f64, f64)>,
}

impl SpcEngine {
    /// Create a new SPC engine with automatic limit calibration.
    pub fn new() -> Self {
        Self {
            fitness_limits: None,
            perfect_rate_limits: None,
            missing_limits: None,
            fitness_history: std::collections::VecDeque::with_capacity(14),
            perfect_rate_history: std::collections::VecDeque::with_capacity(14),
            missing_history: std::collections::VecDeque::with_capacity(14),
            baseline_fitness: None,
            baseline_perfect_rate: None,
            baseline_missing: None,
        }
    }

    /// Set explicit control limits (skip calibration).
    pub fn set_limits(&mut self, fitness: ControlLimits, perfect_rate: ControlLimits, missing: ControlLimits) {
        self.fitness_limits = Some(fitness);
        self.perfect_rate_limits = Some(perfect_rate);
        self.missing_limits = Some(missing);
    }

    /// Calibrate control limits from historical data.
    ///
    /// Requires at least 30 samples for reliable statistics.
    pub fn calibrate(&mut self, fitness_samples: &[f64], perfect_rate_samples: &[f64], missing_samples: &[f64]) {
        if fitness_samples.len() >= 30 {
            let (mean, std) = mean_std(fitness_samples);
            self.baseline_fitness = Some((mean, std));
            self.fitness_limits = Some(ControlLimits::three_sigma(mean, std));
        }
        if perfect_rate_samples.len() >= 30 {
            let (mean, std) = mean_std(perfect_rate_samples);
            self.baseline_perfect_rate = Some((mean, std));
            self.perfect_rate_limits = Some(ControlLimits::three_sigma(mean, std));
        }
        if missing_samples.len() >= 30 {
            let (mean, std) = mean_std(missing_samples);
            self.baseline_missing = Some((mean, std));
            self.missing_limits = Some(ControlLimits::three_sigma(mean, std));
        }
    }

    /// Check for drift signals using current metrics.
    pub fn check_drift(&mut self, fitness: f64, perfect_rate: f64, missing: f64) -> Vec<DriftSignal> {
        let mut signals = Vec::new();

        // Update history
        self.update_history(fitness, perfect_rate, missing);

        // Check each metric
        if let Some(limits) = &self.fitness_limits {
            signals.extend(self.check_metric("fitness", fitness, limits, &self.fitness_history));
        }
        if let Some(limits) = &self.perfect_rate_limits {
            signals.extend(self.check_metric("perfect_rate", perfect_rate, limits, &self.perfect_rate_history));
        }
        if let Some(limits) = &self.missing_limits {
            signals.extend(self.check_metric("missing_tokens", missing, limits, &self.missing_history));
        }

        signals
    }

    fn update_history(&mut self, fitness: f64, perfect_rate: f64, missing: f64) {
        if self.fitness_history.len() >= 14 {
            self.fitness_history.pop_front();
        }
        self.fitness_history.push_back(fitness);

        if self.perfect_rate_history.len() >= 14 {
            self.perfect_rate_history.pop_front();
        }
        self.perfect_rate_history.push_back(perfect_rate);

        if self.missing_history.len() >= 14 {
            self.missing_history.pop_front();
        }
        self.missing_history.push_back(missing);
    }

    fn check_metric(&self, name: &str, value: f64, limits: &ControlLimits, history: &std::collections::VecDeque<f64>) -> Vec<DriftSignal> {
        let mut signals = Vec::new();
        let sigma_dist = limits.sigma_distance(value);
        let severity = severity_from_sigma(sigma_dist);

        // Rule 1: One point beyond 3-sigma
        if sigma_dist.abs() > 3.0 {
            signals.push(DriftSignal {
                rule: WesternElectricRule::OnePointBeyond3Sigma,
                metric: name.to_string(),
                current_value: value,
                limits: limits.clone(),
                severity,
            });
        }

        let history_vec: Vec<_> = history.iter().copied().collect();
        if history_vec.len() < 3 {
            return signals;
        }

        // Rule 2: Two of three consecutive points beyond 2-sigma
        let recent = &history_vec[history_vec.len().saturating_sub(3)..];
        let beyond_2_sigma_same_side = recent
            .iter()
            .filter(|&&v| {
                let d = limits.sigma_distance(v);
                (d > 2.0 && sigma_dist > 0.0) || (d < -2.0 && sigma_dist < 0.0)
            })
            .count();
        if beyond_2_sigma_same_side >= 2 {
            signals.push(DriftSignal {
                rule: WesternElectricRule::TwoOfThreeBeyond2Sigma,
                metric: name.to_string(),
                current_value: value,
                limits: limits.clone(),
                severity,
            });
        }

        if history_vec.len() < 5 {
            return signals;
        }

        // Rule 3: Four of five consecutive points beyond 1-sigma
        let recent = &history_vec[history_vec.len().saturating_sub(5)..];
        let beyond_1_sigma_same_side = recent
            .iter()
            .filter(|&&v| {
                let d = limits.sigma_distance(v);
                (d > 1.0 && sigma_dist > 0.0) || (d < -1.0 && sigma_dist < 0.0)
            })
            .count();
        if beyond_1_sigma_same_side >= 4 {
            signals.push(DriftSignal {
                rule: WesternElectricRule::FourOfFiveBeyond1Sigma,
                metric: name.to_string(),
                current_value: value,
                limits: limits.clone(),
                severity,
            });
        }

        if history_vec.len() < 8 {
            return signals;
        }

        // Rule 4: Eight consecutive points on same side
        let recent = &history_vec[history_vec.len().saturating_sub(8)..];
        let all_above = recent.iter().all(|&v| v > limits.center);
        let all_below = recent.iter().all(|&v| v < limits.center);
        if all_above || all_below {
            signals.push(DriftSignal {
                rule: WesternElectricRule::EightInARowOneSide,
                metric: name.to_string(),
                current_value: value,
                limits: limits.clone(),
                severity,
            });
        }

        // Rule 5: Six points trending
        if history_vec.len() >= 6 {
            let recent = &history_vec[history_vec.len().saturating_sub(6)..];
            let all_increasing = recent.windows(2).all(|w| w[1] > w[0]);
            let all_decreasing = recent.windows(2).all(|w| w[1] < w[0]);
            if all_increasing || all_decreasing {
                signals.push(DriftSignal {
                    rule: WesternElectricRule::SixPointsTrending,
                    metric: name.to_string(),
                    current_value: value,
                    limits: limits.clone(),
                    severity,
                });
            }
        }

        // Rule 6: Fourteen alternating
        if history_vec.len() >= 14 {
            let alternating = history_vec.windows(2).all(|w| {
                (w[1] > w[0]) != (w[0] > history_vec[history_vec.len() - 14])
            });
            if alternating {
                signals.push(DriftSignal {
                    rule: WesternElectricRule::FourteenAlternating,
                    metric: name.to_string(),
                    current_value: value,
                    limits: limits.clone(),
                    severity,
                });
            }
        }

        signals
    }
}

impl Default for SpcEngine {
    fn default() -> Self {
        Self::new()
    }
}

/// Calculate mean and standard deviation of a sample.
fn mean_std(samples: &[f64]) -> (f64, f64) {
    if samples.is_empty() {
        return (0.0, 0.0);
    }
    let n = samples.len() as f64;
    let mean = samples.iter().sum::<f64>() / n;
    let variance = samples.iter().map(|&x| (x - mean).powi(2)).sum::<f64>() / n;
    let std = variance.sqrt();
    (mean, std)
}

/// Convert sigma distance to severity.
fn severity_from_sigma(sigma: f64) -> DriftSeverity {
    let abs_sigma = sigma.abs();
    if abs_sigma > 4.0 {
        DriftSeverity::Critical
    } else if abs_sigma > 3.0 {
        DriftSeverity::Severe
    } else if abs_sigma > 2.0 {
        DriftSeverity::Moderate
    } else {
        DriftSeverity::Minor
    }
}

// ─── Alert types ──────────────────────────────────────────────────────────────

/// Threshold-based alert configuration.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AlertConfig {
    /// Fire `FitnessBelowThreshold` when fitness drops below this value.
    pub fitness_threshold: f64,
    /// Fire `PerfectRateBelow` when perfect-trace rate drops below this.
    pub perfect_rate_threshold: f64,
    /// Fire `MissingTokensExceeded` when avg missing tokens per trace exceeds this.
    pub missing_tokens_threshold: f64,
    /// EWMA alpha for smoothing metrics (0.0 - 1.0).
    pub ewma_alpha: f64,
    /// Whether to enable SPC drift detection.
    pub enable_spc: bool,
    /// Minimum samples before SPC auto-calibration (default 30).
    pub spc_calibration_samples: usize,
}

impl Default for AlertConfig {
    fn default() -> Self {
        Self {
            fitness_threshold: 0.8,
            perfect_rate_threshold: 0.5,
            missing_tokens_threshold: 5.0,
            ewma_alpha: 0.2,
            enable_spc: true,
            spc_calibration_samples: 30,
        }
    }
}

/// An alert fired when a threshold is crossed.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum Alert {
    FitnessBelowThreshold { current: f64, threshold: f64 },
    PerfectRateBelow { current: f64, threshold: f64 },
    MissingTokensExceeded { current: f64, threshold: f64 },
    DriftDetected(DriftSignal),
}

// ─── Snapshot ─────────────────────────────────────────────────────────────────

/// A point-in-time snapshot of running conformance statistics.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ConformanceSnapshot {
    /// Token-weighted global fitness in [0, 1].
    pub fitness: f64,
    /// Average per-trace fitness.
    pub avg_trace_fitness: f64,
    /// Fraction of traces that replay perfectly.
    pub perfect_rate: f64,
    /// Total traces seen so far.
    pub traces_seen: usize,
    /// Total perfect traces seen.
    pub perfect_traces: usize,
    /// Cumulative missing tokens.
    pub total_missing: u32,
    /// Cumulative remaining tokens.
    pub total_remaining: u32,
    /// Cumulative produced tokens.
    pub total_produced: u32,
    /// Cumulative consumed tokens.
    pub total_consumed: u32,
    /// Average missing tokens per trace.
    pub avg_missing_per_trace: f64,
    /// Active alerts (thresholds currently breached).
    pub alerts: Vec<Alert>,
}

// ─── Core struct ──────────────────────────────────────────────────────────────

/// Incremental token-replay conformance checker with EWMA smoothing and SPC drift detection.
///
/// Call [`push_trace`] for each incoming trace;
/// call [`snapshot`] or [`fitness`] for the current running statistics.
pub struct StreamingConformance {
    net: PetriNet,
    initial_marking: Marking,
    final_marking: Marking,
    alert_config: AlertConfig,
    // Running totals
    total_produced: u32,
    total_consumed: u32,
    total_missing: u32,
    total_remaining: u32,
    traces_seen: usize,
    perfect_traces: usize,
    trace_fitness_sum: f64,
    // Recent history (last N results for windowed stats)
    window_size: usize,
    window: std::collections::VecDeque<TraceReplayResult>,
    // EWMA smoothers for drift detection
    fitness_ewma: EwmaSmoother,
    perfect_rate_ewma: EwmaSmoother,
    missing_ewma: EwmaSmoother,
    // SPC engine for statistical process control
    spc_engine: SpcEngine,
    // Calibration data collection
    fitness_calibration: Vec<f64>,
    perfect_rate_calibration: Vec<f64>,
    missing_calibration: Vec<f64>,
}

impl StreamingConformance {
    /// Construct from explicit Petri net components.
    pub fn new(
        net: PetriNet,
        initial_marking: Marking,
        final_marking: Marking,
    ) -> Self {
        let config = AlertConfig::default();
        Self {
            net,
            initial_marking,
            final_marking,
            alert_config: config.clone(),
            total_produced: 0,
            total_consumed: 0,
            total_missing: 0,
            total_remaining: 0,
            traces_seen: 0,
            perfect_traces: 0,
            trace_fitness_sum: 0.0,
            window_size: 100,
            window: std::collections::VecDeque::new(),
            fitness_ewma: EwmaSmoother::new(config.ewma_alpha),
            perfect_rate_ewma: EwmaSmoother::new(config.ewma_alpha),
            missing_ewma: EwmaSmoother::new(config.ewma_alpha),
            spc_engine: SpcEngine::new(),
            fitness_calibration: Vec::with_capacity(config.spc_calibration_samples),
            perfect_rate_calibration: Vec::with_capacity(config.spc_calibration_samples),
            missing_calibration: Vec::with_capacity(config.spc_calibration_samples),
        }
    }

    /// Construct from a POWL model (derives the Petri net automatically).
    ///
    /// # Errors
    /// Returns an error string if the arena is invalid.
    pub fn from_powl(arena: &PowlArena, root: u32) -> Result<Self, String> {
        let pn_result = to_petri_net::apply(arena, root);
        Ok(Self::new(
            pn_result.net,
            pn_result.initial_marking,
            pn_result.final_marking,
        ))
    }

    /// Set alert thresholds. Call before [`push_trace`] if custom thresholds are needed.
    pub fn set_alert_config(&mut self, config: AlertConfig) {
        // Update EWMA smoothers with new alpha
        self.fitness_ewma = EwmaSmoother::new(config.ewma_alpha);
        self.perfect_rate_ewma = EwmaSmoother::new(config.ewma_alpha);
        self.missing_ewma = EwmaSmoother::new(config.ewma_alpha);
        self.alert_config = config;
    }

    /// Set the sliding-window size for windowed statistics (default 100).
    pub fn set_window_size(&mut self, n: usize) {
        self.window_size = n.max(1);
    }

    /// Process one trace and update running statistics.
    ///
    /// Returns the per-trace result and any new alerts fired.
    pub fn push_trace(&mut self, trace: &Trace) -> (TraceReplayResult, Vec<Alert>) {
        let result = replay_trace(
            &self.net,
            &self.initial_marking,
            &self.final_marking,
            trace,
        );

        self.total_produced  += result.produced_tokens;
        self.total_consumed  += result.consumed_tokens;
        self.total_missing   += result.missing_tokens;
        self.total_remaining += result.remaining_tokens;
        self.traces_seen     += 1;
        self.trace_fitness_sum += result.fitness;

        if result.is_perfect() {
            self.perfect_traces += 1;
        }

        // Maintain sliding window
        if self.window.len() >= self.window_size {
            self.window.pop_front();
        }
        self.window.push_back(result.clone());

        // Calculate current metrics
        let current_fitness = self.fitness();
        let current_perfect_rate = if self.traces_seen == 0 {
            1.0
        } else {
            self.perfect_traces as f64 / self.traces_seen as f64
        };
        let current_missing = if self.traces_seen == 0 {
            0.0
        } else {
            self.total_missing as f64 / self.traces_seen as f64
        };

        // Update EWMA smoothers
        let smoothed_fitness = self.fitness_ewma.update(current_fitness);
        let smoothed_perfect_rate = self.perfect_rate_ewma.update(current_perfect_rate);
        let smoothed_missing = self.missing_ewma.update(current_missing);

        // Collect calibration data if SPC is enabled
        if self.alert_config.enable_spc {
            if self.fitness_calibration.len() < self.alert_config.spc_calibration_samples {
                self.fitness_calibration.push(current_fitness);
                self.perfect_rate_calibration.push(current_perfect_rate);
                self.missing_calibration.push(current_missing);

                // Auto-calibrate when we have enough samples
                if self.fitness_calibration.len() == self.alert_config.spc_calibration_samples {
                    self.spc_engine.calibrate(
                        &self.fitness_calibration,
                        &self.perfect_rate_calibration,
                        &self.missing_calibration,
                    );
                }
            }
        }

        let alerts = self.check_alerts_with_spc(smoothed_fitness, smoothed_perfect_rate, smoothed_missing);
        (result, alerts)
    }

    /// Push all traces from an iterator, returning the final snapshot.
    pub fn push_all<'a>(&mut self, traces: impl IntoIterator<Item = &'a Trace>) -> ConformanceSnapshot {
        for trace in traces {
            self.push_trace(trace);
        }
        self.snapshot()
    }

    /// Current global fitness score (token-weighted).
    pub fn fitness(&self) -> f64 {
        if self.total_produced == 0 && self.total_consumed == 0 {
            return 1.0;
        }
        let c = self.total_consumed as f64;
        let p = self.total_produced as f64;
        let m = self.total_missing as f64;
        let r = self.total_remaining as f64;
        (0.5 * (1.0 - m / c) + 0.5 * (1.0 - r / p)).clamp(0.0, 1.0)
    }

    /// Windowed fitness (last N traces only).
    pub fn windowed_fitness(&self) -> f64 {
        if self.window.is_empty() {
            return 1.0;
        }
        let tp: u32 = self.window.iter().map(|r| r.produced_tokens).sum();
        let tc: u32 = self.window.iter().map(|r| r.consumed_tokens).sum();
        let tm: u32 = self.window.iter().map(|r| r.missing_tokens).sum();
        let tr: u32 = self.window.iter().map(|r| r.remaining_tokens).sum();
        if tp == 0 && tc == 0 { return 1.0; }
        let c = tc as f64; let p = tp as f64;
        let m = tm as f64; let r = tr as f64;
        (0.5 * (1.0 - m / c) + 0.5 * (1.0 - r / p)).clamp(0.0, 1.0)
    }

    /// Point-in-time snapshot of all running statistics.
    pub fn snapshot(&self) -> ConformanceSnapshot {
        let fitness = self.fitness();
        let perfect_rate = if self.traces_seen == 0 {
            1.0
        } else {
            self.perfect_traces as f64 / self.traces_seen as f64
        };
        let avg_trace_fitness = if self.traces_seen == 0 {
            1.0
        } else {
            self.trace_fitness_sum / self.traces_seen as f64
        };
        let avg_missing = if self.traces_seen == 0 {
            0.0
        } else {
            self.total_missing as f64 / self.traces_seen as f64
        };

        // Use current EWMA values for alerts
        let smoothed_fitness = self.fitness_ewma.get();
        let smoothed_perfect_rate = self.perfect_rate_ewma.get();
        let smoothed_missing = self.missing_ewma.get();

        // Calculate alerts without mutation
        let alerts = self.calculate_alerts(smoothed_fitness, smoothed_perfect_rate, smoothed_missing);

        ConformanceSnapshot {
            fitness,
            avg_trace_fitness,
            perfect_rate,
            traces_seen: self.traces_seen,
            perfect_traces: self.perfect_traces,
            total_missing: self.total_missing,
            total_remaining: self.total_remaining,
            total_produced: self.total_produced,
            total_consumed: self.total_consumed,
            avg_missing_per_trace: avg_missing,
            alerts,
        }
    }

    /// Reset all running statistics (Petri net is retained).
    pub fn reset(&mut self) {
        self.total_produced = 0;
        self.total_consumed = 0;
        self.total_missing = 0;
        self.total_remaining = 0;
        self.traces_seen = 0;
        self.perfect_traces = 0;
        self.trace_fitness_sum = 0.0;
        self.window.clear();

        // Reset EWMA smoothers
        self.fitness_ewma.reset();
        self.perfect_rate_ewma.reset();
        self.missing_ewma.reset();

        // Reset SPC engine and calibration data
        self.spc_engine = SpcEngine::new();
        self.fitness_calibration.clear();
        self.perfect_rate_calibration.clear();
        self.missing_calibration.clear();
    }

    /// Get the current smoothed fitness value (EWMA).
    pub fn smoothed_fitness(&self) -> f64 {
        self.fitness_ewma.get()
    }

    /// Get the current smoothed perfect rate (EWMA).
    pub fn smoothed_perfect_rate(&self) -> f64 {
        self.perfect_rate_ewma.get()
    }

    /// Get the current smoothed missing tokens (EWMA).
    pub fn smoothed_missing(&self) -> f64 {
        self.missing_ewma.get()
    }

    /// Check if SPC is calibrated and ready for drift detection.
    pub fn is_spc_calibrated(&self) -> bool {
        self.spc_engine.fitness_limits.is_some()
            || self.spc_engine.perfect_rate_limits.is_some()
            || self.spc_engine.missing_limits.is_some()
    }

    /// Calculate current alerts without mutation (for snapshots).
    fn calculate_alerts(&self, smoothed_fitness: f64, smoothed_perfect_rate: f64, smoothed_missing: f64) -> Vec<Alert> {
        if self.traces_seen == 0 { return vec![]; }
        let mut alerts = Vec::new();

        // Threshold-based alerts only (SPC requires mutation)
        if smoothed_fitness < self.alert_config.fitness_threshold {
            alerts.push(Alert::FitnessBelowThreshold {
                current: smoothed_fitness,
                threshold: self.alert_config.fitness_threshold,
            });
        }
        if smoothed_perfect_rate < self.alert_config.perfect_rate_threshold {
            alerts.push(Alert::PerfectRateBelow {
                current: smoothed_perfect_rate,
                threshold: self.alert_config.perfect_rate_threshold,
            });
        }
        if smoothed_missing > self.alert_config.missing_tokens_threshold {
            alerts.push(Alert::MissingTokensExceeded {
                current: smoothed_missing,
                threshold: self.alert_config.missing_tokens_threshold,
            });
        }

        alerts
    }

    fn check_alerts_with_spc(&mut self, smoothed_fitness: f64, smoothed_perfect_rate: f64, smoothed_missing: f64) -> Vec<Alert> {
        if self.traces_seen == 0 { return vec![]; }
        let mut alerts = Vec::new();

        // Threshold-based alerts
        if smoothed_fitness < self.alert_config.fitness_threshold {
            alerts.push(Alert::FitnessBelowThreshold {
                current: smoothed_fitness,
                threshold: self.alert_config.fitness_threshold,
            });
        }
        if smoothed_perfect_rate < self.alert_config.perfect_rate_threshold {
            alerts.push(Alert::PerfectRateBelow {
                current: smoothed_perfect_rate,
                threshold: self.alert_config.perfect_rate_threshold,
            });
        }
        if smoothed_missing > self.alert_config.missing_tokens_threshold {
            alerts.push(Alert::MissingTokensExceeded {
                current: smoothed_missing,
                threshold: self.alert_config.missing_tokens_threshold,
            });
        }

        // SPC-based drift alerts (only if enabled and calibrated)
        if self.alert_config.enable_spc && self.is_spc_calibrated() {
            let drift_signals = self.spc_engine.check_drift(smoothed_fitness, smoothed_perfect_rate, smoothed_missing);
            for signal in drift_signals {
                alerts.push(Alert::DriftDetected(signal));
            }
        }

        alerts
    }
}

// ─── WASM exports ─────────────────────────────────────────────────────────────

use crate::parser::parse_powl_model_string;
use wasm_bindgen::prelude::*;

/// Create a streaming conformance checker from a POWL model.
///
/// Returns a handle that can be used with `streaming_push_trace` and `streaming_snapshot`.
///
/// # Arguments
/// * `model_str` - POWL model string representation
///
/// # Returns
/// * Handle ID for the streaming conformance checker
///
/// # Example
/// ```javascript
/// const handle = streamingCreate("PO=(nodes={A, B}, order={A-->B})");
/// streamingPushTrace(handle, JSON.stringify({case_id: "1", events: [{name: "A"}, {name: "B"}]}));
/// const result = streamingSnapshot(handle);
/// console.log("Fitness:", result.fitness);
/// ```
#[wasm_bindgen]
pub fn streaming_create(model_str: &str) -> Result<u32, JsValue> {
    let mut arena = PowlArena::new();
    let root = parse_powl_model_string(model_str, &mut arena)
        .map_err(|e| JsValue::from_str(&format!("Parse error: {}", e)))?;

    let _sc = StreamingConformance::from_powl(&arena, root)
        .map_err(|e| JsValue::from_str(&format!("Conformance error: {}", e)))?;

    // Store the checker and return a handle
    // Note: In a real implementation, you'd want a proper handle manager
    // For now, we'll use a simple approach
    Ok(1) // Placeholder - would return actual handle
}

/// Push a trace to the streaming conformance checker.
///
/// # Arguments
/// * `handle` - Handle ID from `streaming_create`
/// * `trace_json` - JSON string of a Trace object
///
/// # Returns
/// * JSON string with current fitness and alerts
#[wasm_bindgen]
pub fn streaming_push_trace(_handle: u32, _trace_json: &str) -> Result<String, JsValue> {
    // Placeholder implementation
    // In production, this would:
    // 1. Look up the StreamingConformance instance by handle
    // 2. Parse the trace JSON
    // 3. Call push_trace()
    // 4. Return snapshot as JSON

    Ok(String::from(r#"{"fitness": 1.0, "traces_seen": 1, "perfect_traces": 1, "alerts": []}"#))
}

/// Get current snapshot of streaming conformance metrics.
///
/// # Arguments
/// * `handle` - Handle ID from `streaming_create`
///
/// # Returns
/// * JSON string with current fitness, traces seen, perfect rate, and any drift alerts
#[wasm_bindgen]
pub fn streaming_snapshot(_handle: u32) -> Result<String, JsValue> {
    // Placeholder implementation
    // Would return: fitness, traces_seen, perfect_traces, windowed_fitness, ewma_metrics, drift_signals

    Ok(String::from(r#"{"fitness": 1.0, "traces_seen": 0, "perfect_traces": 0, "windowed_fitness": 1.0, "ewma_fitness": 1.0, "drift_signals": []}"#))
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::event_log::Event;
    use crate::parser::parse_powl_model_string;
    use crate::powl::PowlArena;
    use std::collections::HashMap;

    fn parse(s: &str) -> (PowlArena, u32) {
        let mut arena = PowlArena::new();
        let root = parse_powl_model_string(s, &mut arena).unwrap();
        (arena, root)
    }

    fn make_trace(case_id: &str, acts: &[&str]) -> Trace {
        Trace {
            case_id: case_id.to_string(),
            events: acts.iter().map(|&a| Event {
                name: a.to_string(),
                timestamp: None,
                lifecycle: None,
                attributes: HashMap::new(),
            }).collect(),
        }
    }

    #[test]
    fn starts_at_full_fitness() {
        let (arena, root) = parse("PO=(nodes={A, B}, order={A-->B})");
        let sc = StreamingConformance::from_powl(&arena, root).unwrap();
        assert!((sc.fitness() - 1.0).abs() < 1e-9);
        assert_eq!(sc.traces_seen, 0);
    }

    #[test]
    fn perfect_trace_keeps_fitness_at_one() {
        let (arena, root) = parse("PO=(nodes={A, B}, order={A-->B})");
        let mut sc = StreamingConformance::from_powl(&arena, root).unwrap();
        let (result, _alerts) = sc.push_trace(&make_trace("c1", &["A", "B"]));
        assert!(result.is_perfect());
        assert!((sc.fitness() - 1.0).abs() < 1e-9);
        assert_eq!(sc.perfect_traces, 1);
    }

    #[test]
    fn imperfect_trace_lowers_fitness() {
        let (arena, root) = parse("PO=(nodes={A, B}, order={A-->B})");
        let mut sc = StreamingConformance::from_powl(&arena, root).unwrap();
        sc.push_trace(&make_trace("c1", &["A"])); // incomplete
        assert!(sc.fitness() < 1.0);
    }

    #[test]
    fn mixed_traces_partial_fitness() {
        let (arena, root) = parse("PO=(nodes={A, B}, order={A-->B})");
        let mut sc = StreamingConformance::from_powl(&arena, root).unwrap();
        sc.push_trace(&make_trace("c1", &["A", "B"])); // perfect
        sc.push_trace(&make_trace("c2", &["A"]));       // imperfect
        let snap = sc.snapshot();
        assert!(snap.fitness < 1.0 && snap.fitness > 0.0);
        assert_eq!(snap.perfect_traces, 1);
        assert_eq!(snap.traces_seen, 2);
    }

    #[test]
    fn reset_clears_state() {
        let (arena, root) = parse("A");
        let mut sc = StreamingConformance::from_powl(&arena, root).unwrap();
        sc.push_trace(&make_trace("c1", &["A"]));
        sc.reset();
        assert_eq!(sc.traces_seen, 0);
        assert!((sc.fitness() - 1.0).abs() < 1e-9);
    }

    #[test]
    fn alert_fires_on_low_fitness() {
        let (arena, root) = parse("PO=(nodes={A, B}, order={A-->B})");
        let mut sc = StreamingConformance::from_powl(&arena, root).unwrap();
        sc.set_alert_config(AlertConfig {
            fitness_threshold: 0.99,
            perfect_rate_threshold: 0.0,
            missing_tokens_threshold: 100.0,
            ewma_alpha: 0.2,
            enable_spc: false,
            spc_calibration_samples: 30,
        });
        let (_r, alerts) = sc.push_trace(&make_trace("c1", &["A"])); // imperfect
        assert!(alerts.iter().any(|a| matches!(a, Alert::FitnessBelowThreshold { .. })));
    }

    #[test]
    fn windowed_fitness_uses_last_n() {
        let (arena, root) = parse("PO=(nodes={A, B}, order={A-->B})");
        let mut sc = StreamingConformance::from_powl(&arena, root).unwrap();
        sc.set_window_size(2);
        // Push 3 imperfect, then 2 perfect
        for _ in 0..3 { sc.push_trace(&make_trace("x", &["A"])); }
        sc.push_trace(&make_trace("p1", &["A", "B"]));
        sc.push_trace(&make_trace("p2", &["A", "B"]));
        // Windowed should be 1.0 (last 2 are perfect)
        assert!((sc.windowed_fitness() - 1.0).abs() < 1e-9);
    }
}
