# Etkileşim ölçümü: s6 (2026-09-25, 405c364)

11th Gen Intel(R) Core(TM) i5-11300H @ 3.10GHz, 8 iş parçacığı, 15 GB; Google Chrome 154.0.8037.57; başsız, WebGL2 (ANGLE (Intel, Mesa Intel(R) Iris(R) Xe Graphics (TGL GT2), OpenGL ES 3.2)); Vite geliştirme sunucusu (`window.kentos`). Pencere 1600×900, çizim alanı 1288×759 CSS px, dpr 1. 3 koşu; her koşu sayfayı yeniden açar ve veri setini yeniden kurar. Toplam 3 dk, Chrome en çok 836 MB (PSS).

Süreler ana iş parçacığında ms'dir (`ViewportProbe`, src/viewport/ViewportController.ts; yalnız geliştirme derlemesinde). Her hücre 3 koşunun ortancasıdır; p95'in yanında koşular arasındaki en düşük–en yüksek p95 yazar. Zamanlayıcı adımı 5 µs (cross-origin isolated). Hedefler [ADR 0005](../adr/0005-performance-acceptance-targets.md)'tedir; ADR **taslaktır ve kullanıcı onayı bekler**, bu rapor yalnızca kayıttır.

## Veri setleri

| Ad | İçerik | Nesne | Ham kenar | Üretme | `replaceWith` | İlk kare (bütün katmanlar) | JS yığını | Chrome (yüklemeden sonra) |
|---|---|---|---|---|---|---|---|---|
| `parsel-50k` | ~50 000 parsel (bir kısmı yaylı, bir kısmı delikli) ve içlerinde yapılar, TM36 | 81 229 | 334 336 | 116 ms | 29.1 ms | 148 ms | 39 MB | 786 MB |
| `hat-1m` | 1 000 000 doğru parçası: 2 000 eşyükselti benzeri çoklu çizgi × 500 parça, TM36 | 2 000 | 1 000 000 | 124 ms | 8.47 ms | 142 ms | 59 MB | 855 MB |

- `parsel-50k`: 50 400 parsel (6 146 yaylı cephe, 2 355 delikli), 30 829 yapı; büyük katman `parsel`. Yakın görünüm 1:1 000, 193 nesne görünür; genel görünüm 1:35 166, 81 229 nesne. Tohum 5256.
- `hat-1m`: 2 000 çoklu çizgi, hepsi tek katmanda; büyük katman `esyukselti`. Yakın görünüm 1:1 000, 71 nesne görünür; genel görünüm 1:57 877, 2 000 nesne. Tohum 1000.

## İmleç hareketi (olay başına)

Aynı tohumlu 240 hareketlik yol (buda: ilk 120 (`parsel-50k`), 40 (`hat-1m`)); her hareket kendi karesini çizer. “Seçme” ve “kenar seçme” aracın hareket işleyicisi, “kenet” nesne kenetidir; “olayın tamamı” `pointermove` işleyicisinin bütünüdür (durum çubuğu koordinatları dahil).

