# ADR 9250: Tek örnekli görünüm çizgiyi dar dörtgenle çizer

- **Durum:** kabul edildi (2026-11-25).
- **Plan:** [plan/render.md](../../plan/render.md) RM-10, resmi değiştirmeyen hız.
- **İstek:**
  - kabul ölçütü: “Kamera değişikliğinden onu gösteren kareye ≤ 16 ms”;
  - 24 Kasım: “kaliteden ödün verilmez”.
- **Sözleşmeler:** paylaşılan WGSL değişmedi. Masaüstünün kaynağına `override strokeMargin` geldi; öntanımlısı
  [ADR 9247](9247-stroke-quads-as-strips-with-a-narrower-margin.md)'nin 0,75 px'i.

## §1 Bulgu

**Uzak karenin süresi çizgilerde.** `KENTOS_DIAG_SKIP` ile her tür ayrı ayrı dışarıda bırakıldı (hata ayıklama derlemesi,
`perf::files`, UHD 770, uzak tekerlek p50):

| | Tümü | Çizgisiz | İşaretsiz | Dolgusuz | Yazısız |
|---|---|---|---|---|---|
| NCZ, 4× | 61,1 ms | 28,9 ms | 50,7 ms | 61,1 ms | 59,4 ms |
| sivas.kcad, 4× | 76,8 ms | 20,6 ms | 76,4 ms | 69,1 ms | 77,2 ms |
| sivas.kcad, 1× (sahibin ayarı) | 59,7 ms | 17,7 ms | | 58,4 ms | |

- sivas.kcad'in uzak karesinde 1,08 milyon çizgi parçası var. 1×'te bunlar 42 ms tutuyor, parça başına 39 ns.
  Ölçüm sınamasının kısa çizgilerinde bu 15 ns. Uzak karenin çizgileri daha uzun ve üst üste biniyor; süre çizilen
  piksel sayısında.
- **Dörtgenin payı tek örnekte boşa gidiyor.** Dörtgen, kaplamanın bittiği yerden 0,75 px öteye uzanıyor. Bu payı çok
  örnekli hedef istiyor: en uzak örnek piksel ortasından 0,71 px uzakta. Tek örnekli görünümde ise yalnız piksel ortası
  sayılır: kaplaması olan her piksel ortası zaten yumuşatmanın içinde. İnce bir çizginin dörtgeni 3,5 px enindeyken
  kaplama bandı 2 px.

## §2 Karar

**Tek örnekli görünümün** (`graphics.msaa` 1, ya da HiDPI kapalı) boru hatları çizgi dörtgenini kaplamanın bittiği
yerden 0,05 px öteye uzatır (`ONE_SAMPLE_VIEW_MARGIN`; yuvarlamanın payı). Düz çizgiler ve blokların örnekli çizgileri
bunu alır.

- **Çok örnekli hedefler 0,75'te kalır.** Örneklerde çizim ile üstüne bindirilen tek örnekli koşu (`FLOAT_PICTURE`)
  ikisi de bu payı kullanır.
- **Neden dar pay yalnız burada güvenli:** ADR 9247'nin dersi, aynı kareye giren iki yolun aynı dörtgeni çizmesi
  gerektiğiydi. Tek örnekli görünümde hiçbir şey örneklere çizilmez; hareket karesi de durma karesi de aynı boru
  hattından geçer.
- **Kalan keskin sınırlar:** köşe açıortayının ±0,01 px'lik bandı ve kesik noktasının sahibi. Orada kayan noktanın
  kılı yine bir pikseli öbür yana atabilir.

## §3 Kanıt

**Ölçüm sınaması** (`far_cost`, release, UHD 770, 1 M parça): 1× 14,7 → 12,4 ms (−%16). Çok örnekli yollar aynı
(örneklerde 48,9 ms, tek örnekli koşu 15,5 ms).

**Dosyalar** (hata ayıklama, sahibin ayarı 1×, uzak tekerlek):

| | Önce | Sonra |
|---|---|---|
| NCZ p50 / p95 | 56,9 / 66,1 ms | 49,8 / 58,1 ms (−%12) |
| sivas.kcad p50 / p95 | 59,7 / 67,2 ms | 51,5 / 59,3 ms (−%14) |

Kaydırma ve yakın görünüm değişmedi. Uzak kare ölçütün hâlâ üstünde.

**Resim:**

- GPU sınamaları 1×'te koşuyor ve kaplamanın tam payını denetliyor: uçlar (ADR 9249, ±2/255), köşeler, kesikler,
  kalınlıklar. Hepsi geçti; kaybolan kaplama yok.
- Gerçek pencere (NCZ, 1×, ADR 9249'un ikilisine karşı): dört görüntüde 1 140 / 3 509 / 730 / 209 piksel farklı,
  her birinde en az %96'sı 1 adım. 5 adımı geçen 7 piksel var. En büyüğü (97 adım) çift çizginin başında, köşe
  açıortayının bandına düşen bir piksel.
- Masaüstünde 320 render sınaması (altın görüntüler dahil) ve Mimari'nin render uygunluk sınaması geçti.

## Uygulama

- `crates/render/wgpu/src/styled/shader.rs`: `strokeMargin`, `ONE_SAMPLE_VIEW_MARGIN`.
- `crates/render/wgpu/src/styled/gpu.rs`: tek örnekli görünümün çizgi boru hatlarına sabit.
