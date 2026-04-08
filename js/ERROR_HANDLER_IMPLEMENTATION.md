# Error Handler Implementation Report

## Summary

Implemented a comprehensive error handling mechanism for the POWL (Partially Ordered Workflow Language) code generator, based on the paper "Process Modeling With Large Language Models". The implementation categorizes errors into two tiers and applies different resolution strategies.

## Files Created

1. **`/Users/sac/chatmangpt/pm4py/pm4wasm/js/src/error-handler.ts`** (395 lines)
   - Core error handling implementation
   - Exports `ErrorHandler` class, `ErrorSeverity` enum, and TypeScript types

2. **`/Users/sac/chatmangpt/pm4py/pm4wasm/js/src/error-handler.test.ts`** (468 lines)
   - Comprehensive test suite with 24 test cases
   - 100% pass rate (24/24 tests passing)

3. **`/Users/sac/chatmangpt/pm4py/pm4wasm/js/src/error-handler-example.ts`** (220 lines)
   - Six usage examples demonstrating different scenarios
   - Ready-to-run demonstrations

4. **`/Users/sac/chatmangpt/pm4py/pm4wasm/js/vitest.config.ts`**
   - Vitest configuration for TypeScript testing

## Key Features Implemented

### 1. Two-Tier Error Classification

**Critical Errors** (execution failures, security risks):
- External imports (`import numpy as np`)
- Dangerous functions (`eval()`, `exec()`)
- Self-loops violating irreflexivity (`A --> A`)
- Up to 5 LLM iterations before giving up

**Adjustable Errors** (model quality issues):
- Sub-model reuse without `.copy()` (`partial_order(model) = ...`)
- Up to 2 LLM iterations, then auto-resolve

### 2. Validation System

The `validate()` method checks for:
- **Security violations**: External imports, dangerous functions
- **Structural violations**: Self-loops (irreflexivity)
- **Quality issues**: Sub-model reuse patterns

```typescript
const errors = handler.validate(code);
// Returns: LLMError[] with severity, message, line number, fixable status
```

### 3. Auto-Fix Mechanism

Automatically fixes adjustable errors when LLM iterations exceed threshold:

```typescript
// Before: partial_order(model) = new_model
// After:  partial_order(model.copy()) = new_model
```

### 4. LLM Integration

Generates context-aware refinement prompts:
- Includes conversation history
- Specifies error type and location
- Provides clear guidance on fixes

### 5. Retry Logic

Implements smart retry strategy:
1. Try LLM for adjustable errors (up to maxAdjustableIterations)
2. Auto-fix if still failing after threshold
3. Handle critical errors with LLM (up to maxCriticalIterations)
4. Re-validate after each iteration
5. Return success status and fixed code

## Test Coverage

All 24 tests passing:

**Validation Tests (6 tests)**
- ✓ Detects external imports as critical
- ✓ Detects eval/exec as critical
- ✓ Detects self-loops as critical
- ✓ Detects sub-model reuse as adjustable
- ✓ Detects multiple error types
- ✓ Returns empty array for valid code

**Auto-Resolution Tests (3 tests)**
- ✓ Returns false for critical errors
- ✓ Returns false for non-fixable adjustable errors
- ✓ Returns true for fixable adjustable errors

**Auto-Fix Tests (3 tests)**
- ✓ Fixes sub-model reuse with `.copy()`
- ✓ Fixes self-loops by removing lines
- ✓ Returns unchanged code for unhandled errors

**Integration Tests (5 tests)**
- ✓ Handles adjustable errors with auto-fix after max iterations
- ✓ Fails after max critical iterations
- ✓ Succeeds when LLM fixes errors
- ✓ Handles multiple errors in sequence
- ✓ Returns immediately if no errors present

**Configuration Tests (3 tests)**
- ✓ Gets default config
- ✓ Uses custom config
- ✓ Updates config partially

**Other Tests (4 tests)**
- ✓ Generates refinement prompts with context
- ✓ Extracts code from ```python blocks
- ✓ Extracts code from ``` blocks without language
- ✓ Returns response as-is if no code blocks

## Usage Example

```typescript
import { ErrorHandler } from './error-handler';

const handler = new ErrorHandler({
  maxCriticalIterations: 5,
  maxAdjustableIterations: 2,
  autoResolveAfter: 2
});

const code = `
import numpy as np
A --> A
partial_order(model) = new_model
`;

const conversation = [
  { role: 'user', content: 'Generate a POWL model' }
];

const result = await handler.handleErrors(
  code,
  conversation,
  llmCallFunction
);

console.log(result.success);     // true if all errors fixed
console.log(result.fixedCode);   // corrected code
console.log(result.iterations);  // number of LLM calls made
```

## Configuration Options

```typescript
interface ErrorHandlingConfig {
  maxCriticalIterations: number;      // Default: 5
  maxAdjustableIterations: number;    // Default: 2
  autoResolveAfter: number;           // Default: 2
}
```

## Integration with Existing Code

The error handler integrates seamlessly with the existing POWL codebase:
- Located in `/Users/sac/chatmangpt/pm4py/pm4wasm/js/src/`
- Uses TypeScript with strict type checking
- Compatible with existing test infrastructure (vitest)
- Follows project code standards

## Testing

Run tests with:
```bash
cd /Users/sac/chatmangpt/pm4py/pm4wasm/js
npm run test:ts -- error-handler.test.ts
```

Result: **24/24 tests passing** ✅

## Paper Compliance

This implementation follows the error handling strategy from "Process Modeling With Large Language Models":

1. **Critical Errors**: Execution failures, security risks, major validation violations
   - Requires up to 5 iterations with LLM before giving up
   - Examples: syntax errors, undefined functions, external library usage

2. **Adjustable Errors**: Model quality issues that can be auto-fixed
   - Up to 2 iterations with LLM, then auto-resolve
   - Examples: sub-model reuse, minor validation issues

## Next Steps

The error handler is ready for integration with the LLM-powered POWL generator. To use:

1. Import `ErrorHandler` in your LLM integration code
2. Validate generated code before returning to user
3. Use `handleErrors()` to automatically fix issues
4. Present fixed code or error messages to user

---

**Implementation Date**: April 6, 2026
**Test Status**: All 24 tests passing ✅
**Lines of Code**: 1,083 (implementation + tests + examples)
**Files**: 4 created
