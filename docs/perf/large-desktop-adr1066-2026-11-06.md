# Büyük karma 2B çizim: bir milyon nesnede tarama, Hepsini seç ve ilk kare (6 Kasım 2026)

`plan/cad-advanced.md` CAD-ADV-19, [ADR 1066](../adr/1066-million-far-hatches-select-all-and-first-frame.md). Ölçüm düzeni
ve veri seti ADR 1062'ninkidir. Ham veri: [önce](large-desktop-adr1066-once-2026-11-06.json) (`origin/main` `8563e0e2e`
ikilisi), [sonra](large-desktop-adr1066-sonra-2026-11-06.json) (bu dalın ikilisi). İkisi aynı makinede arka arkaya, aynı
sabit izle; ikisi de sembol tablosu korunarak derlendi (`--config 'profile.release.package.kentos-desktop.strip="none"'`,
`sample` için; kod aynıdır). Tablolar `scripts/perf/large_report.py` ile üretildi, elle sayı girilmedi.

```text
KENTOS_PERF_LABEL=adr1066-once|adr1066-sonra KENTOS_PERF_OUT=docs/perf KENTOS_PERF_SIZES=10000,100000,1000000 \
  cargo test --release -p kentos-desktop --lib perf::large::large -- --ignored --nocapture --test-threads=1
python3 scripts/perf/large_report.py docs/perf/large-desktop-adr1066-sonra-2026-11-06.json \
  docs/perf/large-desktop-adr1066-once-2026-11-06.json
```

- Ölçüm boyunca ortak ağır iş kilidi (`/tmp/kentos-heavy.lock`) tutuldu; makinedeki öbür ajanların kilitsiz işi
  dışlanamaz. İki JSON'un “kaydedilmemiş değişiklik” notu ölçüm anındaki çalışma ağacınındır.
- Açılış, ilk etkileşim, komut, iptal ve bellek tabloları yalnız “sonra”nındır; “önce”nin aynı satırları: ilk kare MİB /
  GPU 10⁴ 28,3 / 14,8, 10⁵ 238,0 / 39,2, 10⁶ 2 255,2 / 280,1 ms; Hepsini seç MİB / GPU 14,5 / 6,7, 171,5 / 26,6,
  2 156,0 / 322,3 ms; süreç tepe ayak izi 1 021 / 2 131 / 15 360 MB.
- Parçalara ayırma (`perf::million` + macOS `sample`) ve katman katman 10⁶ (`perf::large::layers`) ADR 1066 §1'de.

Ortam: Apple M5, 10 iş parçacığı, 32 GB; macOS 26.6.2 (25G83) (25.6.0); rustc 1.96.0 (ac68faa20 2026-05-25); GPU Apple M5 (Metal, ); commit `8563e0e2e` (kaydedilmemiş değişiklikle); release (lto thin); pencere 1440×900; tohum 19.

| Nesne | Dosya | Parmak izi | İş parçacığı (oku, çöz, belge) | İlk kare MİB / GPU | Açılıştan ilk kareye | Açılışta karelerin MİB p95 |
|---:|---:|---|---:|---:|---:|---:|
| 10000 | 2.0 MB | `7f200b185434b8be` | 24,7 | 44,6 / 17,6 | 87,8 | 2,9 |
| 100000 | 20.0 MB | `a0142ee95cc85d46` | 157,7 | 174,4 / 41,5 | 373,7 | 0,4 |
| 1000000 | 199.9 MB | `c48958125ed2220e` | 1640,4 | 1477,8 / 254,2 | 3372,6 | 0,4 |

İlk etkileşim (açılıştan sonraki ilk girdi), MİB / GPU ms:

| Nesne | İmleç | Tekerlek | Kaydırma |
|---:|---:|---:|---:|
| 10000 | 23,5 / 19,0 | 0,7 / 11,8 | 0,8 / 6,0 |
| 100000 | 25,6 / 24,6 | 0,9 / 22,7 | 0,9 / 9,2 |
| 1000000 | 18,5 / 180,9 | 1,6 / 171,1 | 1,4 / 113,4 |

