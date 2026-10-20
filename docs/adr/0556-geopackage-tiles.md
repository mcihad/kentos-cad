# ADR 0556: GeoPackage karo (raster) profili — oku ve yaz

- **Durum:** kabul edildi (2026-10-20). `plan/gis.md` CBS-03'ün raster dilimi (“sonra raster tile”); ADR 0502'nin “sonraya
  kalan”ının birincisi.
- **Bağlam belgesi:** ADR 0502 (GeoPackage crate'i), 0204 (raster katmanları, karolu GeoTIFF yazıcısı), 0515 (COG), 0517
  (sanal mozaik), 0518 (raster oturt: kaynağın okunması), 0228 (web'e ara).
- **Kapsam (ajanın kararı, 20 Ekim):** GeoPackage'ın karo piramidi (`gpkg_tile_matrix_set`, `gpkg_tile_matrix`, PNG
  karolar) bir raster olarak okunur ve çizimin bir rasteri karo piramidi olarak yazılır. Raster katmanı karoları
  dosyadan canlı okumaz: seçilen düzeyin karoları yan yana konup bir GeoTIFF'e yazılır ve o GeoTIFF çizimin rasteri olur.
  Canlı kaynak, `.kcad`'de yeni bir raster kaynağı türü ve okuyucuda yeni bir kat kaynağı ister; sonraya kaldı.
- **Web:** ertelendi (ADR 0228). Web'de gerekenler TODOS.md §0.2'de.
- **Okunmayanlar, nedeniyle söylenir:**
  - JPEG karolar (`tile_format_unsupported`): uygulama katmanında JPEG çözücüsü yok; masaüstü kabuğunun zune-jpeg'i başsız
    komutlarda yoktur ve yeni bağımlılık eklenmedi;
  - WebP karolar (`gpkg_webp` uzantısı): çözücü yok; uzantı `extensions`'ta listelenir;
  - 8 bitten fazla PNG ve `gpkg_2d_gridded_coverage` (yükseklik karoları): ayrı bir profil;
  - tabloda hiç karosu olmayan düzey (`tile_table_empty`).
- **Sonraya kalan:**
  - raster katmanının karoları GeoPackage'tan canlı okuması (görünümün karoları, önbellek, adreste dosya);
  - JPEG ve WebP karolar, 16 bitlik ve yükseklik karoları;
  - 25 milyon pikseli aşan raster: karolar şerit şerit okunup yazılmalı (şimdi tuval bellekte, 4 bant × 25 milyon = 100 MB);
  - karo yazarken JPEG ve renk kuantalama; `gpkg_zoom_other` uzantısı;
  - süre ölçümü (bütçe belirlenmedi; sınır belleğe göre konuldu).

## Bağlam

GeoPackage yalnız vektörün değil, haritacıların ortofoto altlığının da en yaygın alışveriş dosyası: QGIS ve GDAL bir raster'ı
karo piramidi olarak yazar, kurumlar altlığı tek `.gpkg` olarak verir. ADR 0502 vektör ve öznitelik tablolarını aldı; karo
tabloları `Layer`'da `DataType::Tiles` olarak listeleniyor ama okunmuyordu. Plan “önce vektör ve tablo, sonra raster
tile”, QGIS 4.2'nin dosyasının açılmasını ve KentOS'un yazdığının QGIS'te açılmasını ister.

## Karar

### 1. `kentos-geopackage`'ın karo modülü (`tiles.rs`)

Crate karo baytlarını bilmez; satır satır tutar. Yeni:

- `GeoPackage::tile_sets()`: `gpkg_contents`'in `tiles` tabloları, karo matrisi kümeleriyle ve matrisleriyle (`TileSet`,
  `TileMatrix`: zoom, matris boyu, karo boyu, piksel). Karo matrisi kümesi satırı olmayan tablo atlanır (okunacak ızgarası yok).
- `tile_census(tablo, zoom)`: düzeydeki karo sayısı ve karoların biçimleri (en çok 64 karonun ilk baytlarına bakılır, ada göre
  sıralı): `png`, `jpeg`, `webp`, `other` (`tile_format`).
