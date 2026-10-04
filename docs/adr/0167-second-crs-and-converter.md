# ADR 0167: İkinci koordinat sistemi ve koordinat dönüştürücü

- **Durum:** kabul edildi (2026-10-04). `HYB-11`. Ayrıntılar bu ADR'nin varsayılanlarıdır; resmî dönüşüm parametreleri sahibin kararıdır (§7).
- **Tarih:** 2026-10-04
- **Bağlam belgesi:** TODOS.md `HYB-11` (ilgili: `NUM-05`, `NUM-07`, `HYB-12`, `HYB-14`), ADR 0046 (GeoJSON ve Shapefile: dosyanın sistemi gösterilir, dönüştürülmez), ADR 0165 §2–§4 (yerel sistem, eksen ve açı düzeni, ileri TM), ADR 0164 (paftanın kuzey sapması, `kentos-sheet`'in geri TM'i), CLAUDE.md §5 ve §23; Netcad XYZ Sor ve Alan (2. projeksiyon), ArcGIS Pro Coordinate Conversion ve Convert Coordinate Notation, QGIS koordinat gösterimi.

## Bağlam

KentOS'ta her proje bir koordinat sistemi taşır (yerel proje SRID 0). Kayıtta (`geo/crs.ts`, masaüstünde `kentos_project::crs`) TUREF ve ED50 TM3 dilimleri, ED50 ve WGS 84 UTM dilimleri, TUREF, ED50 ve WGS 84 coğrafi sistemleri ve WGS 84 / Pseudo-Mercator vardır. Çekirdekte ileri TM vardır (`geodesy::tm_forward`, Karney 2011, PROJ başvurusuyla); geri TM yalnız paftanın kuzey sapması için, üçüncü dereceden (`kentos-sheet`). Sistemler arası dönüşüm yoktur: içe aktarma dosyanın sistemini gösterir, projeninkinden başkaysa kapalıdır (ADR 0046).

Ölçmecinin günlük işi iki sistemle çalışmaktır: TUREF projede eski ED50 paftalarının koordinatları, saha GNSS'inin coğrafi koordinatları, komşu dilim. Netcad durum çubuğunda ve XYZ Sor'da “2. projeksiyon” değerlerini, Alan'da ikinci sistemin alanını verir; ArcGIS Coordinate Conversion tek nokta ve liste dönüştürür.

Dönüşümün doğruluğu kuraldır (CLAUDE.md §23): dayanağı kayıtlı olmalı, sessiz ve düşük doğruluklu yol olmamalıdır. EPSG'nin Türkiye için kayıtlı dönüşümleri:

| Dönüşüm | EPSG | Yöntem | Doğruluk |
|---|---|---|---|
| TUREF → WGS 84 (1) | 5261 | sıfır dönüşüm | 1 m |
| ED50 → WGS 84 (30) | 1784 | konum vektörü (Helmert, 7 parametre): tx −84,1 m, ty −101,8 m, tz −129,7 m, rx 0, ry 0, rz +0,468″, ds +1,05 ppm | 2 m |
| ED50 → ETRS89 (9) | 1783 | aynı yedi parametre | 2 m |
| TUREF → ETRS89 (1) | 5260 | konum vektörü: tx 0,023, ty 0,036, tz −0,068 m, rx 0,00176″, ry 0,00912″, rz −0,01136″, ds 0,00439 ppm | 0,1 m |

PROJ (9.7) ED50 → TUREF için varsayılan olarak ETRS89 üstünden 2,1 m'lik yolu (EPSG:1783 ve EPSG:5260'ın tersi) seçer. TKGM ve HGK'nin ED50–TUREF dönüşümü (bölgesel parametreler, ızgara) santimetre düzeyindedir ve EPSG'de yoktur.

## Karar

### 1. İkinci sistem projenin ayarıdır

