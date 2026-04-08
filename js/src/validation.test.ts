/**
 * Tests for POWL model validation
 *
 * These tests verify the validation utilities work correctly
 * for soundness checking and error detection.
 */

import { describe, it, expect } from "vitest";
import { PowlValidator, formatValidationResult, getValidationSummary } from "./validation.js";
import type { ValidationError } from "./validation.js";

// Mock PowlModel for testing (since we can't load WASM in unit tests)
class MockPowlModel {
  root: number;
  size: number;

  constructor(
    root: number,
    size: number,
    private nodeMap: Map<number, any>,
    private childrenMap: Map<number, number[]>,
  ) {
    this.root = root;
    this.size = size;
  }

  nodeInfo(idx: number): any {
    return this.nodeMap.get(idx) || { type: "Invalid" };
  }

  children(idx: number): number[] {
    return this.childrenMap.get(idx) || [];
  }

  // Mock method to satisfy interface
  toString(): string {
    return "X(A, B)";
  }

  validate(): void {
    // Mock implementation
  }

  simplify(): any {
    return this;
  }

  simplifyFrequent(): any {
    return this;
  }

  toPetriNet(): any {
    return {};
  }

  footprints(): any {
    return {};
  }

  walk(_visitor: unknown): void {
    // Mock implementation
  }

  activities(): Set<string> {
    return new Set();
  }

  orderEdges(_idx: number): number[] {
    return [];
  }

  closureEdges(_idx: number): number[] {
    return [];
  }

  reductionEdges(_idx: number): number[] {
    return [];
  }
}