| Veri seti | Araç | Görünüm | Ölçüt | p50 | p95 (aralık) | p99 | en çok |
|---|---|---|---|---|---|---|---|
| `parsel-50k` | Seç (üzerine gelme) | yakın (1:1000) | seçme | 0.05 | 0.09 (0.09–0.10) | 0.12 | 0.13 |
| `parsel-50k` | Seç (üzerine gelme) | yakın (1:1000) | olayın tamamı | 0.17 | 0.21 (0.21–0.22) | 0.24 | 0.34 |
| `parsel-50k` | Seç (üzerine gelme) | genel (tümü) | seçme | 0.12 | 0.17 (0.17–0.20) | 0.19 | 0.26 |
| `parsel-50k` | Seç (üzerine gelme) | genel (tümü) | olayın tamamı | 0.21 | 0.27 (0.27–0.29) | 0.29 | 0.35 |
| `parsel-50k` | Çizgi, ilk noktadan sonra | yakın (1:1000) | kenet | 0.07 | 0.11 (0.10–0.12) | 0.12 | 0.59 |
| `parsel-50k` | Çizgi, ilk noktadan sonra | yakın (1:1000) | olayın tamamı | 0.25 | 0.32 (0.32–0.34) | 0.37 | 0.71 |
| `parsel-50k` | Çizgi, ilk noktadan sonra | genel (tümü) | kenet | 0.28 | 0.41 (0.39–0.42) | 0.46 | 0.82 |
| `parsel-50k` | Çizgi, ilk noktadan sonra | genel (tümü) | olayın tamamı | 0.36 | 0.52 (0.51–0.54) | 0.63 | 0.95 |
| `parsel-50k` | Buda | yakın (1:1000) | kenar seçme | 0.07 | 0.12 (0.10–0.12) | 0.15 | 0.19 |
| `parsel-50k` | Buda | yakın (1:1000) | olayın tamamı | 0.16 | 0.22 (0.18–0.22) | 0.24 | 0.27 |
| `hat-1m` | Seç (üzerine gelme) | yakın (1:1000) | seçme | 1.11 | 1.38 (1.33–1.40) | 1.50 | 2.75 |
| `hat-1m` | Seç (üzerine gelme) | yakın (1:1000) | olayın tamamı | 1.22 | 1.49 (1.45–1.52) | 1.63 | 2.83 |
| `hat-1m` | Seç (üzerine gelme) | genel (tümü) | seçme | 2.33 | 2.86 (2.73–2.97) | 3.09 | 4.56 |
| `hat-1m` | Seç (üzerine gelme) | genel (tümü) | olayın tamamı | 2.41 | 2.95 (2.85–3.07) | 3.21 | 4.67 |
| `hat-1m` | Çizgi, ilk noktadan sonra | yakın (1:1000) | kenet | 0.63 | 0.93 (0.71–0.94) | 1.09 | 2.48 |
| `hat-1m` | Çizgi, ilk noktadan sonra | yakın (1:1000) | olayın tamamı | 0.80 | 1.07 (0.93–1.18) | 1.28 | 2.77 |
| `hat-1m` | Çizgi, ilk noktadan sonra | genel (tümü) | kenet | 1.88 | 2.13 (2.07–2.49) | 2.68 | 3.69 |
| `hat-1m` | Çizgi, ilk noktadan sonra | genel (tümü) | olayın tamamı | 1.99 | 2.25 (2.19–2.64) | 2.86 | 3.81 |
| `hat-1m` | Buda | yakın (1:1000) | kenar seçme | 0.67 | 1.03 (0.99–1.12) | 2.15 | 2.60 |
| `hat-1m` | Buda | yakın (1:1000) | olayın tamamı | 0.74 | 1.11 (1.09–1.22) | 2.24 | 2.70 |

## Kareler

