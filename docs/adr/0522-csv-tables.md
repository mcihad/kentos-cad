# ADR 0522: CSV tablolar — WKT ya da X ve Y, `.csvt` ve `.prj` ile iki yönde

- **Durum:** kabul edildi (2026-10-19). `plan/gis.md` CBS-05'in ikinci kalan dilimi (“GeoPackage, GeoJSON ve CSV/XYZ çift
  yönlü”: CSV'nin iki yönü); CBS-06'nın CSV yolu (büyük tam sayı, tarih, NULL), CBS-44'ün QGIS ve GDAL ile alışverişi.
- **Bağlam belgesi:** ADR 0502 (GeoPackage içe aktarma: katmanlar, alanlar, kalıcı kimlik, sistem denetimi, kayıplar),
  0514 (Shapefile dışa aktarma, ESRI `.prj`), 0223 (tablo dosyası okuyucusu), 0222 (açık dönüşüm), 0253 §10 (dosya
  komutları).
- **Web:** ertelendi (ADR 0228). Web'de gerekenler TODOS.md §0.2'de.
- **Sonraya kalan:**
  - XYZ (başlıksız, boşlukla ayrılmış nokta bulutu metni) burada değil: nokta bulutunun metin okuyucusu onu zaten alır (ADR
    0207); çizim nesnesi olarak XYZ yazımı ihtiyaç olursa;
  - GML, FlatGeobuf, GeoParquet (CBS-05'in sonraki dilimleri);
  - içe almada hazırlık alanı, reddedilen satırların ayrı dosyası, doğal anahtarla güncelleme (upsert);
  - dışa aktarmada katman başına ayrı CSV (bugün tek CSV, `katman` sütunuyla).

## Bağlam

CSV, kurumlar arasında en çok dolaşan tablodur: Excel'den gelen koordinat listeleri, QGIS'in “ayrılmış metin” katmanı, GDAL'ın
CSV sürücüsü. KentOS tablo dosyalarını Tablodan nokta ve İşlemler'in dosya parametresiyle okuyabiliyordu (ADR 0223), ama
nesneyi bütün öznitelikleri, türleri ve sistemiyle iki yönde taşıyan bir yol yoktu. Plan iki yönü ister ve büyük sayının,
tarihin, eğrinin ve Z/M'nin sessizce kaybolmamasını.

## Karar

### 1. Biçim: GDAL'ın CSV'si

KentOS kendi CSV lehçesini icat etmez; GDAL'ın CSV sürücüsünün yazdığını yazar ve okur, QGIS de aynısını okur:

- **Geometri** ya ilk sütunda ISO WKT (`WKT`; GDAL'ın `GEOMETRY=AS_WKT`'si), ya `X`, `Y` ve kotluysa `Z` sütunları
  (`GEOMETRY=AS_XY`), ya da hiç.
- **Satırlar** UTF-8, CRLF, RFC 4180 tırnaklaması; ayırıcı virgül, noktalı virgül ya da sekme. Sayılarda ondalık ayırıcı her
  zaman noktadır (Türkçe Excel noktalı virgülü doğrudan açar ama virgüllü ondalığı yazmayız: dosya makineler arasında
  dolaşır).
- **Türler** yanındaki `.csvt`'de, tek satır, GDAL'ın adlarıyla: `WKT`, `String` ya da `String(n)`, `Integer64`, `Real`,
  `Date`, `DateTime`, `Time`, `Integer(Boolean)`.
- **Sistem** yanındaki `.prj`'de. Shapefile'ın ESRI biçimli `.prj`'si (ADR 0514) OGR'ın CSV sürücüsünde tanınmıyor; CSV'nin
  `.prj`'si GDAL'ın kendi yazdığı OGC WKT1'dir (`AUTHORITY["EPSG",…]`'li). Tablo `esri_prj.py`'nin `wkt1`'i, PROJ'dan.

### 2. Dışa aktarma: `file.export.csv`

- Katmanlar (boşsa nesnesi olan her katman) tek CSV olur; birden çok katmanda ikinci sütun `katman` her satırın katmanının
  adıdır.
- Sütunlar: geometri, (gerekirse) katman, sonra katmanların alanları sırasıyla ve türleriyle, sonra hiçbir alanın adlandırmadığı
  öznitelikler adlarına göre, metin olarak. İki katmanda aynı adlı ama farklı türlü alan metin sütunu olur ve söylenir.
- **WKT** her nesne türünü taşır: yay ve yaylı kenar `CIRCULARSTRING` ve `COMPOUNDCURVE` olarak kesin, delikler ve parçalar
  `CURVEPOLYGON`, `MULTISURFACE`, `MULTICURVE`, Z ve M ISO WKT'nin `Z`, `M`, `ZM`'siyle. Elips, eğri ve NURBS'ün WKT'de
  karşılığı yok: en çok 0,1 mm sapan kirişlerle yazılır, sayılarak söylenir.
- **X ve Y** yalnız noktaları yazar; öbür nesneler sayılarak dışarıda kalır.
- Yazı, ölçü, tarama, tablo, resim, raster ve nokta bulutu, yardımcı çizgi ve ışın CSV'de karşılığı olmadığından yazılmaz;
  türlerine göre `left`'te sayılır ve uyarıda söylenir (`export_losses`).
