# ADR 0800: KentOS Launcher

- **Durum:** kabul edildi (2026-10-30).
- **İstek:** sahibin sözleri “launcher tasarımı tıpkı görseldeki gibi olsun” ve “launcher içine ileride rust ile yazılmamış
  yazılımlar da entegre edebiliriz, daha esnek estetik bir launcher tasarla”. Görsel sahibin taslağıdır (`lancher.png`, 1586 × 992,
  koyu): solda KentOS yazısı ve gezinme (Uygulamalar, Güncellemeler, İndirmeler; altta Ayarlar), üstte “Uygulamalarım”, arama ve
  zil, bir özet şeridi (“5 uygulama • 2 güncelleme”, Tümünü güncelle), resimli uygulama kartları (resim, renkli karo, ad, not, sürüm
  ve durum, Başlat), bir Güncelleme merkezi ve altta durum çubuğu.
- **Bağlam belgesi:** ADR 0801 (KentOS Mimari, KentOS Harita ve KentOS GIS ayrı programlar; dal `moduller`), 0050 (Başlangıç
  ekranı ve son dosyalar), 0054 (ikonlar), 0622 (arayüz düzeni), DESIGN.md. Ortak sözleşme `crates/native/launch` (`kentos-launch`)
  main'e bu iki ADR'den önce girdi (`78e94ffd2`).
- **Web:** bir şey değişmez (ADR 0228): launcher masaüstü programıdır. **`.kcad`:** değişmez.
- **Bağımlılık:** yeni dış kitaplık yok. Launcher çalışma alanında zaten olanları kullanır: `iced`, `kentos-ui`, `kentos-launch`,
  `png` (kartların resimleri), `rfd` (dosya seçme), `serde`, `serde_json`.

## Bağlam

Bugün KentOS tek bir programdır: `kentos-cad`. ADR 0801 onu modüllere ayırır (KentOS Mimari, KentOS Harita, KentOS GIS; her biri kendi
programı, `kentos-cad --modul <id>` de olur); KentOS YOL ve KentOS Rapor henüz yoktur. Kullanıcının modüllerini bulup başlatacağı,
son çizimlerine döneceği bir yer yoktu. Sahip bu yerin KentOS'un dışındaki programları da (Python betikleri, Java ya da Electron
programları, web uygulamaları) taşımasını istiyor.

Taslak gerçek olmayan şeyler gösterir: “v3.2.0 Güncel”, “Güncelleme var”, “%64 indiriliyor 124 MB / 194 MB”, “Otomatik güncelleme
açık”, “Bağlı”. KentOS'un bugün bir güncelleme hizmeti, bir indirme sunucusu ya da bir kurulum paketi yoktur. Launcher taslağın
düzenini ve görünüşünü alır, içeriğini KentOS'un bugünkü gerçeğinden kurar.

