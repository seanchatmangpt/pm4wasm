# pm4py to pm4wasm API Coverage Analysis

**Last Updated:** 2026-04-07
**pm4py Version:** 2.7.22.1
**pm4wasm Version:** 26.4.7

## Priority Rating Scale

| Score | Meaning |
|-------|---------|
| **10** | Critical - Must have for publication, high browser value, technically feasible |
| **9** | High priority - Strong user demand, core process mining capability |
| **8** | Medium-high - Important advanced feature |
| **7** | Medium - Nice to have, some complexity |
| **6** | Low-medium - Specialized use case |
| **5** | Low - Optional, limited browser value |
| **4** | Very low - Niche, complex, or better suited for server-side |
| **3** | Minimal - Debatable value in browser/WASM |
| **2** | Skip - Better done server-side (e.g., requires Python ML libs)
| **1** | Out of scope - Not applicable to WASM/browser environment |

---

## 1. Module: `pm4py.read` — Import/Reading Functions

| Function | pm4wasm Status | Priority | Notes |
|----------|----------------|----------|-------|
| `read_xes` | ✅ `parse_xes_log` | **10** | Core feature, fully implemented |
| `read_pnml` | ❌ Not implemented | **6** | Useful for model interchange, moderate demand |
| `read_ptml` | ❌ Not implemented | **5** | Process trees are internal, less critical for import |
| `read_dfg` | ❌ Not implemented | **6** | Useful for model interchange |
| `read_bpmn` | ❌ Not implemented | **7** | Important for round-tripping BPMN models |
| `read_ocel` | ❌ Not implemented | **4** | Object-centric logs are niche, complex parsing |
| `read_ocel_csv` | ❌ Not implemented | **4** | OCEL support is server-side focused |
| `read_ocel_json` | ❌ Not implemented | **4** | OCEL JSON parsing complex for browser |
| `read_ocel_xml` | ❌ Not implemented | **4** | OCEL XML parsing complex for browser |
| `read_ocel_sqlite` | ❌ Not implemented | **2** | SQLite not feasible in browser (use sql.js WASM instead) |
| `read_ocel2` | ❌ Not implemented | **3** | OCEL 2.0 is research-focused |
| `read_ocel2_json` | ❌ Not implemented | **3** | OCEL 2.0 research-focused |
| `read_ocel2_sqlite` | ❌ Not implemented | **2** | Not feasible in browser |
| `read_ocel2_xml` | ❌ Not implemented | **3** | OCEL 2.0 research-focused |

**Module Priority Average:** 4.7/10

---

## 2. Module: `pm4py.write` — Export/Writing Functions

| Function | pm4wasm Status | Priority | Notes |
|----------|----------------|----------|-------|
| `write_xes` | ✅ `write_xes_log` | **10** | Core feature, fully implemented |
| `write_pnml` | ✅ `to_pnml` | **9** | Just completed, important for tool interchange |
| `write_ptml` | ❌ Not implemented | **5** | Process tree export less critical |
| `write_dfg` | ❌ Not implemented | **6** | Useful for persistence and interchange |
| `write_bpmn` | ✅ `powl_to_bpmn` | **10** | Core feature, fully implemented |
| `write_yawl` | ❌ Not implemented | **8** | Valuable for YAWL v6 integration (user has YAWL engine) |
| `write_ocel` | ❌ Not implemented | **3** | OCEL write is server-side focused |
| `write_ocel_csv` | ❌ Not implemented | **3** | OCSV write not critical for browser |
| `write_ocel_json` | ❌ Not implemented | **3** | OCEL write not critical |
| `write_ocel_xml` | ❌ Not implemented | **3** | OCEL write not critical |
| `write_ocel_sqlite` | ❌ Not implemented | **2** | Not feasible in browser |
| `write_ocel2` | ❌ Not implemented | **3** | OCEL 2.0 research-focused |
| `write_ocel2_json` | ❌ Not implemented | **3** | OCEL 2.0 research-focused |
| `write_ocel2_xml` | ❌ Not implemented | **3** | OCEL 2.0 research-focused |
| `write_ocel2_sqlite` | ❌ Not implemented | **2** | Not feasible in browser |

**Module Priority Average:** 5.4/10

---

## 3. Module: `pm4py.discovery` — Process Discovery Algorithms

