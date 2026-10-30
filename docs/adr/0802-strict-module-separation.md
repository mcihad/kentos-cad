# ADR 0802: Modüllerin sıkı ayrımı: iki CAD, her biri ayrı uygulama, modüle göre komutlar ve yüzeyler

- **Durum:** kabul edildi (2026-10-30). ADR 0801'in “Detay / imalat” işini ve “şeritte görünmeyen komut her yerden çalışır” kuralını
  kaldırır; geri kalanını korur.
- **İstek:** sahibin sözleri, sırasıyla:
  - “launcher ile çalıştırdığım zaman hep aynı uygulama çalışıyor, sanki ayrıştırma olmamış gibi”;
  - “alt ajan gerekliyse istediğin kadar çalıştır ama her biri ayrı uygulama olarak çalışacak bunu muhakkak sağlaman gerek”;
  - “dostum bu ayrıştırmayı daha sıkı şekilde yapman gerekiyor ama bazı eksikler de var, harita cad mesela oraya da cbs gibi
    altlık falan eklenebiliyor bilgin olsun, ayrıca arama kısmında ilgili modülde olmayan özellik aramada çıkmasın”;
  - “yeni proje dediğim zaman da hep aynı wizard çalışıyor, uygulamalar ayrı olacağı için wizard kısımları olmayabilir veya olursa
    farklı olmalı zaten, bizde iki tür cad var mimari ve harita 3. üretim cad gerek yok”;
  - “alt taraftaki tablo istatistik ve benzeri sekmeler var, onlar da uygulamaya göre, katmanların olduğu yerdeki sekmeler de
    uygulamaya göre, hepsinin ayrımını güzelce yap dostum mesela blok gis içinde olmasa da olur, ya da adres mimari içinde olmasa da
    olur, bu ve benzeri her şeyi iyice araştırıp test edip ondan sonra ayrımı net şekilde yapman gerekecek”.
- **Bağlam belgesi:** ADR 0800 (KentOS Launcher), 0801 (modüller ve programları), 0626 (sihirbazın işleri), 0622 (masaüstünün şeridi).
- **Web:** bir şey değişmez (ADR 0228).
- **`.kcad`:** şema 135'in `cadWork`'ü yalnız `architecture` ve `map` alır; `detail` hiç yayımlanmadı (şema 135 bu ayrımla birlikte
  gönderiliyor).

## Bağlam

ADR 0801'den sonra sahibin denemesinde üç sorun çıktı:

- **Launcher hep aynı programı açıyordu.** Modüllerin kendi programları `module-programs` özelliğiyle derleniyordu; sıradan derleme
  yalnız `kentos-cad`'i yapıyordu. Launcher modülün programını bulamayınca `kentos-cad --modul`'e düşüyordu. `target/debug`'daki
  `kentos-cad` ise eski bir derlemeydi ve `--modul`'ü tanımıyordu: her kart aynı programı açtı.
- **Ayrım gevşekti.** Şerit işe göre kesiliyordu, ama Komut ara, komut satırı ve kısayollar her komutu buluyordu. Hiçbir şeridin
  tutmadığı komutlar (İşlemler araçları, adres eşleme, ağ analizi …) her modüle açıktı. Alt panelin sekmeleri, sağ panelin
  bölmeleri, durum çubuğu, menüler ve ayar pencereleri modüle hiç bakmıyordu.
- **Yeni proje her programda aynı sihirbazdı**, ve üçüncü bir CAD işi (Detay / imalat) vardı.

İnceleme (30 Ekim): salt okunur bir envanter ajanı arayüzün modüle bağlı yüzeylerini satır satır çıkardı. Bir workflow her modül için
bir denetçi ve bir eleştirmen çalıştırdı. Denetçi modülün sunduğu komutları ve yüzeyleri tek tek değerlendirdi; eleştirmen denetçinin
aşırılıklarını ve kaçırdıklarını kodla doğrulayarak düzeltti. Bulgular: Mimari 853 komutun 656'sını sunuyordu, bunun 145'i yersizdi.
Çoğu raster bağlam sekmesinden (eğim, havza, hidroloji), CBS işlem araçlarından ve “kabuk” komutlarından sızıyordu; “havza”, “tesis”,
“adres” araması Mimari'de sonuç veriyordu. Harita'da 62, GIS'te 59 yersiz komut vardı (GIS'te bloklar, ölçü ve duvar araçları).

## Karar

### 1. Her modül ayrı bir uygulama

- `kentos-mimari`, `kentos-harita` ve `kentos-gis` her derlemede `kentos-cad`'in yanında derlenir; özellik kapısı kalktı. Kendi
  testleri yoktur: test koşusu yine tek ikili bağlar.
