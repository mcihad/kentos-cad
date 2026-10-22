# ADR 0581: Kaynak sorgusu: dış veri kümeleri için ortak sağlayıcı sözleşmesi

- **Durum:** kabul edildi (2026-10-22). `plan/gis.md` CBS-02'nin kalanı: genel kaynak ve veri kümesi sözleşmesi; kaynağa itilen kutu,
  sütun ve süzgeç; kararlı sayfa ve akış; sorgu planının açıklaması; kısmi haritada “bütün kayıtlar” kapsamı.
- **Bağlam belgesi:** ADR 0502 (GeoPackage), 0503 (bağlı katman), 0504 (kaynağın sistemi ve kutu dönüşümü), 0505 (PostGIS),
  0509 (ölçüm tabanı), 0510 (bağlı katmanın parçası), 0532 (`cad.records.page`).
- **Sonraya kalan:**
  - çizim alanının, kenetin ve seçimin bu sözleşmeden okuması (görünür küme, kaydırdıkça sayfa): CBS-04 ve CBS-07;
  - PostGIS'te view ve SQL katmanı, milyon satırda ölçüm: CBS-04;
  - kaynakta toplama (SQL `sum`): kesin toplam kuralı (`kentos.statistics/1`) yerelde kalır, bkz. §6;
  - kararlı alan kimliği için kaynağın kendi numarası (PostgreSQL `attnum`): bugün alanın kimliği adıdır.

## Bağlam

Bağlı katman bir çalışma kopyasıdır (ADR 0503). Kaynağın kendisine soru sormanın tek yolu bütün tabloyu ya da bir alanını
çizime almaktı. Plan, öbür CBS maddelerinin çoğunu bu eksiğe bağlıyor: büyük kurum verisinde istatistik, sayfa sayfa tablo,
iki pencerede aynı kayıt, “görünen parça tam veri değildir” uyarısı. Sağlayıcılar (GeoPackage crate'i, PostGIS crate'i) bugün
yalnız bütün tabloyu ya da kutuyu okur ve yalnız eşitleme için yazar.

## Karar

### 1. Kavramlar

- **Kaynak:** bir GeoPackage dosyası ya da bir PostGIS adresi (`postgresql://…`, parolasız; ADR 0505).
- **Veri kümesi:** kaynağın bir tablosu (`schema.table` ya da GeoPackage tablo adı).
- **Katman görünümü:** çizimdeki bir katman. Yerel katman kendi nesneleridir; bağlı katman kaynağın çalışma kopyasıdır, parçası
  bağlanmışsa (ADR 0510) tam veri kümesi değildir.

### 2. Sağlayıcı sözleşmesi (`kentos_native_application::provider`)

- **Tanım:** veri kümesinin tanımı (`gpkg::Layer`): alanlar adlarıyla ve türleriyle, geometri sütunu, tür, Z ve M, sistem,
  yazılabilirlik. Alanın kimliği adıdır; yeniden adlandırma bir şema değişikliğidir.
- **Tarama (`scan`):** veri kümesinin kayıtlarını anahtar (fid, birincil anahtar) sırasıyla parti parti verir. Partinin boyu
  sabittir (1000); bellek partiyle sınırlıdır. Tarama şunları alır:
  - **Kutu:** veri kümesinin kendi sisteminde. GeoPackage'da mekânsal dizinle; dizin yoksa her geometrinin kutusuyla. PostGIS'te
    `geom && ST_MakeEnvelope(…)`.
  - **Sütunlar:** yalnız istenen alanlar okunur. Geometri yalnız gerektiğinde okunur.
  - **Süzgeç:** kaynağa itilen koşul (§3), değerleriyle.
  - **İmleç:** son anahtardan sonrası (`fid > ?`). Anahtar sırası ekleme ve silmede kararlıdır: değişmeyen kayıt iki kez
    gelmez ve atlanmaz.
  - **Durdurma:** her partinin arasında sorulur.
