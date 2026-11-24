# ADR 9308: Kullanıcının kısayolları ve takma adları

- **Durum:** kabul edildi (2026-11-24). todos-new.md B4.
- **Kural:** CLAUDE.md, “Modül ve otomasyon denetimi (kesin kural, 24 Kasım 2026)”, §25 (kural 1–3), §8, kılavuz §9.3 (Ayar).
- **Web:** iptal edildi; yok (ortak durumların TypeScript çalıştırıcısı donmuş web'dedir ve yeni bölümleri okumaz).
  **`.kcad`:** değişmedi (kısayol proje değil, kişi ayarıdır). **Komut:** yeni komut yok.

## Bağlam

Uygulama ayarlarında yalnız okunur “Klavye kısayolları” listesi (F1, `shortcuts.rs`) ve kapalı gelen klasik CAD komut
adları profili (`drafting.autocadAliases`, ADR 0282; saklanan anahtar sahibin kararıyla olduğu gibi kalır) vardı. Kullanıcı
bir komuta kendi tuşunu veremiyor, bir tuşu başka komuta taşıyamıyor, komut satırına kendi adını tanımlayamıyordu.
Tuşlar katalogdan gelir (`docs/inventory/web.json`, `catalog/v1/additions.json`), tuşa basış `keys.rs`'te web'in yazdığı
biçime çevrilir (`Ctrl+Shift+Z`, macOS'ta ⌘ da `Ctrl+`), `App::shortcut` katalog sırasıyla ilk sunulan komutu bulur.

### Araştırma (kaynaklar yalnız burada anılır)

| Konu | Kaynak | Ne yapılıyor |
|---|---|---|
| Çakışma | QGIS kaynak kodu, `qgsconfigureshortcutsdialog.cpp` (api.qgis.org/api/qgsconfigureshortcutsdialog_8cpp_source.html) | “This shortcut is already assigned to action %1. Reassign?” Evet/Hayır; dosyadan yüklerken “Yes to All / No to All”; Esc kaydı bırakır |
| Çakışma | AutoCAD, “To Customize Shortcut Keys” (help.autodesk.com/cloudhelp/2015/ENU/AutoCAD-LT/files/GUID-9970E91B-A4F5-405C-99C1-46AD804A8858.htm) | Shortcut Keys penceresinde “Currently Assigned To”; istenmiyorsa “use a different key combination” |
| Çakışma | AutoCAD for Mac, “To Customize Shortcut Keys” (help.autodesk.com/cloudhelp/2023/ENU/AutoCAD-MAC-Customization/files/GUID-A14AAC99-BF1F-422B-A546-606C3CEC00C8.htm) | Record Shortcut; tuş başka komuttaysa “an alert displays indicating which command uses the shortcuts keys”; Reset to Default bütün özel tuşları siler |
| Çakışma, bağlam, sıfırlama, dışa aktarma | ArcGIS Pro, “Use keyboard shortcuts” (doc.esri.com/en/arcgis-pro/latest/get-started/manage-keyboard-shortcuts.html) | Birkaç varsayılan değişmez, soluk gösterilir; aynı alt grupta çakışmada “Assign Shortcut” sorusu ve sarı uyarı şeridi; Global ve işlev bağlamı (bağlam kazanır); “Only one unique key can be used in a shortcut”; Restore shortcut, Reset Table, Reset All; özelleştirme `.proExportedUI` ile dışa aktarılır |
| Kaydet/yükle | QGIS Kullanıcı Kılavuzu, “Keyboard shortcuts” (docs.qgis.org/latest/en/docs/user_manual/introduction/qgis_configuration.html) | Change, Set None, Set Default; yalnız kullanıcının ya da bütün tuşlar XML'e; PDF; başka kurulumda yükle |
| Takma ad | AutoCAD, “To Manage Command Aliases” (help.autodesk.com/cloudhelp/2025/ENU/AutoCAD-Customization/files/GUID-D4CACED6-DFBA-43C3-BC42-8D980AB3AE75.htm) | Adlar PGP dosyasında; kullanıcınınkiler dosyanın sonuna; “The last definition is the one the product loads”; dosya elle düzenlenince REINIT; macOS'ta ALIASEDIT kullanıcı dosyasına (`acaduser.pgp`) yazar, Load eski dosyayı alır |
| Takma ad | AutoCAD for Mac, Aliases sekmesi (help.autodesk.com/cloudhelp/2018/ENU/AutoCAD-LT-MAC/files/GUID-6230DE90-E6D8-4FAC-ACC1-7668F70F448E.htm) | Programın adları ve kullanıcınınkiler ayrı dosyada; liste hepsini ya da yalnız özelleri gösterir |
| Takma ad, tuş | BricsCAD, Customize penceresi ve Command aliases (help.bricsys.com/en-us/document/dialog-boxes/c/customize-dialog-box; …/customization/command-aliases) | Keyboard ve Command Aliases sekmeleri; var olan ada düzenlemede üzerine yazma sorulur; pencereden yapılan değişiklik yeniden yükleme istemez |
| İki basışlı tuş, kaynak | Visual Studio Code, “Key Bindings” (code.visualstudio.com/docs/configure/keybindings) | Kullanıcı tuşları ayrı dosyada; `-komut` ile varsayılan kaldırılır; iki basış boşlukla (Ctrl+K Ctrl+C); `when` bağlamı; “Show Same Keybindings” |
| İki basış, sıfırlama | JetBrains IDE'leri, “Keyboard shortcuts” (jetbrains.com/help/idea/configuring-keyboard-and-mouse-shortcuts.html) | Çakışma uyarısı; işletim sisteminin tuşuyla çakışmada ayrı uyarı; “Second stroke”; eylem başına Reset Shortcuts; kişisel tuş haritası yalnız farkları saklar |
| İşletim sistemi (macOS) | Apple Human Interface Guidelines, Keyboards (developer.apple.com/design/human-interface-guidelines/keyboards) | Standart tuşlar: ⌘Q çık, ⌘H gizle, ⌥⌘H öbürlerini gizle, ⌘M küçült, ⌘Boşluk arama, ⌥⌘Boşluk arama sonuçları, ⇧⌘3/4 ekran görüntüsü, ⌥⌘D Dock, ⌥⌘8 büyütme; Control ile kısayoldan kaçının |
| İşletim sistemi (Windows) | Microsoft, “Keyboard (Win32 UX)” (learn.microsoft.com/en-us/windows/win32/uxguide/inter-keyboard) ve “Keyboard shortcuts in Windows” (support.microsoft.com) | “If your program assigns many shortcut keys, provide the ability to customize the assignments”; Windows'un tuşlarını ve Windows logo tuşunu kullanmayın; Ctrl+Alt bazı dillerde AltGr'dir; tuş ipucunda yazılır. Alt+F4 pencereyi kapatır, Alt+Boşluk pencere menüsü, Ctrl+Esc Başlat, Ctrl+Shift+Esc Görev Yöneticisi, Ctrl+Alt+Del güvenlik ekranı |
| İşletim sistemi (Linux) | GNOME Yardım, “Useful keyboard shortcuts” (help.gnome.org/users/gnome-help/stable/shell-keyboard-shortcuts.html.en) | Alt+F2 komut penceresi, Alt+Esc pencereler, Ctrl+Alt+Tab üst çubuk, Ctrl+Alt+Delete kapatma, Shift+Ctrl+Alt+R ekran kaydı |
| Türkiye'deki harita yazılımı | Netcad | Kısayol özelleştirmesinin resmî belgesi bulunamadı (yalnız üçüncü kişilerin varsayılan listesi); karar ona dayanmaz |

Ortak noktalar: tuş başka komuttaysa o komut adıyla söylenir ve onay istenir; tek komutun ya da bütün tuşların varsayılana
dönüşü vardır; yalnız kullanıcının farkları saklanır ve başka kuruluma taşınır; bazı tuşlar değişmez; bağlam (görünüm,
düzenleyici) tuşun anlamını değiştirebilir; takma adlar programın adlarından ayrı bir kullanıcı listesindedir.

## Karar

### 1. İki tipli kullanıcı ayarı (§9.3)

`crates/shared/contracts/src/core/settings/` yeni bir ayar türü alır: `SettingType::Map` (metinlere göre metin ya da metin
listesi; `max` giriş sayısı; anahtar ve her metin en çok 128 karakter, bir giriş en çok 8 metin). İki ayar, `keyboard`
grubunda, kullanıcı kapsamında, masaüstünün, hemen geçerli (`Live`), öntanımlısı boş:

- `keyboard.shortcuts` (en çok 2 000 giriş): komut kimliği → tuşları (`{"tool.rectangle": ["R", "Ctrl+Shift+K"]}`).
  Listede olmayan komut kendi tuşlarını kullanır; boş liste komutun tuşlarını kaldırır.
- `keyboard.aliases` (en çok 500 giriş): ad → komut (`{"DKG": "tool.rectangle"}`).

Ortak sözleşme yalnız biçimi denetler (eşlem, metin, uzunluk: `wrong_type`, `out_of_range`). Bir girişin anlamı (komutu,
tuşu) onu kullanan yerde, o istemcinin komutlarıyla okunur: bu sürümün okuyamadığı giriş (bilinmeyen komut, okunamayan
tuş, KentOS'un ya da işletim sisteminin tuttuğu tuş, KentOS'un adı) **ayarda kalır**, kullanılmaz ve Kısayollar bölümünde
nedeniyle söylenir. Böylece daha yeni bir sürümün ya da başka bir bilgisayarın yazdığı giriş kaybolmaz (ayarların bilinmeyen
anahtar kuralı, ADR 0023).

Ortak kurallar (`keymap.rs`, sözleşmede):

- **Tuşun yazılışı** (`normalize_chord`): değiştiriciler Ctrl, Alt, Shift sırasıyla, sonra tek tuş: A–Z, 0–9, F1–F24,
  Esc, Enter, Space, Delete, Backspace, Tab, Home, End, PageUp, PageDown, Up, Down, Left, Right, `+`, `-`, `,`. Büyük
  küçük harf ve değiştirici sırası serbest okunur, tek biçimde yazılır; `+` ve `-` ile Shift yazılmaz (bazı klavyeler
  Shift'le yazar, `keys.rs` de yazmaz). Türkçe harfler tuş değildir (`keys.rs` harfi ASCII ise harf, değilse konumla okur).
- **KentOS'un tuttuğu tuşlar** (`reserved_key`, her istemcide): Esc, Enter ve Tab her değiştiriciyle; tek başına Boşluk,
  Geri silme ve ok tuşları; rakamlar (Shift'le de: klavyeye göre @ ya da ( yazar), tek başına `+`, `-`, `,`: bunlar
  yazılan değeri başlatır (ADR 0018). Her birinin nedeni Türkçe iletidir.
- **Takma adın yazılışı** (`normalize_alias`): harf ve rakam, harfle başlar (rakamla başlayan yazı komut satırında
  değerdir), en çok 32 karakter, Türkçe büyük harfle saklanır (`çiz` → `ÇİZ`, `ı` → `I`).

Ortak durumlar `fixtures/settings/v1/cases.json`: 14 değer, 2 yazma, 2 dosya durumu ile yeni `chords` (34) ve `aliases`
(9) bölümleri. Üretilenler: `KENTOS_WRITE_SETTINGS=1 cargo test -p kentos-contracts settings` (`settingsSchema.json`),
ts-rs (`SettingType.ts`, `SettingDescriptor.ts`, `ResolvedSetting.ts`, `PolicyRule.ts` değer türü eşlemi de alır).

Göç: ne masaüstü ne web kullanıcı tuşu saklamıştı; alınacak eski kayıt yok. İleriye dönük: ayarların sürümü 1'dir; anlamı
değişirse sürüm 2 ve göç gelir (ADR 0023). Komut kimliği değişirse eski giriş “bu sürümde böyle bir komut yok” diye kalır.

### 2. Tuş haritası (`apps/desktop/src/shell/keymap.rs`)

- **Bir tuş, bir komut.** Kullanıcının bir komuta verdiği tuş başka hiçbir komutun olmaz: o tuş varsayılan olarak başka bir
  komuttaysa, kullanıcınınki durdukça o komut onu kaybeder; kullanıcınınki gidince (Varsayılana dön) geri alır. Sessiz çift
  atama yoktur (bir bağlamda iki komutu aynı tuşa bağlayan ve yalnız uyaran yaklaşım alınmadı: KentOS'ta tuş anında çalışır,
  hangisinin çalıştığı belirsiz kalmamalı).
- **Kişinin, modülün değil.** Aynı harita kentos-cad'de ve her modülün programında geçerlidir. Programın sunmadığı bir
  komutun tuşu orada bir şey yapmaz (`App::offers`); başka bir komutun varsayılanına da düşmez.
- **Bağlamlar.** Çizimin tuşları haritadadır. Pafta tasarımcısının tuşları pafta öndeyken onundur ve bu dilimde
  değiştirilemez (`sheet.*`, ADR 9305 ile katalogdadır; tuşlarını `crates/sheet-ui` kendisi okur). Esc ve Enter'in komutları
  (`tool.cancel`, `tool.confirm`) değişmez. Metin alanındayken çalışan tuşlar: katalog izin verdiği komutun kendi tuşu ya da
  kullanıcının ona verdiği Ctrl, Alt ya da işlev tuşlu tuş (harf alana yazılır).
- **İki basışlı tuş yok.** Bir tuş, değiştiricilerle tek basıştır (çizim ve harita programlarının tuş pencerelerinde de
  böyle). Sıralı yazma ihtiyacını komut satırı adları karşılar.
- **İşletim sisteminin tuşları** (programın çalıştığı sistemin, `system_key`): macOS'ta ⌘Q, ⌘H, ⌥⌘H, ⌘M, ⌥⌘M,
  ⌘Boşluk, ⌥⌘Boşluk, ⇧⌘3, ⇧⌘4, ⇧⌘5, ⌥⌘D, ⌥⌘8; Windows'ta Alt+F4, Alt+Boşluk, Ctrl+Alt+Delete; Linux masaüstünde
  Alt+F2, Ctrl+Alt+Delete, Ctrl+Alt+Shift+R (Esc, Enter ve Tab'lılar zaten KentOS'un). Ayar dosyası bilgisayarlar arasında
  taşınır: başka sistemin tuttuğu tuş burada serbesttir; bu sistemin tuttuğu tuş dosyada kalır, burada kullanılmaz ve söylenir.
- **Canlı uygulama:** harita `App::apply_settings`'te, ayarlar değişince yeniden kurulur; yeniden başlatma yoktur. Şeridin
  menüleri sonra, uygulamayı tutmayan kapanışlarda kurulduğu için harita pencerenin iş parçacığında tutulur (`active`).
- **Gösterildiği her yer:** şerit ipuçları ve menüleri, bölme listeleri, durum çubuğu, uygulama menüsü, çizim menüleri,
  başlangıç ekranı, seçim çipi, Menülerde ara (Komut ara) satırları, Klavye kısayolları penceresi (kullanıcınınki
  “kullanıcının” diye işaretli) ilk tuşu haritadan okur. Şerit harf ipuçları (Alt ya da F6) bozulmaz: Alt'a tek basış
  ipuçlarını açar, `view.keyTips`'in tuşu kullanıcınınkiyse onunla açılır.
- **Takma adlar:** komut satırında modülün kendi adlarından sonra, klasik CAD adlarından önce çalışır
  (`run_profile_name`); komut satırının listesinde ve Menülerde ara'da bulunur. KentOS'un kendi adı, başlığı, tek harfli
  tuşu ve bir modülün adı (KentOS Mimari'nin DU'su) alınamaz; klasik profildeki ad alınabilir, kullanıcınınki önce gelir.

### 3. Uygulama ayarları › Kısayollar (`settings_keys.rs`)

Kitin `ListDetails`'i: solda arama (komutun başlığı, tuşu ya da adı) ve komutlar (programın sunduğu, masaüstünün çalıştırdığı
çizim komutları; arama yokken tuşu ya da adı olanlar, kullanıcının değiştirdikleri başta; en çok 150, kalanı için aramayı
daraltın), her satırda tuş başlıkları, ilk adı ve “varsayılan” ya da “kullanıcının”. Sağda seçilen komutun formu (kitin
`Form`'u): Tuşlar (her tuşun ×'i; Tuş ekle basılan tuşu bekler, Esc vazgeçer), tuşların nereden geldiği, KentOS'un adları,
kullanıcının adları (×), Yeni ad ve Ekle, gerekirse “bu komut … programında yok” notu, Varsayılana dön. Üstte Hepsini
varsayılana döndür; pencerenin “Bu bölümü varsayılana döndür”ü de aynısını yapar. Tuş ya da ad başka komuttaysa şerit o
komutu ve tuşun kimin olduğunu (KentOS'un mu, kullanıcının mı) söyler: Taşı ya da Vazgeç. Her şey pencerenin taslağına
gider: Kaydet hemen uygular, Vazgeç bırakır (Uygulama ayarlarının kuralı). Dışa ve içe aktarma Ayar dosyası bölümündeki
dosyayla olur. Klavye kısayolları penceresinde (F1) “Kısayolları değiştir…” bu bölümü açar.

### 4. Modüller

Bölüm modülün profilindedir (`AppSetting::Shortcuts`, her profilin joker kolsuz eşlemesinde); matris
`apps/kentos-cad/tests/programs/module_surface_tests.rs` (`each_program_has_its_own_settings_sections`, Yol da eklendi).

| Modül | Var / yok | Gerekçe |
|---|---|---|
| Mimari | var | Bölüm KentOS Mimari'de ve kentos-cad'de; liste programın sunduğu komutlardır. Kullanıcının Harita komutuna verdiği tuş burada bir şey yapmaz (sınama: `a_key_of_a_command_a_module_lacks_does_nothing_in_its_program`) |
| Harita | var | Aynı; Mimari'nin Duvar'ına verilen tuş KentOS Harita'da çalışmaz, başka komuta da düşmez |
| Yol | var | Aynı |
| CBS | var | Aynı; CBS projesinin veri komutları da tuş ve ad alır |

Kısayol kişinindir: dört modülde aynı harita, aynı dosya. Bölüm, programda olmayan komutların saklı tuş ya da ad sayısını
ve seçilen komutun hangi programlarda olmadığını söyler.

### 5. Kanallar

| Kanal | Var / yok | Gerekçe |
|---|---|---|
| Python | yok | Kısayol ve takma ad çizimi ya da projeyi değiştirmez; kişinin arayüz tercihidir (§9.2 çizimi değiştiren iş içindir). Uygulama ayarları bugün Python'dan okunmaz ve yazılmaz (`kentos.cad`'de ayar yok). Python komutu kimliğiyle çalıştırır (`kentos.cad.<ad>`); tuşa ya da ada gerek duymaz |
| MCP | yok | Aynı; araçlar katalogdan komut kimliğiyle çalışır (`cad.run`, `arch.run`, `map.run`, `gis.run`, `road.run`). Bir istemcinin kişinin tuşlarını değiştirmesi beklenmedik olur |
| Yapay zekâ ajanı | yok | MCP ile aynı araç katmanı (`kentos_headless::agent_tools`); tuşu değiştirmek ajanın işi değildir |
| Eklenti | yok | Eklentinin izinleri (`document.read`, `document.write`) çizim içindir, uygulama ayarı izni yoktur; bir eklentinin Ctrl+S gibi tuşları kendine alması güvenlik açığı olurdu. Eklenti komutları katalogda değildir; tuş alamazlar |

Tam uyum: kullanıcının tuşu ya da adı, şerit düğmesinin çalıştırdığı aynı komutu aynı kimlikle çalıştırır; doğrulama,
yetki, önizleme ve sonuç değişmez. Kanallar aynı komutlara kimlikleriyle zaten ulaşır.

## Doğrulama

- `cargo test -p kentos-contracts --test settings`: 13/13 (değerler, yazma, çözümleme, dosyalar, hazır ayarlar, eşlem
  türü, `chords_are_written_one_way_and_kentos_keeps_its_own_keys`, `typed_names_are_kept_in_turkish_upper_case`,
  `the_keys_and_names_settings_keep_their_entries_as_written`, üretilen dosyalar).
- `cargo test -p kentos-desktop --lib keymap_tests`: 11/11 (1 resim sınaması elle): tuş kaydı ve Kaydet'ten sonra
  çalışması (yeniden başlatmadan), çakışma ve Taşı/Vazgeç, KentOS'un ve sistemin tuşları, Varsayılana dön ve Hepsini
  varsayılana döndür, adların reddi ve taşınması, adların komut satırında, listede ve Menülerde ara'da çalışması, tuşun
  gösterildiği yerler ve harf ipuçları, modülün sunmadığı komutun tuşu, okunamayan girişin saklanması, ayar dosyasıyla
  gidiş-dönüş ve içe aktarma, bölümün çizimi.
- Kesişen masaüstü sınamaları (`module_tests`, `ribbon_tests`, `history::text`, `catalog`, `settings`, `shortcuts`,
  `alias_profile`, `other_names`, `keys::` süzgeçleri birlikte): 190 geçti, 0 kaldı.
- `make arch` ve `scripts/dev/module_deps.py --check` temiz; modül matrisi `cargo test -p kentos-cad`
  (`each_program_has_its_own_settings_sections`, `a_settings_window_opens_on_a_section_the_program_has`).
- Resimler (`keymap_tests::screens`, `.run/gui-evidence/kisayollar/`) bu dilimde çekilmedi: iş sahibin isteğiyle
  kapatıldı; sınama hazır, elle çalıştırılır.

## Açık kalanlar (sahibin kararı)

- Pafta tasarımcısının tuşları da kullanıcı haritasına bağlansın mı (`crates/sheet-ui`'nin tuş okuması değişir)?
- İki basışlı tuş istenir mi?
- Python, MCP ve ajana haritayı salt okuyan bir yol (ör. ajanın “bu komutun tuşu ne?” sorusu) gerekir mi?
