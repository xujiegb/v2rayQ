#![forbid(unsafe_code)]

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

fn main() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1100.0, 750.0])
            .with_min_inner_size([760.0, 520.0]),
        ..Default::default()
    };
    eframe::run_native("v2rayQ", options, Box::new(|cc| Ok(Box::new(ClientApp::new(cc)))))
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Page {
    Home,
    Nodes,
    Routing,
    Settings,
}

#[derive(Default)]
struct ManualForm {
    name: String,
    host: String,
    port: String,
    protocol: usize,
    credential: String,
    method: String,
    server_name: String,
}

impl ManualForm {
    fn create(&self) -> Result<Node, String> {
        let host = self.host.trim();
        let name = self.name.trim();
        if host.is_empty() || name.is_empty() {
            return Err("Please enter a node name and server address".into());
        }
        let port: u16 = self.port.trim().parse()
            .map_err(|_| "Port must be between 1 and 65535")?;
        if port == 0 {
            return Err("Port cannot be zero".into());
        }
        let credential = self.credential.trim();
        let tls = || Tls {
            server_name: (!self.server_name.trim().is_empty())
                .then(|| self.server_name.trim().to_owned()),
            insecure: false,
            alpn: Vec::new(),
            fingerprint: None,
            ech: None,
            reality: None,
        };
        let protocol = match self.protocol {
            0 => Protocol::Vless(Vless {
                uuid: Uuid::parse_str(credential).map_err(|error| error.to_string())?,
                flow: None,
                transport: None,
                tls: (!self.server_name.trim().is_empty()).then(tls),
            }),
            1 => Protocol::Vmess(Vmess {
                uuid: Uuid::parse_str(credential).map_err(|error| error.to_string())?,
                alter_id: 0,
                security: Some("auto".to_owned()),
                transport: None,
                tls: (!self.server_name.trim().is_empty()).then(tls),
            }),
            2 => Protocol::Trojan(Trojan {
                password: nonempty(credential)?,
                transport: None,
                tls: tls(),
            }),
            3 => Protocol::Shadowsocks(Shadowsocks {
                method: nonempty(self.method.trim())?,
                password: nonempty(credential)?,
                plugin: None,
            }),
            4 => Protocol::AnyTls(AnyTls {
                password: nonempty(credential)?,
                tls: tls(),
            }),
            _ => Protocol::Hysteria2(Hysteria2 {
                password: (!credential.is_empty()).then(|| credential.to_owned()),
                ports: Hysteria2Ports::Single(port),
                obfs: None,
                tls: tls(),
            }),
        };
        Ok(Node::new(name, Endpoint::new(host, port), protocol))
    }
}

fn nonempty(value: &str) -> Result<String, String> {
    if value.is_empty() { Err("Credential or method cannot be empty".into()) }
    else { Ok(value.to_owned()) }
}

struct Download {
    id: Uuid,
    outcome: Result<String, String>,
}

