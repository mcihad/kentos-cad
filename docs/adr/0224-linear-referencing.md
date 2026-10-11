# ADR 0224: Doğrusal referanslama

- **Durum:** kabul edildi (2026-10-11). Kapsamı ben belirledim (sahibin sözü: “Benim seçmeme gerek yok sen sıradan devam et”). İki
  platform (masaüstü ve web); `.kcad` şema 39, biçim sözleşmesi `FORMATS_VERSION` 51; ikonlar sorulmadan seçilir; ilke “Performance
  First”. Madde iki commit'te bitti: 11 Ekim'de sahibin isteğiyle yarıda bırakılıp yapılanı `main`'e WIP olarak gitti (devir notu
  `docs/devir/2026-10-11-gis-28.md`), kalanı (Km yaz ölçüyle, resimler, süreler, bu ADR) ikinci commit'te.
- **Bağlam belgesi:** TODOS.md `GIS-28` (“M değerli rota çizgileri, rotayı kalibre etme, nokta ve çizgi olay tabloları, olayları rota
  üzerinde gösterme ve çakıştırma, nesneleri rota boyunca konumlama; köşede M değeri”); ADR 0142 (köşe kotu: kotun düzenlemelerdeki
  kuralları, `ops::elevation`), ADR 0174 (çok parçalı çizgi), ADR 0189 (Km yaz), ADR 0200 §7 (İşlemler'in dosya parametresi, tablo
  çıktısı), ADR 0223 (tablo dosyaları), ADR 0172 (köşe tablosu), ADR 0156 ve 0158 (oturtma, kauçuk levha), ADR 0046 (Shapefile).
- **Araştırma:** ArcGIS Pro'nun Linear Referencing araç kutusu: Create Routes (Length, One field, Two fields; Coordinate Priority'nin
  dört köşesi, Measure Factor ve Offset, Ignore spatial gaps), Calibrate Routes (arama yarıçapı, Interpolate between, Extrapolate
  before ve after, boşlukları yok sayma), Make Route Event Layer (nokta ve çizgi olayları, öteleme alanı ve yönü), Locate Features Along
  Routes (arama yarıçapı, Keep only the closest route, uzaklık alanı; nokta, çizgi ve alan), Overlay Route Events (Intersect, Union);
  QGIS'in Set M value ve Extract M values'u (ilk, son, en küçük, en büyük); Netcad'in “LRS Nasıl Tanımlanır”ı; Shapefile Technical
  Description'ın PolyLineM ve PolyLineZ'si (M aralığı, −10³⁸'den küçük “no data”).

## Bağlam

Karayolu, demiryolu, boru hattı ve kanal envanterleri yerleri koordinatla değil, bir hat boyunca ölçüyle tutar: “D-100'ün 12+450 –
12+900 kesimi beton”, “13+240'ta okul geçidi levhası, yolun 9 m sağında”, “bu kaza 12+718'de”. KentOS'ta hattın kilometresini yalnız
Km yaz (ADR 0189) biliyordu: çizginin uzunluğundan, bir başlangıç km'siyle. Gerçek rotaların ölçüsü uzunluk değildir: ölçüm, eski
kilometre taşları, kalibrasyon ve atlamalar onu uzunluktan ayırır. Bunun için geometrinin kendisi ölçü taşımalı (köşede M), rotalar
parçalardan kurulup bilinen noktalarla kalibre edilmeli, olay tabloları rotaya yerleştirilmeli, nesneler rota boyunca konumlanmalı ve
iki olay tablosu çakıştırılmalıdır.

## Karar

### 1. M'nin veri modeli

- **Ölçü yalnız çizgide ve çoklu çizgidedir.** Çizgide `ma` (başlangıç) ve `mb` (bitiş), çoklu çizgide ve parçalarında `ms`: köşe başına
  bir sonlu sayı, ölçüsüz köşe `null`, hiçbir köşenin ölçüsü yoksa alan yok (kotun `za`, `zb`, `zs` kuralı, ADR 0142). Alanın, noktanın,
  yayın ve öbür türlerin ölçüsü olmaz: komut da kodek de reddeder. Değer eksi olabilir, köşeden köşeye artması gerekmez (kalibrasyon
  atlaması, ters yönlü rota). Birim rotanındır (km, metre ya da kurumun birimi); dosya söylemez.
- **Rota ayrı bir tür değildir:** ölçülü çizgi ya da çoklu çizgidir. Rota kimliği bir öznitelik alanıdır, araçlarda seçilir.
- **Ölçü yönü:** ilk ve son ölçülü köşeden çıkar: son ölçü ilkinden küçükse rota çizimin tersine akar. Öteleme ve uzaklık ölçü yönünün
  sağına artıdır; araçlarda Sağa ya da Sola seçilir.
- **Kenar boyunca ölçü:** iki ucu ölçülü kenarda doğrusaldır: düz kenarda uzunlukla, yayda açıyla. Bir ucu ölçüsüz kenarın yeri ölçüyle
  bulunmaz.
- **Kodek:** sözleşmenin `LineEntity.ma`, `.mb`, `PathEntity.ms`, `AreaPart.ms`'i ve `EntityGeometry::Line`, `::Polyline`'ın `ms`'i;
  `.kcad` şema 39 (`SCHEMA_WITH_MEASURES`), `FORMATS_VERSION` 51; işçinin sütunlarında çizginin OPT[2] ve OPT[3]'ü, yolun OPT[4]'ü,
  parçanın işareti `PART_MEASURES` (8). Spesifikasyonun “Köşe ölçüleri” bölümü (`docs/specs/kcad-v2.md`) bozuk dosyaların kodlarıyla.
- **Sunucu:** ölçülü nesne, kotlu nesne gibi, `cad_definition`'ını kaynak olarak korur (`geometry_is_source` M'yi Z gibi sayar); PostGIS
  geometrisi türevdir ve M taşımaz.

