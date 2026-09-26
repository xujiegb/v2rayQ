use std::collections::HashMap;

use base64::{Engine as _, engine::general_purpose::STANDARD};
use thiserror::Error;

use crate::model::{
    Endpoint, Hysteria2, Hysteria2Obfs, Hysteria2Ports, Node, PortRange,
    Protocol, Tls,
};

#[derive(Debug, Error)]
pub enum Hysteria2ParseError {
    #[error("invalid Hysteria2 URI scheme")]
    InvalidScheme,
    #[error("missing server")]
    MissingServer,
    #[error("invalid server")]
    InvalidServer,
    #[error("invalid server port")]
    InvalidPort,
    #[error("invalid path")]
    InvalidPath,
    #[error("duplicate parameter: {0}")]
    DuplicateParameter(String),
    #[error("invalid percent encoding")]
    InvalidPercentEncoding,
    #[error("invalid UTF-8")]
    InvalidUtf8,
    #[error("invalid parameter: {0}")]
    InvalidParameter(String),
    #[error("unsupported obfuscation: {0}")]
    UnsupportedObfs(String),
    #[error("obfuscation password is required")]
    MissingObfsPassword,
    #[error("pinSHA256 requires sing-box 1.15.0 or newer")]
    UnsupportedPinSha256,
    #[error("invalid ECH configuration")]
    InvalidEch,
}

pub fn parse(input: &str) -> Result<Node, Hysteria2ParseError> {
    let (scheme, rest) = input
        .split_once("://")
        .ok_or(Hysteria2ParseError::InvalidScheme)?;

    if scheme != "hysteria2" && scheme != "hy2" {
        return Err(Hysteria2ParseError::InvalidScheme);
    }

    let (without_fragment, fragment) = match rest.split_once('#') {
        Some((value, fragment)) => (value, Some(fragment)),
        None => (rest, None),
    };

    let (without_query, query) = match without_fragment.split_once('?') {
        Some((value, query)) => (value, Some(query)),
        None => (without_fragment, None),
    };

    let authority = match without_query.split_once('/') {
        Some((authority, path)) => {
            if !path.is_empty() {
                return Err(Hysteria2ParseError::InvalidPath);
            }
            authority
        }
        None => without_query,
    };

    let (auth, server) = match authority.rsplit_once('@') {
        Some((auth, server)) => {
            let auth = percent_decode(auth)?;
            let auth = if auth.is_empty() { None } else { Some(auth) };
            (auth, server)
        }
        None => (None, authority),
    };

    let (host, ports) = parse_server(server)?;
    let endpoint_port = first_port(&ports);
    let parameters = parse_query(query)?;

    if parameters
        .get("pinSHA256")
        .is_some_and(|value| !value.is_empty())
    {
        return Err(Hysteria2ParseError::UnsupportedPinSha256);
    }

    let obfs = parse_obfs(&parameters)?;
    let insecure = parse_insecure(&parameters)?;
    let server_name = optional_nonempty(&parameters, "sni")?;
    let ech = parse_ech(&parameters)?;

    let name = match fragment {
        Some(fragment) if !fragment.is_empty() => percent_decode(fragment)?,
        _ => format!("{host}:{endpoint_port}"),
    };

    Ok(Node::new(
        name,
        Endpoint::new(host, endpoint_port),
        Protocol::Hysteria2(Hysteria2 {
            password: auth,
            ports,
            obfs,
            tls: Tls {
                server_name,
                insecure,
                alpn: Vec::new(),
                fingerprint: None,
                ech,
                reality: None,
            },
        }),
    ))
}

fn parse_server(
    value: &str,
) -> Result<(String, Hysteria2Ports), Hysteria2ParseError> {
    if value.is_empty() {
        return Err(Hysteria2ParseError::MissingServer);
    }

    if let Some(rest) = value.strip_prefix('[') {
        let end = rest
            .find(']')
            .ok_or(Hysteria2ParseError::InvalidServer)?;
        let host = &rest[..end];
        let suffix = &rest[end + 1..];

        if host.is_empty() {
            return Err(Hysteria2ParseError::MissingServer);
        }

        let ports = if suffix.is_empty() {
            Hysteria2Ports::Single(443)
        } else {
            let port_spec = suffix
                .strip_prefix(':')
                .ok_or(Hysteria2ParseError::InvalidServer)?;
            parse_ports(port_spec)?
        };

        return Ok((host.to_owned(), ports));
    }

    let (host, ports) = match value.rsplit_once(':') {
        Some((host, port_spec)) => {
            if host.is_empty() || host.contains(':') {
                return Err(Hysteria2ParseError::InvalidServer);
            }

            (host.to_owned(), parse_ports(port_spec)?)
        }
        None => (value.to_owned(), Hysteria2Ports::Single(443)),
    };

    if host.is_empty() {
        return Err(Hysteria2ParseError::MissingServer);
    }

    Ok((host, ports))
}

