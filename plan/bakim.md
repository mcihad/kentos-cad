# KentOS — Bakım planı: kod incelemesi ve refactor

Başlangıç: **28 Kasım 2026**, `main` `59d451456`. Sahibin isteği: “Bu projenin daha çok yolu var, o yüzden
yönetilebilir ve sürdürülebilir bir yapısının olması şart; hem teknik hem kod kalitesi hem de harness için durumunu
incele ve yapmamız gerekenleri bir plan haline getir. Önce kodlarımızı review ederek refactor edelim, daha sonra da
diğer adımlara geçeceğiz.”

Planın ilk sürümü dosya boyutlarından ve belgelerden çıkarılmıştı; sahip reddetti (“kodu çok iyi incelemedin”). Bu
sürüm kodun okunmasına dayanır: her bulgunun dosyası ve satırı yazılıdır. Satır numaraları `59d451456`'nındır; biçim
tabanından (BK-01) sonra kayar.

Kararlar: [ADR 9400](../docs/adr/9400-maintenance-plan.md). Bu planın ADR'leri **9400**'den başlar.

Sahibin kararları (28 Kasım 2026):

- Masaüstü çekirdeği crate'lere bölünmez; önce katmanlar arası bağlar azaltılır (24 Kasım kararı sürer, ADR 9199 §5).
- Denetim: tek komut (`make guard`) ve Claude Code hook'u. CI ve git hook'u yok.
- Biçim: `cargo fmt --all` şimdi, tek commit.
- `.kcad`: şema zinciri dondurulur; yeni alanlar modül sürümüne ya da adlı zorunlu uzantıya yazılır.
- Katalog: donmuş web envanteri ile ekler tek masaüstü kataloğunda birleştirilir.

Sahibin ikinci kararı (28 Kasım 2026, yürütme sırasında; [ADR 9402](../docs/adr/9402-web-and-wasm-removed.md)):

- Web uygulaması ve WASM desteği sistemden tamamen kaldırılır; dosyaları silinir. İhtiyaç olursa ileride yeniden
  eklenir.
