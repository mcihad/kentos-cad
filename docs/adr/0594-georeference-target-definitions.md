# ADR 0594: Raster oturtmada hedeflerin sistemi tanımla (WKT, PROJ) ve `.points` dosyasının sistem satırı

- **Durum:** kabul edildi (2026-10-23). `plan/gis.md` CBS-29'un kalanı (ADR 0518'in “sonraya”sı): dış sağlayıcının sistemine
  adaptör, projenin özel tanımında hedefler, `.points`'in `#CRS:` satırı.
- **Bağlam belgesi:**
  - ADR 0518: rasterin oturtma kaydı, `cad.rasters.georef`, `.points` dosyası;
  - ADR 0168: projenin özel sistemleri, WKT ve PROJ okuma (`crs::text`), datum seçimleri;
  - ADR 0583: katmanın kaynak sistemi;
  - ADR 0265: yeni şemanın altı adımı.
- **Web:** ertelendi (ADR 0228), TODOS.md §0.2.
- **`.kcad` şeması:** 100. Oturtma kaydının `crs`'i.
- **Sonraya kalan:**
  - KentOS'un sistem çekirdeğinin okumadığı izdüşümler (Lambert, Albers, Mercator dışı): bugün `crs_text_unreadable`;
  - yeniden örneklenen çıktının projeninkinden başka bir sistemde yazılması;
  - veritabanındaki raster sağlayıcıları (PostGIS raster).

## Bağlam

ADR 0518 oturtmayı kalıcı yaptı ama hedeflerin sistemini yalnız bir EPSG koduyla alıyordu. Saha ve kurum verisinde hedefler
çoğu zaman kayıtlı bir kodu olmayan bir sistemde gelir:

- belediyenin kendi orta meridyenli TM'si;
- bir sağlayıcının WKT'si;
- QGIS'in georeferencer'ının `.points` dosyasının ilk satırı (`#CRS: PROJCRS[…]`).

KentOS bu satırı okuyordu ama “okunmadı, sistemi pencerede verin” deyip atıyordu. Noktaları dosyaya yazarken sistem satırı hiç
yazılmıyordu, QGIS dosyayı açınca sistemi sormak zorunda kalıyordu. ADR 0518'in sınırı açıktı: “kayıt hedeflerin sistemini EPSG
koduyla tutar (projenin özel tanımı değil)”.

## Karar

### 1. `cad.rasters.georef`'in `crs`'i ve kaydın `crs`'i (`.kcad` şema 100)

Komutun girdisi `srid`'in yanında `crs` alır: WKT 1, WKT 2 ya da bir PROJ dizesi. Okuma ADR 0168'in `crs::text::read_text`'iyle
yapılır, projenin Özel koordinat sistemi penceresiyle aynı kuraldır. Okunan tanım üç hâlden birine düşer:

- **kayıttaki bir sistemin kendisi** (`registry_match` tam): EPSG koduymuş gibi davranılır ve kayda `srid` yazılır. Örneğin
  TUREF / TM30'un WKT'si `5254` olarak kaydedilir, TUREF coğrafi sistemi `5252` olarak;
- **projenin kendi sistemi**: hiçbir şey çevrilmez, kayda bir şey yazılmaz;
- **başka bir tanım**: hedefler o sistemden projenin sistemine projenin datum seçimleriyle (`crs::transform_in`) çevrilir. Kayıt
  tanımı **verildiği gibi** tutar (`RasterGeoref.crs`, en çok 16 KB). Yeniden çalıştırma aynı metni yeniden okur.

Retler:

- `srid` ile `crs` birlikte ya da tanım boş veya çok uzun: `invalid_input`;
- tanım okunamıyor: yeni kod `crs_text_unreadable`, sözleri Özel koordinat sistemi penceresininki (sözdizimi, desteklenmeyen
  izdüşüm, birim, başlangıç meridyeni);
- yerel proje (sistemi yok): `target_not_converted`;
- çevrilemeyen nokta: yolu `points[i].target` ile `target_not_converted`.

Datum, metinde adıyla TUREF'se TUREF'tir. Sıfır `+towgs84`'lü bir PROJ dizesi WGS 84 çerçevesindedir ve TUREF'e EPSG'nin boş
coğrafi dönüşümüyle (TUREF to WGS 84 (1)) gelir: enlem ve boylam aynı kalır, elipsoit GRS80 olur. Elipsoitlerin farkı yüzünden
nokta salt izdüşümden 0,1 mm güneye düşer. Bağımsız başvuru bunu yer merkezli adımla açıkça hesaplar. PROJ'un kendisi sıfır kaymada
elipsoit değişimini atlıyor; KentOS EPSG'nin kuralını izler.

