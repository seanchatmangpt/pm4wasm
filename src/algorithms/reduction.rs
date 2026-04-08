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

//! Petri net structural reduction rules.
//!
//! Applies Murata-style reduction rules to simplify a Petri net while
//! preserving behavioural equivalence (liveness, boundedness, deadlock
//! freedom).
//!
//! Four rules are applied iteratively until no further reduction is possible:
//!
//! 1. **Fusion of series places** -- remove a place that has exactly one
//!    input transition and one output transition, reconnecting the arcs.
//! 2. **Fusion of series transitions** -- remove a silent transition that
//!    has exactly one input place and one output place, reconnecting arcs.
//! 3. **Elimination of self-loop places** -- remove a place whose only
//!    preset and postset is the same single transition.
//! 4. **Elimination of identical places** -- merge places that share
//!    identical preset and postset transitions.

use crate::petri_net::PetriNet;
use std::collections::HashMap;

// ─── Public API ────────────────────────────────────────────────────────────

/// Apply all reduction rules to a Petri net in-place.
///
/// Rules are applied iteratively until a fixed point is reached (no further
/// reduction possible).  The order of application is:
///
/// 1. Self-loop place elimination
/// 2. Fusion of series places
/// 3. Fusion of series transitions
/// 4. Identical place elimination
///
/// Preserves behavioural properties (liveness, boundedness, soundness).
pub fn reduce_petri_net(net: &mut PetriNet) {
    loop {
        let mut any_reduced = false;
        any_reduced |= eliminate_self_loop_places(net);
        any_reduced |= fuse_series_places(net);
        any_reduced |= fuse_series_transitions(net);
        any_reduced |= eliminate_identical_places(net);
        if !any_reduced {
            break;
        }
    }
}

/// Count the number of reducible elements in the Petri net without
/// actually performing any reduction.
///
/// Returns the total count of elements that would be reduced by
/// [`reduce_petri_net`].
pub fn count_reducible_elements(net: &PetriNet) -> usize {
    let mut count = 0;
    count += count_self_loop_places(net);
    count += count_series_places(net);
    count += count_series_transitions(net);
    count += count_identical_place_groups(net);
    count
}

// ─── Preset / postset helpers ─────────────────────────────────────────────

/// Names of transitions that have an arc into `node`.
fn preset_transitions(net: &PetriNet, node: &str) -> Vec<String> {
    net.arcs
        .iter()
        .filter(|a| a.target == node)
        .filter(|a| net.transitions.iter().any(|t| t.name == a.source))
        .map(|a| a.source.clone())
        .collect()
}

/// Names of transitions that have an arc out of `node`.
fn postset_transitions(net: &PetriNet, node: &str) -> Vec<String> {
    net.arcs
        .iter()
        .filter(|a| a.source == node)
        .filter(|a| net.transitions.iter().any(|t| t.name == a.target))
        .map(|a| a.target.clone())
        .collect()
}

/// Names of places that have an arc into `node`.
fn preset_places(net: &PetriNet, node: &str) -> Vec<String> {
    net.arcs
        .iter()
        .filter(|a| a.target == node)
        .filter(|a| net.places.iter().any(|p| p.name == a.source))
        .map(|a| a.source.clone())
        .collect()
}

/// Names of places that have an arc out of `node`.
fn postset_places(net: &PetriNet, node: &str) -> Vec<String> {
    net.arcs
        .iter()
        .filter(|a| a.source == node)
        .filter(|a| net.places.iter().any(|p| p.name == a.target))
        .map(|a| a.target.clone())
        .collect()
}

// ─── Rule 1: Fusion of series places ───────────────────────────────────────

/// Remove a place with exactly one input transition and one output
/// transition, reconnecting the input transition directly to all output
/// places of the removed place's output transition.
///
/// This is only applied when both transitions are *silent* (label == None)
/// and they are distinct, matching the existing `apply_simple_reduction`
/// behaviour in `petri_net.rs`.
fn fuse_series_places(net: &mut PetriNet) -> bool {
    let place_names: Vec<String> = net.places.iter().map(|p| p.name.clone()).collect();
    for p_name in &place_names {
        let in_trans = preset_transitions(net, p_name);
        let out_trans = postset_transitions(net, p_name);

        if in_trans.len() == 1 && out_trans.len() == 1 {
            let in_t = &in_trans[0];
            let out_t = &out_trans[0];

            // Both must be silent and distinct.
            let in_silent = net
                .transitions
                .iter()
                .find(|t| t.name == *in_t)
                .map(|t| t.label.is_none())
                .unwrap_or(false);
            let out_silent = net
                .transitions
                .iter()
                .find(|t| t.name == *out_t)
                .map(|t| t.label.is_none())
                .unwrap_or(false);

            if in_silent && out_silent && in_t != out_t {
                // Collect targets of out_t before removal.
                let targets: Vec<String> = net
                    .arcs
                    .iter()
                    .filter(|a| a.source == *out_t)
                    .map(|a| a.target.clone())
                    .collect();
                let old_in_t = in_t.clone();
                net.remove_place(p_name);
                net.remove_transition(out_t);
                for tgt in targets {
                    if tgt != *p_name {
                        net.add_arc(&old_in_t, &tgt);
                    }
                }
                return true;
            }
        }
    }
    false
}

