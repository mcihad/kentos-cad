# KentOS CAD kılavuzu — §12 Dosya haritası ve başvuru

> [CLAUDE.md](../../CLAUDE.md)'nin 12. bölümüdür; 4 Kasım 2026'da kılavuz küçültülürken buraya taşındı. Koddaki ve ADR'lerdeki “CLAUDE.md §12…” atıfları bu dosyayı gösterir; bölüm numaraları korunur.

## 12. Dosya haritası ve başvuru

```text
apps/web/              bağımsız TypeScript/DOM uygulaması
apps/api/              kentosd: HTTP/WS, auth ve yönetim CLI
apps/desktop/          masaüstü kütüphanesi (kentos_desktop) ve tek programı kentos-cad (modül adlarıyla KentOS Mimari, Harita, GIS); Iced + KentOS UI
apps/launcher/         KentOS Launcher (kentos-launcher): manifestlerden uygulamalar, son dosyalar
apps/ui-showcase/      KentOS UI bileşen vitrini
python/                Python SDK'sı: paket `kentos`, tipli katman `kentos.cad` (katalogdan üretilir), testleri
crates/ui/             KentOS UI bileşen kütüphanesi (kentos-ui)
crates/native/         native belge (domain), ürün komutları (application), araç oturumu (interaction), işlem araçları (processing), proje modeli (project), başsız komut sunucusu (headless), Python eklentisi (python), MCP sunucusu (mcp) ve bulut istemcisi (cloud); web'e derlenmez
crates/render/wgpu/    native wgpu çizim hattı (Iced bilmez)
shaders/wgsl/          paylaşılabilir WGSL ve sürümlü düzen sözleşmesi
crates/shared/         contracts, commands (komut türleri ve katalog), geometry-core, expression, style-core, svg-core, formats, kcad, raster
crates/wasm/           yalnız hesap/codec bağlayıcıları
crates/server/         application ve postgres
fixtures/              sürümlü ortak test verisi
scripts/               ortak build/fixture araçları
tools/                 bağımsız araçlar (KCAD v2 Python okuyucusu)
docs/adr/              karar ve geçiş kanıtları
docs/perf/             tarihli ölçümler ve ortam sınırlamaları
docs/inventory/        web özellik envanteri (üretilir) ve elle notları
docs/baseline/         tarihli başlangıç kayıtları: test, e2e, fixture, vitrin
docs/deps/             bağımlılık kaydı
```

`crates/ui`, `apps/ui-showcase`, `apps/desktop` (ilk kabuk), `crates/native/domain`
(belge, ADR 0020), `crates/native/interaction` (araç oturumu, ADR 0021), `crates/native/application`
(ürün komutları, ADR 0022),
`crates/render/wgpu` ve `shaders/wgsl` (ADR 0019) kuruldu.
Stil sistemi: [docs/STYLE.md](../../docs/STYLE.md). Processing:
[docs/PROCESSING.md](../../docs/PROCESSING.md). Tarihli devir notları:
[docs/DEVIR.md](../../docs/DEVIR.md); eski durum/faz notlarını güncel kod ve
TODOS.md ile karşılaştırın.
