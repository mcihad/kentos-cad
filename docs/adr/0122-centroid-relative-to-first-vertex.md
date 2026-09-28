# ADR 0122: Alanın ağırlık merkezi ilk köşeye göre hesaplanır

- **Durum:** kabul edildi (2026-09-28).
- **Tarih:** 2026-09-28
- **Bağlam belgesi:** CLAUDE.md §23 (sayısal ve kadastral doğruluk), ADR 0008 (tek hesap kaynağı), ADR 0055 (çizimin yazıları).

## Bağlam

Masaüstünün performans ölçümü için 100 000 parselli bir çizimin resmi çekildi. Birbirinin aynısı parsellerde etiketler yukarı aşağı oynuyordu.

Etiketin yeri, ortak geometri çekirdeğinin `geometry::centroid` işlevinden gelir (`entity_anchor`). İşlev alanın ağırlık merkezini çarpımların toplamıyla bulur ve bunu mutlak koordinatlarla yapıyordu. TM koordinatlarında (x ≈ 486 000, y ≈ 4 420 000 m) çarpımlar 10¹² mertebesindedir; farkları ise küçük bir parselin alanı kadardır. Yuvarlama bu farkları siler.

Bağımsız kesin hesaba göre (Python `fractions`) hatanın büyüklüğü:

- **Ölçüm çizimi:** 25 × 18 m'lik parsellerde ağırlık merkezi 5,27 m'ye kadar kayıyordu.
- **Çekirdeğin çağrı fixture'ı:** üç TM durumunun beklentisi 4,7 cm'ye kadar yanlıştı. Bu beklentiler, yerini aldığı TypeScript çekirdeğinden kaydedilmişti ve o da aynı hatayı yapıyordu.
- **Stil derleyicisinin dondurulmuş yanıtları:** alan merkezine konan işaretler 0,985 m'ye kadar yanlıştı.

Alanın kendisi (`signed_area`) bu hatadan önceden kurtarılmıştı: koordinatları ilk köşeye göre alır. İfade motorunun `$merkez_x`/`$merkez_y`'si de öyledir. Hatalı olan yalnız `centroid`'di. Onu kullananlar:

- çizimin etiketleri (iki platform, ortak depo);
- ifadelerde alanın çapası (`$x`, `$y`);
- sembol işaretlerinin alan merkezine yerleşimi (stil çekirdeği, `place.rs`);
- çekirdeğin `centroid` çağrısı.

## Karar

- `geometry::centroid` koordinatları ilk köşeye göre alır ve sonucu geri taşır, `signed_area` gibi. Alanı olmayan halkada köşelerin ortalaması da ilk köşeye göredir. Boş halka NaN'dır, eskisi gibi.
- **Çağrı fixture'ı:** `fixtures/geometry/v1/calls-p1-primitives.json`'daki üç TM beklentisi kesin değerlerle değiştirildi (Python `fractions`; float'a doğru yuvarlanmış). TypeScript çekirdeği artık yok. Yeni hesap kesin değere 1e-9'dan yakındır.
- **Stil derleyicisinin yanıtları:** `fixtures/style/v1/cases.json` web'in kaydedicisiyle yeniden yazıldı (`record-style.test.ts`, WASM'daki düzeltilmiş çekirdek).
  - Fark okundu. Değişenler alan merkezindeki işaretlerin konumları ve kaydedicinin bir alan merkezinden kurduğu bir kırpma kutusudur.
  - Değişen 31 işaret kesin ağırlık merkezlerine 2,8·10⁻¹⁴ m içinde oturur; eskileri 0,985 m'ye kadar uzaktı. Yaylı kenarlı 20 durum ayrıca hesaplanmadı: onlarda halka çekirdekte yoğunlaştırılır, aynı işlevden geçer.
  - Kaydedici, fixture kaydedildikten sonra gelen nesne kimliklerini (`uid`, ADR 0014) de yazıyor. Yanıtlar kimliğe bağlı değildir.

## Sonuçlar

- Etiketler, işaretler ve `$x`/`$y` TM koordinatlarında da kesin yerindedir. İki platform aynı düzeltilmiş çekirdeği kullanır (web WASM'la).
- Mutlak koordinatlarla çarpım kuran başka hesap kalmadığı ayrıca taranmalıdır. `signed_area`, ifade motorunun momentleri ve bu işlev artık ilk köşeye göredir.

## Doğrulama

- `cargo test -p kentos-geometry-core --test calls`: 6452 çağrı; üç TM ağırlık merkezi kesin değerleriyle.
- `cargo test -p kentos-style-core --test style` ve web'de `pnpm test` (`src/style/fixture.test.ts`, `src/wasm/calls.wasm.test.ts`) aynı dosyaları okur.
- Bağımsız denetim: yukarıdaki Python hesabı (`fractions`) ADR'nin sayılarını verir. Ölçüm çiziminin parsellerinde yeni hesabın kesin değerden farkı 0'dır (f64 çözünürlüğünde).
