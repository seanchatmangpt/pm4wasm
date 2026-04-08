/**
 * Interactive Refinement Loop Demo
 *
 * This demo shows how to use the ProcessModelingService to:
 * 1. Generate an initial process model from a description
 * 2. Collect user feedback
 * 3. Refine the model iteratively based on feedback
 *
 * This implements the feedback loop described in the paper:
 * "Process Modeling With Large Language Models" (Kourani et al., 2024)
 */

import { ProcessModelingService, UserFeedback } from "../src/process-modeling-service.js";
import { Powl } from "../src/index.js";

/**
 * Mock LLM call function - Replace with actual LLM API call
 *
 * In production, this would call OpenAI, Anthropic, or another LLM provider.
 * For demo purposes, we simulate responses.
 */
async function mockLLMCall(prompt: string): Promise<string> {
  console.log("\n=== LLM Prompt ===");
  console.log(prompt.substring(0, 500) + "...");
  console.log("==================\n");

  // Simulate API delay
  await new Promise(resolve => setTimeout(resolve, 1000));

  // Return mock responses based on prompt content
  if (prompt.includes("loan application")) {
    return `
I'll generate a POWL model for the loan application process.

\`\`\`python
gen = ModelGenerator()

# Activities
submit = gen.activity("Submit application")
credit_check = gen.activity("Credit check")
employment_check = gen.activity("Employment verification")
approve = gen.activity("Approve loan")
reject = gen.activity("Reject loan")

# Parallel checks using partial order
checks = gen.partial_order(dependencies=[
  (submit, credit_check),
  (submit, employment_check),
  (credit_check, approve),
  (employment_check, approve),
  (credit_check, reject),
  (employment_check, reject)
])

# XOR for approve/reject decision
decision = gen.xor(approve, reject)

# Complete model
model = gen.sequence(submit, checks, decision)
\`\`\`

This model represents a loan application process with parallel credit and employment checks, followed by an approval decision.`;
  }

  if (prompt.includes("missing")) {
    return `
I see - we need to add a document review step before the credit check.

\`\`\`python
gen = ModelGenerator()

# Activities
submit = gen.activity("Submit application")
doc_review = gen.activity("Document review")
credit_check = gen.activity("Credit check")
employment_check = gen.activity("Employment verification")
approve = gen.activity("Approve loan")
reject = gen.activity("Reject loan")

# Parallel checks using partial order
checks = gen.partial_order(dependencies=[
  (doc_review, credit_check),
  (doc_review, employment_check),
  (credit_check, approve),
  (employment_check, approve),
  (credit_check, reject),
  (employment_check, reject)
])

# XOR for approve/reject decision
decision = gen.xor(approve, reject)

# Complete model with document review
model = gen.sequence(submit, doc_review, checks, decision)
\`\`\`

Added document review step after submission and before the parallel checks.`;
  }

  // Default response
  return `
\`\`\`python
gen = ModelGenerator()
model = gen.sequence(
  gen.activity("Start"),
  gen.activity("Process"),
  gen.activity("End")
)
\`\`\`
`;
}

/**
 * Demo: Complete refinement workflow
 */
