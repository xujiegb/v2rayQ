use base64::{
    Engine as _,
    engine::general_purpose::{URL_SAFE, URL_SAFE_NO_PAD},
};
use thiserror::Error;
use url::Url;

use crate::model::{Endpoint, Node, Protocol, Shadowsocks, ShadowsocksPlugin};

#[derive(Debug, Error)]
pub enum ShadowsocksParseError {
    #[error(transparent)]
    Url(#[from] url::ParseError),
    #[error("invalid Shadowsocks URI scheme")]
    InvalidScheme,
    #[error("missing server")]
    MissingServer,
    #[error("missing server port")]
    MissingPort,
    #[error("missing user info")]
    MissingUserInfo,
    #[error("invalid user info")]
    InvalidUserInfo,
    #[error("invalid percent encoding")]
    InvalidPercentEncoding,
    #[error("invalid UTF-8")]
    InvalidUtf8,
    #[error("invalid plugin")]
    InvalidPlugin,
}

pub fn parse(input: &str) -> Result<Node, ShadowsocksParseError> {
    let url = Url::parse(input)?;

    if url.scheme() != "ss" {
        return Err(ShadowsocksParseError::InvalidScheme);
    }

    let host = url
        .host_str()
        .filter(|value| !value.is_empty())
        .ok_or(ShadowsocksParseError::MissingServer)?
        .to_owned();

    let port = url.port().ok_or(ShadowsocksParseError::MissingPort)?;

    let (method, password) = parse_user_info(&url)?;
    let plugin = parse_plugin(url.query())?;
    let name = match url.fragment() {
        Some(fragment) if !fragment.is_empty() => percent_decode(fragment)?,
        _ => format!("{host}:{port}"),
    };

    Ok(Node::new(
        name,
        Endpoint::new(host, port),
        Protocol::Shadowsocks(Shadowsocks {
            method,
            password,
            plugin,
        }),
    ))
}

fn parse_user_info(
    url: &Url,
) -> Result<(String, String), ShadowsocksParseError> {
    let username = url.username();

    if username.is_empty() {
        return Err(ShadowsocksParseError::MissingUserInfo);
    }

    if let Some(password) = url.password() {
        let method = percent_decode(username)?;
        let password = percent_decode(password)?;

        if method.is_empty() || password.is_empty() {
            return Err(ShadowsocksParseError::InvalidUserInfo);
        }

        return Ok((method, password));
    }

    let decoded = decode_base64url(username)?;
    let decoded = String::from_utf8(decoded)
        .map_err(|_| ShadowsocksParseError::InvalidUtf8)?;

    let (method, password) = decoded
        .split_once(':')
        .ok_or(ShadowsocksParseError::InvalidUserInfo)?;

    if method.is_empty() || password.is_empty() {
        return Err(ShadowsocksParseError::InvalidUserInfo);
    }

    Ok((method.to_owned(), password.to_owned()))
}

fn decode_base64url(
    value: &str,
) -> Result<Vec<u8>, ShadowsocksParseError> {
    URL_SAFE_NO_PAD
        .decode(value)
        .or_else(|_| URL_SAFE.decode(value))
        .map_err(|_| ShadowsocksParseError::InvalidUserInfo)
}

fn parse_plugin(
    query: Option<&str>,
) -> Result<Option<ShadowsocksPlugin>, ShadowsocksParseError> {
    let Some(query) = query else {
        return Ok(None);
    };

    let mut plugin = None;

    for pair in query.split('&') {
        let (key, value) = pair.split_once('=').unwrap_or((pair, ""));

        if percent_decode(key)? != "plugin" {
            continue;
        }

        let value = percent_decode(value)?;
        let parts = split_plugin(&value);

        if parts.is_empty() || parts[0].is_empty() {
            return Err(ShadowsocksParseError::InvalidPlugin);
        }

        plugin = Some(ShadowsocksPlugin {
            name: parts[0].clone(),
            options: parts[1..].to_vec(),
        });

        break;
    }

    Ok(plugin)
}

fn split_plugin(value: &str) -> Vec<String> {
    let mut parts = Vec::new();
    let mut current = String::new();
    let mut escaped = false;

    for character in value.chars() {
        if escaped {
            current.push(character);
            escaped = false;
            continue;
        }

        match character {
            '\\' => escaped = true,
            ';' => {
                parts.push(current);
                current = String::new();
            }
            _ => current.push(character),
        }
    }

    if escaped {
        current.push('\\');
    }

    parts.push(current);
    parts
}

fn percent_decode(
    value: &str,
) -> Result<String, ShadowsocksParseError> {
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
            return Err(ShadowsocksParseError::InvalidPercentEncoding);
        }

        let high = hex(bytes[index + 1])
            .ok_or(ShadowsocksParseError::InvalidPercentEncoding)?;
        let low = hex(bytes[index + 2])
            .ok_or(ShadowsocksParseError::InvalidPercentEncoding)?;

        decoded.push((high << 4) | low);
        index += 3;
    }

    String::from_utf8(decoded).map_err(|_| ShadowsocksParseError::InvalidUtf8)
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
    fn parses_base64url_user_info() {
        let node = parse(
            "ss://YWVzLTEyOC04LWdjbTp0ZXN0@192.168.100.1:8888#Example1",
        )
        .unwrap();

        assert_eq!(node.name, "Example1");
        assert_eq!(node.endpoint.host, "192.168.100.1");
        assert_eq!(node.endpoint.port, 8888);

        let Protocol::Shadowsocks(config) = node.protocol else {
            panic!();
        };

        assert_eq!(config.method, "aes-128-gcm");
        assert_eq!(config.password, "test");
        assert!(config.plugin.is_none());
    }

    #[test]
    fn parses_plugin() {
        let node = parse(
            "ss://cmM0LW1kNTpwYXNzd2Q@192.168.100.1:8888/?plugin=obfs-local%3Bobfs%3Dhttp#Example2",
        )
        .unwrap();

        let Protocol::Shadowsocks(config) = node.protocol else {
            panic!();
        };

        let plugin = config.plugin.unwrap();

        assert_eq!(plugin.name, "obfs-local");
        assert_eq!(plugin.options, ["obfs=http"]);
    }

    #[test]
    fn parses_plain_aead_2022_user_info() {
        let node = parse(
            "ss://2022-blake3-aes-256-gcm:YctPZ6U7xPPcU%2Bgp3u%2B0tx%2FtRizJN9K8y%2BuKlW2qjlI%3D@192.168.100.1:8888#Example3",
        )
        .unwrap();

        let Protocol::Shadowsocks(config) = node.protocol else {
            panic!();
        };

        assert_eq!(config.method, "2022-blake3-aes-256-gcm");
        assert_eq!(
            config.password,
            "YctPZ6U7xPPcU+gp3u+0tx/tRizJN9K8y+uKlW2qjlI="
        );
    }

    #[test]
    fn ignores_unknown_query_parameters() {
        let node = parse(
            "ss://YWVzLTEyOC04LWdjbTp0ZXN0@192.168.100.1:8888/?unsupported=value#Example",
        )
        .unwrap();

        let Protocol::Shadowsocks(config) = node.protocol else {
            panic!();
        };

        assert!(config.plugin.is_none());
    }
}
