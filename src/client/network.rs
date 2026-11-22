//! An explicit host public-IP lookup; no background calls or Core assumptions.
use serde::{Deserialize, Serialize};
use std::{
    io::Read,
    net::IpAddr,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

#[derive(Deserialize, Serialize)]
pub struct IpInfo {
    pub ip: IpAddr,
    #[serde(default)]
    pub country: String,
    #[serde(default)]
    pub region: String,
    #[serde(default)]
    pub city: String,
    #[serde(default)]
    pub isp: String,
    #[serde(default)]
    pub checked_at: u64,
}

pub fn inspect() -> Result<IpInfo, String> {
    let config = ureq::Agent::config_builder()
        .https_only(true)
        .timeout_global(Some(Duration::from_secs(10)))
        .max_redirects(0)
        .build();
    let agent: ureq::Agent = config.into();
    let mut response = agent
        .get("https://api.ip.sb/geoip")
        .header("User-Agent", "v2rayQ/0.1")
        .call()
        .map_err(|_| "Could not look up the host public IP")?;
    let mut bytes = Vec::new();
    response
        .body_mut()
        .as_reader()
        .take(65537)
        .read_to_end(&mut bytes)
        .map_err(|_| "Could not read IP information")?;
    if bytes.len() > 65536 {
        return Err("Invalid IP information response".into());
    }
    parse(&bytes)
}

fn parse(bytes: &[u8]) -> Result<IpInfo, String> {
    let mut info: IpInfo =
        serde_json::from_slice(bytes).map_err(|_| "Invalid IP information response")?;
    for text in [&info.country, &info.region, &info.city, &info.isp] {
        if text.len() > 256 || text.chars().any(char::is_control) {
            return Err("Invalid IP information response".into());
        }
    }
    info.checked_at = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |time| time.as_secs());
    Ok(info)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn validates_ip_without_requiring_location_fields() {
        assert!(parse(br#"{"ip":"2001:db8::1"}"#).is_ok());
        assert!(parse(br#"{"ip":"example.org"}"#).is_err());
        assert!(parse(br#"{"ip":"192.0.2.1","city":"bad\ncity"}"#).is_err());
    }
}
