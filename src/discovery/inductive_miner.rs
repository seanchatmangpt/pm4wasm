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

/// Inductive Miner (IMd variant — DFG-based).
///
/// Discovers a process tree from an event log using the inductive miner
/// algorithm (Leemans et al., 2013). This is the DFG-based variant (IMd)
/// which works directly on the directly-follows graph rather than UVCL.
///
/// Mirrors `pm4py.discover_process_tree_inductive()`.

use crate::discovery::dfg::discover_dfg;
use crate::event_log::EventLog;
use crate::process_tree::{PtOperator, ProcessTree};
use std::collections::{HashMap, HashSet, VecDeque};

// ─── DFG helper ────────────────────────────────────────────────────────────────

/// A DFG with additional reachability information for the inductive miner.
#[derive(Clone, Debug)]
struct InductiveDFG {
    /// Adjacency: (source, target) → count.
    graph: HashMap<(String, String), usize>,
    /// Start activity frequencies.
    start_activities: HashMap<String, usize>,
    /// End activity frequencies.
    end_activities: HashMap<String, usize>,
    /// Whether the original log contained empty traces.
    skip: bool,
    /// All unique activities.
    alphabet: Vec<String>,
}

impl InductiveDFG {
    fn from_log(log: &EventLog) -> Self {
        let dfg = discover_dfg(log);
        let graph: HashMap<(String, String), usize> = dfg
            .edges
            .into_iter()
            .map(|e| ((e.source, e.target), e.count))
            .collect();
        let start_activities: HashMap<String, usize> = dfg
            .start_activities
            .into_iter()
            .collect();
        let end_activities: HashMap<String, usize> = dfg.end_activities.into_iter().collect();
        let alphabet: Vec<String> = dfg
            .activities
            .into_iter()
            .map(|(a, _)| a)
            .collect();
        let skip = log.traces.iter().any(|t| t.events.is_empty());
        InductiveDFG {
            graph,
            start_activities,
            end_activities,
            skip,
            alphabet,
        }
    }

    /// All vertices (activities) in the DFG.
    fn vertices(&self) -> HashSet<String> {
        let mut set = HashSet::new();
        for (s, t) in self.graph.keys() {
            set.insert(s.clone());
            set.insert(t.clone());
        }
        for a in self.start_activities.keys() {
            set.insert(a.clone());
        }
        for a in self.end_activities.keys() {
            set.insert(a.clone());
        }
        set
    }

    /// Get all successors of a node via BFS.
    fn successors(&self, node: &str) -> HashSet<String> {
        let mut visited = HashSet::new();
        let mut queue = VecDeque::new();
        queue.push_back(node.to_string());
        while let Some(current) = queue.pop_front() {
            for ((src, tgt), _) in &self.graph {
                if src == &current && !visited.contains(tgt) {
                    visited.insert(tgt.clone());
                    queue.push_back(tgt.clone());
                }
            }
        }
        visited
    }

    /// Get all predecessors of a node via BFS.
    fn predecessors(&self, node: &str) -> HashSet<String> {
        let mut visited = HashSet::new();
        let mut queue = VecDeque::new();
        queue.push_back(node.to_string());
        while let Some(current) = queue.pop_front() {
            for ((src, tgt), _) in &self.graph {
                if tgt == &current && !visited.contains(src) {
                    visited.insert(src.clone());
                    queue.push_back(src.clone());
                }
            }
        }
        visited
    }

    /// Check if there is a direct edge from→to.
    fn has_edge(&self, from: &str, to: &str) -> bool {
        self.graph.contains_key(&(from.to_string(), to.to_string()))
    }

    /// Check if there is an edge in either direction between a and b.
    #[allow(dead_code)]
    fn has_any_edge(&self, a: &str, b: &str) -> bool {
        self.has_edge(a, b) || self.has_edge(b, a)
    }

