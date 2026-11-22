use super::*;

impl ClientApp {
    pub(super) fn home(&mut self, ui: &mut egui::Ui) {
        heading(ui, "Overview");
        let selected = self
            .profile
            .nodes
            .iter()
            .find(|node| Some(node.id) == self.profile.selected_node);
        egui::Frame::new()
            .fill(INK)
            .corner_radius(14.0)
            .inner_margin(egui::Margin::same(22))
            .show(ui, |ui| {
                ui.set_min_width(ui.available_width());
                ui.label(RichText::new("Selected node").color(Color32::from_rgb(179, 195, 227)));
                ui.label(
                    RichText::new(selected.map_or("Choose a node", |node| &node.name))
                        .size(25.0)
                        .strong()
                        .color(Color32::WHITE),
                );
                if let Some(node) = selected {
                    ui.label(
                        RichText::new(format!(
                            "{}  ·  {}:{}",
                            node.protocol.label(),
                            node.host,
                            node.port
                        ))
                        .color(Color32::LIGHT_GRAY),
                    );
                }
            });
        ui.add_space(12.0);
        card(ui, |ui| {
            ui.horizontal_wrapped(|ui| {
                ui.label(format!("{} nodes", self.profile.nodes.len()));
                ui.label(format!(
                    "{} subscriptions",
                    self.profile.subscriptions.len()
                ));
                if ui.button("Manage nodes").clicked() {
                    self.page = Page::Nodes;
                }
                if ui.button("Reload profile").clicked() && self.pending == 0 {
                    self.send(Command::Profile);
                }
            });
        });
        card(ui, |ui| {
            ui.strong("Network services");
            ui.label(format!("TUN  ·  {}", service_label(&self.runtime.tun)));
            ui.label(format!(
                "Transparent proxy  ·  {}",
                service_label(&self.runtime.transparent)
            ));
        });
    }

