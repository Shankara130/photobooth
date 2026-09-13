//! Komposisi photo strip klasik — tumpukan vertikal frame 4:5, latar putih,
//! footer tanggal blocky (tanpa dependency font).

use anyhow::{ensure, Context as _, Result};
use image::{imageops, ImageDecoder, Rgb, RgbImage};

pub const STRIP_W: u32 = 600;
pub const MARGIN: u32 = 24;
pub const GAP: u32 = 16;
pub const FRAME_W: u32 = STRIP_W - 2 * MARGIN; // 552
pub const FRAME_H: u32 = 690; // rasio portrait 4:5
pub const FOOTER_H: u32 = 88;

/// Susun frames JPEG jadi strip vertikal, kembalikan bytes JPEG (q90).
pub fn compose_strip(frames: &[Vec<u8>], taken: chrono::DateTime<chrono::Local>) -> Result<Vec<u8>> {
    ensure!(!frames.is_empty() && frames.len() <= 8, "jumlah frame harus 1..=8");

    let n = frames.len() as u32;
    let height = MARGIN + n * FRAME_H + (n - 1) * GAP + MARGIN + FOOTER_H;
    let mut canvas = RgbImage::from_pixel(STRIP_W, height, Rgb([255, 255, 255]));

    for (i, jpeg) in frames.iter().enumerate() {
        let frame = decode_frame(jpeg)?;
        let y = MARGIN + i as u32 * (FRAME_H + GAP);
        imageops::overlay(&mut canvas, &frame, MARGIN as i64, y as i64);
    }

    // Footer: garis pemisah tipis + tanggal blocky di tengah
    let rule_y = height - FOOTER_H + 20;
    for x in MARGIN..STRIP_W - MARGIN {
        canvas.put_pixel(x, rule_y, Rgb([190, 190, 190]));
    }
    let text = taken.format("%Y-%m-%d %H:%M").to_string();
    crate::textblocks::draw_text_centered(&mut canvas, &text, rule_y + 14, 3, 4, [130, 130, 130]);

    let mut buf = Vec::new();
    let mut encoder = image::codecs::jpeg::JpegEncoder::new_with_quality(&mut buf, 90);
    encoder
        .encode_image(&canvas)
        .context("encode strip ke JPEG")?;
    Ok(buf)
}

/// Decode frame JPEG (hormati EXIF orientation) → RgbImage FRAME_W×FRAME_H.
fn decode_frame(jpeg: &[u8]) -> Result<RgbImage> {
    let reader = image::ImageReader::new(std::io::Cursor::new(jpeg))
        .with_guessed_format()
        .context("format frame tidak dikenali")?;
    let mut decoder = reader.into_decoder().context("decode frame JPEG")?;
    let orientation = decoder
        .orientation()
        .unwrap_or(image::metadata::Orientation::NoTransforms);
    let mut img = image::DynamicImage::from_decoder(decoder).context("decode frame JPEG")?;
    img.apply_orientation(orientation);
    Ok(center_crop_resize(img.to_rgb8()))
}

/// Center-crop ke rasio frame (4:5) lalu resize ke FRAME_W×FRAME_H.
fn center_crop_resize(img: RgbImage) -> RgbImage {
    let (w, h) = (img.width(), img.height());
    let target = FRAME_W as f64 / FRAME_H as f64;
    let cur = w as f64 / h as f64;
    let (cw, ch) = if cur > target {
        // terlalu lebar → crop kiri-kanan
        ((h as f64 * target).round() as u32, h)
    } else {
        // terlalu tinggi → crop atas-bawah
        (w, (w as f64 / target).round() as u32)
    };
    let cropped = imageops::crop_imm(&img, (w - cw) / 2, (h - ch) / 2, cw, ch);
    imageops::resize(
        &cropped.to_image(),
        FRAME_W,
        FRAME_H,
        imageops::FilterType::Lanczos3,
    )
}
