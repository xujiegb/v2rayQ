use std::collections::HashMap;

use thiserror::Error;
use url::Url;

use crate::model::{Endpoint, Node, Protocol, Tls, Transport, Trojan};

#[derive(Debug, Error)]
pub enum TrojanParseError {
    #[error(transparent)]
    Url(#[from] url::ParseError),
    #[error("invalid Trojan URI scheme")]
    InvalidScheme,
    #[error("missing password")]
    MissingPassword,
    #[error("missing server")]
    MissingServer,
    #[error("invalid server port")]
    InvalidPort,
    #[error("duplicate parameter: {0}")]
    DuplicateParameter(String),
    #[error("invalid percent encoding")]
    InvalidPercentEncoding,
    #[error("invalid UTF-8")]
    InvalidUtf8,
    #[error("invalid parameter: {0}")]
    InvalidParameter(String),
    #[error("unsupported transport: {0}")]
    UnsupportedTransport(String),
    #[error("unsupported security: {0}")]
    UnsupportedSecurity(String),
    #[error("unsupported Trojan encryption: {0}")]
    UnsupportedEncryption(String),
}

pub fn parse(input: &str) -> Result<Node, TrojanParseError> {
    let url = Url::parse(input)?;

    if url.scheme() != "trojan" {
        return Err(TrojanParseError::InvalidScheme);
    }

    let password = percent_decode(url.username())?;

    if password.is_empty() {
        return Err(TrojanParseError::MissingPassword);
    }

    let host = url
        .host_str()
        .filter(|value| !value.is_empty())
        .ok_or(TrojanParseError::MissingServer)?
        .to_owned();

    let port = url.port().unwrap_or(443);

    if port == 0 {
        return Err(TrojanParseError::InvalidPort);
    }

    let parameters = parse_query(url.query())?;

    validate_security(&parameters)?;
    validate_encryption(&parameters)?;

    let transport = parse_transport(&parameters)?;
    let tls = Tls {
        server_name: optional_nonempty(&parameters, "sni")?,
        insecure: false,
        alpn: parse_alpn(&parameters)?,
        fingerprint: optional_nonempty(&parameters, "fp")?,
        ech: None,
        reality: None,
    };

    let name = match url.fragment() {
        Some(fragment) if !fragment.is_empty() => percent_decode(fragment)?,
        _ => format!("{host}:{port}"),
    };

    Ok(Node::new(
        name,
        Endpoint::new(host, port),
        Protocol::Trojan(Trojan {
            password,
            transport,
            tls,
        }),
    ))
}

fn validate_security(
    parameters: &HashMap<String, String>,
) -> Result<(), TrojanParseError> {
    match parameters.get("security").map(String::as_str) {
        None | Some("tls") => Ok(()),
        Some("") => Err(TrojanParseError::InvalidParameter(
            "security".to_owned(),
        )),
        Some(value) => Err(TrojanParseError::UnsupportedSecurity(
            value.to_owned(),
        )),
    }
}

fn validate_encryption(
    parameters: &HashMap<String, String>,
) -> Result<(), TrojanParseError> {
    match parameters.get("encryption").map(String::as_str) {
        None | Some("none") => Ok(()),
        Some("") => Err(TrojanParseError::InvalidParameter(
            "encryption".to_owned(),
        )),
        Some(value) => Err(TrojanParseError::UnsupportedEncryption(
            value.to_owned(),
        )),
    }
}

fn parse_transport(
    parameters: &HashMap<String, String>,
) -> Result<Option<Transport>, TrojanParseError> {
    let transport = parameters
        .get("type")
        .map(String::as_str)
        .unwrap_or("tcp");

    match transport {
        "tcp" | "original" => Ok(None),
        "ws" => {
            let path = required_nonempty(parameters, "path")?;

            Ok(Some(Transport::WebSocket {
                path,
                host: optional_value(parameters, "host"),
            }))
        }
        "http" => {
            let path = required_nonempty(parameters, "path")?;

            Ok(Some(Transport::Http {
                path: Some(path),
                host: optional_nonempty(parameters, "host")?,
            }))
        }
        "grpc" => {
            let service_name = required_nonempty(parameters, "serviceName")?;

            Ok(Some(Transport::Grpc { service_name }))
        }
        "httpupgrade" => {
            let path = required_nonempty(parameters, "path")?;

            Ok(Some(Transport::HttpUpgrade {
                path,
                host: optional_value(parameters, "host"),
            }))
        }
        value => Err(TrojanParseError::UnsupportedTransport(
            value.to_owned(),
        )),
    }
}

fn parse_alpn(
    parameters: &HashMap<String, String>,
) -> Result<Vec<String>, TrojanParseError> {
    let Some(value) = parameters.get("alpn") else {
        return Ok(Vec::new());
    };

    if value.is_empty() {
        return Err(TrojanParseError::InvalidParameter(
            "alpn".to_owned(),
        ));
    }

    let values = value
        .split(',')
        .map(str::to_owned)
        .collect::<Vec<_>>();

    if values.iter().any(String::is_empty) {
        return Err(TrojanParseError::InvalidParameter(
            "alpn".to_owned(),
        ));
    }

    Ok(values)
}

fn required_nonempty(
    parameters: &HashMap<String, String>,
    key: &str,
) -> Result<String, TrojanParseError> {
    parameters
        .get(key)
        .filter(|value| !value.is_empty())
        .cloned()
        .ok_or_else(|| TrojanParseError::InvalidParameter(key.to_owned()))
}

fn optional_nonempty(
    parameters: &HashMap<String, String>,
    key: &str,
) -> Result<Option<String>, TrojanParseError> {
    match parameters.get(key) {
        None => Ok(None),
        Some(value) if value.is_empty() => Err(
            TrojanParseError::InvalidParameter(key.to_owned()),
        ),
        Some(value) => Ok(Some(value.clone())),
    }
}

fn optional_value(
    parameters: &HashMap<String, String>,
    key: &str,
) -> Option<String> {
    parameters.get(key).cloned()
}

fn parse_query(
    query: Option<&str>,
) -> Result<HashMap<String, String>, TrojanParseError> {
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
            return Err(TrojanParseError::DuplicateParameter(key));
        }
    }

    Ok(parameters)
}

