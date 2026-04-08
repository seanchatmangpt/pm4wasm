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

use crate::event_log::EventLog;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

/// Start activities with their frequencies.
///
/// Mirrors `pm4py.get_start_activities()`.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ActivityFrequency {
    pub activity: String,
    pub count: usize,
}

/// All distinct event attribute keys across the log.
///
/// Mirrors `pm4py.get_event_attributes()`.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AttributeSummary {
    pub name: String,
    pub count: usize,
    pub unique_values: usize,
}

/// Variant with its frequency.
///
/// Mirrors `pm4py.get_variants()`.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct VariantInfo {
    pub activities: Vec<String>,
    pub count: usize,
    pub percentage: f64,
}

/// Get the set of start activities and their frequencies.
pub fn get_start_activities(log: &EventLog) -> Vec<ActivityFrequency> {
    let mut freq: HashMap<String, usize> = HashMap::new();
    for trace in &log.traces {
        if let Some(first) = trace.events.first() {
            *freq.entry(first.name.clone()).or_insert(0) += 1;
        }
    }
    let mut result: Vec<ActivityFrequency> = freq
        .into_iter()
        .map(|(activity, count)| ActivityFrequency { activity, count })
        .collect();
    result.sort_by(|a, b| b.count.cmp(&a.count));
    result
}

/// Get the set of end activities and their frequencies.
///
/// Mirrors `pm4py.get_end_activities()`.
pub fn get_end_activities(log: &EventLog) -> Vec<ActivityFrequency> {
    let mut freq: HashMap<String, usize> = HashMap::new();
    for trace in &log.traces {
        if let Some(last) = trace.events.last() {
            *freq.entry(last.name.clone()).or_insert(0) += 1;
        }
    }
    let mut result: Vec<ActivityFrequency> = freq
        .into_iter()
        .map(|(activity, count)| ActivityFrequency { activity, count })
        .collect();
    result.sort_by(|a, b| b.count.cmp(&a.count));
    result
}

/// Get all variants (activity sequences) with frequencies and percentages.
///
/// Mirrors `pm4py.get_variants()`.
pub fn get_variants(log: &EventLog) -> Vec<VariantInfo> {
    let total = log.traces.len();
    let variant_map = log.variants();
    let mut result: Vec<VariantInfo> = variant_map
        .into_iter()
        .map(|(activities, count)| VariantInfo {
            percentage: if total > 0 {
                (count as f64 / total as f64) * 100.0
            } else {
                0.0
            },
            activities,
            count,
        })
        .collect();
    result.sort_by(|a, b| b.count.cmp(&a.count));
    result
}

/// Get all event attribute keys and their statistics.
///
/// Mirrors `pm4py.get_event_attributes()`.
pub fn get_event_attributes(log: &EventLog) -> Vec<AttributeSummary> {
    let mut attr_stats: HashMap<String, (usize, std::collections::HashSet<String>)> =
        HashMap::new();
    for trace in &log.traces {
        for event in &trace.events {
            for key in event.attributes.keys() {
                let (count, values) =
                    attr_stats.entry(key.clone()).or_insert((0, std::collections::HashSet::new()));
                *count += 1;
                if let Some(val) = event.attributes.get(key) {
                    values.insert(val.clone());
                }
            }
        }
    }
    let mut result: Vec<AttributeSummary> = attr_stats
        .into_iter()
        .map(|(name, (count, values))| AttributeSummary {
            name,
            count,
            unique_values: values.len(),
        })
        .collect();
    result.sort_by(|a, b| b.count.cmp(&a.count));
    result
}

/// Get all trace attribute keys.
///
/// Mirrors `pm4py.get_trace_attributes()`.
pub fn get_trace_attributes(log: &EventLog) -> Vec<AttributeSummary> {
    // For now, trace attributes are limited to case_id.
    // Future: extend Trace struct with trace-level attributes.
    vec![AttributeSummary {
        name: "case_id".to_string(),
        count: log.traces.len(),
        unique_values: log
            .traces
            .iter()
            .map(|t| t.case_id.clone())
            .collect::<std::collections::HashSet<_>>()
            .len(),
    }]
}

