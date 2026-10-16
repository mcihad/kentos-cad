# ADR 0401: Formlar: çekirdek, alan kimliği, komutlar ve yazma profili

- **Durum:** kabul edildi (2026-10-16). Sahibin isteği (16 Ekim):
  - formlar CBS için çok önemli; madde madde değil, grup grup yapılacak;
  - yeni dalda (`forms-core`) yapılacak;
  - form ADR'leri 0400'den başlar.
- **Bağlam belgesi:**
  - `plan/form.md`: FORM-FIRST, FORM-STD (§4), FORM-RULE (§8), FORM-API (§14); bu ADR FM1'in (ilk çalışan yerel form)
    yerel ve masaüstü kısmıdır;
  - ADR 0199 (katman alanları ve tek biçimleri);
  - ADR 0100, 0214 (ifade dili);
  - ADR 0253 (komut protokolü);
  - ADR 0400 (form elemanları ve klavye).
- **Web:** ertelendi (ADR 0228). Web'in tarafı TODOS.md §0.2'de.
- **Sonraya kalan:** FM2 (merkezi yayın, `forms.design`, `forms.publish`, `forms.use` izinleri, sunucunun yazma
  bekçisi), ilişkiler ve ana–ayrıntı, hesaplanan alanlar, dış kaynak, mobil.

## Bağlam

- Katmanın alanları (ADR 0199) değerlerin kuralını tutar ama bir veri giriş ekranı yoktu. Öznitelikler ve Öznitelik
  tablosu alanları düz bir liste olarak gösterir; sekme, grup, koşullu görünürlük ve zorunluluk, kayıt denetimi yoktu.
- Plan (`plan/form.md` §4.2), formun alana **adıyla değil kararlı kimliğiyle** bağlanmasını ister: alan yeniden
  adlandırılınca form bozulmamalı.
- Plan (§8, FORM-RULE-02, -14), kuralın bütün yazma yollarında geçerli olmasını ister. Formu atlayan ham öznitelik
  komutu kuralı delemez.

## Karar

### 1. Bir form

Formun tek tanımı sözleşmededir (`kentos_contracts::forms`), adı `kentos.forms/1 core`:

- **Form** (`ProjectForm`): kimlik, ad, katman, **taslak** (tasarımcının düzenlediği) ve **yayınlar**. Veri girişi
  son yayını kullanır. `enforce` yayının kurallarını katmanın yazma profili yapar (§8).
- **Gövde** (`FormBody`): öğeler ve taslağın **sürümü**. Her kaydedilen değişiklik sürümü bir artırır.
- **Öğeler** (`FormItem`, `type` etiketiyle):
  - `tab`: yalnız üstte; üstte sekme varsa hepsi sekmedir;
  - `group`: ad, bir ya da iki sütun, `visibleWhen`;
  - `field`: alanın **anahtarı** (`field`), etiket, yardım, yer tutucu, öğe türü (`widget`), `readOnly`, `required`,
    `visibleWhen`, `requiredWhen`, `readOnlyWhen`, varsayılan, denetimler (`checks`: kural ve ileti);
  - `note`: düz yazı.
- **Yayın** (`FormRelease`): sürüm (1, 2, …), gövdenin kanonik JSON'unun (ADR 0253 §2) SHA-256'sının ilk 32 onaltılık
  hanesi, gövdenin kopyası ve not. Yayın değişmez; en çok 50 tutulur, fazlası en eskisini düşürür.
- **Yapı kuralları** (`body_problem`, `forms_problem`): öğe kimlikleri tekil (harf, rakam, `-`, `_`, `.`), en çok 4 düzey
  ve 500 öğe, adlar 1–64 karakter, metinler 1000, kurallar 2000 karakter, en çok 20 denetim, en çok 100 form.
- **Öğe türleri ve alan türleri:**

| Alan türü | Öğe türleri |
|---|---|
| Metin | tek satır, çok satır; değer listesiyle açılır liste, seçenek düğmeleri, parçalı seçim |
| Tam sayı, ondalık | sayı; değer listesiyle açılır liste, seçenek düğmeleri, parçalı seçim |
| Tarih | tarih |
| Evet/hayır | onay kutusu, anahtar, seçenek düğmeleri, parçalı seçim (iki değeri liste sayılır) |

Öğe türü verilmezse alanın türünden gelir: değer listesi açılır liste, sayı sayı, tarih tarih, evet/hayır onay kutusu,
200 karakterden uzun metin çok satır.

### 2. Alanın kararlı kimliği

