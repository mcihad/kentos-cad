# 1117 — OGC API Tiles bağlantısı bütün koleksiyonların karo kümelerini sunar

Tarih: 2026-11-20. gis-usage-report.md GIS-058. Durum: kabul edildi.

İlgili: ADR 0208 (harita servisleri); OGC API – Tiles (https://ogcapi.ogc.org/tiles/).

## Sorun

Giriş sayfası iki `tilesets-map` bağlantısı sunan serviste kök adrese bağlanınca yalnız ilk koleksiyonun (Park zemini)
karo kümesi geldi, arayüz “1 katman” dedi; ikinci koleksiyon (Bisiklet yolu) ancak kendi adresine bağlanınca bulundu.
Koleksiyon listesinde karo kümesi olan ilk koleksiyonda arama bitiyordu.

## Karar

Koleksiyonlar belgesinde karo kümesi bağlantısı olan her koleksiyonun karo kümesi listesi sırayla okunur ve birleşik
sunulur. Bir kümenin adı koleksiyonunkiyle birlikte yazılır (“Park zemini — WebMercatorQuad”); ad boşsa ya da
koleksiyonunkiyle aynıysa koleksiyonun adı. Listesi okunamayan koleksiyon ötekileri durdurmaz: sunumun yanında adıyla
söylenir (uyarı); hiçbirininki okunamazsa bağlantı neden okunamadıklarını söyleyerek durur.

## Sınır

Sayfalanmış koleksiyon listesinde (sonraki sayfa bağlantısı) yalnız ilk sayfa okunur.

## Kanıt

`crates/shared/services/src/connect.rs` `collection_tests::every_collection_s_tile_sets_are_offered`: üç koleksiyondan
ikisinin karo kümeleri sunulur (iki seçilebilir katman), okunamayan üçüncü adıyla söylenir.
