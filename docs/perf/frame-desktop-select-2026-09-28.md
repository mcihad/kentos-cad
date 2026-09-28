# Masaüstünün karesi büyük çizimde: select (2026-09-28, 5b2c6a7, kaydedilmemiş değişiklikle)

11th Gen Intel(R) Core(TM) i5-11300H @ 3.10GHz, 8 iş parçacığı, 15 GB; Ubuntu 26.04.1 LTS (7.0.0-34-generic); rustc 1.96.0 (ac68faa20 2026-05-25), `--release` (lto thin). Pencere 1440×900, çizici wgpu. Test `apps/desktop/src/perf/frame.rs`: arayüz pencere açmadan Iced'in çalışma zamanının sırasıyla sürülür. “Olay” bileşenlerin olayı alması, “uygulama” mesajların uygulanması, “görünüm” `App::view` (sahne eskidiyse kurulması dahil), “düzen” Iced'in ağacı karşılaştırıp yerleştirmesi, “çizim” bileşenlerin kareyi kaydetmesidir; “MİB” bunların toplamıdır (arayüz iş parçacığı). “GPU” karenin GPU'da hazırlanıp çizilmesi ve resmin geri okunmasıdır (yalnız wgpu ile; geri okuma pencerede yoktur).

Çizim: parsel başına 20 köşeli bir alan, üç öznitelik ve etiket, tek katman (`perf.rs`'in çizimi), bütünü pencereye sığmış. Süreler milisaniye; parçalar ortanca.

| Parsel | Durum | Kare | MİB p50 | MİB p95 | MİB en çok | Olay | Uygulama | Görünüm | Düzen | Çizim | GPU p50 |
|---|---|---|---|---|---|---|---|---|---|---|---|
| 0 | Çizimi açma (ilk kare) | 1 | 1.44 | 1.44 | 1.44 | 0.00 | 0.03 | 0.25 | 0.42 | 0.75 | 64.93 |
| 0 | Boş kare (yeniden çizim) | 30 | 0.61 | 0.68 | 0.69 | 0.00 | 0.00 | 0.22 | 0.26 | 0.14 | 12.90 |
| 0 | İmleç çizimin üstünde (seçim aracı) | 120 | 0.52 | 0.64 | 1.13 | 0.04 | 0.02 | 0.11 | 0.22 | 0.12 | 12.38 |
| 0 | Tekerlekle yakınlaştırma | 32 | 0.63 | 1.01 | 1.08 | 0.05 | 0.01 | 0.14 | 0.25 | 0.16 | 13.67 |
| 0 | Orta tuşla kaydırma | 40 | 0.48 | 0.60 | 0.95 | 0.04 | 0.01 | 0.11 | 0.18 | 0.13 | 12.21 |
| 0 | Orta tuşla kaydırma, 1:1000 | 40 | 0.49 | 0.54 | 0.56 | 0.04 | 0.01 | 0.11 | 0.18 | 0.14 | 12.35 |
| 0 | Çoklu çizgi çizerken imleç (kenet açık) | 120 | 0.57 | 0.65 | 0.70 | 0.05 | 0.02 | 0.12 | 0.25 | 0.14 | 13.00 |
| 0 | Bir nesneyi silme | 1 | 1.30 | 1.30 | 1.30 | 0.00 | 0.03 | 0.35 | 0.59 | 0.33 | 18.07 |
| 0 | Geri al ve yinele (bir nesne) | 10 | 0.71 | 1.07 | 1.07 | 0.00 | 0.01 | 0.22 | 0.31 | 0.16 | 13.42 |
| 0 | Hepsini seç (Ctrl+A) | 1 | 0.78 | 0.78 | 0.78 | 0.00 | 0.01 | 0.23 | 0.36 | 0.18 | 13.01 |
| 0 | İmleç çizimin üstünde (hepsi seçili) | 60 | 0.51 | 0.58 | 0.66 | 0.04 | 0.02 | 0.11 | 0.21 | 0.12 | 12.48 |
| 0 | Seçimi bırakma (Esc) | 1 | 0.51 | 0.51 | 0.51 | 0.05 | 0.02 | 0.12 | 0.19 | 0.12 | 12.31 |
| 10000 | Çizimi açma (ilk kare) | 1 | 14.21 | 14.21 | 14.21 | 0.00 | 6.33 | 6.73 | 0.48 | 0.67 | 56.16 |
| 10000 | Boş kare (yeniden çizim) | 30 | 0.60 | 0.92 | 1.11 | 0.00 | 0.00 | 0.21 | 0.25 | 0.14 | 12.72 |
| 10000 | İmleç çizimin üstünde (seçim aracı) | 120 | 0.48 | 0.60 | 0.71 | 0.04 | 0.01 | 0.10 | 0.20 | 0.12 | 12.36 |
| 10000 | Tekerlekle yakınlaştırma | 32 | 1.43 | 2.59 | 3.44 | 0.07 | 0.02 | 0.62 | 0.38 | 0.24 | 18.41 |
| 10000 | Orta tuşla kaydırma | 40 | 1.69 | 1.80 | 1.82 | 0.08 | 0.02 | 0.86 | 0.44 | 0.27 | 27.55 |
| 10000 | Orta tuşla kaydırma, 1:1000 | 40 | 0.65 | 0.79 | 0.83 | 0.04 | 0.01 | 0.14 | 0.18 | 0.28 | 13.05 |
| 10000 | Çoklu çizgi çizerken imleç (kenet açık) | 120 | 0.56 | 0.76 | 0.91 | 0.05 | 0.02 | 0.11 | 0.24 | 0.13 | 13.14 |
| 10000 | Bir nesneyi silme | 1 | 9.29 | 9.29 | 9.29 | 0.00 | 0.04 | 8.07 | 0.83 | 0.36 | 25.16 |
| 10000 | Geri al ve yinele (bir nesne) | 10 | 7.78 | 8.40 | 8.40 | 0.00 | 0.02 | 7.02 | 0.44 | 0.26 | 25.93 |
| 10000 | Hepsini seç (Ctrl+A) | 1 | 23.68 | 23.68 | 23.68 | 0.00 | 3.48 | 19.56 | 0.44 | 0.20 | 47.07 |
| 10000 | İmleç çizimin üstünde (hepsi seçili) | 60 | 0.58 | 0.71 | 3.30 | 0.04 | 0.02 | 0.16 | 0.22 | 0.12 | 12.18 |
| 10000 | Seçimi bırakma (Esc) | 1 | 1.59 | 1.59 | 1.59 | 0.05 | 0.02 | 0.10 | 0.44 | 0.98 | 30.85 |
| 50000 | Çizimi açma (ilk kare) | 1 | 52.88 | 52.88 | 52.88 | 0.00 | 32.79 | 19.14 | 0.46 | 0.48 | 60.07 |
| 50000 | Boş kare (yeniden çizim) | 30 | 0.65 | 0.73 | 0.75 | 0.00 | 0.00 | 0.23 | 0.27 | 0.15 | 13.00 |
| 50000 | İmleç çizimin üstünde (seçim aracı) | 120 | 0.58 | 0.66 | 0.69 | 0.04 | 0.10 | 0.12 | 0.21 | 0.12 | 12.32 |
| 50000 | Tekerlekle yakınlaştırma | 32 | 2.80 | 4.70 | 4.99 | 0.09 | 0.02 | 1.90 | 0.50 | 0.29 | 41.79 |
| 50000 | Orta tuşla kaydırma | 40 | 4.09 | 4.25 | 4.29 | 0.08 | 0.02 | 3.22 | 0.46 | 0.28 | 81.30 |
| 50000 | Orta tuşla kaydırma, 1:1000 | 40 | 0.64 | 0.94 | 1.36 | 0.04 | 0.01 | 0.13 | 0.18 | 0.28 | 13.21 |
| 50000 | Çoklu çizgi çizerken imleç (kenet açık) | 120 | 0.70 | 0.83 | 0.92 | 0.05 | 0.16 | 0.12 | 0.25 | 0.14 | 13.08 |
| 50000 | Bir nesneyi silme | 1 | 12.88 | 12.88 | 12.88 | 0.00 | 0.04 | 11.10 | 1.20 | 0.54 | 81.48 |
| 50000 | Geri al ve yinele (bir nesne) | 10 | 11.01 | 12.13 | 12.13 | 0.00 | 0.02 | 10.03 | 0.57 | 0.35 | 82.08 |
| 50000 | Hepsini seç (Ctrl+A) | 1 | 92.37 | 92.37 | 92.37 | 0.00 | 17.73 | 74.04 | 0.41 | 0.19 | 240.44 |
| 50000 | İmleç çizimin üstünde (hepsi seçili) | 60 | 0.86 | 1.02 | 15.20 | 0.04 | 0.09 | 0.37 | 0.23 | 0.12 | 11.95 |
| 50000 | Seçimi bırakma (Esc) | 1 | 6.80 | 6.80 | 6.80 | 0.04 | 0.02 | 0.10 | 0.42 | 6.21 | 83.72 |
| 100000 | Çizimi açma (ilk kare) | 1 | 104.92 | 104.92 | 104.92 | 0.00 | 62.91 | 41.11 | 0.43 | 0.47 | 75.33 |
| 100000 | Boş kare (yeniden çizim) | 30 | 0.63 | 1.05 | 1.13 | 0.00 | 0.00 | 0.23 | 0.25 | 0.13 | 13.01 |
| 100000 | İmleç çizimin üstünde (seçim aracı) | 120 | 0.53 | 0.77 | 1.30 | 0.04 | 0.02 | 0.11 | 0.21 | 0.12 | 12.82 |
| 100000 | Tekerlekle yakınlaştırma | 32 | 5.05 | 6.56 | 7.15 | 0.09 | 0.02 | 4.14 | 0.50 | 0.29 | 100.07 |
| 100000 | Orta tuşla kaydırma | 40 | 5.61 | 6.99 | 7.34 | 0.08 | 0.02 | 4.78 | 0.43 | 0.27 | 170.56 |
| 100000 | Orta tuşla kaydırma, 1:1000 | 40 | 0.68 | 1.04 | 1.09 | 0.04 | 0.01 | 0.14 | 0.19 | 0.30 | 13.72 |
| 100000 | Çoklu çizgi çizerken imleç (kenet açık) | 120 | 0.62 | 1.00 | 1.27 | 0.05 | 0.02 | 0.12 | 0.25 | 0.13 | 13.35 |
| 100000 | Bir nesneyi silme | 1 | 13.17 | 13.17 | 13.17 | 0.00 | 0.05 | 11.49 | 1.12 | 0.52 | 172.87 |
| 100000 | Geri al ve yinele (bir nesne) | 10 | 12.70 | 12.97 | 12.97 | 0.00 | 0.02 | 11.73 | 0.53 | 0.33 | 173.44 |
| 100000 | Hepsini seç (Ctrl+A) | 1 | 171.13 | 171.13 | 171.13 | 0.00 | 20.74 | 149.77 | 0.41 | 0.21 | 464.45 |
| 100000 | İmleç çizimin üstünde (hepsi seçili) | 60 | 1.07 | 1.17 | 22.30 | 0.04 | 0.06 | 0.62 | 0.22 | 0.11 | 13.47 |
| 100000 | Seçimi bırakma (Esc) | 1 | 11.08 | 11.08 | 11.08 | 0.04 | 0.02 | 0.11 | 0.46 | 10.44 | 180.97 |