Şema 100, ADR 0265'in altı adımıyla:

- kodek yalnız `crs`'li kayıt varken 100 yazar;
- şema 2–99'da `crs` bilinmeyen alandır;
- `srid` ile birlikte ya da boş `crs` `bad_value`'dur;
- örnek `georef-crs.kcad` ve üç bozuk örnek;
- bağımsız okuyucu `tools/kcad/kcad.py` ve yazıcı `kcad_v2_reference.py` aynı kuralla;
- belirtim `docs/specs/kcad-v2.md`.

### 2. `.points`'in sistem satırı

- **Okuma:** `#CRS:` satırı hedeflerin sistemidir. Kayıttaki bir sistemin kendisiyse pencerenin EPSG alanına kodu yazılır, değilse
  tanım olarak alınır. Okunamazsa bu söylenir, noktalar yine alınır.
- **Yazma:** Noktaları kaydet… ilk satıra `#CRS:` yazar, QGIS'in okuduğu gibi tek satırda. Pencerede bir tanım varsa o yazılır;
  yoksa verilen EPSG kodunun ya da projenin sisteminin WKT 2'si (`kentos_project::wkt2`). Böylece QGIS dosyayı sistemiyle açar.
- `kentos_formats::raster::gcp::write`'ın yeni `crs` parametresi. Bağımsız başvuru `gcp_file_cases.py`, çok satırlı tanımın tek
  satıra yazılmasıyla.

### 3. Masaüstü

- Raster oturt'ta EPSG alanının altında “Hedeflerin sistemi: Belediye TM 36.5 (tanım)” satırı ve **Kaldır** düğmesi vardır.
- Tanım `.points` dosyasından ya da rasterin kaydından gelir.
- EPSG alanına bir kod yazmak tanımın yerini alır.
- Özette “Hedefler “…” tanımından projenin sistemine çevrildi.” satırı vardır.

Resimler yolda iki kusur gösterdi, ikisi de düzeltildi:

- **Kesik koordinat:** Hesap tablolarının değer sütunları eşit genişlikteydi, milimetreli kuzey koordinatı (11 hane) kesiliyordu.
  Tabloya sütun başına pay geldi (`Table::portion`); Raster oturt'ta hedef sütunları 3, ötekiler 2 pay alır.
- **Yutulan boşluk:** başlıkta birimin önündeki boşluk bazı genişliklerde yutuluyordu (“Sütun(piksel)”). Aralık artık satırın
  kendi boşluğuyla verilir.

## Doğrulama

- Ortak durumlar: `fixtures/commands/v1/cad.rasters.georef.crs.json`, 20 durum. Bağımsız başvuru
  `scripts/fixtures/raster_georef_crs_cases.py`: hedefler tanımın sistemine PROJ'la (GDAL'ın osr'si) yazılıp milimetreye
  yuvarlanır, PROJ'la geri çevrilir. Sıfır `+towgs84` için yer merkezli adım açıkça hesaplanır. Dönüşümler ve ızgaralar
  `raster_georef_command_cases.py`'nin numpy hesabıyla bulunur, KentOS kodu olmadan. Durumlar:
  - WKT 1, WKT 2, PROJ;
  - kayıttaki sistem (5254, 5252) ve projenin kendisi;
  - afin, Helmert; plan olarak projektif, polinom 2, ince plaka;
  - sekiz ret.

  ADR 0518'in 38 durumu değişmedi.
- KCAD: `cargo test -p kentos-kcad --test all` (`a_record_with_a_definition_is_schema_100_and_comes_back`, fixtures),
  `python3 scripts/fixtures/kcad_v2_reference.py --check`.
- `.points`: `python3 scripts/fixtures/gcp_file_cases.py --check`, `cargo test -p kentos-formats --test all raster_gcp`.
- Masaüstü: `rasters::tests::raster_oturt_takes_a_files_system_and_writes_it_back`. Dosyanın tanımı alınır, kayda ve geri dosyaya
  yazılır; kayıttaki sistemin WKT'si kod olur; Kaldır çalışır. Resimler `KENTOS_SHOTS_ONLY=raster-oturt-tanim cargo test -p
  kentos-desktop tools_screens -- --ignored --nocapture` ile `.run/shots/arac-raster-oturt-tanim-*`.
- Python ve MCP: `kentos.cad.rasters.georef(crs=…)`; ortak durumlar tipli katmandan da geçer.
