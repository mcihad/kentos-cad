# ADR 9245: Yere ölçülü yazı işareti büyüyünce dış hattından çizilir

- **Durum:** kabul edildi (2026-11-25).
- **Plan:** [plan/render.md](../../plan/render.md) RM-11'in açık maddesi (“yazı işaretleri”).
- **İstek:**
  - 23 Kasım: “SVG çizgileri, stil çizgileri çok zoom yapınca testere gibi”;
  - 24 Kasım: “kaliteden ödün verilmez”.
- **Sözleşmeler:**
  - Paylaşılan WGSL ve stil çekirdeği değişmedi.
  - `DecodeOptions`'a `text_outlines` eklendi: ev sahibinin yazı tipleri. Yoksa (`None`) her yazı işareti eskisi gibi
    bitmap kalır.

## §1 Bulgu

Stil katmanının yazı işareti (`AtlasImage::Text`) atlasa bitmap olarak basılır (`raster::text_image`): kutunun
yüksekliğinde, en çok 512 px. Yere ölçülü bir yazı işareti yakınlaştıkça büyür ve bitmap'i büyütülür. Kenarları önce
bulanıklaşır, sonra basamaklanır.

SVG simgeleri bunu ADR 9208'de aşmıştı; yazı işaretleri dışarıda kalmıştı. Sebep: yazının dış hattı ev sahibinin yazı
tiplerinden çizim anında geliyor (`ImageSource::text`), çözme anında değil.

NCZ'de yere ölçülü yazı işaretleri var. Gerçek pencere ölçümü bunu gösterdi (§3).

## §2 Karar

### §2.1 Yazı, bitmap'inin bastığı SVG olarak yazılır

`svg_marks::text_svg`, yazı işaretini bitmap'in yerleşimiyle bir SVG'ye yazar:

- kutu 100 birim yüksek, harfler 1,25 kat küçük (80);
- taban çizgisi kutunun %78'inde;
- genişlik, harflerin ilerlemesi artı iki yanda hale;
- hale, harflerin boyunun verilen payı kadar her yanda: yuvarlak birleşimli bir çizgi, dolgunun altında;
- dolgu sarım kuralıyla.

SVG, simgeninkiyle aynı yoldan üçgenlenir (`svg_marks::mesh`): dolgu ve hale ayrı renk toplu çizimleri. Yazı tipleri
çözmeye `DecodeOptions::text_outlines` ile gelir. Masaüstünde bu, stilin görüntüleri (`Images`);
`StyledCache::text_from` ile bütün katman parçalarına geçirilir.

### §2.2 Kutu ve geçiş yükseklikten

- **Kutu:** yazı işaretinin kutusu yüksekliğinin en/boy oranına bölünmüşü kadar geniştir (işaret gölgelendiricisinin
  `fit` 2'si). Yerleştirmenin ölçeği bu genişliktir.
- **Geçiş:** bitmap toplu çiziminin baktığı şeye bakar, yani yazının yüksekliğine. Ekranda 128 pikseli geçince vektörler
  çizilir, altında bitmap; ikisi birden hiçbir zaman.

### §2.3 Okunamayan yazı hiçbir yoldan çizilmez

Bitmap toplu çizimi vektörlü olunca (`vectors: true`) `legible` yalnız “128 pikselin altında mı” diye bakıyordu. Yazının
“3 pikselden küçükse çizme” kuralı (`MIN_TEXT_PX`) böylece atlanıyordu.

Gerçek pencere ölçümü bunu yakaladı: NCZ'nin 8 çentik görüntüsünde 1 327 piksel farklıydı, minik yazılar görünür
olmuştu. Kural artık iki yolda da geçerli.

## §3 Kanıt

**GPU** (UHD 770, 1 ve 4 örnek). Paketteki yazı tipiyle 10 m'lik bir “E” yazı işareti, glifin dış hattının analitik
kaplamasıyla karşılaştırıldı; kenara değmeyen pikseller sayıldı.

| Ölçek | Yol | Ters çizilen piksel |
|---|---|---|
| 0,95 × geçiş | bitmap | 0 / 910 |
| 1,05 × geçiş | vektör | 0 / 1 181 |
| 4 × geçiş | vektör | 0 / 2 295 |
| 20 × geçiş | vektör | 0 / 11 940 |
| 20 × geçiş, vektörsüz (eski) | büyütülmüş bitmap | 110 / 11 940 |

Eşik binde bir. Büyütülmüş bitmap bu eşiği aşıyor, vektör sıfır.

**CPU sınamaları:**

- **SVG yerleşimi:** kare harfli bir yazı tipiyle genişlik 96, yükseklik 100; hale çizgisi 16 genişlikte ve dolgudan
  önce; harfin köşeleri beklenen yerde; iki renk.
- **Çizim yolu:** okunamayacak kadar küçükken hiçbiri, küçükken bitmap, büyüyünce vektör.

**Gerçek pencere** (Xvfb ve lavapipe, NCZ):

- Düzeltmeden önce 8 çentikte 1 327 piksel farklıydı (§2.3).
- Düzeltmeden sonra dört görüntü de öncekiyle birebir aynı. En yakın görüntü (20 çentik) yazıları 128 pikselin altında
  tutuyor.

**Paketler:**

- native stil, render-wgpu (GPU dahil) ve plot sınamaları geçti. plot'un `bands` sınaması bundan önce de kırmızıydı
  (`bound_in_gis`).
- Masaüstünde 310 sınama geçti.

**Sınama düzeni:** aynı ikilideki GPU sınamaları artık sırayla koşuyor (`ONE_AT_A_TIME`). Ölçüm sayacı
(`DRAWN_INSTANCES`) sürecin geneli: yan yana koşan bir sınama öbürünün çizimlerini de sayıyordu.

## Uygulama

- `crates/native/style/src/svg_marks.rs`: `text_of`, `text_svg`, yükseklikten kutu ve geçiş.
- `crates/native/style/src/batches.rs`: `DecodeOptions::text_outlines`, `legible`.
- `apps/desktop/src/cad/style/scene.rs`: `StyledCache::text_from`.
- `apps/desktop/src/shell/viewport/state.rs`: stilin görüntüleri yazı tipi olarak verilir.
- Sınamalar:
  - `svg_marks.rs`: `a_text_marker_is_written_as_its_bitmap_paints_it`, `a_text_marker_is_drawn_one_way_or_none`;
  - `crates/render/wgpu/tests/gpu/text_marks.rs`.
