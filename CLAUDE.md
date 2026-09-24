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

```bash
pnpm install
pnpm dev                  # Vite geliştirme sunucusu
pnpm build                # tsc (tip denetimi) + vite build
pnpm test                 # Vitest birim testleri (geometri, işlemler, belge, biçimlendirici)
pnpm e2e                  # Başsız Chrome'da uçtan uca duman testi (kendi Vite sunucusunu açar)
npx tsc --noEmit -p .     # yalnızca tip denetimi
pnpm rust:test            # Rust çalışma alanı: cargo test + clippy (-D warnings)
pnpm wasm                 # WASM paketi kaynak değiştiyse derlenir (dev/test/build/e2e bunu kendileri çalıştırır)
pnpm rust:wasm            # geometri çekirdeğinin WASM paketi → src/wasm/pkg (depoya girmez)
pnpm rust:wasm:formats    # dosya biçimlerinin WASM paketi → src/io/pkg (depoya girmez; yalnız içe/dışa aktarmada yüklenir)
pnpm test:rust            # rust:test + rust:wasm + WASM golden testleri
pnpm db:setup             # kentosd db-setup + migrate + dev-seed (yerel PostGIS'te kentos_cad, iki rol, örnek kurum)
pnpm api                  # kentosd serve: 127.0.0.1:8787 (veritabanı yoksa yalnızca /v1/health)
pnpm kentosd -- <komut>   # yönetim: tenant add|list, user add|password, member add|list, migrate, dev-seed
pnpm e2e:cloud            # gerçek sunucu ve veritabanıyla bulut akışı (giriş, yükleme, otomatik kayıt, çakışma, kopma)
```

- **API bağlantısı:** `vite` ve `vite preview`, `/v1/` isteklerini ve proje WebSocket'ini (`/v1/ws`) `vite.config.mjs` içindeki küçük bir eklentiyle `127.0.0.1:KENTOS_API_PORT` (varsayılan 8787) adresine iletir. API çalışmıyorsa sessizce 503 döner. Durum çubuğu “Sunucu: bağlı / yok / uyumsuz” gösterir; yerel çizim sunucuya hiç bağlı değildir.
- **Sunucu ayarları** (`kentosd`, önce ortam değişkeni, sonra `.env.local`; `.env.local` depoya girmez, `0600`): `KENTOS_DATABASE_URL` (sunucu rolü), `KENTOS_DATABASE_OWNER_URL` (migration ve yönetim), `KENTOS_PUBLIC_URL` (tarayıcının adresi; WebSocket kaynak denetimi ve OpenID dönüşü, varsayılan `http://localhost:5173`), `KENTOS_COOKIE_SECURE`, `KENTOS_LOCAL_LOGIN`, OpenID için `KENTOS_OIDC_ISSUER`, `KENTOS_OIDC_CLIENT_ID`, isteğe bağlı `KENTOS_OIDC_CLIENT_SECRET`, `KENTOS_OIDC_AUDIENCE`, `KENTOS_OIDC_LABEL`. Veritabanı testleri `KENTOS_TEST_ADMIN_URL` ile geçici `kentos_cad_test_*` veritabanları açar; sunucu yoksa atlanır, `KENTOS_TEST_DB=required` bunu hata sayar (ADR 0006, 0007).

- **Rust araç zinciri** `rust-toolchain.toml` ile sabittir (wasm32 hedefi dahil); derleme `.cargo/config.toml` ile 4 işle sınırlıdır. WASM paketi için `wasm-bindgen` komutu crate sürümüyle aynı olmalıdır: `cargo install wasm-bindgen-cli --version 0.2.128 --locked`. **Uygulama geometriyi Rust çekirdeğinden (WASM) alır** (ADR 0008): `pnpm dev`, `test`, `build` ve `e2e` önce `scripts/wasm/ensure.mjs`'i çalıştırır; çekirdeğin kaynakları değiştiyse paket `nice` ile yeniden derlenir (`src/wasm/pkg/.stamp`). Aynı betik dosya biçimleri paketini (`crates/formats`, `crates/formats-wasm`, `crates/contracts` → `src/io/pkg`) ayrı özet ve damgayla derler: biri değişince öbürü derlenmez, hiçbiri değişmediyse hiçbir şey derlenmez. Bu yüzden Rust araç zinciri bunların hepsi için gereklidir. Sayfa çekirdeği uygulamadan önce başlatır (`src/wasm/core.ts`), worker derlenmiş modülü ilk işiyle alır. Cargo derlerken e2e ya da başka bir ağır iş çalıştırılmaz (ADR 0001).

- **Çizim motoru:** varsayılan WebGL2'dir; WebGPU isteğe bağlıdır.
  - Etkin motor durum çubuğunun sağ alt köşesinde yazar. Tıklayınca motor seçilir: seçim hemen uygulanır (`view.switchBackend`, sayfa yenilenmez) ve `prefs.rendererPreference` ile hatırlanır. Aynı seçim **Görünüm → Çizim motoru** menüsünde ve Uygulama ayarları → Çizim motoru bölümünde de vardır.
  - `?renderer=webgpu` ya da `?renderer=webgl2` URL parametresi açılışta kayıtlı tercihi geçersiz kılar.
  - WebGPU başlatılamazsa WebGL2'ye düşülür ve uyarı yazılır.
- Her değişiklikten sonra `tsc` temiz olmalı ve `pnpm test` geçmeli (bkz. §9.4).
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
- **Araç zinciri:** Vite 8, TypeScript 6, pnpm. Hedef ES2023.
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
| `style/`      | Stil motoru: semboller ve sembol katmanları, katman stilleri (işleyiciler), kitaplık (sistem/kullanıcı/proje, kategori ağacı), .kstil dosyaları, sembol × geometri → çizim ilkelleri (bkz. [docs/STYLE.md](docs/STYLE.md)) | core, geo, model                            | DOM, render, viewport, tools, ui, app  |
| `processing/` | İşlem araçları: bildirimsel tanım, parametreler, kayıt, çalıştırıcı, modeller (bkz. [docs/PROCESSING.md](docs/PROCESSING.md))                                                                                              | core, geo, model                            | DOM, render, viewport, tools, ui, app  |
| `render/`     | `RenderBackend` sözleşmesi, sahne verisi, WebGL2 ve WebGPU arka uçları                                                                                                                                                     | core, model (tip + stil)                    | ui, tools, viewport                    |
| `viewport/`   | Kamera, seçme ve kenetleme dizini, 2B üst katman, çizim döngüsü                                                                                                                                                            | core, model, render, tools (tip), app (tip) | ui                                     |
| `tools/`      | Etkileşimli araçlar ve araç kataloğu                                                                                                                                                                                       | core, model, viewport (tip), app (tip)      | ui                                     |
| `ui/`         | Bileşenler, paneller, pencereler, widget'lar                                                                                                                                                                               | hepsi (servisler `AppContext` üzerinden)    | model'i doğrudan değiştirmek (bkz. §8) |
| `app/`        | Kompozisyon kökü, komutlar, menüler, kısayollar, durum depoları, biçimlendirici                                                                                                                                            | hepsi                                       | —                                      |