- `LayerField`'ın yeni `id`'si vardır. Alanın **anahtarı** kimliği, yoksa adıdır (`LayerField::key`). Form alanı
  anahtarıyla bağlar. Katmanın anahtarları tekildir (`fields_problem`).
- **Kimlik kendiliğinden yazılır, sessizce değil:** belge (`Document::set_layer_fields`) alanların listesi değişirken
  her alanın önceki kimliğini (aynı ad ya da yeniden adlandırmanın kaynağı) taşır. Katmanın bir formunun adıyla
  bağladığı alan yeniden adlandırılınca eski adı kimliği olur; form onu bulmaya devam eder.
- Eski adı yeni bir alan alırsa, o yeni alan taze bir kimlik alır (ad ve sayı: `Kat-2`).
- **Formu olmayan çizimde hiçbir alan kimlik almaz.** Böylece dosya eski şemada kalır, eski ortak durumlar değişmez.

### 3. Dosya: `.kcad` şema 60

- Ayarların `forms`'u (sözleşmenin JSON'u, opak değerler gibi, varsayılan alanı yazılmaz) ve alanın `id`'si.
- Yazıcı 60'ı yalnız formu ya da kimlikli alanı olan çizimde yazar; şema 59 ve öncesinde bu anahtarlar `unknown_field`'dır.
- Okuyucu formları `forms_problem` ile denetler; özeti içeriğine uymayan yayın `bad_value`'dur.
- Spesifikasyon `docs/specs/kcad-v2.md` §6.4.13; bağımsız Python okuyucusu ve yazıcısı aynı kuralla.
- Örnek `fixtures/kcad/v2/forms.kcad`; bozuk örnekler `broken/forms-*`, `broken/field-id-*`.

### 4. Ön denetim (`kentos_forms::preflight`)

Formun kullanılabilir olup olmadığını kayda dokunmadan söyler. Tasarımcı her değişiklikte gösterir; yayınlama ve
formla kaydetme hatası olan formu reddeder. Kaydetme reddetmez: sorunlu taslak saklanır.

| Kod | Ağırlık | Anlamı |
|---|---|---|
| `form_body_invalid` | hata | yapı kurallarına uymuyor (tek sorun olarak) |
| `form_field_unknown` | hata | anahtar katmanda yok (kuralları sorulmaz) |
| `form_field_twice` | hata | bir alan iki öğede |
| `form_widget_unfit` | hata | öğe türü alanın türünü düzenleyemez |
| `form_widget_needs_values` | hata | liste öğesi değer listesi ister |
| `form_default_invalid` | hata | varsayılan alanın kuralına uymuyor |
| `form_required_read_only` | hata | zorunlu ve salt okunur alan varsayılansız |
| `form_rule_invalid` | hata | kural ya da denetim derlenmiyor |
| `form_required_hidden` | hata | katmanın zorunlu alanı formda yok, ya da bir kural onu gizleyebilir ya da kilitleyebilir (grubun kuralı da sayılır) |
| `form_rule_unknown_name` | uyarı | kural katmanda olmayan bir adı okuyor |
| `form_rule_reads_itself` | uyarı | alanın görünürlüğü ya da kilidi alanın kendi değerini okuyor |
| `form_rule_unknown_variable` | uyarı | projede olmayan `@` değeri |
| `form_empty` | uyarı | formda alan yok |

`form_required_hidden`, planın FORM-RULE-04'üdür. Kullanıcı dolduramadığı zorunlu alanla kilitlenmez.

### 5. Kurallar

