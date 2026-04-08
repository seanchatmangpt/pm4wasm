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

//! Trie (prefix tree) data structure for trace prefix analysis.
//!
//! **Reference**: `pm4py.objects.trie.obj.Trie`
//!
//! A trie represents all unique prefixes of activity sequences in an event log.
//! Each node represents an activity, and paths from root represent trace prefixes.
//! Leaf nodes (or intermediate nodes) are marked as `is_final` if they represent
//! the end of a trace in the log.

use serde::{Deserialize, Serialize};

/// Trie node representing a single activity in the prefix tree.
///
/// Each node contains:
/// - `label`: The activity name (None for the root node)
/// - `parent`: Index of parent node (None for root, used for tree reconstruction)
/// - `children`: List of child node indices
/// - `is_final`: True if this node represents the end of a trace
/// - `depth`: Depth in the tree (root = 0)
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TrieNode {
    /// Activity name (None for root node)
    pub label: Option<String>,
    /// Index of parent node in the nodes array (None for root)
    pub parent: Option<usize>,
    /// Indices of child nodes in the nodes array
    pub children: Vec<usize>,
    /// True if this node marks the end of a trace
    #[serde(rename = "final")]
    pub is_final: bool,
    /// Depth in the tree (root = 0, increments by 1 per level)
    pub depth: usize,
}

impl TrieNode {
    /// Create a new root node (depth 0, no label, no parent).
    pub fn root() -> Self {
        TrieNode {
            label: None,
            parent: None,
            children: Vec::new(),
            is_final: false,
            depth: 0,
        }
    }

    /// Create a new child node with the given label and parent.
    pub fn child(label: String, parent: usize, depth: usize) -> Self {
        TrieNode {
            label: Some(label),
            parent: Some(parent),
            children: Vec::new(),
            is_final: false,
            depth,
        }
    }
}

/// A complete trie structure containing all nodes.
///
/// The trie is stored as a flat vector of nodes for efficient serialization.
/// The root is always at index 0.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Trie {
    /// All nodes in the trie (index 0 is always the root)
    pub nodes: Vec<TrieNode>,
}

impl Trie {
    /// Create a new empty trie with just a root node.
    pub fn new() -> Self {
        Trie {
            nodes: vec![TrieNode::root()],
        }
    }

    /// Get the root node (index 0).
    pub fn root(&self) -> &TrieNode {
        &self.nodes[0]
    }

    /// Find or create a child node with the given label from the given parent.
    ///
    /// Returns the index of the child node.
    pub fn get_or_create_child(&mut self, parent_idx: usize, label: &str) -> usize {
        let parent = &self.nodes[parent_idx];
        let depth = parent.depth + 1;

        // Check if child with this label already exists
        for &child_idx in &parent.children {
            if let Some(ref child_label) = self.nodes[child_idx].label {
                if child_label == label {
                    return child_idx;
                }
            }
        }

        // Create new child
        let child_idx = self.nodes.len();
        self.nodes.push(TrieNode::child(label.to_string(), parent_idx, depth));
        self.nodes[parent_idx].children.push(child_idx);
        child_idx
    }

    /// Mark a node as final (end of trace).
    pub fn mark_final(&mut self, node_idx: usize) {
        self.nodes[node_idx].is_final = true;
    }

    /// Get a string representation of the trie (for debugging).
    pub fn to_string(&self) -> String {
        let mut result = String::new();
        self.append_to_string(0, String::new(), &mut result);
        result
    }

    fn append_to_string(&self, node_idx: usize, prefix: String, result: &mut String) {
        let node = &self.nodes[node_idx];

        if let Some(ref label) = node.label {
            result.push_str(&prefix);
            result.push_str(label);
            result.push('\n');

            let mut child_prefix = prefix.clone();
            if node.is_final {
                result.push_str(&prefix);
                result.push_str("  [FINAL]\n");
            }
            child_prefix.push_str("  ");

            for &child_idx in &node.children {
                self.append_to_string(child_idx, child_prefix.clone(), result);
            }
        } else {
            // Root node - just traverse children
            for &child_idx in &node.children {
                self.append_to_string(child_idx, prefix.clone(), result);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_trie_root() {
        let trie = Trie::new();
        assert_eq!(trie.nodes.len(), 1);
        assert!(trie.nodes[0].label.is_none());
        assert!(trie.nodes[0].parent.is_none());
        assert_eq!(trie.nodes[0].depth, 0);
    }

    #[test]
    fn test_trie_add_child() {
        let mut trie = Trie::new();
        let child_idx = trie.get_or_create_child(0, "A");
        assert_eq!(child_idx, 1);
        assert_eq!(trie.nodes.len(), 2);
        assert_eq!(trie.nodes[child_idx].label.as_deref(), Some("A"));
        assert_eq!(trie.nodes[child_idx].parent, Some(0));
        assert_eq!(trie.nodes[child_idx].depth, 1);
        assert_eq!(trie.nodes[0].children.len(), 1);
    }

    #[test]
    fn test_trie_reuse_child() {
        let mut trie = Trie::new();
        let idx1 = trie.get_or_create_child(0, "A");
        let idx2 = trie.get_or_create_child(0, "A");
        assert_eq!(idx1, idx2);
        assert_eq!(trie.nodes[0].children.len(), 1);
    }

    #[test]
    fn test_trie_mark_final() {
        let mut trie = Trie::new();
        let child_idx = trie.get_or_create_child(0, "A");
        trie.mark_final(child_idx);
        assert!(trie.nodes[child_idx].is_final);
    }

    #[test]
    fn test_trie_two_paths() {
        let mut trie = Trie::new();

        // Add path A -> B
        let a_idx = trie.get_or_create_child(0, "A");
        let b_idx = trie.get_or_create_child(a_idx, "B");

        // Add path A -> C
        let c_idx = trie.get_or_create_child(a_idx, "C");

        assert_eq!(trie.nodes[0].children.len(), 1); // Just A
        assert_eq!(trie.nodes[a_idx].children.len(), 2); // B and C
        assert_eq!(trie.nodes[b_idx].children.len(), 0);
        assert_eq!(trie.nodes[c_idx].children.len(), 0);
    }

    #[test]
    fn test_trie_to_string() {
        let mut trie = Trie::new();
        let a_idx = trie.get_or_create_child(0, "A");
        trie.mark_final(a_idx);

        let s = trie.to_string();
        assert!(s.contains("A"));
        assert!(s.contains("[FINAL]"));
    }
}