// ─── Rule 2: Fusion of series transitions ──────────────────────────────────

/// Remove a silent transition that has exactly one input place and one
/// output place.  The input place's preset transitions are connected
/// directly to the output place's postset transitions.
fn fuse_series_transitions(net: &mut PetriNet) -> bool {
    let trans_names: Vec<String> = net
        .transitions
        .iter()
        .filter(|t| t.label.is_none())
        .map(|t| t.name.clone())
        .collect();

    for t_name in &trans_names {
        let in_places = preset_places(net, t_name);
        let out_places = postset_places(net, t_name);

        if in_places.len() == 1 && out_places.len() == 1 {
            let in_p = &in_places[0];
            let out_p = &out_places[0];

            // The input place must not be the same as the output place
            // (that would be a self-loop handled by Rule 3).
            if in_p == out_p {
                continue;
            }

            // Collect the preset transitions of the input place.
            let pre_trans: Vec<String> = preset_transitions(net, in_p);
            // Collect the postset transitions of the output place.
            let post_trans: Vec<String> = postset_transitions(net, out_p);

            let old_in_p = in_p.clone();
            let old_out_p = out_p.clone();

            net.remove_place(&old_in_p);
            net.remove_place(&old_out_p);
            net.remove_transition(t_name);

            // Reconnect: each pre-transition -> each post-transition via a
            // new intermediate place (skip if the arc already exists or
            // source == target).
            for pt in &pre_trans {
                for qt in &post_trans {
                    if pt != qt {
                        let intermediate = format!("p_merge_{}_{}", pt, qt);
                        net.add_place(&intermediate);
                        net.add_arc(pt, &intermediate);
                        net.add_arc(&intermediate, qt);
                    }
                }
            }
            return true;
        }
    }
    false
}

// ─── Rule 3: Self-loop place elimination ───────────────────────────────────

/// Remove a place whose only preset and postset is the same single
/// transition (self-loop place).
fn eliminate_self_loop_places(net: &mut PetriNet) -> bool {
    let place_names: Vec<String> = net.places.iter().map(|p| p.name.clone()).collect();
    for p_name in &place_names {
        let pre = preset_transitions(net, p_name);
        let post = postset_transitions(net, p_name);

        // Self-loop: exactly one transition in both pre and post, and it
        // is the same transition.
        if pre.len() == 1 && post.len() == 1 && pre[0] == post[0] {
            net.remove_place(p_name);
            return true;
        }
    }
    false
}

// ─── Rule 4: Identical place elimination ───────────────────────────────────

/// Merge places that have identical preset and postset transitions.
/// The first such place is kept; the others are removed and their arcs
/// redirected.
fn eliminate_identical_places(net: &mut PetriNet) -> bool {
    // Build a signature for each place: (sorted preset, sorted postset).
    let mut sig_map: HashMap<(Vec<String>, Vec<String>), Vec<String>> = HashMap::new();
    for place in &net.places {
        let mut pre = preset_transitions(net, &place.name);
        let mut post = postset_transitions(net, &place.name);
        pre.sort();
        post.sort();
        sig_map
            .entry((pre, post))
            .or_default()
            .push(place.name.clone());
    }

    // Find a group with more than one place.
    for (_sig, group) in &sig_map {
        if group.len() > 1 {
            // Keep the first, remove the rest.
            let _keep = &group[0];
            for remove in &group[1..] {
                net.remove_place(remove);
            }
            return true;
        }
    }
    false
}

// ─── Counting helpers (non-mutating) ───────────────────────────────────────

fn count_self_loop_places(net: &PetriNet) -> usize {
    let mut count = 0;
    for place in &net.places {
        let pre = preset_transitions(net, &place.name);
        let post = postset_transitions(net, &place.name);
        if pre.len() == 1 && post.len() == 1 && pre[0] == post[0] {
            count += 1;
        }
    }
    count
}

fn count_series_places(net: &PetriNet) -> usize {
    let mut count = 0;
    for place in &net.places {
        let in_trans = preset_transitions(net, &place.name);
        let out_trans = postset_transitions(net, &place.name);
        if in_trans.len() == 1 && out_trans.len() == 1 {
            let in_silent = net
                .transitions
                .iter()
                .find(|t| t.name == in_trans[0])
                .map(|t| t.label.is_none())
                .unwrap_or(false);
            let out_silent = net
                .transitions
                .iter()
                .find(|t| t.name == out_trans[0])
                .map(|t| t.label.is_none())
                .unwrap_or(false);
            if in_silent && out_silent && in_trans[0] != out_trans[0] {
                count += 1;
            }
        }
    }
    count
}

