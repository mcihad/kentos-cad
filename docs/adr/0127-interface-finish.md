# ADR 0127: Arayüzün biçimi: köşeler, gölgeler, menüler, odak ve uzun metin

- **Durum:** kabul edildi (2026-09-28).
- **Tarih:** 2026-09-28
- **Bağlam belgesi:** DESIGN.md §3.3 (vurgu kuralları), §5 (yarıçaplar, gölgeler); ADR 0051 (masaüstünün görünümü), ADR 0126 (ortak görünüş anahtarları).
- **Sahibin isteği (28 Eylül):** "Masaüstü arayüzünü çok daha şık ve estetik hâle getirelim; web tarafı gayet şık. Açılır listelerde ve bağlam menülerinde hafif gölge ve her yerde yarıçap olabilir, ama bu Görünüm ayarlarından ayarlanabilmeli. Odaklanan öğe daha şık gösterilebilir; açılır kutular ve öbürleri uzun metinlerde daha tutarlı olabilir."

## Bağlam

Masaüstü (KentOS UI) ile web yan yana konunca farklar şunlardı:

- **Köşeler:** KentOS UI'da bütün köşeler tek bir değerdeydi (`RADIUS = 2`); pencereler neredeyse sivriydi. Web kademelidir: `--r-xs` 2, `--r-sm` 4, `--r-md` 6, `--r-lg` 10.
- **Gölgeler:** menülerin gölgesi çizilse de menü, kutusunu kırpan bir katmanda çizildiği için kesiliyordu. Pencerelerin gölgesi menülerinkiyle aynıydı.
- **Menüler:**
  - üzerine gelinen satır dolu vurgu zeminindeydi; DESIGN.md §3.3 ve web yumuşak vurgu kullanır;
  - kısayollar eş aralıklı yazıyla yazılıyordu; web'de arayüz yazısıyla ve sönük;
  - başlıklar etiket sütununa hizalı değildi.
- **Bölümlü seçici:** seçili dilim dolu vurgudaydı. DESIGN.md ve web'de dilim çukur bir izin içinde "kabarıktır".
- **Uzun metin:** uzun bir katman adı katman ağacında ve Öznitelikler'de "…" olmadan kırpılıyordu. Açılır kutuda ve menüde iki satıra sarılıp sabit yükseklikteki satırdan taşıyordu (ör. "ve onaylı)" menünün altında kesikti).
- **Odak:** alanın kenarı odakta tam vurgu, üzerine gelince sönük gri oluyordu. Başka bir işaret yoktu.

## Karar

### Biçim dizgesi (KentOS UI `theme::shape`)

- **Kademeler:**

  | Kademe | Nerede |
  |---|---|
  | `xs` | etiketler, renk örnekleri |
  | `sm` | düğmeler, alanlar, menü satırları |
  | `md` | menüler, açılır paneller, ipuçları, kartlar |
  | `lg` | pencereler |

  Yumuşak köşelerde değerler web'inkidir: 2, 4, 6 ve 10 piksel.
- **Köşeler** (`appearance.corners`) bütün kademeleri birlikte değiştirir:
  - **Keskin:** dörtte bir, en çok 2 piksel; klasik CAD programları gibi.
  - **Yumuşak** (varsayılan): web'in değerleri.
  - **Yuvarlak:** 1,6 katı.

  Yazı ayarı gibi bütün arayüzündür ve yalnız değişince kurulur. KentOS UI'daki ve masaüstündeki bütün sabit yarıçaplar bu ayarı izler (`shape::radius`). Haplar, daireler ve bilerek köşeli çizilenler sabit kalır.
- **Gölgeler** (`appearance.shadows`): üç kademe vardır: `Float` (ipuçları, yüzen araç çubukları), `Pop` (menüler, açılır listeler), `Window` (pencereler).
  - **Kapalı:** gölge yok; katmanlar kenarlarıyla ayrılır.
  - **Hafif** (varsayılan).
  - **Belirgin.**

  Aydınlık temada gölge koyudakinin üçte biri kadardır. Yerleşik paneller her zaman düzdür (DESIGN.md).
- İki ayar da iki platformun ortak anahtarıdır (ADR 0126). Web onları `data-corners` ve `data-shadows` ile `--r-*` ve `--shadow-*` belirteçlerine uygular. İki platformda Uygulama ayarları → Görünüm → Biçim'dedir.

### Bileşenler

- **Pencereler** (`style::container::dialog`): web'in `.dialog`'u gibi panel zemini, belirgin kenar, `lg` köşe ve pencere gölgesi.
- **Menüler** (bağlam menüsü, açılır listenin menüsü):
  - kutu: `md` köşe, `Pop` gölge, belirgin kenar (web'in `--c-line-strong`'u, `Tokens::border_strong`);
  - satırlar: yuvarlak, üzerine gelinen satır yumuşak vurgu zemininde ve yazı kendi renginde; tehlikeli komut kırmızı, zemini yumuşak kırmızı;
  - kısayollar ve ipuçları arayüz yazısıyla ve sönük; ayırıcılar kenardan içeride;
  - başlıklar yarı kalın, sönük ve etiket sütununa hizalı.

  Menü kutusunu pencereye kırpan katmandan önce kutunun gölgesi ayrıca çizilir; gölge artık kesilmez. Katmanın içindeki kutu gölgesiz çizilir (`style::container::popover_flat`). İçte ikinci bir gölge kutunun sınırında kesilir, yalnız yuvarlak köşelerin dışında kalır ve köşelerde koyu kareler bırakır (sahibin bildirimi, 28 Eylül: "Radius olunca köşelerde siyahlık kalıyor").
