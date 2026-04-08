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

/// Bit-packed adjacency matrix for partial order relations.
///
/// Mirrors `pm4py/objects/powl/BinaryRelation.py` but uses a flat Vec<u64>
/// where row `i` is stored at words[i * row_words .. (i+1) * row_words].
/// This gives cache-friendly row-OR operations for the Warshall closure.
#[derive(Clone, Debug)]
pub struct BinaryRelation {
    pub n: usize,
    pub row_words: usize,
    /// Flat bit-matrix; bit j of words[i*row_words + j/64] represents edge i→j.
    pub words: Vec<u64>,
}

impl BinaryRelation {
    /// Create an n×n zero matrix.
    pub fn new(n: usize) -> Self {
        let row_words = if n == 0 { 0 } else { (n + 63) / 64 };
        BinaryRelation {
            n,
            row_words,
            words: vec![0u64; n * row_words],
        }
    }

    #[inline]
    fn word_idx(&self, i: usize, j: usize) -> (usize, u32) {
        let idx = i * self.row_words + j / 64;
        let bit = (j % 64) as u32;
        (idx, bit)
    }

    pub fn add_edge(&mut self, i: usize, j: usize) {
        assert!(i < self.n && j < self.n, "edge index out of bounds");
        let (idx, bit) = self.word_idx(i, j);
        self.words[idx] |= 1u64 << bit;
    }

    pub fn remove_edge(&mut self, i: usize, j: usize) {
        assert!(i < self.n && j < self.n, "edge index out of bounds");
        let (idx, bit) = self.word_idx(i, j);
        self.words[idx] &= !(1u64 << bit);
    }

    #[inline]
    pub fn is_edge(&self, i: usize, j: usize) -> bool {
        if i >= self.n || j >= self.n {
            return false;
        }
        let (idx, bit) = self.word_idx(i, j);
        (self.words[idx] >> bit) & 1 == 1
    }

    /// O(n) — check no self-loops.
    pub fn is_irreflexive(&self) -> bool {
        for i in 0..self.n {
            if self.is_edge(i, i) {
                return false;
            }
        }
        true
    }

    /// O(n³) — check transitivity: for all i,j,k: edge(i,j) ∧ edge(j,k) → edge(i,k).
    pub fn is_transitive(&self) -> bool {
        for i in 0..self.n {
            for j in 0..self.n {
                if !self.is_edge(i, j) {
                    continue;
                }
                for k in 0..self.n {
                    if self.is_edge(j, k) && !self.is_edge(i, k) {
                        return false;
                    }
                }
            }
        }
        true
    }

    pub fn is_strict_partial_order(&self) -> bool {
        self.is_irreflexive() && self.is_transitive()
    }

    /// Floyd-Warshall transitive closure, O(n³) with word-level OR operations.
    /// Modifies self in-place.
    pub fn add_transitive_edges(&mut self) {
        // Warshall's algorithm: for each pivot k, for each row i that has
        // edge i→k, OR row k into row i.
        for k in 0..self.n {
            for i in 0..self.n {
                if self.is_edge(i, k) {
                    // OR row k into row i  (word-level, cache-friendly)
                    let row_i_start = i * self.row_words;
                    let row_k_start = k * self.row_words;
                    for w in 0..self.row_words {
                        self.words[row_i_start + w] |= self.words[row_k_start + w];
                    }
                }
            }
        }
    }

    /// O(n³) — return a new relation with redundant edges removed.
    /// An edge i→k is redundant if there exists j with edge(i,j) ∧ edge(j,k).
    pub fn get_transitive_reduction(&self) -> Self {
        assert!(self.is_irreflexive(), "transitive reduction requires irreflexivity");
        let mut res = self.clone();
        for i in 0..self.n {
            for j in 0..self.n {
                if !self.is_edge(i, j) {
                    continue;
                }
                for k in 0..self.n {
                    if i != j && j != k && self.is_edge(j, k) && res.is_edge(i, k) {
                        res.remove_edge(i, k);
                    }
                }
            }
        }
        res
    }

    /// O(n²) — nodes with no incoming edges (in-degree == 0).
    pub fn get_start_nodes(&self) -> Vec<usize> {
        let mut has_incoming = vec![false; self.n];
        for i in 0..self.n {
            for j in 0..self.n {
                if self.is_edge(i, j) {
                    has_incoming[j] = true;
                }
            }
        }
        (0..self.n).filter(|&j| !has_incoming[j]).collect()
    }

    /// O(n²) — nodes with no outgoing edges (out-degree == 0).
    pub fn get_end_nodes(&self) -> Vec<usize> {
        let mut has_outgoing = vec![false; self.n];
        for i in 0..self.n {
            for j in 0..self.n {
                if self.is_edge(i, j) {
                    has_outgoing[i] = true;
                }
            }
        }
        (0..self.n).filter(|&i| !has_outgoing[i]).collect()
    }

