# ADR 0502: GeoPackage — kendi crate'i, içe ve dışa aktarma

- **Durum:** kabul edildi (2026-10-17). `plan/gis.md` CBS-03 (vektör ve tablo), CBS-05, CBS-06 ve CBS-44'ün GeoPackage dilimi.
- **Bağlam belgesi:** ADR 0410 (geometrisiz kayıt), 0199 (katman alanları), 0222 (içe aktarmada açık dönüşüm), 0253 §10
  (dosya komutları), 0257 (DXF içe almanın komutu), 0224 (M).
- **Sahibin kararları (17 Ekim):** SQLite için `rusqlite` ve gömülü SQLite; GeoPackage kendi crate'inde (“Geopackage için
  özel bir crate paketi yap”).
- **Web:** ertelendi (ADR 0228). Web'de gerekenler TODOS.md §0.2'de.
- **Sonraya kalan:**
  - dosyada yerinde düzenleme: bağlı kaynak, sağlayıcı oturumu ve commit (CBS-01, 02, 11, 36);
  - raster karolar ve ilişkili tablo profili (CBS-03'ün P1 uzantıları);
  - 2⁵³'ten büyük tam sayının sayı olarak taşınması ve saat dilimli zaman (CBS-06);
  - büyük dosyada kare kare alma ve sınırlı bellek (CBS-42).

## Bağlam

GeoPackage açık GIS'in en yaygın alışveriş dosyası; KentOS onu ne okuyabiliyor ne yazabiliyordu. Plan, kaynaktaki `fid`
ile KentOS'un kalıcı kimliğinin eşlenmesini ister. Ayrıca Türkçe ad ve değerin, NULL ile boş metnin, deliğin, çok
parçanın ve Z/M'nin kaybolmamasını, QGIS 4.2'nin dosyasının açılıp KentOS'un dosyasının QGIS'te açılmasını, yazmanın
yarıda kesilince yarım şema bırakmamasını ister.

## Karar

### 1. `kentos-geopackage` crate'i

`crates/native/geopackage` yalnız GeoPackage'ı bilir: KentOS türü yok, bağımlılığı yalnız `rusqlite` (`bundled`,
`functions`). Bağımlılık yönü denetiminde kendi grubu (`geopackage`) var; ortak çekirdeğe girmez.

- **Geometri** (`geom.rs`): `GP` başlığı (sürüm, bayraklar, srs, kutu) ve ISO WKB.
  - Z ve M (+1000, +2000, +3000; EWKB'nin üst bitleri de okunur), boş geometri (başlığın 4. biti; noktanın NaN'ı).
  - Eğri uzantısının türleri: dairesel eğri, birleşik eğri, eğri alan, çok eğri, çok yüzey.
  - Okuma hiçbir girdide panik yapmaz: kısa ya da 32 düzeyden derin blob hatadır; sayılar kalan baytlara sığmalıdır.
- **Dosya** (`lib.rs`):
  - **Açma ve keşif:** `application_id` GPKG ya da GP10 ya da `gpkg_contents` tablosu; tablolar sırasıyla, sütunlar
    bildirilen türleriyle (`MEDIUMINT`, `TEXT(40)` olduğu gibi geri yazılır), `gpkg_geometry_columns`, sistemler,
    uzantılar.
  - **Okuma:** satırlar fid sırasıyla; kutuyla (R-tree dizini varsa), `after` ve `limit` ile sayfa sayfa.
  - **Yazma:** tablo satırlarıyla tek işlemde. GeoPackage 1.3 (`user_version` 10300):
    - `gpkg_rtree_index` ve 1.3'ün altı tetikleyicisi; tetikleyicilerin çağırdığı `ST_MinX`, `ST_MaxX`, `ST_MinY`,
      `ST_MaxY` ve `ST_IsEmpty` bağlantıya kaydedilir;
    - eğri varsa `gpkg_geom_<TÜR>` uzantısı;
    - `contents`'in kutusu.
- **`kentos_uids` uzantısı:** KentOS'un satır başına kalıcı kimliği (`table_name`, `fid`, `uid`), `gpkg_extensions`'ta
  `read-write`. Öbür programlar dokunmaz.

### 2. Satırlar nesne, nesneler satır (`kentos_native_application::geopackage`)

