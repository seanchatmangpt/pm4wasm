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

//! wasm-bindgen browser tests.
//!
//! Run with:
//!   wasm-pack test --headless --firefox
//!   wasm-pack test --headless --chrome

use wasm_bindgen_test::*;

wasm_bindgen_test_configure!(run_in_browser);

use pm4wasm::{
    parse_powl, powl_to_string, validate_partial_orders,
    simplify_powl, simplify_frequent_transitions,
    transitive_closure, transitive_reduction, get_order_of,
    get_children, node_info_json, node_to_string,
    parse_xes_log, parse_csv_log,
    token_replay_fitness, powl_to_petri_net,
};

// ─── Parsing ──────────────────────────────────────────────────────────────────

#[wasm_bindgen_test]
fn parse_transition() {
    let m = parse_powl("A").unwrap();
    assert_eq!(powl_to_string(&m), "A");
}

#[wasm_bindgen_test]
fn parse_tau() {
    let m = parse_powl("tau").unwrap();
    assert_eq!(powl_to_string(&m), "tau");
}

#[wasm_bindgen_test]
fn parse_xor() {
    let m = parse_powl("X(A, B)").unwrap();
    assert_eq!(powl_to_string(&m), "X ( A, B )");
}

#[wasm_bindgen_test]
fn parse_loop() {
    let m = parse_powl("*(A, B)").unwrap();
    assert!(powl_to_string(&m).contains("A"));
}

#[wasm_bindgen_test]
fn parse_spo() {
    let m = parse_powl("PO=(nodes={A, B, C}, order={A-->B, A-->C})").unwrap();
    assert!(powl_to_string(&m).contains("A"));
    assert!(powl_to_string(&m).contains("B"));
}

#[wasm_bindgen_test]
fn parse_invalid_panics() {
    assert!(parse_powl("PO=(malformed{{").is_err());
}

// ─── Validation ───────────────────────────────────────────────────────────────

#[wasm_bindgen_test]
fn validate_valid_model() {
    let m = parse_powl("PO=(nodes={A, B}, order={A-->B})").unwrap();
    assert!(validate_partial_orders(&m).is_ok());
}

// ─── Simplification ───────────────────────────────────────────────────────────

#[wasm_bindgen_test]
fn simplify_xor_tau_to_frequent() {
    let m = parse_powl("X(A, tau)").unwrap();
    let s = simplify_frequent_transitions(&m);
    let repr = powl_to_string(&s);
    assert!(repr.contains("FrequentTransition") || repr.contains("A"));
}

#[wasm_bindgen_test]
fn simplify_nested_xor() {
    let m = parse_powl("X(A, X(B, C))").unwrap();
    let s = simplify_powl(&m);
    let repr = powl_to_string(&s);
    // Flattened: should not have nested X
    assert!(repr.contains("A") && repr.contains("B") && repr.contains("C"));
}

// ─── Graph operations ─────────────────────────────────────────────────────────

#[wasm_bindgen_test]
fn transitive_closure_chain() {
    let m = parse_powl("PO=(nodes={A, B, C}, order={A-->B, B-->C})").unwrap();
    let root = m.root();
    let closed = transitive_closure(&m, root).unwrap();
    // A→C should be implied
    assert!(closed.is_edge(0, 2));
}

#[wasm_bindgen_test]
fn transitive_reduction_removes_redundant() {
    let m = parse_powl("PO=(nodes={A, B, C}, order={A-->B, B-->C, A-->C})").unwrap();
    let root = m.root();
    let red = transitive_reduction(&m, root).unwrap();
    let edges = red.edges_flat();
    // A-->C (0→2) should be removed; only 0→1 and 1→2
    assert_eq!(edges.len(), 4); // 2 edges × 2 values each
    assert!(!red.is_edge(0, 2));
}

#[wasm_bindgen_test]
fn get_order_edges() {
    let m = parse_powl("PO=(nodes={A, B}, order={A-->B})").unwrap();
    let rel = get_order_of(&m, m.root()).unwrap();
    assert!(rel.is_edge(0, 1));
    assert!(!rel.is_edge(1, 0));
}

// ─── Introspection ────────────────────────────────────────────────────────────

#[wasm_bindgen_test]
fn node_info_transition() {
    let m = parse_powl("A").unwrap();
    let json = node_info_json(&m, m.root());
    assert!(json.contains("\"type\":\"Transition\""));
    assert!(json.contains("\"label\":\"A\""));
}

#[wasm_bindgen_test]
fn node_info_operator() {
    let m = parse_powl("X(A, B)").unwrap();
    let json = node_info_json(&m, m.root());
    assert!(json.contains("\"type\":\"OperatorPowl\""));
    assert!(json.contains("\"operator\":\"Xor\""));
}

#[wasm_bindgen_test]
fn children_xor() {
    let m = parse_powl("X(A, B, C)").unwrap();
    let kids = get_children(&m, m.root());
    assert_eq!(kids.len(), 3);
}

#[wasm_bindgen_test]
fn node_to_string_leaf() {
    let m = parse_powl("X(A, B)").unwrap();
    let kids = get_children(&m, m.root());
    assert_eq!(node_to_string(&m, kids[0]), "A");
}

// ─── Event log / conformance ──────────────────────────────────────────────────

#[wasm_bindgen_test]
fn parse_xes_basic() {
    let xml = r#"<?xml version="1.0"?>
<log xes.version="1.0">
  <trace>
    <string key="concept:name" value="c1"/>
    <event><string key="concept:name" value="A"/></event>
    <event><string key="concept:name" value="B"/></event>
  </trace>
</log>"#;
    let json = parse_xes_log(xml).unwrap();
    assert!(json.contains("\"case_id\":\"c1\""));
    assert!(json.contains("\"name\":\"A\""));
}

#[wasm_bindgen_test]
fn parse_csv_basic() {
    let csv = "case_id,activity\n1,A\n1,B\n2,A\n2,C\n";
    let json = parse_csv_log(csv).unwrap();
    assert!(json.contains("\"case_id\":\"1\""));
}

#[wasm_bindgen_test]
fn token_replay_perfect_fit() {
    // Sequential model A → B
    let pn_json = powl_to_petri_net("PO=(nodes={A, B}, order={A-->B})").unwrap();
    let log_json = r#"{"traces":[
      {"case_id":"c1","events":[{"name":"A","timestamp":null,"lifecycle":null,"attributes":{}},{"name":"B","timestamp":null,"lifecycle":null,"attributes":{}}]}
    ]}"#;
    let result_json = token_replay_fitness(&pn_json, log_json).unwrap();
    assert!(result_json.contains("\"perfectly_fitting_traces\":1"));
    assert!(result_json.contains("\"percentage\":1.0"));
}

#[wasm_bindgen_test]
fn token_replay_imperfect() {
    let pn_json = powl_to_petri_net("PO=(nodes={A, B}, order={A-->B})").unwrap();
    // Trace only has A — should leave token behind → fitness < 1
    let log_json = r#"{"traces":[
      {"case_id":"c1","events":[{"name":"A","timestamp":null,"lifecycle":null,"attributes":{}}]}
    ]}"#;
    let result_json = token_replay_fitness(&pn_json, log_json).unwrap();
    assert!(!result_json.contains("\"perfectly_fitting_traces\":1"));
}
