# ADR 0053: Masaüstünde GeoJSON ve Shapefile; zip'li Shapefile

- **Durum:** kabul edildi (2026-09-26).
- **Tarih:** 2026-09-26
- **Bağlam belgesi:** CLAUDE.md §5, §9.7, §14, §23; DESIGN.md §7.9, §7.15; TODOS.md `FMT-07`, `UI-11`; ADR 0009 (dosya biçimleri), 0046 (GeoJSON ve Shapefile), 0048 (masaüstünde dosya alışverişi)
- **Sahibin kararı (26 Eylül):** ADR 0046'nın 4. sorusuna (a): `miniz_oxide` eklenir, zip'li Shapefile okunur; masaüstünün GeoJSON/Shapefile dilimiyle yapılır. Masaüstü web ile aynı düzeye gelene kadar yalnız masaüstü işi yapılır.

## Bağlam

- Web GeoJSON'u alıp veriyor, Shapefile'ı alıyor (ADR 0046). Okuyucu ve yazıcı ortak Rust'tadır (`crates/shared/formats`). Masaüstünde bu üç komut soluktu.
- `.zip` hiçbir yerde okunmuyordu; Shapefile'ın dosyaları birlikte seçiliyordu. Kurumlar ve portallar Shapefile'ı çoğu zaman `.zip` olarak dağıtır.
- Masaüstünün koordinat sistemi sorusu yalnız DXF'i ve koordinat listesini biliyordu: dosyanın kendi beyanını göstermiyordu.
- Masaüstünün uzun pencereleri hep en büyük boylarındaydı: kısa içerikte altlarında boşluk kalıyordu.

## Karar

### Zip okuyucusu (`crates/shared/formats/src/zip.rs`)

- Yalnız okur: sondaki dizin kaydı (EOCD), merkez dizin, yerel başlık; yöntem 0 (stored) ve 8 (deflate). Açma `miniz_oxide`'ın inflate'iyle ve sınırla yapılır (`decompress_to_vec_with_limit`); her girdinin CRC-32'si denetlenir.
- Nedeniyle reddedilenler: Zip64, çok diskli arşiv, şifreli girdi, başka sıkıştırma yöntemleri.
- Zip bombasına karşı sınırlar: 1000 girdi, açılmış toplam 512 MiB, girdi başına sıkıştırma oranı 1000. Beyan edilen boyut ve gerçekte açılan boyut ayrı ayrı denetlenir.
- Klasör girdileri, `__MACOSX/` ve `._` çatalları atlanır; ters bölü `/` sayılır.
- Kesik ya da bozuk arşiv paniğe yol açmaz: arşivin her kesiği ve bayt çevirmeleri testte okunur.

### Zip'li Shapefile (`shp::zip_layers`, `shp::read_zip`)

- Arşivdeki her `.shp` bir katmandır ve arşivdeki yoluyla anılır (`katmanlar/yollar`). Parçaları aynı klasörde aynı adı taşıyan `.shx`, `.dbf`, `.prj`, `.cpg`'dir; büyük/küçük harf fark etmez. İki klasördeki aynı adlı katmanlar karışmaz.
- Okuma düz dosyalarınkiyle aynıdır (`shp::read`). Raporun kaynak satırına "Zip arşivinden: .shp, .shx, …" eklenir.
- Fixture'lar (`fixtures/formats/v1/gis`): bağımsız betiğin (`scripts/fixtures/gis_reference.py`) kendi Shapefile takımlarından Python'un `zipfile`'ıyla yazdığı üç arşiv: deflate, stored ve bir klasörde iki katmanlı. `--check` arşivleri açar (CRC'leriyle) ve üyelerin adlarını, yöntemlerini ve baytlarını betiğin dosyalarıyla karşılaştırır. Deflate'in baytları yazan zlib'e bağlı olduğu için bayt karşılaştırması yapılmaz.
- Rust (`tests/gis.rs`): arşivdeki her katman, yanındaki düz dosyalardan okunanla float float aynı okunur. O dosyaları bağımsız okuyucu doğrular.

### Masaüstü pencereleri (`apps/desktop/src/exchange/`)

- **Komutlar:** `file.import.geojson`, `file.import.shp`, `file.export.geojson`. Pencereler web'in `GisImportDialog`'u ve `GeoJsonExportDialog`'udur: metinleri, sırası ve kuralları aynıdır.
- **GeoJSON içe aktar:**
  - katmanlar ve her birinin nereye gittiği: aynı adlı proje katmanı (kilitliyse alınmaz ve söylenir) ya da dosyanın adını taşıyan gruptaki yeni katman;
  - koordinat sistemi sorusu, rapor ve kapsam;
  - her şey tek geri alma adımıyla girer.
- **Shapefile içe aktar:**
  - dosya penceresi birden çok dosya seçer;
  - `.shp` ve aynı adlı parçaları bir katmandır (web'in `shapefileSet`'i, testleriyle); başka dosyalar kullanılmaz ve söylenir; iki `.shp` "bir seferde bir katman" diye reddedilir;
  - tek bir `.zip` seçilirse arşivdir. Birden çok katmanı varsa "Arşivdeki katman" listesinden biri seçilir ve yeniden okunur. Yeni katmanlar arşivin adını taşıyan gruba girer; aynı arşivden sonra alınan katmanlar da aynı gruba.
