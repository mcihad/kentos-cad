# ADR 0750: Bağlı tablonun zorunlu ve varsayılanlı sütunları

- **Durum:** kabul edildi (2026-10-30).
- **İstek:** sahibin sözü “formlarımız çok zayıf”. Sahip PostGIS tablolarını katman olarak bağladı.
- **Web:** bir şey değişmez; TODOS.md §0.2'ye yazıldı. **`.kcad`:** değişmez (alanın `required`'ı ve `default`'u var
  olan özelliklerdir).
- **Bağımlılık:** yeni kitaplık yok.

## Bağlam

Sahibin `kbs.ilceler` tablosu iyi kurulmuş bir kurum tablosudur:

- `id` kimlik sütunudur (identity).
- `genel_id` `uuidv7()`, `olusturma_tarihi` ve `guncelleme_tarihi` `now()` varsayılanlıdır.
- `ek_veri` `'{}'`, `tur` `'İlçe'`, `merkez_mi` `false`, `aktif` `true` varsayılanlıdır.
- `il_id` ve `ad` NOT NULL'dur ve varsayılanları yoktur.

Sahibin tablosu salt okunur incelendi (`pg_attribute`, `pg_attrdef`, `pg_constraint`). Bağlı katmanla formun üç kusuru
çıktı:

- **Kurallar katmana geçmiyordu.** Sağlayıcı sütunun NOT NULL'ını ve varsayılanını okuyordu, ama alan yalnız adını ve
  türünü alıyordu. Formda hiçbir alan zorunlu görünmüyordu.
- **Yeni kayıt reddediliyordu.** Yazma yeni satırın bütün sütunlarını yazar, boş değer NULL'dur. `genel_id`,
  tarihler, `tur`, `merkez_mi` NOT NULL olduğundan KentOS'tan eklenen her yeni ilçe veritabanında reddedilirdi.
- **Yazılsa da sonraki yazma reddedilirdi.** Veritabanının verdiği değerler nesneye dönmüyordu. Taban çizimden
  kaydedildiğinden, sonraki yazma o satırı “başkası değiştirdi” sayardı.

Ayrıca isteğe bağlı `gpkg_extensions` tablosu olmayan geçerli bir GeoPackage'a KentOS yazamıyordu. Bu kusur ortak
durumları yazarken bulundu: GDAL uzamsal dizinsiz bir GeoPackage'ı o tablo olmadan yapar.

