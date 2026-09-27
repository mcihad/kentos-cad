# ADR 0110: Masaüstünde çizim alanının işaretleri: artı imleç, kuzey oku ve ölçek çubuğu

- **Durum:** kabul edildi (2026-09-27).
- **Tarih:** 2026-09-27
- **Bağlam belgesi:** CLAUDE.md §4.9; ADR 0029 (masaüstünde seçim ve kenet işaretleri), 0055 (çizimin yazıları).
- **Kaynak:** web'in `viewport/overlay.ts` (`drawCrosshair`, `drawNorthArrow`, `drawScaleBar`), araçların `cursor`'u (`SelectTool`, `EraseTool`, `HatchTool`, `EdgePickTool` ve türevleri, `pickObjectsTool`), `appearance.crosshair` ayarı.

## Bağlam

Web çizim alanının kaplamasına üç işaret çizer: imlecin artısı, sağ üstte kuzey oku, sağ altta ölçek çubuğu. Masaüstünde üçü de yoktu. İmleç işletim sisteminin küçük artısıydı; kuzey ve ölçek hiç gösterilmiyordu. Web'in “Artı imleç” ayarı (küçük, orta, tam ekran) masaüstüne açık değildi.

## Karar

### Araçların imleci: `kentos_interaction::Cursor`

`Cursor` üç türdür, web'in `cross`, `pick` ve `grab`'ı:

- `Cross`: nokta isteyen araçlar.
- `Pick`: nesne isteyen araçlar. Bunlar web'dekilerdir:
  - araç çalışmıyorken seçim aracı;
  - Sil ve Tarama;
  - kenar seçen araçlar: Kır, Böl, Köşe ekle/sil, Köşe yuvarla, Pah, Ötele, Buda, Uzat ve Uzat-kısalt;
  - Sahneden seç.
- `Grab`: Kaydır.

### Artı imleç: `marks.rs`

Çizim alanının üstünde işletim sisteminin imleci gizlenir (`Interaction::Hidden`). Artı, işaretlerin kaplamasında en son çizilir. Web'in `drawCrosshair`'i gibi:

- çizimin mürekkebiyle, %85 opak, 1 piksel, pikselin ortasında;
- kollar 16 (küçük), 40 (orta) ya da çizim boyunca (tam ekran);
- nesne isterken kollar %55'e kısalır (tam ekran hariç) ve ortada 10 piksellik seçim kutusu olur.

Kaydır'da el imleci görünür, artı çizilmez. Orta tuşla kaydırırken de çizilmez: kaydırma `Panned` olayıyla başlar, ilk düz `Moved` olayı onu bitirir.

Kol boyu `appearance.crosshair` ayarıdır. Şemada masaüstü de ev sahibi olur. Ayarlar penceresinin Görünüm bölümünde, Çizim zemini'nin altında durur.

### Kuzey oku ve ölçek çubuğu: `map_marks.rs`

Yazıların kaplamasına, yazılardan sonra çizilirler. Görünüşün boyu ve ölçeği değişmedikçe o kaplamanın önbelleğinde kalırlar.

**Kuzey oku:** sağ üstte grid kuzeyi. Ok yarısı dolu, yarısı çizgi; mürekkep renginde; üstünde halelı “K”.

**Ölçek çubuğu:** sağ altta.

- Uzunluk: 1, 2, 5 ya da 10 kere 10'un kuvveti; 120 piksele en yakını seçilir, eşitlikte ilki.
- Görünüş: dört dönüşümlü parça ve çerçevesi.
- Uçlarının üstünde halelı “0” ve uzunluk. Uzunluk web'in yazdığı gibidir: “2 km”, “20 m”, “5.0 cm” (`toPrecision(2)`).

## Doğrulama

- **`kentos-interaction` `tests/select.rs`:** seçim aracı, Sil, Buda, Ötele ve Tarama `Pick`; Çizgi ve Taşı `Cross`; Kaydır `Grab`.
- **`marks::crosshair_tests`:**
  - kol boyları web'inki, seçimde kısalıyor;
  - artı araçla, ayarla ve kaydırmayla değişiyor.
- **`map_marks::tests`:** ölçek çubuğunun uzunluğu ve sözleri; iki basamak kuralı.
- **Görüntüler:** `marks::crosshair_tests::screens` (`.run/shots/imlec-*`): seçim artısı, çizgi aracının artısı ve tam ekran artı; kuzey oku ve ölçek çubuğuyla; koyu ve açık tema, 1440 ve 1100 piksel.
