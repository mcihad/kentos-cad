# Arayüz kataloğunun ekleri

Web dururken (ADR 0228) eklenen komutların arayüz kaydı: adı, kısa adı, ikonu, açıklaması, takma adları, kısayolları, kategorisi ve
her proje türünün şeridindeki yeri. Biçimi web'in donmuş envanterinin (`docs/inventory/web.json`) alt kümesidir; türleri ve kuralları
`kentos_contracts::ui_catalog`'dadır (ADR 0228 §4, ADR 0229 §15).

| Dosya | İçerik |
|---|---|
| `v1/additions.json` | `kentos.catalog-additions` sürüm 1: `commands`, `icons` (web setinin 20 × 20'lik ızgarasındaki SVG öğeleri, `<svg>` öğesi olmadan), `ribbon` (türler, sekme; yeni panel `label`, `icon`, `after` ya da var olan panele `into`; öğeler komut ve boy; isteğe bağlı `overflow`: panelin ▾ listesine giden seyrek komutlar), `methods` (araçların yöntemleri: aracın komutu, adı, seçeneği, açıklaması, takma adları, ikonu; aracın bölünmüş düğmesine eklenir, düz düğmesi yöntemlerinin bölünmüş düğmesi olur; ADR 0287) |
| `v1/operations.json` | `kentos.catalog-operations` sürüm 1: her arayüz komutunun, aracının ve yönteminin ürün işlemi düzeyi (semantic, geometry, processing, none, query, session, host), ürün komutu ve işlemi, katalog dışı Python ve MCP yolları; kurallar ve tek tek öğeler (ADR 0244). Yeni komut, araç ya da yöntem eşlemesiyle aynı commit'te gelir; `python3 scripts/cad/manifest.py` `docs/cad/manifest.{json,md}`'yi yazar, `--check` denetler |
| `v1/aliases-autocad.json` | `kentos.aliasProfile/1`: AutoCAD takma adları profili (ADR 0282); her ad için AutoCAD'deki komutu ve başlattığı KentOS komutu (`command`, bir aracın yöntemiyse `option`). KentOS'un kendi adı ya da tek tuş kısayolu olan ad yüklenirken dışarıda kalır; masaüstünün `alias_profile.rs` testi her adın çalışan bir komutu ve yöntemi verdiğini, tekliği ve dışarıda kalanların listesini denetler |

- Masaüstü dosyayı envanterle birlikte gömer (`apps/desktop/src/catalog.rs`); kurallarını oradaki test denetler: kimliklerin tekliği ve
  envanterde olmaması, ikonların, sekmelerin ve panellerin varlığı, boylar, her ikonun her öğesinin çizilmesi.
- Komutlar masaüstünde çalışır sayılır (`implemented`); web ve mobil geri geldiğinde aynı dosyayı okur.
- İlk tüketicisi `GIS-40`'tır (adres arama, ADR 0229).
