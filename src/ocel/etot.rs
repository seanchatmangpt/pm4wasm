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

//! Event-Type / Object-Type graph discovery from OCEL.
//!
//! **Reference**: `pm4py.algo.discovery.ocel.etot`
//!
//! The ETOT graph captures which activity types are related to which object types.
//! This is a lightweight way to understand the structure of an object-centric process.
//!
//! Example result:
//! - Activities: {"Create Order", "Pay", "Ship"}
//! - Object types: {"order", "customer"}
//! - Edges: {("Create Order", "order"), ("Create Order", "customer"), ("Pay", "order")}
//! - Frequencies: {("Create Order", "order"): 100, ("Create Order", "customer"): 50}

use crate::ocel::OCEL;
use std::collections::{HashMap, HashSet};

/// Event-Type / Object-Type graph result.
///
/// Represents the bipartite graph between activities and object types.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct ETOTResult {
    /// All unique activity names in the OCEL
    pub activities: Vec<String>,
    /// All unique object types in the OCEL
    pub object_types: Vec<String>,
    /// Edges connecting activities to object types: (activity, object_type)
    pub edges: Vec<(String, String)>,
    /// Frequency of each edge: how many times an activity relates to an object type
    pub edge_frequencies: HashMap<String, u32>,
}

/// Discover the Event-Type / Object-Type graph from an OCEL.
///
/// **Arguments:**
/// * `ocel` - The object-centric event log
///
/// **Returns:** An ETOTResult containing activities, object types, edges, and frequencies.
///
/// **Algorithm:**
/// 1. Iterate over all relations
/// 2. For each relation, find the corresponding event's activity and object's type
/// 3. Build edges (activity, object_type)
/// 4. Count frequencies of each edge
///
/// **Time Complexity:** O(n) where n is the number of relations
///
/// Mirrors `pm4py.discover_ocel_etot()`.
pub fn discover_etot(ocel: &OCEL) -> ETOTResult {
    let mut activities: HashSet<String> = HashSet::new();
    let mut object_types: HashSet<String> = HashSet::new();
    let mut edges: HashSet<(String, String)> = HashSet::new();
    let mut edge_counts: HashMap<(String, String), u32> = HashMap::new();

    // Build lookup maps for efficiency
    let mut event_activity: HashMap<String, String> = HashMap::new();
    for event in &ocel.events {
        event_activity.insert(event.id.clone(), event.activity.clone());
        activities.insert(event.activity.clone());
    }

    let mut object_type_map: HashMap<String, String> = HashMap::new();
    for obj in &ocel.objects {
        object_type_map.insert(obj.id.clone(), obj.object_type.clone());
        object_types.insert(obj.object_type.clone());
    }

    // Iterate over relations to build edges
    for rel in &ocel.relations {
        if let Some(activity) = event_activity.get(&rel.event_id) {
            if let Some(ot) = object_type_map.get(&rel.object_id) {
                let edge = (activity.clone(), ot.clone());
                edges.insert(edge.clone());
                *edge_counts.entry(edge).or_insert(0) += 1;
            }
        }
    }

    // Convert to sorted vectors for deterministic output
    let mut activities_vec: Vec<String> = activities.into_iter().collect();
    activities_vec.sort();

    let mut object_types_vec: Vec<String> = object_types.into_iter().collect();
    object_types_vec.sort();

    let mut edges_vec: Vec<(String, String)> = edges.into_iter().collect();
    edges_vec.sort();

    // Convert edge frequencies to string-keyed map for JSON serialization
    let mut edge_frequencies: HashMap<String, u32> = HashMap::new();
    for ((activity, ot), count) in edge_counts {
        let key = format!("{}|{}", activity, ot);
        edge_frequencies.insert(key, count);
    }

    ETOTResult {
        activities: activities_vec,
        object_types: object_types_vec,
        edges: edges_vec,
        edge_frequencies,
    }
}

/// Get object types related to a specific activity.
///
/// Returns all object types that appear in relations with the given activity.
pub fn get_object_types_for_activity(ocel: &OCEL, activity: &str) -> Vec<String> {
    let mut related_types: HashSet<String> = HashSet::new();

    // Build lookup maps
    let mut event_activity: HashMap<String, String> = HashMap::new();
    for event in &ocel.events {
        event_activity.insert(event.id.clone(), event.activity.clone());
    }

    let mut object_type_map: HashMap<String, String> = HashMap::new();
    for obj in &ocel.objects {
        object_type_map.insert(obj.id.clone(), obj.object_type.clone());
    }

    // Find relations for this activity
    for rel in &ocel.relations {
        if let Some(ea) = event_activity.get(&rel.event_id) {
            if ea == activity {
                if let Some(ot) = object_type_map.get(&rel.object_id) {
                    related_types.insert(ot.clone());
                }
            }
        }
    }

    let mut result: Vec<String> = related_types.into_iter().collect();
    result.sort();
    result
}

