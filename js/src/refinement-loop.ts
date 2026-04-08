/**
 * Refinement Loop - Interactive process model improvement based on user feedback
 *
 * This module implements the feedback loop described in the paper where users can:
 * 1. View the generated process model
 * 2. Provide feedback/comments
 * 3. Get an updated model incorporating their feedback
 */

import type { PowlModel } from "./index.js";

/**
 * User feedback entry
 */
export interface UserFeedback {
  type: 'text' | 'visual';
  content: string;
  timestamp: Date;
}

/**
 * Active refinement session tracking
 */
export interface RefinementSession {
  originalDescription: string;
  currentModel: PowlModel | null;
  feedbackHistory: UserFeedback[];
  conversationHistory: Array<{ role: string; content: string }>;
  iterationCount: number;
}

/**
 * Result of a refinement operation
 */
export interface RefinementResult {
  success: boolean;
  updatedModel: PowlModel | null;
  prompt: string;
  response: string;
  iteration: number;
  error?: string;
}

/**
 * Refinement loop for iterative process model improvement
 */
export class RefinementLoop {
  private session: RefinementSession;

  constructor(initialDescription: string) {
    this.session = {
      originalDescription: initialDescription,
      currentModel: null,
      feedbackHistory: [],
      conversationHistory: [],
      iterationCount: 0,
    };
  }

  /**
   * Get the current session state
   */
  getSession(): RefinementSession {
    return { ...this.session };
  }

  /**
   * Update the current model
   */
  setCurrentModel(model: PowlModel | null): void {
    this.session.currentModel = model;
  }

  /**
   * Add user feedback to the session
   */
  addFeedback(feedback: UserFeedback): void {
    this.session.feedbackHistory.push(feedback);
  }

  /**
   * Add a conversation message
   */
  addConversationMessage(role: string, content: string): void {
    this.session.conversationHistory.push({ role, content });
  }

  /**
   * Generate prompt incorporating all feedback and conversation history
   */
  generateRefinementPrompt(basePrompt: string): string {
    const feedbackText = this.session.feedbackHistory.length > 0
      ? this.session.feedbackHistory
          .map((f, i) => `Feedback ${i + 1} (${f.type}): ${f.content}`)
          .join('\n')
      : "No feedback yet.";

    const conversation = this.session.conversationHistory.length > 0
      ? this.session.conversationHistory
          .map(msg => `${msg.role}: ${msg.content}`)
          .join('\n\n')
      : "No previous conversation.";

    const currentModelInfo = this.session.currentModel
      ? `Current model:\n${this.session.currentModel.toString()}\n\n`
      : "";

    return `
${basePrompt}

CURRENT MODEL STATE:
${currentModelInfo}USER FEEDBACK HISTORY:
${feedbackText}

PREVIOUS CONVERSATION:
${conversation}

INSTRUCTIONS:
Please update the POWL model to address the user's feedback.
- Maintain all previous improvements while incorporating new requirements
- Provide the complete updated model code
- Explain the changes you made
- If the feedback is unclear, ask clarifying questions
`;
  }

  /**
   * Process refinement with LLM
   */
  async refineModel(
    llmCall: (prompt: string) => Promise<string>,
    basePrompt: string
  ): Promise<RefinementResult> {
    this.session.iterationCount++;
    const refinementPrompt = this.generateRefinementPrompt(basePrompt);

    try {
      const response = await llmCall(refinementPrompt);

      // Add to conversation history
      this.addConversationMessage('user', refinementPrompt);
      this.addConversationMessage('assistant', response);

      // Extract and parse the model
      const updatedModel = this.parseModelFromResponse(response);

      if (updatedModel) {
        this.session.currentModel = updatedModel;
        return {
          success: true,
          updatedModel,
          prompt: refinementPrompt,
          response,
          iteration: this.session.iterationCount,
        };
      }

      return {
        success: false,
        updatedModel: null,
        prompt: refinementPrompt,
        response,
        iteration: this.session.iterationCount,
        error: "Failed to parse model from response",
      };
    } catch (error) {
      return {
        success: false,
        updatedModel: null,
        prompt: refinementPrompt,
        response: "",
        iteration: this.session.iterationCount,
        error: error instanceof Error ? error.message : String(error),
      };
    }
  }

