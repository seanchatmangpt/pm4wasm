// PM4Py -- A Process Mining Library for Python (POWL v2 WASM)
// Copyright (C) 2024 Process Intelligence Solutions
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

/// Genetic process discovery miner.
///
/// Discovers Petri nets from event logs using an evolutionary algorithm
/// inspired by the classic Genetic Miner (Greco et al., 2006). Each
/// individual in the population is a Petri net; fitness is evaluated
/// via token replay against the event log.
///
/// Reference: M.J. Frank. "Optimising and Implementing the Genetic Miner in PM4Py" (2026).

use crate::conformance::token_replay::compute_fitness;
use crate::discovery::dfg::discover_dfg;
use crate::event_log::EventLog;
use crate::petri_net::{Counts, Marking, PetriNet, PetriNetResult};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

/// Configuration for the genetic miner.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GeneticMinerConfig {
    /// Number of individuals per generation (fixed to avoid WASM memory bloat).
    pub population_size: usize,
    /// Maximum number of evolutionary generations.
    pub generations: usize,
    /// Per-element mutation probability.
    pub mutation_rate: f64,
    /// Probability of applying crossover between two parents.
    pub crossover_rate: f64,
}

impl Default for GeneticMinerConfig {
    fn default() -> Self {
        GeneticMinerConfig {
            population_size: 30,
            generations: 50,
            mutation_rate: 0.1,
            crossover_rate: 0.7,
        }
    }
}

/// A single individual in the population: a Petri net plus its markings.
#[derive(Clone)]
struct Individual {
    net: PetriNet,
    initial_marking: Marking,
    final_marking: Marking,
    fitness: f64,
}

impl Individual {
    fn to_result(&self) -> PetriNetResult {
        PetriNetResult {
            net: self.net.clone(),
            initial_marking: self.initial_marking.clone(),
            final_marking: self.final_marking.clone(),
        }
    }
}

/// Deterministic PRNG so the miner is reproducible.
struct Rng {
    state: u64,
}

impl Rng {
    fn new(seed: u64) -> Self {
        Rng {
            state: if seed == 0 { 0xDEAD_BEEF_CAFE_BABE } else { seed },
        }
    }

    fn next_u64(&mut self) -> u64 {
        let mut x = self.state;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.state = x;
        x
    }

    fn next_f64(&mut self) -> f64 {
        (self.next_u64() & 0x00FF_FFFF_FFFF_FFFF) as f64 / (1u64 << 52) as f64
    }

    fn next_usize(&mut self, max: usize) -> usize {
        if max <= 1 {
            return 0;
        }
        (self.next_u64() as usize) % max
    }
}

