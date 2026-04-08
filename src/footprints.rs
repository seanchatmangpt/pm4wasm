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

/// POWL footprint analysis.
///
/// Ports `pm4py/algo/discovery/footprints/powl/variants/bottomup.py`.
///
/// A footprint is a summary of a process model's behavioural properties:
/// - Which activities can start/end a trace
/// - Directly-follows pairs (sequence) and concurrent pairs (parallel)
/// - Which activities always happen (non-skippable)
/// - Minimum trace length
use crate::powl::{Operator, PowlArena, PowlNode};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet, VecDeque};

// ─── Data types ───────────────────────────────────────────────────────────────

pub type ActivitySet = HashSet<String>;
pub type ActivityPairs = HashSet<(String, String)>;

/// Footprints of a POWL model (or sub-model).
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Footprints {
    /// Activities that can start a trace.
    pub start_activities: ActivitySet,
    /// Activities that can end a trace.
    pub end_activities: ActivitySet,
    /// All activities reachable in the model.
    pub activities: ActivitySet,
    /// True if the model can produce an empty trace (the whole model is optional).
    pub skippable: bool,
    /// Directly-follows pairs: `(a, b)` means `a` can be directly followed by `b`.
    pub sequence: ActivityPairs,
    /// Concurrent pairs: `(a, b)` means `a` and `b` can occur concurrently.
    pub parallel: ActivityPairs,
    /// Activities that always happen in every execution.
    pub activities_always_happening: ActivitySet,
    /// Minimum number of activities in any complete trace.
    pub min_trace_length: usize,
}

impl Footprints {
    fn empty_skip() -> Self {
        Footprints {
            skippable: true,
            ..Default::default()
        }
    }

    fn single(label: &str) -> Self {
        let act: ActivitySet = [label.to_string()].into();
        Footprints {
            start_activities: act.clone(),
            end_activities: act.clone(),
            activities: act.clone(),
            skippable: false,
            sequence: Default::default(),
            parallel: Default::default(),
            activities_always_happening: act,
            min_trace_length: 1,
        }
    }
}

// ─── Utility: fix_fp ─────────────────────────────────────────────────────────

/// Remove parallel pairs from sequence; convert bidirectional sequence pairs
/// to parallel.  Mirrors Python `fix_fp`.
fn fix_fp(mut sequence: ActivityPairs, mut parallel: ActivityPairs) -> (ActivityPairs, ActivityPairs) {
    sequence = sequence.difference(&parallel).cloned().collect();
    let bidirectional: ActivityPairs = sequence
        .iter()
        .filter(|(a, b)| sequence.contains(&(b.clone(), a.clone())))
        .cloned()
        .collect();
    for pair in &bidirectional {
        parallel.insert(pair.clone());
        sequence.remove(pair);
    }
    (sequence, parallel)
}

// ─── Merge footprints (AND / parallel semantics) ──────────────────────────────

fn merge_footprints(fps: &[Footprints]) -> Footprints {
    if fps.is_empty() {
        return Footprints::empty_skip();
    }
    let mut merged = fps[0].clone();
    for fp in &fps[1..] {
        merged.activities =
            merged.activities.union(&fp.activities).cloned().collect();
        merged.skippable = merged.skippable && fp.skippable;
        merged.sequence =
            merged.sequence.union(&fp.sequence).cloned().collect();
        merged.parallel =
            merged.parallel.union(&fp.parallel).cloned().collect();
        if !fp.skippable {
            merged.activities_always_happening = merged
                .activities_always_happening
                .union(&fp.activities_always_happening)
                .cloned()
                .collect();
        }
    }
    merged
}

// ─── Per-node-type footprint computation ─────────────────────────────────────

fn footprints_of_transition(label: Option<&str>) -> Footprints {
    match label {
        None => Footprints::empty_skip(),
        Some(l) => Footprints::single(l),
    }
}

fn footprints_of_xor(children: &[Footprints]) -> Footprints {
    let mut start: ActivitySet = Default::default();
    let mut end: ActivitySet = Default::default();
    let mut activities: ActivitySet = Default::default();
    let mut skippable = false;
    let mut sequence: ActivityPairs = Default::default();
    let mut parallel: ActivityPairs = Default::default();
    let mut aah: Option<ActivitySet> = None;

    for fp in children {
        start = start.union(&fp.start_activities).cloned().collect();
        end = end.union(&fp.end_activities).cloned().collect();
        activities = activities.union(&fp.activities).cloned().collect();
        skippable = skippable || fp.skippable;
        sequence = sequence.union(&fp.sequence).cloned().collect();
        parallel = parallel.union(&fp.parallel).cloned().collect();
        if !fp.skippable {
            aah = Some(match aah {
                None => fp.activities_always_happening.clone(),
                Some(prev) => prev
                    .intersection(&fp.activities_always_happening)
                    .cloned()
                    .collect(),
            });
        }
    }

    let (sequence, parallel) = fix_fp(sequence, parallel);
    let min_trace_length = children
        .iter()
        .map(|fp| fp.min_trace_length)
        .min()
        .unwrap_or(0);

    Footprints {
        start_activities: start,
        end_activities: end,
        activities,
        skippable,
        sequence,
        parallel,
        activities_always_happening: aah.unwrap_or_default(),
        min_trace_length,
    }
}

