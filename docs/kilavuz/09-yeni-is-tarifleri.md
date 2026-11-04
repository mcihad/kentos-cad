# KentOS CAD kılavuzu — §9 Yeni iş ekleme tarifleri

> [CLAUDE.md](../../CLAUDE.md)'nin 9. bölümüdür; 4 Kasım 2026'da kılavuz küçültülürken buraya taşındı. Koddaki ve ADR'lerdeki “CLAUDE.md §9…” atıfları bu dosyayı gösterir; bölüm numaraları korunur.

## 9. Yeni iş ekleme tarifleri

### 9.1 Komut

Registry/handler, enable/watch, gerekiyorsa alias/kısayol ve menü kaydı ekleyin;
aynı iş için UI ve otomasyonda ayrı iş kuralı yazmayın. Belgeyi değiştiren ürün komutu §9.2'deki bütünlükle gelir: katalog
ve yetenekler, ortak durumlar ve bağımsız başvuru, Python SDK, MCP, manifest.

### 9.2 Araç

Mevcut araç ailesini seçin; `tools/catalog.ts`, `steps`, preview/input/iptal,
kilitli katman ve tek undo testlerini tamamlayın. Hesabı ortak Rust'a ekleyin.

**Araç = komut (sahibin kararı, 13 Ekim 2026).** Çizimi ya da projeyi değiştiren her yeni araç işini kendi anlamsal ürün
komutuyla yapar; aynı işi Python ve yapay zekâ ajanı, geometriyi kendileri hesaplamadan, aynı komutla yapar. Bunlardan biri
eksikse araç bitmiş sayılmaz:

- **Anlamsal komut:** girdisi kullanıcının verdiğidir (nesnelerin kalıcı kimlikleri, yerleri, tıklanan noktalar, sayılar,
  seçenekler), sonucu komut hesaplar; `validate`, `plan`, `execute`, `CommandResult`, protokolün kaydı ve tekrar oynatması
  (ADR 0250, 0253, 0254, 0255). Hazır geometri alan `cad.entities.create` ve `.edit` işlemleri yalnız komutun iç yazma yoludur.
- **Araç komutu kullanır:** önizleme komutun planından ya da onun hesabından (`plan_in`, `measured_in` gibi), yazma komutun
  `execute`'undan; hesap araçta ikinci kez yazılmaz. Aracın görünümü, seçimi ve seçenekleri girdinin açık alanlarıdır (CMD-07).
- **Katalog:** tanım, girdi, çıktı ve plan şemaları, örnekler, `capabilities` (hedef türler, ret ve uyarı kodları; anlamları
  `codes.rs`'te); `DESKTOP_COMMANDS`, `registry.rs`; başsız sunucu kayıttan çalıştırır.
- **Ortak durumlar:** `fixtures/commands/v1/<komut>.json`, en az 20 durum, KentOS kodu olmadan yazılmış bağımsız başvurudan
  (`scripts/fixtures/<konu>_cases.py --check`); yetenek testi (`capabilities.rs`: her kod bir durumda ya da gerekçesiyle, tür
  denemeleri) geçer.
- **Python ve MCP:** `python3 scripts/python/sdk.py` ile `kentos.cad` yeniden üretilir ve `pnpm py:test` geçer (ortak durumlar
  tipli katmandan da); MCP aracı katalogdan gelir, var olanı değiştiren komut `crates/native/mcp/src/tools.rs`'in yıkıcı
  listesine yazılır.
- **Manifest:** aracın düzeyi `catalog/v1/operations.json`'da `semantic`, `via`'sı komut; `python3 scripts/cad/manifest.py`
  (ADR 0244). Yeni araç `geometry` ya da `none` düzeyinde gelmez.
- **Kanıt:** izler değişmeden geçer; araç komutla çalışırken resimlenir (`kentos-cad kullan …`); aracın yazması kayıtta komutun
  adıyla görünür. Web dururken (ADR 0228) web'in işleyicisi ve aracı TODOS.md §0.2'ye yazılır.

Var olan `geometry` ve `none` düzeyindeki araçlar borçtur: `docs/cad/manifest.md` onları sayar; dokunulan araç bu kurala getirilir.

### 9.3 Ayar

Önce kapsamını belirleyin (§4.4); tip, varsayılan, validasyon, migration,
kalıcılık ve live-apply davranışını birlikte ekleyin. Ayar
`crates/shared/contracts/src/settings/schema.rs`'e eklenir, `KENTOS_WRITE_SETTINGS=1 cargo test
-p kentos-contracts settings` ile üretilir; yeni kural `fixtures/settings/v1` ve iki
çalıştırıcıyla birlikte değişir (ADR 0023).

### 9.4 Test

- Web davranışında `pnpm typecheck`, ilgili Vitest testleri ve kapsamına göre
  `pnpm test/build`; UI değişiminde tarayıcı e2e ve açık/koyu/Büyük yazı kontrolü. Web dururken (ADR 0228) bunlar koşmaz.
- Rust/hesap değişiminde ilgili native testler, clippy, WASM fixture/bağımsız
  referanslar; sözleşme değişiminde Rust → TS üretimi ve drift denetimi.
  Sözleşme tipi ürün komutu kataloğunun şemasını da değiştirir:
  `KENTOS_WRITE_CATALOG=1 cargo test -p kentos-commands catalog` (ADR 0013).
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
DXF (`crates/wasm/dxf-wasm`) ve Netcad NCZ (`crates/shared/ncz`, `crates/wasm/ncz-wasm`)
kendi modüllerindedir; biçim işçisi onları ilk gerektiğinde yükler, sonuçları tipli sütunlarla
geçer ve büyük içe aktarma iki platformda kare kare, tek geri alma adımında yazılır (ADR 0138).
NCZ okuyucusu GPL-2.0-or-later bir kaynaktan türetildi: yazarın izni gelene dek onu içeren
derleme dağıtılmaz (TODOS.md `NCZ-01`).
Bozuk/kötücül girdide panic yok; boyut, nesting/karmaşıklık ve iptal sınırı vardır.
CRS sorulur, kayıp raporlanır, import tek undo olur; Rust ve WASM aynı fixture'ı
okur. Ayrıntı ADR 0009. GeoJSON (okuma/yazma) ve Shapefile (okuma) aynı yoldadır (ADR 0046):
kurallar ADR'de, bağımsız okuyucu `tools/formats/gis.py`, fixture'lar `fixtures/formats/v1/gis`;
dosyanın dediği koordinat sistemi gösterilir, projeninkinden başkaysa içe aktarma kapalıdır,
dönüşüm yoktur. Proje dosyası değişim biçimi değildir: kodek `crates/shared/kcad`,
spesifikasyon `docs/specs/kcad-v2.md`, karar ADR 0025; değişiklik spesifikasyon,
`fixtures/kcad/v2` (bağımsız Python yazıcısıyla), Rust kodeği ve `tools/kcad/kcad.py`
ile birlikte yapılır.
