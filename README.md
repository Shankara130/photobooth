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
cargo run --release   # mode mock — semua fitur tanpa hardware

CAMERA_BACKEND=gphoto TRIGGER_MODE=shot cargo run --release   # mode DSLR + tombol shutter

CAMERA_BACKEND=browser cargo run --release   # mode iPad (lihat langkah iPad di bawah)
```

> Selalu pakai `--release` untuk pemakaian sungguhan — komposisi strip dari foto
> 17 MP butuh puluhan detik di build debug vs <1 s di release (kualitas output
> identik).

Buka `http://localhost:8090` di Mac, atau dari iPad/browser di LAN pakai IP
yang tercetak saat startup.

### Mode iPad (kamera iPad)

Safari hanya mengizinkan akses kamera (`getUserMedia`) pada secure context,
jadi iPad **wajib** membuka lewat HTTPS:

1. Jalankan `CAMERA_BACKEND=browser cargo run`
2. Di iPad buka `https://<ip-mac>:8443`
3. Accept peringatan sertifikat self-signed (sekali)
4. Izinkan akses kamera → MULAI

QR code hasil foto memakai origin HTTP agar tamu bisa scan dari HP tanpa perlu
percaya sertifikat. Origin diambil dari `QR_ORIGIN` bila di-set, kalau tidak dari
IP LAN yang dideteksi server saat startup — jadi QR tetap valid walau layar
kiosk dibuka lewat `localhost`. Jika app di-host di belakang reverse proxy /
home server, set `QR_ORIGIN` ke alamat yang bisa dijangkau tamu.

### Akses dari luar jaringan (Tailscale Funnel)

Server menyediakan listener statis `127.0.0.1:8091` yang melayani **hanya**
`/data` (foto & strip) — tanpa API. Terbitkan ke internet dengan:

```sh
tailscale funnel --bg 8091
# lalu jalankan app dengan QR_ORIGIN=https://<host-mu>.<tailnet>.ts.net
```

Tamu bisa scan QR dari jaringan mana pun (termasuk seluler) lewat URL
`https://...ts.net` bersertifikat valid, tanpa port-forwarding. Kamera, API,
dan UI tetap tidak terekspos. Matikan publikasi dengan `tailscale funnel reset`.

### Penyimpanan di server lain (storage Mac tidak penuh)

```sh
SYNC_TARGET=shankara-server:photobooth-data \
LOCAL_RETENTION_DAYS=7 \
CAMERA_BACKEND=gphoto TRIGGER_MODE=shot cargo run --release
```

- `SYNC_TARGET` — file baru di `photos/` + `strips/` disalin ke server via
  scp/SSH setiap ±2 menit (butuh akses SSH tanpa password — `ssh-copy-id`;
  server tidak perlu rsync). Run pertama mengarsipkan seluruh isi yang ada;
  server jadi arsip permanen dan tidak pernah dihapus dari sana.
- `LOCAL_RETENTION_DAYS` — file lokal lebih tua dari N hari dihapus otomatis
  (sekali per jam). QR yang lebih tua dari retensi akan mati — arsip di server
  tetap lengkap.

## Pemicu shutter kamera (TRIGGER_MODE)

Alih-alih (atau selain) tombol MULAI di layar, sesi bisa dipicu dari tombol
shutter fisik di body DSLR — server memantau event `NewFile` libgphoto2 lewat
`wait_event`, lalu mendorongnya ke browser via SSE `/api/events`.

| Mode    | Perilaku                                                                             |
| ------- | ------------------------------------------------------------------------------------ |
| `off`   | default — tidak ada pemantauan, perilaku identik versi lama                           |
| `start` | tekan shutter = MULAI: countdown ×N lalu capture dari host. Foto dari tekanan pemicu itu diunduh lalu dibuang (agar objek tidak tersangkut di kamera) |
| `shot`  | tiap tekanan shutter = 1 foto langsung dari kamera (tanpa countdown); setelah `SHOTS_PER_STRIP` foto, strip dikomposisi |

Tombol MULAI di layar **selalu** menjalankan sesi otomatis (countdown ×N, capture
dari host) di mode apa pun — mode hanya menentukan perilaku tombol shutter kamera.

