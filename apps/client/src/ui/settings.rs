use std::{
    fs::{self, OpenOptions},
    io::{Read, Write},
};

use super::*;

impl ClientApp {
    pub(super) fn settings(&mut self, ui: &mut egui::Ui) {
        let tr = self.translator;
        heading(ui, tr.text("Settings"));
        ui.add_enabled_ui(self.pending == 0, |ui| {
            card(ui, |ui| {
                let mut language = tr.code().to_owned();
                let name = language::RESOURCES
                    .languages()
                    .iter()
                    .find(|item| item.code == language)
                    .map_or(language.as_str(), |item| item.name.as_str());
                egui::ComboBox::from_label(tr.text("Language"))
                    .selected_text(name)
                    .show_ui(ui, |ui| {
                        for item in language::RESOURCES.languages() {
                            ui.selectable_value(&mut language, item.code.to_owned(), &item.name);
                        }
                    });
                if language != tr.code() {
                    self.send(Command::SaveLanguage { language });
                }
            });
            self.configuration_settings(ui);
            card(ui, |ui| {
                let mut change = None;
                let services = [
                    (NetworkMode::Tun, "TUN", &self.runtime.tun),
                    (NetworkMode::Transparent, "Transparent proxy", &self.runtime.transparent),
                ];
                for (mode, label, status) in services {
                    ui.horizontal_wrapped(|ui| {
                        ui.strong(tr.text(label));
                        ui.label(tr.text(service_label(status)));
                        let enable = status.running != Some(true);
                        let title = tr.text(if enable { "Start" } else { "Stop" });
                        if ui.add_enabled(status.can_change, egui::Button::new(title)).clicked() {
                            change = Some(Command::SetNetwork { mode, enabled: enable });
                        }
                    });
                }
                if let Some(command) = change {
                    self.send(command);
                }
                let authorization = if cfg!(unix) {
                    "Starting or stopping an installed service uses sudo. Graphical sessions need SUDO_ASKPASS."
                } else {
                    "Starting or stopping an installed service requires an administrator session."
                };
                ui.label(tr.text(authorization));
                if ui.button(tr.text("Refresh status")).clicked() {
                    self.send(Command::Status);
                }
            });
            card(ui, |ui| {
                ui.strong(tr.text("Profile database"));
                ui.label(self.data_path.display().to_string());
            });
        });
    }

    fn configuration_settings(&mut self, ui: &mut egui::Ui) {
        let tr = self.translator;
        card(ui, |ui| {
            ui.strong(tr.text("Configuration export and merge"));
            ui.label(tr.text("The export includes node credentials and subscription URLs. Keep the file private."));
            form_row(
                ui,
                tr.text("Configuration file path"),
                &mut self.configuration_path,
                false,
            );
            ui.horizontal_wrapped(|ui| {
                if ui.button(tr.text("Export configuration")).clicked() {
                    self.send(Command::ExportConfiguration);
                }
                if ui.button(tr.text("Read configuration file")).clicked() {
                    match read_configuration(&self.configuration_path) {
                        Ok(contents) => self.configuration = contents,
                        Err(error) => self.notice = error,
                    }
                }
                if ui
                    .add_enabled(
                        !self.configuration.is_empty(),
                        egui::Button::new(tr.text("Save to file")),
                    )
                    .clicked()
                {
                    self.notice =
                        match write_configuration(&self.configuration_path, &self.configuration) {
                            Ok(()) => "Configuration file saved".into(),
                            Err(error) => error,
                        };
                }
                if ui
                    .add_enabled(
                        !self.configuration.is_empty(),
                        egui::Button::new(tr.text("Copy JSON")),
                    )
                    .clicked()
                {
                    ui.ctx().copy_text(self.configuration.to_owned());
                    self.notice = "Configuration copied".into();
                }
            });
            ui.label(tr.text("Configuration JSON"));
            ui.add(
                egui::TextEdit::multiline(&mut self.configuration)
                    .password(true)
                    .desired_rows(5)
                    .desired_width(ui.available_width()),
            );
            ui.checkbox(
                &mut self.use_preferences,
                tr.text("Use imported preferences"),
            );
            ui.horizontal_wrapped(|ui| {
                for (preview, text) in [(true, "Preview merge"), (false, "Merge configuration")] {
                    if ui.button(tr.text(text)).clicked() {
                        self.send(Command::MergeConfiguration {
                            content: self.configuration.to_owned(),
                            use_preferences: self.use_preferences,
                            preview,
                        });
                    }
                }
            });
        });
    }
}

fn read_configuration(path: &str) -> Result<String, String> {
    let mut contents = String::new();
    fs::File::open(path)
        .map_err(|_| "Could not read configuration file")?
        .take(4 * 1024 * 1024 + 1)
        .read_to_string(&mut contents)
        .map_err(|_| "Could not read configuration file")?;
    if contents.len() > 4 * 1024 * 1024 {
        return Err("Configuration exceeds 4 MiB".into());
    }
    Ok(contents)
}

fn write_configuration(path: &str, contents: &str) -> Result<(), String> {
    let mut options = OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let mut file = options.open(path).map_err(|error| {
        if error.kind() == std::io::ErrorKind::AlreadyExists {
            "The configuration file already exists"
        } else {
            "Could not write configuration file"
        }
    })?;
    if file
        .write_all(contents.as_bytes())
        .and_then(|()| file.sync_all())
        .is_err()
    {
        drop(file);
        let _ = fs::remove_file(path);
        return Err("Could not write configuration file".into());
    }
    Ok(())
}
