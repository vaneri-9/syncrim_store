use std::{path::PathBuf, rc::Rc};
use syncrim::{
    common::{ComponentStore, Input},
    components::*,
    fern::fern_setup,
};

fn main() {
    fern_setup();
    let cs = ComponentStore {
        store: vec![
            Rc::new(ProbeEdit::new("probe_edit", (100.0, 100.0))),
            Rc::new(Probe {
                id: "probe".to_string(),
                pos: (250.0, 100.0),
                input: Input::new("probe_edit", "out"),
            }),
        ],
    };

    let path = PathBuf::from("probe_edit.json");
    cs.save_file(&path);

    #[cfg(feature = "gui-egui")]
    syncrim::gui_egui::gui(&cs, &path).ok();

    #[cfg(feature = "gui-vizia")]
    syncrim::gui_vizia::gui(&cs, &path);
}
