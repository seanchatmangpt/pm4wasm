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

/// PTML (Process Tree Markup Language) import/export.
///
/// PTML is an XML format for process trees used by ProM and pm4py.
use crate::process_tree::{PtOperator, ProcessTree};
use quick_xml::events::Event as XmlEvent;
use quick_xml::reader::Reader;
use wasm_bindgen::prelude::*;

/// Serialize a ProcessTree to PTML XML format.
///
/// The PTML format represents process trees as nested XML elements
/// with operator types as tag names and activity labels as text content.
pub fn to_ptml(tree: &ProcessTree) -> String {
    let mut xml = String::from("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
    xml.push_str("<ptml>\n");
    write_tree_node(&mut xml, tree, 1);
    xml.push_str("</ptml>\n");
    xml
}

fn write_tree_node(xml: &mut String, tree: &ProcessTree, indent: usize) {
    let pad = "  ".repeat(indent);

    match (&tree.operator, &tree.label) {
        (Some(op), _) => {
            let tag = match op {
                PtOperator::Sequence => "sequence",
                PtOperator::Xor => "xor",
                PtOperator::Parallel => "parallel",
                PtOperator::Loop => "loop",
            };
            xml.push_str(&format!("{}<{}>\n", pad, tag));
            for child in &tree.children {
                write_tree_node(xml, child, indent + 1);
            }
            xml.push_str(&format!("{}</{}>\n", pad, tag));
        }
        (None, Some(label)) => {
            xml.push_str(&format!("{}<activity label=\"{}\"/>\n", pad, label));
        }
        (None, None) => {
            // Silent/tau transition
            xml.push_str(&format!("{}<tau/>\n", pad));
        }
    }
}

/// Parse a PTML XML string into a ProcessTree.
pub fn from_ptml(xml: &str) -> Result<ProcessTree, String> {
    let mut reader = Reader::from_str(xml);
    reader.config_mut().trim_text(true);

    let mut stack: Vec<ProcessTree> = Vec::new();

    loop {
        match reader.read_event().map_err(|e| e.to_string())? {
            XmlEvent::Start(ref e) => {
                let tag = e.name();
                let tag_str = String::from_utf8_lossy(tag.as_ref()).to_string();

                // Extract label from attributes
                let label: Option<String> = e
                    .attributes()
                    .flatten()
                    .find(|a| a.key.as_ref() == b"label")
                    .map(|a| String::from_utf8_lossy(&a.value).to_string());

                let node = match tag_str.as_str() {
                    "sequence" => Some(ProcessTree::internal(PtOperator::Sequence, vec![])),
                    "xor" => Some(ProcessTree::internal(PtOperator::Xor, vec![])),
                    "parallel" => Some(ProcessTree::internal(PtOperator::Parallel, vec![])),
                    "loop" => Some(ProcessTree::internal(PtOperator::Loop, vec![])),
                    "activity" => Some(ProcessTree::leaf(label)),
                    "tau" => Some(ProcessTree::leaf(None)),
                    _ => None,
                };

                if let Some(n) = node {
                    stack.push(n);
                }
            }

            XmlEvent::Empty(ref e) => {
                let tag = e.name();
                let tag_str = String::from_utf8_lossy(tag.as_ref()).to_string();

                let label: Option<String> = e
                    .attributes()
                    .flatten()
                    .find(|a| a.key.as_ref() == b"label")
                    .map(|a| String::from_utf8_lossy(&a.value).to_string());

                let node = match tag_str.as_str() {
                    "activity" => ProcessTree::leaf(label),
                    "tau" => ProcessTree::leaf(None),
                    _ => continue,
                };

                // Attach to parent
                if let Some(parent) = stack.last_mut() {
                    parent.children.push(node);
                } else {
                    // Root is a leaf
                    stack.push(node);
                }
            }

            XmlEvent::End(ref e) => {
                let tag = e.name();
                let tag_str = String::from_utf8_lossy(tag.as_ref()).to_string();

                // Check if this is closing an operator
                let is_operator = matches!(
                    tag_str.as_str(),
                    "sequence" | "xor" | "parallel" | "loop"
                );

                if is_operator {
                    // Pop completed node and attach to parent
                    if let Some(completed) = stack.pop() {
                        if let Some(parent) = stack.last_mut() {
                            parent.children.push(completed);
                        } else {
                            // This is the root - push it back
                            stack.push(completed);
                        }
                    }
                }
            }

            XmlEvent::Eof => break,
            _ => {}
        }
    }

    // Return the root (should be only element left)
    stack
        .pop()
        .ok_or_else(|| "Empty PTML: no process tree found".to_string())
}

/// WASM export: ProcessTree JSON to PTML.
#[wasm_bindgen]
pub fn to_ptml_json(tree_json: &str) -> Result<String, JsValue> {
    let tree: ProcessTree = serde_json::from_str(tree_json)
        .map_err(|e| JsValue::from_str(&format!("Failed to parse ProcessTree JSON: {}", e)))?;
    Ok(to_ptml(&tree))
}

/// WASM export: PTML to ProcessTree JSON.
#[wasm_bindgen]
pub fn from_ptml_string(xml: &str) -> Result<String, JsValue> {
    let tree = from_ptml(xml).map_err(|e| JsValue::from_str(&e))?;
    serde_json::to_string(&tree)
        .map_err(|e| JsValue::from_str(&format!("Serialization error: {}", e)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_to_ptml_sequence() {
        let tree = ProcessTree::internal(
            PtOperator::Sequence,
            vec![
                ProcessTree::leaf(Some("A".to_string())),
                ProcessTree::leaf(Some("B".to_string())),
            ],
        );
        let ptml = to_ptml(&tree);
        assert!(ptml.contains("<sequence>"));
        assert!(ptml.contains("<activity label=\"A\""));
        assert!(ptml.contains("<activity label=\"B\""));
        assert!(ptml.contains("</sequence>"));
    }

    #[test]
    fn test_ptml_roundtrip() {
        let original = ProcessTree::internal(
            PtOperator::Xor,
            vec![
                ProcessTree::leaf(Some("A".to_string())),
                ProcessTree::internal(
                    PtOperator::Parallel,
                    vec![
                        ProcessTree::leaf(Some("B".to_string())),
                        ProcessTree::leaf(Some("C".to_string())),
                    ],
                ),
                ProcessTree::leaf(None), // tau
            ],
        );
        let ptml = to_ptml(&original);
        let restored = from_ptml(&ptml).unwrap();

        // Verify structure by checking repr
        assert_eq!(original.to_repr(), restored.to_repr());
    }

    #[test]
    fn test_from_ptml_invalid() {
        let result = from_ptml("not xml");
        assert!(result.is_err());
    }

    #[test]
    fn test_to_ptml_tau() {
        let tree = ProcessTree::internal(
            PtOperator::Sequence,
            vec![
                ProcessTree::leaf(Some("A".to_string())),
                ProcessTree::leaf(None),
                ProcessTree::leaf(Some("B".to_string())),
            ],
        );
        let ptml = to_ptml(&tree);
        assert!(ptml.contains("<tau/>"));
    }
}
