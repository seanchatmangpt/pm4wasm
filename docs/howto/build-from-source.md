---
title: "Build pm4wasm from Source"
type: how-to
signal: S=(linguistic, how-to, direct, markdown, troubleshooting-guide)
relates_to:
  - ../architecture.md
  - ../troubleshooting.md
---

# Build pm4wasm from Source

## Prerequisites

- Rust 1.70+ with `wasm32-unknown-unknown` target
- [wasm-pack](https://rustwasm.github.io/wasm-pack/)
- Node.js 18+

## Install wasm-pack (once)

```bash
curl https://rustwasm.github.io/wasm-pack/installer/init.sh -sSf | sh
```

## Build WASM Module

```bash
wasm-pack build --target bundler --release
```

Output: `pkg/pm4wasm.js` + `pkg/pm4wasm_bg.wasm`

## Build TypeScript Client

```bash
cd js/
npm install
npm run build:wasm    # Build WASM via wasm-pack
npm run build:ts      # Build TypeScript bundle
npm run build         # Full build (WASM + TS)
```

## Development Server

```bash
cd js/
npm run dev           # http://localhost:5173
npm run demo          # Live POWL editor + conformance checker
```

## Run Tests

```bash
cargo test                        # Rust tests (native)
cd js/ && npm test                # WASM tests (Firefox headless)
wasm-pack test .. --headless --chrome  # Chrome headless
```

## Troubleshooting

| Issue | Fix |
|-------|-----|
| `wasm32-unknown-unknown` not found | `rustup target add wasm32-unknown-unknown` |
| Graphviz `ExecutableNotFound` | `brew install graphviz` |
| `getrandom` not available for WASM | Add `getrandom = { version = "0.2", features = ["js"] }` to Cargo.toml |
| WASM build fails on macOS ARM64 | `rustup update` |
