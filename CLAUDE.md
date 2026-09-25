# KentOS CAD — Geliştirme kılavuzu

Güncelleme: 25 Eylül 2026. Bu dosya günlük geliştirme kurallarını ve mevcut
kodun sınırlarını tutar. Ayrıntılı gelecek işleri ve kabul kapıları
[TODOS.md](TODOS.md), görsel kurallar [DESIGN.md](DESIGN.md), karar gerekçeleri
[docs/adr/](docs/adr/) içindedir. Tamamlanan dilimlerin uzun geçmişini burada
tekrarlamayın; kod, test ve ilgili ADR'ye bağlantı verin.

Mevcut özellik ile hedefi ayırın. Kodun bulunması testlerin bu oturumda
çalıştırıldığı veya üretim kabulünün tamamlandığı anlamına gelmez.
Çelişkide kodu doğrulayın; ürün yönünde son kullanıcı kararlarını ve güncel
TODOS.md sınırlarını izleyin. Eski belgelerdeki bu dosyayı değiştirmeme veya
eski fazları izleme notları, kullanıcının bu sadeleştirme ve kapsam güncellemesiyle
aşılmıştır. Bölüm numaraları mevcut kod/ADR atıfları için korunmuştur.

## 0. Güncel karar özeti

- Web bağımsız TypeScript/DOM uygulamasıdır; desktop'ın web export'u değildir.
  Yalnız hesaplama/codec kütüphaneleri dar WASM bağlayıcılarıyla kullanılır.
  Web'in belge, komut, etkileşim, settings ve render orkestrasyonu TS'te kalır.
- Desktop Rust ile; `kentos-rc` 25 Eylül'de bu depoya `crates/ui`
  (`kentos-ui` / `kentos_ui`) olarak geçmişiyle alındı (ADR 0016). İlk masaüstü
  kabuğu `apps/desktop`'tadır (ADR 0017); ana CAD çizim alanı `crates/render/wgpu`
  ile Iced'in aygıtında çizilir (ADR 0019); yazı, seçim ve araçlar henüz yok.
  Web özellikleri envanter üzerinden adım adım masaüstüne taşınır.
- Web WebGPU/WebGL2 renderer'larını korur. Uygun WGSL kaynakları native ile
  paylaşılabilir; native Iced/application/wgpu runtime'ı web'e derlenmez.
- Server'ın ana görevi kişisel/kurumsal proje saklama, erişim, yetkilendirme,
  sürümleme, senkronizasyon ve paylaşımdır. Martin rolü şimdiki kapsamda yoktur;
  olası tile/harita servisi ayrı bir gelecek kararıdır.
- Hem CAD hem GIS projeleri PostGIS'te saklanıp çalışılabilecek. CAD tanımı
  sessizce GIS polyline'ına indirgenmeyecek; kaynak ve türev ayrımı korunacak.
- Hedef `.kcad`, yalnız kayıt/yükleme için açık, sürümlü binary/byte snapshot'tır;
  DB, SQL, kalıcı sorgu indeksi veya tile dosyası değildir. Mevcut v1 JSON'dur;
  binary geçişte eski dosyaların göçü gerekir. Cloud dosyaları da aynı formatı kullanır.
- Python embed/SDK ve AI yüzeyi sürümlü command sözleşmelerini kullanacak;
  yetki/transaction kurallarını atlayan ayrı mutasyon yolu kurulmayacak.
- Yeni mimariyi bir seferde baştan yazmayın; çalışan web'i koruyan dikey dilimler
  ve TODOS.md'deki kabul kapılarıyla ilerleyin.

## 1. Ürün ve mevcut durum

KentOS; harita mühendisliği, kadastro, imar ve hassas CAD/GIS çalışmaları için
geliştirilir. 18 uygulaması, imar planı, ifraz/tevhit, yol, mimari ve dijital
ikiz projeleri gelişim yönüdür; hepsi bugün tamamlanmış ürün değildir.
Web ve native masaüstü hedeflenir; mobil/dar ekran şu an öncelik değildir.
Web kabuğunun mevcut alt sınırı 1100×600 px'tir. Arayüz Türkçe; kod,
tanımlayıcılar ve kod yorumları İngilizcedir. Marka KentOS, başlık KentOS CAD'dir.

| Kodda bulunan temel | Kaynak |
|---|---|
| TS/DOM web, komut/araç kataloğu, dinamik giriş, WebGL2/WebGPU | `apps/web/src/` |
| Rust geometri, sayısal politika, pick/snap deposu, stil/ifade, SVG ve format hesapları | `crates/shared/` |
| Dar WASM bağlayıcıları ve Rust → TS sözleşme üretimi | `crates/wasm/`, `crates/shared/contracts/` |
| Belge transaction/rollback, undo/redo ve yerel JSON `.kcad` kaydet/aç | `model/document.ts`, `model/snapshot.ts`, `app/fileIO.ts` |
| Axum/Tokio/SQLx API, PG/PostGIS, kimlik/tenant, `project.changes`, audit/outbox; proje sahipliği, kişisel alan ve paylaşım (ADR 0015) | `apps/api/`, `crates/server/` |
| Web cloud aç/yükle, autosave, IndexedDB taslak, WS/reconnect ve conflict | `app/cloud/`, `ui/cloud/` |
| KentOS UI bileşenleri (Iced 0.14) ve vitrini | `crates/ui/`, `apps/ui-showcase/` |
| Masaüstü kabuğu: şerit, katmanlar, özellikler, komut satırı, `.kcad` aç/kaydet, geri al/yinele; wgpu çizim alanı (çizgi/eğri/dolgu/nokta, kaydır/yakınlaştır); kapalı alan aracı, değer alanı ve web'in tuş anlamları (ADR 0021) | `apps/desktop/` |
| Native wgpu çizim hattı ve paylaşılan WGSL sözleşmesi | `crates/render/wgpu/`, `shaders/wgsl/` |
| Masaüstü belgesi (`kentos-domain`): web `CadDocument`'inin anlamı native olarak, ortak işlem fixture'larıyla sınanır | `crates/native/domain/`, `fixtures/document-ops/` |
| Masaüstü araç oturumu (`kentos-interaction`): durumlar, veri olarak istem, kapalı alan aracı; iki platform `fixtures/interaction/v1` izlerini ve `fixtures/point-input/v1` dilbilgisini geçer | `crates/native/interaction/` |

