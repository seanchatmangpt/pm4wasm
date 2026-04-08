/**
 * Comprehensive prompt template library for LLM-based process modeling
 * Based on "Process Modeling With Large Language Models" (Kourani et al., 2024)
 */

export interface PromptTemplate {
  name: string;
  description: string;
  category: 'simple' | 'complex' | 'with-loops' | 'with-choices' | 'parallel';
  processDescription: string;
  expectedPattern: string;
}

export const PROMPT_TEMPLATES: PromptTemplate[] = [
  // Simple sequential process
  {
    name: 'Simple Sequential',
    description: 'A simple linear process with sequential activities',
    category: 'simple',
    processDescription: `A document approval process where:
1. An employee submits a document
2. The manager reviews the document
3. If approved, the document is archived
4. If rejected, the employee is notified`,
    expectedPattern: 'sequence'
  },

  // Process with choice (XOR)
  {
    name: 'Parallel Gateway',
    description: 'Process with parallel activities that can happen simultaneously',
    category: 'parallel',
    processDescription: `Order handling process:
1. Customer logs into online shop
2. Customer selects items to purchase
3. Simultaneously: customer sets payment method AND customer chooses reward option
4. Customer pays OR completes installment agreement
5. Items are delivered
6. Customer can return items for exchange`,
    expectedPattern: 'partial_order with concurrency'
  },

  // Process with loop
  {
    name: 'Loop with Retry',
    description: 'Process with retry mechanism',
    category: 'with-loops',
    processDescription: `Payment processing process:
1. Initiate payment
2. Process payment
3. If payment fails, retry (up to 3 times)
4. If all retries fail, escalate to manual review
5. If payment succeeds, send confirmation
6. Complete order`,
    expectedPattern: 'loop with escape'
  },

  // Complex nested structure
  {
    name: 'Hotel Room Service',
    description: 'Multi-stage process with parallel activities (from paper)',
    category: 'complex',
    processDescription: `The Evanstonian hotel room service process:
1. Guest calls room service
2. Manager takes order and submits ticket to kitchen
3. Manager gives order to sommelier to fetch wine
4. Manager assigns order to waiter
5. Meanwhile: kitchen prepares food AND sommelier fetches wine
6. Waiter readies cart
7. When food, wine, and cart are ready, waiter delivers to guest
8. Waiter returns to station and debits guest account
9. Waiter may delay billing if has another order`,
    expectedPattern: 'complex partial order'
  },

  // Manufacturing with quality check
  {
    name: 'Bicycle Manufacturing',
    description: 'Manufacturing process with quality control (from paper)',
    category: 'complex',
    processDescription: `Bicycle manufacturing process:
1. Create process instance
2. Reject order OR accept order
3. Inform storehouse and engineering department
4. Process part list
5. Check required quantity of part
6. Reserve part OR back-order part
7. Check part reservation
8. Prepare bicycle assembly
9. Assemble bicycle
10. Ship bicycle
11. Finish process instance`,
    expectedPattern: 'complex with choices and loops'
  },

  // Additional template: Simple approval
  {
    name: 'Simple Approval',
    description: 'Basic approval workflow with decision',
    category: 'with-choices',
    processDescription: `Purchase request approval:
1. Employee submits purchase request
2. Manager reviews request
3. Manager approves OR rejects request
4. If approved, finance processes payment
5. If rejected, employee is notified
6. Process completes`,
    expectedPattern: 'xor choice'
  },

  // Additional template: Loop with validation
  {
    name: 'Data Validation Loop',
    description: 'Process with validation and retry loop',
    category: 'with-loops',
    processDescription: `Data entry validation process:
1. User starts data entry
2. User enters data
3. System validates data
4. If validation fails, return to step 2
5. If validation succeeds, save data
6. Send confirmation
7. Process completes`,
    expectedPattern: 'do-while loop'
  },

  // Additional template: Parallel processing
  {
    name: 'Parallel Document Processing',
    description: 'Process with concurrent document reviews',
    category: 'parallel',
    processDescription: `Document review process:
1. Submit document for review
2. Legal review begins
3. Technical review begins (in parallel with legal)
4. Financial review begins (in parallel with legal and technical)
5. Wait for all reviews to complete
6. If all reviews approve, document is accepted
7. If any review rejects, document is returned
8. Process completes`,
    expectedPattern: 'parallel join'
  },

  // Additional template: Nested complexity
  {
    name: 'Complex Order Fulfillment',
    description: 'Multi-layered process with nested decisions and parallel steps',
    category: 'complex',
    processDescription: `E-commerce order fulfillment:
1. Customer places order
2. Check inventory
3. If in stock: reserve items
4. If out of stock: backorder OR cancel
5. Process payment
6. If payment succeeds: pack items AND print shipping label (in parallel)
7. If payment fails: notify customer AND cancel order
8. Ship package
9. Send tracking information
10. Process completes`,
    expectedPattern: 'nested parallel and choice'
  },

  // Additional template: Simple loop
  {
    name: 'Quality Control Loop',
    description: 'Process with quality check and rework loop',
    category: 'with-loops',
    processDescription: `Manufacturing quality control:
1. Start manufacturing process
2. Produce item
3. Perform quality check
4. If quality check fails: rework item, then return to step 3
5. If quality check passes: approve item
6. Ship item
7. Process completes`,
    expectedPattern: 'repeat-until loop'
  }
];

