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

/// Process tree data model.
///
/// Mirrors `pm4py/objects/process_tree/obj.py` for the subset used by
/// the POWL → ProcessTree conversion.
use serde::{Deserialize, Serialize};

/// Operators supported in a process tree.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum PtOperator {
    /// Sequential composition (`→`)
    Sequence,
    /// Exclusive choice (`×`)
    Xor,
    /// Parallel execution (`∧`)
    Parallel,
    /// Loop (`↺` — do/redo)
    Loop,
}

impl PtOperator {
    pub fn as_str(self) -> &'static str {
        match self {
            PtOperator::Sequence => "->",
            PtOperator::Xor => "X",
            PtOperator::Parallel => "+",
            PtOperator::Loop => "*",
        }
    }
}

/// A node in a process tree.
///
/// Leaf nodes have a label (`Some(str)` for activities, `None` for tau).
/// Internal nodes have an operator and children (label is `None`).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ProcessTree {
    /// Activity label for leaf nodes; `None` for internal nodes and tau leaves.
    pub label: Option<String>,
    /// Operator for internal nodes; `None` for leaf nodes.
    pub operator: Option<PtOperator>,
    /// Children (empty for leaf nodes).
    pub children: Vec<ProcessTree>,
}

impl ProcessTree {
    /// Create a leaf node.
    pub fn leaf(label: Option<String>) -> Self {
        ProcessTree {
            label,
            operator: None,
            children: Vec::new(),
        }
    }

    /// Create an internal node.
    pub fn internal(operator: PtOperator, children: Vec<ProcessTree>) -> Self {
        ProcessTree {
            label: None,
            operator: Some(operator),
            children,
        }
    }

    /// Canonical string representation (mirrors Python __repr__).
    pub fn to_repr(&self) -> String {
        match (&self.operator, &self.label) {
            (None, None) => "tau".to_string(),
            (None, Some(l)) => l.clone(),
            (Some(op), _) => {
                let children: Vec<String> =
                    self.children.iter().map(|c| c.to_repr()).collect();
                format!("{} ( {} )", op.as_str(), children.join(", "))
            }
        }
    }
}
