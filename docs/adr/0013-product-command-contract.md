# ADR 0013: Ürün komutu sözleşmesi, katalog ve şema üretimi

- **Durum:** kabul edildi (2026-09-25). `schemars` bağımlılığını sahip onayladı.
- **Tarih:** 2026-09-25
- **Bağlam belgesi:** CLAUDE.md §4.5, §13.1, §18; TODOS.md §4 (`CMD-01..10`), `ARCH-07`, `AI-01..04`, `PY-09..16`, `TEST-03`; ADR 0002, 0010, 0015

## Bağlam

Bugün “komut” adı üç ayrı şeyi karşılıyor (koddan doğrulandı, 25 Eylül):

- **Web arayüz komutları:**
  - `core/commands.ts` kaydı: `run(args?: unknown): void`, kimlikler `tool.polygon`, `file.save`, `edit.undo` biçiminde.
  - Envanterde 163 tane var (`docs/inventory/web.json`).
  - Menü, şerit, kısayol ve komut satırı bunları çağırır. Tipli girdi, sonuç, sürüm ve yetki yoktur. Çoğu bir pencere açar ya da bir aracı başlatır.
- **Sunucu komutu:**
  - `CommandEnvelope` ile gelen `project.changes` v1: beklenen sürüm, idempotency, audit, outbox.
  - Sunucunun dağıttığı tek komut budur (`application/src/changes.rs`).
- **İşlem araçları:** `defineTool` üst verisiyle tanımlanan dört işlem aracı ve bir model: parametreler, çıktılar, hedefler.

TODOS.md §4, UI düğmesinin, komut satırının, Python'un, HTTP'nin, CLI'nin ve AI'ın aynı sürümlü sözleşmeye girmesini istiyor. Web'de işleyici TS, masaüstünde Rust olacak, hesap ortak Rust'tadır (ADR 0010).

## Karar

### İki düzey

- **Arayüz komutu:** bugünkü web kaydı olduğu gibi kalır (CLAUDE.md §4.5). Pencere açmak, araç seçmek, görünümü değiştirmek gibi arayüz eylemleridir; kimlikleri arayüze özgüdür.
- **Ürün komutu:** tipli, sürümlü, gerektiğinde arayüzsüz (headless) çalışabilen işlemdir. Örnek: `project.changes`, ileride `cad.polygon.create`, `gis.feature.query`, `project.share`.
  - Ürün komutları **katalogdadır**. Python, CLI, HTTP ve AI yalnız bunları görür.
  - Bir arayüz komutu ürün komutuna varabilir: poligon aracı, onaylandığında `cad.polygon.create` çalıştırır. Bu eşleme envantere yazılır (`AI-01`).
- İki düzey tek kavram sanılmaz. `project.changes` bir kalıcılık protokolüdür, arayüz kaydıyla karıştırılmaz (CLAUDE.md §13.1).

### Katalog Rust'ta tanımlanır

`crates/shared/contracts/src/catalog.rs` her ürün komutu için bir `CommandDescriptor` verir:

| Alan | Anlamı |
|---|---|
| `id` | Sabit ad, `alan.nesne.eylem` biçiminde, küçük harf ASCII: `cad.polygon.create`. Bir ad asla başka anlamla yeniden kullanılmaz. Bugünkü `project.changes` şemadan önce doğduğu için `alan.eylem` biçiminde kalır |
| `version` | Girdi ve çıktı şemasının sürümü. Uyumsuz değişiklik yeni sürüm açar; eski sürüm kaldırılana dek kabul edilir |
| `title`, `summary` | Türkçe ad ve bir iki cümlelik açıklama (arayüz, yardım, AI aracı açıklaması) |
| `aliases` | Komut satırı ve arama adları |
| `effect` | `query` (hiçbir şey değişmez), `view` (yalnız yerel oturum: kamera, seçim, etkin araç), `document` (açık belge; geri alınabilir), `project` (sunucudaki proje durumu: kayıt, paylaşım, bulut commit'i), `admin` (kurum ve kullanıcı yönetimi) |
| `hosts` | İşleyicisi olan yerler: `web`, `desktop`, `server` |
| `headless` | Açık girdilerle arayüzsüz çalışabilir mi? Seçim, katman ya da kamera gibi örtük durum gerekiyorsa girdide açıkça yer alır (`CMD-07`) |
| `requires` | Önkoşullar: `document` (açık belge), `cloudProject`, `signedIn` |
| `permissions` | Gereken proje izinleri (ADR 0015 adları). Birden çoksa hangisinin neye bağlı olduğunu `summary` söyler |
| `undo` | `none`, `step` (tek yerel geri alma adımı), `inverse` (bulutta ters komutla; `TX-03`) |
| `cost` | `instant`, `interactive` (kullanıcı bekler), `job` (kuyruğa girer, `202/job_id`) |
| `input`, `output` | JSON Schema: Rust tipinden **üretilir**, elle yazılmaz |
| `examples` | Örnek girdi (ve varsa çıktı); AI ve belgeler için |

- **Girdi ve çıktı tipleri** sözleşme crate'inde Rust tipidir. TS karşılıkları ts-rs ile, JSON Schema'ları `schemars` ile üretilir. Böylece tip ile şema ayrışamaz.
- **`schemars` 1.2.2** (MIT; serde özniteliklerini izler) yalnız `contracts`'ın `schema` özelliğiyle derlenir. Özellik `ts` gibi varsayılanda açıktır. `default-features = false` ile bağlananlar (sunucu, biçimler, WASM) istemedikçe derlemez; tarayıcı paketlerine girmez.
- **Üretilen katalog** `apps/web/src/contracts/generated/commandCatalog.json` dosyasıdır ve depoya girer.
  - `cargo test -p kentos-contracts` katalogu dosyayla karşılaştırır, farkta düşer.
  - Bilinçli bir değişiklik `KENTOS_WRITE_CATALOG=1` ile yeniden yazılır ve fark okunarak commit edilir (`CMD-03`, `TEST-03`).

### Uyum denetimi

- **Sunucu:** `kentos-application` kendi dağıtım tablosunu (`SERVER_COMMANDS`) katalogla karşılaştırır. `server` işaretli her katalog komutunun işleyicisi, her işleyicinin katalog kaydı olmalıdır.
- **Web ve masaüstü:** ilk ürün komutları geldiğinde (`CMD-04..07`, `cad.polygon.create`) aynı karşılaştırma web kaydına ve native kayda eklenir.
- **Python, CLI, AI:** stub'lar, yardım metni ve araç şemaları katalogdan üretilir; eksik sarmalayıcı CI'da raporlanır (`PY-16`, `AI-02`).

### Tel sözleşmesi ve yürütme

- `CommandEnvelope` sunucu komutlarının HTTP tel biçimi olarak kalır: `commandName`, `version`, idempotency anahtarı, beklenen sürümler, `input`.
  - Sunucu girdiyi katalogdaki tipe serileştirerek doğrular. Bilinmeyen ad ya da sürüm reddedilir (bugünkü davranış).
  - Aktör ve yetki oturumdan gelir, zarftaki hiçbir şeye güvenilmez.
- **Yürütme akışının hedefi** `discover → validate → preview/plan → execute → progress/result`'tır (`CMD-04`).
  - Sonuç durumları: `completed`, `queued(job_id)`, `needs_input`, `conflict`, `cancelled`, `failed` (`CMD-05`).
  - Hata sabit bir `code` taşır; iletisi, alan yolu, revizyon ve yeniden deneme bilgisi ayrıdır. Bugünkü `ApiError { error, message, requestId, conflicts }` bu biçime genişler (`ARCH-07`).
  - Yerel çalışmada tenant gerekmez (`CMD-06`).
  - Tekrar oynatma kaydı komut adı ve sürümünü, şema, algoritma ve sayısal politika sürümlerini ve kaynak revizyonunu taşır (`CMD-08`).
  - Onay gerektiren işlemde onay yalnız incelenen plana uygulanır (`CMD-09`).
- **İlk katalog kaydı `project.changes` v1'dir:** bugün çalışan tek ürün komutudur.
  - Girdisi `ProjectChanges`, çıktısı `CommitResult`.
  - Etkisi `project`; host'u `server`; headless.
  - İzinleri `feature.write` (nesneler) ve `project.edit` (proje bilgisi).
  - Geri alma yok (istemci geri alması yereldir); maliyeti `interactive`.
  - Henüz işleyicisi olmayan komutlar katalogda “yayımlanmış” gibi görünmez.

## Sonuçlar

- Sözleşme tiplerinin hepsi `schema` özelliğiyle `JsonSchema` türetir. Katalogdaki her yeni komutun şeması kendiliğinden gelir.
- Bağımlılık kaydına `schemars` ve onun zorunlu bağımlılıkları eklenir (`dyn-clone`, `ref-cast`, `schemars_derive`, `serde_derive_internals`). Bağımlılık yönü denetimi (ADR 0010) değişmez: hepsi saf Rust'tır.
- **Sıradaki dilim (`CMD-04..07`, `TX-01`, §22.1 adım 6):**
  - `cad.polygon.create` için tipli girdi (katman, halka, yay değerleri, delikler);
  - önizleme ve plan;
  - tek geri alma adımı;
  - web işleyicisi ve native işleyici;
  - ikisinin aynı fixture'dan aynı sonucu verdiği test.
  - **26 Eylül:** uygulandı, [ADR 0022](0022-first-product-command.md). Web ve masaüstü kaydı katalogla eşit tutuluyor (yukarıdaki uyum denetimi); sonuç zarfı `CommandResult`, ortak durumlar `fixtures/commands/v1`.