- Her program masaüstüne kendi uygulamasıdır: penceresinin sınıfı ve Wayland uygulama kimliği programın adıdır; Linux için her
  birinin `.desktop` kaydı `apps/desktop/packaging/linux/`'tadır (`StartupWMClass` aynı ad).
- Launcher bir modülü yalnız o modülün kendi programıyla başlatır; bulamazsa kart “Bulunamadı” der ve programın adını söyler.
  `kentos-cad` bir modülün yerine geçmez. Kullanıcı “Konumu seç…” ile `kentos-cad`'i gösterirse o kendi seçimidir: `--modul` ile
  başlar.

### 2. İki CAD işi

`CadWork` yalnız `architecture` (KentOS Mimari) ve `map` (KentOS Harita) alır. Kendi koordinatlı CAD çizimi hangi birimde olursa
olsun bir binanın, koordinat sistemindeki çizim bir haritanın işidir (sihirbaz, Python ve MCP'nin yeni çizimi, işi sorulmamış eski
çizimin tahmini aynı kuralla).

### 3. Yeni proje modüle göre

- **KentOS Mimari:** sihirbaz yok. Yeni proje metre, 1:100 ve yerel koordinatlarla hemen açılır; birim, ölçek ve sistem Proje
  ayarları'ndan değişir.
- **KentOS Harita:** “Yeni harita projesi”: tür sorulmaz, koordinat sistemi (il ve dilim) ve proje adı iki adımdır.
- **KentOS GIS:** “Yeni CBS projesi”: aynı iki adım.
- `kentos-cad`'in sihirbazı eskisi gibidir; işleri ikidir.

### 4. Modülün komutları

Bir modül programı yalnız kendi komutlarını bulur, listeler ve çalıştırır: Komut ara, komut satırı (yazılan ad ve AutoCAD adları),
kısayollar ve `App::run` aynı kümeye bakar. Başka modülün komutu ““Duvar” KentOS Harita'da yok; KentOS Mimari'de var.” cümlesiyle
reddedilir. Küme şeridin verisinden çıkar (`catalog/v1/ribbon.json`, `scripts/ui/ribbon.py`): sekmelerin, panellerin, düğmelerin,
menü bloklarının ve ▾ komutlarının `works`'ü, bağlam sekmelerinin işe göre kesilmesi, `modules` bölümü (CBS şeridinin CAD
modüllerine de verdikleri ve hiçbir şeridin tutmadığı modüle özgü komutlar). Harita, GIS'teki gibi altlıkları, harita
servislerini ve kaynakları tutar; bloklar CAD'indir. Ayrıntılı listeler ve sınırda kalan kararlar Uygulama bölümündedir.

### 5. Modülün yüzeyleri

Alt panelin sekmeleri, sağ panelin bölmeleri, durum çubuğunun hücreleri, Proje ayarları ve Uygulama ayarları'nın bölümleri, katman
menüsü, uygulama menüsünün dosya biçimleri, Kaynaklar'ın bölümleri ve sürüklenip bırakılan dosyalar modülün profilindendir
(`apps/desktop/src/module_surfaces.rs`). Paylaşılan `yerlesim.json`'da başka programın bıraktığı sekme ya da bölme açılmaz. Profil
Uygulama bölümündedir. `kentos-cad` her şeyi gösterir.

## Uygulama

- Sözleşme: `CadWork { Architecture, Map }` (`crates/shared/contracts/src/document.rs`), `kentos_launch::Module::cad_works`,
  `kentos_project::wizard::WORKS` (iki iş, yerel → mimari).
- Programlar: `apps/desktop/Cargo.toml` (özellik yok), `src/lib.rs` (uygulama kimliği), `apps/desktop/packaging/linux/*.desktop`,
  `apps/launcher/packaging/linux/kentos-launcher.desktop`; launcher `resolve.rs` (modül yalnız kendi programıyla).
