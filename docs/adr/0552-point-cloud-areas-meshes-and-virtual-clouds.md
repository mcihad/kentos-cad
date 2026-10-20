# ADR 0552: Nokta bulutunun alan ve profil sorguları, meshin bilgisi ve sorgusu, sanal bulutun özeti

- **Durum:** kabul edildi (2026-10-20). `plan/gis.md` CBS-32'nin kalanı: nokta bulutu ve mesh üstbilgisi sağlayıcı
  sözleşmesinde, eksik biçimler. Kapsamı koordinatör verdi, ayrıntıyı ben belirledim.
- **Bağlam belgesi:** ADR 0536 (nokta bulutunun kaynak bilgisi ve kalıcı noktası), 0207 (nokta bulutu), 0243 (mesh ve çok
  boyutlu veri), 0350 (bağlı dosyaların aranması).
- **Web:** ertelendi (ADR 0228); nokta bulutu web'de zaten yok, mesh'in web yolu da durur. Web'de gerekenler TODOS.md §0.2'de.
- **Sonraya kalan:**
  - NetCDF-4, HDF5, GRIB ve Zarr okuyucuları (aşağıda §5);
  - adresteki (HTTP) bulutun ve NetCDF'in başsız sunucuda okunması (pencere ve komut söyler, bugün dosyalar okunur);
  - metin bulutunun alan ve profil sorgusu (sütun planı her seferinde taranır; uyarıyla atlanır);
  - mesh'in alan ve çizgi sorgusu (profil, bölgesel istatistik) ve dönen yüzün komşuları.

## Bağlam

ADR 0536 bulutun başlığını ve en yakın noktayı kalıcı kimliğiyle verdi. Plan, sağlayıcıya iki şey daha ister: sorguların
çizimin ayrıntı düzeyinden değil kaynağın tam çözünürlüğünden cevaplanması (alan istatistiği, çizgi boyunca profil) ve
mesh'in aynı sözleşmeye girmesi (düğüm, yüz, zaman, değer). Örneklenmiş görüntüden alan sayımı yapmak ya da bir meshin
değerini ekranın pikselinden okumak sessiz bir hatadır; çok dosyalı bir bulutta dosyaların sistemi ya da biçimi
tutmuyorsa bunun çizimde değil sorguda söylenmesi gerekir. Python ve yapay zekâ ajanı bunların hiçbirini bugüne dek
okuyamıyordu.

## Karar

Beş yeni ürün komutu değil dört; beşincisi mevcut komutun cevabına eklendi. Hepsi salt okunurdur (`query`), çizimi
değiştirmez, `undo` adımı yoktur.

