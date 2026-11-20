# ADR 9100: Veri sürücüleri — ortak sözleşme, GeoPackage ve PostGIS sürücüleri

- **Durum:** kabul edildi (2026-11-20). Plan: [plan/veri.md](../../plan/veri.md) VY-01.
- **İstek:** sahibin sözü (20 Kasım 2026): “Geopackage ve portgis için 1. sınıf destek istiyorum ama driver soyutlaması yap ki
  belki ileride Oracle MySQL SQL server entegre edebiliriz, driver soyutlaması ve driverların her biri ayrı crate olacak
  … postgresql bağlanınca bir katmanı ekliyorum ama bütün veriyi önbelleğe alıyor … Gerçek zamanlı olarak bunu
  veritabanından çekmesi lazım … alttaki panellerde de datalar çok yavaş filtreleniyor aranıyor … bunlar da doğrudan
  veritabanından sorgulanmalı.”
- **Önceki kararla ilişkisi:** [plan/gis.md](../../plan/gis.md)'nin “Oracle veya başka ticari veritabanı olmayacaktır” kuralı
  sürücü *yazılmaması* olarak kalır; sözleşme ise ileride yazılabilsin diye bugünden kurulur (sahibin 20 Kasım kararı).
  ADR 0757'nin “bağlı katman çalışma kopyası olarak kalır” kararı VY-02/VY-03'te kaldırılır; bu ADR onların veri yolunu
  kurar. ADR 0581'in `cad.sources.query`'si ve uygulama crate'indeki `TableProvider` bugünkü bağlı katmanlar için
  durur; yeni veri katmanı yalnız bu sürücüleri kullanır.
- **Web:** ertelendi (ADR 0228). **`.kcad`:** değişmez (katman türü VY-02'de). **Komut:** yok (sürücü katmanı).
- **Bağımlılık:** yeni dış kitaplık yok: `rusqlite` (GeoPackage), `sqlx` + `tokio` (PostGIS) çalışma alanında vardı.

## §1 Bağlam

Bugün bir PostGIS ya da GeoPackage tablosu “bağlı katman” olarak eklenince bütün satırları okunur, çizimin nesnelerine
çevrilir ve katmana yazılır (ADR 0503, 0505, 0757). 50 000 satırlık tablo 1,5–3 saniye sürüyor, milyonluk tablo
belleği dolduruyor. Tablo paneli, arama ve süzme de o kopyanın üstünde çalışıyor. `kentos-postgis` her istekte yeni bir
bağlantı açıyor; sağlayıcı arayüzü (`TableProvider`) uygulama crate'inin içinde ve GeoPackage satır türlerine bağlı.

## §2 Araştırma

**Masaüstü CBS'nin veri sağlayıcı mimarisi** (QGIS 4.2.3 bu makinede kurulu; ekranları ayrı profille çekildi,
`.run/gui-evidence/veri/qgis/`; kaynak kodu ve kılavuzu okundu):

- Katman eklemek tabloyu okumaz; sağlayıcı yalnız kataloğu ve şemayı sorar. Harita her çizimde görünen alanı ister
  (`QgsFeatureRequest`: kutu, süzgeç ifadesi, sütun alt kümesi, sıralama, sınır, sadeleştirme).
- PostgreSQL sağlayıcısı satırları sunucu imleciyle okur: `DECLARE … CURSOR`, `FETCH FORWARD n` (qgspostgresfeatureiterator.cpp).
  Kutu `geom && st_makeenvelope(…)` olur; ifade derlenebiliyorsa SQL'e gider, derlenemiyorsa yerelde sınanır. Sıralama
  bugünkü sürümde yerelde yapılır (kaynakta `#if 0`); sınır yalnız her şey SQL'e gidince sunucuya verilir.
- Sadeleştirme sunucuda yapılabilir (“Simplify on provider side if possible”): eşik 1 pikseldeyse
  `st_removerepeatedpoints` + `st_simplify(…, tol·0,7, true)`, değilse `st_simplifypreservetopology(…, tol·0,8)`;
  harita 250 ms'de bir güncellenir (Seçenekler › Görüntüleme).