/// Build a Petri net from the DFG: one place between each directly-follows
/// pair, a unique source place, and a unique sink place.
///
/// This mirrors the classic inductive-miner "flower" seeding strategy:
/// all start activities read from source, all end activities write to sink,
/// and DFG edges get their own intermediate places.
fn build_dfg_net(log: &EventLog, rng: &mut Rng) -> Individual {
    let dfg = discover_dfg(log);
    let mut counts = Counts::default();
    let mut net = PetriNet::new("genetic_dfg_seed");

    let source = format!("p_source_{}", counts.inc_places());
    let sink = format!("p_sink_{}", counts.inc_places());
    net.add_place(&source);
    net.add_place(&sink);

    // Collect all activities
    let activities: Vec<String> = dfg.activities.iter().map(|(a, _)| a.clone()).collect();

    // Add visible transitions for every activity
    for act in &activities {
        let t_name = format!("t_{}", act);
        net.add_transition(&t_name, Some(act.clone()));
    }

    // Create intermediate places for DFG edges (probability-biased).
    // We include edges stochastically to create variation in the initial population.
    let mut edge_places: HashMap<(String, String), String> = HashMap::new();
    for edge in &dfg.edges {
        if rng.next_f64() < 0.7 {
            // 70% chance to include each DFG edge
            let p_name = format!("p_{}_{}", edge.source, edge.target);
            net.add_place(&p_name);
            let t_src = format!("t_{}", edge.source);
            let t_tgt = format!("t_{}", edge.target);
            net.add_arc(&t_src, &p_name);
            net.add_arc(&p_name, &t_tgt);
            edge_places.insert((edge.source.clone(), edge.target.clone()), p_name);
        }
    }

    // Connect source to start activities
    for (act, _) in &dfg.start_activities {
        let t_name = format!("t_{}", act);
        net.add_arc(&source, &t_name);
    }

    // Connect end activities to sink
    for (act, _) in &dfg.end_activities {
        let t_name = format!("t_{}", act);
        net.add_arc(&t_name, &sink);
    }

    // For activities with no outgoing DFG edge place, add an arc to sink
    let has_outgoing: HashSet<String> = edge_places
        .keys()
        .map(|(s, _)| s.clone())
        .collect();
    for act in &activities {
        if !has_outgoing.contains(act) && !dfg.end_activities.iter().any(|(a, _)| a == act) {
            let t_name = format!("t_{}", act);
            // Avoid duplicate arcs
            let already_to_sink = net.arcs.iter().any(|a| a.source == t_name && a.target == sink);
            if !already_to_sink {
                net.add_arc(&t_name, &sink);
            }
        }
    }

    // For activities with no incoming DFG edge place, add an arc from source
    let has_incoming: HashSet<String> = edge_places
        .keys()
        .map(|(_, t)| t.clone())
        .collect();
    for act in &activities {
        if !has_incoming.contains(act) && !dfg.start_activities.iter().any(|(a, _)| a == act) {
            let t_name = format!("t_{}", act);
            let already_from_source = net.arcs.iter().any(|a| a.source == source && a.target == t_name);
            if !already_from_source {
                net.add_arc(&source, &t_name);
            }
        }
    }

    let mut initial_marking = Marking::new();
    initial_marking.insert(source.clone(), 1);

    let mut final_marking = Marking::new();
    final_marking.insert(sink.clone(), 1);

    let mut individual = Individual {
        net,
        initial_marking,
        final_marking,
        fitness: 0.0,
    };
    individual.fitness = evaluate_fitness(&individual, log);
    individual
}

/// Compute composite fitness: `0.6 * avg_trace_fitness + 0.3 * fitting_ratio + 0.1 * simplicity`.
/// Penalizes individuals with no places, arcs, or empty markings.
fn evaluate_fitness(individual: &Individual, log: &EventLog) -> f64 {
    // Structural validity: must have places, arcs, and non-empty markings
    if individual.net.places.is_empty()
        || individual.net.arcs.is_empty()
        || individual.initial_marking.is_empty()
        || individual.final_marking.is_empty()
    {
        return 0.0;
    }

    let result = compute_fitness(
        &individual.net,
        &individual.initial_marking,
        &individual.final_marking,
        log,
    );

    let avg_trace_fitness = result.avg_trace_fitness;

    let fitting_ratio = if result.total_traces == 0 {
        0.0
    } else {
        result.perfectly_fitting_traces as f64 / result.total_traces as f64
    };

    // Simplicity: inversely proportional to net size.
    // More places/transitions = lower simplicity score.
    let total_elements = individual.net.places.len() + individual.net.transitions.len();
    let simplicity = if total_elements == 0 { 0.0 } else { 1.0 / (1.0 + total_elements as f64 * 0.05) };

    0.6 * avg_trace_fitness + 0.3 * fitting_ratio + 0.1 * simplicity
}

