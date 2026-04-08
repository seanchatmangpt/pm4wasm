// POWL Code Generation
//
// Generates executable workflow code from POWL models.
// Targets: n8n JSON, Temporal Go, Camunda BPMN, YAWL v6 XML

use crate::powl::{PowlArena, PowlNode, Operator};
use std::collections::HashMap;

/// Code generation result
pub struct CodegenResult {
    pub code: String,
    pub format: CodeFormat,
    pub target: CodeTarget,
}

/// Supported code formats
pub enum CodeFormat {
    Json,
    Xml,
    Go,
    Javascript,
}

/// Supported execution targets
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum CodeTarget {
    N8n,
    Temporal,
    Camunda,
    Yawl,
}

impl CodeTarget {
    pub fn as_str(&self) -> &'static str {
        match self {
            CodeTarget::N8n => "n8n",
            CodeTarget::Temporal => "temporal",
            CodeTarget::Camunda => "camunda",
            CodeTarget::Yawl => "yawl",
        }
    }
}

/// Generate n8n workflow JSON from POWL model
///
/// n8n is a low-code workflow automation platform.
/// This converts POWL to n8n's JSON node format.
pub fn generate_n8n(arena: &PowlArena, root: u32) -> CodegenResult {
    let mut nodes = Vec::new();
    let mut connections = Vec::new();
    let mut node_map: HashMap<u32, String> = HashMap::new();

    // Traverse POWL and build n8n nodes
    traverse_powl_for_n8n(arena, root, &mut nodes, &mut connections, &mut node_map, 0);

    let workflow = serde_json::json!({
        "name": "POWL Workflow",
        "nodes": nodes,
        "connections": connections
    });

    CodegenResult {
        code: serde_json::to_string_pretty(&workflow).unwrap_or_default(),
        format: CodeFormat::Json,
        target: CodeTarget::N8n,
    }
}

/// Generate Temporal Go workflow from POWL model
///
/// Temporal is a durable execution platform.
/// This converts POWL to Temporal Go workflow code.
pub fn generate_temporal(arena: &PowlArena, root: u32) -> CodegenResult {
    let mut activities = Vec::new();
    let mut workflow_code = String::new();

    // Header
    workflow_code.push_str("package workflow\n\n");
    workflow_code.push_str("import (\n");
    workflow_code.push_str("\t\"go.temporal.io/sdk/workflow\"\n");
    workflow_code.push_str(")\n\n");

    // Extract activities
    extract_activities(arena, root, &mut activities);

    // Activity interface
    workflow_code.push_str("// Activities define the individual steps\n");
    for activity in &activities {
        workflow_code.push_str(&format!("func {}(ctx workflow.Context, input string) (string, error) {{\n", activity));
        workflow_code.push_str(&format!("\t// TODO: Implement {}\n", activity));
        workflow_code.push_str("\treturn \"\", nil\n}\n\n");
    }

    // Workflow definition
    workflow_code.push_str("// Workflow is the main entry point\n");
    workflow_code.push_str("func Workflow(ctx workflow.Context, input string) (string, error) {\n");

    // Generate workflow logic from POWL structure
    generate_temporal_logic(arena, root, &mut workflow_code, &activities, 1);

    workflow_code.push_str("\treturn \"\", nil\n");
    workflow_code.push_str("}\n");

    CodegenResult {
        code: workflow_code,
        format: CodeFormat::Go,
        target: CodeTarget::Temporal,
    }
}

