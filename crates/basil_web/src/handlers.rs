use std::sync::Arc;

use axum::{
    extract::State,
    http::{Request, StatusCode},
    response::IntoResponse,
};

use crate::{static_files, AppState};

pub async fn entry(
    State(app): State<Arc<AppState>>,
    req: Request<axum::body::Body>,
) -> axum::response::Response {
    // Record method/uri for logging prior to moving the request
    let method = req.method().clone();
    let uri = req.uri().clone();
    match static_files::dispatch(&app, req).await {
        Ok(resp) => resp,
        Err(err) => {
            tracing::error!(%method, %uri, error = %format!("{:#}", err), "500 Internal Server Error");
            let msg = format!("<h1>Internal Server Error</h1><pre>{:#}</pre>", err);
            (StatusCode::INTERNAL_SERVER_ERROR, msg).into_response()
        }
    }
}
