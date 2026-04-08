/**
 * ModelGenerator - Interface for POWL model construction
 *
 * This class provides the API used in the Kourani et al. paper's examples
 * for constructing POWL models programmatically.
 *
 * The generated model tree can be converted to POWL string representation
 * and parsed by the Powl WASM library.
 *
 * Usage:
 * ```ts
 * const gen = new ModelGenerator();
 * const create = gen.activity("Create order");
 * const check = gen.activity("Check order");
 * const accept = gen.activity("Accept order");
 * const reject = gen.activity("Reject order");
 *
 * const decision = gen.xor(accept, reject);
 * const model = gen.sequence(create, check, decision);
 *
 * const powlString = gen.toString(model);
 * // powlString -> "S( Create order, Check order, X( Accept order, Reject order ) )"
 * ```
 */

/**
 * Activity node (leaf in the model tree)
 */
export interface ActivityNode {
  type: "activity";
  label: string;
  id: string;
}

/**
 * XOR operator (exclusive choice between alternatives)
 */
export interface XorNode {
  type: "xor";
  children: ModelNode[];
  id: string;
}

/**
 * Loop operator (repeat-do or repeat-until)
 */
export interface LoopNode {
  type: "loop";
  do: ModelNode;
  redo: ModelNode | null;
  id: string;
}

/**
 * Strict partial order (concurrent activities with dependencies)
 */
export interface PartialOrderNode {
  type: "partial_order";
  dependencies: Array<[ModelNode, ModelNode]>;
  nodes: ModelNode[];
  id: string;
}

/**
 * Sequence operator (sequential composition)
 */
export interface SequenceNode {
  type: "sequence";
  children: ModelNode[];
  id: string;
}

/**
 * Any node in the POWL model tree
 */
export type ModelNode =
  | ActivityNode
  | XorNode
  | LoopNode
  | PartialOrderNode
  | SequenceNode;

/**
 * ModelGenerator class for constructing POWL models
 */
export class ModelGenerator {
  private nextId: number = 0;

  /**
   * Generate unique ID for node tracking
   */
  private generateId(): string {
    return `node_${this.nextId++}`;
  }

  /**
   * Create an activity node
   *
   * @param label The activity label (e.g., "Create order")
   * @returns Activity node
   */
  activity(label: string): ActivityNode {
    return {
      type: "activity",
      label,
      id: this.generateId(),
    };
  }

  /**
   * Create an XOR (exclusive choice) operator
   *
   * Represents mutually exclusive paths where exactly one branch is taken.
   * Requires at least 2 alternatives.
   *
   * @param children Alternative branches (must be >= 2)
   * @returns XOR node
   * @throws {Error} if less than 2 children provided
   */
  xor(...children: ModelNode[]): XorNode {
    if (children.length < 2) {
      throw new Error(
        `xor() requires at least 2 arguments, got ${children.length}`
      );
    }
    return {
      type: "xor",
      children,
      id: this.generateId(),
    };
  }

  /**
   * Create a LOOP operator
   *
   * Represents a repeating structure with a mandatory body ('do' part)
   * and an optional redo part. If redo is null, the loop repeats the do part.
   *
   * @param doPart The mandatory body of the loop
   * @param redoPart The optional redo part (null for repeat-until)
   * @returns Loop node
   */
  loop(doPart: ModelNode, redoPart: ModelNode | null = null): LoopNode {
    return {
      type: "loop",
      do: doPart,
      redo: redoPart,
      id: this.generateId(),
    };
  }