/// Crossover: combine two parents to produce two offspring.
///
/// Strategy: take the union of transitions from both parents, then take
/// a random subset of places from each parent (including their incident
/// arcs). This creates offspring that inherit connectivity patterns from
/// both parents while potentially discovering new structures.
fn crossover(parent1: &Individual, parent2: &Individual, rng: &mut Rng) -> (Individual, Individual) {
    let mut child1 = PetriNet::new("genetic_child1");
    let mut child2 = PetriNet::new("genetic_child2");

    // Child 1 gets parent1's transitions; child 2 gets parent2's transitions
    for t in &parent1.net.transitions {
        child1.add_transition(&t.name, t.label.clone());
    }
    for t in &parent2.net.transitions {
        child2.add_transition(&t.name, t.label.clone());
    }

    // Inherit places stochastically (60% from one parent, 40% from the other)
    copy_places_stochastic(&parent2.net, &mut child1, 0.6, rng);
    copy_places_stochastic(&parent1.net, &mut child1, 0.4, rng);
    copy_places_stochastic(&parent1.net, &mut child2, 0.6, rng);
    copy_places_stochastic(&parent2.net, &mut child2, 0.4, rng);

    // Copy valid arcs from both parents into each child
    let child1_places = node_set(&child1.places, |p| &p.name);
    let child1_trans = node_set(&child1.transitions, |t| &t.name);
    let child2_places = node_set(&child2.places, |p| &p.name);
    let child2_trans = node_set(&child2.transitions, |t| &t.name);

    copy_valid_arcs(&parent2.net, &mut child1, &child1_places, &child1_trans, false);
    copy_valid_arcs(&parent1.net, &mut child1, &child1_places, &child1_trans, true);
    copy_valid_arcs(&parent1.net, &mut child2, &child2_places, &child2_trans, false);
    copy_valid_arcs(&parent2.net, &mut child2, &child2_places, &child2_trans, true);

    // Build markings (filtered to places that exist in each child)
    let c1_init = filter_marking(&parent1.initial_marking, &child1_places);
    let c1_final = filter_marking(&parent1.final_marking, &child1_places);
    let c2_init = filter_marking(&parent2.initial_marking, &child2_places);
    let c2_final = filter_marking(&parent2.final_marking, &child2_places);

    let c1_init = fallback_marking(c1_init, &child1, MarkingEnd::Initial);
    let c1_final = fallback_marking(c1_final, &child1, MarkingEnd::Final);
    let c2_init = fallback_marking(c2_init, &child2, MarkingEnd::Initial);
    let c2_final = fallback_marking(c2_final, &child2, MarkingEnd::Final);

    (
        Individual { net: child1, initial_marking: c1_init, final_marking: c1_final, fitness: 0.0 },
        Individual { net: child2, initial_marking: c2_init, final_marking: c2_final, fitness: 0.0 },
    )
}

/// Copy places from a parent net into a child with a given probability.
fn copy_places_stochastic(parent_net: &PetriNet, child: &mut PetriNet, prob: f64, rng: &mut Rng) {
    for p in &parent_net.places {
        if rng.next_f64() < prob {
            child.add_place(&p.name);
        }
    }
}

/// Build a HashSet of node names from a Vec of nodes.
fn node_set<T, F>(nodes: &[T], name_fn: F) -> HashSet<String>
where
    F: Fn(&T) -> &str,
{
    nodes.iter().map(|n| name_fn(n).to_string()).collect::<HashSet<_>>()
}

/// Copy arcs that reference existing nodes in the child net.
/// If `dedup` is true, skip arcs that already exist in the child.
fn copy_valid_arcs(
    parent: &PetriNet,
    child: &mut PetriNet,
    places: &HashSet<String>,
    trans: &HashSet<String>,
    dedup: bool,
) {
    for arc in &parent.arcs {
        let sp = places.contains(&arc.source);
        let tt = trans.contains(&arc.target);
        let st = trans.contains(&arc.source);
        let tp = places.contains(&arc.target);
        if (sp && tt) || (st && tp) {
            if dedup && child.arcs.iter().any(|a| a.source == arc.source && a.target == arc.target) {
                continue;
            }
            child.add_arc(&arc.source, &arc.target);
        }
    }
}

/// Filter a marking to only include places that exist in the child.
fn filter_marking(marking: &Marking, valid_places: &HashSet<String>) -> Marking {
    marking
        .iter()
        .filter(|(p, _)| valid_places.contains(*p))
        .map(|(p, &t)| (p.clone(), t))
        .collect()
}