- `tiles_in(tablo, zoom, sütunlar, satırlar)`: kutunun karoları, satır ve sütun sırasıyla.
- `write_tile_set(NewTileSet, karolar)`: kullanıcı tablosu (`id`, `zoom_level`, `tile_column`, `tile_row`, `tile_data`; zoom,
  sütun, satır tekil), `gpkg_contents`, `gpkg_tile_matrix_set` ve `gpkg_tile_matrix` satırları ve karolar tek işlemde; matrisin
  dışına düşen karo bütün yazmayı geri alır. Satır 0 üsttedir, sütun 0 soldadır; ızgaranın başlangıcı karo matrisi kümesinin
  sol üst köşesidir.

### 2. PNG yazıcısı (`kentos_formats::raster::png::encode`)

8 bitlik gri, gri+alfa, RGB ve RGBA PNG: satırlar süzgeçsiz, Deflate `write::code`'unun (miniz_oxide) zlib'iyle; CRC-32
tablosu kendi kodumuzda. Aynı örnek aynı dosyayı verir; kendi `png::decode`'umuzla gidiş-dönüş testi vardır. Yeni bağımlılık yok.

### 3. `cad.rasters.gpkgread` v1

Girdi: `path`, `table` (dosyanın tek karo tablosu varsa boş olabilir), `zoom` (yoksa en yüksek düzey), `area` (tablonun
sisteminde `[en küçük x, en küçük y, en büyük x, en büyük y]`), `output` (`.tif`), `layer`, `overwrite`, `drawing`,
`expectedRevision`. Çıktı: `tables` (dosyadaki her karo tablosu: karo matrisi kümesinin kutusu, veri kutusu, EPSG, her düzeyin
matrisi, pikseli, karo sayısı ve biçimi, uzantıları), okunan düzey, pencere, boyut, `affine`, `srid`, `tilesRead`,
`tilesMissing`, `path`, `bytes`, `uid`.

- **Pencere:** `area`, yoksa tablonun veri kutusu, o da yoksa karo matrisi kümesinin kutusu. Sınır GDAL'ın GeoPackage
  sürücüsünün kuralıyla pikselin tamına yuvarlanır: ilk piksel `floor((min − başlangıç) / piksel + ½)`, boy `floor((en büyük − en
  küçük) / piksel + ½)`; ızgaradan taşan yer kesilir, hiç değmiyorsa `tile_table_empty`. Rasterin yeri ızgaranın piksel
  sınırına oturur (GDAL veri kutusunun köşesini yerine koyar ve pikseli içeride kaydırır: ikisi yarım pikselden fazla ayrılmaz,
  piksel değerleri aynıdır).
- **Karolar:** PNG ve matrisin karo boyunda olmalı; 8 bitlik gri, gri+alfa, RGB, RGBA ve paletli karolar çözülür (`png::decode`).
  Çıktı her zaman dört bant (kırmızı, yeşil, mavi, alfa), 8 bit: gri karo üç renge çoğalır ve alfa 255 olur; karosu olmayan yer
  dört bantta da 0'dır, yani saydamdır (`tiles_missing` uyarısı). Karo PNG değilse ya da boyu başkaysa `tile_unreadable`; JPEG,
  WebP ya da 8 bitten fazla PNG ise `tile_format_unsupported`.
- **Sınır:** pencere 25 milyon pikseli aşarsa `gpkg_tiles_too_large` (daha kaba düzey ya da dar alan).
- **Sistem:** tablonun sistemi EPSG ise `srid`; tanımsız ya da başka kurumunsa 0 ve `crs_undeclared`. `layer` verilirse ve sistem
  projeninkinden başkaysa `crs_mismatch` (raster çevrilmeden eklenmez).
- **Yazma:** `output` yanında `.kentos-yaziliyor` adıyla yazılır, kendi okuyucumuzla geri okunup boyu ve bandı denetlenir, sonra
  yerine konur; yarım dosya kalmaz. `layer` ile raster `output`'a bağlı olarak eklenir; tek geri alma adımı “GeoPackage karoları”.
  `output` yoksa komut yalnız söyler (`tables`, pencere). `plan` GeoTIFF'in baytlarını kurar ama yazmaz.