- TypeScript yalnız KentOS Bulut (portal) için üretilir.
- `jsmath` (JavaScript'in sayı davranışının kopyası) yerine CAD için doğru sonuç veren bir sayısal temele geçilir.
- Bu iş Aşama 1'in ardından, Aşama 2'den önce yürütülür (Aşama 8).

Kural: refactor davranışı değiştirmez; doğruluk düzeltmesi ayrı commit'tir ve düzeltmeden önce kırmızı olan bir
sınamayla gelir. Her dilim bir commit; önce o crate'in hedefli sınaması, sonra `make guard`. Yürütme sırayla, tek
elden; alt ajan için ayrıca sorulur.

## Nasıl incelendi

- **Okunan omurga:** `apps/desktop/src/shell/app.rs` (tamamı), `shell/app/update.rs`, `shell/app/commands.rs`,
  `shell/module_parts.rs`, `shell/catalog.rs`, `apps/desktop/src/lib.rs`; `native/application/src/lib.rs`,
  `core/registry.rs`, `core/dispatch.rs`, `core/desktop_commands.rs`; `native/headless/src/file_commands.rs`;
  `commands/src/catalog/{gis,mod}.rs`; `kcad/src/encode/schema.rs`, `decode/objects/features.rs`;
  `cad/history/text.rs`.
- **Bütün depoda ölçülen:** işlev boyları (57 862 üretim işlevi), tür boyları, 30 günün git geçmişi (2 093 commit),
  crate bağımlılık grafiği (85 crate), `cargo fmt --check` (yalnız denetim), kalıp sayımları.
- **Üç salt okunur inceleyici:** masaüstü özellik kodu ve arayüz kiti; belge, komut ve geometri çekirdekleri; dosya
  biçimleri ve veri sürücüleri. Her bulgu kodda yeniden açılıp doğrulandı; doğrulanmayan “inceleyiciye göre” diye
  işaretli.
- **Yapılmayan:** derleme, clippy ve sınama koşulmadı. Sunucu, bulut istemcisi, eklenti konağı ve imza
  (`crates/server`, `apps/api`, `native/cloud`, `crates/imza`) yalnız yüzeyden bakıldı. İnceleyicilerin okumadığı
  yerler: raster ve nokta bulutu biçimleri, stil tasarımcısı, `cloud/portal`, çoğu Harita ve Mimari penceresi (BK-72).

## Ölçek

| Konu | Ölçüm |
|---|---|
| Rust | 6 581 dosya, 2 575 064 satır (541 683'ü sınama yolunda) |
| Python | 1 510 dosya, 747 745 satır: 847 üreteç 408 645 satır; SDK 259 634 satır (153 732'si üretilen `types.py`) |
| En büyük crate'ler | masaüstü çekirdeği 585 bin, `native/application` 276 bin, `geometry-core` 180 bin, `interaction` 113 bin |
| 30 günlük hız | 2 093 commit, ortanca 12 dosya; Rust'ta 1,41 milyon satır değişti; commit'lerin %70'i `apps/desktop`'a dokundu |
| İşlevler | 2 209'u 100 satırı, 554'ü 200'ü, 115'i 400'ü, 25'i 800 satırı geçiyor |

## Bulgular

### A. Doğruluk

- **A1 — Yarım yazılan ve geri alınamayan iş.** `application/src/arch/permit_set.rs:796–798` ayarları yazar, ardından
  reddedebilen `pages()`'i çağırır (`:799–807`); `:812–814` görünümleri aynı yolla yazar. Ayarlar geri alma adımı
  değildir. Masaüstü `Err` gelse de grubu kapatır (`shell/exchange/permit_set.rs:273–277`).
- **A2 — Bir Kaydet, birkaç bağımsız yazma.** `gis/forms/relations.rs:326–346` her değişikliği ayrı komutla yazar;
  biri reddedilirse öncekiler yazılı kalır. İnceleyiciye göre `gis/joins_window.rs:404–421` ve
  `gis/event_links_window.rs:323–337` aynı kopya. Doğrusu yan dosyada: `gis/forms/domains.rs:488–515`
  (`begin_group` / `cancel_group`). `cad/leader_tools.rs:511–516` yalnız `Failed`'a bakar; `Conflict` ve
  `NeedsInput` “kaydedildi” sayılır.
- **A3 — Geç gelen sonuç başka pencereye düşüyor.** `shell/exchange/csv_exchange.rs:327–329`: arka plandaki yazma
  bitince koşulsuz `close_exchange()`; o sırada açık olan başka aktarım penceresi kapanır, hata ise pencere gitmişse
  kaybolur. İnceleyiciye göre FlatGeobuf, GML, GeoParquet ve GPKG karo aktarımlarında da aynı.
  `desktop-arch/src/ifc_compare_window.rs:322–338` koşu kimliği taşımaz. Doğru kalıp var: `App::document_reply`
  (`shell/input.rs:168–185`).
- **A4 — Arayüz iş parçacığında ağır iş.** `desktop-arch/src/bcf_window.rs:104–110` BCF'yi ve IFC modelini tıklamanın
  içinde okur. Form tasarımcısı her olayda bütün penceresini kopyalar (`gis/forms/designer/step.rs:8`) ve her kirli
  olayda taslağı diske eşzamanlı yazar (`step.rs:1611–1622`); yazma hatası yutulur (`gis/forms/library.rs:173`).
- **A5 — Komutu ve kilitli katman kuralını atlayan yazmalar.** `application/src/arch/fire_marks.rs:56–64` ve
  `arch/color_legend.rs:42–46` işaret taşıyan bütün nesneleri doğrudan `doc.remove` ile siler; katalog kimlikleri
  yok, hata türleri `String`. İnceleyiciye göre uygulama katmanında 14 böyle yazıcı var. Masaüstünde stil yöneticisi
  `doc.set_styles` ile yazar (`cad/style/mod.rs:138–147`); komutu `cad.library.edit` var ama tek dosya kullanıyor.
- **A6 — Boy sınırı yüzünden yanlış hata kodu.** `application/src/arch/masses.rs:16` ve `:35` kütle için
  `INVALID_SPACE` (mahal) kodunu verir; gerekçesi dosyanın başında (`:1–3`): “the code table is at its size”.
- **A7 — İçe alınan metinde sınırsız özyineleme.** `formats/src/sld.rs:304–353` (`parse_qgis` → `or_of` → `not_of`) ve
  `expression/src/parser.rs:392–416` derinlik saymaz; iç içe parantezli bir stil dosyası yığını taşırır. Sayaç örneği:
  `formats/src/json.rs:135`, `kcad/src/cbor.rs:385`.
- **A8 — Sessiz veri kaybı.**
  - `maplibre/src/lib.rs:2253–2275` 15 renk adı tanır; tanımadığı siyaha döner (`:2773–2775`).
  - `services/src/gml.rs:43–48` okunamayan sayıyı atlar; sonraki bütün koordinatlar kayar.
  - `formats/src/dxf/lexer.rs:15–17` UTF-8 olmayan metni boş döndürür; tarama deseni adı (`dxf/hatch.rs:321`), geçiş
    adı (`:361`) ve resim yolu (`dxf/ordering.rs:46`) kod sayfası çözücüsünden geçmez.
  - `pointcloud/src/write.rs:151–153` 64 KiB'ı aşan VLR'yi bildirmeden atlar.
- **A9 — Kullanıcının sorgusu `execute_batch`'e giriyor.** `native/data-gpkg/src/query.rs:33`.
- **A10 — Aynı hesabın sağlam ve sağlam olmayan kopyaları.** Çekirdek nokta-alan sınamasını kesin yönelimle yapar
  (`geometry-core/src/geometry.rs:110`); yuvarlanmış kesişimli eski biçim en az altı yerde durur
  (`labels/geom.rs:162`, `domain/src/room_width.rs:106`…; inceleyici farklı yazımlarla 15 yer saydı). Alan hesabı ilk
  köşeye kaydırılır (`geometry.rs:45–62`); kaydırmayan kopya `geom/column_join.rs:318`. Gösterim yarımı sıfırdan uzağa
  yuvarlar (`display.rs:45–47`), `format!("{v:.2}")` çifte: inceleyiciye göre çizime yazan dört yer bunu kullanır
  (`arch/color_legend.rs:23`, `arch/fire_marks.rs:37`, `core/block_values.rs:41`, `core/profiles.rs:566`).
- **A11 — Arayüz metninde başka uygulama adı.** `shell/exchange/csv_exchange.rs:524`, `shell/app_menu/mod.rs:154`,
  `crates/sheet-ui/src/export_pdf.rs:32`.

### B. Aynı olgu elle birçok yere yazılıyor

Kalıp: liste kopyalanır, bir sınama iki kopyayı eşit tutar. Sapma yakalanır ama her komutta bütün kopyalar elle
düzenlenir.

- **B1 — Başlıklar iki kez.** `apps/desktop/src/cad/history/text.rs:14` `TITLES: [(&str, &str); 505]`: 505 çiftin
  505'i katalogdaki başlıkla aynı. Dosya 30 günde 147 commit aldı.
- **B2 — Ürün komutu 11–15 yerde.** `cad.entities.delete` için: sabit (`contracts/src/core/cad.rs:422`), katalog
  tanımı (`commands/src/catalog/drawing.rs:205`), kimlik → girdi türü (`catalog/example_inputs.rs:86`; dosyada 517
  kol), kimlik → çıktı türü (`catalog/mod.rs:2233`), yetenekler (`capabilities/drawing.rs`), dağıtım
  (`application/src/core/dispatch.rs:11`), `DESKTOP_COMMANDS` (`core/desktop_commands.rs:28`), `core/registry.rs:1–330`
  `use` listeleri, `lib.rs`, sınama koşucusu (`tests/all/fixtures/run_core.rs`, `inputs_core.rs`). Kimlik ↔ girdi türü
  dört kez elle yazılıyor. İşleyicinin sözleşmesi trait değil; makro `validate/plan/execute` adlarını arar
  (`registry.rs:527–537`).
- **B3 — İşleyici iskeleti her komutta yeniden.** `Err(stop) => stop.into()` 402 dosyada 861 kez; `fn write_execute`
  158 kopya; yerel `fn fail(` 129 kopya. Doğru parçalar var: `call::recorded`, `checks::*`, `create_command!`
  (`arch/arch_elements.rs:1799`).
- **B4 — Masaüstü komutu yedi yerde.** `shell/catalog.rs:14–28` kataloğu donmuş `docs/inventory/web.json` (603 komut),
  `catalog/v1/additions.json` (437), `ribbon.json` ve elle tutulan `operations.json`'dan kurar; üstüne
  `DESKTOP_TITLES/SHORTS/ICONS` yamaları (`:39–58`), `shell/catalog/ported.rs` `PORTED` (913 satır) ve `App::run`'ın
  metin kolu.
- **B5 — Araçlar iki kez.** `interaction/src/core/session.rs` araç kimliklerini `TOOLS`'ta (`:76`) ve `make()`'te
  (`:528`) ayrı ayrı sayar.
- **B6 — `.kcad` şeması.** 28 Eylül'de 3, 28 Kasım'da 257: `kcad/src/schemas.rs` 254 sabit; `encode/schema.rs:42`
  `schema_of` sürüm başına elle bir koşul (1 836 satır); `decode/objects/features.rs:8` sürüm başına bir `bool` (254
  alan). Her sürüm ayrıca belirtimi, iki Python karşılığını (`tools/kcad/kcad.py` 14 314 satır,
  `scripts/fixtures/kcad_v2_reference.py` 8 352 satır) ve ~21 sınama dosyasında birer satırı değiştirir. Biçimde adlı
  zorunlu uzantı (belirtim §3.6) ve modül sürümü (§11) var; ilki hiç kullanılmıyor.

### C. Masaüstü kabuğu: her özellik aynı merkezî dosyaları düzenliyor

- **C1 — Boyutlar.** `shell/app.rs`: `App` 236 alan (`:745–1219`), `Message` 187 kol (`:368`), `Dialog` 88 kol
  (`:157`); `update.rs:516…` `handle` 191 kol; `App::close_dialog` bulut klasöründe (`shell/cloud/mod.rs:982`), 60'tan
  çok kolla ilgisiz özelliklerin durumunu temizler.
- **C2 — Bir pencere = 7 ortak dosyada 10 elle düzenleme.** 221 satırlık `cad/flatten_window.rs` için: `lib.rs:56`,
  `app.rs:275`, `:547`, `:1057`, `app/commands.rs:413`, `app/update.rs:737`, `traces/answers.rs:151` ve `:301`,
  `view.rs:331`, `history/text.rs:45`.
- **C3 — Komut dağıtımı metinle ve sıraya bağlı.** `App::run` (`app/commands.rs:14`, 708 satır): sırası anlam taşıyan
  ~20 denetim (`:43–143`), sonra 328 metin kollu `match`. 13 özelliğin `x_command(id)` eşleşmesi `_ => {}` ile biter
  (`cad/views.rs:130–145`): kolu unutulan komut sessizce hiçbir şey yapmaz.
- **C4 — Her iletiden sonra her şey yoklanıyor.** `update` (`update.rs:182–328`) fare hareketinde de ~20 `follow_*` ve
  ~38 `*_tasks()` çağırır; sıra yorumla tutulur (`:297–303`). Ertelenmiş işler altı ayrı alan (`app.rs:831–841`).
- **C5 — Klasörler Rust modülü değil.** `apps/desktop/src/lib.rs` 290 modülü 289 `#[path]` ile kökte bildirir;
  `native/application/src/lib.rs` 630 modül, 625 `#[path]`, 367 `cfg`. İki `lib.rs` en çok değişen Rust dosyaları (339
  ve 255 commit). Kabukta modül adına ~36 dal var (`shell/start.rs:164–181`, `shell/view.rs:857–861`).
- **C6 — Çözümün yarısı var.** `ModuleParts` (`shell/module_parts.rs:138–198`) çalışıyor: modül paketi kabuğa
  dokunmadan pencere ekliyor. Ortak CBS katmanı da gruplu (`App.gis`, `Message::Gis`, `Dialog::Gis`). CAD pencereleri
  (~60) `App`'e düz yazılı. Eksikleri: `ModuleParts`'ın 27 yönteminin öntanımlı gövdesi yok; modül paketlerinde 103
  uzantı trait'i, bazıları sayıyla bölünmüş (`RoadExchangeApp1/2/3`, `desktop-road/src/road_exchange.rs:16,106,156`).

### D. Biçim ve boy bekçisi gerçeği ölçmüyor

- **D1.** `rustfmt.toml` yok; `cargo fmt --check` 6 581 dosyanın 3 010'unu değiştirmek istiyor (net +162 105 satır).
- **D2.** Tabandaki 69 dosyanın 54'ü biçimsiz; sınırın altında görünen 19 dosya biçimlenince sınırı geçer. Gerçek
  sayı 88. Masaüstü çekirdeğinin kendi sınaması tabansızdır (`apps/desktop/src/tests/source_size_tests.rs`).
- **D3 — Dev işlevler.** `commands/src/catalog/gis.rs:7` 3 531 satır, `catalog/road.rs:7` 2 628, `catalog/mod.rs:141`
  1 894; `kcad/src/encode/schema.rs:42` 1 836; `ifc/src/arch.rs:814` `building` 1 794;
  `gis/forms/designer/step.rs:7` `forms_step` 1 620; `cad/properties/commit.rs:295` `commit` 1 561;
  `application/src/arch/regulation_audit.rs:532` 1 175; `server/application/src/changes.rs:1277` 980.
- **D4 — Üç bekçi kırmızı.** `module_deps.py` (kabuk → cad 77 → 79, gis → cad 54 → 55), `source_size.py`
  (`catalog/gis.rs`, `catalog/mod.rs`), `scripts/arch/deps.mjs` (`kentos-standards → kentos-launch`).

### E. Sınama altyapısı

- **E1.** `crates/native/mcp/tests/` 166 kök dosya, `autotests = false` yok: 316 sınama için 166 ikili bağlanıyor. 162
  dosya kendi `fn call(…)`'ını tanımlıyor, 133'ü aynı. `crates/server/application/tests/` 41 kök. Öbür crate'ler tek
  ikili kullanıyor (`application/tests/all/main.rs`). `target/debug` 186 GB.
- **E2.** Üretim kodunda `unwrap/expect` ~900 (sınamalar dışında); `native/application` lint ile yasaklıyor
  (`lib.rs:44–48`). Yoğunluk iki dosyada: `geometry-core/src/store/walls.rs` 67, `store/inserts.rs` 42.

### F. Tekrarlanan alt düzey kod

- **F1 — Sayı yazımı ve okuması.** `.replace('.', ",")` 91 dosyada 149 yerde; `fn grouped` 14 tanım; `fn number(text)`
  40 tanım; masaüstünde 392 `{:.N}`. Ortak biçimleyici var: `native/domain/src/format.rs`.
- **F2 — Vektör ve tolerans.** `Vec2`'nin yalnız `new`'u var (`geometry-core/src/vec2.rs`); çekirdek crate'lerde yerel
  `sub` 23, `dot` 25, `cross` 18, `dist` 17 tanım. `1e-9` 988, `1e-12` 427, `1e-6` 257 kez sabit olarak yazılı.
- **F3 — Biçim crate'lerinde.** XML kaçışı en az beş kopya, yalnız biri denetim karakterlerini ayıklar
  (`formats/src/wgs84.rs:18`; öbürleri `landxml_write.rs:147`, `sld.rs:447`, `ifc/src/bcf.rs:131`,
  `svg-core/src/export.rs:24`). İnceleyiciye göre onaltılık renk, base64, CRC-32 ve bayt okuyucu da çok kopyalı; iki
  PostGIS sürücüsü (`native/postgis`, `native/data-postgis`) kendi `quote`/`literal`'ını tutuyor; DXF okuyucu dosyayı
  6–8 kez geziyor.
- **F4 — Masaüstünde.** 76 renk sabiti (aynı yeşil altı yerde, `desktop-road/src/corridor_window.rs:1131`…); 42 sabit
  `Pixels(N)`; komut sonucu elle eşleniyor (`App::answered` var; inceleyiciye göre dokuz yerel kopyası). Yedi kit
  bileşenini yalnız vitrin kullanıyor (`RadialMenu`, `Timeline`, `Viewports`, `MiniToolbar`, `ObjectBrowser`,
  `MaterialBrowser`, `PropertiesDialog`).
- **F5 — Hata türü metin.** Üretim kodunda 3 179 `Result<_, String>` imzası; üç hata kodu tabloda yok
  (`core/changeset.rs:240`, `:245`, `core/drawing_inspect/mod.rs:196`).

### G. Üretilen dosyalar, belgeler, harness

- **G1.** Canlı üretim çıktısı donmuş web'in içinde: `apps/web/src/contracts/generated/` 3 899 dosya, 30 günde 646
  commit. `.cargo/config.toml` `TS_RS_EXPORT_DIR`, katalog sınaması (`commands/src/catalog/mod.rs:2170`), SDK üreteci
  ve portal (`apps/portal/src/api/types.ts:3…`) oraya bağlı.
- **G2.** `docs/kilavuz/00-guncel-karar-ozeti.md` 5 053 satır, 30 günde 662 commit; `TODOS.md` 592. Commit'lerin %76'sı
  `docs/adr`'ye dokunuyor.
- **G3.** Projede `.claude/settings.json`, skill, hook, ajan tanımı yok. Kökte beş dağınık inceleme dosyası.

## Maddeler

### Aşama 0 — Zemin

- [x] **BK-01 — Biçim tabanı** (D1, D2; [ADR 9400](../docs/adr/9400-maintenance-plan.md) §2–3). `cargo fmt --all`
  `27d390b97` (3 008 dosya), `.git-blame-ignore-revs`. Boy tabanı gerçek sayılarla yeniden yazıldı (69 → 87 dosya);
  masaüstü çekirdeğinde sınırı geçen `cad/surface_analysis_window.rs` üç dosyaya bölündü. `module_deps.py` çıktısı ve
  Rust kaynağını okuyan üreteçler önce ve sonra aynı; `cargo build -p kentos-cad` derler.
- [x] **BK-02 — Bekçiler yeşil** (D4). `standards → launch` bağı kurala yazıldı (CLAUDE.md §26). Üç yeni bağın kökü
  `cad/labels`'taki çizici altyapısıydı; `shell/glyph_layer.rs`'e taşındı. Kabuk → gis 42 → 41.
- [x] **BK-03 — `make guard` ve hook.** `deps.mjs`, `source_size.py`, `module_deps.py --check`,
  `scripts/cad/manifest.py --check`, `cargo fmt --all -- --check`; Claude Code `Stop` hook'u
  (`scripts/dev/guard_hook.sh`) onu koşar. Başka uygulama adı bekçisi masaüstü sınamasıdır, derleme ister, girmez.
- [x] **BK-04 — Sınama ikilileri** (E1; ADR 9400 §5). MCP 166 → 4 program (tek program 313 sınama, 126 sn; süreç
  değişkeni değiştiren üç sınama ayrı), sunucu 41 → 2 (167 geçti, 4 yok sayıldı, 62,7 sn; `disk_full` ayrı). Bir MCP
  sınama programı 0,88 GB: 166 program ≈ 146 GB, 4 program ≈ 3,5 GB. Sınama yığını artık `.cargo/config.toml`'dan
  gelir. Kalan: ortak `call` yardımcısı (BK-53).
- [x] **BK-05 — Ölçüm tabanı** ([docs/perf/build-bakim-2026-11-28.md](../docs/perf/build-bakim-2026-11-28.md)).
  Clippy'nin 41 uyarısı giderildi, iki sınama komutu temiz. Tek satır değişiklikte derleme: modül paketi 3,7 sn,
  masaüstü çekirdeği 7,9 sn, uygulama katmanı 13,6 sn, geometri çekirdeği 36,4 sn; sıfırdan 176 sn.

### Aşama 1 — Doğruluk düzeltmeleri

- [x] **BK-10** (A1; [ADR 9401](../docs/adr/9401-correctness-fixes-from-the-review.md) §1) İmar takımı önce planlanır,
  sonra yazar; paftalar reddedilirse ayarlar geri konur. Kalıcı çözüm takımın kendi komutudur (BK-19).
- [x] **BK-11** (A2; ADR 9401 §4) Dört pencerenin Kaydet'i (İlişkiler, Birleşik görünümler, Olay bağları, Kılavuz
  stilleri) bütün düzenlemeleri yazar ya da hiçbirini: tek yardımcı `App::save_together`, birden çok düzenleme
  `cad.changeset.apply` ile. `document::in_step` yetmiyordu: bu komutlar ayarları yazar, `transact` ayarları geri
  almaz. Değer listeleri penceresi zaten doğruydu.
- [x] **BK-12** (A3; ADR 9401 §5) Arka plan yanıtı soran pencerenindir. Beş aktarım değil ortak düzenek: aktarım
  pencerelerinin yanıtı pencerenin numarasıyla döner (`exchange::Runs`); geç yanıt başka pencereyi kapatmaz, söylediği
  ileti günlüğüne düşer. 17 hata kolu pencere gitmişse hatayı söyler. IFC karşılaştırması koşu numarası taşır.
- [x] **BK-13** (A4; ADR 9401 §6) BCF okuması kendi iş parçacığında, koşu numarasıyla. Form taslağı arayüz iş
  parçacığının dışında, aynı anda tek yazmayla yazılır; yazılamayınca bir kez söylenir. Kalan: `forms_step`'in her
  olayda pencereyi kopyalaması (BK-40).
- [x] **BK-14** (A7; ADR 9401 §2) İfade dili: en çok 64 düzey ve 1 000 işlem; stil süzgeci: 64 düzey. Ölçüm: 2 MB'lık
  iş parçacığında sürüm derlemesi 3 000 terimli zincirde, 1 000 parantezde taşıyordu.
- [x] **BK-15** (A8, A9; ADR 9401 §3) Beş düzeltme, her biri kırmızıdan yeşile dönen sınamasıyla:
  - DXF: tarama deseni, geçiş ve resim adları çizimin kod sayfasında okunur.
  - WFS: koordinatı okunamayan nesne şekilsiz gelir ve raporda ayrı söylenir.
  - LAS: VLR'ye sığmayan kayıt 1.4'te EVLR olur, 1.2 ve 1.3'te yazma reddedilir.
  - GeoPackage sorgusu: `execute_batch` kalktı; metin SQLite'a verilmeden önce onun kurallarıyla okunur. `prepare`
    tek başına yetmiyordu: ikinci deyimi derliyor, `PRAGMA` derlenirken etki ediyordu. Okuma SQLite'ın kendisiyle
    4 000 metinde karşılaştırılır.
  - MapLibre: CSS'in 148 renk adı okunur (tablo SVG çekirdeğinin); renk olmayan değer söylenir, katmanın kendi
    rengiyse katman alınmaz (ADR 0560'ın kuralı). Beş yeni ortak durum.
  - Açık kalan: SQLite okuma bağlantılarına yetkilendirici (ADR 9401 §3.4, seçilmeyen); üç CSS renk okuyucusunun
    birleşmesi (BK-52).
- [x] **BK-16** (A6; ADR 9401 §8) Hata kodu tablosu altı dosyaya bölündü (sırası ve üretilen katalog aynı);
  `invalid_mass` eklendi (`cad.masses.create`, `cad.arch.edit`). Aynı yeniden üretimde: sözleşme yorumlarındaki kitaplık
  adı kalktı ve bekçiye girdi; `cad.layers.migrate`'in anılmayan `invalid_option` kodu listelendi (yetenek sınaması
  kırmızıydı). Üretilen TypeScript dosyaları yenilenmedi (BK-25).
- [~] **BK-17** (A10; ADR 9401 §9) Yapılan: çizime yazan dört yer `display::fixed`'e bağlandı; nokta-alan kararı
  çekirdekte tek (`ray_crosses`, `point_in_ring`, `point_in_rings`; iki nokta türü için) ve 23 kopya ona bağlı;
  bağımsız başvuru `point_in_rings_cases.py` (239 durum). Ölçüm: çizim ölçeğinde ortak kenardaki orta noktaların
  %4,3'ü yuvarlanmış yazımda iki alana ya da hiçbirine düşüyordu.
  - Alan (§9.3): çekirdekte `ring_area`; ham koordinatla toplayan 39 yerin 36'sı ona bağlı (TM koordinatında
    10 m'lik kare 99,99976 çıkıyordu; 12 mm'lik halkanın yönü ters okunuyordu; çatının yüz eşikleri yuvarlamanın
    altında kalıyordu). Kalan üçü yerel koordinatlı, gerekçesi ADR'de.
  - Ağırlık merkezi (§9.4): ham koordinatla toplayan sekiz kopya bulundu ve çekirdeğin `ring_centroid`'ine
    bağlandı. TM koordinatında 10 m'lik karenin merkezi 3,6 m, küçük bir kolonunki 1,4 km kayıyordu. Yol'un koridor
    denetimi ve yüzey kapsamı çizimi, Mimari denetimin mahal noktası, gölge, çıkma izdüşümü, net genişlik, işlem
    araçlarının denetim olguları, etiket motoru. Üç sınama önce kırmızıydı.
  - NCZ okuyucusundaki kopya da bağlandı; crate'teki 61 ad atfı (ikisi açılış raporunun iletisinde) temizlendi.
  - Kalan: çizime dilin kendi yuvarlamasıyla yazılan sayı dört değilmiş. Tarama (bir yazının ya da özniteliğin
    yanında `{:.N}`) uygulama katmanında yirmi kadar yer daha gösteriyor: Yol'un çizim komutları (koridor, menfez,
    otokorkuluk, kavşak, istinat duvarı, kazı-dolgu, yüzey kapsamı), boru profili çizelgesi, ağ elipsleri,
    uzlaştırmanın öznitelikleri; etkileşim katmanında parsel aracının “±0.00 = …” yazısı. Tek tek okunmadı. Bu
    komutların ortak durumları var: üreteçleri aynı yuvarlamayı kullanıyorsa birlikte değişir (BK-50, BK-88).
  - Gerekçesiyle bırakılan üç kopya ADR'de yazılı.
