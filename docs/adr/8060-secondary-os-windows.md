# ADR 8060: Ayrı işletim sistemi pencereleri — İşlemler aracı ikinci ekranda

- **Durum:** uygulandı (15 Kasım 2026). İşlemler araç penceresi (araç formu ve Toplu çalıştır tablosu) ayrı pencereye
  alınabilir; 16 Kasım'dan beri 3B görünüm de, kendiliğinden ayrı açılarak (§9). Düzenek öbür pencereler için hazır (§2). Gerçek pencerede, yan yana iki ekranın yerini tutan geniş
  bir X ekranında sınandı (§7).
- **Sahibin isteği (15 Kasım, özet):** pencerelerimiz ana pencerenin içinde, tuvalin üstünde çiziliyor ve bu iyi; ama bazı
  pencerelerde kullanıcı çalışırken ana ekranın değiştiğini görmeli, örneğin İşlemler'in araç pencerelerinde. Böyle bir
  pencereyi ikinci ekrana alıp aracı çalıştırmak ve sonucu haritada hemen görmek istiyor; bugün pencere haritayı örtüyor.
  Şimdi işlem araçlarına, sonra başka yerlere. Ana pencereyle konuşan gerçek bir işletim sistemi penceresi olmalı.
- **Bağlam:** ADR 0017 (masaüstü kabuğu), 0746 (pencere boyları, `pencereler.json`), 0802 (her programın kendi uygulama
  kimliği), 8000 (gerçek pencerede kabul), DESIGN.md §7.9 ve §7.11.

## §1. Tek daemon; pencereleri uygulama açar

- `iced::application` tek pencerelidir: görünümü `Fn(&State)` her pencereye aynı ağacı verirdi. iced 0.14'ün
  `iced::daemon`'ı görünümü, başlığı ve temayı pencerenin kimliğiyle sorar (`daemon.rs`: kendiliğinden pencere açmaz, son
  penceresi kapanınca da bitmez; `exit` görevi gerekir). Program artık bir daemon: `App::window_view(id)`,
  `window_title(id)`, `window_theme(id)`.
- **Ana pencere** açılışta `window::open(main_window(program))` ile, eskisiyle aynı ayarlarla açılır: simge,
  1440 × 900, en küçük 1100 × 650, `exit_on_close_request: false`, Linux'ta programın uygulama kimliği. Kimliği
  `Windows::set_main` ile tutulur. Uygulamanın ilk görevleri (sunucu denetimi, Python araması, cihaz ölçeği) pencere
  açıldıktan sonra çalışır (`opened.discard().chain(first)`), `application`'ın yaptığı gibi.
- **`App::view()` ve `App::title()` ana pencerenindir ve değişmedi.** 310 dosyada 1273 `App::view` çağrısı (resim
  testleri, izler, `snapshot`, `kullan`) aynı anlamla çalışıyor; daemon yalnız sarmalayıcıları çağırır. Görevin
  "`App::view()`'ı çağıran testleri uyarla" maddesi bu yüzden boş kaldı: uyarlanacak test yok.
- Tek pencereye göre yazılmış iki yer düzeldi: cihaz ölçeği `window::oldest()` ile değil ana pencerenin kimliğiyle
  sorulur; tam ekran (`view.fullscreen`) `window::latest()` yerine ana pencereyi alır, yoksa ayrı pencere ondan sonra
  açıldığı için onu tam ekran yapardı.

## §2. Kayıt ve yüzeyler (`apps/desktop/src/shell/windows/`)

- `Surface`: ayrı pencerenin gösterdiği şey. Bugün tek değişken var, `ProcessingTool`. `Windows`: ana pencerenin
  kimliği, açık ikincil pencereler (kimlik, yüzey, platformun yapıp yapmadığı), bir sonraki odak isteği, işi ana pencereye
  geçmiş yüzeyler ve hatırlananlar (§4).
- **Pencere durumu izler** (`App::follow_windows`, her mesajdan sonra): ayrı olan ve gösterecek bir şeyi olan yüzeyin
  penceresi açılır, gösterecek şeyi kalmayanınki kapanır. Aracı kapatan her yol (Kapat, Esc, Model tasarımcısı,
  Sunucu işleri, seçim tarifi) penceresini de kapatır; o yolları tek tek değiştirmek gerekmedi.