Kapalı alan dışındaki desktop çizim araçları, çizim alanında yazı/seçim/yakalama, binary KCAD, embed Python, tam AI yüzeyi,
genişletilmiş proje bazlı paylaşım ve kalıcı server worker kabulü gelecek
işlerdir. Mevcut tenant/cloud altyapısını yok saymayın; onu bu kapsamla tamamlayın.
Web'e göre verilen kısa dosya yolları `apps/web/src/` altındadır.

## 2. Çalıştırma

Komutlar depo kökünden çalışır; kesin kaynak `package.json`,
`apps/web/package.json`, `Cargo.toml` ve `rust-toolchain.toml` dosyalarıdır.

```bash
make                     # gruplu komut listesi; servisler: make dev/run/desktop/stop/status
pnpm install
pnpm dev                 # Vite; yerel çizim API olmadan çalışır
pnpm typecheck
pnpm test                # Vitest; gerekli WASM paketlerini kontrol eder
pnpm build               # WASM kontrolü + tsc + Vite
pnpm rust:test           # web/sunucu crate'leri: cargo test + clippy -D warnings + bağımlılık yönü
pnpm rust:test:desktop   # kentos-ui, vitrin ve masaüstü: cargo test + clippy
pnpm arch:deps           # yalnız bağımlılık yönü denetimi (ADR 0010, ARCH-01)
pnpm test:rust           # Rust ve WASM/format entegrasyon testleri
pnpm wasm                # değişen ortak kaynakların WASM paketlerini derle
pnpm e2e                 # gerçek tarayıcı duman testi
pnpm e2e:visual          # görsel karşılaştırma
pnpm e2e:interaction     # etkileşim izleri: poligon kabul izi, tuş anlamları (fixtures/interaction, ADR 0018)
cargo test -p kentos-desktop traces   # aynı izler masaüstünde, pencere açmadan (ADR 0021)
pnpm e2e:cloud           # gerçek API/PostGIS cloud akışı
node scripts/wgsl/browser-check.mjs   # paylaşılan WGSL'yi Chrome WebGPU'da derler ve çizer
KENTOS_GPU_TESTS=1 cargo test -p kentos-render-wgpu --test gpu   # gerçek GPU'da hassasiyet
pnpm perf:interaction    # etkileşim ölçümleri
pnpm inventory           # web özellik envanteri: docs/inventory/web.{json,md}
pnpm inventory:check     # envanter güncel değilse düşer
pnpm db:setup            # yalnız yerel geliştirme DB/rolleri, migration, seed
pnpm api                 # kentosd serve; varsayılan 127.0.0.1:8787
pnpm kentosd -- <komut>  # yönetim CLI; yetkili hedefte bilinçli kullanılır
```

- Rust araç zinciri sabittir; `.cargo/config.toml` derlemeyi 4 işle sınırlar.
  `wasm-bindgen-cli` sürümü workspace bağımlılığıyla aynı olmalıdır
  (şu an `0.2.128`). `pnpm rust:wasm`, `rust:wasm:formats`, `rust:wasm:svg`
  paketleri ayrı derler; üretilen `pkg/` içeriğini elle düzenlemeyin.
- `dev/test/build/e2e`, `scripts/wasm/ensure.mjs` ile kaynak değişimini denetler.
  Ağır cargo, tarayıcı e2e ve benchmark süreçlerini eşzamanlı koşturmayın.
- Vite `/v1/` ve `/v1/ws` isteklerini API'ye iletir. Ayarlar
  `apps/api/src/config.rs`: ortam değişkenleri, ardından `.env.local`.
  DB bağlantı bilgilerini, OIDC sırlarını ve token'ları log'a/depoya yazmayın;
  migration/owner hesabını runtime hesabından ayırın.
- DB testleri için `KENTOS_TEST_ADMIN_URL`; DB zorunluluğu için
  `KENTOS_TEST_DB=required` kullanılır. Atlanan DB testi geçmiş test değildir.
  Kurulum/seed komutlarını üretime veya bilinmeyen veritabanına uygulamayın.
- Geliştirmede `window.kentos` tanı yüzeyi vardır; production'da yoktur.
  WebGL2 varsayılan, WebGPU tercihe bağlıdır; `?renderer=webgpu|webgl2`
  başlangıç tercihini değiştirir. WebGPU başlatılamazsa uyarıyla fallback olur.
