# ADR 0519: Düşey sistem, epok ve dönüşümün kaydı

- **Durum:** kabul edildi (2026-10-19). `plan/gis.md` CBS-08'in G1 dilimi (P0 metadata). Bu dilim üç şeyi sağlar:
  - projenin kotlarının ne olduğu (elipsoit ya da ortometrik, düşey sistem, geoit modeli) ve koordinat epoku projeyle birlikte
    saklanır, komutla atanır, hiçbir kotu değiştirmez;
  - nokta dönüşümü bir ürün komutudur (`cad.crs.transform`) ve ne yaptığını kayıtla söyler: işlemin kimliği ve adımları, aynı işi
    yapan PROJ pipeline'ı, motor, ızgaralar (adı ve SHA-256), kullanım alanı, belirtilen doğruluk;
  - KentOS'un motoru olmayan düşey ve zamana bağlı dönüşümler adıyla reddedilir; daha zayıf bir yola sessizce geçilmez.
- **Bağlam belgesi:** ADR 0167 (ikinci sistem ve dönüştürücü), 0168 (özel sistem, datum seçimleri, NTv2), 0169 §6 (GNSS içe aktar),
  0171 (zemin, elipsoit ve düzlem), 0253 (komut protokolü), 0265 (şema değişikliğinin kuralı), 0228 (web'e ara).
- **Web:** ertelendi (ADR 0228). Web'de gerekenler TODOS.md §0.2'de.
- **Sonraya kalan:** geoit ızgarası okuyan motor (GTX/GeoTIFF; elipsoit ↔ ortometrik, P1), düşey datumlar arası ve zamana bağlı
  (hız modeli, ITRF epokları) dönüşüm (P1/P2), GeoPackage, Shapefile ve GeoJSON'un bileşik ve düşey sisteminin okunup içe aktarma
  raporunda söylenmesi, Koordinat dönüştür penceresinin kotlarla ve kayıtla çalışması, sunucu ile masaüstü sonuç farkının sınanması.

## Bağlam

Dönüşümler 2B'dir (ADR 0167 §3): yükseklik 0 kabul edilir, kotlar dönüşümde yoktur. GNSS içe aktar kotu WGS 84 elipsoit yüksekliği
olarak yazar; ED50 projesinde bu yükseklik projenin elipsoidinin değil, ortometrik kot bekleyen projede de kot değildir. Projenin
kotlarının ne olduğu hiçbir yerde yazmıyordu (ADR 0171 §2: “ortometrik mi elipsoit mi bilinmiyor”). Dönüşümün yolu bir metindir
(“EPSG:1783 + EPSG:5260”); hangi parametrelerin, hangi ızgaranın, hangi alanda geçerli olduğu kayıtta yoktur.

PROJ kendisi Antalya kotundan EGM2008'e, ızgara yoksa yalnız “ballpark” bir düşey dönüşüm önerir: kotlar olduğu gibi kalır, doğruluk
bilinmez. Bu, planın “sessiz fallback yok” kuralının tam karşılığıdır; bağımsız başvuru bunu sınar.

## Karar

### 1. Projenin kotları ve epoku (`.kcad` şema 85)

- `ProjectSettings.heights`: `HeightReference` = `kind` (`ellipsoidal`, `orthometric`; girdide `unknown` “söylenmedi” demektir ve
  saklanmaz), yalnız ortometrikte `verticalSrid` ve `geoid` (`name`, `sha256`). `coordinateEpoch`: 1900–2100 arası ondalık yıl.
- KentOS'un bildiği düşey sistemler `kentos_contracts::VERTICAL_SYSTEMS`'tir, EPSG kaydından: Antalya height 5775, EGM2008 height
  3855, EGM96 height 5773, MSL height 5714, MSL height (ft) 8050, Black Sea height 5735 (adı, datumu, birimi ve kullanım alanı
  PROJ'un veritabanıyla denetlenir). Kotların birimi düşey sistemin birimidir.
- Elipsoit yüksekliği ve epok bir datuma bağlıdır: sistemsiz yerel projede olmaz (`heights_need_system`); sistem kalkınca onlar da
  kalkar. Ortometrik kot yerel projede de tanımlanır.
- `cad.settings.set`'in `heights` ve `coordinateEpoch`'u (`{kind: unknown}` ve 0 kaldırır); `invalid_heights`, `invalid_epoch`.
  Atamadır: hiçbir kot değişmez.
- Şema 85 yalnız proje ikisinden birini anarken yazılır (ADR 0265'in altı adımı: örnek `heights.kcad`, şema 84'te bilinmeyen alan,
  altı `bad_value` örneği, bağımsız Python okuyucu ve yazıcısı; `FORMATS_VERSION` 92).

### 2. Çekirdek: kotla dönüşüm ve kayıt (`crs::operation`)

- `transform_height_in`: elipsoit yüksekliği datum kaymasının yer merkezli adımlarından geçer (PROJ'un 3B pipeline'ları gibi); boş
  dönüşüm (TUREF–WGS 84) ve NTv2 ızgarası onu olduğu gibi bırakır.
- `operation`: adımlar (izdüşüm, Helmert, boş, ızgara; adı, tersi, doğruluğu), aynı işi yapan PROJ pipeline'ı (`+proj=pipeline`,
  EPSG parametreleriyle; kot elipsoit değilse datum kayması yüksekliği 0'da yapar ve kotu `push`/`set`/`pop` ile kenarda tutar),
  ızgaraların kimliği (SHA-256, dosya `<sha>.gsb`), kullanım alanı (EPSG adımlarının Türkiye alanı), toplam doğruluk.
- Motorun adı ve kural sürümü `KentOS geometry-core (kentos.crs/1)`.

### 3. Komut: `cad.crs.transform` (sorgu, yazmaz)

- Taraf: `srid` (kayıttaki EPSG kodu; yoksa projenin sistemi, projenin kotları ve epokuyla), `heights`, `epoch`. Noktalar tarafın
  biriminde, `z` kot. `uncertainty`: kaynağın yatay ve düşey belirsizliği.
- Kotlar: ikisi de elipsoit → datumla taşınır; aynı düşey sistem → kalır; aynı datumun başka birimi → çevrilir (MSL m ↔ ft); bir yan
  söylenmiş öbürü söylenmemiş → `heights_unknown`; elipsoit ↔ ortometrik → `geoid_required`; iki düşey datum →
  `vertical_unsupported`; iki yan da söylenmemiş → kotlar kalır, `heights_unchanged` uyarısı. Kotsuz noktada yükseklik kuralı işlemez.
- Epok: iki ayrı epok → `epoch_unsupported`; verilen epok kayıtta kalır ama uygulanmaz (`epoch_not_applied`); hedefin epoku
  yoksa kaynağınki taşınır.
- Datum yolu: projenin ızgarası bu cihazda yoksa `grid_missing` (EPSG'nin daha düşük doğruluklu yoluna kendiliğinden geçilmez);
  WGS 84'e yolu olmayan proje datumu `no_datum_link`. ED50'nin EPSG işlemi `unofficial_operation` uyarısıdır. Değeri olmayan nokta
  (izdüşümün ya da ızgaranın dışı) boş döner, `points_outside`.

### 4. Raporda ayrı tutulan sayılar

`report`: kaynağın belirsizliği (verilen), dönüşümün belirtilen doğruluğu (yatay ve kot için), ikisinin karelerinin toplamının kökü
(tahmin; mm'ye), projenin topoloji toleransı (köşelerin bir sayıldığı uzaklık, hata değil), gösterim basamakları ve adımı (doğruluk
değil). Biri öbürünün yerine geçmez.

### 5. Masaüstü

Proje ayarları › Kotlar ve epok: Yükseklik türü, Düşey sistem, Geoit modeli (adı, SHA-256), Koordinat epoku; uymayan değer söylenir
ve Kaydet bekler; Kaydet `cad.settings.set` ile yazar. İkinci sistem kayıttaysa altında ona dönüşümün kaydı (`cad.crs.transform`'un
cevabı, çizimin orijininde): işlem, adımlar, kullanım alanı, motor, PROJ pipeline'ı ve ayrı tutulan sayılar.

### 6. GNSS içe aktar

Kot projenin söylediğine göre olur: söylenmemişse eskisi gibi WGS 84 elipsoit yüksekliği; elipsoitse yükseklik WGS 84'ten projenin
datumuna kaydırılır; ortometrikse kot alınmaz (elipsoit yüksekliği öznitelikte kalır) ve pencere nedenini söyler.

## Sonuçlar

- Bir kotun ne olduğu projede yazılıdır ve komutlar onu okur; düşey ya da zamana bağlı dönüşüm “santimetre doğruluğu” sözüyle değil,
  açık retle karşılanır.
- Dönüşümün kaydı başka bir yazılımda (PROJ) aynı sonucu verir: bağımsız başvuru her pipeline'ı pyproj'da çalıştırır.
- ED50 projesinde GNSS'in elipsoit yüksekliği artık projenin elipsoidine kaydırılarak yazılır (ancak proje elipsoit dediğinde).

## Doğrulama

- `python3 scripts/fixtures/crs_vertical_cases.py --check` (28 durum): noktalar PROJ'dan (2B'de ADR 0167'nin EPSG işlemleri, kotla
  3B'ye yükseltilmiş sistemler arasında aynı işlem), kaydın pipeline'ı EPSG parametreleriyle kurulur ve `Transformer.from_pipeline`
  aynı cevabı verir (1e-6 m, 1e-10°), birim çevirisi ve aynı düşey sistemde kotun kalması PROJ'un bileşik sistemleriyle, Antalya →
  EGM2008 için PROJ'un yalnız ballpark yolu olduğu, düşey sistemlerin ve EPSG işlemlerinin alanları PROJ'un veritabanıyla. pyproj
  yoksa betik macOS'ta QGIS'in Python'uyla yeniden çalışır.
- `python3 scripts/fixtures/settings_set_command_cases.py --check` (49 durum; 9'u bu ADR'nin).
- `KENTOS_CASES_ONLY=cad.settings.set,cad.crs.transform cargo test -p kentos-native-application --test all every_command_case`,
  `... capabilities`.
- `cargo test -p kentos-kcad --test all` (`heights`, `schemas`, `fixtures`), `python3 scripts/fixtures/kcad_v2_reference.py --check`.
- `cargo test -p kentos-geometry-core --lib crs::operation` (ED50 → WGS 84 3B, PROJ: 1040,3406805936 m).
- `cargo test -p kentos-interaction --test all gnss` (ED50 / TM33 projesinde kot, PROJ: 1000,0000193492 m).
- `.run/py/bin/python -m unittest python/tests/test_crs.py`; `cargo test -p kentos-desktop project::heights`; resimler
  `cargo test -p kentos-desktop project::heights::tests::screens -- --ignored --nocapture` (`.run/shots/kotlar-epok-*`).
