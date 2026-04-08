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

//! Log Skeletons discovery algorithm.
//!
//! Discovers six types of constraints from an event log:
//! - Equivalence: activities always co-occur with same frequency
//! - Always_after: a always followed by b
//! - Always_before: a always preceded by b
//! - Never_together: a and b never in same trace
//! - Directly_follows: a directly followed by b
//! - Activ_freq: allowed activity frequencies
//!
//! Mirrors `pm4py.algo.discovery.log_skeleton`.

use crate::event_log::EventLog;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

/// A log skeleton model with six constraint types.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LogSkeleton {
    /// Equivalence relations: activities that always co-occur with same frequency.
    pub equivalence: Vec<(String, String)>,
    /// Always-after relations: if a occurs, b always occurs after it.
    pub always_after: Vec<(String, String)>,
    /// Always-before relations: if a occurs, b always occurs before it.
    pub always_before: Vec<(String, String)>,
    /// Never-together relations: a and b never appear in the same trace.
    pub never_together: Vec<(String, String)>,
    /// Directly-follows relations: a is always directly followed by b.
    pub directly_follows: Vec<(String, String)>,
    /// Allowed activity frequencies per activity.
    pub activ_freq: HashMap<String, Vec<usize>>,
}

/// Trace skeleton helper functions.
mod trace_skel {
    use std::collections::{HashMap, HashSet};

    /// Get equivalence relations from a single trace.
    /// Activities a,b are equivalent if they have the same frequency.
    pub fn equivalence(trace: &[String]) -> Vec<(String, String)> {
        let freq = activ_freq_single(trace);
        let mut result = Vec::new();

        for (i, a) in trace.iter().enumerate() {
            for (j, b) in trace.iter().enumerate() {
                if i != j {
                    if let (Some(&fa), Some(&fb)) = (freq.get(a), freq.get(b)) {
                        if fa == fb {
                            result.push((a.clone(), b.clone()));
                        }
                    }
                }
            }
        }

        result
    }

    /// Get all "after" relations from a single trace.
    /// Returns all pairs (a,b) where a comes before b in the trace.
    pub fn after(trace: &[String]) -> Vec<(String, String)> {
        let mut result = Vec::new();
        for i in 0..trace.len() {
            for j in (i + 1)..trace.len() {
                result.push((trace[i].clone(), trace[j].clone()));
            }
        }
        result
    }

    /// Get all "before" relations from a single trace.
    /// Returns all pairs (a,b) where a comes after b in the trace.
    pub fn before(trace: &[String]) -> Vec<(String, String)> {
        let mut result = Vec::new();
        for i in 0..trace.len() {
            for j in 0..i {
                result.push((trace[i].clone(), trace[j].clone()));
            }
        }
        result
    }

    /// Get all combinations of activities in a trace.
    pub fn combos(trace: &[String]) -> HashSet<(String, String)> {
        let mut result = HashSet::new();
        for (i, a) in trace.iter().enumerate() {
            for (j, b) in trace.iter().enumerate() {
                if i != j {
                    result.insert((a.clone(), b.clone()));
                }
            }
        }
        result
    }

    /// Get directly-follows relations from a single trace.
    pub fn directly_follows(trace: &[String]) -> Vec<(String, String)> {
        let mut result = Vec::new();
        for i in 0..trace.len().saturating_sub(1) {
            result.push((trace[i].clone(), trace[i + 1].clone()));
        }
        result
    }

    /// Get activity frequencies from a single trace.
    pub fn activ_freq_single(trace: &[String]) -> HashMap<String, usize> {
        let mut freq = HashMap::new();
        for activity in trace {
            *freq.entry(activity.clone()).or_insert(0) += 1;
        }
        freq
    }
}

