# ADR 0599: Kaynak gezgini: dosyanın tabloları, veritabanları ve servisler, önizleme, arama, son ve sık kullanılanlar

- **Durum:** kabul edildi (2026-10-23). `plan/gis.md` CBS-01'in kalanı:
  - kaynak gezgininde dosyanın katmanları eklenmeden görülür;
  - PostGIS bağlantısı, raster klasörü ve servis katalogları gezginde;
  - ad, tür, sistem ve şema araması;
  - sık kullanılanlar, son kaynaklar, örnek satırlar, kapsam önizlemesi;
  - sürükleyerek ekleme;
  - aynı kaynak iki kez kopyalanmaz.

  Katman kartı (kaynak, yazma yeteneği, kimlik alanı, süzgeç, yükleme kapsamı, güncellik) ADR 0543'ün Kaynak durumu
  penceresi ve ADR 0546'nın Öznitelik tablosundaki kaynak satırıdır. Bu ADR'yle CBS-01 kapanır.
- **Bağlam belgesi:**
  - ADR 0199 §7: Kaynaklar paneli;
  - ADR 0503, 0505, 0510: bağlı katman, PostGIS, alanla bağlama;
  - ADR 0544, 0545: FlatGeobuf, GML;
  - ADR 0204, 0207: raster, nokta bulutu;
  - ADR 0208: harita servisleri ve hazır altlıklar;
  - ADR 0581: kaynağı sorgula.
- **`.kcad` şeması:** değişmez. Gezginin listeleri cihazındır (`kaynak-klasorleri.json` sürüm 2).
- **Web:** ertelendi (ADR 0228), TODOS.md §0.2.

## Bağlam

Kaynaklar paneli klasörlerdeki dosyaları ve KentOS projelerinin katmanlarını gösteriyordu. Bir dosyanın içinde ne
olduğu ancak içe aktarma penceresinde görülüyordu. GeoPackage'ın tabloları, bir veritabanının tabloları ve servislerin
katmanları gezginde yoktu. Belediye çalışanı doğru katmanı bulmak için dosyayı açıp kapatıyor, aynı tabloyu iki kez
bağlamaya çalışıp retle karşılaşıyordu. QGIS 4.2'nin Tarayıcı paneli (sağlayıcı başına kökler, katmanın özellikleri,
sürükle bırak, sık kullanılanlar) karşılaştırma ölçütüdür.

## Karar

### 1. Klasörlerin yeni türleri

Panelin eklediği dosyalar beş türle genişler:

- **GeoPackage** (`.gpkg`): açılabilir satırdır, tablolarını gösterir (§2). Günlük dosyaları (`.gpkg-wal`, `.gpkg-shm`)
  zaten uzantısıyla dışarıda kalır.
- **FlatGeobuf** (`.fgb`) ve **GML** (`.gml`): Katman olarak ekle kendi içe aktarma pencerelerini dosyayla açar.
- **Raster** (`.tif`, `.tiff`, `.vrt`): Raster ekle dosyayla açılır.
- **Nokta bulutu** (`.las`, `.laz`; `.copc.laz` de): Nokta bulutu ekle dosyayla açılır.

Kural `kentos_interaction::sources::source_kind`'dır; bağımsız başvuru `source_list_cases.py`'nin yeni durumudur.

### 2. Tablolar ve önizleme

GeoPackage dosyası açılınca `cad.sources.describe` dosyanın tablolarını verir, iş parçacığında. Her tablonun satırında:

- adı, kısa geometri sözcüğü (alan, çizgi, nokta, tablo), kayıt sayısı, EPSG kodu; tam tür ipucundadır;
- bağlıysa “Bağlı: <katman>” (§7).

Tablo açılınca önizleme gelir:

- **Kapsam:** kaynağın tablo için bildirdiği kutu, tablonun sisteminde. Bunu `SourceTable`'ın yeni `extent`'i verir:
  - GeoPackage'ta `gpkg_contents`'in `min_x … max_y`'si;
  - PostGIS'te `ST_EstimatedExtent` (istatistiği olmayan tabloda yok).

  Kapsam hesaplanmaz, bildirilir; satır “dosyanın bildirdiği” der. Kapsamı göster (§9) çizimde kutuyu gösterir.