/// Get case attributes (number of events per trace).
///
/// Mirrors `pm4py.get_case_attributes()`.
pub fn get_case_attributes(log: &EventLog) -> Vec<AttributeSummary> {
    // case_id is always available
    let mut result = vec![AttributeSummary {
        name: "case_id".to_string(),
        count: log.traces.len(),
        unique_values: log
            .traces
            .iter()
            .map(|t| t.case_id.clone())
            .collect::<std::collections::HashSet<_>>()
            .len(),
    }];

    // Count number of events as a synthetic attribute
    result.push(AttributeSummary {
        name: "event_count".to_string(),
        count: log.traces.len(),
        unique_values: log
            .traces
            .iter()
            .map(|t| t.events.len().to_string())
            .collect::<std::collections::HashSet<_>>()
            .len(),
    });

    result
}

/// Get all case durations in milliseconds.
///
/// Returns a vector of (case_id, duration_ms) pairs.
pub fn get_case_durations(log: &EventLog) -> Vec<CaseDuration> {
    let mut result = Vec::new();
    for trace in &log.traces {
        if trace.events.len() >= 2 {
            let first_ts = trace
                .events
                .first()
                .and_then(|e| e.timestamp.as_ref())
                .and_then(|ts| parse_timestamp(ts));
            let last_ts = trace
                .events
                .last()
                .and_then(|e| e.timestamp.as_ref())
                .and_then(|ts| parse_timestamp(ts));

            if let (Some(first), Some(last)) = (first_ts, last_ts) {
                result.push(CaseDuration {
                    case_id: trace.case_id.clone(),
                    duration_ms: last - first,
                });
            }
        }
    }
    result.sort_by(|a, b| a.duration_ms.cmp(&b.duration_ms));
    result
}

/// Case duration result.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CaseDuration {
    pub case_id: String,
    pub duration_ms: i64,
}

/// Parse an ISO-8601 timestamp to milliseconds since epoch.
pub fn parse_timestamp(ts: &str) -> Option<i64> {
    // Try common formats: full ISO-8601 with timezone, without timezone, date-only, etc.
    if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(ts) {
        return Some(dt.timestamp_millis());
    }
    // ISO-8601 without timezone: 2020-01-01T10:00:00 or 2020-01-01 10:00:00
    if let Ok(dt) = chrono::NaiveDateTime::parse_from_str(ts, "%Y-%m-%dT%H:%M:%S") {
        return Some(dt.and_utc().timestamp_millis());
    }
    if let Ok(dt) = chrono::NaiveDateTime::parse_from_str(ts, "%Y-%m-%d %H:%M:%S") {
        return Some(dt.and_utc().timestamp_millis());
    }
    if let Ok(dt) = chrono::NaiveDate::parse_from_str(ts, "%Y-%m-%d") {
        return dt
            .and_hms_opt(0, 0, 0)
            .map(|dt| dt.and_utc().timestamp_millis());
    }
    None
}

/// Get all distinct values for a given event attribute.
///
/// Mirrors `pm4py.get_attribute_values()`.
pub fn get_attribute_values(log: &EventLog, attribute_key: &str) -> Vec<AttributeValue> {
    let mut freq: HashMap<String, usize> = HashMap::new();
    for trace in &log.traces {
        for event in &trace.events {
            if let Some(val) = event.attributes.get(attribute_key) {
                *freq.entry(val.clone()).or_insert(0) += 1;
            }
            // Also check standard fields
            match attribute_key {
                "concept:name" => {
                    *freq.entry(event.name.clone()).or_insert(0) += 1;
                }
                _ => {}
            }
        }
    }
    let mut result: Vec<AttributeValue> = freq
        .into_iter()
        .map(|(value, count)| AttributeValue { attribute: attribute_key.to_string(), value, count })
        .collect();
    result.sort_by(|a, b| b.count.cmp(&a.count));
    result
}

/// Attribute value with frequency.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AttributeValue {
    pub attribute: String,
    pub value: String,
    pub count: usize,
}

/// Get case durations as JSON (for JS consumption).
///
/// Mirrors `pm4py.get_case_durations()`.
pub fn get_case_durations_json(log: &EventLog) -> Vec<CaseDurationJson> {
    get_case_durations(log)
        .into_iter()
        .map(|d| CaseDurationJson {
            case_id: d.case_id,
            duration_ms: d.duration_ms,
        })
        .collect()
}