fn parse_ports(value: &str) -> Result<Hysteria2Ports, Hysteria2ParseError> {
    if value.is_empty() {
        return Err(Hysteria2ParseError::InvalidPort);
    }

    let mut ranges = Vec::new();

    for item in value.split(',') {
        if item.is_empty() {
            return Err(Hysteria2ParseError::InvalidPort);
        }

        let range = match item.split_once('-') {
            Some((start, end)) => {
                let start = parse_port(start)?;
                let end = parse_port(end)?;

                if start > end {
                    return Err(Hysteria2ParseError::InvalidPort);
                }

                PortRange { start, end }
            }
            None => {
                let port = parse_port(item)?;
                PortRange {
                    start: port,
                    end: port,
                }
            }
        };

        ranges.push(range);
    }

    if ranges.len() == 1 && ranges[0].start == ranges[0].end {
        Ok(Hysteria2Ports::Single(ranges[0].start))
    } else {
        Ok(Hysteria2Ports::Multiple(ranges))
    }
}

fn parse_port(value: &str) -> Result<u16, Hysteria2ParseError> {
    let port = value
        .parse::<u16>()
        .map_err(|_| Hysteria2ParseError::InvalidPort)?;

    if port == 0 {
        return Err(Hysteria2ParseError::InvalidPort);
    }

    Ok(port)
}

fn first_port(ports: &Hysteria2Ports) -> u16 {
    match ports {
        Hysteria2Ports::Single(port) => *port,
        Hysteria2Ports::Multiple(ranges) => ranges[0].start,
    }
}

fn parse_obfs(
    parameters: &HashMap<String, String>,
) -> Result<Option<Hysteria2Obfs>, Hysteria2ParseError> {
    let Some(kind) = parameters.get("obfs") else {
        if parameters.contains_key("obfs-password") {
            return Err(Hysteria2ParseError::InvalidParameter(
                "obfs-password".to_owned(),
            ));
        }

        return Ok(None);
    };

    if kind.is_empty() {
        return Err(Hysteria2ParseError::InvalidParameter(
            "obfs".to_owned(),
        ));
    }

    if kind != "salamander" && kind != "gecko" {
        return Err(Hysteria2ParseError::UnsupportedObfs(kind.clone()));
    }

    let password = parameters
        .get("obfs-password")
        .filter(|value| !value.is_empty())
        .cloned()
        .ok_or(Hysteria2ParseError::MissingObfsPassword)?;

    Ok(Some(Hysteria2Obfs {
        kind: kind.clone(),
        password,
    }))
}

fn parse_insecure(
    parameters: &HashMap<String, String>,
) -> Result<bool, Hysteria2ParseError> {
    match parameters.get("insecure").map(String::as_str) {
        None | Some("0") => Ok(false),
        Some("1") => Ok(true),
        Some(_) => Err(Hysteria2ParseError::InvalidParameter(
            "insecure".to_owned(),
        )),
    }
}

fn parse_ech(
    parameters: &HashMap<String, String>,
) -> Result<Option<String>, Hysteria2ParseError> {
    let Some(value) = parameters.get("ech") else {
        return Ok(None);
    };

    if value.is_empty() {
        return Err(Hysteria2ParseError::InvalidParameter(
            "ech".to_owned(),
        ));
    }

    STANDARD
        .decode(value)
        .map_err(|_| Hysteria2ParseError::InvalidEch)?;

    Ok(Some(value.clone()))
}

fn optional_nonempty(
    parameters: &HashMap<String, String>,
    key: &str,
) -> Result<Option<String>, Hysteria2ParseError> {
    match parameters.get(key) {
        None => Ok(None),
        Some(value) if value.is_empty() => Err(
            Hysteria2ParseError::InvalidParameter(key.to_owned()),
        ),
        Some(value) => Ok(Some(value.clone())),
    }
}