Başvurulan yollar (30 Ekim 2026'da okundu):

- freedesktop.org, *Desktop Entry Specification* 1.5 (27 Nisan 2020): her giriş `Type` (Application, Link, Directory; bilinmeyen tür
  yok sayılır) ve `Name` taşır. `Exec` bir komut satırıdır (program `PATH`'te aranan bir ad da olabilir); `%f`, `%F`, `%u`, `%U` alan
  kodları dosyanın ya da adresin yeridir ve bir komut satırında bunlardan en çok biri bulunur; boşluk ve özel karakter taşıyan bağımsız
  değişken çift tırnağa alınır, tırnağın içinde ters eğik çizgiyle kaçılır. `TryExec` programın kurulu olup olmadığını söyler,
  yoksa giriş yok sayılabilir; `Path` çalışma klasörüdür; `Link` türünde `URL` açılacak adrestir.
  <https://specifications.freedesktop.org/desktop-entry/latest/>,
  <https://specifications.freedesktop.org/desktop-entry/latest/recognized-keys.html>,
  <https://specifications.freedesktop.org/desktop-entry/latest/exec-variables.html>
- freedesktop.org, *XDG Base Directory Specification* 0.8 (8 Mayıs 2021): `$XDG_DATA_HOME` (`~/.local/share`) kullanıcının verisi,
  `$XDG_CONFIG_HOME` (`~/.config`) ayarları, `$XDG_STATE_HOME` (`~/.local/state`) yeniden başlatmada kalan ama taşınacak kadar önemli
  olmayan durum: “logs, history, recently used files”. “All paths set in these environment variables must be absolute”: göreli yol
  geçersiz sayılıp yok sayılır. <https://specifications.freedesktop.org/basedir/latest/>
- JetBrains, *Toolbox App › Manage local projects and IDEs*: Tools sekmesinde kurulu araçlar Installed altında, her birinin yanında
  bir seçenek menüsü (“Show option menu”); Projects sekmesi son projeleri listeler, menüsünden projenin hangi IDE sürümüyle açılacağı
  değiştirilir, proje listeden çıkarılır ya da Finder'da gösterilir.
  <https://www.jetbrains.com/help/toolbox-app/manage-local-projects-and-ides.html>
- Autodesk, *Autodesk Access Help*: “Autodesk Access comes with most Autodesk product installations and keeps them up to date. It
  checks daily for updates, lists the ones that apply to you, and installs them in a few clicks.” Autodesk Desktop App'in 2023'te
  Autodesk Access'e geçtiği Autodesk'in destek makalesinin başlığındadır (“Is the Autodesk Desktop App now called Autodesk Access”;
  sayfanın kendisi açılamadı, HTTP 403). <https://help.autodesk.com/view/ACCESS/ENU/>,
  <https://www.autodesk.com/support/technical/article/caas/sfdcarticles/sfdcarticles/Is-the-Autodesk-Desktop-App-now-called-Autodesk-Access.html>
- Valve, *The New Steam Library*: kitaplığın ana sayfası güncellemelere, son oynananlara ve koleksiyonlara hızlı erişimdir; son
  oynananların rafında oyuna dönmenin “big green button”ı vardır. Steamworks, *Library Assets*: kitaplık kapağı (600 × 900) oyunun
  “the primary way of presenting your game within the Steam Library”dir; başlık (920 × 430), kahraman resmi (3840 × 1240) ve logo
  ayrıdır. <https://store.steampowered.com/libraryupdate>, <https://partner.steamgames.com/doc/store/assets/libraryassets>

KentOS'un çıkardığı:

- **Bir uygulama, onu anlatan küçük bir dosyadır** (masaüstü girişi gibi): adı, simgesi, nasıl başladığı. Ama `.desktop` değil,
  KentOS'un kendi JSON biçimi: aynı dosya Windows ve macOS'ta da okunur; bağımsız değişkenler bir dizidir (tırnak ve kaçış kuralı
  yok, kabuk yok); tek alan kodu vardır, `{dosya}`, çünkü KentOS'un programları bir seferde tek dosya açar (`kentos_launch::args`
  ikinci dosyayı reddeder); kartın resmi, rengi, “yakında” durumu ve KentOS modülü türü biçimin kendisindedir. `TryExec` gibi,
  programın bulunup bulunmadığı başlatmadan önce söylenir.
- **Dosyalar XDG klasörlerinde**, KentOS'un masaüstü programının yaptığı gibi (ADR 0050); göreli değişken yok sayılır.
- **Toolbox'un seçenek menüsü ve Projects sekmesi:** her kartın ⋯ menüsü; Son dosyalar sayfası (açılır, klasöründe gösterilir).
- **Access'in güncelleme merkezi** taslakta vardır; KentOS'un bir güncelleme hizmeti yoktur. Launcher denetlemiyor, indirmiyor ve
  öyleymiş gibi görünmüyor (§6).
- **Steam'in kitaplığı gibi** her uygulama kendi resmiyle görünür; resimler KentOS'un kendi ekranlarıdır.

## Karar

### 1. Ayrı bir program, KentOS'un kitaplığıyla

- Yeni program `kentos-launcher` (`apps/launcher`, çalışma alanının üyesi, öntanımlı üyelerden değil). Iced ve KentOS UI ile
  yazılır, KentOS'un temasını, yazı tiplerini ve ikonlarını kullanır; `kentos-cad`'e bağlı değildir. KentOS'la ortak olan
  `kentos-launch`'tadır: modüllerin kimlikleri ve adları, bir modülü başlatan komut satırı (`[--modul <id>] [dosya]`), programın
  launcher'ın yanında ve `PATH`'te aranması, son dosyaların listesi.
- Pencere 1440 × 900 açılır, en küçük 1100 × 650 olur (CLAUDE.md §1). Sayfalar: Uygulamalar, Son dosyalar, Güncellemeler,
  İndirmeler, Ayarlar.
- `kentos-launcher snapshot çıktı.png [--sayfa …] [--tema …] [--boyut …] [--yazi …] [--olcek …] [--secili …] [--bildirimler]
  [--imlec …]` pencereyi açmadan çizer (KentOS'un `snapshot`'ı gibi): sabit veriyle, kullanıcının hiçbir dosyasını okumadan; resimler
  her makinede aynıdır. `--sayfa ikonlar` simge sayfasıdır (açık ve koyu, 16–56 px, karolu ve karosuz).

### 2. Her uygulama bir manifesttir

- **Biçim `kentos.launcher-app/1`** (`docs/launcher/README.md`, örnekleriyle): `format`, `id`, `name`, `note`, `icon`, `color`,
  `picture`, `version`, `state` (`ready`, `soon`), `launch`, `order`, `files`. `launch`'ın üç türü:
  - `program`: herhangi bir dilde herhangi bir program; `program` tam yol, manifestin klasörüne göre yol ya da `PATH`'teki ad;
    `args` dizi, `{dosya}` açılacak dosyanın yeri (bir bağımsız değişkenin bütünü ya da parçası); `env`; `cwd` (öntanımlı manifestin
    klasörü).
  - `url`: `http://` ya da `https://`; sistemin açıcısıyla (`xdg-open`, `open`, `explorer`).
  - `kentos`: `module` `mimari`, `harita`, `cbs`. Adı ve notu modülün, sürümü launcher'ın (aynı çalışma alanından derlenirler).
- **Üç yer, bu sırayla:** launcher'ın içindekiler (`apps/launcher/assets/apps`: KentOS Mimari, Harita, GIS; YOL ve Rapor “yakında”),
  kurulumun klasörü (launcher'ın yanındaki `uygulamalar/`), kullanıcının klasörü (`$XDG_DATA_HOME/kentos/uygulamalar`). Aynı `id`
  sonraki yerde geçerse öncekinin yerine geçer: bir kurum ya da kullanıcı bir KentOS modülünü kendi kurulumuna yöneltebilir. Aynı
  klasörde ikinci kez geçen `id` yok sayılır ve söylenir. Kartlar `order`'a, eşitse ada göre (KentOS UI'ın Türkçe metin sırası).
- **İki tür sorun.** Başlatmayı bozan (biçim, kimlik, ad, `launch`) manifesti dışarıda bırakır; yalnız görünüşü ilgilendiren (simge,
  renk, resim, sürüm, sıra, dosyalar) öntanımlıyla okunur. İkisi de dosyasıyla ve nedeniyle bildirimlere yazılır; hiçbir manifest
  launcher'ın açılmasını engellemez. Bilinmeyen alan yok sayılır (sonraki sürümün alanları eski launcher'ı bozmaz); dosya en çok
  256 KB.
