//! User-defined collections. Subscription ownership remains independent.
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use super::ClientData;

#[derive(Debug, Deserialize, Serialize)]
pub struct NodeGroup {
    pub id: Uuid,
    pub name: String,
}

#[derive(Deserialize, Serialize)]
pub struct GroupSummary {
    pub id: Uuid,
    pub name: String,
    pub node_count: usize,
}

impl ClientData {
    pub fn save_group(&mut self, id: Option<Uuid>, name: &str) -> Result<(), String> {
        let name = name.trim();
        if name.is_empty() || name.len() > 256 || name.chars().any(char::is_control) {
            return Err("Enter a group name of up to 256 bytes".into());
        }
        if self.groups.iter().any(|group| Some(group.id) != id && group.name.eq_ignore_ascii_case(name)) {
            return Err("A group with this name already exists".into());
        }
        if let Some(id) = id {
            let group = self.groups.iter_mut().find(|group| group.id == id).ok_or("Group not found")?;
            group.name = name.to_owned();
        } else {
            if self.groups.len() >= 128 {
                return Err("A profile can contain up to 128 groups".into());
            }
            self.groups.push(NodeGroup { id: Uuid::new_v4(), name: name.to_owned() });
        }
        Ok(())
    }

    pub fn remove_group(&mut self, id: Uuid) -> Result<(), String> {
        let index = self.groups.iter().position(|group| group.id == id).ok_or("Group not found")?;
        self.groups.remove(index);
        self.memberships.retain(|_, group| *group != id);
        Ok(())
    }

    pub fn assign_group(&mut self, node_id: Uuid, group_id: Option<Uuid>) -> Result<(), String> {
        if !self.nodes.iter().any(|entry| entry.node.id == node_id) {
            return Err("Node not found".into());
        }
        if let Some(id) = group_id {
            if !self.groups.iter().any(|group| group.id == id) {
                return Err("Group not found".into());
            }
            self.memberships.insert(node_id, id);
        } else {
            self.memberships.remove(&node_id);
        }
        Ok(())
    }

    pub(super) fn prune_memberships(&mut self) {
        let ids: std::collections::HashSet<_> = self.nodes.iter().map(|entry| entry.node.id).collect();
        self.memberships.retain(|node, _| ids.contains(node));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn grouping_preserves_subscription_ownership_and_refresh() -> Result<(), String> {
        let mut data = ClientData::default();
        let subscription = data.add_subscription("Subscription", "https://example.org/sub")?;
        let link = "trojan://secret@example.org:443#Example";
        data.apply_subscription(subscription, link)?;
        let node = data.selected_node.ok_or("Missing node")?;
        data.save_group(None, "Travel")?;
        let group = data.groups.first().ok_or("Missing group")?.id;
        data.assign_group(node, Some(group))?;
        data.apply_subscription(subscription, link)?;
        assert_eq!(data.memberships.get(&node), Some(&group));
        assert_eq!(data.nodes[0].subscription_id, Some(subscription));
        assert!(data.save_group(None, "travel").is_err());
        data.remove_group(group)?;
        assert!(data.memberships.is_empty());
        assert_eq!(data.nodes.len(), 1);
        data.remove_subscription(subscription);
        assert!(data.nodes.is_empty());
        Ok(())
    }
}
