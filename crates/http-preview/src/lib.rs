#![forbid(unsafe_code)]

// Small std-only HTTP frontend for local UI previews. Not a production API.
// Authenticated Core RPC must live in its own service module, never here.
use std::{
    io::{self, Read, Write},
    net::{Ipv4Addr, TcpListener, TcpStream},
    time::Duration,
};

use v2rayq_contracts::{CoreBackend, UnavailableCore, API_VERSION};

const SERVER: &str = include_str!("../../../frontends/web/server.html");
const CLIENT: &str = include_str!("../../../frontends/web/client.html");
const CSS: &str = include_str!("../../../frontends/shared/panel.css");
const SCRIPT: &str = include_str!("../../../frontends/shared/panel.js");

#[derive(Debug, Clone, Copy)]
pub enum Mode {
    Client,
    Server,
}

impl Mode {
    pub fn title(self) -> &'static str {
        match self {
            Self::Client => "Client",
            Self::Server => "Server",
        }
    }
}

pub fn serve(mode: Mode, port: u16) -> io::Result<()> {
    let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, port))?;
    println!("v2rayQ {} UI: http://127.0.0.1:{port}/", mode.title());
    for connection in listener.incoming() {
        match connection {
            Ok(stream) => {
                std::thread::spawn(move || {
                    if let Err(error) = serve_connection(stream, mode) {
                        eprintln!("preview request: {error}");
                    }
                });
            }
            Err(error) => eprintln!("preview accept: {error}"),
        }
    }
    Ok(())
}

fn serve_connection(mut stream: TcpStream, mode: Mode) -> io::Result<()> {
    stream.set_read_timeout(Some(Duration::from_secs(5)))?;
    stream.set_write_timeout(Some(Duration::from_secs(5)))?;
    let mut request = [0u8; 4096];
    let size = stream.read(&mut request)?;
    let request_line = String::from_utf8_lossy(&request[..size]);
    let mut parts = request_line.lines().next().unwrap_or("").split_whitespace();
    let method = parts.next().unwrap_or("");
    let path = parts.next().unwrap_or("");
    let head_only = method == "HEAD";
    if method != "GET" && !head_only {
        return respond(&mut stream, false, "405 Method Not Allowed", "text/plain", "Method not allowed");
    }
    let core = UnavailableCore;
    let (status, mime, body) = match path {
        "/" | "/index.html" => ("200 OK", "text/html; charset=utf-8", match mode {
            Mode::Client => CLIENT.to_owned(), Mode::Server => SERVER.to_owned(),
        }),
        "/assets/panel.css" => ("200 OK", "text/css; charset=utf-8", CSS.to_owned()),
        "/assets/panel.js" => ("200 OK", "text/javascript; charset=utf-8", SCRIPT.to_owned()),
        "/api/v1/status" => {
            let value = core.status();
            ("200 OK", "application/json; charset=utf-8", format!(
                r#"{{"core_connected":{},"tun_active":{},"transparent_proxy_active":{},"message":"Core unavailable"}}"#,
                value.connected, value.tun_active, value.transparent_proxy_active,
            ))
        }
        "/api/v1/capabilities" => {
            let value = core.capabilities();
            ("200 OK", "application/json; charset=utf-8", format!(
                r#"{{"api_version":{},"core_connected":{},"can_modify_network":{},"can_manage_server":{},"preview_only":true}}"#,
                API_VERSION, value.core_available, value.can_modify_network, value.can_manage_server,
            ))
        }
        _ => ("404 Not Found", "text/plain; charset=utf-8", "Not found".to_owned()),
    };
    respond(&mut stream, head_only, status, mime, &body)
}

fn respond(stream: &mut TcpStream, head: bool, status: &str, mime: &str, body: &str) -> io::Result<()> {
    write!(
        stream,
        "HTTP/1.1 {status}\r\nContent-Type: {mime}\r\nContent-Length: {}\r\nConnection: close\r\nCache-Control: no-store\r\nX-Content-Type-Options: nosniff\r\nX-Frame-Options: SAMEORIGIN\r\nReferrer-Policy: no-referrer\r\nContent-Security-Policy: default-src 'self'; script-src 'self'; style-src 'self'; connect-src 'self'; frame-ancestors 'self'\r\n\r\n",
        body.len()
    )?;
    if !head { stream.write_all(body.as_bytes())?; }
    stream.flush()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn client_server_binaries_share_html_assets() {
        assert!(CLIENT.contains("panel.css"));
        assert!(SERVER.contains("panel.css"));
        assert!(CLIENT.contains("data-panel=\"client\""));
        assert!(SERVER.contains("data-panel=\"server\""));
        assert!(!SCRIPT.contains("eval("));
    }
}
