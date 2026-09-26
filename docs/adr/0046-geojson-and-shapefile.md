# ADR 0046: GeoJSON içe ve dışa aktarma, Shapefile içe aktarma

- **Durum:** uygulandı, sahibin kararları bekleniyor (2026-09-26). Sorular ve uygulanan varsayılanlar aşağıdadır.
- **Tarih:** 2026-09-26
- **Bağlam belgesi:** CLAUDE.md §5, §9.7, §14, §20, §23; ADR 0009 (dosya biçimleri), 0008 (ortak çekirdek)

## Bağlam

- Envanterde bekleyen üç komut vardı: `file.import.geojson`, `file.export.geojson`, `file.import.shp`. Menüde ve uygulama menüsünde yerleri hazırdı, işleri yoktu.
- ADR 0009'un yolu bellidir: okuyucu ve yazıcı ortak Rust'ta (`crates/shared/formats`), sözleşme `contracts/src/formats.rs`'te, dar WASM bağlayıcısı `crates/wasm/formats-wasm`'da, web akışı `io/` ve `app/fileExchange.ts`'te. Koordinat sistemi sorulur, kayıp raporlanır, içe aktarma tek geri alma adımıdır, kötücül girdide panik yoktur.
- RFC 7946 GeoJSON'u WGS 84 boylam, enlemdir. Projeler çoğunlukla TUREF TM'dedir (ör. EPSG:5256). WGS 84 TUREF değildir: aradaki fark datum ve epoktur, yarım metre düzeyindedir. Koordinat dönüşümü henüz yoktur (`crs.transform`, “Datum dönüşümü”, bekliyor). Sessiz dönüşüm ya da WGS 84'ü TUREF saymak yasaktır (CLAUDE.md §5, §23).

## Karar

### Yerleşim (ADR 0009)

- `crates/shared/formats`:
  - `json.rs`: akışlı JSON okuyucusu. Bütün metni bir kez yürür; milyon özellikli bir koleksiyon JSON ağacı olarak tutulmaz. Sayıyı dosyanın yazdığı gibi tutar. İç içe dizi ve nesne 64 düzeyle sınırlıdır; her hata satırını söyler.
  - `gis.rs`: iki okuyucunun ortak nesne yapımı (nokta, çizgi, yol, delikli alan), katmanlar karşılaşma sırasıyla, kapsam, nesne sınırı (varsayılan bir milyon; kalan sayılır, raporlanır).
  - `geojson/read.rs`, `geojson/write.rs`.
  - `shp/`: `shape.rs` (.shp kayıtları, halkaların dış sınır ve deliğe ayrılması), `dbf.rs` (dBASE III tablosu, kodlama), `prj.rs` (WKT 1).
  - `text.rs`: ISO-8859-9, CP857, CP850 ve CP437 tabloları. Bağımlılık yok.
- Sözleşme (`FORMATS_VERSION` 7):
  - `ImportResult.declaredCrs`: `DeclaredCrs { srid?, text, source }`; `source` `rfc7946`, `geoJsonCrs` ya da `prj`'dir.
  - `GeoJsonReadOptions`, `ShapefileReadOptions`, `GeoJsonLayer`, `GeoJsonWriteInput`.
