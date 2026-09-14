//! Pemicu shutter kamera (TRIGGER_MODE=start|shot) — poll event gphoto2 di
//! background lalu broadcast ke browser via SSE `/api/events`.

use crate::camera::TriggerEvent;
use crate::state::AppState;

/// Interval poll event kamera. Cabang Sony tiap poll membaca semua device
/// property descriptor (transfer USB cukup besar) — jangan terlalu rapat.
const POLL_INTERVAL: std::time::Duration = std::time::Duration::from_millis(250);
/// Backoff saat kamera error/lepas sebelum mencoba lagi.
const ERR_BACKOFF: std::time::Duration = std::time::Duration::from_secs(2);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TriggerMode {
    Off,
    Start,
    Shot,
}

impl TriggerMode {
    pub fn parse(s: &str) -> Option<Self> {
        match s.trim().to_ascii_lowercase().as_str() {
            "" | "off" => Some(Self::Off),
            "start" => Some(Self::Start),
            "shot" => Some(Self::Shot),
            _ => None,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Off => "off",
            Self::Start => "start",
            Self::Shot => "shot",
        }
    }
}

/// Spawn task watcher yang meneruskan trigger kamera ke channel SSE.
/// Panggil hanya jika mode != `Off`.
pub fn spawn_watcher(state: AppState) {
    tokio::spawn(async move {
        tracing::info!("trigger watcher aktif (mode {})", state.trigger_mode.as_str());
        loop {
            let cam = state.camera.clone();
            let polled = tokio::task::spawn_blocking(move || cam.poll_triggers()).await;
            match polled {
                Ok(Ok(triggers)) => {
                    for trigger in triggers {
                        let TriggerEvent::Shutter { jpeg } = trigger;
                        // Mode start: foto dari tekanan pemicu hanya di-drain
                        // agar objek tidak tersangkut, bytes-nya dibuang.
                        let photo = if state.trigger_mode == TriggerMode::Shot {
                            match crate::store::save_media(
                                &state.data_dir,
                                "photos",
                                "photo",
                                &jpeg,
                            )
                            .await
                            {
                                Ok(meta) => Some(meta),
                                Err(e) => {
                                    tracing::error!("simpan foto trigger gagal: {e}");
                                    None
                                }
                            }
                        } else {
                            None
                        };
                        let payload = serde_json::json!({
                            "type": "trigger",
                            "mode": state.trigger_mode.as_str(),
                            "photo": photo,
                        });
                        // Err = belum ada subscriber (browser belum connect) — bukan masalah
                        let _ = state.events.send(payload.to_string());
                    }
                    tokio::time::sleep(POLL_INTERVAL).await;
                }
                // Error kamera (kabel cabut dll.) — with_cam sudah reset koneksi;
                // mundur sebentar sebelum mencoba buka ulang.
                Ok(Err(e)) => {
                    tracing::warn!("poll trigger gagal: {e}");
                    tokio::time::sleep(ERR_BACKOFF).await;
                }
                Err(e) => {
                    tracing::warn!("task poll trigger gagal: {e}");
                    tokio::time::sleep(ERR_BACKOFF).await;
                }
            }
        }
    });
}
