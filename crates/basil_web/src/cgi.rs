use std::{collections::HashMap, net::SocketAddr};

use axum::http::{header, HeaderMap, HeaderName, HeaderValue, Request, StatusCode};
use bytes::Bytes;

pub fn make_env(req: &Request<axum::body::Body>, remote: Option<SocketAddr>) -> HashMap<String, String> {
    let mut map = HashMap::new();
    map.insert("REQUEST_METHOD".into(), req.method().to_string());
    map.insert("PATH_INFO".into(), req.uri().path().to_string());
    map.insert("QUERY_STRING".into(), req.uri().query().unwrap_or("").to_string());
    map.insert("CONTENT_TYPE".into(), req.headers().get(header::CONTENT_TYPE).and_then(|v| v.to_str().ok()).unwrap_or("").to_string());
    map.insert("CONTENT_LENGTH".into(), req.headers().get(header::CONTENT_LENGTH).and_then(|v| v.to_str().ok()).unwrap_or("0").to_string());
    if let Some(addr) = remote { map.insert("REMOTE_ADDR".into(), addr.ip().to_string()); }
    for (name, value) in req.headers().iter() {
        let key = format!("HTTP_{}", name.as_str().to_ascii_uppercase().replace('-', "_"));
        if let Ok(val) = value.to_str() { map.insert(key, val.to_string()); }
    }
    map
}

#[derive(Debug, Clone)]
pub struct ScriptOutput { pub status: Option<StatusCode>, pub headers: HeaderMap, pub body: Bytes }

pub fn parse_status_line(s: &str) -> Option<StatusCode> {
    // Accept formats like "200 OK" or "Status: 200 OK"
    let mut it = s.split_whitespace();
    if let Some(code) = it.next() {
        if let Ok(n) = code.parse::<u16>() { return StatusCode::from_u16(n).ok(); }
    }
    None
}

pub fn split_headers_and_body(raw: &[u8]) -> ScriptOutput {
    // Find CRLFCRLF or LFLF
    let sep4 = raw.windows(4).position(|w| w == b"\r\n\r\n");
    let sep2 = raw.windows(2).position(|w| w == b"\n\n");
    let split_idx = sep4.or(sep2).unwrap_or(raw.len());
    let head_bytes = &raw[..split_idx];
    let body_start = if sep4.is_some() { split_idx + 4 } else if sep2.is_some() { split_idx + 2 } else { split_idx };
    let body = &raw[body_start..];

    let mut status: Option<StatusCode> = None;
    let mut headers = HeaderMap::new();

    for line in head_bytes.split(|&b| b == b'\n') {
        let line = String::from_utf8_lossy(line).trim().to_string();
        if line.is_empty() { continue; }
        let (k, v) = if let Some((k, v)) = line.split_once(':') {
            (k.trim(), v.trim())
        } else { ("", "") };
        if k.is_empty() { continue; }
        if k.eq_ignore_ascii_case("Status") {
            status = parse_status_line(v);
            continue;
        }
        if k.eq_ignore_ascii_case("Location") && status.is_none() {
            status = Some(StatusCode::FOUND);
        }
        if let Ok(name) = HeaderName::from_bytes(k.as_bytes()) {
            if let Ok(val) = HeaderValue::from_str(v) {
                headers.append(name, val);
            }
        }
    }

    ScriptOutput { status, headers, body: Bytes::copy_from_slice(body) }
}
