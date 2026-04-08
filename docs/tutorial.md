# Tutorial: Getting Started with POWL v2 Rust/WASM

## Learning Objectives

By the end of this tutorial, you'll be able to:
- Parse a POWL model string
- Validate partial order constraints
- Convert a POWL model to a Petri net
- Analyze behavioral footprints
- Use the library from JavaScript (browser)

## What is POWL?

POWL (Partially Ordered Workflow Language) is a process model notation that combines:
- **Activities** (transitions with labels like "A", "B", "C")
- **Operators** for control flow (XOR for choice, LOOP for repetition)
- **Partial orders** for concurrency (activities with no ordering constraint run in parallel)

Example model:
```
PO=(nodes={ A, X(B, C), D }, order={ A-->X(B,C), X(B,C)-->D })
```

This means: Execute A, then choose between B or C, then execute D.

## Step 1: Parse a POWL Model

### JavaScript (Browser)

```javascript
import init, { parse_powl } from './pkg/pm4wasm.js';

await init();

// Parse a simple sequential model
const model = parse_powl("PO=(nodes={A, B, C}, order={A-->B, B-->C})");

// or a choice model
const choice = parse_powl("X ( A, B )");

// or a loop model  
const loop = parse_powl("* ( A, B )");

console.log(model.toString());  // Pretty-print the model
```

### Rust (Native)

```rust
use pm4wasm::parser::parse_powl_model_string;
use pm4wasm::powl::PowlArena;

let mut arena = PowlArena::new();
let root = parse_powl_model_string(
    "PO=(nodes={A, B}, order={A-->B})",
    &mut arena
)?;

println!("{}", arena.to_repr(root));
```

## Step 2: Validate the Model

Ensure all partial orders satisfy the strict partial order properties (irreflexive, transitive):

```javascript
try {
  model.validate();
  console.log("✓ Model is valid!");
} catch (error) {
  console.error("✗ Model is invalid:", error.message);
}
```

## Step 3: Convert to a Petri Net

Convert the POWL model to a Petri net for conformance checking or simulation:

```javascript
const petriNetResult = model.to_petri_net();

console.log("Places:", petriNetResult.net.places.length);
console.log("Transitions:", petriNetResult.net.transitions.length);
console.log("Arcs:", petriNetResult.net.arcs.length);

// Access the Petri net structure
petriNetResult.net.places.forEach(place => {
  console.log(`Place: ${place.name}`);
});

petriNetResult.net.transitions.forEach(trans => {
  console.log(`Transition: ${trans.name} [${trans.label || 'silent'}]`);
});
```

The conversion handles:
- **Transitions** → single place-transition-place
- **XOR choice** → split/join with decision transitions
- **LOOP** → feedback arc with exit transition
- **Partial order** → synchronization barriers (tau-split / tau-join) + ordering arcs

## Step 4: Analyze Footprints

Extract behavioral properties of the model:

```javascript
const footprints = model.get_footprints();

console.log("Start activities:", footprints.start_activities);
// Output: Set { 'A' }

console.log("End activities:", footprints.end_activities);
// Output: Set { 'C' }

console.log("Sequence relations:", footprints.sequence);
// Output: Set { ['A', 'B'], ['B', 'C'], ['A', 'C'] }

console.log("Parallel relations:", footprints.parallel);
// Output: Set { }  (empty, since A→B→C is sequential)

console.log("Always happens:", footprints.activities_always_happening);
// Output: Set { 'A', 'B', 'C' }

console.log("Min trace length:", footprints.min_trace_length);
// Output: 3
```

### Understanding Footprints

| Field | Meaning |
|-------|---------|
| `start_activities` | Which activities can start a trace |
| `end_activities` | Which activities can end a trace |
| `sequence` | Pairs (a,b) where a can directly precede b |
| `parallel` | Pairs (a,b) where a and b can execute concurrently |
| `activities_always_happening` | Activities that occur in every execution |
| `min_trace_length` | Minimum number of activities in a valid trace |

## Step 5: Complex Example — Concurrent Activities

```javascript
// A in parallel with (B → C), then D
const complex = parse_powl(
  "PO=(nodes={A, PO=(nodes={B, C}, order={B-->C}), D}, order={A-->D, PO=(nodes={B, C}, order={B-->C})-->D})"
);

complex.validate();

const fp = complex.get_footprints();
console.log("Start:", fp.start_activities);     // A, B (both can start)
console.log("Parallel:", fp.parallel);           // (A,B), (A,C) etc
console.log("Min length:", fp.min_trace_length); // 3 (A, B, C, D = 4? or min is 3?)
```

## Step 6: Simplify a Model

Apply structural normalization (flatten nested operators, merge patterns):

```javascript
let model = parse_powl("X ( A, X ( B, C ) )");

model.simplify();
// Result: X ( A, B, C )  — flattened nested XOR

model = parse_powl("X ( A, tau )");
model.simplify_using_frequent_transitions();
// Result: FrequentTransition(A, min=0, max=1)  — optional activity
```

## Building from Source

### WASM (Browser)

```bash
cd pm4wasm
wasm-pack build --target web --release
```

Output: `pkg/pm4wasm.js` + `pkg/pm4wasm_bg.wasm`

Include in HTML:
```html
<script type="module">
  import init, { parse_powl } from './pkg/pm4wasm.js';
  init().then(() => {
    const model = parse_powl("A");
    console.log(model.toString());
  });
</script>
```

### Native (Rust)

```bash
cd pm4wasm
cargo build --release
cargo test
```

## Common Patterns

### Model: Simple Sequence
```javascript
parse_powl("PO=(nodes={A, B, C}, order={A-->B, B-->C})")
```

### Model: Parallel Block
```javascript
parse_powl("PO=(nodes={A, B, C}, order={})")  // A, B, C all concurrent
```

### Model: Choice
```javascript
parse_powl("X ( A, B )")  // A or B, but not both
```

### Model: Optionality
```javascript
parse_powl("X ( A, tau )")  // A or skip
```

### Model: Loop
```javascript
parse_powl("* ( A, B )")  // Do A, then optionally repeat B→A
```

### Model: Complex Flow
```javascript
// Sequential: A → (B∥C) → (X∨Y) → D
parse_powl(`
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
`)
```

## Error Handling

```javascript
try {
  const model = parse_powl("invalid syntax");
} catch (error) {
  console.error("Parse error:", error);
  // "Parse error: unknown node 'invalid'"
}

try {
  model.validate();
} catch (error) {
  console.error("Validation error:", error);
  // "Validation error: transitivity of the partial order is violated"
}
```

## Next Steps

- **Conformance**: Use the Petri net output with existing process mining tools
- **Discovery**: Combine with event log analysis (from pm4py Python)
- **Visualization**: Render the Petri net or process tree in a diagram
- **Optimization**: Use footprints for process performance analysis

## API Reference

See `docs/reference.md` for complete API documentation.
