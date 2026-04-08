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

/// Footprints-based conformance checking.
///
/// Compares the log's directly-follows graph against a model's footprints
/// to compute fitness, precision, recall, and f1-score.
///
/// Mirrors `pm4py.conformance_diagnostics_footprints()`.
use crate::event_log::EventLog;
use crate::footprints::Footprints;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Footprints conformance result.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FootprintsConformanceResult {
    pub fitness: f64,
    pub precision: f64,
    pub recall: f64,
    pub f1: f64,
}

/// Build the log footprints from the directly-follows graph.
fn log_footprints(log: &EventLog) -> LogFootprints {
    let mut sequence: HashMap<(String, String), usize> = HashMap::new();
    let mut start_activities: HashMap<String, usize> = HashMap::new();
    let mut end_activities: HashMap<String, usize> = HashMap::new();
    let mut activities: std::collections::HashSet<String> = std::collections::HashSet::new();

    for trace in &log.traces {
        let events = &trace.events;
        if let Some(first) = events.first() {
            *start_activities.entry(first.name.clone()).or_insert(0) += 1;
        }
        if let Some(last) = events.last() {
            *end_activities.entry(last.name.clone()).or_insert(0) += 1;
        }
        for event in events {
            activities.insert(event.name.clone());
        }
        for window in events.windows(2) {
            let key = (window[0].name.clone(), window[1].name.clone());
            *sequence.entry(key).or_insert(0) += 1;
        }
    }

    LogFootprints {
        sequence,
        start_activities,
        end_activities,
        activities,
    }
}

#[allow(dead_code)]
struct LogFootprints {
    sequence: HashMap<(String, String), usize>,
    start_activities: HashMap<String, usize>,
    end_activities: HashMap<String, usize>,
    activities: std::collections::HashSet<String>,
}

/// Compute footprints-based conformance metrics.
pub fn check(log: &EventLog, model_fp: &Footprints) -> FootprintsConformanceResult {
    let log_fp = log_footprints(log);

    // Convert model footprints sequence/parallel to sets for comparison
    let model_sequence: std::collections::HashSet<(String, String)> = model_fp.sequence.clone();
    let _model_parallel: std::collections::HashSet<(String, String)> = model_fp.parallel.clone();
    let log_sequence: std::collections::HashSet<(String, String)> =
        log_fp.sequence.keys().cloned().collect();

    // --- Fitness ---
    // Fraction of log's directly-follows pairs that are allowed by the model
    let log_total = log_sequence.len();
    let matching = log_sequence.intersection(&model_sequence).count();
    let fitness = if log_total == 0 {
        1.0
    } else {
        matching as f64 / log_total as f64
    };

    // --- Precision ---
    // Fraction of model's directly-follows pairs that are observed in the log
    let model_total = model_sequence.len();
    let recall_matching = model_sequence.intersection(&log_sequence).count();
    let precision = if model_total == 0 {
        1.0
    } else {
        recall_matching as f64 / model_total as f64
    };

    // --- Recall (same as fitness for footprint comparison) ---
    let recall = fitness;

    // --- F1 ---
    let f1 = if precision + recall == 0.0 {
        0.0
    } else {
        2.0 * precision * recall / (precision + recall)
    };

    FootprintsConformanceResult {
        fitness,
        precision,
        recall,
        f1,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::event_log::{Event, Trace};

    fn make_log(traces: Vec<(&str, &[&str])>) -> EventLog {
        EventLog {
            traces: traces
                .into_iter()
                .map(|(case_id, acts)| Trace {
                    case_id: case_id.to_string(),
                    events: acts
                        .iter()
                        .map(|&a| Event {
                            name: a.to_string(),
                            timestamp: None,
                            lifecycle: None,
                            attributes: std::collections::HashMap::new(),
                        })
                        .collect(),
                })
                .collect(),
        }
    }

    fn make_model_fp(activities: &[&str], sequence: &[(&str, &str)]) -> Footprints {
        let act_set: std::collections::HashSet<String> =
            activities.iter().map(|s| s.to_string()).collect();
        let seq_set: std::collections::HashSet<(String, String)> = sequence
            .iter()
            .map(|(a, b)| (a.to_string(), b.to_string()))
            .collect();
        let start = if !activities.is_empty() {
            [activities[0].to_string()].into_iter().collect()
        } else {
            std::collections::HashSet::new()
        };
        let end = if !activities.is_empty() {
            [activities[activities.len() - 1].to_string()]
                .into_iter()
                .collect()
        } else {
            std::collections::HashSet::new()
        };
        Footprints {
            start_activities: start,
            end_activities: end,
            activities: act_set,
            skippable: false,
            sequence: seq_set,
            parallel: std::collections::HashSet::new(),
            activities_always_happening: std::collections::HashSet::new(),
            min_trace_length: activities.len(),
        }
    }

    #[test]
    fn perfect_conformance() {
        let log = make_log(vec![("1", &["A", "B", "C"]), ("2", &["A", "B", "C"])]);
        let model_fp = make_model_fp(
            &["A", "B", "C"],
            &[("A", "B"), ("B", "C")],
        );
        let result = check(&log, &model_fp);
        assert!((result.fitness - 1.0).abs() < 1e-9);
        assert!((result.precision - 1.0).abs() < 1e-9);
    }

    #[test]
    fn imperfect_fitness_extra_pair() {
        let log = make_log(vec![("1", &["A", "B", "C", "A"])]);
        let model_fp = make_model_fp(
            &["A", "B", "C"],
            &[("A", "B"), ("B", "C")],
        );
        let result = check(&log, &model_fp);
        assert!(result.fitness < 1.0);
        assert!(result.precision > 0.0);
    }

    #[test]
    fn imperfect_precision_missing_pair() {
        let log = make_log(vec![("1", &["A", "B"])]);
        let model_fp = make_model_fp(
            &["A", "B", "C"],
            &[("A", "B"), ("B", "C")],
        );
        let result = check(&log, &model_fp);
        assert!((result.fitness - 1.0).abs() < 1e-9);
        assert!(result.precision < 1.0);
    }

    #[test]
    fn empty_log() {
        let log = make_log(vec![]);
        let model_fp = make_model_fp(&["A"], &[]);
        let result = check(&log, &model_fp);
        assert!((result.fitness - 1.0).abs() < 1e-9);
    }
}
