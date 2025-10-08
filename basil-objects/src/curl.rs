use basil_common::{Result, BasilError};

// For symmetry with other modules, provide a register() entry point even though this
// module currently exposes only global helper functions (no OBJECT types).
pub fn register(_reg: &mut crate::Registry) {
    // No object types to register for now.
}

pub fn http_get(url: &str) -> Result<String> {
    let resp = ureq::get(url)
        .call()
        .map_err(|e| BasilError(format!("HTTP_GET$: request failed: {e}")))?;

    let status = resp.status();
    if !(200..=299).contains(&status) {
        return Err(BasilError(format!(
            "HTTP_GET$: HTTP {status} {}",
            resp.status_text()
        )));
    }

    let body = resp
        .into_string()
        .map_err(|e| BasilError(format!("HTTP_GET$: read body failed: {e}")))?;

    Ok(body)
}

pub fn http_post(url: &str, body: &str, content_type: Option<&str>) -> Result<String> {
    let ct = content_type.unwrap_or("text/plain; charset=utf-8");
    let agent = ureq::AgentBuilder::new()
        .timeout(std::time::Duration::from_secs(30))
        .build();

    let resp = agent
        .post(url)
        .set("Content-Type", ct)
        .send_string(body)
        .map_err(|e| BasilError(format!("HTTP_POST$: request failed: {e}")))?;

    let status = resp.status();
    if !(200..=299).contains(&status) {
        return Err(BasilError(format!(
            "HTTP_POST$: HTTP {status} {}",
            resp.status_text()
        )));
    }

    let body = resp
        .into_string()
        .map_err(|e| BasilError(format!("HTTP_POST$: read body failed: {e}")))?;

    Ok(body)
}
