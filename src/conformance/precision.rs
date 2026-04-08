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

//! ETConformance precision metric.
//!
//! Measures how precisely a Petri net model describes the behavior observed
//! in the event log. Based on the escaping-edges approach: transitions that
//! are enabled in the model at some replay step but never actually fired are
//! "escaping edges." A model with many escaping edges is *underfitting* (too
//! permissive), and the precision score drops accordingly.
//!
//! Formula (aggregated over all traces):
//!
//! ```text
//! precision = 1 - sum(escaping) / (sum(escaping) + sum(consumed))
//! ```
//!
//! The result is clamped to [0.0, 1.0]. An empty log yields precision = 1.0.

use crate::conformance::token_replay::{fire, fire_silent_enabled, is_enabled, postset, preset};
use crate::event_log::{EventLog, Trace};
use crate::petri_net::{Marking, PetriNet};
use serde::{Deserialize, Serialize};

// ─── Result types ─────────────────────────────────────────────────────────────

/// Precision result from ETConformance analysis.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PrecisionResult {
    /// Overall precision score in [0.0, 1.0].
    pub precision: f64,
    /// Total escaping tokens across all traces.
    pub total_escaping: u32,
    /// Total consumed tokens across all traces.
    pub total_consumed: u32,
    /// Number of traces analyzed.
    pub total_traces: usize,
}

// ─── Per-trace computation ────────────────────────────────────────────────────

/// Compute escaping and consumed token counts for a single trace.
///
/// After each visible transition fires (and silent transitions are eagerly
/// resolved), we count how many *other* transitions are currently enabled but
/// will **not** be fired for the current event. Each such transition's preset
/// size contributes to the "escaping" total.
fn precision_for_trace(
    net: &PetriNet,
    initial_marking: &Marking,
    final_marking: &Marking,
    trace: &Trace,
) -> (u32, u32) {
    let mut marking: Marking = initial_marking.clone();
    let mut consumed: u32 = 0;
    let mut escaping: u32 = 0;

    // Fire any initially-enabled silent transitions
    fire_silent_enabled(net, &mut marking);

    for event in &trace.events {
        let activity = &event.name;

        // Find visible transitions matching the activity label
        let visible_candidates: Vec<&str> = net
            .transitions
            .iter()
            .filter(|t| t.label.as_deref() == Some(activity.as_str()))
            .map(|t| t.name.as_str())
            .collect();

        if visible_candidates.is_empty() {
            // Activity not in net — skip (invisible to conformance)
            continue;
        }

        // Pick the first enabled candidate; force-enable if none are ready
        let chosen = if let Some(&t) = visible_candidates
            .iter()
            .find(|&&t| is_enabled(&marking, &preset(net, t)))
        {
            t
        } else {
            // No enabled candidate — inject missing tokens to force-enable
            for p in &preset(net, visible_candidates[0]) {
                let have = marking.get(p).copied().unwrap_or(0);
                if have == 0 {
                    *marking.entry(p.clone()).or_insert(0) += 1;
                }
            }
            visible_candidates[0]
        };

        let pre = preset(net, chosen);
        let post = postset(net, chosen);
        fire(&mut marking, &pre, &post);
        consumed += pre.len() as u32;

        // Fire any newly-enabled silent transitions
        fire_silent_enabled(net, &mut marking);

        // Count escaping edges: transitions enabled now that will NOT be fired
        // for the current event. Each enabled transition's preset size counts
        // as escaping tokens.
        for trans in &net.transitions {
            let trans_pre = preset(net, &trans.name);
            if !trans_pre.is_empty() && is_enabled(&marking, &trans_pre) {
                // This transition is enabled but won't be fired for this event
                if trans.label.as_deref() != Some(activity.as_str()) {
                    escaping += trans_pre.len() as u32;
                }
            }
        }
    }

    // Account for final marking consumption
    let final_consumed: u32 = final_marking.values().sum();
    consumed += final_consumed;

    (escaping, consumed)
}

// ─── Log-level entry point ────────────────────────────────────────────────────

/// Compute ETConformance precision for an event log against a Petri net.
///
/// Returns a precision score between 0.0 (model allows much more behavior
/// than observed) and 1.0 (model exactly matches observed behavior).
///
/// Mirrors `pm4py.precision_etconformance()`.
pub fn compute_precision(
    net: &PetriNet,
    initial_marking: &Marking,
    final_marking: &Marking,
    log: &EventLog,
) -> PrecisionResult {
    let mut total_escaping: u32 = 0;
    let mut total_consumed: u32 = 0;
    let total_traces = log.traces.len();

    for trace in &log.traces {
        let (escaping, consumed) =
            precision_for_trace(net, initial_marking, final_marking, trace);
        total_escaping += escaping;
        total_consumed += consumed;
    }

    let precision = if total_consumed == 0 && total_escaping == 0 {
        1.0
    } else {
        let e = total_escaping as f64;
        let c = total_consumed as f64;
        (1.0 - e / (e + c)).clamp(0.0, 1.0)
    };

    PrecisionResult {
        precision,
        total_escaping,
        total_consumed,
        total_traces,
    }
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::event_log::Event;
    use crate::petri_net::PetriNet;
    use std::collections::HashMap;

    /// Build a simple sequential net: [p_start] -> t_A -> [p1] -> t_B -> [p_end]
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

    fn make_log(cases: &[&[&str]]) -> EventLog {
        EventLog {
            traces: cases
                .iter()
                .enumerate()
                .map(|(i, acts)| Trace {
                    case_id: format!("c{}", i + 1),
                    events: acts
                        .iter()
                        .map(|&a| Event {
                            name: a.to_string(),
                            timestamp: None,
                            lifecycle: None,
                            attributes: HashMap::new(),
                        })
                        .collect(),
                })
                .collect(),
        }
    }

    #[test]
    fn test_perfect_log_high_precision() {
        let (net, initial, final_m) = sequential_net();
        let log = make_log(&[&["A", "B"], &["A", "B"]]);
        let result = compute_precision(&net, &initial, &final_m, &log);
        // Sequential net with matching traces should have high precision
        assert!(result.precision >= 0.5);
    }

    #[test]
    fn test_precision_between_zero_and_one() {
        let (net, initial, final_m) = sequential_net();
        let log = make_log(&[&["A", "B"]]);
        let result = compute_precision(&net, &initial, &final_m, &log);
        assert!(result.precision >= 0.0);
        assert!(result.precision <= 1.0);
    }

    #[test]
    fn test_empty_log_returns_one() {
        let (net, initial, final_m) = sequential_net();
        let log = make_log(&[]);
        let result = compute_precision(&net, &initial, &final_m, &log);
        assert!((result.precision - 1.0).abs() < 1e-9);
        assert_eq!(result.total_escaping, 0);
        assert_eq!(result.total_consumed, 0);
        assert_eq!(result.total_traces, 0);
    }

    #[test]
    fn test_single_trace_count() {
        let (net, initial, final_m) = sequential_net();
        let log = make_log(&[&["A", "B"]]);
        let result = compute_precision(&net, &initial, &final_m, &log);
        assert_eq!(result.total_traces, 1);
    }
}
