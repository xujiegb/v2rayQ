//! Versioned client configuration exchanged by all client frontends.
use std::collections::{HashMap, HashSet};

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::{
    ClientData, MAX_NODES, MAX_SUBSCRIPTION_BYTES, SavedNode, import::identity,
    manual::node_endpoint, subscription_url,
};
use crate::model::{RouteRule, Subscription};

const FORMAT: &str = "v2rayq-client";
const VERSION: u32 = 1;

#[derive(Deserialize, Serialize)]
pub struct Configuration {
    format: String,
    version: u32,
    profile: ClientData,
}

#[derive(Default, Deserialize, Serialize)]
pub struct MergeSummary {
    pub nodes: usize,
    pub subscriptions: usize,
    pub rules: usize,
    pub duplicates: usize,
}

impl Configuration {
    pub fn export(mut profile: ClientData) -> Result<String, String> {
        for subscription in &mut profile.subscriptions {
            subscription.last_error = None;
            subscription.last_updated_at = None;
            subscription.refresh_revision = 0;
        }
        let configuration = Self {
            format: FORMAT.into(),
            version: VERSION,
            profile,
        };
        let contents = serde_json::to_string_pretty(&configuration)
            .map_err(|_| "Could not export configuration")?;
        if contents.len() > MAX_SUBSCRIPTION_BYTES {
            return Err("Configuration exceeds 4 MiB".into());
        }
        Ok(contents)
    }

    pub fn parse(contents: &str) -> Result<Self, String> {
        if contents.len() > MAX_SUBSCRIPTION_BYTES {
            return Err("Configuration exceeds 4 MiB".into());
        }
        let value: serde_json::Value =
            serde_json::from_str(contents.trim_start_matches('\u{feff}'))
                .map_err(|_| "Invalid configuration JSON")?;
        if value.get("format").and_then(|value| value.as_str()) != Some(FORMAT) {
            return Err("Select a v2rayQ client configuration".into());
        }
        if value.get("version").and_then(|value| value.as_u64()) != Some(u64::from(VERSION)) {
            return Err("Unsupported client configuration version".into());
        }
        let mut config: Self =
            serde_json::from_value(value).map_err(|_| "Invalid client configuration fields")?;
        config.validate()?;
        Ok(config)
    }

    fn validate(&mut self) -> Result<(), String> {
        let data = &mut self.profile;
        if data.nodes.len() > MAX_NODES
            || data.subscriptions.len() > 1000
            || data.settings.route_rules.len() > 1000
        {
            return Err("Configuration has too many records".into());
        }
        if data.settings.dns_server.len() > 1024
            || data.settings.dns_server.chars().any(char::is_control)
        {
            return Err("Invalid DNS resolver".into());
        }
        if !data.settings.language.is_empty()
            && !super::language::RESOURCES
                .languages()
                .iter()
                .any(|item| item.code == data.settings.language)
        {
            data.settings.language.clear();
        }
        if !matches!(
            data.settings.theme.as_str(),
            "" | "system" | "light" | "dark"
        ) {
            data.settings.theme.clear();
        }
        let mut groups = HashSet::new();
        if data.groups.len() > 128 {
            return Err("A profile can contain up to 128 groups".into());
        }
        for group in &data.groups {
            if group.id.is_nil()
                || !groups.insert(group.id)
                || group.name.trim().is_empty()
                || group.name.len() > 256
                || group.name.chars().any(char::is_control)
            {
                return Err("Invalid node group".into());
            }
        }
        let mut subscriptions = HashSet::new();
        for item in &mut data.subscriptions {
            if item.id.is_nil() || !subscriptions.insert(item.id) {
                return Err("Duplicate or invalid subscription ID".into());
            }
            let url = subscription_url(&item.url)?;
            item.name = super::subscription_name(&item.name, &url)?;
            item.url = url.into();
        }
        let mut ids = HashSet::new();
        for entry in &mut data.nodes {
            if entry.node.id.is_nil() || !ids.insert(entry.node.id) {
                return Err("Duplicate or invalid node ID".into());
            }
            if entry
                .subscription_id
                .is_some_and(|id| !subscriptions.contains(&id))
            {
                return Err("A node references a missing subscription".into());
            }
            let (name, endpoint) = node_endpoint(
                &entry.node.name,
                &entry.node.endpoint.host,
                &entry.node.endpoint.port.to_string(),
            )?;
            entry.node.name = name;
            entry.node.endpoint = endpoint;
        }
        if data.selected_node.is_some_and(|id| !ids.contains(&id)) {
            return Err("Selected node is missing".into());
        }
        if data
            .memberships
            .iter()
            .any(|(node, group)| !ids.contains(node) || !groups.contains(group))
        {
            return Err("Invalid node group reference".into());
        }
        ids.clear();
        for rule in &mut data.settings.route_rules {
            if rule.id.is_nil() || !ids.insert(rule.id) {
                return Err("Duplicate or invalid route rule ID".into());
            }
            rule.fields = std::mem::take(&mut rule.fields).validate()?;
        }
        Ok(())
    }

