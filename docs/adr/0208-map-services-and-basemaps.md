# ADR 0208: Harita servisleri, altlıklar ve servisten veri

- **Durum:** kabul edildi (2026-10-08). Kapsam sahibin sözleridir (8 Ekim akşamı): “sıradan devam et”; altlıklar eklendiğinde
  yüksek performansla çalışsın; Google haritaları hazır altlık olarak eklenebilsin; sayılanların tamamı (WMS/WMTS, WFS ve OGC API,
  XYZ/TMS altlıklar, OpenStreetMap ve HGM ATLAS, kimlik doğrulama, vekil sunucu, eksen sırası) eklenebilsin; vektör karolar; kullanıcı
  doğrulaması adreste parametre, başlıkta değer, kullanıcı adı ve parola ya da belirteçle; GeoJSON da eklenebilsin; OSM Standart, OSM
  Topo ve öbür karolar eklenebilsin; “ben aksini belirtene kadar hem masaüstü hem web tarafını yapacaksın ve gerektiğinde gerekli
  aşamalarda bulut tarafını da yapacaksın”; ikonlar sorulmadan seçilir. İlke “Performance First”. Madde tek parçada biter.
- **Bağlam belgesi:** TODOS.md `GIS-10` (ve araştırma notu), ADR 0204 (raster: atlas, karolar, piramit, adresten okuma), ADR 0207 §1
  (HTTP aralıkları), ADR 0157 (uzak karolar), ADR 0167 ve 0168 (koordinat dönüşümleri), ADR 0199 (katman alanları), ADR 0090
  (stil motorunun partileri), ADR 0055 (çizimin yazıları), ADR 0046 (GeoJSON), ADR 0015 ve 0024 (proje yetkisi).

## Bağlam

KentOS çizimin altına harita koyamıyor, kamu kurumlarının ve belediyelerin yayımladığı servisleri (WMS, WMTS, WFS, ArcGIS REST) okuyamıyor.
Ölçmeci ve şehir plancısı bir parselin ya da yolun yerini görmek, kurumun verisini çizime almak için bunlara her gün başvurur. QGIS'in
OGC istemcisi ve XYZ karoları, ArcGIS Pro'nun altlıkları ve Netcad'in Online Haritaları (HGM ATLAS dahil) aynı işi görür. Altlıklar çizimin
en çok gezilen yüzüdür: karolar beklenmeden gelmeli, kaydırma ve yakınlaştırma takılmamalı. Proje TM dilimindedir (TUREF, ED50, UTM),
servislerin çoğu Web Mercator'dadır; karolar projenin sistemine yerinde çevrilmelidir. TUREF ve ED50'nin TM dilimleri EPSG'de kuzey-doğu
eksen sırasındadır (EPSG:5254'ün ilk ekseni X, kuzey); WMS 1.3.0 ve WFS 2.0 sistemin eksen sırasını izler, 1.1.1 izlemez: yanlış sıra
haritayı ülkenin öbür ucuna götürür.

## Karar

### 1. Kapsam

- **Çizilen servisler (servis katmanı):** XYZ ve TMS karoları, WMTS 1.0 (KVP ve REST), WMS 1.1.1 ve 1.3.0, OGC API Tiles (harita ve
  vektör), ArcGIS REST (MapServer ve ImageServer'ın karoları ve `export`'u), Google Map Tiles API'nin 2B karoları, vektör karolar (MVT
  2.1; TileJSON, MapLibre stili ya da adres şablonuyla).
- **Çizime alınan veri (servisten veri):** WFS 1.0, 1.1 ve 2.0, OGC API Features, ArcGIS REST'in katman sorgusu (FeatureServer ve
  MapServer katmanı), adresteki GeoJSON. Nesneler bir katmana tek geri alma adımında yazılır, katman kaynağını hatırlar ve Yenile ile
  yeniden alınır.
- **Servis bilgisi:** WMS GetFeatureInfo ve ArcGIS `identify`: tıklanan yerin servisteki kaydı.
- **Kimlik doğrulama:** adreste parametre, başlıkta değer, kullanıcı adı ve parola (HTTP Basic), belirteç (Bearer), ArcGIS belirteci
  (`generateToken`), OAuth 2 istemci kimliği (client credentials); Google'ın API anahtarı ve oturumu.
