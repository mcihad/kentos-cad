# ADR 0435: Form paketi

- **Durum:** kabul edildi (2026-10-17). TODOS.md §16.6 `FORM-AR`; `plan/form.md` FORM-STD-01'in paketi, FORM-STD-07 ve 08,
  FORM-DESIGN-20'nin yerel kısmı.
- **Bağlam belgesi:** ADR 0401 (form), 0414 (alanın başvuruları), 0430 (değer listeleri), 0434 (formun şeması).
- **Web:** ertelendi (ADR 0228). Web'in tarafı TODOS.md §0.2'de.

## Bağlam

Bir kurumda aynı form birçok projede ve katmanda kullanılır. QGIS'te form katmanın stiline (QML) gömülüdür; başka katmana
taşınınca alan adları uymazsa sessizce bozulur.

## Karar

- **Paket** (`FormPackage`, `kentos.form-package` v1, `.kform` dosyası): formun adı ve gövdesi, gövdenin bağladığı ya da
  kurallarında, notlarında ve iş akışında okuduğu alanlar katmandaki tanımlarıyla (katmanın sırasıyla) ve bu alanların bağlı
  olduğu değer listeleri. Kayıt, bağlantı, yetki ya da başka katmanın verisi taşımaz.
- `cad.forms.export` v1 (sorgu): son yayının (ya da `draft` ile taslağın) paketi; `unknown_form`, `form_unpublished`.
- `cad.forms.import` v1: paket katmanın yeni formu (taslağı) olur. Paketin her alanı `map` ile verilen alana, yoksa aynı
  anahtarlı, yoksa aynı adlı alana bağlanır; türü aynı olmalı (`field_kind_mismatch`). Bağlanınca gövdenin anahtarları, ifade ve
  şablonlardaki adlar, iş akışının alanları ve senaryoların değerleri bir kerede yeniden yazılır (`field_refs::rebind_body`; iki
  alan adlarını değiş tokuş edebilir). Katmanda olmayan alanlar `addFields` ile değer listeleriyle eklenir (projede aynı
  kimlikli liste varsa projeninki kullanılır, farklıysa `domain_kept` uyarısı), yoksa `form_fields_missing`. Alanlar ve
  listeler tek geri alma adımı “Formu içe aktar”; form projenin ayarıdır (`cad.forms.edit` gibi). Cevap: formun kimliği,
  bağlar, eklenen alanlar ve listeler, taslağın ön denetim hata sayısı.
- Tasarımcıda **Dışa aktar…** (seçili formun taslağı `.kform`'a) ve **İçe aktar…** (seçili formun katmanına, yoksa etkin
  katmana; alanlar anahtar ve adla bağlanır, eksikler eklenir; yeni form seçilir).

## Denetimler

- `scripts/fixtures/forms_package_cases.py --check`: dışa aktarmanın 20, içe aktarmanın 28 durumu, KentOS kodu olmadan (alanların
  yerleri ve ifadelerde yeniden adlandırma `field_usage_cases.py`'den).
- Python: `test_a_form_goes_to_another_layer_as_a_package`.
- Masaüstü: `a_form_package_goes_to_another_layer_with_its_fields`; resim `form-paket`.

## Sonuçlar

- Bir form dosyayla başka projeye ve katmana taşınır; adlar uymazsa eşlemeyle bağlanır, sessizce bozulmaz.
- Açık: içe aktarırken eşleme penceresi (bugün anahtar ve adla, eksikler eklenerek), kurumun form kitaplığı (bulut).
