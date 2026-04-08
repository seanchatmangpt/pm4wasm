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

/// PNML (Petri Net Markup Language) import/export.
///
/// Supports the PNML 2.0 standard for exchanging Petri nets between tools.
use crate::petri_net::{Marking, PetriNet, PetriNetResult};
#[cfg(test)]
use crate::petri_net::{Arc, Place, Transition};
use quick_xml::events::Event as XmlEvent;
use quick_xml::reader::Reader;
use wasm_bindgen::prelude::*;

/// Convert a PetriNetResult to PNML 2.0 XML format.
///
/// # Arguments
/// * `pn` - PetriNetResult containing the Petri net structure
///
/// # Returns
/// * PNML XML string
///
/// # Example
/// ```ignore
/// let pn = PetriNetResult {
///     net: PetriNet {
///         name: "My Net".to_string(),
///         places: vec![
///             Place { name: "p1".to_string() },
///             Place { name: "p2".to_string() },
///         ],
///         transitions: vec![
///             Transition {
///                 name: "t1".to_string(),
///                 label: Some("A".to_string()),
///                 properties: std::collections::HashMap::new(),
///             },
///         ],
///         arcs: vec![
///             Arc { source: "p1".to_string(), target: "t1".to_string(), weight: 1 },
///             Arc { source: "t1".to_string(), target: "p2".to_string(), weight: 1 },
///         ],
///     },
///     initial_marking: vec![("p1".to_string(), 1)].into_iter().collect(),
///     final_marking: vec![("p2".to_string(), 1)].into_iter().collect(),
/// };
///
/// let pnml = to_pnml(&pn);
/// ```
pub fn to_pnml(pn: &PetriNetResult) -> String {
    let mut xml = String::from("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
    xml.push_str("<pnml xmlns=\"http://www.pnml.org/version-2009/grammar\">\n");

    // Add net
    xml.push_str("  <net id=\"");
    xml.push_str(&pn.net.name);
    xml.push_str("\" type=\"P/T-net\">\n");

    // Add places
    for place in &pn.net.places {
        xml.push_str("    <place id=\"");
        xml.push_str(&place.name);
        xml.push_str("\">\n");
        xml.push_str("      <graphics/>\n");
        xml.push_str("    </place>\n");
    }

    // Add transitions
    for transition in &pn.net.transitions {
        xml.push_str("    <transition id=\"");
        xml.push_str(&transition.name);

        // Add label if present
        if let Some(label) = &transition.label {
            xml.push_str("\" name=\"");
            xml.push_str(label);
        }

        xml.push_str("\">\n");

        // Add tool-specific info if needed
        if !transition.properties.is_empty() {
            xml.push_str("      <toolspecific tool=\"ProM\">\n");
            // Could add tool-specific properties here
            xml.push_str("      </toolspecific>\n");
        }

        xml.push_str("      <graphics/>\n");
        xml.push_str("    </transition>\n");
    }

    // Add arcs
    for arc in &pn.net.arcs {
        xml.push_str("    <arc source=\"");
        xml.push_str(&arc.source);
        xml.push_str("\" target=\"");
        xml.push_str(&arc.target);
        xml.push_str("\">\n");

        // Add inscription (weight)
        xml.push_str("      <inscription>\n");
        xml.push_str("        <text>");
        xml.push_str(&arc.weight.to_string());
        xml.push_str("</text>\n");
        xml.push_str("      </inscription>\n");

        xml.push_str("      <graphics/>\n");
        xml.push_str("    </arc>\n");
    }

    // Add initial marking
    xml.push_str("    <initialmarking>\n");
    for (place, tokens) in &pn.initial_marking {
        if *tokens > 0 {
            xml.push_str("      <place idref=\"");
            xml.push_str(place);
            xml.push_str("\">\n");
            xml.push_str("        <text>");
            xml.push_str(&tokens.to_string());
            xml.push_str("</text>\n");
            xml.push_str("      </place>\n");
        }
    }
    xml.push_str("    </initialmarking>\n");

    // Add final marking
    xml.push_str("    <finalmarking>\n");
    for (place, tokens) in &pn.final_marking {
        if *tokens > 0 {
            xml.push_str("      <place idref=\"");
            xml.push_str(place);
            xml.push_str("\">\n");
            xml.push_str("        <text>");
            xml.push_str(&tokens.to_string());
            xml.push_str("</text>\n");
            xml.push_str("      </place>\n");
        }
    }
    xml.push_str("    </finalmarking>\n");

    xml.push_str("  </net>\n");
    xml.push_str("</pnml>\n");

    xml
}

/// Convert a PetriNetResult (JSON) to PNML 2.0 XML format.
///
/// # WASM Export
///
/// # Arguments
/// * `pn_json` - JSON string of PetriNetResult
///
/// # Returns
/// * PNML XML string
///
/// # Errors
/// * Returns JsValue error if JSON parsing fails
#[wasm_bindgen]
pub fn to_pnml_json(pn_json: &str) -> Result<String, JsValue> {
    let pn: PetriNetResult = serde_json::from_str(pn_json)
        .map_err(|e| JsValue::from_str(&format!("Failed to parse PetriNetResult JSON: {}", e)))?;
    Ok(to_pnml(&pn))
}

// ─── PNML Import ──────────────────────────────────────────────────────────────

/// Parse a PNML 2.0 XML string into a PetriNetResult.
///
/// Handles places, transitions (with labels), arcs, initial and final markings.
pub fn from_pnml(xml: &str) -> Result<PetriNetResult, String> {
    let mut reader = Reader::from_str(xml);
    reader.config_mut().trim_text(true);

    let mut net = PetriNet::new("");
    let mut initial_marking: Marking = Marking::new();
    let mut final_marking: Marking = Marking::new();
    let mut current_transition_id: Option<String> = None;
    let mut current_transition_label: Option<String> = None;
    let mut in_initial_marking = false;
    let mut in_final_marking = false;
    let mut current_marking_place: Option<String> = None;
    let mut current_marking_text: Option<String> = None;
    let mut arc_source: Option<String> = None;
    let mut arc_target: Option<String> = None;

    loop {
        match reader.read_event().map_err(|e| e.to_string())? {
            XmlEvent::Start(ref e) => {
                let tag = e.name();
                match tag.as_ref() {
                    b"net" => {
                        for attr in e.attributes().flatten() {
                            if attr.key.as_ref() == b"id" {
                                net.name = String::from_utf8_lossy(&attr.value).to_string();
                            }
                        }
                    }
                    b"place" => {
                        if in_initial_marking || in_final_marking {
                            // Marking place: extract idref
                            for attr in e.attributes().flatten() {
                                if attr.key.as_ref() == b"idref" {
                                    current_marking_place =
                                        Some(String::from_utf8_lossy(&attr.value).to_string());
                                }
                            }
                        } else {
                            // Net place: extract id and add
                            for attr in e.attributes().flatten() {
                                if attr.key.as_ref() == b"id" {
                                    let id = String::from_utf8_lossy(&attr.value).to_string();
                                    net.add_place(&id);
                                }
                            }
                        }
                    }
                    b"transition" => {
                        current_transition_id = None;
                        current_transition_label = None;
                        for attr in e.attributes().flatten() {
                            match attr.key.as_ref() {
                                b"id" => {
                                    current_transition_id =
                                        Some(String::from_utf8_lossy(&attr.value).to_string());
                                }
                                b"name" => {
                                    current_transition_label =
                                        Some(String::from_utf8_lossy(&attr.value).to_string());
                                }
                                _ => {}
                            }
                        }
                    }
                    b"arc" => {
                        arc_source = None;
                        arc_target = None;
                        for attr in e.attributes().flatten() {
                            match attr.key.as_ref() {
                                b"source" => {
                                    arc_source =
                                        Some(String::from_utf8_lossy(&attr.value).to_string());
                                }
                                b"target" => {
                                    arc_target =
                                        Some(String::from_utf8_lossy(&attr.value).to_string());
                                }
                                _ => {}
                            }
                        }
                    }
                    b"initialmarking" => in_initial_marking = true,
                    b"finalmarking" => in_final_marking = true,
                    _ => {}
                }
            }

            XmlEvent::Empty(ref e) => {
                let tag = e.name();
                match tag.as_ref() {
                    b"arc" => {
                        let mut source = String::new();
                        let mut target = String::new();
                        for attr in e.attributes().flatten() {
                            match attr.key.as_ref() {
                                b"source" => {
                                    source = String::from_utf8_lossy(&attr.value).to_string();
                                }
                                b"target" => {
                                    target = String::from_utf8_lossy(&attr.value).to_string();
                                }
                                _ => {}
                            }
                        }
                        net.add_arc(&source, &target);
                    }
                    b"place" if in_initial_marking || in_final_marking => {
                        for attr in e.attributes().flatten() {
                            if attr.key.as_ref() == b"idref" {
                                current_marking_place =
                                    Some(String::from_utf8_lossy(&attr.value).to_string());
                            }
                        }
                    }
                    _ => {}
                }
            }

            XmlEvent::Text(ref e) => {
                let text = e.unescape().map_err(|e| e.to_string())?;
                let trimmed = text.trim();
                if !trimmed.is_empty() {
                    current_marking_text = Some(trimmed.to_string());
                }
            }

            XmlEvent::End(ref e) => {
                let tag = e.name();
                match tag.as_ref() {
                    b"transition" => {
                        let id = current_transition_id.take();
                        let label = current_transition_label.take();
                        if let Some(id) = id {
                            net.add_transition(&id, label);
                        }
                    }
                    b"initialmarking" => {
                        in_initial_marking = false;
                    }
                    b"finalmarking" => {
                        in_final_marking = false;
                    }
                    b"place" => {
                        if in_initial_marking || in_final_marking {
                            if let (Some(place_id), Some(text)) =
                                (current_marking_place.take(), current_marking_text.take())
                            {
                                let tokens: u32 = text.parse().unwrap_or(0);
                                if tokens > 0 {
                                    if in_initial_marking {
                                        initial_marking.insert(place_id, tokens);
                                    } else {
                                        final_marking.insert(place_id, tokens);
                                    }
                                }
                            }
                            current_marking_text = None;
                        }
                    }
                    b"arc" => {
                        if let (Some(source), Some(target)) =
                            (arc_source.take(), arc_target.take())
                        {
                            net.add_arc(&source, &target);
                        }
                    }
                    _ => {}
                }
            }

            XmlEvent::Eof => break,
            _ => {}
        }
    }

    Ok(PetriNetResult {
        net,
        initial_marking,
        final_marking,
    })
}

/// Parse a PNML XML string (JSON-free) for WASM consumers.
#[wasm_bindgen]
pub fn from_pnml_string(xml: &str) -> Result<String, JsValue> {
    let result = from_pnml(xml).map_err(|e| JsValue::from_str(&e))?;
    serde_json::to_string(&result)
        .map_err(|e| JsValue::from_str(&format!("Serialization error: {}", e)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn test_to_pnml_simple() {
        let pn = PetriNetResult {
            net: PetriNet {
                name: "Test Net".to_string(),
                places: vec![
                    Place { name: "p1".to_string() },
                    Place { name: "p2".to_string() },
                ],
                transitions: vec![
                    Transition {
                        name: "t1".to_string(),
                        label: Some("A".to_string()),
                        properties: HashMap::new(),
                    },
                ],
                arcs: vec![
                    Arc { source: "p1".to_string(), target: "t1".to_string(), weight: 1 },
                    Arc { source: "t1".to_string(), target: "p2".to_string(), weight: 1 },
                ],
            },
            initial_marking: vec![("p1".to_string(), 1)].into_iter().collect(),
            final_marking: vec![("p2".to_string(), 1)].into_iter().collect(),
        };

        let pnml = to_pnml(&pn);
        assert!(pnml.contains("<?xml"));
        assert!(pnml.contains("<net id=\"Test Net\""));
        assert!(pnml.contains("<place id=\"p1\">"));
        assert!(pnml.contains("<transition id=\"t1\""));
        assert!(pnml.contains("name=\"A\""));
        assert!(pnml.contains("<inscription>"));
    }

    #[test]
    fn test_from_pnml_simple() {
        let pnml = r#"<?xml version="1.0" encoding="UTF-8"?>
<pnml xmlns="http://www.pnml.org/version-2009/grammar">
  <net id="Simple" type="P/T-net">
    <place id="source">
      <graphics/>
    </place>
    <place id="sink">
      <graphics/>
    </place>
    <transition id="t1" name="A">
      <graphics/>
    </transition>
    <transition id="t2">
      <graphics/>
    </transition>
    <arc source="source" target="t1">
      <inscription><text>1</text></inscription>
      <graphics/>
    </arc>
    <arc source="t1" target="sink">
      <inscription><text>1</text></inscription>
      <graphics/>
    </arc>
    <initialmarking>
      <place idref="source"><text>1</text></place>
    </initialmarking>
    <finalmarking>
      <place idref="sink"><text>1</text></place>
    </finalmarking>
  </net>
</pnml>"#;

        let result = from_pnml(pnml).unwrap();
        assert_eq!(result.net.name, "Simple");
        assert_eq!(result.net.places.len(), 2);
        assert_eq!(result.net.transitions.len(), 2);
        assert_eq!(result.net.transitions[0].label, Some("A".to_string()));
        assert_eq!(result.net.transitions[1].label, None); // silent transition
        assert_eq!(result.net.arcs.len(), 2);
        assert_eq!(result.initial_marking.get("source"), Some(&1));
        assert_eq!(result.final_marking.get("sink"), Some(&1));
    }

    #[test]
    fn test_pnml_roundtrip() {
        let original = PetriNetResult {
            net: PetriNet {
                name: "RoundTrip".to_string(),
                places: vec![
                    Place { name: "p1".to_string() },
                    Place { name: "p2".to_string() },
                ],
                transitions: vec![
                    Transition {
                        name: "t1".to_string(),
                        label: Some("A".to_string()),
                        properties: HashMap::new(),
                    },
                ],
                arcs: vec![
                    Arc { source: "p1".to_string(), target: "t1".to_string(), weight: 1 },
                    Arc { source: "t1".to_string(), target: "p2".to_string(), weight: 1 },
                ],
            },
            initial_marking: vec![("p1".to_string(), 1)].into_iter().collect(),
            final_marking: vec![("p2".to_string(), 1)].into_iter().collect(),
        };

        let pnml = to_pnml(&original);
        let restored = from_pnml(&pnml).unwrap();

        assert_eq!(restored.net.name, original.net.name);
        assert_eq!(restored.net.places.len(), original.net.places.len());
        assert_eq!(restored.net.transitions.len(), original.net.transitions.len());
        assert_eq!(restored.net.arcs.len(), original.net.arcs.len());
        assert_eq!(restored.initial_marking, original.initial_marking);
        assert_eq!(restored.final_marking, original.final_marking);
    }
}
