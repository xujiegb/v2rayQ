#![forbid(unsafe_code)]

//! Bounded loopback HTTP transport. Each application supplies its own assets and API.
use subtle::ConstantTimeEq;

use std::{
    io::{self, Read, Write},
    net::{Ipv4Addr, TcpListener, TcpStream},
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
    time::{Duration, Instant},
};

pub const COMMAND_LIMIT: usize = 128 * 1024;
const HEADER_LIMIT: usize = 8192;
const MAX_CONNECTIONS: usize = 12;

pub struct Asset {
    pub path: &'static str,
    pub mime: &'static str,
    pub body: &'static str,
}

pub struct Panel {
    pub title: &'static str,
    pub html: &'static str,
    pub assets: &'static [Asset],
}

pub fn serve(
    panel: Panel,
    port: u16,
    dispatch: impl Fn(&[u8]) -> String + Send + Sync + 'static,
) -> io::Result<()> {
    let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, port))?;
    let port = listener.local_addr()?.port();
    let token = uuid::Uuid::new_v4().to_string();
    println!(
        "v2rayQ {}: http://127.0.0.1:{port}/#access={token}",
        panel.title
    );
    io::stdout().flush()?;
    let shared = Arc::new((panel, token, dispatch));
    let connections = Arc::new(AtomicUsize::new(0));
    for stream in listener.incoming() {
        let stream = stream?;
        if connections.fetch_add(1, Ordering::Relaxed) >= MAX_CONNECTIONS {
            connections.fetch_sub(1, Ordering::Relaxed);
            continue;
        }
        let shared = Arc::clone(&shared);
        let permit = Connection(Arc::clone(&connections));
        std::thread::spawn(move || {
            let _permit = permit;
            let (panel, token, dispatch) = &*shared;
            let _ = handle(stream, panel, port, token, dispatch);
        });
    }
    Ok(())
}

struct Connection(Arc<AtomicUsize>);

impl Drop for Connection {
    fn drop(&mut self) {
        self.0.fetch_sub(1, Ordering::Relaxed);
    }
}

fn handle(
    mut stream: TcpStream,
    panel: &Panel,
    port: u16,
    token: &str,
    dispatch: impl Fn(&[u8]) -> String,
) -> io::Result<()> {
    stream.set_write_timeout(Some(Duration::from_secs(5)))?;
    let until = Instant::now() + Duration::from_secs(5);
    let mut buffer = [0; HEADER_LIMIT];
    let mut fields = [httparse::EMPTY_HEADER; 48];
    let mut size = 0;
    while !buffer[..size].windows(4).any(|part| part == b"\r\n\r\n") {
        if size == buffer.len() {
            return failure(&mut stream, "431 Request Header Fields Too Large");
        }
        let count = read_before(&mut stream, &mut buffer[size..], until)?;
        if count == 0 {
            return Ok(());
        }
        size += count;
    }
    let mut request = httparse::Request::new(&mut fields);
    let Ok(httparse::Status::Complete(header_end)) = request.parse(&buffer[..size]) else {
        return failure(&mut stream, "400 Bad Request");
    };
    fn header<'a>(fields: &[httparse::Header<'a>], name: &str) -> Option<&'a str> {
        let mut values = fields
            .iter()
            .filter(|field| field.name.eq_ignore_ascii_case(name));
        let first = values.next()?;
        if values.next().is_some() {
            return None;
        }
        std::str::from_utf8(first.value).ok()
    }
    let Some(host) = header(request.headers, "Host") else {
        return failure(&mut stream, "400 Bad Request");
    };
    if host != format!("127.0.0.1:{port}") && host != format!("localhost:{port}") {
        return failure(&mut stream, "403 Forbidden");
    }
    if request
        .headers
        .iter()
        .any(|field| field.name.eq_ignore_ascii_case("Transfer-Encoding"))
    {
        return failure(&mut stream, "400 Bad Request");
    }
    let method = request.method.unwrap_or_default();
    let path = request
        .path
        .unwrap_or_default()
        .split('?')
        .next()
        .unwrap_or_default();
    if method == "POST" && path == "/api/v1/command" {
        let authorized = header(request.headers, "Authorization")
            .and_then(|value| value.strip_prefix("Bearer "))
            .is_some_and(|value| bool::from(value.as_bytes().ct_eq(token.as_bytes())));
        let origin = format!("http://{host}");
        let has_origin = request
            .headers
            .iter()
            .any(|field| field.name.eq_ignore_ascii_case("Origin"));
        if !authorized || (has_origin && header(request.headers, "Origin") != Some(origin.as_str()))
        {
            return failure(&mut stream, "403 Forbidden");
        }
        if header(request.headers, "Content-Type") != Some("application/json") {
            return failure(&mut stream, "415 Unsupported Media Type");
        }
        let Some(length) =
            header(request.headers, "Content-Length").and_then(|value| value.parse::<usize>().ok())
        else {
            return failure(&mut stream, "411 Length Required");
        };
        if length > COMMAND_LIMIT {
            return failure(&mut stream, "413 Content Too Large");
        }
        let mut body = Vec::with_capacity(length);
        body.extend_from_slice(&buffer[header_end..size.min(header_end + length)]);
        let available = body.len();
        body.resize(length, 0);
        let mut read = available;
        while read < length {
            let count = read_before(&mut stream, &mut body[read..], until)?;
            if count == 0 {
                return failure(&mut stream, "400 Bad Request");
            }
            read += count;
        }
        let reply = dispatch(&body);
        return respond(&mut stream, false, "200 OK", "application/json", &reply);
    }
    if !matches!(method, "GET" | "HEAD") {
        return failure(&mut stream, "405 Method Not Allowed");
    }
    let head = method == "HEAD";
    if matches!(path, "/" | "/index.html") {
        return respond(
            &mut stream,
            head,
            "200 OK",
            "text/html; charset=utf-8",
            panel.html,
        );
    }
    if let Some(asset) = panel.assets.iter().find(|asset| asset.path == path) {
        return respond(&mut stream, head, "200 OK", asset.mime, asset.body);
    }
    respond(
        &mut stream,
        head,
        "404 Not Found",
        "text/plain",
        "Not found",
    )
}

fn read_before(stream: &mut TcpStream, buffer: &mut [u8], until: Instant) -> io::Result<usize> {
    let remaining = until.saturating_duration_since(Instant::now());
    if remaining.is_zero() {
        return Err(io::ErrorKind::TimedOut.into());
    }
    stream.set_read_timeout(Some(remaining))?;
    stream.read(buffer)
}

fn failure(stream: &mut TcpStream, status: &str) -> io::Result<()> {
    respond(stream, false, status, "text/plain", status)
}

fn respond(
    stream: &mut TcpStream,
    head: bool,
    status: &str,
    mime: &str,
    body: &str,
) -> io::Result<()> {
    write!(
        stream,
        "HTTP/1.1 {status}\r\nContent-Type: {mime}\r\nContent-Length: {}\r\nConnection: close\r\nCache-Control: no-store\r\nX-Content-Type-Options: nosniff\r\nX-Frame-Options: DENY\r\nReferrer-Policy: no-referrer\r\nContent-Security-Policy: default-src 'none'; script-src 'self'; style-src 'self'; connect-src 'self'; form-action 'none'; base-uri 'none'; frame-ancestors 'none'\r\n\r\n",
        body.len()
    )?;
    if !head {
        stream.write_all(body.as_bytes())?;
    }
    stream.flush()
}
