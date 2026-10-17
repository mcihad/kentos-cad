# ADR 0443: Formların erişimi ve kabulü: şema keşfi, örneklerde gizlenen alanlar, sürümler, her girişte aynı kod, çeviriler, klavye ve kabul

- **Durum:** kabul edildi (2026-10-17). TODOS.md §16.6 `FORM-AZ`; `plan/form.md` FORM-API-08, -14, -16, FORM-QA-01, -04, -09,
  -16, FORM-DESIGN-16.
- **Bağlam belgesi:** ADR 0401 (formlar), 0253 (komut protokolü, kayıt), 0284 (komut geçmişinde çağrılar), 0422–0423 (alt türler,
  bileşimler), 0434 (formun şeması), 0438 (kayıt kuralları), 0441 §4 (gerektirdikleri), 0442 (yayın ve iş akışı ekleri).
- **Web:** ertelendi (ADR 0228). Web'in tarafı TODOS.md §0.2'de.
- **Sonraya kalan:** keşfin izinlerle süzülmesi (bulutun rolleri), kaydın gizli alanlarının sunucuda maskelenmesi, gerçek ekran
  okuyucusu desteği (Iced'in erişilebilirlik ağacı yok), saha cihazında kabul.

## Bağlam

Bir ajan projenin şemasını öğrenmek için ya bütün katmanları ve formları tek seferde okuyordu ya da hiç okuyamıyordu. Komut
geçmişinin Python'u ve tekrar oynatma girdisi kaydın bütün değerlerini (kimlik numarası gibi) olduğu gibi kopyalıyordu. SDK ile
ev sahibinin komut sürümleri çağrıdan önce karşılaştırılamıyordu. Alt tür, bileşim ve kayıt kurallarının ortak durumları yalnız
`cad.entities.set`'te, `cad.polygon.create`'te ve formda koşuyordu. Form tek dilliydi; odak ve hata gezinmesi yalnız ilk hataya
gidiyordu; Vazgeç ve Esc yazılanı sormadan atıyor, Esc pencereyi kapatmayıp yalnız gizliyordu. Formların kabulü yalnız üreticinin
çıktısıyla karşılaştırmalardaydı.

İşi yaparken bir kusur bulundu: `cad.forms.edit`'in Kaydet'i (Update) taslağın yalnız öğelerini ve “gizlenenleri sil”ini
yazıyordu; tasarımcıda düzenlenen senaryolar, iş akışı, adım adım ve çeviriler kaydedilmiyordu. Python ikizi de aynı kuralı
izlediği için hiçbir ortak durum yakalamıyordu.

## Karar

### 1. Şemayı keşfet

Salt okunur `cad.schema.describe`: projenin katmanları (ağacın sırasıyla, gruplar dışında; alan sayılarıyla), formları (katmanı,
öğe sayısı iç içe öğelerle, son yayın), değer listeleri (değer sayısı) ve ilişkileri (ana ve alt katman), bu sırayla, her biri kısa
özetiyle; bütün tanımı yalnız `expand` ile. `kinds` türleri, `layer` bir katmanın sahip olduklarını (kendisi, formları, alanlarının
bağladığı listeler, taraf olduğu ilişkiler), `query` kimlikte ya da adda geçen sözcükleri (Türkçe katlanarak: `I` ı, `İ` i)
süzer. Sayfa `limit` uzunluğunda (1–100, varsayılan 20), `next` sonraki sayfanın imleci (ondalık yazılmış kayma), `total` bütün
bulunanlar. Retler bu sırayla: `invalid_count`, `invalid_cursor` (sayı olmayan, başında sıfırlı ya da listenin ötesinde imleç),
`layer_not_found`. Python ve MCP katalogdan gelir.

### 2. Örneklerde gizlenen alanlar ve Python betiği

Form alanının `redact`'ı (tasarımcıda **Örneklerde gizle**): projenin herhangi bir formunda (taslak ya da yayın) böyle işaretli
alanın anahtarı ve adı kaydın örneklerinden gizlenir. Komut geçmişinde bir çağrının Python'u ve tekrar oynatma girdisi bu adlar
altındaki her metin değerini, ne kadar derinde olursa olsun (değerler, öznitelikler, alt kayıtlar, notlar), `‹gizli›` yazar
(`kentos_native_application::python_call::redacted`). Komut geçmişinin yeni **Python betiği olarak kopyala** düğmesi çizimin
bütün komut çağrılarını sırasıyla tek betik yapar; gizlenen değer varsa betiğin başında söylenir. Kaydın kendisi değişmez.

### 3. Sürümler ve profil

Python SDK'sına `cad.compatibility()` (SDK'nın üretildiği katalogdaki yerel komutlar ile ev sahibinin çalıştırdıkları: eksik, eski,
yeni, bilinmeyen) ve `cad.require(...)` (uymayanı çağrıdan önce `unknown_version` ile söyler) eklendi; `cad.FORM_PROFILE` biçimin
adıdır (`kentos.forms/1`). Sürüm uzlaşması ve kullanımdan kaldırma süreci `docs/specs/forms-v1.md` §6'dadır: profilin anlamı
değişmez, yeni çalışma zamanı onu sessizce değiştirmez; komutun yeni anlamı yeni sürümdür.

### 4. Her girişte aynı kod; gerektirdikleri ve desteklenmeyenler

