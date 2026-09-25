# ADR 0002: Sözleşmeler ve paylaşılan golden fixture'lar

- **Durum:** kabul edildi
- **Tarih:** 2026-09-23
- **Bağlam belgesi:** CLAUDE.md §14, §20 Faz A
- **Sonraki kararlar (2026-09-25):** sunucudan MVT/TileJSON ile veri gelmesi varsayımı kalktı ([ADR 0012](0012-server-scope-project-cloud.md)); `DocumentSnapshotV1` v1 sözleşmesi olarak kalır, binary `.kcad` yeni bir sürüm olacak ([ADR 0011](0011-kcad-binary-snapshot.md)). Değişen madde aşağıda işaretlidir.

## Bağlam

Tarayıcı, WASM, API ve saklanan dosyalar aynı veriyi konuşacak. Tipler iki dilde elle tutulursa zamanla ayrışır. Geometrinin iki dilde aynı sonucu verdiği de ölçülerek gösterilmelidir, varsayılamaz.

## Karar

### Sözleşmeler: tek tanım Rust'ta

- **Tek kaynak `crates/shared/contracts`'tır.** `cargo test -p kentos-contracts`, ts-rs ile TypeScript tiplerini `apps/web/src/contracts/generated/` altına yazar; bu dosyalar depoya girer.
- **Uygulamanın iç tipleri** (`Entity`, `LayerNode` …) derleme anında sözleşmeye karşı denetlenir (`apps/web/src/contracts/contracts.test.ts`). Bir alan eklenip sözleşmeye yazılmazsa `tsc` hata verir.
- **v1 sözleşmeleri:**
  - `Entity` (13 tür, `kind` etiketli);
  - `LayerNode` ve `LayerStyle`;
  - `ProjectSettings`;
  - `DocumentSnapshotV1` (`format: "kentos.document"`, `version: 1`);
  - `StyleFile` (`kentos-style` v1);
  - `RunJob`;
  - `Health`;
  - `CommandEnvelope` (§18);
  - `NumericPolicy`, `DecimalString`, `ShareValue` (§23, ADR 0004).
- **Sürüm kuralı:** saklanan ya da gönderilen her belge `format` ve `version` taşır. Okuyucu bilmediği sürümü açık bir hatayla reddeder, tahmin etmez.
- **v1 sınırları:**
  - `id` belgenin yerel kimliğidir. Sunucunun kalıcı kimliği UUID'dir; Faz B'de PostgreSQL `feature` tablosuyla ayrı bir `FeatureRef` sözleşmesi olarak gelir (§15). MVT'nin sayısal kimliği ayrı bir eşlemedir.
  - Öznitelikler v1'de metindir.
  - Stil motorunun kendi tipleri (katman işleyicisi, kitaplık öğeleri) opak JSON'dur (`unknown`). `style-core` Rust'a taşınınca tiplenecek.
- **`SceneLayer` sözleşme değildir.** GPU'ya özel iç tiptir (`Float32Array` toplulukları, yerel orijin) ve ağdan gitmez.
  - ~~Sunucudan veri MVT/TileJSON ile gelir; MVT CAD için kayıplıdır, editör gerçek CAD tanımını ayrı feature API'sinden alır (§17).~~
  - ~~Özel ikili sahne karosu yalnızca MVT'nin ölçülmüş sınırı ortaya çıkarsa ayrı sürüm olarak tasarlanır.~~
  - **2026-09-25'te değişti ([ADR 0012](0012-server-scope-project-cloud.md)):** bulut verisi yetkili proje/nesne servisleri ve binary `.kcad` revizyonlarıyla gelir; MVT/tile yayını şimdiki kapsamda yoktur. `SceneLayer`'ın sözleşme olmadığı ve ağdan gitmediği kural sürer.

### Golden fixture'lar

