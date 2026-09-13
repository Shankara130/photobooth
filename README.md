# Photobooth — DSLR/iPad USB Tethering (Rust)

Web app photobooth berbasis Rust (axum + libgphoto2). Mac/server men-tether DSLR
lewat USB; iPad atau browser lain di LAN dipakai sebagai layar — atau sebagai
kamera itu sendiri. Output: photo strip klasik + foto satuan, distribusi via
tombol unduh dan QR code.

## Mode kamera (`CAMERA_BACKEND`)

| Mode         | Nilai    | Kamera                        | Preview                          |
| ------------ | -------- | ----------------------------- | -------------------------------- |
| Mock (default) | `mock`   | gambar pola test              | MJPEG pola test dari server      |
| DSLR USB     | `gphoto` | DSLR via libgphoto2 (USB)     | MJPEG live dari `capture_preview` |
| iPad         | `browser`| kamera iPad (getUserMedia)    | langsung di iPad, upload ke server |

## Prasyarat

- Rust (rustup)
- libgphoto2: `brew install libgphoto2`
- libclang (bindgen): sudah ada di Xcode Command Line Tools; bila build gagal:
  `export LIBCLANG_PATH=/Library/Developer/CommandLineTools/usr/lib`

## Menjalankan

```sh
cargo run                       # mode mock — semua fitur tanpa hardware

CAMERA_BACKEND=gphoto cargo run # mode DSLR (tancapkan DSLR via USB)

CAMERA_BACKEND=browser cargo run # mode iPad (lihat langkah iPad di bawah)
```

Buka `http://localhost:8080` di Mac, atau dari iPad/browser di LAN pakai IP
yang tercetak saat startup.

### Mode iPad (kamera iPad)

Safari hanya mengizinkan akses kamera (`getUserMedia`) pada secure context,
jadi iPad **wajib** membuka lewat HTTPS:

1. Jalankan `CAMERA_BACKEND=browser cargo run`
2. Di iPad buka `https://<ip-mac>:8443`
3. Accept peringatan sertifikat self-signed (sekali)
4. Izinkan akses kamera → MULAI

QR code hasil foto memakai origin HTTP (`:8080`) agar tamu bisa scan dari HP
tanpa perlu percaya sertifikat.

## Environment

| Variabel           | Default  | Keterangan                                  |
| ------------------ | -------- | ------------------------------------------- |
| `CAMERA_BACKEND`   | `mock`   | `mock` \| `gphoto` \| `browser`             |
| `PORT`             | `8080`   | port HTTP                                   |
| `TLS_PORT`         | `8443`   | port HTTPS (0 = matikan)                    |
| `SHOTS_PER_STRIP`  | `4`      | jumlah foto per strip (1..8)                |
| `DATA_DIR`         | `./data` | penyimpanan foto/strip/sertifikat           |
| `WEB_DIR`          | `./web`  | aset frontend                               |
| `RUST_LOG`         | `info`   | level log tracing                           |

## Alur pemakaian

MULAI → countdown 3-2-1 → flash → capture (×`SHOTS_PER_STRIP`) → strip vertikal
(bidang putih, footer tanggal) → unduh / scan QR / GALERI.

## Struktur data

```
data/
  photos/   # foto satuan (uuid.jpg)
  strips/   # photo strip hasil komposisi
  certs/    # sertifikat self-signed HTTPS
```

## Endpoint API

| Method | Path                 | Fungsi                                     |
| ------ | -------------------- | ------------------------------------------ |
| GET    | `/api/info`          | mode backend + kemampuan                   |
| GET    | `/api/preview.mjpg`  | live preview MJPEG (gphoto/mock)           |
| POST   | `/api/capture`       | capture dari kamera server                 |
| POST   | `/api/photos/upload` | upload JPEG dari klien (multipart `photo`) |
| GET    | `/api/photos`        | daftar foto & strip                        |
| POST   | `/api/strip`         | `{ids:[...]}` → susun photo strip          |
| GET    | `/api/qr?u=<url>`    | QR code PNG untuk URL                      |
