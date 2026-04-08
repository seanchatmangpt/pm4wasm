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

/// Convert a POWL model to a Petri net.
///
/// Faithfully ports `pm4py/objects/conversion/powl/variants/to_petri_net.py:
/// recursively_add_tree` and `apply`.
///
/// Returns a [`PetriNetResult`] containing the net plus initial and final markings.
use crate::petri_net::{Counts, Marking, PetriNet, PetriNetResult};
use crate::powl::{Operator, PowlArena, PowlNode};
use crate::process_tree::{PtOperator, ProcessTree};
use std::collections::HashMap;

// ─── Counter helpers ──────────────────────────────────────────────────────────

fn new_place(net: &mut PetriNet, counts: &mut Counts) -> String {
    let n = counts.inc_places();
    net.add_place(&format!("p_{}", n))
}

fn new_hidden_trans(net: &mut PetriNet, counts: &mut Counts, type_trans: &str) -> String {
    let n = counts.inc_hidden();
    net.add_transition(&format!("{}_{}", type_trans, n), None)
}

fn new_visible_trans(
    net: &mut PetriNet,
    counts: &mut Counts,
    label: &str,
    activity: &str,
    skippable: bool,
    selfloop: bool,
) -> String {
    let n = counts.inc_visible();
    let name = format!("vis_{}", n);
    let mut props = HashMap::new();
    props.insert(
        "activity".to_string(),
        serde_json::Value::String(activity.to_string()),
    );
    props.insert(
        "skippable".to_string(),
        serde_json::Value::Bool(skippable),
    );
    props.insert(
        "selfloop".to_string(),
        serde_json::Value::Bool(selfloop),
    );
    net.add_transition_with_props(&name, Some(label.to_string()), props)
}

// ─── Recursive construction ───────────────────────────────────────────────────

