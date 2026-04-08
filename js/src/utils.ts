/**
 * Developer-experience and quality-of-life utilities for working with POWL
 * models, event logs, and conformance results.
 */

import type {
  EventLog,
  FitnessResult,
  PetriNetResult,
  Trace,
  TraceReplayResult,
} from "./types.js";
import type { PowlModel, Powl } from "./index.js";

// ─── Model utilities ──────────────────────────────────────────────────────────

/**
 * Pretty-print a POWL model string with indentation.
 *
 * ```
 * PO=(
 *   nodes={ A, X(B, C), D },
 *   order={ A-->X(B,C), X(B,C)-->D }
 * )
 * ```
 */
export function prettyPrint(repr: string, indent = "  "): string {
  let depth = 0;
  let out = "";
  let i = 0;
  while (i < repr.length) {
    const ch = repr[i];
    if (ch === "{" || ch === "(") {
      out += ch + "\n" + indent.repeat(depth + 1);
      depth++;
    } else if (ch === "}" || ch === ")") {
      depth--;
      out += "\n" + indent.repeat(depth) + ch;
    } else if (ch === "," && repr[i + 1] === " ") {
      out += ch + "\n" + indent.repeat(depth);
      i++; // skip the trailing space
    } else {
      out += ch;
    }
    i++;
  }
  return out.trim();
}

/**
 * Collect all distinct activity labels from a model tree.
 * Excludes `tau` (silent transitions).
 */
export function modelActivities(model: PowlModel): string[] {
  return [...model.activities()].sort();
}

/**
 * Build an adjacency list `{ source: [targets] }` from a model's SPO edges.
 * Useful for custom visualisation.
 */
export function spoAdjacency(
  model: PowlModel,
  spoIdx: number,
): Record<number, number[]> {
  const edges = model.orderEdges(spoIdx);
  const adj: Record<number, number[]> = {};
  for (let i = 0; i < edges.length; i += 2) {
    const src = edges[i], tgt = edges[i + 1];
    (adj[src] ??= []).push(tgt);
  }
  return adj;
}

/**
 * Describe a model's structure as a compact summary string.
 *
 * @example `"SPO(4 nodes, 3 edges) with XOR(2), tau"`
 */
export function modelSummary(model: PowlModel): string {
  const counts: Record<string, number> = {};
  let edges = 0;

  model.walk((_idx, info) => {
    counts[info.type] = (counts[info.type] ?? 0) + 1;
    if (info.type === "StrictPartialOrder") {
      edges += info.edges.length;
    }
  });

  const parts: string[] = [];
  if (counts["StrictPartialOrder"]) {
    parts.push(`${counts["StrictPartialOrder"]} SPO (${edges} edges)`);
  }
  if (counts["OperatorPowl"]) parts.push(`${counts["OperatorPowl"]} operators`);
  if (counts["Transition"]) parts.push(`${counts["Transition"]} activities`);
  if (counts["FrequentTransition"]) {
    parts.push(`${counts["FrequentTransition"]} frequent`);
  }
  return parts.join(", ");
}

// ─── Event log utilities ──────────────────────────────────────────────────────

/**
 * Return the number of distinct activities in a log.
 */
export function logActivities(log: EventLog): string[] {
  const set = new Set<string>();
  for (const trace of log.traces) {
    for (const event of trace.events) set.add(event.name);
  }
  return [...set].sort();
}

/**
 * Compute basic statistics for an event log.
 */
export interface LogStats {
  traceCount: number;
  eventCount: number;
  activityCount: number;
  avgTraceLength: number;
  minTraceLength: number;
  maxTraceLength: number;
  variantCount: number;
}

export function logStats(log: EventLog): LogStats {
  const lengths = log.traces.map((t) => t.events.length);
  const total = lengths.reduce((a, b) => a + b, 0);
  const variants = new Set(
    log.traces.map((t) => t.events.map((e) => e.name).join("\x00")),
  );
  return {
    traceCount: log.traces.length,
    eventCount: total,
    activityCount: logActivities(log).length,
    avgTraceLength: log.traces.length ? total / log.traces.length : 0,
    minTraceLength: log.traces.length ? Math.min(...lengths) : 0,
    maxTraceLength: log.traces.length ? Math.max(...lengths) : 0,
    variantCount: variants.size,
  };
}

/**
 * Group traces by their activity sequence (variant).
 *
 * @returns Map from variant key → traces
 */
export function groupByVariant(log: EventLog): Map<string, Trace[]> {
  const map = new Map<string, Trace[]>();
  for (const trace of log.traces) {
    const key = trace.events.map((e) => e.name).join(" → ");
    (map.get(key) ?? map.set(key, []).get(key)!).push(trace);
  }
  return map;
}

/**
 * Return the top-N most frequent variants.
 */
export function topVariants(
  log: EventLog,
  n = 10,
): Array<{ variant: string; count: number; frequency: number }> {
  const groups = groupByVariant(log);
  return [...groups.entries()]
    .map(([variant, traces]) => ({
      variant,
      count: traces.length,
      frequency: traces.length / log.traces.length,
    }))
    .sort((a, b) => b.count - a.count)
    .slice(0, n);
}

