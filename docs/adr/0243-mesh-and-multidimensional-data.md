# ADR 0243: Mesh ve çok boyutlu veri

- **Durum:** kabul edildi (2026-10-10). Kapsam sahibin sözleridir (10 Ekim): “GIS-37, 38, 42, 43 maddelerini tamamla”, “Yeni branch
  içinde yap bunları”; önceki kararları: “yüksek performans ilk önceliğimiz”, simgeler sorulmadan seçilir, iki platform; yeni bağımlılık
  sahibin onayıyla (CLAUDE.md §3): NetCDF ve mesh okuyucuları KentOS'un kendi kodudur. Madde tek parçada biter; dal `gis-37-38-42-43`.
- **Bağlam belgesi:** TODOS.md `GIS-43` (öncelik düşük), ADR 0204 (raster katmanları: okuyucu, katlar, karolar, görünüş), ADR 0231 ve
  0233 (raster çözümleme işi, Raster hesaplayıcı'nın ifade dili), ADR 0210 (zaman ve zaman sürgüsü), ADR 0100 (ifade dili).

## Bağlam

Hidrodinamik ve taşkın modellerinin (TELEMAC, D-Flow FM, HEC-RAS 2B, TUFLOW, ADCIRC, SMS) sonuçları düzensiz üçgen ve dörtgen ağlardadır
(mesh): düğümlerde ya da yüzlerde, zaman adımlarıyla derinlik, hız, su kotu. İklim ve okyanus verileri (ERA5, CMEMS) ve uydu ürünleri
NetCDF'te düzenli ızgaralı çok boyutlu dizilerdir (zaman, yükseklik, enlem, boylam). QGIS mesh katmanını (MDAL: 2DM, UGRID, SELAFIN …),
mesh hesaplayıcıyı, kesit ve zaman serisi çıkarmayı; ArcGIS Multidimension araç kutusu NetCDF raster katmanını (Make NetCDF Raster Layer:
bir değişken, boyutların seçilen değerleri) ve çok boyutlu raster işlemlerini verir. KentOS'ta bugün raster katmanları (GeoTIFF, PNG, JPEG)
ve onların çözümleme işi vardır; mesh ve NetCDF yoktur.

Araştırmada bulunanlar (10 Ekim):
- **NetCDF klasik biçimi** (Unidata, “NetCDF Classic and 64-bit Offset Format”; CDF-5 “64-bit data”): `CDF` ve sürüm baytı (1, 2, 5),
  kayıt sayısı (`0xFFFFFFFF`: akış, dosyanın boyundan), boyutlar, genel öznitelikler, değişkenler (adı, boyutları, öznitelikleri, türü,
  boyu, başlangıcı); büyük uçlu; dört bayta doldurulmuş adlar ve değerler; kayıt boyutundaki değişkenler kayıt kayıt iç içe, tek kayıt
  değişkeni varsa dolgusuz. Türler byte, char, short, int, float, double; CDF-5'te ubyte, ushort, uint, int64, uint64. NetCDF-4
  HDF5'tir (`\x89HDF`): kendi okuyucusu ayrı ve büyük bir iştir.
- **CF düzenli ızgara:** bir boyutlu koordinat değişkenleri (adı boyutunun adı; `axis`, `standard_name`, `units` ile X/Y ya da
  boylam/enlem), `_FillValue`, `missing_value`, `valid_min`/`valid_max`/`valid_range`, `scale_factor` ve `add_offset` (açılmış değerin
  türü bu özniteliklerin türü), `grid_mapping` (koordinat sistemi: `crs_wkt`, `spatial_ref`, `epsg_code`), zaman `units` dizisiyle
  (“hours since 1900-01-01 00:00:00”) ve `calendar`'la. GDAL artan enlemi kuzey yukarı çevirir, hücre merkezinden köşeye yarım hücre
  kayar, `_FillValue`'su olmayan float değişkende varsayılan dolguyu (9.96921e36) değersiz sayar, paketli değişkeni ham türüyle ve
  ölçeğiyle verir; uzay dışındaki boyutlar bantlardır (KentOS kodu olmadan denendi, 10 Ekim).
- **UGRID 1.0:** `cf_role = "mesh_topology"`, `topology_dimension = 2`, `node_coordinates`, `face_node_connectivity` (`start_index` 0 ya
  da 1, karışık ağlarda `_FillValue`); veri değişkenlerinde `mesh` ve `location` (`node`, `face`, `edge`).
- **2DM** (Aquaveo SMS): `MESH2D`; `ND id x y z`; `E3T id n1 n2 n3 m`, `E4Q id n1 n2 n3 n4 m`, ikinci dereceden `E6T`, `E8Q`, `E9Q`;
  düğüm numaraları boşluklu olabilir. **ASCII DAT:** `DATASET`, `BEGSCL`/`BEGVEC`, `ND`, `NC`, `NAME`, `TIMEUNITS`, `RT_JULIAN`,
  `TS istat t` (istat 1 ise önce eleman başına etkinlik), `ENDDS`.
- **QGIS 4.2.3 (MDAL) ile denenenler:** dörtgen yüz ilk düğümden yelpazeyle üçgenlenir (köşegen 1–3), içbükey yüz de (yüzün dışını da
  kaplar); düğüm verisi üçgende doğrusal (barisentrik) enterpole edilir; vektörde bileşenler enterpole edilip büyüklük alınır; DAT'ın
  etkinlik bayrağı sıfır olan yüz değersizdir. MDAL'dan ayrıldığımız yerler (10 Ekim'de yeniden denendi): MDAL düğümleri numara sırasında
  olmayan 2DM'i açmaz (“nodes are not ordered by index”), KentOS sırasız okur; MDAL `E6T`'yi altı düğümüyle çokgen okur, KentOS ikinci
  dereceden elemanları köşe düğümleriyle (§4); MDAL KentOS'un yazdığı `kentos_mask`'ı bilmez (etkin olmayan yüzü değerli gösterir); iki
  üçgenin ortak kenarında MDAL öbür üçgeni alabilir (KentOS sıradakini). MDAL ASCII DAT'ın sayılarını C kitaplığının yerel ayarıyla
  okur: Türkçe `LC_NUMERIC`'te “0.5” 0 okunur, kesirli saatler ve değerler kesilir; çapraz denetim `C.UTF-8` ile yapılır.

