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

/// Token-based replay conformance checking.
///
/// **Reference**: Rozinat, A., & van der Aalst, W. M. P. (2008). "Conformance
/// Checking of Processes Based on Monitoring Real Behavior." MIS Quarterly,
/// 32(1), 63-76. DOI: 10.2307/25148833
///
/// Implements the classic Rozinat & van der Aalst token replay algorithm.
/// Given a Petri net and an event log, each trace is replayed against the
/// net and a fitness score is computed.
///
/// Fitness formula (per trace):
/// ```text
/// fitness = 0.5 * (1 - missing / consumed) + 0.5 * (1 - remaining / produced)
/// ```
/// where:
/// - `produced`  = tokens added (initial marking + fired transition postsets)
/// - `consumed`  = tokens removed (final marking + fired transition presets, incl. forced)
/// - `missing`   = tokens artificially injected to enable blocked transitions
/// - `remaining` = tokens left in non-final places after replay
use crate::event_log::{EventLog, Trace};
use crate::petri_net::{Marking, PetriNet};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ─── Result types ─────────────────────────────────────────────────────────────

/// Per-trace replay statistics.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TraceReplayResult {
    pub case_id: String,
    /// Fitness in [0.0, 1.0].
    pub fitness: f64,
    /// Convenience flag: `true` when `missing_tokens == 0 && remaining_tokens == 0`.
    pub trace_is_fit: bool,
    pub produced_tokens: u32,
    pub consumed_tokens: u32,
    pub missing_tokens: u32,
    pub remaining_tokens: u32,
    /// Transition names fired during replay, in order.
    pub activated_transitions: Vec<String>,
    /// Final marking after replay (only non-zero token counts).
    pub reached_marking: HashMap<String, u32>,
}

impl TraceReplayResult {
    pub fn is_perfect(&self) -> bool {
        self.missing_tokens == 0 && self.remaining_tokens == 0
    }
}

/// Aggregate fitness result for an entire event log.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FitnessResult {
    /// Global fitness score (weighted average over all tokens).
    pub percentage: f64,
    /// Average per-trace fitness.
    pub avg_trace_fitness: f64,
    /// Number of traces that replay perfectly.
    pub perfectly_fitting_traces: usize,
    /// Total traces in log.
    pub total_traces: usize,
    /// Per-trace breakdown.
    pub trace_results: Vec<TraceReplayResult>,
}

// ─── Petri net helpers ────────────────────────────────────────────────────────

/// Input places (preset) of a transition.
pub(crate) fn preset(net: &PetriNet, trans_name: &str) -> Vec<String> {
    net.arcs
        .iter()
        .filter(|a| a.target == trans_name)
        .filter(|a| net.places.iter().any(|p| p.name == a.source))
        .map(|a| a.source.clone())
        .collect()
}

/// Output places (postset) of a transition.
pub(crate) fn postset(net: &PetriNet, trans_name: &str) -> Vec<String> {
    net.arcs
        .iter()
        .filter(|a| a.source == trans_name)
        .filter(|a| net.places.iter().any(|p| p.name == a.target))
        .map(|a| a.target.clone())
        .collect()
}

/// Test whether a transition is enabled (all preset places have tokens).
pub(crate) fn is_enabled(marking: &Marking, pre: &[String]) -> bool {
    pre.iter().all(|p| marking.get(p).copied().unwrap_or(0) > 0)
}

/// Fire a transition: consume tokens from preset, produce tokens into postset.
/// Returns (consumed, produced).
pub(crate) fn fire(marking: &mut Marking, pre: &[String], post: &[String]) -> (u32, u32) {
    for p in pre {
        *marking.entry(p.clone()).or_insert(0) -= 1;
    }
    for p in post {
        *marking.entry(p.clone()).or_insert(0) += 1;
    }
    (pre.len() as u32, post.len() as u32)
}

// ─── Silent transition firing ─────────────────────────────────────────────────