- `kentos.ui.v1`, `kentos.prefs.v1`, `kentos.processing.v1`, `kentos.styles.v1`
  localStorage anahtarlarıdır. Gönderilmemiş cloud taslakları IndexedDB
  `kentos.cloud/drafts` içindedir; hata ayıklarken kullanıcı verisini izinsiz silmeyin.

## 3. Teknik kısıtlar

- TypeScript strict, `erasableSyntaxOnly`, `verbatimModuleSyntax`: `enum`,
  `namespace`, constructor parameter property kullanmayın; `import type`,
  union ve `as const` tercih edin. Web araç zinciri pnpm/Vite/TypeScript'tir.
- Web UI çatısı DOM `h()` (`ui/dom.ts`) ve `core/signal.ts`'tir.
  React/Vue/Lit geçişi yapmayın.
- Yeni runtime bağımlılığı veya sürüm değişikliği bakım, lisans, platform ve
  ölçüm gerekçesi ister; mevcut küçük/pure çekirdek sınırını koruyun.
  Yeni web runtime kütüphanesi küçük, tree-shaking'e uygun, MIT/BSD lisanslı,
  bakım altında ve DOM gerektirmeden worker'da kullanılabilir olmalı;
  eklemeden kullanıcı onayı alın. Rust sürümleri workspace/lockfile politikasına uysun.
- Tarayıcının ayırdığı `Ctrl+N/T/W`, `Ctrl+Shift+T`, `Alt+F/D/E`
  kısayollarını uygulama komutuna bağlamayın.

## 4. Web mimarisi

### 4.1 Katmanlar

Temel sıra: `core → geo → model → style/processing → render → viewport → tools → ui → app`.
Bu bir yerleşim sırasıdır: sonraki katman öncekini kullanır; ters yönde runtime
bağımlılığı kurulmaz. `AppContext`, viewport/tool arayüzleri için mevcut
`import type` istisnalarını koruyun; somut servisleri yukarıya bağımlı kılmayın.
`model`, `style`, `processing` DOM/UI bilmez; `render` UI/tools bilmez.
`io/` model/contracts düzeyinde, `wasm/` hesap bağlayıcısıdır.
`contracts/generated/` Rust'tan üretilir, elle düzenlenmez.

### 4.2 Kompozisyon kökü

`app/createApp.ts` servisleri kurar. Tema/font ölçeği palette okumasından önce
uygulanır; yeni global singleton veya ikinci belge sahibi oluşturmayın.

### 4.3 AppContext

Özellikler servisleri `app/context.ts` arayüzünden alır. Somut kurulum ve
lifecycle kompozisyon kökündedir; reusable widget'ları `AppContext`'e bağlamayın
(mevcut komuta bağlı `CommandButton` istisnası korunur).

### 4.4 Durum kapsamları

| Kapsam | Sahip / kalıcılık |
|---|---|
| Proje verisi/ayarı | `CadDocument`, `doc.settings`; proje dosyası/cloud revision |
| Kullanıcı tercihi | `ctx.prefs`; web localStorage, ileride native adapter |
| Yerleşim | `ctx.ui`; kullanıcıya özgü panel/ribbon düzeni |
| Oturum | seçim, etkin araç, drafting durumu, pano; proje verisi değil |

Proje varsayılanını değiştirmek açık projenin değerini değiştirmez.
Proje değişikliği dirty/revision üretir. Ayar pencereleri taslakla çalışır;
Kaydet'te uygular. Aynı ayarı hem proje hem uygulama penceresinde çoğaltmayın.

### 4.5 Komutlar

`core/commands.ts` registry, `app/commands.ts` ve ilgili app modülleri handler
kaydeder. Menü, ribbon, kısayol ve komut satırı aynı command ID'yi çağırır.
`isEnabled/isChecked` bağımlılıklarını `watch` ile bildirin. Henüz olmayan
özellik `pending(...)` ile açıkça belirtilir; sessiz no-op düğme koymayın.
`app/menus.ts` ve `tools/catalog.ts` tek kaynaklardır; ribbon'a ayrı araç listesi
yazmayın. Mevcut UI registry, hedefteki tam async/headless command bus değildir.

### 4.6 Klavye ve odak

`e.key` ile Türkçe Q/F klavyeyi koruyun. Metin alanlarında yalnız izinli
kısayollar çalışır; dialog tuşları uygulamaya sızmaz. Aktif araç seçenekleri
ilgili harflerde önceliklidir; Enter/Boşluk odaklı kontrolün yerel anlamını korur.
Koordinat yazmaya başlamak command/dinamik girişi açar; desktop aynı davranışı
hedefler. Kısayol yardımı kayıt üzerinden üretilir.

### 4.7 Araçlar

`tools/catalog.ts` yeni aracın kimlik, grup, yöntem, kısayol ve `steps` kaynağıdır.
Araç DOM'a dokunmaz; mevcut `Tool`, araç aileleri ve `ctx.view` arayüzünü kullanır.
Akış/istem/önizleme TS'te, hesap Rust'tadır. Enter/onay, Esc/iptal ve nested
şeffaf araç davranışı korunur; iptal taslağı kalıcı geometriye dönüştürmez.
Kenet pointerdown/up'ta yeniden hesaplanır; eski pointermove sonucuna güvenilmez.
Masaüstünün karşılığı `kentos_interaction`'dır (ADR 0021): iki taraf `fixtures/interaction/v1`
izlerini ve yazılan değerin `fixtures/point-input/v1` dilbilgisini geçer; davranış
değişikliği ADR 0018'in sırasıyla yapılır.

