# 1085 — Tablonun formu kendi çiziminindir; kaydedilmemiş giriş katman değişimini durdurur

Tarih: 2026-11-20. gis-usage-report.md GIS-022, GIS-023, GIS-024. Durum: kabul edildi (geçişte Kaydet/Vazgeç/Kal seçimi açık).

İlgili: [ADR 0416](0416-form-view-in-attribute-table.md) (tablonun Form görünümü), [ADR 0401](0401-forms-core.md).

## Sorun

Tablonun Form görünümünde ANK-02 (8 bank) açıkken başka bir proje açıldı: yeni projede solda kendi kaydı, sağda önceki
projenin “Park saha kontrolü → ANK-02 → 8” değerleri ve Kaydet kaldı; kopyada Kaydet ancak yazma anında çakışma buldu
(GIS-022). Form taslağı (20 → 17) başka satıra geçişte korunuyor ama Tablo panelinin Katman listesinden başka katmana
geçişte uyarısız siliniyordu (GIS-024).

## Karar

1. Ekrana başka bir çizim gelince (`show_document`) tablonun yerleşik formu ve altında bekleyen formlar kapanır;
   tablonun katmanı, araması, sıralaması ve düzenlenen hücresi yeni çizimde baştan başlar (sütun genişlikleri ve gizli
   sütunlar oturum boyunca kalır). Formda kaydedilmemiş giriş varsa ““<form>” formundaki kaydedilmemiş giriş önceki
   projede kaldı; yazılmadı.” denir (`App::leave_forms_of_document`).
2. Yerleşik formda kaydedilmemiş giriş varken başka katman ya da Tablo görünümü, başka satır gibi durdurulur: uyarı
   söylenir, katman ve taslak kalır (`docked_form_may_leave`).

3. Yerleşik formun sayacı ve okları tablonun gösterdiği satırlardan, tablonun sırasıyladır (GIS-023): arama, Göster,
   ifade süzgeci ya da sıralama değişince açık kayıt gösterilen satırlar arasındaysa yeri yenilenir; değilse
   kaydedilmemiş giriş yoksa ilk gösterilen satıra geçilir, satır yoksa form kapanır; kaydedilmemiş giriş varsa kayıt
   kalır ve kime ait olduğu söylenir (`App::form_follows_rows`). Oklar gizli bir kaydı hiç açmaz.

## Sınama

- `form_scenes::tests::editing::another_drawing_never_shows_the_last_ones_form`: başka çizim açılınca form ve bekleyen
  formlar kapanır, tablo yeni çizimde başlar, kaydedilmemiş giriş söylenir.
- `form_scenes::tests::editing::an_unsaved_entry_stops_another_layer_and_keeps_its_draft`: taslak varken başka katman
  ve Tablo görünümü durdurulur, “Kat 8” taslağı kalır.

- `form_scenes::tests::editing::the_table_form_steps_through_the_rows_shown_only`: sayaç gösterilen satırlarla;
  Göster › Seçililer'de form seçili binaya geçer (1/1), oklar gizli kaydı açmaz.

## Açık

- Geçişten önce Kaydet / Kaydetmeden geç / Düzenlemeye dön seçimini bir pencerede sunmak (bugün uyarı ve kalma).
- Sekmeye geri dönüldüğünde o çizimin formunu yeniden açmak.