### 2. Düzenlemelerde ve dönüşümlerde M

- Düzenlemeler M'yi kotun kurallarıyla taşır: çekirdeğin `ops::elevation::carry_elevations`'ı `zs`'in yerinde `ms` ile çalışır. Kır'da
  yeni uçlar kenar boyunca doğrusal, Uzat'ta uzayan uç kenarın artışını sürdürür, Köşe ekle'de yeni köşe kenarının iki ucu arasında,
  tutamaçla taşınan köşe kendi ölçüsünü korur, Yönü çevir'de ölçüler köşeleriyle döner (rota azalan olur).
- Ölçünün taşınamadığı sonuç (alana çevirme, ölçüsü olmayan türe dönüşme) yazılır ve uyarılır: `measure_lost`. Köşe sayısı kadar
  olmayan ölçüler ve alana verilen ölçüler reddedilir: `invalid_measures`. Köşe tablosunda yazılan ölçü yazıldığı gibidir.
- Dönüşümler (taşı, döndür, ölçekle, aynala) M'yi değiştirmez. Vektör oturtma ve kauçuk levha çekirdeği kotlar gibi ölçülerle de yeniden
  çalıştırır: köşeler ölçülerini korur, yeni köşelerin ölçüsü kenarı boyunca doğrusaldır.

### 3. Biçimler

- **Shapefile:** PolyLineM ve PolyLineZ'nin M'si köşelerin ölçüsüdür; −10³⁸'den küçük (şartnamenin “no data”sı) ya da sayı olmayan M
  köşeyi ölçüsüz bırakır. Noktanın ve alanın (PointM, PointZ, MultiPointM, PolygonM, PolygonZ) M'si alınmaz ve söylenir. İçe aktarma
  penceresi “Ölçülü nesne” sayısını verir. Bağımsız okuyucu `tools/formats/gis.py`, örnek `fixtures/formats/v1/gis/rotalarm.*`.
- **Dışa aktarma:** DXF, GeoJSON, KML ve GPX ölçü taşımaz; düşen ölçüler raporda “Ölçü (M)” notuyla söylenir.

### 4. Çekirdek `ops::lrs`

