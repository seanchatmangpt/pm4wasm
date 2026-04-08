/**
 * Lightweight ML functions for process mining, powered by micro-ml.
 *
 * All functions are async (micro-ml uses WASM initialization).
 * No heavy dependencies — no sklearn, no ONNX, no TensorFlow.
 *
 * Usage:
 * ```ts
 * import { predictRemainingTime, clusterTraces, detectAnomalies } from "./predictive.js";
 *
 * const remaining = await predictRemainingTime(log);
 * const clusters = await clusterTraces(log, { k: 3 });
 * const anomalies = await detectAnomalies(log);
 * ```
 */

import type { EventLog } from "./types.js";

// micro-ml exports
import {
  linearRegressionSimple,
  trendForecast,
  linearRegression,
  kmeans,
  dbscan,
  decisionTree,
  ema,
  sma,
  findPeaks,
  findTroughs,
  rateOfChange,
  seasonalDecompose,
} from "micro-ml";

// ─── Types ──────────────────────────────────────────────────────────────────

/** Result of remaining time prediction for a single case. */
export interface RemainingTimePrediction {
  case_id: string;
  event_index: number;
  predicted_remaining_ms: number;
  actual_remaining_ms?: number;
}

/** Result of trend analysis on a numeric series. */
export interface TrendResult {
  direction: "up" | "down" | "flat";
  slope: number;
  strength: number;
  r_squared: number;
  forecast: number[];
}

/** Result of next-activity prediction for a single case. */
export interface NextActivityPrediction {
  case_id: string;
  event_index: number;
  predicted_activity: string;
  confidence: number;
}

/** Cluster assignment for a trace. */
export interface TraceCluster {
  case_id: string;
  cluster: number;
  is_noise: boolean;
}

/** Clustering result with metadata. */
export interface ClusteringResult {
  clusters: TraceCluster[];
  n_clusters: number;
  n_noise: number;
  centroids: number[][];
  inertia: number;
}

/** Anomaly detection result. */
export interface AnomalyResult {
  case_ids: string[];
  normal_ids: string[];
  n_anomalies: number;
  n_clusters: number;
}

/** Seasonality detection result. */
export interface SeasonalityResult {
  period: number;
  strength: number;
  trend: number[];
  seasonal: number[];
  residual: number[];
}

/** Feature vector for a single trace (used internally). */
export interface TraceFeatures {
  case_id: string;
  features: number[];
}

// ─── Feature Extraction ──────────────────────────────────────────────────────

/**
 * Encode each trace as a numeric feature vector for ML.
 *
 * Features per trace:
 * - Trace length (number of events)
 * - Number of distinct activities
 * - Duration (ms, 0 if no timestamps)
 * - Index of first event's activity in sorted activity list
 * - Index of last event's activity in sorted activity list
 *
 * @internal
 */
function extractTraceFeatures(
  log: EventLog,
  activityList: string[],
): TraceFeatures[] {
  const actIdx = new Map(activityList.map((a, i) => [a, i]));
  return log.traces.map((t) => {
    const duration = (() => {
      const first = t.events[0]?.timestamp;
      const last = t.events[t.events.length - 1]?.timestamp;
      if (first && last) {
        return new Date(last).getTime() - new Date(first).getTime();
      }
      return 0;
    })();

    return {
      case_id: t.case_id,
      features: [
        t.events.length,
        new Set(t.events.map((e) => e.name)).size,
        duration,
        actIdx.get(t.events[0]?.name ?? "") ?? 0,
        actIdx.get(t.events[t.events.length - 1]?.name ?? "") ?? 0,
      ],
    };
  });
}

// ─── Predictive Monitoring ────────────────────────────────────────────────────

/**
 * Predict remaining case time using linear regression on trace position.
 *
 * For each event in each trace, predicts the total remaining case duration
 * based on how far through the trace the event is (event_index / trace_length).
 *
 * Mirrors `pm4py.predict_case_remaining_time()` (lightweight version).
 *
 * @returns Predictions with case_id, event_index, predicted remaining ms, and actual remaining ms.
 */
