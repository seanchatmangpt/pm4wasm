// ─── Example Usage of ErrorHandler ─────────────────────────────────────────────

import {
  ErrorHandler,
  type ConversationMessage
} from './error-handler';

/**
 * Example 1: Basic validation
 */
export function example1_basicValidation() {
  const handler = new ErrorHandler();

  // Code with critical errors
  const badCode = `
import numpy as np  # External import - CRITICAL
A --> A  # Self-loop - CRITICAL
`;

  const errors = handler.validate(badCode);

  console.log('Validation found', errors.length, 'errors:');
  errors.forEach(error => {
    console.log(`- [${error.severity}] ${error.message}`);
  });

  // Output:
  // Validation found 2 errors:
  // - [critical] External imports are not allowed
  // - [critical] Self-loop detected: A violates irreflexivity
}

/**
 * Example 2: Auto-fixing adjustable errors
 */
export async function example2_autoFix() {
  const handler = new ErrorHandler({
    maxCriticalIterations: 5,
    maxAdjustableIterations: 1,  // Only try LLM once for adjustable errors
    autoResolveAfter: 1          // Then auto-fix
  });

  const code = 'partial_order(base_model) = extended_model';
  const conversation: ConversationMessage[] = [
    { role: 'user', content: 'Generate a POWL model' }
  ];

  // Mock LLM that keeps returning bad code
  const mockLLM = async (_prompt: string) => {
    return '```python\npartial_order(base_model) = extended_model\n```';
  };

  const result = await handler.handleErrors(code, conversation, mockLLM);

  console.log('Success:', result.success);
  console.log('Fixed code:', result.fixedCode);
  console.log('Iterations:', result.iterations);

  // Output:
  // Success: true
  // Fixed code: partial_order(base_model.copy()) = extended_model
  // Iterations: 1
}

/**
 * Example 3: LLM fixes critical errors
 */
export async function example3_llmFixesErrors() {
  const handler = new ErrorHandler();

  const code = 'A --> A  # Self-loop violates irreflexivity';
  const conversation: ConversationMessage[] = [
    { role: 'user', content: 'Generate a process model' }
  ];

  // Mock LLM that fixes the error
  const mockLLM = async (_prompt: string) => {
    return '```python\nA --> B\nB --> C\n```';
  };

  const result = await handler.handleErrors(code, conversation, mockLLM);

  console.log('Success:', result.success);
  console.log('Fixed code:', result.fixedCode);

  // Output:
  // Success: true
  // Fixed code: A --> B
  // B --> C
}

/**
 * Example 4: Multiple error types
 */
export async function example4_multipleErrors() {
  const handler = new ErrorHandler();

  const code = `
import pandas as pd  # CRITICAL: external import
A --> A  # CRITICAL: self-loop
partial_order(model) = new  # ADJUSTABLE: sub-model reuse
`;

  const conversation: ConversationMessage[] = [
    { role: 'user', content: 'Create a complex process model' }
  ];

  // Mock LLM that progressively fixes errors
  let callCount = 0;
  const mockLLM = async (prompt: string) => {
    callCount++;

    if (prompt.includes('External imports')) {
      // First call: fix import
      return '```python\nA --> A\npartial_order(model) = new\n```';
    } else if (prompt.includes('Self-loop')) {
      // Second call: fix self-loop
      return '```python\nA --> B\npartial_order(model) = new\n```';
    } else {
      // Third call: fix sub-model reuse
      return '```python\nA --> B\nmodel_copy = model.copy()\n```';
    }
  };

  const result = await handler.handleErrors(code, conversation, mockLLM);

  console.log('Success:', result.success);
  console.log('Total LLM calls:', callCount);
  console.log('Final code:', result.fixedCode);

  // Output:
  // Success: true
  // Total LLM calls: 3
  // Final code: A --> B
  // model_copy = model.copy()
}

/**
 * Example 5: Giving up after max iterations
 */
export async function example5_giveUp() {
  const handler = new ErrorHandler({
    maxCriticalIterations: 2,  // Only try twice
    maxAdjustableIterations: 2,
    autoResolveAfter: 2
  });

  const code = 'import numpy as np  # Keeps failing';
  const conversation: ConversationMessage[] = [
    { role: 'user', content: 'Generate a model' }
  ];

  // Mock LLM that never fixes the error
  const mockLLM = async (_prompt: string) => {
    return '```python\nimport numpy as np\n```';
  };

  const result = await handler.handleErrors(code, conversation, mockLLM);

  console.log('Success:', result.success);
  console.log('Iterations:', result.iterations);
  console.log('Remaining errors:', result.errors?.length);

  // Output:
  // Success: false
  // Iterations: 2
  // Remaining errors: 1
}

/**
 * Example 6: Configuration updates
 */
export function example6_configuration() {
  const handler = new ErrorHandler();

  // Get default config
  console.log('Default config:', handler.getConfig());
  // Output: { maxCriticalIterations: 5, maxAdjustableIterations: 2, autoResolveAfter: 2 }

  // Update config
  handler.updateConfig({
    maxCriticalIterations: 10,
    maxAdjustableIterations: 5
  });

  console.log('Updated config:', handler.getConfig());
  // Output: { maxCriticalIterations: 10, maxAdjustableIterations: 5, autoResolveAfter: 2 }
}

// ─── Run All Examples ───────────────────────────────────────────────────────────

export async function runAllExamples() {
  console.log('=== Example 1: Basic Validation ===');
  example1_basicValidation();
  console.log();

  console.log('=== Example 2: Auto-fix Adjustable Errors ===');
  await example2_autoFix();
  console.log();

  console.log('=== Example 3: LLM Fixes Critical Errors ===');
  await example3_llmFixesErrors();
  console.log();

  console.log('=== Example 4: Multiple Error Types ===');
  await example4_multipleErrors();
  console.log();

  console.log('=== Example 5: Giving Up After Max Iterations ===');
  await example5_giveUp();
  console.log();

  console.log('=== Example 6: Configuration Management ===');
  example6_configuration();
  console.log();
}

// Uncomment to run examples:
// runAllExamples();
