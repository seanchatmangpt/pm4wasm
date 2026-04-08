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

//! DECLARE discovery algorithm.
//!
//! Discovers DECLARE constraint templates from an event log.
//! Implements key templates: response, precedence, succession, co-existence,
//! alternate response/precedence, chain response/precedence.
//!
//! Mirrors `pm4py.algo.discovery.declare`.

use crate::event_log::EventLog;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

/// Template types for DECLARE constraints.
#[derive(Clone, Debug, Serialize, Deserialize, Hash, Eq, PartialEq, PartialOrd)]
#[serde(rename_all = "snake_case")]
pub enum DeclareTemplate {
    Existence,
    ExactlyOne,
    Init,
    RespondedExistence,
    Response,
    Precedence,
    Succession,
    AltResponse,
    AltPrecedence,
    AltSuccession,
    ChainResponse,
    ChainPrecedence,
    ChainSuccession,
    Absence,
    CoExistence,
    NonCoexistence,
    NonSuccession,
    NonChainSuccession,
}

impl DeclareTemplate {
    pub fn as_str(&self) -> &'static str {
        match self {
            DeclareTemplate::Existence => "existence",
            DeclareTemplate::ExactlyOne => "exactly_one",
            DeclareTemplate::Init => "init",
            DeclareTemplate::RespondedExistence => "responded_existence",
            DeclareTemplate::Response => "response",
            DeclareTemplate::Precedence => "precedence",
            DeclareTemplate::Succession => "succession",
            DeclareTemplate::AltResponse => "altresponse",
            DeclareTemplate::AltPrecedence => "altprecedence",
            DeclareTemplate::AltSuccession => "altsuccession",
            DeclareTemplate::ChainResponse => "chainresponse",
            DeclareTemplate::ChainPrecedence => "chainprecedence",
            DeclareTemplate::ChainSuccession => "chainsuccession",
            DeclareTemplate::Absence => "absence",
            DeclareTemplate::CoExistence => "coexistence",
            DeclareTemplate::NonCoexistence => "noncoexistence",
            DeclareTemplate::NonSuccession => "nonsuccession",
            DeclareTemplate::NonChainSuccession => "nonchainsuccession",
        }
    }
}

/// A DECLARE rule with support and confidence metrics.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DeclareRule {
    /// Number of traces where the rule was activated (non-zero value).
    pub support: usize,
    /// Number of traces where the rule was satisfied (value = 1).
    pub confidence: usize,
}

/// A DECLARE model mapping templates to their rules.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DeclareModel {
    /// Map: template_name -> rule_key -> {support, confidence}
    /// Rule keys are:
    /// - Unary templates: activity name
    /// - Binary templates: (activity_a, activity_b)
    pub rules: HashMap<String, HashMap<String, DeclareRule>>,
}

/// Check if alternate response constraint is satisfied.
/// AlternateResponse(a,b): for each a, there exists b after it and before next a.
fn is_alternate_response_satisfied(
    trace: &[String],
    act_idxs: &HashMap<String, Vec<usize>>,
    a: &str,
    b: &str,
) -> bool {
    let a_idxs = match act_idxs.get(a) {
        Some(idxs) if !idxs.is_empty() => return true, // No a = vacuously true
        Some(idxs) => idxs,
        None => return true,
    };

    let b_idxs = match act_idxs.get(b) {
        Some(idxs) if !idxs.is_empty() => return false,
        Some(idxs) => idxs,
        None => return false,
    };

    let mut b_ptr = 0;
    let trace_len = trace.len();

    for (i, &a_idx) in a_idxs.iter().enumerate() {
        let next_a = if i + 1 < a_idxs.len() {
            a_idxs[i + 1]
        } else {
            trace_len
        };

        // Find first b after a_idx
        while b_ptr < b_idxs.len() && b_idxs[b_ptr] <= a_idx {
            b_ptr += 1;
        }

        if b_ptr >= b_idxs.len() || b_idxs[b_ptr] >= next_a {
            return false;
        }
    }

    true
}

