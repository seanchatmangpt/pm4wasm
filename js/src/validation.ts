/**
 * Comprehensive POWL model validation based on soundness guarantees
 * from the POWL paper: irreflexivity, transitivity, proper completion,
 * and no unreachable parts.
 */

import type { PowlModel } from "./index.js";
import type { NodeInfo } from "./types.js";

// ─── Validation Types ────────────────────────────────────────────────────────

export interface ValidationResult {
  isValid: boolean;
  errors: ValidationError[];
  warnings: ValidationWarning[];
  soundness: SoundnessReport;
}

export interface ValidationError {
  type: "irreflexivity" | "transitivity" | "syntax" | "reference" | "completion";
  message: string;
  node?: string;
  severity: "critical" | "error";
}

export interface ValidationWarning {
  type: "reuse" | "complexity" | "naming" | "unreachable";
  message: string;
  node?: string;
}

export interface SoundnessReport {
  isSound: boolean;
  deadlockFree: boolean;
  properCompletion: boolean;
  noUnreachableParts: boolean;
}

// ─── Validator Implementation ─────────────────────────────────────────────────

/**
 * Comprehensive POWL model validator
 *
 * Checks soundness properties from the POWL paper:
 * 1. Irreflexivity: No self-loops in partial orders
 * 2. Transitivity: If A→B and B→C, then A→C
 * 3. Proper completion: All paths can reach end
 * 4. No unreachable parts: All nodes reachable from root
 */
export class PowlValidator {
  /**
   * Validate a POWL model for soundness and correctness
   */
  static validate(model: PowlModel): ValidationResult {
    const errors: ValidationError[] = [];
    const warnings: ValidationWarning[] = [];

    // Get all nodes in the model
    const nodes = this.getAllNodes(model);

    // Check 1: Irreflexivity (no self-loops in partial orders)
    const irreflexivityErrors = this.checkIrreflexivity(model, nodes);
    errors.push(...irreflexivityErrors);

    // Check 2: Transitivity (if A->B and B->C, then A->C)
    const transitivityErrors = this.checkTransitivity(model, nodes);
    errors.push(...transitivityErrors);

    // Check 3: No unreachable parts
    const unreachableWarnings = this.checkUnreachableParts(model, nodes);
    warnings.push(...unreachableWarnings);

    // Check 4: Sub-model reuse (should use copy())
    const reuseWarnings = this.checkSubModelReuse(model, nodes);
    warnings.push(...reuseWarnings);

    // Check 5: Proper completion (all paths end properly)
    const completionErrors = this.checkProperCompletion(model, nodes);
    errors.push(...completionErrors);

    // Check 6: Syntax validation
    const syntaxErrors = this.checkSyntax(model, nodes);
    errors.push(...syntaxErrors);

    // Check 7: Reference integrity
    const referenceErrors = this.checkReferences(model, nodes);
    errors.push(...referenceErrors);

    // Generate soundness report
    const soundness: SoundnessReport = {
      isSound: errors.length === 0,
      deadlockFree: !errors.some((e) => e.type === "irreflexivity"),
      properCompletion: !errors.some((e) => e.type === "completion"),
      noUnreachableParts: unreachableWarnings.length === 0,
    };

    return {
      isValid: errors.filter((e) => e.severity === "critical").length === 0,
      errors,
      warnings,
      soundness,
    };
  }

  /**
   * Check for irreflexivity violations (self-loops in partial orders)
   *
   * From the POWL paper: Strict partial orders must be irreflexive,
   * meaning no element is related to itself.
   */
  private static checkIrreflexivity(
    _model: PowlModel,
    nodes: NodeWithIndex[],
  ): ValidationError[] {
    const errors: ValidationError[] = [];

    for (const node of nodes) {
      if (node.type === "StrictPartialOrder") {
        const edges = node.edges || [];
        for (const [from, to] of edges) {
          if (from === to) {
            errors.push({
              type: "irreflexivity",
              message: `Self-loop detected: node ${from} --> ${from} violates irreflexivity (strict partial orders cannot have self-loops)`,
              node: from.toString(),
              severity: "error",
            });
          }
        }
      }
    }

    return errors;
  }

  /**
   * Check for transitivity violations in partial orders
   *
   * From the POWL paper: Strict partial orders must be transitive.
   * If A→B and B→C, then A→C must be present.
   */
  private static checkTransitivity(
    _model: PowlModel,
    nodes: NodeWithIndex[],
  ): ValidationError[] {
    const errors: ValidationError[] = [];

    // Build adjacency map for all SPO nodes
    const spoNodes = nodes.filter((n) => n.type === "StrictPartialOrder");

    for (const spoNode of spoNodes) {
      const edges = spoNode.edges || [];
      const adj = new Map<number, Set<number>>();

      // Build adjacency list
      for (const [from, to] of edges) {
        if (!adj.has(from)) adj.set(from, new Set());
        adj.get(from)!.add(to);
      }

      // Check transitivity: if A->B and B->C, must have A->C
      for (const [a, targets] of adj) {
        for (const b of targets) {
          if (adj.has(b)) {
            for (const c of adj.get(b)!) {
              if (!targets.has(c)) {
                errors.push({
                  type: "transitivity",
                  message: `Transitivity violation in SPO node ${spoNode.index}: ${a}-->${b} and ${b}-->${c} requires ${a}-->${c}`,
                  node: spoNode.index.toString(),
                  severity: "error",
                });
              }
            }
          }
        }
      }
    }

    return errors;
  }

