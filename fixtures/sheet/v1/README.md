# Pafta düzeni fixture'ları (`kentos.sheet` v1)

[docs/sheet/design.md](../../../docs/sheet/design.md) ve [docs/sheet/tasks-rust.md](../../../docs/sheet/tasks-rust.md) Adım 1. Pafta çekirdeğinin (`crates/shared/sheet`) davranışını iki platformun aynı dosyalarla denetlemesi için yazar. Bütün uzunluklar mikrometre (`Um`, tamsayı), açılar binde bir derece (`Mdeg`, kâğıtta saat yönü, y aşağı), koordinatlar yerde metredir.

Koşanlar:

- Rust: `cargo test -p kentos-sheet` — `tests/fixtures.rs` (bu dizinin her ailesi), `tests/ops.rs` (`ops/`), `tests/templates.rs` (`svg/`), `tests/pdf.rs` (`pdf/`), `tests/wmm.rs` (`wmm/`).
- WASM: `node crates/wasm/sheet-wasm/tests/smoke.mjs` — `ops/`, `display/`, `sync/`, `kpafta/` ve öbür ailelerin dosyalarını `kentos-sheet-wasm` üzerinden koşar ve Rust'ın beklediğiyle karşılaştırır.

Beklenenlerin iki kaynağı vardır; her ailenin satırı hangisi olduğunu söyler:

- **elle**: kuraldan elle (ya da kuralın çekirdekten bağımsız yazılmış bir hesabıyla) çıkarıldı; çekirdek değişince dosya değişmez, çekirdek düzeltilir.
- **kayıt**: çekirdeğin çıktısı `KENTOS_WRITE_SHEET=1 cargo test -p kentos-sheet` ile yazıldı ve gözle denetlendi. Çekirdek değişince yeniden yazılır; fark okunmadan kaydedilmez.

