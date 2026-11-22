use super::*;

impl ClientApp {
    pub(super) fn home(&mut self, ui: &mut egui::Ui) {
        ui.heading(RichText::new("Overview").size(29.0).color(INK));
        ui.label(RichText::new("Your connection at a glance").color(MUTED));
        ui.add_space(20.0);
        if ui.available_width() < 650.0 {
            metric(ui, "ENGINE", "Not connected", "Waiting for Core");
            metric(ui, "NODES", &self.data.nodes.len().to_string(), "Saved locally");
            metric(ui, "SUBSCRIPTIONS", &self.data.subscriptions.len().to_string(), "HTTPS imports");
        } else {
            ui.columns(3, |columns| {
                metric(&mut columns[0], "ENGINE", "Not connected", "Waiting for Core");
                metric(&mut columns[1], "NODES", &self.data.nodes.len().to_string(), "Saved locally");
                metric(&mut columns[2], "SUBSCRIPTIONS", &self.data.subscriptions.len().to_string(), "HTTPS imports");
            });
        }
        ui.add_space(23.0);
        section(ui, "Connection");
        ui.group(|ui| {
            ui.set_min_width(ui.available_width() - 20.0);
            ui.label(RichText::new("No live traffic yet").size(19.0).strong().color(INK));
            ui.label(RichText::new("The external Core API is not available. No traffic, latency, or tunnel status is fabricated.").color(MUTED));
            ui.add_space(8.0);
            if ui.button("Manage nodes").clicked() {
                self.page = Page::Nodes;
            }
        });
    }

    pub(super) fn nodes(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        ui.horizontal_wrapped(|ui| {
            ui.heading(RichText::new("Nodes").size(29.0).color(INK));
            ui.add_space(18.0);
            if ui.button("＋ Share link").clicked() { self.add_link = !self.add_link; self.add_manual = false; }
            if ui.button("＋ Manual node").clicked() { self.add_manual = !self.add_manual; self.add_link = false; }
        });
        ui.label(RichText::new("Import or configure proxy endpoints").color(MUTED));
        ui.add_space(14.0);
        if self.add_link {
            ui.group(|ui| {
                ui.label("Share URI");
                ui.text_edit_singleline(&mut self.share_link);
                if ui.button("Import node").clicked() {
                    match self.data.add_share_link(&self.share_link) {
                        Ok(()) => { self.notice = "Node added".into(); self.share_link.clear(); self.add_link = false; self.save(); }
                        Err(error) => self.notice = error,
                    }
                }
            });
            ui.add_space(10.0);
        }
        if self.add_manual {
            ui.group(|ui| {
                ui.horizontal_wrapped(|ui| {
                    ui.label("Protocol");
                    egui::ComboBox::from_id_salt("protocol")
                        .selected_text(["VLESS", "VMess", "Trojan", "Shadowsocks", "AnyTLS", "Hysteria2"][self.manual.protocol])
                        .show_ui(ui, |ui| {
                            for (index, name) in ["VLESS", "VMess", "Trojan", "Shadowsocks", "AnyTLS", "Hysteria2"].iter().enumerate() {
                                ui.selectable_value(&mut self.manual.protocol, index, *name);
                            }
                        });
                });
                form_row(ui, "Name", &mut self.manual.name);
                form_row(ui, "Server", &mut self.manual.host);
                form_row(ui, "Port", &mut self.manual.port);
                ui.label("UUID / password");
                ui.add(egui::TextEdit::singleline(&mut self.manual.credential)
                    .password(true)
                    .desired_width(ui.available_width().min(500.0)));
                if self.manual.protocol == 3 {
                    form_row(ui, "Cipher", &mut self.manual.method);
                }
                if self.manual.protocol != 3 {
                    form_row(ui, "TLS server name (optional)", &mut self.manual.server_name);
                }
                if ui.button("Save node").clicked() {
                    match self.manual.create() {
                        Ok(node) => {
                            self.data.add_node(node);
                            self.notice = "Node saved".into();
                            self.add_manual = false;
                            self.manual = ManualForm {
                                port: "443".to_owned(),
                                method: "aes-128-gcm".to_owned(),
                                ..Default::default()
                            };
                            self.save();
                        }
                        Err(error) => self.notice = error,
                    }
                }
            });
            ui.add_space(12.0);
        }
        section(ui, "Saved nodes");
        let mut remove = None;
        let mut select = None;
        egui::ScrollArea::vertical().max_height(250.0).show(ui, |ui| {
            if self.data.nodes.is_empty() { ui.label("No nodes yet. Import a link or add one manually."); }
            for entry in &self.data.nodes {
                ui.horizontal_wrapped(|ui| {
                    let picked = self.data.selected_node == Some(entry.node.id);
                    if ui.selectable_label(picked, format!("{}   ·   {}", entry.node.name, protocol_name(&entry.node.protocol))).clicked() {
                        select = Some(entry.node.id);
                    }
                    ui.label(RichText::new(format!("{}:{}", entry.node.endpoint.host, entry.node.endpoint.port)).color(MUTED));
                    if ui.small_button("Delete").clicked() { remove = Some(entry.node.id); }
                });
                ui.separator();
            }
        });
        if let Some(id) = select { self.data.selected_node = Some(id); self.save(); }
        if let Some(id) = remove { self.data.remove_node(id); self.save(); }
        ui.add_space(15.0);
        ui.horizontal_wrapped(|ui| {
            section(ui, "Subscriptions");
            if ui.button("＋ Add subscription").clicked() {
                self.add_subscription = !self.add_subscription;
            }
        });
        if self.add_subscription {
            ui.group(|ui| {
                form_row(ui, "Name", &mut self.subscription_name);
                form_row(ui, "HTTPS URL", &mut self.subscription_url);
                if ui.button("Add and update").clicked() {
                    match self.data.add_subscription(&self.subscription_name, &self.subscription_url) {
                        Ok(id) => {
                            self.add_subscription = false;
                            self.subscription_name.clear();
                            self.subscription_url.clear();
                            self.save();
                            self.fetch_subscription(id, ctx);
                        }
                        Err(error) => self.notice = error,
                    }
                }
            });
        }
        let mut refresh = None;
        let mut remove_sub = None;
        for subscription in &self.data.subscriptions {
            ui.horizontal_wrapped(|ui| {
                ui.label(RichText::new(&subscription.name).strong().color(INK));
                ui.label(RichText::new(
                    subscription.last_error.as_deref().unwrap_or("Ready"))
                    .small().color(if subscription.last_error.is_some() { Color32::DARK_RED } else { MUTED }));
                if ui.small_button("Refresh").clicked() { refresh = Some(subscription.id); }
                if ui.small_button("Delete").clicked() { remove_sub = Some(subscription.id); }
            });
        }
        if let Some(id) = remove_sub { self.data.remove_subscription(id); self.save(); }
        if let Some(id) = refresh { self.fetch_subscription(id, ctx); }
    }