- **Geometri:**
  - Nokta ve çok nokta nokta nesnesidir (parçalar).
  - Çizgi, dairesel ve birleşik eğri çoklu çizgidir. Üç noktalı her yay kenarın `bulge`'udur, tan(θ/4); açı yayın üç
    noktasının çemberinden bulunur. Çok çizgi ve çok eğri parçalı çoklu çizgidir.
  - Alan ve eğri alan delikleriyle alandır; kapanan köşe düşer. Çok alan ve çok yüzey parçalı alandır.
  - Z köşelerin kotu, M çoklu çizginin ölçüsüdür. Alanın M'si alınmaz ve söylenir: alan rota değildir.
  - Geometrisiz ya da boş satır geometrisiz kayıttır (ADR 0410). Koleksiyon alınmaz ve söylenir.
- **Geri yazma:**
  - Yay, kirişinin ortasından sagitta kadar kayan orta noktasıyla dairesel eğri olur; düz kenarlar çizgi olarak kalır.
  - Daire iki yaylı eğri alan, yay dairesel eğri olur.
  - Elips, eğri ve NURBS 0,1 mm'lik kirişlerle yazılır ve sayılır.
  - Yazı, ölçü, blok, tarama, tablo, resim, raster ve nokta bulutu yazılmaz; türlerine göre sayılır.
  - Sütunun türü: tek parçalılar ve çok parçalılar karışıksa hepsi çok parçalı olur; eğri varsa eğri türü seçilir;
    nokta, çizgi ve alan karışıksa `GEOMETRY`.
- **Değerler:**
  - NULL özniteliğin yokluğu, boş metin boş metindir.
  - Evet/hayır `true`/`false`, gerçel sayı en kısa geri okunan metnidir.
  - 2⁵³ − 1'i aşan değeri olan tam sayı sütunu metin alanı olur, her basamağı korunur ve söylenir.
  - `DATETIME`'ın UTC saati yerel tarih-saat alanına diliminsiz ve kesirsiz girer; dilim ya da kesir varsa söylenir.
    Geri yazılırken `Z` ile UTC'dir.
  - İkili sütun alınmaz ve söylenir.
  - Geri yazmada katmanın alanları türleriyle sütun olur (metin uzunluğuyla), öbür öznitelikler metin sütunu olur. Büyük
    küçük harf ayrı ama aynı adlı ikinci sütun `_2` alır ve söylenir (SQLite adları harf ayırmaz).
- **Kalıcı kimlik:** önce dosyanın `kentos_uids`'i; yoksa ADR'nin ad alanında `UUIDv5("gpkg:<tablo>:<fid>")`. Aynı tablonun
  aynı satırı her seferinde aynı kimliği alır; aynı dosya ikinci kez alınamaz (`entity_exists`).

### 3. Komutlar

- **`file.import.gpkg`** (belgeyi değiştirir; tek geri alma adımı “GeoPackage: ad.gpkg”):
  - `tables` alınacak tabloları seçer, `layers` bir tabloyu var olan bir katmana yönlendirir.
  - Öbür tablolar aynı adlı katmana (büyük küçük harf ve Türkçe işaretler bir) gider, yoksa tablonun sütunlarını alan
    olarak taşıyan yeni katmana; yeni katmanlar dosyanın adındaki gruba girer.
  - Dosyanın EPSG sistemi projeninkinden başkaysa `reproject` istenmedikçe `crs_mismatch`. İstenirse projenin datum
    seçimleriyle çevrilir; yaylı nesne kesin çevrilemez, alınmaz ve söylenir. Sistem bildirmeyen dosya olduğu gibi
    alınır (`crs_undeclared`).
  - `plan` ve `validate` belgenin bir kopyasında çalışır, belgeyi değiştirmez.
  - Desteğini `import_apply`'ın kimlikleri koruyan girişinden alır (`apply_import_with_uids`, belgenin
    `add_many_with_uids`'i).
- **`file.export.gpkg`** (başsız sunucunun dosya komutları arasında, ADR 0253 §10):
  - Dosya geçici bir yerde kurulur, baytları okunur; `execute` onları geçici adla yazar, geri okur ve tek adımda yerine
    koyar.
  - `plan` yazmadan söyler. Var olan dosyanın üzerine yalnız `overwrite` ile yazılır.
  - Yazılamayanlar ve yaklaşık yazılanlar `export_losses` ile söylenir.
