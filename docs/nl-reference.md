# Reference: NL → POWL → BPMN API

**What this is:** Complete API reference for the NL→POWL→BPMN pipeline. For getting started, see the [tutorial](./nl-tutorial.md).

---

## Python API

### `generate_powl_from_text(description, max_refinements=2, use_demos=True)`

Generate a verified POWL model from a natural language description.

**Parameters:**

| Parameter | Type | Default | Description |
|---|---|---|---|
| `description` | `str` | required | Process description in natural language |
| `max_refinements` | `int` | `2` | Maximum judge-refinement iterations |
| `use_demos` | `bool` | `True` | Include few-shot demonstrations |

**Returns:** `dict` with keys:

| Key | Type | Description |
|---|---|---|
| `powl` | `str` | Verified POWL model string |
| `verdict` | `bool` | True if judge approved, False if all refinements exhausted |
| `reasoning` | `str` | Judge's evaluation reasoning |
| `refinements` | `int` | Number of refinement iterations used |

**Example:**

```python
from pm4py.algo.dspy.powl.natural_language import generate_powl_from_text

result = generate_powl_from_text(
    "A customer orders a product. The order is validated. "
    "If valid, pick, pack, and bill in parallel. If invalid, cancel.",
    max_refinements=1
)
```

---

### `judge_powl(powl_string, context_description="")`

Evaluate a POWL model's structural quality without ground truth.

**Parameters:**

| Parameter | Type | Default | Description |
|---|---|---|---|
| `powl_string` | `str` | required | POWL model to evaluate |
| `context_description` | `str` | `""` | Original NL description (for plausibility check) |

**Returns:** `dict` with keys:

| Key | Type | Description |
|---|---|---|
| `reasoning` | `str` | Judge's evaluation of each criterion |
| `verdict` | `bool` | True if structurally sound |

---

### `parse_powl_model_string(powl_string)`

Parse a POWL string into a POWL model object.

**Parameters:** `powl_string` (`str`) — POWL model string.

**Returns:** Parsed POWL model object, or `None` if parsing fails.

---

## POWL Syntax Reference

### Operators

| Operator | Syntax | Semantics | Example |
|---|---|---|---|
| **Transition** | `label` | Single activity | `'Register Patient'` |
| **Silent transition** | `tau` | Invisible activity | `tau` |
| **XOR** | `X(a, b, ...)` | Exactly one child executes | `X('Approve', 'Reject')` |
| **LOOP** | `*(do, redo)` | Do `do`, optionally repeat `redo→do` | `*('Submit', 'Fix')` |
| **Partial Order** | `PO=(nodes={...}, order={...})` | All nodes execute; `order` constrains sequence | `PO=(nodes={'A','B'}, order={'A'-->'B'})` |

### Order Syntax

```
order={ 'A'-->'B', 'B'-->'C' }
```

- `-->` means "A precedes B" (A must complete before B starts)
- Omitted pairs in `order` with both nodes present means concurrent (no ordering constraint)

### Frequent Transition

```
FrequentTransition(activity='A', min=0, max=1, selfloop=false)
```

- `min=0`: activity is skippable
- `selfloop=true`: activity can repeat
- Shorthand for `X(A, tau)` or `*(A, tau)`

---

## CLI Reference

### `DiscoverPOWLFromText`

Generate a POWL file from natural language.

```bash
python -m pm4py.cli DiscoverPOWLFromText <description_or_file> <output.powl>
```

| Argument | Description |
|---|---|
| `description_or_file` | Inline text or path to a .txt file |
| `output.powl` | Output POWL file path |

### `DiscoverPOWLToBPMN`

Generate a verified BPMN file from natural language (full pipeline).

```bash
python -m pm4py.cli DiscoverPOWLToBPMN <description_or_file> <output.bpmn>
```

| Argument | Description |
|---|---|
| `description_or_file` | Inline text or path to a .txt file |
| `output.bpmn` | Output BPMN 2.0 XML file path |

**Output:** Prints verdict and refinement count.

### `DiscoverPOWL`

Discover a POWL model from an event log (programmatic, no LLM).

```bash
python -m pm4py.cli DiscoverPOWL <input.xes> <output.powl>
```

---

## Environment Variables

| Variable | Required | Description |
|---|---|---|
| `GROQ_API_KEY` | if using Groq | API key for Groq LLM provider |
| `OPENAI_API_KEY` | if using OpenAI | API key for OpenAI LLM provider |
| `LLM_MODEL` | no | Override default model (e.g., `groq/llama-3.3-70b-versatile`) |
| `LLM_API_BASE` | no | Override default API base URL |

---

## Module Structure

```
pm4py/algo/dspy/powl/
├── __init__.py
├── natural_language.py    # NL → POWL generation with judge-refinement loop
├── judge.py               # POWLJudge ("Dr. van der Aalst")
├── nl_demos.py            # 4 few-shot demos for NL generation
├── react_agent.py         # Event log → POWL agent
├── generation.py          # Tool functions (validate, coverage, fitness, finish)
├── optimize.py            # SIMBA optimization, LM configuration
├── metrics.py             # Quality metrics
├── data.py                # Training data creation
└── demos.py               # 5 few-shot demos for event log generation
```

---

## BPMN Conversion

### Direct Route

```python
bpmn_model = pm4py.convert_to_bpmn(parsed_powl)
pm4py.write_bpmn(bpmn_model, "output.bpmn")
```

Works for most models. May fail for complex DecisionGraph structures.

### Fallback Route (via Petri Net)

```python
net, im, fm = pm4py.convert_to_petri_net(parsed_powl)
bpmn_model = pm4py.convert_to_bpmn(net, im, fm)
pm4py.write_bpmn(bpmn_model, "output.bpmn")
```

Always available because POWL v2 models are guaranteed to convert to sound Petri nets.

### Recommended Pattern

```python
try:
    bpmn_model = pm4py.convert_to_bpmn(parsed)
except Exception:
    net, im, fm = pm4py.convert_to_petri_net(parsed)
    bpmn_model = pm4py.convert_to_bpmn(net, im, fm)
```

---

## Troubleshooting

| Problem | Cause | Fix |
|---|---|---|
| `ModuleNotFoundError: pm4py.algo.dspy` | pm4py not installed editable | `pip install -e .` |
| `API key not configured` | Missing env var | `export GROQ_API_KEY=...` |
| Verdict always False | Description too vague | Be specific about activities, choices, and order |
| Wrong operator (X vs PO) | Ambiguous language | Use "either/or" for XOR, "and/concurrently" for PO |
| BPMN conversion fails | Complex DecisionGraph | Use Petri net fallback (automatic) |
| Timeout during generation | LLM provider slow | Use Groq (fast) or increase timeout |

---

## See Also

- [Tutorial](./nl-tutorial.md) — Step-by-step walkthrough
- [How-To Guide](./nl-howto.md) — Patterns and recipes
- [Explanation](./nl-explanation.md) — How the pipeline works
- [PhD Thesis](../../docs/powl_v2_thesis.md) — Full academic treatment