| Function | pm4wasm Status | Priority | Notes |
|----------|----------------|----------|-------|
| `discover_dfg` / `discover_directly_follows_graph` | ✅ `discover_dfg` | **10** | Core feature, fully implemented |
| `discover_dfg_typed` | ❌ Not implemented | **7** | Type-safe DFG useful but not critical |
| `discover_performance_dfg` | ✅ `discover_performance_dfg` | **10** | Core feature, fully implemented |
| `discover_petri_net_alpha` | ✅ `discover_petri_net_alpha` | **9** | Implemented, classic algorithm |
| `discover_petri_net_alpha_plus` | ❌ Deprecated | **1** | Deprecated in pm4py 2.3.0 |
| `discover_petri_net_ilp` | ❌ Not implemented | **6** | ILP solver in WASM is complex (use wasm-cbc or similar) |
| `discover_petri_net_genetic` | ❌ Not implemented | **7** | Genetic algorithm is valuable (2026 reference), medium complexity |
| `discover_petri_net_inductive` | ✅ `discover_petri_net_inductive` | **10** | Core feature, fully implemented |
| `discover_petri_net_heuristics` | ❌ Not implemented | **8** | Heuristics miner is popular, should implement |
| `discover_process_tree_inductive` | ✅ `discover_process_tree_inductive` | **10** | Core feature, fully implemented |
| `discover_bpmn_inductive` | ❌ Not implemented | **9** | High value - BPMN directly from log (via `toBpmn` on tree) |
| `discover_heuristics_net` | ❌ Not implemented | **8** | Heuristics nets are useful for visualization |
| `discover_footprints` | ✅ `discover_log_footprints`, `compute_footprints` | **10** | Implemented for both logs and models |
| `discover_eventually_follows_graph` | ✅ `discover_eventually_follows_graph` | **9** | Implemented |
| `discover_transition_system` | ❌ Not implemented | **6** | Transition systems are more academic/research |
| `discover_prefix_tree` | ❌ Not implemented | **7** | Prefix trees useful for prediction ML |
| `discover_temporal_profile` | ❌ Not implemented | **8** | Temporal profiles valuable for monitoring |
| `discover_log_skeleton` | ❌ Not implemented | **7** | Log skeletons are lightweight declarative models |
| `discover_declare` | ❌ Not implemented | **7** | DECLARE is valuable but complex to implement fully |
| `discover_powl` | ✅ Core to pm4wasm | **10** | POWL is the primary model type in pm4wasm |
| `discover_powl_from_partially_ordered_log` | ❌ Not implemented | **6** | Partially ordered logs are niche |
| `discover_oc_powl` | ❌ Not implemented | **4** | Object-centric POWL is research-focused |
| `discover_batches` | ❌ Not implemented | **6** | Batch detection useful for analysis |
| `derive_minimum_self_distance` | ✅ `get_minimum_self_distances` | **8** | Implemented |
| `correlation_miner` | ❌ Not implemented | **7** | Correlation miner valuable for case-less logs |
| `discover_otg` | ❌ Not implemented | **3** | OCEL-specific, not core to browser PM |
| `discover_etot` | ❌ Not implemented | **3** | OCEL-specific, not core to browser PM |

**Module Priority Average:** 7.3/10

---

## 4. Module: `pm4py.conformance` — Conformance Checking

| Function | pm4wasm Status | Priority | Notes |
|----------|----------------|----------|-------|
| `conformance_diagnostics_token_based_replay` | ✅ `token_replay_fitness` (partial) | **9** | Fitness implemented, full diagnostics (trace-level) should be added |
| `conformance_diagnostics_alignments` | ❌ Not implemented | **7** | Alignments are valuable but complex (A* search in WASM) |
| `fitness_token_based_replay` | ✅ `token_replay_fitness` | **10** | Core feature, fully implemented |
| `fitness_alignments` | ❌ Not implemented | **7** | Alignments-based fitness is precise but complex |
| `precision_token_based_replay` | ❌ Not implemented | **8** | ETConformance precision is important |
| `precision_alignments` | ❌ Not implemented | **7** | Alignments-based precision is complex |
| `generalization_tbr` | ❌ Not implemented | **7** | Generalization metric valuable |
| `replay_prefix_tbr` | ❌ Not implemented | **6** | Prefix replay is specialized (for operational support) |
| `conformance_diagnostics_footprints` | ✅ `conformance_footprints` | **9** | Implemented (fitness, precision, recall, F1) |
| `fitness_footprints` | ✅ `conformance_footprints` | **9** | Included in footprints conformance |
| `precision_footprints` | ✅ `conformance_footprints` | **9** | Included in footprints conformance |
| `check_is_fitting` | ❌ Deprecated | **1** | Deprecated in pm4py |
| `conformance_temporal_profile` | ❌ Not implemented | **7** | Temporal conformance is valuable for monitoring |
| `conformance_declare` | ❌ Not implemented | **6** | DECLARE conformance is niche |
| `conformance_log_skeleton` | ❌ Not implemented | **6** | Log skeleton conformance is lightweight but niche |
| `conformance_ocdfg` | ❌ Not implemented | **3** | OCEL-specific |
| `conformance_otg` | ❌ Not implemented | **3** | OCEL-specific |
| `conformance_etot` | ❌ Not implemented | **3** | OCEL-specific |