- Yeni proje: `project/mod.rs` (`file.new`: Mimari'de `new_project_at_once`), `project/wizard/mod.rs` (`first_step`, başlık).
- Komutlar: `catalog.rs` (`cut`, `shows`, `module_has`, `contextual_for_work`, `Panel::commands`), `program.rs` (`offers`,
  `not_offered`), `ribbon_search.rs`, `view.rs` (komut satırının listesi), `input.rs` (kısayollar), `app.rs` (`run_typed`),
  `alias_profile.rs`; `scripts/ui/ribbon.py` (`W`, blokların ve öğelerin `works`'ü, `MODULES`).
- **Komut kümeleri** (`catalog.rs`'in `module_has`'ı; sayılar bu kararla): KentOS Mimari 656 → 514, KentOS Harita 763 → 705, KentOS
  GIS 720 → 664 komut. Şeridin tuttuğu komut, şeridi onu tutan modüllerindir. `processing.run.*` ve kullanıcının modelleri
  (`processing.model.*`) İşlem araç kutusunun (`processing.toolbox`) olduğu yerdedir. Hiçbir şeridin tutmadığı modüle özgü komutlar
  `ribbon.json`'un `modules.only`'sindedir; kalan kabuk komutları (111) her yerdedir.
  - Mimari'den çıkanlar: raster bağlam sekmesinin çözümleme panelleri (Yüzey, Raster işlemleri, Uygulamalar) ve raster görünüşü,
    CBS işlem araçları, adres, ağ, `map.*`, `analysis.*`, `layer.dataset`. Mimari'ye girenler: Alan oluştur (`tool.boundary`, AutoCAD
    BOUNDARY) ve Koordinat oku (`crs.query`, AutoCAD ID).
  - Harita'da kalanlar: blok kitaplığı, uzaktan algılamanın Görüntü birleştirme'si ve STAC, hidroloji ile uzaklık ve maliyet
    (altyapı işi). Harita'ya giren: Kuzey oku (Açıklama › İşaretleme).
  - GIS'te kalan: kayıtlı ölçüler (COGO). GIS'e giren: Veri kümesi bilgisi. GIS'ten çıkanlar: bloklar, ölçü, kılavuz, tarama, tablo
    ve resmin bağlam sekmeleri, duvar ve CAD çizim araçları. Ağ analizinin üç İşlemler aracı yalnız GIS'tedir.
  - Kendi kararlarım (sahibin sözüyle değiştirilebilir): `crs.set` Mimari'de durur (durum çubuğunun ve sekme satırının sistem
    düğmesi onu çalıştırır), çizim standardı Harita ve GIS'te de durur, `view.properties` her yerdedir, `workspace.*` yalnız
    kentos-cad'dedir. Denetçilerin önerip eleştirmenlerin reddettikleri eklenmedi: Mimari'de `file.export.gis`, Harita'da
    revizyon işareti, GIS'te Çizimi temizle.
- **Girişler:** `App::run` (başka modülün komutunu `not_offered`'ın cümlesiyle reddeder), `available` ve `why_disabled`, hızlı erişim
  çubuğu (eklemek de), F1 Kısayollar, çizimin menüleri, Yardım menüsü (“KentOS Harita hakkında”), bulut hücreleri, İşlemler paneli
  (araçlar, modeller, Yeni model), model tasarımcısının paleti, Komut ara'nın modelleri, kullanıcının modelleri (şeridin model
  düğmeleri, komut satırı, yazılan ad), Seçim süzgeci (gösterilen şeridin menüsünden: mimari türler yalnız Mimari'de), dar durum
  çubuğunun Yardımcılar'ı, nokta hesaplayıcı (Mimari'de Km ve sapma ile Nokta adından yok), Başka çizimden al (GIS'te Bloklar ve
  ölçü stilleri yok).
- **Yüzeyler** (`module_surfaces.rs`, `App::shows_*`):
  - **Alt panel:** Mimari'de Komut geçmişi, Koordinat listesi, Arama (Veride ara binanın yazılarını da bulur), Denetim, Uyarılar,
    Python. Harita'da Grafik ve Denetim dışındaki hepsi. GIS'te Denetim dışındaki hepsi. Gizli sekmenin açıcıları Komut geçmişi'ni
    açar ve sekmenin hangi programda olduğunu söyler. Harita'da İstatistik'in, tablonun İncele ▾'sinin ve İşlemler sonucunun Grafik'i
    yoktur.
  - **Sağ panel:** Mimari'de Katmanlar, Öznitelikler, Bloklar, Şablonlar. Harita'da altısı. GIS'te Bloklar dışındakiler. F4 programın
    bölmelerini alır; başka yoldan açılan gizli bölme kapatılır.
  - **Durum çubuğu:** Mimari'de Kat var; Harita sistemi, İkinci sistem, Topoloji, Çakışma ve Senaryo yok. Harita'da Harita sistemi,
    İkinci sistem, Topoloji, Çakışma. GIS'te bunlar ve Senaryo. Koordinat sistemi menüsünün İkinci sistem ▸ ve Harita sistemi ▸'si
    Harita ve GIS'te.
  - **Proje ayarları:** Mimari'de Genel, Birim ve ölçek, Koordinat sistemi, Değişkenler, Mimari. Harita ve GIS'te Genel, Birim ve
    ölçek, Koordinat sistemi, Kotlar ve epok, Ölçme, Değişkenler.
  - **Uygulama ayarları:** Coğrafi koordinatlar ve Çizilen nesnenin formu Harita ve GIS'te. Yeni projeler Mimari'de yok, Harita ve
    GIS'te türsüz.
  - **Katman menüsü:** Zaman ayarları, Senaryo ▸ ve Katman türü ▸ GIS'te. Koordinat sistemi ▸, Stil yükle ve kaydet, SLD, QML,
    MapLibre, Veri kümesi bilgisi, servis ve kaynak öğeleri Harita ve GIS'te.
  - **Uygulama menüsü:** Mimari'de yalnız çizim biçimleri (DWG, DXF, NCZ, PDF). Başlık, not ve Hakkında programın adıyla.
  - **Kaynaklar:** Veritabanları ve Servisler Harita ve GIS'te. Mimari'ye bırakılan çizim kendi sekmesinde açılır, PDF içe aktarılır,
    başka dosya hangi programda açıldığı söylenerek reddedilir.
  - **Diğer:** Öznitelikler'in sistem ve servis satırları, üzerine gelme kartının parsel satırları Harita ve GIS'te. Pafta kipi
    şablonlarını programın türüne göre sıralar. Bulutun tür süzgeci ve yükleme türü programdan başlar.