- **Örnek satırlar:** ilk beş kayıt `cad.sources.query` ile (anahtar sırasıyla, `limit: 5`, ilk dört sütun), iş
  parçacığında.

Satırın eylemleri:

- **Bağlı katman olarak aç:** GeoPackage içe aktar penceresi yalnız o tablo seçili ve Bağlı katman olarak aç işaretli
  açılır. Sistem ve alan seçenekleri orada sorulur; Çalıştır `cad.sources.edit`'in `link`'idir.
- **Kopya olarak al:** aynı pencere, bağ işaretsiz.

Kopyalamak ayrı bir üründür; çift tık ve sürükleme bağlar. Tablo satırına basmak önizlemeyi açar.

### 3. Veritabanları

Veritabanları bölümü ADR 0623'ündür: cihazın adlı PostgreSQL bağlantıları (sunucu, veritabanı, kullanıcı, kayıtlı parola,
SSL; Bağlantıyı dene), açılınca katalogdan okunan şemaları ve tabloları; çift tık bağlı katman yapar. Bu ADR'nin ilk
taslağındaki ham `postgresql://` adres kutusu, aynı gün birleştirilen ADR 0623'ün bağlantılarına bırakıldı (23 Ekim).
Gezgin oraya şunları katar:

- arama bağlantının adında, sunucusunda, şemada, tablonun adında, geometri türünde, EPSG'sinde ve yorumunda bakar (§5);
- sık ve son kullanılan veritabanı tablosu bağlantının adıyla tutulur. Kullanılınca bağlantının kataloğu okunmuşsa tablo
  eklenir, okunmamışsa bağlantı açılır ve tablonun satırdan eklenmesi söylenir. Silinen bağlantı nedeniyle söylenir.

Kopya olarak al yoktur: veritabanı tablosunun kopyası Paketle'nin işidir (ADR 0596).

### 4. Servisler

Yeni “Servisler” bölümünde iki grup vardır:

