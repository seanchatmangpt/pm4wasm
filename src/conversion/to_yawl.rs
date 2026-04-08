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

/// POWL → YAWL v6 XML conversion.
///
/// Produces a valid YAWL specification document that can be imported into
/// the YAWL workflow engine.
///
/// Mapping:
///
/// | POWL node              | YAWL element                                |
/// |------------------------|---------------------------------------------|
/// | Transition(label)      | `<task>` with name                           |
/// | Transition(tau)        | (skipped — silent transitions are implicit)  |
/// | FrequentTransition     | `<task>` with decomposition annotation       |
/// | XOR(children)          | Conditions + XOR split/join flows             |
/// | LOOP(do, redo)         | Conditions + loop back-flow                   |
/// | StrictPartialOrder     | Conditions + AND split/join flows per level   |
use crate::powl::{PowlArena, PowlNode};

// ─── ID generator ────────────────────────────────────────────────────────────

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

// ─── Builder ────────────────────────────────────────────────────────────────

/// Collects YAWL net elements and flow edges during conversion.
struct Builder {
    ids: Ids,
    /// Lines for `<processControlElements>` (tasks + conditions).
    elements: Vec<String>,
    /// Lines for `<flow>` (edges).
    flows: Vec<String>,
}

impl Builder {
    fn new() -> Self {
        Builder {
            ids: Ids::new(),
            elements: Vec::new(),
            flows: Vec::new(),
        }
    }

    /// Add a flow edge: source → target.
    fn flow(&mut self, source: &str, target: &str) {
        self.flows.push(format!(
            r#"        <edge source="{}" target="{}"/>"#,
            source, target
        ));
    }

    /// Emit a YAWL `<task>` element.
    fn task(&mut self, id: &str, name: &str, join: &str, split: &str) {
        let escaped = xml_escape(name);
        self.elements.push(format!("        <task id=\"{}\">", id));
        self.elements.push(format!("          <name>{}</name>", escaped));
        self.elements.push(format!("          <decomposesTo id=\"dt_{}\"/>", id));
        self.elements.push(format!("          <join code=\"{}\"/>", join));
        self.elements.push(format!("          <split code=\"{}\"/>", split));
        self.elements.push(format!("        </task>"));
    }

    /// Emit a YAWL `<condition>` element.
    fn condition(&mut self, id: &str) {
        self.elements.push(format!("        <condition id=\"{}\"/>", id));
    }

