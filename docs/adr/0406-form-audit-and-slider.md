# ADR 0406: Kayıtları formla denetleme ve kaydırıcı

- **Durum:** kabul edildi (2026-10-16). Sahibin isteği (16 Ekim): formları QGIS'in form yönetiminden daha iyi hâle
  getirene kadar sürdürmek.
- **Bağlam belgesi:** ADR 0401–0405; `plan/form.md` FORM-DATA-24; TODOS.md §16.6 `FORM-J`.
- **Web:** ertelendi (ADR 0228). Web'in tarafı TODOS.md §0.2'de.

## Bağlam

Form, bir kaydı açarken ya da yazarken denetliyor (ADR 0401, 0402). Ama katmanda form yazılmadan önce gelen, içe aktarılan
ya da form değiştikten sonra kurallarına artık uymayan kayıtlar var: bunlar ancak tek tek açılınca görünüyor.

- **QGIS:** alan kısıtları (Constraints) yalnız düzenleme sırasında; katmanın bütününü denetleyen araç yok, ifadeyle seçmek
  (`not (kural)`) elle kuruluyor. Kaydırıcı (Range › Slider/Dial) sayısal alanların düzenleyici türüdür.
- **ArcGIS:** Validate Features: seçili ya da bütün nesneleri öznitelik kurallarına göre denetler, uymayanları işaretler.

## Karar

### 1. `cad.forms.audit` (salt okunur)

Girdi `{ form, draft?, warnings?, limit?, moment?, user? }`. Formun katmanındaki her kayıt, belgede durduğu gibi hesaplarıyla
tamamlanır (varsayılan verilmez: kayıt vardır) ve formla yazılsa nasıl değerlendirilecekse öyle değerlendirilir: zorunlu
alan, değerin kuralı, süzülen ve başka katmandan gelen seçenekler, denetimler, tekil alanlar. Tekil alanların değerleri
katmanda bir kez sayılır; yalnız değeri birden çok kayıtta olanın sorunu aranır (100 000 kayıtta doğrusal).

Çıktı `{ version?, checked, failing, warned, records: [{ uid, issues }], more? }`: sayılar bütün kayıtlarındır; liste
hatası olanlar (`warnings` ile yalnız uyarısı olanlar da), katmanın sırasıyla, en çok `limit` (varsayılan 500, 1–10 000;
dışı `invalid_count`), fazlası `more`. Retler `cad.forms.validate`'inkiler: `invalid_moment`, `invalid_user`,
`unknown_form`, `form_unpublished`, `form_preflight`. Hiçbir şey yazmaz; düzeltme formla düzenleyerek, `cad.forms.fill` ya
da `cad.forms.submit` iledir (sessiz düzeltme yok, FORM-DATA-24).

### 2. Kaydırıcı öğe türü (`slider`)

Tam ve ondalık sayı alanlarında, alanın en az ve en çok değeri varsa. Aralığı olmayan alanda ön denetim
`form_widget_needs_range` hatası verir. Adım tam sayıda 1, ondalıkta alanın basamağı (yoksa 2); değer kaydırıcının yanında
yazılı. Değer alanın kuralıyla yazılır, kaydırıcı bir denetim değildir.

### 3. Masaüstü

**Kayıtları denetle** (`data.formAudit`; Veri › Tablo, Yönet › Form, Formlar penceresinin alt satırı): sayılar, uyarıları
gösterme anahtarı, kayıt başına ilk sorun ve kalan sayısı, Git (çizimde göster) ve Aç (Formla düzenle), Hepsini seç,
Yeniden denetle. Kaydedilmemiş tasarım varken Formlar penceresi önce kaydetmeyi ister.

## Denetimler

- Ortak durumlar KentOS kodu olmadan yazılmış başvurudan (`scripts/fixtures/forms_cases.py`): `cad.forms.audit` (20 durum:
  hatalılar, uyarılar, sınır ve `more`, an ve kullanıcı, gizlenenleri silen form, taslak, kuralları uygulanan form, tekil
  olmayan alan, yumuşatılan denetim, boş katman, başka katmandan liste, retler); ön denetimde kaydırıcının uyumu ve aralığı.
- Python testi (`test_forms.py`), masaüstü akış testi ve resimler (`form_scenes.rs`: `form-kayit-denetle`, `form-kaydirici`).

## Sonuçlar

- Formun kuralları katmanın bütününe uygulanabilir; uymayan eski veri raporlanır, sessizce düzeltilmez.
- Sınırlı sayısal alanlar sürüklenerek girilir.