Kareler (MİB: arayüz iş parçacığı; GPU: çizim ve geri okuma). Bütçe 16,7 ms (60 Hz).

| Nesne | Koşu | Görünüm | İz | Kare | MİB p95 önce → sonra | MİB p99 sonra | Bütçeyi aşan önce → sonra | GPU p50 önce → sonra | GPU p95 sonra |
|---|---|---|---|---|---|---|---|---|---|
| 10000 | ilk | uzak | imleç | 120 | 0,5 → 0,4 | 0,7 | 0 → 0 | 5,6 → 4,1 | 7,5 |
| 10000 | ilk | uzak | tekerlek | 32 | 1,2 → 1,6 | 2,0 | 0 → 0 | 8,8 → 7,9 | 11,3 |
| 10000 | ilk | uzak | kaydırma | 40 | 1,2 → 1,3 | 1,4 | 0 → 0 | 7,5 → 7,4 | 12,8 |
| 10000 | ilk | yakın 1:200 | imleç | 60 | 0,8 → 0,6 | 0,6 | 0 → 0 | 6,4 → 7,4 | 9,0 |
| 10000 | ilk | yakın 1:200 | tekerlek | 32 | 2,7 → 2,5 | 3,2 | 0 → 0 | 7,3 → 7,0 | 9,0 |
| 10000 | ilk | yakın 1:200 | kaydırma | 40 | 0,8 → 1,1 | 1,3 | 0 → 0 | 7,2 → 7,0 | 8,2 |
| 10000 | ısınmış | uzak | imleç | 120 | 0,6 → 0,4 | 0,6 | 0 → 0 | 7,3 → 2,5 | 4,0 |
| 10000 | ısınmış | uzak | tekerlek | 32 | 1,2 → 1,2 | 1,9 | 0 → 0 | 7,1 → 7,3 | 10,9 |
| 10000 | ısınmış | uzak | kaydırma | 40 | 1,3 → 0,9 | 1,0 | 0 → 0 | 7,4 → 7,3 | 11,1 |
| 10000 | ısınmış | yakın 1:200 | imleç | 60 | 0,6 → 0,5 | 0,8 | 0 → 0 | 6,7 → 4,3 | 8,7 |
| 10000 | ısınmış | yakın 1:200 | tekerlek | 32 | 0,7 → 0,8 | 1,1 | 0 → 0 | 5,9 → 6,4 | 8,7 |
| 10000 | ısınmış | yakın 1:200 | kaydırma | 40 | 0,6 → 1,0 | 1,1 | 0 → 0 | 7,3 → 7,4 | 8,9 |
| 100000 | ilk | uzak | imleç | 120 | 0,4 → 0,5 | 0,6 | 0 → 0 | 2,7 → 4,2 | 7,9 |
| 100000 | ilk | uzak | tekerlek | 32 | 1,2 → 1,4 | 3,0 | 0 → 0 | 16,9 → 17,2 | 23,5 |
| 100000 | ilk | uzak | kaydırma | 40 | 1,0 → 1,1 | 1,2 | 0 → 0 | 11,9 → 10,7 | 12,3 |
| 100000 | ilk | yakın 1:200 | imleç | 60 | 0,6 → 0,6 | 0,7 | 0 → 0 | 2,9 → 2,8 | 5,4 |
| 100000 | ilk | yakın 1:200 | tekerlek | 32 | 3,5 → 3,3 | 4,5 | 0 → 0 | 5,8 → 5,8 | 8,2 |
| 100000 | ilk | yakın 1:200 | kaydırma | 40 | 0,8 → 0,5 | 0,6 | 0 → 0 | 7,5 → 7,4 | 8,3 |
| 100000 | ısınmış | uzak | imleç | 120 | 0,4 → 0,5 | 0,5 | 0 → 0 | 2,7 → 2,8 | 4,4 |
| 100000 | ısınmış | uzak | tekerlek | 32 | 1,4 → 1,5 | 1,7 | 0 → 0 | 17,2 → 15,6 | 21,2 |
| 100000 | ısınmış | uzak | kaydırma | 40 | 1,0 → 1,2 | 1,2 | 0 → 0 | 12,1 → 10,7 | 10,9 |
| 100000 | ısınmış | yakın 1:200 | imleç | 60 | 0,6 → 0,5 | 0,7 | 0 → 0 | 2,9 → 3,1 | 5,5 |
| 100000 | ısınmış | yakın 1:200 | tekerlek | 32 | 0,6 → 0,6 | 0,9 | 0 → 0 | 6,0 → 5,8 | 7,8 |
| 100000 | ısınmış | yakın 1:200 | kaydırma | 40 | 0,7 → 0,9 | 1,0 | 0 → 0 | 7,4 → 7,6 | 12,1 |
| 1000000 | ilk | uzak | imleç | 120 | 0,9 → 0,8 | 0,9 | 0 → 0 | 5,9 → 5,4 | 6,6 |
| 1000000 | ilk | uzak | tekerlek | 32 | 1,9 → 2,0 | 24,0 | 1 → 1 | 72,3 → 64,1 | 107,9 |
| 1000000 | ilk | uzak | kaydırma | 40 | 1,9 → 1,9 | 2,0 | 0 → 0 | 117,2 → 107,7 | 115,7 |
| 1000000 | ilk | yakın 1:200 | imleç | 60 | 0,5 → 0,5 | 0,5 | 0 → 0 | 4,8 → 4,8 | 6,4 |
| 1000000 | ilk | yakın 1:200 | tekerlek | 32 | 3,2 → 3,0 | 24,7 | 1 → 1 | 8,5 → 7,5 | 9,9 |
| 1000000 | ilk | yakın 1:200 | kaydırma | 40 | 0,8 → 0,8 | 1,0 | 0 → 0 | 10,1 → 11,4 | 13,5 |
| 1000000 | ısınmış | uzak | imleç | 120 | 0,8 → 0,9 | 1,0 | 0 → 0 | 5,5 → 5,8 | 6,7 |
| 1000000 | ısınmış | uzak | tekerlek | 32 | 1,9 → 2,0 | 23,3 | 1 → 1 | 71,0 → 65,0 | 110,4 |
| 1000000 | ısınmış | uzak | kaydırma | 40 | 1,8 → 1,9 | 1,9 | 0 → 0 | 118,5 → 108,1 | 113,3 |
| 1000000 | ısınmış | yakın 1:200 | imleç | 60 | 0,5 → 0,5 | 0,6 | 0 → 0 | 4,8 → 4,6 | 6,1 |
| 1000000 | ısınmış | yakın 1:200 | tekerlek | 32 | 0,9 → 1,0 | 21,3 | 1 → 1 | 7,4 → 11,0 | 14,6 |
| 1000000 | ısınmış | yakın 1:200 | kaydırma | 40 | 0,8 → 0,7 | 1,0 | 0 → 0 | 10,1 → 11,3 | 13,5 |

