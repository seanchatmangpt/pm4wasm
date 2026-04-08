# pm4wasm Unification Plan

## Current State

**Already in pm4wasm/src/lib.rs (80+ WASM functions):**
- Core POWL: parse, validate, simplify, graph ops
- Conversions: Petri net, process tree, BPMN
- Event logs: XES/CSV parse/write
- Conformance: token replay, footprints, soundness
- Statistics: 20+ functions (start/end activities, variants, performance, etc.)
- Discovery: DFG, performance DFG, eventually follows, inductive/alpha miners
- Filtering: 15+ filter functions
- Complexity metrics
- Model diff

**Fragmented (needs unification):**
1. ~~**LLM/NL-to-POWL** → scattered in `pm4py/algo/dspy/powl/`~~ ✅ DONE
2. **BPMN conversion** → exists in both Python (`pm4py/objects/conversion/powl/variants/to_bpmn.py`) and WASM
3. ~~**Code generation** → in `pm4py/algo/dspy/powl/codegen.py`~~ ✅ DONE

## Unification Strategy

### Phase 1: Create Unified Module Structure

```
pm4wasm/src/
├── lib.rs                 # Main WASM exports (already comprehensive)
├── powl/                  # POWL core types (already exists)
├── parser.rs              # POWL parser (already exists)
├── binary_relation.rs     # Binary relations (already exists)
├── algorithms/            # Simplification, transitive ops (already exists)
├── conversion/            # All conversions (BPMN, Petri, process tree)
│   ├── mod.rs
│   ├── to_bpmn.rs         # Already exists, add BPMN 2.0 features
│   ├── to_petri_net.rs    # Already exists
│   └── to_process_tree.rs # Already exists
├── discovery/             # Process discovery algorithms
│   ├── mod.rs
│   ├── inductive_miner.rs # Already exists
│   ├── alpha_miner.rs     # Already exists
│   └── dfg.rs             # Already exists
├── conformance/           # Conformance checking
│   ├── mod.rs
│   ├── token_replay.rs    # Already exists
│   ├── footprints.rs       # Already exists
│   └── soundness.rs        # Already exists
├── statistics/            # Event log statistics
│   ├── mod.rs
│   ├── basic.rs           # Start/end activities, variants
│   └── performance.rs     # Case durations, arrival rates
├── filtering/             # Event log filtering
│   ├── mod.rs
│   ├── activities.rs      # Filter by activities
│   ├── attributes.rs      # Filter by attributes
│   ├── variants.rs        # Filter by variants
│   ├── time.rs            # Filter by time range
│   └── case_size.rs       # Filter by case size
├── event_log.rs           # XES/CSV parsing (already exists)
├── footprints.rs          # Behavioral footprints (already exists)
├── complexity.rs          # Complexity metrics (already exists)
├── diff.rs                # Model diff (already exists)
├── streaming.rs           # Streaming drift detection (already exists)
└── llm/                   # NEW: LLM/NL-to-POWL pipeline
    ├── mod.rs
    ├── natural_language.rs  # NL → POWL generation
    ├── judge.rs             # POWLJudge for structural validation
    ├── codegen.rs          # Code generation (n8n, Temporal, Camunda, YAWL)
    └── demos.rs            # Few-shot examples
```

### Phase 2: Port LLM Pipeline to WASM

**Goal:** Make the entire "Describe workflow → get executable BPMN" pipeline work in WASM.

**Components to port:**
1. **POWL generation from NL** → `llm/natural_language.rs`
   - Move logic from `pm4py/algo/dspy/powl/natural_language.py`
   - Use WASM-compatible DSPy or direct Groq API calls via `wasm-bindgen` + `js-sys`

2. **POWLJudge** → `llm/judge.rs`
   - Move validation logic from `pm4py/algo/dspy/powl/judge.py`
   - Structural soundness checking (deadlock-free, liveness, boundedness)

3. **Code generation** → `llm/codegen.rs`
   - Move from `pm4py/algo/dspy/powl/codegen.py`
   - Generate n8n JSON, Temporal Go, Camunda BPMN, YAWL v6 XML

4. **Few-shot demos** → `llm/demos.rs`
   - Move from `pm4py/algo/dspy/powl/nl_demos.py`
   - 15 demos across domains

**Challenge:** LLM API calls from WASM
- **Option A:** Use JavaScript fetch from WASM side (via `js-sys`)
- **Option B:** Pre-compute everything in Python, export only WASM-compatible operations
- **Option C:** Hybrid — LLM calls in JS, POWL operations in WASM

**Recommended:** Option C (Hybrid)
- Keep LLM API calls in JavaScript layer
- Export POWL parsing, validation, simplification, conversions to WASM
- JavaScript orchestrates: `NL text → (JS: LLM API) → POWL string → (WASM: parse/validate) → (WASM: to_bpmn)`

### Phase 3: Unified Conversion Module