- **Kayıt kimliği:** kaynağın sakladığı KentOS kimliği (`kentos_uids`), yoksa dosyanın tablo adından ve anahtardan türetilen kimlik
  (`row_uid`, `database_row_uid`). Aynı kaynağı açan iki pencere aynı kaydı aynı kimlikle bulur; çalışma kopyasının nesnesi de
  aynı kimliği taşır.
- **Damga:** dosyanın boyu ve değişme zamanı. Bütün dosyanın SHA-256'sı sorgu için pahalıdır (ADR 0510). Veritabanında damga yoktur.
  Eşitlemenin sürümü (SHA-256) değişmez.
- **Yerel katman:** aynı sorguyu çizimin nesneleri üstünde yanıtlar. Sıra kalıcı kimliktir, imleç son kimliktir. Kaynağa itilen
  bir şey yoktur.
- Sağlayıcılar (GeoPackage, PostGIS, yerel) aynı sonucu verir; fark yalnız planın “kaynakta” satırlarıdır.

### 3. Süzgecin kaynağa itilmesi (`kentos_expression::pushdown`)

Süzgeç İfadeyle seç'in dilidir (ADR 0100). Dilin kuralları SQL'inkinden ayrıdır:

- Boş değer karşılaştırmada yanlıştır.
- Sayılar arasındaki eşitlik göreli 1e-9 içindedir.
- Metin sıralaması Türkçedir.
- Sayıya benzeyen metin sayı gibi karşılaştırılır.

**Kural:** kaynağa yalnız ifadenin tuttuğu her kaydı tutan bir koşul gider. Kaynaktan gelen her kayıt ifadeyle yerelde yeniden
sınanır. Sonuç, ifadenin bütün veri kümesinde tek tek değerlendirilmesiyle aynıdır. Kaynakta koşul yalnız okunanı azaltır.

İtilen biçimler (alan bir yanda, değişmez değer öbür yanda; yer değiştirince karşılaştırma ters çevrilir):

- **Sayı karşılaştırması** (`<`, `<=`, `>`, `>=`): sayı sütununda (tam ya da ondalık), sayı değerle → `c < ?`.
- **Sayı eşitliği:** sayı sütununda → `c BETWEEN ? AND ?`, pay `1e-8 · max(1, |v|)`. Dilin payından geniştir; kesin değildir.
- **Metin eşitliği:** metin sütununda, sayıya benzemeyen boş olmayan metin değerle → `c = ?`.
- **İçinde** (`içinde`): değerlerin her biri yukarıdaki eşitliklerden biriyse, onların `OR`'u.
- **Arasında:** sayı sütununda iki sayı değerle → `c BETWEEN ? AND ?`.
- **Boş:** `c IS NULL OR c = ''`; PostGIS'in sayı sütununda `c IS NULL`. **Boş değil:** `c IS NOT NULL AND c <> ''`; PostGIS'in sayı
  sütununda `c IS NOT NULL`.
- **ve:** iki yanın itilebilenleri. **veya:** iki yan da itilebiliyorsa.

Başka her şey (değil, eşit değil, gibi, işlevler, geometri değerleri, ifadeli karşılaştırmalar) itilmez. İtilen koşul ifadenin
tamamıysa ve her parçası kesinse plan onu **tam** sayar. Tam koşulda da kayıtlar yerelde yeniden sınanır; plan bunu yalnız söyler.

Lehçeler:

- SQLite (GeoPackage): yer tutucu `?`. Sayı sütunu metin de tutabilir (bir REAL sütununda “ölçülmedi”). Dil böyle bir metni sayıyla
  Türkçe metin sırasıyla karşılaştırır ve tutabilir. Bu yüzden sayı karşılaştırması ve sayı aralığı `(… OR typeof(c) = 'text')`
  olarak gider ve **tam sayılmaz** (rastgele koşullarla yapılan özellik testi bunu buldu).