struct ClientApp {
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
    fn new(cc: &eframe::CreationContext<'_>) -> Self {
        install_font_fallback(&cc.egui_ctx);
        let path = default_data_path();
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

    fn side_navigation(&mut self, ctx: &egui::Context) {
        egui::SidePanel::left("navigation")
            .exact_width(208.0)
            .resizable(false)
            .frame(egui::Frame::new().fill(Color32::from_rgb(236, 240, 248))
                .inner_margin(egui::Margin::same(18)))
            .show(ctx, |ui| {
                ui.add_space(10.0);
                ui.label(RichText::new("◈  v2rayQ").size(25.0).strong().color(ACCENT));
                ui.label(RichText::new("NETWORK CONTROL").small().color(MUTED));
                ui.add_space(36.0);
                self.nav_button(ui, Page::Home, "⌂   Overview");
                self.nav_button(ui, Page::Nodes, "▦   Nodes");
                self.nav_button(ui, Page::Routing, "⇄   Routing");
                self.nav_button(ui, Page::Settings, "⚙   Settings");
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

    fn home(&mut self, ui: &mut egui::Ui) {
        ui.heading(RichText::new("Overview").size(29.0).color(INK));
        ui.label(RichText::new("Your connection at a glance").color(MUTED));
        ui.add_space(20.0);
        ui.columns(3, |columns| {
            metric(&mut columns[0], "ENGINE", "Not connected", "Waiting for Core");
            metric(&mut columns[1], "NODES", &self.data.nodes.len().to_string(), "Saved locally");
            metric(&mut columns[2], "SUBSCRIPTIONS", &self.data.subscriptions.len().to_string(), "HTTPS imports");
        });
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

    fn nodes(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        ui.horizontal(|ui| {
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
                ui.horizontal(|ui| {
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
                ui.horizontal(|ui| {
                    ui.add_sized([175.0, 22.0], egui::Label::new("UUID / password"));
                    ui.add_sized([340.0, 25.0], egui::TextEdit::singleline(&mut self.manual.credential).password(true));
                });
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
                ui.horizontal(|ui| {
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
        ui.horizontal(|ui| {
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
            ui.horizontal(|ui| {
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

    fn routing(&mut self, ui: &mut egui::Ui) {
        ui.heading(RichText::new("Routing").size(29.0).color(INK));
        ui.label(RichText::new("Choose how traffic should be handled after the Core is connected.").color(MUTED));
        ui.add_space(18.0);
        section(ui, "Outbound policy");
        let mut changed = false;
        ui.horizontal(|ui| {
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

    fn settings(&mut self, ui: &mut egui::Ui) {
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
            ui.label("Share URLs, subscription tokens and node passwords are stored in the local JSON file. Protect your user profile.");
        }
    }
}

impl eframe::App for ClientApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        ctx.set_visuals(egui::Visuals::light());
        self.process_downloads();
        self.side_navigation(ctx);
        egui::CentralPanel::default().frame(
            egui::Frame::new().fill(BACKGROUND).inner_margin(egui::Margin::same(28))
        ).show(ctx, |ui| {
            egui::ScrollArea::vertical().show(ui, |ui| {
                match self.page {
                    Page::Home => self.home(ui),
                    Page::Nodes => self.nodes(ui, ctx),
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

fn section(ui: &mut egui::Ui, title: &str) {
    ui.label(RichText::new(title).size(18.0).strong().color(INK));
    ui.add_space(7.0);
}

fn metric(ui: &mut egui::Ui, title: &str, value: &str, description: &str) {
    ui.group(|ui| {
        ui.set_min_size(Vec2::new(180.0, 110.0));
        ui.label(RichText::new(title).small().color(MUTED));
        ui.label(RichText::new(value).size(23.0).strong().color(INK));
        ui.label(RichText::new(description).small().color(MUTED));
    });
}

fn form_row(ui: &mut egui::Ui, label: &str, value: &mut String) {
    ui.horizontal(|ui| {
        ui.add_sized([175.0, 22.0], egui::Label::new(label));
        ui.add_sized([340.0, 25.0], egui::TextEdit::singleline(value));
    });
}

fn protocol_name(value: &Protocol) -> &'static str {
    match value {
        Protocol::Vless(_) => "VLESS",
        Protocol::Vmess(_) => "VMess",
        Protocol::Trojan(_) => "Trojan",
        Protocol::Shadowsocks(_) => "Shadowsocks",
        Protocol::AnyTls(_) => "AnyTLS",
        Protocol::Hysteria2(_) => "Hysteria2",
    }
}

fn install_font_fallback(ctx: &egui::Context) {
    let candidates = [
        "/System/Library/Fonts/PingFang.ttc",
        "/usr/share/fonts/opentype/noto/NotoSansCJK-Regular.ttc",
        "/usr/share/fonts/truetype/noto/NotoSansCJK-Regular.ttc",
        "C:\\Windows\\Fonts\\msyh.ttc",
    ];
    for path in candidates {
        if let Ok(bytes) = std::fs::read(path) {
            let mut fonts = egui::FontDefinitions::default();
            fonts.font_data.insert("system-cjk".into(), egui::FontData::from_owned(bytes).into());
            fonts.families.entry(egui::FontFamily::Proportional).or_default().push("system-cjk".into());
            ctx.set_fonts(fonts);
            return;
        }
    }
}
