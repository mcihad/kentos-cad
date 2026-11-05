# Büyük karma 2B çizim: açılış, kare, yazı girişi, komut, iptal, bellek (6 Kasım 2026)

`plan/cad-advanced.md` CAD-ADV-19, [ADR 1062](../adr/1062-large-2d-drawing-fluency-and-stability.md). Ham veri:
[önce](large-desktop-once-2026-11-05.json) (düzeltmeden önceki ikili, aynı ölçüm kodu),
[sonra](large-desktop-sonra-2026-11-05.json) (ADR 1062 §4 ile). JSON'ların adındaki tarih UTC'dir. Tablolar
`scripts/perf/large_report.py` ile üretildi; elle sayı girilmedi.

```text
KENTOS_PERF_LABEL=sonra KENTOS_PERF_OUT=docs/perf KENTOS_PERF_SIZES=10000,100000,1000000 \
  cargo test --release -p kentos-desktop --lib perf::large::large -- --ignored --nocapture --test-threads=1
python3 scripts/perf/large_report.py docs/perf/large-desktop-sonra-2026-11-05.json docs/perf/large-desktop-once-2026-11-05.json
```

- Ölçüm boyunca ortak ağır iş kilidi (`/tmp/kentos-heavy.lock`) tutuldu; bu ajanın başka ağır işi yoktu. Makinedeki
  öbür ajanların kilitsiz işi dışlanamaz.
- Her boyut kendi sürecinde; “ilk” süreçteki ilk iz, “ısınmış” aynı izin ikinci koşusudur.
- GPU sütunu başsız karenin çizimi ve geri okumasıdır (pencerede geri okuma yoktur); boş pencerenin tabanı aşağıda.
- IOAccelerator sütunu birleşik bellekte görünümün tamponlarını kapsamıyor (1 000 000 nesnede 2 GB'lık tampona karşı
  36–56 MB); GPU belleği için “görünümün tamponları” sütunu esas alınmalı.
- Uzak tekerlekte MİB'in p95 karesinin payı bileşenlerin çizim kaydıdır (100 000 ve 1 000 000 nesnede ~34 ms, ekrandaki
  büyütülmüş yazılar); ayrıntı JSON'da `uiP95Parts` (olay, uygulama, görünüm, düzen, çizim).

Ortam: Apple M5, 10 iş parçacığı, 32 GB; macOS 26.6.2 (25G83) (25.6.0); rustc 1.96.0 (ac68faa20 2026-05-25); GPU Apple M5 (Metal, ); commit `e6145d28c` (kaydedilmemiş değişiklikle); release (lto thin); pencere 1440×900; tohum 19.

| Nesne | Dosya | Parmak izi | İş parçacığı (oku, çöz, belge) | İlk kare MİB / GPU | Açılıştan ilk kareye | Açılışta karelerin MİB p95 |
|---:|---:|---|---:|---:|---:|---:|
| 10000 | 2.0 MB | `7f200b185434b8be` | 23,7 | 57,4 / 16,2 | 98,0 | 2,8 |
| 100000 | 20.0 MB | `a0142ee95cc85d46` | 155,2 | 240,0 / 52,6 | 447,9 | 0,4 |
| 1000000 | 199.9 MB | `c48958125ed2220e` | 1646,1 | 2545,6 / 450,7 | 4642,4 | 0,4 |

İlk etkileşim (açılıştan sonraki ilk girdi), MİB / GPU ms:

| Nesne | İmleç | Tekerlek | Kaydırma |
|---:|---:|---:|---:|
| 10000 | 21,2 / 55,5 | 20,7 / 32,1 | 1,2 / 25,7 |
| 100000 | 25,4 / 395,2 | 26,1 / 381,1 | 0,8 / 236,6 |
| 1000000 | 17,6 / 7284,6 | 17,9 / 6550,0 | 1,1 / 1793,1 |

Kareler (MİB: arayüz iş parçacığı; GPU: çizim ve geri okuma). Bütçe 16,7 ms (60 Hz).

