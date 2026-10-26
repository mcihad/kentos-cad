# ADR 0645: NetCDF-4, HDF5, GRIB ve Zarr; adresteki dosya başsız; metin bulutunda alan ve profil; meshte alan ve hat

- **Durum:** kabul edildi (2026-10-26). `plan/gis.md` CBS-32'nin kalanı. Kapsamı ben belirledim.
- **Kalan işler (ADR 0536 ve 0552'den):**
  - NetCDF-4, HDF5, GRIB ve Zarr okuyucuları;
  - adresteki bulutun, rasterin ve NetCDF'in başsız (Python, MCP) okunması;
  - metin bulutunda alan ve profil;
  - meshte alan ve hat sorgusu.
- **Bağlam belgesi:**
  - ADR 0243 (klasik NetCDF, CF ızgaraları, UGRID, `Cube`);
  - ADR 0207 (nokta bulutu, metin bulutları);
  - ADR 0536 ve 0552 (bulutun ve meshin komutları);
  - ADR 0572 ve 0573 (başsız sunucunun dosyaları);
  - ADR 0642 ve 0644 (adresteki dosya, imza, SAS).
- **Web:** ertelendi (ADR 0228). Web'de gerekenler TODOS.md §0.2'de.
- **`.kcad`:** şema değişmez. Raster ve mesh dosyanın yolunu (Zarr'da klasörün yolunu) ya da adresini önceki gibi tutar.
- **Bağımlılık:** yeni üçüncü taraf kitaplık yok. HDF5, GRIB, Zarr, Blosc ve CCSDS (AEC) okuyucuları KentOS'un kendi
  kodudur. Başvurular netCDF4-python, h5py, zarr-python, numcodecs ve ecCodes'u yalnız sınamada (`uv run --with`) kullanır.
- **Sonraya kalan:**
  - GRIB2'nin JPEG 2000 paketlemesi (5.40). Reddedilir, `grib_set -r -s packingType=grid_ccsds` ile çevirme yolu söylenir.
  - GRIB'in enlem-boylamdan başka ızgaraları (Lambert, kutup, Gauss) ve GRIB1'in karmaşık paketlemesi.
  - HDF5'in szip, LZF, bitshuffle ve n-bit süzgeçleri; HDF5'in bileşik ve değişken uzunluklu sayı türleri (değişken uzunluklu
    metin okunur).
  - Zarr v3'ün `transpose`'u yalnız tam ters sırayla; başka permütasyon reddedilir.
  - Adresteki Zarr deposu yalnız birleşik (consolidated) üst bilgiyle açılır; adres listelenmez.
  - Python modülünün ağı S3 imzası ve Planetary Computer SAS'ı atmaz (masaüstü ve MCP atar, ADR 0644 §1).
  - Mesh alanında yüz, merkezine göre alana girer ya da girmez; yüzün alanla kesişen parçası ağırlık olarak sayılmaz.

## Bağlam

ADR 0552 meshin bilgisini ve sorgusunu, bulutun alanını ve profilini getirdi. Dört şey eksik kaldı.

- **Biçimler.** KentOS klasik NetCDF'i (CDF-1, 2, 5) okuyordu. Bugün çok boyutlu verinin büyük kısmı NetCDF-4 (HDF5
  üstünde), GRIB (hava tahmini) ve Zarr (bulut deposu) olarak gelir. ADR 0243 bunları nedeniyle reddediyor, `nccopy` ile
  çevirmeyi öneriyordu.
- **Başsız adres.** Masaüstü adresteki COG'u, COPC'yi aralıklarla okuyordu (ADR 0207 §1). Python ve MCP'nin dosya yolu
  (`LocalFiles`, `source_bytes`) adresi “yalnız masaüstünde okunur” diye reddediyordu.
- **Metin bulutu.** `cad.pointclouds.region`, `.profile` ve `.query` metin bulutunun (XYZ, PTS, TXT, CSV) noktalarına
  bakmıyor, `cloud_source_unreadable` diyordu.
- **Meshte alan ve hat.** `cad.meshes.query` tek bir yeri soruyordu. Alanın yüzlerinin istatistiği ve hat boyunca değerler
  yoktu. Bulunan yüzün komşuları söylenmiyordu.

## Karar

### §1 Parçalı biçimler sanal klasik NetCDF'tir

NetCDF-4, HDF5, GRIB ve Zarr okuyucuları dosyayı bir **model**e çevirir: boyutlar, global öznitelikler, değişkenler (adı,
boyutları, türü, öznitelikleri, değerlerinin nerede ve nasıl durduğu).

`VirtualFile` modelden bir **CDF-5 dosyası** kurar:

- başlık `write::file_layout_version(…, 5)` ile yazılır;
- her değişkenin değerleri büyük uçlu, art arda durur;
- global `_KentOSFormat` özniteliği kaynağın biçimini söyler (`NetCDF-4 (HDF5)`, `HDF5`, `GRIB2`, `GRIB1`, `Zarr v2`,
  `Zarr v3`); `CubeInfo.format` onu okur ve pencere “NetCDF 5” yerine onu yazar.

`read(offset, len)` istenen baytların düştüğü parçaları (chunk) çözer, parçayı türün büyük uçlu baytlarına çevirir ve
istenen kısmı verir. Çözülen parçalar 256 MB'a kadar saklanır, en eski önce bırakılır.

Böylece ADR 0243'ün bütün kodu (Raster ekle, Mesh ekle, `Cube`, Kesit, Zaman serisi, Mesh hesaplayıcı, `cad.meshes.*`,
İşlemler'in çok boyutlu araçları) değişmeden çalışır. Her ev sahibi yalnız baytlarını sanal dosyaya çevirir.

**Parçaların yeri.**

- HDF5: bölüt tablosu (`Range`, parçanın süzgeç maskesiyle), bitişik veri (bütün satırlar, ~1 MB'lık parçalar);
- Zarr: anahtarlar (`Keys`: önek, ayırıcı, v2 ya da v3 anahtar biçimi, sharding);
- GRIB: her alan bir mesajın veri bölümüdür (`Range`, maske alanın mesajdaki sırası).

**Çözücüler.** zlib, gzip, Zstandard, HDF5'in LZ4'ü, numcodecs'in LZ4'ü, Blosc (BloscLZ, LZ4, Snappy, zlib, Zstandard; bayt
ve bit karıştırma), bayt karıştırma, Fletcher32, CRC32C. Blosc kendi kodumuzdur (`multidim::blosc`); öbürleri biçim
çekirdeğinin çözücüleridir (ADR 0628).

**Eksik parça.** Yazılmamış parça doldurma değeridir. Zarr'da `fill_value` `null` ise zarr-python gibi sıfırdır ve
`_FillValue` söylenmez (mesh kopyası bunu yakaladı: `layer` koordinatının 0'ı NaN okunuyordu).

### §2 HDF5 ve NetCDF-4

Kendi okuyucumuz (`multidim::hdf5`):

- **Süper blok** 0–3; **nesne başlıkları** v1 ve v2, devamları ve paylaşılan iletileriyle.
- **İletiler:** veri uzayı, veri türü (tam sayı, kayan nokta, sabit ve değişken uzunluklu metin, referans), doldurma
  değeri, yerleşim v1–v5 (v5 v4 gibi okunur), süzgeç hattı, öznitelikler (yoğun öznitelikler fraktal yığın ve v2 B-ağacı
  tür 8 ile).
- **Bağlar:** bağ iletileri, yoğun bağlar (tür 5), eski sembol tablosu (v1 B-ağacı, SNOD, yerel yığın). Değişken
  uzunluklu metin ve `DIMENSION_LIST` referansları global yığından okunur.
- **Parça dizinleri:** v1 B-ağacı, tek parça, örtük, sabit dizi (sayfalı), genişleyen dizi, v2 B-ağacı. Süzgeçli parçanın
  boy alanının uzunluğu kayıttan ya da öğe boyundan okunur, formülden değil.
- **NetCDF-4 boyutları:** `DIMENSION_SCALE` veri setleri boyuttur. Değişkenin boyutları `DIMENSION_LIST`'ten,
  `_Netcdf4Dimid` ve `_Netcdf4Coordinates`'ten gelir. Ölçeksiz boyut, uzunluğuyla `phony_dim_k` olur (h5netcdf'in kuralı).
  HDF5'in ve NetCDF-4'ün gizli öznitelikleri (`CLASS`, `NAME`, `REFERENCE_LIST`, `_NCProperties` …) atılır.

### §3 GRIB

Kendi okuyucumuz (`multidim::grib`):

- **GRIB1:** ızgara 0 (enlem-boylam), basit paketleme, IBM kayan noktalı başvuru değeri.
- **GRIB2:** ızgara şablonu 3.0; ürün şablonları 4.0, 4.1, 4.8, 4.11; paketlemeler 5.0 (basit), 5.2 (karmaşık), 5.3
  (uzaysal farklı karmaşık), 5.41 (PNG), 5.42 (CCSDS, kendi AEC çözücümüzle). Bitmap'li alanlar.
- **Tarama:** satırlar kuzeyden güneye, sütunlar batıdan doğuya getirilir (0x80 boylamı çevirir, 0x20 sütun öncelikli);
  0x10 (tek-çift satır) reddedilir.
- **Sayılar:** işaret ve büyüklük; eksik ölçek çarpanı (0xFF) 0 sayılır.
- **Değişkenler:** parametre, üye, istatistik, seviye türü ve ızgara başına bir değişken; zaman ortak boyuttur (UNIX
  saniyesi, `seconds since 1970-01-01 00:00:00`). Birden çok seviye varsa seviye boyutu (`isobaric`, `height` …), yoksa adın
  eki (`t_850hPa`, `u_10m`). `lat` ve `lon` koordinat değişkenleri yazılır.
- **Adlar:** GRIB2 WMO tablosu; GRIB1 WMO tablo 2 ve ECMWF 128 (`t2m`, `u10`, `v10`, `msl`).

### §4 Zarr

Kendi okuyucumuz (`multidim::zarr`):

- **v2:** birleşik `.zmetadata`, ya da `.zgroup` ve listeleme, ya da tek `.zarray`; türler, sıkıştırıcı ve süzgeçler,
  Fortran sırası, `dimension_separator`, xarray'in `_ARRAY_DIMENSIONS`'ı.
- **v3:** `zarr.json`, içinde birleşik üst bilgi ya da listeleme; codec'ler `bytes` (uç), `transpose` (tam ters), gzip,
  zstd, blosc, crc32c, `sharding_indexed`; anahtar biçimleri `default` ve `v2`; `dimension_names`.
- **Doldurma:** NaN olmayan doldurma değeri `_FillValue` özniteliğidir.

Zarr deposu bir klasördür. Masaüstünde ve başsız sunucuda klasörün kendisi ya da üst bilgi dosyası (`.zmetadata`,
`zarr.json`, `.zgroup`, `.zarray`) verilir; raster klasörün yolunu tutar.

### §5 Ev sahipleri ve ağ

**Sanal görünüm.** `kentos_native_application::multidim_source` dosyanın ilk baytlarına bakar: HDF5 ya da GRIB ise
sanal dosyayı açar (`chunked_view`). Zarr klasörü (`zarr_folder`) ve `.zarr` ile biten adres (`zarr_address`) ayrıca
açılır. Açılan sanal dosyalar (en çok 8) yol, boy ve değişme zamanıyla ya da adresle saklanır: karo karo çizilen raster
ve adım adım ilerleyen araç dosyayı bir kez açar.

Kullananlar:

- masaüstünün raster karoları ve çok boyutlu pencereleri (`rasters/tiles.rs`'in `netcdf_source`'u);
- başsız sunucunun dosyaları (`kentos_processing::local_files`);
- `cad.meshes.*` (`source_bytes::netcdf_bytes_for`).

**Ağ.** `kentos_services::remote` iki arayüz tanımlar: aralıkla okunan dosya (`RangeRead`) ve nesne alma (`Remote::object`,
404'te ve S3'ün 403'ünde yok). Uygulama katmanı ağ bilmez; ev sahibi ağını bir kez kaydeder (`remote_host::register`):

- masaüstü ve MCP bulut istemcisinin aralık okuyucusunu (`kentos_cloud::range::HttpRemote`: sürüm denetimi, S3 imzası,
  SAS);
- Python modülü (bağımlılık kuralı `reqwest`'i dışarıda tutar) Python'un kendi `urllib`'ini: `kentos._remote.Http`,
  `kentos` içe aktarılınca `set_remote` ile. Çağrı komutun çalıştığı iş parçacığında GIL'i yalnız okuma boyunca alır.

Ağı olmayan ev sahibi adresi “Bu ortam ağa çıkmıyor …” diye söyler. Böylece Python ve MCP adresteki bulutu
(`cad.pointclouds.*`, İşlemler), rasteri ve NetCDF'i (İşlemler'in raster ve çok boyutlu araçları, `cad.meshes.*`) okur.

**Masaüstü pencereleri.**

- Raster ekle ve Mesh ekle NetCDF-4, HDF5, GRIB ve Zarr'ı açar. Dosya süzgecinde `nc4`, `h5`, `hdf5`, `grib`, `grb`,
  `grib2`, `grb2` ve ayrı bir “Zarr deposu” süzgeci (üst bilgi dosyaları) vardır.
- Raster ekle'nin Adres'ine yazılan NetCDF, NetCDF-4, GRIB ya da Zarr adresi çok boyutlu veri penceresini açar. Raster
  adresi tutar, gömülmez. Zarr klasörü de gömülmez.
- Pencere dosyanın biçimini yazar (`NetCDF-4 (HDF5)`, `GRIB2`, `Zarr v2` …).

### §6 Metin bulutu

`cad.pointclouds.region`, `.profile` ve `.query` metin bulutunu iki geçişte okur (ADR 0207 §2'nin kuralı, `TextPoints`):

1. Birinci geçiş sütunları, ölçeği ve ötelemeyi bulur.
2. İkinci geçiş biçim 6 ya da 7 kayıtları 4 MB'lık parçalarla yazar.

Noktanın sırası kayıtlar arasındaki yeridir (atlanan satırlar sayılmaz). Kalıcı kimliği öbür kaynaklarınki gibi bulut +
kaynak + sıradır. Metin bulutu bütünüyle okunur; noktaları çağrının nokta sınırına sayılır.

### §7 Meshte alan ve hat

**`cad.meshes.region`** (Mesh alan istatistiği):

- **Yüz alanda mı:** merkezi (alanının ağırlık merkezi, ilk düğümünden alınarak) halkanın kapalı alanında ve hiçbir
  deliğin kesin içinde değilse. İçbükey yüzün ağırlık merkezi yüzün dışına düşerse ilk üçgeninin ağırlık merkezi.
- **Değer:** yüzün merkezinde `cad.meshes.query`'nin verdiği değer.
- **Cevap:** her mesh için yüz sayısı, alanları toplamı, isteğe bağlı yüzlerin sırası (`faces`, en çok 100 000) ve her
  veri seti için değeri olan yüz sayısı, en küçük, en büyük, yüz alanlarıyla ağırlıklı ortalama ve değer × alan toplamı.
  Toplamlar iki toplamlı (Knuth) tutulur.

**`cad.meshes.profile`** (Mesh profili):

- **İstasyonlar:** yolun başından `step` aralıkla (0, step, 2·step …) ve yolun sonu.
- **Yer:** parçasının üzerinde `başlangıç + t·(bitiş − başlangıç)`.
- **Cevap:** her istasyonun yeri, yattığı yüz (meshin dışında yok) ve her veri setinin oradaki değeri (yoksa `null`).
- **Sınırlar:** en çok 100 000 istasyon (`too_many_stations`) ve 2 milyon değer (`too_many_mesh_values`). Değer sınırı
  meshler boyunca birikir.

**Ortak kurallar:**

- `time` ve `variables` Mesh sor'unki gibidir. An verilmezse rasterin gösterdiği an; her veri setinin o andan önceki son
  adımı.
- Meshler `uids` sırasıyla; yoksa görünen katmanlardakiler.
- Okunamayan dosya `mesh_source_unreadable` uyarısıyla atlanır.
- Retler: `invalid_mesh_area` (halka ya da delik üç köşeden az, alan sıfır) ve `invalid_mesh_path` (yol iki köşeden az,
  art arda aynı köşe, adım sıfır ya da sonsuz).

**Komşular.** `cad.meshes.query`'nin bulduğu yüz `neighbours`'ı da söyler: kenarlarından birini paylaşan yüzler, sırayla.

**Masaüstü.** Mesh sor penceresinde **Sorulan: Yer · Alan · Hat** seçimi vardır. Pencere seçimden başlar: seçili alan
Alan'ı, çizgi Hat'ı, yoksa Yer'i açar. Alan ve yol Seçimden al ile yeniden okunur; Hat'ta istasyon adımı yazılır, Alan'da
yüzlerin sırası istenebilir. Yer'in cevabında komşu yüzler yazılır.

### §8 Ölçümler

Bu makinede (release), sıcak disk önbelleğiyle. Dosyaları `scripts/perf/chunked_formats.py` yazar: 48 × 721 × 1440 kayan
noktalı alan, NetCDF-4 (zlib 4, bayt karıştırma, parçalar 1 × 181 × 360) ve Zarr v3 (Zstandard, aynı parçalar) olarak;
0,25°'lik küresel ızgaranın 24 GRIB2 mesajı (basit paketleme, 16 bit). Her ölçüm sanal dosyayı yeniden açar.

`cargo test --release -p kentos-formats --test all chunked_timing -- --ignored --nocapture`:

| Dosya | Açılış | Bir adım (721 × 1440) | Noktanın serisi | Bütün değişken (199 MB) |
|---|---:|---:|---:|---:|
| `era.nc` (NetCDF-4) | 3,3 ms | 10,6 ms | 28,6 ms (48 adım) | 600 ms |
| `era.zarr` (Zarr v3) | 10,3 ms | 28,7 ms | 106,5 ms (48 adım) | 1 865 ms |
| `gfs.grib2` (GRIB2) | 23,7 ms | 1,7 ms | 439,8 ms (24 adım) | 594 ms |

- Bir adım, parçaları iş parçacıklarında çözülerek okunur. GRIB'in adımı bir mesajdır, basit paketleme doğrudan çözülür.
- GRIB'de bir noktanın serisi her adımın bütün mesajını çözer: mesaj parçalı değildir. 24 adımda 0,44 s, Zaman serisi
  aracı için yeterli.
- Zarr'ın bütün değişkeni NetCDF-4'ten üç kat yavaştır. Her parçası (1 152 parça) ayrı bir dosyadır; farkın payları
  ölçülmedi.

## Doğrulama

- **Okuyucular.** `scripts/fixtures/chunked_formats_cases.py` (netCDF4-python, h5py, zarr-python, numcodecs, ecCodes;
  KentOS kodu yok) 11 dosyanın 76 değişkenini, her değerini ve özniteliğini yazar:
  - NetCDF-4: CF ızgarası, çok değişkenli ve grup, mesh kopyası;
  - HDF5: en yeni süper blok, yoğun öznitelikler ve bağlar;
  - Zarr: v2 Blosc ve bütün sıkıştırıcılar (Fortran sırası, yazılmamış parça, `/` ayırıcı), v3 Zstandard ve sharding, mesh
    kopyası;
  - GRIB2 (basit, uzaysal farklı, bitmap'li karmaşık, PNG, CCSDS) ve GRIB1 (ECMWF 128).

  `cargo test -p kentos-formats --test all chunked` hepsini birebir okur (GRIB'de 10⁻⁹ göreli).
- **Mesh.** `scripts/fixtures/mesh_cases.py` dört komutun 26 + 41 + 29 + 25 ortak durumunu yazar. Yeni olanlar:
  - komşular;
  - alanın yüzleri (merkez halkanın ve deliğin kenarında dahil), istatistikler kesirlerle;
  - hattın istasyonları, ortak kenardaki istasyon dahil;
  - layered.nc'nin NetCDF-4 ve Zarr kopyaları (`chunked_formats_cases.py --mesh-files`) klasik dosyanın cevabını verir;
  - retler, üç meshli değer sınırı.
- **Bulut.** `pointcloud_area_cases.py` ve `pointcloud_describe_cases.py` metin bulutunu ADR 0207 §2'nin kuralıyla kendisi
  okur: bölgede iki, profilde iki, sorguda üç yeni durum.
- **Masaüstü.** `exchange::area_tests`: Mesh sor'un Alan ve Hat'ı komutların cevabını verir; üç biçim aynı istatistiği
  verir. Resimler `.run/shots/mesh-sor-*-2-alan-*`, `-3-hat-*` ve `cok-boyutlu-*` (NetCDF-4, Zarr ve GRIB'in pencereleri).
