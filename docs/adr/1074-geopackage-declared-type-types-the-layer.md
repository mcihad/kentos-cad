# 1074 — GeoPackage tablosunun bildirdiği geometri türü CBS katmanının türü olur

Tarih: 2026-11-20. gis-usage-report.md GIS-008. Durum: kabul edildi.

İlgili: [ADR 0622](0622-interface-arrangement.md) §5 (katman türü), [ADR 1070](1070-gis-run-output-layer-type-and-fields.md)
(işlem çıktısının türü), [ADR 0503](0503-bound-layers.md) (bağlı katman).

## Sorun

Envanter paketi yeni bir CBS projesine alındığında `POLYGON` olarak tanımlı “Park sınırları” tablosunun katmanı
türsüz (Karışık) geldi; Etkin katman yapılınca Nokta aracı açıktı ve kayıttan sonra katmanda iki poligonla bir nokta
vardı. Paketin `POINT`, `LINESTRING` ve `POLYGON` tablolarının üçünde de KentOS tür kuralı eksikti.

## Dayanak

- OGC GeoPackage, `gpkg_geometry_columns.geometry_type_name`
  ([belgesi](https://www.geopackage.org/spec/#_geometry_columns)): tablonun geometri türü (ör. `POLYGON`,
  `MULTILINESTRING`, `GEOMETRY`) tablo başına bildirilir.

## Karar

1. CBS projesinde (`settings.workspace = gis`) içe aktarmanın açtığı yeni katman, tablosunun bildirdiği türün ailesini
   alır: `POINT`/`MULTIPOINT` → Nokta; `LINESTRING`, `MULTILINESTRING`, `CIRCULARSTRING`, `COMPOUNDCURVE`, `CURVE`,
   `MULTICURVE` → Çizgi; `POLYGON`, `MULTIPOLYGON`, `CURVEPOLYGON`, `SURFACE`, `MULTISURFACE`, `TRIANGLE`, `TIN`,
   `POLYHEDRALSURFACE` → Alan; Z, M, ZM biçimleri düz türlerininki. `GEOMETRY`, koleksiyonlar ve bilinmeyen türler
   Karışık kalır. Kod: `geopackage::layer_geometry_of`, `file_import_gpkg::run_read` (aynı geri alma adımında).
2. Katman türlenmez: tablonun bir satırı türe uymuyorsa (geometrisiz kayıt dahil), katman bağlı katmansa (tablonun
   sonraki satırları olduğu gibi gelir) ya da proje CBS projesi değilse.
3. FlatGeobuf, GML, CSV ve GeoParquet aynı `run_read`'den geçer; bildirdikleri tür aynı yolla okunur.
4. Var olan türlü katmana ekleme ve sonraki yazımlar çizimin bütünlük denetimine tabidir (ADR 0622 §5).

## Sınama

- `gpkg_tests::a_geopackage_tables_declared_type_types_its_layer`: eğrili poligon tablosu Alan katmanı olur, etkinken
  Nokta kapalı, Kapalı alan açık, kayıt ve yeniden açmada kalır; nokta tabloları Nokta katmanı; karma projede türsüz.
- Ortak komut durumları (`fixtures/commands/v1`) değişmeden geçer (bilinen 41 ilgisiz hata dışında).