- **Uygulama ekle, Düzenle, Kaldır** yalnız kullanıcının klasöründe: pencere ad, açıklama, tür (Program ya da Web adresi), program
  (Seç… ile ya da yazılarak), bağımsız değişkenler (boşlukla, tırnaklı), adres, renk ve simgeyi sorar, manifesti yazar. Pencerenin
  göstermedikleri (resim, sürüm, dosyalar, ortam, çalışma klasörü, sıra, kendi simge dosyası) düzenlenen manifestten korunur. Yeni
  uygulamanın kimliği adından (ASCII, tireli, boşta). Dosya bütün yazılır, sonra yerine taşınır.
- **Güven.** Manifest bir program başlatır: bir `.desktop` dosyası kadar güvenilirdir. Launcher manifesti yalnız bu üç yerden okur,
  ağdan almaz; programı kabuksuz, bağımsız değişkenlerini dizi olarak verir.

### 3. Pencere: taslağın düzeni, gerçek içerik

- **Düzen taslağınkidir:** solda KentOS'un işareti ve adı, sayfalar (seçili olan vurgu çubuklu), altta Ayarlar; üstte sayfanın adı
  ve ne işe yaradığı, arama (Türkçe harfler katlanır, her sözcük aranır), zil; özet şeridi; kartlar; Güncelleme merkezi; durum
  çubuğu. Kartlar ızgarada, sayfanın genişliğine göre iki ile dört sütun, kenarları tam piksellerde; 1440 × 900'de üç sütun ve iki
  satır kaydırmasız, 1100 × 650'de iki sütun ve kaydırma. Bir kartın parçaları sabit boylu: aynı satırın kartları aynı hizadadır.
