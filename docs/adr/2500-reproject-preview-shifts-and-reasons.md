# 2500 — Katmanın koordinatlarını dönüştürmede nesne başına kayma önizlemesi ve nedenleri

Tarih: 2026-11-08. plan/map-cad.md MAP-02, dördüncü tur. Durum: taslak. Plan satırı: “Izgara bulunamadığında, kapsam
dışına çıkıldığında veya epok belirsizken ‘başarılı hassas dönüşüm’ sonucu üretilmesin. Önizleme farkları ve nedenleri
göstersin.”

İlgili:
- [ADR 0583](0583-layer-source-crs-and-map-system.md) §2: katmanın kaynak sistemi, Tanımla (eski adı Sistem ata) ve Dönüştür (eski adı Yeniden izdüşür);
- [ADR 1230](1230-coordinate-chain-epoch-grids-and-budget.md) §3 ve §7. §7'deki “Önizlemede nokta başına fark ve
  nedeni” maddesini bu ADR katmanın yeniden izdüşümü için kapatır;
- [ADR 0222](0222-kml-gpx-and-explicit-transformation.md) §2: içe aktarmada yalnız noktalar ve düz kenarlı yollar çevrilir;
- [ADR 0167](0167-second-crs-and-converter.md): her datum çiftinin EPSG yolu.

**Ad değişikliği (8 Kasım 2026, sahibin onayıyla):** katmanın Koordinat sistemi menüsündeki “Sistem ata” ve “Yeniden
izdüşür” haritacılığın kelimeleriyle **Tanımla** ve **Dönüştür** oldu; komutların ve geri alma adımlarının adları
“Katmanın koordinat sistemini tanımla” ve “Katmanın koordinatlarını dönüştür”, raster için “Rasteri projenin sistemine
çevir”. Kimlikler (`cad.layers.crs`, `cad.layers.reproject`, `cad.rasters.reproject`) aynı; eski adlar arama
takma adıdır.

## Dayanak

