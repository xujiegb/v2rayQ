use super::*;

impl ClientApp {
    pub(super) fn nodes(&mut self, ui: &mut egui::Ui) {
        let tr = self.translator;
        heading(ui, tr.text("Nodes"));
        ui.horizontal_wrapped(|ui| {
            for (form, name) in [
                (NodeForm::Share, tr.text("Import links")),
                (NodeForm::Manual, tr.text("Manual node")),
                (NodeForm::Subscription, tr.text("Add subscription")),
                (NodeForm::Scan, tr.text("Scan QR code")),
            ] {
                if ui
                    .selectable_label(self.form == form, tr.text(name))
                    .clicked()
                {
                    self.endpoint_editor = None;
                    if self.subscription_edit.take().is_some() {
                        self.subscription_name.clear();
                        self.subscription_url.clear();
                    }
                    self.form = if self.form == form {
                        NodeForm::None
                    } else {
                        form
                    };
                }
            }
        });
        ui.add_space(8.0);
        self.group_controls(ui);
        ui.add_enabled_ui(self.pending == 0, |ui| {
            match self.form {
                NodeForm::Share => card(ui, |ui| {
                    ui.strong(tr.text("Import share links"));
                    ui.label(tr.text("Paste one link per line or a base64 list."));
                    ui.add(
                        egui::TextEdit::multiline(&mut self.share_link)
                            .password(true)
                            .desired_rows(6)
                            .desired_width(ui.available_width()),
                    );
                    ui.horizontal_wrapped(|ui| {
                        if ui.button(tr.text("Preview import")).clicked() {
                            let content = self.share_link.to_owned();
                            self.send(Command::PreviewImport { content });
                        }
                        if ui.button(tr.text("Import nodes")).clicked() {
                            let content = self.share_link.to_owned();
                            self.send(Command::ImportNodes {
                                content,
                                group_id: Uuid::parse_str(&self.group_filter).ok(),
                            });
                        }
                    });
                }),
                NodeForm::Subscription => card(ui, |ui| {
                    let editing = self.subscription_edit.is_some();
                    ui.strong(if editing {
                        tr.text("Edit subscription")
                    } else {
                        tr.text("New subscription")
                    });
                    form_row(
                        ui,
                        tr.text("Name (optional)"),
                        &mut self.subscription_name,
                        false,
                    );
                    let caption = if editing {
                        tr.text("New HTTPS URL (leave blank to keep current URL)")
                    } else {
                        tr.text("HTTPS URL")
                    };
                    form_row(ui, tr.text(caption), &mut self.subscription_url, true);
                    if ui
                        .button(if editing {
                            tr.text("Save subscription")
                        } else {
                            tr.text("Add and refresh")
                        })
                        .clicked()
                    {
                        let name = self.subscription_name.to_owned();
                        let url = self.subscription_url.to_owned();
                        let command = match self.subscription_edit {
                            Some(id) => Command::UpdateSubscription {
                                id,
                                name,
                                url: Some(url),
                            },
                            None => Command::AddSubscription { name, url },
                        };
                        self.send(command);
                    }
                }),
                NodeForm::Scan => self.scan_form(ui),
                NodeForm::Manual => card(ui, |ui| self.manual_node(ui)),
                NodeForm::None => {}
            }
            if self.form != NodeForm::None && ui.button(tr.text("Cancel")).clicked() {
                self.form = NodeForm::None;
                self.scan_job = None;
                self.subscription_edit = None;
            }
            let mut action = self.endpoint_form(ui);
            card(ui, |ui| {
                ui.strong(tr.text("Saved nodes"));
                if self.profile.nodes.is_empty() {
                    ui.label(tr.text("No nodes saved."));
                }
                const PAGE_SIZE: usize = 30;
                if ui
                    .add(
                        egui::TextEdit::singleline(&mut self.query)
                            .hint_text(tr.text("Search name, address or protocol")),
                    )
                    .changed()
                {
                    self.node_page = 0;
                }
                let needle = self.query.trim().to_lowercase();
                let group_filter = &self.group_filter;
                let matches = |node: &&v2rayQ::client::NodeSummary| {
                    groups::matches_filter(group_filter, node)
                        && (needle.is_empty()
                            || node.name.to_lowercase().contains(&needle)
                            || node.host.to_lowercase().contains(&needle)
                            || node.protocol.label().to_lowercase().contains(&needle))
                };
                let filtered = self.profile.nodes.iter().filter(matches);
                let count = self.profile.nodes.iter().filter(matches).count();
                self.node_page = self.node_page.min(count.saturating_sub(1) / PAGE_SIZE);
                if count == 0 && !needle.is_empty() {
                    ui.label(tr.text("No matching nodes."));
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
                        let mut group_id = node.group_id;
                        egui::ComboBox::from_id_salt(("node_group", node.id))
                            .selected_text(
                                self.profile
                                    .groups
                                    .iter()
                                    .find(|group| Some(group.id) == group_id)
                                    .map_or(tr.text("Default group"), |group| group.name.as_str()),
                            )
                            .show_ui(ui, |ui| {
                                ui.selectable_value(&mut group_id, None, tr.text("Default group"));
                                for group in &self.profile.groups {
                                    ui.selectable_value(&mut group_id, Some(group.id), &group.name);
                                }
                            });
                        if group_id != node.group_id {
                            action = Some(Command::AssignNodeGroup {
                                node_id: node.id,
                                group_id,
                            });
                        }
                        if node.subscription_id.is_some() {
                            if ui.small_button(tr.text("Make local copy")).clicked() {
                                action = Some(Command::CopyNode { id: node.id });
                            }
                        } else if ui.small_button(tr.text("Edit endpoint")).clicked() {
                            self.form = NodeForm::None;
                            self.endpoint_editor = Some(EndpointEditor {
                                id: node.id,
                                name: node.name.to_owned(),
                                host: node.host.to_owned(),
                                port: node.port.to_string(),
                            });
                        }
                        if ui.small_button(tr.text("Delete")).clicked() {
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
                            .add_enabled(self.node_page > 0, egui::Button::new(tr.text("Previous")))
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
                                egui::Button::new(tr.text("Next")),
                            )
                            .clicked()
                        {
                            self.node_page += 1;
                        }
                    });
                }
            });
            card(ui, |ui| {
                ui.strong(tr.text("Subscriptions"));
                if self.profile.subscriptions.is_empty() {
                    ui.label(tr.text("No subscriptions saved."));
                }
                for (index, item) in self.profile.subscriptions.iter().enumerate() {
                    ui.separator();
                    ui.horizontal_wrapped(|ui| {
                        ui.strong(&item.name);
                        ui.label(tr.message(&format!("{} nodes", item.node_count)));
                        if ui.small_button(tr.text("Edit subscription")).clicked() {
                            self.form = NodeForm::Subscription;
                            self.subscription_edit = Some(item.id);
                            self.subscription_name = item.name.to_owned();
                            self.subscription_url.clear();
                        }
                        if let Some(error) = &item.last_error {
                            ui.label(error);
                        }
                        if ui.small_button(tr.text("Refresh")).clicked() {
                            action = Some(Command::RefreshSubscription { id: item.id });
                        }
                        if self.confirm_subscription == Some(index) {
                            if ui
                                .small_button(tr.text("Delete subscription and nodes"))
                                .clicked()
                            {
                                action = Some(Command::RemoveSubscription { id: item.id });
                                self.confirm_subscription = None;
                            }
                            if ui.small_button(tr.text("Cancel")).clicked() {
                                self.confirm_subscription = None;
                            }
                        } else if ui.small_button(tr.text("Delete")).clicked() {
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
        let tr = self.translator;
        let editor = self.endpoint_editor.as_mut()?;
        let mut action = None;
        let mut cancel = false;
        card(ui, |ui| {
            ui.strong(tr.text("Edit node endpoint"));
            form_row(ui, tr.text("Name"), &mut editor.name, false);
            form_row(ui, tr.text("Server"), &mut editor.host, false);
            form_row(ui, tr.text("Port"), &mut editor.port, false);
            ui.horizontal(|ui| {
                if ui.button(tr.text("Save endpoint")).clicked() {
                    action = Some(Command::UpdateNode {
                        id: editor.id,
                        name: editor.name.to_owned(),
                        host: editor.host.to_owned(),
                        port: editor.port.to_owned(),
                    });
                }
                cancel = ui.button(tr.text("Cancel")).clicked();
            });
        });
        if cancel {
            self.endpoint_editor = None;
        }
        action
    }

    fn manual_node(&mut self, ui: &mut egui::Ui) {
        let tr = self.translator;
        let node = &mut self.manual;
        egui::ComboBox::from_label(tr.text("Protocol"))
            .selected_text(node.protocol.label())
            .show_ui(ui, |ui| {
                for protocol in ProtocolKind::ALL {
                    ui.selectable_value(&mut node.protocol, protocol, protocol.label());
                }
            });
        form_row(ui, tr.text("Name"), &mut node.name, false);
        form_row(ui, tr.text("Server"), &mut node.host, false);
        form_row(ui, tr.text("Port"), &mut node.port, false);
        let credential = if node.protocol.uses_uuid() {
            tr.text("UUID")
        } else {
            tr.text("Password")
        };
        form_row(ui, tr.text(credential), &mut node.credential, true);
        if node.protocol == ProtocolKind::Shadowsocks {
            form_row(ui, tr.text("Cipher"), &mut node.method, false);
        } else {
            if node.protocol.uses_uuid() {
                ui.checkbox(&mut node.tls, tr.text("TLS"));
            }
            if !node.protocol.uses_uuid() || node.tls {
                form_row(ui, tr.text("TLS server name"), &mut node.server_name, false);
            }
        }
        if node.protocol != ProtocolKind::Shadowsocks {
            ui.collapsing(tr.text("Advanced connection options"), |ui| {
                if node.protocol.supports_transport() {
                    egui::ComboBox::from_label(tr.text("Transport"))
                        .selected_text(tr.text(node.transport.label()))
                        .show_ui(ui, |ui| {
                            for transport in ManualTransport::ALL {
                                ui.selectable_value(
                                    &mut node.transport,
                                    transport,
                                    tr.text(transport.label()),
                                );
                            }
                        });
                    match node.transport {
                        ManualTransport::Tcp => {}
                        ManualTransport::Grpc => form_row(
                            ui,
                            tr.text("gRPC service name"),
                            &mut node.service_name,
                            false,
                        ),
                        _ => {
                            form_row(
                                ui,
                                tr.text("Transport path"),
                                &mut node.transport_path,
                                false,
                            );
                            form_row(
                                ui,
                                tr.text("Transport host"),
                                &mut node.transport_host,
                                false,
                            );
                        }
                    }
                }
                if !node.protocol.uses_uuid() || node.tls {
                    form_row(ui, tr.text("ALPN (comma separated)"), &mut node.alpn, false);
                }
            });
        }
        if ui.button(tr.text("Save node")).clicked() {
            match self.manual.create() {
                Ok(_) => {
                    let node = std::mem::take(&mut self.manual);
                    self.send(Command::AddNode {
                        node,
                        group_id: Uuid::parse_str(&self.group_filter).ok(),
                    });
                }
                Err(error) => self.notice = error,
            }
        }
    }
}
