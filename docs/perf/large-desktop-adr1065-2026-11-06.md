# Büyük karma 2B çizim: blokların uzak vekili (6 Kasım 2026)

`plan/cad-advanced.md` CAD-ADV-19, [ADR 1065](../adr/1065-far-block-proxy.md). Ölçüm düzeni ve veri seti ADR 1062'ninkidir.
Ham veri: [önce](large-desktop-adr1065-once-2026-11-06.json) (bu dalın vekilsiz ikilisi: `main` `a97b7eea2` + ADR
1064'ün dalı), [sonra](large-desktop-adr1065-sonra-2026-11-06.json) (vekilli ikili, uygulamanın kendi ayarı). İkisi aynı
makinede arka arkaya, aynı sabit izle; tablolar `scripts/perf/large_report.py` ile üretildi, elle sayı girilmedi.

```text
KENTOS_PERF_LABEL=adr1065-once|adr1065-sonra KENTOS_PERF_OUT=docs/perf KENTOS_PERF_SIZES=10000,100000,1000000 \
  cargo test --release -p kentos-desktop --lib perf::large::large -- --ignored --nocapture --test-threads=1
python3 scripts/perf/large_report.py docs/perf/large-desktop-adr1065-sonra-2026-11-06.json \
  docs/perf/large-desktop-adr1065-once-2026-11-06.json
```

- Ölçüm boyunca ortak ağır iş kilidi (`/tmp/kentos-heavy.lock`) tutuldu; makinedeki öbür ajanların kilitsiz işi
  dışlanamaz. “Önce” ikilisi vekil kodundan önce derlendi (ikilide vekilin JSON'u yok, denetlendi); iki JSON'un
  “kaydedilmemiş değişiklik” notu ölçüm anındaki çalışma ağacınındır.
- Tablodaki açılış, ilk etkileşim, komut, iptal ve bellek tabloları yalnız “sonra”nındır; “önce”nin aynı satırları:
  ilk kare MİB / GPU 10⁴ 26,1 / 16,1, 10⁵ 228,4 / 35,6, 10⁶ 2 265,8 / 252,9 ms; görünümün tamponları 24 / 109 / 959 MB
  (sonra aynı); süreç tepe ayak izi 1 013 / 2 174 / 14 336 MB (sonra 1 011 / 2 058 / 14 336 MB); Hepsini seç MİB 14,7 /
  164,1 / 2 018 ms.
- Vekilin tek başına payı aynı ikilinin `KENTOS_PERF_SHORTCUTS=adr1064` koşusuyla ayrıca ölçüldü (`perf::large::layers`,
  ADR 1065 §5).

Ortam: Apple M5, 10 iş parçacığı, 32 GB; macOS 26.6.2 (25G83) (25.6.0); rustc 1.96.0 (ac68faa20 2026-05-25); GPU Apple M5 (Metal, ); commit `f00ac9471` (kaydedilmemiş değişiklikle); release (lto thin); pencere 1440×900; tohum 19.

| Nesne | Dosya | Parmak izi | İş parçacığı (oku, çöz, belge) | İlk kare MİB / GPU | Açılıştan ilk kareye | Açılışta karelerin MİB p95 |
|---:|---:|---|---:|---:|---:|---:|
| 10000 | 2.0 MB | `7f200b185434b8be` | 23,8 | 47,0 / 19,3 | 91,3 | 3,1 |
| 100000 | 20.0 MB | `a0142ee95cc85d46` | 151,2 | 238,5 / 34,7 | 424,6 | 0,4 |
| 1000000 | 199.9 MB | `c48958125ed2220e` | 1664,6 | 2213,8 / 250,3 | 4128,8 | 0,6 |

İlk etkileşim (açılıştan sonraki ilk girdi), MİB / GPU ms:

| Nesne | İmleç | Tekerlek | Kaydırma |
|---:|---:|---:|---:|
| 10000 | 22,2 / 25,2 | 1,1 / 5,8 | 1,3 / 10,1 |
| 100000 | 25,8 / 24,6 | 0,9 / 24,9 | 1,4 / 14,2 |
| 1000000 | 19,0 / 191,7 | 1,7 / 181,4 | 1,5 / 125,2 |

Kareler (MİB: arayüz iş parçacığı; GPU: çizim ve geri okuma). Bütçe 16,7 ms (60 Hz).

| Nesne | Koşu | Görünüm | İz | Kare | MİB p95 önce → sonra | MİB p99 sonra | Bütçeyi aşan önce → sonra | GPU p50 önce → sonra | GPU p95 sonra |
|---|---|---|---|---|---|---|---|---|---|
| 10000 | ilk | uzak | imleç | 120 | 0,5 → 0,6 | 0,8 | 0 → 0 | 2,6 → 7,3 | 8,0 |
| 10000 | ilk | uzak | tekerlek | 32 | 1,3 → 1,4 | 1,8 | 0 → 0 | 7,0 → 7,8 | 10,6 |
| 10000 | ilk | uzak | kaydırma | 40 | 1,1 → 1,2 | 1,6 | 0 → 0 | 7,5 → 7,5 | 9,8 |
| 10000 | ilk | yakın 1:200 | imleç | 60 | 0,6 → 0,6 | 0,9 | 0 → 0 | 7,4 → 7,2 | 8,7 |
| 10000 | ilk | yakın 1:200 | tekerlek | 32 | 2,7 → 2,7 | 3,3 | 0 → 0 | 7,6 → 7,3 | 9,0 |
| 10000 | ilk | yakın 1:200 | kaydırma | 40 | 0,8 → 0,9 | 1,0 | 0 → 0 | 7,5 → 7,3 | 8,7 |
| 10000 | ısınmış | uzak | imleç | 120 | 0,5 → 0,6 | 0,7 | 0 → 0 | 4,0 → 7,3 | 9,0 |
| 10000 | ısınmış | uzak | tekerlek | 32 | 0,8 → 1,4 | 1,8 | 0 → 0 | 7,3 → 7,5 | 12,9 |
| 10000 | ısınmış | uzak | kaydırma | 40 | 1,2 → 1,4 | 1,5 | 0 → 0 | 8,4 → 7,6 | 9,8 |
| 10000 | ısınmış | yakın 1:200 | imleç | 60 | 0,8 → 0,6 | 0,7 | 0 → 0 | 7,3 → 4,9 | 7,3 |
| 10000 | ısınmış | yakın 1:200 | tekerlek | 32 | 0,8 → 0,8 | 1,5 | 0 → 0 | 7,4 → 7,4 | 9,9 |
| 10000 | ısınmış | yakın 1:200 | kaydırma | 40 | 0,7 → 1,1 | 1,3 | 0 → 0 | 7,7 → 7,4 | 8,2 |
| 100000 | ilk | uzak | imleç | 120 | 0,4 → 0,4 | 0,4 | 0 → 0 | 2,7 → 3,9 | 4,5 |
| 100000 | ilk | uzak | tekerlek | 32 | 1,1 → 1,3 | 1,3 | 0 → 0 | 20,5 → 17,6 | 24,2 |
| 100000 | ilk | uzak | kaydırma | 40 | 1,1 → 1,0 | 1,1 | 0 → 0 | 25,2 → 12,1 | 12,3 |
| 100000 | ilk | yakın 1:200 | imleç | 60 | 0,6 → 0,6 | 0,7 | 0 → 0 | 3,0 → 2,8 | 5,2 |
| 100000 | ilk | yakın 1:200 | tekerlek | 32 | 3,6 → 3,5 | 4,4 | 0 → 0 | 4,7 → 5,6 | 8,7 |
| 100000 | ilk | yakın 1:200 | kaydırma | 40 | 0,7 → 0,7 | 1,0 | 0 → 0 | 7,5 → 7,6 | 10,0 |
| 100000 | ısınmış | uzak | imleç | 120 | 0,4 → 0,5 | 0,5 | 0 → 0 | 2,7 → 2,7 | 4,3 |
| 100000 | ısınmış | uzak | tekerlek | 32 | 1,1 → 1,2 | 1,3 | 0 → 0 | 20,6 → 16,3 | 23,9 |
| 100000 | ısınmış | uzak | kaydırma | 40 | 1,0 → 1,0 | 1,0 | 0 → 0 | 25,1 → 12,1 | 13,5 |
| 100000 | ısınmış | yakın 1:200 | imleç | 60 | 0,6 → 0,6 | 0,8 | 0 → 0 | 2,9 → 3,7 | 5,0 |
| 100000 | ısınmış | yakın 1:200 | tekerlek | 32 | 0,7 → 1,1 | 1,5 | 0 → 0 | 5,5 → 5,8 | 10,8 |
| 100000 | ısınmış | yakın 1:200 | kaydırma | 40 | 0,7 → 0,8 | 0,9 | 0 → 0 | 7,0 → 7,6 | 8,1 |
| 1000000 | ilk | uzak | imleç | 120 | 0,9 → 0,9 | 1,0 | 0 → 0 | 6,5 → 5,6 | 6,8 |
| 1000000 | ilk | uzak | tekerlek | 32 | 49,6 → 1,9 | 23,5 | 12 → 1 | 168,0 → 71,1 | 121,7 |
| 1000000 | ilk | uzak | kaydırma | 40 | 12,6 → 2,0 | 2,0 | 0 → 0 | 284,4 → 119,4 | 127,1 |
| 1000000 | ilk | yakın 1:200 | imleç | 60 | 0,6 → 0,6 | 0,6 | 0 → 0 | 4,9 → 5,0 | 6,3 |
| 1000000 | ilk | yakın 1:200 | tekerlek | 32 | 3,4 → 3,2 | 25,7 | 1 → 1 | 11,4 → 8,6 | 11,6 |
| 1000000 | ilk | yakın 1:200 | kaydırma | 40 | 1,1 → 1,0 | 1,3 | 0 → 0 | 8,6 → 11,3 | 13,4 |
| 1000000 | ısınmış | uzak | imleç | 120 | 1,0 → 0,8 | 1,0 | 0 → 0 | 5,6 → 5,5 | 6,7 |
| 1000000 | ısınmış | uzak | tekerlek | 32 | 28,8 → 2,0 | 26,0 | 11 → 1 | 166,7 → 70,8 | 120,5 |
| 1000000 | ısınmış | uzak | kaydırma | 40 | 9,5 → 1,9 | 1,9 | 0 → 0 | 287,2 → 120,6 | 127,9 |
| 1000000 | ısınmış | yakın 1:200 | imleç | 60 | 0,7 → 0,6 | 0,6 | 0 → 0 | 4,9 → 4,9 | 6,2 |
| 1000000 | ısınmış | yakın 1:200 | tekerlek | 32 | 2,5 → 1,2 | 21,9 | 1 → 1 | 8,9 → 8,1 | 11,8 |
| 1000000 | ısınmış | yakın 1:200 | kaydırma | 40 | 1,0 → 0,9 | 1,2 | 0 → 0 | 11,1 → 11,5 | 13,1 |

Geri okuma tabanı (boş pencerenin karesi, aynı yolla), GPU p50: 10000: 2,3 ms, 100000: 2,3 ms, 1000000: 2,2 ms.

Yazı girişi (bir tuşun karesi; yerel yankı = MİB + GPU), ms:

| Nesne | Komut satırı MİB p95 / GPU p95 | Değer alanı MİB p95 / GPU p95 | En kötü yankı (MİB en çok + GPU en çok) |
|---:|---:|---:|---:|
| 10000 | 1,5 / 9,8 | 0,5 / 9,1 | 14,0 |
| 100000 | 1,0 / 9,1 | 0,5 / 8,4 | 13,0 |
| 1000000 | 1,0 / 10,7 | 0,8 / 11,4 | 12,4 |

Komutlar (bir kare; parçalar: yeniden kurulan stilli katman parçası / bütün parçalar), ms:

| Nesne | Komut | MİB | GPU | Kurulan parça | Kurma |
|---:|---|---:|---:|---:|---:|
| 10000 | İlk komut (bir yazıyı taşı) | 293,1 | 13,3 | 1 / 6 | 0,5 |
| 10000 | Bir yazıyı taşı | 1,1 | 4,0 | 1 / 6 | 0,3 |
| 10000 | Bir yazının metnini değiştir | 4,0 | 3,9 | 1 / 6 | 0,2 |
| 10000 | Bir blok yerleşimini taşı | 8,4 | 4,2 | 1 / 6 | 7,8 |
| 10000 | Bir çizgiyi sil | 7,4 | 4,5 | 1 / 6 | 5,9 |
| 10000 | Geri al | 6,6 | 4,4 | 1 / 6 | 5,9 |
| 10000 | Yinele | 6,3 | 4,4 | 1 / 6 | 5,8 |
| 10000 | Hepsini seç (Ctrl+A) | 16,9 | 8,0 | 0 / 6 | – |
| 10000 | Seçimi bırak (Esc) | 0,5 | 5,6 | – | – |
| 100000 | İlk komut (bir yazıyı taşı) | 243,3 | 7,4 | 1 / 102 | 0,2 |
| 100000 | Bir yazıyı taşı | 0,8 | 8,8 | 1 / 102 | 0,2 |
| 100000 | Bir yazının metnini değiştir | 0,7 | 4,2 | 1 / 102 | 0,2 |
| 100000 | Bir blok yerleşimini taşı | 3,9 | 5,8 | 1 / 102 | 3,3 |
| 100000 | Bir çizgiyi sil | 3,4 | 4,4 | 1 / 102 | 2,8 |
| 100000 | Geri al | 3,1 | 5,9 | 1 / 102 | 2,4 |
| 100000 | Yinele | 3,0 | 5,9 | 1 / 102 | 2,4 |
| 100000 | Hepsini seç (Ctrl+A) | 175,3 | 30,5 | 0 / 102 | – |
| 100000 | Seçimi bırak (Esc) | 0,5 | 4,4 | – | – |
| 1000000 | İlk komut (bir yazıyı taşı) | 271,5 | 18,9 | 1 / 1470 | 22,4 |
| 1000000 | Bir yazıyı taşı | 22,3 | 9,9 | 1 / 1470 | 21,4 |
| 1000000 | Bir yazının metnini değiştir | 22,2 | 8,2 | 1 / 1470 | 21,1 |
| 1000000 | Bir blok yerleşimini taşı | 25,3 | 8,4 | 1 / 1470 | 24,5 |
| 1000000 | Bir çizgiyi sil | 25,7 | 9,5 | 1 / 1470 | 24,8 |
| 1000000 | Geri al | 25,1 | 9,9 | 1 / 1470 | 24,3 |
| 1000000 | Yinele | 25,1 | 9,9 | 1 / 1470 | 24,3 |
| 1000000 | Hepsini seç (Ctrl+A) | 2093,9 | 215,0 | 0 / 1470 | – |
| 1000000 | Seçimi bırak (Esc) | 1,3 | 8,9 | – | – |

İptal (ikinci açılış, okurken Esc), ms:

| Nesne | Esc'nin karesi MİB + GPU | Pencere o karede kapandı | İş parçacığının durması (Esc'den sonra) | Hiçbir şey uygulanmadı |
|---:|---:|---|---:|---|
| 10000 | 4,0 | evet | 1,7 | evet |
| 100000 | 3,2 | evet | 1,4 | evet |
| 1000000 | 5,5 | evet | 2,0 | evet |

Bellek, MB (`footprint`: fiziksel ayak izi ve sürecin tepesi; GPU: IOAccelerator; görünümün GPU tamponları `FrameStats`):

| Nesne | Açılışta yerleşik | Açılışta ayak izi / tepe | Sonda ayak izi / tepe | GPU (IOAccelerator) açılış / son | Görünümün tamponları |
|---:|---:|---:|---:|---:|---:|
| 10000 | 312 | 465 / 465 | 607 / 1011 | 45 / 50 | 24 |
| 100000 | 676 | 876 / 965 | 1914 / 2058 | 32 / 53 | 109 |
| 1000000 | 5324 | 6308 / 7168 | 13312 / 14336 | 41 / 58 | 959 |
