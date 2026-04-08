# Changelog

All notable changes to pm4wasm will be documented in this file.

## [26.4.8] - 2026-04-08

### Added
- **Alpha+ miner** (`discover_petri_net_alpha_plus`) - Extended Alpha miner handling loops of length 1 (A→A) and length 2 (A→B→A), plus non-free-choice constructs
- **Prefix tree discovery** (`discover_prefix_tree`) - Trie (prefix tree) data structure with optional max_path_length parameter  
- **DFG typed** (`discover_dfg_typed`) - Typed DFG object return format with (from, to, frequency) triples
- **OCEL support** - Object-centric event log parsing and analysis:
  - `parse_ocel_json()` - Parse JSON-OCEL 1.0/2.0 format
  - `ocel_flatten_by_object_type()` - Flatten OCEL to traditional EventLog by object type
  - `discover_ocel_etot()` - Event-Type / Object-Type graph discovery
  - `ocel_get_summary()` - OCEL statistics (event/object counts, types)
  - `ocel_get_object_types()` - List all object types
  - `ocel_get_event_types()` - List all event types
- **Trie data structure** - Complete trie implementation with nodes, children, final flags, and depth tracking

### Changed
- Fixed WASM build by adding `getrandom` with `js` feature for WASM compatibility
- Improved npm package configuration with explicit WASM file listings
- Updated README.md with new features (Alpha+ miner, prefix tree, DFG typed, OCEL support)
- Updated docs/architecture.md with new modules (trie.rs, ocel/, transformation/, discovery/)
- Updated docs/reference.md with new WASM exports (discover_petri_net_alpha_plus, discover_prefix_tree, discover_dfg_typed, OCEL functions)
- Updated main README license from AGPL-3.0 to Apache-2.0 for consistency

### Fixed
- Reserved keyword issue in TrieNode (`final` → `is_final` with serde rename)
- Function name conflicts in flattening module (renamed local `get_variants` to `get_variants_from_log`)
- Added `#[serde(default)]` to OCEL `o2o` and `e2e` fields for optional JSON parsing
- Removed unused wrapper functions for non-exported filter functions

### Test Coverage
- 290 tests passing (2 pre-existing LLM test failures unrelated to new features)
- 17 new OCEL tests covering parsing, flattening, and ETOT discovery
- 5 new Alpha+ miner tests covering loop-1 and loop-2 patterns
- 5 new prefix tree tests including edge cases
- All DFG typed tests passing

### Technical Details
- **WASM binary size**: ~1.5 MB (pm4wasm_bg.wasm)
- **Package size**: ~1.0 MB (npm tarball)
- **Total files**: 81 files in npm package
- **Dependencies**: Includes AI SDKs (Anthropic, Groq, OpenAI) for LLM integration features

## [26.4.7] - 2026-04-07

### Added
- **LLM Integration**: Natural language to POWL generation using Vercel AI SDK v7
  - Multi-provider support: Groq, OpenAI, Anthropic
  - Domain-specific few-shot demos (5 domains: loan_approval, software_release, ecommerce, manufacturing, healthcare)
  - Automatic validation and refinement loop (up to 3 iterations)
- **Code Generation**: Generate workflow code from POWL models
  - n8n JSON workflows
  - Temporal Go workflows
  - Camunda BPMN XML
  - YAWL v6 XML
- **Validation**: POWL structure validation with soundness checking
- **WASM Functions**:
  - `validate_powl_structure()` - Validate POWL models
  - `get_demos_for_domain()` - Get few-shot examples
  - `generate_code_from_powl()` - Code generation

### Changed
- **License**: Changed from AGPL-3.0 to Apache-2.0 (matching pm4py)
- **Versioning**: Adopted CalVer (Calendar Versioning) - v26.4.7 = 2026 April, week 7, build 7
- **Dependencies**: Updated to latest Vercel AI SDK v7.0.0-beta.72

### Fixed
- TypeScript strict mode compliance (zero `any` types, no unused variables)
- Vercel AI SDK v7 API compatibility
- Provider factory pattern for Groq, OpenAI, Anthropic

### Technical Details
- **Build**: `wasm-pack` for Rust → WASM, `vite` for TypeScript bundling
- **Browser Support**: 100% browser-native, no server required
- **Type Safety**: Full TypeScript strict mode, comprehensive type definitions

## [0.2.0] - 2025-04-07

### Added
- Initial WASM bindings for POWL v2
- Process model parsing and validation
- Petri net conversion
- Conformance checking with token replay
- Event log parsing (XES, CSV)
- Footprints extraction
- Model simplification

[26.4.7]: https://github.com/seanchatmangpt/pm4wasm/releases/tag/v26.4.7
[0.2.0]: https://github.com/seanchatmangpt/pm4wasm/releases/tag/v0.2.0
