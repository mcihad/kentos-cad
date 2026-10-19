# ADR 0514: Shapefile dışa aktarma — kayıpları söyleyerek

- **Durum:** kabul edildi (2026-10-19). `plan/gis.md` CBS-05'in ilk kalan dilimi (“Shapefile dışa aktarımı kodlama, alan
  adı/tür/boyut kayıplarıyla”); CBS-06'nın Shapefile yolu, CBS-44'ün QGIS 4.2 ile alışverişi.
- **Bağlam belgesi:** ADR 0046 (GeoJSON ve Shapefile okuma), 0222 (açık dönüşüm, ortak dışa aktarma geometrisi), 0502
  (GeoPackage), 0224 (M), 0253 §10 (dosya komutları), 0506 (Paketle'nin bütün ya da hiç yazması).
- **Web:** ertelendi (ADR 0228). Web'de gerekenler TODOS.md §0.2'de.
- **Sonraya kalan:**
  - GML, FlatGeobuf, GeoParquet ve CSV/XYZ'nin iki yönü (CBS-05'in sonraki dilimleri);
  - içe almada hazırlık alanı (staging), reddedilen satırların dosyası, doğal anahtarla güncelleme (upsert);
  - Shapefile'da kalıcı kimlik: biçimde yeri yok (FID kayıt sırasıdır); kimliği korumak isteyen GeoPackage kullanır;
  - 2 GB'ı aşan Shapefile: reddedilir, bölmek ya da GeoPackage önerilir.

## Bağlam

Kurumların ve yüklenicilerin çoğu hâlâ Shapefile ister; KentOS onu okuyabiliyor (ADR 0046) ama yazamıyordu. Biçimin
sınırları bilinir ve sessizce veri kaybettirir: bir dosya tek geometri türü taşır, eğri yoktur, sütun adı 10 bayttır,
dBASE'in türleri azdır, NULL ile boş metin ayrılmaz, kod sayfası ayrı bir dosyadadır. Plan bunların her birinin
yazılmadan önce söylenmesini, büyük sayının, tarihin, eğrinin ve Z/M'nin sessizce kaybolmamasını ister.

## Karar

### 1. Nereye: arşiv, klasör ya da tek `.shp`

`file.export.shp`'nin `path`'i üç biçimdedir:

- **`.zip`:** bütün Shapefile'lar tek arşivde (QGIS ve ArcGIS arşivi açmadan okur). Var olan arşivin üzerine yalnız
  `overwrite` ile yazılır.
- **`.shp`:** dışa aktarma tek Shapefile yapıyorsa o adla, parçaları yanında. Birden çok yapıyorsa `several_shapefiles`.
  `overwrite` ile o adın eski parçaları da (eski `.prj`, `.qix`, `.sbn`, `.sbx`, `.shp.xml`) kaldırılır: eski bir dizin
  ya da sistem yeni dosyayı yanlış anlatmasın.
- **Başka yol:** yeni ya da boş bir klasör; her Shapefile katmanın adıyla. Var olan dolu bir klasörün üzerine hiç
  yazılmaz (`file_exists`).

**Bütün ya da hiç:** önce bütün baytlar yapılır (`plan` burada durur, hiçbir şey yazmaz).

- Arşiv `files::write_bytes` ile yazılır (geçici ad, diske indirme, geri okuma, tek adımda yerine koyma).
- `.shp`'nin parçaları tek tek geçici adla yazılıp geri okunur. Eski parçalar kenara alınır, yeniler yerine konur, eskiler
  silinir. Bir yerine koyma başarısız olursa konanlar kaldırılır, eskiler geri gelir.