| Nesne | Koşu | Görünüm | İz | Kare | MİB p95 önce → sonra | MİB p99 sonra | Bütçeyi aşan önce → sonra | GPU p50 önce → sonra | GPU p95 sonra |
|---|---|---|---|---|---|---|---|---|---|
| 10000 | ilk | uzak | imleç | 120 | 0,5 → 0,4 | 0,7 | 0 → 0 | 2,5 → 2,5 | 4,1 |
| 10000 | ilk | uzak | tekerlek | 32 | 27,7 → 25,9 | 31,0 | 7 → 7 | 17,9 → 17,8 | 25,6 |
| 10000 | ilk | uzak | kaydırma | 40 | 1,3 → 1,3 | 1,5 | 0 → 0 | 23,8 → 23,7 | 25,7 |
| 10000 | ilk | yakın 1:200 | imleç | 60 | 0,5 → 0,7 | 0,8 | 0 → 0 | 2,7 → 2,6 | 5,0 |
| 10000 | ilk | yakın 1:200 | tekerlek | 32 | 2,3 → 2,2 | 2,7 | 0 → 0 | 5,6 → 5,6 | 6,9 |
| 10000 | ilk | yakın 1:200 | kaydırma | 40 | 0,8 → 0,7 | 0,9 | 0 → 0 | 5,8 → 7,5 | 8,0 |
| 10000 | ısınmış | uzak | imleç | 120 | 0,4 → 0,5 | 1,0 | 0 → 0 | 2,6 → 2,5 | 4,6 |
| 10000 | ısınmış | uzak | tekerlek | 32 | 1,4 → 1,4 | 1,6 | 0 → 0 | 17,6 → 17,2 | 25,3 |
| 10000 | ısınmış | uzak | kaydırma | 40 | 0,7 → 1,2 | 1,6 | 0 → 0 | 22,9 → 23,7 | 26,2 |
| 10000 | ısınmış | yakın 1:200 | imleç | 60 | 0,6 → 0,6 | 0,8 | 0 → 0 | 2,7 → 2,6 | 5,0 |
| 10000 | ısınmış | yakın 1:200 | tekerlek | 32 | 0,4 → 0,8 | 1,1 | 0 → 0 | 4,2 → 5,1 | 6,2 |
| 10000 | ısınmış | yakın 1:200 | kaydırma | 40 | 0,5 → 0,7 | 1,2 | 0 → 0 | 7,4 → 6,4 | 8,1 |
| 100000 | ilk | uzak | imleç | 120 | 0,4 → 0,4 | 0,4 | 0 → 0 | 2,6 → 2,6 | 4,2 |
| 100000 | ilk | uzak | tekerlek | 32 | 55,0 → 51,6 | 57,5 | 26 → 24 | 138,2 → 132,5 | 231,5 |
| 100000 | ilk | uzak | kaydırma | 40 | 22,0 → 13,5 | 17,4 | 11 → 1 | 219,2 → 216,0 | 236,1 |
| 100000 | ilk | yakın 1:200 | imleç | 60 | 0,4 → 0,5 | 0,6 | 0 → 0 | 2,8 → 2,8 | 4,3 |
| 100000 | ilk | yakın 1:200 | tekerlek | 32 | 3,1 → 3,5 | 4,7 | 0 → 0 | 5,9 → 5,8 | 8,0 |
| 100000 | ilk | yakın 1:200 | kaydırma | 40 | 0,8 → 0,7 | 0,8 | 0 → 0 | 7,5 → 6,7 | 11,2 |
| 100000 | ısınmış | uzak | imleç | 120 | 0,4 → 0,4 | 0,4 | 0 → 0 | 2,7 → 2,7 | 4,3 |
| 100000 | ısınmış | uzak | tekerlek | 32 | 54,9 → 35,0 | 37,7 | 27 → 26 | 134,1 → 131,0 | 221,5 |
| 100000 | ısınmış | uzak | kaydırma | 40 | 22,5 → 8,0 | 17,6 | 8 → 1 | 217,3 → 218,8 | 236,2 |
| 100000 | ısınmış | yakın 1:200 | imleç | 60 | 0,6 → 0,6 | 0,9 | 0 → 0 | 3,5 → 2,7 | 4,8 |
| 100000 | ısınmış | yakın 1:200 | tekerlek | 32 | 3,3 → 2,8 | 3,8 | 0 → 0 | 6,0 → 5,9 | 8,1 |
| 100000 | ısınmış | yakın 1:200 | kaydırma | 40 | 1,2 → 0,8 | 1,0 | 0 → 0 | 6,7 → 6,5 | 10,1 |
| 1000000 | ilk | uzak | imleç | 120 | 0,8 → 0,9 | 0,9 | 0 → 0 | 6,5 → 6,3 | 7,9 |
| 1000000 | ilk | uzak | tekerlek | 32 | 54,9 → 55,3 | 76,5 | 29 → 28 | 1070,8 → 1070,0 | 1728,3 |
| 1000000 | ilk | uzak | kaydırma | 40 | 20,0 → 12,3 | 15,3 | 11 → 0 | 1757,0 → 1769,3 | 1842,9 |
| 1000000 | ilk | yakın 1:200 | imleç | 60 | 0,5 → 0,5 | 0,5 | 0 → 0 | 4,4 → 4,5 | 5,9 |
| 1000000 | ilk | yakın 1:200 | tekerlek | 32 | 3,0 → 2,9 | 24,1 | 1 → 1 | 10,2 → 10,5 | 17,2 |
| 1000000 | ilk | yakın 1:200 | kaydırma | 40 | 0,8 → 0,8 | 0,9 | 0 → 0 | 10,1 → 10,3 | 15,4 |
| 1000000 | ısınmış | uzak | imleç | 120 | 0,8 → 0,8 | 0,9 | 0 → 0 | 5,4 → 5,3 | 6,5 |
| 1000000 | ısınmış | uzak | tekerlek | 32 | 55,0 → 34,5 | 54,5 | 31 → 28 | 1067,4 → 1066,8 | 1733,2 |
| 1000000 | ısınmış | uzak | kaydırma | 40 | 19,9 → 11,3 | 12,5 | 14 → 0 | 1756,9 → 1764,6 | 1852,3 |
| 1000000 | ısınmış | yakın 1:200 | imleç | 60 | 0,6 → 0,5 | 0,5 | 0 → 0 | 4,4 → 4,3 | 5,8 |
| 1000000 | ısınmış | yakın 1:200 | tekerlek | 32 | 3,0 → 2,5 | 22,9 | 1 → 1 | 9,6 → 9,5 | 11,2 |
| 1000000 | ısınmış | yakın 1:200 | kaydırma | 40 | 0,8 → 0,9 | 1,2 | 0 → 0 | 8,1 → 8,8 | 19,2 |

