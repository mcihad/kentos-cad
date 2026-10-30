# ADR 0803: Launcher'ın görünüşü: masa, pafta ve levhalar

- **Durum:** kabul edildi (2026-10-30), dal `launcher-zarif`.
- **İstek:** sahibin sözleri “launcher çok iyi ama daha estetik ve zarif hale getirmeni istiyorum”, “bu uygulama ileride tüm
  uygulamalarımızın paneli olacak”, “genel tasarıma uygun ama daha şık ve kaliteli olmalıdır”, “ayrı bir branch ile yap ekran
  görüntülerini gönder”.
- **Bağlam belgesi:** ADR 0800 (launcher), 0802 (modüller), DESIGN.md §1, §11, §12, §13.
- **Web:** bir şey değişmez (ADR 0228). **`.kcad`, komutlar, manifest biçimi, Python, MCP:** değişmez.
- **Bağımlılık:** yeni dış kitaplık yok.

## Bağlam

ADR 0800'ün penceresi sahibin taslağının düzenidir ve işini görür. Üç şey onu sıradan bırakıyordu:

- **Her şey kutuda.** Özet şeridi, altı kart, kartın içindeki çerçeveli resim, kartın dibinde çerçeveli düğme. DESIGN.md §13 “genel SaaS kartı
  görünümü” ve “aynı köşe yarıçapı ve gölgenin her yerde kullanılması”nı yapılmayacaklar arasında sayar.
- **Hiyerarşi yok.** Çalışan üç program ile henüz çıkmamış üç program aynı boyda, aynı canlılıkta; altı kez tam genişlikte düğme, üçü
  “Yakında”.
- **Panel değil.** Launcher tüm uygulamaların paneli olacak; panelin işi işe dönmeyi kısaltmaktır, oysa son çizimler ayrı sayfadaydı,
  Güncelleme merkezi kaydırmanın altındaydı.

