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

/// Structural and behavioral diff between two POWL models.
///
/// Compares two models along three axes:
///
/// 1. **Activities** — added, removed, always-happening changes
/// 2. **Structure** — operator type changes at comparable positions
/// 3. **Ordering** — new/removed sequence and parallel relations
///
/// The result is a typed [`ModelDiff`] suitable for driving UI highlights,
/// change-log generation, or automated regression detection.
use serde::{Deserialize, Serialize};
use crate::footprints::{self, Footprints, ActivityPairs};
use crate::powl::{PowlArena, PowlNode};

// ─── Types ────────────────────────────────────────────────────────────────────

/// An (a, b) activity pair.
pub type Pair = (String, String);

/// A change in the always-happening set.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum AlwaysChange {
    /// Activity became mandatory (was optional before).
    BecameMandatory(String),
    /// Activity became optional (was mandatory before).
    BecameOptional(String),
}

/// A change in an ordering relation.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum OrderChange {
    /// A sequence relation a→b appeared in the new model.
    SequenceAdded(Pair),
    /// A sequence relation a→b was removed in the new model.
    SequenceRemoved(Pair),
    /// A parallel relation (a ∥ b) appeared.
    ParallelAdded(Pair),
    /// A parallel relation (a ∥ b) was removed.
    ParallelRemoved(Pair),
    /// Start-activity set changed.
    StartAdded(String),
    StartRemoved(String),
    /// End-activity set changed.
    EndAdded(String),
    EndRemoved(String),
}

/// A top-level structural change (operator type at the root or major node).
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct StructureChange {
    /// Human-readable path (e.g. `"root"`, `"root.child[0]"`).
    pub location: String,
    /// Node type in model A.
    pub from: String,
    /// Node type in model B.
    pub to: String,
}

/// Severity classification.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub enum Severity {
    /// No difference.
    None,
    /// Minor behavioural change (optional activities, ordering hints).
    Minor,
    /// Moderate change (activities added/removed, ordering flipped).
    Moderate,
    /// Breaking change (mandatory activities removed, root operator changed).
    Breaking,
}

/// Complete diff result between model A (old) and model B (new).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ModelDiff {
    /// Activities present in B but not A.
    pub added_activities: Vec<String>,
    /// Activities present in A but not B.
    pub removed_activities: Vec<String>,
    /// Changes to the always-happening set.
    pub always_changes: Vec<AlwaysChange>,
    /// Ordering relation changes.
    pub order_changes: Vec<OrderChange>,
    /// Structural (operator type) changes at key positions.
    pub structure_changes: Vec<StructureChange>,
    /// Minimum trace length delta: new_min - old_min.
    pub min_trace_length_delta: i64,
    /// Overall estimated severity of the changes.
    pub severity: Severity,
    /// True when the two models are behaviourally equivalent
    /// (same footprints — does not imply structural equivalence).
    pub behaviourally_equivalent: bool,
}

impl ModelDiff {
    /// True if there are no differences at all.
    pub fn is_empty(&self) -> bool {
        self.added_activities.is_empty()
            && self.removed_activities.is_empty()
            && self.always_changes.is_empty()
            && self.order_changes.is_empty()
            && self.structure_changes.is_empty()
            && self.min_trace_length_delta == 0
    }
}

// ─── Helpers ──────────────────────────────────────────────────────────────────

fn node_type_str(arena: &PowlArena, idx: u32) -> String {
    match arena.get(idx) {
        Some(PowlNode::Transition(t)) => {
            if t.label.is_some() { "Transition".into() } else { "tau".into() }
        }
        Some(PowlNode::FrequentTransition(_)) => "FrequentTransition".into(),
        Some(PowlNode::StrictPartialOrder(_)) => "StrictPartialOrder".into(),
        Some(PowlNode::OperatorPowl(op)) => op.operator.as_str().to_string(),
        None => "Invalid".into(),
    }
}

fn footprint_pairs(fp: &Footprints) -> ActivityPairs {
    fp.sequence.clone()
}

fn parallel_pairs(fp: &Footprints) -> ActivityPairs {
    fp.parallel.clone()
}

/// Recursively collect structural changes by walking both trees in tandem.
fn structural_diff(
    arena_a: &PowlArena,
    idx_a: u32,
    arena_b: &PowlArena,
    idx_b: u32,
    path: &str,
    changes: &mut Vec<StructureChange>,
) {
    let ta = node_type_str(arena_a, idx_a);
    let tb = node_type_str(arena_b, idx_b);

    if ta != tb {
        changes.push(StructureChange {
            location: path.to_string(),
            from: ta.clone(),
            to: tb.clone(),
        });
    }

    // Walk children if both are the same composite type
    let children_a: Vec<u32> = match arena_a.get(idx_a) {
        Some(PowlNode::StrictPartialOrder(s)) => s.children.clone(),
        Some(PowlNode::OperatorPowl(o)) => o.children.clone(),
        _ => vec![],
    };
    let children_b: Vec<u32> = match arena_b.get(idx_b) {
        Some(PowlNode::StrictPartialOrder(s)) => s.children.clone(),
        Some(PowlNode::OperatorPowl(o)) => o.children.clone(),
        _ => vec![],
    };

    let min_len = children_a.len().min(children_b.len());
    for i in 0..min_len {
        structural_diff(
            arena_a,
            children_a[i],
            arena_b,
            children_b[i],
            &format!("{}.child[{}]", path, i),
            changes,
        );
    }
}

// ─── Main entry point ─────────────────────────────────────────────────────────