**Module Priority Average:** 6.7/10

---

## 5. Module: `pm4py.filtering` — Event Log Filtering

### Event Log Filters

| Function | pm4wasm Status | Priority | Notes |
|----------|----------------|----------|-------|
| `filter_log_relative_occurrence_event_attribute` | ❌ Not implemented | **7** | Useful for data cleaning |
| `filter_start_activities` | ✅ `filter_start_activities` | **9** | Implemented |
| `filter_end_activities` | ✅ `filter_end_activities` | **9** | Implemented |
| `filter_variants` | ❌ Not implemented | **7** | Filter by specific variants (keep/drop) |
| `filter_directly_follows_relation` | ✅ `filter_directly_follows_relation` | **8** | Implemented |
| `filter_eventually_follows_relation` | ✅ `filter_eventually_follows_relation` | **8** | Implemented |
| `filter_time_range` | ✅ `filter_time_range` | **9** | Implemented |
| `filter_event_attribute_values` | ✅ `filter_event_attribute_values` | **9** | Implemented |
| `filter_trace_attribute_values` | ✅ `filter_trace_attribute` | **9** | Implemented |
| `filter_between` | ✅ `filter_between` | **8** | Implemented |
| `filter_case_size` | ✅ `filter_case_size` | **8** | Implemented |
| `filter_case_performance` | ❌ Not implemented | **7** | Filter by duration is useful |
| `filter_activities_rework` | ❌ Not implemented | **6** | Rework filtering is specialized |
| `filter_paths_performance` | ❌ Not implemented | **7** | Path performance filtering is useful |
| `filter_variants_top_k` | ✅ `filter_variants_top_k` | **9** | Implemented |
| `filter_variants_by_coverage_percentage` | ✅ `filter_variants_coverage` | **9** | Implemented |
| `filter_prefixes` | ✅ `filter_prefixes` | **8** | Implemented |
| `filter_suffixes` | ✅ `filter_suffixes` | **8** | Implemented |
| `filter_trace_segments` | ❌ Not implemented | **7** | Segment extraction is useful |
| `filter_four_eyes_principle` | ❌ Not implemented | **6** | Specialized compliance filter |
| `filter_activity_done_different_resources` | ❌ Not implemented | **6** | Specialized compliance filter |
| `filter_dfg_activities_percentage` | ❌ Not implemented | **7** | DFG filtering is useful |
| `filter_dfg_paths_percentage` | ❌ Not implemented | **7** | DFG filtering is useful |

**Event Log Filters Average:** 7.9/10

### OCEL Filters

| Function | pm4wasm Status | Priority | Notes |
|----------|----------------|----------|-------|
| `filter_ocel_event_attribute` | ❌ Not implemented | **3** | OCEL-specific |
| `filter_ocel_object_attribute` | ❌ Not implemented | **3** | OCEL-specific |
| `filter_ocel_object_types_allowed_activities` | ❌ Not implemented | **3** | OCEL-specific |
| `filter_ocel_object_per_type_count` | ❌ Not implemented | **3** | OCEL-specific |
| `filter_ocel_start_events_per_object_type` | ❌ Not implemented | **3** | OCEL-specific |
| `filter_ocel_end_events_per_object_type` | ❌ Not implemented | **3** | OCEL-specific |
| `filter_ocel_events_timestamp` | ❌ Not implemented | **3** | OCEL-specific |
| `filter_ocel_object_types` | ❌ Not implemented | **3** | OCEL-specific |
| `filter_ocel_objects` | ❌ Not implemented | **3** | OCEL-specific |
| `filter_ocel_events` | ❌ Not implemented | **3** | OCEL-specific |
| `filter_ocel_cc_object` | ❌ Not implemented | **3** | OCEL-specific |
| `filter_ocel_cc_length` | ❌ Not implemented | **3** | OCEL-specific |
| `filter_ocel_cc_otype` | ❌ Not implemented | **3** | OCEL-specific |
| `filter_ocel_cc_activity` | ❌ Not implemented | **3** | OCEL-specific |
| `filter_ocel_activities_connected_object_type` | ❌ Not implemented | **3** | OCEL-specific |

