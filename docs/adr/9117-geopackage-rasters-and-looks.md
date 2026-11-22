# ADR 9117: GeoPackage'ın döşemeleri raster, görünüşler tabloya, derecede yakınlaşma

- **Durum:** kabul edildi (2026-11-22); adım VY-20. Plan: [plan/veri.md](../../plan/veri.md) VY-20.
- **İstek:**
  - Sahibin seçimi (22 Kasım 2026): ADR 9116 §5'in listesinden “Raster ve stil yazımı”.
  - Aynı gün sahibin bildirdiği hata: “gpkg dosyalarını açarken çok ciddi ölçek problemleri var, Downloads
    içerisindeki dosyalar alt dizinde de olan gpkg dosyaları zoom yapılamıyor ve ekranda nokta gibi çizimler”.
- **Web:** ertelendi (ADR 0228); gerekenler TODOS.md §0.2'de. **`.kcad`:** biçim değişmedi. **Komut:** yeni komut yok.
  `cad.layers.renderer`'ın kurallarına bir madde eklendi (§4). **Bağımlılık:** yeni dış kitaplık yok.

## §1 Bağlam ve araştırma

**Önce:**

- ADR 9116'ya göre açılan GeoPackage'ın döşeme tabloları katman olmuyordu.
- GeoPackage'ta saklanan projede bağlı rasterin yolu mutlak yazılıyordu; dosya taşınınca raster kayboluyordu.
- Veri katmanının görünüşü yalnız Dışa aktar ile `layer_styles`'a gidiyordu. Kaydet yazmıyordu.
- Derece sisteminde (EPSG:4326) açılan projede yakınlaşma piksel başına 11 metrede duruyordu (§5).

