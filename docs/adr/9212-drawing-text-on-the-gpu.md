# ADR 9212: Çizim yazıları GPU'da (tasarım)

- **Durum:** önerildi, aşama aşama uygulanıyor (2026-11-24). Plan: [plan/render.md](../../plan/render.md) RM-13.
- **İstek:** sahibin seçimi (24 Kasım 2026): “Çizim yazıları GPU'da”. İlk istekteki hedef: “çizim alanı saf wgpu
  olmalı”.
- **Bağımlılık:** ADR 0055 (masaüstünde çizim yazısı), 0182 (paragraf), 0196 (eğri boyunca yazı), 0212 (etiket
  motoru), 0292 (ayna kuralı), 0597 ve 0626 (hareket boyunca yerleşim), 0786 (küçük yazı bandı), 9207 §5.

## §1 Bugün

- **Kim çiziyor:** çizim yazıları (nesne yazıları, ölçü değerleri, etiketler, eğri boyunca harfler, çağrı kutuları)
  `apps/desktop/src/cad/labels` içinde iced'in `Canvas`'ına çiziliyor (5 252 satır).
  - Dik yazı iced'in glif önbelleğinden (cosmic-text) gelir.
  - Döndürülmüş yazı, glif dış hatları olarak `Path` ile doldurulur.
  - Hepsinin çevresinde zemin renginde bir hale var.
- **Yerleşim:** paylaşılan geometri deposundan gelir (`Spatial::labels`): görünür katman, görünümdeki kutu,
  ekrandaki boy, ölçek aralığı ve etiketlerin seyreltilmesi. Nesne yazısı ve ölçü değeri hiç düşmez.
- **Maliyet** (ADR 9207 §5, NCZ, UHD 770):
  - yazılar çizilmeyince uzak görünümün tekerlek karesi 76–93 ms'den 54–90 ms'ye iniyor;
  - yakın karelerde 20 ms'ye kadar iniyor, yani yakında yazılar karenin çoğu.
  - Kanvas her değişen karede yeniden üçgenleniyor; tuvali arayüz kütüphanesine bağlayan son büyük parça bu.
- **Yazı tipleri:** çizicinin gömülü TrueType yüzleri (`kentos_plot::fonts::FACES`). Ekran ve PDF aynı baytları
  kullanır.

## §2 Seçenekler

| Seçenek | Yakında | Hale | Bağımlılık | Not |
|---|---|---|---|---|
| Bugünkü kanvas | keskin (vektör) | yol olarak ikinci dolgu | iced | her karede üçgenleme, arayüz katmanında |
| Boyuna göre bitmap atlası (glif önbelleği gibi) | büyütülünce bulanık | ayrı bitmap | yok | SVG simgesindeki sorunun aynısı (ADR 9208) |
| Tek kanallı SDF atlası | ~8× büyütmeye dek keskin, sonra köşeler yuvarlanır | eşiğin kaydırılmasıyla bedava | yok (ttf-parser zaten ağaçta) | harita motorlarının yolu |
| MSDF atlası | çok büyük büyütmede de köşe keskin | aynı | dış kütüphanenin (fdsm) lisansı belirsiz; kendi üreteç ~600 satır | kenar renklendirme ve hata düzeltme ister |
| Vektör glif ağı (ADR 9208'in yolu) | her boyda keskin | dış hattın kalınlaştırılması | yok | küçük boyda 4× MSAA ince çizgiyi kırar (9208'in bulgusu) |

## §3 Karar

**Tek kanallı SDF glif atlası; çok büyük yazı için vektör glif.** Yerleşim ve düzen CPU'da bugünkü gibi kalır;
yalnız **çizim** GPU'ya geçer.

- **Atlas** (`kentos-render-wgpu`):
  - Her (yüz, glif) bir kez SDF'e çevrilir: 64 px'lik em, 8 px yayılma, R8 doku.
  - Kaynak, çizicinin yüzlerinin dış hatlarıdır (ttf-parser).
  - Sayfa dolarsa yeni sayfa açılır; bitmap atlasındaki gibi boy küçültülmez.
- **Çizim:**
  - Glif başına örneklenen bir dörtgen: yer (yüksek/alçak kesin), boy, döndürme ve eğim (ayna ve italik dahil),
    renk, hale rengi ve kalınlığı.
  - Kenar yumuşatma, ekran türeviyle (`fwidth`) yapılır; MSAA'ya gerek kalmaz.
  - Hale, aynı örnekte eşik kaydırılarak çizilir.
- **Çok büyük yazı:** glif ekranda ~512 px'i geçince, ADR 9208'in yoluyla vektör olarak (üçgenlenmiş dış hat)
  çizilir. Köşeler her boyda keskin kalır.
- **MSDF** ileride, köşe yuvarlanması ölçüyle görülürse SDF'in yerine geçebilir; arabirim aynı kalır.
- **Kural korunur:** nesne yazısı ve ölçü değeri hiç düşmez. Okunamayacak kadar küçük yazı bugünkü gibi banttır
  (ADR 0786).

## §4 Aşamalar

1. **Atlas ve gölgelendirici** (`kentos-render-wgpu`): SDF üretimi, glif örneği, hale, GPU sınaması (bir yazı
   döndürülmüş ve haleli, bütünüyle çizilmiş referansla karşılaştırılır).
2. **Nesne yazıları ve parçaları** (`LabelSpot::Text`, `PieceText`): masaüstü, glif örneklerini bugünkü paragraf
   düzeninden kurar ve yazının kanvas yolu kalkar. Görüntü önce ve sonra karşılaştırılır.
