# ADR 9214: Çizim alanının katmanları çizicide

- **Durum:** kabul edildi (2026-11-24). Plan: [plan/render.md](../../plan/render.md) RM-14.
- **İstek:** sahibin 23 Kasım isteği: “çizim alanı saf wgpu olmalı, arayüz ile tuval birbirinden tamamen bağımsız
  olmalı”.
- **Bağımlılık:** [9212](9212-drawing-text-on-the-gpu.md) (glif katmanı, aynı karede yayın), 9205 (canlı kamera),
  0165 §5 (ekran işaretleri), 1068 (tekerlekte ölçeklenen resim).

## §1 Sorun

RM-13'ten sonra çizim yazıları GPU'nun glif katmanındaydı. Çizim alanının öbür katmanları hâlâ wgpu sahnesinin üstüne
yığılmış iced kanvaslarıydı:

- **Katmanlar:** yol projesinin arazi yüzeyleri ve hatları, onaylı biçimler, harita servislerinin etiketleri, veri
  katmanlarının etiketleri, tabloda seçilen satırlar, kaydedilmemiş düzenlemeler.
- **Ekran işaretleri:** kuzey oku, ölçek çubuğu, CAD eksenleri ve 0,0 işareti. Yazıların kanvasındaydılar.
- **Sonuçları:**
  - Her kanvas kendi yolunu iced'in tessellator'ıyla üçgenliyordu.
  - Jest boyunca katmanlar resim gibi taşınıp ölçekleniyordu (9205'in izleyen kabı). Bu, ekrana sabit olması gereken
    kuzey okunu ve ölçek çubuğunu tekerlekte çizimle birlikte büyütüyordu.
  - Kanvas ile sahne iki ayrı çizici, iki ayrı kenar yumuşatmasıydı.

## §2 Karar

**Ortak boyama yüzeyi** (`apps/desktop/src/shell/gpu_frame.rs`):

- Katmanlar `Paint` özelliğiyle çizer. Bir iced kanvas çerçevesi de `GpuFrame` da onu uygular, yani katmanın kodu
  ikisinde aynıdır.
- `GpuFrame` kanvasın aldığı her şeyi alır: çizgi, kesik çizgi, dolgu, dikdörtgen, yazı, kaydet/döndür/taşı.
  Bunları glif katmanının parçalarına çevirir:
  - **Yollar** 0,2 px sapmayla düzleştirilir.
  - **Çizgiler** yumuşatılmış kenarlı dörtgenlerdir. Köşeler çizginin istediği birleşimle doldurulur: miter (kanvasın
    sınırı 4, ötesinde kesik), kesik ya da yuvarlak.
  - **Dolgular** sarılma yönüne göre çokgenlere ayrılır, yani deliğiyle çizilir; çekirdeğin üçgenlemesiyle doldurulur.
  - **Yazılar** atlastaki gliflerdir.
- **Çapa:** bir çağrının bütün parçaları tek bir “yazı”dır ve ilk noktasına çapalanır.
  - Haritaya çapalı olanlar piksel birimlidir ve canlı kamerayla taşınır. Jest boyunca bir resmi gibi büyürler.
  - Ekrana çapalı olanlar yeni `GlyphUnit::Screen` birimindedir: ne jestle büyür ne görünümle döner.

**Yayın:**

- Katmanın kanvası artık hiçbir şey çizmez. `draw` sırasında resmini kendi yuvasına (`GlyphSlot`) koyar ve çizici aynı
  karede onu okur (9212 §5).
- Katmanın anahtarı yuvanın kendisinde durur, kanvasın durumunda değil. Gizlenen katmanın yuvası boşaltılır; aynı
  resimle yeniden gösterildiğinde yine çizilir.
- Sürümler bütün yuvalarda tek sayaçtan gelir. Çizici bir katmanı listedeki yeri ve sürümüyle tutar. Bir veri katmanının
  etiketleri görününce liste kayar; aynı sayılı sürümlü başka bir yuva o yerde yanlış katman bulmasın diye.

**Sıra:** çizim alanı yuvaların sıralı listesini alır (`Styling::glyphs`). Çizici her birini kendi katmanı olarak,
yığındaki sırayla, resmin üstüne çizer:

1. arazi, hat, onaylı biçimler, servis etiketleri;
2. veri katmanlarının etiketleri;
3. seçilen satırlar, düzenlemeler;
4. çizim yazıları;
5. ekran işaretleri.

**Ekran işaretleri:**

- Yazıların boyaması işaretleri ayrı bir yuvaya (`Spots::marks`) ekrana çapalı çizer. Gerçek boy kipinde de görünüm
  değişince yeniden çizilir.
- **0,0 işareti** çizimin bir yeridir. `Paint::on_map_of` ile haritaya çapalanır ve jest boyunca çizimle birlikte
  kayar.

**Arayüz yazı tipleri:**

- İşaretlerin harfleri arayüzün yazı tipinde (Plus Jakarta Sans vb.). Atlas önce yalnız çizim yazı tiplerini tanıyordu
  ve bu harfler GPU yolunda düşüyordu.
- Atlas artık gömülü arayüz yazı tiplerini de alır (`glyphs::extra_face`, kimlikleri `EXTRA_FACES`ten başlar).
- Ne çizim ne arayüz yazı tipi olan bir yazı (sistemin kendi yazı tipi) harflerinin dış hatlarıyla doldurulur. Hiçbir
  yazı düşmez.

**Kanvasta kalanlar:** tutamaçlar, kenet işaretleri, artı imleç, aracın önizlemesi ve kartlar. Bunlar ekran boyunda
çizilen etkileşim işaretleridir ve hareket boyunca gizlidirler (9205).

**Açma kapama:** `KENTOS_DIAG_TEXT=canvas` eski yolu bütünüyle geri getirir: yazılar, katmanlar, işaretler.

## §3 Yan düzeltmeler

- **Tekerlekte ölçeklenen resim:** ADR 9207 §8'den beri hareket kareleri tek örnekle çiziliyordu. Bu kareler ayarın
  ölçeklediği keskin resmi bir kenara koyuyordu. `graphics.wheelZoomScaled` açıkken resim hiç ölçeklenmiyordu.
  - Ayar resmi ölçeklerken hareket kareleri artık ayarın örnek sayısıyla çizilir.
  - Bekçi: `perf::stability::a_wheel_zoom_draws_whole_unless_the_setting_scales_the_picture`. Bu dalda kırmızıydı,
    yeşile döndü.
- **Ölçüm koşumu birimi onaylar:** Suşehri DXF, birim sorusu (ADR 9210) kapatıldığı için koşumda hiç açılmıyordu.
  Koşum artık önerilen birimi onaylıyor.

## §4 Kanıt

**Sınamalar:**

- `gpu_frame::tests` (5):
  - kesik desen köşeyi döner;
  - yol, dolgu ve yazı glif katmanına gider;
  - köşe birleşimleri: miter 2, kesik 1 üçgen, yuvarlak yelpaze; miterin ucu çizginin dış köşesi;
  - işaretlerin harfleri arayüz yazı tipinden glif olarak gider;
  - 0,0 işareti ekran işaretleri arasında haritaya çapalanır.
- **GPU'da** (`KENTOS_GPU_TESTS=1`): `holes_stay_open_and_screen_marks_stay_put`.
  - Delikli kare 4 800 px² kaplıyor, ortası boş.
  - Ekrana çapalı kare kaydırılan ve döndürülen görünümde aynı pikselleri boyuyor (en büyük kanal farkı 0).
  - Haritaya çapalı kare kaydırmayla kayıyor.
  - Glif katmanının öbür sınamaları da geçti (8 GPU sınaması).
- **Masaüstü:** yazılar, görüntü alanı, işaretler, yol katmanları, seçim, düzenleme, servisler, açılış: 113 sınama
  geçti.
- **Daha geniş süzgeç** (yol, harita, CBS, servis, görünüm, ölçüm, pafta): 529 geçti, 2 kırmızı. İkisi de RM-14'ten
  önceki commit'te de kırmızıydı (değişiklikler kenara alınıp denendi):
  - `perf::stability`'nin tekerlek sınaması §3'te düzeldi;
  - `project::tests::a_new_project_made_before_the_area_has_its_size_opens_at_its_scale` yeni proje sihirbazının
    akışında takılıyor (belge açılmıyor). Çizimle ilgisi yok, bu işte dokunulmadı.

**Gerçek pencere** (Xvfb, debug + lavapipe, `.run/gui-evidence/render/rm14/`):

- Önce `KENTOS_DIAG_TEXT=canvas`, sonra GPU. Açılış görünümü ve üç tekerlek adımı.
- Fark yalnız çizim alanında sayıldı:

| Dosya | Ne gösteriyor | Açılış: farklı / dörtte birden çok | Yakın: farklı / dörtte birden çok |
|---|---|---|---|
| Yol fikstürü (`road-places`) | arazi yüzeyi, hat, km yazıları, eksenler, 0,0 | 16 536 / 169 | 6 228 / 22 |
| Denetim çizimi | maskeli yazılar, tablo, CAD eksenleri | 3 826 / 235 | 2 379 / 76 |
| Aynı çizim CBS projesi olarak | kuzey oku, ölçek çubuğu | 4 053 / 289 | 2 547 / 124 |

- Dörtte birden çok sapan pikseller harflerde. Kanvas ipuçlu bitmap harf çiziyor, GPU alan (9212 §6).
- Çizgilerin farkı kenar yumuşatmasının dörtte birinden az: hat çizgisi gözle aynı (`kesit-hat.png`).
- İşaretlerin büyütülmüş kesitleri: `kesit-yol-isaretler.png`, `kesit-kuzey-oku.png`, `kesit-olcek-cubugu.png`.
- **Kuzey okunun sivri ucu** ilk sürümde kanvastakinden biraz inceydi: çizgilerin köşelerinde birleşim yoktu. Köşe
  birleşimleri bunun üzerine eklendi (§2).

**Bilinen sınır:** yarı saydam bir çizginin parçaları köşede üst üste biner ve orada biraz koyulaşır. Kanvasın
üçgenlemesi tek bir ağ çıkarıyordu. Yoldaki yarı saydam çizgilerde (kesikli eksen, paralel hat) görünür bir iz yok. Bir
gün görünürse çizgi tek bir dış hat çokgeni olarak çizilir.

## Uygulama

- `crates/render/wgpu/src/glyphs/`:
  - `GlyphUnit::Screen`, `GlyphQuad::polygon` (delikli), `extra_face`;
  - gölgelendiricide ekran birimi;
  - `Renderer::prepare_glyphs` sıralı küme listesi alır.
- `apps/desktop/src/shell/gpu_frame.rs` (yeni): `Paint`, `GpuFrame`, `publish`, `GpuLayers`.
- `apps/desktop/src/cad/labels/`:
  - `GlyphSlot` (anahtar ve tek sayaçlı sürüm);
  - `Spots::marks`, `Labels::marks_of`, `marks_key`;
  - arayüz yazı tipleri.
- Katmanlar:
  - `road/{terrain,alignment}_layer.rs`;
  - `shell/approved_layer.rs`, `gis/{data_selection,data_edits,data_labels}.rs`;
  - `shell/services/overlay.rs`, `map/map_marks.rs`.
- Çizim alanı: `shell/view/area.rs` (sıralı liste), `shell/viewport/{mod,state}.rs`.
- Ölçüm: `shell/perf/large.rs` (birim onayı).