- [x] **BK-18** (A11; ADR 9401 §7) Üç değil sekiz metin: bekçi arayüz kitlerinin crate'lerini taramıyordu. Metinler
  yeniden yazıldı; bekçi dört ad daha arar ve `crates/ui`, `crates/sheet-ui`, `crates/model-view-ui`'yi de tarar.
  - Açık: “Excel (.xlsx)” 27 yerde biçim adı olarak duruyor (sahibin kararı).
  - Sözleşmelerin belge yorumlarındaki kitaplık adı BK-16'da temizlendi.
  - İkinci tur (§7.1): bekçi artık bütün programların ve crate'lerin metin sabitlerini tarıyor (31 ad; ADR atfı
    uygulama, etkileşim ve işlem katmanlarında da). Çıkan yedi ad ve beş ADR atfı düzeltildi; dosya imzaları ve
    servis türleri gerekçeleriyle izinli listede. Yorumlardaki 800 kadar satır dokunulan dosyada temizlenir.
- [~] **BK-19** (A5; ADR 9401 §10) Ölçüm: uygulama katmanındaki 26 `doc.remove`'un 24'ü bir komutun denetiminden
  sonra. Yapılan (§10.1): dört aracın eski işaretleri (renk planı lejandı, kaçış yolları, gölge, vaziyet ölçüleri)
  kilitli katmandaysa silinmez ve yenisi çizilmez; tek yardımcı `marks::take_away`, silme `cad.entities.delete`'in
  işleyicisiyle. Dört sınama önce kırmızıydı.
  - Yapılan (§10.2): `cad.xrefs.edit`'in `detach`'i, yerleştirmesi kilitli katmandaysa reddedilir. Ortak durumu
    üreteçten; önce kırmızıydı.
  - Yapılan (§10.3): `road.project.rebuild`, nesnesi kilitli katmanda olan çizimi yeniden üretmez (`edited` ile de).
    Ortak durumu üreteçten; önce kırmızıydı.
  - Kalan: stil pencereleri (yönetici, tasarımcı, SVG düzenleyicisi, şablonlar; 20 çağrı) projenin kitaplığını
    `Styles::changed` → `doc.set_styles` ile bütün olarak yazar. `cad.library.edit` öğe öğe çalışır ve dokuz
    işleminde kategori yoktur: geçiş komuta kategori işlemleri eklemeyi ister (§9.2 zinciri), Aşama 2'den sonra.
  - Kalan: dört aracın ve imar takımının kendi komutu (Aşama 2'den sonra).
  - Stil tablolarını yazan pencereler (`style_tables::save_*`) kilide kendi bakıyor ve `cad.styles.edit` ile aynı
    kodu kullanıyor; plan ölçüleri aracı da kilide bakıyor. Onlar bulgu değil.

### Aşama 2 — Tek kaynak: kopyayı türetmeye çevir

- [ ] **BK-20** (B1) `history/text.rs` `TITLES` silinir; başlık katalogdan okunur.
- [ ] **BK-21** (B2, D3) Komut tek yerde bildirilir; `example_inputs`, çıktı eşlemesi, dağıtım, `DESKTOP_COMMANDS`,
  sınama koşucusu ve `use` listeleri ondan üretilir. Katalog tanımı komutun tipleriyle aynı dosyada, kendi işlevinde.
- [ ] **BK-22** (B3) İşleyici iskeleti: `Result → CommandResult` dönüşümü ve `checks::fail` ortak; `create_command!`
  genellenir.
- [ ] **BK-23** (B4) Masaüstü kataloğu tek dosyada; birleşmeden önceki ve sonraki kataloğun eşitliği sınamayla
  kanıtlanır. `PORTED` BK-32'nin işleyici tablosundan türer. Taban dosyası bugün `docs/inventory/web.json`'dır
  (web'in dondurulmuş envanteri; ADR 9402 §4): adı ve yeri bu maddede değişir. Ok ucu ve desen ikonlarının
  üreteçleri web'in dosyalarına yazdığı için silindi (son hâlleri `8e89a663d`'de); ikonlar tek katalogda tutulur,
  kuraldan yeniden üretmek gerekirse üreteç kataloğa yazacak biçimde geri getirilir.
- [ ] **BK-24** (B5) Araç listesi tek tablo.
- [x] **BK-25** (G1) Aşama 8'de yapıldı: BK-82 (komut kataloğu) ve BK-83 (TypeScript yalnız Bulut için).

### Aşama 3 — Masaüstü kabuğu

- [ ] **BK-30** (C2, C6) Pencere arayüzü: başlık, olay, görünüm, iz denetimi ve kapanış tek nesnede; çekirdek ve
  modül pencereleri aynı arayüzü kullanır. `ModuleParts`'a öntanımlı gövdeler.
- [ ] **BK-31** (C1) CAD katmanı CBS gibi gruplanır: `App.cad`, `Message::Cad`, `Dialog::Cad`; pencereler öbek öbek.
- [ ] **BK-32** (C3) Komut tablosu: `(kimlik, işleyici, açık mı, işaretli mi)`; `App::run`'ın zinciri ve 328 kolu
  tabloya döner.
- [ ] **BK-33** (C4) İleti sonrası kancalar açık bir listede; altı `*_wanted` alanı tek kuyruk.
- [ ] **BK-34** (C5) Gerçek modül ağacı: `shell/mod.rs`, `cad/mod.rs`, `gis/mod.rs`; `native/application`'da beş
  klasör modülü.
- [ ] **BK-35** (C5) Kabuktaki modül dalları profile taşınır; `module_deps.baseline.json` azalır.

### Aşama 4 — Dev işlevler ve dosyalar

- [ ] **BK-40** (D3) 800 satırı geçen 25 işlev doğal dikişlerinden bölünür, sonra tabandaki dosyalar. Sıra: `kcad`,
  `properties/commit.rs`, `forms/designer`, `ifc/arch.rs`, `regulation_audit.rs`, arayüz kiti.

### Aşama 5 — Ortak alt düzey yardımcılar

- [ ] **BK-50** (F1) Sayı yazımı ve okuması tek yerden.
- [ ] **BK-51** (F2) `Vec2` işlemleri ve adlı tolerans türü çekirdekte.
- [ ] **BK-52** (F3) XML kaçışı, renk, base64, CRC, bayt okuyucu birer tane; PostGIS'in eski sürücüsü yenisinin
  arkasına. Renk: CSS rengini üç kod okuyor (`svg-core/src/values/color.rs`, `services/src/style/color.rs`,
  `maplibre/src/colour.rs`); ad tablosu iki yerde (ilk ikisi). MapLibre okuyucusu adları BK-15'te SVG çekirdeğinden
  almaya başladı.
- [ ] **BK-53** (F4) Grafik paleti ve yazı boyu tema jetonu; komut sonucu tek yardımcı; kullanılmayan yedi kit
  bileşeni için sahibin kararı. MCP sınamalarının ortak `call` yardımcısı (133 kopya).
- [ ] **BK-54** (F5) Sınır hatalarına tür, önce dosya biçimleri; eksik üç kod tabloya.

### Aşama 6 — `.kcad` şema zinciri

- [ ] **BK-60** (B6) Araştırma, ADR, belirtim §3.6 ve §11 değişikliği. Yeni belge alanı adlı zorunlu uzantıyla, modül
  verisi modül sürümüyle gelir. Mevcut 2–257 dokunulmadan kalır; sabit sayısı bekçiye girer ve artamaz.

### Aşama 7 — Harness, belgeler ve kalan inceleme

- [ ] **BK-70** (G2) Kılavuz §0 alan başına dosyalara bölünür; `TODOS.md`'nin biten bölümleri arşive.
- [~] **BK-71** (G3) Proje skill'leri ve ajan tanımları. Yazıldı: `.claude/skills/dilim-kapat`, `sunucu-db-sinamasi`,
  `kod-incelemesi`; `.claude/agents/inceleyici.md` (salt okunur, Fable değil). Kalan: yeni komut skill'i (BK-21 tarifi
  değiştirince), resim çekme skill'i, kökteki inceleme dosyalarının `docs/devir/`'e taşınması.
