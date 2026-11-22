use super::*;

pub(super) fn heading(ui: &mut egui::Ui, value: &str) {
    ui.heading(RichText::new(value).size(29.0).strong().color(INK));
    ui.add_space(12.0);
}

pub(super) fn card(ui: &mut egui::Ui, content: impl FnOnce(&mut egui::Ui)) {
    egui::Frame::new()
        .fill(Color32::WHITE)
        .corner_radius(11.0)
        .stroke(egui::Stroke::new(1.0, NAVIGATION))
        .inner_margin(egui::Margin::same(16))
        .show(ui, |ui| {
            ui.set_min_width(ui.available_width());
            content(ui);
        });
    ui.add_space(10.0);
}

pub(super) fn form_row(ui: &mut egui::Ui, caption: &str, value: &mut String, secret: bool) {
    ui.label(RichText::new(caption).color(MUTED));
    ui.add(
        egui::TextEdit::singleline(value)
            .password(secret)
            .desired_width(ui.available_width().min(510.0)),
    );
}

pub(super) fn service_label(status: &ServiceStatus) -> &'static str {
    match (status.installed, status.running) {
        (false, _) => "Service not installed",
        (true, Some(true)) => "Running",
        (true, Some(false)) => "Stopped",
        (true, None) => "Status unavailable",
    }
}
