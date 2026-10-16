# ADR 0414: Alanın kullanıldığı yerler ve yeniden adlandırmanın izlenmesi

- **Durum:** kabul edildi (2026-10-16). Sahibin isteği (16 Ekim): formları QGIS'in form yönetiminden daha iyi hâle
  getirene kadar sürdürmek. TODOS.md §16.6 `FORM-U`; `plan/form.md` FORM-DATA-07.
- **Bağlam belgesi:** ADR 0199 (katman alanları), 0401 §2 (alanın kararlı kimliği), 0402 (kurallar), 0404 (başka katmandan liste),
  0405 (ilişkiler), 0210 (zaman), 0211 (süzgeç), 0212 (etiket sınıfları), 0225 (bilgi kartı ve eylemler), 0413 (koşullu biçim).
- **Web:** ertelendi (ADR 0228). Web'in tarafı TODOS.md §0.2'de.

## Bağlam

Bir alanın adı değişince öznitelik değerleri yeni adla taşınıyordu (ADR 0199), formun öğeleri alanın kimliğiyle bağlı kalıyordu
(ADR 0401 §2). Ama alanı **adıyla** anan her şey kopuyordu: formun koşulları ve hesapları, notların `{…}`'ı, katmanın süzgeci,
etiket sınıfları, koşullu biçim, zaman ayarı, bilgi kartı ve eylemler. Silinen alan da bunları sessizce bozuyordu. QGIS'te alanı
yeniden adlandırmak ifadeleri güncellemez; kullanıldığı yer de gösterilmez. ArcGIS Pro alanı silmeden önce bağlı olanları
sormaz; yalnız bazı ilişki sınıfları reddeder.

## Karar

### 1. Kullanıldığı yerler

Bir alanın kullanıldığı her yer (`kentos_contracts::FieldUse`): yerin Türkçe adı (`place`, ““Bina denetimi” formu › “Not”
öğesi › Görünme koşulu”), belgedeki yolu (`path`), alanı nasıl andığı (`how`: `key` alanın anahtarı, `name` adı, `expression`
ifade, `template` `{…}`'lı metin) ve yayımlanmış bir form sürümünde olup olmadığı (`kept`). Yerler sırasıyla:

1. Formlar, ayardaki sırayla: alanın katmanının formlarının taslağı, sonra sürümleri; öğeler sırayla, her öğede alan, görünme,
   zorunluluk ve salt okunurluk koşulları, varsayılan ifadesi, hesap, seçenek süzgeci, denetimler; notun metni; grubun, ilişkili
   kayıtların ve eklerin görünme koşulu. Her formda bu katmandan liste alan öğenin anahtarı, etiketi, eşleşmenin kayıt alanı ve
   süzgeci; bu katmanın ilişkili kayıtlarını gösteren öğenin alanı.
2. İlişkiler: alt katmanı bu katman olanların alanı.
3. Katmanın kendisi: koşullu biçimin kuralları (alan, koşul), süzgeç, etiket ifadesi ve etiket sınıfları (koşul, metin), zaman
   (başlangıç, bitiş, kimlik), bilgi kartı (başlık, alanlar, metin), eylemlerin hedefi.

İfade ve şablon dilin sözcükleriyle okunur (`kentos_expression::editor::fields_of`): tırnak içindeki metin, işlevin adı ya da
anahtar sözcük alan sayılmaz. Katman stilinin işleyicisi (opak JSON) ve başka katmana bakan işlevlerin metin bağımsız değişkenleri
taranmaz.

### 2. Yeniden adlandırma

`cad.layers.edit`'in alanları (`renames`) aynı adımda:

- alanı **anahtarıyla** anan yer varsa (form öğesi, liste, ilişki) alan eski adını kimlik olarak tutar (ADR 0401 §2'nin kuralı
  bütün anahtar yerlerine genişler): anahtar değişmez, hiçbir anahtar yeri yeniden yazılmaz;
- **adıyla**, **ifadeyle** ya da **şablonla** anan yerler yeni adla yeniden yazılır (`rename_field`, `rename_in_template`): kısa
  ad yeni ad tek sözcük olarak okunuyorsa kısa kalır, yoksa köşeli paranteze alınır; kaynağın başka karakteri değişmez;
- katmanın yerleri (koşullu biçim, süzgeç, etiketler, zaman, kart, eylemler) aynı geri alma adımındadır;
- formların taslakları ayardır ve ayar geri alma adımı değildir (ADR 0401): taslaklar yeni adla kalır;
- yayımlanmış sürümler değişmez (ADR 0411 §3): onlardaki ifade eski adı anar; pencere bunu söyler, form yeniden yayınlanır.

### 3. Komut

`cad.fields.usage` v1 (salt okunur, “Alanın kullanıldığı yerler”): `layer`, `field` (adı) → `layer`, `field`, `uses`. Retler:
`layer_not_found`, `not_a_layer`, `service_layer`, `field_not_found`. Python `kentos.cad.fields.usage`, MCP'de araç.

### 4. Masaüstünde

Alanlar penceresinde seçili alanın altında **Kullanıldığı yerler (n)**: her yer bir satır, yayımlanmış sürümdekiler “değişmez”
notuyla. Ad değişince satır “n yer yeni adla güncellenecek” der; kullanılan alan silinince uyarı “bu alan n yerde kullanılıyor;
silinirse o yerler bozulur” der.

## Denetimler

- `scripts/fixtures/field_usage_cases.py --check`: `cad.fields.usage`'in ortak durumları ve yeniden adlandırmanın durumları
  (`fixtures/commands/v1/cad.fields.usage.json`, `cad.layers.edit.renames.json`) KentOS kodu olmadan, dilin sözcük kuralının
  bağımsız bir kopyasıyla.
- `kentos-expression`'ın `rename` testleri; masaüstünde Alanlar penceresinin testi ve resmi `form-alan-kullanim`.

## Sonuçlar

- Alanın adı değişince kurallar kopmaz; silinmeden önce neyin bozulacağı görülür.
- Açık: katman stilinin işleyicisi, başka katmana bakan işlevler, ağ ve adres verisinin alanları; formun sürümünü yeniden
  yayınlamayı önermek.
