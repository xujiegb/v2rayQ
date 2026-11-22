use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::model::{
    AnyTls, Endpoint, Hysteria2, Hysteria2Ports, Node, Protocol, Shadowsocks, Tls, Transport,
    Trojan, Vless, Vmess,
};

#[derive(Clone, Copy, Default, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ProtocolKind {
    #[default]
    Vless,
    Vmess,
    Trojan,
    Shadowsocks,
    AnyTls,
    Hysteria2,
}

impl ProtocolKind {
    pub const ALL: [Self; 6] = [
        Self::Vless,
        Self::Vmess,
        Self::Trojan,
        Self::Shadowsocks,
        Self::AnyTls,
        Self::Hysteria2,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Self::Vless => "VLESS",
            Self::Vmess => "VMess",
            Self::Trojan => "Trojan",
            Self::Shadowsocks => "Shadowsocks",
            Self::AnyTls => "AnyTLS",
            Self::Hysteria2 => "Hysteria2",
        }
    }

    pub fn supports_transport(self) -> bool {
        matches!(self, Self::Vless | Self::Vmess | Self::Trojan)
    }

    pub fn uses_uuid(self) -> bool {
        matches!(self, Self::Vless | Self::Vmess)
    }

    pub fn of(protocol: &Protocol) -> Self {
        match protocol {
            Protocol::Vless(_) => Self::Vless,
            Protocol::Vmess(_) => Self::Vmess,
            Protocol::Trojan(_) => Self::Trojan,
            Protocol::Shadowsocks(_) => Self::Shadowsocks,
            Protocol::AnyTls(_) => Self::AnyTls,
            Protocol::Hysteria2(_) => Self::Hysteria2,
        }
    }
}

#[derive(Clone, Copy, Default, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ManualTransport {
    #[default]
    Tcp,
    WebSocket,
    Http,
    HttpUpgrade,
    Grpc,
}

impl ManualTransport {
    pub const ALL: [Self; 5] = [
        Self::Tcp,
        Self::WebSocket,
        Self::Http,
        Self::HttpUpgrade,
        Self::Grpc,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Self::Tcp => "TCP",
            Self::WebSocket => "WebSocket",
            Self::Http => "HTTP",
            Self::HttpUpgrade => "HTTP Upgrade",
            Self::Grpc => "gRPC",
        }
    }
}

#[derive(Deserialize, Serialize)]
#[serde(default)]
pub struct ManualNode {
    pub name: String,
    pub host: String,
    pub port: String,
    pub protocol: ProtocolKind,
    pub credential: String,
    pub method: String,
    pub server_name: String,
    pub tls: bool,
    pub transport: ManualTransport,
    pub transport_path: String,
    pub transport_host: String,
    pub service_name: String,
    pub alpn: String,
}

impl Default for ManualNode {
    fn default() -> Self {
        Self {
            name: String::new(),
            host: String::new(),
            port: "443".into(),
            protocol: ProtocolKind::default(),
            credential: String::new(),
            method: "aes-128-gcm".into(),
            server_name: String::new(),
            tls: false,
            transport: ManualTransport::Tcp,
            transport_path: String::new(),
            transport_host: String::new(),
            service_name: String::new(),
            alpn: String::new(),
        }
    }
}