- **Hazır altlıklar:** OpenStreetMap Standart, OSM Topo (OpenTopoMap), OSM İnsani (HOT), CyclOSM, OpenFreeMap (vektör: Liberty, Bright,
  Positron), Esri Uydu, HGM ATLAS (Harita ve Ortofoto; anahtar HGM'den), Google (Yol, Uydu, Arazi, Karma; anahtar kullanıcının), MapTiler
  (vektör ve uydu; anahtar kullanıcının).
- **Vekil sunucu:** web, CORS'a izin vermeyen servise ve kimlik bilgisini sayfada tutmak istemeyen kullanıcıya `kentosd`'nin vekili
  üzerinden gider (§13).

### 2. Veri modeli

- **Servis katmanı:** katman ağacının düğümüne `service` (`kentos_contracts::service::ServiceLayer`): `kind` (`xyz`, `wms`, `wmts`,
  `ogcTiles`, `arcgis`, `google`, `vector`), `url` (XYZ'de şablon; WMS, WMTS ve ArcGIS'te servisin adresi; OGC'de karo kümesinin adresi;
  vektörde stilin, TileJSON'un ya da şablonun adresi), `layers` (WMS'in katman adları, WMTS'in katmanı, ArcGIS'in gösterilen katmanları),
  `style` (WMS ve WMTS'in stili; Google'da harita türü), `format`, `srid` (istenen sistem), `grid` (WMTS'in, ArcGIS'in ve OGC'nin karo
  matris kümesi, okunduğu gibi: sistem; her matrisin adı, çözünürlüğü, sol üst köşesi doğu ve kuzey sırasıyla, karo ve matris boyu;
  çevrimdışı çizim yetenekleri çağırmadan çizilir), `template` (WMTS REST'in ve OGC'nin karo şablonu), `tileSize`, `minZoom`, `maxZoom`,
  `subdomains`, `yFlip` (TMS), `transparent`, `version`, `params` (gizli olmayan ek parametreler: `TIME`, `CQL_FILTER` …), `dynamic`
  (WMS ve `export`'ta görünüm başına tek resim), `attribution`, `opacity`, `connection` (bağlantının kimliği), `preset` (hazır altlığın
  kimliği). Servis katmanı nesne tutmaz: komutlar ona nesne yazmayı `service_layer` ile reddeder; grup servis taşımaz.
- **Servisten veri:** katmanın `feed`'i (`FeatureFeed`): `kind` (`wfs`, `ogcFeatures`, `arcgis`, `geojson`), `url`, `name` (tür,
  koleksiyon ya da katman), `srid`, `filter` (CQL ya da ArcGIS `where`), `bbox` (projenin sisteminde istenen alan; yoksa hepsi), `limit`,
  `version`, `key` (Yenile'de nesneleri eşleyen öznitelik), `connection`, `fetched` (son alınış, RFC 3339).
- **Bağlantılar:** proje ayarlarının `connections`'ı (`ServiceConnection`): kimlik, ad, köken (şema, makine ve kapı), doğrulama türü ve
  gizli olmayan adları (parametrenin ya da başlığın adı, belirteç adresi). **Gizli değerler çizime yazılmaz:** cihazdadır (masaüstünde
  `$XDG_CONFIG_HOME/kentos-cad/baglantilar.json`, 0600; web'de `kentos.connections.v1`). Çizim başka cihazda açılınca eksik değer
  sorulur (Bağlantıyı tamamla). Kimlik bilgisi yalnız bağlantının kökenine gider; başka kökene yönlendiren yanıt bilgisiz izlenir.
- **`.kcad` şema 32** (`FORMATS_VERSION` 42): düğümün `service`'i ve `feed`'i, proje ayarlarının `connections`'ı; kurallar
  (`ServiceLayer::problem`, `FeatureFeed::problem`, `ServiceConnection::problem`) okuyucularda, komutlarda ve sunucuda aynı. Bağımsız
  Python okuyucusu ve yazıcısı, örnek `services.kcad` ve bozuk dosyalar.

### 3. Karo ızgaraları ve çizim

- **Izgara** (çekirdek `geom::tiles`, iki platformda aynı): Web Mercator karesi (`WebMercatorQuad`: köken ±20 037 508,342789 m, 0. katın
  pikseli 156 543,033928 m), WMTS'in ve OGC'nin matris kümeleri (ölçek paydası × 0,28 mm / birimin metresi; derecede 2πR/360), ArcGIS'in
  `tileInfo`'su (köken, katların çözünürlükleri), WMS'in ve `export`'un sanal ızgarası (sistemin kökeninden, Web Mercator'un çözünürlük
  merdiveniyle; derecede 360/256'dan; 512 piksellik karolar, işaretli sıra ve sütun).
- **Kat:** görünümün ortasında projenin bir ekran pikselinin karonun sistemindeki boyu (bir adım doğuya ve bir adım kuzeye giden iki
  noktanın ortalaması) ile matrislerin pikseli karşılaştırılır; logaritmik ölçekte en yakın kat (eşitlikte ince olan), servisin katları
  içinde. Resim karolarında cihaz pikseli (yüksek DPI'da keskin), vektör karolarda mantıksal piksel (stilin katı MapLibre'deki gibi CSS
  pikseline göre). Servisin en büyük katından ötesi büyütülerek çizilir.
- **Görünen karolar:** görünüm kutusunun kenarlarında 5'er nokta karonun sistemine çevrilir, kutuları karo aralığını verir; karolar
  görünümün ortasından dışarı doğru sıralanır (en çok 400). Dönüşüm yoksa (karonun sistemi projeninki) hesap doğrudandır.
- **Ağ:** her karo projenin sistemine ağ olarak çizilir: köşelerinin arası n × n'ye bölünür (aynı sistemde 1; karonun kenarı 100 km'den
  uzunsa 16, 10 km'den uzunsa 8, değilse 4; atlas yuvalarının katına yuvarlanır), her düğüm çekirdeğin dönüşümüyle (`crs::transform_in`,
  projenin datum seçimleriyle) taşınır; ağ karo başına bir kez hesaplanır ve saklanır. Karonun dokusu raster atlasının 258'lik yuvasındadır (ADR 0204 §5; kenar pikselleri tekrarlanır); ağın üçgenleri
  raster boru hattıyla, servis katmanı tek çizim çağrısıyla çizilir.
- **Yedek:** gelmemiş karonun yerine atlasın tuttuğu en yakın üst katın parçası çizilir (dörtlü ağaç ızgaralarında).
- **Sıra:** servis katmanı katman ağacındaki yerinde çizilir (altta, çizimin altında; üstte, saydam WMS gibi); saydamlığı `opacity`'dir.
- **Yükleme:** istekler bir kuyrukta, son karenin istediği karolar önde; kareden düşen karonun isteği bırakılır, yolda olan iptal edilir.
  Makine başına en çok 6 istek (OpenStreetMap'in sunucularında 2); 429 ve 503'te `Retry-After`'a uyulur, ağ hatasında üç deneme. Karolar
  ev sahibinin iş parçacıklarında çözülür (masaüstünde `png` ve `zune-jpeg`, web'de tarayıcının `createImageBitmap`'i), atlasa karede
  en çok 32 karo yüklenir. Karede bellek ayrılmaz.
- **Önbellek:** masaüstünde cihazın diskinde (`$XDG_CACHE_HOME/kentos-cad/servis/`, en çok 2 GB, en uzun süredir kullanılmayan önce
  silinir), HTTP'nin kurallarıyla: `Cache-Control` (`max-age`, `no-store`, `no-cache`, `must-revalidate`), `Expires`, `ETag` ve
  `Last-Modified` ile yeniden doğrulama (çekirdek `services::cache`). Çevrimdışıyken eskimiş karo gösterilir (Google'ınki hariç: koşulları
  çevrimdışı kullanımı yasaklar). Web'de tarayıcının önbelleği. Çözülmüş resim karoları bellekte, en uzun süredir çizilmeyen önce bırakılarak:
  masaüstünde 288 MB, web'de 64 MB.
- **Atıf:** çizim alanının sağ alt köşesinde görünen servis katmanlarının atıfları (tekrarsız); tıklanınca bağlantılarıyla liste.

### 4. XYZ ve TMS

- Şablonun yer tutucuları: `{z}`, `{x}`, `{y}`, `{-y}` (TMS), `{s}` (alt alanlar sırayla, karonun sırasından), `{r}` (yüksek DPI'da
  `@2x`), `{quadkey}`. Karo boyu 256 ya da 512, katlar `minZoom`–`maxZoom`. Sistem Web Mercator'dur.
- OpenStreetMap'in karo kullanım koşulları: uygulamayı tanıtan `User-Agent` (`KentOS-CAD/<sürüm> (+adres)`), önbellek başlıklarına uyma,
  toplu indirme ve önden çekme yok, makine başına 2 istek, atıf.

### 5. WMS

- **Yetenekler:** 1.3.0 ve 1.1.1 (`GetCapabilities`); katman ağacı (ad, başlık, özet, sorgulanabilir, sistemler mirasla, kapsam, stiller
  ve lejant adresleri, ölçek aralığı), biçimler, `GetMap` ve `GetFeatureInfo`'nun adresleri, `GetFeatureInfo`'nun biçimleri.
- **Sistem:** projenin sistemi sunuluyorsa o (dönüşümsüz), yoksa 3857, yoksa 4326, yoksa kayıttaki başka bir sistem.
- **Eksen sırası:** kayıttaki her sistemin `axisOrder`'ı (`en`, `ne`, `latlon`; PROJ'dan denetlenir): 1.3.0'da `BBOX` sistemin sırasıyla
  (TUREF ve ED50 TM dilimleri kuzey önce, 4326 enlem önce), 1.1.1'de her zaman doğu önce; `CRS`/`SRS`.
- **İstek:** sanal ızgaranın 512 piksellik karoları (`TILED=true`) ya da `dynamic` ise görünüm başına tek resim (300 ms bekleyerek);
  `TRANSPARENT`, `FORMAT`, `STYLES`, ek parametreler.

### 6. WMTS ve OGC API Tiles

- **WMTS:** yetenekler (katmanlar, stiller, biçimler, `TileMatrixSetLink` ve sınırları, `ResourceURL` şablonları, boyutların varsayılanı),
  `TileMatrixSet`'ler (sistemin eksen sırasıyla verilen sol üst köşe doğu ve kuzeye çevrilir; `urn:ogc:def:crs:EPSG::5254`,
  `EPSG:5254`, `http://www.opengis.net/def/crs/EPSG/0/5254`, `CRS84`). REST şablonu varsa o, yoksa KVP `GetTile`.