Kaydırma orta tuşla 80 adımdır (gidip gelir); “GPU gönderimi” çizim çağrılarının ana iş parçacığındaki süresidir, GPU'nun kendi çizimi dahil değildir. “Kare aralığı” bir karenin başından sonrakinin başına geçen duvar saatidir: GPU'nun işi ve 60 Hz ekran temposu dahil, ulaşılan kare hızını gösterir (16.7 ms = 60 fps; tempo yüzünden 16.7'nin katlarına yakın çıkar). Yeniden kurma koşu başına 6 stil değişikliğidir.

| Veri seti | Senaryo | Ölçüt | p50 | p95 (aralık) | p99 | en çok |
|---|---|---|---|---|---|---|
| `parsel-50k` | Kaydırma, yakın (1:1000) | üst katman | 0.22 | 0.28 (0.27–0.28) | 0.29 | 0.36 |
| `parsel-50k` | Kaydırma, yakın (1:1000) | etiketler | 0.05 | 0.07 (0.07–0.07) | 0.08 | 0.09 |
| `parsel-50k` | Kaydırma, yakın (1:1000) | GPU gönderimi | 0.27 | 0.34 (0.32–0.35) | 0.39 | 0.40 |
| `parsel-50k` | Kaydırma, yakın (1:1000) | kare (CPU) | 0.53 | 0.63 (0.61–0.64) | 0.70 | 0.70 |
| `parsel-50k` | Kaydırma, yakın (1:1000) | kare aralığı (GPU dahil) | 16.7 | 17.0 (17.0–17.3) | 36.5 | 41.2 |
| `parsel-50k` | Kaydırma, genel (tümü) | üst katman | 0.24 | 0.30 (0.30–0.33) | 0.35 | 0.37 |
| `parsel-50k` | Kaydırma, genel (tümü) | etiketler | 0.06 | 0.09 (0.08–0.09) | 0.13 | 0.17 |
| `parsel-50k` | Kaydırma, genel (tümü) | GPU gönderimi | 0.28 | 0.36 (0.36–0.38) | 0.53 | 0.53 |
| `parsel-50k` | Kaydırma, genel (tümü) | kare (CPU) | 0.56 | 0.67 (0.67–0.69) | 0.88 | 0.91 |
| `parsel-50k` | Kaydırma, genel (tümü) | kare aralığı (GPU dahil) | 26.4 | 27.6 (27.5–27.9) | 54.4 | 78.8 |
| `parsel-50k` | Buda önizlemesi, yakın (1:1000) | önizleme (imleç bir kenardayken) | 0.36 | 0.55 (0.53–0.62) | 0.68 | 0.69 |
| `parsel-50k` | Buda önizlemesi, yakın (1:1000) | kare (CPU), bütün kareler | 0.28 | 1.02 (0.57–1.04) | 1.10 | 1.32 |
| `parsel-50k` | Seç, yakın (1:1000) (vurgu değişince tam çizim) | kare (CPU) | 0.29 | 0.74 (0.69–0.75) | 0.83 | 1.27 |
| `parsel-50k` | Çizgi, yakın (1:1000) | kare (CPU) | 0.38 | 0.47 (0.45–0.48) | 0.53 | 1.62 |
| `parsel-50k` | Büyük katmanı yeniden kurma (stil değişikliği) | kurma ve yükleme | 35.7 | 38.6 (38.1–40.9) | 38.6 | 40.9 |
| `hat-1m` | Kaydırma, yakın (1:1000) | üst katman | 0.23 | 0.28 (0.27–0.29) | 0.33 | 0.40 |
| `hat-1m` | Kaydırma, yakın (1:1000) | etiketler | 0.05 | 0.07 (0.07–0.08) | 0.09 | 0.10 |
| `hat-1m` | Kaydırma, yakın (1:1000) | GPU gönderimi | 0.24 | 0.33 (0.29–0.35) | 0.39 | 1.34 |
| `hat-1m` | Kaydırma, yakın (1:1000) | kare (CPU) | 0.51 | 0.62 (0.59–0.63) | 0.71 | 1.60 |
| `hat-1m` | Kaydırma, yakın (1:1000) | kare aralığı (GPU dahil) | 16.7 | 17.3 (17.1–17.5) | 47.0 | 66.0 |
| `hat-1m` | Kaydırma, genel (tümü) | üst katman | 0.22 | 0.27 (0.26–0.30) | 0.34 | 0.76 |
| `hat-1m` | Kaydırma, genel (tümü) | etiketler | 0.05 | 0.08 (0.07–0.08) | 0.10 | 0.29 |
| `hat-1m` | Kaydırma, genel (tümü) | GPU gönderimi | 0.22 | 0.30 (0.29–0.31) | 0.36 | 1.72 |
| `hat-1m` | Kaydırma, genel (tümü) | kare (CPU) | 0.47 | 0.57 (0.55–0.61) | 1.05 | 2.01 |
| `hat-1m` | Kaydırma, genel (tümü) | kare aralığı (GPU dahil) | 46.1 | 48.1 (48.0–48.1) | 92.2 | 92.7 |
| `hat-1m` | Buda önizlemesi, yakın (1:1000) | önizleme (imleç bir kenardayken) | 10.9 | 17.5 (16.8–17.6) | 18.4 | 18.9 |
| `hat-1m` | Buda önizlemesi, yakın (1:1000) | kare (CPU), bütün kareler | 7.26 | 17.4 (17.1–18.3) | 18.9 | 19.1 |
| `hat-1m` | Seç, yakın (1:1000) (vurgu değişince tam çizim) | kare (CPU) | 0.54 | 0.98 (0.92–0.99) | 1.23 | 1.50 |
| `hat-1m` | Çizgi, yakın (1:1000) | kare (CPU) | 0.38 | 0.52 (0.46–0.60) | 0.59 | 1.16 |
| `hat-1m` | Büyük katmanı yeniden kurma (stil değişikliği) | kurma ve yükleme | 39.1 | 52.5 (50.1–53.7) | 52.5 | 53.7 |

## ADR 0005 taslağıyla karşılaştırma (yalnızca kayıt)

| Hedef (taslak) | Ölçülen (p95, koşuların ortancası) | Not |
|---|---|---|
| İmleç hareketinde seçme ve kenet: 100 bin nesnede olay başına < 2 ms | seçme 0.09 / 0.17 ms; kenet 0.11 / 0.41 ms (yakın / genel) | `parsel-50k`, 81 229 nesne |
| Kaydırma: 1 milyon segmentte kare ≤ 16 ms | kare (CPU) 0.62 / 0.57 ms; kare aralığı 17.3 / 48.1 ms (yakın / genel) | `hat-1m`; kare (CPU) yalnız ana iş parçacığıdır, kare aralığı bu makinenin GPU'suyla (başlıktaki) duvar saatidir |
| Bir katmanı yeniden kurma: 100 bin segmentte < 50 ms | 52.5 ms | `hat-1m` büyük katmanı 1 milyon segmenttir; hedef 100 bin segment içindir |
| Bir katmanı yeniden kurma | 38.6 ms | `parsel-50k` `parsel` katmanı, 50 400 alan |

Karşılaştırma bir kabul kararı değildir: hedefler onaylanmadı; ölçüm başsız Chrome'da, ana iş parçacığında alındı.

## Gürültü

- p95'i 0.5 ms'den büyük 48 ölçütte koşular arası yayılım (en yüksek − en düşük p95, ortancaya oranla): ortanca %9, en çok %73 (`hat-1m/rebuild/frame.labels`).
- Tek tek olaylar çöp toplamayla sıçrayabilir: p99 ve “en çok” bu yüzden p95'ten gürültülüdür. Her dizi öncesinde çöp toplama zorlanır.
- Karşılaştırmada iki ölçümün farkı p95 aralıklarının dışına çıkmıyorsa gürültü sayılmalıdır.

## Yöntem

- Veri setleri sayfada, tohumlu üreteçle kurulur ve `kentos.doc.replaceWith` ile açılır (geçmişsiz). Üreteçler ve tohumlar `scripts/perf/datasets.mjs`'tedir; parametreler JSON raporundadır.
- İmleç olayları gerçek `Input.dispatchMouseEvent` hareketleridir. Chrome bir hareketi karesi çizilince yanıtlar; sonraki hareket ancak o zaman gider, böylece olaylar birleşmez ve her olay kendi karesini çizer (her dizide sayılır). Yol, çizim alanının araç kutusu ve komut şeridi dışında kalan kısmında, veri setinin ekrandaki kutusunun içindedir (genel görünümde veri seti tuvali doldurmaz).
- Chrome makinenin GPU'sunda çizer (--use-angle=gl --ignore-gpu-blocklist); kareler Chrome'un kendi 60 Hz hızındadır. SwiftShader (yazılım GPU) ile bu veri setlerinde tek kare saniyeler sürüyor, kareler birikip sayfayı sonradan dakikaya varan sürelerle durduruyordu; ana iş parçacığı süreleri iki durumda da aynıydı.
- Nesne izleme ve bilgi kartı kapalıdır: ikisi de beklemeye (350 ve 500 ms) bağlıdır ve hızlı bir çekirdeğin farklı iş yapmasına yol açardı. Kenet türleri, açıklıklar ve ızgara varsayılandır.
- Google yazı tipleri engellenir (her koşuda aynı yerel yazı tipi, ağ yok). Sunucu bağlantısı yoktur ("Sunucu: yok").
- Başlangıç süreleri bu betikte değil, `scripts/perf/startup.mjs` raporlarındadır.

## Karşılaştırma: baseline (2026-09-24, c110b15) → s6

p95 koşuların ortancasıdır. “Gerileme”: yeni p95, eski ölçümün en yüksek koşusundan %10'dan ve 0.1 ms'den fazla yüksek; “iyileşme”: en düşük koşusundan aynı paylarla düşük. Ölçüm koşulları (makine, Chrome, GPU) aynı değilse karşılaştırma geçersizdir.

Önce: 11th Gen Intel(R) Core(TM) i5-11300H @ 3.10GHz, Google Chrome 153.0.8010.36, ANGLE (Intel, Mesa Intel(R) Iris(R) Xe Graphics (TGL GT2), OpenGL ES 3.2). Şimdi: 11th Gen Intel(R) Core(TM) i5-11300H @ 3.10GHz, Google Chrome 154.0.8037.57, ANGLE (Intel, Mesa Intel(R) Iris(R) Xe Graphics (TGL GT2), OpenGL ES 3.2).

**Makine, Chrome ya da GPU farklı: bu karşılaştırma geçersizdir; önce aynı koşullarda yeni taban alın.**

| Ölçüt | önce p95 | şimdi p95 | oran | sonuç |
|---|---|---|---|---|
| `hat-1m/line-close/move.tool` | 0.04 | 0.15 | 4.29 | gerileme |
| `hat-1m/line-overview/frame.tool` | 0.10 | 0.21 | 2.10 | gerileme |
| `hat-1m/rebuild/frame.render` | 0.13 | 0.26 | 2.08 | gerileme |
| `hat-1m/trim-close/frame.build` | 0.37 | 0.54 | 1.47 | gerileme |
| `parsel-50k/line-close/frame.tool` | 0.09 | 0.20 | 2.11 | gerileme |
| `parsel-50k/line-close/move.tool` | 0.03 | 0.15 | 5.80 | gerileme |
| `parsel-50k/line-overview/frame.tool` | 0.10 | 0.21 | 2.05 | gerileme |
| `parsel-50k/line-overview/move.tool` | 0.02 | 0.14 | 5.60 | gerileme |
| `parsel-50k/pan-close/move.total` | 0.56 | 0.68 | 1.22 | gerileme |
| `parsel-50k/pan-overview/move.total` | 0.60 | 0.82 | 1.37 | gerileme |
| `parsel-50k/select-close/frame.build` | 0.19 | 0.30 | 1.62 | gerileme |
| `hat-1m/line-close/frame.cpu` | 0.80 | 0.52 | 0.65 | iyileşme |
| `hat-1m/line-close/frame.labels` | 0.58 | 0.10 | 0.16 | iyileşme |
| `hat-1m/line-close/frame.overlay` | 0.80 | 0.52 | 0.65 | iyileşme |
| `hat-1m/line-close/move.snap` | 5.71 | 0.93 | 0.16 | iyileşme |
| `hat-1m/line-close/move.total` | 5.81 | 1.07 | 0.18 | iyileşme |
| `hat-1m/line-overview/frame.labels` | 0.35 | 0.09 | 0.26 | iyileşme |
| `hat-1m/line-overview/move.snap` | 31.0 | 2.13 | 0.07 | iyileşme |
| `hat-1m/line-overview/move.total` | 31.1 | 2.25 | 0.07 | iyileşme |
| `hat-1m/pan-close/frame.cpu` | 1.47 | 0.62 | 0.42 | iyileşme |
| `hat-1m/pan-close/frame.labels` | 0.88 | 0.07 | 0.09 | iyileşme |
| `hat-1m/pan-close/frame.overlay` | 1.00 | 0.28 | 0.28 | iyileşme |
| `hat-1m/pan-close/frame.render` | 0.48 | 0.33 | 0.69 | iyileşme |
| `hat-1m/pan-overview/frame.cpu` | 1.59 | 0.57 | 0.36 | iyileşme |
| `hat-1m/pan-overview/frame.labels` | 0.96 | 0.08 | 0.08 | iyileşme |
| `hat-1m/pan-overview/frame.overlay` | 1.17 | 0.27 | 0.23 | iyileşme |
| `hat-1m/rebuild/frame.build` | 131 | 52.5 | 0.40 | iyileşme |
| `hat-1m/rebuild/frame.cpu` | 133 | 53.8 | 0.41 | iyileşme |
| `hat-1m/select-close/frame.cpu` | 1.45 | 0.98 | 0.68 | iyileşme |
| `hat-1m/select-close/frame.labels` | 0.71 | 0.09 | 0.12 | iyileşme |
| `hat-1m/select-close/frame.overlay` | 0.86 | 0.32 | 0.38 | iyileşme |
| `hat-1m/select-close/move.tool` | 3.66 | 1.38 | 0.38 | iyileşme |
| `hat-1m/select-close/move.total` | 3.78 | 1.49 | 0.39 | iyileşme |
| `hat-1m/select-overview/frame.cpu` | 1.44 | 0.84 | 0.59 | iyileşme |
| `hat-1m/select-overview/frame.labels` | 0.70 | 0.08 | 0.11 | iyileşme |
| `hat-1m/select-overview/frame.overlay` | 0.86 | 0.28 | 0.32 | iyileşme |
| `hat-1m/select-overview/move.tool` | 7.15 | 2.86 | 0.40 | iyileşme |
| `hat-1m/select-overview/move.total` | 7.30 | 2.95 | 0.40 | iyileşme |
| `hat-1m/trim-close/frame.cpu` | 754 | 17.4 | 0.02 | iyileşme |
| `hat-1m/trim-close/frame.labels` | 0.50 | 0.06 | 0.12 | iyileşme |
| `hat-1m/trim-close/frame.overlay` | 754 | 16.8 | 0.02 | iyileşme |
| `hat-1m/trim-close/frame.preview` | 763 | 17.5 | 0.02 | iyileşme |
| `hat-1m/trim-close/frame.tool` | 753 | 16.6 | 0.02 | iyileşme |
| `hat-1m/trim-close/move.tool` | 2.83 | 1.03 | 0.36 | iyileşme |
| `hat-1m/trim-close/move.total` | 2.98 | 1.11 | 0.37 | iyileşme |
| `parsel-50k/line-close/frame.cpu` | 6.73 | 0.47 | 0.07 | iyileşme |
| `parsel-50k/line-close/frame.labels` | 6.60 | 0.09 | 0.01 | iyileşme |
| `parsel-50k/line-close/frame.overlay` | 6.73 | 0.47 | 0.07 | iyileşme |
| `parsel-50k/line-close/move.snap` | 9.19 | 0.11 | 0.01 | iyileşme |
| `parsel-50k/line-close/move.total` | 9.28 | 0.32 | 0.04 | iyileşme |
| `parsel-50k/line-overview/frame.cpu` | 9.58 | 0.46 | 0.05 | iyileşme |
| `parsel-50k/line-overview/frame.labels` | 9.40 | 0.09 | 0.01 | iyileşme |
| `parsel-50k/line-overview/frame.overlay` | 9.58 | 0.46 | 0.05 | iyileşme |
| `parsel-50k/line-overview/move.snap` | 12.6 | 0.41 | 0.03 | iyileşme |
| `parsel-50k/line-overview/move.total` | 12.7 | 0.52 | 0.04 | iyileşme |
| `parsel-50k/pan-close/frame.cpu` | 9.04 | 0.63 | 0.07 | iyileşme |
| `parsel-50k/pan-close/frame.labels` | 8.58 | 0.07 | 0.01 | iyileşme |
| `parsel-50k/pan-close/frame.overlay` | 8.68 | 0.28 | 0.03 | iyileşme |
| `parsel-50k/pan-overview/frame.cpu` | 11.8 | 0.67 | 0.06 | iyileşme |
| `parsel-50k/pan-overview/frame.labels` | 11.3 | 0.09 | 0.01 | iyileşme |
| `parsel-50k/pan-overview/frame.overlay` | 11.4 | 0.30 | 0.03 | iyileşme |
| `parsel-50k/rebuild/frame.build` | 112 | 38.6 | 0.35 | iyileşme |
| `parsel-50k/rebuild/frame.cpu` | 123 | 41.3 | 0.34 | iyileşme |
| `parsel-50k/rebuild/frame.labels` | 16.5 | 2.60 | 0.16 | iyileşme |
| `parsel-50k/rebuild/frame.overlay` | 16.7 | 2.92 | 0.17 | iyileşme |
| `parsel-50k/select-close/frame.cpu` | 7.24 | 0.74 | 0.10 | iyileşme |
| `parsel-50k/select-close/frame.labels` | 6.98 | 0.09 | 0.01 | iyileşme |
| `parsel-50k/select-close/frame.overlay` | 7.08 | 0.32 | 0.05 | iyileşme |
| `parsel-50k/select-close/move.tool` | 9.21 | 0.09 | 0.01 | iyileşme |
| `parsel-50k/select-close/move.total` | 9.31 | 0.21 | 0.02 | iyileşme |
| `parsel-50k/select-overview/frame.cpu` | 10.9 | 0.72 | 0.07 | iyileşme |
| `parsel-50k/select-overview/frame.labels` | 10.5 | 0.08 | 0.01 | iyileşme |
| `parsel-50k/select-overview/frame.overlay` | 10.6 | 0.29 | 0.03 | iyileşme |
| `parsel-50k/select-overview/move.tool` | 9.87 | 0.17 | 0.02 | iyileşme |
| `parsel-50k/select-overview/move.total` | 9.91 | 0.27 | 0.03 | iyileşme |
| `parsel-50k/trim-close/frame.cpu` | 15.0 | 1.02 | 0.07 | iyileşme |
| `parsel-50k/trim-close/frame.labels` | 7.40 | 0.07 | 0.01 | iyileşme |
| `parsel-50k/trim-close/frame.overlay` | 14.6 | 0.63 | 0.04 | iyileşme |
| `parsel-50k/trim-close/frame.preview` | 8.47 | 0.55 | 0.06 | iyileşme |
| `parsel-50k/trim-close/frame.tool` | 7.77 | 0.41 | 0.05 | iyileşme |
| `parsel-50k/trim-close/move.tool` | 13.3 | 0.12 | 0.01 | iyileşme |
| `parsel-50k/trim-close/move.total` | 13.4 | 0.22 | 0.02 | iyileşme |
| `hat-1m/line-close/frame.tool` | 0.12 | 0.20 | 1.67 | fark yok |
| `hat-1m/line-overview/frame.cpu` | 0.52 | 0.50 | 0.97 | fark yok |
| `hat-1m/line-overview/frame.overlay` | 0.52 | 0.50 | 0.97 | fark yok |
| `hat-1m/line-overview/move.tool` | 0.02 | 0.04 | 1.40 | fark yok |
| `hat-1m/pan-close/frame.build` | 0.04 | 0.04 | 1.00 | fark yok |
| `hat-1m/pan-close/frame.interval` | 17.1 | 17.3 | 1.01 | fark yok |
| `hat-1m/pan-close/frame.tool` | 0.01 | 0.01 | 1.00 | fark yok |
| `hat-1m/pan-close/move.snap` | 0.02 | 0.02 | 1.00 | fark yok |
| `hat-1m/pan-close/move.tool` | 0.01 | 0.01 | 1.00 | fark yok |
| `hat-1m/pan-close/move.total` | 0.64 | 0.75 | 1.16 | fark yok |
| `hat-1m/pan-overview/frame.build` | 0.05 | 0.03 | 0.50 | fark yok |
| `hat-1m/pan-overview/frame.interval` | 48.1 | 48.1 | 1.00 | fark yok |
| `hat-1m/pan-overview/frame.render` | 0.40 | 0.30 | 0.76 | fark yok |
| `hat-1m/pan-overview/frame.tool` | 0.01 | 0.01 | 1.00 | fark yok |
| `hat-1m/pan-overview/move.snap` | 0.02 | 0.02 | 0.80 | fark yok |
| `hat-1m/pan-overview/move.tool` | 0.01 | 0.01 | 1.00 | fark yok |
| `hat-1m/pan-overview/move.total` | 0.82 | 0.75 | 0.90 | fark yok |
| `hat-1m/rebuild/frame.labels` | 1.45 | 1.45 | 1.00 | fark yok |
| `hat-1m/rebuild/frame.overlay` | 1.51 | 1.98 | 1.31 | fark yok |
| `hat-1m/rebuild/frame.tool` | 0.01 | 0.01 | 1.00 | fark yok |
| `hat-1m/select-close/frame.build` | 0.52 | 0.55 | 1.07 | fark yok |
| `hat-1m/select-close/frame.render` | 0.19 | 0.19 | 1.03 | fark yok |
| `hat-1m/select-close/frame.tool` | 0.01 | 0.01 | 0.50 | fark yok |
| `hat-1m/select-close/move.snap` | 0.02 | 0.02 | 1.00 | fark yok |
| `hat-1m/select-overview/frame.build` | 0.47 | 0.44 | 0.94 | fark yok |
| `hat-1m/select-overview/frame.render` | 0.16 | 0.18 | 1.12 | fark yok |
| `hat-1m/select-overview/frame.tool` | 0.01 | 0.01 | 2.00 | fark yok |
| `hat-1m/select-overview/move.snap` | 0.02 | 0.02 | 0.75 | fark yok |
| `hat-1m/trim-close/frame.render` | 0.14 | 0.16 | 1.19 | fark yok |
| `hat-1m/trim-close/move.snap` | 0.02 | 0.02 | 1.33 | fark yok |
| `parsel-50k/pan-close/frame.build` | 0.05 | 0.04 | 0.78 | fark yok |
| `parsel-50k/pan-close/frame.interval` | 17.0 | 17.0 | 1.00 | fark yok |
| `parsel-50k/pan-close/frame.render` | 0.40 | 0.34 | 0.86 | fark yok |
| `parsel-50k/pan-close/frame.tool` | 0.01 | 0.01 | 2.00 | fark yok |
| `parsel-50k/pan-close/move.snap` | 0.02 | 0.02 | 1.00 | fark yok |
| `parsel-50k/pan-close/move.tool` | 0.00 | 0.01 | 1.00 | fark yok |
| `parsel-50k/pan-overview/frame.build` | 0.05 | 0.04 | 0.80 | fark yok |
| `parsel-50k/pan-overview/frame.interval` | 26.4 | 27.6 | 1.05 | fark yok |
| `parsel-50k/pan-overview/frame.render` | 0.32 | 0.36 | 1.12 | fark yok |
| `parsel-50k/pan-overview/frame.tool` | 0.01 | 0.01 | 2.00 | fark yok |
| `parsel-50k/pan-overview/move.snap` | 0.02 | 0.02 | 1.33 | fark yok |
| `parsel-50k/pan-overview/move.tool` | 0.00 | 0.01 | 1.00 | fark yok |
| `parsel-50k/rebuild/frame.render` | 0.17 | 0.17 | 0.97 | fark yok |
| `parsel-50k/rebuild/frame.tool` | 0.01 | 0.01 | 1.00 | fark yok |
| `parsel-50k/select-close/frame.render` | 0.12 | 0.18 | 1.57 | fark yok |
| `parsel-50k/select-close/frame.tool` | 0.01 | 0.01 | 1.00 | fark yok |
| `parsel-50k/select-close/move.snap` | 0.02 | 0.03 | 1.67 | fark yok |
| `parsel-50k/select-overview/frame.build` | 0.20 | 0.30 | 1.50 | fark yok |
| `parsel-50k/select-overview/frame.render` | 0.12 | 0.18 | 1.54 | fark yok |
| `parsel-50k/select-overview/frame.tool` | 0.01 | 0.01 | 2.00 | fark yok |
| `parsel-50k/select-overview/move.snap` | 0.02 | 0.02 | 1.25 | fark yok |
| `parsel-50k/trim-close/frame.build` | 0.17 | 0.25 | 1.47 | fark yok |
| `parsel-50k/trim-close/frame.render` | 0.11 | 0.17 | 1.43 | fark yok |
| `parsel-50k/trim-close/move.snap` | 0.02 | 0.02 | 1.33 | fark yok |

**11 ölçütte gerileme var.**
