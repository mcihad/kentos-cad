# Büyük karma 2B çizim: uzun çizgilerin uzak düzeyleri ve tekerlekte tutulan yazı (6 Kasım 2026)

`plan/cad-advanced.md` CAD-ADV-19, [ADR 1064](../adr/1064-far-view-line-levels-and-held-text.md). Ölçüm düzeni ve
veri seti ADR 1062'ninkidir. Ham veri: [önce](large-desktop-adr1064-once-2026-11-06.json) (`main`'in `3e5087350`
ikilisi: ADR 1063'ün çizimi, örnekleme ve eğri düzeyleri açık), [sonra](large-desktop-adr1064-sonra-2026-11-06.json)
(bu dalın ikilisi, uygulamanın kendi ayarı: üç kısayol açık, tekerlekte yazı yerleştiği boyda). İkisi aynı makinede arka
arkaya, aynı sabit izle; tablolar `scripts/perf/large_report.py` ile üretildi, elle sayı girilmedi.

```text
KENTOS_PERF_LABEL=adr1064-once|adr1064-sonra KENTOS_PERF_OUT=docs/perf KENTOS_PERF_SIZES=10000,100000,1000000 \
  cargo test --release -p kentos-desktop --lib perf::large::large -- --ignored --nocapture --test-threads=1
python3 scripts/perf/large_report.py docs/perf/large-desktop-adr1064-sonra-2026-11-06.json \
  docs/perf/large-desktop-adr1064-once-2026-11-06.json
```

- Ölçüm boyunca ortak ağır iş kilidi (`/tmp/kentos-heavy.lock`) tutuldu; makinedeki öbür ajanların kilitsiz işi
  dışlanamaz. “Önce” ikilisi bu dilimin değişikliklerinden önce derlendi; iki JSON'un “kaydedilmemiş değişiklik” notu
  ölçüm anındaki çalışma ağacınındır. Tablodaki ilk satırların “açılış” ve “ilk etkileşim” sütunları yalnız “sonra”nındır;
  “önce”nin aynı sütunları: ilk kare MİB / GPU 10⁴ 76,0 / 24,0, 10⁵ 294,2 / 33,1, 10⁶ 2 247,8 / 239,5 ms; görünümün
  tamponları 22 / 84 / 709 MB (sonra 24 / 109 / 959 MB); süreç tepe ayak izi 1 052 / 2 294 / 13 312 MB (sonra
  1 013 / 2 192 / 14 336 MB); en kötü tuş yankısı 18,0 / 27,6 / 20,7 ms (sonra 16,6 / 16,9 / 15,6 ms).
- Çizgi düzeylerinin tek başına payı aynı ikilinin `KENTOS_PERF_SHORTCUTS=adr1063` koşusuyla ayrıca ölçüldü
  (`perf::large::layers`, ADR 1064 §7).
- İmleç karelerinin GPU'su bu tabloda da “önce → sonra” artıyor görünür (10⁴: 2,3 → 4,2 ms); karenin işinin
  değişmediği, GPU'nun saatinin öncesindeki yüke bağlı olduğu ADR 1064 §5'te ölçülerek gösterildi.

Ortam: Apple M5, 10 iş parçacığı, 32 GB; macOS 26.6.2 (25G83) (25.6.0); rustc 1.96.0 (ac68faa20 2026-05-25); GPU Apple M5 (Metal, ); commit `3e5087350` (kaydedilmemiş değişiklikle); release (lto thin); pencere 1440×900; tohum 19.

| Nesne | Dosya | Parmak izi | İş parçacığı (oku, çöz, belge) | İlk kare MİB / GPU | Açılıştan ilk kareye | Açılışta karelerin MİB p95 |
|---:|---:|---|---:|---:|---:|---:|
| 10000 | 2.0 MB | `7f200b185434b8be` | 24,2 | 48,3 / 16,7 | 90,1 | 3,0 |
| 100000 | 20.0 MB | `a0142ee95cc85d46` | 150,6 | 239,6 / 39,7 | 430,0 | 0,4 |
| 1000000 | 199.9 MB | `c48958125ed2220e` | 1738,7 | 2239,1 / 272,7 | 4250,6 | 0,7 |

