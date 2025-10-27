use std::{fs, path::{Path, PathBuf}};

use anyhow::{Context, Result};
use axum::{body::Body, http::{header, HeaderMap, HeaderValue, Request, StatusCode}, response::Response};
use httpdate::HttpDate;
use mime_guess::from_path;
use percent_encoding::percent_decode_str;

use crate::{util, AppState};

pub fn resolve_path(root: &Path, uri_path: &str) -> Result<PathBuf> {
    let decoded = percent_decode_str(uri_path).decode_utf8_lossy();
    let path = decoded.trim_start_matches('/');
    util::safe_join(root, path)
}

pub async fn serve_file(cfg: &crate::config::Config, parts: http::request::Parts, file: PathBuf) -> Result<Response> {
    // HEAD vs GET handling
    let method = parts.method.clone();
    let mut headers = HeaderMap::new();

    // MIME
    let mime = from_path(&file).first_or_octet_stream();
    headers.insert(header::CONTENT_TYPE, HeaderValue::from_str(mime.as_ref()).unwrap_or(HeaderValue::from_static("application/octet-stream")));

    // ETag and Last-Modified
    if cfg.etag {
        if let Ok(etag) = util::etag_weak_for_meta(&file) {
            headers.insert(header::ETAG, HeaderValue::from_str(&etag).unwrap_or(HeaderValue::from_static("W/\"0\"")));
        }
    }
    if let Ok(md) = fs::metadata(&file) {
        if let Ok(modified) = md.modified() {
            let http_date = HttpDate::from(modified);
            headers.insert(header::LAST_MODIFIED, HeaderValue::from_str(&http_date.to_string()).unwrap());
        }
    }

    // Conditional requests
    if let (Some(if_none), Some(etag)) = (parts.headers.get(header::IF_NONE_MATCH), headers.get(header::ETAG)) {
        if if_none == etag { return Ok(Response::builder().status(StatusCode::NOT_MODIFIED).body(Body::empty()).unwrap()); }
    }

    // Cache-Control: dev-friendly
    headers.insert(header::CACHE_CONTROL, HeaderValue::from_static("no-store"));

    let body = if method == http::Method::HEAD {
        Body::empty()
    } else {
        let bytes = tokio::fs::read(&file).await.context("read file")?;
        Body::from(bytes)
    };

    let mut resp = Response::new(body);
    *resp.headers_mut() = headers;
    Ok(resp)
}

pub async fn dispatch(app: &AppState, req: Request<Body>) -> Result<Response> {
    let (parts, body) = req.into_parts();
    let _method = parts.method.clone();
    let path = resolve_path(app.cfg.root.as_std_path(), parts.uri.path())?;

    if path.is_dir() {
        let idx = path.join(&app.cfg.index);
        return serve_file(&app.cfg, parts, idx).await;
    }

    let ext = path.extension().and_then(|s| s.to_str()).unwrap_or("");
    match ext {
        "basil" | "bas" => return crate::script::run_script(app, Request::from_parts(parts, Body::from(body)), path).await,
        "html" => {
            if crate::template::file_contains_basil(&path).await? {
                return crate::template::render_html_with_basil(app, Request::from_parts(parts, Body::from(body)), path).await;
            } else {
                return serve_file(&app.cfg, parts, path).await;
            }
        }
        _ => return serve_file(&app.cfg, parts, path).await,
    }
}