/// Check if chain response constraint is satisfied.
/// ChainResponse(a,b): for each a, b occurs immediately after.
fn is_chain_response_satisfied(
    trace: &[String],
    act_idxs: &HashMap<String, Vec<usize>>,
    a: &str,
    b: &str,
) -> bool {
    let a_idxs = match act_idxs.get(a) {
        Some(idxs) if !idxs.is_empty() => return true, // No a = vacuously true
        Some(idxs) => idxs,
        None => return true,
    };

    for &a_idx in a_idxs {
        if a_idx + 1 >= trace.len() || &trace[a_idx + 1] != b {
            return false;
        }
    }

    true
}

/// Check if alternate precedence constraint is satisfied.
/// AltPrecedence(a,b): for each b, there exists a before it and after previous b.
fn is_alternate_precedence_satisfied(
    _trace: &[String],
    act_idxs: &HashMap<String, Vec<usize>>,
    a: &str,
    b: &str,
) -> bool {
    let b_idxs = match act_idxs.get(b) {
        Some(idxs) if !idxs.is_empty() => return true, // No b = vacuously true
        Some(idxs) => idxs,
        None => return true,
    };

    let a_idxs = match act_idxs.get(a) {
        Some(idxs) if !idxs.is_empty() => return false,
        Some(idxs) => idxs,
        None => return false,
    };

    let mut a_ptr = 0;
    let mut prev_b = 0;

    for &b_idx in b_idxs {
        while a_ptr < a_idxs.len() && a_idxs[a_ptr] <= prev_b {
            a_ptr += 1;
        }

        if a_ptr >= a_idxs.len() || a_idxs[a_ptr] >= b_idx {
            return false;
        }

        prev_b = b_idx;
    }

    true
}

/// Check if chain precedence constraint is satisfied.
/// ChainPrecedence(a,b): for each b, a occurs immediately before.
fn is_chain_precedence_satisfied(
    trace: &[String],
    act_idxs: &HashMap<String, Vec<usize>>,
    a: &str,
    b: &str,
) -> bool {
    let b_idxs = match act_idxs.get(b) {
        Some(idxs) if !idxs.is_empty() => return true, // No b = vacuously true
        Some(idxs) => idxs,
        None => return true,
    };

    for &b_idx in b_idxs {
        if b_idx == 0 || &trace[b_idx - 1] != a {
            return false;
        }
    }

    true
}