- `fixtures/geometry/v1/cases.json` her durum için girdi ve beklenen sonucu taşır. Beklenen sonuçlar bir kez TypeScript referansından kaydedildi (`GOLDEN_WRITE=1`) ve kilitlendi. Değiştirmek, bilinçli ve incelenmiş bir davranış değişikliğidir.
- Bağımsız referanslar (ADR 0004, §23.4): `fixtures/geometry/v1/reference.json` ve `fixtures/numeric/v1/*`. KentOS kodu olmadan, Python kesin aritmetiğiyle üretildi. Eski TypeScript sonucuna eşitlik tek doğruluk ölçütü değildir.
- Aynı dosyayı okuyanlar:
  - TypeScript: `apps/web/src/model/geom/golden.test.ts`;
  - yerel Rust: `crates/shared/geometry-core/tests/golden.rs`;
  - WASM: `apps/web/src/wasm/golden.wasm.test.ts`, `pnpm test:rust` ile. Bağımsız referanslar da (`reference.json`) aynı sınırlarla WASM'da sınanır.
- **Tolerans:** `|gerçek − beklenen| ≤ 1e-9 + 1e-14·max(|gerçek|, |beklenen|)`. Sınır CRS türüne göre yazılır (§14): dosya koordinatlarının metre cinsinden bir projeksiyon düzleminde olduğunu `crs` alanında söyler; iki okuyucu da bunu denetler. Coğrafi (derece) koordinat ve jeodezik hesap ayrı dosya ve ayrı sınırla gelir.
  - Formüller ve işlem sırası iki dilde aynı olduğu için toplama ve çarpma aynı sonucu verir.
  - Fark yalnızca `atan2`, `sin`, `hypot` gibi kütüphane işlevlerinin son bitinden gelebilir.
  - TM koordinatlarında (4,4·10⁶ m) 1 ulp ≈ 9·10⁻¹⁰ m'dir. Göreli terim bu ölçekte 4,4·10⁻⁸ m'ye izin verir; bu da milimetrenin çok altıdır.
- **Faz A kapsamı:** tanımı iki tarafta da tam olan işlevler:
  - bulge yayı, yollu uzunluk, halka alanı, işaretli alan;
  - delikli çokgen alanı ve çevresi;
  - nokta-çokgen testi;
  - düz şekillerin ve dairenin sınır kutusu.
- **Bilinen fark:** TypeScript `entityBounds`, yaylı yolların ve yayların sınırını 72 parçalı ana hatla bulur (yaklaşık). Rust'a kesin yay sınırı taşınırken iki taraf birlikte değişecek; o güne kadar bu işlevler golden setinde yoktur.

### Çağrı fixture'ları (ADR 0008)

- Geometri Rust'a taşınırken her modülün davranışı `fixtures/geometry/v1/calls-*.json` dosyalarına dondurulur: `{ fn, args, expect }` satırları, aynı tolerans ve `crs` alanıyla. Kayıt TS varken yapılır (`apps/web/scripts/fixtures/record-calls.test.ts`); native (`tests/calls.rs`) ve WASM (`apps/web/src/wasm/calls.wasm.test.ts`) aynı dosyaları çekirdeğin çağrı tablosundan geçirir.
- TS silinince bu dosyalar davranış kilidi olarak kalır; `cases.json` gibi, değiştirmek incelenmiş bir karardır.

### CRS kaydı

- `fixtures/crs/v1/registry.json`, `apps/web/src/geo/crs.ts`'ten üretilir (`apps/web/scripts/fixtures/record-crs.test.ts`, `GOLDEN_WRITE=1`). Kaynak TypeScript kaydı kalır (§5).
- `apps/web/src/geo/crs.test.ts`, dosya kayıttan ayrılınca kırılır.
- `crates/shared/contracts/tests/crs.rs` dosyayı bağımsız olarak EPSG değerlerine göre denetler: SRID ile dilim eşlemesi, elipsoit, ölçek katsayısı, başlangıç ötelemesi. TUREF dilim önerisini de aynı kuralla (en yakın orta meridyen, sınırda batı dilimi) yeniden hesaplar.
- Dönüşüm (datum, dilim) Faz B/C'de PROJ/PostGIS ile, sabitlenmiş grid verisiyle gelir. O zaman bu dosyaya dönüşüm referans noktaları eklenir.

## Sonuçlar

- Rust ve TypeScript arasındaki her uyumsuzluk bir testte görünür. Sözleşme değişikliği TS tarafında derleme hatası olarak ortaya çıkar.
- Opak stil alanları v1'de doğrulanmaz. Doğrulama şimdilik `apps/web/src/style/file.ts` içindeki TypeScript okuyucusunda kalır.
