//! Kamera abstraction layer — connector mock, browser, gphoto2.

pub mod browser;
pub mod gphoto;
pub mod mock;
pub mod registry;

use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};

/// Trait bersama semua backend kamera.
pub trait Camera: Send + Sync {
    /// Nama backend: "mock" | "gphoto2" | "browser".
    fn name(&self) -> &'static str;
    fn available(&self) -> bool;
    /// Foto full-res (bytes JPEG).
    fn capture(&self) -> CameraResult<Vec<u8>>;
    /// Satu frame preview kecil (bytes JPEG) untuk stream MJPEG.
    fn preview(&self) -> CameraResult<Vec<u8>>;
    fn info(&self) -> serde_json::Value;
}

#[derive(Debug, thiserror::Error)]
pub enum CameraError {
    #[error("kamera tidak tersedia: {0}")]
    Unavailable(String),
    #[error("gphoto2: {0}")]
    GPhoto2(#[from] gphoto2::Error),
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
}

impl IntoResponse for CameraError {
    fn into_response(self) -> Response {
        let (status, msg) = match &self {
            CameraError::Unavailable(m) => (StatusCode::SERVICE_UNAVAILABLE, m.clone()),
            CameraError::GPhoto2(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()),
            CameraError::Io(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()),
        };
        (
            status,
            axum::Json(serde_json::json!({ "error": msg })),
        )
            .into_response()
    }
}

pub type CameraResult<T> = Result<T, CameraError>;