- Bağlantılar havuzdan alınır (`QgsPostgresConnPool`); bir işlem grubu varsa iterator o bağlantıyı paylaşır.

**OGC GeoPackage 1.4.0 (OGC 12-128r19, 6 Şubat 2024):** `gpkg_contents` içerik çizelgesidir; mekânsal dizin
R*Tree uzantısıdır (Ek F.3, `rtree_<tablo>_<sütun>` ve tetikleyicileri); geometri `GP` başlıklı WKB'dir.

**Türkiye mevzuatı:**

- 49 sayılı Coğrafi Bilgi Sistemleri Hakkında Cumhurbaşkanlığı Kararnamesi (RG 7.11.2019/30941) md. 6(1)(d) ve 7221
  sayılı Kanun (14.2.2020): kamu kurumları ürettikleri coğrafi veriyi ve veri bilgisini Ulusal Coğrafi Bilgi
  Platformu'na sunar.
- Coğrafi Veri İzinleri Yönetmeliği (RG 10.2.2021/31391; değişiklik RG 31.3.2023/32149) md. 4/A ve 13(3): veri
  “ulusal standartlara uygun olarak” paylaşılır, veri bilgisi güncel tutulur.
- TUCBS Birlikte Çalışabilirlik Usul ve Esasları Rehberi Sürüm 1.0 (2020) §3.2.11 Bilginin Korunması: “Dosya biçimleri
  uzun süre erişilebilirliği garantileyebilecek şekilde seçilmeli”. GeoPackage açık bir OGC standardıdır.
- Mevzuat bir veritabanı sürücüsünün iç yapısı, düzenleme oturumu ya da işlem yönetimi hakkında bir şey söylemez; bu
  kararlar KentOS'undur.

## §3 Karar: ortak sözleşme `kentos-data` (`crates/native/data`)

Saf crate: veritabanı, çalışma zamanı, ağ ya da arayüz yok. İçindekiler:

- **Değer (`Value`)**: `Null, Bool, Int, Real, Decimal(metin), Text, Date, DateTime, Time (ISO metin), Blob, Json`.
  Tarihler metin kalır: hiçbir kaynak yolda basamak yitirmez; kesin ondalık metniyle taşınır.
- **Şema**: `Field` (ad, tür, kaynağın kendi tür adı, boş geçilebilir, varsayılan, kaynakça üretilen, açıklama),
  `GeometryField` (ad, OGC tür adı, Z, M, EPSG), `Schema` (tam sayı anahtar, alanlar, geometri). Türün ailesi
  (`Family`: nokta, çizgi, alan, her biri) katmanın ne çizdiğini söyler.
- **Geometri**: `kentos-geopackage`'ın saf geometri modeli ve WKB/GP blob/WKT kodekleri buraya taşındı (`geom`, `wkt`);
  `kentos_geopackage::geom` yolları yeniden ihraçla aynen durur.
- **Koşul (`Condition`)**: SQL değil, ağaç: `And, Or, Not, Compare, In, Between, IsNull, NotNull, Like (fold),
  Contains (hızlı arama), Fids`. Anlamı tek: NULL'la karşılaştırma satırı tutmaz (SQL'in üç değeri), `Like`'ın `%` ve
  `_`'ı, `\` kaçışı; `fold` Türkçe harfleri büyük–küçük ayrımı olmadan karşılaştırır (`İ`→`i`, `I`→`ı`). `holds` aynı
  anlamın Rust'taki yazımıdır; sürücüler kendi SQL'lerini yazar.