/**
 * Filter log to traces containing a given activity.
 */
export function filterByActivity(log: EventLog, activity: string): EventLog {
  return {
    traces: log.traces.filter((t) => t.events.some((e) => e.name === activity)),
  };
}

/**
 * Filter log to traces between `start` and `end` timestamps (ISO-8601 strings).
 */
export function filterByTimeRange(
  log: EventLog,
  start: string,
  end: string,
): EventLog {
  const s = new Date(start).getTime();
  const e = new Date(end).getTime();
  return {
    traces: log.traces.filter((t) => {
      for (const ev of t.events) {
        if (!ev.timestamp) continue;
        const ts = new Date(ev.timestamp).getTime();
        if (ts >= s && ts <= e) return true;
      }
      return false;
    }),
  };
}

/**
 * Sample N random traces from a log (for large-log conformance previewing).
 */
export function sampleTraces(log: EventLog, n: number): EventLog {
  if (n >= log.traces.length) return log;
  const shuffled = [...log.traces].sort(() => Math.random() - 0.5);
  return { traces: shuffled.slice(0, n) };
}

/**
 * Slice a log to the first `k` events per trace.
 */
export function truncateTraces(log: EventLog, k: number): EventLog {
  return {
    traces: log.traces.map((t) => ({
      ...t,
      events: t.events.slice(0, k),
    })),
  };
}

// ─── Conformance utilities ────────────────────────────────────────────────────

/**
 * Partition traces into fitting / non-fitting at a given threshold.
 */
export interface PartitionedTraces {
  fitting: Trace[];
  nonFitting: Trace[];
}

export function partitionByFitness(
  log: EventLog,
  result: FitnessResult,
  threshold = 0.8,
): PartitionedTraces {
  const passing = new Set(
    result.trace_results
      .filter((r) => r.fitness >= threshold)
      .map((r) => r.case_id),
  );
  const fitting: Trace[] = [];
  const nonFitting: Trace[] = [];
  for (const t of log.traces) {
    (passing.has(t.case_id) ? fitting : nonFitting).push(t);
  }
  return { fitting, nonFitting };
}

/**
 * Return per-activity fitness breakdown.
 *
 * For each activity, compute the average fitness of traces that contain it.
 */
export function activityFitness(
  log: EventLog,
  result: FitnessResult,
): Map<string, number> {
  const fitMap = new Map(result.trace_results.map((r) => [r.case_id, r.fitness]));
  const actSum = new Map<string, number>();
  const actCount = new Map<string, number>();

  for (const trace of log.traces) {
    const fit = fitMap.get(trace.case_id) ?? 0;
    const seen = new Set<string>();
    for (const ev of trace.events) {
      if (!seen.has(ev.name)) {
        actSum.set(ev.name, (actSum.get(ev.name) ?? 0) + fit);
        actCount.set(ev.name, (actCount.get(ev.name) ?? 0) + 1);
        seen.add(ev.name);
      }
    }
  }

  const out = new Map<string, number>();
  for (const [act, sum] of actSum) {
    out.set(act, sum / actCount.get(act)!);
  }
  return out;
}

/**
 * Render a conformance result as a compact ASCII table.
 *
 * ```
 * ┌─────────┬────────────┬────────────┐
 * │ case_id │ activities │ fitness    │
 * ├─────────┼────────────┼────────────┤
 * │ case1   │ A → B → C │ 100.0%     │
 * │ case2   │ A → C     │  78.3%     │
 * └─────────┴────────────┴────────────┘
 * Global: 89.2% | 1/2 perfect
 * ```
 */
export function conformanceTable(log: EventLog, result: FitnessResult): string {
  const rows = result.trace_results.map((r) => {
    const trace = log.traces.find((t) => t.case_id === r.case_id);
    const activities = trace
      ? trace.events.map((e) => e.name).join(" → ")
      : "";
    return { id: r.case_id, activities, fitness: r.fitness };
  });

  const idW = Math.max(7, ...rows.map((r) => r.id.length));
  const actW = Math.max(10, ...rows.map((r) => Math.min(r.activities.length, 40)));
  const fitW = 8;

  const hr = `├${"─".repeat(idW + 2)}┼${"─".repeat(actW + 2)}┼${"─".repeat(fitW + 2)}┤`;
  const top = `┌${"─".repeat(idW + 2)}┬${"─".repeat(actW + 2)}┬${"─".repeat(fitW + 2)}┐`;
  const bot = `└${"─".repeat(idW + 2)}┴${"─".repeat(actW + 2)}┴${"─".repeat(fitW + 2)}┘`;

  const cell = (s: string, w: number) => s.slice(0, w).padEnd(w);
  const header = `│ ${cell("case_id", idW)} │ ${cell("activities", actW)} │ ${cell("fitness", fitW)} │`;

  const dataRows = rows.map((r) => {
    const pct = (r.fitness * 100).toFixed(1) + "%";
    return `│ ${cell(r.id, idW)} │ ${cell(r.activities, actW)} │ ${pct.padStart(fitW)} │`;
  });

  return [
    top,
    header,
    hr,
    ...dataRows.flatMap((r, i) => (i < dataRows.length - 1 ? [r] : [r])),
    bot,
    `Global: ${(result.percentage * 100).toFixed(1)}% | ${result.perfectly_fitting_traces}/${result.total_traces} perfect`,
  ].join("\n");
}

