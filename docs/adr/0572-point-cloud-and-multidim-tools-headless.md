# ADR 0572: Nokta bulutu ve çok boyutlu araçlar başsız

- **Durum:** kabul edildi (2026-10-21). `plan/gis.md` CBS-41'in kalanından: “nokta bulutu ve çok boyutlu araçların başsız yolu”.
- **Bağlam belgesi:** ADR 0207 (nokta bulutu), 0243 (mesh ve çok boyutlu veri), 0515 (başsız raster dosyaları, `LocalFiles`), 0548
  (açık çizimin dosyası, kapsam matrisi).
- **Web:** gerekmez. İş başsız sunucunun ve masaüstünün dosyalarıdır; web'in nokta bulutu yoktur (ADR 0207) ve web dururken (ADR 0228)
  çok boyutlu araçları değişmez.
- **Sonraya kalan:**
  - Adresteki (HTTP aralıklı) bulut, raster ve NetCDF başsız sunucuda okunmaz; ret nedenini söyler.
  - Başsız sunucuda mozaik (VRT) raster olarak açılmaz; JPEG raster ve JPEG sıkıştırmalı TIFF de açılmaz.
  - Metin bulutunun (XYZ, PTS, TXT, CSV) planı başsız sunucuda her çalıştırmada yeniden taranır; çalıştırma boyunca önbellektedir.

## Bağlam

ADR 0515'ten beri raster araçları Python'dan ve MCP'den çalışıyordu (`kentos_processing::local_files::LocalFiles`). Nokta bulutu araçları
(Seyrelt, Zemin süzgeci, Karola, Rasterleştir …) ve çok boyutlu araçlar (Kesit, Zaman serisi, Mesh hesaplayıcı) ise başsız sunucuda
“yalnız masaüstünde” diye reddediliyordu. Bulutun LAS, LAZ, COPC ve metin okuyucusu, COPC dizini kurucusu ve NetCDF'in dilimi masaüstü
kabuğundaydı (`apps/desktop/src/pointclouds/`, `rasters/tiles.rs`). CLAUDE.md §0'ın kuralı da çiğneniyordu: sunum olmayan iş kabukta
duruyordu.

## Karar

### 1. Ortak okuyucu: `kentos_processing::cloud_files`

Bulutun ve NetCDF'in dosya işi kabuktan işlem crate'ine taşındı. Okuyucu `ReadAt` (boy ve konumdan okuma) üstündedir. Masaüstünün
`Bytes`'i (dosya, bellek, adres) ve başsız sunucunun dosyası bunu uygular.

- `cloud_read`: biçime göre bulutun okunuşu (LAS, LAZ, COPC; metin bulutu `text_plan`'ın planıyla).
- `reading`, `build_index`, `copc_file`: bir sonuç dosyasından COPC dizini kurulur ve COPC dosyası yazılır (geçici dosya, sonra yeniden
  adlandırma).
- `netcdf_slice`: NetCDF'in başlığı ve bir dilimin gerektirdiği baytlar. `netcdf_reader`: o dilim raster olarak (mesh'in ikinci tur
  gereksinimleriyle).

Masaüstü aynı işlevleri çağırır. Davranışı değişmez; kendi önbelleği (dizin, küp) kabukta kalır.

### 2. Başsız dosyalar

`LocalFiles` şunları da yapar:

- `open_cloud`: bağlı ya da gömülü bulut; metin bulutunun planı çalıştırma boyunca önbellekte.
- `copc`: yazılan bulutun COPC'si.
- `open_cube`: NetCDF küpü (Mesh hesaplayıcı, Zaman serisi).
- `open_raster`: NetCDF veri setini gösteren raster de (Kesit'in yüzeyi).

Dosyalar çizimin klasöründen ve arama yollarından bulunur (ADR 0548 §3). Sonuç sorulan yere ya da kaynağın yanına yazılır.

### 3. Söz

Özet, yazılan dosyanın ev sahibinin verdiği yolunu söyler. Masaüstünde bu sorulan addır, başsız sunucuda çizimin klasörüne göre çözülmüş
yoldur. Ortak durumlar dosyayı adıyla söyler. Python testi özetteki yolu adına indirerek karşılaştırır.

## Doğrulama

- **`python/tests/test_headless_clouds.py`:**
  - **Çok boyutlu:** `fixtures/processing/v1/multidim.json`'ın 9 durumu `cad.processing.run` ile, kopyalanmış bir klasörde, arama yolu
    `multidim` ile oynatılır: özet, tablo, ret. Mesh hesaplayıcı'nın yazdığı iki NetCDF dosyası başvurununkiyle bayt bayt aynıdır.
  - **Bulut:** köyün bulutu (`fixtures/interaction/v1/pointclouds.kcad`) LAZ'a ve COPC'ye seyreltilir, zemini süzülür. Sayılar bağımsız
    başvurunun (`fixtures/pointcloud/v1/ops.json`) sayılarıdır. Yazılan dosyaların başlığı standart kitaplıkla okunur. COPC LAS 1.4'tür ve
    COPC kaydını taşır. Geçici dosya kalmaz.
- **Masaüstü:** `cargo test -p kentos-desktop pointclouds` (okuyucu ve dizin aynı sonuçla).
- **İşlemler:** `cargo test -p kentos-processing --test cases` (pencerenin ortak durumları).
- **Ek düzeltme:** ADR 0567'nin ağ araçlarına eklediği Profil ve Tarih, varsayılanların ortak dosyasında (`network_processing_cases.py`)
  yoktu. Yazıldı.