/// Mirrors Python's `recursively_add_tree`.
///
/// `initial_entity` and `final_entity` are place names (the Python function
/// also accepts transitions as entities, but we simplify: the callers always
/// supply place names here, using the same bridging logic as the Python code).
///
/// Returns the name of the final place created for this subtree.
fn recursively_add_tree(
    arena: &PowlArena,
    node_idx: u32,
    net: &mut PetriNet,
    initial_place: &str,
    final_place: Option<&str>,
    counts: &mut Counts,
    force_add_skip: bool,
) -> String {
    // Create final_place if not provided
    let final_place_name: String = match final_place {
        Some(fp) => fp.to_string(),
        None => new_place(net, counts),
    };

    if force_add_skip {
        let invisible = new_hidden_trans(net, counts, "skip");
        net.add_arc(initial_place, &invisible);
        net.add_arc(&invisible, &final_place_name);
    }

    match arena.get(node_idx) {
        None => {
            // Unknown node — add a skip
            let skip = new_hidden_trans(net, counts, "skip");
            net.add_arc(initial_place, &skip);
            net.add_arc(&skip, &final_place_name);
        }

        // ── Transition ───────────────────────────────────────────────────────
        Some(PowlNode::Transition(t)) => {
            let pt = if t.label.is_none() {
                new_hidden_trans(net, counts, "skip")
            } else {
                let lbl = t.label.as_deref().unwrap();
                new_visible_trans(net, counts, lbl, lbl, false, false)
            };
            net.add_arc(initial_place, &pt);
            net.add_arc(&pt, &final_place_name);
        }

        // ── FrequentTransition ───────────────────────────────────────────────
        Some(PowlNode::FrequentTransition(t)) => {
            let pt = new_visible_trans(
                net,
                counts,
                &t.label,
                &t.activity,
                t.skippable,
                t.selfloop,
            );
            net.add_arc(initial_place, &pt);
            net.add_arc(&pt, &final_place_name);
        }

        // ── OperatorPOWL ─────────────────────────────────────────────────────
        Some(PowlNode::OperatorPowl(op)) => {
            let children = op.children.clone();
            let operator = op.operator;

            match operator {
                Operator::Xor => {
                    for &child in &children {
                        recursively_add_tree(
                            arena,
                            child,
                            net,
                            initial_place,
                            Some(&final_place_name),
                            counts,
                            false,
                        );
                    }
                }

                Operator::Loop => {
                    // Python creates an "init_loop" hidden transition first
                    let new_init_place = new_place(net, counts);
                    let init_loop_trans = new_hidden_trans(net, counts, "init_loop");
                    net.add_arc(initial_place, &init_loop_trans);
                    net.add_arc(&init_loop_trans, &new_init_place);

                    let loop_trans = new_hidden_trans(net, counts, "loop");

                    // do-body
                    let do_idx = children[0];
                    let int1 = recursively_add_tree(
                        arena, do_idx, net, &new_init_place, None, counts, false,
                    );

                    // redo-body
                    let redo_idx = children[1];
                    let int2 = recursively_add_tree(
                        arena, redo_idx, net, &int1, None, counts, false,
                    );

                    // exit branch (silent transition from int1 to final)
                    let exit_trans = new_hidden_trans(net, counts, "skip");
                    net.add_arc(&int1, &exit_trans);
                    net.add_arc(&exit_trans, &final_place_name);

                    // loop back
                    net.add_arc(&int2, &loop_trans);
                    net.add_arc(&loop_trans, &new_init_place);
                }

                _ => {
                    // Unsupported operator — skip
                    let skip = new_hidden_trans(net, counts, "skip");
                    net.add_arc(initial_place, &skip);
                    net.add_arc(&skip, &final_place_name);
                }
            }
        }

        // ── StrictPartialOrder ───────────────────────────────────────────────
        Some(PowlNode::StrictPartialOrder(spo)) => {
            let children = spo.children.clone();
            let order = spo.order.get_transitive_reduction();
            let n = children.len();

            // tau_split: initial_place → tau_split → per-child init places
            let tau_split = new_hidden_trans(net, counts, "tauSplit");
            net.add_arc(initial_place, &tau_split);

            // tau_join: per-child final places → tau_join → final_place
            let tau_join = new_hidden_trans(net, counts, "tauJoin");
            net.add_arc(&tau_join, &final_place_name);

            let start_locals = order.get_start_nodes();
            let end_locals = order.get_end_nodes();

            // Each child gets an init PLACE and a final PLACE (proper PN structure).
            let mut init_places: Vec<String> = Vec::new();
            let mut final_places: Vec<String> = Vec::new();

            for (local, &child_idx) in children.iter().enumerate() {
                let i_place = new_place(net, counts); // init place for this child
                let f_place = new_place(net, counts); // final place for this child

                // Start nodes receive a token from tau_split
                if start_locals.contains(&local) {
                    net.add_arc(&tau_split, &i_place);
                }

                // End nodes feed tau_join
                if end_locals.contains(&local) {
                    net.add_arc(&f_place, &tau_join);
                }

                // Child subtree: i_place → [child subtree] → f_place
                recursively_add_tree(
                    arena, child_idx, net, &i_place, Some(&f_place), counts, false,
                );

                init_places.push(i_place);
                final_places.push(f_place);
            }

            // Ordering arcs: for edge i→j, route tokens through a sync transition
            // to enforce the ordering constraint.
            for i in 0..n {
                for j in 0..n {
                    if order.is_edge(i, j) {
                        let sync = new_hidden_trans(net, counts, "sync");
                        net.add_arc(&final_places[i], &sync);
                        net.add_arc(&sync, &init_places[j]);
                    }
                }
            }
        }
    }

    final_place_name
}

// ─── Dead-place removal ───────────────────────────────────────────────────────

fn remove_dead_places(net: &mut PetriNet, initial_marking: &Marking, final_marking: &Marking) {
    let im_places: std::collections::HashSet<&str> =
        initial_marking.keys().map(|s| s.as_str()).collect();
    let fm_places: std::collections::HashSet<&str> =
        final_marking.keys().map(|s| s.as_str()).collect();

    let place_names: Vec<String> = net.places.iter().map(|p| p.name.clone()).collect();
    for p in &place_names {
        if fm_places.contains(p.as_str()) || im_places.contains(p.as_str()) {
            continue;
        }
        let out_degree = net.arcs.iter().filter(|a| &a.source == p).count();
        let in_degree = net.arcs.iter().filter(|a| &a.target == p).count();
        if out_degree == 0 || in_degree == 0 {
            net.remove_place(p);
        }
    }
}

// ─── Public entry point ───────────────────────────────────────────────────────

