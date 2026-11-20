# 1087 — Formu olmayan kayıttan, tablonun katmanına form oluşturma

Tarih: 2026-11-20. gis-usage-report.md GIS-026. Durum: kabul edildi.

İlgili: [ADR 0401](0401-forms-core.md), [ADR 0416](0416-form-view-in-attribute-table.md).

## Sorun

Haritada “Park koruma 10 m” etkinken tablodan “Park sınırları”nın ANK-02'si Form görünümünde açıldı. Panel kullanıcıyı
Veri › Formlar › Yeni form'a yönlendirdi; tasarımcı etkin harita katmanını önerdi, oluşan boş form tampona bağlandı
ve ANK-02 yine “Bu katmanın formu yok” dedi.

## Karar

1. Tablonun Form görünümünde formu olmayan katmanın kaydı için panel ““<katman>” için form oluştur” düğmesi sunar:
   tasarımcı tablonun katmanıyla, adı katmanın adıyla yeni form adımında açılır (`features::Event::NewFormFor`);
   Oluştur katmanın alanlarını getirir.
2. Şeritten Yeni form, seçili kaydın katmanını etkin katmandan önce önerir; katman seçicisi görünür kalır.

## Sınama

- `form_scenes::tests::editing::a_formless_layer_makes_its_form_from_the_table`: etkin katman başkayken Parsel'in
  kaydından düğme tasarımcıyı Parsel katmanı ve “Parsel” adıyla açar. Resim `.run/gui-evidence/gis-026/`.
