use serde::{Deserialize, Serialize};

use super::RouteRule;

/// Desired state, not evidence that a system proxy or tunnel is running.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(default)]
pub struct RuntimeSettings {
    pub tun_requested: bool,
    pub transparent_proxy_requested: bool,
    pub system_proxy_requested: bool,
    pub route_mode: RouteMode,
    pub language: String,
    pub dns_server: String,
    pub route_rules: Vec<RouteRule>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum RouteMode {
    Direct,
    Global,
    #[default]
    Rules,
}
