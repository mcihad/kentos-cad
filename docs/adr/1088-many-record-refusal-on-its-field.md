# 1088 — Toplu form kaydının reddi alanının yanında söylenir

Tarih: 2026-11-20. gis-usage-report.md GIS-027. Durum: kabul edildi.

İlgili: [ADR 0403](0403-form-designer-and-entry-extras.md) §6 (çok kayıtla form).

## Sorun

İki park birlikte seçilip Formla düzenle'de park koduna “ORTAK-24”, bank sayısına 8 yazıldı. Aynı tekil kodun iki
kayda yazılması doğru biçimde reddedildi; ama form açık kalırken park kodu kutusunda açıklama yoktu; durum satırında
kaydın kimliğinin başı görünüyordu, tam gerekçe ve hiçbir şeyin yazılmadığı yalnız geçmişteydi.

## Karar

`cad.forms.fill` bir alanda reddettiğinde (`not_unique` ya da yolu `values.<alan>` olan ret) form, reddi o alanın
sorunu olarak gösterir ve odağı oraya götürür; yazılan öbür değerler kalır. Tekillikte ileti
““Park kodu” 2 kayıtta aynı olamaz: alan katmanda tekil. Hiçbir kayıt değiştirilmedi.”; başka kurallarda çekirdeğin
iletisi kaydın kimliği olmadan ve “Hiçbir kayıt değiştirilmedi.” ile. Bir değer yazılınca ret kalkar.

## Sınama

- `form_scenes::tests::editing::a_unique_value_for_many_records_is_said_on_its_field`: üç binaya tek kod; ret Kod
  alanında “3 kayıtta aynı olamaz … Hiçbir kayıt değiştirilmedi.”, kat taslağı kalır, hiçbir şey yazılmaz; yazınca
  kalkar.
