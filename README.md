---
title: "pm4wasm: Process Mining in the Browser"
type: navigation
signal: S=(linguistic, brief, inform, markdown, minimal)
relates_to:
  - docs/tutorial.md
  - docs/reference.md
  - docs/architecture.md
---

# pm4wasm

Process mining in the browser. A WebAssembly port of [pm4py](https://pm4py.fit.fraunhofer.de/) — the Python process mining library — with LLM integration for natural language process discovery. No server, no data upload — 100% client-side.

[![npm](https://img.shields.io/npm/v/pm4wasm)](https://www.npmjs.com/package/pm4wasm)
[![Rust](https://img.shields.io/badge/rust-1.70%2B-orange.svg)](https://www.rust-lang.org)
[![License](https://img.shields.io/badge/license-Apache--2.0-blue.svg)](./LICENSE)

## Quick Start

```typescript
import { Powl } from 'pm4wasm';

const powl = await Powl.init();
const model = powl.parse('PO=(nodes={A, B, C}, order={A-->B, B-->C})');
console.log([...model.activities()]);  // ["A", "B", "C"]
```

```bash
npm install pm4wasm
```

## Documentation

### Learning (Tutorials)

Step-by-step lessons for getting started.

- [Getting Started](docs/tutorial.md) — Parse, validate, and convert your first process model (30 min)
- [Natural Language to Process Models](docs/nl-tutorial.md) — Generate process models from text descriptions (20 min)

### Problem Solving (How-To Guides)

Solutions for specific tasks.

- [Build from Source](docs/howto/build-from-source.md) — Compile WASM and TypeScript from source
- [Common Operations](docs/quick-reference.md) — Parsing, conversion, conformance, filtering
- [Troubleshooting](docs/troubleshooting.md) — Known issues and fixes

### Understanding (Explanations)

How and why things work.

- [Architecture](docs/architecture.md) — WASM engine internals, arena storage, binary relations
- [LLM Integration](docs/llm-integration.md) — Natural language pipeline and multi-provider support
- [Process Model Notation](docs/nl-explanation.md) — POWL operators and partial orders

### Reference (Lookup)

Technical details for lookup.

- [API Reference](docs/reference.md) — Complete function and type listing
- [NL API Reference](docs/nl-reference.md) — Natural language processing API
- [Code Examples](docs/examples.md) — Annotated examples for common workflows

## Features

**Process Discovery** — Alpha, Alpha+, Inductive, DFG, correlation, genetic, heuristics, log skeleton, temporal profile

**Conformance Checking** — Token-replay fitness, alignment-based precision, soundness verification

**Event Log Processing** — XES and CSV parsing, 15+ log filters, 20+ statistics

**Model Conversion** — POWL, Petri nets, process trees, BPMN, PNML, YAWL

**Object-Centric Logs** — OCEL JSON parsing, flattening, ETOT discovery

**LLM Integration** — Natural language to process models, multi-provider (Groq, OpenAI, Anthropic)

**Streaming** — Drift detection with EWMA smoothing

## Links

- [npm package](https://www.npmjs.com/package/pm4wasm) — `npm install pm4wasm`
- [pm4py](https://pm4py.fit.fraunhofer.de/) — Python reference implementation
- [License](./LICENSE) — Apache-2.0