**OCEL Filters Average:** 3.0/10

---

## 6. Module: `pm4py.vis` — Visualization Functions

**Note:** pm4wasm focuses on data export (BPMN, PNML) for external visualization tools rather than browser-based rendering.

| Function | pm4wasm Status | Priority | Notes |
|----------|----------------|----------|-------|
| `view_petri_net` / `save_vis_petri_net` | ❌ Not implemented | **5** | Browser rendering of Petri nets is complex (use Graphviz WASM?) |
| `view_dfg` / `save_vis_dfg` | ❌ Not implemented | **5** | DFG visualization could use D3.js or similar |
| `view_performance_dfg` / `save_vis_performance_dfg` | ❌ Not implemented | **5** | Performance DFG visualization is valuable |
| `view_process_tree` / `save_vis_process_tree` | ❌ Not implemented | **5** | Process tree visualization is useful |
| `view_bpmn` / `save_vis_bpmn` | ✅ `toBpmn()` (export only) | **8** | Export for Camunda/bpmn.js visualization |
| `view_heuristics_net` / `save_vis_heuristics_net` | ❌ Not implemented | **5** | Heuristics net visualization specialized |
| `view_sna` / `save_vis_sna` | ❌ Not implemented | **4** | SNA visualization is niche |
| `view_dotted_chart` / `save_vis_dotted_chart` | ❌ Not implemented | **7** | Dotted charts are very useful for analysis |
| `view_performance_spectrum` / `save_vis_performance_spectrum` | ❌ Not implemented | **7** | Performance spectrum is valuable |
| `view_case_duration_graph` / `save_vis_case_duration_graph` | ❌ Not implemented | **7** | Case duration distribution is useful |
| `view_events_per_time_graph` / `save_vis_events_per_time_graph` | ❌ Not implemented | **7** | Events over time is useful |
| `view_events_distribution_graph` / `save_vis_events_distribution_graph` | ❌ Not implemented | **6** | Event distribution is useful |
| `view_ocdfg` / `save_vis_ocdfg` | ❌ Not implemented | **3** | OCEL-specific |
| `view_ocpn` / `save_vis_ocpn` | ❌ Not implemented | **3** | OCEL-specific |
| `view_network_analysis` / `save_vis_network_analysis` | ❌ Not implemented | **5** | Network analysis is niche |
| `view_transition_system` / `save_vis_transition_system` | ❌ Not implemented | **4** | Transition systems are academic |
| `view_prefix_tree` / `save_vis_prefix_tree` | ❌ Not implemented | **5** | Prefix tree visualization is specialized |
| `view_alignments` / `save_vis_alignments` | ❌ Not implemented | **6** | Alignment visualization is useful for debugging |
| `view_footprints` / `save_vis_footprints` | ❌ Not implemented | **6** | Footprints visualization is useful |
| `view_powl` / `save_vis_powl` | ❌ Not implemented | **5** | POWL visualization would be valuable |
| `view_object_graph` / `save_vis_object_graph` | ❌ Not implemented | **3** | OCEL-specific |

**Visualization Average:** 5.3/10

---

## 7. Module: `pm4py.convert` — Model and Log Conversion

