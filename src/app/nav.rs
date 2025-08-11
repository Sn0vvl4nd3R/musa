use super::*;

pub(super) fn nav_tabs(app: &mut super::MusaApp, ui: &mut egui::Ui) {
    ui.horizontal(|ui| {
        let mut tab = |label: &str, v: UiView| {
            let selected = app.view == v;
            if ui.selectable_label(selected, label).clicked() {
                app.view = v;
            }
        };
        tab("Browser", UiView::Browser);
        tab("Player", UiView::Player);
        tab("Playlist", UiView::Playlist);
    });
    ui.add_space(6.0);
}
