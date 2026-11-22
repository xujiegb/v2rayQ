#![forbid(unsafe_code)]

// Shared, allocation-free UI/navigation and transport contracts.
// Presentation frameworks must remain outside this crate.

pub const API_VERSION: u16 = 1;
pub const SERVER_PORT: u16 = 16837;
pub const WEB_CLIENT_PORT: u16 = 16838;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Application {
    NativeClient,
    WebClient,
    Server,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NavItem {
    pub id: &'static str,
    pub title: &'static str,
    pub detail: &'static str,
    pub icon: &'static str,
}

pub const CLIENT_NAV: &[NavItem] = &[
    NavItem { id: "overview", title: "Overview", detail: "Connection information", icon: "⌂" },
    NavItem { id: "nodes", title: "Nodes", detail: "Nodes and subscriptions", icon: "◈" },
    NavItem { id: "routing", title: "Routing", detail: "Traffic routing rules", icon: "⇄" },
    NavItem { id: "settings", title: "Settings", detail: "Application settings", icon: "⚙" },
];

pub const SERVER_NAV: &[NavItem] = &[
    NavItem { id: "overview", title: "Overview", detail: "Server status", icon: "⌂" },
    NavItem { id: "inbounds", title: "Inbounds", detail: "Inbound listeners", icon: "⇥" },
    NavItem { id: "users", title: "Users", detail: "User accounts", icon: "♙" },
    NavItem { id: "outbounds", title: "Outbounds", detail: "Outbound destinations", icon: "⇤" },
    NavItem { id: "nodes", title: "Nodes", detail: "Node registry", icon: "◈" },
    NavItem { id: "services", title: "Services", detail: "Service lifecycle", icon: "▤" },
    NavItem { id: "tls", title: "TLS", detail: "TLS certificates", icon: "♢" },
    NavItem { id: "system", title: "System", detail: "System details", icon: "▧" },
    NavItem { id: "routes", title: "Routes", detail: "Route policies", icon: "⇄" },
    NavItem { id: "dns", title: "DNS", detail: "DNS resolvers", icon: "◎" },
    NavItem { id: "administrators", title: "Administrators", detail: "Panel administration", icon: "♙" },
    NavItem { id: "settings", title: "Settings", detail: "Panel preferences", icon: "⚙" },
];

pub trait CoreBackend {
    fn status(&self) -> CoreStatus;
    fn capabilities(&self) -> Capabilities;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CoreStatus {
    pub connected: bool,
    pub tun_active: bool,
    pub transparent_proxy_active: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Capabilities {
    pub api_version: u16,
    pub can_modify_network: bool,
    pub can_manage_server: bool,
    pub core_available: bool,
}

pub struct UnavailableCore;

impl CoreBackend for UnavailableCore {
    fn status(&self) -> CoreStatus {
        CoreStatus { connected: false, tun_active: false, transparent_proxy_active: false }
    }

    fn capabilities(&self) -> Capabilities {
        Capabilities {
            api_version: API_VERSION,
            can_modify_network: false,
            can_manage_server: false,
            core_available: false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn navigation_ids_are_unique() {
        for navigation in [CLIENT_NAV, SERVER_NAV] {
            let mut ids = HashSet::new();
            for item in navigation {
                assert!(!item.id.is_empty());
                assert!(!item.title.is_empty());
                assert!(ids.insert(item.id));
            }
        }
    }

    #[test]
    fn unavailable_core_never_exposes_privileges() {
        let api = UnavailableCore;
        assert!(!api.status().tun_active);
        assert!(!api.capabilities().can_manage_server);
    }
}
