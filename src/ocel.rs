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

//! Object-Centric Event Log (OCEL) support.
//!
//! **Reference**: `pm4py.objects.ocel.obj.OCEL`
//!
//! OCEL is a different paradigm where each event can relate to multiple objects.
//! Instead of traces grouped by case ID, OCEL has:
//! - **Events**: What happened (activity, timestamp)
//! - **Objects**: Entities involved (object type, attributes)
//! - **Relations**: Many-to-many mapping between events and objects
//!
//! This implementation provides a lightweight OCEL structure compatible with WASM,
//! avoiding the pandas dependency from pm4py's Python implementation.

pub mod flattening;
pub mod etot;

pub use flattening::{flatten_by_object_type, get_object_types, get_event_types, get_summary, OCELSummary};
pub use etot::{discover_etot, get_object_types_for_activity, get_activities_for_object_type, ETOTResult};

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Object-Centric Event Log.
///
/// Contains events, objects, and their many-to-many relations.
/// Unlike traditional event logs (traces with case IDs), OCEL allows
/// events to relate to multiple objects simultaneously.
///
/// **Example**: An "Order Shipped" event might relate to:
/// - Order #123 (type: "order")
/// - Customer #456 (type: "customer")
/// - Item #789 (type: "item")
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct OCEL {
    /// All events in the log
    pub events: Vec<OCELEvent>,
    /// All objects referenced by events
    pub objects: Vec<OCELObject>,
    /// Many-to-many event-object relations
    pub relations: Vec<OCELRelation>,
    /// Global metadata (object types, event types, etc.)
    pub globals: OCELGlobals,
    /// Object-to-object relationships (OCEL 2.0 feature)
    #[serde(rename = "o2o", default)]
    pub object_to_object: Vec<OCELRelation>,
    /// Event-to-event relationships (OCEL 2.0 feature)
    #[serde(rename = "e2e", default)]
    pub event_to_event: Vec<OCELRelation>,
}

/// A single event in an OCEL.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct OCELEvent {
    /// Unique event identifier
    pub id: String,
    /// Activity name (e.g., "Create Order")
    pub activity: String,
    /// Timestamp in ISO 8601 format
    pub timestamp: String,
    /// Additional event attributes
    #[serde(flatten)]
    pub attributes: HashMap<String, serde_json::Value>,
}

/// A single object in an OCEL.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct OCELObject {
    /// Unique object identifier
    pub id: String,
    /// Object type (e.g., "order", "customer", "item")
    #[serde(rename = "type")]
    pub object_type: String,
    /// Additional object attributes
    #[serde(flatten)]
    pub attributes: HashMap<String, serde_json::Value>,
}

/// A relation between an event and an object.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct OCELRelation {
    /// Event identifier
    pub event_id: String,
    /// Object identifier
    pub object_id: String,
    /// Qualifier describing the role (e.g., "creator", "assigned to")
    pub qualifier: Option<String>,
}

/// Global metadata about the OCEL.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct OCELGlobals {
    /// OCEL version (e.g., "1.0", "2.0")
    pub version: Option<String>,
    /// All object types in the log
    #[serde(rename = "objectTypes")]
    pub object_types: Vec<String>,
    /// All event types (activities) in the log
    #[serde(rename = "eventTypes")]
    pub event_types: Vec<String>,
    /// Attribute name mappings
    #[serde(default)]
    pub attributes: HashMap<String, String>,
}

impl Default for OCELGlobals {
    fn default() -> Self {
        OCELGlobals {
            version: Some("1.0".to_string()),
            object_types: Vec::new(),
            event_types: Vec::new(),
            attributes: HashMap::new(),
        }
    }
}

impl Default for OCEL {
    fn default() -> Self {
        OCEL {
            events: Vec::new(),
            objects: Vec::new(),
            relations: Vec::new(),
            globals: OCELGlobals::default(),
            object_to_object: Vec::new(),
            event_to_event: Vec::new(),
        }
    }
}

