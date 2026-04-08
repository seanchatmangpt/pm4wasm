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

/// Filter event log by time range.
///
/// Mirrors `pm4py.filter_time_range()`.

use crate::event_log::EventLog;
use crate::statistics::basic::parse_timestamp;

/// Filter traces to only those that have events within the given time range.
///
/// `start_ms` and `end_ms` are Unix milliseconds since epoch.
/// A trace is kept if ANY of its events fall within [start_ms, end_ms].
pub fn filter_time_range(log: &EventLog, start_ms: i64, end_ms: i64) -> EventLog {
    let traces: Vec<_> = log
        .traces
        .iter()
        .filter(|t| {
            t.events.iter().any(|e| {
                e.timestamp
                    .as_ref()
                    .and_then(|ts| parse_timestamp(ts))
                    .map(|ms| ms >= start_ms && ms <= end_ms)
                    .unwrap_or(false)
            })
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
             2,A,2020-01-02T11:00:00\n\
             2,B,2020-01-02T11:03:00\n\
             2,C,2020-01-02T11:10:00\n",
        )
        .unwrap()
    }

    #[test]
    fn test_filter_time_range() {
        let log = make_test_log();
        // 2020-01-01T00:00:00 to 2020-01-01T23:59:59 = Jan 1 only
        let start = chrono::NaiveDate::from_ymd_opt(2020, 1, 1)
            .unwrap()
            .and_hms_opt(0, 0, 0)
            .unwrap()
            .and_utc()
            .timestamp_millis();
        let end = chrono::NaiveDate::from_ymd_opt(2020, 1, 1)
            .unwrap()
            .and_hms_opt(23, 59, 59)
            .unwrap()
            .and_utc()
            .timestamp_millis();
        let filtered = filter_time_range(&log, start, end);
        assert_eq!(filtered.traces.len(), 1);
        assert_eq!(filtered.traces[0].case_id, "1");
    }

    #[test]
    fn test_filter_time_range_all() {
        let log = make_test_log();
        // Wide range covers everything
        let filtered = filter_time_range(&log, 0, i64::MAX);
        assert_eq!(filtered.traces.len(), 2);
    }
}