| Dizin | İçerik | Beklenen | Rust testi |
|---|---|---|---|
| `ops/` | `book.json` (iki pafta, ana sayfa, resim, değişkenler; elle yazıldı) ve `cases.json`: 83 işlem, 22'si hata (yerleşim düzeni işlemleri dahil). Her başarılı durumda sonuç kitabın SHA-256'sı (sıkı JSON), ters işlem ve Türkçe etiket; her durumda `apply(op)` sonra `apply(inverse)` kitabı geri verir | kayıt (özdeşlik elle kuralı) | `tests/ops.rs` |
| `relayout/` | `a3-strip.json`: A3 yatay kitap ve dört yeni kâğıt (A1 yatay, A4 dikey, özel 500 × 300, A3 dikey); her öğenin yeni çerçevesi. `variants.json`: yerleşim düzenli kitap; dikey düzen, hiçbiri, boyla sınırlı küçük yatay düzen; geçerli düzen (`expectVariant`) | elle (§3.2, §3.2a; ilki kesirli bağımsız hesap) | `fixtures::relayout` |
| `snap/` | `basic.json`: dört taşıma sorgusu (öğeye, kılavuza, eşit aralığa, hiçbir şeye), iki tutamaç sorgusu (kenara, başka öğenin boyuna), beş döndürme; `grid.json`, `grid-off.json`: paftanın 5 mm ızgarası açık ve seçenekle kapalı (`options`) | elle (§5) | `fixtures::snap` |
| `hit/` | Döndürülmüş kutu, çizgi yolu (tolerans ve yarım kalınlık), grup, gizli öğe, içine alan / değen seçim, tutamaç ve döndürme tutamacı | elle (§5) | `fixtures::hit` |
| `display/` | `books/every-kind.json`: 13 öğe türünün hepsi (çift çerçeve ve bölge işaretleri, karelajlı harita, genel bakış, ölçek, not yazılı kuzey oku, lejant, metin, döndürülmüş metin, resim, saydam şekil, elips, ok, sabit tablo, koordinat listesi, antet, grup içinde üçgen ve çokgen, basılmayan not); tasarım ve dışa aktarma kipinde çizim planı | kayıt | `fixtures::display` |
| `svg/` | On sistem şablonu önerilen kâğıdında, örnek cevaplarla; harita çerçevesinde yazıcının gri yer tutucusu (haritanın resmi platformundur) | kayıt | `templates::system_template_svgs_are_golden` |
| `templates/valid/` | Gömülü PNG'li geçerli kullanıcı şablonu | elle | `fixtures::templates` |
| `templates/invalid/` | 13 bozuk şablon, her biri beklenen hata koduyla (`bad_json`, `reserved_id`, `template_has_place`, `missing_asset`, `bad_asset` ×2, `newer_schema`, `unknown_schema`, `out_of_range`, `duplicate_name`, `bad_template`, `page_size_mismatch`, `bad_group`) | elle | `fixtures::templates` |
| `kpafta/valid/` | `.kpafta` dosyaları: gömülü PNG'li büro paftası (kullanıcı şablonu fixture'ının paftası, resmi ve bir proje değişkeniyle) ve resmin baytları olmadan aynısı; okunur, yazılıp yeniden okununca aynı çıkar | elle | `fixtures::kpafta` |
| `kpafta/invalid/` | 10 bozuk dosya, her biri beklenen hata koduyla (`bad_json` ×2, `unknown_schema` ×2, `newer_schema`, `bad_asset` ×4, `duplicate_id`) | elle | `fixtures::kpafta` |
| `sync/` | §13 tablosunun her satırı ve kenar durumları (yalnız bulutta, hiç yüklenmemiş, listeden düşmüş) | elle (§13) | `fixtures::sync` |
| `preflight/` | Her denetimi bir kez tetikleyen pafta; beklenen (önem, kod, öğe) kümesi — ileti metni karşılaştırılmaz, boş olmaması denetlenir | elle (§9, §11a) | `fixtures::preflight_findings` |
| `atlas/` | Süzgeç, Türkçe sıralama, ad ve eşine “ (2)”, kenar paylı sığdırma, döndürülmüş harita, önceden tanımlı ölçekler, sığmayan nesne | elle (§7a) | `fixtures::atlas` |
| `profiles/` | Beş kip/yetenek durumu (araç durumu, adı, hazır biçimleri) ve dört galeri sıralaması | elle (§11a) | `fixtures::profiles` |
| `pdf/` | İfraz sistem şablonunun PDF'inin özeti (`ifraz.json`: SHA-256 ve boy) ve WASM'a verilen girdileri (`ifraz-book.json`, `ifraz-inputs.json`; yazı tiplerinin baytları boş, ev sahibi doldurur) | kayıt (§9a) | `pdf::the_ifraz_pdf_is_the_golden_one`; `smoke.mjs` aynı baytı ister |
| `glyphs/` | Eksik karakter kuralı: `runs.json` (9 satırın yüz parçaları), `sheet.json` (yüzde olmayan karakterli dört yazı: ön denetimin bulguları, çizim planının “?”'si, PDF'in gömdüğü yüzler, SVG'nin `<tspan>`'ları) | elle (§6 “Eksik karakter”, ölçü tablosunun −1 hücrelerinden) | `fixtures::glyphs`; `smoke.mjs` |
| `wmm/` | NOAA/NCEI'nin resmî WMM2025 dosyaları olduğu gibi: `WMM.COF` (katsayılar, `WMM2025COF.zip`'ten), `WMM2025_TestValues.txt` (aynı paketten 100 sınama noktası), `WMM2025_TEST_VALUES.txt` (modelin sayfasından 12 nokta). Kamu malı | NOAA'nın kendi değerleri (§8a) | `wmm::*` (açılar 0,01°, bileşenler 0,1 nT ve 0,01 nT içinde); `smoke.mjs` |

## Elle hesapların özeti

**Yeniden yerleşim** (`relayout/a3-strip.json`): bir eksende eski kutu `b0, l0`, yeni kutu `b1, l1`, öğe `pos, size`:
`start` → `pos − b0 + b1`; `end` → sağ (alt) boşluk korunur; `both` → iki boşluk korunur, boy en az 1 mm; `center` → merkezin kutu merkezine uzaklığı korunur, `round(c − size/2)`; `scale` → iki kenar `round(b1 + (x − b0)·l1/l0)`, yarım sıfırdan uzağa. Grubun çocukları grubun yeni çerçevesine göre, grubun çerçevesi çocuklarının birleşimi. Örnek: A3 → A1'de “Oranlı” öğe (100, 50, 60, 40 mm; kenar boşlukları kutusu 390 × 277 → 811 × 574 mm): sol `round(20 + 80·811/390) = 186,359` mm, sağ `round(20 + 140·811/390) = 311,128` mm → genişlik 124,769 mm.