impl OCEL {
    /// Create a new empty OCEL.
    pub fn new() -> Self {
        Self::default()
    }

    /// Get all unique object types in the OCEL.
    pub fn get_object_types(&self) -> Vec<String> {
        let mut types: std::collections::HashSet<String> = std::collections::HashSet::new();
        for obj in &self.objects {
            types.insert(obj.object_type.clone());
        }
        types.into_iter().collect()
    }

    /// Get all unique activities (event types) in the OCEL.
    pub fn get_activities(&self) -> Vec<String> {
        let mut activities: std::collections::HashSet<String> = std::collections::HashSet::new();
        for event in &self.events {
            activities.insert(event.activity.clone());
        }
        activities.into_iter().collect()
    }

    /// Get all objects related to a specific event.
    pub fn get_objects_for_event(&self, event_id: &str) -> Vec<&OCELObject> {
        let mut object_ids = Vec::new();
        for rel in &self.relations {
            if rel.event_id == event_id {
                object_ids.push(&rel.object_id);
            }
        }

        let mut objects = Vec::new();
        for obj in &self.objects {
            if object_ids.iter().any(|id| *id == &obj.id) {
                objects.push(obj);
            }
        }
        objects
    }

    /// Get all events related to a specific object.
    pub fn get_events_for_object(&self, object_id: &str) -> Vec<&OCELEvent> {
        let mut event_ids = Vec::new();
        for rel in &self.relations {
            if rel.object_id == object_id {
                event_ids.push(&rel.event_id);
            }
        }

        let mut events = Vec::new();
        for event in &self.events {
            if event_ids.iter().any(|id| *id == &event.id) {
                events.push(event);
            }
        }
        // Sort by timestamp
        events.sort_by(|a, b| a.timestamp.cmp(&b.timestamp));
        events
    }

    /// Check if this is an OCEL 2.0 log.
    ///
    /// OCEL 2.0 includes object-to-object and event-to-event relationships.
    pub fn is_ocel20(&self) -> bool {
        !self.object_to_object.is_empty() || !self.event_to_event.is_empty()
    }
}

/// Parse OCEL from JSON string.
///
/// Accepts JSON-OCEL format (both 1.0 and 2.0).
/// Returns an error if JSON is invalid or doesn't match OCEL schema.
///
/// Mirrors `pm4py.read_ocel()`.
pub fn parse_ocel_json(json: &str) -> Result<OCEL, String> {
    serde_json::from_str(json).map_err(|e| format!("OCEL JSON parse error: {}", e))
}

