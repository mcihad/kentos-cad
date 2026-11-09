# 2200 — Topoloji bulgularının hata katmanına yazılması

Tarih: 2026-11-08. plan/map-cad.md MAP-07, üçüncü tur. Durum: taslak. Plan satırları:

- “Topoloji hatası … açıklanabilir kalite sürecine dönüşsün.”
- “GIS'te de Processing, veri formu ve MCP üzerinden aynı denetim sonucu erişilir.”

İlgili:

- [ADR 1980](1980-topology-check-in-processing.md): Topolojiyi kurallarla denetle İşlem aracı. Bu ADR onun
  “Sonra”sındaki “bulguların hata katmanı olarak yazılması, istenirse” maddesini kapatır;
- [ADR 0524](0524-incremental-topology-findings.md) §1: bulgunun kalıcı anahtarı;
- [ADR 0589](0589-topology-check-bound-to-sources.md): istisnaların nesnelerin geometrisine bağlanması.

## Dayanak

- **QGIS, Check validity** (`native:checkvalidity`,
  [Vector geometry](https://docs.qgis.org/latest/en/docs/user_manual/processing_algs/qgis/vectorgeometry.html);
  8 Kasım 2026'da okundu):
  - üç çıktısından biri **Error output**'tur: nokta katmanı;
  - noktalar bulunan geçerlilik sorunlarının tam yeridir, `message` alanı sorunu söyler.
- **ArcGIS Pro, Export Topology Errors**
  ([Data Management](https://doc.esri.com/en/arcgis-pro/latest/tool-reference/data-management/export-topology-errors.html);
  8 Kasım 2026'da okundu):
  - araç, coğrafi veritabanı topolojisinin hatalarını ve istisnalarını dışa aktarır;
  - nokta, çizgi ve alan hataları için üç sınıf yazar;
  - her hatanın alanları şunlardır: kaynak ve hedef nesnenin sınıfı ve kimliği, kural türü, kuralın açıklaması ve
    `IsException` (istisnaysa 1).
- **KentOS bugün** (ADR 1980): araç bulguları tabloda gösterir ve bulgulu nesneleri seçer, çizime bir şey yazmaz.
  Tablo pencere kapanınca gider; bulgular çizimde, paftada ya da GeoPackage'da gösterilemez.

## Karar

1. **Bulguları hata katmanına yaz** (`write`, öntanımlı kapalı). Açıkken tablodaki her bulgu yerinde bir nokta olarak
   **Hata katmanı**'na (`layer`, öntanımlı yeni “Topoloji bulguları”) yazılır.
   - Tabloya giren bulgular yazılır: İstisnaları da listele kapalıyken istisnalar yazılmaz.
   - Yazım tek geri alma adımıdır, adı aracınkidir.
   - Yazılan noktalar sonuçtur: bulgulu nesneler seçilmez.
   - Kapalıyken araç eskisi gibi salt okunurdur.
2. **Nokta, bulgunun yeri.** Yer `cad.topology.check`'inkidir (ADR 1980): örneğin çakışmada parçanın ağırlık
   merkezi, kısa kenarda kenarın ortası.
   - Çizgi ya da alan hata nesnesi (ArcGIS'in üç sınıfı) yazılmaz: bulgunun geometrisi çekirdekte yer ve ölçüdür.
3. **Öznitelikler** tablonun sütunlarıdır:
   - Kural, Sorun, Durum;
   - Nesneler: nesnelerin **kalıcı kimlikleri**, virgülle. Tablodaki `#12` oturumun yeridir, dosyada kalıcı değildir;
   - Ölçü ve Gerekçe, varsa;
   - **Anahtar:** bulgunun kalıcı anahtarı (ADR 0524 §1). Sonraki bir denetim aynı bulguyu bu anahtarla tanır.
4. **Katmanın görünüşü:** kırmızı (`#E5484D`) halka, 10 px. Halka, üstüne düştüğü çizgiden ayrılsın diye seçildi.
   Katman var olan bir katman da olabilir; bu ad yoksa oluşturulur.
5. **Araç = komut:** yazan `cad.processing.run`'dır, aracın kimliği ve `write` ile. Python ve MCP aynı yolu kullanır.
   Pencerenin çalıştırması ile komutunki bir çalıştırmadır ([ADR 0548](0548-coverage-matrix-and-run-equivalence.md)).
6. **Kapsam işareti:** ADR 1980'deki gibi **Harita + Yol + CBS**.

## Kanıt

- **Ortak durumlar:** `scripts/fixtures/geoprocess_cases.py`.
  - Topoloji çizimi v2 biçimine geçti: nesnelerin kalıcı kimlikleri dosyada yazılı. Böylece Nesneler ve Anahtar elle
    bilinir.
  - Anahtar, ADR 0524 §1'in FNV-1a'sının Python'da yeniden yazılmasıdır: kural, kimlikler, toleransa yuvarlanmış yer.
  - Yeni durum `topology-check-errors`: iki çakışma ve kısa kenar için üç nokta, alanları, katmanın görünüşü, tek
    geri alma adımı.
  - İşlem durumlarının koşucusu artık v2 çizimi de okur.
  - Bu Mac'in PROJ'u yeniden yansıtma sayılarını son basamakta oynattığı için yalnız topoloji kısmı eklendi; öbür
    sayılar kayıtlı haliyle kaldı.
  - Geometri, öntanımlar ve pencere ile komut eşitliği denetimleri geçer.
- **MCP** (`crates/native/mcp/tests/topology_errors.rs`): durumun çizimi `.kcad` olarak açılır.
  - Kutu kapalıyken beş nesne seçilir, bir şey eklenmez.
  - Açıkken özet, adım adı, katman ve üç noktanın alanları ile yerleri durumunkidir.
  - Tek geri alma beş nesneye döndürür.
- **Masaüstü** (`processing/geometry_tests.rs`,
  `the_findings_go_to_an_error_layer_in_one_step`):
  - kısa kenarın noktası yerinde, alanlarıyla;
  - seçim yok, tek geri alma katmanı da kaldırır.

  Çapraz masaüstü denetimleri 136 geçer. Resimler `.run/gui-evidence/topoloji-hata-2200/`: kutusu açık pencere ve
  çizimdeki üç halka.

## Sonra

- Hata katmanının GeoPackage'a ve PostGIS'e yazılması (Sonucun yeri) ile istisnaların ayrı alanı (`IsException` gibi).
- Önceki hata katmanını anahtarla eşleyip yalnız yeni bulguları eklemek.
- ~~Veri formu yolu~~ (ADR 3800), taşınmaların sahnede işaretlenmesi, üçlü birleşim ve çoklu köşe reddi (ADR 1980 Sonra).