- **GeoJSON dışa aktar:**
  - kapsam (seçili, görünen, tümü), katmanlar ve özet;
  - WGS 84 projesi RFC 7946 yazılır; başka sistemdeki proje kendi koordinatları ve eski `crs` üyesiyle, pencere bunu önceden söyler;
  - eğriler örneklenir; yazı, ölçü ve sonsuz doğru yazılmaz;
  - dışa aktarma kaydetme değildir.
- Okuma ve yazma kendi iş parçacığında yapılır; eskiye ya da kapanmış pencereye gelen geç yanıt atılır (ADR 0048 gibi).
- Masaüstünün Shapefile komutunun açıklaması masaüstüne özgüdür (`catalog::DESKTOP_DESCRIPTIONS`): web'inki ".zip henüz açılmaz" der.

### Koordinat sistemi sorusu (`apps/desktop/src/crs.rs`)

- Web'in `CrsQuestion.declare`'idir:
  - dosyanın beyanı (RFC 7946, `crs` üyesi, `.prj`) ilk cevaptır;
  - beyan okunamazsa ya da KentOS'ta tanımlı değilse cevap yoktur ("Sistemi seçin…") ve içe aktarma kapalıdır;
  - beyansız dosya (DXF, koordinat listesi, `.prj`'siz Shapefile) projenin sistemiyle başlar.
- Kullanıcı dosyanın dediğinden başka bir sistem seçebilir (WGS 84 diyen ama TM koordinatı taşıyan dosya); pencere bunu ve anlamını söyler.
- Kapsamı bilinen dosyada, seçilen sistemde olamayacak koordinatlar söylenir: metre sisteminde derece aralığında kalan, ya da coğrafi sistemde o aralığı aşan koordinatlar.
- Yalnız projenin sistemindeki dosya içe aktarılır; koordinat dönüşümü yok (CLAUDE.md §5).

### Pencereler içerikleri kadar (KentOS UI)

- `widget::Fit`: esnek olmayan çocuklar önce yerleşir, kaydırma alanına kalan yer verilir. Iced'in sütununda içeriğe göre uzayan kaydırma alanı kendisinden sonraki düğmelere yer bırakmaz; dolduran (`Fill`) kaydırma alanı ise kutuyu hep en büyük boyunda tutar.
- `Dialog::scroll`: kutu içeriği kadar uzar. Uygulama penceresine ya da `max_height`'a sığmayınca gövde kayar; başlık ve düğmeler görünür kalır. Kaydırma çubuğu gövdenin yanındadır, içeriğin üstüne binmez.
- Dosya alışverişi pencerelerinin altısı `scroll` kullanır. Sekmeli pencereler (Uygulama ayarları, Yeni proje, Proje ayarları) sabit boydadır (`height(Fill)`).
- Katman tabloları satırları kadardır; 8 satırdan fazlası tablonun içinde kayar.

## Bağımlılık

- `miniz_oxide` 0.9.1: varsayılan özellikler kapalı, `with-alloc`; MIT OR Zlib OR Apache-2.0; saf Rust (native, wasm32).
- Kilide yeni paket girmedi: `flate2` üzerinden (`adler2` ile) zaten kilitliydi.
- Biçim WASM modülü zip okumaz (web'de `.zip` yok): 1 494 431 bayttan 1 494 614 bayta; gzip -9 ile 482 713'ten 482 787'ye.

## Bu dilimde olmayanlar

- Web'de `.zip`: ortak Rust hazır; biçim işçisine bağlayıcı ve pencere gerekir. Sahiple yapılacak planda.
- Çok katmanlı içe aktarma: ADR 0046'nın 6. sorusu, bir seferde bir katman (`FMT-08`).
- Shapefile dışa aktarma (`FMT-09`) ve koordinat dönüşümü.

## Doğrulama (26 Eylül 2026, Linux)

- `cargo test -p kentos-formats`:
  - zip birim testleri: stored, deflate, CRC uyuşmazlığı, zip bombası, şifreli girdi, Zip64, klasör ve çatallar, çok girdi, her kesik ve bayt çevirme;
  - `tests/gis.rs`: zip'li katmanlar düz dosyalarıyla aynı okunur; katman yoluyla anılır; her kesik paniksiz reddedilir.
- `python3 scripts/fixtures/gis_reference.py --check`: 12 fixture, 2 dışa aktarma çifti ve 3 arşiv. Yeniden yazılan arşivler bu makinede (zlib 1.3.1) diskteki ile bayt bayt aynı.
- `cargo test -p kentos-desktop`:
  - `exchange::gis_tests`: TM36 GeoJSON tek adımda (kilitli katman alınmaz), WGS 84 dosyasının reddi ve kullanıcının cevabı, dosya takımı ve kullanılmayan dosya, zip'te katman seçimi ve grup, bozuk arşiv, GeoJSON dışa aktarmanın geri okunması;
  - `crs::tests` (beyan kuralları), `gis_import::tests` (web'in `shapefileSet` testleri).
- `cargo test -p kentos-ui --features snapshot`: `widget::fit` (kısa gövde, uzun gövde, dolduran çocuk).
- Beyan kuralı bilerek bozuldu (her beyan projenin sistemi sayıldı): üç test yakaladı.
- Görüntüler: 1440×900 ve 1100×650, koyu ve açık (`exchange::gis_tests::screens`, `exchange::tests::screens`, `.run/shots/gis-*`, `aktar-*`).