/**
 * Compute a fitness histogram with `buckets` equally-spaced bins [0..1].
 *
 * @returns Array of `{ range: "0.0-0.1", count, frequency }` objects.
 */
export function fitnessHistogram(
  result: FitnessResult,
  buckets = 10,
): Array<{ range: string; count: number; frequency: number }> {
  const bins = Array.from({ length: buckets }, () => 0);
  for (const r of result.trace_results) {
    const b = Math.min(Math.floor(r.fitness * buckets), buckets - 1);
    bins[b]++;
  }
  const total = result.trace_results.length || 1;
  return bins.map((count, i) => ({
    range: `${(i / buckets).toFixed(1)}-${((i + 1) / buckets).toFixed(1)}`,
    count,
    frequency: count / total,
  }));
}

// ─── Petri net utilities ──────────────────────────────────────────────────────

/**
 * Render a Petri net as a DOT (Graphviz) string for visualisation.
 *
 * ```dot
 * digraph {
 *   rankdir=LR;
 *   "p_start" [shape=circle];
 *   "t_A"     [shape=box, label="A"];
 *   "p_start" -> "t_A";
 * }
 * ```
 */
export function petriNetToDot(pn: PetriNetResult): string {
  const { net } = pn;
  const lines: string[] = ["digraph {", "  rankdir=LR;"];

  for (const p of net.places) {
    const tokens = pn.initial_marking[p.name] ?? 0;
    const label = tokens > 0 ? `${p.name} (${tokens})` : p.name;
    lines.push(`  "${p.name}" [shape=circle, label="${label}"];`);
  }

  for (const t of net.transitions) {
    const label = t.label ?? "τ";
    const style = t.label ? "" : ', style=filled, fillcolor="#555555", fontcolor=white';
    lines.push(`  "${t.name}" [shape=box, label="${label}"${style}];`);
  }

  for (const arc of net.arcs) {
    lines.push(`  "${arc.source}" -> "${arc.target}";`);
  }

  lines.push("}");
  return lines.join("\n");
}

/**
 * Return the set of activities visible in a Petri net
 * (transitions that have a non-null label).
 */
export function petriNetActivities(pn: PetriNetResult): string[] {
  return pn.net.transitions
    .filter((t) => t.label != null)
    .map((t) => t.label as string)
    .sort();
}

// ─── Batch / pipeline utilities ───────────────────────────────────────────────

/**
 * Run conformance checking in batches to avoid blocking the main thread.
 * Yields control via `setTimeout(0)` between each batch.
 *
 * @param powl     Powl instance
 * @param model    POWL model
 * @param log      Full event log
 * @param batchSize Number of traces per batch (default 50)
 * @param onProgress Called with `{ done, total }` after each batch
 */
export async function conformanceBatched(
  powl: Powl,
  model: PowlModel,
  log: EventLog,
  batchSize = 50,
  onProgress?: (p: { done: number; total: number }) => void,
): Promise<FitnessResult> {
  const total = log.traces.length;
  const allResults: TraceReplayResult[] = [];

  for (let i = 0; i < total; i += batchSize) {
    const batch: EventLog = { traces: log.traces.slice(i, i + batchSize) };
    const batchResult = powl.conformance(model, batch);
    allResults.push(...batchResult.trace_results);
    onProgress?.({ done: Math.min(i + batchSize, total), total });
    // yield to browser event loop
    await new Promise<void>((r) => setTimeout(r, 0));
  }

  const perfect = allResults.filter(
    (r) => r.missing_tokens === 0 && r.remaining_tokens === 0,
  ).length;
  const avgFit =
    allResults.reduce((s, r) => s + r.fitness, 0) / (allResults.length || 1);
  const totalProd = allResults.reduce((s, r) => s + r.produced_tokens, 0);
  const totalCons = allResults.reduce((s, r) => s + r.consumed_tokens, 0);
  const totalMiss = allResults.reduce((s, r) => s + r.missing_tokens, 0);
  const totalRem  = allResults.reduce((s, r) => s + r.remaining_tokens, 0);

  const percentage =
    totalProd === 0 && totalCons === 0
      ? 1.0
      : Math.min(
          1,
          Math.max(
            0,
            0.5 * (1 - totalMiss / totalCons) + 0.5 * (1 - totalRem / totalProd),
          ),
        );

  return {
    percentage,
    avg_trace_fitness: avgFit,
    perfectly_fitting_traces: perfect,
    total_traces: total,
    trace_results: allResults,
  };
}