    /// Get outgoing edges for a node.
    #[allow(dead_code)]
    fn outgoing_edges(&self, node: &str) -> Vec<(String, usize)> {
        self.graph
            .iter()
            .filter(|((src, _), _)| src == node)
            .map(|((_, tgt), w)| (tgt.clone(), *w))
            .collect()
    }

    /// Get incoming edges for a node.
    #[allow(dead_code)]
    fn incoming_edges(&self, node: &str) -> Vec<(String, usize)> {
        self.graph
            .iter()
            .filter(|((_, tgt), _)| tgt == node)
            .map(|((src, _), w)| (src.clone(), *w))
            .collect()
    }

    /// Find connected components of the undirected graph.
    fn connected_components(&self) -> Vec<HashSet<String>> {
        let vertices = self.vertices();
        let mut visited = HashSet::new();
        let mut components = Vec::new();

        for v in &vertices {
            if visited.contains(v) {
                continue;
            }
            let mut component = HashSet::new();
            let mut queue = VecDeque::new();
            queue.push_back(v.clone());
            while let Some(current) = queue.pop_front() {
                if component.insert(current.clone()) {
                    for ((src, tgt), _) in &self.graph {
                        if src == &current && !component.contains(tgt) {
                            queue.push_back(tgt.clone());
                        }
                        if tgt == &current && !component.contains(src) {
                            queue.push_back(src.clone());
                        }
                    }
                }
            }
            visited.extend(component.iter().cloned());
            components.push(component);
        }

        // Include isolated activities (no edges but in alphabet)
        for a in &self.alphabet {
            if !visited.contains(a) {
                let mut singleton = HashSet::new();
                singleton.insert(a.clone());
                components.push(singleton);
                visited.insert(a.clone());
            }
        }

        components
    }

    /// Build a sub-DFG containing only the given activities.
    fn project(&self, activities: &HashSet<String>) -> InductiveDFG {
        let mut graph: HashMap<(String, String), usize> = HashMap::new();
        for ((src, tgt), w) in &self.graph {
            if activities.contains(src) && activities.contains(tgt) {
                graph.insert((src.clone(), tgt.clone()), *w);
            }
        }
        let start_activities: HashMap<String, usize> = self
            .start_activities
            .iter()
            .filter(|(a, _)| activities.contains(*a))
            .map(|(a, w)| (a.clone(), *w))
            .collect();
        let end_activities: HashMap<String, usize> = self
            .end_activities
            .iter()
            .filter(|(a, _)| activities.contains(*a))
            .map(|(a, w)| (a.clone(), *w))
            .collect();
        let alphabet: Vec<String> = activities.iter().cloned().collect();
        InductiveDFG {
            graph,
            start_activities,
            end_activities,
            skip: false,
            alphabet,
        }
    }
}

// ─── Cut detection ─────────────────────────────────────────────────────────────

/// Result of a cut detection: operator + groups of activities.
struct CutResult {
    operator: PtOperator,
    groups: Vec<HashSet<String>>,
}

/// Try XOR cut: find connected components of the undirected DFG.
fn detect_xor_cut(dfg: &InductiveDFG) -> Option<CutResult> {
    let components = dfg.connected_components();
    if components.len() > 1 {
        Some(CutResult {
            operator: PtOperator::Xor,
            groups: components,
        })
    } else {
        None
    }
}