/// Fire all currently-enabled silent (tau) transitions in a fixed-point loop.
///
/// POWL-generated Petri nets use silent transitions as synchronization barriers
/// (tau-split, tau-join, sync). These must fire before visible transitions can
/// become enabled. We continue until no more silent transitions are enabled.
///
/// A budget cap prevents infinite loops in cyclic nets (e.g. LOOP body).
///
/// Returns (extra_consumed, extra_produced) from the silent firings.
pub(crate) fn fire_silent_enabled(net: &PetriNet, marking: &mut Marking) -> (u32, u32) {
    let mut total_c = 0u32;
    let mut total_p = 0u32;
    let mut budget = net.transitions.len() * 4 + 16; // generous but bounded
    loop {
        if budget == 0 { break; }
        let mut fired = false;
        for trans in &net.transitions {
            if trans.label.is_some() { continue; } // skip visible
            let pre = preset(net, &trans.name);
            if !pre.is_empty() && is_enabled(marking, &pre) {
                let post = postset(net, &trans.name);
                let (c, p) = fire(marking, &pre, &post);
                total_c += c;
                total_p += p;
                budget -= 1;
                fired = true;
                break; // restart to respect new marking
            }
        }
        if !fired { break; }
    }
    (total_c, total_p)
}

// ─── Core replay ─────────────────────────────────────────────────────────────

/// Replay one trace against the Petri net.
pub fn replay_trace(
    net: &PetriNet,
    initial_marking: &Marking,
    final_marking: &Marking,
    trace: &Trace,
) -> TraceReplayResult {
    let mut marking: Marking = initial_marking.clone();

    let mut produced: u32 = initial_marking.values().sum();
    let mut consumed: u32 = 0;
    let mut missing: u32 = 0;
    let mut activated_transitions: Vec<String> = Vec::new();

    // Fire any initially-enabled silent transitions (e.g. tau-splits at the start)
    let (sc, sp) = fire_silent_enabled(net, &mut marking);
    consumed += sc;
    produced += sp;

    for event in &trace.events {
        let activity = &event.name;

        // Find all transitions with matching label
        let candidates: Vec<&str> = net
            .transitions
            .iter()
            .filter(|t| t.label.as_deref() == Some(activity.as_str()))
            .map(|t| t.name.as_str())
            .collect();

        if candidates.is_empty() {
            // Activity not in net — skip (invisible to conformance)
            continue;
        }

        // Try to find an enabled candidate
        let enabled_trans = candidates
            .iter()
            .find(|&&t| is_enabled(&marking, &preset(net, t)))
            .copied();

        let chosen = if let Some(t) = enabled_trans {
            t
        } else {
            // No enabled candidate — pick first, force-enable by adding missing tokens
            candidates[0]
        };

        let pre = preset(net, chosen);
        let post = postset(net, chosen);

        // Force-enable: inject any missing tokens
        for p in &pre {
            let have = marking.get(p).copied().unwrap_or(0);
            if have == 0 {
                *marking.entry(p.clone()).or_insert(0) += 1;
                produced += 1;
                missing += 1;
            }
        }

        let (c, p) = fire(&mut marking, &pre, &post);
        consumed += c;
        produced += p;
        activated_transitions.push(chosen.to_string());

        // Fire any newly-enabled silent transitions after visible transition
        let (sc, sp) = fire_silent_enabled(net, &mut marking);
        consumed += sc;
        produced += sp;
    }

    // After replay: tokens remaining in non-final places
    let remaining: u32 = marking
        .iter()
        .filter(|(place, &tokens)| {
            tokens > 0 && final_marking.get(*place).copied().unwrap_or(0) == 0
        })
        .map(|(_, &t)| t)
        .sum();

    // Consume tokens in final places
    let final_consumed: u32 = final_marking.values().sum();
    consumed += final_consumed;

    // Compute fitness
    let trace_is_fit = missing == 0 && remaining == 0;
    let fitness = if produced == 0 && consumed == 0 {
        if missing == 0 { 1.0 } else { 0.0 }
    } else {
        let c = consumed as f64;
        let p = produced as f64;
        let m = missing as f64;
        let r = remaining as f64;
        0.5 * (1.0 - m / c) + 0.5 * (1.0 - r / p)
    }
    .clamp(0.0, 1.0);

    // Capture the final marking state (only non-zero token counts)
    let reached_marking: HashMap<String, u32> = marking
        .iter()
        .filter(|(_, &v)| v > 0)
        .map(|(k, &v)| (k.clone(), v))
        .collect();

    TraceReplayResult {
        case_id: trace.case_id.clone(),
        fitness,
        trace_is_fit,
        produced_tokens: produced,
        consumed_tokens: consumed,
        missing_tokens: missing,
        remaining_tokens: remaining,
        activated_transitions,
        reached_marking,
    }
}

