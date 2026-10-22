# ADR 0593: Toplu çalıştırmada ve modellerde kaynaktan okuma ve sonucu dosyaya ya da veritabanına yazma, kayıtta yazılan yer

- **Durum:** kabul edildi (2026-10-22). `plan/gis.md` CBS-24'ün kalanı: model çalıştırıcıya sağlayıcıdan girdi, tipli sink, toplu
  çalıştırmada aynı çalıştırıcı ve kalıcı çalışma kaydı.
- **Bağlam belgesi:**
  - ADR 0529: `cad.processing.batch`, modellerin komutla çalışması, çalışma kaydı (`kentos.run-record` 1);
  - ADR 0551: `onError`, satır içinde durdurma, tablo çıktısı, kaydın `.kcad`'e yazılmaması ve kalıcılığın yerleri;
  - ADR 0592: `cad.processing.run`'da girdiyi kaynağından okumak (`scope: source`) ve sonucu PostGIS'e yazmak;
  - ADR 0508 §4, 0549: `sink`, var olan GeoPackage'a tablo ekleme ve değiştirme;
  - ADR 0253 §10: komut kaydının dosya etkileri (`effects`).
- **Web:** ertelendi (ADR 0228), TODOS.md §0.2.
- **`.kcad` şeması:** değişmez. Kayıt bilerek proje dosyasına yazılmaz (ADR 0551 §6).
- **Sonraya kalan:**
  - araçların akışlı hesabı ve disk üstünde ara depo (ADR 0592'den; bir milyondan büyük girdi);
  - PostGIS'te var olan tabloya satır ekleme;
  - bağlı veritabanı tablosunun sürümünün kayda yazılması (bugün “bilinmiyor” denir, §3);
  - katman özetinin artımlı hesabı (ADR 0551 §5'in sonu);
  - masaüstünde Toplu çalıştır'ın yazma ilerlemesini ayrı göstermesi (bugün satırların ilerlemesiyle birlikte).

## Bağlam

ADR 0592 tek araç çalıştırmasına iki şey getirdi: girdiyi bağlı katmanın bütün tablosundan ya da bir dosyanın veya veritabanının
tablosundan okumak, sonucu PostGIS'e yazmak. Plan CBS-24 bunları model çalıştırıcıda ve toplu çalıştırmada da istiyor:
“Eksik provider dataset/feature stream girdisi, tipli tablo/raster/dosya sink'i ve batch parametre tablosunu aynı
descriptor/executor üzerinden ekle”. Bugünkü durumun dört eksiği vardı:

1. **Toplu çalıştırmanın sink'i yok.** `cad.processing.batch` yalnız çizime yazıyordu. Bir parametre tablosunun sonucunu dosyaya
   almak için her satırı ayrı `cad.processing.run` çağrısıyla çalıştırmak gerekiyordu.
2. **Toplu çalıştırma kaynaktan okumaz.** Satırların girdisi çizimin katmanıydı. Kurumun tipik işi (her ilçenin tablosunu, her
   süzgeci bir satır yapmak) yapılamıyordu.
3. **Kayıt yazılan yeri bilmiyor.** Sonucun dosyası cevapta vardı ama kaydın içinde yoktu. Komut geçmişinin satırı da yazılan
   dosyayı söylemiyordu (`effects` boştu, ADR 0551 §6 bunu sonraya bırakmıştı).
4. **Kaynaktan okunan girdinin kaydı yanlıştı.** ADR 0592'nin kaydında `parameters` girdiyi kopyadaki katmanın adıyla
   (`kaynak-input`) yazıyordu. Kayıtla yeniden çalıştırmak çizimde olmayan bir katman arıyordu. Kaydın `documentRevision`'ı
   kopyanın sürümüydü, çizimin değil. Kaynak dosya sonradan değişince “aynı veri” denebiliyordu.

## Karar

### 1. Toplu çalıştırmanın sink'i

`cad.processing.batch` `cad.processing.run`'ın `sink`'ini alır (yol ya da veritabanı adresi, `schema`, `mode`, `overwrite`,
`tables`; kurallar ve retler aynı). Sink verilince:

- Satırlar sırayla, çizimin **bir kopyasında** çalışır. Her satır `cad.processing.run`'ın aynı çalıştırıcısıdır (aynı `ready`,
  aynı `run_on`, modeller dahil). Çizim değişmez: geri alma adımı yok, revizyon aynı.
- Satırların yaptığı nesneler **tek yazmada** dosyaya ya da veritabanına gider, `cad.processing.run`'ın yazıcısıyla
  (`write_sink`): GeoPackage'da tek SQLite işlemi, PostGIS'te tek veritabanı işlemi. Bir katman bir tablodur. Aynı çıktı katmanını
  veren satırlar, çizimde olduğu gibi, o katmanın tek tablosunu doldurur. Her satıra ayrı tablo için satırlar ayrı katman adı
  verir (Toplu çalıştır'ın “Çıktı katmanı” sütunu). Tablo çıktıları (`sink.tables`) her satırın kendisidir. İki satır aynı adı
  verirse ad satırın adını alır: “Özet tablosu (Satır 1)”.
- `onError`: `stop`'ta duran satır bütün işi geri alır, hiçbir şey yazılmaz (`batch_row_failed`). `continue`'da yalnız o satır
  kopyada geri alınır, ötekiler yazılır. Hepsi durursa dosyaya dokunulmaz, cevapta sink yoktur.
- Durdurma satırlar arasında, aracın içinde ve yazmanın grupları arasında duyulur. Hiçbir şey yazılmaz, dosya eskisi gibi kalır.
- Cevap (`ProcessingBatched`) `sink` ve `sinkReport`'u `cad.processing.run`'ınkiyle aynı biçimde taşır. Özet yeri söyler: “2 satır
  çalıştı: 24 nesne toplu.gpkg dosyasına 2 tabloya yazıldı.”
- Soy belgesi (`kentos.lineage` 1; GeoPackage'da metadata uzantısı, PostGIS'te tablonun yorumu) toplu işin isteğini ve kaydını
  taşır. `run.kind` `batch`'tir.

### 2. Toplu çalıştırmada kaynaktan okuma

- Toplu işin kendi değerlerindeki (`parameters`) `scope: source` girdisi **bir kez** okunur, kopyada `kaynak-<parametre>`
  katmanına. Bütün satırlar onu okur.
- Bir satırın kendi `scope: source` girdisi o satır için okunur, `kaynak-<parametre>-<satır numarası>` katmanına. Böylece her
  satır başka bir tabloyu ya da aynı tablonun başka süzgecini okuyabilir.
- Kaynaktan okunan girdi sink ister. Ortak girdide ret yolu `parameters.input`, satırınkinde `rows[i].parameters.input`'tur.
- Ortak girdinin okuma retleri satır çalışmadan önce gelir, kendi kodlarıyla: `table_not_found`, `invalid_extent`,
  `invalid_expression`, `import_unreadable`, `layer_not_found`, `input_too_large`. Bir satırın kendi girdisinin retleri o satırın
  durmasıdır (`batch_row_failed` ya da `continue`'da `failed`).
- Parçalı bağlı katman `scope: layer` ile verilmişse `partial_scope` bir kez söylenir.
- **Kopyadaki kaynak katmanının adı** artık tablonun adı değil, “bina (kaynak)”tır. Ortak durumların biri bunu buldu: çıktı
  katmanının adı tablonun adıyla aynıysa (`newName: "bina"`), araç katmanı adıyla yeniden kullanıp sonucu kaynağın katmanına
  yazıyordu (ADR 0592'de de vardı).

Modeller aynı yoldan geçer. Bir modelin nesne girdisi `scope: source` alır, toplu işin satırı bütün modeli çalıştırır.
`tests/batch.rs`: parsel ölçü yazıları modeli `bina`'nın 12 karesinden 48 köşe noktası yazar. Kenar yazılarının GeoPackage'da
karşılığı yoktur, bunu `export_losses` söyler. Alanın yazıldığı kareler kopyanındır.

### 3. Kayıtta yazılan yer ve kaynağın dosyası

- `ProcessingRecord.sink` (`RecordSink`):
  - `path`: dosyanın yolu ya da veritabanının adresi (parolasız);
  - `schema`: veritabanında;
  - `mode`;
  - `tables`: her biri `table`, `of` (katmanın kimliği ya da tablo çıktısının adı; toplu işte `rows[i].table`), `row` (tabloyu
    yalnız bir satır yaptıysa `rows[i]`) ve `rows`;
  - `bytes` ve `sha256`: yazmadan sonra dosyanın, veritabanında yok.

  Run'ın ve toplu işin kaydı bunu taşır. Dosya sonradan değişse de kayıt değişmez.
- `RunInput.path`: kaynaktan okunan girdinin dosyası ya da adresi. `revision` (dosyanın SHA-256'sı), `table` ve `rows` yanındadır.
  Cevabın `inputs`'u ve kaydın `inputs`'u aynıdır.
- Kaydın `parameters`'ı kaynaktan okunan girdiyi **verildiği gibi** tutar (`{ "scope": "source", "path", "table", "filter" }`).
  Kayıttaki değerlerle `cad.processing.run` kaynağı yeniden okur.
- `documentRevision` çizimin sürümüdür, kopyanınki değil.
- `rerun_kind` dört hâl ayırır:
  - `SameRevision`;
  - `SameData`;
  - `ChangedData`: okunan katman ya da kaynak dosyası değişti. Dosya SHA-256'sıyla karşılaştırılır.
  - `UnknownSource`: veritabanından okundu, o anki hâli kayıtta yok.

  Aynı sürümde çizimin katmanlarına bakılmaz, yalnız kaynak dosyalarına. Kaynak dosyası değiştiyse çizim aynı sürümde olsa da
  “değişmiş veri” denir.

### 4. Komut geçmişinde yazılan dosya

`cad.processing.run` ve `.batch` kayıtlı adım olarak (ADR 0551 §6) yazdıkları dosyayı satırın `effects`'ine yazar: yol, boy,
SHA-256. Bu, dosya komutlarının yaptığıyla aynıdır (ADR 0253 §10). Veritabanı dosya değildir, plan dosya yazmaz. İkisinde de
`effects` boştur. Uygulama katmanında `call::recorded_with`, komutun çıktısından etkileri söyleyen bir işlevle `recorded`'dır.

### 5. Masaüstü

Toplu çalıştır penceresinde, Hata olursa'nın altında, aracın formundakiyle aynı **Sonucun yeri** vardır: Çizime, GeoPackage
dosyasına, PostGIS veritabanına. Bağlı girdinin **Bütün tablo (kaynaktan oku)** anahtarı da oradadır. İkisi aracın penceresiyle
aynı durumu paylaşır: formda seçilen yer tabloda da seçilidir. Boş yol ya da adres çalışmadan önce söylenir. Kayıt panelinde
“Sonuç: yol · tablolar (satırları) · SHA-256” satırı vardır. Çalıştırma, öncekiler gibi, komutun işidir.

## Doğrulama

- Ortak durumlar `fixtures/commands/v1/cad.processing.batch.sink.json`, 29 durum. Bağımsız başvuru
  `scripts/fixtures/processing_batch_sink_cases.py`: sorgu.gpkg'nin kayıtları sqlite3'le, dosyanın SHA-256'sı hashlib'le, katman
  kimliği ADR 0217'nin kuralıyla, KentOS kodu olmadan. Durumlar:
  - iki satır kendi süzgeçleriyle;
  - yazılan dosya ve kaydın tam hâli;
  - ortak kaynak bir kez okunur;
  - aynı katmana yazan satırlar;
  - üç adlı satır;
  - kutu;
  - bağlı katmanın bütünü ve parçası (`partial_scope`);
  - `addTables`, `table_exists`, `replaceTables`;
  - `stop`, `continue` ve hepsi duran;
  - `nothing_to_export`;
  - tablo çıktılarının satır adıyla ayrılması;
  - 13 ret.

  ADR 0592'nin 29 durumu `path`'le yeniden üretildi.
- `crates/native/processing/tests/batch.rs`:
  - kaydın verilen girdiyi, dosyayı ve sonucun SHA-256'sını tutması, komut geçmişinde `effects`;
  - kaynak dosyası değişince `ChangedData`;
  - veritabanı girdisinde `UnknownSource`;
  - modelin kaynaktan dosyaya, tek başına ve toplu işte yazması.
- `crates/native/postgis/tests/all.rs`'in `processing_results_go_to_postgis`'i geçici docker PostGIS'inde
  (`scripts/fixtures/postgis_cases.py`). Toplu iş `sonuc.Merkezler`'i iki süzgeçle okur ve `toplu` şemasına iki tablo yazar.
  Kayıtta şema vardır, dosya yoktur, komut geçmişinde etki yoktur, yeniden çalıştırma “bilinmiyor”dur. psycopg2 noktaları,
  kalıcı kimlikleri ve soyun `batch` olduğunu denetler.
- Masaüstü: `processing::sink_tests::a_batch_reads_whole_and_writes_each_row_to_a_table_of_the_file`. Parçalı bağlı katmanın 12
  kaydı iki satırda iki tabloya yazılır, çizim değişmez. Resimler `.run/shots/islem-sonuc-toplu-*`.