fn percent_decode(value: &str) -> Result<String, TrojanParseError> {
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
            return Err(TrojanParseError::InvalidPercentEncoding);
        }

        let high = hex(bytes[index + 1])
            .ok_or(TrojanParseError::InvalidPercentEncoding)?;
        let low = hex(bytes[index + 2])
            .ok_or(TrojanParseError::InvalidPercentEncoding)?;

        decoded.push((high << 4) | low);
        index += 3;
    }

    String::from_utf8(decoded).map_err(|_| TrojanParseError::InvalidUtf8)
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
    fn parses_basic_trojan() {
        let node = parse(
            "trojan://password@example.com:443?sni=example.com#Example",
        )
        .unwrap();

        assert_eq!(node.name, "Example");
        assert_eq!(node.endpoint.host, "example.com");
        assert_eq!(node.endpoint.port, 443);

        let Protocol::Trojan(config) = node.protocol else {
            panic!();
        };

        assert_eq!(config.password, "password");
        assert!(config.transport.is_none());
        assert_eq!(
            config.tls.server_name.as_deref(),
            Some("example.com")
        );
        assert!(!config.tls.insecure);
    }

    #[test]
    fn parses_websocket() {
        let node = parse(
            "trojan://password@example.com:443?type=ws&host=cdn.example.com&path=%2Ftrojan&sni=example.com",
        )
        .unwrap();

        let Protocol::Trojan(config) = node.protocol else {
            panic!();
        };

        let Some(Transport::WebSocket { path, host }) = config.transport else {
            panic!();
        };

        assert_eq!(path, "/trojan");
        assert_eq!(host.as_deref(), Some("cdn.example.com"));
    }

    #[test]
    fn defaults_port_to_443() {
        let node = parse(
            "trojan://password@example.com?sni=example.com",
        )
        .unwrap();

        assert_eq!(node.endpoint.port, 443);
    }

    #[test]
    fn ignores_allow_insecure_and_peer() {
        let node = parse(
            "trojan://98a6037c-d5f1-4753-9b9c-b078e1ab3f1f@4u5fap2gow.firewoodzz.com:35002?allowInsecure=1&peer=streaming.hyxyw.com",
        )
        .unwrap();

        let Protocol::Trojan(config) = node.protocol else {
            panic!();
        };

        assert!(!config.tls.insecure);
        assert!(config.tls.server_name.is_none());
    }

    #[test]
    fn rejects_xhttp() {
        let error = parse(
            "trojan://password@example.com:443?type=xhttp",
        )
        .unwrap_err();

        assert!(matches!(
            error,
            TrojanParseError::UnsupportedTransport(ref value)
                if value == "xhttp"
        ));
    }

    #[test]
    fn rejects_extra_encryption() {
        let error = parse(
            "trojan://password@example.com:443?encryption=ss%3Baes-256-gcm%3Asecret",
        )
        .unwrap_err();

        assert!(matches!(
            error,
            TrojanParseError::UnsupportedEncryption(_)
        ));
    }
}