- Platformun henüz yapmadığı pencere kapatılmaz: kapanış `window::open`'un yanıtını (`Event::Opened`) bekler. Yoksa
  pencere kapanışından sonra yapılır ve içi boş, açık kalırdı.
- Bilinmeyen bir kimliğin (kapanmakta olan pencerenin) görünümü boştur: ikinci pencerede ana pencerenin görünümü hiç
  çizilmez.
- **Tek durum:** bütün pencerelerin mesajları tek `App::update`'ten geçer; pencere başına kopya yok. Araç ayrı pencerede
  çalışırken çizimi aynı belge işlemiyle yazar, geri alma adımı aynıdır.
- Yeni bir pencereyi ayırmak: `Surface`'e bir değişken, `shell/windows/app.rs`'teki `match`'lere görünüm, başlık,
  kapanış, tuş ve "işi ana pencerede mi" kolları; durumu kendi modülünde kalır. İşlemler'inki `gis/processing/apart.rs`.
- **Kitte iki ek** (`crates/ui/src/widget/dialog.rs`): `Dialog::header_button` başlığa, tam ekran düğmesinin soluna bir
  pencere düğmesi koyar; `Dialog::window` aynı kutuyu pencerenin tamamı olarak çizer: arkasında karartma, boyutlandırma
  çerçevesi ve tam ekran düğmesi yok, kutunun zemini (`style::container::dialog_window`) pencereyi doldurur, alt çubuk
  pencerenin alt kenarındadır.

## §3. Girdi: hangi pencereden geldi

iced her aboneliğe her pencerenin olaylarını, pencerenin kimliğiyle verir; kabuğun dinleyicileri tek pencere için
yazılmıştı ve ayrı pencerenin tuşlarını ana pencerenin tuşu sanırdı. `shell/windows/routing.rs`:

- `keys(main, dinleyici)`: ana pencerenin olayları dinleyiciye değişmeden gider (`keys::key_event`,
  `drawing_menu_key_event`). İkincil pencerede hiçbir öğenin almadığı tuş `Event::Key` olur ve o pencerenin yüzeyine
  gider; hiçbir zaman `Message::Key`, yani kısayol ya da çizim komutu olmaz. Bir alanın aldığı tuş alanındır: yazı o
  pencerenin alanlarına gider. Değiştirici tuşlar ve odak değişiklikleri ana pencereye ulaşmaz: orada basılı Shift
  ana pencerede ortoyu açmaz.
- `window_events(main)`: ana pencerenin kapatma isteği, boyutu ve ölçek değişikliği tek pencereliyken verdikleri
  mesajları verir (`CloseRequested`, `WindowResized`, `DeviceScale`); bunları gönderen testler aynı anlamla çalışır. Ana
  pencerenin taşınması sorulmaz, her biri bir güncelleme olurdu. İkincil pencerenin kapatma isteği, taşınması, boyutu ve
  kapanması kendi olayıdır; ekran ölçeği markanın resimlerini değiştirmez (onlar ana penceredeki).
- `main_only`: dosya bırakmak (Kaynaklar, SVG düzenleyici) yalnız ana pencerede.
- Ayrı araç penceresinin tuşları çizimin üstündeki pencereninkilerdir: Esc Kapat, Ctrl+Enter Çalıştır, Tab alanlar
  arasında dolaşır; başka hiçbir şey. Bir çizim açılırken ya da büyük içe aktarma sürerken ikincil pencerenin tuşları da,
  ana pencereninkiler gibi bekler. Pencerenin açılması, kapanması, taşınması ve boyutu her durumda alınır; yoksa
  pencereler ve kayıt birbirinden ayrılırdı.
- **Odak:** Sahneden seç başlayınca klavye ana pencereye geçer (nokta orada gösterilir ya da Y,X yazılır), bitince ayrı
  pencereye döner (`Windows::set_away` + `window::gain_focus`). İfade oluşturucu (980 × 700, akışta 1240 × 800) araç
  penceresinden büyüktür; ana pencerede açılır ve klavyeyi o alır.
- **Tema, yazı tipleri, ölçek:** tema her pencerede aynıdır (`window_theme`); yazı tipleri ve yazı boyutu programındır,
  iki pencerede aynı. Her pencere kendi ekranının piksel yoğunluğunu izler (winit). Ana pencerenin cihaz ölçeği eskisi gibi
  ana pencereden okunur.

