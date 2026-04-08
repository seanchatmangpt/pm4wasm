/**
 * Example usage of prompt template library
 */

import {
  PROMPT_TEMPLATES,
  buildPrompt,
  buildRefinementPrompt,
  getTemplatesByCategory,
  getTemplateByName,
  getCategories,
  type PromptTemplate
} from './prompt-templates';

/**
 * Example 1: Get a simple template and build a prompt
 */
export function exampleSimplePrompt(): string {
  const template = getTemplateByName('Simple Sequential');
  if (!template) {
    throw new Error('Template not found');
  }

  return buildPrompt(template);
}

/**
 * Example 2: Get templates by category
 */
export function exampleParallelTemplates(): PromptTemplate[] {
  return getTemplatesByCategory('parallel');
}

/**
 * Example 3: Build a refinement prompt
 */
export function exampleRefinementPrompt(): string {
  const originalDescription = `
  Document approval process:
  1. Submit document
  2. Review document
  3. Approve or reject
  4. Archive if approved, notify if rejected
  `;

  const feedback = 'The model needs to handle the case where review is skipped';
  const history = 'User: Create a document approval model\nAI: [Generated model without skip case]';

  return buildRefinementPrompt(originalDescription, feedback, history);
}

/**
 * Example 4: List all available categories
 */
export function exampleListCategories(): string[] {
  return getCategories();
}

/**
 * Example 5: Build prompt without examples (for faster generation)
 */
export function exampleMinimalPrompt(): string {
  const template = getTemplateByName('Loop with Retry');
  if (!template) {
    throw new Error('Template not found');
  }

  // Build without examples or negative prompts for shorter context
  return buildPrompt(template, false, false);
}

/**
 * Example 6: Display all available templates
 */
export function exampleListAllTemplates(): Array<{name: string; description: string; category: string}> {
  return PROMPT_TEMPLATES.map(t => ({
    name: t.name,
    description: t.description,
    category: t.category
  }));
}

// Console output examples (run with: node --loader ts-node src/prompt-templates-example.ts)
if (import.meta.url === `file://${process.argv[1]}`) {
  console.log('=== Example 1: Simple Sequential Template ===\n');
  console.log(exampleSimplePrompt());
  console.log('\n=== Example 2: Parallel Templates ===\n');
  console.log(exampleParallelTemplates());
  console.log('\n=== Example 3: Refinement Prompt ===\n');
  console.log(exampleRefinementPrompt());
  console.log('\n=== Example 4: All Categories ===\n');
  console.log(exampleListCategories());
  console.log('\n=== Example 5: Minimal Prompt ===\n');
  console.log(exampleMinimalPrompt());
  console.log('\n=== Example 6: All Available Templates ===\n');
  console.table(exampleListAllTemplates());
}
