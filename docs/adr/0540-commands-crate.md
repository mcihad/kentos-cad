# ADR 0540: Komutların türleri ayrı crate'te: `kentos-commands`

- **Durum:** kabul edildi (2026-10-20). Sahibin isteği: “komutların girdi ve çıktı türlerini kentos-contracts'tan ayrı bir crate'e
  taşımak”, CBS-38'den sonra.
- **Bağlam belgesi:** ADR 0013 (ürün komutları ve katalog), 0022, 0170 (derleme ve test süreleri), 0253 (komut protokolü).
- **Web:** etkisi yok. TypeScript türleri ve `commandCatalog.json` aynı yere, bayt bayt aynı yazılır.

## Bağlam

Bir madde ortalama 10 dakika kod, yarım saat derleme ve test sürüyordu (sahibin şikâyeti, 20 Ekim). En pahalı adım
`kentos-contracts`'ın değişmesiydi.

Her yeni komut sözleşmeye girdi, çıktı ve plan türü ekler. `kentos-contracts`'ın altında yalnız serde ve ts-rs vardır, üstünde
neredeyse bütün workspace:
- belge (`kentos-domain`), biçimler, `kcad`, formlar, servisler, stil, pafta, raster, çizim hattı;
- onların üstünde uygulama katmanı, araç oturumu, işlemler, başsız sunucu, Python, MCP ve masaüstü.

Bu yüzden yeni bir komut, komutu hiç bilmeyen crate'leri de yeniden derletiyordu. M5'te masaüstüne kadar zincir yaklaşık 260 s
sürüyordu, çoğu derleyicinin ön yüzünde.

## Karar

1. **Yeni crate `kentos-commands`** (`crates/shared/commands`), `kentos-contracts`'ın üstünde. İçinde:
   - ürün komutlarının girdi, çıktı ve plan türleri: 75 `cad_*` modülü;
   - kataloğun kurulması (`catalog()`, `protocol_commands()`) ve onun testi (`KENTOS_WRITE_CATALOG=1 cargo test -p
     kentos-commands catalog`);
   - yetenekler tablosu (`capabilities_of`, `KINDS`, `PROTOCOL_CODES`).
2. **`kentos-contracts`'ta kalanlar:**
   - verinin modeli: nesne, katman, proje ayarı, form gövdesi, `.kcad`'in taşıdığı her şey;
   - kataloğun tanım türleri (`CommandCatalog`, `CommandDescriptor`, `CommandCapabilities`, `CommandTarget`);
   - alt crate'lerin kullandığı komut türleri: `cad_array`, `cad_blocks`, `cad_edit`, `cad_properties`, `cad_stac`, `cad_ucs`,
     `cad_views`.
   - Taşınan modüllerden alt crate'lerin kullandığı birkaç tür sözleşmenin kendi modüllerine geçti:
     - `FormChange` ve `FormChangeKind` `forms.rs`'e (`kentos-forms` yazar);
     - `FieldUse` ve `FieldUseHow` `fields.rs`'e;
     - `fold_name` `layer.rs`'e (`kentos-domain` kullanır).
   - `REVISION_TEXT`, `UID_TEXT` ve `nullable` yardımcısı, komutların şemaları için `pub` oldu.
3. **Cephe.** `kentos-commands` `kentos-contracts`'ı bütünüyle yeniden dışa aktarır (`pub use kentos_contracts::*`). Komut
   çalıştıran crate'ler her şeyi `kentos_commands` ile adlandırır ve yalnız ona bağlıdır:
   - `kentos-native-application`, `kentos-interaction`, `kentos-processing`;
   - `kentos-headless`, `kentos-python`, `kentos-mcp`;
   - `kentos-desktop`.

   Öbür crate'ler `kentos-commands`'a bağlanmaz. Sunucu da bağlanmaz: sunucu komutları (`project.*`) sözleşmededir.
4. **Özellikler.** `ts` ve `schema` iki crate'te de aynı anlamdadır; `kentos-commands`'ın özellikleri sözleşmeninkileri açar.
   TypeScript türleri aynı klasöre yazılır (`TS_RS_EXPORT_DIR`).
5. **Yeni komut** `kentos-commands`'a yazılır (`docs/MADDE-TARIFI.md` §3). Komutun türünü bir alt crate de kullanacaksa, o tür
   sözleşmeye yazılır.

## Sonuç

- Yalnız komutu değişen bir maddede alttaki crate'ler yeniden derlenmez.
- Sözleşmenin kendisi (veri modeli, `.kcad`) değişince zincir eskisi gibidir.
- Üretilen dosyalar değişmedi:
  - `commandCatalog.json` ve TypeScript türleri;
  - `kentos.cad` Python SDK'sı;
  - CAD manifesti.

## Doğrulama

- `cargo check --workspace --tests --examples` hatasız.
- `KENTOS_WRITE_CATALOG=1 cargo test -p kentos-commands catalog` ardından `git diff` boş:
  - `apps/web/src/contracts/generated/commandCatalog.json`;
  - `catalog/v1/forms/form-v1.schema.json`.
- `python3 scripts/python/sdk.py --check`, `python3 scripts/cad/manifest.py --check`.
- `cargo check --workspace --tests --examples`, `cargo test -p kentos-commands -p kentos-contracts` (1 537 test),
  ürün komutlarının ortak durumları (`cargo test -p kentos-native-application --test all fixtures`), `node scripts/arch/deps.mjs`.
- Süre (M5, `-j 10`, `cargo check -p kentos-desktop`): bir komut modülüne (`cad_trim.rs`) dokunulduktan sonra 68 s. Önce aynı
  değişiklik `kentos-contracts`'ta yaklaşık 260 s sürüyordu.
