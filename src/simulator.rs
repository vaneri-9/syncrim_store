use crate::common::{Component, ComponentStore, Id, Input, OutputType, Signal, Simulator};
use petgraph::{
    algo::toposort,
    dot::{Config, Dot},
    Graph,
};

use log::*;
use std::collections::HashMap;
use std::{fs::File, io::prelude::*, path::PathBuf};

pub struct IdComponent(pub HashMap<String, Box<dyn Component>>);

// Notice:
// The topological order does not enforce any specific order of registers
// Thus registers cannot point to other registers in a cyclic fashion
// This is (likely) not occurring in practice.
//
// A solution is to evaluate register updates separately from other components
// ... but not currently implemented ...
impl Simulator {
    pub fn new(component_store: &ComponentStore, clock: &mut usize) -> Self {
        let mut lens_values = vec![];

        let mut id_start_index = HashMap::new();
        let mut id_component = HashMap::new(); // IdComponent(HashMap::new());

        let mut id_nr_outputs = HashMap::new();
        let mut id_field_index = HashMap::new();
        // allocate storage for lensed outputs

        trace!("-- allocate storage for lensed outputs");
        for c in &component_store.store {
            let (id, ports) = c.get_id_ports();

            trace!("id {}, ports {:?}", id, ports);
            // start index for outputs related to component
            if id_start_index
                .insert(id.clone(), lens_values.len())
                .is_some()
            {
                panic!("Component identifier {:?} is defined twice", id);
            }

            id_component.insert(id.clone(), c);

            // create placeholder for output
            #[allow(clippy::same_item_push)]
            for (index, field_id) in ports.outputs.iter().enumerate() {
                // create the value with a default to 0
                lens_values.push(0);
                if id_field_index
                    .insert((id.clone(), field_id.into()), index)
                    .is_some()
                {
                    panic!("Component {:?} field {:?} is defined twice", id, field_id)
                };
            }
            id_nr_outputs.insert(id.clone(), ports.outputs.len());
        }

        let mut graph = Graph::<_, (), petgraph::Directed>::new();
        let mut id_node = HashMap::new();
        let mut node_comp = HashMap::new();

        // insert nodes
        for (id, c) in &id_component {
            let node = graph.add_node(id.to_owned());
            id_node.insert(id, node);
            node_comp.insert(node, c);
        }

        trace!("\nid_node {:?}", id_node);

        for (node, c) in &node_comp {
            trace!("node {:?}, comp_id {:?}", node, c.get_id_ports());
        }

        // insert edges
        for (to_id, c) in &id_component {
            let to_component = id_component.get(to_id).unwrap();
            let (_, ports) = to_component.get_id_ports();

            trace!("to_id :{}, ports: {:?}", to_id, ports);

            if ports.out_type == OutputType::Combinatorial {
                let to_node = id_node.get(to_id).unwrap();
                let (_, ports) = c.get_id_ports();
                for in_port in &ports.inputs {
                    let from_id = &in_port.id;

                    let from_node = id_node.get(from_id).unwrap();
                    graph.add_edge(*from_node, *to_node, ());
                    trace!(
                        "add_edge {}:{:?} -> {}:{:?}",
                        from_id,
                        from_node,
                        to_id,
                        to_node
                    );
                }
            }
        }

        // topological order
        let top =
            toposort(&graph, None).expect("Topological sort failed, your model contains loops.");
        trace!("--- top \n{:?}", top);

        let mut ordered_components = vec![];
        for node in &top {
            // #[allow(clippy::clone_double_ref)] // old lint
            #[allow(suspicious_double_ref_op)] // changed in Rust 1.71
            let c = (**node_comp.get(node).unwrap()).clone();
            ordered_components.push(c);
        }

        let component_ids: Vec<Id> = ordered_components
            .iter()
            .map(|c| c.get_id_ports().0)
            .collect();

        let mut simulator = Simulator {
            id_start_index,
            ordered_components,
            id_nr_outputs,
            id_field_index,
            sim_state: lens_values,
            history: vec![],
            component_ids,
            graph,
        };

        trace!("sim_state {:?}", simulator.sim_state);

        simulator.clock(clock);
        simulator
    }

    /// get input by index
    pub(crate) fn get(&self, index: usize) -> Signal {
        self.sim_state[index]
    }