/// Discover equivalence relations.
fn equivalence(
    logs_traces: &[(Vec<String>, usize)],
    all_activs: &HashMap<String, usize>,
    noise_threshold: f64,
) -> HashSet<(String, String)> {
    let mut ret0: HashMap<(String, String), usize> = HashMap::new();

    for (trace, freq) in logs_traces {
        let eqs = trace_skel::equivalence(trace);
        for pair in eqs {
            *ret0.entry(pair).or_insert(0) += freq;
        }
    }

    ret0.into_iter()
        .filter(|((a, _), count)| {
            all_activs.get(a).map_or(false, |&total| {
                *count as f64 >= total as f64 * (1.0 - noise_threshold)
            })
        })
        .map(|(pair, _)| pair)
        .collect()
}

/// Discover always-after relations.
fn always_after(
    logs_traces: &[(Vec<String>, usize)],
    _all_activs: &HashMap<String, usize>,
    noise_threshold: f64,
) -> HashSet<(String, String)> {
    // Count traces containing each activity
    let mut traces_with_a: HashMap<String, usize> = HashMap::new();
    for (trace, freq) in logs_traces {
        let activities: HashSet<&String> = trace.iter().collect();
        for activity in activities {
            *traces_with_a.entry(activity.clone()).or_insert(0) += freq;
        }
    }

    // Count traces where b appears after a
    let mut traces_with_a_then_b: HashMap<(String, String), usize> = HashMap::new();
    for (trace, freq) in logs_traces {
        let after_pairs: HashSet<_> = trace_skel::after(trace).into_iter().collect();
        for (a, b) in after_pairs {
            *traces_with_a_then_b.entry((a, b)).or_insert(0) += freq;
        }
    }

    // Keep pairs that satisfy the threshold
    traces_with_a_then_b
        .into_iter()
        .filter(|((a, _), count_ab)| {
            traces_with_a.get(a).map_or(false, |&count_a| {
                *count_ab as f64 >= count_a as f64 * (1.0 - noise_threshold)
            })
        })
        .map(|(pair, _)| pair)
        .collect()
}

/// Discover always-before relations.
fn always_before(
    logs_traces: &[(Vec<String>, usize)],
    _all_activs: &HashMap<String, usize>,
    noise_threshold: f64,
) -> HashSet<(String, String)> {
    // Count traces containing each activity
    let mut traces_with_a: HashMap<String, usize> = HashMap::new();
    for (trace, freq) in logs_traces {
        let activities: HashSet<&String> = trace.iter().collect();
        for activity in activities {
            *traces_with_a.entry(activity.clone()).or_insert(0) += freq;
        }
    }

    // Count traces where b appears before a
    let mut traces_with_b_then_a: HashMap<(String, String), usize> = HashMap::new();
    for (trace, freq) in logs_traces {
        let before_pairs: HashSet<_> = trace_skel::before(trace).into_iter().collect();
        for (a, b) in before_pairs {
            *traces_with_b_then_a.entry((a, b)).or_insert(0) += freq;
        }
    }

    // Keep pairs that satisfy the threshold
    traces_with_b_then_a
        .into_iter()
        .filter(|((a, _), count_ba)| {
            traces_with_a.get(a).map_or(false, |&count_a| {
                *count_ba as f64 >= count_a as f64 * (1.0 - noise_threshold)
            })
        })
        .map(|(pair, _)| pair)
        .collect()
}

