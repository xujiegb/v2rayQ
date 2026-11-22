//! Client profile storage and operations shared by every frontend.
mod configuration;
mod groups;
mod import;
pub mod language;
mod manual;
pub mod network;
pub mod runtime;
mod service;

pub use configuration::MergeSummary;
pub use groups::{GroupSummary, NodeGroup};
pub use manual::{ManualNode, ManualTransport, ProtocolKind};
pub use service::{Access, ClientService, Command, NodeSummary, Profile, Reply};
use std::{
    fs, io,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    model::{Node, RuntimeSettings, Subscription},
    parser,
};

const MAX_SUBSCRIPTION_BYTES: usize = 4 * 1024 * 1024;
const MAX_NODES: usize = 10_000;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SavedNode {
    pub node: Node,
    pub subscription_id: Option<Uuid>,
}

#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct ClientData {
    pub groups: Vec<NodeGroup>,
    pub memberships: std::collections::BTreeMap<Uuid, Uuid>,
    pub nodes: Vec<SavedNode>,
    pub subscriptions: Vec<Subscription>,
    pub selected_node: Option<Uuid>,
    pub settings: RuntimeSettings,
}

impl ClientData {
    const KEY: &str = "client/state";

    pub fn load(path: &Path) -> io::Result<Self> {
        let database = v2rayq_storage::Storage::open(path).map_err(io::Error::other)?;
        if let Some(data) = database.get(Self::KEY).map_err(io::Error::other)? {
            return Ok(data);
        }
        let data = Self::load_legacy(path)?;
        if let Some(data) = data {
            database.put(Self::KEY, &data).map_err(io::Error::other)?;
            return Ok(data);
        }
        Ok(Self::default())
    }

    fn load_legacy(path: &Path) -> io::Result<Option<Self>> {
        let old_path = path.with_extension("json");
        if old_path == path {
            return Ok(None);
        }
        match fs::read(old_path) {
            Ok(bytes) => serde_json::from_slice(&bytes)
                .map(Some)
                .map_err(io::Error::other),
            Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(None),
            Err(error) => Err(error),
        }
    }

    pub fn add_node(&mut self, node: Node) {
        self.selected_node = Some(node.id);
        self.nodes.push(SavedNode {
            node,
            subscription_id: None,
        });
    }

    pub fn add_share_link(&mut self, link: &str) -> Result<(), String> {
        let node = parser::parse(link).map_err(|error| error.to_string())?;
        self.add_node(node);
        Ok(())
    }

    pub fn remove_node(&mut self, id: Uuid) {
        self.nodes.retain(|entry| entry.node.id != id);
        self.memberships.remove(&id);
        if self.selected_node == Some(id) {
            self.selected_node = self.nodes.first().map(|entry| entry.node.id);
        }
    }

    pub fn add_subscription(&mut self, name: &str, link: &str) -> Result<Uuid, String> {
        let url = subscription_url(link)?;
        let name = subscription_name(name, &url)?;
        self.check_subscription_url(&url, None)?;
        let subscription = Subscription::new(name, url.as_str());
        let id = subscription.id;
        self.subscriptions.push(subscription);
        Ok(id)
    }

    pub fn update_subscription(
        &mut self,
        id: Uuid,
        name: &str,
        link: Option<&str>,
    ) -> Result<(), String> {
        let current = self
            .subscriptions
            .iter()
            .find(|entry| entry.id == id)
            .ok_or("Subscription not found")?;
        let url = subscription_url(
            link.filter(|link| !link.trim().is_empty())
                .unwrap_or(&current.url),
        )?;
        let name = subscription_name(name, &url)?;
        self.check_subscription_url(&url, Some(id))?;
        let entry = self
            .subscriptions
            .iter_mut()
            .find(|entry| entry.id == id)
            .ok_or("Subscription not found")?;
        if entry.url != url.as_str() {
            entry.refresh_revision = entry
                .refresh_revision
                .checked_add(1)
                .ok_or("Subscription revision exhausted")?;
            entry.url = url.into();
            entry.last_updated_at = None;
            entry.last_error = None;
        }
        entry.name = name;
        Ok(())
    }

