# Masaüstünün karesi büyük çizimde: before (2026-09-28, 1b72b1e, kaydedilmemiş değişiklikle)

11th Gen Intel(R) Core(TM) i5-11300H @ 3.10GHz, 8 iş parçacığı, 15 GB; Ubuntu 26.04.1 LTS (7.0.0-34-generic); rustc 1.96.0 (ac68faa20 2026-05-25), `--release` (lto thin). Pencere 1440×900, çizici wgpu. Test `apps/desktop/src/perf/frame.rs`: arayüz pencere açmadan Iced'in çalışma zamanının sırasıyla sürülür. “Olay” bileşenlerin olayı alması, “uygulama” mesajların uygulanması, “görünüm” `App::view` (sahne eskidiyse kurulması dahil), “düzen” Iced'in ağacı karşılaştırıp yerleştirmesi, “çizim” bileşenlerin kareyi kaydetmesidir; “MİB” bunların toplamıdır (arayüz iş parçacığı). “GPU” karenin GPU'da hazırlanıp çizilmesi ve resmin geri okunmasıdır (yalnız wgpu ile; geri okuma pencerede yoktur).

Çizim: parsel başına 20 köşeli bir alan, üç öznitelik ve etiket, tek katman (`perf.rs`'in çizimi), bütünü pencereye sığmış. Süreler milisaniye; parçalar ortanca.

| Parsel | Durum | Kare | MİB p50 | MİB p95 | MİB en çok | Olay | Uygulama | Görünüm | Düzen | Çizim | GPU p50 |
|---|---|---|---|---|---|---|---|---|---|---|---|
| 0 | Boş kare (yeniden çizim) | 30 | 0.68 | 0.80 | 0.85 | 0.00 | 0.00 | 0.23 | 0.29 | 0.15 | 13.78 |
| 0 | İmleç çizimin üstünde (seçim aracı) | 120 | 0.53 | 0.62 | 0.71 | 0.04 | 0.01 | 0.12 | 0.23 | 0.13 | 13.04 |
| 0 | Tekerlekle yakınlaştırma | 32 | 0.57 | 0.90 | 1.14 | 0.05 | 0.01 | 0.13 | 0.23 | 0.15 | 13.39 |
| 0 | Orta tuşla kaydırma | 40 | 0.48 | 0.64 | 0.81 | 0.04 | 0.01 | 0.11 | 0.18 | 0.13 | 12.49 |
| 0 | Çoklu çizgi çizerken imleç (kenet açık) | 120 | 0.61 | 0.66 | 0.73 | 0.05 | 0.02 | 0.13 | 0.26 | 0.14 | 13.56 |
| 0 | Bir nesneyi silme | 1 | 0.62 | 0.62 | 0.62 | 0.00 | 0.02 | 0.19 | 0.27 | 0.15 | 12.34 |
| 0 | Geri al ve yinele (bir nesne) | 10 | 0.52 | 0.53 | 0.53 | 0.00 | 0.00 | 0.19 | 0.21 | 0.11 | 12.52 |
| 0 | Hepsini seç (Ctrl+A) | 1 | 0.67 | 0.67 | 0.67 | 0.00 | 0.00 | 0.21 | 0.31 | 0.15 | 12.59 |
| 0 | İmleç çizimin üstünde (hepsi seçili) | 60 | 0.54 | 0.60 | 0.61 | 0.04 | 0.02 | 0.12 | 0.23 | 0.13 | 13.01 |
| 0 | Seçimi bırakma (Esc) | 1 | 0.48 | 0.48 | 0.48 | 0.05 | 0.02 | 0.12 | 0.18 | 0.11 | 13.16 |
| 10000 | Boş kare (yeniden çizim) | 30 | 0.89 | 1.58 | 1.66 | 0.00 | 0.00 | 0.56 | 0.22 | 0.14 | 22.61 |
| 10000 | İmleç çizimin üstünde (seçim aracı) | 120 | 1.45 | 1.64 | 1.81 | 0.08 | 0.02 | 0.63 | 0.47 | 0.24 | 28.33 |
| 10000 | Tekerlekle yakınlaştırma | 32 | 0.97 | 2.12 | 2.42 | 0.05 | 0.01 | 0.39 | 0.27 | 0.17 | 16.78 |
| 10000 | Orta tuşla kaydırma | 40 | 1.54 | 1.63 | 1.66 | 0.08 | 0.02 | 0.76 | 0.41 | 0.26 | 27.21 |
| 10000 | Çoklu çizgi çizerken imleç (kenet açık) | 120 | 1.53 | 1.85 | 1.95 | 0.08 | 0.03 | 0.64 | 0.52 | 0.25 | 28.76 |
| 10000 | Bir nesneyi silme | 1 | 15.81 | 15.81 | 15.81 | 0.00 | 0.04 | 15.10 | 0.46 | 0.21 | 33.68 |
| 10000 | Geri al ve yinele (bir nesne) | 10 | 11.34 | 13.47 | 13.47 | 0.00 | 0.02 | 10.85 | 0.28 | 0.16 | 24.10 |
| 10000 | Hepsini seç (Ctrl+A) | 1 | 31.34 | 31.34 | 31.34 | 0.00 | 2.93 | 27.78 | 0.44 | 0.18 | 43.13 |
| 10000 | İmleç çizimin üstünde (hepsi seçili) | 60 | 5.83 | 6.23 | 6.44 | 0.06 | 0.02 | 5.33 | 0.27 | 0.12 | 35.70 |
| 10000 | Seçimi bırakma (Esc) | 1 | 3.05 | 3.05 | 3.05 | 0.08 | 0.02 | 0.62 | 0.69 | 1.63 | 24.27 |
| 50000 | Boş kare (yeniden çizim) | 30 | 3.39 | 3.47 | 3.62 | 0.00 | 0.00 | 2.66 | 0.47 | 0.25 | 83.36 |
| 50000 | İmleç çizimin üstünde (seçim aracı) | 120 | 3.46 | 3.58 | 4.00 | 0.08 | 0.17 | 2.45 | 0.52 | 0.24 | 83.28 |
| 50000 | Tekerlekle yakınlaştırma | 32 | 2.63 | 4.49 | 4.80 | 0.09 | 0.02 | 1.74 | 0.49 | 0.28 | 42.16 |
| 50000 | Orta tuşla kaydırma | 40 | 3.90 | 4.29 | 4.58 | 0.08 | 0.02 | 3.04 | 0.45 | 0.27 | 79.70 |
| 50000 | Çoklu çizgi çizerken imleç (kenet açık) | 120 | 3.66 | 3.91 | 4.43 | 0.08 | 0.31 | 2.45 | 0.56 | 0.27 | 83.52 |
| 50000 | Bir nesneyi silme | 1 | 67.72 | 67.72 | 67.72 | 0.00 | 0.06 | 66.87 | 0.54 | 0.24 | 100.44 |
| 50000 | Geri al ve yinele (bir nesne) | 10 | 58.96 | 62.80 | 62.80 | 0.00 | 0.02 | 58.37 | 0.24 | 0.14 | 98.78 |
| 50000 | Hepsini seç (Ctrl+A) | 1 | 140.19 | 140.19 | 140.19 | 0.00 | 17.53 | 122.04 | 0.42 | 0.20 | 245.76 |
| 50000 | İmleç çizimin üstünde (hepsi seçili) | 60 | 30.04 | 30.97 | 36.56 | 0.03 | 0.09 | 29.58 | 0.24 | 0.10 | 137.89 |
| 50000 | Seçimi bırakma (Esc) | 1 | 8.49 | 8.49 | 8.49 | 0.04 | 0.02 | 1.49 | 0.44 | 6.50 | 79.78 |
| 100000 | Boş kare (yeniden çizim) | 30 | 5.77 | 5.98 | 6.11 | 0.00 | 0.00 | 5.02 | 0.48 | 0.25 | 175.57 |
| 100000 | İmleç çizimin üstünde (seçim aracı) | 120 | 5.64 | 5.99 | 6.83 | 0.08 | 0.03 | 4.75 | 0.49 | 0.24 | 173.78 |
| 100000 | Tekerlekle yakınlaştırma | 32 | 5.01 | 6.80 | 7.30 | 0.08 | 0.02 | 4.02 | 0.50 | 0.28 | 102.84 |
| 100000 | Orta tuşla kaydırma | 40 | 6.16 | 6.89 | 6.99 | 0.08 | 0.02 | 5.23 | 0.41 | 0.25 | 169.52 |
| 100000 | Çoklu çizgi çizerken imleç (kenet açık) | 120 | 5.75 | 6.42 | 8.11 | 0.08 | 0.03 | 4.65 | 0.53 | 0.25 | 173.87 |
| 100000 | Bir nesneyi silme | 1 | 111.11 | 111.11 | 111.11 | 0.00 | 0.04 | 110.33 | 0.51 | 0.23 | 195.53 |
| 100000 | Geri al ve yinele (bir nesne) | 10 | 113.61 | 115.31 | 115.31 | 0.00 | 0.02 | 112.93 | 0.24 | 0.16 | 196.92 |
| 100000 | Hepsini seç (Ctrl+A) | 1 | 275.84 | 275.84 | 275.84 | 0.00 | 26.86 | 248.41 | 0.40 | 0.17 | 450.61 |
| 100000 | İmleç çizimin üstünde (hepsi seçili) | 60 | 63.18 | 72.19 | 77.38 | 0.03 | 0.05 | 62.68 | 0.24 | 0.10 | 320.80 |
| 100000 | Seçimi bırakma (Esc) | 1 | 14.37 | 14.37 | 14.37 | 0.04 | 0.02 | 2.73 | 0.37 | 11.21 | 177.77 |
