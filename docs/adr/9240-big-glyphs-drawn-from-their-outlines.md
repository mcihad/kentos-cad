# ADR 9240: Büyüyen glif dış hattından çizilir

- **Durum:** kabul edildi (2026-11-25).
- **Plan:** [plan/render.md](../../plan/render.md) RM-13'ün eksik kalan maddesi ([ADR 9212](9212-drawing-text-on-the-gpu.md)
  §3: “glif ekranda ~512 px'i geçince vektör olarak çizilir”) ve RM-11'in yazı işaretlerine giden yolu.
- **İstek:**
  - sahibin 23 Kasım sözü: “SVG çizgileri, stil çizgileri çok zoom yapınca testere gibi”;
  - 24 Kasım: “Autocad kusursuz büyütebiliyor”.
- **Sözleşmeler:**
  - Paylaşılan WGSL değişmedi. Glif gölgelendiricisi yalnız masaüstüne ait.
  - Glif örneği 88 → 92 bayt; ek alan, programın yeri.

## §1 Bulgu

ADR 9212 §3, glif ekranda ~512 px'i geçince dış hattından çizileceğini söylüyordu; bu hiç yapılmamış. Her boy SDF'ten
çiziliyordu. SDF'in özellikleri: em başına 64 px, 8 px yayılma, bayt (R8).

Çok yakında bu üç kusur çıkarıyor:

- **Köşe yuvarlanıyor.** Çift doğrusal okuma, keskin köşeyi bir alan pikseli boyunca yuvarlıyor.
- **Basamak.** On altı kattan sonra baytın adımları basamak olarak görünüyor.
- **Biçim bozuluyor.** Harfin eğik ucu bir yumruya dönüşüyor.

Kanıt (gerçek pencere): 2 m'lik “KEL 47”, açılıştan 36 kat yakında. E'nin üst kolunun ucu basamaklı bir yumru
(`.run/gui-evidence/render/rm13v/`).

**GPU sınaması** (em ≈ 3 000 px, E'nin köşesi, CPU'da birebir dış hat referansıyla karşılaştırma):

| | Ortalama fark | Çeyrekten fazla sapan piksel |
|---|---|---|
| Halesiz | 0,52 | 436 |
| 3 px hale | 1,87 | 867 |

## §2 Karar

### §2.1 Program: dış hattın üçgenleri ve kenarları

`glyphs::vector::program` her glif için bir program kurar. Atlas onu glifin alanıyla birlikte yapar.

- **Dış hat:**
  - em başına 1/20 000 içinde düzleştirilir. Bölme sayısı eğrinin ikinci farkından hesaplanır (karesel için sapma
    ≤ fark/4n²).
  - Sarım kuralıyla doldurulur ve üçgenlenir (`svg_marks::fill_nonzero`).
- **Gözler:** üçgenler glifin kutusunda gözlere bölünür (`vector_tiles::bin_triangles`, ADR 9239'unki). Dış hattın
  kenarları da hale erimi kadar yakınındaki gözlere yazılır.
- **Hale erimi** (`HALO_EM`) em'in 1/32'si: 512 px'lik em'de 16 px, büyüdükçe daha fazla. Alanın kendi erimi olan
  em'in 1/8'i, her kenarı dört kat daha çok göze yazdırıyordu (program glif başına 307 KB).
- **Saklama:** programlar ortak bir depolama tamponunda (bağlama 3). Tampon büyüyünce katmanların bağlamaları yeni
  sayfa gibi yenilenir (`generation`). Glif örneği programın yerini taşır.

### §2.2 Gölgelendirici: büyükse dış hattan

`glyphVs` glifin em'ini aygıt pikseliyle ölçer. 512'yi geçerse `glyphFs` glifi programından okur:

- Parça, glifin em düzlemindeki yeridir. Dört örnek noktası çizimin 4× MSAA düzenindedir.
- Glif geçişi resmin üstüne tek örnekle çizildiği için kenarı bu örnekler yumuşatır.
- Mürekkep, içerideki örneklerin payı kadardır.
- Hale, dış hatta hale genişliği içindeki örneklerin payı kadardır ve mürekkebin altına konur (alanın halesi gibi).

`vector::read` gölgelendiricinin CPU'daki okumasıdır.

### §2.3 Kendini kesmeyen halkalar hızlı yoldan

`svg_marks::fill_rings`, halkaları önce x'e göre bir süpürmeyle denetler (`crossing`).

- Hiçbir kenar başka birini kesmiyorsa halkalar eskisi gibi iç içe dizilir.
- Kesişim motoruna yalnız kesişen halkalar gider: keskin dönen SVG çizgisinin dış hattı, üst üste binen konturlar.
- Kazanç: glif programının yapımı 0,61 → 0,30 ms.

## §3 Kanıt

**Sınamalar:**

- `glyphs::vector`: 6 glifte (E, 8, g, ğ, @, K) programın okuması, konturların kendi sarım sayısıyla
  karşılaştırıldı. Glif başına 200 × 200 noktada fark 0. Hale erimindeki uzaklıklar 1e-5 em içinde.
- `svg_marks`: kesişim denetimi; sekiz rakamı, üst üste ve iç içe kareler, geri katlanan halka, değen halka.
- GPU (UHD 770): E'nin köşesi, em ≈ 3 000 px:

  | | Önce, ortalama fark | Önce, sapan | Sonra, ortalama fark | Sonra, sapan |
  |---|---|---|---|---|
  | Halesiz | 0,52 | 436 | 0,01 | 0 |
  | 3 px hale | 1,87 | 867 | 0,02 | 1 |

  Eşiğin altındaki yazı alanından çiziliyor; ortalama fark 0,24 ile eskisinin aynısı.
- render-wgpu'nun bütün sınamaları (GPU dahil) geçti. Masaüstünün etiket, yazı, altın görüntü ve uygunluk
  sınamaları: 87 geçti.

**Gerçek pencere** (Xvfb ve lavapipe):

- Açılış ve en yakın görüntü öncekiyle aynı (fark 0). En yakında görüntünün tamamı harfin içi.
- 6 ve 36 kat yakında 2 258 ve 5 057 piksel değişti: E'nin ucu kendi eğik biçiminde, keskin.

**Bedel** (sürüm derlemesi, Barlow'un 78 glifi):

| | Değer |
|---|---|
| Program | glif başına 25,5 KB (SDF'i ~6 KB) |
| Yapım | glif başına 0,30 ms (SDF'i 0,24 ms) |

Yalnız kullanılan gliflerin programı kurulur.

## Uygulama

**Glifler:**

- `crates/render/wgpu/src/glyphs/vector.rs` (yeni): `program`, `read`.
- `atlas.rs`: `Slot::program`, `GlyphAtlas::programs`.
- `gpu.rs`: programların tamponu, bağlama 3, örneğin alanı.
- `glyphs.wgsl`: `vgRead`, `vectorGlyph`, büyüklük denetimi.

**Native stil:**

- `svg_marks.rs`: `fill_nonzero`, `crossing`.
- `vector_tiles.rs`: `bin_triangles` paylaşılan oldu.

**Sınamalar:**

- `tests/gpu/glyphs.rs`: `a_glyph_past_its_field_is_read_from_its_outline`.
- Birim sınamaları yukarıda.
