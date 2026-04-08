# Interactive Refinement Loop Implementation

This implementation provides the **interactive refinement loop** for user feedback from the paper *"Process Modeling With Large Language Models"* (Kourani et al., 2024).

## Overview

The refinement loop enables users to:
1. **View** the generated process model
2. **Provide feedback** (text or visual annotations)
3. **Get an updated model** that incorporates their feedback
4. **Iterate** until satisfied with the model

## Architecture

### Core Components

#### 1. **RefinementLoop** (`refinement-loop.ts`)
Manages the interactive refinement session:
- Tracks feedback history
- Maintains conversation context
- Generates refinement prompts
- Parses updated models from LLM responses
- Exports/imports session state for persistence

**Key Features:**
```typescript
- addFeedback(feedback: UserFeedback): Add user feedback
- refineModel(llmCall, basePrompt): Perform refinement iteration
- getSessionSummary(): Get session statistics
- exportSession(): Persist session to JSON
- createSummaryReport(): Generate human-readable summary
```

#### 2. **ProcessModelingService** (`process-modeling-service.ts`)
High-level service orchestrating the complete workflow:
- Initial model generation with error handling
- Refinement session management
- Conformance checking
- Configuration management

**Key Features:**
```typescript
- generateModel(description): Generate initial model
- startRefinementSession(description, model): Begin refinement
- addFeedbackAndRefine(feedback): Incorporate feedback and refine
- validateModel(model, log): Check conformance
- generateRefinementReport(): Get session summary
```

#### 3. **ErrorHandler** (`error-handler.ts`)
Implements the two-tier error handling strategy from the paper:
- **Critical Errors**: Up to 5 LLM iterations (syntax errors, security issues)
- **Adjustable Errors**: Up to 2 LLM iterations, then auto-fix (model quality issues)

**Key Features:**
```typescript
- validate(code): Categorize errors by severity
- handleErrors(code, conversation, llmCall): Retry with refinement
- autoFix(code, error): Apply automatic fixes
```

#### 4. **LLM Prompts** (`llm-prompts.ts`)
Comprehensive prompt templates implementing the four prompting strategies:
1. **Role Prompting**: Assign LLM the role of process modeling expert
2. **Knowledge Injection**: Provide POWL language syntax and rules
3. **Few-Shot Learning**: Include input/output examples
4. **Negative Prompting**: Specify common errors to avoid

## Usage

### Basic Workflow

```typescript
import { ProcessModelingService } from './process-modeling-service.js';
import { UserFeedback } from './refinement-loop.js';

// 1. Create service with LLM integration
const service = new ProcessModelingService(async (prompt) => {
  // Call your LLM API here (OpenAI, Anthropic, etc.)
  const response = await fetch('https://api.example.com/v1/chat', {
    method: 'POST',
    body: JSON.stringify({ prompt })
  });
  const data = await response.json();
  return data.completion;
});

// 2. Generate initial model
const description = `
A loan application process:
1. Customer submits application
2. Bank performs credit check and employment verification
3. Loan is approved or rejected based on results
`;

const result = await service.generateModel(description);

if (result.model) {
  console.log("Initial model:", result.model.toString());

  // 3. Start refinement session
  service.startRefinementSession(description, result.model);

  // 4. Collect user feedback
  const feedback: UserFeedback = {
    type: 'text',
    content: 'Missing document review step after submission',
    timestamp: new Date()
  };

  // 5. Refine model based on feedback
  const refinement = await service.addFeedbackAndRefine(feedback);

  if (refinement.success) {
    console.log("Refined model:", refinement.model.toString());
  }

  // 6. Get session summary
  console.log(service.generateRefinementReport());
}
```

### Error Handling

The service automatically handles errors with the two-tier strategy:

```typescript
const result = await service.generateModel(description);

console.log("Iterations:", result.iterations);
console.log("Errors:", result.errors);
console.log("Warnings:", result.warnings);
```

### Conformance Checking

Validate models against event logs:

```typescript
const log = `case_id,activity,timestamp
1,A,2024-01-01T10:00:00Z
1,B,2024-01-01T10:05:00Z
1,C,2024-01-01T10:10:00Z
`;

const validation = await service.validateModel(model, log);
console.log("Fitness:", validation.percentage);
console.log("Avg trace fitness:", validation.avgTraceFitness);
```