- [ ] **BK-72** Sunucu, bulut istemcisi, eklenti konağı ve imza için aynı inceleme.

### Aşama 8 — Web ve WASM'ın kaldırılması; sayısal temel

Karar ve ölçüm: [ADR 9402](../docs/adr/9402-web-and-wasm-removed.md). Sıra bağımlılığa göredir: önce web'den bir şey
alan her yer kendi kopyasına ya da yeni yerine bağlanır, klasör en son silinir. Her madde bir commit; her birinin
sonunda derleme, ilgili sınamalar, portalın tür denetimi ve `make guard`.

- [x] **BK-80 — Portal kendi ayakları üstünde** (`5db25349f`). Üç TypeScript dosyası, üç stil dosyası ve yazı
  tipleri (13 aile) portalın kendi dosyaları oldu (`src/lib`, `src/styles`, `src/assets/fonts`). Yazı tipi
  üreteçleri kaynağı portaldan okuyor. Kanıt: portalın paketi taşımadan önceki ve sonraki derlemede aynı (özet adlı
  dosyalar değişmedi); `ui_fonts`, `drawing_fonts` `--check` yeşil; `sheet_metrics` yalnız kaynak notunda değişti.
- [x] **BK-81 — Masaüstünün web'den aldıkları** (`726f832d9`). Sihirbazın iki resmi modülünün yanında
  (`wizard/art/`). `tokens.css`'i okuyan iki sınama ve `favicon.svg` portalınkini okuyor. Etiket öntanımlıları
  `fixtures/labels/v1/defaults.json`'da, işlem araçlarının iki alanı `fixtures/processing/v1/tool-words.json`'da
  donduruldu: üretecin bir koşuda sorduğu 20 kapsam ve 4 etiket koşunun kendisinden kaydedildi; `--check` aynı
  90 + 25 durumu veriyor.
