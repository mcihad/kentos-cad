# Büyük karma 2B çizim: kaydırırken tutulan resmin taşınması (6 Kasım 2026)

`plan/cad-advanced.md` CAD-ADV-19, [ADR 1067](../adr/1067-moved-picture-while-panning.md). Ölçüm düzeni ve veri seti ADR
1062'ninkidir. Ham veri: [önce](large-desktop-adr1067-once-2026-11-06.json) (`origin/main` `483a4cc09` ikilisi),
[sonra](large-desktop-adr1067-sonra-2026-11-06.json) (bu dalın ikilisi). İkisi aynı makinede arka arkaya, aynı sabit izle,
ikisi de sembol tablosu korunarak derlendi (kod aynı); tablolar `scripts/perf/large_report.py` ile üretildi, elle sayı
girilmedi.

```text
KENTOS_PERF_LABEL=adr1067-once|adr1067-sonra KENTOS_PERF_OUT=docs/perf KENTOS_PERF_SIZES=10000,100000,1000000 \
  cargo test --release -p kentos-desktop --lib perf::large::large -- --ignored --nocapture --test-threads=1
python3 scripts/perf/large_report.py docs/perf/large-desktop-adr1067-sonra-2026-11-06.json \
  docs/perf/large-desktop-adr1067-once-2026-11-06.json
```

- Ölçüm boyunca ortak ağır iş kilidi (`/tmp/kentos-heavy.lock`) tutuldu; makinedeki öbür ajanların kilitsiz işi
  dışlanamaz. İki JSON'un “kaydedilmemiş değişiklik” notu ölçüm anındaki çalışma ağacınındır.
- İzin kaydırması orta tuşla 40 adımdır, her adım 9/5 px (tam piksel): her karede sağdan ve alttan iki şerit açılır.
- Açılış, ilk etkileşim, komut, iptal ve bellek tabloları yalnız “sonra”nındır; “önce”nin aynı satırları: ilk kare MİB /
  GPU 10⁴ 22,1 / 19,2, 10⁵ 179,3 / 39,0, 10⁶ 1 523,0 / 233,9 ms; görünümün tamponları 24 / 109 / 959 MB.

Ortam: Apple M5, 10 iş parçacığı, 32 GB; macOS 26.6.2 (25G83) (25.6.0); rustc 1.96.0 (ac68faa20 2026-05-25); GPU Apple M5 (Metal, ); commit `483a4cc09` (kaydedilmemiş değişiklikle); release (lto thin); pencere 1440×900; tohum 19.

| Nesne | Dosya | Parmak izi | İş parçacığı (oku, çöz, belge) | İlk kare MİB / GPU | Açılıştan ilk kareye | Açılışta karelerin MİB p95 |
|---:|---:|---|---:|---:|---:|---:|
| 10000 | 2.0 MB | `7f200b185434b8be` | 23,4 | 44,2 / 19,5 | 88,1 | 3,1 |
| 100000 | 20.0 MB | `a0142ee95cc85d46` | 149,6 | 176,6 / 35,4 | 361,8 | 0,3 |
| 1000000 | 199.9 MB | `c48958125ed2220e` | 1598,3 | 1445,6 / 230,5 | 3274,5 | 0,4 |

İlk etkileşim (açılıştan sonraki ilk girdi), MİB / GPU ms:

| Nesne | İmleç | Tekerlek | Kaydırma |
|---:|---:|---:|---:|
| 10000 | 23,8 / 25,3 | 0,7 / 10,2 | 1,3 / 5,8 |
| 100000 | 25,9 / 24,7 | 1,0 / 23,0 | 1,2 / 5,0 |
| 1000000 | 18,5 / 179,6 | 1,8 / 173,2 | 1,4 / 8,7 |

Kareler (MİB: arayüz iş parçacığı; GPU: çizim ve geri okuma). Bütçe 16,7 ms (60 Hz).

