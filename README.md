# POWL v2 Rust/WebAssembly

Browser-native process mining powered by Rust and WebAssembly. A WASM port of [pm4py](https://github.com/pm4py/pm4py-core) bringing core process mining algorithms to the browser with LLM integration for natural language process discovery.

[![Rust](https://img.shields.io/badge/rust-1.70%2B-orange.svg)](https://www.rust-lang.org)
[![wasm-bindgen](https://img.shields.io/badge/wasm--bindgen-0.2%2B-blue.svg)](https://rustwasm.github.io/wasm-bindgen/)
[![License](https://img.shields.io/badge/license-Apache--2.0-blue.svg)](./LICENSE)
[![npm](https://img.shields.io/npm/v/pm4wasm)](https://www.npmjs.com/package/pm4wasm)

## What is POWL?

POWL (Partially Ordered Workflow Language) is a process model notation that combines:

- **Activities** — Transitions with labels (e.g., "A", "Submit Claim", "Payment")
- **Control Flow Operators** — XOR (choice), LOOP (repetition), SEQUENCE, PARALLEL
- **Partial Orders** — Concurrency constraints (activities with no ordering relation run in parallel)

Example model:
```
PO=(nodes={A, X(B, C), D}, order={A-->X(B,C), X(B,C)-->D})
```
Translation: Execute A, then choose between B or C, then execute D.

## Features

### Core Process Mining
- ✅ **Parse POWL models** — Same format as Python pm4py `__repr__`
- ✅ **Validate partial orders** — Irreflexive, transitive, acyclic
- ✅ **Convert to Petri nets** — For conformance checking and simulation
- ✅ **Convert to process trees** — Hierarchical operator notation
- ✅ **Extract footprints** — Behavioral signatures (start/end activities, sequence/parallel relations)
- ✅ **Simplify models** — Flatten nested operators, normalize structure
- ✅ **Conformance checking** — Token-replay fitness against event logs
- ✅ **Event log parsing** — XES and CSV formats
- ✅ **100% browser-native** — No server, no upload, no privacy risk

### Process Discovery
- ✅ **Alpha miner** — Basic process discovery from event logs
- ✅ **Alpha+ miner** — Extended Alpha miner handling loops of length 1 (A→A) and length 2 (A→B→A), plus non-free-choice constructs
- ✅ **Inductive miner** — Robust discovery of process trees
- ✅ **DFG discovery** — Directly-Follows Graph extraction
- ✅ **DFG typed** — Structured DFG object with (from, to, frequency) triples
- ✅ **Prefix tree discovery** — Trie (prefix tree) data structure with optional max_path_length parameter

### Object-Centric Event Logs (OCEL)
- ✅ **Parse OCEL JSON** — JSON-OCEL 1.0/2.0 format support
- ✅ **OCEL flattening** — Flatten OCEL to traditional EventLog by object type
- ✅ **ETOT discovery** — Event-Type / Object-Type graph discovery
- ✅ **OCEL statistics** — Event/object counts, types, and summaries

### Code Generation
- ✅ **BPMN export** — Convert POWL models to BPMN 2.0 XML
- ✅ **PNML export** — Petri Net Markup Language format
- ✅ **Process tree export** — PTML format for process trees
- ✅ **YAWL export** — YAWL v6 XML format

## Quick Start

### Installation

```bash
npm install pm4wasm
```

### Basic Usage

```typescript
import { Powl } from 'pm4wasm';

// Initialize WASM module (one-time)
const powl = await Powl.init();

// Parse a POWL model string
const model = powl.parse('PO=(nodes={A, B, C}, order={A-->B, B-->C})');

// Validate the model (throws if invalid)
model.validate();

// Get canonical string representation
console.log(model.toString());  // "PO=(nodes={ A, B, C }, order={ A-->B, B-->C })"

// List all activities
console.log([...model.activities()]);  // ["A", "B", "C"]

// Convert to Petri net
const petriNet = model.toPetriNet();
console.log(`Places: ${petriNet.net.places.length}`);
console.log(`Transitions: ${petriNet.net.transitions.length}`);

// Parse event log from CSV
const log = powl.parseCsv(
  'case_id,activity\n1,A\n1,B\n1,C\n2,A\n2,C\n'
);

// Check conformance
const fitness = powl.conformance(model, log);
console.log(`Fitness: ${(fitness.percentage * 100).toFixed(1)}%`);
console.log(`Perfectly fitting traces: ${fitness.perfectly_fitting_traces}`);

// Filter traces by fitness threshold
const goodTraces = powl.filterByFitness(model, log, 0.8);
```

## Build from Source

### Prerequisites

- Rust 1.70+ with `wasm32-unknown-unknown` target
- [wasm-pack](https://rustwasm.github.io/wasm-pack/)
- Node.js 18+ (for JavaScript/TypeScript client)

### Build WASM Module

```bash
# Install wasm-pack (once)
curl https://rustwasm.github.io/wasm-pack/installer/init.sh -sSf | sh

# Build WASM
cd pm4wasm
wasm-pack build --target bundler --release
```

Output: `pkg/pm4wasm.js` + `pkg/pm4wasm_bg.wasm`

### Build TypeScript Client

```bash
cd js/
npm install
npm run build:wasm    # Build WASM via wasm-pack
npm run build:ts      # Build TypeScript bundle
```

### Development Server

```bash
cd js/
npm run dev           # Start dev server at http://localhost:5173
npm run demo          # Demo page with live POWL editor
```

### Run Tests

```bash
# Rust tests (native)
cargo test

# WASM tests (browser)
cd js/
npm test              # Firefox headless
# or:
wasm-pack test .. --headless --chrome
```

## Documentation

- **[Tutorial](./docs/tutorial.md)** — Getting started guide with examples
- **[API Reference](./docs/reference.md)** — Complete API documentation
- **[Vision 2030](./docs/vision-2030.md)** — Roadmap and future directions

## Architecture

### Rust Modules

| Module | Purpose |
|--------|---------|
| `src/lib.rs` | wasm-bindgen entry point, arena-based storage |
| `src/powl.rs` | POWL v2 node types (Transition, Operator, SPO) |
| `src/parser.rs` | POWL model string parser |
| `src/binary_relation.rs` | Bit-packed adjacency matrix with Warshall's algorithm |
| `src/petri_net.rs` | Petri net conversion (Place, Transition, Arc) |
| `src/process_tree.rs` | Process tree conversion (hierarchical operators) |
| `src/footprints.rs` | Behavioral signature extraction |
| `src/conformance/token_replay.rs` | Token-replay conformance checking |
| `src/event_log.rs` | XES/CSV event log parsing |
| `src/trie.rs` | Trie (prefix tree) data structure for log analysis |
| `src/streaming.rs` | Streaming drift detection with EWMA smoothing |
| `src/diff.rs` | Behavioral diff between two POWL models |
| `src/complexity.rs` | Model complexity metrics |
| `src/discovery/` | Process discovery algorithms (Alpha, Alpha+, Inductive, DFG, etc.) |
| `src/conversion/` | Converters to BPMN, Petri nets, process trees, YAWL |
| `src/algorithms/` | Marking equation, reduction, simplification, transitive operations |
| `src/ocel/` | Object-Centric Event Log support (parsing, flattening, ETOT) |
| `src/transformation/` | Log-to-trie transformation and other log operations |

### JavaScript/TypeScript Client

The `js/` directory provides a high-level TypeScript API wrapping the WASM module:

- **TypeScript types** — Generated by `wasm-bindgen` for all exported functions
- **Vite dev server** — Hot module replacement during development
- **Demo page** — Live POWL editor + conformance checker
- **Browser tests** — Web platform API integration tests

## Key Concepts

### Arena-Based Storage

All POWL nodes are stored in a flat arena (Vec<PowlNode>). Nodes are referenced by their `u32` index. The root of the parsed tree is always the last node (index `model.len() - 1`).

This design enables:
- Efficient memory layout for WASM
- Fast traversal without pointer chasing
- Easy serialization/deserialization

### Binary Relations

Partial orders are represented as bit-packed adjacency matrices (`BinaryRelation`). Each row is a `u64` bitset, enabling fast set operations:

- **Transitive closure** — O(k³) with bit-level parallelism
- **Transitive reduction** — O(k³) after closure
- **Reachability queries** — O(1) with closure

### Node Types

```rust
enum PowlNode {
  Transition {
    label: Option<String>,  // None for silent (tau)
    id: u32,
  },
  FrequentTransition {
    label: String,
    activity: String,
    skippable: bool,      // min=0 if true
    selfloop: bool,       // can repeat
  },
  StrictPartialOrder {
    children: Vec<u32>,   // Arena indices of child nodes
    order: BinaryRelation, // Ordering relation
  },
  OperatorPowl {
    operator: Operator,   // Xor, Loop, Sequence, Parallel, etc.
    children: Vec<u32>,
  },
}
```

## Common Patterns

### Simple Sequence

```typescript
const model = powl.parse('PO=(nodes={A, B, C}, order={A-->B, B-->C})');
```

### Parallel Block

```typescript
const model = powl.parse('PO=(nodes={A, B, C}, order={})');  // A, B, C all concurrent
```

### Choice (XOR)

```typescript
const model = powl.parse('X(A, B)');  // A or B, but not both
```

### Optionality

```typescript
const model = powl.parse('X(A, tau)');  // A or skip (tau = silent)
```

### Loop

```typescript
const model = powl.parse('*(A, B)');  // Do A, then optionally repeat B→A
```

### Complex Flow

```typescript
// Sequential: A → (B∥C) → (X∨Y) → D
const model = powl.parse(`
  PO=(nodes={
    A,
    PO=(nodes={B, C}, order={}),
    X(X, Y),
    D
  }, order={
    A-->PO=(nodes={B, C}, order={}),
    PO=(nodes={B, C}, order={})-->X(X, Y),
    X(X, Y)-->D
  })
`);
```

## Performance

| Operation | Complexity | Notes |
|-----------|-----------|-------|
| Parsing | O(n) | n = string length |
| Transitive closure | O(k³) | k = children in SPO; bit-packed |
| Transitive reduction | O(k³) | After closure |
| Simplification | O(n) | Recursive single pass |
| Footprints | O(k² + k³) | Closure computation dominates |
| Petri net conversion | O(n + m) | n = nodes, m = ordering edges |
| Process tree conversion | O(n²) | DAG processing |

For typical models (< 100 nodes), all operations complete in < 10 ms in the browser.

## License

Apache-2.0 — See [LICENSE](./LICENSE) for details.

## Contributing

This project is part of the pm4py ecosystem. Contributions are welcome!

1. Fork the repository
2. Create a feature branch (`git checkout -b feat/amazing-feature`)
3. Commit your changes (`git commit -m 'feat: add amazing feature'`)
4. Push to the branch (`git push origin feat/amazing-feature`)
5. Open a Pull Request

## See Also

- [pm4py Python library](https://pm4py.fit.fraunhofer.de/) — Reference implementation
- [Process Mining Handbook](https://www.springer.com/gp/book/9783642355248) — Theory background
- [POWL paper](https://doi.org/10.1007/978-3-030-96123-4_6) — Academic foundation
