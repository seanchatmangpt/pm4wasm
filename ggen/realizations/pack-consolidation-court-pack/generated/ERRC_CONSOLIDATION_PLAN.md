# ERRC Pack Consolidation Plan

This file is generated from the consolidation court RDF. It is a projection, not an editing surface.

| Priority | Disposition | Canonical authority | Members | Status | Migration |
|---:|---|---|---|---|---|
| 1 | EXTRACT_SHARED_ONTOLOGY | `dfcm-pack` | dfcm-explore-candidate-factory-pack, dfcm-maximalist-selection-control-pack, dfcm-explore-court-pack, dfcm-maximalist-court-pack, dfcm-selection-capital-pack, dfcm-pack | OPEN | true |
| 1 | DEPRECATE_INSTANCE | `chatman-ecosystem-release-pack` | chatman-ecosystem-v26-9-1-release-gate, chatman-ecosystem-release-pack | MIGRATION_STARTED | true |
| 1 | EXTRACT_SHARED_ONTOLOGY | `temporal-truth-observability-pack` | replication-epistemic-observability-pack, portfolio-epistemic-observability-pack, temporal-truth-observability-pack, control-plane-causality-observability-pack, run-protocol-observability-pack | OPEN | true |
| 1 | HARD_MERGE | `ggen-self-pack` | pack-authoring-pack, ggen-self-pack | MIGRATION_STARTED | true |
| 1 | HARD_MERGE | `runtime-evidence-authenticity-pack` | runtime-evidence-authenticity-control-pack, runtime-evidence-authenticity-pack | MIGRATION_STARTED | true |
| 1 | EXTRACT_SHARED_ONTOLOGY | `wasm4pm-facts-pack` | wasm4pm-cognition-pack, wasm4pm-algorithms-pack, wasm4pm-facts-pack | OPEN | true |
| 2 | META_COMPOSE | `manufacturing-fanout-controller-pack` | fanout-realization-controller-pack, consumer-realization-frontier-pack, capability-lineage-propagation-pack, manufacturing-fanout-controller-pack, multicell-consumer-factory-pack, forced-top25-admissibility-factory-pack | OPEN | false |
| 2 | HARD_MERGE | `evidence-capital-policy-realization-pack` | evidence-capital-policy-adaptive-control-pack, evidence-capital-policy-realization-pack | OPEN | true |
| 3 | META_COMPOSE | `repo-reconciliation-pack` | repo-reconciliation-pack, temporary-works-pack, repo-intervention-pack, repo-load-path-pack, repo-as-found-pack | KEEP_MODULES | false |
| 4 | KEEP_DISTINCT | `clap-noun-verb-schema-pack` | clap-noun-verb-verification-pack, clap-noun-verb-boundary-pack, clap-noun-verb-behavior-pack, clap-noun-verb-routing-pack, clap-noun-verb-crate-pack, clap-noun-verb-schema-pack | HEALTHY | false |


## Reasons

### EXTRACT_SHARED_ONTOLOGY — `dfcm-pack`

Candidate, CTQ, evidence, falsifier, dependency, rollback, standing and authority are duplicated across sel:, dmc: and dms: namespaces.

### DEPRECATE_INSTANCE — `chatman-ecosystem-release-pack`

The generic pack owns reusable exact-SHA release law; v26.9.1 is a release instance/profile, not a new semantic authority.

### EXTRACT_SHARED_ONTOLOGY — `temporal-truth-observability-pack`

Request, Receipt, MemoryRecord, ExactSubject, standing and currentness are independently reminted; keep projections but share identity law.

### HARD_MERGE — `ggen-self-pack`

ggen-self-pack already declares itself the canonical constructor for pack structure; pack-authoring remains compatibility-only until its authoring surfaces are absorbed.

### HARD_MERGE — `runtime-evidence-authenticity-pack`

Both packs carry the same exact-subject identity and authority envelope; control is a projection over authenticity measurement, not a second semantic authority.

### EXTRACT_SHARED_ONTOLOGY — `wasm4pm-facts-pack`

wasm4pm-facts-pack records repeated drift of identical algorithm IRIs carried by wasm4pm-algorithms-pack; facts must become single-source while dispatch packs remain projections.

### META_COMPOSE — `manufacturing-fanout-controller-pack`

These are valid manufacturing stages but should present one canonical bundle and shared Consumer/Factory/Lineage/Qualification vocabulary.

### HARD_MERGE — `evidence-capital-policy-realization-pack`

Adaptive control consumes the realization measurement surface; keep phase separation but remove independent install-time semantic ownership.

### META_COMPOSE — `repo-reconciliation-pack`

The family is an explicit dependency DAG; consolidation should add a root meta-pack without flattening reusable stages.

### KEEP_DISTINCT — `clap-noun-verb-schema-pack`

Distinct compiler phases with a zero-config starter already composing them; do not flatten.


## Admission rule for future packs

A proposed new pack SHOULD first prove that no current canonical pack, projection, profile, or lawful composition already owns its semantic authority. Creating a new directory is not evidence of a new capability class.