export async function predictRemainingTime(
  log: EventLog,
): Promise<RemainingTimePrediction[]> {
  const results: RemainingTimePrediction[] = [];

  // Build training data: (fraction_complete, remaining_time)
  const xTrain: number[] = [];
  const yTrain: number[] = [];

  for (const trace of log.traces) {
    const firstTs = trace.events[0]?.timestamp;
    const lastTs = trace.events[trace.events.length - 1]?.timestamp;
    if (!firstTs || !lastTs) continue;
    const totalDuration = new Date(lastTs).getTime() - new Date(firstTs).getTime();
    if (totalDuration <= 0) continue;

    for (let i = 0; i < trace.events.length; i++) {
      const eventTs = trace.events[i].timestamp;
      if (!eventTs) continue;
      const elapsed = new Date(eventTs).getTime() - new Date(firstTs).getTime();
      const fraction = elapsed / totalDuration;
      const remaining = totalDuration - elapsed;
      xTrain.push(fraction);
      yTrain.push(remaining);
    }
  }

  if (xTrain.length < 2) return results;

  const model = await linearRegression(xTrain, yTrain);

  // Now predict for each trace
  for (const trace of log.traces) {
    const firstTs = trace.events[0]?.timestamp;
    const lastTs = trace.events[trace.events.length - 1]?.timestamp;
    if (!firstTs || !lastTs) continue;
    const totalDuration = new Date(lastTs).getTime() - new Date(firstTs).getTime();
    if (totalDuration <= 0) continue;

    for (let i = 0; i < trace.events.length; i++) {
      const eventTs = trace.events[i].timestamp;
      if (!eventTs) continue;
      const elapsed = new Date(eventTs).getTime() - new Date(firstTs).getTime();
      const fraction = elapsed / totalDuration;
      const predicted = model.predict([fraction])[0];
      const actual = totalDuration - elapsed;

      results.push({
        case_id: trace.case_id,
        event_index: i,
        predicted_remaining_ms: Math.max(0, predicted),
        actual_remaining_ms: actual,
      });
    }
  }

  return results;
}

// ─── Next Activity Prediction ──────────────────────────────────────────────────

/**
 * Predict the next activity using a decision tree classifier.
 *
 * Features: [trace_length_so_far, activity_index, position_fraction].
 * Target: index of the actual next activity.
 *
 * Mirrors `pm4py.predict_next_activity()` (lightweight version).
 *
 * @returns Predictions with predicted activity and confidence score.
 */
export async function predictNextActivity(
  log: EventLog,
): Promise<NextActivityPrediction[]> {
  const activities = [...new Set(log.traces.flatMap((t) => t.events.map((e) => e.name)))];
  activities.sort();
  if (activities.length < 2) return [];

  const actIdx = new Map(activities.map((a, i) => [a, i]));

  // Build training data
  const xTrain: number[][] = [];
  const yTrain: number[] = [];

  for (const trace of log.traces) {
    if (trace.events.length < 2) continue;
    for (let i = 0; i < trace.events.length - 1; i++) {
      const currentIdx = actIdx.get(trace.events[i].name) ?? 0;
      const nextIdx = actIdx.get(trace.events[i + 1].name) ?? 0;
      const fraction = i / (trace.events.length - 1);
      xTrain.push([i + 1, currentIdx, fraction]);
      yTrain.push(nextIdx);
    }
  }

  if (xTrain.length < 2) return [];

  const model = await decisionTree(xTrain, yTrain, { maxDepth: 8 });

  // Predict for the last event in each trace
  const results: NextActivityPrediction[] = [];
  for (const trace of log.traces) {
    if (trace.events.length < 2) continue;
    const lastEvent = trace.events[trace.events.length - 1];
    const lastIdx = actIdx.get(lastEvent.name) ?? 0;
    const fraction = (trace.events.length - 1) / trace.events.length;
    const prediction = model.predict([[trace.events.length, lastIdx, fraction]])[0];

    results.push({
      case_id: trace.case_id,
      event_index: trace.events.length - 1,
      predicted_activity: activities[prediction] ?? "unknown",
      confidence: 0.5, // decision tree doesn't expose probabilities directly
    });
  }

  return results;
}

// ─── Clustering ────────────────────────────────────────────────────────────────

/**
 * Cluster traces by their feature profiles using k-means.
 *
 * Mirrors `pm4py.cluster_log()` (lightweight version).
 *
 * @param log Event log to cluster.
 * @param options.k Number of clusters (default 3).
 * @returns Cluster assignments with centroids and inertia.
 */