| Nesne | Koşu | Görünüm | İz | Kare | MİB p95 önce → sonra | MİB p99 sonra | Bütçeyi aşan önce → sonra | GPU p50 önce → sonra | GPU p95 sonra |
|---|---|---|---|---|---|---|---|---|---|
| 10000 | ilk | uzak | imleç | 120 | 0,4 → 0,5 | 0,7 | 0 → 0 | 3,9 → 7,2 | 8,5 |
| 10000 | ilk | uzak | tekerlek | 32 | 1,6 → 1,5 | 2,0 | 0 → 0 | 8,2 → 7,5 | 12,3 |
| 10000 | ilk | uzak | kaydırma | 40 | 1,3 → 1,1 | 1,5 | 0 → 0 | 7,3 → 4,0 | 5,1 |
| 10000 | ilk | yakın 1:200 | imleç | 60 | 0,7 → 0,6 | 0,8 | 0 → 0 | 7,3 → 6,6 | 7,7 |
| 10000 | ilk | yakın 1:200 | tekerlek | 32 | 2,8 → 2,6 | 3,4 | 0 → 0 | 7,5 → 7,2 | 9,8 |
| 10000 | ilk | yakın 1:200 | kaydırma | 40 | 0,9 → 0,9 | 1,1 | 0 → 0 | 7,2 → 6,0 | 12,1 |
| 10000 | ısınmış | uzak | imleç | 120 | 0,5 → 0,8 | 0,8 | 0 → 0 | 2,6 → 7,4 | 8,9 |
| 10000 | ısınmış | uzak | tekerlek | 32 | 1,3 → 1,4 | 1,6 | 0 → 0 | 7,1 → 7,4 | 11,7 |
| 10000 | ısınmış | uzak | kaydırma | 40 | 1,3 → 0,9 | 1,5 | 0 → 0 | 7,4 → 3,0 | 4,6 |
| 10000 | ısınmış | yakın 1:200 | imleç | 60 | 0,7 → 0,6 | 0,7 | 0 → 0 | 7,4 → 7,3 | 9,0 |
| 10000 | ısınmış | yakın 1:200 | tekerlek | 32 | 1,0 → 1,0 | 1,2 | 0 → 0 | 6,1 → 7,1 | 8,0 |
| 10000 | ısınmış | yakın 1:200 | kaydırma | 40 | 0,8 → 0,9 | 0,9 | 0 → 0 | 6,4 → 6,3 | 8,0 |
| 100000 | ilk | uzak | imleç | 120 | 0,4 → 0,4 | 0,5 | 0 → 0 | 2,7 → 2,8 | 4,3 |
| 100000 | ilk | uzak | tekerlek | 32 | 1,4 → 1,2 | 1,3 | 0 → 0 | 15,5 → 16,8 | 22,9 |
| 100000 | ilk | uzak | kaydırma | 40 | 1,1 → 0,9 | 0,9 | 0 → 0 | 10,7 → 2,9 | 4,2 |
| 100000 | ilk | yakın 1:200 | imleç | 60 | 0,6 → 0,5 | 0,7 | 0 → 0 | 2,9 → 4,2 | 5,7 |
| 100000 | ilk | yakın 1:200 | tekerlek | 32 | 3,2 → 3,8 | 5,0 | 0 → 0 | 6,0 → 7,6 | 10,2 |
| 100000 | ilk | yakın 1:200 | kaydırma | 40 | 0,9 → 0,8 | 1,2 | 0 → 0 | 5,8 → 7,5 | 11,5 |
| 100000 | ısınmış | uzak | imleç | 120 | 0,4 → 0,5 | 0,6 | 0 → 0 | 2,8 → 4,1 | 5,9 |
| 100000 | ısınmış | uzak | tekerlek | 32 | 1,3 → 1,2 | 1,3 | 0 → 0 | 16,9 → 15,3 | 23,2 |
| 100000 | ısınmış | uzak | kaydırma | 40 | 1,1 → 0,9 | 1,2 | 0 → 0 | 10,8 → 2,8 | 4,4 |
| 100000 | ısınmış | yakın 1:200 | imleç | 60 | 0,6 → 0,6 | 0,7 | 0 → 0 | 3,9 → 3,5 | 5,7 |
| 100000 | ısınmış | yakın 1:200 | tekerlek | 32 | 1,1 → 1,2 | 1,5 | 0 → 0 | 6,5 → 7,6 | 10,6 |
| 100000 | ısınmış | yakın 1:200 | kaydırma | 40 | 0,9 → 0,7 | 0,9 | 0 → 0 | 7,5 → 7,2 | 8,3 |
| 1000000 | ilk | uzak | imleç | 120 | 0,8 → 0,8 | 0,9 | 0 → 0 | 5,6 → 7,3 | 7,5 |
| 1000000 | ilk | uzak | tekerlek | 32 | 1,8 → 1,9 | 23,9 | 1 → 1 | 64,5 → 67,4 | 112,1 |
| 1000000 | ilk | uzak | kaydırma | 40 | 1,8 → 1,6 | 1,8 | 0 → 0 | 110,3 → 8,8 | 12,5 |
| 1000000 | ilk | yakın 1:200 | imleç | 60 | 0,5 → 0,6 | 0,6 | 0 → 0 | 4,9 → 6,7 | 7,0 |
| 1000000 | ilk | yakın 1:200 | tekerlek | 32 | 3,2 → 3,3 | 25,0 | 1 → 1 | 8,0 → 9,3 | 17,1 |
| 1000000 | ilk | yakın 1:200 | kaydırma | 40 | 1,0 → 0,9 | 1,3 | 0 → 0 | 11,6 → 11,2 | 17,2 |
| 1000000 | ısınmış | uzak | imleç | 120 | 0,9 → 0,8 | 0,9 | 0 → 0 | 5,7 → 7,3 | 7,9 |
| 1000000 | ısınmış | uzak | tekerlek | 32 | 1,8 → 1,9 | 22,9 | 1 → 1 | 66,4 → 65,0 | 109,6 |
| 1000000 | ısınmış | uzak | kaydırma | 40 | 1,8 → 1,6 | 1,7 | 0 → 0 | 110,7 → 8,9 | 11,5 |
| 1000000 | ısınmış | yakın 1:200 | imleç | 60 | 0,5 → 0,6 | 0,7 | 0 → 0 | 5,0 → 5,8 | 7,0 |
| 1000000 | ısınmış | yakın 1:200 | tekerlek | 32 | 1,2 → 1,0 | 21,9 | 1 → 1 | 10,8 → 9,2 | 13,5 |
| 1000000 | ısınmış | yakın 1:200 | kaydırma | 40 | 0,8 → 0,7 | 1,0 | 0 → 0 | 11,4 → 9,3 | 12,7 |

