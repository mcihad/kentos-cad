# ADR 9303: Üreteçlerin sabit Python ortamı ve doğru yuvarlanmış başvurular

- **Durum:** kabul edildi (2026-11-24). todos-new.md A1.
- **Kural:** CLAUDE.md §23.4 (bağımsız, yeniden üretilebilir başvuru; beklenen örnek hataya göre yenilenmez, tolerans
  büyütülmez), §14 (ortak çekirdek ve `libm`, [ADR 0008](0008-shared-core-boundary.md)).
- **İlgili:** [ADR 0171](0171-ground-ellipsoid-plane.md) (geographiclib'in `libm` kopyası),
  [ADR 0548](0548-coverage-matrix-and-run-equivalence.md) (kapsam matrisi), [ADR 0131](0131-python-sdk.md) (Python SDK).
- **Ürün:** değişmedi. Bu ortam ürünün çalışma bağımlılığı değildir; yalnız başvuruları yazan ve `--check`'lerini koşan
  geliştirme aracıdır. **`.kcad`:** değişmedi. **Sözleşme:** değişmedi.

## Bağlam

Ortak durumların bağımsız başvuruları `scripts/*/*.py` üreteçleridir: KentOS kodu olmadan beklenenleri yazar,
`--check` ile diskteki dosyayı yeniden üretilenle karşılaştırır. 24 Kasım 2026'daki taramada (sistemin `python3`'ü,
Homebrew 3.15.1) 825 betikten 506'sı yeşil, 40'ı kırmızıydı; 258'i koşamadı (156'sı mpmath, pyproj, shapely,
ifcopenshell, scipy, ezdxf, sympy paketlerinden birini bulamadı; 102'si koşarken import hatası verdi), 21'inin
`--check` biçimi taramanın kalıbına uymadı.

Üreteçler o güne dek elde hangi Python varsa onunla koştu: Homebrew'un `python3`'ü (26 Ekim'e kadar 3.14, sonra
3.15.1), `uv run --python 3.11 --with …` (paketler o günün son sürümü), PROJ ve GDAL isteyenler QGIS'in Python'u
(3.12; PROJ 9.8.1, GDAL 3.12.2; QGIS artık kurulu değil), nokta bulutları `.run/pyref`, GeoParquet Linux'taki
`.run/geo-py`. Depo 12 Ekim'de Linux'tan bu Mac'e taşındı; Mac 14 Kasım'da macOS 26.6.2'den 27.2.1'e yükseldi.

Kırmızıların ölçülen nedenleri:

1. **Son basamak: Python değil, C kütüphanesi.** Aynı betik bu Mac'teki bütün Python'larda (3.9.6, 3.12.13, 3.14.3,
   3.15.1) aynı yanlış basamağı veriyor: `math.sin`, `tan`, `atan2`, `exp`… ve float `**` C kütüphanesinin (libm)
   işlevlerini çağırır. `walls/v1/cases.json` 18 Ekim'de bu Mac'te (macOS 26.6.2) yazıldı ve `tan(π/8)`'i doğru
   yuvarlanmış `0.41421356237309503` olarak taşıyor; macOS 27.2.1'in libm'i `0.4142135623730951` (bir sonraki double)
   veriyor. Rastgele 20 000 girdide macOS 27.2.1'in libm'i doğru yuvarlamadı: `tan` %42, `acos` %14, `atan` %8,
   `cbrt` %8, `sin` ve `cos` %4, `atan2` %0,9, `pow` %0,15; `pow(x, 2.0)` `x*x`'ten %0,14 farklı (mpmath ile 256 bitte
   ölçüldü). Rust tarafı aynı sorunu `libm` crate'iyle her hedefte aynı sonuçla çözmüştü (ADR 0008, 0171); başvurular
   ise sistemin libm'ine bağlı kalmıştı.
