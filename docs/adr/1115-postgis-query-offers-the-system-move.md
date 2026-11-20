# 1115 — PostGIS sorgu katmanına da “Projenin sistemine çevir” sunulur

Tarih: 2026-11-20. gis-usage-report.md GIS-055. Durum: kabul edildi (kısmi).

İlgili: ADR 0582 §3 (sorgu katmanı), ADR 0222 §2.

## Sorun

TM39 projesinde tablo işaretlemeden yalnız EPSG:4326 bir sorgu katmanı eklenince pencere kapandı, geçmişte sistem
farkı ve “reproject verin” önerisi çıktı; aynı form tablo seçilince “Projenin sistemine çevir” kutusunu gösteriyordu.
Kullanıcı SQL'e `ST_Transform` yazmadan sorguyu ekleyemiyordu.

## Karar

Sorgu katmanı yazıldığında (tablo seçilmese de) “Koordinat sistemi: Projenin sistemine çevir” kutusu sunulur; sorgunun
sistemi bağlanınca okunduğu için açıklaması bunu söyler: projeninkinden başkaysa çevirir, aynıysa bir şey değiştirmez.
Kutu `cad.sources.edit`'in var olan `reproject` girdisidir; tablo kendi sisteminde kalır.

## Sınır

Sorgunun geometri türü ve SRID'i pencerede önceden okunmuyor (bağlanırken okunuyor); bağlama hatası pencere kapandıktan
sonra geçmişte söyleniyor, sorgu ve seçimler pencerede kalmıyor. Bunlar sürüyor.

## Kanıt

`apps/desktop/src/shell/exchange/postgis_tests.rs` `a_query_alone_is_offered_the_system_move`: yalnız sorgu
yazılınca kutu görünür ve işaretlenebilir. Resim `.run/gui-evidence/cbs-kullanim/gis-055-sorgu-sistemi.png`.