    pub(super) fn nodes(&mut self, ui: &mut egui::Ui) {
        heading(ui, "Nodes");
        ui.horizontal_wrapped(|ui| {
            for (form, name) in [
                (NodeForm::Share, "Import link"),
                (NodeForm::Manual, "Manual node"),
                (NodeForm::Subscription, "Add subscription"),
            ] {
                if ui.selectable_label(self.form == form, name).clicked() {
                    self.endpoint_editor = None;
                    self.form = if self.form == form {
                        NodeForm::None
                    } else {
                        form
                    };
                }
            }
        });
        ui.add_space(8.0);
        ui.add_enabled_ui(self.pending == 0, |ui| {
            match self.form {
                NodeForm::Share => card(ui, |ui| {
                    form_row(ui, "Share URI", &mut self.share_link, true);
                    if ui.button("Import node").clicked() {
                        let link = self.share_link.to_owned();
                        self.send(Command::AddLink { link });
                    }
                }),
                NodeForm::Subscription => card(ui, |ui| {
                    form_row(ui, "Name", &mut self.subscription_name, false);
                    form_row(ui, "HTTPS URL", &mut self.subscription_url, true);
                    if ui.button("Add and refresh").clicked() {
                        let name = self.subscription_name.to_owned();
                        let url = self.subscription_url.to_owned();
                        self.send(Command::AddSubscription { name, url });
                    }
                }),
                NodeForm::Manual => card(ui, |ui| self.manual_node(ui)),
                NodeForm::None => {}
            }
            let mut action = self.endpoint_form(ui);
            card(ui, |ui| {
                ui.strong("Saved nodes");
                if self.profile.nodes.is_empty() {
                    ui.label("No nodes saved.");
                }
                const PAGE_SIZE: usize = 30;
                if ui
                    .add(
                        egui::TextEdit::singleline(&mut self.query)
                            .hint_text("Search name, address or protocol"),
                    )
                    .changed()
                {
                    self.node_page = 0;
                }
                let needle = self.query.trim().to_lowercase();
                let matches = |node: &&v2rayQ::client::NodeSummary| {
                    needle.is_empty()
                        || node.name.to_lowercase().contains(&needle)
                        || node.host.to_lowercase().contains(&needle)
                        || node.protocol.label().to_lowercase().contains(&needle)
                };
                let filtered = self.profile.nodes.iter().filter(matches);
                let count = self.profile.nodes.iter().filter(matches).count();
                self.node_page = self.node_page.min(count.saturating_sub(1) / PAGE_SIZE);
                if count == 0 && !needle.is_empty() {
                    ui.label("No matching nodes.");
                }
                for node in filtered.skip(self.node_page * PAGE_SIZE).take(PAGE_SIZE) {
                    ui.separator();
                    ui.horizontal_wrapped(|ui| {
                        if ui
                            .selectable_label(
                                Some(node.id) == self.profile.selected_node,
                                &node.name,
                            )
                            .clicked()
                        {
                            action = Some(Command::SelectNode { id: node.id });
                        }
                        ui.label(
                            RichText::new(format!(
                                "{}  ·  {}:{}",
                                node.protocol.label(),
                                node.host,
                                node.port
                            ))
                            .color(MUTED),
                        );
                        if node.subscription_id.is_some() {
                            if ui.small_button("Make local copy").clicked() {
                                action = Some(Command::CopyNode { id: node.id });
                            }
                        } else if ui.small_button("Edit endpoint").clicked() {
                            self.form = NodeForm::None;
                            self.endpoint_editor = Some(EndpointEditor {
                                id: node.id,
                                name: node.name.to_owned(),
                                host: node.host.to_owned(),
                                port: node.port.to_string(),
                            });
                        }
                        if ui.small_button("Delete").clicked() {
                            action = Some(Command::RemoveNode { id: node.id });
                            if self
                                .endpoint_editor
                                .as_ref()
                                .is_some_and(|editor| editor.id == node.id)
                            {
                                self.endpoint_editor = None;
                            }
                        }
                    });
                }
                if count > PAGE_SIZE {
                    ui.horizontal(|ui| {
                        if ui
                            .add_enabled(self.node_page > 0, egui::Button::new("Previous"))
                            .clicked()
                        {
                            self.node_page -= 1;
                        }
                        ui.label(format!(
                            "{} / {}",
                            self.node_page + 1,
                            count.div_ceil(PAGE_SIZE)
                        ));
                        if ui
                            .add_enabled(
                                self.node_page + 1 < count.div_ceil(PAGE_SIZE),
                                egui::Button::new("Next"),
                            )
                            .clicked()
                        {
                            self.node_page += 1;
                        }
                    });
                }
            });
            card(ui, |ui| {
                ui.strong("Subscriptions");
                if self.profile.subscriptions.is_empty() {
                    ui.label("No subscriptions saved.");
                }
                for (index, item) in self.profile.subscriptions.iter().enumerate() {
                    ui.separator();
                    ui.horizontal_wrapped(|ui| {
                        ui.label(&item.name);
                        if let Some(error) = &item.last_error {
                            ui.label(error);
                        }
                        if ui.small_button("Refresh").clicked() {
                            action = Some(Command::RefreshSubscription { id: item.id });
                        }
                        if self.confirm_subscription == Some(index) {
                            if ui.small_button("Delete subscription and nodes").clicked() {
                                action = Some(Command::RemoveSubscription { id: item.id });
                                self.confirm_subscription = None;
                            }
                            if ui.small_button("Cancel").clicked() {
                                self.confirm_subscription = None;
                            }
                        } else if ui.small_button("Delete").clicked() {
                            self.confirm_subscription = Some(index);
                        }
                    });
                }
            });
            if let Some(action) = action {
                self.send(action);
            }
        });
    }

    fn endpoint_form(&mut self, ui: &mut egui::Ui) -> Option<Command> {
        let editor = self.endpoint_editor.as_mut()?;
        let mut action = None;
        let mut cancel = false;
        card(ui, |ui| {
            ui.strong("Edit node endpoint");
            form_row(ui, "Name", &mut editor.name, false);
            form_row(ui, "Server", &mut editor.host, false);
            form_row(ui, "Port", &mut editor.port, false);
            ui.horizontal(|ui| {
                if ui.button("Save endpoint").clicked() {
                    action = Some(Command::UpdateNode {
                        id: editor.id,
                        name: editor.name.to_owned(),
                        host: editor.host.to_owned(),
                        port: editor.port.to_owned(),
                    });
                }
                cancel = ui.button("Cancel").clicked();
            });
        });
        if cancel {
            self.endpoint_editor = None;
        }
        action
    }