### 4.8 Belge ve kayıt

`CadDocument.add/update/remove`, toplu karşılıkları ve `transact` tek mutation
yoludur. Transaction hata verirse rollback; nested işlem savepoint; çoklu edit
tek mantıksal undo adımıdır. Async grup işlemlerinde mevcut group/cancel yolunu kullanın.
`markSaved(revision)` yalnız gerçekten kaydedilen güncel sürümü temizler.
Kaydetme sürerken yapılan yeni değişikliği dirty=false yapmayın.
Mevcut `.kcad` JSON `DocumentSnapshotV1`'dir; okuyucu sürüm/alan/SRID doğrular.
`replaceWith` öncesi aday belge doğrulansın; başarısız açılış mevcut işi kaybettirmesin.
Masaüstünün karşılığı `kentos_domain::Document`'tir; iki belge `fixtures/document-ops/v1`'i
geçer, davranış değişikliği fixture'la birlikte yapılır (ADR 0020).
Her nesnenin kalıcı `uid`'i vardır (ADR 0014): yeni nesne yeni `uid` alır, düzenleme ve
geri alma korur, `replace` yuvayı ve kimliği tutar. v1 dosyası `uid` yazmaz; açılışta
içerikten türetilir.

### 4.8.1 Hesaplama çekirdeği

`model/geom`, `model/ops` ve diğer hesap cepheleri Rust çağrı adapter'larıdır.
Geometri algoritmasını yeniden TS'e yazmayın. `singleSource.test.ts`, native/WASM
fixture'ları ve bağımsız referanslar korunur. Yeni hesap için §14 ve ADR 0008'i izleyin.

### 4.9 Çizim hattı

- `RenderBackend` sözleşmesini ve WebGL2/WebGPU davranış uyumunu koruyun.
- CPU kaynak koordinatı f64; GPU'ya yerel orijine göre float32 fark gider.
  Mutlak büyük dünya koordinatını GPU'ya yüklemeyin.
- Kirli katmanlar güncellenir; seçim/hover/ızgara ayrı cache/katmanlardır.
  `requestRender/requestOverlay` istekleri birleşir; her olayda tam belge rebuild yok.
- Canvas resize sonrası siyah kareyi önleyen mevcut senkron redraw istisnasını koruyun.
- Pick/snap/geometri kararları `PickIndex` → Rust store üzerinden gelir.
  Renderer kaynak geometriyi yuvarlamaz veya kalıcı belgeyi değiştirmez.

### 4.10 Arayüz

`Component` ve `DisposableStore` ile abonelik/dinleyici yaşam döngüsünü kapatın.
Uzun listelerde mevcut `TreeView`/`VirtualRows`'u kullanın; editte bütün ağacı
yeniden kurup odak/yeniden adlandırmayı kaybettirmeyin. Renk/font/ölçek DESIGN.md'dendir.
Uygulama içi sorular `ui/widgets/confirm.ts` (`confirmDialog`, `askUnsaved`,
`askRemove`) üzerinden sorulur; `alert/confirm` veya durum satırında soru yok.
Esc/×/arka plan vazgeçtir. Değişmemiş yeni düzenleyici için kaydet sorusu çıkarmayın.

### 4.11 Processing

`defineTool` metadata'sı UI/komut/parametrelerin kaynağıdır. Hesap salt okunur
girdiyle çalışır, `ChangeSet` üretir; runner sonucu doğrular ve tek undo adımı
uygular. Web Worker server worker değildir. İfade dilini genel JS/Python `eval`
ile değiştirmeyin. Ayrıntı: [docs/PROCESSING.md](docs/PROCESSING.md).

### 4.12 Çalışma modları

`app/workspaces.ts` içindeki CAD/CBS/Hibrit modları sunuşu süzer; veri modelini
ve hesapları değiştirmez. Mod proje ayarıdır. Gizlenen komutun kısayoldan
erişilebilir olması yetki verildiği anlamına gelmez; UI modu güvenlik sınırı değildir.

## 5. Koordinat, birim ve gösterim

- Kodda `x = doğu`, `y = kuzey`; Türk ölçmeciliği UI'ında sıra
  `Y (sağa), X (yukarı)`dır. Noktalı ondalık ve `Y,X` girişi korunur.
- Her proje CRS/SRID taşır; metadata kaynağı `geo/crs.ts`'tir.
  SRID atamak koordinat dönüşümü değildir. Kaynak CRS bilinmiyorsa sorun;
  sessiz reprojection/fallback veya tahmin yapmayın.
- Geometri açısı ile kuzeyden saat yönünde ölçmecilik semtini karıştırmayın.
  Birim dönüşümü açık; metre, m², dönüm (1000 m²), hektar (10000 m²).
- UI sayıları `ctx.format` üzerinden gösterilir; `toFixed` iş kuralı değildir.
  Gösterim basamakları kaynak koordinatı değiştirmez; ayrıntılı doğruluk §23'tedir.

## 6. Performans

### 6.1 Bütçeler

Hedef/ölçüm kaynağı ADR 0005 ve `docs/perf/`'dir; ADR taslak durumunu korur.
Donanım/driver/browser/kalite/commit bilgisi olmadan karşılaştırma yapmayın.
TODOS.md §20 güncel kabul kapsamıdır; eski tile hedefleri bugünkü cloud kapısı değildir.