## Karar

### 1. Kapsam

- **NetCDF çok boyutlu raster:** NetCDF klasik biçimindeki (CDF-1, CDF-2, CDF-5) bir CF düzenli ızgara değişkeni; uzay dışındaki boyutların
  (zaman, yükseklik …) her birinden bir değer seçilir: raster bir **dilimi** gösterir.
- **Mesh katmanı:** NetCDF klasik biçimindeki UGRID 2B ağlar (üçgen, dörtgen ve 16 düğüme kadar çokgen yüzler; düğüm ve yüz veri setleri,
  zaman adımları; vektörler büyüklükleriyle) ve 2DM ile ASCII DAT dosyaları (içe aktarırken tek bir UGRID NetCDF'e yazılır). Mesh,
  okuyucunun her katta isteğe göre rasterleştirdiği **sanal raster**dir; ağın çizgileri görünüşün seçeneğidir.
- **Zaman:** CF zamanlı boyut zaman sürgüsünü (ADR 0210) izleyebilir.
- **İşlemler'in Çok boyutlu veri kategorisi:** Kesit (her rasterde), Zaman serisi (zaman boyutlu ya da çok bantlı rasterde) ve Mesh
  hesaplayıcı (veri setlerinden ifadeyle yeni veri seti, aynı ağda, yeni UGRID dosyasında).
- **Kapsam dışı:** NetCDF-4/HDF5, GRIB, HDF4, Zarr (anlaşılır retle; klasik biçime çevirme yolu söylenir), ikili DAT, XMDF, SELAFIN,
  kenar veri setleri, vektör okları, mesh düzenleme, bir boyutlu ağlar, yeniden izdüşüm, 360 günlük ve sıçramasız takvimler (boyut zaman
  sayılmaz, değerleriyle seçilir).

### 2. NetCDF klasik okuyucu ve yazıcı

- **Okuma** biçim çekirdeğinde (`kentos_formats::multidim::netcdf`), raster okuyucusunun düzeniyle: okuyucu hangi baytları istediğini
  söyler (`Need`), ev sahibi okur. Başlık en çok 64 MB; en çok 1 024 boyut, 8 192 değişken, nesne başına 4 096 öznitelik, değişken başına
  32 boyut; ad en çok 256 bayt UTF-8; öznitelik değeri en çok 16 MB. Taşan sayı, dosyanın dışına uzanan değişken, bilinmeyen tür, kayıt
  boyutu ilk boyut olmayan değişken nedeniyle reddedilir; hiçbir bozuk dosya paniğe yol açmaz.
- **Ret metinleri:** NetCDF-4/HDF5 için “Bu NetCDF-4 (HDF5) dosyası; KentOS klasik NetCDF okur. `nccopy -k cdf5 girdi.nc çıktı.nc`
  ya da `cdo -f nc5 copy girdi.nc çıktı.nc` ile çevirin.”; GRIB ve başka biçimler adıyla.
- **Yazma** (UGRID için, §4 ve §10): CDF-2 (64 bit ofset); tek kayıt değişkeni zamandır, kayıtlar art arda yazılır (bellek bir dilimlik).
- **`vsize`:** değişkenin bayt sayısının 32 bitlik alanı işaretsizdir; 2–4 GiB'lık değişken üst bitini doldurur, 4 GiB'ı aşanda 2³² − 1
  yazılır (Unidata'nın “Note on vsize”'ı; netCDF-C böyle yazar). Okuyucu alanı okumadan geçer, boyu boyutlardan hesaplar; yazıcı 2³² − 4'ü
  aşan değişkende CDF-5'e geçer. (ERA5'in 0,25°'lik bir aylık saatlik değişkeni 2,9 GiB'tır: alanı işaretli okumak onu reddederdi.)

