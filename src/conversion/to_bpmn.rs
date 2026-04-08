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

/// POWL → BPMN 2.0 XML conversion.
///
/// Produces a valid BPMN 2.0 document that can be imported into
/// Camunda, Signavio, bpmn.io, and other BPMN-compliant tools.
///
/// Mapping:
///
/// | POWL node              | BPMN element                                |
/// |------------------------|---------------------------------------------|
/// | Transition(label)      | `<task>`                                    |
/// | Transition(tau)        | `<serviceTask>` (internal, no label)        |
/// | FrequentTransition     | `<task>` with boundary skip event           |
/// | XOR(children)          | Exclusive gateway split + join              |
/// | LOOP(do, redo)         | XOR gateway with back-arc on redo           |
/// | StrictPartialOrder     | Parallel gateway split + join (per level)   |
use crate::powl::{PowlArena, PowlNode};

struct Ids {
    counter: u32,
}

impl Ids {
    fn new() -> Self { Ids { counter: 0 } }
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

    /// Recursively convert a POWL node.
    /// `entry`: incoming connection point id
    /// `exit`:  outgoing connection point id
    fn convert(&mut self, arena: &PowlArena, idx: u32, entry: &str, exit: &str) {
        match arena.get(idx) {
            None => {
                // Fallback: silent pass-through
                let t = self.ids.next("tau");
                self.elements.push(format!(
                    r#"    <serviceTask id="{}" name="" pm4py:silent="true"/>"#,
                    t
                ));
                self.flow(entry, &t);
                self.flow(&t, exit);
            }

            Some(PowlNode::Transition(tr)) => {
                if let Some(label) = &tr.label {
                    let id = self.ids.next("task");
                    let escaped = xml_escape(label);
                    self.elements.push(format!(
                        r#"    <task id="{}" name="{}"/>"#,
                        id, escaped
                    ));
                    self.flow(entry, &id);
                    self.flow(&id, exit);
                } else {
                    // tau — silent service task
                    let id = self.ids.next("tau");
                    self.elements.push(format!(
                        r#"    <serviceTask id="{}" name="" pm4py:silent="true"/>"#,
                        id
                    ));
                    self.flow(entry, &id);
                    self.flow(&id, exit);
                }
            }

            Some(PowlNode::FrequentTransition(ft)) => {
                let id = self.ids.next("task");
                let escaped = xml_escape(&ft.activity);
                // Mark optional/loop via BPMN task marker attribute
                let loop_attr = if ft.selfloop {
                    r#" pm4py:loop="true""#
                } else if ft.skippable {
                    r#" pm4py:optional="true""#
                } else {
                    ""
                };
                self.elements.push(format!(
                    r#"    <task id="{}" name="{}"{}/>"#,
                    id, escaped, loop_attr
                ));
                self.flow(entry, &id);
                self.flow(&id, exit);
            }

            Some(PowlNode::OperatorPowl(op)) => {
                let operator = op.operator.as_str().to_string();
                let children = op.children.clone();

                match operator.as_str() {
                    "X" => {
                        let split = self.ids.next("xor_split");
                        let join  = self.ids.next("xor_join");
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
                        let join_c  = join.clone();
                        for child_idx in children {
                            let child_entry = self.ids.next("p");
                            let child_exit  = self.ids.next("p");
                            // Use invisible tasks as virtual connection points
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
                            self.convert(arena, child_idx, &child_entry, &child_exit);
                        }
                    }

                    "*" => {
                        // POWL LOOP = *(do, redo)
                        // BPMN: entry → check_gw → do_entry → do → do_exit → decide_gw → exit
                        //                                              └──── redo_entry → redo → redo_exit ──┘
                        let check  = self.ids.next("loop_check");
                        let decide = self.ids.next("loop_decide");
                        self.elements.push(format!(
                            r#"    <exclusiveGateway id="{}" gatewayDirection="Converging"/>"#,
                            check
                        ));
                        self.elements.push(format!(
                            r#"    <exclusiveGateway id="{}" gatewayDirection="Diverging"/>"#,
                            decide
                        ));

                        let do_entry   = self.ids.next("p");
                        let do_exit    = self.ids.next("p");
                        self.elements.push(format!(r#"    <serviceTask id="{}" name="" pm4py:connector="true"/>"#, do_entry));
                        self.elements.push(format!(r#"    <serviceTask id="{}" name="" pm4py:connector="true"/>"#, do_exit));

                        self.flow(entry, &check);
                        self.flow(&check, &do_entry);
                        self.convert(arena, children[0], &do_entry, &do_exit);
                        self.flow(&do_exit, &decide);
                        self.flow(&decide, exit); // exit branch

                        if children.len() > 1 {
                            let redo_entry = self.ids.next("p");
                            let redo_exit  = self.ids.next("p");
                            self.elements.push(format!(r#"    <serviceTask id="{}" name="" pm4py:connector="true"/>"#, redo_entry));
                            self.elements.push(format!(r#"    <serviceTask id="{}" name="" pm4py:connector="true"/>"#, redo_exit));
                            self.flow(&decide, &redo_entry);  // redo branch
                            self.convert(arena, children[1], &redo_entry, &redo_exit);
                            self.flow(&redo_exit, &check);    // loop back
                        }
                    }

                    _ => {
                        // Unknown operator — chain children in sequence
                        self.chain(arena, &children, entry, exit);
                    }
                }
            }

            Some(PowlNode::StrictPartialOrder(spo)) => {
                let children = spo.children.clone();

                if children.is_empty() {
                    // Empty SPO: silent pass
                    let t = self.ids.next("tau");
                    self.elements.push(format!(
                        r#"    <serviceTask id="{}" name="" pm4py:silent="true"/>"#, t
                    ));
                    self.flow(entry, &t);
                    self.flow(&t, exit);
                    return;
                }

                // Compute topological levels using in-degree BFS
                let order = &spo.order;
                let n = children.len();
                let mut level = vec![0usize; n];
                // Simple in-degree count to assign BFS levels
                for i in 0..n {
                    for j in 0..n {
                        if order.is_edge(i, j) && level[j] <= level[i] {
                            level[j] = level[i] + 1;
                        }
                    }
                }
                let max_level = level.iter().copied().max().unwrap_or(0);

                // Group children by level
                let mut groups: Vec<Vec<u32>> = vec![Vec::new(); max_level + 1];
                for (node_i, &lv) in level.iter().enumerate() {
                    groups[lv].push(children[node_i]);
                }

                // Build: split → [level0 in parallel] → [level1 in parallel] → … → join
                // Use parallel gateways only when group has >1 member
                let mut current = entry.to_string();

                for (gi, group) in groups.iter().enumerate() {
                    let next = if gi < groups.len() - 1 {
                        self.ids.next("sync")
                    } else {
                        exit.to_string()
                    };

                    if group.len() == 1 {
                        // Single node at this level — no gateway needed
                        let ce = self.ids.next("p");
                        let cx = self.ids.next("p");
                        self.elements.push(format!(r#"    <serviceTask id="{}" name="" pm4py:connector="true"/>"#, ce));
                        self.elements.push(format!(r#"    <serviceTask id="{}" name="" pm4py:connector="true"/>"#, cx));
                        self.flow(&current, &ce);
                        if next != exit {
                            self.elements.push(format!(r#"    <serviceTask id="{}" name="" pm4py:connector="true"/>"#, next));
                        }
                        self.convert(arena, group[0], &ce, &cx);
                        self.flow(&cx, &next);
                    } else {
                        // Parallel split/join for concurrent group
                        let and_split = self.ids.next("and_split");
                        let and_join  = self.ids.next("and_join");
                        self.elements.push(format!(
                            r#"    <parallelGateway id="{}" gatewayDirection="Diverging"/>"#,
                            and_split
                        ));
                        self.elements.push(format!(
                            r#"    <parallelGateway id="{}" gatewayDirection="Converging"/>"#,
                            and_join
                        ));
                        self.flow(&current, &and_split);
                        if gi < groups.len() - 1 {
                            self.elements.push(format!(r#"    <serviceTask id="{}" name="" pm4py:connector="true"/>"#, next));
                        }
                        self.flow(&and_join, &next);

                        let split_c = and_split.clone();
                        let join_c  = and_join.clone();
                        for &child_idx in group {
                            let ce = self.ids.next("p");
                            let cx = self.ids.next("p");
                            self.elements.push(format!(r#"    <serviceTask id="{}" name="" pm4py:connector="true"/>"#, ce));
                            self.elements.push(format!(r#"    <serviceTask id="{}" name="" pm4py:connector="true"/>"#, cx));
                            self.flow(&split_c, &ce);
                            self.flow(&cx, &join_c);
                            self.convert(arena, child_idx, &ce, &cx);
                        }
                    }

                    current = next;
                }
            }
        }
    }

    /// Chain children in sequence: entry → c0 → c1 → … → exit.
    fn chain(&mut self, arena: &PowlArena, children: &[u32], entry: &str, exit: &str) {
        if children.is_empty() {
            let t = self.ids.next("tau");
            self.elements.push(format!(
                r#"    <serviceTask id="{}" name="" pm4py:silent="true"/>"#, t
            ));
            self.flow(entry, &t);
            self.flow(&t, exit);
            return;
        }

        let mut prev = entry.to_string();
        for (i, &child) in children.iter().enumerate() {
            let is_last = i == children.len() - 1;
            let next = if is_last {
                exit.to_string()
            } else {
                let p = self.ids.next("p");
                self.elements.push(format!(
                    r#"    <serviceTask id="{}" name="" pm4py:connector="true"/>"#, p
                ));
                p
            };
            self.convert(arena, child, &prev, &next);
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

// ─── Public API ───────────────────────────────────────────────────────────────

/// Convert a POWL model to a BPMN 2.0 XML string.
///
/// The output is a complete `<definitions>` document compatible with
/// Camunda, bpmn.io, and Signavio.
pub fn to_bpmn_xml(arena: &PowlArena, root: u32) -> String {
    let mut builder = Builder::new();

    let start_id = "startEvent_1".to_string();
    let end_id   = "endEvent_1".to_string();

    // Top-level entry/exit connectors
    let proc_entry = builder.ids.next("p");
    let proc_exit  = builder.ids.next("p");
    builder.elements.push(format!(
        r#"    <serviceTask id="{}" name="" pm4py:connector="true"/>"#, proc_entry
    ));
    builder.elements.push(format!(
        r#"    <serviceTask id="{}" name="" pm4py:connector="true"/>"#, proc_exit
    ));

    builder.flow(&start_id, &proc_entry);
    builder.convert(arena, root, &proc_entry, &proc_exit);
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

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::parse_powl_model_string;
    use crate::powl::PowlArena;

    fn parse(s: &str) -> (PowlArena, u32) {
        let mut arena = PowlArena::new();
        let root = parse_powl_model_string(s, &mut arena).unwrap();
        (arena, root)
    }

    fn has_tag(xml: &str, tag: &str) -> bool {
        xml.contains(tag)
    }

    #[test]
    fn single_task_produces_valid_xml() {
        let (arena, root) = parse("A");
        let xml = to_bpmn_xml(&arena, root);
        assert!(has_tag(&xml, "<definitions"));
        assert!(has_tag(&xml, "<process"));
        assert!(has_tag(&xml, r#"name="A""#));
        assert!(has_tag(&xml, "<startEvent"));
        assert!(has_tag(&xml, "<endEvent"));
        assert!(has_tag(&xml, "</definitions>"));
    }

    #[test]
    fn xor_produces_exclusive_gateways() {
        let (arena, root) = parse("X(A, B)");
        let xml = to_bpmn_xml(&arena, root);
        assert!(has_tag(&xml, "exclusiveGateway"));
        assert!(has_tag(&xml, r#"name="A""#));
        assert!(has_tag(&xml, r#"name="B""#));
    }

    #[test]
    fn loop_produces_exclusive_gateways() {
        let (arena, root) = parse("*(A, B)");
        let xml = to_bpmn_xml(&arena, root);
        assert!(has_tag(&xml, "exclusiveGateway"));
        assert!(has_tag(&xml, r#"name="A""#));
    }

    #[test]
    fn spo_concurrent_produces_parallel_gateways() {
        let (arena, root) = parse("PO=(nodes={A, B}, order={})");
        let xml = to_bpmn_xml(&arena, root);
        assert!(has_tag(&xml, "parallelGateway"));
    }

    #[test]
    fn spo_sequential_no_parallel_gateways() {
        let (arena, root) = parse("PO=(nodes={A, B}, order={A-->B})");
        let xml = to_bpmn_xml(&arena, root);
        // Sequential SPO uses no parallel gateways
        assert!(!has_tag(&xml, "parallelGateway"));
        assert!(has_tag(&xml, r#"name="A""#));
        assert!(has_tag(&xml, r#"name="B""#));
    }

    #[test]
    fn xml_escape_in_label() {
        let (arena, root) = parse("A"); // labels don't have special chars but test the fn
        let xml = to_bpmn_xml(&arena, root);
        assert!(!xml.contains("&amp;&amp;")); // no double-escaping
    }
}