/// JSON-friendly case duration.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CaseDurationJson {
    pub case_id: String,
    pub duration_ms: i64,
}

/// Compute self-distances (waiting times) for each activity.
///
/// For each event, the self-distance is the time between this event and the
/// next occurrence of the same activity in the same case.
///
/// Mirrors `pm4py.get_rework_times()`.
pub fn get_rework_times(log: &EventLog) -> Vec<ReworkTime> {
    let mut results = Vec::new();
    for trace in &log.traces {
        let mut activity_indices: HashMap<String, Vec<usize>> = HashMap::new();
        for (idx, event) in trace.events.iter().enumerate() {
            activity_indices
                .entry(event.name.clone())
                .or_default()
                .push(idx);
        }

        for (activity, indices) in &activity_indices {
            for window in indices.windows(2) {
                let first = &trace.events[window[0]];
                let second = &trace.events[window[1]];
                if let (Some(ts1), Some(ts2)) = (
                    first.timestamp.as_ref().and_then(|t| parse_timestamp(t)),
                    second.timestamp.as_ref().and_then(|t| parse_timestamp(t)),
                ) {
                    if ts2 > ts1 {
                        results.push(ReworkTime {
                            case_id: trace.case_id.clone(),
                            activity: activity.clone(),
                            duration_ms: ts2 - ts1,
                        });
                    }
                }
            }
        }
    }
    results
}

/// Rework time result.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ReworkTime {
    pub case_id: String,
    pub activity: String,
    pub duration_ms: i64,
}

/// Minimum self-distance result for an activity.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MinSelfDistance {
    pub activity: String,
    pub min_distance_ms: i64,
}

/// Get minimum self-distances for each activity.
///
/// The minimum self-distance is the minimum time between two consecutive
/// occurrences of the same activity in any trace.
///
/// Mirrors `pm4py.get_minimum_self_distances()`.
pub fn get_minimum_self_distances(log: &EventLog) -> Vec<MinSelfDistance> {
    let mut min_dist: HashMap<String, i64> = HashMap::new();
    for trace in &log.traces {
        for window in trace.events.windows(2) {
            if window[0].name == window[1].name {
                if let (Some(ts1), Some(ts2)) = (
                    window[0].timestamp.as_ref().and_then(|t| parse_timestamp(t)),
                    window[1].timestamp.as_ref().and_then(|t| parse_timestamp(t)),
                ) {
                    if ts2 > ts1 {
                        let dist = ts2 - ts1;
                        let entry = min_dist.entry(window[0].name.clone()).or_insert(i64::MAX);
                        if dist < *entry {
                            *entry = dist;
                        }
                    }
                }
            }
        }
    }
    let mut result: Vec<MinSelfDistance> = min_dist
        .into_iter()
        .filter(|(_, d)| *d != i64::MAX)
        .map(|(activity, min_distance_ms)| MinSelfDistance { activity, min_distance_ms })
        .collect();
    result.sort_by(|a, b| a.activity.cmp(&b.activity));
    result
}

/// All case durations as a flat JSON array (legacy API).
///
/// Mirrors `pm4py.get_all_case_durations()`.
pub fn get_all_case_durations(log: &EventLog) -> Vec<f64> {
    get_case_durations(log)
        .into_iter()
        .map(|d| d.duration_ms as f64)
        .collect()
}

/// Get case overlap (fraction of shared prefixes between traces).
///
/// Measures how much traces overlap in their first N events.
///
/// Mirrors `pm4py.get_case_overlap()`.
pub fn get_case_overlap(log: &EventLog) -> f64 {
    if log.traces.len() <= 1 {
        return 1.0;
    }
    let total = log.traces.len();
    let min_len = log.traces.iter().map(|t| t.events.len()).min().unwrap_or(0);
    if min_len == 0 {
        return 0.0;
    }

    let mut prefix_counts: HashMap<Vec<String>, usize> = HashMap::new();
    for trace in &log.traces {
        let prefix: Vec<String> = trace.events.iter().take(min_len).map(|e| e.name.clone()).collect();
        *prefix_counts.entry(prefix).or_insert(0) += 1;
    }

    // Overlap = sum of (count/total)^2 for each prefix
    let overlap: f64 = prefix_counts.values().map(|&c| (c as f64 / total as f64).powi(2)).sum();
    overlap
}

