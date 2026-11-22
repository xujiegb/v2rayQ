use std::{
    io::{self, Read},
    path::PathBuf,
    time::Duration,
};

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::runtime::{self, NetworkMode, Runtime};
use super::{ClientData, MAX_SUBSCRIPTION_BYTES, ManualNode, ProtocolKind, default_data_path};
use crate::model::{RouteMode, RuntimeSettings};

#[derive(Clone, Copy)]
pub enum Access {
    Terminal,
    Desktop,
    ProfileOnly,
    System,
}

#[derive(Deserialize, Serialize)]
#[serde(tag = "action", rename_all = "snake_case")]
pub enum Command {
    Profile,
    AddLink {
        link: String,
    },
    AddNode {
        node: ManualNode,
    },
    SelectNode {
        id: Uuid,
    },
    RemoveNode {
        id: Uuid,
    },
    AddSubscription {
        name: String,
        url: String,
    },
    RefreshSubscription {
        id: Uuid,
    },
    RemoveSubscription {
        id: Uuid,
    },
    SaveRouting {
        route_mode: RouteMode,
        dns_server: String,
    },
    Status,
    SetNetwork {
        mode: NetworkMode,
        enabled: bool,
    },
}

#[derive(Default, Deserialize, Serialize)]
pub struct Profile {
    pub nodes: Vec<NodeSummary>,
    pub subscriptions: Vec<SubscriptionSummary>,
    pub selected_node: Option<Uuid>,
    pub settings: RuntimeSettings,
}

#[derive(Deserialize, Serialize)]
pub struct NodeSummary {
    pub id: Uuid,
    pub name: String,
    pub host: String,
    pub port: u16,
    pub protocol: ProtocolKind,
    pub subscription_id: Option<Uuid>,
}

#[derive(Deserialize, Serialize)]
pub struct SubscriptionSummary {
    pub id: Uuid,
    pub name: String,
    pub last_updated_at: Option<i64>,
    pub last_error: Option<String>,
}

#[derive(Default, Deserialize, Serialize)]
pub struct Reply {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub profile: Option<Profile>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub runtime: Option<Runtime>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

pub struct ClientService {
    path: PathBuf,
}

impl Default for ClientService {
    fn default() -> Self {
        Self::new(default_data_path())
    }
}

impl ClientService {
    pub fn new(path: PathBuf) -> Self {
        Self { path }
    }

    pub fn call(&self, command: Command, access: Access) -> Result<Reply, String> {
        match command {
            Command::Status => Ok(Reply {
                runtime: Some(runtime::status(access)),
                ..Reply::default()
            }),
            Command::SetNetwork { mode, enabled } => {
                runtime::set(mode, enabled, access)?;
                Ok(Reply {
                    runtime: Some(runtime::status(access)),
                    ..Reply::default()
                })
            }
            _ if matches!(access, Access::System) => {
                Err("This entry point only controls services".into())
            }
            Command::Profile => {
                let data = ClientData::load(&self.path).map_err(|error| error.to_string())?;
                Ok(Reply {
                    profile: Some(data.into()),
                    ..Reply::default()
                })
            }
            Command::AddSubscription { name, url } => {
                let id = self.update(|data| data.add_subscription(&name, &url))?;
                self.refresh(id)
            }
            Command::RefreshSubscription { id } => self.refresh(id),
            action => {
                let (profile, message) = self.update(|data| {
                    let message = edit_profile(data, action)?;
                    Ok((Profile::from_data(data), message))
                })?;
                Ok(Reply {
                    profile: Some(profile),
                    message: Some(message.into()),
                    ..Reply::default()
                })
            }
        }
    }

    fn update<R>(
        &self,
        edit: impl FnOnce(&mut ClientData) -> Result<R, String>,
    ) -> Result<R, String> {
        let store = v2rayq_storage::Storage::open(&self.path).map_err(|error| error.to_string())?;
        store
            .update(ClientData::KEY, |previous| {
                let mut data = match previous {
                    Some(data) => data,
                    None => ClientData::load_legacy(&self.path)?.unwrap_or_default(),
                };
                let result = edit(&mut data).map_err(io::Error::other)?;
                Ok((data, result))
            })
            .map_err(|error| error.to_string())
    }

