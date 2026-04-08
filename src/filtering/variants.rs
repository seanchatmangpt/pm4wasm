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

/// Filter event log by variants.
///
/// Mirrors `pm4py.filter_variants_top_k()` and `pm4py.filter_variants_reaching()`.

use crate::event_log::EventLog;

/// Filter to keep only the top-k most frequent variants.
pub fn filter_variants_top_k(log: &EventLog, k: usize) -> EventLog {
    let variant_counts = log.variants();
    let mut sorted: Vec<_> = variant_counts.into_iter().collect();
    sorted.sort_by(|a, b| b.1.cmp(&a.1));

    let top_k: std::collections::HashSet<Vec<String>> =
        sorted.into_iter().take(k).map(|(v, _)| v).collect();

    let traces: Vec<_> = log
        .traces
        .iter()
        .filter(|t| {
            let seq: Vec<String> = t.events.iter().map(|e| e.name.clone()).collect();
            top_k.contains(&seq)
        })
        .cloned()
        .collect();

    EventLog { traces }
}

/// Filter to keep only variants that cover at least the given percentage of traces.
pub fn filter_variants_coverage(log: &EventLog, min_coverage: f64) -> EventLog {
    let total = log.traces.len();
    if total == 0 {
        return EventLog { traces: vec![] };
    }

    let variant_counts = log.variants();
    let mut sorted: Vec<_> = variant_counts.into_iter().collect();
    sorted.sort_by(|a, b| b.1.cmp(&a.1));

    let mut cumulative = 0usize;
    let mut keep: std::collections::HashSet<Vec<String>> = std::collections::HashSet::new();
    for (variant, count) in &sorted {
        keep.insert(variant.clone());
        cumulative += count;
        if (cumulative as f64 / total as f64) >= min_coverage {
            break;
        }
    }

    let traces: Vec<_> = log
        .traces
        .iter()
        .filter(|t| {
            let seq: Vec<String> = t.events.iter().map(|e| e.name.clone()).collect();
            keep.contains(&seq)
        })
        .cloned()
        .collect();

    EventLog { traces }
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
    fn test_filter_variants_top_k() {
        let log = make_test_log();
        let filtered = filter_variants_top_k(&log, 2);
        assert!(filtered.traces.len() <= 2);
    }

    #[test]
    fn test_filter_variants_top_k_all() {
        let log = make_test_log();
        let filtered = filter_variants_top_k(&log, 100);
        assert_eq!(filtered.traces.len(), 3);
    }

    #[test]
    fn test_filter_variants_coverage() {
        let log = make_test_log();
        let filtered = filter_variants_coverage(&log, 0.5);
        assert!(filtered.traces.len() >= 2);
    }
}