/// Get all prefixes (partial traces) from the log.
///
/// Returns unique prefixes with their frequency.
///
/// Mirrors `pm4py.get_prefixes_from_log()`.
pub fn get_prefixes_from_log(log: &EventLog) -> Vec<PrefixInfo> {
    let mut prefix_counts: HashMap<Vec<String>, usize> = HashMap::new();
    for trace in &log.traces {
        for len in 1..=trace.events.len() {
            let prefix: Vec<String> = trace.events[..len].iter().map(|e| e.name.clone()).collect();
            *prefix_counts.entry(prefix).or_insert(0) += 1;
        }
    }
    let total_prefixes: usize = prefix_counts.values().sum();
    let mut result: Vec<PrefixInfo> = prefix_counts
        .into_iter()
        .map(|(prefix, count)| PrefixInfo {
            prefix,
            count,
            percentage: if total_prefixes > 0 { count as f64 / total_prefixes as f64 * 100.0 } else { 0.0 },
        })
        .collect();
    result.sort_by(|a, b| b.count.cmp(&a.count));
    result
}

/// Prefix info with frequency.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PrefixInfo {
    pub prefix: Vec<String>,
    pub count: usize,
    pub percentage: f64,
}

/// Get trace attribute values.
///
/// Mirrors `pm4py.get_trace_attribute_values()`.
pub fn get_trace_attribute_values(log: &EventLog, attribute_key: &str) -> Vec<AttributeValue> {
    let mut freq: HashMap<String, usize> = HashMap::new();
    for trace in &log.traces {
        let val = match attribute_key {
            "concept:name" => trace.case_id.clone(),
            _ => String::new(),
        };
        if !val.is_empty() {
            *freq.entry(val).or_insert(0) += 1;
        }
    }
    let mut result: Vec<AttributeValue> = freq
        .into_iter()
        .map(|(value, count)| AttributeValue { attribute: attribute_key.to_string(), value, count })
        .collect();
    result.sort_by(|a, b| b.count.cmp(&a.count));
    result
}

/// Get variants as tuples (activity sequences with count).
///
/// Mirrors `pm4py.get_variants_as_tuples()`.
pub fn get_variants_as_tuples(log: &EventLog) -> Vec<VariantTuple> {
    log.variants()
        .into_iter()
        .map(|(activities, count)| VariantTuple { activities, count })
        .collect()
}

/// Variant tuple.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct VariantTuple {
    pub activities: Vec<String>,
    pub count: usize,
}

/// Get variants with path durations.
///
/// Returns each variant with its total, min, max, and avg duration.
///
/// Mirrors `pm4py.get_variants_paths_duration()`.
pub fn get_variants_paths_duration(log: &EventLog) -> Vec<VariantDuration> {
    let variant_map = log.variants();
    // Group durations by variant
    let mut durations_by_variant: HashMap<Vec<String>, Vec<i64>> = HashMap::new();
    for trace in &log.traces {
        let seq: Vec<String> = trace.events.iter().map(|e| e.name.clone()).collect();
        if let (Some(first_ts), Some(last_ts)) = (
            trace.events.first().and_then(|e| e.timestamp.as_ref()).and_then(|t| parse_timestamp(t)),
            trace.events.last().and_then(|e| e.timestamp.as_ref()).and_then(|t| parse_timestamp(t)),
        ) {
            if last_ts > first_ts {
                durations_by_variant
                    .entry(seq)
                    .or_default()
                    .push(last_ts - first_ts);
            }
        }
    }

    variant_map
        .into_iter()
        .map(|(activities, count)| {
            let durs = durations_by_variant.get(&activities).map(|v| v.as_slice()).unwrap_or(&[]);
            VariantDuration {
                activities,
                count,
                total_duration_ms: durs.iter().sum::<i64>(),
                min_duration_ms: durs.first().copied().unwrap_or(0),
                max_duration_ms: durs.last().copied().unwrap_or(0),
                avg_duration_ms: if !durs.is_empty() {
                    durs.iter().copied().map(|d| d as f64).sum::<f64>() / durs.len() as f64
                } else { 0.0 },
            }
        })
        .collect()
}