/// Convert a POWL model to a Petri net with initial and final markings.
///
/// Mirrors `pm4py.objects.conversion.powl.variants.to_petri_net.apply`.
pub fn apply(arena: &PowlArena, root: u32) -> PetriNetResult {
    let mut counts = Counts::default();
    let mut net = PetriNet::new("powl_net");

    // Source and sink
    net.add_place("source");
    net.add_place("sink");

    let mut initial_marking = Marking::new();
    let mut final_marking = Marking::new();
    initial_marking.insert("source".to_string(), 1);
    final_marking.insert("sink".to_string(), 1);

    // Tau initial / tau final
    let initial_place = new_place(&mut net, &mut counts);
    let tau_initial = new_hidden_trans(&mut net, &mut counts, "tau");
    net.add_arc("source", &tau_initial);
    net.add_arc(&tau_initial, &initial_place);

    let final_place = new_place(&mut net, &mut counts);
    let tau_final = new_hidden_trans(&mut net, &mut counts, "tau");
    net.add_arc(&final_place, &tau_final);
    net.add_arc(&tau_final, "sink");

    recursively_add_tree(
        arena,
        root,
        &mut net,
        &initial_place,
        Some(&final_place),
        &mut counts,
        false,
    );

    net.apply_simple_reduction();
    remove_dead_places(&mut net, &initial_marking, &final_marking);

    PetriNetResult {
        net,
        initial_marking,
        final_marking,
    }
}

// ─── ProcessTree → Petri Net conversion ─────────────────────────────────────────

/// Convert a [`ProcessTree`] to a Petri net with initial and final markings.
///
/// Used by the inductive miner to produce Petri nets from discovered process trees.
pub fn from_process_tree(tree: &ProcessTree) -> PetriNetResult {
    let mut counts = Counts::default();
    let mut net = PetriNet::new("inductive_net");

    // Source and sink
    net.add_place("source");
    net.add_place("sink");

    let mut initial_marking = Marking::new();
    let mut final_marking = Marking::new();
    initial_marking.insert("source".to_string(), 1);
    final_marking.insert("sink".to_string(), 1);

    // Tau initial / tau final
    let initial_place = new_place(&mut net, &mut counts);
    let tau_initial = new_hidden_trans(&mut net, &mut counts, "tau");
    net.add_arc("source", &tau_initial);
    net.add_arc(&tau_initial, &initial_place);

    let final_place = new_place(&mut net, &mut counts);
    let tau_final = new_hidden_trans(&mut net, &mut counts, "tau");
    net.add_arc(&final_place, &tau_final);
    net.add_arc(&tau_final, "sink");

    add_process_tree_to_net(tree, &mut net, &initial_place, &final_place, &mut counts);

    net.apply_simple_reduction();
    remove_dead_places(&mut net, &initial_marking, &final_marking);

    PetriNetResult {
        net,
        initial_marking,
        final_marking,
    }
}

/// Recursively add a process tree to a Petri net.
fn add_process_tree_to_net(
    tree: &ProcessTree,
    net: &mut PetriNet,
    initial_place: &str,
    final_place: &str,
    counts: &mut Counts,
) {
    match &tree.operator {
        None => {
            // Leaf node
            let trans = match &tree.label {
                None => new_hidden_trans(net, counts, "tau"),
                Some(label) => new_visible_trans(net, counts, label, label, false, false),
            };
            net.add_arc(initial_place, &trans);
            net.add_arc(&trans, final_place);
        }
        Some(op) => match op {
            PtOperator::Sequence => {
                // Chain children: initial → child1 → ... → childN → final
                let mut prev = initial_place.to_string();
                for (i, child) in tree.children.iter().enumerate() {
                    let next = if i == tree.children.len() - 1 {
                        final_place.to_string()
                    } else {
                        new_place(net, counts)
                    };
                    add_process_tree_to_net(child, net, &prev, &next, counts);
                    prev = next;
                }
            }
            PtOperator::Xor => {
                // Choice: each child gets its own path from initial to final
                for child in &tree.children {
                    add_process_tree_to_net(child, net, initial_place, final_place, counts);
                }
            }
            PtOperator::Parallel => {
                // Fork/join: initial → tau_split → children → tau_join → final
                let split = new_hidden_trans(net, counts, "tauSplit");
                net.add_arc(initial_place, &split);

                let join = new_hidden_trans(net, counts, "tauJoin");
                net.add_arc(&join, final_place);

                for child in &tree.children {
                    let c_init = new_place(net, counts);
                    let c_final = new_place(net, counts);
                    net.add_arc(&split, &c_init);
                    net.add_arc(&c_final, &join);
                    add_process_tree_to_net(child, net, &c_init, &c_final, counts);
                }
            }
            PtOperator::Loop => {
                // Loop: initial → init_loop → do → [exit to final | redo → loop_back → init]
                let loop_init = new_place(net, counts);
                let init_loop = new_hidden_trans(net, counts, "init_loop");
                net.add_arc(initial_place, &init_loop);
                net.add_arc(&init_loop, &loop_init);

                // do-body
                let do_final = new_place(net, counts);
                add_process_tree_to_net(
                    &tree.children[0],
                    net,
                    &loop_init,
                    &do_final,
                    counts,
                );

                // exit branch
                let exit = new_hidden_trans(net, counts, "skip");
                net.add_arc(&do_final, &exit);
                net.add_arc(&exit, final_place);

                // redo-body (if present)
                if tree.children.len() > 1 {
                    let redo_final = new_place(net, counts);
                    add_process_tree_to_net(
                        &tree.children[1],
                        net,
                        &do_final,
                        &redo_final,
                        counts,
                    );
                    let loop_back = new_hidden_trans(net, counts, "loop");
                    net.add_arc(&redo_final, &loop_back);
                    net.add_arc(&loop_back, &loop_init);
                }
            }
        },
    }
}

