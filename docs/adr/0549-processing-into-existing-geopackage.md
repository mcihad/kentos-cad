# ADR 0549: İşlem sonucunu var olan GeoPackage'a, parça parça ve ya hep ya hiç yazmak

- **Durum:** kabul edildi (2026-10-20). `plan/gis.md` CBS-23'ün kalanı. Kapsamı koordinatörün sırasıyla ben belirledim.
- **Bağlam belgesi:**
  - ADR 0508 §4 (`cad.processing.run`'ın `sink`'i, yeni GeoPackage);
  - 0502 (GeoPackage), 0503 (bağlı katmanlar);
  - 0511 (sonucun soyu), 0512 (geniş değerler);
  - 0530 (CAD'den GIS türevi: `export_losses`, yaylar, Z/M);
  - 0542 (dayanıklı işler, iptal).
- **Web:** ertelendi (ADR 0228). Web'de gerekenler TODOS.md §0.2'de.
- **Sonraya kalan:**
  - girdinin bağlı katmanın kaynağından sayfa sayfa okunması (§6);
  - aracın kendi hesabının iş parçacıklarına ve parçalara bölünmesi: araç sonucunu hâlâ bellekte kurar, yalnız dosyaya
    yazma kısmı sınırlı bellekle gider (§3);
  - masaüstünde İşlemler penceresinde “Sonucu GeoPackage'a yaz” seçeneği (pencerede sink arayüzü yok; komut Python,
    MCP, başsız sunucu ve ajan bağlantısından kullanılır);
  - PostGIS'e yazma.

## Bağlam

ADR 0508 §4, `cad.processing.run`'a `sink: { path, overwrite }` verdi: araç çizimin kopyasında çalışır, ürettiği nesneler
yeni bir GeoPackage'a (katman başına tablo) yazılır, çizim değişmez. Dosya yoksa yazılır, varsa yalnız `overwrite` ile
bütünüyle yenilenir. İki şey eksikti:

1. **Var olan dosyaya yazılamıyordu.** Belediyede asıl iş bir kurumun GeoPackage'ına (QGIS'in ya da başka bir yazılımın
   yazdığı, içinde başka tabloların olduğu dosya) bir analizin sonucunu eklemektir. `overwrite` dosyanın bütün öbür
   tablolarını silerdi; başka yol yoktu.
2. **Yazma tek parçaydı.** Bütün tablolar bellekte kurulup bir kerede yazılıyordu; yarıda kesilen (iptal, hata) bir işin ne
   bıraktığı yalnız “yeni dosyada geçici ad, sonra yeniden adlandırma” ile güvence altındaydı. Var olan dosya için bu yol
   (kopya + yeniden adlandırma) büyük dosyada bütün dosyayı kopyalamak demektir.

## Karar

### 1. Üç kip

`sink.mode`:

| mode | anlam |
|---|---|
| `new` (varsayılan) | yeni dosya; var olan yalnız `overwrite` ile yenilenir (ADR 0508 §4'teki gibi) |
| `addTables` | dosya yerinde olmalı (`file_not_found`); tablolar eklenir; aynı adlı tablo `table_exists` ile reddedilir |
| `replaceTables` | `addTables`, ama aynı adlı özellik ya da öznitelik tablosu yenisiyle değişir |

Kuralları:
- Var olan dosyanın hiçbir öbür tablosuna dokunulmaz: satırları, kimlikleri (`kentos_uids`), dizinleri, meta verisi ve soyu
  olduğu gibi kalır. Yanıt bunları `sinkReport.untouched`'ta sayar.
- `addTables` ve `replaceTables` ile `overwrite` birlikte verilmez (`invalid_input`): ikisi zıt şeyi ister.
- Dosya GeoPackage değilse ya da okunamıyorsa `import_unreadable`; dosyaya dokunulmaz.
- Aynı ad bir özellik ya da öznitelik tablosu değilse (döşeme tablosu, kayıtsız SQLite tablosu, eklentinin tablosu,
  görünüm) `replaceTables` de onu değiştirmez: `table_exists`. `Taken::Other` bunu söyler.
- `replaceTables`'ta aynı adlı tablo yoksa yalnız eklenir (`replaced` yok); yer tutucu bir hata değildir.
- `table_exists` yeni bir hata kodudur (`codes.rs`); iletisi tablonun adını, çaresini (kip `replaceTables`, ya da başka ad) söyler.

### 2. Ya hep ya hiç: dosyanın içinde tek SQLite işlemi

Yeni dosya eskisi gibi geçici adla yazılıp yeniden adlandırılır (eski dosya yeni tamamlanana dek yerinde durur).
Var olan dosyada kopya yoktur: bütün yazma **tek SQLite işlemidir** (`GeoPackage::session()` → `Session`). SQLite'ta DDL de
işlemlidir; `CREATE TABLE`, rtree ve tetikler, `gpkg_contents`, `gpkg_geometry_columns`, `gpkg_extensions`, `kentos_uids`
ve soy belgesi aynı işlemdedir. `Session::commit`'ten önce dosyada hiçbir şey görünmez; oturum bırakılırsa ya da hata
olursa işlem geri alınır, dosya baytı baytına eskisidir. Büyük dosya kopyalanmaz.

`plan` (çizime ve dosyaya dokunmayan çalıştırma) aynı yolu dosyanın **geçici bir kopyasında** koşar ve kopyayı siler:
yanıtın `sink` ve `sinkReport`'u gerçek çalıştırmanınkiyle aynıdır, hata kodları da (`table_exists` dahil) aynıdır.

### 3. Parça parça yazma

Yazma iki geçişlidir ve satırları `BATCH_ROWS = 5000`'lik gruplarla yazar:
1. Planlama: her katmanın tablosu için aile (nokta, çizgi, yüzey…), Z/M'in kimde olduğu, sütunlar, satır ve atlanan ve
   kirişle yazılan sayıları;
2. Yazma: tablo oluşturulur, satırlar grup grup `INSERT` edilir (`prepare_cached`, grup başına bir `kentos_uids` yazımı),
   ardından sınır kutusu, rtree ve soy.

Bellekte aynı anda yalnız bir grubun satırları durur. Sınırlar: araç sonucunu hâlâ bellekte kurar (çizimin kopyasındaki
yeni nesneler); bu ADR yalnız dosyaya yazmayı sınırlar. Aracın kendi hesabını da parçalamak ayrı bir iştir (sonraya kalan).

### 4. İptal

İptal şu noktalarda sorulur: aracın hesabı bittikten sonra, her tablodan önce, her grubun yazımından önce ve işlem
kesinleşmeden önce. Yanıt `cancelled`'dır (`CommandResult::Cancelled`). Bırakılan işlem geri alınır: dosya baytı baytına
eskisidir, geçici dosya yoktur, çizimin revizyonu değişmez. Yeni dosya kipinde geçici dosya silinir, var olan dosya
(`overwrite` ile) yerinde kalır. Komut iş olarak (`start`, `job`, `cancel`) çalıştığında ilerleme yazılan satır / toplam
satırdır.

### 5. Görünür politika: `sinkReport`

Yanıt `sink`'in (dosya, bayt, SHA-256, tablolar; ADR 0508) yanında `sinkReport` verir:
- `mode`;
- `tables`: her tablo için `table`, `layer`, `rows`, `geometry`, `z` ve `m` (`none`, `all`, `some`), `replaced`,
  `columns` (her sütun ve yazıldığı öznitelik: eşleme; çakışan ya da `fid`, `geom` adı değişen sütun adı değişir);
- `untouched`: dosyanın dokunulmayan tabloları;
- `batches`, `batchRows`;
- `ids`: satırların kalıcı kimliklerinin tablosu (`kentos_uids`).

Kayıplar yanıtın `export_losses` uyarısında sayılarıyla söylenir: GeoPackage'da karşılığı olmayan nesneler (yazı, ölçü, blok,
tarama; yazılmaz), 0,1 mm'lik kirişle yazılan eğriler, adı değişen sütunlar. Z ve M kaybı değil, tablonun özelliğidir: her
tablonun `z` ve `m`'i satırların kimde olduğunu (`none`, `all`, `some`) söyler. Aracın yeni nesne yapmadığı ya da yaptıklarının
GeoPackage'da geometrisi olmadığı sonuç boş sayılır: `nothing_to_export` ile reddedilir, dosyaya dokunulmaz. Kalıcı kimlikler
(`kentos_uids`) ve soy belgesi (ADR 0511) her tablonun kendisindedir.

### 6. Girdiyi kaynaktan sayfa sayfa okumak: sonraya

