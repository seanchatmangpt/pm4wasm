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

/// Filter event log by event/trace attributes.
///
/// Mirrors `pm4py.filter_event_attribute_values()` and related.

use crate::event_log::EventLog;

/// Filter to keep only traces that contain at least one event with the given
/// attribute key-value pair.
///
/// Mirrors `pm4py.filter_event_attribute_values()`.
pub fn filter_event_attribute_values(
    log: &EventLog,
    attribute_key: &str,
    attribute_values: &[String],
    positive: bool,
) -> EventLog {
    let value_set: std::collections::HashSet<&str> =
        attribute_values.iter().map(|s| s.as_str()).collect();

    let traces: Vec<_> = log
        .traces
        .iter()
        .filter(|t| {
            let matches = t.events.iter().any(|e| {
                let val = match attribute_key {
                    "concept:name" => Some(e.name.as_str()),
                    _ => e.attributes.get(attribute_key).map(|s| s.as_str()),
                };
                val.map(|v| value_set.contains(v)).unwrap_or(false)
            });
            if positive { matches } else { !matches }
        })
        .cloned()
        .collect();

    EventLog { traces }
}

/// Filter to keep only traces where the trace-level attribute matches.
///
/// Mirrors `pm4py.filter_trace_attribute()`.
pub fn filter_trace_attribute(
    log: &EventLog,
    attribute_key: &str,
    attribute_values: &[String],
    positive: bool,
) -> EventLog {
    let value_set: std::collections::HashSet<&str> =
        attribute_values.iter().map(|s| s.as_str()).collect();

    let traces: Vec<_> = log
        .traces
        .iter()
        .filter(|t| {
            let val = match attribute_key {
                "concept:name" => Some(t.case_id.as_str()),
                _ => None, // no generic trace attributes beyond case_id
            };
            let matches = val.map(|v| value_set.contains(v)).unwrap_or(false);
            if positive { matches } else { !matches }
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
             2,C,2020-01-01T11:10:00\n",
        )
        .unwrap()
    }

    #[test]
    fn test_filter_event_attribute_values() {
        let log = make_test_log();
        let filtered = filter_event_attribute_values(
            &log,
            "concept:name",
            &["B".to_string()],
            true,
        );
        assert_eq!(filtered.traces.len(), 1);
        assert_eq!(filtered.traces[0].case_id, "1");
    }

    #[test]
    fn test_filter_event_attribute_values_negative() {
        let log = make_test_log();
        let filtered = filter_event_attribute_values(
            &log,
            "concept:name",
            &["C".to_string()],
            false,
        );
        // All traces that do NOT contain C → only case 1
        assert_eq!(filtered.traces.len(), 1);
        assert_eq!(filtered.traces[0].case_id, "1");
    }

    #[test]
    fn test_filter_trace_attribute() {
        let log = make_test_log();
        let filtered = filter_trace_attribute(
            &log,
            "concept:name",
            &["1".to_string()],
            true,
        );
        assert_eq!(filtered.traces.len(), 1);
    }
}
