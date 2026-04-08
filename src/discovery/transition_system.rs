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

/// Transition system discovery from event logs.
///
/// Ports `pm4py.algo.discovery.transition_system`.
///
/// A transition system is a state machine that captures all observed
/// behavior in an event log. Each state represents a "view" of the trace,
/// and transitions represent activity executions that move between states.
use crate::event_log::EventLog;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// A state in the transition system.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TSState {
    /// Unique state identifier.
    pub id: usize,
    /// The activity sequence that defines this state (window of activities).
    pub name: String,
}

/// A transition between states.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TSTransition {
    /// Source state ID.
    pub from_state: usize,
    /// Target state ID.
    pub to_state: usize,
    /// Activity label that triggered this transition.
    pub activity: String,
    /// Number of times this transition occurs.
    pub count: usize,
}

/// A transition system.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TransitionSystem {
    /// All states in the system.
    pub states: Vec<TSState>,
    /// All transitions in the system.
    pub transitions: Vec<TSTransition>,
    /// Mapping from state name to state ID.
    pub state_map: HashMap<String, usize>,
}

/// Discover a transition system from an event log.
///
/// Ports `pm4py.algo.discovery.transition_system.algorithm.apply()`.
///
/// The algorithm builds a state machine where:
/// - Each state is defined by a "view" (window of recent activities)
/// - Transitions represent activity executions
///
/// # Arguments
/// * `log` - Event log to analyze
/// * `window` - Size of the lookback window (default: 2)
/// * `direction` - "forward" (default) or "backward" direction
///
/// # Returns
/// Transition system with states and transitions
pub fn discover_transition_system(log: &EventLog, window: usize, direction: &str) -> TransitionSystem {
    let mut states: Vec<TSState> = Vec::new();
    let mut transitions: Vec<TSTransition> = Vec::new();
    let mut state_map: HashMap<String, usize> = HashMap::new();
    let mut transition_map: HashMap<(usize, usize, String), usize> = HashMap::new();

    let is_forward = direction == "forward";

    for trace in &log.traces {
        let activities: Vec<&str> = trace.events.iter().map(|e| e.name.as_str()).collect();

        if activities.is_empty() {
            continue;
        }

        // Build states based on window
        let mut current_state_id = usize::MAX;

        for i in 0..activities.len() {
            let start = if i >= window { i - window } else { 0 };
            let state_activities: Vec<&str> = if is_forward {
                activities[start..=i].to_vec()
            } else {
                activities[i..=(i + window).min(activities.len() - 1)].to_vec()
            };

            let state_name = state_activities.join(", ");

            // Get or create state
            let state_id = if let Some(&id) = state_map.get(&state_name) {
                id
            } else {
                let id = states.len();
                states.push(TSState {
                    id,
                    name: state_name.clone(),
                });
                state_map.insert(state_name, id);
                id
            };

            // Add transition from previous state
            if current_state_id != usize::MAX && i > 0 {
                let activity = activities[i].to_string();
                let key = (current_state_id, state_id, activity.clone());

                *transition_map.entry(key).or_insert(0) += 1;
            }

            current_state_id = state_id;
        }
    }

    // Convert transition map to transitions
    for ((from_state, to_state, activity), count) in transition_map {
        transitions.push(TSTransition {
            from_state,
            to_state,
            activity,
            count,
        });
    }

    TransitionSystem {
        states,
        transitions,
        state_map,
    }
}

// ─── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::event_log::{Event, Trace};

    fn create_test_log() -> EventLog {
        EventLog {
            traces: vec![
                Trace {
                    case_id: "1".to_string(),
                    events: vec![
                        Event { name: "A".to_string(), timestamp: None, lifecycle: None, attributes: HashMap::new() },
                        Event { name: "B".to_string(), timestamp: None, lifecycle: None, attributes: HashMap::new() },
                        Event { name: "C".to_string(), timestamp: None, lifecycle: None, attributes: HashMap::new() },
                    ],
                },
                Trace {
                    case_id: "2".to_string(),
                    events: vec![
                        Event { name: "A".to_string(), timestamp: None, lifecycle: None, attributes: HashMap::new() },
                        Event { name: "B".to_string(), timestamp: None, lifecycle: None, attributes: HashMap::new() },
                    ],
                },
            ],
        }
    }

    #[test]
    fn test_transition_system_simple() {
        let log = create_test_log();
        let ts = discover_transition_system(&log, 2, "forward");

        assert!(!ts.states.is_empty());
        assert!(!ts.transitions.is_empty());
    }

    #[test]
    fn test_transition_system_window_size() {
        let log = create_test_log();
        let ts_small = discover_transition_system(&log, 1, "forward");
        let ts_large = discover_transition_system(&log, 3, "forward");

        // Larger window should create fewer states (more activities fit in each state)
        assert!(ts_large.states.len() <= ts_small.states.len());
    }
}
