# Arayüz kataloğunun ekleri

Web dururken (ADR 0228) eklenen komutların arayüz kaydı: adı, kısa adı, ikonu, açıklaması, takma adları, kısayolları, kategorisi ve
her proje türünün şeridindeki yeri. Biçimi web'in donmuş envanterinin (`docs/inventory/web.json`) alt kümesidir; türleri ve kuralları
`kentos_contracts::ui_catalog`'dadır (ADR 0228 §4, ADR 0229 §15).

| Dosya | İçerik |
|---|---|
| `v1/additions.json` | `kentos.catalog-additions` sürüm 1: `commands`, `icons` (web setinin 20 × 20'lik ızgarasındaki SVG öğeleri, `<svg>` öğesi olmadan), `ribbon` (türler, sekme; yeni panel `label`, `icon`, `after` ya da var olan panele `into`; öğeler komut ve boy; isteğe bağlı `overflow`: panelin ▾ listesine giden seyrek komutlar) |

- Masaüstü dosyayı envanterle birlikte gömer (`apps/desktop/src/catalog.rs`); kurallarını oradaki test denetler: kimliklerin tekliği ve
  envanterde olmaması, ikonların, sekmelerin ve panellerin varlığı, boylar, her ikonun her öğesinin çizilmesi.
- Komutlar masaüstünde çalışır sayılır (`implemented`); web ve mobil geri geldiğinde aynı dosyayı okur.
- İlk tüketicisi `GIS-40`'tır (adres arama, ADR 0229).
