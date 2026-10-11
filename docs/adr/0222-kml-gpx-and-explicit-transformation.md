# ADR 0222: KML/KMZ, GPX/NMEA ve içe aktarmada açık dönüşüm

- **Durum:** kabul edildi (2026-10-11). Kapsamı ben belirledim (sahibin sözü: “Benim seçmeme gerek yok sen sıradan devam et”). İki
  platform (masaüstü ve web); `.kcad` şeması değişmez, biçim sözleşmesi `FORMATS_VERSION` 49; ikonlar sorulmadan seçilir; ilke
  “Performance First”. Madde tek parçada biter.
- **Bağlam belgesi:** TODOS.md `GIS-26` (“WGS84 dışı projede dönüşüm açıkça sorulur”); ADR 0169 §1, §6 (GPX ve NMEA okuyucuları, GNSS
  içe aktar), ADR 0167 ve 0168 (koordinat dönüşümü, doğruluk metni, projenin datum seçimleri), ADR 0046 ve 0053 (GeoJSON ve Shapefile:
  içe aktarma yolu ve “dönüşüm yoktur” kuralı), ADR 0143 ve 0174 (çok parçalı nesneler), ADR 0144 §5 (blokların açılması), ADR 0142
  (kotlar), ADR 0199 §7 (Kaynaklar).
- **Araştırma:** OGC KML 2.2 (07-147r2) ve Google'ın `gx` uzantıları (Track, MultiTrack), KMZ'nin `doc.kml` kuralı; GPX 1.1 şeması
  (öğelerin sırası: `metadata`, `wpt`, `rte`, `trk`; `wpt`'nin `ele`, `name`, `desc`, `type` sırası); RFC 7946 (WGS 84 boylam ve enlem,
  `crs` üyesi yok, sağ el kuralı, üçüncü sayı elipsoitten yükseklik); GDAL'ın LIBKML, KML ve GPX sürücüleri; ArcGIS'in KML To Layer,
  Layer To KML, GPX To Features, Features To GPX'i; QGIS'in GPS araçları ve GPX katmanı; Netcad'in KML Gönder'i ve GPS Operatörleri.

## Bağlam

KML ve KMZ Google Earth'ün, GPX el GPS'lerinin ve gezi uygulamalarının, NMEA alıcıların günlüğüdür; üçü de WGS 84 boylam ve enlemidir.
KentOS GPX ve NMEA'nın konumlarını GNSS içe aktar ile nokta olarak alıyordu (ADR 0169 §6); izleri ve rotaları çizgi olarak almıyor,
KML'yi okumuyor, ikisini de yazmıyordu. GeoJSON ve Shapefile içe aktarma projeninkinden başka sistemi reddediyor, pencereler
“koordinat dönüşümü henüz yok” diyordu. Dönüşüm artık var (ADR 0167, 0168); sessizce yapılmaz (CLAUDE.md §5) ama açıkça sorulabilir.

## Karar

### 1. Kapsam

1. **İçe aktarmada açık dönüşüm** (§2): GeoJSON, Shapefile ve KML/KMZ penceresinde dosyanın sistemi projeninkinden başkaysa pencere
   dönüşümün yolunu ve doğruluğunu söyler, “Projenin sistemine çevir” onayını ister; onaysız içe aktarma kapalı kalır.
2. **KML/KMZ içe aktar** (`file.import.kml`, §3): klasörler katman, yer imleri nesne; biçemleri, adları, açıklamaları, zamanları ve
   verileri.
3. **KML/KMZ dışa aktar** (`file.export.kml`, §4): katmanlar klasör, nesneler yer imi; renkleri, kalınlıkları, öznitelikleri; WGS 84'e.
4. **GPX dışa aktar** (`file.export.gpx`, §5): noktalar yol noktası, çizgiler iz ya da rota, alanların sınırları kapalı iz; WGS 84'e.
5. **GNSS içe aktar'a çizgiler** (§6): GPX'in rotaları ve izleri (parçalarıyla), NMEA günlüğünün izi çoklu çizgi olarak.
6. **GeoJSON dışa aktar'a WGS 84** (§7): “WGS 84'e çevir (RFC 7946)” seçeneği.
7. **Kaynaklar** (§8): `.kml` ve `.kmz` dosyaları KML/KMZ içe aktar penceresine.

