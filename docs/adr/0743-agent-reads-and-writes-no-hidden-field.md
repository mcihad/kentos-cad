# ADR 0743: Ajan gizli alanla süzmez, hesaplamaz ve onu dosyaya yazmaz

- Durum: Kabul edildi (29 Ekim 2026)
- Madde: plan/gis.md CBS-38'in beşinci dilimi
- İlgili: [ADR 0738](0738-agent-veil-entry-pages-and-unreached-events.md) (ajanın okuduğu), [ADR 0638](0638-atlas-systems-clip-sources-and-withheld-fields.md) §5
  (`withheld`), [ADR 0443](0443-forms-reach-and-acceptance.md) §2 (`redact`), [ADR 0550](0550-one-reach-resolver.md) (erişim
  çözücüsü), [ADR 0414](0414-field-usage-and-renames.md) (ifadede alan adı)

## Bağlam

ADR 0738 ajanın cevaplarından gizli alanın değerlerini keser: MCP ve masaüstünün ajan bağlantısı `redact` ve
`withheld` alanların değerlerini `‹gizli›` yazar. Kalan iki yol açıktı:

- **Kâhin:** cevaptaki değer kesilse de çağrının kendisi değeri sorabilir. `Malik = 'Ali Yılmaz'` süzgeciyle sayıyı,
  `upper(Malik)` ile açık bir alana kopyayı, tekil `TC` alanına deneme yazısının reddiyle değerin varlığını öğrenir.
- **Dosya:** ajanın çağırdığı `file.export.*` dosyayı çizimden yazar; dosyada gizli alanların sütunu ve değerleri
  vardır.

Üreticilerin yolu (29 Ekim 2026'da okundu):

- Esri'nin hata kaydı BUG-000173430: bir harita servisinde gizlenen alan sorgunun WHERE koşulunda yine
  kullanılabiliyor; gizleme yalnız gösterimdir. Esri'nin önerdiği güvenli yol alanı hiç içermeyen bir görünüm
  (geodatabase view). <https://support.esri.com/en-us/bug/hidden-fields-in-a-map-service-can-still-be-used-in-the-bug-000173430>
- con terra security.manager NEXT, “Field restriction”: alan kısıtı (`reduced_fields`) alanı cevaptan da sorgudan da
  çıkarır; kısıtlı alanı anan sorgu reddedilir.
  <https://docs.conterra.de/en/securitymanager-next/latest/tutorials/basic/field-restriction.html>

KentOS ikisini birleştirir: alanı anan çağrı reddedilir, dosya alanı hiç tutmayan bir kopyadan yazılır.

## Karar

### 1. Gizli alanı anan çağrı reddedilir

Çizim bir alanı ajandan gizliyorsa (`kentos_headless::Veil`), ajanın her çağrısının girdisi çalışmadan önce
yürünür (`Veil::named_in`). Girdi kendi sırasıyla gezilir (nesnenin anahtarları bayt sırasıyla, dizinin öğeleri
sırayla) ve ilk bulunan yer söylenir:

- gizli alanın adı olan anahtar alanı anar, nerede olursa olsun (öznitelik haritasının anahtarı da: ajan gizli alana
  yazmaz; tekil alanın reddi değerin varlığını söylerdi);
- çağrının yazdığı metinler (`attrs`, `values`, `value`, `text`, `content`, `note`, `notes`, `comment`, `path`
  altındakiler) içeriktir, okunmaz; altlarında yalnız anahtarlara bakılır;
- başka her metin alanı anar: alanın adının kendisiyse, `{…}` içeren bir şablonun ifadeleri alanı okuyorsa ya da bir
  ifade olarak alanı okuyorsa (ADR 0414'ün `fields_of`'u: köşeli ad ya da çağrılmayan ve anahtar sözcük olmayan
  sözcük; tırnaktaki metin, sayı, `$` ve `@` adları ve açıklamalar okumaz).

Ret `hidden_field`, yolu `input.<yer>` ve sözü alanın adıyla ve yerle. Kural ajan komutun ne yaptığını bilmeden
uygulanır: okuyan, hesaplayan ve yazan komut aynıdır.

### 2. Ajanın dosyası gizli alansız bir kopyadan

Ajanın çağırdığı dosya komutu (`FILE_COMMANDS`: DXF, PDF, GeoPackage, Shapefile, CSV, … ve iş olarak da) çağrının
denetimini (beklentiler, istek kimliği) çizimin kendisinde yapar; dosyayı ise
`Document::reading_copy_without(names)`'ten yazar: gizli alanların değerleri her nesneden, tanımları her katmandan
çıkarılmış okuma kopyası. Etiket şablonları ve ifadeler kopyada boş okur. Çağrının satırı çizimin kendi kaydına düşer.

### 3. İşlem sonucu dosyaya yazılmaz

`cad.processing.run` ve `.batch`'in `sink`'i girdilerin özniteliklerini dosyaya taşır; çizim alan gizliyorsa ajanın
`sink`'li çağrısı `hidden_field_sink` ile reddedilir. Sonuç çizime yazılabilir (aynı ad orada da gizlidir), dosya
Dışa aktar komutlarıyla verilir (§2).

### 4. Yerler

Tek giriş `dispatch::call_envelope_for_agent` ve `Jobs::start_for_agent`; masaüstünün ajan bağlantısı
(`rpc::call_for_agent`: `call`, `run`, `start`) ve MCP (`Session::call_for_agent`, `drawing.job.start`) onları
çağırır. Konsolun Python'u çizimin sahibidir: kural ona uygulanmaz (ADR 0550 §3).

## Doğrulama

- `python3 scripts/fixtures/agent_oracle_cases.py --check`: 30 ortak durum, KentOS kodu olmadan (kendi ifade
  sözcükleyicisi); `cargo test -p kentos-headless --lib veil` aynı dosyayı okur.
- `cargo test -p kentos-mcp --test agent_veil`: ilk yazılan, sonra gizlenen alanla iki test; gizli alana yazma,
  ifadeyle okuma ve köşeli ad reddedilir, tırnaktaki ad çalışır, `sink`'li işlem reddedilir, CSV doğrudan ve iş olarak
  gizli sütunsuz yazılır, çağrısı kayıtta.
- `cargo test -p kentos-desktop --bin kentos-cad python::tests::an_agent`: masaüstünün ajan bağlantısında aynı ret.

## Sonuçlar

- Yeni ret kodları `hidden_field`, `hidden_field_sink` (`codes.rs`); yeni dosya alanı, komut ya da arayüz yok.
- Ajan gizli alanın tanımını da değiştiremez (`cad.layers.edit`'in alan listesi, formun öğesi onu anar): alanın
  tanımı ve değerleri çizimin sahibinindir.
- Çizimi gizli alan tutmayan ajan için hiçbir şey değişmez.

## Kalan

- Gizli alanın değerini zaten yazı olarak taşıyan nesneler (alanlı yazı, çizelge hücresi, Etiketleri yazıya çevir'in
  yazısı) kopyada da yazılarıyla kalır.
- Sözcük olarak alan adını içeren serbest bir metin içerik anahtarları dışında yazılırsa (bir katmanın adı) yanlış
  ret verir; söz yeri ve alanı söyler.
