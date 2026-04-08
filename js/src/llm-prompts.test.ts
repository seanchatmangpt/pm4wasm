/**
 * Tests for LLM Prompts Framework
 *
 * Tests the prompting strategies from Kourani et al. 2024
 */

import { describe, it, expect } from "vitest";
import {
  PROMPT_TEMPLATES,
  LLMPowLGenerator,
  type LLMMessage,
} from "./llm-prompts.js";
import { ModelGenerator, type ActivityNode } from "./model-generator.js";

describe("PROMPT_TEMPLATES", () => {
  describe("SYSTEM_PROMPT", () => {
    it("should contain role assignment", () => {
      expect(PROMPT_TEMPLATES.SYSTEM_PROMPT).toContain("expert");
      expect(PROMPT_TEMPLATES.SYSTEM_PROMPT).toContain("process modeling");
    });

    it("should contain POWL function documentation", () => {
      expect(PROMPT_TEMPLATES.SYSTEM_PROMPT).toContain("activity(label)");
      expect(PROMPT_TEMPLATES.SYSTEM_PROMPT).toContain("xor(...args)");
      expect(PROMPT_TEMPLATES.SYSTEM_PROMPT).toContain("loop(do, redo)");
      expect(PROMPT_TEMPLATES.SYSTEM_PROMPT).toContain(
        "partial_order(dependencies)"
      );
    });

    it("should mention key rules", () => {
      expect(PROMPT_TEMPLATES.SYSTEM_PROMPT).toContain("IRREFLEXIVE");
      expect(PROMPT_TEMPLATES.SYSTEM_PROMPT).toContain("TRANSITIVE");
    });
  });

  describe("EXAMPLES", () => {
    it("should contain bicycle manufacturing example", () => {
      expect(PROMPT_TEMPLATES.EXAMPLES).toContain("bicycle");
      expect(PROMPT_TEMPLATES.EXAMPLES).toContain("Create process");
      expect(PROMPT_TEMPLATES.EXAMPLES).toContain("Check part");
    });

    it("should contain loan application example", () => {
      expect(PROMPT_TEMPLATES.EXAMPLES).toContain("loan");
      expect(PROMPT_TEMPLATES.EXAMPLES).toContain("credit check");
    });

    it("should contain order processing example", () => {
      expect(PROMPT_TEMPLATES.EXAMPLES).toContain("rework");
      expect(PROMPT_TEMPLATES.EXAMPLES).toContain("ship");
    });
  });

  describe("COMMON_ERRORS", () => {
    it("should warn about self-loops", () => {
      expect(PROMPT_TEMPLATES.COMMON_ERRORS).toContain("IRREFLEXIVITY");
      expect(PROMPT_TEMPLATES.COMMON_ERRORS).toContain("(A, A)");
    });

    it("should warn about non-transitive dependencies", () => {
      expect(PROMPT_TEMPLATES.COMMON_ERRORS).toContain("TRANSITIVITY");
      expect(PROMPT_TEMPLATES.COMMON_ERRORS).toContain("Missing");
    });

    it("should warn about reusing sub-models without copying", () => {
      expect(PROMPT_TEMPLATES.COMMON_ERRORS).toContain("copy()");
    });
  });

  describe("generatePrompt", () => {
    it("should generate a complete prompt from process description", () => {
      const prompt = PROMPT_TEMPLATES.generatePrompt(
        "Orders are received and processed"
      );

      expect(prompt).toContain("Orders are received and processed");
      expect(prompt).toContain("ModelGenerator");
      expect(prompt).toContain("```python");
    });

    it("should include conversation history for refinements", () => {
      const history: LLMMessage[] = [
        { role: "system", content: "System" },
        { role: "user", content: "First attempt" },
        { role: "assistant", content: "Here's the code" },
        { role: "user", content: "Error: something failed" },
      ];

      const prompt = PROMPT_TEMPLATES.generatePrompt(
        "Orders are received",
        history
      );

      expect(prompt).toContain("PREVIOUS ATTEMPT");
      expect(prompt).toContain("ERROR FEEDBACK");
    });
  });

  describe("ERROR_REFINEMENT", () => {
    it("should include error and conversation history", () => {
      const history: LLMMessage[] = [
        { role: "system", content: "System" },
        { role: "user", content: "Generate model" },
        { role: "assistant", content: "gen.activity('A')" },
      ];

      const refinement = PROMPT_TEMPLATES.ERROR_REFINEMENT(
        "Irreflexivity violation",
        history
      );

      expect(refinement).toContain("Irreflexivity violation");
      expect(refinement).toContain("PREVIOUS ATTEMPT");
      expect(refinement).toContain("gen.activity('A')");
    });
  });

  describe("VALIDATION_FEEDBACK", () => {
    it("should return success message for valid models", () => {
      const feedback = PROMPT_TEMPLATES.VALIDATION_FEEDBACK({
        isValid: true,
        errors: [],
        warnings: [],
      });

      expect(feedback).toContain("✅");
      expect(feedback).toContain("validation passed");
    });

    it("should list errors and warnings for invalid models", () => {
      const feedback = PROMPT_TEMPLATES.VALIDATION_FEEDBACK({
        isValid: false,
        errors: ["Error 1", "Error 2"],
        warnings: ["Warning 1"],
      });

      expect(feedback).toContain("❌");
      expect(feedback).toContain("Error 1");
      expect(feedback).toContain("Error 2");
      expect(feedback).toContain("Warning 1");
    });
  });

  describe("extractModelCode", () => {
    it("should extract code from python markdown blocks", () => {
      const response = `\`\`\`python
gen = ModelGenerator()
a = gen.activity("A")
\`\`\``;

      const code = PROMPT_TEMPLATES.extractModelCode(response);
      expect(code).toContain('gen = ModelGenerator()');
      expect(code).toContain('gen.activity("A")');
    });

    it("should extract code from generic markdown blocks", () => {
      const response = `\`\`\`
gen.activity("A")
\`\`\``;

      const code = PROMPT_TEMPLATES.extractModelCode(response);
      expect(code).toContain('gen.activity("A")');
    });

    it("should return entire response if it looks like code", () => {
      const response = 'gen = ModelGenerator()\nactivity("A")';

      const code = PROMPT_TEMPLATES.extractModelCode(response);
      expect(code).toContain('gen = ModelGenerator()');
    });

    it("should return null if no code found", () => {
      const response = "Here's an explanation of the model...";

      const code = PROMPT_TEMPLATES.extractModelCode(response);
      expect(code).toBeNull();
    });
  });
});

