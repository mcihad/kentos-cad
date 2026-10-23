# ADR 0592: İşlemler'de girdiyi kaynağından okumak ve sonucu veritabanına yazmak

- **Durum:** kabul edildi (2026-10-22). `plan/gis.md` CBS-23: mevcut geoprocessing'e provider girdisi ve parça parça çıktı.
- **Bağlam belgesi:**
  - ADR 0508 §4: `cad.processing.run`'ın `sink`'i, sonucun yeni bir GeoPackage'a yazılması;
  - ADR 0549: var olan GeoPackage'a tablo ekleme ve değiştirme, 5000 satırlık gruplar, iptal;
  - ADR 0511: sonucun soyu (`kentos.lineage` 1);
  - ADR 0581, 0591: `cad.sources.query`, kaynağa giden süzgeç ve kutu, parça parça okuma;
  - ADR 0505, 0510, 0582: PostGIS bağlı katmanı, bağlı katmanın alanla parçası, satırların kalıcı kimlikleri (`kentos_uids`);
  - ADR 0559: İşlemler araçları sunucu işi olarak.
- **Web:** ertelendi (ADR 0228), TODOS.md §0.2.
- **`.kcad` şeması:** değişmez. Kaynaktan okuma ve sonucun yeri çalıştırmanın girdisidir.
- **Sonraya kalan:**
  - araçların akışlı hesabı (bir aracın girdiyi parça parça işlemesi): bugün girdi çalıştırmanın kopyasına bütünüyle okunur, en
    çok bir milyon kayıt;
  - disk üstünde ara depo (bir milyondan büyük girdiler);
  - modellerde kaynaktan okuma ve veritabanına yazma (CBS-24);
  - PostGIS'te var olan tabloya satır ekleme (bugün yalnız yeni tablo ya da tablonun yenisiyle değiştirilmesi);
  - masaüstü penceresinde yazmanın ilerlemesi (bugün “yazılıyor” ve Durdur).

## Bağlam

ADR 0508'den beri `cad.processing.run` sonucunu çizim yerine bir GeoPackage'a yazabiliyor; ADR 0549 var olan dosyaya tablo
eklemeyi ve iptali getirdi. İki eksik kalmıştı:

1. **Girdi çizimdeki nesnelerdir.** Alanla bağlanmış bir katmanda (ADR 0510) çizim tablonun yalnız bir parçasını tutar. Araç o
   parçayla çalışır ve sonuç bütün tablonunmuş gibi görünür. Plan bunu açıkça yasaklar: “görünür alt kümede çalışıp tüm dataset
   işlendi deme”. Bağlı olmayan bir dosyanın ya da veritabanının tablosunu araca vermenin de yolu yoktu: önce içe aktarmak
   gerekiyordu.
2. **Sonuç yalnız GeoPackage'a gider.** Veritabanında çalışan bir kurum sonucu önce dosyaya, sonra elle veritabanına taşıyordu.
   Ayrıca masaüstü penceresinde `sink` seçeneği hiç yoktu: dosyaya yazmak yalnız Python, MCP ve sunucu işinden yapılabiliyordu.

## Karar

### 1. Girdiyi kaynağından okumak

Nesne girdisi (`features` parametresi) yeni bir kapsam alır: `scope: "source"`.

- `{ "scope": "source", "layer": "<bağlı katman>" }`: bağlı katmanın bütün tablosu. Çizimin tuttuğu parça kopyadan çıkarılır,
  tablonun kayıtları aynı katmana girer.
- `{ "scope": "source", "path": "<dosya ya da adres>", "table": "<tablo>" }`: bir GeoPackage'ın ya da PostGIS veritabanının
  tablosu. Kayıtlar `kaynak-<parametre>` kimlikli, tablonun adını taşıyan yeni bir katmana girer; tablonun sütunları katmanın
  alanlarıdır.
