# ADR 0432: Taslak durumlar

- **Durum:** kabul edildi (2026-10-17). TODOS.md §16.6 `FORM-AO`; `plan/form.md` FORM-DATA-06, FORM-RULE-18'in yerel kısmı.
- **Bağlam belgesi:** ADR 0402 (kurallar), 0428 (kayıt iş akışı).
- **Web:** ertelendi (ADR 0228). Web'in tarafı TODOS.md §0.2'de.

## Bağlam

Sahada kayıt çoğu zaman eksik başlar: kat sayısı sonra ölçülür, fotoğraf sonra eklenir. Formun zorunlu alanı kaydı yazdırmazsa
kullanıcı ya boş geçmek için uydurma değer yazar ya da kaydı hiç açmaz. Öte yandan eksik kaydın geçerli kayıt gibi kalması da
olmamalıdır.

## Karar

- İş akışının `drafts`'ı: taslak durumlar (durum alanının listesinden, kilitli durum olamaz; `workflow_problem`).
- Taslak durumdaki kayıtta **formun** zorunluluğu (öğenin `required`'ı, `requiredWhen`) uyarıdır: “… zorunlu; taslakta boş
  kalabilir, ilerletmeden önce doldurun.” Kayıt böyle yazılır (Formla kaydet, Kuralları uygula, toplu yazma). **Katmanın** ve
  **alt türün** zorunluluğu veri kümesinin kuralıdır ve taslakta da hatadır.
- Geçerli kayıt isteyen adım (`valid`) kaydı adımın götürdüğü durumda denetler: taslak olmayan duruma giden adım eksik kaydı
  reddeder (`form_invalid`). Böylece taslak, gönderilmiş ve onaylanmış ayrı kalır; geçerli veri kendiliğinden onaylı değildir.
- Formla düzenle'de taslakta boş zorunlu alan varsa üstte “Taslak: n zorunlu alan boş; kayıt böyle kaydedilir” şeridi, alanın
  altında uyarı. Tasarımcının İş akışı bölümünde **Taslak durumlar**.
- Python ikizinde (`forms_cases.py`) iş akışının iki kuralı da: durum alanı salt okunur, yeni kayıt ilk durumla başlar.

## Denetimler

- `forms_cases.py --check`: Formu denetle'de iki, Formla kaydet'te bir durum (`cad.forms.validate.json`, `cad.forms.submit.json`).
- Masaüstü: `a_draft_is_kept_incomplete_and_a_valid_step_refuses_it`; resim `form-taslak`.

## Sonuçlar

- Eksik kayıt dürüstçe “taslak” olarak kalır; ilerlemek tamlık ister.
- Açık: sunucu gerektiren doğrulamada çevrimdışı kaydın bekleyen durumu (FORM-SRC-13), rollere göre taslak (bulut).
