//! State aplikasi yang dibagikan ke semua handler.

use std::path::PathBuf;
use std::sync::Arc;

use crate::camera::Camera;

#[derive(Clone)]
pub struct AppState {
    pub camera: Arc<dyn Camera>,
    pub data_dir: PathBuf,
    pub web_dir: PathBuf,
    pub shots_per_strip: usize,
    /// Port HTTP — dipakai klien untuk membangun URL QR (origin HTTP).
    pub http_port: u16,
}
