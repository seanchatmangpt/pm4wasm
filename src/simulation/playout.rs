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

/// Process tree and DFG playout (log simulation).
///
/// Ports `pm4py.algo.simulation.playout.process_tree` and
/// `pm4py.algo.simulation.playout.dfg`.
///
/// Playout generates synthetic event logs from process models by executing
/// the model structure. This is useful for:
/// - Testing process mining algorithms
/// - Generating training data for ML models
/// - Validating discovered models
use crate::event_log::{Event, EventLog, Trace};
use crate::process_tree::{PtOperator, ProcessTree};
use rand::Rng;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

// ─── Parameters ────────────────────────────────────────────────────────────────

/// Parameters for playout algorithms.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PlayOutParameters {
    /// Number of traces to generate (default: 100).
    pub num_traces: usize,
    /// Whether to include timestamps (default: true).
    pub include_timestamps: bool,
    /// Starting timestamp (Unix seconds, default: 10000000).
    pub start_timestamp: i64,
    /// Minimum trace length (default: 1).
    pub min_trace_length: usize,
    /// Maximum trace length (default: 100, prevents infinite loops).
    pub max_trace_length: usize,
}

impl Default for PlayOutParameters {
    fn default() -> Self {
        Self {
            num_traces: 100,
            include_timestamps: true,
            start_timestamp: 10000000,
            min_trace_length: 1,
            max_trace_length: 100,
        }
    }
}

// ─── Process Tree Playout ──────────────────────────────────────────────────────

/// Execution state for process tree nodes.
#[derive(Clone, Copy, Debug, PartialEq)]
enum NodeState {
    /// Node is ready to execute
    Enabled,
    /// Node is currently executing
    Open,
    /// Node has completed
    Closed,
}

/// Node execution context.
#[derive(Clone, Debug)]
struct NodeContext {
    state: NodeState,
    parent_idx: Option<usize>,
    child_index: usize, // For sequences: which child we're on
}

/// Execute a process tree to generate a single trace (execution sequence).
fn execute_process_tree(pt: &ProcessTree) -> Vec<String> {
    let mut nodes: Vec<&ProcessTree> = vec![pt];
    let mut contexts: Vec<NodeContext> = vec![NodeContext {
        state: NodeState::Enabled,
        parent_idx: None,
        child_index: 0,
    }];
    let mut enabled: Vec<usize> = vec![0];
    let mut events: Vec<String> = Vec::new();

    while !enabled.is_empty() {
        let Some(&vertex_idx) = enabled.first() else {
            break;
        };

        let vertex = nodes[vertex_idx];
        let ctx = &mut contexts[vertex_idx];

        // Skip if already closed
        if ctx.state == NodeState::Closed {
            enabled.remove(0);
            continue;
        }

        ctx.state = NodeState::Open;

        // If leaf node, record the event and close
        if vertex.children.is_empty() {
            if let Some(ref label) = vertex.label {
                if !label.is_empty() {
                    events.push(label.clone());
                }
            }
            ctx.state = NodeState::Closed;
            enabled.remove(0);

            // For sequence operators, enable next child
            if let Some(parent_idx) = ctx.parent_idx {
                let parent_ctx = &mut contexts[parent_idx];
                let parent = nodes[parent_idx];
                if let Some(PtOperator::Sequence) = parent.operator {
                    parent_ctx.child_index += 1;
                    if parent_ctx.child_index < parent.children.len() {
                        // Enable next child
                        let next_child = &parent.children[parent_ctx.child_index];
                        let next_idx = nodes.len();
                        nodes.push(next_child);
                        contexts.push(NodeContext {
                            state: NodeState::Enabled,
                            parent_idx: Some(parent_idx),
                            child_index: 0,
                        });
                        enabled.push(next_idx);
                    }
                }
            }

            continue;
        }

        // Internal node: enable children based on operator
        let operator = vertex.operator.unwrap_or(PtOperator::Sequence);

        match operator {
            PtOperator::Sequence => {
                // Enable first child
                ctx.child_index = 0;
                if let Some(child) = vertex.children.first() {
                    let child_idx = nodes.len();
                    nodes.push(child);
                    contexts.push(NodeContext {
                        state: NodeState::Enabled,
                        parent_idx: Some(vertex_idx),
                        child_index: 0,
                    });
                    enabled.push(child_idx);
                }
                enabled.remove(0); // Remove parent from enabled
            }
            PtOperator::Xor => {
                // Randomly select one child
                if !vertex.children.is_empty() {
                    let mut rng = rand::thread_rng();
                    let child_idx_val = rng.gen_range(0..vertex.children.len());
                    let child = &vertex.children[child_idx_val];
                    let new_idx = nodes.len();
                    nodes.push(child);
                    contexts.push(NodeContext {
                        state: NodeState::Enabled,
                        parent_idx: Some(vertex_idx),
                        child_index: 0,
                    });
                    enabled.push(new_idx);
                }
                enabled.remove(0); // Remove parent from enabled
            }
            PtOperator::Parallel => {
                // Enable all children
                for child in &vertex.children {
                    let child_idx = nodes.len();
                    nodes.push(child);
                    contexts.push(NodeContext {
                        state: NodeState::Enabled,
                        parent_idx: Some(vertex_idx),
                        child_index: 0,
                    });
                    enabled.push(child_idx);
                }
                enabled.remove(0); // Remove parent from enabled
            }
            PtOperator::Loop => {
                // Loop: do, redo, exit (children[0], children[1], children[2])
                // For basic playout, execute do once, then maybe redo
                if vertex.children.len() >= 1 {
                    let mut rng = rand::thread_rng();

                    // Execute do branch
                    let do_idx = nodes.len();
                    nodes.push(&vertex.children[0]);
                    contexts.push(NodeContext {
                        state: NodeState::Enabled,
                        parent_idx: Some(vertex_idx),
                        child_index: 0,
                    });
                    enabled.push(do_idx);

                    // Maybe add redo
                    if vertex.children.len() >= 2 && rng.gen_bool(0.3) {
                        let redo_idx = nodes.len();
                        nodes.push(&vertex.children[1]);
                        contexts.push(NodeContext {
                            state: NodeState::Enabled,
                            parent_idx: Some(vertex_idx),
                            child_index: 0,
                        });
                        enabled.push(redo_idx);
                    }
                }
                enabled.remove(0); // Remove parent from enabled
            }
        }
    }

    events
}

