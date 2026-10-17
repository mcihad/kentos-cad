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
| `draft` | Taslağın gövdesi (`FormBody`): `revision`, `items`, isteğe bağlı `clearHidden`, `tests`, `workflow` |
| `releases` | Yayınlar, eskiden yeniye: `version` (1, 2, …), `digest`, `body`, isteğe bağlı `note` |
| `enforce` | Son yayın katmanın yazma kuralıdır (yalnız `true` yazılır) |

Öğeler (`items`) `type` ile ayrılır: `tab`, `group`, `field`, `note`, `related`, `attachments`, `location`. Alan öğesi katmanın
alanını anahtarıyla (alanın `id`'si, yoksa adı) anar.

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

## 5. Denetim

- `KENTOS_WRITE_CATALOG=1 cargo test -p kentos-contracts form_schema` şemayı yeniden yazar; test dosyanın güncel olduğunu denetler.
- `python3 scripts/fixtures/form_schema_cases.py --check` deponun ortak dosyalarındaki her formu `jsonschema` ile şemaya karşı
  sınar ve bilerek bozulmuş beş formun reddedildiğini görür (KentOS kodu olmadan).
