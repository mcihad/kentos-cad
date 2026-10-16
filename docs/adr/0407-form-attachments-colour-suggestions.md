# ADR 0407: Formda ekler, renk ve var olan değerlerden öneri

- **Durum:** kabul edildi (2026-10-16). Sahibin isteği (16 Ekim): formları QGIS'in form yönetiminden daha iyi hâle
  getirene kadar sürdürmek.
- **Bağlam belgesi:** ADR 0225 (nesne ekleri), ADR 0401–0406; TODOS.md §16.6 `FORM-K`.
- **Web:** ertelendi (ADR 0228). Web'in tarafı TODOS.md §0.2'de.
- **Sonraya kalan:** formda fotoğrafın küçük resmi ve çekim zamanı, eklerin yükleme hazırlığı ve bulutta saklanması (FORM-WORK-07, -08).

## Bağlam

QGIS'in düzenleyici türlerinden üçü formlarımızda yoktu:

- **Harici kaynak (External Resource) ve Ek (Attachment):** alana dosya yolu yazılır, fotoğraf formda önizlenir. Bir
  alana tek yol; fotoğrafın zorunlu olduğu söylenemez (yalnız alanın boş olmaması).
- **Renk (Color):** metin alanına `#rrggbb`.
- **Benzersiz değerler (Unique Values), düzenlenebilir:** katmanda alanın var olan değerleri öneri olarak.

KentOS'ta nesnenin ekleri alanlarda değil nesnenin kendisindedir (ADR 0225: gömülü fotoğraf, bağlı dosya, adres, not).

## Karar

### 1. Ekler öğesi (`attachments`)

```json
{ "type": "attachments", "id": "fotograflar", "label": "Fotoğraflar", "accept": "photo", "min": 1, "max": 6 }
```

- Kaydın eklerini formda gösterir: fotoğraflar küçük resimleriyle, belgeler ve adresler adlarıyla; Ekle (dosya ya da
  adres), Aç, Kaldır. Yazma `cad.entities.set`'in `attachments`'ıdır (ADR 0225), formun yazmasıyla aynı adımda değil.
- `accept`: `any` (varsayılan: hepsi), `photo` (yalnız fotoğraflar sayılır, gösterilir ve eklenir) ya da `document`
  (fotoğraf olmayanlar: belgeler ve adresler). Fotoğraf: kitaplıkta gömülü varlık ya da `.jpg`, `.jpeg`, `.png` ile biten
  dosya ve adres (ADR 0225'in kuralı). `min`, `max`: 0–100, `min ≤ max`; öğe gösterilirken kayıt sayıyı tutmazsa hata
  (`form_attachments_few`, `form_attachments_many`). Sınırı olmayan ekler öğesinin denetimi yoktur.
- Sayı yalnız var olan kayıtta denetlenir (Formla düzenle, `cad.forms.validate`, `.submit`, `.fill`, `.audit`): ekler
  kaydın özniteliği değildir, belgenin yazma izlemesi (ADR 0402 §6) onları denetlemez; yeni nesne önce çizilir, eki sonra
  eklenir. Uymayan kayıtları Kayıtları denetle (ADR 0406) söyler.
- Sayıların biçimi formun yapısıdır (`min > max` ya da 100'den büyük: gövde geçersiz, `form_body_invalid`; dosya biçimi de
  reddeder); görünürlük kuralı alanlarınki gibi ön denetimden geçer.

### 2. Renk öğe türü (`color`)

Metin alanlarında. Değer `#RRGGBB` ya da saydamlıkla `#RRGGBBAA`; başkası `form_color_invalid` hatasıdır (yazma
yolunda da: kurallarını uygulayan formun katmanında Python'un yazdığı geçersiz renk reddedilir). Formda rengin örneği ve
seçici (KentOS UI `ColorPicker`).

### 3. Var olan değerlerden öneri (`suggest`)

Metin alanında, kendi değer listesi ya da başka katmandan listesi yokken. Formla düzenle yazdıkça katmanda o alanın var
olan değerlerini önerir; değer serbest kalır (liste değildir). `cad.forms.validate`'in durumunda `suggestions`: katmanın
kayıtlarındaki değerler kırpılarak, en çok kullanılandan, eşitlikte katmandaki ilk görünüş sırasıyla, en çok 20
(`kentos_forms::suggestions_of`). Yazılırken gösterilenler yazılanı içerenlerdir, harfler Türkçe kuralla katlanır (`ı`
“Ihlamur”u bulur), değerin kendisi gösterilmez (`suggestions_for`). Uymayan yerde ön denetim `form_suggest_unfit`.

### 4. Masaüstü

Formla düzenle'de renk örneği seçiciyi açar, yanındaki kutu `#RRGGBB` alır; önerili alan KentOS UI'ın `Suggest`'idir;
ekler öğesi kaydın eklerini adları ve kaynaklarıyla listeler (Aç, Kaldır, Ekle ya da Fotoğraf ekle; fotoğraf görüntüleyici
kapanınca forma dönülür), sayı ve sınırlar başlıkta, eksiklik altında. Önizlemede ve toplu kipte ekler yazılmaz. Formlar
penceresinde Ekler öğesi (Başlık, Ekler: Hepsi, Fotoğraflar, Belgeler ve adresler; En az, En çok; Görünürlük) ve alanın
Öneri seçeneği.

## Denetimler

- Ortak durumlar KentOS kodu olmadan yazılmış başvurudan (`scripts/fixtures/forms_cases.py`): ön denetim (6), kayıt (10),
  `cad.forms.validate` (9: öneriler, fotoğraf ve belge sayıları, renk) ve `cad.forms.audit` (1) durumları; çekirdeğin
  önerilerinin sırası ve katlanması `cases.rs`'te.
- Python testi (`test_forms.py`), masaüstü akış testi ve resimler (`form_scenes.rs`: `form-agac`, `form-agac-eksik`,
  `form-agac-oneri`, `form-agac-tasarim`).