    fn check_subscription_url(&self, url: &url::Url, except: Option<Uuid>) -> Result<(), String> {
        if self
            .subscriptions
            .iter()
            .any(|item| Some(item.id) != except && item.url == url.as_str())
        {
            return Err("Subscription already exists".into());
        }
        Ok(())
    }

    pub fn remove_subscription(&mut self, id: Uuid) {
        self.subscriptions.retain(|entry| entry.id != id);
        self.nodes.retain(|entry| entry.subscription_id != Some(id));
        self.prune_memberships();
        if self
            .selected_node
            .is_some_and(|selected| !self.nodes.iter().any(|entry| entry.node.id == selected))
        {
            self.selected_node = self.nodes.first().map(|entry| entry.node.id);
        }
    }

    /// Replace subscribed nodes only after successful parsing. Never delete old
    /// entries when a download or parse fails.
    pub fn apply_subscription(&mut self, id: Uuid, content: &str) -> Result<usize, String> {
        if !self.subscriptions.iter().any(|entry| entry.id == id) {
            return Err("Subscription not found".into());
        }
        let mut parsed = parse_subscription(content)?;
        let mut known = std::collections::HashMap::new();
        for entry in self
            .nodes
            .iter()
            .filter(|entry| entry.subscription_id == Some(id))
        {
            let key = import::identity(&entry.node);
            if self.selected_node == Some(entry.node.id) {
                known.insert(key, entry.node.id);
            } else {
                known.entry(key).or_insert(entry.node.id);
            }
        }
        for node in &mut parsed {
            if let Some(id) = known.get(&import::identity(node)) {
                node.id = *id;
            }
        }
        let count = parsed.len();
        self.nodes.retain(|entry| entry.subscription_id != Some(id));
        self.nodes.extend(parsed.into_iter().map(|node| SavedNode {
            node,
            subscription_id: Some(id),
        }));
        self.prune_memberships();
        if let Some(subscription) = self.subscriptions.iter_mut().find(|entry| entry.id == id) {
            subscription.last_updated_at = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .ok()
                .and_then(|duration| i64::try_from(duration.as_secs()).ok());
            subscription.last_error = None;
        }
        if self
            .selected_node
            .is_none_or(|selected| !self.nodes.iter().any(|entry| entry.node.id == selected))
        {
            self.selected_node = self.nodes.first().map(|entry| entry.node.id);
        }
        Ok(count)
    }

    pub fn subscription_error(&mut self, id: Uuid, message: String) {
        if let Some(subscription) = self.subscriptions.iter_mut().find(|entry| entry.id == id) {
            subscription.last_error = Some(message);
        }
    }
}

fn subscription_url(link: &str) -> Result<url::Url, String> {
    let link = link.trim();
    if link.len() > 4096 || link.chars().any(char::is_control) {
        return Err("Enter an HTTPS subscription address of up to 4,096 bytes".into());
    }
    let mut url = url::Url::parse(link).map_err(|_| "Invalid subscription address")?;
    if url.scheme() != "https"
        || url.host_str().is_none()
        || !url.username().is_empty()
        || url.password().is_some()
    {
        return Err("Subscription requires an HTTPS URL without user information".into());
    }
    url.set_fragment(None);
    Ok(url)
}

fn subscription_name(name: &str, url: &url::Url) -> Result<String, String> {
    let name = name.trim();
    if name.len() > 256 || name.chars().any(char::is_control) {
        return Err("Enter a subscription name of up to 256 bytes".into());
    }
    if name.is_empty() {
        return Ok(url.host_str().unwrap_or("Subscription").to_owned());
    }
    Ok(name.to_owned())
}