fn footprints_of_loop(do_fp: &Footprints, redo_fp: &Footprints) -> Footprints {
    let mut start = do_fp.start_activities.clone();
    let mut end = do_fp.end_activities.clone();
    let activities: ActivitySet = do_fp
        .activities
        .union(&redo_fp.activities)
        .cloned()
        .collect();
    let mut sequence: ActivityPairs = do_fp
        .sequence
        .union(&redo_fp.sequence)
        .cloned()
        .collect();
    let parallel: ActivityPairs = do_fp
        .parallel
        .union(&redo_fp.parallel)
        .cloned()
        .collect();
    let skippable = do_fp.skippable;
    let aah: ActivitySet = if !do_fp.skippable {
        do_fp.activities_always_happening.clone()
    } else {
        Default::default()
    };

    if do_fp.skippable {
        start = start.union(&redo_fp.start_activities).cloned().collect();
        end = end.union(&redo_fp.end_activities).cloned().collect();
    }

    // do.end → redo.start
    for a1 in &do_fp.end_activities {
        for a2 in &redo_fp.start_activities {
            sequence.insert((a1.clone(), a2.clone()));
        }
    }
    // redo.end → do.start
    for a1 in &redo_fp.end_activities {
        for a2 in &do_fp.start_activities {
            sequence.insert((a1.clone(), a2.clone()));
        }
    }
    if do_fp.skippable {
        for a1 in &redo_fp.end_activities {
            for a2 in &redo_fp.start_activities {
                sequence.insert((a1.clone(), a2.clone()));
            }
        }
    }
    if redo_fp.skippable {
        for a1 in &do_fp.end_activities {
            for a2 in &do_fp.start_activities {
                sequence.insert((a1.clone(), a2.clone()));
            }
        }
    }

    let (sequence, parallel) = fix_fp(sequence, parallel);

    Footprints {
        start_activities: start,
        end_activities: end,
        activities,
        skippable,
        sequence,
        parallel,
        activities_always_happening: aah,
        min_trace_length: do_fp.min_trace_length,
    }
}

