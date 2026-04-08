# pm4wasm

Process mining in Rust/WebAssembly — a WASM port of [pm4py](https://github.com/pm4py/pm4py-core) with LLM integration for natural language process discovery.

**pm4wasm** brings the core pm4py algorithms to the browser, enabling process mining entirely client-side with no server required. Plus, LLM-powered features for generating process models from natural language descriptions.

## Install

```bash
npm install pm4wasm
```

## Quick start

```ts
import { Powl } from "pm4wasm";

const powl = await Powl.init();

// Parse POWL model
const model = powl.parse("PO=(nodes={A, B, C}, order={A-->B, A-->C})");
model.validate();                       // throws if invalid
console.log(model.toString());          // canonical string
console.log([...model.activities()]);   // ["A", "B", "C"]

// Convert to Petri net
const petriNet = model.toPetriNet();
console.log(petriNet.net.transitions.length);

// Parse event log from CSV
const log = powl.parseCsv(
  "case_id,activity\n1,A\n1,B\n1,C\n2,A\n2,C\n"
);

// Check conformance
const fitness = powl.conformance(model, log);
console.log(fitness.percentage);         // 0.0 – 1.0
console.log(fitness.perfectly_fitting_traces);

// Filter by fitness threshold
const goodTraces = powl.filterByFitness(model, log, 0.8);
```

## pm4py WASM Port

pm4wasm ports the core pm4py process mining algorithms to WebAssembly:

### Discovery (from pm4py)
- `discoverDFG()` — Directly-Follows Graph
- `discoverDFGTyped()` — Typed DFG object format
- `discoverPerformanceDFG()` — Performance DFG with duration stats
- `discoverEventuallyFollowsGraph()` — Eventually-follows relations
- `discoverProcessTree()` — Inductive process tree
- `discoverPetriNet()` — Inductive Petri net
- `discoverPetriNetAlpha()` — Alpha miner
- `discoverPetriNetAlphaPlus()` — Alpha+ miner (handles loops)
- `discoverPrefixTree()` — Trie (prefix tree) of trace prefixes
- `discoverLogFootprints()` — Footprints discovery

### Conformance (from pm4py)
- `conformance()` — Token-replay fitness
- `conformancePetriNet()` — Fitness on pre-built Petri net
- `conformanceFootprints()` — Footprints-based fitness/precision/recall/f1
- `checkSoundness()` — Deadlock freedom, liveness, boundedness

### Filtering (from pm4py)
- 15 log filters: start/end activities, variants, time range, attributes, case size, prefixes/suffixes, and more

### Statistics (from pm4py)
- 20+ statistics: start/end activities, variants, case durations, rework times, overlaps, performance stats, and more

### I/O (from pm4py)
- XES and CSV read/write
- BPMN 2.0 XML export
- JSON-OCEL read/write (object-centric event logs)

### OCEL Support
- `parseOcelJson()` — Parse OCEL 1.0/2.0 JSON
- `ocelFlattenByObjectType()` — Flatten to traditional log by object type
- `discoverOcelEtot()` — Event-Type / Object-Type graph
- `ocelGetSummary()` — OCEL statistics
- `ocelGetObjectTypes()` — List all object types
- `ocelGetEventTypes()` — List all event types

## LLM Integration (Beyond pm4py)

Generate POWL models from natural language using Groq, OpenAI, or Anthropic:

```ts
// Natural language → POWL
const model = await powl.fromNaturalLanguage(
  "Customer orders and pays, then receives confirmation",
  {
    provider: "groq",
    apiKey: process.env.GROQ_API_KEY,
  },
  "ecommerce"
);

// POWL → BPMN
const bpmn = powl.toBpmn(model.toString());

// Generate workflow code directly
const n8nWorkflow = await powl.naturalLanguageToCode(
  "Order processing with payment",
  "n8n",
  { provider: "groq", apiKey: process.env.GROQ_API_KEY }
);
```

### Supported LLM Providers

| Provider | Speed | Cost | Best For |
|----------|-------|------|----------|
| **Groq** | ⚡⚡⚡ | Free | Development, fast iteration |
| **OpenAI** | ⚡⚡ | Paid | Production, GPT-4o |
| **Anthropic** | ⚡ | Paid | Claude 3.5 Sonnet |

## Build from source

Requires Rust + [wasm-pack](https://rustwasm.github.io/wasm-pack/).

```bash
# Install wasm-pack (once)
curl https://rustwasm.github.io/wasm-pack/installer/init.sh -sSf | sh

# Build WASM then TypeScript
cd js/
npm install
npm run build
```

## Dev / demo server

```bash
npm run demo
# Opens http://localhost:5173 — live POWL editor + conformance checker
```

## Run browser tests

```bash
npm test                   # Firefox headless
# or:
wasm-pack test .. --headless --chrome
```

## API overview

### `Powl.init() → Promise<Powl>`

Loads the WASM module once; safe to call multiple times.

### Parsing

| Method | Description |
|--------|-------------|
| `powl.parse(str)` | Parse a POWL model string |
| `powl.parseXes(xml)` | Parse a XES event log |
| `powl.parseCsv(csv)` | Parse a CSV event log |
| `powl.fetchXes(url)` | Fetch + parse XES from URL |
| `powl.readXesFile(file)` | Parse `File` drag-drop XES |
| `powl.readCsvFile(file)` | Parse `File` drag-drop CSV |

### `PowlModel`

| Method | Description |
|--------|-------------|
| `.toString()` | Canonical model string |
| `.validate()` | Throws on SPO violations |
| `.simplify()` | Structure-normalized model |
| `.simplifyFrequent()` | Convert XOR/LOOP+tau → FrequentTransition |
| `.toPetriNet()` | Returns `PetriNetResult` |
| `.nodeInfo(idx)` | Typed node description |
| `.children(idx)` | Child arena indices |
| `.activities()` | All activity labels |
| `.walk(visitor)` | Pre-order tree traversal |
| `.orderEdges(idx)` | SPO ordering relation edge list |
| `.closureEdges(idx)` | Transitive closure edge list |
| `.reductionEdges(idx)` | Transitive reduction edge list |

### Conformance

| Method | Description |
|--------|-------------|
| `powl.conformance(model, log)` | Token-replay fitness |
| `powl.conformancePetriNet(pn, log)` | Fitness against pre-built Petri net |
| `powl.filterByFitness(model, log, threshold)` | Filter traces by fitness |
| `powl.variants(log)` | Variant frequency map |
