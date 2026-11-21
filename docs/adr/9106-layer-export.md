# ADR 9106: Katmanı dışa aktarma

- **Durum:** kabul edildi (2026-11-21). Plan: [plan/veri.md](../../plan/veri.md) VY-08.
- **İstek:** sahibin sözü (20 Kasım 2026): “katmanlarda sağ tuşa dışa aktarma eklenen gerek qgis gibi ve açılan dialog
  ile tıpkı qgis gibi hatta daha detaylı aktarma seçenekleri”. 21 Kasım: “Alan sayısı çok fazla olduğunda aşağıya doğru
  uzar gider o yüzden alanların olduğu bölgede ayrı bir kaydırma çubuğunun daha olması iyi olur”.
- **Web:** ertelendi (ADR 0228). **`.kcad`:** değişmedi. **Komut:** yeni `cad.layers.export` v1.
- **Sürücü sözleşmesi:** `kentos_data::NewTable.system` (`SystemText`: sistemin adı ve OGC WKT 1). GeoPackage
  sürücüsü yeni tablonun tanıtıcısını benzersiz yapar (§4).
- **Bağımlılık:** yeni dış kitaplık yok.

## §1 Bağlam ve araştırma

**Masaüstü CBS** (QGIS 4.2; kullanım kılavuzu “Creating new layers from an existing layer”, 21 Kasım 2026'da okundu;
kurulu programın ekranları `.run/gui-evidence/veri/qgis/08`, `11`, `12`):

- Katmanın sağ tuşunda Export › Save Features As… ve Save Selected Features As….
- Pencere: Format, File name, Layer name (yalnız birden çok katman tutan biçimlerde), CRS, Encoding, Save only selected
  features. Select fields to export: alanı aç kapa, Select All, Deselect All, dışa aktarılan adı düzenleme, alan başına
  “Replace with displayed values” (değer listesinin kodu yerine açıklaması).
- Persist layer metadata, Geometry (Geometry type: Automatic ya da başka tür, geometri sütunu ekle kaldır; Force
  multi-type; Include z-dimension), Extent, Layer Options (GeoPackage'ta DESCRIPTION, FID, GEOMETRY_NAME, IDENTIFIER,
  SPATIAL_INDEX), Custom Options, Add saved file to map.
- Var olan dosyada üstüne yazma, katmanı değiştirme ya da ekleme sorulur. Doğrudan PostGIS tablosuna yazmak bu
  pencerede yoktur (ayrı araç).

**Türkiye:**

- TUCBS Birlikte Çalışabilirlik Usul ve Esasları Rehberi (TUCBS_BC, ÇŞB CBS Genel Müdürlüğü, sürüm 1.0, 2020; dayanak
  49 Numaralı Cumhurbaşkanlığı Kararnamesi):
  - ilke 6 “Açık Standartların ve Uluslararası Standartların Kullanımı”;
  - §3.2.11: “Dosya biçimleri uzun süre erişilebilirliği garantileyebilecek şekilde seçilmeli”;
  - §4.1.3.2 Açık Veri: veriler “yazılımlar tarafından okunur ve lisans gerektirmeyen bir biçimde” üretilir.

  Hedef biçimlerin hepsi açık standarttır (OGC GeoPackage, GeoJSON RFC 7946, OGC KML, CSV RFC 4180). Shapefile yaygın
  okunduğu için vardır.
- TUCBS Veri Tanımlama Şablonu (TUCBS_VTS) §6.1: yatay koordinat referans sistemleri TUREF (ITRF96, 2005.0) üzerindedir:
  TUREF-UTM, TUREF-TM, TUREF-LKK. Yazılan GeoPackage sistemi kodu, adı (“TUREF / TM36”) ve tanımıyla taşır; başka bir
  program sistemi oradan okur (§6).

## §2 Karar: komut

`cad.layers.export` bir katmanın nesnelerini (çizim katmanı) ya da satırlarını (veri katmanı) yazar.

