# ADR 0760: Alandan çıkınca denetim

- **Durum:** kabul edildi (2026-10-30).
- **İş:** plan/form.md `FORM-RULE-07`'nin kalanı. Değer değişince, kaydederken ve iş akışında ilerlerken sorulan denetimler
  ADR 0438 §2'de vardı; odaktan çıkınca sorulan bir zaman yoktu.
- **Web:** web dururken (ADR 0228) yalnız masaüstü; web'e gerekenler TODOS.md §0.2'de.
- **`.kcad`:** şema değişmez (§3).
- **Bağımlılık:** yeni kitaplık yok.

## Bağlam

Bir alanın denetimi iki zamanda can sıkar:

- Değer değişince sorulursa, kullanıcı daha yazarken hata görür. “Not en az 10 karakter olmalı.” ilk harfte belirir.
- Yalnız kaydederken sorulursa, kullanıcı hatayı çok geç görür.

Arada kalan zaman alandan çıkmaktır: değer yazılıp bitince sorulur.

Başvurulan kaynaklar (30 Ekim 2026'da okundu):

- **ODK, XLSForm `constraint`:** “The `constraint` expression will be evaluated when the user advances to the next screen.”
  ve “Constraints are not evaluated if the response is left blank.”
  <https://docs.getodk.org/form-logic/#validating-and-restricting-responses>
- **MDN, `change` olayı.** İki durumu ayırır. Yazılan öğede: “When the element loses focus after its value was changed: for
  elements where the user's interaction is typing rather than selection, such as a `<textarea>` or the `text`, `search`,
  `url`, `tel`, `email`, or `password` types of the `<input>` element.” Seçilen öğede ise olay seçim anında gelir.
  <https://developer.mozilla.org/en-US/docs/Web/API/HTMLElement/change_event>

## Karar

### 1. Denetimin yeni zamanı: `leave`

Denetimin `on` alanı (`CheckMoment`) dördüncü değeri alır: `leave` (“Alandan çıkınca”). Çekirdekte kural şöyle
(`Prepared::asked`):

| Denetimin zamanı | Değer değişince | Kaydederken | İş akışında ilerlerken |
|---|---|---|---|
| yok | sorulur | sorulur | sorulur |
| `leave` | öğeden çıkıldıysa | sorulur | sorulur |
| `save` | — | sorulur | sorulur |
| `submit` | — | iş akışı yoksa | sorulur |

- Çıkılan öğeler ekranın durumudur (`kentos_forms::Left`). `Prepared::validate_left` ve `assess_left` onu alır.
  `validate_at` ile `assess_at` boş kümeyle aynı yoldan geçer.
- Kaydetme ve iş akışının adımı hiçbir öğeden çıkılmamış olsa da `leave` denetimini sorar. Bu yüzden komutun
  doğrulaması (`cad.forms.validate`, `.submit`, `cad.entities.set`, sunucunun bekçisi) azalmaz: komut her zaman
  kaydetme anındadır.
- ODK'deki gibi boş değerin denetimi sorulmaz. Bu, var olan kuraldır: boş alan zorunluluğa bakar, denetime gitmez.

### 2. Masaüstünde çıkış

- KentOS UI'a odak izleyicisi eklendi: `kentos_ui::widget::focus_watch` (`FocusWatch`).
  - Sardığı parçanın içinde herhangi bir odaklanabilir öğe odak alınca `true`, odak dışarı çıkınca `false` ile haber verir.
  - Metin ve sayı kutusu, seçim kutusu, tarih ve anahtar bu yolla izlenir.
  - iced'in metin girişi odağın bırakılışını bildirmez. Bu yüzden her fare basışı, tuş ve dokunmadan sonra yalnız
    parçanın kendi ağacındaki odaklanabilirler sayılır.
  - Tab odağı bir işlemle taşır, olay üretmez: çıkış tuşun bırakılışında görülür.
  - Setin hazır yolları buna yetmiyordu: `CommandLine::on_focus` komut satırına özgü; `EditCell` çıkışı kendi içinde görür,
    dışarı bildirmez; `focus.rs` yalnız Tab geçişini yapar.
- Formla düzenle'de her alanın denetimi bu izleyiciyle sarılı. Çıkış `Event::Focus(öğe, false)` olarak gelir, öğe
  `Entry::left`'e girer ve kayıt yeniden değerlendirilir. Salt okunur alan ve tasarımcının önizlemesi sarılmaz;
  önizleme zaten her şeyi kaydetme anıyla sorar.
- Seçilen değer (liste, onay kutusu, anahtar, radyo, parçalı seçim, kaydırıcı, renk, ağaç, etiketler) MDN'deki gibi
  seçildiği anda alandan çıkılmış sayılır (`chosen_item`). Yazılan değer (metin, uzun metin, sayı, tarih, aranır liste)
  odağın çıkışını bekler.
- Tasarımcıda denetimin “Ne zaman” seçeneği dört değerlidir: Değer değişince, Alandan çıkınca, Kaydederken,
  İş akışında ilerlerken.

### 3. Dosya ve şema

- **Şema numarası:** formlar `.kcad`'te bütün olarak okunur (şema 72, ADR 0401). Sonraki form eklerinde olduğu gibi
  (ADR 0438–0445) şema numarası artmaz. Bunun bedeli: `leave` taşıyan bir formu daha eski bir KentOS okuyamaz.
- **Güncellenenler:**
  - `tools/kcad/kcad.py` `leave`'i tanır;
  - formun JSON Schema'sı (`catalog/v1/forms/form-v1.schema.json`) ve komut kataloğu sözleşmeden yeniden üretildi;
  - Python SDK'sının `CheckMoment`'i güncellendi.

## Doğrulama

- **Ortak durumlar:** `fixtures/forms/v1/cases.json`'un `moments` grubunda 24 durum.
  - Kapsadıkları: üç zaman, çıkılan ve çıkılmayan öğe, iş akışlı ve iş akışsız form, uyarı, gizli alan, boş değer,
    aralık dışı değer.
  - Kaynak: `scripts/fixtures/forms_cases.py`'nin Python ikizi (`Form.asked`), KentOS kodu olmadan.
  - Rust tarafında `the_checks_are_asked_at_their_moments_as_the_cases_say`.
- **KentOS UI:** `focus_watch` testleri gerçek arayüz ağacında (`kentos_ui::snapshot`) geçiyor.
  - Tıklayınca giriş haber verilir; içeride basmak sessiz kalır.
  - Başka kutuya geçince çıkış ve giriş, boş yere basınca çıkış haber verilir.
  - Tab'ın taşıdığı odak, tuşun bırakılışında görülür.
- **Masaüstü (`forms::entry::left_tests`):**
  - Yazılan Kat ve Not'un denetimi çıkana kadar susar; çıkınca uyarı ve hata belirir.
  - Çıkıldıktan sonra yazılan değerin denetimi o değere cevap verir.
  - Hiç çıkılmayan alanın denetimini Kaydet sorar.
  - Listeden seçilen Durum hemen çıkılmış sayılır, yazılan Kat sayılmaz.
- **Resimler:**
  - `form-cikinca-yazarken`: değerler yazılmış, uyarı yok;
  - `form-cikinca-cikti`: Kat sayısında uyarı, Not'ta hata;
  - `form-cikinca-tasarim`: tasarımcıda “Alandan çıkınca”.
- **Ölçüm:** süre ölçülmedi. Her izleyici yalnız fare basışı, tuş ve dokunmada ve yalnız kendi alanının küçük ağacında
  arar; başka olayda (imleç hareketi, yeniden çizim) hiçbir şey sayılmaz.
