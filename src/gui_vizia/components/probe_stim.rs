use crate::{
    common::{Component, Signal, Simulator, ViziaComponent},
    components::ProbeStim,
    gui_vizia::{popup::NewPopup, tooltip::new_component_tooltip},
};

use vizia::prelude::*;

use log::*;

#[typetag::serde]
impl ViziaComponent for ProbeStim {
    // create view
    fn view(&self, cx: &mut Context) {
        trace!("---- Create ProbeStim View");
        let values = self.values.clone();
        View::build(ProbeStimView {}, cx, |cx| {
            Binding::new(
                cx,
                crate::gui_vizia::GuiData::simulator.then(Simulator::cycle),
                move |cx, cycle| {
                    let cycle = cycle.get(cx);
                    let rhs = if let Some(value) = values.get(cycle - 1) {
                        *value
                    } else {
                        Signal::Unknown
                    };
                    Label::new(cx, &format!("{:?}", rhs)).hoverable(false);
                },
            );
            NewPopup::new(cx, self.get_id_ports()).position_type(PositionType::SelfDirected);
        })
        .position_type(PositionType::SelfDirected)
        .background_color(Color::lightblue())
        .left(Pixels(self.pos.0 - 10.0))
        .top(Pixels(self.pos.1 - 10.0))
        .width(Auto)
        // .width() // TODO, maybe some max width
        .height(Pixels(20.0))
        // TODO: do we want/need tooltip/popup for constants
        .on_press(|ex| ex.emit(PopupEvent::Switch))
        .tooltip(|cx| new_component_tooltip(cx, self));
    }
}
pub struct ProbeStimView {}

impl View for ProbeStimView {
    fn element(&self) -> Option<&'static str> {
        Some("ProbeStim")
    }
}
