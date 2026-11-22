use super::*;

#[derive(Default)]
pub(super) struct ManualForm {
    pub(super) name: String,
    pub(super) host: String,
    pub(super) port: String,
    pub(super) protocol: usize,
    pub(super) credential: String,
    pub(super) method: String,
    pub(super) server_name: String,
}

impl ManualForm {
    pub(super) fn create(&self) -> Result<Node, String> {
        let host = self.host.trim();
        let name = self.name.trim();
        if host.is_empty() || name.is_empty() {
            return Err("Please enter a node name and server address".into());
        }
        let port: u16 = self.port.trim().parse()
            .map_err(|_| "Port must be between 1 and 65535")?;
        if port == 0 {
            return Err("Port cannot be zero".into());
        }
        let credential = self.credential.trim();
        let tls = || Tls {
            server_name: (!self.server_name.trim().is_empty())
                .then(|| self.server_name.trim().to_owned()),
            insecure: false,
            alpn: Vec::new(),
            fingerprint: None,
            ech: None,
            reality: None,
        };
        let protocol = match self.protocol {
            0 => Protocol::Vless(Vless {
                uuid: Uuid::parse_str(credential).map_err(|error| error.to_string())?,
                flow: None,
                transport: None,
                tls: (!self.server_name.trim().is_empty()).then(tls),
            }),
            1 => Protocol::Vmess(Vmess {
                uuid: Uuid::parse_str(credential).map_err(|error| error.to_string())?,
                alter_id: 0,
                security: Some("auto".to_owned()),
                transport: None,
                tls: (!self.server_name.trim().is_empty()).then(tls),
            }),
            2 => Protocol::Trojan(Trojan {
                password: nonempty(credential)?,
                transport: None,
                tls: tls(),
            }),
            3 => Protocol::Shadowsocks(Shadowsocks {
                method: nonempty(self.method.trim())?,
                password: nonempty(credential)?,
                plugin: None,
            }),
            4 => Protocol::AnyTls(AnyTls {
                password: nonempty(credential)?,
                tls: tls(),
            }),
            _ => Protocol::Hysteria2(Hysteria2 {
                password: (!credential.is_empty()).then(|| credential.to_owned()),
                ports: Hysteria2Ports::Single(port),
                obfs: None,
                tls: tls(),
            }),
        };
        Ok(Node::new(name, Endpoint::new(host, port), protocol))
    }
}

fn nonempty(value: &str) -> Result<String, String> {
    if value.is_empty() { Err("Credential or method cannot be empty".into()) }
    else { Ok(value.to_owned()) }
}