describe("PowlValidator", () => {
  describe("checkIrreflexivity", () => {
    it("should detect self-loops in partial orders", () => {
      const model = new MockPowlModel(
        0,
        3,
        new Map([
          [
            0,
            {
              type: "StrictPartialOrder",
              id: 0,
              children: [1, 2],
              edges: [
                [1, 1], // Self-loop
                [1, 2],
              ],
            },
          ],
          [1, { type: "Transition", label: "A", id: 1 }],
          [2, { type: "Transition", label: "B", id: 2 }],
        ]),
        new Map([
          [0, [1, 2]],
          [1, []],
          [2, []],
        ]),
      );

      const result = PowlValidator.validate(model as any);

      expect(result.errors).toHaveLength(1);
      expect(result.errors[0].type).toBe("irreflexivity");
      expect(result.errors[0].node).toBe("1");
      expect(result.soundness.deadlockFree).toBe(false);
    });

    it("should pass when no self-loops exist", () => {
      const model = new MockPowlModel(
        0,
        3,
        new Map([
          [
            0,
            {
              type: "StrictPartialOrder",
              id: 0,
              children: [1, 2],
              edges: [
                [1, 2],
                [2, 3],
              ],
            },
          ],
          [1, { type: "Transition", label: "A", id: 1 }],
          [2, { type: "Transition", label: "B", id: 2 }],
          [3, { type: "Transition", label: "C", id: 3 }],
        ]),
        new Map([
          [0, [1, 2, 3]],
          [1, []],
          [2, []],
          [3, []],
        ]),
      );

      const result = PowlValidator.validate(model as any);

      expect(result.errors.filter((e: ValidationError) => e.type === "irreflexivity")).toHaveLength(
        0,
      );
      expect(result.soundness.deadlockFree).toBe(true);
    });
  });

  describe("checkTransitivity", () => {
    it("should detect transitivity violations", () => {
      const model = new MockPowlModel(
        0,
        3,
        new Map([
          [
            0,
            {
              type: "StrictPartialOrder",
              id: 0,
              children: [1, 2, 3],
              edges: [
                [1, 2],
                [2, 3],
                // Missing [1, 3]
              ],
            },
          ],
          [1, { type: "Transition", label: "A", id: 1 }],
          [2, { type: "Transition", label: "B", id: 2 }],
          [3, { type: "Transition", label: "C", id: 3 }],
        ]),
        new Map([
          [0, [1, 2, 3]],
          [1, []],
          [2, []],
          [3, []],
        ]),
      );

      const result = PowlValidator.validate(model as any);

      expect(result.errors.some((e: ValidationError) => e.type === "transitivity")).toBe(true);
    });

    it("should pass when transitivity holds", () => {
      const model = new MockPowlModel(
        0,
        3,
        new Map([
          [
            0,
            {
              type: "StrictPartialOrder",
              id: 0,
              children: [1, 2, 3],
              edges: [
                [1, 2],
                [2, 3],
                [1, 3], // Transitive edge present
              ],
            },
          ],
          [1, { type: "Transition", label: "A", id: 1 }],
          [2, { type: "Transition", label: "B", id: 2 }],
          [3, { type: "Transition", label: "C", id: 3 }],
        ]),
        new Map([
          [0, [1, 2, 3]],
          [1, []],
          [2, []],
          [3, []],
        ]),
      );

      const result = PowlValidator.validate(model as any);

      expect(result.errors.filter((e) => e.type === "transitivity")).toHaveLength(
        0,
      );
    });
  });

  describe("checkUnreachableParts", () => {
    it("should detect unreachable nodes", () => {
      const model = new MockPowlModel(
        0,
        4,
        new Map([
          [0, { type: "OperatorPowl", operator: "Sequence", id: 0, children: [1, 2] }],
          [1, { type: "Transition", label: "A", id: 1 }],
          [2, { type: "Transition", label: "B", id: 2 }],
          [3, { type: "Transition", label: "C", id: 3 }], // Unreachable
        ]),
        new Map([
          [0, [1, 2]],
          [1, []],
          [2, []],
          [3, []],
        ]),
      );

      const result = PowlValidator.validate(model as any);

      expect(result.warnings.some((w) => w.type === "unreachable")).toBe(true);
      expect(result.soundness.noUnreachableParts).toBe(false);
    });

    it("should pass when all nodes are reachable", () => {
      const model = new MockPowlModel(
        0,
        3,
        new Map([
          [0, { type: "OperatorPowl", operator: "Sequence", id: 0, children: [1, 2] }],
          [1, { type: "Transition", label: "A", id: 1 }],
          [2, { type: "Transition", label: "B", id: 2 }],
        ]),
        new Map([
          [0, [1, 2]],
          [1, []],
          [2, []],
        ]),
      );

      const result = PowlValidator.validate(model as any);

      expect(result.warnings.filter((w) => w.type === "unreachable")).toHaveLength(
        0,
      );
      expect(result.soundness.noUnreachableParts).toBe(true);
    });
  });

  describe("checkProperCompletion", () => {
    it("should detect XOR with no children", () => {
      const model = new MockPowlModel(
        0,
        1,
        new Map([
          [0, { type: "OperatorPowl", operator: "Xor", id: 0, children: [] }],
        ]),
        new Map([
          [0, []],
        ]),
      );

      const result = PowlValidator.validate(model as any);

      expect(result.errors.some((e) => e.type === "completion")).toBe(true);
      expect(result.errors.some((e) => e.severity === "critical")).toBe(true);
    });

    it("should detect Loop with insufficient children", () => {
      const model = new MockPowlModel(
        0,
        2,
        new Map([
          [
            0,
            { type: "OperatorPowl", operator: "Loop", id: 0, children: [1] },
          ],
          [1, { type: "Transition", label: "A", id: 1 }],
        ]),
        new Map([
          [0, [1]],
          [1, []],
        ]),
      );

      const result = PowlValidator.validate(model as any);

      expect(result.errors.some((e) => e.type === "completion")).toBe(true);
    });
  });

  describe("checkSyntax", () => {
    it("should detect invalid nodes", () => {
      const model = new MockPowlModel(
        0,
        1,
        new Map([[0, { type: "Invalid", id: 0 }]]),
        new Map([
          [0, []],
        ]),
      );

      const result = PowlValidator.validate(model as any);

      expect(result.errors.some((e) => e.type === "syntax")).toBe(true);
      expect(result.errors.some((e) => e.severity === "critical")).toBe(true);
    });

    it("should detect empty transition labels", () => {
      const model = new MockPowlModel(
        0,
        1,
        new Map([[0, { type: "Transition", label: "", id: 0 }]]),
        new Map([
          [0, []],
        ]),
      );

      const result = PowlValidator.validate(model as any);

      expect(result.errors.some((e) => e.type === "syntax")).toBe(true);
    });
  });

  describe("checkReferences", () => {
    it("should detect invalid child references", () => {
      const model = new MockPowlModel(
        0,
        1,
        new Map([
          [
            0,
            { type: "OperatorPowl", operator: "Sequence", id: 0, children: [99] },
          ],
        ]),
        new Map([
          [0, []],
        ]),
      );

      const result = PowlValidator.validate(model as any);

      expect(result.errors.some((e) => e.type === "reference")).toBe(true);
      expect(result.errors.some((e) => e.severity === "critical")).toBe(true);
    });

    it("should detect invalid edge references in SPO", () => {
      const model = new MockPowlModel(
        0,
        1,
        new Map([
          [
            0,
            {
              type: "StrictPartialOrder",
              id: 0,
              children: [],
              edges: [
                [99, 1],
              ],
            },
          ],
        ]),
        new Map([
          [0, []],
        ]),
      );

      const result = PowlValidator.validate(model as any);

      expect(result.errors.some((e) => e.type === "reference")).toBe(true);
    });
  });

  describe("Soundness Report", () => {
    it("should report sound when all checks pass", () => {
      const model = new MockPowlModel(
        0,
        3,
        new Map([
          [
            0,
            {
              type: "StrictPartialOrder",
              id: 0,
              children: [1, 2],
              edges: [
                [1, 2],
              ],
            },
          ],
          [1, { type: "Transition", label: "A", id: 1 }],
          [2, { type: "Transition", label: "B", id: 2 }],
        ]),
        new Map([
          [0, [1, 2]],
          [1, []],
          [2, []],
        ]),
      );

      const result = PowlValidator.validate(model as any);

      expect(result.soundness.isSound).toBe(true);
      expect(result.soundness.deadlockFree).toBe(true);
      expect(result.soundness.properCompletion).toBe(true);
      expect(result.soundness.noUnreachableParts).toBe(true);
      expect(result.isValid).toBe(true);
    });

    it("should report unsound when errors exist", () => {
      const model = new MockPowlModel(
        0,
        1,
        new Map([
          [0, { type: "Invalid", id: 0 }],
        ]),
        new Map([
          [0, []],
        ]),
      );

      const result = PowlValidator.validate(model as any);

      expect(result.soundness.isSound).toBe(false);
      expect(result.isValid).toBe(false);
    });
  });
});