2. **Kütüphane başvuruları:** PROJ, GDAL, GEOS, IfcOpenShell ve NumPy'nin doğrusal cebiri de C/C++ kodunda sistemin
   libm'ini kullanır; sürümleri de değişti: başvurularda PROJ 9.5.1, 9.7.1 ve 9.8.1; GDAL 3.12.2 ve 3.13.3; NumPy'nin
   Homebrew kopyası OpenBLAS, PyPI'ın macOS 14+ tekerleği Apple Accelerate.
3. **Üreteçle dosyası ayrışmış:** üreteç ilerlemiş, örnek yeniden yazılmamış (yeni durumlar, notta taşınmış yollar) ya
   da örnek elle düzeltilmiş, üreteç geride kalmış; bir üretecin aldığı ortak modül büyüyünce sahnesi bozulmuş.
4. **`--check` yazıyor:** iki üreteç `--check`'te de dosya yazıyordu.
5. **Biçim:** `scripts/vendor/geographiclib.py`'nin yazdığı crate `cargo fmt --all` tarafından yeniden biçimlenmişti.

## Karar

### 1. Ortam: `scripts/fixtures/pyproject.toml`, `uv.lock`, `.python-version`

uv ile sabit ortam; kilit her paketin sürümünü ve özetini (sha256) tutar. Ortam `.run/fixtures-py`'dedir (Git'e
girmez); tek yol sarmalayıcıdır:

```bash
scripts/fixtures/py scripts/fixtures/<üreteç>.py [--check]   # ortamı kilitten eşitler, betiği onunla koşar
make fixtures-check                                          # hepsi (ONLY=ad, JOBS=4)
```

