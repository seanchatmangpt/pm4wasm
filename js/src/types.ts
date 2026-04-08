// ─── Shared domain types ──────────────────────────────────────────────────────

/** A single event inside a trace. */
export interface LogEvent {
  name: string;
  timestamp?: string;
  lifecycle?: string;
  attributes: Record<string, string>;
}

/** An ordered sequence of events for one case. */
export interface Trace {
  case_id: string;
  events: LogEvent[];
}

/** A parsed XES or CSV event log. */
export interface EventLog {
  traces: Trace[];
}

// ─── Petri net ────────────────────────────────────────────────────────────────

export interface PetriPlace {
  name: string;
}

export interface PetriTransition {
  name: string;
  label?: string | null;
  properties: Record<string, unknown>;
}

export interface PetriArc {
  source: string;
  target: string;
  weight: number;
}

export interface PetriNet {
  name: string;
  places: PetriPlace[];
  transitions: PetriTransition[];
  arcs: PetriArc[];
}

export type Marking = Record<string, number>;

export interface PetriNetResult {
  net: PetriNet;
  initial_marking: Marking;
  final_marking: Marking;
}

// ─── Process tree ─────────────────────────────────────────────────────────────

export type PtOperator = "Sequence" | "Xor" | "Parallel" | "Loop";

export interface ProcessTree {
  label?: string | null;
  operator?: PtOperator | null;
  children: ProcessTree[];
}

// ─── Footprints ───────────────────────────────────────────────────────────────

export interface Footprints {
  start_activities: string[];
  end_activities: string[];
  activities: string[];
  activities_always_happening: string[];
  skippable_activities: string[];
  /** [a, b] pairs where a directly precedes b. */
  sequence: [string, string][];
  /** [a, b] pairs where a and b are concurrent. */
  parallel: [string, string][];
  min_trace_length: number;
}

// ─── Conformance ──────────────────────────────────────────────────────────────

export interface TraceReplayResult {
  case_id: string;
  fitness: number;
  trace_is_fit: boolean;
  produced_tokens: number;
  consumed_tokens: number;
  missing_tokens: number;
  remaining_tokens: number;
  activated_transitions: string[];
  reached_marking: Record<string, number>;
}

export interface FitnessResult {
  /** Global token-weighted fitness in [0, 1]. */
  percentage: number;
  /** Average per-trace fitness. */
  avg_trace_fitness: number;
  perfectly_fitting_traces: number;
  total_traces: number;
  trace_results: TraceReplayResult[];
}

export interface PrecisionResult {
  /** Overall precision score in [0.0, 1.0]. */
  precision: number;
  /** Total escaping tokens across all traces. */
  total_escaping: number;
  /** Total consumed tokens across all traces. */
  total_consumed: number;
  /** Number of traces analyzed. */
  total_traces: number;
}

// ─── Node info ────────────────────────────────────────────────────────────────

export type NodeType =
  | "Transition"
  | "FrequentTransition"
  | "StrictPartialOrder"
  | "OperatorPowl"
  | "Invalid";

export type NodeInfo =
  | { type: "Transition"; label: string; id: number }
  | {
      type: "FrequentTransition";
      label: string;
      activity: string;
      skippable: boolean;
      selfloop: boolean;
    }
  | { type: "StrictPartialOrder"; children: number[]; edges: [number, number][] }
  | { type: "OperatorPowl"; operator: string; children: number[] }
  | { type: "Invalid" };

// ─── Statistics ───────────────────────────────────────────────────────────────

export interface ActivityFrequency {
  activity: string;
  count: number;
}

export interface VariantInfo {
  activities: string[];
  count: number;
  percentage: number;
}

export interface AttributeSummary {
  name: string;
  count: number;
  unique_values: number;
}

export interface PerformanceStats {
  total_cases: number;
  total_events: number;
  avg_case_duration_ms: number;
  min_case_duration_ms: number;
  max_case_duration_ms: number;
  median_case_duration_ms: number;
  total_events_longest_case: number;
  avg_events_per_case: number;
}

