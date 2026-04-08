# LLM Integration with Vercel AI SDK

The pm4wasm JavaScript API now uses the **Vercel AI SDK** for natural language to POWL generation, supporting multiple LLM providers through a unified interface.

## Supported Providers

- **Groq** (default) - Fast inference with Llama models
- **OpenAI** - GPT-4, GPT-4o, etc.
- **Anthropic** - Claude 3.5 Sonnet, etc.

## Installation

The required dependencies are already included in `package.json`:

```json
{
  "dependencies": {
    "ai": "^7.0.0-beta.72",
    "@ai-sdk/groq": "^4.0.0-beta.18",
    "@ai-sdk/openai": "^4.0.0-beta.25",
    "@ai-sdk/anthropic": "^4.0.0-beta.21"
  }
}
```

Install them:

```bash
cd js
npm install
```

## Usage

### Basic Example

```typescript
import { Powl } from "@pm4py/pm4wasm";

// Initialize
const powl = await Powl.init();

// Generate POWL from natural language
const model = await powl.fromNaturalLanguage(
  "A customer submits an order, pays, and receives confirmation",
  {
    provider: "groq",
    apiKey: "gsk_...",  // Your Groq API key
  }
);

console.log(model.toString());
// Output: PO=(nodes={Submit, Order, Pay, Confirm}, order={Submit-->Order, Order-->Pay, Pay-->Confirm})
```

### Provider-Specific Examples

#### Groq (Default - Fast & Free)

```typescript
const model = await powl.fromNaturalLanguage(
  "Loan approval with validation",
  {
    provider: "groq",
    apiKey: process.env.GROQ_API_KEY,
    model: "openai/gpt-oss-20b",  // Default Groq model
  }
);
```

#### OpenAI

```typescript
const model = await powl.fromNaturalLanguage(
  "CI/CD pipeline with staging",
  {
    provider: "openai",
    apiKey: process.env.OPENAI_API_KEY,
    model: "gpt-4o",
  }
);
```

#### Anthropic Claude

```typescript
const model = await powl.fromNaturalLanguage(
  "Healthcare patient admission workflow",
  {
    provider: "anthropic",
    apiKey: process.env.ANTHROPIC_API_KEY,
    model: "claude-3-5-sonnet-20241022",
  }
);
```

### Domain-Specific Few-Shot Demos

The SDK includes domain-specific examples for better results:

```typescript
// Available domains: "loan_approval", "software_release", "ecommerce", 
//                   "manufacturing", "healthcare", or "general"

const model = await powl.fromNaturalLanguage(
  "Patient arrives, gets triaged, either goes to ER or waits for consultation",
  {
    provider: "groq",
    apiKey: "gsk_...",
  },
  "healthcare"  // Uses healthcare-specific few-shot demos
);
```

### Generate Code Directly

```typescript
// Natural language → n8n workflow
const n8nJson = await powl.naturalLanguageToCode(
  "Order processing with payment",
  "n8n",
  { provider: "groq", apiKey: "gsk_..." },
  "ecommerce"
);

// Natural language → Temporal Go workflow
const temporalGo = await powl.naturalLanguageToCode(
  "Microservice orchestration",
  "temporal",
  { provider: "openai", apiKey: process.env.OPENAI_API_KEY }
);

// Natural language → Camunda BPMN
const bpmnXml = await powl.naturalLanguageToCode(
  "Business approval process",
  "camunda",
  { provider: "anthropic", apiKey: process.env.ANTHROPIC_API_KEY }
);

// Natural language → YAWL v6 XML
const yawlXml = await powl.naturalLanguageToCode(
  "Manufacturing production line",
  "yawl",
  { provider: "groq", apiKey: "gsk_..." },
  "manufacturing"
);
```

### Validation & Refinement

The SDK automatically validates generated POWL models and refines them if needed:

```typescript
const model = await powl.fromNaturalLanguage(
  "Complex workflow with loops",
  { provider: "groq", apiKey: "gsk_..." }
);

// Behind the scenes:
// 1. LLM generates initial POWL
// 2. WASM validates soundness (deadlock freedom, liveness, boundedness)
// 3. If invalid, LLM is called again with feedback
// 4. Process repeats up to 3 times or until valid
```

### Manual Validation

```typescript
// Validate any POWL model string
const validation = powl.validatePowlStructure("X(A, B)->C");

console.log(validation.verdict);    // true/false
console.log(validation.reasoning);  // "✅ Model is structurally sound"
console.log(validation.violations); // [] (empty if sound)
```