### 6.2 Sıcak yol kuralları

Pointermove/frame içinde allocation, JSON/FFI, bütün belge gezisi ve tam GPU
upload'ı azaltın; toplu tipli veri kullanın. Uzamsal sorgu indeksten, uzun iş
worker'dan, uzun liste sanallaştırmadan geçer. DOM okuma/yazmayı ayırın.
Optimizasyonu aynı veri ve kalite düzeyinde ölçün; geometri/etiketi gizlice eksiltmeyin.

### 6.3 Durum ve darboğazlar

Rust pick/snap deposu, katman dizinleri, kirli güncelleme ve ağaç sanallaştırması
mevcuttur; bunları yeni yapılacak işler gibi yeniden kurmayın. Güncel darboğazı
profil çıkararak belirleyin; eski tek-makine ölçümlerini bugünün sonucu diye aktarmayın.

## 7. CAD/GIS veri doğruluğu

Belge değişiklikleri geri alınabilir ve kilitli katman kurallarına uygun olmalı.
Gizli katmana çizimde uyarı, atlanan/kayıplı nesnelerde açıklanabilir rapor gerekir.
Geometrik alan, kayıtlı/hukuki alan ve gösterim değeri ayrıdır; birbirini
otomatik ezmez. CAD analitik kaynak ve GIS izdüşümü §15'e uyar.
Topoloji, parsel/hisse ve resmî çıktılar bağımsız referans/kurum kabulü olmadan
tamamlanmış veya mevzuata uygun diye sunulmaz.

## 8. Kod kuralları

Çevredeki adlandırmayı/üslubu koruyun; yorum nedenini anlatsın. Tek dosya tek
sorumluluk taşısın; başında kısa amaç yorumu olsun, 400 satır üstünde ayrıştırmayı değerlendirin.
Signal'i sahibi değiştirir; UI command/belge API'sini kullanır.
Sabit UI rengi, vurgu, font boyu veya CDN fontu eklemeyin; CSS jetonları,
`readCanvasPalette()` ve yerel fontları kullanın. Katman rengi veridir;
renderer/UI davranışını sabit katman ID'sine göre dallandırmayın.
Hata mesajı nedeni ve çözümü söylesin. Global listener/GPU/worker kaynaklarının
dispose/cancel yolu olsun. Kullanıcının mevcut değişikliklerini koruyun.

### 8.1 Tamamlanma

İlgili testler, sözleşme/undo/yetki, UI gerekiyorsa gerçek klavye/fare ve
tema/yazı ölçeği kontrolü yapılır. Çalıştırılmayan kontrol ve nedenini bildirin.
Üretim iddiası veya tamamlandı işareti için TODOS.md kabul kapılarını kullanın.
Komut, araç, ayar, pencere, depo ya da `.kcad` alanı değiştiyse `pnpm inventory`
çalıştırın; farkı okuyup değişiklikle aynı commit'e koyun (docs/inventory/README.md).

## 9. Yeni iş ekleme tarifleri

### 9.1 Komut

Registry/handler, enable/watch, gerekiyorsa alias/kısayol ve menü kaydı ekleyin;
aynı iş için UI ve otomasyonda ayrı iş kuralı yazmayın.

### 9.2 Araç

Mevcut araç ailesini seçin; `tools/catalog.ts`, `steps`, preview/input/iptal,
kilitli katman ve tek undo testlerini tamamlayın. Hesabı ortak Rust'a ekleyin.

### 9.3 Ayar

Önce kapsamını belirleyin (§4.4); tip, varsayılan, validasyon, migration,
kalıcılık ve live-apply davranışını birlikte ekleyin.

### 9.4 Test

- Web davranışında `pnpm typecheck`, ilgili Vitest testleri ve kapsamına göre
  `pnpm test/build`; UI değişiminde tarayıcı e2e ve açık/koyu/Büyük yazı kontrolü.
- Rust/hesap değişiminde ilgili native testler, clippy, WASM fixture/bağımsız
  referanslar; sözleşme değişiminde Rust → TS üretimi ve drift denetimi.
  Sözleşme tipi ürün komutu kataloğunun şemasını da değiştirir:
  `KENTOS_WRITE_CATALOG=1 cargo test -p kentos-contracts catalog` (ADR 0013).
- DB/cloud değişiminde gerçek PostGIS entegrasyonu ve `KENTOS_TEST_DB=required`;
  offline/retry/conflict/tenant/permission senaryoları. Atlananları açıklayın.
- Golden/visual fixture güncellemesi otomatik onay değildir; farkı okuyun,
  algoritmanın kendi çıktısını tek doğruluk kanıtı saymayın. Toleransı büyütmeyin.
- Yalnız doküman değişiminde bağlantı, komut, durum ve tutarlılık kontrolü
  yeterlidir; kod testleri çalışmadıysa çalışmış gibi raporlamayın.

### 9.5 Render backend

WebGPU/WGSL ve WebGL2/GLSL yollarını aynı sahneyle doğrulayın; renk/alpha,
dash, piksel ölçeği, AA ve device-loss davranışını koruyun. Headless SwiftShader
sonuçları gerçek GPU benchmark'ı değildir; mevcut `apps/web/scripts/e2e/cdp.mjs` ayarlarını kullanın.

