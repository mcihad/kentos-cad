# ADR 0434: Formun şeması ve biçim kuralları

- **Durum:** kabul edildi (2026-10-17). TODOS.md §16.6 `FORM-AQ`; `plan/form.md` FORM-STD-01'in formu, FORM-STD-02.
- **Bağlam belgesi:** ADR 0401 (form), 0253 (kanonik JSON ve özet).
- **Web:** gerek yok; şema sözleşmeden üretilir.

## Karar

- Formun biçimi `kentos.form` v1: `ProjectForm`'un JSON Schema'sı (2020-12) sözleşmeden üretilip
  `catalog/v1/forms/form-v1.schema.json`'a yazılır (`kentos_contracts::form_schema`); `$id` biçimin sürümünü taşır, test dosyanın
  güncel olduğunu denetler.
- Biçimin kuralları `docs/specs/forms-v1.md`'de: ne tutulur, yayının özeti, bilinmeyen alanın reddi, varsayılanın yazılmaması,
  uyumluluk ve göç (yeni alan yayımlanmış bir `.kcad` şemasına eklenmez).
- Bağımsız denetim `form_schema_cases.py`: deponun ortak dosyalarındaki 183 form şemaya uyar, bilerek bozulmuş beş form uymaz.

## Sonuçlar

- Dış araçlar formu KentOS kodu olmadan okuyup denetleyebilir.
- Açık: alanları, değer listeleri ve ilişkileriyle birlikte taşınan form paketi (FORM-STD-01'in kalanı, FORM-DESIGN-20).
