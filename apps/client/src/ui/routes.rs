use super::*;

impl ClientApp {
    pub(super) fn routing(&mut self, ui: &mut egui::Ui) {
        let tr = self.translator;
        heading(ui, tr.text("Routing"));
        ui.add_enabled_ui(self.pending == 0, |ui| {
            self.routing_defaults(ui);
            card(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.strong(tr.text("Route rules"));
                    if ui.button(tr.text("Add rule")).clicked() {
                        self.rule_editor = Some(RuleEditor::default());
                    }
                });
                ui.label(
                    tr.text("Order is top to bottom. Rules are saved locally for the future Core."),
                );
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
        let tr = self.translator;
        card(ui, |ui| {
            ui.strong(tr.text("Profile defaults"));
            let Some((mode, dns)) = self.routing.as_mut() else {
                return;
            };
            ui.horizontal_wrapped(|ui| {
                ui.selectable_value(mode, RouteMode::Direct, tr.text("Direct"));
                ui.selectable_value(mode, RouteMode::Global, tr.text("Global"));
                ui.selectable_value(mode, RouteMode::Rules, tr.text("Rule-based"));
            });
            form_row(ui, tr.text("DNS resolver"), dns, false);
            if ui.button(tr.text("Save routing")).clicked() {
                let command = Command::SaveRouting {
                    route_mode: *mode,
                    dns_server: dns.to_owned(),
                };
                self.send(command);
            }
        });
    }

    fn rule_form(&mut self, ui: &mut egui::Ui) -> Option<Command> {
        let tr = self.translator;
        let edit = self.rule_editor.as_mut()?;
        ui.separator();
        egui::ComboBox::from_label(tr.text("Match on"))
            .selected_text(tr.text(edit.fields.kind.label()))
            .show_ui(ui, |ui| {
                for item in RuleMatch::ALL {
                    ui.selectable_value(&mut edit.fields.kind, item, tr.text(item.label()));
                }
            });
        form_row(ui, tr.text("Value"), &mut edit.fields.value, false);
        egui::ComboBox::from_label(tr.text("Action"))
            .selected_text(tr.text(edit.fields.target.label()))
            .show_ui(ui, |ui| {
                for item in RuleTarget::ALL {
                    ui.selectable_value(&mut edit.fields.target, item, tr.text(item.label()));
                }
            });
        let mut command = None;
        let mut cancel = false;
        ui.horizontal(|ui| {
            if ui.button(tr.text("Save rule")).clicked() {
                command = Some(Command::SaveRouteRule {
                    id: edit.id,
                    rule: edit.fields.to_owned(),
                });
            }
            cancel = ui.button(tr.text("Cancel")).clicked();
        });
        if cancel {
            self.rule_editor = None;
        }
        command
    }

    fn rule_rows(&mut self, ui: &mut egui::Ui) -> Option<Command> {
        let tr = self.translator;
        let rules = &self.profile.settings.route_rules;
        let mut command = None;
        if rules.is_empty() {
            ui.label(tr.text("No route rules saved."));
        }
        for (index, rule) in rules.iter().enumerate() {
            ui.separator();
            ui.horizontal_wrapped(|ui| {
                ui.strong(format!("{} · {}", index + 1, rule.fields.value));
                ui.label(format!(
                    "{} → {}",
                    tr.text(rule.fields.kind.label()),
                    tr.text(rule.fields.target.label())
                ));
                if ui.small_button(tr.text("Edit rule")).clicked() {
                    self.rule_editor = Some(RuleEditor {
                        id: Some(rule.id),
                        fields: rule.fields.to_owned(),
                    });
                }
                for (title, direction, enabled) in [
                    (tr.text("Move up"), RuleDirection::Up, index > 0),
                    (
                        tr.text("Move down"),
                        RuleDirection::Down,
                        index + 1 < rules.len(),
                    ),
                ] {
                    if ui
                        .add_enabled(enabled, egui::Button::new(tr.text(title)).small())
                        .clicked()
                    {
                        command = Some(Command::MoveRouteRule {
                            id: rule.id,
                            direction,
                        });
                    }
                }
                if ui.small_button(tr.text("Delete rule")).clicked() {
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