- **İstek (`Request`)**: kutu (tablonun sisteminde), `exact` (kutunun değil geometrinin kendisinin değmesi), koşul,
  sütunlar, geometri okunsun mu, çizim için sadeleştirme toleransı, sıralar (NULL'lar sonda, eşitler anahtara göre),
  sınır, atlama, parça boyu.
- **Sürücü sözleşmesi**: `Driver` (adresi tanır, kaynağı açar), `Source` (`tables`, `schema`, `capabilities`, `extent`,
  `count`, `features` akışı, `unique`, `begin`), `Transaction` (`insert`, `update`, `delete`, `create_table`,
  `drop_table`, `commit`, `rollback`; düşürülünce geri alır). Eşzamansız istemcili sürücü çalışma zamanını içinde tutar;
  çağıranlar eşzamanlı kalır.
- **Kayıt (`Registry`)**: ev sahibi sürücüleri bir kez kaydeder; her adres için tek kaynak açılır ve paylaşılır (bir
  veritabanının bağlantı havuzu bütün katmanların, tablo sayfalarının ve pencerelerin ortak havuzudur).
- **Parolalar (`Secrets`)**: adres parola taşımaz (proje dosyası paylaşılır); ev sahibi parolayı nerede tuttuğunu bir kez
  kaydeder, sürücü adresin parçalarıyla sorar.
- **Değer denetimi (`coerce`)**: bir değer alanına uymuyorsa yazılmaz ve nedeni söylenir (ondalık tam sayı alanında
  kendiliğinden yuvarlanmaz, sözcük tarih alanına düşmez).
- **Durdurma (`Cancel`)**: görünüm başka yere gittiğinde ya da Durdur'a basıldığında okuma iki parça arasında biter.
- **Hatalar (`DataError`)**: ulaşılamadı (çevrimdışı, zaman aşımı, giriş reddi, diğer), tablo yok, alan yok, izin
  yok, desteklenmiyor, geçersiz, kısıt (denetim, teklik, başvuru, boş geçilemez), çakışma, durduruldu, başarısız;
  her biri Türkçe, çözümüyle.

## §4 Karar: istek kaynağa gider

Kaynak ne yapabiliyorsa yapar, kalanı sürücü yapar; dönen satırlar iki durumda da aynıdır, yalnız süre değişir.

- Kutu mekânsal dizinden (GeoPackage R*Tree, PostGIS GiST `&&`) sorulur.
- Koşul, sıralama ve sayfa SQL'e gider.
- Sadeleştirme: PostGIS'te sunucuda (`ST_Simplify(g, tol, true)`: çökmüş parçalar korunur; yay içeren ya da nokta
  geometri olduğu gibi). GeoPackage'da (SQLite'ın işlevi yok) okunduktan sonra Douglas–Peucker'la (Douglas & Peucker
  1973). Halka dört konumun altına inmez, uçlar kalır.

## §5 Karar: GeoPackage sürücüsü `kentos-data-gpkg` (`crates/native/data-gpkg`)