  /**
   * Check for unreachable parts (dead code)
   *
   * All nodes should be reachable from the root through the tree structure.
   */
  private static checkUnreachableParts(
    model: PowlModel,
    _nodes: NodeWithIndex[],
  ): ValidationWarning[] {
    const warnings: ValidationWarning[] = [];

    // Find root node
    const rootIdx = model.root;
    const reachable = new Set<number>([rootIdx]);

    // BFS from root to find all reachable nodes
    const queue = [rootIdx];
    while (queue.length > 0) {
      const current = queue.shift()!;
      const children = model.children(current);

      for (const child of children) {
        if (!reachable.has(child)) {
          reachable.add(child);
          queue.push(child);
        }
      }
    }

    // Check for unreachable nodes
    for (let i = 0; i < model.size; i++) {
      if (!reachable.has(i)) {
        const info = model.nodeInfo(i);
        const label =
          info.type === "Transition"
            ? info.label
            : info.type === "OperatorPowl"
              ? info.operator
              : info.type;
        warnings.push({
          type: "unreachable",
          message: `Unreachable node: ${label} at index ${i} (not reachable from root)`,
          node: i.toString(),
        });
      }
    }

    return warnings;
  }

  /**
   * Check for sub-model reuse without proper copying
   *
   * POWL models should use copy() when reusing sub-models to avoid
   * unintended sharing. This is a heuristic check.
   */
  private static checkSubModelReuse(
    model: PowlModel,
    nodes: NodeWithIndex[],
  ): ValidationWarning[] {
    const warnings: ValidationWarning[] = [];

    // Look for duplicate structures that might indicate reuse without copy
    const structureSignatures = new Map<string, number[]>();

    for (const node of nodes) {
      if (node.type === "OperatorPowl") {
        // Create a signature for this node's structure
        const signature = this.computeStructureSignature(model, node.index);
        if (!structureSignatures.has(signature)) {
          structureSignatures.set(signature, []);
        }
        structureSignatures.get(signature)!.push(node.index);
      }
    }

    // Flag duplicates as potential reuse issues
    for (const [_signature, indices] of structureSignatures) {
      if (indices.length > 1) {
        warnings.push({
          type: "reuse",
          message: `Duplicate structure detected at nodes ${indices.join(
            ", ",
          )}. Ensure sub-model reuse uses copy() to avoid unintended sharing`,
        });
      }
    }

    return warnings;
  }

  /**
   * Check proper completion (all paths can reach end)
   *
   * This is a simplified check - a full implementation would require
   * exhaustive path analysis through the model.
   */
  private static checkProperCompletion(
    _model: PowlModel,
    nodes: NodeWithIndex[],
  ): ValidationError[] {
    const errors: ValidationError[] = [];

    // Check for loops without exit
    for (const node of nodes) {
      if (node.type === "OperatorPowl" && node.operator === "Loop") {
        // Loop should have at least 2 children (body, exit)
        if (node.children.length < 2) {
          errors.push({
            type: "completion",
            message: `Loop node ${node.index} has insufficient children (${node.children.length}), may not have proper exit path`,
            node: node.index.toString(),
            severity: "error",
          });
        }
      }
    }

    // Check for XOR without any children
    for (const node of nodes) {
      if (node.type === "OperatorPowl" && node.operator === "Xor") {
        if (node.children.length === 0) {
          errors.push({
            type: "completion",
            message: `XOR node ${node.index} has no children, creates deadlock`,
            node: node.index.toString(),
            severity: "critical",
          });
        }
      }
    }

    return errors;
  }

  /**
   * Check for syntax errors in the model
   */
  private static checkSyntax(
    _model: PowlModel,
    nodes: NodeWithIndex[],
  ): ValidationError[] {
    const errors: ValidationError[] = [];

    // Check for invalid node types
    for (const node of nodes) {
      if (node.type === "Invalid") {
        errors.push({
          type: "syntax",
          message: `Invalid node detected at index ${node.index}`,
          node: node.index.toString(),
          severity: "critical",
        });
      }
    }

    // Check for empty labels in transitions (except tau)
    for (const node of nodes) {
      if (node.type === "Transition" && node.label === "") {
        errors.push({
          type: "syntax",
          message: `Transition at index ${node.index} has empty label (use "tau" for silent transitions)`,
          node: node.index.toString(),
          severity: "error",
        });
      }
    }

    return errors;
  }