    fn refresh(&self, id: Uuid) -> Result<Reply, String> {
        let subscription = ClientData::load(&self.path)
            .map_err(|error| error.to_string())?
            .subscriptions
            .into_iter()
            .find(|item| item.id == id)
            .ok_or("Subscription not found")?;
        let download = download_subscription(&subscription.url);
        let (profile, message) = self.update(|data| {
            if !data.subscriptions.iter().any(|item| item.id == id) {
                return Err("Subscription was deleted during refresh".into());
            }
            let message = match download.and_then(|content| data.apply_subscription(id, &content)) {
                Ok(count) => format!("Imported {count} nodes"),
                Err(error) => {
                    data.subscription_error(id, error.to_owned());
                    format!("Subscription refresh failed: {error}")
                }
            };
            Ok((Profile::from_data(data), message))
        })?;
        Ok(Reply {
            profile: Some(profile),
            message: Some(message),
            ..Reply::default()
        })
    }
}

fn edit_profile(data: &mut ClientData, action: Command) -> Result<&'static str, String> {
    match action {
        Command::AddLink { link } => {
            data.add_share_link(&link)
                .map_err(|_| "Invalid or unsupported share link")?;
            Ok("Node added")
        }
        Command::AddNode { node } => {
            data.add_node(node.create()?);
            Ok("Node added")
        }
        Command::SelectNode { id } => {
            if !data.nodes.iter().any(|item| item.node.id == id) {
                return Err("Node not found".into());
            }
            data.selected_node = Some(id);
            Ok("Node selected")
        }
        Command::RemoveNode { id } => {
            data.remove_node(id);
            Ok("Node deleted")
        }
        Command::RemoveSubscription { id } => {
            data.remove_subscription(id);
            Ok("Subscription deleted")
        }
        Command::SaveRouting {
            route_mode,
            dns_server,
        } => {
            if dns_server.len() > 1024 || dns_server.chars().any(char::is_control) {
                return Err("Invalid DNS resolver".into());
            }
            data.settings.route_mode = route_mode;
            data.settings.dns_server = dns_server.trim().to_owned();
            Ok("Routing preferences saved")
        }
        _ => Err("Unsupported profile operation".into()),
    }
}

fn download_subscription(url: &str) -> Result<String, String> {
    let config = ureq::Agent::config_builder()
        .timeout_global(Some(Duration::from_secs(12)))
        .https_only(true)
        .max_redirects(4)
        .build();
    let agent: ureq::Agent = config.into();
    let mut response = agent
        .get(url)
        .call()
        .map_err(|_| "Unable to fetch HTTPS subscription")?;
    let mut content = String::new();
    response
        .body_mut()
        .as_reader()
        .take((MAX_SUBSCRIPTION_BYTES + 1) as u64)
        .read_to_string(&mut content)
        .map_err(|_| "Unable to read subscription as UTF-8")?;
    if content.len() > MAX_SUBSCRIPTION_BYTES {
        return Err("Subscription exceeds 4 MiB".into());
    }
    Ok(content)
}

impl From<ClientData> for Profile {
    fn from(data: ClientData) -> Self {
        Self::from_data(&data)
    }
}

impl Profile {
    fn from_data(data: &ClientData) -> Self {
        Self {
            nodes: data
                .nodes
                .iter()
                .map(|entry| NodeSummary {
                    id: entry.node.id,
                    name: entry.node.name.to_owned(),
                    host: entry.node.endpoint.host.to_owned(),
                    port: entry.node.endpoint.port,
                    protocol: ProtocolKind::of(&entry.node.protocol),
                    subscription_id: entry.subscription_id,
                })
                .collect(),
            subscriptions: data
                .subscriptions
                .iter()
                .map(|entry| SubscriptionSummary {
                    id: entry.id,
                    name: entry.name.to_owned(),
                    last_updated_at: entry.last_updated_at,
                    last_error: entry
                        .last_error
                        .as_ref()
                        .map(|_| "Last refresh failed".into()),
                })
                .collect(),
            selected_node: data.selected_node,
            settings: data.settings.to_owned(),
        }
    }
}