- **OGC API Tiles:** açılış sayfası, `/collections` ve `/map/tiles`, karo kümelerinin JSON'u (`tileMatrixSetURI`, şablon bağlantısı
  `{tileMatrix}/{tileRow}/{tileCol}`, veri türü harita ya da vektör), `WebMercatorQuad` içeride, öbür matris kümeleri adreslerinden.

### 7. ArcGIS REST

- Servisin JSON'u (`?f=json`): `tileInfo` (köken, katlar, karo boyu, sistemin `wkid`/`latestWkid`'i; 102100 = 3857), kapsam, katmanlar.
- Önbellekli servis `tile/{level}/{row}/{col}` ile karo karo; önbelleksiz MapServer `export` ile (`bbox`, `bboxSR`, `imageSR`, `size`,
  `format=png32`, `transparent`, `layers=show:`), WMS'in sanal ızgarasıyla ya da görünüm başına.
- Katman sorgusu (§10) `query` ile (`f=geojson`, sayfa sayfa `resultOffset`); `identify` (§11).

### 8. Google Map Tiles API

- **Oturum:** `POST https://tile.googleapis.com/v1/createSession?key=…` (`mapType`: `roadmap`, `satellite`, `terrain`; Arazi'ye
  `layerRoadmap`, Karma uydu ve `layerRoadmap`; `language` `tr-TR`, `region` `TR`; yüksek DPI'da `scaleFactor2x`); oturum süresinin son
  yarım saatine dek cihazda saklanır ve yeniden kullanılır.
- **Karolar:** `https://tile.googleapis.com/v1/2dtiles/{z}/{x}/{y}?session=…&key=…`. **Atıf:** “Google Maps” ve görünümün telif metni
  (`/tile/v1/viewport`, görünüm durunca); koşullar gereği karolar yalnız `Cache-Control`'ün izin verdiği kadar saklanır, önden çekilmez,
  çevrimdışı gösterilmez. Anahtar kullanıcınındır (Google Cloud hesabı, faturalandırma); KentOS anahtar vermez.

### 9. Vektör karolar

- **Kaynak:** MapLibre stili (sürüm 8: `sources`, `layers`, `glyphs` ve `sprite` adresleri), TileJSON 3, şablon ya da OGC API vektör karo
  kümesi. Karolar MVT 2.1'dir (çekirdeğin kendi protobuf okuyucusu; gzip'li gövde tanınır ve açılır).
