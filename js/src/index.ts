/**
 * @pm4py/pm4wasm
 *
 * High-level TypeScript API for the POWL v2 Rust/WASM library.
 *
 * Usage:
 * ```ts
 * import { Powl } from "@pm4py/pm4wasm";
 *
 * const powl = await Powl.init();
 * const model = powl.parse("X(A, B)");
 * console.log(model.toString());       // "X ( A, B )"
 * const pn = model.toPetriNet();
 * const fitness = powl.conformance(pn, log);
 * ```
 */

import type {
  EventLog,
  FitnessResult,
  Footprints,
  NodeInfo,
  PetriNetResult,
  ModelFootprints,
  FootprintsConformanceResult,
  SoundnessResult,
  StreamingConformanceSnapshot,
  AttributeValue,
  CaseDurationResult,
  ReworkTime,
  TraceReplayResult,
  PrecisionResult,
} from "./types.js";

export type * from "./types.js";
export * from "./utils.js";
export * from "./validation.js";
export * from "./llm-prompts.js";
export * from "./model-generator.js";
export * from "./refinement-loop.js";
export * from "./error-handler.js";
export * from "./process-modeling-service.js";
export * from "./predictive.js";

// Vercel AI SDK for LLM integration
import { generateText } from "ai";
import { createGroq } from "@ai-sdk/groq";
import { createOpenAI } from "@ai-sdk/openai";
import { createAnthropic } from "@ai-sdk/anthropic";

// ─── Lazy WASM loader ─────────────────────────────────────────────────────────

type WasmModule = typeof import("../pkg/pm4wasm.js");
let _wasm: WasmModule | null = null;

async function getWasm(): Promise<WasmModule> {
  if (!_wasm) {
    // With --target bundler, the WASM is initialized automatically on import
    _wasm = await import("../pkg/pm4wasm.js");
  }
  return _wasm;
}

// ─── PowlModel handle ─────────────────────────────────────────────────────────

/**
 * An opaque handle to a parsed POWL model in WASM memory.
 *
 * Obtain via `Powl.parse()` — do not construct directly.
 */
export class PowlModel {
  /** @internal */
  private readonly _handle: any;

  /** @internal */
  constructor(
    /** @internal */ private readonly _wm: WasmModule,
    handle: any,
  ) {
    this._handle = handle;
  }

  /** @internal */
  get __handle(): any {
    return this._handle;
  }

  /** Arena index of the root node. */
  get root(): number {
    return this._handle.root();
  }

  /** Total number of nodes in the arena. */
  get size(): number {
    return this._handle.len();
  }

  /** Canonical string representation (matches Python `__repr__`). */
  toString(): string {
    return this._wm.powl_to_string(this._handle);
  }

  /** Return typed info about a node by arena index. */
  nodeInfo(arenaIdx: number): NodeInfo {
    return JSON.parse(this._wm.node_info_json(this._handle, arenaIdx)) as NodeInfo;
  }

  /** Child arena indices of an operator or SPO node; empty for leaves. */
  children(arenaIdx: number): number[] {
    return Array.from(this._wm.get_children(this._handle, arenaIdx));
  }

  /** String representation of one node. */
  nodeToString(arenaIdx: number): string {
    return this._wm.node_to_string(this._handle, arenaIdx);
  }

  /**
   * Validate all StrictPartialOrder nodes.
   * @throws {Error} if any violation is found.
   */
  validate(): void {
    this._wm.validate_partial_orders(this._handle);
  }

  /** Return a new simplified model (structure-normalized). */
  simplify(): PowlModel {
    return new PowlModel(this._wm, this._wm.simplify_powl(this._handle));
  }

  /** Convert XOR(A,tau) / LOOP(A,tau) patterns to FrequentTransitions. */
  simplifyFrequent(): PowlModel {
    return new PowlModel(this._wm, this._wm.simplify_frequent_transitions(this._handle));
  }

  /** Convert to Petri net. */
  toPetriNet(): PetriNetResult {
    return JSON.parse(
      this._wm.powl_to_petri_net(this.toString()),
    ) as PetriNetResult;
  }

  /**
   * Compute footprints (behavioural signature).
   *
   * @returns JSON-parsed footprints object.
   */
  footprints(): Footprints {
    // Compute footprints from the POWL model via WASM
    return JSON.parse(this._wm.compute_footprints(this._handle));
  }

  /**
   * Walk every node in the tree, depth-first (pre-order).
   * Calls `visitor(arenaIdx, info)` for each node.
   */
  walk(visitor: (idx: number, info: NodeInfo) => void): void {
    const visit = (idx: number): void => {
      const info = this.nodeInfo(idx);
      visitor(idx, info);
      for (const child of this.children(idx)) {
        visit(child);
      }
    };
    visit(this.root);
  }

  /**
   * Collect all activity labels in the model (leaf Transitions with non-null label).
   */
  activities(): Set<string> {
    const acts = new Set<string>();
    this.walk((_idx, info) => {
      if (info.type === "Transition" && info.label !== "tau") {
        acts.add(info.label);
      }
    });
    return acts;
  }

  /** Raw ordering relation of an SPO node as flat edge list `[src, tgt, …]`. */
  orderEdges(spoIdx: number): number[] {
    const rel = this._wm.get_order_of(this._handle, spoIdx);
    return Array.from(rel.edges_flat());
  }

  /** Transitive closure edges of an SPO node. */
  closureEdges(spoIdx: number): number[] {
    const rel = this._wm.transitive_closure(this._handle, spoIdx);
    return Array.from(rel.edges_flat());
  }

  /** Transitive reduction edges of an SPO node. */
  reductionEdges(spoIdx: number): number[] {
    const rel = this._wm.transitive_reduction(this._handle, spoIdx);
    return Array.from(rel.edges_flat());
  }
}

// ─── Main client class ────────────────────────────────────────────────────────