```sh
CAMERA_BACKEND=gphoto TRIGGER_MODE=shot cargo run
```

Catatan:

- Mode RAW+JPG di kamera menghasilkan dua event per tekanan — hanya JPEG
  dihitung; RAW diabaikan.
- Saat foto full-res sedang diunduh (1–4 s), live preview sesaat tertahan.
- Koneksi SSE yang sedang reconnect bisa melewatkan satu trigger — tamu tinggal
  menekan tombol lagi.
- **Perilaku tombol body saat tethering tergantung merek/model.** Untuk diagnosis:
  `RUST_LOG=photobooth=debug` — setiap event kamera (`NewFile`, `CaptureComplete`,
  dll) tercetak. Kalau tombol body ternyata dimatikan firmware, alternatif yang
  selalu jalan: tombol USB/arcade yang dikenali sebagai keyboard (HID).

### Catatan khusus Sony ZV-1 (teruji)

- Mode USB kamera **harus "PC Remote"** — cek dengan `gphoto2 --auto-detect`,
  harus tertulis `Sony ZV-1 (PC Control)`. Di mode MTP shutter body diblokir
  kamera ("USB MODE MTP") dan event tidak pernah keluar.
- **Nonaktifkan extension webcam Sony** dulu: System Settings → General →
  Login Items & Extensions → Camera Extensions → matikan Imaging Edge Webcam.
  Selama aktif, kamera selalu diubah jadi webcam (UVC) dan gphoto2 tidak
  mendeteksinya.
- Di mode PC Remote, hasil tombol shutter langsung dikirim ke komputer —
  layar kamera tidak menampilkan review foto; itu normal.
- Di mode PC Control, capture dari host juga didukung (`No Image Capture`
  hanya berlaku di mode MTP) — tombol MULAI/`start` dan tombol shutter
  dua-duanya jalan.

Testing tanpa DSLR (backend mock):

```sh
CAMERA_BACKEND=mock TRIGGER_MODE=shot cargo run
# terminal lain:
curl -N localhost:8090/api/events      # pantau SSE
curl -X POST localhost:8090/api/trigger/inject   # ×N → strip terkomposisi di browser
```

## Environment

| Variabel           | Default  | Keterangan                                  |
| ------------------ | -------- | ------------------------------------------- |
| `CAMERA_BACKEND`   | `mock`   | `mock` \| `gphoto` \| `browser`             |
| `TRIGGER_MODE`     | `off`    | `off` \| `start` \| `shot` — pemicu tombol shutter kamera (lihat bagian khusus; butuh `CAMERA_BACKEND=gphoto`) |
| `PORT`             | `8090`   | port HTTP                                   |
| `TLS_PORT`         | `8443`   | port HTTPS (0 = matikan)                    |
| `QR_ORIGIN`        | (auto)   | origin link QR, mis. `http://192.168.1.10:8090` atau alamat home server — default: IP LAN yang dideteksi server |
| `PUBLIC_PORT`      | `8091`   | port server statis khusus funnel (hanya `/data`; 0 = matikan) |
| `SYNC_TARGET`      | (kosong) | arsip otomatis file baru via scp/SSH, mis. `user@server:photobooth-data` (tanpa perlu rsync di server) |
| `LOCAL_RETENTION_DAYS` | `0`  | hapus foto/strip lokal lebih tua dari N hari (0 = jangan hapus; arsip server tetap utuh) |
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
| GET    | `/api/events`        | SSE stream pemicu shutter (`event: trigger`) |
| POST   | `/api/trigger/inject`| injeksi pemicu sintetis — testing (400 saat `TRIGGER_MODE=off`) |
| POST   | `/api/capture`       | capture dari kamera server                 |
| POST   | `/api/photos/upload` | upload JPEG dari klien (multipart `photo`) |
| GET    | `/api/photos`        | daftar foto & strip                        |
| POST   | `/api/strip`         | `{ids:[...]}` → susun photo strip          |
| GET    | `/api/qr?u=<url>`    | QR code PNG untuk URL                      |
