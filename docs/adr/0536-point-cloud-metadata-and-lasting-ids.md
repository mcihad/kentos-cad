# ADR 0536: Nokta bulutunun kaynak bilgisi ve noktanın kalıcı kimliği

- **Durum:** kabul edildi (2026-10-20). `plan/gis.md` CBS-32'nin ilk dilimi: nokta bulutu. Kapsamı ben belirledim.
- **Bağlam belgesi:** ADR 0207 (nokta bulutu: LAS, LAZ, COPC, metin bulutları, dizin, XYZ sor), 0243 (mesh ve çok
  boyutlu veri), 0350 (bağlı dosyaların aranması, `links::find`).
- **Web:** ertelendi (ADR 0228); nokta bulutu web'de zaten yok (ADR 0207). Web'de gerekenler TODOS.md §0.2'de.
- **Sonraya kalan:**
  - mesh ve NetCDF'in boyut ve zaman bilgisi aynı sözleşmede (`dataset`'in komutu);
  - adresteki (HTTP) bulutun başsız sunucuda okunması (bugün masaüstünün penceresi okur, komut söyler);
  - metin bulutunun sorgusu (sütun planı her seferinde taranır);
  - masaüstünün XYZ sor aracının kalıcı kimliği göstermesi: araç kendi dizinleriyle (COPC olmayan dosyada cihazın
    önbelleğindeki COPC) aradığı için düğüm sırası kaynağın sırası değildir; araç yerini, komut kimliğini verir;
  - AOI, profil ve istatistik sorgularının sağlayıcıya taşınması (CBS-02, CBS-42), çoklu VPC özeti, NetCDF-4, HDF5,
    GRIB ve Zarr bağdaştırıcıları.

## Bağlam

Nokta bulutunun yatay sistemi ve sayısı biliniyordu (ADR 0207). Kaynak dosyanın başka bilgisi ise görünmüyordu: düşey
sistem, çekim günü, noktaların GPS zamanının türü, ek boyutlar, dönüşe göre sayılar. Kotları başka bir düşey dayanakla
karıştırmak, ya da GPS haftasının saniyesini mutlak zaman sanmak sessiz bir hatadır. Plan ayrıca nokta seçiminin kararlı
bir kaynak kimliğine dönmesini, örneklenmiş görüntünün tam bulut istatistiği sayılmamasını ister. Python ve yapay zekâ
ajanı bulutu bugüne dek okuyamıyordu: başsız sunucunun dosya erişimi nokta bulutu araçlarını reddediyordu.

## Karar

1. **`cad.pointclouds.describe` (salt okunur).** Her kaynağın başlığını uygulama katmanında, nokta bulutu çekirdeğiyle
   okur. Dosya projenin öbür bağları gibi aranır (kayıtlı yol, çizimin klasörü, arama yolları); gömülü bulut
   kitaplıktan okunur. Söylenenler:
   - tür ve sürüm, nokta biçimi ve kaydın boyu, sayı, kutu, ölçek ve öteleme;
   - yatay sistem (EPSG, dosyanın yazdığı ad) ve düşey sistem: WKT'nin `VERT_CS`/`VERTCRS` adı ya da GeoTIFF anahtarı
     4096'nın `EPSG:n`'si; yazmayan dosyada yoktur ve arayüz “Dosya söylemiyor” der;
   - başlığın oluşturulma günü, GPS zamanının türü (`standard`, `week`), sistem, yazılım, kaynak kimliği, GUID;
   - LAS 1.4 R15'in tablosundan boyutlar ve ek baytlar türleriyle; dönüşe göre sayılar; COPC'nin düğümleri;
   - `classes` ile her noktanın sınıfı sayılır: dosya bütün okunur, örneklenmez; bir çağrıda en çok 200 milyon nokta
     (`too_many_cloud_points`).
   - Okunamayan kaynak `why` ile söylenir, `cloud_source_unreadable` uyarısıyla; öbürleri okunur.
2. **`cad.pointclouds.query` (salt okunur).** Bir yerin planda erim içindeki en yakın noktasını kaynağın tam
   çözünürlüğünde okur.
   - COPC'de yalnız arama karesine değen düğümler, LAS ve LAZ'da bütün dosya okunur.
   - Noktanın kalıcı kimliği bulutun kalıcı kimliği, kaynağın sırası ve noktanın kaynağın sırasındaki yeridir (LAS,
     LAZ: dosyanın sırası; COPC: düğümler anahtarlarıyla, her düğümün noktaları sırayla). Aynı dosya her okunuşta aynı
     kimliği verir.
   - Eşit uzaklıkta önce bakılan kalır. Bakılan nokta sayısı (`read`) söylenir.
   - Erimi sıfırdan büyük olmayan arama reddedilir (`invalid_reach`). Metin bulutu ve okunamayan kaynak uyarıyla atlanır.
3. **Arayüz.** Öznitelikler'de tek kaynaklı bulutun satırlarına dosya, düşey sistem, oluşturulma, GPS zamanı, ek
   boyutlar ve yazan eklenir; komutun okuyucusundan, dosya başına oturumda bir kez. XYZ sor'un manifestteki karşılığı
   `cad.pointclouds.query`'dir (düzey `query`).
4. **Python ve MCP** iki komutu katalogdan alır. Böylece başsız sunucu ilk kez bir bulutu okur.

## Doğrulama

- `uv run --no-project --with 'laspy[laszip]==2.7.0' --with pyproj python3 scripts/fixtures/pointcloud_describe_cases.py --check`:
  - `cad.pointclouds.describe`'ın 31, `cad.pointclouds.query`'nin 25 durumu, KentOS kodu olmadan yazılmış başvurudan;
  - başvurunun kaynakları: laspy'nin başlığı ve noktaları, başlık baytlarından GUID, pyproj'la WKT'nin yatay ve düşey
    parçaları, GeoTIFF anahtarları sayılarıyla, LAS 1.4 R15 tabloları, COPC 1.0 belirtiminden düğümler ve küpleri;
  - on üç dosya: laspy'nin ve LASzip'in yazdıkları ve `--files`'ın yazdığı `meta-wkt.las` ile `meta-geokeys.las`;
  - ayrıca metin bulutu, dosyasız, adresteki, bozuk ve gömülü kaynak, iki kaynaklı ve gizli katmandaki bulut, retler.
  - `KENTOS_CASES_ONLY=cad.pointclouds cargo test -p kentos-native-application --test all every_command_case`.
- `.run/py/bin/python -m unittest python/tests/test_command_cases.py`: aynı durumlar Python'un tipli katmanından.
- Resimler: `KENTOS_SHOTS_ONLY=bulut-bilgi cargo test -p kentos-desktop tools_screens -- --ignored --nocapture`
  (`.run/shots/arac-bulut-bilgi-*`).
