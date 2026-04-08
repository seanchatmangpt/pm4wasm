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

/// DFG (Directly-Follows Graph) serialization and deserialization.
///
/// Provides JSON-based import/export for DFG structures.
use crate::discovery::dfg::DFGResult;
#[cfg(test)]
use crate::discovery::dfg::DFGEdge;
use wasm_bindgen::prelude::*;

/// Serialize a DFG to JSON string.
pub fn dfg_to_json(dfg: &DFGResult) -> String {
    serde_json::to_string(dfg).unwrap_or_else(|_| "{}".to_string())
}

/// Deserialize a DFG from JSON string.
pub fn dfg_from_json(json: &str) -> Result<DFGResult, String> {
    serde_json::from_str(json).map_err(|e| format!("Failed to parse DFG JSON: {}", e))
}

/// Serialize a DFG to a simple graphviz DOT format.
pub fn dfg_to_dot(dfg: &DFGResult) -> String {
    let mut dot = String::from("digraph DFG {\n");
    dot.push_str("  rankdir=LR;\n");
    dot.push_str("  node [shape=box];\n\n");

    // Add all activity nodes
    let mut all_activities: std::collections::HashSet<String> = std::collections::HashSet::new();
    for (act, _) in &dfg.start_activities {
        all_activities.insert(act.clone());
    }
    for (act, _) in &dfg.end_activities {
        all_activities.insert(act.clone());
    }
    for (act, _) in &dfg.activities {
        all_activities.insert(act.clone());
    }
    for edge in &dfg.edges {
        all_activities.insert(edge.source.clone());
        all_activities.insert(edge.target.clone());
    }

    for act in &all_activities {
        dot.push_str(&format!("  \"{}\";\n", act));
    }
    dot.push('\n');

    // Add edges
    for edge in &dfg.edges {
        dot.push_str(&format!(
            "  \"{}\" -> \"{}\" [label=\"{}\"];\n",
            edge.source, edge.target, edge.count
        ));
    }

    dot.push_str("}\n");
    dot
}

/// WASM export: DFG to JSON.
#[wasm_bindgen]
pub fn dfg_to_json_wasm(dfg_json: &str) -> String {
    match dfg_from_json(dfg_json) {
        Ok(dfg) => dfg_to_json(&dfg),
        Err(_) => "{}".to_string(),
    }
}

/// WASM export: DFG to DOT.
#[wasm_bindgen]
pub fn dfg_to_dot_wasm(dfg_json: &str) -> String {
    match dfg_from_json(dfg_json) {
        Ok(dfg) => dfg_to_dot(&dfg),
        Err(_) => "digraph DFG {}".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_dfg() -> DFGResult {
        DFGResult {
            edges: vec![
                DFGEdge {
                    source: "A".to_string(),
                    target: "B".to_string(),
                    count: 5,
                },
                DFGEdge {
                    source: "B".to_string(),
                    target: "C".to_string(),
                    count: 3,
                },
            ],
            start_activities: vec![("A".to_string(), 5)],
            end_activities: vec![("C".to_string(), 3)],
            activities: vec![
                ("A".to_string(), 5),
                ("B".to_string(), 5),
                ("C".to_string(), 3),
            ],
        }
    }

    #[test]
    fn test_dfg_json_roundtrip() {
        let dfg = sample_dfg();
        let json = dfg_to_json(&dfg);
        let restored = dfg_from_json(&json).unwrap();
        assert_eq!(restored.edges.len(), 2);
        assert_eq!(restored.edges[0].source, "A");
        assert_eq!(restored.edges[0].count, 5);
    }

    #[test]
    fn test_dfg_to_dot() {
        let dfg = sample_dfg();
        let dot = dfg_to_dot(&dfg);
        assert!(dot.contains("digraph DFG"));
        assert!(dot.contains("\"A\" -> \"B\""));
        assert!(dot.contains("label=\"5\""));
    }

    #[test]
    fn test_dfg_from_json_invalid() {
        let result = dfg_from_json("not json");
        assert!(result.is_err());
    }
}
