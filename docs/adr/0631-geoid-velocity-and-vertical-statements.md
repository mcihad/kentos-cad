# ADR 0631 — Geoit modeli, hız ızgarası ve dosyaların düşey sistemi

Durum: kabul edildi (25 Ekim 2026). Kapsam: plan/gis.md `CBS-08`'in kalanı (ADR 0519'un “Kalan”ı). Kapsamı ben belirledim:

- geoit ve hız modeli motoru;
- dış biçimlerin düşey sistemi;
- sunucu ile masaüstü sonuç farkının sınanması.

Bağlam belgeleri:

- ADR 0168 (NTv2 ve cihazın ızgara kitaplığı);
- ADR 0169 §6 (GNSS içe aktar);
- ADR 0519 (düşey sistem, epok, dönüşümün kaydı);
- ADR 0559 (sunucu işleri);
- ADR 0563 (Shapefile, GeoJSON ve KML içe aktarma);
- ADR 0583 (katmanın kaynak sistemi).

Web: ertelendi (ADR 0228).

## Bağlam

ADR 0519 projenin kotlarının ne olduğunu ve epokunu kaydetti. `cad.crs.transform` dönüşümün kaydını verir; elipsoit ile ortometrik
arası, iki düşey datum arası ve iki epok arası dönüşüm adıyla reddedilir, çünkü KentOS'ta bunlar için motor yoktu.

Türkiye'de günlük iş tam olarak budur:

- GNSS ölçüsü elipsoit yüksekliği verir;
- pafta ve proje Antalya kotu ister;
- TUSAGA-Aktif koordinatları ölçü anının epokundadır, TUREF ise 2005,0'dadır.

Dosyalar da düşey sistemlerini söyler:

- QGIS ve GDAL bileşik sistemi Shapefile'ın `.prj`'ine OGC'nin `COMPD_CS`'iyle yazar; ArcGIS `PROJCS[…],VERTCS[…]` diye iki tanım
  yazar.
- GeoPackage bileşik sistemi kendi kimliğiyle (kuruluş `NONE`) ve `COMPD_CS` tanımıyla tutar.
- GeoJSON bileşik bir OGC URN'üyle söyler.

Bugün KentOS bunlardan ikisini yanlış okuyordu:

- Bileşik `.prj` “koordinat sistemi okunamadı” diye reddediliyordu.
- Bileşik GeoPackage “sistem bildirmiyor” sayılıp koordinatlar projenin sisteminde kabul ediliyordu.

Sunucu ile masaüstü farkı: sunucu koordinat dönüştürmez. PostGIS'in `ST_Transform`'u kullanılmaz; işleri `kentos-jobrun` masaüstünün
motoruyla çalıştırır. Bu iddia bir testle kanıtlanmalıydı.

## Karar

### 1. Geoit modeli (`crs::geoid`)

- **Izgara:** düğüm ızgarası `NodeGrid` enlem ve boylam ızgarasıdır. Güneybatı düğümü, aralık ve boyutla tanımlanır; değerler
  güneyden satır satır tutulur; değeri olmayan düğüm NaN'dır.
- **Okuma:** çift doğrusaldır, PROJ'un `vgridshift`'i gibi. Dünyayı saran ızgara (aralık × sütun = 360°) son hücreden ilk sütuna
  sarar. Izgaranın dışı ya da değeri olmayan düğüm `OutsideGrid`'dir.
- **GTX:** 40 baytlık büyük uçlu başlığı ve f32 değerleri çekirdekte okunur. 180°'den büyük boylam −180'den sayılır; −88,8888
  değer yok demektir.
- **GeoTIFF:** biçim çekirdeğinin raster okuyucusuyla okunur (`crs_models::read`):
  - düğümler piksellerin ortasıdır (PixelIsArea da PixelIsPoint da; okuyucu alanın köşelerini verir);
  - tek bant geoittir;
  - üç bant hız ızgarasıdır (doğu, kuzey, yukarı mm/yıl). PROJ'un `deformation`'ı yalnız üç bantlıyı okur; iki bantlı ızgara
    reddedilir.
- **Kot:** ortometrik kot H = h − N'dir:
  - kaynağın ortometrik kotu kendi geoidiyle elipsoit yüksekliğine çevrilir (h = H + N);
  - datum kaymasının yer merkezli adımlarından geçer (ADR 0519 §2);
  - hedefin geoidiyle ortometrik olur.
  N, noktanın o yanın datumundaki enlem ve boylamında okunur. Ayak birimli düşey sistem (MSL ft) metreye çevrilerek geçer.