/**
 * System prompt with role and knowledge injection
 */
export const SYSTEM_PROMPT = `You are an expert in Business Process Management (BPM) and process modeling.

Your task is to generate process models using POWL (Partially Ordered Workflow Language).

## POWL Language Reference

POWL provides the following constructors:

1. **activity(label)** - Creates an activity node
   - Example: \`activity("Submit Document")\`

2. **xor(...args)** - Exclusive choice (OR)
   - Creates a choice between alternative paths
   - Requires 2 or more arguments
   - Example: \`xor(activity("Accept"), activity("Reject"))\`

3. **loop(do, redo)** - Loop construct
   - Creates a repeating pattern
   - \`do\` is the repeated activity
   - \`redo\` is optional (null if no redo)
   - Example: \`loop(activity("Retry"), null)\`

4. **partial_order(dependencies)** - Partial order with explicit ordering
   - Creates concurrent activities with ordering constraints
   - Dependencies is a list of [from, to] pairs
   - Example: \`partial_order(dependencies=[("A", "B"), ("A", "C")])\`

5. **sequence(...args)** - Sequential composition
   - Creates a strict sequence of activities
   - Example: \`sequence(activity("A"), activity("B"), activity("C"))\`

## Critical Rules

### Soundness Requirements
- **Irreflexivity**: No activity can precede itself (no A->A edges)
- **Transitivity**: If A->B and B->C, then A->C must be explicitly stated
- **Proper Completion**: All paths must reach an end state

### Code Quality
- No external imports (only use provided functions)
- No eval() or exec() calls
- Use .copy() when reusing sub-models to avoid aliasing issues

### Common Mistakes to Avoid
1. Creating local choices instead of path-level choices
   - Wrong: \`xor(activity("A"), activity("B"))\` as sub-process
   - Right: Use xor() at the path level, not for local decisions

2. Missing transitivity edges
   - Wrong: \`dependencies=[("A", "B"), ("B", "C")]\`
   - Right: \`dependencies=[("A", "B"), ("B", "C"), ("A", "C")]\`

3. Self-loops in partial orders
   - Wrong: \`dependencies=[("A", "A")]\`
   - Right: Never include self-loops

## Output Format

Generate Python code that uses these functions to create the POWL model:

\`\`\`python
from utils.model_generation import ModelGenerator

gen = ModelGenerator()

# Build your model here using the constructors above
# Example:
# model = gen.activity("Start")

# Return the final model
\`\`\`
`;

/**
 * Few-shot examples for different patterns
 */
