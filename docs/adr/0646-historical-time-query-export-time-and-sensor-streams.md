# ADR 0646: Değişmez duruma bağlı tarihsel sorgu, dışa aktarmada zaman ve sensör gözlemleri

- **Durum:** kabul edildi (2026-10-26). `plan/gis.md` CBS-33'ün kalanı. Kapsamı ben belirledim.
- **Kalan işler (ADR 0537 ve 0595'ten):**
  - tarihsel sorgunun değişmez bir duruma bağlanması;
  - dışa aktarmada aynı zaman süzgeci;
  - sensör ve gözlem akışı: varlık kimliği, olay ve alım zamanı, kalite, geç ve yinelenen kaydın politikası, akış
    kopunca son değerin güncel gösterilmemesi, OGC SensorThings.
- **Bağlam belgesi:**
  - ADR 0210: zamansal katman, anların okunuşu, gösterilme tablosu;
  - ADR 0537: iki zamanlı katman, `cad.time.query`;
  - ADR 0576: `cad.drawing.compare` ve geri alma geçmişinden ya da dosyadan okunan eski durum;
  - ADR 0595: zaman süzgeci kaynakta (`QueryTime`, `time_filter.rs`);
  - ADR 0410: geometrisiz kayıt;
  - ADR 0551: katmanın içerik özeti (`sources::layer_digest`).
- **Web:** ertelendi (ADR 0228). Web'de gerekenler TODOS.md §0.2'de.
- **`.kcad`:** şema değişmez. Sensör katmanı sıradan bir zamansal katmandır.
- **Bağımlılık:** yeni üçüncü taraf kitaplık yok.
- **Sonraya kalan:**
  - kayıt zamanının KentOS'ça kendiliğinden yazılması (sistem sürümlemesi): bugün kayıt zamanını sensör alımı ve
    kullanıcı yazar; düzenleme komutları yazmaz;
  - bulut projesinin sunucudaki revizyonuna (`project.changes`) bağlı tarihsel sorgu: masaüstü bugün geri alma
    geçmişindeki ve dosyadaki durumu sorar;
  - SensorThings'in MQTT ile canlı akışı, `Datastream`'lerin ve `Thing`'lerin keşfi, birim (`unitOfMeasurement`) ve
    gözlenen özelliğin taşınması;
  - İşlemler'in çizimdeki katman girdisinde zaman süzgeci (kaynaktan okunan girdi ADR 0595'ten beri süzülür);
  - GeoPackage dışa aktar'ın penceresi yok: masaüstünde zaman seçeneği pencereli dışa aktarmalardadır (CSV, FlatGeobuf,
    GeoParquet, GML, Shapefile, XYZ); GeoPackage'ı zamanla Python ve MCP yazar;
  - Sensör gözlemleri penceresinde adresten alma: pencere SensorThings dosyasını alır, adresi Python ve MCP alır (ağ
    beklerken arayüz donmasın diye adres işi ayrı iş parçacığına taşınınca eklenecek).

## Bağlam

Plan CBS-33 için üç şey daha istiyor:

- “Tarihsel sorgu immutable revision'a bağlansın.” `cad.time.query` bugün yalnız açık çizimin şimdiki hâlini sorar.
  Çizim değişince aynı soru başka cevap verir ve soran bunu bilemez.
- “Aynı tarih filtresi harita, tablo, analiz ve export'ta tutarlı olmalı.” Harita, tablo ve kaynaktan okunan analiz
  zamanı biliyor. Dışa aktarma ise bütün nesneleri yazıyor.
- Sensör ve gözlem akışı. Varlık kimliği, olay ve alım zamanı, kalite, geç ve yinelenen kayıt politikası gerekiyor.
  Akış kopunca son değer güncelmiş gibi gösterilmemeli.

Model ADR 0537'de var. Geçerlilik zamanı gözlemin olay zamanıdır (`phenomenonTime`). Kayıt zamanı alım zamanıdır. Kimlik
alanı varlıktır. Eksik olan, bu modeli kullanan komutlar.

## Karar

### §1 Tarihsel sorgu değişmez bir duruma bağlanır

`cad.time.query` iki isteğe bağlı girdi alır.

**`revision`** sorunun sorulduğu durumdur. `cad.drawing.compare`'in `against`'i gibidir:

- `{ "kind": "history", "steps": n }`: geri alma geçmişinde `n` adım önceki çizim;
- `{ "kind": "file", "path": "/…/ad.kcad" }`: bir çizim dosyası (KCAD v2 ya da v1 JSON).

Yoksa açık çizimin şimdiki hâli sorulur. Eski durum `drawing_compare`'in okuyucusuyla kurulur, açık çizim değişmez.
Katman o durumda aranır: eski durumda olmayan katman `layer_not_found` ile reddedilir.

