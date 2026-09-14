//! Connector gphoto2 — tethering DSLR via USB (jalur utama full-res).
//!
//! Semua operasi libgphoto2 bersifat blocking (crate gphoto2 menjalankannya di
//! thread latar) → pemanggil wajib membungkus dengan `tokio::task::spawn_blocking`.
//! Satu `Mutex<Option<Camera>>` menjaga koneksi persisten dan menyerialisasi
//! preview vs capture; koneksi di-reset saat error agar dicoba buka ulang.

use std::collections::VecDeque;
use std::time::{Duration, Instant};

use super::{CameraError, CameraResult, TriggerEvent};
use gphoto2::{camera::CameraEvent, Context, Result as GpResult};

/// `NewFile` dalam rentang ini setelah `capture()` host dianggap echo.
const SELF_ECHO_BLACKOUT: Duration = Duration::from_millis(2500);
/// Interval tunggu event per iterasi drain. Bukan 0: cabang Sony PC Control
/// memerlukan wait sungguhan agar event interrupt terbaca (cek 0 ms kosong).
const WAIT_EVENT_SLICE: Duration = Duration::from_millis(30);

struct GpInner {
    /// Context disimpan karena `get_data` butuh `impl AsRef<Context>`.
    context: Context,
    camera: gphoto2::Camera,
}

pub struct GPhotoCamera {
    inner: std::sync::Mutex<Option<GpInner>>,
    /// Model terakhir yang terdeteksi — cache untuk `info()` tanpa membuka kamera.
    last_model: std::sync::Mutex<Option<String>>,
    /// (folder, name, waktu) file yang sudah di-download host — dipakai menekan
    /// echo `NewFile` dari `capture()` sendiri (cap 16, prune 60 detik).
    downloads: std::sync::Mutex<VecDeque<(String, String, Instant)>>,
    /// Waktu `capture()` host terakhir — `NewFile` dalam 2,5 s sesudahnya
    /// dianggap echo dan diabaikan.
    last_host_capture: std::sync::Mutex<Option<Instant>>,
}

impl GPhotoCamera {
    pub fn new() -> GpResult<Self> {
        Ok(Self {
            inner: std::sync::Mutex::new(None),
            last_model: std::sync::Mutex::new(None),
            downloads: std::sync::Mutex::new(VecDeque::new()),
            last_host_capture: std::sync::Mutex::new(None),
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

    /// Catat file yang barusan di-download host — echo `NewFile`-nya diabaikan.
    fn record_download(&self, folder: &str, name: &str) {
        let mut downloads = self.downloads.lock().unwrap();
        let now = Instant::now();
        downloads.retain(|(_, _, at)| now.duration_since(*at) < Duration::from_secs(60));
        downloads.push_back((folder.to_string(), name.to_string(), now));
        while downloads.len() > 16 {
            downloads.pop_front();
        }
    }

    /// Apakah event `NewFile` ini echo dari capture/download host sendiri?
    fn is_self_echo(&self, folder: &str, name: &str) -> bool {
        if self
            .downloads
            .lock()
            .unwrap()
            .iter()
            .any(|(f, n, _)| f == folder && n == name)
        {
            return true;
        }
        // Blackout sesudah capture host: echo bisa muncul dengan nama beda
        // (mode kartu+host) atau lewat urutan CaptureComplete → NewFile.
        self.last_host_capture
            .lock()
            .unwrap()
            .is_some_and(|at| at.elapsed() < SELF_ECHO_BLACKOUT)
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
            *self.last_host_capture.lock().unwrap() = Some(Instant::now());
            let path = inner.camera.capture_image().wait()?; // masih di kamera
            let file = inner
                .camera
                .fs()
                .download(&path.folder(), &path.name())
                .wait()?;
            let jpeg = file.get_data(&inner.context).wait()?.to_vec();
            self.record_download(&path.folder(), &path.name());
            Ok(jpeg)
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

    /// Drain event kamera (nyaris non-blocking): tekanan tombol shutter di body
    /// muncul sebagai `NewFile`. Tiap iterasi menunggu `WAIT_EVENT_SLICE` saja,
    /// jadi mutex kamera hanya dipegan sebentar dan preview MJPEG tetap lancar.
    fn poll_triggers(&self) -> CameraResult<Vec<TriggerEvent>> {
        const MAX_EVENTS_PER_POLL: usize = 8;
        const TRIGGER_COOLDOWN: Duration = Duration::from_secs(1);
        self.with_cam(|inner| {
            let mut triggers = Vec::new();
            let mut last_emit = None::<Instant>;
            for _ in 0..MAX_EVENTS_PER_POLL {
                match inner.camera.wait_event(WAIT_EVENT_SLICE).wait()? {
                    CameraEvent::Timeout => break,
                    CameraEvent::NewFile(path) => {
                        let (folder, name) =
                            (path.folder().to_string(), path.name().to_string());
                        tracing::debug!("NewFile {folder}/{name}");
                        let lower = name.to_ascii_lowercase();
                        if self.is_self_echo(&folder, &name) {
                            tracing::debug!("echo capture host — diabaikan: {folder}/{name}");
                        } else if !lower.ends_with(".jpg") && !lower.ends_with(".jpeg") {
                            // RAW+JPG → dua NewFile per tekanan; hanya JPEG dihitung
                            tracing::debug!("bukan JPEG ({name}) — diabaikan");
                        } else if last_emit.is_some_and(|at| at.elapsed() < TRIGGER_COOLDOWN) {
                            // burst/bracketing → cukup satu trigger
                        } else {
                            // WAJIB download (drain): di Sony objek RAM terus
                            // dilaporkan selama belum diambil host.
                            let file = inner.camera.fs().download(&folder, &name).wait()?;
                            let jpeg = file.get_data(&inner.context).wait()?.to_vec();
                            self.record_download(&folder, &name);
                            tracing::info!(
                                "trigger shutter: {folder}/{name} ({} KiB)",
                                jpeg.len() / 1024
                            );
                            last_emit = Some(Instant::now());
                            triggers.push(TriggerEvent::Shutter { jpeg });
                        }
                    }
                    other => tracing::debug!("event gphoto2 lain: {other:?}"),
                }
            }
            Ok(triggers)
        })
    }
}
