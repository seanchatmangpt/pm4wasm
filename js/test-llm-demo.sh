#!/bin/bash

# Test script for LLM Process Modeling Demo

echo "Testing LLM Process Modeling Demo..."
PORT=5173

# Check if the dev server is running
if ! curl -s http://localhost:$PORT/ > /dev/null; then
    echo "❌ Dev server is not accessible at http://localhost:$PORT/"
    exit 1
fi

echo "✅ Dev server is running on port $PORT"

# Check if the main demo page is accessible
if curl -s http://localhost:$PORT/ | grep -q "POWL v2"; then
    echo "✅ Main demo page is accessible"
else
    echo "❌ Main demo page is not accessible"
fi

# Check if the LLM demo page is accessible
if curl -s http://localhost:$PORT/llm/ | grep -q "LLM Process Modeling"; then
    echo "✅ LLM demo page is accessible"
else
    echo "❌ LLM demo page is not accessible"
fi

# Check if WASM module is available
if curl -s http://localhost:$PORT/pkg/pm4wasm.js | grep -q "Powl"; then
    echo "✅ WASM module is available"
else
    echo "❌ WASM module is not available"
fi

echo ""
echo "Demo URLs:"
echo "  Main demo: http://localhost:$PORT/"
echo "  LLM demo:  http://localhost:$PORT/llm/"
