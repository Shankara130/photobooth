//! Connector mock — generate foto pola test tanpa hardware.
//! Untuk development & demo: seluruh app jalan tanpa DSLR/iPad.

use super::CameraResult;
use image::{Rgb, RgbImage};

pub struct MockCamera {
    counter: std::sync::atomic::AtomicU64,
}

impl MockCamera {
    pub fn new() -> Self {
        Self {
            counter: std::sync::atomic::AtomicU64::new(0),
        }
    }

    fn draw_frame(&self, n: u64, w: u32, h: u32, label: &str) -> Vec<u8> {
        let mut img = RgbImage::new(w, h);

        // Gradasi warna berganti tiap capture
        let r = (n * 60 % 256) as u8;
        let g = (n * 30 % 256) as u8;
        let b = (255 - n * 45 % 256) as u8;

        for (x, y, px) in img.enumerate_pixels_mut() {
            // pola garis vertikal + gradasi biar kelihatan "hidup"
            let stripe = if (x / 40 + y / 40) % 2 == 0 { 30 } else { 0 };
            *px = Rgb([r.saturating_add(stripe), g.saturating_add(stripe), b]);
        }

        // Label capture di tengah (blocky, tanpa font dependency)
        let text = format!("{label} #{n}");
        let y = img.height() / 2 - 30;
        crate::textblocks::draw_text_centered(&mut img, &text, y, 8, 12, [255, 255, 255]);

        let mut buf = std::io::Cursor::new(Vec::new());
        img.write_to(&mut buf, image::ImageFormat::Jpeg).expect("encode jpeg");
        buf.into_inner()
    }
}

impl super::Camera for MockCamera {
    fn name(&self) -> &'static str {
        "mock"
    }

    fn available(&self) -> bool {
        true
    }

    fn capture(&self) -> CameraResult<Vec<u8>> {
        let n = self
            .counter
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        Ok(self.draw_frame(n, 1920, 1280, "CAPTURE"))
    }

    fn preview(&self) -> CameraResult<Vec<u8>> {
        let n = self
            .counter
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        Ok(self.draw_frame(n, 960, 640, "LIVE"))
    }

    fn info(&self) -> serde_json::Value {
        serde_json::json!({
            "backend": "mock",
            "available": true,
            "captures": self.counter.load(std::sync::atomic::Ordering::Relaxed),
        })
    }
}
