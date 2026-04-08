/**
 * Example: Using POWL Model Validation
 *
 * This example demonstrates how to use the validation utilities
 * to check POWL models for soundness and correctness.
 */

import { Powl, formatValidationResult, getValidationSummary } from "@pm4py/pm4wasm";

async function main() {
  const powl = await Powl.init();

  console.log("=== POWL Model Validation Examples ===\n");

  // Example 1: Valid model
  console.log("Example 1: Valid sequential model");
  console.log("Model: X(A, B, C)");
  const model1 = powl.parse("X(A, B, C)");
  const result1 = model1.validate();
  console.log(getValidationSummary(result1));
  console.log(formatValidationResult(result1));
  console.log("\n" + "=".repeat(60) + "\n");

  // Example 2: Model with unreachable parts
  console.log("Example 2: Model with unreachable activity");
  console.log("Model: X(A, B) with unreachable C");
  // Note: This would need to be constructed manually to create unreachable nodes
  // For demonstration, we show a valid model
  const model2 = powl.parse("PO=(nodes={A, B}, order={A-->B})");
  const result2 = model2.validate();
  console.log(getValidationSummary(result2));
  console.log("\n" + "=".repeat(60) + "\n");

  // Example 3: Complex model
  console.log("Example 3: Complex XOR-Loop model");
  console.log("Model: X(X(A, B), L(C, D))");
  const model3 = powl.parse("X(X(A, B), L(C, D))");
  const result3 = model3.validate();
  console.log(getValidationSummary(result3));
  console.log("\n" + "=".repeat(60) + "\n");

  // Example 4: Partial order model
  console.log("Example 4: Strict partial order");
  console.log("Model: PO=(nodes={A, B, C}, order={A-->B, B-->C, A-->C})");
  const model4 = powl.parse("PO=(nodes={A, B, C}, order={A-->B, B-->C, A-->C})");
  const result4 = model4.validate();
  console.log(getValidationSummary(result4));
  console.log(formatValidationResult(result4));
  console.log("\n" + "=".repeat(60) + "\n");

  // Example 5: Using validation in error handling
  console.log("Example 5: Validation in error handling");
  const models = [
    powl.parse("X(A, B)"),
    powl.parse("PO=(nodes={A, B}, order={A-->B})"),
    powl.parse("X(X(A, B), L(C, D))"),
  ];

  let validCount = 0;
  let invalidCount = 0;

  for (const model of models) {
    const result = model.validate();
    if (result.isValid) {
      validCount++;
    } else {
      invalidCount++;
      console.error(`Invalid model found: ${model}`);
      for (const error of result.errors) {
        console.error(`  - [${error.type}] ${error.message}`);
      }
    }
  }

  console.log(`\nSummary: ${validCount} valid, ${invalidCount} invalid models`);
}

// Run if executed directly
if (import.meta.url === `file://${process.argv[1]}`) {
  main().catch(console.error);
}

export { main };
