# ADR 0168: Özel koordinat sistemi, yerel sistem ve datum dönüşümleri

- **Durum:** kabul edildi (2026-10-04). `HYB-12`. Ayrıntılar bu ADR'nin varsayılanlarıdır; resmî parametreler ve ızgaralar sahibin kararıdır (§8).
- **Tarih:** 2026-10-04
- **Bağlam belgesi:** TODOS.md `HYB-12` (ilgili: `NUM-04`, `NUM-06`, `NUM-07`, `HYB-11`, `HYB-14`), ADR 0167 (ikinci sistem, dönüşüm zinciri, dönüştürücü), ADR 0165 §2 (koordinat sistemi olmayan proje, SRID 0), ADR 0046 (`.prj` okunur, dönüştürülmez), ADR 0156 (Vektör oturtma'nın çözümü `ops::fit`), CLAUDE.md §5 ve §23; Netcad Projeksiyon Ayarları (datum dönüşümünde “Kullanıcı Tanımlı”, Düzeltme sekmesinin afin parametreleri, yerel projeksiyon), ArcGIS Pro Define a new coordinate system ve Create Custom Geographic Transformation, QGIS Custom Coordinate Reference System ve NTv2; HGK'nin ED50–TUREF çalışması (Aktuğ vd., Harita Dergisi 146, 2011).

## Bağlam

ADR 0167'den beri KentOS kayıttaki sistemler arasında dönüştürür (TUREF ve ED50 TM3 dilimleri, ED50 ve WGS 84 UTM dilimleri, coğrafi sistemler, Pseudo-Mercator); datum adımları EPSG'nin 1–2 m'lik dönüşümleridir. Ölçmecinin üç ihtiyacı karşılanmaz:

1. **Kayıtta olmayan sistem:** başka orta meridyen ya da ölçek, sıfırdan farklı başlangıç enlemi, başka elipsoit (Bessel, Krasovski), bir `.prj`'nin ya da PROJ dizesinin tanımı. Shapefile'ın `.prj`'si bugün yalnız tanınır (`formats::shp::prj`); tanınmazsa adı gösterilir.
2. **Yerel sistem:** şantiye, aplikasyon ya da eski belediye sistemi; ülke sistemine öteleme, dönüklük ve ölçekle (benzerlik) ya da afinle bağlı. Netcad'de yerel projeksiyonun çıktı koordinatları bir afin dönüşümle elde edilir (çıktı TM3 sayılır); Projeksiyon Ayarları'nın Düzeltme sekmesi seçilen projeksiyona afin parametreleri ekler.
3. **Datum dönüşümünün seçimi:** ED50 ile TUREF arasında EPSG'nin yolu ±2,1 m'dir; kadastroda bölgesel parametreler ya da ızgara kullanılır. HGK'nin çalışması (2011) ülke geneli için 7 parametre (±1,1 m) ve 0,13° × 0,10°'lik hücresel düzeltmeler (çapraz doğrulamada ±0,28 m enlem, ±0,36 m boylam) verir; BÖHHBÜY bölgesel dönüşümü ortak noktalarla denetletir. Izgaralar uluslararası NTv2 biçimiyle dağıtılır (PROJ, QGIS ve ArcGIS okur).

§23'ün kuralı: dönüşüm dayanağıyla kayıtlıdır, sessiz düşük doğruluklu yol (PROJ'un parametresiz “ballpark”ı gibi) yoktur, eksik ızgara açık durum verir (`NUM-06`).

## Karar

### 1. Özel sistem projenin tanımıdır