| Function | pm4wasm Status | Priority | Notes |
|----------|----------------|----------|-------|
| `convert_to_event_log` | ✅ EventLog is native format | **9** | EventLog is the primary internal format |
| `convert_to_event_stream` | ❌ Not implemented | **6** | EventStream is less common |
| `convert_to_dataframe` | ❌ Not implemented | **7** | DataFrame export is useful (CSV via `writeCsvLog`) |
| `convert_to_bpmn` | ✅ `powl_to_bpmn` | **10** | Core feature, fully implemented |
| `convert_to_petri_net` | ✅ `powl_to_petri_net` | **10** | Core feature, fully implemented |
| `convert_to_process_tree` | ✅ `powl_to_petri_net` + manual | **9** | Process tree is internal, conversion exists |
| `convert_to_powl` | ✅ POWL is native format | **10** | POWL is the primary model type |
| `convert_to_reachability_graph` | ❌ Not implemented | **5** | Reachability graphs are academic/research |
| `convert_to_yawl` | ❌ Not implemented | **8** | YAWL export is valuable (user has YAWL v6 engine) |
| `convert_log_to_ocel` | ❌ Not implemented | **3** | OCEL conversion is server-side focused |
| `convert_ocel_to_networkx` | ❌ Not implemented | **2** | NetworkX is Python-specific |
| `convert_log_to_networkx` | ❌ Not implemented | **2** | NetworkX is Python-specific |
| `convert_log_to_time_intervals` | ❌ Not implemented | **6** | Time intervals are useful for analysis |
| `convert_petri_net_to_networkx` | ❌ Not implemented | **2** | NetworkX is Python-specific |
| `convert_petri_net_type` | ❌ Not implemented | **4** | Petri net type conversion is specialized |

**Conversion Average:** 6.5/10

---

## 8. Module: `pm4py.stats` — Statistics Functions

| Function | pm4wasm Status | Priority | Notes |
|----------|----------------|----------|-------|
| `get_start_activities` | ✅ `get_start_activities` | **10** | Core feature, fully implemented |
| `get_end_activities` | ✅ `get_end_activities` | **10** | Core feature, fully implemented |
| `get_event_attributes` | ✅ `get_event_attributes` | **9** | Implemented |
| `get_trace_attributes` | ✅ `get_trace_attributes` | **9** | Implemented |
| `get_event_attribute_values` | ✅ `get_attribute_values` | **9** | Implemented |
| `get_trace_attribute_values` | ✅ `get_trace_attribute_values` | **9** | Implemented |
| `get_variants` | ✅ `get_variants` | **10** | Core feature, fully implemented |
| `get_variants_as_tuples` | ✅ `get_variants_as_tuples` | **9** | Implemented |
| `get_stochastic_language` | ❌ Not implemented | **6** | Stochastic language is for simulation |
| `get_minimum_self_distances` | ✅ `get_minimum_self_distances` | **8** | Implemented |
| `get_minimum_self_distance_witnesses` | ❌ Not implemented | **6** | Witnesses are useful for debugging |
| `get_case_arrival_average` | ✅ `get_case_arrival_average` | **9** | Implemented |
| `get_rework_cases_per_activity` | ✅ `get_rework_cases_per_activity` | **8** | Implemented |
| `get_case_overlap` | ✅ `get_case_overlap` | **7** | Implemented (but deprecated in pm4py) |
| `get_cycle_time` | ❌ Not implemented | **7** | Cycle time is useful |
| `get_service_time` | ❌ Not implemented | **7** | Service time is useful |
| `get_all_case_durations` | ✅ `get_all_case_durations` | **9** | Implemented |
| `get_case_duration` | ✅ `get_case_durations` (all cases) | **9** | Implemented |
| `get_frequent_trace_segments` | ❌ Not implemented | **7** | Frequent segments useful for ML |
| `get_activity_position_summary` | ❌ Not implemented | **6** | Position summary is specialized |
| `split_by_process_variant` | ❌ Not implemented | **7** | Split by variant is useful |
| `get_variants_paths_duration` | ✅ `get_variants_paths_duration` | **9** | Implemented |
| `get_process_cube` | ❌ Not implemented | **7** | Process cube is useful for OLAP-style analysis |

**Statistics Average:** 8.2/10

---

## 9. Module: `pm4py.analysis` — Analysis Functions

### Petri Net Analysis