  /**
   * Create a STRICT PARTIAL ORDER operator
   *
   * Represents concurrent activities with ordering constraints.
   * Dependencies specify which activities must precede others.
   *
   * CRITICAL RULES:
   * - IRREFLEXIVE: No activity can precede itself (A,A is invalid)
   * - TRANSITIVE: If A→B and B→C, then A→C must be explicitly included
   *
   * @param config Configuration with dependencies array
   * @returns Partial order node
   * @throws {Error} if dependencies violate irreflexivity or transitivity
   */
  partial_order(config: {
    dependencies: Array<[ModelNode, ModelNode]>;
  }): PartialOrderNode {
    const { dependencies } = config;

    // Validate irreflexivity: no (A, A) dependencies
    for (const [source, target] of dependencies) {
      if (source.id === target.id) {
        throw new Error(
          `Irreflexivity violation: activity cannot depend on itself (${source.id} -> ${target.id})`
        );
      }
    }

    // Collect all unique nodes
    const nodeMap = new Map<string, ModelNode>();
    for (const [source, target] of dependencies) {
      nodeMap.set(source.id, source);
      nodeMap.set(target.id, target);
    }
    const nodes = Array.from(nodeMap.values());

    // Note: Transitivity validation is deferred to POWL parsing
    // The parser will check if all transitive dependencies are present

    return {
      type: "partial_order",
      dependencies,
      nodes,
      id: this.generateId(),
    };
  }

  /**
   * Create a SEQUENCE operator
   *
   * Represents sequential composition where activities execute in order.
   *
   * @param children Activities to execute in sequence
   * @returns Sequence node
   */
  sequence(...children: ModelNode[]): SequenceNode {
    if (children.length === 0) {
      throw new Error("sequence() requires at least 1 argument");
    }
    return {
      type: "sequence",
      children,
      id: this.generateId(),
    };
  }

  /**
   * Create a deep copy of a model node
   *
   * This is critical when reusing sub-models in multiple places.
   * Each copy gets a unique ID to avoid conflicts.
   *
   * @param node The node to copy
   * @returns A deep copy with new IDs
   */
  copy(node: ModelNode): ModelNode {
    switch (node.type) {
      case "activity":
        return this.activity(node.label);

      case "xor":
        return this.xor(...node.children.map((c) => this.copy(c)));

      case "loop":
        return this.loop(
          this.copy(node.do),
          node.redo ? this.copy(node.redo) : null
        );

      case "partial_order":
        const copiedDependencies = node.dependencies.map(([src, tgt]) => [
          this.copy(src),
          this.copy(tgt),
        ]) as Array<[ModelNode, ModelNode]>;
        return this.partial_order({ dependencies: copiedDependencies });

      case "sequence":
        return this.sequence(...node.children.map((c) => this.copy(c)));
    }
  }

  /**
   * Convert model tree to POWL string representation
   *
   * The output format matches the Python __repr__ format and can be
   * parsed by the Powl WASM library.
   *
   * @param model The root model node
   * @returns POWL string representation
   */
  toString(model: ModelNode): string {
    return this.nodeToString(model);
  }

  /**
   * Recursively convert a node to POWL string format
   */
  private nodeToString(node: ModelNode, depth = 0): string {
    switch (node.type) {
      case "activity":
        return node.label;

      case "xor": {
        const children = node.children
          .map((c) => this.nodeToString(c, depth + 1))
          .join(", ");
        return `X( ${children} )`;
      }

      case "loop": {
        const doStr = this.nodeToString(node.do, depth + 1);
        if (node.redo) {
          const redoStr = this.nodeToString(node.redo, depth + 1);
          return `*( ${doStr}, ${redoStr} )`;
        }
        return `->( ${doStr} )`;
      }

      case "partial_order": {
        // Extract all activity labels
        const activitySet = new Set<string>();
        const dependencyStrings: string[] = [];

        for (const [src, tgt] of node.dependencies) {
          const srcLabel = this.nodeToString(src, 0);
          const tgtLabel = this.nodeToString(tgt, 0);
          activitySet.add(srcLabel);
          activitySet.add(tgtLabel);
          dependencyStrings.push(`${srcLabel}-->${tgtLabel}`);
        }

        const nodes = Array.from(activitySet).join(", ");
        const deps = dependencyStrings.join(", ");

        return `PO=( nodes={ ${nodes} }, order={ ${deps} } )`;
      }

      case "sequence": {
        const children = node.children
          .map((c) => this.nodeToString(c, depth + 1))
          .join(", ");
        return `S( ${children} )`;
      }
    }
  }

