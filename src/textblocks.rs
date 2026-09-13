//! Renderer teks blocky 8×5 — dipakai mock camera & footer photo strip,
//! tanpa dependency font eksternal.

use image::{Rgb, RgbImage};

/// Lebar sel per karakter (8 px glyph + 2 px spasi) pada skala 1.
const CELL_W: u32 = 10;

pub fn text_width(text: &str, scale_w: u32) -> u32 {
    text.chars().count() as u32 * CELL_W * scale_w
}

/// Gambar teks mulai di (x_start, y); tiap karakter = grid 8×5 diperbesar
/// `scale_w × scale_h`. Bagian yang keluar kanvas dilewati.
pub fn draw_text(
    img: &mut RgbImage,
    text: &str,
    x_start: u32,
    y: u32,
    scale_w: u32,
    scale_h: u32,
    color: [u8; 3],
) {
    let mut x = x_start;
    for ch in text.chars() {
        let glyph = glyph_bits(ch);
        for (gy, row) in glyph.iter().enumerate() {
            for gx in 0..8u32 {
                if row >> (7 - gx) & 1 == 1 {
                    for sy in 0..scale_h {
                        for sx in 0..scale_w {
                            let px = x + gx * scale_w + sx;
                            let py = y + gy as u32 * scale_h + sy;
                            if px < img.width() && py < img.height() {
                                img.put_pixel(px, py, Rgb(color));
                            }
                        }
                    }
                }
            }
        }
        x = x.saturating_add(CELL_W * scale_w);
    }
}

/// Gambar teks di tengah horizontal kanvas.
pub fn draw_text_centered(
    img: &mut RgbImage,
    text: &str,
    y: u32,
    scale_w: u32,
    scale_h: u32,
    color: [u8; 3],
) {
    let x0 = img.width().saturating_sub(text_width(text, scale_w)) / 2;
    draw_text(img, text, x0, y, scale_w, scale_h, color);
}

/// Bitmap 8×5 minimal untuk angka, huruf, tanda baca (row-major, 5 rows;
/// bit paling kiri = kolom kiri glyph).
fn glyph_bits(ch: char) -> [u8; 5] {
    match ch {
        '0' => [0x7C, 0x82, 0x82, 0x82, 0x7C],
        '1' => [0x10, 0x30, 0x10, 0x10, 0x7C],
        '2' => [0x7C, 0x02, 0x7C, 0x80, 0xFE],
        '3' => [0x7C, 0x02, 0x3C, 0x02, 0x7C],
        '4' => [0x82, 0x82, 0xFE, 0x02, 0x02],
        '5' => [0xFE, 0x80, 0xFC, 0x02, 0xFC],
        '6' => [0x7C, 0x80, 0xFC, 0x82, 0x7C],
        '7' => [0xFE, 0x02, 0x04, 0x08, 0x10],
        '8' => [0x7C, 0x82, 0x7C, 0x82, 0x7C],
        '9' => [0x7C, 0x82, 0x7E, 0x02, 0x7C],
        'A' => [0x7C, 0x82, 0xFE, 0x82, 0x82],
        'B' => [0xFC, 0x82, 0xFC, 0x82, 0xFC],
        'C' => [0x7C, 0x82, 0x80, 0x82, 0x7C],
        'E' => [0xFE, 0x80, 0xF8, 0x80, 0xFE],
        'I' => [0xFE, 0x10, 0x10, 0x10, 0xFE],
        'L' => [0x80, 0x80, 0x80, 0x80, 0xFE],
        'M' => [0x82, 0xC6, 0xAA, 0x92, 0x82],
        'N' => [0x82, 0xC2, 0xA2, 0x92, 0x86],
        'P' => [0xFC, 0x82, 0xFC, 0x80, 0x80],
        'R' => [0xFC, 0x82, 0xFC, 0x90, 0x86],
        'T' => [0xFE, 0x10, 0x10, 0x10, 0x10],
        'U' => [0x82, 0x82, 0x82, 0x82, 0x7C],
        'V' => [0x82, 0x82, 0x44, 0x44, 0x38],
        '-' => [0x00, 0x00, 0x7C, 0x00, 0x00],
        ':' => [0x00, 0x18, 0x00, 0x18, 0x00],
        '#' => [0x14, 0x7F, 0x14, 0x7F, 0x14],
        ' ' => [0x00, 0x00, 0x00, 0x00, 0x00],
        _ => [0x7C, 0x82, 0x82, 0x82, 0x7C],
    }
}
