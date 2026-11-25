# ADR 9238: Bir çizgi boyunca işaretler: sınır 1 000 000, aşanı sayılır ve söylenir

- **Durum:** kabul edildi (2026-11-25). Plan: [plan/render.md](../../plan/render.md) RM-11, asıl madde.
- **İstek:**
  - planın RM-11 maddesi: “İşaret ve simgeler GPU'da örnekli çizilir; yol başına 50 000 işaret sınırı kalkar”;
  - kabul ölçütü: “Görüntüde sessizce düşen nesne: 0”;
  - 25 Kasım: “Devam et, asla durma, bitene kadar tüm maddeler.”
- **Sözleşmeler:** değişmedi. Stil fikstürleri, `.kcad` ve stil JSON'u aynı.

## §1 Bulgu

**İşaretler GPU'da zaten örnekliydi.**

- Her yerleşim 5 sayı tutar: x, y, açı, en, boy (`MARKER_FLOATS`).
- Her işaret, 4 köşeli bir dörtgenin örneği olarak çizilir (`VertexStepMode::Instance`).
- Büyük SVG simgeleri [ADR 9208](9208-svg-symbols-as-vectors.md)'den beri vektör örneklerle çiziliyor.

**Kalan kayıp, `place_along`'daki yol başına 50 000 sınırıydı.** Yerleşimi sessizce kesiyordu:

- 60 km'lik, 1 m aralıklı bir çizginin işaretleri 50. km'de bitiyordu.
- 5 cm aralıklı bir çizgininkiler 2,5. km'de bitiyordu.
- Kullanıcıya hiçbir yerde söylenmiyordu.

[ADR 0930](0930-complex-linetype-renderer-audit.md) bu sınırı “görünmeyen işaretler için sessiz başarı verilemez”
diye not etmişti. Dalgalar (`wave_paths`) da aynı sabitle kesiliyordu.

## §2 Karar

### §2.1 İşaret sınırı 1 000 000; aşanı sayılır

`MAX_MARKERS_PER_PATH` 50 000'den 1 000 000'a çıktı.

**Sınır neden kalıyor:** yanlış birimle girilmiş çok küçük bir aralık, sonu gelmeyen bir yerleşim üretir. Yol başına
1 000 000 işaret, kurulum sırasında en çok ~40 MB tutar (işaret başına beş `f64`).

**`place_along_counted`** sınırın bıraktığı yerleri, yaratmadan sayar:

- Aralık kipinde aritmetikle. Döngünün kendisi gibi sayar; 5 cm'lik çizgide kayan noktanın birikimiyle son yer
  60 000 m'yi aşıyor, döngü de onu üretmezdi.
- Grup kipinde kalan yerler grup sayısıyla çarpılır. Grubun açık çizginin ucunu aşan üyeleri de sayılır, yani sayı
  en çok bir grubun üyeleri kadar fazla olabilir.

`Sink::markers_left_out` sayıyı alır. Bir yer, işaret simgesinin her katmanı için ayrı bir işarettir; sayı katman
sayısıyla çarpılır.

### §2.2 Dalga sınırı 50 000; o da sayılır

Bir dalga 16 çizgi parçasıdır: 96 sayı. Bir işaret 5 sayıdır. Yani 50 000 dalga, 1 000 000 işaret kadar bellek
tutar; iki sınır bellekte eşittir.

- Sayıyı `wave_paths_counted` üretir, `Sink::waves_left_out` alır.
- Bağlı dalgalarda çizilen dalgaların dışındaki uçlar düz çizgi olarak kalır.
- Bir blok tanımı kaydedilirken dalgası kesilirse tanım örneklenmez. Her yerleştirme kendi kesilen dalgalarını sayar.

### §2.3 Söylenir, katman ve tür başına bir kez

Sayı, Nokta yoğunluğunun sayısıyla ([ADR 0213](0213-additional-renderers.md) §2.4) aynı yoldan masaüstüne gider:

1. `Batches`;
2. `StyledLayer` (`markers_left`, `waves_left`);
3. `StyledLayerPart`;
4. `StyledCache`.

`StyledCache` artık üç türü (`LeftOut`: nokta, işaret, dalga) katman başına birer kez söyler.

Uyarılar, simge düzenleyicisindeki alan adlarıyla:

- “Sık işaret (5 cm)” katmanında 200.000 işaret çizilmedi: bir çizgi boyunca en çok 1.000.000 işaret çizilir.
  Simgenin Aralık değerini büyütün.
- Dalga için: “… en çok 50.000 dalga çizilir. Simgenin Tekrar değerini büyütün.”

Sayılar uygulamanın her yerdeki biçimiyle yazılır (12.345). Nokta yoğunluğu uyarısı aynı cümlede “200.000” ile
“1 000 000”ı yan yana yazıyordu; o da düzeldi.

## §3 Kanıt

**Sınamalar:**

