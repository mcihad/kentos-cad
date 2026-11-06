# Büyük karma 2B çizim: tekerlek dönerken ölçeklenen resim (6 Kasım 2026)

`plan/cad-advanced.md` CAD-ADV-19, [ADR 1068](../adr/1068-zoomed-picture-while-the-wheel-turns.md). Ölçüm düzeni ve veri seti ADR
1062'ninkidir. Ham veri: [önce](large-desktop-adr1067-sonra-2026-11-06.json) (ADR 1067'nin “sonra”sı, `483a4cc09` dalının
ikilisi), [sonra](large-desktop-adr1068-sonra-2026-11-06.json) (bu dalın ikilisi, `a2f6c8646` üstünde), aynı ikiliyle iki
tekrar: [1](large-desktop-adr1068-tekrar1-2026-11-06.json), [2](large-desktop-adr1068-tekrar2-2026-11-06.json). Tablolar
`scripts/perf/large_report.py` ile üretildi, elle sayı girilmedi.

```text
KENTOS_PERF_LABEL=adr1068-sonra KENTOS_PERF_OUT=docs/perf KENTOS_PERF_SIZES=10000,100000,1000000 \
  cargo test --release -p kentos-desktop --lib perf::large::large -- --ignored --nocapture --test-threads=1
python3 scripts/perf/large_report.py docs/perf/large-desktop-adr1068-sonra-2026-11-06.json \
  docs/perf/large-desktop-adr1067-sonra-2026-11-06.json
```