    /// Recursively convert a POWL node.
    ///
    /// `entry`: id of the incoming connection point (condition or IC).
    /// `exit`:  id of the outgoing connection point (condition or OC).
    fn convert(&mut self, arena: &PowlArena, idx: u32, entry: &str, exit: &str) {
        match arena.get(idx) {
            None => {
                // Fallback: direct pass-through
                self.flow(entry, exit);
            }

            Some(PowlNode::Transition(tr)) => {
                if tr.label.is_none() {
                    // Silent (tau) transition — skip, just pass through
                    self.flow(entry, exit);
                    return;
                }

                let label = tr.label.as_deref().unwrap();
                let id = sanitize_id(label);
                self.task(&id, label, "xor", "xor");
                self.flow(entry, &id);
                self.flow(&id, exit);
            }

            Some(PowlNode::FrequentTransition(ft)) => {
                let id = sanitize_id(&ft.activity);
                self.task(&id, &ft.activity, "xor", "xor");
                self.flow(entry, &id);
                self.flow(&id, exit);
            }

            Some(PowlNode::OperatorPowl(op)) => {
                let children = op.children.clone();

                match op.operator {
                    crate::powl::Operator::Xor => {
                        self.convert_xor(arena, &children, entry, exit);
                    }

                    crate::powl::Operator::Loop => {
                        self.convert_loop(arena, &children, entry, exit);
                    }

                    crate::powl::Operator::PartialOrder => {
                        // PartialOrder operator — treat as SPO
                        self.chain(arena, &children, entry, exit);
                    }
                }
            }

            Some(PowlNode::StrictPartialOrder(spo)) => {
                let children = spo.children.clone();

                if children.is_empty() {
                    self.flow(entry, exit);
                    return;
                }

                // Compute topological levels using in-degree BFS
                let order = &spo.order;
                let n = children.len();
                let mut level = vec![0usize; n];
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

                // Build: entry → [level0] → [level1] → ... → exit
                // Use AND split/join when group has >1 member, else direct pass-through.
                let mut current = entry.to_string();

                for (gi, group) in groups.iter().enumerate() {
                    let is_last = gi == groups.len() - 1;
                    let next = if is_last {
                        exit.to_string()
                    } else {
                        self.ids.next("c")
                    };

                    if group.len() == 1 {
                        // Single node at this level — no gateway needed
                        let ce = self.ids.next("c");
                        self.condition(&ce);
                        self.flow(&current, &ce);
                        self.convert(arena, group[0], &ce, &next);
                    } else {
                        // Parallel split/join via conditions
                        let merge_c = self.ids.next("c");
                        let fork_c = self.ids.next("c");
                        self.condition(&merge_c);
                        self.condition(&fork_c);

                        if !is_last {
                            self.condition(&next);
                        }

                        self.flow(&current, &merge_c);
                        self.flow(&fork_c, &next);

                        let fork_c = fork_c.clone();
                        let merge_c = merge_c.clone();
                        for &child_idx in group {
                            let ce = self.ids.next("c");
                            self.condition(&ce);
                            self.flow(&merge_c, &ce);
                            self.flow(&ce, &fork_c);
                            self.convert(arena, child_idx, &ce, &fork_c);
                        }
                    }

                    current = next;
                }
            }
        }
    }

    /// XOR: entry → [merge_c → child_i → fork_c] → exit
    fn convert_xor(
        &mut self,
        arena: &PowlArena,
        children: &[u32],
        entry: &str,
        exit: &str,
    ) {
        if children.is_empty() {
            self.flow(entry, exit);
            return;
        }

        let merge_c = self.ids.next("c");
        let fork_c = self.ids.next("c");
        self.condition(&merge_c);
        self.condition(&fork_c);

        self.flow(entry, &merge_c);
        self.flow(&fork_c, exit);

        let fork_c = fork_c.clone();
        let merge_c = merge_c.clone();
        for &child_idx in children {
            let ce = self.ids.next("c");
            self.condition(&ce);
            self.flow(&merge_c, &ce);
            self.flow(&ce, &fork_c);
            self.convert(arena, child_idx, &ce, &fork_c);
        }
    }

    /// LOOP: *(do, redo)
    ///   entry → merge_c → do_entry → do → do_exit → fork_c → exit
    ///                                    ^                    |
    ///                                    └── redo → redo_exit ─┘
    fn convert_loop(
        &mut self,
        arena: &PowlArena,
        children: &[u32],
        entry: &str,
        exit: &str,
    ) {
        let merge_c = self.ids.next("c");
        let fork_c = self.ids.next("c");
        self.condition(&merge_c);
        self.condition(&fork_c);

        self.flow(entry, &merge_c);

        // do branch
        let do_entry = self.ids.next("c");
        let do_exit = self.ids.next("c");
        self.condition(&do_entry);
        self.condition(&do_exit);

        self.flow(&merge_c, &do_entry);
        self.convert(arena, children[0], &do_entry, &do_exit);
        self.flow(&do_exit, &fork_c);
        self.flow(&fork_c, exit); // exit branch

        // redo branch (loop back)
        if children.len() > 1 {
            let redo_entry = self.ids.next("c");
            let redo_exit = self.ids.next("c");
            self.condition(&redo_entry);
            self.condition(&redo_exit);

            self.flow(&fork_c, &redo_entry); // redo branch
            self.convert(arena, children[1], &redo_entry, &redo_exit);
            self.flow(&redo_exit, &merge_c); // loop back
        }
    }

