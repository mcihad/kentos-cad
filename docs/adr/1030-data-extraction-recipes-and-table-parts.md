# 1030 — Veri çıkarımı tarifi, tekil sayım, birimli metraj ve parçalı tablo

Tarih: 2026-11-05. Durum: kabul (CAD-ADV-13). Kapsam: plan/cad-advanced.md `CAD-ADV-13` “Çizim verisinden güvenilir
tablo ve metraj üret”; sahibin kuralı “maddeyi bütün alt maddeleriyle tek seferde bitir”. Web: ertelendi (ADR 0228);
web'de gerekenler §10'da.

Geçici numaralar (birleştirmede yeniden numaralanır): `.kcad` şeması **182** (`SCHEMA_WITH_EXTRACTION`; 180 ve 181
main dalındaki ADR 1000 ve ADR 1001 içindir), `FORMATS_VERSION` **107**, ADR **1030**.

## Sorun

CAD-ADV-13 bugün (5 Kasım) adım adım ilerledi: blok çizelgesinin etkin öznitelikleri, dosyaya bağlı tablonun satır
anahtarı, çakışma kuralı, formül, alan, birleşim ve biçimlerin anahtarla taşınması, blok adetlerinin sayı olarak dışa
aktarımı (commit `9ee813ed1` … `6c2123db8`). Açık kalanlar:

1. **Yeniden kullanılabilir tarif yoktu:** blok çizelgesi yalnız blok adlarını ve bir özniteliği alıyordu; kapsam, tür,
   alan, hesaplanan sütun, grup, sıralama ve toplama bir arada saklanmıyordu.
2. **Sayım kapsamı örtüktü:** iç içe blok, dinamik blok varyantı, ilişkili dizi, görünmeyen parça ve dış referans için
   seçim yoktu; bir çizim hem kendisi hem bir başkasının dış referansı olarak okunursa iki kez sayılabilirdi.
3. **Metraj birimsizdi:** uzunluk, alan, adet ve katsayı ayrı anlam taşımıyordu; ölçünün elle yazılmış metni (override)
   sayı gibi görünebiliyordu; toplamın yuvarlamadan önce mi sonra mı alındığı tanımlı değildi.
4. **Tablo tek satırlı, tek parçaydı:** hücrede satır sonu yoktu (`cell_problem` reddediyordu), sığmayan hücre
   söylenmiyordu, uzun tablo paftaya sığacak parçalara bölünemiyordu, başlık tekrarlanmıyordu.
5. **Dış açıklama tablosu yalnız bütün bir tablo olarak bağlanabiliyordu;** çıkarılan satırlara anahtarla eklenemiyordu.

## Araştırma

- **AutoCAD Veri Çıkarımı (DATAEXTRACTION, plan R07):** çizim/klasör seçimi, nesne türü süzgeci, “iç içe bloklar”,
  “dış referanslar”, “yalnız model uzayı” seçenekleri; özellik seçimi; “aynı satırları birleştir”, “sayım sütunu”,
  “ad sütunu”; formül sütunu; dış Excel verisini bir eşleme sütunuyla bağlama; sonuç tablo ya da dış dosya; tarif `.dxe`
  dosyasında saklanır; tablo güncelliğini yitirince bildirim. KentOS'un kararı aynı adımları **açık** yapmak: her
  sayım seçeneği tarifte yazılı, varsayılanı kapalı, bir oluşum yoluyla bir kez sayılır.
- **AutoCAD tablosunun bölünmesi (TABLE, “Table Breaks”):** bölme yönü, “üst etiketleri yinele”, en büyük yükseklik,
  aralık. KentOS ilk sürümde yalnız sağa doğru bölmeyi ve “başlık her parçada / yalnız ilkinde” seçimini alır; parçaların
  elle konumlandırılması sınırdır (§9).
- **Çok satırlı yazı aralığı:** AutoCAD'in tek satır aralığı yazı yüksekliğinin 5/3'üdür; KentOS hücre satırlarını da bu
  aralıkla dizer (`geom::table::LINE`).