- PostgreSQL: yer tutucu `$n`, sayı değer `::float8`.

Adlar çift tırnakla yazılır.

### 4. Komut: `cad.sources.query` v1 (salt okunur)

**Girdi:**

- **Ne sorulur:** iki yoldan biri.
  - `layer`: çizimin bir katmanı. `scope` `loaded` (çalışma kopyası ya da yerel nesneler; varsayılan) ya da `source` (bağlı
    katmanın kaynağının tamamı).
  - `path` ve `table`: bir GeoPackage dosyası ya da bir PostGIS adresi ve tablosu.
- `bbox`: projenin sisteminde kutu. Kaynağın sistemi başkaysa kutu ADR 0504'ün dönüşümüyle çevrilir.
- `near`: `at` ve `tolerance`. Tıklanan yerdeki kayıtları verir (tanımlama): önce kutu, sonra geometriye kesin uzaklık.
- `filter`: ifade (süzgeç).
- `fields`: döndürülecek alanlar.
- `geometry`: geometriyi projenin sisteminde ISO WKT olarak döndür.
- `limit`: 0–10 000, varsayılan 100. 0 kayıt döndürmez; sayım ve istatistik içindir.
- `cursor`: sonraki sayfanın imleci.
- `count`: süzgeçten geçen bütün kayıtların sayısı.
- `statistics`: alan adları. Her alanın istatistiği süzgeçten geçen **bütün** kayıtlardan çıkar, yalnız sayfadakilerden değil.

**Çıktı:**

- `source`: tür (`drawing`, `geopackage`, `postgis`), katman, dosya ya da adres, tablo, sistem.
- `rows`: kalıcı kimlik, anahtar, öznitelikler (çalışma kopyasının metin kuralıyla), WKT.
- `next`, `total`.
- `statistics`: okunan, atlanan, toplam, ortalama, en az, en çok, standart sapma; kesin ondalık metinleriyle.
- `plan`: aşağıda.
- `partial`: sonuç çalışma kopyasının parçasından, tam veri kümesi değil.
- `read`: kaynağın verdiği kayıt sayısı.
- `stamp`, `revision`.

**Plan (`plan`):**

- kutu: `index` (mekânsal dizin), `geometry` (geometrinin kutusu, kaynakta) ya da `local`;
- kaynakta okunan sütunlar ve geometrinin okunup okunmadığı;
- kaynağa giden koşul: değerleri yazılmış SQL ve tam olup olmadığı;
- yerelde yapılanlar: `filter`, `near`, `statistics`, `box`;
- sıra (`key` ya da `uid`).

**Retler, sırasıyla:**

- `invalid_input`: hiçbiri ya da ikisi, `path` olmadan `table`, `layer` olmadan `scope`, yerel katmanda `source`;
- `invalid_count`: `limit`;
- `invalid_extent`: ters ya da sonlu olmayan kutu;
- `invalid_tolerance`;
- `invalid_cursor`;
- `invalid_expression`: süzgeç derlenmiyor, `$sıra` ya da `$ölçek` okuyor ya da başka katmana bakıyor;
- `layer_not_found`;
- `file_not_found`;
- `import_unreadable`;
- `table_not_found`;
- `field_not_found`: `fields` ve `statistics`;
- `source_unreachable`;
- `invalid_address`: parolalı ya da bozuk veritabanı adresi;
- `crs_mismatch`: geometri gerekirken tablonun sistemi projeninkine çevrilemiyor (kutunun çevrilemediği de);
- durdurulan soru `cancelled` döner, ret kodu değildir.

**Uyarılar:**

- `partial_scope`: parçası bağlı katmanın `loaded` kapsamı;
- `rows_unreadable`: geometrisi okunamayan kayıtlar atlandı.

### 5. Kısmi harita ve “bütün kayıtlar”