Geri okuma tabanı (boş pencerenin karesi, aynı yolla), GPU p50: 10000: 2,2 ms, 100000: 2,3 ms, 1000000: 2,3 ms.

Yazı girişi (bir tuşun karesi; yerel yankı = MİB + GPU), ms:

| Nesne | Komut satırı MİB p95 / GPU p95 | Değer alanı MİB p95 / GPU p95 | En kötü yankı (MİB en çok + GPU en çok) |
|---:|---:|---:|---:|
| 10000 | 1,9 / 8,3 | 0,6 / 8,1 | 11,6 |
| 100000 | 0,8 / 8,6 | 0,4 / 7,9 | 9,9 |
| 1000000 | 1,0 / 14,7 | 0,6 / 11,4 | 16,3 |

Komutlar (bir kare; parçalar: yeniden kurulan stilli katman parçası / bütün parçalar), ms:

| Nesne | Komut | MİB | GPU | Kurulan parça | Kurma |
|---:|---|---:|---:|---:|---:|
| 10000 | İlk komut (bir yazıyı taşı) | 291,1 | 16,3 | 1 / 6 | 0,6 |
| 10000 | Bir yazıyı taşı | 1,2 | 4,0 | 1 / 6 | 0,3 |
| 10000 | Bir yazının metnini değiştir | 4,5 | 4,0 | 1 / 6 | 0,2 |
| 10000 | Bir blok yerleşimini taşı | 18,8 | 8,4 | 1 / 6 | 18,2 |
| 10000 | Bir çizgiyi sil | 3,7 | 7,4 | 1 / 6 | 1,9 |
| 10000 | Geri al | 3,0 | 5,8 | 1 / 6 | 2,0 |
| 10000 | Yinele | 2,2 | 7,3 | 1 / 6 | 1,8 |
| 10000 | Hepsini seç (Ctrl+A) | 18,0 | 6,7 | 0 / 6 | – |
| 10000 | Seçimi bırak (Esc) | 0,8 | 7,1 | – | – |
| 100000 | İlk komut (bir yazıyı taşı) | 243,4 | 11,5 | 1 / 102 | 0,3 |
| 100000 | Bir yazıyı taşı | 0,7 | 4,1 | 1 / 102 | 0,2 |
| 100000 | Bir yazının metnini değiştir | 0,8 | 4,2 | 1 / 102 | 0,2 |
| 100000 | Bir blok yerleşimini taşı | 7,6 | 6,2 | 1 / 102 | 7,1 |
| 100000 | Bir çizgiyi sil | 1,6 | 4,4 | 1 / 102 | 1,1 |
| 100000 | Geri al | 1,6 | 4,1 | 1 / 102 | 1,0 |
| 100000 | Yinele | 1,6 | 4,4 | 1 / 102 | 1,1 |
| 100000 | Hepsini seç (Ctrl+A) | 172,3 | 25,6 | 0 / 102 | – |
| 100000 | Seçimi bırak (Esc) | 0,5 | 6,0 | – | – |
| 1000000 | İlk komut (bir yazıyı taşı) | 259,7 | 15,7 | 1 / 1470 | 20,7 |
| 1000000 | Bir yazıyı taşı | 21,0 | 6,6 | 1 / 1470 | 20,2 |
| 1000000 | Bir yazının metnini değiştir | 21,2 | 6,9 | 1 / 1470 | 20,1 |
| 1000000 | Bir blok yerleşimini taşı | 28,0 | 9,1 | 1 / 1470 | 27,3 |
| 1000000 | Bir çizgiyi sil | 21,7 | 6,8 | 1 / 1470 | 20,8 |
| 1000000 | Geri al | 21,8 | 8,0 | 1 / 1470 | 21,0 |
| 1000000 | Yinele | 21,9 | 7,9 | 1 / 1470 | 21,1 |
| 1000000 | Hepsini seç (Ctrl+A) | 2256,3 | 709,7 | 0 / 1470 | – |
| 1000000 | Seçimi bırak (Esc) | 1,6 | 9,4 | – | – |

İptal (ikinci açılış, okurken Esc), ms:

| Nesne | Esc'nin karesi MİB + GPU | Pencere o karede kapandı | İş parçacığının durması (Esc'den sonra) | Hiçbir şey uygulanmadı |
|---:|---:|---|---:|---|
| 10000 | 4,0 | evet | 1,6 | evet |
| 100000 | 4,8 | evet | 1,7 | evet |
| 1000000 | 5,1 | evet | 1,1 | evet |

Bellek, MB (`footprint`: fiziksel ayak izi ve sürecin tepesi; GPU: IOAccelerator; görünümün GPU tamponları `FrameStats`):

| Nesne | Açılışta yerleşik | Açılışta ayak izi / tepe | Sonda ayak izi / tepe | GPU (IOAccelerator) açılış / son | Görünümün tamponları |
|---:|---:|---:|---:|---:|---:|
| 10000 | 329 | 492 / 492 | 836 / 1227 | 56 / 54 | 36 |
| 100000 | 852 | 1167 / 1336 | 3030 / 3195 | 50 / 59 | 222 |
| 1000000 | 6349 | 8417 / 10240 | 15360 / 16384 | 36 / 56 | 2081 |