| Sınama | Ne gösteriyor |
|---|---|
| `a_long_line_keeps_every_marker_and_counts_those_past_the_limit` | 60 km'lik, 1 m aralıklı çizgi: 60 001 işaret, 0 kalan. 1 500 001 yer: 1 000 000 çizilen + 500 001 kalan. 400 000 yer × 3'lük grup: 1 000 000 + 200 000 |
| `waves_past_their_limit_are_counted` | 120 000 dalga sığıyor; 50 000 çizilen, 70 000 kalan |
| `a_layer_build_says_the_markers_and_waves_its_paths_left_out` | Katman kurulumu, iki işaret katmanı × 101 = 202 işaret ve 70 000 dalga söylüyor |
| masaüstü: `the_markers_a_layer_left_out_are_said_once` | Sahnenin parçaları sayıyı taşıyor; ikinci sahne yeniden söylemiyor |

- Stil çekirdeğinde 98, native stilde 108, çizim çıktısında 119, masaüstünün stil sahnesinde 17 sınama geçti.
- Çizim çıktısının tek kırmızısı (`bands::a_part_s_band_holds_room_in_the_book_and_one_not_read_is_said`,
  GeoPackage bağı) main'de bu işten önce de kırmızıydı.

**Gerçek pencere** (Xvfb ve lavapipe, hata ayıklama): iki adet 60 km'lik çizgi, işaretler 7 px'lik daire.

| | Önce | Sonra |
|---|---|---|
| Yeşil çizgi, 1 m aralık, 60 001 yer | 50 000 işaret; 50. km'de kesiliyor | 60 001 işaret; sonuna kadar |
| Kırmızı çizgi, 5 cm aralık, 1 200 000 yer | 50 000 işaret; 2,5. km'de kesiliyor | 1 000 000 işaret; 50. km'ye kadar |
| Geçmiş | Uyarı yok | “Sık işaret (5 cm)” katmanında 200.000 işaret çizilmedi … |

- Yeşil çizginin 50. km'sinde, aynı kamerayla (1:164): önce işaretler 49 999. metrede bitiyor, sonra sürüyor.
- Görüntüler: `.run/gui-evidence/render/rm11-sinir/rm11-{acilis,yakin,gecmis}-once-sonra.png`.

**Bellek:** `perf::memory::file_parts`, aynı çizim, 1 060 001 işaret.

| Ölçüm | Sonuç |
|---|---|
| Gerçek GPU (UHD 770), açılış | Yerleşik 316 → 432 MB |
| Gerçek GPU, CPU'da kalan sayılar | 40 MB (veri 20, artık 20) |
| Gerçek GPU, GPU bırakılınca geri verilen | 75 MB |
| Lavapipe, GPU bırakılınca geri verilen | 750 MB |

- Gerçek penceredeki 554 → 1 007 MB'ın büyüğü bu yüzden ölçüm ortamının. Yazılım GPU'su bir milyon dörtgeni
  çizmek için kendi belleğini ayırıyor; büyük bloklar sisteme geri verilse de (`MALLOC_MMAP_THRESHOLD_=65536`)
  147 → 443 MB'ta kaldı.
- Gerçek donanımda 1 000 000 işaretin bedeli ~110 MB.
- İşaretli katmanların sayıları CPU'da kalıyor, çünkü maskeler işaret kutularını CPU'dan okuyor
  ([ADR 0226](0226-symbol-layer-extras.md) §3). Bu, RM-11'den önce de böyleydi: 1 000 000 nokta işaretli bir katman da aynı
  40 MB'ı tutuyordu. Sayıların yalnız maske kaynağı olan katmanlarda kalması RM-19'a yazıldı.

## Uygulama

**Stil çekirdeği:**

- `crates/shared/style-core/src/style/place.rs`:
  - `MAX_MARKERS_PER_PATH`, `MAX_WAVES_PER_PATH`;
  - `place_along_counted`, `wave_paths_counted`.
- `crates/shared/style-core/src/style/prim.rs`: `Sink::markers_left_out`, `Sink::waves_left_out`.
- `crates/shared/style-core/src/style/compile.rs`: sayımlı yerleşimler, sayılar alıcıya.
- `crates/shared/style-core/src/style/cad_band.rs`: geniş çizginin sarmalayıcısı sayıları iletir.
- `crates/shared/style-core/src/style/batch.rs`: `BatchSink` ve `Batches`'te `markers_left`, `waves_left`.

**Taşıyanlar:**

- `crates/native/style/src/batches.rs`: `StyledLayer`'da `markers_left`, `waves_left`.
- `crates/native/style/src/lib.rs`: sınırlar dışarı açıldı.
- `crates/render/wgpu/src/styled/gpu.rs`: `StyledLayerPart::markers_left`, `waves_left`.

**Masaüstü:**

- `apps/desktop/src/cad/style/scene.rs`: `LeftOut`; katman ve tür başına bir kez.
- `apps/desktop/src/shell/viewport/state.rs` ve `apps/desktop/src/shell/app/update.rs`: uyarılar.
