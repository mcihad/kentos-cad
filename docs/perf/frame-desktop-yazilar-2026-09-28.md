# Masaüstünün karesi büyük çizimde: yazilar (2026-09-28, f41f1b23, kaydedilmemiş değişiklikle)

11th Gen Intel(R) Core(TM) i5-11300H @ 3.10GHz, 8 iş parçacığı, 15 GB; Ubuntu 26.04.1 LTS (7.0.0-34-generic); rustc 1.96.0 (ac68faa20 2026-05-25), `--release` (lto thin). Pencere 1440×900, çizici wgpu. Test `apps/desktop/src/perf/frame.rs`: arayüz pencere açmadan Iced'in çalışma zamanının sırasıyla sürülür. “Olay” bileşenlerin olayı alması, “uygulama” mesajların uygulanması, “görünüm” `App::view` (sahne eskidiyse kurulması dahil), “düzen” Iced'in ağacı karşılaştırıp yerleştirmesi, “çizim” bileşenlerin kareyi kaydetmesidir; “MİB” bunların toplamıdır (arayüz iş parçacığı). “GPU” karenin GPU'da hazırlanıp çizilmesi ve resmin geri okunmasıdır (yalnız wgpu ile; geri okuma pencerede yoktur).

Çizim: parsel başına 20 köşeli bir alan, üç öznitelik ve etiket, tek katman (`perf.rs`'in çizimi), bütünü pencereye sığmış. Süreler milisaniye; parçalar ortanca.

| Parsel | Durum | Kare | MİB p50 | MİB p95 | MİB en çok | Olay | Uygulama | Görünüm | Düzen | Çizim | GPU p50 |
|---|---|---|---|---|---|---|---|---|---|---|---|
| 10000 | Çizimi açma (ilk kare) | 1 | 13.76 | 13.76 | 13.76 | 0.00 | 6.81 | 5.71 | 0.48 | 0.76 | 56.80 |
| 10000 | Boş kare (yeniden çizim) | 30 | 0.69 | 0.74 | 0.77 | 0.00 | 0.00 | 0.22 | 0.26 | 0.20 | 12.96 |
| 10000 | İmleç çizimin üstünde (seçim aracı) | 120 | 0.52 | 0.68 | 1.20 | 0.04 | 0.02 | 0.11 | 0.20 | 0.16 | 12.49 |
| 10000 | Tekerlekle yakınlaştırma | 32 | 1.33 | 2.62 | 3.21 | 0.06 | 0.02 | 0.55 | 0.44 | 0.23 | 18.71 |
| 10000 | Orta tuşla kaydırma | 40 | 1.77 | 1.88 | 2.27 | 0.08 | 0.02 | 0.87 | 0.44 | 0.36 | 27.61 |
| 10000 | Orta tuşla kaydırma, 1:1000 | 40 | 0.76 | 1.01 | 1.02 | 0.05 | 0.01 | 0.15 | 0.20 | 0.36 | 14.46 |
| 10000 | Çoklu çizgi çizerken imleç (kenet açık) | 120 | 0.61 | 0.78 | 1.28 | 0.05 | 0.02 | 0.12 | 0.24 | 0.18 | 13.33 |
| 10000 | Bir nesneyi silme | 1 | 4.12 | 4.12 | 4.12 | 0.00 | 0.03 | 3.35 | 0.50 | 0.24 | 25.43 |
| 10000 | Geri al ve yinele (bir nesne) | 10 | 7.32 | 7.81 | 7.81 | 0.00 | 0.02 | 6.57 | 0.43 | 0.31 | 26.13 |
| 10000 | Hepsini seç (Ctrl+A) | 1 | 22.10 | 22.10 | 22.10 | 0.00 | 3.38 | 18.02 | 0.47 | 0.22 | 53.16 |
| 10000 | İmleç çizimin üstünde (hepsi seçili) | 60 | 0.62 | 0.72 | 3.52 | 0.04 | 0.02 | 0.16 | 0.22 | 0.17 | 12.36 |
| 10000 | Taşırken imleç (hepsi seçili) | 30 | 1.49 | 1.69 | 1.71 | 0.04 | 0.38 | 0.21 | 0.23 | 0.63 | 12.78 |
| 10000 | Hepsini taşıma (ikinci nokta) | 1 | 45.25 | 45.25 | 45.25 | 0.07 | 19.87 | 23.84 | 0.45 | 1.02 | 52.10 |
| 10000 | Taşımayı geri alma | 1 | 39.49 | 39.49 | 39.49 | 0.00 | 2.61 | 35.36 | 0.26 | 1.25 | 42.23 |
| 10000 | Seçimi bırakma (Esc) | 1 | 7.62 | 7.62 | 7.62 | 0.04 | 4.04 | 0.17 | 0.47 | 2.90 | 25.01 |
| 10000 | Parsel ölçü yazıları, bu bilgisayarda | 1 | 1005.12 | 1005.12 | 1005.12 | 0.00 | 804.19 | 194.39 | 0.62 | 5.92 | 83.68 |
| 10000 | Parsel ölçü yazıları, arka planda: Çalıştır | 1 | 8.66 | 8.66 | 8.66 | 0.00 | 7.62 | 0.33 | 0.40 | 0.31 | 52.38 |
| 10000 | Parsel ölçü yazıları, arka planda: sonucun uygulanması | 1 | 592.59 | 592.59 | 592.59 | 0.00 | 418.53 | 167.68 | 0.62 | 5.76 | 76.19 |
| 10000 | Parsel ölçü yazıları: sonuçlu çizimde imleç | 20 | 0.62 | 0.67 | 12.11 | 0.04 | 0.02 | 0.18 | 0.22 | 0.16 | 12.71 |
| 10000 | Parsel ölçü yazıları: sonuçlu çizimde kaydırma | 40 | 31.61 | 34.07 | 34.70 | 0.10 | 0.02 | 24.04 | 0.32 | 6.88 | 67.26 |
| 10000 | Parsel ölçü yazıları: sonuçlu çizimde kaydırma, 1:1000 | 40 | 62.67 | 66.30 | 68.92 | 0.07 | 0.02 | 0.67 | 0.33 | 61.65 | 36.82 |