- **Python 3.12.13** (uv'nin python-build-standalone derlemesi). Bütün betikler 3.12'de derleniyor (3.11'de ikisi
  f-string yüzünden derlenmiyor); `triangle`'ın tekerlekleri 3.13'e kadar; QGIS'in Python'u da 3.12'ydi. Kayan
  noktalı sayının yazılışı (`repr`, `json`) sürümden bağımsızdır.
- **Sarmalayıcı (`scripts/fixtures/py`):** `uv sync --locked`; `KENTOS_QGIS_PYTHON=1` (bir üreteç eski düzendeki gibi
  kendini başka bir Python'la yeniden başlatmaz); GDAL'ın Python bağıyla kütüphanesinin sürümü aynı değilse durur ve
  ne yapılacağını söyler. macOS'ta GDAL'ın Arrow kütüphaneleri pyarrow'un aynı sürümdeki kopyalarına yönlendirilir
  (`DYLD_LIBRARY_PATH`): Homebrew'un libgdal'ı Homebrew'un Arrow'una bağlı, pyarrow kendi Arrow'unu getirir; ikisi bir
  süreçte “scheme 'file' already registered” ile düşüyordu (`import_merge_cases`, `geoparquet_cases`).
- **Paketler** (sürümler `pyproject.toml`'da; lisanslar PyPI'ın kaydından):

| Paket | Sürüm | Lisans | Neden |
|---|---|---|---|
| mpmath | 1.4.1 | BSD-3 | yüksek duyarlıklı başvurular (150 betik), `crmath` |
| sympy | 1.14.0 | BSD | kısıt başvuruları (4); mpmath'i <1.4'e sınırlar, `override-dependencies` 1.4.1'i verir, dördü yeşil |
| numpy | 2.5.4 | BSD-3 vb. | 126 betik; macOS arm64'te macOS 11 tekerleği (OpenBLAS 0.3.34), aşağıya bakın |
| scipy | 1.18.1 | BSD-3 | 58 betik |
| pyproj | 3.8.0 | MIT | PROJ 9.8.1'i ve EPSG veritabanını içinde taşır (QGIS'in PROJ'u); `crs_vertical_cases`'in alan sınırları bununla doğrulanıyor |
| shapely | 2.2.0 | BSD-3 | GEOS 3.14.1 |
| triangle | 20250106 | LGPL-3 (sarmalayıcı; Shewchuk'un Triangle'ı ticari olmayan kullanıma açık) | yüzey başvuruları (57); yalnız bu makinede başvuru yazar, ürüne girmez |
| gdal | 3.13.3 | MIT | bağ kaynaktan, Homebrew'un libgdal 3.13.3'üne karşı derlenir (84 betik) |
| ifcopenshell, ifctester | 0.9.0.post1, 0.9.0 | LGPL-3+ | IFC başvuruları |
| bcf-client | 0.9.0 | GPL-3 | BCF başvuruları; geliştirme aracıdır, dağıtılmaz |
| ezdxf | 1.4.5 | MIT | DXF başvuruları |
| fonttools, brotli | 4.68.0, 1.2.1 | MIT | yazı tipi ölçüleri (WOFF2 brotli ister) |
| laspy[laszip], laszip | 2.7.0, 0.3.0 | BSD-2, MIT | nokta bulutları |
| pillow | 13.0.0 | MIT-CMU | resimler |
| pyarrow, zstandard, lz4, geopandas | 26.0.0, 0.25.0, 4.4.5, 1.2.0 | Apache-2, BSD-3 | Parquet ve GeoParquet |
| h5py, netcdf4, zarr, numcodecs, eccodes, eccodeslib | 3.16.0, 1.7.2, 3.4.2, 0.17.1, 2.50.1, 2.50.0.34 | BSD-3, MIT, Apache-2 | çok boyutlu veri (netcdf4'ün 3.12 için macOS tekerleği 1.7.2'de) |
| matplotlib, openpyxl, pypdf, jsonschema, trimesh, owslib, botocore, psycopg2-binary, xsdata, lark, pytest | (kilitte) | PSF, MIT, BSD, Apache-2, LGPL | birer ikişer betik |
| scikit-learn, libpysal, esda, pointpats, kneed | (kilitte) | BSD-3, MIT | mekânsal istatistiğin çapraz denetimi |

- **NumPy'nin BLAS'ı:** PyPI'ın macOS 14+ tekerleği doğrusal cebiri Accelerate'le yapar; başvurular OpenBLAS'la
  (Homebrew'un NumPy'si 0.3.34, Linux) yazıldı. Aynı sürümün `macosx_11_0_arm64` tekerleği OpenBLAS 0.3.34 taşır;
  `[tool.uv.sources]` onu macOS arm64'e özetiyle sabitler. `pointcloud_network_cases` ve `pointcloud_register_cases`
  yalnız bununla yeşil.
- **Ortamın dışında kalan:** QGIS'in Python'u (`qgis` modülü) PyPI'da yok; ona dayanan betikler “koşamıyor” sayılır.
  GDAL kütüphanesi Homebrew'dandır; `brew upgrade gdal` sonrası sarmalayıcı durur: `pyproject.toml`'da gdal'ın sürümü
  yeni kütüphaneninki yapılır, `uv lock` ve `uv cache clean gdal`.

### 2. `make fixtures-check` ve `scripts/fixtures/check_all.py`

- **Üreteç kimdir:** `scripts/<klasör>/<ad>.py` içinde `--check` argümanlarla karşılaştırılıyor ya da argparse'a
  veriliyorsa (betiğin söz dizimi ağacından okunur; başka bir programa geçirilen `--check` sayılmaz). Taramanın
  kaçırdığı 21 betiğin 18'i bu kuralla bulunur. Ne verilirse verilsin yalnız denetleyen sekizi (`CHECK_ONLY`: hiçbiri
  `--files`/`--written` olmadan yazmaz, okundu) `--check`'i yok sayarak denetler. Ortak biçime getirilmeyen üçü
  listelenir, koşturulmaz: `module_links.py` (derlenmiş programın klasörünü ister), `mevzuat/olcum.py` (ölçülecek IFC
  dosyalarını ister), `mevzuat/pdfa_bagimsiz.py` (üreteç değil; qpdf'e `--check` verir).
- **Sonuç:** yeşil (çıkış 0); koşamıyor (ortamda olmayan modül, QGIS, derlenmemiş SDK, PostgreSQL; neyi istediğiyle);
  kırmızı (geri kalan her şey ve süreyi aşan). Çıkış kodu kırmızı varsa 1.
- **`--check` yazmaz:** koşudan önce ve sonra izlenen her dosyanın zamanı ve boyu alınır; değişenler sonda listelenir
  (aynı checkout'ta başkası çalışıyorsa onunkiler de görünür). Bu ölçüm iki üreteci yakaladı (Karar 6).

### 3. Doğru yuvarlanmış temel işlevler: `scripts/fixtures/crmath.py`

Başvuru sistemin libm'ine bağlı olmamalı. `crmath` her sonlu girdi için tam değere en yakın double'ı verir (eşitlikte
çift): değeri mpmath kendi bağlamında 256 bitte hesaplar, Python'un tam tamsayı bölmesi bir kez yuvarlar. Double
duyarlığının bilinen en zor durumları 2⁻¹²⁰ dolayında yakınlık ister; 256 bit bunun çok ötesidir. Özel değerler
(işaretli sıfır, sonsuz, NaN) ve hatalar (alan hatası, taşma) C kütüphanesininkidir (C99 Ek F). IEEE işlemleri ya da
CPython'un kendi kodu olanlar (`sqrt`, `floor`, `fsum`, `hypot`, `dist`, `degrees`…) `math`'ten aynen gelir.

Kullanım: üreteç `import crmath as math` yazar; float kuvvetini `math.pow(x, y)` ya da kare için `x * x` ile alır (float
`**` libm'in `pow`'udur). Doğru yuvarlanmış `pow(x, 2)` tam `x*x`'tir; yani yöntem doğru yuvarlayan bir sistemin
(glibc, macOS 26) yazdığını aynen verir.

**Uygulanan üreteçler** (yalnız kırmızı olanlar ve libm çağrısı onlarda olan ortak modüller; hangi dosyanın doğru
yuvarlanmamış çağrı yaptığı her çağrıyı mpmath'le karşılaştıran geçici bir sarmalayıcıyla ölçüldü): wall_cases,
wall_shift_cases, arch_bond_cases, arch_cases, tag_cases, text_along_cases, leader_cases, lengthen_command_cases,
stationing_cases, dimension_cases, lrs_processing_cases, plan_road_cases, renderer_cases, quick_dimension_cases,
topology_cases, block_scale_cases, geometry_generator_cases, road_sampling_cases, dxf_hatch_cases, centerline_cases,
arch_walk_cases; yol modelinin ortak modülleri road_alignment_cases, road_profile_cases, road_corridor_cases,
road_profile_command_cases, road_wall_cases, road_design_check_cases, road_terrain_compare_section_cases,
road_terrain_cutfill_cases. Bunları alan üreteçler de (text_command_cases, annotations_write_cases…) yeşile döndü.
Öbür üreteçler bu Mac'te yeşil oldukları için değiştirilmedi; son basamakta kırmızıya dönen her biri aynı tarifle
geçer. Yalnız C/C++ kütüphanesinden gelen farkı olanlar (`crmath` onları yeşile çevirmedi) geri alındı.

**Yeniden yazma sınırı:** `crmath`'e geçen bir üretecin değeri en çok 2 ulp oynarsa örnek yeniden yazılır ve onu okuyan
Rust testleri koşturulur: eski değer onu yazan sistemin bir doğru yuvarlamayı kaçırmasıdır (ör. `walls/v1/cases.json`'un
alanı: o durumun atan2/tan/sin sonuçlarından birinin tek 1 ulp kayması eski değeri aynen verir). Daha çok oynarsa
yazılmaz, sahibine sorulur (Doğrulama'daki liste). Böyle yeniden yazılanlar: `walls/v1/cases.json` (1 değer, 1 ulp),
`arch/v1/cases.json` (6 değer, ≤2 ulp), `road/v1/profiles.json` (65 değer, 1 ulp), `road/v1/sections.json` (4 değer,
1 ulp), `road.terrains.cutfill.json` (47 değer, ≤2 ulp).

### 4. Kütüphane başvuruları

PROJ, GDAL, GEOS, IfcOpenShell ve NumPy'nin sonucu kendi C/C++ kodlarındaki libm'e ve sürüme bağlıdır; ortam sürümü
sabitler, sistemin libm'ini sabitleyemez. Bunların son basamağı başka bir sistemde (Linux denetim makinesi) yine
oynayabilir. Kural aynıdır: en çok 2 ulp oynayan yeniden yazılır (bu Mac'in sonucudur, ADR bunu böyle kaydeder):
`cad.datasets.describe.json` (11 değer), `gnss/v1/import.json` (3), `geodesy/v1/ntv2.json` (3), `atlas/v1/cases.json`
(4), `cad.orthophoto.footprint.json` ve `cad.orthophoto.network.json` (3), hepsi 1 ulp. Daha çok oynayanlar
yazılmadı, listelendi. Yalnız sürüm dizgesi değişenler (`"proj": "9.7.1"` → `"9.8.1"`, GDAL'ın sürümü, Python'un
sürümü; her değer aynı) ortamın sürümüyle yeniden yazıldı.

### 5. Ayrışmış üreteçler

- **Örnek yeniden yazıldı** (fark okundu, Rust testleri koştu): `cad.walls.joins.json` (ADR 0785'in beş “Doğramaya
  kadar dönüş” durumu), `cad.field.package.json` (as-built çiziminin iki dönem katmanı, ADR 3620; beklentiler aynı),
  `cad.templates.apply.json` ve `cad.templates.apply.review.json` (0d667170d'nin 21 bloğu), `ifc/v1/revizyon/
  kesit-binasi-r2.ifc` (kaynağı ADR 8166 ve 8169'dan sonra yeniden yazılmıştı; iki değişiklik aynı), dört komut
  dosyasının notu (Rust dosyalarının taşınmış yolları).
- **Üreteç düzeltildi, örnek aynı kaldı:** `share_cases` (Veri girişi rolü, ADR 0411; künye ipuçları, ADR 8100: dosyaya
  elle yazılmıştı), `arch_bond_conflict_cases` (arch_bond_cases'in sonradan eklenen ızgara, kolon ve yay duvarları
  sahneye girmiş, düz çizgi aynı kimliği (13) ızgarayla paylaşmıştı; `bond_drag_scene`'in hatası buydu, dosyalar
  11 000 satır büyüyordu), `acceptance_corpus` (Çizgiden duvar'ın Yöntem seçeneği tek harflik düzenle B oldu, ADR
  8003; iz dosyası elle düzeltilmişti).

### 6. `--check` yazmaz

`gpkg_open_cases` `--check`'te on iki GeoPackage'ı yeniden yazıyordu (sqlite'ın baytları Python derlemesine göre
değişir); artık geçici klasörde kurar, yalnız `expected.json`'u karşılaştırır. `ifc_inspect_cases` bozuk IFC
dosyalarını yazıyordu; artık karşılaştırır.

### 7. geographiclib

`cargo fmt --all` yol bağımlılıklarını da biçimler; crate'in biçimlenmiş hali `fmt-check`'in tuttuğudur. Betik Rust
dosyalarını araç zincirinin rustfmt'inden geçirip (crate'in sürümü, 2018) yazar ve karşılaştırır; crate değişmedi.

### 8. Kapsam matrisi (`scripts/gis/coverage.py`)

Matris web uygulamasına bağlı değildir: okuduğu `commandCatalog.json`'u kentos-commands yazar ve testleri tutar; dosya
yalnız `apps/web` klasöründe durur (taşımak Rust yazarını ve okuyucularını birlikte değiştirmeyi ister). Çıktıyı
makineye bağlayan İşlemler yarısıydı: SDK yoksa matris araçsız yazılıyordu. Artık SDK yüklenemezse durur (çıkış 2,
“koşamıyor”). `docs/gis/coverage.*` bugünkü katalogla (523 komut) yeniden yazıldı.

## Doğrulama

- **Neden kanıtı:** bu Mac'teki dört Python (3.9.6, 3.12.13, 3.14.3, 3.15.1) aynı kırmızıyı verdi; math'in libm
  işlevlerini doğru yuvarlanmışlarıyla değiştiren geçici bir sarmalayıcı ilk on kırmızının sekizini örneğe dokunmadan
  yeşile çevirdi; `crmath`'e geçen 20 üretecin 18'i örnek değişmeden yeşil.
- **Rust testleri** (yeniden yazılan her örneğin okuyucusu, `scripts/dev/svc.sh heavy` ile): kentos-geometry-core
  `arch::`, `walls::`, `crs_text::`, `crs_ntv2::`; kentos-contracts `identity::`; kentos-services `mvt::`;
  kentos-road `profiles::`, `sections::`; kentos-native-application `every_command_case` (cad.walls.joins,
  cad.field.package, cad.layers.time, cad.scenarios.edit, cad.layers.service, cad.addresses.define,
  cad.templates.apply[.review], road.terrains.cutfill, cad.datasets.describe, cad.orthophoto.*) ve `ifc_revisions`;
  kentos-desktop-arch `ifc_revisions`; kentos-interaction `gnss`; kentos-headless `atlas_cases`. Hepsi geçti.
- **Son koşu** (`check_all.py -j 4`, 24 Kasım akşamı, 823 betiğin 821'i bitince): 749 yeşil, 68 kırmızı, 4 koşamıyor
  (QGIS: aggregate_qgis, expression_qgis, geopackage, relate), 3 başka biçimde. Kırmızılar:
  - 2 ulp'den çok oynayan, yazılmayan (sahibin kararı): kütüphane başvuruları crs_transform (≤4 ulp), crs_bounds (10),
    crs_custom (bir enlemde 9·10⁻¹⁰°), crs_measure, ground (areaNoise bir gürültü ölçüsüdür, iki katına çıkıyor),
    ground_survey (4258 ulp, PROJ 9.5.1'le yazılmış), gis_import (≤48), reproject_systems (≤10), photo_import (≤6),
    geoprocess (≤19), kml (≤9), table_points_systems (≤8), map_warp, map_survey, map_survey_network,
    service_tiles, network_adjust, network (≤4), network_processing (≤6), overlay (GEOS, 4096), raster_georef (GDAL),
    ifc_read (IfcOpenShell, ≤87); yol modelinin 40 üreteci (crmath'le de 4–16 384 ulp: hesap zinciri farkı büyütüyor;
    road_platform_water_cases ortak modül crmath'e geçince 13 ulp ile kırmızıya döndü).
  - Ortam: raster_codec_cases (Homebrew'un GDAL'ında LERC kodeki yok), pointcloud_scene (Linux'ta yazılmış; NumPy'nin
    sin/cos/exp'i ve rastgele sayıları platforma bağlı), brand_sizes (piksel aynı, PNG baytları Pillow tekerleğinin
    zlib-ng'si yüzünden farklı; scripts/assets bu işin dışında), distance_scene (OpenBLAS'lı NumPy tekerleğine
    geçince kırmızı; Accelerate'liyle yeşildi).
  - Başkasının süren işi: blocks_command_cases, import_decisions_cases (blok kitaplığı), scripts/gis/coverage.py
    (çalışma ağacındaki işlenmemiş katalog değişikliği; kendi commit'inde yeniden yazılmalı).
- **Açık kalan:** yukarıdaki kırmızılar; ortamı Linux'ta denemek (kilit Linux x86_64'ü de çözüyor).
