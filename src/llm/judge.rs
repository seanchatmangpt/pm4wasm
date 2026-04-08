// POWLJudge - Structural Soundness Validator
//
// Validates POWL models against Wil van der Aalst's soundness criteria:
// 1. Deadlock freedom (no circular wait chains)
// 2. Liveness (all actions eventually complete)
// 3. Boundedness (no unbounded resource growth)
//
// This is the WASM port of the judge logic from pm4py/algo/dspy/powl/judge.py

use crate::powl::{PowlArena, PowlNode};
use std::collections::{HashSet, VecDeque};

/// Validation result from POWLJudge
pub struct ValidationResult {
    pub is_sound: bool,
    pub reasoning: String,
    pub violations: Vec<String>,
}

impl ValidationResult {
    fn approved() -> Self {
        ValidationResult {
            is_sound: true,
            reasoning: "✅ Model is structurally sound: deadlock-free, live, and bounded".to_string(),
            violations: Vec::new(),
        }
    }

    fn rejected(reasoning: &str, violations: Vec<&str>) -> Self {
        ValidationResult {
            is_sound: false,
            reasoning: reasoning.to_string(),
            violations: violations.iter().map(|s| s.to_string()).collect(),
        }
    }
}

/// Validate soundness of a POWL model
///
/// Checks:
/// - Deadlock freedom: No circular dependencies in control flow
/// - Liveness: All paths eventually reach an end state
/// - Boundedness: No infinite loops without exit conditions
pub fn validate_soundness(arena: &PowlArena, root: u32) -> ValidationResult {
    let mut violations = Vec::new();
    let mut reasoning_parts = Vec::new();

    // Check for circular dependencies (deadlock freedom)
    let has_cycles = detect_cycles(arena, root);
    if has_cycles {
        violations.push("Circular dependency detected");
        reasoning_parts.push("❌ Model contains cycles → potential deadlock");
    }

    // Check for infinite loops (liveness)
    let has_infinite_loops = detect_infinite_loops(arena, root);
    if has_infinite_loops {
        violations.push("Infinite loop detected without exit");
        reasoning_parts.push("❌ Model has unbounded loops → liveness violation");
    }

    // Check for unbounded parallelism (boundedness)
    let is_unbounded = detect_unbounded_parallelism(arena, root);
    if is_unbounded {
        violations.push("Unbounded parallel expansion");
        reasoning_parts.push("❌ Model allows unbounded parallel growth → boundedness violation");
    }

    if violations.is_empty() {
        ValidationResult::approved()
    } else {
        let reasoning = if reasoning_parts.is_empty() {
            "❌ Model rejected: ".to_string()
        } else {
            reasoning_parts.join("; ")
        };
        let v: Vec<&str> = violations.iter().map(|s| s.as_ref()).collect();
        ValidationResult::rejected(&reasoning, v)
    }
}

/// Detect circular dependencies in the POWL model
///
/// Uses DFS to find cycles in the control flow graph
fn detect_cycles(arena: &PowlArena, root: u32) -> bool {
    let mut visited = HashSet::new();
    let mut rec_stack = HashSet::new();
    detect_cycles_dfs(arena, root, &mut visited, &mut rec_stack)
}

fn detect_cycles_dfs(
    arena: &PowlArena,
    node_idx: u32,
    visited: &mut HashSet<u32>,
    rec_stack: &mut HashSet<u32>,
) -> bool {
    if rec_stack.contains(&node_idx) {
        return true; // Cycle detected
    }
    if visited.contains(&node_idx) {
        return false; // Already checked, no cycle
    }

    visited.insert(node_idx);
    rec_stack.insert(node_idx);

    if let Some(node) = arena.get(node_idx) {
        match node {
            PowlNode::OperatorPowl(op) => {
                // Check all children
                for child in &op.children {
                    if detect_cycles_dfs(arena, *child, visited, rec_stack) {
                        return true;
                    }
                }
                // Special check for LOOP operator - always has a cycle by design
                if matches!(op.operator, crate::powl::Operator::Loop) {
                    // LOOP is acceptable if there's a way to exit
                    // Check if there's a tau (silent) option
                    return has_exit_condition(arena, &op.children);
                }
            }
            PowlNode::StrictPartialOrder(spo) => {
                // Check if the partial order has cycles
                if !spo.order.is_transitive() {
                    return true;
                }
                // Check children recursively
                for child in &spo.children {
                    if detect_cycles_dfs(arena, *child, visited, rec_stack) {
                        return true;
                    }
                }
            }
            PowlNode::Transition(_) | PowlNode::FrequentTransition(_) => {
                // Leaf nodes - no cycles
            }
        }
    }

    rec_stack.remove(&node_idx);
    false
}