- **Açılır katmanlar:** iced bir açılır katmanı (overlay) yerleşiminin sınırına kırpar. Kutusu kadar yerleşimi olan katmanda gölge aynı biçimde köşelere iner. Bu yüzden ipucu (`tip`) ve harf ipucu rozeti (`key_tip`) de açılır liste ve komut satırı gibi pencere boyunca yerleşir; kutu onun içinde durur.
- **Liste satırları** (`list_item`): seçili ya da klavyeyle gelinen satır yumuşak vurgudur, kenarsız; üzerine gelinen hafif bir katman alır. Açılır kutunun listesi `menu_row` kullanır: üzerine gelince yumuşak vurgu; geçerli değer işaretiyle belli olur.
- **Bölümlü seçici:** çukur iz (alan zemini, ince kenar, `md` köşe, 2 piksel boşluk). Seçili dilim başlık zemininde, belirgin kenarlı ve hafif gölgeli; öbürleri sönük yazılır.
- **Alanlar:** üzerine gelince kenar belirginleşir (`border_strong`). Odakta vurgu çizgisini alır (`Tokens::accent_line`, web'in `--c-accent-line`'ı).
- **Odak halkası** (`widget::focus_ring`): içindeki metin girişi odaktayken kutunun çevresine 3 piksellik yumuşak bir vurgu halesi çizer; köşeleri izler. Iced'in metin girişinin durumundan okunur. Şunlarda vardır:
  - arama kutuları ve sayı alanları (KentOS UI `SearchBox`, `NumberInput`);
  - İşlem penceresinin alanları;
  - masaüstünün kenarlı alanları: yeni proje, proje ayarları, koordinat sistemi araması, kısayol araması, bulut girişi ve formları, hesap pencereleri.
- **Uzun metin:** tek satırlık yerlerde "…" ile kısalır (`Elided`, web'in `text-overflow: ellipsis`'i). Satırlar yüksekliğini korur, yazı başka bir öğenin üstüne binmez:
  - açılır kutunun kapalı hâli ve listesi;
  - bağlam menüsünün satırları;
  - katman ağacının adları;
  - özellik ızgarasının değer ve seçim hücreleri.

  `Elided` bütün metnini bildirir: testler ve ekran okuyucular kısaltılmışını değil kendisini bulur.

## Sonuçlar

- Masaüstü web'in biçim dilini konuşur: kademeli köşeler, yüzen katmanlarda gölge, yumuşak vurgu, kabarık seçim, hale ile odak.
- Kullanıcı köşeleri ve gölgeleri iki platformda birlikte ayarlar. Ayar dosyası bunları da taşır.
- Bütün ekranlar değişti: köşeler 2 pikselden kademelere geçti.
- Kalan farklar ayrı işlerdir:
  - masaüstünün tema renkleri web'in paletinden ayrıdır (ör. koyu temanın zemini nötr grafit, web'inki mavi-grafit);
  - Uygulama ayarları → Görünüm'de web'in tema kartları, vurgu ve yazı tipi örnekleri yoktur (TODOS.md UX-13); ADR 0128 getirdi.

## Doğrulama

- `cargo test -p kentos-ui`: biçimin kademeleri ve köşe seçenekleri, gölgelerin kademesi ve kapanması, anahtarların geri okunması (`theme::shape`).
- `cargo test -p kentos-contracts --test settings`: iki yeni ayar şemada (seçenekler, varsayılanlar, iki platform).
- `cargo test -p kentos-desktop`, `-p kentos-ui-showcase`: bütün masaüstü ve vitrin testleri.
- `pnpm -C apps/web exec vitest run src/core/settings src/app/settings`: web'in tercihleri iki yeni ayarla.
- Köşeler GPU çizicisiyle (`kentos-cad snapshot … --ayar appearance.corners=round --tikla 310,106` ve `--tikla 995,57`): Daire menüsünün ve katman listesinin köşe pikselleri çevreleriyle aynıdır; düzeltmeden önce dışarıdakinden koyuydu (ör. (13,16,20) ile (16,20,26)).
- `cargo test -p kentos-desktop ui_screens -- --ignored --nocapture`: `.run/shots/arayuz-*`:
  - sağ tık menüsü üç köşe ve gölge seçeneğinde;
  - şeritte ve ağaçta uzun katman adı, açık listesiyle;
  - odaklanan arama kutusu;
  - Görünüm → Biçim;
  - koyu ve açık temada.