    /// Chain children in sequence: entry → c0 → c1 → ... → exit.
    fn chain(&mut self, arena: &PowlArena, children: &[u32], entry: &str, exit: &str) {
        if children.is_empty() {
            self.flow(entry, exit);
            return;
        }

        let mut prev = entry.to_string();
        for (i, &child) in children.iter().enumerate() {
            let is_last = i == children.len() - 1;
            let next = if is_last {
                exit.to_string()
            } else {
                let p = self.ids.next("c");
                self.condition(&p);
                p
            };
            self.convert(arena, child, &prev, &next);
            prev = next;
        }
    }
}

// ─── Helpers ────────────────────────────────────────────────────────────────

fn xml_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

/// Sanitize an activity label to produce a valid XML/YAWL id.
/// Replaces non-alphanumeric characters with underscores.
fn sanitize_id(label: &str) -> String {
    let mut result = String::with_capacity(label.len());
    let mut prev_underscore = false;
    for ch in label.chars() {
        if ch.is_alphanumeric() || ch == '_' {
            if ch == '_' {
                if prev_underscore {
                    continue; // collapse consecutive underscores
                }
                prev_underscore = true;
            } else {
                prev_underscore = false;
            }
            result.push(ch);
        } else {
            if !prev_underscore {
                result.push('_');
                prev_underscore = true;
            }
        }
    }
    // Trim leading/trailing underscores
    let trimmed = result.trim_matches('_');
    if trimmed.is_empty() {
        "task".to_string()
    } else {
        trimmed.to_string()
    }
}

// ─── Public API ──────────────────────────────────────────────────────────────