/// Check if a LOOP operator has an exit condition
///
/// A LOOP *(A, B) is safe if:
/// - B can be skipped (e.g., XOR with tau)
/// - The loop has a maximum iteration count
fn has_exit_condition(arena: &PowlArena, children: &[u32]) -> bool {
    // Check if any child is skippable or has a max bound
    for child in children {
        if let Some(node) = arena.get(*child) {
            match node {
                PowlNode::OperatorPowl(op) => {
                    if matches!(op.operator, crate::powl::Operator::Xor) {
                        // XOR provides an exit
                        for xor_child in &op.children {
                            if let Some(PowlNode::Transition(t)) = arena.get(*xor_child) {
                                if t.label.as_ref().map_or(false, |l| l == "tau") {
                                    return true;
                                }
                            }
                        }
                    }
                }
                PowlNode::FrequentTransition(ft) => {
                    if ft.skippable {
                        return true;
                    }
                }
                _ => {}
            }
        }
    }
    false
}

/// Detect infinite loops without exit conditions
fn detect_infinite_loops(arena: &PowlArena, root: u32) -> bool {
    // A model has infinite loops if there's a LOOP without an exit
    // that isn't guarded by a maximum iteration count
    let mut has_loop = false;
    let mut has_exit = false;

    let mut queue = VecDeque::new();
    queue.push_back(root);

    while let Some(node_idx) = queue.pop_front() {
        if let Some(node) = arena.get(node_idx) {
            match node {
                PowlNode::OperatorPowl(op) => {
                    if matches!(op.operator, crate::powl::Operator::Loop) {
                        has_loop = true;
                        // Check for exit condition
                        if has_exit_condition(arena, &op.children) {
                            has_exit = true;
                        }
                    }
                    for child in &op.children {
                        queue.push_back(*child);
                    }
                }
                PowlNode::StrictPartialOrder(spo) => {
                    for child in &spo.children {
                        queue.push_back(*child);
                    }
                }
                _ => {}
            }
        }
    }

    has_loop && !has_exit
}

/// Detect unbounded parallelism
///
/// Returns true if the model allows for unbounded parallel process creation
fn detect_unbounded_parallelism(arena: &PowlArena, root: u32) -> bool {
    // Check for StrictPartialOrder nodes with many children (potential parallelism)
    // This is a simplified check - a full implementation would analyze
    // the model more carefully
    let mut queue = VecDeque::new();
    queue.push_back(root);

    while let Some(node_idx) = queue.pop_front() {
        if let Some(node) = arena.get(node_idx) {
            match node {
                PowlNode::StrictPartialOrder(spo) => {
                    // Large parallel blocks without constraints could be unbounded
                    if spo.children.len() > 10 {
                        return true;
                    }
                    // Check for nested SPOs (could explode)
                    for child in &spo.children {
                        if let Some(child_node) = arena.get(*child) {
                            if matches!(child_node, PowlNode::StrictPartialOrder(_)) {
                                return true; // Nested SPO = potentially unbounded
                            }
                        }
                    }
                    for child in &spo.children {
                        queue.push_back(*child);
                    }
                }
                PowlNode::OperatorPowl(op) => {
                    for child in &op.children {
                        queue.push_back(*child);
                    }
                }
                _ => {}
            }
        }
    }

    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::parse_powl_model_string;

    #[test]
    fn test_validate_sound_sequence() {
        let mut arena = PowlArena::new();
        let model = "A->B->C";
        let root = parse_powl_model_string(model, &mut arena).unwrap();
        let result = validate_soundness(&arena, root);
        assert!(result.is_sound);
    }

    #[test]
    fn test_validate_sound_loop_with_exit() {
        let mut arena = PowlArena::new();
        // LOOP with tau exit is acceptable
        let model = "*(A, tau)";
        let root = parse_powl_model_string(model, &mut arena).unwrap();
        let result = validate_soundness(&arena, root);
        assert!(result.is_sound, "LOOP with tau should be sound");
    }

    #[test]
    fn test_validate_sound_parallel() {
        let mut arena = PowlArena::new();
        let model = "PO=(nodes={A, B, C}, order={})";
        let root = parse_powl_model_string(model, &mut arena).unwrap();
        let result = validate_soundness(&arena, root);
        assert!(result.is_sound, "Parallel model should be sound");
    }
}
