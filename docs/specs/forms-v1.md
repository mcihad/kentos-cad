# KentOS formu, biçim 1 (`kentos.form` v1)

Güncelleme: 17 Ekim 2026. Karar: [ADR 0434](../adr/0434-form-schema.md). Şema:
[`catalog/v1/forms/form-v1.schema.json`](../../catalog/v1/forms/form-v1.schema.json) (JSON Schema 2020-12, sözleşmeden üretilir).

## 1. Ne tutulur

Projenin her formu (`ProjectForm`) projenin ayarındadır (`settings.forms`; `.kcad` şema 60'ta JSON olarak):

| Alan | Anlamı |
|---|---|
| `id` | Formun kimliği: 1–64 harf, rakam, `-`, `_`, `.` |
| `name` | Adı, 1–64 karakter |
| `layer` | Bağlı olduğu katmanın kimliği |
| `draft` | Taslağın gövdesi (`FormBody`): `revision`, `items`, isteğe bağlı `clearHidden`, `tests`, `workflow`, `steps`, `translations` |
| `releases` | Yayınlar, eskiden yeniye: `version` (1, 2, …), `digest`, `body`, isteğe bağlı `note` |
| `enforce` | Son yayın katmanın yazma kuralıdır (yalnız `true` yazılır) |
| `comments` | Gözden geçirenlerin yorumları (ADR 0441 §5): `id` (1, 2, …), isteğe bağlı `item`, `author`, `at`, `text`, `resolved`; en çok 200 |

Öğeler (`items`) `type` ile ayrılır: `tab`, `group`, `field`, `note`, `related`, `attachments`, `location`, `divider`, `action`.
Alan öğesi katmanın alanını anahtarıyla (alanın `id`'si, yoksa adı) anar. Eylem (`action`) kayıtlı bir işlem aracını yalnız
kimliğiyle anar (`tool`, `attributes.calculate` gibi noktalı parçalar); adres, sorgu ya da betik bir araç kimliği değildir ve
şema onu reddeder. Eylemin türü `kind` (`tool` varsayılan, `recompute`, `step` ile adımın adı `step`, `show`) ve araç eyleminin
çalıştığı yer `runs` (`auto`, `here`, `background`; ADR 0442 §5, §6). İş akışının `notes`'u adımların alan notlarını tutan metin alanıdır
(ADR 0442 §4). Grubun `across`'u iki sütunu satır satır doldurur (ADR 0443 §6); alanın `redact`'ı değerini kaydın örneklerinden
(komut geçmişinin Python'u, tekrar oynatma girdisi) gizler (ADR 0443 §2).

## 2. Özet

Yayının `digest`'i gövdesinin kanonik JSON'unun (anahtarlar UTF-8 baytlarına göre, boşluksuz, sayılar en kısa yazımla; ADR 0253
§2) SHA-256'sının ilk 32 onaltılık hanesidir. Okuyucu yayının özetini gövdeden yeniden hesaplar; tutmayan yayın reddedilir.
Yayın değişmez: taslak değişir, yayın yeni sürümdür.

## 3. Bilinmeyen ve varsayılan

- Bilinmeyen alan reddedilir (`deny_unknown_fields`); okuyucu sessizce atmaz.
- Varsayılan değerindeki alan yazılmaz (`false`, boş liste, yok). `.kcad` okuyucusu formu yazıcının biçiminde değilse reddeder.

## 4. Uyumluluk ve göç

- Biçimin sürümü şemanın `$id`'sindedir (`…/kentos.form/v1.json`). Yeni bir alan ya da öğe türü, `.kcad`'in yayımlanmış bir
  şemasına eklenemez: yeni `.kcad` şeması ve gerekiyorsa formun yeni biçim sürümü gelir; eski okuyucu yeni dosyayı bilinmeyen alanla
  reddeder (sessiz kayıp yok).
- Bir alanın anlamı değişmez; anlam değişecekse yeni adla gelir ve eski ad göçle çevrilir (göç yazıcının değil okuyucunun işidir).
- Formun alana bağı alanın kararlı `id`'siyledir (ADR 0401 §2): alan yeniden adlandırılınca form kırılmaz.

## 5. Profil: formun ortamdan istedikleri

Bir formu gösteren her ortam (masaüstü, ileride web ve mobil, Python) aynı biçimi okur ama her ortam her şeyi sağlamaz. Formun
ortamdan istedikleri gövdesinden hesaplanır (`kentos_forms::requirements`, `cad.forms.validate`'in `requires`'ı; ADR 0441 §4):

| Anahtar | Ne zaman |
|---|---|
| `moment` | Kurallardan biri `@tarih` ya da `@simdi` okur |
| `user` | Kurallardan biri `@kullanici` okur |
| `layers` | Başka katmandan liste ya da ilişkili kayıtlar öğesi var |
| `geometry` | Konum öğesi, mekânsal süzgeçli liste ya da kaydı gösteren eylem var |
| `attachments` | Ekler öğesi var |
| `workflow` | Kaydın iş akışı var |
| `tool:<kimlik>` | Eylem öğesi o işlem aracını açar |

Hiçbiri yoksa form **saftır**: yalnız kaydın değerleriyle çalışır, her ortamda aynı açılır.

- Bir ortam formun istediklerinden birini sağlamıyorsa formu **salt okunur** açar ve hangisinin eksik olduğunu söyler; kaydı
  eksik kurallarla yazmaz, eksik öğeyi sessizce gizlemez (`kentos_forms::unsupported`). Eylemin aracı ortamda yoksa düğme kapalı
  görünür ve tasarımcının ön denetimi uyarır (`form_action_unknown`).
- Masaüstü bütün anahtarları sağlar. Tasarımcı formun istediklerini öğe seçilmemişken gösterir.
- Erişilebilirlik: her alan öğesi etiketini taşır (öğenin `label`'ı, yoksa alanın adı ya da takma adı); yardım metni (`help`)
  alanın açıklamasıdır; odak sırası formdaki sıradır (Tab; iki sütunlu grupta `across` ile satır satır), hata alanının altında ve
  özetinde; İlk soruna git ve Sonraki sorun hataları formun sırasıyla dolaşır. Renkler temanın jetonlarındandır: form renk yazmaz.
- Yerelleştirme (ADR 0443 §5): formun dili projenin dilidir (Türkçe); `translations` başka dillerdeki sözleridir, dil etiketiyle
  (`en`, `pt-BR`; `tr` değil, en çok 20): formun adı ve öğe başına `label`, `help`, `placeholder` ve listeli alanın değer
  etiketleri (`choices`, kodlarıyla). Yalnız gösterilen değişir: değerler, kodlar, kurallar ve sorunların kodları formundur; dil
  seçimi kaydı değiştirmez. Formda olmayan öğenin sözleri ön denetimde `form_translation_item` uyarısıdır.

## 6. Sürümler ve kullanımdan kaldırma (ADR 0443 §3)

- Biçim adı `kentos.forms/1`'dir (Python `cad.FORM_PROFILE`). Bu ad altında bir alanın anlamı değişmez; yeni bir eklenti (yeni
  öğe türü, yeni çalışma zamanı) bu profili sessizce değiştirmez: ya profilin bir alanı olarak geriye uyumlu eklenir (eski
  okuyucu reddeder, sessiz kayıp yok, §3) ya da yeni ad (`kentos.forms/2`) ve göçle gelir.
- Her komut çağrısı komutun sürümünü taşır; ev sahibi çalıştırmadığı sürümü `unknown_version` ile reddeder. Python SDK'sı
  `cad.compatibility()` ile ev sahibinin komutlarını kendi üretildiği katalogla karşılaştırır (eksik, eski, yeni, bilinmeyen) ve
  `cad.require(...)` uymayanı çağrıdan önce söyler.
- Bir komutun anlamı değişecekse yeni sürümle gelir; eski sürüm en az bir KentOS sürümü boyunca yanında çalışır, kaldırılacağı ve
  kaldırıldığı sürüm ADR'de ve TODOS.md'de yazılır (katalogda ayrı bir kullanımdan kalkma işareti henüz yoktur). Kaldırılan sürümü
  çağıran `unknown_version` alır.
- Formun şeması (`catalog/v1/forms/form-v1.schema.json`) sözleşmeden üretilir; elle düzenlenmez.

## 7. Denetim

- `KENTOS_WRITE_CATALOG=1 cargo test -p kentos-contracts form_schema` şemayı yeniden yazar; test dosyanın güncel olduğunu denetler.
- `python3 scripts/fixtures/form_schema_cases.py --check` deponun ortak dosyalarındaki her formu `jsonschema` ile şemaya karşı
  sınar ve bilerek bozulmuş altı formun reddedildiğini görür (KentOS kodu olmadan).
