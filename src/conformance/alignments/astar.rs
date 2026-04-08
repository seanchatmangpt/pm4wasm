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

//! A* alignment search for optimal trace-to-model conformance checking.
//!
//! Builds a synchronous product net between the trace and the Petri net model,
//! then uses A* search with an admissible heuristic to find the minimum-cost
//! alignment. Each state in the search is a marking of the sync product net.
//!
//! The heuristic uses a simplified marking equation lower bound: for each
//! remaining token, the minimum transition cost to consume it. This is
//! admissible (never overestimates) and fast to compute. A proper LP-based
//! heuristic will replace this in a future iteration.

use crate::event_log::{EventLog, Trace};
use crate::petri_net::{Marking, PetriNet};
use serde::{Deserialize, Serialize};
use std::collections::{BinaryHeap, HashMap, HashSet};
use std::cmp::Ordering;

// ─── Cost constants ──────────────────────────────────────────────────────────

/// Cost of a synchronous move (trace event matches model transition).
const STD_SYNC_COST: f64 = 0.0;
/// Cost of a log move (trace event has no matching model transition).
const STD_LOG_MOVE_COST: f64 = 10000.0;
/// Cost of a visible model move (model transition fires without trace event).
const STD_MODEL_MOVE_COST: f64 = 10000.0;
/// Cost of an invisible (silent/tau) model move.
const STD_TAU_COST: f64 = 1.0;

/// Maximum number of A* state expansions per trace before giving up.
const MAX_EXPANSIONS: usize = 100_000;

// ─── Public result types ─────────────────────────────────────────────────────

/// A single move in an alignment.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AlignmentMove {
    /// Trace activity name. `None` for model-only moves.
    pub trace_activity: Option<String>,
    /// Model activity label. `None` for log-only moves.
    pub model_activity: Option<String>,
    /// Cost of this move.
    pub cost: f64,
}

/// Result of aligning a single trace against a Petri net.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AlignmentResult {
    /// Sequence of moves mapping trace events to model transitions.
    pub alignment: Vec<AlignmentMove>,
    /// Total cost of the alignment.
    pub cost: f64,
    /// Fitness in [0.0, 1.0]. 1.0 = perfect fit.
    pub fitness: f64,
    /// `true` when the alignment cost is 0 (perfect replay).
    pub is_fit: bool,
}

/// Result of aligning a single trace, tagged with its case ID.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TraceAlignment {
    /// Case identifier from the trace.
    pub case_id: String,
    /// Sequence of alignment moves.
    pub alignment: Vec<AlignmentMove>,
    /// Total cost of the alignment.
    pub cost: f64,
    /// Fitness in [0.0, 1.0].
    pub fitness: f64,
    /// `true` when the alignment cost is 0.
    pub is_fit: bool,
}

/// Result of aligning an entire event log against a Petri net.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LogAlignmentResult {
    /// Per-trace alignment results.
    pub trace_results: Vec<TraceAlignment>,
    /// Average fitness across all traces.
    pub average_fitness: f64,
}

// ─── Synchronous product net (index-based) ───────────────────────────────────

/// A transition in the synchronous product net.
#[derive(Clone, Debug)]
struct SyncTransition {
    /// Index of this transition in the sync product.
    index: usize,
    /// Preset: (place_index, is_trace_place, original_index).
    preset: Vec<(usize, bool, usize)>,
    /// Postset: (place_index, is_trace_place, original_index).
    postset: Vec<(usize, bool, usize)>,
    /// Cost of firing this transition.
    cost: f64,
    /// Label for debugging / alignment output.
    trace_label: Option<String>,
    model_label: Option<String>,
    /// Index of the original model transition, if this is a sync or model move.
    #[allow(dead_code)]
    model_trans_index: Option<usize>,
    /// Index of the trace event, if this is a sync or log move.
    #[allow(dead_code)]
    trace_event_index: Option<usize>,
}