- **Yerleşim:** her programın kendi dosyası: kentos-cad `yerlesim.json`, modüller `yerlesim-mimari.json`, `yerlesim-harita.json`,
  `yerlesim-cbs.json` (`layout::file_name`). Bir modül programı ilk açılışta kentos-cad'in boyutlarıyla başlar, yalnız kendi
  dosyasına yazar. Yerleşimin `bottomTab` kuralı Denetim ve Python'u da alır.
- **Ortak fikstürler:** `fixtures/shell/v1/ribbon.json`'da CBS'nin seçim sekmelerinden CAD'inkiler (Ölçü, Kılavuz, Tarama, Blok,
  Tablo, Resim) çıktı; `fixtures/shell/v1/layout.json` (`layout_cases.py`). Web dönünce uyar (TODOS.md §0.2).

### Açık kalanlar

- **Araçların seçenekleri:** GIS'te Böl'ün Blok seçeneği. Araç oturumu programı bilmez; seçenek düzeyinde ayrım `Context`'e bir
  yetenek alanı ister.
- **Yalnız Mimari'de anlamsız görünüşler:** Stil yöneticisinin MPYY dalı, Katman stili'nin tematik işleyicileri, Karelaj'ın Y/X adları.
- **Ajan bağlantısı:** her program aynı soketi açar (`masaustu.sock`). Aynı anda ikincisi açamaz ve nedenini söyler. Program başına
  soket ve MCP'nin `desktop.attach`'ında program seçimi açıktır.
- **Pafta profilleri:** iş başına satır yoktur; pafta araçları CAD ve CBS'ye göredir.
- Her programın yalnız kendi kodunu taşıması (özellik kapılarıyla kırpma), paketleme ve modül başına sürüm (`OPS-10`), gerçek
  pencerede fare ve klavye kabulü.

## Doğrulama

- `module_tests.rs`: modülün sunduğu ve sunmadığı komutlar, Komut ara, komut satırı ve kısayollar, Mimari'nin sihirbazsız yeni
  projesi, Harita'nın türsüz sihirbazı. Ayrıca `run`'ın reddi, hızlı erişim ve F1, İşlemler'in araçları ve modelleri. `kentos-launcher`'ın
  testleri: modül yalnız kendi programıyla.
- `module_surface_tests.rs`: her programın alt sekmeleri, bölmeleri, durum hücreleri, ayar bölümleri, katman menüsü, dosya
  biçimleri, Kaynaklar'ı, bırakılan dosyalar. Başka programın sekmesi ve bölmesi açılmaz; Mimari'nin nokta hesaplayıcısı. Bir
  çapraz denetim, sunulan her açıcının yüzeyini gösterdiğini sınar. Diğer testler: `selection_commands.rs` (Seçim süzgecinin
  türleri programa göre), `layout_tests.rs` (her programın yerleşim dosyası), `architecture_ribbon.rs` (dar Yardımcılar),
  `drawing_exchange.rs` (GIS'te blok alınmaz), sihirbazın üç testi (iki iş).
- Koşulanlar (30 Ekim): `cargo test -p kentos-desktop --lib` 1433 geçti, 0 düştü. `kentos-launcher`, `kentos-launch`,
  `kentos-project`, `kentos-contracts`, `kentos-kcad` ve `kentos-commands` testleri geçti. Clippy (`-D warnings`) masaüstü ve
  ortak crate'lerde temiz. `cad.settings.set`'in ortak durumları ve başsız sunucunun `cad_open`'ı da geçti. Ayrıca şunlar güncel:
  `ribbon.py --check`, `manifest.py --check`, `layout_cases.py --check`, `sdk.py --check`, `kcad_v2_reference.py --check`,
  `settings_set_command_cases.py --check`.
- Resimler: `module_surface_tests::pictures` (`.run/shots/ayrim-*`), her program 1440 × 900, koyu ve açık.
- **Koşulmadı:** `pnpm py:test` (diskte 5 GB kaldığı için; Python katmanı `sdk.py --check` ile güncel), gerçek pencerede fare ve
  klavye ile dört programın aynı anda açılması.