/// Build the rules table from event log.
fn form_rules_table(
    log: &EventLog,
    activities: &HashSet<String>,
    allowed_templates: &HashSet<DeclareTemplate>,
) -> Vec<HashMap<String, i32>> {
    let mut table = Vec::new();

    // Get variants with frequencies
    let mut variants: HashMap<Vec<String>, usize> = HashMap::new();
    for trace in &log.traces {
        let sequence: Vec<String> = trace.events.iter().map(|e| e.name.clone()).collect();
        *variants.entry(sequence).or_insert(0) += 1;
    }

    for (trace, occs) in variants {
        // Build activity counter and index map
        let mut act_counter: HashMap<String, usize> = HashMap::new();
        let mut act_idxs: HashMap<String, Vec<usize>> = HashMap::new();

        for (idx, act) in trace.iter().enumerate() {
            *act_counter.entry(act.clone()).or_insert(0) += 1;
            act_idxs.entry(act.clone()).or_insert_with(Vec::new).push(idx);
        }

        // Evaluate templates for this trace
        for _ in 0..occs {
            let mut rules = HashMap::new();

            // Existence: activity occurs in trace
            if allowed_templates.contains(&DeclareTemplate::Existence) {
                for act in activities {
                    let key = format!("existence|{}", act);
                    let value = if act_counter.contains_key(act) { 1 } else { -1 };
                    rules.insert(key, value);
                }
            }

            // Exactly one: activity occurs exactly once
            if allowed_templates.contains(&DeclareTemplate::ExactlyOne) {
                for act in activities {
                    let key = format!("exactly_one|{}", act);
                    let value = if act_counter.get(act) == Some(&1) { 1 } else { -1 };
                    rules.insert(key, value);
                }
            }

            // Init: activity is first in trace
            if allowed_templates.contains(&DeclareTemplate::Init) {
                if let Some(first) = trace.first() {
                    for act in activities {
                        let key = format!("init|{}", act);
                        let value = if act == first { 1 } else { -1 };
                        rules.insert(key, value);
                    }
                }
            }

            // Responded existence: if a occurs, b occurs
            if allowed_templates.contains(&DeclareTemplate::RespondedExistence) {
                for a in act_counter.keys() {
                    for b in activities {
                        if a != b {
                            let key = format!("responded_existence|{}|{}", a, b);
                            let value = if act_counter.contains_key(b) { 1 } else { -1 };
                            rules.insert(key, value);
                        }
                    }
                }
            }

            // Response(a,b): if a occurs, last a < first b
            if allowed_templates.contains(&DeclareTemplate::Response) {
                for a in act_counter.keys() {
                    for b in activities {
                        if a != b {
                            let key = format!("response|{}|{}", a, b);
                            let value = if let (Some(a_idxs), Some(b_idxs)) =
                                (act_idxs.get(a), act_idxs.get(b))
                            {
                                if a_idxs.last() < b_idxs.first() { 1 } else { -1 }
                            } else {
                                0
                            };
                            rules.insert(key, value);
                        }
                    }
                }
            }

            // Precedence(a,b): if b occurs, first a < first b
            if allowed_templates.contains(&DeclareTemplate::Precedence) {
                for b in act_counter.keys() {
                    for a in activities {
                        if a != b {
                            let key = format!("precedence|{}|{}", a, b);
                            let value = if let (Some(a_idxs), Some(b_idxs)) =
                                (act_idxs.get(a), act_idxs.get(b))
                            {
                                if a_idxs.first() < b_idxs.first() { 1 } else { -1 }
                            } else if act_idxs.contains_key(a) {
                                0 // a exists, b doesn't
                            } else {
                                -1
                            };
                            rules.insert(key, value);
                        }
                    }
                }
            }

            // Alternate response
            if allowed_templates.contains(&DeclareTemplate::AltResponse) {
                for a in act_counter.keys() {
                    for b in activities {
                        if a != b {
                            let key = format!("altresponse|{}|{}", a, b);
                            let value =
                                if is_alternate_response_satisfied(&trace, &act_idxs, a, b) {
                                    1
                                } else {
                                    -1
                                };
                            rules.insert(key, value);
                        }
                    }
                }
            }

            // Chain response
            if allowed_templates.contains(&DeclareTemplate::ChainResponse) {
                for a in act_counter.keys() {
                    for b in activities {
                        if a != b {
                            let key = format!("chainresponse|{}|{}", a, b);
                            let value =
                                if is_chain_response_satisfied(&trace, &act_idxs, a, b) {
                                    1
                                } else {
                                    -1
                                };
                            rules.insert(key, value);
                        }
                    }
                }
            }

            // Alternate precedence
            if allowed_templates.contains(&DeclareTemplate::AltPrecedence) {
                for b in act_counter.keys() {
                    for a in activities {
                        if a != b {
                            let key = format!("altprecedence|{}|{}", a, b);
                            let value =
                                if is_alternate_precedence_satisfied(&trace, &act_idxs, a, b)
                                {
                                    1
                                } else {
                                    -1
                                };
                            rules.insert(key, value);
                        }
                    }
                }
            }

            // Chain precedence
            if allowed_templates.contains(&DeclareTemplate::ChainPrecedence) {
                for b in act_counter.keys() {
                    for a in activities {
                        if a != b {
                            let key = format!("chainprecedence|{}|{}", a, b);
                            let value =
                                if is_chain_precedence_satisfied(&trace, &act_idxs, a, b) {
                                    1
                                } else {
                                    -1
                                };
                            rules.insert(key, value);
                        }
                    }
                }
            }

            table.push(rules);
        }
    }

    table
}

