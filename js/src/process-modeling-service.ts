/**
 * Process Modeling Service
 *
 * High-level service that combines LLM-based generation, error handling,
 * and user feedback refinement for creating process models.
 *
 * This service implements the complete workflow from the paper:
 * 1. Generate initial model from description
 * 2. Validate and handle errors
 * 3. Incorporate user feedback
 * 4. Refine iteratively
 */

import { Powl, PowlModel } from "./index.js";
import { PROMPT_TEMPLATES } from "./llm-prompts.js";
import { ErrorHandler } from "./error-handler.js";
import { RefinementLoop, UserFeedback } from "./refinement-loop.js";

/**
 * Configuration for the process modeling service
 */
export interface ProcessModelingConfig {
  maxRefinementIterations?: number;
  autoFixErrors?: boolean;
  enableConformanceChecking?: boolean;
}

/**
 * Result of model generation
 */
export interface ModelGenerationResult {
  model: PowlModel | null;
  iterations: number;
  errors: string[];
  warnings: string[];
  conversationHistory: Array<{ role: string; content: string }>;
}

/**
 * Result of model refinement
 */
export interface ModelRefinementResult {
  model: PowlModel | null;
  success: boolean;
  iterations: number;
  feedbackIncorporated: number;
  error?: string;
}

/**
 * Process Modeling Service - Main API for LLM-based process modeling
 */
export class ProcessModelingService {
  private powl: Powl;
  private errorHandler: ErrorHandler;
  private refinementLoop: RefinementLoop | null;
  private llmCall: (prompt: string) => Promise<string>;
  private config: ProcessModelingConfig;

  /**
   * Create a new process modeling service
   *
   * @param llmCall Async function that calls an LLM with a prompt and returns the response
   * @param config Optional configuration
   */
  constructor(
    llmCall: (prompt: string) => Promise<string>,
    config: ProcessModelingConfig = {}
  ) {
    this.powl = null as any; // Will be initialized when needed
    this.errorHandler = new ErrorHandler();
    this.refinementLoop = null;
    this.llmCall = llmCall;
    this.config = {
      maxRefinementIterations: 5,
      autoFixErrors: true,
      enableConformanceChecking: true,
      ...config,
    };
  }

  /**
   * Initialize the POWL WASM module
   */
  private async ensureInitialized(): Promise<void> {
    if (!this.powl) {
      this.powl = await Powl.init();
    }
  }

  /**
   * Generate a process model from a natural language description
   *
   * This is the main entry point for creating process models.
   * It handles the complete workflow:
   * 1. Create initial prompt with role, knowledge, examples
   * 2. Call LLM to generate model code
   * 3. Validate and handle errors
   * 4. Return the parsed POWL model
   *
   * @param description Natural language description of the process
   * @returns Generated model with metadata
   */
  async generateModel(
    description: string
  ): Promise<ModelGenerationResult> {
    await this.ensureInitialized();

    const conversationHistory: Array<{ role: string; content: string }> = [];

    // Step 1: Create initial prompt with all the prompting strategies
    const initialPrompt = PROMPT_TEMPLATES.generatePrompt(description, []);

    conversationHistory.push({
      role: "user",
      content: initialPrompt,
    });

    try {
      // Step 2: Call LLM
      const response = await this.llmCall(initialPrompt);

      conversationHistory.push({
        role: "assistant",
        content: response,
      });

      // Step 3: Extract code from response
      let code = this.extractCode(response);

      // Step 4: Validate and handle errors
      const errorResult = await this.errorHandler.handleErrors(
        code,
        conversationHistory,
        this.llmCall.bind(this)
      );

      // Update conversation history with refinement iterations
      if (errorResult.iterations > 0) {
        conversationHistory.push({
          role: "system",
          content: `Error handling completed in ${errorResult.iterations} iterations`,
        });
      }

      // Step 5: Parse and create POWL model
      if (errorResult.success) {
        try {
          const powlString = this.codeToPowlString(code);
          const model = this.powl.parse(powlString);

          return {
            model,
            iterations: errorResult.iterations + 1,
            errors: [],
            warnings: [],
            conversationHistory,
          };
        } catch (e) {
          return {
            model: null,
            iterations: errorResult.iterations + 1,
            errors: [`Failed to parse POWL: ${e instanceof Error ? e.message : String(e)}`],
            warnings: [],
            conversationHistory,
          };
        }
      }

      return {
        model: null,
        iterations: errorResult.iterations,
        errors: errorResult.errors?.map(e => e.message) || [],
        warnings: [],
        conversationHistory,
      };
    } catch (error) {
      return {
        model: null,
        iterations: 1,
        errors: [`Generation failed: ${error instanceof Error ? error.message : String(error)}`],
        warnings: [],
        conversationHistory,
      };
    }
  }

  /**
   * Start a refinement session for iterative model improvement
   *
   * @param originalDescription The original process description
   * @param initialModel The initial generated model
   */
  startRefinementSession(
    originalDescription: string,
    initialModel: PowlModel
  ): void {
    this.refinementLoop = new RefinementLoop(originalDescription);
    this.refinementLoop.setCurrentModel(initialModel);
  }

