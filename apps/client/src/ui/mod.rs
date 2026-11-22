use std::{
    path::PathBuf,
    sync::mpsc::{self, Receiver, Sender},
};

use eframe::egui::{self, Color32, RichText};
use uuid::Uuid;
use v2rayQ::{
    client::{
        Access, ClientService, Command, ManualNode, ManualTransport, Profile, ProtocolKind, Reply,
        default_data_path,
        runtime::{NetworkMode, Runtime, ServiceStatus},
    },
    model::{RouteMode, RuleDirection, RuleFields, RuleMatch, RuleTarget},
};
use v2rayq_contracts::CLIENT_NAV;

mod pages;
mod routes;
mod theme;
mod widgets;
use widgets::{card, form_row, heading, service_label};

const ACCENT: Color32 = Color32::from_rgb(80, 97, 210);
const INK: Color32 = Color32::from_rgb(25, 38, 65);
const MUTED: Color32 = Color32::from_rgb(81, 99, 126);
const BACKGROUND: Color32 = Color32::from_rgb(242, 246, 252);
const NAVIGATION: Color32 = Color32::from_rgb(230, 237, 247);

#[derive(Clone, Copy, PartialEq, Eq)]
enum Page {
    Home,
    Nodes,
    Routing,
    Settings,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum NodeForm {
    None,
    Share,
    Manual,
    Subscription,
}

enum Completion {
    NodeForm(NodeForm),
    Endpoint,
    Rule,
    Other,
}

struct EndpointEditor {
    id: Uuid,
    name: String,
    host: String,
    port: String,
}

#[derive(Default)]
struct RuleEditor {
    id: Option<Uuid>,
    fields: RuleFields,
}

pub struct ClientApp {
    page: Page,
    profile: Profile,
    runtime: Runtime,
    data_path: PathBuf,
    form: NodeForm,
    share_link: String,
    subscription_edit: Option<Uuid>,
    subscription_name: String,
    subscription_url: String,
    manual: ManualNode,
    query: String,
    node_page: usize,
    confirm_subscription: Option<usize>,
    routing: Option<(RouteMode, String)>,
    endpoint_editor: Option<EndpointEditor>,
    rule_editor: Option<RuleEditor>,
    notice: String,
    pending: usize,
    commands: Sender<Command>,
    replies: Receiver<(Completion, Result<Reply, String>)>,
}

impl ClientApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        Self::new_with_data_path(cc, default_data_path())
    }

    pub fn new_with_data_path(cc: &eframe::CreationContext<'_>, data_path: PathBuf) -> Self {
        let mut visuals = egui::Visuals::light();
        visuals.selection.bg_fill = Color32::from_rgb(216, 225, 254);
        visuals.selection.stroke.color = ACCENT;
        cc.egui_ctx.set_visuals(visuals);
        theme::install_font_fallback(&cc.egui_ctx);
        let (commands, requests) = mpsc::channel();
        let (responses, replies) = mpsc::channel();
        let ctx = cc.egui_ctx.clone();
        let service = ClientService::new(data_path.to_owned());
        std::thread::spawn(move || {
            while let Ok(command) = requests.recv() {
                let completion = match command {
                    Command::ImportNodes { .. } => Completion::NodeForm(NodeForm::Share),
                    Command::AddNode { .. } => Completion::NodeForm(NodeForm::Manual),
                    Command::AddSubscription { .. } | Command::UpdateSubscription { .. } => {
                        Completion::NodeForm(NodeForm::Subscription)
                    }
                    Command::UpdateNode { .. } => Completion::Endpoint,
                    Command::SaveRouteRule { .. } => Completion::Rule,
                    _ => Completion::Other,
                };
                if responses
                    .send((completion, service.call(command, Access::Desktop)))
                    .is_err()
                {
                    break;
                }
                ctx.request_repaint();
            }
        });
        let mut app = Self {
            page: Page::Home,
            profile: Profile::default(),
            runtime: Runtime::default(),
            data_path,
            form: NodeForm::None,
            share_link: String::new(),
            subscription_edit: None,
            subscription_name: String::new(),
            subscription_url: String::new(),
            manual: ManualNode::default(),
            query: String::new(),
            node_page: 0,
            confirm_subscription: None,
            routing: None,
            endpoint_editor: None,
            rule_editor: None,
            notice: String::new(),
            pending: 0,
            commands,
            replies,
        };
        app.send(Command::Profile);
        app.send(Command::Status);
        app
    }

    fn send(&mut self, command: Command) {
        if self.commands.send(command).is_ok() {
            self.pending += 1;
        } else {
            self.notice = "Client service stopped".into();
        }
    }

    fn receive(&mut self) {
        while let Ok((completion, result)) = self.replies.try_recv() {
            self.pending = self.pending.saturating_sub(1);
            match result {
                Ok(reply) => {
                    if let Some(data) = reply.profile {
                        self.routing.get_or_insert_with(|| {
                            (
                                data.settings.route_mode,
                                data.settings.dns_server.to_owned(),
                            )
                        });
                        self.profile = data;
                        self.confirm_subscription = None;
                    }
                    if let Some(runtime) = reply.runtime {
                        self.runtime = runtime;
                    }
                    match completion {
                        Completion::NodeForm(form) => {
                            if form == NodeForm::Share {
                                self.share_link.clear();
                            }
                            if form == NodeForm::Subscription {
                                self.subscription_edit = None;
                                self.subscription_name.clear();
                                self.subscription_url.clear();
                            }
                            if self.form == form {
                                self.form = NodeForm::None;
                            }
                        }
                        Completion::Endpoint => self.endpoint_editor = None,
                        Completion::Rule => self.rule_editor = None,
                        Completion::Other => {}
                    }
                    if let Some(message) = reply.message {
                        self.notice = message;
                    }
                }
                Err(error) => self.notice = error,
            }
        }
    }

    fn navigation(&mut self, root: &mut egui::Ui) {
        if root.available_width() < 710.0 {
            egui::Panel::top("mobile_navigation").show(root, |ui| {
                ui.horizontal_wrapped(|ui| self.nav_items(ui, false));
            });
        } else {
            egui::Panel::left("desktop_navigation")
                .exact_size(196.0)
                .resizable(false)
                .frame(
                    egui::Frame::new()
                        .fill(NAVIGATION)
                        .inner_margin(egui::Margin::same(18)),
                )
                .show(root, |ui| {
                    ui.add_space(10.0);
                    ui.label(RichText::new("◈  v2rayQ").size(25.0).strong().color(ACCENT));
                    ui.add_space(29.0);
                    self.nav_items(ui, true);
                });
        }
    }

    fn nav_items(&mut self, ui: &mut egui::Ui, vertical: bool) {
        for (page, item) in [Page::Home, Page::Nodes, Page::Routing, Page::Settings]
            .into_iter()
            .zip(CLIENT_NAV)
        {
            let label = if vertical {
                format!("{}   {}", item.icon, item.title)
            } else {
                item.title.into()
            };
            let button =
                egui::Button::new(RichText::new(label).size(15.0)).selected(self.page == page);
            let response = if vertical {
                ui.add_sized([160.0, 38.0], button)
            } else {
                ui.add(button)
            };
            if response.clicked() {
                self.page = page;
                if self.pending == 0 && matches!(page, Page::Home | Page::Settings) {
                    self.send(Command::Status);
                }
            }
        }
    }
}

impl eframe::App for ClientApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        self.receive();
        self.navigation(ui);
        let margin = if ui.available_width() < 520.0 { 14 } else { 26 };
        egui::CentralPanel::default()
            .frame(
                egui::Frame::new()
                    .fill(BACKGROUND)
                    .inner_margin(egui::Margin::same(margin)),
            )
            .show(ui, |ui| {
                egui::ScrollArea::vertical().show(ui, |ui| {
                    if self.pending != 0 {
                        ui.spinner();
                    }
                    if !self.notice.is_empty() {
                        ui.label(RichText::new(&self.notice).color(INK));
                    }
                    match self.page {
                        Page::Home => self.home(ui),
                        Page::Nodes => self.nodes(ui),
                        Page::Routing => self.routing(ui),
                        Page::Settings => self.settings(ui),
                    }
                });
            });
    }
}