Parçası bağlı katmanda `loaded` kapsamı `partial` der ve `partial_scope` uyarır. `source` kapsamı kaynağın tamamını tarar. Masaüstünde
alt panelin İstatistik sekmesi bağlı katmanda, ayrıntı bölmesinin başında **Kapsam** sorar (çubukta 1100 px'te yer yok):

- **Çalışma kopyası:** bugünkü gibi, çizimin nesneleri.
- **Kaynağın tümü:** seçili alanın istatistiği komutla (`sources_query::ask_source`), kendi iş parçacığında, Durdur'la. Bölmede kayıt
  sayısı, değer ve sayı olmayan sayıları, toplam, ortalama, en az, en çok, sapma ve kaynağın kaç satır verdiği yazar. Geometriden gelen
  alanlar kaynağa sorulmaz. Yanıt sorulduğu katman ve alanla tutulur; başka alanın geç gelen yanıtı yok sayılır.

### 6. İstatistikler

Toplama `kentos.statistics/1`'in kesin ondalık kuralıyla yerelde yapılır (`ops::statistics::figures`). Kaynağın SQL `sum`'ı
kayan noktadır; aynı veride KentOS'un başka yerdeki toplamıyla bit bit aynı olmaz. Bu yüzden kaynağa yalnız süzgeç ve sütunlar
itilir. Kayıtlar akarken yalnız istenen alanların değerleri tutulur.

### 7. Python, MCP ve manifest

- Python `kentos.cad.sources.query` ve MCP aracı katalogdan gelir. Komut salt okunurdur, MCP'nin yıkıcı listesine girmez.
- Manifest: komut arayüzde ayrı bir komut değil, İstatistik'in Kapsam'ıdır; manifestin satırları değişmez.
- Web: TODOS.md §0.2.

## Doğrulama

- `fixtures/commands/v1/cad.sources.query.json`: KentOS kodu olmadan `scripts/fixtures/source_query_cases.py`'den.
  - GeoPackage'ı (`fixtures/sources/v1/sorgu.gpkg`) GDAL yazar.
  - Başvuru kayıtları OGR'la okur; kutuyu, süzgeci (dilin karşılaştırma kurallarının bağımsız Python ikizi), sayfaları,
    istatistikleri (`kentos.statistics/1`'in başvurusu) ve itilen SQL'in metnini ADR'nin kuralından yazar.
- Özellik testi (`crates/native/application/tests/all/sources_query.rs`): garip değerli 600 satırlık bir tablo doğrudan SQLite'a
  yazılır (sayı sütununda metin, 2⁵³ üstü tam sayı, boş metin, Türkçe harfler). 400 rastgele koşul iki kez sorulur: dosyaya
  (itilerek) ve çizime alınmış kopyasına (itilmeden). Satırlar aynıdır; plan tam diyorsa kaynak yalnız eşleşenleri verir.
- PostGIS: `postgis_cases.py`'nin geçici veritabanında `sorgu.gpkg`'nin tabloları ogr2ogr'la `sorgu` şemasına yüklenir;
  `kentos-postgis`'in `the_queries_answer_alike_from_postgis` testi 15 soruyu iki kaynağa sorar: satırlar, anahtarlar, değerler,
  sayfalar, sayılar ve istatistikler aynı.
- Süre (`provider_timing`, 22 Ekim, release, 8 çekirdek, 1 milyon parsellik GeoPackage, 367 MB):
  - kutu ve `Parsel > 500` süzgeci, ilk sayfa: 357 satır okundu, 2,9 ms;
  - kaynağın tümü, `Ada > 500` ile sayı ve istatistik: 500 000 satır okundu, 854 ms;
  - kaynağın tümü, süzgeçsiz sayı ve istatistik: 1 000 000 satır, 1 561 ms;
  - üçünde de süreç belleği değişmedi (733 MB). Aynı tabloyu bütünüyle bağlamak 11,8 s ve 4,3 GB tepe bellek ister.
