# ADR 0431: Aranır liste ve etiketler

- **Durum:** kabul edildi (2026-10-17). TODOS.md §16.6 `FORM-AN`; `plan/form.md` FORM-DESIGN-07'nin aranır listesi ve çipleri.
- **Bağlam belgesi:** ADR 0401 (form öğeleri), 0407 (öneriler), 0408 (çok değerli alan).
- **Web:** ertelendi (ADR 0228). Web'in tarafı TODOS.md §0.2'de.

## Bağlam

Uzun bir değer listesi (81 il, yüzlerce mahalle) açılır listede kaydırarak seçilmez; çok değerli alanın onay kutuları uzun
listede yer kaplar. QGIS'in Value Map'i yalnız açılır listedir, Value Relation'da arama ayrı bir seçenektir.

## Karar

- `FormWidget`'e iki giriş biçimi: **Aranır liste** (`search`): tek değerli, değer listeli metin ve sayı alanı; yazılan, etiketler
  içinde Türkçe katlamayla aranır (`suggestions_for`), en çok 12 öneri; seçilen etiketin kodu yazılır, yazılan metin değer değildir.
  **Etiketler** (`chips`): değer listeli çok değerli alan; tutulan kodlar çip olarak (× ile çıkar), “Ekle…” listesi kalan kodları
  sunar; kodlar listenin sırasıyla yazılır, listeden kalkmış bir kod sonda kalır.
- Uyum (`widget_fits`): ikisi de değer listesi ister (`form_widget_needs_values`); çipler yalnız çok değerli alanda, aranır liste
  yalnız tek değerlide (`form_widget_unfit`). Bağımsız başvuru `forms_cases.py`'de, `.kcad`'de giriş biçimi adları `kcad.py`'de.

## Denetimler

- `scripts/fixtures/forms_cases.py --check`: beş yeni ön denetim durumu (`fixtures/forms/v1/cases.json`).
- Masaüstü: `a_searched_list_and_chips_write_codes`; resimler `form-aranir-liste` (odakta, öneriler açık), `form-cipler`.

## Sonuçlar

- Uzun listeler yazarak seçilir; çok değerli alan kısa kalır.
- Açık: ağaç biçimli seçim (üst ve alt kodlar; FORM-DESIGN-07'nin kalanı).
