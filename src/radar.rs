use bevy_egui::{egui::{self, Align2, Color32, Pos2, Stroke}, EguiContexts};


pub fn update_radar_ui(mut contexts: EguiContexts) {
    egui::Area::new("radar")
        .anchor(Align2::RIGHT_TOP, (-125.0, 125.0))
        .show(contexts.ctx_mut(), |ui| {
            ui.label("TODO: Radar goes here");
        });
}