- **`Walk`:** rotanın kenarları yürüme sırasıyla, bir kez kurulur: `locate` (noktanın en yakın yeri, ölçüsü, işaretli uzaklığı, yürünen
  uzunluk), `point_at` (ölçünün ilk yeri, ötelemeyle), `stretch` (iki ölçü arasındaki parçalar; değen parçalar tek yol, ötelemesi çekirdeğin
  yol ötelemesiyle, ters verilen olay ters), `frame_at` (ölçünün yeri, ölçü yönü ve yürünen uzunluk), `locate_line`, `range`, `ends`.
- **Rota oluştur (`create`):** parçalar rota kimliğiyle gruplanır. Uzunluk ve Tek alan'da her zincir, rotanın kutusunun seçilen köşesine
  (Koordinat önceliği) en yakın uçtan başlar; uç uca değen parçalar sırayla eklenir, ters çizilen çevrilir, değmeyenler rotanın ayrı
  parçalarıdır ve aralarındaki boşluk istenirse ölçüye katılır. Ölçü uzunluktan ya da parçanın alanındaki uzunluktan gelir, çarpanla
  çarpılıp başlangıca eklenir. İki alan'da her parça kendi başlangıç ve bitiş ölçüsüyle, başlangıcına göre sırada. Yaylar yay kalır.
- **Kalibrasyon (`calibrate`):** rotanın noktaları arama yarıçapındaysa en yakın yerine konur, en az iki gerekir; çoklu çizgide noktaların
  yerine köşe eklenir (`split`), çizgi iki köşesini korur. Her köşenin ölçüsü çevresindeki iki noktanın arasında yürünen uzunlukla
  doğrusaldır; ilk noktadan önce ve sonuncudan sonra istenirse uç aralıkların oranıyla uzatılır, istenmezse eski ölçü kalır; boşluklar
  istenirse yürünen uzunluğa katılır.
- **Olaylar (`events_run`):** nokta olayı ölçünün yerine, çizgi olayı iki ölçü arasındaki parçalar olarak; öteleme ölçü yönünün sağına
  artı. Rotası olmayan (`noRoute`), ölçüsü olmayan (`noMeasure`) ya da rotanın ölçüleri dışında kalan (`outside`) olay yazılmaz.
- **Konumlama (`locate_run`):** rotaların bütün kenarları tek bir paketlenmiş R-ağacında (`RouteIndex`); nokta yarıçap içindeki rotaların
  en yakın yerlerine, çizgi bütün köşeleri yarıçap içindeyse ilk ve son köşesinin ölçüleriyle, alan içinden geçen rota parçalarıyla
  (halkalarla kesilen kenarların ortası içerideyse) konumlanır; yalnız en yakın rota ya da hepsi.
- **Çakıştırma (`overlay`):** iki tablonun çizgi olayları bir olayın başladığı ya da bittiği her ölçüde bölünür (tarama doğrusu): Kesişim
  ikisinin de kapladığı, Birleşim herhangi birinin kapladığı parçalar; bir parçayı kaplayan her olay çifti bir satır, aynı çiftin komşu
  parçaları tek satır.
- **Özet (`summary`):** ilk, son, en küçük ve en büyük ölçü ve ölçülü köşe sayısı.
- **JSON işlemleri** (`api/tables.rs`): `lrsCreate`, `lrsCalibrate`, `lrsEvents`, `lrsLocate`, `lrsOverlay`, `lrsSummaries`; web'in
  sarmalayıcıları `model/ops/lrs.ts`, masaüstünün araçları çekirdeğin işlevlerini doğrudan çağırır. Bir çalıştırma bir çağrıdır.
- **Performans ve bütçeler** (release, bu makine; ölçü sahnesi iki platformda bit bit aynı): 1 000 rota (1 km, 100 kenar, her onuncusu
  yay), 100 000 nokta ve 10 000 çizgi olayı, konumlanacak 100 000 nokta, 100 rotanın karışık sıradaki 10 000 parçası, 50 000 ve 50 000 olay,
  10 000 kalibrasyon noktası. Bütçeler işin gereğinden konur: bir araç çalıştırması 100 000 kayıtta masaüstünde 0,1–0,3 s'yi, web'de
  bunun 2,5 katını geçmez. Olayları rotada göster (100 000 nokta) ≤ 100 ms, (10 000 çizgi) ≤ 100 ms; Rota boyunca konumla (100 000 nokta)
  ≤ 300 ms; Rota oluştur (10 000 parça) ≤ 20 ms; Olayları çakıştır (50 000 + 50 000) ≤ 150 ms; Rotayı kalibre et (1 000 rota, 10 000 nokta)
  ≤ 100 ms. Web'de aynı işler, işlemin JSON'u dahil, ≤ 2,5 katı.

