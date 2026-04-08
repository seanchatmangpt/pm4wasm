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

/// POWL node types and arena storage.
///
/// Mirrors the Python class hierarchy in `pm4py/objects/powl/obj.py`:
///   POWL (abstract)
///   ├── Transition          — labeled activity
///   │   ├── SilentTransition — tau
///   │   └── FrequentTransition — activity with [min,max] frequency
///   ├── StrictPartialOrder  — partial order over children
///   │   └── Sequence        — total order (convenience subtype)
///   └── OperatorPOWL       — XOR choice or LOOP
///
/// Instead of a recursive `Box<dyn POWL>` tree (problematic for wasm-bindgen),
/// nodes are stored in a flat `PowlArena` and referenced by u32 indices.
use crate::binary_relation::BinaryRelation;

// ─── Operator enum ───────────────────────────────────────────────────────────

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Operator {
    Xor,
    Loop,
    PartialOrder,
}

impl Operator {
    pub fn as_str(&self) -> &'static str {
        match self {
            Operator::Xor => "X",
            Operator::Loop => "*",
            Operator::PartialOrder => "PO",
        }
    }
}

// ─── Node variants ───────────────────────────────────────────────────────────

#[derive(Clone, Debug)]
pub struct TransitionNode {
    /// Activity label.  None for silent (tau) transitions.
    pub label: Option<String>,
    /// Unique integer identifier (mirrors Python's `_identifier`).
    pub id: u32,
}

#[derive(Clone, Debug)]
pub struct FrequentTransitionNode {
    /// Displayed activity label (may include `\n[min,max]` suffix).
    pub label: String,
    /// Underlying activity name (without frequency annotation).
    pub activity: String,
    pub skippable: bool,
    pub selfloop: bool,
    pub id: u32,
}

#[derive(Clone, Debug)]
pub struct StrictPartialOrderNode {
    /// Indices into `PowlArena::nodes` for each child.
    pub children: Vec<u32>,
    /// Adjacency matrix over the *local* child indices (0..children.len()).
    pub order: BinaryRelation,
}

#[derive(Clone, Debug)]
pub struct OperatorPowlNode {
    pub operator: Operator,
    /// Indices into `PowlArena::nodes` for each child.
    pub children: Vec<u32>,
}

/// Discriminated union of all node kinds stored in the arena.
#[derive(Clone, Debug)]
pub enum PowlNode {
    Transition(TransitionNode),
    FrequentTransition(FrequentTransitionNode),
    StrictPartialOrder(StrictPartialOrderNode),
    OperatorPowl(OperatorPowlNode),
}

impl PowlNode {
    pub fn is_silent(&self) -> bool {
        matches!(self, PowlNode::Transition(t) if t.label.is_none())
    }

    pub fn label(&self) -> Option<&str> {
        match self {
            PowlNode::Transition(t) => t.label.as_deref(),
            PowlNode::FrequentTransition(t) => Some(&t.label),
            _ => None,
        }
    }
}

// ─── Arena ───────────────────────────────────────────────────────────────────

/// Flat storage for the entire POWL model tree.
///
/// The root of the model is always at index 0 (the last node added by the
/// parser).  Individual nodes reference their children by arena index.
#[derive(Clone, Debug, Default)]
pub struct PowlArena {
    pub nodes: Vec<PowlNode>,
    next_transition_id: u32,
}

impl PowlArena {
    pub fn new() -> Self {
        PowlArena {
            nodes: Vec::new(),
            next_transition_id: 0,
        }
    }

    fn alloc_id(&mut self) -> u32 {
        let id = self.next_transition_id;
        self.next_transition_id += 1;
        id
    }

    /// Add a labeled transition; returns its arena index.
    pub fn add_transition(&mut self, label: Option<String>) -> u32 {
        let id = self.alloc_id();
        let idx = self.nodes.len() as u32;
        self.nodes.push(PowlNode::Transition(TransitionNode { label, id }));
        idx
    }

    /// Add a silent (tau) transition.
    pub fn add_silent_transition(&mut self) -> u32 {
        self.add_transition(None)
    }

    /// Add a FrequentTransition node.
    pub fn add_frequent_transition(
        &mut self,
        activity: String,
        min_freq: i64,
        max_freq: Option<i64>,
    ) -> u32 {
        let id = self.alloc_id();
        let idx = self.nodes.len() as u32;
        let skippable = min_freq == 0;
        let selfloop = max_freq.is_none();
        let max_str = max_freq.map_or_else(|| "-".to_string(), |v| v.to_string());
        let label = if skippable || selfloop {
            format!("{}\n[1,{}]", activity, max_str)
        } else {
            activity.clone()
        };
        self.nodes.push(PowlNode::FrequentTransition(FrequentTransitionNode {
            label,
            activity,
            skippable,
            selfloop,
            id,
        }));
        idx
    }