/// The synchronous product net used for A* search.
///
/// Places are encoded as `Vec<u32>` markings. The first `n_trace_places`
/// entries correspond to trace places, the remaining to model places.
struct SyncProductNet {
    /// Number of trace-specific places (one per trace event + sink).
    n_trace_places: usize,
    /// Number of model places.
    #[allow(dead_code)]
    n_model_places: usize,
    /// Total places.
    #[allow(dead_code)]
    n_places: usize,
    /// Transitions in the sync product.
    transitions: Vec<SyncTransition>,
    /// Initial marking of the sync product (as Vec<u32>).
    initial_marking: Vec<u32>,
    /// Goal marking of the sync product (as Vec<u32>).
    goal_marking: Vec<u32>,
    /// Map from model place name to index in the marking vector.
    #[allow(dead_code)]
    model_place_index: HashMap<String, usize>,
    /// Map from model transition name to index.
    #[allow(dead_code)]
    model_trans_index: HashMap<String, usize>,
}

impl SyncProductNet {
    /// Build the synchronous product net from the Petri net and trace.
    ///
    /// Layout of the marking vector:
    /// ```text
    /// [tp_0, tp_1, ..., tp_{n-1}, tp_sink, mp_0, mp_1, ..., mp_{m-1}]
    /// ```
    /// where `tp_i` are trace places, `tp_sink` is the trace sink place,
    /// and `mp_j` are model places.
    fn build(net: &PetriNet, initial_marking: &Marking, final_marking: &Marking, trace: &Trace) -> Self {
        let n_trace_events = trace.events.len();

        // Trace places: start + one per event
        // Place 0 = start (token initially), Place i+1 = after event i
        let n_trace_places = n_trace_events + 1;

        // Model places
        let mut model_place_index: HashMap<String, usize> = HashMap::new();
        for (i, place) in net.places.iter().enumerate() {
            model_place_index.insert(place.name.clone(), i);
        }
        let n_model_places = net.places.len();

        let n_places = n_trace_places + n_model_places;

        // Model transition index
        let mut model_trans_index: HashMap<String, usize> = HashMap::new();
        for (i, trans) in net.transitions.iter().enumerate() {
            model_trans_index.insert(trans.name.clone(), i);
        }

        // Initial marking: trace place 0 has 1 token + model initial marking
        let mut initial = vec![0u32; n_places];
        initial[0] = 1; // trace start place
        for (place_name, &tokens) in initial_marking {
            if let Some(&idx) = model_place_index.get(place_name) {
                initial[n_trace_places + idx] = tokens;
            }
        }

        // Goal marking: trace position after last event + model final marking
        let mut goal = vec![0u32; n_places];
        goal[n_trace_events] = 1; // after consuming all events
        for (place_name, &tokens) in final_marking {
            if let Some(&idx) = model_place_index.get(place_name) {
                goal[n_trace_places + idx] = tokens;
            }
        }

        let mut transitions: Vec<SyncTransition> = Vec::new();
        let mut next_index = 0usize;

        // Build preset/postset helpers for the model
        // For each model transition, pre/post as (place_name, place_idx)
        let model_trans_pre: Vec<Vec<(String, usize)>> = net.transitions.iter().map(|t| {
            net.arcs.iter()
                .filter(|a| a.target == t.name)
                .filter(|a| net.places.iter().any(|p| p.name == a.source))
                .filter_map(|a| {
                    model_place_index.get(&a.source).map(|&idx| (a.source.clone(), idx))
                })
                .collect()
        }).collect();

        let model_trans_post: Vec<Vec<(String, usize)>> = net.transitions.iter().map(|t| {
            net.arcs.iter()
                .filter(|a| a.source == t.name)
                .filter(|a| net.places.iter().any(|p| p.name == a.target))
                .filter_map(|a| {
                    model_place_index.get(&a.target).map(|&idx| (a.target.clone(), idx))
                })
                .collect()
        }).collect();

        // --- Synchronous transitions (trace_label, model_label) ---
        for (event_idx, event) in trace.events.iter().enumerate() {
            let trace_place_in = event_idx; // place before consuming event
            let trace_place_out = event_idx + 1; // place after consuming event

            for (model_t_idx, model_t) in net.transitions.iter().enumerate() {
                // Only match visible transitions with the same label
                if model_t.label.as_deref() != Some(event.name.as_str()) {
                    continue;
                }

                let mut preset: Vec<(usize, bool, usize)> = Vec::new();
                let mut postset: Vec<(usize, bool, usize)> = Vec::new();

                // Consume token from trace place
                preset.push((trace_place_in, true, trace_place_in));
                // Produce token to next trace place
                postset.push((trace_place_out, true, trace_place_out));

                // Model preset/postset
                for (_, model_p_idx) in &model_trans_pre[model_t_idx] {
                    preset.push((n_trace_places + model_p_idx, false, *model_p_idx));
                }
                for (_, model_p_idx) in &model_trans_post[model_t_idx] {
                    postset.push((n_trace_places + model_p_idx, false, *model_p_idx));
                }

                let idx = next_index;
                next_index += 1;

                transitions.push(SyncTransition {
                    index: idx,
                    preset,
                    postset,
                    cost: STD_SYNC_COST,
                    trace_label: Some(event.name.clone()),
                    model_label: model_t.label.clone(),
                    model_trans_index: Some(model_t_idx),
                    trace_event_index: Some(event_idx),
                });
            }
        }

        // --- Log move transitions (trace_label, >>) ---
        for (event_idx, event) in trace.events.iter().enumerate() {
            let trace_place_in = event_idx;
            let trace_place_out = event_idx + 1;

            let idx = next_index;
            next_index += 1;

            transitions.push(SyncTransition {
                index: idx,
                preset: vec![(trace_place_in, true, trace_place_in)],
                postset: vec![(trace_place_out, true, trace_place_out)],
                cost: STD_LOG_MOVE_COST,
                trace_label: Some(event.name.clone()),
                model_label: None,
                model_trans_index: None,
                trace_event_index: Some(event_idx),
            });
        }

        // --- Model move transitions (>>, model_label) ---
        for (model_t_idx, model_t) in net.transitions.iter().enumerate() {
            let is_invisible = model_t.label.is_none();
            let cost = if is_invisible { STD_TAU_COST } else { STD_MODEL_MOVE_COST };

            let mut preset: Vec<(usize, bool, usize)> = Vec::new();
            let mut postset: Vec<(usize, bool, usize)> = Vec::new();

            for (_, model_p_idx) in &model_trans_pre[model_t_idx] {
                preset.push((n_trace_places + model_p_idx, false, *model_p_idx));
            }
            for (_, model_p_idx) in &model_trans_post[model_t_idx] {
                postset.push((n_trace_places + model_p_idx, false, *model_p_idx));
            }

            let idx = next_index;
            next_index += 1;

            transitions.push(SyncTransition {
                index: idx,
                preset,
                postset,
                cost,
                trace_label: None,
                model_label: model_t.label.clone(),
                model_trans_index: Some(model_t_idx),
                trace_event_index: None,
            });
        }

        SyncProductNet {
            n_trace_places,
            n_model_places,
            n_places,
            transitions,
            initial_marking: initial,
            goal_marking: goal,
            model_place_index,
            model_trans_index,
        }
    }