## §4. Hatırlananlar

- `ayri-pencereler.json` (ayarların yanında, `pencereler.json` gibi): yüzey başına "kendi penceresinde açılır" ve
  pencerenin boyu. **Seçim kalıcıdır:** sonraki araç da, programın sonraki açılışında da aynı biçimde açılır (görevde
  önerilen buydu).
- **Yer yalnız bu çalıştırmada:** pencere kapanırken yeri platforma sorulur (`window::position`); taşınma haberi gelirse
  o da tutulur. Diske yazılmaz. Microsoft'un çok ekranlı yerleşim belgesi kayıtlı bir yeri kullanmadan önce geçerli mi
  diye bakılmasını ister, çünkü ekran taşınmış ya da çıkarılmış olabilir. `SetWindowPlacement` ekran dışı yeri
  kendiliğinden düzeltir; winit ise pencereyi `SetWindowPos` ile yerleştirir ve düzeltmez. iced 0.14 ekranları
  listelemediği için bu denetim yapılamıyor: kalıcı yer, çıkarılmış ekranın üstünde, kimsenin göremeyeceği bir pencere
  açabilirdi.
- **İlk açılış:** `Position::Centered`. iced ortalamayı son pencerenin ekranına göre yapar
  (`window_manager.last_monitor()`), yani ana pencerenin ekranının ortasına.
- X11'de pencere yöneticisi yokken taşınma haberi hiç gelmez: winit 0.30 `Moved`'u yalnız pencere yöneticisinin yapay
  `ConfigureNotify`'ından üretir (ICCCM §4.1.5). Yerin kapanırken sorulmasının nedeni budur. Gerçek pencere düzeneğinde
  (§7) sağ ekrana konup ana pencereye geri alınan araç, yeniden ayrılınca yalnız taşınma haberiyle ortada (720, 0)
  açılıyordu; yer kapanırken sorulunca bıraktığı yerde, sağ ekranda (1440, 0) açıldı.

## §5. Kapanış sırası

- **Ana pencerenin kapatma düğmesi** bugünkü akıştan geçer: kayıt sürerken beklenir, sekmelerdeki kaydedilmemiş iş
  söylenir, kaydedilmemiş değişiklik sorulur, bulutta gönderilmemiş iş taslağa yazılır. Onaylanınca bulut kopyası ve
  kurtarma kopyası kapanır, sonra ikincil pencereler, sonra ana pencere kapanır ve program `iced::exit()` ile biter
  (`App::close_every_window`). Soru açıkken araç penceresi açık kalır; Vazgeç hiçbir şeyi kapatmaz.
- **İkincil pencerenin kapatma düğmesi** Kapat'tır (`exit_on_close_request: false`, kapanışı uygulama yapar). Her durumda
  çalışır; Sahneden seç sürüyorsa önce ondan vazgeçilir.
- Ana pencere kapanırken kayıt kapanır (`Windows::close_all`): ondan sonra hiçbir pencere açılmaz.

## §6. İşlemler araç penceresi ayrıyken

- Çizimin üstündeki pencerenin başlığında **“Ayrı pencerede aç”** (web simgesi `externalOpen`). Ayrı pencerede aynı içerik
  ve başlıkta **“Ana pencereye al”** (yeni simge `windowReturn`, `catalog/v1/additions.json`). Pencerenin adı
  “Tampon — KentOS GIS”: görev çubuğu ve pencere değiştirici ikisini bununla ayırır.
- Ayrıyken çizimin üstünde pencere yoktur: ana pencere fareyi ve klavyeyi her zamanki gibi alır (kaydırma, yakınlaşma,
  seçim, paneller). Araç ayrı pencereden çalışır; ilerleme, Durdur ve sonuç çizimin üstündeki gibidir, sonuç haritada
  hemen görünür.
- Ayrı pencerede **Sonuçları seç** ve **Grafik** pencereyi kapatmaz; çizimin üstündeyken kapatıyorlardı, çünkü pencere
  haritayı örtüyordu. Başka bir pencere açan yollar (Model tasarımcısı, Sunucu işleri, seçim tarifi) aracı eskisi gibi
  kapatır.
- Ana pencerede başka bir pencere açıkken “Ana pencereye al” yapılmaz ve nedeni söylenir: o pencere kapanmadan araç
  çizimin üstüne gelemez.