- **Yeni kodlar:** `table_not_found`, `crs_mismatch`, `crs_undeclared`, `entity_exists`, `export_losses`.
- Python'da `kentos.cad.file.import_.gpkg` ve `kentos.cad.file.export.gpkg` katalogdan üretilir; MCP'de araçtır. Dışa aktarma, var olan
  dosyanın üzerine yazabildiği için yıkıcı listededir.

### 4. Masaüstü

CBS'nin Veri sekmesinde Dosya alışverişi paneline GeoPackage… ve GeoPackage dışa aktar… girdi; komut aramasında da
bulunurlar. İkon: dosya sayfasında veritabanı silindiri. Üç seçenek önizlendi, A seçildi; GeoPackage bir SQLite
veritabanıdır.

- **İçe aktarma penceresi:** dosyanın tablolarını türleri ve satır sayılarıyla onay kutusu olarak, koordinat sistemini
  de projeninkine karşı gösterir.
- **Başka sistemdeki dosya:** "Projenin sistemine çevir" işaretlenmedikçe İçe aktar kapalıdır; sessiz dönüşüm yoktur.
- **Alma ve yazma:** alma komutla yazar, yeni nesnelere yakınlaştırır, sonucu bildirimle ve iletiyle söyler. Dışa
  aktarma, kaydetme penceresinden sonra komutla yazar.

### 5. Bağımsız doğrulama

- `scripts/fixtures/geopackage_cases.py`:
  - `qgis.gpkg` QGIS 4.2.3'ün `QgsVectorFileWriter`'ıyla yazılmış gerçek bir dosyadır. İçinde Türkçe ad ve değer, NULL
    ile boş metin, 2⁵³'ü aşan kimlik, tarih, UTC zaman, evet/hayır, iki parça ve delik, geometrisiz satır, ZM yol, Z
    nokta, eğri alan, birleşik eğri ve öznitelik tablosu var.
  - `expected.json` GDAL'ın OGR'ının onu okuyuşudur. Rust okuyucusu aynısını okur.
  - Crate'in yazdığı `kentos.gpkg` OGR'da aynı okunur.
  - QGIS'in dosyası çizime alınıp `file.export.gpkg` ile yazılan `kentos-export.gpkg` aynı anlamdadır: fid, değerler,
    geometri; yayın orta noktası son bitlerine kadar. QGIS 4.2 onu açar ve satır sayıları tutar.
- **GDAL doğrulayıcısı:** `validate_gpkg.py` iki dosyada da QGIS'in kendi dosyasındakinden fazla bulgu vermez. GDAL
  3.12'nin doğrulayıcısı boşluk bayrağını standarttaki 4. bit yerine 3. bitten okur; GDAL'ın yazıcısı 4. biti yazar.
  Bu yüzden her boş geometri, QGIS'inki de, 152 numaralı bulgudur.
- **İçe aktarma durumları:** `scripts/fixtures/gpkg_import_cases.py`, `file.import.gpkg.json`, 23 durum.
  - Nesneler OGR'ın okuduğundan, ADR'nin kuralıyla Python'da kurulur: yayın `bulge`'u üç noktasının çemberinden, halkanın
    kapanışı, değerlerin metinleri.
  - Durumlar şunları da sınar: kimliğin sonraki bir `cad.entities.relate` adımıyla bulunması, `kentos_uids`'in
    korunması, WGS 84 dosyasının reddi ve çevrilmesi, sistemsiz ve boş dosya, retler.
- **Rust testleri:**
  - crate'in gidiş-dönüşü, R-tree ile kutu sorgusu ve sayfa;
  - başsız sunucuda içe al, dışa yaz, yeniden içe al: aynı kimlikler ve değerler, yaylar 10⁻¹² içinde;
  - masaüstünde pencere, çevirme onayı ve dışa aktarma.
- **Resimler:** `gpkg-*`.

## Sonuçlar

- KentOS gerçek bir QGIS GeoPackage'ını kimlik, değer, eğri ve Z/M kaybı olmadan alır ve QGIS'in açtığı GeoPackage yazar.
  Taşınamayan her şey söylenir.
- Bağlı kaynak olarak yerinde düzenleme bu crate'in üstüne kurulacak: okuma kutuyla ve sayfa sayfa, yazma işlemle hazır.