- Proje ayarı **`crs`**: projenin kendi sistemi özelse tanımın kendisi; o zaman `srid` 0'dır. **`secondCrs`**: ikinci sistem özelse (`secondSrid` yerine). Tanımlar projeyle saklanır ve paylaşılır (QGIS'in projeye gömdüğü özel sistem gibi); `.kcad` şema 13.
- Sunucuda özel sistemli projenin nesneleri SRID 0 ile durur (koordinat sistemi olmayan proje gibi); tanım projenin ayarlarındadır. PostGIS'in `spatial_ref_sys`'ine yazılmaz: kurumlar arasında çakışan kod olmaz.
- Tanımın türleri:
  - **TM izdüşümü:** başlangıç enlemi, orta meridyen, ölçek, sağa ve yukarı öteleme; datum (§2).
  - **Coğrafi:** datum (§2).
  - **Yerel (taban sisteme bağlı):** bir taban sistem (kayıttaki projeksiyonlu bir sistem ya da özel TM) ve düzlemde dönüşüm, bu sistemden tabana: benzerlik (sağa ve yukarı öteleme, dönüklük, ölçek) ya da afin (altı katsayı). Katsayılar yazılır ya da ortak noktalardan hesaplanır (Vektör oturtma'nın çözümü `ops::fit`: artıklar, m0, Kullan). Netcad'in yerel projeksiyonu ve Düzeltme sekmesi budur. Koordinat sistemi olmayan projeyle (SRID 0, “Yerel (koordinat sistemi yok)”) karışmasın diye arayüzdeki adı “Yerel (taban sisteme bağlı)”dır.
- Ad zorunludur. Kayıttaki bir sistemle aynı tanım söylenir (“EPSG:5254 ile aynı; kayıttakini seçin”). Birim metre ve derecedir; başka birimli tanım reddedilir, nedeni söylenir. Özel sistemli projenin çizim birimi metredir.

### 2. Datum

- **Kayıttaki datum:** TUREF, ED50 ya da WGS 84; öbür datumlara ADR 0167 §3'ün yollarıyla ya da projenin seçimiyle (§3).
- **Özel datum:** ad, elipsoit (GRS80, WGS 84, International 1924, Bessel 1841, Krasovski 1940, Clarke 1880 (RGS) ya da a ve 1/f) ve WGS 84'e 7 parametre (3 parametrede dönüklükler ve ölçek 0): ötelemeler (m), dönüklükler (″), ölçek farkı (ppm), dönüklüklerin kuralı açıkça (konum vektörü, EPSG 9606; koordinat çerçevesi, EPSG 9607; dönüklüklerin işaretleri terstir) ve doğruluk (m, kullanıcının yazdığı). Özel datumun öbür datumlara yolu WGS 84 üstündendir (PROJ'un `+towgs84`'ü gibi): özel → WGS 84 → hedef (EPSG:5261 ya da EPSG:1784, ya da projenin seçimi); doğruluklar toplanır, dayanakta adlarıyla yazılır.
- WGS 84'e bağı olmayan özel datum kendi içinde kalır: başka datumdaki sisteme değer verilmez, nedeni söylenir (“datumun WGS 84'e dönüşümü yok”).

### 3. Projenin datum dönüşümleri

- Proje ayarı **`datumTransforms`**: kayıttaki bir datum çifti (ED50–TUREF, ED50–WGS 84, TUREF–WGS 84) için EPSG yolunun yerine:
  - **7 parametre:** ad, yön (hangi datumdan hangisine), kural, doğruluk; ya da
  - **NTv2 ızgarası:** ad, SHA-256, boyut; kaynak ve hedef ızgaranın başlığından denetlenir (elipsoit eksenleri çiftin datumlarınınkiyle tutmalı).
- Seçim yalnız o çifti değiştirir; öbür çiftler EPSG'nin yollarıyla kalır. Dayanakta kullanılan yol yazılır (“±0,3 m, ED50 → TUREF: Bölge 7”). Çiftin tersi aynı dönüşümün tersidir.
- Izgaranın dışında kalan noktaya değer verilmez, nedeni söylenir; EPSG yoluna sessizce dönülmez.

### 4. NTv2 ızgaraları

- **Okuma:** NTv2 (`NUM_OREC` 11, `GS_TYPE` `SECONDS`; küçük ve büyük uçlu), alt ızgaralar ve iç içe ızgaralar (noktayı içeren en sık ızgara), enlem ve boylam kayması (saniye; boylam batıya pozitif), kayıtlar güneyden kuzeye satır satır, satırda doğudan batıya; çift doğrusal ara değer; ters yön PROJ'unki gibi yinelemeyle. Bozuk ya da kötücül dosyada panik yoktur: boyut (en çok 256 MiB), alt ızgara sayısı, kayıt sayısının sınırlarla tutarlılığı ve değerlerin sonluluğu denetlenir; hata nedenini söyler.
- **Kitaplık:** ızgaralar cihazdadır (QGIS'in ve ArcGIS'in kullanıcı klasörü gibi): masaüstünde `$XDG_DATA_HOME/kentos-cad/izgara/<sha256>.gsb`, web'de IndexedDB `kentos.grids`. Proje ızgarayı adı, SHA-256'sı ve boyutuyla anar. Cihazda olmayan ızgara açık durumdur: değer yazılmaz, “NTv2 ızgarası bu cihazda yok: … Proje ayarları › Koordinat sistemi › Izgaralar'dan ekleyin.” Izgaranın `.kcad`'e gömülmesi ve bulutta tutulması sonraki karardır (§8).
- **Doğruluk:** dönüşümün yazdığı; yoksa ızgaranın noktadaki doğruluk alanları (saniyeden metreye, enlemin ve boylamın büyüğü); ızgara da vermiyorsa “doğruluğu bilinmiyor”.

### 5. WKT ve PROJ

