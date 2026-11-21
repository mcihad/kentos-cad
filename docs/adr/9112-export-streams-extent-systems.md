# ADR 9112: Dışa aktarma — akışlı yazıcılar, kapsam kutusu, sistem seçici, KML sembolleri

- **Durum:** kabul edildi (2026-11-22). Plan: [plan/veri.md](../../plan/veri.md) VY-15 (ikinci tur).
- **İstek:** sahibin sözü (21 Kasım 2026): “Hepsini düzelt eksikleri tamamla.” Kapatılan kalanlar (ADR 9106 §7):
  - GeoJSON, Shapefile ve KML yazıcıları katmanı bütün alıyor;
  - kapsam yalnız görünüm, serbest kutu yok;
  - sistem EPSG koduyla yazılıyor, seçici yok;
  - KML'ye sembol yazılmıyor.
- **Sözleşmeler:** `.kcad` değişmedi. `cad.layers.export` §4'te KML sembolleri için bir seçenek alır. Biçim
  çekirdeğinde üç akış: `geojson::Stream`, `kml::LayerStream`, `shp::stream::Stream`. **Bağımlılık:** yok.

## §1 Karar: GeoJSON, Shapefile ve KML akışlı yazılır

**Sorun:** dışa aktarma satırları kaynaktan 2048'lik parçalarla okuyordu (ADR 9106 §3). Bu üç biçimde ise bütün
satırları nesne olarak biriktirip biçim çekirdeğinin bütün yazıcısına veriyordu; yazıcı da dosyanın tamamını bellekte
kuruyordu. Bir milyon parsel GeoJSON'a 2,9 GB, KML'ye 3,5 GB, Shapefile'a 3,7 GB bellekle yazılıyordu (§5).

**Masaüstü CBS:** GDAL'ın GeoJSON ve KML sürücüleri detayları geldikçe yazar. Shapefile sürücüsü kayıt kayıt yazar;
genişliği verilmemiş metin sütununu 80 karakter açar, sığmayanı keser. RESIZE=YES seçeneği yazdıktan sonra sütunları en
uzun değere daraltır.

**Karar:**

- **GeoJSON:** `geojson::Stream` başlığı yazar, her nesneyi bir detay olarak geldiğinde yazar, sonda kapatır. Bütün
  yazıcı (`write`, `write_wgs84`) aynı akışı bir `Vec`'e yazar: iki yolun baytları tanım gereği aynıdır.
- **KML:** KML'nin şeması stilleri yer imlerinden önce ister. `kml::LayerStream` belgenin stillerini baştan alır,
  katmanın klasörünü açar ve her yer imini geldiğinde yazar. Tek stilli bir katman için baytlar bütün yazıcınınkiyle
  aynıdır (sınandı).
- **Shapefile:** başlık ve sütunlar bütün satırları ister:
  - şekil türü, herhangi bir satırda kot ya da ölçü varsa Z ya da M olur;
  - nokta takımı, bir satırda birden çok nokta varsa çok noktalıdır;
  - sütunun türü ve genişliği değerlerinden çıkar (ADR 0514).

  Bu yüzden her nesne, bütün yazıcının ayırdığı gibi geometri ailelerine ayrılıp hedefin yanındaki kendi geçici
  dosyasına (`.<ad>_alan.kentos-gecici`) yazılır: parçaları ve öznitelikleri. Başlığı belirleyen şeyler (kapsam, Z ve
  M, özniteliklerin adları) yolda toplanır. Sonda her takımın:
  - .dbf'i o dosyadan iki ya da üç geçişle hesaplanıp yazılır;
  - .shp ve .shx'i tek geçişle yazılır, .shp'nin başlığı en son yazılır.

  Bütün yazıcının sütun hesabı da aynı geçişlere taşındı (`shp/columns.rs`). Aynı nesnelerden iki yol aynı dosyaları
  bayt bayt verir (sınandı: üç takım, kayıt satırı, kot, kurala uymayan tam sayı, ondalık genişliği, fazladan öznitelik).
- **Geçici dosyalar:** `/tmp`'ye değil hedefin yanına yazılır. `/tmp` birçok sistemde bellektedir; büyük bir dışa
  aktarma orada belleği doldururdu.
- **Durdurma ve hata:** GeoJSON ve KML `<dosya>.part`'a yazılır. Shapefile'ın bütün dosyaları `.part` olarak yazılır ve
  hepsi yazılınca yerine konur. Durdurulan ya da başarısız yazmada geride hiçbir şey kalmaz; önceden orada olan dosya
  olduğu gibi kalır.

## §2 Karar: kapsam kutusu

**Masaüstü CBS:** QGIS'te kapsam katmanın ya da haritanın kapsamından hesaplanır, haritada sürüklenerek çizilir ya da
dört değeri (en küçük ve en büyük X, Y) yazılır.

**Karar:** pencerenin “Yazılacak nesneler” bölümünde Kapsam üç seçenek alır: Bütün katman, Görünümdekiler, Kutuya
değenler. Kutu seçilince:

- Dört kenar haritadaki gibi dizilir: üstte Kuzey, yanlarda Batı ve Doğu, altta Güney. Eksenler projenin adlandırdığı
  gibidir: CBS projesinde Y doğu, X kuzey (haritacının); CAD projesinde X ve Y. Türkçe ondalık virgülü de okunur.
- **Görünümden al:** görünümün kutusu.
- **Katmanın kapsamı:** veri katmanında tablonun kutusu, projenin sistemine çevrilmiş; çizim katmanında nesnelerin
  kutusu.
- **Haritada çiz:** pencere kenara çekilir, kutunun iki köşesi tıklanır (kenetler geçerli). Pencere kutuyla geri
  gelir; Esc'te olduğu gibi döner. Eklentilerin harita aracının yakalama aracı kullanılır.