- Kurallar ifade dilindedir (İfadeyle seç'in dili), katmanın alanları **türleriyle** okunur.
- **Motorun düzeltmesi:** kullanıcının tanımladığı alan önceden metin olarak okunuyordu:
  - `Onaylı = doğru` evet/hayır alanında hiç doğru olmuyordu; oysa İfade oluşturucu tam bu örneği önerir;
  - `Kat + 1` birleştiriyordu.

  Artık çözümleyici tipli alanı türüyle okur (`resolve.rs`):
  - evet/hayır alanı `true` için doğru, `false` için yanlış, başka her şey için boştur (tipli değer veren ev sahibininki de);
  - sayı alanı sayıdır, sayı değilse boştur.

  Bu, katman süzgeci, etiket ve İşlemler gibi tipli şemayla derleyen her yerde aynıdır.
- `@` değerleri projenin değişkenleri ve an ile kullanıcıya bağlı olmayan yerleşiklerdir. `@tarih`, `@şimdi` ve
  `@kullanıcı` form kurallarında yoktur: kuralın cevabı nerede sorulursa sorulsun aynıdır (plan §4.3). Oturumun anı
  ileri bir profildir. `$uzunluk` ve `$alan` bu profilde boştur.

### 6. Bir kayıt (`kentos_forms::Prepared`)

- **Gösterilen:** öğenin kuralı ve onu içeren her grubun kuralı tutuyorsa. Sekme ve not her zaman gösterilir.
- **Zorunlu:** öğe, katman alanı ya da `requiredWhen` öyle diyorsa, **yalnız gösterilirken**.
- **Salt okunur:** öğe ya da `readOnlyWhen` öyle diyorsa.
- **Geçerli kayıt:** gösterilen her alan katman alanının kuralına uyar (ADR 0199'un tek biçimleri), gösterilen zorunlu
  alan doludur, gösterilen ve dolu alanın denetimleri tutar.
- **Gizli alan sorulmaz ve değerini korur** (FORM-RULE-03). Görünürlük veriyi silmez, güvenlik izni de değildir.
- **Boş, öznitelik yoktur.** Öznitelik deposu boş metin ile değersizi ayıramaz (plan FORM-FIRST-04): bu profil ikisini
  aynı sayar ve öyle söyler.
- **Formla yazma** (`apply`, `assess`):
  - yalnız formun gösterdiği alanlara;
  - değerler alan anahtarlarıyla, tek biçime çevrilir;
  - sonrasında gizli ya da salt okunur olan alana değişiklik yazılmaz (FORM-RULE-14);
  - sonra kaydın bütünü denetlenir;
  - bütün kayıt ya da hiçbiri.
- **Varsayılan** yalnız istenince ve boş alana yazılır (yeni kayıt; plan §4.3).
- Bağımsız başvuru `scripts/fixtures/forms_cases.py` (ifade dilinin bağımsız değerlendiricisiyle), ortak durumlar
  `fixtures/forms/v1/cases.json`. Rust testi `crates/shared/forms/tests/all/cases.rs`.

### 7. Katmandan form

`cad.forms.edit`'in `create`'i gövde verilmezse katmanın alanlarından üretir:
- her alan bir öğe, katmanın sırasıyla;
- öğe kimliği adın Türkçe küçük harfi (`İl adı` → `il-adı`, çakışınca `-2`);
- katmanın zorunlu alanı formda da zorunlu.

### 8. Yazma profili

- `enforce` açık formun son yayını, katmanın her öznitelik yazmasının profilidir (FORM-RULE-02).
- `cad.entities.set` (Öznitelikler, Öznitelik tablosu, Şablonu uygula, Python, ajan) katmanın bir alanını değiştiren
  her yazmada sorar:
  - formun göstermediği alan yazılmaz;
  - gizli ya da salt okunur alan değişmez;
  - kayıt geçerli kalır.
  Uymazsa `form_rules`.
- Katmanın alanı olmayan özniteliğin yazılması formun işi değildir.
- Katmanın alanlarına artık uymayan yayın (alan silindi ya da anahtarı değişti) bütün yazmaları reddeder; form yeniden
  yayınlanmalıdır.
- Katmanın kendi kuralları (ADR 0199) formdan önce söylenir (`attribute_required`, `invalid_attribute`).
- Nesne oluşturma ve geometri düzenlemeleri bu profilde sorulmaz; ileri bir dilimdir (TODOS.md).

### 9. Komutlar

| Komut | Ne yapar | Geri alma |
|---|---|---|
| `cad.forms.edit` | `create` (katmanın alanlarından ya da verilen gövdeyle; kimlik verilmezse addan), `update` (taslağın sürümüyle; eski sürüme göre hazırlanan `form_draft_changed`, conflict), `rename`, `remove`, `publish` (ön denetimde hata varsa `form_preflight`; değişmemiş taslak yeniden yayınlanmaz), `enforce` (yayınsız `form_unpublished`) | proje ayarı, adım değil (adlı görünümler gibi) |
| `cad.forms.validate` | ön denetim; `uid` ile kaydın öğeleri, sorunları ve değerler yazılmış gibi öznitelikleri; `draft`, `defaults` | salt okunur |
| `cad.forms.submit` | kaydı formla yazar; ilk sorun `form_invalid`, path onu veren değer | tek adım: “Form: <ad>” |

- Retler, sırası ve iletileri sözleşmenin (`cad_forms.rs`) ve `codes.rs`'in.
- Ortak durumlar `fixtures/commands/v1/cad.forms.{edit,validate,submit}.json` ve `cad.entities.set.forms.json`
  (her biri en az 20), bağımsız başvurudan.
- Python `kentos.cad.forms` ve `Document.forms`; MCP'de üç araç, `edit` ve `submit` yıkıcı listede.

### 10. Masaüstü

- **Formlar** (`data.forms`, `forms/designer.rs`, görünümü `designer_view.rs`):
  - solda projenin formları (katmanı, son yayını, uygulanıp uygulanmadığı), Yeni form (ad ve katman; gövde katmanın
    alanlarından) ve formda olmayan katman alanları: birine basmak onu seçili öğenin yerine ekler;
  - ortada öğe ağacı (sekme, grup, alan, not; kurallı öğede ƒ, hatalı öğede uyarı işareti), araç çubuğu (Sekme, Grup, Not,
    Yukarı, Aşağı, Öğeyi sil) ve ön denetimin sorunları (sorun basılınca öğesi seçilir);
  - sağda seçili öğenin denetçisi: etiket, öğe türü (alana uyanlar, uymayan “alana uymuyor” diye), Zorunlu, Salt okunur, yardım,
    yer tutucu, varsayılan, üç kural ve denetimler; her kuralın yanında ε İfade oluşturucuyu açar (katmanın alanları türleriyle);
  - altta ad, taslağın ve son yayının sürümü, Kuralları uygula; Formu sil, Vazgeç, Yayınla, Kaydet.
- **Kaydet** taslağın kopyasını (`update`, ad değiştiyse `rename`) yazar ve **pencereyi kapatır**; Yayınla yazar ve yayınlar,
  pencere açık kalır; Vazgeç kopyayı bırakır ve kapatır. Kaydedilmemiş kopya varken başka form seçilmez.
- **Formla düzenle** (`data.formEdit`, `forms/entry.rs`): tek seçili nesnenin katmanının ilk yayınlanmış formu (yoksa taslağı):
  - sekmeler (sorun sayısıyla), grup başlıklı kutuda, iki sütunlu grup yan yana; notlar düz yazı;
  - öğe türüne göre denetimler: tek satır, çok satır (metin düzenleyici), sayı, tarih (takvimli), onay kutusu, anahtar, açılır
    liste, seçenek düğmeleri, parçalı seçim; salt okunur alan değerini gösterir;
  - her değişiklikte çekirdek (`assess`, `states`) kuralları yeniden sorar; sorun alanın altında, alana dokunulunca ya da
    Kaydet'ten sonra; İlk soruna git sekmeyi değiştirir ve alana odaklanır;
  - Kaydet yalnız değişen değerleri `cad.forms.submit` ile yazar (formun alanı olmayan kayıt yeni sayılır: varsayılanlar) ve
    pencereyi kapatır; olmazsa pencere ilk sorunda kalır. Tab alanlar arasında döner (ADR 0400).
- **Girişler:** CBS'de Veri › Tablo (Formlar…, Formla düzenle), CAD'de Yönet › Form; Öznitelikler'in Alanlar bölümünde Form satırı (Formla düzenle,
  Formlar…); katmanın sağ tıkında Formlar… (katmanın formu, yoksa onun için yeni form). Komutların arayüz kaydı
  `catalog/v1/additions.json`'da, manifest satırları `catalog/v1/operations.json`'da (düzey `semantic`).
- **Kanıt:** iz `fixtures/interaction/v1/form-entry.json` (çizimi `scripts/fixtures/forms_scene.py`); resimler
  `form_scenes.rs` (`KENTOS_SHOTS_ONLY=form-…`); akış testleri `form_scenes::tests`, `designer::tests`.

## Denetimler

- `cargo test -p kentos-forms`: 48 ortak çekirdek durumu.
- `cargo test -p kentos-native-application`: komut durumları ve yetenekler.
- `cargo test -p kentos-kcad`; `python3 scripts/fixtures/kcad_v2_reference.py --check`.
- `cargo test -p kentos-domain` (`field_ids`), `cargo test -p kentos-expression --test typed`.
- `python3 scripts/fixtures/forms_cases.py --check`.
- `pnpm py:test` (`test_forms.py`), `cargo test -p kentos-mcp` (`an_agent_makes_publishes_and_uses_a_form`).
- `KENTOS_TRACES_ONLY=form-entry cargo test -p kentos-desktop every_trace_passes`; `cargo test -p kentos-desktop form_scenes`;
  `python3 scripts/fixtures/forms_scene.py --check`.

## Sonuçlar

- Formsuz çizimler eskisi gibi yazılır (şema 2–59). Formu olan çizim şema 60'tır; eski sürüm onu açmaz, sessizce formunu
  düşürmez.
- Evet/hayır ve sayı alanları her ifadede türüyle okunur: daha önce yanlış sonuç veren kurallar artık doğru sonuç verir.
- Formun kuralları yalnız formda değil, öznitelik yazan her komutta geçerlidir (`enforce` açıkken).