    /// Add a StrictPartialOrder node.  `children` are arena indices.
    pub fn add_strict_partial_order(&mut self, children: Vec<u32>) -> u32 {
        let n = children.len();
        let idx = self.nodes.len() as u32;
        self.nodes.push(PowlNode::StrictPartialOrder(StrictPartialOrderNode {
            children,
            order: BinaryRelation::new(n),
        }));
        idx
    }

    /// Add a Sequence (total order over children).
    pub fn add_sequence(&mut self, children: Vec<u32>) -> u32 {
        let n = children.len();
        let idx = self.nodes.len() as u32;
        let mut order = BinaryRelation::new(n);
        for i in 0..n {
            for j in (i + 1)..n {
                order.add_edge(i, j);
            }
        }
        self.nodes.push(PowlNode::StrictPartialOrder(StrictPartialOrderNode {
            children,
            order,
        }));
        idx
    }

    /// Add an OperatorPOWL node (XOR or LOOP).
    pub fn add_operator(&mut self, operator: Operator, children: Vec<u32>) -> u32 {
        let idx = self.nodes.len() as u32;
        self.nodes.push(PowlNode::OperatorPowl(OperatorPowlNode { operator, children }));
        idx
    }

    /// Add an edge inside a StrictPartialOrder.
    /// `spo_idx` — arena index of the SPO node.
    /// `child_src`, `child_tgt` — *local* indices (0-based within the SPO's children list).
    pub fn add_order_edge(&mut self, spo_idx: u32, child_src: usize, child_tgt: usize) {
        if let Some(PowlNode::StrictPartialOrder(spo)) = self.nodes.get_mut(spo_idx as usize) {
            spo.order.add_edge(child_src, child_tgt);
        } else {
            panic!("node {} is not a StrictPartialOrder", spo_idx);
        }
    }

    pub fn len(&self) -> usize {
        self.nodes.len()
    }

    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }

    pub fn get(&self, idx: u32) -> Option<&PowlNode> {
        self.nodes.get(idx as usize)
    }

    pub fn get_mut(&mut self, idx: u32) -> Option<&mut PowlNode> {
        self.nodes.get_mut(idx as usize)
    }

    // ─── Validation ──────────────────────────────────────────────────────────

    /// Recursively validate that all StrictPartialOrder nodes have
    /// irreflexive and transitive ordering relations.
    pub fn validate_partial_orders(&self, root: u32) -> Result<(), String> {
        match self.nodes.get(root as usize) {
            Some(PowlNode::StrictPartialOrder(spo)) => {
                if !spo.order.is_irreflexive() {
                    return Err(format!(
                        "node {}: partial order is not irreflexive",
                        root
                    ));
                }
                if !spo.order.is_transitive() {
                    return Err(format!(
                        "node {}: partial order is not transitive",
                        root
                    ));
                }
                for &child in &spo.children {
                    self.validate_partial_orders(child)?;
                }
            }
            Some(PowlNode::OperatorPowl(op)) => {
                for &child in &op.children {
                    self.validate_partial_orders(child)?;
                }
            }
            _ => {}
        }
        Ok(())
    }

    // ─── String representation ───────────────────────────────────────────────

    /// Produce the same string as the Python `__repr__` / `to_string()`.
    pub fn to_repr(&self, idx: u32) -> String {
        match self.nodes.get(idx as usize) {
            None => String::from("<invalid>"),
            Some(PowlNode::Transition(t)) => match &t.label {
                None => "tau".to_string(),
                Some(l) => l.clone(),
            },
            Some(PowlNode::FrequentTransition(t)) => t.label.clone(),
            Some(PowlNode::StrictPartialOrder(spo)) => {
                let nodes_str: Vec<String> =
                    spo.children.iter().map(|&c| self.to_repr(c)).collect();
                let mut edges_str: Vec<String> = Vec::new();
                let n = spo.children.len();
                for i in 0..n {
                    for j in 0..n {
                        if spo.order.is_edge(i, j) {
                            let src_label = self.node_label_or_id(spo.children[i]);
                            let tgt_label = self.node_label_or_id(spo.children[j]);
                            edges_str.push(format!("{}-->{}", src_label, tgt_label));
                        }
                    }
                }
                format!(
                    "PO=(nodes={{{}}}, order={{{}}})",
                    nodes_str.join(", "),
                    edges_str.join(", ")
                )
            }
            Some(PowlNode::OperatorPowl(op)) => {
                let children_str: Vec<String> =
                    op.children.iter().map(|&c| self.to_repr(c)).collect();
                format!("{} ( {} )", op.operator.as_str(), children_str.join(", "))
            }
        }
    }

    fn node_label_or_id(&self, idx: u32) -> String {
        match self.nodes.get(idx as usize) {
            Some(PowlNode::Transition(t)) => match &t.label {
                None => format!("id_{}", idx),
                Some(l) => l.clone(),
            },
            Some(PowlNode::FrequentTransition(t)) => t.label.clone(),
            _ => format!("id_{}", idx),
        }
    }

    // ─── Deep copy ───────────────────────────────────────────────────────────

    /// Deep-copy the subtree rooted at `idx` into a new arena.
    /// Returns the new arena and the index of the root in it.
    pub fn copy_subtree(&self, idx: u32) -> (PowlArena, u32) {
        let mut new_arena = PowlArena::new();
        let new_root = self.copy_node_into(&mut new_arena, idx);
        (new_arena, new_root)
    }

    fn copy_node_into(&self, dest: &mut PowlArena, idx: u32) -> u32 {
        match self.nodes.get(idx as usize) {
            None => panic!("invalid arena index {}", idx),
            Some(PowlNode::Transition(t)) => dest.add_transition(t.label.clone()),
            Some(PowlNode::FrequentTransition(t)) => {
                // Reconstruct min/max from flags
                let min_freq: i64 = if t.skippable { 0 } else { 1 };
                let max_freq: Option<i64> = if t.selfloop { None } else { Some(1) };
                dest.add_frequent_transition(t.activity.clone(), min_freq, max_freq)
            }
            Some(PowlNode::StrictPartialOrder(spo)) => {
                let new_children: Vec<u32> = spo
                    .children
                    .iter()
                    .map(|&c| self.copy_node_into(dest, c))
                    .collect();
                let spo_idx = dest.add_strict_partial_order(new_children);
                // Copy edges
                let n = spo.children.len();
                if let Some(PowlNode::StrictPartialOrder(new_spo)) =
                    dest.nodes.get_mut(spo_idx as usize)
                {
                    for i in 0..n {
                        for j in 0..n {
                            if spo.order.is_edge(i, j) {
                                new_spo.order.add_edge(i, j);
                            }
                        }
                    }
                }
                spo_idx
            }
            Some(PowlNode::OperatorPowl(op)) => {
                let operator = op.operator;
                let new_children: Vec<u32> = op
                    .children
                    .iter()
                    .map(|&c| self.copy_node_into(dest, c))
                    .collect();
                dest.add_operator(operator, new_children)
            }
        }
    }
}

