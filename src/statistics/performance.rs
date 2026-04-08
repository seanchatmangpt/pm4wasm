// PM4Py – A Process Mining Library for Python (POWL v2 WASM)
// Copyright (C) 2024 Process Intelligence Solutions
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use crate::event_log::EventLog;
use crate::statistics::basic::CaseDuration;
use serde::{Deserialize, Serialize};

/// Performance statistics summary.
///
/// Mirrors `pm4py` performance statistics (case duration, cycle time, etc.).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PerformanceStats {
    pub total_cases: usize,
    pub total_events: usize,
    pub avg_case_duration_ms: f64,
    pub min_case_duration_ms: f64,
    pub max_case_duration_ms: f64,
    pub median_case_duration_ms: f64,
    pub total_events_longest_case: usize,
    pub avg_events_per_case: f64,
}

/// Compute performance statistics for an event log.
pub fn get_performance_stats(log: &EventLog) -> PerformanceStats {
    let total_cases = log.traces.len();
    let total_events: usize = log.traces.iter().map(|t| t.events.len()).sum();

    let durations: Vec<CaseDuration> = crate::statistics::basic::get_case_durations(log);

    let avg_duration = if !durations.is_empty() {
        durations.iter().map(|d| d.duration_ms as f64).sum::<f64>() / durations.len() as f64
    } else {
        0.0
    };

    let min_duration = durations
        .first()
        .map(|d| d.duration_ms as f64)
        .unwrap_or(0.0);

    let max_duration = durations
        .last()
        .map(|d| d.duration_ms as f64)
        .unwrap_or(0.0);

    let median_duration = if !durations.is_empty() {
        let mid = durations.len() / 2;
        if durations.len() % 2 == 0 && mid > 0 {
            (durations[mid - 1].duration_ms + durations[mid].duration_ms) as f64 / 2.0
        } else {
            durations[mid].duration_ms as f64
        }
    } else {
        0.0
    };

    let events_longest = log
        .traces
        .iter()
        .map(|t| t.events.len())
        .max()
        .unwrap_or(0);

    let avg_events = if total_cases > 0 {
        total_events as f64 / total_cases as f64
    } else {
        0.0
    };

    PerformanceStats {
        total_cases,
        total_events,
        avg_case_duration_ms: avg_duration,
        min_case_duration_ms: min_duration,
        max_case_duration_ms: max_duration,
        median_case_duration_ms: median_duration,
        total_events_longest_case: events_longest,
        avg_events_per_case: avg_events,
    }
}

/// Get the average case arrival rate (cases per hour).
pub fn get_case_arrival_average(log: &EventLog) -> f64 {
    let durations = crate::statistics::basic::get_case_durations(log);
    if durations.len() < 2 {
        return 0.0;
    }

    let first_start = durations.first().unwrap().duration_ms;
    let last_end = durations.last().unwrap().duration_ms;

    if last_end <= first_start {
        return 0.0;
    }

    let span_ms = last_end - first_start;
    if span_ms <= 0 {
        return 0.0;
    }

    let span_hours = span_ms as f64 / (1000.0 * 60.0 * 60.0);
    if span_hours <= 0.0 {
        return 0.0;
    }

    log.traces.len() as f64 / span_hours
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::event_log::parse_csv;

    fn make_test_log() -> EventLog {
        parse_csv(
            "case_id,activity,timestamp\n\
             1,A,2020-01-01T10:00:00\n\
             1,B,2020-01-01T10:05:00\n\
             2,A,2020-01-01T11:00:00\n\
             2,B,2020-01-01T11:03:00\n\
             2,C,2020-01-01T11:10:00\n\
             3,A,2020-01-02T09:00:00\n\
             3,C,2020-01-02T09:30:00\n",
        )
        .unwrap()
    }

    #[test]
    fn test_performance_stats() {
        let log = make_test_log();
        let stats = get_performance_stats(&log);
        assert_eq!(stats.total_cases, 3);
        assert_eq!(stats.total_events, 7);
        assert!(stats.avg_case_duration_ms > 0.0);
        assert!(stats.min_case_duration_ms > 0.0);
        assert!(stats.max_case_duration_ms >= stats.min_case_duration_ms);
    }

    #[test]
    fn test_avg_events_per_case() {
        let log = make_test_log();
        let stats = get_performance_stats(&log);
        // 7 events / 3 cases = 2.33...
        assert!((stats.avg_events_per_case - 2.333).abs() < 0.01);
    }
}