### 5. İşlemler araçları

İşlemler'e iki platformda yeni kategori **Doğrusal referans** (`linearRef`, ikon `linearRef`) ve altı araç:

| Araç | Kimlik | Ne yapar | Çıktı | İkon |
|---|---|---|---|---|
| Rota oluştur | `linearRef.createRoutes` | Parçaları kimlik alanıyla rotalara birleştirir: Uzunluk, Tek alan ya da İki alan; Koordinat önceliği, çarpan, başlangıç, boşlukları yok sayma | yeni katman (Rotalar), atlananlar tablosu | `routeCreate` |
| Rotayı kalibre et | `linearRef.calibrateRoutes` | Ölçüsü bilinen noktalarla rotaların ölçülerini yeniden verir: arama yarıçapı, önce ve sonra uzatma, boşluklar | rotalar yerinde, tek adım; rota tablosu | `routeCalibrate` |
| Olayları rotada göster | `linearRef.routeEvents` | Nokta ya da çizgi olaylarını (dosya ya da nesnelerin öznitelikleri) rotaya yerleştirir; öteleme sütunu, Sağa ya da Sola | yeni katman (Olaylar), satırın hücreleri öznitelik; atlanan satırlar tablosu | `routeEvents` |
| Rota boyunca konumla | `linearRef.locateAlong` | Nokta, çizgi ve alanları rotalar boyunca konumlar: arama yarıçapı, yalnız en yakını ya da hepsi; Özniteliklere yaz | tablo (Rota, Ölçü ya da Başlangıç ve Bitiş, Uzaklık); isteğe bağlı öznitelikler | `routeLocate` |
| Olayları çakıştır | `linearRef.overlayEvents` | İki çizgi olay tablosunu çakıştırır: Kesişim ya da Birleşim | tablo (Kaydet ile XLSX, ODS, CSV) | `routeOverlay` |
| M değerlerini çıkar | `linearRef.extractMeasures` | Rotanın ilk, son, en küçük ve en büyük ölçüsünü önekli alanlara yazar | öznitelikler, tek adım | `routeMeasures` |

Olay tablosu bir dosya (CSV, TXT, XLSX, ODS; ilk satırı sütun adları, ADR 0223) ya da nesnelerdir. Hücrenin sayısı ADR 0180'in
kuralıyla okunur (virgüllü ondalık da). Sayılar projenin uzunluk basamağıyla. CBS şeridinde altısı Analiz › Ağ analizi ▾'nin altındadır
(web `app/ribbon.ts`'in `LRS_UNDER`'ı, masaüstüne envanterle): kendi paneli sekmeyi 1100 px'ten genişletiyordu.

### 6. Arayüz

