# Büyük karma 2B çizim: blok örnekleme ve eğri düzeyleriyle (6 Kasım 2026)

`plan/cad-advanced.md` CAD-ADV-19, [ADR 1063](../adr/1063-instanced-blocks-and-curve-detail-levels.md). Ölçüm düzeni ve
veri seti ADR 1062'ninkidir. Ham veri: [önce](large-desktop-adr1063-off-2026-11-05.json) (`KENTOS_PERF_SHORTCUTS=off`,
iki kısayol kapalı: eski çizim), [sonra](large-desktop-adr1063-on-2026-11-05.json) (ikisi açık, uygulamanın kendi
ayarı). İkisi aynı ikiliyle arka arkaya; JSON adındaki tarih UTC'dir. Tablolar `scripts/perf/large_report.py` ile
üretildi; elle sayı girilmedi.

```text
KENTOS_PERF_SHORTCUTS=on|off KENTOS_PERF_LABEL=adr1063-on|off KENTOS_PERF_OUT=docs/perf \
  KENTOS_PERF_SIZES=10000,100000,1000000 \
  cargo test --release -p kentos-desktop --lib perf::large::large -- --ignored --nocapture --test-threads=1
python3 scripts/perf/large_report.py docs/perf/large-desktop-adr1063-on-2026-11-05.json \
  docs/perf/large-desktop-adr1063-off-2026-11-05.json
```

- Ölçüm boyunca ortak ağır iş kilidi (`/tmp/kentos-heavy.lock`) tutuldu; makinedeki öbür ajanların kilitsiz işi
  dışlanamaz.
- Commit satırındaki “kaydedilmemiş değişiklik” bu dilimin kodudur (ölçüm commit'ten önce alındı); ölçülen ikiliden
  sonra kodda yalnız davranışı değiştirmeyen düzeltmeler yapıldı (çizginin kutusu ayırmasız büyütülür, bir sabitin adı).
- 10 000 nesnede imleç karelerinin GPU'su 2,5 → 4–7 ms çıktı; resim saklanıyor (tanı izi: anahtar sabit, `holds`
  doğru), fark eğri düzeylerinden (yalnız örneklemeyle 2,5 ms) ve henüz açıklanamadı; 10⁵ ve 10⁶'da yok (ADR 1063 §6).

Ortam: Apple M5, 10 iş parçacığı, 32 GB; macOS 26.6.2 (25G83) (25.6.0); rustc 1.96.0 (ac68faa20 2026-05-25); GPU Apple M5 (Metal, ); commit `ba9a78280` (kaydedilmemiş değişiklikle); release (lto thin); pencere 1440×900; tohum 19.

| Nesne | Dosya | Parmak izi | İş parçacığı (oku, çöz, belge) | İlk kare MİB / GPU | Açılıştan ilk kareye | Açılışta karelerin MİB p95 |
|---:|---:|---|---:|---:|---:|---:|
| 10000 | 2.0 MB | `7f200b185434b8be` | 15,5 | 23,4 / 15,9 | 55,0 | 0,5 |
| 100000 | 20.0 MB | `a0142ee95cc85d46` | 152,4 | 222,7 / 36,6 | 411,8 | 0,4 |
| 1000000 | 199.9 MB | `c48958125ed2220e` | 1656,9 | 2226,6 / 215,5 | 4099,2 | 0,5 |

İlk etkileşim (açılıştan sonraki ilk girdi), MİB / GPU ms:

| Nesne | İmleç | Tekerlek | Kaydırma |
|---:|---:|---:|---:|
| 10000 | 18,3 / 29,4 | 18,7 / 9,7 | 1,6 / 9,7 |
| 100000 | 25,7 / 77,4 | 25,5 / 70,1 | 0,8 / 46,5 |
| 1000000 | 18,1 / 871,4 | 18,0 / 832,4 | 1,1 / 411,0 |

Kareler (MİB: arayüz iş parçacığı; GPU: çizim ve geri okuma). Bütçe 16,7 ms (60 Hz).