    /// Remove an edge while maintaining transitivity (mirrors Python impl).
    pub fn remove_edge_without_violating_transitivity(&mut self, src: usize, tgt: usize) {
        self.remove_edge(src, tgt);
        let n = self.n;
        let mut changed = true;
        while changed {
            changed = false;
            for i in 0..n {
                for j in 0..n {
                    if i == j || !self.is_edge(i, j) {
                        continue;
                    }
                    for k in 0..n {
                        if j == k {
                            continue;
                        }
                        if self.is_edge(j, k) && !self.is_edge(i, k) {
                            self.remove_edge(j, k);
                            changed = true;
                        }
                    }
                }
            }
        }
    }

    /// Grow by one node; preserves all existing edges.
    pub fn add_node(&mut self) -> usize {
        let new_n = self.n + 1;
        let new_row_words = (new_n + 63) / 64;
        if new_row_words != self.row_words {
            // Row width changes — rebuild matrix
            let mut new_words = vec![0u64; new_n * new_row_words];
            for i in 0..self.n {
                for w in 0..self.row_words {
                    new_words[i * new_row_words + w] = self.words[i * self.row_words + w];
                }
            }
            self.row_words = new_row_words;
            self.words = new_words;
        } else {
            // Just append a zero row
            for _ in 0..new_row_words {
                self.words.push(0u64);
            }
        }
        self.n = new_n;
        self.n - 1
    }

    /// Serialise as a list of (src, tgt) pairs (used for WASM JS export).
    pub fn edge_list(&self) -> Vec<(usize, usize)> {
        let mut edges = Vec::new();
        for i in 0..self.n {
            for j in 0..self.n {
                if self.is_edge(i, j) {
                    edges.push((i, j));
                }
            }
        }
        edges
    }
}

// ─── tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_is_strict_partial_order() {
        let r = BinaryRelation::new(0);
        assert!(r.is_strict_partial_order());
    }

    #[test]
    fn single_node_no_edges() {
        let r = BinaryRelation::new(1);
        assert!(r.is_irreflexive());
        assert!(r.is_transitive());
        assert_eq!(r.get_start_nodes(), vec![0]);
        assert_eq!(r.get_end_nodes(), vec![0]);
    }

    #[test]
    fn add_remove_edge() {
        let mut r = BinaryRelation::new(3);
        r.add_edge(0, 1);
        assert!(r.is_edge(0, 1));
        assert!(!r.is_edge(1, 0));
        r.remove_edge(0, 1);
        assert!(!r.is_edge(0, 1));
    }

    #[test]
    fn is_irreflexive_detects_self_loop() {
        let mut r = BinaryRelation::new(3);
        r.add_edge(1, 1);
        assert!(!r.is_irreflexive());
    }

    #[test]
    fn transitivity_check() {
        // 0→1, 1→2 but missing 0→2 ⇒ not transitive
        let mut r = BinaryRelation::new(3);
        r.add_edge(0, 1);
        r.add_edge(1, 2);
        assert!(!r.is_transitive());
        r.add_edge(0, 2);
        assert!(r.is_transitive());
    }

    #[test]
    fn transitive_closure() {
        let mut r = BinaryRelation::new(3);
        r.add_edge(0, 1);
        r.add_edge(1, 2);
        r.add_transitive_edges();
        assert!(r.is_edge(0, 2));
        assert!(r.is_transitive());
    }

    #[test]
    fn transitive_reduction() {
        let mut r = BinaryRelation::new(3);
        r.add_edge(0, 1);
        r.add_edge(1, 2);
        r.add_edge(0, 2); // redundant
        let red = r.get_transitive_reduction();
        assert!(red.is_edge(0, 1));
        assert!(red.is_edge(1, 2));
        assert!(!red.is_edge(0, 2));
    }

    #[test]
    fn start_end_nodes() {
        // 0→1, 1→2
        let mut r = BinaryRelation::new(3);
        r.add_edge(0, 1);
        r.add_edge(1, 2);
        assert_eq!(r.get_start_nodes(), vec![0]);
        assert_eq!(r.get_end_nodes(), vec![2]);
    }

    #[test]
    fn add_node_preserves_edges() {
        let mut r = BinaryRelation::new(2);
        r.add_edge(0, 1);
        let new_id = r.add_node();
        assert_eq!(new_id, 2);
        assert!(r.is_edge(0, 1));
        assert!(!r.is_edge(0, 2));
    }

    #[test]
    fn large_matrix_bit_packing() {
        // 65 nodes to exercise multi-word rows
        let mut r = BinaryRelation::new(65);
        r.add_edge(0, 64);
        r.add_edge(64, 32);
        r.add_transitive_edges();
        assert!(r.is_edge(0, 32));
    }
}