/// Try sequence cut: find groups of activities that execute in order.
fn detect_sequence_cut(dfg: &InductiveDFG) -> Option<CutResult> {
    if dfg.alphabet.len() <= 1 {
        return None;
    }

    // Build one group per activity initially
    let mut groups: Vec<HashSet<String>> = dfg
        .alphabet
        .iter()
        .map(|a| {
            let mut s = HashSet::new();
            s.insert(a.clone());
            s
        })
        .collect();

    // Precompute transitive successors for each activity
    let mut successors_map: HashMap<String, HashSet<String>> = HashMap::new();
    for a in &dfg.alphabet {
        successors_map.insert(a.clone(), dfg.successors(a));
    }
    let mut predecessors_map: HashMap<String, HashSet<String>> = HashMap::new();
    for a in &dfg.alphabet {
        predecessors_map.insert(a.clone(), dfg.predecessors(a));
    }

    // Merge loop: merge groups that can be reordered (concurrent or unrelated)
    let mut changed = true;
    while changed {
        changed = false;
        let mut new_groups = Vec::new();
        let mut merged: Vec<bool> = vec![false; groups.len()];

        for i in 0..groups.len() {
            if merged[i] {
                continue;
            }
            let mut current = groups[i].clone();
            for j in (i + 1)..groups.len() {
                if merged[j] {
                    continue;
                }
                // Check if groups i and j should be merged:
                // For all (a in i, b in j): either (a reaches b AND b reaches a) OR (neither)
                let mut should_merge = true;
                'outer: for a in &groups[i] {
                    for b in &groups[j] {
                        let a_reaches_b = successors_map
                            .get(a)
                            .map(|s| s.contains(b))
                            .unwrap_or(false);
                        let b_reaches_a = successors_map
                            .get(b)
                            .map(|s| s.contains(a))
                            .unwrap_or(false);
                        // Only merge if they are concurrent (both reach each other)
                        // or unrelated (neither reaches the other)
                        if a_reaches_b != b_reaches_a {
                            should_merge = false;
                            break 'outer;
                        }
                    }
                }
                if should_merge {
                    current.extend(groups[j].clone());
                    merged[j] = true;
                    changed = true;
                }
            }
            new_groups.push(current);
        }
        groups = new_groups;
    }

    if groups.len() <= 1 {
        return None;
    }

    // Sort groups by topological order:
    // key = |predecessors| + (|alphabet| - |successors|) for representative
    let alpha_len = dfg.alphabet.len();
    groups.sort_by(|g1, g2| {
        let rep1 = g1.iter().next().unwrap();
        let rep2 = g2.iter().next().unwrap();
        let key1 = predecessors_map.get(rep1).map(|s| s.len()).unwrap_or(0) as i32
            + (alpha_len - successors_map.get(rep1).map(|s| s.len()).unwrap_or(0)) as i32;
        let key2 = predecessors_map.get(rep2).map(|s| s.len()).unwrap_or(0) as i32
            + (alpha_len - successors_map.get(rep2).map(|s| s.len()).unwrap_or(0)) as i32;
        key1.cmp(&key2)
    });

    // Validate: for each pair of consecutive groups, there should be edges from earlier to later
    let valid = groups
        .windows(2)
        .all(|w| {
            let earlier = &w[0];
            let later = &w[1];
            // At least some activity in earlier reaches some activity in later
            earlier.iter().any(|a| {
                successors_map
                    .get(a)
                    .map(|s| s.iter().any(|b| later.contains(b)))
                    .unwrap_or(false)
            })
        });

    if valid {
        Some(CutResult {
            operator: PtOperator::Sequence,
            groups,
        })
    } else {
        None
    }
}

