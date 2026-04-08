# pm4wasm v0.2.0 — Vercel AI SDK Integration

## What's New

**Complete LLM integration using the Vercel AI SDK** — Generate POWL models from natural language using Groq, OpenAI, or Anthropic Claude directly in the browser.

### Quick Start

```bash
cd pm4wasm/js
npm install
npm run build:wasm
```

```typescript
import { Powl } from "@pm4py/pm4wasm";

const powl = await Powl.init();

// Natural language → POWL
const model = await powl.fromNaturalLanguage(
  "Customer orders and pays, then receives confirmation",
  {
    provider: "groq",
    apiKey: process.env.GROQ_API_KEY,
  },
  "ecommerce"
);

// POWL → BPMN
const bpmn = powl.toBpmn(model.toString());
```

## Supported Providers

| Provider | Speed | Cost | Best For |
|----------|-------|------|----------|
| **Groq** | ⚡⚡⚡ | Free | Development, fast iteration |
| **OpenAI** | ⚡⚡ | Paid | Production, GPT-4o |
| **Anthropic** | ⚡ | Paid | Claude 3.5 Sonnet |

**Default Models:**
- Groq: `openai/gpt-oss-20b`
- OpenAI: `gpt-4o`
- Anthropic: `claude-3-5-sonnet-20241022`

## New Dependencies

```json
{
  "ai": "^7.0.0-beta.72",
  "@ai-sdk/groq": "^4.0.0-beta.18",
  "@ai-sdk/openai": "^4.0.0-beta.25",
  "@ai-sdk/anthropic": "^4.0.0-beta.21"
}
```

## Documentation

- **[LLM Integration Guide](docs/llm-integration.md)** — Complete API reference
- **[Examples](js/examples/nl-to-powl-vercel-ai-sdk.ts)** — Code examples

## Features

✅ Natural language to POWL generation
✅ Multi-provider support (Groq, OpenAI, Anthropic)
✅ Domain-specific few-shot demos (5 domains)
✅ Automatic validation and refinement (up to 3 iterations)
✅ Code generation (n8n, Temporal, Camunda, YAWL)
✅ Browser-native (no server required)

## License

AGPL-3.0 — Vercel AI SDK components are Apache 2.0
