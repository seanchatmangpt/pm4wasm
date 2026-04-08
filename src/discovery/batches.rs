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

//! Batch processing pattern detection for event logs.
//!
//! Identifies four types of batch processing based on temporal overlap
//! of activity executions across cases, following Martin et al. (2015).

use crate::event_log::EventLog;
use chrono::{DateTime, FixedOffset};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

/// Classification of a detected batch pattern.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum BatchType {
    /// End of one execution equals start of the next.
    Sequential,
    /// Overlapping executions that are not sequential or parallel.
    Concurrent,
    /// Identical start and end timestamps across all executions.
    Parallel,
    /// Large overlapping batch that disrupts normal flow.
    Disruptive,
}

/// A single detected batch instance.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BatchInstance {
    pub activity: String,
    pub batch_type: BatchType,
    pub case_ids: Vec<String>,
    pub start_time: Option<String>,
    pub end_time: Option<String>,
    pub size: usize,
}

/// Aggregated result of batch detection across all activities.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BatchDetectionResult {
    pub batches: Vec<BatchInstance>,
    pub total_batches: usize,
}

// ─── Internal types ────────────────────────────────────────────────────────────

#[derive(Clone, Debug)]
struct Execution {
    start_ts: i64,
    end_ts: i64,
    start_str: String,
    end_str: String,
    case_id: String,
}

#[derive(Clone, Debug)]
struct Interval {
    start_ts: i64,
    end_ts: i64,
    start_str: String,
    end_str: String,
    case_ids: BTreeSet<String>,
}

impl Ord for Interval {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.start_ts.cmp(&other.start_ts)
    }
}
impl PartialOrd for Interval {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}
impl Eq for Interval {}
impl PartialEq for Interval {
    fn eq(&self, other: &Self) -> bool {
        self.start_ts == other.start_ts && self.end_ts == other.end_ts
    }
}

const MERGE_DISTANCE_SECS: i64 = 15 * 60;
const MIN_BATCH_SIZE: usize = 2;
const DISRUPTIVE_THRESHOLD: usize = 5;

// ─── Timestamp parsing ────────────────────────────────────────────────────────

fn parse_timestamp(s: &str) -> Option<(i64, String)> {
    let dt: DateTime<FixedOffset> = DateTime::parse_from_rfc3339(s).ok()?;
    Some((dt.timestamp(), s.to_string()))
}

// ─── Interval merging ──────────────────────────────────────────────────────────

/// Merge overlapping time intervals. Two intervals [a,b] and [c,d] overlap if a <= c <= b.
fn merge_overlapping(mut intervals: Vec<Interval>) -> Vec<Interval> {
    intervals.sort();
    let mut merged: Vec<Interval> = Vec::new();
    for interval in intervals {
        if let Some(last) = merged.last_mut() {
            if last.end_ts >= interval.start_ts {
                last.end_ts = last.end_ts.max(interval.end_ts);
                if interval.end_ts > last.end_ts {
                    last.end_str = interval.end_str.clone();
                }
                last.case_ids.extend(interval.case_ids.iter().cloned());
                continue;
            }
        }
        merged.push(interval);
    }
    merged
}

/// Merge non-overlapping intervals closer than `max_distance` seconds.
fn merge_near(mut intervals: Vec<Interval>, max_distance: i64) -> Vec<Interval> {
    intervals.sort();
    let mut merged: Vec<Interval> = Vec::new();
    for interval in intervals {
        if let Some(last) = merged.last_mut() {
            if interval.start_ts - last.end_ts <= max_distance {
                last.end_ts = last.end_ts.max(interval.end_ts);
                if interval.end_ts > last.end_ts {
                    last.end_str = interval.end_str.clone();
                }
                last.case_ids.extend(interval.case_ids.iter().cloned());
                continue;
            }
        }
        merged.push(interval);
    }
    merged
}

// ─── Batch type classification ────────────────────────────────────────────────

/// Classify a merged batch interval into a specific batch type based on the
/// temporal relationship of its constituent executions.
fn classify_batch(activity: &str, interval: &Interval, executions: &[Execution]) -> Option<BatchInstance> {
    let size = interval.case_ids.len();
    if size < MIN_BATCH_SIZE {
        return None;
    }

    let mut batch_execs: Vec<&Execution> = executions
        .iter()
        .filter(|e| interval.case_ids.contains(&e.case_id))
        .collect();
    batch_execs.sort_by_key(|e| e.start_ts);

    let min_start = batch_execs.iter().map(|e| e.start_ts).min().unwrap_or(0);
    let max_start = batch_execs.iter().map(|e| e.start_ts).max().unwrap_or(0);
    let min_end = batch_execs.iter().map(|e| e.end_ts).min().unwrap_or(0);
    let max_end = batch_execs.iter().map(|e| e.end_ts).max().unwrap_or(0);

    let batch_type = if min_start == max_start && min_end == max_end {
        BatchType::Parallel
    } else if min_start == max_start || min_end == max_end {
        BatchType::Concurrent
    } else {
        let is_sequential = batch_execs
            .windows(2)
            .all(|w| w[0].end_ts == w[1].start_ts);
        if is_sequential {
            BatchType::Sequential
        } else if size >= DISRUPTIVE_THRESHOLD {
            BatchType::Disruptive
        } else {
            BatchType::Concurrent
        }
    };

    Some(BatchInstance {
        activity: activity.to_string(),
        batch_type,
        case_ids: interval.case_ids.iter().cloned().collect(),
        start_time: Some(interval.start_str.clone()),
        end_time: Some(interval.end_str.clone()),
        size,
    })
}

