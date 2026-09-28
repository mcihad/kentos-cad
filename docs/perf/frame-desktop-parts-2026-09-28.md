# Masaüstünün karesi büyük çizimde: parts (2026-09-28, 6cbc2ef, kaydedilmemiş değişiklikle)

11th Gen Intel(R) Core(TM) i5-11300H @ 3.10GHz, 8 iş parçacığı, 15 GB; Ubuntu 26.04.1 LTS (7.0.0-34-generic); rustc 1.96.0 (ac68faa20 2026-05-25), `--release` (lto thin). Pencere 1440×900, çizici wgpu. Test `apps/desktop/src/perf/frame.rs`: arayüz pencere açmadan Iced'in çalışma zamanının sırasıyla sürülür. “Olay” bileşenlerin olayı alması, “uygulama” mesajların uygulanması, “görünüm” `App::view` (sahne eskidiyse kurulması dahil), “düzen” Iced'in ağacı karşılaştırıp yerleştirmesi, “çizim” bileşenlerin kareyi kaydetmesidir; “MİB” bunların toplamıdır (arayüz iş parçacığı). “GPU” karenin GPU'da hazırlanıp çizilmesi ve resmin geri okunmasıdır (yalnız wgpu ile; geri okuma pencerede yoktur).

Çizim: parsel başına 20 köşeli bir alan, üç öznitelik ve etiket, tek katman (`perf.rs`'in çizimi), bütünü pencereye sığmış. Süreler milisaniye; parçalar ortanca.

| Parsel | Durum | Kare | MİB p50 | MİB p95 | MİB en çok | Olay | Uygulama | Görünüm | Düzen | Çizim | GPU p50 |
|---|---|---|---|---|---|---|---|---|---|---|---|
| 0 | Boş kare (yeniden çizim) | 30 | 0.60 | 0.66 | 0.67 | 0.00 | 0.00 | 0.21 | 0.25 | 0.14 | 12.43 |
| 0 | İmleç çizimin üstünde (seçim aracı) | 120 | 0.47 | 0.55 | 0.82 | 0.04 | 0.01 | 0.11 | 0.20 | 0.12 | 12.23 |
| 0 | Tekerlekle yakınlaştırma | 32 | 0.58 | 0.90 | 1.10 | 0.05 | 0.01 | 0.13 | 0.22 | 0.15 | 13.31 |
| 0 | Orta tuşla kaydırma | 40 | 0.39 | 0.61 | 1.11 | 0.03 | 0.01 | 0.09 | 0.13 | 0.11 | 9.98 |
| 0 | Orta tuşla kaydırma, 1:1000 | 40 | 0.49 | 0.55 | 0.62 | 0.04 | 0.02 | 0.11 | 0.21 | 0.12 | 12.41 |
| 0 | Çoklu çizgi çizerken imleç (kenet açık) | 120 | 0.56 | 0.61 | 0.70 | 0.05 | 0.02 | 0.12 | 0.24 | 0.13 | 13.05 |
| 0 | Bir nesneyi silme | 1 | 0.74 | 0.74 | 0.74 | 0.00 | 0.01 | 0.22 | 0.35 | 0.16 | 13.73 |
| 0 | Geri al ve yinele (bir nesne) | 10 | 0.74 | 0.91 | 0.91 | 0.00 | 0.01 | 0.25 | 0.31 | 0.17 | 14.68 |
| 0 | Hepsini seç (Ctrl+A) | 1 | 0.86 | 0.86 | 0.86 | 0.00 | 0.00 | 0.27 | 0.39 | 0.19 | 13.05 |
| 0 | İmleç çizimin üstünde (hepsi seçili) | 60 | 0.52 | 0.65 | 1.13 | 0.04 | 0.02 | 0.11 | 0.22 | 0.13 | 12.36 |
| 0 | Seçimi bırakma (Esc) | 1 | 0.64 | 0.64 | 0.64 | 0.07 | 0.02 | 0.15 | 0.25 | 0.15 | 12.94 |
| 10000 | Boş kare (yeniden çizim) | 30 | 0.61 | 0.75 | 0.75 | 0.00 | 0.00 | 0.21 | 0.25 | 0.14 | 12.38 |
| 10000 | İmleç çizimin üstünde (seçim aracı) | 120 | 0.49 | 0.61 | 1.01 | 0.04 | 0.02 | 0.11 | 0.20 | 0.12 | 12.27 |
| 10000 | Tekerlekle yakınlaştırma | 32 | 1.16 | 2.44 | 2.65 | 0.06 | 0.02 | 0.53 | 0.31 | 0.19 | 18.99 |
| 10000 | Orta tuşla kaydırma | 40 | 1.69 | 1.79 | 1.81 | 0.08 | 0.02 | 0.87 | 0.43 | 0.27 | 28.09 |
| 10000 | Orta tuşla kaydırma, 1:1000 | 40 | 0.62 | 0.80 | 1.03 | 0.04 | 0.01 | 0.13 | 0.17 | 0.28 | 12.88 |
| 10000 | Çoklu çizgi çizerken imleç (kenet açık) | 120 | 0.48 | 0.71 | 0.97 | 0.04 | 0.02 | 0.10 | 0.20 | 0.11 | 11.73 |
| 10000 | Bir nesneyi silme | 1 | 9.61 | 9.61 | 9.61 | 0.00 | 0.06 | 8.16 | 1.00 | 0.39 | 25.45 |
| 10000 | Geri al ve yinele (bir nesne) | 10 | 7.87 | 8.33 | 8.33 | 0.00 | 0.02 | 7.12 | 0.43 | 0.27 | 26.09 |
| 10000 | Hepsini seç (Ctrl+A) | 1 | 28.22 | 28.22 | 28.22 | 0.00 | 2.18 | 25.42 | 0.43 | 0.18 | 45.93 |
| 10000 | İmleç çizimin üstünde (hepsi seçili) | 60 | 0.54 | 0.68 | 3.78 | 0.04 | 0.02 | 0.15 | 0.21 | 0.11 | 12.00 |
| 10000 | Seçimi bırakma (Esc) | 1 | 2.46 | 2.46 | 2.46 | 0.08 | 0.02 | 0.12 | 0.52 | 1.73 | 31.49 |
| 50000 | Boş kare (yeniden çizim) | 30 | 0.58 | 0.69 | 0.69 | 0.00 | 0.00 | 0.21 | 0.24 | 0.12 | 12.08 |
| 50000 | İmleç çizimin üstünde (seçim aracı) | 120 | 0.52 | 0.71 | 1.22 | 0.04 | 0.09 | 0.11 | 0.18 | 0.11 | 11.32 |
| 50000 | Tekerlekle yakınlaştırma | 32 | 2.74 | 4.96 | 5.31 | 0.09 | 0.02 | 1.88 | 0.49 | 0.29 | 42.61 |
| 50000 | Orta tuşla kaydırma | 40 | 3.94 | 6.04 | 6.45 | 0.08 | 0.02 | 3.08 | 0.46 | 0.28 | 82.20 |
| 50000 | Orta tuşla kaydırma, 1:1000 | 40 | 0.56 | 0.71 | 0.84 | 0.04 | 0.01 | 0.12 | 0.14 | 0.24 | 11.38 |
| 50000 | Çoklu çizgi çizerken imleç (kenet açık) | 120 | 0.62 | 0.89 | 1.45 | 0.04 | 0.15 | 0.12 | 0.21 | 0.12 | 12.10 |
| 50000 | Bir nesneyi silme | 1 | 15.33 | 15.33 | 15.33 | 0.00 | 0.04 | 14.49 | 0.56 | 0.25 | 85.35 |
| 50000 | Geri al ve yinele (bir nesne) | 10 | 13.44 | 15.41 | 15.41 | 0.00 | 0.02 | 12.18 | 0.55 | 0.35 | 81.98 |
| 50000 | Hepsini seç (Ctrl+A) | 1 | 137.65 | 137.65 | 137.65 | 0.00 | 17.16 | 119.85 | 0.46 | 0.19 | 245.50 |
| 50000 | İmleç çizimin üstünde (hepsi seçili) | 60 | 0.87 | 1.27 | 15.79 | 0.04 | 0.09 | 0.38 | 0.23 | 0.12 | 11.75 |
| 50000 | Seçimi bırakma (Esc) | 1 | 7.82 | 7.82 | 7.82 | 0.05 | 0.02 | 0.14 | 0.48 | 7.12 | 80.20 |
| 100000 | Boş kare (yeniden çizim) | 30 | 0.67 | 0.93 | 0.97 | 0.00 | 0.00 | 0.27 | 0.24 | 0.13 | 14.96 |
| 100000 | İmleç çizimin üstünde (seçim aracı) | 120 | 0.51 | 0.74 | 0.86 | 0.04 | 0.02 | 0.10 | 0.18 | 0.10 | 10.78 |
| 100000 | Tekerlekle yakınlaştırma | 32 | 4.90 | 6.84 | 7.22 | 0.09 | 0.02 | 4.13 | 0.51 | 0.30 | 100.31 |
| 100000 | Orta tuşla kaydırma | 40 | 6.33 | 7.66 | 9.66 | 0.08 | 0.02 | 5.40 | 0.43 | 0.27 | 172.32 |
| 100000 | Orta tuşla kaydırma, 1:1000 | 40 | 0.73 | 1.07 | 1.47 | 0.05 | 0.01 | 0.15 | 0.20 | 0.31 | 13.53 |
| 100000 | Çoklu çizgi çizerken imleç (kenet açık) | 120 | 0.61 | 0.91 | 1.63 | 0.04 | 0.02 | 0.12 | 0.23 | 0.12 | 12.88 |
| 100000 | Bir nesneyi silme | 1 | 7.56 | 7.56 | 7.56 | 0.00 | 0.03 | 6.84 | 0.47 | 0.22 | 171.38 |
| 100000 | Geri al ve yinele (bir nesne) | 10 | 12.76 | 15.21 | 15.21 | 0.00 | 0.02 | 11.83 | 0.52 | 0.33 | 173.32 |
| 100000 | Hepsini seç (Ctrl+A) | 1 | 241.93 | 241.93 | 241.93 | 0.00 | 27.12 | 214.20 | 0.40 | 0.20 | 452.34 |
| 100000 | İmleç çizimin üstünde (hepsi seçili) | 60 | 1.18 | 1.30 | 24.83 | 0.04 | 0.06 | 0.70 | 0.23 | 0.12 | 12.37 |
| 100000 | Seçimi bırakma (Esc) | 1 | 14.08 | 14.08 | 14.08 | 0.05 | 0.02 | 0.11 | 0.48 | 13.43 | 178.09 |
