use super::*;

impl ClientApp {
    pub(super) fn routing(&mut self, ui: &mut egui::Ui) {
        heading(ui, "Routing");
        ui.add_enabled_ui(self.pending == 0, |ui| {
            self.routing_defaults(ui);
            card(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.strong("Route rules");
                    if ui.button("Add rule").clicked() {
                        self.rule_editor = Some(RuleEditor::default());
                    }
                });
                ui.label("Order is top to bottom. Rules are saved locally for the future Core.");
                if let Some(action) = self.rule_form(ui) {
                    self.send(action);
                }
                if let Some(action) = self.rule_rows(ui) {
                    self.send(action);
                }
            });
        });
    }

    fn routing_defaults(&mut self, ui: &mut egui::Ui) {
        card(ui, |ui| {
            ui.strong("Profile defaults");
            let Some((mode, dns)) = self.routing.as_mut() else {
                return;
            };
            ui.horizontal_wrapped(|ui| {
                ui.selectable_value(mode, RouteMode::Direct, "Direct");
                ui.selectable_value(mode, RouteMode::Global, "Global");
                ui.selectable_value(mode, RouteMode::Rules, "Rule-based");
            });
            form_row(ui, "DNS resolver", dns, false);
            if ui.button("Save routing").clicked() {
                let command = Command::SaveRouting {
                    route_mode: *mode,
                    dns_server: dns.to_owned(),
                };
                self.send(command);
            }
        });
    }

    fn rule_form(&mut self, ui: &mut egui::Ui) -> Option<Command> {
        let edit = self.rule_editor.as_mut()?;
        ui.separator();
        egui::ComboBox::from_label("Match on")
            .selected_text(edit.fields.kind.label())
            .show_ui(ui, |ui| {
                for item in RuleMatch::ALL {
                    ui.selectable_value(&mut edit.fields.kind, item, item.label());
                }
            });
        form_row(ui, "Value", &mut edit.fields.value, false);
        egui::ComboBox::from_label("Action")
            .selected_text(edit.fields.target.label())
            .show_ui(ui, |ui| {
                for item in RuleTarget::ALL {
                    ui.selectable_value(&mut edit.fields.target, item, item.label());
                }
            });
        let mut command = None;
        let mut cancel = false;
        ui.horizontal(|ui| {
            if ui.button("Save rule").clicked() {
                command = Some(Command::SaveRouteRule {
                    id: edit.id,
                    rule: edit.fields.to_owned(),
                });
            }
            cancel = ui.button("Cancel").clicked();
        });
        if cancel {
            self.rule_editor = None;
        }
        command
    }

    fn rule_rows(&mut self, ui: &mut egui::Ui) -> Option<Command> {
        let rules = &self.profile.settings.route_rules;
        let mut command = None;
        if rules.is_empty() {
            ui.label("No route rules saved.");
        }
        for (index, rule) in rules.iter().enumerate() {
            ui.separator();
            ui.horizontal_wrapped(|ui| {
                ui.strong(format!("{} · {}", index + 1, rule.fields.value));
                ui.label(format!(
                    "{} → {}",
                    rule.fields.kind.label(),
                    rule.fields.target.label()
                ));
                if ui.small_button("Edit rule").clicked() {
                    self.rule_editor = Some(RuleEditor {
                        id: Some(rule.id),
                        fields: rule.fields.to_owned(),
                    });
                }
                for (title, direction, enabled) in [
                    ("Move up", RuleDirection::Up, index > 0),
                    ("Move down", RuleDirection::Down, index + 1 < rules.len()),
                ] {
                    if ui
                        .add_enabled(enabled, egui::Button::new(title).small())
                        .clicked()
                    {
                        command = Some(Command::MoveRouteRule {
                            id: rule.id,
                            direction,
                        });
                    }
                }
                if ui.small_button("Delete rule").clicked() {
                    if self
                        .rule_editor
                        .as_ref()
                        .is_some_and(|editor| editor.id == Some(rule.id))
                    {
                        self.rule_editor = None;
                    }
                    command = Some(Command::RemoveRouteRule { id: rule.id });
                }
            });
        }
        command
    }
}
