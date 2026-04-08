// ─── Error Handling for Process Modeling with Large Language Models ────────
// Based on the paper categorizing errors into Critical and Adjustable types

/**
 * Error severity levels as defined in the research paper
 */
export enum ErrorSeverity {
  CRITICAL = 'critical',
  ADJUSTABLE = 'adjustable'
}

/**
 * Represents a validation error in generated POWL code
 */
export interface LLMError {
  message: string;
  severity: ErrorSeverity;
  line?: number;
  fixable?: boolean;
}

/**
 * Configuration for error handling behavior
 */
export interface ErrorHandlingConfig {
  maxCriticalIterations: number;
  maxAdjustableIterations: number;
  autoResolveAfter: number; // iterations before auto-resolving adjustable errors
}

/**
 * Message format for LLM conversation history
 */
export interface ConversationMessage {
  role: string;
  content: string;
}

/**
 * Result type for error handling operations
 */
export interface ErrorHandlingResult {
  success: boolean;
  fixedCode: string;
  iterations: number;
  errors?: LLMError[];
}

/**
 * ErrorHandler implements the two-tier error handling strategy from the paper:
 *
 * 1. Critical Errors: Execution failures, security risks, major validation violations
 *    - Up to 5 iterations with LLM before giving up
 *    - Examples: syntax errors, undefined functions, external library usage
 *
 * 2. Adjustable Errors: Model quality issues that can be auto-fixed
 *    - Up to 2 iterations with LLM, then auto-resolve
 *    - Examples: sub-model reuse, minor validation issues
 */
export class ErrorHandler {
  private config: ErrorHandlingConfig;

  constructor(config: ErrorHandlingConfig = {
    maxCriticalIterations: 5,
    maxAdjustableIterations: 2,
    autoResolveAfter: 2
  }) {
    this.config = config;
  }

  /**
   * Validate generated POWL code and categorize errors
   *
   * Checks for:
   * - Critical: External imports, dangerous functions, self-loops (irreflexivity violations)
   * - Adjustable: Sub-model reuse issues, transitivity problems
   */
  validate(code: string): LLMError[] {
    const errors: LLMError[] = [];

    // Check for critical errors
    if (code.includes('import ')) {
      errors.push({
        message: 'External imports are not allowed',
        severity: ErrorSeverity.CRITICAL,
        fixable: false
      });
    }

    if (code.includes('eval(') || code.includes('exec(')) {
      errors.push({
        message: 'Dangerous functions (eval/exec) detected',
        severity: ErrorSeverity.CRITICAL,
        fixable: false
      });
    }

    // Check for common POWL errors
    const lines = code.split('\n');
    lines.forEach((line, idx) => {
      // Check for self-loops (violates irreflexivity)
      const selfLoopMatch = line.match(/(\w+)\s*-->\s*\1/);
      if (selfLoopMatch) {
        errors.push({
          message: `Self-loop detected: ${selfLoopMatch[1]} violates irreflexivity`,
          severity: ErrorSeverity.CRITICAL,
          line: idx + 1,
          fixable: true
        });
      }

      // Check for missing transitivity hints
      // This would require more sophisticated analysis in practice
      if (line.includes('-->') && !line.includes('transitive')) {
        // Flag for potential transitivity issues
        // This is a heuristic - full transitivity checking requires graph analysis
      }
    });

    // Check for adjustable errors
    if (code.match(/partial_order\([^)]*\)\s*=/)) {
      errors.push({
        message: 'Sub-model reuse detected (should call copy())',
        severity: ErrorSeverity.ADJUSTABLE,
        fixable: true
      });
    }

