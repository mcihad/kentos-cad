# ADR 0204: Raster katmanları: GeoTIFF, oturtma, bantlar ve gölgeli kabartma

- **Durum:** kabul edildi (2026-10-08). Kapsam sahibin kararlarıdır (8 Ekim, dördü de önerilen seçenek): GeoTIFF ile dünya dosyalı PNG
  ve JPEG, yeni bağımlılık eklemeden kendi okuyucumuzla; kontrol noktalarıyla oturtma bütün dönüşümleriyle; bantlar, renk rampası ve
  gölgeli kabartma; bağlı dosya, küçükler gömülebilir. Madde tek parçada biter.
- **Bağlam belgesi:** TODOS.md `GIS-08` (ve araştırma notu), ADR 0192 (resim nesnesi), ADR 0156 ve 0158 (Vektör oturtma, Kauçuk
  levha), ADR 0046 (GIS biçimleri: dosyanın sistemi projeninkinden başkaysa içe aktarma kapalı), ADR 0157 (uzak karolar), ADR 0025
  (`.kcad` v2), ADR 0009 (dosya biçimleri: bozuk girdide panik yok, sınırlar).

## Bağlam

Ortofoto, taranmış pafta ve sayısal yükseklik modeli (DEM) harita mühendisliğinin altlığıdır. Resim nesnesi (ADR 0192) PNG ve
JPEG'i çizimin içinde bir çerçeveye yerleştirir ama coğrafi konumu, bantları, nodata'yı ve büyük dosyayı bilmez: en çok 4096 piksel
gösterir. Netcad (Raster Yükle, OTOREG, Raster Düzenle), ArcGIS (Add Raster, Georeferencing, Stretch, Hillshade) ve QGIS (raster
katmanı, Georeferencer, Singleband pseudocolor, Hillshade) rasteri coğrafi konumlu, piramitli, karolarla okunan bir katman olarak
tutar. Bu ADR KentOS'un raster nesnesini, okuyucusunu, piramidini, görünüşünü ve oturtmasını tanımlar.

## Karar

### 1. Kaynaklar