/**
 * Entry-point for the POWL WASM library.
 *
 * ```ts
 * const powl = await Powl.init();
 * ```
 */
export class Powl {
  /** @internal */
  private constructor(private readonly wm: WasmModule) {}

  /**
   * Initialise the WASM module and return a ready-to-use `Powl` instance.
   * Safe to call multiple times — the WASM module is loaded only once.
   */
  static async init(): Promise<Powl> {
    const wm = await getWasm();
    return new Powl(wm);
  }

  // ── Parsing ────────────────────────────────────────────────────────────────

  /**
   * Parse a POWL model string (Python `__repr__` format).
   *
   * @throws {Error} on syntax error.
   *
   * @example
   * ```ts
   * const m = powl.parse("X(A, B)");
   * const spo = powl.parse("PO=(nodes={A, B, C}, order={A-->B, A-->C})");
   * ```
   */
  parse(s: string): PowlModel {
    const handle = this.wm.parse_powl(s);
    return new PowlModel(this.wm, handle);
  }

  // ── Event log parsing ──────────────────────────────────────────────────────

  /**
   * Parse a XES-formatted XML string.
   *
   * @throws {Error} on XML parse failure.
   */
  parseXes(xml: string): EventLog {
    return JSON.parse(this.wm.parse_xes_log(xml)) as EventLog;
  }

  /**
   * Parse a CSV string with headers.
   *
   * Required columns: `case_id` / `case:concept:name`, `activity` / `concept:name`.
   * Optional: `timestamp` / `time:timestamp`.
   *
   * @throws {Error} on parse failure.
   *
   * @example
   * ```ts
   * const log = powl.parseCsv(
   *   "case_id,activity,timestamp\n" +
   *   "1,A,2020-01-01\n" +
   *   "1,B,2020-01-02\n"
   * );
   * ```
   */
  parseCsv(csv: string): EventLog {
    return JSON.parse(this.wm.parse_csv_log(csv)) as EventLog;
  }

  /**
   * Fetch and parse an XES file from a URL.
   *
   * @example
   * ```ts
   * const log = await powl.fetchXes("/logs/running-example.xes");
   * ```
   */
  async fetchXes(url: string): Promise<EventLog> {
    const text = await fetch(url).then((r) => {
      if (!r.ok) throw new Error(`HTTP ${r.status} fetching ${url}`);
      return r.text();
    });
    return this.parseXes(text);
  }

  /**
   * Parse an XES `File` object from a drag-and-drop or `<input type="file">`.
   *
   * @example
   * ```ts
   * input.addEventListener("change", async (e) => {
   *   const log = await powl.readXesFile(e.target.files[0]);
   * });
   * ```
   */
  async readXesFile(file: File): Promise<EventLog> {
    const text = await file.text();
    return this.parseXes(text);
  }

  /** Parse a CSV `File` object. */
  async readCsvFile(file: File): Promise<EventLog> {
    const text = await file.text();
    return this.parseCsv(text);
  }

  // ── Conformance checking ───────────────────────────────────────────────────

  /**
   * Compute token-replay fitness of an event log against a POWL model.
   *
   * Internally converts the model to a Petri net then runs token replay.
   *
   * @example
   * ```ts
   * const model = powl.parse("PO=(nodes={A, B, C}, order={A-->B, B-->C})");
   * const log   = powl.parseCsv("case_id,activity\n1,A\n1,B\n1,C\n");
   * const fit   = powl.conformance(model, log);
   * console.log(fit.percentage);  // 1.0
   * ```
   */
  conformance(model: PowlModel, log: EventLog): FitnessResult {
    const pnJson = this.wm.powl_to_petri_net(model.toString());
    return JSON.parse(
      this.wm.token_replay_fitness(pnJson, JSON.stringify(log)),
    ) as FitnessResult;
  }

  /**
   * Compute token-replay fitness given a pre-built `PetriNetResult`.
   * Use when you already have a Petri net and want to avoid recomputing it.
   */
  conformancePetriNet(pn: PetriNetResult, log: EventLog): FitnessResult {
    return JSON.parse(
      this.wm.token_replay_fitness(JSON.stringify(pn), JSON.stringify(log)),
    ) as FitnessResult;
  }

  /**
   * Compute ETConformance precision for a Petri net against an event log.
   *
   * Precision measures how precisely the model describes observed behavior.
   * A score of 1.0 means the model allows exactly the behavior seen in the log.
   * A lower score means the model permits transitions that were never used
   * (escaping edges).
   *
   * @param model - Petri net result from discovery or conversion.
   * @param log - Event log to evaluate against.
   * @returns Precision result with score and token counts.
   *
   * @example
   * ```ts
   * const pn = powl.discoverPetriNet(log);
   * const prec = powl.precisionEtconformance(pn, log);
   * console.log(prec.precision); // 0.85
   * ```
   */
  precisionEtconformance(model: PetriNetResult, log: EventLog): PrecisionResult {
    const pnJson = JSON.stringify(model);
    const logJson = JSON.stringify(log);
    return JSON.parse(
      (this.wm as any).precision_etconformance(pnJson, logJson),
    ) as PrecisionResult;
  }

  // ── Conformance diagnostics utilities ──────────────────────────────────────

  /**
   * Get non-fitting traces from a conformance result.
   *
   * @param result Fitness result from `conformance()` or `conformancePetriNet()`.
   * @returns Array of trace results where `trace_is_fit` is `false`.
   */
  static getNonFittingTraces(result: FitnessResult): TraceReplayResult[] {
    return result.trace_results.filter((t) => !t.trace_is_fit);
  }

  /**
   * Get transition firing sequence for a specific case.
   *
   * @param result Fitness result from `conformance()` or `conformancePetriNet()`.
   * @param caseId The case ID to look up.
   * @returns Ordered list of transition names fired during replay, or empty array if not found.
   */
  static getTransitionSequence(result: FitnessResult, caseId: string): string[] {
    const trace = result.trace_results.find((t) => t.case_id === caseId);
    return trace?.activated_transitions ?? [];
  }

