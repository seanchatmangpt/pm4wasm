/**
 * Natural Language to POWL Example using Vercel AI SDK
 *
 * This example demonstrates how to use pm4wasm with the Vercel AI SDK
 * to convert natural language descriptions to POWL models.
 *
 * Prerequisites:
 * 1. Set up environment variables:
 *    - GROQ_API_KEY for Groq (default, fast & free)
 *    - OPENAI_API_KEY for OpenAI
 *    - ANTHROPIC_API_KEY for Anthropic Claude
 *
 * 2. Install dependencies:
 *    npm install
 *
 * 3. Build WASM:
 *    npm run build:wasm
 */

import { Powl } from "@pm4py/pm4wasm";

async function main() {
  // Initialize the POWL library
  const powl = await Powl.init();

  console.log("=== Natural Language to POWL Examples ===\n");

  // Example 1: Simple sequence with Groq (default, fast & free)
  console.log("1. Simple Order Fulfillment (Groq):");
  const model1 = await powl.fromNaturalLanguage(
    "A customer places an order, the system confirms it, " +
    "payment is processed, and if successful the order is shipped",
    {
      provider: "groq",
      apiKey: process.env.GROQ_API_KEY,
    },
    "ecommerce"
  );
  console.log("POWL:", model1.toString());
  console.log();

  // Example 2: Complex workflow with OpenAI
  console.log("2. Loan Approval with Risk Assessment (OpenAI):");
  const model2 = await powl.fromNaturalLanguage(
    "Customer submits loan application, system validates it, " +
    "performs risk assessment, if high risk then reject, " +
    "if low risk then approve and process payment",
    {
      provider: "openai",
      apiKey: process.env.OPENAI_API_KEY,
      model: "gpt-4o",
    },
    "loan_approval"
  );
  console.log("POWL:", model2.toString());
  console.log();

  // Example 3: Generate code directly
  console.log("3. Generate n8n Workflow from Natural Language:");
  const n8nWorkflow = await powl.naturalLanguageToCode(
    "CI/CD pipeline: code is pushed, build runs, if build fails notify developer, " +
    "if build succeeds run tests, if tests pass deploy to staging",
    "n8n",
    {
      provider: "groq",
      apiKey: process.env.GROQ_API_KEY,
    },
    "software_release"
  );
  console.log("n8n Workflow:", n8nWorkflow.substring(0, 200) + "...");
  console.log();

  // Example 4: Generate BPMN
  console.log("4. Generate BPMN from Natural Language:");
  const model4 = await powl.fromNaturalLanguage(
    "Patient admission: patient arrives, registration completed, " +
    "triage assesses severity, emergency goes to ER immediately, " +
    "non-emergency waits for consultation",
    {
      provider: "anthropic",
      apiKey: process.env.ANTHROPIC_API_KEY,
    },
    "healthcare"
  );
  const bpmn = powl.toBpmn(model4.toString());
  console.log("BPMN:", bpmn.substring(0, 300) + "...");
  console.log();

  // Example 5: Validation with refinement
  console.log("5. Validation with Automatic Refinement:");
  const validation = powl.validatePowlStructure(model1.toString());
  console.log("Valid:", validation.verdict);
  console.log("Reasoning:", validation.reasoning);
  console.log();

  // Example 6: Compare multiple providers
  console.log("6. Provider Comparison:");
  const description = "Software release with build, test, and deploy stages";

  for (const provider of ["groq", "openai"] as const) {
    const apiKey =
      provider === "groq"
        ? process.env.GROQ_API_KEY
        : process.env.OPENAI_API_KEY;

    if (!apiKey) continue;

    const start = Date.now();
    const model = await powl.fromNaturalLanguage(description, {
      provider,
      apiKey,
    });
    const duration = Date.now() - start;

    console.log(`${provider}: ${model.toString()} (${duration}ms)`);
  }
}

// Run the examples
main().catch(console.error);
