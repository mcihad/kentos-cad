# ADR 9248: Veri katmanları sadeleştirilmeden okunur

- **Durum:** kabul edildi (2026-11-25).
- **Plan:** [plan/render.md](../../plan/render.md) RM-12'nin açık sorusu: “Yarım piksellik kaynak sadeleştirmesi resmi
  değiştiriyor mu?”
- **İstek:** 24 Kasım, “kaliteden ödün verilmez” ([ADR 9222](9222-quality-is-not-traded-for-speed.md)).
- **Değiştirdiği:** [ADR 9101](9101-data-layers.md) §6'nın “Sadeleştirme” maddesi ve §8'in 3. maddesi.
- **Sözleşmeler:** sürücü sözleşmesi ([ADR 9100](9100-data-drivers.md), `Request::simplify`) değişmedi. Sürücüler
  sadeleştirmeyi biliyor; model ekranı artık istemiyor.

## §1 Bulgu

Veri katmanının geometrisi o anki ölçeğin yarım pikseline Douglas–Peucker ile sadeleştiriliyordu: PostGIS'te
sunucuda (`ST_Simplify`), GeoPackage'ta okuyucuda. Yalnız düzenlenen katman tam okunuyordu. Bu, ADR 9222'nin kaldırdığı
uzak düzeylerin veri katmanındaki karşılığı.

**Ölçüm:** gerçek pencere (Xvfb ve lavapipe), yalnız çizim alanı. Aynı ikili, tam okuma bir tanı anahtarıyla.

| Veri | Görünüm | Farklı piksel | 5–63 adım | 64+ adım | En çok |
|---|---|---|---|---|---|
| İller: 4 556 il, QGIS'in örnek verisi | Türkiye, 1:6,3 milyon | 2 184 | 1 415 | 71 | 148 |
| | 4 çentik yakın | 126 | 91 | 19 | 99 |
| | 8 çentik yakın | 0 | | | |
| 100 000 sentetik parsel, EPSG:5256 | 1:73 248 | 0 | | | |
| | 4 çentik yakın | 0 | | | |
| | 1:6 645 | 37 360 | 24 832 | 801 | 106 |

- **Silinen şey gerçek geometri.** Parsellerin sol kenarında 0,5 m'lik bir çıkıntı var: (0, 10) → (0,5, 5) → (0, 1).
  1:6 645'te bu 0,28 piksel ediyor. Sadeleştirme onu siliyordu. İllerde de yarım pikselin altındaki kıvrımlar
  siliniyordu; tam okumada sınırın kıvrıldığı yerler daha dolu.
- **Uzakta fark yok.** 100 000 parsel ekranı dolduran koyu bir bant oluyor; iki okuma aynı resmi veriyor.

## §2 Karar

Model ekranı veri katmanını tam okur (`simplify: None`).

- **Basamak kalır.** Okuma ölçeğin çeyrek oktav basamağında yine yenilenir. Böylece yakın görünüm yalnız kendi
  kapsamının satırlarını tutar, etiketler de 20 000 satırın altına inince kurulur (ADR 9101 §6). Basamağın
  sadeleştirmeyle ilgisi kalmadı.
- **Düzenleme bayrağı** (`exact`) artık yalnız nesnelerin yakalama için tutulmasını söylüyor.

## §3 Bedel

**Okuma** (hata ayıklama derlemesi; `DataView::want`'tan okumanın çizilmesine):

| Veri | Görünüm | Sadeleştirilmiş | Tam |
|---|---|---|---|
| 100 000 parsel | uzak, hepsi | 1 918 ms, 400 000 çizgi parçası | 2 698 ms, 1 000 000 |
| | 1:6 645 | 395 ms, 80 000 | 550 ms, 200 000 |
| İller | Türkiye | 134 ms, 28 854 | 124 ms, 29 848 |

- Sentetik parselin halkasında 11 nokta var: düz kenarların ortasında köşeler ve bir çıkıntı. Sadeleştirme her parseli
  4 parçaya indiriyordu, tam okuma 10 parça çiziyor. Oran verinin ayrıntısına bağlı: gerçek il sınırlarında %3,4.
- **Kare:** uzak görünümde süre çizgi parçası sayısıyla büyür. [ADR 9247](9247-stroke-quads-as-strips-with-a-narrower-margin.md)'ye
  göre 1 M parça UHD 770'te tek örnekte ~15 ms.
- 1 milyon parsellik ölçüm hâlâ açık: kentos-portal-db parola istiyor.

## §4 Kanıt

- Gerçek pencerede son kod tam okumanın resmini birebir veriyor: iki çizimin altı görünümünde 0 piksel.
- Masaüstünde veri katmanı, pafta, stil, kaynak ve GeoPackage sınamalarından 431'i geçti.

## Uygulama

- `apps/desktop/src/gis/data_view.rs`: `simplify: None`; `Job::px_per_m` kalktı; `Key::exact` ve `Key::step`'in
  açıklamaları.