- **GeoTIFF ve TIFF** (klasik ve BigTIFF, küçük ve büyük uçlu): şeritli ya da karolu; sıkıştırmasız, LZW, Deflate (8 ve 32946),
  PackBits ve JPEG (7); öngörücü yok, yatay (2) ya da kayan nokta (3); bant başına 8, 16, 32 bit tam sayı (işaretli ya da işaretsiz),
  32 ve 64 bit kayan nokta; bantlar iç içe (chunky) ya da ayrı (planar); fotometrik siyah sıfır, beyaz sıfır, RGB, palet ve (JPEG'le)
  YCbCr; ek bant alfa. İç önizlemeler (indirgenmiş çözünürlüklü IFD'ler) boyları ⌈w / 2ᵏ⌉ × ⌈h / 2ᵏ⌉ ise piramidin katlarıdır.
  1, 2 ve 4 bitlik bantlar, CMYK, CIELab ve başka sıkıştırmalar nedeniyle reddedilir.
- **Coğrafi konum (GeoTIFF):** ModelTransformation ya da ModelPixelScale ile tek ModelTiepoint pikselden dünyaya afin dönüşüm verir;
  PixelIsPoint ise yarım piksel kaydırılır. Koordinat sistemi GeoKey'lerden: ProjectedCSType ya da GeographicType'ın EPSG kodu. Kod
  yoksa ya da kullanıcı tanımlıysa sistem “bilinmiyor” sayılır. GDAL_NODATA etiketi nodata'dır.
- **PNG ve JPEG:** PNG çekirdekte çözülür (bütün renk türleri ve bit derinlikleri, Adam7, tRNS; 16 bitlik gri DEM olarak okunur);
  JPEG platformun çözücüsüyle (masaüstünde zune-jpeg, web'de tarayıcı). Yanındaki dünya dosyası (.pgw, .pngw, .jgw, .jpgw,
  .jpegw, .tfw, .tifw, .wld) afin dönüşümü verir: altı satır A, D, B, E, C, F (C ve F sol üst pikselin ortası). En çok 64 milyon
  piksel; büyüğü GeoTIFF'e çevrilmesi söylenerek reddedilir. TIFF'in yanında dünya dosyası varsa GeoTIFF'in dönüşümü yokken kullanılır.
- **Koordinat sisteminin kuralı (ADR 0046):** dosyanın sistemi projeninkinden başkaysa raster eklenmez, iki sistem adlarıyla söylenir
  (dönüşüm yok). Sistemi bilinmeyen raster (dünya dosyası, kodsuz GeoTIFF) ancak kullanıcı “projenin sisteminde” diye onaylarsa eklenir.
  Konumu olmayan raster (dünya dosyası da yok) oturtulmamış olarak görünümün ortasına, pikseli görünümün kısa kenarının 1/1000'i olacak
  biçimde yerleşir; Raster oturt ile oturtulur.
- **Sınırlar:** dosyanın açılışında IFD sayısı, etiket sayısı ve değer boyları sınırlıdır; karo ya da şerit en çok 64 MB açılır; her
  biçim hatası nedeniyle söylenir, panik yok.

### 2. Nesne

Yeni nesne türü `raster`:

- `affine`: altı sayı `[x₀, a, b, y₀, c, d]`: sütun i ve satır j (sol üst köşe 0, 0; piksel köşeleri) için x = x₀ + a·i + b·j,
  y = y₀ + c·i + d·j (GDAL'ın geotransform sırası). Tersinir olmalı.
- `width`, `height` (piksel, 0'dan büyük), `bands` (1–255), `sample`: `u8`, `i8`, `u16`, `i16`, `u32`, `i32`, `f32` ya da `f64`.
- Kaynak ikisinden biri: `file` (bağlı: dosya yolu) ya da `asset` (gömülü: projenin kitaplığında, ADR 0192'nin kuralıyla, en çok 32 MB).
- `srid`: dosyanın sistemi (0: bilinmiyordu, kullanıcı projeninki diye onayladı).
- `style` (§4) ve `opacity` (0,1–1; yoksa 1). `.kcad` şema 29.

Rasterin piksel değerleri `.kcad`'e yazılmaz; yalnız kaynağı. Raster nesnesi kendi katmanında durur: Raster ekle dosyanın adıyla yeni
katman açar (aynı adımda).

### 3. Piramit, karolar ve önbellek

- **Karolar:** 256 × 256 piksel; k. katın boyu ⌈w / 2ᵏ⌉ × ⌈h / 2ᵏ⌉, son kat tek karoya sığan kattır.
- **Katlar:** 0. kat dosyanın kendisidir; üst katlar dosyanın iç önizlemeleri ya da, yoksa ve raster bir kenarında 4096 pikselden büyükse,
  bir kez hazırlanan **önizleme piramidi**dir: kaynak satır satır bir kez okunur, her kat bir öncekinin 2 × 2 ortalamasıdır (nodata
  sayılmaz; paletli raster sol üst pikselini alır). Piramit cihazın önbelleğine karolu, Deflate'li bir GeoTIFF olarak yazılır (masaüstünde
  `$XDG_CACHE_HOME/kentos-cad/raster/<anahtar>.tif`, web'de IndexedDB `kentos.rasters`); anahtar dosyanın yolu (web'de adı), boyu ve
  değişme zamanından SHA-256'dır. Çizimin sağ altındaki panel (büyük içe aktarmanınki gibi) ilerlemeyi gösterir, Durdur'la bırakılabilir
  (raster o zaman yalnız 0. kattan çizilir). Bir dosyanın geçişi bir kez yürür: geçiş sürerken raster çizimden çıkıp geri gelirse (geri
  alma ve yineleme) yeni kaynağı aynı geçişi bekler ve katlarını ondan alır.
  4096 pikselden küçük rasterin üst katları gerektiğinde bellekte hesaplanır.
- **Okuma:** yalnız gerekli karolar ve şeritler okunur (web'de dosyanın dilimleri); çözülmüş bloklar sınırlı bir bellekte (masaüstünde
  raster başına 96 MB, web'de işçi başına raster başına 48 MB) en az kullanılan önce bırakılarak tutulur; renklenmiş karolar ayrıca
  64 MB'lık bir bellekte (yaklaşık 240 karo).

### 4. Görünüş

`style`'ın alanları:

- `render`: `rgb` (üç bant kırmızı, yeşil ve maviye; isteğe bağlı alfa bandı), `gray` (tek bant gri), `ramp` (tek bant renk rampası),
  `hillshade` (tek bant gölgeli kabartma) ya da `rampShade` (rampa gölgeli kabartmayla çarpılır). Varsayılan: üç ya da dört bantlı
  8 bit `rgb`, paletli raster paletiyle (`gray` yerine `palette`), tek bantlı 8 bit `gray`, öbür tek bantlılar (DEM) `rampShade`.
- `bands`: bant numaraları (1'den), türün istediği kadar.
- `stretch`: `minmax` (bandın istatistiğinin en küçüğü ve en büyüğü), `percent` (%2 ile %98) ya da `manual` (`min`, `max`); 8 bit
  `rgb`'de varsayılan gerdirme yoktur (`none`). İstatistik bir kenarı 1024 pikseli geçmeyen en ince kattan, nodata dışarıda, yaklaşık.
- `ramp`: `Gri`, `Arazi`, `Spektral`, `Viridis`, `Mavi-kırmızı`, `Sıcaklık`; durakları eşit aralıklı sRGB renkler, aralarında doğrusal;
  `invert` ters çevirir.
- Gölgeli kabartma: Horn yöntemi (gdaldem hillshade'in varsayılanı): `azimuth` (derece, kuzeyden saat yönünde, 315), `altitude`
  (derece, 45), `zFactor` (1); kenarda en yakın piksel tekrarlanır. Üst katlarda o katın piksel boyuyla.
- `rampShade`: renk = rampa × (1 − 0,6 + 0,6 · gölge / 255).
- `nodata`: dosyanınkinin yerine geçen değer (yoksa dosyanınki); nodata ve NaN saydamdır.
- `resampling`: `bilinear` (varsayılan) ya da `nearest`; büyütmede ve katlar arasında.

Karonun renkleri çekirdekte hesaplanır (iki platform aynı baytları üretir); GPU yalnız karoyu çizer.

### 5. Çizim

- Raster, resim gibi belge sırasında çizilir (stil motorunda `MODE_RASTER`): çerçevesi nesnenin renginde ince çizgiyle, alanı raster
  boyasıyla. Raster boyası durağandır; çizim hattı her karede görünen karoları kendisi seçer: görünümün raster pikseline düşen kısmından,
  bir aygıt pikseline bir kat pikseli düşecek kat (⌊log₂ (1 / (s · aygıt pikseli/metre))⌋, 0 ile son kat arası; s piksel boyu).
- Karolar bir doku atlasında (aygıt izin veriyorsa 8192 × 4096, 465 yuva; yoksa 4096 × 4096, 225 yuva; 258 × 258'lik yuvalar: karonun
  her yanında komşusundan bir piksel, çift doğrusal süzgeçte dikiş olmasın) en az kullanılan önce bırakılarak durur; bu karede çizilen
  yuva bırakılmaz, karede en çok 32 karo yüklenir. Henüz gelmemiş karonun yerine yüklü en yakın üst katın parçası çizilir; son kat
  ilk istenir. Karolar ev sahibinin iş parçacığında (masaüstü) ya da işçide (web) hazırlanır.
- Stilli çizim hattının sözleşmesi 6. sürüm: raster karoları kendi boru hattıyla (konum ve atlas koordinatı köşelerde, saydamlık).

### 6. Oturtma

**Raster oturt** penceresi: rasteri (Sahneden seç), kontrol noktaları tablosunu (Kaynak: sütun ve satır, rasterin üstünde tıklanarak;
Hedef: Y ve X, çizimde tıklanarak, kenetle ya da yazılarak; Kullan; artık), dönüşümü ve sonucu gösterir.

- Dönüşümler ve en az nokta: Helmert (2), Afin (3), Projektif (4), Polinom 2 (6), Polinom 3 (10), İnce plaka (3, doğrusal olmayan).
  Helmert, afin ve projektif Vektör oturtma'nın çözümleridir (ADR 0156); polinomlar en küçük kareler (sütun ve satır merkezlenip
  ölçeklenerek); ince plaka Kauçuk levha'nın eğrisidir (ADR 0158), noktalardan tam geçer.
- Artıklar hedefte (metre) ve m0 = √(Σ v² / (2n − u)) (u: 4, 6, 8, 12, 20); ince plakada artık yoktur.
- **Uygula:** Helmert ve afin rasterin `affine`'ini değiştirir (tek adım “Raster oturt”); istenirse yanına dünya dosyası yazılır
  (masaüstünde dosyanın yanına, web'de indirilir). Projektif, polinom ve ince plaka **yeniden örnekler**: kuzeye bakan çıktı ızgarası
  hedefteki kapsamı örter, piksel boyu varsayılan olarak dönüşümün kaynak noktalarındaki ortalama ölçeğinden; her çıktı pikselinin
  merkezi ters dönüşümle kaynağa götürülür (Helmert, afin ve projektifte kapalı biçim; polinom ve ince plakada ileri dönüşümün Newton
  yöntemiyle tersi, hedeften kaynağa ayrı kurulan dönüşümün tahmininden başlayarak: gönderilip geri getirilen piksel yerinde kalır;
  GDAL ince plakanın tersini böyle inceltir, polinomlarınkini ayrı kurulan dönüşümde bırakır), ters dönüşüm her 8 çıktı pikselinde bir
  tam, aralarda çift doğrusal (hücrenin ortası 0,02 pikselden çok sapıyorsa hücre tam hesaplanır), ve en yakın ya da çift doğrusal
  örneklenir; kapsam dışı pikseller 8 bitlik rasterde alfa bandıyla, öbürlerinde
  nodata ile (dosyanınki, yoksa kayan noktada NaN, tam sayıda türün en küçüğü) boş kalır. Çıktı karolu, Deflate'li GeoTIFF'tir
  (masaüstünde bağlı kaynağın yanına `<ad>-oturtulmus.tif`, gömülü kaynağınki 32 MB'a kadar gömülür; web'de indirilir ve 32 MB'tan
  küçükse gömülür, büyüğü oturumun bağlı dosyası olur); raster çıktıyı gösterir.

### 7. Seçme, kenet ve düzenleme

Raster resim gibi yalnız çerçevesinden seçilir; kenet köşelerinde ve kenar ortalarında. Tutamacı yoktur (coğrafi konumu yanlışlıkla
bozulmasın). Taşı, Kopyala, Döndür, Ölçekle, Aynala, Hizala ve Vektör oturtma'nın benzerlik ve afin dönüşümleri `affine`'i birleştirerek
değiştirir; projektif ve Kauçuk levha rasteri almaz (Raster oturt söylenir). Patlat, Ötele ve benzerleri almaz.

### 8. Araçlar ve pencereler

- **Raster ekle** (CBS'de Veri › Raster, CAD'de Ekle › Raster): dosya seçilir; pencere boyu, bantları, türü, sistemi, nodata'yı, konumun
  kaynağını (GeoTIFF, dünya dosyası, yok) ve kuralın sonucunu gösterir; Ekle dosyanın adıyla yeni katmanı ve rasteri tek adımda yazar
  (“Raster ekle”) ve rastere yakınlaşır. Masaüstünde Bağlı (varsayılan) ya da Göm, dünya dosyası rasterin yanında aranır; web'de raster
  ve dünya dosyası birlikte seçilir (tarayıcı klasörü görmez), dosya oturum boyunca okunur, 32 MB'tan küçükse Göm seçilebilir.
- **Raster stili** (şeritten ve rasterin Öznitelikler'inin Görünüş satırından): türü, bantlar, gerdirme (istatistikleriyle), rampa
  (geniş örnekleriyle), gölgeli kabartmanın üç değeri, nodata, saydamlık, örnekleme; çizimde canlı önizleme; Uygula tek adım
  (“Raster stili”), Vazgeç iz bırakmaz.
- **Raster oturt** (§6).
- **Öznitelikler:** Kaynak (gömülü adı ve boyu; bağlı yolu ve bulunup bulunmadığı), Boyut (piksel), Bantlar ve tür, Piksel boyu, Sistem,
  Nodata, Saydamlık; Göm (bağlıda); web'de bulunamayan bağlı rasterde **Kaynağı yeniden seç**.
- **Koordinat oku**, noktanın altındaki rasterlerin değerlerini de yazar (“Raster ‹katman›: 852,31”, bant bant), okununca.
- Önizleme piramidinin hazırlığı ve Raster oturt'un yeniden örneklemesi çizimin sağ altındaki panelde ilerleme çubuğu ve Durdur'la.

### 9. Komutlar

`cad.entities.create`'in `raster` geometrisi, işlem `raster` (adım “Raster ekle”); `cad.entities.edit`'in `rasterStyle` (“Raster stili”)
ve `rasterGeoref` (“Raster oturt”) işlemleri (güncelleme ya da kaynağı değişen raster için değiştirme). Ret kodu `invalid_raster`
(tersinmeyen dönüşüm, boy, bant, tür, kaynak, stilin bantları, saydamlık); gömülü kaynak `unknown_asset`; blok tanımında
`raster_in_block`.

### 10. Biçimler

DXF'e ve GeoJSON'a raster yazılmaz, raporda söylenir. PostGIS izdüşümü rasterin çerçevesinin çokgenidir.

### 11. Performans

Sahibin ilkesi (8 Ekim): “Performance First”.

- Çözme, piramit, gerdirme ve gölgeli kabartma hiçbir zaman arayüzün iş parçacığında yapılmaz: masaüstünde iş parçacığı havuzu
  (çekirdek sayısının bir eksiği, en az 1, en çok 4), web'de raster işçileri (2). Piksel verisi işçiden aktarılan (transfer) tipli
  dizilerdir; JSON'a girmez.
- İstekler önce son katın karosu, sonra görünen karolar görünümün ortasından dışarı doğru; görünümden çıkan karonun isteği
  bırakılır (kuşak sayacı). Bir blok bir kez açılır, önbellekte tutulur.
- Karede bellek ayırma yok: görünen karoların listesi ve köşe arabelleği yeniden kullanılır; her raster tek çizim çağrısıdır.
- Sık yollar hızlı: 8 bit RGB(A) karolar örnek örnek değil satır satır kopyalanır; LZW bit okuyucusu kelime kelime çalışır.
- Bütçeler (release, geliştirme makinesi): 256 × 256'lık Deflate'li 8 bit RGB karonun açılıp çizilecek renklere çevrilmesi ≤ 3 ms,
  gölgeli kabartmalı 32 bit DEM karosununki ≤ 6 ms; rasterin eklenmesinden ilk görüntüye (son kat) ≤ 150 ms; kaydırma ve
  yakınlaştırma kareyi düşürmez. Ölçüler Doğrulama'dadır.

## Kapsam dışı

Rasterin yeniden izdüşümü (başka sistemdeki raster), uzaktaki (HTTP) COG ve büyük veri sağlayıcıları (`GIS-09`), JPEG 2000, ECW ve
MrSID, birden çok dosyanın mozaiği, rasterin düzenlenmesi (boyut, kenar silme, bölme, gri, renk, histogram), başka bir rastere göre
otomatik ve paftaları adından toplu oturtma, görünürlük alanı ve kenarlaştırma, eğim, bakı ve öbür yüzey analizleri (`GIS-31`), raster
hesaplayıcı (`GIS-33`), rasterin pafta çıktısı (§16.4).

## Uygulama

- **Sözleşme** `crates/shared/contracts/src/raster.rs`: `RasterEntity`, `RasterFields` (`problem`), `RasterStyle` (`problem`, JSON
  metni), `RasterSample` (`label`), `RasterRender`, `RasterStretch`, `RasterResampling`, `RASTER_RAMPS` ve sınırlar; `Entity::Raster`,
  `EntityGeometry::Raster`, `CreateOperation::Raster`, `EditOperation::RasterStyle` ve `RasterGeoref`; `FORMATS_VERSION` 39.
- **Biçim çekirdeği** `crates/shared/formats/src/raster/`: `tiff` (klasik ve BigTIFF, sınırlar), `geotiff` (dönüşüm, GeoKey'ler,
  GDAL_NODATA), `layout` ve `codec` (LZW, Deflate, PackBits, öngörücüler, JPEG bloğunun akışı), `png`, `world`, `samples`, `source`
  (`Reader`: katlar, blok istekleri, bloklar en az kullanılan önce, karonun parçaları, istatistik, örnek değer), `stats`, `style` (bantlar,
  gerdirme, rampalar, Horn'un gölgeli kabartması, premultiplied 258 × 258), `pyramid` (tek geçişte önizleme piramidi), `write` (karolu,
  Deflate'li GeoTIFF akışı), `warp` (çıktı ızgarası, düğümlü ters dönüşüm, `Job`: yeniden örnekleme karo karo), `place` (Raster ekle'nin
  sistem kuralı ve oturtulmamış yer). JPEG platformun: masaüstünde zune-jpeg, web'de tarayıcı.
- **Geometri çekirdeği**: `geom/raster.rs` (çerçeve, köşeler, dönüşümle birleştirme, piksel, katlar, `level_for`, `visible`),
  `ops/georef.rs` (altı dönüşüm, artıklar, m0, Newton'lu ters; işlem `rasterGeoref`), `ops/rubber.rs`'in `map_and_jacobian`'ı; deponun
  çizim, kenet, seçme, Genel bakış ve paket kayıtları (paket türü 20), düzenlemelerin raster kuralları (`transform`, `warp`, `explode`,
  `split`, `stretch`, `grips`, `edges`, `compare`, `spatial_query`).
- **Stil motoru** `MODE_RASTER` ve `FillPaint::Raster` (`style/build.rs`, `prim.rs`, `batch.rs`); stilli çizim hattının sözleşmesi 6. sürüm
  (`shaders/wgsl/styled/fill.wgsl`'in `rasterVs`, `rasterFs`'i; `styled.layout.json`).
- **`.kcad` şema 29**: kodek `crates/shared/kcad` (nesne türü 17, sütunlarda görünüş alan alan), `docs/specs/kcad-v2.md`, bağımsız okuyucu
  `tools/kcad/kcad.py` ve yazıcı `scripts/fixtures/kcad_v2_reference.py`; `fixtures/kcad/v2/rasters.kcad`, 17 bozuk dosya
  (`broken/raster-*.kcad`).
- **Ürün komutları** iki platformda: `cad.entities.create`'in `raster` işlemi (adım “Raster ekle”), `cad.entities.edit`'in `rasterStyle`
  ve `rasterGeoref`'u, retler `invalid_raster`, `unknown_asset`, `raster_in_block`; dönüşümler afini birleştirir, projektif ve Kauçuk
  levha reddeder. Sunucunun izdüşümü çerçevenin çokgeni (`crates/server/application/src/cad.rs`). DXF ve GeoJSON raporla atlar.
- **Masaüstü**: `rasters/tiles.rs` (kaynaklar anahtarla, işçiler çekirdek sayısının bir eksiği en çok 4, kuşakla düşen istekler, okuyucu
  kilidi yalnız blok listesi ve saklamada, karolar 64 MB, `ready` aboneliği), `rasters/pyramid.rs` (önbellek dosyası, anahtar başına tek
  geçiş), `rasters/jobs.rs` (yeniden örnekleme ve sağ alttaki panel), `rasters/add.rs` (Raster ekle), `rasters/look.rs` (Raster stili:
  geri alma grubu içinde önizleme, Uygula'da tek adım), `calc/raster_fit.rs` (Raster oturt, Hesap tablosuyla), `properties/rows/raster.rs`
  (Öznitelikler), Koordinat oku'nun değerleri (`ViewChange::RasterValues`); çizim hattı `crates/render/wgpu/src/styled/raster_tiles.rs`
  (atlas, görünen karolar, üst kattan yedek, raster başına tek çizim). Kitaplıkta `tiff` varlık; Kullanılmayanları temizle ve çizimler
  arası alışveriş rasterin gömülü dosyasını sayar ve taşır, Dosyadan blok rasteri resimlerle sayıp almaz (`exchange_cases.py` de).
- **Web**: `io/rasterWorker.ts` (iki işçi; formats WASM'ının `RasterOpening`, `RasterFile`'ı dosyanın dilimleriyle; JPEG tarayıcının;
  piramit IndexedDB `kentos.rasters`'ta; yeniden örnekleme), `io/rasterProtocol.ts`, `render/rasterService.ts` (oturumun dosyaları, kuyruk,
  karo belleği, istatistik, değer, piramit ve oturtmanın ilerlemesi), `render/rasterPass.ts` (iki çizim hattının ortak yarısı; görünen
  karolar geometri WASM'ının `rasterTiles`'ından), WebGL2 (`RASTER_VS`, `RASTER_FS`, iki örnekleyici) ve WebGPU (`raster` boru hattı)
  çiziciler; pencereler `ui/raster/RasterAddDialog.ts`, `RasterStyleDialog.ts`, `RasterGeorefDialog.ts`, `RasterJobs.ts`, rampalar
  `ui/raster/ramps.ts`; Öznitelikler `ui/properties/rasterRows.ts` (Kaynağı yeniden seç, Göm); Koordinat oku `tools/coordinateTool.ts`;
  kurallar `model/rasterRules.ts`, `.kcad` okuması `model/snapshot.ts`, sütunlar `io/columns.ts`, paket `wasm/pack.ts`.
- **Komutlar ve şerit**: `raster.add`, `raster.style`, `raster.georef` (`app/rasterCommands.ts`); CAD'de Ekle › Raster, CBS'de Veri ›
  Raster; simgeler `rasterAdd`, `rasterStyle`, `rasterGeoref` (sahibin seçtiği A'lar). Python SDK'sının tipleri katalogdan.

## Doğrulama

- Okuma: `scripts/fixtures/raster_cases.py` (KentOS kodu olmadan; GDAL ve PIL'in yazdığı 21 dosya: karolu ve şeritli, sıkıştırmasız, LZW,
  Deflate (öngörücülerle), PackBits, YCbCr JPEG, 8–64 bitlik türler, chunky ve planar, palet, büyük uçlu, BigTIFF, iç önizlemeler,
  PixelIsPoint, dönük ve coğrafi; PNG'ler: dünya dosyalı 8 ve 16 bit, saydam paletli, Adam7, 4 bit gri) `fixtures/raster/v1/cases.json`:
  her katın örnekleri FNV ile, istatistikler, karoların renkleri; gölgeli kabartma ve rampa gdaldem'le karşılaştırılır (iç piksellerde
  en çok bir adım; 8 558 pikselde fark 0). Çekirdeğin `tests/all/raster.rs`'i hepsini geçer.
- Oturtma: `scripts/fixtures/raster_georef_cases.py` (GDAL'ın GCP dönüştürücüsü afin, polinom ve ince plaka için; Helmert ve projektif
  numpy ile; tersler GDAL'ın ileri dönüşümünde Newton'la) `fixtures/raster/v1/georef.json`: artıklar 1e-6, m0 göreli 1e-9, yoklamalar ve geri
  yoklamalar 1e-6 içinde; ince plakayla yeniden örneklenen raster örnek örnek aynı ve gdalwarp'la çapraz denetlenmiş; düğümlü çift doğrusal
  tersin tam tersten sapması en çok 0,02 piksel. `a_resampling_job_writes_the_tiles_it_works_out`: `warp::Job`'un GeoTIFF'i geri okununca
  bütün karoları, yeri ve sistemiyle hesaplananın aynısı.
- Piramit: `raster_pyramid.rs` (16 bit nodata'lı ve 32 bit NaN delikli 2101 × 1099 raster, 173 satırlık parçalarla; katlar okuyucunun
  kendi hesabıyla aynı; sırasız satırlar reddedilir); masaüstünde 4200 × 64'lük rasterin kaba katı piramit dosyası bir kez yazılınca gelir
  (`rasters::tests`).
- `.kcad` şema 29: `crates/shared/kcad/tests/all/rasters.rs`, bağımsız Python okuyucusu ve yazıcısı (335 dosya), 17 bozuk dosya; web'in
  sütunları Rust'ınkiyle aynı (`io/kcad.wasm.test.ts`).
- Komutlar: `fixtures/commands/v1` (`cad.entities.create`, `.edit`, `.transform`, `cad.blocks.define`, `.edit`) bağımsız Python
  başvurularıyla; iki platformda geçer.
- Kural: `raster::place`'in testleri (aynı sistem, başka sistem, yerel proje, bilinmeyen sistem ve onay, oturtulmamış yer).
- Masaüstü (`rasters::tests`): Raster ekle yeni katmanı ve rasteri tek adımda yazar, geri alma ikisini birden alır; sistemini söylemeyen
  dosya onaysız eklenmez, onayla dünya dosyasının yerine; Raster stili çizimde gösterir, Vazgeç adım bırakmaz, Uygula “Raster stili”
  adımıdır; Raster oturt afini tek adımda yazar, projektifte bağlı dosyanın yanına `tarama-oturtulmus.tif`'i yazıp raster onu gösterir (bu
  test, pencere kapandıktan sonra gelen sonucun düştüğünü yakaladı); karo servisi karoları yapar; geçiş sürerken raster bırakılıp geri
  verilse de piramit dosyası bir kez yazılır. Web: `render/rasterPass.wasm.test.ts` (yuvalar, karoların
  dörtgenleri ve üst kattan yedek), `ui/raster/ramps.test.ts` (pencerelerin rampaları çekirdeğin duraklarıyla aynı).
- Resimler `arac-raster-*` (masaüstünde `tools_screens`, sahne `raster_scenes.rs`; web'de `shots.mjs rasters`, WebGL2 ve WebGPU) ortak
  `fixtures/interaction/v1/rasters.kcad` üzerinde (KentOS kodu olmadan `scripts/fixtures/raster_scene.py`): vadi, ortofoto, DEM, Raster
  ekle (iki dosya), Raster stili, Raster oturt; 1440 × 900 ve 1100 × 650, iki tema. Web'in iki sahnesi işçinin işlerini tarayıcıda
  sınar ve koşulu sağlanmazsa düşer: `raster-oturt-projektif` (yeniden örnekleme işçide, sonuç gömülür, raster onu çizer) ve
  `raster-piramit` (4200 × 300'lük önizlemesiz raster: kaba katlar piramidi bekler, dosyası IndexedDB'ye yazılır, raster ondan çizilir).
- Süreler (release, geliştirme makinesi, `raster_timing`): Deflate'li 8 bit RGB karo 1,11 ms (bütçe 3), 32 bit DEM'in gölgeli kabartması
  2,66 ms (bütçe 6), rampa ve gölge 3,76 ms; önizlemesiz 2048 × 2048'in ilk görüntüsü 140,6 ms (bütçe 150); 6000 × 6000 RGB'nin piramit
  geçişi 0,67 s (161 MB/s), dosyası 1,7 MB; 30 noktalı ince plakayla bir çıktı karosu 6,28 ms (her pikselde tam ters 76,52 ms). Çizim
  hattının karesi raster varken ölçülmedi (karolar ev sahibinin iş parçacıklarında hazırlanır, karede en çok 32 karo yüklenir).