Başvurulan yol (30 Ekim 2026'da okundu):

- Microsoft, *Layering and elevation in Windows* (güncelleme 19 Şubat 2026): “Windows 11 uses a two-layer system for applications.” Taban
  katmanı “is the bottommost layer of every app, and contains controls related to app menus, commands, and navigation”; içerik katmanı
  “focuses the user on the app's central experience” ve “may be on contiguous element, or separated into cards”.
  <https://learn.microsoft.com/en-us/windows/apps/design/signature-experiences/layering>

KentOS'un çıkardığı: iki katman doğrudur; içerik katmanı kartlara bölünmek zorunda değildir. KentOS'un kendi sözlüğünde bu ikisi masa ve
paftadır: DESIGN.md'nin açık teması zaten “Pafta”dır.

Yazılım tarafı (iced 0.14'ün kaynağı): `svg::Style::color` rengin alfasını yok sayar, yalnız RGB'yi basar (`iced_tiny_skia/src/vector.rs`,
`iced_wgpu/src/image/vector.rs`); saydamlık `Svg::opacity` iledir. Yazılım çizicisi (tiny-skia) `ContentFit::Cover`'ın taşan kısmını
kesmez; GPU çizicisi keser.

## Karar

### 1. Masa ve pafta

- Pencerenin zemini masadır: koyu temada pencere renginin %18 koyusu, açık temada pencere grisi. Kenar çubuğu ve durum satırı masada, zeminsiz durur.
- Sayfa masanın üstünde yatan paftadır: üstten ve sağdan 8 piksel açıkta, köşesi `shape::lg` + 2, kenarı çok ince, gölgesiz. Koyu temada
  pencere ile yüzey arasında bir ton, açık temada kâğıt beyazına yakın (`popover`). Renkler jetonlardan türetilir (`style::table_color`,
  `sheet_color`), yeni sabit renk yok.

### 2. Levhalar: kutu yok

- Bir uygulama, atlas levhası gibi durur: resim ve altında yazı (`view/plates.rs`). Zemin, kenar ve gölge yok. Üzerine gelince 8 piksel taşan silik
  bir zemin ikisini toplar; yarıçapı resminkine iç boşluk eklenmişidir, iki eğri yan yana akar. Seçili levhanın zemininde vurgu çizgisi vardır.
- Karo 48'den 40 piksele iner, kendi halkası kalkar (2× ölçekte koyu bir çizgi gibi duruyordu).
- Alt satır: lamba ve sözü (“Kurulu”, “Çalışıyor”, “Bulunamadı”, “Web adresi”), sürüm, sağda düğme. Durum yalnız renkle söylenmez (§11).
- **Tek dolu düğme:** Başlat dinlenirken nötr katmandır, yalnız başlatılacak levhada dolu vurgu rengindedir (§13). ⋯ dinlenirken soluktur; ağaçtan
  çıkmaz, açık menü imleç levhadan ayrılınca kapanmaz.
- **Çift tık başlatır:** resim tıklanacak bir şey gibi durur; tek tık seçer, çift tık Başlat ile aynıdır. Gelecek uygulamada çift tık yoktur.

### 3. Resimler

Sahibin resimleri 16:9'dur; 2:1 kesim üst ve altlarını atıyordu. Artık özgün oranla, 800 × 450, libimagequant paletiyle (`pictures.py`, `save`):
altısı 1 733 712 bayt, sınır 2 MB. Aynı boyda özgünlere göre PSNR, ADR 0800'ün MEDIANCUT ve Floyd–Steinberg yolundan altı resimde de yüksektir (0,85–9,29 dB;
en çok `yol`: 33,09 → 42,38 dB). Özgünler depoya girmez (`.run/launcher-originals/`).

### 4. Sayfanın düzeni

- **Başlık:** 28 piksel normal ağırlıkta başlık (yarı kalın değil), altında tek cümle (`Counts::sentence`: “6 uygulama: 3 kurulu, 3 yakında”). Sağda arama, Uygulama
  ekle (dar yerde yalnız işareti) ve zil.
- **Özet şeridi kalktı.** Sözü başlığın altına, Uygulama ekle başlığa geçti. **Tümünü güncelle** Güncellemeler sayfasına taşındı: orada kapalı ve nedenini
  söyler (ADR 0800 §6 geçerli), ana sayfanın en değerli yerinde kapalı düğme durmaz. Durum satırındaki “Güncelleme denetimi yok” da kalktı; bunu Güncelleme
  merkezi söyler.
- **Bölümler:** çıkmış uygulamalar (büyük levha), Son dosyalar ile Güncelleme merkezi yan yana, Yakında (küçük, soluk resim; “Yakında” sözü başlıkta bir kez,
  ⋯ yalnız dosyadan gelen manifestte). İki raf `Launcher::shelves`'ten gelir; oklar onları satır satır gezer. Arama sürerken yalnız eşleşen levhalar kalır.
- **Son dosyalar ana sayfada:** en çok dört satır, “Tümünü gör” sayfasına götürür. Liste boşsa “Henüz açılan çizim yok. Bir uygulamayı başlatıp çizim açın.”
- **Sütunlar** (`cards::columns`): iki ile dört uygulama, levha başına en az 220 piksele sığıyorsa tek satırda (son satırda yalnız kalan levha yanlışlık gibi durur);
  yoksa en az 300 piksellik kaç levha sığarsa, 2–4 sütun. 1440 × 900'de ve 1100 × 650'de üç levha tek satırda; 18 piksel yazıyla 1100'de iki sütun.
- **En geniş** 1480 piksel, pafta içinde ortada: ötesinde resimler bulanır, göz fazla yol alır.
- **Hiza:** bütün levhalar tam piksel; bölünmeyen artık pikseller ilk aralıklara birer birer verilir (`Plan::gap_before`). Son levha sayfanın kenarında, zilin altında
  biter; Güncelleme merkezi son levhanın tam üstündedir.

### 5. Tablolar ve öbür sayfalar

- Son dosyalar ve sürümler tek satırlık sütunlardır, satırlar arasında ince çizgi vardır, çerçeve yoktur (`view/tables.rs`). Dar satır önce bilgiyi, sonra klasörü bırakır.
- “334 nesne · Yerel” programın yazdığı metindir; §13'ün “A · B · C” kuralı için tabloda virgülle okunur (“334 nesne, Yerel”).
- **Güncellemeler:** üstte durumun sözü ve kapalı Tümünü güncelle, altında uygulamalar ve sürümleri. **İndirmeler:** yalnız boş durum ve Güncellemeler'e yol
  (aynı liste iki sayfada duruyordu). **Ayarlar:** kutu kalktı, gruplar başlık ve ince çizgilerle, en çok 760 piksel.
- Kaydırma çubuğu nötrdür, rayı yoktur (`style::scroller`): vurgu rengi gezinmek için değildir.

### 6. Yazı rengi

Üçüncül ton (`faint`) paftada 3,6:1'dir (WCAG AA 4,5:1 ister; DESIGN.md §11). Okunması gereken ikincil bilgi için `style::soft`: `muted`'dan `faint`'e üçte bir yol, iki temada, paftada ve masada en az 4,5:1 (test).

### 7. Masadaki çizim

Kenar çubuğunun boş yerinde bir imar planı parçası: bükülmüş sokak ızgarası, adalar, parseller, bir geniş yol, bir göbek, birkaç ölçü noktası (`scripts/launcher/art.py`, standart
kitaplık ve sabit tohum, `assets/art/imar.svg`, 20 KB). Launcher'ın tek süsüdür; KentOS'un işini söyler, başka bir ürünle karışmaz. Rengi yazınınkidir, opaklığı koyuda 0,22, açıkta 0,26.
Üstten ve alttan masa rengine karışır; pencere boyu ne olursa olsun yazının altına girmez: nav ve Ayarlar'ın yeri masa rengiyle örtülür, yazılım çizicisinin kesmediği taşma da orada kalır.

### 8. Hareket yok

DESIGN.md §12: “her kartta ayrı hover geçişi yoktur”. Hover anlıktır; animasyon eklenmedi.

## Uygulama

- `apps/launcher/src/view/`: `style.rs` (masa, pafta, levha, nötr düğme, kaydırma çubuğu, `soft`), `mod.rs`, `sidebar.rs`, `header.rs`, `status.rs`, `apps.rs` (sayfa ve `Plan`), yeni `plates.rs`, yeni `app_menu.rs`,
  yeni `tables.rs`, `pages.rs`, `notices.rs`, `dialog.rs`, yeni `interaction_tests.rs`.
- `apps/launcher/src/`: `app.rs` (`shelves`, okların iki rafta satır satır gezmesi), `cards.rs` (`columns`, `Counts::sentence`), `pictures.rs`, `snapshot.rs` (`--ara`, `--ilk`, `--tik`).
- `apps/launcher/assets/pictures/*.png`, `assets/art/imar.svg`, `scripts/launcher/pictures.py`, `scripts/launcher/art.py`.
- DESIGN.md §7.18, `docs/launcher/README.md`, CLAUDE.md, ADR 0800'ün durum satırı.

## Sonuçlar

- Launcher'ın işlevi değişmez; manifestler, başlatma, bildirimler, ayarlar aynıdır. Eklenenler: ana sayfada son dosyalar, levhada çift tık.
- Yeni uygulama eklendikçe düzen büyür: yerleşik olmayanlar resimsizse karonun tonunda sade bir yer tutar, satırlar 2–4 sütuna akar.
- ADR 0800 §3'ün “özet şeridi”, “kart” ve Güncelleme merkezinin ızgaradaki yeri, §6'nın durum çubuğu sözü bu kararla değişti. §6'nın ilkesi (güncelleme yokken varmış gibi yapmamak) aynen sürer.

## Doğrulama

- **Testler:** `cargo test -p kentos-launcher`: 56 test geçti. Yeniler: sütun kuralı; sayfanın 1440'ta sığması, 1100'de kayması; artan piksellerin aralıklara dağılması; 16:9; masa, pafta ve
  yazı karşıtlıkları (iki tema, en az 4,5:1); `Counts::sentence`; `view/interaction_tests.rs` gerçek pencere ağacında fare ve yazı olaylarıyla (`Snapshot::input`): imlecin levhayı uyandırması ve bırakması,
  tek tıkın seçmesi ve çift tıkın başlatması, Başlat düğmesi, gelecek uygulamanın başlamaması, ⋯ menüsünün açık kalması ve öğesinin çalışması, “Tümünü gör” ve Ayarlar'a geçiş, son dosyanın açılması, arama,
  Uygulama ekle, zil; `/bin/sleep`'i Başlat ile gerçekten başlatıp “Çalışıyor” denmesi; resimsiz ve web adresli uygulamalar.
- **Denetimler:** `cargo clippy -p kentos-launcher --all-targets -- -D warnings` temiz; `rustfmt --edition 2024`; `node scripts/arch/deps.mjs`; `python3 scripts/launcher/art.py --check`.
- **Resimler** (`kentos-launcher snapshot`, `.run/gui-evidence/launcher-zarif/`, depoya girmez): koyu ve açık × 1440 × 900; 1100 × 650; 1920 × 1080; 11 ve 18 piksel yazı; 2× ölçek; üzerine gelinen, seçili, ⋯ menüsü, arama,
  ilk açılış (hiçbiri bulunamadı, son dosya yok), çalışan uygulama, resimsiz uygulamalar; Son dosyalar, Güncellemeler, İndirmeler, Ayarlar, Uygulama ekle, Bildirimler; GPU ve yazılım çizicisi. Resimlerde bulunup düzeltilenler:
  çizimin fazla baskın olması ve opaklığın işlememesi; çarpılı adaların yer tutucu resim gibi durması; “Tümünü gör”ün sağdan 6 piksel kaçması; 11 piksel yazıda üçüncü sütunun 2 piksel kayması; yazılım çizicisinde çizimin yazıların altına taşması;
  karonun 2× ölçekteki koyu halkası; “Kurulu uygulamalar” başlığının altında yakında çıkacakların durması.
- **Denenmeyenler:** gerçek pencerede el ile fare ve klavye (olaylar `Snapshot::input` ile pencere ağacından geçti, ama bir pencere açılıp elle kullanılmadı); launcher'dan gerçek KentOS programını başlatmak (süreç testi `/bin/sleep` ile); Windows ve macOS.
