# Başlangıç ölçümü: f6-after (2026-09-25, 55a0fff)

Intel(R) Xeon(R) Processor @ 2.80GHz, 4 iş parçacığı, 16 GB; Chromium 141.0.7390.37; başsız, WebGL2; `vite preview` (yerel). 5 ölçümün ortancası. Hedefler: docs/adr/0005.

| Ölçüt | Soğuk | Ilık |
|---|---|---|
| Etkileşime hazır (`kentos:interactive`; aralık) | 808 ms (756–819) | 2028 ms (1891–2115) |
| İstek sayısı | 27 | 27 |
| Aktarılan toplam | 1535.9 KB | 3.1 KB |
| JS aktarımı | 243.8 KB | 2.6 KB |
| CSS aktarımı | 19.6 KB | 0.2 KB |
| WASM aktarımı | 1109.7 KB | 0.1 KB |
| Script süresi (ana iş parçacığı) | 96 ms | 127 ms |
| Görev süresi (toplam) | 689 ms | 2716 ms |
| JS yığını | 9 MB | 10 MB |

Aktarım, sunucunun gönderdiği sıkıştırılmış boyuttur (`content-encoding: gzip`). Brotli karşılığı için build envanterine bakın.

İlk soğuk yüklemenin istekleri:

| Yol | Durum | Aktarım | Kodlama |
|---|---|---|---|
| `/` | 200 | 0.8 KB | gzip |
| `/assets/emitter-BgRlvLeE.js` | 200 | 0.5 KB | – |
| `/assets/index-DZH6mKJ1.css` | 200 | 19.6 KB | gzip |
| `/assets/text-NQAA7O1A.js` | 200 | 0.5 KB | – |
| `/assets/signal-BEuxsL1Q.js` | 200 | 0.9 KB | – |
| `/assets/PopupMenu-DXK5ZIuy.js` | 200 | 10.6 KB | gzip |
| `/assets/entities-CxB-9br2.js` | 200 | 1.0 KB | – |
| `/assets/core-Cxr8jddY.js` | 200 | 5.1 KB | gzip |
| `/assets/color-BbnSkwok.js` | 200 | 0.9 KB | gzip |
| `/assets/pack-D9eDrjDN.js` | 200 | 1.8 KB | gzip |
| `/assets/expression-Dw20Clgt.js` | 200 | 1.7 KB | gzip |
| `/assets/crs-DXR5jJaS.js` | 200 | 1.3 KB | gzip |
| `/assets/projectSettings-DR46irdx.js` | 200 | 1.1 KB | gzip |
| `/assets/index-CibOo_FA.js` | 200 | 151.3 KB | gzip |
| `/assets/kentos_geometry_wasm_bg-IYchjEwl.wasm` | 200 | 1109.7 KB | – |
| `/assets/sampleProject-BZSGOHy9.js` | 200 | 5.2 KB | gzip |
| `/assets/showcase-CGLPshhb.js` | 200 | 1.5 KB | gzip |
| `/favicon.svg` | 200 | 0.8 KB | – |
| `/assets/system-CzF_h3uS.js` | 200 | 60.6 KB | gzip |
| `/assets/plus-jakarta-sans-latin-wght-normal-eXO_dkmS.woff2` | 200 | 27.0 KB | – |
| `/assets/plus-jakarta-sans-latin-ext-wght-normal-DmpS2jIq.woff2` | 200 | 21.5 KB | – |
| `/assets/ibm-plex-mono-latin-400-normal-DMJ8VG8y.woff2` | 200 | 14.7 KB | – |
| `/assets/barlow-latin-ext-500-normal-DOaysfXq.woff2` | 200 | 14.4 KB | – |
| `/assets/barlow-latin-ext-600-normal-B8NK_A3D.woff2` | 200 | 14.9 KB | – |
| `/assets/barlow-latin-500-normal-BPAOfeC8.woff2` | 200 | 21.8 KB | – |
| `/assets/barlow-latin-600-normal-DILqtrty.woff2` | 200 | 22.6 KB | – |
| `/assets/barlow-latin-400-italic-BjGuzOss.woff2` | 200 | 24.2 KB | – |
