# Troubleshooting Guide

Common issues and solutions when working with POWL v2 Rust/WASM.

## Table of Contents

1. [Build Issues](#build-issues)
2. [Runtime Errors](#runtime-errors)
3. [Performance Issues](#performance-issues)
4. [Validation Errors](#validation-errors)
5. [Browser Compatibility](#browser-compatibility)
6. [Memory Issues](#memory-issues)

---

## Build Issues

### wasm-pack: "command not found"

**Problem:**
```bash
wasm-pack: command not found
```

**Solution:**
Install wasm-pack:
```bash
curl https://rustwasm.github.io/wasm-pack/installer/init.sh -sSf | sh
```

Or via cargo:
```bash
cargo install wasm-pack
```

### "error: linker `lld` not found"

**Problem:**
```
error: linker `lld` not found
  |
  = note: the wasm32-unknown-unknown target may not be installed
```

**Solution:**
Add the wasm32 target:
```bash
rustup target add wasm32-unknown-unknown
```

### "error: failed to resolve: could not find `wasm-bindgen` in `Cargo.toml`"

**Problem:**
Build fails with missing dependency error.

**Solution:**
Update Cargo.toml to include wasm-bindgen:
```toml
[dependencies]
wasm-bindgen = "0.2"
```

Then run:
```bash
cargo clean
cargo build
```

### npm build fails: "Cannot find module './pkg/pm4wasm_bg.wasm'"

**Problem:**
TypeScript build fails because WASM not built first.

**Solution:**
Build WASM before TypeScript:
```bash
cd js/
npm run build:wasm    # Build WASM first
npm run build:ts      # Then build TypeScript
```

---

## Runtime Errors

### "WebAssembly instantiation failed"

**Problem:**
```javascript
Uncaught Error: WebAssembly.instantiate(): could not load wasm file
```

**Solution:**
Ensure WASM file is served correctly:
1. Check that `pm4wasm_bg.wasm` exists in `pkg/`
2. Verify server sends correct MIME type (`application/wasm`)
3. Use absolute import path:
```javascript
import init from './pkg/pm4wasm_bg.wasm';  // Wrong
import init from './pkg/pm4wasm.js';         // Correct
```

### "Parse error: unexpected token"

**Problem:**
```javascript
const model = parse_powl("X(A, B");  // Missing closing paren
// Error: Parse error: unexpected end of input
```

**Solution:**
Check POWL model string syntax:
1. Balance all parentheses: `X(A, B)` not `X(A, B`
2. Use proper separators: `A, B, C` not `A B C`
3. Escape quotes in labels: `"Claim \"A\""` not `"Claim "A""`

Common syntax errors:
- Missing comma: `X(A B)` → `X(A, B)`
- Extra comma: `X(A, B,)` → `X(A, B)`
- Unbalanced braces: `PO=(nodes={A}` → `PO=(nodes={A})`

### "Validation error: self-loop detected"

**Problem:**
```javascript
const model = parse_powl("PO=(nodes={A}, order={A-->A})");
validate_partial_orders(model);
// Error: Validation error: node 0 has a self-loop
```

**Solution:**
Remove self-loops from partial orders:
```javascript
// Wrong
"PO=(nodes={A}, order={A-->A})"

// Correct (no ordering)
"PO=(nodes={A}, order={})"

// Correct (reflexive, but use transitive closure)
"PO=(nodes={A, B}, order={A-->B, B-->A})"  // Cycle, but not self-loop
```

### "node 999 is not a StrictPartialOrder"

**Problem:**
```javascript
const closure = transitive_closure(model, 999);
// Error: node 999 is not a StrictPartialOrder
```

**Solution:**
Use valid arena index:
```javascript
// Get valid SPO indices
const root_idx = model.root();  // Always valid
const children = get_children(model, root_idx);

// Check node type before calling
const info = JSON.parse(node_info_json(model, idx));
if (info.type === "StrictPartialOrder") {
    const closure = transitive_closure(model, idx);
}
```

---

## Performance Issues

### "Transitive closure takes too long"

**Problem:**
Transitive closure on large SPO (> 500 children) takes > 10 seconds.

**Solution:**
1. **Simplify model first:**
```javascript
const simplified = simplify_powl(model);
// Then compute closure on simplified model
```

2. **Use reduction instead of closure:**
```javascript
// If you only need minimal edges, use reduction
const reduction = transitive_reduction(model, spo_idx);
```

3. **Batch operations:**
```javascript
// Compute closure once, reuse for multiple queries
const closure = transitive_closure(model, spo_idx);
for (const [i, j] of queries) {
    const reachable = closure.is_edge(i, j);
}
```

### "Memory usage grows during parsing"

**Problem:**
Parsing large event logs (> 10 MB) causes browser to slow down or crash.

**Solution:**
1. **Use streaming parser:**
```javascript
// Instead of loading entire file at once
for await (const batch of parse_xes_stream(file)) {
    process_batch(batch);
}
```

2. **Filter early:**
```javascript
// Filter traces before parsing to POWL
const filtered = filter_traces(log, { min_length: 3, max_length: 100 });
```

3. **Use Web Workers:**
```javascript
// Offload parsing to worker thread
const worker = new Worker('powl-worker.js');
worker.postMessage({ file: eventLogFile });
```

---

## Validation Errors

### "Validation error: transitivity violated"

**Problem:**
```javascript
const model = parse_powl("PO=(nodes={A, B, C}, order={A-->B, B-->C})");
validate_partial_orders(model);
// Error: Validation error: transitivity violated (A-->C missing)
```

**Solution:**
Add missing transitive edges:
```javascript
// Wrong
"PO=(nodes={A, B, C}, order={A-->B, B-->C})"

// Correct (add A-->C for transitivity)
"PO=(nodes={A, B, C}, order={A-->B, B-->C, A-->C})"

// Or let simplification add it automatically
let model = parse_powl("PO=(nodes={A, B, C}, order={A-->B, B-->C})");
model = simplify_powl(model);  // Adds transitive edges
```

### "Validation error: cycle detected"

**Problem:**
```javascript
const model = parse_powl("PO=(nodes={A, B}, order={A-->B, B-->A})");
validate_partial_orders(model);
// Error: Validation error: cycle detected (A-->B-->A)
```

**Solution:**
Remove cycles (partial orders must be acyclic):
```javascript
// Wrong (cycle)
"PO=(nodes={A, B}, order={A-->B, B-->A})"

// Correct (no ordering, i.e., parallel)
"PO=(nodes={A, B}, order={})"

// Correct (sequential)
"PO=(nodes={A, B}, order={A-->B})"

// If you need repetition, use LOOP operator
"*(A, B)"  // Do A, then optionally repeat B→A
```

---

## Browser Compatibility

### "SharedArrayBuffer is not defined"

**Problem:**
```javascript
ReferenceError: SharedArrayBuffer is not defined
```

**Solution:**
SharedArrayBuffer requires special HTTP headers:
```nginx
# nginx.conf
add_header Cross-Origin-Opener-Policy "same-origin" always;
add_header Cross-Origin-Embedder-Policy "require-corp" always;
```

Or avoid SharedArrayBuffer entirely (use `--no-threads`):
```bash
wasm-pack build --target bundler --release --no-threads
```

### "Safari doesn't support WASM bulk memory"

**Problem:**
Older Safari versions (< 15) lack bulk memory operations.

**Solution:**
1. Update Safari or use polyfill:
```javascript
if (!WebAssembly.Global) {
    // Load polyfill
}
```

2. Or disable bulk memory features:
```bash
RUSTFLAGS='-C target-feature=-bulk-memory' wasm-pack build
```

### "Firefox blocks cross-origin WASM"

**Problem:**
WASM file blocked by CORS when loaded from different origin.

**Solution:**
Serve WASM from same origin or enable CORS:
```nginx
# nginx.conf
location /pkg/ {
    add_header Access-Control-Allow-Origin *;
    add_header Access-Control-Allow-Methods "GET, OPTIONS";
}
```

---

## Memory Issues

### "Out of memory: WASM linear memory limit reached"

**Problem:**
Processing large models (> 1000 nodes) hits WASM memory limit (default 1 GB).

**Solution:**
1. **Increase WASM memory limit:**
```javascript
const memory = new WebAssembly.Memory({ initial: 256, maximum: 2048 });  // 256 pages to 2048 pages (16 MB to 128 MB)
const imports = { env: { memory } };
const instance = await WebAssembly.instantiate(wasmBytes, imports);
```

2. **Process in batches:**
```javascript
// Instead of processing entire log at once
for (const batch of chunks(log, 1000)) {
    const result = powl.conformance(model, batch);
    // Accumulate results
}
```

3. **Free unused models:**
```javascript
let model = parse_powl("...");
// Use model...
model = null;  // Allow GC to reclaim memory
```

### "Memory leak in event log parsing"

**Problem:**
Repeatedly parsing event logs causes memory to grow indefinitely.

**Solution:**
1. **Reuse parser instance:**
```javascript
// Wrong (creates new parser each time)
for (const file of files) {
    const log = powl.parseCsv(file);
}

// Correct (reuse parser)
const parser = new powl.EventLogParser();
for (const file of files) {
    const log = parser.parseCsv(file);
}
```

2. **Explicitly drop references:**
```javascript
let logs = [];
for (const file of files) {
    logs.push(powl.parseCsv(file));
    // Process log immediately
    process_log(logs[logs.length - 1]);
}
logs = null;  // Drop all references
```

---

## Debugging Tips

### Enable WASM debug output

```javascript
const powl = await Powl.init({
    debug: true  // Enable console logging
});
```

### Inspect arena structure

```javascript
const model = parse_powl("X(A, B)");
console.log("Arena size:", model.len());  // Total nodes
console.log("Root index:", model.root()); // Root node

// Walk all nodes
for (let i = 0; i < model.len(); i++) {
    const info = JSON.parse(node_info_json(model, i));
    console.log(`Node ${i}:`, info.type, info.label || info.operator);
}
```

### Validate after each operation

```javascript
let model = parse_powl("...");
model.validate();  // Check immediately

model = simplify_powl(model);
model.validate();  // Check again

const petriNet = to_petri_net(model);
// Verify Petri net structure
console.log("Places:", petriNet.net.places.length);
console.log("Transitions:", petriNet.net.transitions.length);
```

### Profile performance

```javascript
console.time("parse");
const model = parse_powl("...");
console.timeEnd("parse");

console.time("closure");
const closure = transitive_closure(model, root_idx);
console.timeEnd("closure");

console.time("footprints");
const fp = get_footprints(model);
console.timeEnd("footprints");
```

---

## Getting Help

If none of these solutions work:

1. **Check the issue tracker:** [github.com/pm4py/pm4py/issues](https://github.com/pm4py/pm4py/issues)
2. **Create minimal repro:** Smallest code snippet that shows the problem
3. **Include environment info:** Browser version, Rust version, wasm-pack version
4. **Share error message:** Full stack trace and console output

---

## See Also

- [API Reference](./reference.md) — Complete API documentation
- [Tutorial](./tutorial.md) — Getting started guide
- [Architecture](./architecture.md) — Implementation details