- Adres: `.gpkg` ile biten yol ya da `gpkg:` öneki.
- Okuma bağlantıları (en çok 4, salt okunur) saklanır ve iş parçacıklarınca paylaşılır.
- Bir istek tek SQL deyimidir:
  - kutu `t.fid IN (SELECT id FROM rtree… WHERE …)`; dizin yoksa geometrinin kendi kutusu (`ST_MinX` …);
  - Türkçe katlama `kentos_fold` işleviyle, büyük–küçük harf ayıran `LIKE` `kentos_like` işleviyle (SQLite'ın kendi
    `LIKE`'ı yalnız ASCII harfleri katlar);
  - sıralama `NULLS LAST`, sonra anahtar.
- Yazma kendi bağlantısında `BEGIN IMMEDIATE`'la başlar: ikinci yazıcı yarıda düşmez, kilidi bekler (10 s).
  - R*Tree dosyanın tetikleyicileriyle izler.
  - İşlem biterken değişen tablonun `gpkg_contents` kutusu ve zamanı yazılır.
  - Silinen satırın kalıcı kimliği (`kentos_uids`) de gider.
- Yeni tablo `kentos-geopackage`'ın tablo kurucusuyla gelir (`fid` anahtar, R*Tree ve tetikleyicileri). Bilinmeyen
  sistemin EPSG kaydı eklenir.

## §6 Karar: PostGIS sürücüsü `kentos-data-postgis` (`crates/native/data-postgis`)

- Adres `postgresql://` ya da `postgres://`; içinde parola varsa açılmaz.
- Veritabanı başına bir havuz (en çok 4 bağlantı, 5 dakika boşta kalınca kapanır). Her bağlantı `DateStyle ISO, YMD`,
  `standard_conforming_strings on` ve `application_name KentOS` ile kurulur. TLS sistemin kök sertifikalarıyla kurulur.
- Katalog: `pg_class` (tablo, görünüm, gerçekleştirilmiş görünüm, bölümlü tablo), `geometry_columns`,
  `geography_columns`, `reltuples` (tahmini satır), yorum, yazma hakkı.
  - Yazılabilir olmak için tek sütunlu tam sayı birincil anahtar gerekir.
  - Görünümün anahtarı `fid`, `id`, `gid` ya da `ogc_fid` adlı tam sayı sütunudur.
- Satırlar salt okunur bir işlemde sunucu imleciyle (`DECLARE kentos_rows NO SCROLL CURSOR`, `FETCH FORWARD n`) okunur;
  okuyan bitince ya da durdurulunca `CLOSE` gelir.
- Değerler metin olarak okunur (`::text`, ikili veri onaltılık): her tür aynı yolla çözülür.
- Karşılaştırmada değer alanın türüyle bağlanır; tarih ve saat sütunun kendi türüne çevrilir. Türü uymayan değer satır
  tutmaz.
- Türkçe katlama SQL'de yapılır, veritabanının harmanlamasından bağımsızdır:
  `lower(translate(x, 'İIÇĞÖŞÜ', 'iıçğöşü'))`.
- Yazma:
  - değer `coerce`'tan geçip sütunun kendi türüne çevrilerek yazılır;
  - geometri sütunun türüne (tek parça → çoklu) ve boyutlarına (`ST_Force2D/3DZ/3DM/4D`) uydurulur;
  - yeni tablo kimlikli (`GENERATED BY DEFAULT AS IDENTITY`) anahtar ve tipli geometri sütunuyla, GiST diziniyle kurulur.
- SQLSTATE'ler `DataError`'a eşlenir: 23xxx kısıtlar, 42501 izin, 28P01 giriş, 40001 çakışma.

## §7 Ev sahibi ve parola

Masaüstü, başsız ev sahibi, Python ve MCP açılışta `kentos_data_gpkg::register()`, `kentos_data_postgis::register()`
ve parola bulucusunu kaydeder (VY-02 ile). Parola sırası: ev sahibinin bulucusu (cihazın kayıtlı bağlantıları, oturum),
`PGPASSWORD`, `~/.pgpass`.

## §8 Uygunluk takımı

`kentos_data::conformance::run` (özellik `conformance`) her sürücüde aynı veri kümesini sürücünün kendisiyle kurar ve
sorar. Veri kümesi:

- 600 parsel, doğrusal eşlik üretecinden: `x' = (1103515245 x + 12345) mod 2³¹`, tohum 2026;
- NULL değerler ve geometrisiz bir satır;
- Türkçe harfli metinler.

Sorulanlar: şema, bütün satırlar, dört kutu, on beş koşul (sayılarıyla), üç sıralamada üç sayfa, sütun alt kümesi,
farklı değerler, sadeleştirme, durdurma, geri alınan ve yazılan işlem, olmayan satırın reddi. Her cevap veri kümesinden
kaba kuvvetle hesaplananla karşılaştırılır, başka bir sürücünün cevabıyla değil. Metin sıralaması yalnız ASCII alanla
sınanır: harmanlama kaynağındır.

Yeni bir sürücü (Oracle, MySQL, SQL Server) kaydedilmeden önce bu takımı geçer.

## §9 Ölçümler (20 Kasım 2026, release, i5-11300H)

Bir milyon parsel: 1000 × 1000 hücre, 30 m, 7 köşeli çokgen. Yollar:

- PostGIS 18 + PostGIS 3.6.3, aynı makinede docker: `kentos-data-postgis` `tests/perf.rs`;
- GeoPackage, yerel disk: `kentos-data-gpkg` `tests/perf.rs`.

Görünümler `kullanim` sütunuyla, çizim toleransıyla okundu.

| İş | PostGIS | GeoPackage |
|---|---:|---:|
| Kaynağı açma (ilk bağlantı) | 4,3 ms | 0,2 ms |
| Katalog | 17 ms | 26 ms (satır sayılarıyla) |
| Şema | 18 ms | 0,1 ms |
| Sokak 400 × 300 m (140 satır) | 11 ms | 0,2 ms |
| Mahalle 3 × 2 km (6 700 satır) | 173 ms (ilk parça 73 ms) | 6 ms |
| İlçe 12 × 8 km (106 800 satır) | 311 ms (ilk parça 68 ms) | 89 ms |
| Bütün kent 30 × 30 km (1 000 000 satır) | 1 258 ms (ilk parça 20 ms) | 819 ms (ilk parça 218 ms) |
| Süzgeçli sayım (`kullanim = Park`, 250 000) | 40 ms | 56 ms |
| Tablo sayfası, alana göre (ilk 50) | 231 ms | 62 ms |
| Tablo sayfası, 500 000. satırdan | 541 ms | 727 ms |
| Hızlı arama (“EĞİT”, ilk 50) | 2 ms | < 1 ms |

Karşılaştırma: ADR 0757'ye göre bugünkü bağlama 50 000 satırda 1,56–3,05 s sürüyor ve bütün satırları belleğe
yazıyor. Sürücüyle katman eklemek yalnız katalog ve şemadır (≈ 35 ms). Görünüm yalnız görüneni okur.

**Bütçe** (VY-03'ün kabulü):

- ilk parça 100 ms'nin altında;
- sokak ve mahalle ölçeğinde görünüm 200 ms'nin altında;
- bütün kentte çizim sürerken arayüz donmaz (okuma iş parçacığında, parça parça).

## §10 Kapsam dışı

- Oracle, MySQL ve SQL Server sürücüleri yazılmadı; sözleşme onları bekliyor.
- Birden çok geometri sütunlu tabloda ilki okunur; seçim VY-02'nin katman kartında.
- Tam sayı olmayan birincil anahtarlı tablo bugün okunamaz.
- Yazılabilir görünüm yazılmaz.
- `LISTEN/NOTIFY` ile canlı yenileme ileride.

## Uygulama

- `crates/native/data` (`kentos-data`): `value`, `schema`, `catalog`, `condition`, `request`, `feature`, `edit`,
  `driver`, `registry`, `secrets`, `coerce`, `fit`, `simplify`, `cancel`, `error`, `conformance`; `geom`/`wkt`
  `kentos-geopackage`'dan taşındı.
- `crates/native/data-gpkg` (`kentos-data-gpkg`): `source`, `sql`, `convert`, `write`.
- `crates/native/data-postgis` (`kentos-data-postgis`): `connect`, `catalog`, `sql`, `read`, `write`.
- `kentos-geopackage`: `connection()`, `create_table_in`, `drop_table_in`, `registered_name_in`, `refresh_extent_in`.
- `scripts/arch/deps.mjs`: `data`, `data-gpkg`, `data-postgis` grupları; `geopackage`'ı kullanan her grup `data`'yı da.

## Doğrulama

- `cargo test -p kentos-data`: 15 birim testi (koşulun üç değerli anlamı, Türkçe katlama, `LIKE`, sadeleştirme,
  değer denetimi, uydurma, geometri kodekleri).
- `cargo test -p kentos-data-gpkg`: uygunluk takımı iki kez (tablo yeniden kurulur), 80'i aşkın denetim, hepsi geçti.
- `KENTOS_DATA_PG=… cargo test -p kentos-data-postgis`: aynı takım PostGIS 3.6.3'te geçti; adresinde parola olan
  bağlantı reddedildi.
- `cargo test -p kentos-geopackage`: taşımadan sonra testleri geçti.
- Ölçümler §9'da.
- `cargo clippy` yeni crate'lerde temiz.