- [x] **BK-82 — Komut kataloğu `catalog/v1/commands.json`** (`095040391`). Ayar şemasının iki dosyası da orada
  (`settings-schema.json`, `settings-file.schema.json`). İçerikleri değişmedi. Yazan sınamalar ve yedi okuyan yeni
  yere bağlandı; SDK'nın 166 dosyası yalnız başlık satırında, manifest yalnız katalog alanında değişti.
  `docs/gis/coverage.*` bu işten önce de eskiydi (yeni komutlar); dokunulmadı.
- [x] **BK-83 — TypeScript yalnız Bulut için.** `ts-rs` çıktısı `apps/portal/src/api/generated`'a yazılır: portalın
  adını andığı 110 tür ve onların içe aldıkları, 343 dosya (önce 3 903). Kullanılmayan `ts` öznitelikleri aynı
  dilimde silindi: 15 668 satır, 420 dosya. `kentos-commands` ve `kentos-sheet`'in `ts` özelliği ve `ts-rs`
  bağımlılığı kalktı; `kentos-contracts`'ta `derive(TS)` 365 türde kaldı (343'ü dosya üretir, 22'si başka türün
  içine açılır), `ts(export)` yalnız 110 kökte. Kanıt: sıyırmadan önce ve sonra üretilen 343 dosya bayt bayt aynı;
  üretilen küme tam olarak kapanış; portalın tür denetimi, sınamaları ve paketi değişmedi. Portalın kullandığı altı
  tür web'in klasöründe eskimişti (`Entity`, `BeamEntity`…); şimdi günceller.