### 4. `cad.rasters.gpkgwrite` v1

Girdi: `uid` (raster), `path` (`.gpkg`), `table` (`tiles`), `mode` (`new`, `add`), `identifier`, `overwrite`, `drawing`.

- **Kaynak:** 8 bitlik, kuzeyi yukarıda (`b = c = 0`, `a > 0`, `d < 0`), paletsiz, 1–4 bantlı raster; dosyası ya da kitaplık
  kaynağı okunabilmeli (`raster_unsupported`, `raster_not_north_up`, `file_not_found`, `unknown_asset`). Karo bantları: 1 gri, 2
  gri ve alfa, 3 ve 4 renkli (RGBA; alfa rasterde yoksa içeride 255). Rasterin nodata'sı karoda taşınamaz (`nodata_lost`).
  25 milyon pikseli aşan raster `gpkg_tiles_too_large`.
- **Piramit:** en ince düzey rasterin kendi pikseli, ızgaranın başlangıcı rasterin sol üst köşesi; her kaba düzey yarı çözünürlük:
  bir önceki düzeyin tuvalinin (karo ızgarasına sıfırla doldurulmuş) 2 × 2 ortalaması, her örnek `(a + b + c + d + 2) / 4`, tuvalin
  dışındaki örnek 0. En kaba düzey tek karodur. Düzeylerin sayısı `J`: bir kenarı 256'ya inene dek yarıya bölünme sayısı.
- **Matrisler:** karo matrisi kümesinin kutusu en kaba düzeyin tek karosunu kaplar (`256 · piksel · 2^J` bir kenar), veri
  kutusu rasterin kendi kutusu; her düzeyin matrisi kümenin bütün kutusudur (`2^(J − düzey)` karo bir kenar). GDAL'ın
  doğrulayıcısı piramidin buna uymasını ister (her matrisin kapladığı yer kümenin kutusu). Rasterin hiç değmediği karolar
  yazılmaz; okuyucular onları eksik (saydam) sayar.