/// Get activities related to a specific object type.
///
/// Returns all activities that relate to objects of the given type.
pub fn get_activities_for_object_type(ocel: &OCEL, object_type: &str) -> Vec<String> {
    let mut related_activities: HashSet<String> = HashSet::new();

    // Build lookup maps
    let mut event_activity: HashMap<String, String> = HashMap::new();
    for event in &ocel.events {
        event_activity.insert(event.id.clone(), event.activity.clone());
    }

    let mut object_type_map: HashMap<String, String> = HashMap::new();
    for obj in &ocel.objects {
        object_type_map.insert(obj.id.clone(), obj.object_type.clone());
    }

    // Find relations for this object type
    for rel in &ocel.relations {
        if let Some(ot) = object_type_map.get(&rel.object_id) {
            if ot == object_type {
                if let Some(activity) = event_activity.get(&rel.event_id) {
                    related_activities.insert(activity.clone());
                }
            }
        }
    }

    let mut result: Vec<String> = related_activities.into_iter().collect();
    result.sort();
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ocel::parse_ocel_json;

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
    fn test_discover_etot() {
        let ocel = make_test_ocel();
        let etot = discover_etot(&ocel);

        // Should have 3 activities
        assert_eq!(etot.activities.len(), 3);
        assert!(etot.activities.contains(&"Create Order".to_string()));
        assert!(etot.activities.contains(&"Pay".to_string()));
        assert!(etot.activities.contains(&"Ship".to_string()));

        // Should have 2 object types
        assert_eq!(etot.object_types.len(), 2);
        assert!(etot.object_types.contains(&"order".to_string()));
        assert!(etot.object_types.contains(&"customer".to_string()));

        // Should have 3 edges: Create Order->order, Create Order->customer, Pay->order, Ship->order
        assert!(etot.edges.len() >= 3);
        assert!(etot.edges.contains(&(String::from("Create Order"), String::from("order"))));
        assert!(etot.edges.contains(&(String::from("Create Order"), String::from("customer"))));
        assert!(etot.edges.contains(&(String::from("Pay"), String::from("order"))));
    }

    #[test]
    fn test_etot_edge_frequencies() {
        let ocel = make_test_ocel();
        let etot = discover_etot(&ocel);

        // Create Order appears twice with order objects (e1->order1, e3->order2)
        let key = "Create Order|order";
        assert_eq!(etot.edge_frequencies.get(key), Some(&2));

        // Create Order appears once with customer objects (e1->customer1)
        let key = "Create Order|customer";
        assert_eq!(etot.edge_frequencies.get(key), Some(&1));

        // Pay appears once with order objects (e2->order1)
        let key = "Pay|order";
        assert_eq!(etot.edge_frequencies.get(key), Some(&1));
    }

    #[test]
    fn test_get_object_types_for_activity() {
        let ocel = make_test_ocel();

        // Create Order relates to both order and customer
        let types = get_object_types_for_activity(&ocel, "Create Order");
        assert_eq!(types.len(), 2);
        assert!(types.contains(&"order".to_string()));
        assert!(types.contains(&"customer".to_string()));

        // Pay only relates to order
        let types = get_object_types_for_activity(&ocel, "Pay");
        assert_eq!(types.len(), 1);
        assert_eq!(types[0], "order");
    }

    #[test]
    fn test_get_activities_for_object_type() {
        let ocel = make_test_ocel();

        // order type relates to all three activities
        let activities = get_activities_for_object_type(&ocel, "order");
        assert_eq!(activities.len(), 3);
        assert!(activities.contains(&"Create Order".to_string()));
        assert!(activities.contains(&"Pay".to_string()));
        assert!(activities.contains(&"Ship".to_string()));

        // customer type only relates to Create Order
        let activities = get_activities_for_object_type(&ocel, "customer");
        assert_eq!(activities.len(), 1);
        assert_eq!(activities[0], "Create Order");
    }
}