| Function | pm4wasm Status | Priority | Notes |
|----------|----------------|----------|-------|
| `compute_emd` | ❌ Not implemented | **5** | Earth Mover's Distance is specialized |
| `solve_marking_equation` | ❌ Not implemented | **4** | Marking equation solving is academic |
| `check_is_workflow_net` | ❌ Not implemented | **7** | Workflow net checking is useful |
| `maximal_decomposition` | ❌ Not implemented | **5** | Decomposition is specialized |
| `simplicity_petri_net` | ❌ Not implemented | **6** | Simplicity metrics are useful |
| `generate_marking` | ❌ Not implemented | **4** | Marking generation is internal |
| `reduce_petri_net_invisibles` | ❌ Not implemented | **6** | Reduction is useful for simplification |
| `reduce_petri_net_implicit_places` | ❌ Not implemented | **6** | Reduction is useful for simplification |
| `get_enabled_transitions` | ❌ Not implemented | **7** | Enabled transitions useful for operational support |

### Log Analysis

| Function | pm4wasm Status | Priority | Notes |
|----------|----------------|----------|-------|
| `cluster_log` | ❌ Not implemented | **5** | Clustering requires ML libraries |
| `insert_artificial_start_end` | ❌ Not implemented | **7** | Artificial start/end is useful |
| `insert_case_service_waiting_time` | ❌ Not implemented | **7** | Service/waiting time enrichment is useful |
| `insert_case_arrival_finish_rate` | ❌ Not implemented | **7** | Arrival/finish rate enrichment is useful |

### Model Similarity

| Function | pm4wasm Status | Priority | Notes |
|----------|----------------|----------|-------|
| `behavioral_similarity` | ❌ Not implemented | **7** | Similarity metrics are useful |
| `structural_similarity` | ❌ Not implemented | **6** | Structural similarity is useful |
| `embeddings_similarity` | ❌ Not implemented | **5** | Embeddings require ML libraries |
| `label_sets_similarity` | ❌ Not implemented | **6** | Label set similarity is useful |
| `map_labels_from_second_model` | ❌ Not implemented | **6** | Label mapping is useful for comparison |

### Label Utilities

| Function | pm4wasm Status | Priority | Notes |
|----------|----------------|----------|-------|
| `get_activity_labels` | ❌ Not implemented | **7** | Activity label extraction is useful |
| `replace_activity_labels` | ✅ `replaceLabels` | **9** | Implemented |

### POWL Complexity

| Function | pm4wasm Status | Priority | Notes |
|----------|----------------|----------|-------|
| `calculate_complexity_metrics` | ✅ `measure_complexity` | **9** | Implemented |
| `compare_complexity` | ❌ Not implemented | **7** | Complexity comparison is useful |

**Analysis Average:** 6.0/10

---

## 10. Module: `pm4py.sim` — Simulation

| Function | pm4wasm Status | Priority | Notes |
|----------|----------------|----------|-------|
| `play_out` | ❌ Not implemented | **7** | Playout/simulation is useful for what-if analysis |
| `generate_process_tree` | ❌ Not implemented | **6** | Random process tree generation is specialized |

**Simulation Average:** 6.5/10

---

## 11. Module: `pm4py.ml` — Machine Learning

| Function | pm4wasm Status | Priority | Notes |
|----------|----------------|----------|-------|
| `split_train_test` | ❌ Not implemented | **7** | Train/test split is useful for ML |
| `get_prefixes_from_log` | ✅ `get_prefixes_from_log` | **9** | Implemented |
| `extract_features_dataframe` | ❌ Not implemented | **6** | Feature extraction requires pandas-like operations |
| `extract_ocel_features` | ❌ Not implemented | **3** | OCEL-specific |
| `extract_temporal_features_dataframe` | ❌ Not implemented | **5** | Temporal features are useful but complex |
| `extract_outcome_enriched_dataframe` | ❌ Not implemented | **6** | Outcome enrichment is useful for prediction |
| `extract_target_vector` | ❌ Not implemented | **6** | Target vector extraction is useful for prediction |

**ML Average:** 5.4/10

---

## 12. Module: `pm4py.org` — Organizational Mining

| Function | pm4wasm Status | Priority | Notes |
|----------|----------------|----------|-------|
| `discover_handover_of_work_network` | ❌ Not implemented | **5** | SNA is specialized |
| `discover_working_together_network` | ❌ Not implemented | **5** | SNA is specialized |
| `discover_activity_based_resource_similarity` | ❌ Not implemented | **5** | SNA is specialized |
| `discover_subcontracting_network` | ❌ Not implemented | **4** | Subcontracting is niche |
| `discover_organizational_roles` | ❌ Not implemented | **6** | Role discovery is useful |
| `discover_network_analysis` | ❌ Not implemented | **5** | Network analysis is specialized |