## Demo

Run the interactive demo:

```bash
cd js
npm install
npm run build
npm run demo
```

The demo (`demo/refinement-loop-demo.ts`) shows:
1. **Complete refinement workflow**: Generate → Feedback → Refine → Report
2. **Error handling**: Automatic error detection and fixing
3. **Conformance checking**: Model validation against event logs

## Key Features from the Paper

### 1. Four Prompting Strategies

All four strategies from the paper are implemented:

```typescript
import { PROMPT_TEMPLATES } from './llm-prompts.js';

// Strategy 1: Role Prompting
console.log(PROMPT_TEMPLATES.SYSTEM_PROMPT);

// Strategy 2: Knowledge Injection
// (included in SYSTEM_PROMPT)

// Strategy 3: Few-Shot Learning
console.log(PROMPT_TEMPLATES.EXAMPLES);

// Strategy 4: Negative Prompting
console.log(PROMPT_TEMPLATES.COMMON_ERRORS);
```

### 2. Two-Tier Error Handling

```typescript
import { ErrorHandler, ErrorSeverity } from './error-handler.js';

const handler = new ErrorHandler({
  maxCriticalIterations: 5,
  maxAdjustableIterations: 2,
  autoResolveAfter: 2
});

// Critical errors: Up to 5 LLM iterations
// Adjustable errors: Up to 2 LLM iterations, then auto-fix
```

### 3. Interactive Refinement Loop

```typescript
import { RefinementLoop, UserFeedback } from './refinement-loop.js';

const loop = new RefinementLoop(description);

// Add feedback
loop.addFeedback({
  type: 'text',
  content: 'Add approval step',
  timestamp: new Date()
});

// Refine model
const result = await loop.refineModel(llmCall, basePrompt);

// Get summary
const summary = loop.getSessionSummary();
console.log(summary);
```

## Data Structures

### UserFeedback
```typescript
interface UserFeedback {
  type: 'text' | 'visual';
  content: string;
  timestamp: Date;
}
```

### RefinementSession
```typescript
interface RefinementSession {
  originalDescription: string;
  currentModel: PowlModel | null;
  feedbackHistory: UserFeedback[];
  conversationHistory: Array<{role: string; content: string}>;
  iterationCount: number;
}
```

### ModelGenerationResult
```typescript
interface ModelGenerationResult {
  model: PowlModel | null;
  iterations: number;
  errors: string[];
  warnings: string[];
  conversationHistory: Array<{role: string; content: string}>;
}
```

## Configuration

```typescript
const service = new ProcessModelingService(llmCall, {
  maxRefinementIterations: 5,    // Max refinement iterations
  autoFixErrors: true,            // Enable auto-fixing adjustable errors
  enableConformanceChecking: true // Enable conformance validation
});
```

## Persistence

Refinement sessions can be exported and imported:

```typescript
// Export session
const json = loop.exportSession();
localStorage.setItem('refinement-session', json);

// Import session
const saved = localStorage.getItem('refinement-session');
const restored = RefinementLoop.importSession(saved);
```

## Integration with LLM Providers

### OpenAI Example
```typescript
const service = new ProcessModelingService(async (prompt) => {
  const response = await fetch('https://api.openai.com/v1/chat/completions', {
    method: 'POST',
    headers: {
      'Content-Type': 'application/json',
      'Authorization': `Bearer ${API_KEY}`
    },
    body: JSON.stringify({
      model: 'gpt-4',
      messages: [{ role: 'user', content: prompt }]
    })
  });
  const data = await response.json();
  return data.choices[0].message.content;
});
```

### Anthropic Example
```typescript
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
```

## Testing

Run the test suite:

```bash
npm test
```

## References

- **Paper**: Kourani, et al. "Process Modeling With Large Language Models" (2024)
- **POWL**: Partially Ordered Workflow Language
- **Error Handling**: Two-tier strategy (Critical vs. Adjustable errors)
- **Prompting**: Role, Knowledge, Few-Shot, Negative Prompting

## License

AGPL-3.0 (same as pm4py)