- Alt türlerin, bileşimlerin ve kayıt kurallarının `cad.entities.set` durumlarında değerin kendisinden reddedilen her kayıt yeni
  alan (`cad.polygon.create`) ve geometrisiz kayıt (`cad.records.create`) olarak da aynı kodla reddedilir (test `entries.rs`). Bir
  düzenlemenin kendi işi olan farklar dışarıdadır ve bilerek öyledir: alt türün zorunlu alanını boşaltmak (yeni kayıt bunun için
  reddedilmez, ADR 0422), düzenlemenin dokunmadığı alanlar. Kodlar dilin değil komutundur; dil yalnız cümleleri değiştirir.
- Formun gerektirdikleri ve bir ortamın sağlamadıkları ortak çekirdek durumlarındadır (`fixtures/forms/v1/cases.json`'un `needs`'i):
  saf form, an ve kullanıcı, başka katman, konum, ekler, iş akışı, araçlar; üç ortam (masaüstü, yalnız veri, çevrimdışı
  görüntüleyici).

### 5. Çeviriler

Gövdenin `translations`'ı: dil etiketi (`en`, `pt-BR`; `tr` değil, en çok 20) başına formun adı ve öğe başına `label`, `help`,
`placeholder` ve listeli alanın değer etiketleri (`choices`, kodlarıyla). Yalnız gösterilen değişir: değerler, kodlar, kurallar
formundur. Formda olmayan öğenin sözleri ön denetimde `form_translation_item` uyarısıdır; sürüm farkında çevirilerin değişmesi
yerleşimdir. Tasarımcıda öğe seçilmemişken **Çeviriler**: diller (Dil ekle, kaldır), seçili dilin sözleri formun kendi sözlerinin
yanında; yazmak tek geri alma adımı. Formla düzenle'nin altında dil seçici (Türkçe ve formun dilleri, adlarıyla).

### 6. Klavye, hatalar ve kapanış

- İki sütunlu grubun `across`'u (tasarımcıda **Satır satır**): öğeler satır satır dizilir, Tab önce yana sonra aşağı gider;
  yoksa birinci sütun, sonra ikincisi.
- **İlk soruna git** sonra **Sonraki sorun**: her basış formun sırasıyla bir sonraki hataya gider (sekme açılır, kapalı grup açılır,
  alan odak alır), sondan sonra başa döner; özet “k/n sorun” der.
- Yazılanı kaydedilmemiş formda Vazgeç ya da Esc sorar: **Kaydetmeden kapat**, **Düzenlemeye dön**, **Kaydet ve kapat**; soruda
  Esc düzenlemeye döner. Kaydet ve kapat reddedilirse form sorunlarıyla kalır. Değişmemiş form sormadan kapanır. Esc artık
  pencerenin Vazgeç'idir (önceden yalnız gizliyordu).

### 7. Kabul

- Gerçek girdi: masaüstü testi pencerenin kendisinde (KentOS UI'ın `snapshot`'ı) Tab ve Shift+Tab'ı, odaklı alana yazmayı, sekme
  değişince yazılanın kalmasını, Kaydet'in odağı soruna götürmesini ve Esc'in sorusunu sınar.
- Python'dan kabul senaryosu (`python/tests/test_forms_acceptance.py`): merkez formu tasarlar ve yayınlar, saha yazar, inceleyen
  notla geri gönderir, onaylanan kayıt kilitlenir, ajan şemayı sayfa sayfa bulur. Beklentiler ADR'lerden elle yazıldı (notların
  kanonik JSON'u, sayılar), kodun döndürdüğünden alınmadı.
- `cad.forms.edit`'in Kaydet'i taslağın bütününü yazar (öğeler, senaryolar, iş akışı, adım adım, çeviriler); yalnız çeviri de bir
  değişikliktir. Rust ve Python ikizi birlikte düzeltildi; ortak durum eklendi.

## Denetimler

- `scripts/fixtures/schema_describe_cases.py --check`: `cad.schema.describe.json` 29 durum (`domains_cases.py`'nin katmanları ve
  listeleriyle, KentOS kodu olmadan).
- `scripts/fixtures/forms_cases.py --check`: gerektirdikler ve desteklenmeyenler (15 durum), çevirinin ön denetimi, Kaydet'in bütün
  taslağı; `forms_compare_cases.py`: çevirilerin farkı.
- Rust: `entries.rs` (her girişte aynı kod), `kentos-forms`'un `needs` testi, `python_call`'ın gizleme testi.
- Masaüstü: `form_access_scenes` (klavye, kapanış, sorunlar, çeviri, tasarımcıda çeviri); resimler `form-ceviri-giris`,
  `form-ceviriler`, `form-sorun-gezinme`, `form-kapanis-sorusu`, `form-satir-satir`.
- Python: `test_versions.py`, `test_forms_acceptance.py`.

## Sonuçlar

- Ajan bağlamına bütün şemayı değil gerekeni koyar; kaydın gizli değerleri örneklere sızmaz.
- SDK ile KentOS'un uyuşmazlığı çağrıdan önce söylenir; biçimin adı sabittir.
- Kuralların kodları her girişte aynıdır; form başka dillerde gösterilir, veri aynı kalır.
- Klavyeyle form baştan sona doldurulur; yazılan sormadan atılmaz.
- Tasarımcıda yapılan her değişiklik Kaydet'le gerçekten kaydedilir.