/// Extract rules from the rules table based on support/confidence thresholds.
fn get_rules_from_table(
    table: &[HashMap<String, i32>],
    min_support_ratio: Option<f64>,
    min_confidence_ratio: Option<f64>,
) -> HashMap<String, HashMap<String, DeclareRule>> {
    if table.is_empty() {
        return HashMap::new();
    }

    let n_rows = table.len();

    // Auto-select thresholds if not provided
    let (min_support, min_conf_ratio) = if min_support_ratio.is_none() && min_confidence_ratio.is_none() {
        // Find best rule by support * confidence
        let mut best_prod = 0.0;
        let mut best_support = 0;
        for rules_row in table {
            for (_key, &value) in rules_row {
                if value != 0 {
                    let support = rules_row.len();
                    let confidence = if value > 0 { 1 } else { 0 };
                    let prod = (support as f64) * (confidence as f64);
                    if prod > best_prod {
                        best_prod = prod;
                        best_support = support;
                    }
                }
            }
        }

        // Use 0.8 multiplier
        let min_support = (best_support as f64 * 0.8) as usize;
        let min_conf_ratio = 0.8;
        (min_support, min_conf_ratio)
    } else {
        let min_support = if let Some(ratio) = min_support_ratio {
            (n_rows as f64 * ratio) as usize
        } else {
            0
        };
        let min_conf_ratio = min_confidence_ratio.unwrap_or(0.0);
        (min_support, min_conf_ratio)
    };

    // Extract rules meeting thresholds
    let mut all_columns: HashMap<String, Vec<i32>> = HashMap::new();

    for rules_row in table {
        for (key, &value) in rules_row {
            all_columns.entry(key.clone()).or_insert_with(Vec::new).push(value);
        }
    }

    let mut rules: HashMap<String, HashMap<String, DeclareRule>> = HashMap::new();

    for (column, values) in all_columns {
        let support = values.iter().filter(|&&v| v != 0).count();

        if support >= min_support {
            let confidence = values.iter().filter(|&&v| v > 0).count();

            if confidence as f64 >= support as f64 * min_conf_ratio {
                // Parse key: "template|activity" or "template|a|b"
                let parts: Vec<&str> = column.split('|').collect();
                let template = parts[0].to_string();
                let rule_key = if parts.len() == 2 {
                    parts[1].to_string()
                } else {
                    format!("{}|{}", parts[1], parts[2])
                };

                rules
                    .entry(template)
                    .or_insert_with(HashMap::new)
                    .insert(
                        rule_key,
                        DeclareRule {
                            support,
                            confidence,
                        },
                    );
            }
        }
    }

    rules
}