describe("formatValidationResult", () => {
  it("should format valid result", () => {
    const result = {
      isValid: true,
      errors: [],
      warnings: [],
      soundness: {
        isSound: true,
        deadlockFree: true,
        properCompletion: true,
        noUnreachableParts: true,
      },
    };

    const formatted = formatValidationResult(result);

    expect(formatted).toContain("✅ Model is VALID");
    expect(formatted).toContain("Is Sound: ✅");
    expect(formatted).toContain("Deadlock-Free: ✅");
  });

  it("should format invalid result with errors", () => {
    const result = {
      isValid: false,
      errors: [
        {
          type: "irreflexivity" as const,
          message: "Self-loop detected",
          node: "1",
          severity: "error" as const,
        },
      ],
      warnings: [
        {
          type: "unreachable" as const,
          message: "Unreachable node",
          node: "2",
        },
      ],
      soundness: {
        isSound: false,
        deadlockFree: false,
        properCompletion: true,
        noUnreachableParts: false,
      },
    };

    const formatted = formatValidationResult(result);

    expect(formatted).toContain("❌ Model has ERRORS");
    expect(formatted).toContain("Errors (1)");
    expect(formatted).toContain("Warnings (1)");
    expect(formatted).toContain("[irreflexivity]");
    expect(formatted).toContain("[unreachable]");
  });
});

describe("getValidationSummary", () => {
  it("should summarize valid model", () => {
    const result = {
      isValid: true,
      errors: [],
      warnings: [],
      soundness: {
        isSound: true,
        deadlockFree: true,
        properCompletion: true,
        noUnreachableParts: true,
      },
    };

    const summary = getValidationSummary(result);

    expect(summary).toBe("✅ VALID (SOUND)");
  });

  it("should summarize model with warnings", () => {
    const result = {
      isValid: true,
      errors: [],
      warnings: [
        {
          type: "unreachable" as const,
          message: "Unreachable node",
          node: "2",
        },
      ],
      soundness: {
        isSound: false,
        deadlockFree: true,
        properCompletion: true,
        noUnreachableParts: false,
      },
    };

    const summary = getValidationSummary(result);

    expect(summary).toContain("⚠️");
    expect(summary).toContain("1 warning(s)");
    expect(summary).toContain("(UNSound)");
  });

  it("should summarize invalid model", () => {
    const result = {
      isValid: false,
      errors: [
        {
          type: "irreflexivity" as const,
          message: "Self-loop",
          severity: "error" as const,
        },
      ],
      warnings: [
        {
          type: "unreachable" as const,
          message: "Unreachable node",
        },
      ],
      soundness: {
        isSound: false,
        deadlockFree: false,
        properCompletion: true,
        noUnreachableParts: false,
      },
    };

    const summary = getValidationSummary(result);

    expect(summary).toBe("❌ INVALID: 1 error(s), 1 warning(s) (UNSound)");
  });
});