- WASM: `readGeoJson`, `writeGeoJson`, `readShapefile` (dosyalar ayrı `Uint8Array`'lerle; .shp zorunlu, öbürleri isteğe bağlı). Yazıcının girdisindeki nesneler DXF yazıcısının elle yazılmış ziyaretçisiyle okunur (`dxf::Objects`).
- Web:
  - `io/protocol.ts`, `io/client.ts`, `io/formatsWorker.ts`: üç işlem.
  - `io/shapefile.ts`: seçilen dosyalardan bir Shapefile katmanı.
  - `app/fileIO.ts`: `pickManyForImport`, seçicide `openMany` (yoksa çok dosyalı gizli girdi).
  - `app/fileExchange.ts`: üç komut; `app/commands.ts`'teki `pending` kayıtları kalktı.
  - `ui/io/GisImportDialog.ts` (GeoJSON ve Shapefile), `ui/io/GeoJsonExportDialog.ts`.
  - `ui/io/common.ts`: `CrsQuestion.declare`.
- Yeni bağımlılık yoktur.

### Koordinat sistemi

- Okuyucu dosyanın ne dediğini bildirir; hiçbir koordinatı dönüştürmez.
  - GeoJSON'da `crs` üyesi yoksa RFC 7946'dır: 4326.
  - `crs` adı `EPSG::n` ya da `EPSG:n` ile bitiyorsa (büyük/küçük harf fark etmez) n'dir; `CRS84` içeriyorsa 4326'dır. `{"type":"EPSG","properties":{"code":n}}` n'dir. Başka her şey okunamadı sayılır.
  - `.prj` (WKT 1, ESRI ya da OGC): önce en dıştaki düğümün kendi `AUTHORITY["EPSG","n"]`'si.
  - Yoksa datum ve parametreler, tam eşitlikle:
    - Datum adı küçük harfe çevrilip yalnız ASCII harf ve rakamları bırakılarak karşılaştırılır. `turef`, `turkishnationalreferenceframe`, `itrf96`, `itrf1996` → TUREF; `european1950`, `europeandatum1950`, `ed50` → ED50; `wgs1984`, `wgs84` → WGS 84.
    - Transverse Mercator, metre, FE 500000, FN 0, başlangıç enlemi 0:
      - ölçek 1, orta meridyen 27…45 (3°): TUREF TM 5253–5259, ED50 TM 2319–2325;
      - ölçek 0,9996, orta meridyen 27, 33, 39, 45: ED50 UTM 23035–23038, WGS 84 UTM 32635–32638.
    - Yalnız GEOGCS: WGS 84 → 4326, TUREF → 5252.
  - Başka her .prj tanınmaz; adı gösterilir.
- Pencere (`CrsQuestion.declare`):
  - Dosyanın dediği sistem seçili gelir. Okunamayan bir ifadede hiçbir sistem seçili gelmez; ifade bir sisteme tahmin edilmez.
  - `.prj`'siz Shapefile DXF gibi projenin sistemiyle gelir ve bunu söyler.
- Uygulanan varsayılanlar:
  1. WGS 84 dosyası WGS 84 (4326) projesine olduğu gibi girer.
  2. Koordinatları projenin sisteminde olan dosya dönüştürülmeden girer. Bu, `crs` üyesiyle, `.prj`'yle ya da pencerede açık seçimle söylenir. Açık seçim dosyanın dediğinden başkaysa pencere uyarır: koordinatlar dönüştürülmeden seçilen sistemde sayılır.
  3. WGS 84 → TM dönüşümü yoktur. Pencere “koordinat dönüşümü henüz yok” der, Koordinat → Datum dönüşümünü (`crs.transform`) gösterir ve içe aktarmayı kapalı tutar. Özetin ilk satırı da bunu söyler.
- Kapsam denetimi uyarır, engellemez. Seçilen sistem coğrafiyken koordinatlar ±180, ±90 dışındaysa uyarır. Projeksiyonluyken hepsi ±180, ±90 içindeyse “derece gibi görünüyor” der.
- Dışa aktarma:
  - 4326 projesi RFC 7946 GeoJSON'u yazar; `crs` üyesi yoktur.
  - Başka bir proje kendi koordinatlarıyla yazılır; eski (2008) `crs` üyesi `urn:ogc:def:crs:EPSG::<srid>` sistemi adlandırır.
  - Pencere yazmadan önce dosyanın RFC 7946 olmayacağını ve dönüşümün olmadığını söyler. Yazıcının raporu da aynı notu düşer.

### GeoJSON okuma

- Kök bir FeatureCollection, tek bir Feature ya da çıplak bir geometridir. Üyeler her sırada gelebilir. `features` iki kez varsa dosya reddedilir.
- Dosya UTF-8 olmalıdır (baştaki BOM atlanır); UTF-16 ve bozuk UTF-8 nedeniyle reddedilir. JSON hatası satırıyla söylenir; hiçbir şey alınmaz.
- Katman:
  - Özelliğin `kentos.layer`'ı (boş olmayan metinse).
  - Yoksa belgenin boş olmayan `name`'i.
  - O da yoksa dosyanın uzantısız adı.
  - `kentos.label` etiket olur.
- Geometri:
  - Point → nokta, üçüncü sayı Z'dir. MultiPoint → konum başına nokta.
  - LineString: iki konum çizgidir, daha fazlası yoldur, ikiden azı alınmaz. MultiLineString: üye başına.
  - Polygon: ilk halka dış sınır, öbürleri deliktir. Son konum ilkiyle x ve y'de tam aynıysa düşer. Kapanmamış halka olduğu gibi alınır, raporlanır. Üçten az köşeli dış sınır alanı düşürür; delik düşer. Halka yönü değiştirilmez.
  - MultiPolygon: üye başına. GeometryCollection: üye başına, iç içe.
  - Z yalnız noktada kalır. Çizgi ve alanda düşer, raporlanır; dördüncü ve sonraki sayılar da.
- Konum en az iki sayıdan oluşan bir dizidir. Değilse ya da x, y, z sonlu değilse (`1e999`) en küçük birim alınmaz: nokta, LineString, MultiLineString üyesi, bütün Polygon, MultiPolygon üyesi, GeometryCollection üyesi. Geri kalanı okunur.
- Öznitelikler, `properties`'in üyeleri:
  - metin olduğu gibi; sayı dosyanın yazdığı gibi (`1.50`, `1e3`, `-0.0`);
  - `true` ve `false`; `null` özniteliği bırakır;
  - nesne ya da dizi sıkı JSON metni olur, raporlanır;
  - yinelenen anahtarda sonuncu geçer.
- Bilinmeyen tür, geometrisiz ya da `null` geometrili özellik ve Feature olmayan öğe satırıyla raporlanır.

### Shapefile okuma

- Katmanın dosyaları birlikte seçilir: `.shp` zorunlu; aynı adlı `.shx`, `.dbf`, `.prj`, `.cpg` kullanılır. Ad büyük/küçük harf ayırmadan karşılaştırılır.
  - Başka ad ya da tür kullanılmaz ve söylenir.
  - Birden çok `.shp` seçildiyse bir seferde bir katman alınır; pencere bunu söyler.
  - `.zip` okunmaz (sahibe soru 4).
- Kayıtlar `.shp`'den sırayla okunur. `.shx` yalnız sayı karşılaştırması için okunur. Bir kaydın uzunluğu dosyanın sonunu aşıyorsa okuma orada durur, raporlanır. Uzunluklar ve sayılar hep dosyanın baytlarına karşı denetlenir; dosya taşan bellek ayırtamaz.
- Türler:
  - Null hiçbir şey vermez.
  - Point, PointZ, PointM noktadır; Z yalnız PointZ'den gelir.
  - MultiPoint(Z/M) nokta başına bir noktadır.
  - PolyLine(Z/M): parça başına; iki nokta çizgi, fazlası yol, azı hiçbir şey.
  - Polygon(Z/M): halkalar (aşağıda).
  - MultiPatch ve bilinmeyen türler raporlanır.
- Z türlerinde Z bloğu gerekir, M bloğu isteğe bağlıdır. M türlerinde M bloğu gerekir. Eksik kayıt hiçbir şey vermez. M değerleri hep düşer, raporlanır.
- Parçalar 0'dan başlamalı, azalmamalı, nokta sayısını aşmamalıdır; değilse kayıt alınmaz.
- Sonlu olmayan x ya da y: noktada o nokta, yolda o parça, alanda bütün kayıt düşer.
- Halkalar:
  - Kapanış noktası düşer.
  - Shoelace alanı eksiyse (saat yönü) dış sınır, artıysa delik, sıfırsa hiçbir şey. Üçten az köşe de hiçbir şey vermez.
  - Delik, ilk köşesini tutan ilk dış sınıra gider (çift-tek ışın). Hiçbir dış sınır tutmazsa kendi başına alan olur, raporlanır.
  - Saat yönünde halkası olmayan kayıtta her halka ayrı alandır.
  - Alanlar dış sınırlarının parça sırasıyla gelir.
- DBF (dBASE III):
  - n'inci kaydın öznitelikleri n'inci şeklindir. Silinmiş kaydın şekli alınmaz. Yalnız tam kayıtlar sayılır.
  - Alanlar: C (sondaki boşluk ve NUL silinir; boşsa bırakılır); N, F (iki uçtan silinir; metin olduğu gibi); L (`T t Y y` → `true`, `F f N n` → `false`, başkası bırakılır); D (`YYYYMMDD` → `YYYY-MM-DD`, `00000000` ve boş bırakılır).
  - Başka türler (M, B, G, I, T…) alınmaz, raporlanır. Adı boş alan bırakılır.
  - Bozuk başlıkta kayda sığmayan alanlar alınmaz, raporlanır. Tablo okunamazsa şekiller özniteliksiz alınır.
- Kodlama:
  - `.cpg` (boşluklar silinir, büyük/küçük harf fark etmez): `UTF-8`, `1254`/`ANSI 1254`, `1252`, `ISO-8859-9`/`LATIN5`, `857`/`OEM 857`, `850`, `437` ve benzerleri.
  - Yoksa dil sürücüsü baytı: 0xCA → 1254; 0x6B, 0x88 → CP857; 0x03, 0x58, 0x59 → 1252; 850 ve 437 listeleri.
  - Öbür durumda Windows-1254 varsayılır ve söylenir. Tanınmayan `.cpg`'de de öyle.
  - Kodlama pencerede kaynağıyla gösterilir.

### GeoJSON yazma

- FeatureCollection'dır, `name` çizimin adıdır. Her satırda bir özellik vardır. Sayılar float64'e geri dönen en kısa ondalıktır (`num::plain`).
- `properties`: öznitelikler metin olarak. `kentos`: katman adı ve boş olmayan etiket; KentOS geri okur, öbür programlar yok sayar. Nesnenin kendi rengi ve sembolü yazılmaz, raporlanır.
- Geometri:
  - Nokta Point'tir (Z ile); çizgi ve yaysız yol LineString'dir.
  - Alan Polygon'dur (delikleriyle). Halkalar RFC 7946'nın sağ el kuralına çevrilir: dış sınır saat yönünün tersine, delik saat yönünde. İlk köşe yerinde kalır; çevrilenler raporlanır.
  - Tarama Polygon'dur; deseni yazılmaz.
- Eğriler uygulamanın kendi örneklemesiyle yazılır; çekirdeğin `entity_outline`'ının kullandığı işlevler doğrudan çağrılır:
  - daire: 72 kenarlı kapalı LineString;
  - yay, elips: turda 72 adım; elipste turda 144 nokta;
  - spline: açıklık başına 16 nokta. Uçları kaynak köşelere sabitlenir (eğri uydurma noktalarından geçer).
  - Yaylı yol ve alan da örneklenir. Hepsi raporlanır.
- Yazı, ölçü, uçsuz doğru ve ışın yazılmaz, raporlanır. Sonlu olmayan koordinatlı nesne ve üçten az köşeli halka da yazılmaz.

### Bağımsız doğrulama (§23.4)

- `tools/formats/gis.py`: bağımsız okuyucu. Yalnız standart kitaplıkla, Rust'a bakmadan, bu kurallardan yazıldı (ayrı bir ajan).
- `scripts/fixtures/gis_reference.py`: fixture yazıcısı. `--check` her dosyayı yeniden üretir ve okur. Elle yazılmış bir `SUMMARY` ile de karşılaştırır. Halkaları kesirli (tam) aritmetikle denetler. `export/` çiftlerini bağımsız okuyucuyla geri okur.
- `fixtures/formats/v1/gis`: 12 fixture (6 GeoJSON, 6 Shapefile takımı) ve `export/`'ta 2 çift. Rust (`tests/gis.rs`) ve WASM (`io/gis.wasm.test.ts`) her fixture'ı `expected.json` ile float float karşılaştırır. Yazıcının baytları commit'lenmiş dosyayla aynıdır.
- Başvuru iki yerde daha sıkıdır; fixture'larda ikisi de yoktur:
  - bozuk DBF başlığında bütün dosyayı reddeder;
  - WKT'de virgülleri zorunlu tutar.

### Modül boyutu

- Biçim modülü (`kentos_formats_wasm_bg.wasm`) 1 348 503 bayttan 1 494 431 bayta büyüdü; gzip -9 ile 426 587'den 482 715'e.
- Modül yalnız içe ve dışa aktarmada, çizim açıp kaydederken yüklenir; başlangıca yük eklenmedi.
- Eğrileri `entity_outline` yerine kendi işlevleriyle örneklemek 8 KB kazandırdı: ölçü ve yazı düzeni modüle girmez.

## TM projeksiyonu önerisi (yapılmadı)

- Aynı datum içinde Transverse Mercator ileri ve geri hesabı ortak Rust'ta yapılabilir. Coğrafi TUREF (5252) ↔ TUREF TM ya da WGS 84 ↔ WGS 84 UTM içindir. Yöntem Krüger serileridir, Karney 2011'in 6. derece açılımı; GRS80, WGS 84 ve International 1924 elipsoitleriyle.
- Bağımsız referans (§23.4): PROJ (`cs2cs`) ve GeographicLib (`TransverseMercatorProj`) çıktıları, sürümleriyle `fixtures/crs/v1`'e yazılır. Tolerans 0,1 mm'dir. Dilim kenarı, dilim dışı, ekvator ve yüksek enlem noktaları eklenir.
- Datumlar arası (WGS 84 → TUREF, ED50 → TUREF) ayrı bir karardır: epok, levha hızı ya da 7 parametre ve kaynağı gerekir. Projeksiyonla birlikte sessizce yapılmaz. Bu yüzden öneri, RFC 7946 dosyasını TM projesine tek başına açmaz.

## Sahibe sorular ve varsayılanlar

Önerilen seçenek ilk sıradadır; uygulanan odur.

1. **WGS 84 GeoJSON'u TM projesine almak:**
   - (a) Dönüşüm (`crs.transform`) bağımsız referanslarla gelene kadar kapalı kalır. Kullanıcı dosyanın gerçekte projenin sisteminde olduğunu açıkça seçebilir.
   - (b) WGS 84'ü TUREF sayıp yalnız TM projeksiyonu yapmak. Yarım metre düzeyinde datum ve epok farkı kalır; kadastroya uygun değildir.
   - (c) PROJ'u WASM olarak eklemek: büyük bir bağımlılıktır.
2. **TM projeksiyonu:**
   - (a) Yukarıdaki öneri olarak kalsın; datum ve epok kararıyla birlikte ayrı bir dilimde yapılsın.
   - (b) Şimdi yalnız aynı datum içinde (5252 ↔ TUREF TM) yapılsın.
3. **TM projesinin GeoJSON'u:**
   - (a) Projenin koordinatları ve eski `crs` üyesiyle yazılır; pencere RFC 7946 olmadığını söyler.
   - (b) Yalnız 4326 projelerinde izin verilir.
4. **`.zip` Shapefile:**
   - (a) `miniz_oxide` eklenir (yalnız inflate; saf Rust, MIT/Zlib/Apache-2.0). Yanında küçük bir ZIP dizin okuyucusu yazılır; boyut, oran ve girdi sayısı sınırlarıyla (zip bombası).
   - (b) `.zip` olmaz; dosyalar birlikte seçilir (bugünkü hâl).
5. **`.prj`'siz Shapefile:**
   - (a) DXF gibi projenin sistemi seçili gelir ve söylenir.
   - (b) Kullanıcı açıkça seçmeden içe aktarma kapalı kalır.
6. **Birden çok katman:**
   - (a) Şimdilik bir seferde bir katman alınır. Sonra her `.shp` ayrı katman olarak tek bir içe aktarmaya (tek geri alma adımı) girer.
   - (b) Şimdi yapılır.
7. **Öznitelik türleri:**
   - (a) Tipli öznitelik (§24.1) gelene kadar metin kalır. Sayı dosyanın yazdığı gibi okunur, metin olarak yazılır.
   - (b) Dışa aktarmada sayıya benzeyen metin sayı yazılır: yuvarlama ve biçim riski vardır.
8. **İki konumlu LineString:** (a) çizgi; (b) iki noktalı yol.
9. **Daire ve tam elips dışa aktarımı:** (a) kapalı LineString (KentOS'ta eğridir, alan değildir); (b) Polygon.
10. **Kodlaması bilinmeyen `.dbf`:**
    - (a) Windows-1254 varsayılır ve söylenir.
    - (b) Pencerede kodlama seçimi sunulur.
11. **`kentos` yabancı üyesi:** (a) katman ve etiket onunla gider ve geri gelir; (b) hiç yazılmaz.

## Bu adımda olmayanlar

- Shapefile dışa aktarma, `.zip` ve çok katmanlı içe aktarma.
- Koordinat dönüşümü ve tipli öznitelikler.
- GeoJSON'un `bbox`'ı ve Feature `id`'si okunmaz, yazılmaz.
- TopoJSON; MultiPatch; renk ve sembolün GeoJSON'a gidip dönmesi.

## Doğrulama (26 Eylül 2026, Linux)

- Rust:
  - `cargo test -p kentos-formats`: 66 birim testi (JSON okuyucusu, GeoJSON okuma ve yazma, Shapefile kayıtları, DBF, WKT, kod sayfaları, nesne sınırı).
  - `tests/gis.rs`: 12 fixture bağımsız okuyucuyla float float aynı okunur; 2 dışa aktarmanın baytları commit'lenmiş dosyayla aynıdır ve geri okunur. Her fixture dosyasının her kesiği ve fixture başına 2000 bozuk kopya panik vermeden okunur.
  - `cargo clippy -p kentos-formats -p kentos-formats-wasm -p kentos-contracts --all-targets -- -D warnings` temiz; `cargo fmt` uygulandı.
  - `pnpm rust:test` (bütün çalışma alanı: test, clippy, bağımlılık yönü) geçti.
- `python3 scripts/fixtures/gis_reference.py --check`: 12 fixture, 44 dosya ve 2 `export/` çifti tutarlı.
- Sözleşme: `cargo test -p kentos-contracts` TypeScript türlerini yeniden üretti (`DeclaredCrs`, `CrsSource`, `GeoJsonReadOptions`, `ShapefileReadOptions`, `GeoJsonLayer`, `GeoJsonWriteInput`, `ImportResult`).
- Web:
  - `pnpm typecheck` geçti.
  - `pnpm test`: 103 dosya, 1414 test geçti. 13 dosya atlandı: sunucu isteyen veritabanı ve bulut testleri, önceden de atlanıyordu. Yeni testler:
    - `io/gis.wasm.test.ts`: 12 fixture WASM'la float float aynı; dışa aktarma baytları aynı; ret nedeniyle.
    - `io/shapefile.test.ts`: dosyaların katmana ayrılması.
  - `pnpm build` geçti. Pencereler ayrı parçalardadır: `GisImportDialog` 7,8 KB, `GeoJsonExportDialog` 6,5 KB.
  - `pnpm e2e`: 176 denetim geçti. Yeni denetimler:
    - `tm.geojson`: katmanlar, `crs` üyesinin söylediği sistem seçili ve içe aktarma açık. Nesneler bağımsız okuyucunun okuduğu gibi, koordinatlar bit bit. Tek geri alma adımı.
    - `features.geojson` (RFC 7946) TM projesine alınmaz; pencere “koordinat dönüşümü henüz yok” der. Projenin sistemini açıkça seçmek uyarılır, derece gibi görünen sayılar gösterilir. Vazgeç hiçbir şey almaz.
    - Shapefile (`noktalar` ve başka bir katmanın `.dbf`'i): dosyalar, kodlama ve `.prj` söylenir, başka katmanın dosyası kullanılmaz. Nesneler Z ve Türkçe öznitelikleriyle aynı. Tek geri alma adımı.
    - GeoJSON dışa aktarma: pencere RFC 7946 olmayacağını söyler; dosyada `crs` üyesi, 3 özellik ve kotlar vardır. Geri okununca aynı adlı katmana aynı nesneler gelir. Geri almalar çizimi ilk hâline döndürür.
  - `pnpm inventory`: `file.import.geojson`, `file.export.geojson`, `file.import.shp` çalışıyor; bekleyen komut 18 → 15, pencere 59 → 62.
- Ekran görüntüleri (koyu, açık, Büyük yazı), `apps/web/scripts/e2e/out/` altında: `io-geojson-import-*`, `io-geojson-wgs84-*`, `io-shp-import-*`, `io-geojson-export-*`.
- Bu adımda çalıştırılmayan: `pnpm e2e:cloud` (bulut kodu değişmedi; biçim modülünün yeni sürümüyle yerel `.kcad` kaydet/aç smoke'ta geçti).

## Bilerek bozma (26 Eylül)

Her biri tek başına uygulandı, denetim çalıştı, sonra geri alındı.

- R1: GeoJSON halkasının kapanış konumu düşmedi. Birim testi düştü. `tests/gis.rs` dört fixture'da farkı gösterdi (`bare-polygon`, `features`, `nonfinite`, `tm`).
- R2: Shapefile'da saat yönünün tersindeki halkalar dış sınır sayıldı. Birim testi ve `tests/gis.rs` düştü (`alanlarz`: 3 nesne ≠ 1, `parseller`: 7 ≠ 5).
- R3: `.cpg` okunmadı. Birim testi ve `tests/gis.rs` düştü (`karisik`, `kuyular`, `yollar` kodlamaları).
- R4: GeoJSON yazıcısı halka yönünü çevirmedi. Birim testi ve dışa aktarma baytları testi düştü.
- R5: ESRI'nin `D_European_1950`'si ED50 sayılmadı. `.prj` birim testi düştü.
- R6: GeoJSON sayı özniteliği float64'ten yeniden yazıldı (`1.50` → `1.5`). Birim testi düştü.
- P1: bağımsız okuyucu halkanın kapanış konumunu tuttu. `--check` altı `expected.json`'ın yeniden üretilenle aynı olmadığını söyledi.
- P2: dışa aktarma denetimi sağ el kuralını beklemedi. `--check` `karma`'nın alanını ve taramasını gösterdi.
- W1: pencere dosyanın dediğini yok sayıp projenin sistemini seçti (WGS 84'ü TUREF saymak). `pnpm e2e`'de yalnız “RFC 7946 dosyası TM projesine alınmaz” denetimi düştü (seçili 5256, içe aktarma açık).