// ─── Log-level entry point ────────────────────────────────────────────────────

/// Compute token-replay fitness for every trace in `log`.
pub fn compute_fitness(
    net: &PetriNet,
    initial_marking: &Marking,
    final_marking: &Marking,
    log: &EventLog,
) -> FitnessResult {
    let trace_results: Vec<TraceReplayResult> = log
        .traces
        .iter()
        .map(|t| replay_trace(net, initial_marking, final_marking, t))
        .collect();

    let perfectly_fitting_traces = trace_results.iter().filter(|r| r.is_perfect()).count();
    let total_traces = trace_results.len();

    let avg_trace_fitness = if total_traces == 0 {
        1.0
    } else {
        trace_results.iter().map(|r| r.fitness).sum::<f64>() / total_traces as f64
    };

    // Global (token-weighted) fitness
    let total_produced: u32 = trace_results.iter().map(|r| r.produced_tokens).sum();
    let total_consumed: u32 = trace_results.iter().map(|r| r.consumed_tokens).sum();
    let total_missing: u32 = trace_results.iter().map(|r| r.missing_tokens).sum();
    let total_remaining: u32 = trace_results.iter().map(|r| r.remaining_tokens).sum();

    let percentage = if total_produced == 0 && total_consumed == 0 {
        1.0
    } else {
        let c = total_consumed as f64;
        let p = total_produced as f64;
        let m = total_missing as f64;
        let r = total_remaining as f64;
        (0.5 * (1.0 - m / c) + 0.5 * (1.0 - r / p)).clamp(0.0, 1.0)
    };

    FitnessResult {
        percentage,
        avg_trace_fitness,
        perfectly_fitting_traces,
        total_traces,
        trace_results,
    }
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::petri_net::PetriNet;

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
        use crate::event_log::Event;
        use std::collections::HashMap;
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

    #[test]
    fn perfect_trace_fitness_1() {
        let (net, initial, final_m) = sequential_net();
        let trace = make_trace("c1", &["A", "B"]);
        let result = replay_trace(&net, &initial, &final_m, &trace);
        assert_eq!(result.missing_tokens, 0);
        assert_eq!(result.remaining_tokens, 0);
        assert!((result.fitness - 1.0).abs() < 1e-9);
        assert!(result.is_perfect());
    }

    #[test]
    fn missing_activity_lowers_fitness() {
        let (net, initial, final_m) = sequential_net();
        // Trace only has A, skips B
        let trace = make_trace("c1", &["A"]);
        let result = replay_trace(&net, &initial, &final_m, &trace);
        // p1 has a token but isn't the final place → remaining = 1
        assert_eq!(result.remaining_tokens, 1);
        assert!(result.fitness < 1.0);
    }

    #[test]
    fn extra_activity_forces_missing_token() {
        let (net, initial, final_m) = sequential_net();
        // B comes before A — B's preset (p1) is empty → missing token
        let trace = make_trace("c1", &["B", "A"]);
        let result = replay_trace(&net, &initial, &final_m, &trace);
        assert!(result.missing_tokens > 0);
        assert!(result.fitness < 1.0);
    }

    #[test]
    fn log_level_fitness_all_perfect() {
        let (net, initial, final_m) = sequential_net();
        let log = EventLog {
            traces: vec![
                make_trace("c1", &["A", "B"]),
                make_trace("c2", &["A", "B"]),
            ],
        };
        let result = compute_fitness(&net, &initial, &final_m, &log);
        assert_eq!(result.perfectly_fitting_traces, 2);
        assert!((result.percentage - 1.0).abs() < 1e-9);
        assert!((result.avg_trace_fitness - 1.0).abs() < 1e-9);
    }

    #[test]
    fn log_level_fitness_mixed() {
        let (net, initial, final_m) = sequential_net();
        let log = EventLog {
            traces: vec![
                make_trace("c1", &["A", "B"]),  // perfect
                make_trace("c2", &["A"]),        // imperfect
            ],
        };
        let result = compute_fitness(&net, &initial, &final_m, &log);
        assert_eq!(result.perfectly_fitting_traces, 1);
        assert_eq!(result.total_traces, 2);
        assert!(result.percentage < 1.0 && result.percentage > 0.0);
    }
}