export const FEW_SHOT_EXAMPLES = {
  sequential: `
Process: Simple document approval
Model:
\`\`\`python
gen = ModelGenerator()
submit = gen.activity("Submit Document")
review = gen.activity("Review Document")
approve = gen.activity("Approve")
archive = gen.activity("Archive Document")
reject = gen.activity("Notify Employee")
choice = gen.xor(approve, reject)
model = gen.sequence(submit, review, choice, archive)
\`\`\`
`,

  choice: `
Process: Loan approval
Model:
\`\`\`python
gen = ModelGenerator()
apply = gen.activity("Apply Loan")
assess = gen.activity("Assess Credit")
approve = gen.activity("Approve Loan")
reject = gen.activity("Reject Loan")
notify = gen.activity("Notify Applicant")
decision = gen.xor(approve, reject)
model = gen.sequence(apply, assess, decision, notify)
\`\`\`
`,

  parallel: `
Process: Order handling with parallel steps
Model:
\`\`\`python
gen = ModelGenerator()
login = gen.activity("Login")
select = gen.activity("Select Items")
payment = gen.activity("Set Payment")
reward = gen.activity("Choose Reward")
pay = gen.activity("Pay")
installment = gen.activity("Installment Agreement")
deliver = gen.activity("Deliver Items")
parallel = gen.partial_order(dependencies=[(select, pay), (select, installment)])
payment_choice = gen.xor(pay, installment)
order = gen.partial_order(dependencies=[(login, select), (login, payment)])
model = gen.partial_order(dependencies=[(order, parallel), (parallel, reward), (payment_choice, deliver)])
\`\`\`
`
};

/**
 * Negative prompts for common errors
 */
export const NEGATIVE_PROMPTS = `
## Common Errors to AVOID

1. **Self-loops in partial orders**
   - DO NOT: \`dependencies=[("A", "A")]\`
   - WHY: Violates irreflexivity requirement
   - FIX: Remove self-loops

2. **Missing transitivity edges**
   - DO NOT: \`dependencies=[("A", "B"), ("B", "C")]\` without A->C
   - WHY: Violates transitivity requirement
   - FIX: Add \`dependencies=[("A", "C")]\`

3. **Local choices instead of path choices**
   - DO NOT: Use xor() for local activity selection
   - WHY: Creates unsound models with dead ends
   - FIX: Use xor() at the path level, encompassing complete alternative paths

4. **Reusing sub-models without copying**
   - DO NOT: Use the same sub-model variable twice
   - WHY: Creates aliasing issues
   - FIX: Call .copy() before reusing

5. **External imports**
   - DO NOT: import any libraries
   - WHY: Security risk and violates sandboxing
   - FIX: Only use provided ModelGenerator functions
`;

/**
 * Build complete prompt from template
 */
export function buildPrompt(
  template: PromptTemplate,
  includeExamples: boolean = true,
  includeNegativePrompts: boolean = true
): string {
  let prompt = SYSTEM_PROMPT;

  prompt += `\n## Process Description\n\n${template.processDescription}\n`;

  if (includeExamples) {
    prompt += `\n## Example\n\n${FEW_SHOT_EXAMPLES.sequential}\n`;
  }

  if (includeNegativePrompts) {
    prompt += `\n${NEGATIVE_PROMPTS}\n`;
  }

  prompt += `\n\nPlease generate the POWL model code for this process.\n`;

  return prompt;
}

/**
 * Get refinement prompt for incorporating user feedback
 */
export function buildRefinementPrompt(
  originalDescription: string,
  feedback: string,
  conversationHistory: string
): string {
  return `${SYSTEM_PROMPT}

## Original Process
${originalDescription}

## User Feedback
${feedback}

## Conversation History
${conversationHistory}

Please update the POWL model to address the user's feedback while maintaining all previous improvements.
`;
}

/**
 * Get templates by category
 */
export function getTemplatesByCategory(category: PromptTemplate['category']): PromptTemplate[] {
  return PROMPT_TEMPLATES.filter(t => t.category === category);
}

/**
 * Get template by name
 */
export function getTemplateByName(name: string): PromptTemplate | undefined {
  return PROMPT_TEMPLATES.find(t => t.name === name);
}

/**
 * Get all template categories
 */
export function getCategories(): PromptTemplate['category'][] {
  return Array.from(new Set(PROMPT_TEMPLATES.map(t => t.category)));
}
