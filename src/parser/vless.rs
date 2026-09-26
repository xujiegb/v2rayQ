use std::collections::HashMap;

use thiserror::Error;
use url::Url;
use uuid::Uuid;

use crate::model::{Endpoint, Node, Protocol, Reality, Tls, Transport, Vless};

#[derive(Debug, Error)]
pub enum VlessParseError {
    #[error(transparent)]
    Url(#[from] url::ParseError),
    #[error(transparent)]
    Uuid(#[from] uuid::Error),
    #[error("invalid VLESS URI scheme")]
    InvalidScheme,
    #[error("missing UUID")]
    MissingUuid,
    #[error("missing server")]
    MissingServer,
    #[error("missing server port")]
    MissingPort,
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
    #[error("unsupported VLESS encryption: {0}")]
    UnsupportedEncryption(String),
    #[error("unsupported VLESS flow: {0}")]
    UnsupportedFlow(String),
    #[error("unsupported gRPC mode: {0}")]
    UnsupportedGrpcMode(String),
    #[error("gRPC authority is not supported by sing-box")]
    UnsupportedGrpcAuthority,
    #[error("missing REALITY public key")]
    MissingRealityPublicKey,
    #[error("missing REALITY fingerprint")]
    MissingRealityFingerprint,
}

pub fn parse(input: &str) -> Result<Node, VlessParseError> {
    let url = Url::parse(input)?;

    if url.scheme() != "vless" {
        return Err(VlessParseError::InvalidScheme);
    }

    let username = percent_decode(url.username())?;

    if username.is_empty() {
        return Err(VlessParseError::MissingUuid);
    }

    let uuid = Uuid::parse_str(&username)?;

    let host = url
        .host_str()
        .filter(|value| !value.is_empty())
        .ok_or(VlessParseError::MissingServer)?
        .to_owned();

    let port = url.port().ok_or(VlessParseError::MissingPort)?;

    if port == 0 {
        return Err(VlessParseError::InvalidPort);
    }

    let parameters = parse_query(url.query())?;

    validate_encryption(&parameters)?;
    let flow = parse_flow(&parameters)?;
    let transport = parse_transport(&parameters)?;
    let tls = parse_tls(&parameters)?;

    let name = match url.fragment() {
        Some(fragment) if !fragment.is_empty() => percent_decode(fragment)?,
        _ => format!("{host}:{port}"),
    };

    Ok(Node::new(
        name,
        Endpoint::new(host, port),
        Protocol::Vless(Vless {
            uuid,
            flow,
            transport,
            tls,
        }),
    ))
}

fn validate_encryption(
    parameters: &HashMap<String, String>,
) -> Result<(), VlessParseError> {
    match parameters.get("encryption").map(String::as_str) {
        None | Some("none") => Ok(()),
        Some("") => Err(VlessParseError::InvalidParameter(
            "encryption".to_owned(),
        )),
        Some(value) => Err(VlessParseError::UnsupportedEncryption(
            value.to_owned(),
        )),
    }
}

fn parse_flow(
    parameters: &HashMap<String, String>,
) -> Result<Option<String>, VlessParseError> {
    match parameters.get("flow").map(String::as_str) {
        None | Some("") => Ok(None),
        Some("xtls-rprx-vision") => Ok(Some("xtls-rprx-vision".to_owned())),
        Some(value) => Err(VlessParseError::UnsupportedFlow(value.to_owned())),
    }
}

fn parse_transport(
    parameters: &HashMap<String, String>,
) -> Result<Option<Transport>, VlessParseError> {
    let transport = parameters
        .get("type")
        .map(String::as_str)
        .unwrap_or("tcp");

    match transport {
        "tcp" => Ok(None),
        "ws" => Ok(Some(Transport::WebSocket {
            path: optional_nonempty(parameters, "path")?
                .unwrap_or_else(|| "/".to_owned()),
            host: optional_value(parameters, "host"),
        })),
        "http" => Ok(Some(Transport::Http {
            path: Some(
                optional_nonempty(parameters, "path")?
                    .unwrap_or_else(|| "/".to_owned()),
            ),
            host: optional_nonempty(parameters, "host")?,
        })),
        "grpc" => {
            if let Some(mode) = parameters.get("mode") {
                if mode.is_empty() {
                    return Err(VlessParseError::InvalidParameter(
                        "mode".to_owned(),
                    ));
                }

                if mode != "gun" {
                    return Err(VlessParseError::UnsupportedGrpcMode(
                        mode.clone(),
                    ));
                }
            }

            if parameters
                .get("authority")
                .is_some_and(|value| !value.is_empty())
            {
                return Err(VlessParseError::UnsupportedGrpcAuthority);
            }

            let service_name = parameters
                .get("serviceName")
                .filter(|value| !value.is_empty())
                .ok_or_else(|| {
                    VlessParseError::InvalidParameter(
                        "serviceName".to_owned(),
                    )
                })?
                .clone();

            Ok(Some(Transport::Grpc { service_name }))
        }
        "httpupgrade" => Ok(Some(Transport::HttpUpgrade {
            path: optional_nonempty(parameters, "path")?
                .unwrap_or_else(|| "/".to_owned()),
            host: optional_value(parameters, "host"),
        })),
        value => Err(VlessParseError::UnsupportedTransport(value.to_owned())),
    }
}

fn parse_tls(
    parameters: &HashMap<String, String>,
) -> Result<Option<Tls>, VlessParseError> {
    let security = parameters
        .get("security")
        .map(String::as_str)
        .unwrap_or("none");

    match security {
        "none" => Ok(None),
        "tls" => Ok(Some(Tls {
            server_name: optional_nonempty(parameters, "sni")?,
            insecure: false,
            alpn: parse_alpn(parameters)?,
            fingerprint: Some(
                optional_nonempty(parameters, "fp")?
                    .unwrap_or_else(|| "chrome".to_owned()),
            ),
            ech: None,
            reality: None,
        })),
        "reality" => {
            let public_key = parameters
                .get("pbk")
                .filter(|value| !value.is_empty())
                .ok_or(VlessParseError::MissingRealityPublicKey)?
                .clone();

            let fingerprint = parameters
                .get("fp")
                .filter(|value| !value.is_empty())
                .ok_or(VlessParseError::MissingRealityFingerprint)?
                .clone();

            Ok(Some(Tls {
                server_name: optional_nonempty(parameters, "sni")?,
                insecure: false,
                alpn: parse_alpn(parameters)?,
                fingerprint: Some(fingerprint),
                ech: None,
                reality: Some(Reality {
                    public_key,
                    short_id: optional_value(parameters, "sid"),
                }),
            }))
        }
        value => Err(VlessParseError::UnsupportedSecurity(value.to_owned())),
    }
}

fn parse_alpn(
    parameters: &HashMap<String, String>,
) -> Result<Vec<String>, VlessParseError> {
    let Some(value) = parameters.get("alpn") else {
        return Ok(Vec::new());
    };

    if value.is_empty() {
        return Err(VlessParseError::InvalidParameter("alpn".to_owned()));
    }

    let values = value
        .split(',')
        .map(str::to_owned)
        .collect::<Vec<_>>();

    if values.iter().any(String::is_empty) {
        return Err(VlessParseError::InvalidParameter("alpn".to_owned()));
    }

    Ok(values)
}

fn optional_nonempty(
    parameters: &HashMap<String, String>,
    key: &str,
) -> Result<Option<String>, VlessParseError> {
    match parameters.get(key) {
        None => Ok(None),
        Some(value) if value.is_empty() => Err(
            VlessParseError::InvalidParameter(key.to_owned()),
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
) -> Result<HashMap<String, String>, VlessParseError> {
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
            return Err(VlessParseError::DuplicateParameter(key));
        }
    }

    Ok(parameters)
}

fn percent_decode(value: &str) -> Result<String, VlessParseError> {
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
            return Err(VlessParseError::InvalidPercentEncoding);
        }

        let high = hex(bytes[index + 1])
            .ok_or(VlessParseError::InvalidPercentEncoding)?;
        let low = hex(bytes[index + 2])
            .ok_or(VlessParseError::InvalidPercentEncoding)?;

        decoded.push((high << 4) | low);
        index += 3;
    }

