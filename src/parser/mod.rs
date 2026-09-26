pub mod shadowsocks;

use thiserror::Error;
use url::Url;

use crate::model::Node;

#[derive(Debug, Error)]
pub enum ParseError {
    #[error(transparent)]
    Url(#[from] url::ParseError),
    #[error(transparent)]
    Shadowsocks(#[from] shadowsocks::ShadowsocksParseError),
    #[error("unsupported protocol: {0}")]
    UnsupportedProtocol(String),
}

pub fn parse(input: &str) -> Result<Node, ParseError> {
    let url = Url::parse(input)?;

    match url.scheme() {
        "ss" => Ok(shadowsocks::parse(input)?),
        scheme => Err(ParseError::UnsupportedProtocol(scheme.to_owned())),
    }
}