    /// Check if a transition is enabled in the given marking.
    fn is_enabled(&self, marking: &[u32], trans: &SyncTransition) -> bool {
        trans.preset.iter().all(|&(place_idx, _, _)| marking[place_idx] > 0)
    }

    /// Fire a transition, producing a new marking.
    fn fire(&self, marking: &[u32], trans: &SyncTransition) -> Vec<u32> {
        let mut new_marking = marking.to_vec();
        for &(place_idx, _, _) in &trans.preset {
            new_marking[place_idx] -= 1;
        }
        for &(place_idx, _, _) in &trans.postset {
            new_marking[place_idx] += 1;
        }
        new_marking
    }

    /// Compute an admissible heuristic: lower bound on remaining cost.
    ///
    /// This uses a simplified approach:
    /// - For each model place with excess tokens (above goal), compute the
    ///   minimum cost of any transition that consumes from that place.
    /// - Sum these as a lower bound on the model side.
    /// - For trace places: remaining trace events that haven't been consumed
    ///   each require at minimum a log move (cost = STD_LOG_MOVE_COST).
    ///
    /// This is admissible because every remaining token must be consumed by
    /// at least one transition firing, and we use the minimum cost among
    /// all possible consumer transitions.
    fn heuristic(&self, marking: &[u32]) -> f64 {
        // Quick check: if already at goal, h = 0
        if marking == &self.goal_marking {
            return 0.0;
        }

        let mut h = 0.0f64;

        // Trace side: count remaining trace events not yet consumed
        // Tokens in trace places 0..n_trace_events indicate unconsumed events
        // If trace place i has a token, event i is still pending.
        // We use cost 0 per pending event (admissible: assumes best case = sync move).
        // The model-side deficit calculation already accounts for needing transitions
        // to produce tokens in the right places.
        for i in 0..self.n_trace_places.saturating_sub(1) {
            if marking[i] > 0 {
                h += 0.0;
            }
        }

        // Model side: for places with excess tokens above goal, find minimum
        // cost transition that consumes from that place
        for (place_idx, &goal_tokens) in self.goal_marking.iter().enumerate() {
            let current = marking[place_idx];
            if current <= goal_tokens {
                continue;
            }
            let excess = current - goal_tokens;
            // Find minimum cost transition consuming from this place
            let mut min_cost = f64::INFINITY;
            for trans in &self.transitions {
                let consumes_from = trans.preset.iter().any(|&(p, _, _)| p == place_idx);
                if consumes_from && trans.cost < min_cost {
                    min_cost = trans.cost;
                }
            }
            if min_cost.is_finite() {
                h += min_cost * excess as f64;
            } else {
                // No transition consumes from this place — infeasible path
                // Return infinity to signal this state cannot reach the goal
                return f64::INFINITY;
            }
        }

        // Also check model places with deficit (below goal tokens)
        // These need transitions that produce into them
        for (place_idx, &goal_tokens) in self.goal_marking.iter().enumerate() {
            if place_idx < self.n_trace_places {
                continue; // skip trace places
            }
            let current = marking[place_idx];
            if current < goal_tokens {
                let deficit = goal_tokens - current;
                // Find minimum cost transition producing into this place
                let mut min_cost = f64::INFINITY;
                for trans in &self.transitions {
                    let produces_to = trans.postset.iter().any(|&(p, _, _)| p == place_idx);
                    if produces_to && trans.cost < min_cost {
                        min_cost = trans.cost;
                    }
                }
                if min_cost.is_finite() {
                    h += min_cost * deficit as f64;
                } else {
                    return f64::INFINITY;
                }
            }
        }

        h
    }
}