- **Stilin alt kümesi** (çekirdek `services::style`): katman türleri `background`, `fill`, `line`, `circle`, `symbol` (yazı), `raster`
  (stilin raster kaynağı), `fill-extrusion` düz alan olarak; süzgeçler (eski biçim ve ifade); ifadeler `get`, `has`, `!has`, `literal`, `!`,
  `==`, `!=`, `<`, `<=`, `>`, `>=`, `all`, `any`, `none`, `in`, `!in`, `match`, `case`, `coalesce`, `step`, `interpolate` (`linear`,
  `exponential`, `cubic-bezier`), `zoom`, `geometry-type`, `id`, `concat`, `to-string`, `to-number`, `to-boolean`, `upcase`, `downcase`,
  `length`, `at`, `rgb`, `rgba`, `to-color`; eski işlevler (`stops`, `base`, `property`, `type`). Renkler CSS'in biçimleri. Desteklenmeyen
  özellik ve ifade katmanı düşürmez: o özellik varsayılanını alır, katmanın notunda söylenir.
- **Çizim:** karo bir kez çözülür ve projenin sistemine taşınır; stil karenin katında (çeyrek kat adımlarıyla) değerlendirilir; alanlar
  üçgenlenir, çizgiler ve daireler stil motorunun partileriyle (ADR 0090; `style-core`'un `BatchSink`'i) paketlenir; iki platform aynı
  partileri çizer. Yazılar: noktada (`text-anchor`, `text-offset`, `text-max-width` ile satırlar) ve çizgi boyunca (`symbol-placement:
  line`, `symbol-spacing`, eğri boyunca harfler; ADR 0196'nın kuralı), haleli; çakışan yazı düşer (ekranda, stilin sırasıyla, `text-padding`);
  yazı tipi çizimin Arimo'su (düz, eğik, kalın; yerleşim çekirdeğin Arimo genişlikleriyle iki platformda aynı). İkonlar bu maddede yoktur (§17).
- **Noktalar ağdan:** vektör karonun noktaları projenin sistemine resim karosunun çizildiği ağla taşınır (`geom::tiles::MeshMap`, hücrede
  çift doğrusal): nokta başına dönüşüm yerine düğüm başına. Ankara'nın z14 karosunda (21 914 nokta) en büyük sapma 4 × 4 ağda 3,6 mm, 8 × 8'de
  0,9 mm, 16 × 16'da 0,23 mm; karonun kendi adımı (4096'lık kapsam) aynı karoda 0,46 m'dir.

### 10. Servisten veri

