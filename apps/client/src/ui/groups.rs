use super::*;

impl ClientApp {
    pub(super) fn group_controls(&mut self, ui: &mut egui::Ui) {
        let tr = self.translator;
        let mut action = None;
        card(ui, |ui| {
            ui.horizontal_wrapped(|ui| {
                ui.strong(tr.text("Node groups"));
                let mut name = tr.text("All nodes").to_owned();
                if self.group_filter == "local" {
                    name = tr.text("Local nodes").into();
                } else if self.group_filter == "subscriptions" {
                    name = tr.text("Subscriptions").into();
                } else if let Some(group) = self
                    .profile
                    .groups
                    .iter()
                    .find(|group| group.id.to_string() == self.group_filter)
                {
                    name = group.name.to_owned();
                } else if let Some(subscription) = self
                    .profile
                    .subscriptions
                    .iter()
                    .find(|item| format!("subscription:{}", item.id) == self.group_filter)
                {
                    name = subscription.name.to_owned();
                }
                let old = self.group_filter.to_owned();
                egui::ComboBox::from_id_salt("group_filter")
                    .selected_text(name)
                    .show_ui(ui, |ui| {
                        ui.selectable_value(
                            &mut self.group_filter,
                            String::new(),
                            tr.text("All nodes"),
                        );
                        ui.selectable_value(
                            &mut self.group_filter,
                            "local".into(),
                            tr.text("Local nodes"),
                        );
                        ui.selectable_value(
                            &mut self.group_filter,
                            "subscriptions".into(),
                            tr.text("Subscriptions"),
                        );
                        for group in &self.profile.groups {
                            ui.selectable_value(
                                &mut self.group_filter,
                                group.id.to_string(),
                                format!("{} · {}", group.name, group.node_count),
                            );
                        }
                        for item in &self.profile.subscriptions {
                            ui.selectable_value(
                                &mut self.group_filter,
                                format!("subscription:{}", item.id),
                                &item.name,
                            );
                        }
                    });
                if self.group_filter != old {
                    self.node_page = 0;
                }
            });
            ui.add_enabled_ui(self.pending == 0, |ui| {
                ui.horizontal_wrapped(|ui| {
                    ui.add(
                        egui::TextEdit::singleline(&mut self.group_name)
                            .hint_text(tr.text("Group name"))
                            .desired_width(180.0),
                    );
                    if ui
                        .button(tr.text(if self.group_edit.is_some() {
                            "Save group"
                        } else {
                            "Add group"
                        }))
                        .clicked()
                    {
                        action = Some(Command::SaveNodeGroup {
                            id: self.group_edit,
                            name: self.group_name.to_owned(),
                        });
                    }
                    if self.group_edit.is_some() && ui.button(tr.text("Cancel")).clicked() {
                        self.group_edit = None;
                        self.group_name.clear();
                    }
                });
                for group in &self.profile.groups {
                    ui.horizontal_wrapped(|ui| {
                        ui.label(&group.name);
                        ui.small(format!("{} {}", group.node_count, tr.text("Nodes")));
                        if ui.small_button(tr.text("Rename group")).clicked() {
                            self.group_edit = Some(group.id);
                            self.group_name = group.name.to_owned();
                        }
                        if ui
                            .small_button(tr.text("Remove group"))
                            .on_hover_text(tr.text("Removing a group keeps its nodes."))
                            .clicked()
                        {
                            action = Some(Command::RemoveNodeGroup { id: group.id });
                            if self.group_filter == group.id.to_string() {
                                self.group_filter.clear();
                            }
                            if self.group_edit == Some(group.id) {
                                self.group_edit = None;
                                self.group_name.clear();
                            }
                        }
                    });
                }
            });
        });
        if let Some(command) = action {
            self.send(command);
        }
    }
}

pub(super) fn matches_filter(filter: &str, node: &v2rayQ::client::NodeSummary) -> bool {
    match filter {
        "" => true,
        "local" => node.subscription_id.is_none(),
        "subscriptions" => node.subscription_id.is_some(),
        value => {
            Uuid::parse_str(value).is_ok_and(|id| node.group_id == Some(id))
                || value
                    .strip_prefix("subscription:")
                    .and_then(|value| Uuid::parse_str(value).ok())
                    .is_some_and(|id| node.subscription_id == Some(id))
        }
    }
}