/// Try parallel (concurrency) cut: find groups of concurrent activities.
fn detect_parallel_cut(dfg: &InductiveDFG) -> Option<CutResult> {
    if dfg.alphabet.len() <= 1 {
        return None;
    }

    // Build one group per activity
    let mut groups: Vec<HashSet<String>> = dfg
        .alphabet
        .iter()
        .map(|a| {
            let mut s = HashSet::new();
            s.insert(a.clone());
            s
        })
        .collect();

    // Merge groups that are NOT fully concurrent
    let mut changed = true;
    while changed {
        changed = false;
        let mut new_groups = Vec::new();
        let mut merged: Vec<bool> = vec![false; groups.len()];

        for i in 0..groups.len() {
            if merged[i] {
                continue;
            }
            let mut current = groups[i].clone();
            for j in (i + 1)..groups.len() {
                if merged[j] {
                    continue;
                }
                // Check if groups are concurrent:
                // For all (a in i, b in j): a→b AND b→a must both exist
                let mut concurrent = true;
                'outer: for a in &groups[i] {
                    for b in &groups[j] {
                        if !dfg.has_edge(a, b) || !dfg.has_edge(b, a) {
                            concurrent = false;
                            break 'outer;
                        }
                    }
                }
                if !concurrent {
                    // Merge non-concurrent groups
                    current.extend(groups[j].clone());
                    merged[j] = true;
                    changed = true;
                }
            }
            new_groups.push(current);
        }
        groups = new_groups;
    }

    if groups.len() <= 1 {
        return None;
    }

    // Filter: remove single-activity groups that don't contain both a start and end activity
    // (absorb them into a neighbor)
    let groups: Vec<HashSet<String>> = groups
        .into_iter()
        .filter(|g| {
            if g.len() > 1 {
                return true;
            }
            let act = g.iter().next().unwrap();
            dfg.start_activities.contains_key(act) && dfg.end_activities.contains_key(act)
        })
        .collect();

    if groups.len() <= 1 {
        return None;
    }

    Some(CutResult {
        operator: PtOperator::Parallel,
        groups,
    })
}

/// Try loop cut: detect do/redo structure.
fn detect_loop_cut(dfg: &InductiveDFG) -> Option<CutResult> {
    if dfg.graph.is_empty() {
        return None;
    }

    // Start/end activities form the "do" part
    let mut do_set: HashSet<String> = HashSet::new();
    for a in dfg.start_activities.keys() {
        do_set.insert(a.clone());
    }
    for a in dfg.end_activities.keys() {
        do_set.insert(a.clone());
    }

    // Find connected components of remaining activities (excluding start/end)
    let remaining: HashSet<&str> = dfg
        .alphabet
        .iter()
        .filter(|a| !do_set.contains(*a))
        .map(|a| a.as_str())
        .collect();

    if remaining.is_empty() {
        // Only start/end activities — no redo possible
        return None;
    }

    // Build sub-graph of remaining activities
    let mut sub_graph: HashMap<String, Vec<String>> = HashMap::new();
    for a in &remaining {
        sub_graph.insert(a.to_string(), Vec::new());
    }
    for ((src, tgt), _) in &dfg.graph {
        if remaining.contains(src.as_str()) && remaining.contains(tgt.as_str()) {
            sub_graph
                .entry(src.clone())
                .or_default()
                .push(tgt.clone());
            sub_graph
                .entry(tgt.clone())
                .or_default()
                .push(src.clone());
        }
    }

    // BFS to find connected components among remaining activities
    let mut visited: HashSet<String> = HashSet::new();
    let mut redo_groups: Vec<HashSet<String>> = Vec::new();

    for a in &remaining {
        if visited.contains(*a) {
            continue;
        }
        let mut component = HashSet::new();
        let mut queue = VecDeque::new();
        queue.push_back(a.to_string());
        while let Some(current) = queue.pop_front() {
            if component.insert(current.clone()) {
                if let Some(neighbors) = sub_graph.get(&current) {
                    for n in neighbors {
                        if !component.contains(n) {
                            queue.push_back(n.clone());
                        }
                    }
                }
            }
        }
        visited.extend(component.iter().cloned());
        redo_groups.push(component);
    }

    // Filter: merge redo groups that are reachable from start activities
    // (simplified version of the full filtering)
    let start_succs: HashSet<String> = dfg
        .start_activities
        .keys()
        .flat_map(|a| dfg.successors(a))
        .filter(|a| remaining.contains(a.as_str()))
        .collect();

    let mut filtered_redo = Vec::new();
    for group in redo_groups {
        if group.iter().any(|a| start_succs.contains(a)) {
            do_set.extend(group.iter().cloned());
        } else {
            filtered_redo.push(group);
        }
    }

    // Filter: merge redo groups reachable from end activities
    let end_preds: HashSet<String> = dfg
        .end_activities
        .keys()
        .flat_map(|a| dfg.predecessors(a))
        .filter(|a| remaining.contains(a.as_str()))
        .collect();

    let mut final_redo = Vec::new();
    for group in filtered_redo {
        if group.iter().any(|a| end_preds.contains(a)) {
            do_set.extend(group.iter().cloned());
        } else {
            final_redo.push(group);
        }
    }

    if final_redo.is_empty() {
        return None;
    }

    // Merge all redo groups into one
    let redo_set: HashSet<String> = final_redo.into_iter().flatten().collect();

    if redo_set.is_empty() || do_set.is_empty() {
        return None;
    }

    Some(CutResult {
        operator: PtOperator::Loop,
        groups: vec![do_set, redo_set],
    })
}

