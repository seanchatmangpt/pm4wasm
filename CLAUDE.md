# CLAUDE.md

pm4wasm is a Rust/WebAssembly process mining library (v26.4.8, Apache-2.0) — WASM port of pm4py with browser-native POWL v2 and LLM integration.

## Setup

```bash
# Rust/WASM
rustup update                    # Ensure latest Rust
rustup target add wasm32-unknown-unknown

# Node.js/TypeScript (js/ client)
cd js/
npm install

# Prerequisites
brew install graphviz             # macOS (visualization, for Python interop tests)
```

## Commands

### Rust / WASM

```bash
# From repository root
cargo check                      # Type-check (fast)
cargo test                       # 290 tests passing
cargo fmt                        # Format code
cargo clippy                     # Lint

# Build WASM package
wasm-pack build --target bundler --release --out-dir js/pkg

# WASM binary size budget: <500KB gzipped (currently ~1.5MB uncompressed, ~374KB gzipped)
```

No `Makefile` — use `cargo`/`wasm-pack` directly (global `cargo make` rule doesn't apply here).

### TypeScript / npm

```bash
cd js/

npm install                      # Install dependencies
npm run build:wasm               # Build WASM via wasm-pack
npm run build:ts                 # Build TypeScript bundle
npm run build                    # Full build (WASM + TS)

npm run dev                      # Start dev server at http://localhost:5173
npm run demo                     # Demo page with live POWL editor

npm test                         # Full test suite (TypeScript + WASM)
npm run test:ts                  # TypeScript tests only (vitest)
npm run test:wasm                # WASM tests (wasm-pack, Firefox headless)
npm run test:watch               # Watch mode for TypeScript tests

npm run prepublishOnly           # Build before publishing to npm
```

### Docker

```bash
# Build WASM server image
docker build -t pm4wasm .

# Or use docker-compose
docker-compose up                # Starts pm4wasm + PostgreSQL + Redis
```

## WASM Export Pattern

All browser-callable functions use `#[wasm_bindgen]` in `src/lib.rs`:

```rust
#[wasm_bindgen]
pub fn my_function(input_json: &str) -> Result<String, JsValue> {
    let input: MyType = serde_json::from_str(input_json)
        .map_err(|e| JsValue::from_str(&format!("JSON error: {}", e)))?;
    let result = my_module::do_thing(&input);
    serde_json::to_string(&result)
        .map_err(|e| JsValue::from_str(&format!("JSON error: {}", e)))
}
```

**Prefer `lib.rs` wrappers** over `#[wasm_bindgen]` in individual module files. This keeps the FFI layer centralized and makes TypeScript type generation cleaner.

### Error Handling

All `Result<String, JsValue>` functions throw JavaScript `Error` on failure:

```rust
#[wasm_bindgen]
pub fn parse_powl(s: &str) -> Result<PowlModel, JsValue> {
    if let Err(e) = parse_powl_model_string(s, &mut arena) {
        return Err(JsValue::from_str(&format!("Parse error: {}", e)));
    }
    Ok(model)
}
```

## Known Test Failures

Two LLM tests always fail (pre-existing, not caused by new work):
- `llm::demos::tests::test_get_demos`
- `llm::judge::tests::test_validate_sound_loop_with_exit`

These are related to external API dependencies and can be ignored.

## Architecture

### Rust Modules (`src/`)

| Module | Purpose |
|--------|---------|
| `lib.rs` | wasm-bindgen entry point, arena-based storage, FFI layer |
| `powl.rs` | POWL v2 node types (Transition, Operator, SPO, FrequentTransition) |
| `parser.rs` | POWL model string parser (same format as Python `__repr__`) |
| `binary_relation.rs` | Bit-packed adjacency matrix with Warshall's algorithm |
| `petri_net.rs` | Petri net conversion (Place, Transition, Arc) |
| `process_tree.rs` | Process tree conversion (hierarchical operators) |
| `footprints.rs` | Behavioral signature extraction |
| `event_log.rs` | XES/CSV event log parsing |
| `trie.rs` | Trie (prefix tree) data structure for log analysis |
| `streaming.rs` | Streaming drift detection with EWMA smoothing |
| `diff.rs` | Behavioral diff between two POWL models |
| `complexity.rs` | Model complexity metrics |
| `discovery/` | Process discovery (Alpha, Alpha+, Inductive, DFG, causal, heuristics, etc.) |
| `conformance/` | Token replay, alignments, precision, footprints conformance, soundness |
| `conversion/` | BPMN, PNML, PTML, DFG, Petri nets, YAWL (bidirectional) |
| `algorithms/` | Marking equation, reduction, simplification, transitive operations |
| `ocel/` | Object-Centric Event Log support (parsing, flattening, ETOT) |
| `transformation/` | Log-to-trie transformation and other log operations |
| `quality/` | Generalization metrics |
| `statistics/` | Basic and performance statistics |
| `filtering/` | Activity, attribute, case size, time, variant filtering |
| `simulation/` | Playout simulation |
| `llm/` | Natural language to POWL generation, code generation, validation |

### TypeScript Client (`js/`)

- **`src/index.ts`** — Main TypeScript wrapper around WASM exports
- **`src/llm-*.ts`** — LLM integration (Vercel AI SDK v7, Anthropic/Groq/OpenAI)
- **`src/validation.ts`** — POWL structure validation
- **`src/error-handler.ts`** — Centralized error handling
- **`package.json`** — npm package configuration ("pm4wasm")
- **`vite.config.ts`** — Vite bundler for TypeScript
- **`vitest.config.ts`** — Test runner configuration

Full module docs: [docs/architecture.md](docs/architecture.md)

## Publishing to npm

```bash
cd js/

# 1. Update version in package.json
# 2. Build WASM + TypeScript
npm run build

# 3. Dry-run to inspect package
npm pack --dry-run

# 4. Publish
npm publish

# Package name: "pm4wasm"
# Repository: https://github.com/seanchatmangpt/pm4wasm
```

The npm package includes:
- `dist/` — TypeScript bundle (index.js, index.d.ts)
- `pkg/` — WASM files (pm4wasm_bg.wasm, pm4wasm.js, type definitions)

## Troubleshooting

| Issue | Fix |
|-------|-----|
| `quick_xml` misses self-closing elements | Handle both `Empty` (self-closing `<tag/>`) and `Start`/`End` (non-self-closing `<tag>...</tag>`) events |
| `#[allow(dead_code)]` on struct doesn't suppress field warnings | Place `#[allow(dead_code)]` on each unused field directly |
| Graphviz `ExecutableNotFound` | `brew install graphviz` |
| WASM build fails on macOS ARM64 | `rustup update` |
| `getrandom` not available for WASM | Add `getrandom = { version = "0.2", features = ["js"] }` to Cargo.toml dependencies |
| Reserved keyword `final` in Rust struct | Rename to `is_final` with `#[serde(rename = "final")]` |
| TypeScript strict mode errors | No `any` types, all variables used, proper type annotations |

## CI/CD

GitHub Actions workflows (`.github/workflows/`):
- `wasm-ci.yml` — Rust tests, WASM build, TypeScript tests, type checking
- `server-ci.yml` — Node.js server tests
- `saas-sdk-ci.yml` — SaaS SDK tests
- `deploy-production.yml` — Production deployment

## Test Data

Test event logs and models are included in the repository for development and testing.