- Sahneden seç sürerken ayrı pencere ana pencerede ne istendiğini söyler (Vazgeç; nesne seçiminde Bitti de), seçim bitince
  form aynı değerlerle geri gelir.

## §7. Ölçümler (15 Kasım 2026)

**Birim testleri:** 9/9. `windows::tests` (4): kayıt; pencere olaylarının yönü; seçim ve boy kalıcı, yer yalnız bu
çalıştırmada; daemon yokken her pencere ana penceredir. `processing::apart_tests` (5): araç ayrılıp geri alınır, ayrı
pencereden çalışır ve Sonuçları seç onu kapatmaz; pencerenin kapatma düğmesi Kapat'tır ve sonraki araç ayrı açılır;
ayrı penceredeki L ve Delete ana pencereye ulaşmaz, aynı L ana pencerede Çizgi'yi başlatır, Ctrl+Enter çalıştırır, Esc
kapatır; Sahneden seç klavyeyi ana pencereye verip geri alır; ana pencere kapanınca öbürleri kapanır ve program biter,
kaydedilmemiş değişiklik önce sorulur. İlgili gruplar
(`processing::`, `ribbon_search`, `unsaved_close`, `saving::`, `cloud::tests::offline`, `cloud::kinds_tests`,
`app::tests`, `keys::`, `dialog_size`, `icons::tests`, `catalog::tests`): 183 geçti, 26 yok sayılan (resim testleri).
Kitin `dialog`, `sizable`, `overlay` testleri (`--features snapshot`): 4/4. Clippy (`--no-deps`, iki paket, bütün hedefler)
dokunulan dosyalarda yeni uyarı vermedi; kit Windows hedefine (`x86_64-pc-windows-msvc`) derleniyor, masaüstü paketi bu
makinede o hedefe denetlenemedi (`ring` ve `aws-lc-sys` MSVC C derleyicisi istiyor).

**Gerçek pencere** (`fixtures/interaction/real/ayri-pencere-islem.txt`): Xvfb :95, 2880 × 900 (iki 1440 × 900 ekran),
yazılım Vulkan'ı (lavapipe), hata ayıklama derlemesi, sahibin ayarlarının kopyası yalıtılmış bir ev dizininde. KentOS GIS,
yapay `samples/eklenti-vitrini/vitrin.kcad`, İşlemler › Tampon, Yapı katmanı (5 kapalı alan), 5 m.

| Adım | İşlemci | Duvar saati |
|---|---|---|
| Ayrı pencerede aç (pencere açılıp ilk karesi çizilene dek) | 860 ms | 636 ms |
| Ayrı pencereden Çalıştır (5 tampon yazılır, iki pencere çizilir) | 1270 ms | 891 ms |
| Ana pencerede 2 tekerlek adımı uzaklaşma, araç ayrı pencerede açık | 2960 ms | 1109 ms |
| Aynı uzaklaşma, araç kapalı (tek pencere) | 2560 ms | 1110 ms |

iced her mesajdan sonra bütün pencereleri yeniden kurup çizer; araç penceresi açıkken ana penceredeki etkileşim yazılımla
çizimde ~%16 fazla işlemci harcadı, duvar saati değişmedi. Ekran kartıyla ölçülmedi.

