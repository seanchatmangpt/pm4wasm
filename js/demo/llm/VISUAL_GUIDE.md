# LLM Process Modeling Demo - Visual Guide

## Page Layout

```
┌─────────────────────────────────────────────────────────────────┐
│                    🚀 LLM Process Modeling                      │
│              Generate process models from text using POWL        │
└─────────────────────────────────────────────────────────────────┘
┌──────────────────────────────┬──────────────────────────────────┐
│  📝 Input                    │  🎨 Generated Model              │
│  ┌────────────────────────┐  │  ┌────────────────────────────┐ │
│  │ Describe your process │  │  │ Status: ✅ Success          │ │
│  │                        │  │  │                            │ │
│  │ [Textarea]             │  │  │ PO=(nodes={Login, Select}, │ │
│  │                        │  │  │     order={Login-->Select})│ │
│  │                        │  │  │                            │ │
│  └────────────────────────┘  │  └────────────────────────────┘ │
│  Examples:                    │  📤 Export                      │
│  [Bicycle] [Hotel] [Order]    │  [BPMN] [Petri] [JSON]         │
│  ✨ Generate Model  Clear     │                                  │
│                              │                                  │
│  💬 Feedback                  │                                  │
│  ┌────────────────────────┐  │                                  │
│  │ Provide feedback...    │  │                                  │
│  │                        │  │                                  │
│  └────────────────────────┘  │                                  │
│  🔄 Refine Model             │                                  │
│                              │                                  │
│  📜 Conversation History     │                                  │
│  ┌────────────────────────┐  │                                  │
│  │ 👤 User: ...           │  │                                  │
│  │ 🤖 Assistant: ...      │  │                                  │
│  └────────────────────────┘  │                                  │
└──────────────────────────────┴──────────────────────────────────┘
```

## Color Scheme

### Header
- **Background**: Linear gradient (135deg, #667eea → #764ba2)
- **Text**: White
- **Vibe**: Professional, modern, trustworthy

### Input Panel
- **Border**: #e0e0e0
- **Textarea**: White with #ddd border
- **Primary Button**: Gradient #667eea → #764ba2
- **Secondary Button**: #f0f0f0

### Output Panel
- **Background**: #f8f9fa
- **Model Output**: Monospace font
- **Status Messages**:
  - Success: #d4edda background, #155724 text
  - Error: #f8d7da background, #721c24 text
  - Info: #d1ecf1 background, #0c5462 text

## Interaction Flow

### Step 1: Input
```
User types process description
    ↓
User clicks "✨ Generate Model"
    ↓
System shows: "Generating model..." (blue info)
```

### Step 2: Generation
```
System analyzes text (1.5 seconds)
    ↓
Extracts activities (Login, Select, Pay, etc.)
    ↓
Detects structure (sequence, parallel, choice)
    ↓
Generates POWL model
    ↓
System shows: "Model generated successfully!" (green success)
```

### Step 3: Output
```
Model appears in right panel:
PO=(nodes={A, B, C}, order={A-->B, B-->C})

Export buttons become visible
```

### Step 4: Feedback (Optional)
```
User types feedback
    ↓
User clicks "🔄 Refine Model"
    ↓
System updates model based on feedback
    ↓
Conversation history updates
```

## Example Process Transformation

### Input (Natural Language)
```
A small company manufactures customized bicycles.
The user starts an order by logging in to their account.
Then, the user simultaneously selects items and sets payment method.
```

### Output (POWL Model)
```
PO=(nodes={Login, X(Select, SetPayment)},
    order={Login-->X(Select,SetPayment)})
```

### Explanation
- **Login**: Initial activity
- **X(Select, SetPayment)**: Parallel execution
- **Order**: Login must complete before parallel tasks start

## Status Messages

### Success State
```
┌─────────────────────────────┐
│ ✅ Model generated           │
│     successfully!            │
└─────────────────────────────┘
```

### Error State
```
┌─────────────────────────────┐
│ ❌ Please enter a process   │
│     description              │
└─────────────────────────────┘
```

### Loading State
```
┌─────────────────────────────┐
│ ℹ️  Generating model...      │
└─────────────────────────────┘
```

## Conversation History Format

```
┌─────────────────────────────┐
│ 👤 User                     │
│ A small company manufactures│
│ customized bicycles...      │
├─────────────────────────────┤
│ 🤖 Assistant                │
│ Generated POWL model:       │
│ PO=(nodes={Login, Select}   │
├─────────────────────────────┤
│ 👤 User                     │
│ Add a quality check step    │
├─────────────────────────────┤
│ 🤖 Assistant                │
│ Refined POWL model:         │
│ PO=(nodes={Login, Select,   │
│      QualityCheck}          │
└─────────────────────────────┘
```

## Responsive Design

### Desktop (>1400px)
- Side-by-side panels (50% each)
- Full textarea height (200px)
- All buttons visible

### Tablet (768px-1400px)
- Side-by-side panels (50% each)
- Stacked buttons
- Scrollable conversation history

### Mobile (<768px)
- Stacked panels (100% width)
- Compact buttons
- Collapsible sections

## Browser Compatibility

- ✅ Chrome/Edge (Chromium)
- ✅ Firefox
- ✅ Safari
- ✅ Opera
- ⚠️ IE11 (not supported - no ES modules)

## Performance Metrics

- **Initial Load**: <2 seconds
- **WASM Initialization**: <1 second
- **Model Generation**: 1.5 seconds (simulated)
- **Model Refinement**: 1.5 seconds (simulated)
- **Export**: Instant (client-side)
