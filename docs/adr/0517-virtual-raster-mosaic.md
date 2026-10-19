# ADR 0517: Sanal raster mozaiği (VRT)

- **Durum:** kabul edildi (2026-10-19). `plan/gis.md` CBS-28'in “çok dosyalı sanal mozaik” dilimi. COG dışa aktarımı ayrı dilimdir
  (ADR 0515); bu ADR ona dokunmaz.
- **Bağlam belgesi:** ADR 0204 (raster katmanları, okuyucu, piramit), 0231 §2 (çözümlemenin ev sahibi dosyaları), 0233 (İşlemler'in
  Mozaik aracı: GeoTIFF'e kopyalayan), 0228 (web'e ara).
- **Web:** ertelendi (ADR 0228). Web'de gerekenler TODOS.md §0.2'de.
- **Sonraya kalan:**
  - mozaiğin dosyalarının önizlemesi yoksa uzak görünüm her dosyanın tam çözünürlüğünden hesaplanır; mozaik için önizleme
    piramidi (ya da üyelerin kendi piramitleri) yok;
  - VRT yalnız dosyadan açılır; adresten (HTTP) ve gömülü VRT, adresteki üyeler yok;
  - paletli, dönük ve kuzeyi yukarıda olmayan dosyalar, farklı sistemdeki dosyalar (yeniden izdüşüm) mozaiğe konmaz;
  - bant eşleme (dosyanın bandını seçmek), ölçek/öteleme, `-addalpha`, `-tap`, `-te` (kapsamı elle verme) yok;
  - tarih önceliği yalnız TIFF DateTime etiketinden; tarih alanı verme ve dosya zamanı yok;
  - ETag ile kaynağın değişmesi, disk ve bellek bütçesinin kaynak yaşam döngüsüne bağlanması (CBS-28'in sağlayıcı kısmı).

## Bağlam

Ortofoto ve yükseklik modelleri çoğunlukla paftalara bölünmüş gelir: yüzlerce GeoTIFF, örtüşen kenarlar, farklı yıllar ve
çözünürlükler. KentOS'ta her dosya ayrı bir raster katmanıydı. İşlemler'in Mozaik aracı (ADR 0233) bunları yeni bir GeoTIFF'e
kopyalayarak birleştiriyordu: çok gigabaytlı ortofotoda disk ve zaman ister, kaynak değişince kopya eskir.

GDAL'ın sanal raster dosyası (VRT) bu işin yaygın çözümüdür: dosyaların nerede durduğunu söyleyen küçük bir XML. Piksel kopyalanmaz;
GDAL, QGIS ve gdalbuildvrt aynı dosyayı okur ve yazar.

## Karar

### 1. Biçim: GDAL'ın VRT'si

Mozaik GDAL'ın `VRTDataset`'idir (`kentos_formats::raster::vrt`):

- **KentOS'un okudukları:**
  - boyut, `GeoTransform`;
  - `SRS`: `EPSG:n` ya da son yetkilisi EPSG olan WKT;
  - bandın `dataType`'ı ve `NoDataValue`'su;
  - `SimpleSource` ve `ComplexSource`'un dosyası (`relativeToVRT`), bandı, `SrcRect`, `DstRect` ve `NODATA`'sı.
- **Sıra:** sonraki kaynak öncekilerin üstündedir. Kaynağın nodata'sı alttakinin görünmesini sağlar.
- **Bantlar:** her bant aynı dosyaları aynı dikdörtgenlerle almalıdır (gdalbuildvrt böyle yazar).
- **Reddedilenler, nedeni söylenerek:**
  - `subClass`'lı (warp, türetilmiş, pansharpen) VRT;
  - `AveragedSource`, `KernelFilteredSource`;
  - ölçek, LUT;
  - mozaiğin içinde mozaik.
- **Sınırlar (ADR 0009):** 16 MB metin, 10 000 kaynak.
- **Yazma:** KentOS GDAL'ın öğeleriyle yazar. Nodata'lı dosya `ComplexSource` olur. Sistem `EPSG:n` olarak yazılır (GDAL kullanıcı
  girdisi olarak okur). Dosya VRT'nin klasörünün altındaysa ondan göreli yazılır.

### 2. Okuma: dosyaların okuyucuları ve katları

- **Okuyucu:** `Reader::mosaic` her dosyanın kendi okuyucusunu (TIFF'in dizinleri, blokları ve önbelleği) bir mozaik okuyucusunda
  toplar. Mozaiğin her katı (`LevelFrom::Mosaic`) dosyaların o kata en yakın kendi katlarından kurulur: pikseli mozaiğinkinden büyük
  olmayan en kaba kat. Böylece uzak görünüm dosyaların kendi önizlemelerini okur.
- **Piksel:**
  - her pikselin merkezi dosyanın katında en yakın piksele bakar (GDAL'ın VRT kaynağını okuduğu gibi);
  - kaynaklar alttan üste yazılır;
  - dosyanın nodata'sı ya da NaN'ı alttakini bırakır;
  - hiçbir dosyanın örtmediği yer mozaiğin nodata'sıdır (yoksa 0, GDAL gibi).
- **Ev sahibinin blokları:** blok isteği (`BlockNeed`) hangi dosyanın olduğunu `member` ile söyler (0: rasterin kendi dosyası,
  k: mozaiğin k'inci dosyası). Ev sahibi o dosyadan okur:
  - masaüstünün çizimi: `rasters::tiles::Opened::read`;
  - çözümlemenin dosyaları: `pointclouds::files`'ın `open_raster`'ı, `tiles::BlockSource`.
- **Dosya tutamağı:** üye dosya her okumada açılır ve kapanır. Binlerce dosyalı mozaik tutamak tutmaz.
- **Açma:** `vrt::open_mosaic` VRT'yi okur ve her dosyayı ev sahibinin açıcısıyla açar (masaüstünde `open_reader_part`; TIFF parça
  parça). Mozaiğe piramit dosyası yapılmaz (`needs_pyramid` yok).
- **Raster ekle:** `.vrt` seçilebilir ve hep bağlı kalır (gömülmez: dosyaları yanında olmalıdır).

### 3. Komut: `cad.rasters.mosaic`

Girdi:

- `sources`: GeoTIFF'ler; mutlak ya da çizimin klasöründen göreli.
- `path`: yazılacak `.vrt`.
- `order`:
  - `given`: listenin ilki üstte;
  - `finest`: pikselin alanı küçük olan üstte;
  - `newest`: TIFF DateTime'ı yeni olan üstte; tarihsizler en altta.
  
  Eşitlikte listenin sırası geçer.
- `resolution`: `highest`, `lowest` ya da `average`; her eksen kendi başına, gdalbuildvrt'nin `-resolution`'ı gibi. `pixelSize`
  verilirse kare piksel olur.
- `nodata`: verilmezse sırayla ilk nodata'lı dosyanınki.
- `layer`: verilirse mozaik o katmana `path`'e bağlı raster olarak eklenir.
- `overwrite`, `drawing`, `expectedRevision`.

Kural gdalbuildvrt'ninkidir:

- **Kapsam:** dosyaların birleşimi.
- **Boyut:** `(int)(0.5 + kapsam / piksel)`.
- **DstRect:** `((x₀ − minX)/we, (maxY − y₀)/ns, w·a/we, h·|d|/ns)`. Yuvarlanan boyutun bittiği yerde kesilir; o zaman SrcRect de
  kısalır (`GetSrcDstWin`). Mozaikte hiçbir şeyi kalmayan dosya dışarıda kalır.
- **Yazma sırası:** dosyalar alttan üste yazılır; böylece üstteki son kaynaktır.

Retler sırasıyla:

1. `mosaic_empty`;
2. `mosaic_too_large`: 10 000'den çok dosya;
3. `invalid_file_path`: yol, `.vrt` değil, klasörü yok, göreli ama çizimin dosyası yok;
4. `invalid_pixel_size`;
5. dosya dosya: `file_not_found`, `import_unreadable` (GeoTIFF değil), `mosaic_unplaced`, `mosaic_rotated`, `mosaic_mismatch`
   (paletli; sistem, bant sayısı ya da veri türü ilk dosyanınkinden başka);
6. `mosaic_too_large`: bir kenarı 2 milyar pikseli geçen mozaik;
7. `file_exists`;
8. `layer` verildiyse: katmanın retleri ve `crs_mismatch` (mozaiğin sistemi projeninkinden başka);
9. sürüm.

Uyarılar:

- `mosaic_resampled`: pikseli mozaiğinkinden başka dosyalar;
- `mosaic_undated`: tarih sırasında tarihsiz dosyalar;
- `layer_hidden`.

Dosya yarım kalmaz: geçici adla yazılır, geri okunur, okuyucunun kuralıyla ayrıştırılıp karşılaştırılır, sonra yerine konur.
`layer` ile raster tek geri alma adımıdır (“Sanal mozaik”). Geri alma dosyayı silmez (DXF dışa aktarmadaki gibi). Plan yazmadan
söyler. Çıktı: yol, boyut, bantlar, örnek türü, geotransform, sistem, nodata, dosyalar (sıra, okunduğu yer, VRT'deki adı, boyutu,
pikseli, tarihi, SrcRect ve DstRect'i), bayt ve eklenen rasterin kimliği.

### 4. Doğrulama

- **Ortak durumlar:** `fixtures/commands/v1/cad.rasters.mosaic.json`. 38 durumu `scripts/fixtures/raster_mosaic_cases.py`, KentOS kodu
  olmadan yazar: dosyaların başlıkları GDAL'la, kural bu ADR'den. Tamamlanan her adım gdalbuildvrt'nin kendi VRT'siyle 1e-9 içinde
  karşılaştırılır (boyut, geotransform, SrcRect, DstRect). Masaüstünün çalıştırıcısı ve Python SDK'sı aynı durumları geçer. Dosya
  yazan durumlar için iki çalıştırıcıya `$tmp:` yer tutucusu eklendi.
- **Pikseller:** `fixtures/raster/v1/mosaic`'te GDAL'ın kendi VRT'leri (`gdal-*.vrt`) ve pikselleri (`gdal-*.raw`) var. Üç mozaik:
  - verilen sıra;
  - 10, 7,5 ve 5 m'lik yükseklik modelleri en kaba pikselle;
  - üç yılın ortofotoları.

  KentOS'un okuyucusu bunları değer değer aynı okur (`kentos-formats`'ın `raster_mosaic` testi). Komutun yazdığı VRT'ler
  (`kentos-*.vrt`) GDAL'la açılınca GDAL'ınkiyle aynı pikselleri verir (`raster_mosaic_cases.py --check`). Delikli ve farklı
  çözünürlüklü dosyalar dahil.
- **Masaüstü:** pencerenin akışı `rasters::mosaic::tests`'te: plan, sıralama, yazma, tek adım, geri alma ve ret. Resimler
  `raster_scenes.rs`'te (`mozaik-pencere`, `mozaik-cizim`, `mozaik-dem`, `mozaik-dem-cizim`).

### 5. Arayüz

`raster.mosaic` (Sanal mozaik):

- **Yeri:** CBS'de Veri › Raster ve Raster › Raster, CAD'de Ekle › Raster.
- **İkon:** `rasterMosaic`; kesikli çerçevede dört pafta.
- **Pencere:**
  - Dosyalar listesi: Sıra, Dosya, Boyut, Piksel ve Tarih sütunları; Dosya ekle…, Kaldır, Yukarı, Aşağı;
  - Örtüşen yerde üstteki: Listenin sırası, İnce piksel üstte ya da Yeni tarih üstte;
  - Mozaiğin pikseli: En ince, En kaba ya da Ortalama; Piksel boyu, Nodata;
  - Mozaik dosyası: varsayılanı ilk dosyanın yanında `mozaik.vrt`; Kaydet…;
  - Katman olarak ekle ve katmanın adı.
- **Önizleme:** pencere her seçimde komutun planını gösterir: boyut, piksel, sistem, bantlar, nodata ve uyarılar. Her dosyanın sırası
  da plandan gelir.
- **Oluştur:** komutu çalıştırır. Katman olarak eklenirse yeni katman ve raster tek adımdır.

İşlemler'in Mozaik aracı (kopyalayan) yerinde kalır. Ad çakışmasın diye komutun takma adları `SANALMOZAIK`, `BUILDVRT`, `VRT`'dir.

## Sonuçlar

- Paftalı ortofoto ve yükseklik modelleri tek katmanda görünür ve tek raster olarak çözümlenir. İşlemler'in raster araçları mozaiği
  ev sahibinin dosyalarıyla okur. Kopya yoktur.
- Mozaik dosyası GDAL ve QGIS'te aynı açılır, onların VRT'si de KentOS'ta açılır.
- Öncelik kuralı yazma anında sıraya çevrilir. VRT'de tarih ya da çözünürlük yoktur; sırayı değiştirmek için komut yeniden
  çalıştırılır.
