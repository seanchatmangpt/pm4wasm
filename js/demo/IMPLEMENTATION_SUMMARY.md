# Implementation Summary: Interactive Refinement Loop

## Overview

This implementation provides the **interactive refinement loop** for user feedback from the paper *"Process Modeling With Large Language Models"* (Kourani et al., 2024).

## What Was Implemented

### 1. Core Modules

#### **`refinement-loop.ts`** (New)
Implements the interactive refinement session management:
- `RefinementLoop` class for tracking feedback and conversation history
- `addFeedback()` - Add user feedback to session
- `refineModel()` - Perform refinement iteration with LLM
- `generateRefinementPrompt()` - Create context-aware prompts
- `exportSession()` / `importSession()` - Persist and restore sessions
- `createSummaryReport()` - Generate human-readable session summary

**Key Data Structures:**
```typescript
interface UserFeedback {
  type: 'text' | 'visual';
  content: string;
  timestamp: Date;
}

interface RefinementSession {
  originalDescription: string;
  currentModel: PowlModel | null;
  feedbackHistory: UserFeedback[];
  conversationHistory: Array<{role: string; content: string}>;
  iterationCount: number;
}
```

#### **`process-modeling-service.ts`** (New)
High-level service orchestrating the complete workflow:
- `ProcessModelingService` class
- `generateModel()` - Generate initial model from description
- `startRefinementSession()` - Begin interactive refinement
- `addFeedbackAndRefine()` - Incorporate feedback and update model
- `validateModel()` - Check conformance against event logs
- `generateRefinementReport()` - Get session summary

**Key Features:**
- Integrates LLM calls, error handling, and refinement
- Manages conversation history for context
- Supports multiple LLM providers (OpenAI, Anthropic, etc.)
- Configurable iteration limits and auto-fix behavior

#### **`error-handler.ts`** (Already Existed)
Implements the two-tier error handling strategy from the paper:
- **Critical Errors**: Up to 5 LLM iterations (syntax errors, security issues)
- **Adjustable Errors**: Up to 2 LLM iterations, then auto-fix (model quality issues)

**Methods:**
- `validate()` - Categorize errors by severity
- `handleErrors()` - Retry with automatic refinement
- `autoFix()` - Apply automatic fixes for adjustable errors
- `generateRefinementPrompt()` - Create error-specific prompts

#### **`llm-prompts.ts`** (Already Existed)
Comprehensive prompt templates implementing four prompting strategies:
1. **Role Prompting**: Assign LLM the role of process modeling expert
2. **Knowledge Injection**: Provide POWL language syntax and rules
3. **Few-Shot Learning**: Include input/output examples
4. **Negative Prompting**: Specify common errors to avoid

**Templates:**
- `SYSTEM_PROMPT` - Role and knowledge injection
- `EXAMPLES` - Few-shot learning examples from paper
- `COMMON_ERRORS` - Negative prompting with anti-patterns
- `ERROR_REFINEMENT` - Error-specific refinement prompts
- `VALIDATION_FEEDBACK` - Validation result feedback

### 2. Demo Files

#### **`refinement-loop-demo.ts`** (New)
TypeScript demo showing the complete workflow:
- `demoRefinementWorkflow()` - Generate → Feedback → Refine → Report
- `demoErrorHandling()` - Automatic error detection and fixing
- `demoConformanceChecking()` - Model validation against event logs
- `mockLLMCall()` - Mock LLM for demo purposes

#### **`refinement.html`** (New)
Interactive HTML demo page:
- **Input Panel**: Process description and user feedback
- **Model Display**: Current POWL model visualization
- **Feedback History**: Track all feedback items
- **Statistics**: Iterations, feedback count, model size
- **Buttons**: Generate, Refine, Reset

**Features:**
- Gradient purple theme matching modern design
- Responsive layout (mobile-friendly)
- Real-time model updates
- Visual feedback history
- Mock LLM integration (replaceable with real API)

#### **`README.md`** (New)
Comprehensive documentation:
- Architecture overview
- Usage examples
- API reference
- LLM integration guide
- Configuration options
- Persistence support

### 3. Integration

#### **`index.ts`** (Updated)
Added exports for new modules:
```typescript
export * from "./refinement-loop.js";
export * from "./error-handler.js";
export * from "./process-modeling-service.js";
```

## Key Features from the Paper