Geri okuma tabanı (boş pencerenin karesi, aynı yolla), GPU p50: 10000: 3,5 ms, 100000: 2,3 ms, 1000000: 2,3 ms.

Yazı girişi (bir tuşun karesi; yerel yankı = MİB + GPU), ms:

| Nesne | Komut satırı MİB p95 / GPU p95 | Değer alanı MİB p95 / GPU p95 | En kötü yankı (MİB en çok + GPU en çok) |
|---:|---:|---:|---:|
| 10000 | 1,9 / 8,5 | 0,4 / 9,0 | 14,7 |
| 100000 | 0,8 / 7,9 | 0,3 / 6,3 | 9,9 |
| 1000000 | 1,3 / 12,2 | 0,6 / 12,2 | 24,1 |

Komutlar (bir kare; parçalar: yeniden kurulan stilli katman parçası / bütün parçalar), ms:

| Nesne | Komut | MİB | GPU | Kurulan parça | Kurma |
|---:|---|---:|---:|---:|---:|
| 10000 | İlk komut (bir yazıyı taşı) | 287,0 | 11,8 | 1 / 6 | 0,4 |
| 10000 | Bir yazıyı taşı | 1,3 | 4,0 | 1 / 6 | 0,3 |
| 10000 | Bir yazının metnini değiştir | 4,5 | 4,1 | 1 / 6 | 0,3 |
| 10000 | Bir blok yerleşimini taşı | 8,6 | 4,2 | 1 / 6 | 8,1 |
| 10000 | Bir çizgiyi sil | 7,6 | 4,4 | 1 / 6 | 5,9 |
| 10000 | Geri al | 6,7 | 4,6 | 1 / 6 | 5,8 |
| 10000 | Yinele | 6,1 | 4,6 | 1 / 6 | 5,6 |
| 10000 | Hepsini seç (Ctrl+A) | 11,8 | 6,6 | 0 / 6 | – |
| 10000 | Seçimi bırak (Esc) | 0,5 | 5,6 | – | – |
| 100000 | İlk komut (bir yazıyı taşı) | 230,7 | 7,7 | 1 / 102 | 0,2 |
| 100000 | Bir yazıyı taşı | 0,8 | 8,8 | 1 / 102 | 0,2 |
| 100000 | Bir yazının metnini değiştir | 0,8 | 4,1 | 1 / 102 | 0,2 |
| 100000 | Bir blok yerleşimini taşı | 3,9 | 8,8 | 1 / 102 | 3,4 |
| 100000 | Bir çizgiyi sil | 3,4 | 4,6 | 1 / 102 | 2,6 |
| 100000 | Geri al | 3,2 | 4,6 | 1 / 102 | 2,6 |
| 100000 | Yinele | 3,0 | 6,1 | 1 / 102 | 2,4 |
| 100000 | Hepsini seç (Ctrl+A) | 95,6 | 31,6 | 0 / 102 | – |
| 100000 | Seçimi bırak (Esc) | 0,5 | 4,5 | – | – |
| 1000000 | İlk komut (bir yazıyı taşı) | 270,5 | 17,1 | 1 / 1470 | 22,4 |
| 1000000 | Bir yazıyı taşı | 22,2 | 8,0 | 1 / 1470 | 21,3 |
| 1000000 | Bir yazının metnini değiştir | 22,2 | 10,7 | 1 / 1470 | 21,0 |
| 1000000 | Bir blok yerleşimini taşı | 25,3 | 8,2 | 1 / 1470 | 24,5 |
| 1000000 | Bir çizgiyi sil | 24,9 | 9,3 | 1 / 1470 | 24,0 |
| 1000000 | Geri al | 24,3 | 10,7 | 1 / 1470 | 23,5 |
| 1000000 | Yinele | 23,9 | 9,7 | 1 / 1470 | 23,1 |
| 1000000 | Hepsini seç (Ctrl+A) | 1130,9 | 268,5 | 0 / 1470 | – |
| 1000000 | Seçimi bırak (Esc) | 3,7 | 10,1 | – | – |

İptal (ikinci açılış, okurken Esc), ms:

| Nesne | Esc'nin karesi MİB + GPU | Pencere o karede kapandı | İş parçacığının durması (Esc'den sonra) | Hiçbir şey uygulanmadı |
|---:|---:|---|---:|---|
| 10000 | 5,8 | evet | 1,7 | evet |
| 100000 | 3,2 | evet | 1,3 | evet |
| 1000000 | 6,8 | evet | 1,6 | evet |

Bellek, MB (`footprint`: fiziksel ayak izi ve sürecin tepesi; GPU: IOAccelerator; görünümün GPU tamponları `FrameStats`):

| Nesne | Açılışta yerleşik | Açılışta ayak izi / tepe | Sonda ayak izi / tepe | GPU (IOAccelerator) açılış / son | Görünümün tamponları |
|---:|---:|---:|---:|---:|---:|
| 10000 | 310 | 463 / 463 | 578 / 1020 | 45 / 50 | 27 |
| 100000 | 715 | 922 / 983 | 2059 / 2221 | 55 / 53 | 112 |
| 1000000 | 5369 | 6358 / 7218 | 13312 / 14336 | 41 / 57 | 962 |