impl ManualNode {
    pub fn create(&self) -> Result<Node, String> {
        let (name, endpoint) = node_endpoint(&self.name, &self.host, &self.port)?;
        let port = endpoint.port;
        let uuid = || Uuid::parse_str(self.credential.trim()).map_err(|_| "Enter a valid UUID");
        let password = || nonempty(&self.credential, "Enter a password");
        for field in [
            &self.server_name,
            &self.transport_path,
            &self.transport_host,
            &self.service_name,
            &self.alpn,
        ] {
            if field.len() > 1024 || field.chars().any(char::is_control) {
                return Err("Connection options must be at most 1,024 bytes and contain no control characters".into());
            }
        }
        let transport = || {
            let path = self.transport_path.trim();
            let host = optional(&self.transport_host);
            match self.transport {
                ManualTransport::Tcp => None,
                ManualTransport::WebSocket => Some(Transport::WebSocket {
                    path: path_or_root(path),
                    host,
                }),
                ManualTransport::Http => Some(Transport::Http {
                    path: optional(path),
                    host,
                }),
                ManualTransport::HttpUpgrade => Some(Transport::HttpUpgrade {
                    path: path_or_root(path),
                    host,
                }),
                ManualTransport::Grpc => Some(Transport::Grpc {
                    service_name: self.service_name.trim().to_owned(),
                }),
            }
        };
        let server_name = self.server_name.trim();
        let tls = || Tls {
            server_name: (!server_name.is_empty()).then(|| server_name.to_owned()),
            insecure: false,
            alpn: self.alpn.split(',').filter_map(optional).collect(),
            fingerprint: None,
            ech: None,
            reality: None,
        };
        let protocol = match self.protocol {
            ProtocolKind::Vless => Protocol::Vless(Vless {
                uuid: uuid()?,
                flow: None,
                transport: transport(),
                tls: self.tls.then(tls),
            }),
            ProtocolKind::Vmess => Protocol::Vmess(Vmess {
                uuid: uuid()?,
                alter_id: 0,
                security: Some("auto".into()),
                transport: transport(),
                tls: self.tls.then(tls),
            }),
            ProtocolKind::Trojan => Protocol::Trojan(Trojan {
                password: password()?,
                transport: transport(),
                tls: tls(),
            }),
            ProtocolKind::Shadowsocks => Protocol::Shadowsocks(Shadowsocks {
                method: nonempty(self.method.trim(), "Enter a cipher")?,
                password: password()?,
                plugin: None,
            }),
            ProtocolKind::AnyTls => Protocol::AnyTls(AnyTls {
                password: password()?,
                tls: tls(),
            }),
            ProtocolKind::Hysteria2 => Protocol::Hysteria2(Hysteria2 {
                password: (!self.credential.is_empty()).then(|| self.credential.to_owned()),
                ports: Hysteria2Ports::Single(port),
                obfs: None,
                tls: tls(),
            }),
        };
        Ok(Node::new(name, endpoint, protocol))
    }
}

pub(super) fn node_endpoint(
    name: &str,
    host: &str,
    port: &str,
) -> Result<(String, Endpoint), String> {
    let name = name.trim();
    let host = host.trim();
    if name.is_empty() || name.len() > 256 || name.chars().any(char::is_control) {
        return Err("Enter a node name with up to 256 bytes".into());
    }
    let host = if host.starts_with('[') && host.ends_with(']') {
        &host[1..host.len() - 1]
    } else {
        host
    };
    if host.is_empty()
        || host.len() > 253
        || (host.parse::<std::net::IpAddr>().is_err()
            && !host.split('.').all(|label| {
                !label.is_empty()
                    && label.len() <= 63
                    && !label.starts_with('-')
                    && !label.ends_with('-')
                    && label
                        .bytes()
                        .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-')
            }))
    {
        return Err("Enter a hostname or IP address".into());
    }
    let port = port
        .trim()
        .parse::<u16>()
        .ok()
        .filter(|port| *port != 0)
        .ok_or("Port must be between 1 and 65535")?;
    Ok((name.to_owned(), Endpoint::new(host, port)))
}

fn nonempty(value: &str, message: &'static str) -> Result<String, &'static str> {
    if value.is_empty() {
        Err(message)
    } else {
        Ok(value.to_owned())
    }
}

fn path_or_root(path: &str) -> String {
    if path.is_empty() {
        "/".into()
    } else {
        path.to_owned()
    }
}

fn optional(text: &str) -> Option<String> {
    let text = text.trim();
    (!text.is_empty()).then(|| text.to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn manual_websocket_and_tls_use_shared_model() -> Result<(), String> {
        let node = ManualNode {
            name: "Example".into(),
            host: "example.org".into(),
            protocol: ProtocolKind::Vless,
            credential: "048c4ef1-4a3c-46a1-9d7c-ddb01285d842".into(),
            tls: true,
            server_name: "tls.example.org".into(),
            alpn: "h2, http/1.1".into(),
            transport: ManualTransport::WebSocket,
            transport_path: "/ws".into(),
            transport_host: "ws.example.org".into(),
            ..ManualNode::default()
        }
        .create()?;
        let Protocol::Vless(options) = node.protocol else {
            return Err("Expected VLESS".into());
        };
        assert_eq!(
            options.transport,
            Some(Transport::WebSocket {
                path: "/ws".into(),
                host: Some("ws.example.org".into())
            })
        );
        assert_eq!(
            options.tls.ok_or("Missing TLS settings")?.alpn,
            ["h2", "http/1.1"]
        );
        Ok(())
    }
}