export async function clusterTraces(
  log: EventLog,
  options: { k?: number } = {},
): Promise<ClusteringResult> {
  const activities = [...new Set(log.traces.flatMap((t) => t.events.map((e) => e.name)))];
  activities.sort();
  const features = extractTraceFeatures(log, activities);
  const k = options.k ?? Math.min(3, Math.max(1, log.traces.length));

  if (features.length < k) {
    return {
      clusters: features.map((f) => ({ case_id: f.case_id, cluster: 0, is_noise: false })),
      n_clusters: 1,
      n_noise: 0,
      centroids: [],
      inertia: 0,
    };
  }

  const dataMatrix: number[][] = features.map((f) => f.features);
  const model = await kmeans(dataMatrix, { k });

  const labels = model.getAssignments();
  const clusters: TraceCluster[] = features.map((f, i) => ({
    case_id: f.case_id,
    cluster: labels[i],
    is_noise: false,
  }));

  return {
    clusters,
    n_clusters: model.k,
    n_noise: 0,
    centroids: model.getCentroids(),
    inertia: model.inertia ?? 0,
  };
}

// ─── Anomaly Detection ────────────────────────────────────────────────────────

/**
 * Detect anomalous traces using DBSCAN density-based clustering.
 *
 * Traces not assigned to any cluster are flagged as anomalies.
 *
 * Mirrors a lightweight version of pm4py's outlier detection.
 *
 * @param log Event log to analyze.
 * @param options.eps DBSCAN neighborhood radius (default 0.5).
 * @param options.minPoints Minimum points for a cluster (default 2).
 * @returns Anomaly detection result with flagged case IDs.
 */
export async function detectAnomalies(
  log: EventLog,
  options: { eps?: number; minPoints?: number } = {},
): Promise<AnomalyResult> {
  const activities = [...new Set(log.traces.flatMap((t) => t.events.map((e) => e.name)))];
  activities.sort();
  const features = extractTraceFeatures(log, activities);

  if (features.length < 2) {
    return { case_ids: [], normal_ids: log.traces.map((t) => t.case_id), n_anomalies: 0, n_clusters: 0 };
  }

  // Normalize features for DBSCAN
  const nFeatures = features[0].features.length;
  const flatFeatures = features.flatMap((f) => f.features);

  // Min-max normalization per feature
  const normalized = new Array(flatFeatures.length);
  for (let j = 0; j < nFeatures; j++) {
    const col = features.map((f) => f.features[j]);
    const min = Math.min(...col);
    const max = Math.max(...col);
    const range = max - min || 1;
    for (let i = 0; i < col.length; i++) {
      normalized[i * nFeatures + j] = (col[i] - min) / range;
    }
  }

  const normalizedMatrix: number[][] = [];
  for (let i = 0; i < features.length; i++) {
    normalizedMatrix.push([]);
    for (let j = 0; j < nFeatures; j++) {
      normalizedMatrix[i].push(normalized[i * nFeatures + j]);
    }
  }

  const model = await dbscan(normalizedMatrix, {
    eps: options.eps ?? 0.5,
    minPoints: options.minPoints ?? 2,
  });

  const labels = model.getLabels();
  const anomalyIds: string[] = [];
  const normalIds: string[] = [];

  features.forEach((f, i) => {
    if (labels[i] === -1) {
      anomalyIds.push(f.case_id);
    } else {
      normalIds.push(f.case_id);
    }
  });

  return {
    case_ids: anomalyIds,
    normal_ids: normalIds,
    n_anomalies: anomalyIds.length,
    n_clusters: model.nClusters,
  };
}

// ─── Trend Analysis ───────────────────────────────────────────────────────────

/**
 * Analyze the trend of case durations over time.
 *
 * Uses linear regression on case durations sorted by arrival time.
 *
 * @returns Trend direction, slope, strength (r-squared), and forecast values.
 */
export async function analyzeDurationTrend(
  log: EventLog,
  forecastPeriods = 5,
): Promise<TrendResult> {
  // Extract case durations sorted by first event timestamp
  const caseDurations: { time: number; duration: number }[] = [];
  for (const trace of log.traces) {
    const firstTs = trace.events[0]?.timestamp;
    const lastTs = trace.events[trace.events.length - 1]?.timestamp;
    if (firstTs && lastTs) {
      caseDurations.push({
        time: new Date(firstTs).getTime(),
        duration: new Date(lastTs).getTime() - new Date(firstTs).getTime(),
      });
    }
  }

  if (caseDurations.length < 2) {
    return { direction: "flat", slope: 0, strength: 0, r_squared: 0, forecast: [] };
  }

  // Sort by time
  caseDurations.sort((a, b) => a.time - b.time);
  const y = caseDurations.map((c) => c.duration);

  const model = await linearRegressionSimple(y);
  const forecast = model.predict(
    Array.from({ length: forecastPeriods }, (_, i) => y.length + i),
  );

  return {
    direction: model.slope > 0 ? "up" : model.slope < 0 ? "down" : "flat",
    slope: model.slope,
    strength: model.rSquared,
    r_squared: model.rSquared,
    forecast,
  };
}

