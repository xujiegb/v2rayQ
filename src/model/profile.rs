use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Profile {
    pub id: Uuid,
    pub name: String,
    pub active_node_id: Option<Uuid>,
}

impl Profile {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            id: Uuid::new_v4(),
            name: name.into(),
            active_node_id: None,
        }
    }

    pub fn select_node(&mut self, node_id: Uuid) {
        self.active_node_id = Some(node_id);
    }

    pub fn clear_node(&mut self) {
        self.active_node_id = None;
    }
}
