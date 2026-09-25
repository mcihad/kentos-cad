# Başlangıç ölçümü: wasm-compressed (2026-09-25, 1e57a85)

Intel(R) Xeon(R) Processor @ 2.80GHz, 4 iş parçacığı, 16 GB; Chromium 141.0.7390.37; başsız, WebGL2; `vite preview` (yerel). 3 ölçümün ortancası. Hedefler: docs/adr/0005.

| Ölçüt | Soğuk | Ilık |
|---|---|---|
| Etkileşime hazır (`kentos:interactive`; aralık) | 830 ms (778–833) | 2274 ms (2175–2286) |
| İstek sayısı | 27 | 27 |
| Aktarılan toplam | 682.6 KB | 0.8 KB |
| JS aktarımı | 204.5 KB | 0.7 KB |
| CSS aktarımı | 17.1 KB | 0.0 KB |
| WASM aktarımı | 298.3 KB | 0.0 KB |
| Script süresi (ana iş parçacığı) | 122 ms | 158 ms |
| Görev süresi (toplam) | 776 ms | 3040 ms |
| JS yığını | 9 MB | 15 MB |

Aktarım, sunucunun gönderdiği sıkıştırılmış boyuttur (`content-encoding: gzip, br`). Brotli karşılığı için build envanterine bakın.

İlk soğuk yüklemenin istekleri:

| Yol | Durum | Aktarım | Kodlama |
|---|---|---|---|
| `/` | 200 | 0.8 KB | gzip |
| `/assets/index-6zzzaVzp.css` | 200 | 17.1 KB | br |
| `/assets/text-NQAA7O1A.js` | 200 | 0.5 KB | – |
| `/assets/emitter-BgRlvLeE.js` | 200 | 0.5 KB | – |
| `/assets/core-DwucVzgu.js` | 200 | 4.8 KB | br |
| `/assets/index-C92eBV0I.js` | 200 | 127.6 KB | br |
| `/assets/entities-DwIRlwRt.js` | 200 | 1.0 KB | – |
| `/assets/pack-D9eDrjDN.js` | 200 | 1.6 KB | br |
| `/assets/color-BbnSkwok.js` | 200 | 0.8 KB | br |
| `/assets/PopupMenu-DXK5ZIuy.js` | 200 | 9.1 KB | br |
| `/assets/expression-DS2qfcOR.js` | 200 | 1.5 KB | br |
| `/assets/signal-BEuxsL1Q.js` | 200 | 0.9 KB | – |
| `/assets/projectSettings-DR46irdx.js` | 200 | 0.9 KB | br |
| `/assets/crs-DXR5jJaS.js` | 200 | 1.1 KB | br |
| `/assets/kentos_geometry_wasm_bg-DCuHYXN7.wasm` | 200 | 298.3 KB | br |
| `/assets/sampleProject-DhqBOGvE.js` | 200 | 4.8 KB | br |
| `/assets/showcase-CGLPshhb.js` | 200 | 1.4 KB | br |
| `/assets/system-CzF_h3uS.js` | 200 | 48.1 KB | br |
| `/favicon.svg` | 200 | 0.8 KB | – |
| `/assets/plus-jakarta-sans-latin-wght-normal-eXO_dkmS.woff2` | 200 | 27.0 KB | – |
| `/assets/plus-jakarta-sans-latin-ext-wght-normal-DmpS2jIq.woff2` | 200 | 21.5 KB | – |
| `/assets/ibm-plex-mono-latin-400-normal-DMJ8VG8y.woff2` | 200 | 14.7 KB | – |
| `/assets/barlow-latin-ext-500-normal-DOaysfXq.woff2` | 200 | 14.4 KB | – |
| `/assets/barlow-latin-500-normal-BPAOfeC8.woff2` | 200 | 21.8 KB | – |
| `/assets/barlow-latin-ext-600-normal-B8NK_A3D.woff2` | 200 | 14.9 KB | – |
| `/assets/barlow-latin-600-normal-DILqtrty.woff2` | 200 | 22.6 KB | – |
| `/assets/barlow-latin-400-italic-BjGuzOss.woff2` | 200 | 24.2 KB | – |
