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
use std::collections::HashMap;

/// A Directly-Follows Graph (DFG).
///
/// Mirrors `pm4py.discover_dfg()`.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DFGResult {
    /// Directed edges with frequency: (source, target) → count.
    pub edges: Vec<DFGEdge>,
    /// Start activities with frequencies.
    pub start_activities: Vec<(String, usize)>,
    /// End activities with frequencies.
    pub end_activities: Vec<(String, usize)>,
    /// All activity frequencies.
    pub activities: Vec<(String, usize)>,
}

/// An edge in the DFG.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DFGEdge {
    pub source: String,
    pub target: String,
    pub count: usize,
}

/// Discover a Directly-Follows Graph from an event log.
///
/// Mirrors `pm4py.discover_dfg()`.
pub fn discover_dfg(log: &EventLog) -> DFGResult {
    let mut edge_freq: HashMap<(String, String), usize> = HashMap::new();
    let mut start_freq: HashMap<String, usize> = HashMap::new();
    let mut end_freq: HashMap<String, usize> = HashMap::new();
    let mut activity_freq: HashMap<String, usize> = HashMap::new();

    for trace in &log.traces {
        for i in 0..trace.events.len() {
            *activity_freq
                .entry(trace.events[i].name.clone())
                .or_insert(0) += 1;
        }

        for window in trace.events.windows(2) {
            let src = &window[0].name;
            let tgt = &window[1].name;
            *edge_freq.entry((src.clone(), tgt.clone())).or_default() += 1;
        }

        if let Some(first) = trace.events.first() {
            *start_freq.entry(first.name.clone()).or_insert(0) += 1;
        }
        if let Some(last) = trace.events.last() {
            *end_freq.entry(last.name.clone()).or_insert(0) += 1;
        }
    }

    let mut edges: Vec<DFGEdge> = edge_freq
        .into_iter()
        .map(|((source, target), count)| DFGEdge {
            source,
            target,
            count,
        })
        .collect();
    edges.sort_by(|a, b| b.count.cmp(&a.count));

    let mut start_activities: Vec<(String, usize)> =
        start_freq.into_iter().collect();
    start_activities.sort_by(|a, b| b.1.cmp(&a.1));

    let mut end_activities: Vec<(String, usize)> = end_freq.into_iter().collect();
    end_activities.sort_by(|a, b| b.1.cmp(&a.1));

    let mut activities: Vec<(String, usize)> = activity_freq.into_iter().collect();
    activities.sort_by(|a, b| b.1.cmp(&a.1));

    DFGResult {
        edges,
        start_activities,
        end_activities,
        activities,
    }
}

/// A typed DFG result matching pm4py.objects.dfg.obj.DFG format.
///
/// Returns a structured DFG object with graph as (from, to, frequency) triples,
/// and start/end activities as (activity, frequency) pairs.
///
/// Mirrors `pm4py.discover_dfg_typed()`.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DFGTyped {
    /// The graph as (from_activity, to_activity, frequency) triples.
    pub graph: Vec<(String, String, u32)>,
    /// Start activities as (activity, frequency) pairs.
    pub start_activities: Vec<(String, u32)>,
    /// End activities as (activity, frequency) pairs.
    pub end_activities: Vec<(String, u32)>,
    /// All activities as (activity, frequency) pairs.
    pub activities: Vec<(String, u32)>,
}

/// Discover a typed DFG from an event log.
///
/// Returns a DFGTyped object with graph, start_activities, end_activities, and activities.
///
/// Mirrors `pm4py.discover_dfg_typed()`.
pub fn discover_dfg_typed(log: &EventLog) -> DFGTyped {
    let dfg = discover_dfg(log);

    // Convert edges to (from, to, frequency) format
    let graph: Vec<(String, String, u32)> = dfg
        .edges
        .into_iter()
        .map(|edge| (edge.source, edge.target, edge.count as u32))
        .collect();

    // Convert activities to (activity, frequency) format
    let activities: Vec<(String, u32)> = dfg
        .activities
        .into_iter()
        .map(|(activity, count)| (activity, count as u32))
        .collect();

    let start_activities: Vec<(String, u32)> = dfg
        .start_activities
        .into_iter()
        .map(|(activity, count)| (activity, count as u32))
        .collect();

    let end_activities: Vec<(String, u32)> = dfg
        .end_activities
        .into_iter()
        .map(|(activity, count)| (activity, count as u32))
        .collect();

    DFGTyped {
        graph,
        start_activities,
        end_activities,
        activities,
    }
}

/// Discover a performance DFG (edges annotated with average duration).
///
/// Mirrors `pm4py.discover_performance_dfg()`.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PerformanceDFGResult {
    pub edges: Vec<PerformanceDFGEdge>,
    pub start_activities: Vec<(String, usize)>,
    pub end_activities: Vec<(String, usize)>,
    pub activities: Vec<(String, usize)>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PerformanceDFGEdge {
    pub source: String,
    pub target: String,
    pub count: usize,
    pub avg_duration_ms: f64,
    pub min_duration_ms: f64,
    pub max_duration_ms: f64,
}

