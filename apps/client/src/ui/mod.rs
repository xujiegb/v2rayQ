

use std::{
    io::Read,
    path::PathBuf,
    sync::mpsc::{self, Receiver, Sender},
};

use eframe::egui::{self, Color32, RichText, Vec2};
use uuid::Uuid;
use v2rayQ::{
    client::{ClientData, default_data_path},
    model::{
        AnyTls, Endpoint, Hysteria2, Hysteria2Ports, Node, Protocol, RouteMode,
        Shadowsocks, Tls, Trojan, Vless, Vmess,
    },
};

const ACCENT: Color32 = Color32::from_rgb(89, 104, 206);
const INK: Color32 = Color32::from_rgb(35, 43, 63);
const MUTED: Color32 = Color32::from_rgb(112, 123, 144);
const BACKGROUND: Color32 = Color32::from_rgb(246, 248, 252);

use v2rayq_contracts::CLIENT_NAV;
mod manual;
mod pages;
mod widgets;
mod theme;
use manual::ManualForm;
use widgets::{form_row, metric, protocol_name, section};
use theme::install_font_fallback;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Page {
    Home,
    Nodes,
    Routing,
    Settings,
}

struct Download {
    id: Uuid,
    outcome: Result<String, String>,
}

pub struct ClientApp {
    page: Page,
    data: ClientData,
    data_path: PathBuf,
    share_link: String,
    subscription_name: String,
    subscription_url: String,
    manual: ManualForm,
    add_manual: bool,
    add_link: bool,
    add_subscription: bool,
    show_secrets: bool,
    notice: String,
    downloads_tx: Sender<Download>,
    downloads_rx: Receiver<Download>,
}

impl ClientApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        Self::new_with_data_path(cc, default_data_path())
    }

    pub fn new_with_data_path(cc: &eframe::CreationContext<'_>, path: PathBuf) -> Self {
        cc.egui_ctx.set_visuals(egui::Visuals::light());
        install_font_fallback(&cc.egui_ctx);
        let (data, notice) = match ClientData::load(&path) {
            Ok(data) => (data, String::new()),
            Err(error) => (ClientData::default(), format!("Could not load saved settings: {error}")),
        };
        let (tx, rx) = mpsc::channel();
        Self {
            page: Page::Home, data, data_path: path, share_link: String::new(),
            subscription_name: String::new(), subscription_url: String::new(),
            manual: ManualForm { port: "443".to_owned(), method: "aes-128-gcm".to_owned(), ..Default::default() },
            add_manual: false, add_link: false, add_subscription: false,
            show_secrets: false, notice, downloads_tx: tx, downloads_rx: rx,
        }
    }

    fn save(&mut self) {
        if let Err(error) = self.data.save(&self.data_path) {
            self.notice = format!("Could not save settings: {error}");
        }
    }

    fn fetch_subscription(&mut self, id: Uuid, ctx: &egui::Context) {
        let Some(subscription) = self.data.subscriptions.iter().find(|entry| entry.id == id) else {
            return;
        };
        let url = subscription.url.to_owned();
        let tx = self.downloads_tx.clone();
        let ctx = ctx.clone();
        self.notice = format!("Updating {}…", subscription.name);
        std::thread::spawn(move || {
            let result = (|| -> Result<String, String> {
                let mut response = ureq::get(&url).call().map_err(|error| error.to_string())?;
                let mut content = String::new();
                response.body_mut().as_reader().take(4 * 1024 * 1024 + 1)
                    .read_to_string(&mut content).map_err(|error| error.to_string())?;
                if content.len() > 4 * 1024 * 1024 {
                    return Err("Subscription exceeds 4 MiB".into());
                }
                Ok(content)
            })();
            let _ = tx.send(Download { id, outcome: result });
            ctx.request_repaint();
        });
    }

    fn process_downloads(&mut self) {
        while let Ok(download) = self.downloads_rx.try_recv() {
            let result = download.outcome.and_then(|content|
                self.data.apply_subscription(download.id, &content)
            );
            match result {
                Ok(count) => self.notice = format!("Imported {count} nodes"),
                Err(error) => {
                    self.data.subscription_error(download.id, error.clone());
                    self.notice = error;
                }
            }
            self.save();
        }
    }

    fn side_navigation(&mut self, root_ui: &mut egui::Ui) {
        if root_ui.available_width() < 720.0 {
            egui::Panel::top("mobile_navigation").show(root_ui, |ui| {
                ui.horizontal_wrapped(|ui| {
                    for (page, item) in [Page::Home, Page::Nodes, Page::Routing, Page::Settings]
                        .into_iter().zip(CLIENT_NAV.iter()) {
                        if ui.selectable_label(self.page == page, item.title).clicked() {
                            self.page = page;
                        }
                    }
                });
            });
            return;
        }
        egui::Panel::left("navigation")
            .exact_size(208.0)
            .resizable(false)
            .frame(egui::Frame::new().fill(Color32::from_rgb(236, 240, 248))
                .inner_margin(egui::Margin::same(18)))
            .show(root_ui, |ui| {
                ui.add_space(10.0);
                ui.label(RichText::new("◈  v2rayQ").size(25.0).strong().color(ACCENT));
                ui.label(RichText::new("NETWORK CONTROL").small().color(MUTED));
                ui.add_space(36.0);
                for (page, item) in [Page::Home, Page::Nodes, Page::Routing, Page::Settings]
                    .into_iter().zip(CLIENT_NAV.iter()) {
                    let label = format!("{}   {}", item.icon, item.title);
                    self.nav_button(ui, page, &label);
                }
                ui.add_space(28.0);
                ui.separator();
                ui.add_space(12.0);
                ui.label(RichText::new("CORE STATUS").small().color(MUTED));
                ui.label(RichText::new("●  Not connected").color(Color32::from_rgb(178, 113, 45)));
                ui.add_space(4.0);
                ui.label(RichText::new("Profiles can be edited offline.").small().color(MUTED));
            });
    }

    fn nav_button(&mut self, ui: &mut egui::Ui, page: Page, label: &str) {
        let selected = self.page == page;
        if ui.add_sized([174.0, 38.0],
            egui::Button::new(RichText::new(label).size(15.0)
                .color(if selected { ACCENT } else { INK }))
                .selected(selected)).clicked() {
            self.page = page;
        }
        ui.add_space(5.0);
    }

}

impl eframe::App for ClientApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        self.process_downloads();
        self.side_navigation(ui);
        egui::CentralPanel::default().frame(
            egui::Frame::new().fill(BACKGROUND).inner_margin(egui::Margin::same(28))
        ).show(ui, |ui| {
            egui::ScrollArea::vertical().show(ui, |ui| {
                match self.page {
                    Page::Home => self.home(ui),
                    Page::Nodes => self.nodes(ui, &ctx),
                    Page::Routing => self.routing(ui),
                    Page::Settings => self.settings(ui),
                }
                if !self.notice.is_empty() {
                    ui.add_space(20.0);
                    ui.separator();
                    ui.label(RichText::new(&self.notice).color(ACCENT));
                }
            });
        });
    }
}