// ─── Per-activity detection ────────────────────────────────────────────────────

fn detect_single(activity: &str, mut executions: Vec<Execution>) -> Vec<BatchInstance> {
    if executions.len() < MIN_BATCH_SIZE {
        return Vec::new();
    }
    executions.sort_by_key(|e| e.start_ts);

    let intervals: Vec<Interval> = executions
        .iter()
        .map(|e| {
            let mut cases = BTreeSet::new();
            cases.insert(e.case_id.clone());
            Interval {
                start_ts: e.start_ts,
                end_ts: e.end_ts,
                start_str: e.start_str.clone(),
                end_str: e.end_str.clone(),
                case_ids: cases,
            }
        })
        .collect();

    let merged = merge_near(merge_overlapping(intervals), MERGE_DISTANCE_SECS);

    merged
        .iter()
        .filter_map(|interval| classify_batch(activity, interval, &executions))
        .collect()
}

// ─── Public API ────────────────────────────────────────────────────────────────

/// Discover batch processing patterns in an event log.
///
/// Groups events by activity, then for each activity detects temporal
/// overlaps between executions across different cases. Activities without
/// timestamps are silently skipped.
pub fn discover_batches(log: &EventLog) -> BatchDetectionResult {
    let mut activity_execs: std::collections::BTreeMap<String, Vec<Execution>> =
        std::collections::BTreeMap::new();

    for trace in &log.traces {
        for event in &trace.events {
            let ts = match &event.timestamp {
                Some(s) => match parse_timestamp(s) {
                    Some((epoch, _)) => epoch,
                    None => continue,
                },
                None => continue,
            };
            let ts_str = event.timestamp.clone().unwrap_or_default();
            activity_execs.entry(event.name.clone()).or_default().push(Execution {
                start_ts: ts,
                end_ts: ts,
                start_str: ts_str.clone(),
                end_str: ts_str,
                case_id: trace.case_id.clone(),
            });
        }
    }

    let mut all_batches: Vec<BatchInstance> = Vec::new();
    for (activity, executions) in &activity_execs {
        all_batches.extend(detect_single(activity, executions.clone()));
    }
    all_batches.sort_by(|a, b| b.size.cmp(&a.size));

    BatchDetectionResult {
        total_batches: all_batches.len(),
        batches: all_batches,
    }
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::event_log::{Event, EventLog, Trace};
    use std::collections::HashMap;

    fn ts(minute: i64) -> String {
        format!("2024-01-01T00:{:02}:00+00:00", minute)
    }

    fn event(name: &str, minute: i64) -> Event {
        Event {
            name: name.to_string(),
            timestamp: Some(ts(minute)),
            lifecycle: None,
            attributes: HashMap::new(),
        }
    }

    #[test]
    fn test_empty_log_returns_no_batches() {
        let log = EventLog { traces: vec![] };
        let result = discover_batches(&log);
        assert_eq!(result.total_batches, 0);
        assert!(result.batches.is_empty());
    }

    #[test]
    fn test_log_without_timestamps_returns_no_batches() {
        let traces = vec![Trace {
            case_id: "c1".to_string(),
            events: vec![
                Event { name: "A".into(), timestamp: None, lifecycle: None, attributes: HashMap::new() },
                Event { name: "B".into(), timestamp: None, lifecycle: None, attributes: HashMap::new() },
            ],
        }];
        assert_eq!(discover_batches(&EventLog { traces }).total_batches, 0);
    }

    #[test]
    fn test_detects_sequential_batches() {
        let traces = vec![
            Trace { case_id: "case1".into(), events: vec![event("Check", 0), event("Approve", 3)] },
            Trace { case_id: "case2".into(), events: vec![event("Check", 1), event("Approve", 4)] },
            Trace { case_id: "case3".into(), events: vec![event("Check", 2), event("Approve", 5)] },
        ];
        let result = discover_batches(&EventLog { traces });
        assert!(result.total_batches >= 1);
        let check = result.batches.iter().find(|b| b.activity == "Check").unwrap();
        assert_eq!(check.size, 3);
        assert!(check.case_ids.contains(&"case1".into()));
        assert!(check.case_ids.contains(&"case2".into()));
        assert!(check.case_ids.contains(&"case3".into()));
    }

    #[test]
    fn test_single_event_per_activity_no_batch() {
        let log = EventLog {
            traces: vec![Trace { case_id: "c1".into(), events: vec![event("A", 0), event("B", 1)] }],
        };
        assert_eq!(discover_batches(&log).total_batches, 0);
    }

    #[test]
    fn test_parallel_batch_identical_timestamps() {
        let traces = vec![
            Trace { case_id: "case1".into(), events: vec![event("Print", 10)] },
            Trace { case_id: "case2".into(), events: vec![event("Print", 10)] },
        ];
        let result = discover_batches(&EventLog { traces });
        assert_eq!(result.total_batches, 1);
        assert_eq!(result.batches[0].batch_type, BatchType::Parallel);
        assert_eq!(result.batches[0].size, 2);
    }

    #[test]
    fn test_disruptive_batch_large_size() {
        let traces: Vec<Trace> = (0..6)
            .map(|i| Trace {
                case_id: format!("case{}", i + 1),
                events: vec![event("Ship", i)],
            })
            .collect();
        let result = discover_batches(&EventLog { traces });
        let ship = result.batches.iter().find(|b| b.activity == "Ship").unwrap();
        assert_eq!(ship.size, 6);
        assert_eq!(ship.batch_type, BatchType::Disruptive);
    }
}