  /**
   * Validate a model node structure
   *
   * Checks for common structural errors before converting to POWL string.
   *
   * @param node The node to validate
   * @returns Object with isValid flag and error messages
   */
  validate(node: ModelNode): {
    isValid: boolean;
    errors: string[];
    warnings: string[];
  } {
    const errors: string[] = [];
    const warnings: string[] = [];

    const validateNode = (n: ModelNode, path: string): void => {
      switch (n.type) {
        case "xor":
          if (n.children.length < 2) {
            errors.push(
              `${path}: XOR operator has ${n.children.length} children (requires at least 2)`
            );
          }
          n.children.forEach((child, i) =>
            validateNode(child, `${path}.children[${i}]`)
          );
          break;

        case "loop":
          if (!n.do) {
            errors.push(`${path}: LOOP operator missing 'do' part`);
          } else {
            validateNode(n.do, `${path}.do`);
          }
          if (n.redo) {
            validateNode(n.redo, `${path}.redo`);
          }
          break;

        case "partial_order":
          // Check for duplicate dependencies
          const depSet = new Set<string>();
          for (const [src, tgt] of n.dependencies) {
            const key = `${src.id}->${tgt.id}`;
            if (depSet.has(key)) {
              warnings.push(
                `${path}: Duplicate dependency detected: ${key}`
              );
            }
            depSet.add(key);
          }

          // Validate all dependency nodes
          n.dependencies.forEach(([src, tgt], i) => {
            validateNode(src, `${path}.dependencies[${i}][0]`);
            validateNode(tgt, `${path}.dependencies[${i}][1]`);
          });
          break;

        case "sequence":
          if (n.children.length === 0) {
            errors.push(`${path}: SEQUENCE operator has no children`);
          }
          n.children.forEach((child, i) =>
            validateNode(child, `${path}.children[${i}]`)
          );
          break;

        case "activity":
          if (!n.label || n.label.trim() === "") {
            errors.push(`${path}: ACTIVITY node has empty label`);
          }
          break;
      }
    };

    validateNode(node, "root");

    return {
      isValid: errors.length === 0,
      errors,
      warnings,
    };
  }

  /**
   * Get statistics about a model tree
   *
   * @param node The root node
   * @returns Statistics object
   */
  getStatistics(node: ModelNode): {
    totalNodes: number;
    nodeTypeCounts: Record<string, number>;
    maxDepth: number;
    activities: string[];
  } {
    const nodeTypeCounts: Record<string, number> = {};
    const activities: string[] = [];
    let maxDepth = 0;

    const traverse = (n: ModelNode, depth: number): void => {
      maxDepth = Math.max(maxDepth, depth);
      nodeTypeCounts[n.type] = (nodeTypeCounts[n.type] || 0) + 1;

      if (n.type === "activity") {
        activities.push(n.label);
      }

      switch (n.type) {
        case "xor":
        case "sequence":
          n.children.forEach((c) => traverse(c, depth + 1));
          break;
        case "loop":
          traverse(n.do, depth + 1);
          if (n.redo) traverse(n.redo, depth + 1);
          break;
        case "partial_order":
          n.dependencies.forEach(([src, tgt]) => {
            traverse(src, depth + 1);
            traverse(tgt, depth + 1);
          });
          break;
        case "activity":
          // Leaf node, no children
          break;
      }
    };

    traverse(node, 0);

    return {
      totalNodes: Object.values(nodeTypeCounts).reduce((a, b) => a + b, 0),
      nodeTypeCounts,
      maxDepth,
      activities: [...new Set(activities)].sort(),
    };
  }
}

/**
 * Create a default ModelGenerator instance
 */
export function createModelGenerator(): ModelGenerator {
  return new ModelGenerator();
}
