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
- [ ] **BK-04 — Sınama ikilileri** (E1). MCP 166 → 1 (`tests/all/main.rs`, ortak `call`), sunucu 41 → 1. Bağlama
  süresi ve `target/` boyu önce ve sonra ölçülür.
- [ ] **BK-05 — Ölçüm tabanı.** Clippy'nin sonucu; sıfırdan ve tek satır değişiklikte derleme süresi; `docs/perf/`.

### Aşama 1 — Doğruluk düzeltmeleri

- [ ] **BK-10** (A1) İmar takımı `call::recorded` üzerinden; ret ayarları geri alır.
- [ ] **BK-11** (A2) Çok adımlı Kaydet için tek yardımcı (`document::in_step`, `shell/document.rs:691` üstüne); üç
  pencere ve `leader_tools`.
- [ ] **BK-12** (A3) Arka plan yanıtı koşu kimliği taşır; beş aktarım ve IFC karşılaştırması.
- [ ] **BK-13** (A4) BCF okuması kendi iş parçacığında; form taslağı gecikmeli ve hatası görünür yazılır.
- [ ] **BK-14** (A7) İki ayrıştırıcıya derinlik sayacı.
- [ ] **BK-15** (A8, A9) Dört sessiz kayıp hataya ya da rapora döner; `execute_batch` yerine `prepare`.
- [ ] **BK-16** (A6) Hata kodu tablosu disiplin başına dosyalara bölünür, `INVALID_MASS` eklenir.
- [ ] **BK-17** (A10) Yerel nokta-alan ve alan kopyaları çekirdektekine bağlanır; çizime yazan yerler
  `display::fixed`'e. Beklenen değerler bağımsız başvuruyla denetlenir.
- [ ] **BK-18** (A11) Üç arayüz metni.
- [ ] **BK-19** (A5) Doğrudan yazıcılar kilitli katman denetimine ve tek geri alma adımına bağlanır; komuta
  çevrilmeleri Aşama 2'den sonra. Stil yöneticisi `cad.library.edit`'e geçer.

### Aşama 2 — Tek kaynak: kopyayı türetmeye çevir

- [ ] **BK-20** (B1) `history/text.rs` `TITLES` silinir; başlık katalogdan okunur.
- [ ] **BK-21** (B2, D3) Komut tek yerde bildirilir; `example_inputs`, çıktı eşlemesi, dağıtım, `DESKTOP_COMMANDS`,
  sınama koşucusu ve `use` listeleri ondan üretilir. Katalog tanımı komutun tipleriyle aynı dosyada, kendi işlevinde.
- [ ] **BK-22** (B3) İşleyici iskeleti: `Result → CommandResult` dönüşümü ve `checks::fail` ortak; `create_command!`
  genellenir.
- [ ] **BK-23** (B4) Masaüstü kataloğu tek dosyada; birleşmeden önceki ve sonraki kataloğun eşitliği sınamayla
  kanıtlanır. `PORTED` BK-32'nin işleyici tablosundan türer.
- [ ] **BK-24** (B5) Araç listesi tek tablo.
- [ ] **BK-25** (G1) Üretilen çıktılar donmuş web'in dışına taşınır; portal ve SDK üreteci yeni yeri okur.

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
  arkasına.
- [ ] **BK-53** (F4) Grafik paleti ve yazı boyu tema jetonu; komut sonucu tek yardımcı; kullanılmayan yedi kit
  bileşeni için sahibin kararı.
- [ ] **BK-54** (F5) Sınır hatalarına tür, önce dosya biçimleri; eksik üç kod tabloya.

### Aşama 6 — `.kcad` şema zinciri

- [ ] **BK-60** (B6) Araştırma, ADR, belirtim §3.6 ve §11 değişikliği. Yeni belge alanı adlı zorunlu uzantıyla, modül
  verisi modül sürümüyle gelir. Mevcut 2–257 dokunulmadan kalır; sabit sayısı bekçiye girer ve artamaz.

### Aşama 7 — Harness, belgeler ve kalan inceleme

- [ ] **BK-70** (G2) Kılavuz §0 alan başına dosyalara bölünür; `TODOS.md`'nin biten bölümleri arşive.
- [ ] **BK-71** (G3) Proje skill'leri (dilim kapatma, yeni komut, resim çekme) ve ajan tanımları; kökteki inceleme
  dosyaları `docs/devir/`'e.
- [ ] **BK-72** Sunucu, bulut istemcisi, eklenti konağı ve imza için aynı inceleme.

## Doğrulama

- Her dilim: `make guard` yeşil; dokunulan crate'in hedefli sınaması (`scripts/dev/svc.sh heavy cargo test -p <crate>
  <süzgeç>`; masaüstünde `RUST_MIN_STACK=67108864`, tek süzgeç); masaüstü dilimlerinde ayrıca `module_tests`,
  `ribbon_tests`, `history::text`, katalog ve ayar süzgeçleri.
- Tek kaynak dilimleri: üretilen dosyalar (`catalog/v1/*`, `commandCatalog.json`, SDK) `git diff` ile farksız.
- Kabuk dilimleri: bir `screens` sınaması koşulur, resme bakılır.
- Sayılar her aşamanın sonunda yeniden ölçülür: biçimsiz dosya 3 010 → 0; sınırı aşan dosya 88 → 0; 800 satırı aşan
  işlev 25 → 0; MCP sınama ikilisi 166 → 1; bir komutun elle yazıldığı yer 11–15 → 3; bir pencerenin dokunduğu merkezî
  dosya 7 → 0; `App` 236 alan, `Message` 187 kol, `Dialog` 88 kol.