- Dört sayı bir yer çevrelemiyorsa (batı doğudan, güney kuzeyden küçük değilse) pencere bunu yazar, komut çalışmaz.

Kutu komuta `bbox` olarak gider (ADR 9106 §2). Veri katmanında tablonun sistemine çevrilip kaynakta sorulur.

## §3 Karar: koordinat sistemi seçici

- “Koordinat sistemi (EPSG)” metin kutusunun yerine aranabilir bir liste geldi (`system_select.rs`):
  - ilk satır varsayılandır ve adını söyler: “Katmanın sistemi · TUREF / TM36 (EPSG:5256)”, çizim katmanında projenin
    sistemi;
  - varsayılan değilse projenin sistemi gelir;
  - sonra kayıttaki bütün sistemler, Harita sistemi'ndeki gibi datuma göre gruplu.

  Arama adın, datumun ya da EPSG kodunun bir parçasını alır.
- KML'de ve proje koordinatları açılmamış GeoJSON'da liste yoktur: “WGS 84 (EPSG:4326)” yazar ve nedeni söylenir.
- Yeni katman penceresi de aynı listeyi kullanır (ADR 9107 §7); orada da metin kutusu vardı.

## §4 Karar: KML'ye semboller

*(VY-15c'de yazılacak.)*

## §5 Kanıt

**Ölçüm:** `public.perf_parseller` tablosunun bir milyon parseli, hata ayıklama derlemesi. Her yol ayrı süreçte
koşuldu, en yüksek bellek `/usr/bin/time -v` ile ölçüldü (`layer_export_memory_tests`).

| Biçim | Akışlı | Bütün katman yolu | Dosya |
|---|---|---|---|
| GeoJSON | 290 MB | 2 905 MB | 385 MB |
| Shapefile | 287 MB | 3 739 MB | 198 MB |
| KML | 284 MB | 3 455 MB | 581 MB |

- Akışlı yolda bellek dosyanın boyutundan bağımsızdır: üç biçimde ~285 MB, çoğu sınama uygulamasının kendisi.
- Süreler karşılaştırılamaz: bütün katman yolu (eski yolun eşdeğeri) satırları dışa aktarmanın satır hattından
  geçirmeden doğrudan nesne yapar. Akışlı dışa aktarma bu hattan geçer: GeoJSON 68 s, Shapefile 63 s, KML 56 s.

**Sınamalar:**

- `kentos-formats`:
  - KML akışı bütün yazıcının baytlarını verir;
  - Shapefile akışı bütün yazıcının dosyalarını bayt bayt verir, bırakılınca geride dosya kalmaz;
  - GeoJSON, Shapefile ve KML'nin önceki bütün sınamaları geçti (221 + 138 + 63 + 6).
- `cad.layers.export`'un 31 ortak durumu geçti. Bunlar dosyayı okuyup karşılaştırır, Windows-1254 .dbf de dahil.
- Masaüstü `layer_export_tests`:
  - virgüllü yazılan kutu okunur;
  - batısı doğusundan büyük kutu söylenir, hiçbir şey çalışmaz;
  - katmanın kapsamı ve görünüm kutuyu doldurur;
  - haritada çizilen kutu pencereye döner, Esc kutuyu olduğu gibi bırakır;
  - listeden seçilen sistem komuta gider, KML'de gitmez;
  - listenin ilk satırı varsayılandır ve varsayılan sistem ikinci kez listelenmez.

**Gerçek pencere:** bir milyon satırlık yalıtılmış PostGIS tablosu, Parseller'in Farklı kaydet… penceresi.

- Biçim Shapefile seçildi. Sistem listesinde “TM33” arandı: “TUREF (ITRF96) · TUREF / TM33 (EPSG:5255)” ve “ED50 ·
  ED50 / TM33 (EPSG:2321)” geldi; ilki seçildi.
- Kapsam'da Kutuya değenler seçilince Kuzey (X), Batı (Y), Doğu (Y), Güney (X) ve üç düğme göründü.
- Haritada çiz'e basınca pencere çekildi, komut satırı “Dışa aktarma kapsamı: dikdörtgenin bir köşesini gösterin ya da
  Y,X yazın” dedi. İki köşe tıklandı, pencere kutuyla döndü: Batı 587979,497, Güney 4426713,306, Doğu 590103,816,
  Kuzey 4428837,625.
- Dışa aktar: “Shapefile parseller-kutu.shp: 5.184 nesne yazıldı (alan)”. Dosyanın kendisi:
  - .shx'te de .dbf'te de 5 184 kayıt, şekil türü 5 (alan);
  - .shp başlığındaki boy dosyanın gerçek boyuyla aynı;
  - kapsam TM33'te (doğu 844 232–846 463), .prj “TUREF_TM33”.
- Görüntüler `.run/gui-evidence/veri/vy15/sonra`.

## Uygulama

- Biçim çekirdeği: `crates/shared/formats/src/geojson/write.rs` (`Stream`), `kml/stream.rs` (yeni),
  `shp/stream.rs` (yeni), `shp/columns.rs` (yeni), `shp/write.rs` (.dbf'in başı ve kaydı ayrıldı).
- Uygulama: `crates/native/application/src/gis/layer_export_streams.rs` (yeni), `layer_export_sinks.rs` (bütün katman
  hedefi kaldırıldı).
- Masaüstü: `apps/desktop/src/gis/layer_export_extent.rs` (yeni), `system_select.rs` (yeni), `layer_export.rs`,
  `layer_create.rs`, `shell/input.rs` (çizilen kutu), `layer_export_memory_tests.rs` (elle koşulan ölçüm).
