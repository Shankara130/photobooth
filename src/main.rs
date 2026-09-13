//! Photobooth — DSLR USB tethering (gphoto2) + mode kamera iPad (browser),
//! UI web vanilla yang diserve axum. Jalankan lalu buka dari iPad di LAN.

mod api;
mod camera;
mod state;
mod store;
mod strip;
mod textblocks;
mod tls;

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

    let state = AppState {
        camera: camera::registry::backend_from_env()?,
        data_dir: std::path::PathBuf::from(env_or("DATA_DIR", "./data")),
        web_dir: std::path::PathBuf::from(env_or("WEB_DIR", "./web")),
        shots_per_strip,
        http_port: port,
    };
    store::init_dirs(&state.data_dir).await?;

    let app = api::router(state.clone());
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