/// Discover never-together relations.
fn never_together(
    logs_traces: &[(Vec<String>, usize)],
    all_activs: &HashMap<String, usize>,
    _len_log: usize,
    noise_threshold: f64,
) -> HashSet<(String, String)> {
    let all_activities: Vec<&String> = all_activs.keys().collect();

    // Start with all possible pairs
    let mut ret0: HashMap<(String, String), usize> = HashMap::new();
    for (i, a) in all_activities.iter().enumerate() {
        for b in all_activities.iter().skip(i + 1) {
            if a != b {
                ret0.insert(((*a).clone(), (*b).clone()), *all_activs.get(*a).unwrap_or(&0));
            }
        }
    }

    // Subtract pairs that actually appear together
    for (trace, freq) in logs_traces {
        let freq_val = *freq;  // Dereference once to get usize
        let combos = trace_skel::combos(trace);
        for pair in combos {
            // Check both orderings since combos returns ordered pairs
            if let Some(count) = ret0.get_mut(&pair) {
                *count = count.saturating_sub(freq_val);
            } else {
                // Try reversed pair
                let reversed = (pair.1.clone(), pair.0.clone());
                if let Some(count) = ret0.get_mut(&reversed) {
                    *count = count.saturating_sub(freq_val);
                }
            }
        }
    }

    // Keep pairs that satisfy the threshold
    ret0.into_iter()
        .filter(|((a, _), count)| {
            all_activs.get(a).map_or(false, |&total| {
                *count as f64 >= total as f64 * (1.0 - noise_threshold)
            })
        })
        .map(|(pair, _)| pair)
        .collect()
}

/// Discover directly-follows relations.
fn directly_follows(
    logs_traces: &[(Vec<String>, usize)],
    all_activs: &HashMap<String, usize>,
    noise_threshold: f64,
) -> HashSet<(String, String)> {
    let mut ret0: HashMap<(String, String), usize> = HashMap::new();

    for (trace, freq) in logs_traces {
        let df = trace_skel::directly_follows(trace);
        for pair in df {
            *ret0.entry(pair).or_insert(0) += freq;
        }
    }

    ret0.into_iter()
        .filter(|((a, _), count)| {
            all_activs.get(a).map_or(false, |&total| {
                *count as f64 >= total as f64 * (1.0 - noise_threshold)
            })
        })
        .map(|(pair, _)| pair)
        .collect()
}

/// Discover allowed activity frequencies.
fn activ_freq(
    logs_traces: &[(Vec<String>, usize)],
    all_activs: &HashMap<String, usize>,
    len_log: usize,
    noise_threshold: f64,
) -> HashMap<String, Vec<usize>> {
    let mut ret0: HashMap<String, HashMap<usize, usize>> = HashMap::new();

    for (trace, freq) in logs_traces {
        let freq_map = trace_skel::activ_freq_single(trace);

        // Ensure all activities are represented
        for activity in all_activs.keys() {
            let freq_value = *freq_map.get(activity).unwrap_or(&0);
            *ret0
                .entry(activity.clone())
                .or_insert_with(HashMap::new)
                .entry(freq_value)
                .or_insert(0) += freq;
        }
    }

    let mut ret = HashMap::new();
    for activity in all_activs.keys() {
        if let Some(freq_map) = ret0.get(activity) {
            let mut freqs: Vec<(usize, usize)> = freq_map.iter().map(|(&k, &v)| (k, v)).collect();
            freqs.sort_by(|a, b| b.1.cmp(&a.1)); // Sort by count descending

            // Keep top frequencies covering (1.0 - noise) of traces
            let mut added = 0;
            let mut keep_freqs = Vec::new();
            for (freq, count) in freqs {
                added += count;
                keep_freqs.push(freq);
                if added as f64 >= len_log as f64 * (1.0 - noise_threshold) {
                    break;
                }
            }
            ret.insert(activity.clone(), keep_freqs);
        }
    }

    ret
}

