# pm4wasm — Claude Code Configuration

pm4wasm is a Rust/WebAssembly process mining library (v26.4.8, Apache-2.0) — WASM port of pm4py with browser-native LLM integration. ~25,300 lines of Rust, 117 wasm_bindgen exports.

**REQUIRED READING:**
- **Root instructions**: See `../CLAUDE.md` (Constitutional Law, Evidence Standards, Git Safety, Dependencies)

## Setup

```bash
rustup update && rustup target add wasm32-unknown-unknown
cd js/ && npm install
brew install graphviz  # macOS (visualization)
```

## Commands

### Rust / WASM (repo root)

```bash
cargo check                      # Type-check
cargo test                       # 290 pass, 2 fail (LLM — ignore)
cargo test --lib foo::tests      # Single test module
cargo fmt && cargo clippy        # Format + lint
wasm-pack build --target bundler --release --out-dir js/pkg
ls -lh js/pkg/pm4wasm_bg.wasm    # Size check (budget: <500KB gzipped)
```

No `Makefile` — use `cargo`/`wasm-pack` directly.

### TypeScript / npm (`js/`)

```bash
npm run build                    # Full build (WASM + TS)
npm run dev                      # Dev server at http://localhost:5173
npm run demo                     # Live process model editor
npm test                         # Full suite (TS + WASM)
npm run test:ts                  # Vitest only
npx vitest run src/path.test.ts  # Single TS test
npm run test:wasm                # Firefox headless
npm run test:watch               # Watch mode
```

### Docker

```bash
docker-compose up                # pm4wasm + PostgreSQL + Redis (no Dockerfile exists)
```

## WASM Export Pattern

All `#[wasm_bindgen]` exports live in `src/lib.rs` only — never in submodule files.

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

Every export double-serializes (JSON parse → process → JSON serialize) — necessary for wasm-bindgen, not fixable.

## Core Architecture Patterns

### Arena-Based Memory (`src/powl.rs:116-370`)

wasm-bindgen can't handle recursive trait objects, so POWL nodes live in a flat `Vec<PowlNode>` referenced by `u32` indices.

```rust
pub struct PowlArena { pub nodes: Vec<PowlNode>, next_transition_id: u32 }
```

- Root = `model.len() - 1` (last added node)
- No automatic GC — free with `model = null` in JS
- Arena indices go stale after mutations — always use fresh indices
- Deep copy requires explicit `copy_subtree()`

### Bit-Packed Relations (`src/binary_relation.rs`)

`u64` bitset rows for O(1) edge lookup. Transitive closure/reduction via Warshall's O(n³).

### Clone-Heavy Pattern

Arena indices lack lifetime guarantees → extensive `.clone()` calls throughout (e.g., `astar.rs`). Trade-off for WASM-safe memory.

## Known Test Failures

Always ignore (external API deps): `llm::demos::tests::test_get_demos`, `llm::judge::tests::test_validate_sound_loop_with_exit`

## Key Stats

- **`lib.rs`**: 2461 lines, 117 exports — all FFI in one file by design
- **Tests**: 290 passing, 2 failing (LLM)
- **WASM binary**: ~1.5MB uncompressed, ~440KB gzipped
- **`Cargo.toml`** says "POWL v2" (legacy) — library covers all pm4py algorithms
- **Zero `unsafe` blocks** in entire codebase

## Architecture

### Rust Modules (`src/`)

| Module | Purpose |
|--------|---------|
| `lib.rs` | wasm-bindgen FFI layer, arena storage |
| `powl.rs` | POWL node types + PowlArena |
| `parser.rs` | POWL model string parser (Python `__repr__` format) |
| `binary_relation.rs` | Bit-packed adjacency matrix, Warshall's |
| `event_log.rs` | XES/CSV event log parsing |
| `petri_net.rs` | Petri net (Place, Transition, Arc) |
| `process_tree.rs` | Process tree conversion |
| `footprints.rs` | Behavioral signature extraction |
| `discovery/` | Alpha, Alpha+, Inductive, DFG, heuristics, correlation, declare, genetic, log skeleton, temporal profile, transition system, performance spectrum, batches |
| `conformance/` | Token replay, A* alignments, precision, footprints, soundness |
| `conversion/` | BPMN, PNML, PTML, DFG, Petri nets, process trees, YAWL (bidirectional) |
| `algorithms/` | Marking equation, reduction, simplification, transitive, label replacing |
| `filtering/` | Activity, attribute, case size, time, variants |
| `streaming.rs` | EWMA drift detection |
| `diff.rs` | Behavioral diff between POWL models |
| `llm/` | NL→POWL generation (judge.rs, demos.rs, codegen.rs) |
| `ocel/` | Object-centric logs (flattening.rs, etot.rs) |
| `statistics/` | Basic + performance statistics |
| `quality/` | Generalization metrics |
| `simulation/` | Playout simulation |
| `transformation/` | Log-to-trie transformation |

