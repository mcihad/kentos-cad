# 1072 — GeoPackage'ın her tablosu kendi koordinat sisteminde

Tarih: 2026-11-20. gis-usage-report.md GIS-006. Durum: kabul edildi (tanımsız sistemli tablonun kaynağını seçmek açık).

İlgili: [ADR 0502](0502-geopackage.md), [ADR 0583](0583-layer-source-crs-and-map-system.md) §1 (katmanın kaynak sistemi).

## Sorun

İki tablolu bir pakette ilk tablo TM33 (EPSG:5255), ikincisi WGS 84 (EPSG:4326) idi. GeoPackage içe aktar penceresi
dosyanın sistemini ilk tablodan okuyup “dosya projenin sisteminde” dedi ve dönüşüm kutusunu göstermedi; yalnız ikinci
tablo seçilince Aktar aynı pencerede EPSG:4326 için dönüşüm gerektiğini söyleyen hatayla durdu. İçe aktarma da
tabloların sistemlerinden yalnız ilkini esas alıyordu.

## Dayanak

- OGC GeoPackage, `gpkg_geometry_columns` ([belgesi](https://www.geopackage.org/spec/#_geometry_columns)): koordinat
  sistemi (`srs_id`) geometri sütunu, yani tablo başına kayıtlıdır; bir dosyada farklı sistemli tablolar olabilir.

## Karar

1. **Pencere.** Her tablonun satırı kendi sistemini söyler (`EPSG:5255`, `sistemi yok`). Dönüşüm kutusu ve özet,
   işaretli tablolardan hesaplanır: projenin sistemindekiler olduğu gibi, sistemi olmayanlar projenin sisteminde
   sayılarak, ötekiler (sistemleri sayılarak) çevrilerek alınır. Kutu “Projenin sistemine çevir (EPSG:4326 →
   EPSG:5255)” der; işaretli tablolar ya da kutu değişince önceki çalıştırmanın iletisi silinir.
2. **İçe aktarma** (`file.import.gpkg`). Her tablo kendi sisteminden çevrilir; projenin sistemindeki ve sistemi
   olmayan tablolara dokunulmaz. `reproject` verilmemişse ret, çevrilmesi gereken tabloları ve sistemlerini adlar.
   Tek sistem varsa yeni katmanlar onu kaynak sistemi olarak kaydeder; birden çok sistem varsa her katman kendi
   tablosununkini (aynı geri alma adımında). Eski “ilki esas alındı” kaybı kalktı.

## Sınama

- `fixtures/geopackage/v1/cift-sistem.gpkg`: raporun iki tablolu paketi (sentetik üç ağaç, A_TM33 ve B_WGS84).
- `gpkg_tests::a_geopackages_tables_each_keep_their_own_system`: iki tablo birlikte; dönüşümsüz bekler, dönüşümle
  ikisi de gelir; WGS 84 ağaçları pyproj 3.7 / PROJ 9.8.1 başvurusunun 1 mm içinde TM33 ağaçlarının üstüne düşer;
  katmanlar 5255 ve 4326 kaydeder.
- `gpkg_tests::the_wgs84_table_alone_asks_for_the_move`: yalnız WGS 84 tablosu dönüşüm ister; resim
  `.run/gui-evidence/gis-006/`.

## Açık

- Sistemi tanımsız bir tablonun kaynak sistemini kullanıcının seçmesi (bugün projenin sisteminde sayılır).
