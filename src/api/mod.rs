//! Handler HTTP — semua route `/api/*` + static file (web/, data/).

pub mod capture;
pub mod events;
pub mod info;
pub mod photos;
pub mod preview;
pub mod qr;
pub mod strip;
pub mod trigger;
pub mod upload;

use axum::extract::DefaultBodyLimit;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::Router;
use tower_http::services::ServeDir;
use tower_http::trace::TraceLayer;

use crate::state::AppState;

pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/api/info", get(info::info))
        .route("/api/preview.mjpg", get(preview::preview_mjpeg))
        .route("/api/capture", post(capture::capture))
        .route("/api/photos", get(photos::list_photos))
        .route("/api/photos/upload", post(upload::upload))
        .route("/api/strip", post(strip::compose))
        .route("/api/events", get(events::events))
        .route("/api/trigger/inject", post(trigger::inject))
        .route("/api/qr", get(qr::qr))
        .nest_service("/data", ServeDir::new(&state.data_dir))
        .fallback_service(ServeDir::new(&state.web_dir))
        // ⚠ default axum cuma 2 MiB — JPEG iPad 12 MP bisa 3–6 MiB
        .layer(DefaultBodyLimit::max(64 * 1024 * 1024))
        .layer(TraceLayer::new_for_http())
        .with_state(state)
}

/// Error API umum — body JSON `{"error": ...}`.
pub enum AppError {
    BadRequest(String),
    NotFound(String),
    Internal(String),
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, msg) = match self {
            AppError::BadRequest(m) => (StatusCode::BAD_REQUEST, m),
            AppError::NotFound(m) => (StatusCode::NOT_FOUND, m),
            AppError::Internal(m) => (StatusCode::INTERNAL_SERVER_ERROR, m),
        };
        (status, axum::Json(serde_json::json!({ "error": msg }))).into_response()
    }
}

impl From<std::io::Error> for AppError {
    fn from(e: std::io::Error) -> Self {
        AppError::Internal(e.to_string())
    }
}
