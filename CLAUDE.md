# KentOS CAD — Geliştirme kılavuzu

Bu dosya, KentOS CAD üzerinde çalışan Claude (ve ekipteki herkes) için bağlayıcı
kuralları ve mimariyi anlatır. Görsel dil, renkler ve bileşen kuralları için
[DESIGN.md](DESIGN.md) dosyasına bakın. Kod ile bu belge çelişirse önce kodu
doğrulayın, sonra belgeyi güncelleyin; belge güncel tutulmak zorundadır.

**Yeni hedef (23 Eylül 2026):** §13 ve sonrası Rust/WASM ortak çekirdek, Rust
backend, PostgreSQL/PostGIS veri modeli, tenant, worker ve bulut dağıtımının
bağlayıcı mimarisidir. §1–12 mevcut tarayıcı uygulamasını ve eski yol haritasını
anlatır. Gelecek veri katmanında çelişki varsa §13 ve sonrası geçerlidir.
Rust backend, Rust/WASM çekirdek, sunucu worker ve kalıcı PostgreSQL katmanı
**henüz bu depoda uygulanmış değildir**. Bu dosyadaki hedefleri çalışan özellik diye
raporlamayın.

## 0. Güncel karar özeti ve kapsam kontrolü

**Belge revizyonu: 2026-09-23 / konsolide-v2.** Bu dosya mevcut depo
kılavuzunun güncellenmiş halidir; önceki bağımsız mimari taslakları
uygulama talimatı olarak kullanmayın. Mevcut kod envanteri §1–12 ve
§13.1'de, uygulanacak hedef sözleşmeler §13–24'tedir. Eski envanterde
"yapıldı" denmesi backend'de de uygulandığı anlamına gelmez.

| Konuşulan karar               | Bağlayıcı karşılığı                                                                              | Ayrıntı      |
| ----------------------------- | ------------------------------------------------------------------------------------------------ | ------------ |
| Veritabanı                    | PostgreSQL + PostGIS; özel embedded/KV veri motoru ve özel WAL/MVCC tasarımı kapsamdan çıkarıldı | §13, §15     |
| Backend stack                 | Rust + Axum + Tokio + SQLx; HTTP middleware için Tower                                           | §14          |
| Ağır işlemler                 | Ayrı sunucu worker modu, PostgreSQL kalıcı job kuyruğu, lease/fencing ve idempotent sonuç        | §18          |
| Çoklu tenant                  | Tenant üyeliği, tenant başına kullanıcı/koltuk tahsisi, rol ve kaynak yetkisi, RLS ve kota       | §16          |
| Bulut kalıcılığı              | PostgreSQL proje/veri; S3/MinIO dosyalar ve büyük çıktılar; yedek/PITR/restore                   | §16          |
| Ortak hesaplama               | Tek Rust algoritması, backend native ve frontend WASM; bağımsız doğruluk testleri                | §14, §23     |
| Katman yayını                 | Wizard, MVT/TileJSON, stil/sprite/glyph, izin ve revision/cache tutarlılığı                      | §17, §24     |
| İlk yayın motoru              | Martin, KentOS gateway arkasında kullanılır                                                      | §17          |
| Gelecekte Martin bağımsızlığı | Şimdi uygulanmaz; ölçüm, eşdeğerlik, kademeli geçiş ve geri dönüş kapısına bağlıdır              | §17.1, Faz E |
| Harita istemcisi              | Mevcut kendi WebGL2/WebGPU renderer'ları; MapLibre GL JS zorunlu runtime değildir                | §17          |
| Stil doğruluğu                | Kaynak `.kstil`/MPYY korunur; dış MapLibre çıktısında destek matrisi ve kayıp raporu gerekir     | §17          |
| Frontend                      | Mevcut TypeScript DOM/Signal korunur; modülerlik için React'e geçiş yok                          | §20          |
| Lazy load                     | Stil/workflow/layout/ileri analiz/3D ayrı JS, CSS, WASM ve asset yükleme sınırları               | §20          |
| Sürekli bağlantı              | WebSocket izlenir, heartbeat/reconnect/replay/resync; job socket'tan bağımsız                    | §21          |
| Proje açılışı                 | Yerel ve bulut projeleri asenkron/aşamalı; iptal, generation ve sınırlı bellek                   | §21.2        |
| Otomatik kayıt                | Bulutta açık; dayanıklı yerel komut kuyruğu, commit ACK, idempotency ve conflict                 | §21.3        |
| Gelecek 3D urban design       | İmar/parsel/senaryo tabanlı; ortak Rust, ağır worker, LOD ve streaming                           | §22, Faz F   |
| Kadastro ve mülkiyet          | Kaynak hassasiyeti, açık yuvarlama politikası, kesin hisse/decimal, robust geometri              | §23          |
| Kalite hedefi                 | Çalışan kabul senaryoları ve ölçüm zorunlu; belgeyi tamamlamak 10/10 ürün kanıtı değildir        | §19.0        |
| Önceki CBS kapsamı            | Katalog/provider, tipli şema/domain/ilişki/form, agentic komutlar ve operasyon korunur           | §24          |

Önceki genel taslaklardaki React zorunluluğu, SSE'yi birincil bağlantı
yapma, ilk sürümde Martin yerine özel tile motoru yazma ve WebGPU'yu
henüz hiç yok sayma talimatları bu depo için geçerli değildir. Yeni
özelliklerin hepsini tek sprintte kurmayın; §19'un çalışan dikey dilim
sırasını izleyin. Bu revizyon uygulama değişikliği veya GitHub'a push
yapıldığı iddiası değildir.

---

## 1. Ürün

**KentOS CAD**, tarayıcıda çalışan bir harita, kadastro ve kent bilgi sistemi
çizim ortamıdır. Doğrudan rakibi **Netcad**'dir. Hedef kullanıcılar harita
mühendisleri, LİHKAB ve serbest harita büroları, kadastro ve belediye imar
teknisyenleridir: günde saatlerce, çoğunlukla klavye kısayoluyla çalışan
profesyoneller.

- **Yalnızca masaüstü.** Mobil ve dar ekran için çalışma yapılmaz. Kabuk en az 1100×600 px'tir.
- **Arayüz dili Türkçedir.** Kod, tanımlayıcılar ve yorumlar İngilizcedir.
- **Marka:** menü çubuğunda "KentOS" yazar, sayfa başlığı "KentOS CAD"dir, logo "K" harfidir.

### 1.1 Hibrit ilke: CAD çekirdeği, GIS veri modeli

KentOS ne saf bir CAD (AutoCAD) ne de saf bir GIS (QGIS) olacak. Kullanıcının
günlük işi hassas çizimdir, verisi ise coğrafi ve özniteliklidir.

| Konu     | CAD tarafı (nasıl çizilir)                                     | GIS tarafı (veri ne anlama gelir)                        |
| -------- | -------------------------------------------------------------- | -------------------------------------------------------- |
| Geometri | Hassas nokta girişi, kenetleme, orto, komut satırı, tutamaçlar | Her nesne bir koordinat sisteminde (SRID) yaşar          |
| Nesne    | Çizgi, çoklu çizgi, yay, daire, yazı, ölçü                     | Öznitelikli detay (ada, parsel, nitelik, tapu alanı)     |
| Katman   | Renk, çizgi tipi, kalınlık, kilit                              | Tipli öznitelik şeması, sorgu, tematik gösterim (planlı) |
| Doğruluk | Geri alınabilir düzenleme, toleranslar                         | Topoloji: komşu parseller ortak sınır paylaşır (planlı)  |
| Çıktı    | Pafta, yazdırma, DXF                                           | GeoJSON, Shapefile, WFS, PostGIS (planlı)                |

Karar verirken sorulacak soru şu: **"Bir harita mühendisi bunu Netcad'de nasıl
yapıyor ve biz bunu verinin anlamını bozmadan nasıl daha iyi yaparız?"**

---

## 2. Çalıştırma

**Depo düzeni** (monorepo; ayrıntı §12, hedef §14): kök hem Cargo hem pnpm çalışma alanıdır.

- `apps/web/`: tarayıcı uygulaması (TypeScript, Vite; pnpm paketi `@kentos/web`). §4–§11'deki `model/`, `tools/`, `ui/` gibi klasör adları `apps/web/src/`'ye göredir.
- `apps/api/`: sunucu (`kentosd`, Axum). İleride `apps/desktop/` (wgpu masaüstü) buraya gelir.
- `crates/shared/`: platformdan bağımsız Rust kütüphaneleri (`geometry-core`, `style-core`, `svg-core`, `contracts`, `formats`); web, sunucu ve masaüstü aynı kodu kullanır. DOM, tarayıcı, veritabanı ve ağ bilmezler.
- `crates/wasm/`: bu kütüphanelerin tarayıcı bağlayıcıları (`geometry-wasm`, `formats-wasm`, `svg-wasm`); çıktıları `apps/web/src/wasm/pkg`, `apps/web/src/io/pkg` ve `apps/web/src/style/svg/pkg`'dir.
- `crates/server/`: yalnız sunucunun kütüphaneleri (`postgres`, `application`).
- `fixtures/`: Rust ve TypeScript testlerinin paylaştığı sürümlü dosyalar; `scripts/`: kökten çalışan ortak araçlar (WASM derleme denetimi, Python referans üreticileri); `docs/`: belgeler ve ölçümler.

Komutlar kökten çalıştırılır; web komutları `apps/web`'e iletilir (`pnpm -C apps/web …` ile doğrudan da çalışır).

```bash
pnpm install
pnpm dev                  # Vite geliştirme sunucusu
pnpm build                # tsc (tip denetimi) + vite build
pnpm test                 # Vitest birim testleri (geometri, işlemler, belge, biçimlendirici)
pnpm e2e                  # Başsız Chrome'da uçtan uca duman testi (kendi Vite sunucusunu açar)
pnpm typecheck            # yalnızca tip denetimi (apps/web: tsc --noEmit)
pnpm rust:test            # Rust çalışma alanı: cargo test + clippy (-D warnings)
pnpm wasm                 # WASM paketi kaynak değiştiyse derlenir (dev/test/build/e2e bunu kendileri çalıştırır)
pnpm rust:wasm            # geometri çekirdeğinin WASM paketi → apps/web/src/wasm/pkg (depoya girmez)
pnpm rust:wasm:formats    # dosya biçimlerinin WASM paketi → apps/web/src/io/pkg (depoya girmez; yalnız içe/dışa aktarmada yüklenir)
pnpm rust:wasm:svg        # SVG düzenleyicisinin WASM paketi → apps/web/src/style/svg/pkg (depoya girmez; yalnız düzenleyici açılınca yüklenir)
pnpm test:rust            # rust:test + WASM paketi + apps/web/src/wasm ve apps/web/src/io testleri (çağrı ve depo fixture'ları, bağımsız referans, biçimler)
pnpm db:setup             # kentosd db-setup + migrate + dev-seed (yerel PostGIS'te kentos_cad, iki rol, örnek kurum ve üç hesap: ayse, mehmet, zeynep)
pnpm api                  # kentosd serve: 127.0.0.1:8787 (veritabanı yoksa yalnızca /v1/health)
pnpm kentosd -- <komut>   # yönetim: tenant add|list, user add|password, member add|list, project deleted|restore, migrate, dev-seed
pnpm e2e:cloud            # gerçek sunucu ve veritabanıyla bulut akışı (giriş, yükleme, otomatik kayıt, çakışma, kopma, yeniden adlandırma, silme)
pnpm perf:interaction     # etkileşim ölçümü (parsel-50k, hat-1m): seçme, kenet, buda önizlemesi, kaydırma, katman kurma → docs/perf/ (§9.4)
```

- **API bağlantısı:** `vite` ve `vite preview`, `/v1/` isteklerini ve proje WebSocket'ini (`/v1/ws`) `apps/web/vite.config.mjs` içindeki küçük bir eklentiyle `127.0.0.1:KENTOS_API_PORT` (varsayılan 8787) adresine iletir. API çalışmıyorsa sessizce 503 döner. Durum çubuğu “Sunucu: bağlı / yok / uyumsuz” gösterir; yerel çizim sunucuya hiç bağlı değildir.
- **Sunucu ayarları** (`kentosd`, önce ortam değişkeni, sonra `.env.local`; `.env.local` depoya girmez, `0600`): `KENTOS_DATABASE_URL` (sunucu rolü), `KENTOS_DATABASE_OWNER_URL` (migration ve yönetim), `KENTOS_PUBLIC_URL` (tarayıcının adresi; WebSocket kaynak denetimi ve OpenID dönüşü, varsayılan `http://localhost:5173`), `KENTOS_COOKIE_SECURE`, `KENTOS_LOCAL_LOGIN`, `KENTOS_EVENT_RETENTION_DAYS` (proje olaylarının saklandığı gün sayısı, 1–3650, varsayılan 7; daha eski imleçli istemci projeyi yeniden açar), OpenID için `KENTOS_OIDC_ISSUER`, `KENTOS_OIDC_CLIENT_ID`, isteğe bağlı `KENTOS_OIDC_CLIENT_SECRET`, `KENTOS_OIDC_AUDIENCE`, `KENTOS_OIDC_LABEL`. Veritabanı testleri `KENTOS_TEST_ADMIN_URL` ile geçici `kentos_cad_test_*` veritabanları açar; sunucu yoksa atlanır, `KENTOS_TEST_DB=required` bunu hata sayar (ADR 0006, 0007).

- **Rust araç zinciri** `rust-toolchain.toml` ile sabittir (wasm32 hedefi dahil); derleme `.cargo/config.toml` ile 4 işle sınırlıdır. WASM paketi için `wasm-bindgen` komutu crate sürümüyle aynı olmalıdır: `cargo install wasm-bindgen-cli --version 0.2.128 --locked`. **Uygulama geometriyi Rust çekirdeğinden (WASM) alır** (ADR 0008): `pnpm dev`, `test`, `build` ve `e2e` önce `scripts/wasm/ensure.mjs`'i çalıştırır; çekirdeğin kaynakları (ifade dili ve stil derleyicisi, `crates/shared/style-core`, dahil) değiştiyse paket `nice` ile yeniden derlenir (`apps/web/src/wasm/pkg/.stamp`). Aynı betik dosya biçimleri paketini (`crates/shared/formats`, `crates/wasm/formats-wasm`, `crates/shared/contracts` → `apps/web/src/io/pkg`) ayrı özet ve damgayla derler: biçimler değişince çekirdek derlenmez; biçimler çekirdeğin halka örneklemesini kullandığı için çekirdek değişince biçim paketi de derlenir; hiçbiri değişmediyse hiçbir şey derlenmez. SVG düzenleyicisinin paketi de öyledir (`crates/shared/svg-core`, `crates/wasm/svg-wasm` → `apps/web/src/style/svg/pkg`; geometri çekirdeği ya da style-core değişince de derlenir); düzenleyici açılırken yüklenir (`style/svg/core.ts` `initSvgCore`). Bu yüzden Rust araç zinciri bunların hepsi için gereklidir. Sayfa çekirdeği uygulamadan önce başlatır (`apps/web/src/wasm/core.ts`), worker derlenmiş modülü ilk işiyle alır. Cargo derlerken e2e ya da başka bir ağır iş çalıştırılmaz (ADR 0001).

- **Çizim motoru:** varsayılan WebGL2'dir; WebGPU isteğe bağlıdır.
  - Etkin motor durum çubuğunun sağ alt köşesinde yazar. Tıklayınca motor seçilir: seçim hemen uygulanır (`view.switchBackend`, sayfa yenilenmez) ve `prefs.rendererPreference` ile hatırlanır. Aynı seçim **Görünüm → Çizim motoru** menüsünde ve Uygulama ayarları → Çizim motoru bölümünde de vardır.
  - `?renderer=webgpu` ya da `?renderer=webgl2` URL parametresi açılışta kayıtlı tercihi geçersiz kılar.
  - WebGPU başlatılamazsa WebGL2'ye düşülür ve uyarı yazılır.
- Her değişiklikten sonra `pnpm typecheck` temiz olmalı ve `pnpm test` geçmeli (bkz. §9.4).
- Geliştirme modunda uygulama bağlamı `window.kentos` olarak açıktır (üretim derlemesinde yoktur). Tarayıcıda doğrulama yaparken durumu buradan okuyun, ör. `kentos.doc.size`, `kentos.tools.activeId.value`.
- Arayüzü etkileyen her değişiklik **gerçek tarayıcıda** denenmelidir: tıklama, klavye, açık ve koyu tema, "Büyük" yazı boyutu.
- Tercihler `localStorage`'da `kentos.ui.v1` (yerleşim), `kentos.prefs.v1` (uygulama ayarları) `kentos.processing.v1` (işlem araçlarının son değerleri ve kullanıcı modelleri) ve `kentos.styles.v1` (kullanıcının stil kitaplığı) anahtarlarında durur. Bulut projelerinin gönderilmemiş değişiklikleri IndexedDB'de (`kentos.cloud` / `drafts`, hesap ve proje başına) durur. Temiz başlangıç için bu anahtarları silin.

---

## 3. Değişmez teknik kısıtlar

- **TypeScript strict** ve `erasableSyntaxOnly`:
  - `enum`, `namespace` ve yapıcı parametre özellikleri (`constructor(private x)`) yasaktır.
  - Birleşim tipleri (`'a' | 'b'`) ve `as const` nesneleri kullanın.
  - Alanları açıkça tanımlayıp yapıcıda atayın.
- **`verbatimModuleSyntax`:** yalnızca tip olan içe aktarmalar `import type` ile yazılır.
- **UI çatısı yok.** DOM, `ui/dom.ts` içindeki `h()` ile kurulur; tepkisellik `core/signal.ts` ile sağlanır. React, Vue veya Lit eklenmez.
- **Çalışma zamanı bağımlılığı eklemek bir karardır.** Kabul ölçütleri:
  - küçük olmalı, ağaç sallamaya (tree-shaking) uygun olmalı
  - MIT/BSD lisanslı olmalı
  - worker içinde çalışabilmeli, DOM gerektirmemeli
  - bakım altında olmalı

  Aday örnekleri: `earcut` (üçgenleme), `flatbush`/`rbush` (R-tree), `proj4` (dönüşüm). Eklemeden önce kullanıcıya sorun.
- **Araç zinciri:** Vite 8, TypeScript 6, pnpm (çalışma alanı: kök ve `apps/web`). Hedef ES2023.
- **Tarayıcı kısayolları:** Tarayıcının yakaladığı kısayollar bağlanmaz: `Ctrl+N`, `Ctrl+T`, `Ctrl+W`, `Ctrl+Shift+T`, `Alt+F`, `Alt+D`, `Alt+E`.

---

## 4. Mimari

### 4.1 Katmanlar ve bağımlılık yönü

```
core ─► geo ─► model ─► style ─► processing ─► render ─► viewport ─► tools ─► ui ─► app (kompozisyon kökü)
```

Oklar "şunu kullanabilir" yönündedir: bir katman yalnızca **solundakileri**
içe aktarabilir. `app/context.ts` içindeki `AppContext` **tipi** her yerden
`import type` ile kullanılabilir; somut servisler yalnızca `app/createApp.ts`
içinde kurulur.

| Klasör        | Sorumluluk                                                                                                                                                                                                                 | İçe aktarabileceği                          | Asla                                   |
| ------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------- | -------------------------------------- |
| `core/`       | Signal, Emitter, Disposable, CommandRegistry, Keymap                                                                                                                                                                       | Yalnızca DOM tipleri                        | model, ui                              |
| `geo/`        | EPSG/CRS kaydı; ileride dönüşümler, geodezik hesaplar                                                                                                                                                                      | core                                        | DOM, model                             |
| `model/`      | Belge, varlıklar, katman ağacı, geometri, seçim, proje ayarları, geri alma                                                                                                                                                 | core, geo                                   | DOM, render, ui                        |
| `style/`      | Stil motoru: semboller ve sembol katmanları, katman stilleri (işleyiciler), kitaplık (sistem/kullanıcı/proje, kategori ağacı), .kstil dosyaları, sembol × geometri → çizim ilkelleri (hesap `crates/shared/style-core`'da; bkz. [docs/STYLE.md](docs/STYLE.md)) | core, geo, model                            | DOM, render, viewport, tools, ui, app  |
| `processing/` | İşlem araçları: bildirimsel tanım, parametreler, kayıt, çalıştırıcı, modeller (bkz. [docs/PROCESSING.md](docs/PROCESSING.md))                                                                                              | core, geo, model                            | DOM, render, viewport, tools, ui, app  |
| `render/`     | `RenderBackend` sözleşmesi, sahne verisi, WebGL2 ve WebGPU arka uçları                                                                                                                                                     | core, model (tip + stil)                    | ui, tools, viewport                    |
| `viewport/`   | Kamera, seçme ve kenetleme dizini, 2B üst katman, çizim döngüsü                                                                                                                                                            | core, model, render, tools (tip), app (tip) | ui                                     |
| `tools/`      | Etkileşimli araçlar ve araç kataloğu                                                                                                                                                                                       | core, model, viewport (tip), app (tip)      | ui                                     |
| `ui/`         | Bileşenler, paneller, pencereler, widget'lar                                                                                                                                                                               | hepsi (servisler `AppContext` üzerinden)    | model'i doğrudan değiştirmek (bkz. §8) |
| `app/`        | Kompozisyon kökü, komutlar, menüler, kısayollar, durum depoları, biçimlendirici                                                                                                                                            | hepsi                                       | —                                      |

**`io/`** (dosya alışverişi: biçim worker'ı `formatsWorker.ts`, sayfa tarafı `client.ts`, okunan nesneleri belgeye koyan `apply.ts`, koordinat listesi seçenekleri `coords.ts`; `pkg/` üretilir) `processing` ile aynı düzeydedir: core, geo, model ve contracts'ı içe aktarır, DOM'a dokunmaz; pencereleri `ui/io/`'dadır. Biçimlerin kendisi Rust'tadır (`crates/shared/formats`, §9.7); uygulamanın da hesapladığı geometriyi (yaylı halkanın noktaları, alan, içerme) ortak çekirdekten alır. `io/` ve `ui/io/` başlangıç paketine girmez, komut çalışınca yüklenir (§20).

**`wasm/`** (Rust geometri çekirdeğinin tarayıcı cephesi: `core.ts` başlatma ve `op()` çağrıları; `pkg/` üretilir) modelin altındadır: `model` ve sağındaki her katman onu içe aktarabilir, o yalnızca `pkg/`'yi içe aktarır. Çağrı kümeleri, sahne üreteci ve fixture okuyucuları (`wasm/calls/`) teste özeldir (ADR 0008).

**`contracts/`** zincirin dışındadır: Rust'tan (`crates/shared/contracts`, ts-rs) üretilen sürümlü sözleşme tipleri (`generated/`, elle düzenlenmez) ve sözleşme sürümü (`version.ts`). Hiçbir şey içe aktarmaz; her katman buradan tip alabilir. Uygulamanın kendi tipleri sözleşmeye `contracts.test.ts`'te derleme anında denetlenir (ADR 0002).

Bağımlılık yönünü bozan bir içe aktarma gerekiyorsa tasarım yanlıştır. Bu
durumda bir arayüz ya da olay ekleyin, döngüsel bağımlılık kurmayın.

### 4.2 Kompozisyon kökü

`app/createApp.ts` tek kurulum noktasıdır. Sıra şöyledir:

1. `CommandRegistry` ve `Keymap`
2. `UiState` ve `Preferences` (localStorage)
3. `CadDocument` (şimdilik örnek proje, SRID = `prefs.defaultSrid`)
4. Tema ve yazı ölçeği CSS'e uygulanır. Bu, paleti okuyan her şeyden önce olmalıdır.
5. `AppContext` nesnesi kurulur; `ToolManager` ve `ViewportController` ona bağlanır.
6. Komutlar ve kısayollar kaydedilir; `AppShell` DOM'a takılır.
7. `view.mount()` çizim arka ucunu başlatır.

Başka hiçbir modül servis oluşturmaz.

### 4.3 AppContext

Bütün özellik modüllerinin tek bağımlılığıdır (`app/context.ts`):

| Servis       | Tür                  | Görev                                                                                                                                     |
| ------------ | -------------------- | ----------------------------------------------------------------------------------------------------------------------------------------- |
| `commands`   | `CommandRegistry`    | Kullanıcının tetikleyebildiği her şey                                                                                                     |
| `keymap`     | `Keymap`             | Kısayol → komut eşlemesi                                                                                                                  |
| `doc`        | `CadDocument`        | Açık proje: varlıklar, katmanlar, proje ayarları, geçmiş                                                                                  |
| `selection`  | `Selection`          | Seçili ve üzerine gelinen varlık kimlikleri                                                                                               |
| `settings`   | `DraftingSettings`   | Oturumluk çizim yardımcıları (kenet, ızgara, orto, geçerli renk ve tip)                                                                   |
| `prefs`      | `Preferences`        | Uygulama ayarları (kalıcı, kullanıcıya özel)                                                                                              |
| `ui`         | `UiState`            | Çalışma alanı yerleşimi (kalıcı)                                                                                                          |
| `format`     | `Formatter`          | Sayıdan metne tek geçit (proje birimlerini kullanır)                                                                                      |
| `log`        | `MessageLog`         | Komut geçmişi, uyarılar, durum çubuğu mesajı                                                                                              |
| `tools`      | `ToolManager`        | Etkin araç, istem metni                                                                                                                   |
| `view`       | `ViewportController` | Kamera, seçme, çizim isteği                                                                                                               |
| `clipboard`  | `Clipboard`          | Kopyalanan nesneler (oturumluk; `app/clipboard.ts`)                                                                                       |
| `processing` | `ProcessingService`  | İşlem araçları kaydı, çalıştırıcı ve geçmişi, araçların son değerleri (`app/processing.ts`)                                               |
| `styles`     | `StyleService`       | Stil kitaplığı: sistem (salt okunur), kullanıcı (`kentos.styles.v1`) ve proje (`doc.styles`) sembolleri, kategori ağacı (`app/styles.ts`) |
| `files`      | `DocumentFiles`      | Yerel çizim dosyası (.kcad): kaydet, farklı kaydet, aç, yeni proje; kaydedilen dosyanın tutamacı; içe aktarılacak dosyanın seçimi (`pickForImport`); dosya pencereleri (`picker`, duman testinde bellek içi) (`app/fileIO.ts`)                         |
| `server`     | `ServerStatus`       | API'nin yanıt verip vermediği (`/v1/health`, üretilen `Health` sözleşmesiyle doğrulanır; sözleşme sürümü farklıysa “uyumsuz”) (`app/server.ts`) |
| `cloud`      | `CloudSession`       | Oturum, açık bulut projesi (yeniden adlandırma, silme, ayrılma), otomatik kayıt (`ProjectSync`) ve canlı olaylar (`ProjectSocket`) (`app/cloud/`) |

İleride birden fazla belge açılacaksa, belgeye bağlı servisler (`format`,
`view` içindeki önbellekler) belge değişince yeniden kurulmalıdır. Bunun için
`doc` doğrudan önbelleğe alınmaz; her seferinde `ctx.doc` üzerinden okunur.

### 4.4 Durum kapsamları (en önemli ayrım)

Yeni bir ayar ya da durum eklemeden önce **hangi kapsama ait olduğuna** karar verin:

| Kapsam                      | Nerede                                                      | Saklama                          | Kim görür                           | Örnekler                                                                                                                                                                                                                                                                              |
| --------------------------- | ----------------------------------------------------------- | -------------------------------- | ----------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **Proje ayarları**          | `model/projectSettings.ts` → `doc.settings`                 | Proje dosyası (.kcad)            | Projeyi açan herkes                 | SRID, uzunluk ve alan hassasiyeti, alan birimi, açı birimi, çizim ölçeği, proje adı, çalışma modu (§4.12), çizim yazı tipi (çizimdeki yazı, ölçü ve etiketler)                                                                                                                                                                                                   |
| **Belge verisi**            | `CadDocument`, `LayerStore`                                 | Proje dosyası                    | Projeyi açan herkes                 | Varlıklar, katman ağacı ve stilleri (işleyiciler dahil), nesne sembolleri, öznitelikler, projenin stil kitaplığı (`doc.styles`)                                                                                                                                                       |
| **Uygulama ayarları**       | `app/state.ts` → `ctx.prefs`                                | `localStorage` `kentos.prefs.v1` | Yalnızca bu kullanıcı, tüm projeler | Tema, vurgu rengi (`prefs.accent`, varsayılan lacivert), arayüz yazı tipi (`prefs.uiFont`, varsayılan Plus Jakarta Sans), arayüz düzeni (klasik ya da şerit, `prefs.shell`), yazı boyutu (beş kademe), artı imleç, fare yardımcıları (imleç yanında giriş, bilgi kartı), kenet türleri ve yarıçapları, çizim motoru, çizim kalitesi (`prefs.renderQuality`: yüksek, dengeli, hızlı), sembol boyutu (çizim ölçeğinde / ekranda sabit), **yeni proje varsayılan SRID'si (5256), çalışma modu ve çizim yazı tipi**; işlem araçlarının son değerleri (`kentos.processing.v1`); kullanıcının stil kitaplığı (`kentos.styles.v1`) |
| **Çalışma alanı yerleşimi** | `app/state.ts` → `ctx.ui`                                   | `localStorage` `kentos.ui.v1`    | Yalnızca bu kullanıcı               | Panel genişlikleri, araç kutusu konumu, sütun sayısı ve katlanan grupları, açık sekme, sağ dok sekmesi (Katmanlar/İşlemler), İşlemler görünümü ve katlanan kategoriler; şeridin açık sekmesi, daraltılmışlığı, hızlı erişime eklenenler ve şeritle birlikte araç kutusunun açıklığı                                                                                                                |
| **Oturum durumu**           | `DraftingSettings`, `Selection`, `ToolManager`, `Clipboard` | Saklanmaz                        | Bu oturum                           | Kenet/Izgara/Orto düğmeleri, seçim, etkin araç, pano, işlem geçmişi                                                                                                                                                                                                                   |

Kurallar:

- **Bir iş arkadaşı projeyi açtığında aynısını görmesi gerekiyorsa proje ayarıdır.** Kişisel tercih ise uygulama ayarıdır.
- Proje ayarı değişince proje kaydedilmemiş sayılır (`doc.dirty`).
- Proje ayarları **Dosya → Proje ayarları…** penceresinde (`ui/settings/ProjectSettingsDialog.ts`) düzenlenir.
- Uygulama ayarları **Araçlar → Uygulama ayarları…** penceresinde (`ui/settings/AppSettingsDialog.ts`, `Ctrl+,`) düzenlenir.
- İki pencere de `SettingsShell` kullanır ve kapsamını sol altta açıkça yazar. Bir ayar asla iki pencerede birden durmaz. Karşı pencereye bağlantı verilebilir (ör. "Proje ayarlarını aç").
- Bir değer hem varsayılan hem proje değeri olarak varsa (SRID gibi): varsayılan uygulama ayarıdır, projedeki kopya proje ayarıdır. Varsayılanı değiştirmek açık projeyi etkilemez.

### 4.5 Komutlar (`core/commands.ts`, `app/commands.ts`)

Kullanıcının yaptığı her şey bir `Command`'dır: menü, araç çubuğu, araç
kutusu, kısayol, komut satırı ve bağlam menüsü hep aynı komutu çağırır.

```ts
{ id: 'view.zoomExtents', title: 'Tümünü göster', category: 'Görünüm', icon: 'zoomExtents',
  aliases: ['ZE', 'TUMU'], run: () => view.zoomExtents(),
  isEnabled?: () => boolean, isChecked?: () => boolean, watch?: [signal…] }
```

- **Kimlik biçimi:** `alan.eylem`. Örnekler: `file.save`, `edit.undo`, `view.rightPanel`, `draft.snap`, `tool.line`, `layer.new`, `crs.set`.
- **`watch`:** `isEnabled` ve `isChecked` sonucunu etkileyen sinyallerdir. Düğmeler bunlara abone olarak kendini günceller.
- **`aliases`:** Komut satırından yazılabilen adlardır. Türkçe karakterler katlanır (`CIZGI` = `ÇİZGİ`).
- **Henüz yapılmamış özellikler** `pending(...)` ile kaydedilir (`Command.pending`; hazır olmayan araçların komutu da) ve dürüstçe uyarı verir; şerit onları soluk simge ve “Geliştirme aşamasında” ipucuyla gösterir. Sessizce hiçbir şey yapmayan düğme olmaz.
- **`short`:** dar yerlerde (şerit düğmesi) gösterilen kısa ad (“Bulut projesini yeniden adlandır…” → “Yeniden adlandır”); yoksa başlık kullanılır, sondaki “…” şeritte yazılmaz.
- **Menü modeli** `app/menus.ts` içindedir ve **klasik menü çubuğu ile şeridin tek kaynağıdır.** Menü öğeleri komut kimliğidir; başlık, simge, kısayol ve durum komuttan çözülür. `{ section: 'Başlık' }` başlıklı bir blok açar: menüde ayırıcı, şeritte panel olur; aynı başlıklı bloklar birleşir. `@tools:draw` bir araç grubunun bütün araçlarını bölüm bölüm, `@tools:map/measure` tek bölümü katalogdan getirir; **araçlar menüde tek tek yazılmaz.** Alt menü şeritte açılır düğmedir, `inline: true` ise blokları panel olur. Bir komut bir menüde bir kez yer alır.
- **Şerit** (`app/ribbon.ts` → `RIBBON_TABS`): sekmeler menülerden (`{ menu: 'draw' }`) ve araç gruplarından (`{ tools: 'select' }`) kurulur; aynı başlıklı paneller birleşir, bir komut bir sekmede bir kez yer alır. Yalnız Giriş sekmesi gündelik araçları seçer (`pick`; her biri kendi sekmesinde de vardır; `compact` bütün düğmelerini küçük, `workspaces` yalnız o modlarda gösterir) ve sunum ipuçları taşır (`lead`, `keep`, `launchers`). **Düğme boyu anlamdan gelir, sıradan değil:** ana araçlar (katalogda `primary`) ve `PRIMARY_COMMANDS` büyük, geri kalanı küçük; ana öğesi olmayan bir ya da iki öğeli panel büyük; panelde en çok dört büyük (`MAX_LARGE`). Bir araç ailesi (`family`) ve yöntemleri olan araç (`methods`) tek bölünmüş düğmedir (son seçilen üstte; yöntem aracı o seçenekle başlatır), seyrek araçlar (`rare`) panel başlığındaki ▾ listesindedir (DESIGN.md §7.3.1). Sekme çalışma moduna göre adlanabilir (`labels`), mod boşalttığı sekmeyi göstermez; hızlı erişim Kaydet, Geri al, Yinele ile başlar (`QUICK_ACCESS`). `app/menus.test.ts` kataloğun her aracının menülerde, `app/ribbon.test.ts` her aracın ve menülerin her komutunun şeritte olduğunu ve boyut kurallarını, `app/workspaces.test.ts` her modda gösterilenlerin menüde ve şeritte olduğunu, gizlenenlerin olmadığını denetler.

### 4.6 Kısayollar (`core/keymap.ts`, `app/keybindings.ts`)

- **Akor biçimi:** `Ctrl+Shift+Z`, `Alt+P`, `L`, `F3`, `+`, `Ctrl+,`. Sıra her zaman Ctrl, Alt, Shift'tir.
- **Harfler basılan karaktere göre eşlenir** (`e.key`, ı ve i → I). Böylece Türkçe Q ve F klavyede de tuşun üstünde yazan harf çalışır. Değiştirici tuş karakteri bozarsa fiziksel konuma (`e.code`) düşülür.
- **Metin kutusunda** yalnızca `allowInInput: true` olan bağlar çalışır. Bunlar F tuşları ve `Ctrl+S`, `Ctrl+O`, `Ctrl+P` gibi global komutlardır.
- **Odak bir düğme, ağaç satırı ya da menüdeyken** Enter ve Boşluk yerel anlamını korur.
- **Komut çalışırken seçenek harfleri önceliklidir** (`Keymap.intercept`, `ui/bottom/CommandLine.ts`): Shift'siz ve Ctrl'siz basılan harf istemdeki bir seçeneğin tuşuysa (ör. düzgün çokgende `S`, çoklu çizgide `Y`, çizgide `G`) o seçenek tek tuşla çalışır; Boşluk ya da Enter gerekmez ve aynı harfli araç kısayolu çalışmaz. Eşleşmeyen harf araç kısayolu olarak kalır; Shift'li kısayollar hiç etkilenmez. Seçenek düğmelerindeki tuş etiketi bu yüzden gerçekten o tuşu gösterir.
- **Türkçe harfli seçenekler** Türkçe işaretsiz harfle de seçilir: “Çap (Ç)” C'ye, “Şerit (Ş)” S'ye basınca çalışır (önce tam eşleşme aranır; `optionForKey`). Ç tuşu olmayan klavyede de tek tuş yeter.
- **Hiçbir bağa uymayan** rakam, `@` ya da `.` basılırsa komut satırı odak alır. Böylece koordinat hemen yazılabilir (`Keymap.fallback`).
- **Açık bir pencere** (Dialog) içindeki tuşlar uygulama kısayollarına ulaşmaz.
- **Kısayol listesi** (F1) kısayol haritasından üretilir. Elle liste tutulmaz.

### 4.7 Araçlar (`tools/`)

- **`tools/catalog.ts`:** tek bildirimsel liste. Her araç bir kimlik, etiket, simge, grup (`select | draw | annotate | transform | modify | area | map`), gruplu bölümlerde bölüm (`section`: `tools/Tool.ts` → `TOOL_SECTIONS`, ör. Çizim → Çizgi, Eğri, Şekil, Yardımcı, Nokta), kısayol, takma adlar, açıklama, **fareyle kullanım adımları** (`steps`), şerit sunumu (`primary` ana araç, `family` aile, `methods` başlatma yöntemleri, `rare` seyrek) ve `create(ctx)` içerir. Araç kutusu, klasik menüler, şerit, kısayollar, ipuçları ve komut satırı bu listeden üretilir: **yeni araç yalnız buraya eklenir**, menüye ya da şeride ayrıca yazılmaz. Bölümü olmayan araç grubunun adıyla sonda görünür (`tools/sections.ts`). Yeni araç `steps` olmadan eklenmez: kullanıcı aracı fareyle nasıl kullanacağını ipucundan öğrenir.
- **Henüz yapılmamış araçlar** `create` vermez; `PendingTool` olur, `ready: false` görünür ve ipucunda "Geliştirme aşamasında" yazar.
- **`Tool` sözleşmesi** (`tools/Tool.ts`):
  - `pointerDown/Move/Up`
  - `input(text)`: komut satırı
  - `confirm()`: Enter ya da sağ tık
  - `cancel()`: Esc; `true` dönerse araç kendi içinde halletmiştir (ör. sıcak tutamaç bırakıldı) ve açık kalır
  - `draw(g, view)`: üst katman önizlemesi
  - `snapFrom()`: dik kenetin başlangıç noktası
  - `activeGrip()`: sıcak tutamaç
  - `prompt` sinyali, `cursor`, `snaps` (getter olabilir)
- **Araçlar DOM'a dokunmaz.** Görünüm alanına `ctx.view` üzerinden erişir: `pick`, `pickEdge`, `pickRect`, `edgesIn`, `gripAt`, `trim`, `extend`, `ghosts`, `transformEntities`, `worldTolerance`, `requestOverlay`, `camera`.
- **Araç aileleri** (yeni araç yazarken birine dahil edin):

  | Aile                  | Dosya                                                                                                                 | Akış                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                 |
  | --------------------- | --------------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
  | `PointInputTool`      | `tools/drawTools.ts`, `tools/curveTools.ts`, `tools/shapeTools.ts`, `tools/parallelTool.ts`, `tools/annotateTools.ts` | Nokta dizisi: çizgi (G geri, K kapat), çoklu çizgi (`tools/pathTool.ts`; Y yay parçası: teğet ya da tek parça için A açı, M merkez, R yarıçap, İ ikinci nokta, T doğrultu; D düz; U son doğrultuda uzunluk), halka ve revizyon bulutu (`tools/markupTools.ts`), paralel çizgi (eksen noktaları; S sol, A sağ mesafe yazılır ya da iki tıkla gösterilir, E eksen, U alan olarak, K kapat), alan, nokta, ölçüm, parsel; dikdörtgen (köşe yuvarla/pah, döndür, boyutlar), döndürülmüş dikdörtgen (kenar → genişlik), düzgün çokgen (içten, dıştan, kenardan); daire (merkez-yarıçap/çap, 2N, 3N, TTY, TTT), yay (AutoCAD'in tüm yöntemleri, bkz. §10), eğri; yazı, ölçü |
  | Ölçü                  | `tools/dimensionTool.ts`                                                                                              | Tür başta tek tuşla seçilir: Hizalı (H), Doğrusal (D; yön imlecin yerinden, `Y` yatay ΔY, `X` düşey ΔX sabitler, `O` serbest bırakır), Açı (A; iki kenara tıklanır ya da `K` ile köşe ve iki kol; yayın konduğu bölge açıyı seçer), Yarıçap (R), Çap (Ç). Yerleştirme adımında yazılan sayı ötelenmeyi (açıda yarıçapı) tam verir                                                                                                                                                                                                                                                                                                                                    |
  | Referans hat          | `tools/perpTools.ts`                                                                                                  | Önce bir hatta tıklanır (başlangıç A, tıklamaya yakın uç), sonra ona göre çalışılır: dik in (her nokta hatta dik iner; yay ve dairede merkeze), dik çık (dik ayak tıklanır ya da yazılır, dik boy gösterilir ya da yazılır, sağa artı)                                                                                                                                                                                                                                                                                                                                                                                                                               |
  | Tek tık               | `tools/hatchTool.ts`, `tools/areaTools.ts`                                                                            | Tarama: “Sınır: kapalı nesne” (varsayılan, Netcad gibi) tıklanan yeri çevreleyen en küçük kapalı nesneyi doldurur; içindeki ya da kenarına taşan, ondan küçük kapalı nesneler (parseldeki bina) ada olur ve taranmaz. “Sınır: çizgiler” (B, AutoCAD gibi) görünür çizgilerin kapattığı yüzü doldurur, içteki gruplar ada olur; sınır tek katmana daraltılabilir (K). “Adalar” (A) adaları kapatır. İçine tıklayarak alan aynı yüzleri kullanır (`tools/visibleFaces.ts`: görünüm, çizim ya da katman görünürlüğü değişince yeniden kurulan, çekirdekte tutulan yüz dizini)                                                                                                                 |
  | `SelectionFirstTool`  | `tools/modifyTools.ts`, `tools/arrangeTools.ts`                                                                       | Seçim yoksa önce seçtirir, Enter ile aşamalara geçer, sonucu **afin dönüşümle** uygular: taşı, kopyala, döndür (R referans doğrultu: iki nokta ya da açı, sonra yeni doğrultu; K kopya), ölçekle (R referans uzunluk: iki nokta ya da değer, sonra yeni uzunluk; K kopya), aynala, dizi; kutupsal dizi (merkez; N adet, A doldurma açısı, D nesneleri döndür; önizlemeli, sağ tık uygular), hizala (iki kaynak-hedef çifti, Ö ölçekle; ilk çiftten sonra sağ tık yalnız taşır)                                                                                                                                                                                       |
  | `SelectionActionTool` | `tools/editTools.ts`, `tools/areaTools.ts`                                                                            | Seçim varsa hemen çalışır, yoksa seçtirip Enter bekler: birleştir, patlat; alan birleştir, alan kesiştir, alana çevir, çizgiye çevir                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                 |
  | Alan işlemleri        | `tools/areaTools.ts`                                                                                                  | Alan çıkar (iki seçim: kesilecekler, sonra çıkarılacaklar), alan böl (alanları seç, sonra kesme çizgisini çiz ya da “Çizgiyle kes” ile göster; parçalar ve alanları canlı görünür), içine tıklayarak alan (tek tık; oluşacak bölge önceden boyanır)                                                                                                                                                                                                                                                                                                                                                                                                                  |
  | `EdgePickTool`        | `tools/edgeTools.ts`, `tools/pathEditTools.ts`, `tools/cornerTools.ts`, `tools/lengthenTool.ts`                       | İmlecin altındaki kenara doğrudan etki eder: ötele, buda, uzat; uzat-kısalt (uca tıklanır; dinamikte yeni uç fareyle gösterilir ya da toplam boy yazılır, fark/yüzde/toplam kiplerinde tıklanan uç hemen değişir); kır, böl, köşe ekle/sil; `CornerTool` alt ailesi iki çizgiye ya da çoklu çizginin komşu iki kenarına etki eder: köşe yuvarla, pah                                                                                                                                                                                                                                                                                                                 |
  | Diğer                 | `tools/SelectTool.ts`, `tools/editTools.ts`                                                                           | Seçim (pencere/kesişim, tutamaçla düzenleme), kaydırma, pencere yakınlaştırma; esnet (kesişim penceresi → temel → hedef); yapıştır                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                   |

- **Şeffaf araçlar** (`ctx.tools.nest(child)` / `unnest(point)`): çalışan komutu bitirmeden üstünde açılır (AutoCAD 'CAL gibi). Sonuç noktası üst araca `acceptPoint(p)` ile, tıklanmış gibi verilir; Esc yalnızca şeffaf aracı kapatır. Nokta alan her araç ailesi `acceptPoint`'i uygular (`PointInputTool`, `SelectionFirstTool`, seçim aracında sıcak tutamaç, esnet, yapıştır).
- **Alan işlemleri** (Netcad "Alan işlemleri"; toolbox'ta "Alan" grubu, Değiştir → Alan işlemleri): içine tıklayarak alan (`Shift+B`), alana çevir (`Alt+G`), alan birleştir (`Alt+B`), alan kesiştir (`Alt+K`), alan çıkar (`Alt+C`), alan böl (`Alt+L`), çizgiye çevir. Geometri `model/geom/region.ts`'tedir (bkz. §4.8.1); araçlar yalnızca seçer, önizler ve tek geri alma adımı kaydeder. Kurallar:
  - **Veri kimde kalır:** birleştirmede ilk seçilen alan katmanını, rengini, özniteliklerini ve etiketini verir (tevhit). Kesiştirmede kaynaklar kalır (`S` ile silinir), ortak parça boş öznitelikle eklenir. Çıkarmada kesilen alan özniteliklerini korur, çıkarılan alan yerinde kalır (`S` ile silinir). Bölmede her parça özgün alanın özniteliklerini taşır; kullanıcı parsel numaralarını günceller (ifraz ayrı araçtır ve topoloji üzerinde çalışacak, §7).
  - **Kesilmeyen alan değişmez:** çıkarma bir alana dokunmuyorsa o alan yeniden yazılmaz (daire çokgene çevrilmez).
  - **Alana çevir:** kapalı çoklu çizgi (ilk ve son nokta aynı), daire, tam elips ve kapalı eğri yerinde alana döner. Açık çizgiler (çizgi, yay, açık çoklu çizgi) yerinde kalır; kapattıkları her bölge yeni bir alan olur (bindirme ve kesişmeler dahil).
  - **İçine tıklayarak alan:** sınır kümesi görünümdeki görünür çizgilerdir (nokta, yazı, ölçü ve tarama hariç); “Sınır katmanı (K)” ile tek katmana daraltılır (ör. eşyükseltiler bölgeyi bölmesin). “Adalar (A)” açıkken bölgenin içindeki kapalı şekiller delik olur. Sonuç etkin katmana eklenir. Yüzler bir kez hesaplanır, görünüm ya da çizim değişince yenilenir (`faceIndex`).
- **Nokta hesabı** (`tools/pointCalc.ts`, Netcad'in Koordinat hesap makinası): nokta beklenirken komut şeridindeki "Nokta hesabı" düğmesi, basılı sağ tık menüsü ya da komut satırında takma adla açılır: yan nokta (YAN: dik ayak, dik boy sağa artı), kenar kesişimi (KKES: iki uzaklık; iki çözümden biri tıklanır), doğru kesişimi (DKES: 4 nokta), hat üzerinde nokta (HAT: uzaklık ya da a/b), açı-mesafe (AM: bakılan doğrultudan saat yönünde, proje açı biriminde), iki nokta ortası (ORTA). Ölçmecilik yapıları `model/geom/survey.ts`'te, aracın kendi hesapları (orta nokta, oran, açı birimi, iki çözümden tıklanana yakını) Rust çekirdeğindedir (`tools/constructions.ts` → `geometry-core::tools`). Menüde (`ui/shell/calcMenu.ts`) her yöntemin krokisini gösteren bir simgesi (tıklanan noktalar tutamaç, hesaplanan nokta halka) ve ne hesapladığını, nereye tıklanıp ne yazılacağını söyleyen bir açıklama satırı vardır; kullanıcı yöntemi adından tanımak zorunda kalmaz.
- **Katalog dışı araçlar** (`ctx.tools.run(tool, label)`): içeriği o anki duruma bağlı olan araçlar (ör. panodaki nesnelerle `PasteTool`) katalogda durmaz, "son komutu yinele"ye girmez.

- **İmleç kısıtlaması** tek yerdedir (`tools/tracking.ts` → `constrainPoint`; orto ve kutupsal kilidi Rust çekirdeği hesaplar: `constrainCursor`). Öncelik sırası: nesne keneti, nesne izleme, orto (Shift tersine çevirir), kutupsal izleme (F10, adım `prefs.polarIncrement`). Kutupsal kilit ışına 10 px yaklaşınca devreye girer.
- **Nesne izleme** (`viewport/objectTracking.ts`: hesap Rust çekirdeğinde, `geometry-core::tools::object_tracking`; `settings.tracking`, Shift+F3, durum çubuğunda "İzleme"): nokta bekleyen bir komutta uç, orta, merkez, düğüm, çeyrek ya da kesişim keneti üzerinde 350 ms beklemek o noktayı izleme noktası yapar (yeşil artı, en çok 3). Aynı noktada tekrar beklemek bırakır. İmleç bir izleme noktasının yatay/dikey hizasına (kutupsal açıksa açı adımlarına) 8 px yaklaşınca kilitlenir. İki hizanın kesişimi tek hizadan önce gelir; aracın son noktası (`snapFrom`) yalnızca kesişimlere katılır. Eksen yönleri tam değerle hesaplanır, böylece "tam üstü" aynı X'i verir. Kenet varken izleme uygulanmaz. İzleme varken yazılan tek sayı izleme noktasından hiza boyunca mesafedir: araçlar yazılan noktayı `pointFromText` ile çözer (`parsePointInput` + `view.trackAlong`). İzleme noktaları komut değişince silinir.
- **`ToolPointer.track`:** kilitlenilen hiza. `world` önceliği `snap → track → raw`'dır.
- **Tutamaçla düzenleme** (seçim aracı): seçili ve kilitsiz bir nesnenin tutamacını sürüklemek o noktayı taşır. Tıklayıp bırakmak tutamacı "sıcak" yapar; sonraki tıklama ya da yazılan koordinat yerleştirir, Esc vazgeçer. Kenet ve kutupsal izleme bu sırada çalışır. Tutamaç anlamları `model/ops/grips.ts` içindedir.
- **Kenar ortası tutamaçları:** çoklu çizgi ve kapalı alanda köşelerden sonra her kenar için bir orta tutamaç gelir (içi boş baklava). Düz kenarda sürüklemek oraya yeni köşe ekler, yay kenarında yayı sürüklenen noktadan geçecek biçimde büker. Ekranda 28 px'ten kısa kenarlarda gösterilmez (`midGripVisible`).
- **Pano:** `Ctrl+C` seçimi `ctx.clipboard`'a derin kopya olarak alır; taban noktası sınır kutusunun sol alt köşesidir. `Ctrl+V` bu köşeyi imlece bağlayıp tıklanan yere koyar, `Ctrl+Shift+V` özgün koordinatlara yapıştırır. `Ctrl+X` kopyalayıp siler. Nesne katmanı yoksa ya da kilitliyse etkin katmana yapıştırılır.
- **Yerinde yazı düzenleme:** Seçim aracında yazı ya da ölçüye çift tıklamak `view.requestTextEdit(id)` çağırır. Asıl düzenleyici arayüz katmanındadır (`ui/shell/InlineTextEditor.ts`), çünkü araçlar DOM'a dokunmaz. Düzenleyici yazının üstüne aynı boyut ve açıyla oturur; düzenlenen yazı üst katmanda gizlenir (`view.setEditing`).
- **Yazılı seçenekler** aşamaya göre değişir: sayı (açı, faktör, mesafe, yarıçap, satır/sütun), harf (`K` kopya, `S` kaynağı sil, `K` kapat, `G` geri).
- **İstem düzeni (bağlayıcı):** `Araç: adım [Seçenek (TUŞ) / Seçenek (TUŞ): değer; not]`. `ui/promptOptions.ts` bunu ayrıştırır ve her seçeneği komut şeridinde (çizim alanının üstü) ve komut satırında **düğmeye** çevirir; düğme, tuşu yazmakla aynı işi yapar (`tool.input(TUŞ)`, `Enter` → onay, `Esc` → iptal). Köşeli parantez içinde `(TUŞ)` taşımayan parçalar not olarak gösterilir. Bölücüler ` / ` ve `;`'dür; bu yüzden değerlerin içinde bu karakterler kullanılmaz. Fareyle ulaşılamayan bir seçenek yazılmaz.
- **Fare önce gelir:** her araç yalnızca fareyle tamamlanabilmelidir. Sayı gerektiren yerlerde fareyle gösterme yolu sunulur (ör. köşe yuvarlamada yarıçap imleç çekilerek, ötelemede "Noktadan geç"), yazılan değer yalnızca kesinlik içindir.
- **Sağ tuş (zamana duyarlı, `ViewportController.onRightDown/onRightUp`):** kısa sağ tık çalışan komutta Enter'dır, seçim aracında bağlam menüsünü açar. 300 ms basılı tutmak komut menüsünü açar (onayla, iptal, istemdeki seçenekler, tek seferlik kenet, orto, kutupsal). Shift+sağ tık doğrudan kenet menüsünü açar. Tarayıcının `contextmenu` olayı yalnızca engellenir; zamanlaması işletim sistemine göre değiştiği için kullanılmaz. Menüleri `ui/shell/viewportMenus.ts` kurar (`contextmenu` olayı `kind: 'select' | 'command' | 'snap'` taşır).
- **Tek seferlik kenet:** `view.snapOverride` bir sonraki sol tıklamada yalnızca seçilen türe kenetler (F3 kapalı olsa bile) ve tıklamadan sonra kendiliğinden temizlenir. Komut şeridi bunu "Sonraki tık: …" etiketiyle gösterir. Kenet menülerinde her tür, çizimdeki işaretiyle aynı biçimde çizilmiş bir simge taşır (`ui/icons.ts` → `snapEndpoint` …).
- **İmleç yanında değer girişi** (`ui/shell/CursorInput.ts`, `prefs.cursorInput`): komut çalışırken ve fare çizim alanındayken rakam, `@` ya da `.` basılırsa alan imlecin yanında açılır; metni komut satırıyla aynı yoldan (`tool.input`) gönderir. Kapalıyken ya da fare çizim alanı dışındayken komut satırı kullanılır.
- **Bilgi kartı** (`ui/shell/HoverCard.ts`, `prefs.hoverInfo`): seçim aracında bir nesnenin üzerinde 500 ms durunca tür, katman, ada/mahalle/nitelik, tapu alanı ile hesaplanan alan, uzunluk ya da yarıçap gösterilir.
- **Tutamaç menüsü:** seçili çoklu çizgi ya da alanın köşe tutamacına sağ tık "Köşeyi sil", kenar ortası tutamacına sağ tık "Ortasına köşe ekle" ve "Yaya dönüştür" ya da "Düz kenar yap" sunar.
- **Seçim isteyen araçlar** (`SelectionFirstTool` ve alt aileleri) seçim aşamasında tıklamayla tek tek ve sürüklemeyle pencere/kesişim seçimi kabul eder; sağ tık seçimi onaylar.
- **Buda ve uzat** (`tools/edgeTools.ts`, `BoundaryEdgeTool`): sınır varsayılan olarak görünen bütün kenarlardır (AutoCAD hızlı kip). “Sınır seç (S)” ile sınır nesneleri tıklanır, sağ tık onaylar; seçilen sınırlar seçim olarak vurgulu kalır, “Tüm kenarlar (T)” geri döner. Shift+tık öbür işlemi yapar (budada uzatır, uzatta budar). Önizlemeyi ve sonucu geometri deposu hesaplar (`view.trim`, `view.extend`): hesaba yalnız hedefe (uzatmada ucun ışınına ya da çemberine) değebilecek kenarlar girer, sonuç bütün kenarlarla aynıdır.
- **Köşe yuvarla ve pah** (`tools/cornerTools.ts`): imleç bir köşeye (çoklu çizgi köşesi ya da iki çizginin birleştiği uç) 12 px yaklaşınca köşe halkayla işaretlenir. Tıklayınca köşe kilitlenir; imleç bir kenar boyunca çekildikçe teğet/kesim mesafesi canlı büyür (yakınlığa göre yuvarlanmış adımla), ikinci tık uygular. Yazılan değer tam uygular, sağ tık son değeri kullanır. Birleşmeyen iki çizgide sırayla iki çizgiye tıklanır. Araç uygulamadan sonra bir sonraki köşeyi bekler (AutoCAD'in Çoklu kipi). “Kırp (K)” iki araçta ortaktır: kapalıyken kenarlar olduğu gibi kalır, yalnızca yay ya da pah çizgisi eklenir (TRIMMODE=0).
- **Semantik:**

  | Tuş                  | Davranış                                                                       |
  | -------------------- | ------------------------------------------------------------------------------ |
  | Esc                  | Araçtan çıkar, seçime döner. Seçim aracındaysa seçimi temizler.                |
  | Enter / sağ tık      | Geçerli nesneyi bitirir; araçta kalınır. Bekleyen bir şey yoksa araçtan çıkar. |
  | Seçim aracında Enter | Son aracı tekrarlar.                                                           |
  | Boşluk               | Komut satırına gider.                                                          |

- **Koordinat girişi** (`tools/coordinateInput.ts`; metni TS çözer, noktayı Rust çekirdeği kurar):
  - `Y,X` mutlak
  - `@dY,dX` göreli
  - `@mesafe<açı` kutupsal (derece, doğudan saat yönünün tersine)
  - tek sayı: imleç doğrultusunda mesafe

  Ondalık ayırıcı nokta, koordinat ayırıcı virgüldür.
- **Harita araçlarının hedef katmanları** (`parsel`, `kot`) katalogdaki `LAYERS` yapılandırmasındadır. Araç kodunda katman kimliği sabit yazılmaz.

### 4.8 Belge modeli (`model/`)

- **`CadDocument`:** varlıklar `Map<id, Entity>` içinde durur. Bütün düzenlemeler `add`, `update`, `remove` ve bunları gruplayan `transact(label, fn)` üzerinden yapılır. Her işlem tersine çevrilebilir bir `Op` olarak kaydedilir; geri alma yığını 200 adımla sınırlıdır. Çok nesneye dokunan bir komut `addMany`, `updateMany` ve `remove`'u kullanır: her nesne yine kendi işlemidir (sırayla `update` ile aynı sonuç), ama bütünü tek değişikliktir, dinleyiciler her olayı bir kez duyar (tarayıcıda 10 000 nesneyi taşımanın belge adımı nesne nesne 145 ms, tek değişiklikte 14 ms); işlemin içinde ona katılırlar. `transact` ya hep ya hiç çalışır: `fn` hata fırlatırsa yaptıkları geri alınır, hiçbir şey kaydedilmez ve `dirty` değişmez. İç içe çağrı bir kayıt noktasıdır; gerekçe [docs/adr/0003-transaction-semantics.md](docs/adr/0003-transaction-semantics.md)'de.
- **Kaydedilmemiş işareti (`dirty`) belgenin sürümünden gelir:** her kayıt, geri alma, yineleme, proje ayarı, ad, stil kitaplığı ve katman ağacı ya da katman durumu değişikliği `doc.revision`'ı artırır. `markSaved(revision)` yalnızca yazılan sürüm hâlâ güncelse işareti temizler; yazım sürerken yapılan değişiklik kaydedilmemiş kalır.
- **Çizim dosyası (.kcad)** sürümlü `DocumentSnapshotV1`'dir (`model/snapshot.ts`, sözleşme `crates/shared/contracts`): nesneler, katman ağacı, proje ayarları, yerel orijin, başlangıç görünümü ve projenin stil kitaplığı. Koordinatlar JSON'da float64 olarak bit bit korunur. Okuyucu biçimi, sürümü ve her alanı doğrular; bilinmeyen sürüm, tür ya da SRID “yer: sorun” biçiminde Türkçe hatayla reddedilir, tahmin edilmez. `doc.replaceWith(içerik)` açık belgeyi yerinde değiştirir (`ctx.doc` aynı nesne kalır), geçmişi siler ve belgeyi temiz başlatır; açık bir işlem ya da grup varken reddedilir.
- **Kaydet/Aç** (`app/fileIO.ts`, `ctx.files`): Kaydet (`Ctrl+S`) açılan ya da son kaydedilen dosyaya sormadan yazar, ilk seferde Farklı kaydet (`Ctrl+Shift+S`) gibi yer sorar; proje dosyanın adını (uzantısız) alır. Tarayıcının dosya penceresi (File System Access API) kullanılır; işaret yalnızca yazıcı hatasız kapanınca temizlenir. Dosyaya yazamayan tarayıcıda çizim indirme olarak verilir ve kaydedilmemiş sayılır, çünkü saklandığı doğrulanamaz. Aç (`Ctrl+O`) kaydedilmemiş değişiklik varsa önce sorar (Kaydet ve devam et / Kaydetmeden devam et / Vazgeç); kendiliğinden kaydeden bir bulut projesinde sorulmaz, çünkü değişiklikler gönderilir ya da cihaz taslağında kalır (izleyicinin düzenlemeleri kalmadığı için ona sorulur). Dosyadaki proje stilleri paylaşılan .kstil gibi doğrulanır. Üretim derlemesinde kaydedilmemiş değişiklikle sekme kapatılırken tarayıcı sorar. Duman testi tarayıcı penceresi yerine bellek içi bir seçici (`kentos.files.picker`) kullanır.
- **Yeni proje** (Dosya → Yeni proje…, `Ctrl+Alt+N`, `YENI`; `ui/settings/NewProjectDialog.ts`, `model/newProject.ts`): ad, koordinat sistemi (varsayılanı `prefs.defaultSrid`, ortak EPSG seçici) ve çizim ölçeğiyle boş bir çizim açar. Katman ağacı örnek projeninkidir (`model/standardLayers.ts`; harita araçlarının hedefi `parsel` ve `kot` dahil), birimler varsayılandır. Yerel orijin dilimin çalışma alanı ortasıdır (`geo/crs.ts` → `workAreaCentre`: TM/UTM'de Y 500 000, X 4 320 000); başlangıç görünümü yoktur, ekran seçilen ölçekte bir paftalık alanı gösterir. Oluştur'a basınca kaydedilmemiş yerel değişiklik pencerenin üstünde sorulur (Vazgeç pencereye döner); açık bulut projesi bırakılır (`ctx.cloud.leave`: bekleyenler en çok 5 sn gönderilir, gerisi cihaz taslağında kalır ve kullanıcıya söylenir). Dosya tutamacı sıfırlanır: ilk Kaydet yer sorar.
- **Dosya alışverişi** (`app/fileExchange.ts` komutları, `io/`, `ui/io/`, Rust `crates/shared/formats`; §9.7, ADR 0009):
  - Komut önce dosya penceresini açar (tarayıcı kullanıcının tıklamasını ister); pencerenin kodu yanında yüklenir. Dosya ayrı bir Web Worker'da Rust biçim modülüyle okunur.
  - Kaynağın koordinat sistemi her zaman sorulur (“Bu koordinatlar hangi sistemde?”, varsayılan projeninki). Başka bir sistem seçilirse içe aktarma kapanır ve nedeni yazar: datum ve dilim dönüşümü yok, koordinatlar sessizce dönüştürülmez (§5).
  - Okunan nesneler `.kcad` okuyucusunun denetiminden geçer (`readEntityList`); yeni katmanlar kurulur (katman kurmak geri alınmaz, işlem araçlarındaki gibi) ve bütün nesneler **tek geri alma adımı** ve tek değişiklik olayıyla eklenir (`CadDocument.addMany`); görünüm içe aktarılanlara yakınlaşır. Açık bir düzenleme ya da çalışan bir işlem modelinin grubu varken içe aktarma yapılmaz, beklenmesi söylenir (modelin geri alma adımına katılır, modelin iptaliyle geri alınırdı). Büyük dosya worker'a kopyalanmadan verilir (`readDxf` bütün arabelleği devreder). Dışa aktarma çizimi kaydetmez, `dirty`'ye dokunmaz.
  - **Koordinat listesi** (Dosya → İçe aktar → Koordinat listesi, Koordinat → Nokta listesi içe aktar; `NCN`): Netcad NCN, TXT, CSV. Ayırıcı (boşluk, sekme, `;`, `,`), ondalık işaret (nokta; `;` ya da sekmeyle ayrılmış Türkçe tablolarda virgül), başlık satırı ve kodlama (UTF-8, UTF-16, Windows-1254) bulunur. Sütunlar (Ad, Y, X, Z, Kod) başlık adlarından ya da Netcad sırasından (Ad Y X Z) sayıların büyüklüğüne göre önerilir (TM'de sağa değerler 10⁶'dan küçük, yukarı değerler büyük); önizleme tablosunun başlığından ya da sıra düğmelerinden değiştirilir ve her seçim dosyayı yeniden okur. Nokta olmayan satırlar satır numarası ve nedeniyle listelenir, alınmaz. Noktalar seçilen ya da dosya adıyla kurulan katmana `label` ve `Ad` (varsa `Kod` ve dosyadaki basamaklarla `Z (m)`) öznitelikleriyle gelir; koordinatlar dosyadaki ondalığa en yakın float64'tür, yuvarlanmaz.
  - **DXF içe aktar** (Dosya → İçe aktar → DXF; `DXF`): ASCII DXF (R12–2018). DWG açılamaz; kapalı biçim olduğu komut açıklamasında ve DWG seçilince yazar ("DXF olarak kaydedin"), ikili DXF de aynı biçimde reddedilir.
    - Kodlama: AutoCAD 2007 (`$ACADVER` AC1021) ve sonrası UTF-8; öncesi `$DWGCODEPAGE` (ANSI_1254, ANSI_1252; yoksa Türkçe Windows; başka bir sayfa Windows-1252 okunur ve raporlanır), ama ASCII dışı baytları geçerli UTF-8 olan eski dosya UTF-8 okunur. `\U+XXXX` kaçışları, `%%d/%%p/%%c` kodları çözülür. `$INSUNITS` rapora yazılır ama uygulanmaz: koordinatlar ölçeklenmez.
    - Katmanlar LAYER tablosundan (DXF katman adları büyük/küçük harf ayırmaz: nesnenin katmanı tablodaki yazılışıyla alınır): ACI renk (7 → `ink`; 250–255 AutoCAD'in gri tonları) ya da gerçek renk (420), dondurulmuş/kapalı → gizli, kilit, çizgi tipi (LTYPE deseninden: kesikli, noktalı kesik, noktalı), kalınlık (370). Pencerede her DXF katmanı nesne sayısıyla listelenir; adı (büyük/küçük harf ve Türkçe işaretler katlanarak) projede varsa oraya eklenir, yoksa dosya adıyla kurulan grupta yeni katman olur; işareti kaldırılan katman alınmaz, kilitli hedef seçilemez.
    - Nesneler: LINE, LWPOLYLINE ve 2B POLYLINE (bulge birebir; kapalı → kapalı alan), 3B POLYLINE, CIRCLE, ARC (hep saat yönünün tersine), ELLIPSE, SPLINE (geçiş noktaları → eğri; yalnız denetim noktalıysa 1 mm içinde çoklu çizgi; derecesi 25'ten büyükse alınmaz), POINT (Z ≠ 0 kot olur), TEXT/ATTRIB (hizalı yazı 11. noktadan, genişlik tahminiyle sol alta taşınır), MTEXT (biçimlendirme temizlenir, satırlara bölünür), HATCH (çoklu çizgi sınırları uygulamanın kendi yaylı halkalarını örneklediği gibi, ortak çekirdekle; kenar yayları ve elipsleri aynı adımla, 72 parça/tur; hesaplanamayan eğri kenar denetim noktalarıyla alınır ve raporlanır; adalar iç içeliğe göre; desen: dolu, tek çizgi ailesi, dik iki aile → çapraz), SOLID/TRACE/3DFACE → kapalı alan, XLINE/RAY, LEADER → çoklu çizgi; DIMENSION ve ACAD_TABLE kendi anonim bloklarından çizgi ve yazılara patlatılır (tanım noktaları alınmaz). KentOS'un yazdığı ölçü, dosyanın en üst düzeyinde ve DXF'teki türü ile tanım noktası (10) yazıldığı gibi durdukça KentOS verisinden ölçü olarak geri okunur; başka bir programda değiştirilmişse bloğundan patlatılır ve söylenir.
    - INSERT (MINSERT dizileri, iç içe bloklar): tam dönüşümle patlatılır; 0 katmanındaki alt nesneler eklemenin katmanını, BYBLOCK renkli olanlar eklemenin rengini alır; kendini içeren blok, eksik blok, dış başvuru (xref) ve okunamayan öznitelik (ATTRIB) raporlanır. MINSERT en çok 10 000 sütun ve 10 000 satır açılır. Blokları açmak, alınabilecek her nesne için en çok 8 adım sürer (varsayılan 8 milyon): hiçbir şey çizmeyen blokların iç içe eklemeleri de okuyucuyu durduramaz, sınıra varılırsa raporlanır. Nesne koordinat sistemi (210) keyfî eksen algoritmasıyla uygulanır: yaygın (0,0,−1) durumunda x aynalanır, yay yönü ve bulge işareti döner. Benzerlik dönüşümü şekli korur (daire daire, bulge bulge kalır, dörtte bir dönüşler tam); eşit olmayan ölçek daireyi elipse çevirir, yaylı kenarları noktalara böler ve bunu raporlar. Birim dönüşüm koordinatları bit bit kopyalar.
    - Alınmayanlar (kâğıt uzayı, IMAGE, 3B katılar, MLINE, MLEADER, bilinmeyen türler) ve dönüştürülenler türüyle, sayısıyla ve ilk satır numaralarıyla raporlanır; hiçbiri okumayı durdurmaz. Bir dosyadan en çok 1 000 000 nesne alınır (fazlası raporlanır).
  - **Koordinat listesi dışa aktar** (Dosya → Dışa aktar; `NCNYAZ`): seçili, görünen katmanlardaki ya da bütün nokta nesneleri; NCN (boşluk), TXT (sekme), CSV (`;` ya da `,`), sütun sırası, başlık satırı, UTF-8 ya da Windows-1254. Değerler geri okununca aynı float64'ü veren en kısa ondalıkla yazılır.
  - **DXF dışa aktar** (Dosya → Dışa aktar → DXF; `DXFYAZ`; `ui/io/DxfExportDialog.ts`, Rust `dxf/writer/`): seçili, görünen katmanlardaki ya da bütün nesneler katmanlarıyla (pencerede katman başına sayılarıyla listelenir; işareti kaldırılan katman yazılmaz) AutoCAD 2007 ASCII DXF'i (AC1021) olur. Bu sürüm metni UTF-8 tutan (Türkçe harfler kod sayfası tahminsiz), gerçek renk ve çizgi kalınlığı taşıyan en eski sürümdür; birim metredir (`$INSUNITS` 6). Dosyada AutoCAD 2000+ çiziminin istediği her şey vardır (tutamaçlar, sahipler, dokuz tablo, model ve kâğıt uzayı blokları, kök sözlük, yerleşimler); ezdxf denetiminden hatasız geçer.
    - Koordinatlar geri okununca aynı float64'ü veren en kısa ondalıkla yazılır. Her türün aynı veriyi tutan bir DXF nesnesi vardır: POINT (Z), LINE, LWPOLYLINE (bulge birebir; kapalı alan kapalı), CIRCLE, ARC, ELLIPSE (parametreleriyle), XLINE/RAY, TEXT (tek satır: satır sonları boşluk olur), HATCH (sınır ve adalar çoklu çizgi yolu; dolu ya da kullanıcı tanımlı desen: bir çizgi ailesi, çaprazda dik iki aile).
    - Üç tür biçim değiştirir: adalı alanın adaları, alanın tutamacını taşıyan kendi kapalı çoklu çizgileridir; eğri, uygulamanın merkezcil Catmull-Rom eğrisinin açıklık açıklık tam Bézier biçimi olan kübik B-spline'dır (çekirdeğin `catmull_rom_beziers`'i; geçiş noktaları da yazılır), bir uydurma değildir; ölçü gerçek DXF ölçüsüdür (DIMENSION; `dxf/dimension.rs`): kendi anonim bloğu (`*D1` …) KentOS'un çizdiği uzatma çizgilerini ve eğik uçları, açı ölçüsünün yayını tek ARC ve değeri MTEXT olarak taşır, böylece dosya her programda KentOS'taki gibi görünür; türü (hizalı, döndürülmüş, üç noktalı açı, yarıçap, çap) ve tanım noktaları çekirdeğin ölçü yerleşiminden gelir, Standard stilinin boyları ölçü başına geçersiz kılınır (AutoCAD'in DSTYLE verisi: yazı yüksekliği, eğik uç, aralıklar, yazının çizginin üstünde durması, basamaklar, açı birimi), Standard stili değeri KentOS gibi gösterir (ondalık nokta, sondaki sıfırlar, yuvarlama yok); böylece ölçüyü düzenleyip yeniden çizen program da yakın çizer. Kendi yazısı olmayan ölçünün bloğuna çizimde görünen değer yazılır, ölçünün yazısı boş kalır (başka program yeniden ölçer). KentOS'ta yazı tek satırlıdır (satır içi düzenleyici tek satır, MTEXT içe aktarılırken satırlarına bölünür): yazılar TEXT'tir, MTEXT yalnız ölçü değerindedir.
    - DXF'in söyleyemediği, nesnenin KentOS verisi (XDATA, uygulama adı `KENTOS`) olarak yazılır: etiket, öznitelikler, sembol, tema renkleri, yayın dereceye yuvarlanan radyanları, taramanın desen ötelemelerine yuvarlanan açı ve aralığı, noktanın 0 kotu, eğrinin KentOS eğrisi olduğu ve adanın hangi alanın olduğu. Okuyucu bu veriyi yalnız DXF'in sayıları hâlâ aynı şeyi söylerken alır: başka bir programda değiştirilen renk ya da açı kazanır. Böylece KentOS'un yazdığı DXF KentOS'a aynı nesneler olarak (ölçüler dahil) bit bit geri okunur. Nesne başına veri AutoCAD'in 16 KB sınırını aşarsa etiket, öznitelik ve sembol yazılmaz ve söylenir.
    - Katmanlar: DXF katmanları düz bir listedir ve adları büyük/küçük harf ayırmaz; aynı adlı katmanlar grup adlarını önlerine alır (“Kadastro - Sınır”), DXF'in kabul etmediği karakterler `_` olur. Gizli katman kapalı, kilitli katman kilitli yazılır; çizgi tipleri (kesikli, noktalı kesik, noktalı) çizim ölçeğinde kâğıt milimetresine göre boyutlanır, kalınlık AutoCAD'in en yakın kalınlığına iner. `ink` DXF rengi 7'dir; `fg` 7'ye, `fg-dim` 8'e iner (KentOS verisiyle tema rengi olarak geri gelir). Katman stilleri (işleyiciler, semboller, dolgu, etiket biçimi) yazılmaz.
    - Pencere yazmadan önce neyin değişeceğini söyler (DXF ölçüsü olarak yazılan ölçüler, adalar, KentOS verisi, tema renkleri, stiller, aynı adlı ve gizli katmanlar); yazıcının raporu (yaklaşıklıklar ve yazılmayanlar, Türkçe) günlüğe düşer. Sayı olmayan (sonsuz ya da tanımsız) değeri olan nesne yazılmaz ve adıyla söylenir.
- **Bulut projesi** (`app/cloud/`, `ctx.cloud`, Faz B): Dosya → Bulut projesi aç / Buluta yükle; oturum durum çubuğundaki sunucu hücresinin menüsünden ya da Buluta giriş penceresinden (yerel hesap ya da kurumun OpenID girişi) açılır.
  - **Açma:** önce proje bilgileri ve olay imleci, sonra nesneler 2 000'lik sayfalarla; iptal edilebilir, eski bir açılışın geç gelen yanıtı atılır. Sunucudan ve cihazdan gelen her şey `.kcad` okuyucusuyla (proje stilleri `.kstil` gibi) denetlenir (`cloud/incoming.ts`).
  - **Yükleme:** proje bütün katmanlar kilitsiz açılır, nesneler 2 000'lik komutlarla gider, sonra çizimin kendi katman ağacı (kilitleriyle) geri konur; sunucu kilitli katmana yazmayı reddeder (§7).
  - **Otomatik kayıt** (`cloud/sync.ts`, `syncCore.ts`): nesneler sunucunun onayladığı hâlle karşılaştırılır (`tracker.ts`: yerel numara ↔ UUID ve sürüm; geri almayla kayıtlı hâle dönmek bir şey göndermez). Son düzenlemeden 1 sn sonra (ilkinden en geç 5 sn) tek `project.changes` komutu gider; sırada tek komut olur. Değişiklikler ve yoldaki komut gönderilmeden önce IndexedDB'ye yazılır; kopan bağlantıda ya da kaybolan yanıtta aynı komut aynı idempotency anahtarıyla gider (sunucu kaydından yanıtlar, iki kez yazmaz). “Buluta kaydedildi” yalnızca sunucunun yanıtından sonra ve bekleyen bir şey yokken yazar. `Ctrl+S` hemen gönderir. Proje bilgisi değişiklikleri `project.edit` yetkisi ister; yetkisi olmayanda cihazda kalır ve bir kez söylenir. İzleyicinin değişiklikleri gönderilmez (“Salt okunur”).
  - **Çakışma:** 409'da hiçbir şeyin üzerine yazılmaz; gönderim durur, kullanıcı “Sunucudakini al” (varsayılan) ya da “Benimkini kaydet” der (`ui/cloud/ConflictDialog.ts`).
  - **Başka editörler** (`cloud/socket.ts`, `syncRemote.ts`): WebSocket son uygulanan imleçten abone olur, kaçanlar önce gelir; imleç saklanan olaylardan eskiyse (sunucu olayları `KENTOS_EVENT_RETENTION_DAYS` gün tutar, ADR 0006) ya da en yeni olayın ötesindeyse sunucu `resyncRequired` der ve proje yeniden açılır (cihaz taslağı geri gelir); 20 sn'de bir yoklama, 60 sn sessizlik ölü bağlantıdır, yeniden bağlanma 1–30 sn arası geri çekilir. Kendi komutlarımızın olayları istek kimliğiyle atlanır; gelen nesneler geri alma adımı yazmadan uygulanır; gönderilmemiş yerel değişikliği olan nesne üzerine yazılmaz, çakışma olur.
  - **Cihaz taslağı** (`syncRestore.ts`): proje yeniden açılınca gönderilmemiş değişiklikler geri konur; önce yoldaki komut kendi anahtarıyla gider. Tabanı sunucuda değişmiş olan çakışmadır. Yeniden açıldıktan sonra yapılmış bir düzenleme eski taslaktan önce gelir.
  - **Yeniden adlandırma ve silme** (Bulut projesi aç listesinde seçili proje için “Yeniden adlandır…” ve “Sil…”; açık proje için Dosya ve hesap menüsünde `cloud.rename`, `cloud.delete`; `ui/cloud/ProjectActions.ts`): ad bir proje bilgisi değişikliğidir (`project.edit`; açık projede otomatik kayıtla, başkasında sürümüyle `project.changes`). Silme `DELETE …/projects/{id}` ve `project.delete` yetkisi (yönetici, sahip) ister, önce sorar (güvenli düğme “Vazgeç” odakta, “Projeyi sil” kırmızı çizgili). Silme yumuşaktır (ADR 0006): proje listeden kalkar, açma ve yazma 410 `project_deleted` alır, nesneler kalır; işletmeci `kentosd project restore` ile geri getirir. Projeyi açık tutan editör `project.deleted` olayını (ya da bir komutun 410'unu) alınca eşitleme `deleted` durumuna geçer: hiçbir şey gönderilmez, canlı bağlantı kapanır, düzenlemeler cihaz taslağında kalır, durum çubuğu “Proje silindi” der; Kaydet ve hücreye tıklama yerel dosyaya kaydettirir, dosyaya kaydedince çizim silinen projeden ayrılır. Projeyi kendisi silen kullanıcının çizimi ekranda kalır ve projeden ayrılır.
  - **Projeden ayrılma** (`detach`, `leave`): eşitleme kapanınca (`ProjectSync.dispose`, `SyncCore.closed`) yoldaki komutun geç gelen yanıtı, gelen olaylar ve taslak geri koyma çizime artık dokunmaz; çizim o anda başka bir projeyi tutuyor olabilir. Taslak son hâliyle kalır (yoldaki komut anahtarıyla içindedir); `leave` önce onu yazar.
- **Olaylar:**
  - `changed { layerIds }`: geometri ya da üyelik değişti; GPU tamponu yeniden kurulur.
  - `attrs { ids }`: yalnızca öznitelik değişti; tampon kurulmaz, etiket ve panel yenilenir.
- **`load()`:** geçmiş tutmadan toplu yükleme yapar (dosya açma).
- **`touched { ids, layerStyles, external }`:** her uygulanan değişikliğin (düzenleme, geri alma, yineleme, başarısız işlemin geri sarılması, dışarıdan gelen değişiklik) dokunduğu nesneler. Bulut eşitlemesi ve geometri deposu yalnızca bunları karşılaştırır.
- **`reset`:** `load` ve `replaceWith` bütün nesneleri değiştirdi (`touched` gelmez); nesnelerin kopyaları (geometri deposu) baştan kurulur.
- **`applyExternal({ put, remove, meta })`:** başka bir editörün kaydettiği nesneleri ve proje bilgilerini geri alma adımı yazmadan ve kaydedilmemiş saymadan uygular; bu nesnelere dokunan geri alma adımları silinir (`forgetHistoryOf`), böylece geri alma başkasının değişikliğini sessizce geri çeviremez (§15). Açık bir işlem ya da grup varken reddedilir (`busy`); yeni nesneler `allocateId()` ile numara alır. `markUnsaved()` cihaz taslağı geri konunca belgeyi kaydedilmemiş yapar.
- **`beginGroup(label)`:** `end()` çağrılana kadar yapılan bütün işlemleri (await arasında da) tek geri alma adımında toplar; `cancel()` yapılanları geri alır ve hiçbir şey kaydetmez. İşlem modelleri bunu kullanır.
- **`Entity`:** türler `point | line | polyline | polygon | circle | arc | ellipse | spline | xline | ray | text | dimension | hatch`.
  - `ellipse`: DXF ELLIPSE biçimi: merkez `c`, büyük eksen vektörü `major`, küçük/büyük `ratio`, parametreler `t0 → t1` (saat yönünün tersine; eşitse tam elips). Nokta `c + major·cos t + minor·sin t`. Budama, kırma ve uzatma parametre uzayında yapılır, parçalar eliptik yay kalır. Öteleme (matematikte elips değildir) gerçek öteleme noktalarından sık bir çoklu çizgi verir.
  - `xline` / `ray`: taban noktası `p` ve birim yön `dir`; iki yöne ya da tek yöne sonsuz yardımcı çizgi. Tümünü göster ve sınır kutusu yalnızca `p`'yi sayar; pencere seçimi (tamamen içeride) onları hiç seçmez, kesişim seçimi seçer. Budama ve kırma AutoCAD gibi ışın ya da çizgi parçası üretir.
  - `spline`: geçiş noktalarından merkezcil Catmull-Rom eğrisi (`pts`, `closed`); benzerlik dönüşümlerinde tam doğrudur.
  - `dimension`: ölçü (`a`, `b`, `offset`, yazı yüksekliği `height`, isteğe bağlı `text`, `style`). `style` yoksa hizalıdır; `linear` ölçülen doğrultuyu `angle` ile taşır (0 = ΔY yatay, 90 = ΔX düşey), `angular` köşeyi `c` ile taşır ve `offset` yay yarıçapıdır (açı a kolundan b koluna saat yönünün tersine), `radius`/`diameter`'da `a` merkez, `b` çember üzerindedir ve `offset` çemberin dışına uzantıdır. Boş `text` ölçülen değeri proje birimiyle gösterir (açı proje açı biriminde; önek R / Ø); tek geçit `view.dimensionText`. Aralıklar ve ölçü uçları yazı yüksekliğinden türetilir. Yansıtmada hizalı/doğrusal ölçünün `offset` işareti değişir, açı ölçüsünün kolları yer değiştirir.
  - `polygon` delikli olabilir (**adalı alan**): `holes` aynı köşe + bulge biçiminde halkalardır. Alan delikleri çıkarır; çevre, köşeler (kenet) ve kenarlar (budama sınırı, seçme) delikleri içerir; dolgu delikleri boş bırakır (`render/triangulate.ts`, köprüyle ear clipping); delik içine tıklamak alanı seçmez. Dönüşüm, esnet, patlat ve tutamaçlar (delik köşeleri, kenar ortası tutamaçlarından sonra gelir) delikleri izler. Alanı açan işlemler (buda, kır) adalı alanı reddeder, çünkü delikler kaybolurdu; köşe ekle/sil yalnızca dış halkada çalışır. `CadDocument.update` alan başka türe dönüşünce `holes` alanını düşürür. Ötele yalnızca dış halkayı öteler.
  - `polyline` / `polygon`: köşeler (`pts`) ve isteğe bağlı `bulges`. `bulges[i] = tan(θ/4)`, `pts[i] → pts[i+1]` kenarının (kapalı alanda son eleman kapanış kenarının) yay açısıdır; pozitif saat yönünün tersidir, 0 düz kenardır. DXF LWPOLYLINE ile birebir aynıdır. Yay yoksa alan hiç yazılmaz. **Geometriyi `doc.update` ile değiştirirken yeni şekilde yay yoksa `bulges: undefined` açıkça verilir**, yoksa eski yaylar birleştirmede kalır. Yaylı bir şeklin halkası `polygonRing(e)`, çevresi `entityOutline(e)` ile alınır; `e.pts` doğrudan dolgu ya da içerik testi için kullanılmaz.
  - `hatch`: sınır halkası (`ring`), isteğe bağlı adalar (`holes`, taranmadan kalan halkalar) ve desen (`solid | lines | cross`, açı, aralık). Adalı bir alanın içine tarama yapılınca alanın delikleri taramanın adaları olur. Sınır şekline bağlı (ilişkisel) değildir; desen dünya ızgarasına hizalı olduğu için komşu taramalar ortak sınırda örtüşür.
  - `text`: seçim ve sınır kutusu, döndürülmüş yaklaşık metin kutusudur (`textBox`, harf başına ~0,55 em). Yay (`arc`) merkez, yarıçap ve radyan cinsinden `a0 → a1` açılarıyla, **her zaman saat yönünün tersine** tutulur; aynalama gibi yönü çeviren işlemler başlangıç ve bitişi değiştirerek bu kuralı korur. Ortak alanlar `layerId`, `color?` (yoksa katmana göre), `attrs: Record<string,string>` ve `label?` (çizimde gösterilen kısa metin: parsel no, nokta adı).
- **`LayerStore`:** ağaç yapısı (grup ya da katman).
  - Görünürlük ve kilit üst düğümden devralınır (`isVisible`, `isLocked`).
  - `events.structure` ağaç şekli değişince tetiklenir.
  - `events.state` görünürlük, kilit ya da stil değişince etkilenen yaprak kimlikleriyle tetiklenir.
  - `events.expanded` bir grup açılınca ya da kapanınca tetiklenir (görünüm durumu; düzenleme sayılmaz).
  - `version` sinyali ucuz liste aboneliği içindir.
- **Katman stili değişikliği geri alınabilir:** arayüz `doc.setLayerStyle(id, patch, etiket)` kullanır (Katmanlar paneli renk/tip/kalınlık, katman stili penceresi); geri alma stili olduğu gibi geri koyar (`LayerStore.replaceStyle`). Görünürlük, kilit ve açık/kapalı durumu geçmişe girmez.
- **`LayerStyle`:** çizim motoru ve üst katman katman adını **bilmez**. Her görsel davranış stil alanıdır:
  - `color`: hex ya da tema jetonu: `fg` / `fg-dim` (ana ve ikincil mürekkep) ya da `ink` (CAD renk 7, "Siyah": açık zeminde siyah, koyu zeminde beyaz). Jetonlar `render/color.ts` içindeki `resolveColor` ile çözülür; arayüzdeki renk örnekleri `colorSwatch` kullanır, jetonu doğrudan CSS'e yazmaz. Taslak katmanı `ink` ile başlar.
  - `lineType`, `lineWeight` (mm), `fill`
  - `point { symbol, size }`
  - `label` (`LabelStyle`: yerleşim, boyut, şablon, görünür ölçek aralığı)
  - `pickInterior` (çokgenin içine tıklayınca seçilsin mi)

  Yeni bir görsel özel durum gerekiyorsa `LayerStyle`'a alan ekleyin; `if (layerId === '…')` yazmayın.
- **`ProjectSettings`:** SRID ve CRS tanımı, uzunluk ve alan hassasiyeti, alan birimi, açı birimi, çizim ölçeği. `toJSON()` ve `assign()` dosya biçimine hazırdır.

### 4.8.1 Geometri çekirdeği (`model/geom/`, `model/ops/`)

CAD doğruluğunun kaynağıdır. **Saf fonksiyonlardan oluşur, DOM ve belge bilmez, her fonksiyonun birim testi vardır.**

**Hesap Rust çekirdeğindedir** (`crates/shared/geometry-core`, ADR 0008). `model/ops`'un bütün dosyaları ve üst düzey `model/geom` modülleri (bindirme ve alan cebiri, paralel, ölçmecilik, şekiller, teğet daire, öteleme, tarama, ölçü) ince cephelerdir: TS dosyası adı, tipi ve belge yorumunu tutar, işlevi `op('ad')` ile çekirdeğe gider. Nesne döndüren işlemler `ops/entityOp.ts` ile sarılır (çekirdeğin yazmadığı `bulges`/`holes`, `doc.update` birleştirmesinde eski yayları bırakmasın diye `undefined` olarak eklenir). İlkel modüller (`affine`, `arc`, `bulge`, `intersect`, `ellipse`, `spline`), `model/geometry.ts`'in ölçüleri ve `model/entities.ts`'in nesne işlevleri de cephedir (S3b). İmleç hareketi başına çağrılan küçük ölçüler JSON'dan değil sayı alan girişlerden (`dist`, `angleDeg`, `bearingGrad`, `distToSegment`) ve çekirdeğin belleğindeki kazıma tamponundan (`signedArea`, `pathLength`, `centroid`, `pointInPolygon`) geçer (`wasm/core.ts`). TS'te kalanlar hesap değil kayıttır: tipler, kutu büyütme (`emptyBounds`, `extendBounds`), `bulgeAt`, `entityGeometry`, sabitler ve etiket tabloları. Çok nesneye bakan yerler (tümünü göster, pano, kutupsal dizinin seçim ortası) geometri deposuna sorar (`view.extent`); taşı, kopyala, döndür, ölçekle, aynala, diziler ve yapıştır dönüşümü de depoda yapar, yalnız yeni geometri paketli döner (`view.transformEntities`, §4.9). Cephelerde aritmetik ve `Math.` yoktur: `model/singleSource.test.ts` bu klasörlerin her dosyasını ve depo okuyucularını TypeScript'in sözdizimi ağacıyla denetler (sayma ve dizinleme serbest, istisna gerekçesiyle yazılır); yeni hesap `crates/shared/geometry-core`'a yazılır (§14). Aşağıdaki tablo işlemlerin ne yaptığını anlatır; Rust'taki karşılıkları aynı adlı dosyalardadır.

| Dosya                                                                              | İçerik                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                    |
| ---------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `geom/affine.ts`                                                                   | `Affine` (`[a,b,c,d,e,f]`), öteleme, döndürme, ölçekleme, aynalama, birleştirme, `isReflection`                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                           |
| `geom/arc.ts`                                                                      | Açı normalleştirme, süpürme açısı, üç noktadan çember ve yay, yay parçalama (tessellation)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                |
| `geom/bulge.ts`                                                                    | Çoklu çizgi yay parçaları: bulge → merkez/yarıçap/işaretli açı, üç noktadan ve teğetten bulge, kenar ortası ve teğeti, kenarlar, çevre, alan (shoelace + daire parçaları), ters çevirme, sıfır uzunluklu kenar temizliği                                                                                                                                                                                                                                                                                                                                                                                                                                                                                  |
| `geom/intersect.ts`                                                                | **`Edge`** (doğru parçası ya da yay/daire) ve kesişimler: parça-parça, parça-yay, yay-yay, ışın-kenar; en yakın nokta, dik ayak. Yay kenarının `sweep`'i **işaretlidir** (negatif = saat yönü), böylece çoklu çizgi yaylarında yol yönü korunur; yay üzerinde olma testi `onEdgeArc` ile yapılır                                                                                                                                                                                                                                                                                                                                                                                                          |
| `geom/offset.ts`                                                                   | Gönyeli (miter) yol öteleme, keskin köşede pah; yaylı yolda yaylar merkezleri etrafında büyür/küçülür, komşular taşıyıcı doğru/çember kesişiminde birleşir; noktanın hangi tarafta olduğu                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                 |
| `geom/shapes.ts`                                                                   | Dikdörtgen (kenardan, döndürülmüş köşelerden, boyuttan), düzgün çokgen (içten, dıştan, kenardan), AutoCAD yay yöntemleri (başlangıç-merkez-bitiş/açı/kiriş, başlangıç-bitiş-açı/yön/yarıçap/merkez), revizyon bulutu (`cloudOf`: kenarlar yay boyunda kirişlere bölünür, hepsi dışa bombeli)                                                                                                                                                                                                                                                                                                                                                                                                              |
| `geom/ellipse.ts`                                                                  | Elips ve eliptik yay: nokta, türev, parametre (afin dönüşümle birim çembere), yay uzunluğu (Simpson), alan, en yakın parametre (Newton), doğru kesişimi (tam), teğet noktaları (tam), eksenden/merkezden kurulum                                                                                                                                                                                                                                                                                                                                                                                                                                                                                          |
| `geom/parallel.ts`                                                                 | Paralel çizgi: eksenin sol ve sağ mesafedeki gönyeli yanları (`parallelSides`), aradaki koridor alanı (`corridorArea`; kapalı eksende delikli halka)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                      |
| `geom/survey.ts`                                                                   | Ölçmecilik yapıları: yan nokta (dik ayak/dik boy, sağa artı), kenar kesişimi, doğru kesişimi, hat üzerinde nokta, açı-mesafe (saat yönünde)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                               |
| `geom/tangentCircle.ts`                                                            | İki nesneye teğet, verilen yarıçaplı daire (TTY): paralel doğru ve çemberlerin kesişimleri; üç nesneye teğet daire (TTT, Apollonius): her kenar için bir teğetlik denklemi (doğru ±r, çember R+r, R−r, r−R), her yön birleşimi çözülür (üç doğru tam, çember içerende Newton; seçilen noktaların ortasına taşınmış koordinatlarda). İkisinde de teğet noktaları tıklanan yerlere en yakın çözüm alınır                                                                                                                                                                                                                                                                                                    |
| `geom/overlay.ts` (Rust: `geom/arrangement.rs`, `geom/overlay.rs`)                 | **Düzlem bindirme motoru:** kenarlar (doğru parçası ve yay) kesiştikleri, dokundukları ve örtüştükleri yerde kesilir; üst üste binen parçalar (komşu parsellerin ortak sınırı) tek parça olur ve hangi kaynağın hangi yönde geçtiği sayılır; bir kural iki yandaki sarım sayılarından parçanın sonuç sınırı olup olmadığına karar verir; kalan parçalar sonuç hep solda kalacak biçimde halkalara bağlanır, tek noktada değen halkalar ayrılır, delikler en küçük dış halkaya verilir. Yaylar yay kalır; girdi köşeleri koordinatlarını bit bit korur (`Source.points`); köşe birleştirme toleransı `TOL = 1e-6` m; kesişim noktasında doğrusal devam eden ve girdi köşesi olmayan noktalar birleştirilir |
| `geom/region.ts`                                                                   | Alan cebiri: `unionAreas`, `intersectAreas`, `subtractAreas`, `splitArea` (kesme çizgisi alanı baştan başa geçmeli), `faceIndex` / `faceAt` / `allFaces` (çizgilerin kapattığı yüzler, içteki gruplar delik), `insideArea`, `netArea`                                                                                                                                                                                                                                                                                                                                                                                                                                                                     |
| `geom/spline.ts`                                                                   | Merkezcil Catmull-Rom (Barry–Goldman), açık ve kapalı                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                     |
| `geom/hatch.ts`                                                                    | Tarama çizgilerini halkaya ve adalarına kırpma (tek-çift kuralı, yarı açık tepe kuralı, dünya ızgarasına hizalı, en çok 20 000 çizgi)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                     |
| `geom/dimension.ts`                                                                | Ölçü yerleşimi (hizalı, doğrusal ΔY/ΔX, açı, yarıçap, çap): uzatma çizgileri, eğik uçlar, her zaman okunur yazı, değer ve birimi, seçme kenarları, tutamaç yeri; `dimensionOffsetAt` (bir noktadan geçen ötelenme), `linearAngleFor` (yatay mı düşey mi, AutoCAD gibi yerleşimden), `sectorArms` (iki doğrunun, yayın konduğu bölgedeki açısı)                                                                                                                                                                                                                                                                                                                                                            |
| `ops/edgeLabels.ts`                                                                | Kenar ölçüsü yazılarının yeri: kenar ortası, halkanın dışı, okunur açı                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                    |
| `ops/edges.ts`                                                                     | Nesne → `Edge[]`. **Yeni nesne türü yalnızca kenarlarını vererek** kesişim, budama, uzatma ve kenetlemeye katılır.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                        |
| `ops/transform.ts`                                                                 | Her nesne türüne afin dönüşüm. Yazı aynalanınca okunur kalır (MIRRTEXT = 0). Araçlar sonucu geometri deposunun paketli yanıtından okur (`transformedFrom`: yeni geometri depodan, öbür alanlar nesnenin kendisinden); JSON'lu toplu çağrı (`transformEntities`) bu yolun karşılaştırıldığı başvurudur.                                                                                                                                                                                                                                                                                                                                                                                                                     |
| `ops/curveCuts.ts`                                                                 | Yol olmayan eğriler için budama, kırma, uzatma, öteleme: elips (parametre uzayında, kesimler doğruda tam, yayda alternatif izdüşümle) ve yardımcı çizgiler (parçalar ışın ya da çizgi; kesimler taban noktasından çözülür)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                |
| `ops/path.ts`                                                                      | Nesneyi uzunluk parametreli yol (`s ∈ [0, L]`) olarak görür: noktası, teğeti, en yakın `s`, kesimler, alt yol (yay parçaları tam kesilir), eşit bölme ve aralık parametreleri. Buda, kır ve böl bunu kullanır.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                            |
| `ops/trim.ts`                                                                      | Hızlı budama ve uzatma. Kapalı şekiller açılır, daire yaya dönüşür; yayla biten çoklu çizgi kendi çemberi boyunca uzar.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                   |
| `ops/break.ts`, `ops/join.ts`, `ops/explode.ts`, `ops/stretch.ts`, `ops/vertex.ts` | Kır (iki nokta arası ya da tek noktadan; kapalıda saat yönünün tersine), birleştir (uç toleranslı zincir; kapanırsa alan), patlat (çizgi/yay, eğri → çoklu çizgi, ölçü → çizgi + yazı), esnet (penceredeki köşeler), köşe ekle/sil (yay kenarı aynı çember üzerinde ikiye bölünür)                                                                                                                                                                                                                                                                                                                                                                                                                        |
| `ops/areas.ts`                                                                     | Nesne ↔ alan: `areaOfEntity` (alan ve daire tam; tam elips ve kapalı eğri 1 mm içinde çokgen; ilk ve son noktası aynı çoklu çizgi), `polygonOfArea`, `polylinesOfPolygon`, `lineSource`                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                   |
| `ops/lengthen.ts`                                                                  | Uzat-kısalt: yeni toplam boy bir uçtan; kısaltma yolu keser (yay tam), uzatma son parçayı sürdürür (düz parça doğrultusunda, yay kendi çemberinde, tam turu geçemez); `lengthToward` imleçten boy                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                         |
| `ops/offset.ts`, `ops/fillet.ts`, `ops/grips.ts`                                   | Nesne öteleme; iki çizgi için köşe yuvarlama ve pah, çoklu çizgi köşesinde yuvarlama (yay parçası) ve pah (`cornerOfPath`); tutamaç anlamları                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                             |

- **İşlemler geometri döndürür, belgeyi değiştirmez.** Sonuç `EntityGeometry` ya da `{ error }` olur. Kaydı araç yapar (`doc.transact`); böylece her değişiklik tek adımda geri alınır.
- **Hata dili kullanıcıya yöneliktir** (`{ error: 'Yarıçap bu çizgiler için çok büyük.' }`). Araç bunu doğrudan `log.warn` ile gösterir.
- **Toleranslar:** kesişimde parametre toleransı `1e-9`, kesim noktası birleştirmede `1e-7 × L`. Ekran toleransı araçtan `view.worldTolerance(px)` ile gelir; geometri çekirdeğinde piksel yoktur.
- **Sayısal kararlılık:** yardımcı çizgiler CPU'da `CONSTRUCTION_REACH` (1000 km) yarı uzunluğunda kenar olarak hesaba girer. Bu uzunlukta ikinci derece denklemin diskriminantı basamak kaybeder; bu yüzden `lineCircleParams` merkezden doğruya dikme ayağından çözer, yardımcı çizgi kesimleri uzak uçlardan değil taban noktasından hesaplanır ve doğruyla kesişen eğrilerde nokta doğrunun üzerinden alınır. Yeni bir kesişim yazarken aynı kural geçerlidir: büyük sayıların farkını almayın.

### 4.9 Çizim hattı (`render/`, `viewport/`)

```
CadDocument ──(changed/state olayları)──► ViewportController.dirtyLayers
                                               │ rAF
                                               ▼
                          styledLayer.buildStyledLayer(layer) → SceneLayer (stil motoru toplulukları, çekirdekte tek çağrı)
                          sceneBuilder.buildSceneLayer(vurgu)  → SceneLayer (ince çizgi, dolgu, nokta)
                                               ▼
                          RenderBackend.upload(layer) / render(FrameState)
```

- **`RenderBackend` sözleşmesi** (`render/types.ts`): `init`, `resize`, `upload(SceneLayer)`, `remove(id)`, `render(FrameState)`, `dispose`.
  - WebGL2 (varsayılan) ve WebGPU tam olarak uygulanmıştır ve aynı çizimi üretir (bkz. §9.5).
  - Motor çalışırken değiştirilebilir: yeni arka uç kendi tuvalini alır, bütün katmanlar belgeden yeniden yüklenir ve eski tuval ancak ilk kare çizildikten sonra kaldırılır; boş kare görünmez.
  - Arka uca yalnızca `SceneLayer` ve `FrameState` gider; varlık, katman ağacı ya da DOM gitmez.
- **Belge katmanları stil motorundan geçer** (`render/styledLayer.ts`, ayrıntı [docs/STYLE.md](docs/STYLE.md) §6): her nesne kendi sembolüyle (`entity.symbol`), yoksa katmanın işleyicisiyle (`style.renderer`), yoksa katmanın basit görünüşüyle (renk, çizgi tipi, kalınlık, dolgu, nokta simgesi) çizilir. Semboller derlenip `SceneLayer.styled` topluluklarına dönüşür: kalın ve kesikli vuruşlar (örneklenmiş parçalar), dünya ızgarasına hizalı taramalar ve döşemeler, SDF ya da atlas görüntüsü işaretler.
  - **Katman çekirdekte tek çağrıda kurulur** (`crates/shared/style-core/src/style`, ADR 0008 “Stil derleyicisi”): sayfa katmanın programını (kullanılan kitaplık sembolleri, işleyici, renk başına basit görünüş ve taramaların kümeleri, renkler, döşenen görüntülerin boyları; `CoreStyleProgram`), nesne başına dört sayıyı (kip, küme ya da sembol, basit görünüş, renk) ve ifadelerin okuduğu tabloyu verir. Geometri deposu (`PickIndex.styled` → `CoreStore.buildStyled`) nesnelerin çizilecek geometrisini kendisi okur (eğriler parçalanmış, halkalar yönlendirilmiş); işleyiciyi çözer, sembolleri derler, toplulukları kurar. `$alan`, `$uzunluk`, `$y`, `$x` bir ifade isterse katman için bir kez hesaplanır. Yanıt toplulukların tanımı (JSON) ve bütün sayılar tek `Float32Array`'dir.
  - **Sayfada gösterim kalır** (`render/styledBatches.ts`): tema paletinden renkler, atlas görüntüleri (yazı, SVG, raster, desen döşemeleri) ve bir topluluğun geometrisinin ötesine taşma payı. SVG, yazı, raster ve desen görüntüleri iki arka ucun paylaştığı doku atlasındadır (`render/atlas.ts`, `backend.useAtlas`). Önizleme ve lejant tek sembolü aynı çekirdekle derler (`style/compile.ts` `compileSymbol`).
  - Çizim ölçeği ya da stil kitaplığı değişince bütün katmanlar, yalnız öznitelik değişince işleyicisi olan katmanlar yeniden kurulur. Vurgu katmanlarının geometrisi de depodan gelir (`PickIndex.drawn`, `style/geometry.ts` `DrawnReader`; nesnenin kendi noktaları kopyalanmaz).
- **`SceneLayer`:** katman başına çizgi, dolgu ve nokta topluları (vurgu ve ızgara bunları kullanır) ile stilli topluluklar (`styled`). Renk ve kesikli çizgi deseni topluya aittir.
  - Çizgi: segment listesi ve kümülatif mesafe. Kesik desen parça gölgelendiricide piksel cinsinden hesaplanır.
  - Dolgu: katman bitince bütün dolgular Rust çekirdeğinde tek çağrıda kulak kırpmayla (ear clipping) üçgenlenir (`render/fillQueue.ts` → `triangulateMany`).
  - Nokta: gölgelendiricide çizilen simgeler (halka, artı, üçgen).
- **Yerel orijin (RTC):** Dünya koordinatları CPU'da float64 ve mutlaktır. GPU'ya yalnızca `doc.origin`'e göre farklar float32 olarak gider. TM koordinatları 4,4 milyon metreye ulaşır; mutlak float32 santimetre titremesine yol açar. **GPU'ya asla mutlak koordinat yüklemeyin.**
- **Çizim sırası:** alt katmanlar (ızgara), ağaçtaki yaprak sırasının tersi (listede üstteki en son, yani en üstte çizilir), üst katmanlar (`__hover`, `__sel`). Her geçişte katman katman önce düz dolgular ve stilli topluluklar (sembol düzeyine göre), sonra bütün katmanların ince çizgileri, sonra noktalar çizilir.
- **Yardımcı çizgiler** (`xline`, `ray`) GPU'ya görünüm alanının üç katı büyüklüğündeki bir kutuya kırpılarak gider (`BuildOptions.clip`); görünüm kutudan çıkınca ya da ölçek iki kattan fazla değişince bu çizgileri içeren katmanlar ve vurgular yeniden kurulur. Böylece GPU'ya hiçbir zaman uzak (float32'de titreyen) koordinat gitmez.
- **Vurgu ayrı katmandır.** Seçim değişince yalnızca `__sel` ve `__hover` yeniden kurulur, belge katmanlarına dokunulmaz.
- **`ViewportController`:**
  - `requestRender()` GPU'yu ve üst katmanı, `requestOverlay()` yalnızca 2B üst katmanı çizdirir. İkisi de `requestAnimationFrame` içinde birleştirilir.
  - `stats` son karenin adım sürelerini tutar. `probe` (`ViewportProbe`) yalnız geliştirme derlemesindedir: etkileşim ölçümü (`apps/web/scripts/perf/interaction.mjs`) `kentos.view.probe`'a boş bir kayıt koyar; her `pointermove` kenet, araç ve toplam süresini, her kare başlangıç anını, `stats` adımlarını ve üst katmandaki etiket ve araç önizlemesi sürelerini ekler. Bütün kullanımlar `import.meta.env.DEV` arkasındadır ve alan `declare` ile tanımlıdır: üretim derlemesi bayt bayt aynı kalır.
  - Olay işleyicisinde asla eşzamanlı çizim yapmayın.
  - **Tek istisna boyut değişimidir:** canvas'ın `width`/`height` değeri değişince tampon temizlenir ve WebGL bağlamı `alpha: false` olduğu için siyah görünür. Çizim bir sonraki kareye bırakılırsa tarayıcı arada bu siyah tamponu gösterir; panel ayırıcısı sürüklenirken ekran yanıp söner. Bu yüzden `resize()` (ResizeObserver içinde, düzenden sonra ve boyamadan önce çalışır) boyut gerçekten değiştiyse hemen `frame()` çağırır. Duman testi sürükleme sırasında ekran akışını kare kare inceleyerek bunu denetler.
- **Üst katman** (`viewport/overlay.ts`, Canvas2D) şunları çizer: etiketler (`LabelStyle` ile; hangilerinin nerede çizileceğini geometri deposu söyler), tutamaçlar, kenet işareti, artı imleç, ölçek çubuğu, "K" kuzey oku ve araç önizlemeleri. GPU metni (SDF) gelene kadar yazılar buradadır.
- **`PickIndex`** (`viewport/picking.ts`): Rust geometri deposunun (`geometry-core::store`, ADR 0008 “Geometri deposu”) ince yüzüdür. Depo nesnelerin kopyasını, katman tablosunu ve Hilbert sıralı bir R-ağacını tutar, kuralları belge sırasıyla birebir uygular:
  - Seçme önceliği: nokta ve kenar, sonra imleci içeren en küçük çokgen (bina, parsel, ada sırasıyla).
  - Kenetleme türleri: uç, orta, merkez, nokta, çeyrek, kesişim, dik, en yakın. Tercihlerden süzülür (`prefs.snap*`).
  - Kenet önceliği: eşit uzaklıkta uç ve nokta, kesişimden; kesişim, merkezden; merkez, çeyrekten; çeyrek, ortadan; orta, dikten önce gelir. "En yakın" yalnızca başka aday yoksa kullanılır.
  - **Kenet her `pointerdown` ve `pointerup`'ta yeniden hesaplanır.** Fare hareketi olmadan gelen tıklama (kalem, dokunma, hızlı tıklama) eski kenet noktasına yapışmamalıdır.
  - `hitEdge` (yalnızca kenar seçimi) ve `edgesIn` (sınır kenarları) değiştirme araçları içindir. Budama ve uzatma sınır olarak görünür alandaki tüm kenarları kullanır.
  - Pencere seçimi (soldan sağa, tamamen içeride) ve kesişim seçimi (sağdan sola, temas).
  - Eşitleme: `touched` ile silmeler hemen, eklenen ve değişenler bir sonraki sorguda paketli (`wasm/pack.ts`: noktalar `Float64Array`) gider; `load` ve `replaceWith` `reset` yayar ve her şey yeniden gönderilir.
  - Üst katmanın kararları da depodandır: `labels` (hangi yazı, ölçü ve etiket nerede; `viewport/storeRecords.ts`), `grips` (seçili nesnelerin tutamaçları).
  - Araçların önizlemeleri ve toplamlar da: `trim`, `extend` (buda ve uzat), `ghosts` (taşı, kopyala, döndür, ölçekle, aynala, dizi ve yapıştırmanın hayalet yolları; `tools/preview.ts` → `strokePaths`), `stretchGhosts`, `measure` (özellikler panelinin toplamları). Yapıştırmanın nesneleri belgede olmadığı için `PasteTool` kendi deposunu kurar (hayaletler de yapıştırılan geometri de ondan; özgün koordinatlara yapıştırma çağrı için bir depo kurar).
  - Taşı, kopyala, döndür, ölçekle, aynala, diziler ve yapıştırmanın kendisi de depodadır (`transformEntities`): depo kendi kopyalarını dönüştürür ve yalnız yeni geometriyi `wasm/pack.ts` düzeninde, sayı olarak döndürür (`CoreStore.transformPacked`, `unpackEntities`); nesneler JSON'dan geçmez. Öbür alanlar (kimlik, katman, renk, öznitelikler, etiket, sembol) nesnenin kendisinden gelir, öznitelikler kopyalanır (`model/ops/transform.ts` `transformedFrom`); sonuç JSON'lu çağrınınkiyle alan alan ve bit bit aynıdır, yalnız −0 burada da korunur.

### 4.10 Arayüz (`ui/`)

- **`Component`:** tek kök elemana ve bir `DisposableStore`'a sahiptir. `dispose()` her aboneliği ve dinleyiciyi bırakır. Her `subscribe` ve `listen` çağrısının dönüşü `this.d.add(...)` ile saklanır.
- **`ui/widgets/`:** genel ve bağımsız parçalar: `PopupMenu`, `Dropdown`, `TreeView`, `PropertyGrid`, `Dialog` (Tab pencerede döner), `confirm` (onay penceresi), `Splitter`, `tooltip`, `controls` (segmented, switch, stepper, textField, settingRow, note). Widget'lar `AppContext` bilmez. Tek istisna `CommandButton`'dır, çünkü komuta bağlı düğmedir.
- **Soru sormanın tek yolu onay penceresidir** (`ui/widgets/confirm.ts`, DESIGN.md §7.9.1): `confirmDialog` soran pencerenin üstünde açılır (`stack`), yanıtını söz olarak döndürür; Esc, × ve arka plan hiçbir şeyi değiştirmeyen yanıttır. Hazır biçimleri `askUnsaved` (“Kaydet ve kapat”, “Kaydetmeden kapat”, “Vazgeç”; Enter kaydeder) ve `askRemove` (odak Vazgeç'te, silen düğme `btn--danger`). Durum satırına, menüye ya da satır içine soru yazılmaz. Kendi kaydı olan düzenleyiciler (SVG, sembol ve model tasarımcısı) `Dialog.beforeClose`'ta yalnız kaydedilmemiş değişiklik varsa sorar: “değişti mi” kaydedilen ya da açılan hâle göre bakılır, hiç değiştirilmemiş yeni bir çizim, sembol ya da model sormadan kapanır.
- **Paneller** (`LayersPanel`, `PropertiesPanel`, `BottomPanel`) modelden okur, değişikliği komut ya da belge API'si ile yapar. Panel içi yeniden çizimler mikro görevde birleştirilir (`PropertiesPanel.schedule`). `LayersPanel` ağacı yalnız biçimi değişince kurar (`events.structure`, `events.expanded`; bir görevdeki değişiklikler mikro görevde tek kurulumda birleşir); düzenleme, geri alma ve yinelemede nesne sayılarını, katman durumu, etkin katman ya da tema değişince göz, kilit, renk ve işaretleri satırlara yerinde yazar: satırlar aynı öğe kalır (üzerine gelme, odak ve açık yeniden adlandırma alanı bozulmaz). Satırdaki göz, kilit ve renk düğmeleri tıklanınca klavye odağını almaz.
- **`TreeView`** (katmanlar, işlemler, stil yöneticisi) yalnız kaydırma penceresindeki satırları ve iki yanında 40'ar satır daha kurar: görünen satırlar veri olarak tutulur, iki boşluk öğesi gerisini temsil eder, kaydırma gelen satırları kurar ve gidenleri bırakır. Ağaç kendi kaydırma kutusudur; satır boyu gizli bir örnek satırdan (`--row-h`) ölçülür, yazı ölçeği değişince yeniden. `focus` ve `rowOf` satırı önce görünüme getirir (klavye, F2 ile yeniden adlandırma); satırlar `aria-setsize`/`aria-posinset` taşır. Satırın kendi abonelikleri (ipucu gibi) `renderRow`'dan döner ve satır gidince bırakılır; `releaseRow` panelin satır kaydını temizler (Katmanlar paneli yerinde yazmayı yalnız kurulu satırlara yapar, kaydırılıp gelen satır o anki sayı ve durumla kurulur).
- **`AppShell`** yerleşimi kurar ve bölgeleri doldurur. Bileşenler birbirini tanımaz. Üst bölge (`shell__chrome`) `prefs.shell`'e göre menü çubuğu ve araç çubuğu ya da şerittir; ayar değişince sayfa yenilenmeden değişir.
- **Şerit** (`ui/ribbon/`: `Ribbon.ts` sekmeler, sığdırma, daraltma; `panels.ts` panel düzeyleri ve alanlı paneller; `controls.ts` düğmeler; `search.ts` Komut ara): ayrı JS ve CSS parçasıdır (§20), yalnız seçilince yüklenir; yüklenirken yerini aynı yükseklikte bir yer tutucu tutar, yüklenemezse klasik arayüze dönülür ve söylenir. Sekmeler ilk açılışta kurulur ve saklanır. Pencere daralınca paneller sağdan sola, her biri bir adım inerek küçülür (büyük → küçük etiketli → yalnız simge → tek düğme; genişlik kazandırmayan adım atlanır, Giriş'te Çizim ve Değiştir en son). Seçim varken bağlamsal **Seçim** sekmesi (sayı, türler, seçime uygulanan komutlar) çıkar; çalışan aracı içeren sekmeler amber nokta taşır; İşlemler sekmesi işlem kaydını ve modelleri izler. `Ctrl+F1` ya da sekmeye çift tık şeridi sekmelere daraltır; daraltılmışken sekme çizimin üstünde açılır, bir komut çalışınca kapanır. `Alt+Q` Komut ara (klasik arayüzde komut satırı). Sağ tık düğmeyi hızlı erişime ekler. Şerit düğmesine tıklamak klavye odağını almaz (Enter son komutu yinelemeye devam eder). Şeritle birlikte araç kutusu kapalıdır (F9 ile açılır, ayrı hatırlanır). Geçerli özellik alanları (katman, renk, tip, kalınlık, ölçek) araç çubuğuyla ortaktır (`ui/toolbar/fields.ts`).
- **Ayar pencereleri** `ui/settings/`: `SettingsShell` (iskelet, taslak ve Kaydet/Vazgeç), `crsPicker` (ortak EPSG seçici), `ProjectSettingsDialog`, `AppSettingsDialog`.
- **Sağ dok** üst yuvada "Katmanlar | İşlemler" sekmelerini (`ui.dockTab`), altta öznitelikleri taşır. Aynı yuvayı paylaşan paneller başlıkta sekme şeridi gösterir (`Panel.setTabs`).

### 4.11 İşlem araçları (`processing/`)

Toplu işlemler (QGIS Processing gibi) için ayrı bir çatıdır; ayrıntılar [docs/PROCESSING.md](docs/PROCESSING.md)'dedir. Kısaca:

- **Araç bildirimseldir** (`defineTool`): kimlik, etiket, kategori, açıklama, yardım, takma adlar, parametreler (tür, zorunluluk, varsayılan, sınır, görünürlük koşulu, gelişmiş), çıktılar ve çalışabileceği yerler (`client | worker | server | postgis`). Pencere, araç kutusu satırı, menü, komut (`processing.run.<id>`) ve geçmiş bu tanımdan üretilir.
- **Araç belgeyi değiştirmez:** `run` çözülmüş girdiler ve salt okunur belge alır, `ChangeSet` döndürür. `ProcessingRunner` doğrular, sayfaya bağlı olanı kopyalanabilir bir işe (`RunJob`: nesne kimlikleri, hedef katman) çözer, `Executor`'ı seçer ve sonucu **tek geri alma adımı** olarak uygular; kilitli katmanları atlar, yeni hedef katmanı yalnızca yazılırsa kurar.
- **Çalışma yeri:** sayfa (`clientExecutor`) ya da Web Worker (`processing/worker/`; belge nesnelerin kopyasıyla gider, Durdur worker'ı sonlandırır). Pencerede araç başına seçilir; Otomatik, 2 000 nesne ve üstünü worker'a gönderir. `run` bu yüzden DOM'a ve modül durumuna dokunmaz, sonucu yapılandırılmış kopyayla taşınabilir olmalıdır.
- **Geometri çekirdekten** (ADR 0008 S4): iki çalışma yeri de aynı `runJob`'ı çağırır (`processing/job.ts`); çalıştırma okuduğu nesneleri kendi geometri deposuna paketler (`processing/geometry.ts` `ObjectStore`) ve araç `ctx.geometry` ile kimlikten sorar: köşe numaralama (halka sırası, ortak köşeler, dışa bakan yön; adlar TS'te `numbering.ts`), köşe yazısının yeri, kenar ölçüsü yazıları ve ortak kenar testi, ifadelerin `$alan`, `$uzunluk`, `$y`, `$x` değerleri (bütün nesneler için bir kez). “Görünen” kapsamının kutu testi ve penceredeki ifade önizlemesi görünümün deposundan gelir (`FeatureHost.geometry`). Yeni bir aracın geometrisi `crates/shared/geometry-core`'a yazılır; TS'te metin, sayaç ve akış kalır.
- **Parametre değer tipleri tanımdan çıkar.** Tanım içindeki ok fonksiyonlarının argümanı tiplenir (`(v: Shown)`, `(c: DefaultsContext)`), yoksa çıkarım bozulur.
- **Nesne kapsamları:** seçili, görünen, tümü (görünür katmanlar), katman (grup dahil) ve modellerde önceki adımın çıktısı (`ids`). Kullanıcı bir çalıştırmada nesne türlerini daraltabilir (`kinds`: yalnızca kapalı alanlar gibi). Zorunlu girdi boş kalırsa araç çalışmaz, alanda yönlendirme yazar.
- **İfadeler** (`model/expression/`; stil motoru da kullanır): koşul ve değer parametreleri için güvenli, `eval`'siz bir dil: alanlar (`Nitelik`, `[Tapu alanı]`), geometri değişkenleri (`$alan`, `$uzunluk`, `$katman` …), Türkçe ve İngilizce işlev adları (`yuvarla`/`round`), `ve`/`veya`/`değil`. Öznitelik metni sayı gibi okunur, boş değer kuralları sabittir; hata mesajı karakter yerini söyler. Seçim üreten araçlar belgeyi değiştirmez, `select` döndürür.
  - Dil Rust'tadır (`crates/shared/style-core`, ADR 0008 “İfade dili”): sunucu aynı ifadeyi aynı biçimde hesaplayacak. TS derlemenin sonucunu (`exprCompile`: okunan alanlar ve değişkenler, hata ve konumu) alır ve bir ifadeyi bütün nesneler için tek çağrıda değerlendirir (`evaluateAll`: okunanların tablosu gider, değerler sütun olarak döner; `as` ile sayı, metin ya da doğru/yanlış). Nesne başına çağrı yoktur.
  - İşlev ve değişken menüleri çekirdeğin kataloğundandır (`exprCatalog`). Metinler Türkçe sabit bir tabloyla sıralanır, tarayıcının ICU'suna bağlı değildir.
- **Modeller** (akış diyagramları; `processing/model.ts`, `modelRunner.ts`, `modelEdit.ts`): adım değerleri sabit, model girdisi ya da önceki adımın çıktısı olabilir (tür uyumu `canFeed`). Model tek geri alma adımıdır (`CadDocument.beginGroup`); bir adım çalışmazsa önceki adımlar geri alınır. Yerleşik modeller değiştirilemez (kopyası düzenlenir); kullanıcının modelleri `kentos.processing.v1`'de. **Model tasarımcısı** (`ui/processing/model/`): solda girdiler ve araçlar, ortada kutu-bağlantı diyagramı (porttan sürükleyip bağlama), sağda seçilenin ayarları; kendi geri alma yığını, kaydedilmemiş değişiklik uyarısı.
- **Arayüz:** İşlemler menüsü (kategoriler kayıttan üretilir), sağ dokta İşlemler sekmesi (arama, kategori ağacı, geçmiş), `ui/processing/ToolDialog.ts` penceresi.

### 4.12 Çalışma modları (`app/workspaces.ts`)

Bir proje, bir veri modeli, birkaç sunuş: **Hibrit** (CAD + CBS, her şey; eski dosyaların ve varsayılanın modu), **CAD** (teknik çizim: Harita, Koordinat ve İşlemler menüleri, parsel ve arazi araçları gizli; Harita sekmesinin kalanı “Ölçme”), **CBS** (coğrafi bilgi sistemi: yardımcı çizgi, şekil, elips, eğri, ölçü, tarama, dizi, köşe araçları gizli; Giriş'te Harita paneli). **3D Plan** ve **Afet Analizi** duyurulmuştur (`status: 'soon'`): “Yakında” yazar, seçilemez; böyle bir modu adlandıran dosya Hibrit gösterilir.

- **Mod bir proje ayarıdır** (`ProjectSettings.workspace`, sözleşmede `Workspace`, .kcad'de isteğe bağlı: yoksa Hibrit); Yeni proje penceresinde kartlarla sorulur (varsayılanı uygulama ayarı `prefs.defaultWorkspace`), Proje ayarları → Genel'de, durum çubuğundaki mod hücresinde ve Görünüm → Çalışma modu'nda (`workspace.<id>` radyo komutları) değişir. Değiştirmek projeyi kaydedilmemiş yapar.
- **Mod yalnız sunuştur:** veri, dosya ve hesap aynıdır; gizlenen her komut komut satırından, kısayoluyla ve Komut ara'dan yine çalışır. Mod menü çubuğunu (`visibleMenus`), menü bloklarını (`menuBlocks` süzgeci), şeridi (`RibbonInputs.filter`) ve araç kutusunu süzer; sağ dok CAD'de İşlemler sekmesini gizler. Mod değişince bunlar yerinde yeniden kurulur.
- **Yeni mod** bir kayıt girdisidir: kimlik (sözleşmedeki `Workspace` enum'una da), ad, alt başlık, simge, açıklama, üç madde, durum ve gizledikleri (`hide`: ana menüler; araç grupları, `grup/bölüm` ya da `tool.x`; komutlar). Kayıt sözleşmeyle aynı değilse modül yüklenirken hata verir; `app/workspaces.test.ts` adlandırılan her menünün, aracın ve komutun var olduğunu denetler.

---

## 5. Koordinat, birim ve hassasiyet kuralları

1. **Eksen adları terstir; dikkat.** İç temsilde `x` = doğu (Türk ölçmeciliğinde **Y, sağa değer**), `y` = kuzey (**X, yukarı değer**). Arayüzde her zaman "Y (sağa)" ve "X (yukarı)" yazılır, Y önce gelir. Kod içinde `x/y` kullanın; arayüz metninde `Y/X` kullanın.
2. **Her proje açık bir SRID taşır** (`doc.settings.crs`). Yeni projelerin varsayılanı `prefs.defaultSrid = 5256` (TUREF / TM36).
3. **CRS bilgisinin tek kaynağı `geo/crs.ts`'tir.** Projeksiyon parametreleri başka yerde yazılmaz. Yeni sistem gerekiyorsa kayda ekleyin.
4. **Atamak ile dönüştürmek farklıdır.** Proje SRID'sini değiştirmek yalnızca etiketi değiştirir, koordinatlar aynı kalır; arayüz bunu uyarıyla söyler. Dönüşüm (TUREF ↔ ED50, TM dilimleri arası) `geo/transform.ts` içinde açık, geri alınabilir bir işlem olacak. **Sessizce yeniden projeksiyon yapılmaz.**
5. **Açılar:**
   - Geometri içinde radyan ya da doğudan saat yönünün tersine derece kullanılır.
   - Ölçmecilik semti **kuzeyden saat yönünde grad** olarak `bearingGrad()` ile hesaplanır.
   - Gösterim `ctx.format.bearing()` ile yapılır; birim proje ayarından gelir.
6. **Etkileşim toleransları** `px / camera.scale` ile seçme/kenet adaylarını bulur; kaynak geometri doğruluğunu belirlemez. Mevcut kodda kullanılan sabit eşikler kadastral doğruluk kanıtı değildir. Hesap/topoloji/koordinat dönüşümü toleransları §23'e göre ayrı ve sürümlü tanımlanır.
7. **Alan ve uzunluk** kaynak geometri üzerinden CPU'da hesaplanır; GPU verisinden ya da ekrandan ölçülmez. Mevcut float64 yaklaşımı tek başına kadastral yeterlilik sağlamaz. Düz kenarlı halkalarda alan hesabı, eğrilerde analitik yöntem ve tüm nihai sonuçlar §23 doğrulamasına tabidir.
8. **Birimler:** metre, m², dönüm = 1 000 m², hektar = 10 000 m².
9. **Sayıdan metne tek geçit `ctx.format`'tır** (`coord`, `length`, `area`, `bearing`, `point`). `toFixed`'i arayüz metninde doğrudan kullanmayın.
10. **Ondalık ayırıcı her yerde noktadır** (`486512.340`). Komut satırı `Y,X` biçiminde virgülü koordinat ayırıcı olarak kullanır; görülen değer kopyalanıp aynen yazılabilmelidir.
11. **Öznitelik değerleri veridir, gösterim metni değildir.** Şimdilik metin olarak saklanıyor; tipli şema geldiğinde sayı ve tarih alanları gerçek tipte tutulacak.

---

## 6. Performans

### 6.1 Bütçeler (hedef)

| Senaryo                          | Hedef                                        |
| -------------------------------- | -------------------------------------------- |
| Kaydırma ve yakınlaştırma        | 1 milyon segmentte 60 fps (16 ms kare)       |
| İmleç hareketinde seçme ve kenet | 100 bin varlıkta her olayda < 2 ms           |
| Bir katmanı yeniden kurma        | 100 bin segmentte < 50 ms (gerekirse worker) |
| İlk açılış (örnek proje)         | < 1 s                                        |
| Açık panelde seçim değişikliği   | < 8 ms                                       |

Bugünkü uygulama örnek proje ölçeğinde (yüzlerce varlık) rahattır. Aşağıdaki
kurallar büyük veriye geçerken kodun yeniden yazılmasını önlemek içindir.

### 6.2 Kurallar

1. **Sıcak yollarda** (`pointermove`, kare döngüsü) bellek ayırmayı en aza indirin. Döngüde closure, JSON, regex ya da dizi kopyası üretmeyin.
2. **GPU tamponları yalnızca kirlenen katman için** yeniden kurulur. Seçim, üzerine gelme ve ızgara kendi katmanlarıdır.
3. **Her çizim `requestRender` ya da `requestOverlay` ile istenir.** Tek karede birleşir.
4. **Uzamsal sorgular `PickIndex` üzerinden yapılır.** Arayüz kodunda imleç hareketi başına `doc.all()` gezilmez.
5. **Uzun listeler sanallaştırılır.** 500 satırı aşabilecek her liste (katman ağacı, öznitelik tablosu, koordinat listesi, komut geçmişi) büyük veri desteğinden önce sanallaştırılmalıdır.
6. **Ağır işler worker'a gider:** dosya ayrıştırma (DXF, NCZ, SHP), büyük çokgen üçgenleme, eşyükselti ve TIN üretimi. Veri `ArrayBuffer` aktarımıyla taşınır; worker içinde DOM kullanılmaz.
7. **Büyük ölçekte geometri** `Float64Array` koordinat havuzlarında tutulur. Bugünkü `Vec2[]` binlerce varlıkta yeterlidir, milyonlarda değildir. Geçiş, `Entity` API'sini koruyarak yapılacak.
8. **Çizgi kalınlığı** için `gl.lineWidth` kullanılmaz (çoğu sürücüde 1 px). Kalın çizgiler örneklenmiş dörtgenlerle (instanced quads) çizilecek.
9. **Etiketler ölçek eşikleriyle ayıklanır** (`LabelStyle.minScale`, `minFeaturePx`). Görünmeyecek etiket için metin ölçülmez.
10. **DOM okuma ve yazma karışmaz.** Önce ölçün, sonra yazın; döngü içinde `getBoundingClientRect` ile stil yazmayı art arda yapmayın.
11. **Ölçmeden optimizasyon yapılmaz.** `performance.mark/measure` kullanın. İmleç başına seçme ve kenet, kaydırma, buda önizlemesi ve katman kurma süreleri `pnpm perf:interaction` ile ölçülür (§9.4, `docs/perf/`); bir değişikliğin bunları geriletmediği tabanla karşılaştırılarak gösterilir. Planlanan `?debug=perf` bayrağı kare süresi, yüklenen segment sayısı ve seçme süresini gösterecek.

### 6.3 Bilinen darboğazlar (büyük veri öncesi çözülecek)

- Üst katmanın etiket kararı geometri deposunun R-ağacından gelir (S1b); genel görünümde 81 bin nesnede kare başına ~10 ms (bulut) sürüyor → etiket önbelleği ya da ölçeğe göre seyreltme gerekiyor.
- Katman ağacının biçimi değişince (katman eklenir, adı değişir, grup açılır ya da kapanır) pencerenin satırları baştan kurulur, kimliğe göre yeniden kullanılmaz: sanallaştırmadan sonra bu ~50 satırdır (aşağıda); daha büyük ağaçlarda da sabit kalır, yeniden kullanmak kalan kazançtır. Uzun öteki listeler (öznitelik tablosu, koordinat listesi, komut geçmişi) henüz sanallaştırılmadı (§6.2 kural 5).

Çözülenler: `CadDocument` katman başına dizin tutar (`byLayer`, `countByLayer` bütün çizimi gezmez; 100 bin nesnede 40 katmanın kurulumu 71 ms → 0,3 ms); ızgara görünümün üç katı bir kutu için kurulur ve görünüm kutudan çıkmadıkça, aralığı, orijini ya da renkleri değişmedikçe yeniden kurulmaz (`render/grid.ts` `gridExtent`; kaydırma ve yakınlaşmada 380 karede 380 yükleme → 19); düzenlemenin geometriyi değiştirip değiştirmediği JSON metni kurulmadan, JSON'un yazacağı biçimde alan alan karşılaştırılır (`model/sameJson.ts`; 10 bin öznitelik düzenlemesi 77 ms → 13 ms). `LayersPanel` düzenlemede, geri almada ve yinelemede ağacı kurmaz: nesne sayıları ve katman durumu satırlara yerinde yazılır, satırlar aynı öğe kalır; ağaç yalnız biçimi değişince, görev başına bir kez kurulur (424 katman, ekranda 317 satır: düzenleme başına panel p50 20,6 → 0,1 ms; 300 katman ve 3 000 çizgili içe aktarma 3,4 s → 28 ms). `TreeView` sanallaştırıldı: 300 katmanlı bir grupta (317 satır) 317 yerine 52 satır kurulur; yeniden adlandırmada kurulum 23 → 7 ms ve ardından stil ve yerleşim 41 → 8 ms, grubu açıp kapamada 16 → 5 ms ve 34 → 8 ms (bulut, başsız Chrome).

---

## 7. GIS ve CAD doğruluk ilkeleri

- **Her düzenleme geri alınabilir.** Belgeyi değiştiren her yol `CadDocument` API'sinden geçer. Çok adımlı işlemler `transact` ile tek adım olur.
- **Kilitli katman** düzenlenmez, taşınmaz, silinmez. Araç bunu kullanıcıya söyler ve atlanan nesne sayısını raporlar.
- **Gizli katmana çizim** yapılabilir ama uyarı verilir.
- **Parsel numaralandırma:** Yeni parsel, hedef katmandaki en büyük `Parsel` özniteliğinin bir fazlasını alır. Ada ve mahalle kullanıcıdan istenir. Tapu alanı float64 koordinatlardan hesaplanıp özniteliğe yazılır.
- **Etiket ve öznitelik tutarlılığı:** `Parsel` ya da `Ada` özniteliği değişince, etiket aynı değeri gösteriyorsa etiket de güncellenir.
- **Topoloji (planlı):** Parseller ortak kenarları paylaşan bir düzlemsel graf üzerinde tutulacak. İfraz ve tevhid bu graf üzerinde çalışacak; alan toplamları ada alanıyla doğrulanacak. Bu karar veri modelinin temelidir; paylaşımsız çokgenlerle ifraz yazılmamalıdır.

---

## 8. Kod kuralları (sürdürülebilirlik)

- **Çevredeki kod gibi yazın:** aynı adlandırma, aynı yorum yoğunluğu, aynı deyimler.
- **Yorumlar "neden"i açıklar,** "ne"yi değil. Her dosyanın başında kısa bir amaç yorumu bulunur.
- **Tek dosya, tek kavram.** 400 satırı geçen dosya bölünmeyi düşündürmelidir.
- **Sabit renk yok.** Arayüz renkleri CSS jetonlarından (`var(--c-…)`), çizim alanı renkleri `readCanvasPalette()` üzerinden alınır. Katman renkleri veridir. Vurgu rengi kullanıcının seçimidir: amber ya da başka bir vurgu değeri yazılmaz, `--c-accent*` jetonları kullanılır (DESIGN.md §3.5).
- **Yazı tipi dışarıdan yüklenmez.** Arayüz `--font-ui`'yi kullanır; yazı tipi dosyaları `assets/fonts`'tadır, CDN ya da Google Fonts bağlantısı eklenmez. Çizimdeki yazı nesneleri arayüz yazı tipini izlemez.
- **Sabit piksel yazı boyutu yok.** `--fs-*` jetonları kullanılır; yükseklikler `--ui-scale` ile ölçeklenir (bkz. DESIGN.md).
- **Katman kimliğine göre dal yok** (`render/`, `viewport/`, `ui/`): davranış `LayerStyle`'dan gelir.
- **Model değişikliği:**
  - Arayüz, belgeyi yalnızca `CadDocument` API'si ya da komutlar üzerinden değiştirir.
  - `Signal`'lere dışarıdan `set` yalnızca sahibi olan depoda yapılır. Ayar pencereleri taslak üzerinde çalışır, Kaydet'te uygular.
- **Kaynak sızıntısı yok.** Her abonelik ve dinleyici bir `DisposableStore`'a eklenir. Global dinleyiciler (`window`) yalnızca açıkça bırakılabilen yerlerde kurulur.
- **Hata mesajları** ne olduğunu ve nasıl düzeltileceğini söyler, özür dilemez: "“Parsel sınırı” katmanı kilitli. Kilidi Katmanlar panelinden açın…"
- **Yapılmamış özellik** asla sessiz kalmaz: `pending(...)` komutu ya da `PendingTool` kullanılır.
- **Kullanıcıya soru** yalnız onay penceresiyle sorulur (`ui/widgets/confirm.ts`, §4.10); tarayıcının `confirm()`/`alert()`'i kullanılmaz.

### 8.1 Tamam sayılma listesi (her değişiklik için)

- [ ] `pnpm typecheck` temiz, `pnpm build` başarılı.
- [ ] Katman bağımlılık yönü korunuyor (§4.1).
- [ ] Yeni durum doğru kapsamda (§4.4).
- [ ] Yeni eylemler komut olarak kayıtlı, gerekiyorsa kısayolu ve takma adı var, menüde yer alıyor.
- [ ] Belge değişiklikleri geri alınabilir.
- [ ] Sayılar `ctx.format` üzerinden gösteriliyor.
- [ ] Renk, yazı ve ölçü DESIGN.md jetonlarıyla uyumlu; koyu ve açık temada denendi.
- [ ] Tarayıcıda gerçek fare ve klavyeyle denendi.
- [ ] Bu belge ya da DESIGN.md etkilendiyse güncellendi.

---

## 9. Tarifler

### 9.1 Yeni komut

`app/commands.ts` içinde `registerCoreCommands` listesine ekleyin. Kısayol
gerekiyorsa `app/keybindings.ts`'e, menüde görünecekse `app/menus.ts`'te uygun
bölüme komut kimliğini yazın: şerit menü modelinden kurulduğu için komut
şeritte o bölümün panelinde kendiliğinden çıkar. Şeritte düğme olacaksa
simgesi, adı uzunsa `short` adı olsun. Araç çubuğu düğmesi için
`commandButton(ctx, id, this.d)` kullanın.

### 9.2 Yeni araç

1. `tools/` içinde `Tool` uygulayın. Uygun aileden türetin (§4.7): nokta dizisi → `PointInputTool`, seçime dönüşüm → `SelectionFirstTool`, seçime tek adımlık işlem → `SelectionActionTool`, kenara etki → `EdgePickTool`. Geometri hesabını `model/ops/` altına saf fonksiyon olarak yazıp test edin; araç yalnızca akışı ve önizlemeyi yönetir.
2. `tools/catalog.ts`'e tanımı ekleyin: kimlik, etiket, simge, grup, gruplu bölümlerde bölüm (`section`), kısayol, takma adlar, açıklama, `steps`, `create`.
3. Simge yoksa `ui/icons.ts`'e 20×20, 1,4 px çizgili bir simge çizin (bkz. DESIGN.md §6).

Komut, kısayol, araç kutusu düğmesi, klasik menüdeki yeri, şerit düğmesi ve F1 listesi kendiliğinden oluşur; menüye ya da şeride elle eklenmez.

### 9.3 Yeni ayar

1. Kapsama karar verin (§4.4).
2. **Proje ayarıysa:** `ProjectSettingsData`, `PROJECT_SETTINGS_DEFAULTS`, `ProjectSettings` sinyali, `toJSON` ve `assign` güncellenir; `ProjectSettingsDialog`'a bölüm ya da satır eklenir.
3. **Uygulama ayarıysa:** `PreferencesData` ve `PREFERENCE_DEFAULTS` güncellenir; `AppSettingsDialog`'a satır eklenir.
4. Ayarı okuyan kod sinyale abone olur. Bölümün `keys` listesine alanı ekleyin ki "varsayılana döndür" çalışsın.

### 9.4 Testler

**Vitest** (`pnpm test`). Test dosyaları kodun yanında `*.test.ts` olarak durur ve yalnızca saf katmanları sınar:

| Dosya                                 | Kapsam                                                                                                                                                                                                                                                                                                                                                                                                                                                                                     |
| ------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| `model/geom/geom.test.ts`             | Afin dönüşüm (büyük TM koordinatında hassasiyet dahil), yay, kesişimler, öteleme                                                                                                                                                                                                                                                                                                                                                                                                           |
| `model/ops/ops.test.ts`               | Nesne dönüşümü, budama (kapalı şekil ve daire dahil), uzatma, öteleme, köşe yuvarlama, tutamaçlar                                                                                                                                                                                                                                                                                                                                                                                          |
| `model/geom/curves.test.ts`           | Eğri, tarama kırpma, ölçü yerleşimi (tüm türler, doğrusal yön seçimi, açı bölgesi), teğet noktaları, kenar ölçüleri                                                                                                                                                                                                                                                                                                                                                                        |
| `model/document.test.ts`              | Geri alma ve yineleme, `transact`, await arasında gruplama ve grubu iptal, çok nesne tek değişiklikte (`updateMany` sırayla `update` ile aynı sonuç, iki kez verilen kimlik ve düşen delikler dahil, her olaydan bir tane, tek geri alma adımı; işlemin içinde ve başarısız işlemde; `remove` tek olay, bilinmeyen ve yinelenen kimlik), katman devralma, grubun açılıp kapanma olayı (düzenleme sayılmaz), proje ayarları, `Formatter`                                                                                                                                                                                                                                                                                                                  |
| `model/byLayer.test.ts` | Katman başına dizin (`byLayer`, `countByLayer`): ekleme, değiştirme, silme, geri alma, yineleme, başarısız işlem, iptal edilen grup, `load`, `replaceWith`, dışarıdan gelen değişiklik ve katmanı değişen nesnede (belgedeki yerini korur) bütün çizimi gezmekle aynı nesneler ve sıra; tek tek ve toplu (`updateMany`, `addMany`, çok kimlikli `remove`) rastgele düzenleme dizisi |
| `model/sameJson.test.ts` | JSON metni kurmadan JSON eşitliği: `JSON.stringify` karşılaştırmasıyla aynı sonuç (−0 ve 0, NaN/±∞ ve null, dizide undefined ve boşluk, düşen alanlar, anahtar sırası; rastgele veri ve yakın kopyalar); belgenin her nesne türü ve alanında (`bulges`, `holes`, iç içe noktalar dahil) eski karşılaştırmayla aynı `changed`/`attrs` olayı |
| `render/grid.test.ts` | Izgara: görünüm kutunun içinde kaldıkça, aralık, orijin ve renkler aynıyken kurulan ızgaranın yeniden kullanılması; dünya hizalı, orijine göre, kutuyu baştan başa geçen çizgiler; görünümün içinde yalnız görünüm için kurulan ızgarayla aynı çizgiler |
| `model/geom/ellipse.test.ts`          | Elips: parametre, uzunluk (Ramanujan'a karşı), doğru kesişimi, en yakın nokta (yay ucu en yakınken de, TM'de tam dik ayak), teğetler, eksenden kurulum                                                                                                                                                                                                                                                                                                                                                                                  |
| `model/ops/curves2.test.ts`           | Elips nesnesi (aynalama, budama, kırma, uzatma, öteleme, tutamaçlar) ve yardımcı çizgiler (budama → ışın/çizgi, kırma, öteleme)                                                                                                                                                                                                                                                                                                                                                            |
| `model/geom/parallel.test.ts`         | Paralel çizgi yanları, gönye köşeleri, sıfır mesafe, koridor alanı, kapalı eksen                                                                                                                                                                                                                                                                                                                                                                                                           |
| `model/geom/survey.test.ts`           | Ölçmecilik yapıları ve işaret kuralları                                                                                                                                                                                                                                                                                                                                                                                                                                                    |
| `model/geom/shapes.test.ts`           | Dikdörtgen ve düzgün çokgen yapıları, yay yöntemleri                                                                                                                                                                                                                                                                                                                                                                                                                                       |
| `model/geom/bulge.test.ts`            | Bulge yardımcıları, teğet devam, ters çevirme, TTY ve TTT daireleri (üçgenin iç teğet dairesi, üç daire, TM koordinatında doğru-doğru-daire)                                                                                                                                                                                                                                                                                                                                               |
| `ui/promptOptions.test.ts`            | İstem ayrıştırma: araç, adım, seçenekler, değerler, notlar                                                                                                                                                                                                                                                                                                                                                                                                                                 |
| `app/menus.test.ts` | Tek kaynak: araç bölümleri (her araç kendi grubunun bölümünü adlandırır, sıra, bölümsüz araç grubun adıyla sonda), menülerin kataloğun her aracına ulaşması, aynı başlıklı blokların birleşmesi, komutun menüde bir kez yer alması, kataloğa eklenen aracın başka bir şey yazmadan grubunun menüsünde çıkması |
| `app/ribbon.test.ts` | Şeridin her aracı ve menülerin her komutunu içermesi (hızlı erişimle), kataloğa eklenen aracın şerit sekmesinde kendiliğinden çıkması, İşlemler sekmesinin kayıttan ve model kitaplığından kurulması, sekmede tekil panel başlıkları ve komutlar, anlama göre düğme boyu (ana araç büyük, gerisi küçük, ana öğesiz bir ya da iki öğeli panel büyük, sıkışık panel küçük, panelde en çok dört büyük, ana aracın her sekmede aynı boyu), aileler ve yöntemler bölünmüş düğmede (Giriş'e ailesiyle gelen dikdörtgen), seyrek araçlar panel ▾ listesinde, yalnız bilinen sekme, komut ve menülerin adlandırılması |
| `app/appearance.test.ts` | Görünüm: varsayılanların lacivert, Plus Jakarta Sans ve (çizimde) Barlow olması, çizim yazı tiplerinin sözleşmeyle aynı listesi, amber dışındaki her vurgunun iki temada her jetonu tanımlaması, sistem dışındaki her arayüz ve çizim yazı tipinin Latin ve Latin Extended yüzünün yerel dosyadan (başka sunucu yok) gelmesi, dosyaların ve OFL lisansının var olması |
| `app/workspaces.test.ts` | Çalışma modları: sözleşmeyle aynı kayıt, duyurulan modların Hibrit gösterilmesi, Hibrit'in süzgeçsiz arayüzle aynılığı, her modda gösterilen her aracın menüde ve şeritte olması ve gizlenenlerin hiçbir yerde olmaması, CAD'in menüleri ve “Ölçme” sekmesi, CBS'nin Giriş Harita paneli, adlandırılan menü, araç ve komutların varlığı |
| `viewport/objectTracking.test.ts`     | Nesne izleme (çekirdekten): tek hiza, kesişim, son noktayla kesişim, kutupsal açılar, hiza boyunca mesafe                                                                                                                                                                                                                                                                                                                                                                                  |
| `model/geom/region.test.ts`           | Alan cebiri: örtüşen, komşu (ortak kenar), T-bağlantılı, köşede değen, delikli alanlar; daire ve yay kenarları; TM koordinatında girdi köşelerinin bit bit korunması; bölme, yüzler, adalar, sarkan çizgi                                                                                                                                                                                                                                                                                  |
| `model/ops/areas.test.ts`             | Nesne ↔ alan dönüşümleri; adalı alanda alan, çevre, kenar, aynalama, tutamaç, esnet, patlat ve belgenin deliği düşürmesi                                                                                                                                                                                                                                                                                                                                                                   |
| `render/triangulate.test.ts`          | Delikli halkaların üçgenlenmesi (köprü, iç bükey köşe), toplam alan                                                                                                                                                                                                                                                                                                                                                                                                                        |
| `render/fillQueue.test.ts` | Katmanın dolgularının tek çağrıda üçgenlenmesi: her dolgu, adıyla tek başına üçgenlense alacağı üçgenleri kendi dizisine, sırasıyla ve bit bit alır (üçgensiz halkalar dahil) |
| `render/batches.test.ts` | Uzak görünümde okunamayacak kadar küçük yazının atlanması (dünya ve ekran birimi, dpr) |
| `model/ops/edit.test.ts`              | Uzat-kısalt (çizgi, yay, köşeleri aşan kısaltma, yayla biten çoklu çizgi, imleçten boy); yaylı çoklu çizgide uzunluk/alan/budama/uzatma/öteleme; birleştir, patlat, kır, esnet, köşe ekle/sil, pah ve köşe yuvarlama, bölme                                                                                                                                                                                                                                                                |
| `tools/coordinateInput.test.ts`       | Mutlak, göreli, kutupsal ve mesafe girişi (noktalar çekirdekten)                                                                                                                                                                                                                                                                                                                                                                                                                           |
| `tools/tracking.test.ts` | İmleç kısıtlaması (çekirdekten): kenetli ve ilk noktada olduğu gibi, orto (büyük olan hareket), kutupsal kilit ve açının 0–360'a çevrilmesi, ışından uzak imleç |
| `tools/constructions.test.ts` | Araçların çekirdekteki yapı hesapları: orta nokta, oran, açı birimi, yakın çözüm; doğrultu, yarıçapla düzgün çokgenin yatay alt kenarı, yay devamı (çizgi, yay, yaylı çoklu çizgi), çaptan daire, elips parametresi ve döndürme, açıortay, yarıçap ve merkezle yay parçası, okunur yazı açısı, halka; döndürme ve ölçek, kutupsal dizi (dönmeyen kopyaların TM'de aynı ötelemesi), hizala; çoklu çizgi ve iki çizgi köşesi, çekilen boyun adımı, köşe yayı ve pah, açı ve yarıçap ölçüsünün kolları |
| `processing/processing.test.ts`       | Numara biçimi, köşe sırası ve ortak köşe (boş halka, adsız var olan nokta, başlangıçtan çok uzakta toleranssız ortak köşe), “görünen” kapsamı, parametre varsayılanları ve doğrulama, kayıt ve arama, çalıştırıcı (belgeyle, tek geri alma, boş girdi), tür süzgeci ve alan özetleri, ifadeyle seçim kipleri, öznitelik hesabı (etiket, boş sonuç, koşul, geri alma), model sıralama, denetim ve tür uyumu, model çalıştırma (zincir, tek geri alma, hatada geri alma), model düzenleme (adlandırma, zincirleme, uygun kaynaklar, silme, dizme)                                                 |
| `processing/worker/worker.test.ts`    | Worker'da çalıştırma (sahte worker, yapılandırılmış kopya): sayfayla aynı sonuç ve tek geri alma, worker'ın kendi deposundan kenar ölçüleri ve alan, worker'da ifade derleme, Otomatik seçim eşiği, bilinmeyen araç, çöken worker, Durdur ve yeni worker                                                                                                                                                                                                                                                                                      |
| `style/svg/svg.test.ts` | SVG çizim modeli (bu ve aşağıdaki `style/svg` testleri SVG çekirdeğini, `crates/shared/svg-core`, WASM'dan sınar): yol verisi (bütün komutlar, bitişik yay bayrakları, yay → kübik, geri yazma), kutular, türü koruyan dönüşümler, gruplu ve parametreli SVG çıktısı, düzgün çokgen/yıldız, içe alma (dönüşümler, boyalar, atlananlar) |
| `style/legend.test.ts` | Lejant: işleyicisiz katman, kategoriler ve diğer değerler, kapalı kategori ve kurallar, üst kural adıyla alt kurallar, nesnelerin kendi sembolleri |
| `style/classify.test.ts`              | Katman stili sınıflama: ifade değerleri (geometri değerleri depodan, bütün nesneler için bir kez), benzersiz değerler ve doğal sıra, eşit aralık ve eşit sayı, renk rampası, geometriye göre basit semboller                                                                                                                                                                                                                                                                                                                                         |
| `style/system/system.test.ts`         | Sistem kitaplığı: benzersiz kimlikler, her sembolün doğrulanması, kullanılan çizimlerin varlığı, her öğenin kategorisi                                                                                                                                                                                                                                                                                                                                                                     |
| `style/style.test.ts`                 | Stil motoru: alan halkalarının yönü, derleme çekirdekten (`compileSymbol`: kesik ve kaydırma, dönüşümlü işaretler, çizgi boyunca okunur yazı, köşeden uzak duran kodlar, içe kaydırılmış kenar, tarama, öznitelikten yazı, veriye bağlı boyut/açı/renk/görünürlük, desen döşemesi, iç içe işaret düzeyleri), kitaplık (sistem salt okunur, kopya, ağaç ve arama, projeye varlıklarıyla kopya), .kstil (dışa/içe aktarma, çakışma kipleri, doğrulama, SVG temizliği). Birimler, yerleşim, dalgalar, iç nokta ve işleyiciler çekirdeğin kendi testlerindedir |
| `style/fixture.test.ts` | Dondurulmuş stil derleyicisi yanıtları (`fixtures/style/v1/cases.json`: 160 sembol bir nesnede, 48 katman) uygulamanın yolundan: tek sembol (`compileSymbol`) ve bütün katman (`buildStyledLayer`); katmanlarda sayfanın çekirdeğe verdikleri (program, nesne sayıları, ifade tablosu) de dosyayla karşılaştırılır |
| `model/expression/expression.test.ts` | İfade dili (çekirdekten, toplu değerlendirmeyle): alanlar ve değişkenler, metin-sayı aritmetiği, karşılaştırma ve boş değer kuralları, Türkçe/İngilizce işlevler, konumlu hata mesajları, önizleme                                                                                                                                                                                                                                                                                                                               |
| `model/expression/fixture.test.ts` | Dondurulmuş ifade yanıtları (`fixtures/expression/v1/cases.json`: 400 kaynak, hatalar ve konumları, beş sonuç biçiminde değerler) uygulamanın yolundan: sayfanın kurduğu tablo, WASM'da çekirdek, geri okunan sütun |
| `style/svg/importSvg.test.ts` | SVG içe alma: renk sözdizimleri ve alfa, bütün dönüşümler, viewBox ve preserveAspectRatio, CSS sınıf/kimlik/torun seçicileri ve devralma, `<use>`/`<symbol>`/`<defs>`, birimler (mm → çizim birimi, sembol boyu), iç içe `<svg>`, bütün ilkeller, saydamlık, kesik, uç, köşe, dolgu kuralı, degrade ve desenin düz renge inmesi, kırpma/maske/görüntü sayımı, `<tspan>` satırları, renk eşleme (siyah, baskın, ikinci renk), düzenleyicinin kendi kaynağı (kimlik, ad, grup, gizli, altlık), öznitelik seçicisi ve yazı boyu adının yalnız dosyadan okunması (`[constructor]`) |
| `style/svg/exportSvg.test.ts` | SVG dışa aktarma: sembol SVG (parametreler, mm boyu, zemin) ve geri okuma, düz SVG (önizleme renkleri, alfa → opacity, mm), seçime kırpma, `<defs>` içinde altlık, kaynak görünümü (kimlik, ad, gizli, öğe aralıkları, kararlı gidiş-dönüş), PNG boyu (piksel, DPI) ve pHYs parçası (CRC) |
| `style/svg/trace.test.ts` | Bitmap izleme: parlaklık ve alfa, tek piksel halkası, keskin köşeli kare, delikli halka ve içindeki ada, çapraz şeridin merdiveni, benek temizliği (gürültü), yumuşak düğümlü disk ve alanı, kapalı halkada Douglas–Peucker |
| `model/geom/golden.test.ts` | Rust çekirdeğiyle paylaşılan golden geometri durumları (`fixtures/geometry/v1/cases.json`): bulge yayı, uzunluk, halka ve işaretli alan, nokta-çokgen, delikli alan ve çevre; TM koordinatları; tolerans dosyada |
| `model/geom/reference.test.ts` | Bağımsız kesin referansa (Python kesirleri, 60 basamak π; `fixtures/geometry/v1/reference.json`) göre doğruluk: ondalık metinden TM parsel ve adalı alan, yaylı alanlar ve çevre; her durumun hata sınırı içinde (§23.4) |
| `model/snapshot.test.ts` | .kcad dosyası: 13 nesne türünün TM koordinatında bit bit gidiş-dönüşü (bulge, delik, elips, ölçü, proje sembolü), kilitli örnek dosyaya (`fixtures/document/v1/sample.json`) eşitlik, bozuk dosyaların Türkçe “yer: sorun” iletisiyle reddi, `dirty` akışı (geri alma, katman durumu, yazım sürerken yapılan değişiklik) |
| `app/fileIO.test.ts` | Kaydet/Aç (bellek içi dosyayla): işaret yalnızca yazımdan sonra temizlenir, yazım hatası ve vazgeçme kaydedilmemiş bırakır, yazım sürerken yapılan değişiklik kaydedilmemiş kalır, bozuk dosya açık çizime dokunmaz, yazılamayan dosya kayıt hedefi olmaz, aynı anda tek dosya komutu; Yeni proje (temiz çizimde sormadan, dosyasız ve tek paftalık görünümle; kaydedilmemiş değişiklikte Vazgeç/Kaydetmeden/Kaydet ve başarısız kayıtta durma; kendiliğinden kaydeden bulut projesinden sormadan ayrılıp cihazda kalanı söyleme; izleyiciye sorma; açık işlemde bekleme) |
| `model/newProject.test.ts` | Yeni proje içeriği: standart katmanlar (harita araçlarının hedefleri dahil), seçilen sistem ve ölçek, varsayılan birimler, çalışma alanı orijini, adın kırpılması ve varsayılanı, tanımsız SRID'nin reddi; belgeyi temiz ve geçmişsiz değiştirmesi, geçerli .kcad olarak kaydı, her projenin kendi stil kopyası, ilk görünümün bir pafta olması |
| `app/server.test.ts` | Sağlık yanıtının güvenilmeyen veri olarak okunması; yalnız aynı sözleşme sürümünde “bağlı”; 503, 404, HTML dizin sayfası, sözleşmeye uymayan gövde ve ağ hatasında “yok” ve nedeni; eşzamanlı denetimin paylaşılması |
| `contracts/contracts.test.ts` | Uygulama tiplerinin (`Entity`, `LayerNode`, `ProjectSettings`, `StyleFile`, `RunJob` …) Rust'tan üretilen sözleşmelere derleme anında uyması (`tsc` denetler) |
| `geo/crs.test.ts` | CRS kaydı: Rust ile paylaşılan dosyayla birebir aynılık (kayıt değişince yeniden kaydedilir; Rust tarafı EPSG değerlerine göre denetler), tekil SRID, varsayılan, dilim önerisi (sınırda batı dilimi), SRID/ad/bölge araması, yeni projenin çalışma alanı ortası (TM, UTM, coğrafi, Pseudo-Mercator) |
| `model/external.test.ts` | Dışarıdan gelen değişiklikler: `touched` olayları (geri alma ve başarısız işlem dahil), geri alma adımı ve kaydedilmemiş işareti yazmadan uygulama, başkasının dokunduğu nesnenin geri alma adımının silinmesi, dokunulmayanların korunması, proje bilgilerinin sessiz uygulanması, açık grupta reddetme |
| `app/cloud/sync.test.ts` | Bulut otomatik kaydı, sunucunun bellek içi benzeriyle (`fakeServer.ts`): yanıttan sonra “kaydedildi”, geri almayla sıfır gönderim, silip geri alınca aynı kimlikle yeniden açma, ölü ağda ve kaybolan yanıtta iki kez yazmama, çakışmada durma ve iki çözüm yolu, başka editörün değişikliğinin geri alma adımsız gelmesi, yerel değişiklikli nesnede çakışma, yeniden açılışta cihaz taslağı ve kayıp komutun aynı anahtarla gönderilmesi, yeniden açıldıktan sonraki düzenlemenin taslaktan önce gelmesi, proje bilgisi yetkisi, kurala uymayan gelen nesnenin reddi, yolda komutu varken bırakılan projenin sonraki çizime dokunmaması (geç yanıt ve olaylar atlanır, taslak iki değişikliği ve komutu tutar), başka editörün projeyi silmesi (olayla `deleted` durumu, önceki olayların nesnesi istenmez, sonraki düzenleme gönderilmez ve taslakta kalır, bir kez bildirilir) ve 410 alan komutun aynı duruma geçmesi |
| `model/singleSource.test.ts` | Tek hesap kaynağı bekçisi (§14): `model/geom`, `model/ops`, `model/expression` ve depo okuyucularında (`model/geometry.ts`, `entities.ts`, `render/triangulate.ts`, `tools/constructions.ts`, `coordinateInput.ts`, `viewport/objectTracking.ts`, `picking.ts`, `storeRecords.ts`, `processing/geometry.ts`) aritmetik ve `Math.` olmaması, TypeScript'in sözdizimi ağacıyla; sayma ve dizinleme serbest, istisnalar gerekçeli (`extendBounds`) ve kullanılmayan istisna hata; bekçinin kendisinin örnekle sınanması |
| `viewport/picking.test.ts` | Belgeyle eşitlenen geometri deposunun her turda baştan kurulan depoyla aynı yanıtları vermesi: örnek projede ve gizli/kilitli/yalnız kenar/ağaçta olmayan, etiket stilli katmanlı rastgele sahnede seçme, kenar seçme (süzgeçli), kenet, pencere ve kesişim seçimi, çevreleyen şekil, çakışanlar, sınır kenarları, etiketler, tutamaçlar, hayaletler, taşı/kopyala dönüşümü (paketli), esnet, toplamlar, kapsam, kutu, çizilen geometri, ifade değerleri, buda ve uzat; turlar arasında ekleme, taşıma, silme, geri alma, yineleme, işlem, öznitelik, dış değişiklik, katman durumu ve yeniden yükleme; depo sırasının belgeninkiyle aynılığı |
| `wasm/store.wasm.test.ts` | Dondurulmuş depo fixture'ları (`fixtures/geometry/v1/store-v1.json`, işlem araçlarının `store-processing.json`'u; S3c'de silinen TS'ten kaydedildi) WASM deposundan; işlem sorguları uygulamanın yolundan (`ObjectStore`); sorguların ortak okuyucusu `wasm/calls/storeCases.ts` (kaydedicilerle aynı) |
| `processing/runs.test.ts` | Örnek projede ve düzenlenen rastgele sahnede her yerleşik aracın rastgele değerlerle sayfada ve worker'ın iş işleyicisinde bit bit aynı sonucu vermesi; görünen kapsamın görünümün deposundan ve kendi deposundan aynı çıkması; ifade önizlemesinin depo değerleriyle ve nesne başına aynı olması |
| `wasm/pack.test.ts` | Paketli nesnelerin (`wasm/pack.ts`) her türde JSON ile aynı nesneyi kurması; −0 ve NaN'ın korunması |
| `wasm/transform.wasm.test.ts` | Taşı, kopyala ve yapıştırın depodan paketli yolu: paketlenenin her türde bit bit geri okunması (−0, NaN, ±∞; deponun kendi kopyasıyla aynı geometri), rastgele nesne ve afinlerde (öteleme, döndürme, eşit ve eşit olmayan ölçek, aynalama, bileşik; başlangıç yakını ve TM) JSON'lu çağrıyla (`transformEntities`) alan sırası dahil bit bit aynı sonuç, öbür alanların korunması ve özniteliklerin kendi kopyası, görünümün deposundan düzenlemeler arasında aynı sonuç, belgede olmayan nesnelerin (pano) 1…n numarayla yapıştırılması, −0'ın paketli yolda korunması, depoda olmayan nesnede açık hata (`TRANSFORM_ROUNDS` ile derin koşu) |
| `wasm/calls.wasm.test.ts` | Dondurulmuş çağrı fixture'ları (`fixtures/geometry/v1/calls-*.json`) uygulamanın yolundan (`core.ts` → WASM) |
| `wasm/triangulate.wasm.test.ts` | Tipli toplu üçgenleme (`triangulateMany`): dizinlerin gösterdiği koordinatların adıyla çağrılan `triangulate` ile bit bit aynı olması, basit halkada üçgen alanlarının toplamının halkanın alanına eşitliği, veriden büyük boyutların kırpılması |
| `wasm/calls/reference.test.ts` | Adıyla çağrılan işlemlerin bağımsız referansa (`reference-calls.json`, Python kesirleri ve 60 basamaklı kökler) göre doğruluğu: Rust çekirdeği (WASM), her durumun hata sınırı içinde |
| `io/apply.test.ts` | Okunan nesnelerin belgeye konması: bin nesne tek geri alma adımı ve tek olay, adla birleşen katman, grupta yeni katmanlar (ikinci içe aktarmada aynı grup), seçilmeyen katmanın dışarıda kalması, kilitli hedefin ve bozuk nesnenin hiçbir şeyi değiştirmeden reddi, çalışan bir modelin grubu açıkken beklemesi |
| `io/client.test.ts` | Biçim worker'ının sayfa tarafı, sahte worker'la: ilk istekte tek worker, yanıtların kimlikle eşleşmesi, koordinat listesinin kopyayla, DXF'nin kopyasız (daha büyük arabelleğe bakan görünüm kopyayla) gitmesi, DXF dışa aktarmanın nesnelerinin kopyayla gitmesi, yazılan dosya ve raporu, okuyucu hatasında worker'ın kalması, tuzakta ve yüklenemeyen worker'da bekleyenlerin Türkçe iletiyle düşmesi ve yeni worker, gönderilemeyen istek, yarım dakika boşta kapanma, Durdur |
| `io/coords.test.ts` | Koordinat listesi pencerelerinin yardımcıları: sütun sırasının dosyaya yayılması, seçimin hangi sıraya uyduğu, çizimin noktalarının satırlara dönüşmesi (ad etiketten ya da `Ad` özniteliğinden, Z ve Kod) |
| `io/formats.wasm.test.ts` | Rust dosya biçimleri WASM derlemesinde (`apps/web/src/io/pkg`), `fixtures/formats/v1` dosyalarıyla: sözleşme sürümü, Netcad NCN (bit bit koordinat), Windows-1254 ve ondalık virgüllü Türkçe tablo, bozuk satırların numaralı iletileri, yazıp geri okuma; DXF: katman tablosu (renk, gizli, kilitli), tam koordinat, patlatılmış bloklar, rapor, DWG reddi; DXF yazma: AutoCAD 2007 başlığı, rapor, geri okununca aynı nesneler (TM'de bit bit koordinat, etiket, öznitelik, renk, ada, yayın radyanları, KentOS eğrisi, 0 kotu, ölçüler: MTEXT'in özel işaretlerini taşıyan kendi yazısıyla da) ve katmanlar (tema rengi, gizli, kilitli, çizgi tipi, kalınlık). Paket yoksa atlanır |
| `wasm/golden.wasm.test.ts` | Paketin çalışma alanı sürümüyle derlendiği (eski paket kırılır). Golden durumlar ve bağımsız referanslar S3b'den beri `model/geom/golden.test.ts` ve `reference.test.ts`'te uygulamanın yolundan (cephe → WASM) koşar. Paket yoksa atlanır; `pnpm test:rust` derleyip çalıştırır |
| `style/svg/pathOps.test.ts` | SVG düzenleyicisinin yol işlemleri: kesişen, komşu ve iç içe karelerde birleşim/kesişim/fark/dışlama, delik, boş kesişim, çizgiyle ve daireyle bölme; eğrilerin eğri kalması (iki dairenin birleşimi, daire deliği), even-odd halka ve tek çizgiyle yıldız; yolu kes (düz ve eğri, tam kesim noktası); çizgiyi yola çevirme (düz/kare/yuvarlak uç, sivri/pah/yuvarlak köşe, kapalı halka, kesik desen, az düğümlü eğri) ve içe/dışa öteleme; şekil düzeyinde birleşim, topla/ayır (delikler kalır), dolgulu çizgi, kaybolan şekil; sadeleştir, kapat, aç |
| `style/svg/nodeOps.test.ts` | Düğüm türleri (okuma, köşe → yumuşak/simetrik/otomatik, otomatiğin komşuyu izlemesi), ortaya düğüm ekleme (eğride ve kapanış parçasında), biçimi koruyarak silme, uçları birleştirme (iki yol, kendi kendini kapatma), düğümde kırma, parça silme, düz/eğri parça, köşe yuvarlama ve pah (yarıçap, komşuya varan kesim, büyük yarıçap, düz devam eden ve uç düğüm, çoklu köşe, eğri kenar, sürükleme uzaklığından yarıçap), hizala ve dağıt |
| `style/svg/arrange.test.ts` | Birimler (grup tek birim, seçim sırası), seçime/ilk/son/en büyük/tuvale hizalama, blok olarak hizalama, eşit aralık ve eşit boşluk; taşı (göreli, mutlak, ayrı ayrı adımla), ölçek, döndürme yönü ve merkezi, eğme, kutuya göre matris; satır-sütun, dairesel (tam tur ve yay, dönmeden) ve aynalı dizi, kopyaların grupları; sıra (öne, arkaya, en öne, en arkaya) |
| `style/svg/snapping.test.ts` | SVG düzenleyicisinin kenetlemesi: köşe/yumuşak düğüm, parça ortası, ağırlık merkezi, kutu noktaları, tuval köşesi ve kenarı, kesişim (eğriyle dahil), dik ayak ve teğet noktası (başlangıç noktasından), kılavuz, kılavuz kesişimi ve kılavuzla kesişim, noktaların çizgilerden önce gelmesi, taşınan şekil ve düğümlerin dışarıda kalması |
| `style/svg/fixture.test.ts` | SVG çekirdeğinin dondurulmuş yanıtları (`fixtures/svg/v1/cases.json`) WASM paketinden: tablonun her işlemi (anahtar sırasıyla ve sayı sayı; reddedilen çağrının iletisi) ve tipli girişler (kenet dizini, resmin ve PNG'nin baytları) |

**Rust testleri** (`pnpm rust:test`: `cargo test` + clippy `-D warnings`):
- `crates/shared/geometry-core/tests/golden.rs`: TS ile aynı golden dosya ve bağımsız referanslar;
- `tests/store.rs`: geometri deposu dondurulmuş TS yanıtlarına karşı (bütün `store-*.json`: `store-v1.json`, işlem araçlarının `store-processing.json`'u); depo birim testleri: belge sırası, R-ağacı, paketli okuyucu ve yazıcı (her türün bit bit gidiş-dönüşü, −0, NaN, ±∞), paketli dönüşümün JSON'lu `transformEntities` çağrısıyla aynılığı (TM'de her tür, her afin türü, bilinmeyen kimlik), işlem sorguları;
- `tests/calls.rs`: dondurulmuş çağrı fixture'ları çekirdeğin çağrı tablosundan (durumun gerekçeli `tol` sınırıyla) ve bağımsız referans (`reference-calls.json`); `jsmath` (Math.round, sign, min/max, V8 `Math.hypot`), `api` (JSON yazıcı: NaN/±∞, en kısa sayı biçimi; çağrı tablosu);
- `tests/numeric.rs`: §23 yuvarlama, hisse ve dağıtım, Python'la üretilmiş dosyalara karşı;
- `crates/shared/contracts/tests/document.rs`: .kcad örneğinin gidiş-dönüşü ve reddi;
- `tests/crs.rs`: CRS kaydının EPSG değerleri;
- `apps/api`: sağlık isteği, gerçek HTTP ile;
- `crates/wasm/geometry-wasm`: art arda dizilmiş halkaların aralıkları (veriden büyük boyutlar kırpılır).
- `crates/shared/geometry-core` `geom/arrangement.rs`, `ops/transform.rs`: sarım dizininin açı toplamıyla aynı sonucu (yay ucundan geçen ışın dahil), toplu dönüşümün sırası.
- `crates/shared/formats` (`cargo test -p kentos-formats`): kodlamalar (Windows-1254 Türkçe harfleri, UTF-16, BOM), kesin sayı okuma ve en kısa yazım (bit bit gidiş-dönüş, `nan`/`inf` reddi, ondalık virgül), rapor gruplama; koordinat listesi: ayırıcı, ondalık, başlık ve sütun önerisi (sayı büyüklüğüyle X/Y ayrımı, uluslararası başlık), tırnaklı CSV, yorum satırları, numaralı hata iletileri, dört ayırıcı ve iki kodlamayla yazıp okuma; `tests/coords.rs` fixture dosyaları. DXF: grup çiftleri (CRLF/LF, bozuk dosyada satır numarası), Türkçe kod sayfası, `\U+` ve `%%` kodları, MTEXT biçim temizliği, ACI renk tablosu, keyfî eksen algoritması, dörtte bir dönüşlerin tam değeri, gerilmiş daireden elips, NURBS örnekleme (ikinci derece Bézier, rasyonel çeyrek çember, iç düğümler, derece sınırı), tarama sınır yolları (örneklenen ve yerine denetim noktası konan eğriler), saat yönünde yay kenarı, uygulamanın yay adımı ve ortak çekirdekle örneklenen yaylı halka ve açık yol; `tests/dxf.rs`: her tür ve katman tablosu, aynalı nesne koordinat sistemi, iç içe ve dizi bloklar (0 katmanı ve BYBLOCK devri, öznitelikler, kendini içeren ve eksik blok), adalı ve çapraz taramalar, ölçünün bloğundan patlatılması, DWG/ikili/bozuk dosya iletileri, 60 000 nesnede tek geçiş ve nesne sınırı, katman adının tablodaki yazılışı, okunamayan özniteliğin raporu, hiçbir şey çizmeyen iç içe bloklarda dolaşma sınırı, MINSERT ızgarası ve 10 000 sınırı. DXF yazma: KentOS verisi (her öğenin geri okunması, bilinmeyen öğelerin atlanması, şapka gösterimi), uygulama renklerinin DXF rengine çevrilmesi, yazıcı girdisinin sözleşmenin kendi okuyucusuyla aynı okunması ve aynı reddi, katman adları ve kalınlıklar, yazı değerleri, MTEXT gösterimiyle yazılan değerlerin aynı geri okunması; `tests/dxf_write.rs`: 13 türün (beş ölçü biçimi dahil) TM'de yazılıp aynı nesne olarak geri okunması (adalı alan, yaylı yol, eğri, tarama desenleri, Türkçe yazı, etiket ve öznitelikler), katman tablosu (renk, çizgi tipi deseni, kalınlık, gizli, kilitli), AutoCAD'in istediği yapı (bölümler, tablo sırası, tekil tutamaçlar ve çözülen başvurular, kayıtlı uygulama adları, kapsam, birim), ölçünün DXF ölçüsü olması (türü, tanım noktaları, kendi anonim bloğu ve blok kaydı, tek ARC olan açı yayı, bloğun 0 katmanında BYBLOCK renkli MTEXT değeri, DSTYLE boyları, 42'de ölçülen değer), başka programda türü ya da tanım noktası değiştirilen ölçünün bloğundan çizilmesi ve söylenmesi, başka programda değiştirilen renk ve açının KentOS verisinden önce gelmesi, DXF'in olduğu gibi tutamadığı adlar ve öznitelikler (16 KB sınırı, aynı adlı katmanlar, yazılmayan nesneler), boş çizim.
- `crates/server/postgres`: ortam dosyası, kurulum adı ve parola denetimi.
- `crates/shared/geometry-core` `tools`: kutupsal dizinin dönmeyen kopyalarının TM'de aynı ötelemesi, çekilen boyun adımı, izlemede “tam üstü”nün aynı X'i, hizaların eksenlerle birleşmesi, serbest imlecin `tracking: null` yazılması.
- `crates/shared/geometry-core` `geom/spline.rs`: eğrinin Bézier açıklıklarının çizilen Catmull-Rom eğrisiyle aynılığı (TM'de ve yinelenen noktada), komşu açıklıkların aynı teğetle birleşmesi, iki noktanın tek düz açıklık olması.
- `crates/shared/style-core` (`cargo test -p kentos-style-core`): stil derleyicisi (`style/tests.rs`: birimler, çizgi boyunca işaret yerleri ve gruplar, keskin köşeden uzak durma, dalgalar ve adımlı dalgalar, alanın iç noktası, kategorili/aralıklı/kural tabanlı işleyiciler ve ölçek aralıkları, eşit stillerin orijine göre tek toplulukta toplanması, ölçü çizgilerinin her ölçekte çizilmesi, yazı ve ölçüde tek sembolün boş kalması; `tests/style.rs` dondurulmuş sembol ve katman yanıtları), ifade dili (`expr/tests.rs`: TS testlerinin karşılığı, tablo ve sonuç biçimleri, tablo boyu uymayınca ret), JavaScript'in sayı yazımı (`String`, `toPrecision`, `toFixed`: tam sayılı yollar ve en kısa biçim bütün ondalık açılımla karşılaştırılır; `NUMBER_ROUNDS` derin koşu), UTF-16 metin, Türkçe büyük/küçük harf ve sıralama; `tests/cases.rs` dondurulmuş ifade yanıtları.
- `crates/shared/svg-core` (`cargo test -p kentos-svg-core`): JavaScript'in sayı okuması (`Number`, `parseFloat`, `parseInt(s, 16)`), renkler ve boya zincirleri (`url(#a) url(#b) red`), stil sayfaları (tek geçişte yorum temizliği, özgüllük, alan sırası), düz gelen dosya ağacı; içe almada 20 000 iç içe grup ve 2 000 derinlikte `<use>` zinciri (çağrı yığınına ulaşmaz, döngü kırık sayılır); `tests/cases.rs` dondurulmuş yanıtlar (tablonun her işleminin en az üç durumu, tipli girişler).
- `crates/shared/geometry-core` `tessellate`, `ewkb`: kiriş toleransı (daire, yaylı yol, elips, eğri), PostGIS'in EWKB baytlarıyla birebir aynılık ve bit bit gidiş-dönüş.
- `crates/shared/geometry-core` `predicates`: uyarlamalı yön yüklemi (§23.3): Kettner'in sınıf ızgarası (yuvarlanmış determinantın yanıldığı yerler dahil) ve TM koordinatında 200 000 neredeyse aynı doğrultudaki üçlüde tam sayı aritmetiğiyle aynı işaret ve permütasyonlarda tutarlılık, kolay durumda düz determinant, kesin açılım toplamları; nokta-halka sınamasında ve sarım dizininde TM'deki bir kenarın bir iki ulp yanındaki 20 000 noktada kesin karar (eski yuvarlanmış kesişim bir kısmında yanılır); iki yarım daire yayından çember çapının üstündeki ve yanındaki 60 000 noktanın sarımı 1 (eski iki ayrı yuvarlanmış çapraz çarpım bir kısmında 0 ya da 2 verir); halka izlemede köşe çevresindeki sıra (`geom/overlay.rs`): 1 500 km'deki köşeden mikroradyan arayla çıkan düz parçalar tam sayı aritmetiğiyle aynı sırada, bir doğruya teğet kısa yaylar bükümlerine göre (eski kiriş kuralı ikisinde de yanılır), başlangıçtan 100 km'de kırpılmamış köşe yuvarlamanın çevresinde her noktanın tam bir yüzde olması; doğru çapraz çarpım (`cross_accurate`): TM'de 100 000 neredeyse paralel dörtlüde tam sayı aritmetiğinin 2⁻⁴¹ içinde, tam sıfırda sıfır, iyi durumda düz çarpımla bit bit aynı, `compress`; kesişim parametreleri (`geom/intersect.rs`): 20 000 neredeyse paralel TM parça çiftinde (dördünden biri ortak uçta) her kesişim bulunur, t tam değerin 1e-12'si içinde, bandın açıkça dışındakiler alınmaz (eski formül ortak uçtaki değmelerin bir kısmını kaçırır); `reference-calls.json`'da 85 kesin işaretli durum ve iki neredeyse paralel kesişim (Python kesirleri), donmuş çağrılar `calls-r1-predicates.json`.
- `crates/server/application/tests/identity.rs` (geçici veritabanı): tenant ayrımı (satır güvenliği, başka tenant adına yazma), sunucu rolünün yapamadıkları (proje satırı silmek dahil), yerel giriş, oturumlar, koltuk ve üyelik, OpenID kimliği.
- `crates/server/application/tests/changes.rs`: 13 türün PostGIS'ten bit bit dönmesi, sürümler, 409'da sunucu kopyası, idempotency, kilitli katman, yetkiler, tenant ayrımı, iki eşzamanlı yazar.
- `crates/server/application/tests/retention.rs`: olay günlüğünün budanması (bir saatten kısa pencerenin reddi, partiler, iki tenant'ın olayları, proje ufku, ufuktan önceki imlece `ResyncRequired`, sonrasının sürmesi, bütün olayları gitmiş projede açılış imlecinin ufuk olması, olaysız proje, budamadan sonra yeni olaylar, ufukta tenant ayrımı).
- `crates/server/application/tests/lifecycle.rs`: yeniden adlandırma (proje bilgisi sürümü, olay, editöre ve boş ada ret, eski sürümde çakışma); silme (yalnız yönetici, başka tenant'a 404, listeden kalkma, açma/okuma/yazmada 410, silinmeden önceki komutun günlükten yanıtlanması, kim ve hangi istekle `project.deleted` olayı, ikinci silmenin bir şey değiştirmemesi, nesnelerin ve denetimin kalması), işletmecinin listelemesi ve geri getirmesi; silme ile yazma aynı anda.
- `apps/api` (`http/tests.rs`, `http/ws_tests.rs`, `oidc/tests.rs`, `cli.rs`, `config.rs`): WebSocket gerçek soket üzerinde (RFC 6455'in gereken kısmını testin kendisi konuşur): budanmış günlükte eski imlece `resyncRequired`, ufuktan kalan olay, en yeni olayın ötesine `resyncRequired`, HTTP olay günlüğünde 410 `resync_required`; saklama süresi ayarının sınırları; istek kimliği, veritabanısız mod, çerezli giriş ve CSRF başlığı, proje/komut/olay yolları ve başka tenant'ın 404'ü; proje silme (proje yöneticisine ve izleyiciye 403, başka tenant'a 404, başlıksız isteğe 403, tekrarı 204, sonrasında açma/okuma/yazmada 410, listede yok, olay günlüğünde var); komut satırında tek başına `--`; sahte sağlayıcıyla OpenID (kod + PKCE, tek kullanımlık state, nonce, audience, HS* reddi, anahtar yenileme sınırı, erişim belirteci).
- **Bulut uçtan uca** `apps/web/scripts/e2e/cloud.mjs` (`pnpm e2e:cloud`): gerçek `kentosd` ve geliştirme veritabanı, tarayıcıda `ayse`, HTTP üzerinden ikinci editör `mehmet`: giriş, yükleme, otomatik kayıt (tam koordinat), yeniden yüklemede kalıcılık, canlı değişiklik, çakışma ve çözüm, sunucu dururken bekleyen düzenlemenin geri gelince bir kez kaydı, yeniden bağlanma; açık projenin listeden yeniden adlandırılması (proje yöneticisinde Sil'in devre dışı ve nedenini söylemesi), yönetici `zeynep`'in açık projeyi HTTP'den silmesi (açık tutanda “Proje silindi”, çizim yerinde, sonraki düzenleme cihaz taslağında, 410 ve listede yok), tarayıcıda `zeynep` olarak listeden onaylı silme (Vazgeç odakta). Oluşturduğu “E2E …” projeleri geliştirme veritabanında kalır; sildikleri yalnız işaretlenir.

Kurallar:

- `model/geom` ve `model/ops` altındaki her yeni fonksiyon test ile gelir. Sınır durumları (paralel, çakışık, sıfır uzunluk, açı 0/2π geçişi) mutlaka sınanır.
- Hata düzeltmesi, önce hatayı yeniden üreten bir testle başlar.
- **Uçtan uca duman testi** `apps/web/scripts/e2e/smoke.mjs` (`pnpm e2e`): kendi Vite sunucusunu açar, başsız Chrome'u DevTools protokolüyle (`apps/web/scripts/e2e/cdp.mjs`, bağımlılıksız) sürer, gerçek fare ve klavye olayları gönderir ve belgeyi `window.kentos` ile doğrular. Ekran görüntüleri `apps/web/scripts/e2e/out/`'a düşer. Çizim, budama, eğri, ölçü, yazı, tarama, yerinde düzenleme, yay kipli çoklu çizgi, patlat/birleştir, pah, kır, pano, taşı, kopyala, aynala, dikdörtgen dizi ve Ctrl+V ile yapıştır (yazılan noktalarla tam koordinat, öbür alanların korunması, tek geri alma adımı), araç kutusu (tüm araçlar kaydırmasız görünür, grup katlama), komut şeridi düğmeleri, fareyle köşe yuvarlama, basılı sağ tıkla tek seferlik kenet, imleç yanında değer girişi, tutamaç menüsü, nesne izleme, panel boyutlandırırken siyah kare çıkmaması (`Page.startScreencast` ile), Katmanlar paneli (nesne ekleyip geri alıp yineleyince katmanın ve grubunun sayısının izlemesi ve satırların aynı öğe kalması; göz düğmesinin katmanı aynı satırda gizleyip odağı almaması; 300 katmanlı grupta yalnız pencere yakınındaki satırların kurulması, End'in son satıra kaydırıp onu sayısı ve yeriyle kurup seçmesi), SVG düzenleyicisinde çift tık (kırık çizgiyi bitirir, yolun düğümlerini açar, düğümü yumuşatıp köşe yapar, parçaya düğüm ekler) ve seçili iki şekilden birine hareketsiz tık (seçimi daraltır, hiçbir şeyi taşımaz, geri alma adımı yazmaz), onay penceresi (dokunulmamış SVG çiziminin, yeni modelin ve yeni sembolün sormadan kapanması; değişiklikle × basınca sorunun ayrı pencerede, “Kaydet ve kapat” odakta açılması, Tab'ın pencerede kalması, Esc, Vazgeç ve ikinci ×'in çalışmaya dönmesi, “Kaydetmeden kapat”ın hiçbir şey kaydetmemesi, “Kaydet ve kapat”ın modeli kaydedip kapatması, sembolün adı değişince Vazgeç'in sorması; kitaplıktan silmede Vazgeç odakta, Esc'in silmemesi, Sil'in silmesi), paralel çizgi ve dik çık (yazılan mesafelerle tam koordinat), alan işlemleri (Alt+B birleştir, Alt+C ile ada bırakan çıkarma, adalı alanın taranması, Shift+B ve çizgilerle sınırlı tarama ile çizgilerin kapattığı bölgeye tıklayarak alan), işlem araçları (İşlemler menüsünden pencere, canlı girdi sayısı ve önizleme, çalıştırma, geçmiş, tek geri alma adımı; ifadeyle seçimde canlı eşleşme sayısı ve seçim, Web Worker'ın sayfayla aynı sonucu vermesi, yerleşik modelin tek geri alma adımıyla çalışması, tasarımcıda girdiye bağlı adımlı modelin kaydedilmesi), uygulama menüsü (KentOS logosu açar ve modül o zaman yüklenir; İçe aktar biçimleri, Bulut bölmesi, Esc, satırın komutu), Yeni proje (`Ctrl+Alt+N`: ad, sistem, ölçek ve çalışma modu, “Yakında” kartın seçilmemesi; kaydedilmemiş değişiklik sorusu ve Vazgeç'in pencereye dönmesi; boş çizimde yazılan çizgi ve ilk kayıtta yerin proje adıyla sorulması), çalışma modları (CAD'de Harita, Koordinat ve İşlemler menülerinin ve parsel aracının gizlenmesi, `PARSEL`'in adıyla yine çalışması; durum çubuğundan Hibrit'e dönüş, menüde 3D Plan ve Afet Analizi'nin “Yakında” yazması), şerit (Uygulama ayarlarından seçilip hemen yerleşmesi, kataloğun her aracının şeritte bir düğmede, bölünmüş düğmede ya da panel ▾ listesinde olması ve her düğmenin kayıtlı komutu, Daire ▾'nin beş yöntemi ve 3 nokta'nın aracı o yöntemle başlatıp hatırlanması, Eğri ▾'deki Halka, şeritten çalışan aracın dolu amber düğmesi ve Giriş'teki noktası, 1100 px'te her sekmenin panelleri küçülterek sığması ve 1600 px'te Değiştir'in etiketlerini koruması, seçimle çıkan Seçim sekmesi ve Esc ile kaybolması, Ctrl+F1 ile daraltma ve komutla kapanan açılır şerit, Alt+Q ile arayıp Enter'la çalıştırma, sağ tıkla hızlı erişime ekleme, açık tema, Şerit arayüzü düğmesiyle klasiğe dönüş), varsayılan motorun WebGL2 olması, durum çubuğundan WebGPU'ya canlı geçiş ve iki motorun aynı sahneyi çizmesi (ızgara kapalı karşılaştırılır; soluk ızgara çizgileri motorlar arasında yalnızca örneklemeyle farklılaşır), dosya alışverişi (bellek içi seçiciyle koordinat listesi içe aktarma: önizlemede Ad Y X Z önerisi ve bozuk satır, başka koordinat sistemi seçilince içe aktarmanın kapanması, tam koordinatlar ve dosya adıyla yeni katman, tek geri alma adımı; seçili noktaların NCN olarak aynı metinle dışa aktarılması; DXF içe aktarmada katman tablosu ve alınmayanların raporu, dosya adlı grupta yeni katmanlar, tam koordinatla patlatılmış bloklar, işareti kaldırılan katmanın dışarıda kalması, içe aktarılan nesnenin hemen seçilebilmesi (geometri deposu), tek geri alma adımı; DXF dışa aktarmada seçimin katmanıyla listelenmesi ve değişeceklerin söylenmesi, AutoCAD 2007 dosyasının ölçüleri DXF ölçüsü olarak yazması ve çizimin kaydedilmemiş durumunun değişmemesi, dosya yeniden içe aktarılınca aynı adlı katmana dönmesi ve aynı nesnelerin (ölçüler ölçü olarak) bit bit, etiket ve öznitelikleriyle gelmesi), geri alma akışlarını sınar. Tam değer bekleyen kontrollerde noktalar komut satırından mutlak koordinatla girilir; ekrandan tıklanan nokta piksel yuvarlaması kadar (~0,1 m) sapar. Yeni bir kullanıcı akışı eklendiğinde buraya bir kontrol eklenir.
- Tıklama noktaları ekrandan tahmin edilmez; dünya koordinatından `camera.worldToScreen` ile hesaplanır.
- **Fixture kaydedicileri** uygulamanın yolundan kaydeder ve `apps/web/scripts/fixtures/record-*.test.ts`'tedir (`record-expression`: ifade yanıtları; `record-style`: stil derleyicisinin sembol ve katman yanıtları; `record-svg`: SVG çekirdeğinin yanıtları, tablonun her işlemi `style/svg/cases/calls.ts` `CALLS`'tan; `record-collation`: Türkçe sıralama tablosu, Node'un ICU'sundan `crates/shared/style-core/src/js/collation_tr.rs`): `GOLDEN_WRITE=1 pnpm -C apps/web exec vitest run scripts/fixtures/<ad>.test.ts` (değişen dosya bilinçli bir golden değişikliğidir, farkı okunur). Dilden bağımsız referans üreticileri kökten çalışır: `python3 scripts/fixtures/<ad>.py`.
- **Etkileşim ölçümü** `apps/web/scripts/perf/interaction.mjs` (`pnpm perf:interaction`; bir şeyi doğrulamaz, ölçer): kendi Vite sunucusunu ve tek başsız Chrome'u açar (WebGL2 makinenin GPU'sunda, `--use-angle=gl`; SwiftShader'da tek kare saniyeler sürdüğü için onu reddeder), ADR 0005'in `parsel-50k` ve `hat-1m` veri setlerini sayfada tohumlu üretip `doc.replaceWith` ile açar. Gerçek fare hareketleriyle seç (üzerine gelme), çizgi (ilk noktadan sonra kenet) ve buda araçlarında olay başına süreyi 1:1000 ve genel görünümde, orta tuşla kaydırmada kare süresini ve GPU dahil kare aralığını, buda önizlemesinde kare süresini, stil değişikliğinde büyük katmanın yeniden kurulmasını `view.probe` ile toplar. Her koşu sayfayı yeniden açar; 3 koşunun p50/p95/p99'u ve p95 aralığı `docs/perf/interaction-<etiket>.{json,md}`'ye yazılır (`--label baseline` tabanı yazar; başka etiket tabanla karşılaştırılır). Nesne izleme ve bilgi kartı ölçümde kapalıdır (beklemeye bağlıdırlar). Makinede başka ağır süreç çalışırken çalıştırılmaz.
- **İfade ölçümü** `apps/web/scripts/perf/expression.test.ts` (`EXPRESSION_BENCH=1 pnpm -C apps/web exec vitest run scripts/perf/expression.test.ts --disable-console-intercept`; ölçer, doğrulamaz): 100 000 nesnede dört tipik ifade (koşul, numaralama, alan yazısı, yuvarlama), tablonun kurulması ve bütün değerlerin okunması dahil; p50/p95.
- **Katman kurma ölçümü** `apps/web/scripts/perf/style.test.ts` (`STYLE_BENCH=1 pnpm -C apps/web exec vitest run scripts/perf/style.test.ts --disable-console-intercept`; ölçer, doğrulamaz): kural tabanlı parsel katmanı (dolgu, kenar, alan yazıları), MPYY işaretli sınır çizgisi, yalın görünüşte çizgiler ve kategorili noktalar; `buildStyledLayer`'ın p50/p95'i, topluluk ve sayı adedi.
- **Ağır modüllerin açılışı** `apps/web/scripts/perf/modules.mjs` (`pnpm build`, sonra `apps/web`'de `node scripts/perf/modules.mjs --label <etiket>`; ölçer, doğrulamaz): üretim derlemesini `vite preview` ile sunar; stil yöneticisi, model tasarımcısı ve SVG düzenleyicisini her biri boş bir profille komut satırından (takma ad + Enter) açar; Enter'dan pencerenin belgeye girmesine ve boyandığı kareye kadar sayfanın içinde ölçer, sonra kapatıp ikinci açılışı ölçer, ilk açılışın isteklerini (JS, CSS, WASM, aktarılan boyutlarıyla) yazar → `docs/perf/modules-<etiket>-<tarih>.{json,md}`. Makinede başka ağır süreç çalışırken çalıştırılmaz.
- **Taşı/kopyala/yapıştır ölçümü** `apps/web/scripts/perf/transform.test.ts` (`TRANSFORM_BENCH=1 pnpm -C apps/web exec vitest run scripts/perf/transform.test.ts --silent=false --reporter=verbose`; ölçer, doğrulamaz): 10 000 karışık nesne TM koordinatlarında, JSON'lu çağrı ile deponun paketli yolu aynı süreçte yan yana; dönüşümün ve komutun (geri alma adımıyla; araçların yazdığı gibi tek değişiklikte ve karşılaştırma için nesne nesne) p50/p95'i.
- Sıradaki eksikler: `core` (komut arama, kısayol çözümleme).

### 9.5 Çizim arka uçları (WebGL2 ve WebGPU)

`render/webgpu/WebGPUBackend.ts`, `WebGL2Backend` ile adım adım aynı `RenderBackend` sözleşmesini uygular:

- çizgiler için `line-list` hattı, köşe verisi `{pos: vec2f, dist: f32}`
- dolgular için `triangle-list`
- noktalar için örneklenmiş dörtgenler (WebGPU'da nokta boyutu yoktur); simge, köşe gölgelendiricisinde piksel cinsinden kurulur
- bağ grubu 0 kare verisi (öteleme, ölçek, piksel/metre, dpr, görünüm boyutu), bağ grubu 1 topluya ait stil (renk, kesik desen, nokta boyutu ve simgesi); stil tamponu yükleme sırasında bir kez yazılır
- 4× MSAA; karışım WebGL2 ile aynıdır

WGSL gölgelendiricileri (`render/webgpu/shaders.ts`) GLSL'deki kesik desen ve nokta simgesi mantığının birebir karşılığıdır; birinde yapılan değişiklik ötekine de yapılır. `lib.dom` yalnızca WebGPU bayrak tiplerini bildirir; `GPUBufferUsage` gibi sabitler dosyada belirtimdeki değerleriyle tanımlıdır. `createBackend` tarayıcı desteklemiyorsa WebGL2'ye düşer.

Başsız Chrome'da `--enable-unsafe-webgpu` tek başına yetmez (aygıt ilk gönderimde düşer). `apps/web/scripts/e2e/cdp.mjs` içindeki `WEBGPU_ARGS` Vulkan/SwiftShader bayraklarını ekler; duman testi iki motorun çizdiği piksel sayısını karşılaştırır.

### 9.6 Yeni işlem aracı

`processing/builtin/<ad>.ts` içinde `defineTool({...})` ile tanımı yazın, `BUILTIN_TOOLS` listesine ekleyin; hesabın saf kısmını test edin ve çalıştırıcıyla belge üzerinde bir test ekleyin. Arayüz kodu yazılmaz. Tarif ve kurallar: [docs/PROCESSING.md](docs/PROCESSING.md) §8.

### 9.7 Yeni dosya biçimi

Biçimler Rust'tadır, çünkü sunucunun ileride çalışacak içe aktarma işi aynı kodu kullanacak (§14; ADR 0009):

1. **Okuyucu/yazıcı** `crates/shared/formats/src/<biçim>.rs`: baytlardan `ImportResult` (sözleşme `crates/shared/contracts/src/formats.rs`: kimliği 0 olan `Entity` listesi, `layerId` kaynak katmanın adı; katmanlar; rapor: türe göre sayılar, alınmayanlar ve dönüştürülenler, Türkçe neden ve ilk satır numaralarıyla) ya da nesnelerden bayt.
   - Koordinatlar dosyadaki ondalığa en yakın float64'tür; yazıcı geri okununca aynı sayıyı veren en kısa ondalığı yazar (`num.rs`).
   - Hiçbir girdi paniğe yol açmaz (lint); bozuk satır sayılır ve raporlanır. Tek geçiş, ikinci dereceden iş yok.
   - Aşkın işlevler `libm`'den gelir; native ve WASM aynı bitleri verir (`clippy.toml` std işlevlerini yasaklar).
   - Uygulamanın da hesapladığı geometri (yaylı halkanın noktaları, alan, içerme) `kentos-geometry-core`'dan alınır, kopyalanmaz; `geom.rs`'te yalnız dosya okumaya özgü olan durur.
   - Dosyanın boyutuyla değil, içindekinin çarpımıyla büyüyen iş (iç içe bloklar, diziler, eğri derecesi) sınırlanır ve sınıra varılınca raporlanır.
2. **WASM sınırı** `crates/wasm/formats-wasm`: dosya bayt, seçenekler ve sonuç JSON olarak geçer. JSON'u sınır crate'i ayrıştırır (`serde_json::from_str` başka bir crate'te çağrılırsa ayrıştırıcının ikinci bir kopyası modüle girer, ~25 KB); `Entity` listesi gibi büyük bir girdi serde'nin türettiği kodla değil biçimin kendi okuyucusuyla okunur (`dxf::WriteInput`; türetilmiş kod ~100 KB). Paket `apps/web/src/io/pkg`'a derlenir. `io/formatsWorker.ts` onu ilk istekte yükler (`?url` varlığı); `io/client.ts` worker'ı 30 sn boşta kalınca kapatır (büyük dosyanın belleği geri verilir), tuzakta yenisini açar.
3. **Pencere** `ui/io/`: dosya ve kaynağın bilgileri, seçenekler, önizleme ya da özet, “Bu koordinatlar hangi sistemde?” (`CrsQuestion`), hedef katman. İçe aktarma `io/apply.ts` ile tek geri alma adımıdır. Komut `app/fileExchange.ts`'e yazılır ve pencereyi dinamik içe aktarmayla açar.
4. **Testler:** Rust birim testleri ve `fixtures/formats/v1/` dosyalarıyla `crates/shared/formats/tests/`; aynı dosyalar WASM'da `io/formats.wasm.test.ts`; akış duman testinde, bellek içi seçiciyle.

Kaynağın SRID'si bilinmiyorsa kullanıcıya sorulur; asla tahmin edilmez. Kaynak projeden farklı bir sistemdeyse içe aktarma yapılmaz (dönüşüm yok, §5).

---

## 10. Yol haritası ve hedef mimari

1. **CAD çekirdeği:**
   - **Yapıldı:** yay; döndür, ölçekle, aynala, ötele, buda, uzat, köşe yuvarla, dikdörtgen dizi; tutamaçla düzenleme; kutupsal izleme; kesişim, dik, en yakın ve çeyrek kenetleri
   - **Yapıldı (2. aşama):** eğri; hizalı ölçü; tarama (dolu, çizgili, çapraz); yerinde yazı düzenleme; yazı açısı ve yüksekliği; teğet keneti; "Kenar ölçülerini yaz"
   - **Yapıldı (3. aşama, A grubu):** çoklu çizgide yay parçaları (bulge) ve yay kipi; birleştir, patlat (eğri → çoklu çizgi dahil), kır, esnet, köşe ekle/sil ve kenar ortası tutamaçları, pah ve çoklu çizgi köşesinde yuvarlama/pah; böl (eşit parça ve aralık); pano (kes, kopyala, yapıştır, özgün koordinata yapıştır); daire 2N/3N/TTY, merkezden yay
   - **Netcad çizim eşdeğerliği (referans Netcad'dir; AutoCAD ikincil ölçüttür. Bağlayıcı hedef: çizim kusursuz olmadan başka işe geçilmez):**
     - *Netcad'e özgü, var:* Koordinat hesap makinası (yan nokta, kenar kesişimi, doğru kesişimi, hat üzerinde nokta, açı-mesafe, orta nokta), köşe yuvarla/kır/sil, paralel al (ötele), böl, birleştir; **alan işlemleri** (birleştir, kesiştir, çıkar, böl; yaylar ve ortak sınırlar tam), adalı alan, alana çevir, içine tıklayarak alan (adalarla), çizgiye çevir; Paralel Çizgi (sol/sağ mesafe, gönyeli köşeler, eksen isteğe bağlı, koridor alan olarak); Dik in, Dik çık.
     - *Netcad'e özgü, eksik:* alanı verilen alana göre bölme (ifraz, topolojiyle); sembol ve blok yerleştirme; klotoid (spiral) nesnesi; poligon ve kutupsal ölçü hesapları (Hesap menüsü).
     - *Var:* ELLIPSE (eksenden, merkezden, döndürme, eliptik yay), XLINE (nokta, yatay, düşey, açı, açıortay), RAY, LINE (Geri, Kapat), PLINE (yay: teğet, açı, merkez, yarıçap, ikinci nokta, doğrultu; Uzunluk; Geri), DONUT (dolu tarama olarak), REVCLOUD (dikdörtgen, çokgen), RECTANG (köşe yuvarla, pah, döndür, boyutlar) ve üç noktalı dikdörtgen, POLYGON (içten, dıştan, kenardan), CIRCLE (merkez-yarıçap, merkez-çap, 2N, 3N, TTY, TTT), ARC (üç nokta; başlangıç-merkez-bitiş/açı/kiriş; başlangıç-bitiş-merkez/açı/yön/yarıçap; merkez-başlangıç-bitiş/açı/kiriş; devam), SPLINE, POINT, DIVIDE/MEASURE, TEXT, ölçüler (hizalı, doğrusal ΔY/ΔX, açı, yarıçap, çap), HATCH; MOVE, COPY, ROTATE (Referans, Kopya), SCALE (Referans, Kopya), MIRROR, STRETCH, dikdörtgen ve kutupsal ARRAY, ALIGN, LENGTHEN, OFFSET (mesafe, noktadan geç), TRIM, EXTEND (sınır seçme, Shift ile öbür işlem), BREAK, JOIN, EXPLODE, FILLET ve CHAMFER (çoklu, kırpmasız), tutamaçlar, tek seferlik kenet, nesne izleme, kutupsal izleme, orto, dinamik giriş.
     - *Eksik:* PLINE kalınlığı (genişlik; çizgi kalınlığı gelince); MTEXT; yol boyunca ARRAY.
   - **Stil motoru (sürüyor, [docs/STYLE.md](docs/STYLE.md)):** 1. aşama (çekirdek: semboller, işleyiciler, kitaplık, .kstil, derleme) ve 2. aşama (GPU çizimi: kalın/kesikli vuruş, tarama, döşeme, SDF ve atlas işaretleri, iki arka uçta eşit) yapıldı; 3. aşama (stil yöneticisi, sembol tasarımcısı, katman stili penceresi, nesneye sembol), 4. aşama (SVG çizim düzenleyicisi, raster desenler) ve 5. aşama (MPYY sistem kitaplığı: EK-1a, 1c, 1ç, 1d ve MSP gösterimleri, 770 sembol, 79 piktogram) yapıldı; motorda yaklaşık kalan gösterimler STYLE.md §8'de.
   - **Sıradaki (B, semboloji):** sembol ve blok kütüphanesi (belgeye tanım kaydı, `insert` türü, ölçek/açı, patlatma); çizgi tipi kütüphanesi (desenli ve sembollü hatlar); Mekânsal Planlar Yapım Yönetmeliği gösterimleri ve lejant
   - **Sonra (C, D):** yatay/düşey, açı ve yarıçap ölçüsü; adalı ve ilişkisel tarama; nokta hesapları (dik ayak, doğrultu-mesafe, otomatik nokta numarası); kutupsal ve yol boyunca dizi; özellik eşle, yön ters çevir, benzerini seç; imleç yanında dinamik giriş kutusu
2. **Veri modeli:**
   - tipli katman şemaları (alan, tür, alan kümesi)
   - `Float64Array` geometri havuzu, R-tree
   - parsel topolojisi
3. **GIS:**
   - öznitelik tablosu (alt panelde, seçimle eşlenik), sorgu ve filtre, tematik stil
   - raster, WMS ve XYZ altlık (yeni `SceneLayer` türü)
   - CRS dönüşümleri: TUREF ↔ ED50 7 parametre ve grid; TM ve UTM dilimleri
   - **İşlem araçları:** çatı, pencere, araç kutusu ve geçmiş; ifade dili, alan ve ifade parametreleri, tür süzgeci, Web Worker çalıştırıcısı, modeller (çalıştırıcı, kitaplık, akış diyagramı tasarımcısı) yapıldı (köşe numaralandırma, kenar uzunlukları, öznitelik hesapla, ifadeyle seç; Parsel ölçü yazıları modeli). Sıradaki: daha çok araç (sadeleştir, çift nesneleri temizle, parsel numaralandır, alan çizelgesi), modellerin proje dosyasında saklanması ve dışa aktarımı; Rust sunucu çalıştırıcısı ve PostGIS kalıcılığı §13–19 kapsamında.
4. **Harita işleri:**
   - ifraz, tevhid, aplikasyon (istasyondan semt ve mesafe)
   - kot noktası ve TIN, eşyükselti, boy kesit, hacim
   - pafta düzeni ve PDF çıktı
5. **Kalıcılık:**
   - `.kcad` biçimi: JSON manifest (proje ayarları, katmanlar, şemalar) ve ikili geometri parçaları
   - IndexedDB otomatik kayıt
   - ileride §13–19'daki PostgreSQL/PostGIS bulut projesine eşitleme
6. GPU metin (SDF) ve kalın çizgiler (örneklenmiş dörtgen); iki arka uçta birlikte. WebGPU arka ucu yapıldı.
7. **Eklenti API'si:** komut, araç, panel ve IO bağdaştırıcısı katkıları; mevcut kayıtlar bu API'nin ilk kullanıcılarıdır.

---

## 11. Teknik borç ve bilinen kısayollar

- Dosya alışverişi:
  - Koordinat listesinde her rol tek sütundan okunur (aynı rol iki sütuna verilirse yalnız ilki). Boşlukla ayrılmış dosyada boşluk içeren nokta adı okunamaz (sekme ya da `;` kullanın); dışa aktarmada adın boşlukları `_` olur.
  - Kaynak sistem projeninkinden farklıysa içe aktarma kapalıdır (dönüşüm yok).
  - İçe aktarmayla kurulan katmanlar geri alınmaz: nesneler tek adımda geri alınır, katmanlar boş kalır.
  - DXF: KentOS'ta blok nesnesi olmadığı için bloklar patlatılır (blok tanımı kaybolur); başka programların ölçüleri çizgi ve yazıya patlatılır (ölçü olarak düzenlenemez; KentOS'un yazdığı ölçüler ölçü olarak döner). Yazı hizası, genişlik çarpanı, eğiklik ve yazı tipi alınmaz (yazılar sol alttan yerleşir, hizalı yazının yeri yazı genişliği tahminiyle bulunur); çoklu çizgi genişliği ve Z değerleri (nokta kotu dışında) alınmaz. Geçiş noktalı SPLINE, KentOS'un Catmull-Rom eğrisiyle kurulur (noktalardan geçer, arası AutoCAD'inkinden biraz farklı olabilir). Tarama sınırındaki yaylar parçalıdır; kesikli desenler düz çizgi, ikiden çok aileli desenler ilk aileyle yaklaşık alınır. Degrade dolgu düz dolgu olur. Kâğıt uzayı, raster görüntü, 3B katı/yüzey, MLINE ve MLEADER alınmaz. İkili DXF ve DWG okunmaz. Sınırlar: bir dosyadan en çok 1 000 000 nesne, blok açmada en çok 8 milyon adım, MINSERT en çok 10 000 × 10 000, NURBS derecesi en çok 25.
  - Okuma worker'dadır, ama sonucun JSON'dan ayrıştırılması, nesnelerin denetimi ve belgeye eklenmesi sayfanın ana iş parçacığında yapılır; dışa aktarmada nesneler sayfada worker'a kopyalanır. 100 000 nesnede (34,5 MB DXF; bulut konteyneri, başsız Chrome) ana iş parçacığı dışa aktarmada kopya için ~0,3 s bekler (worker yazar, toplam ~2,2 s); içe aktarmada worker ~0,8 s okur, sayfa 24,5 MB JSON'u ~0,87 s'de ayrıştırır, denetleyip ~0,19 s'de ekler, ardından katmanların kurulduğu ilk kare ~1,1 s sürer. Arayüz bu sürelerde bekler; kullanıcının makinesinde ölçülmeli.
  - DXF dışa aktarma: yalnız AutoCAD 2007 ASCII DXF (başka sürüm, ikili DXF ve DWG yok); blok ve kâğıt uzayı yazılmaz (ölçülerin kendi anonim blokları dışında). Ölçüyü başka bir program düzenler ya da yeniden çizerse kendi kurallarıyla çizer (uçlar, yazının yeri ve değerin biçimi biraz değişebilir) ve KentOS onu artık ölçü olarak değil bloğundan patlatarak okur. Yazı tek satırdır, sol alttan yerleşir, yazı tipi Standard'dır (arial). Eğrinin geçiş noktaları da yazılır: başka bir programda geçiş noktaları düzenlenirse eğri o programın yöntemiyle yeniden kurulur (biraz değişir). Tarama deseni kullanıcı tanımlıdır (`_USER`: bir çizgi ailesi ya da çift); taramanın sınırı ilişkisel değildir. `fg` ve `fg-dim` tema renkleri en yakın sabit renge (7, 8), saydamlık düşer; katman stilleri, dolgular, semboller ve etiket biçimleri yazılmaz (etiket metni ve öznitelikler yalnız KentOS verisi olarak gider; başka programlar göstermez). Çizgi tiplerinin desenleri sabittir (çizim ölçeğinde 2,5/1,25 mm kesik gibi), kalınlık AutoCAD'in en yakın kalınlığına iner. Dosya ezdxf 1.4 denetiminden hatasız geçer ve bu depodaki okuyucuya bit bit döner; AutoCAD, Netcad, BricsCAD ve QGIS ile açma denemesi yapılmadı.
- Uygulama kodla üretilen örnek projeyle açılıyor (`model/sampleProject.ts`); Yeni proje boş çizim açar. Son açılan dosyalar listesi yok. Dosya ikili parçasız, tek JSON'dur.
- Yeni projenin yerel orijini dilimin çalışma alanı ortasıdır ve ilk nesnelerle yeniden çapalanmaz: veri buradan uzaksa (ülke içinde en çok ~340 km) çok yakın görünümde ekran kartında santimetre düzeyinde titreme olabilir. Kaynak koordinatlar etkilenmez (float64).
- Pano yalnızca bu sekmede (bellekte) çalışıyor; sekmeler ya da uygulamalar arası kopyalama yok.
- Öznitelikler serbest metin; şema yok.
- Çizgi kalınlıkları ekranda 1 px. `lineWeight` şimdilik yalnızca veri.
- Köşe yuvarlama ve pah iki çizgi arasında ya da bir çoklu çizginin iki düz komşu kenarı arasında çalışıyor; çizgi ile çoklu çizgi, yay ile çizgi arası ve "tüm köşeler" seçeneği yok.
- Uzatma yalnızca çizgi, açık çoklu çizgi ve yayda çalışıyor; budama çoklu çizgide kendi kendini kesmeyi yok sayıyor.
- Birleştirme aynı uçta birden fazla aday varsa ilk bulunanla devam ediyor (dallanan ağlarda sonuç seçim sırasına bağlı).
- Esnet, aynı pencereye giren bir yayı üç tanımlama noktasından yeniden kuruyor; tek ucu taşınan yayın orta noktası yarı yol kadar kayıyor (AutoCAD'deki gibi sehim korunmuyor).
- Yol ötelemesinde, dar iç köşelerde oluşan kendi kendini kesen parçalar temizlenmiyor.
- Dizi dikdörtgen ve kutupsal; yol boyunca dizi yok. Diziler ilişkisel değil (tek tek kopyalar).
- Tarama ilişkisel değil: sınır değişince tarama güncellenmez. Yayları ve daireleri parçalı (72 parça/tur) saklar.
- Alan işlemlerinde parça sınıflandırma artık ikinci dereceden değil: sarım sayıları bantlara bölünmüş kenarlarla sabit, genel bir ışın üzerinden sayılır (`WindingIndex`; ışın bir yay ucundan geçerse tam açı toplamına düşülür), en yakın parça bir hücre ızgarasıyla bulunur. 900 örtüşen karenin birleşimi yaklaşık 0,5 s sürer. İçine tıklayarak alan görünümdeki bütün kenarları bindirir; büyük veride R-tree ile tıklanan yerin çevresine bakılmalı.
- Adalı alanda buda, kır ve dış halka dışında köşe ekle/sil yok (önce Patlat); ötele yalnızca dış halkayı öteler.
- Eğri doğrudan budanamaz, kırılamaz, uzatılamaz, ötelenemez; önce Patlat ile çoklu çizgiye dönüştürülür (kenarları sınır olarak her zaman kullanılır).
- Ölçüler ilişkisel değil (ölçülen nesne değişince güncellenmez); koordinat (ordinat) ve yay uzunluğu ölçüsü yok. Ölçü yazısı ve yazıların seçim kutusu yaklaşık genişlikle hesaplanıyor (gerçek glif ölçüsü yok).
- Tarama deseni her katman yeniden kurulumunda CPU'da üretiliyor; çok sayıda sık taramada GPU tarafına (desen gölgelendiricisi) taşınmalı.
- Pencere seçiminde çokgenlerin sınır kutusu kullanılıyor (tam geometri testi değil).
- Ayar pencereleri her değişiklikte bölümü yeniden çiziyor (odak korunuyor); kısa formlar için yeterli.
- Arayüz bileşenlerinin birim testi yok; arayüz yalnızca duman testiyle (`pnpm e2e`) sınanıyor (şeridin modeli `app/ribbon.test.ts`'te).
- Şerit: klavye harf ipuçları (KeyTips) yok, çünkü tarayıcı Alt tuşunu kendine alır; sekmelere ve düğmelere ok tuşlarıyla, komutlara Alt+Q ile ulaşılır. Seçim sekmesinin içeriği seçilen türlere göre daralmıyor (alan işlemleri her zaman görünür). Hızlı erişim yalnız ekleme ve kaldırmayı biliyor, sıralama yok. Panel genişlikleri bir kez ölçülür; yazı ölçeği ya da yazı tipi değişince yeniden ölçülür, başka içerik değişikliği (ör. uzayan bir etiket) bir sonraki sekme kurulumuna kalır. Bölünmüş düğmenin son seçimi yalnız oturumda hatırlanır; yöntemlerin kendi simgesi yok (düğme aracın simgesini gösterir, yöntem ipucunda yazar); yöntem aracı istemde seçeneğe basılmış gibi başlatır, bu yüzden aracın ilk adımında geçerli bir seçenek olmalıdır.
- Çizim yazı tipi: yazıların seçim kutusu ve hizalı DXF yazısının yeri harf başına ~0,55 em tahminiyle bulunur (Barlow'a göre); geniş yazı tiplerinde (Courier Prime, IBM Plex Mono) kutu yazıdan biraz dar kalır. DXF dışa aktarma yazıları yine Standard (arial) stiliyle yazar; projenin çizim yazı tipi DXF'e taşınmaz.
- Çizim kalitesi: kenar yumuşatma değişince çizim motoru (bağlam) yeniden kurulur; bu bir an sürer. “Ekranda sabit” semboller yakınlaştırmada boylarını korur; çizgi boyunca işaret aralıkları ise yakınlaştırma durunca yeniden hesaplanır.
- Çalışma modları yalnız sunuştur: CAD ve CBS neyi gizlediğini koddaki listeyle bilir (kullanıcı özelleştiremez); 3D Plan ve Afet Analizi yalnız duyurudur (kart, menü ve sözleşme değeri var, arayüzü ve hesapları yok). Bulutta izleyicinin mod değişikliği kendi cihazında kalır (proje bilgisi yetkisi gerekir).
- Geometrinin tek kaynağı Rust çekirdeğidir (`crates/shared/geometry-core`, ADR 0008): seçme, kenet, etiket ve tutamaç kararları, araç önizlemeleri (S1), katman kurulurken çizilen geometri ve dolgu üçgenlemesi (S2), `model/geom`, `model/ops` ve ilkel modüller (S3a–S3b), işlem araçlarının geometrisi (S4), araçların ve nesne izlemenin satır içi hesapları (S5). TS algoritmaları TS ile yan yana derin koşulardan sonra silindi (S3c); cephelerde aritmetik olmadığını `model/singleSource.test.ts` denetler. İfade dili ve stil derleyicisi de çekirdektedir (`crates/shared/style-core`, ADR 0008 “İfade dili”, “Stil derleyicisi”); katman kurma 2–9 kat hızlandı (`apps/web/scripts/perf/style.test.ts`). SVG düzenleyicisinin geometrisi (yol işlemleri, düğümler, kenet, izleme, SVG okuma ve yazma) `crates/shared/svg-core`'dadır (ADR 0008 “SVG düzenleyicisi”); kendi paketi düzenleyiciyle yüklenir. TS'te geometri algoritması kalmadı; `style/svg` de bekçinin denetimindedir. Düzenleyicide şekiller JSON'la geçer: 300 ayrıntılı şekilde taşıma 10 ms, öğelerin kurulması 13 ms (eski TS 1 ve 3 ms); sistem kitaplığının çizimleri küçüktür (ortanca 2, en çok 120 şekil), büyük çizimler sıklaşırsa paketli geçiş gerekir. Taşı, kopyala, döndür, ölçekle, aynala, diziler ve yapıştır dönüşümü geometri deposunda yapar ve yalnız yeni geometriyi paketli alır (JSON'dan geçmez): 10 000 karışık TM nesnesinde dönüşüm p50 294 ms'den 18 ms'ye, taşıma ve kopyalama komutu (geri alma adımıyla) ~310 ms'den 28–35 ms'ye, yapıştırma 26 ms'ye (özgün koordinatlara 50 ms) indi (`apps/web/scripts/perf/transform.test.ts`, Node'da WASM; eski TS yolu ~10 ms'ydi). Belgenin adımı tek değişikliktir (`updateMany`, `addMany`): tarayıcıda bütün dinleyicilerle 10 000 nesnede 145 ms'den 14 ms'ye indi. Yaylı nesnelerin sınır kutusu 72 parçalı ana hatla yaklaşık bulunuyor ve golden setinde yok (ADR 0002). WASM paketi 1 128 KB (gzip 394,9 KB; ifade dili +45,6 KB, stil derleyicisi +45,4 KB); başlangıç WASM'ının boyutu kullanıcı kararıyla bir sınır değildir (2026-09-24: “önemli değil, artabilir”; ADR 0005), yine her dilimde ölçülüp ADR 0008'e yazılır; işlev adları pakette kalır. SVG düzenleyicisinin paketi ayrıdır (861 318 bayt, gzip 315 900) ve yalnız düzenleyici açılınca yüklenir (§20).
- Sağlam geometrik kararlar (§23.3) yolda: kesin yön yüklemi var (`geometry-core::predicates::orient2d`, Shewchuk'un uyarlamalı yüklemi, bağımlılıksız, bağımsız referansla; ADR 0008 “Sağlam kararlar”), ve nokta-halka kararı (`point_in_polygon`: seçme, içerme, DXF tarama adaları), sarım sayıları (açı toplamı ve bindirmenin ışın dizini: alan cebiri, yüzler, içine tıklayarak alan) ve halka izlemede parçaların köşe çevresindeki sırası (düz parçalar kesin; yaylarda halkanın kendi köşe ve açısından teğet ve eğrilik) ona geçti. İki doğrunun kesişim parametreleri (`line_line`, `seg_seg`) doğrular neredeyse paralel olsa da doğrudur: çapraz çarpımlar `predicates::cross_accurate` ile 2⁻⁴¹ içinde alınır (hata sınırı izin verdikçe düz değer, yoksa sıkıştırılmış kesin açılım); 1e-9 bandı ve 1e-12 paralellik sınırı CAD toleransı olarak kaldı, bandın kenarından uzak her karar kesindir. Yay kesişimleri ve ortak sınır kararları henüz f64'te sabit toleranslarla verilir (yayda diskriminant ve `r1 + r2 ± 1e-9`, bindirmede köşe birleştirme `TOL = 1e-6` m). Kararlar yalnız Rust'ta tek tek geçirilir; değişen sonuçlar golden dosyalara bilinçle işlenir.
- §23 sayısal politika (yuvarlama, hisse, artık dağıtımı) yalnızca Rust'ta var. Onaylı resmî politika olmadığı için durumu `draft`; kesin kadastral işlemler kapalı. `ctx.format` yalnızca gösterimdir.
- Bulut (Faz B) sınırları:
  - Tipli öznitelik şeması yok: öznitelikler sunucuda da metin (`properties jsonb`).
  - MVT/tile yayını yok; proje açılışı bütün nesneleri indirir (bbox'a göre kısmi açılış yok).
  - Başarısız giriş sınırı süreç belleğinde ve giriş adına göre (ADR 0007); kurum/üye/koltuk yönetimi yalnız komut satırından.
  - Proje silme yumuşaktır; silinen projeyi geri getirme yalnız komut satırından (`kentosd project restore`), belli bir süre sonra kalıcı silme yok. Silinmiş hâlde açık tutulan proje geri getirilirse editör onu listeden yeniden açmalıdır.
  - Katman görünürlüğü ve açık/kapalı durumu proje verisi (herkes için); etkin katman kişiye özel, eşitlenmez.
  - Çakışma çözümü bütün çakışmalar için tek seçim; nesne bazlı karşılaştırma yok.
  - Canlı olay sinyali tek sunucu sürecinde (çok süreçte 5 sn'lik denetim yakalar; PostgreSQL LISTEN/NOTIFY yok). Olay günlüğü `KENTOS_EVENT_RETENTION_DAYS` gün (varsayılan 7) saklanır ve `kentosd serve` saatte bir budar; komut günlüğü (idempotency) ve denetim kaydı budanmıyor.
  - Yükleme kesilirse proje sunucuda yarım kalır (sürdürülemez).
  - Kurumun OpenID sunucusuyla gerçek deneme yapılmadı (yalnız sahte sağlayıcı).
- ADR 0005 hedefleri taslak. Ağır modüllerin açılışı `apps/web/scripts/perf/modules.mjs` ile ölçülür (üretim derlemesi; bulutta ilk / ikinci açılış stil yöneticisi 220 / 107 ms, model tasarımcısı 125 / 36 ms, SVG düzenleyicisi 163 / 42 ms; hedef 400 / 150 ms). §6.1 etkileşim bütçeleri `pnpm perf:interaction` ile ölçülüyor (taban `docs/perf/interaction-baseline.md`, geometri henüz TypeScript'teyken): 81 bin nesnede imleç başına seçme ve kenet p95 9–13 ms, eşyükseltilerde (1 milyon segment) buda önizlemesi kare başına ~0,75 s, genel görünümde kaydırma GPU'da ~21 fps. Geometri deposundan (S1) sonra aynı bulut makinesinde seçme ve kenet 0,1–3 ms'ye, budama önizlemesi 18 ms'ye indi; eşyükseltilerde genel görünümde kenet ~32 ms (p95) idi; kesişim adaylarının kesin bir alt sınırla elenmesiyle ~7 kat hızlandı (Node'da WASM mikro ölçümü p95 16,4 → 2,4 ms, yanıtlar bit bit aynı; ADR 0008). Kabul ölçümü kullanıcının makinesinde yapılacak (`docs/perf/README.md`).

---

## 12. Dosya haritası

```
apps/web/src/
  main.ts                    Giriş: stiller + createApp
  app/                       Kompozisyon kökü
    createApp.ts             Servisleri kurar, AppContext'i bağlar, kabuğu takar
    context.ts               AppContext arayüzü
    commands.ts              Çekirdek komutlar, tema ve yazı ölçeği uygulama
    keybindings.ts           Varsayılan kısayollar
    menus.ts                 Ana menü modeli (klasik menü ve şeridin tek kaynağı: başlıklı bloklar, `@tools:` araç referansları), komuttan menü öğesi çözümü
    ribbon.ts                Şerit modeli: sekmeler menülerden ve araç gruplarından türetilir (`RIBBON_TABS`, `ribbonTabs`), anlama göre düğme boyu, bölünmüş düğmeler, panel ▾ listesi, hızlı erişim varsayılanları
    appearance.ts            Görünüm seçimleri: vurgu renkleri (`ACCENTS`, `styles/accents.css`), arayüz yazı tipleri (`UI_FONTS`) ve projenin çizim yazı tipleri (`DRAWING_FONTS`; `styles/fonts.css`), uygulanmaları
    workspaces.ts            Çalışma modları: kayıt (Hibrit, CAD, CBS; yakında 3D Plan, Afet Analizi), modun süzgeci (`workspaceFilter`, `filterOf`)
    state.ts                 DraftingSettings, MessageLog, UiState, Preferences (localStorage)
    clipboard.ts             Clipboard: kopyalanan nesneler ve taban noktası (oturumluk)
    fileIO.ts                DocumentFiles: yerel .kcad kaydet/farklı kaydet/aç, içe aktarılacak dosyanın seçimi, dosya seçici (tarayıcı ya da test için bellek içi)
    fileExchange.ts          Dosya alışverişi komutları (koordinat listesi ve DXF içe/dışa aktar); pencereleri ve biçim modülünü ilk kullanımda yükler
    server.ts                ServerStatus: API sağlık denetimi (boşta, odakta, ağ dönünce, istekle), yanıtın sözleşmeye göre okunması
    cloud/                   Bulut: api (istemci), session (ctx.cloud: oturum, aç, yükle), sync + syncCore + syncRemote + syncRestore (otomatik kayıt, olaylar, taslak), tracker (fark), socket (WebSocket), drafts (IndexedDB), incoming (gelen veriyi denetleme), commands, fakeServer (testler için)
    format.ts                Formatter: sayıdan metne tek geçit
    processing.ts            ProcessingService: işlem kaydı, çalıştırıcı, son değerler; işlem komutları
    styles.ts                StyleService: stil kitaplığı (sistem + kullanıcı localStorage + proje); stil komutları, nesneye sembol verme
  contracts/                 Sürümlü sözleşmeler: generated/ (ts-rs çıktısı, elle düzenlenmez), version.ts, contracts.test.ts (derleme anında uyum)
  io/                        Dosya alışverişi (başlangıçta yüklenmez): formatsWorker.ts (Rust biçim modülünü çalıştıran worker), client.ts (istek/yanıt, boşta kapanma, tuzakta yenileme), protocol.ts, apply.ts (okunanı tek geri alma adımıyla belgeye koyma), coords.ts (sütun sıraları, biçimler, noktalar), version.ts; pkg/ `pnpm wasm` ile üretilir, depoya girmez (+ testler)
  core/                      Bağımsız temel yapılar (signal, emitter, disposable, commands, keymap)
  geo/crs.ts                 EPSG kaydı (TUREF/ED50 TM, UTM, WGS84), arama, dilim önerisi, yeni projenin çalışma alanı ortası
  geo/crsFixture.ts          Kaydın Rust ile paylaşılan sürümlü dosyası (fixtures/crs/v1/registry.json) ve dilim önerisi örnekleri
  model/                     Belge, varlıklar, geometri, katmanlar, seçim, proje ayarları, örnek proje, standart katman ağacı (standardLayers), boş yeni proje (newProject), tek hesap kaynağı bekçisi (singleSource.test.ts)
    expression/              İfade dilinin cephesi (dil `crates/shared/style-core`'da): expression.ts (derleme sonucu, okunanların tablosu, sütunun okunması, önizleme), expressionLib.ts (değer tipi, deponun ölçü düzeni, işlev ve değişken kataloğu); cases.ts (testlerin tohumlu kaynak ve nesne üreteci); işlem araçları ve stil motoru kullanır
    geom/                    Geometri çekirdeğinin cepheleri (Rust'ta): afin, yay, bulge, kesişim, elips, eğri; öteleme, teğet daire, düzlem bindirme ve alan cebiri, şekiller, ölçmecilik, tarama, ölçü (+ testler)
    ops/                     Nesne işlemleri, Rust cepheleri (entityOp: nesne döndürenlerin sarmalayıcısı): kenarlar, yol parametresi, dönüşüm (toplu dahil), budama/uzatma, kır, birleştir, patlat, esnet, köşe, öteleme, köşe yuvarlama/pah, tutamaçlar (+ testler)
  style/                     Stil motoru (hesabı `crates/shared/style-core`'da): compile (tek sembolün çekirdek cephesi: önizleme ve lejant), primitives (çizim ilkellerinin tipleri), geometry (geometri sınıfı, depodan çizilen geometrinin okuyucusu), fromLayer (basit görünüş ve tarama sembolleri), library, file (.kstil); cases.ts ve fixture.ts testlerin üreteci ve fixture yardımcıları (+ testler); türler model/style.ts'de
    classify.ts              Katman stili sınıflama (benzersiz değer, eşit aralık/sayı, rampalar)
    legend.ts                Lejant satırları (katman, sınıf, nesne sembolleri)
    showcase.ts              Gösterim kataloğu: her sistem sembolü örnek geometride (demo projede paftanın altı)
    system/                  Sistem kitaplığı (salt okunur, kopyalanabilir): temel çizgi tipleri, işaretler, alanlar; mpyy/ (MPYY gösterimleri: dsl.ts yardımcılar, pictograms.ts + pictogramDrawings.ts piktogramlar, uip/ nip/ cdp/ msp/ ortak/ kademe bölümleri)
    svg/                     SVG düzenleyicisinin modeli; hesabı `crates/shared/svg-core`'da, dosyalar onun cepheleridir (core.ts: paketi düzenleyici açılınca başlatır, `svgOp(ad)`; pkg/ `pnpm wasm` ile üretilir, depoya girmez; testSetup.ts; cases/: testlerin üreteçleri (geometry, files) ve `CALLS` (calls); + testler, fixture.test.ts). Temel: svgModel, pathData. Dosya: svgValues (renk, dönüşüm, uzunluk, CSS), importSvg (içe alma: stil, <use>, birimler, renk eşleme, özet), exportSvg (sembol/düz SVG, kaynak görünümü, PNG boyu ve DPI), trace (bitmap izleme). Düzenleme: bezier (parça, bölme, düzleştirme, uzunluk), fitCurve (Schneider uydurma), pathBool (kirişi izlenen düzleştirme, dolgu kuralları, bindirme, eğrilerin geri kurulması, yolu kes), pathStroke (çizgi dış hattı, öteleme), pathOps (şekil düzeyinde yol işlemleri), nodeOps (düğüm işlemleri, köşe yuvarla/pah), arrange (hizala, dağıt, dönüştür, diziler, sıra), snapping (kenet dizini)
  processing/                İşlem araçları: types (sözleşme), parameters, features (kapsamlar), categories, registry, runner, job (RunJob, Executor, runJob), geometry (ObjectStore: çalıştırmanın geometri deposu), model, modelRunner, modelEdit (+ testler)
    worker/                  Web Worker çalıştırıcısı: protokol, iş yürütme, executor, worker girişi (+ testler)
    builtin/                 Yerleşik araçlar: köşe numaralandırma (numbering: numara biçimi ve adlar, hesabı çekirdekte; vertexNumbering), kenar uzunlukları, öznitelik hesapla, ifadeyle seç; yerleşik modeller
  render/                    RenderBackend sözleşmesi, sahne kurucu, delikli üçgenleme, ızgara, renk, çizim kalitesi (`quality.ts`: kenar yumuşatma ve piksel oranı); webgl2/ ve webgpu/
    styledLayer.ts           Belge katmanı → katmanın programı (semboller, işleyici, kümeler, renkler), nesne başına kip ve ifade tablosu → çekirdekte tek çağrı (`PickIndex.styled`)
    styledBatches.ts         Çekirdeğin topluluklarını GPU topluluklarına çevirir: tema renkleri, atlas görüntüleri (yazı, SVG, raster, desen döşemesi), taşma payı
    fillQueue.ts             Katmanın dolgularını toplar, katman bitince çekirdekte tek çağrıda üçgenler (`triangulateMany`)
    atlas.ts                 Doku atlası: SVG, raster, yazı işaretleri ve desen döşemeleri (iki arka uç ortak)
    canvasShapes.ts          İşaret şekillerinin Canvas2D çizimi (atlas döşemeleri ve önizlemeler)
    symbolPreview.ts         Sembollerin Canvas2D önizlemesi (aynı çizim ilkelleri): kitaplık resimleri, tasarımcı, lejant
    webgl2/styled*.ts        Stilli toplulukların GLSL gölgelendiricileri ve çizicisi
    webgpu/styled*.ts        Aynısının WGSL karşılığı
  viewport/                  Kamera, ViewportController, PickIndex (geometri deposunun yüzü ve eşitlemesi), storeRecords (depodan gelen etiket ve tutamaç kayıtları, varsayılan etiketler), üst katman çizimi
  tools/                     Tool sözleşmesi (gruplar ve bölümler: `TOOL_SECTIONS`), ToolManager, katalog, bölümler (`sections.ts`), koordinat girişi, imleç kısıtlaması (tracking)
    constructions.ts         Araçların yapı hesapları: Rust çekirdeğinin (`geometry-core::tools`) tipli çağırıcıları
    drawTools.ts             PointInputTool ailesi: çizgi, nokta, sil
    pathTool.ts              Çoklu çizgi, kapalı alan, parsel, ölçüm (yay seçenekleriyle)
    markupTools.ts           Halka, revizyon bulutu
    curveTools.ts            Yay (tüm AutoCAD yöntemleri), daire (merkez-yarıçap/çap, 2N, 3N, TTY), eğri
    shapeTools.ts            Dikdörtgen (seçenekleriyle), döndürülmüş dikdörtgen, düzgün çokgen
    parallelTool.ts          Paralel çizgi (eksen + sol/sağ yanlar ya da koridor alanı)
    perpTools.ts             Dik in, dik çık (referans hatta göre)
    ellipseTool.ts           Elips ve eliptik yay (eksenden, merkezden, döndürmeyle)
    constructionTools.ts     Yardımcı çizgi (nokta, yatay, düşey, açı, açıortay) ve ışın
    annotateTools.ts         Yazı
    dimensionTool.ts         Ölçü: hizalı, doğrusal ΔY/ΔX, açı, yarıçap, çap
    hatchTool.ts             Tarama (kapalı nesne ya da çizgilerle sınır, ada algılama)
    visibleFaces.ts          Görünür çizgilerin kapattığı yüzler (önbellekli; tarama ve içine tıklayarak alan)
    modifyTools.ts           SelectionFirstTool ailesi: taşı, kopyala, döndür, ölçekle, aynala, dizi
    arrangeTools.ts          Kutupsal dizi, hizala
    lengthenTool.ts          Uzat-kısalt
    edgeTools.ts             EdgePickTool tabanı: ötele, buda, uzat
    cornerTools.ts           CornerTool: köşe yuvarla, pah (köşeye tıkla, imleçle boyut göster)
    pathEditTools.ts         Kır, böl, köşe ekle/sil
    editTools.ts             Birleştir, patlat, esnet, yapıştır
    areaTools.ts             Alan işlemleri: içine tıklayarak alan, alana çevir, alan birleştir/kesiştir/çıkar/böl, çizgiye çevir
    targetLayer.ts           Yeni nesnelerin yazılacağı katman (kilitli/gizli uyarıları)
    pickPointTool.ts         Başkası için tek nokta ister (işlem aracı nokta parametresi)
    SelectTool.ts            Seçim (tutamaçla düzenleme dahil), kaydırma, pencere yakınlaştırma
  ui/
    shell/AppShell.ts        Yerleşim ve bölgeler
    shell/InlineTextEditor.ts  Yazı ve ölçü için yerinde düzenleyici
    shell/CommandBar.ts      Komut şeridi: çalışan komutun adımı, seçenek düğmeleri, tek seferlik kenet, fare hatırlatması
    shell/CursorInput.ts     İmleç yanında değer girişi (dinamik giriş)
    shell/HoverCard.ts       Üzerine gelinen nesnenin bilgi kartı
    shell/viewportMenus.ts   Çizim alanındaki sağ tuş menüleri: boşta, komut, kenet, tutamaç
    shell/fullscreenButton.ts  Menü çubuğunun ve şeridin sağındaki Tam ekran düğmesi (`view.fullscreen`)
    shell/brandButton.ts     KentOS logosu (her zaman lacivert karo, `--c-brand*`) ve yazısı; tıklayınca uygulama menüsünü yükleyip açar
    appmenu/                 Uygulama menüsü (ilk kullanımda yüklenir, `styles/appmenu.css` ile): dosya komutları, içe/dışa aktarma biçimleri, bulut (hesap, açık proje, son projeler), çizimin özeti
    promptOptions.ts         İstem ayrıştırma ve seçenek düğmeleri (komut şeridi ve komut satırı ortak)
    menu/ toolbar/ toolbox/  Menü çubuğu, araç çubuğu (`fields.ts`: şeritle ortak geçerli özellik alanları), kayan araç kutusu
    ribbon/                  Şerit (ayrı parça, seçilince yüklenir): Ribbon (sekmeler, sığdırma, daraltma ve açılır şerit, Seçim sekmesi, hızlı erişim), panels (düzeyler, alanlı paneller), controls (düğmeler), search (Komut ara)
    dock/ layers/ properties/  Sağ dok (Katmanlar/İşlemler sekmeleri), katman ağacı, öznitelik paneli
    processing/              İşlem aracı penceresi (ToolDialog, modeller dahil), parametre kontrolleri, araç kutusu ve geçmiş paneli
      model/                 Model tasarımcısı: ModelDesigner, ModelCanvas, modelPalette, modelInspector
    bottom/                  Komut satırı ve alt panel (geçmiş, koordinat listesi, uyarılar)
    statusbar/               Durum çubuğu
    settings/                SettingsShell, crsPicker, workspacePicker (çalışma modu kartları), Proje ve Uygulama ayarları pencereleri, Yeni proje penceresi
    style/                   Stil yöneticisi, sembol tasarımcısı (katman formları, alanlar), katman stili (kurallar, sembol yuvası), resimler, .kstil dosyaları
    svgedit/                 SVG çizim düzenleyicisi: SvgEditor (pencere). Dosya: svgFile (Dosya menüsü, aç/ekle, pano ve sürükle-bırak, kitaplıktan aç, farklı kaydet), svgImport, svgExport, svgDocProps, svgReference (izleme altlığı), svgTrace (bitmap izle), svgSource (XML kaynağı), readSvg. Düzenleme: svgView (ortak türler), svgCanvas (görünüm, seçim, çizim araçları), svgNodeTool, svgSnap, svgRulers (cetvel, kılavuz), svgMeasure, svgActions (menü/panel/tuş işlemleri, panel ayarları), svgMenus (Yol, Nesne, Seç, Kenet, Cetvel), svgProps (sekmeler, Özellikler), svgStyleProps (çizgi biçimi, kutu), svgNodeProps, svgAlign, svgTransform, svgArray, svgObjects (şekil listesi), svgIcons
    widgets/                 Genel parçalar (menü, açılır liste, ağaç, özellik ızgarası, pencere, onay penceresi `confirm.ts`, kontroller)
    cloud/                   Buluta giriş, bulut projeleri (aç, yükle), yeniden adlandırma ve silme (ProjectActions), kayıt çakışması pencereleri
    io/                      Dosya alışverişi pencereleri: CoordImportDialog, CoordExportDialog, DxfImportDialog, DxfExportDialog; common (dosya satırı, alanlar, özet satırları, CrsQuestion), scope (dışa aktarma kapsamı), save (dışa aktarılanı yazma), zoom
    dialogs.ts               Kısayol listesi ve Hakkında
    icons.ts                 Simge seti
  assets/fonts/              Uygulamayla gelen arayüz yazı tipleri (woff2, Latin ve Latin Extended; her klasörde OFL lisansı): arayüz için Plus Jakarta Sans, Inter, IBM Plex Sans, Source Sans 3, Noto Sans, Roboto; çizim yazıları için Barlow, Arimo, Overpass, Quicksand, Architects Daughter, Courier Prime; değerler ve çizim için IBM Plex Mono
  styles/                    fonts (@font-face, yerel dosyalar), tokens, accents (vurgu renkleri), base, shell, controls, panels, settings, processing, model, style, svgedit (SVG düzenleyicisinin düzenleme araçları); io.css (dosya alışverişi pencereleri, onlarla birlikte yüklenir); ribbon.css (şeritle birlikte yüklenir)
  wasm/                      Rust çekirdeğinin tarayıcı cephesi: core.ts (başlatma, op() çağrıları, NaN/±∞ geri çevirme, hata bildirimi, tipli toplu girişler: `triangulateMany`, `offsetPathXY`, `hatchLinesXY`, `cornerTexts`; sayı alan küçük ölçüler (`coreDist` …) ve kazıma tamponlu halka ölçüleri (`ringCentroid` …); durum tutan sınıflar: geometri deposu `CoreStore`, yüz dizini `CoreFaceIndex`), pack.ts (nesneleri depo için sayı akışına paketleme; deponun aynı düzendeki yanıtını okuma: `unpackEntities`), testSetup.ts (Vitest), calls/ (çağrı kümeleri ve tohumlu üreteç, depo sahnesi, toleranslı karşılaştırma, depo fixture'larının ortak okuyucusu `storeCases.ts`, bağımsız referans testi; S3c'ye kadar TS ↔ Rust parity ve eski TS referansları buradaydı), golden, çağrı ve depo fixture testleri; pkg/ `pnpm wasm` ile üretilir, depoya girmez
crates/                      Rust kütüphaneleri, rolüne göre (Cargo çalışma alanı, kök Cargo.toml)
  shared/                    Platformdan bağımsız: web (WASM ile), sunucu ve ileride masaüstü (wgpu) uygulaması aynen kullanır; DOM, tarayıcı, veritabanı ve ağ bilmez
    geometry-core/           Saf analitik geometri (f64), jsmath (JavaScript sayı anlamı, libm), predicates (kesin yön yüklemi, §23.3), api (çağrı tablosu, JSON yazıcı), store (geometri deposu: R-ağacı, seçme, kenet, paketli okuyucu ve yazıcı, paketli dönüşüm, işlem sorguları), processing (köşe numaralama, kenar ölçüleri), tools (nokta girişi, nesne izleme, araçların yapı hesapları), triangulate ve §23 sayısal politika (rust_decimal); clippy.toml (std aşkın işlevleri yasak); tests/ (golden, çağrı fixture'ları, depo, bağımsız referans, sayısal)
    contracts/               Sürümlü sözleşmeler (Entity, katman, ayarlar, .kcad, .kstil, RunJob, Health, komut zarfı, §23 sayısal) → TS tipleri; tests/ (.kcad ve CRS dosyaları); TS üretimi `ts` özelliğinde (varsayılan açık); yalnız Rust tipleri isteyenler `default-features = false` ile bağlanır
    style-core/              Stil motorunun çekirdeği (native ve WASM; `kentos-geometry-core`'a bağlı): expr/ (ifade dili: lexer sözcükler ve UTF-16 konumlar, parser öncelik tırmanması, library işlevler ve değişkenler, value değer kuralları, rows toplu değerlendirme tablosu ve sütunu), js/ (dilin dayandığı JavaScript anlamı: number `String`/`toPrecision`/`toFixed`, text UTF-16 ve Türkçe harf, collate + collation_tr Türkçe sıralama tablosu), style/ (stil derleyicisi: model semboller, katmanlar ve işleyiciler JSON'dan, place işaret yerleri, gruplar, dalgalar ve iç nokta, compile birimler ve katmanlar, resolve işleyiciler ve ölçek aralıkları, prim çizim ilkelleri ve JSON yazıcısı, batch GPU toplulukları ve son stillerin önbelleği, build katmanın tek çağrısı ve tek sembol önizlemesi), api (`exprCompile`, `exprCatalog`, `styleCompile`); clippy.toml (std aşkın işlevleri yasak); tests/ (dondurulmuş ifade, sembol ve katman yanıtları)
    svg-core/                SVG düzenleyicisinin çekirdeği (native ve WASM; `kentos-geometry-core` ve `kentos-style-core`'a bağlı; TS dosyası başına bir modül): path (yol verisi, yay → kübik, matris), bezier, fit (Schneider uydurma), boolean (yol cebri: kirişi izlenen düzleştirme, dolgu kuralları, çekirdeğin bindirmesi, eğrilerin geri kurulması, yolu kes), stroke (çizgi dış hattı, öteleme), ops (şekil düzeyinde yol işlemleri), shape ve model (şekiller alan sırasıyla, öğe ve dosya, kutular, dönüşümler), nodes (düğüm işlemleri), snap (kenet dizini), arrange (hizala, dağıt, diziler), trace (bitmap izleme), values/ (kavram başına dosya: number JavaScript'in sayı okuması, color renk ve boya, units dönüşüm, uzunluk ve viewBox, xml dosyanın öğe listesi, css stil sayfaları), import (SVG okuma: stiller, `<use>`, birimler, renk eşleme), export (SVG yazma, kaynak görünümü, PNG boyu ve DPI), api (çağrı tablosu); clippy.toml (std aşkın işlevleri yasak); tests/ (dondurulmuş yanıtlar)
    formats/                 Dosya biçimleri (saf; native ve WASM): coords (NCN/TXT/CSV okuma ve yazma), dxf/ (lexer: grup çiftleri; strings: kod sayfası, \U+, %%, MTEXT; entity: grupları modele okuma; hatch: sınır yolları ve desen; emit: dönüşümlerle uygulamanın nesnelerine; aci: renk dizini ve uygulama renginin DXF rengi; xdata: KentOS genişletilmiş verisi; writer/: yazıcı (mod: başlık ve bölümler, template: AutoCAD'in istediği sabit tablolar ve nesneler, layers: katman adları, çizgi tipleri, kalınlıklar, entities: nesneler, input: yazıcı girdisinin okuyucusu)), geom (dörtte bir dönüşleri tam afin dönüşüm, nesne koordinat sistemi, elips kurulumu, DXF tarama yayları; yaylı halka, alan ve içerme ortak çekirdekten: `kentos-geometry-core`), nurbs (de Boor, toleranslı örnekleme), math (libm, dörtte bir dönüşlerde tam sin/cos), text (UTF-8/16, Windows-1254/1252), num (kesin sayı okuma ve yazma), report; clippy.toml (std aşkın işlevleri yasak); tests/ (fixture dosyaları)
  wasm/                      Tarayıcı bağlayıcıları (wasm-bindgen); hesap yapmaz, ortak kütüphaneleri dışa açar
    geometry-wasm/           Çekirdeğin tarayıcı sınırı (wasm-bindgen): çağrı tablosu (opId/callOp, JSON; geometry-core ve style-core), düz Float64Array ve sayı girişleri, ifadenin toplu değerlendirmesi (`exprEvaluate` → `ExprColumn`), halka ölçüleri için kazıma tamponu, geometri deposu (store.rs; stilli katmanın tek çağrısı `buildStyled`, `StyleProgram`, `StyledBatches`) ve yüz dizini (faces.rs) → `apps/web/src/wasm/pkg`
    formats-wasm/            Biçimlerin tarayıcı sınırı (wasm-bindgen): readCoords, writeCoords, readDxf, writeDxf; dosya bayt, seçenek ve sonuç JSON → `apps/web/src/io/pkg`
    svg-wasm/                SVG düzenleyicisinin tarayıcı sınırı (wasm-bindgen): çağrı tablosu (opId/callOp, JSON), kenet dizini sınıfı (`SnapIndex`), resmin baytlarıyla izleme (`inkMask`, `traceContours`, `traceBitmap`), PNG baytları (`crc32`, `withPngDpi`) → `apps/web/src/style/svg/pkg`
  server/                    Yalnız sunucu: PostgreSQL/PostGIS ve kullanım durumları
    postgres/                Havuzlar, tenant kapsamlı işlem (`Db::scoped`), migration'lar (`migrations/`; `build.rs` yeni dosyada yeniden derletir), `db-setup`, geçici test veritabanları (`testing`), `.env.local` okuma
    application/             Kullanım durumları: identity (yerel giriş, oturum), tenancy (rol, yetki, erişim), admin (komut satırı; silinen projeyi geri getirme dahil), cad (Entity ↔ PostGIS satırı), projects, lifecycle (proje silme), changes (`project.changes`), events (olay günlüğü, budama ve ufuk); tests/ (geçici veritabanıyla; common/ ortak yardımcılar)
apps/                        Çalıştırılabilir uygulamalar
  api/                       kentosd: `serve` (Axum; http/ auth, projects, ws, error; oidc.rs; hub.rs; saatlik olay budaması), `db-setup`, `migrate`, yönetim komutları (cli.rs), config.rs
  web/                       Tarayıcı uygulaması (pnpm paketi `@kentos/web`): src/ (yukarıdaki ağaç), index.html, public/, vite.config.mjs (/v1 isteklerini yerel API'ye ileten eklenti, dev ve preview), tsconfig.json, package.json
    scripts/e2e/             Başsız Chrome duman testi (cdp.mjs sürücü, smoke.mjs senaryo) ve bulut senaryosu (cloud.mjs)
    scripts/perf/            Build envanteri (bundle.mjs), başlangıç ölçümü (startup.mjs) ve etkileşim ölçümü (interaction.mjs: parsel-50k, hat-1m) → kökteki docs/perf/; taşı/kopyala/yapıştır ölçümü (transform.test.ts, `TRANSFORM_BENCH=1`), ifade ölçümü (expression.test.ts, `EXPRESSION_BENCH=1`), ağır modüllerin açılışı (modules.mjs)
    scripts/fixtures/        Uygulamanın yolundan fixture kaydedicileri (GOLDEN_WRITE=1; record-calls: çağrı kümeleri; record-expression: ifade yanıtları; record-collation: Türkçe sıralama tablosu)
    scripts/showcase/        Gösterim kataloğunun ekran görüntüleri
  desktop/                   (ileride) wgpu masaüstü CAD/CBS uygulaması; crates/shared kütüphanelerini doğrudan (native) kullanır
fixtures/                    İki dilin paylaştığı sürümlü dosyalar: geometry/v1 (golden, bağımsız referans), expression/v1 (ifade dilinin yanıtları), style/v1 (stil derleyicisinin sembol ve katman yanıtları), svg/v1 (SVG düzenleyicisinin çekirdeğinin yanıtları), numeric/v1, document/v1 (.kcad örneği), crs/v1 (CRS kaydı), formats/v1 (koordinat listeleri: NCN, Windows-1254 CSV, bozuk satırlar, UTF-16; DXF: her tür, aynalı nesne koordinat sistemi, iç içe ve dizi bloklar, adalı taramalar, ölçü)
Cargo.toml, rust-toolchain.toml, .cargo/config.toml   Rust çalışma alanı (üyeler crates/* ve apps/api), sabit araç zinciri, 4 işlik derleme sınırı
package.json, pnpm-workspace.yaml   pnpm çalışma alanı (apps/web); kökteki betikler (pnpm dev, test, build, e2e, rust:*, api, db:setup) web komutlarını apps/web'e iletir
scripts/wasm/ensure.mjs      WASM paketlerini (geometri ve stil çekirdeği, dosya biçimleri) kaynak özeti değiştiyse derler (dev/test/build/e2e öncesi)
scripts/fixtures/            Bağımsız referans üreticileri (Python decimal/fractions; kökten çalıştırılır)
docs/adr/                    Mimari kararlar (0001 çalışma alanı, 0002 sözleşme ve fixture, 0003 işlem anlamı, 0004 sayısal politika, 0005 performans hedefleri (taslak), 0006 veri katmanı, 0007 kimlik doğrulama, 0008 ortak çekirdek sınırı, 0009 dosya biçimleri)
docs/perf/                   Ölçüm raporları ve özet (README.md)
docs/PROCESSING.md           İşlem araçları mimarisi, parametre türleri, çalışma yerleri, modeller, tarif
docs/STYLE.md                Stil motoru: MPYY araştırması, sembol katmanları, birimler, işleyiciler, kitaplık, çizim hattı, aşamalar
```

---

## 13. Rust backend hedefi: PostgreSQL + PostGIS

Bu bölüm, `c7450a748c3b0d673ba2676490fedfc339b7cf2a` commit'indeki gerçek
koda bakılarak 23 Eylül 2026'da güncellendi. Bu commit bir Vite/TypeScript
tarayıcı uygulamasıdır; Rust crate'i, Cargo workspace, HTTP sunucusu,
kalıcı proje kaydı ve çoklu tenant henüz yoktur. `apps/web/src/processing/worker/`
bir tarayıcı Web Worker'ıdır; sunucu job worker'ı değildir. WebGL2/WebGPU
arka uçları, stil motoru ve `SceneLayer` sözleşmesi mevcuttur.

> **Doğrulanmış durum (2026-09-23, `e916874` sonrası):** Faz A dilimleri depodadır:
> - Cargo çalışma alanı: `crates/shared/contracts`, `crates/shared/geometry-core`, `crates/wasm/geometry-wasm`, `apps/api`.
> - Yalnızca `GET /v1/health` veren API.
> - Native ve WASM'da aynı golden ve bağımsız referans dosyalarıyla sınanan geometri alt kümesi.
> - §23 sayısal politika çekirdeği, sürümlü sözleşmeler.
> - Yerel `.kcad` kaydet/aç.
>
> PostgreSQL/PostGIS, kimlik, tenant, sunucu worker'ı, MVT/Martin ve bulut kaydı yoktur. Ayrıntı: §11–12, `docs/adr/`, `docs/perf/`.
>
> **Doğrulanmış durum (2026-09-24, `e503cd8` sonrası):** Faz B'nin ilk dikey dilimi depodadır:
> - PostgreSQL/PostGIS (`kentos_cad`), satır güvenliği, iki rol.
> - Yerel hesap ve OpenID ile giriş; kurum, üyelik ve koltuk.
> - Kayıpsız nesne saklama; `project.changes` (sürüm, 409, idempotency, audit, outbox).
> - WebSocket olayları; tarayıcıda bulut projesi açma/yükleme, otomatik kayıt, cihaz taslağı ve çakışma çözümü.
>
> Tipli öznitelik şeması, MVT/Martin, sunucu worker'ı ve arayüzden kurum yönetimi yoktur. Kurumun OpenID sunucusuyla gerçek deneme yapılmadı (§11).
>
> **Doğrulanmış durum (2026-09-24, `65ed097` sonrası):** §14'ün ortak çekirdeği istemcide tamamlandı:
> - Uygulamanın geometrisi `crates/shared/geometry-core`'dan gelir (native ve WASM aynı fixture'larla): geometri deposu (seçme, kenet, etiketler, tutamaçlar, araç önizlemeleri, çizilen geometri), `model/geom`, `model/ops`, işlem araçları ve araçların hesapları. Sunucu aynı çekirdeği PostGIS geometrisini üretmek için kullanır (tessellate, EWKB).
> - TS algoritmaları, TS ile yan yana derin koşulardan (işlem başına 20 000 rastgele durum) sonra silindi; cephelerde aritmetik olmadığını bir test denetler.
>
> Komutların sunucuda çekirdekle yeniden doğrulanması (§14, §18), stil motorunun geometrisi (style-core) ve §23.3 sağlam kararlar yoktur. Etkileşim ölçümü bulutta yazılım GPU'suyla yapıldı; kullanıcının makinesindeki kabul ölçümü bekliyor (§11, `docs/perf/`).
>
> **Doğrulanmış durum (2026-09-24, `0ebd723` sonrası):** depo kullanıcı kararıyla monorepo düzenine geçti (ADR 0001 “Güncelleme”):
> - Tarayıcı uygulaması `apps/web/` (pnpm çalışma alanı paketi `@kentos/web`); komutlar kökten çalışır.
> - Rust crate'leri `crates/shared/` (geometry-core, contracts, formats: platformdan bağımsız), `crates/wasm/` (geometry-wasm, formats-wasm) ve `crates/server/` (postgres, application) altında; `contracts`'ın TS üretimi `ts` özelliğinde.
> - Hesap, sözleşme ve arayüz davranışı değişmedi; `apps/desktop` (wgpu) henüz yoktur.
>
> **Doğrulanmış durum (2026-09-24, style-core Y1–Y3 sonrası):** ifade dili ve stil derleyicisi `crates/shared/style-core`'dadır (native ve WASM aynı fixture'larla):
> - İfade dilinin derlenmesi ve toplu değerlendirilmesi; stil motorunun hesabı (işaret yerleşimi, sembollerin derlenmesi, işleyiciler, GPU toplulukları). Bir katman geometri deposunun yanında tek çağrıda kurulur; sayfada renkler, atlas görüntüleri ve yükleme kalır.
> - TS algoritmaları, TS ile yan yana derin koşulardan (20 000 ifade kaynağı; 20 000 sembol ve 20 000 katman) sonra silindi.
>
> SVG düzenleyicisinin geometrisi (sıradaki dilim) ve sunucunun stil yayını (§17) yoktur.
>
> **Doğrulanmış durum (2026-09-24, style-core Y4 sonrası):** SVG düzenleyicisinin geometrisi `crates/shared/svg-core`'dadır (native ve WASM aynı fixture'larla):
> - Yol işlemleri, düğümler, kenet, hizalama ve diziler, bitmap izleme, SVG dosyasının okunması ve yazılması. Kendi WASM paketi düzenleyici açılınca yüklenir; başlangıç paketi değişmedi.
> - TS algoritmaları, TS ile yan yana derin koşudan (işlem başına 20 000 rastgele durum, bit bit; alan sırası JSON metniyle, yol düğümleri ve alt yollar dışında aynı: onları çekirdek tek sırayla yazar) sonra silindi.
>
> Sunucunun stil yayını (§17) yoktur.

**Kesin karar:** Ana kalıcı veri deposu PostgreSQL + PostGIS. Bu proje için
ayrı bir disk motoru, WAL, MVCC, uzamsal indeks veya dağıtık veritabanı
protokolü yazmayın. Rust; CAD/geometri hesapları, sunucu iş kuralları,
PostGIS sorgu planları, tile yayınlama, stil sözleşmeleri ve ağır işler
üzerinde çalışır. PostgreSQL'ü yalnızca veri satırı değil; transaction,
yetki sınırı, veri bütünlüğü, kalıcı job/outbox ve yedekleme temeli olarak
kullanın. Bir CAD ihtiyacı PostGIS'te ölçülmüş ve tekrarlanabilir biçimde
karşılanamıyorsa **dar kapsamlı PostgreSQL uzantısını** sonradan değerlendirin;
baştan özel veri tipi veya uzantı yazmak bir hedef değildir.

Bu karar mevcut 2D CAD davranışını, analitik eğrileri, MPYY stillerini,
WebGPU'yu veya native/WASM ortak Rust çekirdeği hedefini kaldırmaz.
PostGIS `CircularString`, `CompoundCurve` ve `CurvePolygon` saklayabilir;
her CAD kavramının bu tiplere kayıpsız eşleneceğini varsaymayın.

### 13.1 Mevcut kodla entegrasyon

| Mevcut kod                                             | Korunacak davranış                                          | Sunucuya geçiş                                                                                     |
| ------------------------------------------------------ | ----------------------------------------------------------- | -------------------------------------------------------------------------------------------------- |
| `apps/web/src/model/document.ts`                                | Yerel hızlı edit ve 200 adımlı undo                         | Sunucu kimliği, satır sürümü, beklenen sürümle commit, uzak undo sözleşmesi                        |
| `apps/web/src/model/entities.ts`                                | f64 koordinatlar; bulge, delik, elips, spline, ölçü, tarama | Kaynak CAD tanımını sürümle; PostGIS geometri projeksiyonu ile tipli GIS alanları bağla            |
| `apps/web/src/core/commands.ts`                                 | Menü/kısayol/komut satırı ortak giriş                       | Kalıcı komut için sürümlü schema, async sonuç, yetki, idempotency                                  |
| `apps/web/src/processing/runner.ts` ve `apps/web/src/processing/worker/` | Bildirimsel araç ve tarayıcı worker'ı                       | Ayrı Rust server executor; işi tüm tarayıcı belgesi yerine dataset/snapshot referansı ile çalıştır |
| `apps/web/src/style/*`, `docs/STYLE.md`                         | `.kstil`, MPYY sembolleri, stil derleme                     | Stil belgesini DB'de sürümle; server ve WASM ortak ifade semantiğini fixture ile doğrula           |
| `apps/web/src/render/types.ts`, `apps/web/src/render/webgpu/*`           | Çalışan WebGPU ve WebGL2                                    | Tile streaming, feature ID ve seçili nesne overlay'i; fallback'i koru                              |
| `apps/web/src/geo/crs.ts`                                       | SRID kataloğu; atamak/dönüştürmek ayrımı                    | PROJ/PostGIS dönüşümlerini pinlenmiş CRS/grid verisiyle doğrula                                    |

İlk güvenilirlik düzeltmesi: `CadDocument.transact` içindeki `finally`,
`fn()` hata attığında önceden uygulanmış değişiklikleri de commit edebilir.
Bunu rollback semantiği ve regresyon testiyle düzeltin. Sunucu transaction'ı
istemcideki `transact` yerine geçmez. `apps/web/src/app/commands.ts` içindeki `file.save`
şimdilik yalnızca `dirty=false` yapar; başarılı sunucu commit'i olmadan
kaydedildi diye göstermeyin.

## 14. Workspace, bileşenler ve sınırlar

**Kesin backend stack:** HTTP/API ve WebSocket için Axum, asenkron I/O
için Tokio, PostgreSQL/PostGIS erişimi için SQLx ve açık parametreli SQL,
HTTP middleware için Tower. Kimlik/tenant context, request ID, timeout,
istek boyutu ve concurrency sınırları composition root'ta kurulur.
İş kuralları middleware/route içine taşınmaz. Migration kontrollü tek
süreçte çalışır; bağlantı havuzu ve sorgu süreleri bütçelenir. Ağır CPU
hesabı Tokio I/O görevini bloke etmez; sınırlandırılmış CPU havuzu veya
ayrı worker kullanılır. Stack değişikliği sessizce yapılmaz; ADR gerekir.

Mevcut `apps/web/src/` ağacını (2026-09-24'te kullanıcı kararıyla kökteki `src/`'den taşındı, ADR 0001) toplu yeniden düzenlemeyin. Çalışan dikey dilim ilerledikçe
modüler Cargo workspace ekleyin. İlk kapsam için ayrı Rust **api** ve
**worker** süreç modları ile ortak uygulama çekirdeği yeterlidir:

```text
apps/web/                 TypeScript arayüzü ve WebGPU/WebGL2 (pnpm paketi)
apps/api/                 Axum: OIDC, feature/command API, TileJSON/MVT, events
apps/worker/              ağır analiz, import/export, indeks, tile warmup
apps/desktop/             (gelecek) wgpu masaüstü CAD/CBS; crates/shared'i native kullanır
crates/shared/            platformdan bağımsız, web/sunucu/masaüstü ortak kütüphaneler
  geometry-core/          saf Rust CAD geometri; native + WASM
  contracts/              sürümlü request/response/command; TS tip üretimi (`ts` özelliği)
  formats/                dosya biçimleri (DXF, koordinat listeleri)
  style-core/             sürümlü CAD stil/ifade IR; native + WASM
crates/wasm/              tarayıcıya dar geometri/stil/biçim API'si (wasm-bindgen)
crates/server/            yalnız sunucu
  application/            ortak use case, yetki, işlem ve job sözleşmesi
  postgres/               SQLx, PostGIS SQL, migration ve veri repository'si
  tiles/                  MVT/TileJSON yayınlama, cache invalidation
docs/adr/                 ölçülen mimari kararlar
benchmarks/               PostGIS/Martin ile karşılaştırılabilir yükler
```

`crates/shared` altındaki bir crate Tokio, SQLx, HTTP, DOM, wasm-bindgen ve
wgpu'ya bağımlı olamaz; tarayıcıya `crates/wasm`, sunucuya `crates/server` ve
`apps/api`, masaüstüne `apps/desktop` üzerinden açılır (dizin düzeni
2026-09-24'te kullanıcı kararıyla bu gruplara ayrıldı).

Saf Rust çekirdek Tokio, SQLx, HTTP, DOM ve WebGPU'ya bağımlı değildir.
UI, CLI, chat ve MCP aynı `application` kullanım durumlarına gider; HTTP,
MCP ve worker iş kurallarını yeniden yazmaz. Sunucu aktörü oturumdan
çıkarır; TypeScript'in kendi role/tenant bilgisini yetkili veri saymayın.
Rust ile TypeScript arasındaki protokol sürümlüdür; bilinmeyen komut/alan
sürümü kontrollü hata verir. WASM yerel önizleme yapar, nihai doğrulama
sunucuda yapılır. Geometriyi TS'den Rust'a bir anda taşımayın: mevcut
`model/geom/` fixture'ları üzerinden tolerans ve sonuç eşitliği kurun.

**Tek hesaplama kaynağı kapısı:** Yeni CAD hesapları `geometry-core` içinde
bir kez yazılır; aynı crate native ve `wasm32` olarak derlenir. Sunucu ve
istemci aynı sürümlü işlem adı, input/output şeması, birim, CRS, null/hata
ve tolerans kurallarını kullanır. Her taşınan işlem için aynı fixtures
(düz çizgi, bulge, delik, dejenerasyon, büyük koordinat, sınır durumları)
iki hedefte çalışır. Beklenen fark sınırı algoritma ve CRS başına yazılır;
iki sonuç farklıysa sessizce istemci sonucunu commit etmeyin. WASM yalnız
yerel hesap/önizleme ve doğrulamayı yapar; veri yetkisi, PostgreSQL sorgusu
ve commit sunucudadır. Eski TypeScript algoritması, Rust eşdeğerliği
kanıtlandıktan sonra tek kaynak ilkesini koruyacak şekilde kaldırılır.

## 15. PostGIS veri modeli: GIS kaynağı ve CAD tanımı

**İki gösterimin rolü açıktır:** `geom` GIS sorgu, indeks ve yayın için
PostGIS `geometry` kolonudur. Nokta/çizgi/poligon gibi doğrudan eşlenen
nesnelerde `geom` aynı zamanda kaynaktır. Analitik CAD nesnesi PostGIS
içinde kayıpsız ifade edilemiyorsa sürümlü `cad_definition` (ör. bulge,
elips, spline, ölçü, ilişkisel tarama, blok) kaynak tanımdır; `geom` onun
geometri projeksiyonudur. Render/MVT için örneklenen geometri **asıl CAD
tanımının yerini almaz**. İki kolonun değişimi tek DB transaction'ında,
aynı Rust dönüşüm sürümüyle olur; değişim tetikleyicisi belirsiz bırakılmaz.

**Kaynak ve türev sözleşmesi:** Her nesne türü için `source_kind`,
`cad_definition.schema_version` (varsa), kaynak koordinatların SRID'si,
birim ve tolerans tanımlanır. Nokta/düz çizgi/poligonda kaynak `geom`;
analitik CAD türünde kaynak `cad_definition` veya kayıpsızlığı round-trip
ile doğrulanmış native PostGIS curve'dür. Türetilmiş `geom` kaynakla aynı
revision'da ve **tek** yazma yolunda üretilir; başka bir API yalnızca
`geom`u güncelleyip CAD tanımını eskitemez. Dönüştürücü sürümü revizyonda
tutulur. İçe aktarma, kopyalama, bölme, geri alma ve yeniden açma dâhil
her işlem kaynak -> serialize -> deserialize -> kaynak eşdeğerliğini sınar.
Görünüm toleransı kaynak geometri toleransından ayrı sürümlenir.

Örnek kavramsal tablo (migration, indeks ve kısıtları fazda somutlaştır):

```sql
feature (
  tenant_id uuid, project_id uuid, layer_id uuid, id uuid,
  version bigint, srid integer, kind text,
  geom geometry, cad_definition jsonb,
  properties jsonb, created_by uuid, updated_by uuid,
  created_at timestamptz, updated_at timestamptz,
  PRIMARY KEY (tenant_id, project_id, layer_id, id)
)
```

`properties jsonb` dinamik alanlar içindir; sık filtrelenen, domain/ilişki
kuralına sahip alanlar tipli tablo kolonuna veya katman şemasına uygun
ayrı indeksli yapıya taşınır. Öznitelikler bugün `Record<string,string>`;
null/boolean/integer/decimal/date/text/enum dönüşümü sürümlü migration'dır.
Projeye ve layer'a ait SRID doğrulanır; geometrinin SRID'si, boyutu,
türü, empty/invalid davranışı katman sözleşmesine bağlanır. Bir layer'ın
CRS'sini sessizce değiştirmeyin. Büyük kataloglar için veri başına tablo
veya paylaşımlı tablo/partition seçimini gerçek yükle ölçün; her kullanıcıya
ayrı tablo kurmayın.

PostGIS'in native eğri tiplerini uygun CAD nesnelerinde kullanmayı bir PoC
ile sınayın: `CircularString`, `CompoundCurve`, `CurvePolygon`, delikler,
round-trip koordinat ve precision testleri. Özellikle DXF bulge semantiği,
dağıtık eğri segmentleri, tam daire, elips, spline ve CAD constraint'lerinin
kaybını kontrol edin. Bazı uzamsal fonksiyonlar ve MVT, doğrusal geometri
ister; `ST_CurveToLine` veya Rust tessellation çıktısını açık toleransla
üretin. Edit ekranında analitik kaynak geometriyi ayrı endpoint'ten alın.
Custom PostgreSQL type/opclass ancak bu PoC'nin açığını, mevcut
`geometry + cad_definition` ile çözülemediğini ve bakım maliyetini ADR ile
gösterdikten sonra gündeme gelir. Böyle bir gereksinim çıkarsa CAD tipini
ve ilgili dönüşüm/indeks işlemlerini uygulamadan bağımsız, sürümlü bir
PostgreSQL/PostGIS uzantısı olarak tasarlayın; olgunlaşınca başka projelerin
de kullanabilmesi için ayrı açık kaynak paket olarak paylaşılabilir.

Örnek sorgu stratejisi: `tenant_id/project_id/layer_id` seçimi +
`geom && bbox` ile GiST aday filtresi + yetkili kesin mekânsal koşul.
Yazma transaction'ı geometri/CAD tanımı, tipli öznitelik, `version`,
revision/audit ve outbox'ı birlikte commit eder. `expected_version` koşullu
`UPDATE` sıfır satır etkilerse 409 döner; sessiz last-write-wins yoktur.
Kaynak değişince türetilmiş geometri, tile generation ve stil bağımlılığı
güncellenir. Mevcut `number` ID sunucu kimliği değildir; UUID benzeri kararlı
kimlik taşıyın, MVT'nin sayısal ID kısıtını ayrı eşleme olarak çözün.

**Edit commit protokolü:** Yerel çizim anında önizlenir; kaydetme isteği
`tenant/project/feature_id`, `command_id`, `idempotency_key`, `base_version`
ve kaynak CAD tanımını taşır. Sunucu Rust çekirdeği ve katman kurallarıyla
yeniden doğrular; yetkiyi kontrol eder; koşullu sürüm güncellemesi, `geom`,
audit, proje `data_revision` ve outbox'ı bir PostgreSQL transaction'ında
commit eder. Yanıt kalıcı feature ID + yeni sürüm + proje revizyonudur.
Tarayıcı ancak bu yanıttan sonra ilgili değişikliği kaydedildi sayar;
yeniden deneme aynı idempotency sonucu verir. 409'da yerel taslak korunur,
sunucu ve yerel geometri/alan farkı kullanıcıya gösterilir. Sunucuya
kaydedilmiş bir değişikliği geri almak, yetki ve güncel sürüm kontrolünden
geçen **yeni ters komuttur**; başka editörün değişikliğini sessizce geri
çevirmez. Yerel undo yalnızca henüz commit edilmemiş editleri de kapsayabilir.

## 16. Çoklu tenant, kullanıcı tahsisi ve bulut verisi

`Tenant`, `User(issuer,subject)`, `Membership`, `SeatAllocation`, `Project`,
`ProjectGrant`, `LayerPolicy`, `Publication`, `Job`, `AuditEvent` başlangıç
varlıklarıdır. **Üyelik** ile **koltuk tahsisi** ayrı tutulur: tenant yöneticisi
kullanıcı ekler/davet eder, devre dışı bırakır ve kalan koltuğu görür.
Bir kullanıcı ayrı üyeliklerle birden çok tenant'a katılabilir. Keycloak
OIDC entegrasyonunda issuer/audience/subject doğrulayın; e-posta kalıcı
kimlik değildir. Tenant seçimi yalnızca URL/JWT header değerine güvenmez:
aktif membership ve proje yetkisi sunucuda kontrol edilir.

İlk dağıtım: paylaşılan PostgreSQL cluster, her tenant-bağlı tabloda
`tenant_id`, bileşik benzersiz anahtarlar ve **uygulama yetkisi + RLS**.
RLS'yi gerçek uygulama rolüyle test edin; owner/superuser bypass davranışı,
connection pool'da `SET LOCAL` ile tenant bağlamı, transaction sonrasında
bağlam temizliği ve `SECURITY DEFINER` fonksiyonları ayrıca denetlenir.
İleride büyük/hassas tenant'ı ayrı DB'ye alma ihtimali repository/router
sınırında kalabilir; başlangıçta tenant başına ayrı cluster veya shard
orkestrasyonu kurmayın. Tile, identify, export, attachment, event ve MCP'de
aynı kaynak yetkisi uygulanır; shared cache anahtarı tenant/policy scope
ve revision içerir.

Bulut kalıcılığı PostgreSQL'ün yönetilen ya da kalıcı volume üzerinde
kurulması, WAL arşivi, point-in-time recovery, düzenli backup ve **geri
yükleme tatbikatı** ile sağlanır. MinIO/S3 uyumlu nesne depolama büyük
attachment, import/export dosyası, job çıktısı ve gerekiyorsa tile paketi
içindir; canlı veritabanı dosyasını nesne olarak açmaya çalışmayın.
DB kaydı ve S3 yükleme iki ayrı transaction olduğundan geçici nesne +
commit sonrası görünürlük + yetim temizliği tasarlayın. Tenant başına kota,
retention, şifreleme, bağlantı sırrı ve maliyet metriği tutun.

## 17. Martin sınıfı MVT, stil ve güncellik

**Martin ile işlev eşdeğerliği hedefi:** MVT katman yayını, TileJSON,
stil JSON, sprite, glyph/font ve çoklu kaynak desteği. Martin bunları
bugün sağlar; ilk sürümde Martin'i ayrı yayın servisi olarak Rust API
gateway arkasında kullanın. Yayın sihirbazı Martin kaynak/yayın tanımını
üretebilir; özel PostGIS fonksiyonlarıyla sorguyu kontrol edebilir.
Üretimde doğrudan Martin erişimini kapatıp private kaynağa her istekte
gateway yetkisi uygulayın. Stil/sprite/font URL'lerinin de aynı erişim
denetiminden geçtiğini doğrulayın. Daha sonra tek Rust sürecine gerek
duyulursa Martin'in `martin-core` kütüphanesini değerlendirin.

PostGIS'in `ST_TileEnvelope`, `ST_AsMVTGeom` ve `ST_AsMVT` işlemleri temel
MVT yoludur. KentOS Rust API tenant yetkisini, yayın revision'ını, katman
sihirbazını, özel sorgu doğrulamasını, veri güncelliğini, cache politikasını
ve event'i üstlenir. Martin'in mevcut özelliklerini sıfırdan kopyalamayın.
Yalnızca eşit veri/indeks/kaliteyle ölçülen gerçek bir darboğaz veya
karşılanmayan ürün gereksinimi için özel tile kodu yazın.

```text
GET /v1/tenants/{tenant}/projects/{project}/publications/{pub}/tilejson.json
GET /v1/tenants/{tenant}/projects/{project}/publications/{pub}/tiles/{z}/{x}/{y}.mvt
GET /v1/tenants/{tenant}/projects/{project}/styles/{style}/style.json
GET /v1/tenants/{tenant}/projects/{project}/styles/{style}/sprite[@2x].{json,png}
GET /v1/tenants/{tenant}/projects/{project}/fonts/{font}/{range}.pbf
GET /v1/tenants/{tenant}/projects/{project}/features/{id}
```

Tile sorgusu izinli alanlara, SRID dönüşümüne, indeksle tamponlu bbox
seçimine, açık clipping/quantization politikasına ve sorgu sınırına sahip.
MVT CAD için kayıplıdır; API editörün gerçek CAD tanımını döndürür. Stil
sistemi veritabanı satırına gömülmez: `.kstil` kaynak belge/sürüm ve
MapLibre uyumlu stil çıktısı birbirinden ayrılır. Basit çizgi/dolgu,
etiket, ikon ve desteklenen desenleri sürümlü MapLibre Style JSON, sprite
ve glyph olarak yayınlayın. `.kstil` içindeki her sembolün MapLibre'a tam
çevrilebildiğini varsaymayın; yayın önizlemesinde tam/kısmi/desteklenmiyor
raporu üretin. MPYY tarama, çizgi boyu işaret, ölçü ve özel CAD sembolizminin
tam görünümü KentOS WebGPU/WebGL2 stil motorundadır. Başka istemciler için
gerektiğinde sunucu tarafı raster/stil çıktısı ekleyin; etkileşimli CAD edit
bu raster çıktıdan yapılamaz. Özel binary scene tile ancak MVT + gerçek
feature API'sinin ölçülen sınırı ortaya çıkarsa ayrı sürüm olarak tasarlanır.
Mevcut WebGL2 fallback sürer.

**MapLibre GL JS çalışma zamanı zorunlu değildir.** Martin sunucu/yayın
katmanıdır; KentOS'un kendi `render/webgpu` ve `render/webgl2` arka uçları
birincil harita istemcisi olacaktır. Şu anki `SceneLayer` yerel belgeyi
çizer; MVT/TileJSON'dan veri alma, tile yaşam döngüsü, feature ID/picking,
zoom LOD, etiket çakışması ve büyük katmanlar için bellek sınırı henüz bu
depoda tamamlanmış değildir. Bunları geliştirmeden MapLibre'a gerek kalmadı
diye ürün eşdeğerliği iddia etmeyin. MapLibre Style Specification dış yayın
formatı; MapLibre GL JS ise bağımsız uyumluluk testi ve isteğe bağlı üçüncü
taraf istemci olarak tutulur. Gelişmiş `.kstil`/MPYY sembolleri kendi
renderer'ımızda tam, dışa aktarılan MapLibre stilinde destek matrisi kadar
görünür. Martin'in MapLibre projesine ait olması, tarayıcıda MapLibre GL JS
yüklenmesini gerektirmez.

Publication wizard: taslak -> kaynak/şema doğrulama -> alan/erişim/zoom
profili -> örnek tile -> değişmez revision -> atomik aktifleştirme.
Cache key tenant, publication revision, veri generation, policy scope,
filter, z/x/y, encoding içerir. Değişiklik transaction'ında outbox'a eski/
yeni bbox ve etkilenen katman kaydı yazılır; worker cache'i geçersiz kılar
ve event yayar. İstemci yeni tile gelene kadar yerel edit overlay'ini
korur. Public/private cache politikası ayrı; revize yetki eski tile URL'siyle
aşılamaz. Tile ve güncel veri farklı read replica'lardan geliyorsa
commit watermark uyumunu gözetin; ilk sürüm primary'den okumak daha basittir.

**Tile güncellik kapısı:** Feature commit'i monoton `data_revision` üretir.
Outbox olayı commit'ten sonra yayınlanır ve `feature_id`, önce/sonra bbox,
layer ve revision taşır. Tenant/policy kapsamına bağlı tile cache'i bu
olayla kirletilir; eski izin anahtarıyla 304 veya eski cache sonucu
dönülmez. İstemci commit edilen feature'ı yerel overlay'de gösterip eski
tile'daki aynı ID'yi gizler; silmede tombstone kullanır. Ancak yeni tile'ın
commit revision'ını kapsadığı doğrulandıktan sonra overlay kaldırılır.
Olay kaçarsa reconnect sırasında proje revision'ı karşılaştırılıp ilgili
tile'lar yeniden istenir. Bu akış iki istemci, edit, silme, reconnect ve
yetki iptali senaryolarıyla doğrulanır; yalnız TTL güncellik garantisi
sayılmaz. Martin'in dahili cache davranışı bu invariant'ı sağlayamıyorsa
KentOS gateway revision/URL veya cache devre dışı politikasıyla sağlar.

Martin karşılaştırması aynı PostgreSQL/PostGIS verisi, benzer sorgu,
indeksler, tile payload, donanım ve cache sıcaklığıyla yapılır. p50/p95/p99,
CPU, RSS, QPS, DB süresi, pool beklemesi ve tile byte raporlanır.
İki ölçüm raporu tutun: (1) saf Martin ve (2) KentOS gateway + yetki +
cache + gerçek kullanıcı politikası. Aynı sorgu ve veriyle önce en az
eşdeğer doğruluk ve görsel kalite, sonra performans optimizasyonu.
"Daha hızlı" hedefini ancak belirtilen veri/donanım/cache koşullarında
geçen benchmark sonrası ilan edin. Görsel kaliteyi ayrıca aynı sahnede
MapLibre ve KentOS render ekran görüntüleri, sembol/etiket yerleşimi,
çizgi kalınlığı, ölçek ve hit-testing sonuçlarıyla değerlendirin.

### 17.1 Uzun vadede Martin bağımlılığını kaldırma (şimdilik uygulanmaz)

Martin ilk üretim yayın motorudur. Bağımlılığı kaldırma hedefi **gelecek
fazın değerlendirme konusu** olarak kayıtlıdır; ilk sürümde kendi MVT
sunucumuzu yazmak, Martin'i çatallamak veya göç için ayrı framework kurmak
görev değildir. Önce gerçek katman yayını, stiller, kullanıcı yetkileri,
worker, backup ve canlı güncelleme çalışır hale gelsin. Gateway URL'leri
KentOS'a ait kaldığından istemciler Martin'in iç adreslerine bağlanmaz.

Gelecekte karar kapısı:

1. Martin'in karşılamadığı **somut** gereksinimi veya ölçülmüş maliyet/
   darboğazı bir ADR'de kaydet. Aynı sorgu ve veriyle Martin'i yeniden
   yapılandırma, PostGIS sorgusunu iyileştirme, cache ve `martin-core`
   seçeneklerinin yeterli olup olmadığını önce değerlendir.
2. Çıkacak yayının kapsamını envanterle: MVT/TileJSON, kaynak birleştirme,
   stil JSON, sprite, glyph/font, kullanılan raster/static çıktı, HTTP
   encoding/ETag, izin, tenant cache sınırı, yayın revision'ı ve canlı
   geçersizleştirme. Kullanılmayan Martin özelliklerini sırf eşdeğerlik
   uğruna uygulama; kullanılan hiçbir özelliği sessizce düşürme.
3. Rust/PostGIS tabanlı alternatif için aynı veri ve stillerde golden tile,
   geometrik doğruluk, görsel MPYY/etiket örnekleri, yetkisiz erişim,
   güncellik, yük ve çökme sonrası toparlanma testleri kur. Karşılaştırmada
   gateway dahil ve hariç p50/p95/p99, throughput, bellek, DB süresi ve
   işletim maliyetini ayrı raporla. Kabul eşikleri geçiş ADR'sinde gerçek
   iş yüküne göre belirlenir; ölçüm olmadan hız üstünlüğü varsayma.
4. Ancak bu kapı geçilirse yayın uygulamasını gateway arkasında kademeli
   devreye al: shadow karşılaştırma -> sınırlı tenant/canary -> genişletme.
   Martin'i geri dönüş yolu olarak tut; veri veya stil formatını müşteri
   istemcisinde kırmadan geçiş yap. Silme kararı üretim ölçümünden sonra.

Bu göç **PostgreSQL/PostGIS'i kaldırmaz**. Hedef yalnızca tile/stil yayın
servisindeki Martin bağımlılığını gerekçesi oluştuğunda kaldırmaktır.

## 18. Komut, MCP ve ağır işler için sunucu worker modu

`CommandRegistry` şu anda `run(args?: unknown): void` ile yerelde çalışır.
Kalıcı komutlar için `command_name`, `version`, `tenant_id`, `project_id`,
`idempotency_key`, `expected_versions`, `input`, `request_id` zarfı kurun.
Kimlik doğrulama -> yetki -> input parse/preview -> DB transaction ->
outbox -> sonuç sırasını bütün HTTP/CLI/MCP girişlerinde kullanın.
`map.zoomTo` gibi salt istemci komutları sunucu komutu sayılmaz. MCP'nin
listelediği araçlar ve her çağrı yetkiye göre sınırlanır; ajanlar için
diff/preview, iptal, audit ve riskli toplu değişikliklerde onay gerekir.

Sunucu modları:

```text
kentosd --mode api       # auth, CRUD/command, TileJSON/MVT, kalıcı WebSocket
kentosd --mode worker    # kalıcı analiz, import/export, tile warmup
kentosd --mode all       # yerel geliştirme
```

Job kayıtları PostgreSQL'de kalır. Worker claim için `FOR UPDATE SKIP LOCKED`
ve lease/fencing token kullanın; job durumları `queued -> running ->
succeeded|failed|canceled`. Retry en az bir kez olabilir: çıktılar
idempotent, commit `job_id`/fencing ile korunur. Uzun analiz tutarlı
snapshot/revision referansı taşır, commit sırasında hedef veri değişmişse
conflict veya açık rebase politikası uygular. Tokio reaktöründe ağır CPU işi
bloklamayın; sınırlandırılmış blocking pool/ayrı worker süreci kullanın.
Tenant başına eşzamanlı job, CPU/bellek, depolama ve tile üretim kotası;
ilk sürümde adil kuyruk. Web Worker kullanıcı etkileşimi için kalır;
sunucu worker'ına tarayıcı belgesinin tamamı gönderilmez.

WebSocket kopması, proje sekmesinin kapanması veya kullanıcının sayfayı
yenilemesi kabul edilmiş sunucu job'ını iptal etmez. Job ömrü DB'deki
kayıt/lease'e bağlıdır; socket veya HTTP request cancellation token'ına
bağlanamaz. İptal yalnızca yetkili `job.cancel` komutuyla veya açık
sunucu timeout/kota politikasıyla yapılır. Kabul yanıtı kaybolan isteğin
sonucu idempotency anahtarıyla sorgulanır; bağlantı geri geldiğinde aynı
iş sıfırdan başlatılmaz. Bağlantı/proje/autosave sözleşmesi §21'de tanımlıdır.

## 19. Uygulama sırası ve kabul

### 19.0 Mimari kabul kapıları — üretim iddiası için zorunlu

`CLAUDE.md`nin yazılmış olması bu kapıları geçirmez. Her kapının sonucu
gerçek servis, gerçek PostgreSQL/PostGIS ve hedef tarayıcıda tekrarlanabilir
olmalı; mock, yalnızca birim testi veya diyagram üretim kanıtı değildir.

| Kapı                                     | Kanıt / başarısızlık ölçütü                                                                                                                                                                                                                                                                         |
| ---------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **Uçtan uca kayıt**                      | A kullanıcısı analitik CAD nesnesini çizer/kaydeder, B aynı projede görür; yenileme ve sunucu yeniden başlaması sonrası nesne, kimlik, stil ve geometri korunur. Sunucu commit'i başarısızsa istemci taslağı ve `dirty` korunur.                                                                    |
| **Kayıpsız CAD**                         | Her desteklenen tür, delik/bulge/CRS ve büyük koordinat için kaynak round-trip fixture'ı geçer; MVT çiziminden kaynak CAD geometrisi yeniden kurulmaz. Desteklenmeyen tür açık hata verir.                                                                                                          |
| **Native/WASM ortak çekirdek**           | Aynı crate'in native/WASM sürümleri aynı fixture'larda belgelenmiş toleransı sağlar; eski TS uygulamasıyla geçiş testleri temizdir. Tek işlem için iki ayrı kalıcı hesaplama uygulaması kalmaz.                                                                                                     |
| **Kadastral sayısal doğruluk**           | §23'ün bağımsız referans, topoloji, eşik/yuvarlama, hisse ve alan korunumu testleri geçer. Native/WASM aynı nihai ondalık sonucu ve topolojik kararı üretir; belirsiz sonuç commit/yayın için engellenir.                                                                                           |
| **Çakışmasız edit/undo**                 | İki editör aynı sürümü değiştirince biri açık 409 alır, taslağı kaybolmaz; retry/idempotency mükerrer kayıt üretmez; ters komut başka editörün yeni değişikliğini silmez.                                                                                                                           |
| **Tile ve stil güncelliği**              | Commit sonrası diğer istemci değişikliği görür; overlay ve MVT aynı feature'ı çift çizmez. Stil, sprite/glyph, cache ve yetki iptali için public/private senaryoları geçer. Dış MapLibre stili destek matrisindeki sonucu; KentOS renderer tam CAD/MPYY sahnesini üretir.                           |
| **Modüler frontend**                     | Günlük 2D çizim ilk yükü stil/model/layout tasarımcısını, gelecekteki 3D urban design kodunu ve kullanılmayan büyük varlıkları indirmez. §20'deki başlangıç ve ilk özellik açılışı bütçeleri gerçek build/tarayıcı ölçümüyle geçer; lazy modül hatasında taslak kaybı olmaz.                        |
| **Bağlantı, asenkron proje ve autosave** | §21 senaryolarında WebSocket kopunca kabul edilmiş job devam eder; yeniden bağlanmada olay/job/revision eşitlenir. Proje açılışı arayüzü bloke etmez; eski proje isteği yeni projeyi değiştiremez. Bulut projesi otomatik kaydolur; commit yanıtı kaybolsa bile tekrar yazma ve taslak kaybı olmaz. |
| **Tenant ve iş izolasyonu**              | Farklı tenant üyeleri API, tile, style, identify, export, MCP, event ve job sonucunda diğerinin verisine erişemez. Connection pool/RLS, cache ve worker negatif testleri geçer.                                                                                                                     |
| **Dayanıklılık ve yük**                  | Worker durup yeniden alınır; yan etki iki kez commit edilmez. Backup + WAL/PITR geri yüklemesiyle seçilen noktadaki veri/audit tutarlıdır. Martin ve KentOS için aynı veri/indeks/donanımda soğuk/sıcak cache p50/p95/p99, hata, bellek, CPU ve görsel kalite raporu vardır.                        |

Kritik bir veri kaybı, tenant sızıntısı, kayıpsız geometri bozulması veya
yeniden başlatmada kaybolan commit **sürüm engelidir**. Etkileşim/tile/job
p95/p99 hedefleri, kabul edilecek veri seti, eşzamanlı kullanıcı sayısı,
referans donanım ve renderer görsel toleransı Faz A'da ADR'ye **ölçümden
önce** yazılır; sonra rapora uyacak şekilde değiştirilmez. Kapsamı eksilterek
benchmark kazanmak kabul edilmez. Kapıların sonuçları test komutu, commit,
ortam ve sınırlamayla raporlanır. Bu kapılar geçmeden ürün veya mimari için
"10/10 tamamlandı" ifadesi kullanmayın.

### Faz A — Kod güvenilirliği ve ortak sözleşme

- `transact` rollback ve `dirty` doğruluğu; gerçek Save/Open akışı öncesi test.
- Cargo workspace, ilk native/WASM analitik geometri fixture'ları;
  TypeScript bulge/delik/CRS sonuçlarıyla karşılaştırma.
- §23 sayısal politika/yuvarlama sözleşmesi ve bağımsız doğruluk
  fixture'ları; eski TS sonucuyla eşitliği tek doğruluk ölçütü saymayın.
- Sürümlü `Entity`, stil, `RunJob`, command sözleşmeleri; Rust API health.
- §20'deki yöntemle build çıktısının entry ve isteğe bağlı chunk
  envanteri; başlangıç yükünün tekrar üretilebilir baseline ölçümü.

**Çıkış:** Var olan `pnpm build`, `pnpm test`, `pnpm e2e` korunur; aynı
analitik nesne iki ortamda belirtilen toleransla aynı sonucu verir.

### Faz B — Bir tenant'ta gerçek proje ve izinli kayıt

- OIDC, tenant membership/rol; PostgreSQL/PostGIS migration ve RLS.
- Bir katmanda aç/düzenle/kaydet, tipli alanlar, `expected_version`, audit.
- Bir tenant'ın diğerini ID, API ve tile yolu ile görememesi testi.
- Asenkron proje açılışı, bulut autosave, yerel bekleyen değişiklik günlüğü
  ve izlenen WebSocket bağlantısı; §21'deki ilk bağlantı/commit kopma testleri.

**Çıkış:** Tarayıcı yenilenince çizim verisi durur; iki editörde çakışma
409 olur; `Kaydet` ancak DB commit başarısında tamamlanır.

### Faz C — Yayın ve stil

- Martin üzerinden PostGIS -> MVT/TileJSON, stil JSON, sprite, glyph;
  yayın wizard, izin, cache, outbox invalidation.
- Native eğri ve CAD JSONB round-trip, zoom'a göre örnekleme fixture'ları.
- Standart MapLibre ve KentOS WebGPU istemcisinde aynı veri kimliği.

**Çıkış:** Yetkili kullanıcı yayını açar; değişiklik doğru tile'da görünür;
yetkisiz tile, metadata, identify ve export alamaz.

### Faz D — Worker, analiz, operasyon

- PostgreSQL job tablosu/lease/retry; gerçek ağır GIS analizi veya import.
- PITR/yedek/geri yükleme tatbikatı; yük altında RLS, pool, tile ve job testi.
- Kademeli performans iyileştirmesi; yalnızca kanıtlanan darboğazı düzelt.

**Çıkış:** Worker çökmesi iş kaybettirmez veya iki kez yan etki üretmez;
API ağır işte yanıt verir; restore edilen proje geometri ve revizyonu korur.

### Gelecek Faz E — Yayın motoru bağımsızlığı için karar

- Faz A–D gerçek üretim ölçümleri ve §17.1 karar kapısı tamamlanmadan
  başlanmaz. Çıkış kararı Martin'i korumak da olabilir.
- Geçiş gerekiyorsa doğrulanmış kapsam, kademeli rollout ve geri dönüş
  planıyla Martin bağımlılığını kaldır; PostGIS veri katmanını koru.

### Gelecek Faz F — İmar planından 3D urban design

- §22'nin veri ve modül sınırlarını bugünkü tasarımda koruyun; 3D ürün
  özelliklerini şimdi topluca uygulamayın. Uygulamaya Faz A–D'nin ilgili
  kayıt, ortak çekirdek, iş ve izolasyon kapıları geçildikten sonra başlayın.
- İlk dikey dilim: tek parsel + sürümlü imar kısıtları -> ortak Rust
  çekirdeğinde yapı kütlesi -> isteğe bağlı 3D görünüm -> senaryo kaydı.
- Sonra arazi, çok parsel, senaryo karşılaştırma, ileri analiz ve büyük
  şehir sahnelerine ölçülmüş ihtiyaçla genişletin.
- Bu fazın başlaması Martin'in kaldırılmasına bağlı değildir; Faz E
  ayrı bir yayın motoru kararıdır.

Claude Opus görevi: Önce repo ve bu dokümanın mevcut/güncel ayrımını oku.
Faz A'dan bir dikey dilim tamamla; ölçmeden "Martin'den hızlı" yazma.
Mevcut stil ve WebGPU kodunu koru. Yeni DB motoru veya PostgreSQL tipi
önermeden önce bu kararın hangi somut eksikliği çözdüğünü benchmark/ADR ile
göster. Her değişiklikte migration, tenant sınırı, başarısızlık kurtarma ve
uygun doğrulamayı raporla.

## 20. Modüler frontend ve isteğe bağlı yükleme

KentOS'un arayüzü mevcut DOM/`h()`/Signal mimarisini sürdürür (§3–4);
React veya büyük bir plugin framework'ü yalnızca modülerlik için eklemeyin.
**Modülerlik iki ayrı gerektir:** kod sahipliği/bağımlılık sınırı ve gerçek
bundle ayrımı. Sadece klasör açmak başlangıç JS'ini küçültmez; yalnızca
`import()` yazmak da ortak statik import edilen ağır modülü ayırmaz.

### 20.1 Mevcut kod envanteri ve ilk sınırlar

Bu commit'te `StyleManager`, `LayerStyleDialog`, `SymbolDesigner` ve
`ModelDesigner` dinamik import ile açılıyor; bunları tekrar "henüz yok"
diye raporlamayın. Ancak `apps/web/src/app/createApp.ts` uygulama başında demo/
showcase, sistem stil kitaplığı ve işlem penceresini; `apps/web/src/ui/dock/RightDock.ts`
görünür olmasa bile `ProcessingPanel`i statik import/oluşturma yoluyla
yüklüyor. `apps/web/src/style/system/mpyy/index.ts` içindeki `import.meta.glob`
`eager: true` bütün MPYY sayfa tanımlarını ilk grafiğe alıyor.
`apps/web/src/main.ts` stil/model/processing CSS'ini baştan yüklüyor.
`createBackend.ts` her iki renderer'ı import ediyor; varsayılan WebGL2
oturumunda WebGPU kodunun ayrı chunk olmasını ölçerek değerlendirin.

Kullanıcının bildirdiği yaklaşık **700 KB** build çıktısı bu ortamda
ölçülmüş başlangıç aktarım boyutu değildir. Önce `pnpm build` çıktısında
**toplam üretilen dosya**, **ilk sayfada istenen JS/CSS**, **gzip/Brotli
aktarımı**, **parse/evaluate süresi** ve **ilk etkileşime hazır olma**
ölçümlerini ayrı kaydedin. Başlangıç maliyetini toplam dist boyutuyla
karıştırmayın. Profilde hangi importun chunk'a girdiği somut olarak
görülmeden dosya adı veya elle `manualChunks` kuralıyla optimize etmeyin.

| Başlangıç çekirdeği                                                                                                                    | İsteğe bağlı modül                                                                                        |
| -------------------------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------- |
| Kabuk, katman ağacı, komut/kısayol kataloğunun hafif metadata'sı, kamera, temel çizim/render, aktif belgenin kullandığı stil tanımları | Stil yöneticisi/tasarımcısı, tüm MPYY katalog/önizleme verisi, stil import/export                         |
| Temel çizim araçları, seçim, aktif nesnenin düzenlenmesi                                                                               | Model/iş akışı tasarımcısı, ileri analiz/rapor/pafta-layout ve gelecekte raster/TIN araçları              |
| Varsayılan WebGL2; açıkça seçilirse WebGPU                                                                                             | Büyük format parser'ları ve ilgili WASM/worker kodu                                                       |
| 2D proje/nesne kimliği, hafif görünüm ve senaryo metadata'sı                                                                           | Gelecekte 3D urban design, 3D sahne, arazi, yapı kütlesi üretimi ve bunlara özel shader/texture/mesh/WASM |

Bu tablo varsayılan çizim için gereken gerçek sembollerin eksik
kalabileceği anlamına gelmez. **Aktif projenin bağımlı stillerini** açılışta
çözün; geri kalan sistem kataloğunun ad/kategori/arama indeksini hafif
metadata olarak tutup sembol tanımı ve SVG/raster içeriğini kategori veya
kullanım bazında isteyin. Katalog ve stil varlıkları sürümlü/cache'lenebilir
olsun. Mevcut showcase bütün sistem kütüphanesini kullandığından gerçek
ürün başlangıç ölçümünde demo ayrı senaryo olmalıdır; testleri kırmadan
özel demo girişine veya açılır örneğe taşımayı planlayın.

### 20.2 Yükleme sözleşmesi

- Bir özellik girişinde hafif `id`, `command`, `capabilities`, `load()`
  tanımı bulunur. Komut araması menüde görünür kalır; çalıştırınca ilgili
  modül yüklenir. Domain çekirdeği özellik UI dosyasını statik import etmez.
- Modüller yalnızca gerekli olduğunda açılır. Stil tasarımcısı,
  model/iş akışı tasarımcısı, layout tasarımcısı, ileri processing penceresi,
  format import/export ve ağır veri analizleri ayrı yükleme sınırlarıdır.
  Panel ilk etkinleştirmede kurulup kapanırken abonelik/GPU kaynağı bırakır;
  kaydedilmiş panel sekmesi geri açılıyorsa önceden hazırlanır.
- Dinamik import/asset/WASM yüklemesi başarısızsa komut açık bir hata ve
  yeniden dene eylemi verir; boş pencere veya kaybolan taslak bırakmaz.
  Geç açılan editör mevcut belge/sürüm/tenant bağlamını yeniden kontrol
  eder. Geç gelen import sonucu eski belgeye uygulanmaz.
- CSS özellik modülüyle yüklenir; başlangıç kabuğunun tema/token CSS'i
  ortak kalır. İki modülde kullanılan küçük stil/yardımcı kodu gereksiz
  tekrarlamayın; devasa ortak chunk'a taşımayın.
- Geometri için başlangıçta zorunlu Rust/WASM altkümesi ile ağır analiz,
  CRS grid, büyük format parser veya model tasarım araçlarının kod/verisi
  ayrı yükleme kararlarıdır. `wasm-bindgen` ile tek büyük `.wasm` üretilirse
  lazy JS importu başlangıç indirmesini azaltmayabilir; `.wasm` indirme,
  derleme, başlatma ve transferi de ayrıca ölçün. Backend native çekirdeği
  aynı sözleşmeleri sağlar; chunk ayrımı hesap semantiğini değiştirmez.
- Her komutun modülü sürüm ve yetenek bildirir. Yetkisiz kullanıcının
  özelliği gizlense bile sunucu yetkisi ayrıca doğrulanır. Sadece paket
  parçalaması için gereksiz ağ isteği, paket tekrarı veya bellek şişmesi
  yaratmayın.

### 20.3 Büyük uygulama için modül sınırları ve yaşam döngüsü

- Kabuk; oturum, proje, komut, seçim, kayıt kuyruğu, job ve bağlantı
  servislerini yayımlanan dar arayüzlerle sunar. Özellik modülleri bu
  sözleşmelere bağımlıdır; kabuk özelliklerin iç uygulamasına bağımlı
  olamaz. Modüller birbirinin iç dosyalarını import etmez; birlikte
  çalışma sürümlü sözleşme/komut/olay üzerinden olur. Döngüsel bağımlılık
  ve ağır özellikleri yeniden dışa aktaran ortak barrel dosyaları engellenir.
- Stil, workflow, layout ve gelecekte urban design ayrı özellik
  girişleridir. Urban design içinde 3D görünüm, arazi, kütle üretimi ve
  analiz de ihtiyaçlarına göre ayrılır; tek devasa "3D" chunk zorunlu
  değildir. Manifestler yalnız hafif metadata ve loader içerir.
- Yaşam döngüsü `load -> activate -> deactivate -> dispose` olarak
  tanımlanır. `deactivate` sırasında tutulacak kaynaklar bütçeyle
  sınırlanır; `dispose` DOM, abonelik, zamanlayıcı, özel worker ve GPU
  buffer/texture kaynaklarını bırakır. Ortak kaynakların sahipliği ve
  referans sayımı belirgindir. Dinamik import edilmiş JS modülünün
  tarayıcı modül cache'inden silineceği varsayılmaz; kaynak temizliği
  ile kodun bellekten tamamen boşaltılması aynı garanti değildir.
- Proje verisi, bekleyen autosave ve sunucu job'ı panel yaşam döngüsüne
  bağlı değildir. Panel kapanması bunları kaybettirmez veya job'ı iptal
  etmez. Yeniden açılış aynı proje/senaryo revision'ına bağlanır; geç
  sonuçlar §21'deki generation kontrolünden geçer.
- İlk 2D açılışta 3D'ye özel JS/CSS/WASM, shader, texture, mesh ve worker
  yüklenmez/başlatılmaz. Ortak Rust kaynak kodu kullanımı bütün WASM
  özelliklerini tek başlangıç dosyasına bağlamayı gerektirmez; temel 2D
  ve ileri 3D için ayrı derleme girişleri ve açık veri sözleşmeleri kurun.
- Prefetch yalnız kullanıcı niyeti, açıkça geri yüklenecek görünüm veya
  ölçülmüş ihtiyaçla, ağ/bellek bütçesi içinde çalışır. Bütün modülleri
  açılıştan hemen sonra arka planda indirmek lazy load kabul edilmez.
- CI'da import/chunk grafiğiyle bağımlılık sınırlarını ve 2D girişinin
  3D koduna erişmediğini doğrulayın. Her ağır modülün aktarım, ilk açılış,
  ana thread süresi, CPU/GPU bellek ve arka plan iş bütçesi olsun.
  Tekrarlı aç/kapat ve 2D/3D geçişlerinde kaynakların bütçede kaldığını
  ölçün; paket boyutu tek performans ölçütü değildir.

### 20.4 Kabul ve uygulama sırası

1. Sabit referans tarayıcı/cihaz ve soğuk/ılık cache ile ilk yük, ilk çizim,
   ağır modülü ilk/ikinci açma ve modül arası geçişi ölç; chunk grafiğini
   kaydet. Başlangıç JS/CSS/WASM byte ve çalıştırma bütçesini ADR'de
   **değişiklikten önce** belirle. Başka bir modülü ilk yükten çıkarıp ilk
   tıklamayı kabul edilemez geciktirmek kazanım sayılmaz.
2. Mevcut lazy sınırların gerçekten ayrı chunk verdiğini doğrula; sonra
   sistem stil kataloğu/showcase, işlem paneli/dialog ve alternatif renderer
   gibi ölçülen başlangıç bağımlılıklarını sırayla ayır. Her adımda farkı
   yeniden ölç ve bağımlılık yönünü gözden geçir.
3. Gelecek layout/pafta, workflow ve stil tasarımcısı modüllerinin hafif
   komut kaydı + dinamik yükleme + hata/iptal/dispose şablonunu uygula.
   Açılmayan modülün kodu, CSS'i, asset'i ve ona özel WASM'i ilk ağ
   isteğinde görünmemelidir; kullanılan modül ilk kullanımda eksiksiz
   çalışmalıdır.
4. Görsel ve uçtan uca testler: doğrudan komutla ve menüden açma,
   internet/asset hatası ve retry, tema, panel geri yükleme, belge
   değiştirme sırasında geç import, aktif stilin doğru çizimi, WebGPU
   fallback, worker/işlem iptali. Başlangıç ve ağır modül budget'ları CI'da
   izlenir; meşru büyüme için açıklamalı budget güncellemesi gerekir.

**Mimari kabul kapısı:** Günlük çizim akışı büyük tasarımcıların kodunu
yüklemeden çalışır; büyük özellikler gerektiğinde açılır ve taslağı
korur. Gerçek `pnpm build` çıktısı ve tarayıcı ölçümüyle başlangıç maliyeti
iyileştirilmiş, özelliklerin ilk açılış gecikmesi görünür ve kabul edilen
bütçeye uygun, davranış eşdeğerliği testle gösterilmiş olmalıdır.

## 21. Sürekli bağlantı, asenkron projeler ve bulut otomatik kaydı

Bu bölüm hedef davranıştır; mevcut tarayıcı uygulamasında uygulanmış
sayılmaz. Axum WebSocket sunucusu, Tokio ağ işleri ve SQLx/PostgreSQL
kalıcılığı kullanılır. WebSocket sürekli oturum/olay kanalıdır; tile,
büyük dosya, sayfalı sorgu ve idempotent command gönderimi HTTP üzerinden
çalışabilir. Socket açık olmak bütün servislerin sağlıklı olduğu veya
bir değişikliğin veritabanına kaydedildiği anlamına gelmez.

### 21.1 Bağlantı izleme ve yeniden eşitleme

- İstemci durumu `connecting`, `online`, `reconnecting`, `offline`,
  `auth_required` olarak izler. Durum çubuğunda bağlantı, son başarılı
  eşitleme ve bekleyen kayıt sayısı gösterilir. Bağlantı durumu ile
  kaydetme durumu ayrı sinyallerdir.
- Tarayıcı uyumlu uygulama heartbeat/ack mesajı ve timeout kullanın;
  native WebSocket ping API'sinin tarayıcı JavaScript'inde var olduğunu
  varsaymayın. İlk ayar heartbeat 20 saniye, yanıt yokluğu eşiği 60 saniye;
  bunlar yapılandırılabilir ve arka plan sekmesi/cihaz uyku durumunda
  yeniden değerlendirilir. `navigator.onLine` yalnızca ipucudur.
- Otomatik reconnect üstel bekleme + jitter uygular (başlangıç 1 saniye,
  üst sınır 30 saniye); normal kullanımda denemeler sürer. Kimlik doğrulama
  hatası sonsuz retry üretmez; oturum yenileme/giriş gerekir. Uykudan veya
  yeniden görünür duruma dönüşte sağlık ve revision kontrol edilir.
- Her istemci/proje aboneliği son uygulanmış event cursor'ını tutar.
  Sunucu kalıcı outbox/event akışından eksikleri tekrar verir; yinelenen
  olaylar event ID/revision ile elenir. Saklama süresi aşılmış cursor için
  `resync_required` döner; yeni proje snapshot'ı alınır. Snapshot revision'ı
  ile sonraki event cursor'ı arasında boşluk kalmayacak bir bootstrap
  protokolü kurun. Cursor taşıma katmanının "mesaj gönderildi" bilgisi değil,
  istemcinin olayı uygulamış olduğunun işaretidir.
- Socket yeniden kurulduğunda tenant/proje abonelikleri ve yetkiler tekrar
  doğrulanır; aktif job'ların durumu DB'den alınır. Presence/cursor gibi
  geçici olaylar kalıcı komut ve veri revizyonlarından ayrıdır. Yavaş
  istemcide sınırlı kuyruk/backpressure kullanın; kalıcı veri olayını
  sessizce düşürmek yerine resync başlatın.
- WSS, oturum doğrulama ve Origin kontrolü uygulanır. Uzun ömürlü token
  URL/query string'e konmaz. Yetki iptalinde ilgili abonelik kapanır;
  sonraki komutlar tekrar yetkilendirilir.

**Kopma davranışı:** Sunucunun kalıcı olarak kabul ettiği işler §18'e göre
devam eder. Henüz sunucuya ulaşmamış komutlar "bekliyor" durumundadır;
başlamış veya kaydedilmiş gösterilmez. Kullanıcı yüklenmiş veri üzerinde
izinli yerel çizim/önizlemeye devam edebilir; yeni uzak sorgu, eksik tile
ve sunucu analizi bağlantı bekler. Web Worker ve yerel WASM hesapları
socket'a bağlı değildir. Bu davranış ilk sürümde tam offline proje
replikası desteği olduğu anlamına gelmez.

### 21.2 Bütün projeler asenkron ve aşamalı açılır

Bulut ve yerel kaynak sağlayıcıları `openProject` işlemini asenkron
sunmalıdır. Önce proje kimliği/yetki, metadata, layer ağacı, CRS ve stil
manifesti gelir; ardından görünür bölge tile'ları, gerekli semboller ve
seçili/editlenecek nesnelerin tam geometrileri öncelikle yüklenir.
Milyonlarca nesnenin tamamını RAM'e almadan kullanılabilir çalışma alanı
açılır. Yerel büyük dosya ayrıştırma ve ağır dönüşüm worker/WASM'da
yapılır; `async` anahtar kelimesi tek başına CPU işini ana thread'den ayırmaz.

Proje durumu `opening -> interactive -> ready`, ayrıca `error/canceled`
durumlarını taşır. `interactive` görünür çalışma alanının kullanılabilir
olduğunu, `ready` ise açılış için gerekli manifest/ilk veri kapsamının
tamamlandığını belirtir; tüm veri kümesinin indirilmesi demek değildir.
Yükleme adımı/ilerleme ve iptal görünür olur; toplam bilinmiyorsa sahte
yüzde gösterilmez. Metadata, tile, özellik ve asset yüklemeleri ayrı
eşzamanlılık/bellek bütçesi ve geri basınç kullanır.

Her açılış bir `load_generation` ve iptal sinyali taşır. Proje veya tenant
değişince eski istekler/abonelikler iptal edilir; geç dönen cevap yalnızca
eşleşen proje/generation'a uygulanır. Render ve edit servisleri yeni
belgeye atomik bağlanır; eski projenin seçim, undo, job görünümü ve taslağı
yenisine karışmaz. Sekme değişimi kayıt kuyruğunu kaybetmez; kuyruğun sahibi
panel yaşam döngüsü değil proje oturum servisidir.

### 21.3 Bulut projelerinde otomatik kayıt varsayılandır

Kaynak sağlayıcı `storage_mode: cloud | local` ve yazma yeteneğini bildirir.
Bulut projesinde otomatik kayıt varsayılan olarak açıktır; yerel dosyada
ayrı dosya kaydetme ve kurtarma taslağı politikası geçerlidir. Otomatik
kayıt; tamamlanmış CAD komutlarını, katman/proje metadata'sını, proje
stillerini ve projeye ait model/layout tanımlarını kapsar. Kişisel UI
tercihleri kendi kullanıcı kapsamındadır. Tasarımcıdaki henüz onaylanmamış
taslak ayrı kurtarılır; her form tuşu yayınlanmış proje verisi sayılmaz.

- Pointer hareketlerini sunucuya yazmayın. Tamamlanmış yerel transaction
  sonrası değişikliği proje/tenant/kullanıcı kimliğiyle IndexedDB'de
  dayanıklı bekleyen komut günlüğüne alın. Başarısız yerel yazma/kota
  durumunda taslağın kalıcı olduğu iddia edilmez; kullanıcıya açık hata
  gösterilir ve veri bellekten sessizce atılmaz.
- İlk ayar son tamamlanmış değişiklikten 1 saniye sonra debounce,
  kesintisiz değişiklikte en geç 5 saniyede gönderim; yapılandırılabilir.
  Manuel Kaydet/Ctrl+S bekleyen kuyruğu hemen göndermeyi dener. Tüm proje
  snapshot'ı yerine değişen komut/nesne/metadata gönderilir; undo adımları
  ve atomiklik korunur.
- Her projede sıralı commit kuyruğu kullanın; bir kayıt sürerken yapılan
  yeni editler yeni yerel revision ile sırada kalır. Sunucunun ACK'i
  yalnız kapsadığı `local_revision`/komutları temizler; eski ACK yeni editin
  `dirty` durumunu silemez. Kimlik eşlemesi, base version ve idempotency
  §15'teki protokolle aynıdır. IndexedDB'deki komut ancak sunucu commit'i
  doğrulandıktan sonra kuyruktan çıkarılır.
- Durumlar `saved`, `pending`, `saving`, `offline_pending`, `conflict`,
  `error` olarak görünür. "Kaydedildi" yalnızca sunucu commit ACK'i ve
  bekleyen edit olmaması halinde gösterilir. Yerel taslak koruması ile
  bulut kaydı farklı durumlardır.
- Kopma veya zaman aşımında kuyruğu koruyup aynı idempotency anahtarıyla
  devam edin. Commit gerçekleşip yanıt kaybolursa aynı komutun sonucu
  sorgulanır/yeniden döndürülür. 409'da otomatik üzerine yazma yoktur;
  yerel taslak korunur, etkilenen kuyruğun devamı çatışma çözülene kadar
  durur. Yetki iptalinde göndermeyi durdurup açıklayıcı durum gösterin.
- Sekme/uygulama kapanışındaki son ağ isteğine güvenmeyin. Kurtarma
  günlüğü edit sırasında kalıcılaşır. Sayfa yeniden açılınca oturum,
  tenant, kaynak proje revision'ı ve bekleyen komutlar karşılaştırılarak
  devam edilir; sunucudan gelen snapshot bekleyen yerel editleri ezemez.
  Oturum/tenant değişimi başka kullanıcının taslağını otomatik göndermez.

### 21.4 Zorunlu uçtan uca doğrulama

1. Bağlantıyı heartbeat öncesinde/sonrasında kes; arayüz doğru durumu
   gösterir, yeniden bağlanır ve kaçırılan olayları çift uygulamadan alır.
2. Uzun job sırasında socket'ı ve sekmeyi kapat; job devam eder, tekrar
   açılışta aynı `job_id` ilerleme/sonuç görünür; yeni job üretilmez.
3. Commit öncesinde, commit sonrasında ACK ulaşmadan ve autosave sırasında
   yeni edit yapılırken ağı kes; kurtarma sonrası kayıp/mükerrer kayıt ve
   yanlış "Kaydedildi" durumu oluşmaz.
4. İki istemcide aynı feature'ı değiştir; reconnect/autosave çatışmayı
   gösterir, kaynak CAD tanımı ile yerel taslak korunur.
5. Büyük proje açılırken arayüz/iptal çalışır; hızlı proje/tenant değişiminde
   eski yanıt, tile, seçim ve stil yeni projeye uygulanmaz.
6. IndexedDB kota/yazma hatası, süresi dolmuş oturum, event cursor süresinin
   aşılması ve read-only projede otomatik kayıt durumları doğru gösterilir.

## 22. Gelecek hedef: imar planına dayalı 3D urban design

KentOS uzun vadede CAD/GIS, imar planlama ve 3D kentsel tasarım çalışma
alanlarını aynı proje üzerinde sunacaktır. Bu bölüm **gelecek ürün
kapsamıdır**; mevcut kodda 3D urban design tamamlanmış değildir. Bugün
§20'nin sınırlarını kurun; henüz kullanılmayan 3D motoru, bağımlılıkları
ve araçlarını başlangıç uygulamasına eklemeyin. Render motoru/format
seçimini ilk dikey dilimde mevcut renderer, veri ölçeği ve cihaz
yetenekleriyle karşılaştırılmış bir ADR ile yapın.

### 22.1 Kaynak veri, kurallar ve senaryolar

- Parsel, imar planı/kullanım kararı, yapı ve arazi kaynakları kalıcı
  kimlikleri ve revision'larıyla tutulur. Yükseklik/kat, çekme mesafesi,
  TAKS/KAKS/emsal gibi parametreler kapsam, birim, kaynak plan ve kural
  sürümüyle modellenir. Eksik veya çelişkili kural sessiz varsayımla
  doldurulmaz; kullanıcıya çözümlenecek durum olarak gösterilir.
- Parametrik tasarım tanımı, kullanıcı kararları ve senaryo girdileri
  kaynağı oluşturur; mesh/LOD/önizleme yeniden üretilebilir çıktıdır.
  Serbest düzenlenmiş geometri veya elle yapılan istisnalar ayrıca
  sürümlenir; yeniden üretim bunları sessizce ezemez.
- Her üretim `scenario_id`, kaynak revision'ları, parametreler,
  `algorithm_version` ve sonuç manifestiyle izlenir. Plan/parsel değişince
  bağımlı sonuçlar eski olarak işaretlenir; yalnız etkilenen kapsam
  yeniden hesaplanır. Senaryo karşılaştırması aynı kaynak tabanını veya
  tabanlar arasındaki farkı açıkça gösterir.
- 2D ve 3D aynı kalıcı feature kimliklerini kullanır; seçim ve özellik
  inceleme ortak servisler üzerinden eşitlenir. Ayrı ve zamanla ayrışan
  bir "3D proje veritabanı" oluşturmayın. PostGIS kaynak mekânsal veriyi,
  PostgreSQL senaryo/kural tanımlarını, nesne depolama büyük türetilmiş
  sahne varlıklarını tutar; hepsi tenant/proje yetkilerine tabidir.
- Hesap sonuçları kullanılan kuralların değerlendirmesidir; belirsiz
  kural veya eksik veri varken otomatik "imar uygunluğu onaylandı"
  sonucu üretilmez. Sonuç, kullanılan veri ve kural sürümünü gösterir.

### 22.2 Ortak hesaplama ve büyük sahne

- Yapı kütlesi/parametrik geometri hesapları tek Rust uygulamasından
  native worker ve tarayıcı WASM olarak derlenir. §14'ün parity/tolerans
  kapısı burada da geçerlidir. İleri 3D crate'leri temel 2D çekirdeğine
  ters bağımlılık yaratmaz; farklı paketleme ayrı algoritma yazmak değildir.
- Etkileşimli küçük önizleme Web Worker/WASM'da, büyük parsel kümeleri,
  arazi/mesh üretimi ve ağır analiz sunucu job'ında çalışır. Her girdide
  tüm şehir yeniden üretilmez. Sunucu kabul ettiği job'ı socket kapansa
  da sürdürür; eski revision için biten sonuç yeni senaryonun güncel
  sonucu olarak otomatik etkinleştirilmez.
- Yatay CRS, düşey referans, yükseklik birimi ve yerel sahne orijini açık
  sözleşmelerdir. Kaynak/hesap koordinatları f64 hassasiyetini korur;
  render aktarımında yerel orijine göre f32 kullanılabilir. Büyük
  koordinat ve 2D/3D dönüşüm fixture'ları gerekir. Z koordinatı bulunması
  bütün mekânsal işlemlerin 3D semantiği sağladığı varsayımına dönüşmez;
  her işlemin boyutu ve doğruluğu ayrı doğrulanır.
- Sahne görünür bölge/mesafe/önceliğe göre aşamalı yüklenir; LOD,
  görünürlük elemesi, uygun tekrarlar için instancing ve sınırlı cache
  kullanır. Bütün kent mesh'lerini RAM/GPU'ya almayın. Ağ, çözümleme,
  worker, GPU upload ve GPU bellek ayrı bütçelenir; kullanılmayan
  varlıklar sahiplik kurallarıyla serbest bırakılır.
- Cihaz yetenekleri kontrol edilir; destek yetersizse açıklayıcı durum
  ve kullanılabilir 2D çalışma alanı sunulur. 3D açılış hatası mevcut
  2D projeyi veya kaydedilmemiş değişikliklerini kapatmaz.

### 22.3 3D uygulamaya alındığında kabul kapıları

1. Aynı sürümde yalnız 2D proje açıldığında 3D'ye özel ağ isteği/worker
   oluşmaz; 3D komutu ilk kullanımda modülü ve gerekli bölgeyi yükler.
2. Aynı parsel/parametre fixture'ı native ve WASM'da belgelenmiş
   toleransla aynı geometri ve hesap değerlerini verir.
3. Plan revision'ı değişince ilgili sonuç eski görünür; yeniden üretim
   doğru kapsamı günceller, elle düzenlemeler ve başka senaryolar korunur.
4. Üretim sırasında bağlantı kopması, worker yeniden başlaması ve modül
   kapanması job/sonuç kimliğini veya kaydı kaybettirmez.
5. Büyük sahne ve tekrarlı 2D/3D geçişleri belirlenmiş kare süresi,
   ilk açılış ve CPU/GPU bellek bütçelerini geçer; feature kimliği,
   seçim, koordinat hassasiyeti ve autosave tutarlı kalır.

## 23. Kadastral ve mülkiyet hesaplarında sayısal doğruluk

**Bağlayıcı öncelik:** Kaynak sınır, koordinat, alan, hisse ve hak
hesabının doğruluğu performanstan önce gelir. Görsel yaklaşıklaştırmalar
serbest olabilir; mülkiyeti etkileyen kaynak işlemde sessiz yuvarlama,
koordinat kaydırma veya yaklaşık sonucun kesin gibi kaydı yasaktır.
Bu bölüm §5 ve diğer bölümlerdeki genel f64/tolerans ifadelerini
kadastral işlemler için tamamlar ve çelişki halinde önceliklidir.
Hiçbir veri tipi veya ortak Rust kodu tek başına "sıfır hata" kanıtı
sayılmaz; aşağıdaki kapılar geçmeden kadastral kullanıma hazır denmez.

### 23.1 Sayı temsili ve kayıpsız veri yolu

- Kaynaktan gelen koordinatların özgün ondalık değerleri, birimleri,
  CRS/datum bilgisi ve kaynak hassasiyeti korunur. İçe aktarmada erken
  `Number`/f64 dönüşümüyle kaybolmuş basamaklar sonradan decimal'a
  çevirerek geri kazanılmış sayılmaz. Kesin ondalık koordinat gerektiren
  kaynaklarda §15'in `source_kind` sözleşmesi genişletilir: sürümlü
  kesin koordinat tanımı kaynak, PostGIS geometry sorgu/render türevidir;
  çift yönlü bağımsız yazma yolu açılmaz.
- Geometri algoritmaları için f64 kullanılabilir; fakat hata analizi,
  sağlam geometrik kararlar ve gereken yerde adaptif/yüksek hassasiyetli
  hesap yolu zorunludur. Tüm geometriyi decimal'a çevirmek tek başına
  kesişim, trigonometrik işlem veya topoloji doğruluğunu çözmez.
- Kayıtlı resmî alan, kesin ondalık katsayılar ve nihai ondalık sonuçlar
  PostgreSQL `NUMERIC` ve uyumlu Rust decimal temsiliyle taşınır.
  Hisse pay/payda olarak kesin rasyonel tutulur; 1/3 sonlu ondalığa
  zorlanmaz. Tip kapasitesi, taşma, ölçek ve aritmetik kuralları açıktır.
  API/WASM/IndexedDB aktarımında bu değerler decimal string veya
  pay/payda sözleşmesiyle korunur; JavaScript `Number` üzerinden geçmez.
- DB kolon ölçeğinin örtük yuvarlamasına güvenmeyin. Ölçek aşımı
  uygulama katmanında doğrulanır; yalnız tanımlı politika dönüştürebilir.
  NaN/Infinity, taşma ve geçersiz birim kontrollü hata verir.
- Geometriden hesaplanan alan, kaynaktan alınan kayıtlı alan ve
  yuvarlanmış gösterim ayrı alanlar/anlamlardır. Birbirini otomatik
  güncellemez; fark ve hesap yöntemi kullanıcıya açıklanabilir olmalıdır.

### 23.2 Yuvarlama tek, açık ve sürümlü bir iş kuralıdır

- İşlem türüne göre `numeric_policy_id/version`; birim, çıktı ölçeği,
  ara işlem hassasiyeti, yuvarlama noktaları, eşitlikte yön, negatif
  sayılar ve dağıtım artığı davranışı tanımlanır. Evrensel olarak
  "iki ondalık" veya tek bir yuvarlama modu seçmeyin. Uygulanacak
  kurumsal/resmî kural doğrulanıp sürümlenmeden nihai işlem açılmaz.
- Gösterim basamakları kaynak değeri değiştirmez. `ctx.format` yalnız
  sunum yapar; onun çıktısı hesap girdisi olarak geri okunmaz. Düzenleme
  ve tam değer kopyalama kaynağı korur. `toFixed`, `Math.round`, SQL
  `round` ve decimal kütüphanesinin varsayılanları iş kuralı olamaz.
- Ara adımlarda gereksiz yuvarlamayı ve çift yuvarlamayı önleyin.
  Kural açıkça ara adım yuvarlaması istiyorsa o adım uygulanır ve
  denetim izine girer. Sonsuz ondalıklı/irrasyonel sonuçlarda gerekli
  hassasiyet ve hata sınırıyla nihai yuvarlama kararını doğrulayın.
- Hisse ve alan dağıtımında toplam korunumu ayrı invariant'tır.
  Yuvarlama artığını rastgele son parsele veya hissedara vermeyin;
  varsa onaylı dağıtım kuralını deterministik uygulayıp farkı kaydedin.
  Kural yoksa uyuşmazlık çözülmeden nihai kayıt tamamlanmaz.

### 23.3 Geometrik kararlar, toleranslar ve dönüşümler

- Yönelim, kesişim, çakışma, iç/dış ve ortak sınır kararlarında sağlam
  (robust/adaptive veya gerektiğinde exact) predicates kullanın.
  Kesin predicate, üretilen kesişim koordinatının da kesin olduğu
  anlamına gelmez; constructions ayrıca doğrulanır. Yakın paralel,
  teğet, çok kısa kenar ve büyük koordinatlarda sabit epsilon ekleyerek
  hata gizlemeyin.
- Piksel seçim toleransı, kullanıcı kenet işlemi, topoloji birleştirme
  toleransı, hesap hata sınırı, kaynak ölçü belirsizliği ve rapor
  basamakları ayrı kavramlardır. Yakın iki sınır otomatik aynı kabul
  edilmez. Snap/grid/simplify/geometry repair kaynak değiştiriyorsa
  açık komut, önizleme, audit ve geri alma gerekir; okuma/kayıt sırasında
  gizli tamir uygulanmaz. Ortak parsel sınırları tutarlı güncellenir;
  boşluk, çakışma ve alan değişimi doğrulanmadan commit edilmez.
- Alan/uzunluk yöntemi düzlemsel, elipsoidal veya 3D olarak açıkça
  seçilir; ekran projeksiyonundan ölçüm yapılmaz. Eğri alan/uzunluğu
  render tessellation'ından alınmaz. Yerel orijin, kararlı toplama ve
  gerektiğinde yüksek hassasiyetli yöntemler bağımsız referansla sınanır.
- CRS dönüşümü kaynak/hedef datum, eksen sırası, birim, gerekiyorsa
  epoch, kullanılan grid ve dönüşüm sürümünü kaydeder. Gerekli grid
  eksikse daha düşük doğruluklu dönüşüme sessiz fallback yasaktır.
  Gidiş-dönüş testinin geçmesi tek başına mutlak doğruluk kanıtı değildir.
- Karar/yuvarlama eşiğini kesen hata aralığında hesap hassasiyeti
  artırılır. Kaynak belirsizliği veya algoritma sınırı nedeniyle karar
  yine doğrulanamıyorsa sonuç `needs_review` olur; nihai komut atomik
  olarak reddedilir veya inceleme bekler. İnceleme sayısal belirsizliği
  sihirli biçimde ortadan kaldırmaz; çözüm ve dayanak kayda girer.

### 23.4 Bağımsız doğrulama ve üretim engelleri

- Native/WASM eşitliği gereklidir ama aynı hatayı paylaşabilirler.
  Bağımsız yüksek hassasiyetli referanslar, analitik örnekler ve
  uzmanlarca doğrulanmış kontrol verileriyle sonuçları karşılaştırın.
- Tam yarım değer, eşiğin iki yanı, negatif değer, çok büyük/küçük
  koordinat, delik, eğri, ortak kenar, yakın paralel/teğet, hisse
  toplamı ve bölme/birleştirme alan korunumu fixture'ları zorunludur.
  DB/API/WASM/IndexedDB/export round-trip testleri kaynak hassasiyetini
  ve kesin ondalık/rasyonel değerleri korumalıdır.
- Native/WASM geometrik ara sonuçları belgelenmiş hata sınırında
  olabilir; fakat nihai ondalık sonuç, yuvarlama yönü ve topolojik
  karar aynı olmak zorundadır. Farkta sunucu sonucu sessizce seçilmez;
  işlem engellenir ve sürüm/hata araştırılır. Deterministik işlem sırası,
  paralel toplamlar ve derleyici optimizasyonları da doğrulanır.
- Denetim kaydı kaynak revision'ları/hash'leri, algoritma/build sürümü,
  sayısal politika, CRS dönüşümü, yuvarlama öncesi sonuç/hata sınırı,
  nihai sonuç ve aktörü içerir; işlem yeniden üretilebilir olmalıdır.
  Onaysız politika, doğrulanamayan eşik, sınır kayması veya açıklanamayan
  alan/hisse farkı üretim engelidir. Mevcut sabit toleransları büyütmek
  ya da test beklentisini değiştirmek bu kapıyı geçirmez.

## 24. Korunan CBS ürün kapsamı ve operasyon sözleşmeleri

Bu bölüm önceki genel CBS planındaki tamamlayıcı gereksinimleri mevcut
CAD deposuna uyarlar. Tamamlanmış özellik listesi değildir; §19'un
fazları içinde ihtiyaç duyulan dikey dilime eklenir. Paket/framework
adayları zorunlu bağımlılık sayılmaz; mevcut DOM/Signal, `.kstil`,
WebGL2/WebGPU ve §13–23 kararları korunur.

### 24.1 Veri kataloğu, sağlayıcı ve şema

- `Connection`, `Dataset`, `DatasetView`, `ProjectLayer`, alan/domain,
  ilişki, form ve publication tanımları ayrıdır. Aynı dataset farklı
  projelerde farklı stil/formla kullanılabilir. Tenant güvenlik sınırıdır;
  proje/çalışma alanı paylaşımı kaynak erişimini kendiliğinden genişletmez.
- Provider şema, geometri tipi, Z/M, CRS, extent, kararlı kimlik, revision
  ve `query/read/edit/transaction/tile/identify/export/schema_edit`
  yeteneklerini bildirir. Yönetilen PostGIS, haricî table/view ve uzak
  harita kaynakları aynı yeteneklere sahip varsayılmaz. Güvenilir kimliği
  veya açık yazma eşlemesi olmayan view düzenlemeye açılmaz.
- Yönetilen kayıtlarda tenant, kalıcı UUID, sürüm, oluşturma/güncelleme
  zamanı ve iç kullanıcı kimlikleri bulunur. Sayısal bigint kullanılıyorsa
  API'de decimal string taşınır. Tipli iş alanları normal kolonlarda,
  yapılandırılmış ilave veri JSONB'de tutulur; aynı anlam iki yerde
  bağımsız kaynak olamaz. `hstore` yalnız somut metin anahtar/değer veya
  uyumluluk ihtiyacı varsa eklenir; bütün veriye zorunlu değildir.
- Kodlu/aralık domain, subtype, bağımlı alan kombinasyonları, lookup ve
  ilişkiler sürümlenir. Bölme/birleştirme/silmenin öznitelik ve ilişkili
  kayıtlara etkisi açık politikadır; alan/hisse dağıtımında §23 geçerlidir.
- Şema değişimi önce veri kaybı, form/stil/yayın/ilişki bağımlılıkları ve
  kilit/süre etkisini gösteren migration planı üretir. Kontrolsüz DDL veya
  sessiz kolon silme yoktur. Şema revizyonu edit ve autosave ile denetlenir.
- Büyük öznitelik tablosu sunucuda filtre/sıralama ve kararlı sayfalama
  kullanır; tüm kayıtları tarayıcıya almaz. Büyük seçimler yetki ve veri
  revision'ına bağlı sorgu/selection token ile temsil edilebilir.

### 24.2 Form, ifade, workflow, layout ve ajan arayüzü

- Form veri şeması, yerleşim ve davranış tanımları ayrı sürümlenir.
  Domain/decimal/tarih/dosya, sayfalı lookup, ilişkili alt form ve
  haritadan nesne seçimi desteklenir. Koşullu görünürlük yetki yerine
  geçmez; alan ve işlem izni sunucuda doğrulanır. Form tasarımcısı lazy
  yüklenir; sıradan nesne inceleme bütün tasarımcıyı indirmez.
- İş kuralı ifadeleri ortak Rust çekirdeğinde native/WASM çalışır;
  tip/null/decimal/birim/saat dilimi/hata semantiği sürümlüdür. Ağ,
  dosya, keyfî SQL veya `eval` erişimi yoktur; işlem ve derinlik bütçesi
  vardır. MapLibre ifade diliyle aynı dil olduğu varsayılmaz.
- Workflow tanımı tipli giriş/çıkışlı DAG, veri/komut/kural revision'ları,
  ara çıktı, retry/iptal ve provenance içerir. Sunucu job'ının yaşam
  döngüsü tasarımcıdan bağımsızdır. Layout/pafta; sayfa, ölçek, harita
  çerçevesi, lejant ve çıktı tanımlarını proje verisi olarak saklar;
  büyük export sunucu job'ıdır. İkisi de ayrı lazy özelliklerdir.
- Her ürün işlevi yetki kapsamında UI, komut, MCP ve chat üzerinden
  kullanılabilir olmalıdır. Komut şeması sürüm, girdi/çıktı, capability,
  önkoşul, preview, idempotency, iptal ve undo/compensation taşır.
  Headless çalışamayan görsel komut bunu açıkça bildirir.
- Model sağlayıcısı adaptörle ayrılır; ajan şemalı komut çağırır.
  Proje/seçim bağlamı yetkiyle filtrelenir; veri metni talimat sayılmaz.
  Ajan doğrudan SQL/DOM yoluyla iş kurallarını aşamaz. Yüksek etkili
  toplu silme, paylaşım ve şema değişimi etki önizlemesi ve ürün onay
  politikası gerektirir; sıradan yetkili işlemlerde gereksiz onay yoktur.

### 24.3 Yayın sihirbazının tam akışı

Taslak kaydedilip devam ettirilebilir; aynı adımlar MCP/komutla erişilir:

1. Tenant/proje ve izinli kaynak/dataset/view seç.
2. Kimlik, geometri, CRS, boyut ve alan şemasını doğrula.
3. Yayınlanacak alan, filtre ve gerçekten desteklenen query/edit/export
   yeteneklerini belirle; gizli alanlar varsayılan yayınlanmaz.
4. Rol/grup, alan/satır, public/private ve paylaşım kapsamını belirle.
5. Zoom/LOD, tile boyutu/süre sınırı, cache ve güncellik profili seç.
6. Kaynak `.kstil` ve form revision'ını bağla; dış MapLibre uyumluluk
   raporunu ve gerekli sprite/glyph varlıklarını doğrula.
7. İndeks/sorgu planı, örnek tile, farklı roller ve yetkisiz erişimi sına.
8. Sonuç URL'leri, kapsam ve maliyet özetinden sonra değişmez revision
   oluştur; hazır olduğunda aktif işaretçiyi atomik değiştir.

İlk motor Martin'dir. İndeks/materialized view önerisi ayrı migration
planıdır; yayın komutu gizli DDL çalıştırmaz. Pause/archive/yetki iptali
cache ve katalogda uygulanır. Yapılandırmayı geri almak veriyi geçmişe
döndürmez. Haricî DB değişiklikleri için trigger/change feed veya açık
polling+TTL güncellik profili seçilir; LISTEN/NOTIFY kalıcı günlük değildir.

### 24.4 Operasyon, güvenlik ve ileri çevrimdışı kapsam

- OIDC oturumu; issuer/audience/imza/süre ve key rotation doğrulaması,
  uygun PKCE/BFF ve cookie/CSRF politikası içerir. Kaynak sırları tarayıcı,
  log veya MCP çıktısına verilmez. Uzak kaynak proxy'sinde SSRF, redirect,
  boyut ve süre sınırları uygulanır. SQL değerleri bind, identifier'lar
  izinli katalogdan seçilip doğru quote edilir.
- Request -> command -> job -> SQL -> tile akışı correlation ID ile
  izlenir. Yapısal log/trace/metric; pool beklemesi, sorgu, cache, job
  backlog, lease kaybı, reconnect ve autosave hatasını kapsar. Token ve
  hassas koordinat/öznitelik loglanmaz; audit erişimi ayrı yetkidir.
- Docker/Compose geliştirme ve üretim dağıtımı, ayrı API/worker ölçeği,
  health/readiness, graceful shutdown, queue/pool drain ve kontrollü
  migration tanımlanır. Tile yükü edit/commit işlerini aç bırakmaz;
  bağlantı/CPU bütçeleri ayrılır. Yedek/PITR için hedef RPO/RTO belirlenir
  ve restore tatbikatıyla doğrulanır; ölçülmeden SLA iddia edilmez.
- Worker `auto/embedded/external` seçimi gerekirse aynı kalıcı kuyruk ve
  lease/fencing üzerinden yapılır. Dış worker yokken sınırlı embedded
  executor yalnız açık ayarla açılır; failover mükerrer commit üretmez.
- Gelecek masaüstü/tam offline paket için taşıma bağımsız servis,
  kalıcı kimlik, schema revision, tombstone ve değişiklik günlüğü korunur.
  Tauri/Electron ve yerel veri adaptörü ayrıca ADR ile seçilir; bulut
  PostgreSQL kararı değişmez. §21'deki bağlantı kaybı/kurtarma desteği tam
  offline replica değildir. Tam replica; veri kapsamı, indirme izni,
  sync cursor, çatışma, yerel erişim süresi ve platform testleri ister.
- GEOS/GDAL/PROJ ve GPU compute gibi ileri sağlayıcılar ölçülen ihtiyaçla
  eklenir. Lisans, FFI/süreç sınırı, capability, doğruluk ve native/WASM
  kapsamı doğrulanmadan destekleniyor denmez. GPU görüntüsü veya yaklaşık
  analiz, §23 kadastral hesabının doğruluk sözleşmesini değiştiremez.

Resmî referanslar (uygularken sürüm sabitle):
PostgreSQL numeric/floating-point ve yuvarlama: https://www.postgresql.org/docs/current/datatype-numeric.html ;
Robust predicates, Shewchuk: https://www.cs.cmu.edu/~quake/robust.html ;
PostGIS veri/curve türleri: https://postgis.net/docs/using_postgis_dbmanagement.html ;
ST_CurveToLine: https://postgis.net/docs/ST_CurveToLine.html ;
ST_AsMVTGeom: https://postgis.net/docs/ST_AsMVTGeom.html ;
ST_AsMVT: https://postgis.net/docs/ST_AsMVT.html ;
PostgreSQL RLS: https://www.postgresql.org/docs/current/ddl-rowsecurity.html ;
PostgreSQL queue claim: https://www.postgresql.org/docs/current/sql-select.html ;
Martin yayın: https://maplibre.org/martin/using/ .
Martin stil/sprite/font: https://maplibre.org/martin/sources-styles/ ;
Martin kütüphane: https://maplibre.org/martin/martin-as-a-library/ ;
MapLibre stil katmanları: https://maplibre.org/maplibre-style-spec/layers/ .
