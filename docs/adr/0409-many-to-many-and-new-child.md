# ADR 0409: Çoktan çoğa ilişki ve formdan yeni alt kayıt

- **Durum:** kabul edildi (2026-10-16). Sahibin isteği (16 Ekim): formları QGIS'in form yönetiminden daha iyi hâle
  getirene kadar sürdürmek.
- **Bağlam belgesi:** ADR 0404 (başka katmandan liste, `$uid` başvurusu, ilişkili kayıtlar), ADR 0405 (ilişkiler ve silme
  politikası), ADR 0408 (çok değerli alan); TODOS.md §16.6 `FORM-M`.
- **Web:** ertelendi (ADR 0228). Web'in tarafı TODOS.md §0.2'de.
- **Sonraya kalan:** geometrisiz tablo katmanı (yeni nesne türü, `.kcad` ve sunucu değişikliği; `FORM-N`).

## Bağlam

- **QGIS:** çoktan çoğa ilişki bir bağlantı tablosuyla kurulur (iki 1:N ilişki); ilişki düzenleyicisi alt kaydı ekler,
  bağlar ve bağını koparır; yeni alt kayıt geometrili katmanda çizim ister.
- KentOS'ta 1:N ilişki bir alanın tek kalıcı kimlik tutmasıyla kuruluyordu (ADR 0404, 0405). Bir malikin birden çok binası,
  bir yolun birden çok mahallesi bağlantı katmanı olmadan yazılamıyordu.

## Karar

### 1. Çok değerli başvuru

ADR 0408'in çok değerli alanı listesiz metin alanında da olur: değerler kendi sırasıyla, birer kez, `; ` ile birleşir
(`check_value`). Formda böyle bir alana başka katmandan liste (`lookup`, `$uid` ile) verilince çoklu seçimle düzenlenir: her
kimlik listede olmalıdır (`form_lookup_missing`), etiketler (ada, kod) kimliklerine çevrilir. Listesiz çok değerli alan tek
ya da çok satırlı metinle de düzenlenir. Bağlantı tablosu gerekmez: alt kayıt üst kayıtların kimliklerini tutar.

İlişkili kayıtlar öğesi alanında kaydın kimliğini tutan nesneleri listeler: tek kimlik ya da kimliklerden biri (bir kimlikte
`;` yoktur).

### 2. Silme politikası çok değerli bağda

Üst nesne silinince alt kaydın listesinden yalnız onun kimliği çıkar; başka üstü kalan alt kayıt `cascade`'de silinmez.
Liste boşalınca `unlink`'te alan boşalır, `cascade`'de alt kayıt silinir. `restrict` kimliği listesinde olan alt kayıt varken
silmeyi reddeder. Formda Bağla kimliği listeye ekler, Bağı kopar yalnız onu çıkarır (`cad.entities.set`).

### 3. Formdan yeni alt kayıt

İlişkili kayıtlar kutusunda **Yeni**: alt katmanda, kaydın yerinde (`$y`, `$x`'in noktası) bir nokta `cad.point.create` ile
oluşur, alanı kaydın kimliğini tutar; alt katmanın formu varsa o kayıt onunla açılır. Kaydın yeri yoksa söylenir. Alt
katmanın kuralları uygulanan formu yeni kaydı reddederse nedeni söylenir.

## Denetimler

- Alanın kuralları (`layer_field_cases.py`: listesiz çok değerli 4 durum), formun kuralları (`forms_cases.py`: 2 ön denetim,
  2 kayıt, 1 yazma; ilişkili kayıtlarda iki binanın sayacı) ve silme politikası (`cad.entities.delete.relations.json`: 5
  çoktan çoğa durumu) KentOS kodu olmadan yazılmış başvurudan, iki uygulamada aynı.
- KCAD: `broken/field-multiple-number.kcad` (çok değerli yalnız metin alanı).
- Python testi (`test_forms.py`), masaüstü akış testi ve resimler (`form_scenes.rs`: `form-malik`, `form-bina-malikler`).

## Sonuçlar

- Çoktan çoğa ilişki iki katmanla, ek tablo olmadan kurulur; silme politikası öbür bağları korur.
- Alt kayıt formun içinden oluşturulup açılır.
