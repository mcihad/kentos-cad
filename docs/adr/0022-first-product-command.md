# ADR 0022: İlk ürün komutu `cad.polygon.create`: web ve masaüstü işleyicileri, ortak durumlar

- **Durum:** kabul edildi (2026-09-26). Yön ADR 0013'ün “Sıradaki dilim”inden ve TODOS.md §22.1 adım 6'dan (`CMD-04..07`) gelir. Sonuç tipi, denetimler ve sıraları, hata kodları, native crate, web katmanı ve durum dosyasının biçimi bu dilimin kararıdır.
- **Tarih:** 2026-09-26
- **Bağlam belgesi:** TODOS.md `CMD-03..10`, `ARCH-07`, `DOM-12`, `AI-01`, `AI-02`, `PY-16`; ADR 0003 (işlem), 0010 (platform sınırı), 0013 (ürün komutu sözleşmesi), 0014 (kalıcı kimlik), 0018 ve 0021 (araç oturumu), 0020 (native belge)

## Bağlam

- ADR 0013 iki düzey ayırdı: arayüz komutu (`core/commands.ts`, pencere açar, araç seçer) ve ürün komutu (katalogda, tipli, sürümlü, arayüzsüz çalışabilir). Katalogda yalnız sunucunun üç komutu vardı (`project.changes`, `project.share`, `project.access.revoke`).
- Kapalı alan aracı iki platformda belgeye kendisi yazıyordu (koddan doğrulandı, 25 Eylül):
  - web: `PathTool` (`closed: true`) → `PointInputTool.create` → `writableLayer` → `CadDocument.add`;
  - masaüstü: `kentos_interaction::polygon::Polygon::create` → `Document::add`.
- Kilitli katman denetimi, gizli katman uyarısı ve etkin katmanın okunması araçların içindeydi. Python, AI ya da CLI aynı işi yapmak isteseydi bu kuralları yeniden yazması gerekirdi (CLAUDE.md §18).
- TODOS.md `CMD-04` akışı (`discover → validate → preview/plan → execute → progress/result`), `CMD-05` açık sonuç durumlarını, `CMD-06` tenant'sız yerel yürütme bağlamını, `CMD-07` örtük arayüz durumunun girdide açık olmasını ister.

## Karar

### Katalog kaydı

`cad.polygon.create` v1 (`crates/shared/contracts/src/cad.rs`, `catalog.rs`):

| Alan | Değer | Neden |
|---|---|---|
| `effect` | `document` | Açık çizimi değiştirir, geri alınabilir |
| `hosts` | `web`, `desktop` | İki işleyici var; sunucu işleyicisi yok, katalogda görünmez (ADR 0013) |
| `headless` | evet | Girdide her şey açık (`CMD-07`) |
| `requires` | `document` | Açık çizim |
| `permissions` | yok | Yerel çizim kullanıcının kendisinindir. Bulut projesinde değişiklik `project.changes` ile gider, orada `feature.write` sunucuda denetlenir. Komut sunucuda çalışınca izin buraya girer |
| `undo` | `step` | Tek geri alma adımı, adı “Ekle” (aracın hep yazdığı ad) |
| `cost` | `instant` | |
| `aliases` | yok | `KA`, `ALAN`, `POLYGON` arayüz komutu `tool.polygon`'undur; aynı adlar iki düzeye verilmez |
| `examples` | 2 | Düz dikdörtgen (çıktısıyla); yarım daire kenarlı, delikli, renkli, beklenen sürümlü alan |

**Girdi** `PolygonCreate`:

| Alan | Anlamı |
|---|---|
| `layerId` | Hedef katmanın kimliği; grup değil. Araç onu etkin katmandan doldurur, komut arayüzü okumaz |
| `pts` | Dış halkanın köşeleri: x doğu (Y), y kuzey (X), float64. En az 3; ilk köşe sonda tekrarlanmaz |
| `bulges`? | Kenar başına bir yay değeri, kapanış kenarı (son köşe → ilk) sonda: tan(θ/4), saat yönünün tersi artı, 0 düz. `PathEntity`'nin sakladığı biçim |
| `holes`? | Delikler: her biri en az 3 köşeli, yay değeri verildiyse kenar başına bir tane (`RingGeometry`) |
| `color`? | Renk; yoksa katmana göre. Web aracı güncel rengi yazar; masaüstünde güncel renk yok |
| `attrs`? | Öznitelikler (v1'de metin); yoksa boş |
| `expectedRevision`? | Girdinin hazırlandığı belge sürümü, ondalık metin. Belge o sürümde değilse hiçbir şey yazılmaz: `conflict` |

**Çıktı** `PolygonCreated { uid, id, revision }`: kalıcı kimlik (UUIDv7), yuva (açık belgede geçerli) ve yazıdan sonraki sürüm. **Plan** `PolygonPlan { entity, revision }`: yazılacak nesne (yuvası 0, yazılınca verilir) ve planın yapıldığı sürüm.

**Sürüm metindir** (`DOM-12`): `^(0|[1-9][0-9]*)$`, şemada da bu desen. Yalnız eşitliği anlam taşır: web açılışta bir kez sayar, masaüstü saymaz (ADR 0020); iki uygulama aynı çizimde farklı sayılar verebilir. Hiç olmamış büyük bir sürüm (`99999999999999999999999`) çakışmadır, hata değildir; iki taraf böylece sayıyı çözmeden karşılaştırır.

### Sonuç: `CommandResult<T>` (`CMD-05`)

`crates/shared/contracts/src/command.rs`. Durum etiketle taşınır, başarı bir doğru/yanlıştan okunmaz:

| `status` | Taşıdığı | Bu komutta |
|---|---|---|
| `completed` | `output`, `warnings` | yazıldı (ya da denetim/plan tamam) |
| `queued` | `jobId` | yok (maliyet `instant`) |
| `needs_input` | `error` (eksik alanın yolu) | yok |
| `conflict` | `error` (şimdiki sürümle) | beklenen sürüm tutmadı |
| `cancelled` | — | yok |
| `failed` | `error` | reddedildi |

- `CommandError { code, message, path?, revision? }`: sabit kod programlar içindir; ileti Türkçe, nedeni ve çözümü söyler (CLAUDE.md §8); `path` girdideki alandır (`pts[2].y`, `holes[0].bulges`); `revision` çakışmada belgenin şimdiki sürümüdür (`ARCH-07`'nin ilk parçası).
- `CommandWarning { code, message, path? }`: tamamlanmış işin uyarısı.
- Üç kip aynı tipi döndürür, çıktısı kipin kendisidir: doğrulama `null`, plan `PolygonPlan`, yürütme `PolygonCreated`.
- Katalogdaki `output` şeması yürütmenin çıktısıdır (`project.changes`'teki gibi); sonuç zarfı ortaktır.

### Akış (`CMD-04`) ve denetimlerin sırası

- **validate:** girdiyi belgeye karşı denetler, hiçbir şey yazmaz.
- **plan:** aynı denetim; yazılacak nesneyi ve planın sürümünü verir. Hiçbir şey yazmaz: belge, sürümü, kirli bayrağı, geri alma ve yineleme geçmişi değişmez.
- **execute:** yeniden denetler, belgenin kendi `add`'iyle tek adım yazar. Açık bir işlem ya da grup varsa ona katılır; sürüm o bitince değişir. Planın sürümü `expectedRevision` olarak verilirse yalnız incelenen plan yazılır ya da hiçbir şey.

İlk tutmayan denetim cevap verir:

| Sıra | Kod | Durum | Yol | İleti (ADR'de özet; tam metin durum dosyasında) |
|---|---|---|---|---|
| 1 | `too_few_corners` | failed | `pts`, `holes[h].pts` | `Kapalı alanın en az 3 köşesi olmalı; 2 köşe verildi. Eksik köşeleri ekleyin.` |
| 1 | `not_finite` | failed | `pts[i].x` … `holes[h].bulges[i]` | köşe ya da kenar sırası, eksen (doğu (Y), kuzey (X)), “NaN ya da sonsuz”, düzeltme |
| 1 | `bulge_count` | failed | `bulges`, `holes[h].bulges` | köşe ve yay değeri sayısı, “kapanış kenarı dahil” |
| 2 | `invalid_revision` | failed | `expectedRevision` | sürümün yazımı |
| 3 | `revision_conflict` | conflict | `expectedRevision` | `Çizim bu komut hazırlandıktan sonra değişti; hiçbir şey yazılmadı. …`, şimdiki sürümle |
| 4 | `layer_not_found` | failed | `layerId` | |
| 4 | `not_a_layer` | failed | `layerId` | grup verildi |
| 4 | `layer_locked` | failed | `layerId` | aracın bugünkü metni, örneğin `“Yapı” katmanı kilitli. Kilidi Katmanlar panelinden açın ya da başka bir katmanı etkinleştirin.` |
| — | `layer_hidden` | uyarı | `layerId` | aracın bugünkü metni, `“Yapı” katmanı gizli; çizilen nesne görünmeyecek.`; nesne yazılır |
| — | `slots_exhausted` | failed | — | yalnız masaüstü: belgenin `u32` yuvaları bitti |

- Birinci sırada önce dış halka, sonra her delik; her halkada köşe sayısı, sonlu koordinatlar (köşe sırasıyla, önce x), yay değeri sayısı, sonlu yay değerleri.
- **Neden bu sıra:** kendi başına bozuk girdi çizime bakılmadan reddedilir. Çizim girdinin hazırlandığı andan sonra değiştiyse katmanın şimdiki hâli çağıranın gördüğü hâl değildir; çakışma katmandan önce gelir.
- Kilit ve görünürlük üst gruplardan miras alınır; ileti katmanın kendi adını verir (bugünkü gibi).
- **Denetlenmeyen:** geometrik geçerlilik (kendini kesen halka, sıfır alan, dışarıdaki delik). Çizim araçları ve dosya okuyucusu da denetlemiyor. Topoloji ve onarım ayrı komutlardır (`NUM-10`).
- **Hesap yok:** denetimler sayım, sonlu sayı ve katman ağacıdır. TS'te geometri yazılmadı (CLAUDE.md §4.8.1). Aracın iletisindeki alan önceki gibi ortak çekirdekten gelir (web WASM `bulgeRingArea`, masaüstü `bulge_ring_area`).
- Girdi olduğu gibi saklanır: yay değerleri sıfır da olsa, delik listesi boş da olsa. Gizli sadeleştirme yok (CLAUDE.md §23.3). Belge çağıranın dizilerini değil kendi kopyasını tutar.

### Yürütme bağlamı (`CMD-06`)

- Yerel `ExecutionContext` yalnız açık belgedir: tenant, proje, aktör, yetki yok; belge kullanıcının kendisinindir.
- Bulut host'u aktörü ve hakları doğrulanmış oturumdan ekleyecek, girdiden asla. `CommandEnvelope` sunucunun tel biçimi olarak kalır.
- validate ve plan belgeyi yalnız okur (web'de tip düzeyinde değil, sözleşmeyle; fixture denetler).

### Örtük durum (`CMD-07`)

- Etkin katman `layerId` olur; web'in güncel rengi `color` olur. Aynı Python ya da AI çağrısı başka bir etkin katman yüzünden başka yere yazmaz.

### Masaüstü: `crates/native/application`

- Paket `kentos-native-application`, crate `kentos_native_application`. TODOS.md §2.1'deki `native/application` yeridir (“native command/use case/port”). `kentos-application` adı sunucunun kullanım durumlarınındır; sunucu paketi taşınmadı.
- Saf Rust, yalnız native. Bağımlılıkları `kentos-contracts` (varsayılan özellikler kapalı) ve `kentos-domain`. Geliştirme bağımlılıkları `serde_json` ve katalog testi için sözleşmelerin `schema` özelliği (sunucunun katalog testi gibi); gönderilen ikiliye girmez.
- `scripts/arch/deps.mjs`'te grubu `application`'dır: `shared` ve `domain`'i kullanır; `interaction` ve `desktop` onu kullanabilir. Çalışma zamanı, tarayıcı bağları, Iced/wgpu, PyO3, GDAL, PROJ altında bulunamaz. `default-members`'tadır.
- `DESKTOP_COMMANDS` masaüstünün çalıştırdığı komutlardır; `tests/catalog.rs` onu katalogdaki `desktop` komutlarıyla eşit tutar (sunucunun `SERVER_COMMANDS`'ı gibi).
- `polygon::{validate, plan, execute}` ve `polygon::codes`.

### Web: `apps/web/src/product`

- `ProductCommand<Girdi, Çıktı, Plan>`: `id`, `version`, `validate`, `plan`, `execute`. `ExecutionContext { doc }`. `WEB_COMMANDS` ve `findProductCommand`.
- `registry.test.ts` kaydı `commandCatalog.json`'daki `web` komutlarıyla eşit tutar.
- Katman yeri `model`'den sonra, `tools`'tan öncedir: belgeyi alır, DOM ve arayüz bilmez; araçlar onu çağırır. `polygonCreate.ts` modele yalnız tip olarak bağlıdır.
- Arayüz komut kaydı (`core/commands.ts`) olduğu gibi kaldı (ADR 0013, “İki düzey”).

### Ortak durumlar: `fixtures/commands/v1`

- Biçim `kentos.command-cases` v1, `fixtures/commands/README.md`'de. `cad.polygon.create.json`: 23 durum, 63 adım.
- Her durum bir `.kcad` v1 çizimini uygulamanın açtığı yoldan açar; adımlar komutu bir kipte çalıştırır (`validate`, `plan`, `execute`), geri alır, yineler, sürüm ya da kimlik saklar.
- Sonuç **tamamıyla** karşılaştırılır: durum, kod, yol, ileti kelimesi kelimesine, uyarılar; tamamlanmışta çıktı. Yürütmeden sonra belge: nesneler, nesnenin alanları (kalıcı kimlik hariç), geri al/yinele, kirli bayrağı, sürüm, kimlikler.
- Sürüm ve kimlik adla yazılır: `$current`, `$<ad>`, `$uid` (yazılan nesnenin kalıcı kimliği ve o yuvadaki nesnenin kimliği).
- JSON NaN ve ±∞ taşıyamaz: `nonFinite` tablosu girdi okunduktan sonra yola değeri koyar.
- Kapsam: başarı; yay değerleri; delikler, renk, öznitelik, grubun içindeki katman; gizli katman ve gizli grup uyarısı; kilitli katman ve kilitli grup; bilinmeyen katman (ad kimlik değildir, boş kimlik); grup; üçten az köşe (dış halka, delik); NaN ve ±∞ (köşe, delik köşesi, yay değeri, delik yay değeri); yay değeri sayısı (eksik kapanış, fazla, boş liste, delik); beklenen sürüm tutar; çakışma ve yeniden hazırlama; yazımı bozuk sürümler; denetim sırası; doğrulama ve plan yazmaz (geçmiş ve yineleme dahil); plan → sürümüyle yürütme; tek adımda geri alma ve aynı kimlikle yineleme; her yeni alan yeni kimlik.
- Koşucular: web `apps/web/src/product/fixtures.test.ts` (`CadDocument`), masaüstü `crates/native/application/tests/fixtures.rs` (`kentos_domain::Document`).
- **Kural:** beklenen değerler bu ADR'den elle yazıldı, iki koşucuyla doğrulandı; bir uygulamanın çıktısından kopyalanmadı.

### Araçlar komuttan yazar

- **Web:** `PathTool` kapalı alanda (`closed`, parsel ve ölçme değil) `polygonCreate.execute`'u çağırır; girdiye etkin katmanı ve güncel rengi koyar. Komutun reddini ya da uyarısını `warn` düzeyinde, sonra alanı `success` düzeyinde yazar.
- **Masaüstü:** `Polygon::create` `polygon::execute`'u çağırır, iletileri aynı sırayla söyler. `slots_exhausted` bugünkü gibi `error` düzeyindedir.
- **Değişmeyenler:** iletiler, sıraları, geri alma adı “Ekle”, tek adım, araç sonrası durum. İzler (`fixtures/interaction/v1`) değiştirilmeden iki platformda geçer.
- **Fark (yalnız ulaşılamayan yolda):** etkin katman çizimde yoksa araç önceden sessizce hiçbir şey yazmıyordu; şimdi komutun `layer_not_found` iletisini söyler. Etkin katman iki platformda her zaman var olan bir katmandır.
- **Doğrudan yazmaya devam edenler:** `PathTool`'un parsel (numara, tapu alanı özniteliği, seçim), çoklu çizgi ve ölçme biçimleri, öbür bütün araçlar. Her biri kendi ürün komutuyla taşınacak. 26 Eylül: çoklu çizgi ve çizgi aracı da komuttan yazıyor ([ADR 0027](0027-line-and-polyline-commands.md)).
- **Envanter:** araç kataloğu aracın vardığı ürün komutunu söyler (`ToolDescriptor.productCommand`); envanterde `tools[polygon].productCommand = "cad.polygon.create"` (ADR 0013'ün eşleme kuralı, `AI-01`'in ilk satırı).

### Ortak olan, ortak olmayan

| | Web | Masaüstü | Ortak |
|---|---|---|---|
| İşleyici | `product/polygonCreate.ts` (TS) | `native/application/src/polygon.rs` (Rust) | — |
| Kayıt | `WEB_COMMANDS` | `DESKTOP_COMMANDS` | katalog (`commandCatalog.json`), eşitlik testleri |
| Girdi, çıktı, plan, sonuç | üretilen TS tipleri | `kentos-contracts` | Rust sözleşme tipleri, JSON Schema |
| Davranış, kodlar, iletiler | | | `fixtures/commands/v1` |
| Belge | `CadDocument.add` | `Document::add` | `fixtures/document-ops/v1` (ADR 0020) |
| Araç | `PathTool` | `kentos_interaction::polygon` | `fixtures/interaction/v1` |

## Sonuçlar

- **Testler:**
  - `kentos-contracts`: sonuç zarfının tel biçimi (altı durum, okunup geri yazılır) ve katalog: örnek girdiler girdi tipine, örnek çıktı çıktı tipine uyar.
  - `kentos-native-application`: katalog eşitliği; 23 ortak durum; yalnız masaüstünde olabilen iki test: yuvası bitmiş belge (`slots_exhausted`, hiçbir şey yazılmaz) ve işlem içinde yürütme (işleme katılır, tek adım, sürüm işlem bitince değişir).
  - `kentos-interaction`: gizli katmanda uyarı sonra başarı iletisi (17 test).
  - Web: 23 ortak durum, dosya biçimi, kayıt eşitliği (26 vitest).
- **Bağımlılıklar:** yeni paket yok. `Cargo.lock`'a yalnız yeni crate'in satırları girdi: 631 paketten 632'ye, çalışma alanının kendi crate'leri 17'den 18'e.
- **Ters deneme:** kalıtılan görünürlük bilerek bozuldu (katmanın kendi `visible`'ı okundu). İki koşucu da tam `gizli grubun katmanı da gizlidir; uyarı katmanın adını verir` durumunda düştü: uyarı beklenirken boş liste geldi. Geri alındı.
- ADR 0013'ün uyum denetimi maddesi (web ve native kayıt eşitliği) bu dilimle kapandı.

## Ertelenenler

- **`CMD-08` tekrar oynatma kaydı:** komut adı ve sürümü, şema, algoritma ve sayısal politika sürümleri, kaynak sürüm. Bu komut hesap yapmadığı için kayda eksik bir şey girmez; kayıt ilk hesaplı komutla gelir.
- **`CMD-09` plan özeti:** plan bugün yalnız sürümüyle bağlanır. Onay gerektiren komutlarda planın özeti (hash) ve onayın yalnız o plana uygulanması.
- **Sunucu host'u:** `cad.polygon.create`'in sunucuda çalışması (`feature.write`, `project.changes` ile ilişkisi, `TX-03` ters komutu).
- **Arayüzsüz giriş:** JSON'dan tipsiz çağrı (bilinmeyen ad ve sürümün reddi, girdi ayrıştırma hatası, `CMD-10`), Python sarmalayıcısı (`PY-16`), AI araç şeması (`AI-02`). Hata kodlarının ve plan şemasının katalogda listelenmesi (`AI-02`).
- **`needs_input`, `queued`, `cancelled`:** tipte var, bu komut üretmez.
- **`ARCH-07`'nin kalanı:** sunucunun `ApiError`'unun bu biçime geçmesi, yeniden deneme bilgisi.
- **`TX-02`:** okuma kümesinin sürümü; bugün tek belge sürümü karşılaştırılır.
- Öbür araçların ürün komutları: parsel, nokta … Çizgi ve çoklu çizgi 26 Eylül'de geldi (`cad.line.create`, `cad.polyline.create`; ADR 0027).

## Doğrulama (26 Eylül 2026, Linux; main `f6fb65b` üstünde)

- `cargo fmt --all --check` temiz.
- `pnpm rust:test`: 442 test geçti, clippy temiz, bağımlılık yönü temiz (18 crate, 23 crate × hedef).
  - Veritabanı testleri yerel sunucuda çalıştı: `KENTOS_TEST_DB=required cargo test -p kentos-application -p kentos-postgres -p kentos-api` 50 test geçti. Sunucunun katalog eşitliği yeşil.
- `pnpm rust:test:desktop`: masaüstü 27, render 27, KentOS UI 170, vitrin 55 test geçti; clippy temiz. Native iz oynatıcısı (`cargo test -p kentos-desktop traces`) 4 iz × 3 varyant geçti.
- `pnpm typecheck` temiz. `pnpm test`: 915 geçti, 13 atlandı (başlangıçtaki 13).
- `pnpm e2e:interaction`: 4 iz × 3 varyant geçti.
- `pnpm inventory:check` güncel.