// ─── A* search state ─────────────────────────────────────────────────────────

/// A state in the A* search.
#[derive(Clone, Debug)]
struct AstarState {
    /// Current marking of the sync product net.
    marking: Vec<u32>,
    /// Cost from start to this state.
    g: f64,
    /// Heuristic estimate from this state to goal.
    h: f64,
    /// Sequence of transition indices fired to reach this state.
    path: Vec<usize>,
}

impl AstarState {
    fn f(&self) -> f64 {
        self.g + self.h
    }
}

/// Wrapper for min-heap ordering (BinaryHeap is a max-heap by default).
struct MinHeapEntry {
    f: f64,
    state: AstarState,
}

impl PartialEq for MinHeapEntry {
    fn eq(&self, other: &Self) -> bool {
        self.f == other.f
    }
}

impl Eq for MinHeapEntry {}

impl PartialOrd for MinHeapEntry {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        // Reverse ordering for min-heap
        other.f.partial_cmp(&self.f)
    }
}

impl Ord for MinHeapEntry {
    fn cmp(&self, other: &Self) -> Ordering {
        // Reverse ordering for min-heap; break ties by g (prefer longer paths)
        other.f.partial_cmp(&self.f).unwrap_or(Ordering::Equal)
            .then_with(|| self.state.g.partial_cmp(&other.state.g).unwrap_or(Ordering::Equal))
    }
}

// ─── Core A* search ──────────────────────────────────────────────────────────

