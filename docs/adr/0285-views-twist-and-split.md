# ADR 0285: Adlı görünümler, döndürülmüş görünüm ve bölünmüş görünüm

- **Durum:** kabul edildi (2026-10-14). [plan/cad.md](../../plan/cad.md) §7 `CAD-INPUT-21`: “Zoom extent/window/previous/selection, named
  view, döndürülmüş görünüm ve birden çok model viewport iş akışlarını tamamla.” Kapsamı ben belirledim.
- **Bağlam belgesi:** ADR 0056 (Pencere yakınlaştır, Seçime yakınlaştır), 0141 (Önceki ve Sonraki görünüm, Katmana ve Gruba
  yakınlaştır), 0181 (Genel bakış ve Büyüteç), 0261 (UCS), 0265 (yeni şemanın kuralı), 0283 (şeffaf komutlar).
- **Web:** ertelendi (ADR 0228); TODOS.md §0.2.

## Bağlam

Tümünü göster, Pencere yakınlaştır, Seçime yakınlaştır, Katmana ve Gruba yakınlaştır, Önceki ve Sonraki görünüm vardı. Görünümü adıyla
saklamak, döndürmek (AutoCAD'in view twist'i ve PLAN'ı) ve çizim alanını iki görünüme bölmek yoktu.

## Karar

1. **Adlı görünümler projenin ayarıdır:** bir görünüm adı, ortadaki dünya noktası, gösterilen dünya eni ve boyu ve dönüşüyle (ekranın x
   ekseninin doğudan saat yönünün tersine açısı, [0, 2π)) saklanır; en çok 100, ad 1–64 karakter ve tekil. `.kcad` şema 52
   (`settings.namedViews`, §6.4.12; yazıcı 52'yi yalnız adlı görünüm varken yazar, eski şemada alan bilinmeyen alandır). Yazma
   `cad.views.edit` ile (`keep`, `rename`, `remove`; geri alma adımı değil, ayar), ortak durumlar KentOS kodu olmadan yazılmış başvurudan
   (`scripts/fixtures/views_command_cases.py`, 22 durum), Python (`kentos.cad.views.edit`) ve MCP. Görünüme gitmek kameranındır, çizimi
   değiştirmez; Önceki görünüm oradan geri getirir.
2. **Görünümler penceresi** (`view.namedViews`, Görünüm sekmesi): liste, Ad, Göster (ya da çift tık), Güncelle, Yeni görünüm kaydet,
   Yeniden adlandır, Sil, Kapat; pencere açık kalır.
3. **Döndürülmüş görünüm:** kamera dönüşüyle (`crates/render/wgpu` kamerası ve stilli çizim hattı dönüşü taşır; yazılar, işaretler,
   tutamaçlar, seçim kutusu, kenet ve tıklama dönmüş görünümde doğru yerde). Görünümü döndür aracı (`tool.viewTwist`): iki nokta ekranın
   x eksenini verir ya da açı yazılır (projenin açı birimi ve yönüyle); UCS'e göre (U) UCS'in x eksenini yatay yapar, Kuzey yukarı (K)
   geri çevirir. Ayrıca `view.northUp` (şeritte) ve `view.twistUcs` (komut satırında ve aracın U'su; şeridin Görünüm sekmesi geniş
   pencerede tam sığsın diye şeritte değil). Araç görünümü değiştirir, çizimi değil; çalışan komutu bitirmeden çalışır
   (ADR 0283).
4. **Bölünmüş görünüm** (`view.split`): çizim alanı yan yana iki model görünümü olur, her biri kendi kamerasıyla; biri etkindir (imleç,
   çalışan araç, kenet, işaretler, kartlar ve görünüm komutları onundur). Öbür görünüme basmak onu önce etkin yapar ve basma ona gider;
   tekerlek ve orta tuş öbür görünümü etkin yapmadan taşır. Başka çizim açmak tek görünüme döner.
5. **Kısayol listesi:** “Öbür görünüme tık” satırı.

## Sonuçlar

- Görünümler projeyle taşınır; bulut, Python ve ajan aynı komutla yazar.
- Açık kalan: ikiden çok görünüm alanı (2×2 düzen), görünüm alanlarının boyunun sürüklenmesi, görünüm alanına göre katman görünürlüğü.
  Bunlar `CAD-SHEET` ve görünüm alanı modeliyle birlikte ele alınacak.

## Doğrulama

- `python3 scripts/fixtures/views_command_cases.py --check`, `python3 scripts/fixtures/kcad_v2_reference.py --check`,
  `cargo test -p kentos-kcad`, `cargo test -p kentos-native-application`, `cargo test -p kentos-interaction --test all view_twist`,
  `cargo test -p kentos-render-wgpu`, `cargo test -p kentos-mcp`, `pnpm py:test`; bütün izler değişmeden geçer.
- Resimler: `KENTOS_SHOTS_ONLY=gorunumler,gorunum-dondurulmus,gorunum-bolunmus cargo test -p kentos-desktop tools_screens -- --ignored --nocapture`.