### 9.6 Processing aracı

`processing/builtin/` içinde bildirim ve katalog kaydı; pure hesap testi ve
runner üzerinden belge testi ekleyin. Ayrıntı `docs/PROCESSING.md`'dedir.

### 9.7 Dosya biçimi

Reader/writer `crates/shared/formats`, şema `contracts`, dar binding
`crates/wasm/formats-wasm`, web akışı `io/` ve `app/fileExchange.ts` içindedir.
Bozuk/kötücül girdide panic yok; boyut, nesting/karmaşıklık ve iptal sınırı vardır.
CRS sorulur, kayıp raporlanır, import tek undo olur; Rust ve WASM aynı fixture'ı
okur. Ayrıntı ADR 0009; yeni KCAD binary tasarımı TODOS.md §9'dadır.

## 10. Yol haritası

Tek ayrıntılı yol haritası [TODOS.md](TODOS.md)'dir. Buraya ikinci checkbox
listesi, eski Faz A–F sırası veya her tamamlanan commit'in dökümünü eklemeyin.
Yapılmış işin durumunu §1'de kısa tutun; kanıtı test/ADR/ölçümde saklayın.

## 11. Teknik borç

Borçları TODOS.md'de ilgili görev/kabul koşuluyla izleyin. Mevcut kodda
olmayan kusuru eski nottan hareketle yeniden düzeltmeye çalışmayın:
transaction rollback, gerçek kaydet/aç, backend, cloud, stil/SVG Rust çekirdeği
ve sanallaştırma temelleri zaten vardır. Binary KCAD, native desktop, geniş
paylaşım, typed attributes ve otomasyonun kalan kapsamı ayrı gelecek iştir.
Production DB TLS ve bağımsız gerçek ortam doğrulaması gibi eksikleri özellik
varlığına bakarak kapatmayın.

## 12. Dosya haritası ve başvuru

```text
apps/web/              bağımsız TypeScript/DOM uygulaması
apps/api/              kentosd: HTTP/WS, auth ve yönetim CLI
apps/desktop/          masaüstü kabuğu (kentos-cad): Iced + KentOS UI
apps/ui-showcase/      KentOS UI bileşen vitrini
crates/ui/             KentOS UI bileşen kütüphanesi (kentos-ui)
crates/native/         native belge (domain) ve araç oturumu (interaction); web'e derlenmez
crates/render/wgpu/    native wgpu çizim hattı (Iced bilmez)
shaders/wgsl/          paylaşılabilir WGSL ve sürümlü düzen sözleşmesi
crates/shared/         contracts, geometry-core, style-core, svg-core, formats
crates/wasm/           yalnız hesap/codec bağlayıcıları
crates/server/         application ve postgres
fixtures/              sürümlü ortak test verisi
scripts/               ortak build/fixture araçları
docs/adr/              karar ve geçiş kanıtları
docs/perf/             tarihli ölçümler ve ortam sınırlamaları
docs/inventory/        web özellik envanteri (üretilir) ve elle notları
docs/baseline/         tarihli başlangıç kayıtları: test, e2e, fixture, vitrin
docs/deps/             bağımlılık kaydı
```

`crates/ui`, `apps/ui-showcase`, `apps/desktop` (ilk kabuk), `crates/native/domain`
(belge, ADR 0020), `crates/native/interaction` (araç oturumu, ADR 0021),
`crates/render/wgpu` ve `shaders/wgsl` (ADR 0019) kuruldu.
Stil sistemi: [docs/STYLE.md](docs/STYLE.md). Processing:
[docs/PROCESSING.md](docs/PROCESSING.md). Tarihli devir notları:
[docs/DEVIR.md](docs/DEVIR.md); eski durum/faz notlarını güncel kod ve
TODOS.md ile karşılaştırın.

## 13. Rust backend ve depolama sınırı

Backend vardır: Axum/Tokio/SQLx ile PostGIS, kimlik/tenant, proje işlemleri ve WS.
Cloud dosya modunda binary revizyonlar nesne deposunda, katalog/izinler DB'de
olabilir; projeyi buluta koymak zorunlu CAD→GIS import'u değildir. Canlı DB
modunda PostgreSQL/PostGIS otoritedir. Özel DB/WAL/MVCC motoru tasarlamayın.
Sunucu transaction'ı istemcinin undo/transaction sorumluluğunun yerine geçmez.

### 13.1 Entegrasyon

Mevcut `project.changes` expected version, idempotency, audit/outbox temellerini
genişletin. UI command registry ile bu protokolü tek kavram sanmayın.
Yerel hızlı edit, dosya dayanıklılığı ve server commit onayı ayrı anlam taşır.

## 14. Workspace ve tek hesaplama kaynağı

Saf Rust hesap crate'leri DOM, Iced, SQLx, ağ veya host runtime bilmez.
Web bunları WASM, desktop/server native çağırır. Rust'tan üretilen veri/komut
sözleşmesi ortak olabilir; web uygulama runtime'ı ortaklaştırılmaz.
Sıcak yollarda toplu tipli buffer kullanın; nesne başına JSON/FFI üretmeyin.
Mevcut `jsmath`/`libm`, lint ve deterministic davranış kurallarını koruyun;
`unwrap/expect` ile kullanıcı girdisinde panic üretmeyin.
Yeni hesap çağrısına sınır/rastgele fixture, native/WASM ve bağımsız referans
ekleyin. Taşıma ayrıntıları [ADR 0008](docs/adr/0008-shared-core-boundary.md)'dedir.

