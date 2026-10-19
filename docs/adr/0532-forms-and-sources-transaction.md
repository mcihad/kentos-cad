# ADR 0532: Formun bağlı kaynaklarda işlem sınırı, çoktan çoğa ilişki profili ve sayfalı kayıt okuma

- **Durum:** kabul edildi (2026-10-19). `plan/gis.md` CBS-15'in ilk tam dilimi (GeoPackage bağlı katman; PostGIS ve sunucu
  ayrı).
- **Bağlam belgesi:** ADR 0405 (ilişkiler), 0408 ve 0409 (çok değerli başvuru, çoktan çoğa), 0440 (kaydın alt kayıtlarıyla
  gönderimi), 0444 (ilişkili kayıtlar, `requires`), 0503 ve 0510 (bağlı katman, alan sınırlı çalışma kopyası), 0512 (geniş
  değerler), 0253 (komut protokolü), 0211 (katman süzgeci), 0527 (birleşik görünüm).
- **Web:** ertelendi (ADR 0228). Web'de gerekenler TODOS.md §0.2'de.
- **`.kcad` şeması:** değişmez. Eşleme dosyanın kendisinde durur (§2); projeye yeni alan yazılmaz.
- **Sonraya kalan:**
  - PostGIS'te birden çok tabloyu tek işlemde yazmak (`TableProvider::edit` bir tabloyu yazar; `with` PostGIS'te
    `transaction_split` der) ve PostGIS'te çoktan çoğa için bağlantı tablosu (sqlx'li sağlayıcının `edit_many`'si);
  - ayrı dosyalardaki katmanların birlikte yazılması (iki SQLite dosyasında tek işlem yoktur; `transaction_split` ile
    reddedilir, iki aşamalı işlem tasarlanmadı);
  - dosyada KentOS ilişkisi olan tabloyu ilk bağlarken çok değerli alanın kendiliğinden dolması: dosya ilişkinin hangi alana
    ait olduğunu bilmez, alan `cad.sources.sync read` ile dolar (§2'nin sonu);
  - bulutta sayfa sayfa okumanın izinlere göre süzülmesi (toplam dahil): yerelde erişim politikası katman süzgeçleridir
    (§3); roller ve kurum izinleri ADR 0440'ın kalanıdır;
  - formun arayüzde başka katmandan listeleri ve ilişkili kayıtları `cad.records.page` üzerinden okuması (arayüz bugün
    belgeyi doğrudan okur; komut aynı verinin Python ve MCP yüzüdür);
  - ölçekli yazma: `commit`'in planı çizimin kopyası üzerinde kurulur (`sources::sync_plan` ile aynı yol), çok büyük
    çizimde pahalıdır.

## Bağlam

CBS-15, formun kayıt ve alt kayıt gönderimini (ADR 0440) dış bir veri kümesinin işlem sınırına bağlamayı ister:

1. Bir üst kayıt ve alt kayıtları bağlı bir GeoPackage'a yazılırken ya hepsi ya hiçbiri; bir alt kayıt reddedilirse üst kayıt
   kısmen yazılmamalı. `cad.sources.sync` bir katmanı bir SQLite işleminde yazıyordu; üst ve alt katman iki ayrı işlemdi, ikinci
   işlemin reddi birincinin yazılmış kalması demekti.
2. Çok değerli yerel başvuru (alt kayıt birden çok üst kaydın kimliğini `; ` ile tutar, ADR 0409) dosyada ya noktalı virgüllü
   metin sütunu ya da GeoPackage'ın ilişki profili (OGC 18-000: `gpkgext_relations` ve bir eşleme tablosu) olabilir. Bilinmeyen
   bir temsil gizlice metne çevrilmemeli.
3. Başka bir veri kümesinden seçenek ve ilişkili kayıtlar sayfalı ve kaynağın sürümüyle okunmalı; toplam sayı dahi erişim
   politikasından geçmeli.
4. Bina → parsel → malik örneği harita, tablo, form, Python ve MCP'de aynı sonucu vermeli.

Örnek (`fixtures/geopackage/v1/iliskili*.gpkg`): `malik` (öznitelik tablosu), `parsel` (çokgen; `malikler` alanı sahiplerin
kimlikleri), `bina` (çokgen; `parsel` alanı parselin kimliği). Projenin ilişkileri: `parsel-malik` (üst: malik, alt: parsel,
alan: `malikler`, çok değerli) ve `bina-parsel` (üst: parsel, alt: bina, alan: `parsel`, tek kimlik).

## Karar

### 1. Birlikte yazma: `with` ve `commit`

- **`cad.sources.sync`'in `with`'i** (yalnız yazma): asıl katmanın yanında başka bağlı katmanlar aynı işlemde yazılır.
  `GeoPackage::edit_many` (kentos-geopackage) birkaç tabloyu ve eşleme tablolarını tek SQLite işleminde yazar; hata olursa
  hiçbiri yazılmaz.
- Her katman yazmadan önce denetlenir: salt okunur tablo (`source_readonly`), dosya katmanlar onunla son eşleştiğinden beri
  değişmiş (`source_changed`), tablonun tutamadığı geometri (`geometry_mismatch`), tabloda olmayan kayda başvuru
  (`relation_dangling`). Biri reddedilirse hiçbiri yazılmaz; ret ilk reddeden katmanın yoludur (`layer` ya da `with`).
- Katmanlar tek GeoPackage dosyasında olmalı: ayrı dosya ya da veritabanı `transaction_split`; aynı katman iki kez
  `invalid_input`; `with` okumada `invalid_input`. Cevabın `layers`'ı her katmanın payı (eklenen, değişen, silinen), üstteki
  sayılar toplamlarıdır; `relations` eşli ilişkilerin eşleme satırlarıdır.
- Yazmadan sonra dosyanın SHA-256'sı değişir: yazılan katmanların bağları (sürüm ve özet) ve dosyanın dokunulmamış öteki bağlı
  tabloları (bağın sürümü yazmadan önceki dosyanınsa) yeni sürüme taşınır. Dokunulmayan tablo dosyayla eşleşmeye devam eder;
  bugüne dek ikinci tabloyu yazmak `source_changed`'ti.
- **`cad.forms.submit`'in `commit`'i:** kaydın ve alt kayıtlarının katmanları (kaydınki önce) bağlıysa gönderim aynı adımın
  ardından dosyaya yazılır (`sources::commit`, `with` ile aynı yol). Plan ve doğrulama, gönderimin yazıldığı çizim kopyasında
  dosyanın yazmasını da planlar: dosyanın reddi gönderimin reddidir, ne çizimde ne dosyada bir şey yazılmaz. Cevabın
  `committed`'ı dosyaya yazılandır. Planın denetlediği ile yazmanın arasında dosya değişirse (yarış) çizim bir geri alma adımıyla
  eski hâline döner ve ret söylenir. Hiçbir katman bağlı değilse `source_not_found` (yol `commit`); bağlı olmayan katman
  yazılmaz.

### 2. Çoktan çoğa başvuru: GeoPackage ilişki profili

- **Temsil.** Alt katmanın çok değerli alanı iki katman aynı GeoPackage dosyasının tablolarına bağlıyken iki biçimde tutulur:
  - **metin:** alanın sütunu, kimlikler `; ` ile birleşmiş (bugüne dek olan);
  - **eşli:** OGC 18-000'in `gpkgext_relations` satırı (`relation_name` `kentos_references`, taban tablo alt katmanın tablosu,
    ilgili tablo üst katmanınki, anahtar sütunlar tabloların fid'i) ve `base_id`/`related_id` eşleme tablosu; satırlar alt
    kaydın fid'i ile üst kaydın fid'idir. Eşleme tablosu tek temsildir: alan sütun olarak yazılmaz (tabloda sütun varsa tabloda
    ne tutuyorsa onu tutar, yeni sütun açılmaz) ve sütundan okunmaz.
  - Eşleme dosyanın kendisinde durur (`gpkgext_relations`'ın satırı): proje ayarına alan eklenmez, `.kcad` şeması değişmez.
- **`cad.sources.relate`** (`check`, `map`, `unmap`):
  - `check` ilişkinin dosyadaki durumunu söyler: `unbound`, `split` (ayrı dosyalar), `single` (tek kimlikli alan), `text`,
    `mapped`, `foreign` (dosyanın başka temsilli ama biçimli ilişkisi, nedeniyle), `broken` (profile uymayan: eşleme tablosu
    yok, `base_id`/`related_id` yok ya da tam sayı değil, tablo ya da anahtar sütun dosyada yok), ters yönde yazılmış ilişki
    `foreign`'dır;
  - `map` iki tabloyu, eşleme tablosunu (adı verilebilir, varsayılan iki tablonun adı) ve satırlarını (katmanların alandaki
    başvurularından, alt kaydın fid sırasıyla, alanın sırasıyla; yinelenen kimlik tek satır) tek işlemde yazar, bağların sürümünü
    taşır; dosyada `foreign`/`broken` ilişki varsa `relation_unsupported` (üzerine yazılmaz, metne de çevrilmez), eşli ise
    `relation_mapped`, eşleme adı ad değilse ya da dosyada o adda tablo varsa `invalid_mapping`, tabloda olmayan kayda başvuru
    `relation_dangling`;
  - `unmap` ilişkiyi, eşleme tablosunu ve uzantı satırlarını dosyadan kaldırır (son ilişki gidince `gpkgext_relations` de gider:
    dosya ilk hâline döner); başvurular katmanda kalır ve sonraki yazmada metin sütunu olur.
- **Yazma ve okuma.** Eşli ilişkinin katmanlarından biri yazılırken (`with`'li ya da tek) eşleme satırları katmanın alanından
  kurulur ve katmanlarla aynı işlemde yazılır; eşit satır kümesi yeniden yazılmaz. Yazılmakta olan tabloda olmayan yeni nesne
  eşleme satırında yazma sırasında verdiği fid'le (`Key::Inserted`) yer alır. Alan sınırlı çalışma kopyasında (ADR 0510) alanın
  dışındaki satırların eşleme satırları korunur. Okuma (`direction read`) çok değerli alanı eşleme satırlarından doldurur; satır
  tabloda olmayan bir üst satıra gidiyorsa `relation_dangling` (dosya onarılmadan okunmaz).
- **Bilinmeyen temsil gizlice çevrilmez.** Eşlenmemiş çoktan çoğa başvuru metin sütunu olarak yazılır ve her yazmada
  `relation_unmapped` uyarısı (adıyla) söylenir; dosyanın başka temsilli ilişkisinin eşleme satırlarına dokunulmaz ve onlardan
  alan doldurulmaz, aynı uyarı nedeniyle söylenir.
- **`cad.sources.describe`** dosyanın tablolarında ilişkileri de listeler: ad, taban, ilgili, eşleme tablosu, satır sayısı,
  tür (`kentos`, `foreign`, `broken`) ve bozuksa nedeni.
- **Sınır:** dosyada KentOS ilişkisi varken proje ilişkisi sonradan tanımlanırsa katmanın alanı boştur; yazma eşleme satırlarını
  katmanın hâline getirir ve planda silinecek satırlar `relations.removed` ile görünür. Önce `cad.sources.sync read` alanı
  doldurur. Dosyanın ilişkisi alanın adını taşımaz; bu ADR onu dosyaya yazmaz (sonraya kalan).

### 3. Sayfalı, sürümlü okuma: `cad.records.page`

- Bir katmanın kayıtları (`layer`) ya da bir ilişkinin bir kayda bağladığı kayıtlar (`relation` ve `of`; `side` `children`:
  `of` üsttür, alanı onu tutan alt kayıtlar; `parents`: `of` alttır, üst katmanın alanın tuttuğu kayıtları, alanın sırasıyla),
  1–500'lük (varsayılan 50) sayfalarda, kalıcı kimlik sırasıyla; `cursor` önceki sayfanın son kimliği.
- `total` sayfalardan bağımsız eşleşen kayıtlardır ve **katman süzgeçlerinden sonra** sayılır (ADR 0211): süzgecin dışarıda
  bıraktığı nesne ne kayıt ne toplam olarak görünür. Yerelde erişim politikası budur; kurum izinleri sonraya kalır.
- `query` kaydın öznitelik değerlerinde bütün sözcükleri arar (büyük küçük harf Türkçe kuralıyla denk: I ı, İ i; harfler
  sadeleştirilmez); `fields` yalnız adı verilen öznitelikleri döndürür.
- Katman bir kaynağa bağlıysa `source` kaynağın sürümünü söyler: bağın kimliği, tablo, `revision` (katman kaynakla son
  eşleştiğinde dosyanın SHA-256'sı) ve `edited` (katmanda kaynağın bilmediği değişiklik var: sayfa çalışma kopyasınındır).
  `expectedSource` ve `expectedRevision` ilk sayfanın söylediklerini geri verir: kaynak ya da çizim değiştiyse sonraki sayfa
  okunmaz (`source_changed`, `revision_conflict`; çakışma), sayfalar aynı verinin sayfalarıdır.

### 4. Protokolün planı dosya yazmaz

Protokol bir komutun planının özetini ve plan kimliğini, komutun `execute`'unu bir okuma kopyasında çalıştırarak bulur (ADR
0253 §5). Dosya yazan komutlarda bu, planın dosyayı gerçekten yazması demekti: `cad.sources.sync` ve `cad.processing.run`'ın
`sink`'i buna yakalanmıştı, MCP ve Python'da `plan` çağrısı dosyayı yazıyordu (bu dilimin MCP sınaması bunu buldu).
`ExecutionContext::dry()` o kopyada çalışan komutu söyler (`CallState.dry`); `cad.sources.sync` (yazma), `cad.sources.relate`,
`cad.forms.submit`'in `commit`'i ve `cad.processing.run`'ın sink'i `dry`'da dosyaya yazmaz, yazılacağını planlar.
Regresyon: `a_plan_through_the_protocol_writes_no_file_but_its_execute_does`.

### 5. Masaüstü

- **İlişkiler** penceresi (ADR 0405): kayıtlı ilişkinin altında **Dosyada** bölümü: durum (metin sütunu, eşli: tablo ve satır
  sayısı, başka temsil ve nedeni, ayrı kaynaklar, bağlı değil) ve **Dosyaya eşle** / **Eşlemeyi kaldır** (dosyaya hemen yazar;
  kaydedilmemiş değişiklik varsa önce Kaydet denir).
- **Formla düzenle:** kaydın katmanı bağlıysa altbilgide **Kaynağa da yaz** düğmesi; Kaydet gönderimi `commit` ile yapar,
  cevap dosyaya yazılanı katman katman söyler, ret gönderimin retti olarak görünür.
- **Katmanlar › Kaynak ▸ Kaynağa birlikte yaz** (`source.writeTogether`): katman ve proje ilişkilerinin ona bağladığı, aynı
  dosyadaki bağlı katmanlar tek işlemde; ret tümünü durdurur ve söylenir.
- Resimler: `.run/shots/arac-kaynak-iliski-metin-*`, `-eslendi-*`, `-yabanci-*`, `arac-kaynak-form-commit-*`, `-yazildi-*`,
  `arac-kaynak-birlikte-*`, `-reddedildi-*`; 1440×900 ve 1100×650, koyu ve açık (`-acik`).

## Doğrulama

- **Ortak durumlar** (KentOS kodu olmadan, `scripts/fixtures/source_relations_cases.py`; dosyalar
  `source_relations_files.py`'nin sqlite3 ile yazdığı `iliskili*.gpkg`):
  `cad.sources.relate.json` 42, `cad.records.page.json` 41, `cad.sources.sync.together.json` 25,
  `cad.forms.submit.commit.json` 20 durum. Durumun beklentisi dosyanın tablolarından, ilişkinin tanımından ve ADR'nin
  sırasından kurulur (eşleme satırları çocuğun fid sırasıyla, alanın sırasıyla; sayfa kimlik sırasıyla). Dosyaya yazan
  `execute` ortak durumlarda yok (fixture dosyası değişirdi); onu `crates/native/application/tests/all/source_relations.rs`
  geçici kopyada sınar.
- **Yazılan dosyalar bağımsız okuyucuyla** (`python3 scripts/fixtures/source_relations_cases.py --verify`): GDAL 3.13'ün
  ilişki arayüzü KentOS'un yazdığı ilişkiyi `ManyToMany` ilişkilendirme olarak okur (`parsel` → `malik`, `base_id`/`related_id`);
  `validate_gpkg.py` üç dosyada da hatasız (kaldırılan eşlemeden sonra dosya ilişki öncesi hâlindedir), kasten bozuk
  `iliskili-bozuk.gpkg`'de ise aynı eksiği bulur; eşleme satırları, tabloların satırları ve `kentos_uids`; eşlenmiş alanın sütunu
  eşliyken yazılmaz.
- **Araçlar:** Python (`python/tests/test_source_relations.py`) ve MCP (`crates/native/mcp/tests/source_relations.rs`) örneği baştan
  sona: bağla, eşle, sahipleri sayfa sayfa oku, parsel ve sahibi tek işlemde yaz, form kaydı `commit`'le; dosya sqlite3 ile
  okunur. Python'un tipli katmanı ortak durumları da koşar.
- **Masaüstü:** `source_relation_scenes::tests` (pencereden eşle ve kaldır, yabancı ilişki, formun `commit`'i, birlikte yaz ve
  reddi).
- **Ölçüm** (`page_timing`, 100 000 kayıtlı katman, release): 50'lik sayfa 3 ms; arama 32 ms; bir kaydın 30 000 alt kaydı 15 ms.
  Eşleme ve birlikte yazma ADR 0503'ün farkının üstüne aynı yürüyüşü ekler (eşleme satırları kimlik sözlükleriyle, doğrusal).

## Sonuçlar

- Bir üst kayıt ile alt kayıtları dosyaya ya hep birlikte ya hiç yazılır; reddedilen alt kayıt üst kaydı yazılı bırakmaz, çizimi
  de.
- Çoktan çoğa başvuru GeoPackage'da standart ilişki olarak (QGIS ve GDAL okur) ya da açıkça metin olarak durur; biri ötekine
  gizlice dönmez, tanınmayan temsil söylenir ve üzerine yazılmaz.
- Başka katmanın kayıtları sayfa sayfa, kaynağın sürümüyle ve süzgeçlerin sayısıyla okunur; Python, MCP ve arayüz aynı veriyi
  görür.
- Protokolün planı dosya yazmaz (önceden yazan `cad.sources.sync` ve sink'in düzeltilmesi bu dilimin yan kazancıdır).
