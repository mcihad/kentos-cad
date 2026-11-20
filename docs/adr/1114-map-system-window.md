# 1114 — Şeritteki Harita sistemi sistem listesini doğrudan açar

Tarih: 2026-11-20. gis-usage-report.md GIS-054. Durum: kabul edildi.

İlgili: ADR 0583 §3 (harita sistemi: projenin görünüm ayarı, nesneler dönüşmez).

## Sorun

Harita › Koordinat sistemi › Harita sistemi yalnız durum çubuğundaki koordinat sistemi düğmesine sağ tıklamayı tarif
etti; dar pencerede o düğme durum çubuğuna sığmadığı için gizliydi. Aynı işlem yalnız üstteki kürenin sağ tık
menüsünden yapılabildi; görünüm sistemi ile nesnelerin saklandığı sistem seçicide ayrı görünmüyordu.

## Karar

`view.mapCrs` bir pencere açar: başta “Nesnelerin saklandığı sistem: …” ve “Harita şimdi şu sistemde çiziliyor: …”;
Projenin sistemi, WGS 84 / Pseudo-Mercator ve WGS 84 düğme olarak (seçili olan vurgulu), öbür bütün sistemler
datumlarıyla aranabilir bir listede. Seçim `cad.settings.set`'in `mapSrid`'ini yazar (eskisi gibi), pencere açık kalır
ve yeni durumu söyler; Kapat çizime döner. Durum çubuğunun genişliğinden bağımsızdır; düğmeler ve liste klavyeyle
gezilebilir.

## Kanıt

`apps/desktop/src/tests/map_crs_tests.rs` `the_ribbon_s_map_system_lists_the_systems_at_once`: komut pencereyi açar,
iki sistem ayrı yazılır, Pseudo-Mercator seçilince harita sistemi 3857 olur ve pencere bunu söyler, projenin sistemi
geri getirir. Resim `.run/gui-evidence/cbs-kullanim/gis-054-harita-sistemi.png`.
