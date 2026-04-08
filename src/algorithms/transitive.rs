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

/// Transitive-closure and transitive-reduction helpers operating directly
/// on `BinaryRelation`.  The methods on `BinaryRelation` itself already
/// contain the core bit-packed implementations; this module provides
/// higher-level utilities used by the discovery algorithms.
use crate::binary_relation::BinaryRelation;

/// Compute the transitive closure of `rel` and return a new relation.
/// (Non-destructive wrapper around `BinaryRelation::add_transitive_edges`.)
pub fn transitive_closure(rel: &BinaryRelation) -> BinaryRelation {
    let mut result = rel.clone();
    result.add_transitive_edges();
    result
}

/// Return true when `rel` is a strict partial order (irreflexive + transitive).
pub fn is_strict_partial_order(rel: &BinaryRelation) -> bool {
    rel.is_strict_partial_order()
}

/// Generate a BinaryRelation from an *eventually-follows* (EFG) matrix.
///
/// Adds edge i→j when (i,j) ∈ EFG and (j,i) ∉ EFG, then closes transitively.
/// This is the initial order construction used by the Maximal POWL variant:
/// `maximal_partial_order_cut.py:generate_initial_order`.
pub fn generate_initial_order_from_efg(
    n: usize,
    efg: &BinaryRelation,
) -> BinaryRelation {
    let mut order = BinaryRelation::new(n);
    for i in 0..n {
        for j in 0..n {
            if i != j && efg.is_edge(i, j) && !efg.is_edge(j, i) {
                order.add_edge(i, j);
            }
        }
    }
    order.add_transitive_edges();
    order
}

/// Check whether `order` is a valid partial order cut given the EFG.
///
/// A cut is valid when:
/// 1. `order` is a strict partial order (irreflexive + transitive).
/// 2. There are no bidirectional EFG edges (a,b) where order has a→b or b→a.
///
/// Mirrors the `is_valid_order` helper in the Python discovery variants.
pub fn is_valid_order(order: &BinaryRelation, efg: &BinaryRelation) -> bool {
    if !order.is_strict_partial_order() {
        return false;
    }
    let n = order.n;
    for i in 0..n {
        for j in 0..n {
            if i == j {
                continue;
            }
            // If both directions exist in EFG, they must not be ordered
            if efg.is_edge(i, j) && efg.is_edge(j, i) {
                if order.is_edge(i, j) || order.is_edge(j, i) {
                    return false;
                }
            }
        }
    }
    true
}

/// Merge two groups of node indices by joining them into a single group.
/// Used by the dynamic-clustering algorithm.
pub fn merge_groups(groups: &mut Vec<Vec<usize>>, a: usize, b: usize) {
    if a == b {
        return;
    }
    let (lo, hi) = if a < b { (a, b) } else { (b, a) };
    // Remove hi and append its contents to lo
    let hi_group = groups.remove(hi);
    groups[lo].extend(hi_group);
}

/// Build a BinaryRelation over clusters from per-node EFG.
///
/// Adds cluster-level edge src_cluster→tgt_cluster when ALL pairs
/// (i ∈ src, j ∈ tgt) have efg[i][j] = true AND none have efg[j][i] = true.
pub fn cluster_order_from_efg(
    clusters: &[Vec<usize>],
    efg: &BinaryRelation,
) -> BinaryRelation {
    let k = clusters.len();
    let mut order = BinaryRelation::new(k);
    'outer: for (ci, src) in clusters.iter().enumerate() {
        'inner: for (cj, tgt) in clusters.iter().enumerate() {
            if ci == cj {
                continue;
            }
            // Check all_forward: every (i,j) pair has efg[i][j]
            for &i in src {
                for &j in tgt {
                    if !efg.is_edge(i, j) {
                        continue 'inner;
                    }
                }
            }
            // Check none_backward: no (j,i) pair has efg[j][i]
            for &i in src {
                for &j in tgt {
                    if efg.is_edge(j, i) {
                        continue 'outer;
                    }
                }
            }
            order.add_edge(ci, cj);
        }
    }
    order
}

// ─── tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn chain(n: usize) -> BinaryRelation {
        let mut r = BinaryRelation::new(n);
        for i in 0..(n - 1) {
            r.add_edge(i, i + 1);
        }
        r
    }

    #[test]
    fn closure_of_chain() {
        let r = chain(4);
        let closed = transitive_closure(&r);
        // 0→1, 0→2, 0→3, 1→2, 1→3, 2→3
        assert!(closed.is_edge(0, 3));
        assert!(closed.is_transitive());
    }

    #[test]
    fn initial_order_from_efg() {
        let mut efg = BinaryRelation::new(3);
        efg.add_edge(0, 1); // 0 follows before 1 (not vice-versa)
        efg.add_edge(1, 2);
        let order = generate_initial_order_from_efg(3, &efg);
        assert!(order.is_edge(0, 1));
        assert!(order.is_edge(1, 2));
        assert!(order.is_edge(0, 2)); // transitive
        assert!(order.is_strict_partial_order());
    }

    #[test]
    fn valid_order_with_bidirectional_efg() {
        let mut order = BinaryRelation::new(3);
        order.add_edge(0, 1);
        order.add_edge(0, 2);
        order.add_edge(1, 2);
        let mut efg = BinaryRelation::new(3);
        efg.add_edge(0, 1);
        efg.add_edge(1, 2);
        efg.add_edge(0, 2);
        assert!(is_valid_order(&order, &efg));
    }

    #[test]
    fn invalid_order_bidirectional_efg_but_ordered() {
        let mut order = BinaryRelation::new(2);
        order.add_edge(0, 1);
        let mut efg = BinaryRelation::new(2);
        efg.add_edge(0, 1);
        efg.add_edge(1, 0); // bidirectional — node 0 and 1 should be concurrent
        assert!(!is_valid_order(&order, &efg));
    }

    #[test]
    fn cluster_order_basic() {
        // Two clusters: [0,1] → [2,3]
        // efg: every i in {0,1} → every j in {2,3}, no reverse
        let mut efg = BinaryRelation::new(4);
        for i in 0..2 {
            for j in 2..4 {
                efg.add_edge(i, j);
            }
        }
        let clusters = vec![vec![0, 1], vec![2, 3]];
        let order = cluster_order_from_efg(&clusters, &efg);
        assert!(order.is_edge(0, 1)); // cluster 0 → cluster 1
        assert!(!order.is_edge(1, 0));
    }
}