- **Hedef:** `format`:
  - `gpkg`: GeoPackage dosyasının (`path`) tablosu; dosya yoksa yapılır;
  - `postgis`: veritabanının (`address`) `schema` şemasındaki tablo;
  - `geojson`, `shapefile`, `csv`, `kml`: dosya.

  `name` tablo adıdır (KML'de belge adı). Verilmezse katmanın adından üretilir: Türkçe harfler sadeleşir, küçük harf,
  alt çizgi.
- **Ne yazılır:**
  - `uids`: çizim katmanının seçili nesneleri; `keys`: veri katmanının seçili satırları;
  - `bbox`: kutuya değenler;
  - `filter`: İfadeyle seç dilinde koşul.

  Katman süzgecinin gizlediği nesne yazılmaz.
- **Alanlar:** `fields` sırayla seçilenlerdir. `to` yazılan ad; `labels` listeli değerin kodu yerine görünen adı.
- **Geometri:**
  - `geometry` (`auto` katmanınki, `point`, `line`, `area`, `none` geometrisiz tablo);
  - `multi`, `z` (kotsuz nesne 0), `m`;
  - `srid`: yazılan sistem; verilmezse veri katmanının sistemi, çizim katmanında projeninki.
- **Biçime özel (`options`):**
  - GeoPackage ve PostGIS: tablonun açıklaması, anahtar ve geometri sütununun adı, mekânsal dizin;
  - CSV: ayırıcı, noktada X ve Y sütunları (WKT yerine);
  - GeoJSON: proje koordinatları (eski `crs` üyesiyle).

  `encoding`: Shapefile ve CSV için UTF-8 ya da Windows-1254.
- **Hedef varsa (`existing`):**
  - `refuse`: hiçbir şey yazılmaz (`target_exists`);
  - `overwriteFile`: dosya yeniden yazılır;
  - `replaceLayer`: tablo yeniden yapılır, dosyanın öbür tabloları kalır;
  - `append`: satırlar eklenir; tabloda olmayan alan bırakılır, uyarılır (`field_left_out`).
- **Diğer:** `metadata` katmanın veri kümesi bilgisini GeoPackage tablosunun üst verisi yapar. `addToMap` yazılan
  tabloyu veri katmanı olarak ekler, aynı geri alma adımında.
- **Çıktı:** biçim, dosyalar, tablo, yazılan sayı, alanların yazılan adları, geometri türü (`POLYGON Z` gibi), sistem,
  eklenen katman.
- **Uyarı ve retler:**
  - uyarılar: `export_skipped` (yazılamayan nesneler sayılır, ilkinin nedeniyle), `field_left_out`;
  - retler: `no_target`, `invalid_option`, `target_exists`, `nothing_to_export`, `cancelled`.

  `plan` her satırı yapar, hiçbir şey yazmaz.

## §3 Karar: satırlar nasıl yapılır

- **Çizim katmanı:** nesneleri belgeden alınır.
- **Veri katmanı:** satırları kaynaktan 2048'lik parçalarla okunur; okunan parça yazılır. Bir milyon satırlık tablo
  hiçbir an bütün olarak bellekte durmaz. Kutu ve seçili anahtarlar kaynağa gider.
- **Süzgeç** her parçaya İfadeyle seç'in kendi motoruyla sorulur (ADR 9102 §4'teki gibi, SQL'e çevrilmez). Kaynak
  yalnız seçilen alanları ve süzgecin okuduklarını okur.
- **Satır**, düzenleme oturumunun kaydettiği gibi yapılır (`data_write::Rows`):
  - alan yeniden adlandırılır, görünen değer yazılır, değer alanın türüne getirilir;
  - geometri istenen türe getirilir (alanın çevresi çizgi, kapalı çizgi alan); yay kirişlenir;
  - sisteme taşınır, Z ve M istendiği gibi, istenirse çok parçalı.

  Yazılamayan nesne (yazı, ölçü, türe getirilemeyen) bırakılır ve sayılır.

## §4 Karar: hedefler

- **GeoPackage ve PostGIS tablosu** sürücülerden geçer (`create_table`, `insert`) ve tek işlemle yazılır.
  - **Geçici dosya:** yapılacak GeoPackage önce yanına (`<dosya>.part`) yazılır, bitince yerine konur. Durdurulan ya da
    başarısız yazmada bu dosya silinir, işlem geri alınır: üstüne yazılacak dosya olduğu gibi kalır.
  - **Tanıtıcı:** tablonun tanıtıcısı (`gpkg_contents.identifier`) tablonun adıdır; eklenen katmanın adıyla aynıdır.
    Tanıtıcı benzersizdir: başka tablo o adla listeliyse sürücü “ad (tablo)”, sonra “ad (2)” verir. Önceden tanıtıcı
    katmanın adıydı: aynı katmanı aynı dosyaya iki tablo olarak yazmak UNIQUE hatasıyla düşüyordu.
  - **Sistem:** yeni sistem satırı adı ve OGC WKT 1 tanımıyla yazılır (`NewTable.system`;
    `geopackage.rs::srs_of` ile aynı kaynak). Önceden `EPSG:5256` adı ve `undefined` tanımıyla yazılıyordu. PostGIS
    kendi kataloğunu kullanır, bu alanı okumaz.
  - **Üst veri:** işlemden sonra yazılır (`gpkg_profile::DATASET_STANDARD`).
- **CSV** satırlar geldikçe yazılır; o da `.part` dosyasıyla yazılır ve sonunda yerine konur.
- **GeoJSON, Shapefile, KML** biçim çekirdeğinin yazıcılarıyla yazılır:
  - GeoJSON RFC 7946 (WGS 84) ya da proje koordinatlarıyla;
  - Shapefile `.prj` (ESRI WKT) ve `.cpg` ile;
  - KML WGS 84.

  Bu yazıcılar katmanı bütün olarak alır.

## §5 Karar: masaüstü penceresi

- **Menü:** katmanın sağ tuşunda Dışa aktar ▸ Farklı kaydet… ve Seçilenleri farklı kaydet…. İkincisi katmanın seçimi
  yokken soluktur. Veri katmanının seçimi tablonun seçili satırlarıdır, yalnız o katmanınkiyse.
- **Pencere bölümleri:**
  - Biçim; Dosya (Gözat…) ya da Bağlantı ve Şema; Tablo adı; Koordinat sistemi (EPSG); Kodlama;
  - Yazılacak nesneler: yalnız seçilenler, yalnız görünümdekiler, süzgeç;
  - Alanlar; Geometri; biçimin seçenekleri;
  - Hedef varsa; üst veri; haritaya ekle.

  Biçimde olmayan seçenek gösterilmez.
- **Alanlar:** üstte Tümünü seç, Hiçbirini seçme ve “24 alandan 24 tanesi yazılacak”. Altında başlıklı bir liste: Alan
  (kutu ve ad), Türü, Yazılacak adı, listeli alanda Görünen değer. Liste en çok yaklaşık sekiz satır yüksekliğindedir
  (264 piksel, yazı boyutuyla büyür), ötesi kendi kaydırma çubuğuyla kayar. Başlık sabit kalır. 24 alanlı katman
  pencereyi uzatmaz (sahibin 21 Kasım isteği).
- **Çalışırken:**
  - Yazma arka planda yürür. Pencerenin alt çubuğu ilerlemeyi söyler: “251.904 / 1.000.000 kayıt okundu, 62.976 nesne
    yazıldı (%25)” ve çubuk. Toplam yazmadan önce kaynakta sayılır (kutu ve anahtarlarla).
  - Vazgeç'in yerinde Durdur durur: yazma durur, işlem geri alınır, `.part` silinir. Alt çubuk bunu hata olarak değil
    bilgi olarak söyler: “İş durduruldu; hiçbir şey yazılmadı.”. Pencereyi kapatmak da durdurur.
  - Ret ve hata alt çubukta görünür. Uzun formun sonunda kalmaz.
- **Bitince:** bildirim “Dışa aktarıldı: GeoPackage konut.gpkg › konut: 250.000 nesne yazıldı (alan).”. Geometri türü
  Türkçe söylenir (`MULTIPOLYGON Z`: “çok parçalı alan, Z”). Haritaya eklenen katman tablonun adını alır.

## §6 Kanıt

- **Ortak durumlar:** `cad.layers.export` için 31 durum
  (`scripts/fixtures/layer_export_command_cases.py`). Rust ve Python koşucuları aynı dosyayı okur. Koşuculara dosya
  okuma adımı (`readFile`: json, windows1254, metin) eklendi.
- **`kentos-data-gpkg`:** sistem adı ve tanımı yazılır; tanımı olmayan kodla adlanır; ikinci tablo kendi tanıtıcısını
  alır (4 sınama).
- **Masaüstü `layer_export_tests`:**
  - pencereden komut girdisi; süzgeçle GeoPackage'a yazma ve haritaya ekleme; tanıtıcı ve sistem satırı; ilerleme
    (40 / 40);
  - bayat cevabın atılması;
  - ilk parçadan sonra durdurma: eski dosya bayt bayt aynı, `.part` yok, pencere nedeni söyler.
- **Gerçek pencere:**
  - bir milyon satırlık PostGIS tablosundan `kullanim = 'Konut'` GeoPackage'a yazıldı: 250.000 satır. Hata ayıklama
    derlemesinde yaklaşık 33 saniye: 7,5 saniyede %25, 15 saniyede %47, 30 saniyede %93;
  - Durdur 250 milisaniye içinde yanıt verdi, hiçbir dosya kalmadı; ikinci çalıştırma bitti ve katman haritaya eklendi;
  - 24 alanlı GeoPackage katmanında alan listesi kendi içinde kaydı.
- **Başka bir CBS kitaplığıyla okuma:** yazılan dosya geçerli; sistem EPSG:5256 “TUREF / TM36”; 250.000 Polygon; alan
  türleri doğru; ilk satırın alanı 676 m².
- Bu sınamada bulunup düzeltildi:
  - sistem satırı `undefined` yazılıyordu;
  - tanıtıcı çakışması;
  - durdurulan yazma yeni dosyayı bırakıyordu, üstüne yazılan dosyayı ise baştan siliyordu;
  - çalışırken pencerede hiçbir ilerleme görünmüyordu, ret uzun formun sonunda kalıyordu;
  - alt menünün simgesi yoktu;
  - Seçilenleri farklı kaydet başka katmanın tablo seçimini alabiliyordu.

## §7 Kalan

- ~~KentOS'un 32 bitlik tam sayısı GeoPackage'ta `INTEGER` (64 bit) yazılır.~~ Kapandı (VY-11,
  [ADR 9108](9108-integer-widths-and-groups.md) §2): Tam sayı `MEDIUMINT`, Büyük tam sayı `INTEGER` yazılır.
- ~~Süzgeç kaynağa gitmez.~~ Kapandı (VY-12, [ADR 9109](9109-expression-to-the-source.md) §3): süzgecin gönderilebilir
  parçası kaynakta sorulur, satırlar ve toplam onunla okunur; süzgeç yine burada da sorulur.
- GeoJSON, Shapefile ve KML yazıcıları katmanı bütün alır. Milyonluk veri katmanında bellek ister; akışlı yazıcı
  ileride.
- PostGIS hedefi kayıtlı bir bağlantı ister (PostGIS'e bağlan). Kapsam yalnız görünümdür (QGIS'teki serbest kutu yok).
  Sistem EPSG koduyla verilir, seçici yok. DXF, KML ve TAB'a sembol yazma yok.
- Web (ADR 0228).