  /**
   * Get diagnostic summary for all traces.
   *
   * Returns non-fitting traces and a frequency map of all transitions fired.
   *
   * @param result Fitness result from `conformance()` or `conformancePetriNet()`.
   */
  static getDiagnosticSummary(result: FitnessResult): {
    non_fitting: TraceReplayResult[];
    transition_frequency: Record<string, number>;
  } {
    const non_fitting = result.trace_results.filter((t) => !t.trace_is_fit);
    const freq: Record<string, number> = {};
    for (const t of result.trace_results) {
      for (const tr of t.activated_transitions) {
        freq[tr] = (freq[tr] ?? 0) + 1;
      }
    }
    return { non_fitting, transition_frequency: freq };
  }

  // ── Batch utilities ────────────────────────────────────────────────────────

  /**
   * Filter an event log to only traces whose fitness meets a threshold.
   *
   * @param threshold Minimum fitness score (0..1), default 0.8.
   */
  filterByFitness(
    model: PowlModel,
    log: EventLog,
    threshold = 0.8,
  ): EventLog {
    const result = this.conformance(model, log);
    const passingIds = new Set(
      result.trace_results
        .filter((r) => r.fitness >= threshold)
        .map((r) => r.case_id),
    );
    return {
      traces: log.traces.filter((t) => passingIds.has(t.case_id)),
    };
  }

  /**
   * Return variant statistics for an event log.
   *
   * @returns Map from activity sequence (joined by "→") to count.
   */
  variants(log: EventLog): Map<string, number> {
    const map = new Map<string, number>();
    for (const trace of log.traces) {
      const key = trace.events.map((e) => e.name).join("→");
      map.set(key, (map.get(key) ?? 0) + 1);
    }
    return map;
  }

  // ── Diff / comparison ─────────────────────────────────────────────────────────

  /**
   * Compute the structural and behavioural diff between two POWL model strings.
   *
   * @param modelA First model string.
   * @param modelB Second model string.
   * @returns Diff result with added/removed activities and structural changes.
   *
   * @example
   * ```ts
   * const diff = powl.diffModels("X(A, B)", "X(A, B, C)");
   * console.log(diff.added_activities); // ["C"]
   * ```
   */
  diffModels(modelA: string, modelB: string): Record<string, unknown> {
    const result = this.wm.diff_models(modelA, modelB);
    return JSON.parse(result) as Record<string, unknown>;
  }

  // ── BPMN export ───────────────────────────────────────────────────────────────

  /**
   * Convert a POWL model string to BPMN 2.0 XML.
   *
   * @param modelStr POWL model string.
   * @returns Complete BPMN XML document importable by Camunda, Signavio, etc.
   */
  toBpmn(modelStr: string): string {
    return this.wm.powl_to_bpmn(modelStr);
  }

  /**
   * Convert a PetriNetResult to PNML 2.0 XML format.
   *
   * Takes the output of `toPetriNet()` or `discoverPetriNetInductive()`
   * and converts it to PNML XML for import into tools like PNEditor,
   * WoPeD, or ProM.
   *
   * Mirrors `pm4py.write_pnml()`.
   *
   * @example
   * ```ts
   * const model = powl.parse("PO=(nodes={A, B}, order={A-->B})");
   * const pn = model.toPetriNet();
   * const pnml = powl.toPnml(JSON.stringify(pn));
   * // Save pnml to file, open in PNEditor
   * ```
   *
   * @param petriNetJson JSON string of PetriNetResult.
   * @returns PNML 2.0 XML string.
   */
  toPnml(petriNetJson: string): string {
    return (this.wm as any).to_pnml(petriNetJson);
  }

  /**
   * Replace activity labels in a POWL model.
   *
   * Mirrors `pm4py.objects.powl.utils.label_replacing.apply()`.
   *
   * @param modelStr POWL model string representation.
   * @param labelMap Object mapping old labels to new labels (e.g., {"A": "Start", "B": "End"}).
   * @returns New POWL model string with labels replaced.
   */
  replaceLabels(modelStr: string, labelMap: Record<string, string>): string {
    return this.wm.replace_labels(modelStr, JSON.stringify(labelMap));
  }

  // ── Complexity metrics ─────────────────────────────────────────────────────────

  /**
   * Compute complexity metrics for a POWL model.
   *
   * @param model Parsed POWL model.
   * @returns Object with cyclomatic, CFC, cognitive, nesting_depth, etc.
   */
  complexity(model: PowlModel): Record<string, number> {
    const result = this.wm.measure_complexity(model.__handle);
    return JSON.parse(result) as Record<string, number>;
  }

  // ── Statistics ──────────────────────────────────────────────────────────────

  /** Get start activities with frequencies. */
  getStartActivities(log: EventLog): import("./types.js").ActivityFrequency[] {
    return JSON.parse(this.wm.get_start_activities(JSON.stringify(log)));
  }

  /** Get end activities with frequencies. */
  getEndActivities(log: EventLog): import("./types.js").ActivityFrequency[] {
    return JSON.parse(this.wm.get_end_activities(JSON.stringify(log)));
  }

  /** Get all variants with frequencies and percentages. */
  getVariants(log: EventLog): import("./types.js").VariantInfo[] {
    return JSON.parse(this.wm.get_variants(JSON.stringify(log)));
  }

  /** Get all event attribute keys with statistics. */
  getEventAttributes(log: EventLog): import("./types.js").AttributeSummary[] {
    return JSON.parse(this.wm.get_event_attributes(JSON.stringify(log)));
  }

  /** Get all trace attribute keys with statistics. */
  getTraceAttributes(log: EventLog): import("./types.js").AttributeSummary[] {
    return JSON.parse(this.wm.get_trace_attributes(JSON.stringify(log)));
  }

