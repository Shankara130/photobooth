//! Connector gphoto2 — tethering DSLR via USB (jalur utama full-res).
//!
//! Semua operasi libgphoto2 bersifat blocking (crate gphoto2 menjalankannya di
//! thread latar) → pemanggil wajib membungkus dengan `tokio::task::spawn_blocking`.
//! Satu `Mutex<Option<Camera>>` menjaga koneksi persisten dan menyerialisasi
//! preview vs capture; koneksi di-reset saat error agar dicoba buka ulang.

use super::{CameraError, CameraResult};
use gphoto2::{Context, Result as GpResult};

struct GpInner {
    /// Context disimpan karena `get_data` butuh `impl AsRef<Context>`.
    context: Context,
    camera: gphoto2::Camera,
}

pub struct GPhotoCamera {
    inner: std::sync::Mutex<Option<GpInner>>,
    /// Model terakhir yang terdeteksi — cache untuk `info()` tanpa membuka kamera.
    last_model: std::sync::Mutex<Option<String>>,
}

impl GPhotoCamera {
    pub fn new() -> GpResult<Self> {
        Ok(Self {
            inner: std::sync::Mutex::new(None),
            last_model: std::sync::Mutex::new(None),
        })
    }

    /// Buka kamera sekali & pertahankan, lalu jalankan `f`. Saat `f` error,
    /// koneksi di-drop agar percobaan berikutnya membuka ulang (kabel bisa
    /// dicabut-colok kapan saja).
    fn with_cam<T>(&self, f: impl FnOnce(&GpInner) -> GpResult<T>) -> CameraResult<T> {
        let mut guard = self.inner.lock().unwrap();
        if guard.is_none() {
            let context = Context::new()?;
            let camera = context.autodetect_camera().wait()?;
            let model = camera.abilities().model().to_string();
            tracing::info!("kamera terdeteksi: {model}");
            *self.last_model.lock().unwrap() = Some(model);
            *guard = Some(GpInner { context, camera });
        }
        let inner = guard.as_ref().expect("kamera sudah dibuka di atas");
        match f(inner) {
            Ok(v) => Ok(v),
            Err(e) => {
                tracing::warn!("operasi kamera gagal ({e}) — reset koneksi");
                *guard = None;
                Err(CameraError::GPhoto2(e))
            }
        }
    }

    fn cached_model(&self) -> Option<String> {
        self.last_model.lock().unwrap().clone()
    }
}

impl super::Camera for GPhotoCamera {
    fn name(&self) -> &'static str {
        "gphoto2"
    }

    fn available(&self) -> bool {
        self.with_cam(|_| Ok(())).is_ok()
    }

    fn capture(&self) -> CameraResult<Vec<u8>> {
        self.with_cam(|inner| {
            let path = inner.camera.capture_image().wait()?; // masih di kamera
            let file = inner
                .camera
                .fs()
                .download(&path.folder(), &path.name())
                .wait()?;
            Ok(file.get_data(&inner.context).wait()?.to_vec())
        })
    }

    fn preview(&self) -> CameraResult<Vec<u8>> {
        self.with_cam(|inner| {
            Ok(inner
                .camera
                .capture_preview()
                .wait()?
                .get_data(&inner.context)
                .wait()?
                .to_vec())
        })
    }

    fn info(&self) -> serde_json::Value {
        let available = self.available();
        serde_json::json!({
            "backend": "gphoto2",
            "available": available,
            "model": self.cached_model(),
        })
    }
}