- Bu oturumda üretici belgeleri yeniden açılmadı; yukarıdakiler planın R07 kaynağından ve AutoCAD yardımının bilinen
  davranışlarındandır. Ücretli ürün çalıştırılmadı.

## Karar

### 1. Kapsam

Var: tarif sözleşmesi ve doğrulaması; saf çıkarım çekirdeği (`kentos_formats::extract`); `cad.extract.run` (yazısız) ve
`cad.extract.table` (tek geri alma adımı “Veri çıkarımı tablosu”); Tabloyu güncelle'nin çıkarım tablosunu yenilemesi;
tablonun parçaları (`breaks`), hücre satır sonu ve sığmayan hücre; `.kcad` şeması 182; tarif dosyası (`.kxtarif`);
masaüstü Veri çıkarımı penceresi ve yerleştirme aracı; Python SDK, MCP, katalog, manifest.

Yok (sınır, §9): pafta kitabında (sheet core) bir tablo çerçevesinin başka paftaya devamı; parçaların elle konumu;
dinamik bloğun görünürlük dışı eylemlerinin (taşıma/germe) gizlediği parçalar; DXF'e ACAD_TABLE olarak parça yazımı.

### 2. Tarif (`kentos_contracts::ExtractionRecipe`)

`name` (boş değil, tek satır, ≤100 harf) · `current` (açık çizim okunur; varsayılan evet) · `drawings` (başka çizimler,
yollar çizimin klasöründen; her biri bir kez, ≤16) · `kinds` (`insert, line, polyline, polygon, circle, arc, ellipse,
spline, hatch, dimension, text, point, leader`; boş: `insert`) · `blocks` (Türkçe büyük-küçük harf katlamasıyla) ·
`layers` (etkin katman adı) · `scope` (§3) · `columns` (1–30) · `group` · `sort` (≤5 anahtar) · `total` · `join` (§5).

Sütun değerleri (`ExtractValue`): `drawing` (dosya adı, açık çizim “Bu çizim”), `kind` (Türkçe tür adı; bütün sayılan
dizi “Dizi”), `block`, `layer`, `variant`, `attribute{tag}`, `count` (adet), `length{unit: m|cm|mm|km}`,
`area{unit: m2|donum|ha}`, `measured{unit}` (ölçünün geometrisinden gerçek uzunluk), `shown` (yazılan metin, sözcük),
`overridden` (evet/hayır), `computed{column, factor, unit}` (önceki bir miktar sütunu × noktalı ondalık katsayı),
`joined{field}`. Başlık etiket ve birimdir: `Uzunluk (m)`, `Adet (adet)`.

Reddedilenler (`invalid_extraction_recipe`, alanın yoluyla): boş ad; açık çizim de başka çizim de yok; tekrar eden
çizim; bilinmeyen tür; boş sütun başlığı; yazı sütununa ondalık; **ölçünün yazılan metnini çarpan hesaplanan sütun**
(“Ölçünün yazılan metni sayı olarak hesaplanmaz; gerçek ölçü için Gerçek ölçü sütununu kullanın”); kendinden sonraki ya
da miktar olmayan sütunu çarpan hesaplanan sütun; geçersiz katsayı; eşleme olmadan eşlenen sütun; aralık dışı ya da iki
kez sıralama; miktar sütunu olmadan birleştirme ya da toplam; yalnız miktar sütunları olan toplam; eşlemenin anahtarı bir
yazı sütunu değilse; eşlenen sütunu olmayan eşleme.

### 3. Sayım kapsamı ve tekil sayım

`CountScope`: `nested` (blokların içine bak), `arrays: items|whole`, `hidden`, `xrefs`. Hepsi açıkça seçilir.

- Her çizimin nesneleri sırasıyla gezilir. `nested` ile bir yerleştirmenin bloğuna girilir: iç yerleştirmeler de sayılır
  (Masa 1, içindeki 4 Sandalye ayrıca 4), parçalar üstteki bütün yerleştirmelerin dönüşümüyle konur (benzerlik değilse
  çekirdeğin afin yerleştirmesi, `block::place`). En çok 16 düzey (`extraction_too_deep`, savunma).