async function demoRefinementWorkflow() {
  console.log("╔════════════════════════════════════════════════════════════╗");
  console.log("║  Process Model Refinement Loop - Interactive Demo          ║");
  console.log("╚════════════════════════════════════════════════════════════╝\n");

  // Initialize POWL
  console.log("🔧 Initializing POWL WASM module...");
  const powl = await Powl.init();
  console.log("✅ POWL initialized\n");

  // Create service
  const service = new ProcessModelingService(mockLLMCall, {
    maxRefinementIterations: 5,
    autoFixErrors: true,
  });

  // Step 1: Generate initial model
  console.log("📝 Step 1: Generate initial model");
  console.log("─────────────────────────────────────────");
  const description = `
A loan application process:
1. Customer submits application
2. Bank performs credit check and employment verification in parallel
3. Based on the results, the loan is either approved or rejected
`;

  const result = await service.generateModel(description);

  if (result.model) {
    console.log("✅ Model generated successfully!");
    console.log("   Iterations:", result.iterations);
    console.log("   Model:", result.model.toString());
    console.log("");

    // Step 2: Start refinement session
    console.log("🔄 Step 2: Start refinement session");
    console.log("─────────────────────────────────────────");
    service.startRefinementSession(description, result.model);
    console.log("✅ Refinement session started\n");

    // Step 3: Collect user feedback
    console.log("💬 Step 3: Collect user feedback");
    console.log("─────────────────────────────────────────");

    const feedback1: UserFeedback = {
      type: "text",
      content: "The model is missing a document review step that happens after submission",
      timestamp: new Date(),
    };

    console.log("Feedback 1:", feedback1.content);
    console.log("");

    // Step 4: Refine based on feedback
    console.log("🔧 Step 4: Refine model based on feedback");
    console.log("─────────────────────────────────────────");

    const refinement1 = await service.addFeedbackAndRefine(feedback1);

    if (refinement1.success && refinement1.model) {
      console.log("✅ Model refined successfully!");
      console.log("   Iterations:", refinement1.iterations);
      console.log("   Feedback incorporated:", refinement1.feedbackIncorporated);
      console.log("   Updated model:", refinement1.model.toString());
      console.log("");

      // Step 5: More feedback
      console.log("💬 Step 5: Additional feedback");
      console.log("─────────────────────────────────────────");

      const feedback2: UserFeedback = {
        type: "text",
        content: "We need to add a loop for resubmission if documents are incomplete",
        timestamp: new Date(),
      };

      console.log("Feedback 2:", feedback2.content);
      console.log("");

      const refinement2 = await service.addFeedbackAndRefine(feedback2);

      if (refinement2.success && refinement2.model) {
        console.log("✅ Model refined successfully!");
        console.log("   Iterations:", refinement2.iterations);
        console.log("   Feedback incorporated:", refinement2.feedbackIncorporated);
        console.log("   Final model:", refinement2.model.toString());
        console.log("");
      }
    }

    // Step 6: Generate summary report
    console.log("📊 Step 6: Refinement summary");
    console.log("─────────────────────────────────────────");
    console.log(service.generateRefinementReport());
  } else {
    console.log("❌ Model generation failed:");
    console.log("   Errors:", result.errors);
    console.log("   Warnings:", result.warnings);
  }

  console.log("\n✨ Demo complete!");
}

/**
 * Demo: Error handling workflow
 */
async function demoErrorHandling() {
  console.log("\n╔════════════════════════════════════════════════════════════╗");
  console.log("║  Error Handling Demo                                        ║");
  console.log("╚════════════════════════════════════════════════════════════╝\n");

  const service = new ProcessModelingService(mockLLMCall);

  const description = "A simple process with A, B, C in sequence";

  console.log("📝 Generating model with potential errors...");
  const result = await service.generateModel(description);

  if (result.model) {
    console.log("✅ Model generated despite errors!");
    console.log("   Iterations:", result.iterations);
    console.log("   Model:", result.model.toString());
  } else {
    console.log("❌ Model generation failed:");
    result.errors.forEach(err => console.log("   -", err));
  }
}

/**
 * Demo: Conformance checking
 */
async function demoConformanceChecking() {
  console.log("\n╔════════════════════════════════════════════════════════════╗");
  console.log("║  Conformance Checking Demo                                  ║");
  console.log("╚════════════════════════════════════════════════════════════╝\n");

  // Initialize POWL
  const powl = await Powl.init();

  // Create a simple model
  const model = powl.parse("→(A, B, C)");

  // Create an event log
  const log = `case_id,activity,timestamp
1,A,2024-01-01T10:00:00Z
1,B,2024-01-01T10:05:00Z
1,C,2024-01-01T10:10:00Z
2,A,2024-01-02T11:00:00Z
2,B,2024-01-02T11:05:00Z
2,C,2024-01-02T11:10:00Z
`;

  const service = new ProcessModelingService(mockLLMCall);

  console.log("📊 Validating model against event log...");
  console.log("   Model:", model.toString());
  console.log("");

  try {
    const validation = await service.validateModel(model, log);
    console.log("✅ Validation complete!");
    console.log("   Fitness:", (validation.percentage * 100).toFixed(1) + "%");
    console.log("   Avg trace fitness:", (validation.avgTraceFitness * 100).toFixed(1) + "%");
  } catch (error) {
    console.log("❌ Validation failed:", error);
  }
}

/**
 * Run all demos
 */
async function runDemos() {
  try {
    await demoRefinementWorkflow();
    await demoErrorHandling();
    await demoConformanceChecking();
  } catch (error) {
    console.error("\n❌ Demo failed:", error);
  }
}

// Run demos if this file is executed directly
if (import.meta.url === `file://${process.argv[1]}`) {
  runDemos();
}

export {
  demoRefinementWorkflow,
  demoErrorHandling,
  demoConformanceChecking,
  mockLLMCall,
};