    String::from_utf8(decoded).map_err(|_| VlessParseError::InvalidUtf8)
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
    fn parses_reality_tcp() {
        let node = parse(
            "vless://e6c737fb-08d5-462c-b34e-ad26bde9625c@154.36.187.84:63138?type=tcp&encryption=none&security=reality&pbk=3GxstxZUIS-NHA0XPig-bOkPktd1sLsEcocGqPm_vUM&fp=firefox&sni=www.redhat.com&sid=cafe2d8f&spx=%2F&flow=xtls-rprx-vision#JP%20E",
        )
        .unwrap();

        assert_eq!(node.name, "JP E");
        assert_eq!(node.endpoint.host, "154.36.187.84");
        assert_eq!(node.endpoint.port, 63138);

        let Protocol::Vless(config) = node.protocol else {
            panic!();
        };

        assert_eq!(config.flow.as_deref(), Some("xtls-rprx-vision"));
        assert!(config.transport.is_none());

        let tls = config.tls.unwrap();
        assert_eq!(tls.server_name.as_deref(), Some("www.redhat.com"));
        assert_eq!(tls.fingerprint.as_deref(), Some("firefox"));

        let reality = tls.reality.unwrap();
        assert_eq!(
            reality.public_key,
            "3GxstxZUIS-NHA0XPig-bOkPktd1sLsEcocGqPm_vUM"
        );
        assert_eq!(reality.short_id.as_deref(), Some("cafe2d8f"));
    }

    #[test]
    fn rejects_xhttp() {
        let error = parse(
            "vless://e6c737fb-08d5-462c-b34e-ad26bde9625c@example.com:443?type=xhttp",
        )
        .unwrap_err();

        assert!(matches!(
            error,
            VlessParseError::UnsupportedTransport(ref value)
                if value == "xhttp"
        ));
    }

    #[test]
    fn rejects_mkcp() {
        let error = parse(
            "vless://e6c737fb-08d5-462c-b34e-ad26bde9625c@example.com:443?type=kcp",
        )
        .unwrap_err();

        assert!(matches!(
            error,
            VlessParseError::UnsupportedTransport(ref value)
                if value == "kcp"
        ));
    }

    #[test]
    fn rejects_non_none_encryption() {
        let error = parse(
            "vless://e6c737fb-08d5-462c-b34e-ad26bde9625c@example.com:443?encryption=mlkem768x25519",
        )
        .unwrap_err();

        assert!(matches!(
            error,
            VlessParseError::UnsupportedEncryption(_)
        ));
    }

    #[test]
    fn ignores_unsupported_reality_spider_x() {
        let node = parse(
            "vless://e6c737fb-08d5-462c-b34e-ad26bde9625c@example.com:443?security=reality&pbk=abc&fp=chrome&spx=%2F",
        )
        .unwrap();

        let Protocol::Vless(config) = node.protocol else {
            panic!();
        };

        assert!(config.tls.unwrap().reality.is_some());
    }
}