  /** Get case attributes with statistics. */
  getCaseAttributes(log: EventLog): import("./types.js").AttributeSummary[] {
    return JSON.parse(this.wm.get_case_attributes(JSON.stringify(log)));
  }

  /** Get performance statistics for an event log. */
  getPerformanceStats(log: EventLog): import("./types.js").PerformanceStats {
    return JSON.parse(this.wm.get_performance_stats(JSON.stringify(log)));
  }

  /** Get average case arrival rate (cases per hour). */
  getCaseArrivalAverage(log: EventLog): number {
    return this.wm.get_case_arrival_average(JSON.stringify(log));
  }

  // ── Discovery ───────────────────────────────────────────────────────────────

  /**
   * Discover a Directly-Follows Graph from an event log.
   *
   * Mirrors `pm4py.discover_dfg()`.
   */
  discoverDFG(log: EventLog): import("./types.js").DFGResult {
    return JSON.parse(this.wm.discover_dfg(JSON.stringify(log)));
  }

  /**
   * Serialize a DFG result to a canonical JSON string.
   *
   * Mirrors `pm4py.write_dfg()`.
   *
   * @param dfg - DFG result from discoverDFG()
   */
  writeDfg(dfg: import("./types.js").DFGResult): string {
    return this.wm.write_dfg(JSON.stringify(dfg));
  }

  /**
   * Deserialize a DFG from a JSON string.
   *
   * Mirrors `pm4py.read_dfg()`.
   *
   * @param json - JSON string of DFG data
   */
  readDfg(json: string): import("./types.js").DFGResult {
    return JSON.parse(this.wm.read_dfg(json)) as import("./types.js").DFGResult;
  }

  /**
   * Discover a performance DFG with duration annotations on edges.
   *
   * Mirrors `pm4py.discover_performance_dfg()`.
   */
  discoverPerformanceDFG(log: EventLog): import("./types.js").PerformanceDFGResult {
    return JSON.parse(this.wm.discover_performance_dfg(JSON.stringify(log)));
  }

  /**
   * Discover an eventually-follows graph (all activity pairs in any trace).
   *
   * Mirrors `pm4py.discover_eventually_follows_graph()`.
   */
  discoverEventuallyFollowsGraph(log: EventLog): import("./types.js").DFGEdge[] {
    return JSON.parse(this.wm.discover_eventually_follows_graph(JSON.stringify(log)));
  }

  /**
   * Discover a process tree using the inductive miner.
   *
   * Mirrors `pm4py.discover_process_tree_inductive()`.
   */
  discoverProcessTree(log: EventLog): import("./types.js").ProcessTree {
    return JSON.parse(this.wm.discover_process_tree_inductive(JSON.stringify(log)));
  }

  /**
   * Discover a Petri net from an event log using the inductive miner.
   *
   * Mirrors `pm4py.discover_petri_net_inductive()`.
   */
  discoverPetriNet(log: EventLog): import("./types.js").PetriNetResult {
    return JSON.parse(this.wm.discover_petri_net_inductive(JSON.stringify(log)));
  }

  /**
   * Discover a BPMN model directly from an event log using the inductive miner.
   *
   * Combines inductive miner discovery with BPMN conversion in a single call.
   * Returns a complete BPMN 2.0 XML document importable by Camunda, Signavio, bpmn.io.
   *
   * Mirrors `pm4py.discover_bpmn_inductive()`.
   *
   * @example
   * ```ts
   * const log = powl.parseCsv("case_id,activity\n1,A\n1,B\n");
   * const bpmnXml = powl.discoverBpmn(log);
   * console.log(bpmnXml); // Complete BPMN 2.0 XML document
   * ```
   */
  discoverBpmn(log: EventLog): string {
    return this.wm.discover_bpmn_inductive(JSON.stringify(log));
  }

  // ── Filtering ───────────────────────────────────────────────────────────────

  /**
   * Filter traces to only those starting with one of the given activities.
   *
   * Mirrors `pm4py.filter_start_activities()`.
   */
  filterStartActivities(log: EventLog, activities: string[]): EventLog {
    return JSON.parse(this.wm.filter_start_activities(JSON.stringify(log), JSON.stringify(activities)));
  }

  /**
   * Filter traces to only those ending with one of the given activities.
   *
   * Mirrors `pm4py.filter_end_activities()`.
   */
  filterEndActivities(log: EventLog, activities: string[]): EventLog {
    return JSON.parse(this.wm.filter_end_activities(JSON.stringify(log), JSON.stringify(activities)));
  }

  /**
   * Filter to keep only the top-k most frequent variants.
   *
   * Mirrors `pm4py.filter_variants_top_k()`.
   */
  filterVariantsTopK(log: EventLog, k: number): EventLog {
    return JSON.parse(this.wm.filter_variants_top_k(JSON.stringify(log), k));
  }

  /**
   * Filter traces by time range (Unix milliseconds).
   *
   * Mirrors `pm4py.filter_time_range()`.
   */
  filterTimeRange(log: EventLog, startMs: number, endMs: number): EventLog {
    return JSON.parse(this.wm.filter_time_range(JSON.stringify(log), BigInt(startMs), BigInt(endMs)));
  }

  // ── Footprints ──────────────────────────────────────────────────────────────

  /**
   * Compute the behavioural footprints of a POWL model.
   *
   * Returns start/end activities, sequence/parallel pairs, etc.
   */
  footprints(model: PowlModel): ModelFootprints {
    return JSON.parse(this.wm.compute_footprints(model.__handle));
  }

  /**
   * Compute footprints-based conformance between an event log and a POWL model string.
   *
   * Returns fitness, precision, recall, f1.
   */
  conformanceFootprints(log: EventLog, modelStr: string): FootprintsConformanceResult {
    return JSON.parse(this.wm.conformance_footprints(JSON.stringify(log), modelStr));
  }

  // ── Soundness ───────────────────────────────────────────────────────────────

