//! Penyimpanan media — file JPEG di DATA_DIR; metadata discan ulang dari disk
//! (stateless, tanpa sidecar JSON).

use std::path::{Path, PathBuf};

use serde::Serialize;

/// Ukuran upload minimum yang dianggap valid.
pub const MIN_UPLOAD: usize = 1024; // 1 KiB
/// Ukuran upload maksimum.
pub const MAX_UPLOAD: usize = 30 * 1024 * 1024; // 30 MiB
/// Batas jumlah item yang dilaporkan `GET /api/photos`.
const LIST_CAP: usize = 200;

#[derive(Debug, Serialize)]
pub struct MediaMeta {
    pub id: String,
    pub kind: &'static str, // "photo" | "strip"
    pub url: String,
    pub created: String, // RFC 3339, zona lokal
    pub size: u64,
}

pub async fn init_dirs(data_dir: &Path) -> std::io::Result<()> {
    tokio::fs::create_dir_all(data_dir.join("photos")).await?;
    tokio::fs::create_dir_all(data_dir.join("strips")).await?;
    Ok(())
}

/// Validasi kasar upload: magic number JPEG (FF D8) + rentang ukuran.
pub fn is_valid_jpeg(bytes: &[u8]) -> bool {
    bytes.len() >= MIN_UPLOAD && bytes.len() <= MAX_UPLOAD && bytes.starts_with(&[0xFF, 0xD8])
}

/// Simpan JPEG ke `data_dir/<sub>/<uuid>.jpg`.
pub async fn save_media(
    data_dir: &Path,
    sub: &str,
    kind: &'static str,
    jpeg: &[u8],
) -> std::io::Result<MediaMeta> {
    let id = uuid::Uuid::new_v4().simple().to_string();
    let path = data_dir.join(sub).join(format!("{id}.jpg"));
    tokio::fs::write(&path, jpeg).await?;
    let url = format!("/data/{sub}/{id}.jpg");
    Ok(MediaMeta {
        id,
        kind,
        url,
        created: chrono::Local::now().to_rfc3339(),
        size: jpeg.len() as u64,
    })
}

/// Path file foto berdasarkan id (id sudah disanitasi pemanggil).
pub fn photo_path(data_dir: &Path, id: &str) -> PathBuf {
    data_dir.join("photos").join(format!("{id}.jpg"))
}

/// Scan photos/ + strips/, urut terbaru dulu.
pub async fn list_media(data_dir: &Path) -> std::io::Result<Vec<MediaMeta>> {
    let mut items: Vec<(std::time::SystemTime, MediaMeta)> = Vec::new();
    for (sub, kind) in [("photos", "photo"), ("strips", "strip")] {
        let mut dir = tokio::fs::read_dir(data_dir.join(sub)).await?;
        while let Some(entry) = dir.next_entry().await? {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) != Some("jpg") {
                continue;
            }
            let Some(id) = path.file_stem().and_then(|s| s.to_str()) else {
                continue;
            };
            let meta = match entry.metadata().await {
                Ok(m) => m,
                Err(_) => continue,
            };
            let modified = match meta.modified() {
                Ok(t) => t,
                Err(_) => continue,
            };
            items.push((
                modified,
                MediaMeta {
                    id: id.to_string(),
                    kind,
                    url: format!("/data/{sub}/{id}.jpg"),
                    created: chrono::DateTime::<chrono::Local>::from(modified).to_rfc3339(),
                    size: meta.len(),
                },
            ));
        }
    }
    items.sort_by(|a, b| b.0.cmp(&a.0));
    Ok(items.into_iter().take(LIST_CAP).map(|(_, m)| m).collect())
}
