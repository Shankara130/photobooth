//! Connector browser — kamera iPad/klien via getUserMedia di web UI.
//! Capture terjadi di sisi klien lalu di-upload lewat `POST /api/photos/upload`;
//! backend ini hanya penanda mode agar UI tahu harus capture client-side.

use super::{CameraError, CameraResult};

pub struct BrowserCamera;

impl super::Camera for BrowserCamera {
    fn name(&self) -> &'static str {
        "browser"
    }

    fn available(&self) -> bool {
        false
    }

    fn capture(&self) -> CameraResult<Vec<u8>> {
        Err(CameraError::Unavailable(
            "mode browser: capture dilakukan klien via getUserMedia — upload lewat POST /api/photos/upload".into(),
        ))
    }

    fn preview(&self) -> CameraResult<Vec<u8>> {
        Err(CameraError::Unavailable(
            "mode browser: preview langsung dari kamera klien (getUserMedia), bukan dari server".into(),
        ))
    }

    fn info(&self) -> serde_json::Value {
        serde_json::json!({
            "backend": "browser",
            "available": false,
            "model": serde_json::Value::Null,
        })
    }
}
