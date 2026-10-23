# ADR 0595: Zaman süzgecini kaynağa taşımak

- **Durum:** kabul edildi (2026-10-23). `plan/gis.md` CBS-33'ün ikinci dilimi (ADR 0537'nin “sonraya”sının ilki): zaman
  süzgecinin sağlayıcıya, İşlemler'in girdisine ve kaynaktan okunan tabloya taşınması.
- **Bağlam belgesi:**
  - ADR 0210: zamansal katman, anların okunuşu, gösterilme tablosu;
  - ADR 0537: iki zamanlı katman, `cad.time.query`;
  - ADR 0581: `cad.sources.query`, kaynağa giden koşulun kapsayan kuralı;
  - ADR 0591: kaynakta sıralama ve Kaynağın tümü kipi;
  - ADR 0592: İşlemler'de kaynaktan okunan girdi.
- **Web:** ertelendi (ADR 0228), TODOS.md §0.2.
- **`.kcad` şeması:** değişmez.
- **Sonraya kalan:**
  - kayıt zamanının değişmez sürüm kaydına (projenin revizyonları) bağlanması;
  - sensör ve gözlem akışı: varlık kimliği, olay ve alım zamanı, kalite, geç ve yinelenen kayıt, akış kopunca son değerin
    güncel gösterilmemesi, OGC SensorThings;
  - dışa aktarmada aynı süzgeç (bugün dışa aktarma bütün nesneleri yazar);
  - tipli sütunda dizin kullanan koşul (bugün sütunun metni karşılaştırılır, §2);
  - iş günü gibi takvim dışı birimler ve projenin yerel saat dilimi.

## Bağlam

ADR 0537 “şu tarihte geçerli” ile “o gün bilinen”i ayırdı. Ama soru yalnız çizimin nesnelerinde sorulabiliyordu
(`cad.time.query`, Zaman sürgüsü). Bağlı katmanın kaynağın tümünü okuyan soruları zamanı bilmiyordu: Öznitelik tablosunun
Kaynağın tümü kipi, İstatistik'in Kaynağın tümü kapsamı, İşlemler'in `scope: source` girdisi. Bu yüzden sürgü 2021'i
gösterirken tablo ve analiz bütün yılları sayıyordu. Plan bunu açıkça istiyor: “aynı tarih filtresi harita, tablo, analiz ve
export'ta tutarlı olmalı”, “zaman filtresi provider'a itilebilsin”.

## Karar

### 1. Sorunun zamanı

`cad.sources.query` `time` alır, `cad.time.query`'nin dilinde:

- **geçerlilik zamanı:** `at` bir an, ya da [`from`, `to`) aralığı;
- **bilinen gün:** `knownAt`;
- **alanlar:** `fields`, katmanın zaman ayarının biçiminde (`start`, `end`, `cumulative`, `recordedStart`, `recordedEnd`).
  Zaman ayarı olan katmanda verilmezse katmanınkiler alınır. Tablo sorusunda (`path`, `table`) ya da zaman ayarı olmayan
  katmanda verilmeleri gerekir.

Her kayıt burada ADR 0210 §4'ün ve ADR 0537'nin kuralıyla sınanır:

- aralıklı, anlık ya da birikimli;
- zamansız kayıt (başlangıcı boş ya da okunmayan) her anda gösterilir;
- bilinen gün kaydın kayıt aralığında olmalıdır.

Kural `time_filter.rs`'te `cad.time.query` ile ortaktır. Planın `local`'ı `time` der; `exact` yanlıştır.

Retler:

- an ile aralık birlikte, bitişsiz ya da ters aralık, boş soru: `invalid_period`;
- okunmayan an: `invalid_moment`;
- alansız tablo ya da zaman ayarı olmayan katman: `not_temporal`;
- kayıt başlangıcı olmadan bilinen gün: `not_recorded`;
- kendi içinde bozuk alanlar: `invalid_input`;
- tabloda olmayan sütun: `field_not_found`, yolu `time.fields.end` gibi.

### 2. Kaynağa giden koşul

