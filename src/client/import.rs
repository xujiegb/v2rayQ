use std::collections::HashSet;

use base64::{
    Engine as _,
    engine::general_purpose::{STANDARD, STANDARD_NO_PAD, URL_SAFE, URL_SAFE_NO_PAD},
};
use uuid::Uuid;

use super::{ClientData, MAX_NODES, MAX_SUBSCRIPTION_BYTES};
use crate::{
    model::{Endpoint, Node, Protocol},
    parser,
};

#[derive(Default)]
pub(super) struct Import {
    pub nodes: Vec<Node>,
    skipped: usize,
}

type Identity<'a> = (&'a Endpoint, &'a Protocol);

pub(super) fn identity(node: &Node) -> Identity<'_> {
    (&node.endpoint, &node.protocol)
}

impl Import {
    pub fn parse(text: &str) -> Result<Self, String> {
        if text.len() > MAX_SUBSCRIPTION_BYTES {
            return Err("Input exceeds 4 MiB".into());
        }
        let text = text.trim().trim_start_matches('\u{feff}');
        let mut parsed = Self::read_lines(text)?;
        if parsed.nodes.is_empty() {
            let encoded: String = text
                .chars()
                .filter(|character| !character.is_ascii_whitespace())
                .collect();
            for engine in [STANDARD, STANDARD_NO_PAD, URL_SAFE, URL_SAFE_NO_PAD] {
                if let Ok(bytes) = engine.decode(&encoded)
                    && let Ok(decoded) = std::str::from_utf8(&bytes)
                {
                    parsed = Self::read_lines(decoded.trim_start_matches('\u{feff}'))?;
                    if !parsed.nodes.is_empty() {
                        break;
                    }
                }
            }
        }
        if parsed.nodes.is_empty() {
            return Err("No supported share links found".into());
        }
        Ok(parsed)
    }

    fn read_lines(text: &str) -> Result<Self, String> {
        let mut parsed = Self::default();
        for line in text
            .lines()
            .map(str::trim)
            .filter(|line| !line.is_empty() && !line.starts_with('#'))
        {
            match parser::parse(line) {
                Ok(node) => {
                    if parsed.nodes.len() == MAX_NODES {
                        return Err("Too many nodes in one import".into());
                    }
                    parsed.nodes.push(node);
                }
                Err(_) => parsed.skipped += 1,
            }
        }
        Ok(parsed)
    }

    pub fn deduplicate<'a>(&mut self, existing: impl Iterator<Item = Identity<'a>>) -> usize {
        let keep: Vec<_> = {
            let existing: HashSet<_> = existing.collect();
            let mut seen = HashSet::new();
            self.nodes
                .iter()
                .map(|node| {
                    let key = identity(node);
                    !existing.contains(&key) && seen.insert(key)
                })
                .collect()
        };
        let count = self.nodes.len();
        let mut keep = keep.into_iter();
        self.nodes.retain(|_| keep.next() == Some(true));
        count - self.nodes.len()
    }

    pub fn apply(
        mut self,
        data: &mut ClientData,
        preview: bool,
        group_id: Option<Uuid>,
    ) -> Result<String, String> {
        if group_id.is_some_and(|id| !data.groups.iter().any(|group| group.id == id)) {
            return Err("Group not found".into());
        }
        let duplicates = self.deduplicate(
            data.nodes
                .iter()
                .filter(|entry| entry.subscription_id.is_none())
                .map(|entry| identity(&entry.node)),
        );
        let count = self.nodes.len();
        if !preview {
            let selection = data
                .selected_node
                .or_else(|| self.nodes.first().map(|node| node.id));
            for node in self.nodes {
                if let Some(group) = group_id {
                    data.memberships.insert(node.id, group);
                }
                data.add_node(node);
            }
            data.selected_node = selection;
        }
        let label = if preview {
            "Ready to import"
        } else {
            "Imported"
        };
        let nodes = if count == 1 { "node" } else { "nodes" };
        let duplicate_label = if duplicates == 1 {
            "duplicate"
        } else {
            "duplicates"
        };
        Ok(format!(
            "{label} {count} {nodes} · {duplicates} {duplicate_label} · {} skipped",
            self.skipped
        ))
    }
}