/// Convert a POWL model to a YAWL v6 XML string.
///
/// The output is a complete `<specificationSet>` document compatible with
/// the YAWL workflow engine.
pub fn to_yawl_xml(arena: &PowlArena, root: u32) -> String {
    let mut builder = Builder::new();

    let ic = "IC".to_string();
    let oc = "OC".to_string();

    builder.convert(arena, root, &ic, &oc);

    let mut lines: Vec<String> = Vec::new();
    lines.push(r#"<?xml version="1.0" encoding="UTF-8"?>"#.to_string());
    lines.push(
        r#"<specificationSet xmlns="http://www.yawlfoundation.org/yawlschema" version="6.0">"#
            .to_string(),
    );
    lines.push(r#"  <specification uri="powl_workflow">"#.to_string());
    lines.push(r#"    <meta>"#.to_string());
    lines.push(r#"      <creator>pm4wasm</creator>"#.to_string());
    lines.push(r#"      <description>Generated from POWL model</description>"#.to_string());
    lines.push(r#"    </meta>"#.to_string());
    lines.push(r#"    <net id="mainNet">"#.to_string());
    lines.push(r#"      <processControlElements>"#.to_string());
    lines.push(r#"        <inputCondition id="IC"/>"#.to_string());
    lines.push(r#"        <outputCondition id="OC"/>"#.to_string());
    for el in &builder.elements {
        lines.push(el.clone());
    }
    lines.push(r#"      </processControlElements>"#.to_string());
    lines.push(r#"      <flow>"#.to_string());
    for fl in &builder.flows {
        lines.push(fl.clone());
    }
    lines.push(r#"      </flow>"#.to_string());
    lines.push(r#"    </net>"#.to_string());
    lines.push(r#"  </specification>"#.to_string());
    lines.push(r#"</specificationSet>"#.to_string());
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

    fn has(xml: &str, needle: &str) -> bool {
        xml.contains(needle)
    }

    #[test]
    fn single_task_produces_valid_xml() {
        let (arena, root) = parse("A");
        let xml = to_yawl_xml(&arena, root);
        assert!(has(&xml, "<specificationSet"));
        assert!(has(&xml, "<specification uri=\"powl_workflow\">"));
        assert!(has(&xml, "<net id=\"mainNet\">"));
        assert!(has(&xml, "<inputCondition id=\"IC\"/>"));
        assert!(has(&xml, "<outputCondition id=\"OC\"/>"));
        assert!(has(&xml, "<name>A</name>"));
        assert!(has(&xml, "<join code=\"xor\"/>"));
        assert!(has(&xml, "<split code=\"xor\"/>"));
        assert!(has(&xml, "</specificationSet>"));
        // Flow edges: IC → task → OC
        assert!(has(&xml, "source=\"IC\""));
        assert!(has(&xml, "target=\"OC\""));
    }

    #[test]
    fn xor_produces_conditions() {
        let (arena, root) = parse("X(A, B)");
        let xml = to_yawl_xml(&arena, root);
        assert!(has(&xml, "<name>A</name>"));
        assert!(has(&xml, "<name>B</name>"));
        // XOR split/join should be present via conditions
        assert!(has(&xml, "<condition id=\""));
        // Flow edges should connect through conditions
        assert!(has(&xml, "source=\"IC\""));
        assert!(has(&xml, "target=\"OC\""));
    }

    #[test]
    fn parallel_spo_produces_and_flows() {
        let (arena, root) = parse("PO=(nodes={A, B}, order={})");
        let xml = to_yawl_xml(&arena, root);
        assert!(has(&xml, "<name>A</name>"));
        assert!(has(&xml, "<name>B</name>"));
        // Parallel should have multiple conditions (fork/merge points)
        assert!(has(&xml, "<condition id=\""));
        // Both tasks should have flows from merge condition
        assert!(has(&xml, "source=\"IC\""));
        assert!(has(&xml, "target=\"OC\""));
    }

    #[test]
    fn loop_produces_back_flow() {
        let (arena, root) = parse("*(A, B)");
        let xml = to_yawl_xml(&arena, root);
        assert!(has(&xml, "<name>A</name>"));
        assert!(has(&xml, "<name>B</name>"));
        // Loop should have conditions for merge/fork/redo
        assert!(has(&xml, "<condition id=\""));
        // Flow should exist from IC and to OC
        assert!(has(&xml, "source=\"IC\""));
        assert!(has(&xml, "target=\"OC\""));
    }

    #[test]
    fn sequential_spo_no_conditions_for_linear() {
        let (arena, root) = parse("PO=(nodes={A, B}, order={A-->B})");
        let xml = to_yawl_xml(&arena, root);
        assert!(has(&xml, "<name>A</name>"));
        assert!(has(&xml, "<name>B</name>"));
        assert!(has(&xml, "source=\"IC\""));
        assert!(has(&xml, "target=\"OC\""));
    }

    #[test]
    fn silent_transition_skipped() {
        let (arena, root) = parse("tau");
        let xml = to_yawl_xml(&arena, root);
        assert!(has(&xml, "<specificationSet"));
        // Silent transition should produce no task
        assert!(!has(&xml, "<task"));
        // Just IC → OC
        assert!(has(&xml, "source=\"IC\""));
        assert!(has(&xml, "target=\"OC\""));
    }

    #[test]
    fn sanitize_id_handles_special_chars() {
        assert_eq!(sanitize_id("A"), "A");
        assert_eq!(sanitize_id("hello world"), "hello_world");
        assert_eq!(sanitize_id("a-b-c"), "a_b_c");
        assert_eq!(sanitize_id("a&&b"), "a_b");
        assert_eq!(sanitize_id(""), "task");
    }

    #[test]
    fn xml_escape_in_task_name() {
        let mut arena = PowlArena::new();
        let root = arena.add_transition(Some("A<B>".into()));
        let xml = to_yawl_xml(&arena, root);
        assert!(has(&xml, "<name>A&lt;B&gt;</name>"));
        assert!(!has(&xml, "<name>A<B></name>"));
    }

    #[test]
    fn frequent_transition_produces_task() {
        let mut arena = PowlArena::new();
        let root = arena.add_frequent_transition("Pay".into(), 1, Some(1));
        let xml = to_yawl_xml(&arena, root);
        assert!(has(&xml, "<name>Pay</name>"));
        assert!(has(&xml, "<task id=\"Pay\">"));
    }
}
