'use strict';

// Photobooth — state machine flow:
// boot → home → countdown → flash → capture → review → (ulang N) → composing → result → home

const $ = (id) => document.getElementById(id);

const state = {
  info: null,
  stream: null, // MediaStream mode browser
  shots: [],    // MediaMeta sesi berjalan
  busy: false,
};

const LIVE_SCREENS = new Set(['home', 'countdown', 'capturing', 'review']);
const sleep = (ms) => new Promise((r) => setTimeout(r, ms));

function show(id) {
  for (const s of document.querySelectorAll('.screen')) {
    s.classList.toggle('active', s.id === id);
  }
  const needLive = LIVE_SCREENS.has(id);
  $('live').hidden = !needLive;
  // matikan stream MJPEG di layar solid — hemat bandwidth & kamera
  if (!needLive) stopMjpeg();
  else if (state.info?.features.serverPreview && !$('mjpg').getAttribute('src')) startMjpeg();
}

function startMjpeg() {
  $('mjpg').src = '/api/preview.mjpg';
  $('mjpg').hidden = false;
}

function stopMjpeg() {
  $('mjpg').removeAttribute('src');
  $('mjpg').hidden = true;
}

function toast(msg, ms = 4000) {
  const t = $('toast');
  t.textContent = msg;
  t.hidden = false;
  clearTimeout(toast._t);
  toast._t = setTimeout(() => (t.hidden = true), ms);
}

function flash() {
  const f = $('flash');
  f.classList.remove('go');
  void f.offsetWidth; // restart animasi
  f.classList.add('go');
}

async function errText(resp) {
  try {
    return (await resp.json()).error || `HTTP ${resp.status}`;
  } catch {
    return `HTTP ${resp.status}`;
  }
}

async function init() {
  try {
    const resp = await fetch('/api/info');
    state.info = await resp.json();
  } catch (e) {
    document.querySelector('#boot .muted').textContent = 'Server tidak terjangkau: ' + e.message;
    return;
  }
  setupPreview();
  $('home-status').textContent =
    `Mode ${state.info.backend.toUpperCase()} · ${state.info.shotsPerStrip} foto per strip`;
  show('home');
}

function setupPreview() {
  const f = state.info.features;
  if (f.serverPreview) {
    startMjpeg();
  } else if (f.clientPreview) {
    startCamera();
  } else {
    $('placeholder').hidden = false;
  }
}

async function startCamera() {
  try {
    state.stream = await navigator.mediaDevices.getUserMedia({
      video: { width: { ideal: 4032 }, height: { ideal: 3024 }, facingMode: 'user' },
      audio: false,
    });
    const v = $('cam');
    v.srcObject = state.stream;
    v.hidden = false;
  } catch (e) {
    toast('Kamera tidak bisa diakses: ' + e.message + ' — di iPad wajib lewat HTTPS.');
    $('placeholder').hidden = false;
  }
}

function countdown(from, label) {
  return new Promise((resolve) => {
    show('countdown');
    $('count-label').textContent = label;
    let k = from;
    const el = $('count');
    const tick = () => {
      if (k === 0) return resolve();
      el.textContent = k;
      el.classList.remove('pop');
      void el.offsetWidth;
      el.classList.add('pop');
      k -= 1;
      setTimeout(tick, 1000);
    };
    tick();
  });
}

async function captureServer() {
  show('capturing');
  const resp = await fetch('/api/capture', { method: 'POST' });
  if (!resp.ok) throw new Error(await errText(resp));
  return resp.json();
}

async function captureClient() {
  const v = $('cam');
  if (!v.videoWidth) throw new Error('kamera belum siap');
  // freeze frame sinkron SEBELUM flash/upload — video tetap hidup di balik overlay
  const canvas = document.createElement('canvas');
  canvas.width = v.videoWidth;
  canvas.height = v.videoHeight;
  canvas.getContext('2d').drawImage(v, 0, 0);
  const blob = await new Promise((res) => canvas.toBlob(res, 'image/jpeg', 0.9));
  show('capturing');
  const fd = new FormData();
  fd.append('photo', blob, 'photo.jpg');
  const resp = await fetch('/api/photos/upload', { method: 'POST', body: fd });
  if (!resp.ok) throw new Error(await errText(resp));
  return resp.json();
}

function addThumb(meta) {
  const img = document.createElement('img');
  img.src = meta.url;
  $('thumbs').appendChild(img);
}

async function startSession() {
  if (state.busy) return;
  state.busy = true;
  state.shots = [];
  $('thumbs').innerHTML = '';
  try {
    for (let i = 1; i <= state.info.shotsPerStrip; i++) {
      await countdown(3, `Foto ${i} dari ${state.info.shotsPerStrip}`);
      flash();
      const meta = state.info.features.serverCapture
        ? await captureServer()
        : await captureClient();
      state.shots.push(meta);
      addThumb(meta);
      show('review');
      $('review-note').textContent =
        i < state.info.shotsPerStrip ? 'Hebat! Bersiap lagi…' : 'Selesai — menyusun strip…';
      await sleep(1200);
    }
    show('composing');
    const resp = await fetch('/api/strip', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ ids: state.shots.map((s) => s.id) }),
    });
    if (!resp.ok) throw new Error(await errText(resp));
    showResult(await resp.json());
  } catch (e) {
    toast('Gagal: ' + e.message);
    show('home');
  } finally {
    state.busy = false;
  }
}

function qrLinkFor(meta) {
  // tamu scan dari HP — origin HTTP cukup (tak perlu percaya sertifikat self-signed)
  const host = location.hostname || 'localhost';
  const port = state.info.port || location.port;
  return `http://${host}:${port}${meta.url}`;
}

function showResult(meta) {
  $('strip-img').src = meta.url;
  const dl = $('btn-download');
  dl.href = meta.url;
  dl.download = `photobooth-${meta.id}.jpg`;
  $('qr-img').src = '/api/qr?u=' + encodeURIComponent(qrLinkFor(meta));
  show('result');
}

async function openGallery() {
  try {
    const resp = await fetch('/api/photos');
    const { items } = await resp.json();
    const grid = $('gallery-grid');
    grid.innerHTML = '';
    for (const it of items.slice(0, 60)) {
      const a = document.createElement('a');
      a.href = it.url;
      a.download = `${it.kind}-${it.id}.jpg`;
      if (it.kind === 'strip') a.classList.add('strip');
      const img = document.createElement('img');
      img.src = it.url;
      img.loading = 'lazy';
      a.appendChild(img);
      grid.appendChild(a);
    }
    show('gallery');
  } catch (e) {
    toast('Galeri gagal dimuat: ' + e.message);
  }
}

document.addEventListener('DOMContentLoaded', () => {
  $('btn-start').addEventListener('click', startSession);
  $('btn-gallery').addEventListener('click', openGallery);
  $('btn-again').addEventListener('click', () => show('home'));
  $('btn-home').addEventListener('click', () => show('home'));
  $('btn-gallery-back').addEventListener('click', () => show('home'));
  init();
});