### 3. CF düzenli ızgara

- **Değişken:** en az iki boyutlu, sayısal; son iki boyutu Y ve X. Boyutun koordinat değişkeni varsa ve `axis` (`X`/`Y`), `standard_name`
  (`projection_x_coordinate`, `longitude` …) ya da birimi (`degrees_east`, `degrees_north`) onu söylüyorsa onu, yoksa adı (`x`, `lon`,
  `longitude`, `y`, `lat`, `latitude`) karar verir; son iki boyut sırayla Y ve X değilse ret (“son iki boyut Y ve X olmalı”).
- **Yerleşim:** koordinatlar düzenli aralıklıysa (her değer ilk değer artı aralığın katından en çok aralığın binde biri sapar) dönüşüm
  hücre merkezinden köşeye yarım aralık kayar; artan Y satırları ters çevrilir (kuzey yukarı). Koordinat değişkeni yoksa raster yerleşimsiz
  (ADR 0204'ün kuralı); düzensiz aralık reddedilir.
- **Değerler:** paketli değişken (`scale_factor`, `add_offset`) açılır: değer = ham × ölçek + öteleme, türü özniteliklerin türü (float:
  32 bit, double: 64 bit kayan nokta). `_FillValue`, `missing_value`, geçerli aralığın dışı ve (dolgu niteliği olmayan float ve double
  değişkende) varsayılan dolgu değersizdir: kayan noktada NaN. Paketsiz tam sayı değişken türünü korur, dolgusu nodata'dır; 64 bit tam
  sayı 64 bit kayan noktaya çevrilir.
- **Koordinat sistemi:** `grid_mapping`'in `crs_wkt` ya da `spatial_ref`'i (WKT, ADR 0168'in okuyucusu), yoksa `epsg_code`
  (`EPSG:5254`); enlem ve boylamlı ızgara WGS 84 (EPSG:4326). Bulunmazsa sistemsizdir (ADR 0204'ün sorusu).
- **Boyutlar:** son ikisi dışındakiler dilim boyutlarıdır (en çok 8); her birinin değerleri koordinat değişkeninden (yoksa 0, 1, …),
  birimi `units`'ten.

### 4. UGRID, 2DM ve ASCII DAT

- **Ağ:** düğümler (x, y; en çok 20 milyon), yüzler (en çok 20 milyon; yüz başına 3–16 düğüm, `_FillValue` ile kısa yüzler). Dışbükey
  yüz ilk düğümden yelpazeyle üçgenlenir (QGIS'in kuralı); içbükey yüz kulak kırpmayla (geometri çekirdeğinin üçgenlemesi; QGIS yelpazeyle
  dışını da kaplar). Saat yönündeki yüz çevrilir; yinelenen düğüm, sıfır alanlı yüz ve dizinin dışındaki düğüm reddedilir (yüzün sırasıyla).
- **Veri seti:** `mesh`'i ağı, `location`'ı `node` ya da `face` olan sayısal değişken; ağın konum boyutu son boyutudur, öbürleri dilim
  boyutlarıdır (zaman, katman). Değerler §3'ün kuralıyla açılır. **Vektör:** aynı ağda, aynı konumda ve boyutlarda iki değişken ki adları
  yalnız son `x`↔`y` harfinde ayrılır (`ucx`/`ucy`, `hiz_x`/`hiz_y`, `x_velocity`/`y_velocity`) ya da adları `u` ve `v`'dir: tek veri
  seti olarak sunulur, gösterilen büyüklüktür (bileşenler enterpole edilir, sonra büyüklük alınır).
- **2DM ve DAT içe aktarma:** `ND`, `E3T`, `E4Q`; `E6T`, `E8Q`, `E9Q` köşe düğümleriyle; düğüm numaraları sıralanır. 2DM'in düğüm
  kotu “Taban kotu” veri setidir. ASCII DAT'ın skaler ve vektör veri setleri (`ND` ağın düğüm sayısına eşit olmalı; `NC` yüzlerinkine),
  zamanlar `TIMEUNITS`'le (Hours varsayılan; Seconds, Minutes, Days), etkinlik bayrağı sıfır olan yüz o adımda değersiz (yüzün üstündeki
  düğüm değerleri o yüzde gösterilmez; dosyaya o adımın yüz maskesi olarak yazılır). `RT_JULIAN` ya da pencerede yazılan başlangıç zamanı
  zamanları ana zaman yapar; ikisi de yoksa zamanlar başlangıçtan saattir ve sürgüyü izleyemez. Sonuç tek bir UGRID NetCDF'tir (CDF-2):
  ağ, her veri seti kendi değişkeni (vektörün iki bileşeni), maske ve zaman.

### 5. Raster olarak gösterim

- **Dilim:** raster bir değişkenin bir dilimini gösterir; tek bant (vektörde büyüklük). Dilim değişince yalnız o dilimin blokları
  okunur; ızgara satırları blok, ağın veri seti tek bloktur (aynı ev sahibi yolu: `needs`, okuma, `keep`).
- **Izgara:** düzey 0 dosyanın kendisidir (sıkıştırmasız büyük uçlu satır blokları, ters satırlar okuyucuda çevrilir); kaba düzeyler ADR
  0204'ün kuralıyla (2 × 2 ortalama, 4096'dan büyük kenarda bir kez hazırlanan önizleme piramidi; piramidin adı dilimi de taşır).
- **Mesh:** ağın kutusuna oturan eksenlere paralel sanal ızgara: hücre boyu pencerede yazılır (varsayılan: ağın ortalama yüz boyunun
  sekizde biri, 1-2-5 dizisinde aşağı yuvarlanmış: yüzler görünümü doldurduğunda dolgu yumuşak, ağın çizgileri ince kalır; kenar en çok 65 536 hücre). Her düzeyin karosu doğrudan ağdan rasterleştirilir: piksel
  merkezi bir üçgenin içindeyse (kenarı dahil; yüzlerin sırasıyla ilk bulunan) düğüm verisi barisentrik, yüz verisi yüzün değeri; hiçbir
  üçgende değilse ya da değerlerden biri değersizse NaN. Kaba düzeyde de ağdan örneklenir (ortalama değil). Örnek türü veri setinin
  açılmış türü (32 ya da 64 bit kayan nokta).
- **Ağ çizgileri:** görünüşün `edges` rengi (`#RRGGBB`) verilince yüzlerin kenarları karoya bir piksel kalınlığında çizilir; ortalama
  kenar o düzeyde dört pikselden kısaysa çizilmez (çizgiler birbirine karışır, karo pahalanır).
- **Koordinat oku** ve rasterin değerleri pikselin değeridir (ADR 0204). Raster çözümleme araçları gösterilen dilimde çalışır.

### 6. Sözleşme ve `.kcad` şema 37

`RasterFields`'e `dataset` (yoksa düz raster):

| Alan | Anlamı |
|---|---|
| `variable` | Değişkenin dosyadaki adı (vektörde x bileşeni). |
| `vector` | Vektörün y bileşeninin adı; varsa büyüklük gösterilir. |
| `mesh` | Ağın topoloji değişkeninin adı (UGRID); yoksa CF ızgara. |
| `dims` | Dilim boyutları dosyadaki sırasıyla: `name`, gösterilen `index`, `values` (koordinatlar; zamanda ana zamanlar, ms), `time` (CF zamanı), `units`. |
| `followTime` | Gösterilen zaman adımı zaman sürgüsünü izler. |

`RasterStyle`'a `edges` (ağ çizgilerinin rengi). Kurallar: değişken adı 1–256 harf, denetim karakteri yok; en çok 8 boyut, her birinde
1–100 000 sonlu değer, `index` değerlerin içinde; en çok bir zaman boyutu, değerleri azalmayan; `followTime` zaman boyutu ister; dataset'li
rasterin bandı 1'dir; `edges` mesh ister ve `#RRGGBB` olur. `FORMATS_VERSION` 47; bağımsız Python okuyucusu ve yazıcısı, örnek dosya.

### 7. Zaman

- **CF zamanı:** `units` “<birim> since <tarih>” (seconds, minutes, hours, days ve kısaltmaları; tarih `YYYY-M-D`, isteğe bağlı saat,
  kesirli saniye ve UTC farkı), `calendar` `standard`, `gregorian`, `proleptic_gregorian` ya da yok; değerler milisaniyeye yuvarlanır
  (ADR 0210'un ana zamanı). `standard` takvimde 1582-10-15'ten önceki başlangıç ve öbür takvimler zaman sayılmaz.
- **Zaman sürgüsü:** `followTime`'lı raster sürgünün aralığına girer (ilk ve son adımı); gösterilen adım, sürgünün anına (Anlık: pencerenin
  başı; Aralık: sonu, hariç) kadarki son adımdır; ilk adımdan önce raster çizilmez. Sürgü kapalıyken belgenin `index`'i gösterilir.
  Seçim stil çekirdeğinde, deponun penceresinden: aynı belge, aynı pencere, aynı dilim; iki platform aynı.

### 8. Kesit

- **Girdi:** raster; çizgiler (çizgi ve çoklu çizgi, yaylar dahil); Adım (m; varsayılan rasterin hücre boyu); Kotlu çizgi (isteğe bağlı).
- **Hesap:** her çizginin başından Adım aralıklarla ve sonunda noktalar; değer: mesh'te ağdan enterpolasyon (§5'in kuralı, piksel değil),
  öbür rasterlerde noktanın düştüğü hücrenin değeri. Gösterilen dilimde.
