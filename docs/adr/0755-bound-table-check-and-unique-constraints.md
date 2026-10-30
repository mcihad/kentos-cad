# ADR 0755: Bağlı tablonun CHECK ve tekil kısıtları

- **Durum:** kabul edildi (2026-10-30).
- **İstek:** sahibin sözü “formlarımız çok zayıf”; ADR 0750, 0753 ve 0754'ün çizgisi: bağlı tablonun kendi kuralları formda.
  Sahibin `kentos` veritabanında (salt okunur incelendi) 172 CHECK, 144 UNIQUE (118'i tek, 26'sı birden çok sütunlu, bazıları
  `NULLS NOT DISTINCT`) ve 2 EXCLUDE kısıtı var.
- **Web:** bir şey değişmez; TODOS.md §0.2'ye yazıldı. **`.kcad`:** şema 134 (opt-in): bağın `checks` ve `uniques`'i, alanın
  sütununun `check`'i.
- **Bağımlılık:** yeni kitaplık yok (düzenli ifadeler zaten `regex-lite` ile).

## Bağlam

Bağlı tablonun CHECK ve UNIQUE kısıtları katmana gelmiyordu:

- **Formda.** `tur` sütununun CHECK'i `(tur)::text = ANY (ARRAY['is', 'personel', 'butce_disi'])` olan bir tabloda form serbest
  bir metin kutusu gösteriyordu; yanlış değer çalışma kopyasına yazılıyor, ancak Kaynağa yaz'da veritabanı reddediyordu.
- **Kaynağa yazarken.** Veritabanının reddi `file_unwritable` olarak, ham hata metniyle (“new row for relation … violates check
  constraint …”) dönüyordu: hangi kısıtın, hangi alanın tutmadığı Türkçe söylenmiyordu.
- **Tekil kısıt.** `UNIQUE (mali_yil, yevmiye_no)` gibi birden çok sütunlu tekillik katmanın kuralları arasında yoktu; alanın kendi
  `unique`'i yalnız tek alanı bilir (ADR 0438 §3).