- **Kayıt:** `vgridshift` adımlarıyla yazılır: kaynakta `+multiplier=1`, hedefte `−1`. Yöntem `geoid`'dir. Geoidin doğruluğu
  bilinmediğinden kotların doğruluğu boştur.

### 2. Hız ızgarası

İki ayrı epok `velocity` ile (adı ve SHA-256'sı) verilince noktalar kaynağın datumunda taşınır:

- taşıma `to − from` yıl kadardır;
- hız noktada okunur, doğu, kuzey ve yukarı bileşenleri yer merkezli koordinatlara çevrilip eklenir (PROJ'un `deformation`'ı ve
  `+dt`'si gibi);
- sonra datum kayması uygulanır;
- kayıtta `cart`, `deformation`, `inv cart` adımları vardır; `epochApplied` doğrudur;
- hız ızgarasının doğruluğu bilinmediğinden yatay doğruluk da boştur.

Kotları kenarda kalan bir dönüşümde (aynı düşey sistem, bilinmeyen yükseklik) kot değişmez. Pipeline da onu `push`/`pop` ile
saklar.

Retler:

- velocity yoksa `epoch_unsupported`;
- ızgara bu cihazda yoksa `velocity_missing`;
- özet geçersizse `invalid_epoch`.

### 3. Izgaralar cihazın kitaplığında

Geoit modelleri ve hız ızgaraları NTv2 ızgaralarının yanında tutulur:

- yer `$XDG_DATA_HOME/kentos-cad/izgara/`'dir; `KENTOS_GRID_DIR` önce gelir (sunucu ve testler);
- dosya `<sha>.gtx` ya da `<sha>.tif` adıyla, türü notunda (`<sha>.json`) tutulur.

Masaüstünde:

- Proje ayarları › Izgaralar'ın Ekle…'si üçünü de alır; `.gsb` her zaman NTv2'dir.
- Kotlar ve epok'ta geoit modeli kitaplıktan seçilir: adı ve SHA-256'sı birlikte gelir.

Her ev sahibi (masaüstü, başsız sunucu, Python, MCP) adı geçen ızgarayı bu klasörden okur (`crs_models::load`). Başka türden bir
ızgarayı (geoit yerine hız) okumaz.

Retler:

- geoit adı geçmezse `geoid_required`;
- ızgara bu cihazda yoksa ya da başka türdense `geoid_missing`;
- iki düşey datumdan birinin geoidi yoksa `vertical_unsupported`.

### 4. Dosyaların düşey sistemi

Biçim çekirdeğinin `vertical::vertical_of`'u bir koordinat sistemi bildiriminin düşey yarısını okur: adını ve bildirim veriyorsa
EPSG kodunu. Okuduğu bildirimler:

- WKT 1: `COMPD_CS`'in `VERT_CS`'i, ESRI'nin `VERTCS`'i;
- WKT 2: `VERTCRS`, `VERTICALCRS`;
- PROJJSON: `VerticalCRS`, `CompoundCRS`'in bileşeni;
- OGC bileşik URN ve `EPSG:a+b`.

Addan kod tahmin edilmez.

- **Shapefile:**
  - `.prj` `COMPD_CS`'in yatay yarısıyla okunur;
  - ESRI'nin iki tanımlı biçiminde ilk tanım yatay sistemdir;
  - düşey sistem bildirilen sistemin `vertical`'ıdır ve rapordaki “Düşey sistem (.prj)” olgusudur.
- **GeoJSON:** bileşik URN'ün ilk bileşeni yatay sistemdir, ikincisi düşey.
- **GeoPackage:** kuruluşu EPSG olmayan, tanımı `COMPD_CS` olan sistemin yatay EPSG kodu tanımdan okunur; düşey sistem tablo
  sisteminin yanında tutulur (`Read.verticals`).

İçe aktarma komutları düşey sistemi uyarıyla söyler (`vertical_crs`). Kotlar olduğu gibi alınır:

- proje aynı düşey sistemdeyse “projenin kotları da öyle” denir;
- değilse projenin kotları söylenir ve kotların çevrilmediği, çevirmek için Koordinat dönüştür'ün geoit modeliyle kullanılacağı
  eklenir.

`DeclaredCrs.vertical` bildirilen sistemle birlikte çıktıdadır.

### 5. GNSS içe aktar

Projenin kotları ortometrikse ve geoidi bu cihazdaysa GNSS'in elipsoit yüksekliği önce projenin datumuna kaydırılır, sonra geoitle
ortometrik kota çevrilir (H = h − N). Geoidi yoksa kot eskisi gibi alınmaz ve neden söylenir.

### 6. Sunucu ile masaüstü

Sunucu koordinat dönüştürmez; işlerini `kentos-jobrun` masaüstünün motoruyla yapar. `kentos-api`'nin `job_run` testi bunu sınar:
Koordinat sistemine dönüştür sunucu işi olarak çalışır ve parsellerin köşeleri masaüstü motorunun `transform_in`'iyle bit bit aynı
çıkar. Belgeli tolerans sıfırdır.

### 7. EPSG'nin kullanım alanı

PROJ 9.7.1'in veritabanında EPSG:5261, 1784, 1783 ve 5260'ın Türkiye alanının güneyi 34,42°'dir (eskiden 34,2°). Çekirdeğin
`TURKIYE` sabiti ve ADR 0519'un başvurusu buna getirildi.

## Doğrulama

- **`scripts/fixtures/geoid_cases.py`:**
  - KentOS kodu olmadan, GDAL'ın ve kendi `struct`'ının yazdığı ızgaralarla (`fixtures/geoid/v1`): Türkiye geoidi GTX (değeri
    olmayan köşesiyle), aynısı GeoTIFF'in piksel alanı ve piksel noktası, küresel 1° geoit, üç bantlı hız ızgarası;
  - 25 `cad.crs.transform` durumu;
  - her sayı PROJ'dan: kaydın pipeline'ı, ızgaralar SHA-256 adlarıyla;
  - ve ikinci yoldan: PROJ'un 3B işlemi, betiğin kendi çift doğrusal N'i ve hızın kendi yer merkezli toplamı (1e-6 m, 1e-10°).
- **`scripts/fixtures/crs_vertical_cases.py`:** ADR 0519'un 28 durumu yeni retlerle.
- **`scripts/fixtures/vertical_import_cases.py`:** 21 içe aktarma durumu; GDAL'ın yazdığı dosyalar (`fixtures/vertical/v1`), düşey
  sistem GDAL'ın OSR'ıyla.
- **Rust ve Python:** iki çalıştırıcı da 53 + 21 durumu geçer; `pnpm py:test` 259 test.
- **Çekirdek:** `crs::geoid`'in GTX, sarma ve değer yok testleri, biçim çekirdeğinde `vertical`'ın bildirimleri.
- **Masaüstü:**
  - `project::heights`'ta kitaplıktan geoit ve Kotlar ve epok'un resimleri (`.run/shots/kotlar-epok-kitapliktan-*`);
  - `grids`;
  - `kentos-interaction`'ın GNSS testi (30 m'lik geoitle 970,0000193 m).
- **Sunucu:** `cargo test -p kentos-api --test job_run a_transformation`.
- **Süreler** (release, bu makine; `cargo test --release -p kentos-geometry-core --lib crs::geoid::tests::timing -- --ignored
  --nocapture`):

  | İş | Süre |
  |---|---|
  | Küresel 2,5′ geoidin (EGM2008'in aralığı, 37 milyon düğüm, 142 MB) GTX'ten okunuşu | 65 ms |
  | 100 000 nokta, TUREF / TM36 elipsoit → elipsoit | 85,1 ms |
  | 100 000 nokta, elipsoit → ortometrik (geoitle) | 87,5 ms |
  | 100 000 nokta, hız ızgarasıyla 21,5 yıl | 122,5 ms |

  Geoit nokta başına 25 ns ekler; hız, yer merkezli gidiş-dönüşüyle 0,37 µs.

## Sonuçlar

- `CBS-08` kapanır:
  - geoit ve hız ızgarasıyla dönüşüm motoru vardır;
  - Shapefile, GeoJSON ve GeoPackage'ın bileşik ve düşey sistemi okunur ve söylenir;
  - sunucu ile masaüstü farkı sıfır olarak sınanır.
- Açık kalanlar ayrı madde olur:
  - zamana bağlı Helmert (ITRF çerçeveleri arası 14 parametre);
  - FlatGeobuf, GeoParquet ve GML'in düşey sistemi;
  - Koordinat dönüştür penceresinde kotlar ve epok;
  - web (TODOS.md §0.2).