- **Öznitelikler:** çizgide “M (başlangıç)” ve “M (bitiş)”, çoklu çizgide “M” (ilk → son; ölçüsüz köşe varsa not); salt okunur.
- **Köşe tablosu (Koordinat listesi):** “M (ölçü)” sütunu, düzenlenir (ADR 0172'nin kuralları; `vertex_edit_cases.py`).
- **Üzerine gelme kartı:** “M” (ilk → son).
- **Km yaz ölçüyle** (ADR 0189'un eki): seçilen hattın ölçüsü varsa aracın istemine **Ölçü (M)** gelir ve hat alınınca açıktır; açıkken
  km'ler rotanın ölçüleridir (`ops::stationing::measure_stations`): aralığın en küçük ile en büyük ölçü arasındaki katları ve istenirse
  yürüme sırasındaki ilk ve son ölçü, her biri o ölçünün yürüme sırasındaki ilk yerinde, rota orada ölçü yönünde akarak; yürünen uzunluğa
  göre sıralı, 10⁻⁹ içinde iki istasyon bir sayılır (katınki kalır). Ölçüler başlangıcı ve yönü belirlediği için Başlangıç (B) ve Ters
  (T) o sırada sunulmaz. Ölçüsüz hatta Ölçü yoktur, araç eskisi gibi uzunlukla çalışır. Hattın rotası iki platformda tek yerden gelir
  (web `model/ops/lrs.ts`'in `routeOf`'u, masaüstü `kentos_native_application::geometry::route`); İşlemler'in araçları da onu kullanır.
- **Masaüstünün komut satırı** (KentOS UI): seçeneğin tuşu yazılınca öneri listesinin başına o seçenek gelir. Önceden liste seçenekleri
  yalnız ad ve değerlerindeki sözcüklerle eşliyordu; Km yaz'da “M” yazıp Enter'a basmak, değeri “20 m” olan Aralık'ı seçiyordu. Web'de
  araç çalışırken yazılan zaten aracın girdisidir; iki platform artık aynı davranır.

## Uygulama

- **Sözleşme ve kodek:** `crates/shared/contracts/src/entity.rs`, `cad_edit.rs`; üretilen TS ve `commandCatalog.json`;
  `crates/shared/kcad` (`SCHEMA_WITH_MEASURES`, `encode/objects.rs`, `decode/objects.rs`, `columns.rs`), web `io/columns.ts`, `io/kcad.ts`;
  `docs/specs/kcad-v2.md`; bağımsız Python tarafı `tools/kcad/kcad.py`, `scripts/fixtures/kcad_v2_reference.py`; örnekler
  `fixtures/kcad/v2/measures.{json,kcad}` ve beş bozuk ölçü dosyası (`broken/schema-version-39.kcad` yerine `-40`); sunucu
  `crates/server/application/src/cad.rs`.
- **Düzenlemeler:** masaüstü `crates/native/application` (`geometry.rs`, `edit.rs`'in `written_measures` ve `carry_measures`'ı,
  `elevation.rs`'in `measure_paths`, `carry_measures`, `assign_measures`'ı, `transform.rs`, `codes.rs`'in `MEASURE_LOST` ve
  `INVALID_MEASURES`'ı); web `product/elevation.ts`, `product/entitiesEdit.ts`, `product/entitiesTransform.ts`, `model/ops/transform.ts`,
  `model/snapshot.ts`, `model/entities.ts`.
- **Biçimler:** `crates/shared/formats/src/shp/shape.rs`, `shp/mod.rs`, `gis.rs`, `import.rs` (“Ölçülü nesne”); dışa aktarmanın notu
  `export_geom.rs`, `kml/write.rs`, `gnss/gpx_write.rs`, `dxf/writer/entities.rs`.
- **Çekirdek:** `crates/shared/geometry-core/src/ops/lrs/` (`mod`, `create`, `calibrate`, `overlay`, `area`, `index`, `calls`);
  `ops/stationing.rs`'in `measure_stations`, `measure_stationing`'i, `NO_MEASURES` ve `stationingMeasure` işlemi.
- **Araçlar:** web `processing/builtin/lrs/` (`shared.ts`, `routes.ts`, `events.ts`, `locate.ts`, `index.ts`), `builtin/index.ts`,
  `categories.ts`; masaüstü `crates/native/processing/src/builtin/lrs/` (`mod.rs`, `routes.rs`, `events.rs`, `locate.rs`), `builtin/mod.rs`,
  `categories.rs` (29 kategori); `apps/desktop/src/catalog.rs` ve `ported.json`; ikonlar `apps/web/src/ui/icons.ts` (masaüstüne
  envanterle).
- **Arayüz:** masaüstü `properties/rows.rs` (`measure_rows`), `vertices/` (`edit.rs`, `mod.rs`, `view.rs`), `hover_card.rs`,
  `kentos_interaction::station_labels` (Ölçü), `kentos_native_application::geometry::route`; web `ui/properties/elevationRows.ts`
  (`measureRows`), `ui/bottom/vertexEdit.ts`, `VertexTable.ts`, `ui/shell/HoverCard.ts`, `tools/stationLabelTool.ts`,
  `tools/constructions.ts`'in `stationingMeasure`'ı, `model/ops/lrs.ts`'in `routeOf` ve `measured`'ı; KentOS UI
  `widget/command_line.rs` (`keyed`).
- **Bağımsız başvurular:** `scripts/fixtures/lrs_cases.py` (`fixtures/lrs/v1/cases.json`), `lrs_processing_cases.py`
  (`fixtures/processing/v1/lrs.json`, `lrs.kcad`, `lrs-noktalar.csv`, `lrs-cizgiler.csv`, `lrs-hizlar.csv`), `stationing_cases.py`'nin
  `measureCases`'i, `edit_command_cases.py`'nin M bölümü, `transform_command_cases.py`, `vertex_edit_cases.py`, `gis_reference.py`,
  `kcad_v2_reference.py`.
- **Ortak iz ve sahneler:** `fixtures/interaction/v1/stationing-measure.json` ve `.kcad`; resimlerin yolu
  `fixtures/interaction/v1/lrs.kcad` ve `lrs/` (`kaplama.csv`, `hiz.csv`, `korkuluk.csv`, `levhalar.csv`), KentOS kodu olmadan
  `scripts/fixtures/lrs_scene.py` yazar.
- **Sınamalar:** çekirdek `tests/all/lrs.rs` (başvurunun durumları, R-ağacı kaba kuvvetle), `tests/all/stationing.rs` (ölçüyle
  durumlar); web `model/ops/lrs.wasm.test.ts`, `tools/stationing.wasm.test.ts`, `processing/cases.test.ts` (sayfada ve işçide);
  masaüstü `kentos-processing`'in `tests/cases.rs`'i (`the_linear_referencing_cases_do_what_they_say`),
  `apps/desktop/src/processing/lrs_tests.rs` (her ortak durum pencereden, dosya diyalogun okuduğu gibi seçilerek); KentOS UI'ın
  `a_typed_key_chooses_its_option_before_a_value_that_starts_so`'su.
- **Resimler:** masaüstü `processing::lrs_tests::screens` (`islem-dr-*`) ve `tools_screens`'in `km-yaz-olcu`'su
  (`stationing_scenes.rs`), web `shots.mjs lrs` (`dr-*`) ve `shots.mjs stationing --only stationing-measure`.
- **Süre ölçümleri:** `cargo test --release -p kentos-geometry-core --test all lrs::timing -- --ignored --nocapture`, web
  `apps/web/scripts/perf/lrs.test.ts` (`LRS_BENCH=1`, aynı sahne SplitMix64'le bit bit).

## Doğrulama

11 Ekim 2026, geliştirme makinesinde (13. nesil Intel Core i5-13500, 20 iş parçacığı, Linux; derleme 4 işle):

- **Bağımsız başvurular** (KentOS kodu olmadan, `--check` ile geçer): `lrs_cases.py` (59 durum: 9 konum, 10 ölçünün yeri, 9 parça, 8 rota
  oluşturma, 6 kalibrasyon, 2 çakıştırma, 5 özet, 3 çizgi ve 7 alan konumu; 50 basamakta), `lrs_processing_cases.py` (14 durum),
  `stationing_cases.py` (13 uzunlukla ve 4 ölçüyle durum), `edit_command_cases.py` (113 durumun 8'i M), `transform_command_cases.py` (56),
  `vertex_edit_cases.py` (58), `gis_reference.py` (`rotalarm` ve `yollarz`'ın M'si), `kcad_v2_reference.py` (ölçülü dosya ve beş bozuk).
- **Platformlar:** başvurunun durumları çekirdekte ve WASM'da aynı; İşlemler'in 14 durumu web'de sayfada ve işçide, masaüstünde doğrudan
  ve penceresinden (dosyalar diyalogun okuduğu gibi) aynı sonuçları verir; Km yaz'ın ölçüyle dört durumu çekirdekte ve WASM'da başvuruyla
  aynı. Ortak iz `stationing-measure.json` (30 adım) iki platformda üç varyantta geçer: Ölçü'nün görünmesi ve açık gelmesi, M'den km
  (1+005.000 … 1+062.000), Başlangıç ve Ters'in gizlenmesi, Ölçü kapalıyken uzunluk, tek adımda yazma, ölçüsüz çizgi, seçili rotanın başta
  alınması. İz önce masaüstünde 11. adımda düştü: komut satırı “M”yi Aralık'ın “20 m”siyle eşliyordu (§6'nın komut satırı düzeltmesi).
- **Takım:** `pnpm typecheck`, Vitest (5 198 test; 37'si yalnız süre ölçümünde koşar), `rust:test` (3 208), `rust:test:desktop`
  (1 307 ve clippy), `kentos-mcp`, `py:test`, `build` (çekirdek WASM 5 512,11 kB, gzip 1 974,27 kB), `inventory:check`, `e2e` (210
  denetim), `e2e:interaction` (143 iz × 3 varyant), `e2e:layout` (564 görünüm). Masaüstü takımı bir kez düştü: Shapefile içe
  aktarmanın satırı WIP'in eklediği “Ölçülü nesne”yi de söylüyordu, test beklemiyordu (test düzeldi, ölçüsüz dosyada bilginin
  olmadığı da sınanır). Düzen denetimi Model tasarımcısının paletinde GIS-25'in uzun araç adını (“Ortak kenarları koruyarak
  sadeleştir”) kesik buldu; satırın zengin ipucu adı bütün gösteriyordu, denetim yalnız `title`'ı tanıyordu: ipucu yardımcısı
  (`ui/widgets/tooltip.ts`) hedefini `data-tooltip` ile işaretler, denetim onu da sayar.
- **Resimler** iki platformda 1440×900 ve 1100×650'de, iki temada: masaüstü `islem-dr-*` (şeridin Ağ analizi ▾ listesi, altı aracın
  pencereleri, yeni rotalar ve yeni D-100'ün M'si Öznitelikler'de, korkuluklar ve levhalar rotaların yanında, Rota boyunca konumla'nın ve
  Olayları çakıştır'ın tabloları, M değerlerini çıkar, D-100'ün Öznitelikler'deki M'si, Koordinat listesindeki M sütunu ve kartı) ve
  `arac-km-yaz-olcu-*`; web `dr-*` ve `stationing-measure-*`. İlk resimler ortak durum çiziminde (`fixtures/processing/v1/lrs.kcad`)
  çekildi; dağınık ve küçük olduğu için resimlere kendi yolu yazıldı (`lrs_scene.py`).
- **Süreler** (release; çekirdek `lrs::timing`'in beş koşudan en kısası, web `scripts/perf/lrs.test.ts`'in gönderilen WASM'la 10 koşusunun
  p50'si; iki platformda aynı sayılar: 81 391 konum, 193 469 parça):

  | İş | Masaüstü | Bütçe | Web | Web bütçesi |
  |---|---|---|---|---|
  | Olayları rotada göster, 100 000 nokta, 1 000 rota | 18,9 ms | 100 | 142,2 ms | 250 |
  | Olayları rotada göster, 10 000 çizgi, ötelemeli | 18,3 ms | 100 | 96,1 ms | 250 |
  | Rota boyunca konumla, 100 000 nokta, 20 m | 130,7 ms | 300 | 273,5 ms | 750 |
  | Rota oluştur, 10 000 parçadan 100 rota | 2,6 ms | 20 | 12,6 ms | 50 |
  | Olayları çakıştır, 50 000 ve 50 000 olay, birleşim | 46,6 ms | 150 | 236,2 ms | 375 |
  | Rotayı kalibre et, 1 000 rota, 10 000 nokta | 28,9 ms | 100 | 106,1 ms | 250 |

  Web'de sürenin çoğu işlemin JSON'udur (100 000 olay girer, 100 000 nokta çıkar): çekirdek 19 ms, web 142 ms. İlk bütçeler masaüstünde
  ölçümün iki katıydı (nokta olayları 40 ms) ve web'in 2,5 katı (100 ms) JSON yüzünden aşıldı; bütçeler bu yüzden ölçüme değil işin
  gereğine bağlandı (§4). Gerekirse olaylar tipli dizilerle geçirilebilir; şimdi gerekmiyor.