- Dinamik blok varyantı: parçalar `Dynamic::vary_traced` ile değerlerin çizdiği gibidir; görünürlük durumunun gizlediği
  parçalar gizlidir. `variant` sütunu varsayılandan farklı değerleri `ad=değer` (ada göre, `; `) yazar.
- Görünmeyen parça: gizlenen parça ile görünmez ya da dondurulmuş katmandaki (veya grubundaki) nesne `hidden` kapalıyken
  sayılmaz ve sayısı söylenir (`extraction_hidden`). Bloğun katmanındaki (`""`) parça yerleştirmenin katmanındadır.
- İlişkili dizi: `items` her öğeyi kendi yerleştirmesi olarak sayar (elle silinen öğe yoktur); `whole` diziyi ilk öğesinin
  yerinde tek bir “Dizi” oluşumu sayar, öğeleri sayılmaz ve içlerine bakılmaz.
- Dış referans: `xrefs` kapalıyken bir yerleştirmedir; açıkken kendisi sayılmaz, içeriği iç blok gibi gezilir (parçaların
  yolu kaynak kimliğiyle). Yüklü olmayan referansa bakılmaz (`xref_unloaded`). **Tarifin ayrıca okuduğu çizime giden
  referansa (yolu ya da içerik özeti aynı) hiç bakılmaz**: o çizim bir kez, kendisi olarak sayılır (`xref_read_apart`).
- Oluşum yolu: çizimin tarifteki yolu (açık çizim boş), `#`, kalıcı kimlik, her iç parçanın yerel kimliği (`/12`; dış
  referansta `/x:<kaynak kimliği>`); bütün sayılan dizi `#dizi:<kimlik>`. **Bir yol bir kez sayılır.** Satırın `members`'ı
  oluşumlarının yollarıdır; böylece her sayının kaynağı izlenebilir.
- Açık çizimin kendi dosyası tarifte de okunursa ret (`drawing_read_twice`); yol `drawingPath` ile verilir.
- 1 000 000 oluşumdan çok okuyan tarif kesilmez, reddedilir (`extraction_too_large`).

### 4. Değerler, birimler, toplam

- Uzunluk: eğrinin uzunluğu (kapalı çoklu çizginin kapanış kenarı, alanın delik çevreleri dahil), yerleştirilmiş
  geometriden. Alan: kapalı nesnenin delikler çıkmış alanı (kapalı çoklu çizgi dahil); dönüm 1000 m², hektar 10 000 m².
- Gerçek ölçü: ölçünün düzeninin uzunluk değeri; açı ölçüsünde boş (`angles` notu). Yazılan: projenin biçimiyle
  `Format::dimension_value`; **sözcüktür, hiçbir toplamaya girmez.** `<>` içermeyen metin elle yazılmış sayılır
  (`dimension_overrides`).
- Hesaplanan: kaynak sütunun kendi birimindeki değeri × katsayı, oluşum başına.
- Birleştirme: anahtar, eşlenen sütunlar dışındaki yazı sütunlarının U+001F ile birleşimi; satırlar ilk oluşum sırasıyla;
  miktarlar Neumaier telafili toplamla. Boş değer toplamda yok sayılır; hepsi boşsa hücre boştur.
- Sıralama: sözcükler kod noktası sırasıyla, sayılar sayısal; boş değer yöne bakmadan sonda; eşitlikte okuma sırası.
- Toplam: miktar sütunlarının **yuvarlanmamış** oluşum değerlerinin toplamı; “Toplam” ilk yazı sütununa yazılır.
- Hücre metni en sonda `display::fixed` ile yazılır (önce 7 basamakta gürültü, sonra sütunun basamağında, yarım sıfırdan
  uzağa); basamak: sütunun kendi, yoksa projenin uzunluk ya da alan basamağı, adet 0, hesaplanan kaynağınınki.
  Örnek: 43/128 m'lik üç çizgi tek tek 0.34, toplamı 1.0078125 → **1.01** (yuvarlanmış hücrelerin toplamı 1.02 olurdu).

### 5. Dış veriyle eşleme

