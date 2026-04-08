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

/// Event log types and parsers (XES and CSV).
///
/// Mirrors the essential pm4py event log concepts used for conformance checking.
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use quick_xml::events::Event as XmlEvent;
use quick_xml::reader::Reader;

// ─── Core types ───────────────────────────────────────────────────────────────

/// A single event in a trace.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Event {
    /// Activity name (concept:name).
    pub name: String,
    /// ISO-8601 timestamp string (time:timestamp), if present.
    pub timestamp: Option<String>,
    /// Lifecycle transition (lifecycle:transition), if present.
    pub lifecycle: Option<String>,
    /// All other attributes as key → value strings.
    pub attributes: HashMap<String, String>,
}

/// An ordered sequence of events for one case.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Trace {
    /// Case identifier (concept:name on the trace, or row-derived).
    pub case_id: String,
    pub events: Vec<Event>,
}

/// A collection of traces.
#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct EventLog {
    pub traces: Vec<Trace>,
}

impl EventLog {
    /// All distinct activity names across all traces, sorted.
    pub fn activities(&self) -> Vec<String> {
        let mut set: std::collections::BTreeSet<String> = Default::default();
        for trace in &self.traces {
            for event in &trace.events {
                set.insert(event.name.clone());
            }
        }
        set.into_iter().collect()
    }

    /// Activity sequences per trace → count map (variants).
    pub fn variants(&self) -> HashMap<Vec<String>, usize> {
        let mut map: HashMap<Vec<String>, usize> = HashMap::new();
        for trace in &self.traces {
            let seq: Vec<String> = trace.events.iter().map(|e| e.name.clone()).collect();
            *map.entry(seq).or_insert(0) += 1;
        }
        map
    }
}

// ─── XES parser ──────────────────────────────────────────────────────────────

/// Parse a XES-formatted XML string into an [`EventLog`].
///
/// Only `concept:name`, `time:timestamp`, and `lifecycle:transition` are
/// promoted to typed fields; all other attributes are stored in `attributes`.
pub fn parse_xes(xml: &str) -> Result<EventLog, String> {
    let mut reader = Reader::from_str(xml);
    reader.config_mut().trim_text(true);

    let mut log = EventLog::default();
    let mut current_trace: Option<Trace> = None;
    let mut current_event: Option<Event> = None;
    let mut in_trace = false;
    let mut in_event = false;

    loop {
        match reader.read_event().map_err(|e| e.to_string())? {
            XmlEvent::Start(ref e) => match e.name().as_ref() {
                b"trace" => {
                    in_trace = true;
                    current_trace = Some(Trace {
                        case_id: String::new(),
                        events: Vec::new(),
                    });
                }
                b"event" if in_trace => {
                    in_event = true;
                    current_event = Some(Event {
                        name: String::new(),
                        timestamp: None,
                        lifecycle: None,
                        attributes: HashMap::new(),
                    });
                }
                _ => {}
            },

            XmlEvent::Empty(ref e) => {
                let tag = e.name();
                let tag_bytes = tag.as_ref();
                // Only handle attribute-bearing elements (string, int, float, date, boolean)
                match tag_bytes {
                    b"string" | b"int" | b"float" | b"date" | b"boolean" => {
                        let mut key = String::new();
                        let mut value = String::new();
                        for attr in e.attributes() {
                            let attr = attr.map_err(|e| e.to_string())?;
                            let k = std::str::from_utf8(attr.key.as_ref())
                                .unwrap_or("")
                                .to_string();
                            let v = attr
                                .unescape_value()
                                .map_err(|e| e.to_string())?
                                .to_string();
                            match k.as_str() {
                                "key" => key = v,
                                "value" => value = v,
                                _ => {}
                            }
                        }
                        if key.is_empty() {
                            continue;
                        }

                        if in_event {
                            if let Some(ev) = current_event.as_mut() {
                                match key.as_str() {
                                    "concept:name" => ev.name = value,
                                    "time:timestamp" => ev.timestamp = Some(value),
                                    "lifecycle:transition" => ev.lifecycle = Some(value),
                                    _ => {
                                        ev.attributes.insert(key, value);
                                    }
                                }
                            }
                        } else if in_trace {
                            if let Some(tr) = current_trace.as_mut() {
                                if key == "concept:name" {
                                    tr.case_id = value;
                                }
                            }
                        }
                    }
                    _ => {}
                }
            }

            XmlEvent::End(ref e) => match e.name().as_ref() {
                b"event" if in_event => {
                    in_event = false;
                    if let (Some(ev), Some(tr)) =
                        (current_event.take(), current_trace.as_mut())
                    {
                        tr.events.push(ev);
                    }
                }
                b"trace" if in_trace => {
                    in_trace = false;
                    if let Some(tr) = current_trace.take() {
                        log.traces.push(tr);
                    }
                }
                _ => {}
            },

            XmlEvent::Eof => break,
            _ => {}
        }
    }

    Ok(log)
}

