# Quick Start Guide - LLM Process Modeling Demo

## 30-Second Quick Start

### 1. Start the Server
```bash
cd /Users/sac/chatmangpt/pm4py/pm4wasm/js
npm run demo
```

### 2. Open Browser
Navigate to: **http://localhost:5173/llm/**

### 3. Try an Example
Click **"Bicycle Manufacturing"** button → Click **"✨ Generate Model"**

### 4. View Results
See the generated POWL model on the right panel!

---

## Detailed Walkthrough

### Step 1: Describe Your Process
In the text area, enter a process description like:
```
Users log in to their account, then select items and choose payment method.
Finally, they complete the purchase.
```

Or click an example button:
- **Bicycle Manufacturing**: Complex process with parallel tasks
- **Hotel Service**: Multi-actor service process
- **Online Shop**: E-commerce with choices

### Step 2: Generate Model
Click the **"✨ Generate Model"** button.

**What happens:**
1. System analyzes your text
2. Extracts activities (verbs and nouns)
3. Detects process structure (sequence, parallel, choice)
4. Generates formal POWL model

**Timing:** ~1.5 seconds

### Step 3: Review the Output
The right panel shows:
```
PO=(nodes={Login, Select, Choose, Complete},
    order={Login-->Select, Select-->Choose, Choose-->Complete})
```

**Understanding POWL syntax:**
- `nodes={...}`: List of activities
- `order={A-->B}`: A must happen before B
- `X(A, B)`: A and B happen in parallel
- `*(A, B)`: Either A or B happens (choice)

### Step 4: Refine (Optional)
If the model isn't perfect:

1. **Provide feedback:**
   ```
   Add a "Verify Payment" step after payment method
   ```

2. **Click "🔄 Refine Model"**

3. **See the updated model** with new activity added

### Step 5: Export
Choose your preferred format:

- **BPMN 2.0**: For business process management tools
- **Petri Net**: For academic/research tools
- **JSON**: For data storage and API integration

---

## Common Use Cases

### Use Case 1: Document Existing Processes
**Input:** Describe what your team does
**Output:** Formal process model for documentation

### Use Case 2: Process Design
**Input:** Describe how you want the process to work
**Output:** POWL model for implementation planning

### Use Case 3: Process Analysis
**Input:** Description of current process
**Output:** Model ready for conformance checking

### Use Case 4: Education
**Input:** Any process description
**Output:** Learn POWL syntax through examples

---

## Tips for Best Results

### ✅ Do:
- **Be specific**: "User logs in" vs "User does something"
- **Use order words**: "then", "after", "finally"
- **Specify parallelism**: "simultaneously", "at the same time"
- **Indicate choices**: "either X or Y", "optionally"

### ❌ Don't:
- **Use vague language**: "stuff happens", "do things"
- **Skip steps**: "Start process, then end"
- **Assume context**: The AI doesn't know your domain

---

## Example Transformations

### Simple Sequence
**Input:**
```
User logs in, views dashboard, then logs out.
```

**Output:**
```
PO=(nodes={Login, View, Logout},
    order={Login-->View, View-->Logout})
```

### Parallel Process
**Input:**
```
User simultaneously submits order and receives confirmation email.
```

**Output:**
```
X(SubmitOrder, SendEmail)
```

### Choice Process
**Input:**
```
Customer either pays by credit card or PayPal.
```

**Output:**
```
*(PayCredit, PayPaypal)
```

### Complex Process
**Input:**
```
User logs in, then simultaneously selects items and sets payment method.
After selecting, user either pays or completes installment agreement.
Finally, items are delivered.
```

**Output:**
```
PO=(nodes={Login, X(Select, SetPayment), *(Pay, Installment), Deliver},
    order={Login-->X(Select,SetPayment),
           X(Select,SetPayment)-->* (Pay,Installment),
           *(Pay,Installment)-->Deliver})
```

---

## Troubleshooting

### Issue: "Please enter a process description"
**Solution:** Type or paste a process description in the text area

### Issue: Model doesn't capture all activities
**Solution:** Use the feedback loop to add missing activities

### Issue: Model structure is wrong
**Solution:** Be more explicit about order ("then", "after") and parallelism ("simultaneously")

### Issue: Page won't load
**Solution:** Make sure the dev server is running (`npm run demo`)

---

## Next Steps

1. **Try your own process**: Describe something you do at work
2. **Experiment with feedback**: See how the model evolves
3. **Export and analyze**: Download JSON for further processing
4. **Read the docs**: Check README.md for advanced features

---

## Getting Help

- **Documentation**: `/demo/llm/README.md`
- **Implementation Details**: `/demo/llm/IMPLEMENTATION_SUMMARY.md`
- **Visual Guide**: `/demo/llm/VISUAL_GUIDE.md`
- **Main Project**: `/Users/sac/chatmangpt/pm4py/pm4wasm/`

---

**Demo URL:** http://localhost:5173/llm/
**Project:** POWL v2 - Process Mining in WebAssembly
**Date:** April 6, 2026
