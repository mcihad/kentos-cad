# 1076 — Şeması hazır boş katman GeoPackage'a boş tablo olarak yazılır

Tarih: 2026-11-20. gis-usage-report.md GIS-010. Durum: kabul edildi.

İlgili: [ADR 1073](1073-geopackage-export-scope.md) (dışa aktarım kapsamı), [ADR 1075](1075-geopackage-field-rules-round-trip.md)
(alan kuralları).

## Sorun

Saha ekibine gönderilecek “Bakım rotaları” çizgi katmanı hazırlandı: `rota_kodu` metin alanı görünen adlı ve
zorunlu, henüz rota yok. GeoPackage'a yazılınca dosyada yalnız nokta tablosu vardı; ekibin veri gireceği rota tablosu
ve sütunu yoktu. Bir rota çizilince tablo yazılıyordu.

## Dayanak

- QGIS, New GeoPackage Layer
  ([belgesi](https://docs.qgis.org/3.44/en/docs/user_manual/managing_data_source/create_layers.html#creating-a-new-geopackage-layer),
  20 Kasım 2026'da okundu): katmanın geometri türü ve alanları ilk nesne çizilmeden tanımlanır, tablo boş oluşur.
- GDAL GeoPackage sürücüsü ([belgesi](https://gdal.org/en/stable/drivers/vector/gpkg.html)): nesnesi olmayan
  tablo türü, sütunları ve koordinat sistemiyle yazılır ve okunur.

## Karar

1. Adıyla istenen (`layers`) nesnesiz katman, türü Nokta, Çizgi ya da Alan ise ya da alanları varsa boş tablo olarak
   yazılır: türü `POINT`, `LINESTRING`, `POLYGON` (türsüz alanlı katman öznitelik tablosu), sütunları alanlarından,
   projenin koordinat sistemiyle, alan kurallarıyla (ADR 1075).
2. `file.export.gpkg` `emptyLayers` (geriye uyumlu, varsayılan kapalı): `layers` ve `uids` verilmeden bütün projeyle
   birlikte bu boş katmanlar da yazılır.
3. GeoPackage dışa aktar penceresinde Tüm proje kapsamında “Şeması hazır boş katmanları da yaz (n)” kutusu (açık
   gelir) adlarıyla bunları söyler; kapatılınca yalnız nesnesi olan katmanlar yazılır.

## Sınama

- `gpkg_tests::a_layers_empty_template_goes_out_as_an_empty_table`: boş “Bakım rotaları” 0 satırlı `LINESTRING`
  tablosu olur, `rota_kodu` sütunuyla; geri okununca türlü, alanı görünen adlı ve zorunlu; kutu kapalıyken yazılmaz.
  GDAL dosyada tabloyu Line String, 0 nesne, EPSG:5255 ve `rota_kodu NOT NULL`, görünen adıyla okudu. Resim
  `.run/gui-evidence/gis-010/`.