- [x] **BK-84 — WASM.** Dokuz bağlayıcı crate (`crates/wasm`), `wasm-bindgen`, `wasm` ve `wasm-dev` profilleri,
  `scripts/wasm` silindi. Ortak crate'lerdeki 13 `cfg(target_arch = "wasm32")` satırı kalktı (5 dosya): iş parçacıklı
  yol tek yol. Bağımlılık bekçisi ortak kitaplıkları yalnız ana makine hedefinde denetler (85 crate × 108 hedef →
  76 × 76); tarayıcı bağımlılıklarının yasağı duruyor. Kanıt: çalışma alanının tamamı bütün hedefleriyle derlendi;
  dokunulan crate'lerin sınamaları yeşil.
- [x] **BK-85 — `apps/web` silindi.** Kalan 5 472 izlenen dosya ve klasördeki derleme çıktıları (151 MB). Portalın
  ekran resmi betiklerinin kullandığı tarayıcı sürücüsü (`cdp.mjs`) portala taşındı. Yalnız web'e hizmet eden dört
  betik silindi: `scripts/ui/arrow_icons.py`, `hatch_icons.py` (web'in TypeScript dosyalarına yazıyordu),
  `scripts/wgsl/browser-check.mjs` (ortak WGSL'yi tarayıcıda derliyordu), `scripts/usage/compare.py` (iki platformun
  resimlerini yan yana koyuyordu). pnpm çalışma alanında tek paket portal; kilit dosyasından yalnız web'in bölümü
  çıktı. Yeni bekçi `scripts/dev/removed_paths.py` (`make guard`): iki klasör geri gelemez; kaynakta onları anan
  satırların sayısı (`apps/web` 484, `wasm` 319) yalnız azalır. Kanıt: çalışma alanı bütün hedefleriyle derlendi;
  masaüstü çekirdeğinin ve dört modül paketinin bütün sınamaları koşuldu (çekirdekte bu daldan önce kırmızı olan iki
  sınama dışında yeşil); portalın tür denetimi, sınamaları ve paketi değişmedi; SDK ve manifest denetimleri yeşil.
- [x] **BK-86 — Konaklardan `web` çıktı.** Komut kataloğu (`f6809f193`): `CommandHost`'tan `Web` çıktı; 24 komutun
  konağı yalnız `desktop`; katalogda tam 24 satır değişti, SDK ve manifest aynı çıktı. Ayar şeması: 68 ayar artık
  yalnız `desktop`; yalnız web'in kullandığı `graphics.backend` emekli anahtar (eski dosyada sessizce düşer, ADR 0155
  kuralı); masaüstünün içe almasındaki “web'e özgü değer” iletisi kalktı. Ortak durumlarda o anahtarı örnek alan
  sekiz durum `graphics.scene3dQuality`'ye taşındı, emekli anahtarın durumu eklendi. Sunucunun tarayıcı istemcisi
  (`x-kentos-client: web`, `Client::Web`) kalır: o portaldır.
