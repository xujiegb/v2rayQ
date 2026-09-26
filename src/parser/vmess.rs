use std::collections::HashMap;

use thiserror::Error;
use url::Url;
use uuid::Uuid;

use crate::model::{Endpoint, Node, Protocol, Reality, Tls, Transport, Vmess};

#[derive(Debug, Error)]
pub enum VmessParseError {
    #[error(transparent)]
    Url(#[from] url::ParseError),
    #[error(transparent)]
    Uuid(#[from] uuid::Error),
    #[error("invalid VMess URI scheme")]
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
    #[error("unsupported VMess encryption: {0}")]
    UnsupportedEncryption(String),
    #[error("unsupported transport: {0}")]
    UnsupportedTransport(String),
    #[error("unsupported security: {0}")]
    UnsupportedSecurity(String),
    #[error("unsupported gRPC mode: {0}")]
    UnsupportedGrpcMode(String),
    #[error("gRPC authority is not supported by sing-box")]
    UnsupportedGrpcAuthority,
    #[error("missing REALITY public key")]
    MissingRealityPublicKey,
    #[error("missing REALITY fingerprint")]
    MissingRealityFingerprint,
    #[error("certificate pinning from this share link is not supported by sing-box 1.14")]
    UnsupportedCertificatePin,
    #[error("separate certificate verification name is not supported by sing-box")]
    UnsupportedVerifyName,
}

pub fn parse(input: &str) -> Result<Node, VmessParseError> {
    let url = Url::parse(input)?;

    if url.scheme() != "vmess" {
        return Err(VmessParseError::InvalidScheme);
    }

    let username = percent_decode(url.username())?;

    if username.is_empty() {
        return Err(VmessParseError::MissingUuid);
    }

    let uuid = Uuid::parse_str(&username)?;

    let host = url
        .host_str()
        .filter(|value| !value.is_empty())
        .ok_or(VmessParseError::MissingServer)?
        .to_owned();

    let port = url.port().ok_or(VmessParseError::MissingPort)?;

    if port == 0 {
        return Err(VmessParseError::InvalidPort);
    }

    let parameters = parse_query(url.query())?;

    reject_unsupported_tls_fields(&parameters)?;

    let security = parse_encryption(&parameters)?;
    let transport = parse_transport(&parameters)?;
    let tls = parse_tls(&parameters)?;

    let name = match url.fragment() {
        Some(fragment) if !fragment.is_empty() => percent_decode(fragment)?,
        _ => format!("{host}:{port}"),
    };

    Ok(Node::new(
        name,
        Endpoint::new(host, port),
        Protocol::Vmess(Vmess {
            uuid,
            alter_id: 0,
            security: Some(security),
            transport,
            tls,
        }),
    ))
}

fn parse_encryption(
    parameters: &HashMap<String, String>,
) -> Result<String, VmessParseError> {
    match parameters.get("encryption").map(String::as_str) {
        None => Ok("auto".to_owned()),
        Some("") => Err(VmessParseError::InvalidParameter(
            "encryption".to_owned(),
        )),
        Some("auto") => Ok("auto".to_owned()),
        Some("none") => Ok("none".to_owned()),
        Some("aes-128-gcm") => Ok("aes-128-gcm".to_owned()),
        Some("chacha20-poly1305") => Ok("chacha20-poly1305".to_owned()),
        Some(value) => Err(VmessParseError::UnsupportedEncryption(
            value.to_owned(),
        )),
    }
}

fn parse_transport(
    parameters: &HashMap<String, String>,
) -> Result<Option<Transport>, VmessParseError> {
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
                    return Err(VmessParseError::InvalidParameter(
                        "mode".to_owned(),
                    ));
                }

                if mode != "gun" {
                    return Err(VmessParseError::UnsupportedGrpcMode(
                        mode.clone(),
                    ));
                }
            }

            if parameters
                .get("authority")
                .is_some_and(|value| !value.is_empty())
            {
                return Err(VmessParseError::UnsupportedGrpcAuthority);
            }

            let service_name = parameters
                .get("serviceName")
                .filter(|value| !value.is_empty())
                .ok_or_else(|| {
                    VmessParseError::InvalidParameter(
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
        value => Err(VmessParseError::UnsupportedTransport(
            value.to_owned(),
        )),
    }
}

fn parse_tls(
    parameters: &HashMap<String, String>,
) -> Result<Option<Tls>, VmessParseError> {
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
            ech: optional_value(parameters, "ech"),
            reality: None,
        })),
        "reality" => {
            let public_key = parameters
                .get("pbk")
                .filter(|value| !value.is_empty())
                .ok_or(VmessParseError::MissingRealityPublicKey)?
                .clone();

            let fingerprint = parameters
                .get("fp")
                .filter(|value| !value.is_empty())
                .ok_or(VmessParseError::MissingRealityFingerprint)?
                .clone();

            Ok(Some(Tls {
                server_name: optional_nonempty(parameters, "sni")?,
                insecure: false,
                alpn: parse_alpn(parameters)?,
                fingerprint: Some(fingerprint),
                ech: optional_value(parameters, "ech"),
                reality: Some(Reality {
                    public_key,
                    short_id: optional_value(parameters, "sid"),
                }),
            }))
        }
        value => Err(VmessParseError::UnsupportedSecurity(
            value.to_owned(),
        )),
    }
}

