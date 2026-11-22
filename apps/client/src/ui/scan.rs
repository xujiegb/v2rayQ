//! QR decoding stays in the native UI; decoded shares use the common importer.
use super::*;
use std::{fs::File, io::Read, path::PathBuf};

pub(super) enum ScanResult {
    Links(String),
    Subscription(String),
}

impl ClientApp {
    pub(super) fn scan_form(&mut self, ui: &mut egui::Ui) {
        let tr = self.translator;
        let mut source = None;
        card(ui, |ui| {
            ui.strong(tr.text("Scan QR code"));
            ui.label(tr.text("Drop a PNG or JPEG image here, or enter its file path."));
            form_row(ui, tr.text("Image path"), &mut self.scan_path, false);
            if ui
                .add_enabled(
                    self.scan_job.is_none(),
                    egui::Button::new(tr.text("Read QR image")),
                )
                .clicked()
            {
                source = Some(PathBuf::from(self.scan_path.trim()));
            }
            if self.scan_job.is_some() {
                ui.spinner();
            }
            ui.label(tr.text("Recognized links are previewed before import. Subscription URLs need confirmation."));
        });
        if self.scan_job.is_none() {
            let dropped = ui
                .ctx()
                .input_mut(|input| std::mem::take(&mut input.raw.dropped_files));
            if let Some(file) = dropped.into_iter().next() {
                source = Some(file.path().to_path_buf());
            }
        }
        if let Some(source) = source {
            let (sender, receiver) = mpsc::channel();
            let context = ui.ctx().clone();
            std::thread::spawn(move || {
                let result = read_file(source).and_then(|bytes| decode(&bytes));
                let _ = sender.send(result);
                context.request_repaint();
            });
            self.scan_job = Some(receiver);
        }
    }

    pub(super) fn receive_scan(&mut self, context: &egui::Context) {
        let result = match self.scan_job.as_ref().map(Receiver::try_recv) {
            Some(Ok(result)) => Some(result),
            Some(Err(mpsc::TryRecvError::Disconnected)) => Some(Err("QR reader stopped".into())),
            Some(Err(mpsc::TryRecvError::Empty)) => {
                context.request_repaint_after(std::time::Duration::from_millis(100));
                None
            }
            None => None,
        };
        if let Some(result) = result {
            self.scan_job = None;
            match result {
                Ok(ScanResult::Links(content)) => {
                    self.share_link = content;
                    self.form = NodeForm::Share;
                    self.send(Command::PreviewImport {
                        content: self.share_link.to_owned(),
                    });
                }
                Ok(ScanResult::Subscription(url)) => {
                    self.subscription_edit = None;
                    self.subscription_name.clear();
                    self.subscription_url = url;
                    self.form = NodeForm::Subscription;
                    self.notice = "QR subscription ready to review".into();
                }
                Err(error) => self.notice = error,
            }
        }
    }
}

fn read_file(path: PathBuf) -> Result<Vec<u8>, String> {
    let mut bytes = Vec::new();
    File::open(path)
        .map_err(|_| "Could not read QR image")?
        .take(8 * 1024 * 1024 + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| "Could not read QR image")?;
    Ok(bytes)
}

fn decode(bytes: &[u8]) -> Result<ScanResult, String> {
    let contents = v2rayq_qr::decode(bytes)?;
    let mut links = Vec::new();
    let mut subscription = None;
    for contents in contents {
        let contents = contents.trim();
        if v2rayQ::client::parse_subscription(contents).is_ok() {
            if !links.iter().any(|saved| saved == contents) {
                links.push(contents.to_owned());
            }
        } else if contents.starts_with("https://")
            && contents.len() <= 4096
            && !contents.chars().any(char::is_control)
        {
            if subscription.is_some() {
                return Err("Scan one subscription QR code at a time".into());
            }
            subscription = Some(contents.to_owned());
        }
    }
    if !links.is_empty() {
        return Ok(ScanResult::Links(links.join("\n")));
    }
    subscription
        .map(ScanResult::Subscription)
        .ok_or_else(|| "No supported QR code found".into())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn invalid_images_fail_without_panicking() {
        assert!(decode(b"not an image").is_err());
        assert!(decode(&[]).is_err());
    }
}