| Nesne | Koşu | Görünüm | İz | Kare | MİB p95 önce → sonra | MİB p99 sonra | Bütçeyi aşan önce → sonra | GPU p50 önce → sonra | GPU p95 sonra |
|---|---|---|---|---|---|---|---|---|---|
| 10000 | ilk | uzak | imleç | 120 | 0,5 → 0,5 | 0,6 | 0 → 0 | 2,5 → 4,1 | 7,4 |
| 10000 | ilk | uzak | tekerlek | 32 | 27,7 → 25,9 | 31,1 | 8 → 7 | 18,9 → 7,8 | 17,4 |
| 10000 | ilk | uzak | kaydırma | 40 | 1,4 → 0,9 | 1,4 | 0 → 0 | 23,7 → 7,2 | 9,4 |
| 10000 | ilk | yakın 1:200 | imleç | 60 | 0,6 → 0,7 | 0,8 | 0 → 0 | 2,6 → 7,3 | 8,6 |
| 10000 | ilk | yakın 1:200 | tekerlek | 32 | 2,3 → 2,5 | 3,2 | 0 → 0 | 5,6 → 7,4 | 9,8 |
| 10000 | ilk | yakın 1:200 | kaydırma | 40 | 0,8 → 0,9 | 1,0 | 0 → 0 | 7,6 → 7,7 | 8,0 |
| 10000 | ısınmış | uzak | imleç | 120 | 0,5 → 0,5 | 0,7 | 0 → 0 | 2,5 → 4,3 | 7,5 |
| 10000 | ısınmış | uzak | tekerlek | 32 | 1,3 → 1,1 | 1,9 | 0 → 0 | 17,1 → 11,1 | 15,5 |
| 10000 | ısınmış | uzak | kaydırma | 40 | 1,4 → 1,0 | 1,5 | 0 → 0 | 23,2 → 7,6 | 11,6 |
| 10000 | ısınmış | yakın 1:200 | imleç | 60 | 0,6 → 0,6 | 0,9 | 0 → 0 | 2,6 → 7,3 | 8,8 |
| 10000 | ısınmış | yakın 1:200 | tekerlek | 32 | 0,7 → 1,0 | 1,6 | 0 → 0 | 5,7 → 7,4 | 10,0 |
| 10000 | ısınmış | yakın 1:200 | kaydırma | 40 | 0,8 → 0,8 | 1,1 | 0 → 0 | 7,5 → 7,5 | 9,6 |
| 100000 | ilk | uzak | imleç | 120 | 0,4 → 0,4 | 0,5 | 0 → 0 | 2,6 → 2,7 | 4,3 |
| 100000 | ilk | uzak | tekerlek | 32 | 52,9 → 36,3 | 43,7 | 25 → 7 | 133,2 → 29,0 | 47,4 |
| 100000 | ilk | uzak | kaydırma | 40 | 13,7 → 0,9 | 1,1 | 1 → 0 | 217,6 → 44,8 | 49,0 |
| 100000 | ilk | yakın 1:200 | imleç | 60 | 0,5 → 0,7 | 0,7 | 0 → 0 | 2,9 → 3,0 | 5,1 |
| 100000 | ilk | yakın 1:200 | tekerlek | 32 | 3,4 → 4,0 | 4,7 | 0 → 0 | 6,0 → 4,8 | 6,6 |
| 100000 | ilk | yakın 1:200 | kaydırma | 40 | 0,9 → 0,9 | 1,1 | 0 → 0 | 5,9 → 7,2 | 8,7 |
| 100000 | ısınmış | uzak | imleç | 120 | 0,4 → 0,4 | 0,5 | 0 → 0 | 2,7 → 2,7 | 4,3 |
| 100000 | ısınmış | uzak | tekerlek | 32 | 51,4 → 25,7 | 33,2 | 29 → 7 | 134,8 → 29,1 | 47,7 |
| 100000 | ısınmış | uzak | kaydırma | 40 | 14,9 → 1,0 | 1,1 | 1 → 0 | 221,2 → 44,7 | 48,5 |
| 100000 | ısınmış | yakın 1:200 | imleç | 60 | 0,6 → 0,7 | 0,9 | 0 → 0 | 3,0 → 2,8 | 5,4 |
| 100000 | ısınmış | yakın 1:200 | tekerlek | 32 | 3,3 → 0,7 | 0,9 | 0 → 0 | 4,8 → 5,0 | 7,1 |
| 100000 | ısınmış | yakın 1:200 | kaydırma | 40 | 0,7 → 0,8 | 0,8 | 0 → 0 | 6,9 → 5,8 | 6,8 |
| 1000000 | ilk | uzak | imleç | 120 | 0,9 → 0,9 | 1,0 | 0 → 0 | 5,7 → 6,3 | 8,7 |
| 1000000 | ilk | uzak | tekerlek | 32 | 55,9 → 55,1 | 75,3 | 28 → 28 | 1073,3 → 275,7 | 414,9 |
| 1000000 | ilk | uzak | kaydırma | 40 | 12,7 → 13,9 | 64,2 | 0 → 1 | 1783,8 → 402,9 | 423,4 |
| 1000000 | ilk | yakın 1:200 | imleç | 60 | 0,6 → 0,6 | 0,7 | 0 → 0 | 6,0 → 4,9 | 6,1 |
| 1000000 | ilk | yakın 1:200 | tekerlek | 32 | 3,4 → 3,0 | 25,1 | 1 → 1 | 13,8 → 8,4 | 11,6 |
| 1000000 | ilk | yakın 1:200 | kaydırma | 40 | 1,1 → 1,1 | 1,3 | 0 → 0 | 8,1 → 11,7 | 14,5 |
| 1000000 | ısınmış | uzak | imleç | 120 | 1,0 → 1,0 | 1,1 | 0 → 0 | 6,5 → 5,6 | 6,7 |
| 1000000 | ısınmış | uzak | tekerlek | 32 | 35,0 → 53,1 | 74,3 | 28 → 30 | 1074,5 → 273,4 | 414,8 |
| 1000000 | ısınmış | uzak | kaydırma | 40 | 11,4 → 13,0 | 16,5 | 0 → 0 | 1771,6 → 409,2 | 431,1 |
| 1000000 | ısınmış | yakın 1:200 | imleç | 60 | 0,7 → 0,7 | 0,7 | 0 → 0 | 5,1 → 4,9 | 6,2 |
| 1000000 | ısınmış | yakın 1:200 | tekerlek | 32 | 2,7 → 2,6 | 23,2 | 1 → 1 | 10,7 → 9,6 | 14,1 |
| 1000000 | ısınmış | yakın 1:200 | kaydırma | 40 | 1,0 → 1,0 | 1,2 | 0 → 0 | 11,7 → 11,8 | 14,0 |

