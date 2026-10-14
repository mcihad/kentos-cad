# ADR 0270: Nokta yapımları, anlamsal API'leri ve geçici izleme noktası

- **Durum:** kabul edildi (2026-10-14). [plan/cad.md](../../plan/cad.md) §7 `CAD-INPUT-06`: “Extension, parallel, iki noktanın ortası ve
  tracking acquire gibi ek snap yöntemlerini mevcut destek üzerinden tamamla; eksik yöntemleri semantik API'ye ekle.” Kapsamı ben
  belirledim.
- **Bağlam belgesi:** ADR 0083 (nokta hesaplayıcı), 0085 (nesne izleme), 0163 (Uzantı, Paralel), 0166 §5 (Referans noktası), 0249 ve
  0263 (nesnenin yerleri), 0253 (komut protokolü), 0261 (UCS), 0269 (kenet kapsamı).
- **Web:** ertelendi (ADR 0228); TODOS.md §0.2.

## Bağlam

Arayüzde Uzantı ve Paralel kenetleri (ADR 0163), nesne izleme (ADR 0085), Referans noktası (ADR 0166 §5) ve nokta hesaplayıcının İki
nokta ortası (ADR 0083) vardı. İki eksik vardı:

1. **Anlamsal API yoktu.** Python ve yapay zekâ ajanı bir noktayı “bu çizginin ucundan 5 m ötede” ya da “şu noktadan bu kenara paralel
   3 m” diye isteyemiyordu; geometriyi kendileri hesaplamak zorundaydı (CLAUDE.md §9.2'nin “geometriyi kendileri hesaplamadan” kuralına
   aykırı).
2. **Geçici izleme noktası yoktu.** AutoCAD'in TT'si (Temporary track point) gibi, bir noktayı yerleştirmeden izleme noktası yapmak için
   imleci onun üstünde bekletmek gerekiyordu; Nesne izleme kapalıyken hiç olmuyordu. İki nokta ortası ve Referans noktası (AutoCAD'in
   From'u) tek seferlik kenet menüsünde (Shift+sağ tık) değildi.

## Karar

### 1. `cad.points.construct` v1

Salt okunur ürün komutu (`effect: query`, geri alma adımı yok): yapımların noktalarını sırasıyla verir. Noktalar koordinat ya da
nesnenin yeri olarak (`MeasuredPoint`: `at` ya da `place`, blok içi parça yoluyla), ölçüler metre, açı UCS'in x ekseninden saat
yönünün tersine radyan.

| Yapım | Nokta |
|---|---|
| `mid { a, b }` | iki noktanın ortası |
| `extension { end, distance }` | nesnenin açık ucundan (çizginin, açık çoklu çizginin, yayın; yaylı son kenarın da) kenarı boyunca `distance` (≥ 0) öte: düz kenarın doğrusu, yayın çemberi (uzaklık çember boyunca) |
| `parallel { through, edge, distance }` | `through`'dan, nesnenin `edge.near`'a en yakın düz kenarı doğrultusunda (ilk noktasından ikincisine) işaretli `distance`; eğrinin, elipsin ve NURBS'ün kirişleri kenar sayılmaz |
| `track { x, y }` | UCS'te `x`'in x'i, `y`'nin y'si (nesne izlemenin iki hizasının kesişimi; AutoCAD'in .X ve .Y süzgeçleri) |
| `polar { from, angle, distance }` | `from`'dan açıyla uzaklık |
| `along { a, b, distance }` | `a`'dan `b`'ye doğru işaretli `distance` |

- Retler yapım yapım sırayla: `no_constructions` (yok ya da 10 000'den çok), `not_finite`, `invalid_distance`, `invalid_uid`,
  `entity_not_found`, `no_such_place`, `piece_gone`, `not_an_end`, `no_straight_edge`, `same_points`; yer `constructions[i]…`. Biri
  reddedilince hiçbir nokta verilmez.
- `plan` aynı cevabı verir; `validate` yalnız denetler. Masaüstünün komut kaydında, başsız sunucuda, Python'da
  (`kentos.cad.points.construct`) ve MCP'de (salt okunur araç, `readOnlyHint`) aynı işleyici.
- Yeni kodlar: `no_constructions`, `not_an_end`, `no_straight_edge` (`codes.rs`).

### 2. Araçların kuralıyla aynı

- İki nokta ortası nokta hesaplayıcının (ADR 0083) kuralıdır; Uzantı ve Paralel kenetlerinin noktaları (ADR 0163) aynı doğrular
  üstündedir; `track` nesne izlemenin iki hizasının kesişimidir (ADR 0085). Komut bu kuralları yeniden yazmaz, aynı tanımları verir;
  ortak durumlar bağımsız başvurudan.

### 3. Geçici izleme noktası ve tek seferlik menü

- `draft.trackPoint` (Geçici izleme noktası): nokta bekleyen bir komut çalışırken sonraki tıklama ya da yazılan nokta köşe olmaz, nesne
  izlemenin noktası olur (`ObjectTracking::acquire_point`); imleç onun yatay ve dikey hizalarına ve kutupsal açılarına oturur,
  **Nesne izleme kapalıyken de**; komut bitince kalkar. İstem “geçici izleme noktasını belirtin”; Esc önce isteği bırakır. Komutu
  yokken söylenir (`NO_TRACK_TOOL`).
- Tek seferlik kenet menüsünde (Shift+sağ tık) kenet türlerinden sonra: Geçici izleme noktası, Referans noktası ve İki nokta ortası.

## Sonuçlar

- Python ve ajan, çizim yardımlarının noktalarını geometriyi kendileri hesaplamadan alır; aynı noktayı araç da verir.
- Geçici izleme noktası Nesne izleme'yi açmadan hizalama sağlar; AutoCAD'in Shift+sağ tık menüsünün nokta öğeleri tamamlandı.

## Doğrulama

- Bağımsız başvuru `python3 scripts/fixtures/point_construct_cases.py --check` (`fixtures/commands/v1/cad.points.construct.json`, 30
  durum: altı yapım, on ret, plan ve validate, sonsuz sayılar); masaüstünde `cargo test -p kentos-native-application` (ortak durumlar,
  yeteneklerin tür denemesi: Paralel'in düz kenarlı türleri).
- `pnpm py:test` (Python'un tipli katmanı), `cargo test -p kentos-mcp`.
- Etkileşim izi `fixtures/interaction/v1/track-point.json` masaüstünde üç çeşitte (Nesne izleme kapalıyken geçici izleme noktası,
  İki nokta ortası).
- Resim: `cargo test -p kentos-desktop drawing_menus::screens -- --ignored --nocapture` (`.run/shots/sag-tik-kenet-*`: tek seferlik
  menünün yeni öğeleri).