`join{path, sheet?, key, on}`: CSV/XLSX/ODS dosyasının ilk satırı başlıklardır; `key` başlıklı sütunun her satırında
boş olmayan, tekil bir anahtar olmalıdır (yoksa `join_unusable`). Satırın `on` sütununun kırpılmış metni anahtarla
eşlenir; eşlenen sütunlar o satırdan doldurulur. Dosyada olmayan anahtarlar `notes.unmatched` ve `join_unmatched`
uyarısı, satır kullanmayan dosya anahtarları `notes.unused`. Satır sırası değil anahtar kullanılır.

### 6. Çıkarım tablosu ve yenileme

`TableSource::Extraction(ExtractionSource{recipe, read, keys, copy, overrides})`: okunan dosyaların SHA-256 özetleri,
her veri satırının anahtarı (birleştirilmiş satırda yazı sütunları, değilse oluşum yolu) ve son çıkarılan hücreler
(yalnız tarifin sütunları; tablonun sonraki sütunları kullanıcınındır). Tablo her adımda yeniden yazılmaz (okuduğu
çizimler dosyadır); Tabloyu güncelle (`cad.tables.edit` `refresh`) yazar:

- kaynağı değişmemiş elle düzeltilmiş hücre korunur (`kept`); kaynağı da değişmişse `keep` kuralında tablo olduğu gibi
  kalır (`refresh_kept`), `replace`'de kaynak yazılır (`replaced`);
- kullanıcının ek sütunları, biçimleri, formülleri, alanları ve satır yükseklikleri anahtarla taşınır; **toplamın bütün
  veri satırlarını kapsayan formül aralığı satırlar çoğalınca ya da azalınca onlarla büyür/küçülür**
  (`table_formula::remap_rows_spanning`); bölünen aralık, kalkan satıra başvuru, satırlara yayılan birleşim reddedilir;
- kullanıcı verisi olan satır kaynaktan kalkarsa `keep`'te tablo korunur;
- tabloya elle satır eklenmiş ya da silinmişse anahtarlar eşlenemez: tablo korunur;
- okunan çizim yoksa `missing`, okunamıyorsa `unreadable`.

`cad.extract.run` bir tabloyla çağrılınca `stale` kaynakların değişip değişmediğini söyler. `file.export.table`
çıkarım tablosunu tablonun hücreleriyle yazar.

### 7. Tablonun hücre satırları ve parçaları (`.kcad` şeması 182)

- **Hücre satırı:** hücrede `\n` olabilir (en çok 20 satır; başka denetim karakteri yine ret). Düzen her satırı ayrı
  koyar (`CellPlace.line`), satırlar 5/3 yükseklik arayla, blok kutunun ortasında. `SetCell` satır sonunu korur (çekirdeğin
  tek satır kuralı ADR 0184 §6 dosyadan gelen hücreler için sürer). Yazıya sığdır'ın satır derinliği 2 yükseklik + her
  ek satıra 5/3 (`fit_depth`). Dosya kopyası, anahtarlar ve çıkarım kopyası tek satırdır (`line_problem`).
- **Sığmayan hücre:** satırı kutudan (iki yanda yarım yükseklik dışında) geniş ya da satırları kutudan derin olan hücre
  (`TableGeom::overflowing`).
- **Parçalar (`TableBreaks{height, gap, once}`):** bir parça satırları sığdıkça alır (en az bir, ilki başlık ve bir satır
  daha); birleşik alan bölünmez (parça alanın başında biter, olmuyorsa sonunda; bir kez ilerledikten sonra yalnız ilerler);
  başlık her parçanın üstünde yinelenir (`once` ya da birleşik başlıkta değil); parça k, k·(genişlik+aralık) sağdadır.
  Her parça kendi satırlarının tablosu olarak çizilir (çizgiler, çerçeve şeridi, hücreler); dış çizgi bütün parçaları
  kapsar. Mağaza paketi tür 38 (`store/pack.rs`); DXF yazıcısı parçalı ya da çok satırlı tabloda yazıları çekirdeğin
  yerleşiminden yazar.
