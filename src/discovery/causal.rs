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

/// Causal graph discovery from directly-follows graphs.
///
/// Ports `pm4py.algo.discovery.causal`.
///
/// A causal graph identifies which activities have a causal relationship:
/// - A → B is causal if A always precedes B (B never precedes A)
/// - This is the alpha miner's definition of causality
use super::dfg::DFGEdge;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Causal relation result.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CausalGraph {
    /// Causal relations: (from, to) → 1 (binary: either causal or not)
    pub relations: HashMap<(String, String), usize>,
}

/// Discover causal relations using the alpha miner variant.
///
/// Ports `pm4py.algo.discovery.causal.variants.alpha.apply()`.
///
/// A relation (A, B) is causal if:
/// - A directly follows B in the log (frequency > 0)
/// - B never directly follows A (either absent or frequency = 0)
///
/// # Arguments
/// * `dfg_edges` - DFG edges from `discover_dfg()`
///
/// # Returns
/// Causal graph with binary relations (value = 1 if causal)
pub fn discover_causal_alpha(dfg_edges: &[DFGEdge]) -> CausalGraph {
    let mut edge_freq: HashMap<(String, String), usize> = HashMap::new();

    // Build frequency map from DFG edges
    for edge in dfg_edges {
        edge_freq.insert((edge.source.clone(), edge.target.clone()), edge.count);
    }

    let mut causal_alpha: HashMap<(String, String), usize> = HashMap::new();

    for ((from, to), freq) in &edge_freq {
        if *freq > 0 {
            // Check reverse relation
            let reverse_key = (to.clone(), from.clone());
            let is_causal = if let Some(reverse_freq) = edge_freq.get(&reverse_key) {
                // Causal if reverse frequency is 0
                *reverse_freq == 0
            } else {
                // Causal if reverse relation doesn't exist
                true
            };

            if is_causal {
                causal_alpha.insert((from.clone(), to.clone()), 1);
            }
        }
    }

    CausalGraph {
        relations: causal_alpha,
    }
}

/// Discover causal relations using the heuristic variant.
///
/// Ports `pm4py.algo.discovery.causal.variants.heuristic.apply()`.
///
/// The heuristic variant uses a threshold-based approach:
/// - Relation (A, B) is causal if its frequency is significantly higher
///   than the reverse frequency (B, A).
///
/// # Arguments
/// * `dfg_edges` - DFG edges from `discover_dfg()`
/// * `threshold` - Minimum ratio for causality (default: 0.8)
///
/// # Returns
/// Causal graph with strength values (0-1, higher = stronger causality)
pub fn discover_causal_heuristic(dfg_edges: &[DFGEdge], threshold: f64) -> CausalGraph {
    let mut edge_freq: HashMap<(String, String), usize> = HashMap::new();

    // Build frequency map from DFG edges
    for edge in dfg_edges {
        edge_freq.insert((edge.source.clone(), edge.target.clone()), edge.count);
    }

    let mut causal_heuristic: HashMap<(String, String), usize> = HashMap::new();

    for ((from, to), freq) in &edge_freq {
        if *freq > 0 {
            // Check reverse relation
            let reverse_key = (to.clone(), from.clone());
            let strength = if let Some(reverse_freq) = edge_freq.get(&reverse_key) {
                let total = *freq + *reverse_freq;
                if total == 0 {
                    0.0
                } else {
                    (*freq as f64) / (total as f64)
                }
            } else {
                1.0 // No reverse relation = full causality
            };

            if strength >= threshold {
                causal_heuristic.insert((from.clone(), to.clone()), (strength * 1000.0) as usize);
            }
        }
    }

    CausalGraph {
        relations: causal_heuristic,
    }
}

// ─── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_causal_alpha_simple() {
        // A → B → C (linear sequence)
        let edges = vec![
            DFGEdge { source: "A".to_string(), target: "B".to_string(), count: 10 },
            DFGEdge { source: "B".to_string(), target: "C".to_string(), count: 10 },
        ];

        let result = discover_causal_alpha(&edges);

        assert_eq!(result.relations.len(), 2);
        assert_eq!(result.relations.get(&("A".to_string(), "B".to_string())), Some(&1));
        assert_eq!(result.relations.get(&("B".to_string(), "C".to_string())), Some(&1));
    }

    #[test]
    fn test_causal_alpha_no_loop() {
        // A ⇄ B (bidirectional, not causal)
        let edges = vec![
            DFGEdge { source: "A".to_string(), target: "B".to_string(), count: 5 },
            DFGEdge { source: "B".to_string(), target: "A".to_string(), count: 3 },
        ];

        let result = discover_causal_alpha(&edges);

        // No causal relations since both directions exist
        assert_eq!(result.relations.len(), 0);
    }

    #[test]
    fn test_causal_heuristic_threshold() {
        // A → B (10 times), B → A (2 times) - strong causality
        let edges = vec![
            DFGEdge { source: "A".to_string(), target: "B".to_string(), count: 10 },
            DFGEdge { source: "B".to_string(), target: "A".to_string(), count: 2 },
        ];

        let result = discover_causal_heuristic(&edges, 0.8);

        // A → B should be causal (10/12 = 0.83 > 0.8)
        assert_eq!(result.relations.get(&("A".to_string(), "B".to_string())), Some(&833)); // ~0.83 * 1000
    }
}
