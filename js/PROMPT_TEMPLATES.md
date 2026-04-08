# Prompt Template Library for Process Modeling

A comprehensive library of prompt templates for LLM-based process modeling, based on "Process Modeling With Large Language Models" (Kourani et al., 2024).

## Overview

This library provides:
- **10+ pre-built templates** covering common BPM patterns
- **System prompts** with POWL language reference and soundness requirements
- **Few-shot examples** for different process patterns
- **Negative prompts** to prevent common errors
- **Refinement prompts** for iterative model improvement

## Installation

The templates are included in the `@pm4py/pm4wasm` package:

```typescript
import {
  PROMPT_TEMPLATES,
  buildPrompt,
  buildRefinementPrompt,
  getTemplatesByCategory,
  getTemplateByName
} from '@pm4py/pm4wasm/prompt-templates';
```

## Quick Start

```typescript
import { buildPrompt, getTemplateByName } from '@pm4py/pm4wasm/prompt-templates';

// Get a template
const template = getTemplateByName('Simple Sequential');

// Build a complete prompt
const prompt = buildPrompt(template);

// Send to LLM
const response = await llm.generate(prompt);
```

## Available Templates

### Simple Patterns

- **Simple Sequential** - Basic linear workflow
- **Simple Approval** - Approval with decision point

### Choice Patterns (XOR)

- **Parallel Gateway** - Parallel activities with choices
- **Simple Approval** - Basic approve/reject workflow

### Loop Patterns

- **Loop with Retry** - Retry mechanism with escape
- **Data Validation Loop** - Do-while validation loop
- **Quality Control Loop** - Repeat-until quality check

### Parallel Patterns

- **Parallel Gateway** - Concurrent activities
- **Parallel Document Processing** - Multiple parallel reviews

### Complex Patterns

- **Hotel Room Service** - Multi-stage with parallel activities (from paper)
- **Bicycle Manufacturing** - Manufacturing with quality control (from paper)
- **Complex Order Fulfillment** - Nested parallel and choice

## Template Categories

Templates are organized by category:

```typescript
import { getTemplatesByCategory } from '@pm4py/pm4wasm/prompt-templates';

// Get all simple templates
const simple = getTemplatesByCategory('simple');

// Get all complex templates
const complex = getTemplatesByCategory('complex');

// Get all templates with loops
const withLoops = getTemplatesByCategory('with-loops');

// Get all templates with choices
const withChoices = getTemplatesByCategory('with-choices');

// Get all parallel templates
const parallel = getTemplatesByCategory('parallel');
```

## Building Prompts

### Standard Prompt (with examples and warnings)

```typescript
const prompt = buildPrompt(template);
// Includes: system prompt + process description + examples + negative prompts
```

### Minimal Prompt (faster, less context)

```typescript
const prompt = buildPrompt(template, false, false);
// Includes: system prompt + process description only
```

### Custom Prompt

```typescript
const prompt = buildPrompt(template, true, false);
// Include examples but exclude negative prompts
```

## Refinement Prompts

When an LLM generates an invalid model, use refinement prompts to guide correction:

```typescript
import { buildRefinementPrompt } from '@pm4py/pm4wasm/prompt-templates';

const refinementPrompt = buildRefinementPrompt(
  originalProcessDescription,
  userFeedback,
  conversationHistory
);

// Example:
const prompt = buildRefinementPrompt(
  'Document approval: submit → review → approve/reject → archive',
  'The model has a self-loop in the partial order',
  'User: Create model\nAI: [generated model with error]'
);
```

## System Prompt Components

The system prompt includes:

### 1. POWL Language Reference
- `activity(label)` - Create activity nodes
- `xor(...args)` - Exclusive choice
- `loop(do, redo)` - Loop construct
- `partial_order(dependencies)` - Concurrent activities
- `sequence(...args)` - Sequential composition

### 2. Soundness Requirements
- **Irreflexivity**: No self-loops (A→A is invalid)
- **Transitivity**: If A→B and B→C, then A→C must be stated
- **Proper Completion**: All paths must terminate

### 3. Common Mistakes
- Self-loops in partial orders
- Missing transitivity edges
- Local choices instead of path-level choices
- Reusing sub-models without copying
- External imports

### 4. Few-Shot Examples
- Sequential pattern example
- Choice (XOR) pattern example
- Parallel pattern example

### 5. Negative Prompts
- DO NOT / WHY / FIX structure for each error
- Clear explanations of soundness violations

## Template Structure

Each template includes:

```typescript
interface PromptTemplate {
  name: string;              // Human-readable name
  description: string;       // What the template demonstrates
  category: 'simple' | 'complex' | 'with-loops' | 'with-choices' | 'parallel';
  processDescription: string; // Natural language process description
  expectedPattern: string;   // Expected POWL pattern
}
```

## Usage Examples

### Example 1: Get Simple Template

```typescript
import { getTemplateByName, buildPrompt } from '@pm4py/pm4wasm/prompt-templates';

const template = getTemplateByName('Simple Sequential');
const prompt = buildPrompt(template);
```

### Example 2: Get All Parallel Templates

```typescript
import { getTemplatesByCategory } from '@pm4py/pm4wasm/prompt-templates';

const parallelTemplates = getTemplatesByCategory('parallel');
console.log(parallelTemplates.map(t => t.name));
// Output: ['Parallel Gateway', 'Parallel Document Processing']
```

### Example 3: Build Refinement Prompt

```typescript
import { buildRefinementPrompt } from '@pm4py/pm4wasm/prompt-templates';

const prompt = buildRefinementPrompt(
  'Order processing: check inventory → process payment → ship',
  'Add error handling for payment failures',
  'Previous model did not handle payment errors'
);
```

### Example 4: List All Templates

```typescript
import { PROMPT_TEMPLATES } from '@pm4py/pm4wasm/prompt-templates';

PROMPT_TEMPLATES.forEach(template => {
  console.log(`${template.name}: ${template.description}`);
});
```

## Testing

Run the test suite:

```bash
cd js
npm run test:ts -- src/prompt-templates.test.ts
```

All 37 tests should pass, covering:
- Template structure and content
- System prompt components
- Few-shot examples
- Negative prompts
- Prompt building logic
- Refinement prompts
- Category filtering
- Edge cases

## Research Foundation

This library is based on the paper:

> **Process Modeling With Large Language Models**
> Kourani et al., 2024
> https://arxiv.org/abs/2403.14006

The templates incorporate:
- Process descriptions from the paper's evaluation
- Soundness requirements from formal process modeling theory
- Common error patterns identified in LLM-generated models
- Best practices for prompt engineering with code generation

## Contributing

To add new templates:

1. Create a template following the `PromptTemplate` interface
2. Add to `PROMPT_TEMPLATES` array in `src/prompt-templates.ts`
3. Add tests in `src/prompt-templates.test.ts`
4. Run tests to ensure quality: `npm run test:ts`

## License

AGPL-3.0 (same as pm4py-core)