    pub fn merge(
        self,
        data: &mut ClientData,
        use_preferences: bool,
    ) -> Result<MergeSummary, String> {
        let mut summary = MergeSummary::default();
        let imported = self.profile;
        let mut subscriptions = HashMap::new();
        for item in imported.subscriptions {
            let old_id = item.id;
            let new_id = if let Some(existing) = data
                .subscriptions
                .iter()
                .find(|entry| entry.url == item.url)
            {
                summary.duplicates += 1;
                existing.id
            } else {
                if data.subscriptions.len() >= 1000 {
                    return Err("Configuration has too many records".into());
                }
                let mut added = Subscription::new(item.name, item.url);
                added.enabled = item.enabled;
                let id = added.id;
                data.subscriptions.push(added);
                summary.subscriptions += 1;
                id
            };
            subscriptions.insert(old_id, new_id);
        }
        let mut nodes = HashMap::new();
        let plans = {
            let mut known: HashMap<_, _> = data
                .nodes
                .iter()
                .map(|entry| {
                    let (endpoint, protocol) = identity(&entry.node);
                    ((entry.subscription_id, endpoint, protocol), entry.node.id)
                })
                .collect();
            let mut plans = Vec::with_capacity(imported.nodes.len());
            for entry in &imported.nodes {
                let subscription = entry
                    .subscription_id
                    .and_then(|id| subscriptions.get(&id).copied());
                let (endpoint, protocol) = identity(&entry.node);
                let key = (subscription, endpoint, protocol);
                let (id, added) = match known.get(&key) {
                    Some(id) => {
                        summary.duplicates += 1;
                        (*id, false)
                    }
                    None => {
                        if data.nodes.len() + summary.nodes >= MAX_NODES {
                            return Err("Configuration has too many records".into());
                        }
                        let id = Uuid::new_v4();
                        known.insert(key, id);
                        summary.nodes += 1;
                        (id, true)
                    }
                };
                nodes.insert(entry.node.id, id);
                plans.push((subscription, id, added));
            }
            plans
        };
        for (mut entry, (subscription_id, id, added)) in imported.nodes.into_iter().zip(plans) {
            if added {
                entry.node.id = id;
                data.nodes.push(SavedNode {
                    node: entry.node,
                    subscription_id,
                });
            }
        }
        let mut group_ids = HashMap::new();
        for group in imported.groups {
            let old_id = group.id;
            let id = if let Some(saved) = data
                .groups
                .iter()
                .find(|saved| saved.name.eq_ignore_ascii_case(&group.name))
            {
                saved.id
            } else {
                data.save_group(None, &group.name)?;
                data.groups.last().ok_or("Group not found")?.id
            };
            group_ids.insert(old_id, id);
        }
        for (node, group) in imported.memberships {
            if let (Some(node), Some(group)) = (nodes.get(&node), group_ids.get(&group)) {
                data.memberships.entry(*node).or_insert(*group);
            }
        }
        let mut settings = imported.settings;
        let rules = std::mem::take(&mut settings.route_rules);
        if use_preferences {
            settings.route_rules = std::mem::take(&mut data.settings.route_rules);
            data.settings = settings;
        }
        for rule in rules {
            if data
                .settings
                .route_rules
                .iter()
                .any(|saved| saved.fields == rule.fields)
            {
                summary.duplicates += 1;
            } else {
                if data.settings.route_rules.len() >= 1000 {
                    return Err("Configuration has too many records".into());
                }
                data.settings.route_rules.push(RouteRule {
                    id: Uuid::new_v4(),
                    fields: rule.fields,
                });
                summary.rules += 1;
            }
        }
        if data.selected_node.is_none() {
            data.selected_node = imported
                .selected_node
                .and_then(|id| nodes.get(&id).copied())
                .or_else(|| data.nodes.first().map(|entry| entry.node.id));
        }
        Ok(summary)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> Result<ClientData, String> {
        let mut data = ClientData::default();
        let subscription = data.add_subscription("Example", "https://example.org/sub")?;
        data.apply_subscription(
            subscription,
            "trojan://secret@example.org:443?security=tls#Example",
        )?;
        data.settings.dns_server = "9.9.9.9".into();
        Ok(data)
    }

    #[test]
    fn round_trip_merge_is_idempotent_and_preserves_references() -> Result<(), String> {
        let json = Configuration::export(fixture()?)?;
        let mut merged = ClientData::default();
        merged.settings.dns_server = "1.1.1.1".into();
        let first = Configuration::parse(&json)?.merge(&mut merged, false)?;
        assert_eq!((first.nodes, first.subscriptions), (1, 1));
        assert_eq!(merged.settings.dns_server, "1.1.1.1");
        let selected = merged.selected_node;
        assert!(selected.is_some());
        assert_eq!(
            merged.nodes[0].subscription_id,
            Some(merged.subscriptions[0].id)
        );
        let second = Configuration::parse(&json)?.merge(&mut merged, true)?;
        assert_eq!(
            (second.nodes, second.subscriptions, second.duplicates),
            (0, 0, 2)
        );
        assert_eq!(merged.settings.dns_server, "9.9.9.9");
        assert_eq!(merged.selected_node, selected);
        Ok(())
    }

    #[test]
    fn malformed_references_and_future_versions_are_rejected() -> Result<(), String> {
        let exported = Configuration::export(fixture()?)?;
        let mut json: serde_json::Value =
            serde_json::from_str(&exported).map_err(|error| error.to_string())?;
        json["version"] = serde_json::json!(2);
        assert!(Configuration::parse(&json.to_string()).is_err());
        json["version"] = serde_json::json!(1);
        json["profile"]["subscriptions"] = serde_json::json!([]);
        assert!(Configuration::parse(&json.to_string()).is_err());
        Ok(())
    }
}
