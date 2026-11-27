# ADR 9338: GeoJSON dışa aktar ve Paketle veri katmanlarının kaynak satırlarını kapsar

- **Durum:** kabul edildi (2026-11-27). Kaynak: CBS kullanım incelemesi, 27 Kasım 2026 (`gis-eksik-27-11-2026.md`, madde
  2 ve 5, P1). Sahibin sözü: “Gis eksik 27 11 dosyasındaki maddeleri tamamla”.
- **İlgili:**
  - [ADR 9101](9101-data-layers.md) (veri katmanı: satırlar kaynaktadır) ve [ADR 9196](9196-storage-by-project-kind.md)
    (CBS verisi proje dosyasına yazılmaz);
  - [ADR 0046](0046-geojson-and-shapefile.md) ve [ADR 1077](1077-geojson-export-rfc7946-default.md) (GeoJSON dışa aktar, RFC 7946);
  - [ADR 0506](0506-relative-sources-and-package.md), [ADR 0596](0596-package-sources-offline.md) ve [ADR 2780](2780-delivery-package-open-files.md)
    (Paketle, bağlı kaynaklar, devir dosyaları);
  - [ADR 9107](9107-project-in-a-source.md) §3 (veri katmanının dosyası proje klasöründen göreli yolla bulunur);
  - [ADR 9106](9106-layer-export.md) (`cad.layers.export`).

## Bağlam

- **Madde 2:** yalnız GeoPackage'a bağlı veri katmanları olan projede Dosya › Dışa aktar › GeoJSON “Görünen katmanlar
  (0)” ve “Tümü (0)” gösterdi; Dışa aktar kapalı kaldı. Aynı anda harita çizgileri, alt tablo satırları gösteriyordu.
  - Pencerenin kapsamları yalnız çizimin nesnelerini sayıyordu (`scope_entities`). Veri katmanının satırları
    kaynaktadır, çizimin nesnesi değildir.
- **Madde 5:** aynı projede Dosya › Paketle › GeoPackage “Seçilen katmanlarda GeoPackage'a yazılacak nesne yok”
  hatası verdi. Kodda iki eksik bulundu:
  - devir dosyaları çizimin nesnelerinden yazılıyordu; veri katmanlarının satırları girmiyordu;
  - paket veri katmanlarının kaynağını hiç taşımıyordu. Paketle yalnız eski bağlı katmanların (`settings.sources`)
    dosyalarını kopyalıyordu; veri katmanının (`layer.data`) GeoPackage'ı pakete girmiyor, paket başka yerde açılınca
    katman boş kalıyordu.
- **İncelemenin kabulü:**
  - GeoJSON: dışa aktarma GeoPackage, PostGIS ve servis katmanlarının satırlarını kapsasın; katman, seçim, görünüm ve
    süzgeç ayrı kapsamlar olarak söylensin; sayım arka planda olsun; karışık geometri önceden söylensin; EPSG:5256
    kaynağından RFC 7946 boylam, enlem ve proje koordinatı ayrı ayrı doğrulansın; okunamayan kaynağın katmanı
    söylensin.
  - Paketle: devir dosyasının kaynak tabloları kapsayıp kapsamadığı klasör sorulmadan gösterilsin; GeoPackage, PostGIS
    ve salt okunur servis için kopya ile başvuru ayrılsın; paket başka klasörde, kaynağın eski yolu yokken açılsın;
    3, 3, 1 satır ve CRS doğrulansın; olmayan çıktıda ara dosya kalmasın.

## Araştırma

