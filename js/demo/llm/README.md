# LLM Process Modeling Demo

This demo showcases an interactive LLM-based process modeling interface using POWL (Partially Ordered Workflow Language).

## Features

- **Text-to-Model Conversion**: Generate process models from natural language descriptions
- **Interactive Feedback Loop**: Refine models through conversational feedback
- **Example Processes**: Pre-loaded examples from academic papers (bicycle manufacturing, hotel service, online shop)
- **Export Capabilities**: Export models to BPMN 2.0, Petri Net, or JSON formats
- **Conversation History**: Track the entire modeling conversation

## How to Use

1. **Start the Dev Server**:
   ```bash
   cd js
   npm run demo
   ```

2. **Access the LLM Demo**:
   - Main demo: http://localhost:5173
   - LLM demo: http://localhost:5173/llm/

3. **Generate a Model**:
   - Enter a process description in the text area
   - Click "✨ Generate Model"
   - View the generated POWL model

4. **Refine the Model**:
   - Provide feedback in the feedback section
   - Click "🔄 Refine Model"
   - The model updates based on your input

5. **Export**:
   - Click "BPMN 2.0" for BPMN export
   - Click "Petri Net" for Petri Net export
   - Click "JSON" to download the model as JSON

## Example Process Descriptions

### Bicycle Manufacturing
```
A small company manufactures customized bicycles. The user starts an order by logging in to their account. Then, the user simultaneously selects the items to purchase and sets a payment method. Afterward, the user either pays or completes an installment agreement. After selecting the items, the user chooses between multiple options for a free reward. Finally, the items are delivered. The user has the right to return items for exchange.
```

### Hotel Service
```
The Evanstonian is an upscale independent hotel. When a guest calls room service, the room-service manager takes down the order. She then submits an order ticket to the kitchen to begin preparing the food. She also gives an order to the sommelier to fetch wine from the cellar. Finally, she assigns the order to the waiter. While the kitchen and sommelier are doing their tasks, the waiter readies a cart. Once the food, wine, and cart are ready, the waiter delivers it to the guest's room. After returning to the room-service station, the waiter debits the guest's account.
```

### Online Shop
```
Consider a process for purchasing items from an online shop. The user starts an order by logging in to their account. Then, the user simultaneously selects the items to purchase and sets a payment method. Afterward, the user either pays or completes an installment agreement. After selecting the items, the user chooses between multiple options for a free reward. Finally, the items are delivered.
```

## Technical Implementation

- **Frontend**: Pure HTML/CSS/JavaScript with ES modules
- **WASM Integration**: Uses the POWL WASM module for process model manipulation
- **State Management**: In-memory state for current model and conversation history
- **Export**: Client-side file generation for JSON export

## Future Enhancements

- Real LLM API integration (OpenAI, Anthropic, etc.)
- Visual process model rendering
- Advanced refinement patterns
- Model comparison and diff visualization
- Conformance checking integration

## Academic Context

This demo implements concepts from research on LLM-based process modeling, demonstrating how natural language descriptions can be transformed into formal process models (POWL) through iterative refinement.