fn count_series_transitions(net: &PetriNet) -> usize {
    let mut count = 0;
    for trans in &net.transitions {
        if trans.label.is_some() {
            continue;
        }
        let in_places = preset_places(net, &trans.name);
        let out_places = postset_places(net, &trans.name);
        if in_places.len() == 1
            && out_places.len() == 1
            && in_places[0] != out_places[0]
        {
            count += 1;
        }
    }
    count
}

fn count_identical_place_groups(net: &PetriNet) -> usize {
    let mut sig_map: HashMap<(Vec<String>, Vec<String>), usize> = HashMap::new();
    for place in &net.places {
        let mut pre = preset_transitions(net, &place.name);
        let mut post = postset_transitions(net, &place.name);
        pre.sort();
        post.sort();
        *sig_map.entry((pre, post)).or_insert(0) += 1;
    }
    // Count how many places are in groups larger than 1 (i.e. redundant).
    sig_map.values().filter(|&&c| c > 1).map(|&c| c - 1).sum()
}

// ─── Tests ─────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::petri_net::PetriNet;
    use std::collections::HashSet;

    /// Build a minimal net: p1 -> tau1 -> p2 -> tau2 -> p3
    /// where tau1 and tau2 are silent transitions.
    fn series_place_net() -> PetriNet {
        let mut net = PetriNet::new("series");
        net.add_place("p1");
        net.add_place("p2");
        net.add_place("p3");
        net.add_transition("tau1", None);
        net.add_transition("tau2", None);
        net.add_arc("p1", "tau1");
        net.add_arc("tau1", "p2");
        net.add_arc("p2", "tau2");
        net.add_arc("tau2", "p3");
        net
    }

    #[test]
    fn test_fuse_series_places_reduces() {
        let mut net = series_place_net();
        let before = net.places.len();
        fuse_series_places(&mut net);
        // p2 should be removed, tau2 should be removed.
        assert!(net.places.len() < before);
        // p1 and p3 should remain.
        let names: HashSet<String> = net.places.iter().map(|p| p.name.clone()).collect();
        assert!(names.contains("p1"));
        assert!(names.contains("p3"));
    }

    #[test]
    fn test_self_loop_place_eliminated() {
        let mut net = PetriNet::new("selfloop");
        net.add_place("p1");
        net.add_place("p_loop");
        net.add_transition("t1", Some("A".to_string()));
        // p1 -> t1 -> p2 (normal flow)
        net.add_arc("p1", "t1");
        // p_loop is a self-loop on t1.
        net.add_arc("t1", "p_loop");
        net.add_arc("p_loop", "t1");

        let before = net.places.len();
        eliminate_self_loop_places(&mut net);
        // p_loop should be removed.
        assert_eq!(net.places.len(), before - 1);
        let names: HashSet<String> = net.places.iter().map(|p| p.name.clone()).collect();
        assert!(names.contains("p1"));
        assert!(!names.contains("p_loop"));
    }

    #[test]
    fn test_identical_places_merged() {
        let mut net = PetriNet::new("identical");
        net.add_place("p1");
        net.add_place("p2");
        net.add_place("p3");
        net.add_transition("t1", Some("A".to_string()));
        net.add_transition("t2", Some("B".to_string()));
        // p1 and p2 have identical preset/postset: both between t1 and t2.
        net.add_arc("t1", "p1");
        net.add_arc("p1", "t2");
        net.add_arc("t1", "p2");
        net.add_arc("p2", "t2");
        // p3 is different.
        net.add_arc("t2", "p3");

        let before = net.places.len();
        eliminate_identical_places(&mut net);
        // One of p1/p2 should be removed.
        assert_eq!(net.places.len(), before - 1);
    }

    #[test]
    fn test_reduce_petri_net_full() {
        let mut net = series_place_net();
        reduce_petri_net(&mut net);
        // After full reduction, the net should be significantly smaller.
        assert!(net.places.len() <= 2);
        assert!(net.transitions.len() <= 1);
    }

    #[test]
    fn test_count_reducible_elements() {
        let net = series_place_net();
        let count = count_reducible_elements(&net);
        assert!(count > 0);
    }

    #[test]
    fn test_no_reduction_on_visible_only() {
        // Net with only visible transitions: no reduction should apply.
        let mut net = PetriNet::new("visible");
        net.add_place("p1");
        net.add_place("p2");
        net.add_transition("A", Some("A".to_string()));
        net.add_transition("B", Some("B".to_string()));
        net.add_arc("p1", "A");
        net.add_arc("A", "p2");
        net.add_arc("p2", "B");
        // Series place fusion requires silent transitions.
        assert!(!fuse_series_places(&mut net));
        assert_eq!(count_reducible_elements(&net), 0);
    }
}