describe("LLMPowLGenerator", () => {
  it("should initialize with system prompt", () => {
    const generator = new LLMPowLGenerator();
    const history = generator.getHistory();

    expect(history).toHaveLength(1);
    expect(history[0].role).toBe("system");
    expect(history[0].content).toContain("expert");
  });

  it("should generate prompt for process description", () => {
    const generator = new LLMPowLGenerator();
    const prompt = generator.generatePrompt(
      "Process orders from creation to shipping"
    );

    expect(prompt).toContain("Process orders from creation to shipping");
    expect(prompt).toContain("ModelGenerator");
  });

  it("should process LLM response and extract code", () => {
    const generator = new LLMPowLGenerator();
    const prompt = generator.generatePrompt("Simple process");

    // Add the user prompt to history (simulating sending it to LLM)
    generator.addUserMessage(prompt);

    const llmResponse = `\`\`\`python
gen = ModelGenerator()
a = gen.activity("A")
model = a
\`\`\``;

    const result = generator.processResponse(llmResponse);

    expect(result.modelCode).toContain('gen.activity("A")');
    expect(result.conversationHistory).toHaveLength(3); // system, user, assistant
  });

  it("should handle errors and generate refinement prompts", () => {
    const generator = new LLMPowLGenerator();
    const prompt = generator.generatePrompt("Process");

    // Add the user prompt to history (simulating sending it to LLM)
    generator.addUserMessage(prompt);

    // Add a fake assistant response to history (simulating LLM response)
    generator.addAssistantMessage("gen.activity('A')");

    // Now handle an error
    const refinement = generator.handleError("Irreflexivity violation detected");

    expect(refinement).toContain("Irreflexivity violation detected");
    expect(refinement).toContain("PREVIOUS ATTEMPT");
  });

  it("should reset conversation history", () => {
    const generator = new LLMPowLGenerator();
    generator.generatePrompt("Process");
    generator.reset();

    const history = generator.getHistory();
    expect(history).toHaveLength(1); // Only system prompt
    expect(history[0].role).toBe("system");
  });

  it("should generate validation feedback", () => {
    const generator = new LLMPowLGenerator();

    const feedback = generator.generateValidationFeedback({
      isValid: false,
      errors: ["Self-loop detected"],
      warnings: [],
    });

    expect(feedback).toContain("❌");
    expect(feedback).toContain("Self-loop detected");
  });
});