- **Proje ayarı** `secondSrid` (`ProjectSettings::second_srid`, isteğe bağlı; yoksa ikinci sistem yoktur), `.kcad` şema 12. Netcad'deki gibi projeyle saklanır ve paylaşılır: TM30 projesinin ikinci sistemi ED50 TM30, komşu projeninki başka dilim olabilir.
- Proje ayarları penceresinde “İkinci koordinat sistemi” alanı (kayıttaki sistemler ve “Yok”); durum çubuğunun koordinat sistemi hücresinin menüsünde “İkinci sistem ▸”.
- Yerel projede (SRID 0) ikinci sistem yoktur; kayıtta olmayan SRID'ler seçilemez. İkinci sistem projeninkiyle aynı olamaz.
- Coğrafi ikinci sistemin yazılışı kullanıcı tercihidir: `display.geographic` = `dms` (varsayılan: 40°45′12.3456″K 29°55′01.2345″D) ya da `dd` (40.7534293°K 29.9170096°D); uygulamanın noktalı ondalığıyla, enlem önce; basamaklar projenin açı basamaklarından değil, sabit: DMS saniyenin 4 basamağı (≈3 mm), DD derecenin 7 basamağı (≈1 cm).

### 2. Nerede görünür

- **Durum çubuğu:** imlecin koordinatlarının yanında ikinci sistemin değerleri, sistemin kısa adıyla (“ED50 TM30 Y … X …”, “WGS 84 40°… 29°…”). Değerler iletinin yerini paylaşır (4 Ekim, 2. adımda): ikinci sistem varken öbür hücreler o yer değerler kadar olana dek çekilir; ileti yokken değerler, ikisine yer yetmezse ileti gösterildiği birkaç saniye onların yerinde. Basamaklar iletiye bağlı değildir, çubuk kıpırdamaz. 1440 piksellik pencerede görünür, 1100'de çekilir.
- **Koordinat oku:** tıklanan noktanın ikinci sistemdeki değerleri de satır olarak.
- **Mesafe ölç ve Alan hesapla:** ikinci sistem projeksiyonluysa (TM, UTM) uzunluk ve alan ikinci sistemin düzleminde de verilir: köşeler (eğri ve elips 0,1 mm'lik açık sınırla, ADR 0149) dönüştürülür, düzlemde ölçülür. Coğrafi ikinci sistemde uzunluk ve alan verilmez (elipsoit üstü ölçüler `HYB-14`'tür).
- **Dönüştürücü** (§4).

### 3. Dönüşüm zinciri

Çekirdek (`geodesy`) sistemleri kayıttaki tanımlarıyla alır (projeksiyon parametreleri ve datum), kayıt iki platformda ortak durumla sınanır:

- **Projeksiyon:** TM ve UTM ileri (var olan, 6. derece) ve geri (yeni, Karney 2011'in 6. derece serisi); Pseudo-Mercator küresel formülüyle.
- **Datum:** coğrafi → yer merkezli (elipsoidin kendisiyle, yükseklik 0) → Helmert (konum vektörü) → yer merkezli → coğrafi. TUREF ile WGS 84 arasında sıfır dönüşüm (EPSG:5261), ED50 ile WGS 84 arasında EPSG:1784; ED50 ile TUREF arasında PROJ'un (ve QGIS'in) varsayılan yolu: ED50 → ETRS89 (9) (EPSG:1783, aynı yedi parametre, 2 m) ve TUREF → ETRS89 (1)'in tersi (EPSG:5260, yedi parametreli Helmert: tx 0,023, ty 0,036, tz −0,068 m, rx 0,00176″, ry 0,00912″, rz −0,01136″, ds 0,00439 ppm; 0,1 m), toplam 2,1 m. Kullanıcı değerleri QGIS'tekilerle karşılaştırabilsin diye her çift PROJ'un varsayılan yolunu izler; ED50 → WGS 84 ile ED50 → TUREF bu yüzden santimetrelerle ayrışır (TUREF ile WGS 84 arası sıfır dönüşümdür).
- Her dönüşüm sonucu doğruluğunu ve dayanağını taşır (“±2 m, EPSG:1784”); aynı datumda yalnız projeksiyon değişiyorsa “kesin, yalnız projeksiyon”.
- Yükseklik ve epoch yoktur (2B, yükseklik 0 alınır: yatayda 1 mm'nin altında etkisi); `NUM-07`'nin yükseklik ve epoch senaryoları ayrı iştir.

### 4. Dönüştürücü

- **Koordinat dönüştür** penceresi, komutu `crs.transform` (bugün bekleyen): kaynak ve hedef sistem (varsayılan: proje ve ikinci sistem), tek nokta ve liste.
- **Tek nokta:** projeksiyonlu sistemde türün eksen sırasıyla iki değer (CBS'de Y, X; CAD'de X, Y); coğrafide enlem ve boylam, DD ya da DMS (`40 45 12.3456`, `40°45'12.3456"`, `40.75342930` kabul edilir). Sonuç hedefin yazılışıyla, kopyalanabilir; doğruluk ve dayanak altında. “Çizimden seç” ile noktayı çizimden alır (proje sistemindeyse).
- **Liste:** satır satır yapıştırılır (`ad Y X`, virgül, sekme ya da boşlukla; Hesap pencerelerinin tablosu gibi), dönüştürülür, panoya ya da dosyaya (CSV) verilir; okunamayan satır söylenir.
- Dönüştürücü çizimi değiştirmez; dönüştürülmüş noktaları çizime yazmak ayrı karardır.

### 5. Doğruluğun sunulması

- Datum değişen her yerde (durum çubuğu ipucu, Koordinat oku, dönüştürücü) doğruluk yazılır. ED50 değerlerinin yanında “±2 m, resmî dönüşüm değil” ipucu vardır.
- Sessiz yol yoktur: tanımı olmayan bir çift için değer yazılmaz, nedeni söylenir.

### 6. Ortak çekirdek ve başvuru

- `geodesy::tm_inverse` (6. derece), `to_geocentric`, `from_geocentric`, `helmert_position_vector`, `mercator_forward/inverse`, `transform(from, to, p) -> Option<Transformed>` (nokta, doğruluk, dayanak), DMS yazılışı ve okunuşu; çağrılar `crsTransform`, `formatDms`, `parseAngleDms`.
- **Bağımsız başvuru:** `scripts/fixtures/crs_transform_cases.py`, PROJ (pyproj) ile, KentOS kodu olmadan: her sistem çifti için kayıttaki tanımlar ve EPSG işlemleri; dilim ortası, dilim kenarı, uzak doğu ve batı, kuzey ve güney sınırları; noktalar 1 mm içinde (projeksiyon zincirlerinde 1e-6 m); DMS yazılışı ve okunuşu kesin metinle.
- Kayıt iki platformda aynıdır: `fixtures/crs/v1/registry.json` (SRID, ad, tür, datum, elipsoit, parametreler) web ve masaüstü kayıtlarıyla karşılaştırılır.

### 7. Kapsam dışı ve sahibin kararları

- **[M] Resmî ED50–TUREF dönüşümü:** TKGM/HGK parametreleri ya da ızgarası (NTv2) `HYB-12`'nin işidir; sahip verir. O gelince §3'ün zinciri onu kullanır, doğruluk satırı değişir.
- Yükseklik, geoit ve epoch (`NUM-07`), elipsoit ve zemin ölçüleri (`HYB-14`), özel sistem tanımı (`HYB-12`).
- Dosyaların içe aktarmada dönüştürülmesi (ADR 0046'nın kararı sürer).

### 8. İş sırası

1. Çekirdek: geri TM, Pseudo-Mercator, yer merkezli dönüşümler ve Helmert, zincir, DMS; WASM çağrıları; bağımsız başvuru ve ortak durumlar; ortak kayıt fixture'ı.

   *(4 Ekim: tamam.)*
   - **`geodesy::tm_inverse`:** Karney'in 6. derece serisi, uyumlu enlemin tanjantı Newton'la (ileri TM'in eşi).
   - **`crs`:** `Datum` (elipsoidiyle), `System` (`geographic`, `tm`, `mercator`; kayıttaki girdinin adsız hâli), `transform(from, to, p) -> Option<Transformed>` (nokta, doğruluk, dayanak); datum adımları yer merkezli koordinatlarda PROJ'un küçük açılı konum vektörü Helmert'iyle ve tersiyle; `format_dms`, `format_dd`, `parse_angle`. Çağrılar `crsTransform`, `formatDms`, `formatDd`, `parseAngle`; web `model/geom/crsTransform.ts`.
   - **Kayıttan sistem:** web `systemOf`, masaüstü `kentos_project::crs::System::transform_system`; ikisi başvurudaki sistemlerle sınanır.
   - **Başvuru:** `scripts/fixtures/crs_transform_cases.py` (`fixtures/geodesy/v1/transform.json`, PROJ 9.7.1): 18 sistem çiftinde 40 dönüşüm (TM, UTM, coğrafi, Pseudo-Mercator; dilim kenarları; üç datum yolu), 48 yazılış (DMS 2 ve 4 basamak, DD 7 basamak; taşma, işaret), 19 okunuş. Çekirdek (yerli ve çağrı tablosu) ve web (WASM): ızgara noktaları 1e-6 m, enlem ve boylam 1e-11° içinde; ilk ölçümde fark nanometre düzeyindedir.
2. Proje ayarı `secondSrid` ve `.kcad` şema 12 (spesifikasyon, kodek, Python okuyucu ve yazıcı, örnek dosyalar); `display.geographic`; Proje ayarları ve durum çubuğu hücresinin menüsü; durum çubuğunda ikinci değerler; Koordinat oku. İki platformda, resimleriyle.

   *(4 Ekim: tamam.)*
   - **Sözleşme:** `ProjectSettings::second_srid` (`secondSrid`, yalnız varken yazılır) ve `second()`: projeninkinden başka bir sistem, yerel projede yok. Ayarlar kurulurken (masaüstü `Document::set_settings`, açılış ve dışarıdan gelen ayarlar; web `ProjectSettings.assign`) geçersiz ikinci sistem düşer; sunucunun bütün ayarlarında olmayan ikinci sistem yoktur (web'de çizim birimi de böyle).
   - **`.kcad` şema 12:** spesifikasyon §6.1 ve §6.4; Rust kodeği, `tools/kcad/kcad.py`, bağımsız yazıcı; `second-crs.kcad` ve bozuk dosyalar: şema 11'de `secondSrid` (`unknown_field`), 0, projeninkiyle aynı ve yerel projede (`bad_value`). Yazıcı geçersiz ikinci sistemi yazmaz, `bad_value` ile reddeder. Web'in v1 ve v2 başı da aynı kuralla okur; kayıtta olmayan sistem korunur, değerleri gösterilmez.
   - **`display.geographic`:** ayar şemasında yeni “Gösterim” grubu; Uygulama ayarları › Görünüm › Koordinatlar ve İkinci sistem menüsünün Coğrafi değerler'i.
   - **Okunuş:** masaüstü `kentos_interaction::second::Second`, web `model/secondCrs.ts` (3. adımda araçların altındaki katmana taşındı; biçimlendiriciyi yapısal bir arayüzle alır): kısa ad (“ED50 TM30”), değerler (projeksiyonluda türün eksen adları ve uzunluk basamakları, coğrafide enlem önce), doğruluk (“±2.1 m, EPSG:1783 + EPSG:5260; resmî dönüşüm değil”, “±1 m, EPSG:5261”, “kesin, yalnız projeksiyon”).
   - **Durum çubuğu:** §2'nin kuralı; hücrenin ipucu doğruluğu söyler, tıklamak İkinci sistem listesini açar. Masaüstünün genişlik tahmininde çizim yardımcılarının lambası, aralığı ve dolgusu ölçülerek düzeltildi (önceden eksik sayılan ~60 piksel iletinin payını yiyordu); iletinin en az yeri masaüstünde 10 harftir, ikinci sistem yokken çubuk eskisi gibidir.
   - **Menüler:** koordinat sistemi hücresinin ve sekme satırındaki sistem düğmesinin sağ tık menüsü: Koordinat sistemi…, İkinci sistem ▸ (Yok, datum başına alt menü, seçili olan yanında; Coğrafi değerler); yeni simge `crsSecond` (küre ve küçük 2).
   - **Proje ayarları › Koordinat sistemi:** “İkinci koordinat sistemi” alanı (kayıttaki sistemler datumlarıyla ve Yok; yerel projede yok); Kaydet söyler.
   - **Koordinat oku:** ikinci satır “ED50 TM30: Y=…, X=… (±2.1 m, …)”, etikette sistemin adı ve iki değeri; dışarıda kalan nokta için uyarı. Ortak iz `fixtures/interaction/v1/second-crs.json` (kesin metinler, PROJ'un değerleri) iki platformda geçer.
   - **Resimler:** masaüstü `second_crs::tests::screens` (`.run/shots/ikinci-sistem-*`), web `shots.mjs secondcrs`.
3. Mesafe ölç ve Alan hesapla'nın ikinci sistem değerleri; ortak iz.

   *(4 Ekim: tamam.)*
   - **Çekirdek:** `crs::measure::plane_measures(from, to, rings, closed)`: yol ya da alanın halkaları (dıştaki önce, sonrakiler delik) projenin sisteminde verilir, ikinci sistemin düzleminde ölçülür; düz kenarların uçları ve yaylar 0,1 mm'lik sehimli düz parçalar olarak (`n = max(1, ⌈|θ| / (2·acos(1 − 0,0001/r))⌉)`, `arc_pieces`) taşınır; uzunluk parçaların toplamı, alan her halkanın ilk noktasına göre ayakkabı bağı alanı, delikler düşülür. Coğrafi sistemde ve Pseudo-Mercator'da ölçü yoktur (`NoPlane`), ulaşılamayan nokta da söylenir. Çağrı `crsPlaneMeasures`; web `model/geom/crsTransform.ts`.
   - **Başvuru:** `scripts/fixtures/crs_measure_cases.py` (`fixtures/geodesy/v1/measure.json`, PROJ): beş şekil (açık yol, yaylı yol, 40 × 25 m'lik alan, yaylı kenarlı ve delikli alan, 1 km'lik parsel) beş ikinci sistemde, 25 ölçü (TM, UTM, komşu dilim, coğrafi ve Pseudo-Mercator redleri) ve yayların parça sayıları. Yerli ve WASM: uzunluk 1e-6 m, alan her 100 m çevre için 1e-6 m² içinde (noktalar PROJ'la nanometre içinde uyuşur, alan çevresi kadar oynar).
   - **Araçlar:** Mesafe ölç'ün toplamı ve Alan hesapla'nın (köşeleriyle ya da İçine tıkla ile, delikleriyle) alanı ve çevresi kendi satırlarından sonra “ED50 TM30 düzleminde: …” satırıyla; Sabit ilk nokta'nın her ışını “(ED50 TM30 düzleminde …)” ekiyle; coğrafi ve Pseudo-Mercator ikinci sistemde nedeni söylenir. Masaüstü `path.rs` (`say_second`), web `pathTool.ts` (`saySecondMeasures`) ve `measureTools.ts`.
   - **Ortak iz:** `second-crs.json`'a Mesafe ölç (R1'den çizginin ucuna) ve Alan hesapla (yazılan köşelerle 40 × 25 m) adımları, kesin metinlerle (PROJ'un değerleri); kullanım senaryosu `usage-second-crs.json` iki platformda resimlerle.
4. Koordinat dönüştür penceresi (tek nokta, liste, çizimden seç), `crs.transform`; iki platformda, resimleriyle.

Her adım iki platformda, ortak fixture'larla, kendi commit'inde ilerler.

## Sonuçlar

- Ölçmeci TUREF projede ED50 ve coğrafi değerleri imlecin altında görür, tek nokta ve liste dönüştürür; Netcad'in 2. projeksiyonu ve ArcGIS'in Coordinate Conversion'ı karşılanır.
- Doğruluk her yerde yazılıdır; datum dönüşümü bugün EPSG'nin 1–2 m'lik dönüşümleridir, kadastro için resmî parametre beklenir.
- `.kcad` şema 12 bir alan ekler; eski dosyalar ikinci sistemsiz açılır.

## Doğrulama

- **Çekirdek:** PROJ başvurusuna göre iki platformda (yerli ve WASM).
- **Arayüz:** ortak izlerle ve resimlerle iki platformda; durum çubuğu 1100×650'de de sığar.
