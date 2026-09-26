# ADR 0058: Masaüstünde alt panel, paneller, tam ekran, komut arama ve sunucu denetimi

- **Durum:** kabul edildi (2026-09-26).
- **Tarih:** 2026-09-26
- **Bağlam belgesi:** CLAUDE.md §4.5, §4.6, §5, §21.1; DESIGN.md §7.3, §7.8; TODOS.md `UI-11`, `UX-06`; ADR 0017 (masaüstü kabuğu), 0041 (bulut arayüzü), 0054 (web ikonları ve ipucu)
- **Sahibin yönü (26 Eylül):** masaüstü web ile aynı düzeye getirilir; görsellik en önemli şeylerden biridir. Durum çubuğunun yazı boyu arayüzünkiyle aynı olsun.

## Bağlam

- Masaüstü Görünüm ve Araçlar menülerinin şu komutlarını "web'de var; masaüstüne henüz taşınmadı" diye geri çeviriyordu:
  - Komut geçmişi paneli (`view.bottomPanel`, F2) ve Koordinat listesi (`view.coords`);
  - Katman ve öznitelik paneli (`view.rightPanel`, F4);
  - Tam ekran (`view.fullscreen`);
  - Komut ara (`view.commandSearch`, Alt+Q);
  - Koordinat sistemi… (`crs.set`);
  - Sunucu bağlantısını denetle (`server.check`).
- Web'in davranışı koddan okundu (26 Eylül):
  - **Alt panel** (`ui/bottom/BottomPanel.ts`): komut satırının üstünde üç sekme: Komut geçmişi, Koordinat listesi, Uyarılar (görülmemiş uyarı sayısıyla). Koordinat listesi seçili noktaların Y, X, Z ve katmanını ya da ilk nesnenin köşelerini, her kenarın uzunluğu ve semtiyle gösterir.
  - **Sağ panel** (`app/commands.ts`, `toggle('view.rightPanel', …)`): katman ve özellikler paneli birlikte gizlenir ve aynı düzende geri gelir.
  - **Tam ekran:** tarayıcının Fullscreen API'si; Esc ya da aynı düğme çıkar. Düğmesi şeridin sekme satırının sağındadır (`ui/shell/fullscreenButton.ts`).
  - **Komut ara:** şeritte arama kutusuna, klasik arayüzde komut satırına odaklanır.
  - **Koordinat sistemi…:** Proje ayarları, koordinat sistemi sayfasında açılır (`openProjectSettings('crs')`). Durum çubuğunun koordinat sistemi hücresi tıklanınca aynısını yapar.
  - **Sunucu denetimi** (`app/server.ts`): `GET /v1/health`, 4 saniye bekler. Sözleşme sürümü uygulamanınkinden başkaysa sunucu uyumsuzdur. İletiler: "Sunucu bağlı: {hizmet} {sürüm}.", "Sunucu uyumsuz. …", "Sunucu yok. … Çizim sunucusuz çalışmaya devam ediyor."

## Karar

### Alt panel (`apps/desktop/src/bottom.rs`)

- F2 alt paneli açar ya da kapatır. Komut satırının üstündeki üç sekme web'inkilerdir: Komut geçmişi, Koordinat listesi, Uyarılar. Uyarılar sekmesi görülmemiş uyarıları sayar.
- Koordinat listesi seçili noktaların Y, X, Z ve katmanını, yoksa ilk nesnenin köşelerini gösterir. Her satırda kenarın uzunluğu ve semti vardır. Satırlar kaydırıldıkça kurulur.
- Sekmelerin içeriği komut satırının zeminindedir (alan rengi). İlk hâlinde arada pencerenin gri zemini bir şerit gibi görünüyordu.

### Koordinat listesinin alanı (iki platformda da)