## 15. CAD ve PostGIS kaynak otoritesi

Mevcut `crates/server/application/src/cad.rs`: basit geometride `geom`,
analitik CAD'de `cad_definition` kaynak; GIS geometri türevdir. Kaynak türü,
projection sürümü/toleransı ve revision birlikte korunur. Gelecekteki blok,
ölçü, ilişki ve proje metadata'sı da round-trip sözleşmesine dahil edilir.
Doğrudan PostGIS projesinde `.kcad` metadata/bağlantı referansı taşıyabilir;
parola taşımaz. Tam snapshot/export seçimi açık olmalı; aynı veriye iki
bağımsız yazılabilir otorite kurmayın. Ayrıntı TODOS.md §10–11, ADR 0006.

## 16. Tenant, proje yetkisi ve paylaşım

Mevcut tenant/rol altyapısını koruyup proje bazlı yetkiyle genişletin.
Proje erişimi proje düzeyindedir (ADR 0015): rolü `kentos.project_role`, izinleri
`application/src/access.rs` verir. Projeye dokunan her kullanım durumu, HTTP yolu, ürün komutu
ve WS aboneliği `access::project` ile başlar; yazanlar proje kilidi altında yeniden sorar.
Erişilemeyen proje var olmayan gibi 404'tür. Projeye bağlı satırlar yalnız `app.project_id`
kapsamında görünür.
Kimlik/izin server'da doğrulanır; tenant üyeliği her projeyi görme hakkı değildir.
API, WS, sorgu, dosya/asset, job, Python ve AI aynı erişim modelini kullanır.
Kişisel ve kurumsal sahiplik, davet, paylaşım ve izin iptali TODOS.md §12'dedir.
RLS/pool context ve runtime/admin rolleri test edilir. İndirilmiş kopyanın
geri alınabileceğini veya görüntüleme izninin DRM sağladığını iddia etmeyin.

## 17. Harita yayını: olası gelecek kapsamı

Martin entegrasyonu veya Martin eşdeğeri şu an görev değildir; cloud
saklama/açma/paylaşım için MVT/TileJSON servisi zorunlu tutulmaz.
Mevcut outbox/event güncelliği sync için korunur; gelecekte yayın için de
kullanılabilir. Ayrı yayın kararı gerekirse TODOS.md §12.5 izlenir.

### 17.1 Gelecek değerlendirmesi

Somut ihtiyaç olmadan tile crate'i, compatibility endpoint'i veya yayın fazı
başlatmayın. Olası yayın türevleri `.kcad` ya da CAD kaynak modelinin yerine geçmez.

## 18. Command, Python/AI ve ağır işler

Ürün komutları tipli/sürümlü, yetkili, iptal/önizleme/sonuç sözleşmeli olmalı.
Python ve AI bunları sarar; repository/SQL/DOM'dan iş kurallarını atlamaz.
Web processing worker'ı mevcut; kalıcı server worker ayrı teslimdir.
Server job tasarımında lease/heartbeat/fencing, idempotency, staged sonuç,
revision/yetkiyi commit anında doğrulama ve kaynak kotaları gerekir.
Uzun hesap API event loop/UI thread'de çalışmaz; socket kapanması işi yok etmez.

## 19. Teslim ve kabul

Faz sırası TODOS.md §22'dedir. Kütüphane eklemek veya ekran göstermek tek başına
teslim değildir. Proje bulutu önceliği eski yayın fazlarının yerini alır.

### 19.0 Kabul kapıları

İlgili correctness, platform uyumu, yetki, veri kaybı/restore ve performans
testlerinin kanıtını verin. Test atlamayı başarı, golden üretmeyi bağımsız
doğrulama, çalışan bir örneği bütün ürünün tamamlanması saymayın.

## 20. Lazy load ve modül yaşam döngüsü

### 20.1 Mevcut sınırlar

Stil/SVG düzenleyicisi, format pencereleri, ribbon ve ağır modüllerin mevcut
JS/CSS/WASM yükleme sınırlarını koruyun; başlangıca gereksiz import eklemeyin.

### 20.2 Yükleme

Yükleme async, hatası görünür ve yeniden denenebilir olsun. İptal/geç kalan
sonuç açık belgenin veya yeni oturumun üstüne uygulanmasın.

### 20.3 Yaşam döngüsü

Aç/kapat, tema/backend değişimi ve yeniden login'de listener, worker,
GPU kaynağı ve task temizlenir; mount/unmount tekrarları sızıntı yaratmasın.

### 20.4 Ölçüm

Startup ve modül açılışını üretim bundle'ında ölçün. WASM boyutu izlenir,
ancak kaldırılmış keyfi başlangıç boyut sınırını yeniden eklemeyin.
Python/3D/ileri analiz sıradan 2D açılışa zorunlu yük olmamalı.

## 21. Bağlantı, proje açılışı ve autosave

### 21.1 Sürekli bağlantı

Mevcut WS heartbeat/reconnect/replay/resync akışını koruyun. Auth, offline,
uyumsuz protokol ve server hatası ayrı durumlardır; socket açık diye veri güncel değildir.

### 21.2 Asenkron açılış

