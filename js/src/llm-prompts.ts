/**
 * LLM Prompting Framework for POWL Model Generation
 *
 * Implementation of prompting strategies from:
 * "Process Modeling With Large Language Models" (Kourani et al., 2024)
 *
 * Four key strategies:
 * 1. Role Prompting - Assign LLM the role of process modeling expert
 * 2. Knowledge Injection - Provide POWL language knowledge
 * 3. Few-Shot Learning - Provide input/output examples
 * 4. Negative Prompting - Specify what to avoid
 */

/**
 * Message format for LLM conversation history
 */
export interface LLMMessage {
  role: "system" | "user" | "assistant";
  content: string;
}

/**
 * Prompt templates for LLM-based POWL generation
 */
export const PROMPT_TEMPLATES = {
  /**
   * System prompt combining role assignment + knowledge injection
   *
   * Strategy 1 (Role) + Strategy 2 (Knowledge Injection)
   */
  SYSTEM_PROMPT: `You are an expert in process modeling and Business Process Management (BPM).
Your task is to construct process models using POWL (Partially Ordered Workflow Language).

POWL Construction Functions:
- activity(label) - generates an activity node with the given label
- xor(...args) - choice operator (n >= 2 arguments) for complete exclusive paths
- loop(do, redo) - loop operator with mandatory body and optional redo part
- partial_order(dependencies) - strict partial order over activities
- sequence(...args) - sequential composition of activities

Key Rules:
- All partial orders must be IRREFLEXIVE (no self-loops: A→A is invalid)
- All partial orders must be TRANSITIVE (if A→B and B→C, then A→C must be explicitly stated)
- Use xor() for complete mutually exclusive paths, not local choices
- Call model.copy() before reusing sub-models in multiple places
- A loop with redo=None is equivalent to a repeat-until loop
- Partial orders use the format: [(A, B), (B, C)] meaning A precedes B, B precedes C`,

  /**
   * Few-shot examples from the Kourani et al. paper
   *
   * Strategy 3: Few-Shot Learning
   */
  EXAMPLES: `Example 1: Bicycle Manufacturing (Simple)
Process: A small company manufactures customized bicycles. First, an order is created.
Then the company checks if all required parts are available. If not, the order is rejected.
If yes, the company checks if a part is in stock. If yes, the part is reserved.
If not, the part is back-ordered. Finally, the order is accepted.

Model:
\`\`\`python
gen = ModelGenerator()
create = gen.activity("Create process")
reject = gen.activity("Reject order")
accept = gen.activity("Accept order")
check = gen.activity("Check part")
reserve = gen.activity("Reserve part")
backorder = gen.activity("Back-order part")

# Choice between reserve and backorder
check_reserve = gen.xor(reserve, backorder)

# Loop: check part, then either reserve or backorder, repeat if needed
part_loop = gen.loop(do=check, redo=check_reserve)

# Sequential flow: create -> part_loop -> accept (with reject as exception)
model = gen.sequence(create, part_loop, accept)
\`\`\`

Example 2: Loan Application (Parallel)
Process: A loan application is received. Two checks happen in parallel:
credit check and employment verification. If both pass, the loan is approved.
If either fails, the loan is rejected.

Model:
\`\`\`python
gen = ModelGenerator()
receive = gen.activity("Receive application")
credit_check = gen.activity("Credit check")
employment_check = gen.activity("Employment verification")
approve = gen.activity("Approve loan")
reject = gen.activity("Reject loan")

# Parallel checks
checks = gen.partial_order(dependencies=[
  (receive, credit_check),
  (receive, employment_check),
  (credit_check, approve),
  (employment_check, approve),
  (credit_check, reject),
  (employment_check, reject)
])

# XOR for approve/reject decision
decision = gen.xor(approve, reject)

# Complete model
model = gen.sequence(receive, checks, decision)
\`\`\`

Example 3: Order Processing with Rework
Process: Orders are received and processed. If processing fails, the order
is sent back for rework (up to 3 times). If it succeeds, the order is shipped.
If it fails 3 times, the order is cancelled.

Model:
\`\`\`python
gen = ModelGenerator()
receive = gen.activity("Receive order")
process = gen.activity("Process order")
rework = gen.activity("Rework order")
ship = gen.activity("Ship order")
cancel = gen.activity("Cancel order")

# Loop: process with rework, exit to ship or cancel
process_loop = gen.loop(do=process, redo=rework)

# Final decision: ship or cancel
final_decision = gen.xor(ship, cancel)

# Complete flow
model = gen.sequence(receive, process_loop, final_decision)
\`\`\``,

  /**
   * Negative prompting - common errors to avoid
   *
   * Strategy 4: Negative Prompting
   */
  COMMON_ERRORS: `CRITICAL ERRORS TO AVOID:

1. Self-loops in partial orders (IRREFLEXIVITY violation)
   ❌ WRONG: gen.partial_order(dependencies=[(A, A)])
   ✅ RIGHT: Never include (A, A) - activities cannot precede themselves

2. Non-transitive dependencies (TRANSITIVITY violation)
   ❌ WRONG: gen.partial_order(dependencies=[(A, B), (B, C)])  # Missing A→C
   ✅ RIGHT: gen.partial_order(dependencies=[(A, B), (B, C), (A, C)])

3. Local choices instead of complete XOR paths
   ❌ WRONG: gen.sequence(A, gen.xor(B, C), D)  # B and C must both lead to D
   ✅ RIGHT: gen.xor(gen.sequence(A, B, D), gen.sequence(A, C, D))

4. Reusing sub-models without copying
   ❌ WRONG: sub = gen.activity("Subtask"); model = gen.xor(sub, sub)
   ✅ RIGHT: sub = gen.activity("Subtask"); model = gen.xor(sub, sub.copy())

5. XOR with less than 2 arguments
   ❌ WRONG: gen.xor(A)  # XOR requires choice between alternatives
   ✅ RIGHT: gen.xor(A, B)  # At least 2 alternatives

6. Loop with invalid structure
   ❌ WRONG: gen.loop(do=None, redo=A)  # 'do' part is mandatory
   ✅ RIGHT: gen.loop(do=A, redo=B)  # Both parts specified`,

  /**
   * Generate a complete prompt for process model generation
   *
   * @param processDescription Natural language description of the process
   * @param conversationHistory Previous messages in the conversation (for error refinement)
   * @returns Complete prompt as a user message
   */
  generatePrompt(
    processDescription: string,
    conversationHistory: LLMMessage[] = []
  ): string {
    const basePrompt = `${this.SYSTEM_PROMPT}

${this.EXAMPLES}

${this.COMMON_ERRORS}

TASK:
Generate a POWL model for the following process description:

${processDescription}

Requirements:
1. Use ModelGenerator class with methods: activity(), xor(), loop(), partial_order(), sequence()
2. Ensure all partial orders are irreflexive and transitive
3. Use xor() for complete exclusive paths
4. Call copy() before reusing any sub-model
5. Return only the Python code, no explanation

Response format:
\`\`\`python
gen = ModelGenerator()
# Your model construction code here
\`\`\``;

    // If this is a refinement after an error, include the error context
    if (conversationHistory.length > 2) {
      // More than system + first user message means we're in a refinement loop
      const lastAssistant = conversationHistory[conversationHistory.length - 2];
      const lastUser = conversationHistory[conversationHistory.length - 1];

      return `${basePrompt}

PREVIOUS ATTEMPT:
${lastAssistant.content}

ERROR FEEDBACK:
${lastUser.content}

Please fix the error and regenerate the model code.`;
    }

    return basePrompt;
  },

  /**
   * Generate error refinement prompt
   *
   * @param error The error message or validation failure
   * @param conversationHistory Full conversation history
   * @returns Refinement prompt message
   */
  ERROR_REFINEMENT(
    error: string,
    conversationHistory: LLMMessage[]
  ): string {
    // Get the last assistant message (the code that failed)
    const lastAssistant = conversationHistory
      .slice()
      .reverse()
      .find((msg) => msg.role === "assistant");

    let refinement = `ERROR ENCOUNTERED:
${error}

${this.COMMON_ERRORS}`;

    if (lastAssistant) {
      refinement += `

PREVIOUS ATTEMPT:
${lastAssistant.content}`;
    }

    refinement += `

Please analyze the error, identify which rule was violated, and regenerate the POWL model code with the fix applied.`;

    return refinement;
  },

  /**
   * Generate validation feedback prompt
   *
   * @param validationResult Result from POWL validation
   * @param modelCode The generated model code
   * @returns Feedback prompt
   */
  VALIDATION_FEEDBACK(validationResult: {
    isValid: boolean;
    errors: string[];
    warnings: string[];
  }): string {
    if (validationResult.isValid) {
      return "✅ Model validation passed. The POWL model is correct and ready for use.";
    }

    let feedback = "❌ Model validation failed. Please fix the following issues:\n\n";

    if (validationResult.errors.length > 0) {
      feedback += "ERRORS (must fix):\n";
      validationResult.errors.forEach((err, i) => {
        feedback += `  ${i + 1}. ${err}\n`;
      });
    }

    if (validationResult.warnings.length > 0) {
      feedback += "\nWARNINGS (should fix):\n";
      validationResult.warnings.forEach((warn, i) => {
        feedback += `  ${i + 1}. ${warn}\n`;
      });
    }

    feedback += "\n" + this.COMMON_ERRORS;

    return feedback;
  },

  /**
   * Extract model code from LLM response
   *
   * @param response The raw LLM response text
   * @returns Extracted Python code or null if not found
   */
  extractModelCode(response: string): string | null {
    // Try to extract code from markdown code blocks
    const codeBlockMatch = response.match(/```python\n([\s\S]*?)\n```/);
    if (codeBlockMatch) {
      return codeBlockMatch[1].trim();
    }

    // Try without language specifier
    const genericCodeBlockMatch = response.match(/```\n([\s\S]*?)\n```/);
    if (genericCodeBlockMatch) {
      return genericCodeBlockMatch[1].trim();
    }

    // If no code blocks found, check if the entire response is code
    const trimmed = response.trim();
    if (trimmed.startsWith("gen = ModelGenerator()") ||
        trimmed.includes("gen.activity(") ||
        trimmed.includes("gen.xor(") ||
        trimmed.includes("gen.loop(") ||
        trimmed.includes("gen.partial_order(")) {
      return trimmed;
    }

    return null;
  },
};