pub fn default_data_path() -> PathBuf {
    if let Some(path) = std::env::var_os("V2RAYQ_CLIENT_DB") {
        return PathBuf::from(path);
    }
    #[cfg(target_os = "windows")]
    if let Some(dir) = std::env::var_os("APPDATA") {
        return PathBuf::from(dir).join("v2rayQ").join("client.redb");
    }
    #[cfg(any(target_os = "macos", target_os = "ios"))]
    if let Some(home) = std::env::var_os("HOME") {
        return PathBuf::from(home)
            .join("Library")
            .join("Application Support")
            .join("v2rayQ")
            .join("client.redb");
    }
    if let Some(dir) = std::env::var_os("XDG_CONFIG_HOME") {
        return PathBuf::from(dir).join("v2rayQ").join("client.redb");
    }
    if let Some(home) = std::env::var_os("HOME") {
        return PathBuf::from(home)
            .join(".config")
            .join("v2rayQ")
            .join("client.redb");
    }
    PathBuf::from("v2rayQ-client.redb")
}

/// Handles newline-delimited share links or base64-wrapped link lists.
/// The existing protocol parsers are the sole authority for supported shares.
pub fn parse_subscription(input: &str) -> Result<Vec<Node>, String> {
    let mut parsed = import::Import::parse(input)?;
    parsed.deduplicate(std::iter::empty());
    Ok(parsed.nodes)
}

#[cfg(test)]
mod tests {
    use super::*;
    use base64::{Engine as _, engine::general_purpose::STANDARD};

    const SHARE: &str = "trojan://password@example.org:443?security=tls#Example";

    #[test]
    fn imports_plain_and_base64_subscriptions() {
        let plain = parse_subscription(SHARE);
        assert!(plain.as_ref().is_ok_and(|nodes| nodes.len() == 1));
        let encoded = STANDARD.encode(SHARE);
        let wrapped = parse_subscription(&encoded);
        assert!(wrapped.as_ref().is_ok_and(|nodes| nodes.len() == 1));
    }

    #[test]
    fn keeps_nodes_on_failed_refresh() {
        let mut state = ClientData::default();
        let id = state
            .add_subscription("Sample", "https://example.org/sub")
            .unwrap();
        assert!(state.apply_subscription(id, SHARE).is_ok());
        assert!(state.apply_subscription(id, "invalid").is_err());
        assert_eq!(state.nodes.len(), 1);
    }

    #[test]
    fn refresh_preserves_the_selected_endpoint() -> Result<(), String> {
        let mut state = ClientData::default();
        let id = state.add_subscription("Sample", "https://example.org/sub")?;
        state.apply_subscription(id, SHARE)?;
        let active = state.selected_node;
        assert!(active.is_some());
        let renamed = "trojan://password@example.org:443?security=tls#New%20name";
        state.apply_subscription(id, renamed)?;
        assert_eq!(state.selected_node, active);
        assert_eq!(state.nodes[0].node.name, "New name");
        Ok(())
    }

    #[test]
    fn refresh_reuses_all_node_ids_and_deduplicates_equivalent_links() -> Result<(), String> {
        let mut data = ClientData::default();
        let id = data.add_subscription("Example", "https://example.org/sub")?;
        let second = "trojan://other@example.org:8443?security=tls#Other";
        data.apply_subscription(id, &format!("{SHARE}\n{second}\n{second}"))?;
        let ids: Vec<_> = data.nodes.iter().map(|entry| entry.node.id).collect();
        assert_eq!(ids.len(), 2);
        data.apply_subscription(id, &format!("{second}\n{SHARE}"))?;
        let reversed: Vec<_> = data.nodes.iter().rev().map(|entry| entry.node.id).collect();
        assert_eq!(reversed, ids);
        Ok(())
    }

    #[test]
    fn deleting_a_subscription_only_removes_its_nodes() {
        let mut state = ClientData::default();
        let id = state
            .add_subscription("Sample", "https://example.org/sub")
            .unwrap();
        assert!(state.apply_subscription(id, SHARE).is_ok());
        state.add_share_link(SHARE).unwrap();
        state.remove_subscription(id);
        assert_eq!(state.nodes.len(), 1);
    }
}
