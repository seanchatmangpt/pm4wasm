// PM4Py -- A Process Mining Library for Python (POWL v2 WASM)
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

/// ProcessTree -> BPMN 2.0 XML conversion.
///
/// Converts the inductive miner's process tree output directly to a valid
/// BPMN 2.0 document, without requiring an intermediate POWL representation.
///
/// Mapping:
///
/// | ProcessTree node       | BPMN element                                |
/// |------------------------|---------------------------------------------|
/// | leaf(label)            | `<task>`                                    |
/// | leaf(None) / tau       | `<serviceTask>` (silent, no label)          |
/// | Sequence(children)     | Chained sequence flows                      |
/// | Xor(children)          | Exclusive gateway split + join              |
/// | Parallel(children)     | Parallel gateway split + join               |
/// | Loop(do, redo)         | XOR gateway with back-arc on redo           |

use crate::process_tree::{PtOperator, ProcessTree};

struct Ids {
    counter: u32,
}

impl Ids {
    fn new() -> Self {
        Ids { counter: 0 }
    }
    fn next(&mut self, prefix: &str) -> String {
        self.counter += 1;
        format!("{}_{}", prefix, self.counter)
    }
}

struct Builder {
    ids: Ids,
    elements: Vec<String>,
    flows: Vec<String>,
    flow_counter: u32,
}

impl Builder {
    fn new() -> Self {
        Builder {
            ids: Ids::new(),
            elements: Vec::new(),
            flows: Vec::new(),
            flow_counter: 0,
        }
    }

    fn flow(&mut self, source: &str, target: &str) {
        self.flow_counter += 1;
        let id = format!("flow_{}", self.flow_counter);
        self.flows.push(format!(
            r#"    <sequenceFlow id="{}" sourceRef="{}" targetRef="{}"/>"#,
            id, source, target
        ));
    }

    /// Recursively convert a ProcessTree node.
    /// `entry`: incoming connection point id
    /// `exit`:  outgoing connection point id
    fn convert(&mut self, tree: &ProcessTree, entry: &str, exit: &str) {
        match &tree.operator {
            None => {
                // Leaf node
                if let Some(label) = &tree.label {
                    let id = self.ids.next("task");
                    let escaped = xml_escape(label);
                    self.elements.push(format!(
                        r#"    <task id="{}" name="{}"/>"#,
                        id, escaped
                    ));
                    self.flow(entry, &id);
                    self.flow(&id, exit);
                } else {
                    // tau -- silent service task
                    let id = self.ids.next("tau");
                    self.elements.push(format!(
                        r#"    <serviceTask id="{}" name="" pm4py:silent="true"/>"#,
                        id
                    ));
                    self.flow(entry, &id);
                    self.flow(&id, exit);
                }
            }
            Some(op) => match op {
                PtOperator::Sequence => {
                    // Chain children in sequence: entry -> c0 -> c1 -> ... -> exit
                    self.chain(tree, entry, exit);
                }
                PtOperator::Xor => {
                    let split = self.ids.next("xor_split");
                    let join = self.ids.next("xor_join");
                    self.elements.push(format!(
                        r#"    <exclusiveGateway id="{}" gatewayDirection="Diverging"/>"#,
                        split
                    ));
                    self.elements.push(format!(
                        r#"    <exclusiveGateway id="{}" gatewayDirection="Converging"/>"#,
                        join
                    ));
                    self.flow(entry, &split);
                    self.flow(&join, exit);

                    let split_c = split.clone();
                    let join_c = join.clone();
                    for child in &tree.children {
                        let child_entry = self.ids.next("p");
                        let child_exit = self.ids.next("p");
                        self.elements.push(format!(
                            r#"    <serviceTask id="{}" name="" pm4py:connector="true"/>"#,
                            child_entry
                        ));
                        self.elements.push(format!(
                            r#"    <serviceTask id="{}" name="" pm4py:connector="true"/>"#,
                            child_exit
                        ));
                        self.flow(&split_c, &child_entry);
                        self.flow(&child_exit, &join_c);
                        self.convert(child, &child_entry, &child_exit);
                    }
                }
                PtOperator::Parallel => {
                    let split = self.ids.next("and_split");
                    let join = self.ids.next("and_join");
                    self.elements.push(format!(
                        r#"    <parallelGateway id="{}" gatewayDirection="Diverging"/>"#,
                        split
                    ));
                    self.elements.push(format!(
                        r#"    <parallelGateway id="{}" gatewayDirection="Converging"/>"#,
                        join
                    ));
                    self.flow(entry, &split);
                    self.flow(&join, exit);

                    let split_c = split.clone();
                    let join_c = join.clone();
                    for child in &tree.children {
                        let child_entry = self.ids.next("p");
                        let child_exit = self.ids.next("p");
                        self.elements.push(format!(
                            r#"    <serviceTask id="{}" name="" pm4py:connector="true"/>"#,
                            child_entry
                        ));
                        self.elements.push(format!(
                            r#"    <serviceTask id="{}" name="" pm4py:connector="true"/>"#,
                            child_exit
                        ));
                        self.flow(&split_c, &child_entry);
                        self.flow(&child_exit, &join_c);
                        self.convert(child, &child_entry, &child_exit);
                    }
                }
                PtOperator::Loop => {
                    // Loop = *(do, redo)
                    // entry -> check_gw -> do_entry -> do -> do_exit -> decide_gw -> exit
                    //                                              |---- redo_entry -> redo -> redo_exit --|
                    let check = self.ids.next("loop_check");
                    let decide = self.ids.next("loop_decide");
                    self.elements.push(format!(
                        r#"    <exclusiveGateway id="{}" gatewayDirection="Converging"/>"#,
                        check
                    ));
                    self.elements.push(format!(
                        r#"    <exclusiveGateway id="{}" gatewayDirection="Diverging"/>"#,
                        decide
                    ));

                    let do_entry = self.ids.next("p");
                    let do_exit = self.ids.next("p");
                    self.elements.push(format!(
                        r#"    <serviceTask id="{}" name="" pm4py:connector="true"/>"#,
                        do_entry
                    ));
                    self.elements.push(format!(
                        r#"    <serviceTask id="{}" name="" pm4py:connector="true"/>"#,
                        do_exit
                    ));

                    self.flow(entry, &check);
                    self.flow(&check, &do_entry);
                    self.convert(&tree.children[0], &do_entry, &do_exit);
                    self.flow(&do_exit, &decide);
                    self.flow(&decide, exit); // exit branch

                    if tree.children.len() > 1 {
                        let redo_entry = self.ids.next("p");
                        let redo_exit = self.ids.next("p");
                        self.elements.push(format!(
                            r#"    <serviceTask id="{}" name="" pm4py:connector="true"/>"#,
                            redo_entry
                        ));
                        self.elements.push(format!(
                            r#"    <serviceTask id="{}" name="" pm4py:connector="true"/>"#,
                            redo_exit
                        ));
                        self.flow(&decide, &redo_entry); // redo branch
                        self.convert(&tree.children[1], &redo_entry, &redo_exit);
                        self.flow(&redo_exit, &check); // loop back
                    }
                }
            },
        }
    }

