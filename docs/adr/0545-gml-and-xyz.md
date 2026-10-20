# ADR 0545: GML ve XYZ alışverişi

- **Durum:** kabul edildi (2026-10-20). `plan/gis.md` CBS-05'in bir dilimi. Kapsamı ben belirledim.
- **Bağlam belgesi:**
  - ADR 0502 (GeoPackage ve alma kuralı);
  - 0514 (Shapefile dışa aktarma);
  - 0522 (CSV alışverişi);
  - 0544 (FlatGeobuf);
  - 0208 (harita servisleri: WFS'in GML'i ve eksen sırası kaydı);
  - OGC GML 3.2.1 (ISO 19136) ve basit nesneler profili (OGC 10-100r3); GDAL'ın GML sürücüsü.
- **Web:** ertelendi (ADR 0228). Web'de gerekenler TODOS.md §0.2'de.
- **Sonraya kalan:**
  - GeoParquet (parquet kitaplığı ister: sahibin onayı), staging ve upsert (CBS-05'in kalanı);
  - XYZ okuma: konum listesi zaten Koordinat listesi, CSV (X, Y, Z sütunları) ve nokta bulutunun metin okuyucusundan alınır;
    ayrı bir XYZ içe aktarması yok;
  - GML'in karmaşık içeriği (GMLAS'ın iç içe ve ilişkili nesneleri), `ArcByCenterPoint`, `Circle`, `Solid`, TIN ve çokyüzlü
    yüzey, bir nesnede ikinci geometri özelliği, `gml:id`'yi alan yapmak, WFS-T;
  - M değerleri (GML'de yok).

## Bağlam

GML, kamu kurumlarının (INSPIRE, CBS veri paylaşımı, WFS) alışverişte en çok kullandığı XML biçimidir; QGIS, GDAL ve ArcGIS
yazar ve okur. KentOS'ta GML'in yalnız WFS yanıtı olarak okunan bir parçası vardı (`kentos_services::gml`: GeoJSON'a çevirir,
yay, tür ve boş değer taşımaz). Dosya olarak GML ne okunuyordu ne yazılıyordu. XYZ (metin satırlarında X Y Z) nokta
bulutu ve ölçü aletlerinin ortak biçimidir; vektör çizimden yazan bir yol yoktu.

## Karar

### 1. Kapsam

- **`file.export.gml`:** GML 3.2 basit nesneler, GDAL'ın `FORMAT=GML3.2` yazdığı profil.
- **`file.import.gml`:** GML 2, 3.1 ve 3.2 (GDAL'ın ve WFS sunucularının yazdığı).
- **`file.export.xyz`:** noktalar ya da bütün köşeler `X Y Z` satırları.

Üçü de başsız sunucunun dosya komutu ya da GeoPackage alma yolunun komutudur; Python ve MCP katalogdan alır.

### 2. Kodek: `kentos_native_application::{gml_write, gml_read, gml_table, xyz_table}`

GeoPackage çekirdeğinin geometri ve değer türlerini kullanır (`Geometry`, `Curve`, `Value`, `ColumnKind`), çünkü okuma
GeoPackage alma yolundan geçer. XML okuması `kentos_formats::xml::document` ile (roxmltree; DTD yok, 256 düzeyden derin
belge okunmadan reddedilir); `xml` modülü `Node` ve `Document`'ı yeniden dışa açar, yeni bağımlılık yoktur. Dosya en çok 256
MiB'tır.

### 3. Yazma

**Yapı.** GDAL'ınki gibi: `ogr:FeatureCollection` (ad alanı `http://ogr.maptools.org/`, GDAL'ın varsayılanı; `gml` 3.2),
`gml:boundedBy` zarfı (koleksiyonun ve her nesnenin; yayların gerçek kapsamıyla), her nesne `featureMember` içinde nesne türünün
adıyla bir öğe (`gml:id="tür.sıra"`), geometri `geometryProperty`'de, özellikler ardından. NULL değer `xsi:nil="true"`, boş
metin boş öğe. Bütün yazılır ya da hiç: önce şema, GML en son.

**Nesne türleri.** Her katman bir nesne türüdür (öğe adı katmanın adından, XML adına çevrilerek: harf, rakam, `_`, `-`, `.`;
rakamla başlarsa `_` önde; aynı ad `_2` alır; değişen ad `export_losses`'ta söylenir). FlatGeobuf ve CSV'nin tek tablosu
`katman` sütunuyla yazılırdı; GML'de her katmanın kendi geometri türü ve alanları vardır, GDAL ve QGIS birden çok türü
katman katman okur. Özellik adları da XML adına çevrilir; `geometryProperty` ve birbirinin adı olan alan `_` alır.

**Özellikler.** Katmanın alanları sırasıyla, ardından hiçbir alanın adlamadığı öznitelikler adlarıyla metin olarak. Değerler:
metin kaçışlı (`& < >`; XML 1.0'da bulunmayan denetim karakterleri atılır, sayılır), tam sayı, ondalık sayı (en kısa metin,
üstel yazım yok), evet/hayır `true`/`false`, tarih `2024-02-29`, tarih-saat `2024-03-01T10:20:30Z`.

**Geometri.** `Point`, `LineString`, `Polygon` (`exterior`, `interior`), `MultiPoint`, `MultiCurve`, `MultiSurface`,
`MultiGeometry`. Yaylar kesin: `Curve`'ün `segments`'inde `ArcString`; yaylı halka `Ring`'in `curveMember`'ları; bileşik eğri
`CompositeCurve`. Elips, eğri ve NURBS kirişle (`export_losses`).

**Boyut.** Her `posList` boyutunu söyler (`srsDimension="2"` ya da `"3"`): GDAL, boyutu olmayan listeyi koleksiyon zarfının
boyutuyla okur ve hem kotlu hem kotsuz nesne türü olan dosyada kotsuz listeleri üçlü sanıp reddeder (kendi yazdığı iki
türlü dosyayı da okuyamıyor, bulgu). `pos` sayıların adedinden. Bir geometride herhangi bir köşenin kotu varsa geometri kotludur;
kotsuz köşe 0 yazılır ve söylenir (GML'de kotsuz konum yok). M yazılmaz, söylenir.

**Sistem ve eksen sırası.** Projenin sistemi `srsName="urn:ogc:def:crs:EPSG::kod"` (zarflarda ve en üst geometride) olarak yazılır
ve koordinatlar EPSG'nin eksen sırasıyla gider: TUREF'in ve ED50'nin TM dilimleri kuzey önce, coğrafi sistemler enlem önce,
UTM ve Pseudo-Mercator doğu önce. Sıra `fixtures/crs/v1/registry.json`'dan (`kentos_services::crs::axis_order`), PROJ'a karşı
denetlenmiş kayıttan; kayıtta olmayan sistem doğu önce sayılır. Sistemsiz projede `srsName` yok, söylenir.

**Şema.** Dosyanın yanına `<ad>.xsd` (GDAL'ın yapısı): her nesne türü `ogr:<ad>_Type`, geometri özelliği
`gml:…PropertyType` (geometrilerin türüne göre `Point`, `LineString`, `Curve`, `Polygon`, `Surface`, `MultiPoint`,
`MultiCurve`, `MultiSurface`, karışıksa `Geometry`), özellikler `xs:string` (uzunluğuyla `maxLength`), `xs:long`,
`xs:double`, `xs:boolean`, `xs:date`, `xs:dateTime`. `schema: false` şemayı ve `xsi:schemaLocation`'ı yazmaz. `schemaLocation` boşlukla ayrılan bir liste olduğundan dosya adındaki
boşluk ve URI'de olmayan işaretler yüzde kodlanır (`Örnek pafta.xsd` → `Örnek%20pafta.xsd`; GDAL adı olduğu gibi yazar). GML'in
adı `.xsd` ile bitemez (şema ezilirdi): `invalid_file_path`.

### 4. Okuma

**Belge.** Koleksiyonun üyeleri: `featureMember`, `member` (WFS 2.0) ve çoğul `featureMembers`; her nesnenin öğe adı nesne
türüdür. Kök adı `Collection` ile bitiyorsa üyesiz belge de koleksiyondur (alınacak nesne yok: `nothing_to_import`); başka
kök, bozuk XML ya da UTF-8 olmayan dosya `import_unreadable`.

**Geometri.** `Point`, `LineString`, `LinearRing`, `Polygon` (3'ün `exterior` ve `interior`'ı, 2'nin `outerBoundaryIs` ve
`innerBoundaryIs`'i), `MultiPoint`, `MultiCurve`, `MultiLineString`, `MultiSurface`, `MultiPolygon`, `MultiGeometry`,
`Curve` (`LineStringSegment`, `ArcString`, `Arc`), `CompositeCurve`, `Ring`, `Surface` (yamalarıyla). Konumlar `posList`,
`pos`, GML 2'nin `coordinates`'ı (`cs`, `ts`) ve `coord`'u. Yaylar kesin; art arda gelen çizgiler ve yay dizileri eklenti
noktasında birleşirse tek parça olur, geri kalanı bileşik eğri. Boyut `srsDimension`'dan (üstteki ata kalıtılır), `pos`'ta
sayıların adedinden. Okunmayan geometri (`Solid`, `TIN`, `ArcByCenterPoint` …, sayı olmayan koordinat, bütün konumlara
bölünmeyen liste) o nesnenin satırını düşürür ve GeoPackage alma yolunun “N satır alınmadı” iletisinde söylenir; geometri
koleksiyonu çizimde nesne olmadığından aynı yoldan düşer.

**Eksen sırası.** Bir geometrinin `srsName`'i sırayı söyler (ataya kalıtılır): `urn:` ve `http` adları EPSG'nin sırasıyla
(yukarıdaki kayıt; bildirilen sistem kayıtta yoksa doğu önce), `EPSG:kod` ve GML 2'nin adları doğu önce, OGC CRS84
(`urn:ogc:def:crs:OGC:1.3:CRS84`, `…/OGC/1.3/CRS84`) boylam önce ve EPSG:4326'dır. Çıkan koordinatlar hep doğu, kuzey. Adı geometride
olmayan dosya koleksiyonun `boundedBy` zarfının adına bakar; hiçbiri yoksa `crs_undeclared`. Birden çok sistem varsa ilki
esas alınır, söylenir; EPSG olmayan ad söylenir. `srid` girdisi dosya sistem bildirmediğinde.

**Özellikler.** Geometri içermeyen öğeler özelliktir; `gml:` ad alanındakiler (`boundedBy`, `name`, `description` …) alınmaz;
`gml:id` alan olmaz (kalıcı kimlik zaten dosyadan türer; GDAL `gml_id` alanı açar). İç içe basit öğeler `dış_iç` adlı alan
olur (GDAL'ın adı yalnız iç öğedir, ad çakışmasında dış_iç); tekrarlanan öğede ilk değer alınır, söylenir; ikinci geometri
özelliği alınmaz, söylenir. `xsi:nil="true"` boş değer; **boş öğe metin alanında boş metin, öbür türlerde boş değerdir**
(GDAL boş öğeyi boş değer sayar; GML'de ikisi ayrıdır: boş metin ile `xsi:nil`, CBS-06).

**Türler.** Üç kaynaktan, bu sırayla:
1. Dosyanın yanındaki aynı adlı `.xsd` (`xs:element`'in `type`'ı ya da satır içi `restriction`'ı: `string`, `integer` ve
   benzerleri, `decimal`, `double`, `float`, `boolean`, `date`, `dateTime`; `maxLength` ve `length` alan uzunluğu olur;
   `gml:…PropertyType` geometridir). Şemanın adlamadığı özellik değerlerinden türlenir.
2. GDAL'ın `.gfs`'i (`Type`, `Subtype`, `Width`).
3. Değerlerden: hepsi tam sayı (baştaki sıfırsız, `i64`'e sığan: “007” bir kimliktir, metin kalır) → tam sayı; hepsi sayı →
   ondalık; `true`/`false` → evet/hayır; `YYYY-MM-DD` → tarih; ISO tarih-saat → tarih-saat; başka → metin (uzunluksuz:
   GDAL en uzun değerin baytını uzunluk yapar, biz yapmayız). Boş değerler türü belirlemez.

Bildirilen türe uymayan değer sütunu metin yapar ve söylenir. 2⁵³'ten büyük tam sayılar GeoPackage alma kuralıyla metin
alanı olur (`import_losses`).

**Alma.** `file_import_gpkg::run_read` (FlatGeobuf ve CSV gibi): her nesne türü bir tablo (`tables` ile seçilir, olmayan ad
`table_not_found`), `layers` türü katmana yönlendirir; yoksa türün adındaki katmana, yoksa alanlarıyla yeni katmana;
sistem denetimi (`crs_mismatch`, `reproject`), kalıcı kimlik `UUIDv5("gpkg:gml/ad.gml/tür:sıra")`, sıra türün
nesnelerinde 1'den; tek geri alma adımı “GML: ad.gml”.

### 5. XYZ

`file.export.xyz`: UTF-8, LF, her satır `X Y Z` (X doğu, Y kuzey, projenin sisteminde, GDAL'ın XYZ sürücüsünün ve nokta
bulutu araçlarının sırası; Türk ölçmeciliğinin Y,X sırası değil). Dosya koordinat sistemi taşımaz: cevap `srid`'i söyler.

- **`vertices`:** `points` (varsayılan: nokta nesnelerinin ve çok noktanın konumları; çizgi, alan ve öbürleri yazılmaz,
  türlerine göre sayılır, hiç nokta yoksa `nothing_to_export` `all`'ı önerir) ya da `all` (her nesnenin her köşesi; kapalı
  halka ya da eğrinin tekrarlanan son köşesi bir kez; yayın uç noktaları, orta noktası değil, sayılır; bileşik eğride
  parçaların ortak köşesi bir kez; elips, eğri ve NURBS kirişlerinin köşeleriyle, söylenir). Geometri GeoPackage dışa aktarmanın
  geometrisidir, yani daire yayının uç noktalarıdır.
- **`separator`:** boşluk, virgül, noktalı virgül, sekme. **`header`:** ilk satır `X Y Z` (ayırıcıyla).
- **`decimals`:** 0–15 sabit basamak; yoksa `double`'a geri okunan en kısa metin (üstel yazım yok, `-0` yok); 15'ten
  fazlası `invalid_input`.
- **Kot:** üçüncü sütun hep var. Kotu olmayan konum `withoutZ: zero` ile 0 yazılır (varsayılan; çizimde konum kotsuz kalır,
  söylenir) ya da `skip` ile yazılmaz.

Özellikler yazılmaz (XYZ yalnız konumdur).

### 6. Masaüstü, Python ve MCP

CBS › Veri › Dosya alışverişi'nde GML içe aktar…, GML dışa aktar… ve XYZ dışa aktar… pencereleri:
- GML içe aktarmada komutun planı (nesne türü → katman) ve Projenin sistemine çevir;
- GML dışa aktarmada nesne türleri, geometri ve özellikleri, şema ve yazılmayanlar, şemanın yazılıp yazılmayacağı;
- XYZ dışa aktarmada seçimler komutun girdisidir; satır sayısı, kotlu satırlar ve yazılmayanlar komutun planından.

Pencerelerin komutu `file.import.gml`, `file.export.gml`, `file.export.xyz`'tir; üçü de MCP'nin yıkıcı listesindedir.

### 7. Performans

Bütçe (100 000 noktalık 62 MiB'lık dosya, makinenin tek iş parçacığı): yazma ≤ 0,5 sn, okuma ≤ 1 sn; 10 000 alan (13 MiB):
her biri ≤ 0,25 sn. Okuma belgeyi bir kez ağaca alır (roxmltree) ve nesneleri satır satır tablolaştırır. Masaüstünde dışa
aktarma iş parçacığında yapılır.

## Uygulama

- Sözleşme: `crates/shared/contracts/src/file_gml.rs`; katalog ve yetenekler `kentos-commands`'ta; TS türleri üretildi.
- Kodek ve komutlar: `gml_read.rs`, `gml_write.rs`, `gml_table.rs`, `file_import_gml.rs`, `xyz_table.rs`;
  başsız dosya komutları `file_commands.rs`'te (`export_gml`, `export_xyz`).
- Masaüstü: `exchange/gml_exchange.rs`, `exchange/xyz_exchange.rs`, testler `exchange/gml_tests.rs`.
- Arayüz kaydı `catalog/v1/additions.json` (üç komut, üç ikon: `importGml`, `exportGml`, `exportXyz`; Veri › Dosya alışverişi).

## Doğrulama

- `scripts/fixtures/gml_exchange_cases.py` (GDAL'lı Python; macOS'ta QGIS'inki):
  - dosyaları GDAL'ın GML sürücüsü QGIS 4.2.3'ün GeoPackage'ından yazar (GML 3.2 şemalı, şemasız ve `.gfs`'li; GML 3.1.1; GML 2;
    WGS 84'te enlem boylam sırası) ve elle yazılmış GML'ler (`featureMembers`, iç içe özellik, `wfs:member`, http adlı sistem,
    CRS84, sistemsiz, bos, geometri koleksiyonu, üç kesimli eğri, bozuk, kesik, GML olmayan XML);
  - 35 ortak durum (`fixtures/commands/v1/file.import.gml.json`) OGR'ın okuduğundan GeoPackage alma kuralıyla kurulur;
    OGR'dan ayrılan üç yer yukarıda ve betiğin başında (`gml_id`, boş öğe, uzunluksuz çıkarım) ve iç içe adlar;
  - `--check` ayrıca KentOS'un yazdığı dosyaları (`KENTOS_WRITE_GML=1 cargo test -p kentos-headless --test headless gmls`)
    OGR'la açar: geometriler kaynağın GeoPackage'ındakilerle eşit, alanlar ve değerler kaynağınkilerle, sistem EPSG:5256,
    WGS 84'te `pos` enlem önce; GDAL'ın dosyalarını KentOS'un okuyup yeniden yazdığı `fixtures/gml/v1/geri` OGR'da
    GDAL'ın kendi dosyalarıyla aynı;
  - XYZ: `KENTOS_WRITE_XYZ=1 cargo test -p kentos-headless --test headless xyz_files` dosyaları yazar; betik satırları
    kaynağın OGR geometrilerinin köşelerinden (kendi kuralıyla, KentOS kodu olmadan) üretip karşılaştırır.
- Birimler: `crates/native/application/tests/all/gml.rs` (her geometrinin gidiş-dönüşü dört sistemde, eksen sırası, türler ve
  şema, GML olmayanlar ve bozuk geometriler, XML adları, yay zarfı); başsız: `gmls_go_out_and_come_back`, `xyz_files_go_out`.
- Süre: `cargo test --release -p kentos-native-application --test all gml_timing -- --ignored --nocapture` (100 000 nokta:
  yazma 0,14 sn, okuma 0,30 sn; 10 000 alan: 0,05 sn ve 0,05 sn).
- Masaüstü: `cargo test -p kentos-desktop exchange::gml_tests`; resimler `exchange::gml_tests::screens`, `.run/shots/gml-*` ve
  `xyz-*`.
- GDAL'ın yan bulguları: GDAL kendi yazdığı, kotlu ve kotsuz nesne türü olan çok katmanlı GML'i okuyamıyor (boyutsuz `posList`'ler
  koleksiyon zarfının boyutuyla okunuyor); karışık Z'li çok türlü dosyada eğri geometrileri 3B'ye yükseltiyor (değerleri 0).
  Birincisi için KentOS her `posList`'e boyutunu yazar; ikincisinde karşılaştırma kaynağın boyutuna göre yapılır.