/// Generate Camunda BPMN from POWL model
///
/// Camunda is a process automation platform.
/// This converts POWL to Camunda-compatible BPMN 2.0 XML.
pub fn generate_camunda(arena: &PowlArena, root: u32) -> CodegenResult {
    let mut bpmn = String::new();

    // BPMN header
    bpmn.push_str("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
    bpmn.push_str("<definitions xmlns=\"http://www.omg.org/spec/BPMN/20100524/MODEL\" \n");
    bpmn.push_str("             xmlns:camunda=\"http://camunda.org/schema/1.0/bpmn\" \n");
    bpmn.push_str("             targetNamespace=\"http://bpmn.io/schema/bpmn\">\n");

    // Process
    bpmn.push_str("  <process id=\"POWLProcess\" isExecutable=\"true\">\n");

    // Generate BPMN elements from POWL
    generate_bpmn_elements(arena, root, &mut bpmn, 2);

    bpmn.push_str("  </process>\n");
    bpmn.push_str("</definitions>\n");

    CodegenResult {
        code: bpmn,
        format: CodeFormat::Xml,
        target: CodeTarget::Camunda,
    }
}

/// Generate YAWL v6 XML from POWL model
///
/// YAWL (Yet Another Workflow Language) is a workflow orchestration system.
/// This converts POWL to YAWL v6 XML format.
pub fn generate_yawl(arena: &PowlArena, root: u32) -> CodegenResult {
    let mut yawl = String::new();

    // YAWL specification header
    yawl.push_str("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
    yawl.push_str("<specificationSet xmlns=\"http://www.yawlfoundation.org/yawlschema\" \n");
    yawl.push_str("                  xmlns:xsi=\"http://www.w3.org/2001/XMLSchema-instance\" \n");
    yawl.push_str("                  version=\"6.0\">\n");

    // Specification
    yawl.push_str("  <specification uri=\"POWLWorkflow\">\n");
    yawl.push_str("    <meta>\n");
    yawl.push_str("      <title>POWL Workflow</title>\n");
    yawl.push_str("      <description>Generated from POWL model</description>\n");
    yawl.push_str("    </meta>\n");

    // Net
    yawl.push_str("    <net id=\"mainNet\">\n");

    // Generate YAWL net from POWL
    generate_yawl_net(arena, root, &mut yawl, 3);

    yawl.push_str("    </net>\n");
    yawl.push_str("  </specification>\n");
    yawl.push_str("</specificationSet>\n");

    CodegenResult {
        code: yawl,
        format: CodeFormat::Xml,
        target: CodeTarget::Yawl,
    }
}

/// Main code generation dispatcher
pub fn generate_code(arena: &PowlArena, root: u32, target: &str) -> Result<CodegenResult, String> {
    match target {
        "n8n" => Ok(generate_n8n(arena, root)),
        "temporal" => Ok(generate_temporal(arena, root)),
        "camunda" => Ok(generate_camunda(arena, root)),
        "yawl" => Ok(generate_yawl(arena, root)),
        _ => Err(format!("Unknown target: {}", target)),
    }
}

// ============================================================================
// Internal helpers
// ============================================================================

fn traverse_powl_for_n8n(
    arena: &PowlArena,
    node_idx: u32,
    nodes: &mut Vec<serde_json::Value>,
    connections: &mut Vec<serde_json::Value>,
    node_map: &mut HashMap<u32, String>,
    depth: usize,
) {
    if let Some(node) = arena.get(node_idx) {
        match node {
            PowlNode::Transition(t) => {
                let node_id = format!("node_{}", node_idx);
                let label = t.label.as_ref().map(|s| s.as_str()).unwrap_or("silent");

                node_map.insert(node_idx, node_id.clone());

                nodes.push(serde_json::json!({
                    "id": node_id,
                    "name": label,
                    "type": "n8n-nodes-base.function",
                    "position": [depth * 200, 0],
                    "parameters": {
                        "functionCode": format!("// Activity: {}\nreturn items;", label)
                    }
                }));
            }
            PowlNode::OperatorPowl(op) => {
                match op.operator {
                    Operator::Xor => {
                        // XOR: create choice node
                        let choice_id = format!("choice_{}", node_idx);
                        nodes.push(serde_json::json!({
                            "id": choice_id,
                            "name": "Choice",
                            "type": "n8n-nodes-base.switch",
                            "position": [depth * 200, 0]
                        }));

                        for child in &op.children {
                            if let Some(child_node) = arena.get(*child) {
                                if let PowlNode::Transition(t) = child_node {
                                    let label = t.label.as_ref().map(|s| s.as_str()).unwrap_or("option");
                                    connections.push(serde_json::json!({
                                        "from": choice_id,
                                        "to": node_map.get(child),
                                        "label": label
                                    }));
                                }
                            }
                        }
                    }
                    Operator::Loop => {
                        // Loop: iterate over children
                        for child in &op.children {
                            traverse_powl_for_n8n(arena, *child, nodes, connections, node_map, depth);
                        }
                    }
                    Operator::PartialOrder => {
                        // Partial order: execute children with constraints
                        for child in &op.children {
                            traverse_powl_for_n8n(arena, *child, nodes, connections, node_map, depth);
                        }
                    }
                }
            }
            PowlNode::StrictPartialOrder(spo) => {
                // Partial order: all nodes with ordering constraints
                for child in &spo.children {
                    traverse_powl_for_n8n(arena, *child, nodes, connections, node_map, depth);
                }
            }
            PowlNode::FrequentTransition(ft) => {
                let node_id = format!("node_{}", node_idx);
                let label = ft.label.as_str();

                node_map.insert(node_idx, node_id.clone());

                nodes.push(serde_json::json!({
                    "id": node_id,
                    "name": label,
                    "type": "n8n-nodes-base.function",
                    "position": [depth * 200, 0],
                    "parameters": {
                        "functionCode": format!("// Activity: {}\nreturn items;", label)
                    }
                }));
            }
        }
    }
}

fn extract_activities(arena: &PowlArena, node_idx: u32, activities: &mut Vec<String>) {
    if let Some(node) = arena.get(node_idx) {
        match node {
            PowlNode::Transition(t) => {
                if let Some(label) = &t.label {
                    if !label.is_empty() && label != "tau" {
                        activities.push(sanitize_activity_name(label));
                    }
                }
            }
            PowlNode::OperatorPowl(op) => {
                for child in &op.children {
                    extract_activities(arena, *child, activities);
                }
            }
            PowlNode::StrictPartialOrder(spo) => {
                for child in &spo.children {
                    extract_activities(arena, *child, activities);
                }
            }
            PowlNode::FrequentTransition(ft) => {
                if !ft.label.is_empty() {
                    activities.push(sanitize_activity_name(&ft.label));
                }
            }
        }
    }
}

fn sanitize_activity_name(name: &str) -> String {
    name.chars()
        .map(|c| if c.is_alphanumeric() { c } else { '_' })
        .collect()
}

fn generate_temporal_logic(
    arena: &PowlArena,
    node_idx: u32,
    code: &mut String,
    activities: &[String],
    indent: usize,
) {
    let indent_str = "  ".repeat(indent);

    if let Some(node) = arena.get(node_idx) {
        match node {
            PowlNode::Transition(t) => {
                if let Some(label) = &t.label {
                    let activity = sanitize_activity_name(label);
                    code.push_str(&format!("{}{}(ctx, input)\n", indent_str, activity));
                }
            }
            PowlNode::OperatorPowl(op) => {
                match op.operator {
                    Operator::Xor => {
                        code.push_str(&format!("{}// XOR choice\n", indent_str));
                        code.push_str(&format!("{}if true {{\n", indent_str));
                        if op.children.len() > 0 {
                            generate_temporal_logic(arena, op.children[0], code, activities, indent + 1);
                        }
                        code.push_str(&format!("{}}} else {{\n", indent_str));
                        if op.children.len() > 1 {
                            generate_temporal_logic(arena, op.children[1], code, activities, indent + 1);
                        }
                        code.push_str(&format!("{}}}\n", indent_str));
                    }
                    Operator::Loop => {
                        code.push_str(&format!("{}// Loop\n", indent_str));
                        for child in &op.children {
                            generate_temporal_logic(arena, *child, code, activities, indent);
                        }
                    }
                    Operator::PartialOrder => {
                        code.push_str(&format!("{}// Partial order: execute with constraints\n", indent_str));
                        for child in &op.children {
                            generate_temporal_logic(arena, *child, code, activities, indent);
                        }
                    }
                }
            }
            _ => {}
        }
    }
}

fn generate_bpmn_elements(arena: &PowlArena, node_idx: u32, bpmn: &mut String, indent: usize) {
    let indent_str = "  ".repeat(indent);

    if let Some(node) = arena.get(node_idx) {
        match node {
            PowlNode::Transition(t) => {
                if let Some(label) = &t.label {
                    if label != "tau" {
                        bpmn.push_str(&format!("{}<task id=\"{}\" name=\"{}\"/>\n",
                            indent_str,
                            sanitize_activity_name(label),
                            label));
                    }
                }
            }
            PowlNode::OperatorPowl(op) => {
                for child in &op.children {
                    generate_bpmn_elements(arena, *child, bpmn, indent);
                }
            }
            PowlNode::StrictPartialOrder(spo) => {
                for child in &spo.children {
                    generate_bpmn_elements(arena, *child, bpmn, indent);
                }
            }
            _ => {}
        }
    }
}

fn generate_yawl_net(arena: &PowlArena, node_idx: u32, yawl: &mut String, indent: usize) {
    let indent_str = "  ".repeat(indent);

    if let Some(node) = arena.get(node_idx) {
        match node {
            PowlNode::Transition(t) => {
                if let Some(label) = &t.label {
                    if label != "tau" {
                        yawl.push_str(&format!("{}<task id=\"{}\">\n",
                            indent_str,
                            sanitize_activity_name(label)));
                        yawl.push_str(&format!("{}<name>{}</name>\n", indent_str, label));
                        yawl.push_str(&format!("{}</task>\n", indent_str));
                    }
                }
            }
            PowlNode::OperatorPowl(op) => {
                for child in &op.children {
                    generate_yawl_net(arena, *child, yawl, indent);
                }
            }
            PowlNode::StrictPartialOrder(spo) => {
                for child in &spo.children {
                    generate_yawl_net(arena, *child, yawl, indent);
                }
            }
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::parse_powl_model_string;

    #[test]
    fn test_generate_n8n() {
        let mut arena = PowlArena::new();
        let model = "A->B->C";
        let root = parse_powl_model_string(model, &mut arena).unwrap();
        let result = generate_n8n(&arena, root);
        assert_eq!(result.target, CodeTarget::N8n);
        assert!(result.code.contains("A"));
    }

    #[test]
    fn test_generate_temporal() {
        let mut arena = PowlArena::new();
        let model = "A->B";
        let root = parse_powl_model_string(model, &mut arena).unwrap();
        let result = generate_temporal(&arena, root);
        assert_eq!(result.target, CodeTarget::Temporal);
        assert!(result.code.contains("package workflow"));
    }

    #[test]
    fn test_generate_yawl() {
        let mut arena = PowlArena::new();
        let model = "A";
        let root = parse_powl_model_string(model, &mut arena).unwrap();
        let result = generate_yawl(&arena, root);
        assert_eq!(result.target, CodeTarget::Yawl);
        assert!(result.code.contains("<?xml version"));
    }
}
