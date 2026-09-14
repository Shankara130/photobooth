//! Pengelola storage — arsip file baru ke server lain (SYNC_TARGET, via scp
//! over SSH — tidak butuh rsync di server) dan pembersihan file lokal lama
//! (LOCAL_RETENTION_DAYS) agar penyimpanan mesin capture (Mac) tidak penuh.
//! Arsip di server tidak pernah dihapus.

use std::path::Path;
use std::time::Duration;

use crate::state::AppState;

const SYNC_INTERVAL: Duration = Duration::from_secs(120);
const CLEANUP_INTERVAL: Duration = Duration::from_secs(3600);

/// Spawn task latar — no-op bila SYNC_TARGET kosong dan retensi 0.
pub fn spawn(state: AppState) {
    if state.sync_target.is_none() && state.retention_days == 0 {
        return;
    }
    tokio::spawn(async move {
        tracing::info!(
            "janitor aktif — arsip: {}, retensi lokal: {} hari",
            state.sync_target.as_deref().unwrap_or("-"),
            state.retention_days
        );
        // cleanup langsung jalan pada tick pertama, bukan menunggu 1 jam
        let mut last_cleanup = std::time::Instant::now() - CLEANUP_INTERVAL;
        loop {
            tokio::time::sleep(SYNC_INTERVAL).await;
            if let Some(target) = state.sync_target.clone() {
                sync_to_server(&state.data_dir, &target).await;
            }
            if state.retention_days > 0 && last_cleanup.elapsed() >= CLEANUP_INTERVAL {
                last_cleanup = std::time::Instant::now();
                cleanup_old(&state.data_dir, state.retention_days).await;
            }
        }
    });
}

/// Salin file baru (mtime lebih baru dari marker) ke server via scp.
/// Marker `data/.sync_marker` disentuh DULU — file yang muncul di tengah
/// siklus akan terkirim lagi di siklus berikut (scp menimpa, tidak masalah).
/// Run pertama (marker belum ada) mengarsipkan seluruh isi `photos/`+`strips/`.
async fn sync_to_server(data_dir: &Path, target: &str) {
    use tokio::process::Command;

    let Some((host, rdir)) = target.split_once(':') else {
        tracing::warn!("SYNC_TARGET harus berformat host:dir — dapat {target:?}");
        return;
    };
    let marker = data_dir.join(".sync_marker");
    let cutoff = tokio::fs::metadata(&marker)
        .await
        .and_then(|m| m.modified())
        .ok();
    if tokio::fs::write(&marker, b"sync").await.is_err() {
        tracing::warn!("gagal menulis marker sync — arsip dilewati siklus ini");
        return;
    }
    for sub in ["photos", "strips"] {
        let Ok(mut rd) = tokio::fs::read_dir(data_dir.join(sub)).await else {
            continue;
        };
        let mut fresh: Vec<std::path::PathBuf> = Vec::new();
        while let Ok(Some(entry)) = rd.next_entry().await {
            if !entry.file_name().to_string_lossy().ends_with(".jpg") {
                continue;
            }
            let Ok(meta) = entry.metadata().await else { continue };
            if let Some(cut) = cutoff {
                if !matches!(meta.modified(), Ok(t) if t > cut) {
                    continue;
                }
            }
            fresh.push(entry.path());
        }
        if fresh.is_empty() {
            continue;
        }
        let mkdir_ok = matches!(
            Command::new("ssh")
                .arg(host)
                .arg(format!("mkdir -p {rdir}/{sub}"))
                .output()
                .await,
            Ok(o) if o.status.success()
        );
        if !mkdir_ok {
            tracing::warn!("mkdir remote {rdir}/{sub} gagal — lewati {sub}");
            continue;
        }
        let mut cmd = Command::new("scp");
        cmd.arg("-q");
        for path in &fresh {
            cmd.arg(path);
        }
        cmd.arg(format!("{host}:{rdir}/{sub}/"));
        match cmd.output().await {
            Ok(o) if o.status.success() => {
                tracing::info!("arsip {sub}: {} file → {host}:{rdir}/{sub}", fresh.len());
            }
            Ok(o) => {
                tracing::warn!("scp {sub} gagal: {}", String::from_utf8_lossy(&o.stderr));
            }
            Err(e) => tracing::warn!("menjalankan scp gagal: {e}"),
        }
    }
}

/// Hapus foto/strip lokal yang dimodifikasi sebelum cutoff retensi.
async fn cleanup_old(data_dir: &Path, days: u64) {
    let cutoff = std::time::SystemTime::now()
        .checked_sub(Duration::from_secs(days * 86400))
        .unwrap_or(std::time::SystemTime::UNIX_EPOCH);
    let mut freed = 0u64;
    for sub in ["photos", "strips"] {
        let Ok(mut rd) = tokio::fs::read_dir(data_dir.join(sub)).await else {
            continue;
        };
        while let Ok(Some(entry)) = rd.next_entry().await {
            if !entry.file_name().to_string_lossy().ends_with(".jpg") {
                continue;
            }
            let Ok(meta) = entry.metadata().await else { continue };
            if !matches!(meta.modified(), Ok(t) if t < cutoff) {
                continue;
            }
            let size = meta.len();
            if tokio::fs::remove_file(entry.path()).await.is_ok() {
                freed += size;
            }
        }
    }
    if freed > 0 {
        tracing::info!(
            "retensi lokal: {} MiB dihapus (lebih tua dari {days} hari — arsip server tetap)",
            freed / 1_048_576
        );
    }
}