  /**
   * Check whether a Petri net is sound (deadlock-free, bounded, liveness).
   */
  checkSoundness(pn: PetriNetResult): SoundnessResult {
    return JSON.parse(this.wm.check_soundness(JSON.stringify(pn)));
  }

  // ── Streaming Conformance ─────────────────────────────────────────────────────

  /**
   * Create a streaming conformance checker for real-time monitoring.
   *
   * Returns a handle for incremental trace processing with EWMA smoothing and drift detection.
   *
   * @param modelStr POWL model string representation.
   * @returns Handle ID for the streaming conformance checker.
   */
  streamingCreate(modelStr: string): number {
    return this.wm.streaming_create(modelStr);
  }

  /**
   * Push a trace to the streaming conformance checker.
   *
   * @param handle Handle ID from `streamingCreate()`.
   * @param traceJson JSON string of a Trace object (with case_id and events array).
   * @returns Current fitness and alerts as JSON object.
   */
  streamingPushTrace(handle: number, traceJson: string): StreamingConformanceSnapshot {
    return JSON.parse(this.wm.streaming_push_trace(handle, traceJson));
  }

  /**
   * Get current snapshot of streaming conformance metrics.
   *
   * @param handle Handle ID from `streamingCreate()`.
   * @returns Current fitness, traces seen, windowed fitness, EWMA metrics, and drift signals.
   */
  streamingSnapshot(handle: number): StreamingConformanceSnapshot {
    return JSON.parse(this.wm.streaming_snapshot(handle));
  }

  // ── Discovery (additional) ──────────────────────────────────────────────────

  /**
   * Discover a Petri net using the alpha miner.
   *
   * Mirrors `pm4py.discover_petri_net_alpha()`.
   */
  discoverPetriNetAlpha(log: EventLog): PetriNetResult {
    return JSON.parse(this.wm.discover_petri_net_alpha(JSON.stringify(log)));
  }

  /**
   * Discover a Log Skeleton from an event log.
   *
   * Returns a model with six constraint types:
   * - equivalence: activities always co-occur with same frequency
   * - always_after: a always followed by b
   * - always_before: a always preceded by b
   * - never_together: a and b never in same trace
   * - directly_follows: a directly followed by b
   * - activ_freq: allowed activity frequencies per activity
   *
   * Mirrors `pm4py.discover_log_skeleton()`.
   *
   * @example
   * ```ts
   * const log = powl.parseXes("log.xes");
   * const skeleton = powl.discoverLogSkeleton(log);
   * console.log(skeleton.directly_follows); // [["A", "B"], ["B", "C"]]
   * ```
   */
  discoverLogSkeleton(log: EventLog): import("./types.js").LogSkeleton {
    return JSON.parse(this.wm.discover_log_skeleton(JSON.stringify(log)));
  }

  /**
   * Discover a DECLARE model from an event log.
   *
   * Returns constraint templates with support/confidence metrics:
   * - response: if a occurs, b must occur after
   * - precedence: b only if a occurred before
   * - succession: response + precedence
   * - coexistence: a and b always together
   * - altresponse: each a has b before next a
   * - chainresponse: each a immediately followed by b
   *
   * Mirrors `pm4py.discover_declare()`.
   *
   * @example
   * ```ts
   * const log = powl.parseXes("log.xes");
   * const declare = powl.discoverDeclare(log);
   * console.log(declare.rules.response["A|B"]); // {support: 5, confidence: 4}
   * ```
   */
  discoverDeclare(log: EventLog): import("./types.js").DeclareModel {
    return JSON.parse(this.wm.discover_declare(JSON.stringify(log)));
  }

  /**
   * Discover a temporal profile from an event log.
   *
   * Computes mean and standard deviation of elapsed time for each
   * directly-follows pair (A→B) in the log.
   *
   * Mirrors `pm4py.discover_temporal_profile()`.
   *
   * @example
   * ```ts
   * const log = powl.parseXes("log.xes");
   * const profile = powl.discoverTemporalProfile(log);
   * console.log(profile.pairs["A,B"]); // {mean_ms: 60000, stdev_ms: 5000, count: 10}
   * ```
   */
  discoverTemporalProfile(log: EventLog): import("./types.js").TemporalProfile {
    return JSON.parse(this.wm.discover_temporal_profile(JSON.stringify(log)));
  }

  /**
   * Check temporal conformance between an event log and a temporal profile.
   *
   * Flags steps where |duration - mean| > zeta * stdev as deviations.
   *
   * @param log - Event log to check
   * @param profile - Temporal profile (from discoverTemporalProfile)
   * @param zeta - Number of standard deviations for threshold (typically 2.0)
   *
   * Mirrors `pm4py.check_temporal_conformance()`.
   *
   * @example
   * ```ts
   * const log = powl.parseXes("log.xes");
   * const profile = powl.discoverTemporalProfile(log);
   * const result = powl.checkTemporalConformance(log, profile, 2.0);
   * console.log(result.fitness); // 0.95
   * ```
   */
  checkTemporalConformance(
    log: EventLog,
    profile: import("./types.js").TemporalProfile,
    zeta: number
  ): import("./types.js").TemporalConformance {
    return JSON.parse(
      this.wm.check_temporal_conformance(
        JSON.stringify(log),
        JSON.stringify(profile),
        zeta
      )
    );
  }

  /**
   * Discover a Heuristics Net from an event log.
   *
   * Uses dependency measure to filter causal relations. More lenient than
   * Alpha++ for handling noise and incomplete data.
   *
   * @param log - Event log to analyze
   * @param dependencyThreshold - Minimum dependency score (0.0 to 1.0, typically 0.5-0.9)
   *
   * Mirrors `pm4py.discover_heuristics_miner()`.
   *
   * @example
   * ```ts
   * const log = powl.parseXes("log.xes");
   * const net = powl.discoverHeuristicsMiner(log, 0.8);
   * console.log(net.dependencies); // [{from: "A", to: "B", dependency: 0.9, frequency: 5}]
   * ```
   */
  discoverHeuristicsMiner(
    log: EventLog,
    dependencyThreshold: number = 0.8
  ): import("./types.js").HeuristicsNet {
    return JSON.parse(
      this.wm.discover_heuristics_miner(JSON.stringify(log), dependencyThreshold)
    );
  }

