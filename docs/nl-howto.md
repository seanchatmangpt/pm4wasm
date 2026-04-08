# How-To Guide: NL → POWL Patterns and Recipes

**What this is:** Practical recipes for common NL→POWL tasks. Assumes you've completed the [tutorial](./nl-tutorial.md).

---

## Write Effective Process Descriptions

### Do

```
A customer submits an order. The order is validated by the system.
If the order is valid, items are picked, packed, and billed concurrently.
If the order is invalid, the customer is notified and the order is cancelled.
After fulfillment, a confirmation email is sent.
```

- Use specific activity names ("Submit Order" not "do stuff")
- Explicitly state choices ("If valid... If invalid...")
- Explicitly state parallelism ("picked, packed, and billed concurrently")
- Mention loops explicitly ("retry", "repeat", "loop back")

### Don't

```
Some stuff happens with orders and then shipping and maybe returns.
```

- Vague descriptions produce vague models
- Missing decision points create incorrect control flow
- Ambiguous parallelism leads to wrong operator selection

---

## Recipe: Simple XOR Choice

**Pattern:** "If condition, do A. Otherwise, do B."

```
A loan application is reviewed. If approved, funds are disbursed.
If rejected, a rejection letter is sent.
```

**Expected POWL:** `X('Disburse Funds', 'Send Rejection Letter')`

---

## Recipe: Loop with Retry

**Pattern:** "Do X. If it fails, retry X up to N times."

```
A document is submitted for review. If the review fails,
corrections are requested and the document is resubmitted.
```

**Expected POWL:** `*('Submit Document', 'Request Corrections')`

---

## Recipe: Parallel Activities (Partial Order)

**Pattern:** "A and B happen at the same time / concurrently."

```
An order is picked from the warehouse, packed into a box,
and billed to the customer. These three steps happen concurrently.
```

**Expected POWL:** `PO=(nodes={'Pick', 'Pack', 'Bill'}, order={})`

---

## Recipe: Nested Choice Inside Parallel

**Pattern:** Parallel activities, each with their own choices.

```
An order is picked and packed concurrently. During picking,
if the item is out of stock, a backorder is placed.
During packing, if the item is fragile, special packaging is used.
```

**Expected POWL:**
```
PO=( nodes={
    X('Pick Item', 'Place Backorder'),
    X('Standard Pack', 'Fragile Pack')
  }, order={} )
```

---

## Recipe: Multi-Agent Orchestration (A2A+MCP)

**Pattern:** Human-in-the-loop with agent coordination.

```
A human submits a task to a swarm orchestrator. Agents report
capabilities. The orchestrator assigns subtasks. Agents execute
and report results. If an agent is silent, escalate to human.
The human approves or requests revision.
```

**Key modeling decisions:**
- "If silent, escalate" → LOOP with escalation as redo
- "Approve or request revision" → XOR
- "Report capabilities" then "assign" → sequential in PO

---

## Recipe: Escalation Loop

**Pattern:** "Try X. If timeout/failure, escalate to Y. Y decides what to do."

```
A support ticket is handled by a chatbot. If the chatbot cannot
resolve the issue within 3 attempts, the ticket is escalated to
a human agent. The human agent resolves the issue or escalates
to a manager.
```

**Expected POWL:** `*('Chatbot Attempt', 'Escalate to Human')` nested inside a larger structure.

---

## Configure the Generation

### Use More Refinements

```python
result = generate_powl_from_text(description, max_refinements=5)
```

Default is 2. More refinements give the judge more chances to correct issues but take longer.

### Disable Few-Shot Demos

```python
result = generate_powl_from_text(description, use_demos=False)
```

Disabling demos may produce lower-quality results on the first attempt.

### Use a Different LLM Provider

```bash
export LLM_MODEL="groq/llama-3.3-70b-versatile"
export LLM_API_KEY="your-key"
```

Or use OpenAI:
```bash
export LLM_MODEL="openai/gpt-4o"
export LLM_API_KEY="your-openai-key"
```

---

## Handle Common Errors

### "Model not parseable"

The LLM generated invalid POWL syntax. This usually means:
- Unbalanced parentheses
- Missing commas between elements
- Invalid operator syntax

**Fix:** The refinement loop handles this automatically. If it persists, try simplifying the description.

### "Wrong operator used (X vs PO)"

The LLM used `PO` (all successors complete) when you meant `X` (exactly one).

**Fix:** Be more explicit in your description: "either A or B, not both" → clearly signals XOR.

### "BPMN conversion failed"

The direct POWL→BPMN path failed for a complex model.

**Fix:** The pipeline automatically falls back to POWL→Petri Net→BPMN. If both fail, simplify the description.

---

## See Also

- [Tutorial](./nl-tutorial.md) — Step-by-step walkthrough
- [Explanation](./nl-explanation.md) — How the pipeline works internally
- [Reference](./nl-reference.md) — Complete API