- [~] **BK-87 — Kurallar ve belgeler.** Yapılan: CLAUDE.md bugünkü durumu söylüyor (kaldırma, kataloğun yeri,
  TypeScript kuralı, bekçi; `native/WASM` koşulu çıktı); kılavuz §4 tarih olarak işaretlendi (koddaki 14 “§4.N”
  atfının gösterdiği davranış kuralları masaüstünde geçerli); envanter belgesi dondurulmuş olduğunu söylüyor;
  `Makefile` ve `Cargo.toml` yorumları düzeltildi.
  - Kalan: kaynakta `apps/web` anan 484 ve `wasm` anan 319 satır (çoğu yorum: “the web's is `apps/web/src/…`”,
    “native and WASM”). Bekçi (`scripts/dev/removed_paths.py`) sayıyı tutar, yalnız azalır; dokunulan dosyada
    temizlenir.
  - Kalan: `TODOS.md`, `DESIGN.md` ve kılavuzun öbür bölümlerindeki web sözleri (BK-70 ile birlikte).
- Aşamanın sonunda bir kez koşulan geniş doğrulama: çalışma alanı bütün hedefleriyle derlendi; masaüstü çekirdeği
  (2 616 sınama), dört modül paketi, birleşik program, başsız konak ve MCP (313 sınama) sınamaları yeşil; portalın
  tür denetimi, 9 sınaması ve paketi taşımadan önceki hâliyle aynı. Koşulmayan: sunucunun veritabanı sınamaları,
  Python'ın tam takımı, eklenti sınamaları.
