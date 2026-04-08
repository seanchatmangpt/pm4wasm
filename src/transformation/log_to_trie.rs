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

//! Prefix tree (trie) discovery from event logs.
//!
//! **Reference**: `pm4py.algo.transformation.log_to_trie`
//!
//! Builds a trie (prefix tree) from an event log. Each unique trace prefix
//! becomes a node in the tree, allowing efficient prefix-based operations
//! like log comparison and compression.

use crate::event_log::EventLog;
use crate::trie::Trie;
use std::collections::HashMap;

/// Discover a prefix tree (trie) from an event log.
///
/// Each unique trace prefix in the log becomes a path in the trie.
/// Nodes that represent the end of a trace are marked as `is_final = true`.
///
/// **Arguments:**
/// * `log` - Event log
/// * `max_path_length` - Optional maximum trace length (traces are truncated)
///
/// **Returns:** A Trie structure with all trace prefixes
///
/// Mirrors `pm4py.discover_prefix_tree()`.
///
/// **Algorithm:**
/// 1. Get all unique variants from the log
/// 2. For each variant, walk down the trie creating nodes as needed
/// 3. Mark the final node of each variant as `is_final = true`
pub fn discover_prefix_tree(log: &EventLog, max_path_length: Option<usize>) -> Trie {
    // Get all unique variants (activity sequences)
    let variants = get_variants_from_log(log);
    let mut trie = Trie::new();

    for variant in &variants {
        // Truncate variant if max_path_length is specified
        let activities = if let Some(max_len) = max_path_length {
            if variant.activities.len() > max_len {
                &variant.activities[..max_len]
            } else {
                &variant.activities
            }
        } else {
            &variant.activities
        };

        // Walk down the trie, creating nodes as needed
        let mut current_idx = 0; // Start at root

        for (i, activity) in activities.iter().enumerate() {
            // Find or create child node with this activity
            current_idx = trie.get_or_create_child(current_idx, activity);

            // Mark as final if this is the last activity in the variant
            if i == activities.len() - 1 {
                trie.mark_final(current_idx);
            }
        }
    }

    trie
}

/// Get variants from an event log.
///
/// This is a helper function that extracts unique activity sequences
/// along with their counts. Used by prefix tree discovery.
fn get_variants_from_log(log: &EventLog) -> Vec<Variant> {
    let mut variant_map: HashMap<Vec<String>, usize> = HashMap::new();

    for trace in &log.traces {
        let activities: Vec<String> = trace.events.iter().map(|e| e.name.clone()).collect();
        *variant_map.entry(activities.clone()).or_insert(0) += 1;
    }

    // Convert to Variant structs
    variant_map
        .into_iter()
        .map(|(activities, count)| Variant { activities, count })
        .collect()
}

/// A variant represents a unique trace with its frequency.
#[derive(Clone, Debug)]
struct Variant {
    activities: Vec<String>,
    count: usize,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::event_log::{Event, Trace};

    fn make_test_log(activities: Vec<Vec<&str>>) -> EventLog {
        let traces = activities
            .into_iter()
            .enumerate()
            .map(|(i, acts)| Trace {
                case_id: format!("{}", i),
                events: acts
                    .into_iter()
                    .map(|a| Event {
                        name: a.to_string(),
                        timestamp: None,
                        lifecycle: None,
                        attributes: HashMap::new(),
                    })
                    .collect(),
            })
            .collect();
        EventLog { traces }
    }

    #[test]
    fn test_discover_prefix_tree_simple() {
        let log = make_test_log(vec![vec!["A", "B"], vec!["A", "C"]]);
        let trie = discover_prefix_tree(&log, None);

        // Root should have one child: A
        assert_eq!(trie.root().children.len(), 1);

        // A should have two children: B and C
        let a_idx = trie.root().children[0];
        assert_eq!(trie.nodes[a_idx].children.len(), 2);

        // Both B and C should be marked as final
        let b_idx = trie.nodes[a_idx].children[0];
        let c_idx = trie.nodes[a_idx].children[1];
        assert!(trie.nodes[b_idx].is_final);
        assert!(trie.nodes[c_idx].is_final);
    }

    #[test]
    fn test_discover_prefix_tree_reuse_path() {
        let log = make_test_log(vec![vec!["A", "B"], vec!["A", "B", "C"]]);
        let trie = discover_prefix_tree(&log, None);

        // Root -> A -> B (shared path)
        let a_idx = trie.root().children[0];
        let b_idx = trie.nodes[a_idx].children[0];

        // B should have one child: C
        assert_eq!(trie.nodes[b_idx].children.len(), 1);

        // B should be final (first trace ends at B)
        assert!(trie.nodes[b_idx].is_final);

        // C should be final
        let c_idx = trie.nodes[b_idx].children[0];
        assert!(trie.nodes[c_idx].is_final);
    }

    #[test]
    fn test_discover_prefix_tree_max_length() {
        let log = make_test_log(vec![vec!["A", "B", "C", "D"]]);
        let trie = discover_prefix_tree(&log, Some(2));

        // Should only have A -> B (truncated to 2)
        let a_idx = trie.root().children[0];
        assert_eq!(trie.nodes[a_idx].children.len(), 1);

        let b_idx = trie.nodes[a_idx].children[0];
        assert_eq!(trie.nodes[b_idx].children.len(), 0);
        assert!(trie.nodes[b_idx].is_final);
    }

    #[test]
    fn test_discover_prefix_tree_single_activity() {
        let log = make_test_log(vec![vec!["A"]]);
        let trie = discover_prefix_tree(&log, None);

        // Root -> A
        let a_idx = trie.root().children[0];
        assert_eq!(trie.nodes[a_idx].label.as_deref(), Some("A"));
        assert!(trie.nodes[a_idx].is_final);
    }

    #[test]
    fn test_discover_prefix_tree_empty_log() {
        let log = EventLog { traces: vec![] };
        let trie = discover_prefix_tree(&log, None);

        // Should have just the root node
        assert_eq!(trie.nodes.len(), 1);
        assert!(trie.root().children.is_empty());
    }
}