// ─── CSV parser ───────────────────────────────────────────────────────────────

/// Parse a CSV string into an [`EventLog`].
///
/// Expected columns (first row is header, case-insensitive):
/// - `case_id` / `case:concept:name` / `case` — case identifier
/// - `concept:name` / `activity` — activity name
/// - `time:timestamp` / `timestamp` — optional timestamp
///
/// Traces are grouped by case_id in order of first occurrence.
pub fn parse_csv(csv: &str) -> Result<EventLog, String> {
    let mut lines = csv.lines();
    let header_line = lines.next().ok_or("CSV is empty")?;

    let headers: Vec<&str> = header_line.split(',').map(str::trim).collect();

    // Find column indices
    let case_col = headers
        .iter()
        .position(|h| {
            matches!(
                h.to_lowercase().as_str(),
                "case_id" | "case:concept:name" | "case"
            )
        })
        .ok_or("CSV missing case_id column")?;

    let activity_col = headers
        .iter()
        .position(|h| {
            matches!(h.to_lowercase().as_str(), "concept:name" | "activity")
        })
        .ok_or("CSV missing activity column")?;

    let timestamp_col = headers.iter().position(|h| {
        matches!(h.to_lowercase().as_str(), "time:timestamp" | "timestamp")
    });

    // Build traces in case-order
    let mut case_order: Vec<String> = Vec::new();
    let mut trace_map: HashMap<String, Trace> = HashMap::new();

    for (line_no, line) in lines.enumerate() {
        if line.trim().is_empty() {
            continue;
        }
        let cols: Vec<&str> = line.split(',').collect();

        let case_id = cols
            .get(case_col)
            .map(|s| s.trim().to_string())
            .unwrap_or_else(|| format!("case_{}", line_no));

        let activity = cols
            .get(activity_col)
            .map(|s| s.trim().to_string())
            .unwrap_or_default();

        if activity.is_empty() {
            continue;
        }

        let timestamp = timestamp_col
            .and_then(|i| cols.get(i))
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty());

        let event = Event {
            name: activity,
            timestamp,
            lifecycle: None,
            attributes: HashMap::new(),
        };

        let trace = trace_map.entry(case_id.clone()).or_insert_with(|| {
            case_order.push(case_id.clone());
            Trace {
                case_id: case_id.clone(),
                events: Vec::new(),
            }
        });
        trace.events.push(event);
    }

    let traces = case_order
        .into_iter()
        .filter_map(|id| trace_map.remove(&id))
        .collect();

    Ok(EventLog { traces })
}

// ─── XES writer ────────────────────────────────────────────────────────────────