**`io/`** (dosya alışverişi: biçim worker'ı `formatsWorker.ts`, sayfa tarafı `client.ts`, okunan nesneleri belgeye koyan `apply.ts`, koordinat listesi seçenekleri `coords.ts`; `pkg/` üretilir) `processing` ile aynı düzeydedir: core, geo, model ve contracts'ı içe aktarır, DOM'a dokunmaz; pencereleri `ui/io/`'dadır. Biçimlerin kendisi Rust'tadır (`crates/formats`, §9.7). `io/` ve `ui/io/` başlangıç paketine girmez, komut çalışınca yüklenir (§20).

**`wasm/`** (Rust geometri çekirdeğinin tarayıcı cephesi: `core.ts` başlatma ve `op()` çağrıları; `pkg/` üretilir) modelin altındadır: `model` ve sağındaki her katman onu içe aktarabilir, o yalnızca `pkg/`'yi içe aktarır. Parity ve fixture testleri (`wasm/parity/`) teste özeldir (ADR 0008).

**`contracts/`** zincirin dışındadır: Rust'tan (`crates/contracts`, ts-rs) üretilen sürümlü sözleşme tipleri (`generated/`, elle düzenlenmez) ve sözleşme sürümü (`version.ts`). Hiçbir şey içe aktarmaz; her katman buradan tip alabilir. Uygulamanın kendi tipleri sözleşmeye `contracts.test.ts`'te derleme anında denetlenir (ADR 0002).

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
| `files`      | `DocumentFiles`      | Yerel çizim dosyası (.kcad): kaydet, farklı kaydet, aç; kaydedilen dosyanın tutamacı; içe aktarılacak dosyanın seçimi (`pickForImport`); dosya pencereleri (`picker`, duman testinde bellek içi) (`app/fileIO.ts`) |
| `server`     | `ServerStatus`       | API'nin yanıt verip vermediği (`/v1/health`, üretilen `Health` sözleşmesiyle doğrulanır; sözleşme sürümü farklıysa “uyumsuz”) (`app/server.ts`) |
| `cloud`      | `CloudSession`       | Oturum, açık bulut projesi, otomatik kayıt (`ProjectSync`) ve canlı olaylar (`ProjectSocket`) (`app/cloud/`) |

İleride birden fazla belge açılacaksa, belgeye bağlı servisler (`format`,
`view` içindeki önbellekler) belge değişince yeniden kurulmalıdır. Bunun için
`doc` doğrudan önbelleğe alınmaz; her seferinde `ctx.doc` üzerinden okunur.

### 4.4 Durum kapsamları (en önemli ayrım)

Yeni bir ayar ya da durum eklemeden önce **hangi kapsama ait olduğuna** karar verin:

| Kapsam                      | Nerede                                                      | Saklama                          | Kim görür                           | Örnekler                                                                                                                                                                                                                                                                              |
| --------------------------- | ----------------------------------------------------------- | -------------------------------- | ----------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **Proje ayarları**          | `model/projectSettings.ts` → `doc.settings`                 | Proje dosyası (.kcad)            | Projeyi açan herkes                 | SRID, uzunluk ve alan hassasiyeti, alan birimi, açı birimi, çizim ölçeği, proje adı                                                                                                                                                                                                   |
| **Belge verisi**            | `CadDocument`, `LayerStore`                                 | Proje dosyası                    | Projeyi açan herkes                 | Varlıklar, katman ağacı ve stilleri (işleyiciler dahil), nesne sembolleri, öznitelikler, projenin stil kitaplığı (`doc.styles`)                                                                                                                                                       |
| **Uygulama ayarları**       | `app/state.ts` → `ctx.prefs`                                | `localStorage` `kentos.prefs.v1` | Yalnızca bu kullanıcı, tüm projeler | Tema, yazı boyutu, artı imleç, fare yardımcıları (imleç yanında giriş, bilgi kartı), kenet türleri ve yarıçapları, çizim motoru, sembol boyutu (çizim ölçeğinde / ekranda sabit), **yeni proje varsayılan SRID'si (5256)**; işlem araçlarının son değerleri (`kentos.processing.v1`); kullanıcının stil kitaplığı (`kentos.styles.v1`) |
| **Çalışma alanı yerleşimi** | `app/state.ts` → `ctx.ui`                                   | `localStorage` `kentos.ui.v1`    | Yalnızca bu kullanıcı               | Panel genişlikleri, araç kutusu konumu, sütun sayısı ve katlanan grupları, açık sekme, sağ dok sekmesi (Katmanlar/İşlemler), İşlemler görünümü ve katlanan kategoriler                                                                                                                |
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
- **Henüz yapılmamış özellikler** `pending(...)` ile kaydedilir ve dürüstçe uyarı verir. Sessizce hiçbir şey yapmayan düğme olmaz.
- **Menü modeli** `app/menus.ts` içindedir. Menü öğeleri komut kimliğidir; başlık, simge, kısayol ve durum komuttan çözülür.

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

- **`tools/catalog.ts`:** tek bildirimsel liste. Her araç bir kimlik, etiket, simge, grup (`select | draw | annotate | transform | modify | map`), kısayol, takma adlar, açıklama, **fareyle kullanım adımları** (`steps`) ve `create(ctx)` içerir. Araç kutusu, menüler, kısayollar, ipuçları ve komut satırı bu listeden üretilir. Yeni araç `steps` olmadan eklenmez: kullanıcı aracı fareyle nasıl kullanacağını ipucundan öğrenir.
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
- **Araçlar DOM'a dokunmaz.** Görünüm alanına `ctx.view` üzerinden erişir: `pick`, `pickEdge`, `pickRect`, `edgesIn`, `gripAt`, `worldTolerance`, `requestOverlay`, `camera`.
- **Araç aileleri** (yeni araç yazarken birine dahil edin):

  | Aile                  | Dosya                                                                                                                 | Akış                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                 |
  | --------------------- | --------------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
  | `PointInputTool`      | `tools/drawTools.ts`, `tools/curveTools.ts`, `tools/shapeTools.ts`, `tools/parallelTool.ts`, `tools/annotateTools.ts` | Nokta dizisi: çizgi (G geri, K kapat), çoklu çizgi (`tools/pathTool.ts`; Y yay parçası: teğet ya da tek parça için A açı, M merkez, R yarıçap, İ ikinci nokta, T doğrultu; D düz; U son doğrultuda uzunluk), halka ve revizyon bulutu (`tools/markupTools.ts`), paralel çizgi (eksen noktaları; S sol, A sağ mesafe yazılır ya da iki tıkla gösterilir, E eksen, U alan olarak, K kapat), alan, nokta, ölçüm, parsel; dikdörtgen (köşe yuvarla/pah, döndür, boyutlar), döndürülmüş dikdörtgen (kenar → genişlik), düzgün çokgen (içten, dıştan, kenardan); daire (merkez-yarıçap/çap, 2N, 3N, TTY, TTT), yay (AutoCAD'in tüm yöntemleri, bkz. §10), eğri; yazı, ölçü |
  | Ölçü                  | `tools/dimensionTool.ts`                                                                                              | Tür başta tek tuşla seçilir: Hizalı (H), Doğrusal (D; yön imlecin yerinden, `Y` yatay ΔY, `X` düşey ΔX sabitler, `O` serbest bırakır), Açı (A; iki kenara tıklanır ya da `K` ile köşe ve iki kol; yayın konduğu bölge açıyı seçer), Yarıçap (R), Çap (Ç). Yerleştirme adımında yazılan sayı ötelenmeyi (açıda yarıçapı) tam verir                                                                                                                                                                                                                                                                                                                                    |
  | Referans hat          | `tools/perpTools.ts`                                                                                                  | Önce bir hatta tıklanır (başlangıç A, tıklamaya yakın uç), sonra ona göre çalışılır: dik in (her nokta hatta dik iner; yay ve dairede merkeze), dik çık (dik ayak tıklanır ya da yazılır, dik boy gösterilir ya da yazılır, sağa artı)                                                                                                                                                                                                                                                                                                                                                                                                                               |
  | Tek tık               | `tools/hatchTool.ts`, `tools/areaTools.ts`                                                                            | Tarama: “Sınır: kapalı nesne” (varsayılan, Netcad gibi) tıklanan yeri çevreleyen en küçük kapalı nesneyi doldurur; içindeki ya da kenarına taşan, ondan küçük kapalı nesneler (parseldeki bina) ada olur ve taranmaz. “Sınır: çizgiler” (B, AutoCAD gibi) görünür çizgilerin kapattığı yüzü doldurur, içteki gruplar ada olur; sınır tek katmana daraltılabilir (K). “Adalar” (A) adaları kapatır. İçine tıklayarak alan aynı yüzleri kullanır (`tools/visibleFaces.ts`: görünüm, çizim ya da katman görünürlüğü değişince yeniden kurulan önbellek)                                                                                                                 |
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
- **Nokta hesabı** (`tools/pointCalc.ts`, Netcad'in Koordinat hesap makinası): nokta beklenirken komut şeridindeki "Nokta hesabı" düğmesi, basılı sağ tık menüsü ya da komut satırında takma adla açılır: yan nokta (YAN: dik ayak, dik boy sağa artı), kenar kesişimi (KKES: iki uzaklık; iki çözümden biri tıklanır), doğru kesişimi (DKES: 4 nokta), hat üzerinde nokta (HAT: uzaklık ya da a/b), açı-mesafe (AM: bakılan doğrultudan saat yönünde, proje açı biriminde), iki nokta ortası (ORTA). Geometri `model/geom/survey.ts`'tedir. Menüde (`ui/shell/calcMenu.ts`) her yöntemin krokisini gösteren bir simgesi (tıklanan noktalar tutamaç, hesaplanan nokta halka) ve ne hesapladığını, nereye tıklanıp ne yazılacağını söyleyen bir açıklama satırı vardır; kullanıcı yöntemi adından tanımak zorunda kalmaz.
- **Katalog dışı araçlar** (`ctx.tools.run(tool, label)`): içeriği o anki duruma bağlı olan araçlar (ör. panodaki nesnelerle `PasteTool`) katalogda durmaz, "son komutu yinele"ye girmez.

- **İmleç kısıtlaması** tek yerdedir (`tools/tracking.ts` → `constrainPoint`). Öncelik sırası: nesne keneti, nesne izleme, orto (Shift tersine çevirir), kutupsal izleme (F10, adım `prefs.polarIncrement`). Kutupsal kilit ışına 10 px yaklaşınca devreye girer.
- **Nesne izleme** (`viewport/objectTracking.ts`, saf ve testli; `settings.tracking`, Shift+F3, durum çubuğunda "İzleme"): nokta bekleyen bir komutta uç, orta, merkez, düğüm, çeyrek ya da kesişim keneti üzerinde 350 ms beklemek o noktayı izleme noktası yapar (yeşil artı, en çok 3). Aynı noktada tekrar beklemek bırakır. İmleç bir izleme noktasının yatay/dikey hizasına (kutupsal açıksa açı adımlarına) 8 px yaklaşınca kilitlenir. İki hizanın kesişimi tek hizadan önce gelir; aracın son noktası (`snapFrom`) yalnızca kesişimlere katılır. Eksen yönleri tam değerle hesaplanır, böylece "tam üstü" aynı X'i verir. Kenet varken izleme uygulanmaz. İzleme varken yazılan tek sayı izleme noktasından hiza boyunca mesafedir: araçlar yazılan noktayı `pointFromText` ile çözer (`parsePointInput` + `view.trackAlong`). İzleme noktaları komut değişince silinir.
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
- **Buda ve uzat** (`tools/edgeTools.ts`, `BoundaryEdgeTool`): sınır varsayılan olarak görünen bütün kenarlardır (AutoCAD hızlı kip). “Sınır seç (S)” ile sınır nesneleri tıklanır, sağ tık onaylar; seçilen sınırlar seçim olarak vurgulu kalır, “Tüm kenarlar (T)” geri döner. Shift+tık öbür işlemi yapar (budada uzatır, uzatta budar).
- **Köşe yuvarla ve pah** (`tools/cornerTools.ts`): imleç bir köşeye (çoklu çizgi köşesi ya da iki çizginin birleştiği uç) 12 px yaklaşınca köşe halkayla işaretlenir. Tıklayınca köşe kilitlenir; imleç bir kenar boyunca çekildikçe teğet/kesim mesafesi canlı büyür (yakınlığa göre yuvarlanmış adımla), ikinci tık uygular. Yazılan değer tam uygular, sağ tık son değeri kullanır. Birleşmeyen iki çizgide sırayla iki çizgiye tıklanır. Araç uygulamadan sonra bir sonraki köşeyi bekler (AutoCAD'in Çoklu kipi). “Kırp (K)” iki araçta ortaktır: kapalıyken kenarlar olduğu gibi kalır, yalnızca yay ya da pah çizgisi eklenir (TRIMMODE=0).
- **Semantik:**

  | Tuş                  | Davranış                                                                       |
  | -------------------- | ------------------------------------------------------------------------------ |
  | Esc                  | Araçtan çıkar, seçime döner. Seçim aracındaysa seçimi temizler.                |
  | Enter / sağ tık      | Geçerli nesneyi bitirir; araçta kalınır. Bekleyen bir şey yoksa araçtan çıkar. |
  | Seçim aracında Enter | Son aracı tekrarlar.                                                           |
  | Boşluk               | Komut satırına gider.                                                          |

- **Koordinat girişi** (`tools/coordinateInput.ts`):
  - `Y,X` mutlak
  - `@dY,dX` göreli
  - `@mesafe<açı` kutupsal (derece, doğudan saat yönünün tersine)
  - tek sayı: imleç doğrultusunda mesafe

  Ondalık ayırıcı nokta, koordinat ayırıcı virgüldür.
- **Harita araçlarının hedef katmanları** (`parsel`, `kot`) katalogdaki `LAYERS` yapılandırmasındadır. Araç kodunda katman kimliği sabit yazılmaz.

### 4.8 Belge modeli (`model/`)

- **`CadDocument`:** varlıklar `Map<id, Entity>` içinde durur. Bütün düzenlemeler `add`, `update`, `remove` ve bunları gruplayan `transact(label, fn)` üzerinden yapılır. Her işlem tersine çevrilebilir bir `Op` olarak kaydedilir; geri alma yığını 200 adımla sınırlıdır. `transact` ya hep ya hiç çalışır: `fn` hata fırlatırsa yaptıkları geri alınır, hiçbir şey kaydedilmez ve `dirty` değişmez. İç içe çağrı bir kayıt noktasıdır; gerekçe [docs/adr/0003-transaction-semantics.md](docs/adr/0003-transaction-semantics.md)'de.
- **Kaydedilmemiş işareti (`dirty`) belgenin sürümünden gelir:** her kayıt, geri alma, yineleme, proje ayarı, ad, stil kitaplığı ve katman ağacı ya da katman durumu değişikliği `doc.revision`'ı artırır. `markSaved(revision)` yalnızca yazılan sürüm hâlâ güncelse işareti temizler; yazım sürerken yapılan değişiklik kaydedilmemiş kalır.
- **Çizim dosyası (.kcad)** sürümlü `DocumentSnapshotV1`'dir (`model/snapshot.ts`, sözleşme `crates/contracts`): nesneler, katman ağacı, proje ayarları, yerel orijin, başlangıç görünümü ve projenin stil kitaplığı. Koordinatlar JSON'da float64 olarak bit bit korunur. Okuyucu biçimi, sürümü ve her alanı doğrular; bilinmeyen sürüm, tür ya da SRID “yer: sorun” biçiminde Türkçe hatayla reddedilir, tahmin edilmez. `doc.replaceWith(içerik)` açık belgeyi yerinde değiştirir (`ctx.doc` aynı nesne kalır), geçmişi siler ve belgeyi temiz başlatır; açık bir işlem ya da grup varken reddedilir.
- **Kaydet/Aç** (`app/fileIO.ts`, `ctx.files`): Kaydet (`Ctrl+S`) açılan ya da son kaydedilen dosyaya sormadan yazar, ilk seferde Farklı kaydet (`Ctrl+Shift+S`) gibi yer sorar; proje dosyanın adını (uzantısız) alır. Tarayıcının dosya penceresi (File System Access API) kullanılır; işaret yalnızca yazıcı hatasız kapanınca temizlenir. Dosyaya yazamayan tarayıcıda çizim indirme olarak verilir ve kaydedilmemiş sayılır, çünkü saklandığı doğrulanamaz. Aç (`Ctrl+O`) kaydedilmemiş değişiklik varsa önce sorar (Kaydet ve devam et / Kaydetmeden devam et / Vazgeç); dosyadaki proje stilleri paylaşılan .kstil gibi doğrulanır. Üretim derlemesinde kaydedilmemiş değişiklikle sekme kapatılırken tarayıcı sorar. Duman testi tarayıcı penceresi yerine bellek içi bir seçici (`kentos.files.picker`) kullanır.
- **Dosya alışverişi** (`app/fileExchange.ts` komutları, `io/`, `ui/io/`, Rust `crates/formats`; §9.7, ADR 0009):
  - Komut önce dosya penceresini açar (tarayıcı kullanıcının tıklamasını ister); pencerenin kodu yanında yüklenir. Dosya ayrı bir Web Worker'da Rust biçim modülüyle okunur.
  - Kaynağın koordinat sistemi her zaman sorulur (“Bu koordinatlar hangi sistemde?”, varsayılan projeninki). Başka bir sistem seçilirse içe aktarma kapanır ve nedeni yazar: datum ve dilim dönüşümü yok, koordinatlar sessizce dönüştürülmez (§5).
  - Okunan nesneler `.kcad` okuyucusunun denetiminden geçer (`readEntityList`); yeni katmanlar kurulur (katman kurmak geri alınmaz, işlem araçlarındaki gibi) ve bütün nesneler **tek geri alma adımı** ve tek değişiklik olayıyla eklenir (`CadDocument.addMany`); görünüm içe aktarılanlara yakınlaşır. Dışa aktarma çizimi kaydetmez, `dirty`'ye dokunmaz.
  - **Koordinat listesi** (Dosya → İçe aktar → Koordinat listesi, Koordinat → Nokta listesi içe aktar; `NCN`): Netcad NCN, TXT, CSV. Ayırıcı (boşluk, sekme, `;`, `,`), ondalık işaret (nokta; `;` ya da sekmeyle ayrılmış Türkçe tablolarda virgül), başlık satırı ve kodlama (UTF-8, UTF-16, Windows-1254) bulunur. Sütunlar (Ad, Y, X, Z, Kod) başlık adlarından ya da Netcad sırasından (Ad Y X Z) sayıların büyüklüğüne göre önerilir (TM'de sağa değerler 10⁶'dan küçük, yukarı değerler büyük); önizleme tablosunun başlığından ya da sıra düğmelerinden değiştirilir ve her seçim dosyayı yeniden okur. Nokta olmayan satırlar satır numarası ve nedeniyle listelenir, alınmaz. Noktalar seçilen ya da dosya adıyla kurulan katmana `label` ve `Ad` (varsa `Kod` ve dosyadaki basamaklarla `Z (m)`) öznitelikleriyle gelir; koordinatlar dosyadaki ondalığa en yakın float64'tür, yuvarlanmaz.
  - **Koordinat listesi dışa aktar** (Dosya → Dışa aktar; `NCNYAZ`): seçili, görünen katmanlardaki ya da bütün nokta nesneleri; NCN (boşluk), TXT (sekme), CSV (`;` ya da `,`), sütun sırası, başlık satırı, UTF-8 ya da Windows-1254. Değerler geri okununca aynı float64'ü veren en kısa ondalıkla yazılır.
- **Bulut projesi** (`app/cloud/`, `ctx.cloud`, Faz B): Dosya → Bulut projesi aç / Buluta yükle; oturum durum çubuğundaki sunucu hücresinin menüsünden ya da Buluta giriş penceresinden (yerel hesap ya da kurumun OpenID girişi) açılır.
  - **Açma:** önce proje bilgileri ve olay imleci, sonra nesneler 2 000'lik sayfalarla; iptal edilebilir, eski bir açılışın geç gelen yanıtı atılır. Sunucudan ve cihazdan gelen her şey `.kcad` okuyucusuyla (proje stilleri `.kstil` gibi) denetlenir (`cloud/incoming.ts`).
  - **Yükleme:** proje bütün katmanlar kilitsiz açılır, nesneler 2 000'lik komutlarla gider, sonra çizimin kendi katman ağacı (kilitleriyle) geri konur; sunucu kilitli katmana yazmayı reddeder (§7).
  - **Otomatik kayıt** (`cloud/sync.ts`, `syncCore.ts`): nesneler sunucunun onayladığı hâlle karşılaştırılır (`tracker.ts`: yerel numara ↔ UUID ve sürüm; geri almayla kayıtlı hâle dönmek bir şey göndermez). Son düzenlemeden 1 sn sonra (ilkinden en geç 5 sn) tek `project.changes` komutu gider; sırada tek komut olur. Değişiklikler ve yoldaki komut gönderilmeden önce IndexedDB'ye yazılır; kopan bağlantıda ya da kaybolan yanıtta aynı komut aynı idempotency anahtarıyla gider (sunucu kaydından yanıtlar, iki kez yazmaz). “Buluta kaydedildi” yalnızca sunucunun yanıtından sonra ve bekleyen bir şey yokken yazar. `Ctrl+S` hemen gönderir. Proje bilgisi değişiklikleri `project.edit` yetkisi ister; yetkisi olmayanda cihazda kalır ve bir kez söylenir. İzleyicinin değişiklikleri gönderilmez (“Salt okunur”).
  - **Çakışma:** 409'da hiçbir şeyin üzerine yazılmaz; gönderim durur, kullanıcı “Sunucudakini al” (varsayılan) ya da “Benimkini kaydet” der (`ui/cloud/ConflictDialog.ts`).
  - **Başka editörler** (`cloud/socket.ts`, `syncRemote.ts`): WebSocket son uygulanan imleçten abone olur, kaçanlar önce gelir; 20 sn'de bir yoklama, 60 sn sessizlik ölü bağlantıdır, yeniden bağlanma 1–30 sn arası geri çekilir. Kendi komutlarımızın olayları istek kimliğiyle atlanır; gelen nesneler geri alma adımı yazmadan uygulanır; gönderilmemiş yerel değişikliği olan nesne üzerine yazılmaz, çakışma olur.
  - **Cihaz taslağı** (`syncRestore.ts`): proje yeniden açılınca gönderilmemiş değişiklikler geri konur; önce yoldaki komut kendi anahtarıyla gider. Tabanı sunucuda değişmiş olan çakışmadır. Yeniden açıldıktan sonra yapılmış bir düzenleme eski taslaktan önce gelir.
- **Olaylar:**
  - `changed { layerIds }`: geometri ya da üyelik değişti; GPU tamponu yeniden kurulur.
  - `attrs { ids }`: yalnızca öznitelik değişti; tampon kurulmaz, etiket ve panel yenilenir.
- **`load()`:** geçmiş tutmadan toplu yükleme yapar (dosya açma).
- **`touched { ids, layerStyles, external }`:** her uygulanan değişikliğin (düzenleme, geri alma, yineleme, başarısız işlemin geri sarılması, dışarıdan gelen değişiklik) dokunduğu nesneler. Bulut eşitlemesi yalnızca bunları karşılaştırır.
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
| `geom/arrangement.ts`, `geom/overlay.ts`                                           | **Düzlem bindirme motoru:** kenarlar (doğru parçası ve yay) kesiştikleri, dokundukları ve örtüştükleri yerde kesilir; üst üste binen parçalar (komşu parsellerin ortak sınırı) tek parça olur ve hangi kaynağın hangi yönde geçtiği sayılır; bir kural iki yandaki sarım sayılarından parçanın sonuç sınırı olup olmadığına karar verir; kalan parçalar sonuç hep solda kalacak biçimde halkalara bağlanır, tek noktada değen halkalar ayrılır, delikler en küçük dış halkaya verilir. Yaylar yay kalır; girdi köşeleri koordinatlarını bit bit korur (`Source.points`); köşe birleştirme toleransı `TOL = 1e-6` m; kesişim noktasında doğrusal devam eden ve girdi köşesi olmayan noktalar birleştirilir |
| `geom/region.ts`                                                                   | Alan cebiri: `unionAreas`, `intersectAreas`, `subtractAreas`, `splitArea` (kesme çizgisi alanı baştan başa geçmeli), `faceIndex` / `faceAt` / `allFaces` (çizgilerin kapattığı yüzler, içteki gruplar delik), `insideArea`, `netArea`                                                                                                                                                                                                                                                                                                                                                                                                                                                                     |
| `geom/spline.ts`                                                                   | Merkezcil Catmull-Rom (Barry–Goldman), açık ve kapalı                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                     |
| `geom/hatch.ts`                                                                    | Tarama çizgilerini halkaya ve adalarına kırpma (tek-çift kuralı, yarı açık tepe kuralı, dünya ızgarasına hizalı, en çok 20 000 çizgi)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                     |
| `geom/dimension.ts`                                                                | Ölçü yerleşimi (hizalı, doğrusal ΔY/ΔX, açı, yarıçap, çap): uzatma çizgileri, eğik uçlar, her zaman okunur yazı, değer ve birimi, seçme kenarları, tutamaç yeri; `dimensionOffsetAt` (bir noktadan geçen ötelenme), `linearAngleFor` (yatay mı düşey mi, AutoCAD gibi yerleşimden), `sectorArms` (iki doğrunun, yayın konduğu bölgedeki açısı)                                                                                                                                                                                                                                                                                                                                                            |
| `ops/edgeLabels.ts`                                                                | Kenar ölçüsü yazılarının yeri: kenar ortası, halkanın dışı, okunur açı                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                    |
| `ops/edges.ts`                                                                     | Nesne → `Edge[]`. **Yeni nesne türü yalnızca kenarlarını vererek** kesişim, budama, uzatma ve kenetlemeye katılır.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                        |
| `ops/transform.ts`                                                                 | Her nesne türüne afin dönüşüm. Yazı aynalanınca okunur kalır (MIRRTEXT = 0).                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                              |
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
                          styledLayer.buildStyledLayer(layer) → SceneLayer (stil motoru toplulukları)
                          sceneBuilder.buildSceneLayer(vurgu)  → SceneLayer (ince çizgi, dolgu, nokta)
                                               ▼
                          RenderBackend.upload(layer) / render(FrameState)
```

- **`RenderBackend` sözleşmesi** (`render/types.ts`): `init`, `resize`, `upload(SceneLayer)`, `remove(id)`, `render(FrameState)`, `dispose`.
  - WebGL2 (varsayılan) ve WebGPU tam olarak uygulanmıştır ve aynı çizimi üretir (bkz. §9.5).
  - Motor çalışırken değiştirilebilir: yeni arka uç kendi tuvalini alır, bütün katmanlar belgeden yeniden yüklenir ve eski tuval ancak ilk kare çizildikten sonra kaldırılır; boş kare görünmez.
  - Arka uca yalnızca `SceneLayer` ve `FrameState` gider; varlık, katman ağacı ya da DOM gitmez.
- **Belge katmanları stil motorundan geçer** (`render/styledLayer.ts`, ayrıntı [docs/STYLE.md](docs/STYLE.md) §6): her nesne kendi sembolüyle (`entity.symbol`), yoksa katmanın işleyicisiyle (`style.renderer`), yoksa katmanın basit görünüşüyle (renk, çizgi tipi, kalınlık, dolgu, nokta simgesi) çizilir. Semboller derlenip `SceneLayer.styled` topluluklarına dönüşür: kalın ve kesikli vuruşlar (örneklenmiş parçalar), dünya ızgarasına hizalı taramalar ve döşemeler, SDF ya da atlas görüntüsü işaretler. SVG, yazı, raster ve desen görüntüleri iki arka ucun paylaştığı doku atlasındadır (`render/atlas.ts`, `backend.useAtlas`). Çizim ölçeği ya da stil kitaplığı değişince bütün katmanlar, yalnız öznitelik değişince işleyicisi olan katmanlar yeniden kurulur.
- **`SceneLayer`:** katman başına çizgi, dolgu ve nokta topluları (vurgu ve ızgara bunları kullanır) ile stilli topluluklar (`styled`). Renk ve kesikli çizgi deseni topluya aittir.
  - Çizgi: segment listesi ve kümülatif mesafe. Kesik desen parça gölgelendiricide piksel cinsinden hesaplanır.
  - Dolgu: kulak kırpma (ear clipping) ile üçgenlenir.
  - Nokta: gölgelendiricide çizilen simgeler (halka, artı, üçgen).
- **Yerel orijin (RTC):** Dünya koordinatları CPU'da float64 ve mutlaktır. GPU'ya yalnızca `doc.origin`'e göre farklar float32 olarak gider. TM koordinatları 4,4 milyon metreye ulaşır; mutlak float32 santimetre titremesine yol açar. **GPU'ya asla mutlak koordinat yüklemeyin.**
- **Çizim sırası:** alt katmanlar (ızgara), ağaçtaki yaprak sırasının tersi (listede üstteki en son, yani en üstte çizilir), üst katmanlar (`__hover`, `__sel`). Her geçişte katman katman önce düz dolgular ve stilli topluluklar (sembol düzeyine göre), sonra bütün katmanların ince çizgileri, sonra noktalar çizilir.
- **Yardımcı çizgiler** (`xline`, `ray`) GPU'ya görünüm alanının üç katı büyüklüğündeki bir kutuya kırpılarak gider (`BuildOptions.clip`); görünüm kutudan çıkınca ya da ölçek iki kattan fazla değişince bu çizgileri içeren katmanlar ve vurgular yeniden kurulur. Böylece GPU'ya hiçbir zaman uzak (float32'de titreyen) koordinat gitmez.
- **Vurgu ayrı katmandır.** Seçim değişince yalnızca `__sel` ve `__hover` yeniden kurulur, belge katmanlarına dokunulmaz.
- **`ViewportController`:**
  - `requestRender()` GPU'yu ve üst katmanı, `requestOverlay()` yalnızca 2B üst katmanı çizdirir. İkisi de `requestAnimationFrame` içinde birleştirilir.
  - Olay işleyicisinde asla eşzamanlı çizim yapmayın.
  - **Tek istisna boyut değişimidir:** canvas'ın `width`/`height` değeri değişince tampon temizlenir ve WebGL bağlamı `alpha: false` olduğu için siyah görünür. Çizim bir sonraki kareye bırakılırsa tarayıcı arada bu siyah tamponu gösterir; panel ayırıcısı sürüklenirken ekran yanıp söner. Bu yüzden `resize()` (ResizeObserver içinde, düzenden sonra ve boyamadan önce çalışır) boyut gerçekten değiştiyse hemen `frame()` çağırır. Duman testi sürükleme sırasında ekran akışını kare kare inceleyerek bunu denetler.
- **Üst katman** (`viewport/overlay.ts`, Canvas2D) şunları çizer: etiketler (`LabelStyle` ile), tutamaçlar, kenet işareti, artı imleç, ölçek çubuğu, "K" kuzey oku ve araç önizlemeleri. GPU metni (SDF) gelene kadar yazılar buradadır.
- **`PickIndex`** (`viewport/picking.ts`):
  - Seçme önceliği: nokta ve kenar, sonra imleci içeren en küçük çokgen (bina, parsel, ada sırasıyla).
  - Kenetleme türleri: uç, orta, merkez, nokta, çeyrek, kesişim, dik, en yakın. Tercihlerden süzülür (`prefs.snap*`).
  - Kenet önceliği: eşit uzaklıkta uç ve nokta, kesişimden; kesişim, merkezden; merkez, çeyrekten; çeyrek, ortadan; orta, dikten önce gelir. "En yakın" yalnızca başka aday yoksa kullanılır.
  - **Kenet her `pointerdown` ve `pointerup`'ta yeniden hesaplanır.** Fare hareketi olmadan gelen tıklama (kalem, dokunma, hızlı tıklama) eski kenet noktasına yapışmamalıdır.
  - `hitEdge` (yalnızca kenar seçimi) ve `edgesIn` (sınır kenarları) değiştirme araçları içindir. Budama ve uzatma sınır olarak görünür alandaki tüm kenarları kullanır.
  - Pencere seçimi (soldan sağa, tamamen içeride) ve kesişim seçimi (sağdan sola, temas).
  - Şimdilik sınır kutusu önbelleğiyle doğrusal tarama yapar. API aynı kalacak, iç yapı R-tree'ye geçecek.

### 4.10 Arayüz (`ui/`)

- **`Component`:** tek kök elemana ve bir `DisposableStore`'a sahiptir. `dispose()` her aboneliği ve dinleyiciyi bırakır. Her `subscribe` ve `listen` çağrısının dönüşü `this.d.add(...)` ile saklanır.
- **`ui/widgets/`:** genel ve bağımsız parçalar: `PopupMenu`, `Dropdown`, `TreeView`, `PropertyGrid`, `Dialog`, `Splitter`, `tooltip`, `controls` (segmented, switch, stepper, textField, settingRow, note). Widget'lar `AppContext` bilmez. Tek istisna `CommandButton`'dır, çünkü komuta bağlı düğmedir.
- **Paneller** (`LayersPanel`, `PropertiesPanel`, `BottomPanel`) modelden okur, değişikliği komut ya da belge API'si ile yapar. Panel içi yeniden çizimler mikro görevde birleştirilir (`PropertiesPanel.schedule`).
- **`AppShell`** yerleşimi kurar ve bölgeleri doldurur. Bileşenler birbirini tanımaz.
- **Ayar pencereleri** `ui/settings/`: `SettingsShell` (iskelet, taslak ve Kaydet/Vazgeç), `crsPicker` (ortak EPSG seçici), `ProjectSettingsDialog`, `AppSettingsDialog`.
- **Sağ dok** üst yuvada "Katmanlar | İşlemler" sekmelerini (`ui.dockTab`), altta öznitelikleri taşır. Aynı yuvayı paylaşan paneller başlıkta sekme şeridi gösterir (`Panel.setTabs`).

### 4.11 İşlem araçları (`processing/`)

Toplu işlemler (QGIS Processing gibi) için ayrı bir çatıdır; ayrıntılar [docs/PROCESSING.md](docs/PROCESSING.md)'dedir. Kısaca:

- **Araç bildirimseldir** (`defineTool`): kimlik, etiket, kategori, açıklama, yardım, takma adlar, parametreler (tür, zorunluluk, varsayılan, sınır, görünürlük koşulu, gelişmiş), çıktılar ve çalışabileceği yerler (`client | worker | server | postgis`). Pencere, araç kutusu satırı, menü, komut (`processing.run.<id>`) ve geçmiş bu tanımdan üretilir.
- **Araç belgeyi değiştirmez:** `run` çözülmüş girdiler ve salt okunur belge alır, `ChangeSet` döndürür. `ProcessingRunner` doğrular, sayfaya bağlı olanı kopyalanabilir bir işe (`RunJob`: nesne kimlikleri, hedef katman) çözer, `Executor`'ı seçer ve sonucu **tek geri alma adımı** olarak uygular; kilitli katmanları atlar, yeni hedef katmanı yalnızca yazılırsa kurar.
- **Çalışma yeri:** sayfa (`clientExecutor`) ya da Web Worker (`processing/worker/`; belge nesnelerin kopyasıyla gider, Durdur worker'ı sonlandırır). Pencerede araç başına seçilir; Otomatik, 2 000 nesne ve üstünü worker'a gönderir. `run` bu yüzden DOM'a ve modül durumuna dokunmaz, sonucu yapılandırılmış kopyayla taşınabilir olmalıdır.
- **Parametre değer tipleri tanımdan çıkar.** Tanım içindeki ok fonksiyonlarının argümanı tiplenir (`(v: Shown)`, `(c: DefaultsContext)`), yoksa çıkarım bozulur.
- **Nesne kapsamları:** seçili, görünen, tümü (görünür katmanlar), katman (grup dahil) ve modellerde önceki adımın çıktısı (`ids`). Kullanıcı bir çalıştırmada nesne türlerini daraltabilir (`kinds`: yalnızca kapalı alanlar gibi). Zorunlu girdi boş kalırsa araç çalışmaz, alanda yönlendirme yazar.
- **İfadeler** (`model/expression/`; stil motoru da kullanır): koşul ve değer parametreleri için güvenli, `eval`'siz bir dil: alanlar (`Nitelik`, `[Tapu alanı]`), geometri değişkenleri (`$alan`, `$uzunluk`, `$katman` …), Türkçe ve İngilizce işlev adları (`yuvarla`/`round`), `ve`/`veya`/`değil`. Öznitelik metni sayı gibi okunur, boş değer kuralları sabittir; hata mesajı karakter yerini söyler. Seçim üreten araçlar belgeyi değiştirmez, `select` döndürür.
- **Modeller** (akış diyagramları; `processing/model.ts`, `modelRunner.ts`, `modelEdit.ts`): adım değerleri sabit, model girdisi ya da önceki adımın çıktısı olabilir (tür uyumu `canFeed`). Model tek geri alma adımıdır (`CadDocument.beginGroup`); bir adım çalışmazsa önceki adımlar geri alınır. Yerleşik modeller değiştirilemez (kopyası düzenlenir); kullanıcının modelleri `kentos.processing.v1`'de. **Model tasarımcısı** (`ui/processing/model/`): solda girdiler ve araçlar, ortada kutu-bağlantı diyagramı (porttan sürükleyip bağlama), sağda seçilenin ayarları; kendi geri alma yığını, kaydedilmemiş değişiklik uyarısı.
- **Arayüz:** İşlemler menüsü (kategoriler kayıttan üretilir), sağ dokta İşlemler sekmesi (arama, kategori ağacı, geçmiş), `ui/processing/ToolDialog.ts` penceresi.

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
11. **Ölçmeden optimizasyon yapılmaz.** `performance.mark/measure` kullanın. Planlanan `?debug=perf` bayrağı kare süresi, yüklenen segment sayısı ve seçme süresini gösterecek.

### 6.3 Bilinen darboğazlar (büyük veri öncesi çözülecek)

- `CadDocument.byLayer()` her katman için bütün varlıkları geziyor → katman başına dizin gerekiyor.
- `PickIndex` doğrusal tarıyor → R-tree gerekiyor (statik veri için `flatbush`, düzenlenen veri için `rbush` benzeri).
- Izgara her kamera değişiminde yeni `Float32Array` ayırıyor → önceden ayrılmış tampona `bufferSubData` ile yazılmalı.
- Üst katman etiketleri her karede bütün varlıkları geziyor → görünür karo ve etiket önbelleği gerekiyor.
- `LayersPanel` her değişiklikte ağacın tamamını yeniden çiziyor → satır bazlı güncelleme ve sanallaştırma gerekiyor.
- `geometryChanged()` `JSON.stringify` ile karşılaştırıyor → alan bazlı karşılaştırma gerekiyor.

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
- **Sabit renk yok.** Arayüz renkleri CSS jetonlarından (`var(--c-…)`), çizim alanı renkleri `readCanvasPalette()` üzerinden alınır. Katman renkleri veridir.
- **Sabit piksel yazı boyutu yok.** `--fs-*` jetonları kullanılır; yükseklikler `--ui-scale` ile ölçeklenir (bkz. DESIGN.md).
- **Katman kimliğine göre dal yok** (`render/`, `viewport/`, `ui/`): davranış `LayerStyle`'dan gelir.
- **Model değişikliği:**
  - Arayüz, belgeyi yalnızca `CadDocument` API'si ya da komutlar üzerinden değiştirir.
  - `Signal`'lere dışarıdan `set` yalnızca sahibi olan depoda yapılır. Ayar pencereleri taslak üzerinde çalışır, Kaydet'te uygular.
- **Kaynak sızıntısı yok.** Her abonelik ve dinleyici bir `DisposableStore`'a eklenir. Global dinleyiciler (`window`) yalnızca açıkça bırakılabilen yerlerde kurulur.
- **Hata mesajları** ne olduğunu ve nasıl düzeltileceğini söyler, özür dilemez: "“Parsel sınırı” katmanı kilitli. Kilidi Katmanlar panelinden açın…"
- **Yapılmamış özellik** asla sessiz kalmaz: `pending(...)` komutu ya da `PendingTool` kullanılır.

### 8.1 Tamam sayılma listesi (her değişiklik için)

- [ ] `npx tsc --noEmit -p .` temiz, `pnpm build` başarılı.
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
gerekiyorsa `app/keybindings.ts`'e, menüde görünecekse `app/menus.ts`'e komut
kimliğini yazın. Araç çubuğu düğmesi için `commandButton(ctx, id, this.d)`
kullanın.

### 9.2 Yeni araç

1. `tools/` içinde `Tool` uygulayın. Uygun aileden türetin (§4.7): nokta dizisi → `PointInputTool`, seçime dönüşüm → `SelectionFirstTool`, seçime tek adımlık işlem → `SelectionActionTool`, kenara etki → `EdgePickTool`. Geometri hesabını `model/ops/` altına saf fonksiyon olarak yazıp test edin; araç yalnızca akışı ve önizlemeyi yönetir.
2. `tools/catalog.ts`'e tanımı ekleyin: kimlik, etiket, simge, grup, kısayol, takma adlar, açıklama, `create`.
3. Simge yoksa `ui/icons.ts`'e 20×20, 1,4 px çizgili bir simge çizin (bkz. DESIGN.md §6).

Komut, kısayol, araç kutusu düğmesi ve F1 listesi kendiliğinden oluşur.

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
| `model/document.test.ts`              | Geri alma ve yineleme, `transact`, await arasında gruplama ve grubu iptal, katman devralma, proje ayarları, `Formatter`                                                                                                                                                                                                                                                                                                                                                                    |
| `model/geom/ellipse.test.ts`          | Elips: parametre, uzunluk (Ramanujan'a karşı), doğru kesişimi, en yakın nokta, teğetler, eksenden kurulum                                                                                                                                                                                                                                                                                                                                                                                  |
| `model/ops/curves2.test.ts`           | Elips nesnesi (aynalama, budama, kırma, uzatma, öteleme, tutamaçlar) ve yardımcı çizgiler (budama → ışın/çizgi, kırma, öteleme)                                                                                                                                                                                                                                                                                                                                                            |
| `model/geom/parallel.test.ts`         | Paralel çizgi yanları, gönye köşeleri, sıfır mesafe, koridor alanı, kapalı eksen                                                                                                                                                                                                                                                                                                                                                                                                           |
| `model/geom/survey.test.ts`           | Ölçmecilik yapıları ve işaret kuralları                                                                                                                                                                                                                                                                                                                                                                                                                                                    |
| `model/geom/shapes.test.ts`           | Dikdörtgen ve düzgün çokgen yapıları, yay yöntemleri                                                                                                                                                                                                                                                                                                                                                                                                                                       |
| `model/geom/bulge.test.ts`            | Bulge yardımcıları, teğet devam, ters çevirme, TTY ve TTT daireleri (üçgenin iç teğet dairesi, üç daire, TM koordinatında doğru-doğru-daire)                                                                                                                                                                                                                                                                                                                                               |
| `ui/promptOptions.test.ts`            | İstem ayrıştırma: araç, adım, seçenekler, değerler, notlar                                                                                                                                                                                                                                                                                                                                                                                                                                 |
| `viewport/objectTracking.test.ts`     | Nesne izleme: tek hiza, kesişim, son noktayla kesişim, kutupsal açılar, hiza boyunca mesafe                                                                                                                                                                                                                                                                                                                                                                                                |
| `model/geom/region.test.ts`           | Alan cebiri: örtüşen, komşu (ortak kenar), T-bağlantılı, köşede değen, delikli alanlar; daire ve yay kenarları; TM koordinatında girdi köşelerinin bit bit korunması; bölme, yüzler, adalar, sarkan çizgi                                                                                                                                                                                                                                                                                  |
| `model/ops/areas.test.ts`             | Nesne ↔ alan dönüşümleri; adalı alanda alan, çevre, kenar, aynalama, tutamaç, esnet, patlat ve belgenin deliği düşürmesi                                                                                                                                                                                                                                                                                                                                                                   |
| `render/triangulate.test.ts`          | Delikli halkaların üçgenlenmesi (köprü, iç bükey köşe), toplam alan                                                                                                                                                                                                                                                                                                                                                                                                                        |
| `render/batches.test.ts` | Uzak görünümde okunamayacak kadar küçük yazının atlanması (dünya ve ekran birimi, dpr) |
| `model/ops/edit.test.ts`              | Uzat-kısalt (çizgi, yay, köşeleri aşan kısaltma, yayla biten çoklu çizgi, imleçten boy); yaylı çoklu çizgide uzunluk/alan/budama/uzatma/öteleme; birleştir, patlat, kır, esnet, köşe ekle/sil, pah ve köşe yuvarlama, bölme                                                                                                                                                                                                                                                                |
| `tools/coordinateInput.test.ts`       | Mutlak, göreli, kutupsal ve mesafe girişi                                                                                                                                                                                                                                                                                                                                                                                                                                                  |
| `processing/processing.test.ts`       | Numara biçimi, köşe sırası ve ortak köşe, parametre varsayılanları ve doğrulama, kayıt ve arama, çalıştırıcı (belgeyle, tek geri alma, boş girdi), tür süzgeci ve alan özetleri, ifadeyle seçim kipleri, öznitelik hesabı (etiket, boş sonuç, koşul, geri alma), model sıralama, denetim ve tür uyumu, model çalıştırma (zincir, tek geri alma, hatada geri alma), model düzenleme (adlandırma, zincirleme, uygun kaynaklar, silme, dizme)                                                 |
| `processing/worker/worker.test.ts`    | Worker'da çalıştırma (sahte worker, yapılandırılmış kopya): sayfayla aynı sonuç ve tek geri alma, worker'da ifade derleme, Otomatik seçim eşiği, bilinmeyen araç, çöken worker, Durdur ve yeni worker                                                                                                                                                                                                                                                                                      |
| `style/svg/svg.test.ts` | SVG çizim modeli: yol verisi (bütün komutlar, bitişik yay bayrakları, yay → kübik, geri yazma), kutular, türü koruyan dönüşümler, gruplu ve parametreli SVG çıktısı, düzgün çokgen/yıldız, içe alma (dönüşümler, boyalar, atlananlar) |
| `style/legend.test.ts` | Lejant: işleyicisiz katman, kategoriler ve diğer değerler, kapalı kategori ve kurallar, üst kural adıyla alt kurallar, nesnelerin kendi sembolleri |
| `style/classify.test.ts`              | Katman stili sınıflama: ifade değerleri, benzersiz değerler ve doğal sıra, eşit aralık ve eşit sayı, renk rampası, geometriye göre basit semboller                                                                                                                                                                                                                                                                                                                                         |
| `style/system/system.test.ts`         | Sistem kitaplığı: benzersiz kimlikler, her sembolün doğrulanması, kullanılan çizimlerin varlığı, her öğenin kategorisi                                                                                                                                                                                                                                                                                                                                                                     |
| `style/style.test.ts`                 | Stil motoru: birimler, alan halkalarının yönü, çizgi boyunca işaret yerleşimi, alanın iç noktası, derleme (kesik ve kaydırma, dönüşümlü işaretler, içe kaydırılmış kenar, tarama, öznitelikten yazı, veriye bağlı boyut/açı/renk/görünürlük, desen döşemesi), işleyiciler (kategorili, aralıklı, iç içe kurallar ve ölçek aralığı), kitaplık (sistem salt okunur, kopya, ağaç ve arama, projeye varlıklarıyla kopya), .kstil (dışa/içe aktarma, çakışma kipleri, doğrulama, SVG temizliği) |
| `model/expression/expression.test.ts` | İfade dili: alanlar ve değişkenler, metin-sayı aritmetiği, karşılaştırma ve boş değer kuralları, Türkçe/İngilizce işlevler, konumlu hata mesajları, önizleme                                                                                                                                                                                                                                                                                                                               |
| `style/svg/importSvg.test.ts` | SVG içe alma: renk sözdizimleri ve alfa, bütün dönüşümler, viewBox ve preserveAspectRatio, CSS sınıf/kimlik/torun seçicileri ve devralma, `<use>`/`<symbol>`/`<defs>`, birimler (mm → çizim birimi, sembol boyu), iç içe `<svg>`, bütün ilkeller, saydamlık, kesik, uç, köşe, dolgu kuralı, degrade ve desenin düz renge inmesi, kırpma/maske/görüntü sayımı, `<tspan>` satırları, renk eşleme (siyah, baskın, ikinci renk), düzenleyicinin kendi kaynağı (kimlik, ad, grup, gizli, altlık) |
| `style/svg/exportSvg.test.ts` | SVG dışa aktarma: sembol SVG (parametreler, mm boyu, zemin) ve geri okuma, düz SVG (önizleme renkleri, alfa → opacity, mm), seçime kırpma, `<defs>` içinde altlık, kaynak görünümü (kimlik, ad, gizli, öğe aralıkları, kararlı gidiş-dönüş), PNG boyu (piksel, DPI) ve pHYs parçası (CRC) |
| `style/svg/trace.test.ts` | Bitmap izleme: parlaklık ve alfa, tek piksel halkası, keskin köşeli kare, delikli halka ve içindeki ada, çapraz şeridin merdiveni, benek temizliği (gürültü), yumuşak düğümlü disk ve alanı, kapalı halkada Douglas–Peucker |
| `model/geom/golden.test.ts` | Rust çekirdeğiyle paylaşılan golden geometri durumları (`fixtures/geometry/v1/cases.json`): bulge yayı, uzunluk, halka ve işaretli alan, nokta-çokgen, delikli alan ve çevre; TM koordinatları; tolerans dosyada |
| `model/geom/reference.test.ts` | Bağımsız kesin referansa (Python kesirleri, 60 basamak π; `fixtures/geometry/v1/reference.json`) göre doğruluk: ondalık metinden TM parsel ve adalı alan, yaylı alanlar ve çevre; her durumun hata sınırı içinde (§23.4) |
| `model/snapshot.test.ts` | .kcad dosyası: 13 nesne türünün TM koordinatında bit bit gidiş-dönüşü (bulge, delik, elips, ölçü, proje sembolü), kilitli örnek dosyaya (`fixtures/document/v1/sample.json`) eşitlik, bozuk dosyaların Türkçe “yer: sorun” iletisiyle reddi, `dirty` akışı (geri alma, katman durumu, yazım sürerken yapılan değişiklik) |
| `app/fileIO.test.ts` | Kaydet/Aç (bellek içi dosyayla): işaret yalnızca yazımdan sonra temizlenir, yazım hatası ve vazgeçme kaydedilmemiş bırakır, yazım sürerken yapılan değişiklik kaydedilmemiş kalır, bozuk dosya açık çizime dokunmaz, yazılamayan dosya kayıt hedefi olmaz, aynı anda tek dosya komutu |
| `app/server.test.ts` | Sağlık yanıtının güvenilmeyen veri olarak okunması; yalnız aynı sözleşme sürümünde “bağlı”; 503, 404, HTML dizin sayfası, sözleşmeye uymayan gövde ve ağ hatasında “yok” ve nedeni; eşzamanlı denetimin paylaşılması |
| `contracts/contracts.test.ts` | Uygulama tiplerinin (`Entity`, `LayerNode`, `ProjectSettings`, `StyleFile`, `RunJob` …) Rust'tan üretilen sözleşmelere derleme anında uyması (`tsc` denetler) |
| `geo/crs.test.ts` | CRS kaydı: Rust ile paylaşılan dosyayla birebir aynılık (kayıt değişince yeniden kaydedilir; Rust tarafı EPSG değerlerine göre denetler), tekil SRID, varsayılan, dilim önerisi (sınırda batı dilimi), SRID/ad/bölge araması |
| `model/external.test.ts` | Dışarıdan gelen değişiklikler: `touched` olayları (geri alma ve başarısız işlem dahil), geri alma adımı ve kaydedilmemiş işareti yazmadan uygulama, başkasının dokunduğu nesnenin geri alma adımının silinmesi, dokunulmayanların korunması, proje bilgilerinin sessiz uygulanması, açık grupta reddetme |
| `app/cloud/sync.test.ts` | Bulut otomatik kaydı, sunucunun bellek içi benzeriyle (`fakeServer.ts`): yanıttan sonra “kaydedildi”, geri almayla sıfır gönderim, silip geri alınca aynı kimlikle yeniden açma, ölü ağda ve kaybolan yanıtta iki kez yazmama, çakışmada durma ve iki çözüm yolu, başka editörün değişikliğinin geri alma adımsız gelmesi, yerel değişiklikli nesnede çakışma, yeniden açılışta cihaz taslağı ve kayıp komutun aynı anahtarla gönderilmesi, yeniden açıldıktan sonraki düzenlemenin taslaktan önce gelmesi, proje bilgisi yetkisi, kurala uymayan gelen nesnenin reddi |
| `wasm/parity/parity.test.ts` | TypeScript ile Rust çekirdeğinin yan yana karşılaştırması: her çağrı kümesinin adlı sınır durumları ve işlem başına 200 tohumlu rastgele çağrı (`PARITY_CASES` artırır); sayılar golden toleransıyla, metin, uzunluk ve anahtarlar tam (ADR 0008) |
| `wasm/calls.wasm.test.ts` | Dondurulmuş çağrı fixture'ları (`fixtures/geometry/v1/calls-*.json`) uygulamanın yolundan (`core.ts` → WASM) |
| `wasm/parity/reference.test.ts` | Adıyla çağrılan işlemlerin bağımsız referansa (`reference-calls.json`, Python kesirleri ve 60 basamaklı kökler) göre doğruluğu: Rust (WASM) ve TS, her durumun hata sınırı içinde |
| `io/apply.test.ts` | Okunan nesnelerin belgeye konması: bin nesne tek geri alma adımı ve tek olay, adla birleşen katman, grupta yeni katmanlar (ikinci içe aktarmada aynı grup), seçilmeyen katmanın dışarıda kalması, kilitli hedefin ve bozuk nesnenin hiçbir şeyi değiştirmeden reddi |
| `io/formats.wasm.test.ts` | Rust dosya biçimleri WASM derlemesinde (`src/io/pkg`), `fixtures/formats/v1` dosyalarıyla: sözleşme sürümü, Netcad NCN (bit bit koordinat), Windows-1254 ve ondalık virgüllü Türkçe tablo, bozuk satırların numaralı iletileri, yazıp geri okuma. Paket yoksa atlanır |
| `wasm/golden.wasm.test.ts` | Rust çekirdeğinin WASM derlemesinde aynı golden durumlar ve bağımsız referanslar; paketin çalışma alanı sürümüyle derlendiği (eski paket kırılır). Paket yoksa atlanır; `pnpm test:rust` derleyip çalıştırır |
| `style/svg/pathOps.test.ts` | SVG düzenleyicisinin yol işlemleri: kesişen, komşu ve iç içe karelerde birleşim/kesişim/fark/dışlama, delik, boş kesişim, çizgiyle ve daireyle bölme; eğrilerin eğri kalması (iki dairenin birleşimi, daire deliği), even-odd halka ve tek çizgiyle yıldız; yolu kes (düz ve eğri, tam kesim noktası); çizgiyi yola çevirme (düz/kare/yuvarlak uç, sivri/pah/yuvarlak köşe, kapalı halka, kesik desen, az düğümlü eğri) ve içe/dışa öteleme; şekil düzeyinde birleşim, topla/ayır (delikler kalır), dolgulu çizgi, kaybolan şekil; sadeleştir, kapat, aç |
| `style/svg/nodeOps.test.ts` | Düğüm türleri (okuma, köşe → yumuşak/simetrik/otomatik, otomatiğin komşuyu izlemesi), ortaya düğüm ekleme (eğride ve kapanış parçasında), biçimi koruyarak silme, uçları birleştirme (iki yol, kendi kendini kapatma), düğümde kırma, parça silme, düz/eğri parça, köşe yuvarlama ve pah (yarıçap, komşuya varan kesim, büyük yarıçap, düz devam eden ve uç düğüm, çoklu köşe, eğri kenar, sürükleme uzaklığından yarıçap), hizala ve dağıt |
| `style/svg/arrange.test.ts` | Birimler (grup tek birim, seçim sırası), seçime/ilk/son/en büyük/tuvale hizalama, blok olarak hizalama, eşit aralık ve eşit boşluk; taşı (göreli, mutlak, ayrı ayrı adımla), ölçek, döndürme yönü ve merkezi, eğme, kutuya göre matris; satır-sütun, dairesel (tam tur ve yay, dönmeden) ve aynalı dizi, kopyaların grupları; sıra (öne, arkaya, en öne, en arkaya) |
| `style/svg/snapping.test.ts` | SVG düzenleyicisinin kenetlemesi: köşe/yumuşak düğüm, parça ortası, ağırlık merkezi, kutu noktaları, tuval köşesi ve kenarı, kesişim (eğriyle dahil), dik ayak ve teğet noktası (başlangıç noktasından), kılavuz, kılavuz kesişimi ve kılavuzla kesişim, noktaların çizgilerden önce gelmesi, taşınan şekil ve düğümlerin dışarıda kalması |

**Rust testleri** (`pnpm rust:test`: `cargo test` + clippy `-D warnings`):
- `crates/geometry-core/tests/golden.rs`: TS ile aynı golden dosya ve bağımsız referanslar;
- `tests/calls.rs`: dondurulmuş çağrı fixture'ları çekirdeğin çağrı tablosundan ve bağımsız referans (`reference-calls.json`); `jsmath` (Math.round, sign, min/max, V8 `Math.hypot`), `api` (JSON yazıcı: NaN/±∞, en kısa sayı biçimi; çağrı tablosu);
- `tests/numeric.rs`: §23 yuvarlama, hisse ve dağıtım, Python'la üretilmiş dosyalara karşı;
- `crates/contracts/tests/document.rs`: .kcad örneğinin gidiş-dönüşü ve reddi;
- `tests/crs.rs`: CRS kaydının EPSG değerleri;
- `apps/api`: sağlık isteği, gerçek HTTP ile;
- `crates/wasm`: halka bölücü.
- `crates/formats` (`cargo test -p kentos-formats`): kodlamalar (Windows-1254 Türkçe harfleri, UTF-16, BOM), kesin sayı okuma ve en kısa yazım (bit bit gidiş-dönüş, `nan`/`inf` reddi, ondalık virgül), rapor gruplama; koordinat listesi: ayırıcı, ondalık, başlık ve sütun önerisi (sayı büyüklüğüyle X/Y ayrımı, uluslararası başlık), tırnaklı CSV, yorum satırları, numaralı hata iletileri, dört ayırıcı ve iki kodlamayla yazıp okuma; `tests/coords.rs` fixture dosyaları.
- `crates/postgres`: ortam dosyası, kurulum adı ve parola denetimi.
- `crates/geometry-core` `tessellate`, `ewkb`: kiriş toleransı (daire, yaylı yol, elips, eğri), PostGIS'in EWKB baytlarıyla birebir aynılık ve bit bit gidiş-dönüş.
- `crates/application/tests/identity.rs` (geçici veritabanı): tenant ayrımı (satır güvenliği, başka tenant adına yazma), sunucu rolünün yapamadıkları, yerel giriş, oturumlar, koltuk ve üyelik, OpenID kimliği.
- `crates/application/tests/changes.rs`: 13 türün PostGIS'ten bit bit dönmesi, sürümler, 409'da sunucu kopyası, idempotency, kilitli katman, yetkiler, tenant ayrımı, iki eşzamanlı yazar.
- `apps/api` (`http/tests.rs`, `oidc/tests.rs`): istek kimliği, veritabanısız mod, çerezli giriş ve CSRF başlığı, proje/komut/olay yolları ve başka tenant'ın 404'ü; sahte sağlayıcıyla OpenID (kod + PKCE, tek kullanımlık state, nonce, audience, HS* reddi, anahtar yenileme sınırı, erişim belirteci).
- **Bulut uçtan uca** `scripts/e2e/cloud.mjs` (`pnpm e2e:cloud`): gerçek `kentosd` ve geliştirme veritabanı, tarayıcıda `ayse`, HTTP üzerinden ikinci editör `mehmet`: giriş, yükleme, otomatik kayıt (tam koordinat), yeniden yüklemede kalıcılık, canlı değişiklik, çakışma ve çözüm, sunucu dururken bekleyen düzenlemenin geri gelince bir kez kaydı, yeniden bağlanma. Oluşturduğu “E2E …” projeleri geliştirme veritabanında kalır.

Kurallar:

- `model/geom` ve `model/ops` altındaki her yeni fonksiyon test ile gelir. Sınır durumları (paralel, çakışık, sıfır uzunluk, açı 0/2π geçişi) mutlaka sınanır.
- Hata düzeltmesi, önce hatayı yeniden üreten bir testle başlar.
- **Uçtan uca duman testi** `scripts/e2e/smoke.mjs` (`pnpm e2e`): kendi Vite sunucusunu açar, başsız Chrome'u DevTools protokolüyle (`scripts/e2e/cdp.mjs`, bağımlılıksız) sürer, gerçek fare ve klavye olayları gönderir ve belgeyi `window.kentos` ile doğrular. Ekran görüntüleri `scripts/e2e/out/`'a düşer. Çizim, budama, eğri, ölçü, yazı, tarama, yerinde düzenleme, yay kipli çoklu çizgi, patlat/birleştir, pah, kır, pano, araç kutusu (tüm araçlar kaydırmasız görünür, grup katlama), komut şeridi düğmeleri, fareyle köşe yuvarlama, basılı sağ tıkla tek seferlik kenet, imleç yanında değer girişi, tutamaç menüsü, nesne izleme, panel boyutlandırırken siyah kare çıkmaması (`Page.startScreencast` ile), paralel çizgi ve dik çık (yazılan mesafelerle tam koordinat), alan işlemleri (Alt+B birleştir, Alt+C ile ada bırakan çıkarma, adalı alanın taranması, Shift+B ve çizgilerle sınırlı tarama ile çizgilerin kapattığı bölgeye tıklayarak alan), işlem araçları (İşlemler menüsünden pencere, canlı girdi sayısı ve önizleme, çalıştırma, geçmiş, tek geri alma adımı; ifadeyle seçimde canlı eşleşme sayısı ve seçim, Web Worker'ın sayfayla aynı sonucu vermesi, yerleşik modelin tek geri alma adımıyla çalışması, tasarımcıda girdiye bağlı adımlı modelin kaydedilmesi), varsayılan motorun WebGL2 olması, durum çubuğundan WebGPU'ya canlı geçiş ve iki motorun aynı sahneyi çizmesi (ızgara kapalı karşılaştırılır; soluk ızgara çizgileri motorlar arasında yalnızca örneklemeyle farklılaşır), dosya alışverişi (bellek içi seçiciyle koordinat listesi içe aktarma: önizlemede Ad Y X Z önerisi ve bozuk satır, başka koordinat sistemi seçilince içe aktarmanın kapanması, tam koordinatlar ve dosya adıyla yeni katman, tek geri alma adımı; seçili noktaların NCN olarak aynı metinle dışa aktarılması), geri alma akışlarını sınar. Tam değer bekleyen kontrollerde noktalar komut satırından mutlak koordinatla girilir; ekrandan tıklanan nokta piksel yuvarlaması kadar (~0,1 m) sapar. Yeni bir kullanıcı akışı eklendiğinde buraya bir kontrol eklenir.
- Tıklama noktaları ekrandan tahmin edilmez; dünya koordinatından `camera.worldToScreen` ile hesaplanır.
- Sıradaki eksikler: `core` (komut arama, kısayol çözümleme).

### 9.5 Çizim arka uçları (WebGL2 ve WebGPU)

`render/webgpu/WebGPUBackend.ts`, `WebGL2Backend` ile adım adım aynı `RenderBackend` sözleşmesini uygular:

- çizgiler için `line-list` hattı, köşe verisi `{pos: vec2f, dist: f32}`
- dolgular için `triangle-list`
- noktalar için örneklenmiş dörtgenler (WebGPU'da nokta boyutu yoktur); simge, köşe gölgelendiricisinde piksel cinsinden kurulur
- bağ grubu 0 kare verisi (öteleme, ölçek, piksel/metre, dpr, görünüm boyutu), bağ grubu 1 topluya ait stil (renk, kesik desen, nokta boyutu ve simgesi); stil tamponu yükleme sırasında bir kez yazılır
- 4× MSAA; karışım WebGL2 ile aynıdır

WGSL gölgelendiricileri (`render/webgpu/shaders.ts`) GLSL'deki kesik desen ve nokta simgesi mantığının birebir karşılığıdır; birinde yapılan değişiklik ötekine de yapılır. `lib.dom` yalnızca WebGPU bayrak tiplerini bildirir; `GPUBufferUsage` gibi sabitler dosyada belirtimdeki değerleriyle tanımlıdır. `createBackend` tarayıcı desteklemiyorsa WebGL2'ye düşer.

Başsız Chrome'da `--enable-unsafe-webgpu` tek başına yetmez (aygıt ilk gönderimde düşer). `scripts/e2e/cdp.mjs` içindeki `WEBGPU_ARGS` Vulkan/SwiftShader bayraklarını ekler; duman testi iki motorun çizdiği piksel sayısını karşılaştırır.

### 9.6 Yeni işlem aracı

`processing/builtin/<ad>.ts` içinde `defineTool({...})` ile tanımı yazın, `BUILTIN_TOOLS` listesine ekleyin; hesabın saf kısmını test edin ve çalıştırıcıyla belge üzerinde bir test ekleyin. Arayüz kodu yazılmaz. Tarif ve kurallar: [docs/PROCESSING.md](docs/PROCESSING.md) §8.

### 9.7 Yeni dosya biçimi

Biçimler Rust'tadır, çünkü sunucunun ileride çalışacak içe aktarma işi aynı kodu kullanacak (§14; ADR 0009):

1. **Okuyucu/yazıcı** `crates/formats/src/<biçim>.rs`: baytlardan `ImportResult` (sözleşme `crates/contracts/src/formats.rs`: kimliği 0 olan `Entity` listesi, `layerId` kaynak katmanın adı; katmanlar; rapor: türe göre sayılar, alınmayanlar ve dönüştürülenler, Türkçe neden ve ilk satır numaralarıyla) ya da nesnelerden bayt.
   - Koordinatlar dosyadaki ondalığa en yakın float64'tür; yazıcı geri okununca aynı sayıyı veren en kısa ondalığı yazar (`num.rs`).
   - Hiçbir girdi paniğe yol açmaz (lint); bozuk satır sayılır ve raporlanır. Tek geçiş, ikinci dereceden iş yok.
   - Aşkın işlevler `libm`'den gelir; native ve WASM aynı bitleri verir (`clippy.toml` std işlevlerini yasaklar).
2. **WASM sınırı** `crates/formats-wasm`: dosya bayt, seçenekler ve sonuç JSON olarak geçer. Paket `src/io/pkg`'a derlenir. `io/formatsWorker.ts` onu ilk istekte yükler (`?url` varlığı); `io/client.ts` worker'ı 30 sn boşta kalınca kapatır (büyük dosyanın belleği geri verilir), tuzakta yenisini açar.
3. **Pencere** `ui/io/`: dosya ve kaynağın bilgileri, seçenekler, önizleme ya da özet, “Bu koordinatlar hangi sistemde?” (`CrsQuestion`), hedef katman. İçe aktarma `io/apply.ts` ile tek geri alma adımıdır. Komut `app/fileExchange.ts`'e yazılır ve pencereyi dinamik içe aktarmayla açar.
4. **Testler:** Rust birim testleri ve `fixtures/formats/v1/` dosyalarıyla `crates/formats/tests/`; aynı dosyalar WASM'da `io/formats.wasm.test.ts`; akış duman testinde, bellek içi seçiciyle.

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
- Örnek proje kodla üretiliyor (`model/sampleProject.ts`); “Yeni proje” komutu yok. Kaydet/Aç yalnızca yerel .kcad dosyasıyla çalışıyor; bulut kaydı, otomatik kayıt ve son açılan dosyalar listesi sunucuyla gelecek (§21). Dosya ikili parçasız, tek JSON'dur.
- Pano yalnızca bu sekmede (bellekte) çalışıyor; sekmeler ya da uygulamalar arası kopyalama yok.
- Öznitelikler serbest metin; şema yok.
- Çizgi kalınlıkları ekranda 1 px. `lineWeight` şimdilik yalnızca veri.
- Budama ve uzatma, sınır olarak görünür alandaki tüm kenarları her imleç hareketinde yeniden topluyor (önizleme için). Büyük veride R-tree ile yalnızca hedefin çevresine bakılmalı.
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
- Arayüz bileşenlerinin birim testi yok; arayüz yalnızca duman testiyle (`pnpm e2e`) sınanıyor.
- Rust çekirdeği (`crates/geometry-core`) TypeScript geometrisinin yalnızca bir alt kümesini karşılıyor: bulge yayı, yol uzunluğu, halka ve delikli alan, çevre, nokta-çokgen, sınır kutusu. Uygulama çekirdeği açılışta yükler ve çağrı yolu kuruludur (ADR 0008), ama çalışan geometri henüz TypeScript'tir; modüller taşındıkça TS algoritması silinecek. Yaylı nesnelerin sınır kutusu TS'de 72 parçalı ana hatla yaklaşık bulunuyor ve golden setinde yok (ADR 0002).
- §23 sayısal politika (yuvarlama, hisse, artık dağıtımı) yalnızca Rust'ta var. Onaylı resmî politika olmadığı için durumu `draft`; kesin kadastral işlemler kapalı. `ctx.format` yalnızca gösterimdir.
- Bulut (Faz B) sınırları:
  - Tipli öznitelik şeması yok: öznitelikler sunucuda da metin (`properties jsonb`).
  - MVT/tile yayını yok; proje açılışı bütün nesneleri indirir (bbox'a göre kısmi açılış yok).
  - Başarısız giriş sınırı süreç belleğinde ve giriş adına göre (ADR 0007); kurum/üye/koltuk yönetimi yalnız komut satırından; projeyi silme ve yeniden adlandırma arayüzü yok.
  - Katman görünürlüğü ve açık/kapalı durumu proje verisi (herkes için); etkin katman kişiye özel, eşitlenmez.
  - Çakışma çözümü bütün çakışmalar için tek seçim; nesne bazlı karşılaştırma yok.
  - Canlı olay sinyali tek sunucu sürecinde (çok süreçte 5 sn'lik denetim yakalar; PostgreSQL LISTEN/NOTIFY yok); outbox hiç budanmıyor.
  - Yükleme kesilirse proje sunucuda yarım kalır (sürdürülemez).
  - Kurumun OpenID sunucusuyla gerçek deneme yapılmadı (yalnız sahte sağlayıcı).
- ADR 0005 hedefleri taslak; ağır modül açılışı ve §6.1 etkileşim bütçeleri henüz ölçülmüyor (`docs/perf/README.md`).

---

## 12. Dosya haritası

```
src/
  main.ts                    Giriş: stiller + createApp
  app/                       Kompozisyon kökü
    createApp.ts             Servisleri kurar, AppContext'i bağlar, kabuğu takar
    context.ts               AppContext arayüzü
    commands.ts              Çekirdek komutlar, tema ve yazı ölçeği uygulama
    keybindings.ts           Varsayılan kısayollar
    menus.ts                 Ana menü modeli, komuttan menü öğesi çözümü
    state.ts                 DraftingSettings, MessageLog, UiState, Preferences (localStorage)
    clipboard.ts             Clipboard: kopyalanan nesneler ve taban noktası (oturumluk)
    fileIO.ts                DocumentFiles: yerel .kcad kaydet/farklı kaydet/aç, içe aktarılacak dosyanın seçimi, dosya seçici (tarayıcı ya da test için bellek içi)
    fileExchange.ts          Dosya alışverişi komutları (koordinat listesi içe/dışa aktar); pencereleri ve biçim modülünü ilk kullanımda yükler
    server.ts                ServerStatus: API sağlık denetimi (boşta, odakta, ağ dönünce, istekle), yanıtın sözleşmeye göre okunması
    cloud/                   Bulut: api (istemci), session (ctx.cloud: oturum, aç, yükle), sync + syncCore + syncRemote + syncRestore (otomatik kayıt, olaylar, taslak), tracker (fark), socket (WebSocket), drafts (IndexedDB), incoming (gelen veriyi denetleme), commands, fakeServer (testler için)
    format.ts                Formatter: sayıdan metne tek geçit
    processing.ts            ProcessingService: işlem kaydı, çalıştırıcı, son değerler; işlem komutları
    styles.ts                StyleService: stil kitaplığı (sistem + kullanıcı localStorage + proje); stil komutları, nesneye sembol verme
  contracts/                 Sürümlü sözleşmeler: generated/ (ts-rs çıktısı, elle düzenlenmez), version.ts, contracts.test.ts (derleme anında uyum)
  io/                        Dosya alışverişi (başlangıçta yüklenmez): formatsWorker.ts (Rust biçim modülünü çalıştıran worker), client.ts (istek/yanıt, boşta kapanma, tuzakta yenileme), protocol.ts, apply.ts (okunanı tek geri alma adımıyla belgeye koyma), coords.ts (sütun sıraları, biçimler, noktalar), version.ts; pkg/ `pnpm wasm` ile üretilir, depoya girmez (+ testler)
  core/                      Bağımsız temel yapılar (signal, emitter, disposable, commands, keymap)
  geo/crs.ts                 EPSG kaydı (TUREF/ED50 TM, UTM, WGS84), arama, dilim önerisi
  geo/crsFixture.ts          Kaydın Rust ile paylaşılan sürümlü dosyası (fixtures/crs/v1/registry.json) ve dilim önerisi örnekleri
  model/                     Belge, varlıklar, geometri, katmanlar, seçim, proje ayarları, örnek proje
    expression/              İfade dili (ayrıştırma, derleme, değerler, işlevler; işlem araçları ve stil motoru kullanır)
    geom/                    Saf geometri çekirdeği: afin, yay, bulge, kesişim, öteleme, teğet daire, düzlem bindirme ve alan cebiri (+ testler)
    ops/                     Nesne işlemleri: kenarlar, yol parametresi, dönüşüm, budama/uzatma, kır, birleştir, patlat, esnet, köşe, öteleme, köşe yuvarlama/pah, tutamaçlar (+ testler)
  style/                     Stil motoru: geometry, compile, primitives, resolve, fromLayer, library, file (.kstil) (+ testler); türler model/style.ts'de
    classify.ts              Katman stili sınıflama (benzersiz değer, eşit aralık/sayı, rampalar)
    legend.ts                Lejant satırları (katman, sınıf, nesne sembolleri)
    showcase.ts              Gösterim kataloğu: her sistem sembolü örnek geometride (demo projede paftanın altı)
    system/                  Sistem kitaplığı (salt okunur, kopyalanabilir): temel çizgi tipleri, işaretler, alanlar; mpyy/ (MPYY gösterimleri: dsl.ts yardımcılar, pictograms.ts + pictogramDrawings.ts piktogramlar, uip/ nip/ cdp/ msp/ ortak/ kademe bölümleri)
    svg/                     SVG düzenleyicisinin saf modeli (+ testler). Temel: svgModel, pathData. Dosya: svgValues (renk, dönüşüm, uzunluk, CSS), importSvg (içe alma: stil, <use>, birimler, renk eşleme, özet), exportSvg (sembol/düz SVG, kaynak görünümü, PNG boyu ve DPI), trace (bitmap izleme). Düzenleme: bezier (parça, bölme, düzleştirme, uzunluk), fitCurve (Schneider uydurma), pathBool (kirişi izlenen düzleştirme, dolgu kuralları, bindirme, eğrilerin geri kurulması, yolu kes), pathStroke (çizgi dış hattı, öteleme), pathOps (şekil düzeyinde yol işlemleri), nodeOps (düğüm işlemleri, köşe yuvarla/pah), arrange (hizala, dağıt, dönüştür, diziler, sıra), snapping (kenet dizini)
  processing/                İşlem araçları: types (sözleşme), parameters, features (kapsamlar), categories, registry, runner, job (RunJob, Executor), model, modelRunner, modelEdit (+ testler)
    worker/                  Web Worker çalıştırıcısı: protokol, iş yürütme, executor, worker girişi (+ testler)
    builtin/                 Yerleşik araçlar: köşe numaralandırma (numbering + vertexNumbering), kenar uzunlukları, öznitelik hesapla, ifadeyle seç; yerleşik modeller
  render/                    RenderBackend sözleşmesi, sahne kurucu, delikli üçgenleme, ızgara, renk; webgl2/ ve webgpu/
    styledLayer.ts           Belge katmanı → stil motoru → GPU toplulukları (sembol seçimi ve geri düşüşler)
    styledSink.ts            Çizim ilkellerini topluluklara toplar (vuruş örnekleri, üçgenlenmiş dolgu, işaret örnekleri)
    atlas.ts                 Doku atlası: SVG, raster, yazı işaretleri ve desen döşemeleri (iki arka uç ortak)
    canvasShapes.ts          İşaret şekillerinin Canvas2D çizimi (atlas döşemeleri ve önizlemeler)
    symbolPreview.ts         Sembollerin Canvas2D önizlemesi (aynı çizim ilkelleri): kitaplık resimleri, tasarımcı, lejant
    webgl2/styled*.ts        Stilli toplulukların GLSL gölgelendiricileri ve çizicisi
    webgpu/styled*.ts        Aynısının WGSL karşılığı
  viewport/                  Kamera, ViewportController, PickIndex, üst katman çizimi
  tools/                     Tool sözleşmesi, ToolManager, katalog, koordinat girişi, imleç kısıtlaması (tracking)
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
    promptOptions.ts         İstem ayrıştırma ve seçenek düğmeleri (komut şeridi ve komut satırı ortak)
    menu/ toolbar/ toolbox/  Menü çubuğu, araç çubuğu, kayan araç kutusu
    dock/ layers/ properties/  Sağ dok (Katmanlar/İşlemler sekmeleri), katman ağacı, öznitelik paneli
    processing/              İşlem aracı penceresi (ToolDialog, modeller dahil), parametre kontrolleri, araç kutusu ve geçmiş paneli
      model/                 Model tasarımcısı: ModelDesigner, ModelCanvas, modelPalette, modelInspector
    bottom/                  Komut satırı ve alt panel (geçmiş, koordinat listesi, uyarılar)
    statusbar/               Durum çubuğu
    settings/                SettingsShell, crsPicker, Proje ve Uygulama ayarları pencereleri
    style/                   Stil yöneticisi, sembol tasarımcısı (katman formları, alanlar), katman stili (kurallar, sembol yuvası), resimler, .kstil dosyaları
    svgedit/                 SVG çizim düzenleyicisi: SvgEditor (pencere). Dosya: svgFile (Dosya menüsü, aç/ekle, pano ve sürükle-bırak, kitaplıktan aç, farklı kaydet), svgImport, svgExport, svgDocProps, svgReference (izleme altlığı), svgTrace (bitmap izle), svgSource (XML kaynağı), readSvg. Düzenleme: svgView (ortak türler), svgCanvas (görünüm, seçim, çizim araçları), svgNodeTool, svgSnap, svgRulers (cetvel, kılavuz), svgMeasure, svgActions (menü/panel/tuş işlemleri, panel ayarları), svgMenus (Yol, Nesne, Seç, Kenet, Cetvel), svgProps (sekmeler, Özellikler), svgStyleProps (çizgi biçimi, kutu), svgNodeProps, svgAlign, svgTransform, svgArray, svgObjects (şekil listesi), svgIcons
    widgets/                 Genel parçalar (menü, açılır liste, ağaç, özellik ızgarası, pencere, kontroller)
    cloud/                   Buluta giriş, bulut projeleri (aç, yükle), kayıt çakışması pencereleri
    io/                      Dosya alışverişi pencereleri: CoordImportDialog, CoordExportDialog; common (dosya satırı, alanlar, özet satırları, CrsQuestion), scope (dışa aktarma kapsamı), save (dışa aktarılanı yazma), zoom
    dialogs.ts               Kısayol listesi ve Hakkında
    icons.ts                 Simge seti
  styles/                    tokens, base, shell, controls, panels, settings, processing, model, style, svgedit (SVG düzenleyicisinin düzenleme araçları); io.css (dosya alışverişi pencereleri, onlarla birlikte yüklenir)
  wasm/                      Rust çekirdeğinin tarayıcı cephesi: core.ts (başlatma, op() çağrıları, NaN/±∞ geri çevirme, hata bildirimi), testSetup.ts (Vitest), parity/ (TS ↔ Rust çağrı kümeleri, karşılaştırma), golden ve çağrı fixture testleri; pkg/ `pnpm wasm` ile üretilir, depoya girmez
crates/
  contracts/                 Sürümlü sözleşmeler (Entity, katman, ayarlar, .kcad, .kstil, RunJob, Health, komut zarfı, §23 sayısal) → TS tipleri; tests/ (.kcad ve CRS dosyaları)
  geometry-core/             Saf analitik geometri (f64), jsmath (JavaScript sayı anlamı, libm), api (çağrı tablosu, JSON yazıcı) ve §23 sayısal politika (rust_decimal); clippy.toml (std aşkın işlevleri yasak); tests/ (golden, çağrı fixture'ları, bağımsız referans, sayısal)
  wasm/                      Çekirdeğin tarayıcı sınırı (wasm-bindgen): çağrı tablosu (opId/callOp, JSON) ve düz Float64Array girişleri
  formats/                   Dosya biçimleri (saf; native ve WASM): coords (NCN/TXT/CSV okuma ve yazma), text (UTF-8/16, Windows-1254/1252), num (kesin sayı okuma ve yazma), report; clippy.toml (std aşkın işlevleri yasak); tests/ (fixture dosyaları)
  formats-wasm/              Biçimlerin tarayıcı sınırı (wasm-bindgen): readCoords, writeCoords; dosya bayt, seçenek ve sonuç JSON
  postgres/                  Havuzlar, tenant kapsamlı işlem (`Db::scoped`), migration'lar (`migrations/`), `db-setup`, geçici test veritabanları (`testing`), `.env.local` okuma
  application/               Kullanım durumları: identity (yerel giriş, oturum), tenancy (rol, yetki, erişim), admin (komut satırı), cad (Entity ↔ PostGIS satırı), projects, changes (`project.changes`), events; tests/ (geçici veritabanıyla)
apps/api/                    kentosd: `serve` (Axum; http/ auth, projects, ws, error; oidc.rs; hub.rs), `db-setup`, `migrate`, yönetim komutları (cli.rs), config.rs
fixtures/                    İki dilin paylaştığı sürümlü dosyalar: geometry/v1 (golden, bağımsız referans), numeric/v1, document/v1 (.kcad örneği), crs/v1 (CRS kaydı), formats/v1 (koordinat listeleri: NCN, Windows-1254 CSV, bozuk satırlar, UTF-16)
Cargo.toml, rust-toolchain.toml, .cargo/config.toml   Rust çalışma alanı, sabit araç zinciri, 4 işlik derleme sınırı
vite.config.mjs              /v1 isteklerini yerel API'ye ileten eklenti (dev ve preview)
scripts/e2e/                 Başsız Chrome duman testi (cdp.mjs sürücü, smoke.mjs senaryo) ve bulut senaryosu (cloud.mjs)
scripts/wasm/ensure.mjs      WASM paketlerini (geometri çekirdeği, dosya biçimleri) kaynak özeti değiştiyse derler (dev/test/build/e2e öncesi)
scripts/fixtures/            Fixture kaydedicileri (GOLDEN_WRITE=1; record-calls: çağrı kümeleri) ve bağımsız referans üreticileri (Python decimal/fractions)
scripts/perf/                Build envanteri (bundle.mjs) ve başlangıç ölçümü (startup.mjs) → docs/perf/
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
kalıcı proje kaydı ve çoklu tenant henüz yoktur. `src/processing/worker/`
bir tarayıcı Web Worker'ıdır; sunucu job worker'ı değildir. WebGL2/WebGPU
arka uçları, stil motoru ve `SceneLayer` sözleşmesi mevcuttur.

> **Doğrulanmış durum (2026-09-23, `e916874` sonrası):** Faz A dilimleri depodadır:
> - Cargo çalışma alanı: `crates/contracts`, `crates/geometry-core`, `crates/wasm`, `apps/api`.
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
| `src/model/document.ts`                                | Yerel hızlı edit ve 200 adımlı undo                         | Sunucu kimliği, satır sürümü, beklenen sürümle commit, uzak undo sözleşmesi                        |
| `src/model/entities.ts`                                | f64 koordinatlar; bulge, delik, elips, spline, ölçü, tarama | Kaynak CAD tanımını sürümle; PostGIS geometri projeksiyonu ile tipli GIS alanları bağla            |
| `src/core/commands.ts`                                 | Menü/kısayol/komut satırı ortak giriş                       | Kalıcı komut için sürümlü schema, async sonuç, yetki, idempotency                                  |
| `src/processing/runner.ts` ve `src/processing/worker/` | Bildirimsel araç ve tarayıcı worker'ı                       | Ayrı Rust server executor; işi tüm tarayıcı belgesi yerine dataset/snapshot referansı ile çalıştır |
| `src/style/*`, `docs/STYLE.md`                         | `.kstil`, MPYY sembolleri, stil derleme                     | Stil belgesini DB'de sürümle; server ve WASM ortak ifade semantiğini fixture ile doğrula           |
| `src/render/types.ts`, `src/render/webgpu/*`           | Çalışan WebGPU ve WebGL2                                    | Tile streaming, feature ID ve seçili nesne overlay'i; fallback'i koru                              |
| `src/geo/crs.ts`                                       | SRID kataloğu; atamak/dönüştürmek ayrımı                    | PROJ/PostGIS dönüşümlerini pinlenmiş CRS/grid verisiyle doğrula                                    |

İlk güvenilirlik düzeltmesi: `CadDocument.transact` içindeki `finally`,
`fn()` hata attığında önceden uygulanmış değişiklikleri de commit edebilir.
Bunu rollback semantiği ve regresyon testiyle düzeltin. Sunucu transaction'ı
istemcideki `transact` yerine geçmez. `src/app/commands.ts` içindeki `file.save`
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

Mevcut `src/` ağacını toplu taşımayın. Çalışan dikey dilim ilerledikçe
modüler Cargo workspace ekleyin. İlk kapsam için ayrı Rust **api** ve
**worker** süreç modları ile ortak uygulama çekirdeği yeterlidir:

```text
src/                     mevcut TypeScript arayüzü ve WebGPU/WebGL2
apps/api/                 Axum: OIDC, feature/command API, TileJSON/MVT, events
apps/worker/              ağır analiz, import/export, indeks, tile warmup
crates/contracts/         sürümlü request/response/command; TS tip üretimi
crates/geometry-core/     saf Rust CAD geometri; native + WASM
crates/style-core/        sürümlü CAD stil/ifade IR; native + WASM
crates/application/       ortak use case, yetki, işlem ve job sözleşmesi
crates/postgres/          SQLx, PostGIS SQL, migration ve veri repository'si
crates/tiles/             MVT/TileJSON yayınlama, cache invalidation
crates/wasm/              tarayıcıya dar geometri/stil API'si
docs/adr/                 ölçülen mimari kararlar
benchmarks/               PostGIS/Martin ile karşılaştırılabilir yükler
```

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
diye raporlamayın. Ancak `src/app/createApp.ts` uygulama başında demo/
showcase, sistem stil kitaplığı ve işlem penceresini; `src/ui/dock/RightDock.ts`
görünür olmasa bile `ProcessingPanel`i statik import/oluşturma yoluyla
yüklüyor. `src/style/system/mpyy/index.ts` içindeki `import.meta.glob`
`eager: true` bütün MPYY sayfa tanımlarını ilk grafiğe alıyor.
`src/main.ts` stil/model/processing CSS'ini baştan yüklüyor.
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
