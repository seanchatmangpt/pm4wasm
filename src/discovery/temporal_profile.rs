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

//! Temporal profile discovery and conformance.
//!
//! A temporal profile records, for every directly-follows pair (A→B) in an
//! event log, the mean and standard deviation of the elapsed time (ms).
//! Conformance checking flags edges whose observed duration deviates more than
//! `zeta` standard deviations from the mean.

use crate::event_log::EventLog;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Parse ISO-8601 timestamp string to milliseconds since epoch.
fn parse_timestamp_ms(ts: &str) -> Option<i64> {
    // Simple ISO-8601 parser for formats like "2024-01-01T10:00:00Z"
    // This is a simplified version - for production use chrono or similar
    let ts = ts.trim().replace('Z', "+00:00");

    // Parse the date and time
    let parts: Vec<&str> = ts.split('T').collect();
    if parts.len() != 2 {
        return None;
    }

    let date_parts: Vec<i64> = parts[0].split('-').filter_map(|s| s.parse().ok()).collect();
    if date_parts.len() != 3 {
        return None;
    }

    let time_parts: Vec<&str> = parts[1].split(':').collect();
    if time_parts.len() < 2 {
        return None;
    }

    let hour: i64 = time_parts[0].parse().ok()?;
    let minute: i64 = time_parts[1].parse().ok()?;
    let second: f64 = time_parts.get(2)
        .and_then(|s| s.split('+').next())
        .and_then(|s| s.parse().ok())
        .unwrap_or(0.0);

    // Simplified calculation (ignoring timezone for now)
    // For accurate parsing, use chrono crate
    let year = date_parts[0];
    let month = date_parts[1];
    let day = date_parts[2];
    let days_from_epoch = (year - 1970) * 365 + (month - 1) * 30 + (day - 1);
    let ms = days_from_epoch * 86400000_i64
        + hour * 3600000_i64
        + minute * 60000_i64
        + (second * 1000.0) as i64;

    Some(ms)
}

/// Get timestamp in milliseconds from an event, defaulting to 0 if missing.
fn event_timestamp_ms(event: &crate::event_log::Event) -> i64 {
    event.timestamp
        .as_ref()
        .and_then(|ts| parse_timestamp_ms(ts))
        .unwrap_or(0)
}

/// A temporal profile entry for a directly-follows pair.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TemporalPair {
    /// Mean duration in milliseconds
    pub mean_ms: f64,
    /// Standard deviation in milliseconds
    pub stdev_ms: f64,
    /// Number of observations
    pub count: usize,
}

/// A temporal profile for an event log.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TemporalProfile {
    /// Map: (activity_from, activity_to) -> temporal statistics
    pub pairs: HashMap<(String, String), TemporalPair>,
}

/// Temporal conformance deviation.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TemporalDeviation {
    /// Case identifier
    pub case_id: String,
    /// Source activity
    pub from: String,
    /// Target activity
    pub to: String,
    /// Observed duration in milliseconds
    pub duration_ms: i64,
    /// Expected mean duration in milliseconds
    pub mean_ms: f64,
    /// Expected standard deviation in milliseconds
    pub stdev_ms: f64,
    /// Number of standard deviations from mean
    pub zeta: f64,
    /// Whether this is a deviation (|zeta| > threshold)
    pub deviation: bool,
}

/// Temporal conformance result.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TemporalConformance {
    /// Total traces checked
    pub total_traces: usize,
    /// Total steps checked
    pub total_steps: usize,
    /// Number of deviations
    pub deviations: usize,
    /// Overall fitness (1.0 - deviations/steps)
    pub fitness: f64,
    /// Detailed deviation information
    pub details: Vec<TemporalDeviation>,
}

/// Discover a temporal profile from an event log.
///
/// For each directly-follows pair (A→B), computes the mean and standard
/// deviation of the time elapsed between A and B across all traces.
///
/// # Arguments
/// * `log` - Event log to analyze
///
/// # Returns
/// Temporal profile with statistics for each directly-follows pair
pub fn discover_temporal_profile(log: &EventLog) -> TemporalProfile {
    let mut acc: HashMap<(String, String), (f64, f64, usize)> = HashMap::new();

    for trace in &log.traces {
        let events = &trace.events;
        for i in 0..events.len().saturating_sub(1) {
            let t1 = event_timestamp_ms(&events[i]);
            let t2 = event_timestamp_ms(&events[i + 1]);

            if t2 > t1 {
                let dur = (t2 - t1) as f64;
                let key = (events[i].name.clone(), events[i + 1].name.clone());
                let entry = acc.entry(key).or_insert((0.0, 0.0, 0));
                entry.0 += dur;
                entry.1 += dur * dur;
                entry.2 += 1;
            }
        }
    }

    let mut pairs = HashMap::new();
    for ((a, b), (sum, sum_sq, cnt)) in acc {
        let mean = sum / cnt as f64;
        let variance = (sum_sq / cnt as f64) - mean * mean;
        let stdev = variance.max(0.0).sqrt();
        pairs.insert(
            (a, b),
            TemporalPair {
                mean_ms: mean,
                stdev_ms: stdev,
                count: cnt,
            },
        );
    }

    TemporalProfile { pairs }
}