- Bütün ya da hiç: üç dosyanın baytları önce yapılır (`plan` burada durur), sonra her biri geçici adla yazılıp geri okunur
  ve yerine konur (`files::write_bytes`). Var olan dosyanın (`.csvt`, `.prj` dahil) üzerine yalnız `overwrite` ile.

### 3. İçe aktarma: `file.import.csv`

- **Okuma** tablo dosyası okuyucusuyla (ADR 0223, İşlemler'in sınırları: 250 000 satır, 500 sütun; sınırı aşan söylenir).
  İlk satır başlıktır; boş adlar `alan_n`, yinelenenler `_` ekiyle.
- **Geometri** `geometry` ile verilir ya da (yoksa) adlarından bulunur: önce WKT sütunu (`wkt`, `geom`, `geometry`,
  `the_geom`, `wkt_geom`), sonra X ve Y (`x`/`y`, `lon`/`lat`, `longitude`/`latitude`, `easting`/`northing`, `doğu`/`kuzey`)
  ve `z` ya da `kot`, yoksa geometrisiz. Büyük küçük harf ayrılmaz. Verilen sütun yoksa ret (`import_unreadable`, sütunlar
  sayılarak).
- **WKT** okuyucusu GeoPackage crate'indedir (`kentos_geopackage::wkt`): ISO WKT'nin bütün türleri, eğriler, Z ve M,
  EWKT'nin `SRID=…;` öneki; iç içe derinlik 32'yle sınırlı. Okunamayan satır kayıt olarak gelir ve satır numarasıyla söylenir.
- **Türler** `.csvt`'den (`String(n)` uzunluğu alanın uzunluğu olur), yoksa değerlerden: hepsi tam sayıysa tam sayı (2⁵³'ü
  aşan varsa metin, GeoPackage'daki gibi söylenerek), hepsi sayıysa ondalık, hepsi tarihse tarih, yoksa metin. GDAL'ın
  `2024/02/29` ve `2024/03/01 10:20:30+00` biçimleri ISO'ya çevrilir.
- **Sistem** `.prj`'den (ESRI ya da OGC WKT1; EPSG adlandırılmışsa), yoksa `srid`'den, yoksa bilinmez (`crs_undeclared`).
  Projeninkinden başkaysa `reproject` olmadan ret (`crs_mismatch`); dönüşüm açık onaylıdır (ADR 0222).
- **Yazma** GeoPackage içe aktarmasının yolundan (`file_import_gpkg::run_read`): CSV tek tablodur; satırlar `layer`'a, yoksa
  dosyanın adlı katmanına, yoksa sütunları alanları olan yeni katmana. Her satırın kalıcı kimliği dosyanın adı ve satır
  numarasından türer (`csv/<ad>`); aynı dosyayı ikinci kez almak `entity_exists`'tir. Tek geri alma adımı “CSV: ad.csv”.

### 4. Masaüstü

CBS'de Veri › Dosya alışverişi'nde **CSV içe aktar…** ve **CSV dışa aktar…**:

- İçe aktarma penceresi dosyanın sütun ve satır sayısını, bulunan geometriyi (WKT sütunu, X ve Y sütunları, Geometrisiz) ve
  sütun seçicilerini, “Başka sistemdeyse projenin sistemine çevir” onayını ve komutun planını (kaç nesne ve kayıt, uyarılar ya
  da ret) seçim değiştikçe gösterir; plan çizimin kopyasında çalışır.
- Dışa aktarma penceresinde Geometri (WKT, X ve Y, Geometrisiz) ve Ayırıcı; yazılmadan önce satırlar, sütunlar ve türleri,
  yazılmayanlar. Dışa aktar… yolu sorar ve komutla yazar.

### 5. Python ve MCP

`kentos.cad.file.export.csv` ve `kentos.cad.file.import_.csv` katalogdan; ikisi de MCP'nin yıkıcı listesinde (biri dosya
yazar, öbürü çizimi değiştirir).

## Doğrulama

- `scripts/fixtures/csv_exchange_cases.py --check`: GDAL'ın CSV sürücüsünün yazdığı on dosya (`fixtures/csv/v1`: WKT'li
  parseller delikli ve çok parçalı, kotlu yol, eğriler, geometrisiz malik tablosu, `AS_XYZ` noktalar, sistemsiz, WGS 84, boş,
  elle yazılmış, bozuk) OGR'la okunur ve ADR'nin kuralıyla 21 durumun beklentisi kurulur
  (`fixtures/commands/v1/file.import.csv.json`); KentOS'un yazdığı CSV'ler (`fixtures/csv/v1/out`) OGR'la geri okunur ve
  kaynağın geometrileriyle karşılaştırılır.
- `cargo test -p kentos-headless --test headless csv_tables`: dışa aktarma ve geri alma (`KENTOS_WRITE_CSV=1` çıktıları
  yazar).
- `python/tests/test_csv.py`: stdlib `csv` ile okunan dosya ve gidiş-dönüş.
- Masaüstü: `exchange::csv_tests` (pencere, tek adım, geri alma, sistem onayı, dışa aktarma), resimler
  `exchange::csv_tests::screens` (`.run/shots/csv-*`).