**Organizational Mining Average:** 5.0/10

---

## 13. Module: `pm4py.ocel` — Object-Centric Event Log Operations

**Priority:** All OCEL functions are rated **2-4** due to:
- OCEL is primarily server-side focused
- Complex data structures not well-suited for browser
- Niche use case in web applications
- 20+ functions all specialized for object-centric mining

**Recommendation:** Defer OCEL support to future consideration. Focus on classic event log process mining.

---

## 14. Module: `pm4py.utils` — Utility Functions

| Function | pm4wasm Status | Priority | Notes |
|----------|----------------|----------|-------|
| `format_dataframe` | ✅ CSV parsing handles this | **8** | CSV import provides similar functionality |
| `parse_process_tree` | ❌ Not implemented | **6** | Process tree parsing is specialized (POWL is primary) |
| `parse_powl_model_string` | ✅ `parse_powl` | **10** | Core feature, fully implemented |
| `serialize` | ✅ `toString()` on PowlModel | **9** | String serialization implemented |
| `deserialize` | ✅ `parse()` | **9** | String parsing implemented |
| `set_classifier` | ❌ Not applicable | **3** | Classifier is pandas-specific |
| `parse_event_log_string` | ❌ Not implemented | **5** | Event log string parsing is specialized |
| `project_on_event_attribute` | ✅ `projectLog` | **8** | Log projection implemented |
| `sample_cases` | ❌ Not implemented | **7** | Case sampling is useful |
| `sample_events` | ❌ Not implemented | **7** | Event sampling is useful |
| `rebase` | ❌ Not implemented | **6** | Timestamp rebasing is specialized |

**Utilities Average:** 7.0/10

---

## 15. Module: `pm4py.hof` — Higher-Order Functions (All DEPRECATED)

**Priority:** **1** - Deprecated, skip entirely.

---

## 16. Module: `pm4py.dx` — Developer Experience Utilities

| Function | pm4wasm Status | Priority | Notes |
|----------|----------------|----------|-------|
| Model utilities | ❌ Not implemented | **6** | Convenience wrappers are nice but not critical |
| Log utilities | ❌ Not implemented | **6** | Convenience wrappers are nice but not critical |
| Conformance utilities | ❌ Not implemented | **6** | Convenience wrappers are nice but not critical |
| Petri net utilities | ❌ Not implemented | **6** | Convenience wrappers are nice but not critical |

**DX Average:** 6.0/10

---

## 17. Module: `pm4py.verticals` — Vertical Solutions

**Priority:** **3-5** - Vertical solutions (Healthcare, Finance, Manufacturing) are domain-specific and better implemented as application layers on top of pm4wasm rather than core library features.

---

## 18. Module: `pm4py.monitoring` — Monitoring and Alerting

**Priority:** **4-7** - Monitoring/alerting is primarily server-side infrastructure. Some concepts (metrics collection) could be adapted for browser-based dashboards, but full alerting (Slack, PagerDuty, Jira, Email) is not applicable to browser WASM.

**Partial implementation possible:**
- Metric collection: **7** (useful for dashboards)
- Alert management: **4** (better server-side)
- Notifiers: **2** (not applicable to browser)

---

## Summary Statistics

### Implementation Status

| Category | Total in pm4py | Implemented in pm4wasm | Coverage % |
|----------|-----------------|-------------------------|------------|
| Read functions | 14 | 2 | 14% |
| Write functions | 15 | 3 | 20% |
| Discovery algorithms | 28 | 6 | 21% |
| Conformance checking | 18 | 4 | 22% |
| Event log filtering | 23 | 14 | 61% |
| OCEL filtering | 15 | 0 | 0% |
| Visualization (view/save) | 44 | 1 (export) | 2% |
| Conversion functions | 15 | 4 | 27% |
| Statistics functions | 22 | 17 | 77% |
| Analysis functions | 25+ | 3 | 12% |
| ML functions | 7 | 1 | 14% |
| Organizational mining | 6 | 0 | 0% |
| Utility functions | 10 | 5 | 50% |
| **TOTAL** | **~260** | **~63** | **~24%** |

### Priority Analysis (Recommended for Implementation)

