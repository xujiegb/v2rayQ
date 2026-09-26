use std::collections::HashMap;

use thiserror::Error;
use url::Url;

use crate::model::{AnyTls, Endpoint, Node, Protocol, Tls};

#[derive(Debug, Error)]
pub enum AnyTlsParseError {
    #[error(transparent)]
    Url(#[from] url::ParseError),
    #[error("invalid AnyTLS URI scheme")]
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
}

pub fn parse(input: &str) -> Result<Node, AnyTlsParseError> {
    let url = Url::parse(input)?;

    if url.scheme() != "anytls" {
        return Err(AnyTlsParseError::InvalidScheme);
    }

    let password = percent_decode(url.username())?;

    if password.is_empty() {
        return Err(AnyTlsParseError::MissingPassword);
    }

    let host = url
        .host_str()
        .filter(|value| !value.is_empty())
        .ok_or(AnyTlsParseError::MissingServer)?
        .to_owned();

    let port = url.port().unwrap_or(443);

    if port == 0 {
        return Err(AnyTlsParseError::InvalidPort);
    }

    let parameters = parse_query(url.query())?;
    let insecure = parse_insecure(&parameters)?;
    let server_name = optional_nonempty(&parameters, "sni")?;

    let name = match url.fragment() {
        Some(fragment) if !fragment.is_empty() => percent_decode(fragment)?,
        _ => format!("{host}:{port}"),
    };

    Ok(Node::new(
        name,
        Endpoint::new(host, port),
        Protocol::AnyTls(AnyTls {
            password,
            tls: Tls {
                server_name,
                insecure,
                alpn: Vec::new(),
                fingerprint: None,
                ech: None,
                reality: None,
            },
        }),
    ))
}

fn parse_insecure(
    parameters: &HashMap<String, String>,
) -> Result<bool, AnyTlsParseError> {
    match parameters.get("insecure").map(String::as_str) {
        None | Some("0") => Ok(false),
        Some("1") => Ok(true),
        Some(_) => Err(AnyTlsParseError::InvalidParameter(
            "insecure".to_owned(),
        )),
    }
}

fn optional_nonempty(
    parameters: &HashMap<String, String>,
    key: &str,
) -> Result<Option<String>, AnyTlsParseError> {
    match parameters.get(key) {
        None => Ok(None),
        Some(value) if value.is_empty() => Err(
            AnyTlsParseError::InvalidParameter(key.to_owned()),
        ),
        Some(value) => Ok(Some(value.clone())),
    }
}

fn parse_query(
    query: Option<&str>,
) -> Result<HashMap<String, String>, AnyTlsParseError> {
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
            return Err(AnyTlsParseError::DuplicateParameter(key));
        }
    }

    Ok(parameters)
}

fn percent_decode(value: &str) -> Result<String, AnyTlsParseError> {
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
            return Err(AnyTlsParseError::InvalidPercentEncoding);
        }

        let high = hex(bytes[index + 1])
            .ok_or(AnyTlsParseError::InvalidPercentEncoding)?;
        let low = hex(bytes[index + 2])
            .ok_or(AnyTlsParseError::InvalidPercentEncoding)?;

        decoded.push((high << 4) | low);
        index += 3;
    }

    String::from_utf8(decoded).map_err(|_| AnyTlsParseError::InvalidUtf8)
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
    fn parses_basic_anytls() {
        let node = parse(
            "anytls://letmein@example.com/?sni=real.example.com#Example",
        )
        .unwrap();

        assert_eq!(node.name, "Example");
        assert_eq!(node.endpoint.host, "example.com");
        assert_eq!(node.endpoint.port, 443);

        let Protocol::AnyTls(config) = node.protocol else {
            panic!();
        };

        assert_eq!(config.password, "letmein");
        assert_eq!(
            config.tls.server_name.as_deref(),
            Some("real.example.com")
        );
        assert!(!config.tls.insecure);
    }

    #[test]
    fn parses_insecure() {
        let node = parse(
            "anytls://letmein@example.com:8443/?sni=example.com&insecure=1",
        )
        .unwrap();

        let Protocol::AnyTls(config) = node.protocol else {
            panic!();
        };

        assert_eq!(node.endpoint.port, 8443);
        assert!(config.tls.insecure);
    }

    #[test]
    fn decodes_password_and_tag() {
        let node = parse(
            "anytls://hello%40world@example.com/#Hong%20Kong",
        )
        .unwrap();

        let Protocol::AnyTls(config) = node.protocol else {
            panic!();
        };

        assert_eq!(node.name, "Hong Kong");
        assert_eq!(config.password, "hello@world");
    }

    #[test]
    fn ignores_third_party_extensions() {
        let node = parse(
            "anytls://98a6037c-d5f1-4753-9b9c-b078e1ab3f1f@kqpso1l6p6.firewoodzz.com:41956?peer=streaming.hyxyw.com&insecure=1&udp=1&fingerprint=chrome#Hong%20Kong",
        )
        .unwrap();

        let Protocol::AnyTls(config) = node.protocol else {
            panic!();
        };

        assert!(config.tls.insecure);
        assert!(config.tls.server_name.is_none());
        assert!(config.tls.fingerprint.is_none());
    }

    #[test]
    fn rejects_invalid_insecure_value() {
        let error = parse(
            "anytls://letmein@example.com/?insecure=true",
        )
        .unwrap_err();

        assert!(matches!(
            error,
            AnyTlsParseError::InvalidParameter(ref key)
                if key == "insecure"
        ));
    }
}