Kural kapsayandır (ADR 0581 §3'ün ilkesi): kaynak, kuralın tuttuğu her kaydı ve belki fazlasını verir; fazlası burada elenir.
Tarih sütununun ilk on karakteri, anlarla karşılaştırılır. Anlar her yandan bir gün genişletilir; bir saat dilimi bir anı
bir günden az kaydırır. Bir değer şu üç hâlde tutulur: boşsa, ISO tarihiyle başlamıyorsa (`01.06.2024`, bir sözcük, bir
sayı), ya da ilk on karakteri genişletilmiş sınırların içindeyse. Kaynağa gidenler:

| Soru | Sütun | Koşul |
|---|---|---|
| bir an ya da aralık | başlangıç | ≤ son an + 1 gün |
| aralıklı ve birikimli değilse | bitiş | ≥ ilk an − 1 gün |
| anlıksa | başlangıç | ≥ ilk an − 1 gün |
| bilinen gün | kayıt başlangıcı | ≤ gün + 1 gün |
| bilinen gün | kayıt bitişi | ≥ gün − 1 gün |

Kaynağa göre biçimi:

- **SQLite:** `("c" IS NULL OR "c" NOT GLOB '[0-9][0-9][0-9][0-9]-[0-9][0-9]-[0-9][0-9]*' OR substr("c", 1, 10) <= ?)`.
- **PostgreSQL:** sütunun metni üzerinde aynı koşul, `CAST(c AS text) !~ '^[0-9]{4}-…'` ve `left(…, 10)` ile. Böylece metin,
  `date` ve `timestamptz` sütunları aynı kuralla süzülür.

Filtrenin itilen kısmıyla `AND` ile birleşir. Plan değerleri yazılmış SQL'i gösterir.

### 3. İşlemler'in girdisi

`scope: source` girdisi de `time` alır. Bağlı katmanın kendi alanları kullanılır; tablo için `fields` verilir. Kayıtlar
çalıştırmanın kopyasına zamanla okunur. Ret yolları `parameters.input.time.at` gibidir.

### 4. Masaüstü

Zaman sürgüsü açıkken zamansal bir bağlı katmanın kaynak soruları sürgünün anını ve bilinen gününü sorar:

- Öznitelik tablosunun Kaynağın tümü kipi;
- İstatistik'in Kaynağın tümü kapsamı.

Sürgü oynayınca açık sorular yeniden sorulur (`time_sources`). Tablonun alt satırı “Zaman sürgüsünün anı kaynakta ve burada”
der. Yalnız zaman koşulu gittiğinde “süzgeç kaynakta” demez.

## Doğrulama

- Ortak durumlar: `fixtures/commands/v1/cad.sources.query.time.json`, 33 durum. Bağımsız başvuru
  `scripts/fixtures/source_time_cases.py`. `zaman.gpkg`'yi GDAL yazar: DATE ve DATETIME sütunlu sürümler ile metin olarak
  yazılmış anlarla olaylar (UTC, bölgeli, tarih, `01.06.2024`, boş, sözcük). Kayıtlar sqlite3'le okunur, kurallar
  `temporal_cases.py`'nin ve `time_query_cases.py`'nin başvurusundan gelir. Kaynağın kaç kayıt verdiği (`read`) ve planın SQL'i
  §2'nin kuralıyla saklanan metinlerden hesaplanır, KentOS kodu olmadan. Durumlar:
  - an, aralık, yarı açık uçlar, artık gün, başlangıcı boş sürüm, ISO olmayan tarih, birikimli;
  - bilinen gün (geç kayıt, kayıt anının kendisi);
  - bölgeli anlar;
  - sayım ve süzgeçle;
  - bağlı katmanın kendi ayarıyla, kaynakta ve çalışma kopyasında;
  - İşlemler'in girdisi;
  - 11 ret.
- PostGIS: `crates/native/postgis/tests/all.rs`'in `times_answer_alike_from_postgis`'i geçici docker PostGIS'inde
  (`scripts/fixtures/postgis_cases.py`). Aynı dokuz soru dosyada ve veritabanında aynı kayıtları verir. Bu hem metin olarak
  kopyalanan değerlerle (sqlite3'ten psycopg2'yle) hem `date` ve `timestamptz` sütunlarıyla denenir. Tipli sütunlarda yalnız
  ISO olmayan tarihli satır dışarıda bırakılır, çünkü onu bir date sütunu tutamaz.
- Masaüstü: `features::whole_tests::the_whole_source_is_read_at_the_sliders_time`. Soru sürgünün zamanını taşır, gösterilen her
  satır o zamandadır, sürgü oynayınca yeniden sorulur. Resim `.run/shots/tablo-tumu-*-5-zaman-*`.
- `cad.time.query`'nin 33 durumu ortak modüle geçişten sonra değişmeden geçer.