- İkisi de isteğe bağlı `filter` (İfadeyle seç'in dili) ve `bbox` alır.
- Kayıtlar `cad.sources.query`'nin okuduğu gibi okunur (`sources_query::stage`):
  - süzgecin kaynağın denetleyebildiği kısmı kaynağa gider (ADR 0581 §3);
  - parça parça (1000'erlik) anahtar sırasıyla gelir;
  - her kayıt burada ifadeyle yeniden sınanır;
  - kayıt projenin sistemine çevrilir, kalıcı kimliğini (`kentos_uids` ya da anahtardan türetilen) taşır.
- Okuma çalıştırmanın kopyasınadır. Çizim değişmez.
- Kaynaktan okumanın sonucu yalnız bir `sink`'e yazılır. `sink` yoksa `invalid_parameters` (`parameters.<ad>`): bir milyonluk
  tablo çizime nesne olarak yazılmaz.
- Okunan kayıt sınırı `MOST_STAGED` (1 000 000). Bir fazlası `input_too_large`, kopyaya hiçbir şey girmeden.
- Cevabın `inputs`'u, kaynaktan okunan girdi için şunları söyler:
  - `table`: `dosya › tablo` ya da `PostGIS: sunucu/veritabanı › şema.tablo`;
  - `rows`: okunan kayıt sayısı;
  - `revision`: GeoPackage dosyasının SHA-256'sı (veritabanında yok);
  - `source`: bağlı katmanın kaynağı.
  Soy belgesi (ADR 0511) bunları da taşır.
- Retlerin yeri girdinin parametresidir:
  - `layer_not_found`, `invalid_input`: bağsız katman;
  - `invalid_parameters`: ikisi birden ya da tablosuz yol;
  - `invalid_extent`: `.bbox`;
  - `table_not_found`: `.table`;
  - `import_unreadable`: `.path`;
  - `invalid_expression`: `.filter`;
  - `source_unreachable`, `invalid_address`.
- **Parçanın söylenmesi.** Alanla bağlı bir katman `scope: "layer"` ile verilip sonuç bir `sink`'e gidiyorsa uyarı
  `partial_scope` (`parameters.<ad>`) iner: “katman tablonun yalnız bir parçası; araç o parçayla çalıştı”. Bütün tablo için
  girdinin nasıl verileceği uyarıda söylenir. `sink`'siz çalıştırma da aynı uyarıyı alır.
- Boş okuma boş katmandır: aracın kendi reddi gelir (“bu katmanda uygun nesne yok”).

### 2. Sonucu PostGIS'e yazmak

`sink.path` bir veritabanı adresiyse (`postgresql://`, `postgres://`) sonuç oraya yazılır. Yeni alan `sink.schema` tabloların
şemasıdır (varsayılan `public`). Şema GeoPackage'a verilirse `invalid_input`; nokta ya da tırnak taşıyorsa `invalid_input`.
Parola taşıyan adres `invalid_address`.

- Tablolar önce geçici bir GeoPackage'a, dosya `sink`'inin yazıcısıyla yazılır. Böylece tablonun biçimi, sütun eşlemesi,
  Z ve M, yayların kirişleri ve kayıplar (`export_losses`) iki hedefte aynıdır.
- Sonra tablolar geri okunur ve sağlayıcının yeni işlemiyle veritabanında açılır: `TableProvider::create_tables`.
  `kentos-postgis`'in `create.rs`'i bunu tek işlemde yapar:
  - olmayan şemayı açar;
  - tabloyu açar. Anahtarı `fid bigint PRIMARY KEY`. Geometri sütunu türü, Z ve M'si ve sistemiyle tiplidir
    (`geometry(PointZ, 5256)`); satırları karışık boyutlu sütun tipsiz `geometry`'dir. Sütunlar bağlı katmanın türleriyledir;
  - satırları 5000'erlik gruplarla yazar, her grup öncesi iptal sorulur;
  - uzamsal dizin kurar;
  - satırların kalıcı kimliklerini şemanın `kentos_uids`'ine yazar;
  - tablonun yorumu soy belgesidir.
  İşlem kesinleşene dek hiçbir şey görünmez. Ret, hata ya da iptal hepsini geri alır.
- PostgreSQL 63 bayttan uzun adı sessizce keser. Böyle bir tablo ya da sütun adı reddedilir.
- **Aynı adlı tablo:**
  - `mode: new` (varsayılan) ve `addTables`: reddeder (`table_exists`, veritabanına dokunulmadı);
  - `replaceTables`: tabloyu aynı işlemde kaldırıp yeniden açar.
  - `overwrite`, yeni tablolar için `replaceTables` gibidir. `replaceTables` ile birlikte verilirse `invalid_input`.
- **Plan** geçici dosyayı yazar ve veritabanına yalnız hangi adların olduğunu sorar. Reddedilecekse reddeder;
  `replaced`'i söyler. Veritabanına yazmaz.
- **Cevap:**
  - `sink.path` adrestir; `bytes` 0, `sha256` boştur (veritabanının dosyası yok);
  - tablolar `şema.tablo` adıyla;
  - `sinkReport.ids` `kentos_uids`'tir.
- PostGIS sağlayıcısı kayıtlı olmayan ortamda `source_unreachable`.
  Ulaşılamayan veritabanı da `source_unreachable`. Ötekiler `file_unwritable`.

### 3. Masaüstü penceresi

İşlemler penceresinin sağ sütununda, Nerede çalışır'ın altında **Sonuç** (sunucu seçiliyken ADR 0559'un Sonuç'u):

- **Çizime**: bugünkü gibi.
- **GeoPackage dosyasına**: yol ve Seç…, Tablolar (Yeni, Tablo ekle, Tabloyu değiştir), Varsa üzerine yaz.
- **PostGIS veritabanına**: adres (çizimin PostGIS katmanından önerilir), Şema, Tablolar. Parolanın adrese yazılmadığı söylenir.

Girdisi alanla ya da bütünüyle bağlı bir katman olan her nesne parametresi için bir anahtar gelir:
**“<Parametre>: bütün tablo (kaynaktan oku)”**. Altında katman ve tablo söylenir; parça varsa bu da. Sonuç çizime giderken
anahtar kapalıdır ve ipucu nedenini söyler.

Çalıştır, `cad.processing.run`'ı pencerenin değerleri, seçim, görünümün kutusu ve `sink` ile çalıştırır. Çalıştırma çizimin
okuma kopyasında, başka bir iş parçacığında yapılır (`run_to_sink`). Durdur iptal bayrağını kaldırır: dosya ya da veritabanı
olduğu gibi kalır. Satır söyler:

- yazılan tabloları satır sayılarıyla;
- kaynaktan okunanları;
- uyarıları (`partial_scope` dahil).

Çizim ve geri alma değişmez.

## Doğrulama

- **Ortak durumlar:** `fixtures/commands/v1/cad.processing.run.source.json`, 29 durum. Üretici
  `scripts/fixtures/processing_source_cases.py`; ADR 0581'in `sorgu.gpkg`'si (GDAL'ın yazdığı) sqlite3 ve hashlib'le, KentOS kodu
  olmadan okunur. Durumlar:
  - bütün tablo, süzgeçler, kutu, kutu ve süzgeç, nokta tablosu;
  - parçalı bağlı katman kaynağından ve `scope: layer` ile (`partial_scope`), bağlı katman süzgeçle;
  - dosyaya yazıp `cad.sources.describe` ile okuma;
  - on beş ret.
  Masaüstü koşucusu: `KENTOS_CASES_ONLY=cad.processing.run.source cargo test -p kentos-native-application --test all
  every_command_case`.
- **Noktaların yeri:** `processing_sink.rs`'in `an_input_read_from_its_source_goes_to_a_file`'ı dosyayı yazar. Betiğin
  `--written`'ı onu okur; noktalar karelerin EPSG:5254'e (çizimin sistemi; kaynak 5256) PROJ'la (GDAL'ın osr'si) taşınan
  köşelerinin ağırlık merkezleriyle 1,9·10⁻⁹ m içinde, öznitelikleriyle.
- **Kayıt sınırı:** `sources_query.rs`'in `a_source_input_stops_at_its_cap`'ı. Sınır 249 iken 250 kayıt `input_too_large`
  (`parameters.input.table`); süzgeç tablonun değil süzgecin kayıtlarını sayar.
- **PostGIS:** `kentos-postgis`'in `processing_results_go_to_postgis`'i, `postgis_cases.py`'nin geçici docker veritabanında:
  - yeni şemaya yazma, aynı adın reddi, planın reddi ve `replaced`'i, `replaceTables`;
  - veritabanındaki tablonun kaynak girdisi olarak süzgeçle okunması.
  Betik psycopg2'yle şunları denetler:
  - geometri sütunu `geometry(Point,5256)`;
  - 12 nokta karelerin merkezlerinde;
  - `kentos_uids`'te 12 kimlik;
  - tablonun yorumunda `kentos.lineage` ve okunan 12 kayıt.
- **Masaüstü:** `processing::sink_tests`:
  - bağlı girdi dosyaya bütün tablo olarak yazılır, çizim değişmez, parça seçilince 4 nokta ve `partial_scope`;
  - adressiz veritabanı ve parolalı adresin reddi.
  Resimler `.run/shots/islem-sonuc-*` (çizim, dosya, veritabanı, yazıldı; iki tema, 1440 × 900 ve 1100 × 650).
