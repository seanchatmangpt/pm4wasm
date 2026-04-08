# Quick Reference Guide

A concise reference for common POWL v2 Rust/WASM operations.

## Table of Contents

1. [NL → POWL → BPMN (AI-Assisted)](#nl--powl--bpmn-ai-assisted)
2. [Parsing Models](#parsing-models)
3. [Model Inspection](#model-inspection)
4. [Validation](#validation)
5. [Transformation](#transformation)
6. [Conversion](#conversion)
7. [Analysis](#analysis)
8. [Event Logs](#event-logs)
9. [Conformance](#conformance)

---

## NL → POWL → BPMN (AI-Assisted)

### Generate POWL from Natural Language

```python
from pm4py.algo.dspy.powl.natural_language import generate_powl_from_text

result = generate_powl_from_text(
    "A customer submits an order. If valid, pick, pack, and bill in parallel. "
    "If invalid, cancel and refund."
)
print(result["powl"])       # POWL model string
print(result["verdict"])    # True (verified)
print(result["refinements"]) # 0 or 1
```

### Convert to BPMN

```python
from pm4py.objects.powl.parser import parse_powl_model_string
import pm4py

parsed = parse_powl_model_string(result["powl"])
try:
    bpmn = pm4py.convert_to_bpmn(parsed)
except Exception:
    net, im, fm = pm4py.convert_to_petri_net(parsed)
    bpmn = pm4py.convert_to_bpmn(net, im, fm)
pm4py.write_bpmn(bpmn, "output.bpmn")
```

### CLI Commands

```bash
# NL → POWL file
python -m pm4py.cli DiscoverPOWLFromText "process description..." output.powl

# NL → BPMN file (full pipeline with verification)
python -m pm4py.cli DiscoverPOWLToBPMN "process description..." output.bpmn

# From a text file
python -m pm4py.cli DiscoverPOWLToBPMN description.txt output.bpmn

# Event log → POWL (programmatic, no LLM)
python -m pm4py.cli DiscoverPOWL running-example.xes output.powl
```

### Key Operator Distinction

| Operator | Syntax | Meaning | Use When |
|---|---|---|---|
| **XOR** | `X(A, B)` | Exactly one executes | "either A or B" |
| **LOOP** | `*(A, B)` | Do A, optionally repeat B→A | "repeat", "retry" |
| **Partial Order** | `PO=(nodes={A,B}, order={})` | All execute concurrently | "A and B in parallel" |

**Critical:** In `PO=()`, ALL outgoing edges mean ALL successors MUST complete. If only ONE should execute, use `X()` instead.

---

## Parsing Models

### Simple Transition

```javascript
const model = parse_powl("A");
console.log(powl_to_string(model));  // "A"
```

### Sequential Model

```javascript
const model = parse_powl("PO=(nodes={A, B, C}, order={A-->B, B-->C})");
```

### Parallel Model

```javascript
const model = parse_powl("PO=(nodes={A, B, C}, order={})");
// A, B, C execute concurrently
```

### Choice (XOR)

```javascript
const model = parse_powl("X(A, B)");
// Execute A or B, but not both
```

### Loop

```javascript
const model = parse_powl("*(A, B)");
// Do A, then optionally repeat B→A
```

### Optional Activity

```javascript
const model = parse_powl("X(A, tau)");
// Execute A or skip (tau = silent)
```

### Complex Nested Model

```javascript
const model = parse_powl(`
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
// A → (B∥C) → (X∨Y) → D
```

---

## Model Inspection

### Get All Activities

```javascript
const model = parse_powl("PO=(nodes={A, B, C}, order={A-->B, B-->C})");
const activities = model.activities();
console.log([...activities]);  // ["A", "B", "C"]
```

### Get Node Info

```javascript
const info = JSON.parse(node_info_json(model, root_idx));

// Transition node
{
  "type": "Transition",
  "label": "A",
  "id": 0
}

// Operator node
{
  "type": "OperatorPowl",
  "operator": "Xor",
  "children": [1, 2]
}

// SPO node
{
  "type": "StrictPartialOrder",
  "children": [1, 2, 3],
  "edges": [[0, 1], [1, 2]]
}
```

### Get Children

```javascript
const children = get_children(model, root_idx);
console.log(children);  // [1, 2, 3]
```

### Get Node String

```javascript
const str = node_to_string(model, idx);
console.log(str);  // "A" or "X ( A, B )" or "PO=(nodes={ ... }, order={ ... })"
```

### Walk Tree

```javascript
const visit = (idx) => {
    const info = JSON.parse(node_info_json(model, idx));
    console.log(`Node ${idx}:`, info.type, info.label || info.operator);

    if (info.children) {
        for (const child_idx of info.children) {
            visit(child_idx);
        }
    }
};

visit(model.root());
```

---

## Validation

### Validate Partial Orders

```javascript
try {
    validate_partial_orders(model);
    console.log("✓ Model is valid");
} catch (e) {
    console.error("✗ Invalid:", e.message);
}
```

### Check if Relation is Transitive

```javascript
const order = get_order_of(model, spo_idx);
if (order.is_transitive()) {
    console.log("✓ Transitive");
} else {
    console.log("✗ Not transitive");
}
```

### Check if Relation is Strict Partial Order

```javascript
const order = get_order_of(model, spo_idx);
if (order.is_strict_partial_order()) {
    console.log("✓ Strict partial order");
} else {
    console.log("✗ Not a strict partial order");
}
```

---

## Transformation

### Simplify Model

```javascript
let model = parse_powl("X(A, X(B, C))");
console.log(powl_to_string(model));  // "X ( A, X ( B, C ) )"

model = simplify_powl(model);
console.log(powl_to_string(model));  // "X ( A, B, C )"
```

### Simplify Frequent Transitions

```javascript
let model = parse_powl("X(A, tau)");
model = simplify_frequent_transitions(model);
// Converts to: FrequentTransition(activity=A, skippable=true, selfloop=false)
```

### Transitive Closure

```javascript
const closure = transitive_closure(model, spo_idx);
// Now check reachability
console.log(closure.is_edge(0, 2));  // true if 0 can reach 2
```

### Transitive Reduction

```javascript
const reduction = transitive_reduction(model, spo_idx);
// Minimal edge set preserving reachability
console.log(reduction.edges_flat());  // [src0, tgt0, src1, tgt1, ...]
```

---

## Conversion

### To Petri Net

```javascript
const result = to_petri_net(model);

console.log("Places:", result.net.places.length);
console.log("Transitions:", result.net.transitions.length);
console.log("Arcs:", result.net.arcs.length);

// Inspect places
result.net.places.forEach(place => {
    console.log(`Place: ${place.name}`);
});

// Inspect transitions
result.net.transitions.forEach(trans => {
    const label = trans.label || "(silent)";
    console.log(`Transition: ${trans.name} [${label}]`);
});

// Inspect markings
console.log("Initial marking:", result.initial_marking);
console.log("Final marking:", result.final_marking);
```

### To Process Tree

```javascript
const result = to_process_tree(model);

function print_tree(node, indent = 0) {
    const prefix = "  ".repeat(indent);
    if (node.label) {
        console.log(`${prefix}${node.label}`);
    } else {
        console.log(`${prefix}${node.operator}`);
        for (const child of node.children) {
            print_tree(child, indent + 1);
        }
    }
}

print_tree(result.root);
```

---

## Analysis

### Get Footprints

```javascript
const fp = get_footprints(model);

console.log("Start activities:", [...fp.start_activities]);
console.log("End activities:", [...fp.end_activities]);
console.log("All activities:", [...fp.activities]);
console.log("Always happens:", [...fp.activities_always_happening]);
console.log("Skippable:", [...fp.skippable_activities]);
console.log("Min trace length:", fp.min_trace_length);
console.log("Max trace length:", fp.max_trace_length);

// Sequence relations (direct precedence)
for (const [a, b] of fp.sequence) {
    console.log(`${a} → ${b}`);
}

// Parallel relations (concurrency)
for (const [a, b] of fp.parallel) {
    console.log(`${a} ∥ ${b}`);
}
```

### Get Complexity Metrics

```javascript
const metrics = get_complexity(model);

console.log("Total nodes:", metrics.total_nodes);
console.log("Transitions:", metrics.transition_count);
console.log("Operators:", metrics.operator_count);
console.log("Nesting depth:", metrics.max_nesting_depth);
console.log("Control flow complexity:", metrics.cfc_score);
```

### Compare Models

```javascript
const diff = compare_powl(model_v1, model_v2);

console.log("Added activities:", diff.added_activities);
console.log("Removed activities:", diff.removed_activities);
console.log("Added parallel pairs:", diff.added_parallel_pairs);
console.log("Removed parallel pairs:", diff.removed_parallel_pairs);
console.log("Conformance delta:", diff.conformance_delta);
```

---

## Event Logs

### Parse CSV

```javascript
const csv = `case_id,activity
1,A
1,B
1,C
2,A
2,C`;

const log = parse_csv(csv);
console.log("Traces:", log.traces.length);
console.log("Events:", log.total_events());
```

### Parse XES

```javascript
const xml = `<log>...</log>`;
const log = parse_xes(xml);
```

### Parse XES from URL

```javascript
const log = await fetch_xes("https://example.com/log.xes");
```

### Parse XES from File

```javascript
const fileInput = document.getElementById("file-input");
const file = fileInput.files[0];
const log = await read_xes_file(file);
```

### Get Variants

```javascript
const variants = get_variants(log);

for (const [trace, count] of Object.entries(variants)) {
    console.log(`${trace}: ${count} occurrences`);
}
```

### Filter Traces

```javascript
// Filter by length
const filtered = filter_traces(log, { min_length: 3, max_length: 10 });

// Filter by activity
const filtered = filter_traces(log, { must_contain: ["A", "B"] });

// Filter by fitness
const filtered = filter_by_fitness(model, log, 0.8);
```

---

## Conformance

### Token Replay Fitness

```javascript
const result = conformance(model, log);

console.log("Fitness:", (result.percentage * 100).toFixed(1) + "%");
console.log("Perfectly fitting traces:", result.perfectly_fitting_traces);
console.log("Traces with deviations:", result.traces_with_deviations);

// Deviation details
for (const trace_dev of result.deviations) {
    console.log(`Trace ${trace_dev.trace_index}:`);
    console.log("  Missing tokens:", trace_dev.missing_tokens);
    console.log("  Remaining tokens:", trace_dev.remaining_tokens);
    console.log("  Fitness:", (trace_dev.fitness * 100).toFixed(1) + "%");
}
```

### Filter by Fitness

```javascript
const good_traces = filter_by_fitness(model, log, 0.8);
console.log("Good traces:", good_traces.traces.length);
```

### Petri Net Conformance

```javascript
const pn_result = to_petri_net(model);
const result = conformance_petri_net(pn_result, log);
console.log("Fitness:", result.percentage);
```

---

## Common Patterns

### Check if Model is Sound

```javascript
function is_sound(model) {
    try {
        validate_partial_orders(model);

        const fp = get_footprints(model);
        if (fp.start_activities.size === 0) return false;
        if (fp.end_activities.size === 0) return false;

        return true;
    } catch (e) {
        return false;
    }
}

if (is_sound(model)) {
    console.log("✓ Model is sound");
} else {
    console.log("✗ Model is unsound");
}
```

### Find All Paths

```javascript
function find_all_paths(model) {
    const paths = [];
    const fp = get_footprints(model);

    // Simple DFS (for acyclic models only)
    function dfs(current, visited, path) {
        if (visited.has(current)) return;

        visited.add(current);
        path.push(current);

        if (fp.end_activities.has(current)) {
            paths.push([...path]);
        } else {
            for (const [a, b] of fp.sequence) {
                if (a === current) {
                    dfs(b, visited, path);
                }
            }
        }

        path.pop();
        visited.delete(current);
    }

    for (const start of fp.start_activities) {
        dfs(start, new Set(), []);
    }

    return paths;
}

const paths = find_all_paths(model);
console.log("All paths:", paths);
```

### Compute Cyclomatic Complexity

```javascript
function cyclomatic_complexity(model) {
    const fp = get_footprints(model);
    const edges = fp.sequence.size + fp.parallel.size;
    const nodes = fp.activities.size;
    const components = 1;  // Assume single connected component

    return edges - nodes + 2 * components;
}

const cc = cyclomatic_complexity(model);
console.log("Cyclomatic complexity:", cc);
```

### Detect Deadlocks

```javascript
function has_deadlock(petri_net_result) {
    // Check if there are places that can never be marked
    const { net, initial_marking } = petri_net_result;

    for (const place of net.places) {
        // Find incoming arcs
        const incoming = net.arcs.filter(a => a.target === place.name);
        if (incoming.length === 0 && !initial_marking[place.name]) {
            // Place has no incoming arcs and is not initially marked
            return true;
        }
    }

    return false;
}

const pn_result = to_petri_net(model);
if (has_deadlock(pn_result)) {
    console.log("⚠ Potential deadlock detected");
}
```

---

## Performance Tips

### Batch Operations

```javascript
// Instead of multiple closure calls
const closure1 = transitive_closure(model, idx1);
const closure2 = transitive_closure(model, idx2);

// Compute once and reuse
const closure = transitive_closure(model, root_idx);
```

### Simplify Before Analysis

```javascript
// Simplify first to reduce complexity
const simplified = simplify_powl(model);

// Then analyze
const fp = get_footprints(simplified);
const pn = to_petri_net(simplified);
```

### Use Iterators Instead of Arrays

```javascript
// Instead of
const activities = [...model.activities()];

// Use iterator directly
for (const activity of model.activities()) {
    console.log(activity);
}
```

---

## TypeScript Types

```typescript
interface PowlModel {
    root(): number;
    len(): number;
    is_empty(): boolean;
    activities(): IterableIterator<string>;
    toString(): string;
    validate(): void;
    simplify(): PowlModel;
    simplifyFrequent(): PowlModel;
    toPetriNet(): PetriNetResult;
    nodeInfo(idx: number): string;
    children(idx: number): number[];
}

interface Footprints {
    start_activities: Set<string>;
    end_activities: Set<string>;
    activities: Set<string>;
    activities_always_happening: Set<string>;
    skippable_activities: Set<string>;
    sequence: Set<[string, string]>;
    parallel: Set<[string, string]>;
    min_trace_length: number;
    max_trace_length?: number;
}

interface PetriNetResult {
    net: {
        name: string;
        places: Array<{ name: string }>;
        transitions: Array<{ name: string; label?: string }>;
        arcs: Array<{ source: string; target: string; weight: number }>;
    };
    initial_marking: Record<string, number>;
    final_marking: Record<string, number>;
}
```

---

## See Also

- [API Reference](./reference.md) — Complete API documentation
- [Tutorial](./tutorial.md) — Getting started guide
- [Troubleshooting](./troubleshooting.md) — Common issues and solutions