1. **`cad.pointclouds.region`: alanın istatistiği.**
   - Girdi: planda bir halka (`ring`, en az üç köşe, kapalı sayılır; son köşe birincinin aynısıysa atılır) ve delikler
     (`holes`); bulutlar `uids` ile, verilmezse görünenler.
   - Alanın içi: sınır dahil kapalı halka, eksi deliklerin kesin içi. Karar halkanın kenarlarına göre doğuya ışının
     kesişme sayısıdır, kenarın işareti kesin (`orient2d`); kenardaki nokta içeridedir.
   - Cevap bulut başına ve toplam: nokta sayısı, içerdeki noktaların kutusu, yüksekliğin ve yoğunluğun (intensity) en
     küçüğü, en büyüğü ve ortalaması (toplam çift-float ile kesin, bir kez yuvarlanır), sınıf başına sayı; alanın büyüklüğü
     (halka eksi delikler, ilk köşeye göre kaydırılmış ayakkabı bağı; halka basit sayılır) ve alan başına nokta.
   - Okunan nokta (`read`): COPC'de yalnız küpü halkanın kutusuna değen düğümler, LAS ve LAZ'da kutusu kutuya değen dosya
     bütün. Bir çağrıda en çok 200 milyon nokta (`too_many_cloud_points`); kayıtlı sayısı bunu aşan bir dosya hiç
     okunmadan reddedilir.
   - Metin bulutu ve okunamayan kaynak uyarıyla atlanır (`cloud_source_unreadable`, ADR 0536'nın kodu).
   - Retler: `invalid_cloud_area` (üç köşeden az, sonlu olmayan köşe, alanı sıfır olan halka ya da deliklerin kapladığı
     halka).
2. **`cad.pointclouds.profile`: yol boyunca profil.**
   - Girdi: `path` (en az iki köşe, art arda aynı köşe yok), `width` (koridorun toplam genişliği), `step` (istasyon
     adımı), `points` (noktalar da listelensin).
   - Koridor: yola uzaklığı en çok `width / 2` olan noktalar; uzaklık en yakın doğru parçasınındır (uçlar yuvarlak, kare
     uzaklık kesin karşılaştırılır). Noktanın istasyonu o parçanın başlangıcı artı ayağının parçadaki yeridir (eşit
     uzaklıkta önceki parça), ofseti yolun solunda artıdır.
   - İstasyonlar `[from, to)` aralıklıdır, boş olanlar da cevaptadır; son istasyon yolun ucunu içerir. En çok 100 000
     istasyon (`too_many_stations`). Her istasyonda sayı ve yüksekliğin en küçüğü, en büyüğü, ortalaması.
   - `points` ile noktalar kalıcı kimlikleriyle (bulut, kaynak, yer; ADR 0536) istasyona göre sıralı gelir; en çok 100 000
     (`too_many_listed_points`).
   - COPC'de bir düğüm koridorun herhangi bir parçasının genişletilmiş kutusuna değiyorsa okunur.
   - Retler: `invalid_corridor`.
3. **`cad.meshes.describe`: meshin bilgisi.** Mesh, bir rasterin gösterdiği NetCDF dosyasındaki UGRID ağıdır (2DM ve DAT
   Mesh ekle'de böyle bir dosyaya çevrilir, ADR 0243). Başlık ve ağ dosyadan okunur: düğüm, yüz, kenar (her yüzün kenarları
   birer kez) ve üçgen sayısı, kutusu, yüzlerin alanı, kenarların ortalaması, sistemi (EPSG, coğrafi mi) ve veri setleri: ad,
   vektörün y bileşeni, uzun ad, birim, düğümde ya da yüzde, boyutları. Zaman boyutu (CF zamanı) adım sayısı, ilk ve son
   an (1970'ten milisaniye) ve zaman sürgüsünün yazışıyla söylenir. Rasterin gösterdiği veri seti ve adımları (`shown`)
   da cevaptadır. Okunamayan dosya `why` ile söylenir (`mesh_source_unreadable`); `bad-hdf5.nc` ve `bad-grib.nc` bu yolla
   okunmadıklarını söyler. Ret: `not_a_mesh`.
4. **`cad.meshes.query`: yüz ve değerler.**
   - Yüz kesin bulunur: kapalı alanı yeri tutan üçgenlerin en küçük sıralısı (ortak kenar ve düğümde en küçük yüz). Kalıcı
     kimliği meshin kalıcı kimliği, dosyadaki ağın adı ve yüzün dosyadaki sırasıdır; düğümleri (dosyanın sırasıyla) ve
     alanı da verilir.
   - Değer: düğümdeki veri setinde üçgende doğrusal ara değer (bir düğümün değeri yoksa yok), yüzdeki veri setinde yüzün
     değeri, vektörde büyüklüğü; yüz kapalıysa (`kentos_mask` 0) ya da değer yoksa yok. Hepsi rasterin kendi sayı kuralıyla
     (`Mesh::value_by`, ölçek ve öteleme, doldurma değeri).
   - An: `time` (1970'ten milisaniye) verilirse her veri setinin o andan önceki son adımı, ilk adımdan önceyse değer yok;
     verilmezse rasterin gösterdiği adımın anı bütün veri setlerine uygulanır. Zaman boyutu olmayan veri seti dilimini
     olduğu gibi okur; gösterilen veri setinin öbür boyutları rasterin dilimidir, diğerlerinin ilk değeridir.
     `series` ile her adımın değeri de gelir.
   - Mesh'ler `uids` sırasıyla (yoksa görünenler) bakılır; yeri tutan ilki cevaplar, okunamayan uyarılır ve geçilir.
     `variables` adlı veri setleriyle sınırlar; yüzü bulan mesh'te olmayan ad `mesh_variable_not_found`.
   - Yalnız gereken değerler dosyadan okunur: bir noktanın üç düğümü (ya da yüzü), vektörün öbür bileşeni ve maske;
     yakın değerler tek okumada birleşir (`Cube::series_of`, `Steps`). Okunan değer sayısı `read`'dedir; bir çağrıda en çok
     2 milyon değer (`too_many_mesh_values`).
5. **Sanal bulutun özeti: `cad.pointclouds.describe`'ın `summary`'si.** İki ya da daha çok kaynaklı bulutun cevabına
   eklenir (tek dosyalıda yoktur). Kaynak sayısı ve okunanlar, noktaların toplamı (okunan dosyada başlığınki, okunmayanda
   kaydınki), kutuların birleşimi, dosyaların ortak yatay sistemi ve uyuşmazlıklar, sırasıyla: `epsg`, `vertical`,
   `point_format`, `gps_time`, `scale`, `extra` (başlığı okunan dosyaların sistemi, biçimi, GPS zamanı, ölçeği ya da ek
   boyutları aynı değil), `count` ve `bounds` (bulutun kaydı başlıktan farklı), `overlap` (iki dosyanın kutusu pozitif
   alanla örtüşüyor, en çok 50 çift). Metin kaynağının başlığı yoktur: toplamlara girer, karşılaştırmaya girmez.
6. **Biçimler.** NetCDF-4 (HDF5 kabı), HDF5, GRIB ve Zarr okunmaz: ne HDF5'in B-ağaçları ve kümeleri ne GRIB'in kodlamaları
   ne de Zarr'ın parçalı depoları çalışma alanındaki bir bileşenle okunabilir, hiçbiri küçük bir bağımlılık da değildir.
   CLAUDE.md §3 yeni çalışma zamanı bağımlılığı için sahibin onayını ister; onay yoktur. Bugün `Cube::parse` bu dosyaları
   başlığından tanır ve nedenini söyler (`bad-hdf5.nc`, `bad-grib.nc` durumları); `cad.meshes.describe` o iletiyi `why`
   olarak verir. Yazılacaksa kendi okuyucumuz ADR 0243'ün kuralıyla (dosyanın gereken parçaları, kendi crate'i) gelir.
7. **Arayüz.** Üç pencere (CAD › Ekle › Bulut ve mesh, CBS › Analiz › Analiz): Nokta bulutu alan istatistiği (seçili alan ya
   da kapalı çoklu çizgi, delikleriyle), Nokta bulutu profili (seçili çizgi ya da çoklu çizgi, genişlik, adım, noktaları
   listele) ve Mesh sor (X, Y, isteğe bağlı an, her adımı göster); dosyalar pencerenin iş parçacığında okunur, Çalıştır
   komutun planını çağırır, Panoya kopyala cevabı metin olarak alır. Öznitelikler'de çok dosyalı bulutun satırlarına
   Dosyalar ve Uyuşmazlık, mesh rasterinin satırlarına Kenar ve üçgen, Yüzlerin alanı, Mesh sistemi, Veri setleri ve Zaman
   adımları eklenir; ikisi de komutun okuyucusundan, dosya başına oturumda bir kez.
8. **Python ve MCP** komutları katalogdan alır (`kentos.cad.pointclouds.region`, `.profile`, `kentos.cad.meshes.describe`,
   `.query`); hepsi salt okunur olduğundan MCP'nin yıkıcı listesine girmez.

## Doğrulama

- Bağımsız başvurular, KentOS kodu olmadan:
  - `uv run --no-project --with 'laspy[laszip]==2.7.0' --with pyproj python3 scripts/fixtures/pointcloud_describe_cases.py --check`:
    `cad.pointclouds.describe`'ın 42 (11'i özet), `cad.pointclouds.query`'nin 25 durumu; yeni dosyalar `tile-a` … `tile-f.las`
    (özetin uyuşmazlıkları) ve `area-grid.las` (41 × 41 kafes, her koordinat ikilik tabanda tam) `--files` ile yazılır;
  - `uv run --no-project --with 'laspy[laszip]==2.7.0' --with pyproj --with mpmath python3 scripts/fixtures/pointcloud_area_cases.py --check`:
    `cad.pointclouds.region`'ın 40, `cad.pointclouds.profile`'ın 40 durumu: noktalar laspy'den, alanın içi ve koridorun
    uzaklığı kesirlerle (kenar işaretleri tam), istasyon ve ofset mpmath'le 40 basamakta, ortalamalar `math.fsum`'la,
    COPC'de okunan düğümler COPC 1.0 belirtiminden; kafes sınırdaki noktaları, deliğin kenarını, kafesten geçen eğik kenarı,
    istasyon sınırındaki ve yarı genişlikteki noktaları kesin sınar;
  - `uv run --no-project --with mpmath --with numpy python3 scripts/fixtures/mesh_cases.py --check`: `cad.meshes.describe`'ın 24,
    `cad.meshes.query`'nin 40 durumu: dosyalar `netcdf_classic.py`'nin yazdığı klasik NetCDF'ler (`ugrid.nc`, 2DM ve DAT'tan dönüşen
    `sms-start.nc`, yeni `quad-static.nc` ve `layered.nc`), ağın okunuşu UGRID 1.0 kurallarından, yüz ve değerler
    `multidim_cases.py`'nin kesirli aramasından (QGIS MDAL ile çapraz denetimi o betikte).
  - `KENTOS_CASES_ONLY=cad.pointclouds,cad.meshes cargo test -p kentos-native-application --test all every_command_case`.
  - `too_many_mesh_values` ortak durumla kurulamaz (333 bin adımlı zaman ekseni megabaytlık dosya ister); `meshes.rs`'in
    birim testinde.
- `.run/py/bin/python -m unittest python/tests/test_command_cases.py`: aynı durumlar Python'un tipli katmanından.
- Masaüstü: `cargo test -p kentos-desktop area_tests`; resimler
  `cargo test -p kentos-desktop area_tests::screens -- --ignored --nocapture` (`.run/shots/bulut-alan-*`, `bulut-profil-*`,
  `mesh-sor-*`, `mesh-oznitelik-*`, `bulut-ozet-*`; 1440×900 ve 1100×650, iki tema).
