use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Node {
    pub id: Uuid,
    pub name: String,
    pub endpoint: Endpoint,
    pub protocol: Protocol,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Endpoint {
    pub host: String,
    pub port: u16,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "type", content = "options", rename_all = "snake_case")]
pub enum Protocol {
    Vless(Vless),
    Vmess(Vmess),
    Trojan(Trojan),
    Shadowsocks(Shadowsocks),
    AnyTls(AnyTls),
    Hysteria2(Hysteria2),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Vless {
    pub uuid: Uuid,
    pub flow: Option<String>,
    pub transport: Option<Transport>,
    pub tls: Option<Tls>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Vmess {
    pub uuid: Uuid,
    pub alter_id: u16,
    pub security: Option<String>,
    pub transport: Option<Transport>,
    pub tls: Option<Tls>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Trojan {
    pub password: String,
    pub transport: Option<Transport>,
    pub tls: Tls,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Shadowsocks {
    pub method: String,
    pub password: String,
    pub plugin: Option<ShadowsocksPlugin>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ShadowsocksPlugin {
    pub name: String,
    pub options: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AnyTls {
    pub password: String,
    pub tls: Tls,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Hysteria2 {
    pub password: Option<String>,
    pub ports: Hysteria2Ports,
    pub obfs: Option<Hysteria2Obfs>,
    pub tls: Tls,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum Hysteria2Ports {
    Single(u16),
    Multiple(Vec<PortRange>),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PortRange {
    pub start: u16,
    pub end: u16,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Hysteria2Obfs {
    pub kind: String,
    pub password: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Tls {
    pub server_name: Option<String>,
    pub insecure: bool,
    pub alpn: Vec<String>,
    pub fingerprint: Option<String>,
    pub reality: Option<Reality>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Reality {
    pub public_key: String,
    pub short_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Transport {
    Tcp,
    WebSocket {
        path: String,
        host: Option<String>,
    },
    Http {
        path: Option<String>,
        host: Option<String>,
    },
    HttpUpgrade {
        path: String,
        host: Option<String>,
    },
    Grpc {
        service_name: String,
    },
    Quic,
}

impl Node {
    pub fn new(name: impl Into<String>, endpoint: Endpoint, protocol: Protocol) -> Self {
        Self {
            id: Uuid::new_v4(),
            name: name.into(),
            endpoint,
            protocol,
        }
    }
}

impl Endpoint {
    pub fn new(host: impl Into<String>, port: u16) -> Self {
        Self {
            host: host.into(),
            port,
        }
    }
}