### Module Dependency Graph

```
lib.rs (117 exports)
├── powl.rs → binary_relation.rs
├── parser.rs → powl.rs
├── event_log.rs
├── petri_net.rs → event_log.rs
├── footprints.rs → powl.rs
├── discovery/ → process_tree.rs, dfg.rs, event_log.rs
├── conformance/ → petri_net.rs, footprints.rs
├── conversion/ → powl.rs, process_tree.rs
├── streaming.rs → conformance/
├── llm/ (judge, demos, codegen)
└── algorithms/ → binary_relation.rs
```

### TypeScript Client (`js/src/`)

```
index.ts (lazy WASM loader)
├── types.ts, utils.ts
├── validation.ts (POWL soundness)
├── llm-prompts.ts (4-strategy prompting)
├── model-generator.ts (POWL builder)
├── refinement-loop.ts (iterative improvement)
├── error-handler.ts (two-tier: critical 5 iterations, adjustable 2)
├── process-modeling-service.ts (orchestration)
├── prompt-templates.ts, predictive.ts (micro-ml)
```

## LLM Integration

Four-strategy prompting (Kourani et al. 2024) in `js/src/llm-prompts.ts`: role, knowledge injection, few-shot (`src/llm/demos.rs`), negative prompting. Providers: Groq (primary), OpenAI, Anthropic. Env vars: `GROQ_API_KEY`, `OPENAI_API_KEY`, `ANTHROPIC_API_KEY`.

## Publishing

`cd js/ && npm run build && npm pack --dry-run && npm publish`

**Missing governance files**: CONTRIBUTING.md, CHANGELOG.md, CODE_OF_CONDUCT.md, SECURITY.md don't exist yet but are referenced in README.
**License inconsistency**: SAAS.md says AGPL-3.0 but LICENSE file, Cargo.toml, and README all say Apache-2.0. SAAS.md needs fixing.

## Browser-Specific Issues

| Issue | Fix |
|-------|-----|
| OOM with large models (>1000 nodes) | `model = null` in JS; WASM default is 1 GB |
| Safari <15 WASM failure | `RUSTFLAGS='-C target-feature=-bulk-memory' wasm-pack build` |
| Wrong MIME type | Server must return `application/wasm` |
| CORS on WASM file | Same-origin or CORS-enabled |
| Debug WASM in DevTools | `RUST_LOG=debug npm run build:wasm` then Sources → pm4wasm_bg.wasm |

## Troubleshooting

| Issue | Fix |
|-------|-----|
| `quick_xml` self-closing elements | Handle both `Empty` and `Start`/`End` events |
| `#[allow(dead_code)]` on struct fields | Place on each field directly, not the struct |
| WASM build fails macOS ARM64 | `rustup update` |
| `getrandom` not available | `getrandom = { version = "0.2", features = ["js"] }` in Cargo.toml |
| `final` reserved keyword | Rename to `is_final` with `#[serde(rename = "final")]` |
| WASM binary too large | `wasm-opt = false` by default; set `true` or run `wasm-opt` manually |

## Docs (Diátaxis)

- `docs/architecture.md` — Technical deep dive
- `docs/reference.md` — Full API reference (117 functions)
- `docs/troubleshooting.md` — Common issues
- `docs/llm-integration.md`, `docs/nl-tutorial.md`, `docs/nl-reference.md` — LLM docs
- `docs/tutorial.md`, `docs/quick-reference.md`, `docs/examples.md` — Getting started
- `docs/howto/build-from-source.md` — Build guide

All docs use YAML frontmatter with `signal:` encoding.

**README frontmatter pattern**: Uses Signal Theory encoding `S=(Mode, Genre, Type, Format, Structure)`. Preserve this pattern in rewrites.
**Example**: `signal: S=(linguistic, overview, inform, markdown, navigation)`

## Project Identity

**pm4wasm = pm4py + wasm** — NOT "POWL v2". `Powl` JS class name is historical; library covers all pm4py algorithms.

**Scoped npm package**: `@seanchatmangpt/pm4wasm` (NOT `pm4wasm`) — use this name in all installation instructions.
**Brand drift warning**: Some docs/ files still reference "POWL v2" in titles (INDEX.md, tutorial.md, architecture.md, vision-2030.md). This is legacy naming to be fixed.