describe("ModelGenerator", () => {
  describe("activity creation", () => {
    it("should create activity node with label", () => {
      const gen = new ModelGenerator();
      const activity = gen.activity("Create order");

      expect(activity.type).toBe("activity");
      expect(activity.label).toBe("Create order");
      expect(activity.id).toMatch(/^node_\d+$/);
    });

    it("should generate unique IDs for each activity", () => {
      const gen = new ModelGenerator();
      const a1 = gen.activity("A");
      const a2 = gen.activity("B");

      expect(a1.id).not.toBe(a2.id);
    });
  });

  describe("XOR operator", () => {
    it("should create XOR with 2 or more children", () => {
      const gen = new ModelGenerator();
      const a = gen.activity("A");
      const b = gen.activity("B");

      const xor = gen.xor(a, b);

      expect(xor.type).toBe("xor");
      expect(xor.children).toHaveLength(2);
    });

    it("should throw error with less than 2 children", () => {
      const gen = new ModelGenerator();
      const a = gen.activity("A");

      expect(() => gen.xor(a)).toThrow("at least 2 arguments");
    });
  });

  describe("LOOP operator", () => {
    it("should create loop with do and redo parts", () => {
      const gen = new ModelGenerator();
      const doPart = gen.activity("Do");
      const redoPart = gen.activity("Redo");

      const loop = gen.loop(doPart, redoPart);

      expect(loop.type).toBe("loop");
      expect(loop.do).toEqual(doPart);
      expect(loop.redo).toEqual(redoPart);
    });

    it("should create loop with null redo part", () => {
      const gen = new ModelGenerator();
      const doPart = gen.activity("Do");

      const loop = gen.loop(doPart, null);

      expect(loop.type).toBe("loop");
      expect(loop.redo).toBeNull();
    });
  });

  describe("PARTIAL_ORDER operator", () => {
    it("should create partial order with dependencies", () => {
      const gen = new ModelGenerator();
      const a = gen.activity("A");
      const b = gen.activity("B");
      const c = gen.activity("C");

      const po = gen.partial_order({
        dependencies: [[a, b], [b, c]],
      });

      expect(po.type).toBe("partial_order");
      expect(po.dependencies).toHaveLength(2);
      expect(po.nodes).toHaveLength(3);
    });

    it("should throw error on self-loops", () => {
      const gen = new ModelGenerator();
      const a = gen.activity("A");

      expect(() =>
        gen.partial_order({
          dependencies: [[a, a]],
        })
      ).toThrow("Irreflexivity violation");
    });
  });

  describe("SEQUENCE operator", () => {
    it("should create sequence with children", () => {
      const gen = new ModelGenerator();
      const a = gen.activity("A");
      const b = gen.activity("B");
      const c = gen.activity("C");

      const seq = gen.sequence(a, b, c);

      expect(seq.type).toBe("sequence");
      expect(seq.children).toHaveLength(3);
    });

    it("should throw error with no children", () => {
      const gen = new ModelGenerator();

      expect(() => gen.sequence()).toThrow("at least 1 argument");
    });
  });

  describe("copy method", () => {
    it("should create deep copy of activity", () => {
      const gen = new ModelGenerator();
      const original = gen.activity("A");
      const copy = gen.copy(original) as ActivityNode;

      expect(copy.type).toBe(original.type);
      expect(copy.label).toBe(original.label);
      expect(copy.id).not.toBe(original.id);
    });

    it("should create deep copy of complex structure", () => {
      const gen = new ModelGenerator();
      const a = gen.activity("A");
      const b = gen.activity("B");
      const xor = gen.xor(a, b);

      const copy = gen.copy(xor) as any;

      expect(copy.type).toBe("xor");
      expect(copy.children).toHaveLength(2);
      expect(copy.children[0].label).toBe("A");
      expect(copy.id).not.toBe(xor.id);
    });
  });

  describe("toString conversion", () => {
    it("should convert activity to label string", () => {
      const gen = new ModelGenerator();
      const activity = gen.activity("Create Order");

      expect(gen.toString(activity)).toBe("Create Order");
    });

    it("should convert XOR to POWL format", () => {
      const gen = new ModelGenerator();
      const a = gen.activity("A");
      const b = gen.activity("B");
      const xor = gen.xor(a, b);

      const result = gen.toString(xor);

      expect(result).toContain("X");
      expect(result).toContain("A");
      expect(result).toContain("B");
    });

    it("should convert SEQUENCE to POWL format", () => {
      const gen = new ModelGenerator();
      const a = gen.activity("A");
      const b = gen.activity("B");
      const seq = gen.sequence(a, b);

      const result = gen.toString(seq);

      expect(result).toContain("S");
      expect(result).toContain("A");
      expect(result).toContain("B");
    });

    it("should convert LOOP to POWL format", () => {
      const gen = new ModelGenerator();
      const doPart = gen.activity("Do");
      const redoPart = gen.activity("Redo");
      const loop = gen.loop(doPart, redoPart);

      const result = gen.toString(loop);

      expect(result).toContain("*");
      expect(result).toContain("Do");
      expect(result).toContain("Redo");
    });
  });

  describe("validation", () => {
    it("should validate correct model", () => {
      const gen = new ModelGenerator();
      const a = gen.activity("A");
      const b = gen.activity("B");
      const xor = gen.xor(a, b);

      const validation = gen.validate(xor);

      expect(validation.isValid).toBe(true);
      expect(validation.errors).toHaveLength(0);
    });

    it("should detect XOR with insufficient children", () => {
      const gen = new ModelGenerator();
      const a = gen.activity("A");
      const invalidXor: any = { type: "xor", children: [a], id: "bad" };

      const validation = gen.validate(invalidXor);

      expect(validation.isValid).toBe(false);
      expect(validation.errors.length).toBeGreaterThan(0);
    });

    it("should detect empty activity labels", () => {
      const gen = new ModelGenerator();
      const invalidActivity: any = {
        type: "activity",
        label: "",
        id: "bad",
      };

      const validation = gen.validate(invalidActivity);

      expect(validation.isValid).toBe(false);
      expect(validation.errors).toContain("root: ACTIVITY node has empty label");
    });
  });

  describe("statistics", () => {
    it("should count nodes by type", () => {
      const gen = new ModelGenerator();
      const a = gen.activity("A");
      const b = gen.activity("B");
      const c = gen.activity("C");
      const xor = gen.xor(a, b);
      const seq = gen.sequence(xor, c);

      const stats = gen.getStatistics(seq);

      expect(stats.totalNodes).toBe(5); // 3 activities + 1 xor + 1 sequence
      expect(stats.nodeTypeCounts["activity"]).toBe(3);
      expect(stats.nodeTypeCounts["xor"]).toBe(1);
      expect(stats.nodeTypeCounts["sequence"]).toBe(1);
    });

    it("should collect activity labels", () => {
      const gen = new ModelGenerator();
      const a = gen.activity("Alpha");
      const b = gen.activity("Beta");
      const xor = gen.xor(a, b);

      const stats = gen.getStatistics(xor);

      expect(stats.activities).toEqual(["Alpha", "Beta"]);
    });

    it("should calculate max depth", () => {
      const gen = new ModelGenerator();
      const a = gen.activity("A");
      const b = gen.activity("B");
      const c = gen.activity("C");
      const xor = gen.xor(a, b);
      const seq = gen.sequence(xor, c);

      const stats = gen.getStatistics(seq);

      expect(stats.maxDepth).toBeGreaterThan(1);
    });
  });
});
