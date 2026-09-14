//! State aplikasi yang dibagikan ke semua handler.

use std::path::PathBuf;
use std::sync::Arc;

use crate::camera::Camera;
use crate::trigger::TriggerMode;

#[derive(Clone)]
pub struct AppState {
    pub camera: Arc<dyn Camera>,
    pub data_dir: PathBuf,
    pub web_dir: PathBuf,
    pub shots_per_strip: usize,
    /// Port HTTP — dipakai klien untuk membangun URL QR (origin HTTP).
    pub http_port: u16,
    /// Origin publik untuk link QR (mis. `http://192.168.1.10:8090`) — dari
    /// env `QR_ORIGIN` atau IP LAN; `None` = fallback ke hostname browser.
    pub qr_origin: Option<String>,
    /// Tujuan arsip rsync (mis. `user@server:/srv/photobooth`) — `None` = mati.
    pub sync_target: Option<String>,
    /// Hapus file lokal lebih tua dari N hari (0 = jangan pernah).
    pub retention_days: u64,
    /// Mode pemicu shutter kamera (TRIGGER_MODE).
    pub trigger_mode: TriggerMode,
    /// Broadcast event trigger (JSON string) → SSE `/api/events`.
    pub events: tokio::sync::broadcast::Sender<String>,
}