Geri okuma tabanı (boş pencerenin karesi, aynı yolla), GPU p50: 10000: 2,4 ms, 100000: 2,3 ms, 1000000: 2,3 ms.

Yazı girişi (bir tuşun karesi; yerel yankı = MİB + GPU), ms:

| Nesne | Komut satırı MİB p95 / GPU p95 | Değer alanı MİB p95 / GPU p95 | En kötü yankı (MİB en çok + GPU en çok) |
|---:|---:|---:|---:|
| 10000 | 1,1 / 11,0 | 0,4 / 9,0 | 14,7 |
| 100000 | 0,9 / 10,7 | 0,5 / 8,9 | 16,0 |
| 1000000 | 1,3 / 12,8 | 0,6 / 8,1 | 14,8 |

Komutlar (bir kare; parçalar: yeniden kurulan stilli katman parçası / bütün parçalar), ms:

| Nesne | Komut | MİB | GPU | Kurulan parça | Kurma |
|---:|---|---:|---:|---:|---:|
| 10000 | İlk komut (bir yazıyı taşı) | 256,7 | 14,8 | 1 / 6 | 0,3 |
| 10000 | Bir yazıyı taşı | 0,8 | 3,8 | 1 / 6 | 0,3 |
| 10000 | Bir yazının metnini değiştir | 1,4 | 4,1 | 1 / 6 | 0,3 |
| 10000 | Bir blok yerleşimini taşı | 8,7 | 3,8 | 1 / 6 | 8,0 |
| 10000 | Bir çizgiyi sil | 3,6 | 4,4 | 1 / 6 | 2,9 |
| 10000 | Geri al | 3,1 | 6,0 | 1 / 6 | 2,5 |
| 10000 | Yinele | 3,1 | 4,0 | 1 / 6 | 2,4 |
| 10000 | Hepsini seç (Ctrl+A) | 15,2 | 7,3 | 0 / 6 | – |
| 10000 | Seçimi bırak (Esc) | 0,5 | 5,6 | – | – |
| 100000 | İlk komut (bir yazıyı taşı) | 240,7 | 15,1 | 1 / 102 | 0,2 |
| 100000 | Bir yazıyı taşı | 0,7 | 4,3 | 1 / 102 | 0,2 |
| 100000 | Bir yazının metnini değiştir | 0,9 | 4,2 | 1 / 102 | 0,2 |
| 100000 | Bir blok yerleşimini taşı | 3,9 | 4,2 | 1 / 102 | 3,4 |
| 100000 | Bir çizgiyi sil | 1,7 | 4,4 | 1 / 102 | 1,2 |
| 100000 | Geri al | 1,5 | 7,4 | 1 / 102 | 1,0 |
| 100000 | Yinele | 1,7 | 4,4 | 1 / 102 | 1,1 |
| 100000 | Hepsini seç (Ctrl+A) | 172,2 | 31,3 | 0 / 102 | – |
| 100000 | Seçimi bırak (Esc) | 0,6 | 4,3 | – | – |
| 1000000 | İlk komut (bir yazıyı taşı) | 274,3 | 19,0 | 1 / 1470 | 22,8 |
| 1000000 | Bir yazıyı taşı | 22,4 | 6,8 | 1 / 1470 | 21,5 |
| 1000000 | Bir yazının metnini değiştir | 22,2 | 9,6 | 1 / 1470 | 21,2 |
| 1000000 | Bir blok yerleşimini taşı | 25,3 | 8,3 | 1 / 1470 | 24,5 |
| 1000000 | Bir çizgiyi sil | 24,6 | 9,8 | 1 / 1470 | 23,7 |
| 1000000 | Geri al | 24,3 | 9,8 | 1 / 1470 | 23,3 |
| 1000000 | Yinele | 23,1 | 10,0 | 1 / 1470 | 22,3 |
| 1000000 | Hepsini seç (Ctrl+A) | 2058,5 | 196,3 | 0 / 1470 | – |
| 1000000 | Seçimi bırak (Esc) | 0,9 | 10,4 | – | – |

İptal (ikinci açılış, okurken Esc), ms:

| Nesne | Esc'nin karesi MİB + GPU | Pencere o karede kapandı | İş parçacığının durması (Esc'den sonra) | Hiçbir şey uygulanmadı |
|---:|---:|---|---:|---|
| 10000 | 4,5 | evet | 1,4 | evet |
| 100000 | 3,3 | evet | 1,4 | evet |
| 1000000 | 5,6 | evet | 1,7 | evet |

Bellek, MB (`footprint`: fiziksel ayak izi ve sürecin tepesi; GPU: IOAccelerator; görünümün GPU tamponları `FrameStats`):

| Nesne | Açılışta yerleşik | Açılışta ayak izi / tepe | Sonda ayak izi / tepe | GPU (IOAccelerator) açılış / son | Görünümün tamponları |
|---:|---:|---:|---:|---:|---:|
| 10000 | 306 | 457 / 457 | 640 / 1037 | 43 / 54 | 22 |
| 100000 | 646 | 834 / 941 | 2163 / 2306 | 46 / 60 | 84 |
| 1000000 | 5016 | 5785 / 6403 | 11264 / 12288 | 40 / 60 | 709 |