**Yapışma** (`snap/basic.json`, tolerans 2 mm): adaylar sayfanın kenarları ve ortası, kenar boşlukları, kılavuzlar, öbür öğelerin sol/orta/sağ kenarları; eşitlikte kılavuz > öğe > sayfa > ızgara. Üçüncü sorguda kutu B'nin sağına 31 mm'de bırakılır; A ile B arası 30 mm olduğu için sol kenarı 140 + 30 = 170 mm'ye yapışır (1 mm düzeltme), iki aralık işareti A–B ve B–kutu arasında; üst kenar A ve B'nin üstüne (20 mm) zaten oturur.

**İsabet** (`hit/basic.json`): 40 × 20 mm kutu (100, 100) köşesinde, 45° döndürülmüş; merkez (120, 110). Köşeler `x' = cx + dx·cos − dy·sin`, `y' = cy + dx·sin + dy·cos`: kuzeydoğu (141,213; 117,071), döndürme tutamacı üst kenarın 8 mm yukarısında (132,728; 97,272). (137, 102) noktası dönmemiş çerçevenin içinde ama kutunun yerel koordinatlarında (6,364; −17,678) — dışında. Çizgi (20, 20) → (80, 50): (50; 36,5) noktası çizgiye 3/√5 = 1,342 mm; 0,25 mm yarım kalınlıkla 1 mm tolerans yetmez, 1,2 mm yeter.

**Atlas** (`atlas/plan.json`): haritanın içeriği çerçeve eksi 6 mm yazı bandı = 165 × 150 mm. 1002 nesnesi 300 × 100 m, %10 payla 360 × 120 m → gereken payda `max(360/0,165; 120/0,150) = 2181,8` → sığan en küçük standart ölçek 1/2500. 1004 nesnesi 30 × 30 km → `36 000/0,150 = 240 000`, standart ölçeklerin hiçbiri yetmez: 1/240000 ve uyarı. Dönük haritada (90°, 80 × 60 mm, önceden tanımlı 500/1000/2000) kutunun eni boyu yer değiştirir: 1001 nesnesi 50 × 40 m → `max(40/0,08; 50/0,06) = 833,3` → 1/1000.

**Eşitleme** (`sync/rules.json`): satırlar §13 tablosunun sırasıyla; eylemler kimlik sırasıyla.

**Profiller** (`profiles/modes.json`): CAD satırının söylemediği araçların (şekil, ölçek çubuğu) hazır biçimleri ortak profilden; hibrit CAD ile GIS'in birleşimi (tablo: revizyon, çizim listesi, sabit tablo). Galeri: önce proje türüne uyanlar, sonra kipin şablonları, sonra ortaklar, en sonda rozetli öbür kipinkiler; aynı sınıfta kipin galeri sırası.

**Yerleşim düzenleri** (`relayout/variants.json`): 400 × 280 yatay kâğıtta harita, sağda şerit ve şeridin üstünde logo; dikey düzen haritayı ve şeridi 200 × 280 kâğıtta anar (şerit altta). 210 × 297 dikey kâğıtta dikey düzen geçerli olur: harita iki kenar boşluğunu korur (10…200 × 10…207), şerit altta 80 mm (207…287); logoyu düzen anmaz, temel düzenden gelir: sağ kenar boşluğuna 40 mm (390 − 350), yani 200 − 40 − 30 = 130. 500 × 300 yatayda hiçbir düzen uymaz: temel düzen kısıtlarıyla (logo 490 − 40 − 30 = 420).

## Kayıtları yeniden yazmak

```bash
KENTOS_WRITE_SHEET=1 cargo test -p kentos-sheet
git diff fixtures/sheet/v1   # farkı okuyun; bilinçli değilse kaydetmeyin
```
