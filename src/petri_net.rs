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

/// Petri net data model for WASM output.
///
/// Mirrors the essential fields from `pm4py/objects/petri_net/obj.py` used
/// by the POWL → Petri net conversion, serialised as JSON for the browser.
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ─── Core structures ─────────────────────────────────────────────────────────

/// A place in a Petri net (represented by a unique name string).
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Place {
    pub name: String,
}

/// A transition in a Petri net.
/// `label` is `None` for silent/invisible transitions.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Transition {
    pub name: String,
    pub label: Option<String>,
    /// Optional properties: "activity", "skippable", "selfloop"
    pub properties: HashMap<String, serde_json::Value>,
}

/// An arc in a Petri net.
/// Arcs connect a place to a transition or a transition to a place.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Arc {
    /// Either a place name or a transition name (source).
    pub source: String,
    /// Either a transition name or a place name (target).
    pub target: String,
    /// Arc weight (default 1).
    pub weight: u32,
}

/// A Petri net: sets of places, transitions and arcs.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct PetriNet {
    pub name: String,
    pub places: Vec<Place>,
    pub transitions: Vec<Transition>,
    pub arcs: Vec<Arc>,
}

/// A marking: maps place names to token counts.
pub type Marking = HashMap<String, u32>;

// ─── Builder helpers ──────────────────────────────────────────────────────────

/// Counter state used during Petri net construction (mirrors Python's `Counts`).
#[derive(Default)]
pub struct Counts {
    pub num_places: u32,
    pub num_hidden: u32,
    pub num_visible: u32,
}

impl Counts {
    pub fn inc_places(&mut self) -> u32 {
        self.num_places += 1;
        self.num_places
    }
    pub fn inc_hidden(&mut self) -> u32 {
        self.num_hidden += 1;
        self.num_hidden
    }
    pub fn inc_visible(&mut self) -> u32 {
        self.num_visible += 1;
        self.num_visible
    }
}

impl PetriNet {
    pub fn new(name: &str) -> Self {
        PetriNet {
            name: name.to_string(),
            places: Vec::new(),
            transitions: Vec::new(),
            arcs: Vec::new(),
        }
    }

    /// Add a place; returns its name.
    pub fn add_place(&mut self, name: &str) -> String {
        self.places.push(Place { name: name.to_string() });
        name.to_string()
    }

    /// Add a visible transition; returns its name.
    pub fn add_transition(&mut self, name: &str, label: Option<String>) -> String {
        self.transitions.push(Transition {
            name: name.to_string(),
            label,
            properties: HashMap::new(),
        });
        name.to_string()
    }

    /// Add a visible transition with properties.
    pub fn add_transition_with_props(
        &mut self,
        name: &str,
        label: Option<String>,
        props: HashMap<String, serde_json::Value>,
    ) -> String {
        self.transitions.push(Transition {
            name: name.to_string(),
            label,
            properties: props,
        });
        name.to_string()
    }

    /// Add a directed arc.
    pub fn add_arc(&mut self, source: &str, target: &str) {
        self.arcs.push(Arc {
            source: source.to_string(),
            target: target.to_string(),
            weight: 1,
        });
    }

    /// Remove a place and all its incident arcs.
    pub fn remove_place(&mut self, name: &str) {
        self.places.retain(|p| p.name != name);
        self.arcs.retain(|a| a.source != name && a.target != name);
    }

    /// Remove a transition and all its incident arcs.
    pub fn remove_transition(&mut self, name: &str) {
        self.transitions.retain(|t| t.name != name);
        self.arcs.retain(|a| a.source != name && a.target != name);
    }

    /// Apply simple structural reduction: remove "pass-through" places
    /// (exactly one in-arc from a transition and one out-arc to a transition)
    /// by merging the two transitions.  Mirrors `reduction.apply_simple_reduction`.
    pub fn apply_simple_reduction(&mut self) {
        loop {
            let mut reduced = false;
            let place_names: Vec<String> =
                self.places.iter().map(|p| p.name.clone()).collect();
            for p_name in &place_names {
                let in_trans: Vec<String> = self
                    .arcs
                    .iter()
                    .filter(|a| &a.target == p_name)
                    .map(|a| a.source.clone())
                    .collect();
                let out_trans: Vec<String> = self
                    .arcs
                    .iter()
                    .filter(|a| &a.source == p_name)
                    .map(|a| a.target.clone())
                    .collect();

                // Only reduce when exactly one hidden transition feeds the place
                // and exactly one hidden transition drains it.
                if in_trans.len() == 1 && out_trans.len() == 1 {
                    let in_t = &in_trans[0];
                    let out_t = &out_trans[0];
                    // Both must be silent (label == None)
                    let in_silent = self
                        .transitions
                        .iter()
                        .find(|t| &t.name == in_t)
                        .map(|t| t.label.is_none())
                        .unwrap_or(false);
                    let out_silent = self
                        .transitions
                        .iter()
                        .find(|t| &t.name == out_t)
                        .map(|t| t.label.is_none())
                        .unwrap_or(false);

                    if in_silent && out_silent && in_t != out_t {
                        // Redirect out_t's in-arcs to in_t
                        let old_out_t = out_t.clone();
                        let old_in_t = in_t.clone();
                        // Find all out-arcs of out_t
                        let targets_of_out_t: Vec<String> = self
                            .arcs
                            .iter()
                            .filter(|a| a.source == old_out_t)
                            .map(|a| a.target.clone())
                            .collect();
                        // Remove the intermediate place and out_t
                        self.remove_place(p_name);
                        self.remove_transition(&old_out_t);
                        // Re-add out-arcs from in_t
                        for tgt in targets_of_out_t {
                            if tgt != *p_name {
                                self.add_arc(&old_in_t, &tgt);
                            }
                        }
                        reduced = true;
                        break;
                    }
                }
            }
            if !reduced {
                break;
            }
        }
    }
}

// ─── Full conversion result ───────────────────────────────────────────────────

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PetriNetResult {
    pub net: PetriNet,
    pub initial_marking: Marking,
    pub final_marking: Marking,
}
