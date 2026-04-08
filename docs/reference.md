# API Reference: POWL v2 Rust/WASM

Complete API documentation for the `pm4wasm` library.

## Table of Contents

1. [Core Types](#core-types)
2. [Parsing](#parsing)
3. [Validation](#validation)
4. [Model Manipulation](#model-manipulation)
5. [Graph Operations](#graph-operations)
6. [Introspection](#introspection)
7. [Conversions](#conversions)
   - [To Petri Net](#to-petri-net)
   - [To Process Tree](#to-process-tree)
8. [Analysis](#analysis)
   - [Footprints](#footprints)
9. [Process Discovery](#process-discovery)
10. [Object-Centric Event Logs](#object-centric-event-logs-ocel)

---

## Core Types

### `PowlModel`

The main opaque handle representing a parsed POWL model. Contains a flat arena of all nodes referenced by u32 indices.

**Methods:**

#### `root() -> u32`
Returns the arena index of the root node.

```javascript
const model = parse_powl("A");
const root_idx = model.root();  // 0
```

#### `len() -> usize`
Returns the total number of nodes in the arena.

```javascript
const model = parse_powl("X(A, B)");
console.log(model.len());  // 3 (XOR + A + B)
```

#### `is_empty() -> boolean`
Returns whether the arena is empty.

---

### `BinaryRelationJs`

A serializable adjacency matrix representing a partial order (strict partial order relation). Internally uses bit-packed rows for memory efficiency.

**Methods:**

#### `n() -> usize`
Returns the number of nodes in the relation.

```javascript
const rel = transitive_closure(model, spo_idx);
const node_count = rel.n();
```

#### `is_edge(i: usize, j: usize) -> boolean`
Tests whether a directed edge i→j exists.

```javascript
if (rel.is_edge(0, 1)) {
  console.log("Node 0 precedes node 1");
}
```

#### `edges_flat() -> Vec<u32>`
Returns all edges as a flat array `[src0, tgt0, src1, tgt1, …]`.

```javascript
const edges = rel.edges_flat();
for (let i = 0; i < edges.length; i += 2) {
  const src = edges[i], tgt = edges[i+1];
  console.log(`Edge: ${src} → ${tgt}`);
}
```

#### `is_irreflexive() -> boolean`
Returns true if the relation has no self-loops (no node relates to itself).

```javascript
const rel = transitive_reduction(model, spo_idx);
console.log(rel.is_irreflexive());  // true for valid SPO
```

#### `is_transitive() -> boolean`
Returns true if the relation is transitive (if a→b and b→c then a→c).

#### `is_strict_partial_order() -> boolean`
Returns true if both `is_irreflexive()` and `is_transitive()` hold.

```javascript
const rel = get_order_of(model, spo_idx);
if (!rel.is_strict_partial_order()) {
  console.error("Invalid partial order");
}
```

#### `start_nodes() -> Vec<u32>`
Returns arena indices of nodes with no incoming edges.

```javascript
const rel = get_order_of(model, spo_idx);
const starts = rel.start_nodes();
// e.g., [0, 1] if nodes 0 and 1 can start the PO
```

#### `end_nodes() -> Vec<u32>`
Returns arena indices of nodes with no outgoing edges.

```javascript
const ends = rel.end_nodes();
// e.g., [2] if only node 2 can end the PO
```

---

## Parsing

### `parse_powl(s: &str) -> Result<PowlModel, JsValue>`

Parse a POWL model string (same format as Python's `__repr__`) and return a `PowlModel` handle.

**Arguments:**
- `s`: POWL model string

**Returns:** `PowlModel` on success

**Throws:** JavaScript `Error` if parsing fails (e.g., syntax error, unmatched braces)

**Examples:**

```javascript
// Simple transition
const model = parse_powl("A");

// Sequential model
const seq = parse_powl("PO=(nodes={A, B, C}, order={A-->B, B-->C})");

// Choice (XOR) operator
const choice = parse_powl("X(A, B)");

// Loop operator
const loop_model = parse_powl("*(A, B)");

// Nested partial order
const nested = parse_powl(
  "PO=(nodes={A, PO=(nodes={B, C}, order={}), D}, order={A-->D, PO=(nodes={B, C}, order={})-->D})"
);
```

**Error examples:**

```javascript
try {
  parse_powl("invalid{syntax");
} catch (e) {
  console.log(e.message);  // "POWL parse error: ..."
}

try {
  parse_powl("PO=(nodes={A}, order={A-->A})");  // Self-loop
} catch (e) {
  // Parsing succeeds, but validation will catch it
}
```

---

## Validation

### `validate_partial_orders(model: &PowlModel) -> Result<(), JsValue>`

Validate that all `StrictPartialOrder` nodes in the model have irreflexive and transitive orderings.

**Arguments:**
- `model`: A `PowlModel` to validate

**Throws:** JavaScript `Error` describing the first violation found

**Examples:**

```javascript
const model = parse_powl("PO=(nodes={A, B, C}, order={A-->B, B-->C, A-->C})");

try {
  validate_partial_orders(model);
  console.log("✓ Model is valid");
} catch (e) {
  console.error("✗ Validation failed:", e.message);
}
```

**Violations detected:**
- Self-loops (reflexivity)
- Intransitivity (A→B, B→C but no A→C)
- Cycles in the order relation

---

## Model Manipulation

### `powl_to_string(model: &PowlModel) -> String`

Return the canonical string representation of the model (same format as Python's `__repr__`).

**Arguments:**
- `model`: A `PowlModel`

**Returns:** String representation

**Examples:**

```javascript
const model = parse_powl("X(A, B)");
console.log(powl_to_string(model));  // "X ( A, B )"

const spo = parse_powl("PO=(nodes={A, B}, order={A-->B})");
console.log(powl_to_string(spo));
// "PO=(nodes={ A, B }, order={ A-->B })"
```

---

### `simplify_powl(model: &PowlModel) -> PowlModel`

Recursively simplify the model by:
- Flattening nested XOR/LOOP operators
- Merging redundant patterns
- Inlining single-child sub-SPOs

Returns a new `PowlModel` (original is unchanged).

**Arguments:**
- `model`: A `PowlModel`

**Returns:** Simplified `PowlModel`

**Examples:**

```javascript
// Flatten nested XOR
let model = parse_powl("X(A, X(B, C))");
model = simplify_powl(model);
console.log(powl_to_string(model));  // "X ( A, B, C )"

// Inline single-child SPO
model = parse_powl("PO=(nodes={PO=(nodes={A}, order={})}, order={})");
model = simplify_powl(model);
console.log(powl_to_string(model));  // "A"
```

---

### `simplify_frequent_transitions(model: &PowlModel) -> PowlModel`

Convert common patterns to `FrequentTransition` nodes for optionality and loops:
- `XOR(A, tau)` → `FrequentTransition(activity=A, skippable=true, selfloop=false)`
- `LOOP(A, tau)` → `FrequentTransition(activity=A, skippable=true, selfloop=true)`

Returns a new `PowlModel`.

**Arguments:**
- `model`: A `PowlModel`

**Returns:** Simplified `PowlModel` with frequency annotations

**Examples:**

```javascript
// Optional activity
let model = parse_powl("X(A, tau)");
model = simplify_frequent_transitions(model);
console.log(powl_to_string(model));
// "FrequentTransition(activity=A, min=0, max=1, selfloop=false)"

// Repeated activity
model = parse_powl("*(A, tau)");
model = simplify_frequent_transitions(model);
// Converts to loop frequency annotation
```

---

## Graph Operations

### `transitive_closure(model: &PowlModel, spo_arena_idx: u32) -> Result<BinaryRelationJs, JsValue>`

Compute the transitive closure of the ordering relation of a `StrictPartialOrder` node.

**Arguments:**
- `model`: A `PowlModel`
- `spo_arena_idx`: Arena index of a `StrictPartialOrder` node

**Returns:** `BinaryRelationJs` containing the closure

**Throws:** If the node is not an SPO

**Algorithm:** Floyd-Warshall with bit-packed word-level operations (O(n³) but highly optimized)

**Examples:**

```javascript
const model = parse_powl(
  "PO=(nodes={A, B, C}, order={A-->B, B-->C})"
);
const spo_idx = model.root();

const closure = transitive_closure(model, spo_idx);
console.log(closure.n());  // 3

// Now check implied edges
console.log(closure.is_edge(0, 1));  // true (direct)
console.log(closure.is_edge(1, 2));  // true (direct)
console.log(closure.is_edge(0, 2));  // true (implied by closure)
```

---

### `transitive_reduction(model: &PowlModel, spo_arena_idx: u32) -> Result<BinaryRelationJs, JsValue>`

Compute the transitive reduction of the ordering relation (remove redundant edges while preserving reachability).

**Arguments:**
- `model`: A `PowlModel`
- `spo_arena_idx`: Arena index of an SPO node

**Returns:** `BinaryRelationJs` with minimal edge set

**Throws:** If the node is not an SPO or relation is not irreflexive

**Algorithm:** O(n³) — tests each edge against transitive closure of remainder

**Examples:**

```javascript
const model = parse_powl(
  "PO=(nodes={A, B, C}, order={A-->B, B-->C, A-->C})"
);
const reduction = transitive_reduction(model, model.root());

console.log(reduction.edges_flat());
// [0, 1, 1, 2]  — the A→C edge is redundant and removed
```

---

### `get_order_of(model: &PowlModel, spo_arena_idx: u32) -> Result<BinaryRelationJs, JsValue>`

Return the raw (non-closed, non-reduced) ordering relation of an SPO node.

**Arguments:**
- `model`: A `PowlModel`
- `spo_arena_idx`: Arena index of an SPO

**Returns:** `BinaryRelationJs` with the original edges

**Examples:**

```javascript
const model = parse_powl("PO=(nodes={A, B, C}, order={A-->B, B-->C})");
const raw_order = get_order_of(model, model.root());

console.log(raw_order.n());  // 3
console.log(raw_order.edges_flat());  // [0, 1, 1, 2]
console.log(raw_order.is_transitive());  // true (already minimal)
```

---

## Introspection

### `get_children(model: &PowlModel, arena_idx: u32) -> Vec<u32>`

Return the child arena indices of an SPO or OperatorPOWL node.

**Arguments:**
- `model`: A `PowlModel`
- `arena_idx`: Arena index of a node

**Returns:** Vector of u32 child indices; empty for leaf nodes

**Examples:**

```javascript
const model = parse_powl("X(A, B, C)");
const root_idx = model.root();

const children = get_children(model, root_idx);
console.log(children.length);  // 3 (A, B, C)

// Get the first child (should be transition A)
const first_child = children[0];
console.log(node_to_string(model, first_child));  // "A"
```

---

### `node_to_string(model: &PowlModel, arena_idx: u32) -> String`

Return the string representation of a single node by arena index.

**Arguments:**
- `model`: A `PowlModel`
- `arena_idx`: Arena index of a node

**Returns:** String representation of that node

**Examples:**

```javascript
const model = parse_powl("X(A, X(B, C))");

const root_children = get_children(model, model.root());
for (const child_idx of root_children) {
  console.log(node_to_string(model, child_idx));
}
// Output:
// A
// X ( B, C )
```

---

### `node_info_json(model: &PowlModel, arena_idx: u32) -> String`

Return a JSON string describing the node type, label, and structure.

**Arguments:**
- `model`: A `PowlModel`
- `arena_idx`: Arena index of a node

**Returns:** JSON string with format depending on node type

**JSON Formats:**

```javascript
// Transition
{ "type": "Transition", "label": "A", "id": 0 }

// Silent transition (tau)
{ "type": "Transition", "label": "tau", "id": 1 }

// FrequentTransition
{ "type": "FrequentTransition", "label": "ActivityA", "activity": "ActivityA", "skippable": true, "selfloop": false }

// StrictPartialOrder
{ "type": "StrictPartialOrder", "children": [1, 2], "edges": [[0, 1]] }

// OperatorPowl
{ "type": "OperatorPowl", "operator": "Xor", "children": [1, 2] }
```

**Examples:**

```javascript
const model = parse_powl("X(A, B)");
const root_idx = model.root();

const root_info = JSON.parse(node_info_json(model, root_idx));
console.log(root_info.type);      // "OperatorPowl"
console.log(root_info.operator);  // "Xor"
console.log(root_info.children);  // [1, 2]
```

---

## Conversions

### To Petri Net

#### `to_petri_net(model: &PowlModel) -> Result<PetriNetResult, JsValue>`

Convert a POWL model to a Petri net for conformance checking or simulation.

**Arguments:**
- `model`: A `PowlModel`

**Returns:** `PetriNetResult` containing the net and initial/final markings

**Algorithm:**

1. **Transitions** → Single arc (place → transition → place)
2. **XOR choice** → Split (input → decision transitions) + join (converge to output)
3. **LOOP** → Feedback arc with exit guard transition
4. **PartialOrder** → Synchronization barriers (tau-split / tau-join) with ordering arcs between children

The resulting Petri net can be serialized to JSON for visualization or conformance testing.

**Examples:**

```javascript
const model = parse_powl("PO=(nodes={A, B, C}, order={A-->B, B-->C})");
const pn_result = to_petri_net(model);

const { net, initial_marking, final_marking } = pn_result;
console.log(`Places: ${net.places.length}`);
console.log(`Transitions: ${net.transitions.length}`);
console.log(`Arcs: ${net.arcs.length}`);

// Inspect places
net.places.forEach(place => {
  console.log(`Place: ${place.name}`);
});

// Inspect transitions
net.transitions.forEach(trans => {
  const label = trans.label || "(silent)";
  console.log(`Transition: ${trans.name} [${label}]`);
});

// Inspect arcs
net.arcs.forEach(arc => {
  console.log(`Arc: ${arc.source} → ${arc.target} (weight ${arc.weight})`);
});

// Inspect markings
console.log("Initial marking:", initial_marking);
console.log("Final marking:", final_marking);
```

**Output Types:**

```typescript
type Place = {
  name: string;
}

type Transition = {
  name: string;
  label?: string;  // null for silent transitions
  properties: Record<string, any>;  // e.g., "activity", "skippable"
}

type Arc = {
  source: string;
  target: string;
  weight: number;  // default 1
}

type PetriNet = {
  name: string;
  places: Place[];
  transitions: Transition[];
  arcs: Arc[];
}

type Marking = Record<string, number>;  // place_name -> token_count

type PetriNetResult = {
  net: PetriNet;
  initial_marking: Marking;
  final_marking: Marking;
}
```

---

### To Process Tree

#### `to_process_tree(model: &PowlModel) -> Result<ProcessTreeResult, JsValue>`

Convert a POWL model to a process tree (hierarchical notation with operators).

**Arguments:**
- `model`: A `PowlModel`

**Returns:** `ProcessTreeResult` containing the root process tree node

**Algorithm:**

For `StrictPartialOrder` nodes:
1. Build a directed acyclic graph (DAG) from the partial order
2. Find connected components (undirected view)
3. Compute transitive reduction
4. Assign BFS levels to all nodes
5. Group nodes by level → SEQUENCE of PARALLEL blocks

For other operators, the tree structure is preserved as-is.

**Examples:**

```javascript
const model = parse_powl("X(A, B)");
const pt_result = to_process_tree(model);

const { root } = pt_result;
console.log(root.label);       // null (operator node)
console.log(root.operator);    // "Xor"
console.log(root.children.length);  // 2

// For sequential partial order
const po_model = parse_powl(
  "PO=(nodes={A, B}, order={A-->B})"
);
const po_pt = to_process_tree(po_model);
// Result: SEQUENCE(A, B) or similar hierarchical structure
```

**Output Types:**

```typescript
type Operator = "Sequence" | "Xor" | "Parallel" | "Loop";

type ProcessTree = {
  label?: string;  // null for internal (operator) nodes
  operator?: Operator;  // null for leaf (activity) nodes
  children: ProcessTree[];
}

type ProcessTreeResult = {
  root: ProcessTree;
}
```

---

## Analysis

### Footprints

#### `get_footprints(model: &PowlModel) -> Result<Footprints, JsValue>`

Extract behavioral properties (footprints) of the POWL model.

**Arguments:**
- `model`: A `PowlModel`

**Returns:** `Footprints` object

**Footprints Object:**

```typescript
type Footprints = {
  start_activities: Set<string>;        // Activities that can start a trace
  end_activities: Set<string>;          // Activities that can end a trace
  activities: Set<string>;              // All activities in the model
  activities_always_happening: Set<string>;  // Activities in every execution
  skippable_activities: Set<string>;    // Optional activities
  sequence: Set<[string, string]>;      // Direct precedence pairs (a can precede b)
  parallel: Set<[string, string]>;      // Concurrency pairs (a and b can run together)
  min_trace_length: number;             // Minimum activities in a valid trace
  max_trace_length?: number;            // Maximum (for finite models)
}
```

**Algorithm:**

1. **Single transition**: Start/end = {label}, always_happening = {label} (unless skippable)
2. **XOR choice**: Union of child footprints, all activities optional
3. **LOOP**: Do/redo cycle analysis, redo activity is skippable
4. **StrictPartialOrder**:
   - Compute transitive closure to determine reachability
   - Identify true start/end nodes (no non-skippable predecessors/successors)
   - Detect concurrency from non-ordered pairs in closure
   - Build sequence and parallel relations

**Examples:**

```javascript
const model = parse_powl("PO=(nodes={A, B, C}, order={A-->B, A-->C})");
const footprints = get_footprints(model);

console.log("Start activities:", Array.from(footprints.start_activities));
// ["A"]

console.log("End activities:", Array.from(footprints.end_activities));
// ["B", "C"]

console.log("Parallel pairs:", Array.from(footprints.parallel));
// [["B", "C"], ["C", "B"]]

console.log("Min trace length:", footprints.min_trace_length);
// 3 (A, then B and C in any order)
```

**Complex example with optionality:**

```javascript
const model = parse_powl(
  "PO=(nodes={A, X(B, tau), C}, order={A-->C, A-->X(B, tau), X(B, tau)-->C})"
);
const fp = get_footprints(model);

console.log("Start:", Array.from(fp.start_activities));       // ["A"]
console.log("End:", Array.from(fp.end_activities));           // ["C"]
console.log("Always:", Array.from(fp.activities_always_happening));  // ["A", "C"]
console.log("Skippable:", Array.from(fp.skippable_activities));      // ["B"]
console.log("Min length:", fp.min_trace_length);              // 2 (A, C)
```

---

## Process Discovery

### Alpha+ Miner

#### `discover_petri_net_alpha_plus(log_json: &str) -> Result<String, JsValue>`

Extended Alpha miner that handles loops of length 1 (A→A), loops of length 2 (A→B→A), and non-free-choice constructs.

**Arguments:**
- `log_json`: JSON string of an event log (same format as `parse_csv_log` output)

**Returns:** JSON string of Petri net result

**Throws:** JavaScript `Error` if parsing fails

**Examples:**

```javascript
// Loop of length 1 (self-loop)
const log1 = [
  { case_id: "1", activity: "A" },
  { case_id: "1", activity: "A" },
  { case_id: "1", activity: "B" }
];
const net1 = discover_petri_net_alpha_plus(JSON.stringify(log1));
console.log(net1);  // Petri net with loop transition

// Loop of length 2 (short loop)
const log2 = [
  { case_id: "1", activity: "A" },
  { case_id: "1", activity: "B" },
  { case_id: "1", activity: "A" },
  { case_id: "1", activity: "C" }
];
const net2 = discover_petri_net_alpha_plus(JSON.stringify(log2));
console.log(net2);  // Petri net with A→B→A loop structure
```

---

### Prefix Tree Discovery

#### `discover_prefix_tree(log_json: &str, max_path_length: Option<usize>) -> Result<String, JsValue>`

Build a trie (prefix tree) of all trace prefixes in the event log.

**Arguments:**
- `log_json`: JSON string of an event log
- `max_path_length`: Optional maximum depth for the trie (None = unlimited)

**Returns:** JSON string of TrieNode structure

**Throws:** JavaScript `Error` if parsing fails

**TrieNode Structure:**

```typescript
type TrieNode = {
  label: string | null;        // Activity name (null for root)
  parent: number | null;       // Parent node index (null for root)
  children: number[];          // Child node indices
  is_final: boolean;           // True if this node is end of a trace
  depth: number;               // Depth in tree
}
```

**Examples:**

```javascript
const log = [
  { case_id: "1", activity: "A" },
  { case_id: "1", activity: "B" },
  { case_id: "1", activity: "C" },
  { case_id: "2", activity: "A" },
  { case_id: "2", activity: "C" }
];

// Build full trie
const trie = discover_prefix_tree(JSON.stringify(log), null);
console.log(trie);

// Build trie with max depth of 2
const shallow_trie = discover_prefix_tree(JSON.stringify(log), 2);
console.log(shallow_trie);
```

---

### DFG Typed

#### `discover_dfg_typed(log_json: &str) -> Result<String, JsValue>`

Discover a Directly-Follows Graph with structured return format.

**Arguments:**
- `log_json`: JSON string of an event log

**Returns:** JSON string of DFGTyped object

**DFGTyped Structure:**

```typescript
type DFGTyped = {
  graph: Array<[string, string, number]>;  // (from, to, frequency) triples
  start_activities: Array<[string, number]>;  // (activity, count)
  end_activities: Array<[string, number]>;    // (activity, count)
}
```

**Examples:**

```javascript
const log = [
  { case_id: "1", activity: "A" },
  { case_id: "1", activity: "B" },
  { case_id: "1", activity: "C" },
  { case_id: "2", activity: "A" },
  { case_id: "2", activity: "B" }
];

const dfg = discover_dfg_typed(JSON.stringify(log));
const result = JSON.parse(dfg);

console.log("Graph edges:", result.graph);
// [["A", "B", 2], ["B", "C", 1]]

console.log("Start activities:", result.start_activities);
// [["A", 2]]

console.log("End activities:", result.end_activities);
// [["C", 1], ["B", 1]]
```

---

## Object-Centric Event Logs (OCEL)

### Parse OCEL JSON

#### `parse_ocel_json(json: &str) -> Result<String, JsValue>`

Parse an OCEL (Object-Centric Event Log) in JSON-OCEL 1.0/2.0 format.

**Arguments:**
- `json`: JSON-OCEL string

**Returns:** JSON string of OCEL object

**OCEL Structure:**

```typescript
type OCEL = {
  events: OCELEvent[];
  objects: OCELObject[];
  relations: OCELRelation[];
  globals: Record<string, any>;
  o2o: Record<string, any>;      // Object-to-object relations (optional)
  e2e: Record<string, any>;      // Event-to-event relations (optional)
}

type OCELEvent = {
  id: string;
  activity: string;
  timestamp: string | null;
  attributes: Record<string, any>;
}

type OCELObject = {
  id: string;
  object_type: string;
  attributes: Record<string, any>;
}

type OCELRelation = {
  event_id: string;
  object_id: string;
}
```

**Examples:**

```javascript
const ocelJson = JSON.stringify({
  objectTypes: ["order", "item"],
  eventTypes: ["Create Order"],
  objects: [
    { id: "o1", type: "order" },
    { id: "i1", type: "item" }
  ],
  events: [
    {
      id: "e1",
      type: "Create Order",
      timestamp: "2024-01-01T00:00:00Z",
      objects: ["o1", "i1"]
    }
  ]
});

const ocel = parse_ocel_json(ocelJson);
const result = JSON.parse(ocel);
console.log("Events:", result.events.length);
console.log("Objects:", result.objects.length);
```

---

### OCEL Flattening

#### `ocel_flatten_by_object_type(ocel_json: &str, object_type: &str) -> Result<String, JsValue>`

Flatten an OCEL to a traditional event log by expanding events for a specific object type.

**Arguments:**
- `ocel_json`: JSON string of OCEL object (from `parse_ocel_json`)
- `object_type`: Object type to flatten by (e.g., "order", "item")

**Returns:** JSON string of traditional event log

**Examples:**

```javascript
const ocel = parse_ocel_json(ocelJson);

// Flatten by "order" object type
const orderLog = ocel_flatten_by_object_type(ocel, "order");
const result = JSON.parse(orderLog);

console.log("Traces:", result.length);
// Each trace represents the lifecycle of one order object
```

---

### OCEL ETOT Discovery

#### `discover_ocel_etot(ocel_json: &str) -> Result<String, JsValue>`

Discover the Event-Type / Object-Type graph from an OCEL.

**Arguments:**
- `ocel_json`: JSON string of OCEL object

**Returns:** JSON string of ETOT graph with edge frequencies

**Examples:**

```javascript
const ocel = parse_ocel_json(ocelJson);
const etot = discover_ocel_etot(ocel);
const result = JSON.parse(etot);

console.log("ETOT edges:", result.edges);
// [["Create Order", "order", 5], ["Create Order", "item", 10]]
```

---

### OCEL Summary

#### `ocel_get_summary(ocel_json: &str) -> Result<String, JsValue>`

Get summary statistics for an OCEL.

**Arguments:**
- `ocel_json`: JSON string of OCEL object

**Returns:** JSON string with summary statistics

**Examples:**

```javascript
const ocel = parse_ocel_json(ocelJson);
const summary = ocel_get_summary(ocel);
const result = JSON.parse(summary);

console.log("Total events:", result.total_events);
console.log("Total objects:", result.total_objects);
console.log("Event types:", result.event_types);
console.log("Object types:", result.object_types);
```

---

### OCEL Object Types

#### `ocel_get_object_types(ocel_json: &str) -> Result<String, JsValue>`

List all object types in an OCEL.

**Arguments:**
- `ocel_json`: JSON string of OCEL object

**Returns:** JSON array of object type strings

**Examples:**

```javascript
const ocel = parse_ocel_json(ocelJson);
const types = ocel_get_object_types(ocel);
const result = JSON.parse(types);

console.log("Object types:", result);
// ["order", "item", "customer"]
```

---

### OCEL Event Types

#### `ocel_get_event_types(ocel_json: &str) -> Result<String, JsValue>`

List all event types in an OCEL.

**Arguments:**
- `ocel_json`: JSON string of OCEL object

**Returns:** JSON array of event type strings

**Examples:**

```javascript
const ocel = parse_ocel_json(ocelJson);
const types = ocel_get_event_types(ocel);
const result = JSON.parse(types);

console.log("Event types:", result);
// ["Create Order", "Pay Order", "Ship Order"]
```

---

## Type Mappings

### POWL Node Types (Internal)

```rust
enum PowlNode {
  Transition {
    label: Option<String>,
    id: u32,
  },
  FrequentTransition {
    label: String,
    activity: String,
    skippable: bool,
    selfloop: bool,
  },
  StrictPartialOrder {
    children: Vec<u32>,
    order: BinaryRelation,
  },
  OperatorPowl {
    operator: Operator,
    children: Vec<u32>,
  },
}

enum Operator {
  Xor,
  Loop,
  PartialOrder,
}
```

---

## Error Handling

All functions that return `Result<T, JsValue>` throw JavaScript `Error` objects on failure.

**Common errors:**

```javascript
try {
  parse_powl("PO=(nodes={A}, order={A-->A})");
  validate_partial_orders(model);  // Will catch self-loop
} catch (e) {
  console.error("Error:", e.message);
  // "Validation error: ..."
}

try {
  transitive_reduction(model, 999);  // Invalid arena index
} catch (e) {
  console.error("Error:", e.message);
  // "node 999 is not a StrictPartialOrder"
}
```

---

## Performance Notes

- **Parsing**: O(n) where n = string length; no copies
- **Transitive closure**: O(k³) where k = number of children in SPO; bit-packed operations
- **Transitive reduction**: O(k³) after closure; tests each edge against reduced closure
- **Simplification**: O(n) where n = total nodes; recursive single pass
- **Footprints**: O(k² + k³) for partial orders; closure computation dominates
- **Petri net conversion**: O(n + m) where n = nodes, m = ordering edges
- **Process tree conversion**: O(n²) DAG processing; topological sort + component detection

For typical models (< 100 nodes), all operations complete in < 10 ms in the browser.

---

## See Also

- [Tutorial](./tutorial.md) — Learning-focused guide with examples
- `docs/` — Additional documentation
- Python `pm4py` library — Reference implementation for algorithms

