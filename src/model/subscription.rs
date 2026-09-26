use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Subscription {
    pub id: Uuid,
    pub name: String,
    pub url: String,
    pub enabled: bool,
    pub last_updated_at: Option<i64>,
    pub last_error: Option<String>,
}

impl Subscription {
    pub fn new(name: impl Into<String>, url: impl Into<String>) -> Self {
        Self {
            id: Uuid::new_v4(),
            name: name.into(),
            url: url.into(),
            enabled: true,
            last_updated_at: None,
            last_error: None,
        }
    }
}