- Liste bir kapalı alanın dış halkasıyla deliklerini tek dizi yapıyordu. Altındaki toplam bu dizinin shoelace alanıydı ve yayları kiriş sayıyordu. Örnek parsel 742,26 m² iken 633,50 m² görünüyordu. Bir kenar da dış halkadan deliğe uzanıyordu.
- Artık iki platform nesnenin kendi alanını ve uzunluğunu çekirdekten alır (`entity_area`, `entity_length`): yaylar izlenir, delikler düşülür. Özellikler paneli de bunu gösterir.
- Her kenar kendi halkasında kalır: web'de `coordinates.ts` `vertexListing`, masaüstünde `Listing` ve `Ring`.

### Durum çubuğu

- Hücrelerin hepsi KentOS UI etiketleriyle yazılır: arayüzün yazı boyu ve yazı tipi, ikincil renk (web'in `--fs-xs`, `--c-text-2`). Önceden Iced'in varsayılan 16 px'iydi ve yazı boyu ayarını izlemiyordu.
- Koordinat sistemi hücresi web'inki gibidir:
  - ikonu web'in `crs` ikonu;
  - ipucu "EPSG:{srid}. Y sağa, X yukarı değerdir. Değiştirmek için tıklayın.";
  - tıklanınca `crs.set` çalışır.

### Paneller ve tam ekran (`apps/desktop/src/view_commands.rs`)

- **F4** katman ve özellikler panellerini birlikte kapatır. İkinci basış, kapatılırken saklanan düzeni olduğu gibi geri koyar: kenarlar, sekmeler, boyutlar, yüzen pencereler (`App::hidden_docks`).
  - İkisi de başlığına daraltılmışsa F4 onları açar.
  - Komutun işareti (şeritteki basılı görünüş) panellerden biri görünürken yanar.
- **Tam ekran** pencerenin kipini değiştirir (`window::set_mode`, `Fullscreen` ya da `Windowed`).
  - Şeridin sekme satırının sağında, belgenin adından sonra web'in düğmesi vardır: tam ekrandayken ikonu `fullscreenExit`, ipucu "Tam ekrandan çık", kısayolu Esc.
  - **Esc, web'den ayrılır.** Tarayıcıda Esc tam ekrandan her zaman çıkar; bu tarayıcının kuralıdır. Masaüstünde Esc önce komutun, değer alanının ve seçimin işidir (ADR 0018). Tam ekrandan yalnız iptal edilecek bir şey kalmadıysa çıkar. Açıklaması masaüstünde bunu söyler (`DESKTOP_DESCRIPTIONS`).
  - Pencere yöneticisinin kendi kısayoluyla çıkılırsa komutun işareti bunu bilmez. Iced kip değişikliğini bildirmiyor.

### Komut arama (Alt+Q)

- Masaüstünün şeridinde henüz arama kutusu yok. Alt+Q web'in klasik arayüzündeki gibi komut satırına odaklanır ve bütün komutların listesini açar; yazdıkça süzülür.
- Liste ilk kez bütün komutlarla açılınca iki kusur göründü ve düzeltildi (KentOS UI `command_line`):
  - Adı olmayan komut kimliğiyle listelenir (`analysis.volume`). 100 px'lik ad sütunu bu adları başlığın üstüne taşırıyordu. Sütun artık listedeki en uzun ada göre genişler. En çok 20 harftir, eş aralıklı yüzde harf başına 0,6 em; daha uzun ad kırpılır. Başlık da kendi sütununda kırpılır. Liste 460 px'ten 500 px'e genişledi.
  - Masaüstünde çalışmayan komutlar listede şeritteki gibi soluktur (`Command::dimmed`). Açıklama satırı önce nedenini söyler: "Web'de var; masaüstüne henüz taşınmadı." ya da web'in bekleyen notu.
- Komut satırının ipucu ("Komut ya da koordinat yazın; …") komut çalışırken gösterilmez. ADR 0056'nın görüntülerinde KentOS UI'ın kendi varsayılanı "Komut yazın" istemin yanında görünüyordu. Artık o da boştur.

### Koordinat sistemi…

- `crs.set` Proje ayarları'nı koordinat sistemi sayfasında açar (`project::COMMANDS`). Açık çizim yoksa Proje ayarları gibi bunu söyler.
- Şeridin Koordinat sistemi grubunun pencere açıcısı (↘) da bununla çalışır.

### Sunucu denetimi

- `Cloud::health` (`kentos-cloud`) `GET /v1/health`'i 4 saniyelik sınırla sorar (web'in süresi).
  - İstek, görev çalışınca gider. Kurulup bırakılan görev (ör. testte) ağa çıkmaz.