**`expectedStamp`** soranın sabitlediği durumdur. Cevap her zaman `stamp` verir: katmanın o durumdaki içerik özeti
(`sources::layer_digest`). Bu, nesnelerin kalıcı kimliklerine, türlerine, geometrilerine ve özniteliklerine göre
kanonik JSON'un SHA-256'sıdır. Aynı özeti `expectedStamp` olarak veren soru, katman o durumda başkaysa
`state_changed` ile döner. Bu bir çakışmadır (`conflict`, yeniden denenebilir): soru o durumla yeniden sorulabilir.

Böylece:

- geri alma geçmişindeki adım sayısı kaysa da (çizime yeni adım eklenince) soru sessizce başka bir durumu sormaz;
- dosya değiştirilirse soru sessizce başka bir durumu sormaz;
- aynı özetle verilen iki cevap aynı veriden gelir.

Cevap ayrıca `state`'i söyler: sorulan durumun adı (“1 adım geri, “Taşı” öncesi”, dosyanın adı, yoksa “Şimdiki hâl”).

### §2 Dışa aktarmada zaman

Yedi dışa aktarma isteğe bağlı bir `time` alır (`QueryTime`, ADR 0595'teki gibi):
`file.export.gpkg`, `.shp`, `.csv`, `.fgb`, `.parquet`, `.gml` ve `.xyz`.

- Kural tek: her nesne `TimeFilter::passes` ile, `cad.time.query`'nin ve kaynak sorusunun kuralıyla sınanır.
- Süzgeç yazılan katmanların her birine uygulanır. Zaman alanları `time.fields`'tan, yoksa katmanın kendi zaman
  ayarından gelir.
- Zaman ayarı olmayan katman bütünüyle yazılır: harita onu her zamanda gösterir. Yazılan katmanların hiçbirinin zaman
  ayarı yoksa ve `time.fields` verilmediyse dışa aktarma `not_temporal` ile reddedilir.
- Bilinen gün yalnız sürümlerinin kayıt zamanını tutan katmanlara uygulanır. Hiçbiri tutmuyorsa `not_recorded`.
- Geçmeyen nesneler çizimin bir kopyasından kaldırılır, yazıcı kopyayı yazar. Açık çizim değişmez, geri alma adımı
  yoktur.
- Uyarı `time_left_out` kaç nesnenin yazılmadığını söyler.

Masaüstünde dışa aktarma pencerelerinde (CSV, FlatGeobuf, GeoParquet, GML, Shapefile, XYZ) Zaman sürgüsü açıkken
**Zaman sürgüsünün zamanıyla** seçeneği görünür. İşaretlenince sürgünün anı ya da aralığı ve Bilinen günü verilir;
pencerenin planı (yazılacak nesne sayısı) yeniden hesaplanır. Seçim oturumundur, kapalı başlar.

### §3 Sensör katmanı

Sensör katmanı zaman ayarı olan bir kayıt katmanıdır. Şema değişmez:

| Kavram | Katmanın zaman ayarı |
|---|---|
| Varlık (sensör, sayaç) | `key` |
| Olay zamanı (`phenomenonTime`) | `start` |
| Alım zamanı | `recordedStart` |
| Yerine yenisi geçti | `recordedEnd` |

Gözlemler geometrisiz kayıtlardır (ADR 0410). Değer ve kalite, komutun adını verdiği alanlara yazılır. Alanların türü
ve kuralı katmanın alanlarınındır (`checks::object_attrs`).

### §4 `cad.sensors.ingest`

Gözlemleri katmana tek geri alma adımında (“Sensör gözlemleri”) yazar.

**Kaynak.** Tam olarak biri verilir:

- `observations`: `{ asset, at, value?, quality? }` satırları;
- `file`: bir SensorThings `Observations` cevabının mutlak yolu (`{"value": [...]}` ya da gözlemlerin dizisi);
- `url`: bir SensorThings `Observations` adresi. `@iot.nextLink` en çok `pages` sayfa izlenir (varsayılan 10, en çok
  100). Kalan sayfa varsa cevabın `more`'u doğrudur. Ağ ev sahibinindir (`remote_host`).

**SensorThings'ten okuma.** `file` ve `url`'de varlık `asset` ile verilir: bir `Datastream`'in gözlemleri tek varlığındır.

- Olay zamanı `phenomenonTime`'dır. Aralıksa (`a/b`) başlangıcı alınır.
- Değer `result`'tır: sayı en kısa yazımıyla, metin olduğu gibi, evet/hayır `true`/`false`, nesne kanonik JSON.
  `null` değer yazılmaz.
- Kalite `resultQuality`'dir, aynı kuralla.

**Alım zamanı.** `receivedAt` alım zamanıdır, yoksa şimdi (UTC, saniyeye). Katmanın `recordedStart`'ı varsa ona
yazılır. Olay zamanı katmanın `start`'ına UTC olarak yazılır (`2024-06-01T10:00:00Z`; gece yarısı `2024-06-01`).

**Yinelenen gözlem.** Aynı varlığın aynı andaki gözlemidir. Anlar okunarak karşılaştırılır, metinle değil. Karşılaştırma
katmanın şimdiki sürümleriyle (kayıt bitişi boş olanlar) ve partinin önceki satırlarıyla yapılır. `duplicates`:

- `first` (varsayılan): var olan kalır, gelen atlanır (`skipped`);
- `last`: gelen geçer. Katman `recordedEnd` tutuyorsa eski sürümün kayıt bitişine alım zamanı yazılır ve yenisi
  eklenir: eski değer o güne dek bilinmiş olarak kalır. Tutmuyorsa eskinin değeri ve kalitesi yerinde değişir
  (`replaced`). Partinin içinde son satır kalır.
- `all`: hepsi yazılır.

**Geç gelen gözlem.** Varlığın daha önce yazılmış (ya da partide önce gelmiş) en son anından önceki andadır.
Yinelenen geç sayılmaz. `late`:

- `accept` (varsayılan): yazılır, `late` sayılır. `lateField` verildiyse o alana `evet` yazılır;
- `reject`: yazılmaz, `rejected`'da `late_observation` ile söylenir.

**Satır retleri.** Komutu değil satırı reddeder, `rejected`'da sırası, varlığı, anı, kodu ve nedeniyle:

- `invalid_asset`: boş varlık;
- `invalid_moment`: okunmayan an;
- `late_observation`: geç gelen gözlem, `late: reject` ile;
- `invalid_attribute` ya da `attribute_required`: alanın kuralına uymayan değer ya da kalite (sayı olmayan, aralık
  dışı, listede olmayan). Kod ve ileti katmanın alanlarının denetimininkidir (`checks::object_attrs`).

Hiçbir satır yazılmazsa adım açılmaz.

**Komutun retleri.** Beklenen sürüm (`invalid_revision`, `revision_conflict`); katmanın varlığı, katman olması ve
kilitli olmaması (`checks::layer_at`), bir de:

- `not_temporal`: zaman ayarı yok;
- `no_asset_field`: zaman ayarında kimlik alanı (`key`) yok;
- `invalid_source`: kaynaklardan hiçbiri ya da birden çoğu;
- `invalid_input`: `file` ya da `url` ile `asset` yok; değer alanı boş;
- dosyanın ve adresin okunamaması (`file_not_found`, `import_unreadable`, `source_unreachable`);
- en çok 100 000 gözlem (`invalid_count`).

**Cevap:** `added` (yeni kayıtların kalıcı kimlikleri), `replaced` (değeri değişen ya da kayıt bitişi yazılan eski
kayıtlar), `skipped`, `late`, `rejected`, `pages`, `more`, `receivedAt` ve sürüm.

### §5 `cad.sensors.latest` (salt okunur)

Her varlığın son gözlemini verir. Bir değer o andaki gerçek durum gibi gösterilmez: yaşı söylenir.

**Sorulan an.** `at` sorulan andır; yoksa şimdi. `knownAt` ADR 0537'deki gibi o gün bilinen sürümleri alır. Yoksa
şimdiki sürümler alınır (kayıt bitişi boş, katman kayıt zamanı tutmuyorsa hepsi).

**Son gözlem.** Varlığın anı `at`'ten geç olmayan en geç gözlemidir. Eşitlikte alım zamanı geç olan, sonra çizimde
sonra gelen seçilir.

**Cevap.** Her varlık için:

- `asset`, `uid`, `observedAt`, `value`, `quality`;
- `ageSeconds`: `at − observedAt`;
- `stale`: `staleAfter` saniye verildiyse yaş ondan büyükse.

Varlıklar doğal sırayla dizilir (`text::natural`). `expectedAssets` verilen varlıklardan hiç gözlemi olmayanlar `missing`'de
söylenir. Anı okunmayan gözlemler `unreadable` sayılır.

Retler: `cad.time.query`'ninkiler (`layer_not_found`, `not_a_layer`, `not_temporal`, `invalid_moment`, `not_recorded`),
`no_asset_field`, `invalid_input` (sıfırdan küçük `staleAfter`).

### §6 Masaüstü

**Sensör gözlemleri penceresi** (`time.sensors`, CBS › Harita › Zaman ve senaryo ▾):

- Katman yalnız sensör katmanlarıdır: zaman ayarı kimlik alanı adlandıran ve geometrili nesnesi olmayan kayıt
  katmanları. Değer ve kalite alanı, Sorulan an ve Eskime süresi (dakika) seçilir.
- Sorulan an boşsa Zaman sürgüsü açıkken onun anıdır (aralıkta aralığın son anı) ve Bilinen günü sürgününkidir;
  sürgü kapalıysa şimdi.
- Tablo her varlığın son değerini, kalitesini, anını ve yaşını gösterir. Eskiyen satır uyarı simgesiyle
  “Akış kesik: 2 sa” der, öbürleri “Güncel”.
- Sürgü oynadıkça tablo yeniden sorulur.
- **Gözlem al…** bir SensorThings JSON dosyasını, yazılan varlığın gözlemleri olarak alır. Aynı andaki gözlem (İlki
  kalır, Sonuncusu geçer, Hepsi yazılır) ve Geç gelen gözlem (Kabul et, Reddet) seçilir; sonuç tek cümlede söylenir.

Dışa aktarma pencerelerindeki zaman seçeneği §2'dedir.

### §7 Python ve MCP

- Komutlar `kentos.cad.sensors.ingest` ve `.latest` olarak üretilir. `cad.time.query`'nin yeni girdileri tiplerdedir.
- MCP'de `cad.sensors.ingest` yıkıcı listededir: var olan kaydın değerini ya da kayıt bitişini değiştirir.
- Python'un ağı `url`'yi `kentos._remote.Http` ile okur (ADR 0645 §5).

### §8 Ölçümler

Bu makinede, release. `cargo test --release -p kentos-native-application --test all sensors_timing -- --ignored --nocapture`:

| İş | Süre |
|---|---:|
| 1 000 sayacın 100 000 okuması, boş sensör katmanına | 391 ms |
| 10 000 yinelenen okuma, sonuncusu geçer (eski sürümler biter) | 285 ms |
| 110 000 kayıtta 1 000 varlığın son gözlemi | 56 ms |
| 100 000 parselin içerik özeti | 134 ms |
| Özetle sabitlenmiş tarihsel soru (özet ve süzgeç) | 161 ms |
| Dışa aktarmanın zamanı: 100 000 parselden 80 000'i kopyadan kaldırılır | 136 ms |

Yinelenen ve geç gözlem, katmanın şimdiki sürümlerinin (varlık, an) eşlemesiyle bulunur: her satır sabit sürede.

## Doğrulama

- **Tarihsel sorgu:** `scripts/fixtures/time_query_cases.py` 47 durum yazar (33'ü ADR 0537'den, 14'ü yeni):
  - bir adım önceki durum (silinen geç düzeltme orada görünür);
  - şimdiki hâl ve geçmiş durum özetiyle sabitlenir;
  - çizim değişince ya da geri alma adımı kayınca eski özet reddedilir;
  - tutmayan özet;
  - dosyadaki durum, özetiyle ve şimdiki hâlin özetiyle;
  - dosyada olmayan katman, boş geçmiş, yetmeyen adım, göreli ve olmayan dosya.

  Özet Python'da bağımsız hesaplanır: kurulum kimlikleriyle verilen v2 çizimidir, dosya bağımsız KCAD v2 yazıcısıyla
  (`kcad_v2_reference.py`) yazılır, kanonik JSON `protocol_cases.py`'ninkidir. Bütün durumlar ilk denemede geçti.
- **Sensörler:** `scripts/fixtures/sensor_cases.py` alma için 41, son gözlem için 28 durum yazar. Anlar
  `temporal_cases.py`'den, alanların kuralı `layer_field_cases.py`'den, doğal sıra onun `natural_cmp`'inden gelir.
  SensorThings dosyaları (`fixtures/sensors/v1`) OGC SensorThings API 1.1'in Observations cevabından yazıldı.
- **Adres:** `python/tests/test_sensors.py` yerel bir HTTP sunucusuyla iki sayfayı (`@iot.nextLink`), sayfa sınırını
  ve olmayan adresi (`source_unreachable`) sınar.
- **Dışa aktarma:** `scripts/fixtures/time_export_cases.py` 17 durum yazar (12'si dosya, 5'i ret). Başsız sunucunun
  testi (`time_exports`) dosyaları `fixtures/time-export/v1/out`'a yazar. `--written` her dosyanın tuttuğu nesneleri
  okur ve başvurunun seçtikleriyle karşılaştırır: GeoPackage, CSV, FlatGeobuf, GML ve Shapefile'ı OGR ile, GeoParquet'i
  pyarrow ile, XYZ'yi köşelerinin doğularıyla.
- **Masaüstü:** resimler `KENTOS_SHOTS_ONLY=sensor-gozlemleri,sensor-gozlemleri-an,zaman-disa-aktar`.
