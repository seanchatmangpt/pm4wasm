# POWL v2 Rust/WASM Documentation

Complete documentation for the POWL (Partially Ordered Workflow Language) Rust/WebAssembly implementation.

## Getting Started

**New to POWL?** Start here:

1. **[README](../README.md)** — Project overview, features, installation
2. **[Tutorial](./tutorial.md)** — Learn POWL concepts and basic usage
3. **[Quick Reference](./quick-reference.md)** — Common operations and patterns

## Central Paradigm: NL → POWL → BPMN

The primary use case: generate verified process models from natural language.

```python
from pm4py.algo.dspy.powl.natural_language import generate_powl_from_text
result = generate_powl_from_text("A customer orders a product...")
# CLI: python -m pm4py.cli DiscoverPOWLToBPMN "description..." output.bpmn
```

### Diátaxis Documentation

| Type | Document | Purpose |
|---|---|---|
| **Tutorial** | [NL Tutorial](./nl-tutorial.md) | Step-by-step: generate your first verified BPMN from NL |
| **How-To** | [NL Recipes](./nl-howto.md) | Patterns for XOR, LOOP, PO, escalation, multi-agent |
| **Explanation** | [NL Explanation](./nl-explanation.md) | How the pipeline works, why POWL v2, the XOR vs PO problem |
| **Reference** | [NL Reference](./nl-reference.md) | Complete API, CLI commands, POWL syntax, troubleshooting |

**Thesis:** [From Natural Language to Verified BPMN](../../docs/powl_v2_thesis.md) — PhD thesis with NL→POWL→BPMN as central paradigm.

## Core Documentation

### Architecture & Design

**[Architecture](./architecture.md)**
- Module organization and data structures
- Parsing pipeline and conversion algorithms
- Memory layout and WASM bindings
- Performance characteristics

### API Documentation

**[API Reference](./reference.md)**
- Complete API for all exported functions
- Type definitions and return values
- Error handling and edge cases
- Performance notes

### Guides & Examples

**[Examples](./examples.md)**
- Practical code snippets for common tasks
- Browser integration examples (React, Vue, vanilla JS)
- Event log processing and conformance checking
- Visualization and export

**[Quick Reference](./quick-reference.md)**
- Parsing models (simple, sequential, parallel, choice, loop)
- Model inspection and validation
- Transformation and conversion
- Analysis and footprints
- NL → POWL → BPMN CLI commands

### Support

**[Troubleshooting](./troubleshooting.md)**
- Build issues (wasm-pack, cargo, npm)
- Runtime errors (parsing, validation)
- Performance issues (memory, speed)
- Browser compatibility

## Vision & Roadmap

**[Vision 2030](./vision-2030.md)**
- NL→POWL→BPMN paradigm (2025.5 — complete)
- Event log processing in WASM
- Conformance checking and discovery
- SIMD acceleration and multi-threading
- LLM-guided simplification and federated process mining

## Documentation Structure

```
docs/
├── README.md              # This file
├── tutorial.md            # Getting started guide
├── reference.md           # Complete API reference
├── quick-reference.md     # Common operations reference
├── examples.md            # Practical code examples
├── architecture.md        # Implementation details
├── troubleshooting.md     # Common issues and solutions
└── vision-2030.md         # Future roadmap
```

## Quick Links

### By Task

| Task | Documentation |
|------|---------------|
| **Install and set up** | [README](../README.md) |
| **Learn POWL concepts** | [Tutorial](./tutorial.md) |
| **Parse a model** | [Quick Reference: Parsing](./quick-reference.md#parsing-models) |
| **Validate a model** | [Quick Reference: Validation](./quick-reference.md#validation) |
| **Check conformance** | [Examples: Conformance](./examples.md#conformance-checking) |
| **Convert to Petri net** | [API Reference: To Petri Net](./reference.md#to-petri-net) |
| **Extract footprints** | [Examples: Footprints](./examples.md#extract-footprints) |
| **Debug an error** | [Troubleshooting](./troubleshooting.md) |
| **Integrate in browser** | [Examples: Browser Integration](./examples.md#browser-integration) |
| **Understand architecture** | [Architecture](./architecture.md) |

### By Topic

| Topic | Documentation |
|-------|---------------|
| **POWL language** | [Tutorial: What is POWL?](./tutorial.md#what-is-powl) |
| **Node types** | [Architecture: Core Data Structures](./architecture.md#core-data-structures) |
| **Parsing** | [API Reference: Parsing](./reference.md#parsing) |
| **Validation** | [API Reference: Validation](./reference.md#validation) |
| **Transformation** | [API Reference: Model Manipulation](./reference.md#model-manipulation) |
| **Conversion** | [API Reference: Conversions](./reference.md#conversions) |
| **Analysis** | [API Reference: Analysis](./reference.md#analysis) |
| **Event logs** | [Examples: Event Log Processing](./examples.md#event-log-processing) |
| **Performance** | [Architecture: Performance Characteristics](./architecture.md#performance-characteristics) |
| **WASM bindings** | [Architecture: WASM Bindings](./architecture.md#wasm-bindings) |

## Contributing to Documentation

Found an error or want to improve the docs? Contributions welcome!

1. Edit the markdown file directly
2. Follow the existing structure and style
3. Add code examples with syntax highlighting
4. Update cross-references if adding new sections
5. Test all code examples before submitting

## Additional Resources

### External Resources

- **[pm4py Python library](https://pm4py.fit.fraunhofer.de/)** — Reference implementation
- **[Process Mining Handbook](https://www.springer.com/gp/book/9783642355248)** — Theory background
- **[POWL paper](https://doi.org/10.1007/978-3-030-96123-4_6)** — Academic foundation
- **[WebAssembly site](https://webassembly.org/)** — WASM documentation
- **[wasm-bindgen guide](https://rustwasm.github.io/wasm-bindgen/)** — FFI layer

### Related Projects

- **[pm4py-rust](https://github.com/pm4py/pm4py-rust)** — Native Rust implementation
- **[pm4py](https://github.com/pm4py/pm4py)** — Python process mining library

## Support

### Getting Help

- **Check the [Troubleshooting Guide](./troubleshooting.md)** — Common issues and solutions
- **Search [Issues](https://github.com/pm4py/pm4py/issues)** — Known bugs and feature requests
- **Ask a question** — Open a new issue with the `question` label

### Reporting Issues

When reporting issues, include:

1. **Environment info**
   - Browser name and version
   - Rust version (`rustc --version`)
   - wasm-pack version (`wasm-pack --version`)
   - Node.js version (`node --version`)

2. **Minimal reproducible example**
   - Smallest code snippet that shows the problem
   - Input data (POWL model or event log)

3. **Error messages**
   - Full stack trace
   - Console output
   - Screenshots (if applicable)

---

**Last updated:** 2026-04-06