/// Generate an event log by playout of a process tree.
///
/// Ports `pm4py.algo.simulation.playout.process_tree.algorithm.apply()`.
///
/// # Arguments
/// * `pt` - Process tree to execute
/// * `params` - Playout parameters (number of traces, etc.)
///
/// # Returns
/// Simulated event log with synthetic traces
pub fn play_out_process_tree(pt: &ProcessTree, params: &PlayOutParameters) -> EventLog {
    let mut log = EventLog { traces: Vec::new() };
    let mut current_timestamp = params.start_timestamp;

    for i in 0..params.num_traces {
        let events = execute_process_tree(pt);

        // Skip traces that are too short (optional filter)
        if events.len() < params.min_trace_length {
            continue;
        }

        let trace_events: Vec<Event> = events
            .iter()
            .enumerate()
            .map(|(j, name)| Event {
                name: name.clone(),
                timestamp: if params.include_timestamps {
                    let ts = current_timestamp + j as i64;
                    Some(format!("{}", ts))
                } else {
                    None
                },
                lifecycle: None,
                attributes: HashMap::new(),
            })
            .collect();

        log.traces.push(Trace {
            case_id: format!("{}", i),
            events: trace_events,
        });

        current_timestamp += events.len() as i64 + 10; // Gap between traces
    }

    log
}

// ─── DFG Playout ───────────────────────────────────────────────────────────────

/// DFG structure for playout.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DirectedGraph {
    /// All activities in the graph.
    pub activities: Vec<String>,
    /// Adjacency map: activity -> list of successors.
    pub adj: HashMap<String, Vec<String>>,
}

impl Default for DirectedGraph {
    fn default() -> Self {
        Self {
            activities: Vec::new(),
            adj: HashMap::new(),
        }
    }
}

