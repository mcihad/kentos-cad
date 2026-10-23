# ADR 0505: PostGIS tablosu bağlı katman olarak

- **Durum:** kabul edildi (2026-10-17). `plan/gis.md` CBS-04'ün ilk dilimi: kurumun var olan PostGIS tablosu ADR 0503'ün
  bağlı katmanıyla açılır, düzenlenir ve tek veritabanı işleminde geri yazılır.
- **Bağlam belgesi:** ADR 0503 (bağlı katman), 0504 (kaynağın sistemi), 0502 (GeoPackage), 0006 ve 0007 (sunucunun
  PostGIS'i).
- **Bağımlılık:** sahibin onayı (17 Ekim): “sqlx, TLS ile”. Kilide yeni üçüncü taraf crate girmedi: sqlx ve tokio
  workspace'teydi, rustls ve sistemin kök sertifikaları servis istemcisiyle zaten kilitteydi. sqlx'e
  `tls-rustls-ring-native-roots` özelliği eklendi.
- **Web:** ertelendi (ADR 0228). Web'de gerekenler TODOS.md §0.2'de.
- **Sonraya kalan:**
  - kutu, sütun ve süzgeç sorgusunu kaynağa itmek, milyon satırlı tabloda sayfa sayfa ilk harita (bugün tablo bütün okunur);
  - view'lar ve salt okunur sorgu katmanı (SQL);
  - geometri sütunu birden çok olan tabloda sütun seçimi (bugün ilki);
  - tetikleyici ya da outbox ile değişiklik izleme (bugün sürüm, satırların SHA-256'sı);
  - bağlantı ayarlarının proje dışında adla saklanması (ADR 0208'in bağlantıları gibi);
  - Python ve MCP'de PostGIS'e özgü yardımcılar (komutlar bugün de çalışıyor).

## Bağlam

ADR 0503 bağlı katmanı GeoPackage dosyasıyla kurdu. Kurumların asıl verisi PostGIS'tedir; plan, tabloyu kopyalayıp KentOS
şemasına çevirmeden açmayı ister. İki engel vardı:

- **Komutlar saf.** `kentos-native-application` çalışma zamanı ve ağ bilmez. `scripts/arch/deps.mjs` tokio ve sqlx'i
  uygulamada, başsız sunucuda, Python'da ve MCP'de yasaklar.
- **Parola.** Proje dosyası paylaşılır, buluta gider. Parola ona yazılmamalı.

## Karar

### 1. Sağlayıcı arayüzü

- Uygulamada `provider::TableProvider`:
  - `tables(adres)`: veritabanının tabloları;
  - `read(adres, tablo)`: tablo bütün, sürümüyle;
  - `edit(adres, tablo, sürüm, değişiklikler)`: tek işlemde yazar, eklenenlerin anahtarlarını ve yeni sürümü verir.
- Tablo GeoPackage tablosu gibi taşınır: `kentos_geopackage`'ın katmanı, satırları, değerleri ve `Edits`'i. Karşılaştırma,
  fark ve yazma kuralları iki kaynakta aynı koddur.
- Ev sahibi sağlayıcıyı başlarken bir kez kaydeder (`register_postgis`):
  - masaüstünün `main`'i;
  - Python modülünün açılışı;
  - MCP sunucusu.
- Sağlayıcı kaydetmeyen ortamda (uygulamanın testleri, başsız sunucunun kendisi) veritabanı adresi `source_unreachable`
  ile reddedilir: “Bu ortamda PostGIS sağlayıcısı yok”.

### 2. `kentos-postgis`

- Ayrı crate, `deps.mjs`'in `postgis` grubu.
  - sqlx ile, TLS rustls ve sistemin kök sertifikalarıyla.
  - Kendi tek iş parçacıklı tokio çalışma zamanı: komutlar eşzamanlı kalır.
  - Python, MCP ve masaüstü onu kullanır; tokio ve sqlx bu ev sahiplerine yalnız onun üzerinden girer (yasak listelerinden
    çıktılar, axum ve sunucu çatıları yasak kalır).
- **Tablolar:** tek tam sayı birincil anahtarlı tablolar listelenir. Anahtarsız tablo okunmaz, listelenmez.
- **Geometri:**
  - geometri tablosu `geometry_columns`'tadır, sistem oradaki SRID'dir;
  - geometri `ST_AsBinary` ile ISO WKB okunur, GeoPackage'ın WKB okuyucusuyla;
  - geri `ST_SetSRID(ST_GeomFromWKB($n), srid)` ile yazılır.
- **Sütunlar:** türleri `format_type`'tan okunur.
  - Tam sayı, gerçek sayı, mantıksal, tarih ve metin kendi türüyle okunur.
  - `numeric`, zaman ve öbür türler metin okunur ve `CAST($n AS tür)` ile yazılır, böylece kesin sayı double'dan geçmez.
- **Sürüm:** satırların anahtar sırasıyla okunan içeriğinin SHA-256'sı. `xmin` kullanılmaz (plan: kalıcı sürüm değil).
- **Yazma:** tek işlem.
  1. Tablo `SHARE ROW EXCLUSIVE` ile yazarlara kilitlenir.
  2. Sürüm kilidin altında yeniden okunur. Farklıysa hiçbir şey yazılmaz: `source_changed`, çakışma.
  3. Sütunlar `ALTER TABLE … ADD COLUMN` ile eklenir.
  4. Satırlar güncellenir, eklenir (`RETURNING` anahtar), silinir.
  5. Eklenenlerin kalıcı kimlikleri şemanın `kentos_uids` tablosuna yazılır (tablo adı, anahtar, uuid); silinenlerinki
     silinir.
  6. Yeni sürüm aynı işlemde okunur.
- **Kalıcı kimlik:** KentOS'un yazdığı satır kimliğini `kentos_uids`'ten alır. Öbürleri veritabanının adı, tablosu ve
  anahtarından UUIDv5'tir (`pg:veritabanı/şema.tablo:anahtar`): aynı satır her bağlamada aynı kimliği alır.

### 3. Adres ve parola

- Bağın `kind`'i `postgis`, `path`'i adrestir: `postgresql://kullanıcı@sunucu:5432/veritabanı?sslmode=require`.
- Adreste parola olamaz (kullanıcı bilgisinde ya da `password=` parametresinde). Sunucu ya da veritabanı eksikse
  `invalid_address`, hiçbir şeye ulaşmadan.
- Aynı kural `.kcad`'in okuyucularında: sözleşmenin `sources_problem`'i ve bağımsız Python okuyucusu. Parolalı adresli
  dosya bozuk sayılır (`broken/sources-password.kcad`).
- Parola `PGPASSWORD`'dan ya da `~/.pgpass`'tan okunur. (ADR 0623'ten beri önce bu oturumda yazılan, sonra cihaza
  kaydedilen bağlantının parolası; pencerede adres yazılmaz, kayıtlı bağlantı seçilir.)
- Ulaşılamayan veritabanı `source_unreachable` (bağlantı 15 saniyede kurulmazsa da). Bağlı katmanın durumunda `found`
  false.

### 4. Komutlar

- `cad.sources.describe`: `path` adresse veritabanının tabloları (`şema.tablo`), GeoPackage'ınkiyle aynı biçimde.
- `cad.sources.edit`:
  - `link` adresle tabloyu yeni katmana alır (“PostGIS: tablo” adımı) ya da var olan katmana bağlar;
  - `relink` adresi değiştirir; tablo aynı içerikte olmalı. Dosya ile veritabanı arasında yeniden bağlanmaz
    (`invalid_address`);
  - `unlink` veritabanına ulaşmadan çözer.
- `cad.sources.sync`: GeoPackage'taki kurallarla, yazma sağlayıcının işlemiyle.
- Retlerin yolları ayrılır:
  - ulaşılamayan adres adresin alanını gösterir (`edit.path`);
  - olmayan tablo tablonunkini gösterir (`edit.table`).
- ADR 0504'ün `reproject`'i aynen geçer: başka sistemdeki tablo projenin sisteminde açılır, kendi sisteminde yazılır.

### 5. Masaüstü

- Veri › Dosya alışverişi'nde **PostGIS'e bağlan…** (`source.postgis`, ikon `sourceDatabase`). Pencere:
  - **Adres** ve parolanın nereden okunduğu;
  - **Tabloları listele** (`cad.sources.describe`, arayüzün iş parçacığının dışında);
  - tablolar türleri, sistemleri, satır sayıları ve salt okunurlarıyla;
  - başka sistemdeki tablo seçiliyse **Projenin sistemine çevir**;
  - **Bağla** her tabloyu `cad.sources.edit` ile bağlar.
- Bağlı katmanın rozeti, Kaynağa yaz ve Kaynaktan oku ADR 0503'teki gibidir.

## Doğrulama

- `scripts/fixtures/postgis_cases.py`:
  1. Docker'da `postgis/postgis:18-3.6`'yı boş bir portta açar; 5432'deki veritabanına dokunulmaz.
  2. QGIS'in GeoPackage'ını ve WGS 84 dosyasını GDAL'ın `ogr2ogr`'uyla `kadastro` şemasına yükler.
  3. `kentos-postgis`'in uçtan uca testini koşar: bağla, düzenle, yaz, başka çizimde yeniden bağla, başkası olarak yaz,
     çakışma, geri okuma, başka sistemdeki tablo.
  4. Masaüstünün penceresiyle listeler ve bağlar.
  5. KentOS kodu olmadan psycopg2'yle denetler:
     - KentOS'un söylediği tablolar kataloğa uyar;
     - alınan nesneler satırlara uyar;
     - yazmadan sonra tablo: yeniden adlandırılan ve silinen parseller, `durum` metin sütunu, eklenen üçgen 50 m² ve
       kalıcı kimliği `kentos_uids`'te.
- Ortak durumlar (`bound_sources_cases.py`), adres retleri: describe'da 4, edit'te 7. Sağlayıcıya bağlı retler ortak
  durumlarda yok: Python sağlayıcıyı kaydeder, uygulamanın testleri kaydetmez. Onlar `sources.rs`'in sağlayıcısız
  testinde ve `kentos-postgis`'in testlerinde.
- Resimler: `postgis_cases.py --screens`, `.run/shots/postgis-*`.
