# Forward Deployment Context

This repository is part of the **Chatman Ecosystem**, a portfolio built to make forward deployment repeatable, governed, and evidence-bearing.

Sean Chatman is publicly documenting the case for **The 2,001st Forward-Deployed Agentic Architect** while building the **operating system for forward deployment**.

## Local role

Within that portfolio, `pm4wasm` is a portable process-mining and conformance-analysis surface. It brings selected process-intelligence capabilities toward WebAssembly hosts so forward-deployed applications can analyze operational evidence closer to the customer environment.

```text
admitted event data → portable process analysis
→ discovered or conformance result → bounded diagnosis
→ repair/construction intent → receipt
```

Portability expands where analysis can run. It does not automatically preserve every native capability, prove business semantics, or authorize a repair.

```text
A = μ(O*)
R = receipt(A)
```

## Boundaries

- This file does not replace the repository’s supported algorithms, data formats, host contracts, license, or exact maturity status.
- Native-library capability is not automatically available in WASM.
- Event-log parsing does not prove object identity or business meaning.
- Analytical output is an observation or candidate diagnosis, not an authorized action.
- Portability and operational claims require exact target execution and receipts.

The canonical portfolio narrative is maintained in `seanchatmangpt/chatman-ecosystem`.