/// Generate an event log by playout of a directly-follows graph.
///
/// Ports `pm4py.algo.simulation.playout.dfg.algorithm.apply()`.
///
/// The algorithm:
/// 1. Identify start activities (no incoming edges)
/// 2. Random walk following outgoing edges
/// 3. End when reaching a sink (no outgoing edges) or max length
///
/// # Arguments
/// * `dfg` - Directed graph as adjacency map (activity -> [successors])
/// * `start_activities` - Activities that can start a trace
/// * `end_activities` - Activities that can end a trace
/// * `params` - Playout parameters
///
/// # Returns
/// Simulated event log with synthetic traces
pub fn play_out_dfg(
    dfg: &DirectedGraph,
    start_activities: &[String],
    end_activities: &[String],
    params: &PlayOutParameters,
) -> EventLog {
    let mut log = EventLog { traces: Vec::new() };
    let mut rng = rand::thread_rng();
    let mut current_timestamp = params.start_timestamp;

    // Convert start/end to sets for fast lookup
    let start_set: HashSet<&str> = start_activities.iter().map(|s| s.as_str()).collect();
    let end_set: HashSet<&str> = end_activities.iter().map(|s| s.as_str()).collect();

    // Build adjacency map for fast lookup
    let mut outgoing: HashMap<&str, Vec<&str>> = HashMap::new();
    for (src, dsts) in &dfg.adj {
        outgoing.insert(src.as_str(), dsts.iter().map(|s| s.as_str()).collect());
    }

    for i in 0..params.num_traces {
        let mut trace = Vec::new();

        // Pick random start activity
        let current_str = if !start_set.is_empty() {
            let idx = rng.gen_range(0..start_set.len());
            start_set.iter().nth(idx).unwrap().to_string()
        } else if let Some(first) = dfg.activities.first() {
            first.clone()
        } else {
            break; // No activities
        };

        let mut current_str = current_str;
        trace.push(current_str.clone());

        // Random walk
        for _ in 0..params.max_trace_length {
            // Check if we can end here
            if end_set.contains(current_str.as_str()) && rng.gen_bool(0.3) {
                break;
            }

            // Get successors
            if let Some(successors) = outgoing.get(current_str.as_str()) {
                if successors.is_empty() {
                    break; // Sink node
                }

                // Randomly pick successor
                if !successors.is_empty() {
                    let next_idx = rng.gen_range(0..successors.len());
                    let next = successors[next_idx];
                    current_str = next.to_string();
                    trace.push(current_str.clone());
                } else {
                    break;
                }
            } else {
                break; // No outgoing edges
            }
        }

        // Skip traces that are too short
        if trace.len() < params.min_trace_length {
            continue;
        }

        let trace_events: Vec<Event> = trace
            .iter()
            .enumerate()
            .map(|(j, name): (usize, &String)| Event {
                name: name.clone(),
                timestamp: if params.include_timestamps {
                    let ts = current_timestamp + j as i64;
                    Some(format!("{}", ts))
                } else {
                    None
                },
                lifecycle: None,
                attributes: HashMap::new(),
            })
            .collect();

        let trace_len = trace_events.len();
        log.traces.push(Trace {
            case_id: format!("{}", i),
            events: trace_events,
        });

        current_timestamp += trace_len as i64 + 10;
    }

    log
}

// ─── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_play_out_simple_sequence() {
        // A -> B -> C
        let pt = ProcessTree::internal(
            PtOperator::Sequence,
            vec![
                ProcessTree::leaf(Some("A".to_string())),
                ProcessTree::leaf(Some("B".to_string())),
                ProcessTree::leaf(Some("C".to_string())),
            ],
        );

        let params = PlayOutParameters {
            num_traces: 10,
            include_timestamps: false,
            ..Default::default()
        };

        let log = play_out_process_tree(&pt, &params);

        assert_eq!(log.traces.len(), 10);
        for trace in &log.traces {
            assert_eq!(trace.events.len(), 3);
            assert_eq!(trace.events[0].name, "A");
            assert_eq!(trace.events[1].name, "B");
            assert_eq!(trace.events[2].name, "C");
        }
    }

    #[test]
    fn test_play_out_xor() {
        // A xor B
        let pt = ProcessTree::internal(
            PtOperator::Xor,
            vec![
                ProcessTree::leaf(Some("A".to_string())),
                ProcessTree::leaf(Some("B".to_string())),
            ],
        );

        let params = PlayOutParameters {
            num_traces: 100,
            include_timestamps: false,
            ..Default::default()
        };

        let log = play_out_process_tree(&pt, &params);

        assert_eq!(log.traces.len(), 100);
        for trace in &log.traces {
            assert_eq!(trace.events.len(), 1);
            assert!(trace.events[0].name == "A" || trace.events[0].name == "B");
        }
    }

    #[test]
    fn test_play_out_dfg_simple() {
        // A -> B -> C
        let mut dfg = DirectedGraph::default();
        dfg.activities.extend(["A", "B", "C"].map(|s| s.to_string()));
        dfg.adj.insert("A".to_string(), vec!["B".to_string()]);
        dfg.adj.insert("B".to_string(), vec!["C".to_string()]);

        let params = PlayOutParameters {
            num_traces: 10,
            include_timestamps: false,
            ..Default::default()
        };

        let log = play_out_dfg(&dfg, &["A".to_string()], &["C".to_string()], &params);

        assert!(!log.traces.is_empty());
        for trace in &log.traces {
            assert!(!trace.events.is_empty());
            assert_eq!(trace.events[0].name, "A");
        }
    }
}