// ─── DFG / Discovery ──────────────────────────────────────────────────────────

export interface DFGEdge {
  source: string;
  target: string;
  count: number;
}

export interface DFGResult {
  edges: DFGEdge[];
  start_activities: [string, number][];
  end_activities: [string, number][];
  activities: [string, number][];
}

export interface PerformanceDFGEdge extends DFGEdge {
  avg_duration_ms: number;
  min_duration_ms: number;
  max_duration_ms: number;
}

export interface PerformanceDFGResult {
  edges: PerformanceDFGEdge[];
  start_activities: [string, number][];
  end_activities: [string, number][];
  activities: [string, number][];
}

// ─── Footprints ────────────────────────────────────────────────────────────────

export interface ModelFootprints {
  start_activities: string[];
  end_activities: string[];
  activities: string[];
  skippable: boolean;
  sequence: [string, string][];
  parallel: [string, string][];
  activities_always_happening: string[];
  min_trace_length: number;
}

// ─── Conformance ───────────────────────────────────────────────────────────────

export interface FootprintsConformanceResult {
  fitness: number;
  precision: number;
  recall: number;
  f1: number;
}

// ─── Soundness ────────────────────────────────────────────────────────────────

export interface SoundnessResult {
  sound: boolean;
  deadlock_free: boolean;
  bounded: boolean;
  liveness: boolean;
}

// ─── Streaming Conformance ─────────────────────────────────────────────────

export interface StreamingConformanceSnapshot {
  fitness: number;
  traces_seen: number;
  perfect_traces: number;
  windowed_fitness: number;
  ewma_fitness: number;
  drift_signals: DriftSignal[];
}

export interface DriftSignal {
  rule: string;
  metric: string;
  current_value: number;
  limits: {
    center: number;
    upper: number;
    lower: number;
  };
  severity: "warning" | "critical" | "info";
}

// ─── Attribute Values ─────────────────────────────────────────────────────────

export interface AttributeValue {
  attribute: string;
  value: string;
  count: number;
}

// ─── Case Duration ─────────────────────────────────────────────────────────────

export interface CaseDurationResult {
  case_id: string;
  duration_ms: number;
}

// ─── Log Skeleton ───────────────────────────────────────────────────────────────

export interface LogSkeleton {
  equivalence: [string, string][];
  always_after: [string, string][];
  always_before: [string, string][];
  never_together: [string, string][];
  directly_follows: [string, string][];
  activ_freq: Record<string, number[]>;
}

// ─── DECLARE ─────────────────────────────────────────────────────────────────────

export interface DeclareModel {
  rules: Record<string, Record<string, DeclareRule>>;
}

export interface DeclareRule {
  support: number;
  confidence: number;
}

// ─── Rework Time ──────────────────────────────────────────────────────────────

export interface ReworkTime {
  case_id: string;
  activity: string;
  duration_ms: number;
}

// ─── Temporal Profile ───────────────────────────────────────────────────────────

export interface TemporalPair {
  mean_ms: number;
  stdev_ms: number;
  count: number;
}

export interface TemporalProfile {
  pairs: Record<string, TemporalPair>;
}

export interface TemporalDeviation {
  case_id: string;
  from: string;
  to: string;
  duration_ms: number;
  mean_ms: number;
  stdev_ms: number;
  zeta: number;
  deviation: boolean;
}

export interface TemporalConformance {
  total_traces: number;
  total_steps: number;
  deviations: number;
  fitness: number;
  details: TemporalDeviation[];
}

// ─── Heuristics Miner ────────────────────────────────────────────────────────────

export interface Dependency {
  from: string;
  to: string;
  dependency: number;
  frequency: number;
}

export interface HeuristicsNet {
  activities: string[];
  dependencies: Dependency[];
  start_activities: Record<string, number>;
  end_activities: Record<string, number>;
}
