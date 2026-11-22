#![forbid(unsafe_code)]

// Safe, read-only localhost preview of the future administration panel.
// Never expose this unfinished endpoint publicly or execute Core actions.
use std::{
    io::{Read, Write},
    net::{IpAddr, Ipv4Addr, TcpListener, TcpStream},
    time::Duration,
};

const PANEL: &str = include_str!("../../../frontends/web/server.html");
const CLIENT: &str = include_str!("../../../frontends/web/client.html");
const CSS: &str = include_str!("../../../frontends/shared/panel.css");
const JS: &str = include_str!("../../../frontends/shared/panel.js");

fn main() -> std::io::Result<()> {
    let ip = IpAddr::V4(Ipv4Addr::LOCALHOST);
    let listener = TcpListener::bind((ip, 16837))?;
    println!("v2rayQ preview: http://127.0.0.1:16837/ (localhost only)");
    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                std::thread::spawn(move || {
                    if let Err(error) = handle(stream) {
                        eprintln!("request failed: {error}");
                    }
                });
            }
            Err(error) => eprintln!("accept failed: {error}"),
        }
    }
    Ok(())
}

fn handle(mut stream: TcpStream) -> std::io::Result<()> {
    stream.set_read_timeout(Some(Duration::from_secs(5)))?;
    stream.set_write_timeout(Some(Duration::from_secs(5)))?;
    let mut buffer = [0u8; 4096];
    let size = stream.read(&mut buffer)?;
    let request = String::from_utf8_lossy(&buffer[..size]);
    let mut words = request.lines().next().unwrap_or("").split_whitespace();
    let method = words.next().unwrap_or("");
    let path = words.next().unwrap_or("").split('?').next().unwrap_or("");
    if method != "GET" && method != "HEAD" {
        return respond(&mut stream, method == "HEAD", "405 Method Not Allowed", "text/plain; charset=utf-8", "Read-only preview");
    }
    let (status, mime, content) = match path {
        "/" | "/server" => ("200 OK", "text/html; charset=utf-8", PANEL),
        "/client" => ("200 OK", "text/html; charset=utf-8", CLIENT),
        "/assets/panel.css" => ("200 OK", "text/css; charset=utf-8", CSS),
        "/assets/panel.js" => ("200 OK", "text/javascript; charset=utf-8", JS),
        "/api/v1/status" => (
            "200 OK", "application/json; charset=utf-8",
            r#"{"core_connected":false,"tun_active":false,"transparent_proxy_active":false,"message":"Core adapter is not available"}"#,
        ),
        "/api/v1/capabilities" => (
            "200 OK", "application/json; charset=utf-8",
            r#"{"api_version":1,"core_connected":false,"management_actions":[],"preview_only":true}"#,
        ),
        _ => ("404 Not Found", "text/plain; charset=utf-8", "Not found"),
    };
    respond(&mut stream, method == "HEAD", status, mime, content)
}

fn respond(stream: &mut TcpStream, head: bool, status: &str, mime: &str, body: &str) -> std::io::Result<()> {
    write!(
        stream,
        "HTTP/1.1 {status}\r\nContent-Type: {mime}\r\nContent-Length: {}\r\nConnection: close\r\nCache-Control: no-store\r\nX-Content-Type-Options: nosniff\r\nX-Frame-Options: SAMEORIGIN\r\nReferrer-Policy: no-referrer\r\nContent-Security-Policy: default-src 'self'; script-src 'self'; style-src 'self'; connect-src 'self'; img-src 'self' data:; frame-ancestors 'self'\r\n\r\n",
        body.len()
    )?;
    if !head {
        stream.write_all(body.as_bytes())?;
    }
    stream.flush()
}
