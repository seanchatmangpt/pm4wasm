// pm4wasm LLM Module
//
// This module provides the bridge between LLM API calls (handled in JavaScript)
// and POWL operations (handled in WASM). The LLM pipeline is split:
//
// 1. JavaScript layer: Handles LLM API calls (Groq, OpenAI, etc.)
// 2. WASM layer: Handles POWL parsing, validation, simplification, conversion
//
// This architecture enables the entire "Describe workflow → get executable BPMN"
// pipeline to work in the browser with data staying client-side.

pub mod judge;
pub mod demos;
pub mod codegen;

use crate::powl::PowlArena;
use crate::parser::parse_powl_model_string;

pub use codegen::{CodegenResult, CodeFormat, CodeTarget, generate_code};

/// Result of natural language to POWL generation
///
/// This is the WASM side of the pipeline. The actual LLM API call happens
/// in JavaScript (see js/src/llm.ts), which then calls this to validate
/// and process the POWL model.
#[derive(Debug)]
pub struct NLToPOWLResult {
    /// The POWL model string (parseable by parse_powl)
    pub powl: String,
    /// Whether the judge approved this model
    pub verdict: bool,
    /// Judge's reasoning (if verdict is false)
    pub reasoning: String,
    /// Number of refinement iterations
    pub refinements: u32,
}

/// Validate a POWL model string against structural soundness criteria
///
/// Uses POWLJudge to check:
/// - Deadlock freedom (no circular waits)
/// - Liveness (all actions eventually complete)
/// - Boundedness (no unbounded resource consumption)
///
/// Returns a tuple of (verdict: bool, reasoning: String)
pub fn validate_powl_structure(model_str: &str) -> (bool, String) {
    let mut arena = PowlArena::new();
    match parse_powl_model_string(model_str, &mut arena) {
        Ok(root) => {
            // Check for soundness using the judge
            let validation = judge::validate_soundness(&arena, root);
            (validation.is_sound, validation.reasoning)
        }
        Err(e) => (false, format!("Parse error: {}", e)),
    }
}

/// Extract few-shot demos for a specific domain
///
/// Returns JSON array of few-shot examples for the given domain.
/// Used by the JavaScript LLM layer to construct prompts.
pub fn get_demos_for_domain(domain: &str) -> String {
    match domain {
        "loan_approval" | "finance" => demos::get_loan_approval_demos(),
        "software_release" | "it" | "devops" => demos::get_software_release_demos(),
        "ecommerce" | "retail" => demos::get_ecommerce_demos(),
        "manufacturing" | "production" => demos::get_manufacturing_demos(),
        "healthcare" | "medical" => demos::get_healthcare_demos(),
        _ => demos::get_general_demos(),
    }
}

/// Judge validation result
#[derive(Debug)]
pub struct JudgeValidation {
    pub is_sound: bool,
    pub reasoning: String,
    pub violations: Vec<String>,
}

impl JudgeValidation {
    pub fn approved() -> Self {
        JudgeValidation {
            is_sound: true,
            reasoning: "Model is structurally sound".to_string(),
            violations: Vec::new(),
        }
    }

    pub fn rejected(reason: &str) -> Self {
        JudgeValidation {
            is_sound: false,
            reasoning: reason.to_string(),
            violations: vec![reason.to_string()],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_simple_model() {
        let (verdict, reasoning) = validate_powl_structure("A");
        assert!(verdict, "Simple model should be valid");
        assert!(!reasoning.is_empty());
    }

    #[test]
    fn test_validate_parallel_model() {
        let (verdict, _) = validate_powl_structure("PO=(nodes={A, B}, order={})");
        assert!(verdict, "Parallel model should be valid");
    }

    #[test]
    fn test_get_demos() {
        let demos = get_demos_for_domain("finance");
        assert!(!demos.is_empty(), "Should return demos JSON");

        let demos = get_demos_for_domain("unknown");
        assert!(!demos.is_empty(), "Should return general demos");
    }
}