Başvurulan yollar (30 Ekim 2026'da okundu):

- QGIS'in kullanıcı kılavuzu, proje özellikleri › Veri kaynakları: *Evaluate default values on provider side* kapalı
  başlar. Kapalıyken PostgreSQL'in varsayılanı formda ifadesiyle görünür, satır yazılınca veritabanı verir.
  <https://docs.qgis.org/latest/en/docs/user_manual/introduction/qgis_configuration.html>
- PostgreSQL'in `INSERT`'i: `DEFAULT` sütunu kendi varsayılanıyla doldurur; `RETURNING` yazılan satırı döndürür.
  <https://www.postgresql.org/docs/current/sql-insert.html>
- SQLite'ın `INSERT`'i: sütun listesinde olmayan sütun varsayılanını alır. `PRAGMA table_info` varsayılanı yazıldığı
  gibi verir (`'İlçe'`, `0`, `upper('ilce')`).
  <https://www.sqlite.org/lang_insert.html>, <https://www.sqlite.org/pragma.html#pragma_table_info>
- GeoPackage 1.4 belirtimi: `gpkg_extensions` tablosu bir eklenti kullanılana dek isteğe bağlıdır.
  <https://www.geopackage.org/spec/#extension_mechanism>

## Karar

### 1. Alanlar tablonun kurallarını alır

Bağlanan ya da içe aktarılan tablonun her sütunu alan olurken (`geopackage::field_of`):

- **NOT NULL ve varsayılansız sütun** zorunlu alan olur (`required`). Form onu yıldızla gösterir, boş bırakılınca söyler.
- **Sabit varsayılan** alanın varsayılanı olur (`default`, alanın türünün metniyle). Yeni nesne onu alır (ADR 0199 §2).
  Sabit varsayılanlar (`literal_default`):
  - tırnaklı metin: `'İlçe'::character varying`, `'Ali''nin'`, `'{}'::jsonb`;
  - sayı: `7`, `(-1)`, `'3'`;
  - `true` ya da `false`; SQLite'ın mantıksal `0` ve `1`'i.
- **Veritabanının hesapladığı varsayılan** alana konmaz: `now()`, `uuidv7()`, `gen_random_uuid()`, `nextval(…)`,
  `CURRENT_TIMESTAMP`, `upper('x')`. `NULL` varsayılan sayılmaz. Alanın türüne uymayan sabit (tam sayı sütununda
  `'yok'`) de konmaz.
- **Tablodan okuma** (`cad.sources.sync` read) tablonun kurallarını var olan alanlara katar: zorunluluk eklenir,
  varsayılan alanın kendisininki yoksa konur. Kullanıcının koyduğu hiçbir kural kaldırılmaz. Önceden bağlanmış
  katmanlar böylece ilk okumada kuralları alır.

### 2. Yeni satırın boş değeri varsayılanlı sütunda tablonundur

- **Boş bırakılan değer.** Yeni satırın varsayılanı olan bir sütundaki boş değeri yazılmaz, tablonun varsayılanı
  alınır (`gpkg::left_to_default`):
  - PostgreSQL'de o yere `DEFAULT` yazılır, sonraki parametreler yeniden numaralanır;
  - GeoPackage'ta sütun `INSERT`'in listesinden çıkar.
- **Değer verilen sütun** varsayılanı almaz.
- **Varsayılanlı ve NULL olabilen bir sütunu yeni kayıtta boş bırakmak** veritabanının varsayılanını verir. QGIS de
  böyle yapar. KentOS'ta boş ile verilmemiş ayrılmaz.

### 3. Tablonun verdiği değer aynı adımda nesneye döner

Yazmanın işleminde yeni satırların varsayılana bırakılan sütunları geri okunur:

- PostgreSQL'de satırları okuyan aynı yolla (`rows_by_keys`);
- GeoPackage'ta sütunlar satırın `fid`'iyle.

Değerler yazmanın adımında nesnenin özniteliği olur, sütunun alanının türüyle okunarak (`field_text`): UUID, an, JSON.
Taban ve katmanın özeti bundan sonra kaydedilir. Ardından tablodan okumak hiçbir şeyi değiştirmez.

### 4. NOT NULL sütunu boş bırakan yazma reddedilir

Yazmadan önce, planda da, şu nesneler aranır:

- NOT NULL ve varsayılansız sütunu boş olan yeni nesne;
- NOT NULL sütunu boşaltılmış değişen nesne.

Bulunursa yazma `source_not_null` ile reddedilir; nesneler kimlikleriyle, sütunlar adlarıyla söylenir, hiçbir şey
yazılmaz. Zorunlu alan nesnede boşaltılamaz (`attribute_required`, ADR 0199 §2). Bu ret, alanı zorunlu olmayan eski bir
katman içindir.

### 5. `gpkg_extensions` gerekince açılır

KentOS'un yazdığı her eklenti satırından önce tablo yoksa belirtimdeki tanımıyla açılır (`ensure_extensions`): kalıcı
kimlikler, soy, R-ağacı, eğri türleri, üst veri ve ilişkiler.

### 6. Sonra: `cad.sources.query`'nin `invalid_file_path`'i

ADR 0740'ın ortak durumu bu kodu veriyordu ama katalog anmıyordu; kataloğa eklendi.

## Doğrulama

- `python3 scripts/fixtures/source_defaults_cases.py --check`: `fixtures/commands/v1/cad.sources.defaults.json`'un 21
  durumu. Kurallar KentOS kodu olmadan sqlite3'ün `PRAGMA table_info`'sundan okunur. Örnek dosya `kurallar.gpkg`'yi
  `--files` GDAL ve sqlite3 ile yazar.
  - Bağlanınca alanların on kuralı.
  - Yeni nesnenin varsayılanları; hesaplanan sütunun tablodan dönmesi; yazdıktan sonra okumanın bir şey
    değiştirmemesi; değer verilen sütun; iki yeni nesne; boşaltılan varsayılanlı NOT NULL sütun.
  - `source_not_null`'un dört durumu (planda da); zorunlu alanın boşaltılamaması.
  - Rust'ta (`KENTOS_CASES_ONLY=cad.sources.defaults cargo test -p kentos-native-application --test all
    every_command_case`) ve Python'da (`test_command_cases`) aynı sonuç.
- `cargo test -p kentos-native-application --lib geopackage::tests`: varsayılanın PostgreSQL ve SQLite yazımları,
  türe çevrilmesi, alanın zorunluluğu.
- `/usr/bin/python3 scripts/fixtures/postgis_cases.py --only a_new_row_takes_the_tables_defaults`: geçici docker
  PostGIS'inde (5432'ye dokunulmaz) sahibinkine benzeyen `kurallar.ilce` tablosu. Tabloda identity, `uuidv7()`, `now()`,
  `'{}'::jsonb`, `'İlçe'`, `(-1)` ve NOT NULL sütunlar var.
  - Bağlanınca alanların kuralları geçer.
  - Yalnız adıyla yazılan ilçenin nesnesi veritabanının UUID'sini ve zamanını alır; psycopg2 tabloda aynısını okur.
  - Ardından okumak bir şey değiştirmez.
  - Adsız ilçe `source_not_null` ile reddedilir.
- Dokunulan sandıkların testleri (GeoPackage, uygulama katmanı, PostGIS, başsız sunucu) ve masaüstünün bütün testleri
  (1388) geçer.
  - Mimari ajanın ADR 0734'ünden kalan `walls_partition` testi yaylı bölmenin reddini bekliyor. Bu değişiklikle
    ilgisizdir.
  - `clippy -D warnings`.
- `cargo test -p kentos-desktop --bin kentos-cad district_form_screens -- --ignored --test-threads=1`: sahibin tablosunun
  kurallarıyla üretilen form, `.run/shots/form-ilce-*`. Formda `il_id*` ve `ad*` zorunludur.

## Kalan

- Formun yeni kaydı katman alanının varsayılanını göstermiyor (komut oluştururken veriyor): formun oturumu ve bağımsız
  ikizi birlikte (ADR 0751).
- Veritabanının hesapladığı sütunun formda “veritabanı verir” diye söylenmesi.
- Yabancı anahtarın (`il_id → kbs.iller`) başka katmandan liste ya da ilişki olarak önerilmesi.
