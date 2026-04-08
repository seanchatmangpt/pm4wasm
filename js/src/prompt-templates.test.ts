/**
 * Tests for prompt template library
 */

import {
  PROMPT_TEMPLATES,
  SYSTEM_PROMPT,
  FEW_SHOT_EXAMPLES,
  NEGATIVE_PROMPTS,
  buildPrompt,
  buildRefinementPrompt,
  getTemplatesByCategory,
  getTemplateByName,
  getCategories,
  type PromptTemplate
} from './prompt-templates';

describe('Prompt Templates Library', () => {
  describe('PROMPT_TEMPLATES', () => {
    test('should have at least 10 templates', () => {
      expect(PROMPT_TEMPLATES.length).toBeGreaterThanOrEqual(10);
    });

    test('all templates should have required fields', () => {
      PROMPT_TEMPLATES.forEach(template => {
        expect(template).toHaveProperty('name');
        expect(template).toHaveProperty('description');
        expect(template).toHaveProperty('category');
        expect(template).toHaveProperty('processDescription');
        expect(template).toHaveProperty('expectedPattern');

        expect(typeof template.name).toBe('string');
        expect(typeof template.description).toBe('string');
        expect(typeof template.processDescription).toBe('string');
        expect(typeof template.expectedPattern).toBe('string');
        expect(['simple', 'complex', 'with-loops', 'with-choices', 'parallel']).toContain(template.category);
      });
    });

    test('template names should be unique', () => {
      const names = PROMPT_TEMPLATES.map(t => t.name);
      const uniqueNames = new Set(names);
      expect(uniqueNames.size).toBe(names.length);
    });
  });

  describe('SYSTEM_PROMPT', () => {
    test('should contain POWL language reference', () => {
      expect(SYSTEM_PROMPT).toContain('POWL Language Reference');
      expect(SYSTEM_PROMPT).toContain('activity(label)');
      expect(SYSTEM_PROMPT).toContain('xor');
      expect(SYSTEM_PROMPT).toContain('loop');
      expect(SYSTEM_PROMPT).toContain('partial_order');
      expect(SYSTEM_PROMPT).toContain('sequence');
    });

    test('should contain soundness requirements', () => {
      expect(SYSTEM_PROMPT).toContain('Irreflexivity');
      expect(SYSTEM_PROMPT).toContain('Transitivity');
      expect(SYSTEM_PROMPT).toContain('Proper Completion');
    });

    test('should contain common mistakes section', () => {
      expect(SYSTEM_PROMPT).toContain('Common Mistakes to Avoid');
      expect(SYSTEM_PROMPT).toContain('Self-loops');
      expect(SYSTEM_PROMPT).toContain('Missing transitivity');
    });

    test('should specify output format', () => {
      expect(SYSTEM_PROMPT).toContain('Output Format');
      expect(SYSTEM_PROMPT).toContain('ModelGenerator');
    });
  });

  describe('FEW_SHOT_EXAMPLES', () => {
    test('should have examples for different patterns', () => {
      expect(FEW_SHOT_EXAMPLES).toHaveProperty('sequential');
      expect(FEW_SHOT_EXAMPLES).toHaveProperty('choice');
      expect(FEW_SHOT_EXAMPLES).toHaveProperty('parallel');
    });

    test('examples should contain code blocks', () => {
      Object.values(FEW_SHOT_EXAMPLES).forEach(example => {
        expect(example).toContain('```python');
        expect(example).toContain('```');
      });
    });

    test('sequential example should use sequence', () => {
      expect(FEW_SHOT_EXAMPLES.sequential).toContain('sequence');
    });

    test('choice example should use xor', () => {
      expect(FEW_SHOT_EXAMPLES.choice).toContain('xor');
    });

    test('parallel example should use partial_order', () => {
      expect(FEW_SHOT_EXAMPLES.parallel).toContain('partial_order');
    });
  });

  describe('NEGATIVE_PROMPTS', () => {
    test('should list common errors', () => {
      expect(NEGATIVE_PROMPTS).toContain('Common Errors to AVOID');
      expect(NEGATIVE_PROMPTS).toContain('Self-loops');
      expect(NEGATIVE_PROMPTS).toContain('Missing transitivity');
      expect(NEGATIVE_PROMPTS).toContain('Local choices');
      expect(NEGATIVE_PROMPTS).toContain('External imports');
    });

    test('should provide DO NOT and WHY sections for each error', () => {
      expect(NEGATIVE_PROMPTS).toContain('DO NOT');
      expect(NEGATIVE_PROMPTS).toContain('WHY');
      expect(NEGATIVE_PROMPTS).toContain('FIX');
    });
  });

  describe('buildPrompt', () => {
    test('should build prompt with all components by default', () => {
      const template = PROMPT_TEMPLATES[0];
      const prompt = buildPrompt(template);

      expect(prompt).toContain(SYSTEM_PROMPT);
      expect(prompt).toContain('## Process Description');
      expect(prompt).toContain(template.processDescription);
      expect(prompt).toContain('## Example');
      expect(prompt).toContain('Common Errors to AVOID');
      expect(prompt).toContain('Please generate the POWL model code');
    });

    test('should build prompt without examples when disabled', () => {
      const template = PROMPT_TEMPLATES[0];
      const prompt = buildPrompt(template, false, true);

      expect(prompt).toContain(SYSTEM_PROMPT);
      expect(prompt).toContain('## Process Description');
      expect(prompt).not.toContain('## Example');
      expect(prompt).toContain('Common Errors to AVOID');
    });

    test('should build prompt without negative prompts when disabled', () => {
      const template = PROMPT_TEMPLATES[0];
      const prompt = buildPrompt(template, true, false);

      expect(prompt).toContain(SYSTEM_PROMPT);
      expect(prompt).toContain('## Process Description');
      expect(prompt).toContain('## Example');
      expect(prompt).not.toContain('Common Errors to AVOID');
    });

    test('should build minimal prompt when both disabled', () => {
      const template = PROMPT_TEMPLATES[0];
      const prompt = buildPrompt(template, false, false);

      expect(prompt).toContain(SYSTEM_PROMPT);
      expect(prompt).toContain('## Process Description');
      expect(prompt).not.toContain('## Example');
      expect(prompt).not.toContain('Common Errors to AVOID');
      expect(prompt).toContain('Please generate the POWL model code');
    });

    test('should handle different template categories', () => {
      const categories = getCategories();
      categories.forEach(category => {
        const templates = getTemplatesByCategory(category);
        if (templates.length > 0) {
          const prompt = buildPrompt(templates[0]);
          expect(prompt.length).toBeGreaterThan(0);
          expect(prompt).toContain(templates[0].processDescription);
        }
      });
    });
  });

  describe('buildRefinementPrompt', () => {
    test('should build refinement prompt with all components', () => {
      const originalDescription = 'Original process description';
      const feedback = 'The model needs to include error handling';
      const history = 'User: Create a model\nAI: Here is the model';

      const prompt = buildRefinementPrompt(originalDescription, feedback, history);

      expect(prompt).toContain(SYSTEM_PROMPT);
      expect(prompt).toContain('## Original Process');
      expect(prompt).toContain(originalDescription);
      expect(prompt).toContain('## User Feedback');
      expect(prompt).toContain(feedback);
      expect(prompt).toContain('## Conversation History');
      expect(prompt).toContain(history);
      expect(prompt).toContain('Please update the POWL model');
    });

    test('should maintain system prompt in refinement', () => {
      const prompt = buildRefinementPrompt('desc', 'feedback', 'history');
      expect(prompt).toContain('POWL Language Reference');
      expect(prompt).toContain('Soundness Requirements');
    });
  });

  describe('getTemplatesByCategory', () => {
    test('should return templates for valid categories', () => {
      const simpleTemplates = getTemplatesByCategory('simple');
      expect(Array.isArray(simpleTemplates)).toBe(true);
      simpleTemplates.forEach(template => {
        expect(template.category).toBe('simple');
      });
    });

    test('should return empty array for non-existent category', () => {
      const templates = getTemplatesByCategory('non-existent' as any);
      expect(templates).toEqual([]);
    });

    test('should return all categories', () => {
      const categories = getCategories();
      expect(categories).toContain('simple');
      expect(categories).toContain('complex');
      expect(categories).toContain('with-loops');
      expect(categories).toContain('with-choices');
      expect(categories).toContain('parallel');
    });

    test('should have templates in each category', () => {
      const categories = getCategories();
      categories.forEach(category => {
        const templates = getTemplatesByCategory(category);
        expect(templates.length).toBeGreaterThan(0);
      });
    });
  });

  describe('getTemplateByName', () => {
    test('should return template for valid name', () => {
      const template = getTemplateByName('Simple Sequential');
      expect(template).toBeDefined();
      expect(template?.name).toBe('Simple Sequential');
      expect(template?.category).toBe('simple');
    });

    test('should return undefined for non-existent name', () => {
      const template = getTemplateByName('Non-existent Template');
      expect(template).toBeUndefined();
    });

    test('should find templates with different names', () => {
      const names = ['Simple Sequential', 'Parallel Gateway', 'Loop with Retry'];
      names.forEach(name => {
        const template = getTemplateByName(name);
        expect(template).toBeDefined();
        expect(template?.name).toBe(name);
      });
    });
  });

  describe('Template Content Quality', () => {
    test('process descriptions should be detailed', () => {
      PROMPT_TEMPLATES.forEach(template => {
        // Should have multiple steps
        const lines = template.processDescription.split('\n').filter(line => line.trim());
        expect(lines.length).toBeGreaterThanOrEqual(3);

        // Should contain numbered steps or clear structure
        const hasStructure = lines.some(line => /^\d+\./.test(line) ||
                                        line.includes('1.') ||
                                        line.includes('step'));
        expect(hasStructure || template.category === 'simple').toBe(true);
      });
    });

    test('expected patterns should be descriptive', () => {
      PROMPT_TEMPLATES.forEach(template => {
        expect(template.expectedPattern.length).toBeGreaterThan(5);
      });
    });

    test('templates from paper should be marked complex', () => {
      const hotelTemplate = getTemplateByName('Hotel Room Service');
      expect(hotelTemplate?.category).toBe('complex');

      const bikeTemplate = getTemplateByName('Bicycle Manufacturing');
      expect(bikeTemplate?.category).toBe('complex');
    });
  });

  describe('Prompt Integration', () => {
    test('built prompts should be properly formatted', () => {
      const template = PROMPT_TEMPLATES[0];
      const prompt = buildPrompt(template);

      // Should have clear section headers
      expect(prompt).toMatch(/##\s+\w+/);

      // Should have proper line breaks
      expect(prompt).toContain('\n\n');

      // Should not have excessive blank lines (allow up to 5)
      const blankLines = prompt.split('\n\n\n').length - 1;
      expect(blankLines).toBeLessThan(5);
    });

    test('built prompts should be complete', () => {
      const template = PROMPT_TEMPLATES[0];
      const prompt = buildPrompt(template);

      // Should start with system prompt
      expect(prompt.startsWith('You are an expert')).toBe(true);

      // Should contain the request to generate code
      expect(prompt).toContain('Please generate the POWL model code');
    });

    test('refinement prompts should include all context', () => {
      const prompt = buildRefinementPrompt(
        'Process: A -> B -> C',
        'Add error handling',
        'Model created without errors'
      );

      expect(prompt).toContain('Process: A -> B -> C');
      expect(prompt).toContain('Add error handling');
      expect(prompt).toContain('Model created without errors');
    });
  });

  describe('Edge Cases', () => {
    test('should handle empty feedback in refinement', () => {
      const prompt = buildRefinementPrompt('desc', '', 'history');
      expect(prompt).toContain('## User Feedback');
      expect(prompt.length).toBeGreaterThan(0);
    });

    test('should handle empty history in refinement', () => {
      const prompt = buildRefinementPrompt('desc', 'feedback', '');
      expect(prompt).toContain('## Conversation History');
      expect(prompt.length).toBeGreaterThan(0);
    });

    test('should handle very long process descriptions', () => {
      const longDesc = 'Step 1\n'.repeat(100);
      const template: PromptTemplate = {
        name: 'Long Process',
        description: 'Test',
        category: 'complex',
        processDescription: longDesc,
        expectedPattern: 'complex'
      };

      const prompt = buildPrompt(template);
      expect(prompt).toContain(longDesc);
    });
  });
});
