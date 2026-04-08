# Implementation Summary: Prompt Template Library

## What Was Implemented

Created a comprehensive prompt template library for LLM-based process modeling in the POWL WASM project, based on the research paper "Process Modeling With Large Language Models" (Kourani et al., 2024).

## Files Created

### 1. Core Library (`js/src/prompt-templates.ts`)
**Location:** `/Users/sac/chatmangpt/pm4py/pm4wasm/js/src/prompt-templates.ts`

**Components:**
- **10+ prompt templates** covering common BPM patterns
- **System prompt** with POWL language reference and soundness requirements
- **Few-shot examples** for sequential, choice, and parallel patterns
- **Negative prompts** preventing common LLM errors
- **Builder functions** for customizing prompts
- **Filtering utilities** by category and name

**Key Features:**
- Templates organized by category: simple, complex, with-loops, with-choices, parallel
- Process descriptions from the research paper (Hotel Room Service, Bicycle Manufacturing)
- Soundness requirements: irreflexivity, transitivity, proper completion
- Common error patterns with DO NOT/WHY/FIX structure

### 2. Comprehensive Test Suite (`js/src/prompt-templates.test.ts`)
**Location:** `/Users/sac/chatmangpt/pm4py/pm4wasm/js/src/prompt-templates.test.ts`

**Test Coverage (37 tests, all passing):**
- Template structure validation
- System prompt component verification
- Few-shot example correctness
- Negative prompt completeness
- Prompt building logic (all variations)
- Refinement prompt generation
- Category filtering
- Name-based lookup
- Template content quality
- Edge case handling

**Test Results:**
```
✓ 37/37 tests passing
✓ 0 failures
✓ Duration: ~180ms
```

### 3. Usage Examples (`js/src/prompt-templates-example.ts`)
**Location:** `/Users/sac/chatmangpt/pm4py/pm4wasm/js/src/prompt-templates-example.ts`

**Examples Include:**
- Building simple prompts
- Getting templates by category
- Creating refinement prompts
- Listing all available categories
- Building minimal prompts (faster generation)
- Displaying all available templates

### 4. Documentation (`js/PROMPT_TEMPLATES.md`)
**Location:** `/Users/sac/chatmangpt/pm4py/pm4wasm/js/PROMPT_TEMPLATES.md`

**Documentation Covers:**
- Quick start guide
- Available templates (all 10+)
- Template categories
- Building prompts (standard, minimal, custom)
- Refinement prompts
- System prompt components
- Template structure
- Usage examples
- Testing instructions
- Research foundation

### 5. Updated Package Configuration
**File:** `/Users/sac/chatmangpt/pm4py/pm4wasm/js/package.json`

**Changes:**
- Added `vitest` and `@vitest/ui` as dev dependencies
- Added `test:ts` script for TypeScript tests
- Added `test:watch` script for development
- Updated `test` script to run both TypeScript and WASM tests

## Template Library Contents

### Templates by Category

**Simple (2 templates):**
1. Simple Sequential - Basic linear workflow
2. Simple Approval - Approval with decision point

**With Loops (3 templates):**
1. Loop with Retry - Retry mechanism with escape
2. Data Validation Loop - Do-while validation
3. Quality Control Loop - Repeat-until quality check

**With Choices (1 template):**
1. Simple Approval - Basic approve/reject workflow

**Parallel (2 templates):**
1. Parallel Gateway - Concurrent activities
2. Parallel Document Processing - Multiple parallel reviews

**Complex (3 templates):**
1. Hotel Room Service - Multi-stage with parallel activities (from paper)
2. Bicycle Manufacturing - Manufacturing with quality control (from paper)
3. Complex Order Fulfillment - Nested parallel and choice

**Total: 10+ templates**

### System Prompt Components

1. **POWL Language Reference**
   - `activity(label)` - Create activity nodes
   - `xor(...args)` - Exclusive choice (n >= 2)
   - `loop(do, redo)` - Loop construct
   - `partial_order(dependencies)` - Concurrent activities
   - `sequence(...args)` - Sequential composition

2. **Soundness Requirements**
   - Irreflexivity: No self-loops (A→A invalid)
   - Transitivity: If A→B and B→C, then A→C required
   - Proper Completion: All paths must terminate

3. **Common Mistakes (5 errors covered)**
   - Self-loops in partial orders
   - Missing transitivity edges
   - Local choices instead of path choices
   - Reusing sub-models without copying
   - External imports

4. **Few-Shot Examples (3 patterns)**
   - Sequential: Document approval
   - Choice: Loan approval
   - Parallel: Order handling

## API Reference

### Main Functions

```typescript
// Build complete prompt
buildPrompt(template, includeExamples?, includeNegativePrompts?): string

// Build refinement prompt
buildRefinementPrompt(originalDescription, feedback, conversationHistory): string

// Get templates by category
getTemplatesByCategory(category): PromptTemplate[]

// Get template by name
getTemplateByName(name): PromptTemplate | undefined

// Get all categories
getCategories(): string[]
```

### Data Structures

```typescript
interface PromptTemplate {
  name: string;
  description: string;
  category: 'simple' | 'complex' | 'with-loops' | 'with-choices' | 'parallel';
  processDescription: string;
  expectedPattern: string;
}
```

## Research Foundation

Based on the paper:
> **Process Modeling With Large Language Models**
> Kourani et al., 2024
> https://arxiv.org/abs/2403.14006

The templates incorporate:
- Process descriptions from the paper's evaluation (Hotel Room Service, Bicycle Manufacturing)
- Soundness requirements from formal process modeling theory
- Common error patterns identified in LLM-generated models
- Best practices for prompt engineering with code generation

## Testing Results

All 37 tests passing:
- ✓ Template structure validation
- ✓ System prompt components
- ✓ Few-shot examples
- ✓ Negative prompts
- ✓ Prompt building logic
- ✓ Refinement prompts
- ✓ Category filtering
- ✓ Name-based lookup
- ✓ Template content quality
- ✓ Edge cases

Test suite can be run with:
```bash
cd pm4wasm/js
npm run test:ts -- src/prompt-templates.test.ts
```

## Usage Example

```typescript
import {
  getTemplateByName,
  buildPrompt,
  getTemplatesByCategory
} from '@pm4py/pm4wasm/prompt-templates';

// Get a specific template
const template = getTemplateByName('Hotel Room Service');

// Build a complete prompt with examples and warnings
const prompt = buildPrompt(template);

// Or get all parallel templates
const parallelTemplates = getTemplatesByCategory('parallel');
```

## Key Features

1. **Research-Based**: Templates from published paper on LLM process modeling
2. **Comprehensive**: Covers simple, complex, loop, choice, and parallel patterns
3. **Soundness-Focused**: Explicit requirements for valid process models
4. **Error-Prevention**: Negative prompts guide LLMs away from common mistakes
5. **Customizable**: Build prompts with/without examples and warnings
6. **Refinement-Support**: Iterative improvement prompts for fixing invalid models
7. **Well-Tested**: 37 tests ensure quality and correctness
8. **Type-Safe**: Full TypeScript types and interfaces

## Integration with POWL WASM

This library integrates seamlessly with the existing POWL WASM codebase:
- Located in `js/src/` alongside existing TypeScript files
- Uses same tooling (vitest, TypeScript)
- Follows same code style and conventions
- Ready for use in browser-based process modeling tools