/// Compute the diff between model A (`arena_a`, `root_a`) and model B (`arena_b`, `root_b`).
pub fn diff(
    arena_a: &PowlArena,
    root_a: u32,
    arena_b: &PowlArena,
    root_b: u32,
) -> ModelDiff {
    let fp_a = footprints::apply(arena_a, root_a);
    let fp_b = footprints::apply(arena_b, root_b);

    let added_activities: Vec<String> = fp_b.activities.difference(&fp_a.activities)
        .cloned().collect();
    let removed_activities: Vec<String> = fp_a.activities.difference(&fp_b.activities)
        .cloned().collect();

    // Always-happening changes
    let mut always_changes = Vec::new();
    for act in fp_b.activities_always_happening.difference(&fp_a.activities_always_happening) {
        always_changes.push(AlwaysChange::BecameMandatory(act.clone()));
    }
    for act in fp_a.activities_always_happening.difference(&fp_b.activities_always_happening) {
        always_changes.push(AlwaysChange::BecameOptional(act.clone()));
    }

    // Ordering changes
    let seq_a = footprint_pairs(&fp_a);
    let seq_b = footprint_pairs(&fp_b);
    let par_a = parallel_pairs(&fp_a);
    let par_b = parallel_pairs(&fp_b);

    let mut order_changes = Vec::new();

    for p in seq_b.difference(&seq_a) {
        let pair: Pair = p.clone();
        order_changes.push(OrderChange::SequenceAdded(pair));
    }
    for p in seq_a.difference(&seq_b) {
        let pair: Pair = p.clone();
        order_changes.push(OrderChange::SequenceRemoved(pair));
    }
    for p in par_b.difference(&par_a) {
        let pair: Pair = p.clone();
        order_changes.push(OrderChange::ParallelAdded(pair));
    }
    for p in par_a.difference(&par_b) {
        let pair: Pair = p.clone();
        order_changes.push(OrderChange::ParallelRemoved(pair));
    }
    for a in fp_b.start_activities.difference(&fp_a.start_activities) {
        order_changes.push(OrderChange::StartAdded(a.clone()));
    }
    for a in fp_a.start_activities.difference(&fp_b.start_activities) {
        order_changes.push(OrderChange::StartRemoved(a.clone()));
    }
    for a in fp_b.end_activities.difference(&fp_a.end_activities) {
        order_changes.push(OrderChange::EndAdded(a.clone()));
    }
    for a in fp_a.end_activities.difference(&fp_b.end_activities) {
        order_changes.push(OrderChange::EndRemoved(a.clone()));
    }

    // Structural changes
    let mut structure_changes = Vec::new();
    structural_diff(arena_a, root_a, arena_b, root_b, "root", &mut structure_changes);

    // Trace length delta
    let min_trace_length_delta =
        fp_b.min_trace_length as i64 - fp_a.min_trace_length as i64;

    // Severity
    let severity = if !removed_activities.is_empty()
        || always_changes
            .iter()
            .any(|c| matches!(c, AlwaysChange::BecameOptional(_)))
        || structure_changes.iter().any(|c| c.location == "root")
    {
        Severity::Breaking
    } else if !added_activities.is_empty()
        || !order_changes.is_empty()
        || min_trace_length_delta != 0
    {
        Severity::Moderate
    } else if !always_changes.is_empty() || !structure_changes.is_empty() {
        Severity::Minor
    } else {
        Severity::None
    };

    // Behavioural equivalence (same footprints for shared activities)
    let behaviourally_equivalent = added_activities.is_empty()
        && removed_activities.is_empty()
        && order_changes.is_empty()
        && always_changes.is_empty()
        && min_trace_length_delta == 0;

    ModelDiff {
        added_activities,
        removed_activities,
        always_changes,
        order_changes,
        structure_changes,
        min_trace_length_delta,
        severity,
        behaviourally_equivalent,
    }
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::parse_powl_model_string;
    use crate::powl::PowlArena;

    fn parse(s: &str) -> (PowlArena, u32) {
        let mut arena = PowlArena::new();
        let root = parse_powl_model_string(s, &mut arena).unwrap();
        (arena, root)
    }

    #[test]
    fn identical_models_no_diff() {
        let (aa, ra) = parse("X(A, B)");
        let (ab, rb) = parse("X(A, B)");
        let d = diff(&aa, ra, &ab, rb);
        assert!(d.behaviourally_equivalent);
        assert!(d.is_empty());
        assert_eq!(d.severity, Severity::None);
    }

    #[test]
    fn added_activity_detected() {
        let (aa, ra) = parse("A");
        let (ab, rb) = parse("X(A, B)");
        let d = diff(&aa, ra, &ab, rb);
        assert!(d.added_activities.contains(&"B".to_string()));
        // A becomes optional in X(A,B) vs being mandatory alone → Breaking
        assert!(d.severity >= Severity::Moderate);
    }

    #[test]
    fn removed_activity_is_breaking() {
        let (aa, ra) = parse("X(A, B)");
        let (ab, rb) = parse("A");
        let d = diff(&aa, ra, &ab, rb);
        assert!(d.removed_activities.contains(&"B".to_string()));
        assert_eq!(d.severity, Severity::Breaking);
    }

    #[test]
    fn structure_change_at_root() {
        let (aa, ra) = parse("X(A, B)");
        let (ab, rb) = parse("*(A, B)");
        let d = diff(&aa, ra, &ab, rb);
        assert!(d.structure_changes.iter().any(|c| c.location == "root"));
    }

    #[test]
    fn ordering_change_detected() {
        let (aa, ra) = parse("PO=(nodes={A, B}, order={A-->B})");
        let (ab, rb) = parse("PO=(nodes={A, B}, order={})");
        let d = diff(&aa, ra, &ab, rb);
        // A→B sequence should be removed; A∥B parallel should appear
        assert!(!d.order_changes.is_empty());
    }
}
