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

/// Recursive-descent parser for POWL model strings.
///
/// Mirrors `pm4py/objects/powl/parser.py:parse_powl_model_string()`.
///
/// Grammar (informally):
///   powl  ::= partial_order | xor | loop | tau | transition
///   partial_order ::= "PO=(nodes={" nodes "}, order={" edges "})"
///                   | "PO(nodes={" nodes "}, order={" edges "})"
///   xor   ::= "X (" powl ("," powl)* ")"
///   loop  ::= "* (" powl ("," powl)* ")"
///   tau   ::= "tau"
///   transition ::= label     (any string not matching above)
///
/// The Python implementation first calls `hie_utils.indent_representation`
/// which tokenizes by unrolling nested braces/parens into an indented list.
/// Here we implement an equivalent tokeniser inline.
use crate::powl::{Operator, PowlArena};

// ─── Tokeniser ────────────────────────────────────────────────────────────────

/// Break the POWL string into a flat list of tokens by tracking bracket depth.
/// Each "top-level" comma-separated item is emitted as one token even if it
/// contains nested parens/braces.
fn tokenize(s: &str) -> Vec<String> {
    let mut tokens: Vec<String> = Vec::new();
    let mut depth = 0usize;
    let mut cur = String::new();
    for ch in s.chars() {
        match ch {
            '(' | '{' => {
                depth += 1;
                cur.push(ch);
            }
            ')' | '}' => {
                if depth > 0 {
                    depth -= 1;
                }
                cur.push(ch);
            }
            ',' if depth == 0 => {
                let tok = cur.trim().to_string();
                if !tok.is_empty() {
                    tokens.push(tok);
                }
                cur.clear();
            }
            _ => {
                cur.push(ch);
            }
        }
    }
    let tok = cur.trim().to_string();
    if !tok.is_empty() {
        tokens.push(tok);
    }
    tokens
}

// ─── Parser ───────────────────────────────────────────────────────────────────

/// Parse a POWL model string and return the arena + root index.
///
/// # Errors
/// Returns a descriptive error string on parse failure.
pub fn parse_powl_model_string(
    s: &str,
    arena: &mut PowlArena,
) -> Result<u32, String> {
    let s = s
        .replace('\n', "")
        .replace('\r', "")
        .replace('\t', "")
        .trim()
        .to_string();

    if s.is_empty() {
        return Err("empty POWL string".to_string());
    }

    // ── Partial order: PO=(nodes={…}, order={…}) or PO(nodes={…}, order={…}) ──
    if s.starts_with("PO=") || s.starts_with("PO(") {
        return parse_partial_order(&s, arena);
    }

    // ── Exclusive choice: X ( child, child, … ) ──
    if s.starts_with("X (") || s.starts_with("X(") {
        return parse_operator(&s, "X", Operator::Xor, arena);
    }

    // ── Loop: * ( do, redo ) ──
    if s.starts_with("* (") || s.starts_with("*(") {
        return parse_operator(&s, "*", Operator::Loop, arena);
    }

    // ── Silent transition ──
    if s == "tau" {
        let idx = arena.add_silent_transition();
        return Ok(idx);
    }

    // ── Labeled transition ──
    let label = s.trim_matches('\'').to_string();
    Ok(arena.add_transition(Some(label)))
}

// ─── Partial order parsing ────────────────────────────────────────────────────

fn parse_partial_order(s: &str, arena: &mut PowlArena) -> Result<u32, String> {
    // Remove outer PO=(...) / PO=(nodes={…}, order={…})
    // Find "nodes={" and extract node list, then "order={" and extract edge list.
    let nodes_str = extract_braced_content(s, "nodes={")?;
    let order_str = extract_braced_content(s, "order={")?;

    // Parse children
    let node_tokens: Vec<String> = if nodes_str.trim().is_empty() {
        Vec::new()
    } else {
        tokenize(nodes_str.trim())
    };

    // Map each token string → (arena_idx, label_for_edge_lookup)
    let mut child_indices: Vec<u32> = Vec::new();
    let mut token_to_local: Vec<(String, u32)> = Vec::new(); // (token_str, local_idx)

    for tok in &node_tokens {
        let child_idx = parse_powl_model_string(tok, arena)?;
        let local = child_indices.len() as u32;
        child_indices.push(child_idx);
        token_to_local.push((tok.clone(), local));
    }

    // Create SPO node
    let spo_idx = arena.add_strict_partial_order(child_indices.clone());

    // Parse edges: each edge is "SRC-->TGT"
    if !order_str.trim().is_empty() {
        let edge_tokens: Vec<String> = tokenize(order_str.trim());
        for edge_tok in &edge_tokens {
            if let Some(arrow_pos) = edge_tok.find("-->") {
                let src_str = edge_tok[..arrow_pos].trim();
                let tgt_str = edge_tok[arrow_pos + 3..].trim();

                let src_local = token_to_local
                    .iter()
                    .position(|(t, _)| node_label_matches(t, src_str))
                    .ok_or_else(|| format!("edge source '{}' not found in nodes", src_str))?;
                let tgt_local = token_to_local
                    .iter()
                    .position(|(t, _)| node_label_matches(t, tgt_str))
                    .ok_or_else(|| format!("edge target '{}' not found in nodes", tgt_str))?;

                arena.add_order_edge(spo_idx, src_local, tgt_local);
            }
        }
    }

    Ok(spo_idx)
}