/// Run A* search on the sync product net to find the optimal alignment.
///
/// Returns `None` if no alignment is found within the expansion budget.
fn astar_search(sync_net: &SyncProductNet) -> Option<AstarState> {
    let initial_h = sync_net.heuristic(&sync_net.initial_marking);

    // If heuristic is infinity from the start, the problem is infeasible
    if initial_h.is_infinite() {
        return None;
    }

    let initial_state = AstarState {
        marking: sync_net.initial_marking.clone(),
        g: 0.0,
        h: initial_h,
        path: Vec::new(),
    };

    let mut open: BinaryHeap<MinHeapEntry> = BinaryHeap::new();
    open.push(MinHeapEntry {
        f: initial_state.f(),
        state: initial_state,
    });

    let mut closed: HashSet<Vec<u32>> = HashSet::new();
    let mut expansions = 0usize;

    while let Some(entry) = open.pop() {
        let state = entry.state;

        // Skip if already visited
        if closed.contains(&state.marking) {
            continue;
        }

        // Budget check
        expansions += 1;
        if expansions > MAX_EXPANSIONS {
            return None;
        }

        // Mark as visited
        closed.insert(state.marking.clone());

        // Goal check
        if state.marking == sync_net.goal_marking {
            return Some(state);
        }

        // Expand: try all enabled transitions
        for trans in &sync_net.transitions {
            if !sync_net.is_enabled(&state.marking, trans) {
                continue;
            }

            let new_marking = sync_net.fire(&state.marking, trans);
            let new_g = state.g + trans.cost;

            // Prune: skip if already in closed set
            if closed.contains(&new_marking) {
                continue;
            }

            let new_h = sync_net.heuristic(&new_marking);

            // Prune: if heuristic is infinite, this path is infeasible
            if new_h.is_infinite() {
                continue;
            }

            let mut new_path = state.path.clone();
            new_path.push(trans.index);

            let new_state = AstarState {
                marking: new_marking,
                g: new_g,
                h: new_h,
                path: new_path,
            };

            open.push(MinHeapEntry {
                f: new_state.f(),
                state: new_state,
            });
        }
    }

    // Open set exhausted — no alignment found
    None
}

/// Convert a solved A* path into human-readable alignment moves.
fn path_to_alignment(path: &[usize], sync_net: &SyncProductNet) -> Vec<AlignmentMove> {
    path.iter().map(|&trans_idx| {
        let trans = &sync_net.transitions[trans_idx];
        AlignmentMove {
            trace_activity: trans.trace_label.clone(),
            model_activity: trans.model_label.clone(),
            cost: trans.cost,
        }
    }).collect()
}

// ─── Public API ──────────────────────────────────────────────────────────────

/// Align a single trace against a Petri net using A* search.
///
/// Builds a synchronous product net between the trace and the model, then
/// searches for the minimum-cost alignment. Returns alignment details
/// including cost, fitness, and the sequence of moves.
///
/// # Arguments
/// * `net` - The Petri net model
/// * `initial_marking` - Initial marking of the Petri net
/// * `final_marking` - Final (accepting) marking of the Petri net
/// * `trace` - The trace to align
///
/// # Returns
/// An `AlignmentResult` with the optimal alignment, cost, and fitness.
/// If no alignment is found within the expansion budget, returns a
/// result with `is_fit = false` and maximum cost.
pub fn align_trace(
    net: &PetriNet,
    initial_marking: &Marking,
    final_marking: &Marking,
    trace: &Trace,
) -> AlignmentResult {
    // Handle empty trace
    if trace.events.is_empty() {
        let alignment = Vec::new();
        return AlignmentResult {
            alignment,
            cost: 0.0,
            fitness: 1.0,
            is_fit: true,
        };
    }

    let sync_net = SyncProductNet::build(net, initial_marking, final_marking, trace);

    match astar_search(&sync_net) {
        Some(state) => {
            let alignment = path_to_alignment(&state.path, &sync_net);
            let cost = state.g;
            let is_fit = cost == 0.0;

            // Fitness based on alignment cost:
            // fitness = 1 - cost / (cost + trace_length * SYNC_COST_NORMALIZER)
            // When cost = 0, fitness = 1.0
            // As cost grows, fitness approaches 0.0
            let trace_len = trace.events.len() as f64;
            let fitness = if trace_len == 0.0 {
                1.0
            } else {
                let max_cost = trace_len * STD_LOG_MOVE_COST;
                if max_cost == 0.0 {
                    1.0
                } else {
                    (1.0 - cost / max_cost).max(0.0)
                }
            };

            AlignmentResult {
                alignment,
                cost,
                fitness,
                is_fit,
            }
        }
        None => {
            // No alignment found — return worst case
            AlignmentResult {
                alignment: Vec::new(),
                cost: f64::INFINITY,
                fitness: 0.0,
                is_fit: false,
            }
        }
    }
}