  /**
   * Parse model from LLM response
   * Extracts POWL model code from markdown code blocks
   */
  private parseModelFromResponse(response: string): PowlModel | null {
    // Try to extract Python code block
    const codeMatch = response.match(/```python\n([\s\S]+?)```/);
    if (codeMatch) {
      // Return the raw code - will be parsed by the service
      return { code: codeMatch[1], type: 'powl' } as any;
    }

    // Try to extract plain code block
    const plainMatch = response.match(/```\n([\s\S]+?)```/);
    if (plainMatch) {
      return { code: plainMatch[1], type: 'powl' } as any;
    }

    // Look for POWL string patterns directly in text
    const powlMatch = response.match(/POWL?\s*=\s*([A-Z]\(.+\))/s);
    if (powlMatch) {
      return { code: powlMatch[1], type: 'powl-string' } as any;
    }

    return null;
  }

  /**
   * Get session summary
   */
  getSessionSummary(): {
    feedbackCount: number;
    hasModel: boolean;
    conversationTurns: number;
    iterationCount: number;
  } {
    return {
      feedbackCount: this.session.feedbackHistory.length,
      hasModel: !!this.session.currentModel,
      conversationTurns: Math.floor(this.session.conversationHistory.length / 2),
      iterationCount: this.session.iterationCount,
    };
  }

  /**
   * Export session for persistence
   */
  exportSession(): string {
    return JSON.stringify({
      originalDescription: this.session.originalDescription,
      feedbackHistory: this.session.feedbackHistory,
      conversationHistory: this.session.conversationHistory,
      iterationCount: this.session.iterationCount,
      currentModelString: this.session.currentModel?.toString() || null,
    }, null, 2);
  }

  /**
   * Import session from JSON
   */
  static importSession(json: string, initialModel: PowlModel | null = null): RefinementLoop {
    const data = JSON.parse(json);
    const loop = new RefinementLoop(data.originalDescription);
    loop.session.feedbackHistory = data.feedbackHistory.map((f: any) => ({
      ...f,
      timestamp: new Date(f.timestamp),
    }));
    loop.session.conversationHistory = data.conversationHistory;
    loop.session.iterationCount = data.iterationCount;
    loop.session.currentModel = initialModel;
    return loop;
  }

  /**
   * Reset the session (keep original description)
   */
  reset(): void {
    this.session.currentModel = null;
    this.session.feedbackHistory = [];
    this.session.conversationHistory = [];
    this.session.iterationCount = 0;
  }

  /**
   * Create a text summary of the refinement session
   */
  createSummaryReport(): string {
    const summary = this.getSessionSummary();
    const lines: string[] = [
      "=== Refinement Session Summary ===",
      "",
      `Original Description: ${this.session.originalDescription}`,
      `Iterations: ${summary.iterationCount}`,
      `Feedback Items: ${summary.feedbackCount}`,
      `Conversation Turns: ${summary.conversationTurns}`,
      `Has Valid Model: ${summary.hasModel ? "Yes" : "No"}`,
      "",
    ];

    if (this.session.feedbackHistory.length > 0) {
      lines.push("Feedback History:");
      this.session.feedbackHistory.forEach((f, i) => {
        lines.push(`  ${i + 1}. [${f.type}] ${f.content}`);
        lines.push(`     Time: ${f.timestamp.toISOString()}`);
      });
      lines.push("");
    }

    if (this.session.currentModel) {
      lines.push("Current Model:");
      lines.push(this.session.currentModel.toString());
      lines.push("");
    }

    return lines.join("\n");
  }
}