/// Serialize an [`EventLog`] to XES XML format.
///
/// Produces a valid XES 1.0 XML document with `<string>`, `<date>`, and
/// `<int>` elements for typed attributes.
pub fn write_xes(log: &EventLog) -> String {
    let mut xml = String::from(r#"<?xml version="1.0" encoding="UTF-8"?>"#);
    xml.push_str("\n<log xes.version=\"1.0\" xes.features=\"nested-attributes\">\n");

    for trace in &log.traces {
        xml.push_str("  <trace>\n");
        xml.push_str(&format!(
            "    <string key=\"concept:name\" value=\"{}\"/>\n",
            escape_xml(&trace.case_id)
        ));
        for event in &trace.events {
            xml.push_str("    <event>\n");
            xml.push_str(&format!(
                "      <string key=\"concept:name\" value=\"{}\"/>\n",
                escape_xml(&event.name)
            ));
            if let Some(ts) = &event.timestamp {
                xml.push_str(&format!(
                    "      <date key=\"time:timestamp\" value=\"{}\"/>\n",
                    escape_xml(ts)
                ));
            }
            if let Some(lc) = &event.lifecycle {
                xml.push_str(&format!(
                    "      <string key=\"lifecycle:transition\" value=\"{}\"/>\n",
                    escape_xml(lc)
                ));
            }
            for (key, value) in &event.attributes {
                xml.push_str(&format!(
                    "      <string key=\"{}\" value=\"{}\"/>\n",
                    escape_xml(key),
                    escape_xml(value)
                ));
            }
            xml.push_str("    </event>\n");
        }
        xml.push_str("  </trace>\n");
    }

    xml.push_str("</log>\n");
    xml
}

/// Minimal XML entity escaping.
fn escape_xml(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for ch in s.chars() {
        match ch {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&apos;"),
            _ => out.push(ch),
        }
    }
    out
}

// ─── CSV writer ────────────────────────────────────────────────────────────────

/// Serialize an [`EventLog`] to CSV format.
///
/// Header: `case_id,activity,timestamp`
/// If a trace has events with timestamps, the timestamp column is included.
pub fn write_csv(log: &EventLog) -> String {
    let has_timestamps = log
        .traces
        .iter()
        .any(|t| t.events.iter().any(|e| e.timestamp.is_some()));

    let mut csv = String::from("case_id,activity");
    if has_timestamps {
        csv.push_str(",timestamp");
    }
    csv.push('\n');

    for trace in &log.traces {
        for event in &trace.events {
            csv.push_str(&trace.case_id);
            csv.push(',');
            csv.push_str(&event.name);
            if has_timestamps {
                csv.push(',');
                csv.push_str(event.timestamp.as_deref().unwrap_or(""));
            }
            csv.push('\n');
        }
    }

    csv
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_simple_xes() {
        let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<log xes.version="1.0">
  <trace>
    <string key="concept:name" value="case1"/>
    <event>
      <string key="concept:name" value="A"/>
      <date key="time:timestamp" value="2020-01-01T00:00:00"/>
    </event>
    <event>
      <string key="concept:name" value="B"/>
    </event>
  </trace>
  <trace>
    <string key="concept:name" value="case2"/>
    <event>
      <string key="concept:name" value="A"/>
    </event>
    <event>
      <string key="concept:name" value="C"/>
    </event>
  </trace>
</log>"#;
        let log = parse_xes(xml).unwrap();
        assert_eq!(log.traces.len(), 2);
        assert_eq!(log.traces[0].case_id, "case1");
        assert_eq!(log.traces[0].events.len(), 2);
        assert_eq!(log.traces[0].events[0].name, "A");
        assert_eq!(
            log.traces[0].events[0].timestamp.as_deref(),
            Some("2020-01-01T00:00:00")
        );
        assert_eq!(log.traces[1].events[1].name, "C");
    }

    #[test]
    fn parse_simple_csv() {
        let csv = "case_id,activity,timestamp\n\
                   1,A,2020-01-01\n\
                   1,B,2020-01-02\n\
                   2,A,2020-01-03\n\
                   2,C,2020-01-04\n";
        let log = parse_csv(csv).unwrap();
        assert_eq!(log.traces.len(), 2);
        assert_eq!(log.traces[0].case_id, "1");
        assert_eq!(log.traces[0].events.len(), 2);
        assert_eq!(log.traces[0].events[0].name, "A");
        assert_eq!(log.traces[1].events[1].name, "C");
    }

    #[test]
    fn csv_case_column_aliases() {
        let csv = "case:concept:name,concept:name\n\
                   x,A\n\
                   x,B\n";
        let log = parse_csv(csv).unwrap();
        assert_eq!(log.traces[0].case_id, "x");
        assert_eq!(log.traces[0].events.len(), 2);
    }

    #[test]
    fn activities_and_variants() {
        let csv = "case_id,activity\n\
                   1,A\n\
                   1,B\n\
                   2,A\n\
                   2,B\n\
                   3,A\n\
                   3,C\n";
        let log = parse_csv(csv).unwrap();
        let acts = log.activities();
        assert_eq!(acts, vec!["A", "B", "C"]);

        let vars = log.variants();
        assert_eq!(*vars.get(&vec!["A".into(), "B".into()]).unwrap(), 2);
        assert_eq!(*vars.get(&vec!["A".into(), "C".into()]).unwrap(), 1);
    }
}
