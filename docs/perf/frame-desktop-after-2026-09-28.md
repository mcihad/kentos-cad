# Masaüstünün karesi büyük çizimde: after (2026-09-28, 1b72b1e, kaydedilmemiş değişiklikle)

11th Gen Intel(R) Core(TM) i5-11300H @ 3.10GHz, 8 iş parçacığı, 15 GB; Ubuntu 26.04.1 LTS (7.0.0-34-generic); rustc 1.96.0 (ac68faa20 2026-05-25), `--release` (lto thin). Pencere 1440×900, çizici wgpu. Test `apps/desktop/src/perf/frame.rs`: arayüz pencere açmadan Iced'in çalışma zamanının sırasıyla sürülür. “Olay” bileşenlerin olayı alması, “uygulama” mesajların uygulanması, “görünüm” `App::view` (sahne eskidiyse kurulması dahil), “düzen” Iced'in ağacı karşılaştırıp yerleştirmesi, “çizim” bileşenlerin kareyi kaydetmesidir; “MİB” bunların toplamıdır (arayüz iş parçacığı). “GPU” karenin GPU'da hazırlanıp çizilmesi ve resmin geri okunmasıdır (yalnız wgpu ile; geri okuma pencerede yoktur).

Çizim: parsel başına 20 köşeli bir alan, üç öznitelik ve etiket, tek katman (`perf.rs`'in çizimi), bütünü pencereye sığmış. Süreler milisaniye; parçalar ortanca.

| Parsel | Durum | Kare | MİB p50 | MİB p95 | MİB en çok | Olay | Uygulama | Görünüm | Düzen | Çizim | GPU p50 |
|---|---|---|---|---|---|---|---|---|---|---|---|
| 0 | Boş kare (yeniden çizim) | 30 | 0.60 | 0.65 | 0.80 | 0.00 | 0.00 | 0.22 | 0.25 | 0.13 | 12.31 |
| 0 | İmleç çizimin üstünde (seçim aracı) | 120 | 0.50 | 0.58 | 0.72 | 0.04 | 0.02 | 0.11 | 0.21 | 0.12 | 12.30 |
| 0 | Tekerlekle yakınlaştırma | 32 | 0.63 | 0.94 | 1.06 | 0.05 | 0.01 | 0.15 | 0.25 | 0.16 | 13.64 |
| 0 | Orta tuşla kaydırma | 40 | 0.45 | 0.52 | 0.79 | 0.04 | 0.01 | 0.11 | 0.16 | 0.13 | 12.32 |
| 0 | Orta tuşla kaydırma, 1:1000 | 40 | 0.51 | 0.55 | 0.61 | 0.04 | 0.01 | 0.12 | 0.18 | 0.14 | 12.64 |
| 0 | Çoklu çizgi çizerken imleç (kenet açık) | 120 | 0.57 | 0.67 | 0.80 | 0.05 | 0.02 | 0.12 | 0.24 | 0.14 | 12.99 |
| 0 | Bir nesneyi silme | 1 | 0.69 | 0.69 | 0.69 | 0.00 | 0.01 | 0.20 | 0.31 | 0.17 | 12.56 |
| 0 | Geri al ve yinele (bir nesne) | 10 | 0.62 | 0.63 | 0.63 | 0.00 | 0.00 | 0.21 | 0.26 | 0.14 | 12.58 |
| 0 | Hepsini seç (Ctrl+A) | 1 | 0.82 | 0.82 | 0.82 | 0.00 | 0.00 | 0.24 | 0.37 | 0.21 | 12.86 |
| 0 | İmleç çizimin üstünde (hepsi seçili) | 60 | 0.51 | 0.61 | 0.72 | 0.04 | 0.02 | 0.11 | 0.21 | 0.12 | 12.28 |
| 0 | Seçimi bırakma (Esc) | 1 | 0.44 | 0.44 | 0.44 | 0.05 | 0.02 | 0.10 | 0.16 | 0.12 | 11.26 |
| 10000 | Boş kare (yeniden çizim) | 30 | 0.59 | 0.66 | 0.70 | 0.00 | 0.00 | 0.21 | 0.24 | 0.14 | 12.46 |
| 10000 | İmleç çizimin üstünde (seçim aracı) | 120 | 0.48 | 0.64 | 1.07 | 0.04 | 0.02 | 0.10 | 0.20 | 0.12 | 12.12 |
| 10000 | Tekerlekle yakınlaştırma | 32 | 1.14 | 2.38 | 2.64 | 0.06 | 0.02 | 0.54 | 0.31 | 0.20 | 17.37 |
| 10000 | Orta tuşla kaydırma | 40 | 1.70 | 1.77 | 1.79 | 0.08 | 0.02 | 0.88 | 0.44 | 0.27 | 27.23 |
| 10000 | Orta tuşla kaydırma, 1:1000 | 40 | 0.65 | 0.90 | 1.05 | 0.04 | 0.01 | 0.13 | 0.18 | 0.28 | 13.34 |
| 10000 | Çoklu çizgi çizerken imleç (kenet açık) | 120 | 0.56 | 0.75 | 0.80 | 0.04 | 0.02 | 0.11 | 0.24 | 0.13 | 13.05 |
| 10000 | Bir nesneyi silme | 1 | 16.52 | 16.52 | 16.52 | 0.00 | 0.07 | 15.80 | 0.45 | 0.20 | 24.21 |
| 10000 | Geri al ve yinele (bir nesne) | 10 | 10.96 | 12.56 | 12.56 | 0.00 | 0.02 | 10.49 | 0.26 | 0.16 | 24.20 |
| 10000 | Hepsini seç (Ctrl+A) | 1 | 26.54 | 26.54 | 26.54 | 0.00 | 2.96 | 22.98 | 0.42 | 0.18 | 43.02 |
| 10000 | İmleç çizimin üstünde (hepsi seçili) | 60 | 0.57 | 0.65 | 3.60 | 0.04 | 0.02 | 0.16 | 0.22 | 0.12 | 12.60 |
| 10000 | Seçimi bırakma (Esc) | 1 | 1.69 | 1.69 | 1.69 | 0.05 | 0.02 | 0.11 | 0.48 | 1.03 | 31.05 |
| 50000 | Boş kare (yeniden çizim) | 30 | 0.62 | 0.70 | 0.71 | 0.00 | 0.00 | 0.22 | 0.26 | 0.14 | 12.82 |
| 50000 | İmleç çizimin üstünde (seçim aracı) | 120 | 0.59 | 0.69 | 1.17 | 0.04 | 0.10 | 0.12 | 0.22 | 0.12 | 12.35 |
| 50000 | Tekerlekle yakınlaştırma | 32 | 2.77 | 4.73 | 5.09 | 0.09 | 0.02 | 1.87 | 0.50 | 0.29 | 43.15 |
| 50000 | Orta tuşla kaydırma | 40 | 4.03 | 4.28 | 4.35 | 0.08 | 0.02 | 3.18 | 0.46 | 0.28 | 81.30 |
| 50000 | Orta tuşla kaydırma, 1:1000 | 40 | 1.08 | 1.27 | 1.47 | 0.07 | 0.02 | 0.23 | 0.31 | 0.44 | 18.65 |
| 50000 | Çoklu çizgi çizerken imleç (kenet açık) | 120 | 0.69 | 0.86 | 1.40 | 0.04 | 0.16 | 0.13 | 0.25 | 0.13 | 13.11 |
| 50000 | Bir nesneyi silme | 1 | 74.51 | 74.51 | 74.51 | 0.00 | 0.03 | 73.71 | 0.52 | 0.25 | 103.83 |
| 50000 | Geri al ve yinele (bir nesne) | 10 | 59.92 | 64.33 | 64.33 | 0.00 | 0.02 | 59.49 | 0.25 | 0.16 | 98.36 |
| 50000 | Hepsini seç (Ctrl+A) | 1 | 129.02 | 129.02 | 129.02 | 0.00 | 16.89 | 111.54 | 0.40 | 0.18 | 240.86 |
| 50000 | İmleç çizimin üstünde (hepsi seçili) | 60 | 0.85 | 0.94 | 14.86 | 0.04 | 0.09 | 0.38 | 0.23 | 0.12 | 11.99 |
| 50000 | Seçimi bırakma (Esc) | 1 | 7.26 | 7.26 | 7.26 | 0.05 | 0.02 | 0.10 | 0.46 | 6.62 | 85.21 |
| 100000 | Boş kare (yeniden çizim) | 30 | 0.62 | 0.68 | 0.68 | 0.00 | 0.00 | 0.22 | 0.26 | 0.14 | 12.85 |
| 100000 | İmleç çizimin üstünde (seçim aracı) | 120 | 0.52 | 0.67 | 1.19 | 0.04 | 0.02 | 0.11 | 0.20 | 0.12 | 12.38 |
| 100000 | Tekerlekle yakınlaştırma | 32 | 4.57 | 6.40 | 6.55 | 0.08 | 0.02 | 3.56 | 0.49 | 0.29 | 103.60 |
| 100000 | Orta tuşla kaydırma | 40 | 5.87 | 6.91 | 7.36 | 0.08 | 0.02 | 5.00 | 0.41 | 0.27 | 172.47 |
| 100000 | Orta tuşla kaydırma, 1:1000 | 40 | 1.37 | 1.56 | 1.62 | 0.08 | 0.02 | 0.28 | 0.42 | 0.58 | 29.31 |
| 100000 | Çoklu çizgi çizerken imleç (kenet açık) | 120 | 0.68 | 1.07 | 1.52 | 0.05 | 0.02 | 0.12 | 0.26 | 0.14 | 13.14 |
| 100000 | Bir nesneyi silme | 1 | 120.60 | 120.60 | 120.60 | 0.00 | 0.04 | 119.80 | 0.52 | 0.24 | 193.05 |
| 100000 | Geri al ve yinele (bir nesne) | 10 | 114.54 | 123.63 | 123.63 | 0.00 | 0.02 | 113.93 | 0.27 | 0.16 | 197.47 |
| 100000 | Hepsini seç (Ctrl+A) | 1 | 243.62 | 243.62 | 243.62 | 0.00 | 26.82 | 216.18 | 0.43 | 0.18 | 451.00 |
| 100000 | İmleç çizimin üstünde (hepsi seçili) | 60 | 1.14 | 1.38 | 24.57 | 0.04 | 0.07 | 0.65 | 0.24 | 0.11 | 12.10 |
| 100000 | Seçimi bırakma (Esc) | 1 | 13.60 | 13.60 | 13.60 | 0.05 | 0.02 | 0.10 | 0.45 | 12.98 | 178.86 |