// ─── Main algorithm ────────────────────────────────────────────────────────────

/// Discover a process tree using the inductive miner (DFG-based variant).
///
/// Mirrors `pm4py.discover_process_tree_inductive()`.
pub fn inductive_miner(log: &EventLog) -> ProcessTree {
    let dfg = InductiveDFG::from_log(log);

    // Handle empty traces first
    if dfg.skip {
        let tau = ProcessTree::leaf(None);
        let non_empty = mine_dfg(&InductiveDFG {
            skip: false,
            ..dfg
        });
        return ProcessTree::internal(PtOperator::Xor, vec![tau, non_empty]);
    }

    mine_dfg(&dfg)
}

/// Recursive mining on a DFG structure.
fn mine_dfg(dfg: &InductiveDFG) -> ProcessTree {
    // Base case: empty log → tau
    if dfg.alphabet.is_empty() {
        return ProcessTree::leaf(None);
    }

    // Base case: single activity
    if dfg.alphabet.len() == 1 {
        return ProcessTree::leaf(Some(dfg.alphabet[0].clone()));
    }

    // Base case: DFG has no edges but multiple activities → XOR of leaves
    if dfg.graph.is_empty() {
        let children: Vec<ProcessTree> = dfg
            .alphabet
            .iter()
            .map(|a| ProcessTree::leaf(Some(a.clone())))
            .collect();
        return ProcessTree::internal(PtOperator::Xor, children);
    }

    // Try cuts in order: XOR, Sequence, Parallel, Loop
    if let Some(cut) = detect_xor_cut(dfg) {
        let children: Vec<ProcessTree> = cut
            .groups
            .iter()
            .map(|g| mine_dfg(&dfg.project(g)))
            .collect();
        return ProcessTree::internal(cut.operator, children);
    }

    if let Some(cut) = detect_sequence_cut(dfg) {
        let children: Vec<ProcessTree> = cut
            .groups
            .iter()
            .map(|g| mine_dfg(&dfg.project(g)))
            .collect();
        return ProcessTree::internal(cut.operator, children);
    }

    if let Some(cut) = detect_parallel_cut(dfg) {
        let children: Vec<ProcessTree> = cut
            .groups
            .iter()
            .map(|g| mine_dfg(&dfg.project(g)))
            .collect();
        return ProcessTree::internal(cut.operator, children);
    }

    if let Some(cut) = detect_loop_cut(dfg) {
        let children: Vec<ProcessTree> = cut
            .groups
            .iter()
            .map(|g| mine_dfg(&dfg.project(g)))
            .collect();
        return ProcessTree::internal(cut.operator, children);
    }

    // Fall-through: flower model
    flower_model(dfg)
}

/// Flower model fall-through: LOOP(tau, PARALLEL(a1, a2, ..., an))
fn flower_model(dfg: &InductiveDFG) -> ProcessTree {
    let redo_children: Vec<ProcessTree> = dfg
        .alphabet
        .iter()
        .map(|a| ProcessTree::leaf(Some(a.clone())))
        .collect();
    let redo = ProcessTree::internal(PtOperator::Parallel, redo_children);
    ProcessTree::internal(PtOperator::Loop, vec![ProcessTree::leaf(None), redo])
}

// ─── Post-processing ───────────────────────────────────────────────────────────

