use std::{
    io::{self, Read},
    path::PathBuf,
    time::Duration,
};

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::configuration::{Configuration, MergeSummary};
use super::import::Import;
use super::manual::node_endpoint;
use super::runtime::{self, NetworkMode, Runtime};
use super::{ClientData, MAX_SUBSCRIPTION_BYTES, ManualNode, ProtocolKind, default_data_path};
use crate::model::{
    Hysteria2Ports, Protocol, RouteMode, RouteRule, RuleDirection, RuleFields, RuntimeSettings,
};

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
    InspectNetwork,
    SaveNodeGroup {
        id: Option<Uuid>,
        name: String,
    },
    RemoveNodeGroup {
        id: Uuid,
    },
    AssignNodeGroup {
        node_id: Uuid,
        group_id: Option<Uuid>,
    },
    ExportConfiguration,
    MergeConfiguration {
        content: String,
        #[serde(default)]
        use_preferences: bool,
        #[serde(default)]
        preview: bool,
    },
    PreviewImport {
        content: String,
    },
    ImportNodes {
        content: String,
        #[serde(default)]
        group_id: Option<Uuid>,
    },
    AddLink {
        link: String,
    },
    AddNode {
        node: ManualNode,
        #[serde(default)]
        group_id: Option<Uuid>,
    },
    UpdateNode {
        id: Uuid,
        name: String,
        host: String,
        port: String,
    },
    CopyNode {
        id: Uuid,
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
    UpdateSubscription {
        id: Uuid,
        name: String,
        url: Option<String>,
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
    SaveRouteRule {
        id: Option<Uuid>,
        rule: RuleFields,
    },
    RemoveRouteRule {
        id: Uuid,
    },
    MoveRouteRule {
        id: Uuid,
        direction: RuleDirection,
    },
    SaveTheme {
        theme: String,
    },
    SaveLanguage {
        language: String,
    },
    Status,
    SetNetwork {
        mode: NetworkMode,
        enabled: bool,
    },
}

#[derive(Default, Deserialize, Serialize)]
pub struct Profile {
    #[serde(default)]
    pub groups: Vec<super::GroupSummary>,
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
    #[serde(default)]
    pub group_id: Option<Uuid>,
}

#[derive(Deserialize, Serialize)]
pub struct SubscriptionSummary {
    pub id: Uuid,
    pub name: String,
    pub node_count: usize,
    pub last_updated_at: Option<i64>,
    pub last_error: Option<String>,
}

#[derive(Default, Deserialize, Serialize)]
pub struct Reply {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ip_info: Option<super::network::IpInfo>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub configuration: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub merge: Option<MergeSummary>,
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
            Command::InspectNetwork => Ok(Reply {
                ip_info: Some(super::network::inspect()?),
                ..Reply::default()
            }),
            Command::Profile => {
                let data = ClientData::load(&self.path).map_err(|error| error.to_string())?;
                Ok(Reply {
                    profile: Some(data.into()),
                    ..Reply::default()
                })
            }
            Command::ExportConfiguration => {
                let data = ClientData::load(&self.path).map_err(|error| error.to_string())?;
                Ok(Reply {
                    configuration: Some(Configuration::export(data)?),
                    ..Reply::default()
                })
            }
            Command::MergeConfiguration {
                content,
                use_preferences,
                preview,
            } => {
                let config = Configuration::parse(&content)?;
                if preview {
                    let mut data =
                        ClientData::load(&self.path).map_err(|error| error.to_string())?;
                    return Ok(Reply {
                        merge: Some(config.merge(&mut data, use_preferences)?),
                        ..Reply::default()
                    });
                }
                let (profile, summary) = self.update(|data| {
                    let summary = config.merge(data, use_preferences)?;
                    Ok((Profile::from_data(data), summary))
                })?;
                Ok(Reply {
                    profile: Some(profile),
                    merge: Some(summary),
                    message: Some("Configuration merged".into()),
                    ..Reply::default()
                })
            }
            Command::PreviewImport { content } => {
                let parsed = Import::parse(&content)?;
                let mut data = ClientData::load(&self.path).map_err(|error| error.to_string())?;
                Ok(Reply {
                    message: Some(parsed.apply(&mut data, true, None)?),
                    ..Reply::default()
                })
            }
            Command::ImportNodes { content, group_id } => {
                let parsed = Import::parse(&content)?;
                let (profile, message) = self.update(|data| {
                    let message = parsed.apply(data, false, group_id)?;
                    Ok((Profile::from_data(data), message))
                })?;
                Ok(Reply {
                    profile: Some(profile),
                    message: Some(message),
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
        let (url, revision) = self.update(|data| {
            let subscription = data
                .subscriptions
                .iter_mut()
                .find(|item| item.id == id)
                .ok_or("Subscription not found")?;
            subscription.refresh_revision = subscription
                .refresh_revision
                .checked_add(1)
                .ok_or("Subscription revision exhausted")?;
            Ok((subscription.url.to_owned(), subscription.refresh_revision))
        })?;
        let download = download_subscription(&url);
        let (profile, message) = self.update(|data| {
            let current = data
                .subscriptions
                .iter()
                .find(|item| item.id == id)
                .ok_or("Subscription was deleted during refresh")?;
            if current.refresh_revision != revision {
                return Err("A newer subscription refresh or edit replaced this result".into());
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
        Command::SaveNodeGroup { id, name } => {
            data.save_group(id, &name)?;
            Ok("Group saved")
        }
        Command::RemoveNodeGroup { id } => {
            data.remove_group(id)?;
            Ok("Group removed; nodes kept")
        }
        Command::AssignNodeGroup { node_id, group_id } => {
            data.assign_group(node_id, group_id)?;
            Ok("Node group updated")
        }
        Command::AddLink { link } => {
            data.add_share_link(&link)
                .map_err(|_| "Invalid or unsupported share link")?;
            Ok("Node added")
        }
        Command::AddNode { node, group_id } => {
            if group_id.is_some_and(|id| !data.groups.iter().any(|group| group.id == id)) {
                return Err("Group not found".into());
            }
            let node = node.create()?;
            let id = node.id;
            data.add_node(node);
            data.assign_group(id, group_id)?;
            Ok("Node added")
        }
        Command::UpdateNode {
            id,
            name,
            host,
            port,
        } => {
            let entry = data
                .nodes
                .iter_mut()
                .find(|item| item.node.id == id)
                .ok_or("Node not found")?;
            if entry.subscription_id.is_some() {
                return Err("Make a local copy before editing a subscription node".into());
            }
            let (name, endpoint) = node_endpoint(&name, &host, &port)?;
            if let Protocol::Hysteria2(options) = &mut entry.node.protocol {
                match &mut options.ports {
                    Hysteria2Ports::Single(port) => *port = endpoint.port,
                    Hysteria2Ports::Multiple(_) if endpoint.port != entry.node.endpoint.port => {
                        return Err(
                            "Keep the original port for a Hysteria2 node with port hopping".into(),
                        );
                    }
                    Hysteria2Ports::Multiple(_) => {}
                }
            }
            entry.node.name = name;
            entry.node.endpoint = endpoint;
            Ok("Node updated")
        }
        Command::CopyNode { id } => {
            let original = data
                .nodes
                .iter()
                .find(|item| item.node.id == id)
                .ok_or("Node not found")?;
            let mut node = original.node.to_owned();
            node.id = Uuid::new_v4();
            let name = node.name.trim();
            let end = name.floor_char_boundary(248);
            node.name = format!("Copy of {}", &name[..end]);
            data.add_node(node);
            Ok("Local copy created")
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
        Command::UpdateSubscription { id, name, url } => {
            data.update_subscription(id, &name, url.as_deref())?;
            Ok("Subscription saved")
        }
        Command::RemoveSubscription { id } => {
            data.remove_subscription(id);
            Ok("Subscription deleted")
        }
        Command::SaveTheme { theme } => {
            if !matches!(theme.as_str(), "system" | "light" | "dark") {
                return Err("Unsupported theme".into());
            }
            data.settings.theme = theme;
            Ok("Theme saved")
        }
        Command::SaveLanguage { language } => {
            if !language.is_empty()
                && !super::language::RESOURCES
                    .languages()
                    .iter()
                    .any(|item| item.code == language)
            {
                return Err("Unsupported language".into());
            }
            data.settings.language = language;
            Ok("Language saved")
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
        Command::SaveRouteRule { id, rule } => {
            let fields = rule.validate()?;
            let rules = &mut data.settings.route_rules;
            if let Some(id) = id {
                let index = rule_index(rules, id)?;
                rules[index].fields = fields;
            } else {
                if rules.len() >= 1000 {
                    return Err("A profile can contain up to 1,000 route rules".into());
                }
                rules.push(RouteRule {
                    id: Uuid::new_v4(),
                    fields,
                });
            }
            Ok("Route rule saved")
        }
        Command::RemoveRouteRule { id } => {
            let rules = &mut data.settings.route_rules;
            let index = rule_index(rules, id)?;
            rules.remove(index);
            Ok("Route rule deleted")
        }
        Command::MoveRouteRule { id, direction } => {
            let rules = &mut data.settings.route_rules;
            let index = rule_index(rules, id)?;
            let neighbor = match direction {
                RuleDirection::Up => index.checked_sub(1),
                RuleDirection::Down => (index + 1 < rules.len()).then_some(index + 1),
            };
            if let Some(neighbor) = neighbor {
                rules.swap(index, neighbor);
            }
            Ok("Route order saved")
        }
        _ => Err("Unsupported profile operation".into()),
    }
}

fn rule_index(rules: &[RouteRule], id: Uuid) -> Result<usize, &'static str> {
    rules
        .iter()
        .position(|rule| rule.id == id)
        .ok_or("Route rule not found")
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
        let mut counts = std::collections::HashMap::new();
        for id in data.nodes.iter().filter_map(|node| node.subscription_id) {
            *counts.entry(id).or_insert(0) += 1;
        }
        let mut group_counts = std::collections::HashMap::new();
        for id in data.memberships.values() {
            *group_counts.entry(*id).or_insert(0) += 1;
        }
        Self {
            groups: data
                .groups
                .iter()
                .map(|group| super::GroupSummary {
                    id: group.id,
                    name: group.name.to_owned(),
                    node_count: group_counts.get(&group.id).copied().unwrap_or_default(),
                })
                .collect(),
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
                    group_id: data.memberships.get(&entry.node.id).copied(),
                })
                .collect(),
            subscriptions: data
                .subscriptions
                .iter()
                .map(|entry| SubscriptionSummary {
                    id: entry.id,
                    name: entry.name.to_owned(),
                    node_count: counts.get(&entry.id).copied().unwrap_or_default(),
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn copying_a_subscription_node_preserves_options_and_survives_refresh() -> Result<(), String> {
        let mut data = ClientData::default();
        let subscription = data.add_subscription("Test", "https://example.org/sub")?;
        let share = "trojan://secret@example.org:443?type=ws&path=%2Fsocket&security=tls#Test";
        data.apply_subscription(subscription, share)?;
        let id = data
            .selected_node
            .ok_or("The imported node should be selected")?;
        let edit = |id| Command::UpdateNode {
            id,
            name: "Updated".into(),
            host: "proxy.example.org".into(),
            port: "8443".into(),
        };
        assert!(edit_profile(&mut data, edit(id)).is_err());
        edit_profile(&mut data, Command::CopyNode { id })?;
        let copy_id = data.selected_node.ok_or("The copy should be selected")?;
        assert_ne!(id, copy_id);
        edit_profile(&mut data, edit(copy_id))?;
        data.apply_subscription(subscription, share)?;
        let local = data
            .nodes
            .iter()
            .find(|item| item.node.id == copy_id)
            .ok_or("Missing local copy")?;
        let subscribed = data
            .nodes
            .iter()
            .find(|item| item.subscription_id == Some(subscription))
            .ok_or("Missing subscription node")?;
        assert_eq!(local.node.name, "Updated");
        assert_eq!(local.node.endpoint.host, "proxy.example.org");
        assert_eq!(local.node.protocol, subscribed.node.protocol);
        assert_eq!(data.selected_node, Some(copy_id));
        Ok(())
    }

    #[test]
    fn editing_a_single_port_hysteria_node_updates_both_port_fields() -> Result<(), String> {
        let mut data = ClientData::default();
        data.add_share_link("hy2://secret@example.org:443/#Example")?;
        let id = data
            .selected_node
            .ok_or("The imported node should be selected")?;
        edit_profile(
            &mut data,
            Command::UpdateNode {
                id,
                name: "Example".into(),
                host: "example.org".into(),
                port: "8443".into(),
            },
        )?;
        let node = &data.nodes.first().ok_or("Missing node")?.node;
        let Protocol::Hysteria2(config) = &node.protocol else {
            return Err("Expected a Hysteria2 node".into());
        };
        assert_eq!(node.endpoint.port, 8443);
        assert_eq!(config.ports, Hysteria2Ports::Single(8443));
        Ok(())
    }
}
