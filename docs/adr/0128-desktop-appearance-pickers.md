# ADR 0128: Masaüstünde Görünüm'ün seçicileri: tema kartları, vurgu örnekleri, yazı tipi kartları

- **Durum:** kabul edildi (2026-09-28).
- **Tarih:** 2026-09-28
- **Bağlam belgesi:** DESIGN.md §7 (Uygulama ayarları'nın denetimleri); ADR 0126 (ortak görünüş anahtarları), ADR 0127 (arayüzün biçimi); TODOS.md UX-13 (Görünüş); docs/inventory/parity-audit.md A5.
- **Sahibin isteği (28 Eylül):** "Masaüstü arayüzünü çok daha şık ve estetik hâle getirelim; web tarafı gayet şık."

## Bağlam

Web'in Uygulama ayarları → Görünüm'ü seçenekleri örnekleriyle gösterir:

- temalar etkin temadan bağımsız renkli küçük çalışma alanlarıdır;
- vurgular yarısı koyu, yarısı açık temanın tonunda yuvarlak örneklerdir;
- yazı tipleri kendi harfleriyle yazılmış kartlardır;
- yazı boyutu beş adımdır.

Masaüstü aynı ayarları açılır kutularla, 14 piksellik renk noktalarıyla ve bir piksel alanıyla soruyordu. Seçeneği görmeden seçmek gerekiyordu. ADR 0126 anahtarları birleştirdi; bu karar görüntüyü birleştirir.

## Karar

Masaüstünün Görünüm bölümü web'in sırasıyla ve biçimiyle kurulur (`apps/desktop/src/settings_look.rs`). Her seçici, öbür alanlar gibi pencerenin taslağına yazar; ayar Kaydet'le uygulanır (web de öyle).

- **Tema:** dört tema kartı yan yana, satırı doldurur.
  - Her kart, temanın kendi renklerinde küçük bir çalışma alanıdır: üst çubuk, sağda panel ve yazı satırları, yüzen araç kutusu (ilk araç vurgu renginde), çizimde eğik iki parsel ve vurgu renginde kesikli bir seçim. Temanın jetonlarından (`Tokens::base`) ve çizim alanının paletinden çizilir.
  - Yazı satırları ile boştaki araçlar panelle yazı arasında soluk bir tondadır (web'in `--m-dim`'i); yüksek karşıtlık temasında da beyaz çubuk olmaz.
  - Seçili kart vurgu renginde iki piksellik kenar alır, üzerine gelinen belirgin kenar.
  - Altında masaüstünün Çizim zemini seçimi durur.
- **Vurgu rengi:** on hazır renk beşerli iki satırda. Her örnek 30 piksellik bir dairedir: sol üst yarısı koyu temaların, sağ alt yarısı açık temanın tonu (web'in 135°'si). Seçilen örnek işaretlidir, çevresinde halka vardır; seçenek alan zemininde ve kenarlıdır.
  - Yanında **Özel renk**: `#RRGGBB` alanı ve rengin örneği. Okunan renk seçili görünür. Henüz okunmayan yazı olduğu gibi kalır; altında "Okunamadı: #RRGGBB biçiminde yazın (ör. #2F6FD0)." kırmızıyla yazar. Bu, masaüstünün web'den fazlasıdır (ADR 0126: seçeneklerin birleşimi).
- **Yazı tipi:** yedi kart, dörderli satırlarda. Her kart kendi yazı tipinde, yarı kalın "Ağ Şı İ 123" örneği, adı ve web'in notunu yazar. Not iki satırlık yerde sarar; bir satırın kartları aynı boydadır. Seçili kart yumuşak vurgu zemininde ve vurgu kenarlıdır.
  - Altında masaüstünün Eş aralıklı yazı seçimi ve **Yazı boyutu** durur. Yazı boyutu web'in beş adımıdır (Küçük 12, Standart 13, Büyük 14, Çok büyük 15, En büyük 16). Yanındaki piksel alanı aradaki ve dıştaki boyları da yazar (11–18); o zaman hiçbir adım seçili görünmez. Dar pencerede alan adımların altına iner.
- Biçim (köşeler, gölgeler), İmleç ve fare yardımcıları ve Açılış eskisi gibi form satırlarıdır.
- Adlar ve açıklamalar ortak şemadan okunur: temaların adları `appearance.theme`'in seçeneklerinden, grup açıklamaları ayarların açıklamalarından gelir. Yazı tiplerinin notları web'in `UI_FONTS`'undadır.
- Grupların başlıkları formun bölüm başlıkları gibidir: yarı kalın ad, yanında ince çizgi. Gruplar arası 18 piksel boşluktur, formdaki bölümler gibi.

## Sonuçlar

- Masaüstünde tema, vurgu ve yazı tipi seçmeden önce görülür; iki platformun Görünüm'ü aynı sırayı ve biçimi izler.
- Web'den farklar:
  - Masaüstü dört temayı, on vurguyu ve özel rengi gösterir. Web şimdilik iki tema ve dört vurgu gösterir; kalanları web'e getirmek ADR 0126'nın sonraki dilimidir.
  - Web'in Arayüz düzeni kartları masaüstünde yoktur: masaüstünde yalnız şerit vardır.
- Eski açılır kutular ve renk noktaları (`accent_choice`) kalktı.

## Doğrulama

- `cargo test -p kentos-desktop settings_look_tests`:
  - kartlar şemanın tema ve yazı tipi kimliklerini aynı sırayla yazar; yazı boyunun adımları şemanın aralığındadır;
  - fareyle: "Açık pafta" kartı, "Mavi" örneği, "Inter" kartı ve kaydırılıp tıklanan "Büyük" adımı taslağa yazar; Kaydet'ten önce kayıtlı ayar değişmez.
- `cargo test -p kentos-desktop settings_look_tests::screens -- --ignored --nocapture`: `.run/shots/gorunum-*`:
  - kartlar, yazı tipleri ve bölümün sonu koyu ve açık temada, 1440×900 ve 1100×650'de;
  - özel renk ve okunamayan renk;
  - 1100×650'de 16 piksel yazı: kesik ya da taşan yok.
- GPU çizicisiyle: `kentos-cad snapshot … --komut tools.options`.
- `cargo test -p kentos-desktop`, `cargo clippy -p kentos-desktop --all-targets -- -D warnings`.
