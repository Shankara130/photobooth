//! GET /api/info — mode aktif + kemampuan; UI menyesuaikan perilaku dari sini.

use axum::extract::State;
use axum::Json;
use serde_json::json;

use crate::state::AppState;

pub async fn info(State(st): State<AppState>) -> Json<serde_json::Value> {
    let cam = st.camera.clone();
    let camera = tokio::task::spawn_blocking(move || cam.info())
        .await
        .unwrap_or_else(|e| json!({ "error": e.to_string() }));

    let client_mode = st.camera.name() == "browser";
    Json(json!({
        "backend": st.camera.name(),
        "camera": camera,
        "shotsPerStrip": st.shots_per_strip,
        "triggerMode": st.trigger_mode.as_str(),
        "qrOrigin": st.qr_origin,
        "port": st.http_port,
        "features": {
            "serverCapture": !client_mode,
            "serverPreview": !client_mode,
            "clientPreview": client_mode,
        },
    }))
}