Başvurulan yollar (30 Ekim 2026'da okundu):

- PostgreSQL, *Constraints* §5.5.1: “a check constraint is satisfied if the check expression evaluates to true or the null
  value”; CHECK yalnız yeni ya da güncellenen satırı okur, başka satıra ya da tabloya bakan CHECK desteklenmez ve koşulun değişmez
  olduğu varsayılır. §5.5.3: tekil kısıtta iki NULL varsayılan olarak eşit sayılmaz, `NULLS NOT DISTINCT` bunu değiştirir. §5.5.6:
  dışlama kısıtı iki satırın karşılaştırmasıdır. <https://www.postgresql.org/docs/current/ddl-constraints.html>
- PostgreSQL, `pg_constraint`: `contype` `c` CHECK, `u` UNIQUE, `x` EXCLUDE, `f` yabancı anahtar, `p` birincil anahtar; `conkey`
  kısıtın sütunları; `conindid` tekil kısıtın dizini; CHECK'in metni için `pg_get_constraintdef()`. `CREATE UNIQUE INDEX` ile
  yapılan dizin `pg_constraint`'te yoktur, `pg_index`'tedir. <https://www.postgresql.org/docs/current/catalog-pg-constraint.html>
- PostgreSQL, Ek A, sınıf 23: `23502` not_null_violation, `23503` foreign_key_violation, `23505` unique_violation, `23514`
  check_violation, `23P01` exclusion_violation; bu sınıfın hataları kısıtın adını söyler.
  <https://www.postgresql.org/docs/current/errcodes-appendix.html>
- SQLite, *CREATE TABLE* §3.7: CHECK'in ifadesi NUMERIC'e çevrilir; sıfırsa kısıt bozulur, NULL ya da sıfırdan farklıysa
  bozulmaz; alt sorgu içeremez; yalnız yazarken denetlenir. §3.6: tekil kısıtta NULL'lar birbirinden ve her değerden farklıdır.
  Adsız CHECK'in reddi ifadesinin yazıldığı metinle söylenir (“CHECK constraint failed: a > 0”, SQLite 3.46 ile denendi).
  <https://www.sqlite.org/lang_createtable.html>
- SQLite, `PRAGMA index_list`: dizinin tekil olup olmadığı, kaynağı (`c` CREATE INDEX, `u` UNIQUE kısıtı, `pk` birincil anahtar) ve
  kısmi olup olmadığı; `PRAGMA index_info`'da ifade sütunu `cid = -2`. <https://www.sqlite.org/pragma.html>
- QGIS, *Attributes Form › Constraints*: alanın üç kısıtı (Not null, Unique, Expression); zorlanan kısıt (“hard”) karşılanmadan
  kaydedilmez, zorlanmayan (“soft”) uyarır; `QgsFieldConstraints.ConstraintOriginProvider` kısıtın sağlayıcıdan geldiğini söyler ve
  kısıtlar varsayılan olarak zorlanır. Belgeler sağlayıcının CHECK kısıtını ifade kısıtına çevirdiğini söylemez.
  <https://docs.qgis.org/latest/en/docs/user_manual/working_with_vector/vector_properties.html>,
  <https://qgis.org/pyqgis/master/core/QgsFieldConstraints.html>
- GDAL: `OGRFieldDefn::IsUnique()` 3.2'den beri, yazabilen sürücü `GDAL_DCAP_UNIQUE_FIELDS` der; GeoPackage'ın şema eki
  kısıtları 3.3'ten beri alan alanı (field domain) olarak okunur; CHECK'i okuyan bir arayüz yok.
  <https://gdal.org/en/stable/api/ogrfeature_cpp.html>, <https://gdal.org/en/stable/drivers/vector/gpkg.html>

## Karar

### 1. Bağ tablonun kısıtlarını tutar

Bağlı katmanın bağı (`SourceLink`) tablonun kısıtlarını tablodan son okunduğu gibi tutar (`.kcad` şema 134, yalnız varsa):

- `checks`: her CHECK'in adı (`name`; SQLite'ın adsız kısıtında yok), ifadesi (`sql`, tablonun verdiği metin: PostgreSQL'de
  `pg_get_constraintdef`'in `CHECK (…)`'unun içi, SQLite'ta `CREATE TABLE` metninde yazıldığı gibi), okuduğu sütunlar
  (`columns`) ve yalnız tablonun denetleyip denetlemediği (`tableOnly`, yalnız `true` yazılır). Yalnız tablonun denetlediği
  CHECK, okuduğu bir sütunun karşılaştırmasını KentOS'un tablonun yaptığı gibi yapamadığıdır: SQLite'ta bayt sırasından başka bir
  harmanlamayla (`COLLATE NOCASE`) ya da bildirilen türünün eğilimi (affinity) alanın türününkinden başka olan sütun (`TEXT`
  bildirilmiş sütunda tam sayı alanı: SQLite değeri metin olarak saklar ve öyle karşılaştırır), PostgreSQL'de `character(n)`
  (`bpchar`, sondaki boşlukları saymaz) ya da belirleyici olmayan (non-deterministic) bir harmanlama.
- `uniques`: her tekil kısıtın adı, sütunları (sırasıyla) ve NULL'ları eşit sayıp saymadığı (`nullsEqual`, PostgreSQL'in
  `NULLS NOT DISTINCT`'i). Tekil kısıt da düz sütunlu tekil dizin de (`CREATE UNIQUE INDEX`) buraya girer; kısmi ve ifadeli dizin
  girmez (katmanın bir satırını tek başına bağlamaz).

Bağlama ve her okuma onları yeniden yazar: tabloda kalkan kısıt bağdan da kalkar. Birincil anahtar ve EXCLUDE tutulmaz (birincisi
satırın anahtarıdır, ikincisi iki satırın karşılaştırması: yalnız tablo denetler, reddi §5'teki gibi söylenir). Sınırlar: bağ başına
en çok 256 CHECK ve 256 tekil kısıt, CHECK'in metni en çok 8000 karakter; kısıt sütunları tablonun sütunlarıdır.

### 2. Okuma

- **GeoPackage (SQLite).** CHECK'ler tablonun `CREATE TABLE` metninden okunur (`sqlite_master`): sütunun kendi kısıtı ve tablo
  kısıtı, isteğe bağlı `CONSTRAINT ad`'ıyla; metin aynı yerden sütunların `COLLATE`'ini de verir. Tekil kısıtlar (kaynağı `u`) ve
  düz tekil dizinler (kaynağı `c`, kısmi olmayan) `PRAGMA index_list` ve `PRAGMA index_info` ile, yapılış sırasıyla (liste en
  yeniyi önce verir); ifade sütunu olan atlanır, tekil kısıtın adı yoktur (SQLite onu `sqlite_autoindex_…` diye adlandırır).
  Okuyucu ADR 0754'ün sözcükleyicisini (`geopackage::made`) paylaşır.
- **PostgreSQL.** `pg_constraint`'in `c` ve `u` satırları (`conkey`'in sütunları, `pg_get_constraintdef(oid)`), `pg_index`'in
  kısıtı olmayan, kısmi olmayan, ifadesiz tekil dizinleri; `NULLS NOT DISTINCT` sürüm 15'ten beri `indnullsnotdistinct`'ten.
  `NOT VALID` CHECK de tutulur: var olan satırlara bakılmaz ama her yeni yazma onu sağlamalıdır.

### 3. KentOS CHECK'i SQL'in kendi anlamıyla sorar

CHECK'i KentOS'un ifade diline çevirmek doğru olmaz: dil iki değerlidir (boş bir değerle karşılaştırma “yanlış”tır), boş metinle
NULL'u ayırmaz ve sayıya benzeyen metinleri sayı olarak karşılaştırır. Bu yüzden formlar çekirdeği (`kentos_forms::table_checks`)
kısıtın kendi SQL metnini okur ve SQL'in üç değerli anlamıyla, kaydın öznitelikleri üzerinde sorar:

- Öznitelik yoksa değer NULL'dur, boş metin `''`'dir (yazıcının tabloya yazdığının aynısı, ADR 0750 §3). Değerin türü alanın
  türündendir: sayılar kesin ondalık olarak, tarih ve saatler ISO metinleriyle, evet/hayır doğru/yanlış olarak karşılaştırılır.
- Desteklenen dil: sabitler, sütunlar, `::tür` ve `CAST`, `= <> != < <= > >=`, `IS [NOT] NULL`, `IS [NOT] DISTINCT FROM`,
  `IS [NOT] TRUE/FALSE`, `[NOT] IN (…)`, `= ANY (ARRAY[…])`, `<> ALL (ARRAY[…])`, `[NOT] BETWEEN`, `[NOT] LIKE`/`ILIKE`,
  SQLite'ın `GLOB`'u, PostgreSQL'in `~ ~* !~ !~*`'ı, `AND OR NOT`, `+ - * / %`, `||`, `CASE WHEN … END` ve `length`,
  `char_length`, `octet_length`, `btrim`, `trim`, `ltrim`, `rtrim`, `lower`, `upper`, `abs`, `coalesce`, `nullif`.
- Değerlendirme kesin bir SQL değeri ya da “bilinmez” verir. Bilinmez: PostgreSQL'de metinlerin sıralaması (veritabanının
  harmanlamasına bağlı; SQLite'ta baytların sırasıdır, bilinir), PostgreSQL'de ASCII dışı harfin büyük küçük çevrimi, ASCII dışı
  bir düzenli ifade, bilinmeyen işlev, `CURRENT_DATE` ve benzeri, hesaplanan sütun (ADR 0754), SQLite'ta tarih-saat ve saat
  dilimli tarih, JSON ve bayt değerleri.
- Sayılar iki kez tutulur: kesin ondalık (128 bitlik mantis ve ölçek) ve veritabanının tutabileceği ikili kayan sayı. İki sayının
  sırası ancak ikisinde aynıysa ve kayan sayılar birbirine 4 ε'dan yakın değilse söylenir; yoksa bilinmez. SQLite sayı eğilimli
  sütuna yazılan sayıya benzeyen metni sayı olarak saklar; KentOS da öyle okur (`sqlite_number`).
- Kısıt üç durumdan birindedir (`Judged`): **bütünüyle** KentOS denetler; **kısmen** denetler (bilinmez bir parçası var: bir `AND`'in
  bilinen yanı yanlışsa yine reddedilir); **yalnız tablo** denetler (ayrıştırılamayan metin ya da `tableOnly`).
- **Kural: KentOS yalnız tablonun reddedeceğini reddeder.** Kısıt yalnız sonucu kesin olarak yanlışsa (SQLite'ta sıfırsa) bozulur;
  NULL, doğru ve bilinmez geçer. Böylece istemci hiçbir zaman tablonun kabul edeceği bir yazmayı durdurmaz; denetleyemediğini tablo
  denetler ve reddi §5'teki gibi adıyla söylenir.

Kısıt, okuduğu bir sütun değişince sorulur (yeni kayıtta her biri), katmanın kayıt kuralları gibi (ADR 0438 §1). Bozulan kısıt
`rule_broken`'dır: tablonun kısıtı katmanın kaydının kuralıdır. İletisi kısıtın sık biçimlerinde Türkçe bir cümledir (“tur şunlardan
biri olmalı: is, personel, butce_disi”, “oncelik 1 ile 3 arasında olmalı”, “bitis, baslangic alanından büyük olmalı”, “kod şu kalıba
uymalı: …”), başka biçimlerde kısıtın adı ve metni. Formla düzenle onu alanının yanında gösterir (QGIS'in zorlanan kısıtı gibi
kaydetmeyi durdurur); komutlar aynı kuralla reddeder.

### 4. Değer listesi olan CHECK alanın listesi olur

Tek sütunu bir değer listesine bağlayan CHECK (`x IN (…)`, `x = ANY (ARRAY[…])`, `x IS NULL OR x IN (…)`) alanın değer listesi
olur, kodları ve etiketleri listenin değerleri; alanın sütunu onu `column.check` ile anar (kısıtın adı, adsızsa metni). Yabancı
anahtarın listesindeki kural (ADR 0753 §2) aynen geçerlidir: alanın kendi listesi, projenin listesi ya da yabancı anahtarı varsa
dokunulmaz; kısıt kalkınca liste de kalkar. Liste yalnız her kodu alanın türünde bir değer olan (metinde boşluksuz ve olduğu
gibi, sayıda alanın kuralına uyan) ve NULL içermeyen listeden yapılır: `x IN ('a', NULL)` her değeri geçirir, liste değildir. Form
böylece serbest kutu yerine açılır liste gösterir.

### 4a. `numeric(p, s)` alanın ölçeği olur

PostgreSQL `numeric(10, 2)` sütuna yazılan 12,505'i 12,51 olarak saklar ve CHECK'i saklanan değerle sorar. KentOS kısıtı yazılan
değerle sorduğundan iki yan ayrılabilirdi; bu yüzden ondalık alanın ölçeği (`scale`, ADR 0199) bağlarken ve her okumada sütunun
`numeric(p, s)`'inden alınır (alanın kendi ölçeği yoksa, en çok 15). Alanın kuralı (fazla basamak yazılamaz) yazılanı saklananla
aynı tutar.

### 5. Tekil kısıt katmanda sorulur, tablonun reddi adıyla söylenir

- **Tekillik.** Bağın tekil kısıtları (tek ya da birden çok sütun) çalışma kopyasının kayıtları üzerinde sorulur: yazılan kayıtla
  aynı değerleri taşıyan başka bir kayıt `not_unique`'tir. NULL'lu bir değer takımı, kısıt `nullsEqual` değilse karşılaştırılmaz;
  boş metin bir değerdir. Alan bağı (ADR 0510) katmana alınmamış satırları göremez: onların çakışmasını tablo söyler.
- **Tablonun reddi.** Kaynağa yazarken veritabanının kısıt reddi ham metinle değil koduyla söylenir; hiçbir şey yazılmaz:
  - `source_check`: bir CHECK ya da EXCLUDE kısıtı (PostgreSQL `23514`, `23P01`; SQLite `SQLITE_CONSTRAINT_CHECK`); ileti kısıtın
    adını ve bağda biliniyorsa cümlesini söyler. SQLite adsız kısıtı metniyle andığından bağdaki kısıt adıyla ya da metniyle
    bulunur; alanın `column.check`'i de adsız kısıtı metniyle anar.
  - `source_unique`: tekil kısıt (PostgreSQL `23505` ayrıntısıyla, SQLite `UNIQUE constraint failed: t.a, t.b`).
  - `source_reference`: yabancı anahtar (PostgreSQL `23503`, SQLite `SQLITE_CONSTRAINT_FOREIGNKEY`): başvurulan satır yok ya da
    silinen satıra başka satırlar başvuruyor.
  - `source_not_null`: NOT NULL (PostgreSQL `23502`, SQLite `SQLITE_CONSTRAINT_NOTNULL`), ADR 0750'nin ön denetiminin kaçırdığı.

### 6. Masaüstü

Alanlar penceresi bağlı katmanda tablonun kısıtlarını söyler: alanın satırında “Tabloda tekil” ve “Değerler tablonun kısıtından”,
altta “Tablonun kısıtları” listesi (her CHECK'in cümlesi ve “KentOS da denetler”, “KentOS kısmen denetler” ya da “Yalnız tablo
denetler”, birden çok sütunlu tekil kısıtlar sütunlarıyla, “boşlar da aynı sayılır” ile). Formla düzenle değer listesini açılır
liste olarak, bozulan kısıtı alanın yanında gösterir.

### 7. Yapılmayanlar

- EXCLUDE ve başka satıra bakan kısıtlar yalnız tabloda denetlenir; reddi §5'teki gibi söylenir.
- Metin sıralaması ve PostgreSQL'de ASCII dışı büyük küçük harf: harmanlama bilinmeden yargılanmaz (bilinmez, geçer).
- GeoPackage şema ekinin `gpkg_data_column_constraints`'i (aralık, liste, glob; SQLite onları zorlamaz) bu kararda okunmaz.
- Tetikleyicilerin (trigger) koşulları okunmaz.

## Sonuçlar

- Bağlı tablonun CHECK'i formda, komutta ve Kaynağa yaz'dan önce sorulur; değer listesi olan CHECK açılır liste olur.
- İstemci tablonun kabul edeceği bir yazmayı hiçbir zaman reddetmez; denetleyemediğini tablo reddeder ve ret adıyla söylenir.
- Birden çok sütunlu tekil kısıt katmanda sorulur.
- `.kcad` şema 134; eski dosyalar bayt bayt aynı.

## Doğrulama

- **Kısıtların yargısı, veritabanlarının kendi cevabıyla.** `scripts/fixtures/table_checks_cases.py` 59 kısıtı (32'si SQLite,
  27'si PostgreSQL) ve 183 kaydı KentOS kodu olmadan yazar; her kaydı SQLite'ın kendisine (Python'un `sqlite3`'ü) ve geçici bir
  PostgreSQL 16 sunucusuna (docker, `postgres:16-alpine`; 5432'ye dokunulmaz) yazıp cevabını alır (`fixtures/table-checks/v1/
  cases.json`; `--check` dosyadan denetler, `--write` ve `--pg` sunucuyla yeniden yazar). Durumlar: değer listeleri (`IN`,
  `= ANY (ARRAY[…])`, `'{a,b}'::text[]`), aralıklar, iki alanın karşılaştırması, uzunluk ve `btrim`, `LIKE`, `GLOB`, düzenli
  ifade, `CASE`, `COALESCE`, `IS DISTINCT FROM`, kesin ondalık ve kayan sayının sınırı (`0.1 + 0.2`), SQLite'ın eğilimleri
  (sayı eğilimli sütuna yazılan sayıya benzeyen metin), `COLLATE NOCASE`, PostgreSQL'de metin sıralaması ve ASCII dışı büyük
  küçük harf, bilinmeyen işlev, `NOT VALID`. Tablonun reddettiği 80 kaydın 69'unu KentOS önceden reddeder; kalan 11'i
  bilinmez parçalıdır ve tabloya kalır. KentOS tablonun kabul ettiği hiçbir kaydı reddetmez (Rust'ta
  `table_checks::tests::the_shared_cases` her kaydı böyle denetler). 52 kısıt bütünüyle, 5'i kısmen KentOS'ta, 2'si yalnız
  tabloda. Ayrıca 23 birim testi (sözcükleyici, ayrıştırıcı, değerler, desenler, cümleler, listeler, sütunlar).
- **Komut durumları.** `scripts/fixtures/source_constraints_cases.py` 25 ortak durumu (`fixtures/commands/v1/
  cad.sources.constraints.json`) GDAL ve `sqlite3` ile yazılan `kisitli.gpkg` ve `kisitli-2.gpkg` üzerinden, kısıtları ADR'nin
  kuralıyla dosyanın kendi metninden ve `PRAGMA`'larından okuyarak yazar: bağın `checks` ve `uniques`'i, değer listesi olan
  alan, `rule_broken`, `not_unique`, formun `form_layer_rule` ve `form_unique`'i, Kaynağa yaz'da `source_check` ve
  `source_unique` (dosyaya doğrudan yazılan kayıtla), okumada kısıtın kalkması ve değişmesi, plan. ADR 0754'ün dosyasındaki
  CHECK de artık bağda (`source_generated_cases.py` aynı okuyucuyu kullanır).
- **PostGIS.** `scripts/fixtures/postgis_cases.py --only a_tables_constraints` geçici docker PostGIS'inde sahibinkine benzeyen
  `muhasebe.fis`'i kurar (CHECK'ler, `UNIQUE NULLS NOT DISTINCT`, kısıtsız ve kısmi tekil dizin, `kbs.iller`'e yabancı anahtar):
  bağın kısıtları, `character(1)` sütunun yalnız tabloda kalması, listeye dönen CHECK, `numeric(14, 2)`'nin ölçeği, KentOS'un
  önceden reddettikleri, tablonun reddettikleri (`source_check`, `source_unique`, `source_reference`) ve son yazma; tablonun
  satırları `psycopg2` ile denetlenir.
- **`.kcad` şema 134.** `table-constraints.json`/`.kcad` örneği ve beş bozuk dosya (şema 133'te kısıt, boş CHECK, sütunsuz ve
  sütunu yinelenen tekil kısıt, boş `column.check`), bağımsız Python yazıcısı (`kcad_v2_reference.py`) ve okuyucusu
  (`tools/kcad/kcad.py`) ile; eski dosyalar bayt bayt aynı.
- **Masaüstü.** `constraint_screens::alanlar_and_the_form_say_the_tables_constraints`: Alanlar'ın sözleri, Formla düzenle'nin
  açılır listesi ve bozulan kısıtın iletisi. Resimler 1100 × 650 ve 1440 × 900'de, sahibin görünüşüyle (`kisit-1-alanlar-liste`,
  `kisit-2-alanlar-tekil`, `kisit-3-formla-duzenle`).
- **Takımlar.** `kentos-forms`, `kentos-contracts`, `kentos-geopackage`, `kentos-kcad`, `kentos-commands`,
  `kentos-native-application`, `kentos-postgis`, `kentos-processing`: 2829 test geçti. Düşen iki testten biri ADR 0754'ün ortak
  durumlarıydı (dosyasının CHECK'i bağa geldi; başvuru güncellendi), diğeri main'de bu işten önce de düşen
  `walls_partition::source_height_and_architecture_survive_and_curved_edges_are_explicitly_refused` (ADR 0734 yaylı kaynağı
  kabul eder, test eski reddi bekler; mimari ajanın dosyası).
- **Python.** `pnpm py:test`: üretici güncel, 375 test (ortak durumlar `cad.sources.constraints.json` ve güncellenen
  `cad.sources.generated.json` dahil) ve `kentos-python`'ın clippy'si geçti.
- **Süreler (release,** Intel Core i5-11300H, `f63a89448` üstünde; `cargo test --release -p kentos-forms --lib
  table_checks::tests::timing -- --ignored --nocapture`**).** Sık altı kısıt (değer listesi, aralık, alt sınır, düzenli ifade, iki
  tarihin karşılaştırması, `btrim`'li uzunluk) bir kez kurulur: 15,3 µs. 100 000 yeni kayıt altısıyla sorulur: 154,6 ms (kayıt
  başına 1,5 µs). Tek alanı değişen 100 000 düzenleme yalnız o alanı okuyan kısıtla sorulur: 51,6 ms (0,5 µs). Kısıtlar bağın
  okunduğu her seferde bir kez okunur (tek katalog sorgusu); yazmanın yoluna eklediği yalnız bu sorulardır.

## Kalan

- `gpkg_data_column_constraints` ve GDAL'ın alan alanları (field domains).
- Kullanıcının bağlı olmayan katmanda tanımladığı birden çok alanlı anahtar (plan/form.md FORM-DATA-03).