- **WFS:** yetenekler (türler, başlık, varsayılan ve öbür sistemler, kapsam, çıktı biçimleri), `GetFeature` (2.0'da `TYPENAMES`, `COUNT`,
  `STARTINDEX` ile sayfa sayfa; 1.1'de `MAXFEATURES`); `BBOX` sistemin eksen sırasıyla ve sistemiyle; biçim sunuluyorsa GeoJSON, yoksa GML
  2, 3.1 ve 3.2 (çekirdeğin GML okuyucusu: nokta, çizgi, alan, çoklular, `Curve`'ün `LineStringSegment`'leri, `Surface`'ın `PolygonPatch`'leri,
  `posList` ve `srsDimension`, `coordinates`; öznitelikler düz alanlar).
- **OGC API Features:** koleksiyonlar, `items` (`bbox`, `limit`, `next` bağlantısıyla sayfa sayfa), CRS kısmı 2 sunuluyorsa `crs` ve
  `bbox-crs`, yoksa CRS84.
- **ArcGIS:** katmanın `query`'si (`where`, `geometry`, `inSR`, `outSR`, `outFields=*`, `f=geojson`, `resultOffset`/`resultRecordCount`).
- **GeoJSON adresi:** tek istek; RFC 7946 (CRS84) ya da eski `crs` üyesi.
- **Alma:** alan (Görünüm, Çizimin kapsamı, Seçimin kapsamı, Hepsi), süzgeç, en çok nesne (varsayılan 50 000, en çok 500 000), hedef
  katman (yeni, türün adıyla ve kendi rengiyle; ya da var olan), alanlar değerlerden (ADR 0199); koordinatlar servisin sisteminden projeninkine
  çekirdeğin dönüşümüyle (taşınamayan nesne sayılır ve söylenir). Tek geri alma adımı “Servisten veri al”, iş arka planda (masaüstünde
  kendi iş parçacığında, web'de kendi işçisinde), durdurulabilir.
- **İstenen sistem:** listede türün sunduğu sistemler; ArcGIS katmanında (sunucu çevirir) önce projenin sistemi, sonra katmanınki ve
  WGS 84 (`FeedConnecting::systems`); seçilmezse projeninki listedeyse o, değilse WGS 84 (`asked_srid`); liste istenecek sistemi seçili
  gösterir. Okuyucunun atladıkları (geometrisi boş, okunamayan) sayfa sayfa toplanır ve sonuçta söylenir; geometrisi boş gelen nesne
  servisin istenen sistemde gösteremediği nesne olabilir, WGS 84'te yeniden almak önerilir (`feed::taken_words`, iki platformun sözü).
  **Yenile:** aynı istek yeniden; `key` verilmişse aynı anahtarlı nesne kimliğini koruyarak güncellenir, yenisi eklenir, kalmayanı silinir;
  verilmemişse katmanın nesneleri yenileriyle değişir; tek adım “Servisi yenile”.

### 11. Servis bilgisi

- **Servis bilgisi** aracı: tıklanan yerde görünen WMS katmanlarına `GetFeatureInfo` (`INFO_FORMAT` sırasıyla `application/json`,
  `application/geo+json`, `text/plain`, `application/vnd.ogc.gml`, `text/html` metne çevrilerek), ArcGIS'e `identify`; sonuç alt panelin
  Servis bilgisi listesinde katman katman, alan ve değer.

### 12. Bağlantılar ve kimlik doğrulama

- **Türler:** Yok; Parametre (adreste bir ya da birkaç `ad=değer`); Başlık (bir ya da birkaç `Ad: değer`); Kullanıcı adı ve parola (HTTP
  Basic); Belirteç (`Authorization: Bearer`); ArcGIS belirteci (kullanıcı adı ve parolayla `generateToken`, süresi dolmadan yenilenir,
  `token` parametresi); OAuth 2 (istemci kimliği ve sırrıyla belirteç adresinden erişim belirteci, süresi dolmadan yenilenir); Google
  anahtarı (§8).
- **Bağlantılar penceresi:** liste, ekle, düzenle, sil, Dene (bağlantıyı kullanan servise sorar ve sonucu söyler). Servis ve veri
  pencereleri bağlantıyı seçer ya da yenisini açar. Değerleri bu cihazda olmayan bağlantı söylenir: hazır altlık eklenince Bağlantılar
  o bağlantıda açılır, Öznitelikler'in Bağlantı satırı “değerleri bu cihazda yok” der, katmanın rozeti nedenini gösterir.
- Gizli değer günlüğe, hata iletisine, önbelleğin anahtarına ya da çizime yazılmaz; önbelleğin anahtarı adresin gizli parametreler
  çıkarılmış hâlidir.

### 13. Vekil sunucu (kentosd)

- `GET /v1/proxy?url=…`: oturum açmış kullanıcı içindir. Hedefin adresi `http` ya da `https`; makinenin adı sunucuda çözülür ve bağlantı
  çözülen adrese kurulur (DNS yeniden bağlama yok); geri döngü, özel, bağlantı yerel (169.254/16, bulut üst verisi dahil), CGNAT,
  çok noktaya yayın ve belirsiz adresler reddedilir (`KENTOS_PROXY_ALLOW` ile kurumun iç ağı açılabilir); yönlendirme en çok 3, her biri
  yeniden denetlenir (303, ya da POST'tan sonra 301 ve 302, gövdesiz GET olarak sürer). Yanıt en çok 32 MB ve 30 saniye; çerez gitmez
  gelmez; ortamın vekili kullanılmaz (hedefi kendisi çözerdi); hedeften `Accept-Encoding: identity` istenir; yalnız güvenli başlıklar
  geçer (`Content-Type`, `Cache-Control`, `ETag`, `Last-Modified`, `Expires`, sıkıştırılmış gelirse `Content-Encoding`). Hedefe gidecek
  başlıklar `x-kentos-header-<ad>` olarak gelir (bağlantının, çerezin, vekilin ve tarayıcının `sec-` başlıkları hiç gitmez); `POST` de
  geçer (ArcGIS'in belirteci). Kullanıcı başına aynı anda en çok 16 istek. Günlüğe yalnız makine, durum ve bayt sayısı yazılır.
- Web önce doğrudan gider; tarayıcı reddederse (CORS ya da ağ) ve oturum açıksa vekilden; vekile giden köken oturum boyunca hatırlanır.
  Bağlantı başına seçilen bir **Erişim** ayarı (Doğrudan, Sunucu üzerinden) sonraki iştir.

### 14. Arayüz

- **Şerit:** CBS'de Harita › **Altlık** paneli: Altlık ▾ (hazır altlıklar ikonlarıyla ve Altlığı kaldır), Harita servisi, Servisten veri al,
  Servis bilgisi, Bağlantılar. CAD'de Ekle › aynı Altlık paneli.
- **Harita servisi** penceresi: solda tür (Hazır altlıklar, XYZ / TMS, WMS, WMTS, OGC API Tiles, Vektör karo, ArcGIS REST, Google); sağda
  adres, bağlantı, Bağlan (yetenekler) ve aranabilir katman listesi, stil, biçim, sistem (önerileni ile), saydamlık, tek resim, katlar;
  Ekle servis katmanını ağaca koyar (altlık en alta, saydam servis altlıkların üstüne).
- **Servisten veri al** penceresi: tür, adres, bağlantı, Bağlan, tür ve koleksiyon listesi, alan, süzgeç, en çok nesne, hedef katman, sayı
  tahmini, Al (ilerleme ve Durdur).
- **Katmanlar:** servis katmanının ikonu hazır altlığınki ya da türünden, hata rozeti ve iletisi; sağ tıkta Servis ayarları, Yeniden yükle,
  Önbelleği temizle, Servisin kapsamına yakınlaştır, Saydamlık ▸, Yeniden adlandır, Sil (nesne işlemleri yok; servis katmanı etkin katman
  yapılamaz, çift tık Servis ayarları'nı açar); veri katmanında Yenile. Web'de Önbelleği temizle servisin her yanıtını bir kez tarayıcının
  önbelleğini atlayarak yeniden ister (tarayıcının önbelleği sayfaya silinemez).
- **Öznitelikler:** seçim yokken ağaçta seçilen (yoksa etkin) katman servis katmanıysa en üstte Servis katmanı (tür, adres, katmanlar,
  stil, sistem, katlar, bağlantı, saydamlık, atıf, durum), veri katmanıysa Veri kaynağı (tür, adres, tür adı, istenen sistem, süzgeç, alan,
  anahtar alan, bağlantı, son alınış).

### 15. Komutlar ve otomasyon

- `cad.layers.service` v1 (iki platformda, ortak durumlarla): `add` (servis katmanı), `addFeed` (veri katmanı, alanlarıyla), `update`,
  `remove`; bağlantıların gizli olmayan tanımı da aynı adımda. Kodlar `invalid_service`, `invalid_feed`, `invalid_connection`,
  `service_layer`, `unknown_layer`.
- Python: `kentos.services`: `presets`, `add_basemap`, `read_service`, `add_service`, `read_feed`, `import_features` (çekirdeğin okuyucuları
  `kentos._native` üzerinden, istekler `urllib` ile, bağlantının gizli değeri yalnız istekte; `import_features` `cad.layers.service`'in
  `addFeed`'i ve `cad.entities.create` ile tek adımda yazar, `Taken` uygulamaların sözüyle döner). Python'da Yenile ve katman rengi yoktur.
  MCP'de `cad.layers.service` katalogdan gelir (`remove` katman sildiği için yıkıcı işaretli).

### 16. Performans

- Görünen karoların seçimi karede ≤ 0,2 ms; karonun ağı ≤ 0,5 ms (bir kez); 256'lık PNG'nin çözülmesi ≤ 3 ms; bir şehir karosunun (z14) MVT
  çözümü, stili ve partileri ≤ 10 ms (bir kez, katın çeyreğinde yeniden stil ≤ 3 ms); önbellekten tam görünüm ≤ 100 ms; iki altlıklı karede
  CPU p50 < 2 ms, bellek ayrımı yok. Release süreleri ADR'nin Doğrulama'sında (`perf::services`).

### 17. Kapsam dışı (sonraki işler)

WCS, WFS-T (servise yazma), STAC, CSW katalog istemcisi, SensorThings; vektör karoların ikonları (sprite) ve 3B binaları; WebP karolar
masaüstünde (`image-webp` kilitte var; onay ister); kurumun sunucuda saklanan ortak bağlantıları; paftada altlık (§16.4'le).

## Uygulama

9 Ekim 2026, tek parçada, iki platformda ve bulut tarafıyla.

- **Sözleşme ve dosya:** `kentos_contracts::service` (`ServiceLayer`, `TileGrid`, `TileMatrix`, `ServiceParam`, `FeatureFeed`,
  `ServiceConnection`, `ConnectionSecret` ve kuralları; `bbox` WGS 84'te batı, güney, doğu, kuzey), `cad_layers` (`cad.layers.service`);
  `.kcad` şema 32 (`FORMATS_VERSION` 42; kodek `kcad/src/{encode,decode}/services.rs`), bağımsız Python okuyucusu ve yazıcısı
  (`tools/kcad/kcad.py`, `kcad_v2_reference.py`), `services.kcad` ve 51 bozuk dosya; kuralların durumları `fixtures/services/v1/rules.json`
  (`service_rules_cases.py`), komutun durumları `fixtures/commands/v1/cad.layers.service.json` (`layer_service_command_cases.py`).
- **Çekirdek:** `crates/shared/services` (`kentos-services`): `request` (WMS, WMTS, WFS, OGC API, ArcGIS istekleri, eksen sırası), `caps`
  (WMS, WMTS, WFS, OGC API, ArcGIS, TileJSON), `connect` (Harita servisi'nin Bağlan'ı), `feed` (Servisten veri al'ın Bağlan'ı, sayfa sayfa
  alma, sistem listesi, `taken_words`), `auth`, `cache` (HTTP'nin kuralları), `mvt`, `style` (MapLibre stilinin alt kümesi ve partiler),
  `labels`, `gml`, `info`, `google`, `source`, `template`, `picture` (atlas yuvaları), `presets`, `attribution`, `crs`; geometri çekirdeğinin
  `geom::tiles`'ı (ızgaralar, kat, görünen karolar, ağ, `MeshMap`, quadkey).
- **Masaüstü:** `services/` (`hub`: servislerin durumu, karolar, bellek sınırı, atıflar; `resolve`, `net` (iş parçacıkları, makine başına
  sınır, `Retry-After`), `cache` (disk), `secrets`, `systems`, `decode` (resim ve vektör karolar), `overlay` (etiketler ve atıf şeridi),
  `app`, `basemaps`, `window`, `feed_window`, `connections`, `info`); çizim hattında `styled/service_tiles.rs`; katman ağacı, Öznitelikler,
  alt panelin Servis bilgisi sekmesi, şerit ve komutlar.
- **Web:** `io/services/` (WASM `crates/wasm/services-wasm`, karoların işçisi `worker.ts`, Servisten veri al'ın işçisi `feedWorker.ts`,
  istek ve vekile düşme `fetch.ts`), `render/serviceHub.ts`, `render/servicePass.ts` (iki çizim hattında), `viewport/serviceLayers.ts`
  (etiketler, atıf şeridi), `app/services.ts`, `app/serviceCommands.ts`, `app/connectionSecrets.ts`, `model/servicePresets.ts`,
  `serviceRules.ts`, `serviceRead.ts`, `serviceSystems.ts`, `ui/services/` (Bağlantılar, Harita servisi, Servisten veri al),
  `ui/bottom/ServiceInfoPanel.ts` ve `serviceInfoRun.ts`, `ui/layers/serviceMenu.ts`, `ui/properties/serviceRows.ts`, `styles/services.css`.
- **Sunucu:** `apps/api/src/http/proxy.rs` (`/v1/proxy`, kendi çözücüsüyle `reqwest`, `KENTOS_PROXY_ALLOW`); yeni bağımlılık yok.
- **Otomasyon:** `kentos._native`'in servis bağları (`crates/native/python/src/services.rs`), `python/kentos/services.py`; başsız
  komut sunucusunda ve MCP'de `cad.layers.service`.
- **Simgeler** (sorulmadan seçildi): `basemap`, `basemapRemove`, her hazır altlığın kendi simgesi, `serviceAdd`, `serviceFeed`, `serviceInfo`,
  `serviceConnections`, `serviceSettings`, `serviceReload`, `serviceCache`, `serviceExtent`, `feedRefresh` (`ui/serviceIcons.ts`, masaüstü
  aynılarını envanterden okur).
- **Ayrılan ya da sonraya kalanlar:** bağlantı başına Erişim ayarı (web her zaman önce doğrudan gider); web'de projenin NTv2 ızgaralı datum
  seçimi servislerin WASM'ında yoktur (ızgaralar geometri modülündedir; böyle bir projede web'de servisin karoları çizilmez, nedeni henüz
  söylenmez); Python'da
  Yenile ve katman rengi; vektör karoların ikonları (§17). Kaydırmada etiketlerin her karede yeniden yerleşmesi ve çeyrek katta stilin
  baştan kurulması bütçeyi aşar (Doğrulama): sonraki iş, kaydırma boyunca yerleşimi tutmak ve üçgenlemeyi katlar arasında saklamak.

## Doğrulama

**Bağımsız başvurular** (KentOS kodu olmadan): karolar `service_tiles_cases.py` (pyproj ve PROJ'un hatlarıyla, ADR'nin kurallarından:
TUREF TM33 ve TM30 ile Web Mercator projelerinde Web Mercator ve coğrafi ızgara, 512'lik karolar, katları sınırlı servis; 7 görünümün
kutusu, merkezi, pikseli, katı ve sırasıyla karoları, 3 karo ağı, 10 quadkey; çekirdeğin `tests/all/tiles.rs`'i kutuları, merkezleri ve
düğümleri 1e-6 m (coğrafide 1e-11°) içinde, katları ve karoları birebir verir); vektör karo `service_mvt_cases.py` (GDAL'ın MVT sürücüsünün
yazdığı z14 karosu: 3 katman, 6 nesne; nokta, çoklu nokta, çizgi, çoklu çizgi, delikli ve çoklu alan; metin, tam ve ondalık sayı,
evet/hayır, kimlik; GDAL'ın okuyucusuyla); yetenekler `service_caps_cases.py` (OWSLib'le WMS 1.3.0 ve 1.1.1, WMTS 1.0.0'ın üç matris kümesi,
TUREF TM'nin kuzey önce eksen sırası dahil, WFS 2.0.0; adlar, biçimler ve sistemler birebir, pikseller 1e-12, köşeler ve kutular 1e-9
içinde); kurallar `service_rules_cases.py` (servis, besleme, bağlantı ve kökenin 93 durumu) ve komut `layer_service_command_cases.py`
(36 durum), iki platformda ve Python'da. Hazır altlıkların tablosu (`fixtures/services/v1/presets.json`, 6 grupta 16 altlık) iki
platformun ve Python'un tek kaynağıdır.

**`.kcad` şema 32:** bağımsız yazıcı 427 dosya (`kcad_v2_reference.py --check`); 51 bozuk dosyanın her biri Python okuyucusunda ve Rust
kodeğinde nedeniyle reddedilir; `kcad/tests/all/services.rs` (şema 32 yalnız servis, besleme ya da bağlantı varken, bit bit gidiş dönüş,
yazıcının retleri, okuyucunun hata yerleri). Bulunan eski hata: web'in kaydı katmanın alanlarını (şema 26) düşürüyordu; servis alanlarıyla
birlikte düzeltildi ve bayt bayt teste eklendi.

**Uygulama ve ağ:** çekirdeğin `feed` testleri (ArcGIS katmanı projenin sisteminde istenir ve listede görünür, GeoJSON adresinde seçilecek
sistem yoktur, sayfaların atladıkları toplanır, sonuç cümlesi), `tests/all/caps.rs` ve `mvt.rs`; vekilin 7 testi (`proxy_tests.rs`: yalnız
genel İnternet geneldir, kurumun `KENTOS_PROXY_ALLOW` ağları, yalnız önekli başlıklar gider (çerez ve `Host` asla), adres istek gitmeden
reddedilir, yönlendirmeler izlenir ve yalnız güvenli başlıklar döner, fazla yönlendirme, büyük yanıt ve içeriye yönlenme reddedilir,
kullanıcı başına en çok 16 istek); Python'un 4 testi yerel bir sunucuyla (hazır altlık, projenin sisteminde WMS, geometrisiz nesnenin söylendiği GeoJSON adresi, anahtarın çizime girmediği sayfalı ArcGIS
katmanı); hazır altlıklar iki platformda aynı testle (`app/services.test.ts`, `services::basemaps::tests`: en alta konur, sonraki onun yerini
alır, kaldırılır, her biri tek geri alma adımı; anahtarı bu cihazda olmayan altlık Bağlantılar'ı kendi bağlantısında açar). Masaüstünün
testleri kullanıcının `baglantilar.json`'unu okumaz (bellekte). Gerçek servislerle resimlerde: OpenStreetMap, OSM Topo, Esri uydu,
OpenFreeMap Liberty (vektör), HGM ATLAS'ın anahtar istemesi, terrestris OSM-WMS, GeoSolutions GeoServer WMS (`topp:states`), ArcGIS
Online World_Topo_Map ve World Countries (Generalized) FeatureServer. Google Map Tiles API anahtarla sınanmadı (anahtar yok); oturumu
ve isteği çekirdeğin testleri sınar.

**Görsel inceleme:** web 15 sahne (`shots.mjs services`), masaüstü 14 sahne (`tools_screens`'in `servis-*`'ı), 1440×900 ve 1100×650'de
iki temada. İncelemede bulunanlar: Servisten veri al'da ArcGIS 251 ülkeden 112'sini TM33'te geometrisiz döndürüyordu, iki platform
bunları söylemeden atlıyordu: çekirdek atlananları nedeniyle sayar, sonuç cümlesi söyler, sistem listesi servisin verdikleridir ve istenen
sistem kurala bağlandı (§10); web'de Yeniden yükle karoları yeniden çözmüyordu (işçiye `forget`); yüksek DPI'da masaüstü katı mantıksal,
web cihaz pikseliyle seçiyordu: iki platformda resim karoları cihaz, vektör karolar mantıksal pikselle (§3); Harita servisi penceresinde
arama kutusu eziliyordu; Öznitelikler'de servis bölümü en üste alındı; servis katmanında Etkin katman yap kapalı, çift tık Servis
ayarları'nı açar.

**Süreler** (release; 11th Gen Intel i5-11300H, 4 çekirdek 8 iş parçacığı; Iris Xe ve RTX 3050 Mobile, wgpu varsayılan bağdaştırıcısıyla;
9 Ekim; `perf::services`, `perf::services_style_split`). Proje TUREF / TM33, servisler Web Mercator; kareler 1440×900.

| Ölçü | Sonuç | Bütçe |
|---|---|---|
| Görünen karolar, 1:1000 (48 karo) / 1:50 000 (24 karo); p50 (p99) | 21,2 µs (29,2) / 19,3 µs (21,9) | ≤ 0,2 ms |
| Karonun ağı, kat 18, 4 × 4 / kat 8, 16 × 16; p50 | 10,3 / 118,8 µs | ≤ 0,5 ms |
| 256'lık PNG karo (22 269 bayt) çözme ve kesme; p50 (p99) | 0,40 ms (0,45) | ≤ 3 ms |
| Şehir karosu (z14, 194 718 bayt): MVT okuma; p50 (p99) | 1,25 ms (1,38) | — |
| aynı karonun stili ve partileri, kat 14; p50 (p99) | 4,59 ms (5,32) | okumayla ≤ 10 ms |
| aynı, katın çeyreğinde (14,25) yeniden stil; p50 | 4,61 ms | ≤ 3 ms: aşılır |
| Tam görünüm cihazın önbelleğinden, iki altlık | 125 ms | ≤ 100 ms: aşılır |
| İki altlıklı boş kare, MİB p50 (p95) | 0,88 ms (1,00) | < 2 ms |
| İki altlıklı orta tuşla kaydırma, MİB p50 (p95); çizimin payı | 2,25 ms (2,78); 1,58 ms | < 2 ms: aşılır |
| aynı iki kare, GPU p50 (başsız pencerenin resmi geri okuması dahil) | 12,9 / 13,1 ms | — |

Vektör karonun stilinde zamanın %70'i nokta başına koordinat dönüşümüydü (21 914 nokta × 0,43 µs; 13,59 ms): noktalar karonun ağıyla
(`MeshMap`, resim karolarının çizildiği aynı ağ) taşınır, süre 4,59 ms'ye iner. Ağın PROJ'dan en büyük sapması 4 × 4'te 3,6 mm, 8 × 8'de
0,9 mm, 16 × 16'da 0,23 mm'dir; z14 karosunun kendi adımı (4096'lık ölçekte bir birim) Ankara'da ~0,46 m'dir. Bütçeyi aşan üç süre
`REN-17`'dedir: kaydırmada etiketler her karede yeniden yerleşir ve masaüstünde tutulan resim silinir; çeyrek katta stil baştan kurulur;
önbellekten iki altlıklı tam görünüm 125 ms sürer (payları ayrıca ölçülmedi). Karede bellek ayrımı sayılmadı. Ölçülmeyenler: web'in kare
süreleri (başsız SwiftShader gerçek GPU ölçüsü değildir), gerçek ağın gecikmesi, başka GPU'lar.

**Takımlar** (9 Ekim): `pnpm rust:test` (2 789 test, clippy, bağımlılık yönü 37 crate), `pnpm rust:test:desktop` (921 test, clippy;
yeni ikisiyle 923), `pnpm test` (4 042 test; yeni ikisiyle 4 044), `pnpm py:test` (45 test, 568 komut durumu), `pnpm e2e:interaction`
(133 iz × 3), `pnpm build`, `pnpm inventory` (126 pencerenin hepsi masaüstünde). İlk koşuda düşen üç test (web'de menünün kategorisi,
masaüstünde yerleşim planı ve `ported.json`) yeni sekme ve komutlarla güncellendi; biçimlemeden sonra iki clippy yeniden geçti.