/// Discover a log skeleton from an event log.
///
/// Mirrors `pm4py.discover_log_skeleton()`.
///
/// # Arguments
/// * `log` - Event log to analyze
/// * `noise_threshold` - Noise threshold (0.0 = strict, 0.1 = tolerant)
///
/// # Returns
/// Log skeleton with all six constraint types
pub fn discover_log_skeleton(log: &EventLog, noise_threshold: f64) -> LogSkeleton {
    // Build variant frequencies: trace sequence -> count
    let mut logs_traces_vec: Vec<(Vec<String>, usize)> = Vec::new();
    let mut logs_traces_map: HashMap<Vec<String>, usize> = HashMap::new();

    for trace in &log.traces {
        let sequence: Vec<String> = trace.events.iter().map(|e| e.name.clone()).collect();
        *logs_traces_map.entry(sequence).or_insert(0) += 1;
    }

    for (sequence, count) in logs_traces_map {
        logs_traces_vec.push((sequence, count));
    }

    // Count all activity occurrences
    let mut all_activs: HashMap<String, usize> = HashMap::new();
    for trace in &log.traces {
        for event in &trace.events {
            *all_activs.entry(event.name.clone()).or_insert(0) += 1;
        }
    }

    let len_log = log.traces.len();

    // Discover all six constraint types
    let equivalence = equivalence(&logs_traces_vec, &all_activs, noise_threshold);
    let always_after = always_after(&logs_traces_vec, &all_activs, noise_threshold);
    let always_before = always_before(&logs_traces_vec, &all_activs, noise_threshold);
    let never_together = never_together(&logs_traces_vec, &all_activs, len_log, noise_threshold);
    let directly_follows = directly_follows(&logs_traces_vec, &all_activs, noise_threshold);
    let activ_freq = activ_freq(&logs_traces_vec, &all_activs, len_log, noise_threshold);

    // Convert to sorted vectors for consistent output
    let mut equivalence: Vec<_> = equivalence.into_iter().collect();
    equivalence.sort();
    let mut always_after: Vec<_> = always_after.into_iter().collect();
    always_after.sort();
    let mut always_before: Vec<_> = always_before.into_iter().collect();
    always_before.sort();
    let mut never_together: Vec<_> = never_together.into_iter().collect();
    never_together.sort();
    let mut directly_follows: Vec<_> = directly_follows.into_iter().collect();
    directly_follows.sort();

    LogSkeleton {
        equivalence,
        always_after,
        always_before,
        never_together,
        directly_follows,
        activ_freq,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::event_log::parse_csv;

    #[test]
    fn test_log_skeleton_basic() {
        let csv = "case_id,activity\n\
                   1,A\n\
                   1,B\n\
                   2,A\n\
                   2,C\n\
                   3,A\n\
                   3,B";
        let log = parse_csv(csv).unwrap();

        // Test with zero noise (requires 100% confidence)
        let skeleton = discover_log_skeleton(&log, 0.0);
        // A is not always directly followed by the same activity, so directly_follows is empty
        assert!(skeleton.directly_follows.is_empty());

        // Test with noise threshold 0.5 (allows 50% noise)
        let skeleton = discover_log_skeleton(&log, 0.5);
        // With 50% noise tolerance, A->B should appear (2 out of 3 traces)
        assert!(skeleton.directly_follows.iter().any(|(a, b)| a == "A" && b == "B"));
    }

    #[test]
    fn test_never_together() {
        let csv = "case_id,activity\n\
                   1,A\n\
                   1,B\n\
                   2,A\n\
                   2,C";
        let log = parse_csv(csv).unwrap();
        let skeleton = discover_log_skeleton(&log, 0.0);

        // B and C never appear together (order doesn't matter)
        let has_b_never_c = skeleton.never_together.iter().any(|(a, b)| a == "B" && b == "C");
        let has_c_never_b = skeleton.never_together.iter().any(|(a, b)| a == "C" && b == "B");
        assert!(has_b_never_c || has_c_never_b, "B and C should never be together");
    }

    #[test]
    fn test_activ_freq() {
        let csv = "case_id,activity\n\
                   1,A\n\
                   1,B\n\
                   2,A\n\
                   2,B\n\
                   2,B";
        let log = parse_csv(csv).unwrap();
        let skeleton = discover_log_skeleton(&log, 0.0);

        // B appears once in trace 1, twice in trace 2
        assert!(skeleton.activ_freq.contains_key("B"));
    }
}