- **OGP/IOGP, Guidance Note 7-1, “Using the EPSG Geodetic Parameter Dataset”**, sürüm 5, Kasım 2009
  ([kopya](https://cdn.NetBSD.org/pub/pkgsrc/distfiles/epsg-docs-200911/G7-1.pdf), 8 Kasım 2026'da okundu; epsg.org 403
  verdi, güncel sürüm 9'dur, Mart 2025, okunmadı):
  - §5.7: datum, sistem ve dönüşüm kayıtlarının bir kullanım alanı vardır. Kutunun sınırları yaklaşıktır, kayıt seçmek
    içindir ve WGS 84 derecesiyle verilir;
  - §6.5: dönüşüm seçmenin ölçütlerinden biri, dönüşümün kullanım alanının dönüştürülecek verinin alanıyla tutarlı olup
    olmadığıdır. Öbürleri amaç, yerini alan kayıt, doğruluk ve yöntemdir;
  - §4.3.5: dönüşüm (çevirmeden farklı olarak) çıkan konumun doğruluğundan bir şey yitirir;
  - §6.5.4.1: dönüşümün “doğruluğu” dikkatle okunmalı, çoğu zaman türetmenin iç kesinliğidir.
- **PROJ, “Computation of coordinate operations between two CRS”**
  ([belge](https://proj.org/en/stable/operations/operations_computation.html), 8 Kasım 2026'da okundu):
  - `proj_trans()` noktayı işlemlerin kullanım alanıyla karşılaştırabilir; o noktaya uyan işlemi seçebilir;
  - kullanım alanının hesabı yaklaşıktır, sınırlayıcı kutuya dayanır;
  - doğruluğu bilinen işlem bilinmeyenden önce gelir.
- **QGIS Kullanıcı Kılavuzu, “Working with Projections”**
  ([belge](https://docs.qgis.org/latest/en/docs/user_manual/working_with_projections/working_with_projections.html),
  8 Kasım 2026'da okundu):
  - §6.6: aday dönüşümlerin her satırı doğruluğunu ve kullanım alanını gösterir. Bu sistemde kullanılamayan dönüşüm
    gri görünür ve seçilemez;
  - §6.2: katmanın sistemini değiştirmek veri kaynağını değiştirmez (KentOS'un Tanımla'sı).
- **KentOS bugün:**
  - Dönüştür (o zamanki adıyla Yeniden izdüşür) önizlemesiz yazıyordu. Cevapta yalnız nesne sayısı, yol ve doğruluk vardı;
  - zincirin kararı (ADR 1230) yalnız Koordinat dönüştür'deydi;
  - yazı, daire, yay ya da blok içeren katman “(sistemin dışında)” diye reddediliyordu. Neden yanlıştı:
    `move_entity` bu türleri hiç almaz (ADR 0222 §2);
  - kataloğun özeti “yay ve eğriler dönüşümün kuralıyla çevrilir” diyordu. Bu da yanlıştı.

## Karar

1. **Cevap önizlemedir** (`cad.layers.reproject`, plan da aynısını verir). Her nesne için `shifts` satırı:
   - kalıcı kimliği, türü, nokta sayısı;
   - ilk noktasının şimdiki ve çevrilmiş yeri, o noktanın kayması (yeni − şimdiki) ve uzunluğu;
   - bütün noktalarının en küçük ve en büyük kayması.

   Birimler projenin birimidir. En çok 1000 nesne listelenir (`REPROJECT_LISTED`), hepsi sayılır. Notlu nesneler önce,
   sonra öbürleri gelir; ikisi de çizim sırasıyla.
2. **Toplu kayma:** bütün noktaların sayısı (`points`), ortalama kayma (`meanDx`, `meanDy`), en küçüğü ve en büyüğü,
   notlu nesne sayısı (`flagged`). Nesne yoksa ortalama ve sınırlar yoktur.
3. **Notlar** (`REPROJECT_NOTES`, bu sırayla). Kutular EPSG'nin kullanım alanlarıdır:
   - `outside_source_area`: bir noktası kaynak sistemin kutusunun dışında. Nokta, kaynağın kendi datumundaki
     enlem ve boylamıyla okunur. Çoğu zaman nesneler başka bir sistemdedir;
   - `outside_operation_area`: aynı yer datum dönüşümünün EPSG işleminin kutusunun dışında. Dönüşümün doğruluğu orada
     geçerli değildir. Datum aynıysa ya da yol projenin kendi dönüşümüyse bu kutu yoktur;
   - `outside_project_area`: noktanın vardığı yer projenin sisteminin kutusunun dışında.

   Kutular PROJ'un veritabanından yazılır (`fixtures/crs/v1/areas.json`, `scripts/fixtures/crs_areas.py`; EPSG
   v12.029). Kayıt dosyası değişmedi: web'in testi onu TypeScript kaynağıyla birebir karşılaştırır ve web duruyor
   ([ADR 0228](0228-web-pause-desktop-and-cloud-first.md)). Kutu yaklaşıktır (GN7-1 §5.7); not bir uyarıdır, ret
   değildir: `outside_area`.
4. **Zincirin kararı:** `status` ve `reasons`, Koordinat dönüştür'ün aynı iki sistem için verdiği karardır (ADR 1230
   §3, aynı sıra). Yalnız yatay yol değerlendirilir; Dönüştür kotları olduğu gibi bırakır (ADR 0583 §2). Sınırlı
   zincirde `chain_limited` uyarısı nedenleri sayar. Boş katmanda karar yoktur.
5. **Ret nedenleri** (`crs_unreached`): çevrilemeyen her nesnenin nedeni söylenir:
   - sistemin dışında;
   - ızgaranın dışında;
   - ızgara bu cihazda yok;
   - datumun WGS 84'e yolu yok;
   - yay ya da eğrili çizgi, daire, yazı, blok gibi tam çevrilemeyen türden.

   Neden tekse eski cümle kalır, birden çoksa her biri sayısıyla. Öneri nedene göredir: doğru sistemi `from` ile
   vermek, ya da bu nesneleri başka katmana almak. `uids` hepsini verir. Katalog özeti düzeltildi.
6. **Masaüstü** (`reproject_window.rs`). Katmanın Koordinat sistemi ▸ Dönüştür ▸ menüsü ve komut satırının
   Koordinatları dönüştür'ü artık yazmaz, önizlemeyi açar:
   - tek satırlık giriş, yol ve doğruluk, zincir (tam ya da uyarının nedenleri), notların uyarısı, toplu kayma;
   - kitin tablosu: Nesne, Nokta, Y, X, dY, dX, Kayma, En büyük, Kullanım alanı dışı (kaynak, dönüşüm, proje). CAD
     projesinde eksenler X, Y'dir;
   - Uygula tek geri alma adımıyla yazar, Vazgeç hiçbir şey yazmaz;
   - ret durumunda tablo yoktur: neden satırı, menüden başka sistem seçmenin yolu ve **Nesneleri seç** (nesneleri
     seçer, yakınlaştırır).
7. **Kapsam işareti:** **Harita + CBS** (Katmanlar panelinin menüsü ikisinde de var).

## Kanıt

- **Ortak durumlar** (`scripts/fixtures/layer_crs_cases.py`, KentOS kodu yok). Önizleme PROJ'dan yazılır:
  - nesne başına kayma PROJ'un çevirdiği noktalardan;
  - kutular `CRS.area_of_use` ve yolun işlemlerinin `CoordinateOperation.area_of_use` kutusundan. Nokta, kaynağın
    datumunda PROJ'un tersiyle okunur;
  - zincirin kararı ADR 1230 §3'ün kurallarıyla (`crs_chain_rules.py`).
- **Kurulum:** iki katman eklendi.
  - Uzak: eski paftanın bir noktası ve 44,3° K'de bir nokta;
  - Yazılar: bir nokta, bir yazı, bir daire.
- **Yeni durumlar:**
  - uzaktaki nokta üç notla önce listelenir; plan ve yazma;
  - yazı ve daire tam çevrilemez;
  - karışık nedenler (1 sistemin dışında, 2 türünden).
- **Var olan durumlar** yeni alanlarla yazıldı:
  - ED50 / TM36 → TUREF / TM36: (−21,48; −182,70) m, 183,96 m, zincir sınırlı (ED50'nin EPSG işlemi);
  - Ankara'nın GPS, UTM, TUREF coğrafi ve Web Mercator noktaları: `outside_project_area` (TM36 dilimi dışında). TUREF
    coğrafide zincir tamdır;
  - ED50 / TM33 numarası: `outside_source_area`.
- **Sonuç:** 24 durum (21'den). Eski noktalar bu Mac'in PROJ'uyla 10⁻⁸ m düzeyinde yeniden yazıldı (1 mm `within`
  içinde). Rust ve Python geçer.
- **Rust birim testi:** her kayıt sisteminin (yerel hariç) kullanım alanı vardır; kutu kenarları ve antimeridyen.
- **MCP** (`crates/native/mcp/tests/layer_reproject_preview.rs`):
  - plan durumun önizlemesini verir, hiçbir şey yazmaz;
  - yazma durumun cevabını verir;
  - karışık ret durumun cümlesini verir.
- **Masaüstü** (`reproject_window_tests.rs`):
  - pencerenin planı durumunkidir; Uygula durumun nesnelerini yazar, geri alma tek adımdır;
  - Vazgeç yazmaz;
  - ret durumun cümlesini ve kimliklerini verir, Nesneleri seç üç nesneyi seçer;
  - eski test (`map_crs_tests`) önizlemeden geçer.

  Pencerenin testleri ve çapraz denetimler (şerit, modül, katalog, ayarlar, geçmiş metni) birlikte 137 geçer. Resimler
  `.run/gui-evidence/yeniden-izdusur-2500/`.

## Sonra

- Yazı, blok ve dairenin taşınması: yerleşme noktası, meridyen yakınsaması farkıyla dönüş ve ölçek.
- EPSG'nin çokgen kullanım alanları (bugün kutu).
- Çizgilerde her köşenin satırı (bugün ilk nokta ve köşelerin en küçük/en büyük kayması).
- Koordinat dönüştür'ün kendisinde nokta başına karar (ADR 1230 §7'nin öbür yarısı).
- Büyük katmanda önizlemenin iş olarak koşması (ilerleme, durdurma).
- Web (TODOS §0.2).
