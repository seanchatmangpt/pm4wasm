# Tutorial: Generate a Verified Process Model from Natural Language

**What you'll learn:** How to go from a text description to a verified BPMN model in under 30 seconds.

**Prerequisites:** Python 3.9+, pm4py installed (`pip install -e .`), `GROQ_API_KEY` or `OPENAI_API_KEY` environment variable.

---

## Step 1: Describe Your Process in Plain English

Write a paragraph describing what happens in your process. Be specific about:
- **Activities** (what people/systems do)
- **Choices** (if/else, either/or)
- **Loops** (repeat, retry, again)
- **Parallelism** (things happening at the same time)

Example:

```
A hospital handles patient admissions. A patient arrives and registers.
A nurse triages the patient. If high urgency, go to emergency room.
If low urgency, wait in lobby then see a doctor. The doctor may order
lab tests. The doctor reviews results and prescribes medication or
recommends surgery. After treatment, the patient is discharged.
```

## Step 2: Generate and Verify

```python
from pm4py.algo.dspy.powl.natural_language import generate_powl_from_text

description = """
A hospital handles patient admissions. A patient arrives and registers.
A nurse triages the patient. If high urgency, go to emergency room.
If low urgency, wait in lobby then see a doctor. The doctor may order
lab tests. The doctor reviews results and prescribes medication or
recommends surgery. After treatment, the patient is discharged.
"""

result = generate_powl_from_text(description, max_refinements=2)

print(f"Verdict: {result['verdict']}")
print(f"Refinements needed: {result['refinements']}")
print(f"\nPOWL Model:\n{result['powl']}")
```

Output:

```
Verdict: True
Refinements needed: 1

POWL Model:
PO=( nodes={
    'Register', 'Triage',
    X( PO=( nodes={'Emergency Room'}, order={} ), *('Wait in Lobby', 'See Doctor') ),
    *('Order Lab Tests', 'Review Results'),
    X('Prescribe Medication', 'Recommend Surgery'),
    'Discharge'
  }, order={
    'Register'-->'Triage',
    'Triage'-->X( PO=( nodes={'Emergency Room'}, order={} ), *('Wait in Lobby', 'See Doctor') ),
    X( PO=( nodes={'Emergency Room'}, order={} ), *('Wait in Lobby', 'See Doctor') )-->*('Order Lab Tests', 'Review Results'),
    *('Order Lab Tests', 'Review Results')-->X('Prescribe Medication', 'Recommend Surgery'),
    X('Prescribe Medication', 'Recommend Surgery')-->'Discharge'
  })
```

## Step 3: Convert to BPMN

```python
from pm4py.objects.powl.parser import parse_powl_model_string
import pm4py

parsed = parse_powl_model_string(result["powl"])

# Try direct conversion, fall back to Petri net route
try:
    bpmn_model = pm4py.convert_to_bpmn(parsed)
except Exception:
    net, im, fm = pm4py.convert_to_petri_net(parsed)
    bpmn_model = pm4py.convert_to_bpmn(net, im, fm)

pm4py.write_bpmn(bpmn_model, "hospital_admission.bpmn")
print("BPMN written to hospital_admission.bpmn")
```

Open `hospital_admission.bpmn` in Camunda Modeler, Signavio, or any BPMN editor.

## Step 4: CLI One-Liner (Skip Python)

```bash
python -m pm4py.cli DiscoverPOWLToBPMN \
  "A hospital handles patient admissions. A patient arrives and registers. \
   A nurse triages the patient. If high urgency, go to emergency room. \
   If low urgency, wait in lobby then see a doctor." \
  hospital_admission.bpmn
```

Output:

```
BPMN model (VERIFIED, 1 refinements) written to hospital_admission.bpmn
```

## What Happened Under the Hood

1. **DSPy agent** parsed your description and generated a POWL model string
2. **validate_powl()** checked the string is syntactically valid
3. **POWLJudge** ("Dr. van der Aalst") evaluated structural soundness
4. If rejected, **refinement loop** appended judge feedback and re-generated
5. **BPMN converter** produced industry-standard BPMN 2.0 XML

## What the Operators Mean

| Symbol | Name | Meaning |
|---|---|---|
| `X(A, B)` | XOR | Execute A **or** B (exactly one) |
| `*(A, B)` | LOOP | Do A, optionally repeat B then A |
| `PO=(nodes={...}, order={...})` | Partial Order | All nodes execute; `order` constrains sequencing |

**The most common error:** Using `PO` edges when you mean `X`. In a `PO`, all outgoing edges mean all successors complete. If only one should execute, use `X()`.

## Next Steps

- [How-To Guide](./nl-howto.md) — Common patterns and recipes
- [Explanation](./nl-explanation.md) — How the pipeline works internally
- [Reference](./nl-reference.md) — Complete API and configuration
