// PM4Py – A Process Mining Library for Python (POWL v2 WASM)
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

/// Simulation algorithms for process mining.
///
/// Ports `pm4py.algo.simulation` for browser-native log generation.
///
/// The main function is `play_out()` which simulates event logs from process models:
/// - Process tree playout: Generate traces by executing a process tree
/// - DFG playout: Generate traces from a directly-follows graph
/// - Petri net playout: Generate traces from a Petri net
pub mod playout;

pub use playout::{play_out_process_tree, play_out_dfg, PlayOutParameters};