/// Align an entire event log against a Petri net using A* search.
///
/// Aligns each trace independently and aggregates the results.
///
/// # Arguments
/// * `net` - The Petri net model
/// * `initial_marking` - Initial marking of the Petri net
/// * `final_marking` - Final (accepting) marking of the Petri net
/// * `log` - The event log containing traces to align
///
/// # Returns
/// A `LogAlignmentResult` with per-trace results and average fitness.
pub fn align_log(
    net: &PetriNet,
    initial_marking: &Marking,
    final_marking: &Marking,
    log: &EventLog,
) -> LogAlignmentResult {
    let trace_results: Vec<TraceAlignment> = log
        .traces
        .iter()
        .map(|trace| {
            let result = align_trace(net, initial_marking, final_marking, trace);
            TraceAlignment {
                case_id: trace.case_id.clone(),
                alignment: result.alignment,
                cost: result.cost,
                fitness: result.fitness,
                is_fit: result.is_fit,
            }
        })
        .collect();

    let average_fitness = if trace_results.is_empty() {
        1.0
    } else {
        trace_results.iter().map(|r| r.fitness).sum::<f64>() / trace_results.len() as f64
    };

    LogAlignmentResult {
        trace_results,
        average_fitness,
    }
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::event_log::Event;
    use std::collections::HashMap;

    /// Build a simple sequential net: [p_start] → t_A → [p1] → t_B → [p_end]
    fn sequential_net() -> (PetriNet, Marking, Marking) {
        let mut net = PetriNet::new("seq");
        net.add_place("p_start");
        net.add_place("p1");
        net.add_place("p_end");
        net.add_transition("t_A", Some("A".into()));
        net.add_transition("t_B", Some("B".into()));
        net.add_arc("p_start", "t_A");
        net.add_arc("t_A", "p1");
        net.add_arc("p1", "t_B");
        net.add_arc("t_B", "p_end");

        let mut initial = Marking::new();
        initial.insert("p_start".into(), 1);
        let mut final_m = Marking::new();
        final_m.insert("p_end".into(), 1);

        (net, initial, final_m)
    }

    fn make_trace(case_id: &str, acts: &[&str]) -> Trace {
        Trace {
            case_id: case_id.to_string(),
            events: acts
                .iter()
                .map(|&a| Event {
                    name: a.to_string(),
                    timestamp: None,
                    lifecycle: None,
                    attributes: HashMap::new(),
                })
                .collect(),
        }
    }

    /// Build a net with an invisible transition: [p_start] → tau → [p1] → t_A → [p_end]
    fn net_with_invisible() -> (PetriNet, Marking, Marking) {
        let mut net = PetriNet::new("invisible");
        net.add_place("p_start");
        net.add_place("p1");
        net.add_place("p_end");
        net.add_transition("tau_1", None); // invisible
        net.add_transition("t_A", Some("A".into()));
        net.add_arc("p_start", "tau_1");
        net.add_arc("tau_1", "p1");
        net.add_arc("p1", "t_A");
        net.add_arc("t_A", "p_end");

        let mut initial = Marking::new();
        initial.insert("p_start".into(), 1);
        let mut final_m = Marking::new();
        final_m.insert("p_end".into(), 1);

        (net, initial, final_m)
    }

    /// Build a parallel net: [p_start] → t_A → [p_a], [p_start] → t_B → [p_b], [p_a] → t_C → [p_end], [p_b] → t_C → [p_end]
    /// Uses 2 tokens in p_start so both A and B can fire (AND-split semantics).
    fn parallel_net() -> (PetriNet, Marking, Marking) {
        let mut net = PetriNet::new("parallel");
        net.add_place("p_start");
        net.add_place("p_a");
        net.add_place("p_b");
        net.add_place("p_join");
        net.add_place("p_end");
        net.add_transition("t_A", Some("A".into()));
        net.add_transition("t_B", Some("B".into()));
        net.add_transition("t_C", Some("C".into()));
        net.add_arc("p_start", "t_A");
        net.add_arc("p_start", "t_B");
        net.add_arc("t_A", "p_a");
        net.add_arc("t_B", "p_b");
        net.add_arc("p_a", "t_C");
        net.add_arc("p_b", "t_C");
        net.add_arc("t_C", "p_end");

        let mut initial = Marking::new();
        initial.insert("p_start".into(), 2); // 2 tokens for AND-split
        let mut final_m = Marking::new();
        final_m.insert("p_end".into(), 1);

        (net, initial, final_m)
    }

    #[test]
    fn test_perfect_alignment_sequential() {
        let (net, initial, final_m) = sequential_net();
        let trace = make_trace("c1", &["A", "B"]);
        let result = align_trace(&net, &initial, &final_m, &trace);

        assert!(result.is_fit);
        assert!((result.cost - 0.0).abs() < 1e-9);
        assert!((result.fitness - 1.0).abs() < 1e-9);

        // Should have exactly 2 sync moves
        assert_eq!(result.alignment.len(), 2);
        assert_eq!(result.alignment[0].trace_activity.as_deref(), Some("A"));
        assert_eq!(result.alignment[0].model_activity.as_deref(), Some("A"));
        assert_eq!(result.alignment[0].cost, 0.0);
        assert_eq!(result.alignment[1].trace_activity.as_deref(), Some("B"));
        assert_eq!(result.alignment[1].model_activity.as_deref(), Some("B"));
        assert_eq!(result.alignment[1].cost, 0.0);
    }

    #[test]
    fn test_deviating_trace_log_moves() {
        let (net, initial, final_m) = sequential_net();
        // Trace has an activity "C" that is not in the model
        let trace = make_trace("c1", &["A", "C", "B"]);
        let result = align_trace(&net, &initial, &final_m, &trace);

        assert!(!result.is_fit);
        assert!(result.cost > 0.0);
        assert!(result.fitness < 1.0);
        assert!(result.fitness > 0.0);

        // Should have a log move for "C" (model_activity = None)
        let log_moves: Vec<_> = result.alignment.iter()
            .filter(|m| m.model_activity.is_none())
            .collect();
        assert_eq!(log_moves.len(), 1);
        assert_eq!(log_moves[0].trace_activity.as_deref(), Some("C"));
    }

    #[test]
    fn test_extra_activity_in_model_model_moves() {
        // Model has A, B, C but trace only has A, B
        let mut net = PetriNet::new("extra");
        net.add_place("p_start");
        net.add_place("p1");
        net.add_place("p2");
        net.add_place("p_end");
        net.add_transition("t_A", Some("A".into()));
        net.add_transition("t_B", Some("B".into()));
        net.add_transition("t_C", Some("C".into()));
        net.add_arc("p_start", "t_A");
        net.add_arc("t_A", "p1");
        net.add_arc("p1", "t_B");
        net.add_arc("t_B", "p2");
        net.add_arc("p2", "t_C");
        net.add_arc("t_C", "p_end");

        let mut initial = Marking::new();
        initial.insert("p_start".into(), 1);
        let mut final_m = Marking::new();
        final_m.insert("p_end".into(), 1);

        let trace = make_trace("c1", &["A", "B"]);
        let result = align_trace(&net, &initial, &final_m, &trace);

        assert!(!result.is_fit);
        assert!(result.cost > 0.0);

        // Should have a model move for "C" (trace_activity = None)
        let model_moves: Vec<_> = result.alignment.iter()
            .filter(|m| m.trace_activity.is_none())
            .collect();
        assert_eq!(model_moves.len(), 1);
        assert_eq!(model_moves[0].model_activity.as_deref(), Some("C"));
        assert_eq!(model_moves[0].cost, STD_MODEL_MOVE_COST);
    }

    #[test]
    fn test_invisible_transition_handled() {
        let (net, initial, final_m) = net_with_invisible();
        let trace = make_trace("c1", &["A"]);
        let result = align_trace(&net, &initial, &final_m, &trace);

        // The invisible transition must fire before A.
        // The alignment should have a model move for tau (cost = STD_TAU_COST)
        // and a sync move for A (cost = 0).
        assert_eq!(result.alignment.len(), 2);

        let model_moves: Vec<_> = result.alignment.iter()
            .filter(|m| m.trace_activity.is_none())
            .collect();
        assert_eq!(model_moves.len(), 1);
        assert_eq!(model_moves[0].cost, STD_TAU_COST);

        let sync_moves: Vec<_> = result.alignment.iter()
            .filter(|m| m.trace_activity.is_some() && m.model_activity.is_some())
            .collect();
        assert_eq!(sync_moves.len(), 1);
        assert_eq!(sync_moves[0].trace_activity.as_deref(), Some("A"));
    }

    #[test]
    fn test_parallel_net_perfect_trace() {
        let (net, initial, final_m) = parallel_net();
        // Both A and B must fire before C (in any order)
        let trace = make_trace("c1", &["A", "B", "C"]);
        let result = align_trace(&net, &initial, &final_m, &trace);

        assert!(result.is_fit);
        assert!((result.cost - 0.0).abs() < 1e-9);
        assert_eq!(result.alignment.len(), 3);
    }

    #[test]
    fn test_parallel_net_reverse_order() {
        let (net, initial, final_m) = parallel_net();
        // B before A is also valid
        let trace = make_trace("c1", &["B", "A", "C"]);
        let result = align_trace(&net, &initial, &final_m, &trace);

        assert!(result.is_fit);
        assert!((result.cost - 0.0).abs() < 1e-9);
    }

    #[test]
    fn test_empty_trace() {
        let (net, initial, final_m) = sequential_net();
        let trace = make_trace("c1", &[]);
        let result = align_trace(&net, &initial, &final_m, &trace);

        assert!(result.is_fit);
        assert!((result.cost - 0.0).abs() < 1e-9);
        assert!(result.alignment.is_empty());
    }

    #[test]
    fn test_log_level_alignment() {
        let (net, initial, final_m) = sequential_net();
        let log = EventLog {
            traces: vec![
                make_trace("c1", &["A", "B"]),
                make_trace("c2", &["A", "C", "B"]), // C is a log move
            ],
        };
        let result = align_log(&net, &initial, &final_m, &log);

        assert_eq!(result.trace_results.len(), 2);
        assert!(result.trace_results[0].is_fit);
        assert!(!result.trace_results[1].is_fit);
        assert!(result.average_fitness > 0.0);
        assert!(result.average_fitness < 1.0);
    }

    #[test]
    fn test_alignment_result_serialization() {
        let (net, initial, final_m) = sequential_net();
        let trace = make_trace("c1", &["A", "B"]);
        let result = align_trace(&net, &initial, &final_m, &trace);

        // Verify it serializes without error
        let json = serde_json::to_string(&result).unwrap();
        assert!(json.contains("\"alignment\""));
        assert!(json.contains("\"cost\""));
        assert!(json.contains("\"fitness\""));
        assert!(json.contains("\"is_fit\":true"));
    }

    #[test]
    fn test_log_alignment_result_serialization() {
        let (net, initial, final_m) = sequential_net();
        let log = EventLog {
            traces: vec![
                make_trace("c1", &["A", "B"]),
            ],
        };
        let result = align_log(&net, &initial, &final_m, &log);

        let json = serde_json::to_string(&result).unwrap();
        assert!(json.contains("\"trace_results\""));
        assert!(json.contains("\"average_fitness\""));
    }
}
