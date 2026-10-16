# ADR 0404: Formlar: başka katmandan seçenek listesi, ilişki başvurusu ve ilişkili kayıtlar

- **Durum:** kabul edildi (2026-10-16). Sahibin isteği (16 Ekim): formları QGIS'in form yönetiminden daha iyi hâle
  getirene kadar sürdürmek.
- **Bağlam belgesi:** ADR 0401–0403; `plan/form.md` FORM-SRC-01, -04, -05, FORM-REL-10, -12, FORM-DATA-20; TODOS.md §16.6
  `FORM-H`.
- **Web:** ertelendi (ADR 0228). Web'in tarafı TODOS.md §0.2'de.
- **Sonraya kalan:** N:M bağlantı tablosu, ilişkinin silme politikası (kademeli silme), geometrisiz tablo katmanı, dış kaynak
  (kurum servisi) seçenekleri.

## Bağlam

Kodlu değer listesi bugün katman alanının kendi `values`'ıdır (ADR 0199). Gerçek veride liste çoğu zaman başka bir
katmandadır: il, ilçe ve mahalle katmanları, malzeme sözlüğü, yapı türleri tablosu. Bir kaydın başka bir nesneye bağlanması
da (bina → parsel, sayaç → abone) aynı ihtiyaçtır.

- **QGIS:** Value Relation öğesi başka katmandan anahtar ve değer sütunu, ifade süzgeci, `current_value('alan')` ile
  zincirleme, çoklu seçim, otomatik tamamlama sunar. Relation Reference öğesi tanımlı bir ilişkinin üst nesnesini seçer
  (haritadan seçme, önizleme). İlişki düzenleyicisi alt kayıtları formun içinde gösterir.
  - Zayıflığı: `current_value()` açılışta ilk değeri görmez, zincirdeki alt liste kullanıcı üst alanı yeniden yazana dek
    süzülmez (QGIS #21852, GitHub #29667); kurallar yalnız formdadır.
- **ArcGIS Field Maps:** alan alanı (domain) ve koşullu değerler (contingent values); başka katmandan liste Arcade ile
  hesaplanır, yazımı ayrıca denetlenmez.

## Karar

### 1. Alanın seçenek listesi başka katmandan: `FormField.lookup`

```json
"lookup": {
  "layer": "mahalle",
  "key": "kod",
  "label": "ad",
  "matches": [{ "field": "ilce", "record": "ilce" }],
  "filter": "durum = 'etkin'"
}
```

- `layer`: listenin katmanının kimliği. `key`: saklanan değerin alanının anahtarı; `$uid` ise nesnenin kalıcı kimliği
  (ilişki başvurusu, §3). `label`: gösterilen alanın anahtarı, yoksa `key`.
- `matches`: zincirleme. Listedeki nesnenin `field` alanı kaydın `record` alanına eşit olmalıdır (ikisi de alan
  anahtarı). Kaydın alanı boşsa liste boştur: önce üst alan seçilir.
- `filter`: listenin katmanının alanlarını ve projenin `@` değerlerini (anı ve kullanıcıyı değil) okuyan ifade; doğru dönen
  nesneler listededir.
- Liste: anahtarı boş olmayan nesnelerin anahtarları, bir kez (ilk nesnenin etiketiyle; etiketi boşsa kodu), etiketin doğal
  sırasıyla (ADR 0227'nin doğal sırası), eşitlikte kodun sırasıyla.
- Alanın kendi değer listesi (`values`) ya da `choiceFilter` ile birlikte verilmez (`form_rule_conflict`).
- Varsayılan öğe türü açılır listedir; radyo ve parçalı seçim de olur.

### 2. Kuralları

- **Ön denetim** (`preflight_in`, projenin öbür katmanlarını bilerek): alanın kendi değer listesi ya da seçenek süzgeci
  varsa `form_rule_conflict`; listenin katmanı yoksa `form_lookup_layer`; anahtar, etiket ya da eşleşmenin alanı o katmanda
  yoksa, eşleşmenin kayıt alanı formun katmanında yoksa `form_lookup_field`; süzgeç okunmazsa `form_rule_invalid`
  (`lookup.filter`). Hepsi hatadır, kuralın yeri `lookup`'tır.
- **Durum:** öğenin durumunda liste `options`'tadır (kod ve etiket), `choices` kodlardır (süzgeçli seçenekle aynı alan).
- **Yazma:** etiket yazılırsa kodu yazılır (alanın değer listesindeki gibi). Kayıttaki değer listede yoksa
  `form_lookup_missing` hatasıdır; değer boşsa sorun yoktur (zorunluluk ayrıdır). Açılışta da doğrudur: liste kaydın o
  anki değerleriyle kurulur.
- **Belgenin izlemesi** (ADR 0402 §4) ve `cad.entities.set`'in yazma profili listeyi de sorar: bütün yazma yollarında aynı.

### 3. İlişki başvurusu: `key: "$uid"`

Kayıt başka bir nesnenin kalıcı kimliğini saklar (bina → parsel). Liste o katmanın nesneleri, etiketi `label` alanıdır.
Silinen nesneye başvuran kayıt `form_lookup_missing` verir. Formla düzenle seçilen nesneyi çizimde gösterir (Git).

### 4. İlişkili kayıtlar: `related` öğesi

```json
{ "type": "related", "id": "binalar", "label": "Parseldeki binalar", "layer": "bina", "field": "parsel" }
```

Formun içinde, başka katmanın bu kayda başvuran nesnelerinin listesi: `field` alanı bu kaydın kimliğine (`$uid`) eşit
olanlar. Satırda nesnenin etiketi ya da türü; **Aç** o nesnenin formunu açar, **Git** çizimde gösterir. Liste salt
okunurdur, yazma alt nesnenin kendi formuyla yapılır. Ön denetim: katman ya da alan yoksa `form_lookup_layer`,
`form_lookup_field`. `cad.forms.validate` öğenin durumuna kayıtların kimliklerini (`related`) ekler.

### 5. Değerlerin gördüğü veri

Liste çağrı anında çizimden kurulur (`Document::form_world`): listenin katmanının bütün nesneleri; katman süzgeci (ADR 0211)
görünüşündür, verinin kuralı değildir. Form hazırlanırken bir kez kurulur (süzgeç o zaman sorulur), değerler yazıldıkça
yalnız zincirle süzülür. Belgenin izlemesi ve `cad.entities.set`'in yazma profili aynı listeyi kurar.

## Denetimler

- Ortak durumlar KentOS kodu olmadan yazılmış başvurudan (`scripts/fixtures/forms_cases.py`; doğal sıra
  `chart_cases.py`'den): çekirdekte 6 ön denetim, 5 kayıt, 4 yazma; komut durumlarında `cad.forms.validate`'e 4 (ilişkili
  sayaçlar, kimlikleri `$uidOf:N`), `.submit`'e 4, `cad.entities.set.forms`'a 2, `.fill`'e 1.
- KCAD'in bağımsız Python okuyucusu `lookup`'ı ve `related` öğesini okur; `forms.kcad` örneği ikisini de taşır.
- Masaüstünün akış testleri ve resimleri `form_scenes.rs`'te.

## Sonuçlar

- Başka katmandan liste bütün yazma yollarında denetlenir; zincirleme açılışta da doğrudur.
- Kayıt başka nesneye kalıcı kimliğiyle bağlanır; adın değişmesi bağı koparmaz.
