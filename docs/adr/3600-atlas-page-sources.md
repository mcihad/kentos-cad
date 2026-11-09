# 3600 — Atlas sayfalarının kaynak dökümü

Tarih: 2026-11-09. plan/map-cad.md MAP-18, altıncı tur. Durum: taslak. ADR 1320 §7'nin ilk kalan işi: “Atlas sayfaları
(zemin bilinmeyen pencere) için sayfa başına kaynak dökümü ve atlas nesnesinin kimliği”.

İlgili:
- [ADR 0534](0534-atlas-pdf-pinned-revision.md): atlas PDF'i sabitlenen sürümden (`file.export.atlas`);
- [ADR 1320](1320-sheet-sources-and-update-publication.md): paftanın kaynak dökümü ve güncelleme yayını.

## Dayanak

- **QGIS atlas** (QgsLayoutAtlas, ADR 1320'nin okuması): kapsama katmanının her nesnesi bir sayfadır; harita o nesneye
  göre yerleşir. Değişen sayfayı bulmak kullanıcının ifadesidir.
- **ArcGIS Pro harita serisi** (ADR 1320'nin okuması): dışa aktarım sayfa kümesini kullanıcı seçer; hangi sayfanın
  değiştiğini ürün söylemez.
- **KentOS bugün:**
  - paftanın sayfası kaynaklarını listeliyordu (ADR 1320 §2). Zemini bilinmeyen penceresi olan sayfanınki `null`'dı:
    atlas haritası sayfadan sayfaya yer değiştirir, paftanın tek bir zemini yoktur;
  - atlasın cevabı her sayfanın nesnesini (`feature`) ve kutusunu veriyordu, sayfanın çizdiklerini vermiyordu.
- **Bulunan:** atlas şablonundaki genel bakış haritası (1/25000, `overviewOf`) her sayfada atlas haritasını izler. Zemini
  1,8 × 1,9 km olduğundan örnek çizimin her sayfası çizimin bütününü gösterir.

## Karar

1. **Sayfanın zeminleri:** her atlas sayfasının, PDF'in onu dizdiği gibi her haritası ve projenin zeminindeki kutusu
   (`AtlasPlanned::page_grounds`; sayfanın görüntü listesi, PDF çizilmeden):
   - atlas haritası sayfanın yerinde, genel bakış onu izleyerek, duran haritalar kendi yerinde;
   - sayfanın çizmediği (yerleşmemiş) harita sayfaya bir şey katmaz.
2. **Sayfanın kaynakları** (`sheets_sources::atlas_page_sources`, ADR 1320 §2'nin kuralı sayfa sayfa):
   - sayfanın kendi nesnesi önce (atlas nesnesinin kimliği);
   - sonra çizim sırasıyla: bir haritanın zeminine kutusu değen ve haritanın bastığı nesneler, gösterilen model tablolarının
     okudukları, sayfaya bakan görünüm işaretleri, kâğıt öznitelik tablosunun katmanındakiler;
   - kutu sınaması fazlasını listeleyebilir, eksiğini listelemez. Katmanın süzgeci kutu sınamasında uygulanmaz (paftanınki
     gibi).
3. **Cevap** (`file.export.atlas`, ek alanlar; komut sürümü değişmez):
   - `pages[].sources`: sayfanın kaynakları, kalıcı kimlikleriyle;
   - `objects`: listelenen her nesnenin içerik özeti, ADR 1320'nin `kentos.object-digest/1`'i (anlık görüntü biçimi,
     yuva kimliği çıkarılarak, anahtarlar sıralı, SHA-256).

   Doğrulama (`validate`) sayfa yapmadığı için kaynak listelemez.
4. **Masaüstü** (Atlası PDF yap): önizlemenin sayfa listesinde **Kaynak** sütunu (sayfanın kaynak sayısı).
5. **Kapsam işareti:** ADR 1320 gibi **CAD + CBS** (pafta).

## Kanıt

- **Bağımsız başvuru** (`scripts/fixtures/atlas_cases.py`, pyproj; KentOS kodu yok):
  - sayfanın zemini: merkez ± haritanın içeriğinin yarısı (165 × 150 mm) çarpı ölçek. Genel bakış aynı merkezde 72 × 76 mm
    çarpı 25000. Kutu sınaması parsellerin ve yolların kutularıyla;
  - iki yeni durum: şablonun kendisiyle her sayfa 109 nesne; yalnız atlas haritalı paftada (yeni `plain` kurulumu: genel
    bakış ve katman tablosu çıkarılmış) her sayfa yalnız komşularını sayar. İlk sayfa 4, büyük parselin sayfası 14 nesne;
  - ilk sayfanın dört nesnesinin özeti çizimin JSON'undan hesaplanır;
  - beklentide `$uidOf:N` çizimin N numaralı nesnesinin kalıcı kimliğidir; atlas koşucusu bunu çözer. 37 durum geçer.
- **Python** (`python/tests/test_atlas_page_sources.py`): yalnız atlas haritalı paftada 100 sayfanın kaynakları ve dört
  özet durumunki; genel bakışla her sayfa 109 nesne.
- **MCP** (`crates/native/mcp/tests/atlas_page_sources.rs`): her sayfa kendi nesnesiyle başlar, kaynak sayısı durumunki,
  listelenen her nesnenin özeti var.
- **Masaüstü** (`exchange::atlas_tests::the_preview_lists_each_page_s_sources`):
  - önizlemede ilk sayfa 4, büyük parselinki 14 kaynak; genel bakışla her sayfa 109;
  - resimler `.run/gui-evidence/atlas-kaynak-3600/` (Kaynak sütunu).

  Atlas pencere testleri ve ADR 0534'ün durumları değişmeden geçer.

## Sonra

- Atlasın güncelleme yayını: önceki çıktının kaynak dökümüyle değişen sayfaların bulunması (ADR 1320'nin `previous`'u
  gibi). Kayıt bugün yalnız cevaptadır; çıktının yanında bir kayıt dosyası gerekir (ADR 0534'ün “Sonra”sı).
- Total station ve nivelman kalitesi (MAP-18'in öbür maddesi).
- Web (TODOS §0.2).
