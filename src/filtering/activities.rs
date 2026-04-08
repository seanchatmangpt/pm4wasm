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

/// Filter event log by start/end activities, prefixes, suffixes, and paths.
///
/// Mirrors `pm4py.filter_start_activities()`, `pm4py.filter_end_activities()`,
/// `pm4py.filter_between()`, `pm4py.filter_prefixes()`, `pm4py.filter_suffixes()`,
/// `pm4py.filter_directly_follows_relation()`, `pm4py.filter_eventually_follows_relation()`,
/// `pm4py.filter_trim()`.

use crate::event_log::EventLog;

/// Filter traces to only those that start with one of the given activities.
pub fn filter_start_activities(log: &EventLog, activities: &[String]) -> EventLog {
    let activity_set: std::collections::HashSet<&str> =
        activities.iter().map(|a| a.as_str()).collect();
    let traces: Vec<_> = log
        .traces
        .iter()
        .filter(|t| {
            t.events
                .first()
                .map(|e| activity_set.contains(e.name.as_str()))
                .unwrap_or(false)
        })
        .cloned()
        .collect();
    EventLog { traces }
}

/// Filter traces to only those that end with one of the given activities.
pub fn filter_end_activities(log: &EventLog, activities: &[String]) -> EventLog {
    let activity_set: std::collections::HashSet<&str> =
        activities.iter().map(|a| a.as_str()).collect();
    let traces: Vec<_> = log
        .traces
        .iter()
        .filter(|t| {
            t.events
                .last()
                .map(|e| activity_set.contains(e.name.as_str()))
                .unwrap_or(false)
        })
        .cloned()
        .collect();
    EventLog { traces }
}

/// Filter to keep only events between two activities (inclusive).
///
/// For each trace, keeps events from the first occurrence of `act1` to the
/// first occurrence of `act2`. Traces missing either activity are removed.
///
/// Mirrors `pm4py.filter_between()`.
pub fn filter_between(log: &EventLog, act1: &str, act2: &str) -> EventLog {
    let traces: Vec<_> = log
        .traces
        .iter()
        .filter_map(|t| {
            let start_idx = t.events.iter().position(|e| e.name == act1)?;
            let end_idx = t.events.iter().rposition(|e| e.name == act2)?;
            if start_idx > end_idx {
                return None;
            }
            Some(crate::event_log::Trace {
                case_id: t.case_id.clone(),
                events: t.events[start_idx..=end_idx].to_vec(),
            })
        })
        .collect();
    EventLog { traces }
}

/// Filter to keep only traces that start with the given prefix activities.
///
/// Mirrors `pm4py.filter_prefixes()`.
pub fn filter_prefixes(log: &EventLog, prefix: &[String]) -> EventLog {
    if prefix.is_empty() {
        return log.clone();
    }
    let traces: Vec<_> = log
        .traces
        .iter()
        .filter(|t| {
            t.events.len() >= prefix.len()
                && t.events[..prefix.len()]
                    .iter()
                    .zip(prefix.iter())
                    .all(|(e, a)| e.name == *a)
        })
        .cloned()
        .collect();
    EventLog { traces }
}

/// Filter to keep only traces that end with the given suffix activities.
///
/// Mirrors `pm4py.filter_suffixes()`.
pub fn filter_suffixes(log: &EventLog, suffix: &[String]) -> EventLog {
    if suffix.is_empty() {
        return log.clone();
    }
    let traces: Vec<_> = log
        .traces
        .iter()
        .filter(|t| {
            let slen = suffix.len();
            t.events.len() >= slen
                && t.events[t.events.len() - slen..]
                    .iter()
                    .zip(suffix.iter())
                    .all(|(e, a)| e.name == *a)
        })
        .cloned()
        .collect();
    EventLog { traces }
}

/// Filter to keep only traces containing a directly-follows relation (a -> b).
///
/// Mirrors `pm4py.filter_directly_follows_relation()`.
pub fn filter_directly_follows_relation(log: &EventLog, a: &str, b: &str) -> EventLog {
    let traces: Vec<_> = log
        .traces
        .iter()
        .filter(|t| {
            t.events.windows(2).any(|w| w[0].name == a && w[1].name == b)
        })
        .cloned()
        .collect();
    EventLog { traces }
}

/// Filter to keep only traces containing an eventually-follows relation (a ... b).
///
/// Mirrors `pm4py.filter_eventually_follows_relation()`.
pub fn filter_eventually_follows_relation(log: &EventLog, a: &str, b: &str) -> EventLog {
    let traces: Vec<_> = log
        .traces
        .iter()
        .filter(|t| {
            let pos_a = t.events.iter().position(|e| e.name == a);
            pos_a.map(|pa| t.events[pa..].iter().any(|e| e.name == b)).unwrap_or(false)
        })
        .cloned()
        .collect();
    EventLog { traces }
}

/// Trim traces to remove events before the first start activity and after
/// the last end activity occurrence.
///
/// Mirrors `pm4py.filter_trim()`.
pub fn filter_trim(log: &EventLog, start_activity: &str, end_activity: &str) -> EventLog {
    let traces: Vec<_> = log
        .traces
        .iter()
        .filter_map(|t| {
            let start_idx = t.events.iter().position(|e| e.name == start_activity)?;
            let end_idx = t.events.iter().rposition(|e| e.name == end_activity)?;
            if start_idx > end_idx {
                return Some(t.clone());
            }
            Some(crate::event_log::Trace {
                case_id: t.case_id.clone(),
                events: t.events[start_idx..=end_idx].to_vec(),
            })
        })
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
    fn test_filter_start_activities() {
        let log = make_test_log();
        let filtered = filter_start_activities(&log, &["A".to_string()]);
        assert_eq!(filtered.traces.len(), 3);
    }

    #[test]
    fn test_filter_end_activities() {
        let log = make_test_log();
        let filtered = filter_end_activities(&log, &["B".to_string()]);
        assert_eq!(filtered.traces.len(), 1);
    }

    #[test]
    fn test_filter_end_activities_multiple() {
        let log = make_test_log();
        let filtered = filter_end_activities(&log, &["B".to_string(), "C".to_string()]);
        assert_eq!(filtered.traces.len(), 3);
    }
}
