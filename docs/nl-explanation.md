# Explanation: How NL → POWL → BPMN Works

**What this is:** The conceptual and technical explanation of the NL→POWL→BPMN pipeline. For how-to, see the [tutorial](./nl-tutorial.md) and [recipes](./nl-howto.md).

---

## The Paradigm Shift

Process mining has historically operated on one input modality: **event logs**. Discovery algorithms extract process models from observed behavior, measured against fitness, precision, generalization, and simplicity.

But most processes are never logged. A startup designing its first workflow has no event log. A compliance team documenting a new regulation works from a standard operating procedure. A systems architect designing multi-agent orchestration works from a specification.

**The NL→POWL→BPMN pipeline addresses this gap:** natural language → verified formal model → executable BPMN.

---

## Why POWL v2, Not Process Trees or BPMN Directly

| Requirement | Process Trees | BPMN | POWL v2 |
|---|---|---|---|
| Express non-block-structured patterns | No | Yes | Yes |
| Formal semantics for verification | Yes | No | Yes |
| String representation for LLM generation | Yes | No | Yes |
| XOR vs. PO syntactically distinct | Partial | No | Yes |
| Converts to BPMN | Possible | Native | Yes |
| Soundness guarantees | Yes | No | Yes |

POWL v2 is the only formalism satisfying all requirements. Process trees lack expressiveness. BPMN lacks formal semantics. POWL v2 provides both.

---

## The Three Stages

### Stage 1: Generation

A DSPy ReAct agent generates a POWL model string from the natural language description. The agent:

1. **Parses** the description to extract activities and control-flow patterns
2. **Maps** linguistic cues to POWL operators:
   - "if/else", "either/or" → `X()` (XOR)
   - "repeat", "retry", "again" → `*()` (LOOP)
   - "concurrently", "in parallel" → `PO=()` (partial order)
   - "then", "after", "before" → edges in `order`
3. **Validates** via `validate_powl()` tool call
4. **Returns** via `finish()` tool call

The agent uses few-shot demonstrations that teach correct operator selection through example trajectories. The most important lesson: **in a `PO=()`, all outgoing edges mean all successors must complete. If only one should execute, use `X()`.**

### Stage 2: Verification

The `POWLJudge` evaluates the POWL on four criteria:

| Criterion | Question |
|---|---|
| **Syntactic validity** | Is the POWL string well-formed? |
| **Structural soundness** | Is it deadlock-free and live? |
| **Behavioral plausibility** | Are the right operators used? |
| **Modeling quality** | Is it appropriately abstract? |

The judge returns True/False with reasoning. It does NOT compare against a ground truth model—it evaluates quality in isolation. This is essential because in NL process discovery, there is no single "correct" model.

### Stage 3: Refinement

When the judge rejects a model, its reasoning is appended to the original description:

```
PREVIOUS ATTEMPT REJECTED.
Issues: In the low-urgency branch, 'Review Results' has two outgoing edges
but only one should execute (medication OR surgery). Use X() not multiple
edges in PO.
```

The agent re-generates with this augmented description. Empirical results show that even complex processes (21 activities, 4 XOR decisions, 3 feedback loops) pass verification on the first or second attempt.

---

## The XOR vs. PO Distinction

This is the single most important concept in NL→POWL generation, and the most common source of errors.

**XOR (X):** Exactly one branch executes.
```
X('Prescribe Medication', 'Recommend Surgery')
→ Either prescribe medication OR recommend surgery, not both
```

**Partial Order (PO):** All branches execute (possibly in any order).
```
PO=(nodes={'Pick', 'Pack', 'Bill'}, order={})
→ Pick AND Pack AND Bill all execute (concurrently)
```

**The error:** An LLM reads "prescribe medication or recommend surgery" and creates two concurrent edges in a partial order (both execute). But "or" means XOR (exactly one). The judge catches this and the refinement loop corrects it.

**Why it matters:** If a BPMN model has parallel execution where exclusive choice was intended, the process will execute both branches, leading to incorrect behavior (e.g., prescribing medication AND performing surgery).

---

## Why Mathematical Verification Matters

Without verification, LLM-generated workflows can have:

- **Deadlocks:** A branch with no path to completion. The workflow hangs.
- **Improper completion:** A terminal state reachable only through activities that shouldn't be on that path.
- **Unbounded loops:** No escape condition, allowing infinite repetition.

These are 30-year-old formal properties from Petri net theory (Carl Adam Petri, 1962) and workflow soundness theory (van der Aalst, 1998). The NL→POWL pipeline applies them to AI-generated workflows for the first time.

The strategic implication: no other agent framework (LangChain, CrewAI, AutoGen, Claude Code, OpenAI Swarm) can verify workflow correctness. This creates an unassailable competitive advantage (see the [PhD thesis](../../docs/powl_v2_thesis.md) for the full Porter's Five Forces analysis).

---

## The Few-Shot Demonstrations

Four demonstrations teach the agent correct POWL generation:

1. **Loan approval** — XOR for approve/reject, LOOP for document retry
2. **Software release** — Multiple XOR decisions, LOOP for test-fix cycles
3. **E-commerce** — PO for concurrent activities, XOR for valid/cancel
4. **A2A+MCP swarm** — 21-activity multi-agent orchestration with 4 XOR decisions and 3 feedback loops

The demos progress from simple to complex, each demonstrating a specific operator selection lesson. Demo 4 (the most complex) passes verification on the first attempt, showing that the demos effectively teach even advanced patterns.

---

## Event Log Discovery vs. NL Discovery

| Aspect | Event Log Discovery | NL Discovery |
|---|---|---|
| **Input** | Observed traces (XES/CSV) | Text description |
| **Algorithm** | Inductive miner variants | DSPy ReAct agent |
| **Quality metric** | Fitness/precision against log | Structural soundness (no ground truth) |
| **Verification** | Token replay conformance | POWLJudge |
| **Use case** | Process improvement (as-is analysis) | Process design (to-be modeling) |
| **Relationship** | Complementary | Complementary |

Both use the same POWL v2 formalism, the same verification infrastructure, and the same BPMN export path. They address different input modalities for different stages of the process lifecycle.

---

## See Also

- [Tutorial](./nl-tutorial.md) — Step-by-step walkthrough
- [How-To Guide](./nl-howto.md) — Patterns and recipes
- [Reference](./nl-reference.md) — Complete API
- [PhD Thesis](../../docs/powl_v2_thesis.md) — Full academic treatment