- Klasör yanında geçici bir klasörde kurulup tek adımda yerine konur (Paketle'nin yolu, ADR 0506).

Aynı çizim aynı baytları verir: `.dbf`'in tarihi 1 Ocak 1980'dir, zip yazıcısınınki gibi. Kayıttaki etki arşivin, ya da
dosyaların ad ve özet listesinin SHA-256'sıdır.

### 2. Shapefile'lar: katman başına, geometri türü başına

- **Tür ayrımı:** bir Shapefile tek tür taşır. Katmanın nesneleri noktalar, çizgiler ve alanlar olarak ayrılır. Birden
  çok türü olan katman `_nokta`, `_cizgi`, `_alan` ekli Shapefile'lar olur. Adlar büyük küçük harf ayırmadan tekildir
  (`_2`); dosya adında olmayan karakterler `_` olur.
- **Şekil türleri:**
  - Nokta Point, birden çok noktalı nesnesi olan küme MultiPoint olur.
  - Çizgi ve çok parçalı çizgi PolyLine, alan delikleri ve parçalarıyla Polygon olur.
  - Kotlu köşesi olan küme Z türüdür, kotsuz köşeleri 0 kotla yazılır ve söylenir. Shapefile'da kotsuz köşe yoktur.
  - Çizginin M'si taşınır: M'si olan ve kotu olmayan küme M türü, ikisi de olan Z türü ve M bloğudur. M'siz köşe
    belirtimin “veri yok”udur (−10³⁸'den küçük).
- **Halka yönü:** Shapefile'ın kuralıyla, dış sınır saat yönünde, delikler saat yönünün tersine. İlk köşe aynı kalır.
- **Geometrisiz kayıt:** boş şekil (Null Shape) satırıdır (ADR 0410). Yalnız kayıtlı katman yalnız `.dbf`'tir (ve
  `.cpg`); GDAL ve QGIS onu tablo olarak açar.
- **Eğriler** (daire, yay, elips, yaylı yol ve alan) eğriden en çok **0,1 mm** sapan kirişlerle yazılır ve sayılır:
  - Ortak dışa aktarma geometrisi (`export_geom`) Shapefile için kirişin açısını yarıçaptan bulur: 2·acos(1 − 0,1 mm / r).
  - Yaylı yolda yolun en büyük yayının yarıçapı, elipste büyük yarı eksen kullanılır. Kiriş hiçbir zaman uygulamanın
    kendi adımından (turda 72) geniş olmaz.
  - NURBS zaten 0,1 mm'lik kirişlerle yazılıyordu.
  - GeoJSON, KML ve GPX'in davranışı değişmedi: `Words.chord` onlarda yok.
- **Blok** açılır. Her geometri türü kendi Shapefile'ında bir satırdır, yerleştirmenin öznitelikleriyle.
- **Yazılmayanlar:** yazı, ölçü, tablo, resim, raster, nokta bulutu, uçsuz doğru. Türlerine göre sayılır (`left`).

### 3. `.dbf`: adlar, türler, değerler

**Sütunlar:** önce katmanın alanları kendi sıralarıyla, sonra öbür öznitelikler adlarının sırasıyla. dBASE en çok 255
sütun alır; fazlası yazılmaz ve söylenir.

**Ad:**

- `.dbf`'in kod sayfasında en çok 10 bayttır, bir karakterin sonunda kesilir. Kod sayfasında olmayan harf `_` olur.
- Sondaki boşluk atılır: okuyucular onu siler ve sonraki ad onunla çakışırdı.
- Büyük küçük harf ayırmadan tekildir; çakışan ad `_1`, `_2` … alır (gerekirse daha kısa kesilerek).
- Değişen her ad `renamed`'de söylenir.

**Türler:**

| Alan | dBASE | Kural |
|---|---|---|
| metin | C | genişlik değerlerin en uzun baytı (alanın uzunluğundan az değil), en çok 254; uzun metin karakter sınırında kesilir ve sayılır |
| tam sayı | N(w, 0) | en çok 18 hane; daha uzunu C (dBASE okuyucuları 19 haneyi kayan noktaya çevirir) |
| ondalık | N(w, d) | d değerlerin ve alanın basamaklarının en çoğu, **en az 1** (okuyucu kesirsiz N'yi tam sayı sanar); en çok 15 anlamlı basamak, daha çoğu C |
| tarih | D | `YYYYAAGG` |
| evet/hayır | L | `T`, `F`, değersiz `?` |
| tarih-saat, saat | C | `YYYY-AA-GGTSS:DD:SS`, `SS:DD:SS`; dBASE'te bu türler yok, söylenir |
| alanın kuralına uymayan değeri olan alan | C | değerler yazıldığı gibi; söylenir |
| alan olmayan öznitelik | C | — |

**Değerler:**

- Değersiz sayı, tarih ve evet/hayır boşlukla ya da `?` ile yazılır ve okuyucular onu NULL okur.
- Metin sütununda boş metin ile değerin yokluğu ayrılamaz: ikisi de boşluktur, sayılır.
- Metnin sonundaki boşlukları okuyucular siler, sayılır.

**Kod sayfası (`encoding`):**

- UTF-8 (varsayılan; `.cpg` `UTF-8`, dil sürücüsü baytı 0).
- Windows-1254 (`.cpg` `1254`, dil sürücüsü 0xCA): yalnız eski kod sayfalarını okuyan programlar için. Onda olmayan
  harf `?` olur ve sayılır.

### 4. `.prj`: ArcGIS ve QGIS'in tanıdığı biçim

- **Kayıttaki sistemler:** `.prj` ESRI biçimli WKT 1'dir, GDAL'ın `MorphToESRI`'sinin yazdığı metin
  (`fixtures/crs/v1/esri-prj.json`, `scripts/fixtures/esri_prj.py` bir kez GDAL'dan yazar ve `--check` ile denetler;
  çalışırken GDAL gerekmez).
- **Neden ESRI biçimi:** OGC biçimli WKT 1'i (`crs::text::write_wkt`) QGIS 4.2 geçerli ama adsız, kodsuz bir özel
  sistem olarak okudu, `AUTHORITY` eklenmiş olanı da; ESRI biçimini EPSG:5256 olarak tanıdı.
- **Projenin kendi tanımı** (ADR 0168): OGC WKT 1'iyle yazılır ve okuyan programın onu özel sistem tanıyacağı
  söylenir.
- **Yerel proje:** `.prj` yazılmaz ve söylenir.

### 5. Komut, araç ve otomasyon

- **`file.export.shp`** başsız sunucunun dosya komutları arasındadır (`plan` yazmadan söyler, `execute` yazar). Çizimi
  değiştirmez, geri alma adımı değildir.
  - Retler sırasıyla: `invalid_file_path`, `file_exists`, `layer_not_found`, `nothing_to_export`, `several_shapefiles`
    (yeni kod), `file_unwritable`.
  - Uyarı `export_losses`: yazılamayanları ve değişenleri cümle cümle söyler.
  - Çıktıda Shapefile'lar (ad, katman, satır, şekil türü), dosyalar (ad, bayt, SHA-256), `prj`, `left`, `approximated`,
    `renamed`, `asText`, `cut`, `replaced`, `blankText` ve yazıcının bütün raporu bulunur.
- **Katmanlar:** hesap biçim çekirdeğinde (`kentos_formats::shp::write`), çizimden girdi uygulama katmanında
  (`kentos_native_application::shapefile`), hedef ve yazma başsız sunucuda (`kentos_headless::shp_export`).
- **Python ve MCP:** Python'da `kentos.cad.file.export.shp` katalogdan üretilir. MCP'de araçtır; var olan dosyanın
  üzerine yazabildiği için yıkıcı listededir.
- **Masaüstü:** CBS › Veri › Dosya alışverişi'nde **Shapefile dışa aktar…** (ikon: sayfa, dışa giden ok, nokta, çizgi
  ve alan; üç seçenekten B). Pencere yazılmadan önce gösterir:
  - Shapefile'ları türleri ve satırlarıyla;
  - kayıpları;
  - Kod sayfası ve Hedef seçimlerini (Tek arşiv, Yeni klasör, tek Shapefile; sonuncusu dışa aktarma tek Shapefile
    yapıyorsa).

  **Dışa aktar…** yolu sorar ve komutla yazar.
- **İşlem manifesti:** `semantic`, `via` `file.export.shp`.

### 6. Okuyucudaki düzeltme

KentOS'un Shapefile okuyucusu PointZ'nin, MultiPointZ'nin ve alanların M bloğunu yalnız “veri yok”tan büyük bir değer
varsa “M atıldı” diye sayar. Belirtime uygun yazılmış PointZ'nin −10³⁸'den küçük M'si ölçü değildir; KentOS'un
kendi dosyası okunurken boş bir uyarı çıkmasın.

## Doğrulama

- **Başsız sunucuda uçtan uca** (`shapefiles_go_out_whole_with_what_they_cannot_hold_said`). Çizim QGIS 4.2.3'ün
  GeoPackage'ından alınır; üstüne nokta katmanına bir üçgen, yol katmanına 25 m'lik bir daire, uzun Türkçe adlı iki
  öznitelik ve bir yazı eklenir.
  - `plan` yazmaz, `execute` aynı çıktıyı verir.
  - Yedi Shapefile türleri ve satırlarıyla çıkar: nokta katmanı ikiye ayrılır, malik `.dbf` olarak yazılır.
  - Yazı sayılır, üç eğri kirişlenir, `.prj` vardır.
  - KentOS'un kendi okuyucusuyla geri okunan parsel değerleri ve yolun M'si [0, 50, 101] doğrudur.
  - Windows-1254 arşivi yazılır, tek `.shp` yazılır; `several_shapefiles`, `file_exists` ve `layer_not_found` retleri
    sınanır.
  - `KENTOS_WRITE_SHP=1` klasörü `fixtures/shapefile/v1/out`'a koyar. İki koşuda baytlar aynıdır.
- **Bağımsız denetim** (`scripts/fixtures/shp_export_cases.py`). Betik KentOS kodu kullanmaz. Kaynağı GDAL'ın OGR'ıyla,
  çıktıyı OGR'la ve kuruluysa QGIS 4.2'nin kendi Python'uyla okur; bu ADR'nin kurallarını yeniden yazarak denetler:
  - Shapefile'ların adları;
  - `.dbf` başlığı (sürüm 3, 1980-01-01, sürücü baytı), `.cpg`;
  - sütun adları ve türleri, değerler (boş metin değersiz);
  - halkalar: başlangıç ve yönden bağımsız aynı köşeler, dış sınır saat yönünde, delikler tersine, geçerlilik;
  - düz yolların Z ve M'si ISO WKT olarak aynı;
  - eğrilerde kirişlerin her köşesi ve ortası eğriye, eğrinin sık örneklemi kirişlere en çok 0,1 mm;
  - dairenin köşeleri daire üstünde, kirişlerinin sapması 0,1 mm'den az;
  - `.prj` EPSG:5256 olarak tanınır; QGIS 4.2 yedi Shapefile'ı açar ve sistemi EPSG:5256 okur;
  - Windows-1254 arşivinde Türkçe değerler, `.cpg` `1254` ve sürücü 0xCA.
  - Özeti `fixtures/shapefile/v1/expected.json`'dadır.
- **Python** (`python/tests/test_shapefile.py`): Windows-1254 arşivi; `plan`, uyarı, şekil türleri, sütun adları ve
  türleri, Türkçe değerler yalnız standart kitaplıkla okunur; `file_exists`.
- **Resimler:** `shp-*` (Shapefile dışa aktar penceresi).

## Sonuçlar

- KentOS kurumların istediği Shapefile'ı QGIS ve ArcGIS'in tanıdığı sistemle yazar.
- Biçimin taşıyamadığı her şey (tür ayrımı, eğri, ad, tür, boş metin, kod sayfası, kot, sistem) yazılmadan önce
  söylenir; büyük sayı, ondalık, tarih ve M sessizce kaybolmaz.
- Kalıcı kimlik Shapefile'da yoktur. Kimliği korumak gereken iş GeoPackage'la yapılır.