    pub(super) fn routing(&mut self, ui: &mut egui::Ui) {
        ui.heading(RichText::new("Routing").size(29.0).color(INK));
        ui.label(RichText::new("Choose how traffic should be handled after the Core is connected.").color(MUTED));
        ui.add_space(18.0);
        section(ui, "Outbound policy");
        let mut changed = false;
        ui.horizontal_wrapped(|ui| {
            changed |= ui.selectable_value(&mut self.data.settings.route_mode, RouteMode::Direct, "Direct").changed();
            changed |= ui.selectable_value(&mut self.data.settings.route_mode, RouteMode::Global, "Global").changed();
            changed |= ui.selectable_value(&mut self.data.settings.route_mode, RouteMode::Rules, "Rule-based").changed();
        });
        ui.add_space(13.0);
        section(ui, "DNS");
        changed |= ui.text_edit_singleline(&mut self.data.settings.dns_server).changed();
        ui.label(RichText::new("Values are saved locally; no network configuration has been changed.").small().color(MUTED));
        if changed { self.save(); }
    }

    pub(super) fn settings(&mut self, ui: &mut egui::Ui) {
        ui.heading(RichText::new("Settings").size(29.0).color(INK));
        ui.label(RichText::new("Connection preferences and system integration").color(MUTED));
        ui.add_space(18.0);
        section(ui, "Traffic interception");
        let changed = ui.checkbox(&mut self.data.settings.tun_requested, "Request TUN mode").changed()
            | ui.checkbox(&mut self.data.settings.transparent_proxy_requested, "Request transparent proxy").changed()
            | ui.checkbox(&mut self.data.settings.system_proxy_requested, "Request system proxy").changed();
        if changed { self.save(); }
        ui.add_space(10.0);
        ui.group(|ui| {
            ui.label(RichText::new("Requires a privileged helper").strong().color(INK));
            ui.label("These switches store a preference, not an active system configuration.");
            ui.label("macOS / Linux / FreeBSD must use a trusted OS authorization prompt (sudo/doas/polkit) when a real TUN helper is implemented.");
            ui.label("No password is collected or stored by v2rayQ. Core and helper are not yet connected.");
        });
        ui.add_space(16.0);
        section(ui, "Local data");
        ui.label(format!("Configuration: {}", self.data_path.display()));
        ui.checkbox(&mut self.show_secrets, "Show advanced connection information");
        if self.show_secrets {
            ui.label("Share URLs, subscription tokens and node passwords are stored in the private redb database. Protect your user profile.");
        }
    }
}

