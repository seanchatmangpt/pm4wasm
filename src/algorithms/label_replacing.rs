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

/// Label replacement utility for POWL models.
///
/// Ports `pm4py/objects/powl/utils/label_replacing.py:apply`.
use crate::powl::{PowlArena, PowlNode};
use std::collections::HashMap;
use wasm_bindgen::prelude::*;

/// Replace activity labels in a POWL subtree according to a dictionary.
///
/// Creates a deep copy of the subtree with all transition labels mapped via the dictionary.
pub fn apply(
    arena: &PowlArena,
    node_idx: u32,
    label_map: &HashMap<String, String>,
    dest_arena: &mut PowlArena,
) -> u32 {
    match arena.get(node_idx) {
        None => node_idx, // shouldn't happen

        Some(PowlNode::Transition(t)) => {
            let new_label = t.label.as_ref().and_then(|l| {
                label_map.get(l).cloned().or_else(|| Some(l.clone()))
            });
            dest_arena.add_transition(new_label)
        }

        Some(PowlNode::FrequentTransition(t)) => {
            let new_activity = label_map
                .get(&t.activity)
                .cloned()
                .unwrap_or_else(|| t.activity.clone());
            let min_freq = if t.skippable { 0 } else { 1 };
            let max_freq = if t.selfloop { None } else { Some(1) };
            dest_arena.add_frequent_transition(new_activity, min_freq, max_freq)
        }

        Some(PowlNode::OperatorPowl(op)) => {
            let new_children: Vec<u32> = op
                .children
                .iter()
                .map(|&c| apply(arena, c, label_map, dest_arena))
                .collect();
            dest_arena.add_operator(op.operator, new_children)
        }

        Some(PowlNode::StrictPartialOrder(spo)) => {
            let old_children = spo.children.clone();
            let old_order = spo.order.clone();
            let mut new_children: Vec<u32> = Vec::new();
            let n = old_children.len();

            for &c in &old_children {
                new_children.push(apply(arena, c, label_map, dest_arena));
            }

            let spo_idx = dest_arena.add_strict_partial_order(new_children);

            // Restore edges (indices map 1-to-1)
            for i in 0..n {
                for j in 0..n {
                    if old_order.is_edge(i, j) {
                        dest_arena.add_order_edge(spo_idx, i, j);
                    }
                }
            }

            spo_idx
        }
    }
}

// ─── WASM exports ─────────────────────────────────────────────────────────────

/// Replace activity labels in a POWL model.
///
/// # Arguments
/// * `model_str` - POWL model string representation
/// * `label_map_json` - JSON string mapping old labels to new labels (e.g., {"A": "Start", "B": "End"})
///
/// # Returns
/// * New POWL model string with labels replaced
///
/// # Example
/// ```javascript
/// const powl = await Powl.init();
/// const result = powl.replaceLabels("X(A, B)", '{"A": "X", "B": "Y"}');
/// // Returns: "X ( X, Y )"
/// ```
#[wasm_bindgen]
pub fn replace_labels(model_str: &str, label_map_json: &str) -> Result<String, JsValue> {
    // Parse the model
    let mut arena = PowlArena::new();
    let root = crate::parser::parse_powl_model_string(model_str, &mut arena)
        .map_err(|e| JsValue::from_str(&format!("Parse error: {}", e)))?;

    // Parse the label map
    let label_map: HashMap<String, String> = serde_json::from_str(label_map_json)
        .map_err(|e| JsValue::from_str(&format!("Invalid label map JSON: {}", e)))?;

    // Create destination arena and apply replacement
    let mut dest_arena = PowlArena::new();
    let new_root = apply(&arena, root, &label_map, &mut dest_arena);

    // Return the new model as a string
    Ok(dest_arena.to_repr(new_root))
}

// ─── tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::parse_powl_model_string;

    #[test]
    fn replace_single_label() {
        let mut arena = PowlArena::new();
        let root = parse_powl_model_string("A", &mut arena).unwrap();

        let mut map = HashMap::new();
        map.insert("A".to_string(), "B".to_string());

        let mut dest = PowlArena::new();
        let new_root = apply(&arena, root, &map, &mut dest);

        let repr = dest.to_repr(new_root);
        assert_eq!(repr, "B");
    }

    #[test]
    fn replace_in_xor() {
        let mut arena = PowlArena::new();
        let root = parse_powl_model_string("X ( A, B )", &mut arena).unwrap();

        let mut map = HashMap::new();
        map.insert("A".to_string(), "X".to_string());
        map.insert("B".to_string(), "Y".to_string());

        let mut dest = PowlArena::new();
        let new_root = apply(&arena, root, &map, &mut dest);

        let repr = dest.to_repr(new_root);
        assert!(repr.contains("X") && repr.contains("Y"), "got: {}", repr);
    }

    #[test]
    fn replace_in_po() {
        let mut arena = PowlArena::new();
        let root =
            parse_powl_model_string("PO=(nodes={A, B}, order={A-->B})", &mut arena).unwrap();

        let mut map = HashMap::new();
        map.insert("A".to_string(), "Start".to_string());
        map.insert("B".to_string(), "End".to_string());

        let mut dest = PowlArena::new();
        let new_root = apply(&arena, root, &map, &mut dest);

        let repr = dest.to_repr(new_root);
        assert!(repr.contains("Start") && repr.contains("End"), "got: {}", repr);
    }
}
