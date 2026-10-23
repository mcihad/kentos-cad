# ADR 0596: Paketle'de kaynakların yolu: kopya, bağ ya da gömülü; dışarıda bırakılanlar

- **Durum:** kabul edildi (2026-10-23). `plan/gis.md` CBS-36'nın kalanı (ADR 0506'nın “sonraya”sı):
  - PostGIS tablosunun çevrimdışı kopyası, GeoPackage olarak pakete;
  - bağlı katmanın bağını pakette gömülü kopyaya çevirme;
  - lisans ve erişim sınırına göre dışarıda bırakma.
- **Bağlam belgesi:** ADR 0503 (bağlı katman), 0505 (PostGIS), 0506 (göreli yol ve Paketle), 0510 (alanla bağlama),
  0538 (uzlaştırma tabanı), 0592 (kaynaktan okuma).
- **Web:** ertelendi (ADR 0228), TODOS.md §0.2.
- **`.kcad` şeması:** değişmez.
- **Sonraya kalan:**
  - formun `redact` alanlarını paketten kesmek (bugün çalışma kopyası projenin içindedir, kesmek bir şeyi korumaz;
    kesmek için önce paketin çalışma kopyasından da kesilmesi tasarlanmalı);
  - stillerin, formların ve modellerin dış bağımlılıklarının pakete toplanması (bugün projenin içindeler);
  - uzlaştırma tabanının veritabanı kopyasına taşınması (bugün düşer, §2);
  - bir kaynağı kaynak başına ayrı yolla seçmek (bugün tür başına: GeoPackage'lar ve veritabanları).

## Bağlam

Paketle (ADR 0506) bağlı GeoPackage'ları kopyalıyor, veritabanı bağını olduğu gibi bırakıyordu. Paketi açan bilgisayar
veritabanına ulaşamazsa katman yalnız projenin içindeki çalışma kopyasıyla kalıyor, bağı da kopuk görünüyordu. Plan
şunları ister: “bağlı kaynak, gömülü snapshot ve offline çalışma kopyası kullanıcı seçimi olsun”, “Paketle komutu
lisans/erişim sınırına göre asset'leri ve kaynakları toplasın”. Parola, belirteç ve imzalı adres yine projeye girmez.

## Karar

### 1. Kaynağın yolu

`file.package` iki seçim alır; her biri `copy`, `keep` ya da `embed`:

- **`geopackages`** (varsayılan `copy`): bağlı GeoPackage'lar;
- **`databases`** (varsayılan `keep`): bağlı PostGIS tabloları.

Anlamları:

- **`copy`:** kaynak pakete kopyalanır, bağ kopyayı gösterir. GeoPackage dosyası bayt bayt gider (ADR 0506). Veritabanı
  tablosu GeoPackage'a yazılır (§2).
- **`keep`:** bağ olduğu gibi kalır, dosya kopyalanmaz. Kalanlar listesinde nedeni `kept` (GeoPackage) ya da `database`
  (veritabanı, ADR 0506'daki gibi). Bağın dosyası aranmaz.
- **`embed`:** paketin projesinde bağ kalkar; katman, nesneleriyle birlikte projenin kendi katmanı olur
  (`cad.sources.edit`'in `unlink`'iyle aynı). Bağın uzlaştırma tabanı da kalkar. Dosya kopyalanmaz, aranmaz. Cevabın
  `embedded`'ı her gömülen bağı söyler: kimliği, katmanı, kaynağı, tablosu, nesne sayısı. Alanla bağlı katman (ADR 0510)
  yalnız alanının nesneleriyle gömülür.

Seçimler çizimi değiştirmez: yalnız paketin projesine yazılır.

### 2. Veritabanı tablosunun kopyası

`databases: copy` ile her PostGIS bağı için:

1. Bağın durumu sorulur (`sources::layer_state`). Tablo, katmanın son eşleşmesinden beri değiştiyse paket yapılmaz:
   `source_changed`, yolu `databases`. Önce tablodan okunur ya da uzlaştırılır. Böylece kopya, katmanın son eşleştiği
   hâldir ve bağın özeti (`digest`) geçerli kalır.
2. Kayıtlar, `cad.sources.query`'nin okuduğu yoldan okunur (`sources_query::stage`, ADR 0592). Alanla bağlı katmanda yalnız
   alanın kayıtları gelir: kopya hiçbir zaman bağın okuduğundan fazlasını taşımaz. En çok bir milyon kayıt
   (`input_too_large`).
3. Kayıtlar projenin sisteminde, kalıcı kimlikleriyle bir GeoPackage'a yazılır (`geopackage::write`). Tablonun adı
   tablonun kısa adıdır (`public.parsel` → `parsel`); alanları tablonun sütunlarıdır.
4. Dosya `veri/<şema>-<tablo>.gpkg` olarak pakete girer (ad bir kez alınır, ADR 0506'nın kuralı). Cevaptaki dosyanın
   `from`'u veritabanının adresidir (parolasız), `table`'ı tablonun adı, `rows`'u kayıt sayısı.
5. Paketin projesinde bağ GeoPackage bağı olur:
   - `path` paketteki kopya, `relative` `veri/…`;
   - `table` yazılan tablonun adı;
   - `revision` kopyanın SHA-256'sı;
   - `srid` projenin sistemi (kayıtlar projenin sisteminde yazıldı);
   - `digest`, `extent`, `key`, `parked` olduğu gibi.
   - Uzlaştırma tabanı düşer: tablonun satır kimlikleri kopyada başkadır. Yeniden okumada katman kopyayla eşleşir.

Bağın okuduğu satır yoksa tablo yazılmaz: GeoPackage boş tablo tutmaz, bağ gömülür (katman zaten boştur).

Ulaşılamayan veritabanı `source_unreachable` ile reddedilir, yolu `databases`. Veritabanı okuması yalnız planda ve
yürütmede yapılır; `validate` okumaz. Planda yazılan kopya geçici bir dosyadır, plan bırakılınca silinir.

### 3. Dışarıda bırakılanlar

`leaveOut`: pakete girmeyecekler. Her biri şunlardan biridir:

- **bağın kimliği** (`kaynak-N`): bağ kalkar, katmanın nesneleri paketin projesinden çıkarılır (katman boş kalır).
  Lisansı dağıtmaya izin vermeyen veri böyle pakete hiç girmez. Bağı bırakmak tehlikeli olurdu: boş katman bağın özetiyle
  karşılaştırılınca bütün satırlar silinmiş görünür.
- **bir nesnenin kalıcı kimliği:** nesnenin dosyaları (resim, raster, bulut, tablo, ek) kopyalanmaz, bağları olduğu gibi
  kalır. Nesne pakettedir.
- **`xref:<ad>`:** dış başvurunun kaynağı kopyalanmaz, bağı kalır.

Kalanlar listesinde nedeni `excluded`. Bağ için `objects` çıkarılan nesnelerin sayısıdır. Seçimlerden önce gelir: bırakılan
bağ ne kopyalanır ne gömülür. Var olmayan bir şeyi gösteren öğe `invalid_input`, yolu `leaveOut[i]`.

### 4. Retlerin sırası

1. `invalid_file_path`, `file_exists` (ADR 0506);
2. `invalid_input` (`leaveOut`);
3. `source_unreachable`, `source_changed`, `input_too_large` (veritabanı kopyası);
4. `file_unwritable`.

### 5. Masaüstü

Paketle, çizimde bağlı katman varken önce küçük bir pencere açar:

- **GeoPackage kaynakları:** Kopyala, Bağlı bırak, Göm;
- **Veritabanı kaynakları:** Bağlı bırak, Kopyala, Göm;
- bağların listesi, her birinde “Pakete al” onayı (kaldırılan `leaveOut`'a gider).

Klasör ondan sonra sorulur. Bağlı katman yoksa doğrudan klasör sorulur. Cevabın duyurusu kopyalanan tabloları ve gömülen
bağları da sayar.

## Doğrulama

- Ortak durumlar: `fixtures/package/v1/cases.json`, 28 durum. Bağımsız başvuru `scripts/fixtures/package_options_cases.py`,
  KentOS kodu olmadan: paketin dosyaları, kalanlar, gömülenler, paketin projesinin bağları ve katmanların nesne sayıları
  sqlite3 ve hashlib ile kurulur. Başsız sunucuda `package_cases` (`crates/native/headless/tests/headless.rs`) oynatır:
  çizimi kurar, paketler, paketin projesini açıp bağlarını ve katmanlarını sayar.
- PostGIS: `crates/native/postgis/tests/all.rs`'in `a_database_table_goes_into_the_package` testi, geçici docker PostGIS'inde
  (`scripts/fixtures/postgis_cases.py`). Tablo pakete kopyalanır, alanla bağlı tablonun yalnız alanı gelir, paket
  veritabanı kapalıyken açılır ve katman kopyayla eşleşir. Değişmiş tablo `source_changed` ile reddedilir. Kopya OGR'la
  okunur: satırlar, kalıcı kimlikler ve geometriler veritabanınınkiyle aynı.
- Masaüstü: `exchange::package_tests`; resimler `.run/shots/paket-*`.
