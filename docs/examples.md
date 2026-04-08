# Example Code Snippets

Practical examples for common POWL v2 Rust/WASM use cases.

## Table of Contents

1. [Basic Operations](#basic-operations)
2. [Event Log Processing](#event-log-processing)
3. [Conformance Checking](#conformance-checking)
4. [Visualization](#visualization)
5. [Advanced Analysis](#advanced-analysis)
6. [Browser Integration](#browser-integration)

---

## Basic Operations

### Parse and Validate

```javascript
import { Powl } from '@pm4py/pm4wasm';

const powl = await Powl.init();

// Parse a model
const model = powl.parse('PO=(nodes={A, B, C}, order={A-->B, B-->C})');

// Validate (throws if invalid)
try {
    model.validate();
    console.log('✓ Model is valid');
} catch (e) {
    console.error('✗ Invalid:', e.message);
}
```

### Get Model Statistics

```javascript
const model = powl.parse('X(A, B, C)');

console.log('Activities:', [...model.activities()]);
// Output: ["A", "B", "C"]

console.log('Node count:', model.len());
// Output: 4 (XOR + A + B + C)

console.log('Root index:', model.root());
// Output: 3 (last node)

console.log('String representation:', model.toString());
// Output: "X ( A, B, C )"
```

### Simplify Model

```javascript
// Flatten nested operators
let model = powl.parse('X(A, X(B, C))');
console.log(model.toString());  // "X ( A, X ( B, C ) )"

model = model.simplify();
console.log(model.toString());  // "X ( A, B, C )"

// Convert XOR(A, tau) to FrequentTransition
model = powl.parse('X(A, tau)');
model = model.simplifyFrequent();
console.log(model.toString());
// Output: "FrequentTransition(activity=A, min=0, max=1, selfloop=false)"
```

---

## Event Log Processing

### Parse CSV Event Log

```javascript
const csv = `case_id,activity,timestamp
1,A,2024-01-01T10:00:00Z
1,B,2024-01-01T10:05:00Z
1,C,2024-01-01T10:10:00Z
2,A,2024-01-01T11:00:00Z
2,C,2024-01-01T11:05:00Z`;

const log = powl.parseCsv(csv);
console.log('Traces:', log.traces.length);
console.log('Total events:', log.totalEvents());
```

### Parse XES Event Log

```javascript
const xml = `
<log xes.version="1.0" xes.features="nested-attributes">
    <trace>
        <string key="concept:name" value="Case 1"/>
        <event>
            <string key="concept:name" value="A"/>
            <date key="time:timestamp" value="2024-01-01T10:00:00Z"/>
        </event>
        <event>
            <string key="concept:name" value="B"/>
            <date key="time:timestamp" value="2024-01-01T10:05:00Z"/>
        </event>
    </trace>
</log>`;

const log = powl.parseXes(xml);
```

### Load XES from File (Drag-and-Drop)

```javascript
document.getElementById('file-input').addEventListener('change', async (e) => {
    const file = e.target.files[0];
    const log = await powl.readXesFile(file);

    console.log(`Loaded ${log.traces.length} traces`);
    console.log(`Total events: ${log.totalEvents()}`);
});
```

### Get Variants

```javascript
const variants = powl.variants(log);

// Display top 5 variants
const top5 = Object.entries(variants)
    .sort((a, b) => b[1] - a[1])
    .slice(0, 5);

for (const [trace, count] of top5) {
    console.log(`${trace}: ${count} occurrences (${(count / log.traces.length * 100).toFixed(1)}%)`);
}
```

### Filter Event Log

```javascript
// Filter by trace length
const filtered = powl.filterTraces(log, {
    minLength: 3,
    maxLength: 10
});

// Filter by activity presence
const filtered = powl.filterTraces(log, {
    mustContain: ['A', 'B']
});

// Filter by case attributes
const filtered = powl.filterTraces(log, {
    attribute: 'customer_type',
    value: 'premium'
});
```

---

## Conformance Checking

### Check Fitness

```javascript
const model = powl.parse('PO=(nodes={A, B, C}, order={A-->B, B-->C})');
const log = powl.parseCsv('case_id,activity\n1,A\n1,B\n1,C\n2,A\n2,C');

const result = powl.conformance(model, log);

console.log('Overall fitness:', (result.percentage * 100).toFixed(1) + '%');
console.log('Perfectly fitting traces:', result.perfectlyFittingTraces);
console.log('Traces with deviations:', result.tracesWithDeviations);
```

### Analyze Deviations

```javascript
const result = powl.conformance(model, log);

for (const traceDev of result.deviations) {
    console.log(`\nTrace ${traceDev.traceIndex}:`);

    if (traceDev.missingTokens > 0) {
        console.log(`  ⚠ Missing tokens: ${traceDev.missingTokens}`);
    }

    if (traceDev.remainingTokens > 0) {
        console.log(`  ⚠ Remaining tokens: ${traceDev.remainingTokens}`);
    }

    console.log(`  Fitness: ${(traceDev.fitness * 100).toFixed(1)}%`);

    if (traceDev.deviation) {
        console.log(`  Deviation type: ${traceDev.deviation.type}`);
        console.log(`  Deviation details:`, traceDev.deviation);
    }
}
```

### Filter by Fitness Threshold

```javascript
// Keep only traces with fitness >= 0.8
const goodTraces = powl.filterByFitness(model, log, 0.8);

console.log(`Original traces: ${log.traces.length}`);
console.log(`High-fitness traces: ${goodTraces.traces.length}`);

// Recompute fitness on filtered log
const newResult = powl.conformance(model, goodTraces);
console.log('New fitness:', (newResult.percentage * 100).toFixed(1) + '%');
```

### Compare Model Versions

```javascript
const modelV1 = powl.parse('X(A, B)');
const modelV2 = powl.parse('X(A, B, C)');

const resultV1 = powl.conformance(modelV1, log);
const resultV2 = powl.conformance(modelV2, log);

console.log('Model v1 fitness:', (resultV1.percentage * 100).toFixed(1) + '%');
console.log('Model v2 fitness:', (resultV2.percentage * 100).toFixed(1) + '%');

if (resultV2.percentage > resultV1.percentage) {
    console.log('✓ Model v2 fits better');
}
```

---

## Visualization

### Convert to Petri Net for Visualization

```javascript
const model = powl.parse('X(A, B)');
const petriNet = model.toPetriNet();

console.log('Petri Net Structure:');
console.log('Places:', petriNet.net.places.length);
console.log('Transitions:', petriNet.net.transitions.length);
console.log('Arcs:', petriNet.net.arcs.length);

// Generate DOT format for Graphviz
let dot = 'digraph PetriNet {\n';
dot += '  rankdir=LR;\n';

// Add places
for (const place of petriNet.net.places) {
    dot += `  "${place.name}" [shape=circle];\n`;
}

// Add transitions
for (const trans of petriNet.net.transitions) {
    const label = trans.label || 'tau';
    dot += `  "${trans.name}" [shape=rect,label="${label}"];\n`;
}

// Add arcs
for (const arc of petriNet.net.arcs) {
    dot += `  "${arc.source}" -> "${arc.target}"` +
           `[label="${arc.weight}"];\n`;
}

dot += '}';
console.log(dot);
```

### Visualize Process Tree

```javascript
const model = powl.parse('X(A, B)');
const processTree = model.toProcessTree();

function treeToDot(node, id = 'root') {
    let dot = '';

    if (node.label) {
        // Leaf node (activity)
        dot += `  ${id} [label="${node.label}",shape=ellipse];\n`;
    } else {
        // Operator node
        dot += `  ${id} [label="${node.operator}",shape=box];\n`;

        // Recursively add children
        for (let i = 0; i < node.children.length; i++) {
            const childId = `${id}_${i}`;
            dot += treeToDot(node.children[i], childId);
            dot += `  ${id} -> ${childId};\n`;
        }
    }

    return dot;
}

let dot = 'digraph ProcessTree {\n';
dot += '  node [fontname="Arial"];\n';
dot += treeToDot(processTree.root);
dot += '}';
console.log(dot);
```

### Export to BPMN

```javascript
const model = powl.parse('X(A, B)');
const bpmnXml = powl.toBpmn(model);

// Download as file
const blob = new Blob([bpmnXml], { type: 'application/xml' });
const url = URL.createObjectURL(blob);

const a = document.createElement('a');
a.href = url;
a.download = 'model.bpmn';
a.click();

URL.revokeObjectURL(url);
```

---

## Advanced Analysis

### Extract Footprints

```javascript
const model = powl.parse('PO=(nodes={A, B, C}, order={A-->B, A-->C})');
const fp = powl.getFootprints(model);

console.log('Start activities:', [...fp.startActivities]);
// Output: ["A"]

console.log('End activities:', [...fp.endActivities]);
// Output: ["B", "C"]

console.log('Always happens:', [...fp.activitiesAlwaysHappening]);
// Output: ["A"]

console.log('Skippable:', [...fp.skippableActivities]);
// Output: []

console.log('Sequence relations:');
for (const [a, b] of fp.sequence) {
    console.log(`  ${a} → ${b}`);
}
// Output:
//   A → B
//   A → C

console.log('Parallel relations:');
for (const [a, b] of fp.parallel) {
    console.log(`  ${a} ∥ ${b}`);
}
// Output:
//   B ∥ C
//   C ∥ B

console.log('Min trace length:', fp.minTraceLength);
// Output: 2 (A, then B or C)
```

### Detect Parallelism

```javascript
const model = powl.parse('PO=(nodes={A, B, C}, order={})');
const fp = powl.getFootprints(model);

if (fp.parallel.size > 0) {
    console.log('✓ Model has parallel activities');

    // Find maximal parallel sets
    const parallelSets = [];
    for (const [a, b] of fp.parallel) {
        parallelSets.push([a, b]);
    }

    console.log('Parallel pairs:', parallelSets);
} else {
    console.log('✗ Model is purely sequential');
}
```

### Compute Model Complexity

```javascript
const model = powl.parse('X(A, X(B, C))');
const metrics = powl.getComplexity(model);

console.log('Complexity Metrics:');
console.log('  Total nodes:', metrics.totalNodes);
console.log('  Transitions:', metrics.transitionCount);
console.log('  Operators:', metrics.operatorCount);
console.log('  Max nesting depth:', metrics.maxNestingDepth);
console.log('  Control-flow complexity:', metrics.cfcScore);

// Simplify and compare
const simplified = model.simplify();
const metricsSimple = powl.getComplexity(simplified);

console.log('\nAfter simplification:');
console.log('  Total nodes:', metricsSimple.totalNodes);
console.log('  Reduction:', metrics.totalNodes - metricsSimple.totalNodes, 'nodes');
```

### Compare Two Models

```javascript
const model1 = powl.parse('X(A, B)');
const model2 = powl.parse('X(A, B, C)');

const diff = powl.compareModels(model1, model2);

console.log('Model Comparison:');
console.log('  Added activities:', diff.addedActivities);
// Output: ["C"]

console.log('  Removed activities:', diff.removedActivities);
// Output: []

console.log('  Added parallel pairs:', diff.addedParallelPairs);
console.log('  Removed parallel pairs:', diff.removedParallelPairs);

console.log('  Conformance delta:', diff.conformanceDelta);
// Output: +0.1 (10% improvement)
```

### Find All Possible Traces

```javascript
const model = powl.parse('X(A, B)');
const fp = powl.getFootprints(model);

function generateTraces(fp, current = [], visited = new Set()) {
    // Get last activity in current trace
    const last = current[current.length - 1];

    // If trace is complete (ends at end activity)
    if (current.length > 0 && fp.endActivities.has(last)) {
        return [current];
    }

    // Find possible next activities
    let nextActivities = [];

    if (current.length === 0) {
        // Start from start activities
        nextActivities = [...fp.startActivities];
    } else {
        // Find activities that can follow last
        for (const [a, b] of fp.sequence) {
            if (a === last) {
                nextActivities.push(b);
            }
        }
    }

    // Recursively generate traces
    const traces = [];
    for (const next of nextActivities) {
        if (!visited.has(next)) {
            visited.add(next);
            const newTrace = [...current, next];
            traces.push(...generateTraces(fp, newTrace, visited));
            visited.delete(next);
        }
    }

    return traces;
}

const traces = generateTraces(fp);
console.log('All possible traces:');
for (const trace of traces) {
    console.log('  ', trace.join(' → '));
}
```

---

## Browser Integration

### Complete Browser Example

```html
<!DOCTYPE html>
<html>
<head>
    <title>POWL Process Mining</title>
</head>
<body>
    <h1>POWL Process Mining Demo</h1>

    <div>
        <h2>1. Load Event Log</h2>
        <input type="file" id="file-input" accept=".csv,.xes">
        <p id="file-info"></p>
    </div>

    <div>
        <h2>2. Parse Model</h2>
        <textarea id="model-input" rows="5" cols="50">
PO=(nodes={A, B, C}, order={A-->B, B-->C})
        </textarea>
        <button id="parse-btn">Parse & Validate</button>
        <pre id="model-output"></pre>
    </div>

    <div>
        <h2>3. Conformance Check</h2>
        <button id="conformance-btn">Check Fitness</button>
        <pre id="conformance-output"></pre>
    </div>

    <script type="module">
        import { Powl } from './pkg/pm4wasm.js';

        // Initialize WASM
        const powl = await Powl.init();
        let currentLog = null;

        // File input handler
        document.getElementById('file-input').addEventListener('change', async (e) => {
            const file = e.target.files[0];
            if (file.name.endsWith('.csv')) {
                currentLog = await powl.readCsvFile(file);
            } else if (file.name.endsWith('.xes')) {
                currentLog = await powl.readXesFile(file);
            } else {
                alert('Please upload a CSV or XES file');
                return;
            }

            document.getElementById('file-info').textContent =
                `Loaded ${currentLog.traces.length} traces, ${currentLog.totalEvents()} events`;
        });

        // Parse model button
        document.getElementById('parse-btn').addEventListener('click', () => {
            const modelStr = document.getElementById('model-input').value;

            try {
                const model = powl.parse(modelStr);
                model.validate();

                const output = {
                    valid: true,
                    activities: [...model.activities()],
                    nodeCount: model.len(),
                    string: model.toString()
                };

                document.getElementById('model-output').textContent =
                    JSON.stringify(output, null, 2);
            } catch (e) {
                document.getElementById('model-output').textContent =
                    `Error: ${e.message}`;
            }
        });

        // Conformance check button
        document.getElementById('conformance-btn').addEventListener('click', () => {
            if (!currentLog) {
                alert('Please load an event log first');
                return;
            }

            const modelStr = document.getElementById('model-input').value;
            const model = powl.parse(modelStr);

            const result = powl.conformance(model, currentLog);

            const output = {
                fitness: (result.percentage * 100).toFixed(1) + '%',
                perfectlyFittingTraces: result.perfectlyFittingTraces,
                tracesWithDeviations: result.tracesWithDeviations,
                deviations: result.deviations.map(d => ({
                    traceIndex: d.traceIndex,
                    fitness: (d.fitness * 100).toFixed(1) + '%',
                    missingTokens: d.missingTokens,
                    remainingTokens: d.remainingTokens
                }))
            };

            document.getElementById('conformance-output').textContent =
                JSON.stringify(output, null, 2);
        });
    </script>
</body>
</html>
```

### React Integration

```jsx
import React, { useState, useEffect } from 'react';
import { Powl } from '@pm4py/pm4wasm';

function ProcessMiningApp() {
    const [powl, setPowl] = useState(null);
    const [model, setModel] = useState(null);
    const [log, setLog] = useState(null);
    const [result, setResult] = useState(null);

    useEffect(() => {
        Powl.init().then(setPowl);
    }, []);

    const handleFileUpload = async (file) => {
        if (file.name.endsWith('.csv')) {
            const eventLog = await powl.readCsvFile(file);
            setLog(eventLog);
        } else if (file.name.endsWith('.xes')) {
            const eventLog = await powl.readXesFile(file);
            setLog(eventLog);
        }
    };

    const handleParse = (modelStr) => {
        const parsedModel = powl.parse(modelStr);
        parsedModel.validate();
        setModel(parsedModel);
    };

    const handleConformance = () => {
        const conformanceResult = powl.conformance(model, log);
        setResult(conformanceResult);
    };

    if (!powl) return <div>Loading...</div>;

    return (
        <div>
            <input type="file" onChange={(e) => handleFileUpload(e.target.files[0])} />
            <textarea onChange={(e) => handleParse(e.target.value)} />
            <button onClick={handleConformance}>Check Fitness</button>

            {result && (
                <div>
                    <h2>Results</h2>
                    <p>Fitness: {(result.percentage * 100).toFixed(1)}%</p>
                    <p>Perfectly fitting traces: {result.perfectlyFittingTraces}</p>
                    <p>Traces with deviations: {result.tracesWithDeviations}</p>
                </div>
            )}
        </div>
    );
}

export default ProcessMiningApp;
```

### Vue Integration

```vue
<template>
  <div>
    <input type="file" @change="handleFileUpload" />
    <textarea v-model="modelString" @input="handleParse"></textarea>
    <button @click="handleConformance">Check Fitness</button>

    <div v-if="result">
      <h2>Results</h2>
      <p>Fitness: {{ (result.percentage * 100).toFixed(1) }}%</p>
      <p>Perfectly fitting traces: {{ result.perfectlyFittingTraces }}</p>
      <p>Traces with deviations: {{ result.tracesWithDeviations }}</p>
    </div>
  </div>
</template>

<script>
import { Powl } from '@pm4py/pm4wasm';

export default {
  data() {
    return {
      powl: null,
      modelString: 'PO=(nodes={A, B, C}, order={A-->B, B-->C})',
      model: null,
      log: null,
      result: null
    };
  },
  async mounted() {
    this.powl = await Powl.init();
  },
  methods: {
    async handleFileUpload(event) {
      const file = event.target.files[0];
      if (file.name.endsWith('.csv')) {
        this.log = await this.powl.readCsvFile(file);
      } else if (file.name.endsWith('.xes')) {
        this.log = await this.powl.readXesFile(file);
      }
    },
    handleParse() {
      this.model = this.powl.parse(this.modelString);
      this.model.validate();
    },
    handleConformance() {
      this.result = this.powl.conformance(this.model, this.log);
    }
  }
};
</script>
```

---

## See Also

- [Quick Reference](./quick-reference.md) — Common operations reference
- [API Reference](./reference.md) — Complete API documentation
- [Tutorial](./tutorial.md) — Getting started guide
