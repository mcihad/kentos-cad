# ADR 0515: Bulut için düzenlenmiş GeoTIFF (COG) ve başsız raster dosyaları

- **Durum:** kabul edildi (2026-10-19). `plan/gis.md` CBS-28'in ilk dilimi (“doğrulanmış COG dışa aktarımı”); CBS-41'in
  kalanı “başsız dosya erişimi”nin raster yolu.
- **Bağlam belgesi:** ADR 0204 (raster katmanları, karolu GeoTIFF yazıcısı), 0207 §1 (adresten HTTP aralıklarıyla okuma),
  0231 §2 (çözümleme sonuçlarının yazıcısı), 0233 (Raster işlemleri), 0508 (İşlemler'in araçları komut olarak).
- **Web:** ertelendi (ADR 0228). Web'de gerekenler TODOS.md §0.2'de.
- **Sonraya kalan (CBS-28'in sonraki dilimleri):** çok dosyalı sanal mozaik (örtüşme, çözünürlük ve tarih önceliği),
  ortak raster sağlayıcısı (Range, ETag, disk ve RAM bütçesi tek yaşam döngüsünde), codec profili; adresteki rasterin,
  JPEG'in, NetCDF diliminin ve nokta bulutlarının başsız okunması.

## Bağlam

KentOS'un raster çözümlemeleri (yüzey, interpolasyon, raster işlemleri, raster ve vektör, hidroloji, uzaklık, uygunluk,
uzaktan algılama) sonuçlarını karolu, Deflate'li, önizleme katlı GeoTIFF olarak yazıyordu, ama yazıcı akış hâlindedir:
başlık, karolar, en sonda dizinler. Böyle bir dosya bir adresten okunurken önce sonundaki dizinler istenir, sonra
katların karoları dosyanın her yerinden. Bulut için düzenlenmiş GeoTIFF (COG) dizinleri başa, en küçük katın karolarını
öne koyar; GDAL, QGIS, tarayıcı kütüphaneleri ve KentOS'un kendi adres okuyucusu (ADR 0207 §1) onu az istekle okur.
Plan, COG uygunluğunun bağımsız bir denetleyiciyle doğrulanmasını ister.

İşlemler'in araçları ADR 0508'le komut oldu, ama başsız sunucunun dosyası yoktu: raster araçları Python ve MCP'den
“bu ortamda dosya erişimi yok” diye reddediliyordu.

## Karar

### 1. Düzen

`kentos_formats::raster::cog`:

- **Okuma:** küçük sonlu (II), klasik ya da BigTIFF, karolu bir TIFF'in dizinlerini okur. Şeritli, büyük sonlu ve
  karosuz dosya reddedilir.
- **Yeniden düzen:** dizinlerin bütün girdileri bayt bayt korunur, yalnız karoların yerleri (324) yeni yerlerini
  söyler. Dosya şu sırayla yazılır:
  1. başlık;
  2. GDAL'ın yapı bilgisi (`GDAL_STRUCTURAL_METADATA_SIZE`, `LAYOUT=IFDS_BEFORE_DATA`, `BLOCK_ORDER=ROW_MAJOR`);
  3. bütün dizinler, değerleriyle (tam görüntü, sonra küçülen katlar);
  4. karolar: en küçük katınkiler önce, tam görüntününkiler en son, her katta satır satır.
- **Bayt kopyası:** karoların baytlarına dokunulmaz. `plan` önce yazılacakları, sonra kopyalanacak aralıkları verir.
  Bellek yalnız dizinlerin değerleridir; ev sahibi karoları akış dosyasından okuyup kopyalar.
- **Kararlılık:** aynı dosya aynı baytları verir, COG'un COG'u kendisidir.
- **Okunuş:** GDAL düzenin adını `LAYOUT=COG` okur. Blok öncü ve artçısı (leader, trailer) bildirilmez, yazılmaz.

### 2. Bütün raster sonuçları COG

İşlemler'in `Sink`'ine `finish_cog` eklendi. Raster sonucu yazan altı yol (Yüzey analizi, İnterpolasyon ve yoğunluk,
Raster işlemleri ve onların üstündeki hidroloji, uygunluk, uzaktan algılama araçları, Raster ve vektör, Uzaklık ve
maliyet, Rasterleştir) sonucu COG olarak bitirir:

- akış dosyası `.yaziliyor` adıyla yazılır, sonra yanında COG'a çevrilir (`.cog-yaziliyor`), diske indirilir ve tek
  adımda yerine konur;
- akış dosyası silinir; yarıda kalan iş geride dosya bırakmaz;
- ek maliyet bir dosya kopyasıdır. Sıkıştırılmış karolar yeniden kodlanmaz.

NetCDF yazan Mesh hesaplayıcı değişmedi.

### 3. COG olarak kaydet

Raster işlemleri'ne **COG olarak kaydet** (`raster.cog`) eklendi. Raster çekirdeğinin `copy` işi rasteri kendi
ızgarasında, en yakın hücreyle okur; her hücrenin merkezi tam kendi hücresine düşer. Böylece:

- bantlar, örnek türü, değerler, değeri olmayan hücre ve alfa olduğu gibi kalır;
- önizleme katları ADR 0231 §2'nin ortalamasıdır;
- PNG, şeritli ya da önizlemesiz TIFF, JPEG sıkıştırmalı TIFF (masaüstünde) ve NetCDF dilimi de COG olur.

Masaüstünde CBS › Raster › Raster işlemleri'ndedir (ikon: raster ızgarası ve bulut; üç seçenekten A). Arayüz kaydı
`catalog/v1/additions.json`'da, işlem manifestinde `semantic`, `via` `cad.processing.run`.

### 4. Başsız sunucunun dosyaları

`kentos_processing::local_files::LocalFiles` İşlemler'in `Files`'ının başsız karşılığıdır; `cad.processing.run` her
çalıştırmaya onu verir (`Runner::with_files`). Python'un `cad.processing.run`'ı ve MCP raster araçlarını böylece
çalıştırır.

- **Okuduğu:** rasterin bağlı dosyası, yerinde aralık aralık; projenin kitaplığındaki gömülü baytlar. Biçimler GeoTIFF
  (karolu ya da şeritli, ADR 0204'ün okuyucusuyla) ve PNG.
- **Yazdığı:** sonucu istenen yere ya da kaynağın yanına, önce `.yaziliyor` adıyla, sonra yeniden adlandırarak;
  raster sonucu COG olarak.
- **Okumadığı, nedeniyle reddedilir:** adresteki raster, JPEG, NetCDF dilimi, nokta bulutları ve COPC. Bunlar
  masaüstünde kalır.
- **Göreli yol:** çalışma klasöründen okunur. Çizimin klasörü komuta henüz gelmiyor (sonraki iş).

## Doğrulama

- **Biçim çekirdeği** (`cog` modülünün testleri): üç katlı bir dosyada bütün dizinler bütün karolardan önce, en küçük
  katın karoları önce; her karonun baytları kaynağınkiyle aynı; öbür girdiler bayt bayt aynı; aynı dosya aynı baytları
  verir, COG'un COG'u kendisidir; şeritli ve büyük sonlu dosya reddedilir.
- **İşlemler'in ortak durumları:** yüzey, interpolasyon, raster işlemleri, raster ve vektör, hidroloji, uzaklık,
  uygunluk ve uzaktan algılama durumlarının yazdığı her raster COG'dur. Yeniden düzenlenince kendisidir, değerleri eski
  başvurularına uyar.
- **Başsız uçtan uca** (`kentos-native-application`'ın `cog` testi). Dört raster `cad.processing.run`'la COG olur:
  - şeritli float32, delikli;
  - GDAL'ın önizlemeleriyle 16 bit, dönük ızgara;
  - PNG;
  - RGBA TIFF.

  Her biri yeniden düzenlenince kendisidir, çizime tek adımda eklenir, geride `.yaziliyor` kalmaz; aynı raster aynı
  baytları verir. Adresteki raster ve olmayan dosya nedeniyle reddedilir. `KENTOS_WRITE_COG=1` çıktıyı
  `fixtures/cog/v1/out`'a koyar.
- **Bağımsız denetim** (`scripts/fixtures/cog_cases.py`). Kaynakları GDAL yazar (`--files`). Betik KentOS kodu
  olmadan:
  - KentOS'un COG'larını GDAL'ın `validate_cloud_optimized_geotiff.py`'siyle tam denetler: hata ve uyarı olmamalı;
  - `LAYOUT=COG`, Deflate, 256'lık karolar, konum (çizimin affine'i, dönük olan dahil) ve EPSG'yi denetler;
  - 0. katın değerlerini kaynağınkilerle bit bit karşılaştırır;
  - önizleme katlarının boylarını ve değerlerini ADR 0231 §2'nin kuralıyla numpy'da üst katın yazılmış değerlerinden
    yeniden hesaplar.
- **Resimler:** `raster-cog-*` (şeritte COG olarak kaydet, penceresi, çizime eklenen COG).

## Sonuçlar

- KentOS'un yazdığı her raster bulut için düzenlidir: bir adrese konduğunda KentOS, QGIS ve GDAL onu bütünüyle
  indirmeden açar.
- Raster çözümlemeleri Python'dan ve MCP'den de çalışır (bağlı ve gömülü GeoTIFF ve PNG).
- Sonuç başına bir dosya kopyası daha yazılır; büyük sonuçlarda geçici disk iki katına çıkar.
