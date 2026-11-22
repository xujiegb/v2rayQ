use super::*;

pub(super) fn section(ui: &mut egui::Ui, title: &str) {
    ui.label(RichText::new(title).size(18.0).strong().color(INK));
    ui.add_space(7.0);
}

pub(super) fn metric(ui: &mut egui::Ui, title: &str, value: &str, description: &str) {
    ui.group(|ui| {
        ui.set_min_size(Vec2::new(180.0, 110.0));
        ui.label(RichText::new(title).small().color(MUTED));
        ui.label(RichText::new(value).size(23.0).strong().color(INK));
        ui.label(RichText::new(description).small().color(MUTED));
    });
}

pub(super) fn form_row(ui: &mut egui::Ui, label: &str, value: &mut String) {
    if ui.available_width() < 580.0 {
        ui.label(label);
        ui.add(egui::TextEdit::singleline(value).desired_width(ui.available_width().min(500.0)));
    } else {
        ui.horizontal(|ui| {
            ui.add_sized([175.0, 22.0], egui::Label::new(label));
            ui.add_sized([340.0, 25.0], egui::TextEdit::singleline(value));
        });
    }
}

pub(super) fn protocol_name(value: &Protocol) -> &'static str {
    match value {
        Protocol::Vless(_) => "VLESS",
        Protocol::Vmess(_) => "VMess",
        Protocol::Trojan(_) => "Trojan",
        Protocol::Shadowsocks(_) => "Shadowsocks",
        Protocol::AnyTls(_) => "AnyTLS",
        Protocol::Hysteria2(_) => "Hysteria2",
    }
}

