# 1109 — Çizim içe aktarımı (NCZ, DXF, DWG) koordinatları projenin sistemine çevirebilir

Tarih: 2026-11-20. gis-usage-report.md GIS-052. Durum: kabul edildi (kısmi).

İlgili: ADR 0222 §2 (CBS dosyalarında “Projenin sistemine çevir”), ADR 0167/0168 (Koordinat dönüştür), CLAUDE.md §5.

## Sorun

TM33 (EPSG:5255) bir CBS projesinde TM39 (EPSG:5257) bildiren NCZ açılınca iki sistem doğru tanındı ama İçe aktar
kapalı kaldı; koordinatları yanlış sisteme atamadan projeye almanın arayüzde yolu yoktu (dosyayı elle yanlış
etiketlemek gerekiyordu).

## Karar

1. Çizim içe aktarma penceresi (NCZ, DXF, DWG) CBS dosyalarındaki gibi “Projenin sistemine çevir” sunar: dosyanın
   sistemi ve projeninki biliniyor ve bir dönüşüm yolu varsa. Seçilince İçe aktar açılır; dönüşümün doğruluğu ve
   yolu var olan CRS satırında, sınırı uyarı satırında söylenir.
2. Taşıma Koordinat dönüştür'ün taşımasıdır (`kentos_formats::reproject`): projeksiyonlar, datum kayması, projenin
   datum seçimleri; kotlar olduğu gibi. Yalnız noktalar ve düz kenarlı çizgi/alanlar tam taşınır; yay, daire, yazı,
   blok ve pafta çerçevesi gibi nesneler yaklaşık taşınmaz, dışarıda kalır ve raporda sayılır (sessiz yaklaşım yok).
3. `file.import.dxf`/`dwg` girdisine `srid` (dosyanın sistemi; yoksa dosyanın bildirdiği) ve `reproject` eklenir:
   komut taşımayı kendisi yapar, betik ve MCP aynı sonucu alır; yerel projede ya da çevrilemeyen sistemde
   `crs_mismatch`. Büyük dosya (karelerle yazılan) ve NCZ (pencerenin kendi yazımı) aynı işlevle önce taşınır.

## Sınır

Eğrilerin, yazıların ve blokların yaklaşık taşınması (yayın ara noktalarıyla yeniden kurulması, yazı açısının yerel
meridyen yakınsamasıyla döndürülmesi) yapılmadı; bunlar şimdilik alınmaz ve sayılır. NCZ içe aktarımı komut değildir;
komut olunca `srid`/`reproject` girdisini o da taşımalıdır.

## Kanıt

`apps/desktop/src/shell/exchange/tests.rs` `an_ncz_in_another_zone_goes_in_moved_when_asked`: TM36 projesine TM39
NCZ çevrilerek alınır; 1284 noktası (421 000, 4 448 000) pyproj'un EPSG:5257 → 5256 dönüşümüyle (676 579.8990742341,
4 449 648.080327207) 1 µm içinde aynı yerde; düz olmayanlar dışarıda; tek geri alma. Var olan “sistemini söyle” testi
değişmeden geçer. Resim `.run/gui-evidence/cbs-kullanim/gis-052-ncz-cevir.png`.
