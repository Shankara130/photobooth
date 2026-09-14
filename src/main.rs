//! Photobooth — DSLR USB tethering (gphoto2) + mode kamera iPad (browser),
//! UI web vanilla yang diserve axum. Jalankan lalu buka dari iPad di LAN.

mod api;
mod camera;
mod janitor;
mod state;
mod store;
mod strip;
mod textblocks;
mod tls;
mod trigger;

use std::net::{IpAddr, SocketAddr};

use anyhow::Context as _;

use crate::state::AppState;

fn env_or(key: &str, default: &str) -> String {
    std::env::var(key).unwrap_or_else(|_| default.to_string())
}

/// IP antarmuka LAN utama (tanpa mengirim paket — trik UDP connect).
fn lan_ip() -> Option<IpAddr> {
    let sock = std::net::UdpSocket::bind("0.0.0.0:0").ok()?;
    sock.connect("8.8.8.8:80").ok()?;
    sock.local_addr().ok().map(|a| a.ip())
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info".into()),
        )
        .init();

    // Provider crypto untuk rustls — axum-server dipakai tanpa provider bawaan.
    let _ = rustls::crypto::ring::default_provider().install_default();

    let port: u16 = env_or("PORT", "8090").parse().context("PORT tidak valid")?;
    let tls_port: u16 = env_or("TLS_PORT", "8443")
        .parse()
        .context("TLS_PORT tidak valid (0 = matikan HTTPS)")?;
    let shots_per_strip = env_or("SHOTS_PER_STRIP", "4")
        .parse::<usize>()
        .unwrap_or(4)
        .clamp(1, 8);

    let trigger_mode = match trigger::TriggerMode::parse(&env_or("TRIGGER_MODE", "off")) {
        Some(mode) => mode,
        None => {
            tracing::warn!("TRIGGER_MODE tidak valid — pakai off");
            trigger::TriggerMode::Off
        }
    };

    // Origin untuk QR: env QR_ORIGIN (mis. alamat home server) → IP LAN →
    // fallback ke hostname browser kiosk di frontend. QR HARUS bisa di-scan
    // dari HP tamu, jadi localhost tidak berguna.
    let qr_origin = {
        let custom = env_or("QR_ORIGIN", "").trim().trim_end_matches('/').to_string();
        if !custom.is_empty() {
            Some(custom)
        } else {
            lan_ip().map(|ip| format!("http://{ip}:{port}"))
        }
    };
    if let Some(origin) = &qr_origin {
        tracing::info!("origin QR: {origin}");
    }

    let state = AppState {
        camera: camera::registry::backend_from_env()?,
        data_dir: std::path::PathBuf::from(env_or("DATA_DIR", "./data")),
        web_dir: std::path::PathBuf::from(env_or("WEB_DIR", "./web")),
        shots_per_strip,
        http_port: port,
        trigger_mode,
        qr_origin,
        sync_target: {
            let t = env_or("SYNC_TARGET", "").trim().to_string();
            (!t.is_empty()).then_some(t)
        },
        retention_days: env_or("LOCAL_RETENTION_DAYS", "0")
            .parse()
            .unwrap_or(0),
        events: tokio::sync::broadcast::channel::<String>(16).0,
    };
    if trigger_mode != trigger::TriggerMode::Off && state.camera.name() != "gphoto2" {
        tracing::warn!(
            "TRIGGER_MODE hanya berarti untuk CAMERA_BACKEND=gphoto (backend aktif: {}) \
             — endpoint inject tetap jalan",
            state.camera.name()
        );
    }
    store::init_dirs(&state.data_dir).await?;
    if trigger_mode != trigger::TriggerMode::Off {
        trigger::spawn_watcher(state.clone());
    }
    janitor::spawn(state.clone());

    let app = api::router(state.clone());

    // Server statis khusus publik (funnel) — HANYA /data (foto & strip),
    // tanpa API — supaya yang terekspos ke internet tidak lebih dari perlu.
    // Jalankan `tailscale funnel --bg $PUBLIC_PORT` untuk menerbitkannya.
    let public_port: u16 = env_or("PUBLIC_PORT", "8091")
        .parse()
        .context("PUBLIC_PORT tidak valid (0 = matikan)")?;
    if public_port != 0 {
        let static_app = axum::Router::new()
            .nest_service("/data", tower_http::services::ServeDir::new(&state.data_dir));
        let pub_addr = SocketAddr::from(([127, 0, 0, 1], public_port));
        let pub_listener = tokio::net::TcpListener::bind(pub_addr).await?;
        tokio::spawn(async move {
            if let Err(e) = axum::serve(pub_listener, static_app).await {
                tracing::warn!("server publik /data berhenti: {e}");
            }
        });
        tracing::info!("server publik /data (untuk funnel): 127.0.0.1:{public_port}");
    }

    let http_addr = SocketAddr::from(([0, 0, 0, 0], port));
    let http = axum::serve(
        tokio::net::TcpListener::bind(http_addr).await?,
        app.clone().into_make_service(),
    );

    tracing::info!("photobooth siap — http://localhost:{port}");
    let lan = lan_ip();
    if let Some(ip) = lan {
        tracing::info!("dari iPad/browser LAN : http://{ip}:{port}");
    }

    if tls_port == 0 {
        tracing::info!("HTTPS dimatikan (TLS_PORT=0)");
        http.await?;
    } else {
        // Safari iPad hanya mengizinkan getUserMedia pada secure context →
        // listener HTTPS self-signed wajib untuk mode kamera iPad.
        let (cert, key) = tls::ensure_cert(&state.data_dir).await?;
        let rustls_config = axum_server::tls_rustls::RustlsConfig::from_pem(cert, key).await?;
        let https_addr = SocketAddr::from(([0, 0, 0, 0], tls_port));
        let https = axum_server::bind_rustls(https_addr, rustls_config).serve(app.into_make_service());
        if let Some(ip) = lan {
            tracing::info!("kamera iPad (HTTPS)   : https://{ip}:{tls_port} — accept warning sekali");
        }
        tokio::try_join!(http, https)?;
    }

    Ok(())
}
