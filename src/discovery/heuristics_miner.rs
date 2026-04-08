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

//! Heuristics Miner - discovers process models from real-world logs.
//!
//! More lenient than Alpha++ for handling noise and incomplete data.
//! Uses dependency measure to filter causal relations.

use crate::event_log::EventLog;
use crate::petri_net::PetriNetResult;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

/// Dependency measure for a pair of activities.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Dependency {
    /// Source activity
    pub from: String,
    /// Target activity
    pub to: String,
    /// Dependency score (0.0 to 1.0)
    pub dependency: f64,
    /// Frequency of the relation
    pub frequency: usize,
}

/// Heuristics net result with dependency measures.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct HeuristicsNet {
    /// All activities (nodes)
    pub activities: Vec<String>,
    /// Dependency relations (edges with measures)
    pub dependencies: Vec<Dependency>,
    /// Start activities with frequencies
    pub start_activities: HashMap<String, usize>,
    /// End activities with frequencies
    pub end_activities: HashMap<String, usize>,
}

/// Compute dependency measure between two activities.
///
/// The dependency measure is: (A→B - B→A) / (A→B + B→A + 1)
/// where A→B is the number of times A is directly followed by B.
fn compute_dependency(
    follows: &HashMap<(String, String), usize>,
    precedes: &HashMap<(String, String), usize>,
    a: &str,
    b: &str,
) -> f64 {
    let ab = follows.get(&(a.to_string(), b.to_string())).copied().unwrap_or(0);
    let ba = precedes.get(&(a.to_string(), b.to_string())).copied().unwrap_or(0);

    let ab_f = ab as f64;
    let ba_f = ba as f64;

    (ab_f - ba_f) / (ab_f + ba_f + 1.0)
}