/// Simplify a process tree by flattening nested same-operator nodes and
/// removing single-child operator nodes.
pub fn simplify_tree(tree: ProcessTree) -> ProcessTree {
    match tree.operator {
        None => tree, // leaf
        Some(op) => {
            let children: Vec<ProcessTree> = tree
                .children
                .into_iter()
                .map(simplify_tree)
                .collect();

            // Flatten: if child has same operator, absorb its children
            let mut flat_children = Vec::new();
            for child in &children {
                if child.operator == Some(op) {
                    flat_children.extend(child.children.clone());
                } else {
                    flat_children.push(child.clone());
                }
            }

            // Remove single-child operators
            if flat_children.len() == 1 {
                return flat_children.into_iter().next().unwrap();
            }

            ProcessTree::internal(op, flat_children)
        }
    }
}

// ─── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::event_log::parse_csv;

    fn make_simple_log() -> EventLog {
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
    fn test_inductive_miner_produces_tree() {
        let log = make_simple_log();
        let tree = inductive_miner(&log);
        let repr = tree.to_repr();
        assert!(!repr.is_empty());
        // Should contain activities A, B, C
        assert!(repr.contains("A"));
        assert!(repr.contains("B"));
        assert!(repr.contains("C"));
    }

    #[test]
    fn test_single_activity_log() {
        let log = parse_csv(
            "case_id,activity,timestamp\n\
             1,A,2020-01-01T10:00:00\n\
             2,A,2020-01-01T11:00:00\n",
        )
        .unwrap();
        let tree = inductive_miner(&log);
        assert_eq!(tree.to_repr(), "A");
    }

    #[test]
    fn test_empty_log() {
        let log = parse_csv("case_id,activity,timestamp\n").unwrap();
        let tree = inductive_miner(&log);
        assert_eq!(tree.to_repr(), "tau");
    }

    #[test]
    fn test_sequence_log() {
        let log = parse_csv(
            "case_id,activity,timestamp\n\
             1,A,2020-01-01T10:00:00\n\
             1,B,2020-01-01T10:05:00\n\
             1,C,2020-01-01T10:10:00\n\
             2,A,2020-01-01T11:00:00\n\
             2,B,2020-01-01T11:05:00\n\
             2,C,2020-01-01T11:10:00\n",
        )
        .unwrap();
        let tree = inductive_miner(&log);
        let repr = tree.to_repr();
        assert!(repr.contains("->"), "Expected sequence operator, got: {}", repr);
    }

    #[test]
    fn test_xor_log() {
        // Two disconnected activities (never in same trace directly)
        let log = parse_csv(
            "case_id,activity,timestamp\n\
             1,A,2020-01-01T10:00:00\n\
             2,B,2020-01-01T11:00:00\n",
        )
        .unwrap();
        let tree = inductive_miner(&log);
        let repr = tree.to_repr();
        assert!(repr.contains("X"), "Expected XOR operator, got: {}", repr);
    }

    #[test]
    fn test_parallel_log() {
        // A and B appear in both orders → concurrent
        let log = parse_csv(
            "case_id,activity,timestamp\n\
             1,A,2020-01-01T10:00:00\n\
             1,B,2020-01-01T10:05:00\n\
             2,B,2020-01-01T11:00:00\n\
             2,A,2020-01-01T11:05:00\n",
        )
        .unwrap();
        let tree = inductive_miner(&log);
        let repr = tree.to_repr();
        assert!(repr.contains("+"), "Expected parallel operator, got: {}", repr);
    }

    #[test]
    fn test_simplify_tree() {
        let tree = ProcessTree::internal(
            PtOperator::Sequence,
            vec![
                ProcessTree::internal(
                    PtOperator::Sequence,
                    vec![ProcessTree::leaf(Some("A".to_string()))],
                ),
                ProcessTree::leaf(Some("B".to_string())),
            ],
        );
        let simplified = simplify_tree(tree);
        // Nested sequence should be flattened
        assert_eq!(simplified.children.len(), 2);
    }
}
