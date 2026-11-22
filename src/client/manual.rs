use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::model::{
    AnyTls, Endpoint, Hysteria2, Hysteria2Ports, Node, Protocol, Shadowsocks, Tls, Trojan, Vless,
    Vmess,
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
        }
    }
}

impl ManualNode {
    pub fn create(&self) -> Result<Node, String> {
        let name = self.name.trim();
        let host = self.host.trim();
        if name.is_empty() || name.len() > 256 {
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
        let port = self
            .port
            .trim()
            .parse::<u16>()
            .ok()
            .filter(|port| *port != 0)
            .ok_or("Port must be between 1 and 65535")?;
        let uuid = || Uuid::parse_str(self.credential.trim()).map_err(|_| "Enter a valid UUID");
        let password = || nonempty(&self.credential, "Enter a password");
        let server_name = self.server_name.trim();
        let tls = || Tls {
            server_name: (!server_name.is_empty()).then(|| server_name.to_owned()),
            insecure: false,
            alpn: Vec::new(),
            fingerprint: None,
            ech: None,
            reality: None,
        };
        let protocol = match self.protocol {
            ProtocolKind::Vless => Protocol::Vless(Vless {
                uuid: uuid()?,
                flow: None,
                transport: None,
                tls: self.tls.then(tls),
            }),
            ProtocolKind::Vmess => Protocol::Vmess(Vmess {
                uuid: uuid()?,
                alter_id: 0,
                security: Some("auto".into()),
                transport: None,
                tls: self.tls.then(tls),
            }),
            ProtocolKind::Trojan => Protocol::Trojan(Trojan {
                password: password()?,
                transport: None,
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
        Ok(Node::new(name, Endpoint::new(host, port), protocol))
    }
}

fn nonempty(value: &str, message: &'static str) -> Result<String, &'static str> {
    if value.is_empty() {
        Err(message)
    } else {
        Ok(value.to_owned())
    }
}
