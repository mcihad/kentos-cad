# 1073 — GeoPackage dışa aktarımının kapsamı: seçili nesneler, etkin katman, tüm proje

Tarih: 2026-11-20. gis-usage-report.md GIS-007. Durum: kabul edildi (tablonun süzgecini ayrı kapsam olarak sunmak açık).

İlgili: [ADR 0502](0502-geopackage.md), [ADR 0646](0646-historical-time-query-export-time-and-sensor-streams.md) §2 (zamanla dışa aktarım).

## Sorun

Tek park seçiliyken Dosya › GeoPackage dışa aktar doğrudan dosya adı istedi ve bütün envanteri (2 park, 3 ağaç,
1 yol, 1 tampon) yazdı; GeoJSON penceresi aynı seçimde “Seçili (1)” gösterip yalnız o parkı yazıyordu. Saha ekibine
bir park göndermek isteyen kişi fazladan altı nesneyi göremiyordu.

## Dayanak

- QGIS, Save Vector Layer as… › “Save only selected features”
  ([belgesi](https://docs.qgis.org/3.44/en/docs/user_manual/managing_data_source/create_layers.html#vector-specific-parameters),
  20 Kasım 2026'da okundu): kayıt penceresi yazmadan önce yalnız seçili nesneleri yazmayı sorar.
- ArcGIS Pro, Export Features
  ([belgesi](https://doc.esri.com/en/arcgis-pro/latest/tool-reference/conversion/export-features.html), 20 Kasım
  2026'da okundu): katmanda seçim varsa yalnız seçili nesneler aktarılır; araç bunu girdisinde söyler.

## Karar

1. **Pencere** (`apps/desktop/src/shell/exchange/gpkg_out.rs`). GeoPackage dışa aktar önce kapsamı sorar:
   “Seçili nesneler (n)”, “Etkin katman (n)”, “Tüm proje (n)”; seçim varsa seçili nesneler önde gelir, yoksa o
   seçenek kapalıdır. Özet yazılacak nesne ve katman sayısını ve katmanların adlarını söyler; zaman sürgüsü açıksa
   zaman satırı da gelir (Shapefile penceresindeki gibi). Dışa aktar… dosyayı sorar.
2. **Komut** (`file.export.gpkg` v1, geriye uyumlu): `uids` verilirse yalnız o nesneler, kendi katmanlarının
   tablolarına yazılır; olmayan kimlik `entity_not_found` ile reddedilir. Etkin katman `layers: [etkin]`, tüm proje
   boş `layers` ile yazılır.

## Sınama

- `gpkg_tests::the_geopackage_export_writes_the_scope_it_shows`: iki park, bir yol, üç ağaç; ikinci park seçili.
  Seçili nesneler 1 park, Etkin katman 2 park, Tüm proje üç tablo (2, 1, 3) yazar; satırlar dosyadan geri okunur.
  Resim `.run/gui-evidence/gis-007/`.

## Açık

- Öznitelik tablosunun süzgecini (görünen kayıtlar) ayrı kapsam olarak sunmak.
- Web'in GeoPackage dışa aktarımı aynı pencereyi ister (TODOS.md §0.2).
