# 1099 — Shapefile arşivinin katmanları birlikte alınır

Tarih: 2026-11-20. gis-usage-report.md GIS-042. Durum: kabul edildi.

İlgili: ADR 0222 (Shapefile/GeoJSON/KML içe aktarma), ADR 0563 (içe aktarma komutları), CLAUDE.md §5 (kaynak CRS).

## Sorun

Dört katmanlı park envanteri “Tek arşiv” ile yazılıp boş projeye alınınca pencere “Bütün katmanlar (1)” gösterdi ve
yalnız ilk katmanı aldı; öbür üç katman için arşiv üç kez daha açıldı.

## Karar

1. `file.import.shp` zaten `entry` verilmeyince arşivin bütün katmanlarını okuyup birleştiriyordu; pencere ise bir
   seferde birini okuyordu. Pencere artık arşivin bütün katmanlarını okur (komutun birleştirmesiyle aynı:
   `file_import_gis::merge`), her katmanı nesne sayısı, `.prj`'sinin söylediği sistem ve gideceği katmanla listeler;
   İçe aktar hepsini tek geri alma adımında alır (komut `entry`siz).
2. Okunamayan üye adıyla ve nedeniyle rapora yazılır (“… okunamadı; alınmaz, öbür katmanlar alınır”) ve dışarıda
   kalır; hiçbiri okunamazsa ilk üyenin hatası söylenir.
3. **Bir seferde biri** yalnız iki durumda sürer: iki katmanın adı aynıysa (ayrı klasörlerde; komut yolunu ister) ya
   da katmanların `.prj`'leri farklı sistem söylüyorsa — tek bir sistem cevabı bazılarını yanlış yere koyardı, sessiz
   varsayım yapılmaz. O zaman eski “Arşivdeki katman” seçimi nedeniyle birlikte görünür.

## Sınır

Birleşik sonucun dosya satırı (şekil türü, kayıt sayısı) ilk katmanınkidir; katmanların listesi tabloda ve dosya
satırının “Katmanlar” kısmındadır.

## Kanıt

`apps/desktop/src/shell/exchange/gis_tests.rs` `an_archive_s_layers_come_in_together`: aktarma örneği tek arşive
yazılır (beş katman), boş TUREF/TM36 projesinde beşi EPSG:5256 ile listelenir, İçe aktar yedi nesneyi tek adımda alır,
komut girdisinde `entry` yok, tek geri alma hepsini geri alır; farklı sistemli `katmanlar.zip` yine tek tek. Resim
`.run/gui-evidence/cbs-kullanim/gis-042-zip-katmanlar.png`.