- **Okuma:** WKT 1 (OGC ve ESRI: `PROJCS`, `GEOGCS`, `TOWGS84`), WKT 2 (`PROJCRS`, `GEOGCRS`, `BASEGEOGCRS`, `CONVERSION`; `BOUNDCRS` ile `ABRIDGEDTRANSFORMATION`), PROJ dizesi (`+proj=tmerc|utm|longlat`, `+lat_0`, `+lon_0`, `+k`, `+x_0`, `+y_0`, `+zone`, `+south`, `+ellps` ya da `+a` ile `+rf` veya `+b`, `+towgs84`, `+datum=WGS84`, `+units=m`); metin yapıştırılır ya da `.prj` seçilir. TM dışı izdüşüm (Lambert, stereografik …) “desteklenmiyor” der, adını söyler. `formats::shp::prj`'nin ayrıştırıcısı çekirdeğe taşınır; Shapefile aynı okuyucuyu kullanır, davranışı değişmez.
- **Yazma:** tanım WKT 1 (OGC, `TOWGS84` ile) ve PROJ dizesi olarak kopyalanır; yerel sistem WKT 2'nin `DERIVEDPROJCRS`'i ve PROJ hattı (`+proj=pipeline … +proj=affine`) olarak.
- Okunan tanım kayıttaki bir sistemse (EPSG kodu ya da parametreleriyle) o önerilir.

### 6. Arayüz