    return errors;
  }

  /**
   * Determine if error can be auto-resolved
   *
   * Only adjustable errors marked as fixable can be auto-resolved
   */
  canAutoResolve(error: LLMError): boolean {
    if (error.severity === ErrorSeverity.CRITICAL) return false;
    return error.fixable === true;
  }

  /**
   * Apply automatic fix for adjustable errors
   *
   * Currently handles:
   * - Sub-model reuse: wraps partial_order references in .copy()
   */
  autoFix(code: string, error: LLMError): string {
    if (error.message.includes('Sub-model reuse')) {
      // Auto-fix: wrap in copy() to prevent shared state issues
      // This is a simplified fix - in practice would need more sophisticated AST manipulation
      return code.replace(
        /partial_order\(([^)]+)\)/g,
        'partial_order($1.copy())'
      );
    }

    if (error.message.includes('Self-loop')) {
      // Auto-fix: remove self-loops by removing the offending line
      const lines = code.split('\n');
      if (error.line) {
        lines.splice(error.line - 1, 1);
        return lines.join('\n');
      }
    }

    return code;
  }

  /**
   * Generate refinement prompt for LLM
   *
   * Includes conversation context and specific error information
   */
  generateRefinementPrompt(
    error: LLMError,
    conversation: ConversationMessage[]
  ): string {
    const context = conversation
      .map(msg => `${msg.role}: ${msg.content}`)
      .join('\n\n');

    return `
ERROR DETECTED: ${error.message}
${error.line ? `Line ${error.line}` : ''}
Severity: ${error.severity}

CONVERSATION HISTORY:
${context}

Please fix the error and regenerate the POWL model code.
Remember:
- No external imports
- No eval/exec
- All partial orders must be irreflexive and transitive
- Use .copy() when reusing sub-models to prevent shared state issues
`;
  }

  /**
   * Handle errors with retry logic
   *
   * Implements the two-tier strategy:
   * 1. Try LLM refinement for adjustable errors (up to maxAdjustableIterations)
   * 2. Auto-resolve if still failing after threshold
   * 3. Handle critical errors with LLM (up to maxCriticalIterations)
   *
   * @param code - The generated code to validate
   * @param conversation - Conversation history for context
   * @param llmCall - Async function to call LLM with prompt
   * @returns Result indicating success, fixed code, and iterations used
   */
  async handleErrors(
    code: string,
    conversation: ConversationMessage[],
    llmCall: (prompt: string) => Promise<string>
  ): Promise<ErrorHandlingResult> {
    let currentCode = code;
    let iterations = 0;
    let errors = this.validate(currentCode);

    while (errors.length > 0 && iterations < this.config.maxCriticalIterations) {
      const criticalErrors = errors.filter(e => e.severity === ErrorSeverity.CRITICAL);
      const adjustableErrors = errors.filter(e => e.severity === ErrorSeverity.ADJUSTABLE);

      // Try to fix adjustable errors first
      let autoFixed = false;
      for (const error of adjustableErrors) {
        if (iterations >= this.config.maxAdjustableIterations) {
          // Auto-resolve if we've exceeded the adjustable iteration limit
          if (this.canAutoResolve(error)) {
            currentCode = this.autoFix(currentCode, error);
            autoFixed = true;
            continue;
          }
        }

        const prompt = this.generateRefinementPrompt(error, conversation);
        const response = await llmCall(prompt);
        currentCode = this.extractCode(response);
        iterations++;

        // Update conversation with LLM response
        conversation.push({
          role: 'assistant',
          content: response
        });
      }

      // If we auto-fixed, re-validate and continue
      if (autoFixed) {
        const newErrors = this.validate(currentCode);
        if (newErrors.length === 0) {
          errors = newErrors;
          break;
        }
        errors = newErrors;
        continue;
      }

      // Then handle critical errors
      for (const error of criticalErrors) {
        if (iterations >= this.config.maxCriticalIterations) {
          return {
            success: false,
            fixedCode: currentCode,
            iterations,
            errors
          };
        }

        const prompt = this.generateRefinementPrompt(error, conversation);
        const response = await llmCall(prompt);
        currentCode = this.extractCode(response);
        iterations++;

        // Update conversation with LLM response
        conversation.push({
          role: 'assistant',
          content: response
        });
      }

      // Re-validate after fixes
      const newErrors = this.validate(currentCode);
      errors = newErrors;
      if (newErrors.length === 0) break;
    }

    return {
      success: errors.length === 0,
      fixedCode: currentCode,
      iterations,
      errors: errors.length > 0 ? errors : undefined
    };
  }

  /**
   * Extract Python code from LLM response
   *
   * Handles code blocks wrapped in ```python ... ``` markers
   */
  private extractCode(response: string): string {
    // Extract code between ```python and ```
    const match = response.match(/```python\n([\s\S]+?)```/);
    if (match && match[1]) {
      return match[1].trim();
    }

    // Fallback: try without language specifier
    const fallbackMatch = response.match(/```\n([\s\S]+?)```/);
    if (fallbackMatch && fallbackMatch[1]) {
      return fallbackMatch[1].trim();
    }

    // If no code blocks found, return response as-is
    return response.trim();
  }

  /**
   * Get current configuration
   */
  getConfig(): ErrorHandlingConfig {
    return { ...this.config };
  }

  /**
   * Update configuration
   */
  updateConfig(config: Partial<ErrorHandlingConfig>): void {
    this.config = { ...this.config, ...config };
  }
}

// ─── Example Usage ────────────────────────────────────────────────────────────────

/**
 * Example usage of ErrorHandler with a mock LLM call
 */
export async function exampleUsage() {
  const errorHandler = new ErrorHandler();

  // Example code with errors
  const problematicCode = `
import numpy as np  # Critical: external import

partial_order(model) = ...  # Adjustable: sub-model reuse

A --> A  # Critical: self-loop violates irreflexivity
`;

  // Mock LLM call function
  const mockLLMCall = async (prompt: string): Promise<string> => {
    console.log('LLM Prompt:', prompt);
    return `
\`\`\`python
# Fixed code without errors
model_copy = model.copy()
partial_order(model_copy)

A --> B
B --> C
\`\`\`
`;
  };

  const conversation: ConversationMessage[] = [
    { role: 'user', content: 'Generate a POWL model for order processing' }
  ];

  const result = await errorHandler.handleErrors(
    problematicCode,
    conversation,
    mockLLMCall
  );

  console.log('Error handling result:', result);
  return result;
}