/// Variant with duration stats.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct VariantDuration {
    pub activities: Vec<String>,
    pub count: usize,
    pub total_duration_ms: i64,
    pub min_duration_ms: i64,
    pub max_duration_ms: i64,
    pub avg_duration_ms: f64,
}

/// Get cases per activity that show rework (activity appears more than once).
///
/// Mirrors `pm4py.get_rework_cases_per_activity()`.
pub fn get_rework_cases_per_activity(log: &EventLog) -> Vec<ReworkActivity> {
    let mut activity_rework: HashMap<String, usize> = HashMap::new();
    let mut activity_total: HashMap<String, usize> = HashMap::new();

    for trace in &log.traces {
        let mut seen: HashSet<String> = HashSet::new();
        for event in &trace.events {
            if seen.contains(&event.name) {
                *activity_rework.entry(event.name.clone()).or_insert(0) += 1;
            }
            seen.insert(event.name.clone());
            *activity_total.entry(event.name.clone()).or_insert(0) += 1;
        }
    }

    let all_activities: Vec<String> = activity_total.keys().cloned().collect();
    let mut result: Vec<ReworkActivity> = all_activities
        .into_iter()
        .map(|activity| {
            let rework = *activity_rework.get(&activity).unwrap_or(&0);
            let total = *activity_total.get(&activity).unwrap_or(&0);
            ReworkActivity {
                activity,
                rework_cases: rework,
                total_cases: total,
            }
        })
        .collect();
    result.sort_by(|a, b| b.rework_cases.cmp(&a.rework_cases));
    result
}

/// Rework activity info.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ReworkActivity {
    pub activity: String,
    pub rework_cases: usize,
    pub total_cases: usize,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::event_log::parse_csv;

    fn make_test_log() -> EventLog {
        parse_csv(
            "case_id,activity,timestamp\n\
             1,A,2020-01-01T10:00:00\n\
             1,B,2020-01-01T10:05:00\n\
             2,A,2020-01-01T11:00:00\n\
             2,B,2020-01-01T11:03:00\n\
             2,C,2020-01-01T11:10:00\n\
             3,A,2020-01-02T09:00:00\n\
             3,C,2020-01-02T09:30:00\n",
        )
        .unwrap()
    }

    #[test]
    fn test_start_activities() {
        let log = make_test_log();
        let starts = get_start_activities(&log);
        assert_eq!(starts.len(), 1);
        assert_eq!(starts[0].activity, "A");
        assert_eq!(starts[0].count, 3);
    }

    #[test]
    fn test_end_activities() {
        let log = make_test_log();
        let ends = get_end_activities(&log);
        assert_eq!(ends.len(), 2);
        // B appears as end in case 1, C appears as end in cases 2 and 3
        let b_end = ends.iter().find(|e| e.activity == "B");
        let c_end = ends.iter().find(|e| e.activity == "C");
        assert!(b_end.is_some());
        assert!(c_end.is_some());
        assert_eq!(b_end.unwrap().count, 1);
        assert_eq!(c_end.unwrap().count, 2);
    }

    #[test]
    fn test_variants() {
        let log = make_test_log();
        let variants = get_variants(&log);
        assert_eq!(variants.len(), 3); // A,B | A,B,C | A,C
        // Each variant has count 1
        for v in &variants {
            assert_eq!(v.count, 1);
        }
        // Verify all three variants exist
        let has_ab = variants.iter().any(|v| v.activities == vec!["A", "B"]);
        let has_abc = variants.iter().any(|v| v.activities == vec!["A", "B", "C"]);
        let has_ac = variants.iter().any(|v| v.activities == vec!["A", "C"]);
        assert!(has_ab, "variant A,B missing");
        assert!(has_abc, "variant A,B,C missing");
        assert!(has_ac, "variant A,C missing");
    }

    #[test]
    fn test_case_durations() {
        let log = make_test_log();
        let durations = get_case_durations(&log);
        assert_eq!(durations.len(), 3);
        // Sorted by duration: case1=5min, case2=10min, case3=30min
        assert_eq!(durations[0].duration_ms, 300_000); // case 1: 5 min
        assert_eq!(durations[1].duration_ms, 600_000); // case 2: 10 min (11:10 - 11:00)
        assert_eq!(durations[2].duration_ms, 1_800_000); // case 3: 30 min
    }
}