fn footprints_of_partial_order(
    children_fps: &[Footprints],
    _order_n: usize,
    order_is_edge: &dyn Fn(usize, usize) -> bool,
) -> Footprints {
    let n = children_fps.len();
    if n == 0 {
        return Footprints::empty_skip();
    }

    // Build adjacency list (original, before reduction)
    let mut adj: Vec<Vec<usize>> = vec![Vec::new(); n];
    for i in 0..n {
        for j in 0..n {
            if order_is_edge(i, j) {
                adj[i].push(j);
            }
        }
    }

    // Transitive closure (reachability sets)
    let closure: Vec<HashSet<usize>> = {
        let mut cl: Vec<HashSet<usize>> = (0..n)
            .map(|i| {
                let mut s = HashSet::new();
                s.insert(i);
                s
            })
            .collect();
        for start in 0..n {
            let mut visited: HashSet<usize> = HashSet::new();
            let mut queue = VecDeque::new();
            queue.push_back(start);
            while let Some(cur) = queue.pop_front() {
                if visited.contains(&cur) {
                    continue;
                }
                visited.insert(cur);
                cl[start].insert(cur);
                for &nxt in &adj[cur] {
                    queue.push_back(nxt);
                }
            }
        }
        cl
    };

    // Transitive reduction adjacency
    let reduced_adj: Vec<Vec<usize>> = (0..n)
        .map(|i| {
            adj[i]
                .iter()
                .filter(|&&j| {
                    // Keep i→j if no intermediate k (k≠i,k≠j) with i→k and k→...→j
                    !adj[i].iter().any(|&k| k != j && closure[k].contains(&j))
                })
                .cloned()
                .collect()
        })
        .collect();

    // Merge child footprints (AND-semantics)
    let merged = merge_footprints(children_fps);
    let mut sequence = merged.sequence.clone();
    let mut parallel = merged.parallel.clone();

    // Start nodes: c is a start if no non-skippable predecessor reaches c
    let mut start_activities: ActivitySet = Default::default();
    for (i, fp_c) in children_fps.iter().enumerate() {
        let is_start = children_fps.iter().enumerate().all(|(pi, fp_p)| {
            pi == i || fp_p.skippable || !closure[pi].contains(&i)
        });
        if is_start {
            start_activities = start_activities
                .union(&fp_c.start_activities)
                .cloned()
                .collect();
        }
    }

    // End nodes: c is an end if no non-skippable successor of c exists
    let mut end_activities: ActivitySet = Default::default();
    for (i, fp_c) in children_fps.iter().enumerate() {
        let is_end = children_fps.iter().enumerate().all(|(qi, fp_q)| {
            qi == i || fp_q.skippable || !closure[i].contains(&qi)
        });
        if is_end {
            end_activities = end_activities
                .union(&fp_c.end_activities)
                .cloned()
                .collect();
        }
    }

    // Sequence edges from reduced partial order
    for i in 0..n {
        for &j in &reduced_adj[i] {
            for a1 in &children_fps[i].end_activities {
                for a2 in &children_fps[j].start_activities {
                    sequence.insert((a1.clone(), a2.clone()));
                }
            }
        }
    }

    // Skip edges: c→d if all paths from c to d pass only through skippable nodes
    for i in 0..n {
        for j in 0..n {
            if i == j || !closure[i].contains(&j) {
                continue;
            }
            let all_skippable_intermediates = children_fps.iter().enumerate().all(|(k, fp_k)| {
                k == i || k == j || fp_k.skippable
                    || !(closure[i].contains(&k) && closure[k].contains(&j))
            });
            if all_skippable_intermediates {
                for a1 in &children_fps[i].end_activities {
                    for a2 in &children_fps[j].start_activities {
                        sequence.insert((a1.clone(), a2.clone()));
                    }
                }
            }
        }
    }

    // Concurrency: no ordering in either direction
    for i in 0..n {
        for j in (i + 1)..n {
            if !closure[i].contains(&j) && !closure[j].contains(&i) {
                for a1 in &children_fps[i].activities {
                    for a2 in &children_fps[j].activities {
                        parallel.insert((a1.clone(), a2.clone()));
                        parallel.insert((a2.clone(), a1.clone()));
                    }
                }
            }
        }
    }

    let (sequence, parallel) = fix_fp(sequence, parallel);

    let min_trace_length: usize = children_fps
        .iter()
        .filter(|fp| !fp.skippable)
        .map(|fp| fp.min_trace_length)
        .sum();

    Footprints {
        start_activities,
        end_activities,
        activities: merged.activities,
        skippable: merged.skippable,
        sequence,
        parallel,
        activities_always_happening: merged.activities_always_happening,
        min_trace_length,
    }
}

// ─── Recursive entry point ────────────────────────────────────────────────────

/// Compute footprints of the subtree rooted at `node_idx` using memoization.
pub fn compute(
    arena: &PowlArena,
    node_idx: u32,
    cache: &mut HashMap<u32, Footprints>,
) -> Footprints {
    if let Some(fp) = cache.get(&node_idx) {
        return fp.clone();
    }

    let fp = match arena.get(node_idx) {
        None => Footprints::empty_skip(),

        Some(PowlNode::Transition(t)) => footprints_of_transition(t.label.as_deref()),
        Some(PowlNode::FrequentTransition(t)) => {
            let mut fp = footprints_of_transition(Some(&t.activity));
            // FrequentTransition may be skippable
            if t.skippable {
                fp.skippable = true;
                fp.activities_always_happening.clear();
            }
            fp
        }

        Some(PowlNode::OperatorPowl(op)) => {
            let children = op.children.clone();
            let operator = op.operator;
            let child_fps: Vec<Footprints> = children
                .iter()
                .map(|&c| compute(arena, c, cache))
                .collect();
            match operator {
                Operator::Xor => footprints_of_xor(&child_fps),
                Operator::Loop if child_fps.len() == 2 => {
                    footprints_of_loop(&child_fps[0], &child_fps[1])
                }
                _ => footprints_of_xor(&child_fps), // fallback
            }
        }

        Some(PowlNode::StrictPartialOrder(spo)) => {
            let children = spo.children.clone();
            let order = spo.order.clone();
            let n = children.len();
            let child_fps: Vec<Footprints> = children
                .iter()
                .map(|&c| compute(arena, c, cache))
                .collect();
            footprints_of_partial_order(&child_fps, n, &|i, j| order.is_edge(i, j))
        }
    };

    cache.insert(node_idx, fp.clone());
    fp
}