- **Proje ayarları › Koordinat sistemi:** projenin sisteminin ve ikinci sistemin listesinde “Özel sistem…”; tanımı olan proje tanımın adını gösterir, Düzenle ile açar. Yeni gruplar: **Datum dönüşümleri** (üç çift; her biri EPSG (varsayılan), 7 parametre… ya da NTv2 ızgarası…) ve **Izgaralar** (cihazdakiler: ad, kaynak → hedef, kapsam, boyut, SHA-256'nın başı; Ekle…, Kaldır; projenin andığı ama cihazda olmayan uyarıyla).
- **Özel koordinat sistemi** penceresi: Ad; Tür (TM izdüşümü, Coğrafi, Yerel (taban sisteme bağlı)); türün alanları; Datum (kayıttaki ya da özel; özelde elipsoit ve 7 parametre); yerel sistemde taban sistem, benzerlik ya da afin ve “Ortak noktalardan hesapla” (tablo: bu sistemde Y, X; tabanda Y, X; Kullan; artıklar ve m0); WKT ya da PROJ'dan al; WKT ve PROJ olarak kopyala; Deneme noktası (yazılan noktanın WGS 84'te ve projenin sisteminde değeri, doğruluğuyla); Kaydet ve Vazgeç. Kaydet çizimi dönüştürmez, bunu söyler.
- Dönüştürücü, durum çubuğu, Koordinat oku, Mesafe ölç ve Alan hesapla özel sistemleri kayıttakiler gibi kullanır (ADR 0167'nin yerleri).

### 7. Ortak çekirdek ve başvuru

- `crs`: tanım (TM, coğrafi, yerel), özel datum, iki kurallı Helmert, düzlem dönüşümü (benzerlik, afin; tersi), projenin seçimleri ve bunlarla dönüşüm; `crs::wkt` ve `crs::proj` okuma ve yazma; `crs::ntv2` (okuma, ara değer, ileri ve geri). Web'e WASM çağrılarıyla; ızgaranın baytları bir kez yüklenir, SHA-256'sıyla anılır.
- **Bağımsız başvurular:** `scripts/fixtures/crs_custom_cases.py` (PROJ/pyproj'un açık hatlarıyla: özel TM ve coğrafi, iki Helmert kuralı, özel datumdan kayıttakilere, yerel sistem PROJ'un `affine` adımıyla, projenin 7 parametresi), `scripts/fixtures/crs_text_cases.py` (WKT ve PROJ dizelerinden pyproj'un okuduğu parametreler; yazılan metinleri PROJ geri okur) ve `scripts/fixtures/ntv2_cases.py` (KentOS kodu olmadan yazılan sentetik ızgaralar: tek, iç içe, kenarlar, büyük uçlu; PROJ'un `hgridshift`'iyle ileri ve geri; bozuk dosyalar). Izgara noktaları 1e-6 m, enlem ve boylam 1e-11° içinde.

### 8. Kapsam dışı ve sahibin kararları

- **[M] Resmî parametreler ve ızgaralar:** TKGM'nin ve HGK'nin ED50–TUREF parametreleri ya da ızgarası. KentOS kaynağı, kuralı ve lisansı belli olmayan hazır parametre ya da ızgara dağıtmaz; sahip verirse kitaplığa “resmî” olarak eklenir.
- Izgaranın `.kcad`'e gömülmesi ve bulutta tutulması (`NUM-06`'nın kalanı); yükseklik, geoit ve epoch (`NUM-07`); TM dışı izdüşümler; zemin ölçüleri (`HYB-14`).
- Çizimin nesneleriyle bir sistemden öbürüne dönüştürülmesi: bugün Vektör oturtma ve Kauçuk levha var; sistemler arası “çizimi dönüştür” ayrı karardır.

### 9. İş sırası

1. Çekirdek.
   1a. Tanım modeli, özel datum ve iki Helmert kuralı, yerel sistem, projenin 7 parametreli datum dönüşümleri; WASM çağrıları; bağımsız başvuru `crs_custom_cases.py`.

       *(4 Ekim: tamam.)*
       - **Çekirdek:** `crs::Datum` kayıttaki üç datum ve projenin datumu (`CustomDatum`: ad, elipsoit, WGS 84'e 7 parametre; yoksa bağsız); `Helmert` iki kuralla (karşılaştırma ve hesap konum vektörüne çevrilerek); `System::Tm`'de başlangıç enlemi (`latitudeOfOrigin`, yoksa 0: ızgaranın o enlemdeki kuzey değeri düşülür); `System::Local` (taban ve `Plane`: benzerlik ya da afin, PROJ'un `affine` adımının toplama sırasıyla ve tersiyle); projenin seçimleri (`Choice`: kayıttaki iki datum, ad, 7 parametre; tersi aynı seçimdir). Yol (`crs::datum::path`): aynı datumda yok; kayıttaki datumlar arasında seçim ya da EPSG'nin yolu; projenin datumu WGS 84 üstünden. Adımlar PROJ'un hattı gibi işler: ardışık Helmert'ler yer merkezli kalır, boş TUREF–WGS 84 adımından önce ve sonda enlem ve boylama, son adımın vardığı datumun elipsoidiyle dönülür (yükseklik taşınır; WGS 84 yerine GRS80 elipsoidi 0,1 mm fark eder). Yanıt `Transformed { point, accuracy, via, unofficial }`: doğruluk adımların toplamı (milimetreye; biri bilinmiyorsa yok), resmî olmama EPSG'nin bir ED50 işlemi kullanıldıysa; değer yoksa nedeni (`outside`, `noLink`). Çağrılar `crsTransformIn` ve `crsPlaneMeasures`'ın isteğe bağlı seçimleri; web `model/geom/crsTransform.ts`.
       - **Doğruluk metni:** iki platformda çekirdeğin `unofficial`'ından (önceden iki yandan biri ED50 diye bakılırdı); bilinmeyen doğrulukta “doğruluğu bilinmiyor, …”. Kayıttaki sistemlerin metinleri değişmedi.
       - **Başvuru:** `scripts/fixtures/crs_custom_cases.py` (`fixtures/geodesy/v1/custom.json`, PROJ 9.7.1): 24 dönüşümde 48 nokta (başlangıcı ve ölçeği farklı TM, Bessel ve Krasovski datumları iki kuralla, Pseudo-Mercator, projenin iki seçimi ve tersi, doğruluğu yazılmamış seçim, benzerlik ve afin yerel sistemler, özel TM'ye bağlı yerel sistem, bağsız datumun redleri). Yerli ve WASM: ızgara noktaları 1e-6 m, açılar 1e-11° içinde; doğruluk, dayanak ve resmîlik kesin.
   1b. WKT ve PROJ okuma ve yazma (Shapefile'ın okuyucusu buna taşınır); bağımsız başvuru `crs_text_cases.py`.
2. NTv2: okuma, ara değer, ileri ve geri; zincirde ızgaralı datum dönüşümü; bağımsız başvuru `ntv2_cases.py` ve bozuk dosyalar.
3. Sözleşme ve kayıt: `crs`, `secondCrs`, `datumTransforms`; `.kcad` şema 13 (spesifikasyon, kodek, Python okuyucu ve yazıcı, örnek dosyalar); iki platformun çözücüsü (durum çubuğu, Koordinat oku, ölçüler, dönüştürücü); cihazın ızgara kitaplığı.
4. Arayüz: Özel koordinat sistemi penceresi, Proje ayarları'nın grupları (Datum dönüşümleri, Izgaralar); iki platformda, resimleriyle; ortak iz.

Her adım iki platformda, ortak fixture'larla, kendi commit'inde ilerler.

## Sonuçlar

- Kayıtta olmayan sistem, yerel sistem ve bölgesel datum dönüşümü projede tanımlanır, iki platformda aynı hesapla kullanılır; Netcad'in Projeksiyon Ayarları, ArcGIS'in özel sistemi ve dönüşümü, QGIS'in özel sistemi ve NTv2'si karşılanır.
- Doğruluk ve dayanak her yerde yazılıdır; ızgara yoksa değer de yoktur.
- `.kcad` şema 13 üç alan ekler; eski dosyalar özel sistemsiz açılır.

## Doğrulama

- **Çekirdek:** PROJ başvurularına göre iki platformda (yerli ve WASM).
- **Arayüz:** ortak izlerle ve resimlerle iki platformda.
