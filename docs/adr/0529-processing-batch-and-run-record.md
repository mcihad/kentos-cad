# ADR 0529: İşlemler'de toplu çalıştırma, modellerin komutla çalışması ve çalışmanın kaydı

- **Durum:** kabul edildi (2026-10-19). `plan/gis.md` CBS-24'ün ilk dilimi; kapsamı ben belirledim. Aynı araç ya da model bir
  parametre tablosunun satırlarıyla, `cad.processing.run`'ın aynı çalıştırıcısıyla ve tek geri alma adımıyla çalışır; her
  çalışma (araç, model, toplu iş) kendi kaydını verir.
- **Bağlam belgesi:** ADR 0084 (İşlemler), 0116 (Model tasarımcısı), 0124 ve 0125 (arka planda çalışma, modelin yeniden
  oynatılması), 0253 (komut protokolü, işler), 0508 (işlem araçları komut olarak, `sink`), 0511 (sonucun soyu, girdinin
  içerik özeti).
- **Web:** ertelendi (ADR 0228); geri dönüşte gerekenler TODOS.md §0.2'de.
- **Sonraya kalan:**
  - `onError: continue`: bir satır durunca öbürlerinin yazılması. `Document` grupları iç içe geri alınamaz (içteki
    `begin_group` boş bir belirteçtir), bu yüzden durmuş bir model satırının kendi adımlarını tek başına geri alamayız;
    çözüm belgenin iç içe geri alma noktalarıdır.
  - satırın içinde durdurma: iptal satırlar arasında duyulur; çalışan aracı yarıda kesmek için `Runner`'ın iptal bayrağı
    iş parçacığından yazılabilir olmalı.
  - kaydın proje dosyasında saklanması (`.kcad` şeması); bugün kayıt komutun cevabında ve masaüstü pencerelerindedir.
  - sağlayıcıdan akan girdi (`provider` dataset/feature stream), tablo ve raster sink'i, batch'te `sink`; bunlar CBS-24'ün
    öbür yarısıdır.
  - girdi özetinin hızı: katmanın özeti nesne başına yaklaşık 8 µs (ölçüm aşağıda); artımlı ya da önbellekli özet.
  - kayıttan tekrar çalıştırma komutu (`cad.processing.rerun`): bugün kaydın `parameters`, `selection` ve `view`'u
    `cad.processing.run`'a aynen verilir; `rerun_kind` yalnız ne ile karşılaşacağını söyler.
  - İşlemler'in Geçmiş sekmesinde toplu çalıştırmaların kartı (toplu iş komut geçmişinde görünür; oturumun çalıştırıcı
    geçmişine yazmaz) ve kayıttan “Yeniden çalıştır” sorusu.
  - kullanıcının kendi modelleri komutla çalışmaz (başsız ev sahibi yalnız yerleşik modelleri bilir).

## Bağlam

İşlemler'in araçları ve modeli tek tek çalışıyordu. QGIS Processing'in toplu işleme (batch) penceresi ve ArcGIS
ModelBuilder'ın yineleyicisi aynı işi bir parametre tablosuyla tekrarlar; sonuç “bir tanım” olarak saklanabilir. KentOS'ta
ne tablo ne de bir çalışmanın nelerden yapıldığını söyleyen bir kayıt vardı. Plan: bir çalışmanın girdisinin hangi
sürümden olduğu, akan kayıt sayısı, adım süresi ve motor sonradan söylenebilmeli; veri kaynağı sonradan değişse eski
çalışmanın bilgisi değişmemeli; “aynı parametrelerle güncel veride” yeniden çalıştırma ile “aynı sürümde” yeniden
çalıştırma ayrı söylenmeli; üç adımlı parsel modeli arayüzde, Python'da ve MCP'de aynı sonucu vermeli, ortadaki adım durunca
yerel değişiklikler geri alınmalı. Ayrı bir araç standardı kurulmaz: toplu iş, modeller ve kayıtlar `cad.processing.run`'ın
tanımından geçer.

## Karar

### 1. `cad.processing.batch`