Hesap ortak Rust çekirdeğindedir (`kentos-formats`; dönüşüm geometri çekirdeğinin `crs::transform_in`'i); web onu biçim işçisinde
WASM'la, masaüstü kendi iş parçacığında çalıştırır.

### 2. İçe aktarmada açık dönüşüm

Koordinat sistemi sorusu (masaüstünde `crs::CrsQuestion`, web'de `ui/io/common.ts`'in `CrsQuestion`'ı) GeoJSON, Shapefile ve KML/KMZ
pencerelerinde **çevrilebilir**dir. Seçilen sistem projeninkinden başkaysa, proje yerel değilse ve iki sistem de dönüşümlerce
okunabiliyorsa (kayıttaki sistem ya da projenin tanımı, ADR 0168 §1):

- Pencere koordinatların kapsamının ortasındaki noktayı projenin datum seçimleriyle çevirir (`transform_in`) ve yolu Koordinat
  dönüştür'ün metniyle söyler: “Koordinatlar WGS 84 sisteminde, proje TUREF / TM36 sisteminde: içe aktarmak için çevrilmeleri gerekir
  (dönüşüm: ±1 m, EPSG:5261). Koordinatlar sessizce çevrilmez; çevirmek için “Projenin sistemine çevir”i işaretleyin.” Ortası
  ulaşılamıyorsa nedeniyle uyarır ve içe aktarma kapalı kalır.
- Altında **Projenin sistemine çevir** kutusu. İşaretlenince nesneler arka planda (masaüstünde iş parçacığında, web'de biçim
  işçisinde) çevrilir; pencerenin katmanları, sayıları, kapsamı ve raporu çevrilmiş nesnelerindir; “Koordinatlar projenin sistemine
  çevrildi” satırı çıkar. Resmî olmayan yol (ED50'nin EPSG dönüşümü) uyarı satırıdır.
- Çevirme (`kentos_formats::reproject`) her köşeyi Koordinat dönüştür'ün yoluyla taşır; kotlar olduğu gibi kalır (dönüşüm 2B). Bu
  okuyucular yalnız nokta, çizgi ve düz kenarlı yol verir; başka tür ve bir köşesi ulaşılmayan nesne alınmaz, nedeniyle söylenir
  (“Çevrilemeyen nesne”, “Eğrili ya da başka türden nesne”) ve rapordaki sayılardan düşer. Katmanların kutuları ve görünüm yeniden
  hesaplanır; nesnesi kalmayan katman açılmaz. Yol, doğruluk ve resmî olmayışı sonuçla birlikte döner.
- Seçim değişirse (başka sistem) çevirme o sistemden yeniden yapılır; aynı sistem için bir kez. Sürmekte olan çevirme okumanın ve
  kaynak sistemin adıyla tutulur: başka sistem seçilince pencere onu beklemez, sonucu yalnız kendi sistemi seçiliyse kullanılır (A'dan
  B'ye geçip A'ya dönünce A'nın çevrilmiş nesneleri hemen alınır).

DXF, NCZ ve koordinat listesi pencereleri çevirmez; başka sistemdeki dosya için “bu dosya türü içe aktarılırken çevrilmez (GeoJSON,
Shapefile ve KML çevrilebilir)” der. Eski “dönüşüm henüz yok” metinleri kalktı.

### 3. KML/KMZ okuma

`kentos_formats::kml::read` (roxmltree, derinlik koruması `xml`):

1. KMZ (zip) içinden `doc.kml`, yoksa kökteki ilk `.kml`, yoksa herhangi bir `.kml`; hiçbiri yoksa “KMZ arşivinde .kml dosyası yok”.
   XML olmayan, 256 düzeyden derin ya da kökü `kml` olmayan metin okunmaz, nedeni söylenir.
2. **Klasörler katman:** her Folder adını (yoksa “Adsız klasör”, söylenir) içindekilerin yoluna ekler; Document eklemez. Yer iminin
   katmanı yolunun “ / ” ile birleşimidir (“Yollar / Ana yollar”); klasör dışındakiler ilk Document'ın adına, o da yoksa “KML”ye.
   Görünürlüğü 0 olan klasörün (ya da altındakinin) katmanı gizli açılır.
3. **Geometri:** Point (bir konum), LineString (iki ya da daha çok), LinearRing (üç ya da daha çok; yazıldığı gibi kapalı yol), Polygon
   (dış halka ve delikler; tekrarlanan kapanış konumu düşer, art arda aynılar bir sayılınca üçten az köşesi kalan halka alınmaz),
   MultiGeometry (üyeleri), gx:Track (`gx:coord` konumları, iki ya da daha çok), gx:MultiTrack. Yer imi verdiği her türden bir nesne
   olur, sırayla noktalar, yollar, alanlar: nokta ya da çok noktalı nesne; çizgi, çoklu çizgi ya da çok parçalı çoklu çizgi; alan ya da
   çok parçalı alan (ADR 0143, 0174). Karışık MultiGeometry söylenir.
4. **Koordinatlar** boşlukla ayrılmış `boylam,enlem[,yükseklik]`; virgülle başlayan ya da virgülle biten sözcükten sonra gelen sözcük
   ona eklenir (bazı yazıcıların “32.82 ,39.92”si). Sayı ondalıktır (işaret, rakamlar, nokta, üs), en yakın float64'e bir kez
   yuvarlanır; sonlu, |boylam| ≤ 180, |enlem| ≤ 90. Okunmayan geometri yer iminin satırıyla söylenir (dış halka Polygon'unki, delik
   LinearRing'inki). Yükseklik ancak `altitudeMode` absolute ise (deniz seviyesinden) köşenin kotu olur; değilse yer imi başına bir kez
   söylenir.
5. **Öznitelikler:** Ad (name), Açıklama (description), Zaman (TimeStamp/when), Başlangıç ve Bitiş (TimeSpan), ExtendedData'nın Data
   (value) ve SimpleData'ları adlarıyla; metin karakter verisidir (CDATA da), kırpılır.
6. **Biçem:** yer iminin kendi Style'ı, yoksa styleUrl'ün dosyadaki `#id`'si (Style, ya da StyleMap'in normal çiftinin Style'ı ya da
   styleUrl'ü); başka dosyaya bağlı biçem söylenir. Noktanın rengi IconStyle'ınki (yoksa LineStyle'ınki), yolun LineStyle'ınki, alanın
   kenarlıklıysa LineStyle'ınki (yoksa PolyStyle'ınki), değilse PolyStyle'ınki. `aabbggrr` → `#rrggbb`. Kalınlık LineStyle'ın
   genişliği (piksel, 25,4/96 mm) en yakın standart kalem (0,13 … 2 mm). Katmanın rengi nesnelerinin en sık rengi.
7. NetworkLink, GroundOverlay, ScreenOverlay, PhotoOverlay, gx:Tour ve Model sayılır, alınmaz, söylenir.
8. Dosyanın dediği sistem WGS 84'tür (`CrsSource::Kml`, EPSG:4326); pencere bunu söyler ve §2'nin onayını ister.

### 4. KML/KMZ yazma

`kentos_formats::kml::write(input, from, choices)`: nesneler projenin sisteminden (`from`) WGS 84'e projenin datum seçimleriyle çevrilir.

- **Klasörler:** her katman bir Folder, grup yolu dıştan içe Folder'lar; katmanlar girdinin sırasıyla, listede olmayan katmanın
  nesneleri ardından katman kimliğinin adıyla.
- **Yer imi:** adı yazının metni (yazı adlı nokta olur), değilse etiketi, o da yoksa Ad özniteliği; öznitelikler ExtendedData'nın
  Data'ları; geometrisi GeoJSON'unkiyle aynı kuralla (`export_geom`: eğriler turda 72 adım örneklenir, tarama alanı, halkalar dış sınır
  saat yönünün tersine ve delikler saat yönünde, blok yerleştirmesi bloğunun nesneleriyle MultiGeometry).
- **Biçemler:** paylaşılan `s1`, `s2` … ilk kullanıldıkları sırayla: LineStyle nesnenin rengi (yoksa katmanınki, o da yoksa siyah) ve
  genişliği (nesnenin kalınlığı, yoksa katmanınki, yoksa 0,25 mm; pikselde iki basamak, standart kalemler geri okununca kendileri;
  en ince çizgi 0 bir piksel); alanda PolyStyle aynı renk beşte iki saydam; noktada IconStyle.
- Bütün köşeleri kotlu geometri kotlarıyla ve `altitudeMode` absolute; öbürleri kotsuz (kısmen kotlu olan söylenir). Boylam ve enlem on
  basamak (yaklaşık 0,01 mm), kotlar olduğu gibi (en kısa ondalık).
- Ölçü, tablo, resim, raster, nokta bulutu, sonsuz doğru ve ışın yazılmaz, söylenir; bir köşesi WGS 84'e çevrilemeyen nesne yazılmaz,
  nedeniyle söylenir.
- **KMZ:** KML `doc.kml` olarak zip arşivinde (`zip::write`: sıkıştırılınca küçülürse Deflate, UTF-8 adı, sabit tarih; aynı girdi aynı
  baytlar).

### 5. GPX yazma

`kentos_formats::gnss::gpx_write::write`: GPX 1.1 (`creator="KentOS CAD"`), metadata adı çizimin adı; şemanın sırasıyla yol noktaları,
rotalar, izler. Nokta (çok noktalının her noktası) yol noktası, yazı metniyle adlı yol noktası; çizgiler, çoklu çizginin yolları ve
örneklenmiş eğriler istenirse yol başına bir rota, değilse nesne başına parçalı bir iz; alanların sınırı ve delikleri (kapalı, sağ el
kuralıyla) her zaman iz parçalarıdır (GPX'te alan yok). Ad etiket ya da Ad, açıklama Açıklama, tür Kod özniteliği; öbür öznitelikler
yazılmaz, söylenir. Köşenin kotu `ele`'dir, olduğu gibi. Enlem ve boylam on basamak.

### 6. GNSS içe aktar'da çizgiler

GPX okuyucusu rotaları (`rte`, bir parça) ve izleri (`trk`, `trkseg` başına parça), NMEA okuyucusu günlüğün bütün konumlarını (iki ya da
daha çoksa bir parça) `GnssRead.lines` olarak verir; parça noktaların `points` içindeki yerleridir, okunmayan nokta yoktur, boş parça ve
parçasız çizgi listelenmez. GNSS içe aktar'da **Çizgiler** kutusu (varsayılan açık): her çizgi noktalar gibi çevrilen noktalarından bir
çoklu çizgi olur, nokta türlerinin seçiminden bağımsız; iki noktalı parçası olmayan ya da bir noktası ulaşılmayan çizgi alınmaz,
söylenir. Etiketi ve Ad'ı adı; Tür “GNSS izi” ya da “GNSS rotası”, Kaynak “GPX izi”, “GPX rotası” ya da “NMEA kaydı”, Nokta, Dönüşüm;
köşelerin kotu noktaların elipsoit yüksekliği.

### 7. GeoJSON'u WGS 84'e çevir

GeoJSON dışa aktar'da projenin sistemi okunabiliyor ve WGS 84 değilse **WGS 84'e çevir (RFC 7946)** kutusu: işaretlenince
(`geojson::write_wgs84`) koordinatlar WGS 84 boylam ve enlemine çevrilip on basamağa yuvarlanır, `crs` üyesi yazılmaz; ulaşılmayan
nesne yazılmaz, söylenir. Kotlar olduğu gibi (RFC 7946 elipsoitten yükseklik ister; KentOS kotu çevirmez, rapor söyler).

### 8. Kaynaklar, komutlar ve simgeler

- Kaynaklar `.kml` ve `.kmz`'yi KML/KMZ türüyle listeler, KML/KMZ içe aktar penceresine açar (ortak durumlar `fixtures/sources/v1`).
- Komutlar `file.import.kml` (KML, KMZ, KMLAL, KMZAL, GOOGLEEARTH), `file.export.kml` (KMLVER, KMLYAZ, KMZVER, KMZYAZ), `file.export.gpx`
  (GPXVER, GPXYAZ); şeridin İçe aktar ve Dışa aktar listelerinde, uygulama menüsünde ve Dosya menüsünde.
- Simgeler (sorulmadan seçildi): `importKml` ve `exportKml` dosya sayfasında ekvatoru ve bir meridyeniyle küre, `exportGpx` uçları
  noktalı kırık iz çizgisi.

### 9. Başarım

Okuma ve yazma tek geçiştir: KML metni tek `String`e yazılır, biçemler küçük bir listede, katmanların nesneleri sıralı listelerde;
dönüşüm köşe başına bir çağrıdır; okuyucunun söylediği satırlar bir kez kurulan satır dizininden ikili aramayla bulunur (`xml::Rows`).
Masaüstünde iş parçacığında, web'de biçim işçisinde çalışır; çevirme penceresi arayüzü bekletmez.

## Uygulama

- **Çekirdek** `kentos-formats`: `kml/read.rs`, `kml/style.rs`, `kml/write.rs`, `gnss/gpx_write.rs`, `reproject.rs`, `export_geom.rs`
  (GeoJSON yazıcısından ayrılan ortak geometri: `Words` biçimin sözleri, `Geometry::map`), `zip.rs`'in `write`'ı, `num.rs`'in `decimals`
  ve `rounded`'ı, `xml.rs`'in `Rows`'u (GPX ve JobXML okuyucuları da), `geojson::write_wgs84`; `gnss/gpx.rs` ve `gnss/nmea.rs`
  çizgileri verir. Sözleşme `GnssLine`, `KmlLayer`,
  `KmlWriteInput`, `GpxWriteInput`, `CrsSource::Kml`.
- **WASM** `formats-wasm`: `readKml`, `writeKml`, `writeGpx`, `writeGeoJsonWgs84`, `reprojectImport`.
- **Masaüstü:** `crs.rs`'in `CrsQuestion::convertible`, `move_between`, `way`, `ready`, `convert_view` ve `Project`'in sistemi ve
  seçimleri; `exchange/gis_import.rs` (KML kaynağı, çevirme), `exchange/wgs84_export.rs` (KML/KMZ ve GPX penceresi),
  `exchange/geojson_export.rs` (WGS 84), `exchange/gnss_import.rs` ve `kentos_interaction::gnss`'in `place_all`, `place_lines`,
  `line_entities`'i; `kentos_interaction::sources`'un `Kml`'si; komutlar `catalog.rs` ve `ported.json`'da; testler ve resimler
  `exchange/kml_tests.rs`.
- **Web:** `ui/io/common.ts`'in `CrsQuestion`'ı (`convertible`, `moveBetween`, `way`, `ready`), `GisImportDialog.ts` (KML, çevirme),
  `Wgs84ExportDialog.ts`, `wgs84.ts`, `GeoJsonExportDialog.ts`, `GnssImportDialog.ts` ve `model/gnssImport.ts`'in `placeGnssAll`,
  `placeGnssLines`, `gnssLineEntities`'i; `io/client.ts`, `io/protocol.ts`, `io/formatsWorker.ts`; komutlar `app/fileExchange.ts`,
  menüler, şerit, uygulama menüsü, Kaynaklar (`model/sources.ts`, `app/sources.ts`); simgeler `ui/icons.ts`; resimler `shots.mjs kml`.

## Doğrulama

- **Bağımsız başvurular** (KentOS kodu olmadan):
  - `scripts/fixtures/kml_cases.py` (`fixtures/formats/v1/kml/`): yedi örnek dosya (klasörler, biçemler, StyleMap, veriler, zaman, izler,
    karışık MultiGeometry, bozuk ve atlanan öğeler; KMZ; adsız belge; KML'siz KMZ, XML değil, kök, derinlik) expat ve zipfile ile
    okunur; okuma float float aynı (`tests/all/kml.rs`, web `io/kml.wasm.test.ts`). Dört taşıma PROJ'la (EPSG:5261 ve ED50'nin
    EPSG:1784 yolu), 10⁻⁶ m içinde, doğrulukları ve yollarıyla. Yazıcının üç dosyası (TM33'ten KML, ED50'den KMZ, Pseudo-Mercator'dan
    KML) `--check` ile okunup girdisine göre denetlenir: klasörler, yer imleri, adlar, veriler, biçemler, her köşe PROJ'la 1,5·10⁻¹⁰
    derece içinde, daireler uygulamanın örneklemesiyle.
  - `scripts/fixtures/wgs84_write_cases.py` (`fixtures/formats/v1/wgs84/`): GPX'in iz ve rota çıktıları ve ED50'den WGS 84 GeoJSON'u
    aynı yoldan denetlenir.
  - `gnss_gpx_cases.py` ve `gnss_nmea_cases.py`'nin çizgileri (`lines`), `gnss_import_cases.py`'nin çizgileri (`expectLines`, PROJ'la),
    `source_list_cases.py`'nin KML türü.
- **Yazıcıların baytları** iki platformda kaydedilenlerle aynı (`KENTOS_WRITE_KML=1`, `KENTOS_WRITE_WGS84=1` yeniden yazar).
- **Pencereler:** masaüstü `exchange::kml_tests` (örnek parselin KML'si TM36 çizimine çevrilerek köşesi çizimdeki köşeye oturur, tek
  geri alma adımı; KML, KMZ ve GPX yazılıp geri okunur; GeoJSON WGS 84'te), `gnss_tests`'in iz çizgisi; web duman testinin GeoJSON
  adımı onayla çevirip içe aktarmayı açar.
- **Süreler** (release, bu makine; masaüstü `cargo test --release -p kentos-formats --test all kml_timing -- --ignored --nocapture`, web
  `KML_BENCH=1 … scripts/perf/kml.test.ts` gönderilen WASM'la; üç koşunun en iyisi):

  | İş (100 000 nesne: 40 000 parsel, 30 000 yol, 30 000 kotlu nokta; 350 000 köşe) | Masaüstü | Web | Bütçe |
  |---|---|---|---|
  | KML yaz (her köşe TM36'dan WGS 84'e; 32 MB) | 361 ms | 743 ms | 900 / 2500 ms |
  | KMZ yaz (4,3 MB) | 816 ms | 1265 ms | 1600 / 4000 ms |
  | KML oku | 394 ms | 522 ms | 900 / 2500 ms |
  | KMZ oku | 486 ms | 621 ms | 1200 / 3000 ms |
  | Projenin sistemine çevir (okunan 100 000 nesne) | 183 ms | 458 ms | 500 / 2500 ms |
  | GPX yaz | 330 ms | 715 ms | 900 / 2500 ms |
  | GeoJSON yaz (WGS 84'e) | 393 ms | 813 ms | 900 / 2500 ms |

  Web'in süreleri işçinin yaptığı gibi girdinin JSON'unu da içerir. İlk ölçümde KML okuma 198 s sürdü: roxmltree'nin `text_pos_at`'i
  her çağrıda metni baştan tarıyor (“yalnız hatalar için”), okuyucu ise her yer imi için satırını soruyordu. Satırlar artık bir kez
  kurulan yeni satır dizininden ikili aramayla bulunur (`xml::Rows`); aynı düzeltme GPX ve Trimble JobXML okuyucularına da girdi (100 000
  iz noktalı bir GPX'te de aynı kare büyüme vardı).

- **Resimler:** masaüstü `.run/shots/kml-*` (32), web `apps/web/scripts/e2e/out/shots/kml` (28); 1440 × 900 ve 1100 × 650'de, iki temada.

## Sonuçlar

KML/KMZ ve GPX iki yönde (GPX'in okunuşu GNSS içe aktar'dır); WGS 84 dışındaki projede dönüşüm her yerde açıkça sorulur ya da söylenir.
Kapsam dışı: KML'nin resim kaplamaları (Resim ekle ile yerleştirilir), modelleri, ağ bağlantıları ve gezintileri; KML'ye yazının yazı
olarak yazılması (adlı nokta olur); GPX'in kendi uzantıları (öznitelikler yazılmaz); kotların geoit modeliyle çevrilmesi; DXF, NCZ ve
koordinat listesinin içe aktarılırken çevrilmesi (Koordinat dönüştür ve İşlemler'in Koordinat sistemine dönüştür'ü var).
