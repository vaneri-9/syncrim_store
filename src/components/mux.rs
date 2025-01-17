use crate::common::{Component, Id, Input, OutputType, Ports, Simulator};
use log::*;
use serde::{Deserialize, Serialize};
#[derive(Serialize, Deserialize)]
pub struct Mux {
    pub id: Id,
    pub pos: (f32, f32),
    pub select: Input,
    pub m_in: Vec<Input>,
}

#[typetag::serde]
impl Component for Mux {
    fn to_(&self) {
        trace!("mux");
    }

    fn get_id_ports(&self) -> (Id, Ports) {
        let mut inputs = vec![self.select.clone()];
        let mut m = self.m_in.clone();
        inputs.append(&mut m);

        (
            self.id.clone(),
            Ports {
                inputs,
                out_type: OutputType::Combinatorial,
                outputs: vec!["out".into()],
            },
        )
    }

    // propagate selected input value to output
    fn clock(&self, simulator: &mut Simulator) {
        // get input value
        let select = simulator.get_input_val(&self.select) as usize;
        trace!("select {}", select);
        let value = simulator.get_input_val(&self.m_in[select]);

        // set output
        simulator.set_out_val(&self.id, "out", value);
    }
}