- **Kart:** resim (2:1), renkli karo ve simge, ad ve not (sığmayan “…” ile kısalır, bütünü ipucunda), ⋯ menüsü, sürüm ve durum,
  ana düğme. Ana düğme üzerine gelinen ya da seçili kartta dolu, öbürlerinde çizgilidir (taslakta seçili kart doludur).
- **Resimler gerçek ekranlardır:** `scripts/launcher/pictures.py` KentOS'un kendisini (`kentos-cad snapshot`) deponun çizimleriyle
  çalıştırır: Mimari için `samples/konut-3-1.kcad`'in planı, Harita için etiket motorunun parselleri, GIS için tematik haritalar,
  YOL için Plan yolu'nun kavşağı, Rapor için bir parsel katmanının grafiği. 720 × 360 PNG, beşi birlikte 489 859 bayt. Taslağın üç
  boyutlu bina resmi gibi KentOS'un yapamadığı bir şey gösterilmez.
- **Simgeler:** web'in 20 × 20 çizgi simge kümesinden (`stakeout` Harita'nın ölçme aleti, `modeGis` GIS'in haritası, `document`
  Rapor, `crs` web, `chart` grafik, ayarlar, dışarıda açma); kümede olmayanlar (bina, yol, dört karo, zil) aynı ızgarada yeni çizildi.
  Karonun sekiz rengi (`blue`, `teal`, `green`, `yellow`, `orange`, `red`, `purple`, `gray`) yukarıdan aşağı açılan bir geçişle
  boyanır; simge ile karo arasında her renkte en az 3:1 karşıtlık (test). Simge karonun içinde tam piksellere oturur.
- **Tema ve yazı:** Sistem (işletim sisteminin teması izlenir), Açık, Koyu; yazı boyutu 11–18 px, bütün ölçüler onunla büyür. 18 px'te
  ve 1100 × 650'de de her şey sığar: özet şeridinin düğmeleri sığmayınca ikonlarına iner, arama başlığın bıraktığı yeri alır.