  /**
   * Check for reference integrity (all child references valid)
   */
  private static checkReferences(
    _model: PowlModel,
    nodes: NodeWithIndex[],
  ): ValidationError[] {
    const errors: ValidationError[] = [];
    const validIndices = new Set(nodes.map((n) => n.index));

    for (const node of nodes) {
      if (
        node.type === "StrictPartialOrder" ||
        node.type === "OperatorPowl"
      ) {
        for (const childIdx of node.children) {
          if (!validIndices.has(childIdx)) {
            errors.push({
              type: "reference",
              message: `Invalid child reference: node ${node.index} references non-existent child ${childIdx}`,
              node: node.index.toString(),
              severity: "critical",
            });
          }
        }
      }

      if (node.type === "StrictPartialOrder") {
        for (const [from, to] of node.edges || []) {
          if (!validIndices.has(from)) {
            errors.push({
              type: "reference",
              message: `Invalid edge reference: edge from non-existent node ${from}`,
              node: node.index.toString(),
              severity: "error",
            });
          }
          if (!validIndices.has(to)) {
            errors.push({
              type: "reference",
              message: `Invalid edge reference: edge to non-existent node ${to}`,
              node: node.index.toString(),
              severity: "error",
            });
          }
        }
      }
    }

    return errors;
  }

  /**
   * Get all nodes in the model with their indices
   */
  private static getAllNodes(model: PowlModel): NodeWithIndex[] {
    const nodes: NodeWithIndex[] = [];
    const visited = new Set<number>();

    const traverse = (idx: number): void => {
      if (visited.has(idx)) return;
      visited.add(idx);

      const info = model.nodeInfo(idx);
      nodes.push({ index: idx, ...info });

      const children = model.children(idx);
      for (const child of children) {
        traverse(child);
      }
    };

    traverse(model.root);
    return nodes;
  }

  /**
   * Compute a structural signature for a node (for duplicate detection)
   */
  private static computeStructureSignature(
    model: PowlModel,
    idx: number,
  ): string {
    const info = model.nodeInfo(idx);
    const children = model.children(idx);

    if (info.type === "Transition") {
      return `T:${info.label}`;
    } else if (info.type === "OperatorPowl") {
      const childSigs = children.map((c) =>
        this.computeStructureSignature(model, c),
      );
      return `O:${info.operator}(${childSigs.join(",")})`;
    } else if (info.type === "StrictPartialOrder") {
      return `SPO:[${children.join(",")}]`;
    }
    return "?";
  }
}

// ─── Type Helpers ─────────────────────────────────────────────────────────────

type NodeWithIndex = NodeInfo & { index: number };

// ─── Formatting Utilities ─────────────────────────────────────────────────────

/**
 * Format validation results for display
 */
export function formatValidationResult(result: ValidationResult): string {
  const lines: string[] = [];

  lines.push("=== POWL Model Validation ===\n");

  // Soundness report
  lines.push("Soundness:");
  lines.push(`  Is Sound: ${result.soundness.isSound ? "✅" : "❌"}`);
  lines.push(`  Deadlock-Free: ${result.soundness.deadlockFree ? "✅" : "❌"}`);
  lines.push(
    `  Proper Completion: ${result.soundness.properCompletion ? "✅" : "❌"}`,
  );
  lines.push(
    `  No Unreachable Parts: ${result.soundness.noUnreachableParts ? "✅" : "❌"}`,
  );
  lines.push("");

  // Errors
  if (result.errors.length > 0) {
    lines.push(`Errors (${result.errors.length}):`);
    result.errors.forEach((err) => {
      const severity = err.severity === "critical" ? "🚨" : "❌";
      lines.push(`  ${severity} [${err.type}] ${err.message}`);
      if (err.node) lines.push(`     Node: ${err.node}`);
    });
    lines.push("");
  }

  // Warnings
  if (result.warnings.length > 0) {
    lines.push(`Warnings (${result.warnings.length}):`);
    result.warnings.forEach((warn) => {
      lines.push(`  ⚠️  [${warn.type}] ${warn.message}`);
      if (warn.node) lines.push(`     Node: ${warn.node}`);
    });
  }

  // Overall status
  lines.push("");
  lines.push(result.isValid ? "✅ Model is VALID" : "❌ Model has ERRORS");

  return lines.join("\n");
}

/**
 * Get a short validation summary (one line)
 */
export function getValidationSummary(result: ValidationResult): string {
  const errorCount = result.errors.length;
  const warningCount = result.warnings.length;
  const sound = result.soundness.isSound ? "SOUND" : "UNSound";

  if (errorCount === 0 && warningCount === 0) {
    return `✅ VALID (${sound})`;
  } else if (errorCount === 0) {
    return `⚠️  VALID with ${warningCount} warning(s) (${sound})`;
  } else {
    return `❌ INVALID: ${errorCount} error(s), ${warningCount} warning(s) (${sound})`;
  }
}