/// True if a node token (full POWL string) matches the short label used in an
/// edge declaration.  Simple tokens are compared directly; complex nodes match
/// their full repr.
fn node_label_matches(token: &str, label: &str) -> bool {
    token.trim() == label.trim()
        || token.trim().trim_matches('\'') == label.trim()
}

/// Extract the content between a specific `key` and its matching `}`.
/// e.g. `extract_braced_content("PO=(nodes={A,B}, order={})", "nodes={")`
/// returns `"A,B"`.
fn extract_braced_content<'a>(s: &'a str, key: &str) -> Result<&'a str, String> {
    let start = s
        .find(key)
        .ok_or_else(|| format!("'{}' not found in '{}'", key, s))?;
    let content_start = start + key.len();
    // Find the matching closing brace
    let rest = &s[content_start..];
    let mut depth = 1usize;
    let mut end = 0usize;
    for (i, ch) in rest.char_indices() {
        match ch {
            '{' | '(' => depth += 1,
            '}' | ')' => {
                depth -= 1;
                if depth == 0 {
                    end = i;
                    break;
                }
            }
            _ => {}
        }
    }
    Ok(&rest[..end])
}

// ─── Operator parsing ─────────────────────────────────────────────────────────

fn parse_operator(
    s: &str,
    prefix: &str,
    op: Operator,
    arena: &mut PowlArena,
) -> Result<u32, String> {
    // Strip prefix and outer parens: "X ( A, B )" → "A, B"
    let after_prefix = s[prefix.len()..].trim();
    let inner = strip_outer_parens(after_prefix)
        .ok_or_else(|| format!("malformed operator expression: '{}'", s))?;

    let child_tokens = tokenize(inner.trim());
    if child_tokens.is_empty() {
        return Err(format!("operator '{}' has no children", prefix));
    }

    let mut children: Vec<u32> = Vec::new();
    for tok in &child_tokens {
        let child_idx = parse_powl_model_string(tok, arena)?;
        children.push(child_idx);
    }

    // Validate arity
    match op {
        Operator::Xor if children.len() < 2 => {
            return Err("XOR requires at least 2 children".to_string());
        }
        Operator::Loop if children.len() != 2 => {
            return Err("LOOP requires exactly 2 children".to_string());
        }
        _ => {}
    }

    Ok(arena.add_operator(op, children))
}

/// Given a string that starts with `(` or ` (`, return the content between
/// the outer parens.  Returns `None` if no parens are found.
fn strip_outer_parens(s: &str) -> Option<&str> {
    let s = s.trim_start();
    if !s.starts_with('(') {
        return None;
    }
    let inner = &s[1..];
    // Find matching closing paren
    let mut depth = 1usize;
    for (i, ch) in inner.char_indices() {
        match ch {
            '(' | '{' => depth += 1,
            ')' | '}' => {
                depth -= 1;
                if depth == 0 {
                    return Some(&inner[..i]);
                }
            }
            _ => {}
        }
    }
    None
}

// ─── tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(s: &str) -> (PowlArena, u32) {
        let mut arena = PowlArena::new();
        let root = parse_powl_model_string(s, &mut arena).expect("parse failed");
        (arena, root)
    }

    #[test]
    fn parse_transition() {
        let (arena, root) = parse("A");
        assert_eq!(arena.to_repr(root), "A");
    }

    #[test]
    fn parse_silent() {
        let (arena, root) = parse("tau");
        assert_eq!(arena.to_repr(root), "tau");
    }

    #[test]
    fn parse_xor() {
        let (arena, root) = parse("X ( A, B )");
        assert_eq!(arena.to_repr(root), "X ( A, B )");
    }

    #[test]
    fn parse_loop() {
        let (arena, root) = parse("* ( A, tau )");
        assert_eq!(arena.to_repr(root), "* ( A, tau )");
    }

    #[test]
    fn parse_partial_order_no_edges() {
        let (arena, root) = parse("PO=(nodes={A, B}, order={})");
        assert_eq!(arena.to_repr(root), "PO=(nodes={A, B}, order={})");
    }

    #[test]
    fn parse_partial_order_with_edge() {
        let (arena, root) = parse("PO=(nodes={NODE1, NODE2}, order={NODE1-->NODE2})");
        assert_eq!(
            arena.to_repr(root),
            "PO=(nodes={NODE1, NODE2}, order={NODE1-->NODE2})"
        );
    }

    #[test]
    fn parse_nested() {
        let s = "PO=(nodes={A, X ( B, C )}, order={A-->X ( B, C )})";
        let (arena, root) = parse(s);
        // Should produce a PO with two children (A and X(B,C)), one edge
        let repr = arena.to_repr(root);
        assert!(repr.contains("A-->"));
        assert!(repr.contains("X ( B, C )"));
    }

    #[test]
    fn docstring_example() {
        // From parser.py docstring
        let s = "PO=(nodes={ NODE1, NODE2, NODE3 }, order={ NODE1-->NODE2 })";
        let (arena, root) = parse(s);
        assert!(arena.validate_partial_orders(root).is_ok());
        let repr = arena.to_repr(root);
        assert!(repr.contains("NODE1-->NODE2"));
    }
}