// ─── tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_simple_sequence() {
        let mut arena = PowlArena::new();
        let a = arena.add_transition(Some("A".into()));
        let b = arena.add_transition(Some("B".into()));
        let seq = arena.add_sequence(vec![a, b]);
        assert_eq!(arena.to_repr(seq), "PO=(nodes={A, B}, order={A-->B})");
    }

    #[test]
    fn build_xor() {
        let mut arena = PowlArena::new();
        let a = arena.add_transition(Some("A".into()));
        let tau = arena.add_silent_transition();
        let xor = arena.add_operator(Operator::Xor, vec![a, tau]);
        assert_eq!(arena.to_repr(xor), "X ( A, tau )");
    }

    #[test]
    fn validate_valid_po() {
        let mut arena = PowlArena::new();
        let a = arena.add_transition(Some("A".into()));
        let b = arena.add_transition(Some("B".into()));
        let c = arena.add_transition(Some("C".into()));
        let po = arena.add_strict_partial_order(vec![a, b, c]);
        arena.add_order_edge(po, 0, 1); // A→B
        arena.add_order_edge(po, 1, 2); // B→C
        arena.add_order_edge(po, 0, 2); // A→C (transitivity)
        assert!(arena.validate_partial_orders(po).is_ok());
    }

    #[test]
    fn validate_missing_transitive_edge_fails() {
        let mut arena = PowlArena::new();
        let a = arena.add_transition(Some("A".into()));
        let b = arena.add_transition(Some("B".into()));
        let c = arena.add_transition(Some("C".into()));
        let po = arena.add_strict_partial_order(vec![a, b, c]);
        arena.add_order_edge(po, 0, 1); // A→B
        arena.add_order_edge(po, 1, 2); // B→C
        // Missing A→C  ⇒ not transitive
        assert!(arena.validate_partial_orders(po).is_err());
    }

    #[test]
    fn copy_subtree_is_independent() {
        let mut arena = PowlArena::new();
        let a = arena.add_transition(Some("A".into()));
        let b = arena.add_transition(Some("B".into()));
        let seq = arena.add_sequence(vec![a, b]);
        let (new_arena, new_root) = arena.copy_subtree(seq);
        assert_eq!(new_arena.to_repr(new_root), arena.to_repr(seq));
    }
}
