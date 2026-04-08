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

//! OCEL flattening to traditional event logs.
//!
//! **Reference**: `pm4py.objects.ocel.util.flattening`
//!
//! OCEL can be "flattened" to a traditional event log by choosing one
//! object type as the case notion. Each object of that type becomes a case,
//! and events are grouped by the objects they relate to.
//!
//! This enables applying all existing pm4wasm discovery algorithms (DFG,
//! Alpha+, Inductive Miner) to each object-type perspective.

use crate::event_log::{Event, EventLog, Trace};
use crate::ocel::OCEL;
use std::collections::HashMap;

/// Flatten an OCEL to a traditional event log using the specified object type as case ID.
///
/// **Arguments:**
/// * `ocel` - The object-centric event log
/// * `object_type` - The object type to use as case identifier (e.g., "order", "customer")
///
/// **Returns:** A traditional EventLog where each trace corresponds to one object
/// of the specified type, containing all events related to that object in order.
///
/// **Algorithm:**
/// 1. Filter objects to those of the specified type
/// 2. For each object, get all related events
/// 3. Sort events by timestamp
/// 4. Create a trace with the events
///
/// Mirrors `pm4py.ocel_flattening()`.
///
/// **Example:**
/// ```rust
/// let ocel = parse_ocel_json(json_str)?;
/// let order_log = flatten_by_object_type(&ocel, "order")?;
/// let customer_log = flatten_by_object_type(&ocel, "customer")?;
///
/// // Now apply existing discovery algorithms
/// let dfg = discovery::dfg::discover_dfg(&order_log);
/// let net = discovery::inductive_miner::inductive_miner(&customer_log);
/// ```
pub fn flatten_by_object_type(ocel: &OCEL, object_type: &str) -> Result<EventLog, String> {
    // Step 1: Filter objects to those of the specified type
    let target_objects: Vec<_> = ocel
        .objects
        .iter()
        .filter(|obj| obj.object_type == object_type)
        .collect();

    if target_objects.is_empty() {
        return Err(format!(
            "No objects found with type '{}'",
            object_type
        ));
    }

    // Step 2: Build a map from object_id to related events
    let mut object_events: HashMap<String, Vec<&crate::ocel::OCELEvent>> = HashMap::new();

    for obj in &target_objects {
        let events = ocel.get_events_for_object(&obj.id);
        object_events.insert(obj.id.clone(), events);
    }

    // Step 3: Create traces
    let mut traces = Vec::new();

    for (object_id, events) in object_events {
        if events.is_empty() {
            // Objects with no events get an empty trace
            traces.push(Trace {
                case_id: object_id.clone(),
                events: Vec::new(),
            });
            continue;
        }

        // Convert OCEL events to EventLog events
        let log_events: Vec<Event> = events
            .into_iter()
            .map(|ocel_event| {
                // Convert JSON attributes to string attributes
                let attributes: HashMap<String, String> = ocel_event
                    .attributes
                    .iter()
                    .map(|(k, v)| {
                        let value_str = match v {
                            serde_json::Value::String(s) => s.clone(),
                            serde_json::Value::Number(n) => n.to_string(),
                            serde_json::Value::Bool(b) => b.to_string(),
                            serde_json::Value::Null => "null".to_string(),
                            _ => v.to_string(),
                        };
                        (k.clone(), value_str)
                    })
                    .collect();

                Event {
                    name: ocel_event.activity.clone(),
                    timestamp: Some(ocel_event.timestamp.clone()),
                    lifecycle: None,
                    attributes,
                }
            })
            .collect();

        traces.push(Trace {
            case_id: object_id,
            events: log_events,
        });
    }

    // Sort traces by case_id for deterministic output
    traces.sort_by(|a, b| a.case_id.cmp(&b.case_id));

    Ok(EventLog { traces })
}

/// Get all available object types in an OCEL.
///
/// Returns a sorted list of unique object types.
///
/// Mirrors `pm4py.ocel_object_types`.
pub fn get_object_types(ocel: &OCEL) -> Vec<String> {
    let mut types: Vec<String> = ocel.get_object_types();
    types.sort();
    types
}

/// Get all available activities (event types) in an OCEL.
///
/// Returns a sorted list of unique activities.
///
/// Mirrors `pm4py.ocel_event_types`.
pub fn get_event_types(ocel: &OCEL) -> Vec<String> {
    let mut activities: Vec<String> = ocel.get_activities();
    activities.sort();
    activities
}

/// Get statistics about an OCEL.
///
/// Returns counts of events, objects, relations, and object types.
///
/// Mirrors `pm4py.ocel_summary`.
pub fn get_summary(ocel: &OCEL) -> OCELSummary {
    let object_types = get_object_types(ocel);
    let event_types = get_event_types(ocel);

    OCELSummary {
        event_count: ocel.events.len(),
        object_count: ocel.objects.len(),
        relation_count: ocel.relations.len(),
        object_types: object_types.clone(),
        object_type_count: object_types.len(),
        event_types: event_types.clone(),
        event_type_count: event_types.len(),
        is_ocel20: ocel.is_ocel20(),
    }
}