    fn manual_node(&mut self, ui: &mut egui::Ui) {
        let node = &mut self.manual;
        egui::ComboBox::from_label("Protocol")
            .selected_text(node.protocol.label())
            .show_ui(ui, |ui| {
                for protocol in ProtocolKind::ALL {
                    ui.selectable_value(&mut node.protocol, protocol, protocol.label());
                }
            });
        form_row(ui, "Name", &mut node.name, false);
        form_row(ui, "Server", &mut node.host, false);
        form_row(ui, "Port", &mut node.port, false);
        let credential = if node.protocol.uses_uuid() {
            "UUID"
        } else {
            "Password"
        };
        form_row(ui, credential, &mut node.credential, true);
        if node.protocol == ProtocolKind::Shadowsocks {
            form_row(ui, "Cipher", &mut node.method, false);
        } else {
            if node.protocol.uses_uuid() {
                ui.checkbox(&mut node.tls, "TLS");
            }
            if !node.protocol.uses_uuid() || node.tls {
                form_row(ui, "TLS server name", &mut node.server_name, false);
            }
        }
        if node.protocol != ProtocolKind::Shadowsocks {
            ui.collapsing("Advanced connection options", |ui| {
                if node.protocol.supports_transport() {
                    egui::ComboBox::from_label("Transport")
                        .selected_text(node.transport.label())
                        .show_ui(ui, |ui| {
                            for transport in ManualTransport::ALL {
                                ui.selectable_value(
                                    &mut node.transport,
                                    transport,
                                    transport.label(),
                                );
                            }
                        });
                    match node.transport {
                        ManualTransport::Tcp => {}
                        ManualTransport::Grpc => {
                            form_row(ui, "gRPC service name", &mut node.service_name, false)
                        }
                        _ => {
                            form_row(ui, "Transport path", &mut node.transport_path, false);
                            form_row(ui, "Transport host", &mut node.transport_host, false);
                        }
                    }
                }
                if !node.protocol.uses_uuid() || node.tls {
                    form_row(ui, "ALPN (comma separated)", &mut node.alpn, false);
                }
            });
        }
        if ui.button("Save node").clicked() {
            match self.manual.create() {
                Ok(_) => {
                    let node = std::mem::take(&mut self.manual);
                    self.send(Command::AddNode { node });
                }
                Err(error) => self.notice = error,
            }
        }
    }

    pub(super) fn settings(&mut self, ui: &mut egui::Ui) {
        heading(ui, "Settings");
        card(ui, |ui| {
            let mut change = None;
            for (mode, label, status) in [
                (NetworkMode::Tun, "TUN", &self.runtime.tun),
                (
                    NetworkMode::Transparent,
                    "Transparent proxy",
                    &self.runtime.transparent,
                ),
            ] {
                ui.horizontal_wrapped(|ui| {
                    ui.strong(label);
                    ui.label(service_label(status));
                    let enable = status.running != Some(true);
                    if ui
                        .add_enabled(
                            status.can_change && self.pending == 0,
                            egui::Button::new(if enable { "Start" } else { "Stop" }),
                        )
                        .clicked()
                    {
                        change = Some(Command::SetNetwork {
                            mode,
                            enabled: enable,
                        });
                    }
                });
            }
            if let Some(command) = change {
                self.send(command);
            }
            ui.add_space(5.0);
            ui.label(if cfg!(unix) { "Starting or stopping an installed service uses sudo. Graphical sessions need SUDO_ASKPASS." }
                else { "Starting or stopping an installed service requires an administrator session." });
            if ui
                .add_enabled(self.pending == 0, egui::Button::new("Refresh status"))
                .clicked()
            {
                self.send(Command::Status);
            }
        });
        card(ui, |ui| {
            ui.strong("Profile database");
            ui.label(self.data_path.display().to_string());
        });
    }
}