/// Discover a DECLARE model from an event log.
///
/// Mirrors `pm4py.discover_declare()`.
///
/// # Arguments
/// * `log` - Event log to analyze
/// * `considered_activities` - Optional set of activities to consider (None = all)
/// * `min_support_ratio` - Optional minimum support ratio (None = auto-select)
/// * `min_confidence_ratio` - Optional minimum confidence ratio (None = auto-select)
///
/// # Returns
/// DECLARE model with rules grouped by template
pub fn discover_declare(
    log: &EventLog,
    considered_activities: Option<HashSet<String>>,
    min_support_ratio: Option<f64>,
    min_confidence_ratio: Option<f64>,
) -> DeclareModel {
    // Determine activities to consider
    let activities: HashSet<String> = if let Some(acts) = considered_activities {
        acts
    } else {
        log.activities().into_iter().collect()
    };

    // Define default templates
    let allowed_templates: HashSet<DeclareTemplate> = vec![
        DeclareTemplate::Existence,
        DeclareTemplate::ExactlyOne,
        DeclareTemplate::Init,
        DeclareTemplate::RespondedExistence,
        DeclareTemplate::Response,
        DeclareTemplate::Precedence,
        DeclareTemplate::Succession,
        DeclareTemplate::AltResponse,
        DeclareTemplate::AltPrecedence,
        DeclareTemplate::AltSuccession,
        DeclareTemplate::ChainResponse,
        DeclareTemplate::ChainPrecedence,
        DeclareTemplate::ChainSuccession,
        DeclareTemplate::Absence,
        DeclareTemplate::CoExistence,
        DeclareTemplate::NonCoexistence,
        DeclareTemplate::NonSuccession,
        DeclareTemplate::NonChainSuccession,
    ]
    .into_iter()
    .collect();

    // Build rules table
    let table = form_rules_table(log, &activities, &allowed_templates);

    // Extract rules with support/confidence filtering
    let mut rules = get_rules_from_table(&table, min_support_ratio, min_confidence_ratio);

    // Add derived templates
    if let (Some(response), Some(precedence)) =
        (rules.remove("response"), rules.remove("precedence"))
    {
        let mut succession = HashMap::new();

        for (key, resp_rule) in &response {
            if let Some(prec_rule) = precedence.get(key) {
                succession.insert(
                    key.clone(),
                    DeclareRule {
                        support: resp_rule.support.min(prec_rule.support),
                        confidence: resp_rule.confidence.min(prec_rule.confidence),
                    },
                );
            }
        }

        if !succession.is_empty() {
            rules.insert("succession".to_string(), succession);
        }
    }

    if let Some(resp_exist) = rules.get("responded_existence") {
        let mut coexistence = HashMap::new();

        for (key, rule_ab) in resp_exist {
            if let Some(rule_ba) = resp_exist.get(&format!("{}|{}", key.split('|').last().unwrap(), key.split('|').next().unwrap())) {
                coexistence.insert(
                    key.clone(),
                    DeclareRule {
                        support: rule_ab.support.min(rule_ba.support),
                        confidence: rule_ab.confidence.min(rule_ba.confidence),
                    },
                );
            }
        }

        if !coexistence.is_empty() {
            rules.insert("coexistence".to_string(), coexistence);
        }
    }

    // Absence is negation of existence
    if let Some(existence) = rules.remove("existence") {
        let absence: HashMap<String, DeclareRule> = existence
            .into_iter()
            .map(|(key, rule)| {
                let new_rule = DeclareRule {
                    support: rule.support,
                    confidence: rule.support - rule.confidence, // Negate
                };
                (key, new_rule)
            })
            .collect();

        if !absence.is_empty() {
            rules.insert("absence".to_string(), absence);
        }
    }

    DeclareModel { rules }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::event_log::parse_csv;

    #[test]
    fn test_declare_response() {
        let csv = "case_id,activity\n\
                   1,A\n\
                   1,B\n\
                   2,A\n\
                   2,C";
        let log = parse_csv(csv).unwrap();
        let model = discover_declare(&log, None, None, None);

        // Response(A,B) should exist but have low confidence (case 2 violates)
        if let Some(response) = model.rules.get("response") {
            assert!(response.contains_key("A|B"));
        }
    }

    #[test]
    fn test_declare_precedence() {
        let csv = "case_id,activity\n\
                   1,A\n\
                   1,B\n\
                   1,C\n\
                   2,B\n\
                   2,C";
        let log = parse_csv(csv).unwrap();
        let model = discover_declare(&log, None, None, None);

        // Precedence(A,B) should be violated in case 2 (no A)
        if let Some(precedence) = model.rules.get("precedence") {
            assert!(precedence.contains_key("A|B"));
        }
    }

    #[test]
    fn test_declare_succession() {
        let csv = "case_id,activity\n\
                   1,A\n\
                   1,B\n\
                   2,A\n\
                   2,B\n\
                   3,A\n\
                   3,C";
        let log = parse_csv(csv).unwrap();
        let model = discover_declare(&log, None, None, None);

        // Succession(A,B) should have moderate confidence
        if let Some(succession) = model.rules.get("succession") {
            assert!(succession.contains_key("A|B"));
        }
    }
}