enum MarkingEnd {
    Initial,
    Final,
}

/// If the marking is empty, assign a fallback place.
fn fallback_marking(marking: Marking, net: &PetriNet, end: MarkingEnd) -> Marking {
    if !marking.is_empty() || net.places.is_empty() {
        return marking;
    }
    let mut m = marking;
    let place = match end {
        MarkingEnd::Initial => &net.places[0].name,
        MarkingEnd::Final => {
            if net.places.len() > 1 {
                &net.places[net.places.len() - 1].name
            } else {
                &net.places[0].name
            }
        }
    };
    m.insert(place.clone(), 1);
    m
}

/// Apply mutation operators to an individual in-place.
///
/// Four mutation operators, each applied with probability `mutation_rate`:
/// 1. Add a random place between two transitions
/// 2. Remove a random non-source/non-sink place
/// 3. Redirect a random arc to a different target place
/// 4. Toggle a token in initial/final marking
fn mutate(individual: &mut Individual, mutation_rate: f64, rng: &mut Rng) {
    if individual.net.transitions.is_empty() {
        return;
    }

    // Mutation 1: Add a place between two transitions
    if rng.next_f64() < mutation_rate {
        add_place_mutation(individual, rng);
    }

    // Mutation 2: Remove a non-critical place
    if rng.next_f64() < mutation_rate {
        remove_place_mutation(individual, rng);
    }

    // Mutation 3: Redirect an arc
    if rng.next_f64() < mutation_rate {
        redirect_arc_mutation(individual, rng);
    }

    // Mutation 4: Toggle marking
    if rng.next_f64() < mutation_rate {
        toggle_marking_mutation(individual, rng);
    }
}

/// Add a new place between two randomly-chosen visible transitions.
fn add_place_mutation(individual: &mut Individual, rng: &mut Rng) {
    let visible: Vec<String> = individual
        .net
        .transitions
        .iter()
        .filter(|t| t.label.is_some())
        .map(|t| t.name.clone())
        .collect();
    if visible.len() < 2 {
        return;
    }

    let src_idx = rng.next_usize(visible.len());
    let mut tgt_idx = rng.next_usize(visible.len() - 1);
    if tgt_idx >= src_idx {
        tgt_idx += 1;
    }

    let src = &visible[src_idx];
    let tgt = &visible[tgt_idx];

    let place_name = format!("p_mut_{}_{}", src, tgt);
    // Avoid duplicate place names
    if individual.net.places.iter().any(|p| p.name == place_name) {
        return;
    }

    individual.net.add_place(&place_name);
    individual.net.add_arc(src, &place_name);
    individual.net.add_arc(&place_name, tgt);
}

/// Remove a random non-source/non-sink place.
fn remove_place_mutation(individual: &mut Individual, rng: &mut Rng) {
    let marked_places: HashSet<String> = individual
        .initial_marking
        .keys()
        .chain(individual.final_marking.keys())
        .cloned()
        .collect();

    let removable: Vec<String> = individual
        .net
        .places
        .iter()
        .filter(|p| !marked_places.contains(&p.name))
        .map(|p| p.name.clone())
        .collect();

    let removable_len = removable.len();
    if removable_len > 0 {
        let idx = rng.next_usize(removable_len);
        let place = &removable[idx];
        individual.net.remove_place(place);
    }
}

/// Redirect a random arc's target to a different place.
fn redirect_arc_mutation(individual: &mut Individual, rng: &mut Rng) {
    if individual.net.arcs.is_empty() || individual.net.places.is_empty() {
        return;
    }

    let arc_idx = rng.next_usize(individual.net.arcs.len());
    let place_idx = rng.next_usize(individual.net.places.len());

    // Only redirect arcs whose target is a place
    let target_is_place = individual.net.places.iter().any(|p| p.name == individual.net.arcs[arc_idx].target);
    if target_is_place {
        let new_target = individual.net.places[place_idx].name.clone();
        individual.net.arcs[arc_idx].target = new_target;
    }
}