/// Compute footprints of a full POWL model.
pub fn apply(arena: &PowlArena, root: u32) -> Footprints {
    let mut cache = HashMap::new();
    compute(arena, root, &mut cache)
}

/// Discover footprints from an event log.
///
/// Computes footprints from the log's directly-follows graph.
/// This is useful for comparing log behavior against model footprints.
pub fn discover_from_log(log: &crate::event_log::EventLog) -> Footprints {
    let mut sequence: ActivityPairs = ActivityPairs::new();
    let mut parallel: ActivityPairs = ActivityPairs::new();
    let mut start_activities: ActivitySet = ActivitySet::new();
    let mut end_activities: ActivitySet = ActivitySet::new();
    let mut activities: ActivitySet = ActivitySet::new();
    let mut sequence_counts: HashMap<(String, String), usize> = HashMap::new();

    // Build statistics from log
    for trace in &log.traces {
        let events = &trace.events;

        if !events.is_empty() {
            start_activities.insert(events[0].name.clone());
            end_activities.insert(events[events.len() - 1].name.clone());
        }

        for event in events {
            activities.insert(event.name.clone());
        }

        for window in events.windows(2) {
            let from = window[0].name.clone();
            let to = window[1].name.clone();
            *sequence_counts.entry((from.clone(), to.clone())).or_insert(0) += 1;
            sequence.insert((from, to));
        }
    }

    // Detect parallel pairs (bidirectional sequence)
    for (a, b) in sequence.clone() {
        if sequence.contains(&(b.clone(), a.clone())) {
            parallel.insert((a, b));
        }
    }

    // Remove parallel pairs from sequence
    for pair in parallel.clone() {
        sequence.remove(&pair);
    }

    Footprints {
        start_activities,
        end_activities,
        activities,
        skippable: false,
        sequence,
        parallel,
        activities_always_happening: ActivitySet::new(), // Not computed from log
        min_trace_length: 1, // Simplified; could be computed from log
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
    fn single_activity() {
        let (arena, root) = build("A");
        let fp = apply(&arena, root);
        assert!(fp.start_activities.contains("A"));
        assert!(fp.end_activities.contains("A"));
        assert!(!fp.skippable);
        assert_eq!(fp.min_trace_length, 1);
    }

    #[test]
    fn tau_is_skippable() {
        let (arena, root) = build("tau");
        let fp = apply(&arena, root);
        assert!(fp.skippable);
        assert_eq!(fp.min_trace_length, 0);
    }

    #[test]
    fn xor_ab_can_start_with_either() {
        let (arena, root) = build("X ( A, B )");
        let fp = apply(&arena, root);
        assert!(fp.start_activities.contains("A"));
        assert!(fp.start_activities.contains("B"));
        assert!(!fp.skippable);
    }

    #[test]
    fn xor_a_tau_is_skippable() {
        let (arena, root) = build("X ( A, tau )");
        let fp = apply(&arena, root);
        assert!(fp.skippable);
    }

    #[test]
    fn sequence_po_start_end() {
        let (arena, root) = build("PO=(nodes={A, B}, order={A-->B})");
        let fp = apply(&arena, root);
        assert!(fp.start_activities.contains("A"), "start: {:?}", fp.start_activities);
        assert!(fp.end_activities.contains("B"), "end: {:?}", fp.end_activities);
        assert!(fp.sequence.contains(&("A".to_string(), "B".to_string())));
    }

    #[test]
    fn concurrent_po_produces_parallel() {
        let (arena, root) = build("PO=(nodes={A, B}, order={})");
        let fp = apply(&arena, root);
        assert!(fp.parallel.contains(&("A".to_string(), "B".to_string())));
        assert!(fp.parallel.contains(&("B".to_string(), "A".to_string())));
    }

    #[test]
    fn loop_has_do_as_start() {
        let (arena, root) = build("* ( A, B )");
        let fp = apply(&arena, root);
        assert!(fp.start_activities.contains("A"));
        assert!(!fp.skippable);
    }

    #[test]
    fn nested_po() {
        // A → (B ∥ C) → D
        // PO=(nodes={A, X, D}, order={A-->X, X-->D}) where X = PO=(nodes={B,C},order={})
        let (arena, root) =
            build("PO=(nodes={A, PO=(nodes={B, C}, order={}), D}, order={A-->PO=(nodes={B, C}, order={}), PO=(nodes={B, C}, order={})-->D})");
        let fp = apply(&arena, root);
        assert!(fp.start_activities.contains("A"));
        assert!(fp.end_activities.contains("D"));
        assert!(fp.activities.contains("B"));
    }
}
