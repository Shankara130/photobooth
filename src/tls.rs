//! Sertifikat self-signed — HTTPS wajib agar getUserMedia jalan di Safari iPad
//! (secure context). Dibuat sekali dengan rcgen, disimpan di `data/certs/`,
//! dipakai ulang saat restart.

use std::net::IpAddr;
use std::path::Path;

use anyhow::{Context as _, Result};

/// Pastikan pasangan cert/key PEM ada di `data/certs/`; buat baru bila belum.
pub async fn ensure_cert(data_dir: &Path) -> Result<(Vec<u8>, Vec<u8>)> {
    let dir = data_dir.join("certs");
    let cert_path = dir.join("cert.pem");
    let key_path = dir.join("key.pem");

    if let (Ok(cert), Ok(key)) = (
        tokio::fs::read(&cert_path).await,
        tokio::fs::read(&key_path).await,
    ) {
        tracing::info!("sertifikat HTTPS dipakai ulang: {}", cert_path.display());
        return Ok((cert, key));
    }

    tokio::fs::create_dir_all(&dir).await?;
    let (cert, key) = tokio::task::spawn_blocking(generate)
        .await
        .context("generate sertifikat")??;
    tokio::fs::write(&cert_path, &cert).await?;
    tokio::fs::write(&key_path, &key).await?;
    tracing::info!("sertifikat self-signed baru: {}", cert_path.display());
    Ok((cert, key))
}

fn generate() -> Result<(Vec<u8>, Vec<u8>)> {
    let mut params = rcgen::CertificateParams::new(vec!["localhost".to_string()])?;
    if let Some(ip) = lan_ip() {
        params.subject_alt_names.push(rcgen::SanType::IpAddress(ip));
    }
    params
        .subject_alt_names
        .push(rcgen::SanType::IpAddress("127.0.0.1".parse()?));

    let key_pair = rcgen::KeyPair::generate()?;
    let cert = params.self_signed(&key_pair)?;
    Ok((
        cert.pem().into_bytes(),
        key_pair.serialize_pem().into_bytes(),
    ))
}

fn lan_ip() -> Option<IpAddr> {
    let sock = std::net::UdpSocket::bind("0.0.0.0:0").ok()?;
    sock.connect("8.8.8.8:80").ok()?;
    sock.local_addr().ok().map(|a| a.ip())
}