- Sunucu adresi açık oturumun sunucusudur; oturum yoksa ayarın (`cloud.server`).
- İletiler web'in sözleridir:
  - "Sunucuya soruluyor: {adres}…"
  - "Sunucu bağlı: {hizmet} {sürüm}."
  - "Sunucu uyumsuz. Sunucu sözleşme sürümü {n}, uygulama {m} bekliyor. Uygulamayı ya da sunucuyu güncelleyin." (uyarı)
  - "Sunucu yok. {neden} Çizim sunucusuz çalışmaya devam ediyor."
- Denetim sürerken komut kapalıdır (web: `checking`).
- Masaüstü sunucuyu kendiliğinden sormaz; web açılışta, pencere odağa gelince ve ağ dönünce sorar. Masaüstünde bağlantının durumu bulut hücreleridir (ADR 0041, 0044).

## Bu dilimde olmayanlar

- Şeritte komut arama kutusu (web'in `RibbonSearch`'ü) ve şerit harf ipuçları (`view.keyTips`).
- Izgara (`draft.grid`, F7), çizgi kalınlığı (`view.lineWeights`), nesne izleme (`draft.tracking`), semboller (`view.symbols`).
- Klasik araç kutusu (`view.toolbox`, `view.toolboxDock`).
- Durum çubuğunda web'in sunucu hücresi ve hesap menüsü: masaüstünde hesabın adı ve Çıkış ayrı hücrelerdir (ADR 0041).
- Alt panelde web'in "Geçmişi temizle" ve "Paneli kapat" düğmeleri ve panelin yüksekliğini sürükleyerek değiştirme (96 px ile pencerenin %60'ı arası, çift tık 190 px): sonraki dilim.
- Komut çalışırken geçmiş satırlarının yanındaki soluk komut adları kayboluyor, çünkü komut satırına o sırada komut listesi verilmiyor (ADR 0056'nın görüntülerinde görülüyor): sonraki dilim.

## Doğrulama

- `view_commands::tests`:
  - F4 iki paneli kapatır ve açar;
  - sekmeleri ve genişliği değişmiş düzen aynen döner;
  - tam ekran açılıp kapanır; Esc önce seçimi kaldırır, iptal edilecek bir şey yoksa tam ekrandan çıkar;
  - `crs.set` Proje ayarları'nı açar;
  - sunucunun üç yanıtı web'in sözleriyle söylenir;
  - denetim sürerken komut kapalıdır.
- `bottom::tests`: örnek parselin 742,26 m²'si ve halkaları. Web'de `coordinates.test.ts`: delikli karenin 96 m²'si, yarım dairenin π/2'si, açık yol.
- `catalog::tests`: `ported.json` güncel. Taşınan komut 91/167 (ADR 0056'nın sekiz komutu dahil).
- `pnpm rust:test:desktop`, `pnpm rust:test`, `pnpm inventory:check`.
- Görüntüler (`view_commands::screens`, `bottom::screens`; `.run/shots/gorunum-*`, `alt-panel-*`), koyu ve açık, 1440×900 ve 1100×650:
  - F4 ile paneller kapalı ve tam ekran düğmesi "çık" hâlinde;
  - Alt+Q ile komut listesi;
  - Proje ayarları'nın koordinat sistemi sayfası;
  - Araçlar sekmesinde sunucu denetiminin iletileri;
  - alt panelin üç sekmesi.
