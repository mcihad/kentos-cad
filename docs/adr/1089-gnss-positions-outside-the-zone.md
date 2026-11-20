# 1089 — GNSS konumu projenin dilimi dışındaysa önizleme söyler

Tarih: 2026-11-20. gis-usage-report.md GIS-034. Durum: kabul edildi.

İlgili: [ADR 0169](0169-field-data.md) (GNSS içe aktarma), [ADR 2500](2500-reproject-preview-shifts-and-reasons.md) §3 (sistemin kullanım alanı).

## Sorun

TUREF / TM30 (EPSG:5254) projesine yaklaşık 35,84° D boylamındaki GPX ve NMEA konumları alındı. Doğu değerleri
~999 658 m oldu; önizleme dönüşümün ±1 m doğruluğunu ve bozuk satırları söyledi ama konumların sistemin dilimi dışında
olduğunu söylemedi. Sayısal dönüşüm yapılabiliyor; eksik olan dilim seçimini değerlendirecek bilgiydi.

## Dayanak

- PROJ `projinfo` ([belgesi](https://proj.org/en/stable/apps/projinfo.html)) ve EPSG veri tabanı: EPSG:5254'ün kullanım
  alanı 28°30'–31°30' D (KentOS'ta `fixtures/crs/v1/areas.json`).

## Karar

GNSS içe aktar önizlemesi dosyanın WGS 84 konumlarını projenin sisteminin kullanım alanıyla karşılaştırır
(`AreaOfUse::holds_near`, kenar payıyla). Dışarıda kalan konum varsa uyarı satırı sayısını, boylam aralığını ve
sistemin dilimini (kayıttaki Türkçe tanımıyla) söyler; bunun dönüşüm doğruluğundan ayrı olduğunu belirtir. Kullanım
alanı bilinmeyen (özel) sistemde dilim uygunluğunun denetlenmediği söylenir.

## Sınama

- `gnss_tests::positions_outside_the_projects_zone_are_said`: örnek GPX TM36'da uyarı vermez, TM30'da verir; dokuz
  konumun hepsi TM30'un alanı dışında. Resim `.run/gui-evidence/gis-034/`.