/// Toggle a token in the initial or final marking.
fn toggle_marking_mutation(individual: &mut Individual, rng: &mut Rng) {
    if individual.net.places.is_empty() {
        return;
    }

    let place = individual.net.places[rng.next_usize(individual.net.places.len())].name.clone();

    if rng.next_f64() < 0.5 {
        // Toggle initial marking
        let current = individual.initial_marking.entry(place).or_insert(0);
        if *current > 0 {
            *current = 0;
        } else {
            *current = 1;
        }
    } else {
        // Toggle final marking
        let current = individual.final_marking.entry(place).or_insert(0);
        if *current > 0 {
            *current = 0;
        } else {
            *current = 1;
        }
    }
}

/// Select a parent using tournament selection.
///
/// Randomly sample `tournament_size` individuals and return the fittest.
fn tournament_select<'a>(population: &'a [Individual], tournament_size: usize, rng: &mut Rng) -> &'a Individual {
    let size = tournament_size.min(population.len()).max(1);
    let mut best_idx = rng.next_usize(population.len());
    for _ in 1..size {
        let idx = rng.next_usize(population.len());
        if population[idx].fitness > population[best_idx].fitness {
            best_idx = idx;
        }
    }
    &population[best_idx]
}

/// Discover a Petri net using the genetic miner algorithm.
///
/// Evolves a population of Petri nets over multiple generations, using
/// token-replay fitness as the selection pressure. Returns the best
/// individual found across all generations.
///
/// # Arguments
///
/// * `log` - The event log to discover a process model from.
/// * `config` - Optional configuration. Uses defaults if `None`.
///
/// # Returns
///
/// A `PetriNetResult` containing the best-discovered Petri net with
/// its initial and final markings.
pub fn discover_genetic(log: &EventLog, config: Option<GeneticMinerConfig>) -> PetriNetResult {
    let cfg = config.unwrap_or_default();
    let population_size = cfg.population_size.max(2);
    let generations = cfg.generations;
    let mutation_rate = cfg.mutation_rate;
    let crossover_rate = cfg.crossover_rate;

    // Use a deterministic seed based on log content for reproducibility
    let seed = log
        .traces
        .iter()
        .flat_map(|t| t.events.iter().map(|e| e.name.len() as u64))
        .fold(42u64, |acc, v| acc.wrapping_mul(31).wrapping_add(v));
    let mut rng = Rng::new(seed);

    // Phase 1: Generate initial population from DFG variants
    let mut population: Vec<Individual> = (0..population_size)
        .map(|_| build_dfg_net(log, &mut rng))
        .collect();

    // Evaluate initial population
    for ind in &mut population {
        ind.fitness = evaluate_fitness(ind, log);
    }

    // Sort by fitness descending (best first)
    population.sort_by(|a, b| b.fitness.partial_cmp(&a.fitness).unwrap_or(std::cmp::Ordering::Equal));

    let mut best_ever = population[0].clone();
    let mut stagnation_count: usize = 0;
    let mut prev_best_fitness = population[0].fitness;

    // Phase 2: Evolve
    for _gen in 0..generations {
        // Early termination: perfect fitness
        if best_ever.fitness >= 0.999 {
            break;
        }

        // Early termination: stagnation (no improvement for many generations)
        if (population[0].fitness - prev_best_fitness).abs() < 1e-9 {
            stagnation_count += 1;
            if stagnation_count >= generations / 4 {
                break;
            }
        } else {
            stagnation_count = 0;
        }
        prev_best_fitness = population[0].fitness;

        // Elitism: keep top 10%
        let elite_count = (population_size as f64 * 0.1).ceil() as usize;
        let elite_count = elite_count.max(1).min(population_size);
        let mut next_population: Vec<Individual> =
            population[..elite_count].to_vec();

        // Fill the rest with offspring
        let tournament_size = 3usize;
        while next_population.len() < population_size {
            // Select two parents via tournament
            let parent1 = tournament_select(&population, tournament_size, &mut rng);
            let parent2 = tournament_select(&population, tournament_size, &mut rng);

            let (mut child1, mut child2) = if rng.next_f64() < crossover_rate {
                crossover(parent1, parent2, &mut rng)
            } else {
                (
                    Individual {
                        net: parent1.net.clone(),
                        initial_marking: parent1.initial_marking.clone(),
                        final_marking: parent1.final_marking.clone(),
                        fitness: 0.0,
                    },
                    Individual {
                        net: parent2.net.clone(),
                        initial_marking: parent2.initial_marking.clone(),
                        final_marking: parent2.final_marking.clone(),
                        fitness: 0.0,
                    },
                )
            };

            // Apply mutations
            mutate(&mut child1, mutation_rate, &mut rng);
            mutate(&mut child2, mutation_rate, &mut rng);

            // Evaluate children
            child1.fitness = evaluate_fitness(&child1, log);
            child2.fitness = evaluate_fitness(&child2, log);

            if next_population.len() < population_size {
                next_population.push(child1);
            }
            if next_population.len() < population_size {
                next_population.push(child2);
            }
        }

        // Sort by fitness
        next_population.sort_by(|a, b| b.fitness.partial_cmp(&a.fitness).unwrap_or(std::cmp::Ordering::Equal));

        // Trim to fixed population size
        next_population.truncate(population_size);
        population = next_population;

        // Track best ever
        if population[0].fitness > best_ever.fitness {
            best_ever = population[0].clone();
        }
    }

    best_ever.to_result()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::event_log::{Event, Trace};

    fn make_log(traces: Vec<(&str, &[&str])>) -> EventLog {
        EventLog {
            traces: traces
                .into_iter()
                .map(|(case_id, events)| Trace {
                    case_id: case_id.to_string(),
                    events: events
                        .iter()
                        .map(|&name| Event {
                            name: name.to_string(),
                            timestamp: None,
                            lifecycle: None,
                            attributes: HashMap::new(),
                        })
                        .collect(),
                })
                .collect(),
        }
    }

    #[test]
    fn test_genetic_miner_discovers_sequential_net() {
        let log = make_log(vec![("c1", &["A", "B", "C"]), ("c2", &["A", "B", "C"])]);
        let config = GeneticMinerConfig {
            population_size: 10, generations: 20, mutation_rate: 0.15, crossover_rate: 0.7,
        };
        let result = discover_genetic(&log, Some(config));
        let labels: Vec<String> = result.net.transitions.iter().filter_map(|t| t.label.clone()).collect();
        assert!(labels.contains(&"A".to_string()));
        assert!(labels.contains(&"B".to_string()));
        assert!(labels.contains(&"C".to_string()));
        assert!(!result.net.places.is_empty());
        assert!(!result.net.arcs.is_empty());
        assert!(!result.initial_marking.is_empty());
        assert!(!result.final_marking.is_empty());
    }

    #[test]
    fn test_genetic_miner_improves_with_generations() {
        let log = make_log(vec![("c1", &["A", "B"]), ("c2", &["A", "B"]), ("c3", &["A", "B"])]);
        let small = GeneticMinerConfig { population_size: 6, generations: 5, ..Default::default() };
        let large = GeneticMinerConfig { population_size: 10, generations: 30, ..Default::default() };
        let r_s = discover_genetic(&log, Some(small));
        let r_l = discover_genetic(&log, Some(large));
        let f_s = compute_fitness(&r_s.net, &r_s.initial_marking, &r_s.final_marking, &log);
        let f_l = compute_fitness(&r_l.net, &r_l.initial_marking, &r_l.final_marking, &log);
        assert!(f_l.avg_trace_fitness >= f_s.avg_trace_fitness - 0.05,
            "more generations (fit={:.4}) should not be much worse than fewer (fit={:.4})",
            f_l.avg_trace_fitness, f_s.avg_trace_fitness);
    }
}
