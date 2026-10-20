# ADR 0565: QML'de tarama, işaretli çizgi ve veriye bağlı değerler

- **Durum:** kabul edildi (2026-10-21). `plan/gis.md` CBS-17. ADR 0555'in “sonraya kalan”larından üçü: çizgi deseni
  dolgusu, işaretli çizgi, veriye bağlı değerler. Kapsamı ben belirledim.
- **Bağlam belgesi:**
  - ADR 0555: QML alışverişi, profil, kayıpların söylenmesi;
  - ADR 0186: tarama dolgusu `hatchFill`;
  - ADR 0094: sembol tasarımcısının katmanları, `markerLine`;
  - ADR 0100: ifade dili;
  - ADR 0564: görünüş alanın adını izler.
- **Web:** ertelendi (ADR 0228). Web'de gerekenler TODOS.md §0.2'de.
- **Sonraya kalan:**
  - QGIS'in degrade, nokta deseni, SVG ve yazı işareti katmanları;
  - renklerin ve öbür özelliklerin veriye bağlı değerleri;
  - işlevli ve karşılaştırmalı ifadeler (`round`, `if`, `CASE` …). Bunlar iki dilde aynı anlamla yazılabilir, ama anlamın
    eşitliği ayrı bir başvuru ister;
  - etiketler, Katman çizimi, tematik işleyiciler.

## Bağlam

ADR 0555 QGIS'in dolgu, çizgi, işaret ve merkez dolgusu katmanlarını taşıdı. QGIS kullanıcısının en sık kullandığı iki
katman bunların dışında kalıyordu:
- çizgi deseni dolgusu (`LinePatternFill`): taranmış imar alanları, sit alanları;
- işaretli çizgi (`MarkerLine`): ok yönlü yollar, çit ve korkuluk simgeleri.

Ayrıca veriye bağlı değerler (`data_defined_properties`), örneğin alana göre kalınlık ya da yöne göre dönüş, yalnız
“alınmadı” diye söyleniyordu. KentOS'un sembolleri bunların üçünü de taşır: tarama dolgusu, işaretli çizgi ve
`{"expr", "fallback"}` değerleri.

## Karar

### 1. Tarama ve çizgi deseni

KentOS'un `hatchFill`'i QGIS'in `LinePatternFill`'idir. Eşlemesi:

| KentOS | QGIS |
|---|---|
| açı | `lineangle` |
| aralık | `distance` |
| kaydırma | `offset` |
| birim | `distance_unit`, `offset_unit` |
| renk, kalınlık, kesikler | alt simgenin basit çizgisi (`@ad@i`) |

İki yazılım da açıyı saat yönünün tersine ölçer. Bu QGIS'te çizdirilerek denetlendi; açı çevrilmez.

Yazılmayanlar söylenir: taramanın kaydırması (`stagger`) ve kesik başlangıcı (`dashOffset`) QGIS'te yoktur.

Okumada alt simge basit çizgi değilse tarama alınmaz ve söylenir. Alt simgenin birden çok katmanı varsa yalnız ilki
alınır. Çizginin birimi aralığınkinden başkaysa kalınlık ve kesikler alınmaz. Her durum adıyla söylenir.

### 2. İşaretli çizgi

KentOS'un `markerLine`'ı QGIS'in `MarkerLine`'ıdır. Eşlemesi:
- yerleşim: Aralıkla `Interval`, Köşelerde `Vertex`, İç köşelerde `InnerVertices`, İlk ve son köşe `FirstVertex` ve
  `LastVertex`, Orta `CentralPoint`, Kenar ortaları `SegmentCenter`;
- aralık ve baştan uzaklık birimleriyle;
- işaretin doğrultuya dönmesi (`rotate`);
- halkalar (`ring_filter`);
- işaret alt simge olarak.

QGIS birden çok yerleşimi birlikte tutabilir (`Interval|FirstVertex`), KentOS'ta tek yerleşim vardır. İlk tanınan alınır,
öbürleri adlarıyla söylenir. Paralel kaydırma ve işaret grubu yazılmaz, söylenir.

### 3. Veriye bağlı değerler

Üç değer iki yönde taşınır:

| KentOS | QGIS'in özelliği | Not |
|---|---|---|
| basit çizginin kalınlığı | `outlineWidth` | QGIS 4.2 `StrokeWidth`'i bu adla yazar |
| işaretin boyu | `size` | |
| işaretin dönüşü | `angle` | QGIS saat yönünde, KentOS tersine döner. İfade `-(…)` ile çevrilir; iki kez çevrilen ifade yazıldığı gibi döner |

İfadeler yalnız alanlar, sayılar, `+ - * / %` ve parantezlerse çevrilir. Bu küçük dilde iki yazılımın anlamı aynıdır.
Çevirmede alan adının yazılışı değişir: KentOS'ta `[Ad]`, QGIS'te `"Ad"` ya da yalın sözcük. QGIS'in alanla bağlı değeri
(`type` 2, `field`) alanın kendisi olarak okunur.

Sabit değer KentOS'ta `fallback` olur, QGIS'te özelliğin kendi değeri. QGIS'in varsayılanı olan sıfır açı yazılmaz; geri
okunduğunda `fallback`'siz ifade gelir. Anlamı aynıdır.

Söylenenler:
- KentOS'un bu katmanında karşılığı olmayan veriye bağlı değer, örneğin renkler: “QGIS'in “fillColor” veriye bağlı değeri
  KentOS'un bu katmanında yok; sabit değeri alındı”;
- çevrilemeyen ifade, örneğin `round("genislik", 1)`: ifade adıyla söylenir, sabit değer alınır.

## Doğrulama

- `python3 scripts/fixtures/qml_cases.py --check`:
  - bağımsız okuyucuya taramanın, işaretli çizginin ve veriye bağlı değerlerin kuralları eklendi;
  - QGIS 4.2'nin yazdığı iki yeni QML var. `qgis-tarama.qml`: çizgi deseni ile kalınlığı `"alan" / 1000` ve rengi alana bağlı
    kenar. `qgis-isaretli.qml`: iki yerleşimli işaretli çizgi; işaretin boyu, yönü ve rengi veriye bağlı; çizginin kalınlığı
    `round(…)`;
  - `file.import.qml`'nin 29 durumu yeniden üretildi;
  - KentOS'un yazdığı `kentos-tarama.qml` ve `kentos-isaretli.qml` QGIS'te `loadNamedStyle` ile açılır. Taramanın açısı,
    aralığı, kaydırması ve çizgisi, işaretli çizginin aralığı, yerleşimi ve işareti, üç ifade QGIS'in okuduğuyla
    karşılaştırılır.
- `KENTOS_WRITE_QML=1 cargo test -p kentos-headless --test headless layer_looks_go_out_as_qml`: yedi görünüş başsız sunucuda
  yazılır ve geri okunur.
- `cargo test -p kentos-formats --lib qml`: çekirdeğin birim testleri, ifadelerin çevrilip çevrilmediği dahil.
- `cargo test -p kentos-native-application --test all fixtures`: ortak durumlar masaüstünün işleyicisiyle.
