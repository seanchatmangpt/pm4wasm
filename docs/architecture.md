# POWL v2 Rust/WASM Architecture

This document describes the architecture of the POWL v2 Rust/WebAssembly implementation, including module organization, data structures, and key algorithms.

## Table of Contents

1. [Overview](#overview)
2. [Module Organization](#module-organization)
3. [Core Data Structures](#core-data-structures)
4. [Parsing Pipeline](#parsing-pipeline)
5. [Conversion Algorithms](#conversion-algorithms)
6. [Analysis Operations](#analysis-operations)
7. [Memory Layout](#memory-layout)
8. [WASM Bindings](#wasm-bindings)

---

## Overview

The POWL v2 Rust/WASM crate is organized as a library crate (`pm4wasm`) that compiles to WebAssembly via `wasm-bindgen`. The architecture follows these principles:

- **Arena-based storage** — All nodes in a flat `Vec<PowlNode>`, referenced by `u32` index
- **Bit-packed relations** — Binary relations use `u64` bitsets for efficient set operations
- **Zero-copy parsing** — String parsing directly into arena without intermediate allocations
- **Functional transformations** — Most operations return new models rather than mutating in place

```
┌─────────────────────────────────────────────────────────────┐
│                     JavaScript / Browser                     │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐     │
│  │ parse_powl() │  │ to_petri_net │  │get_footprints│     │
│  └──────┬───────┘  └──────┬───────┘  └──────┬───────┘     │
└─────────┼──────────────────┼──────────────────┼────────────┘
          │                  │                  │
┌─────────┼──────────────────┼──────────────────┼────────────┐
│         ▼                  ▼                  ▼            │
│  ┌──────────────────────────────────────────────────────┐  │
│  │              wasm-bindgen FFI Layer                   │  │
│  │  (PowlModel, BinaryRelationJs, PetriNetResult, ...) │  │
│  └──────────────────────────────────────────────────────┘  │
│                                                             │
│  ┌──────────────────────────────────────────────────────┐  │
│  │                    Rust Core                          │  │
│  │  ┌─────────┐ ┌─────────┐ ┌─────────┐ ┌──────────┐   │  │
│  │  │ parser  │ │  powl   │ │ binary  │ │  foot    │   │  │
│  │  │         │ │  types  │ │  rel    │ │  prints  │   │  │
│  │  └─────────┘ └─────────┘ └─────────┘ └──────────┘   │  │
│  │  ┌─────────┐ ┌─────────┐ ┌─────────┐ ┌──────────┐   │  │
│  │  │  petri  │ │ process │ │  event  │ │  token   │   │  │
│  │  │   net   │ │  tree   │ │   log   │ │  replay  │   │  │
│  │  └─────────┘ └─────────┘ └─────────┘ └──────────┘   │  │
│  └──────────────────────────────────────────────────────┘  │
│                                                             │
└─────────────────────────────────────────────────────────────┘
```

---

## Module Organization

### Core Modules

#### `src/lib.rs`
- **Purpose:** wasm-bindgen entry point, FFI layer
- **Exports:** `PowlModel`, `BinaryRelationJs`, `parse_powl`, etc.
- **Responsibilities:**
  - Wrap Rust types in wasm-bindgen-compatible structs
  - Handle JavaScript-Rust type conversions
  - Provide ergonomic API for JavaScript consumers

#### `src/powl.rs`
- **Purpose:** POWL v2 node type definitions
- **Key Types:**
  - `PowlNode` — Enum of all node types (Transition, Operator, SPO)
  - `PowlArena` — Flat storage arena (`Vec<PowlNode>`)
  - `Operator` — Control flow operators (Xor, Loop, Sequence, etc.)
- **Responsibilities:**
  - Define the in-memory representation of POWL models
  - Provide arena operations (add node, get node, etc.)
  - Implement tree traversal methods

#### `src/parser.rs`
- **Purpose:** Parse POWL model strings (same format as Python `__repr__`)
- **Grammar:**
  ```
  model ::= Transition | FrequentTransition | Operator | StrictPartialOrder
  Transition ::= label (e.g., "A", "Submit Claim")
  Operator ::= X(A, B) | *(A, B) | +(A, B) | →(A, B)
  StrictPartialOrder ::= PO=(nodes={...}, order={...})
  ```
- **Responsibilities:**
  - Tokenize input string
  - Build parse tree according to grammar
  - Populate arena with parsed nodes
  - Report syntax errors with line/column information

#### `src/binary_relation.rs`
- **Purpose:** Bit-packed adjacency matrix for partial orders
- **Key Types:**
  - `BinaryRelation` — Adjacency matrix with `u64` bitset rows
  - `BitMatrix` — 2D bitset for O(1) edge queries
- **Algorithms:**
  - **Warshall's algorithm** — Transitive closure in O(k³)
  - **Transitive reduction** — Remove redundant edges in O(k³)
  - **Reachability** — O(1) queries with closure
- **Optimizations:**
  - Bit-level parallelism (64 edges per `u64`)
  - Cache-friendly row-major layout
  - In-place operations where possible

### Conversion Modules

#### `src/conversion/to_petri_net.rs`
- **Purpose:** Convert POWL model to Petri net (Place/Transition/Arc)
- **Algorithm:**
  1. **Transition** → Single place-transition-place triple
  2. **XOR choice** → Split (input → decision transitions) + join
  3. **LOOP** → Feedback arc with exit guard transition
  4. **PartialOrder** → Synchronization barriers (tau-split/tau-join) + ordering arcs
- **Output:** `PetriNetResult` with net, initial marking, final marking
- **Responsibilities:**
  - Generate unique IDs for places/transitions
  - Compute initial/final markings from model structure
  - Preserve semantics (same traces as POWL model)

#### `src/conversion/to_process_tree.rs`
- **Purpose:** Convert POWL model to process tree (hierarchical operators)
- **Algorithm:**
  1. **StrictPartialOrder** → Build DAG, compute levels, group by level
  2. **Other operators** — Preserve tree structure as-is
- **Output:** `ProcessTreeResult` with root node
- **Responsibilities:**
  - Detect concurrency (non-ordered pairs in transitive closure)
  - Build SEQUENCE of PARALLEL blocks from SPO levels
  - Assign operator labels (Sequence, Xor, Parallel, Loop)

#### `src/conversion/to_bpmn.rs`
- **Purpose:** Convert POWL model to BPMN 2.0 XML
- **Algorithm:**
  1. Convert to Petri net first
  2. Map Petri net elements to BPMN elements
  3. Generate XML with proper namespaces
- **Output:** BPMN 2.0 XML string
- **Responsibilities:**
  - Generate valid BPMN 2.0 XML
  - Handle gateways (exclusive, parallel, event-based)
  - Preserve layout hints (if available)

### Analysis Modules

#### `src/footprints.rs`
- **Purpose:** Extract behavioral signatures (footprints) from POWL models
- **Key Types:**
  - `Footprints` — Start/end activities, sequence/parallel relations
- **Algorithm:**
  1. **Single transition** — Start/end = {label}, always_happening = {label}
  2. **XOR choice** — Union of child footprints, all optional
  3. **LOOP** — Do/redo cycle analysis, redo is skippable
  4. **StrictPartialOrder** — Compute closure, detect start/end nodes, find concurrency
- **Responsibilities:**
  - Compute start_activities (no predecessors, not skippable)
  - Compute end_activities (no successors, not skippable)
  - Compute sequence relation (direct precedence pairs)
  - Compute parallel relation (non-ordered pairs in closure)
  - Compute min/max trace lengths

#### `src/conformance/token_replay.rs`
- **Purpose:** Token-based replay conformance checking
- **Algorithm:**
  1. Convert POWL to Petri net
  2. For each trace in event log:
     - Initialize marking (initial marking)
     - For each event: fire transition if enabled, else record deviation
     - Check if final marking reached
  3. Aggregate statistics (fitness, perfectly fitting traces, etc.)
- **Output:** `ConformanceResult` with fitness percentage, deviations
- **Responsibilities:**
  - Simulate token flow through Petri net
  - Detect missing/remaining tokens
  - Compute trace-level and aggregate fitness

#### `src/event_log.rs`
- **Purpose:** Parse XES and CSV event logs
- **Key Types:**
  - `EventLog` — Container for traces (list of events)
  - `Event` — Single event with case_id, activity, timestamp
- **Responsibilities:**
  - Parse XES XML (streaming, SAX-style)
  - Parse CSV (flexible column mapping)
  - Validate event log structure
  - Handle missing/optional fields

### Utility Modules

#### `src/algorithms/simplify.rs`
- **Purpose:** Structural normalization of POWL models
- **Transformations:**
  - Flatten nested XOR/LOOP operators
  - Merge redundant patterns
  - Inline single-child sub-SPOs
  - Convert XOR(A, tau) → FrequentTransition(A, skippable=true)
  - Convert LOOP(A, tau) → FrequentTransition(A, selfloop=true)

#### `src/algorithms/transitive.rs`
- **Purpose:** Transitive closure and reduction algorithms
- **Algorithms:**
  - Warshall's algorithm (closure)
  - Reduction via closure subtraction
- **Responsibilities:**
  - In-place closure computation
  - Redundant edge detection
  - Graph property validation (irreflexive, transitive)

#### `src/complexity.rs`
- **Purpose:** Compute complexity metrics for POWL models
- **Metrics:**
  - Node count (by type)
  - Operator nesting depth
  - Partial order density
  - Control flow complexity

#### `src/diff.rs`
- **Purpose:** Behavioral diff between two POWL models
- **Comparison:**
  - Added/removed activities
  - Changed control flow
  - Parallelism differences
  - Conformance delta

#### `src/streaming.rs`
- **Purpose:** Streaming drift detection with EWMA smoothing
- **Algorithm:**
  - Incrementally update discovered model as events arrive
  - Detect concept drift via statistical tests
  - EWMA (Exponentially Weighted Moving Average) smoothing

#### `src/trie.rs`
- **Purpose:** Trie (prefix tree) data structure for log analysis
- **Key Types:**
  - `TrieNode` — Node with label, parent, children, is_final, depth
- **Responsibilities:**
  - Build prefix trees from event log traces
  - Support efficient prefix-based queries
  - Track trace endings via is_final flag

---

### Discovery Modules

#### `src/discovery/alpha_miner.rs`
- **Purpose:** Basic Alpha miner for process discovery
- **Algorithm:**
  - Extract causal relations from event log
  - Build place/transition net from relations
  - Handle basic control flow patterns

#### `src/discovery/alpha_plus_miner.rs`
- **Purpose:** Extended Alpha miner handling complex loops
- **Algorithm:**
  - Preprocessing: Identify loop-1 activities (A→A), build A/B dictionaries
  - Get relations: Extended causal/parallel detection (loops of length 2: A→B→A)
  - Processing: Apply Alpha miner with extended relation handling
  - Postprocessing: Re-insert loop transitions with proper arcs
- **Handles:**
  - Loops of length 1 (self-loops)
  - Loops of length 2 (short loops)
  - Non-free-choice constructs

#### `src/discovery/inductive_miner.rs`
- **Purpose:** Robust process tree discovery
- **Algorithm:**
  - Recursively split logs based on cut detection
  - Build process tree with operators (Sequence, Xor, Parallel, Loop)
  - Handle invisible and duplicate tasks

#### `src/discovery/dfg.rs`
- **Purpose:** Directly-Follows Graph discovery
- **Key Types:**
  - `DFGTyped` — Structured DFG with (from, to, frequency) triples
- **Responsibilities:**
  - Extract activity succession from logs
  - Compute start/end activities
  - Return typed DFG object for structured access

---

### OCEL Modules

#### `src/ocel/mod.rs`
- **Purpose:** Object-Centric Event Log support
- **Key Types:**
  - `OCEL` — Events, objects, relations, globals
  - `OCELEvent` — Event with id, activity, timestamp
  - `OCELObject` — Object with id, object_type
  - `OCELRelation` — Event-object mapping
- **Responsibilities:**
  - Represent OCEL data structures
  - Provide OCEL-specific operations

#### `src/ocel/jsonocel.rs`
- **Purpose:** Parse OCEL JSON format (JSON-OCEL 1.0/2.0)
- **Algorithm:**
  - Parse JSON-OCEL structure
  - Extract events, objects, relations
  - Handle optional o2o/e2e fields
- **Responsibilities:**
  - Validate OCEL JSON format
  - Build OCEL in-memory representation

#### `src/ocel/flattening.rs`
- **Purpose:** Flatten OCEL to traditional event logs
- **Algorithm:**
  - Select object type to flatten by
  - Expand events to object-centric traces
  - Preserve event ordering per object
- **Responsibilities:**
  - Convert OCEL to EventLog by object type
  - Enable traditional process mining on OCEL data

#### `src/ocel/etot.rs`
- **Purpose:** Event-Type / Object-Type graph discovery
- **Algorithm:**
  - Extract event types and object types from OCEL
  - Build bipartite graph of connections
  - Compute edge frequencies
- **Responsibilities:**
  - Discover ETOT structure
  - Return typed graph representation

---

### Transformation Modules

#### `src/transformation/mod.rs`
- **Purpose:** Log transformation operations
- **Modules:**
  - `log_to_trie.rs` — Prefix tree discovery from event logs

#### `src/transformation/log_to_trie.rs`
- **Purpose:** Build trie (prefix tree) from event log
- **Algorithm:**
  - Get all variants from log
  - For each variant, walk down trie creating nodes as needed
  - Mark final nodes for complete traces
- **Parameters:**
  - `max_path_length` — Optional limit on trie depth
- **Responsibilities:**
  - Efficient prefix-based log representation
  - Support for trace pattern analysis

---

## Core Data Structures

### PowlNode

All POWL nodes are represented by the `PowlNode` enum:

```rust
pub enum PowlNode {
    Transition {
        label: Option<String>,  // None for silent (tau)
        id: u32,
    },
    FrequentTransition {
        label: String,
        activity: String,
        skippable: bool,        // min=0 if true
        selfloop: bool,         // can repeat
    },
    StrictPartialOrder {
        children: Vec<u32>,     // Arena indices of child nodes
        order: BinaryRelation,  // Ordering relation (bit matrix)
    },
    OperatorPowl {
        operator: Operator,
        children: Vec<u32>,
    },
}
```

### BinaryRelation

Bit-packed adjacency matrix for partial orders:

```rust
pub struct BinaryRelation {
    n: usize,                   // Number of nodes
    rows: Vec<u64>,             // Bitset rows (n * n bits, packed)
}

impl BinaryRelation {
    // O(1) edge query
    pub fn is_edge(&self, i: usize, j: usize) -> bool {
        let word_idx = i * self.n + j / 64;
        let bit_mask = 1u64 << (j % 64);
        (self.rows[word_idx] & bit_mask) != 0
    }

    // O(k³) transitive closure (Warshall)
    pub fn transitive_closure(&mut self) {
        for k in 0..self.n {
            for i in 0..self.n {
                if self.is_edge(i, k) {
                    for j in 0..self.n {
                        if self.is_edge(k, j) {
                            self.add_edge(i, j);
                        }
                    }
                }
            }
        }
    }
}
```

### PowlArena

Flat storage arena for all nodes:

```rust
pub struct PowlArena {
    nodes: Vec<PowlNode>,
}

impl PowlArena {
    pub fn add_node(&mut self, node: PowlNode) -> u32 {
        let idx = self.nodes.len() as u32;
        self.nodes.push(node);
        idx  // Return arena index
    }

    pub fn get_node(&self, idx: u32) -> &PowlNode {
        &self.nodes[idx as usize]
    }
}
```

---

## Parsing Pipeline

The parser follows a recursive descent approach:

```
Input String
    ↓
Tokenizer (chars → tokens)
    ↓
Parser (tokens → AST)
    ↓
Arena Builder (AST → PowlArena)
    ↓
PowlModel (arena + root index)
```

### Example

Input: `"X(A, B)"`

1. **Tokenizer:** `[X, (, A, comma, B, )]`
2. **Parser:** `OperatorPowl { operator: Xor, children: [1, 2] }`
3. **Arena Builder:**
   - Add Transition("A") → index 1
   - Add Transition("B") → index 2
   - Add OperatorPowl → index 3
4. **PowlModel:** arena = [3 nodes], root = 3

---

## Conversion Algorithms

### POWL → Petri Net

Each POWL node maps to a Petri net fragment:

```
Transition "A":
  p1 → A → p2

XOR(A, B):
  p1 → A → p3
  p1 → B → p3
  (choice: fire A or B, not both)

LOOP(A, B):
  p1 → A → p2
  p2 → B → p1
  p2 → exit → p3
  (repeat B→A or exit)

PO=(nodes={A, B}, order={A-->B}):
  p1 → tau_split → p2
  p2 → A → p3
  p2 → B → p4
  p3 → tau_join → p5
  p4 → tau_join → p5
  (A and B synchronize via tau_split/tau_join)
```

### POWL → Process Tree

StrictPartialOrder nodes undergo level-based grouping:

```
Input: PO=(nodes={A, B, C}, order={A-->B, A-->C})

1. Compute transitive closure:
   A→B, A→C, B→?, C→?

2. Find start nodes (no predecessors): {A}

3. BFS traversal to assign levels:
   Level 0: {A}
   Level 1: {B, C}  (both reachable from A in 1 step)

4. Group by level:
   SEQUENCE(PARALLEL(A), PARALLEL(B, C))

Output: Process tree with SEQUENCE root
```

---

## Analysis Operations

### Footprint Extraction

For each POWL node type:

| Node Type | Start Activities | End Activities | Sequence | Parallel |
|-----------|------------------|----------------|----------|----------|
| **Transition** | {label} | {label} | {} | {} |
| **XOR(A, B)** | start(A) ∪ start(B) | end(A) ∪ end(B) | seq(A) ∪ seq(B) | {} |
| **LOOP(A, B)** | start(A) | end(A) | seq(A) ∪ seq(B) ∪ {(B, A)} | {} |
| **PO** | compute from closure | compute from closure | direct edges | non-ordered pairs |

### Token-Replay Conformance

For each trace in event log:

```
Input trace: [A, B, C]

1. Initialize marking (initial_marking)

2. For each event:
   - Find transition matching event label
   - Check if transition is enabled (enough tokens)
   - Fire transition (consume tokens, produce tokens)
   - If not enabled: record deviation (missing token)

3. After all events:
   - Check if final_marking reached
   - Record deviation (remaining tokens)

4. Aggregate:
   - fitness = (consumed + produced + remaining) / (consumed + produced + missing)
   - perfectly_fitting_traces = count(traces with fitness = 1.0)
```

---

## Memory Layout

### WASM Linear Memory

```
┌─────────────────────────────────────────────────┐
│                  JavaScript                     │
│  (access via ArrayBuffer, typed arrays)         │
└────────────────────┬────────────────────────────┘
                     │
┌────────────────────┼────────────────────────────┐
│      WASM Linear Memory (1 GB max)              │
│  ┌──────────────────────────────────────────┐  │
│  │  PowlArena (Vec<PowlNode>)               │  │
│  │  ┌──────┐ ┌──────┐ ┌──────┐ ┌──────┐   │  │
│  │  │ Node0│ │ Node1│ │ Node2│ │ Node3│   │  │
│  │  └──────┘ └──────┘ └──────┘ └──────┘   │  │
│  └──────────────────────────────────────────┘  │
│  ┌──────────────────────────────────────────┐  │
│  │  BinaryRelation (Vec<u64>)               │  │
│  │  ┌─────┐ ┌─────┐ ┌─────┐ ┌─────┐       │  │
│  │  │row0 │ │row1 │ │row2 │ │row3 │ ...   │  │
│  │  └─────┘ └─────┘ └─────┘ └─────┘       │  │
│  └──────────────────────────────────────────┘  │
│  ┌──────────────────────────────────────────┐  │
│  │  String Data (UTF-8)                     │  │
│  │  "A" "B" "C" "Submit Claim" ...         │  │
│  └──────────────────────────────────────────┘  │
└─────────────────────────────────────────────────┘
```

### JavaScript Interop

wasm-bindgen generates bindings for zero-copy access:

```rust
#[wasm_bindgen]
pub struct PowlModel {
    arena: PowlArena,
    root: u32,
}

#[wasm_bindgen]
impl PowlModel {
    // Returns a *view* into WASM memory (no copy)
    pub fn activities(&self) -> JsIterator {
        // Iterate arena, collect labels, return JS iterator
    }
}
```

JavaScript side:
```typescript
const model = parse_powl("X(A, B)");
for (const activity of model.activities()) {
    console.log(activity);  // "A", "B"
}
```

---

## WASM Bindings

### Exported Types

| Rust Type | wasm-bindgen Wrapper | JavaScript Type |
|-----------|----------------------|-----------------|
| `PowlArena` | `PowlModel` | `Object` (opaque handle) |
| `BinaryRelation` | `BinaryRelationJs` | `Object` with methods |
| `PetriNetResult` | `PetriNetResult` | `Object` with net/markings |
| `Footprints` | `Footprints` | `Object` with Sets/Maps |

### Exported Functions

```rust
#[wasm_bindgen]
pub fn parse_powl(s: &str) -> Result<PowlModel, JsValue>;

#[wasm_bindgen]
pub fn powl_to_string(model: &PowlModel) -> String;

#[wasm_bindgen]
pub fn validate_partial_orders(model: &PowlModel) -> Result<(), JsValue>;

#[wasm_bindgen]
pub fn transitive_closure(model: &PowlModel, idx: u32) -> Result<BinaryRelationJs, JsValue>;

#[wasm_bindgen]
pub fn to_petri_net(model: &PowlModel) -> Result<PetriNetResult, JsValue>;

#[wasm_bindgen]
pub fn get_footprints(model: &PowlModel) -> Result<Footprints, JsValue>;
```

### Error Handling

All `Result<T, JsValue>` functions throw JavaScript `Error` on failure:

```rust
#[wasm_bindgen]
pub fn parse_powl(s: &str) -> Result<PowlModel, JsValue> {
    if let Err(e) = parse_powl_model_string(s, &mut arena) {
        return Err(JsValue::from_str(&format!("Parse error: {}", e)));
    }
    Ok(model)
}
```

JavaScript side:
```typescript
try {
    const model = parse_powl("invalid{syntax");
} catch (e) {
    console.error(e.message);  // "Parse error: ..."
}
```

---

## Performance Characteristics

| Operation | Time Complexity | Space Complexity | Notes |
|-----------|----------------|------------------|-------|
| **Parsing** | O(n) | O(n) | n = string length |
| **Transitive closure** | O(k³) | O(k²) | k = children in SPO |
| **Transitive reduction** | O(k³) | O(k²) | After closure |
| **Footprints** | O(k² + k³) | O(k²) | Closure dominates |
| **Petri net conversion** | O(n + m) | O(n + m) | n = nodes, m = edges |
| **Process tree conversion** | O(n²) | O(n) | DAG processing |
| **Token replay** | O(t × e) | O(p + t) | t = traces, e = events, p = places |

For typical models (< 100 nodes, < 1000 traces), all operations complete in < 100 ms in the browser.

---

## See Also

- [API Reference](./reference.md) — Complete API documentation
- [Tutorial](./tutorial.md) — Getting started guide
- [Vision 2030](./vision-2030.md) — Future roadmap