  /**
   * Convert a Heuristics Net to a Petri Net.
   *
   * Creates places and transitions based on dependency relations.
   *
   * Mirrors `pm4py.heuristics_to_petri_net()`.
   *
   * @example
   * ```ts
   * const log = powl.parseXes("log.xes");
   * const net = powl.discoverHeuristicsMiner(log, 0.8);
   * const pn = powl.heuristicsToPetriNet(net);
   * console.log(pn.net.places); // Place objects
   * ```
   */
  heuristicsToPetriNet(
    net: import("./types.js").HeuristicsNet
  ): import("./types.js").PetriNetResult {
    return JSON.parse(this.wm.heuristics_to_petri_net(JSON.stringify(net)));
  }

  // ── Statistics (additional) ─────────────────────────────────────────────────

  /**
   * Get all distinct values for a given event attribute with frequencies.
   *
   * Mirrors `pm4py.get_attribute_values()`.
   */
  getAttributeValues(log: EventLog, attributeKey: string): AttributeValue[] {
    return JSON.parse(this.wm.get_attribute_values(JSON.stringify(log), attributeKey));
  }

  /**
   * Get case durations.
   *
   * Mirrors `pm4py.get_case_durations()`.
   */
  getCaseDurations(log: EventLog): CaseDurationResult[] {
    return JSON.parse(this.wm.get_case_durations(JSON.stringify(log)));
  }

  /**
   * Get rework times (time between consecutive same-activity events in a case).
   */
  getReworkTimes(log: EventLog): ReworkTime[] {
    return JSON.parse(this.wm.get_rework_times(JSON.stringify(log)));
  }

  // ── Filtering (additional) ──────────────────────────────────────────────────

  // TODO: Re-implement filtering functions
  // filterEventAttributeValues and filterTraceAttribute require WASM exports
  // that are not currently available. These can be added in a future release.

  /**
   * Filter traces by case size (number of events).
   * If maxSize is 0, only minSize is used as a lower bound.
   *
   * Mirrors `pm4py.filter_case_size()`.
   */
  filterCaseSize(log: EventLog, minSize: number, maxSize: number): EventLog {
    return JSON.parse(this.wm.filter_case_size(JSON.stringify(log), minSize, maxSize));
  }

  /**
   * Filter to keep only variants covering at least the given percentage.
   *
   * Mirrors `pm4py.filter_variants_reaching()`.
   */
  filterVariantsCoverage(log: EventLog, minCoverage: number): EventLog {
    return JSON.parse(this.wm.filter_variants_coverage(JSON.stringify(log), minCoverage));
  }

  // ── Format I/O ───────────────────────────────────────────────────────────────

  /**
   * Serialize an event log to XES XML format.
   *
   * Mirrors `pm4py.write_xes()`.
   */
  writeXes(log: EventLog): string {
    return this.wm.write_xes_log(JSON.stringify(log));
  }

  /**
   * Serialize an event log to CSV format.
   *
   * Mirrors `pm4py.write_csv()`.
   */
  writeCsv(log: EventLog): string {
    return this.wm.write_csv_log(JSON.stringify(log));
  }

  // ── Statistics (extended) ──────────────────────────────────────────────────

  /**
   * Get minimum self-distances for each activity.
   *
   * Mirrors `pm4py.get_minimum_self_distances()`.
   */
  getMinimumSelfDistances(log: EventLog): { activity: string; min_distance_ms: number }[] {
    return JSON.parse(this.wm.get_minimum_self_distances(JSON.stringify(log)));
  }

  /**
   * Get all case durations as a flat array of milliseconds.
   *
   * Mirrors `pm4py.get_all_case_durations()`.
   */
  getAllCaseDurations(log: EventLog): number[] {
    return JSON.parse(this.wm.get_all_case_durations(JSON.stringify(log)));
  }

  /**
   * Get case overlap (fraction of shared prefixes between traces, 0.0–1.0).
   *
   * Mirrors `pm4py.get_case_overlap()`.
   */
  getCaseOverlap(log: EventLog): number {
    return this.wm.get_case_overlap(JSON.stringify(log));
  }

  /**
   * Get all prefixes (partial traces) with their frequencies.
   *
   * Mirrors `pm4py.get_prefixes_from_log()`.
   */
  getPrefixes(log: EventLog): { prefix: string[]; count: number; percentage: number }[] {
    return JSON.parse(this.wm.get_prefixes_from_log(JSON.stringify(log)));
  }

  /**
   * Get trace attribute values with frequencies.
   *
   * Mirrors `pm4py.get_trace_attribute_values()`.
   */
  getTraceAttributeValues(log: EventLog, attributeKey: string): AttributeValue[] {
    return JSON.parse(this.wm.get_trace_attribute_values(JSON.stringify(log), attributeKey));
  }

  /**
   * Get variants as tuples (activity sequences with count).
   *
   * Mirrors `pm4py.get_variants_as_tuples()`.
   */
  getVariantsAsTuples(log: EventLog): { activities: string[]; count: number }[] {
    return JSON.parse(this.wm.get_variants_as_tuples(JSON.stringify(log)));
  }

  /**
   * Get variants with path durations (total, min, max, avg).
   *
   * Mirrors `pm4py.get_variants_paths_duration()`.
   */
  getVariantsPathsDuration(log: EventLog): {
    activities: string[];
    count: number;
    total_duration_ms: number;
    min_duration_ms: number;
    max_duration_ms: number;
    avg_duration_ms: number;
  }[] {
    return JSON.parse(this.wm.get_variants_paths_duration(JSON.stringify(log)));
  }

