# ADR 0627: GeoPackage'ın kalanı — JPEG ve WebP karolar, yükseklik karoları, tabloya bağlı raster, ilişki ve stil profili

- **Durum:** kabul edildi (2026-10-24). `plan/gis.md` CBS-03'ün “Kalan”ı: canlı karo kaynağı, JPEG ve WebP karolar, yükseklik
  karoları, ilişki profili; planın “ilişki/stil/form paketinin GeoPackage standardı mı KentOS uzantısı mı olduğu belgeli
  olsun” ve “ağ paylaşımı çok yazarlı sunucu gibi sunulmamalı” ölçütleri.
- **Bağlam belgesi:** ADR 0502 (GeoPackage crate'i), 0503 (bağlı katman), 0532 (ilişki profili bağlı katmanda), 0555 (QML),
  0516 (SLD), 0556 (karo profili), 0204 (raster katmanları ve okuyucu), 0228 (web'e ara).
- **Yeni bağımlılık (sahibin onayıyla, 23 Ekim):** `image-webp` 0.2.4 (MIT ya da Apache-2.0; `image` ailesinin saf Rust WebP
  kodeği, kayıpsız ve kayıplı çözme, kayıpsız kodlama; `kentos-formats`'ta). `zune-jpeg` (ADR 0092'de onaylı) uygulama katmanına
  da girdi: başsız komutlar JPEG karoyu da okur.
- **Web:** ertelendi (ADR 0228); gerekenler TODOS.md §0.2'de.

## Bağlam

ADR 0556 karo tablolarını yalnız 8 bitlik PNG ile, seçilen düzeyi bir GeoTIFF'e yazarak okuyordu; JPEG ve WebP karolar,
yükseklik karoları (OGC 17-066r1) ve karoların dosyadan canlı okunması sonraya kalmıştı. GDAL'ın ve QGIS'in yazdığı
ortofotoların çoğu JPEG karoludur, sayısal yükseklik modelleri yükseklik karosudur. ADR 0532 çoktan çoğa başvuruyu bağlı
katmanda OGC 18-000 ilişki profiline eşlemişti; dosyaya alanın adını yazmamış, düz dışa ve içe aktarmada ilişki ve görünüş
taşınmıyordu. QGIS bir GeoPackage'ı açınca katmanın varsayılan stilini kendi `layer_styles` tablosundan alır.

## Karar

### 1. Resim karoları: PNG, JPEG ve WebP

- Biçim karonun ilk baytlarından (`raster::gpkg::format`): PNG, JPEG, WebP ya da TIFF. Bir tabloda biçimler karışık olabilir
  (GDAL'ın `TILE_FORMAT=AUTO`'su saydam karoyu PNG, diğerini JPEG yazar).
- **JPEG** `zune-jpeg` ile çözülür (ev sahibi: uygulama katmanı ve masaüstü). libjpeg-turbo'dan (GDAL) ters DCT ve renk
  örneklemesi yüzünden **en çok 4 düzey** ayrılır (ölçüldü, GDAL 3.12.2: `jpeg.gpkg` en çok 3, ortalama 0,32; karışık tabloda
  en çok 4, ortalama 0,10). Dosya denetimi JPEG karolu tabloda bu farka izin verir, diğerlerinde örnek örnek aynılık ister.
- **WebP** `image-webp` ile; kayıpsız ve kayıplı karolar libwebp'nin (GDAL) çözdüğüyle **bit bit aynı** (ölçüldü).
  Animasyonlu WebP okunmaz.
- Karonun başlığı çözülmeden denetlenir: tablonun türüne uymayan biçim `tile_format_unsupported` (resim tablosunda 8 bitten
  fazla PNG ya da TIFF; tam sayılı kapsamda 16 bitlik gri PNG olmayan; kayan noktalı kapsamda TIFF olmayan), başlığı okunmayan
  karo `tile_unreadable`; sözler sabittir (kütüphanenin iletisi değil), karonun yerini (zoom/sütun/satır) söyler.

### 2. Yükseklik karoları (OGC 17-066r1)

**Okuma, GDAL 3.12'nin sürücüsünün kuralıyla** (`ogrgeopackagedatasource.cpp`, `gdalgeopackagerasterband.cpp`; amaç bir tablonun
KentOS'ta, GDAL'da ve QGIS'te aynı okunması):

| Kapsam | Bandın türü | Değer | Nodata |
|---|---|---|---|
| `integer`, ölçek 1, öteleme 0, her karonun ölçeği 1 ve ötelemesi 0 | `u16` | saklanan | boş değer |
| `integer`, ölçek 1, öteleme −32768, karolar 0 ötelemeli (boş 65535 iken 0 ya da 1) | `i16` | ⌊(s·kö+ko)·ö+o+0,5⌋, [−32768, 32767] | boş 32767'den büyükse −32768, değilse boş |
| `integer`, ölçek 1, öteleme −32767, boş 65535, karolar düz | `i16` | aynı | −32768 |
| diğer `integer` | `f32` | (s·kö+ko)·ö+o; `precision` 1 iken (boşsa da 1) ⌊·+0,5⌋ | en küçük kayan nokta (−FLT_MAX) |
| `float` | `f32` | karonun değeri (ölçek ve öteleme uygulanmaz) | boşun 32 bitlik hâli |

- Tam sayılı kapsamın boş değeri saklanan bir değerdir: 0…65535 arası tam sayı değilse yok sayılır (GDAL da öyle).
- Eksik karo nodata'yla, nodata yoksa 0 ile dolar.
- GDAL'ın `precision` kuralı ölçekli tam sayılı kapsamın değerini tam sayıya yuvarlar (`kapsam-olcekli.gpkg`'nin 78 000 değerinden
  77 995'i bu yüzden ayrıydı; kural eklendi, şimdi aynı).

**Yazma** (`cad.rasters.gpkgwrite`, tek bantlı 8 bitten büyük raster):

- `u16` → `integer` kapsam, 16 bitlik gri PNG; boş değer rasterin nodata'sı (0…65535 tam sayıysa);
- nodata'sız `i16` → `integer`, öteleme −32768;
- diğerleri (`i16` nodata'lı, `u32`, `i32`, `f32`, `f64`) → `float`, 32 bitlik kayan noktalı TIFF karolar (küçük uçlu, tek şerit,
  LZW: GDAL'ın yazdığı ve doğrulayıcısının istediği biçim); boş değer nodata'nın 32 bitlik hâli; 32 bite yuvarlanan değer
  `precision_lost` ile söylenir;
- `gpkg_2d_gridded_coverage_ancillary` (`grid-value-is-center`, `precision` 1), üç uzantı satırı ve `gpkg_spatial_ref_sys`'te
  4979; WebP yükseklik karosu yazılmaz (`tile_format_unsupported`). Kaba düzeyler ve karo istatistikleri §5'te.

### 3. Tabloya bağlı raster (canlı karo kaynağı)

- `cad.rasters.gpkgread`'in `link: true`'su rasteri GeoTIFF yazmadan tabloya bağlı ekler (katman ister, `output` almaz:
  `link_unusable`). Rasterin `tiles`'ı `{table, zoom, x, y}`: tablonun adı, okunan zoom ve pencerenin ilk pikseli; `file`
  GeoPackage'ın yolu. `.kcad` şema 103 (`SCHEMA_WITH_RASTER_TILES`), `FORMATS_VERSION` 97; bağımsız Python okuyucusu ve
  örnekleri (`raster-tiles.kcad`, altı bozuk örnek).
- Pencere sola ve yukarı, en kaba düzeyin adımının katına kayar (2^K, K art arda ikiye katlanan zoom'ların sayısı): her kaba
  düzeyin pikseli bir zoom'un tam pikseli olur. **Yalnız karosu olan zoom'lar düzeydir** (GDAL'ın önizlemeleri gibi): GDAL'ın
  önizlemesiz yazdığı dosyada matriste karosuz zoom'lar durur; onlar düzey sayılsaydı uzaklaşınca raster boş görünürdü.
  Karosuz düzey okuyucunun kuralıyla hesaplanır (dört çocuğun nodata'sız ortalaması).
- Okuyucu protokolü (`LevelFrom::Tiles`, `TILES_FILE`): bir blok bir karodur; ev sahibi karonun baytlarını ölçeği ve ötelemesiyle
  (16 bayt) çerçeveleyip verir, okuyucu çözer; JPEG ev sahibinin çözücüsüne gider. Masaüstünde karolar iş parçacıklarında okunur:
  SQLite'tan blob kısa bir kilitle alınır, çözme kilit dışında yapılır.
- Sahnenin anahtarı `gpkg:<yol>#<JSON>` (tablo, zoom, pencere, rasterin boyu, bant sayısı, türü): aynı tabloya farklı boyda
  bağlanan iki raster ayrı okunur. Anahtar yolu çizimin klasöründe ve arama yollarında arar (taşınan dosya bulunur). Tablonun
  içeriği değiştiyse (bant, tür) raster açılmaz ve “yeniden bağlayın” der.
- Büyük (bir kenarı 4096'dan uzun) ve kaba zoom'ları olmayan tablo, GeoTIFF'teki gibi bir kez önizleme piramidi alır (cihazın
  önbelleği; adı dosyanın yolu, boyu, değişme zamanı ve tablosu).
- İşlem araçları bağlı rasteri okur (başsız `LocalFiles` ve masaüstü `DesktopFiles`: `raster_files::table_open`); Raster oturt
  reddeder (`not_a_raster`: yeri tablonun ızgarasındadır, oturtmak için GeoTIFF olarak okunur).

### 4. Akış ve sınırlar

- Okuma GeoTIFF'i karo karo okuyucunun üstünden yazar (bellek sınırlı; GeoTIFF'e en çok 2³² piksel, `gpkg_tiles_too_large`;
  bağlı okumada düzey başına en çok 4 milyar karo).
- Yazma piramidi şerit şerit kurar: ince şerit kesilmeden önce kaba şeride yarılanır (256 MiB şerit sınırı); resim karoları
  isteğe göre kayıpsız WebP (`format: webp`, `gpkg_webp` uzantısıyla).

### 5. Kapsamın kaba düzeyleri ve karo istatistikleri

- Kaba düzeyin değeri çocuklardan ince görüntünün içinde ve boş olmayanların ortalamasıdır: tam sayılı kapsamda saklanan
  değerlerle yarım yukarı ((2·toplam + n) / 2n), kayan noktada 64 bitlik ortalamanın 32 bitlik hâli; hiçbiri yoksa boş değer
  (yoksa 0; NaN'li çocukları olan boşsuz kayan nokta kapsamında NaN). Resim karolarının kuralı ADR 0556'nın: (a+b+c+d+2)/4,
  tuvalin dışı 0.
- Her yükseklik karosunun `gpkg_2d_gridded_tile_ancillary` satırı: ölçek 1, öteleme 0; geçerli ve düzeyin görüntüsü içindeki
  değerlerin (değer uzayında) en küçüğü, en büyüğü, ortalaması ve nüfus standart sapması; değeri olmayan karoda boş.

### 6. İlişki profili (OGC 18-000 ve KentOS belgesi)

- **Dışa aktarma** (`file.export.gpkg`): projenin iki katmanı da yazılan her ilişkisi ADR 0532'nin biçimiyle bir ilişki olur:
  `gpkgext_relations`'ta `kentos_references`, taban alt katmanın tablosu, ilgili üst katmanınki, eşleme tablosunun satırları alt
  ve üst kaydın fid'leri (alanın değerlerinden, `; ` ayrılmış kimlikler). Eşleme tablosunun adı `alt_üst` (`_2` …), ADR 0532
  gibi `gpkg_contents`'e yazılmaz (OGC 18-000'in sözü).
- OGC 18-000'in söylemediği **KentOS belgesidir**: `gpkg_metadata` (standart `https://kentos.app/relation/1`, JSON: eşleme
  tablosu, ilişkinin kimliği, adı, alanı, çok değerli mi, silme kuralı), alt tablonun alan sütununa sütun kapsamıyla bağlı
  (tablo kapsamlı başvuru `gpkg_contents`'teki bir tabloya gitmeli; eşleme tablosu orada değil). Bu bir **KentOS uzantısıdır**:
  diğer okuyucular onu atlar, ilişkiyi OGC 18-000'den bulur (GDAL'ın ilişki arayüzü ve QGIS 4.2 bulur).
- İlişkinin bir katmanı yazılmazsa ilişki yazılmaz ve söylenir; dosyaya yazılmayan üst kayda başvuru eşlemeye girmez, sayısı
  söylenir (`export_losses`). Cevabın `relations`'ı (eşleme, taban, ilgili, satır).
- **İçe aktarma** (`file.import.gpkg`): KentOS belgesi olan ilişki projenin ilişkisi olarak geri gelir (kimlik çakışırsa `-2` …,
  aynı katmanlar ve alanla bir ilişki varsa ikinci kez eklenmez, döngü ve 100 sınırı denetlenir); çok değerli alan yine çok
  değerli olur (sütun yalnız metin der).
- **Başka yazıcının** (QGIS, GDAL) çoktan çoğa ilişkisi: iki tablosu da alınıyorsa taban tablonun katmanında eşleme tablosunun
  adıyla çok değerli metin alanı açılır, değerleri ilgili kayıtların kalıcı kimlikleri (eşlemenin sırasıyla, `; ` ile) ve bir
  ilişki eklenir (kimliği eşlemenin adı, `_` yerine `-`; silme kuralı bağı kopar). Bir tablosu alınmayan ilişki alınmaz ve
  söylenir.
- Bütün dosya alınırken (`tables` boş) eşleme tabloları ve `layer_styles` katman olmaz (içerikleri alan ve görünüş olarak gelir);
  adıyla istenirse tablo olarak alınır. Bağlı katmanın (ADR 0503) ilişkileri ADR 0532'nin kuralında kalır.
- Cevabın `relations`'ı eklenen ilişkilerin kimlikleri.

### 7. Stil profili (QGIS'in `layer_styles`'ı)

- **Dışa aktarma:** kendi görünüşü olan her nesne tablosunun katmanının görünüşü QGIS'in varsayılan stili olarak yazılır:
  `layer_styles` QGIS 4.2'nin açtığı gibi (aynı DDL, `gpkg_contents`'te öznitelik tablosu), satırda QML (ADR 0555'in yazıcısı)
  ve SLD (ADR 0516'nınki), `useAsDefault` 1. Çizimin varsayılan görünüşündeki katmanın (işleyicisiz, temanın rengi) verecek
  görünüşü yoktur, yazılmaz: QGIS kendi varsayılanıyla çizer. Görünüşü QGIS'in simge katmanlarıyla söylenemeyen katman
  söylenir. Cevabın `styles`'ı.
- **İçe aktarma:** tablonun varsayılan stili (QGIS'in seçimi: işaretli olan, sonra en yeni) içe aktarmanın açtığı katmanın
  görünüşü olur (QML okuyucusu, ADR 0555; taşınan SVG çizimleri kitaplığa); çizimin var olan katmanı kendi görünüşünü korur.
  Alınamayan parçalar söylenir. Cevabın `styles`'ı.
- `layer_styles` bir OGC uzantısı değil, **QGIS'in kendi tablosudur** (GDAL korur).
- **Form yazılmaz:** formun alanları, ilişkileri ve kuralları projenindir; paketi `cad.forms.export`'tur (ADR 0435), GeoPackage'a
  konmaz.

### 8. Ağ paylaşımı

- `cad.sources.edit`'in bağlaması GeoPackage ağ dosya sistemindeyse `network_share` uyarısı verir (reddetmez): SQLite'ın kilitleri
  ağda güvenilir değildir, dosyaya aynı anda tek kişi yazmalı; birlikte düzenleme KentOS sunucusu ya da PostGIS'tir.
- Ağ dosya sistemi Linux'ta `/proc/self/mountinfo`'nun en derin bağlama noktasından (NFS, SMB/CIFS, sshfs, 9p, Ceph …), macOS'ta
  `mount`'un listesinden, Windows'ta UNC yolundan (`\\sunucu\pay`) anlaşılır; eşlenmiş Windows sürücüsü anlaşılmaz.

### 9. Masaüstü, Python ve MCP

- GeoPackage karolarını oku: **Okuma** seçeneği (GeoTIFF'e yaz ya da Tabloya bağlı; bağlıda Bağlı ekle, dosya sorulmaz, katman
  hep yeni), satırda içerik (resim ya da yükseklik, tür ve boş değer). Rasteri GeoPackage karosu yaz: resim rasterinde **Karo
  biçimi** (PNG ya da WebP), yükseklik rasterinde kapsamın notu; satırda kapsamın türü ve boş değeri.
- GeoPackage içe aktarma penceresi eşleme tablolarını ve `layer_styles`'ı katman olarak listelemez (aynı kural:
  `GeoPackage::side_tables`) ve dosyanın tablolarla gelecek ilişki ve QGIS stili sayısını söyler; içe ve dışa aktarmanın
  iletisi alınan ve yazılan ilişki ve stil sayısını söyler.
- Python ve MCP katalogdan: `link`, `format`, cevapların `relations` ve `styles`'ı.

## Doğrulama

- Ortak durumlar: `cad.rasters.gpkgread` 76, `cad.rasters.gpkgwrite` 49, `file.import.gpkg` 28 (beşi ilişki ve stil),
  `cad.rasters.georef` 39 (bağlı raster reddi); bağımsız başvurular `gpkg_tiles_cases.py`, `gpkg_import_cases.py` (QML'in görünüşü
  `qml_cases.py`'nin okuyucusundan), `raster_georef_command_cases.py`.
- Kaynaklar GDAL ve QGIS'le: `jpeg.gpkg`, `webp-gercek.gpkg`, `karisik.gpkg`, `kapsam-*.gpkg` (GDAL'ın kendi −32768 nodata'lı
  `i16`'sı dahil, altısı GDAL'ın kurallarını sınar), `merkator.gpkg`; `profil-qgis.gpkg` (GDAL'ın ilişki arayüzü ve QGIS 4.2.3'ün
  `saveStyleToDatabase`'i), `profil-kentos.gpkg` (başvurunun sqlite3'le yazdığı KentOS biçimi).
- Dosya denetimi (`KENTOS_WRITE_GPKGTILES=1 cargo test -p kentos-native-application --test all gpkg_tiles_files`, sonra
  `gpkg_tiles_cases.py --check`): 24 okunan raster GDAL'ın aynı zoom ve penceresiyle (türü ve nodata'sıyla), bağlı rasterin 12
  düzeyi (zoom'dan okunanlar GDAL'ınkiyle, hesaplananlar okuyucunun kuralıyla), 17 yazılan GeoPackage `validate_gpkg.py` ile,
  her zoom kuralın tuvaliyle ve kapsamın karo istatistikleri, QGIS 4.2.3 hepsini doğru boyda, sistemde ve kapsamın türüyle açar.
- İlişki ve stil: `KENTOS_WRITE_GPKG=1 cargo test -p kentos-headless --test headless geopackage` QGIS'in dosyasını alır, yazar,
  geri alır (aynı ilişki, alan değerleri ve görünüş); `gpkg_import_cases.py --check` yazılanı `validate_gpkg`, GDAL'ın ilişki
  arayüzü, eşlemenin satırları, KentOS belgesi ve QGIS 4.2.3 (varsayılan stil, `relationships()`) ile denetler.
- Masaüstü: bağlı tablonun anahtarı, açılışı ve iki-üç düzeyde uygulamanın okuduğu piksellerle aynılığı
  (`rasters::tests::a_linked_tile_table_draws_from_its_file`), pencereden bağlama, WebP ve kapsam yazma
  (`exchange::gpkg_tiles_tests`), QGIS'in dosyasının ilişkisi ve stiliyle alınması (`exchange::gpkg_tests`); ağ dosya
  sisteminin okunuşu `file_place`'in testlerinde. Resimler `gpkg-karo-*-7…11-*` ve `gpkg-*-12-profil-*`, `-13-profil-alindi-*`.
- Süreler (release, bu makinede: i5-11300H, 15 GB; `cargo test --release -p kentos-native-application --test all
  gpkg_tiles_timing -- --ignored --nocapture`): 4096² renkli rasterin PNG piramidi 1,9 s (341 karo, 13 MB), kayıpsız WebP
  piramidi 0,49 s (15 MB); 4096² `f32` yüksekliğin kapsamı 2,5 s (85 MB: kayan noktanın gürültülü bitleri LZW'yle sıkışmaz;
  karolar GDAL'ınki gibi öngörücüsüz LZW'li tek şerit TIFF); en ince düzeyin GeoTIFF'e okunması PNG'de 1,4 s, WebP'de 1,4 s, kapsamda 2,8 s; bağlı rasterin
  açılışı 1 ms'nin altında, en ince düzeyin ilk karosu 1–2 ms, en kaba düzeyin tamamı 1 ms.

## Sonraya kalan

- `gpkg_zoom_other` (ikiye katlanmayan zoom'lar), JPEG ve kayıplı WebP yazma, renk kuantalama.
- Alanların tanımı (değer listesi, zorunluluk; `gpkg_data_columns` uzantısı), formların GeoPackage'a konması.
- Başka yazıcının birden çoka ilişkisi (GDAL GeoPackage'ta yalnız çoktan çoğayı yazar) ve medya ilişkileri (ekler).
- Eşlenmiş Windows sürücüsünün ağ paylaşımı olduğunun anlaşılması.