Geri okuma tabanı (boş pencerenin karesi, aynı yolla), GPU p50: 10000: 3,7 ms, 100000: 2,4 ms, 1000000: 2,2 ms.

Yazı girişi (bir tuşun karesi; yerel yankı = MİB + GPU), ms:

| Nesne | Komut satırı MİB p95 / GPU p95 | Değer alanı MİB p95 / GPU p95 | En kötü yankı (MİB en çok + GPU en çok) |
|---:|---:|---:|---:|
| 10000 | 1,1 / 7,8 | 0,4 / 5,6 | 12,2 |
| 100000 | 0,8 / 10,5 | 0,4 / 8,6 | 22,0 |
| 1000000 | 0,6 / 7,6 | 0,4 / 8,1 | 8,5 |

Komutlar (bir kare; parçalar: yeniden kurulan stilli katman parçası / bütün parçalar), ms:

| Nesne | Komut | MİB | GPU | Kurulan parça | Kurma |
|---:|---|---:|---:|---:|---:|
| 10000 | İlk komut (bir yazıyı taşı) | 288,9 | 5,7 | 1 / 6 | 0,4 |
| 10000 | Bir yazıyı taşı | 1,1 | 8,6 | 1 / 6 | 0,2 |
| 10000 | Bir yazının metnini değiştir | 4,7 | 4,1 | 1 / 6 | 0,2 |
| 10000 | Bir blok yerleşimini taşı | 9,1 | 4,4 | 1 / 6 | 8,4 |
| 10000 | Bir çizgiyi sil | 7,6 | 9,4 | 1 / 6 | 6,0 |
| 10000 | Geri al | 6,7 | 6,2 | 1 / 6 | 6,0 |
| 10000 | Yinele | 6,3 | 4,4 | 1 / 6 | 5,8 |
| 10000 | Hepsini seç (Ctrl+A) | 11,1 | 11,4 | 0 / 6 | – |
| 10000 | Seçimi bırak (Esc) | 0,7 | 6,9 | – | – |
| 100000 | İlk komut (bir yazıyı taşı) | 240,2 | 12,2 | 1 / 102 | 0,2 |
| 100000 | Bir yazıyı taşı | 0,8 | 8,7 | 1 / 102 | 0,2 |
| 100000 | Bir yazının metnini değiştir | 0,8 | 5,8 | 1 / 102 | 0,2 |
| 100000 | Bir blok yerleşimini taşı | 4,3 | 4,4 | 1 / 102 | 3,7 |
| 100000 | Bir çizgiyi sil | 3,3 | 5,9 | 1 / 102 | 2,6 |
| 100000 | Geri al | 3,0 | 4,4 | 1 / 102 | 2,4 |
| 100000 | Yinele | 3,0 | 4,5 | 1 / 102 | 2,5 |
| 100000 | Hepsini seç (Ctrl+A) | 109,2 | 38,5 | 0 / 102 | – |
| 100000 | Seçimi bırak (Esc) | 0,5 | 4,2 | – | – |
| 1000000 | İlk komut (bir yazıyı taşı) | 265,2 | 17,6 | 1 / 1470 | 22,5 |
| 1000000 | Bir yazıyı taşı | 22,0 | 7,2 | 1 / 1470 | 21,0 |
| 1000000 | Bir yazının metnini değiştir | 22,1 | 7,1 | 1 / 1470 | 21,0 |
| 1000000 | Bir blok yerleşimini taşı | 24,8 | 7,3 | 1 / 1470 | 24,0 |
| 1000000 | Bir çizgiyi sil | 24,5 | 8,5 | 1 / 1470 | 23,6 |
| 1000000 | Geri al | 23,5 | 7,6 | 1 / 1470 | 22,7 |
| 1000000 | Yinele | 23,7 | 8,6 | 1 / 1470 | 22,9 |
| 1000000 | Hepsini seç (Ctrl+A) | 1152,0 | 203,0 | 0 / 1470 | – |
| 1000000 | Seçimi bırak (Esc) | 2,8 | 8,8 | – | – |

İptal (ikinci açılış, okurken Esc), ms:

| Nesne | Esc'nin karesi MİB + GPU | Pencere o karede kapandı | İş parçacığının durması (Esc'den sonra) | Hiçbir şey uygulanmadı |
|---:|---:|---|---:|---|
| 10000 | 2,8 | evet | 1,6 | evet |
| 100000 | 3,2 | evet | 1,4 | evet |
| 1000000 | 5,6 | evet | 2,1 | evet |

Bellek, MB (`footprint`: fiziksel ayak izi ve sürecin tepesi; GPU: IOAccelerator; görünümün GPU tamponları `FrameStats`):

| Nesne | Açılışta yerleşik | Açılışta ayak izi / tepe | Sonda ayak izi / tepe | GPU (IOAccelerator) açılış / son | Görünümün tamponları |
|---:|---:|---:|---:|---:|---:|
| 10000 | 313 | 465 / 465 | 584 / 1014 | 45 / 49 | 24 |
| 100000 | 703 | 913 / 1016 | 2041 / 2183 | 55 / 53 | 109 |
| 1000000 | 5359 | 6343 / 7203 | 13312 / 14336 | 41 / 58 | 959 |