Araçlar girdilerini çizimin belgesinden (`reading_copy`) alır; bağlı bir katmanın nesneleri çalışma kopyasındadır, kaynağın
sayfalarında değil. Aracı doğrudan kaynak sayfalarına bağlamak her aracın girdisini “akış” yapmayı, ortak sorgu çekirdeğinin
(ilişkiler, tamponlar, birleştirme) bütün katmanı bekleyen yapısını değiştirmeyi ister. Bu temiz bir dilim değil. Bugünkü
yol bağlı katmanı çalışma kopyası olarak (alan sınırlı ya da bütün) açmaktır (ADR 0503, 0510); sonuç yine buradaki gibi
sayfalı yazılır. Sayfa sayfa girdi araç aracı akışlı hesaba taşındıkça (ADR 0542'nin dayanıklı işleriyle birlikte)
yapılacak.

## Uygulama

- `crates/native/geopackage/src/lib.rs`: `GeoPackage::session()`, `Session` (`taken`, `registered_name`, `srs`, `add_srs`,
  `drop_table`, `start_table`, `append`, `finish_table`, `add_metadata`, `commit`, `rollback`), `TableWrite`, `Taken`; var
  olan 1.4 dosyasının sürümüne uyan rtree tetikleri (`update2`, `4`, `5`, `6`, `7`: QGIS'in dosyasına eklenen tablo onun
  doğrulayıcısından geçer; yeni dosya 1.3 tetikleriyle yazılır, `write_table` aynı üç adımdan (`start_in`, `append_in`,
  `finish_in`) oluşur).
- `crates/native/application/src/geopackage.rs`: `write_in` (iki geçiş, `BATCH_ROWS`, `Flow`), `Plan`, `plan`, `Clash`,
  `WriteStop`, `Families`, `conform`; `TableWritten` (`z`, `m`, `columns`, `replaced`), `Written` (`batches`).
- `crates/native/application/src/processing_host.rs`: `ToolHost::run` bir `BatchFlow` alır ve `RunEnd::{Ran, Cancelled}`
  döner; `JobFlow` işin ilerlemesine ve durdurmasına eşler.
- `crates/native/processing/src/command.rs`: `sink_target`, `write_sink`; yeni dosya geçici adla, var olan dosya tek
  işlemle.
- `crates/shared/commands/src/cad_processing.rs`: `SinkMode`, `RunSink.mode`, `SinkReport`, `SinkTable`, `SinkColumn`,
  `SinkDimension`, `ProcessingRan.sinkReport`; `codes.rs`: `table_exists`.
- Python `kentos.cad.processing` yeniden üretildi (`scripts/python/sdk.py`); MCP'de `cad.processing.run` zaten yıkıcı
  listesinde (`mode: replaceTables` var olan dosyanın tablosunu değiştirir).

## Doğrulama

- `cad.processing.run`'ın 77 ortak durumundan 28'i bu ADR'nin: üç araçla (Tampon, Gruplayarak birleştir, Kesişim) `addTables`,
  plan, aynı adlı tabloda `table_exists`, `replaceTables` (var olan ve olmayan ad), aynı dosyaya iki sonuç, yeni dosyaya sonra
  ekleme ve yedi ret (dosya yok, `overwrite` birlikte, GeoPackage olmayan dosya, `.gpkg` olmayan ad). Durumlar
  `scripts/fixtures/processing_command_cases.py`'nin üretici kurallarından; Rust'ta ve Python SDK'sında geçer.
- `cargo test -p kentos-native-application --test all processing_sink`: beş test. Ekleme sonrası QGIS dosyasının altı
  tablosu satır satır eskisi gibi; ikinci ekleme `table_exists` ve dosya baytı baytına aynı; `replaceTables` yalnız o
  tabloyu yeniler, yetim soy belgesi ya da kimlik kalmaz; iptal her soru noktasında ve her üç kipte (n = 1…12) dosya
  baytı baytına ve çizim revizyonu aynı, geçici dosya yok; 2 satırlık grupta grup sayısı ve ilerleme.
- Bağımsız denetim: `scripts/fixtures/processing_sink_cases.py --check` (OGR, sqlite3, GDAL'ın `validate_gpkg.py`'si; KentOS
  kodu yok): `fixtures/geopackage/v1/kentos-run-ekle.gpkg` QGIS'in `qgis.gpkg`'sinin altı tablosunu OGR'ın okuduğuyla aynı
  taşır, eklenen Tampon yolların 5 m tamponları (alan 5e-4 içinde), tek soy belgesi, kimlikler, bütünlük denetimi,
  doğrulayıcı QGIS'in kendi dosyasının bulgularından fazlasını bulmaz.
- Çalıştırılmadı: masaüstü penceresi ve resimleri (İşlemler penceresinde sink arayüzü yok), PostGIS, milyon satırlık süre
  ölçümü (bu ADR bellek sınırını koyar, süre bütçesi koymaz).
