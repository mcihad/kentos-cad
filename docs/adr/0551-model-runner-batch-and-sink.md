# ADR 0551: Toplu çalıştırmada hata kipi, satır içinde durdurma, tablo çıktısı ve hızlı katman özeti

- **Durum:** kabul edildi (2026-10-20). `plan/gis.md` CBS-24'ün kalanı (ADR 0529'un “sonraya kalan”ı). Kapsamı koordinatörün
  sırasıyla ben belirledim; sahibe sorulacak bir ürün çatallanması yoktu.
- **Bağlam belgesi:**
  - ADR 0529 (toplu çalıştırma, kayıt);
  - 0549 (var olan GeoPackage'a yazma);
  - 0508 (işlem araçları komut olarak, `sink`);
  - 0511 (sonucun soyu, girdi özeti);
  - 0253 (komut protokolü, işler);
  - 0515 (raster sonuçları COG).
- **Web:** ertelendi (ADR 0228). Web'de gerekenler TODOS.md §0.2'de.
- **Sonraya kalan:**
  - sağlayıcıdan sayfa sayfa akan girdi (§7);
  - kaydın proje dosyasında saklanması (§6: bilerek yapılmadı, `.kcad` şeması 92 boş kaldı);
  - aracın kendi hesabını bölmek: araç sonucunu belleğe kurar (ADR 0549 §3);
  - masaüstü İşlemler penceresinin tek-araç formunda sink seçeneği (yalnız Toplu çalıştır'a `Hata olursa` geldi);
  - katman özetinin artımlı hesabı (§5'in sonu).

## Bağlam

ADR 0529 toplu çalıştırmayı ve çalışmanın kaydını getirdi, altı şeyi sonraya bıraktı. Bu ADR dördünü kapatır, ikisini gerekçeyle
bırakır:

| ADR 0529'un “sonraya kalan”ı | Bu ADR |
|---|---|
| `onError: continue` | §1, §2 |
| satırın içinde durdurma | §4 |
| hızlı katman özeti | §5 |
| tablo sink'i | §3 |
| raster sink'i | §8: zaten var, söylendi |
| kaydın projede saklanması | §6: karar verildi, saklanmaz |
| sağlayıcıdan akan girdi | §7: sonraya, nedeniyle |

## Karar

### 1. `onError`

`cad.processing.batch` girdisinin `onError`'ı:

- `stop` (varsayılan, ADR 0529'daki gibi): çalışırken duran bir satır bütün toplu işi geri alır, `batch_row_failed`.
- `continue`: duran satır **tek başına** geri alınır, öteki satırlar yazılır; hepsi yine tek geri alma adımıdır (“Toplu: <araç>”).
  Duran satırlar cevabın `failed` listesindedir (`index`, `label`, `code`, `message`: satır tek başına çalışsaydı vereceği ret),
  kaydın `failed`'ındadır (`rows[i]` kimliğiyle) ve `batch_rows_failed` uyarısı bunu söyler. Duran satır `rows`'ta yoktur, kaydın
  adımlarında da. Bütün satırlar durursa hiçbir şey yazılmaz: adım yoktur, revizyon değişmez, uyarı “hiçbir satır yazılmadı” der.

Ne bir satırın hatası sayılır: aracın çalışırken durması (`tool_failed`: kilitli katman, alanın değeri reddetmesi, girdi yok) ve
değerlerinin satırdan önceki satırların yazdıklarına artık uymaması. Sayılmaz:

- **çalışmadan önceki retler** (bilinmeyen araç, UUID olmayan kimlik, çizimde olmayan nesne, aracın olmayan parametresi, yarım
  ifade): bunlar isteğin hatasıdır, `continue` ile de hiçbir şey yazılmadan reddedilir (yolu `rows[i].…`);
- **durdurma** (§4): iptal bir satır hatası değildir, hangi kipte olursa olsun bütün toplu iş geri alınır.

### 2. Satırın geri alınması: `Document::group_savepoint` ve `rollback_to`

ADR 0529 “iç içe geri alma” yokluğunu engel olarak yazmıştı: bir `begin_group` başka bir grubun içinde boş bir belirteçtir ve duran
modelin kendi grubu önceki adımlarını geri alamaz. Çözüm iç içe grup değil, yer imidir:

- `group_savepoint() -> Option<Savepoint>`: değişikliklerin şimdi toplandığı yerin işareti. Komut protokolünün açık adımı
  (`run_step`: komut gövdesi onun içinde yazar, her satırın `transact`'ı ona katılır) varsa orası, yoksa açık grup; ikisi de yoksa yok.
- `rollback_to(Savepoint)`: işaretten sonra yazılanı en yeniden eskiye geri alır, işaretten sonra kuyruğa alınan komut satırlarını da
  atar, adımı ya da grubu açık bırakır; hiçbir şey kaydedilmez (kuşak ilerler, ADR 0020 “alınan grup”la aynı kural).

Modelin ilk adımının nesneleri ve **katmanı** da böylece satırla birlikte gider (katman ekleme/silme de işlemdir). İşaretin iki yeri
vardır çünkü satırın yazdığı iki yerden birinde durur: komut protokolünden gelen çağrıda (Python, MCP, masaüstü penceresi, ortak
durumlar: ADR 0551 §6'dan beri hepsi) açık adımda, doğrudan çağrıda (`ToolHost::batch`, testler) grupta.

### 3. Tablo çıktısı: `sink.tables`

Aracın tablo çıktıları (özet, rapor, matris; `{ columns, rows }`) bugüne dek `sink`'e gitmezdi: tablo üreten ve nesne üretmeyen araç
`nothing_to_export` alırdı. `sink.tables: true` ile bu tablolar GeoPackage'a **öznitelik tablosu** olarak yazılır:

- tablonun adı çıktının etiketidir (“Özet tablosu”); boş ya da `gpkg_`/`rtree_` ile başlıyorsa `tablo_` önekiyle, dosyada ya da aynı
  yazımda bir ad varsa `_2`, `_3` … eklenir; aynı adlı tablo `table_exists` kuralına uyar (ADR 0549 §1: `addTables` reddeder,
  `replaceTables` yenisiyle değiştirir);
- sütunlar başlıklardır (boş başlık `sutun_N`, tekrar eden ya da `fid` `_2`, `_3` …), **hepsi metindir**: hücreler aracın gösterdiği
  yazıdır (“2500.00”, “12,5”), sayıya çevrilmez; çevirmek aracın sayı biçiminin (ondalık, basamak) dosyaya sızması olurdu;
- sıra: önce nesne katmanlarının tabloları, sonra tablo çıktıları, çıktının adına göre;
- satırlar yine 5000'lik gruplarla ve aynı işlemde yazılır; iptal ve hata aynı kuralla dosyayı olduğu gibi bırakır;
- `sinkReport.tables[i]`'de `layer` yerine `output` (aracın çıktısının adı) vardır, `geometry` yoktur, `z` ve `m` `none`;
  `sink.tables[i].layer`'da çıktının adı durur (`file.export.gpkg`'nin şeması korunur);
- `nothing_to_export` iletisi `sink.tables`'ı söyler.

Tablolar kendi soy belgesini taşır (ADR 0511): `layer: null`, `output: <ad>`.

### 4. Satır içinde durdurma

Durdurma bugüne dek satırlar arasında ve sink'in grupları arasında duyuluyordu; uzun süren bir aracı yarıda kesmenin yolu yoktu.
Araçlar zaten kendi döngülerinde `feedback.canceled()` sorar (hesaplayıcılar, raster işleri, adres, nokta bulutu); eksik olan,
sorunun cevabının canlı bir bayrağa bağlanmasıydı:

- `BatchFlow::stop_flag() -> Option<Arc<AtomicBool>>` (varsayılan yok): iş olarak çalışan komutun durdurma bayrağı
  (`Progress::stop_flag`; iş parçacığı sınırını aşan tek bayrak).
- `Runner::with_stop(bayrak)`: çalıştırıcının araca verdiği geri bildirim bayrağı sorulduğunda okur; araç başlamadan önce de
  sorulur (başlamamış araç hiç başlamaz). Model adımlar arasında ve çalışan adımın içinde durur.
- Sonuç: `Outcome::Stopped { status: Canceled }`, çalıştırıcının tarihçesinde “iptal” kaydı, çizim olduğu gibi.
  `cad.processing.run` bunu `RunEnd::Cancelled` yapar (sink'siz çalıştırma da artık iş olarak durdurulur); toplu iş **bütün**
  toplu işi geri alır (grup iptal edilir) ve `Cancelled` döner, hangi `onError` ile olursa olsun.

Kendisi hiç sormayan araç bitince duyulur: komut o zaman da `Cancelled` döner ve hiçbir şey yazılmaz; yazmadan önceki son soru
bunun için vardır. Sınır: durdurma aracı **kendi soru noktasında** keser; hesap sorusuz bir döngüdeyse o döngü biter.

### 5. Hızlı katman özeti ve doğru anahtar

- **Hız.** `sources::layer_digest` (ADR 0503, 0511) aynı baytları üretir, ama nesnelerin kanonik yazımını 32 768'lik turlarda,
  turu makinenin iş parçacıklarına (en çok sekiz) bölerek hazırlar ve SHA-256'ya sırayla verir: bayt sırası aynıdır, özet aynıdır
  (`tests/all/layer_digest.rs` 40 000 nesnede tek parçalı kuralla karşılaştırır). Bellek turla sınırlıdır (bir milyon nesne
  bir kerede yazılmaz).
- **Anahtar.** Bir toplu işin satırları aynı katmanı okurken özet bir kez alınır. Anahtar bugüne dek çizimin kuşağıydı; komut
  protokolünün açık adımında satırların yazdığı değişiklik kuşağı oynatmaz (kuşak adım kapanınca ilerler), yani ikinci satır birinci
  satırın yazdığını okurken **eski özeti** alırdı. Anahtar `Document::change_mark()` oldu: her nesne işleminde (düzenleme, geri alma,
  geri alınan satır) ilerler; eşit işaret, aralarında hiçbir nesne işlenmedi demektir. `tests/batch.rs`'te açık adımda ikinci satırın
  özeti birincinin ve üçüncünün özetinden farklıdır.

### 6. Kayıt proje dosyasına yazılmaz; kalıcılık nerede

Karar: **`.kcad`'e `runRecords` yazılmaz, şema 92 boş kaldı.** Gerekçe: kayıt çizimin bir parçası olsaydı yazılması bir düzenleme
olurdu: çizimin revizyonu oynardı (seçen bir araç bile belgeyi kirletirdi), “çalışma ile aynı sürümde” ayrımı (`SameRevision`)
hiçbir zaman doğru çıkmazdı, her çalışma dosyayı büyütürdü, geri alma geçmişiyle ilişkisi tanımsızdı. Kalıcılık bu yerlerde vardır:

1. **İstek defteri.** `requestId` verilen çağrının cevabı, kaydıyla, proje dosyasının defterinde saklanır (ADR 0253 §6, `.kcad` şema
   45; 256 istek, her cevap 64 KB'a kadar). Bunun doğru olması için yapılan bir düzeltme: `cad.processing.run` ve `.batch` şimdiye
   dek protokolün kayıtlı adımına (`call::recorded`) sarılı değildi, yani ne komut geçmişine, ne deftere, ne `requestId`'nin “aynı
   istek bir kez uygulanır” kuralına giriyorlardı. İkisi de artık diğer yazan komutlar gibi tek kayıtlı adımdır: komut geçmişinde
   Türkçe başlığı ve Python karşılığıyla görünür (masaüstü penceresinin çalıştırmaları dahil), aynı `requestId` ikinci kez
   uygulanmaz, cevabı (kayıt dahil) deftere yazılır. Dosyaya yazan (`sink`) çalıştırma çizimi değiştirmez; satırı geri alma adımı
   olmadan defterdedir (yazılan dosyanın `effects`'i bu ADR'de journal'a eklenmedi, sonraya).
2. **Sonucun soyu.** Sink'e yazılan her tablo kaydı GeoPackage'ın metadata uzantısında taşır (`kentos.lineage` 1): belge artık
   `run` alanıyla çalışmanın bütün kaydını (araç ve sürümü, değerler, girdi özetleri, motor, süreler, adım sayıları) içerir. Veri
   kaynağı sonradan değişse de dosyadaki kayıt değişmez.
3. **Komut cevabı.** Kayıt bir değerdir; script'in kendisi JSON olarak saklar.

Çizimde gösterilen “bu proje geçmişte neleri çalıştırdı” listesi, gerekirse, defter üzerine kurulur ve ayrı bir karardır.

### 7. Sağlayıcıdan akan girdi: sonraya

Araçlar girdilerini çizimin belgesinden (`reading_copy`) alır; bağlı bir katmanın nesneleri çalışma kopyasındadır, kaynağın
sayfalarında değil. Aracı doğrudan sayfalara bağlamak her aracın girdisini akış yapmayı, ortak sorgu çekirdeğinin (ilişkiler,
tamponlar, birleştirme) bütün katmanı bekleyen yapısını değiştirmeyi ister; temiz bir dilim değil (ADR 0549 §6 ile aynı sonuç).
Bugünkü yol bağlı katmanı çalışma kopyası olarak açmaktır (alan sınırlı ya da bütün, ADR 0503, 0510); sonuç artık sayfalı ve ya hep
ya hiç yazılır. Akışlı girdi, araçlar akışlı hesaba taşındıkça (ADR 0542'nin dayanıklı işleriyle) yapılacak.

### 8. Raster sink'i: zaten var

Raster araçlarının sonucu GeoPackage'a gitmez: dosyaya yazılan bir raster (GeoTIFF ya da COG) aracın kendi **dosya parametresidir**
(`output`): çalıştırıcı sonucu yanına yazar, bitince yerine koyar (`Files::create` → `Sink::finish`/`finish_cog`, ADR 0515); yarıda
kalan, durdurulan ya da hata veren çalıştırma geride dosya bırakmaz (`Drop`). Yani “uzun dosya çıktıları staged kalır” ölçütü
raster için karşılanıyordu; bu ADR yeni bir raster sink'i eklemez. §4 sayesinde bu araçlar artık satır içinde de durur.

### 9. Masaüstü: Toplu çalıştır

Tablonun üstünde **Hata olursa: Dur | Devam et** seçimi (KentOS UI `Segmented`, ipuçlarıyla). Devam et ile biten çalıştırmanın
altbilgisi uyarı simgesiyle özeti söyler; duran satırın hücresi “Durdu; nedeni aşağıda”, tablonun altında **Duran satırlar (geri
alındı)** kutusu her satırın adını ve nedenini yazar; kayıt kutusu yalnız çalışan satırları adım olarak sayar.

## Uygulama

- `crates/native/domain/src/history.rs`: `Savepoint`, `Document::group_savepoint`, `rollback_to`.
- `crates/shared/commands/src/cad_processing.rs`: `BatchOnError`, `ProcessingBatch.onError`, `BatchRowFailed`,
  `ProcessingBatched.failed`, `RecordFailure`, `ProcessingRecord.failed`, `RunSink.tables`, `SinkTable.output` ve isteğe bağlı
  `layer`; `codes.rs`: `batch_rows_failed`; `capabilities.rs`: yeni uyarı ve ADR 0549'un retleri.
- `crates/native/processing/src/command.rs`: `Halt`, `run_on(…, stop)`, toplu işin satır döngüsü, `text_tables`,
  `Lineage.run`; `batch_table.rs`: `Table.on_error`; `runner.rs`: `Runner::with_stop`, `stop_asked`, `Collect.stop`.
- `crates/native/application`: `processing_run.rs` ve `processing_batch.rs` (`call::recorded` ile kayıtlı adım), `processing_host.rs` (`BatchFlow::stop_flag`, `BatchEnd::Ran`'ın uyarıları), `jobs.rs`
  (`Progress::stop_flag`), `geopackage.rs` (`TextTable`, `write_with_tables`, `table_name`), `sources.rs` (`layer_digest`).
- `apps/desktop/src/processing/batch.rs`: `OnError`, `Hata olursa` seçimi, `failed_panel`, `Line::Partial`.
- Python `kentos.cad.processing` (üretilir): `on_error`, `failed`, `tables`, `output`.

## Doğrulama

- **Ortak durumlar:** `cad.processing.batch` 31 durum (10 yeni: `onError` — ortadaki, ilk, son, iki satırın durması; hepsinin
  durması; hiçbiri durmaz; açık `stop`; plan; model satırlarının ortadaki adımda durması ve ilk adımın katmanıyla geri alınması;
  çalışmadan önceki ret); `cad.processing.run` 89 durum (12 yeni: tablo çıktısı yeni dosyaya, plan, var olan dosyaya ekleme, ad
  çakışması, değiştirme, dört aracın tablosu, nesneyle birlikte tablo, `sink.tables` yokken ve tablo yokken `nothing_to_export`).
  Beklenenler İşlemler'in kendi bağımsız ortak durumlarından ve ADR'den; tablo adı web'in dondurulmuş tanımındaki etiketten.
- **Bağımsız dosya denetimi:** `processing_sink_cases.py --check` (OGR, sqlite3, GDAL'ın `validate_gpkg.py`'si): `kentos-run-ozet.gpkg`
  `statistics.summary`'nin tablosunu Ada'ya göre özeti Python'da yeniden hesaplanmış değerlerle (toplam, ortalama, en az, en çok,
  örneklem sapması; okunamayan değer atlanır) karşılaştırır; öznitelik tablosu, metin sütunları, soy belgesinde `run`.
- **Testler:** `kentos-domain` `document.rs` (savepoint: grup ve açık adım); `kentos-processing` `tests/batch.rs` (14: devam,
  durdurma, hepsi durur, model satırı, satır içinde durdurma, açık adımda özet anahtarı), `tests/stop.rs` (aracın kendi sorusunda
  durma); `kentos-native-application` `processing_sink` (7), `layer_digest` (2); masaüstü `batch_tests` (devam).
- **Süre** (release, Apple M5, 20 000 parsel, `cargo test --release -p kentos-processing --test batch batch_timing --
  --ignored --nocapture`):

  | Toplu iş | ADR 0529 | Bu ADR |
  |---|---|---|
  | 20 satır, girdi katmanını yazar | 3 704 ms (185 ms/satır) | 1 748 ms (87 ms/satır; satırın kendi işi 27–52 ms) |
  | 20 satır, yalnız seçer | 544 ms (27 ms/satır) | 338 ms (17 ms/satır) |
  | katmanın özeti tek başına | 162 ms | 54 ms |

  Özet 3 kat hızlandı (sekiz iş parçacığı, SHA-256 ardışık kaldı); katmanı yazan satırda satır başına ek yük 135 ms'den 54 ms'ye
  indi ama hâlâ satırın kendi işinden (27–52 ms) büyük: ADR 0529'un “ek yük satırın işinden küçük olmalı” bütçesi bu yazan satırlar
  için **hâlâ aşılıyor**. Kalan, nesne başına önbelleğe alınmış bayt ve değişen nesneler için artımlı özettir (`changes_since`'in
  verdiği yuvalar bunu mümkün kılar); özetin değeri aynı kalacağı için sonraya yazıldı.
- **Görsel inceleme:** `.run/shots/islem-toplu-*` (Hata olursa seçimi, Devam et ile biten çalıştırma, duran satırın kutusu;
  1440×900 ve 1100×650, koyu ve açık).
- **Ölçülmeyenler:** 500 satırlık tablonun çizimi; PostGIS bağlı katmanda toplu iş; bir milyon satırlık tablo çıktısı.