fn parse_query(
    query: Option<&str>,
) -> Result<HashMap<String, String>, Hysteria2ParseError> {
    let mut parameters = HashMap::new();

    let Some(query) = query else {
        return Ok(parameters);
    };

    for pair in query.split('&') {
        if pair.is_empty() {
            continue;
        }

        let (raw_key, raw_value) = pair.split_once('=').unwrap_or((pair, ""));
        let key = percent_decode(raw_key)?;
        let value = percent_decode(raw_value)?;

        if parameters.insert(key.clone(), value).is_some() {
            return Err(Hysteria2ParseError::DuplicateParameter(key));
        }
    }

    Ok(parameters)
}

fn percent_decode(value: &str) -> Result<String, Hysteria2ParseError> {
    let bytes = value.as_bytes();
    let mut decoded = Vec::with_capacity(bytes.len());
    let mut index = 0;

    while index < bytes.len() {
        if bytes[index] != b'%' {
            decoded.push(bytes[index]);
            index += 1;
            continue;
        }

        if index + 2 >= bytes.len() {
            return Err(Hysteria2ParseError::InvalidPercentEncoding);
        }

        let high = hex(bytes[index + 1])
            .ok_or(Hysteria2ParseError::InvalidPercentEncoding)?;
        let low = hex(bytes[index + 2])
            .ok_or(Hysteria2ParseError::InvalidPercentEncoding)?;

        decoded.push((high << 4) | low);
        index += 3;
    }

    String::from_utf8(decoded).map_err(|_| Hysteria2ParseError::InvalidUtf8)
}

fn hex(value: u8) -> Option<u8> {
    match value {
        b'0'..=b'9' => Some(value - b'0'),
        b'a'..=b'f' => Some(value - b'a' + 10),
        b'A'..=b'F' => Some(value - b'A' + 10),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_basic_uri() {
        let node = parse(
            "hy2://password@example.com:8443/?sni=real.example.com#Example",
        )
        .unwrap();

        assert_eq!(node.name, "Example");
        assert_eq!(node.endpoint.host, "example.com");
        assert_eq!(node.endpoint.port, 8443);

        let Protocol::Hysteria2(config) = node.protocol else {
            panic!();
        };

        assert_eq!(config.password.as_deref(), Some("password"));
        assert!(matches!(config.ports, Hysteria2Ports::Single(8443)));
        assert_eq!(
            config.tls.server_name.as_deref(),
            Some("real.example.com")
        );
    }

    #[test]
    fn parses_port_hopping() {
        let node = parse(
            "hysteria2://password@example.com:123,5000-6000/?insecure=1",
        )
        .unwrap();

        let Protocol::Hysteria2(config) = node.protocol else {
            panic!();
        };

        assert_eq!(node.endpoint.port, 123);
        assert!(config.tls.insecure);

        let Hysteria2Ports::Multiple(ranges) = config.ports else {
            panic!();
        };

        assert_eq!(
            ranges,
            vec![
                PortRange {
                    start: 123,
                    end: 123,
                },
                PortRange {
                    start: 5000,
                    end: 6000,
                },
            ]
        );
    }

    #[test]
    fn parses_gecko_obfs() {
        let node = parse(
            "hy2://password@example.com/?obfs=gecko&obfs-password=secret",
        )
        .unwrap();

        let Protocol::Hysteria2(config) = node.protocol else {
            panic!();
        };

        let obfs = config.obfs.unwrap();
        assert_eq!(obfs.kind, "gecko");
        assert_eq!(obfs.password, "secret");
    }

    #[test]
    fn treats_userpass_as_password() {
        let node = parse(
            "hy2://user%3Apassword@example.com/",
        )
        .unwrap();

        let Protocol::Hysteria2(config) = node.protocol else {
            panic!();
        };

        assert_eq!(config.password.as_deref(), Some("user:password"));
    }

    #[test]
    fn rejects_certificate_pin_on_sing_box_1_14() {
        let error = parse(
            "hy2://password@example.com/?pinSHA256=deadbeef",
        )
        .unwrap_err();

        assert!(matches!(
            error,
            Hysteria2ParseError::UnsupportedPinSha256
        ));
    }

    #[test]
    fn ignores_nonstandard_bandwidth_parameters() {
        let node = parse(
            "hy2://password@example.com/?upmbps=100&downmbps=100",
        )
        .unwrap();

        let Protocol::Hysteria2(config) = node.protocol else {
            panic!();
        };

        assert!(config.obfs.is_none());
    }
}
