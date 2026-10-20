# ADR 0544: FlatGeobuf alışverişi

- **Durum:** kabul edildi (2026-10-20). `plan/gis.md` CBS-05'in bir dilimi. Kapsamı ben belirledim.
- **Bağlam belgesi:**
  - ADR 0502 (GeoPackage ve alma kuralı);
  - 0514 (Shapefile dışa aktarma);
  - 0522 (CSV alışverişi);
  - <https://flatgeobuf.org> (biçim, v3) ve GDAL'ın FlatGeobuf sürücüsü.
- **Web:** ertelendi (ADR 0228). Web'de gerekenler TODOS.md §0.2'de.
- **Sonraya kalan:**
  - bağlı katman olarak FlatGeobuf (dosya salt okunur akış biçimidir; düzenlemesi yeniden yazmaktır);
  - HTTP aralıklarıyla uzaktaki dosyanın görünen kutusunu dizinden okumak;
  - GeoParquet, GML ve XYZ yazımı (CBS-05'in kalanı).

## Bağlam

FlatGeobuf, GeoPackage'ın yanında GIS'in açık, hızlı ve tek dosyalık değişim biçimidir. QGIS ve GDAL onu yazar ve okur.
Dosya tek bir katmandır:
- büyülü sayı `fgb\x03fgb\x00`;
- başlık: ad, kutu, geometri türü, Z ve M, sütunlar, nesne sayısı, dizin düğüm boyu, sistem;
- isteğe bağlı paketli Hilbert R-ağacı;
- nesneler: geometri ve özellikler.

Başlık ve her nesne boyu önünde yazılı bir FlatBuffer'dır. Biçimin Rust kitaplığı geozero ve flatbuffers'a bağlıdır. Yeni bir
çalışma bağımlılığı onay ister (CLAUDE.md §3); biçim küçük ve iyi tanımlı olduğundan kendimiz yazdık.

## Karar

### 1. Kodek: `kentos_geopackage::fgb`

GeoPackage çekirdeğinin geometri ve değer türlerini kullanır (`Geometry`, `Curve`, `Value`, `ColumnKind`).

**Yazıcı.** FlatBuffers'ı ileri yönde yazar: tablo, vtable'ı hemen önünde, çocukları ardında. Her başvuru ileriye gösterir,
biçimin istediği gibi. Alanlar boylarına göre dizilir (8, 4, 2, 1). FlatBuffers'ın kendi `finish_size_prefixed`'i gibi değerler
boyun ilk baytından hizalanır, her tampon 8'in katıdır.

**Okuyucu.** Her başvuruyu, uzunluğu ve vtable'ı okumadan önce sınırlarla denetler. Hizayı sormaz, değerleri kopyalayarak okur:
yazarlar hizayı boy önekinden sayar, okuyucu öneksiz tamponu tutar. Bozuk dosya reddedilir, sonunun ötesi okunmaz. Başlık ya da
nesne en çok 1 GiB'tır, geometri en çok 32 düzey iç içedir. Dizin okunmaz, boyu biçimin formülüyle bulunup atlanır.

**Geometriler.** Nokta, çok nokta, çizgi, alan, çok parçalı çizgi ve alan, koleksiyon. Yaylar CircularString, CompoundCurve,
CurvePolygon, MultiCurve ve MultiSurface olarak kesin. Z ve M köşelerde. Tek halkalı ya da tek parçalı geometride `ends`
yazılmaz (GDAL gibi), okurken yokluğu tek parça demektir. Çokyüzlü yüzey, TIN ve üçgen okunmaz; nesnenin geometrisi
nedeniyle düşer.

**Sütunlar ve değerler.**
- Metin `String` (uzunluğuyla), tam sayı `Long`, ondalık `Double`, evet/hayır `Bool`, ikili `Binary`.
- Tarih ve tarih-saat `DateTime`. Tarih yalnız gün olarak yazılır. Okurken bütün değerleri gün olan tarih-saat sütunu tarih
  olur; GDAL tarihleri “T00:00:00”la yazar, onlar tarih-saat kalır.
- NULL özellik yazılmaz, boş metin yazılır: ikisi ayrıdır (CBS-06).
- 2⁵³'ten büyük tam sayılar GeoPackage alma kuralıyla metin alanı olur.

**Dizin.** Nesneler kutularının merkezinin Hilbert koduna göre yazılır (sıra yakın nesneleri yan yana getirir; dizinin
doğruluğunu etkilemez). Önlerine
16'lık düğümlerle paketli R-ağacı gelir: yapraklar nesnelerin bayt yeri, iç düğümler ilk çocuğun sırası. Konumu olmayan bir
nesne varsa dizin yazılmaz, düğüm boyu açıkça 0'dır. GDAL de konumsuz nesneyle dizin yazmayı reddeder.

**Karışık katmanlar.** Bir dosyada türü karışık katmanlar varsa başlığın türü `Unknown`'dur, her nesne kendi türünü taşır.
Z'si olan bir nesne varsa bütün dosya Z'lidir; Z'siz köşenin Z'si NaN yazılır ve geri okununca yine yoktur: KentOS'ta kotsuz
köşe 0 kotlu değildir.

### 2. Komutlar

`file.export.fgb`:
- katmanların (boşsa nesnesi olan bütün katmanların) nesneleri tek dosyada;
- birden çok katmanda `katman` sütunu;
- projenin sistemi EPSG koduyla;
- yazı, ölçü, tablo, resim, raster ve nokta bulutu sayılarak dışarıda, elips, eğri ve NURBS kirişlerle (`export_losses`);
- bütün yazılır ya da hiç, plan yazmaz;
- başsız sunucunun dosya komutudur (`file_commands`), çizimi değiştirmez.

`file.import.fgb`:
- GeoPackage alma yolundan (`file_import_gpkg::run_read`): katman, alan, kalıcı kimlik (UUIDv5 `gpkg:fgb/ad.fgb:sıra`), sistem
  denetimi, `reproject` ve kayıplar aynı;
- tek geri alma adımı “FlatGeobuf: ad.fgb”;
- bozuk dosya `import_unreadable` ile reddedilir.

### 3. Masaüstü, Python ve MCP

CBS › Veri › Dosya alışverişi'nde FlatGeobuf içe aktar… ve FlatGeobuf dışa aktar… pencereleri:
- içe aktarmada komutun planı ve Projenin sistemine çevir;
- dışa aktarmada nesneler, sütunlar ve türleri, geometri türü, dizin ve yazılmayanlar.

Python ve MCP komutları katalogdan alır; iki komut da MCP'nin yıkıcı listesindedir.

## Doğrulama

- `scripts/fixtures/fgb_exchange_cases.py` (GDAL'lı Python; macOS'ta QGIS'inki):
  - dosyaları GDAL QGIS 4.2.3'ün GeoPackage'ından yazar: dizinli ve dizinsiz parseller, yaylı yol, kotlu nokta, eğri alan,
    bileşik eğri, kayıtlar, WGS 84, sistemsiz, boş;
  - bozuk ve kesik dosyalar ayrıca, kesik dosyanın yeri biçimin tanımından;
  - 22 ortak durum (`fixtures/commands/v1/file.import.fgb.json`) OGR'ın okuduğundan GeoPackage alma kuralıyla.
- `--check` ayrıca KentOS'un yazdığı dosyaları OGR'la açar (`KENTOS_WRITE_FGB=1 cargo test -p kentos-headless --test headless
  flatgeobufs`):
  - geometriler kaynağın GeoPackage'ındakilerle eşit;
  - türler ve EPSG:5256 doğru;
  - dizinli dosyada OGR'ın dizinden süzdüğü, bütün nesneleri gezen süzgeçle aynı;
  - KentOS'un okuyup yeniden yazdığı GDAL dosyaları OGR'da GDAL'ın kendileriyle aynı (`KENTOS_WRITE_FGB=1 cargo test -p
    kentos-geopackage --lib fgb`, `fixtures/fgb/v1/geri`).
- Masaüstü: `cargo test -p kentos-desktop exchange::fgb_tests`; resimler `exchange::fgb_tests::screens`, `.run/shots/fgb-*`.
