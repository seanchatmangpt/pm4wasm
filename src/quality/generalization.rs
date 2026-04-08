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

//! Generalization quality metric for process models.
//!
//! **Reference**: Buijs, J. C. A. M., van der Aalst, W. M. P., et al. (2012).
//! "A Genetic Perspective on Process Discovery: Towards Quality-Aware Process Mining."
//! International Journal of Business Process Integration and Management, 1(2), 63-76.
//! DOI: 10.1504/IJBPIM.2012.048807
//!
//! Measures how well a Petri net generalises to unseen behaviour, avoiding
//! overfitting to the observed log.  The algorithm mirrors the pm4py
//! token-based generalization: transitions that fire rarely or not at all
//! contribute a penalty of `1 / sqrt(count)`.  A model where every transition
//! fires frequently scores close to 1.0; a model with many unused transitions
//! scores close to 0.0.

use crate::conformance::token_replay::compute_fitness;
use crate::event_log::EventLog;
use crate::petri_net::{Marking, PetriNet};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ─── Public types ───────────────────────────────────────────────────────────

/// Quality metrics for a process model evaluated against an event log.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct QualityMetrics {
    /// Generalization score in [0, 1].
    pub generalization: f64,
    /// Number of places in the model.
    pub num_places: usize,
    /// Number of transitions in the model.
    pub num_transitions: usize,
    /// Number of arcs in the model.
    pub num_arcs: usize,
}

// ─── Core algorithm ─────────────────────────────────────────────────────────

/// Compute generalization and structural quality metrics for a Petri net
/// against an event log.
///
/// The generalization score uses the token-replay approach from
/// `pm4py.algo.evaluation.generalization.variants.token_based`:
///
/// 1. Replay every trace to get activated transitions per trace.
/// 2. Count how often each transition fires across the entire log.
/// 3. For each transition: if it fired `n` times, add `1 / sqrt(n)`;
///    if it never fired, add `1`.
/// 4. `generalization = 1 - penalty_sum / num_visible_transitions`
///
/// This penalises models with many rarely-used or unused transitions
/// (overfitting) and rewards models where all transitions are exercised.
pub fn compute_quality(
    net: &PetriNet,
    initial_marking: &Marking,
    final_marking: &Marking,
    log: &EventLog,
) -> QualityMetrics {
    let num_transitions = net.transitions.len();
    let generalization = if num_transitions == 0 {
        1.0
    } else {
        compute_generalization(net, initial_marking, final_marking, log)
    };
    QualityMetrics {
        generalization,
        num_places: net.places.len(),
        num_transitions,
        num_arcs: net.arcs.len(),
    }
}

/// Internal generalization computation.
fn compute_generalization(
    net: &PetriNet,
    initial_marking: &Marking,
    final_marking: &Marking,
    log: &EventLog,
) -> f64 {
    let replay = compute_fitness(net, initial_marking, final_marking, log);

    // Count per-transition firing frequency across all traces.
    let mut trans_occ: HashMap<String, u64> = HashMap::new();
    for trace_result in &replay.trace_results {
        for t_name in &trace_result.activated_transitions {
            *trans_occ.entry(t_name.clone()).or_insert(0) += 1;
        }
    }

    // Sum penalty: 1/sqrt(n) per visible transition (silent excluded).
    let mut penalty_sum = 0.0_f64;
    for trans in &net.transitions {
        if trans.label.is_none() {
            continue;
        }
        let count = trans_occ.get(&trans.name).copied().unwrap_or(0);
        penalty_sum += if count > 0 { 1.0 / (count as f64).sqrt() } else { 1.0 };
    }

    let visible_count = net.transitions.iter().filter(|t| t.label.is_some()).count();
    if visible_count == 0 { return 1.0; }
    1.0 - penalty_sum / visible_count as f64
}

// ─── Tests ──────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::event_log::{Event, Trace};
    /// Build a sequential net: source -> A -> p1 -> B -> sink.
    fn seq_net() -> (PetriNet, Marking, Marking) {
        let mut net = PetriNet::new("seq");
        net.add_place("source"); net.add_place("p1"); net.add_place("sink");
        net.add_transition("tA", Some("A".into()));
        net.add_transition("tB", Some("B".into()));
        net.add_arc("source", "tA"); net.add_arc("tA", "p1");
        net.add_arc("p1", "tB"); net.add_arc("tB", "sink");
        let mut im = Marking::new(); im.insert("source".into(), 1);
        let mut fm = Marking::new(); fm.insert("sink".into(), 1);
        (net, im, fm)
    }

    /// Same as seq_net but with an extra unused visible transition C.
    fn seq_net_with_unused() -> (PetriNet, Marking, Marking) {
        let mut net = PetriNet::new("seq+extra");
        net.add_place("source"); net.add_place("p1"); net.add_place("p2"); net.add_place("sink");
        net.add_transition("tA", Some("A".into()));
        net.add_transition("tB", Some("B".into()));
        net.add_transition("tC", Some("C".into()));
        net.add_arc("source", "tA"); net.add_arc("tA", "p1");
        net.add_arc("p1", "tB"); net.add_arc("tB", "p2"); net.add_arc("p2", "sink");
        net.add_arc("source", "tC"); net.add_arc("tC", "sink");
        let mut im = Marking::new(); im.insert("source".into(), 1);
        let mut fm = Marking::new(); fm.insert("sink".into(), 1);
        (net, im, fm)
    }

    fn make_event(name: &str) -> Event {
        Event { name: name.into(), timestamp: None, lifecycle: None, attributes: HashMap::new() }
    }

    fn log_ab(n: usize) -> EventLog {
        EventLog {
            traces: (0..n).map(|i| Trace {
                case_id: i.to_string(),
                events: vec![make_event("A"), make_event("B")],
            }).collect(),
        }
    }

    #[test]
    fn test_generalization_perfect_fit() {
        let (net, im, fm) = seq_net();
        let m = compute_quality(&net, &im, &fm, &log_ab(10));
        assert!(m.generalization > 0.5, "expected > 0.5, got {:.4}", m.generalization);
        assert_eq!(m.num_places, 3);
        assert_eq!(m.num_transitions, 2);
        assert_eq!(m.num_arcs, 4);
    }

    #[test]
    fn test_generalization_unused_transition_lowers_score() {
        let (net_ok, im_ok, fm_ok) = seq_net();
        let (net_extra, im_extra, fm_extra) = seq_net_with_unused();
        let log = log_ab(10);
        let ok = compute_quality(&net_ok, &im_ok, &fm_ok, &log);
        let extra = compute_quality(&net_extra, &im_extra, &fm_extra, &log);
        assert!(ok.generalization > extra.generalization,
            "clean ({:.4}) should beat extra ({:.4})", ok.generalization, extra.generalization);
    }
}