- “Önce” bu dalın tabanından değil ADR 1067'nin aynı gün aynı makinede alınmış ölçümüdür; arada `main`'e giren işler
  (ADR 1027'nin büyük seçim sınırları dahil) de farkın içindedir. Hepsini seç'in 1 131 → 271 ms'si ADR 1027'nindir
  (25 000'den büyük seçimde Öznitelikler yalnız sayıyı, 20 000'den büyükte vurgu kutuyu gösterir); tekerleğin farkı bu
  ADR'nindir (ölçekleme kapalı kontrol: `perf::stability`, ADR 1068 §4).
- Ölçüm boyunca ortak ağır iş kilidi (`/tmp/kentos-heavy.lock`) tutuldu; makinedeki öbür işler dışlanamaz. JSON'ların
  “kaydedilmemiş değişiklik” notu ölçüm anındaki çalışma ağacınındır (bu ADR'nin değişikliği).
- İzin tekerleği orta noktada 8 çentik içeri, 8 dışarı, iki kez (her çentik e^0,15 ≈ 1,16 kat); tekerlek durunca 200 ms
  beklenir, duruşun bütün çizilen karesi tekerlek karelerinde sayılmaz.

Ortam: Apple M5, 10 iş parçacığı, 32 GB; macOS 26.6.2 (25G83) (25.6.0); rustc 1.96.0 (ac68faa20 2026-05-25); GPU Apple M5 (Metal, ); commit `a2f6c8646` (kaydedilmemiş değişiklikle); release (lto thin); pencere 1440×900; tohum 19.

| Nesne | Dosya | Parmak izi | İş parçacığı (oku, çöz, belge) | İlk kare MİB / GPU | Açılıştan ilk kareye | Açılışta karelerin MİB p95 |
|---:|---:|---|---:|---:|---:|---:|
| 10000 | 2.0 MB | `7f200b185434b8be` | 15,2 | 19,3 / 14,5 | 49,2 | 0,4 |
| 100000 | 20.0 MB | `a0142ee95cc85d46` | 151,8 | 163,5 / 29,1 | 344,5 | 0,4 |
| 1000000 | 199.9 MB | `c48958125ed2220e` | 1572,2 | 1440,2 / 243,4 | 3256,0 | 0,4 |

İlk etkileşim (açılıştan sonraki ilk girdi), MİB / GPU ms:

| Nesne | İmleç | Tekerlek | Kaydırma |
|---:|---:|---:|---:|
| 10000 | 18,9 / 19,1 | 0,7 / 4,3 | 1,7 / 5,5 |
| 100000 | 25,2 / 24,0 | 0,8 / 4,4 | 1,1 / 3,2 |
| 1000000 | 18,4 / 177,0 | 2,0 / 18,3 | 2,8 / 19,0 |

Kareler (MİB: arayüz iş parçacığı; GPU: çizim ve geri okuma). Bütçe 16,7 ms (60 Hz).

| Nesne | Koşu | Görünüm | İz | Kare | MİB p95 önce → sonra | MİB p99 sonra | Bütçeyi aşan önce → sonra | GPU p50 önce → sonra | GPU p95 sonra |
|---|---|---|---|---|---|---|---|---|---|
| 10000 | ilk | uzak | imleç | 120 | 0,5 → 0,6 | 0,8 | 0 → 0 | 7,2 → 4,1 | 7,9 |
| 10000 | ilk | uzak | tekerlek | 32 | 1,5 → 1,3 | 1,3 | 0 → 0 | 7,5 → 4,4 | 7,0 |
| 10000 | ilk | uzak | kaydırma | 40 | 1,1 → 0,8 | 1,8 | 0 → 0 | 4,0 → 4,1 | 8,5 |
| 10000 | ilk | yakın 1:200 | imleç | 60 | 0,6 → 0,5 | 0,8 | 0 → 0 | 6,6 → 4,6 | 7,5 |
| 10000 | ilk | yakın 1:200 | tekerlek | 32 | 2,6 → 2,6 | 3,2 | 0 → 0 | 7,2 → 5,7 | 8,8 |
| 10000 | ilk | yakın 1:200 | kaydırma | 40 | 0,9 → 0,8 | 1,3 | 0 → 0 | 6,0 → 7,4 | 9,1 |
| 10000 | ısınmış | uzak | imleç | 120 | 0,8 → 0,5 | 0,8 | 0 → 0 | 7,4 → 2,5 | 5,9 |
| 10000 | ısınmış | uzak | tekerlek | 32 | 1,4 → 1,2 | 1,9 | 0 → 0 | 7,4 → 3,5 | 5,6 |
| 10000 | ısınmış | uzak | kaydırma | 40 | 0,9 → 0,7 | 1,1 | 0 → 0 | 3,0 → 2,7 | 4,4 |
| 10000 | ısınmış | yakın 1:200 | imleç | 60 | 0,6 → 0,5 | 0,6 | 0 → 0 | 7,3 → 4,3 | 6,0 |
| 10000 | ısınmış | yakın 1:200 | tekerlek | 32 | 1,0 → 1,0 | 1,3 | 0 → 0 | 7,1 → 5,3 | 7,2 |
| 10000 | ısınmış | yakın 1:200 | kaydırma | 40 | 0,9 → 0,7 | 1,2 | 0 → 0 | 6,3 → 7,5 | 11,1 |
| 100000 | ilk | uzak | imleç | 120 | 0,4 → 0,4 | 0,5 | 0 → 0 | 2,8 → 2,7 | 4,6 |
| 100000 | ilk | uzak | tekerlek | 32 | 1,2 → 1,1 | 1,2 | 0 → 0 | 16,8 → 4,4 | 6,0 |
| 100000 | ilk | uzak | kaydırma | 40 | 0,9 → 1,0 | 1,1 | 0 → 0 | 2,9 → 4,5 | 6,2 |
| 100000 | ilk | yakın 1:200 | imleç | 60 | 0,5 → 0,5 | 0,7 | 0 → 0 | 4,2 → 3,0 | 4,5 |
| 100000 | ilk | yakın 1:200 | tekerlek | 32 | 3,8 → 3,4 | 4,5 | 0 → 0 | 7,6 → 5,5 | 7,6 |
| 100000 | ilk | yakın 1:200 | kaydırma | 40 | 0,8 → 0,8 | 1,2 | 0 → 0 | 7,5 → 6,7 | 9,3 |
| 100000 | ısınmış | uzak | imleç | 120 | 0,5 → 0,4 | 0,5 | 0 → 0 | 4,1 → 2,7 | 4,2 |
| 100000 | ısınmış | uzak | tekerlek | 32 | 1,2 → 0,9 | 1,1 | 0 → 0 | 15,3 → 2,8 | 4,5 |
| 100000 | ısınmış | uzak | kaydırma | 40 | 0,9 → 0,9 | 1,0 | 0 → 0 | 2,8 → 2,9 | 4,6 |
| 100000 | ısınmış | yakın 1:200 | imleç | 60 | 0,6 → 0,6 | 0,6 | 0 → 0 | 3,5 → 3,1 | 5,5 |
| 100000 | ısınmış | yakın 1:200 | tekerlek | 32 | 1,2 → 0,8 | 1,3 | 0 → 0 | 7,6 → 5,2 | 9,6 |
| 100000 | ısınmış | yakın 1:200 | kaydırma | 40 | 0,7 → 0,9 | 1,0 | 0 → 0 | 7,2 → 6,7 | 9,3 |
| 1000000 | ilk | uzak | imleç | 120 | 0,8 → 0,9 | 1,0 | 0 → 0 | 7,3 → 6,9 | 8,0 |
| 1000000 | ilk | uzak | tekerlek | 32 | 1,9 → 1,7 | 2,0 | 1 → 0 | 67,4 → 8,1 | 12,7 |
| 1000000 | ilk | uzak | kaydırma | 40 | 1,6 → 1,5 | 1,5 | 0 → 0 | 8,8 → 8,7 | 11,8 |
| 1000000 | ilk | yakın 1:200 | imleç | 60 | 0,6 → 0,5 | 0,7 | 0 → 0 | 6,7 → 7,4 | 7,9 |
| 1000000 | ilk | yakın 1:200 | tekerlek | 32 | 3,3 → 3,0 | 4,0 | 1 → 0 | 9,3 → 10,1 | 12,9 |
| 1000000 | ilk | yakın 1:200 | kaydırma | 40 | 0,9 → 0,8 | 0,9 | 0 → 0 | 11,2 → 12,2 | 14,9 |
| 1000000 | ısınmış | uzak | imleç | 120 | 0,8 → 0,8 | 0,9 | 0 → 0 | 7,3 → 6,9 | 9,6 |
| 1000000 | ısınmış | uzak | tekerlek | 32 | 1,9 → 1,7 | 1,8 | 1 → 0 | 65,0 → 7,0 | 12,8 |
| 1000000 | ısınmış | uzak | kaydırma | 40 | 1,6 → 1,5 | 1,6 | 0 → 0 | 8,9 → 8,3 | 12,2 |
| 1000000 | ısınmış | yakın 1:200 | imleç | 60 | 0,6 → 0,6 | 0,7 | 0 → 0 | 5,8 → 7,4 | 7,7 |
| 1000000 | ısınmış | yakın 1:200 | tekerlek | 32 | 1,0 → 0,8 | 1,0 | 1 → 0 | 9,2 → 8,4 | 14,9 |
| 1000000 | ısınmış | yakın 1:200 | kaydırma | 40 | 0,7 → 0,8 | 0,9 | 0 → 0 | 9,3 → 13,6 | 17,1 |

Geri okuma tabanı (boş pencerenin karesi, aynı yolla), GPU p50: 10000: 2,3 ms, 100000: 2,2 ms, 1000000: 2,2 ms.

Yazı girişi (bir tuşun karesi; yerel yankı = MİB + GPU), ms:

| Nesne | Komut satırı MİB p95 / GPU p95 | Değer alanı MİB p95 / GPU p95 | En kötü yankı (MİB en çok + GPU en çok) |
|---:|---:|---:|---:|
| 10000 | 1,1 / 9,9 | 0,6 / 8,1 | 14,0 |
| 100000 | 1,2 / 10,5 | 0,4 / 8,0 | 14,6 |
| 1000000 | 0,8 / 14,5 | 0,4 / 13,7 | 16,8 |

Komutlar (bir kare; parçalar: yeniden kurulan stilli katman parçası / bütün parçalar), ms:

| Nesne | Komut | MİB | GPU | Kurulan parça | Kurma |
|---:|---|---:|---:|---:|---:|
| 10000 | İlk komut (bir yazıyı taşı) | 241,2 | 5,8 | 1 / 6 | 0,3 |
| 10000 | Bir yazıyı taşı | 0,7 | 7,0 | 1 / 6 | 0,2 |
| 10000 | Bir yazının metnini değiştir | 1,3 | 3,7 | 1 / 6 | 0,2 |
| 10000 | Bir blok yerleşimini taşı | 8,8 | 4,3 | 1 / 6 | 8,3 |
| 10000 | Bir çizgiyi sil | 6,5 | 4,5 | 1 / 6 | 5,7 |
| 10000 | Geri al | 6,1 | 6,1 | 1 / 6 | 5,6 |
| 10000 | Yinele | 6,2 | 6,0 | 1 / 6 | 5,6 |
| 10000 | Hepsini seç (Ctrl+A) | 8,9 | 6,4 | 0 / 6 | – |
| 10000 | Seçimi bırak (Esc) | 0,5 | 5,5 | – | – |
| 100000 | İlk komut (bir yazıyı taşı) | 237,9 | 13,3 | 1 / 102 | 0,3 |
| 100000 | Bir yazıyı taşı | 0,8 | 5,6 | 1 / 102 | 0,2 |
| 100000 | Bir yazının metnini değiştir | 0,9 | 4,3 | 1 / 102 | 0,2 |
| 100000 | Bir blok yerleşimini taşı | 4,1 | 4,3 | 1 / 102 | 3,5 |
| 100000 | Bir çizgiyi sil | 3,1 | 6,1 | 1 / 102 | 2,4 |
| 100000 | Geri al | 3,0 | 9,0 | 1 / 102 | 2,4 |
| 100000 | Yinele | 2,8 | 4,5 | 1 / 102 | 2,3 |
| 100000 | Hepsini seç (Ctrl+A) | 17,7 | 9,1 | 0 / 102 | – |
| 100000 | Seçimi bırak (Esc) | 0,4 | 5,9 | – | – |
| 1000000 | İlk komut (bir yazıyı taşı) | 255,1 | 12,9 | 1 / 1470 | 22,2 |
| 1000000 | Bir yazıyı taşı | 21,0 | 15,7 | 1 / 1470 | 20,2 |
| 1000000 | Bir yazının metnini değiştir | 21,5 | 16,3 | 1 / 1470 | 20,4 |
| 1000000 | Bir blok yerleşimini taşı | 25,4 | 13,4 | 1 / 1470 | 24,5 |
| 1000000 | Bir çizgiyi sil | 24,7 | 10,9 | 1 / 1470 | 23,9 |
| 1000000 | Geri al | 24,1 | 13,7 | 1 / 1470 | 23,3 |
| 1000000 | Yinele | 23,6 | 10,3 | 1 / 1470 | 22,8 |
| 1000000 | Hepsini seç (Ctrl+A) | 271,3 | 18,9 | 0 / 1470 | – |
| 1000000 | Seçimi bırak (Esc) | 0,6 | 9,0 | – | – |

İptal (ikinci açılış, okurken Esc), ms:

| Nesne | Esc'nin karesi MİB + GPU | Pencere o karede kapandı | İş parçacığının durması (Esc'den sonra) | Hiçbir şey uygulanmadı |
|---:|---:|---|---:|---|
| 10000 | 3,0 | evet | 1,3 | evet |
| 100000 | 7,7 | evet | 1,5 | evet |
| 1000000 | 6,8 | evet | 2,3 | evet |

Bellek, MB (`footprint`: fiziksel ayak izi ve sürecin tepesi; GPU: IOAccelerator; görünümün GPU tamponları `FrameStats`):

| Nesne | Açılışta yerleşik | Açılışta ayak izi / tepe | Sonda ayak izi / tepe | GPU (IOAccelerator) açılış / son | Görünümün tamponları |
|---:|---:|---:|---:|---:|---:|
| 10000 | 315 | 470 / 470 | 586 / 1021 | 46 / 45 | 30 |
| 100000 | 720 | 933 / 993 | 1231 / 1371 | 55 / 53 | 115 |
| 1000000 | 5356 | 6344 / 7204 | 7802 / 8173 | 41 / 61 | 965 |