- Girdi: `tool` (araç ya da `model:` kimliği), ortak `parameters`, `rows` (her biri isteğe bağlı `label` ve kendi
  `parameters`'ı), `selection` ve `view`. Bir satırın değerleri ortak değerlerin üstüne gelir; hepsinin üstünde aracın
  çizimdeki varsayılanı vardır. Her satır, o değerlerle `cad.processing.run`'dır: aynı `ready` (aynı retler), aynı
  çalıştırıcı, aynı tek aracın sonucu. En çok 500 satır.
- **Sıra:** satırlar sırayla çalışır, her biri öncekilerin yazdığını görür (ikinci satırın `'P-' || Parsel`'i birincinin
  `P-1`'ini okur).
- **Geri alma ve hata: tek adım, ya hep ya hiç.** Bütün toplu iş tek geri alma adımıdır (“Toplu: <araç>”). Her satır ayrı
  bir geri alma adımı olsaydı yarım kalan bir toplu işi geri almak için kullanıcı kaç satır çalıştığını bilmek zorunda
  kalırdı ve bir satırın hatası öncekileri yazılmış bırakırdı; plan modelin kuralına (“orta adım hata verirse yerel
  değişiklikler geri alınır”) uyar. Bir satır çalışıp dururlarsa ya da satırlar arasında durdurulursa (iş olarak,
  `ExecutionContext::stopped`) toplu işin grubu geri alınır, çizim, sürüm ve geri alma geçmişi olduğu gibi kalır.
- **Retler sırayla:** `tools_unavailable`, `empty_batch`, `too_many_rows`; sonra **her satırın** `cad.processing.run`
  retleri (`unknown_tool`, `invalid_uid`, `entity_not_found`, `invalid_parameters`) çalışmadan önce verilir, yolu satırın
  kendi değeriyse `rows[i].parameters.x`, ortak değerse `parameters.x`; bir satırın hatası öncekiler geçerli olsa da
  hiçbir şey yazdırmaz. Çalışırken: `batch_row_failed`, yolu `rows[i]`, iletisi “Satır N çalışmadı: <neden> Toplu işin
  hiçbir sonucu yazılmadı.”.
- **Cevap:** her satırın `cad.processing.run` cevabı (`rows[i].ran`, kendi kaydıyla), toplu işin özeti (“N satır çalıştı:
  a nesne eklendi, c nesne değişti.”), adımın adı ve toplu işin kaydı.
- `plan` aynısını çizimin okuma kopyasında çalıştırır; `validate` yalnız retleri denetler; `sink` bu komutta yoktur.
- Komut bir iş olarak da çalışır (`jobs::Job::start`): hesap okuma kopyasında başka iş parçacığında, ilerleme satır sayısıyla
  (`ExecutionContext::progress`), Durdur satırlar arasında duyulur. Masaüstü penceresi bunu böyle kullanır.

### 2. Çalışmanın kaydı (`ProcessingRecord`, `kentos.run-record` 1)

Her `cad.processing.run` cevabı `record` taşır (araç, model ve toplu iş için aynı biçim):

| Alan | Anlamı |
|---|---|
| `kind` | `tool`, `model` ya da `batch` |
| `tool`, `label`, `toolVersion` | araç ya da `model:` kimliği; aracın sürümü (bir aracın aynı değerlerden hesapladığı değişince artar; bugün hepsi 1) |
| `definition` | modelin tanımının SHA-256'sı: adımları, bağları, sabit değerleri, girdileri ve çıktıları; tasarımcıdaki kutu yerleri hariç. Araçta yok |
| `parameters` | koşunun aldığı değerler, varsayılanlar doldurulmuş; nesneler kalıcı kimlikle (`scope: uids`), dosya adı ve yoluyla (satırları değil). Aynen `cad.processing.run`'a verilince aynı işi yeniden çalıştırır. Toplu işte ortak değerler (satırınkiler satırın kendi kaydında) |
| `selection`, `view` | girdinin kendi seçimi ve kutusu |
| `inputs` | okunan katmanlar ve **okunduğu andaki içerik özetleri** (ADR 0511), bağlı katmanın kaynak sürümü |
| `documentRevision`, `srid` | çalışma başlarken çizimin sürümü; projenin koordinat sistemi (yerel proje: yok) |
| `seed` | `seed` parametresi olan aracın tohumu |
| `engine`, `host`, `startedAt`, `ms` | `KentOS <sürüm>`, `desktop` ya da `headless`, başlangıç (UTC), toplam süre |
| `steps` | araç tek adım; model adımları sırasıyla; toplu iş satırları (`rows[i]`). Her adımda `recordsIn`, `recordsOut`, `ms`, `summary` |
| `total` | `recordsIn` (adımların girdileri toplamı: birkaç adımın okuduğu nesne her biri için sayılır), `recordsOut` (çalışmanın yaptığı, değiştirdiği ya da seçtiği ayrı nesneler) |

- **Sayılar kesindir:** `recordsIn` aracın girdi nesnelerinden (`Job::size`: öznitelik girdileri çözüldükten sonra), `recordsOut`
  yazılan ve seçilen nesnelerden, çalışma bittiğinde sayılır. İlerleme (`Feedback::progress`) yaklaşık kalabilen bir akıştır;
  son kesin toplam `total`'dir. Akan sağlayıcı girdisi gelince bu sayı yine sonda kesin verilir.
- **Kayıt değerdir:** bir veri kaynağını sonradan değiştirmek kaydı değiştirmez. Kayıt komutun cevabındadır, masaüstünde
  pencerelerde gösterilir; proje dosyasına yazılmaz (`.kcad` şeması değişmedi, ayrı karar: ne kadar tutulacağı ve
  büyümesi).
- **Yeniden çalıştırmanın üç hâli** (`kentos_processing::record::rerun_kind`): çizim kaydın başladığı sürümde
  (`SameRevision`); sürüm başka ama okunan her katmanın içerik özeti aynı (`SameData`: aynı veriyle aynı sonuç);
  okunan bir katman değişmiş ya da yok (`ChangedData(katmanlar)`: aynı değerler güncel veride başka sonuç verebilir). Toplu
  çalıştırma penceresi bunu kayıtla birlikte söyler. (Çalıştırma kendi girdisini yazıyorsa, bitince `ChangedData` doğru
  cevaptır.)
- **Girdi özeti önbelleği:** bir katmanın özeti çizimin her durumu için bir kez alınır (`doc.generation()` anahtarıyla):
  katmana dokunmayan satırlar paylaşır, katmanı yazan satır yeni özeti alır.

### 3. Modeller komutla çalışır

`cad.processing.run` ve `.batch` `tool: "model:builtin.parcelSheet"` gibi kimlikleri kabul eder (ADR 0508'in “sonraya
kalan”ı): modelin girdileri parametreleridir, `Registry::get` aynı `ready`'den geçirir, `run_model` adımları sırayla çalıştırır
(ADR 0125'in grubu: tek adım, adı modelin adı). Orta adım durunca model grubu geri alır; komut `tool_failed` verir ve
mesaj modelin sözüdür (“2. adım (Kenar uzunluklarını yaz) çalışmadı: … Önceki adımların sonuçları geri alındı.”).
`cad.processing.tools` `models: true` ile modelleri de listeler. Yerleşik model tek: Parsel ölçü yazıları (üç adım:
Köşe noktalarını numarala, Kenar uzunluklarını yaz, Öznitelik hesapla); kullanıcının modelleri başsız ev sahibinde yoktur.

### 4. Masaüstü penceresi: Toplu çalıştır

İşlemler penceresinin altbilgisinde **Toplu çalıştır…** (ikon `table`; açılan tablo öbür ikonlar `tableColumnRight`,
`tableRowBelow`, `tableRowDelete`). Aynı pencere form yerine tabloyu gösterir (Forma dön geri getirir):

- **Sütun ekle ▾** aracın hücreye yazılabilen parametrelerini listeler (sayı, metin, evet/hayır, seçenek, katman, nesne
  kapsamı, ifade, alan); chip'in × işareti sütunu kaldırır. **Satır ekle**, satırın sil düğmesi (son satır kalır).
- Hücre metni parametrenin türüyle okunur (`batch_table::cell_value`; çekirdekte, pencerede değil): sayıda virgül, evet/hayır,
  seçenek değeri ya da adı, katman kimliği ya da adı (bulunmazsa yeni katman adı), nesne kapsamı için katman adı ya da
  “seçim”, “hepsi”, “görünen”. Boş hücre penceredeki değeri alır. Tabloya uymayan hücre satırıyla ve parametresiyle söylenir.
- **Toplu çalıştır** tablonun girdisini yapar (`batch_table::input`, penceredeki değerler `command_values` ile komutun
  sözüne çevrilir: nesneler kalıcı kimlikle) ve komutu iş olarak başlatır; satır durumları gelince doldurulur: çalışan satır
  “3 → 6 kayıt · 12 ms”, durmuş satır “Durdu; nedeni altta”, öncekiler “Geri alındı”, sonrakiler “Çalışmadı”. Alttaki
  satır nedeni ya da özeti söyler; **Durdur** satırlar arasında duyulur.
- **Kayıt** bölümü bitmiş toplu işin kaydını gösterir: tür, araç ve sürümü, motor, ev sahibi, çizim sürümü ve koordinat
  sistemi, her satırın adımı (girdi → çıktı kayıt, süre), toplam ve yeniden çalıştırmanın ne ile karşılaşacağı.
- İşlemler'in **Geçmiş** sekmesindeki kartlar çalıştırmanın girdi → çıktı sayısını, modelde her adımın payını ve süresini de
  yazar (`RunRecord.records_in`, `records_out`, `steps`).

### 5. Otomasyon

- Python `kentos.cad.processing.batch` ve `.run`'ın `record`'u katalogdan üretilir (`BatchRow`, `ProcessingBatched`,
  `ProcessingRecord`, `RecordStep`, `RecordTotal`); MCP'de `cad.processing.batch` yıkıcı listede (kayıtlı katalog komutu).
- Katalog: `catalog.rs`'te tanım, `capabilities.rs`'te üç ret grubu, `codes.rs`'te `empty_batch`, `too_many_rows`,
  `batch_row_failed`; birim sözlüğüne `ms`. Komut şemalarındaki her sayının birimi var (`x-unit`).
- Manifest (`catalog/v1/operations.json`): toplu çalıştırma bir araç değil komuttur; yeni araç eklenmediği için satır
  eklenmedi.

### 6. Performans

Toplu iş okuma kopyasında başka iş parçacığında çalışır; arayüz iş parçacığı yalnız bitince yazar. Bütçe: bir satırın
ek yükü (kayıt, özet, değerlerin yazımı) satırın kendi işinden küçük olmalı. Ölçüm (aşağıda) bunu yalnız girdi katmanına
dokunmayan satırlar için sağlar; katmanı yazan satırda katmanın özeti hesaptan büyüktür.

## Uygulama

- **Sözleşme** (`crates/shared/contracts`): `cad_processing.rs` (`ProcessingBatch`, `BatchRow`, `ProcessingBatched`,
  `BatchRowRan`, `ProcessingRecord`, `RecordStep`, `RecordTotal`, `ProcessingRan.record`, `ProcessingTools.models`),
  `catalog.rs`, `capabilities.rs`, `codes.rs`, `units.rs`.
- **Çekirdek** (`crates/native/processing`): `command.rs` (modeller, `run_on`, kayıt, toplu iş, `register_as`,
  `command_values`), `record.rs` (`rerun_kind`), `batch_table.rs` (tablo ve hücrelerin okunuşu), `runner.rs`
  (`RunRecord`'un sayıları ve `StepRecord`), `model_runner.rs` (adım kayıtları).
- **Uygulama** (`crates/native/application`): `processing_batch.rs`, `processing_host.rs` (`BatchFlow`, `BatchEnd`,
  `check_batch`, `batch`), `registry.rs`, `lib.rs` (`DESKTOP_COMMANDS`).
- **Masaüstü** (`apps/desktop`): `processing/batch.rs` (tablo durumu, iş, görünüm), pencerenin altbilgisi ve görünümü
  (`window.rs`), Geçmiş kartları (`panel.rs`), saat (`app.rs`), `main.rs` (`register_as("desktop")`).
- **Python / MCP:** `python/kentos/cad/*` (üretilir), `python/tests/test_processing_batch.py`; MCP `tools.rs`.
- **Seçilen ikonlar:** `table` (Toplu çalıştır…), `tableColumnRight` (Sütun ekle), `tableRowBelow` (Satır ekle),
  `tableRowDelete` (satırı sil).

## Doğrulama

- **Bağımsız başvuru:** `scripts/fixtures/processing_batch_cases.py` (KentOS kodu olmadan; satırların sonuçları İşlemler'in
  kendi bağımsız ortak durumlarından, toplu işin kuralları ADR'den) `fixtures/commands/v1/cad.processing.batch.json`'ı
  yazar: 21 durum. İki satırın aynı nesnelerde ayrı alanlar yazması ve tek geri alma adımı, satırın öncekinin yazdığını
  görmesi (“P-P-P-1”), ortak ve satıra özel değerler, satır adları (verilen, kırpılan, boş), farklı nesnelere kalıcı
  kimlikle, model araç olarak (modelin kendi ortak durumu), seçen araçlar ve kutu, çalışırken duran satır (geri alma),
  modelin orta adımının durması, plan ve validate, on bir ret (satır yolları dahil), kaydın sayıları.
  `processing_command_cases.py` `cad.processing.run.json`'a model durumlarını (modelin ortak durumu, bilinmeyen model, plan,
  orta adımın durması ve ilk adımın geri alınması) ve kayıtların tam beklentisini (adım sayıları, toplamlar) ekler:
  54 durum; `cad.processing.tools.json` 25 durum (modeller listede).
- **Testler:** `KENTOS_CASES_ONLY=cad.processing cargo test -p kentos-native-application --test all every_command_case`;
  `capabilities` testi her kodu bir durumda ya da gerekçesiyle sınar; `kentos-processing` `tests/batch.rs` (6: satırlar
  arasında durdurma bütün satırları geri alır, tek geri alma adımı ve ilerleme, toplu kaydın adımları ve sayıları,
  ortada duran model katman bırakmaz, kayıtta kalıcı kimlik ve tohum ve sistem, yeniden çalıştırmanın üç hâli) ve
  `batch_table` (3); masaüstü `processing::batch_tests` (5: pencereden toplu çalıştırma gerçek işle, tek geri alma adımı ve
  kayıt, hücre reddi, durmuş satır ve geri alınanlar, Geçmiş'te modelin adımları); Python `test_processing_batch.py`
  (4) ve `pnpm py:test` (148, ortak durumlar SDK'dan da).
- **Görsel inceleme:** `.run/shots/islem-toplu-*` (altbilgide Toplu çalıştır…, tablo, sonuç ve kayıt, durmuş satır, Geçmiş'te
  model adımları; 1440×900 ve 1100×650, koyu ve açık). Düzeltilenler: durmuş satırın uzun nedeni tabloda satırı kesiyordu
  (kısa metin, neden altta); az sütunlu tablo genişliği doldurmuyordu (yatay kaydırma yalnız dört sütundan sonra).
- **Süre** (release, Apple M5, 20 000 parsel, `cargo test --release -p kentos-processing --test batch batch_timing --
  --ignored --nocapture`):

  | Toplu iş | Toplam | Satır başına |
  |---|---|---|
  | 20 satır, her biri girdi katmanını yazar (`attributes.calculate`) | 3 704 ms | 185 ms (satırın kendi işi 33–66 ms; kalanı katman özeti) |
  | 20 satır, yalnız seçer (`selection.byExpression`) | 544 ms | 27 ms (özet bir kez) |
  | Katmanın özeti tek başına | 162 ms | nesne başına yaklaşık 8 µs |

  Katmanı yazan satırda kaydın özeti satırın işinden 3 kat uzundur: bütçe aşıldı, “sonraya kalan”a yazıldı
  (artımlı ya da daha hızlı özet; özet, bir önceki ADR'nin `layer_digest`'i).
- **Ölçülmeyenler:** masaüstünde 500 satırlık tablonun çizimi; PostGIS bağlı katmanda toplu iş; çok iş parçacıklı satırlar
  (satırlar bilerek sıralıdır).
- **Bilinen, bu işle ilgisiz kırık denetimler** (yolda görüldü, `main`'de de kırık): `history::text::tests::the_titles_are_the_catalog_s`
  (üç komutun başlığı `TITLES`'ta yok: `cad.rasters.georef`, `file.export.csv`, `file.import.csv`) ve
  `processing::tests::the_toolbox_is_a_tab_beside_the_layers…` (araç sayısı 148 yerine 151).
