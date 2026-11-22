pub mod anytls;
pub mod hysteria2;
pub mod shadowsocks;
pub mod trojan;
pub mod vless;
pub mod vmess;

use thiserror::Error;

use crate::model::Node;

#[derive(Debug, Error)]
pub enum ParseError {
    #[error(transparent)]
    AnyTls(#[from] anytls::AnyTlsParseError),
    #[error(transparent)]
    Hysteria2(#[from] hysteria2::Hysteria2ParseError),
    #[error(transparent)]
    Shadowsocks(#[from] shadowsocks::ShadowsocksParseError),
    #[error(transparent)]
    Trojan(#[from] trojan::TrojanParseError),
    #[error(transparent)]
    Vless(#[from] vless::VlessParseError),
    #[error(transparent)]
    Vmess(#[from] vmess::VmessParseError),
    #[error("invalid share URI")]
    InvalidUri,
    #[error("unsupported protocol: {0}")]
    UnsupportedProtocol(String),
}

/// Dispatch on the scheme without first parsing a generic URL:
/// Hysteria2 port hopping URIs are valid shares, but not generic URLs.
pub fn parse(input: &str) -> Result<Node, ParseError> {
    let (scheme, _) = input
        .trim()
        .split_once("://")
        .ok_or(ParseError::InvalidUri)?;

    match scheme {
        "anytls" => Ok(anytls::parse(input.trim())?),
        "hy2" | "hysteria2" => Ok(hysteria2::parse(input.trim())?),
        "ss" => Ok(shadowsocks::parse(input.trim())?),
        "trojan" => Ok(trojan::parse(input.trim())?),
        "vless" => Ok(vless::parse(input.trim())?),
        "vmess" => Ok(vmess::parse(input.trim())?),
        scheme => Err(ParseError::UnsupportedProtocol(scheme.to_owned())),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn allows_hysteria2_port_hopping() {
        let result = parse("hy2://secret@example.org:8443,10000-10010/#Node");
        assert!(result.is_ok(), "{result:?}");
    }
}