- **Dosya:** `new` geçici adla yazılır, SQLite'tan okunup matrisler, kutu ve karo sayısı denetlenir, sonra yerine konur; var
  olan dosyanın üzerine yalnız `overwrite` ile yazılır (`file_exists`). `add` var olan GeoPackage'a tek işlemde tablo ekler:
  aynı adlı tablo `table_exists`; bir adım başarısız olursa dosya olduğu gibi kalır. Rasterin sistemi (EPSG kodu ve WKT'si) dosyaya
  yazılır; sistemsiz raster tanımsız kartezyen (−1). `plan` yazmadan söyler (`bytes` 0). Çizimi değiştirmez ve geri alma adımı
  değildir.
- **Adlar:** tablo adı harf ya da alt çizgiyle başlar, harf, rakam ve alt çizgi içerir, 64 karakteri geçmez ve `gpkg_`, `gpkgext_`,
  `sqlite_`, `rtree_` ile başlamaz (`invalid_table_name`).
- **Yeni kodlar:** `no_tile_table`, `zoom_not_found`, `invalid_area`, `tile_table_empty`, `gpkg_tiles_too_large`,
  `tile_unreadable`, `tile_format_unsupported`, `invalid_table_name`, `raster_unsupported`, `raster_not_north_up`;
  uyarılar `tiles_missing`, `nodata_lost`. Python'da `kentos.cad.rasters.gpkgread` ve `.gpkgwrite` katalogdan üretilir; MCP'de
  araçtır, ikisi de yıkıcı listededir (`overwrite` ve `mode add` var olan dosyayı değiştirir).

### 5. Masaüstü

Şeritte CAD › Ekle › Raster ile CBS › Raster › Raster panelinde ve CBS › Veri › Dosya alışverişi ▾'de **GeoPackage karoları…**
(`raster.gpkgRead`) ve **Rasteri GeoPackage karosu yaz…** (`raster.gpkgWrite`); ikonlar ızgara ve veritabanı silindiri (ok
yönü okuma ve yazmayı ayırır). Okuma penceresi dosyanın karo tablolarını ve zoom düzeylerini piksel, ızgara ve karo sayısıyla
radyo olarak sıralar, “Yalnız görünen alan” ve “Katman olarak ekle” ister ve komutun planını söyler (boyut, piksel, sistem, eksik
karo; proje başka sistemdeyse neden). Raster olarak ekle… GeoTIFF'in yerini sorar ve komutu çalıştırır: yeni katman ve raster tek adımda
geri alınır, dosya kalır. Yazma penceresi çizimdeki rasterleri sıralar, tablo adı ve “Var olan GeoPackage'a ekle” ister,
planı (düzey, karo sayısı, en ince piksel, karo bantları) söyler; Yaz… dosyayı sorar ve komutu pencerenin iş parçacığında çalıştırır.
Hesap yoktur: planlar ve yazmalar komutlarındır.

### 6. Bağımsız doğrulama

- `scripts/fixtures/gpkg_tiles_cases.py`: kaynaklar GDAL'ın GeoPackage sürücüsünün yazdığı dosyalardır (RGBA ve gri PNG karolar,
  JPEG karolar, iki karo tablosu, kaymış veri kutusu) ve `sqlite3` ile bozulanlar (eksik karo, PNG olmayan karo, başka boyda karo,
  16 bitlik karo, WebP başlığı, tanımsız sistem, okunamayacak kadar büyük matris, boş tablo); yazıcının kaynakları GDAL'ın
  GeoTIFF'leridir (RGB, RGBA, gri, gri ve alfa, nodata'lı, sistemsiz, coğrafi, 16 bit, paletli).
  - **Ortak durumlar:** `cad.rasters.gpkgread.json` 49, `cad.rasters.gpkgwrite.json` 33 durum, KentOS kodu olmadan yazılmış
    başvurudan (tablolar ve zoomlar `sqlite3`'ten, pencere ve piramit ADR'nin kuralından). Masaüstünde ve Python'da (tipli katman)
    aynı durumlar geçer.
  - **Dosyalar:** `gpkg_tiles_files` testi (`KENTOS_WRITE_GPKGTILES=1`) GeoTIFF'leri ve GeoPackage'ları `fixtures/gpkg-tiles/v1/out`'a
    yazar; betik `--check` onları GDAL'la açar: okunan her raster GDAL'ın aynı düzey ve penceredeki piksellerini bit bit taşır
    (yer ve boy GDAL'ınkiyle yarım pikselden fazla ayrılmaz, kaymış kutuda dahil); yazılan her GeoPackage GDAL'ın
    `validate_gpkg.py`'sinden hata vermeden geçer, GDAL en ince düzeyi rasterin kendi pikselleriyle okur ve her kaba düzeyi kuralın
    numpy ortalamasıyla, karo sayısı beklenenle aynıdır; QGIS 4.2 hepsini geçerli raster olarak açar (boy ve EPSG).
  - **İç döngü:** testte yazılan her GeoPackage okuma komutuyla geri okunur ve en ince düzeyi kaynağın pikselleriyle birebir aynıdır;
    yerine konamayan dosya (yerinde klasör olan yol) `file_unwritable` verir, geride yarım dosya ve çizimde adım kalmaz.
- **Rust:** crate'in karo gidiş-dönüşü ve matris dışı karonun bütün yazmayı geri alması; PNG yazıcısının gidiş-dönüşü; pencere ve
  piramit aritmetiği; masaüstünde pencerelerin testleri.
- **Resimler:** `gpkg-karo-*` (`exchange::gpkg_tiles_tests::screens`).

## Sonuçlar

- KentOS GDAL ve QGIS'in yazdığı GeoPackage karo tablolarını GDAL'ın okuduğu piksellerle okur; yazdığı karo tablosu GDAL'ın
  doğrulayıcısından geçer ve QGIS'te açılır.
- Okunan raster sıradan bir raster katmanıdır (GeoTIFF'e bağlı): stil, oturtma, COG olarak kaydet ve çözümleme araçları onunla
  çalışır. Karolar GeoPackage'tan canlı okunmadığı için dosya değişirse raster yeniden okunmalıdır.
- Sınır belleğe göre konduğu için çok büyük karo piramitleri bir pencere ya da kaba düzeyle okunur; şerit şerit okuma sonraya kaldı.
