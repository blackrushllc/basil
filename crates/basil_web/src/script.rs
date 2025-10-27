use std::{path::PathBuf, process::Stdio};

use anyhow::{Context, Result};
use axum::{body::{Body, to_bytes}, http::{header, Request, StatusCode}, response::Response};
use tokio::{io::AsyncWriteExt, process::Command, time};

use crate::{cgi, compile, util, AppState};

pub async fn run_script(app: &AppState, req: Request<Body>, path: PathBuf) -> Result<Response> {
    // Compute bytecode path
    let bytecode_path = if let Some(dir) = app.cfg.bytecode_dir.as_ref() {
        let mut out = dir.clone().into_std_path_buf();
        let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("script");
        util::ensure_parent_dir(&out)?;
        out.push(format!("{}.basilx", stem));
        out
    } else {
        util::change_ext(&path, "basilx")
    };

    // Compile if stale (no-op: basilc rebuilds as needed, but we keep path prep for future)
    let _co = compile::compile_if_stale(&path, &bytecode_path, app.cfg.bytecode_dir.as_ref().map(|b| b.as_std_path())).await?;

    // Run basil VM: process-runner via basilc
    let timeout = app.cfg.script_timeout;
    let (parts, body) = req.into_parts();
    let env = cgi::make_env(&Request::from_parts(parts.clone(), Body::empty()), None);
    let mut child = Command::new(if cfg!(target_os = "windows") { "basilc.exe" } else { "basilc" })
        .arg("run")
        .arg(&path) // For now run source; TODO: run bytecode once basilc supports it
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .envs(env)
        .spawn()
        .context("spawn basilc")?;

    // Stream request body into stdin
    if let Some(mut sin) = child.stdin.take() {
        let body_bytes = to_bytes(Body::from(body), app.cfg.upload_limit).await.unwrap_or_default();
        let _ = sin.write_all(&body_bytes).await;
    }

    let out = match time::timeout(timeout, child.wait_with_output()).await {
        Ok(Ok(o)) => o,
        Ok(Err(e)) => {
            tracing::error!(script = %path.display(), error = %e, "Failed to run script process");
            return Ok(error_response(StatusCode::INTERNAL_SERVER_ERROR, &format!("Failed to run script: {}", e)));
        }
        Err(_) => {
            tracing::error!(script = %path.display(), timeout_secs = %timeout.as_secs(), "Script timed out");
            return Ok(error_response(StatusCode::GATEWAY_TIMEOUT, "Script timed out"));
        }
    };

    if !out.stderr.is_empty() {
        tracing::warn!(stderr = %util::stderr_tail(&out.stderr, 2000), "script stderr");
    }

    // Parse CGI-like headers
    let so = cgi::split_headers_and_body(&out.stdout);
    let mut resp = Response::new(Body::from(so.body));
    *resp.headers_mut() = so.headers;
    if let Some(status) = so.status { *resp.status_mut() = status; }
    else if resp.headers().get(header::LOCATION).is_some() { *resp.status_mut() = StatusCode::FOUND; }

    Ok(resp)
}

fn error_response(status: StatusCode, msg: &str) -> Response {
    let body = format!("<h1>{}</h1><pre>{}</pre>", status, msg);
    let mut resp = Response::new(Body::from(body));
    *resp.status_mut() = status;
    resp
}