/// Discover a Heuristics Net from an event log.
///
/// # Arguments
/// * `log` - Event log to analyze
/// * `dependency_threshold` - Minimum dependency score for an edge (0.0 to 1.0)
///
/// # Returns
/// Heuristics net with activities, dependencies, and start/end activities
pub fn discover_heuristics_miner(log: &EventLog, dependency_threshold: f64) -> HeuristicsNet {
    let mut activities: HashSet<String> = HashSet::new();
    let mut follows: HashMap<(String, String), usize> = HashMap::new();
    let mut precedes: HashMap<(String, String), usize> = HashMap::new();
    let mut start_activities: HashMap<String, usize> = HashMap::new();
    let mut end_activities: HashMap<String, usize> = HashMap::new();

    // Build statistics from log
    for trace in &log.traces {
        let events = &trace.events;

        if !events.is_empty() {
            // Record start activity
            *start_activities
                .entry(events[0].name.clone())
                .or_insert(0) += 1;

            // Record end activity
            *end_activities
                .entry(events[events.len() - 1].name.clone())
                .or_insert(0) += 1;
        }

        for i in 0..events.len() {
            activities.insert(events[i].name.clone());

            // Record directly-follows relations
            if i < events.len().saturating_sub(1) {
                let from = &events[i].name;
                let to = &events[i + 1].name;
                *follows.entry((from.clone(), to.clone())).or_insert(0) += 1;
                *precedes.entry((to.clone(), from.clone())).or_insert(0) += 1;
            }
        }
    }

    // Compute dependency measures and filter by threshold
    let mut dependencies = Vec::new();
    for ((from, to), freq) in &follows {
        let dep = compute_dependency(&follows, &precedes, from, to);
        if dep >= dependency_threshold {
            dependencies.push(Dependency {
                from: from.clone(),
                to: to.clone(),
                dependency: dep,
                frequency: *freq,
            });
        }
    }

    // Sort by dependency descending
    dependencies.sort_by(|a, b| {
        b.dependency
            .partial_cmp(&a.dependency)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    let activity_list: Vec<String> = activities.into_iter().collect();

    HeuristicsNet {
        activities: activity_list,
        dependencies,
        start_activities,
        end_activities,
    }
}

/// Convert a Heuristics Net to a Petri Net.
///
/// Creates places and transitions based on dependency relations.
pub fn heuristics_to_petri_net(net: &HeuristicsNet) -> PetriNetResult {
    use crate::petri_net::{PetriNet, Place, Transition, Arc, Marking};
    use std::collections::HashMap;

    let mut places = Vec::new();
    let mut transitions = Vec::new();
    let mut arcs = Vec::new();
    let mut initial_marking: Marking = HashMap::new();
    let mut final_marking: Marking = HashMap::new();

    // Create a transition for each activity
    for activity in &net.activities {
        transitions.push(Transition {
            name: activity.clone(),
            label: Some(activity.clone()),
            properties: HashMap::new(),
        });
    }

    // Create source place
    let source_place = "p_source".to_string();
    places.push(Place {
        name: source_place.clone(),
    });
    let source_tokens = net.start_activities.values().sum::<usize>() as u32;
    initial_marking.insert(source_place.clone(), source_tokens);

    // Create sink place
    let sink_place = "p_sink".to_string();
    places.push(Place {
        name: sink_place.clone(),
    });
    final_marking.insert(sink_place.clone(), net.end_activities.values().sum::<usize>() as u32);

    // Create arcs based on dependencies
    for dep in &net.dependencies {
        // Find transition indices
        let from_idx = net
            .activities
            .iter()
            .position(|a| a == &dep.from)
            .unwrap_or(0);
        let to_idx = net.activities.iter().position(|a| a == &dep.to).unwrap_or(0);

        let from_transition = &transitions[from_idx].name;
        let to_transition = &transitions[to_idx].name;

        // Add arcs through intermediate place
        let place_name = format!("p_{}_{}", dep.from, dep.to);
        places.push(Place {
            name: place_name.clone(),
        });

        // Arc from source to first transition if it's a start activity
        if net.start_activities.contains_key(&dep.from) {
            arcs.push(Arc {
                source: source_place.clone(),
                target: from_transition.clone(),
                weight: 1,
            });
        }

        // Arc from transition to place
        arcs.push(Arc {
            source: from_transition.clone(),
            target: place_name.clone(),
            weight: 1,
        });

        // Arc from place to next transition
        arcs.push(Arc {
            source: place_name.clone(),
            target: to_transition.clone(),
            weight: 1,
        });
    }

    // Add arcs from last transitions to sink
    for (i, activity) in net.activities.iter().enumerate() {
        if net.end_activities.contains_key(activity) {
            let transition = &transitions[i].name;
            arcs.push(Arc {
                source: transition.clone(),
                target: sink_place.clone(),
                weight: 1,
            });
        }
    }

    let petri_net = PetriNet {
        name: "heuristics_miner".to_string(),
        places,
        transitions,
        arcs,
    };

    PetriNetResult {
        net: petri_net,
        initial_marking,
        final_marking,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::event_log::parse_csv;

    #[test]
    fn test_heuristics_miner_simple() {
        let csv = "case_id,activity\n\
                   1,A\n\
                   1,B\n\
                   1,C\n\
                   2,A\n\
                   2,B\n\
                   2,C\n\
                   3,A\n\
                   3,C";
        let log = parse_csv(csv).unwrap();
        let net = discover_heuristics_miner(&log, 0.5);

        assert_eq!(net.activities.len(), 3);
        assert!(net.activities.contains(&"A".to_string()));
        assert!(net.activities.contains(&"B".to_string()));
        assert!(net.activities.contains(&"C".to_string()));

        // A→B should have high dependency (appears in 2/3 traces)
        let ab_dep = net.dependencies.iter().find(|d| d.from == "A" && d.to == "B");
        assert!(ab_dep.is_some());

        // B→C appears in 2/3 traces, so with threshold 0.5 it should be included
        let bc_dep = net.dependencies.iter().find(|d| d.from == "B" && d.to == "C");
        assert!(bc_dep.is_some());

        // A→C appears in all 3 traces
        let ac_dep = net.dependencies.iter().find(|d| d.from == "A" && d.to == "C");
        assert!(ac_dep.is_some());
    }

    #[test]
    fn test_heuristics_to_petri_net() {
        let csv = "case_id,activity\n\
                   1,A\n\
                   1,B\n\
                   2,A\n\
                   2,B";
        let log = parse_csv(csv).unwrap();
        let net = discover_heuristics_miner(&log, 0.5);
        let pn = heuristics_to_petri_net(&net);

        assert!(!pn.net.places.is_empty());
        assert!(!pn.net.transitions.is_empty());
        assert!(!pn.net.arcs.is_empty());
    }
}