    /// get input value
    pub fn get_input_val(&self, input: &Input) -> Signal {
        let nr_out = *self.id_nr_outputs.get(&input.id).unwrap();
        let index = *self
            .id_field_index
            .get(&(input.id.clone(), input.field.clone()))
            .unwrap_or_else(|| {
                panic!(
                    "Component {:?}, field {:?} not found.",
                    input.id, input.field
                )
            });
        if index < nr_out {
            let start_index = *self.id_start_index.get(&input.id).unwrap();
            self.get(start_index + index)
        } else {
            panic!(
                "ICE: Attempt to read {:?} at index {}, where {:?} has only {} outputs.",
                input.id, index, input.id, nr_out
            )
        }
    }

    /// get start index by id
    pub(crate) fn get_id_start_index(&self, id: &str) -> usize {
        *self.id_start_index.get(id).unwrap()
    }

    // set value by index
    fn set(&mut self, index: usize, value: Signal) {
        self.sim_state[index] = value;
    }

    /// set value by Id (instance) and Id (field)
    pub fn set_out_val(&mut self, id: &str, field: &str, value: Signal) {
        let index = *self
            .id_field_index
            .get(&(id.into(), field.into()))
            .unwrap_or_else(|| panic!("Component {}, field {} not found.", id, field));
        let start_index = self.get_id_start_index(id);
        self.set(start_index + index, value);
    }

    /// iterate over the evaluators and increase clock by one
    pub fn clock(&mut self, clock: &mut usize) {
        // push current state
        self.history.push(self.sim_state.clone());
        let ordered_components = self.ordered_components.clone();

        for component in ordered_components {
            component.clock(self);
        }
        *clock = self.history.len();
    }

    /// reverse simulation using history if clock > 1
    pub fn un_clock(&mut self, clock: &mut usize) {
        if *clock > 1 {
            let state = self.history.pop().unwrap();
            // set old state
            self.sim_state = state;
            let ordered_components = self.ordered_components.clone();

            for component in ordered_components {
                component.un_clock();
            }
        }
        *clock = self.history.len();
    }

    /// reset simulator
    pub fn reset(&mut self, clock: &mut usize) {
        self.history = vec![];
        self.sim_state.iter_mut().for_each(|val| *val = 0);
        self.clock(clock);
    }

    /// save as `dot` file with `.gv` extension
    pub fn save_dot(&self, path: &PathBuf) {
        let mut path = path.to_owned();
        path.set_extension("gv");
        let mut file = File::create(path).unwrap();
        let dot_string = format!(
            "{:?}",
            Dot::with_config(&self.graph, &[Config::EdgeNoLabel])
        );
        file.write_all(dot_string.as_bytes()).unwrap();
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::components::*;
    use std::rc::Rc;

    #[test]
    fn test_define() {
        let cs = ComponentStore {
            store: vec![Rc::new(ProbeOut::new("po1"))],
        };

        let mut clock = 0;
        let _simulator = Simulator::new(&cs, &mut clock);

        assert_eq!(clock, 1);
    }

    #[test]
    #[should_panic(expected = "Component identifier \"po1\" is defined twice")]
    fn test_redefined() {
        let cs = ComponentStore {
            store: vec![Rc::new(ProbeOut::new("po1")), Rc::new(ProbeOut::new("po1"))],
        };

        let mut clock = 0;
        let _simulator = Simulator::new(&cs, &mut clock);

        assert_eq!(clock, 1);
    }

    #[test]
    fn test_get_input_val() {
        let cs = ComponentStore {
            store: vec![Rc::new(ProbeOut::new("po1"))],
        };

        let mut clock = 0;
        let simulator = Simulator::new(&cs, &mut clock);

        assert_eq!(clock, 1);
        let _ = simulator.get_input_val(&Input::new("po1", "out"));
    }

    #[test]
    #[should_panic(expected = "Component \"po1\", field \"missing\" not found.")]
    fn test_get_input_out_of_range() {
        let cs = ComponentStore {
            store: vec![Rc::new(ProbeOut::new("po1"))],
        };

        let mut clock = 0;
        let simulator = Simulator::new(&cs, &mut clock);

        assert_eq!(clock, 1);
        let _ = simulator.get_input_val(&Input::new("po1", "missing"));
    }
}