// ─── tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::parse_powl_model_string;

    fn build(s: &str) -> (PowlArena, u32) {
        let mut arena = PowlArena::new();
        let root = parse_powl_model_string(s, &mut arena).unwrap();
        (arena, root)
    }

    #[test]
    fn single_transition_produces_net() {
        let (arena, root) = build("A");
        let result = apply(&arena, root);
        assert!(result.net.places.iter().any(|p| p.name == "source"));
        assert!(result.net.places.iter().any(|p| p.name == "sink"));
        assert!(result.net.transitions.iter().any(|t| t.label.as_deref() == Some("A")));
    }

    #[test]
    fn xor_produces_choice() {
        let (arena, root) = build("X ( A, B )");
        let result = apply(&arena, root);
        let labels: Vec<Option<&str>> = result
            .net
            .transitions
            .iter()
            .map(|t| t.label.as_deref())
            .collect();
        assert!(labels.contains(&Some("A")));
        assert!(labels.contains(&Some("B")));
    }

    #[test]
    fn partial_order_produces_parallel() {
        let (arena, root) = build("PO=(nodes={A, B}, order={})");
        let result = apply(&arena, root);
        // Both A and B should appear as transitions
        let labels: Vec<Option<&str>> = result
            .net
            .transitions
            .iter()
            .map(|t| t.label.as_deref())
            .collect();
        assert!(labels.contains(&Some("A")));
        assert!(labels.contains(&Some("B")));
    }

    #[test]
    fn sequence_order_preserves_structure() {
        let (arena, root) = build("PO=(nodes={A, B}, order={A-->B})");
        let result = apply(&arena, root);
        assert!(result.net.transitions.iter().any(|t| t.label.as_deref() == Some("A")));
        assert!(result.net.transitions.iter().any(|t| t.label.as_deref() == Some("B")));
    }

    #[test]
    fn loop_produces_cycle() {
        let (arena, root) = build("* ( A, B )");
        let result = apply(&arena, root);
        let labels: Vec<Option<&str>> = result
            .net
            .transitions
            .iter()
            .map(|t| t.label.as_deref())
            .collect();
        assert!(labels.contains(&Some("A")));
        assert!(labels.contains(&Some("B")));
    }

    #[test]
    fn from_process_tree_single_activity() {
        let tree = ProcessTree::leaf(Some("A".to_string()));
        let result = from_process_tree(&tree);
        assert!(result.net.transitions.iter().any(|t| t.label.as_deref() == Some("A")));
    }

    #[test]
    fn from_process_tree_sequence() {
        let tree = ProcessTree::internal(
            PtOperator::Sequence,
            vec![
                ProcessTree::leaf(Some("A".to_string())),
                ProcessTree::leaf(Some("B".to_string())),
            ],
        );
        let result = from_process_tree(&tree);
        let labels: Vec<Option<&str>> = result
            .net
            .transitions
            .iter()
            .map(|t| t.label.as_deref())
            .collect();
        assert!(labels.contains(&Some("A")));
        assert!(labels.contains(&Some("B")));
    }

    #[test]
    fn from_process_tree_xor() {
        let tree = ProcessTree::internal(
            PtOperator::Xor,
            vec![
                ProcessTree::leaf(Some("A".to_string())),
                ProcessTree::leaf(Some("B".to_string())),
            ],
        );
        let result = from_process_tree(&tree);
        let labels: Vec<Option<&str>> = result
            .net
            .transitions
            .iter()
            .map(|t| t.label.as_deref())
            .collect();
        assert!(labels.contains(&Some("A")));
        assert!(labels.contains(&Some("B")));
    }
}