- **Sonuç:** tablo (Çizgi, Uzaklık (m), Y, X, Değer); isteğe bağlı yeni katmanda değerleri köşe kotu olan çoklu çizgiler (değersiz noktalar
  çizgiyi böler). Özet “k çizgi, n nokta (m değersiz).”.

### 9. Zaman serisi

- **Girdi:** raster (zaman boyutlu dataset'li raster; yoksa çok bantlı rasterin bantları); noktalar (çok noktalının her noktası).
- **Hesap:** her zaman adımı (öbür boyutlar rasterin dilimindeki değerlerinde) ya da bant için her noktanın değeri, §8'in kuralıyla.
  Okuma yalnız noktaların bloklarınadır.
- **Sonuç:** tablo: satırlar adımlar (Adım, Zaman ya da Bant), sütunlar noktalar (noktanın `ad` özniteliği, yoksa “Nokta k”). Özet
  “k nokta, n adım.”.

### 10. Mesh hesaplayıcı

- **Girdi:** mesh raster; İfade (ADR 0100'ün dili ve ADR 0233'ün matematik işlevleri; değişkenler veri setlerinin adlarıyla, boşluklu ad
  tırnakla); Zaman özeti (Yok, En büyük, En küçük, Ortalama, Toplam); Veri setinin adı; Çıktı dosyası.
- **Kurallar:** ifadenin andığı veri setleri aynı konumda (düğüm ya da yüz) olmalı; zamanlılar aynı zaman adımlarında, zamansızlar her adıma
  yayılır; katmanlı veri seti reddedilir. Değersiz girdi değersiz sonuç verir; Zaman özeti değersizleri atlar (hepsi değersizse değersiz).
- **Sonuç:** kaynağın ağı, zamanı (özetle yok olur) ve yeni veri setiyle yeni UGRID dosyası (CDF-2, 32 bit kayan nokta, değersiz NC_FILL_FLOAT)
  ve kaynağın katmanının hemen üstündeki yeni katmanda onu gösteren mesh raster.

### 11. Arayüz

- **Raster ekle** NetCDF'i de alır: değişken listesi (ızgaralar), her dilim boyutu için değer seçimi, Zaman sürgüsünü izle.
- **Mesh ekle** (`mesh.add`): 2DM (ve DAT'ları) ya da UGRID NetCDF; veri seti, dilim, hücre boyu, ağ çizgileri; 2DM'de başlangıç zamanı
  ve yazılacak NetCDF (masaüstünde 2DM'in yanında, web'de gömülür ya da indirilir).
- **Raster stili:** dataset'li rasterde Veri seti bölümü (boyutların değerleri, Zaman sürgüsünü izle); mesh'te Ağ çizgileri ve rengi.
- **Öznitelikler:** Veri seti, boyutların gösterilen değerleri, mesh'te düğüm ve yüz sayısı.
- **Şerit:** CBS'nin Raster sekmesinin Raster panelinde Raster ekle, Mesh ekle ve Raster stili; Raster oturt, Kesit, Zaman serisi ve
  Mesh hesaplayıcı aynı panelin ▾'inde (sekme on paneliyle 1100 px'e sığsın diye; ayrı Çok boyutlu veri paneli sığmadı); Veri › Raster
  panelinde Raster ekle'nin yanında Mesh ekle. İşlemler'de Çok boyutlu veri kategorisi.

### 12. Performans

Bütçeler (release, geliştirme makinesi; web tek iş parçacıklı işçide):

| İş | Masaüstü | Web |
|---|---|---|
| NetCDF başlığı ve değişken listesi (1 000 değişken) | ≤ 50 ms | ≤ 200 ms |
| Mesh'in açılışı (1 milyon yüz: ağ ve kova dizini) | ≤ 1 s | ≤ 3 s |
| Mesh karosu (1 milyon yüz; ince ve en kaba düzey) | ≤ 30 ms | ≤ 100 ms |
| Ağ çizgili karo | ≤ 40 ms | ≤ 150 ms |
| Zaman adımı değişince ilk görünüm (1 milyon düğüm) | ≤ 0,5 s | ≤ 1,5 s |
| 2DM ve DAT içe aktarma (1 milyon yüz, 24 adım) | ≤ 5 s | ≤ 15 s |
| Zaman serisi (100 nokta, 744 adım, 1440 × 721 ızgara) | ≤ 2 s | ≤ 10 s |
| Kesit (10 çizgi, 10 000 nokta, mesh) | ≤ 0,5 s | ≤ 2 s |
| Mesh hesaplayıcı (1 milyon düğüm, 24 adım, En büyük) | ≤ 3 s | ≤ 10 s |

### 13. Doğrulama

- **Bağımsız NetCDF:** `scripts/fixtures/netcdf_classic.py` (KentOS kodu ve libnetcdf olmadan, Unidata'nın belgesinden okuyucu ve yazıcı);
  GDAL'ın çok boyutlu API'si (libnetcdf) onun dosyalarını okur, onun yazdıklarını o okur.
- **Bağımsız başvuru** `scripts/fixtures/multidim_cases.py`: ızgaraların okunuşu (dilimler, paketleme, dolgu, ters satırlar, zaman),
  ağların okunuşu ve rasterleştirme (kesirlerle barisentrik), kesit, zaman serisi, hesaplayıcı; GDAL'la ızgara değerleri, QGIS'in MDAL'ıyla
  2DM, DAT ve UGRID'in nokta değerleri (dışbükey yüzlerde) çapraz denetlenir.
- **İşlemler'in ortak durumları** iki platformda, resimlerin sahnesi, süreler, resimler iki temada iki boyda.

## Uygulama

Tek parçada, iki platformda (10 Ekim).

- **Sözleşme ve şema 37:** rasterin `dataset`'i (`RasterDataset`: `variable`, `vector`, `mesh`, `dims` (`DatasetDim`: ad, gösterilen
  `index`, değerler, `time`, `units`), `followTime`) ve görünüşün `edges`'i; kurallar `RasterDataset::problem` ve rasterin `problem`'inde
  (adlar 1–256 harf, en çok 8 boyut, boyut başına 1–100 000 sonlu değer, `index` değerlerin içinde, zaman değerleri azalmaz, en çok bir zaman
  boyutu, `followTime` zaman boyutu ister, `edges` `#RRGGBB` ve yalnız mesh'te); komutların reddi `invalid_raster`. `.kcad` şema 37,
  `FORMATS_VERSION` 47; kodek, sütunlar, bağımsız Python okuyucu ve yazıcısı, `fixtures/kcad/v2/multidim.kcad` ve on üç bozuk dosya.
  Kitaplığın gömülü NetCDF'i `format: "netcdf"`, `data:application/x-netcdf` (iki platformda kabul, `.kstil` denetimi de).
- **Biçim çekirdeği** `kentos_formats::multidim`: `netcdf` (CDF-1, 2, 5 okuma, baytları isteyerek; §2'nin sınırları; `vsize` işaretsiz),
  `cf` (eksenler, düzenli adım, paketleme, dolgu, CF zamanı), `ugrid` (topolojiler, veri setleri, maske), `sms` (2DM, ASCII DAT), `mesh`
  (üçgenler, kova dizini, değer, kenarlar, varsayılan hücre), `cube` (dosyanın listesi `info`, dilimin okuyucusu `open`: ızgara satır
  blokları ya da sanal ızgaralı mesh; `Part`, anahtarın parçası), `series` (zaman serisinin okunacak parçaları, 4 KB'tan yakınları birleşik),
  `write` (UGRID CDF-2 ve 2DM ile DAT'tan UGRID). Raster okuyucusunun karosu mesh'te ağdan rasterleştirilir, `edges`'le kenarlar çizilir
  (`render_tile`, `tile_parts_with`).
- **Stil çekirdeği:** rasterin sahne anahtarı `taban#{json}` (değişken, vektör, mesh, dilim; mesh'te ızgara), zaman sürgüsünü izleyen
  rasterin dilimi deponun penceresinden (Anlık a, Aralık b'den bir önceki an); iki platformda aynı anahtar (`raster_key_tests`,
  `rasterKey.test.ts`).
- **Raster çekirdeği** `kentos_raster::multidim`: `points` (blokları okunan noktaların değerleri), `profile` (Kesit: `Walk` ile istasyonlar,
  çok parçalı çizgi parça parça; mesh'te `mesh_value`, ızgarada hücre), `series` (Zaman serisi: zaman boyutu ya da bantlar, adlı noktalar),
  `calc` (Mesh hesaplayıcı: ifade dili veri setlerinin adlarıyla ve uzun adlarıyla, ortak zaman adımları, özet, UGRID çıktısı); iş, raster
  işleri gibi ihtiyaç → oku → `put` → `step` → `finish`.
- **WASM:** formats-wasm `NetcdfFile` (listeleme, dilim, `needs(part)`), `smsToUgrid`, `meshGrid`, `datasetStyle`, `meshCounts`;
  raster-wasm `CubeOpening`, `OpsOpening.addCube` (çözümleme araçları NetCDF'in gösterilen diliminde), `MultidimAnalysis`.
- **İşlemler:** `Files::open_cube` (`CubeOpen`) iki ev sahibinde; Çok boyutlu veri kategorisi (`builtin/multidim/`, web
  `processing/builtin/multidim/tools.ts`): Kesit (`multidim.profile`; KESIT, PROFIL, BOYKESIT), Zaman serisi (`multidim.series`;
  ZAMANSERISI, ZSERI), Mesh hesaplayıcı (`multidim.meshCalculator`); bütün raster çözümleme araçları NetCDF rasterin gösterilen diliminde.
- **Masaüstü:** `rasters/multidim.rs` (Raster ekle'nin NetCDF'i ve Mesh ekle penceresi; hücre boyu projenin uzunluk biçimiyle),
  `rasters/tiles.rs` (dosya başına küp, dilim başına okuyucu), `rasters/look.rs` (Veri seti bölümü), `properties/rows/raster.rs` (Veri seti,
  boyutlar, Ağ), `mesh.add` ve üç işlem komutu; resimler `multidim_scenes.rs`.
- **Web:** raster işçisinin `cube`, `cubePlace`, `sms`, `meshCounts` istekleri (`io/rasterWorker.ts`, `render/rasterService.ts`; bir
  dosyanın dilimleri aynı işçide, küp paylaşılır); çözümleme işçisinin `multidim` işi (`io/rasterAnalysisWorker.ts`): parçalar `readRuns`'la
  okunur: tarayıcıda her dosya okumasının ~0,1 ms'lik sabit bedeli 128 KB okumanınkine yakın olduğundan 128 KB'tan yakın parçalar tek
  okumada (en çok 16 MB) alınıp bölünür, 256 okuma ya da 32 MB birlikte (Zaman serisi 74 400 parçada 11,1 s'den 1,1 s'ye); pencere
  `ui/raster/MultidimDialog.ts`, Raster stili'nin ve Öznitelikler'in bölümleri, `model/datasetLabels.ts` (boyut değerlerinin gösterimi
  çekirdekten), `model/rasterTimes.ts` (sürgünün aralığına izleyen rasterler).
- **Zaman sürgüsü** (ADR 0210'un eki, 10 Ekim): sürgünün yazıları çekirdekten (`time::show_window`, `show_ends`; web'e `timeShowWindow`,
  `timeShowEnds`): bir gün içindeki aralık tarihi bir kez yazar (“01.05.2024 14:00 – 15:00”), uçlar gün altı adımda bir gün içindeyse yalnız
  saat, değilse yalnız tarih; çubuk kendi genişliğine göre sığar: uçlar, sözcükler, Hız ve Döngü bu sırayla çekilir, iz en az 80 px kalır
  (masaüstünde yazılar ölçülür ve çubuğa çizim alanının genişliği verilir, web'de taşma ölçülür; `TimeBar.fit`, `temporal/bar.rs`).
  Eskiden masaüstü çubuğa pencerenin genişliğini veriyordu ve 1100 px'te iz görünmüyordu; web'de tarihli-saatli uçlar “Adım”ın üstüne
  taşıyordu.
- **Şerit** (§11): CBS'nin Raster sekmesinde Raster paneli (Raster ekle, Mesh ekle, Raster stili; ▾'de Raster oturt, Kesit, Zaman serisi,
  Mesh hesaplayıcı), Veri › Raster'de Mesh ekle; ikonlar `meshAdd`, `multidimProfile`, `timeSeries`, `meshCalculator`, `multidimData`.

## Doğrulama

- **Bağımsız başvuru** `scripts/fixtures/multidim_cases.py --check`: 78 durum (`fixtures/multidim/v1`): ızgaralar, CF zamanı, ağlar,
  rasterleştirme kesirlerle, 2DM ve DAT, UGRID yazımı bayt bayt, kesit, zaman serisi, hesaplayıcı (çıktısı bayt bayt), retler; dosyalar
  başvurunun kendi NetCDF yazıcısından (`netcdf_classic.py`, Unidata'nın belgesinden; `vsize`'ı da işaretsiz). GDAL'ın netCDF sürücüsüyle
  (libnetcdf) 18 bant aynı; 2,9 ve 4,8 GiB'lık iki ızgara (yalnız başlık ve koordinatlar depoda, seyrek kopyaları GDAL'a) aynı ızgarayı
  verir; QGIS 4.2.3'ün MDAL'ıyla 78 nokta değeri aynı. GDAL'ın buradaki libnetcdf'i CDF-5 okumaz: CDF-5 dosyası yalnız başvurunun
  okuyucusuyla denetlenir.
- **Biçim ve raster çekirdeği:** `cargo test -p kentos-formats --test all multidim` (durumların hepsi, büyük dosyalar `large`),
  `cargo test -p kentos-raster --test all multidim` (Kesit, Zaman serisi, Mesh hesaplayıcı; hesaplayıcının dosyası başvurununkiyle bayt
  bayt).
- **İşlemler:** `scripts/fixtures/multidim_processing_cases.py --check`, 9 durum (`fixtures/processing/v1/multidim.json`, `multidim.kcad`,
  `multidim/`) iki platformda (`cargo test -p kentos-processing --test cases`, web `processing/cases.test.ts`).
- **Komutlar ve şema:** `create_command_cases.py` ve `edit_command_cases.py --check` (veri setinin kuralları), `kcad_v2_reference.py
  --check` (529 dosya). **Zaman:** `temporal_cases.py --check`'e sürgünün yazılarının 17 durumu (`showWindows`, `showEnds`), iki platformda.
- **Süreler** (release; 13th Gen Intel Core i5-13500, 20 çekirdek; web Chrome'un işçisinde tek iş parçacığı, dosyalar diskten kullanıcının
  verdiği gibi; `multidim_timing.rs`, `scripts/perf/raster.mjs --only multidim`, aynı dosyalarla):

  | İş | Masaüstü | Bütçe | Web (p50) | Bütçe |
  |---|---|---|---|---|
  | NetCDF başlığı ve listesi (1 000 değişken) | 3,4 ms | 50 ms | 9 ms | 200 ms |
  | Mesh'in açılışı (999 698 yüz) | 264 ms | 1 s | 273 ms | 3 s |
  | Mesh karosu, en ince ve en kaba düzey | 1,1 ve 1,1 ms | 30 ms | 2 ve 2 ms | 100 ms |
  | Ağ çizgili karo (2. düzey, 8 321 kenar) | 1,7 ms | 40 ms | 2 ms | 150 ms |
  | Zaman adımı değişince ilk görünüm (1 000 000 düğüm, 25 karo) | 115 ms | 0,5 s | 150 ms | 1,5 s |
  | 2DM ve DAT içe aktarma (999 698 yüz, 24 adım) | 445 ms | 5 s | 614 ms | 15 s |
  | Zaman serisi (100 nokta, 744 adım, 1440 × 721; 74 400 parça) | 25 ms | 2 s | 1,09 s | 10 s |
  | Kesit (10 çizgi, 10 020 nokta, mesh) | 8 ms | 0,5 s | 474 ms | 2 s |
  | Mesh hesaplayıcı (1 000 000 düğüm, 24 adım, En büyük) | 285 ms | 3 s | 1,24 s | 10 s |

  Masaüstünün süreleri dosya bellekteyken (okumalar dahil değil); web'inkiler işçinin bütün işi (başlangıcı, modül, dosyanın okunması,
  Kesit'te ve hesaplayıcıda NetCDF'in açılışı da). Zaman serisinin dosyası 3 GB'lık seyrek dosyadır (okunan yerler sıfır).
- **Resimler:** sekiz sahne (`md-serit`, `md-ekle`, `md-mesh-ekle`, `md-zaman`, `md-stil`, `md-kesit`, `md-seri`, `md-hesap-cizim`) iki
  temada iki boyda, masaüstünde `tools_screens` (32) ve web'de `shots.mjs multidim` (32); sahnenin çizimi `multidim_scene.py --check`.
  Zaman sürgüsünün ADR 0210 sahneleri (`zaman-surgu`, `zaman-surgu-2015`) de yeniden çekildi.