/**
 * Forecast future case arrival rate.
 *
 * Uses trend forecast on case arrival counts per time bucket.
 *
 * @param log Event log.
 * @param periods Number of future periods to forecast.
 * @returns Trend result with forecast values.
 */
export async function forecastCaseArrival(
  log: EventLog,
  periods = 5,
): Promise<TrendResult> {
  // Bucket cases by hour
  const hourBuckets = new Map<number, number>();
  for (const trace of log.traces) {
    const ts = trace.events[0]?.timestamp;
    if (ts) {
      const hour = Math.floor(new Date(ts).getTime() / 3_600_000);
      hourBuckets.set(hour, (hourBuckets.get(hour) ?? 0) + 1);
    }
  }

  const sortedHours = [...hourBuckets.keys()].sort((a, b) => a - b);
  const counts = sortedHours.map((h) => hourBuckets.get(h)!);

  if (counts.length < 2) {
    return { direction: "flat", slope: 0, strength: 0, r_squared: 0, forecast: [] };
  }

  const model = await linearRegressionSimple(counts);
  const forecastResult = await trendForecast(counts, periods);

  return {
    direction: forecastResult.direction,
    slope: model.slope,
    strength: model.rSquared,
    r_squared: model.rSquared,
    forecast: forecastResult.getForecast(),
  };
}

// ─── Smoothing ───────────────────────────────────────────────────────────────

/**
 * Smooth a numeric series using exponential moving average.
 *
 * Useful for smoothing fitness trends, arrival rates, or other time-series metrics.
 *
 * @param data Numeric series to smooth.
 * @param window Smoothing window size (default 5).
 * @returns Smoothed series.
 */
export async function smoothSeries(
  data: number[],
  window = 5,
  method: "ema" | "sma" = "ema",
): Promise<number[]> {
  if (method === "ema") {
    return ema(data, window);
  }
  return sma(data, window);
}

// ─── Peak/Anomaly Detection in Metrics ────────────────────────────────────────

/**
 * Find peaks (local maxima) in a numeric series.
 *
 * Useful for detecting sudden spikes in process metrics (e.g., case duration outliers).
 *
 * @param data Numeric series.
 * @returns Indices of peak values.
 */
export async function findMetricPeaks(data: number[]): Promise<number[]> {
  return findPeaks(data);
}

/**
 * Find troughs (local minima) in a numeric series.
 *
 * @param data Numeric series.
 * @returns Indices of trough values.
 */
export async function findMetricTroughs(data: number[]): Promise<number[]> {
  return findTroughs(data);
}

// ─── Seasonality Detection ────────────────────────────────────────────────────

/**
 * Detect seasonality in case arrival rates.
 *
 * @param log Event log.
 * @returns Period and strength of detected seasonal pattern.
 */
export async function detectArrivalSeasonality(
  log: EventLog,
): Promise<SeasonalityResult> {
  // Extract case counts per hour
  const hourBuckets = new Map<number, number>();
  for (const trace of log.traces) {
    const ts = trace.events[0]?.timestamp;
    if (ts) {
      const hour = Math.floor(new Date(ts).getTime() / 3_600_000);
      hourBuckets.set(hour, (hourBuckets.get(hour) ?? 0) + 1);
    }
  }

  const sortedHours = [...hourBuckets.keys()].sort((a, b) => a - b);
  const counts = sortedHours.map((h) => hourBuckets.get(h)!);

  if (counts.length < 4) {
    return { period: 0, strength: 0, trend: counts, seasonal: counts, residual: counts };
  }

  const period = Math.max(2, Math.floor(counts.length / 4));
  const decomposition = await seasonalDecompose(counts, period);

  return {
    period: decomposition.period,
    strength: 0, // micro-ml doesn't expose strength directly
    trend: decomposition.getTrend(),
    seasonal: decomposition.getSeasonal(),
    residual: decomposition.getResidual(),
  };
}

// ─── Rate of Change ──────────────────────────────────────────────────────────

/**
 * Compute rate of change (percentage change) for a numeric series.
 *
 * Useful for monitoring drift in fitness scores, case durations, etc.
 *
 * @param data Numeric series.
 * @param periods Number of periods to look back (default 1).
 * @returns Rate of change values.
 */
export async function computeRateOfChange(
  data: number[],
  periods = 1,
): Promise<number[]> {
  return rateOfChange(data, periods);
}
