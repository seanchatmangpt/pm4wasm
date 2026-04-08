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

//! Performance spectrum discovery.
//!
//! For a given target activity, measures the time duration between each
//! occurrence of that activity and the next activity in the trace.  Results
//! are grouped by `(target_activity, next_activity)` pair with aggregate
//! statistics (min, max, mean, median, count).
//!
//! Mirrors `pm4py.algo.discovery.performance_spectrum.variants.log`.

use crate::event_log::EventLog;
use chrono::DateTime;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ─── Public types ──────────────────────────────────────────────────────────

/// Aggregate performance measurements for one directly-follows pair.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ActivityPerformance {
    /// The target activity (the activity being analysed).
    pub activity: String,
    /// The activity that directly follows the target.
    pub next_activity: String,
    /// Number of observed occurrences of this pair.
    pub count: usize,
    /// Minimum duration in milliseconds.
    pub min_duration_ms: f64,
    /// Maximum duration in milliseconds.
    pub max_duration_ms: f64,
    /// Mean (average) duration in milliseconds.
    pub mean_duration_ms: f64,
    /// Median duration in milliseconds.
    pub median_duration_ms: f64,
}

/// Full performance spectrum result for a target activity.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PerformanceSpectrumResult {
    /// Per-pair performance measurements, sorted by next_activity name.
    pub measurements: Vec<ActivityPerformance>,
    /// The activity that was analysed.
    pub target_activity: String,
}

// ─── Core algorithm ────────────────────────────────────────────────────────

/// Discover the performance spectrum for `activity` in the given event log.
///
/// For every trace, each occurrence of `activity` is paired with the
/// immediately following event (if any).  The time difference between the
/// two timestamps is recorded.  Events without parseable timestamps are
/// silently skipped.
///
/// Returns an empty `measurements` vector when no valid pairs are found.
pub fn discover_performance_spectrum(
    log: &EventLog,
    activity: &str,
) -> PerformanceSpectrumResult {
    // Collect raw durations per (activity, next_activity) pair.
    let mut buckets: HashMap<(String, String), Vec<f64>> = HashMap::new();

    for trace in &log.traces {
        let events = &trace.events;
        for i in 0..events.len() {
            if events[i].name != activity {
                continue;
            }
            // Need a next event with a timestamp.
            let next_idx = i + 1;
            if next_idx >= events.len() {
                continue;
            }
            let ts_start = parse_timestamp(&events[i].timestamp);
            let ts_end = parse_timestamp(&events[next_idx].timestamp);
            match (ts_start, ts_end) {
                (Some(start), Some(end)) => {
                    let duration_ms = (end - start).num_milliseconds() as f64;
                    let key = (activity.to_string(), events[next_idx].name.clone());
                    buckets.entry(key).or_default().push(duration_ms);
                }
                _ => continue,
            }
        }
    }

    // Compute aggregate statistics per bucket.
    let mut measurements: Vec<ActivityPerformance> = buckets
        .into_iter()
        .map(|((act, next_act), mut durations)| {
            durations.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
            let count = durations.len();
            let min_d = durations.first().copied().unwrap_or(0.0);
            let max_d = durations.last().copied().unwrap_or(0.0);
            let sum: f64 = durations.iter().sum();
            let mean_d = if count > 0 { sum / count as f64 } else { 0.0 };
            let median_d = if count > 0 {
                let mid = count / 2;
                if count % 2 == 0 && count >= 2 {
                    (durations[mid - 1] + durations[mid]) / 2.0
                } else {
                    durations[mid]
                }
            } else {
                0.0
            };
            ActivityPerformance {
                activity: act,
                next_activity: next_act,
                count,
                min_duration_ms: min_d,
                max_duration_ms: max_d,
                mean_duration_ms: mean_d,
                median_duration_ms: median_d,
            }
        })
        .collect();

    measurements.sort_by(|a, b| a.next_activity.cmp(&b.next_activity));

    PerformanceSpectrumResult {
        measurements,
        target_activity: activity.to_string(),
    }
}

// ─── Helpers ───────────────────────────────────────────────────────────────

/// Try to parse a timestamp string into a `DateTime<chrono::Utc>`.
///
/// Accepts common ISO-8601 variants including:
/// - `2020-01-01T00:00:00Z`
/// - `2020-01-01T00:00:00+00:00`
/// - `2020-01-01T00:00:00`
/// - `2020-01-01 00:00:00`
///
/// Returns `None` for `None` inputs or unparseable strings.
fn parse_timestamp(ts: &Option<String>) -> Option<DateTime<chrono::Utc>> {
    let ts_str = (*ts).as_deref()?;
    if ts_str.is_empty() {
        return None;
    }
    // Try RFC 3339 / ISO 8601 with timezone first.
    if let Ok(dt) = DateTime::parse_from_rfc3339(ts_str) {
        return Some(dt.with_timezone(&chrono::Utc));
    }
    // Try without timezone (assume UTC).
    if let Ok(dt) = DateTime::parse_from_str(ts_str, "%Y-%m-%dT%H:%M:%S") {
        return Some(dt.with_timezone(&chrono::Utc));
    }
    // Try space-separated variant.
    if let Ok(dt) = DateTime::parse_from_str(ts_str, "%Y-%m-%d %H:%M:%S") {
        return Some(dt.with_timezone(&chrono::Utc));
    }
    // Try date-only (midnight UTC).
    if let Ok(dt) = DateTime::parse_from_str(ts_str, "%Y-%m-%d") {
        return Some(dt.with_timezone(&chrono::Utc));
    }
    None
}