/**
 * Process model generation request
 */
export interface ModelGenerationRequest {
  processDescription: string;
  conversationHistory?: LLMMessage[];
}

/**
 * Process model generation response
 */
export interface ModelGenerationResponse {
  modelCode: string | null;
  rawResponse: string;
  conversationHistory: LLMMessage[];
}

/**
 * Main interface for LLM-based POWL generation
 */
export class LLMPowLGenerator {
  private conversationHistory: LLMMessage[] = [];

  constructor() {
    // Initialize with system prompt
    this.conversationHistory.push({
      role: "system",
      content: PROMPT_TEMPLATES.SYSTEM_PROMPT,
    });
  }

  /**
   * Generate a prompt for the given process description
   */
  generatePrompt(processDescription: string): string {
    return PROMPT_TEMPLATES.generatePrompt(
      processDescription,
      this.conversationHistory
    );
  }

  /**
   * Handle an error and generate refinement prompt
   */
  handleError(error: string): string {
    const refinementPrompt = PROMPT_TEMPLATES.ERROR_REFINEMENT(
      error,
      this.conversationHistory
    );

    this.conversationHistory.push({
      role: "user",
      content: refinementPrompt,
    });

    return refinementPrompt;
  }

  /**
   * Process LLM response and extract model code
   *
   * Note: This assumes the user prompt was already added to history via generatePrompt()
   * or was sent separately. If you need to track the user prompt, call generatePrompt()
   * and manually add it to history before calling this method.
   */
  processResponse(llmResponse: string): ModelGenerationResponse {
    // Add assistant response to history
    this.conversationHistory.push({
      role: "assistant",
      content: llmResponse,
    });

    // Extract model code
    const modelCode = PROMPT_TEMPLATES.extractModelCode(llmResponse);

    return {
      modelCode,
      rawResponse: llmResponse,
      conversationHistory: [...this.conversationHistory],
    };
  }

  /**
   * Reset conversation history (start fresh)
   */
  reset(): void {
    this.conversationHistory = [
      {
        role: "system",
        content: PROMPT_TEMPLATES.SYSTEM_PROMPT,
      },
    ];
  }

  /**
   * Get current conversation history
   */
  getHistory(): LLMMessage[] {
    return [...this.conversationHistory];
  }

  /**
   * Add a user message to conversation history
   *
   * This is typically used after generating a prompt to track what was sent to the LLM
   *
   * @param content The user message content (typically the prompt)
   */
  addUserMessage(content: string): void {
    this.conversationHistory.push({
      role: "user",
      content,
    });
  }

  /**
   * Add an assistant message to conversation history
   *
   * This is typically used after receiving an LLM response
   *
   * @param content The assistant message content (typically the LLM response)
   */
  addAssistantMessage(content: string): void {
    this.conversationHistory.push({
      role: "assistant",
      content,
    });
  }

  /**
   * Generate validation feedback message
   */
  generateValidationFeedback(validationResult: {
    isValid: boolean;
    errors: string[];
    warnings: string[];
  }): string {
    return PROMPT_TEMPLATES.VALIDATION_FEEDBACK(validationResult);
  }
}