**Masaüstü CBS** (QGIS 3.44 kullanım kılavuzu, 22 Kasım 2026'da okundu):

- *QGIS Configuration*, §5.3.1 General Properties: katmanların yolu projede mutlak ya da “relative to the project file
  location” saklanır. Kılavuz göreliyi önerir: “when both layers and project files can be moved or shared”.
- *General Tools*, §8.5.2.2–8.5.2.3 (ADR 9116 §1): GeoPackage'ta tablonun varsayılan stili `layer_styles`'tadır ve
  katman açılırken o stil yüklenir. Save as Default orayı yazar.

**OGC GeoPackage 1.3.1** (OGC 12-128r18, 22 Kasım 2026'da okundu):

- `gpkg_contents`'in kutusu döşeme tablosunda “informational”dır. Kılavuz bu kutunun varsayılan görünüm olarak
  kullanılabileceğini söyler, ama “there are no requirements that this bounding box be exact”.
- `gpkg_tile_matrix_set`'in kutusu kesindir (Requirement 144): bütün karolar onun içindedir. Karoların koordinatı
  ondan hesaplanır. Kutu karolardan büyük olabilir.

**QGIS 4.2.2'de ölçüldü** (PyQGIS, ekran dışı; betikler oturumun karalama klasöründe):

- QGIS kategorili stilin değerlerini türüyle yazar: tam sayı `type="long" value="1"`, gerçel sayı `type="double"`,
  metin `type="string"`, boş `type="NULL" value="NULL"`.
- Değeri metin (`type="string" value="1"`) olan kategori tam sayı alanının 1 değerli nesnesini alır: QGIS değeri
  yazısıyla arar. Ölçüm §6'da.

## §2 Karar: döşeme tabloları raster olarak açılır

`gpkg_open` projesi olmayan dosyadan proje kurarken (ADR 9116 §2):

- **Her döşeme tablosu** (`gpkg_contents`'te `tiles` ya da `2d-gridded-coverage`) bir katman olur. Katmanın adı
  tablonun adıdır. Döşeme katmanları listenin en altındadır, tabloların altında çizilir.
- **Katmanda bir bağlı raster** durur (`cad.rasters.gpkgread`, `link`): en ayrıntılı düzeyin pikselleri,
  `gpkg_contents`'in kutusu kadar. Kutu yoksa döşeme kümesinin kutusu alınır. Karolar dosyada kalır, görünüm
  gerektikçe okunur.
- **Sistem:** döşeme tablolarının sistemleri de oy verir. Geometrili tablolar önce sayılır, eşitlikte önce gelen
  kazanır. Raster çevrilmeden çizilir, vektör çevrilerek çizilir. Bu yüzden rasterlerin sistemi seçilebilir olmalıdır.
- **İlk görünüm** döşeme tablolarının kutusunu da içerir.
- **Alınamayan döşeme tablosu** söylenir (`tiles_not_taken`). Okuyucunun komut sözleri açılışın sözleriyle değişir:
  - başka sistemde: “karoları EPSG:4326'da, proje EPSG:3857'de; raster bir sistemden öbürüne çevrilmeden çizilmez.”;
  - kutu hiçbir karoyu kapsamıyorsa: “… kutu bir nokta ya da ızgaranın dışında.”;
  - öbür hatalarda okuyucunun kendi iletisi.

  Döşeme kümeleri okunamazsa `tiles_unread` söylenir.
- **Masaüstünün iletisi** rasterleri de sayar: “… dosyanın içine yazıldı (1 veri katmanı, döşeme tablolarından 1
  raster). Kaydet bu dosyaya yazar.”

## §3 Karar: rasterin dosyası GeoPackage'a göre yazılır

- GeoPackage'ta saklanan projede bağlı rasterin ve resmin dosyası, GeoPackage'ın klasörüne göre yazılır
  (`source_places::with_relative_files`, `project_store::with_relative` içinden). Dosyanın kendi rasteri yalnız
  dosyanın adıyla yazılır.
- Okunurken yol dosyanın şimdi durduğu yere göre mutlak olur (`project_store::decode`, `with_absolute_files`).
  Masaüstü (Veri kaynağından aç, GeoPackage açılışı) ve headless (Python, MCP) aynı işlevi kullanır.
- Dosya rasterleriyle birlikte taşınır ya da kopyalanır; açılınca raster bulunur.

## §4 Karar: görünüş tablonun varsayılan stili olarak yazılır

`gpkg_looks::write(doc, file)`, GeoPackage'a kaydedilince (masaüstünde Kaydet, headless'ta yolsuz `save()` ve
`save_to`):

- **Hangi katmanlar:** o dosyanın tablosundan çizilen, geometrili veri katmanları. Çizimin varsayılan görünüşündeki
  (işleyicisi yok, rengi temanın) katman yazılmaz.
- **Ne yazılır:** katmanın görünüşü tablonun yeni varsayılan stilidir. QML (kategorili ve dereceli işleyici olduğu gibi
  kalır) ve yanında SLD yazılır. Açıklaması `KentOS`'tur. Eski satırlar tabloda kalır, artık varsayılan değildir. Hepsi
  tek işlemde yazılır.
- **Değişmeyen yazılmaz:**
  - tablonun varsayılanı KentOS'un daha önce yazdığı QML'in aynı metniyse;
  - ya da başka programın stili, açılıştaki gibi okununca (ADR 9116 §2) bu görünüşü veriyorsa.

  Böylece her kayıt yeni satır eklemez.
- **Masaüstü:** yazım arayüzün iş parçacığında değil, kaydın ardından yürür. Yazılınca şu söylenir: “Görünüşler
  GeoPackage'ın stil tablosuna varsayılan stil olarak yazıldı: binalar. Dosyayı açan başka bir CBS de tabloları böyle
  çizer; eski stiller tabloda durur.”
  - Yazılamayan katman uyarıdır.
  - Yazım bütünüyle başarısızsa: “Proje kaydedildi ama katmanların görünüşü … yazılamadı: …”.
  - Kaydın kendi iletisi ve günlüğün iletisi önce söylenir.
- **Headless:** kayıt cevabında `looks` (yazılan tablolar), `looksLeft` ve `looksUnwritten` vardır. Python'da
  `StoredSaved.looks`, MCP'de `drawing.save`'in cevabı.
- **Kategorinin değeri metindir:**
  - `cad.layers.renderer` sayı, mantık, liste ya da nesne olan kategori değerini reddeder: ““categories[1].value” bir
    metin olmalı: nesnenin değeri yazısıyla karşılaştırılır; 2 yerine "2" yazın.” `null` boş değerin sınıfıdır.
  - Önce sayı kabul ediliyor ama sessizce boş değer oluyordu (`model.rs`), hiçbir nesneyi almıyordu. QML'e de
    `value=""` gidiyordu. KentOS değeri yazısıyla karşılaştırır (`resolve.rs`). QGIS de yazısıyla arar (§1).
  - Ortak durum: `cad.layers.renderer.json`'da “bozuk işleyici: kategorinin değeri sayı” (46 durum).

## §5 Karar: derece projesinde yakınlaşma, metre projesindeki kadar

**Hata:**

- Çizim alanının kamerası (`kentos-render-wgpu`) yakınlaşmayı dünya birimi başına 10 000 pikselde durduruyordu
  (`MAX_SCALE`). Bu sayının belgesi “metre başına”dır: piksel başına 0,1 mm.
- Derece projesinde dünya birimi derecedir. Sınır piksel başına 1/10 000 derece, yaklaşık 11 metre oluyordu.
- Sahibin indirdiği Boston kesitleri (WGS 84) bu yüzden açıldı ama yakınlaşmadı. 270 metrelik bir teleferik hattı 24
  piksel kaldı, “nokta gibi” göründü.
- `fit` de aynı sınırla kesiliyordu. İlk görünüm, Tümünü göster ve Pencere yakınlaştır da küçük kalıyordu.

**Karar:**

- Kameranın sınırları `MIN_SCALE` ve `MAX_SCALE` çarpı `Camera::metres_per_unit`'tir (`scale_limits`,
  `clamp_scale`). Değer `zoom_at`'te, `fit`'te ve masaüstünde kayıtlı görünüme dönüşte kullanılır.
- Masaüstü değeri çizim açılırken, sığdırmadan önce verir (`viewport::opened`). Belgenin her değişiminden sonra da
  verir (`follow_map`); proje dereceye geçerse ya da dereceden çıkarsa sınır onu izler.
- Değer, coğrafi projede sisteminin elipsoidinin ekvatorunda bir derecedir: a·π/180 (WGS 84'te 111 319,49 m).
  Öbür projelerde 1'dir (`viewport::unit_metres`).
- Bu bir sınırdır, ölçü değildir. Durum çubuğunun ölçeği derecenin görünümün ortasındaki uzunluğunu alır (GIS-040,
  `map_marks::metres_per_unit`).

## §6 Kanıt

- **Bağımsız durumlar:** `scripts/fixtures/gpkg_open_cases.py` 13 GeoPackage yazar. Yeni üçü:
  - `ortofotolu.gpkg`: parseller ve iki düzeyli PNG döşeme tablosu;
  - `yalniz-ortofoto.gpkg`: yalnız döşeme tablosu;
  - `derece.gpkg`: WGS 84'te bir mahalle (270 m'lik hat, 20 m'lik binalar).

  PNG karoları betiğin kendi yazıcısıyla yazılır. Beklenenler ADR'nin kurallarından çıkarılır: raster katmanı en
  altta, sistem oyu, döşemeleri içeren ilk görünüm.
- **Native** (`tests/all/gpkg_open.rs`):
  - her durum beklendiği gibi açılır;
  - rasterin dosyası saklanan projede “ortofotolu.gpkg” yazılır, taşınan dosyadan açılınca yeni yerindedir;
  - değişen görünüş 3 satırın yanına 4. satır olarak varsayılan yazılır, değişmeyen yazılmaz;
  - bütün komut durumları geçer, yeni ret durumu dahil.
- **Kamera:** `camera::tests`'te derece kamerası 0,002° × 0,0024°'lik mahalleyi alana sığdırır (504 piksel) ve metre
  kamerasının derinliğine iner. Metre kamerası 10 000'de kalır.
- **Masaüstü** (`gpkg_open_tests`):
  - döşemeli dosya rasteriyle açılır, ileti rasteri sayar, taşınan dosyada raster bulunur;
  - Kaydet değişen görünüşü yazar, ikinci Kaydet yazmaz;
  - derece dosyası alanı doldurur (704 piksel), Yakınlaştır bir adım daha (1,5 kat) iner.
- **Python ve MCP:** `test_gpkg_open` (raster, `looks`, sayı değerin reddi) ve MCP `gpkg_open` (raster katmanı,
  `drawing.save`'in `looks`'u, ikinci kayıtta boş).
- **QGIS 4.2.2 ile karşılaştırma** (PyQGIS, ekran dışı):
  - KentOS'un yazdığı tek renkli görünüş QGIS'te `singleSymbol` `#2a6fdb` açıldı. Eski iki satır durdu, yeni varsayılan
    “binalar”, açıklaması KentOS. İkinci kayıt hiçbir şey yazmadı.
  - Kategorili görünüş `categorizedSymbol` açıldı: alan `kat`, değerler `"1"`, `"2"` ve Diğerleri. QGIS'in çizdiği:
    kat 1 `#fdae61`, kat 2 `#d7191c`, kat 3 ve 4 `#abd9e9`.
  - Gerçek pencerede KentOS projesini yazdıktan sonra dosyayı QGIS okudu: `parseller` 2 nesne, `ortofoto` 512 × 512
    raster, EPSG:5256.
- **Gerçek pencere** (`.run/gui-evidence/gpkg-raster-9117`):
  - `ortofotolu.gpkg`'nin bir kopyası program açılırken verildi. Raster parsellerin altında çizildi, dört karo dikişsiz
    birleşti. İleti “(1 veri katmanı, döşeme tablolarından 1 raster)” dedi.
  - Görünüşün pencereden değiştirilip kaydedilmesi gerçek pencerede yapılmadı: sahip o sırada bilgisayarı kullanıyordu
    ve onun ekranına fare ve klavye olayı gönderilmedi. Bu adım masaüstü, Python ve MCP sınamalarıyla doğrulandı;
    pencerede yapılması §7'de.
- **Sahibin dosyaları** (kopyaları; ekran dışı resimler `.run/gui-evidence/gpkg-*-9117`, sınamanın `KENTOS_GPKG`'si):
  - NextGIS Boston kesitlerinden `aerialway-line.gpkg` alanı doldurdu (50 m'lik ölçek çubuğu, 1:1 334).
  - `highway-line.gpkg` (15 MB) bütün kenti 1:92 538'de, üç adım yakında 1:27 419'da gösterdi. On bir adım yakında
    1:1 070'te şeritler ve kavşaklar ayrı göründü. Önceki sınır orada yaklaşık 1:31 000'di.
  - NGA'nın örnek GeoPackage'ı (projesi silinmiş kopya): 17 döşeme tablosunun 13'ü raster olarak geldi. Dördü söylendi:
    üçü EPSG:4326'da (proje EPSG:3857), biri noktadan ibaret kutulu.

## §7 Kalan

- **Noktadan ibaret kutu:** `gpkg_contents`'in kutusu karoları kapsamıyorsa (NGA'nın `point2_tiles`'ı), en ayrıntılı
  düzeyde dosyada duran karoların kutusu alınmalı.
- **Başka sistemdeki döşeme tablosu** rasterin çevrilmesini bekler.
- **Gerçek pencerede görünüş:** Katman stili'nden değiştirip Kaydet'le yazmak, bilgisayar boşken.
- **Web** (TODOS.md §0.2): web kamerasının sınırı (`apps/web/src/viewport/Camera.ts`, `MAX_SCALE = 1e4`) de dünya
  birimi başınadır ve derece projesinde aynı hatayı verir. Kategori değerinin kuralı WASM'dan gelir; yeni ortak durum
  web'in WASM'ı yenilenince geçer.
- ADR 9116 §5'in öbürleri sürüyor: ilişkiler, üst veri, birden çok geometri sütunu, açılışta tablo seçimi, bakım.