### 1. Four Prompting Strategies ✅
All four strategies implemented in `llm-prompts.ts`:
- Role Prompting: "You are an expert in process modeling..."
- Knowledge Injection: POWL construction functions and rules
- Few-Shot Learning: 3 detailed examples from paper
- Negative Prompting: 6 common errors with fixes

### 2. Two-Tier Error Handling ✅
Implemented in `error-handler.ts`:
- **Critical Errors**: Syntax errors, security issues, self-loops
  - Up to 5 LLM iterations
  - No auto-fix (requires human intervention)
- **Adjustable Errors**: Sub-model reuse, transitivity issues
  - Up to 2 LLM iterations
  - Auto-fix after threshold

### 3. Interactive Refinement Loop ✅
Implemented in `refinement-loop.ts`:
- Track feedback history
- Maintain conversation context
- Generate refinement prompts
- Parse updated models
- Export/import sessions

### 4. Complete Workflow ✅
Implemented in `process-modeling-service.ts`:
1. Generate initial model from description
2. Validate and handle errors
3. Collect user feedback
4. Refine model iteratively
5. Validate against event logs
6. Generate summary reports

## Usage Example

```typescript
import { ProcessModelingService } from './process-modeling-service.js';

// Create service with LLM integration
const service = new ProcessModelingService(async (prompt) => {
  const response = await fetch('https://api.anthropic.com/v1/messages', {
    method: 'POST',
    headers: {
      'Content-Type': 'application/json',
      'x-api-key': API_KEY,
      'anthropic-version': '2023-06-01'
    },
    body: JSON.stringify({
      model: 'claude-3-opus-20240229',
      max_tokens: 4096,
      messages: [{ role: 'user', content: prompt }]
    })
  });
  const data = await response.json();
  return data.content[0].text;
});

// Generate initial model
const result = await service.generateModel(`
A loan application process:
1. Customer submits application
2. Bank performs credit check and employment verification
3. Loan is approved or rejected based on results
`);

// Start refinement session
service.startRefinementSession(description, result.model);

// Add feedback and refine
const refinement = await service.addFeedbackAndRefine({
  type: 'text',
  content: 'Add a document review step after submission',
  timestamp: new Date()
});

// Get session summary
console.log(service.generateRefinementReport());
```

## File Structure

```
pm4wasm/js/
├── src/
│   ├── index.ts (updated)
│   ├── refinement-loop.ts (new)
│   ├── process-modeling-service.ts (new)
│   ├── error-handler.ts (existed)
│   ├── llm-prompts.ts (existed)
│   ├── types.ts (existed)
│   └── utils.ts (existed)
└── demo/
    ├── refinement-loop-demo.ts (new)
    ├── refinement.html (new)
    └── README.md (new)
```

## Testing the Implementation

### 1. Run the TypeScript Demo
```bash
cd pm4wasm/js
npm install
npm run build
npm run demo
```

### 2. Open the HTML Demo
```bash
cd pm4wasm/js/demo
# Open refinement.html in a browser
# Or use a simple HTTP server:
python -m http.server 8000
# Then visit http://localhost:8000/refinement.html
```

### 3. Test with Real LLM
Replace the `mockLLMCall` function in the demo with actual API calls:
- OpenAI: `https://api.openai.com/v1/chat/completions`
- Anthropic: `https://api.anthropic.com/v1/messages`
- Other providers: Adapt the request format

## Next Steps

### For Production Use:
1. **Replace mock LLM** with actual API integration
2. **Add authentication** for LLM providers
3. **Implement rate limiting** for API calls
4. **Add persistence** (database or file system)
5. **Create user interface** (extend the HTML demo)
6. **Add tests** (unit tests for all modules)
7. **Optimize prompts** based on your specific use case
8. **Add monitoring** (track iterations, errors, success rates)

### For Research:
1. **Collect metrics** on refinement iterations
2. **Analyze feedback patterns** across users
3. **Compare LLM providers** (OpenAI vs Anthropic vs others)
4. **Measure model quality** improvements
5. **Study error categories** and their frequency
6. **Evaluate prompt strategies** effectiveness

## References

- **Paper**: Kourani, et al. "Process Modeling With Large Language Models" (2024)
- **POWL**: Partially Ordered Workflow Language
- **Error Handling**: Two-tier strategy (Critical vs. Adjustable errors)
- **Prompting**: Role, Knowledge, Few-Shot, Negative Prompting

## License

AGPL-3.0 (same as pm4py)