- `.kcad` 182: `breaks` JSON'u, satır sonlu hücre, `schedule`'ın `kind: extraction` biçimi; 181 ve öncesi
  `breaks`'i ve çıkarım kaynağını `unknown_field`, satır sonunu `bad_value` ile reddeder.

### 8. Arayüz

- **Veri çıkarımı** (`table.extract`, ikon `dataExtraction` — huni ile beslenen tablo, yeni; takma adlar
  `VERICIKARIMI`, `DATAEXTRACTION`, `DX`): Açıklama › Tablo panelinin taşmasında. Pencere `RoadDialog` iskeletinde:
  Kaynak (tarif adı, Bu çizim, Çizimler + “Çizim ekle…”), Süzgeç (tür anahtarları, bloklar, katmanlar), Sayım kapsamı,
  Sütunlar (her sütun bir anahtar; öznitelik adı, katsayı ve birim alanları), Satırlar, Dış veri (+ “Tablo seç…”), Tablo
  (parça derinliği, aralık). Sağda satır/oluşum/okunan dosya/toplam rakamları, uyarılar ve satırların tablosu; her
  değişiklikte `cad.extract.run` yeniden okunur, çizim değişmez. Altta “Tarifi kaydet…”, “Tarif aç…” (`.kxtarif`), Kapat
  ve **Tabloyu yerleştir**.
- **Yerleştirme aracı** (`kentos_interaction::extract_place`): tablo `cad.extract.table`'ın planından imlece asılır
  (parçalar, kesik çizgiler, soluk yazılar), tıklama ya da yazılan nokta aynı komutla yazar.
- Öznitelikler'in kaynak satırı “Veri çıkarımı “ad”: n satır, bu ve k başka çizimden”.

### 9. Sınırlar (sonraki iş)

- Pafta kitabında bir tablo çerçevesinin başka paftaya devamı (`ContinueIn` yalnız aynı paftada): CAD-ADV-16 (pafta ve
  yayın akışı). Bu maddede parçalar model tablosunda yan yana durur; her parça ayrı bir görünüş çerçevesine konarak
  paftalara dağıtılır.
- Parçaların elle konumu, aşağı/sola bölme, parça başına “nakli yekün” satırı.
- Dinamik bloğun görünürlük dışındaki eylemlerinin parça gizlemesi (`vary_traced` taşır, gizlemez).
- Önizleme arayüz iş parçacığında çalışır (ortak durumlarda birkaç ms); büyük çizim kümesi için iş parçacığı sonraki
  iştir.
- Veri çıkarımı tablosunu masaüstü tablo düzenleyicisinde çok satırlı yazma (Shift+Enter) — komut ve Python/MCP satır
  sonunu alır, düzenleyicinin hücre girişi tek satırdır.

### 10. Web'e ertelenenler (TODOS.md §0.2 için)

