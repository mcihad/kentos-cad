# ADR 0129: Masaüstünün koyu ve açık teması web'in paletidir

- **Durum:** kabul edildi (2026-09-28).
- **Tarih:** 2026-09-28
- **Bağlam belgesi:** DESIGN.md §3.1–3.2 (renkler), §7 (düğmeler); ADR 0126 (ortak görünüş anahtarları), ADR 0127 (arayüzün biçimi); TODOS.md UI-08 (tek tasarım belirteci kaynağı).
- **Sahibin isteği (28 Eylül):** "Web tarafı arayüzü gayet şık; masaüstü tarafını iyileştireceğiz."

## Bağlam

ADR 0126'dan beri iki platform aynı `appearance.theme` değerini okur: "Koyu grafit" ve "Açık pafta" iki yanda aynı adı taşır. Renkler ise ayrıydı:

- Masaüstünün koyu teması nötr bir grafitti (pencere `#1d1f23`, panel `#282b30`, alan `#18191c`).
- Web'inki DESIGN.md §3.1'in mavi-grafitidir (menü çubuğu `#181e26`, panel `#212932`, alan `#171c23`). Çizim alanı ise iki yanda zaten web'in `#141a21`'iydi.
- Açık temada da yüzeyler, kenarlar ve yazılar birkaç ton ayrıydı.

Aynı temayı seçen kullanıcı iki platformda iki ayrı arayüz görüyordu. ADR 0127 bunu kalan fark olarak yazdı. DESIGN.md iki platformun ortak kaynağıdır.

## Karar

- KentOS UI'ın `Tokens::DARK` ve `Tokens::LIGHT`'ı web'in `tokens.css`'indeki değerlerdir:

  | KentOS UI | Web | Koyu | Açık |
  |---|---|---|---|
  | `window` (sekme şeridi, durum çubuğu) | `--c-menubar` | `#181e26` | `#e3e7eb` |
  | `surface` (şerit, paneller, pencereler) | `--c-panel` | `#212932` | `#f4f6f8` |
  | `header`, `surface_alt` | `--c-panel-head` | `#252e38` | `#eaeef1` |
  | `surface_hover` | `--c-panel` üstünde `--c-hover` | `#2d353d` | `#e7e9ec` |
  | `field` | `--c-field` | `#171c23` | `#ffffff` |
  | `border` | `--c-line` | `#2d3641` | `#d3d9df` |
  | `border_strong()` | `--c-line-strong` | `#3d4856` | `#b5bfc9` |
  | `text`, `muted` | `--c-text`, `--c-text-2` | `#d6dde5`, `#9ba7b5` | `#1b232c`, `#4d5966` |
  | `popover` | `--c-popover` | `#27303b` | `#fbfcfd` |
  | `success`, `warning`, `danger`, `info` | `--c-ok`, `--c-warn`, `--c-danger`, `--c-info` | web'inkiler | web'inkiler |

- Seçimin yumuşak vurgusu (`selection()`) web'in `--c-accent-soft`'udur: koyuda vurgunun %18'i, açıkta %12'si.
- İkincil düğme web'in `.btn`'idir: alan zemininde, belirgin kenarlı. Üzerine gelince zemin açılır, kenar yazının sönük tonuna döner.
- Gece ve yüksek karşıtlık temaları masaüstünündür. Web onları ADR 0126'nın sonraki diliminde bu değerlerle alacaktır.
- **Tek kaynak:** palet `tokens.css`'te yazılıdır. Masaüstünün bir testi (`appearance_tokens_tests`) iki temanın belirteçlerini dosyadan okuyup karşılaştırır. Bir yanda değişen renk öbür yanda değişmezse test düşer. Bu, UI-08'in ilk adımıdır: belirteçler henüz tek dosyadan üretilmiyor, ama ayrılamazlar.

## Sonuçlar

- Aynı tema iki platformda aynı renklerdedir: mavi-grafit kabuk, bir ton daha derin çizim alanı, aynı kenar ve yazı tonları.
- Masaüstünün bütün ekranları değişir.
- Kalan farklar:
  - Web'in üçüncül yazısı (`--c-text-3`) masaüstünde yoktur; notlar ikincil yazıyla yazılır.
  - Web'in şeridi araç çubuğu tonundadır (`--c-toolbar`), masaüstününki panel tonunda.

## Doğrulama

- `cargo test -p kentos-desktop appearance_tokens`: iki temanın belirteçleri `tokens.css` ile en çok 1/255 farkla aynıdır.
- `cargo test -p kentos-ui`, `-p kentos-desktop`, `-p kentos-ui-showcase`.
- Önce ve sonra resimleri, GPU çizicisiyle (`kentos-cad snapshot`) ve ekran testleriyle (`ui_screens`, `settings_look_tests::screens`), web'inkilerle yan yana.
