# ADR 0758: Katmanın dizinleri ve birden çok alanlı anahtarları

- **Durum:** kabul edildi (2026-10-30).
- **İş:** plan/form.md `FORM-DATA-03`'ün kalanı: “Primary/global UUID, unique keys, nullable, required, type, default,
  precision/length/range ve indexing alanlarını tanımla.” Kalıcı kimlik, tür, zorunlu, varsayılan, uzunluk, basamak, aralık ve
  tek alanın tekilliği vardı (ADR 0014, 0199, 0438); bağlı tablonun birden çok sütunlu tekil kısıtı tablodan okunuyordu (ADR
  0755). Bağlı olmayan katmanda birden çok alanlı anahtar ve dizin yoktu.
- **Web:** web dururken (ADR 0228) yalnız masaüstü, başsız sunucu, Python ve MCP; web'e gerekenler TODOS.md §0.2'de.
  **`.kcad`:** şema 136 (opt-in): katmanın `indexes`'i.
- **Bağımlılık:** yeni kitaplık yok.

## Bağlam

Bir katmanın kayıtlarında bir alan tek başına kaydı ayırt etmeyebilir: parselin numarası adada tekildir (ada + parsel), bir
bağımsız bölümün numarası binada (bina + bölüm). ADR 0438'in tekil alanı bunu söyleyemez; bağlı tablonun birden çok sütunlu
tekil kısıtı (ADR 0755) yalnız tablodan gelir. Bir alanda çok aranan değer (mahalle adı, ada numarası) için veritabanında
dizin kurulur; KentOS'ta tanımlanan katmanın dizini GeoPackage'a yazılırken yoktu, okunan GeoPackage'ın dizinleri katmana
geçmiyordu.

Üreticilerin yolu (30 Ekim 2026'da okundu):

- ArcGIS Pro, *Add Attribute Index*: “Attribute indexes are used by ArcGIS to quickly locate records that match an attribute
  query.” Alanlar (bir ya da birçoğu: birleşik dizin), ad, **Unique** (dizinin her değeri farklı) ve Ascending; BLOB, raster ve
  geometri alanına dizin kurulmaz; tekil ve artan dizin dosya geodatabase'inde ve Shapefile'da desteklenmez.
  <https://pro.arcgis.com/en/pro-app/latest/tool-reference/data-management/add-attribute-index.htm>
- QGIS, *Create attribute index* (`native:createattributeindex`): “Creates an index against a field of the attribute table to
  speed up queries. The support for index creation depends on both the layer’s data provider and the field type.”
  <https://docs.qgis.org/latest/en/docs/user_manual/processing_algs/qgis/vectorgeneral.html>
- SQLite (GeoPackage), *CREATE INDEX*: `UNIQUE` yinelenen girdiyi reddeder; “For the purposes of unique indices, all NULL values
  are considered different from all other NULL values and are thus unique”; kısmi dizin `WHERE` ile, ifade dizini sütun yerine
  ifadeyle. `PRAGMA index_list` dizinin tekilliğini, kaynağını (`c` CREATE INDEX, `u` UNIQUE kısıtı, `pk` birincil anahtar) ve
  kısmi olup olmadığını; `PRAGMA index_info` sütunlarını söyler, ifade sütununda `cid = -2`.
  <https://www.sqlite.org/lang_createindex.html>, <https://www.sqlite.org/pragma.html>
- PostgreSQL, *CREATE INDEX*: `UNIQUE` dizin kurulurken var olan veride ve her eklemede yinelenen değeri denetler (var olan veri
  yineliyorsa dizin kurulmaz); `NULLS NOT DISTINCT` boşları aynı sayar; “Indexes can have up to 32 columns”. *ALTER TABLE ...
  DROP COLUMN*: “Indexes and table constraints involving the column will be automatically dropped as well.”
  <https://www.postgresql.org/docs/current/sql-createindex.html>, <https://www.postgresql.org/docs/current/sql-altertable.html>

## Karar

### 1. Katmanın dizinleri

Katman düğümünün `indexes`'i (sözleşmenin `LayerIndex`'i, `kentos_contracts::layer_indexes`; `.kcad` şema 136, yalnız varsa):