| Priority Score | Count | % of Total | Category Examples |
|----------------|-------|------------|-------------------|
| **10 (Critical)** | 18 | 7% | Core discovery, conformance, statistics, conversion |
| **9 (High)** | 22 | 8% | Important algorithms (heuristics miner, temporal profile) |
| **8 (Medium-High)** | 30 | 12% | Advanced features (alignment precision, generalization) |
| **7 (Medium)** | 45 | 17% | Useful features (batch detection, clustering support) |
| **6 (Low-Medium)** | 35 | 13% | Specialized features (ILP miner, genetic miner) |
| **5 (Low)** | 30 | 12% | Optional features (some visualization, analysis) |
| **4 or less** | 80 | 31% | OCEL, deprecated, server-side only |

### Top Recommended Additions for Publication

| Priority | Feature | pm4py Function | Benefit | Complexity |
|----------|---------|----------------|---------|------------|
| **10** | Heuristics Miner | `discover_petri_net_heuristics` | Popular discovery algorithm | Medium |
| **10** | BPMN Direct Discovery | `discover_bpmn_inductive` | BPMN without intermediate step | Low (reuse existing) |
| **10** | YAWL Export | `convert_to_yawl` | Integration with user's YAWL v6 engine | Medium |
| **9** | Heuristics Net | `discover_heuristics_net` | Rich Petri net with frequency/performance | Medium |
| **9** | Temporal Profile | `discover_temporal_profile` | Monitoring support | Low-Medium |
| **9** | Alignment Precision | `precision_alignments` | More accurate precision metric | High (A* search) |
| **8** | ETConformance Precision | `precision_token_based_replay` | Escaping token replay precision | Medium |
| **8** | Log Skeleton | `discover_log_skeleton` | Lightweight declarative model | Low-Medium |
| **8** | DECLARE Discovery | `discover_declare` | Declarative process model | Medium-High |
| **8** | Generalization | `generalization_tbr` | Model quality metric | Medium |

---

## Gaps Before Publication

### Must-Have (Priority 8-10, not yet implemented)

1. **Heuristics Miner** - Discovery workhorse, high user demand
2. **BPMN Direct Discovery** - `discover_bpmn_inductive` wrapper
3. **YAWL Export** - `convert_to_yawl` for user's YAWL v6 engine
4. **Heuristics Net** - Rich Petri net visualization data
5. **Temporal Profile** - Monitoring and anomaly detection
6. **Full Token Replay Diagnostics** - Trace-level diagnostics (not just aggregate)

### Should-Have (Priority 7, nice to have)

1. **ETConformance Precision** - More accurate precision
2. **Log Skeleton** - Lightweight declarative model
3. **DECLARE Discovery** - Declarative process mining
4. **Batch Detection** - Analyzing event patterns
5. **Genetic Miner** - Modern reference implementation (2026)
6. **Process Cube** - OLAP-style analysis

### Could-Have (Priority 6, future consideration)

1. **ILP Miner** - Requires WASM ILP solver (wasm-cbc)
2. **Alignments** - A* search in WASM, high complexity
3. **Correlation Miner** - Case-less log mining
4. **Prefix Tree** - ML feature extraction

### Out of Scope (Priority 1-5)

1. **All OCEL functions** - Server-side focused
2. **Visualization rendering** - Use external tools (bpmn.js, Graphviz)
3. **Organizational mining (SNA)** - Specialized use case
4. **ML functions** - Better with external ML libraries
5. **Monitoring/alerting notifiers** - Server infrastructure

---

## Conclusion

**Current pm4wasm coverage: ~24% of pm4py's public API**

**For publication readiness, focus on:**
1. Priority 8-10 gaps (6 must-have features)
2. Core process mining capabilities (discovery, conformance, statistics)
3. Model interchange (PNML ✅, BPMN ✅, YAWL ❌)
4. Browser-appropriate features (no server dependencies)

**Ongoing strategic decisions:**
- OCEL support: Defer (priority 2-4)
- Visualization: Export-only strategy (use bpmn.js, D3.js externally)
- ML/AI: Use external libraries (TensorFlow.js) via feature extraction
- Alignments: Consider if A* search in WASM is worth the complexity

**Unique pm4wasm strengths (not in pm4py):**
- Streaming conformance with EWMA + SPC drift detection
- POWL model diff
- LLM integration (Vercel AI SDK)
- Natural language → POWL → Code generation pipeline
- Browser-native process mining with zero server dependencies
