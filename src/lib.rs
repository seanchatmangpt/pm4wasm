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

//! # pm4wasm
//!
//! WebAssembly port of POWL v2 (Partially Ordered Workflow Language) from pm4py.
//!
//! ## Quick start (JavaScript / browser)
//!
//! ```js
//! import init, {
//!   parse_powl,
//!   validate_partial_orders,
//!   powl_to_string,
//!   simplify_powl,
//!   simplify_frequent_transitions,
//!   transitive_reduction,
//!   transitive_closure,
//!   is_strict_partial_order,
//! } from './pkg/pm4wasm.js';
//!
//! await init();
//!
//! // Parse a POWL model string (same format as Python __repr__)
//! const model = parse_powl("PO=(nodes={A, B, C}, order={A-->B, A-->C})");
//!
//! // Validate
//! validate_partial_orders(model);   // throws on violation
//!
//! // Serialise back
//! console.log(powl_to_string(model));
//!
//! // Graph ops on a binary relation
//! const rel = transitive_closure(model, 0);  // root SPO's order, closed
//! const red = transitive_reduction(rel);
//! ```
//!
//! ## Architecture
//!
//! All nodes are stored in a flat [`PowlModel`] arena (a thin wasm-bindgen
//! wrapper around [`PowlArena`]).  Nodes are referenced by their `u32` index.
//! The root of the parsed tree is always the *last* node added (index
//! `model.len() - 1`).

use wasm_bindgen::prelude::*;

mod binary_relation;
pub mod powl;
pub mod parser;
pub mod algorithms;
pub mod petri_net;
pub mod process_tree;
pub mod footprints;
pub mod conversion;
pub mod event_log;
pub mod conformance;
pub mod complexity;
pub mod diff;
pub mod streaming;
pub mod statistics;
pub mod discovery;
pub mod filtering;
pub mod llm;
pub mod quality;
pub mod simulation;
pub mod transformation;
pub mod trie;
pub mod ocel;

use binary_relation::BinaryRelation;
use powl::{PowlArena, PowlNode};
use parser::parse_powl_model_string;
use algorithms::simplify as simplify_algo;
use algorithms::transitive as transitive_algo;
use event_log::EventLog;
use petri_net::PetriNetResult;
use footprints::Footprints;

// ─── JS-visible wrapper types ────────────────────────────────────────────────

/// Flat arena holding the entire POWL model tree.
///
/// Construct with [`parse_powl`].  The root node is at index `model.root()`.
#[wasm_bindgen]
pub struct PowlModel {
    arena: PowlArena,
    root: u32,
}

#[wasm_bindgen]
impl PowlModel {
    /// Index of the root node.
    pub fn root(&self) -> u32 {
        self.root
    }

    /// Total number of nodes in the arena.
    pub fn len(&self) -> usize {
        self.arena.len()
    }

    pub fn is_empty(&self) -> bool {
        self.arena.is_empty()
    }
}

/// A serialisable binary relation (adjacency matrix) exposed to JavaScript.
///
/// Construct via [`transitive_closure`], [`transitive_reduction`], or
/// [`get_order_of`].
#[wasm_bindgen]
pub struct BinaryRelationJs {
    inner: BinaryRelation,
}

#[wasm_bindgen]
impl BinaryRelationJs {
    /// Number of nodes.
    pub fn n(&self) -> usize {
        self.inner.n
    }

    /// Test whether edge i→j exists.
    pub fn is_edge(&self, i: usize, j: usize) -> bool {
        self.inner.is_edge(i, j)
    }

    /// Return all edges as a flat `[src0, tgt0, src1, tgt1, …]` array.
    pub fn edges_flat(&self) -> Vec<u32> {
        self.inner
            .edge_list()
            .into_iter()
            .flat_map(|(s, t)| [s as u32, t as u32])
            .collect()
    }

    pub fn is_irreflexive(&self) -> bool {
        self.inner.is_irreflexive()
    }

    pub fn is_transitive(&self) -> bool {
        self.inner.is_transitive()
    }

    pub fn is_strict_partial_order(&self) -> bool {
        self.inner.is_strict_partial_order()
    }

    /// Nodes with no incoming edges.
    pub fn start_nodes(&self) -> Vec<u32> {
        self.inner
            .get_start_nodes()
            .into_iter()
            .map(|x| x as u32)
            .collect()
    }

    /// Nodes with no outgoing edges.
    pub fn end_nodes(&self) -> Vec<u32> {
        self.inner
            .get_end_nodes()
            .into_iter()
            .map(|x| x as u32)
            .collect()
    }
}

// ─── Public WASM API ─────────────────────────────────────────────────────────

/// Parse a POWL model string (the same format as the Python `__repr__`) and
/// return an opaque [`PowlModel`] handle.
///
/// # Errors
/// Throws a JavaScript `Error` if parsing fails.
#[wasm_bindgen]
pub fn parse_powl(s: &str) -> Result<PowlModel, JsValue> {
    let mut arena = PowlArena::new();
    let root = parse_powl_model_string(s, &mut arena)
        .map_err(|e| JsValue::from_str(&format!("POWL parse error: {}", e)))?;
    Ok(PowlModel { arena, root })
}

/// Validate that all `StrictPartialOrder` nodes in `model` have irreflexive
/// and transitive ordering relations.
///
/// # Errors
/// Throws a JavaScript `Error` describing the first violation found.
#[wasm_bindgen]
pub fn validate_partial_orders(model: &PowlModel) -> Result<(), JsValue> {
    model
        .arena
        .validate_partial_orders(model.root)
        .map_err(|e| JsValue::from_str(&e))
}

/// Return the string representation of the model root (mirrors Python `__repr__`).
#[wasm_bindgen]
pub fn powl_to_string(model: &PowlModel) -> String {
    model.arena.to_repr(model.root)
}

/// Recursively simplify the model (merge XOR+LOOP patterns, flatten nested
/// XORs, inline sub-SPOs where possible).  Returns a new [`PowlModel`].
#[wasm_bindgen]
pub fn simplify_powl(model: &PowlModel) -> PowlModel {
    let mut arena = model.arena.clone();
    let new_root = simplify_algo::simplify(&mut arena, model.root);
    PowlModel { arena, root: new_root }
}

/// Convert `XOR(A, tau)` / `LOOP(A, tau)` patterns to `FrequentTransition`
/// nodes.  Returns a new [`PowlModel`].
#[wasm_bindgen]
pub fn simplify_frequent_transitions(model: &PowlModel) -> PowlModel {
    let mut arena = model.arena.clone();
    let new_root =
        simplify_algo::simplify_using_frequent_transitions(&mut arena, model.root);
    PowlModel { arena, root: new_root }
}

/// Return the transitive closure of the ordering relation of a
/// `StrictPartialOrder` node.
///
/// `spo_arena_idx` is the arena index of the SPO node (use `model.root()` for
/// the root, or another index for a nested SPO).
///
/// # Errors
/// Throws if `spo_arena_idx` does not point to a `StrictPartialOrder`.
#[wasm_bindgen]
pub fn transitive_closure(
    model: &PowlModel,
    spo_arena_idx: u32,
) -> Result<BinaryRelationJs, JsValue> {
    match model.arena.get(spo_arena_idx) {
        Some(PowlNode::StrictPartialOrder(spo)) => {
            let closed = transitive_algo::transitive_closure(&spo.order);
            Ok(BinaryRelationJs { inner: closed })
        }
        _ => Err(JsValue::from_str(&format!(
            "node {} is not a StrictPartialOrder",
            spo_arena_idx
        ))),
    }
}

/// Return the transitive reduction of the ordering relation of a
/// `StrictPartialOrder` node.
///
/// # Errors
/// Throws if the node is not an SPO or the relation is not irreflexive.
#[wasm_bindgen]
pub fn transitive_reduction(
    model: &PowlModel,
    spo_arena_idx: u32,
) -> Result<BinaryRelationJs, JsValue> {
    match model.arena.get(spo_arena_idx) {
        Some(PowlNode::StrictPartialOrder(spo)) => {
            let red = spo.order.get_transitive_reduction();
            Ok(BinaryRelationJs { inner: red })
        }
        _ => Err(JsValue::from_str(&format!(
            "node {} is not a StrictPartialOrder",
            spo_arena_idx
        ))),
    }
}

/// Return the raw ordering relation of a `StrictPartialOrder` node as a
/// [`BinaryRelationJs`].
///
/// # Errors
/// Throws if `spo_arena_idx` does not point to a `StrictPartialOrder`.
#[wasm_bindgen]
pub fn get_order_of(
    model: &PowlModel,
    spo_arena_idx: u32,
) -> Result<BinaryRelationJs, JsValue> {
    match model.arena.get(spo_arena_idx) {
        Some(PowlNode::StrictPartialOrder(spo)) => {
            Ok(BinaryRelationJs { inner: spo.order.clone() })
        }
        _ => Err(JsValue::from_str(&format!(
            "node {} is not a StrictPartialOrder",
            spo_arena_idx
        ))),
    }
}

/// Return the string representation of an individual node by arena index.
#[wasm_bindgen]
pub fn node_to_string(model: &PowlModel, arena_idx: u32) -> String {
    model.arena.to_repr(arena_idx)
}

/// Return the child arena indices of an SPO or OperatorPOWL node as a flat
/// `u32` array.  Returns an empty array for leaf nodes.
#[wasm_bindgen]
pub fn get_children(model: &PowlModel, arena_idx: u32) -> Vec<u32> {
    match model.arena.get(arena_idx) {
        Some(PowlNode::StrictPartialOrder(spo)) => spo.children.clone(),
        Some(PowlNode::OperatorPowl(op)) => op.children.clone(),
        _ => Vec::new(),
    }
}