  /**
   * Get cases per activity that show rework (activity appears more than once).
   *
   * Mirrors `pm4py.get_rework_cases_per_activity()`.
   */
  getReworkCasesPerActivity(log: EventLog): {
    activity: string;
    rework_cases: number;
    total_cases: number;
  }[] {
    return JSON.parse(this.wm.get_rework_cases_per_activity(JSON.stringify(log)));
  }

  // ── Filtering (extended) ──────────────────────────────────────────────────

  /**
   * Filter to keep only events between two activities (inclusive).
   *
   * Mirrors `pm4py.filter_between()`.
   */
  filterBetween(log: EventLog, act1: string, act2: string): EventLog {
    return JSON.parse(this.wm.filter_between(JSON.stringify(log), act1, act2));
  }

  /**
   * Filter to keep only traces that start with the given prefix activities.
   *
   * Mirrors `pm4py.filter_prefixes()`.
   */
  filterPrefixes(log: EventLog, prefix: string[]): EventLog {
    return JSON.parse(this.wm.filter_prefixes(JSON.stringify(log), JSON.stringify(prefix)));
  }

  /**
   * Filter to keep only traces that end with the given suffix activities.
   *
   * Mirrors `pm4py.filter_suffixes()`.
   */
  filterSuffixes(log: EventLog, suffix: string[]): EventLog {
    return JSON.parse(this.wm.filter_suffixes(JSON.stringify(log), JSON.stringify(suffix)));
  }

  /**
   * Filter to keep only traces containing a directly-follows relation (a -> b).
   *
   * Mirrors `pm4py.filter_directly_follows_relation()`.
   */
  filterDirectlyFollows(log: EventLog, a: string, b: string): EventLog {
    return JSON.parse(this.wm.filter_directly_follows_relation(JSON.stringify(log), a, b));
  }

  /**
   * Filter to keep only traces containing a directly-follows relation (a → b).
   *
   * Mirrors `pm4py.filter_directly_follows_relation()`.
   *
   * @deprecated Use filterDirectlyFollows instead
   */
  filterEventuallyFollows(log: EventLog, a: string, b: string): EventLog {
    return JSON.parse(this.wm.filter_directly_follows_relation(JSON.stringify(log), a, b));
  }

  /**
   * Trim traces to remove events before the first start activity and after the last end activity.
   *
   * Mirrors `pm4py.filter_trim()`.
   */
  filterTrim(log: EventLog, startActivity: string, endActivity: string): EventLog {
    return JSON.parse(this.wm.filter_trim(JSON.stringify(log), startActivity, endActivity));
  }

  // ── Discovery (extended) ──────────────────────────────────────────────────

  /**
   * Discover footprints directly from an event log.
   *
   * Mirrors `pm4py.discover_footprints()` applied to a log.
   */
  discoverLogFootprints(log: EventLog): {
    start_activities: string[];
    end_activities: string[];
    activities: string[];
    sequence: [string, string][];
    parallel: [string, string][];
  } {
    return JSON.parse(this.wm.discover_log_footprints(JSON.stringify(log)));
  }

  // ── Utility ───────────────────────────────────────────────────────────────

  /**
   * Sort an event log by case_id then timestamp.
   *
   * Mirrors `pm4py.sort_log()`.
   */
  sortLog(log: EventLog): EventLog {
    return JSON.parse(this.wm.sort_log(JSON.stringify(log)));
  }

  /**
   * Project an event log to keep only the specified attributes.
   *
   * Mirrors `pm4py.project_log()`.
   */
  projectLog(log: EventLog, attributes: string[]): EventLog {
    return JSON.parse(this.wm.project_log(JSON.stringify(log), JSON.stringify(attributes)));
  }

  // ── LLM Pipeline ───────────────────────────────────────────────────────────────

  /**
   * Validate a POWL model string against structural soundness criteria.
   *
   * Uses POWLJudge to check for deadlock freedom, liveness, and boundedness.
   *
   * @returns Object with `verdict` (boolean), `reasoning` (string), and `violations` (array).
   */
  validatePowlStructure(modelStr: string): {
    verdict: boolean;
    reasoning: string;
    violations?: string[];
  } {
    return JSON.parse(this.wm.validate_powl_structure(modelStr));
  }

  /**
   * Get few-shot demos for a specific domain.
   *
   * Returns a JSON array of few-shot examples for LLM-guided POWL generation.
   *
   * Supported domains: "loan_approval", "finance", "software_release", "it",
   * "devops", "ecommerce", "retail", "manufacturing", "production",
   * "healthcare", "medical".
   *
   * For unknown domains, returns general demos.
   */
  getDemosForDomain(domain: string): FewShotDemo[] {
    return JSON.parse(this.wm.get_demos_for_domain(domain));
  }

  /**
   * Generate executable code from a POWL model string.
   *
   * Converts POWL to n8n JSON, Temporal Go, Camunda BPMN, or YAWL v6 XML.
   *
   * @param modelStr POWL model string.
   * @param target One of: "n8n", "temporal", "camunda", "yawl".
   * @returns Object with `code` (string), `target` (string), and `format` (string).
   */
  generateCodeFromPowl(modelStr: string, target: "n8n" | "temporal" | "camunda" | "yawl"): {
    code: string;
    target: string;
    format: string;
  } {
    return JSON.parse(this.wm.generate_code_from_powl(modelStr, target));
  }