İlk etkileşim (açılıştan sonraki ilk girdi), MİB / GPU ms:

| Nesne | İmleç | Tekerlek | Kaydırma |
|---:|---:|---:|---:|
| 10000 | 21,2 / 21,0 | 0,7 / 9,2 | 1,4 / 7,0 |
| 100000 | 25,2 / 44,8 | 0,9 / 46,5 | 0,8 / 26,6 |
| 1000000 | 18,2 / 494,9 | 1,3 / 479,9 | 1,1 / 307,2 |

Kareler (MİB: arayüz iş parçacığı; GPU: çizim ve geri okuma). Bütçe 16,7 ms (60 Hz).

| Nesne | Koşu | Görünüm | İz | Kare | MİB p95 önce → sonra | MİB p99 sonra | Bütçeyi aşan önce → sonra | GPU p50 önce → sonra | GPU p95 sonra |
|---|---|---|---|---|---|---|---|---|---|
| 10000 | ilk | uzak | imleç | 120 | 0,4 → 0,5 | 0,7 | 0 → 0 | 2,2 → 5,8 | 7,9 |
| 10000 | ilk | uzak | tekerlek | 32 | 26,7 → 1,2 | 1,7 | 7 → 0 | 8,0 → 7,3 | 10,8 |
| 10000 | ilk | uzak | kaydırma | 40 | 0,9 → 1,0 | 1,1 | 0 → 0 | 7,0 → 7,5 | 9,6 |
| 10000 | ilk | yakın 1:200 | imleç | 60 | 0,4 → 0,7 | 0,7 | 0 → 0 | 3,7 → 7,3 | 8,8 |
| 10000 | ilk | yakın 1:200 | tekerlek | 32 | 2,5 → 2,6 | 3,2 | 0 → 0 | 6,4 → 7,3 | 8,4 |
| 10000 | ilk | yakın 1:200 | kaydırma | 40 | 0,6 → 0,8 | 1,2 | 0 → 0 | 7,5 → 7,6 | 9,1 |
| 10000 | ısınmış | uzak | imleç | 120 | 0,5 → 0,5 | 0,9 | 0 → 0 | 2,3 → 4,2 | 6,4 |
| 10000 | ısınmış | uzak | tekerlek | 32 | 1,2 → 1,1 | 1,6 | 0 → 0 | 7,2 → 7,6 | 11,8 |
| 10000 | ısınmış | uzak | kaydırma | 40 | 0,7 → 1,1 | 1,5 | 0 → 0 | 7,6 → 7,4 | 9,6 |
| 10000 | ısınmış | yakın 1:200 | imleç | 60 | 0,4 → 0,5 | 0,6 | 0 → 0 | 3,6 → 4,6 | 7,4 |
| 10000 | ısınmış | yakın 1:200 | tekerlek | 32 | 0,8 → 0,9 | 1,2 | 0 → 0 | 7,6 → 7,7 | 10,8 |
| 10000 | ısınmış | yakın 1:200 | kaydırma | 40 | 0,8 → 0,8 | 1,0 | 0 → 0 | 7,5 → 7,4 | 8,3 |
| 100000 | ilk | uzak | imleç | 120 | 0,4 → 0,4 | 0,5 | 0 → 0 | 2,4 → 2,7 | 4,2 |
| 100000 | ilk | uzak | tekerlek | 32 | 36,5 → 1,2 | 1,3 | 7 → 0 | 29,8 → 22,4 | 32,1 |
| 100000 | ilk | uzak | kaydırma | 40 | 0,9 → 1,0 | 1,1 | 0 → 0 | 45,2 → 25,1 | 28,3 |
| 100000 | ilk | yakın 1:200 | imleç | 60 | 0,6 → 0,9 | 1,0 | 0 → 0 | 2,5 → 3,8 | 8,1 |
| 100000 | ilk | yakın 1:200 | tekerlek | 32 | 3,2 → 3,3 | 4,4 | 0 → 0 | 4,9 → 5,3 | 8,2 |
| 100000 | ilk | yakın 1:200 | kaydırma | 40 | 0,8 → 0,7 | 1,1 | 0 → 0 | 6,3 → 6,0 | 7,8 |
| 100000 | ısınmış | uzak | imleç | 120 | 0,4 → 0,5 | 0,9 | 0 → 0 | 2,5 → 2,8 | 4,5 |
| 100000 | ısınmış | uzak | tekerlek | 32 | 25,7 → 1,4 | 1,9 | 6 → 0 | 28,2 → 21,2 | 32,4 |
| 100000 | ısınmış | uzak | kaydırma | 40 | 0,9 → 1,1 | 1,1 | 0 → 0 | 45,3 → 25,5 | 27,7 |
| 100000 | ısınmış | yakın 1:200 | imleç | 60 | 0,6 → 0,6 | 0,9 | 0 → 0 | 2,4 → 4,1 | 5,1 |
| 100000 | ısınmış | yakın 1:200 | tekerlek | 32 | 0,7 → 0,8 | 0,8 | 0 → 0 | 4,2 → 5,9 | 7,0 |
| 100000 | ısınmış | yakın 1:200 | kaydırma | 40 | 0,7 → 0,9 | 1,3 | 0 → 0 | 7,1 → 6,0 | 9,5 |
| 1000000 | ilk | uzak | imleç | 120 | 1,0 → 1,0 | 1,9 | 0 → 0 | 6,4 → 6,8 | 10,8 |
| 1000000 | ilk | uzak | tekerlek | 32 | 56,9 → 49,2 | 55,6 | 28 → 13 | 273,7 → 165,4 | 300,8 |
| 1000000 | ilk | uzak | kaydırma | 40 | 14,0 → 10,9 | 14,9 | 1 → 0 | 402,1 → 287,0 | 312,0 |
| 1000000 | ilk | yakın 1:200 | imleç | 60 | 0,7 → 0,6 | 0,7 | 0 → 0 | 4,7 → 4,8 | 6,1 |
| 1000000 | ilk | yakın 1:200 | tekerlek | 32 | 3,1 → 3,1 | 24,7 | 1 → 1 | 9,6 → 10,4 | 13,4 |
| 1000000 | ilk | yakın 1:200 | kaydırma | 40 | 1,3 → 1,2 | 4,5 | 0 → 0 | 9,3 → 11,7 | 16,0 |
| 1000000 | ısınmış | uzak | imleç | 120 | 1,0 → 1,0 | 1,1 | 0 → 0 | 5,9 → 5,6 | 6,8 |
| 1000000 | ısınmış | uzak | tekerlek | 32 | 53,7 → 29,3 | 31,5 | 30 → 12 | 273,3 → 162,9 | 290,2 |
| 1000000 | ısınmış | uzak | kaydırma | 40 | 12,7 → 9,2 | 11,0 | 1 → 0 | 406,5 → 281,8 | 309,3 |
| 1000000 | ısınmış | yakın 1:200 | imleç | 60 | 0,7 → 0,6 | 0,7 | 0 → 0 | 4,8 → 5,7 | 7,5 |
| 1000000 | ısınmış | yakın 1:200 | tekerlek | 32 | 3,0 → 2,6 | 24,6 | 1 → 1 | 10,6 → 11,3 | 14,1 |
| 1000000 | ısınmış | yakın 1:200 | kaydırma | 40 | 1,1 → 1,0 | 1,5 | 0 → 0 | 10,0 → 7,9 | 13,6 |