/// Check a log against a temporal profile.
///
/// Every directly-follows step in every trace is measured. A step is flagged
/// as a deviation when `|duration - mean| > zeta * stdev`.
///
/// # Arguments
/// * `log` - Event log to check
/// * `profile` - Temporal profile to check against
/// * `zeta` - Number of standard deviations for threshold (typically 2.0 or 3.0)
///
/// # Returns
/// Temporal conformance result with fitness and deviations
pub fn check_temporal_conformance(
    log: &EventLog,
    profile: &TemporalProfile,
    zeta: f64,
) -> TemporalConformance {
    let mut deviations = Vec::new();
    let mut total_steps = 0;

    for trace in &log.traces {
        let case_id = trace.case_id.clone();
        let events = &trace.events;

        for i in 0..events.len().saturating_sub(1) {
            let from = &events[i].name;
            let to = &events[i + 1].name;
            let t1 = event_timestamp_ms(&events[i]);
            let t2 = event_timestamp_ms(&events[i + 1]);

            if let Some(pair) = profile.pairs.get(&(from.clone(), to.clone())) {
                if t2 > t1 {
                    total_steps += 1;
                    let duration_ms = t2 - t1;
                    let zeta_value = if pair.stdev_ms > 0.0 {
                        ((duration_ms as f64) - pair.mean_ms).abs() / pair.stdev_ms
                    } else {
                        0.0
                    };

                    let is_deviation = zeta_value > zeta;

                    if is_deviation {
                        deviations.push(TemporalDeviation {
                            case_id: case_id.clone(),
                            from: from.clone(),
                            to: to.clone(),
                            duration_ms,
                            mean_ms: pair.mean_ms,
                            stdev_ms: pair.stdev_ms,
                            zeta: zeta_value,
                            deviation: true,
                        });
                    }
                }
            }
        }
    }

    let total_traces = log.traces.len();
    let num_deviations = deviations.len();
    let fitness = if total_steps > 0 {
        1.0 - (num_deviations as f64 / total_steps as f64)
    } else {
        1.0
    };

    TemporalConformance {
        total_traces,
        total_steps,
        deviations: num_deviations,
        fitness,
        details: deviations,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::event_log::parse_csv;

    #[test]
    fn test_temporal_profile_simple() {
        let csv = "case_id,activity,time:timestamp\n\
                   1,A,2024-01-01T10:00:00Z\n\
                   1,B,2024-01-01T10:01:00Z\n\
                   2,A,2024-01-01T11:00:00Z\n\
                   2,B,2024-01-01T11:02:00Z";
        let log = parse_csv(csv).unwrap();
        let profile = discover_temporal_profile(&log);

        assert!(profile.pairs.contains_key(&("A".to_string(), "B".to_string())));
        let pair = &profile.pairs[&("A".to_string(), "B".to_string())];
        assert_eq!(pair.count, 2);
        // Mean should be around 90 seconds (60000ms + 120000ms) / 2 = 90000ms
        assert!((pair.mean_ms - 90000.0).abs() < 1000.0);
    }

    #[test]
    fn test_temporal_conformance_no_deviations() {
        let csv = "case_id,activity,time:timestamp\n\
                   1,A,2024-01-01T10:00:00Z\n\
                   1,B,2024-01-01T10:01:00Z\n\
                   2,A,2024-01-01T11:00:00Z\n\
                   2,B,2024-01-01T11:01:00Z";
        let log = parse_csv(csv).unwrap();
        let profile = discover_temporal_profile(&log);
        let result = check_temporal_conformance(&log, &profile, 2.0);

        assert_eq!(result.total_traces, 2);
        assert_eq!(result.total_steps, 2);
        assert_eq!(result.deviations, 0);
        assert_eq!(result.fitness, 1.0);
    }
}