Yerel/cloud açılış iptal edilebilir ve generation/revision kontrollüdür.
Geç dönen yükleme kullanıcının yeni belgesini ezmez; doğrulanmamış aday belge
mevcut çalışmanın yerine geçirilmez. Büyük veride bounded bellek kullanın.

### 21.3 Otomatik kayıt

Cloud taslağı kalıcı kuyruğa alınmadan güvende denmez. Expected revision,
idempotency ve conflict davranışı korunur; başkasının güncel işi sessizce ezilmez.
Komut ACK, binary snapshot finalize ve cihazda kayıt ayrı durumlardır.
Yavaş kaydetme sırasında değişen belge eski revision adına temizlenmez.

### 21.4 Doğrulama

Offline/reconnect, tekrar istek, ACK kaybı, mixed-client, quota, yetki iptali,
upload/finalize crash ve restore senaryolarını native/web/server sınırında sınayın.

## 22. Gelecekte 3D şehir, yol ve mimari

TODOS.md §17–19 ileri kapsamı tutar. Semantik parsel/yapı/yol/terrain kaynağı
ile render mesh/LOD/3D Tiles ürününü ayırın. Web 3D renderer bağımsız kalır;
paylaşılabilir hesap/WGSL tekrar kullanılır. Bu hedefler bugünkü cloud'a
Martin rolü veya bütün desktop uygulamasını WASM yapma şartı getirmez.

## 23. Sayısal ve kadastral doğruluk

Kaynak sınır, koordinat, alan ve hak doğruluğu performanstan önce gelir.
f64/decimal veya ortak Rust tek başına sıfır hata/kadastral uygunluk kanıtı değildir.

### 23.1 Kayıpsız sayı yolu

Kaynağın hassasiyet, birim, CRS/datum ve gerekiyorsa özgün ondalık değerlerini
koruyun. Kesin decimal/rasyonel değerler JS Number üzerinden geçmez;
decimal string ve pay/payda sözleşmesi kullanılır. `NUMERIC` ölçeğinin örtük
yuvarlamasına güvenmeyin; taşma/NaN/Infinity kontrollü hatadır.
Geometrik alan, kayıtlı alan ve gösterim ayrı anlamlardır.

### 23.2 Yuvarlama

`numeric_policy_id/version`, birim, ölçek, eşitlik yönü, ara adımlar ve
dağıtım artığı kuralı açık/sürümlü olmalı. Gösterim metni hesap girdisi değildir.
`toFixed`, `Math.round`, SQL round veya kütüphane varsayılanını iş kuralı yapmayın.
Hisse/alan toplamı korunur; artığı rastgele son parsele vermeyin.
Onaysız veya belirsiz kuralda nihai kayıt tamamlanmaz.

### 23.3 Geometrik kararlar

Robust/adaptive gerektiğinde exact predicates kullanın; kesin predicate
üretilen koordinatın kesinliğini garanti etmez. Seçim piksel toleransı,
snap/topoloji toleransı, hesap hata sınırı ve kaynak belirsizliği ayrıdır.
Gizli snap/grid/repair/simplify veya büyük epsilon ile hata örtmeyin.
Alan/uzunluk kaynaktan ve açık düzlemsel/elipsoidal/3D yöntemle hesaplanır;
render tessellation'ı nihai ölçü değildir. CRS grid/epoch/sürümü kaydedilir;
eksik grid'de sessiz düşük doğruluk fallback'i yapılmaz.
Eşiği kesen hata aralığında hassasiyeti artırın; hâlâ belirsizse `needs_review`
veya atomik ret, dayanağı kayıtlı çözüm gerekir.

### 23.4 Bağımsız doğrulama

Native/WASM eşitliği yanında bağımsız yüksek hassasiyetli referans gerekir.
Sınır, yarım değer, negatif, yakın paralel/teğet, büyük koordinat, delik/eğri,
ortak sınır ve alan/hisse korunumu fixture'larını koruyun.
Nihai decimal, yuvarlama yönü ve topolojik karar platformlar arasında aynı
olmalıdır; farkta server sonucunu sessiz seçmeyin. Algoritma/build, kaynak
revision, numeric policy ve dönüşüm provenance'ı sonucu yeniden üretilebilir kılsın.
Mevcut toleransı büyütmek veya beklenen fixture'ı hataya göre yenilemek yasaktır.
Ayrıntı: ADR 0004/0008 ve TODOS.md §3/§20.

## 24. Ürün kapsamının izlenmesi

### 24.1 Veri/provider

Typed schema, domain/ilişki/form, provider ve PostGIS kapsamı TODOS.md §3/§11/§16'dadır.
Mevcut string özniteliği tipli model tamamlandı diye sunmayın.

### 24.2 Workflow ve otomasyon

Stil, form, processing, pafta, Python ve AI aynı veri/komut sözleşmesine
bağlanır; ayrıntıları TODOS.md §13–16'dan izleyin.

### 24.3 Paylaşım

Proje paylaşımı ile harita yayını ayrı özelliklerdir. Şimdiki hedef kişisel/
kurumsal projelerin yetkili saklama/erişim/senkronizasyonudur; TODOS.md §12.

### 24.4 Operasyon

TLS, sır yönetimi, kotalar, backup/PITR + object restore, audit ve dağıtım
TODOS.md §21'de izlenir. Restore denemesi olmadan backup çalışıyor denmez.
Yerel geliştirme kolaylıklarını production güvenlik ayarı gibi kullanmayın.