/// Summary statistics for an OCEL.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct OCELSummary {
    /// Total number of events
    pub event_count: usize,
    /// Total number of objects
    pub object_count: usize,
    /// Total number of event-object relations
    pub relation_count: usize,
    /// All unique object types
    pub object_types: Vec<String>,
    /// Number of unique object types
    pub object_type_count: usize,
    /// All unique event types (activities)
    pub event_types: Vec<String>,
    /// Number of unique event types
    pub event_type_count: usize,
    /// Whether this is an OCEL 2.0 log
    pub is_ocel20: bool,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ocel::{OCELEvent, OCELObject, OCELRelation, parse_ocel_json};

    fn make_test_ocel() -> OCEL {
        let json = r#"{
            "events": [
                {"id": "e1", "activity": "Create Order", "timestamp": "2020-01-01T10:00:00Z"},
                {"id": "e2", "activity": "Pay", "timestamp": "2020-01-01T11:00:00Z"},
                {"id": "e3", "activity": "Create Order", "timestamp": "2020-01-01T12:00:00Z"},
                {"id": "e4", "activity": "Ship", "timestamp": "2020-01-01T13:00:00Z"}
            ],
            "objects": [
                {"id": "order1", "type": "order"},
                {"id": "order2", "type": "order"},
                {"id": "customer1", "type": "customer"}
            ],
            "relations": [
                {"event_id": "e1", "object_id": "order1"},
                {"event_id": "e2", "object_id": "order1"},
                {"event_id": "e3", "object_id": "order2"},
                {"event_id": "e4", "object_id": "order2"},
                {"event_id": "e1", "object_id": "customer1"}
            ],
            "globals": {
                "objectTypes": ["order", "customer"],
                "eventTypes": ["Create Order", "Pay", "Ship"]
            }
        }"#;

        parse_ocel_json(json).unwrap()
    }

    #[test]
    fn test_flatten_by_object_type() {
        let ocel = make_test_ocel();

        // Flatten by "order" type
        let order_log = flatten_by_object_type(&ocel, "order").unwrap();

        assert_eq!(order_log.traces.len(), 2);

        // First order (order1)
        let trace1 = &order_log.traces[0];
        assert_eq!(trace1.case_id, "order1");
        assert_eq!(trace1.events.len(), 2);
        assert_eq!(trace1.events[0].name, "Create Order");
        assert_eq!(trace1.events[1].name, "Pay");

        // Second order (order2)
        let trace2 = &order_log.traces[1];
        assert_eq!(trace2.case_id, "order2");
        assert_eq!(trace2.events.len(), 2);
        assert_eq!(trace2.events[0].name, "Create Order");
        assert_eq!(trace2.events[1].name, "Ship");
    }

    #[test]
    fn test_flatten_by_customer_type() {
        let ocel = make_test_ocel();

        // Flatten by "customer" type
        let customer_log = flatten_by_object_type(&ocel, "customer").unwrap();

        assert_eq!(customer_log.traces.len(), 1);
        assert_eq!(customer_log.traces[0].case_id, "customer1");
        // customer1 only related to e1 (Create Order)
        assert_eq!(customer_log.traces[0].events.len(), 1);
        assert_eq!(customer_log.traces[0].events[0].name, "Create Order");
    }

    #[test]
    fn test_flatten_invalid_type() {
        let ocel = make_test_ocel();

        // Try to flatten by non-existent type
        let result = flatten_by_object_type(&ocel, "product");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("No objects found"));
    }

    #[test]
    fn test_get_object_types() {
        let ocel = make_test_ocel();
        let types = get_object_types(&ocel);

        assert_eq!(types.len(), 2);
        assert!(types.contains(&"customer".to_string()));
        assert!(types.contains(&"order".to_string()));
    }

    #[test]
    fn test_get_event_types() {
        let ocel = make_test_ocel();
        let types = get_event_types(&ocel);

        assert_eq!(types.len(), 3);
        assert!(types.contains(&"Create Order".to_string()));
        assert!(types.contains(&"Pay".to_string()));
        assert!(types.contains(&"Ship".to_string()));
    }

    #[test]
    fn test_get_summary() {
        let ocel = make_test_ocel();
        let summary = get_summary(&ocel);

        assert_eq!(summary.event_count, 4);
        assert_eq!(summary.object_count, 3);
        assert_eq!(summary.relation_count, 5);
        assert_eq!(summary.object_type_count, 2);
        assert_eq!(summary.event_type_count, 3);
        assert!(!summary.is_ocel20);
    }
}