/// Discover a performance DFG with duration annotations.
pub fn discover_performance_dfg(log: &EventLog) -> PerformanceDFGResult {
    let dfg = discover_dfg(log);

    // Compute duration stats per edge
    let mut edge_durations: HashMap<(String, String), Vec<i64>> = HashMap::new();
    for trace in &log.traces {
        for window in trace.events.windows(2) {
            if let (Some(first_ts), Some(last_ts)) =
                (window[0].timestamp.as_ref(), window[1].timestamp.as_ref())
            {
                if let (Ok(first), Ok(last)) =
                    (chrono::DateTime::parse_from_rfc3339(first_ts), chrono::DateTime::parse_from_rfc3339(last_ts))
                {
                    let dur = last.timestamp_millis() - first.timestamp_millis();
                    edge_durations
                        .entry((window[0].name.clone(), window[1].name.clone()))
                        .or_default()
                        .push(dur);
                }
            }
        }
    }

    let mut edges: Vec<PerformanceDFGEdge> = dfg
        .edges
        .into_iter()
        .map(|edge| {
            let durations = edge_durations
                .get(&(edge.source.clone(), edge.target.clone()))
                .map(|v| v.as_slice())
                .unwrap_or(&[]);
            let avg = if !durations.is_empty() {
                durations.iter().copied().map(|d| d as f64).sum::<f64>() / durations.len() as f64
            } else {
                0.0
            };
            let min = durations
                .iter()
                .copied()
                .map(|d| d as f64)
                .fold(f64::MAX, f64::min);
            let max = durations
                .iter()
                .copied()
                .map(|d| d as f64)
                .fold(f64::MIN, f64::max);
            PerformanceDFGEdge {
                source: edge.source,
                target: edge.target,
                count: edge.count,
                avg_duration_ms: avg,
                min_duration_ms: if min == f64::MAX { 0.0 } else { min },
                max_duration_ms: if max == f64::MIN { 0.0 } else { max },
            }
        })
        .collect();

    edges.sort_by(|a, b| b.count.cmp(&a.count));

    PerformanceDFGResult {
        edges,
        start_activities: dfg.start_activities,
        end_activities: dfg.end_activities,
        activities: dfg.activities,
    }
}

/// Discover an eventually-follows graph (all activity pairs that appear in any trace).
///
/// Mirrors `pm4py.discover_eventually_follows_graph()`.
pub fn discover_eventually_follows_graph(log: &EventLog) -> Vec<DFGEdge> {
    let mut edge_freq: HashMap<(String, String), usize> = HashMap::new();

    for trace in &log.traces {
        let activity_set: std::collections::HashSet<&str> =
            trace.events.iter().map(|e| e.name.as_str()).collect();
        for i in 0..trace.events.len() {
            for j in (i + 1)..trace.events.len() {
                let src = &trace.events[i].name;
                let tgt = &trace.events[j].name;
                if activity_set.contains(tgt.as_str()) {
                    *edge_freq
                        .entry((src.clone(), tgt.clone()))
                        .or_default() += 1;
                }
            }
        }
    }

    let mut edges: Vec<DFGEdge> = edge_freq
        .into_iter()
        .map(|((source, target), count)| DFGEdge {
            source,
            target,
            count,
        })
        .collect();
    edges.sort_by(|a, b| b.count.cmp(&a.count));
    edges
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
    fn test_discover_dfg() {
        let log = make_test_log();
        let dfg = discover_dfg(&log);
        // A->B appears in cases 1 and 2, so count 2
        assert!(dfg.edges.iter().any(|e| e.source == "A" && e.target == "B"));
        // A->C appears in case 3, so count 1
        assert!(dfg.edges.iter().any(|e| e.source == "A" && e.target == "C"));
        // B->C appears in case 2, so count 1
        assert!(dfg.edges.iter().any(|e| e.source == "B" && e.target == "C"));
    }

    #[test]
    fn test_start_end_activities() {
        let log = make_test_log();
        let dfg = discover_dfg(&log);
        assert_eq!(dfg.start_activities.len(), 1);
        assert_eq!(dfg.start_activities[0].0, "A");
        assert_eq!(dfg.start_activities[0].1, 3);
        assert_eq!(dfg.end_activities.len(), 2);
    }

    #[test]
    fn test_eventually_follows() {
        let log = make_test_log();
        let efg = discover_eventually_follows_graph(&log);
        // A->C should exist (case 2 and 3)
        assert!(efg.iter().any(|e| e.source == "A" && e.target == "C"));
        // B->C should exist (case 2)
        assert!(efg.iter().any(|e| e.source == "B" && e.target == "C"));
    }
}