fn reject_unsupported_tls_fields(
    parameters: &HashMap<String, String>,
) -> Result<(), VmessParseError> {
    if parameters
        .get("pcs")
        .is_some_and(|value| !value.is_empty())
    {
        return Err(VmessParseError::UnsupportedCertificatePin);
    }

    if parameters
        .get("vcn")
        .is_some_and(|value| !value.is_empty())
    {
        return Err(VmessParseError::UnsupportedVerifyName);
    }

    Ok(())
}

fn parse_alpn(
    parameters: &HashMap<String, String>,
) -> Result<Vec<String>, VmessParseError> {
    let Some(value) = parameters.get("alpn") else {
        return Ok(Vec::new());
    };

    if value.is_empty() {
        return Err(VmessParseError::InvalidParameter(
            "alpn".to_owned(),
        ));
    }

    let values = value
        .split(',')
        .map(str::to_owned)
        .collect::<Vec<_>>();

    if values.iter().any(String::is_empty) {
        return Err(VmessParseError::InvalidParameter(
            "alpn".to_owned(),
        ));
    }

    Ok(values)
}

fn optional_nonempty(
    parameters: &HashMap<String, String>,
    key: &str,
) -> Result<Option<String>, VmessParseError> {
    match parameters.get(key) {
        None => Ok(None),
        Some(value) if value.is_empty() => Err(
            VmessParseError::InvalidParameter(key.to_owned()),
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
) -> Result<HashMap<String, String>, VmessParseError> {
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
            return Err(VmessParseError::DuplicateParameter(key));
        }
    }

    Ok(parameters)
}

fn percent_decode(value: &str) -> Result<String, VmessParseError> {
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
            return Err(VmessParseError::InvalidPercentEncoding);
        }

        let high = hex(bytes[index + 1])
            .ok_or(VmessParseError::InvalidPercentEncoding)?;
        let low = hex(bytes[index + 2])
            .ok_or(VmessParseError::InvalidPercentEncoding)?;

        decoded.push((high << 4) | low);
        index += 3;
    }

    String::from_utf8(decoded).map_err(|_| VmessParseError::InvalidUtf8)
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
    fn parses_vmess_aead_tcp() {
        let node = parse(
            "vmess://f08a563a-674d-4ffb-9f02-89d28aec96c9@qv2ray.net:9265#VMessTCPAuto",
        )
        .unwrap();

        assert_eq!(node.name, "VMessTCPAuto");
        assert_eq!(node.endpoint.host, "qv2ray.net");
        assert_eq!(node.endpoint.port, 9265);

        let Protocol::Vmess(config) = node.protocol else {
            panic!();
        };

        assert_eq!(config.alter_id, 0);
        assert_eq!(config.security.as_deref(), Some("auto"));
        assert!(config.transport.is_none());
        assert!(config.tls.is_none());
    }

    #[test]
    fn parses_vmess_websocket_tls() {
        let node = parse(
            "vmess://44efe52b-e143-46b5-a9e7-aadbfd77eb9c@qv2ray.net:6939?type=ws&security=tls&host=qv2ray.net&path=%2Fsomewhere#VMessWebSocketTLS",
        )
        .unwrap();

        let Protocol::Vmess(config) = node.protocol else {
            panic!();
        };

        let Some(Transport::WebSocket { path, host }) = config.transport else {
            panic!();
        };

        assert_eq!(path, "/somewhere");
        assert_eq!(host.as_deref(), Some("qv2ray.net"));
        assert!(config.tls.is_some());
    }

    #[test]
    fn parses_supported_encryption() {
        let node = parse(
            "vmess://5dc94f3a-ecf0-42d8-ae27-722a68a6456c@qv2ray.net:35897?encryption=aes-128-gcm",
        )
        .unwrap();

        let Protocol::Vmess(config) = node.protocol else {
            panic!();
        };

        assert_eq!(
            config.security.as_deref(),
            Some("aes-128-gcm")
        );
    }

    #[test]
    fn rejects_xhttp() {
        let error = parse(
            "vmess://f08a563a-674d-4ffb-9f02-89d28aec96c9@example.com:443?type=xhttp",
        )
        .unwrap_err();

        assert!(matches!(
            error,
            VmessParseError::UnsupportedTransport(ref value)
                if value == "xhttp"
        ));
    }

    #[test]
    fn rejects_mkcp() {
        let error = parse(
            "vmess://f08a563a-674d-4ffb-9f02-89d28aec96c9@example.com:443?type=kcp",
        )
        .unwrap_err();

        assert!(matches!(
            error,
            VmessParseError::UnsupportedTransport(ref value)
                if value == "kcp"
        ));
    }

    #[test]
    fn rejects_legacy_alter_id_parameter() {
        let node = parse(
            "vmess://f08a563a-674d-4ffb-9f02-89d28aec96c9@example.com:443?aid=64",
        )
        .unwrap();

        let Protocol::Vmess(config) = node.protocol else {
            panic!();
        };

        assert_eq!(config.alter_id, 0);
    }
}