Geri okuma tabanı (boş pencerenin karesi, aynı yolla), GPU p50: 10000: 3,8 ms, 100000: 2,3 ms, 1000000: 2,3 ms.

Yazı girişi (bir tuşun karesi; yerel yankı = MİB + GPU), ms:

| Nesne | Komut satırı MİB p95 / GPU p95 | Değer alanı MİB p95 / GPU p95 | En kötü yankı (MİB en çok + GPU en çok) |
|---:|---:|---:|---:|
| 10000 | 1,6 / 10,3 | 0,4 / 6,3 | 16,6 |
| 100000 | 0,9 / 10,3 | 0,5 / 7,8 | 16,9 |
| 1000000 | 1,5 / 12,2 | 0,9 / 11,6 | 15,6 |

Komutlar (bir kare; parçalar: yeniden kurulan stilli katman parçası / bütün parçalar), ms:

| Nesne | Komut | MİB | GPU | Kurulan parça | Kurma |
|---:|---|---:|---:|---:|---:|
| 10000 | İlk komut (bir yazıyı taşı) | 282,6 | 5,7 | 1 / 6 | 0,6 |
| 10000 | Bir yazıyı taşı | 1,4 | 7,0 | 1 / 6 | 0,2 |
| 10000 | Bir yazının metnini değiştir | 4,3 | 5,6 | 1 / 6 | 0,2 |
| 10000 | Bir blok yerleşimini taşı | 8,6 | 4,0 | 1 / 6 | 7,8 |
| 10000 | Bir çizgiyi sil | 7,7 | 6,1 | 1 / 6 | 6,2 |
| 10000 | Geri al | 6,5 | 7,5 | 1 / 6 | 6,0 |
| 10000 | Yinele | 6,4 | 4,6 | 1 / 6 | 5,9 |
| 10000 | Hepsini seç (Ctrl+A) | 17,8 | 9,3 | 0 / 6 | – |
| 10000 | Seçimi bırak (Esc) | 0,8 | 6,9 | – | – |
| 100000 | İlk komut (bir yazıyı taşı) | 241,3 | 6,0 | 1 / 102 | 0,2 |
| 100000 | Bir yazıyı taşı | 0,8 | 7,2 | 1 / 102 | 0,2 |
| 100000 | Bir yazının metnini değiştir | 0,8 | 5,6 | 1 / 102 | 0,2 |
| 100000 | Bir blok yerleşimini taşı | 3,8 | 5,9 | 1 / 102 | 3,3 |
| 100000 | Bir çizgiyi sil | 3,2 | 4,6 | 1 / 102 | 2,7 |
| 100000 | Geri al | 2,9 | 5,9 | 1 / 102 | 2,4 |
| 100000 | Yinele | 3,0 | 4,6 | 1 / 102 | 2,4 |
| 100000 | Hepsini seç (Ctrl+A) | 173,4 | 29,3 | 0 / 102 | – |
| 100000 | Seçimi bırak (Esc) | 0,6 | 4,7 | – | – |
| 1000000 | İlk komut (bir yazıyı taşı) | 276,1 | 15,8 | 1 / 1470 | 23,1 |
| 1000000 | Bir yazıyı taşı | 22,3 | 8,5 | 1 / 1470 | 21,4 |
| 1000000 | Bir yazının metnini değiştir | 22,4 | 9,6 | 1 / 1470 | 21,3 |
| 1000000 | Bir blok yerleşimini taşı | 26,4 | 8,4 | 1 / 1470 | 25,6 |
| 1000000 | Bir çizgiyi sil | 25,2 | 9,8 | 1 / 1470 | 24,3 |
| 1000000 | Geri al | 24,4 | 9,8 | 1 / 1470 | 23,6 |
| 1000000 | Yinele | 24,2 | 9,7 | 1 / 1470 | 23,4 |
| 1000000 | Hepsini seç (Ctrl+A) | 2019,9 | 307,9 | 0 / 1470 | – |
| 1000000 | Seçimi bırak (Esc) | 1,3 | 10,2 | – | – |

İptal (ikinci açılış, okurken Esc), ms:

| Nesne | Esc'nin karesi MİB + GPU | Pencere o karede kapandı | İş parçacığının durması (Esc'den sonra) | Hiçbir şey uygulanmadı |
|---:|---:|---|---:|---|
| 10000 | 4,4 | evet | 1,7 | evet |
| 100000 | 3,2 | evet | 1,5 | evet |
| 1000000 | 7,1 | evet | 1,0 | evet |

Bellek, MB (`footprint`: fiziksel ayak izi ve sürecin tepesi; GPU: IOAccelerator; görünümün GPU tamponları `FrameStats`):

| Nesne | Açılışta yerleşik | Açılışta ayak izi / tepe | Sonda ayak izi / tepe | GPU (IOAccelerator) açılış / son | Görünümün tamponları |
|---:|---:|---:|---:|---:|---:|
| 10000 | 315 | 467 / 467 | 606 / 1013 | 45 / 50 | 24 |
| 100000 | 700 | 907 / 971 | 2027 / 2192 | 55 / 53 | 109 |
| 1000000 | 5343 | 6322 / 7181 | 13312 / 14336 | 41 / 59 | 959 |
