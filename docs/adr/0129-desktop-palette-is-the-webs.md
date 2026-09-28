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
  | `faint` (yeni) | `--c-text-3` | `#6d7988` | `#7a8591` |
  | `popover` | `--c-popover` | `#27303b` | `#fbfcfd` |
  | `success`, `warning`, `danger`, `info` | `--c-ok`, `--c-warn`, `--c-danger`, `--c-info` | web'inkiler | web'inkiler |

- Seçimin yumuşak vurgusu (`selection()`) web'in `--c-accent-soft`'udur: koyuda vurgunun %18'i, açıkta %12'si.
- **Üçüncül yazı** (`faint`, `style::text::faint`): web küçük yazılarını üçüncül tonda yazar: alanların yardımı, grup notları, menü kısayolları, sayılar. Masaüstünde bunların karşılığı `label::caption`'dır; artık bu tondadır. İpucu balonunun gövdesi web'deki gibi ikincil tonda kalır. Gece temasında `#5b636e`, yüksek karşıtlıkta ikincil yazıyla aynıdır (7:1).
- Ağaçlarda grup adları web'deki gibi ikincil tondadır (etkin satır hariç).
- Ağaç satırının göz ve kilit düğmeleri web'in `.ibtn--row`'udur (`style::button::row_toggle`): dinlenirken üçüncül tonda ve soluk; öne çıkan durumda (gizli, kilitli, seçilemez) ikincil tonda; üzerine gelince yazının renginde. Gizli ve kilitli katmanlar ilk bakışta seçilir.
- İkincil düğme web'in `.btn`'idir: alan zemininde, belirgin kenarlı. Üzerine gelince zemin açılır, kenar yazının sönük tonuna döner.
- **Marka ve sekmeler** (DESIGN.md §2 ve şerit): sekme satırındaki marka düğmesi web'in `.brand`'ıdır: lacivert karo üstünde K logosu (`widget::brand_mark`, web'in `brandMark`'ıyla aynı 24 birimlik çizim), markanın renginde "Kent" ve ince "OS", üçüncül tonda ok; zemini yoktur, üzerine gelince hafif bir katman alır, ipucu web'inkidir. Logo Başlangıç ekranında 44, uygulama menüsünde 28 pikseldir; marka renkleri (`theme::brand`) de `tokens.css`'e bağlıdır. Açık sekme ana renkte, yarı kalın ve altında 2 piksellik vurgu çizgisidir; öbür sekmeler ikincil tonda, üzerine gelince zemin açılır. Daraltılmış şeritte hiçbir sekme açık görünmez.
- **Bütün sekme şeritleri** aynı biçimdedir (sahibin isteği, 28 Eylül: "bunu tüm sekmelere uygula"; DESIGN.md §7.5): dok yığınlarının başlıkları, alt panel ve pencerelerdeki KentOS UI `Tabs`'ı (ör. İfade oluşturucunun Metin | Akış'ı) kutu çizmez. Etkin sekmenin içeriğe bakan kenarında 2 piksellik vurgu çizgisi durur; odakta olmayan dok yığınında çizgi daha solgundur. Üzerine gelinen sekme hafif bir katman alır, sekmeler arasında ayraç yoktur (`widget::tabs::tab`; `Tabs::content` kalktı).
- **Şeridin gölgesi** (sahibin isteği, 28 Eylül: "ribbon altında çok hafif gölge, sağdaki panelleri etkilemeden"; DESIGN.md'de `--shadow-bar`): şeridin altında yalnız çizim alanının (ve altındaki alt panelin) üst kenarına düşen 12 piksellik yumuşak bir koyulaşma; yandaki dok panelleri düz kalır. Koyu temada %50'den, açıkta %7'den sıfıra iner. Görünüm → Gölgeler → Kapalı'da yoktur, Belirgin'de 1,5 katıdır.
- Gece ve yüksek karşıtlık temaları masaüstünündür. Web onları ADR 0126'nın sonraki diliminde bu değerlerle alacaktır.
- **Tek kaynak:** palet `tokens.css`'te yazılıdır. Masaüstünün bir testi (`appearance_tokens_tests`) iki temanın belirteçlerini dosyadan okuyup karşılaştırır. Bir yanda değişen renk öbür yanda değişmezse test düşer. Bu, UI-08'in ilk adımıdır: belirteçler henüz tek dosyadan üretilmiyor, ama ayrılamazlar.

## Sonuçlar

- Aynı tema iki platformda aynı renklerdedir: mavi-grafit kabuk, bir ton daha derin çizim alanı, aynı kenar ve yazı tonları.
- Masaüstünün bütün ekranları değişir.
- Yazı düzeyleri web'inki gibidir: birincil, ikincil, üçüncül.
- Kalan fark: web'in şeridi araç çubuğu tonundadır (`--c-toolbar`), masaüstününki panel tonunda.

## Doğrulama

- `cargo test -p kentos-desktop appearance_tokens`: iki temanın belirteçleri (üçüncül yazı dahil) `tokens.css` ile en çok 1/255 farkla aynıdır.
- `cargo test -p kentos-ui`, `-p kentos-desktop`, `-p kentos-ui-showcase`.
- Önce ve sonra resimleri, GPU çizicisiyle (`kentos-cad snapshot`) ve ekran testleriyle (`ui_screens`, `settings_look_tests::screens`), web'inkilerle yan yana.