- **Hazır altlıklar:** ADR 0208'in hazır altlıkları; bölüm kapalı başlar. Altlık olarak ekle altlığı yapar
  (`basemap.<ad>`'ın yolu).
- **Kayıtlı servisler:** cihazın listesi: tür, adres, ad ve eklendiği projenin bağlantısının kimliği.
  - Servis ekle… Harita servisi penceresini açar.
  - Pencerenin Ekle'si başarıyla biten her servis listeye kendiliğinden girer.
  - Servis açılınca yetenekleri okunur (`connect_now`, iş parçacığında, projenin aynı köklü bağlantısıyla).
  - Katmanlar satır satır listelenir; Katman olarak ekle Harita servisi penceresini o servis, o katman seçili ve bağlı
    olarak açar.

### 5. Arama

Panelin üstünde arama kutusu vardır. Sözcükler boşlukla ayrılır; her sözcük satırın bir alanında geçmelidir:

- büyük küçük harf Türkçe kuralla bir sayılır (I ı'nın, İ i'nin büyüğü);
- noktalı ve noktasız i de bir sayılır, çünkü panelin sözcükleri İngilizce de olabilir (MULTIPOLYGON, EPSG);
- ş, ğ, ç, ö, ü kendileridir.

Satırın alanları:

- adı;
- türünün adı;
- `EPSG:<kod>`;
- geometri türü;
- şeması;
- klasörün ya da dosyanın yolu.

Ağaçta eşleşen satır ve ataları görünür. Yalnız okunmuş satırlar aranır; panel bunu söyler. Kural
`kentos_interaction::sources::search`'tür.

### 6. Son kullanılanlar ve sık kullanılanlar

Panelin eklediği her kaynak son kullanılanlara girer:

- en yenisi önde, en çok 12 kayıt;
- yeniden kullanılan öne geçer.

Satırın menüsündeki Sık kullanılanlara ekle kaynağı sık kullanılanlara koyar:

- ekleme sırasıyla, en çok 50 kayıt;
- aynı menü çıkarır.

İki bölüm de boşken görünmez. Kaynak, ne olduğuyla tutulur:

- dosya;
- dosyanın tablosu;
- veritabanının tablosu;
- servis;
- servisin katmanı;
- hazır altlık.

Liste `kaynak-klasorleri.json`'un 2. sürümüdür. Sürüm 1 okunur.

Kaynağın yerinin denetimi:

- Yeri olmayan dosya satırda “bulunamadı” der. Eklenmeye çalışılınca neden söylenir; boş katman yapılmaz.
- Veritabanı ve servis açılınca denetlenir.

### 7. Aynı tablo bir kez bağlanır

`cad.sources.edit` aynı dosyanın aynı tablosunu ikinci kez bağlamayı zaten reddeder (`source_bound`). Gezgin bunu önce
söyler. Bağlı tablonun satırında Bağlı katman olarak aç yerine **Katmana git** vardır: katmanı Katmanlar'da seçer ve
gösterir.

Aynı veriyi başka bir stille görmek için bağ çoğaltılmaz. Görünümün teması kullanılır (ADR 0523): bir görünüm bir katman
durumudur, çizimin kopyasında çalışır.

### 8. Sürükleyerek ekleme

İki yol vardır:

- **Panelden:** bir dosya, tablo, servis katmanı, altlık, son ya da sık kullanılan satırı basılıp çizimin üstünde
  bırakılınca Katman olarak ekle'si çalışır. Panelde bırakılan sürükleme satırın tıklamasıdır. Sürüklenen satır
  işaretlenir, imleç tutan elle gösterilir.
- **Sistemin dosya yöneticisinden:** pencereye bırakılan dosya türüne göre (§1) Katman olarak ekle'nin yolundan gider.
  Panelin tanımadığı dosya nedeniyle söylenir.

### 9. Kapsamı göster

Tablonun önizlemesindeki Kapsamı göster:

1. Kutuyu projenin sistemine çevirir (`crs::transform_bounds`, ADR 0504). Sistemi bilinmeyen ya da çevrilemeyen kutu
   söylenir.
2. Görünümü ona getirir.
3. Çizimde kesikli bir çerçeve bırakır, sol üst köşesinde tablonun adıyla. Çerçeve kaydedilmez; Veride ara'nın işareti
   gibi İşareti kaldır (`data.unmark`) siler.

## Doğrulama

- **Panelin kuralları:** `source_list_cases.py` (KentOS kodu olmadan, `fixtures/sources/v1/cases.json`):
  - türler;
  - arama: sözcükler, Türkçe harfler, ağaçta atalar;
  - son ve sık kullanılanlar.

  `kentos-interaction`'ın `sources` testi oynatır.
- **`cad.sources.describe`'ın `extent`'i:**
  - GeoPackage: `bound_sources_cases.py` `gpkg_contents`'ten sqlite3'le.
  - PostGIS: `postgis_cases.py` ANALYZE'dan sonra `ST_EstimatedExtent`'le (`cad.sources.describe`'ın ve Python'un
    cevabında; Veritabanları'nın satırları ADR 0623'ün kataloğundandır).
- **Masaüstü:** `sources::tests` (11 test):
  - GeoPackage'ın tabloları, önizlemesi;
  - bağlama ve Katmana git;
  - Kopya olarak al'ın penceresi;
  - servis listesi;
  - arama;
  - son ve sık kullanılanlar;
  - sürükleme;
  - Kapsamı göster.
- **Resimler:** `.run/shots/kaynaklar-*`: gezgin, arama, kapsam, veritabanı ve servisler; iki temada, 1440×900 ve
  1100×650.
- **Python ve MCP:** yeni komut yok. Önizleme `cad.sources.describe` ve `cad.sources.query`'dir, ekleme
  `cad.sources.edit`, `file.import.*` ve `cad.layers.service`'tir; `extent` kataloğun şemasıyla Python'a gelir.

## Sonraya kalan

- Kurumsal katalog (CSW, STAC kataloğu, CBS-10/31).
- Raster ve bulutun meta verisinin gezginde önizlemesi. Bugün eklemeden önce Raster ekle ve Nokta bulutu ekle
  pencereleri gösterir.
