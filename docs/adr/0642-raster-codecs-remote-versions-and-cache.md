# ADR 0642: Raster kaynağında codec profili, değişen adres, Range'siz sunucu ve önbellek bütçesi

- **Durum:** kabul edildi (2026-10-26). `plan/gis.md` CBS-28'in kalanı: codec profili, ortak raster kaynağının yaşam döngüsü
  (Range yokluğu, ETag değişimi, disk ve RAM bütçesi), çok gigabaytlı ortofotoda ilk görüntü. Sanal mozaik ADR 0517'de, COG
  yazımı ADR 0515'te bitti. Kapsamı ben belirledim.
- **Bağlam belgesi:** ADR 0204 (raster katmanları, karolar, piramit), 0207 §1 (adresten parça parça okuma), 0515 (COG), 0517
  (sanal mozaik), 0627 (GeoPackage'ın WebP karoları).
- **Web:** ertelendi (ADR 0228). Web'de gerekenler TODOS.md §0.2'de.
- **`.kcad`:** şema değişmez.
- **Sonraya kalan:**
  - LERC, LZMA, JPEG XL ve CCITT sıkıştırmaları okunmuyor; profilde yazılı, ret sözü çevirme yolunu verir.
  - Range'siz sunucudan indirme pencerede ilerleme göstermez ve durdurulamaz; en çok 8 GB indirir.
  - Kimlikli adres (belirteç, parola) ham raster için yok; ADR 0208'in servis bağlantıları karolar içindir.

## Bağlam

Bugün COG'lar çoğunlukla Deflate, LZW, JPEG, ZSTD ya da WebP sıkıştırmasıyla yazılıyor; GDAL'ın COG sürücüsü ZSTD'yi ve WebP'yi
önerir. KentOS'un TIFF okuyucusu ZSTD'yi ve WebP'yi okumuyordu, ret sözü de hangi sıkıştırma olduğunu adıyla söylemiyordu.

Adresten okunan raster sunucuda değişirse okuma “açıldıktan sonra değişmiş; nesneyi yeniden ekleyin” diye duruyordu. Kullanıcı
elle yeniden eklemeliydi.

Parça parça okumaya (HTTP Range) izin vermeyen sunucu ya da adresteki PNG ve JPEG reddediliyor, “indirip dosya olarak ekleyin”
deniyordu; uygulama bunu kendisi yapmıyordu.

Büyük rasterlerin önizleme piramitleri cihazın önbelleğinde birikiyordu; sınırı yoktu.

## Karar

### 1. Codec profili

TIFF'in sıkıştırmaları tek tabloda (`raster::layout::TIFF_CODECS`): GDAL'ın GTiff sürücüsünün yazdığı her sıkıştırmanın tag
değeri, adı ve KentOS'un okuyup okumadığı. Okunanlar:

- sıkıştırmasız, LZW, Deflate (8 ve 32946), PackBits, JPEG (ADR 0204);
- **ZSTD (50000):** formatların kendi ZSTD çözücüsüyle (`compress::zstd`, ADR 0628 §1); yatay ve kayan nokta öngörücüsü,
  karo ve şerit, iç içe ve düzlemsel bantlar;
- **WebP (50001):** GeoPackage karolarının çözücüsüyle (`raster::webp`, image-webp); yalnız 8 bitlik, bantları iç içe RGB ve
  RGBA (GDAL başkasını yazmaz). RGB görüntüden okunan RGBA blokta saydamlık bandı tamdır.

Okunmayanlar (CCITT RLE, Fax3, Fax4, LERC, LZMA, JPEG XL) adlarıyla ve numaralarıyla reddedilir; söz okunanları sayar ve
dosyanın Deflate'e çevrilme yolunu verir: `gdal_translate -co TILED=YES -co COMPRESS=DEFLATE girdi.tif cikti.tif`
(`layout::compression_refused`). Raster ekle'nin bilgi satırında sıkıştırmanın adı yazar.

### 2. Adresteki dosya değişince yeniden açılır

Adresten okuma her istekte sunucunun sürümünü (ETag ya da Last-Modified) açılıştakiyle karşılaştırır. Ayrılınca okuma
`range::CHANGED` sözüyle reddedilir (`range::is_changed`). Raster karolarının işçisi bu reddi alınca:

- kaynağı kapatır, bir sonraki karoda yeniden açar (yeni sürümün başlığı, yeni piramit kimliği);
- kaynağın sayacını artırır; karoların anahtarı bu sayacı taşır. Eski sürümün önbellekteki karosu yeni sürümünkilerle hiç
  gösterilmez;
- pencereye bir kez söyler: “… adresindeki raster değişti; yeni sürümü okunuyor.”

Değişiklik ilk yeni blok okumasında bulunur; o ana dek gösterilen karolar tek bir sürümün karolarıdır.

### 3. Range'siz sunucu: indir ve dosya olarak ekle

Adresin sunucusu parça parça okumaya izin vermezse, ya da adres PNG veya JPEG ise Raster ekle reddin altında **İndir ve dosya
olarak ekle** düğmesini gösterir:

- dosya bütünüyle iner (`range::download`): önce `<ad>.part`, bitince adı verilir;
- yeri, çizimin dosyası varsa onun yanındaki `veri` klasörü, yoksa cihazın `kentos-cad/indirilen` klasörüdür. Ad, adresin son
  parçasıdır (sorgusuz); varsa `-2`, `-3`… eklenir;
- en çok 8 GB indirilir; bildirilen boy daha büyükse indirme başlamaz, gelen bayt sayısı bildirileni tutmazsa dosya bırakılmaz;
- inen dosya bağlı dosya olarak okunur, pencere onunla devam eder.

### 4. Bellek ve disk bütçesi

Bellek bütçeleri önceden vardı ve aynen sürüyor:

- bir rasterin okuyucusunun blokları 96 MB;
- yapılmış karolar 64 MB;
- adresten okunan baytlar 64 MB (64 KB'lık bloklar, en az kullanılan önce gider).

Disk için yeni bir bütçe var: önizleme piramitlerinin klasörü (`$XDG_CACHE_HOME/kentos-cad/raster`) en çok 8 GB tutar
(`pyramid::CACHE_MOST`).
- Bir piramit yapılınca, en uzun süredir kullanılmayanlar bütçeye inene dek silinir. Var olan piramidi açmak dosyanın zamanını
  yeniler.
- Yazılmakta olan (`.part`), yeni yapılan ve son bir dakikada dokunulan dosyalar silinmez.

### 5. Çok gigabaytlı ortofotoda ilk görüntü

Bu makinede, release, `first_view_of_a_large_cog`. Dosya `scripts/perf/big_cog.py`'nin yazdığı 4,05 GB'lık COG: 65 536 ×
65 536 RGB piksel, 256'lık JPEG karolar, 9 kat. Bu bilgisayardaki bir sunucudan HTTP Range ile okundu. Aktarılan baytları
sunucu saydı.

| Görünüm | Süre | Aktarılan | İstek |
|---|---|---|---|
| Açılış: başlık (9 katın dizinleri) | 30 ms | 1,11 MB | 7 |
| Bütün görüntü: 5. kat, 2048 × 2048, 64 karo | 37 ms | 5,24 MB | 66 |
| Yakın görünüm: 0. kat, 1100 × 650, 15 karo | 34 ms | 2,36 MB | 28 |

Toplam aktarılan 8,72 MB, dosyanın %0,2'si. İlk yararlı görüntü (açılış ve bütün görüntü) 67 ms'de ve 6,4 MB ile gelir.
Dosya bütünüyle indirilmez. Ağ gecikmesi ölçülmedi: gerçek sunucuda istek sayısı başına gidiş-dönüş süresi eklenir. Komşu
bloklar zaten tek istekte okunur (ADR 0207 §1).

## Doğrulama

- **Codec'ler:** `scripts/fixtures/raster_codec_cases.py` GDAL'la 10 dosya yazar ve okur (`fixtures/raster/v1/codecs/`,
  `codecs.json`). KentOS kodu kullanmaz. Dosyalar:
  - ZSTD: yatay öngörücülü RGB karolar; kayan nokta öngörücülü, nodata'lı f32; u16 şeritler; düzlemsel i16;
  - WebP kayıpsız: RGB ve RGBA;
  - WebP kayıplı;
  - okunmayanlar: LERC, LERC ile ZSTD, LZMA.

  `raster_codecs.rs` sınaması:
  - kodun tablosu başvurunun profiliyle aynıdır;
  - kayıpsız dosyaların 0. katı GDAL'ın okuduğuyla bit bit aynıdır;
  - kayıplı WebP'nin bant ortalamaları bir adım, örnek pikselleri altı adım içindedir;
  - okunmayanların reddi adı, numarayı ve çevirme komutunu söyler.
- **Değişen adres:** `a_changed_address_is_opened_again_and_its_tiles_made_anew`. Test sunucusu dosyayı değiştirir ve
  ETag'ini artırır. Yeni bir karo değişikliği bulur ve söyler; ilk karo yeniden istenince yeni dosyanın karosu gelir, dosyadan
  açılanla aynı olur.
- **Range'siz sunucu:** `an_address_without_ranges_is_downloaded_and_added_as_a_file`. Sunucu her isteğe bütün dosyayı
  `200` ile verir. Pencere reddi söyler ve indirmeyi önerir; dosya çizimin `veri` klasörüne iner ve baytları aynıdır. Raster
  dosya olarak okunur; ikinci indirmenin adı `-2` alır.
- **Disk bütçesi:** `the_pyramid_files_keep_within_their_budget_the_oldest_going_first`. Bütçe aşılınca en eski dosya gider;
  tutulan, yazılmakta olan ve yeni dokunulan kalır.