// ─── Tests ─────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::event_log::{Event, Trace};

    fn make_event(name: &str, timestamp: &str) -> Event {
        Event {
            name: name.to_string(),
            timestamp: Some(timestamp.to_string()),
            lifecycle: None,
            attributes: HashMap::new(),
        }
    }

    fn make_event_no_ts(name: &str) -> Event {
        Event {
            name: name.to_string(),
            timestamp: None,
            lifecycle: None,
            attributes: HashMap::new(),
        }
    }

    #[test]
    fn test_single_pair_basic_stats() {
        // Two traces: A->B with known durations of 1000ms and 3000ms.
        let log = EventLog {
            traces: vec![
                Trace {
                    case_id: "c1".to_string(),
                    events: vec![
                        make_event("A", "2020-01-01T00:00:00Z"),
                        make_event("B", "2020-01-01T00:00:01Z"),
                    ],
                },
                Trace {
                    case_id: "c2".to_string(),
                    events: vec![
                        make_event("A", "2020-01-01T01:00:00Z"),
                        make_event("B", "2020-01-01T01:00:03Z"),
                    ],
                },
            ],
        };
        let result = discover_performance_spectrum(&log, "A");
        assert_eq!(result.target_activity, "A");
        assert_eq!(result.measurements.len(), 1);
        let m = &result.measurements[0];
        assert_eq!(m.next_activity, "B");
        assert_eq!(m.count, 2);
        assert_eq!(m.min_duration_ms, 1000.0);
        assert_eq!(m.max_duration_ms, 3000.0);
        assert_eq!(m.mean_duration_ms, 2000.0);
        assert_eq!(m.median_duration_ms, 2000.0);
    }

    #[test]
    fn test_median_odd_count() {
        // Three durations: 1000, 2000, 5000 -> median should be 2000.
        let log = EventLog {
            traces: vec![
                Trace {
                    case_id: "c1".to_string(),
                    events: vec![
                        make_event("A", "2020-01-01T00:00:00Z"),
                        make_event("B", "2020-01-01T00:00:01Z"), // 1000ms
                    ],
                },
                Trace {
                    case_id: "c2".to_string(),
                    events: vec![
                        make_event("A", "2020-01-01T01:00:00Z"),
                        make_event("B", "2020-01-01T01:00:02Z"), // 2000ms
                    ],
                },
                Trace {
                    case_id: "c3".to_string(),
                    events: vec![
                        make_event("A", "2020-01-01T02:00:00Z"),
                        make_event("B", "2020-01-01T02:00:05Z"), // 5000ms
                    ],
                },
            ],
        };
        let result = discover_performance_spectrum(&log, "A");
        assert_eq!(result.measurements[0].count, 3);
        assert_eq!(result.measurements[0].median_duration_ms, 2000.0);
    }

    #[test]
    fn test_multiple_next_activities() {
        // A->B and A->C pairs.
        let log = EventLog {
            traces: vec![
                Trace {
                    case_id: "c1".to_string(),
                    events: vec![
                        make_event("A", "2020-01-01T00:00:00Z"),
                        make_event("B", "2020-01-01T00:00:02Z"),
                    ],
                },
                Trace {
                    case_id: "c2".to_string(),
                    events: vec![
                        make_event("A", "2020-01-01T01:00:00Z"),
                        make_event("C", "2020-01-01T01:00:10Z"),
                    ],
                },
            ],
        };
        let result = discover_performance_spectrum(&log, "A");
        assert_eq!(result.measurements.len(), 2);
        // Sorted by next_activity: B comes before C.
        assert_eq!(result.measurements[0].next_activity, "B");
        assert_eq!(result.measurements[0].mean_duration_ms, 2000.0);
        assert_eq!(result.measurements[1].next_activity, "C");
        assert_eq!(result.measurements[1].mean_duration_ms, 10000.0);
    }

    #[test]
    fn test_missing_timestamps_skipped() {
        // Events without timestamps should be silently skipped.
        let log = EventLog {
            traces: vec![Trace {
                case_id: "c1".to_string(),
                events: vec![
                    make_event_no_ts("A"),
                    make_event("B", "2020-01-01T00:00:05Z"),
                ],
            }],
        };
        let result = discover_performance_spectrum(&log, "A");
        assert_eq!(result.measurements.len(), 0);
    }

    #[test]
    fn test_no_occurrences() {
        let log = EventLog {
            traces: vec![Trace {
                case_id: "c1".to_string(),
                events: vec![
                    make_event("X", "2020-01-01T00:00:00Z"),
                    make_event("Y", "2020-01-01T00:00:01Z"),
                ],
            }],
        };
        let result = discover_performance_spectrum(&log, "A");
        assert!(result.measurements.is_empty());
    }

    #[test]
    fn test_multiple_occurrences_in_one_trace() {
        // A appears twice in one trace, each followed by a different activity.
        let log = EventLog {
            traces: vec![Trace {
                case_id: "c1".to_string(),
                events: vec![
                    make_event("A", "2020-01-01T00:00:00Z"),
                    make_event("B", "2020-01-01T00:00:01Z"),
                    make_event("A", "2020-01-01T00:00:05Z"),
                    make_event("C", "2020-01-01T00:00:06Z"),
                ],
            }],
        };
        let result = discover_performance_spectrum(&log, "A");
        assert_eq!(result.measurements.len(), 2);
    }
}