/// Serialize OCEL to JSON string.
///
/// Produces JSON-OCEL format compatible with pm4py.
pub fn serialize_ocel_json(ocel: &OCEL) -> Result<String, String> {
    serde_json::to_string_pretty(ocel)
        .map_err(|e| format!("OCEL JSON serialization error: {}", e))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ocel_empty() {
        let ocel = OCEL::new();
        assert!(ocel.events.is_empty());
        assert!(ocel.objects.is_empty());
        assert!(ocel.relations.is_empty());
    }

    #[test]
    fn test_ocel_parse_simple() {
        let json = r#"{
            "events": [
                {"id": "e1", "activity": "Create Order", "timestamp": "2020-01-01T10:00:00Z"}
            ],
            "objects": [
                {"id": "o1", "type": "order"}
            ],
            "relations": [
                {"event_id": "e1", "object_id": "o1"}
            ],
            "globals": {
                "objectTypes": ["order"],
                "eventTypes": ["Create Order"]
            }
        }"#;

        let ocel = parse_ocel_json(json).unwrap();
        assert_eq!(ocel.events.len(), 1);
        assert_eq!(ocel.objects.len(), 1);
        assert_eq!(ocel.relations.len(), 1);
        assert_eq!(ocel.events[0].activity, "Create Order");
        assert_eq!(ocel.objects[0].object_type, "order");
    }

    #[test]
    fn test_get_object_types() {
        let mut ocel = OCEL::new();
        ocel.objects = vec![
            OCELObject {
                id: "o1".to_string(),
                object_type: "order".to_string(),
                attributes: HashMap::new(),
            },
            OCELObject {
                id: "o2".to_string(),
                object_type: "customer".to_string(),
                attributes: HashMap::new(),
            },
            OCELObject {
                id: "o3".to_string(),
                object_type: "order".to_string(),
                attributes: HashMap::new(),
            },
        ];

        let types = ocel.get_object_types();
        assert_eq!(types.len(), 2);
        assert!(types.contains(&"order".to_string()));
        assert!(types.contains(&"customer".to_string()));
    }

    #[test]
    fn test_get_activities() {
        let mut ocel = OCEL::new();
        ocel.events = vec![
            OCELEvent {
                id: "e1".to_string(),
                activity: "Create Order".to_string(),
                timestamp: "2020-01-01T10:00:00Z".to_string(),
                attributes: HashMap::new(),
            },
            OCELEvent {
                id: "e2".to_string(),
                activity: "Pay".to_string(),
                timestamp: "2020-01-01T11:00:00Z".to_string(),
                attributes: HashMap::new(),
            },
            OCELEvent {
                id: "e3".to_string(),
                activity: "Create Order".to_string(),
                timestamp: "2020-01-01T12:00:00Z".to_string(),
                attributes: HashMap::new(),
            },
        ];

        let activities = ocel.get_activities();
        assert_eq!(activities.len(), 2);
        assert!(activities.contains(&"Create Order".to_string()));
        assert!(activities.contains(&"Pay".to_string()));
    }

    #[test]
    fn test_get_objects_for_event() {
        let mut ocel = OCEL::new();
        ocel.objects = vec![
            OCELObject {
                id: "o1".to_string(),
                object_type: "order".to_string(),
                attributes: HashMap::new(),
            },
            OCELObject {
                id: "o2".to_string(),
                object_type: "customer".to_string(),
                attributes: HashMap::new(),
            },
        ];
        ocel.relations = vec![
            OCELRelation {
                event_id: "e1".to_string(),
                object_id: "o1".to_string(),
                qualifier: None,
            },
            OCELRelation {
                event_id: "e1".to_string(),
                object_id: "o2".to_string(),
                qualifier: None,
            },
        ];

        let objects = ocel.get_objects_for_event("e1");
        assert_eq!(objects.len(), 2);
        assert_eq!(objects[0].id, "o1");
        assert_eq!(objects[1].id, "o2");
    }

    #[test]
    fn test_get_events_for_object() {
        let mut ocel = OCEL::new();
        ocel.events = vec![
            OCELEvent {
                id: "e1".to_string(),
                activity: "Create Order".to_string(),
                timestamp: "2020-01-01T10:00:00Z".to_string(),
                attributes: HashMap::new(),
            },
            OCELEvent {
                id: "e2".to_string(),
                activity: "Pay".to_string(),
                timestamp: "2020-01-01T11:00:00Z".to_string(),
                attributes: HashMap::new(),
            },
        ];
        ocel.relations = vec![
            OCELRelation {
                event_id: "e1".to_string(),
                object_id: "o1".to_string(),
                qualifier: None,
            },
            OCELRelation {
                event_id: "e2".to_string(),
                object_id: "o1".to_string(),
                qualifier: None,
            },
        ];

        let events = ocel.get_events_for_object("o1");
        assert_eq!(events.len(), 2);
        assert_eq!(events[0].activity, "Create Order");
        assert_eq!(events[1].activity, "Pay");
    }

    #[test]
    fn test_is_ocel20() {
        let mut ocel = OCEL::new();
        assert!(!ocel.is_ocel20());

        ocel.object_to_object = vec![OCELRelation {
            event_id: "e1".to_string(),
            object_id: "o1".to_string(),
            qualifier: None,
        }];
        assert!(ocel.is_ocel20());
    }
}