Consolidate all POWL conversions into one place:

**Existing in pm4wasm:**
- `src/conversion/to_bpmn.rs`
- `src/conversion/to_petri_net.rs`
- `src/conversion/to_process_tree.rs`

**In Python (needs Rust equivalents):**
- `pm4py/objects/conversion/powl/variants/to_bpmn.py` → Already in WASM
- Add any missing BPMN 2.0 features (collaboration, choreography, extensions)

### Phase 4: Enhanced Discovery Algorithms

Add missing discovery algorithms to WASM:

**Already in WASM:**
- Inductive miner (`discover_process_tree_inductive`)
- Alpha miner (`discover_petri_net_alpha`)
- DFG (`discover_dfg`, `discover_performance_dfg`)

**Add to WASM:**
- Split miner (`discover_powl_split_miner`)
- IM/IMf (`discover_powl_im`)
- Heuristics miner (`discover_powl_heuristics`)

### Phase 5: Unified JavaScript API

Create a single `Powl` class in `js/src/index.ts` that wraps all WASM functions:

```typescript
export class Powl {
  // Core
  static async init(): Promise<Powl>
  parse(str: string): PowlModel
  validate(): void
  toString(): string
  simplify(): PowlModel

  // Conversions
  toPetriNet(): PetriNetResult
  toProcessTree(): ProcessTreeResult
  toBPMN(): string  // BPMN 2.0 XML

  // Event logs
  static parseXES(xml: string): EventLog
  static parseCSV(csv: string): EventLog
  static parseXESFile(file: File): Promise<EventLog>

  // Conformance
  conformance(log: EventLog): ConformanceResult
  footprints(): Footprints

  // Discovery
  static discoverFromLog(log: EventLog, algorithm?: 'inductive'|'alpha'|'split'): PowlModel

  // Statistics
  static getStartActivities(log: EventLog): ActivityStats
  static getVariants(log: EventLog): VariantStats
  // ... 20+ more stat functions

  // Filtering
  static filterStartActivities(log: EventLog, activities: string[]): EventLog
  static filterVariantsTopK(log: EventLog, k: number): EventLog
  // ... 15+ more filter functions

  // LLM pipeline (hybrid: JS + WASM)
  static async fromNaturalLanguage(text: string, llmConfig?: LLMConfig): Promise<PowlModel>
  generateCode(target: 'n8n'|'temporal'|'camunda'|'yawl'): string
}
```

## Implementation Status

### ✅ COMPLETED

**Phase 1** (COMPLETE): Module structure created
- `pm4wasm/src/llm/mod.rs` - LLM module bridge
- `pm4wasm/src/llm/judge.rs` - POWLJudge soundness validation
- `pm4wasm/src/llm/demos.rs` - Few-shot demos for 5 domains
- `pm4wasm/src/llm/codegen.rs` - Code generation (n8n, Temporal, Camunda, YAWL)
- `pm4wasm/src/lib.rs` - Updated to export LLM functions to WASM

**Phase 2** (COMPLETE): LLM pipeline ported to WASM
- Hybrid architecture: LLM API calls in JavaScript, POWL operations in WASM
- WASM exports: `validate_powl_structure()`, `get_demos_for_domain()`, `generate_code_from_powl()`
- JavaScript API: `Powl.fromNaturalLanguage()`, `Powl.naturalLanguageToCode()`
- Judge-refinement loop with up to 3 iterations

**Phase 3** (COMPLETE): Unified JavaScript API
- Enhanced `js/src/index.ts` with LLM pipeline methods
- Added type definitions: `FewShotDemo`, `LLMConfig`
- Complete NL → POWL → BPMN pipeline works in browser

### 🔄 IN PROGRESS

**Phase 4** (PENDING): Enhanced discovery algorithms
- Add Split miner (`discover_powl_split_miner`)
- Add IM/IMf (`discover_powl_im`)
- Add Heuristics miner (`discover_powl_heuristics`)

**Phase 5** (PENDING): Enhanced BPMN conversion
- Add BPMN 2.0 features (collaboration, choreography, extensions)

### 📋 TODO

- Unified documentation (`docs/unified-api.md`)
- Add Split miner, IM/IMf, Heuristics miner to WASM
- Enhanced BPMN 2.0 features

## Success Criteria

- [x] All POWL operations available via single `Powl` class in JavaScript
- [x] LLM-to-POWL pipeline works entirely in browser (LLM API call via JS, POWL via WASM)
- [ ] No functionality gap between Python pm4py and pm4wasm (discovery algorithms pending)
- [ ] Unified documentation (`docs/unified-api.md`)
- [x] All tests pass in both native Rust and WASM

## Next Steps

1. Create `pm4wasm/src/llm/` module structure
2. Port LLM pipeline logic to Rust (with JS bridge for LLM API calls)
3. Update `js/src/index.ts` with unified `Powl` class
4. Update documentation to reflect unified architecture