- `name`: 1–64 karakter, başında ve sonunda boşluk ve denetim karakteri olmadan, katmanda Türkçe katlanarak bir kez (ADR 0178
  §2'nin `fold`'u: “İl” ile “il” aynı addır). Ret, form ve dosya dizini bu adla anar.
- `fields`: katmanın 1–32 alanının kararlı anahtarları (ADR 0401 §2: kimlik, yoksa ad), sırayla, her biri bir kez. Bayt tutan
  alan (`binary`) dizine girmez (ArcGIS'in BLOB kuralı). 32 PostgreSQL'in sınırıdır.
- `unique` (yalnız `true` yazılır): dizin katmanın **anahtarıdır** (§3).
- `nullsEqual` (yalnız `true`, yalnız tekil dizinde): boş değerler aynı sayılır (PostgreSQL'in `NULLS NOT DISTINCT`'i).
- Bir katmanın en çok 64 dizini olur; aynı alanları aynı sırayla tutan iki dizin olmaz (biri ötekinin aynısıdır). Aynı alanlar
  başka sırayla başka bir dizindir.

Kurallar tek yerde: `layer_indexes_problem` (yer ve Türkçe cümle); kodek, belge ve komut aynı işlevi sorar.

**Alan değişince** (`follow_layer_indexes`, Alanlar penceresinin ve `cad.layers.edit`'in aynı adımında): kaldırılan ya da bayt
tutmaya başlayan alanı anan dizin **bütünüyle** gider, PostgreSQL'in `DROP COLUMN`'u gibi: kalan alanlarıyla duran tekil dizin
verilenden sıkı bir anahtar olurdu. Yeniden adlandırma anahtarı değiştirmez (dizin kararlı anahtarı anar).

### 2. `cad.layers.indexes` v1

Girdi `{layer, indexes, expectedRevision?}`; tek geri alma adımı “Dizinler”; boş liste kaldırır; aynısı değişmez. Çıktı ve plan
`cad.layers.rules`'unkiler (`LayersComposed`, `LayersComposePlan`). Retler sırayla: `invalid_revision`, `revision_conflict`,
`layer_not_found`, `not_a_layer`, `service_layer`, `invalid_indexes` (`indexes/<yer>`), `index_not_unique`.

`index_not_unique`: **yeni** ya da öncekinden **sıkı** (tekil olmayan tekil oldu, boşları ayrı sayan boşları aynı saymaya
başladı) tekil dizini katmanın kayıtları zaten bozuyorsa dizin yazılmaz; ret ilk yinelenen değerleri ve kayıtların kalıcı
kimliklerini (`uids`) söyler: ““Mahalle” dizini tekil olamaz: “Merkez” 2 kayıtta var. Önce kayıtları düzeltin ya da dizini
tekil yapmayın.” PostgreSQL de yinelenen veride `CREATE UNIQUE INDEX`'i reddeder. Olduğu gibi kalan tekil dizin yeniden
sorulmaz: başka yoldan gelmiş eski bir yineleme dizinleri düzenlemeyi kilitlemez (kayıt kuralları da var olan kayıtları yeniden
yargılamaz, ADR 0438 §1).

Uyarı `index_not_in_source`: katman bir tabloya bağlıysa dizini KentOS denetler, tabloda kurulmaz; tablonun kendi tekil
kısıtları (ADR 0755) yine sorulur.

### 3. Anahtar her yazmada

Tekil dizin, kaydı kim yazarsa yazsın (form, tablo, komut, içe aktarma, Python, ajan) sorulur: `checks::unique_clash`, tek
alanın tekilliğinin (ADR 0438 §3) ve tablonun tekil kısıtlarının (ADR 0755 §5) yanında, aynı yazmalar kümesiyle
(`cad.entities.set`, oluşturan komutlar, `cad.fields.write`, `cad.records.*`, formun yazmaları):

- Değer kanonik metnidir (alanın tek biçimi: `+101` ile `101` aynıdır). Boş ya da yalnız boşluktan oluşan değer **değer
  değildir** (KentOS'un boş kuralı): içinde boş olan küme karşılaştırılmaz, `nullsEqual` değilse. Bu SQL'in NULL kuralıdır.
- Yalnız dizinin bir alanını değiştiren yazma sorulur (yeni kayıt her zaman). Yazmalar birlikte sayılır: iki kayda birden aynı
  değerler verilemez; iki kaydın değerleri yer değiştirebilir.
- Ret `not_unique`, yeri yazmanın değiştirdiği **ilk** alan: ““Ada”, “Parsel” alanları katmanda birlikte tekil (“Ada ve parsel”
  dizini); (“101”, “5”) başka bir kayıtta var. Başka değerler verin.”
- `cad.fields.write` (Yapıştır, Alan hesapla) çakışan kümeyi tutan yazmaları çıkarır, yalnız onları (`UniqueClash.writes`);
  önceden yalnız reddin alanına bakılıyordu, bu birden çok alanlı anahtarda aynı adalı her parseli çıkarırdı.
- Az yazmada (en çok 16: bir hücre, bir form) katman her yazılan anahtar için bir kez taranır, değerler yerinde
  karşılaştırılır; bütün kayıtların anahtarları yalnız bir çakışma bulunursa sayılır (100 000 kayıtta anahtar kümesi kurmak
  yazma başına ~25 ms'dir).
- **Kaydı çoğalt** (ADR 0415): kopya, formun tekil tuttuğu alanların yanında katmanın tekil alanlarını (ADR 0438 §3; önceden
  kopyalanıyordu, kopya tekilliği sessizce bozuyordu) ve tekil dizinlerin alanlarını boş bırakır, her biri bir kez söylenir
  (`unique_cleared`: “Katmanın “Ada ve parsel” anahtarındaki “Ada” alanı; kopyalarda boş bırakıldı.”). Boşları aynı sayan
  anahtarın ikinci boşunu veren çoğaltma reddedilir (`not_unique`).

**Formda:** `Prepared::with_indexes` katmanın tekil dizinlerini bilir; kaydın değerlerini başka kayıt tutuyorsa `form_unique`,
kuralı `index/unique`, dizinin formda gösterilen ilk alanında (dizinin sırasıyla; form hiçbirini göstermiyorsa ilk alanında).
`unique_keys` dizinin alanlarını da sayar: Formla düzenle öbür kayıtları yalnız gerektiğinde toplar. Belgenin form izleyicisi
ve sunucunun form bekçisi de dizinleri alır.

### 4. Bağlı katman

Bağlı katmanın dizini KentOS'undur: yazmalar sorulur, tabloda dizin kurulmaz (§2'nin uyarısı). Tablonun kendi dizinleri ve
tekil kısıtları bağa ADR 0755'teki gibi okunur; bağlanan katman tablonun dizinlerini dizin olarak almaz (tablonun tekil kısıtı
zaten sorulur).

### 5. GeoPackage

**Yazarken** (`file.export.gpkg`, işlemlerin GeoPackage çıktısı, Paketle'nin kopyası): katmanın her dizini, alanlarının
yazıldığı sütunlar üzerinde `CREATE [UNIQUE] INDEX "{tablo}_{ad}"` olur (dosyada alınmış ad `_2`, `_3` … alır; SQLite'ta dizin
adı bütün dosyada tekildir). Dosyanın tutamadığı söylenir (`export_losses`):

- `nullsEqual`: SQLite boş değerleri hep farklı sayar; dizin tekil yazılır, kural düşer.
- Tekil dizini dosyanın değerleri bozuyorsa (KentOS boş metni değer saymaz, SQLite sayar) dizin düz yazılır.

Cevap tablo başına yazılan dizinleri sayar (`GpkgTableExported.indexes`).

**Okurken** (`file.import.gpkg`, yalnız içe aktarmanın açtığı katman): `PRAGMA index_list` ve `index_info` ile
(`kentos_geopackage::indexes`): CREATE INDEX'in ve UNIQUE kısıtının dizinleri; birincil anahtarın, kısmi dizin, ifade dizini,
fid'i ya da geometriyi anan dizin ve bayt sütunundaki dizin alınmaz. Ad `{tablo}_` önekinden arınır; UNIQUE kısıtının dizini
sütunlarının adlarıyla anılır (“tc”); aynı ad numaralanır; aynı sütunlar bir dizindir, biri tekilse tekil. Katmanın alanlarına
uymayan (bir ilişki onu başvuru alanı yaptı) dizin söylenerek bırakılır. Çizimdeki bir katmana giden tablonun dizinleri
alınmaz: katmanın kendi dizinleri vardır. Cevabın `indexes`'i alınan dizinleri sayar.

PostGIS'e tablo yazan yollar (işlemlerin veritabanı çıktısı) dizin kurmaz; açık iş.

### 6. Masaüstü: Dizinler penceresi

`data.layerIndexes` (CBS › Veri › Form panelinin ▾'i, Kayıt kuralları'nın yanında; KentOS Harita'da da): katman seçici, solda
dizinler (adı, alanları, “anahtar · kayıtlar uyuyor” ya da “n kayıt bozuyor”, “aranır”), sağda seçilen dizin: ad, “Tekil
(katmanın anahtarı)”, alanların çipleri (seçildiği sırayla dizine girer), sıralı alan listesi (yukarı, aşağı, çıkar), “Boş
değerler aynı sayılır” (yalnız tekilde), durumu. Kayıtların bozduğu tekil dizinde Kaydet kapalıdır ve neden yazılır; bağlı
katmanda bilgi satırı. Kaydet `cad.layers.indexes` ile tek adımda yazar. İkon: liste ve anahtar.

### 7. Python, MCP, katalog

`kentos.cad.layers.indexes` (üretilmiş), MCP aracı yıkıcı listede (boş liste dizinleri kaldırır), komut geçmişinde “Dizinler”,
işlem manifestinde `data.layerIndexes` → `cad.layers.indexes` (semantic).

## Sonuçlar

- Katmanın anahtarı formdan bağımsızdır: tablo, Yapıştır, Python ve ajan aynı kurala uyar.
- GeoPackage'a giden katman aranan alanlarının dizinini götürür; QGIS ve GDAL o dosyada dizini kullanır.
- Var olan kayıtların bozduğu anahtar kurulamaz; kayıtlar önce düzeltilir. Bu, dizinsiz veride bir tekil dizinin yanlış güven
  vermesini önler.
- KentOS bellekteki katmanda dizini aramayı hızlandırmak için kullanmaz (geometri deposu ve sütun motoru zaten taranır); dizin
  bir kural ve dosyanın dizinidir.

## Doğrulama

- Ortak durumlar, KentOS kodu olmadan (`scripts/fixtures/layer_indexes_cases.py --check`; formun ikizi `forms_cases.py`,
  alanların `layer_field_cases.py`): `cad.layers.indexes` 30, `cad.entities.set.indexes` 20, `cad.polygon.create.indexes` 20,
  `cad.fields.write.indexes` 20, `cad.forms.validate.indexes` 20, `cad.records.duplicate.indexes` 20 durum; bütün komut
  durumları masaüstü işleyicileriyle (`every_command_case`).
- GeoPackage: KentOS'un yazdığı `kentos-dizinler.gpkg` yalnız SQLite ile okunur (`--written`: üç dizin sütunlarıyla, anahtar
  ikinci (101, 5)'i reddeder); GDAL ve SQLite'ın yazdığı `dizinli.gpkg` (`--files`) içe aktarılınca dört dizin gelir, kısmi,
  ifade, bayt ve fid dizinleri gelmez, anahtar çizimde de tutar (`layer_indexes` testleri).
- `.kcad`: şema 136 örneği ve altı bozuk örnek bağımsız yazıcıdan (`kcad_v2_reference.py --check`), Python okuyucusu aynı kuralı
  okur.
- Ölçümler (release, bu makinede; `cargo test --release -p kentos-native-application --test all layer_indexes::timing --
  --ignored --nocapture`): 100 000 kayıtlı katmanda iki tekil dizinin kurulması 143 ms
  (önce 164), tek kaydın reddi 52 ms (çakışma bulununca bütün sayım; önce 59), tek kaydın yazımı 17,7 ms (`FEW_WRITES`'in
  yerinde karşılaştırması; önce 59,3), 10 000 değerin `cad.fields.write`'ı (1 000'i çakışan) 190 ms (önce 205).
- Resimler sahibin görünüşüne yakın açık temada, 1100 × 650 ve 1440 × 900: `form-dizinler`, `form-dizin-bozuk`,
  `form-dizin-yeni` (`KENTOS_SHOTS_ONLY=… cargo test -p kentos-desktop --lib tools_screens -- --ignored --nocapture`).
  Masaüstü testlerinde üç uygulamayı birden tutan kurtarma testi hata ayıklama yapısında 2 MiB yığının sınırındaydı; kendi
  8 MiB'lık iş parçacığında koşar.
