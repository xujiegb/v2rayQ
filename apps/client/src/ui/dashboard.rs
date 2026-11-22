use super::*;

impl ClientApp {
    pub(super) fn home(&mut self, ui: &mut egui::Ui) {
        let tr = self.translator;
        heading(ui, tr.text("Overview"));
        ui.label(RichText::new(tr.text("Your connection, at a glance.")).color(MUTED));
        ui.add_space(12.0);
        let mut command = None;
        card(ui, |ui| {
            ui.horizontal_wrapped(|ui| {
                ui.strong(tr.text("Proxy"));
                egui::ComboBox::from_id_salt("proxy_mode")
                    .selected_text(tr.text(if self.proxy_mode == NetworkMode::Tun { "TUN" } else { "Transparent proxy" }))
                    .show_ui(ui, |ui| {
                        ui.selectable_value(&mut self.proxy_mode, NetworkMode::Tun, tr.text("TUN"));
                        ui.selectable_value(&mut self.proxy_mode, NetworkMode::Transparent, tr.text("Transparent proxy"));
                    });
                let (status, other) = if self.proxy_mode == NetworkMode::Tun {
                    (&self.runtime.tun, &self.runtime.transparent)
                } else { (&self.runtime.transparent, &self.runtime.tun) };
                let running = status.running == Some(true);
                ui.label(tr.text(service_label(status)));
                let available = status.can_change && self.pending == 0 && (running || other.running != Some(true));
                let title = tr.text(if running { "Turn off proxy" } else { "Turn on proxy" });
                if ui.add_enabled(available, egui::Button::new(title).selected(running)).clicked() {
                    command = Some(Command::SetNetwork { mode: self.proxy_mode, enabled: !running });
                }
                if other.running == Some(true) && !running { ui.label(tr.text("Stop the other mode before switching.")); }
                if ui.add_enabled(self.pending == 0, egui::Button::new(tr.text("Refresh status"))).clicked() {
                    command = Some(Command::Status);
                }
            });
            ui.label(RichText::new(tr.text("Controls the installed host service. Node changes await the Core adapter.")).small().color(MUTED));
        });
        if let Some(command) = command { self.send(command); }
        ui.add_space(12.0);
        let selected = self.profile.nodes.iter().find(|node| Some(node.id) == self.profile.selected_node);
        egui::Frame::new().fill(INK).corner_radius(18.0).inner_margin(egui::Margin::same(24)).show(ui, |ui| {
            ui.set_min_width(ui.available_width());
            ui.label(RichText::new(tr.text("Current node")).color(Color32::LIGHT_GRAY).size(13.0));
            ui.label(RichText::new(selected.map_or(tr.text("Choose a node"), |node| node.name.as_str())).size(27.0).strong().color(Color32::WHITE));
            if let Some(node) = selected {
                ui.label(RichText::new(format!("{} · {}:{}", node.protocol.label(), node.host, node.port)).color(Color32::LIGHT_GRAY));
                let source = self.profile.subscriptions.iter().find(|item| Some(item.id) == node.subscription_id).map_or(tr.text("Local nodes"), |item| item.name.as_str());
                ui.label(RichText::new(source).color(Color32::LIGHT_GRAY));
            }
            if ui.button(tr.text("Manage nodes")).clicked() { self.page = Page::Nodes; }
        });
        ui.add_space(12.0);
        card(ui, |ui| {
            ui.strong(tr.text("Live traffic"));
            ui.horizontal_wrapped(|ui| {
                for (label, speed) in [("Download", self.runtime.traffic.download_bps), ("Upload", self.runtime.traffic.upload_bps)] {
                    ui.label(tr.text(label));
                    ui.label(RichText::new(speed.map_or_else(|| "—".into(), format_speed)).size(25.0).strong().color(ACCENT));
                }
            });
            if !self.runtime.traffic.core_connected { ui.label(tr.text("Traffic statistics await the Core adapter.")); }
        });
        ui.add_space(12.0);
        let mut inspect = false;
        card(ui, |ui| {
            ui.horizontal_wrapped(|ui| {
                ui.strong(tr.text("Host public IP"));
                if ui.add_enabled(self.pending == 0, egui::Button::new(tr.text("Check IP location"))).clicked() { inspect = true; }
            });
            if let Some(info) = &self.ip_info {
                ui.label(RichText::new(info.ip.to_string()).size(22.0).strong());
                ui.label([info.city.as_str(), info.region.as_str(), info.country.as_str()].into_iter().filter(|text| !text.is_empty()).collect::<Vec<_>>().join(" · "));
                ui.label(&info.isp);
            } else { ui.label(tr.text("Not checked")); }
            ui.label(RichText::new(tr.text("Checked on demand via IP.SB from the backend host; it may differ from the proxy exit.")).small().color(MUTED));
        });
        if inspect { self.send(Command::InspectNetwork); }
    }
}

fn format_speed(bytes: u64) -> String {
    if bytes >= 1024 * 1024 { format!("{:.1} MiB/s", bytes as f64 / (1024.0 * 1024.0)) }
    else if bytes >= 1024 { format!("{:.1} KiB/s", bytes as f64 / 1024.0) }
    else { format!("{bytes} B/s") }
}