`cad.extract.run/table` işleyicisi ve ortak durumları; `TableSource.extraction`, `TableEntity.breaks`, satır sonlu hücre
için `io/kcad.ts` (şema 182) ve `io/columns.ts` (tablonun JSON'u); `tableLayout`'un `line` alanı ve parçalar (WASM
yeniden derlenmeli); mağaza paketinin tür 38'i; `FORMATS_VERSION` 107; Veri çıkarımı penceresi; `version.ts`.

## Uygulama

- Sözleşme: `crates/shared/contracts/src/extraction.rs` (tarif, kapsam, sütunlar, birimler, kaynak, kurallar);
  `table.rs` (`TableSource::Extraction`, `TableBreaks`, `breaks`, `cell_problem` satır sonu, `line_problem`);
  `table_formula.rs` (`remap_rows_spanning`); `codes.rs` (15 kod); `formats.rs` (107).
- Komutlar: `crates/shared/commands/src/cad_extract.rs`; `cad_tables.rs` (`TableEdit::Breaks`); katalog, yetenekler.
- Çekirdek: `crates/shared/formats/src/extract.rs` (gezinti, değerler, eşleme, birleştirme, sıralama, toplam, tarif
  dosyası); `crates/shared/geometry-core/src/geom/table.rs` (parçalar, satırlar, sığmayan hücre, `fit_depth`), `entity.rs`
  (`TableBreaks`), `store/pack.rs` (tür 38), `store/labels.rs`, `store/legible.rs` (satır), `ops/transform.rs`.
- Uygulama: `crates/native/application/src/extract.rs` (kaynakları okuma, `run`, `table`, `refresh`), `tables_edit.rs`
  (yenileme kolu, `Breaks`, çok satırlı `SetCell`, satırlı `Fit`), `geometry.rs`.
- Dosya biçimi: `crates/shared/kcad` (şema 182); `tools/kcad/kcad.py`, `scripts/fixtures/kcad_v2_reference.py`;
  `fixtures/kcad/v2/table-extraction.{json,kcad}`, üç eski şema bozuk örneği; `docs/specs/kcad-v2.md`.
- DXF: `crates/shared/formats/src/dxf/writer/{entities,input}.rs`.
- Masaüstü: `apps/desktop/src/tables/extract.rs`, `tables/mod.rs`, `app.rs`, `view.rs`, `labels.rs`, `catalog.rs`;
  `crates/native/interaction/src/extract_place.rs`, `spatial.rs`, `table_place.rs`, `tag_tools.rs`; `crates/native/plot/
  src/labels.rs`; `catalog/v1/{additions,operations,ribbon}.json`, `scripts/ui/ribbon.py`.
- Otomasyon: Python SDK `kentos.cad.extract` (üretildi), MCP testi.

## Doğrulama

### Bağımsız başvurular

- `scripts/fixtures/extract_command_cases.py --check` (Python standart kütüphanesi ve bağımsız KCAD yazıcısı, KentOS
  kodu yok; geometri ikili kesirli seçildi, böylece çekirdeğin kayan noktası da kesin): çıkarımın bütün kuralları
  yeniden yazıldı (gezinti, iç içe yerleştirme dönüşümü kesirlerle, görünürlük, gizli katman, dizi, dış referans,
  tekil yol, eşleme, birleştirme, sıralama, toplam, gösterim kuralı). `fixtures/commands/v1/cad.extract.run.json`
  **31 durum**, `cad.extract.table.json` **47 durum** (tablo, yenileme, `Breaks`, çok satırlı `SetCell`, satırlı
  `Fit`); ikinci çizim `fixtures/commands/v1/extract/kat2.kcad` bağımsız yazıcıyla yazıldı.
- Kabul satırının sayıları (betikte ayrıca sabitlendi): iki çizimden, iç içe bloklar, iki dizi, gizli katman ve dış
  referansla parça listesi **Kapı 3, Masa 3, Sandalye S-01 23, Sandalye S-02 2, Toplam 31**; Kat2 dış referansı ayrıca
  okunan çizim olduğu için içine bakılmadı (`xref_read_apart`); gizli katmandaki sandalye sayılmadı (1). Metraj: üç
  43/128 m duvar çizgisi **1.01** (yuvarlanmış hücrelerin toplamı 1.02 olurdu), gerçek ölçü toplamı **7.000** (elle
  yazılmış “5.00” 4 m'lik ölçüde metin olarak kaldı; 5+3 = 8 değil), ağırlık 7.5 kg/m ile **7.56**.
- `scripts/fixtures/table_parts_cases.py --check` (kesirler): `fixtures/table/v1/parts.json`, **10 düzen** (başlık her
  parçada / yalnız ilkinde, aralık, çerçeve şeridi, yalnız dış çizgi, bölünmeyen birleşik alan, sığmayan tek satır,
  başlıksız, tek parça, bölmesiz, çok satırlı ve boş satırlı hücre), sığmayan hücre ve satır derinliği durumları;
  `geometry-core` `tests/all/table_parts.rs` 2 test.
- Dış dosyalar salt okunur: MCP testi çıkarım ve yenilemeden sonra `kat2.kcad` ve `aciklama.csv` baytlarının
  değişmediğini, çizimin `run`'dan sonra baytı baytına aynı kaydedildiğini denetler.

### `.kcad`

`python3 scripts/fixtures/kcad_v2_reference.py --check` (1581 dosya); örnek `table-extraction.kcad` Rust yazıcısıyla
bayt bayt aynı; üç eski şema bozuk örneği (`unknown_field`, `bad_value`, `unknown_field`) iki okuyucuda aynı kodla
reddedilir; `kentos-kcad` 236 + 6 test.

### Testler (bu oturumda, hedefli)

| Takım | Sonuç |
|---|---|
| `kentos-native-application --test all fixtures` (bütün ortak durumlar; `origin/main` 2d55a0a31 + b41492c6c üstüne yeniden kurulduktan sonra) | bütün durumlar geçti; test yalnız `main`'in kendi açığıyla düşüyor: `cad.standards.conflicts` için `fixtures/commands/v1`'de dosya yok (bu maddeden değil). Yeniden kurmadan önce düşen `cad.xrefs.list › değişti: göreli yolda…` `main`'de düzelmiş |
| aynı, `cad.tables`, `cad.extract`, `cad.entities.create` | geçti (`cad.tables.edit › Güncelle: bağlı dosya…` bugünkü çakışma kuralına göre düzeltildi, aşağıda) |
| `capabilities` (4 test) | geçti |
| `kentos-commands` (1527), `kentos-contracts` (1259 + 34 + 10) | geçti |
| `kentos-geometry-core` (624 + 350 + diğerleri) | geçti |
| `kentos-formats`, `kentos-domain`, `kentos-kcad` | geçti; `formats/tests/gis.rs › exports_are_written_as_committed_and_read_back` bu maddeden önce de bu makinede düşüyor (yay örnekleme koordinatlarında son basamak farkı; dokunulmadı) |
| `kentos-mcp --test mcp data_extraction` | geçti |
| `kentos-desktop --lib extract_window_tests`, `--lib table` (57) | geçti |

Düzeltilen eski beklenti: `cad.tables.edit.json`'un “Güncelle: bağlı dosya, elle değişen hücreler korunur” durumu
5 Kasım'daki çakışma kuralından (`e2df89d71`) sonra yanlıştı (kaynak ve kullanıcının aynı hücreyi değiştirdiği `keep`
yenilemesinde tablo artık korunur). Üreteci `table_command_cases.py` aynı kurala göre değiştirildi ve JSON aynı biçimde
güncellendi; betik bu makinede `mpmath` olmadığı için yeniden çalıştırılamadı (`--check` mpmath olan makinede koşmalı).
`create_command_cases.py`'nin “satır sonu reddi” durumu satır sonu artık hücrenin olduğu için sekme karakteriyle
yazıldı (betik çalıştırıldı, `--check` geçti).

### Çalıştırılmayanlar

- `pnpm py:test`: masaüstünün `.run/py` sanal ortamı bu çalışma ağacında yok; kurmak paket indirmek demek. SDK
  `scripts/python/sdk.py` ile üretildi; `test_command_cases.py` yeni ortak durumları oynatmadı.
- `table_cases.py --check` ve `table_command_cases.py --check`: `mpmath` yok (yukarıda).
- Tam `pnpm rust:test` / `rust:test:desktop`: sahibin “yalnız hedefli test” kuralı; izler (`traces`) koşmadı.
- Süre ölçümü: release derlemesi yapılmadı. Debug ortak durumlarda önizleme (iki çizim, 31 oluşum) milisaniyeler
  içinde; büyük çizim kümesinde ölçüm sonraki iştir (§9).

### Görsel inceleme

`extract_window_tests::screens` (1440×900 açık, 1100×650 koyu): pencere, metraj penceresi, üç parçalı tablo. Bulunup
düzeltilen: başlıkta birim iki kez (“Adet (adet)” → “Adet”); aynı çizicide pencereyi yeniden açınca anahtarların eski
konumla çizilmesi (resim düzeneği, yeni çizici); yeni çizicide doğrudan komutla yazılan tablonun çizgilerinin ilk
karede görünmemesi (resim düzeneği: aynı çizici). Koyu darda metraj tablosu yatay kayar (ağırlık sütunu sağda).
