# 5180 — Kurum eşlemesinde ad dönüşümünün biçimi (TUCBS, BÖHHBÜY)

Tarih: 2026-11-11. plan/map-cad.md MAP-17, onuncu tur. Durum: taslak. ADR 4800'ün “Sonra”sındaki “kurum profilleri: ad
dönüşümünün biçimi (TUCBS lowerCamelCase, BÖHHBÜY BÜYÜK_HARF) profilin seçimi olarak”. Dilim maddenin sıradaki işinin
bütünüdür.

İlgili:
- [ADR 0558](0558-institution-profile-import-migration-mapping.md) §6: kurum eşlemesi (`kentos.profile-mapping`), “hiçbir ad
  kendiliğinden değişmez”;
- [ADR 0735](0735-profile-files-inside-the-project.md): profil ve eşleme çizimin içinde (metin olarak; `.kcad` şeması
  değişmez);
- [ADR 4800](4800-dxf-defined-attribute-name-back.md): okunur ad ile teknik ad; DXF etiketinde boşluk “_”.

## Dayanak

- **TUCBS Veri Tanımlama Dokümanı Şablonu (VTS), ÇŞB CBS GM 2020, §5.2.3** (`.run/research/map-cad/mevzuat/pdf/tucbs/`,
  10–11 Kasım 2026'da okundu): “Öznitelik adı, öznitelik adları için belirlenmiş kurallara uygundur, (lowerCamelCase).
  Kısaltmalar gibi tüm harfleri büyük harflerden oluşan kelimeler istisnadır.” **GKM §9.5.2.5** aynı yazımı kod ve değer
  listesi değerleri için gereksinim yapar (boşluk yok, ilk sözcükten sonraki her sözcüğün ilk harfi büyük).
  - Yazılı olmayanlar: ilk sözcük kısaltmaysa ne olacağı; Türkçe harfin kalıp kalmayacağı; rakamın yeri.
  - Tema dokümanlarının örnekleri (AK, OG): `tucbsNo`, `tucbsId` (ilk sözcük kısaltma, küçük), `sYLKK` (sonraki
    sözcük kısaltma, büyük kalır), `araziKullanimiDurumu5000` (rakam sözcüğe bitişik), `yasalBaglayicilikDurumu` (ASCII).
    GKM'nin kendi örneği `ToprakTürüDeğeri` Türkçe harfi korur: kaynaklar çelişir.
- **BÖHHBÜY Ek-1 öznitelik tablosu** (RG 30460 mük., 4/10/2025 değişikliğiyle; `mevzuat/pdf/bohhbuy/bohhbuy.txt`):
  bir kod listesidir, kod ↔ tanım: `OLCU_YONTEMI` — “DETAYIN ÖLÇÜM YÖNTEMİ (KODLU)”, `NITELIK_01` — “PARSEL CİNSİ
  BİLGİSİ (KODLU)”, `ADRES_1_IL` — “BAĞLI BULUNULAN İLİN KODU”, `MALIK` — “… MERNİS NUMARASI”.
  - Kodlar büyük harf, ASCII, sözcükler ve rakamlar “_” ile ayrı yazılmış; **bu yazım yönetmelikte kural olarak geçmez**,
    listenin görünüşüdür.
  - Kodlar tanımdan türetilemez (“DETAYIN ÖLÇÜM YÖNTEMİ” → `OLCU_YONTEMI`): kurumun kodu ancak açıkça eşlenir.
- **BÖHHBÜY Ek-2 UVDF §4.4.1.2**: “Kolon isimlerindeki geçersiz karakter kullanımı ve veri uzunluğunda, tablonun
  oluşturulduğu veri tabanının kabul kriterleri esas alınır.” Havuzdan gelen kolonun adı havuzdakiyle aynı olmalıdır
  (havuz.xml bulunamadı, R11-MAP-17).
- **Benzer programlar** (`.run/research/map-cad/uygulama/MAP-17.10.md`, resimler `MAP-17.10-resim/`):
  - QGIS 3.40 “Refactor fields”: alan eşlemesi satır satır elle yazılan bir tablodur; ad biçimi kuralı yok, başka
    katmanın alanları şablon olarak alınır.
  - FME BulkAttributeRenamer: bütün ya da seçili adlar toptan değişir (önek, sonek, düzenli ifade, UPPERCASE, lowercase,
    Title case); camelCase ya da BÜYÜK_HARF_ALT_ÇİZGİ hazır değildir; adı boş kalan öznitelik düşer.
  - Netcad'in vikisi giriş sayfasına yönlendirdi, okunmadı.
- **KentOS bugün:** kurum eşlemesi her adı açıkça yazar (`fields`); eşlemenin anmadığı alan profilin adıyla kalır. Kurumun
  adlandırma biçimi (bütün alanlar BÜYÜK_HARF) bir bir yazılmak zorundaydı; çakışma yalnız tam eşitlikle aranıyordu,
  oysa katman alanlarını Türkçe katlanmış adla ayırır (`fields_problem`).

## Karar

1. **Eşlemenin ad biçimi (`names`).** `kentos.profile-mapping` 1'e isteğe bağlı `names`: `case` (`lowerCamelCase` ya da
   `upperSnakeCase`) ve `letters` (`keep`, öntanımlı; ya da `ascii`). Eşlemenin kendi yazdığı ad (`fields`) her zaman önce
   gelir, sonra biçim, yoksa profilin adı. Biçim yalnız **alan adlarına** uygulanır: katman yolu ve kodlar eşlemede açık
   yazılır (aşağıda “Sonra”).
   - `names`'li eşleme sınıfsız olabilir (yalnız biçim: her sınıf profilin katman yolunda); `names`'siz eşlemede sınıf
     1–256 kuralı sürer.
   - Biçim sürümü 1 kalır: alan isteğe bağlıdır, eski eşlemeler aynen okunur. Bu dilimden önceki KentOS `names`'li
     eşlemeyi okuyamaz (bilinmeyen alan): listede sorunuyla görünür, komutlar reddeder.
