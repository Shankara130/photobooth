//! Registry — memilih backend kamera aktif (mock | gphoto2 | browser) dari env
//! `CAMERA_BACKEND` (default: mock).

use super::{browser, gphoto, mock, Camera};
use std::sync::Arc;

pub fn backend_from_env() -> anyhow::Result<Arc<dyn Camera>> {
    let choice = std::env::var("CAMERA_BACKEND").unwrap_or_else(|_| "mock".into());
    match choice.as_str() {
        "gphoto" | "gphoto2" => {
            let cam = gphoto::GPhotoCamera::new()
                .map_err(|e| anyhow::anyhow!("gphoto2 init gagal: {e}"))?;
            tracing::info!("backend kamera: gphoto2");
            Ok(Arc::new(cam))
        }
        "browser" => {
            tracing::info!("backend kamera: browser (capture di klien via getUserMedia)");
            Ok(Arc::new(browser::BrowserCamera))
        }
        other => {
            if other != "mock" {
                tracing::warn!("CAMERA_BACKEND tidak dikenal: {other:?} — pakai mock");
            }
            tracing::info!("backend kamera: mock");
            Ok(Arc::new(mock::MockCamera::new()))
        }
    }
}