- Kılavuzun güncel karar özetine (§0) iki giriş yazıldı: ADR 9401'in düzeltmelerinden çıkan kurallar ve ADR 9402.
- Aşama 1'in sonunda kalan iki eski kırmızı sınama kapandı (ADR 9401 §11): `module_features` (27 Kasım'da `gis/`
  altına özelliksiz eklenen `lineage` çekirdeğe taşındı) ve `ifc_revisions` (BCF'nin kamerası platformun `tan`'ını
  kullanıyordu; IFC crate'indeki beş platform çağrısı `libm`'e bağlandı, crate'e clippy.toml eklendi; saklanan dosya
  değişmeden yeşile döndü).
- [~] **BK-88 — Sayısal temel** ([ADR 9403](../docs/adr/9403-numeric-base.md)). `jsmath` 696 dosyada kullanılıyor
  (`js_max` 882, `js_hypot` 702, `js_min` 494, `js_round` 159 çağrı; `libm`'den 15 işlev). Sıra: araştırma (doğru
  yuvarlanan temel işlevler, hatasız dönüşümler, kesin yüklemler, aralık aritmetiği; kaynakları ADR'de), karar,
  bağımsız yüksek hassasiyetli başvuru, işlev ailesi başına geçiş. Beklenen değerler başvurudan yeniden üretilir,
  koddan değil. JavaScript'in metin davranışını taşıyan yardımcılar (`js_trim`, `js_number`, `parse_number`) aynı işte
  adlandırılır ve kuralları yazılır.
  - [x] Araştırma ve ölçüm: `libm` sonuçlarının %3–18'i en yakın sayı değil (1 ulp); `pxfm` 13 işlevde 297 812 girdinin
    hepsinde doğru; `hypot`'u hiçbir kitaplık doğru yuvarlamıyor; bugünkü `js_hypot` uzunlukların %35'inde en yakın
    sayıyı vermiyor (ADR 9403 “Ölçüm”).
  - [x] `kentos-num` crate'i (`crates/shared/num`, bağımlılıksız, her crate'in altında) ve doğru yuvarlanan `hypot`;
    bağımsız başvuru `fixtures/numeric/v1/hypot.json` (5 729 durum, üreteci `scripts/fixtures/numeric_hypot.py`).
  - [x] `js_hypot` → `kentos_num::hypot`: bağımsız başvuruya karşı 22 682 ortak durumdan ikisi oynadı ve kaynağında
    düzeltildi (raster oturt'un öntanımlı pikseli, ADR 9403 §7); saklanan beş çıktı farkı okunarak yazıldı (araç
    `scripts/fixtures/ulp_diff.py`, ADR 9403 §6).
  - [x] Platformun `hypot`'una giden 586 çağrı ve `libm::hypot` sayısal temele (20 crate); bekçi
    `scripts/dev/platform_math.py` platformun matematik yöntemlerini ve `js_` yardımcılarını sayar, sayı yalnız azalır.
    Bu sırada bulunan: eşit uzunluktaki kaçış yollarının seçimi son basamağa bağlıydı (düzeltildi, ADR 9403 §7).
  - [ ] Adlar: `js_hypot` → `hypot` (702 çağrı; mekanik).
  - Açık, bu işten bağımsız (ADR 9403 §6): DWG codec'inin bir sınaması, Python kanalının tam koşusunda
    `cad.arch.performance` ve `cad.arch.ifc` durumları, `fixtures/raster/v1/georef.json` üretecinin ortam farkı.
  - [ ] Davranışı değişmeyen adlar (`minimum`, `maximum`, `sign`, sıralama).
  - [ ] Yuvarlama: 159 `js_round` yerinin sınıflandırması ve geçişi.
  - [ ] Metin: tek `trim`, tek katı sayı okuyucu, sayı yazımı (BK-50 ile).
  - [x] Aşkın işlevler (sahibin kararı 28 Kasım 2026: `pxfm`): `kentos-num` 21 işlevi doğru yuvarlanmış verir;
    `pow` ve `atan2`'nin özel değerleri ve tam orta noktaya düşen kuvvetler temelin kendi kodudur (kitaplık
    standarttan sapıyordu); başvuru `fixtures/numeric/v1/elementary.json` (8 176 durum, bit bit). `jsmath`'in dışa
    aktardıkları ve doğrudan `libm` çağrıları temele bağlandı; saklanan beş çıktı farkı okunarak yazıldı.
  - [ ] Platformun yöntemleriyle yapılan çağrılar (`x.sin()`, `x.atan2(y)`, `powi`…; bekçide ~900) sayısal temele.
  - [ ] Platform kitaplığı yasağının bütün hesap crate'lerine yayılması; `jsmath`'in silinmesi.

## Doğrulama

- Her dilim: `make guard` yeşil; dokunulan crate'in hedefli sınaması (`scripts/dev/svc.sh heavy cargo test -p <crate>
  <süzgeç>`; masaüstünde `RUST_MIN_STACK=67108864`, tek süzgeç); masaüstü dilimlerinde ayrıca `module_tests`,
  `ribbon_tests`, `history::text`, katalog ve ayar süzgeçleri.
- Tek kaynak dilimleri: üretilen dosyalar (`catalog/v1/*`, `commandCatalog.json`, SDK) `git diff` ile farksız.
- Kabuk dilimleri: bir `screens` sınaması koşulur, resme bakılır.
- Sayılar her aşamanın sonunda yeniden ölçülür: biçimsiz dosya 3 010 → 0; sınırı aşan dosya 88 → 0; 800 satırı aşan
  işlev 25 → 0; MCP sınama ikilisi 166 → 1; bir komutun elle yazıldığı yer 11–15 → 3; bir pencerenin dokunduğu merkezî
  dosya 7 → 0; `App` 236 alan, `Message` 187 kol, `Dialog` 88 kol.
