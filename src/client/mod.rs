//! Client-side profile management. No daemon or privileged OS operations.
use std::{
    fs, io,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

use base64::{Engine as _, engine::general_purpose::{STANDARD, STANDARD_NO_PAD, URL_SAFE, URL_SAFE_NO_PAD}};
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
    pub nodes: Vec<SavedNode>,
    pub subscriptions: Vec<Subscription>,
    pub selected_node: Option<Uuid>,
    pub settings: RuntimeSettings,
}

impl ClientData {
    pub fn load(path: &Path) -> io::Result<Self> {
        match fs::read(path) {
            Ok(bytes) => serde_json::from_slice(&bytes)
                .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error)),
            Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(Self::default()),
            Err(error) => Err(error),
        }
    }

    pub fn save(&self, path: &Path) -> io::Result<()> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let bytes = serde_json::to_vec_pretty(self).map_err(io::Error::other)?;
        let tmp = path.with_extension("json.tmp");
        let mut file = fs::OpenOptions::new()
            .write(true).create(true).truncate(true).open(&tmp)?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            // Store proxy secrets in a file accessible only by this user.
            file.set_permissions(fs::Permissions::from_mode(0o600))?;
        }
        std::io::Write::write_all(&mut file, &bytes)?;
        file.sync_all()?;
        drop(file);
        #[cfg(not(windows))]
        fs::rename(&tmp, path)?;
        #[cfg(windows)]
        {
            // Windows cannot always atomically replace an existing file.
            if fs::rename(&tmp, path).is_err() {
                fs::copy(&tmp, path)?;
                fs::remove_file(&tmp)?;
            }
        }
        Ok(())
    }

    pub fn add_node(&mut self, node: Node) {
        self.selected_node = Some(node.id);
        self.nodes.push(SavedNode { node, subscription_id: None });
    }

    pub fn add_share_link(&mut self, link: &str) -> Result<(), String> {
        let node = parser::parse(link).map_err(|error| error.to_string())?;
        self.add_node(node);
        Ok(())
    }

    pub fn remove_node(&mut self, id: Uuid) {
        self.nodes.retain(|entry| entry.node.id != id);
        if self.selected_node == Some(id) {
            self.selected_node = self.nodes.first().map(|entry| entry.node.id);
        }
    }

    pub fn add_subscription(&mut self, name: &str, link: &str) -> Result<Uuid, String> {
        let url = url::Url::parse(link.trim()).map_err(|error| error.to_string())?;
        if url.scheme() != "https" || url.host_str().is_none()
            || !url.username().is_empty() || url.password().is_some() {
            return Err("Subscription requires an HTTPS URL without embedded credentials".into());
        }
        if name.trim().is_empty() {
            return Err("Subscription name cannot be empty".into());
        }
        if self.subscriptions.iter().any(|item| item.url == url.as_str()) {
            return Err("Subscription already exists".into());
        }
        let subscription = Subscription::new(name.trim(), url.as_str());
        let id = subscription.id;
        self.subscriptions.push(subscription);
        Ok(id)
    }

    pub fn remove_subscription(&mut self, id: Uuid) {
        self.subscriptions.retain(|entry| entry.id != id);
        self.nodes.retain(|entry| entry.subscription_id != Some(id));
        if self.selected_node.is_some_and(|selected|
            !self.nodes.iter().any(|entry| entry.node.id == selected)) {
            self.selected_node = self.nodes.first().map(|entry| entry.node.id);
        }
    }

    /// Replace subscribed nodes only after successful parsing. Never delete old
    /// entries when a download or parse fails.
    pub fn apply_subscription(&mut self, id: Uuid, content: &str) -> Result<usize, String> {
        if !self.subscriptions.iter().any(|entry| entry.id == id) {
            return Err("Subscription not found".into());
        }
        let parsed = parse_subscription(content)?;
        let count = parsed.len();
        self.nodes.retain(|entry| entry.subscription_id != Some(id));
        self.nodes.extend(parsed.into_iter().map(|node| SavedNode {
            node,
            subscription_id: Some(id),
        }));
        if let Some(subscription) = self.subscriptions.iter_mut().find(|entry| entry.id == id) {
            subscription.last_updated_at = SystemTime::now()
                .duration_since(UNIX_EPOCH).ok()
                .and_then(|duration| i64::try_from(duration.as_secs()).ok());
            subscription.last_error = None;
        }
        if self.selected_node.is_none_or(|selected|
            !self.nodes.iter().any(|entry| entry.node.id == selected)) {
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

pub fn default_data_path() -> PathBuf {
    #[cfg(target_os = "windows")]
    if let Some(dir) = std::env::var_os("APPDATA") {
        return PathBuf::from(dir).join("v2rayQ").join("client.json");
    }
    if let Some(dir) = std::env::var_os("XDG_CONFIG_HOME") {
        return PathBuf::from(dir).join("v2rayQ").join("client.json");
    }
    if let Some(home) = std::env::var_os("HOME") {
        return PathBuf::from(home).join(".config").join("v2rayQ").join("client.json");
    }
    PathBuf::from("v2rayQ-client.json")
}

/// Handles newline-delimited share links or base64-wrapped link lists.
/// The existing protocol parsers are the sole authority for supported shares.
pub fn parse_subscription(input: &str) -> Result<Vec<Node>, String> {
    if input.len() > MAX_SUBSCRIPTION_BYTES {
        return Err("Subscription exceeds 4 MiB".into());
    }
    let mut nodes = parse_lines(input);
    if nodes.is_empty() {
        // Subscription providers sometimes wrap base64 payloads at fixed columns.
        let encoded: String = input.chars().filter(|c| !c.is_ascii_whitespace()).collect();
        for engine in [STANDARD, STANDARD_NO_PAD, URL_SAFE, URL_SAFE_NO_PAD] {
            if let Ok(bytes) = engine.decode(encoded)
                && bytes.len() <= MAX_SUBSCRIPTION_BYTES
                && let Ok(decoded) = String::from_utf8(bytes)
            {
                nodes = parse_lines(&decoded);
                if !nodes.is_empty() {
                    break;
                }
            }
        }
    }
    if nodes.is_empty() {
        return Err("No supported share links found in subscription".into());
    }
    if nodes.len() > MAX_NODES {
        return Err("Subscription has too many nodes".into());
    }
    Ok(nodes)
}

fn parse_lines(input: &str) -> Vec<Node> {
    input.lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .filter_map(|line| parser::parse(line).ok())
        .take(MAX_NODES + 1)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

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
        let id = state.add_subscription("Sample", "https://example.org/sub").unwrap();
        assert!(state.apply_subscription(id, SHARE).is_ok());
        assert!(state.apply_subscription(id, "invalid").is_err());
        assert_eq!(state.nodes.len(), 1);
    }

    #[test]
    fn deleting_a_subscription_only_removes_its_nodes() {
        let mut state = ClientData::default();
        let id = state.add_subscription("Sample", "https://example.org/sub").unwrap();
        assert!(state.apply_subscription(id, SHARE).is_ok());
        state.add_share_link(SHARE).unwrap();
        state.remove_subscription(id);
        assert_eq!(state.nodes.len(), 1);
    }
}
