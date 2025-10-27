use std::sync::Arc;

use axum::{extract::State, http::{Request, StatusCode}, response::IntoResponse};

use crate::{static_files, AppState};

pub async fn entry(State(app): State<Arc<AppState>>, req: Request<axum::body::Body>) -> axum::response::Response {
    match static_files::dispatch(&app, req).await {
        Ok(resp) => resp,
        Err(err) => {
            let msg = format!("<h1>Internal Server Error</h1><pre>{:#}</pre>", err);
            (StatusCode::INTERNAL_SERVER_ERROR, msg).into_response()
        }
    }
}