- **Klavye:** Ctrl+F (macOS'ta ⌘F) arama, oklar kartlar arasında, Enter seçili kartı ya da görünen ilk başlatılabilir kartı başlatır,
  Esc sırasıyla pencereyi, bildirimleri, aramayı kapatır.

### 4. Başlatma

- **Ne çalışır** (`resolve`): `kentos` türünde önce modülün kendi programı (`kentos-mimari`, `kentos-harita`, `kentos-gis`), yoksa
  `kentos-cad --modul <id>`; ikisi de önce launcher'ın yanında, sonra `PATH`'te. `program` türünde tam yol, manifestin klasörüne göre
  yol ya da `PATH`'teki ad. Bulunamayanın kartı “Bulunamadı” der, düğmesi **Konumu seç…** olur, nedeni ipucundadır. Seçilen yer
  yalnız o uygulama için `$XDG_CONFIG_HOME/kentos-launcher/konumlar.json`'a yazılır, ⋯ menüsünde **Konumu sıfırla** bırakır; yeri
  kaybolan konum söylenir.
- **Her uygulama kendi sürecidir.** Kabuksuz başlar, standart girdisi boş, çıktısı ve hatası
  `$XDG_STATE_HOME/kentos-launcher/gunluk/<id>.log`'a (önceki kopyası çalışıyorsa `<id>-2.log` …); Unix'te kendi süreç grubunda
  (launcher'ın terminaline basılan Ctrl+C ona gitmez). Launcher onu beklemez: bir şey çalışırken saniyede bir bakar. 0'dan başka bir
  kodla ya da sinyalle kapanan programın bildirimi nedenini, çıktısının son dolu satırını ve günlüğün yerini söyler. Launcher
  kapanınca başlattıkları açık kalır. Çalışan uygulamanın kartı “Çalışıyor” (birden çoksa sayısıyla) der; Başlat bir tane daha açar.
- **Dosya aç…:** `files`'ı olan uygulamanın ⋯ menüsünde; seçilen dosya `{dosya}`'nın yerine ya da bağımsız değişkenlerin sonuna.
  KentOS modülünde `kentos-cad`'e `--modul <id> <dosya>`, modülün kendi programına `<dosya>` (`kentos_launch::args::command_line`).
- **Bildirimler** (zil): okunamayan manifestler, başlamayan ve hatayla kapanan programlar; en yenisi önce, okunmamışı varken zilde
  nokta, biri biri ya da hepsi temizlenir. Bellektedir, dosyaya yazılmaz; en çok 100.
- **Uygulama başlayınca küçült** (Ayarlar, kapalı başlar): launcher görev çubuğuna iner.

### 5. Son dosyalar ve Ayarlar

- **Son dosyalar** `kentos_launch::recent`'in listesidir (`$XDG_STATE_HOME/kentos-cad/son-dosyalar.json`): KentOS'un programları
  yazar, launcher yalnız okur. Satırda ad, klasör (ev `~` ile), hangi modülde çalışıldığı (liste biliyorsa), programın yazdığı bilgi
  (nesne sayısı, koordinat sistemi) ve ne zaman (“az önce”, “3 saat önce”, “dün”; tarih değil, saat dilimi tahmin edilmez). Yerinde
  olmayan dosya soluk ve “Bulunamadı”dır, açılmaz. Açmak dosyayı çalışıldığı modülde, modülü bilinmiyorsa `kentos-cad <dosya>` ile
  açar. Arama adda ve klasörde.
- **Ayarlar** `$XDG_CONFIG_HOME/kentos-launcher/ayarlar.json`: tema, yazı boyutu, Uygulama başlayınca küçült; uygulama klasörleri
  (Klasörü aç; kullanıcının klasörü açılınca oluşturulur, kurulumun klasörü yoksa düğme kapalıdır ve nedenini söyler); Hakkında.
  Okunamayan ayar dosyası öntanımlılarla açılır.

### 6. Güncelleme ve indirme yok; söylenir

- Güncelleme denetimi, indirme ve kurma bu sürümde yoktur. Launcher bunu her yerde açıkça söyler, hiçbir şeyi öyleymiş gibi
  göstermez: özet şeridinin **Tümünü güncelle**'si kapalıdır ve ipucu nedenini söyler; gezinmede Güncellemeler'in sayısı yoktur;
  Güncelleme merkezi kurulu uygulamaların sürümlerini listeler ve “Güncelleme denetimi henüz yok” der; Güncellemeler sayfası kurulu
  sürümleri ve yeni sürümün kurulum paketiyle kurulacağını, İndirmeler sayfası launcher'ın bir şey indirmediğini söyler; durum
  çubuğu “Güncelleme denetimi yok” der. Taslağın “Bağlı” sözünün karşılığı yoktur (launcher sunucuya bağlanmaz): durum çubuğunun
  solu “Hazır” ya da çalışan programlardır.
- Kartın sözleri yalnız bilinenlerdir: Kurulu (programı bulundu), Çalışıyor, Web adresi, Bulunamadı, Yakında.

### 7. Sonraya kalanlar

- **Güncelleme hizmeti:** imzalı bir sürüm listesi (kanal, sürüm, paket, SHA-256, imza), denetleme, gerçek ilerlemeyle indirme,
  imza ve özet denetimi, kurma ve geri dönme; o zaman Tümünü güncelle, Güncellemeler'in sayısı ve İndirmeler sayfası çalışır.
- **Paketleme:** Linux (deb, rpm, AppImage), Windows (MSI), macOS (dmg); launcher'ın kendi `.desktop` girişi ve simgesi; kurulumun
  `uygulamalar/` klasörü.
- **İmzalama:** Windows Authenticode, macOS imzası ve onaylatma (notarization); manifestlerin ve sürüm listesinin imzası.
- **Modül başına kırpma:** ADR 0801'in programları bugün aynı kitaplıkla derlenir; her modülün yalnız kendi kodunu taşıması ayrı iştir.
- Sistemin `.desktop` girişlerini okuyup kart yapmak; manifestin kendi simgesini pencereden seçmek; uygulamaların sırasını
  sürükleyerek değiştirmek; Windows ve macOS'ta denemek.

## Uygulama

- `apps/launcher/Cargo.toml`; `src/main.rs` (pencere ve `snapshot`), `app.rs` (durum ve güncelleme), `manifest.rs` (biçim, okuma,
  yazma), `registry.rs` (üç yer), `draft.rs` (Uygulama ekle ve Düzenle), `resolve.rs` (ne çalışır), `process.rs` (süreçler,
  günlükler, açıcı), `locations.rs` (Konumu seç…), `notices.rs`, `recent.rs`, `settings.rs`, `files.rs` (XDG ve bütün yazma),
  `cards.rs` (kartın durumu, arama, ızgara), `glyphs.rs`, `tones.rs`, `pictures.rs`, `snapshot.rs`; `src/view/` (`mod.rs`,
  `style.rs`, `sidebar.rs`, `header.rs`, `apps.rs`, `pages.rs`, `dialog.rs`, `notices.rs`, `status.rs`).
- `apps/launcher/assets/apps/*.json` (beş yerleşik manifest), `apps/launcher/assets/pictures/*.png` (beş resim),
  `scripts/launcher/pictures.py`.
- `docs/launcher/README.md` (biçim) ve `docs/launcher/ornekler/` (Python betiği, Java programı, Electron programı, web adresi,
  yerleşik bir modülün yerine geçen kurum manifesti).
- `Cargo.toml` (üye), `package.json` (`rust:test:desktop`'ın testine ve clippy'sine `-p kentos-launcher`).

## Sonuçlar

- Çalışma alanında yeni bir program; masaüstü takımı onu da sınar. Web, `.kcad`, komutlar, Python ve MCP değişmez.
- Bir kurum KentOS'un dışındaki programlarını tek bir JSON dosyasıyla launcher'a koyar; KentOS'un kendi modülleri de aynı biçimdir.
- **ADR 0801'e bağlılık:** KentOS kartları ADR 0801'in programlarını (`kentos-mimari` …) ya da `kentos-cad --modul <id>`'yi başlatır.
  main'deki bugünkü `kentos-cad` ilk bağımsız değişkenini dosya sayar ve `--modul`'ü bilmez (`apps/desktop/src/main.rs`); bunu okuyan
  `kentos-cad` ve modüllerin programları dal `moduller`'dedir. O dal birleşene kadar launcher'dan başlatılan `kentos-cad`, “--modul”
  adlı bir dosyayı açmaya çalışır. Modülü bilinmeyen son dosya (`kentos-cad <dosya>`) bugün de açılır.

## Doğrulama

- **Birim testleri:** `cargo test -p kentos-launcher`: 44 test geçti. Manifest (okuma, yazıp geri okuma, reddedilen alanlar, görünüşün
  öntanımlıları, KentOS modülünün adı, `docs/launcher/ornekler`'in beş örneğinin uyarısız okunması); üç yer ve kimliğin yerine
  geçmesi, bozuk ve gizli dosyalar; Uygulama ekle, Düzenle (kendi simge dosyası korunur), Kaldır, yerleşik olanın yazılamaması;
  ne çalışır (modülün kendi programı, yoksa `kentos-cad --modul`, her modülün komut satırının `kentos_launch::args::parse` ile geri
  okunması, gösterilen yer, `{dosya}`, bağımsız değişkenlerin bölünüp birleşmesi); süreçler (`/bin/sh` ile: iyi biten, 1 koduyla ve
  sinyalle biten, son satır, ikinci kopyanın ayrı günlüğü, başlamayan program, platformun açıcısı); Konumu seç…'in saklanması; son
  dosyaların satırları ve zamanları; ayarların gidiş-dönüşü ve bozuk dosya; bildirimler; kartın durumları, özet, arama, 2–4 sütun;
  1440 × 900 ve 1100 × 650'de ızgara; okların seçimi gezdirmesi; bulunamayan programın söylenmesi; son çizimin modülünde açılması;
  karoların karşıtlığı; resimlerin boyu (720 × 360, toplam ≤ 1,5 MB) ve PNG çözümü.
- **Denetimler:** `cargo clippy -p kentos-launcher --all-targets -- -D warnings` temiz; `node scripts/arch/deps.mjs` (“Bağımlılık yönü
  temiz”); launcher'ın dosyaları `rustfmt --edition 2024` ile.
- **Resimler** (`kentos-launcher snapshot`, `.run/shots/`, depoya girmez): Uygulamalar koyu ve açık × 1440 × 900 ve 1100 × 650 × 13
  ve 18 px (sekiz); Son dosyalar, Güncellemeler, İndirmeler, Ayarlar, Uygulama ekle, Bildirimler; Uygulamalar 2× ölçekte; simge
  sayfası; taslakla yan yana. Resimlerde bulunup düzeltilenler: açık temada kapalı bağlantının rengi; 1100 px'te özet şeridinin
  düğmelerinin sığmaması (ikonlarına iner); 1440 px'te gerilen Ayarlar ve Güncellemeler panelleri; Uygulama ekle'de tek satıra
  sığmayan simgeler (iki satır); 18 px'te yarı kalın adın düz yazının genişliğiyle kısaltılıp iki satıra kırılması (artık çizildiği
  ağırlıkla ölçülür); arama kutusunun ve karodaki simgenin yarım piksele düşmesi.
- **Denenmeyenler:** gerçek pencerede fare ve klavyeyle kullanım (güncelleme mantığı birim testlerinde, görünüş `snapshot`'ta);
  KentOS programlarını launcher'dan gerçekten başlatmak (ADR 0801'in programları bu dalda yok); Windows ve macOS (açıcı, süreç grubu,
  yollar); sistemin temasının pencere açıkken değişmesi; dosya seçme pencereleri (`rfd`); güncelleme, paketleme, imzalama (§7).