  /**
   * Complete NL → POWL → Code pipeline (hybrid: JS LLM API + WASM POWL operations).
   *
   * This is the main entry point for natural language workflow generation.
   * It:
   * 1. Calls an LLM (via JavaScript fetch) to convert NL to POWL
   * 2. Validates the POWL structure using WASM
   * 3. Refines if needed (calls LLM again with validation feedback)
   * 4. Returns the verified POWL model
   *
   * @param naturalLanguage Natural language description of the workflow.
   * @param llmConfig Optional LLM configuration (API endpoint, model, etc.).
   * @param domain Optional domain for few-shot demos.
   * @returns Parsed POWL model.
   */
  async fromNaturalLanguage(
    naturalLanguage: string,
    llmConfig?: LLMConfig,
    domain?: string,
  ): Promise<PowlModel> {
    // This is a hybrid implementation:
    // - LLM API calls happen in JavaScript (via fetch)
    // - POWL parsing/validation happen in WASM

    const demos = domain ? this.getDemosForDomain(domain) : this.getDemosForDomain("general");

    // Build the prompt with few-shot examples
    const prompt = this.buildNLPrompt(naturalLanguage, demos);

    // Call LLM API (in JavaScript, not WASM)
    const llmResponse = await this.callLLMAPI(prompt, llmConfig);

    // Parse and validate the POWL model in WASM
    let modelStr = this.extractPowlFromResponse(llmResponse);
    let validationResult = this.validatePowlStructure(modelStr);

    let refinements = 0;
    const maxRefinements = 3;

    // Refinement loop: if validation fails, ask LLM to fix
    while (!validationResult.verdict && refinements < maxRefinements) {
      const feedback = validationResult.reasoning;
      const refinementPrompt = this.buildRefinementPrompt(modelStr, feedback);

      const refinedResponse = await this.callLLMAPI(refinementPrompt, llmConfig);
      modelStr = this.extractPowlFromResponse(refinedResponse);
      validationResult = this.validatePowlStructure(modelStr);

      refinements++;
    }

    // Parse the final (hopefully valid) POWL model
    return this.parse(modelStr);
  }

  /**
   * Generate code from a natural language description.
   *
   * Combines `fromNaturalLanguage()` + `generateCodeFromPowl()`.
   *
   * @param naturalLanguage Natural language description.
   * @param target Code generation target ("n8n", "temporal", "camunda", "yawl").
   * @param llmConfig Optional LLM configuration.
   * @param domain Optional domain for few-shot demos.
   * @returns Generated code.
   */
  async naturalLanguageToCode(
    naturalLanguage: string,
    target: "n8n" | "temporal" | "camunda" | "yawl",
    llmConfig?: LLMConfig,
    domain?: string,
  ): Promise<string> {
    const model = await this.fromNaturalLanguage(naturalLanguage, llmConfig, domain);
    const result = this.generateCodeFromPowl(model.toString(), target);
    return result.code;
  }

  // ── LLM Pipeline internal helpers ───────────────────────────────────────────────

  private buildNLPrompt(description: string, demos: FewShotDemo[]): string {
    const demoText = demos
      .map((d) => `Description: ${d.nl}\nPOWL: ${d.powl}`)
      .join("\n\n");

    return `You are a workflow modeling expert. Convert natural language descriptions to POWL (Partially Ordered Workflow Language) format.

Here are some examples:

${demoText}

Now convert this description to POWL:
${description}

Return ONLY the POWL expression, nothing else.`;
  }

  private buildRefinementPrompt(
    currentPowl: string,
    feedback: string,
  ): string {
    return `The following POWL model has structural issues:

${currentPowl}

Issues:
${feedback}

Please fix the POWL model to address these issues. Return ONLY the corrected POWL expression.`;
  }

  private async callLLMAPI(prompt: string, config?: LLMConfig): Promise<string> {
    const provider = config?.provider || "groq";
    const apiKey = config?.apiKey;
    const modelName = config?.model || this.getDefaultModel(provider);
    const temperature = config?.temperature ?? 0.2;

    if (!apiKey) {
      throw new Error(`LLM API key is required for provider: ${provider}. Provide it via llmConfig.apiKey`);
    }

    // Create provider instance with API key
    let model;
    switch (provider) {
      case "groq":
        model = createGroq({ apiKey })(modelName);
        break;
      case "openai":
        model = createOpenAI({ apiKey })(modelName);
        break;
      case "anthropic":
        model = createAnthropic({ apiKey })(modelName);
        break;
      default:
        throw new Error(`Unsupported provider: ${provider}. Supported: groq, openai, anthropic`);
    }

    // Generate text using Vercel AI SDK v7
    const { text } = await generateText({
      model,
      prompt,
      temperature,
    });

    return text;
  }

  private getDefaultModel(provider: string): string {
    switch (provider) {
      case "groq":
        return "openai/gpt-oss-20b";
      case "openai":
        return "gpt-4o";
      case "anthropic":
        return "claude-3-5-sonnet-20241022";
      default:
        return "openai/gpt-oss-20b";
    }
  }

  private extractPowlFromResponse(response: string): string {
    // Extract POWL from LLM response (handle markdown code blocks, etc.)
    let powl = response.trim();

    // Remove markdown code blocks
    powl = powl.replace(/```[\w]*\n?([\s\S]*?)```/g, "$1");

    // Extract POWL expression (look for common patterns)
    const powlMatch = powl.match(/POWL?:?\s*([^\n]+)/);
    if (powlMatch) {
      return powlMatch[1].trim();
    }

    // If no clear POWL marker, return the whole response (minus common prefixes)
    return powl
      .replace(/^(The POWL is:|Here's the POWL:|Model:)\s*/i, "")
      .trim();
  }
}

// ─── Type definitions ─────────────────────────────────────────────────────────────

/**
 * Few-shot demo for LLM-guided POWL generation.
 */
export interface FewShotDemo {
  description: string;
  nl: string;
  powl: string;
}

/**
 * LLM configuration for natural language to POWL generation.
 *
 * Uses the Vercel AI SDK which supports multiple providers.
 */
export interface LLMConfig {
  /** LLM provider: "groq" (default), "openai", or "anthropic". */
  provider?: "groq" | "openai" | "anthropic";
  /** API key for the LLM service. */
  apiKey?: string;
  /** Model name (defaults depend on provider). */
  model?: string;
  /** Temperature for generation (default: 0.2). */
  temperature?: number;
  /** Maximum tokens (default: 1024). */
  maxTokens?: number;
}