    /// Chain children in sequence: entry -> c0 -> c1 -> ... -> exit.
    fn chain(&mut self, tree: &ProcessTree, entry: &str, exit: &str) {
        if tree.children.is_empty() {
            let t = self.ids.next("tau");
            self.elements.push(format!(
                r#"    <serviceTask id="{}" name="" pm4py:silent="true"/>"#,
                t
            ));
            self.flow(entry, &t);
            self.flow(&t, exit);
            return;
        }

        let mut prev = entry.to_string();
        for (i, child) in tree.children.iter().enumerate() {
            let is_last = i == tree.children.len() - 1;
            let next = if is_last {
                exit.to_string()
            } else {
                let p = self.ids.next("p");
                self.elements.push(format!(
                    r#"    <serviceTask id="{}" name="" pm4py:connector="true"/>"#,
                    p
                ));
                p
            };
            self.convert(child, &prev, &next);
            prev = next;
        }
    }
}

fn xml_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

// -- Public API ----------------------------------------------------------------

/// Convert a [`ProcessTree`] to a BPMN 2.0 XML string.
///
/// The output is a complete `<definitions>` document compatible with
/// Camunda, bpmn.io, and Signavio.
pub fn process_tree_to_bpmn_xml(tree: &ProcessTree) -> String {
    let mut builder = Builder::new();

    let start_id = "startEvent_1".to_string();
    let end_id = "endEvent_1".to_string();

    // Top-level entry/exit connectors
    let proc_entry = builder.ids.next("p");
    let proc_exit = builder.ids.next("p");
    builder.elements.push(format!(
        r#"    <serviceTask id="{}" name="" pm4py:connector="true"/>"#,
        proc_entry
    ));
    builder.elements.push(format!(
        r#"    <serviceTask id="{}" name="" pm4py:connector="true"/>"#,
        proc_exit
    ));

    builder.flow(&start_id, &proc_entry);
    builder.convert(tree, &proc_entry, &proc_exit);
    builder.flow(&proc_exit, &end_id);

    let mut lines: Vec<String> = Vec::new();
    lines.push(r#"<?xml version="1.0" encoding="UTF-8"?>"#.to_string());
    lines.push(r#"<definitions"#.to_string());
    lines.push(r#"  xmlns="http://www.omg.org/spec/BPMN/20100524/MODEL""#.to_string());
    lines.push(r#"  xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance""#.to_string());
    lines.push(r#"  xmlns:pm4py="http://pm4py.org/bpmn-ext""#.to_string());
    lines.push(r#"  targetNamespace="http://pm4py.org/powl""#.to_string());
    lines.push(r#"  exporter="pm4py-pm4wasm""#.to_string());
    lines.push(r#"  exporterVersion="0.1.0">"#.to_string());
    lines.push(r#"  <process id="process_1" isExecutable="false">"#.to_string());
    lines.push(format!(r#"    <startEvent id="{}"/>"#, start_id));
    lines.push(format!(r#"    <endEvent id="{}"/>"#, end_id));
    for el in &builder.elements {
        lines.push(el.clone());
    }
    for fl in &builder.flows {
        lines.push(fl.clone());
    }
    lines.push("  </process>".to_string());
    lines.push("</definitions>".to_string());
    lines.join("\n")
}

// -- Tests ---------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn has_tag(xml: &str, tag: &str) -> bool {
        xml.contains(tag)
    }

    #[test]
    fn single_leaf_produces_valid_xml() {
        let tree = ProcessTree::leaf(Some("A".to_string()));
        let xml = process_tree_to_bpmn_xml(&tree);
        assert!(has_tag(&xml, "<definitions"));
        assert!(has_tag(&xml, "<process"));
        assert!(has_tag(&xml, r#"name="A""#));
        assert!(has_tag(&xml, "<startEvent"));
        assert!(has_tag(&xml, "<endEvent"));
        assert!(has_tag(&xml, "</definitions>"));
    }

    #[test]
    fn xor_produces_exclusive_gateways() {
        let tree = ProcessTree::internal(
            PtOperator::Xor,
            vec![
                ProcessTree::leaf(Some("A".to_string())),
                ProcessTree::leaf(Some("B".to_string())),
            ],
        );
        let xml = process_tree_to_bpmn_xml(&tree);
        assert!(has_tag(&xml, "exclusiveGateway"));
        assert!(has_tag(&xml, r#"name="A""#));
        assert!(has_tag(&xml, r#"name="B""#));
    }

    #[test]
    fn parallel_produces_parallel_gateways() {
        let tree = ProcessTree::internal(
            PtOperator::Parallel,
            vec![
                ProcessTree::leaf(Some("A".to_string())),
                ProcessTree::leaf(Some("B".to_string())),
            ],
        );
        let xml = process_tree_to_bpmn_xml(&tree);
        assert!(has_tag(&xml, "parallelGateway"));
        assert!(has_tag(&xml, r#"name="A""#));
        assert!(has_tag(&xml, r#"name="B""#));
    }

    #[test]
    fn loop_produces_exclusive_gateways() {
        let tree = ProcessTree::internal(
            PtOperator::Loop,
            vec![
                ProcessTree::leaf(Some("A".to_string())),
                ProcessTree::leaf(Some("B".to_string())),
            ],
        );
        let xml = process_tree_to_bpmn_xml(&tree);
        assert!(has_tag(&xml, "exclusiveGateway"));
        assert!(has_tag(&xml, r#"name="A""#));
        assert!(has_tag(&xml, r#"name="B""#));
    }

    #[test]
    fn sequence_chains_activities() {
        let tree = ProcessTree::internal(
            PtOperator::Sequence,
            vec![
                ProcessTree::leaf(Some("A".to_string())),
                ProcessTree::leaf(Some("B".to_string())),
                ProcessTree::leaf(Some("C".to_string())),
            ],
        );
        let xml = process_tree_to_bpmn_xml(&tree);
        assert!(has_tag(&xml, r#"name="A""#));
        assert!(has_tag(&xml, r#"name="B""#));
        assert!(has_tag(&xml, r#"name="C""#));
        // No gateways for pure sequence
        assert!(!has_tag(&xml, "Gateway"));
    }

    #[test]
    fn tau_leaf_produces_silent_task() {
        let tree = ProcessTree::leaf(None);
        let xml = process_tree_to_bpmn_xml(&tree);
        assert!(has_tag(&xml, "pm4py:silent"));
    }

    #[test]
    fn nested_operators() {
        // X(A, ->(B, C))
        let tree = ProcessTree::internal(
            PtOperator::Xor,
            vec![
                ProcessTree::leaf(Some("A".to_string())),
                ProcessTree::internal(
                    PtOperator::Sequence,
                    vec![
                        ProcessTree::leaf(Some("B".to_string())),
                        ProcessTree::leaf(Some("C".to_string())),
                    ],
                ),
            ],
        );
        let xml = process_tree_to_bpmn_xml(&tree);
        assert!(has_tag(&xml, "exclusiveGateway"));
        assert!(has_tag(&xml, r#"name="A""#));
        assert!(has_tag(&xml, r#"name="B""#));
        assert!(has_tag(&xml, r#"name="C""#));
    }
}
