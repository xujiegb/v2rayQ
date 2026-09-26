pub mod anytls;
pub mod hysteria2;
pub mod shadowsocks;
pub mod trojan;
pub mod vless;

use thiserror::Error;
use url::Url;

use crate::model::Node;

#[derive(Debug, Error)]
pub enum ParseError {
    #[error(transparent)]
    AnyTls(#[from] anytls::AnyTlsParseError),
    #[error(transparent)]
    Hysteria2(#[from] hysteria2::Hysteria2ParseError),
    #[error(transparent)]
    Url(#[from] url::ParseError),
    #[error(transparent)]
    Shadowsocks(#[from] shadowsocks::ShadowsocksParseError),
    #[error(transparent)]
    Trojan(#[from] trojan::TrojanParseError),
    #[error(transparent)]
    Vless(#[from] vless::VlessParseError),
    #[error("unsupported protocol: {0}")]
    UnsupportedProtocol(String),
}

pub fn parse(input: &str) -> Result<Node, ParseError> {
    let url = Url::parse(input)?;

    match url.scheme() {
        "anytls" => Ok(anytls::parse(input)?),
        "hy2" | "hysteria2" => Ok(hysteria2::parse(input)?),
        "ss" => Ok(shadowsocks::parse(input)?),
        "trojan" => Ok(trojan::parse(input)?),
        "vless" => Ok(vless::parse(input)?),
        scheme => Err(ParseError::UnsupportedProtocol(scheme.to_owned())),
    }
}
