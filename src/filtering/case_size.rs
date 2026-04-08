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

/// Filter event log by case size (number of events per trace).
///
/// Mirrors `pm4py.filter_case_size()`.

use crate::event_log::EventLog;

/// Filter to keep only traces with event count in [min_size, max_size].
///
/// If `max_size` is 0, only `min_size` is used as a lower bound.
pub fn filter_case_size(log: &EventLog, min_size: usize, max_size: usize) -> EventLog {
    let traces: Vec<_> = log
        .traces
        .iter()
        .filter(|t| {
            let len = t.events.len();
            if max_size == 0 {
                len >= min_size
            } else {
                len >= min_size && len <= max_size
            }
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
            "case_id,activity\n\
             1,A\n\
             1,B\n\
             2,A\n\
             2,B\n\
             2,C\n\
             3,A\n",
        )
        .unwrap()
    }

    #[test]
    fn test_filter_case_size() {
        let log = make_test_log();
        let filtered = filter_case_size(&log, 2, 2);
        assert_eq!(filtered.traces.len(), 1);
        assert_eq!(filtered.traces[0].case_id, "1");
    }

    #[test]
    fn test_filter_case_size_min_only() {
        let log = make_test_log();
        let filtered = filter_case_size(&log, 2, 0);
        assert_eq!(filtered.traces.len(), 2);
    }
}
