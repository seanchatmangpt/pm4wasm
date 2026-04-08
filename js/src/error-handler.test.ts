// ─── Tests for ErrorHandler ─────────────────────────────────────────────────────

import { describe, it, expect, vi } from 'vitest';
import {
  ErrorHandler,
  ErrorSeverity,
  type LLMError,
  type ErrorHandlingConfig,
  type ConversationMessage
} from './error-handler';

describe('ErrorHandler', () => {
  describe('validate', () => {
    it('should detect external imports as critical errors', () => {
      const handler = new ErrorHandler();
      const code = 'import numpy as np\nmodel = ...';

      const errors = handler.validate(code);

      expect(errors).toHaveLength(1);
      expect(errors[0].severity).toBe(ErrorSeverity.CRITICAL);
      expect(errors[0].message).toContain('External imports');
      expect(errors[0].fixable).toBe(false);
    });

    it('should detect eval/exec as critical errors', () => {
      const handler = new ErrorHandler();
      const code = 'result = eval(user_input)';

      const errors = handler.validate(code);

      expect(errors).toHaveLength(1);
      expect(errors[0].severity).toBe(ErrorSeverity.CRITICAL);
      expect(errors[0].message).toContain('Dangerous functions');
      expect(errors[0].fixable).toBe(false);
    });

    it('should detect self-loops as critical errors', () => {
      const handler = new ErrorHandler();
      const code = 'A --> A\nB --> C';

      const errors = handler.validate(code);

      const selfLoopErrors = errors.filter(e => e.message.includes('Self-loop'));
      expect(selfLoopErrors).toHaveLength(1);
      expect(selfLoopErrors[0].severity).toBe(ErrorSeverity.CRITICAL);
      expect(selfLoopErrors[0].line).toBe(1);
      expect(selfLoopErrors[0].fixable).toBe(true);
    });

    it('should detect sub-model reuse as adjustable errors', () => {
      const handler = new ErrorHandler();
      const code = 'partial_order(base_model) = extended_model';

      const errors = handler.validate(code);

      expect(errors).toHaveLength(1);
      expect(errors[0].severity).toBe(ErrorSeverity.ADJUSTABLE);
      expect(errors[0].message).toContain('Sub-model reuse');
      expect(errors[0].fixable).toBe(true);
    });

    it('should detect multiple errors of different types', () => {
      const handler = new ErrorHandler();
      const code = `
import pandas as pd
A --> A
partial_order(model) = new_model
`;

      const errors = handler.validate(code);

      expect(errors.length).toBeGreaterThanOrEqual(3);

      const criticalErrors = errors.filter(e => e.severity === ErrorSeverity.CRITICAL);
      const adjustableErrors = errors.filter(e => e.severity === ErrorSeverity.ADJUSTABLE);

      expect(criticalErrors.length).toBeGreaterThanOrEqual(2); // import + self-loop
      expect(adjustableErrors.length).toBeGreaterThanOrEqual(1); // sub-model reuse
    });

    it('should return empty array for valid code', () => {
      const handler = new ErrorHandler();
      const code = `
A --> B
B --> C
model_copy = model.copy()
`;

      const errors = handler.validate(code);

      expect(errors).toHaveLength(0);
    });
  });

  describe('canAutoResolve', () => {
    it('should return false for critical errors', () => {
      const handler = new ErrorHandler();
      const criticalError: LLMError = {
        message: 'External import detected',
        severity: ErrorSeverity.CRITICAL,
        fixable: true
      };

      expect(handler.canAutoResolve(criticalError)).toBe(false);
    });

    it('should return false for non-fixable adjustable errors', () => {
      const handler = new ErrorHandler();
      const adjustableError: LLMError = {
        message: 'Some error',
        severity: ErrorSeverity.ADJUSTABLE,
        fixable: false
      };

      expect(handler.canAutoResolve(adjustableError)).toBe(false);
    });

    it('should return true for fixable adjustable errors', () => {
      const handler = new ErrorHandler();
      const adjustableError: LLMError = {
        message: 'Sub-model reuse detected',
        severity: ErrorSeverity.ADJUSTABLE,
        fixable: true
      };

      expect(handler.canAutoResolve(adjustableError)).toBe(true);
    });
  });

  describe('autoFix', () => {
    it('should fix sub-model reuse by adding .copy()', () => {
      const handler = new ErrorHandler();
      const code = 'partial_order(base_model) = extended_model';
      const error: LLMError = {
        message: 'Sub-model reuse detected (should call copy())',
        severity: ErrorSeverity.ADJUSTABLE,
        fixable: true
      };

      const fixed = handler.autoFix(code, error);

      expect(fixed).toContain('base_model.copy()');
      expect(fixed).not.toContain('partial_order(base_model)');
    });

    it('should fix self-loops by removing the offending line', () => {
      const handler = new ErrorHandler();
      const code = 'A --> A\nB --> C\nC --> D';
      const error: LLMError = {
        message: 'Self-loop detected: A violates irreflexivity',
        severity: ErrorSeverity.CRITICAL,
        line: 1,
        fixable: true
      };

      const fixed = handler.autoFix(code, error);

      expect(fixed).not.toContain('A --> A');
      expect(fixed).toContain('B --> C');
      expect(fixed).toContain('C --> D');
    });

    it('should return unchanged code for unhandled errors', () => {
      const handler = new ErrorHandler();
      const code = 'some code here';
      const error: LLMError = {
        message: 'Unknown error',
        severity: ErrorSeverity.ADJUSTABLE,
        fixable: true
      };

      const fixed = handler.autoFix(code, error);

      expect(fixed).toBe(code);
    });
  });

  describe('generateRefinementPrompt', () => {
    it('should include error details and conversation context', () => {
      const handler = new ErrorHandler();
      const error: LLMError = {
        message: 'Self-loop detected',
        severity: ErrorSeverity.CRITICAL,
        line: 5,
        fixable: true
      };
      const conversation: ConversationMessage[] = [
        { role: 'user', content: 'Generate a model' },
        { role: 'assistant', content: 'Here is your model' }
      ];

      const prompt = handler.generateRefinementPrompt(error, conversation);

      expect(prompt).toContain('ERROR DETECTED: Self-loop detected');
      expect(prompt).toContain('Line 5');
      expect(prompt).toContain('Severity: critical');
      expect(prompt).toContain('CONVERSATION HISTORY:');
      expect(prompt).toContain('user: Generate a model');
      expect(prompt).toContain('assistant: Here is your model');
      expect(prompt).toContain('No external imports');
      expect(prompt).toContain('All partial orders must be irreflexive');
    });
  });

  describe('extractCode', () => {
    it('should extract code from ```python blocks', () => {
      const handler = new ErrorHandler();
      const response = `
Some text here

\`\`\`python
A --> B
B --> C
\`\`\`

More text
`;

      const code = handler['extractCode'](response);

      expect(code).toContain('A --> B');
      expect(code).toContain('B --> C');
      expect(code).not.toContain('```');
      expect(code).not.toContain('Some text here');
    });

    it('should extract code from ``` blocks without language', () => {
      const handler = new ErrorHandler();
      const response = `
\`\`\`
A --> B
\`\`\`
`;

      const code = handler['extractCode'](response);

      expect(code).toContain('A --> B');
    });

    it('should return response as-is if no code blocks', () => {
      const handler = new ErrorHandler();
      const response = 'A --> B\nB --> C';

      const code = handler['extractCode'](response);

      expect(code).toBe(response);
    });
  });

  describe('handleErrors - integration tests', () => {
    it('should handle adjustable errors with auto-fix after max iterations', async () => {
      const config: ErrorHandlingConfig = {
        maxCriticalIterations: 5,
        maxAdjustableIterations: 1,
        autoResolveAfter: 1
      };
      const handler = new ErrorHandler(config);

      const code = 'partial_order(model) = new_model';
      const conversation: ConversationMessage[] = [
        { role: 'user', content: 'Generate a model' }
      ];

      let llmCallCount = 0;
      const mockLLMCall = vi.fn(async (_prompt: string) => {
        llmCallCount++;
        // Return code that still has the error to test auto-fix
        return '```python\npartial_order(model) = new_model\n```';
      });

      const result = await handler.handleErrors(code, conversation, mockLLMCall);

      // Should call LLM once, then auto-fix
      expect(llmCallCount).toBe(1);
      expect(result.success).toBe(true);
      expect(result.fixedCode).toContain('model.copy()');
      expect(result.iterations).toBe(1);
    });

    it('should fail after max critical iterations', async () => {
      const config: ErrorHandlingConfig = {
        maxCriticalIterations: 2,
        maxAdjustableIterations: 2,
        autoResolveAfter: 2
      };
      const handler = new ErrorHandler(config);

      const code = 'import numpy as np';
      const conversation: ConversationMessage[] = [
        { role: 'user', content: 'Generate a model' }
      ];

      const mockLLMCall = vi.fn(async (_prompt: string) => {
        // Keep returning code with import
        return '```python\nimport numpy as np\n```';
      });

      const result = await handler.handleErrors(code, conversation, mockLLMCall);

      expect(result.success).toBe(false);
      expect(result.iterations).toBe(2);
      expect(mockLLMCall).toHaveBeenCalledTimes(2);
    });

    it('should succeed when LLM fixes errors', async () => {
      const handler = new ErrorHandler();

      const code = 'A --> A';
      const conversation: ConversationMessage[] = [
        { role: 'user', content: 'Generate a model' }
      ];

      const mockLLMCall = vi.fn(async (_prompt: string) => {
        // Return fixed code on second attempt
        return '```python\nA --> B\nB --> C\n```';
      });

      const result = await handler.handleErrors(code, conversation, mockLLMCall);

      expect(result.success).toBe(true);
      expect(result.fixedCode).toContain('A --> B');
      expect(mockLLMCall).toHaveBeenCalled();
    });

    it('should handle multiple errors in sequence', async () => {
      const handler = new ErrorHandler();

      const code = `
import pandas as pd
A --> A
partial_order(model) = new_model
`;
      const conversation: ConversationMessage[] = [
        { role: 'user', content: 'Generate a model' }
      ];

      const callSequence: string[] = [];
      const mockLLMCall = vi.fn(async (prompt: string) => {
        callSequence.push(prompt);

        // Return progressively better code
        if (prompt.includes('External imports')) {
          return '```python\nA --> A\npartial_order(model) = new_model\n```';
        } else if (prompt.includes('Self-loop')) {
          return '```python\nA --> B\npartial_order(model) = new_model\n```';
        } else {
          return '```python\nA --> B\nmodel_copy = model.copy()\n```';
        }
      });

      const result = await handler.handleErrors(code, conversation, mockLLMCall);

      expect(result.success).toBe(true);
      expect(callSequence.length).toBeGreaterThan(0);
      expect(mockLLMCall).toHaveBeenCalled();
    });

    it('should return immediately if no errors present', async () => {
      const handler = new ErrorHandler();

      const code = 'A --> B\nB --> C';
      const conversation: ConversationMessage[] = [
        { role: 'user', content: 'Generate a model' }
      ];

      const mockLLMCall = vi.fn(async (_prompt: string) => {
        return '```python\nshould not be called\n```';
      });

      const result = await handler.handleErrors(code, conversation, mockLLMCall);

      expect(result.success).toBe(true);
      expect(result.fixedCode).toBe(code);
      expect(result.iterations).toBe(0);
      expect(mockLLMCall).not.toHaveBeenCalled();
    });
  });

  describe('configuration management', () => {
    it('should get default config', () => {
      const handler = new ErrorHandler();
      const config = handler.getConfig();

      expect(config.maxCriticalIterations).toBe(5);
      expect(config.maxAdjustableIterations).toBe(2);
      expect(config.autoResolveAfter).toBe(2);
    });

    it('should use custom config', () => {
      const customConfig: ErrorHandlingConfig = {
        maxCriticalIterations: 10,
        maxAdjustableIterations: 5,
        autoResolveAfter: 3
      };
      const handler = new ErrorHandler(customConfig);
      const config = handler.getConfig();

      expect(config.maxCriticalIterations).toBe(10);
      expect(config.maxAdjustableIterations).toBe(5);
      expect(config.autoResolveAfter).toBe(3);
    });

    it('should update config partially', () => {
      const handler = new ErrorHandler();
      handler.updateConfig({ maxCriticalIterations: 15 });

      const config = handler.getConfig();
      expect(config.maxCriticalIterations).toBe(15);
      expect(config.maxAdjustableIterations).toBe(2); // unchanged
      expect(config.autoResolveAfter).toBe(2); // unchanged
    });
  });
});