## API Key Management

### Environment Variables (Recommended)

```bash
# .env.local
GROQ_API_KEY=gsk_...
OPENAI_API_KEY=sk-...
ANTHROPIC_API_KEY=sk-ant-...
```

```typescript
const model = await powl.fromNaturalLanguage(
  "Workflow description",
  {
    provider: "groq",
    apiKey: process.env.GROQ_API_KEY,
  }
);
```

### Runtime Configuration

```typescript
const config = {
  provider: "groq" as const,
  apiKey: "gsk_...",
  model: "llama-3.3-70b-versatile",
  temperature: 0.2,
  maxTokens: 1024,
};

const model = await powl.fromNaturalLanguage("Description", config);
```

## Complete Pipeline Example

```typescript
import { Powl } from "@pm4py/pm4wasm";

async function workflowToBpmn() {
  const powl = await Powl.init();

  // 1. Natural language → POWL model
  const model = await powl.fromNaturalLanguage(
    "Customer places order, system validates payment, " +
    "if payment succeeds then ship items, otherwise cancel order",
    {
      provider: "groq",
      apiKey: process.env.GROQ_API_KEY,
    },
    "ecommerce"
  );

  // 2. Validate
  const validation = powl.validatePowlStructure(model.toString());
  if (!validation.verdict) {
    throw new Error(`Invalid model: ${validation.reasoning}`);
  }

  // 3. Convert to BPMN
  const bpmn = powl.toBpmn(model.toString());

  // 4. Save or use the BPMN
  return bpmn;
}
```

## Error Handling

```typescript
try {
  const model = await powl.fromNaturalLanguage(
    "Workflow description",
    { provider: "groq", apiKey: "gsk_..." }
  );
} catch (error) {
  if (error.message.includes("API key")) {
    console.error("Missing or invalid API key");
  } else if (error.message.includes("LLM API error")) {
    console.error("LLM provider error:", error.message);
  } else {
    console.error("Unexpected error:", error);
  }
}
```

## Advanced Configuration

### Temperature

Lower temperature (0.0-0.3) for more deterministic outputs:

```typescript
const model = await powl.fromNaturalLanguage(
  "Strict business process",
  {
    provider: "groq",
    apiKey: "gsk_...",
    temperature: 0.1,  // More deterministic
  }
);
```

Higher temperature (0.7-1.0) for more creative outputs:

```typescript
const model = await powl.fromNaturalLanguage(
  "Exploratory workflow design",
  {
    provider: "openai",
    apiKey: process.env.OPENAI_API_KEY,
    temperature: 0.8,  // More creative
  }
);
```

### Max Tokens

```typescript
const model = await powl.fromNaturalLanguage(
  "Very complex workflow with many steps",
  {
    provider: "groq",
    apiKey: "gsk_...",
    maxTokens: 2048,  // Allow longer responses
  }
);
```

## Provider Comparison

| Provider | Speed | Cost | Quality | Best For |
|----------|-------|------|---------|----------|
| **Groq** | ⚡⚡⚡ | Free | Good | Fast iteration, development |
| **OpenAI** | ⚡⚡ | Paid | Excellent | Production, complex workflows |
| **Anthropic** | ⚡ | Paid | Excellent | Nuanced reasoning, long contexts |

**Default Models:**
- Groq: `openai/gpt-oss-20b`
- OpenAI: `gpt-4o`
- Anthropic: `claude-3-5-sonnet-20241022`

## Browser Usage

The SDK works entirely in the browser with no server required:

```html
<!DOCTYPE html>
<html>
<head>
  <script type="module">
    import { Powl } from 'https://cdn.jsdelivr.net/npm/@pm4py/pm4wasm';

    const powl = await Powl.init();
    
    document.getElementById('generate').addEventListener('click', async () => {
      const description = document.getElementById('description').value;
      const apiKey = document.getElementById('apiKey').value;
      
      const model = await powl.fromNaturalLanguage(description, {
        provider: 'groq',
        apiKey: apiKey,
      });
      
      document.getElementById('output').textContent = model.toString();
    });
  </script>
</head>
<body>
  <textarea id="description" placeholder="Describe your workflow..."></textarea>
  <input id="apiKey" type="password" placeholder="Groq API key">
  <button id="generate">Generate POWL</button>
  <pre id="output"></pre>
</body>
</html>
```

## License

This LLM integration is part of pm4wasm and is licensed under AGPL-3.0.
The Vercel AI SDK has its own license (Apache 2.0).