3. **Ölçü değerleri, etiketler, eğri boyunca harfler, hücreler, çağrı kutuları:** her biri ayrı adım.
4. **Kanvas katmanı kalkar.** Tekerlek ve kaydırma karelerinde yazılar da canlı kamerayla GPU'da çizilir.

Her aşama kendi ölçümüyle (yakın ve uzak tekerlek karesi, yazı sayısı) ve gerçek pencere görüntüleriyle kapanır.

## §5 Aşama 1: atlas ve gölgelendirici (24 Kasım 2026)

**Yapılan** (`crates/render/wgpu/src/glyphs/`):

- **Alan** (`sdf.rs`): glifin dış hattı 64 px'lik eme düzleştirilir. Her pikselin en yakın parçaya uzaklığı, sıfırdan
  farklı sarım kuralıyla işaretlenir; kenarın iki yanındaki 8 px bir bayta yazılır (128 kenar).
  - Her parça yalnız kendi kutusunun 8 px yakınındaki piksellere bakar; işaret satır satır, kesişimler sıralanarak
    bulunur.
  - Sonuç, her pikseli her parçayla deneyen hesapla bayt bayt aynıdır (sınama 10 glifte karşılaştırır).
- **Atlas** (`atlas.rs`): anahtar, geometri çekirdeğinin yazı tipi kimliği ve glif numarasıdır (`FontId`, glif).
  - Kaynak çekirdeğin paketli yüzleri: 22 çizim yüzü ve 7 yedek yüz. Çizicinin yüzleri (`kentos_plot::fonts::FACES`)
    bunların aynısı; ekran ve PDF aynı baytları kullanmaya devam eder.
  - Alanlar 2048 px'lik tek baytlı sayfalara raf raf dizilir. Sayfa dolunca yenisi açılır; hiçbir alan yeniden
    yapılmaz, hiçbir sayfa baştan başlamaz.
  - Sayfalar CPU'da da tutulur (sayfa başına 4 MiB). Doku yeni bir sayfa için büyütülünce bu kopyadan doldurulur.
  - Alan yapımı bir son anla sınırlanabilir. O ana yetişmeyen glifler o kare çizilmez ve katman “bekleyen” der;
    masaüstü katmanı sonraki karede yeniden kurar.
- **Glif örneği** (`gpu.rs`, `glyphs.wgsl`; 88 bayt): yazının dünya çapası (iki float32 parça, katmanın 65 536 m'lik
  ızgaradaki kökeninden), emi çıktıya götüren 2×2 matris, alanın kutusu ve atlastaki yeri, yazı ve hale rengi, hale
  kalınlığı, sayfa ve birim.
  - **İki birim:** dünya metresi (nesne yazısı: yakınlaşınca büyür, görünümle döner) ve mantıksal piksel (etiket:
    boyunu korur; isteğe göre görünümle döner ya da ekranda yerleştirildiği gibi kalır).
  - Çekirdeğin glif planı (`text::shaping::placed`) aynı kimliği taşıdığı için masaüstü planı doğrudan örneğe
    çevirebilir (`GlyphPlace::of`: glifin boyu, yapay eğimi ve başlangıcı, sonra planın dönüşümü).
- **Gölgelendirici:** alan tek örneklenir. Uzaklık ekran pikseline çevrilir; kenar ve hale bir piksel boyunca
  yumuşatılır. Renk önçarpımlı karıştırılır; MSAA gerekmez.
- **İki sınır:**
  - Alan kenardan em'in sekizde biri kadar uzanır; daha kalın hale ona kesilir. 1,5 px'lik hale 12 px'lik emden
    itibaren tamdır, daha küçük yazıda incelir.
  - 4 px'ten küçük emde yumuşatma alanın erimine sığsın diye biraz keskinleşir. Bundan küçük yazı zaten bant olarak
    çizilir (ADR 0786).

**Ölçüm (release, alan yapımı):**

- bir alan 0,24 ms;
- bir yüzün Latin ve Türkçe takımı (107 glif) 25 ms;
- 22 çizim yüzünün hepsi 2 332 alan, 553 ms, 3 sayfa.

İlk açılışta birkaç yüz kullanan bir çizim ~50–80 ms alan yapar. Bu yüzden ikinci aşamada masaüstü son anı kullanacak.

**GPU sınaması** (`tests/gpu/glyphs.rs`, UHD 770, Vulkan; `KENTOS_GPU_TESTS=1`):

| Sınama | Sonuç |
|---|---|
| “Kentos Ağ 17”, 30° döndürülmüş, 1,5 px haleli, ölçme ızgarası sayılarında; aynı dış hatlar CPU'da bütünüyle doldurulup hale yuvarlak birleşimli çizgiyle çizilerek karşılaştırıldı | Renk kanallarında ortalama fark 0,24 / 255; dörtte birden fazla sapan piksel 0; hale pikseli 926 / 971 |
| Aynı yazı dünya ve piksel biriminde, ölçek iki katına çıkınca | Dünya yazısının mürekkebi ×3,998, piksel yazısınınki ×1,007 |
| 2000 px/m'de, yani katmanın float32 adımı 3,9 px iken, ¼, ½ ve 1 piksellik kaydırma | 0,276 / 0,252; 0,522 / 0,515; 1,000 / 1,000 px |
| 160 px'lik sayfalara bölünmüş atlas (13 sayfa) ile tek sayfa | Bayt bayt aynı görüntü |

- Çeyrek ve yarım pikseldeki ~0,02 px'lik sapma, harfler piksel ızgarasını geçerken alanın örneklenmesinden geliyor:
  tam pikselde sıfır.
- Görüntüler: `.run/gui-evidence/render/rm13/` (GPU ile referans yan yana: `yan-yana.png`).