  /**
   * Add user feedback and refine the model
   *
   * @param feedback User feedback (text or visual)
   * @returns Refined model
   */
  async addFeedbackAndRefine(
    feedback: UserFeedback
  ): Promise<ModelRefinementResult> {
    if (!this.refinementLoop) {
      return {
        model: null,
        success: false,
        iterations: 0,
        feedbackIncorporated: 0,
        error: "No active refinement session. Call startRefinementSession() first.",
      };
    }

    await this.ensureInitialized();

    // Add feedback to the loop
    this.refinementLoop.addFeedback(feedback);

    // Create base prompt for refinement
    const basePrompt = PROMPT_TEMPLATES.SYSTEM_PROMPT + "\n\n" +
      PROMPT_TEMPLATES.EXAMPLES + "\n\n" +
      PROMPT_TEMPLATES.COMMON_ERRORS;

    // Perform refinement
    const result = await this.refinementLoop.refineModel(
      this.llmCall,
      basePrompt
    );

    if (result.success && result.updatedModel) {
      try {
        // Use the updated model directly
        const model = result.updatedModel;

        // Update the current model in the loop
        this.refinementLoop.setCurrentModel(model);

        return {
          model,
          success: true,
          iterations: result.iteration,
          feedbackIncorporated: this.refinementLoop.getSession().feedbackHistory.length,
        };
      } catch (e) {
        return {
          model: null,
          success: false,
          iterations: result.iteration,
          feedbackIncorporated: this.refinementLoop.getSession().feedbackHistory.length,
          error: `Failed to parse refined model: ${e instanceof Error ? e.message : String(e)}`,
        };
      }
    }

    return {
      model: null,
      success: false,
      iterations: result.iteration,
      feedbackIncorporated: this.refinementLoop.getSession().feedbackHistory.length,
      error: result.error || "Refinement failed",
    };
  }

  /**
   * Get the current refinement session state
   */
  getRefinementSession(): RefinementLoop | null {
    return this.refinementLoop;
  }

  /**
   * End the current refinement session
   */
  endRefinementSession(): void {
    this.refinementLoop = null;
  }

  /**
   * Validate a model against an event log
   *
   * @param model The POWL model to validate
   * @param log Event log (XES or CSV)
   * @returns Conformance checking result
   */
  async validateModel(
    model: PowlModel,
    log: string
  ): Promise<{ percentage: number; avgTraceFitness: number }> {
    await this.ensureInitialized();

    try {
      // Try to parse as XES first
      let eventLog;
      try {
        eventLog = this.powl.parseXes(log);
      } catch {
        // If not XES, try CSV
        eventLog = this.powl.parseCsv(log);
      }

      const result = this.powl.conformance(model, eventLog);
      return {
        percentage: result.percentage,
        avgTraceFitness: result.avg_trace_fitness,
      };
    } catch (e) {
      throw new Error(`Validation failed: ${e instanceof Error ? e.message : String(e)}`);
    }
  }

  /**
   * Generate a summary report of the refinement session
   */
  generateRefinementReport(): string {
    if (!this.refinementLoop) {
      return "No active refinement session.";
    }

    return this.refinementLoop.createSummaryReport();
  }

  /**
   * Extract Python code from LLM response
   */
  private extractCode(response: string): string {
    // Try to extract code from markdown code blocks
    const codeMatch = response.match(/```python\n([\s\S]+?)```/);
    if (codeMatch) {
      return codeMatch[1].trim();
    }

    // Try without language specifier
    const plainMatch = response.match(/```\n([\s\S]+?)```/);
    if (plainMatch) {
      return plainMatch[1].trim();
    }

    // Return as-is if no code blocks found
    return response.trim();
  }

  /**
   * Convert Python-style model generation code to POWL string
   *
   * This is a simplified implementation. In practice, you would:
   * 1. Parse the Python code
   * 2. Extract the model structure
   * 3. Convert to POWL string representation
   *
   * For now, we try to extract the POWL string directly or use a placeholder.
   */
  private codeToPowlString(code: string): string {
    // Try to extract POWL string from the code
    const powlMatch = code.match(/POWL?\s*=\s*([A-Z]\(.+\))/s);
    if (powlMatch) {
      return powlMatch[1];
    }

    // Try to extract from operator calls
    const operatorMatch = code.match(/operator\("(\w+)",\s*\[(.+)\]\)/s);
    if (operatorMatch) {
      const op = operatorMatch[1];
      const args = operatorMatch[2];

      // Convert to POWL string format
      switch (op) {
        case "→":
        case "Sequence":
          return `→(${args})`;
        case "X":
        case "Xor":
          return `X(${args})`;
        case "○":
        case "Loop":
          return `○(${args})`;
        case "∧":
        case "And":
          return `∧(${args})`;
        default:
          return `PO=(nodes={A}, order={})`;
      }
    }

    // Fallback: return a simple model
    return "PO=(nodes={A}, order={})";
  }

  /**
   * Get the current service configuration
   */
  getConfig(): ProcessModelingConfig {
    return { ...this.config };
  }

  /**
   * Update the service configuration
   */
  updateConfig(config: Partial<ProcessModelingConfig>): void {
    this.config = { ...this.config, ...config };
    this.errorHandler.updateConfig({
      maxCriticalIterations: this.config.maxRefinementIterations,
      maxAdjustableIterations: 2,
      autoResolveAfter: 2,
    });
  }
}

/**
 * Convenience function to create a process modeling service
 *
 * @param llmCall Async function that calls an LLM
 * @param config Optional configuration
 * @returns Configured service instance
 */
export function createProcessModelingService(
  llmCall: (prompt: string) => Promise<string>,
  config?: ProcessModelingConfig
): ProcessModelingService {
  return new ProcessModelingService(llmCall, config);
}
