# ADR 0573: Başsız sunucuda mozaik ve JPEG rasterler

- **Durum:** kabul edildi (2026-10-21). `plan/gis.md` CBS-41'in kalanından: “başsız mozaik ve JPEG raster”.
- **Bağlam belgesi:** ADR 0204 (raster katmanları), 0515 (başsız raster dosyaları, `LocalFiles`), 0517 (sanal raster mozaiği), 0572
  (nokta bulutu ve çok boyutlu araçlar başsız).
- **Web:** gerekmez. İş başsız sunucunun ve masaüstünün dosyalarıdır.
- **Sonraya kalan:**
  - Adresteki (HTTP aralıklı) raster başsız sunucuda okunmaz; ret nedenini söyler.
  - Gömülü mozaik açılmaz: dosyaları yanında olmalıdır. Ret bunu söyler.

## Bağlam

Başsız sunucu (Python, MCP) ADR 0515'ten beri GeoTIFF, TIFF ve PNG rasterleri okuyordu. Üç tür ise “yalnız masaüstünde” diye reddediliyordu:
JPEG resim, JPEG sıkıştırmalı TIFF ve sanal mozaik (VRT, ADR 0517). Bunların okuyucusu masaüstü kabuğundaydı
(`apps/desktop/src/rasters/tiles.rs`): JPEG'in boyu ve pikselleri zune-jpeg ile, mozaiğin dosyaları ve blokları.

## Karar

### 1. Ortak okuyucu: `kentos_processing::raster_files`

Okuyucu kabuktan işlem crate'ine taşındı:

- `read_at`: dosyanın bir bölümü.
- `jpeg_size`, `jpeg_pixels`: JPEG'in boyu ve pikselleri. Pikseller TIFF'in JPEG karoları için de kullanılır.
- `picture_reader`: PNG ya da JPEG, bütün olarak bellekte okunur.
- `mosaic_text`, `mosaic_reader`: VRT metni (en çok 16 MB, UTF-8) ve dosyaları. Mozaiğin içindeki mozaik reddedilir.
- `read_member`: mozaiğin bir dosyasının bloğu.

Masaüstü aynı işlevleri çağırır; davranışı değişmez. zune-jpeg işlem crate'ine de eklendi. Sürümü workspace'inkidir: sahibin onayı
2026-09-27, ADR 0092.

### 2. Başsız dosyalar

`LocalFiles::open_raster` şunları açar:

- **TIFF:** parça parça okunur. JPEG sıkıştırmalı karoları `jpeg_pixels` ile çözülür.
- **PNG ve JPEG:** bütün olarak okunur, en çok 512 MB.
- **VRT mozaik:** bağlı dosyadan açılır. Dosyaları VRT'nin yanında aranır, her biri kendi okuyucusuyla. Mozaiğin blokları kendi
  dosyalarından okunur.
- **Gömülü VRT:** reddedilir: “Gömülü mozaik (VRT) açılmaz: dosyaları yanında olmalı; mozaiği bağlı dosya olarak ekleyin.”

## Doğrulama

- **Bağımsız başvuru: `scripts/fixtures/headless_raster_cases.py`**
  - `--files`, GDAL'la `rgb.png`'den `fixtures/raster/v1/files/rgb.jpg`'i yazar.
  - Üç dosyanın ilk bandını GDAL'ın okuyuşuyla özetler: JPEG resim, JPEG sıkıştırmalı TIFF (`rgb-jpeg-ycbcr.tif`) ve KentOS'un VRT'si
    (`kentos-dem-verilen.vrt`). Özet: geçerli hücre sayısı, ortalama, en küçük ve en büyük değer.
  - Özet `fixtures/raster/v1/headless.json`'a yazılır. `--check` güncel olduğunu denetler.
- **`python/tests/test_headless_rasters.py`:**
  - Her rasterde, rasteri saran tek bölgeyle Bölgesel istatistik `cad.processing.run` ile çalıştırılır.
  - **Mozaik:** sayı, ortalama, en küçük ve en büyük, tablonun üç basamağında GDAL'ınkiyle aynıdır.
  - **JPEG'ler:** sayı, en küçük ve en büyük GDAL'ınkiyle aynıdır. zune-jpeg'in ortalaması GDAL'ın libjpeg'inkinden yaklaşık 0,36 birim
    yüksektir (157,002 ile 156,648; 146,925 ile 146,566). Bu, iki çözücünün yuvarlama ve renk dönüşümü farkıdır. Masaüstü aynı
    çözücüyü kullanır. Test ortalamayı yarım birimle tutar.
  - Gömülü mozaiğin reddi sözüyle sınanır.
- **Masaüstü:** `cargo test -p kentos-desktop rasters::`.