/// Return a JSON string describing the node at `arena_idx`.
///
/// Format: `{"type":"Transition","label":"A"}` or
///         `{"type":"StrictPartialOrder","children":[0,1],"edges":[[0,1]]}`
#[wasm_bindgen]
pub fn node_info_json(model: &PowlModel, arena_idx: u32) -> String {
    match model.arena.get(arena_idx) {
        None => r#"{"type":"Invalid"}"#.to_string(),
        Some(PowlNode::Transition(t)) => {
            let label = t.label.as_deref().unwrap_or("tau");
            format!(r#"{{"type":"Transition","label":"{}","id":{}}}"#, label, t.id)
        }
        Some(PowlNode::FrequentTransition(t)) => {
            format!(
                r#"{{"type":"FrequentTransition","label":"{}","activity":"{}","skippable":{},"selfloop":{}}}"#,
                t.label, t.activity, t.skippable, t.selfloop
            )
        }
        Some(PowlNode::StrictPartialOrder(spo)) => {
            let children_json: Vec<String> =
                spo.children.iter().map(|c| c.to_string()).collect();
            let edges: Vec<String> = spo
                .order
                .edge_list()
                .iter()
                .map(|(s, t)| format!("[{},{}]", s, t))
                .collect();
            format!(
                r#"{{"type":"StrictPartialOrder","children":[{}],"edges":[{}]}}"#,
                children_json.join(","),
                edges.join(",")
            )
        }
        Some(PowlNode::OperatorPowl(op)) => {
            let op_str = op.operator.as_str();
            let children_json: Vec<String> =
                op.children.iter().map(|c| c.to_string()).collect();
            format!(
                r#"{{"type":"OperatorPowl","operator":"{}","children":[{}]}}"#,
                op_str,
                children_json.join(",")
            )
        }
    }
}

// ─── Event log API ───────────────────────────────────────────────────────────

/// Parse a XES-formatted XML string and return the event log as a JSON string.
///
/// # Errors
/// Throws a JavaScript `Error` on parse failure.
#[wasm_bindgen]
pub fn parse_xes_log(xml: &str) -> Result<String, JsValue> {
    let log = event_log::parse_xes(xml)
        .map_err(|e| JsValue::from_str(&format!("XES parse error: {}", e)))?;
    serde_json::to_string(&log)
        .map_err(|e| JsValue::from_str(&format!("JSON serialisation error: {}", e)))
}

/// Parse a CSV string (with headers) and return the event log as a JSON string.
///
/// Required columns: `case_id` / `case:concept:name`, `concept:name` / `activity`.
/// Optional: `time:timestamp` / `timestamp`.
///
/// # Errors
/// Throws a JavaScript `Error` on parse failure.
#[wasm_bindgen]
pub fn parse_csv_log(csv: &str) -> Result<String, JsValue> {
    let log = event_log::parse_csv(csv)
        .map_err(|e| JsValue::from_str(&format!("CSV parse error: {}", e)))?;
    serde_json::to_string(&log)
        .map_err(|e| JsValue::from_str(&format!("JSON serialisation error: {}", e)))
}

// ─── Conformance API ──────────────────────────────────────────────────────────

/// Compute token-replay fitness of `log_json` (output of [`parse_xes_log`] /
/// [`parse_csv_log`]) against `petri_net_json` (output of [`powl_to_petri_net`]).
///
/// Returns a JSON string with shape:
/// ```json
/// {
///   "percentage": 0.97,
///   "avg_trace_fitness": 0.96,
///   "perfectly_fitting_traces": 42,
///   "total_traces": 44,
///   "trace_results": [...]
/// }
/// ```
///
/// # Errors
/// Throws if either JSON input cannot be deserialised.
#[wasm_bindgen]
pub fn token_replay_fitness(petri_net_json: &str, log_json: &str) -> Result<String, JsValue> {
    let pn_result: PetriNetResult = serde_json::from_str(petri_net_json)
        .map_err(|e| JsValue::from_str(&format!("Petri net JSON error: {}", e)))?;
    let log: EventLog = serde_json::from_str(log_json)
        .map_err(|e| JsValue::from_str(&format!("Event log JSON error: {}", e)))?;
    let result = conformance::token_replay::compute_fitness(
        &pn_result.net,
        &pn_result.initial_marking,
        &pn_result.final_marking,
        &log,
    );
    serde_json::to_string(&result)
        .map_err(|e| JsValue::from_str(&format!("JSON serialisation error: {}", e)))
}

/// Compute A* alignment for a single trace against a Petri net.
///
/// Returns JSON with `cost`, `is_fit`, `moves` (each with `type`, `label`, `cost`).
/// Move types: "sync" (model+log match), "log" (log only), "model" (model only).
///
/// Mirrors `pm4py.align_trace()`.
///
/// # Errors
/// Throws if JSON cannot be deserialised.
#[wasm_bindgen]
pub fn align_trace(petri_net_json: &str, trace_json: &str) -> Result<String, JsValue> {
    let pn_result: PetriNetResult = serde_json::from_str(petri_net_json)
        .map_err(|e| JsValue::from_str(&format!("Petri net JSON error: {}", e)))?;
    let trace: event_log::Trace = serde_json::from_str(trace_json)
        .map_err(|e| JsValue::from_str(&format!("Trace JSON error: {}", e)))?;
    let result = conformance::alignments::astar::align_trace(
        &pn_result.net,
        &pn_result.initial_marking,
        &pn_result.final_marking,
        &trace,
    );
    serde_json::to_string(&result)
        .map_err(|e| JsValue::from_str(&format!("JSON serialisation error: {}", e)))
}

/// Compute A* alignments for every trace in an event log against a Petri net.
///
/// Returns JSON with `total_cost`, `avg_cost`, `trace_alignments` (per-trace breakdown).
///
/// Mirrors `pm4py.align_log()`.
///
/// # Errors
/// Throws if JSON cannot be deserialised.
#[wasm_bindgen]
pub fn align_log(petri_net_json: &str, log_json: &str) -> Result<String, JsValue> {
    let pn_result: PetriNetResult = serde_json::from_str(petri_net_json)
        .map_err(|e| JsValue::from_str(&format!("Petri net JSON error: {}", e)))?;
    let log: EventLog = serde_json::from_str(log_json)
        .map_err(|e| JsValue::from_str(&format!("Event log JSON error: {}", e)))?;
    let result = conformance::alignments::astar::align_log(
        &pn_result.net,
        &pn_result.initial_marking,
        &pn_result.final_marking,
        &log,
    );
    serde_json::to_string(&result)
        .map_err(|e| JsValue::from_str(&format!("JSON serialisation error: {}", e)))
}

/// Import a PNML 2.0 XML string into a PetriNetResult JSON.
///
/// Mirrors `pm4py.read_pnml()`.
///
/// # Errors
/// Throws if the PNML XML cannot be parsed.
#[wasm_bindgen]
pub fn from_pnml(xml: &str) -> Result<String, JsValue> {
    let result = conversion::pnml::from_pnml(xml)
        .map_err(|e| JsValue::from_str(&format!("PNML parse error: {}", e)))?;
    serde_json::to_string(&result)
        .map_err(|e| JsValue::from_str(&format!("JSON serialisation error: {}", e)))
}

/// Compute ETConformance precision of `log_json` against `petri_net_json`.
///
/// Returns JSON:
/// ```json
/// {
///   "precision": 0.85,
///   "total_escaping": 5,
///   "total_consumed": 100,
///   "total_traces": 10
/// }
/// ```
///
/// Precision measures how precisely the model describes observed behavior.
/// A score of 1.0 means the model allows exactly the behavior seen in the log.
/// A lower score means the model permits transitions that were never used
/// (escaping edges).
///
/// Mirrors `pm4py.precision_etconformance()`.
///
/// # Errors
/// Throws if either JSON input cannot be deserialised.
#[wasm_bindgen]
pub fn precision_etconformance(petri_net_json: &str, log_json: &str) -> Result<String, JsValue> {
    let pn_result: PetriNetResult = serde_json::from_str(petri_net_json)
        .map_err(|e| JsValue::from_str(&format!("Petri net JSON error: {}", e)))?;
    let log: EventLog = serde_json::from_str(log_json)
        .map_err(|e| JsValue::from_str(&format!("Event log JSON error: {}", e)))?;
    let result = conformance::precision::compute_precision(
        &pn_result.net,
        &pn_result.initial_marking,
        &pn_result.final_marking,
        &log,
    );
    serde_json::to_string(&result)
        .map_err(|e| JsValue::from_str(&format!("JSON serialisation error: {}", e)))
}

/// Compute complexity metrics for a POWL model and return JSON.
///
/// Returns a JSON object with `cyclomatic`, `cfc`, `cognitive`, `nesting_depth`,
/// `branching_factor`, `activity_count`, `node_count`, and `halstead` fields.
///
/// # Errors
/// Throws if `model` is empty.
#[wasm_bindgen]
pub fn measure_complexity(model: &PowlModel) -> Result<String, JsValue> {
    let report = complexity::measure(&model.arena, model.root);
    serde_json::to_string(&report)
        .map_err(|e| JsValue::from_str(&format!("JSON error: {}", e)))
}

/// Compute simplicity metric for a Petri net.
///
/// Simplicity measures how "simple" a model is based on its structure.
/// Uses the arc_degree variant: 1 - (arcs / (places * transitions)).
/// Returns a value in [0.0, 1.0] where 1.0 is simplest.
///
/// Mirrors `pm4py.analysis.simplicity_petri_net()` with variant="arc_degree".
///
/// # Errors
/// Throws if JSON cannot be deserialised.
#[wasm_bindgen]
pub fn simplicity_petri_net(petri_net_json: &str) -> Result<f64, JsValue> {
    let pn: PetriNetResult = serde_json::from_str(petri_net_json)
        .map_err(|e| JsValue::from_str(&format!("PetriNetResult JSON error: {}", e)))?;
    let num_places = pn.net.places.len();
    let num_transitions = pn.net.transitions.len();
    let num_arcs = pn.net.arcs.len();
    Ok(complexity::simplicity_arc_degree(num_places, num_transitions, num_arcs))
}

/// Compute the structural and behavioural diff between two POWL model strings.
///
/// Returns a JSON object describing added/removed activities, ordering changes,
/// structural operator changes, and an overall severity level.
///
/// # Errors
/// Throws if either string fails to parse.
#[wasm_bindgen]
pub fn diff_models(model_a: &str, model_b: &str) -> Result<String, JsValue> {
    let mut arena_a = PowlArena::new();
    let root_a = parse_powl_model_string(model_a, &mut arena_a)
        .map_err(|e| JsValue::from_str(&format!("Model A parse error: {}", e)))?;
    let mut arena_b = PowlArena::new();
    let root_b = parse_powl_model_string(model_b, &mut arena_b)
        .map_err(|e| JsValue::from_str(&format!("Model B parse error: {}", e)))?;
    let d = diff::diff(&arena_a, root_a, &arena_b, root_b);
    serde_json::to_string(&d)
        .map_err(|e| JsValue::from_str(&format!("JSON error: {}", e)))
}

/// Convert a POWL model string to BPMN 2.0 XML.
///
/// Returns a complete `<definitions>` XML document importable by Camunda,
/// bpmn.io, Signavio, and other BPMN-compliant tools.
///
/// # Errors
/// Throws on parse failure.
#[wasm_bindgen]
pub fn powl_to_bpmn(s: &str) -> Result<String, JsValue> {
    let mut arena = PowlArena::new();
    let root = parse_powl_model_string(s, &mut arena)
        .map_err(|e| JsValue::from_str(&format!("POWL parse error: {}", e)))?;
    Ok(conversion::to_bpmn::to_bpmn_xml(&arena, root))
}

/// Parse a BPMN 2.0 XML string and convert to a POWL model string.
///
/// Returns a POWL model string that can be parsed with `parse_powl`.
/// Handles both pm4wasm-generated BPMN (with `pm4py:connector`/`pm4py:silent`
/// markers) and generic BPMN from external tools (Camunda, bpmn.io, Signavio).
///
/// Mirrors `pm4py.read_bpmn()`.
///
/// # Errors
/// Throws on parse failure or invalid BPMN structure.
#[wasm_bindgen]
pub fn read_bpmn(bpmn_xml: &str) -> Result<String, JsValue> {
    conversion::from_bpmn::bpmn_to_powl_string(bpmn_xml)
        .map_err(|e| JsValue::from_str(&format!("BPMN parse error: {}", e)))
}

/// Convert a POWL model string to YAWL v6 XML.
///
/// Returns a complete YAWL specification document importable by the
/// YAWL workflow engine.
///
/// # Errors
/// Throws on parse failure.
#[wasm_bindgen]
pub fn powl_to_yawl(s: &str) -> Result<String, JsValue> {
    let mut arena = PowlArena::new();
    let root = parse_powl_model_string(s, &mut arena)
        .map_err(|e| JsValue::from_str(&format!("POWL parse error: {}", e)))?;
    Ok(conversion::to_yawl::to_yawl_xml(&arena, root))
}

/// Convert a POWL model to Petri net and return the result as a JSON string.
///
/// Convenience wrapper combining `parse_powl` + conversion in one call.
///
/// # Errors
/// Throws on parse or conversion failure.
#[wasm_bindgen]
pub fn powl_to_petri_net(s: &str) -> Result<String, JsValue> {
    let mut arena = PowlArena::new();
    let root = parse_powl_model_string(s, &mut arena)
        .map_err(|e| JsValue::from_str(&format!("POWL parse error: {}", e)))?;
    let model = PowlModel { arena, root };
    let result = conversion::to_petri_net::apply(&model.arena, model.root);
    serde_json::to_string(&result)
        .map_err(|e| JsValue::from_str(&format!("JSON serialisation error: {}", e)))
}

/// Convert a PetriNetResult (JSON) to PNML 2.0 XML format.
///
/// Takes the JSON output from `powl_to_petri_net` or `discover_petri_net_inductive`
/// and converts it to PNML XML for import into tools like PNEditor, WoPeD, or ProM.
///
/// Mirrors `pm4py.write_pnml()`.
///
/// # Errors
/// Throws if the PetriNetResult JSON cannot be parsed.
#[wasm_bindgen]
pub fn to_pnml(petri_net_json: &str) -> Result<String, JsValue> {
    let pn: PetriNetResult = serde_json::from_str(petri_net_json)
        .map_err(|e| JsValue::from_str(&format!("PetriNetResult JSON error: {}", e)))?;
    Ok(conversion::pnml::to_pnml(&pn))
}

/// Convert a POWL model string to a process tree.
///
/// Returns JSON with `label`, `operator`, and `children` fields.
/// Leaf nodes have `label` set; internal nodes have `operator` set
/// to one of: "->" (sequence), "X" (xor), "+" (parallel), "*" (loop).
///
/// Mirrors `pm4py.convert_to_process_tree()`.
///
/// # Errors
/// Throws on parse failure.
#[wasm_bindgen]
pub fn powl_to_process_tree(powl_string: &str) -> Result<String, JsValue> {
    let mut arena = PowlArena::new();
    let root = parse_powl_model_string(powl_string, &mut arena)
        .map_err(|e| JsValue::from_str(&format!("POWL parse error: {}", e)))?;
    let result = conversion::to_process_tree::apply(&arena, root);
    serde_json::to_string(&result)
        .map_err(|e| JsValue::from_str(&format!("JSON serialisation error: {}", e)))
}

/// Convert a process tree (JSON) to a POWL model string.
///
/// Takes the JSON output from `powl_to_process_tree` and converts it
/// back to a POWL model string that can be parsed with `parse_powl`.
///
/// Note: Sequence and Parallel operators are converted to StrictPartialOrder.
///
/// Mirrors `pm4py.convert_to_powl()` from process tree.
///
/// # Errors
/// Throws if JSON cannot be deserialised.
#[wasm_bindgen]
pub fn process_tree_to_powl(process_tree_json: &str) -> Result<String, JsValue> {
    use crate::powl::Operator;
    let pt: crate::process_tree::ProcessTree = serde_json::from_str(process_tree_json)
        .map_err(|e| JsValue::from_str(&format!("Process tree JSON error: {}", e)))?;

    fn build_powl(pt: &crate::process_tree::ProcessTree, arena: &mut PowlArena) -> u32 {
        match &pt.operator {
            None => {
                // Leaf node
                let label = pt.label.as_deref();
                if let Some(l) = label {
                    if l.is_empty() || l == "tau" {
                        arena.add_transition(None)
                    } else {
                        arena.add_transition(Some(l.to_string()))
                    }
                } else {
                    arena.add_transition(None)
                }
            }
            Some(op) => {
                let children: Vec<u32> = pt.children.iter()
                    .map(|c| build_powl(c, arena))
                    .collect();

                match op {
                    crate::process_tree::PtOperator::Xor => {
                        // XOR is directly supported
                        arena.add_operator(Operator::Xor, children)
                    }
                    crate::process_tree::PtOperator::Loop => {
                        // Loop is directly supported
                        arena.add_operator(Operator::Loop, children)
                    }
                    crate::process_tree::PtOperator::Sequence => {
                        // Sequence → StrictPartialOrder with chain order
                        let spo_idx = arena.add_strict_partial_order(children.clone());
                        for i in 0..children.len().saturating_sub(1) {
                            arena.add_order_edge(spo_idx, i, i + 1);
                        }
                        // Add transitive edges
                        for i in 0..children.len() {
                            for j in (i + 2)..children.len() {
                                arena.add_order_edge(spo_idx, i, j);
                            }
                        }
                        spo_idx
                    }
                    crate::process_tree::PtOperator::Parallel => {
                        // Parallel → StrictPartialOrder with no order
                        arena.add_strict_partial_order(children)
                    }
                }
            }
        }
    }

    let mut arena = PowlArena::new();
    let root = build_powl(&pt, &mut arena);
    Ok(arena.to_repr(root))
}

// ─── Statistics API ──────────────────────────────────────────────────────────────

/// Get start activities with frequencies from an event log JSON.
///
/// Returns JSON: `[{"activity":"A","count":3}, ...]`
///
/// Mirrors `pm4py.get_start_activities()`.
#[wasm_bindgen]
pub fn get_start_activities(log_json: &str) -> Result<String, JsValue> {
    let log: EventLog = serde_json::from_str(log_json)
        .map_err(|e| JsValue::from_str(&format!("Event log JSON error: {}", e)))?;
    let result = statistics::basic::get_start_activities(&log);
    serde_json::to_string(&result)
        .map_err(|e| JsValue::from_str(&format!("JSON error: {}", e)))
}

/// Get end activities with frequencies from an event log JSON.
///
/// Returns JSON: `[{"activity":"C","count":2}, ...]`
///
/// Mirrors `pm4py.get_end_activities()`.
#[wasm_bindgen]
pub fn get_end_activities(log_json: &str) -> Result<String, JsValue> {
    let log: EventLog = serde_json::from_str(log_json)
        .map_err(|e| JsValue::from_str(&format!("Event log JSON error: {}", e)))?;
    let result = statistics::basic::get_end_activities(&log);
    serde_json::to_string(&result)
        .map_err(|e| JsValue::from_str(&format!("JSON error: {}", e)))
}

/// Get all variants (activity sequences) with frequencies and percentages.
///
/// Returns JSON: `[{"activities":["A","B"],"count":1,"percentage":33.3}, ...]`
///
/// Mirrors `pm4py.get_variants()`.
#[wasm_bindgen]
pub fn get_variants(log_json: &str) -> Result<String, JsValue> {
    let log: EventLog = serde_json::from_str(log_json)
        .map_err(|e| JsValue::from_str(&format!("Event log JSON error: {}", e)))?;
    let result = statistics::basic::get_variants(&log);
    serde_json::to_string(&result)
        .map_err(|e| JsValue::from_str(&format!("JSON error: {}", e)))
}

/// Get all event attribute keys with their statistics.
///
/// Returns JSON: `[{"name":"concept:name","count":7,"unique_values":3}, ...]`
///
/// Mirrors `pm4py.get_event_attributes()`.
#[wasm_bindgen]
pub fn get_event_attributes(log_json: &str) -> Result<String, JsValue> {
    let log: EventLog = serde_json::from_str(log_json)
        .map_err(|e| JsValue::from_str(&format!("Event log JSON error: {}", e)))?;
    let result = statistics::basic::get_event_attributes(&log);
    serde_json::to_string(&result)
        .map_err(|e| JsValue::from_str(&format!("JSON error: {}", e)))
}

/// Get all trace attribute keys with their statistics.
///
/// Returns JSON: `[{"name":"case_id","count":3,"unique_values":3}, ...]`
///
/// Mirrors `pm4py.get_trace_attributes()`.
#[wasm_bindgen]
pub fn get_trace_attributes(log_json: &str) -> Result<String, JsValue> {
    let log: EventLog = serde_json::from_str(log_json)
        .map_err(|e| JsValue::from_str(&format!("Event log JSON error: {}", e)))?;
    let result = statistics::basic::get_trace_attributes(&log);
    serde_json::to_string(&result)
        .map_err(|e| JsValue::from_str(&format!("JSON error: {}", e)))
}

/// Get case attributes with their statistics.
///
/// Returns JSON: `[{"name":"case_id","count":3,"unique_values":3}, ...]`
///
/// Mirrors `pm4py.get_case_attributes()`.
#[wasm_bindgen]
pub fn get_case_attributes(log_json: &str) -> Result<String, JsValue> {
    let log: EventLog = serde_json::from_str(log_json)
        .map_err(|e| JsValue::from_str(&format!("Event log JSON error: {}", e)))?;
    let result = statistics::basic::get_case_attributes(&log);
    serde_json::to_string(&result)
        .map_err(|e| JsValue::from_str(&format!("JSON error: {}", e)))
}

/// Get performance statistics for an event log.
///
/// Returns JSON with `total_cases`, `total_events`, `avg_case_duration_ms`,
/// `min_case_duration_ms`, `max_case_duration_ms`, `median_case_duration_ms`,
/// `total_events_longest_case`, `avg_events_per_case`.
#[wasm_bindgen]
pub fn get_performance_stats(log_json: &str) -> Result<String, JsValue> {
    let log: EventLog = serde_json::from_str(log_json)
        .map_err(|e| JsValue::from_str(&format!("Event log JSON error: {}", e)))?;
    let result = statistics::performance::get_performance_stats(&log);
    serde_json::to_string(&result)
        .map_err(|e| JsValue::from_str(&format!("JSON error: {}", e)))
}

/// Get the average case arrival rate (cases per hour).
///
/// Mirrors `pm4py.get_case_arrival_average()`.
#[wasm_bindgen]
pub fn get_case_arrival_average(log_json: &str) -> Result<f64, JsValue> {
    let log: EventLog = serde_json::from_str(log_json)
        .map_err(|e| JsValue::from_str(&format!("Event log JSON error: {}", e)))?;
    Ok(statistics::performance::get_case_arrival_average(&log))
}

/// Get all distinct values for a given event attribute with frequencies.
///
/// Returns JSON: `[{"attribute":"concept:name","value":"A","count":3}, ...]`
///
/// Mirrors `pm4py.get_attribute_values()`.
#[wasm_bindgen]
pub fn get_attribute_values(log_json: &str, attribute_key: &str) -> Result<String, JsValue> {
    let log: EventLog = serde_json::from_str(log_json)
        .map_err(|e| JsValue::from_str(&format!("Event log JSON error: {}", e)))?;
    let result = statistics::basic::get_attribute_values(&log, attribute_key);
    serde_json::to_string(&result)
        .map_err(|e| JsValue::from_str(&format!("JSON error: {}", e)))
}

/// Get case durations as JSON.
///
/// Returns JSON: `[{"case_id":"1","duration_ms":300000}, ...]`
///
/// Mirrors `pm4py.get_case_durations()`.
#[wasm_bindgen]
pub fn get_case_durations(log_json: &str) -> Result<String, JsValue> {
    let log: EventLog = serde_json::from_str(log_json)
        .map_err(|e| JsValue::from_str(&format!("Event log JSON error: {}", e)))?;
    let result = statistics::basic::get_case_durations_json(&log);
    serde_json::to_string(&result)
        .map_err(|e| JsValue::from_str(&format!("JSON error: {}", e)))
}

/// Get rework times (time between consecutive same-activity events in a case).
///
/// Returns JSON: `[{"case_id":"1","activity":"A","duration_ms":60000}, ...]`
///
/// Mirrors `pm4py.get_rework_times()`.
#[wasm_bindgen]
pub fn get_rework_times(log_json: &str) -> Result<String, JsValue> {
    let log: EventLog = serde_json::from_str(log_json)
        .map_err(|e| JsValue::from_str(&format!("Event log JSON error: {}", e)))?;
    let result = statistics::basic::get_rework_times(&log);
    serde_json::to_string(&result)
        .map_err(|e| JsValue::from_str(&format!("JSON error: {}", e)))
}

/// Get minimum self-distances for each activity.
///
/// Returns JSON: `[{"activity":"A","min_distance_ms":300000}, ...]`
///
/// Mirrors `pm4py.get_minimum_self_distances()`.
#[wasm_bindgen]
pub fn get_minimum_self_distances(log_json: &str) -> Result<String, JsValue> {
    let log: EventLog = serde_json::from_str(log_json)
        .map_err(|e| JsValue::from_str(&format!("Event log JSON error: {}", e)))?;
    let result = statistics::basic::get_minimum_self_distances(&log);
    serde_json::to_string(&result)
        .map_err(|e| JsValue::from_str(&format!("JSON error: {}", e)))
}

/// Get all case durations as a flat JSON array of milliseconds.
///
/// Returns JSON: `[300000, 600000, 1800000, ...]`
///
/// Mirrors `pm4py.get_all_case_durations()`.
#[wasm_bindgen]
pub fn get_all_case_durations(log_json: &str) -> Result<String, JsValue> {
    let log: EventLog = serde_json::from_str(log_json)
        .map_err(|e| JsValue::from_str(&format!("Event log JSON error: {}", e)))?;
    let result = statistics::basic::get_all_case_durations(&log);
    serde_json::to_string(&result)
        .map_err(|e| JsValue::from_str(&format!("JSON error: {}", e)))
}

/// Get case overlap (fraction of shared prefixes between traces, 0.0–1.0).
///
/// Mirrors `pm4py.get_case_overlap()`.
#[wasm_bindgen]
pub fn get_case_overlap(log_json: &str) -> Result<f64, JsValue> {
    let log: EventLog = serde_json::from_str(log_json)
        .map_err(|e| JsValue::from_str(&format!("Event log JSON error: {}", e)))?;
    Ok(statistics::basic::get_case_overlap(&log))
}

/// Get all prefixes (partial traces) from the log with their frequencies.
///
/// Returns JSON: `[{"prefix":["A"],"count":3,"percentage":50.0}, ...]`
///
/// Mirrors `pm4py.get_prefixes_from_log()`.
#[wasm_bindgen]
pub fn get_prefixes_from_log(log_json: &str) -> Result<String, JsValue> {
    let log: EventLog = serde_json::from_str(log_json)
        .map_err(|e| JsValue::from_str(&format!("Event log JSON error: {}", e)))?;
    let result = statistics::basic::get_prefixes_from_log(&log);
    serde_json::to_string(&result)
        .map_err(|e| JsValue::from_str(&format!("JSON error: {}", e)))
}

/// Get trace attribute values with frequencies.
///
/// Returns JSON: `[{"attribute":"concept:name","value":"1","count":1}, ...]`
///
/// Mirrors `pm4py.get_trace_attribute_values()`.
#[wasm_bindgen]
pub fn get_trace_attribute_values(log_json: &str, attribute_key: &str) -> Result<String, JsValue> {
    let log: EventLog = serde_json::from_str(log_json)
        .map_err(|e| JsValue::from_str(&format!("Event log JSON error: {}", e)))?;
    let result = statistics::basic::get_trace_attribute_values(&log, attribute_key);
    serde_json::to_string(&result)
        .map_err(|e| JsValue::from_str(&format!("JSON error: {}", e)))
}

/// Get variants as tuples (activity sequences with count).
///
/// Returns JSON: `[{"activities":["A","B"],"count":1}, ...]`
///
/// Mirrors `pm4py.get_variants_as_tuples()`.
#[wasm_bindgen]
pub fn get_variants_as_tuples(log_json: &str) -> Result<String, JsValue> {
    let log: EventLog = serde_json::from_str(log_json)
        .map_err(|e| JsValue::from_str(&format!("Event log JSON error: {}", e)))?;
    let result = statistics::basic::get_variants_as_tuples(&log);
    serde_json::to_string(&result)
        .map_err(|e| JsValue::from_str(&format!("JSON error: {}", e)))
}

/// Get variants with path durations (total, min, max, avg).
///
/// Returns JSON: `[{"activities":["A","B"],"count":1,"total_duration_ms":300000,...}, ...]`
///
/// Mirrors `pm4py.get_variants_paths_duration()`.
#[wasm_bindgen]
pub fn get_variants_paths_duration(log_json: &str) -> Result<String, JsValue> {
    let log: EventLog = serde_json::from_str(log_json)
        .map_err(|e| JsValue::from_str(&format!("Event log JSON error: {}", e)))?;
    let result = statistics::basic::get_variants_paths_duration(&log);
    serde_json::to_string(&result)
        .map_err(|e| JsValue::from_str(&format!("JSON error: {}", e)))
}

/// Get cases per activity that show rework (activity appears more than once).
///
/// Returns JSON: `[{"activity":"A","rework_cases":2,"total_cases":3}, ...]`
///
/// Mirrors `pm4py.get_rework_cases_per_activity()`.
#[wasm_bindgen]
pub fn get_rework_cases_per_activity(log_json: &str) -> Result<String, JsValue> {
    let log: EventLog = serde_json::from_str(log_json)
        .map_err(|e| JsValue::from_str(&format!("Event log JSON error: {}", e)))?;
    let result = statistics::basic::get_rework_cases_per_activity(&log);
    serde_json::to_string(&result)
        .map_err(|e| JsValue::from_str(&format!("JSON error: {}", e)))
}

// ─── Filtering API ───────────────────────────────────────────────────────────────

/// Filter log to keep only traces starting with specified activities.
///
/// Returns filtered log as JSON.
///
/// Mirrors `pm4py.filter_start_activities()`.
#[wasm_bindgen]
pub fn filter_start_activities(log_json: &str, activities_json: &str) -> Result<String, JsValue> {
    let log: EventLog = serde_json::from_str(log_json)
        .map_err(|e| JsValue::from_str(&format!("Event log JSON error: {}", e)))?;
    let activities: Vec<String> = serde_json::from_str(activities_json)
        .map_err(|e| JsValue::from_str(&format!("Activities JSON error: {}", e)))?;
    let result = filtering::activities::filter_start_activities(&log, &activities);
    serde_json::to_string(&result)
        .map_err(|e| JsValue::from_str(&format!("JSON error: {}", e)))
}

/// Filter log to keep only traces ending with specified activities.
///
/// Returns filtered log as JSON.
///
/// Mirrors `pm4py.filter_end_activities()`.
#[wasm_bindgen]
pub fn filter_end_activities(log_json: &str, activities_json: &str) -> Result<String, JsValue> {
    let log: EventLog = serde_json::from_str(log_json)
        .map_err(|e| JsValue::from_str(&format!("Event log JSON error: {}", e)))?;
    let activities: Vec<String> = serde_json::from_str(activities_json)
        .map_err(|e| JsValue::from_str(&format!("Activities JSON error: {}", e)))?;
    let result = filtering::activities::filter_end_activities(&log, &activities);
    serde_json::to_string(&result)
        .map_err(|e| JsValue::from_str(&format!("JSON error: {}", e)))
}

/// Filter log to keep only traces containing a specific activity relation.
///
/// Keeps traces where activity `a` is directly followed by activity `b`.
///
/// Returns filtered log as JSON.
///
/// Mirrors `pm4py.filter_directly_follows_relation()`.
#[wasm_bindgen]
pub fn filter_directly_follows_relation(log_json: &str, a: &str, b: &str) -> Result<String, JsValue> {
    let log: EventLog = serde_json::from_str(log_json)
        .map_err(|e| JsValue::from_str(&format!("Event log JSON error: {}", e)))?;
    let result = filtering::activities::filter_directly_follows_relation(&log, a, b);
    serde_json::to_string(&result)
        .map_err(|e| JsValue::from_str(&format!("JSON error: {}", e)))
}

/// Filter log to keep only traces between two activities.
///
/// Keeps traces that contain both activities and where `act1` appears before `act2`.
///
/// Returns filtered log as JSON.
///
/// Mirrors `pm4py.filter_between()`.
#[wasm_bindgen]
pub fn filter_between(log_json: &str, act1: &str, act2: &str) -> Result<String, JsValue> {
    let log: EventLog = serde_json::from_str(log_json)
        .map_err(|e| JsValue::from_str(&format!("Event log JSON error: {}", e)))?;
    let result = filtering::activities::filter_between(&log, act1, act2);
    serde_json::to_string(&result)
        .map_err(|e| JsValue::from_str(&format!("JSON error: {}", e)))
}

/// Filter log to keep only traces starting with a specific prefix.
///
/// Returns filtered log as JSON.
///
/// Mirrors `pm4py.filter_prefixes()`.
#[wasm_bindgen]
pub fn filter_prefixes(log_json: &str, prefix_json: &str) -> Result<String, JsValue> {
    let log: EventLog = serde_json::from_str(log_json)
        .map_err(|e| JsValue::from_str(&format!("Event log JSON error: {}", e)))?;
    let prefix: Vec<String> = serde_json::from_str(prefix_json)
        .map_err(|e| JsValue::from_str(&format!("Prefix JSON error: {}", e)))?;
    let result = filtering::activities::filter_prefixes(&log, &prefix);
    serde_json::to_string(&result)
        .map_err(|e| JsValue::from_str(&format!("JSON error: {}", e)))
}

/// Filter log to keep only traces ending with a specific suffix.
///
/// Returns filtered log as JSON.
///
/// Mirrors `pm4py.filter_suffixes()`.
#[wasm_bindgen]
pub fn filter_suffixes(log_json: &str, suffix_json: &str) -> Result<String, JsValue> {
    let log: EventLog = serde_json::from_str(log_json)
        .map_err(|e| JsValue::from_str(&format!("Event log JSON error: {}", e)))?;
    let suffix: Vec<String> = serde_json::from_str(suffix_json)
        .map_err(|e| JsValue::from_str(&format!("Suffix JSON error: {}", e)))?;
    let result = filtering::activities::filter_suffixes(&log, &suffix);
    serde_json::to_string(&result)
        .map_err(|e| JsValue::from_str(&format!("JSON error: {}", e)))
}

/// Filter log to keep only traces within a case size range.
///
/// Returns filtered log as JSON.
///
/// Mirrors `pm4py.filter_case_size()`.
#[wasm_bindgen]
pub fn filter_case_size(log_json: &str, min_size: usize, max_size: usize) -> Result<String, JsValue> {
    let log: EventLog = serde_json::from_str(log_json)
        .map_err(|e| JsValue::from_str(&format!("Event log JSON error: {}", e)))?;
    let result = filtering::case_size::filter_case_size(&log, min_size, max_size);
    serde_json::to_string(&result)
        .map_err(|e| JsValue::from_str(&format!("JSON error: {}", e)))
}

/// Filter log to keep only events within a time range.
///
/// Returns filtered log as JSON.
///
/// Mirrors `pm4py.filter_time_range()`.
#[wasm_bindgen]
pub fn filter_time_range(log_json: &str, start_ms: i64, end_ms: i64) -> Result<String, JsValue> {
    let log: EventLog = serde_json::from_str(log_json)
        .map_err(|e| JsValue::from_str(&format!("Event log JSON error: {}", e)))?;
    let result = filtering::time::filter_time_range(&log, start_ms, end_ms);
    serde_json::to_string(&result)
        .map_err(|e| JsValue::from_str(&format!("JSON error: {}", e)))
}

/// Filter log to keep only the top K most frequent variants.
///
/// Returns filtered log as JSON.
///
/// Mirrors `pm4py.filter_variants_top_k()`.
#[wasm_bindgen]
pub fn filter_variants_top_k(log_json: &str, k: usize) -> Result<String, JsValue> {
    let log: EventLog = serde_json::from_str(log_json)
        .map_err(|e| JsValue::from_str(&format!("Event log JSON error: {}", e)))?;
    let result = filtering::variants::filter_variants_top_k(&log, k);
    serde_json::to_string(&result)
        .map_err(|e| JsValue::from_str(&format!("JSON error: {}", e)))
}

/// Filter log to keep only variants covering at least a percentage of traces.
///
/// Returns filtered log as JSON.
///
/// Mirrors `pm4py.filter_variants_by_coverage_percentage()`.
#[wasm_bindgen]
pub fn filter_variants_coverage(log_json: &str, min_coverage: f64) -> Result<String, JsValue> {
    let log: EventLog = serde_json::from_str(log_json)
        .map_err(|e| JsValue::from_str(&format!("Event log JSON error: {}", e)))?;
    let result = filtering::variants::filter_variants_coverage(&log, min_coverage);
    serde_json::to_string(&result)
        .map_err(|e| JsValue::from_str(&format!("JSON error: {}", e)))
}

// ─── Discovery API ──────────────────────────────────────────────────────────────

/// Discover a Directly-Follows Graph from an event log.
///
/// Returns JSON with `edges`, `start_activities`, `end_activities`, `activities`.
///
/// Mirrors `pm4py.discover_dfg()`.
#[wasm_bindgen]
pub fn discover_dfg(log_json: &str) -> Result<String, JsValue> {
    let log: EventLog = serde_json::from_str(log_json)
        .map_err(|e| JsValue::from_str(&format!("Event log JSON error: {}", e)))?;
    let result = discovery::dfg::discover_dfg(&log);
    serde_json::to_string(&result)
        .map_err(|e| JsValue::from_str(&format!("JSON error: {}", e)))
}

/// Discover a typed DFG (structured format) from an event log.
///
/// Returns JSON with `graph` (from, to, frequency triples), `start_activities`,
/// `end_activities`, and `activities` (activity, frequency pairs).
///
/// Mirrors `pm4py.discover_dfg_typed()`.
///
/// # Errors
/// Throws if JSON cannot be deserialised.
#[wasm_bindgen]
pub fn discover_dfg_typed(log_json: &str) -> Result<String, JsValue> {
    let log: EventLog = serde_json::from_str(log_json)
        .map_err(|e| JsValue::from_str(&format!("Event log JSON error: {}", e)))?;
    let result = discovery::dfg::discover_dfg_typed(&log);
    serde_json::to_string(&result)
        .map_err(|e| JsValue::from_str(&format!("JSON error: {}", e)))
}

/// Serialize a DFG result to a canonical JSON string (round-trip safe).
///
/// Takes the JSON output from `discover_dfg` and returns a pretty-printed JSON string.
///
/// Mirrors `pm4py.write_dfg()`.
#[wasm_bindgen]
pub fn write_dfg(dfg_json: &str) -> Result<String, JsValue> {
    let dfg: discovery::dfg::DFGResult = serde_json::from_str(dfg_json)
        .map_err(|e| JsValue::from_str(&format!("DFG JSON error: {}", e)))?;
    serde_json::to_string_pretty(&dfg)
        .map_err(|e| JsValue::from_str(&format!("JSON error: {}", e)))
}

/// Deserialize a DFG from a JSON string.
///
/// Validates the JSON structure and returns the DFG result as JSON.
///
/// Mirrors `pm4py.read_dfg()`.
#[wasm_bindgen]
pub fn read_dfg(dfg_json: &str) -> Result<String, JsValue> {
    let dfg: discovery::dfg::DFGResult = serde_json::from_str(dfg_json)
        .map_err(|e| JsValue::from_str(&format!("DFG JSON error: {}", e)))?;
    serde_json::to_string(&dfg)
        .map_err(|e| JsValue::from_str(&format!("JSON error: {}", e)))
}

/// Discover a performance DFG with duration annotations on edges.
///
/// Returns JSON with edges containing `avg_duration_ms`, `min_duration_ms`, `max_duration_ms`.
///
/// Mirrors `pm4py.discover_performance_dfg()`.
#[wasm_bindgen]
pub fn discover_performance_dfg(log_json: &str) -> Result<String, JsValue> {
    let log: EventLog = serde_json::from_str(log_json)
        .map_err(|e| JsValue::from_str(&format!("Event log JSON error: {}", e)))?;
    let result = discovery::dfg::discover_performance_dfg(&log);
    serde_json::to_string(&result)
        .map_err(|e| JsValue::from_str(&format!("JSON error: {}", e)))
}

/// Discover an eventually-follows graph (all activity pairs in any trace).
///
/// Returns JSON array of edges: `[{"source":"A","target":"C","count":2}, ...]`
///
/// Mirrors `pm4py.discover_eventually_follows_graph()`.
#[wasm_bindgen]
pub fn discover_eventually_follows_graph(log_json: &str) -> Result<String, JsValue> {
    let log: EventLog = serde_json::from_str(log_json)
        .map_err(|e| JsValue::from_str(&format!("Event log JSON error: {}", e)))?;
    let result = discovery::dfg::discover_eventually_follows_graph(&log);
    serde_json::to_string(&result)
        .map_err(|e| JsValue::from_str(&format!("JSON error: {}", e)))
}

/// Discover a process tree using the inductive miner.
///
/// Returns a JSON representation of the process tree with `label`, `operator`,
/// and `children` fields. Leaf nodes have `label` set; internal nodes have
/// `operator` set to one of: `"->"`, `"X"`, `"+"`, `"*"`.
///
/// Mirrors `pm4py.discover_process_tree_inductive()`.
#[wasm_bindgen]
pub fn discover_process_tree_inductive(log_json: &str) -> Result<String, JsValue> {
    let log: EventLog = serde_json::from_str(log_json)
        .map_err(|e| JsValue::from_str(&format!("Event log JSON error: {}", e)))?;
    let tree = discovery::inductive_miner::inductive_miner(&log);
    let simplified = discovery::inductive_miner::simplify_tree(tree);
    serde_json::to_string(&simplified)
        .map_err(|e| JsValue::from_str(&format!("JSON error: {}", e)))
}

/// Discover a BPMN model directly from an event log using the inductive miner.
///
/// Returns a complete BPMN 2.0 XML document. Combines inductive miner discovery
/// with BPMN conversion in a single call -- no intermediate steps required.
///
/// Mirrors `pm4py.discover_bpmn_inductive()`.
///
/// # Errors
/// Throws if the event log JSON cannot be parsed.
#[wasm_bindgen]
pub fn discover_bpmn_inductive(log_json: &str) -> Result<String, JsValue> {
    let log: EventLog = serde_json::from_str(log_json)
        .map_err(|e| JsValue::from_str(&format!("Event log JSON error: {}", e)))?;
    let tree = discovery::inductive_miner::inductive_miner(&log);
    let simplified = discovery::inductive_miner::simplify_tree(tree);
    Ok(conversion::process_tree_to_bpmn::process_tree_to_bpmn_xml(&simplified))
}

/// Discover a Petri net from an event log using the inductive miner.
///
/// Returns the same JSON as `powl_to_petri_net`: `{net, initial_marking, final_marking}`.
///
/// Mirrors `pm4py.discover_petri_net_inductive()`.
#[wasm_bindgen]
pub fn discover_petri_net_inductive(log_json: &str) -> Result<String, JsValue> {
    let log: EventLog = serde_json::from_str(log_json)
        .map_err(|e| JsValue::from_str(&format!("Event log JSON error: {}", e)))?;
    let tree = discovery::inductive_miner::inductive_miner(&log);
    let simplified = discovery::inductive_miner::simplify_tree(tree);
    // Convert process tree to Petri net using existing conversion
    let result = conversion::to_petri_net::from_process_tree(&simplified);
    serde_json::to_string(&result)
        .map_err(|e| JsValue::from_str(&format!("JSON error: {}", e)))
}

/// Discover a Petri net from an event log using the alpha miner.
///
/// Returns the same JSON format as other discovery functions.
///
/// Mirrors `pm4py.discover_petri_net_alpha()`.
#[wasm_bindgen]
pub fn discover_petri_net_alpha(log_json: &str) -> Result<String, JsValue> {
    let log: EventLog = serde_json::from_str(log_json)
        .map_err(|e| JsValue::from_str(&format!("Event log JSON error: {}", e)))?;
    let result = discovery::alpha_miner::alpha_miner(&log);
    serde_json::to_string(&result)
        .map_err(|e| JsValue::from_str(&format!("JSON error: {}", e)))
}

/// Discover a Petri net using the Alpha+ miner algorithm (extends Alpha with loop handling).
///
/// Input: Event log JSON (same format as other discovery functions).
/// Output: PetriNetResult JSON with places, transitions, arcs, and markings.
///
/// Returns the same JSON format as other discovery functions.
///
/// Mirrors `pm4py.discover_petri_net_alpha_plus()`.
#[wasm_bindgen]
pub fn discover_petri_net_alpha_plus(log_json: &str) -> Result<String, JsValue> {
    let log: EventLog = serde_json::from_str(log_json)
        .map_err(|e| JsValue::from_str(&format!("Event log JSON error: {}", e)))?;
    let result = discovery::alpha_plus_miner::alpha_plus_miner(&log);
    serde_json::to_string(&result)
        .map_err(|e| JsValue::from_str(&format!("JSON error: {}", e)))
}

/// Check whether a Petri net is sound (deadlock-free, bounded, liveness).
///
/// Returns JSON: `{"sound":true,"deadlock_free":true,"bounded":true,"liveness":true}`
///
/// Mirrors `pm4py.check_soundness()`.
#[wasm_bindgen]
pub fn check_soundness(petri_net_json: &str) -> Result<String, JsValue> {
    let pn_result: PetriNetResult = serde_json::from_str(petri_net_json)
        .map_err(|e| JsValue::from_str(&format!("Petri net JSON error: {}", e)))?;
    let result = conformance::soundness::check_soundness(
        &pn_result.net,
        &pn_result.initial_marking,
        &pn_result.final_marking,
    );
    serde_json::to_string(&result)
        .map_err(|e| JsValue::from_str(&format!("JSON error: {}", e)))
}

// ============================================================================
// OCEL (Object-Centric Event Log) Functions
// ============================================================================

/// Parse an OCEL from JSON string (JSON-OCEL format).
///
/// Input: OCEL JSON string (JSON-OCEL 1.0 or 2.0 format).
/// Output: Serialized OCEL object with events, objects, relations, and globals.
///
/// Returns the same JSON format (round-trips through parse/serialize).
///
/// Mirrors `pm4py.read_ocel()`.
#[wasm_bindgen]
pub fn parse_ocel_json(ocel_json: &str) -> Result<String, JsValue> {
    let ocel = ocel::parse_ocel_json(ocel_json)
        .map_err(|e| JsValue::from_str(&format!("OCEL JSON error: {}", e)))?;
    serde_json::to_string(&ocel)
        .map_err(|e| JsValue::from_str(&format!("JSON error: {}", e)))
}

/// Get a summary of an OCEL.
///
/// Input: OCEL JSON string.
/// Output: JSON with event_count, object_count, relation_count, object_types, etc.
///
/// Mirrors `pm4py.ocel_summary()`.
#[wasm_bindgen]
pub fn ocel_get_summary(ocel_json: &str) -> Result<String, JsValue> {
    let ocel: ocel::OCEL = serde_json::from_str(ocel_json)
        .map_err(|e| JsValue::from_str(&format!("OCEL JSON error: {}", e)))?;
    let summary = ocel::get_summary(&ocel);
    serde_json::to_string(&summary)
        .map_err(|e| JsValue::from_str(&format!("JSON error: {}", e)))
}

/// Discover Event-Type / Object-Type graph from an OCEL.
///
/// Input: OCEL JSON string.
/// Output: JSON with activities, object_types, edges, and edge_frequencies.
///
/// Mirrors `pm4py.discover_ocel_etot()`.
#[wasm_bindgen]
pub fn discover_ocel_etot(ocel_json: &str) -> Result<String, JsValue> {
    let ocel: ocel::OCEL = serde_json::from_str(ocel_json)
        .map_err(|e| JsValue::from_str(&format!("OCEL JSON error: {}", e)))?;
    let result = ocel::discover_etot(&ocel);
    serde_json::to_string(&result)
        .map_err(|e| JsValue::from_str(&format!("JSON error: {}", e)))
}

/// Flatten an OCEL to a traditional event log by object type.
///
/// Input: OCEL JSON string and object type name.
/// Output: Traditional EventLog JSON (traces with case_id and events).
///
/// The resulting log can be used with all standard discovery functions:
/// - discover_dfg()
/// - discover_petri_net_alpha_plus()
/// - inductive_miner()
///
/// Mirrors `pm4py.ocel_flattening()`.
#[wasm_bindgen]
pub fn ocel_flatten_by_object_type(ocel_json: &str, object_type: &str) -> Result<String, JsValue> {
    let ocel: ocel::OCEL = serde_json::from_str(ocel_json)
        .map_err(|e| JsValue::from_str(&format!("OCEL JSON error: {}", e)))?;
    let log = ocel::flatten_by_object_type(&ocel, object_type)
        .map_err(|e| JsValue::from_str(&e))?;
    serde_json::to_string(&log)
        .map_err(|e| JsValue::from_str(&format!("JSON error: {}", e)))
}

/// Get all object types in an OCEL.
///
/// Input: OCEL JSON string.
/// Output: JSON array of object type names.
///
/// Mirrors `pm4py.ocel_object_types`.
#[wasm_bindgen]
pub fn ocel_get_object_types(ocel_json: &str) -> Result<String, JsValue> {
    let ocel: ocel::OCEL = serde_json::from_str(ocel_json)
        .map_err(|e| JsValue::from_str(&format!("OCEL JSON error: {}", e)))?;
    let types = ocel::get_object_types(&ocel);
    serde_json::to_string(&types)
        .map_err(|e| JsValue::from_str(&format!("JSON error: {}", e)))
}

/// Get all event types (activities) in an OCEL.
///
/// Input: OCEL JSON string.
/// Output: JSON array of activity names.
///
/// Mirrors `pm4py.ocel_event_types`.
#[wasm_bindgen]
pub fn ocel_get_event_types(ocel_json: &str) -> Result<String, JsValue> {
    let ocel: ocel::OCEL = serde_json::from_str(ocel_json)
        .map_err(|e| JsValue::from_str(&format!("OCEL JSON error: {}", e)))?;
    let types = ocel::get_event_types(&ocel);
    serde_json::to_string(&types)
        .map_err(|e| JsValue::from_str(&format!("JSON error: {}", e)))
}

/// Discover a Log Skeleton from an event log.
///
/// Returns JSON with six constraint types: equivalence, always_after, always_before,
/// never_together, directly_follows, and activ_freq.
///
/// Mirrors `pm4py.discover_log_skeleton()`.
#[wasm_bindgen]
pub fn discover_log_skeleton(log_json: &str) -> Result<String, JsValue> {
    let log: EventLog = serde_json::from_str(log_json)
        .map_err(|e| JsValue::from_str(&format!("Event log JSON error: {}", e)))?;
    let result = discovery::log_skeleton::discover_log_skeleton(&log, 0.0);
    serde_json::to_string(&result)
        .map_err(|e| JsValue::from_str(&format!("JSON error: {}", e)))
}

/// Discover a DECLARE model from an event log.
///
/// Returns JSON with constraint templates (response, precedence, succession, etc.)
/// and their support/confidence metrics.
///
/// Mirrors `pm4py.discover_declare()`.
#[wasm_bindgen]
pub fn discover_declare(log_json: &str) -> Result<String, JsValue> {
    let log: EventLog = serde_json::from_str(log_json)
        .map_err(|e| JsValue::from_str(&format!("Event log JSON error: {}", e)))?;
    let result = discovery::declare::discover_declare(&log, None, None, None);
    serde_json::to_string(&result)
        .map_err(|e| JsValue::from_str(&format!("JSON error: {}", e)))
}

/// Discover a Petri net using the genetic miner (evolutionary algorithm).
///
/// Returns the same JSON format as other discovery functions: `{net, initial_marking, final_marking}`.
/// Optionally accepts a JSON config object with `population_size`, `generations`,
/// `mutation_rate`, `crossover_rate`.
///
/// Mirrors `pm4py.discover_petri_net_genetic()`.
///
/// # Errors
/// Throws if JSON cannot be deserialised.
#[wasm_bindgen]
pub fn discover_petri_net_genetic(log_json: &str, config_json: &str) -> Result<String, JsValue> {
    let log: EventLog = serde_json::from_str(log_json)
        .map_err(|e| JsValue::from_str(&format!("Event log JSON error: {}", e)))?;
    let config: Option<discovery::genetic_miner::GeneticMinerConfig> =
        if config_json.is_empty() {
            None
        } else {
            Some(serde_json::from_str(config_json)
                .map_err(|e| JsValue::from_str(&format!("Config JSON error: {}", e)))?)
        };
    let result = discovery::genetic_miner::discover_genetic(&log, config);
    serde_json::to_string(&result)
        .map_err(|e| JsValue::from_str(&format!("JSON serialisation error: {}", e)))
}

/// Discover a Petri net from an event log using the heuristics miner.
///
/// The heuristics miner is more lenient than the alpha miner for handling
/// noise and incomplete data. Uses dependency measures to filter causal relations.
///
/// Returns the same JSON format as other discovery functions: `{net, initial_marking, final_marking}`.
///
/// Mirrors `pm4py.discover_petri_net_heuristics()`.
///
/// # Arguments
/// * `log_json` - Event log JSON
/// * `dependency_threshold` - Minimum dependency score for an edge (0.0 to 1.0, typically 0.8-0.99)
///
/// # Errors
/// Throws if JSON cannot be deserialised.
#[wasm_bindgen]
pub fn discover_petri_net_heuristics(log_json: &str, dependency_threshold: f64) -> Result<String, JsValue> {
    let log: EventLog = serde_json::from_str(log_json)
        .map_err(|e| JsValue::from_str(&format!("Event log JSON error: {}", e)))?;
    let net = discovery::heuristics_miner::discover_heuristics_miner(&log, dependency_threshold);
    let result = discovery::heuristics_miner::heuristics_to_petri_net(&net);
    serde_json::to_string(&result)
        .map_err(|e| JsValue::from_str(&format!("JSON serialisation error: {}", e)))
}

/// Discover a Heuristics Net from an event log.
///
/// Returns JSON with activities, dependencies (with dependency scores and frequencies),
/// and start/end activities. Useful for visualization of causal relations.
///
/// Mirrors `pm4py.discover_heuristics_net()`.
///
/// # Arguments
/// * `log_json` - Event log JSON
/// * `dependency_threshold` - Minimum dependency score for an edge (0.0 to 1.0)
///
/// # Errors
/// Throws if JSON cannot be deserialised.
#[wasm_bindgen]
pub fn discover_heuristics_net(log_json: &str, dependency_threshold: f64) -> Result<String, JsValue> {
    let log: EventLog = serde_json::from_str(log_json)
        .map_err(|e| JsValue::from_str(&format!("Event log JSON error: {}", e)))?;
    let result = discovery::heuristics_miner::discover_heuristics_miner(&log, dependency_threshold);
    serde_json::to_string(&result)
        .map_err(|e| JsValue::from_str(&format!("JSON serialisation error: {}", e)))
}

/// Discover footprints from a POWL model.
///
/// Footprints summarize the behavioral properties of a process model:
/// - Start/end activities
/// - Sequence (directly-follows) and parallel (concurrent) pairs
/// - Minimum trace length
/// - Whether activities are skippable
///
/// Mirrors `pm4py.discover_footprints()` for POWL models.
///
/// # Errors
/// Throws if POWL string cannot be parsed.
#[wasm_bindgen]
pub fn discover_footprints_from_model(powl_string: &str) -> Result<String, JsValue> {
    let mut arena = PowlArena::new();
    let root = parse_powl_model_string(powl_string, &mut arena)
        .map_err(|e| JsValue::from_str(&format!("POWL parse error: {}", e)))?;
    let result = footprints::apply(&arena, root);
    serde_json::to_string(&result)
        .map_err(|e| JsValue::from_str(&format!("JSON serialisation error: {}", e)))
}

/// Discover footprints from an event log.
///
/// Computes footprints from the log's directly-follows graph.
/// Returns the same structure as `discover_footprints_from_model`.
///
/// Mirrors `pm4py.discover_footprints()` for event logs.
///
/// # Errors
/// Throws if JSON cannot be deserialised.
#[wasm_bindgen]
pub fn discover_footprints_from_log(log_json: &str) -> Result<String, JsValue> {
    let log: EventLog = serde_json::from_str(log_json)
        .map_err(|e| JsValue::from_str(&format!("Event log JSON error: {}", e)))?;
    let result = footprints::discover_from_log(&log);
    serde_json::to_string(&result)
        .map_err(|e| JsValue::from_str(&format!("JSON serialisation error: {}", e)))
}

/// Compute footprints-based conformance diagnostics.
///
/// Compares the log's directly-follows graph against a model's footprints
/// to compute fitness, precision, recall, and f1-score.
///
/// Returns JSON: `{"fitness": 0.95, "precision": 0.85, "recall": 0.95, "f1": 0.90}`
///
/// Mirrors `pm4py.conformance_diagnostics_footprints()`.
///
/// # Errors
/// Throws if JSON cannot be deserialised.
#[wasm_bindgen]
pub fn footprints_diagnostics(log_json: &str, model_fp_json: &str) -> Result<String, JsValue> {
    let log: EventLog = serde_json::from_str(log_json)
        .map_err(|e| JsValue::from_str(&format!("Event log JSON error: {}", e)))?;
    let model_fp: Footprints = serde_json::from_str(model_fp_json)
        .map_err(|e| JsValue::from_str(&format!("Footprints JSON error: {}", e)))?;
    let result = conformance::footprints_conf::check(&log, &model_fp);
    serde_json::to_string(&result)
        .map_err(|e| JsValue::from_str(&format!("JSON serialisation error: {}", e)))
}

/// Compute footprints-based fitness.
///
/// Fraction of the log's directly-follows pairs that are allowed by the model.
/// Returns a value in [0.0, 1.0] where 1.0 is perfect fitness.
///
/// Mirrors `pm4py.fitness_footprints()`.
///
/// # Errors
/// Throws if JSON cannot be deserialised.
#[wasm_bindgen]
pub fn footprints_fitness(log_json: &str, model_fp_json: &str) -> Result<f64, JsValue> {
    let log: EventLog = serde_json::from_str(log_json)
        .map_err(|e| JsValue::from_str(&format!("Event log JSON error: {}", e)))?;
    let model_fp: Footprints = serde_json::from_str(model_fp_json)
        .map_err(|e| JsValue::from_str(&format!("Footprints JSON error: {}", e)))?;
    let result = conformance::footprints_conf::check(&log, &model_fp);
    Ok(result.fitness)
}

/// Compute footprints-based precision.
///
/// Fraction of the model's directly-follows pairs that are observed in the log.
/// Returns a value in [0.0, 1.0] where 1.0 is perfect precision.
///
/// Mirrors `pm4py.precision_footprints()`.
///
/// # Errors
/// Throws if JSON cannot be deserialised.
#[wasm_bindgen]
pub fn footprints_precision(log_json: &str, model_fp_json: &str) -> Result<f64, JsValue> {
    let log: EventLog = serde_json::from_str(log_json)
        .map_err(|e| JsValue::from_str(&format!("Event log JSON error: {}", e)))?;
    let model_fp: Footprints = serde_json::from_str(model_fp_json)
        .map_err(|e| JsValue::from_str(&format!("Footprints JSON error: {}", e)))?;
    let result = conformance::footprints_conf::check(&log, &model_fp);
    Ok(result.precision)
}

/// Discover a process model using correlation mining (case-less / timestamp-based).
///
/// Unlike other discovery algorithms, correlation mining works without case IDs
/// by detecting temporal gaps between activity occurrences.
///
/// Returns JSON with `start_activity`, `end_activity`, `trace_count`, `edges`.
///
/// Mirrors `pm4py.discover_correlation()`.
///
/// # Arguments
/// * `log_json` - Event log JSON
/// * `correlation_threshold` - Gap threshold in seconds (typically 5.0-3600.0)
///
/// # Errors
/// Throws if JSON cannot be deserialised.
#[wasm_bindgen]
pub fn discover_correlation(log_json: &str, correlation_threshold: f64) -> Result<String, JsValue> {
    let log: EventLog = serde_json::from_str(log_json)
        .map_err(|e| JsValue::from_str(&format!("Event log JSON error: {}", e)))?;
    let mut config = discovery::correlation_miner::CorrelationConfig::default();
    config.correlation_threshold = correlation_threshold;
    let result = discovery::correlation_miner::discover_correlation_from_log(&log, Some(config));
    serde_json::to_string(&result)
        .map_err(|e| JsValue::from_str(&format!("JSON serialisation error: {}", e)))
}

/// Detect batch processing patterns in an event log.
///
/// Returns JSON with `batches` array, each containing `type` (sequential, concurrent,
/// parallel, concurrent_parallel), `activity`, `instances` with start/end timestamps.
///
/// Mirrors `pm4py.discover_batches()`.
///
/// # Errors
/// Throws if JSON cannot be deserialised.
#[wasm_bindgen]
pub fn discover_batches(log_json: &str) -> Result<String, JsValue> {
    let log: EventLog = serde_json::from_str(log_json)
        .map_err(|e| JsValue::from_str(&format!("Event log JSON error: {}", e)))?;
    let result = discovery::batches::discover_batches(&log);
    serde_json::to_string(&result)
        .map_err(|e| JsValue::from_str(&format!("JSON serialisation error: {}", e)))
}

/// Discover performance spectrum for a specific activity.
///
/// Returns JSON with `activity`, `overall_stats`, `instance_data` for visualization.
/// Useful for D3.js or similar charting libraries.
///
/// Mirrors `pm4py.discover_performance_spectrum()`.
///
/// # Errors
/// Throws if JSON cannot be deserialised.
#[wasm_bindgen]
pub fn discover_performance_spectrum(log_json: &str, activity: &str) -> Result<String, JsValue> {
    let log: EventLog = serde_json::from_str(log_json)
        .map_err(|e| JsValue::from_str(&format!("Event log JSON error: {}", e)))?;
    let result = discovery::performance_spectrum::discover_performance_spectrum(&log, activity);
    serde_json::to_string(&result)
        .map_err(|e| JsValue::from_str(&format!("JSON serialisation error: {}", e)))
}

/// Compute generalization quality metric for a Petri net against an event log.
///
/// Returns JSON with `generalization` score in [0.0, 1.0].
/// Higher scores indicate the model generalizes well (not overfitting).
///
/// Mirrors `pm4py.generalization()`.
///
/// # Errors
/// Throws if JSON cannot be deserialised.
#[wasm_bindgen]
pub fn generalization(petri_net_json: &str, log_json: &str) -> Result<String, JsValue> {
    let pn_result: PetriNetResult = serde_json::from_str(petri_net_json)
        .map_err(|e| JsValue::from_str(&format!("Petri net JSON error: {}", e)))?;
    let log: EventLog = serde_json::from_str(log_json)
        .map_err(|e| JsValue::from_str(&format!("Event log JSON error: {}", e)))?;
    let result = quality::generalization::compute_quality(
        &pn_result.net,
        &pn_result.initial_marking,
        &pn_result.final_marking,
        &log,
    );
    serde_json::to_string(&result)
        .map_err(|e| JsValue::from_str(&format!("JSON serialisation error: {}", e)))
}

/// Reduce a Petri net by applying structural reduction rules.
///
/// Applies fusion of series places, fusion of series transitions,
/// elimination of self-loop places, and other reduction rules.
///
/// Returns the reduced Petri net as JSON (same format as discovery functions).
///
/// Mirrors `pm4py.reduce_petri_net()`.
///
/// # Errors
/// Throws if JSON cannot be deserialised.
#[wasm_bindgen]
pub fn reduce_petri_net(petri_net_json: &str) -> Result<String, JsValue> {
    let mut pn_result: PetriNetResult = serde_json::from_str(petri_net_json)
        .map_err(|e| JsValue::from_str(&format!("Petri net JSON error: {}", e)))?;
    algorithms::reduction::reduce_petri_net(&mut pn_result.net);
    serde_json::to_string(&pn_result)
        .map_err(|e| JsValue::from_str(&format!("JSON serialisation error: {}", e)))
}

/// Count the number of reducible elements in a Petri net.
///
/// Returns the count of elements that could be reduced by `reduce_petri_net()`.
///
/// Mirrors `pm4py.count_reducible_elements()`.
///
/// # Errors
/// Throws if JSON cannot be deserialised.
#[wasm_bindgen]
pub fn count_reducible_elements(petri_net_json: &str) -> Result<usize, JsValue> {
    let pn_result: PetriNetResult = serde_json::from_str(petri_net_json)
        .map_err(|e| JsValue::from_str(&format!("Petri net JSON error: {}", e)))?;
    Ok(algorithms::reduction::count_reducible_elements(&pn_result.net))
}

// ─── Temporal Profile API ──────────────────────────────────────────────────────

/// Discover a temporal profile from an event log.
///
/// Returns JSON with directly-follows pairs and their mean/stdev duration in ms.
///
/// Mirrors `pm4py.discover_temporal_profile()`.
#[wasm_bindgen]
pub fn discover_temporal_profile(log_json: &str) -> Result<String, JsValue> {
    let log: EventLog = serde_json::from_str(log_json)
        .map_err(|e| JsValue::from_str(&format!("Event log JSON error: {}", e)))?;
    let result = discovery::temporal_profile::discover_temporal_profile(&log);
    serde_json::to_string(&result)
        .map_err(|e| JsValue::from_str(&format!("JSON error: {}", e)))
}

/// Check temporal conformance between an event log and a temporal profile.
///
/// Returns JSON with fitness, deviations count, and detailed deviation list.
///
/// # Arguments
/// * `log_json` - Event log JSON
/// * `profile_json` - Temporal profile JSON (from discover_temporal_profile)
/// * `zeta` - Number of standard deviations for threshold (typically 2.0)
#[wasm_bindgen]
pub fn check_temporal_conformance(log_json: &str, profile_json: &str, zeta: f64) -> Result<String, JsValue> {
    let log: EventLog = serde_json::from_str(log_json)
        .map_err(|e| JsValue::from_str(&format!("Event log JSON error: {}", e)))?;
    let profile: discovery::temporal_profile::TemporalProfile = serde_json::from_str(profile_json)
        .map_err(|e| JsValue::from_str(&format!("Temporal profile JSON error: {}", e)))?;
    let result = discovery::temporal_profile::check_temporal_conformance(&log, &profile, zeta);
    serde_json::to_string(&result)
        .map_err(|e| JsValue::from_str(&format!("JSON error: {}", e)))
}

// ─── Heuristics Miner API ───────────────────────────────────────────────────────

/// Discover a Heuristics Net from an event log.
///
/// Returns JSON with activities, dependency measures, and start/end activities.
///
/// Mirrors `pm4py.discover_heuristics_miner()`.
///
/// # Arguments
/// * `log_json` - Event log JSON
/// * `dependency_threshold` - Minimum dependency score (0.0 to 1.0, typically 0.5-0.9)
#[wasm_bindgen]
pub fn discover_heuristics_miner(log_json: &str, dependency_threshold: f64) -> Result<String, JsValue> {
    let log: EventLog = serde_json::from_str(log_json)
        .map_err(|e| JsValue::from_str(&format!("Event log JSON error: {}", e)))?;
    let result = discovery::heuristics_miner::discover_heuristics_miner(&log, dependency_threshold);
    serde_json::to_string(&result)
        .map_err(|e| JsValue::from_str(&format!("JSON error: {}", e)))
}

/// Convert a Heuristics Net to a Petri Net.
///
/// Returns JSON with places, transitions, and arcs.
///
/// # Arguments
/// * `net_json` - Heuristics net JSON (from discover_heuristics_miner)
#[wasm_bindgen]
pub fn heuristics_to_petri_net(net_json: &str) -> Result<String, JsValue> {
    let net: discovery::heuristics_miner::HeuristicsNet = serde_json::from_str(net_json)
        .map_err(|e| JsValue::from_str(&format!("Heuristics net JSON error: {}", e)))?;
    let result = discovery::heuristics_miner::heuristics_to_petri_net(&net);
    serde_json::to_string(&result)
        .map_err(|e| JsValue::from_str(&format!("JSON error: {}", e)))
}

// ─── Footprints API ────────────────────────────────────────────────────────────

/// Compute the behavioural footprints of a POWL model.
///
/// Returns JSON with `start_activities`, `end_activities`, `activities`,
/// `skippable`, `sequence` (directly-follows pairs), `parallel` (concurrent pairs),
/// `activities_always_happening`, and `min_trace_length`.
///
/// Mirrors the footprint analysis from `pm4py.discover_footprints()`.
#[wasm_bindgen]
pub fn compute_footprints(model: &PowlModel) -> Result<String, JsValue> {
    let fp = footprints::apply(&model.arena, model.root);
    serde_json::to_string(&fp)
        .map_err(|e| JsValue::from_str(&format!("JSON error: {}", e)))
}

/// Compute footprints-based conformance between an event log and a POWL model.
///
/// Compares the log's directly-follows graph against the model's footprints to
/// compute fitness, precision, recall, and f1-score.
///
/// Returns JSON: `{"fitness":0.95,"precision":0.85,"recall":0.90,"f1":0.87}`
#[wasm_bindgen]
pub fn conformance_footprints(log_json: &str, model_str: &str) -> Result<String, JsValue> {
    let log: EventLog = serde_json::from_str(log_json)
        .map_err(|e| JsValue::from_str(&format!("Event log JSON error: {}", e)))?;
    let mut arena = PowlArena::new();
    let root = parse_powl_model_string(model_str, &mut arena)
        .map_err(|e| JsValue::from_str(&format!("POWL parse error: {}", e)))?;

    let model_fp = footprints::apply(&arena, root);
    let result = conformance::footprints_conf::check(&log, &model_fp);
    serde_json::to_string(&result)
        .map_err(|e| JsValue::from_str(&format!("JSON error: {}", e)))
}

// ─── Event Log Write API ───────────────────────────────────────────────────────

/// Serialize an event log (JSON) to XES XML format.
///
/// Mirrors `pm4py.write_xes()`.
#[wasm_bindgen]
pub fn write_xes_log(log_json: &str) -> Result<String, JsValue> {
    let log: EventLog = serde_json::from_str(log_json)
        .map_err(|e| JsValue::from_str(&format!("Event log JSON error: {}", e)))?;
    Ok(event_log::write_xes(&log))
}

/// Serialize an event log (JSON) to CSV format.
///
/// Mirrors `pm4py.write_csv()`.
#[wasm_bindgen]
pub fn write_csv_log(log_json: &str) -> Result<String, JsValue> {
    let log: EventLog = serde_json::from_str(log_json)
        .map_err(|e| JsValue::from_str(&format!("Event log JSON error: {}", e)))?;
    Ok(event_log::write_csv(&log))
}

// ─── Utility ─────────────────────────────────────────────────────────────────

/// Set up the `console_error_panic_hook` in debug/dev builds so Rust panics
/// surface as useful browser console messages.  Call once after `init()`.
#[wasm_bindgen(start)]
pub fn start() {
    #[cfg(feature = "console_error_panic_hook")]
    console_error_panic_hook::set_once();
}

/// Trim traces to remove events before the first start activity and after the last end activity.
///
/// Mirrors `pm4py.filter_trim()`.
#[wasm_bindgen]
pub fn filter_trim(log_json: &str, start_activity: &str, end_activity: &str) -> Result<String, JsValue> {
    let log: EventLog = serde_json::from_str(log_json)
        .map_err(|e| JsValue::from_str(&format!("Event log JSON error: {}", e)))?;
    let filtered = filtering::activities::filter_trim(&log, start_activity, end_activity);
    serde_json::to_string(&filtered)
        .map_err(|e| JsValue::from_str(&format!("JSON error: {}", e)))
}

// ─── LLM API ──────────────────────────────────────────────────────────────────────

/// Validate a POWL model string against structural soundness criteria.
///
/// Uses POWLJudge to check for deadlock freedom, liveness, and boundedness.
/// Returns a JSON object with `verdict` (boolean), `reasoning` (string),
/// and `violations` (array of violation strings).
///
/// # Errors
/// Throws if the POWL string cannot be parsed.
#[wasm_bindgen]
pub fn validate_powl_structure(model_str: &str) -> Result<String, JsValue> {
    let (verdict, reasoning) = llm::validate_powl_structure(model_str);
    let result = serde_json::json!({
        "verdict": verdict,
        "reasoning": reasoning
    });
    serde_json::to_string(&result)
        .map_err(|e| JsValue::from_str(&format!("JSON error: {}", e)))
}

/// Get few-shot demos for a specific domain.
///
/// Returns a JSON array of few-shot examples for LLM-guided POWL generation.
/// Supported domains: "loan_approval", "finance", "software_release", "it",
/// "devops", "ecommerce", "retail", "manufacturing", "production",
/// "healthcare", "medical".
///
/// For unknown domains, returns general demos.
#[wasm_bindgen]
pub fn get_demos_for_domain(domain: &str) -> String {
    llm::get_demos_for_domain(domain)
}

/// Generate executable code from a POWL model string.
///
/// Converts POWL to n8n JSON, Temporal Go, Camunda BPMN, or YAWL v6 XML.
///
/// # Errors
/// Throws if the POWL string cannot be parsed or if the target is unknown.
#[wasm_bindgen]
pub fn generate_code_from_powl(model_str: &str, target: &str) -> Result<String, JsValue> {
    let mut arena = PowlArena::new();
    let root = parse_powl_model_string(model_str, &mut arena)
        .map_err(|e| JsValue::from_str(&format!("POWL parse error: {}", e)))?;
    let result = llm::generate_code(&arena, root, target)
        .map_err(|e| JsValue::from_str(&e))?;
    let output = serde_json::json!({
        "code": result.code,
        "target": result.target.as_str(),
        "format": match result.format {
            llm::CodeFormat::Json => "json",
            llm::CodeFormat::Xml => "xml",
            llm::CodeFormat::Go => "go",
            llm::CodeFormat::Javascript => "javascript",
        }
    });
    serde_json::to_string(&output)
        .map_err(|e| JsValue::from_str(&format!("JSON error: {}", e)))
}

// ─── Log Footprints (from event log, not model) ──────────────────────────────────

/// Discover footprints directly from an event log's directly-follows graph.
///
/// Returns JSON with `start_activities`, `end_activities`, `activities`,
/// `sequence` (directly-follows pairs), and `parallel` (bidirectional pairs).
///
/// Mirrors `pm4py.discover_footprints()` applied to a log.
#[wasm_bindgen]
pub fn discover_log_footprints(log_json: &str) -> Result<String, JsValue> {
    let log: EventLog = serde_json::from_str(log_json)
        .map_err(|e| JsValue::from_str(&format!("Event log JSON error: {}", e)))?;

    let mut start_act: std::collections::HashSet<String> = std::collections::HashSet::new();
    let mut end_act: std::collections::HashSet<String> = std::collections::HashSet::new();
    let mut activities: std::collections::HashSet<String> = std::collections::HashSet::new();
    let mut sequence: std::collections::HashSet<(String, String)> = std::collections::HashSet::new();

    for trace in &log.traces {
        if let Some(first) = trace.events.first() {
            start_act.insert(first.name.clone());
        }
        if let Some(last) = trace.events.last() {
            end_act.insert(last.name.clone());
        }
        for event in &trace.events {
            activities.insert(event.name.clone());
        }
        for window in trace.events.windows(2) {
            sequence.insert((window[0].name.clone(), window[1].name.clone()));
        }
    }

    // Bidirectional sequence pairs → parallel
    let mut parallel: std::collections::HashSet<(String, String)> = std::collections::HashSet::new();
    let bidirectional: std::collections::HashSet<(String, String)> = sequence
        .iter()
        .filter(|(a, b)| sequence.contains(&(b.clone(), a.clone())))
        .cloned()
        .collect();
    for pair in &bidirectional {
        parallel.insert(pair.clone());
        sequence.remove(pair);
    }

    let result = serde_json::json!({
        "start_activities": start_act,
        "end_activities": end_act,
        "activities": activities,
        "sequence": sequence,
        "parallel": parallel,
    });
    serde_json::to_string(&result)
        .map_err(|e| JsValue::from_str(&format!("JSON error: {}", e)))
}

// ─── Utility Functions ──────────────────────────────────────────────────────────

/// Sort an event log by case_id then timestamp.
///
/// Returns the sorted event log as JSON.
///
/// Mirrors `pm4py.sort_log()`.
#[wasm_bindgen]
pub fn sort_log(log_json: &str) -> Result<String, JsValue> {
    let mut log: EventLog = serde_json::from_str(log_json)
        .map_err(|e| JsValue::from_str(&format!("Event log JSON error: {}", e)))?;
    log.traces.sort_by(|a, b| a.case_id.cmp(&b.case_id));
    for trace in &mut log.traces {
        trace.events.sort_by(|a, b| {
            match (&a.timestamp, &b.timestamp) {
                (Some(ta), Some(tb)) => ta.cmp(tb),
                (Some(_), None) => std::cmp::Ordering::Less,
                (None, Some(_)) => std::cmp::Ordering::Greater,
                (None, None) => std::cmp::Ordering::Equal,
            }
        });
    }
    serde_json::to_string(&log)
        .map_err(|e| JsValue::from_str(&format!("JSON error: {}", e)))
}

/// Project an event log to keep only the specified attributes.
///
/// `attributes_json` is a JSON array of attribute key strings.
/// The `concept:name` (activity name) attribute is always preserved.
///
/// Mirrors `pm4py.project_log()`.
#[wasm_bindgen]
pub fn project_log(log_json: &str, attributes_json: &str) -> Result<String, JsValue> {
    let mut log: EventLog = serde_json::from_str(log_json)
        .map_err(|e| JsValue::from_str(&format!("Event log JSON error: {}", e)))?;
    let attrs: std::collections::HashSet<String> = serde_json::from_str(attributes_json)
        .map_err(|e| JsValue::from_str(&format!("Attributes JSON error: {}", e)))?;

    for trace in &mut log.traces {
        for event in &mut trace.events {
            event.attributes.retain(|k, _| attrs.contains(k));
        }
    }
    serde_json::to_string(&log)
        .map_err(|e| JsValue::from_str(&format!("JSON error: {}", e)))
}

// ─── Simulation ───────────────────────────────────────────────────────────────────

/// Simulate an event log from a process tree using playout.
///
/// Generates synthetic traces by executing the process tree structure.
/// This is useful for testing, generating training data, and validating models.
///
/// Returns JSON with `traces` array, each containing `case_id` and `events`.
///
/// Mirrors `pm4py.play_out()` for process trees.
///
/// # Arguments
/// * `process_tree_json` - Process tree JSON (from `powl_to_process_tree`)
/// * `num_traces` - Number of traces to generate (default: 100)
/// * `include_timestamps` - Whether to include timestamps (default: true)
///
/// # Errors
/// Throws if JSON cannot be deserialised.
#[wasm_bindgen]
pub fn play_out(process_tree_json: &str, num_traces: usize, include_timestamps: bool) -> Result<String, JsValue> {
    let pt: crate::process_tree::ProcessTree = serde_json::from_str(process_tree_json)
        .map_err(|e| JsValue::from_str(&format!("Process tree JSON error: {}", e)))?;
    let params = simulation::PlayOutParameters {
        num_traces,
        include_timestamps,
        ..Default::default()
    };
    let result = simulation::play_out_process_tree(&pt, &params);
    serde_json::to_string(&result)
        .map_err(|e| JsValue::from_str(&format!("JSON error: {}", e)))
}

/// Simulate an event log from a directly-follows graph using playout.
///
/// Generates synthetic traces by random walk through the DFG structure.
/// This is useful for testing and generating synthetic data.
///
/// Returns JSON with `traces` array, each containing `case_id` and `events`.
///
/// Mirrors `pm4py.play_out()` for DFGs.
///
/// # Arguments
/// * `dfg_json` - DFG JSON (from `discover_dfg`)
/// * `start_activities_json` - Start activities JSON array
/// * `end_activities_json` - End activities JSON array
/// * `num_traces` - Number of traces to generate (default: 100)
/// * `include_timestamps` - Whether to include timestamps (default: true)
///
/// # Errors
/// Throws if JSON cannot be deserialised.
#[wasm_bindgen]
pub fn play_out_dfg(
    dfg_json: &str,
    start_activities_json: &str,
    end_activities_json: &str,
    num_traces: usize,
    include_timestamps: bool,
) -> Result<String, JsValue> {
    // Parse DFG JSON
    let dfg_result: discovery::dfg::DFGResult = serde_json::from_str(dfg_json)
        .map_err(|e| JsValue::from_str(&format!("DFG JSON error: {}", e)))?;

    // Build DirectedGraph for playout
    let mut dfg = simulation::playout::DirectedGraph::default();
    for (activity, _) in &dfg_result.activities {
        dfg.activities.push(activity.clone());
    }
    for edge in &dfg_result.edges {
        dfg.adj.entry(edge.source.clone()).or_insert_with(Vec::new).push(edge.target.clone());
    }

    let start_activities: Vec<String> = serde_json::from_str(start_activities_json)
        .map_err(|e| JsValue::from_str(&format!("Start activities JSON error: {}", e)))?;
    let end_activities: Vec<String> = serde_json::from_str(end_activities_json)
        .map_err(|e| JsValue::from_str(&format!("End activities JSON error: {}", e)))?;

    let params = simulation::PlayOutParameters {
        num_traces,
        include_timestamps,
        ..Default::default()
    };

    let result = simulation::play_out_dfg(&dfg, &start_activities, &end_activities, &params);
    serde_json::to_string(&result)
        .map_err(|e| JsValue::from_str(&format!("JSON error: {}", e)))
}

// ─── Causal Graph Discovery ─────────────────────────────────────────────────────

/// Discover causal relations from a directly-follows graph (alpha variant).
///
/// Causal relations identify which activities have a one-way dependency:
/// - A → B is causal if A always precedes B (B never precedes A)
///
/// Returns JSON with `relations` map: (from, to) → 1 (binary causal indicator).
///
/// Mirrors `pm4py.discover_causal()` with variant="alpha".
///
/// # Arguments
/// * `dfg_json` - DFG JSON (from `discover_dfg`)
///
/// # Errors
/// Throws if JSON cannot be deserialised.
#[wasm_bindgen]
pub fn discover_causal_alpha(dfg_json: &str) -> Result<String, JsValue> {
    let dfg: discovery::dfg::DFGResult = serde_json::from_str(dfg_json)
        .map_err(|e| JsValue::from_str(&format!("DFG JSON error: {}", e)))?;
    let result = discovery::causal::discover_causal_alpha(&dfg.edges);
    serde_json::to_string(&result)
        .map_err(|e| JsValue::from_str(&format!("JSON error: {}", e)))
}

/// Discover causal relations from a directly-follows graph (heuristic variant).
///
/// The heuristic variant uses a threshold-based approach where:
/// - Relation (A, B) is causal if its frequency is significantly higher than (B, A)
///
/// Returns JSON with `relations` map: (from, to) → strength (0-1000).
///
/// Mirrors `pm4py.discover_causal()` with variant="heuristic".
///
/// # Arguments
/// * `dfg_json` - DFG JSON (from `discover_dfg`)
/// * `threshold` - Minimum ratio for causality (default: 0.8)
///
/// # Errors
/// Throws if JSON cannot be deserialised.
#[wasm_bindgen]
pub fn discover_causal_heuristic(dfg_json: &str, threshold: f64) -> Result<String, JsValue> {
    let dfg: discovery::dfg::DFGResult = serde_json::from_str(dfg_json)
        .map_err(|e| JsValue::from_str(&format!("DFG JSON error: {}", e)))?;
    let result = discovery::causal::discover_causal_heuristic(&dfg.edges, threshold);
    serde_json::to_string(&result)
        .map_err(|e| JsValue::from_str(&format!("JSON error: {}", e)))
}

// ─── Transition System Discovery ───────────────────────────────────────────────

/// Discover a transition system from an event log.
///
/// A transition system is a state machine that captures all observed behavior:
/// - Each state represents a "view" of the trace (window of recent activities)
/// - Transitions represent activity executions that move between states
///
/// Returns JSON with `states` (id, name) and `transitions` (from_state, to_state, activity, count).
///
/// Mirrors `pm4py.discover_transition_system()`.
///
/// # Arguments
/// * `log_json` - Event log JSON
/// * `window` - Size of the lookback window (default: 2)
/// * `direction` - "forward" (default) or "backward" direction
///
/// # Errors
/// Throws if JSON cannot be deserialised.
#[wasm_bindgen]
pub fn discover_transition_system(log_json: &str, window: usize, direction: &str) -> Result<String, JsValue> {
    let log: EventLog = serde_json::from_str(log_json)
        .map_err(|e| JsValue::from_str(&format!("Event log JSON error: {}", e)))?;
    let result = discovery::transition_system::discover_transition_system(&log, window, direction);
    serde_json::to_string(&result)
        .map_err(|e| JsValue::from_str(&format!("JSON error: {}", e)))
}

/// Discover a prefix tree (trie) from an event log.
///
/// A prefix tree represents all unique prefixes of activity sequences in the log.
/// Each node in the tree represents an activity, and paths from root represent trace prefixes.
/// Nodes that represent the end of a trace are marked as `is_final = true`.
///
/// Returns JSON with the trie structure: nodes array with label, parent, children, is_final, depth.
///
/// Mirrors `pm4py.discover_prefix_tree()`.
///
/// # Arguments
/// * `log_json` - Event log JSON
/// * `max_path_length` - Optional maximum trace length (traces are truncated)
///
/// # Errors
/// Throws if JSON cannot be deserialised.
#[wasm_bindgen]
pub fn discover_prefix_tree(log_json: &str, max_path_length: Option<usize>) -> Result<String, JsValue> {
    let log: EventLog = serde_json::from_str(log_json)
        .map_err(|e| JsValue::from_str(&format!("Event log JSON error: {}", e)))?;
    let result = transformation::discover_prefix_tree(&log, max_path_length);
    serde_json::to_string(&result)
        .map_err(|e| JsValue::from_str(&format!("JSON error: {}", e)))
}

/// Discover a Petri net using ILP miner (STUB - NOT IMPLEMENTED).
///
/// The ILP (Integer Linear Programming) miner requires an LP solver
/// which is not available in pure WASM. This function returns an error.
///
/// Consider using `discover_petri_net_inductive()` or `discover_petri_net_heuristics()` instead.
///
/// Mirrors `pm4py.discover_petri_net_ilp()`.
///
/// # Errors
/// Always throws an error explaining ILP miner is not available in WASM.
#[wasm_bindgen]
pub fn discover_petri_net_ilp(_log_json: &str, _alpha: f64) -> Result<String, JsValue> {
    Err(JsValue::from_str(
        "ILP miner is not available in WASM. \
         It requires an external LP solver which cannot be bundled in pure WebAssembly. \
         Please use `discover_petri_net_inductive()` or `discover_petri_net_heuristics()` instead."
    ))
}

// ─── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // Test PowlModel methods (not WASM-bound functions)
    #[test]
    fn test_powl_model_root() {
        let mut arena = PowlArena::new();
        let root = parser::parse_powl_model_string("PO=(nodes={A}, order={})", &mut arena).unwrap();
        let model = PowlModel { arena, root };
        assert_eq!(model.root(), root);
    }

    #[test]
    fn test_powl_model_len() {
        let mut arena = PowlArena::new();
        let root = parser::parse_powl_model_string("PO=(nodes={A}, order={})", &mut arena).unwrap();
        let model = PowlModel { arena, root };
        // Parsed model creates a StrictPartialOrder node plus the transition node
        assert!(model.len() >= 1);
    }

    #[test]
    fn test_powl_model_is_empty() {
        let arena = PowlArena::new();
        let model = PowlModel { arena, root: 0 };
        assert!(model.is_empty());
    }

    // Test node_info_json (not WASM-bound)
    #[test]
    fn test_node_info_json_transition() {
        let mut arena = PowlArena::new();
        let root = parser::parse_powl_model_string("PO=(nodes={A}, order={})", &mut arena).unwrap();
        let model = PowlModel { arena, root };
        let json = node_info_json(&model, 0);
        assert!(json.contains("Transition"));
        assert!(json.contains("A"));
    }

    #[test]
    fn test_node_info_json_invalid() {
        let arena = PowlArena::new();
        let model = PowlModel { arena, root: 0 };
        let json = node_info_json(&model, 999);
        assert!(json.contains("Invalid"));
    }

    // Test get_children (not WASM-bound)
    #[test]
    fn test_get_children_transition() {
        let mut arena = PowlArena::new();
        let root = parser::parse_powl_model_string("PO=(nodes={A}, order={})", &mut arena).unwrap();
        let model = PowlModel { arena, root };
        let children = get_children(&model, 0);
        assert!(children.is_empty());
    }

    #[test]
    fn test_get_children_spo() {
        let mut arena = PowlArena::new();
        let root = parser::parse_powl_model_string("PO=(nodes={A, B}, order={A-->B})", &mut arena).unwrap();
        let model = PowlModel { arena, root };
        let children = get_children(&model, root);
        assert_eq!(children.len(), 2);
    }

    // Test node_to_string (not WASM-bound)
    #[test]
    fn test_node_to_string() {
        let mut arena = PowlArena::new();
        let root = parser::parse_powl_model_string("PO=(nodes={A}, order={})", &mut arena).unwrap();
        let model = PowlModel { arena, root };
        let s = node_to_string(&model, 0);
        assert!(!s.is_empty());
    }

    // Test discover_bpmn_inductive logic (non-WASM, tests core pipeline)
    #[test]
    fn test_discover_bpmn_inductive_logic() {
        let log = EventLog {
            traces: vec![
                event_log::Trace {
                    case_id: "1".to_string(),
                    events: vec![
                        event_log::Event {
                            name: "A".to_string(),
                            timestamp: None,
                            lifecycle: None,
                            attributes: std::collections::HashMap::new(),
                        },
                        event_log::Event {
                            name: "B".to_string(),
                            timestamp: None,
                            lifecycle: None,
                            attributes: std::collections::HashMap::new(),
                        },
                    ],
                },
            ],
        };
        // Run the same pipeline as discover_bpmn_inductive but without JsValue
        let tree = discovery::inductive_miner::inductive_miner(&log);
        let simplified = discovery::inductive_miner::simplify_tree(tree);
        let bpmn_xml = conversion::process_tree_to_bpmn::process_tree_to_bpmn_xml(&simplified);
        assert!(bpmn_xml.contains("<definitions"));
        assert!(bpmn_xml.contains("<process"));
        assert!(bpmn_xml.contains("<startEvent"));
        assert!(bpmn_xml.contains("<endEvent"));
        assert!(bpmn_xml.contains("</definitions>"));
        // The inductive miner should discover A -> B sequence
        assert!(bpmn_xml.contains(r#"name="A""#));
        assert!(bpmn_xml.contains(r#"name="B""#));
    }

    #[test]
    fn test_discover_bpmn_inductive_xor_log() {
        // Two alternative traces -> XOR
        let log = EventLog {
            traces: vec![
                event_log::Trace {
                    case_id: "1".to_string(),
                    events: vec![
                        event_log::Event {
                            name: "A".to_string(),
                            timestamp: None,
                            lifecycle: None,
                            attributes: std::collections::HashMap::new(),
                        },
                    ],
                },
                event_log::Trace {
                    case_id: "2".to_string(),
                    events: vec![
                        event_log::Event {
                            name: "B".to_string(),
                            timestamp: None,
                            lifecycle: None,
                            attributes: std::collections::HashMap::new(),
                        },
                    ],
                },
            ],
        };
        let tree = discovery::inductive_miner::inductive_miner(&log);
        let simplified = discovery::inductive_miner::simplify_tree(tree);
        let bpmn_xml = conversion::process_tree_to_bpmn::process_tree_to_bpmn_xml(&simplified);
        assert!(bpmn_xml.contains("<definitions"));
        assert!(bpmn_xml.contains("exclusiveGateway"));
        assert!(bpmn_xml.contains(r#"name="A""#));
        assert!(bpmn_xml.contains(r#"name="B""#));
    }
}