Resimler (`.run/` dışında, oturumun çıktısında): 01 çizimin üstündeki pencerede düğme ve ipucu, 02 ayrıldı, 03 sağ ekranda,
04 sonuç haritada, 05 ana pencere kullanılıyor, 06 ayrı pencerede yazılan 8 m ve Ctrl+Enter (L Çizgi'yi başlatmadı),
07 Sonuçları seç (pencere açık kaldı), 08 “Ana pencereye al” ipucu, 09 geri alındı, 10 kapatıldı. Keşif oturumunda
yazılan `ayri-pencereler.json` oturumun ev dizinindeydi; sahibin `~/.config/kentos-cad`'inde böyle bir dosya yok.

**Düzenekte görülen iki aksama:** keşif sırasında bir koşuda ayırdıktan sonra 5,5 sn boyunca iki pencere de yeni kare
çizmedi; başka bir koşuda `fit`'ten hemen sonraki Çalıştır tıkı kayboldu. İkisi de makinede paralel derlemeler sürerken,
yazılımla çizimde oldu; aynı adımlar yeniden koşunca geçti. Senaryo artık tıklamadan önce imleci düğmenin üstüne getirip
bekliyor; neden kesin bulunmadı (§8).

## §8. Kalanlar

- Öbür pencereler (Öznitelik tablosu, Katman stili, Model tasarımcısı, İfade oluşturucu…): her biri bir `Surface`
  değişkeni ve kendi `apart.rs`'i.
- İfade oluşturucu ayrı pencerenin içinde değil, ana pencerede açılıyor (araç penceresinden büyük); kendi yüzeyi olabilir.
- Sahiplenilmiş pencere yok: iced 0.14 ana pencerenin üstünde kalıp başka programların üstüne çıkmayan bir pencereyi her
  platformda vermiyor. Ayrı pencere sıradan penceredir; aynı ekranda ana pencereye tıklanınca arkasında kalabilir.
  Windows'ta sahiplenilmiş pencere sahibinin ekranında açılır (Microsoft); bu da ileride iced'e bağlı.
- Kalıcı yer: iced ekranları listelediğinde, görünürlük denetimiyle birlikte.
- Wayland'de pencere yeri bilinmez ve verilemez; ilk açılış ve yeniden açılış bileşiğin seçtiği yerdedir.
- Odak isteği (`gain_focus`) en iyi çabadır: pencere yöneticisi reddedebilir; X11'de pencere yöneticisi yokken hiç
  çalışmaz (düzenek odağı `fit` ile verir).
- iced'in widget işlemleri (odak, Tab) bütün pencerelere uygulanır: ana pencerede başka bir pencere açıkken ayrı
  penceredeki Tab yanlış kapsamı seçebilir. Görülmedi, sınanmadı.
- `window::frames()` her pencerenin karesinde gelir; iki pencereyle kare başına iş yapan abonelikler (içe aktarma dilimleri,
  istatistik sayımı) iki kat sık çalışır. Ölçülmedi.
- §7'deki iki düzenek aksaması; gerçek çok ekranlı Windows ve KDE oturumunda deneme yapılmadı.

## §9. 3B görünüm: ikinci yüzey (16 Kasım 2026)

- **Sahibin isteği (16 Kasım, özet):** mimari projeyi 3B gösteren pencereyi düzenleyelim, özellikle araç çubuğunu ve alt
  tarafı; bileşenlerde ayrı pencere desteğimiz var, İşlemler'in araç pencerelerinde kullanıyoruz. Bu pencere işletim
  sisteminin penceresi olsa daha iyi olur, çünkü kişi onu başka bir ekrana sürükleyip projeyi incelemek isteyebilir.
- **Yüzey:** `Surface::Scene3d` (anahtar `3b-gorunum`; ilk boy 1180 × 760, en küçük 720 × 460, 12 px yazıda). Görünüm,
  başlık, kapanış ve tuşları `app.rs`'teki eşleşmelerdedir; kendi kodu `scene3d_view.rs`. Pencerenin adı
  “3B görünüm — KentOS CAD”.
- **Kendiliğinden ayrı açılır** (`Surface::opens_apart`): araç çizimin üstünde çalıştırılır, model planın yanında izlenir.
  Kullanıcının seçimi yine kalıcıdır. `Kept.own_window` `Option<bool>` oldu: seçim yapılmamışken dosyaya `ownWindow`
  yazılmaz ve yüzeyin kendi davranışı geçerlidir. Böylece seçimden önce tutulan boy (`keep_size` kaydı açar) pencereyi ana
  pencereye döndürmez. Eski dosyalar aynı okunur (`ownWindow` olan kayıt seçim sayılır).
- **Ayrıyken pencere kutusu yoktur:** başlık işletim sisteminin çubuğundadır. Pencere araç şeridi, sahne ve durum
  çubuğundan oluşur; “Ana pencereye al” (`windowReturn`) şeridin sağ ucundadır. Çizimin üstündeyken “Ayrı pencerede aç”
  başlıktadır (§6'daki gibi). Ana pencerede başka bir pencere açıkken geri alınmaz, nedeni söylenir. Esc kapatır, Tab
  alanlar arasında döner. Kapatma düğmesi Kapat'tır.
- **Çizimi izler:** ayrıyken ana pencere serbesttir; kullanıcı çizerken, geri alırken ya da seçerken 3B eskimemeli.
  `App::follow_scene3d` her mesajdan sonra, `follow_windows`'tan önce çalışır. Çizimin üretimi ya da revizyonu değişince
  sahne yeniden kurulur: bina IFC gövdelerinden, yol arazi ve koridorlardan. Kamera ve gizli katlar yerinde kalır. Planın
  seçimi (seçimin sürümü) ya da Denetim'de seçilen bulgular değişince sahne yeniden boyanır. Bulgular tek sayılık bir
  anahtardır: denetimin çizim durumu ve bulgu sayısı, seçilen satırlar, grup ve “çizim değişti”. Fare her kıpırdadığında
  liste kurulmaz. Çizim kapanınca pencere kapanır. Gösterilecek bir şey kalmazsa pencere kapanır ve nedenini söyler.
- **İz oynatıcı:** ana pencerede pencere yokken ayrı 3B görünüm yanıt verir (`dialog_title`, `dialog_control`).
- **Sınamalar:** `windows::tests::the_3d_view_opens_apart_until_the_user_chooses_otherwise` (seçimsiz boy, dosyada
  `ownWindow` yok, seçim kalıcı); `scene3d_window_tests::the_window_opens_beside_the_plan_and_comes_over_the_drawing_and_back`
  (ayrı açılış, ad, iz, Ana pencereye al, ana pencerede Esc, Ayrı pencerede aç, ayrı pencerede Esc ve kapatma düğmesi,
  çizim kapanınca kapanış); `scene3d_building_tests` (planın seçimi açık pencerede boyanır). Resimler:
  `.run/gui-evidence/3b-pencere/`, `.run/gui-evidence/bina-3b-8067/`.
- **Kalanlar:** büyük binada her düzenlemede yeniden kurulum ölçülmedi (ADR 8067 §8). Gerçek pencerede, fareyle iki ekranda
  kabul yapılmadı. Yolda arazinin ve koridorların her revizyonda yeniden kurulması da ölçülmedi.

## Uygularken verilen kararlar (sahibin söylemediği)

1. `App::view()`/`title()` yerinde bırakıldı, daemon yeni sarmalayıcıları çağırır (§1).
2. Seçim (ayrı açılır mı) ve boy kalıcı, yer yalnız bu çalıştırmada (§4).
3. Ayrı pencerede Sonuçları seç ve Grafik pencereyi kapatmaz (§6).
4. Ayrı pencerede Esc aracı kapatır, çizimin üstündeki gibi (DESIGN.md §7.9); yazı alanındayken Esc alanındır.
5. İkincil pencerenin kapatma düğmesi açılış ve içe aktarma sürerken de çalışır; tuşları ise bekler.
6. Ana pencerede başka bir pencere açıkken “Ana pencereye al” reddedilir, nedeni söylenir.
7. Sahneden seç sürerken ayrı pencere kapanmaz, ne istendiğini söyler; klavye ana pencereye geçip geri gelir.
8. İfade oluşturucu ana pencerede açılır, klavyeyi o alır.
9. İkincil pencere ana pencerenin uygulama kimliğini taşır (Linux), masaüstü ikisini tek program sayar; görev çubuğunda
   gizlenmez.
10. Tam ekran komutu ana pencereyi alır, son açılan pencereyi değil.

## Kaynaklar

- iced 0.14.0 `src/daemon.rs` (daemon pencere açmaz, son penceresiyle bitmez; görünüm, başlık, tema pencere kimliğiyle);
  iced_winit 0.14.1 `src/lib.rs` (`Position::Centered` son pencerenin ekranına göre; her mesajdan sonra bütün pencereler
  yeniden kurulup çizilir; widget işlemleri bütün pencerelerde).
- winit 0.30.13 `src/platform_impl/linux/x11/event_processor.rs` `configure_notify` (`Moved` yalnız yapay
  `ConfigureNotify`'da); `src/platform_impl/windows/window.rs` (ilk yer `SetWindowPos` ile).
- ICCCM §4.1.5, Configuring the Window (yapay `ConfigureNotify` kök koordinatlarıyla).
- Microsoft Learn, “Positioning Objects on Multiple Display Monitors” (kayıtlı yeri kullanmadan önce geçerli mi diye bakın;
  sahiplenilmiş pencere sahibinin ekranında) ve `SetWindowPlacement` açıklamaları (ekran dışı yer düzeltilir).
