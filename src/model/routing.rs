use std::net::IpAddr;

use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RuleMatch {
    #[default]
    Domain,
    DomainSuffix,
    IpCidr,
    Port,
}

impl RuleMatch {
    pub const ALL: [Self; 4] = [Self::Domain, Self::DomainSuffix, Self::IpCidr, Self::Port];

    pub fn label(self) -> &'static str {
        match self {
            Self::Domain => "Domain",
            Self::DomainSuffix => "Domain suffix",
            Self::IpCidr => "IP / CIDR",
            Self::Port => "Destination port",
        }
    }

    fn normalize(self, value: &str) -> Result<String, &'static str> {
        let value = value.trim();
        if value.is_empty() || value.len() > 253 || value.chars().any(char::is_control) {
            return Err("Enter a rule value of up to 253 bytes");
        }
        match self {
            Self::Domain | Self::DomainSuffix => {
                let domain = match self {
                    Self::DomainSuffix => value.strip_prefix('.').unwrap_or(value),
                    _ => value,
                };
                let domain = domain.strip_suffix('.').unwrap_or(domain);
                let Ok(url::Host::Domain(domain)) = url::Host::parse(domain) else {
                    return Err("Enter a valid domain name");
                };
                if domain.len() > 253
                    || !domain.split('.').all(|label| {
                        !label.is_empty()
                            && label.len() <= 63
                            && !label.starts_with('-')
                            && !label.ends_with('-')
                            && label
                                .bytes()
                                .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-')
                    })
                {
                    return Err("Enter a valid domain name");
                }
                Ok(domain)
            }
            Self::IpCidr => {
                if let Some((ip, width)) = value.split_once('/') {
                    let ip = ip.parse::<IpAddr>().map_err(|_| "Invalid IP address")?;
                    let width = width
                        .parse::<u8>()
                        .map_err(|_| "Invalid CIDR prefix length")?;
                    let max_width = if ip.is_ipv4() { 32 } else { 128 };
                    if width > max_width {
                        return Err("Invalid CIDR prefix length");
                    }
                    Ok(format!("{ip}/{width}"))
                } else {
                    value
                        .parse::<IpAddr>()
                        .map(|ip| ip.to_string())
                        .map_err(|_| "Enter a valid IP address or CIDR")
                }
            }
            Self::Port => value
                .parse::<u16>()
                .ok()
                .filter(|port| *port > 0)
                .map(|port| port.to_string())
                .ok_or("Enter a destination port between 1 and 65535"),
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RuleTarget {
    #[default]
    Proxy,
    Direct,
    Block,
}

impl RuleTarget {
    pub const ALL: [Self; 3] = [Self::Proxy, Self::Direct, Self::Block];

    pub fn label(self) -> &'static str {
        match self {
            Self::Proxy => "Selected proxy",
            Self::Direct => "Direct",
            Self::Block => "Block",
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct RuleFields {
    pub kind: RuleMatch,
    pub value: String,
    pub target: RuleTarget,
}

impl RuleFields {
    pub fn validate(mut self) -> Result<Self, &'static str> {
        self.value = self.kind.normalize(&self.value)?;
        Ok(self)
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RouteRule {
    pub id: Uuid,
    #[serde(flatten)]
    pub fields: RuleFields,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RuleDirection {
    Up,
    Down,
}