- **QGIS, katmanı farklı kaydet:** bir vektör katmanı kaynağı ne olursa olsun (dosya, veritabanı, servis) yeni bir
  dosyaya yazılır; yalnız seçilen nesneler ve hedef CRS seçilebilir
  ([Creating new layers from an existing layer](https://docs.qgis.org/3.44/en/docs/user_manual/managing_data_source/create_layers.html)).
- **QGIS, Package layers işlemi:** katmanları tek bir GeoPackage'a toplar; seçilenler, stiller ve ilişkili katmanlar
  seçeneklidir ([algoritmanın kaynağı](https://api.qgis.org/api/3.44/qgsalgorithmpackage_8cpp_source.html)).
- **ArcGIS Pro, proje paketi:** kurum içinde paylaşılan pakette kurumsal veritabanının verisi başvuru olarak kalır;
  “kurum dışında paylaş” seçilince veri pakete dosya veritabanı olarak kopyalanır. Başvuruyla kalan paket, kaynağa
  erişimi olmayan yerde kırık bağlarla açılır
  ([Share a project package](https://pro.arcgis.com/en/pro-app/3.6/help/sharing/overview/project-package.htm)).
- **RFC 7946 §4:** GeoJSON'un koordinat sistemi WGS 84 boylam ve enlemidir; `crs` üyesi kaldırıldı
  ([RFC 7946](https://www.rfc-editor.org/rfc/rfc7946#section-4)).
- **KentOS kararı:**
  - Dışa aktarmanın kaynağı, haritanın gösterdiği satırlardır; veri katmanı da çizim katmanı gibi kapsama girer.
  - Pakette kopya ile başvuru kaynak türüne göre ayrı seçilir (KentOS'ta önceden de böyleydi; artık veri katmanları da
    aynı seçime uyar). Veri katmanı gömülmez: CBS verisi proje dosyasına yazılmaz (ADR 9196).

## Karar

### 1. GeoJSON dışa aktar: kapsamlar kaynak satırlarını sayar

- **Kaynak satırları** (`shell/exchange/source_rows.rs`): pencere açılınca her veri katmanının satırları
  kaynaktan istenir (ADR 9130'un yolu: bütün tablo, arayüzün iş parçacığı dışında okunur, çizim değişmedikçe saklanır).
  Satırlar gelince pencere kapsamlarını yeniden toplar. Çizimin tuttuğu satırlar (düzenleme oturumunda alınmış ya da
  yeni olanlar) çizimin nesnesi olarak gelir; kaynaktaki eşleri iki kez sayılmaz.
- **Kapsamlar:**
  - **Seçili:** çizimde seçilenler ve veri katmanının tablosunda seçilen satırlar.
  - **Görünen katmanlar:** haritanın çizdiği; görünür katmanlar, katman süzgeçleriyle (süzgeç kaynak satırlarına da
    uygulanır). Katman tablosunda süzgeçli katman “kaynaktan, süzgeçten 2/3” der.
  - **Tümü:** bütün katmanların bütün nesneleri ve kaynak satırları.
  - Katman tablosu (katman kapsamı) her kapsamda katmanları tek tek bırakmaya yarar; veri katmanının satırı
    “kaynaktan” der.
- **Okuma:** bir kaynak okunurken özet “Kaynaktan okunuyor: …” der, sayılar okunanla gelir, Dışa aktar okuma bitene
  kadar çalışmaz. Okunamayan kaynak özetin hata satırında adıyla ve nedeniyle söylenir. O katman yazılmaz, ötekiler
  yazılır; yazımdan sonra komut satırı da katmanı adıyla söyler.
- **Karışık geometri:** nokta, çizgi ve alan birlikte yazılacaksa özet “Karışık geometri (…) tek dosyada yazılır”
  der.
- Koordinatlar ADR 1077'deki gibidir: RFC 7946 (WGS 84'e çevrilir, `crs` yok) ya da projenin koordinatları (eski `crs`
  üyesiyle).

### 2. Paketle: veri katmanlarının kaynağı

- **GeoPackage** (`geopackages`):
  - **Kopyala:** dosya bayt bayt `veri/` altına bir kez kopyalanır, kaç katman okursa okusun. Her katmanın adresi
    paketteki kopyadır; proje klasöründen göreli yolu `veri/<ad>`'dır. Paket taşınınca proje açılırken dosya bu
    yoldan bulunur (ADR 9107 §3).
  - **Göm:** veri katmanının dosyası gömülmez, kopyalanır; seçeneğin açıklaması bunu söyler.
  - **Bağlı bırak:** katman dosyayı yerinde adlandırır; paket raporunda “bağlı bırakıldı” olarak söylenir.
- **Veritabanı** (`databases`):
  - **Kopyala:** katmanın satırları `cad.layers.export` ile `veri/<tablo>.gpkg`'ye yazılır. Katman o tabloyu okur;
    paket veritabanına ulaşmadan açılır.
  - **Bağlı bırak** ya da **Göm:** katman veritabanını parolasız adıyla adlandırır.
- **Servis katmanları:** adresleriyle kalır; salt okunurdur, verisi kopyalanmaz ve devir dosyalarına girmez.
- **Pencere** (klasör sorulmadan önce):
  - “Veri katmanları” listesi her katmanın kaynağını, satır sayısını (kaynaktan sayılır, sayılırken “sayılıyor…”) ve
    paketin ona ne yapacağını söyler. Servis katmanları ayrı bir satırdadır.
  - GeoPackage ve Veritabanı kaynakları seçenekleri veri katmanı varken de çıkar.

### 3. Devir dosyaları kaynak satırlarını taşır

- Devir dosyaları istenince her veri katmanının satırları paketlenirken kaynaktan okunur. Devir dosyalarının
  projesinde katmanlar bağsız, kendi satırlarıyla durur: açık dosyada bağ yoktur.
- **Pencere:** devir biçimi işaretliyken özet “Devir dosyaları kaynak tablolardan 7 satırı kapsar: qa_secim 1,
  qa_yollar 3, qa_noktalar 3.” der.
  - Sayım sürerken Klasör seç kapalıdır.
  - Okunamayan kaynakta hata satırı vardır ve Klasör seç kapalıdır.

### 4. Bütün ya da hiç

- Okunamayan bir kaynak devir dosyalarını yazdırmaz: komut `source_unreachable` ile, katmanın adını söyleyerek
  reddeder.
- Veritabanı tablosu kopyalanamazsa da aynı kodla reddedilir.
- Plan geçici klasörde yapılır, hedef klasör hiç açılmaz; olmayan pakette ara dosya ve başarı iletisi kalmaz.
- Devir dosyası istenmeyen pakette yerinde olmayan GeoPackage, öbür bağlar gibi “yerinde yok” diye söylenir.

### 5. Modüller

| İş | Mimari | Harita | Yol | CBS |
|---|---|---|---|---|
| GeoJSON kapsamlarında kaynak satırları | yok: Mimari'de veri katmanı yoktur | var: CBS tabloları altlık olarak eklenebilir | var: aynı gerekçe | var |
| Paketle'de veri katmanlarının kaynağı, devir kapsamı | yok: aynı gerekçe | var | var | var |
| Paketle'de servis katmanları satırı | var | var | var | var |

### 6. Kanallar

| Kanal | Var mı | Gerekçe |
|---|---|---|
| Python | var | Paket `file.package` komutudur: veri katmanlarının kopyası, devir kapsamı ve ret her kanalda aynıdır. Bir veri katmanının satırları GeoJSON'a `cad.layers.export` (`format: geojson`) ile yazılır. |
| MCP | var | Aynı komutlar. |
| Yapay zekâ ajanı | var | Aynı komutlar. |
| Eklenti | yok | Dosya işleri eklentiye kapalıdır. |

## Açık

- **Seçili ve tablo:** veri katmanında Seçili, tablonun seçtiği satırları satırların okunduğu kadarıyla (en çok 1000)
  alır. Tablonun süzgeci (yalnız seçilenleri göster vb.) ayrı bir kapsam değildir.
- **Masaüstü penceresinde GeoJSON:** dosya biçiminde servis (WFS) katmanı yoktur; servisten alınan nesneler çizimin
  nesnesi olduğu için zaten kapsamdadır.
- **Gerçek pencere:** fare ve klavyeyle kabul (`scripts/usage/real/`) bu dilimde yapılmadı.

## Doğrulama

27 Kasım 2026'da koşuldu:

- **Masaüstü** (`shell/exchange/source_export_tests.rs`). Örnek: yalnız veri katmanları olan proje; EPSG:5256'da
  `qa_noktalar` 3, `qa_yollar` 3, `qa_secim` 1 satır.
  - `geojson_writes_the_data_layers_rows_from_their_sources`:
    - kaynak okunurken sayılar 0, Dışa aktar dosya yazmıyor;
    - okununca Seçili 0, Görünen katmanlar 6 (yolların süzgeci üçten ikisini geçiriyor), Tümü 7;
    - RFC 7946 dosyasında 7 nesne, `crs` yok. A ve B'nin boylam, enlemi PROJ'un kendi verdiği değerlere 1e-7
      dereceden yakın (`cs2cs EPSG:5256 EPSG:4326`);
    - proje koordinatlı dosyada 6 nesne, `EPSG::5256` ve kaynağın metreleri birebir.
  - `a_source_that_cannot_be_read_is_named_and_the_rest_is_written`: dosyası taşınmış dördüncü katman adıyla
    söyleniyor, öbür 7 satır yazılıyor.
  - `paketle_carries_the_data_layers_sources_and_their_rows`:
    - pencere satırları 1, 3, 3 sayıyor;
    - paket `veri/qa.gpkg`'yi taşıyor; devir GeoPackage'ında üç tablo 3, 3, 1 satır, `srs_id` 5256;
    - paket başka klasöre taşınıp ilk dosya silinince `Session::open` üç katmanı `veri/qa.gpkg`'de buluyor, satırlar
      3, 3, 1, katmanların ve projenin sistemi 5256.
  - `a_package_whose_source_cannot_be_read_writes_nothing`: devirli plan `source_unreachable` ile, katmanın adıyla
    reddediliyor, hedef klasör açılmıyor; devirsiz paket yapılıyor, eksik dosya söyleniyor.
  - `a_database_layer_is_copied_as_a_geopackage` (geçici PostGIS 18, `KENTOS_DATA_PG`):
    - Kopyala ile tablo `veri/<tablo>.gpkg`'ye 3 satır, 5256 olarak yazılıyor; devir dosyasında da 3 satır var;
    - tablo silinip paket taşınınca katman `gpkg` sürücüsüyle kopyasını okuyor.
  - `-- package_tests gis_tests source_export data_rows data_selection kml_tests`: 42 geçti.
  - `-- module_tests ribbon_tests history::text catalog settings`: 156 geçti. Kırmızı tek sınama bilinen bağımlılık
    tabanıdır (`shell/approved_layer.rs`, `shell/gpu_frame.rs`, `gis/data_selection.rs`; bu dilimden değil).
- **Headless:** `cargo test -p kentos-headless --test headless -- package`: 21 geçti.
- **Resim** (`.run/gui-evidence/kullanim-9338/`, açık ve koyu tema; büyütülerek bakıldı):
  - `01-geojson-kaynak`: Görünen katmanlar (7), üç veri katmanı “kaynaktan”, karışık geometri satırı;
  - `02-paketle-kaynak`: Veri katmanları listesi, satır sayıları ve devir kapsamı.