2. **Sözcükler** (KentOS'un kuralı; metinler sözcüğü tanımlamaz): harf ve rakam dizileri; başka her karakter ayırır.
   Küçük harften ya da rakamdan sonra gelen büyük harf, bir de ardından küçük harf gelen büyük harf yeni sözcük başlatır
   (`TUCBSNo` → `TUCBS`, `No`; `ParselNo` → `Parsel`, `No`). Rakam önceki sözcükte kalır (`Durumu5000`; tema
   dokümanının yazımı).
3. **lowerCamelCase** (VTS §5.2.3): ilk sözcük küçük; sonraki sözcüklerden iki ve daha çok harfli, harflerinin hepsi büyük
   olan kısaltmadır ve olduğu gibi kalır, öbürlerinin ilk harfi büyük, kalanı küçük. **İlk sözcük kısaltma olsa da küçük
   yazılır**: metin bunu söylemez, tema dokümanlarının `tucbsNo`'su öyle yazar. Profilin adları tümüyle büyük harfliyse
   her sözcük kısaltma sayılır (`KAT SAYISI` → `katSAYISI`): adları kurum eşlemede yazmalıdır.
4. **upperSnakeCase**: her sözcük büyük harfle, “_” ile (`Adres 1 il` → `ADRES_1_IL`, `KapiNo` → `KAPI_NO`). Bu,
   BÖHHBÜY Ek-1'in **yazımıdır**, kuralı değil; arayüz ve ileti onu “BÜYÜK_HARF” diye anar ve Ek-1 kodlarının addan
   türetilemediğini söyler.
5. **Harfler.** `keep`: Türkçe harfler kalır, büyük küçük harf Türkçe yapılır (i ↔ İ, ı ↔ I; `KapiNo` → `KAPİ_NO`). `ascii`:
   ç ğ ı İ ö ş ü ile â î û c g i I o s u a i u olur; başka ASCII dışı harf (`é`) **yazılamaz**. Metinler sessiz kaldığı için
   (GKM örneği Türkçe harfli, tema dokümanları ASCII) seçim kullanıcınındır; öntanımlı `keep` hiçbir harfi değiştirmez.
6. **Sessiz kayıp yok.** Biçimin yazamadığı ad (harfi ya da rakamı yok; ASCII karşılığı olmayan harf) `unfit`, sınıfta
   başka bir alanla Türkçe katlanmış olarak aynı ada düşen ad `clash` olur (ikisi de söylenir). Böyle bir eşleme
   `mapping_problem`'den geçmez: içe aktarma, Veri profilini uygula ve denetle `invalid_profile_mapping` ile reddeder,
   ileti alanı ve çözümü söyler (“kurumdaki adını eşlemede yazın”). Çakışma artık Türkçe katlanmış adla aranır (katman
   alanlarını böyle ayırır); eski tam eşitlik denetimi bunun içindedir, ADR 0558'in iletileri aynı kalır.
7. **`cad.profiles.names` 1 (“Kurumdaki alan adları”)**, salt okunur: profilin her alanı, profilin sırasıyla: sınıf,
   profildeki ad, kurumdaki ad, nereden geldiği (`profile`, `form`, `mapping`), sorunu ve iletisi; `renamed`, `unfit`,
   `clashes`, `holds`. `names` eşlemenin biçiminin yerine okunur (önizleme; eşleme yoksa yalnız biçim). Ret: profilin
   (`unknown_profile`, `invalid_profile`) ve eşlemenin biçimi (`unknown_mapping`, `invalid_profile_mapping`); adların
   sorunları ret değil, satırdır (`mapping_shape_problem` ile `names_problem` ayrıldı).
8. **`through`**: eşlemenin anmadığı sınıflar da biçimle okunur (katman yolu ve kodları profilin); adı değişen alanın
   kimliği profilin adıdır (ilişki ve form bağları kopmaz). Kimlik alanı, kayıtlı alan, ilişki ve adres verisi alanları
   biçimin adıyla yazılır.
9. **Masaüstü:** Veri profili penceresinde Bulgular | Kurum adları. Kurum adları: Ad biçimi (Eşlemedeki, lowerCamelCase,
   BÜYÜK_HARF), Harfler (ASCII yaz), dayanağın tek satırı, kitin veri tablosunda Sınıf | Profildeki ad | Kurumdaki ad |
   Nereden; sorunlu satıra basınca nedeni. **Biçimi eşlemeye yaz** seçili eşlemeyi biçimiyle yeniden içe aktarır (durduğu
   yere: çizimin içi ya da klasör); yalnız eşleme seçiliyken ve biçim tutarken açıktır. Sonra Bulgular ve Uygula kurumun
   adlarını bununla okur.
10. **Uygun denmez.** Biçim, kurumun verisinin TUCBS'ye ya da BÖHHBÜY'e uygunluğunu kanıtlamaz: temanın ya da Ek-1'in
    adıyla eşleşme açık eşlemedir (kullanıcının verisi).

## Doğrulama

- **Ortak durumlar** (`scripts/fixtures/profile_names_cases.py`, KentOS kodu olmadan: sözcükler, harfler, büyük küçük harf,
  çakışma ve `through` yeniden yazıldı; denetim ve uygulama adımlarının cevabı `profile_cases.py`'nin başvuru kurallarıyla
  biçimin okuduğu profilde): `fixtures/commands/v1/cad.profiles.names.json` 32 durum: yerleşik profil dört biçimle; Türkçe
  harfli, boşluklu, kısaltmalı, rakamlı adlarla bir profil dört biçimle (VTS ve Ek-1 örnekleri: `tucbsNo`,
  `araziKullanimiDurumu5000`, `OLCU_YONTEMI`, `ADRES_1_IL`, `NITELIK_01`); çakışma, yazılamayan ad ve eşlemenin onları
  gidermesi; eşlemenin kendi adları ve biçimi, önizleme; sınıfsız biçim; katlanmış adla çakışma; klasördeki eşleme adıyla;
  doğrulama; altı ret; biçimle denetim (boş çizim; kurumun çizimi, eşlemenin adları ve biçim birlikte), biçimle uygulama ve geri alma, çakışan biçimin iki komutta reddi.
  `KENTOS_CASES_ONLY=cad.profiles` bütün profil komutlarının durumları ve yetenek testi geçer; `profile_cases.py` ve
  `profile_files_cases.py --check` aynı.
- **Birim** (`kentos-commands`, `cad_profile_names::tests`): metinlerin örnekleri.
- **Python** (`python/tests/test_profile_names.py`): satırlar profilin alanları, BÜYÜK_HARF ASCII düzenli ifadesi,
  lowerCamelCase boşluksuz ve küçük harfle başlar, eşlemenin adları önce, sayılar satırlarınki; sınıfsız biçimli eşleme
  çizimin içine alınır, denetim `MAHALLE_KODU` der; çakışan biçim uygulamada reddedilir; hiçbir şey yazılmaz.
- **MCP** (`crates/native/mcp/tests/profiles_names.rs`): ortak durumun cevabı, biçimle denetim, bilinmeyen eşleme.
- **Masaüstü** (`data_profile_names_tests`): biçim ve harf seçimi; eşleme çizime alınınca kendi adları önce;
  Biçimi eşlemeye yaz çizimin eşlemesine `names` yazar, Bulgular `MAHALLE_KODU` der; çakışan ve yazılamayan adlar
  söylenir, yazma kapalıdır. Resimler `.run/gui-evidence/kurum-adlari-5180/`.

## Sonra

- Kodların biçimi: GKM §9.5.2.5 kod ve değer listesi değerlerini de lowerCamelCase ister; bugün kodlar eşlemede açık
  yazılır.
- Katman adının biçimi: VTS'nin katman adı “iki harfli tema kısaltması . UpperCamelCase nesne adı” (`AD.Adres`); KentOS'un
  katman yolu ağaçtır, eşlemede açık yazılır.
- DXF öznitelik etiketi (ADR 4800) hâlâ boşluğu “_” yapar; kurum biçimiyle dışa aktarma ayrı iştir.
- Ek-1 kod listesinin (ve havuz.xml bulunursa havuzun) hazır eşleme olarak sunulması: resmî dosya bekler.
- Web: `cad.profiles.names` ve eşlemenin `names`'i (TODOS §0.2).
