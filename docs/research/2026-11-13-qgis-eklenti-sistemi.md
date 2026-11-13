# QGIS eklenti sistemi: `.kek` için inceleme ve dersler

Tarih: 13 Kasım 2026. Konu: Sahip, KentOS'un `.kek` eklenti sisteminin QGIS'inki kadar güçlü ve sağlıklı olmasını istiyor ("örneğimiz orası"). Bu belge QGIS'in eklenti sistemini resmî belgelerden, kaynak kodundan, eklenti sitesinin kodundan, QEP'lerden ve geliştirici listelerinden inceler; ekosistemi neyin büyüttüğünü ve neyin yıprattığını ayırır; her alan için [`plan/eklenti.md`](../../plan/eklenti.md)'nin karşılığını ve somut bir öneri yazar. Plan değiştirilmedi; öneriler sahibin kararına sunulur.

## Yöntem ve sınırlar

- Bütün kaynaklar 13 Kasım 2026'da okundu. Kaynak kısaltmaları ([D1], [K4], [W9] …) en sondaki listededir.
- Kod okumaları `qgis/QGIS` ve `qgis/QGIS-Plugins-Website` depolarının `master` dalındandır. Okunan dosyaların son commit tarihleri: `src/app/qgspluginregistry.cpp` 17 Eylül 2026; `python/pyplugin_installer/*` ve `qgspluginmanager.cpp` 5 Kasım 2026; sitenin `validator.py`'si 1 Ekim 2026, `views.py`'si 7 Ekim 2026, `security_scanner.py`'si 24 Haziran 2026.
- **[H*] ile işaretli sayılar benim hesabımdır**, sitenin yayımladığı istatistik değildir: `https://plugins.qgis.org/plugins/plugins.xml?qgis=3.44` ve `?qgis=4.0` dosyaları 13 Kasım 2026'da indirildi (sunucu başlığı `last-modified: Fri, 13 Nov 2026 11:57:30 GMT`) ve Python'la sayıldı. Bu XML yalnız o QGIS sürümüyle uyumlu, onaylı sürümleri listeler; uyumsuz (ör. yalnız QGIS 2) eklentiler sayıma girmez.
- E-posta listesi iletileri ve bazı belge sayfaları özetleyici okuma aracıyla okundu; alıntılar o okumadandır. Kaynak kodları ve QEP'ler ham metinden okundu.
- `blog.qgis.org`'un 2025 tarihli Qt6/QGIS 4 yazıları ve 6 Temmuz 2026 tarihli site v4.0.0 yazısı 404 döndü; `hub.qgis.org` erişilemedi. Bunlara dayanan bilgi yazılmadı.
- Öneri etiketleri: **Al** (QGIS'teki gibi al), **Uyarla** (fikri al, biçimini KentOS'a göre değiştir), **Kaçın** (QGIS'in bedel ödediği yol).

## Özet

- **QGIS'i büyüten formül basit:** bir klasör, içinde `__init__.py` ve `metadata.txt` [D1]; uygulamanın içinde önceden tanımlı resmî depo, tek tıkla kurma ve açılışta güncelleme bildirimi [K4][D5]; PyQGIS ve Qt'nin bütün gücü. Sonuç: QGIS 3.44 kataloğunda 4.461 kayıt / 3.927 paket, paketlerin toplam indirme sayılarının toplamı yaklaşık 103 milyon [H1]. Büyüme hızlanıyor: bugün listelenen paketlerin 587'si 2025'te, 1.567'si 2026'da (13 Kasım'a kadar) açıldı [H3].
- **QGIS'in kronik dertleri aynı mimari kökten geliyor:** tek süreç, tek Python yorumlayıcısı ve kodla kurulan Qt arayüzü. Temizlik yazarın `unload()`'ına bağlı ve unutulduğunda QGIS çöküyor [K9]; çökme ancak sonradan bir bekçiyle (watchdog) yakalanıyor [K2]; bağımlılık çatışması 2020'den beri çözülemedi [Q6][A1][L5]; açılış yavaşlığının baş şüphelisi eklentiler [K12]; QGIS 4'ün (Qt6) çıkışından 8 ay sonra paketlerin yalnız %42'si yeni sürümde [H2].
- **KentOS planı bu kökleri tasarımla kesiyor:** `ExtensionLease`, eklenti sürümü başına ayrı süreç, kilitli uv ortamı, bildirimsel `kentos.ui`, `integrity.json` ve `requestedCapabilities`. QGIS'in deneyimi bu kararları doğruluyor; hiçbiri "kolaylık" için gevşetilmemeli.
- **QGIS'in önde olduğu, planda eksik ya da zayıf olanlar:** keşif üst verisi (uzun açıklama, etiket, ana sayfa/kaynak/sorun izleyici bağlantıları, değişiklik notu), deneysel kanal, eklentiler arası bağımlılık, uygulama sürümüne göre uyumluluk aralığı, çökme bekçisi, eklenti başına açılış süresi görünümü, yerelleştirilmiş üst veri, birden çok ve çevrimdışı katalog kaynağı, emeklilik ve bakıcı devri yolu, sürüm göçü denetleyicisi ve **veri kaynağı/altlık harita katkı noktası** (QGIS'te en çok indirilen 15 eklentinin en az 5'i bu türden [H4]).
- **En acil öneri ADIM 16 için:** manifest bilinmeyen alanı reddediyor ve sözlüğü genişletmek sürüm artışı sayılıyor (plan §3.2). Keşif, bağlantı, bağımlılık ve kanal alanları isteğe bağlı olarak v1'e şimdi girmezse ilk yayın döneminde `schemaVersion: 2` kaçınılmaz olur.
- **Depo yönetişimi 2026'da hızla değişti** ve 22A'ya hazır dersler bıraktı: engelleyici otomatik tarama (Bandit, detect-secrets) [W4][Q1], geriye dönük herkese açık "güvenlik sorunu" rozetine tepki [L4], oy istismarı ve oylamanın kaldırılması [L3][K14], çalışma anında bağımlılık indiren eklentilerin yayından kaldırılması [L5], AI ile üretilmiş eklentiler ve LLM beyanı tartışması [L10][L11].

---

## 1. Eklentinin anatomisi ve yaşam döngüsü

### QGIS ne yapıyor

**Dosyalar ve kimlik.** Eklenti, profilin `python/plugins` klasöründeki bir Python paketidir [D5]. Zorunlu dosyalar `metadata.txt` (UTF-8) ve `__init__.py`'dir; `__init__.py` bir `classFactory(iface)` fonksiyonu tanımlar ve ana sınıfın örneğini döndürür. Ana sınıfta yalnız üç yöntem zorunludur: `__init__` (arayüz nesnesini alır), `initGui()` (yüklenirken), `unload()` (kaldırılırken). `form.ui`, `resources.qrc` gibi dosyalar isteğe bağlıdır; `LICENSE` yalnız resmî depoya yükleme için zorunludur [D1]. Eklentinin kalıcı kimliği klasör adıdır (`package_name`): sitede benzersizdir ve değiştirilemez, görünen `name` ise değişebilir [W9][D6]. `metadata.txt` alanlarının tamamı §9'daki eşleme tablosundadır.

**Açılışta yükleme: hepsi, hemen, sırayla.** QGIS açılışta bütün Python eklentilerini bulur, `metadata.txt`'deki `plugin_dependencies`'e göre sıralar (`updateAvailablePlugins(sort_by_dependencies=True)`) ve önceki oturumda etkin olan her birini sırayla içe aktarır, `classFactory(iface)` ile örnekler ve `initGui()`'yi çağırır [K11][K1][K2]. Tembel (ilk kullanımda) yükleme yoktur. Önce `name`/`description`/`version` okunabilir mi ve `qgisMinimumVersion`–`qgisMaximumVersion` aralığı tutuyor mu bakılır; tutmayan eklenti "is not compatible with this version of QGIS. It will be disabled." diye günlüğe yazılıp devre dışı bırakılır [K2]. Her eklentinin süresi `time.process_time()` ile ölçülüp `qgis.utils.plugin_times`'a yazılır [K1]; `QgsScopedRuntimeProfile` ile Geliştirme/Hata Ayıklama panelinin başlangıç profiline girer [K2]. Bu paneli ekleyen commit'in gerekçesi: *"QGIS startup times have been an ongoing issue for a number of years, so let's help provide users with some tools to help track down what's causing these (\*hint\* it's probably a plugin \*hint\*)"* (Nyall Dawson, 18 Mayıs 2020) [K12]. Komut satırında `--noplugins` eklentileri, `--nopython` Python'u bütünüyle kapatır [K10].

**Yükleme hatası ve çökme.** İçe aktarma, `classFactory()` ya da `initGui()` Python istisnası atarsa QGIS çökmez: mesaj çubuğunda "Couldn't load plugin '…' due to an error when calling its initGui() method" ve "Stack trace" / "View message log" düğmeleri çıkar, iz günlüğe düşer [K1]. Ancak `loadPythonPlugin` başarıyı denetlemeden eklentiyi etkin olarak kaydeder (kodda `// TODO: test success`); bozuk eklenti her açılışta aynı hatayı yeniden verir [K2]. Yerel kod çökmesine (segfault) karşı **bekçi** var: yüklemeden hemen önce `PythonPlugins/watchDogTimestamp/<paket>` ayarı yazılır, yükleme bitince silinir; açılışta 5 saniyeden eski bir damga kalmışsa QGIS önceki açılışın o eklentide çöktüğünü varsayar, eklentiyi devre dışı bırakır ve "This plugin is disabled because it previously crashed QGIS." iletisini "Enable Plugin" / "Ignore" düğmeleriyle gösterir [K2]. Mekanizma en geç 2016'dan beri var; 2020'de ileti ve düğmeler, 2024'te zaman damgası eklendi [K13].

**Kaldırma ve yeniden yükleme.** `unloadPlugin`, eklentinin `unload()`'ını çağırır; sonra eklenti nesnesini siler, içe aktarma kancasının eklentiye ait diye izlediği bütün modülleri `sys.modules`'tan atar, Qt kaynakları için `qCleanupResources()` çağırır, `sys.path` girdilerini kaldırır ve (Mayıs 2026'dan beri) eklentiye verilen ayar ağacı düğümünün (`SETTINGS_NODE`) kaydını siler [K1][K20]. `reloadPlugin` = kaldır + içe aktar + başlat [K1]. Geliştiriciler bunu "Plugin Reloader" eklentisiyle kullanır (330.517 indirme, en çok indirilen 44. eklenti) [D1][H1].

**Temizlik yazarın işi.** `QgisInterface`, eklentinin arayüze dokunduğu yolları sunar: `addToolBarIcon`, `addPluginToMenu` (ve Raster/Vector/Database/Web/Mesh/Processing menü çeşitleri), `addDockWidget`, `registerLocatorFilter`, `registerOptionsWidgetFactory`, `registerMapLayerConfigWidgetFactory`, `registerCustomDropHandler` …; çoğunun `remove*`/`unregister*`/`deregister*` karşılığı vardır ve bunları `unload()`'da çağırmak yazarın sorumluluğudur [K9][D1]. `addToolBar`'ın kaldırma karşılığı yoktur; araç çubuğunu yazar kendisi siler [K9]. Arayüz belgesi sonucu açıkça yazıyor: *"Plugins which register filters to the locator bar must take care to correctly call deregisterLocatorFilter() and deregister their filters upon plugin unload to avoid crashes."* [K9]. QGIS'in kendi kaldırma kodunda menü, araç çubuğu, dock, harita aracı, İşlemler sağlayıcısı ya da sinyal bağlantısını geri alan bir adım yoktur [K1].

**Python–C++ bellek sınırı.** QEP 427 (Temmuz 2026), QGIS 2'den kalan Python tabanlı Processing arayüz API'sini kaldırırken gerekçeyi şöyle koyuyor: C++ ve Python nesne ömürlerinin karışması *"unpredictable object deletion or memory leakage"* üretiyor ve düzeltilemiyor; doğru model *"Python is used **solely** to subclass C++ base classes, with all memory management living at the C++ level"* [Q4].

**İşlemler (Processing) sağlayıcısı.** Eklenti `initProcessing()` içinde `QgsApplication.processingRegistry().addProvider(...)` çağırır, `unload()`'da `removeProvider(...)`; `metadata.txt`'ye `hasProcessingProvider=yes` yazılır [D3]. Başsız `qgis_process` aracı yalnız etkin ve `hasProcessingProvider` bildiren eklentileri yükler ve `initGui()` yerine yalnız `initProcessing()`'i çağırır; bayrağı olmayan eklentiyi etkinleştirirken "Plugin does not report having a Processing provider" uyarısı verir [K8][K1]. Yani başsız çalışabilirlik gönüllüdür: bayrağı ve `initProcessing()`'i olmayan eklentinin algoritmaları yalnız masaüstünde vardır.

**Sunucu ve C++ eklentileri.** QGIS Server eklentisi `serverClassFactory(serverIface)` sunar ve `server=True` taşır; filtre, erişim denetimi, önbellek, servis ve OGC API kaydedebilir; sunucu açılışında bir kez yüklenir [D4][K1]. C++ eklentisi paylaşılan kütüphanedir ve `name`, `description`, `category`, `version` sembollerini dışa vermelidir [K2]; C++ API kararlılık güvencesinin dışındadır (QEP 425 §1.2.1) [Q2]. Dış eklentilerin çoğu Python'dur [D5].

### KentOS planı ne diyor

Saf fonksiyonlar, `handler: "modül:fonksiyon"` dizgisiyle içe aktarmadan bilinen katkılar, `unload` yerine `ExtensionLease` ile tek çağrıda temizlik (§1.1, §3.2, ADIM 17); eklenti süreçleri ayrı, bütün eklentileri aynı süreçte içe aktarmak varsayılan değil (§1.3, ADIM 19); kod yürütme tembel başlar (PY-RUN-05); çökme uygulamayı kapatmaz (PY-RUN-13); araç tanımı çalışma zamanlarını bildirir (`capabilities.runtimes`, §3.3).

### Öneri

| Konu | Karar | Adım | Gerekçe ve somut iş |
|---|---|---|---|
| Kayıt temizliği | **Kaçın** (QGIS yolu) — plan zaten kiralama | 17, 22 | QGIS'in locator uyarısı, yazara bırakılan temizliğin çökme ürettiğinin resmî kabulüdür. Kabulde "etkinleştir → devre dışı → yeniden etkinleştir" sonrası kayıt defterinde o kiralamaya ait sıfır öğe testi korunmalı. |
| Çökme bekçisi | **Al** | 17 | Python ayrı süreçte olsa da eklentinin JSON'unu çizen host kodu (şerit, panel, form) bir eklenti yüzünden düşebilir. Etkinleştirme sırasında `(id, sürüm)` için bekçi kaydı; bir sonraki açılışta kayıt duruyorsa eklenti nedenli "hata" durumunda açılır, "Yeniden etkinleştir / Kapalı bırak" sorulur. Ayrı süreç N kez art arda çökerse aynı durum. |
| Başarısız yükleme | **Kaçın** (`TODO: test success`) | 17 | Etkinleştirme başarısızsa durum makinesi "hata"da kalır, her açılışta yeniden denenmez; neden ve günlük Eklentiler sekmesinde görünür. |
| Açılış maliyeti | **Kaçın** (hepsini açılışta yükleme) | 17, 19 | Açılışta yalnız manifest okunur ve katkılar kaydedilir; Python süreci ilk çağrıda başlar (PY-RUN-05). Eklenti başına "kayıt süresi / ilk çağrı süresi" ölçülüp gösterilir (QGIS'in hata ayıklama paneli örneği). |
| Başsız eşdeğerlik | **Uyarla** | 19, 21 | QGIS'te gönüllü (`hasProcessingProvider` + `initProcessing()`). KentOS'ta her İşlemler aracı `capabilities.runtimes` bildirsin; verify Kapı 3 başsız kuru koşuyu zorunlu yapsın; yalnız masaüstünde çalışan araç katalogda öyle görünsün. |
| Bellek sahipliği | **Al** (QEP 427 dersi) | 19, 22 | Python hiçbir UI ya da belge nesnesine sahip olmaz; plan PY-RUN-07 ve PY-UI-14 ile aynı. Kod incelemesinde "Python'a nesne işaretçisi vermeyelim" gerekçesi olarak QEP 427 anılabilir. |
| Eklenti ayar ad alanı | **Al** | 17 | QGIS 2026'da her eklentiye host'un yönettiği ayar düğümü vermeye başladı; plan PY-RUN-18 ile aynı yönde. |
| Yeniden yükleme | **Al** | 21 | Plugin Reloader'ın 330 bin indirmesi, geliştirme döngüsünün en çok kullanılan aracı olduğunu gösteriyor; `extension.hot_reload` aynı ihtiyaç. |

---

## 2. Eklenti Yöneticisi (`pyplugin_installer`)

### QGIS ne yapıyor

**Sekmeler ve işlemler.** Yönetici "Plugins > Manage and Install Plugins" menüsündedir. Sekmeler: All, Installed, Not installed, New (son denetimden beri gelenler), Upgradeable, Invalid ("broken for any reason"), Install from ZIP, Settings [D5]. Arama ad, yazar, açıklama ve etikete bakar; `tag:` önekiyle yalnız etikette arar [D5][K7]. İşlemler: kur, deneysel sürümü kur, yeniden kur, yükselt, "Upgrade All", deneyselden kararlıya düşür, kaldır; onay kutusu eklentiyi geçici olarak açıp kapatır [D5]. Sorunlu eklentiler kırmızıdır ve Invalid sekmesine düşer; hata sınıfları `broken`, `incompatible`, `dependent` [K7][K4].

**Kurma = hemen çalıştırma.** Depodan ya da ZIP'ten kurulan eklenti kurulumun hemen ardından içe aktarılır ve başlatılır [K3]. Kurulu ama hiçbir depoda olmayan eklenti `orphan` sayılır; kaldırırken "this plugin isn't available in any accessible repository!" uyarısı çıkar [K4][K3]. Kullanıcı klasöründeki eski bir kopya QGIS ile gelen daha yeni çekirdek eklentiyi maskeliyorsa yönetici bunu söyleyip kaldırmayı önerir [K3].

**Yükseltme atomik değil.** İndirilen zip geçici klasöre açılır, **eski klasör silinir**, sonra yenisi kopyalanır; kopyalama başarısız olursa önceki sürüme dönüş yoktur [K6]. Önceki sürüm saklanmaz; "eski sürüme dönme" yalnız depoda sunulan sürümlere ve deneysel/kararlı ayrımına dayanır [D5][K3]. Kaldırma eklenti klasörünü ve etkinlik ayarını siler; eklentinin kendi ayar ya da veri kayıtlarını temizleyen bir adım kodda yok [K3].

**Deneysel ve kullanımdan kaldırılmış.** `allow-experimental` ve `allow-deprecated` ayarları varsayılan kapalıdır [K7]. Deneysel sürüm ayrı düğmeyle kurulur; kullanımdan kaldırılmış eklentiler gri görünür ve sıralamada en sona itilir [D5][K7].

**Güncelleme denetimi.** Varsayılan açık; açılışta en çok 3 günde bir (`CHECK_ON_START_INTERVAL = 3`) [K4][K7][D5]. "New" sekmesi görülmüş eklentilerin listesini (`seen-plugins`) tutar [K7].

**Depolar.** Resmî depo `https://plugins.qgis.org/plugins/plugins.xml` önceden tanımlıdır; listeden silinse bile yönetici açılışta onu yeniden ekler, ancak devre dışı bırakılabilir [K4]. Ek depo bir ad ve `http(s)://` ya da `file://` adresiyle eklenir; özel depolar basic ya da PKI kimlik doğrulaması (`authcfg`) kullanabilir [D5][K6]. Her istek `?qgis=<ana.ara>` parametresi taşır; site o sürüme uyumlu kayıtları döndürür [K4][W9]. Biçim: `<plugins>` altında `<pyqgis_plugin name="" version="" plugin_id="">` ve alt öğeler `description`, `about`, `author_name`, `homepage`, `download_url`, `file_name`, `icon`, `category`, `tags`, `changelog`, `author_email`, `tracker`, `repository`, `downloads`, `create_date`, `update_date`, `experimental`, `deprecated`, `trusted`, `plugin_dependencies`, `qgis_minimum_version`, `qgis_maximum_version`, `disabled` [K4]. Resmî XML bunlara ek olarak `uploaded_by`, `average_vote`, `rating_votes`, `external_dependencies`, `server` taşıyor; buna karşılık `changelog`, `author_email`, `category` ve `plugin_dependencies` öğelerini hiç taşımıyor. Yani resmî depodan gelen bir eklentinin değişiklik notu ve bağımlılıkları, kurulmadan ya da yükseltilmeden önce yöneticide görünmüyor; yönetici bunları ancak kurulu eklentinin yerel `metadata.txt`'sinden okuyabiliyor [H1][K4]. Boyut: 3.44 için 9,6 MB, gzip ile aktarılan 2,09 MB [W6]. Kurumlar için diğer yollar: `QGIS_PLUGINPATH` ortam değişkeni en yüksek öncelikli ek eklenti klasörlerini verir [K11]; `QGIS_GLOBAL_SETTINGS_FILE` ile ayarlar dağıtılır [D8]; güvenlik sayfası hassas ortamlara iç depo kurmayı ve genel depoya erişimi kapatmayı önerir [L15].

**Depolar arasında ad alanı yok.** Yönetici farklı depolardan gelen kayıtları zip adının köküyle (`id`) birleştirir; depo adı kimliğin parçası değildir ve sonradan işlenen kaydın üst verisi ile indirme adresi öncekinin yerine geçebilir (kod okuması) [K4].

**Eklentiler arası bağımlılık.** `plugin_dependencies` QGIS 3.8'den beri var: diğer eklentilerin **görünen `name`'iyle** virgüllü liste, yalnız `==` ile tam sürüm sabitleme (`MyOtherPlugin==1.12,YetAnotherPlugin`) [D1][K1]. Çözüm kurulumdan **sonra**, kurulan eklentinin yerel `metadata.txt`'sinden okunur; bir diyalog kurulacak ve yükseltilecekleri sorar; aralık, çakışma çözümü ya da ters bağımlılık denetimi yoktur [K5][K3]. Resmî XML bu alanı taşımadığı için katalogda bağımlılık kurulumdan önce görünmez [H1].

**ZIP'ten kurma.** Şifreli zip desteklenir [D5][K3]. Kurmadan önce uyarı: *"Installing a plugin from an untrusted source can harm your computer. Only continue if you received the plugin from a source you trust. Continue?"* — "Don't show this again." kutusuyla kapatılabilir [K7].

**Oylar kaldırıldı.** Masaüstünden oy verme 2014'te eklendi [K14]. PSC 8 Eylül 2026'da oylamanın hem siteden hem masaüstünden kaldırılmasına karar verdi; masaüstünden kaldırma 5 Kasım 2026'da birleşti (PR #67508), artık yalnız indirme sayısı görünüyor [K14]. Gerekçe Ağustos 2026'daki oy istismarı (bkz. §7) [L3]. 13 Kasım XML'inde hiçbir kaydın `average_vote` değeri sıfırdan farklı değil [H1].

### KentOS planı ne diyor

ADIM 18: Eklentiler sekmesinde ".kek paketi incele / yükle", izin ve bağımlılık özeti, etkinleştir/devre dışı/kaldır, güncelleme ve geri dönüş; `.kek`'e çift tıklamak yalnız incelemeyi açar (PY-KEK-15). ADIM 17: inceleme → bağımlılık/izin/uyumluluk → kullanıcı seçimi → hazırlık alanına kurulum → etkinleştirme; güncelleme ayrı hazırlanır ve atomik geçer, başarısızsa önceki sürüm hazır kalır (PY-KEK-13, 14). 22A: kişisel, kiracı ve genel katalog kapsamları (PY-CLOUD-02); keşif sayfalı (ECO-API-07).

### Öneri

| Konu | Karar | Adım | Somut iş |
|---|---|---|---|
| Sekme yapısı ve arama | **Al** | 18 | Kurulu / Bulunabilir / Güncellenebilir / Yeni / Sorunlu ayrımı ve etiket süzgeci. QGIS'in "kırmızı + Sorunlu sekmesi + nedeni" deseni planın "nedenini söyler" ilkesine uyuyor. |
| Değişiklik notu kurmadan önce | **Uyarla** | 18, 22A | QGIS'in depo biçimi `changelog`'u taşıyabiliyor ama resmî katalog göndermiyor; kullanıcı yükseltmeyi neyin değiştiğini görmeden yapıyor. KentOS'ta inceleme ve güncelleme ekranı değişiklik notunu ve yetenek/bağımlılık farkını (PY-CLOUD-14) birlikte göstersin. |
| Kurunca çalıştırma | **Kaçın** | 17 | QGIS kurduğu kodu hemen başlatıyor. Planın "inceleme ≠ etkinleştirme" ayrımı korunmalı. |
| Yıkıcı güncelleme | **Kaçın** (sil-sonra-kopyala) | 17 | Planın atomik geçişi doğru; kabulde "kopyalama yarıda kesildi, eski sürüm hâlâ çalışıyor" ve "eski sürüm yerelde saklı, depoya muhtaç olmadan geri dönülüyor" testleri olsun. |
| Kaldırınca veri | **Kaçın** (QGIS ayarları bırakıyor) | 17 | Plan PY-KEK-16 ve PY-RUN-18 doğru; kaldırma diyaloğu silinecek ayar/veri kapsamını göstersin. |
| Birden çok katalog kaynağı | **Uyarla** | 18, 22A | Resmî bulut, kiracı kataloğu ve **çevrimdışı statik katalog** (iç ağ ya da USB'deki dizin + onaylı paketler; QGIS'in `file://` deposu karşılığı). QGIS'in ad alanı boşluğuna düşmemek için: kimlik `id`'dir, kaynak sabitlenir; aynı `id` iki kaynaktan gelirse sessiz birleştirme yok, kullanıcı ya da kurum politikası seçer (Python dizinleri için PY-UV-26'nın eklenti karşılığı). |
| Kurum dağıtımı | **Uyarla** | 18A, 22A | QGIS'teki `QGIS_PLUGINPATH` + genel ayar dosyası + "genel depoyu kapat" önerisinin karşılığı: kurum politikası izinli katalog kaynaklarını ve önceden kurulu paketleri belirler (PY-OPT-12). |
| Güncelleme denetimi | **Al** | 18, 22A | Varsayılan açık, seyrek (QGIS: 3 gün), kapatılabilir; tek büyük XML yerine sayfalı ve artımlı katalog (QGIS'in 3.44 kataloğu 9,6 MB). |
| Dosyadan kurma uyarısı | **Uyarla** | 17, 18 | Uyarı kalsın ama "bir daha gösterme" yerine güven durumu (yerel / kurum / onaylı genel, PY-KEK-12) her zaman görünsün; uyarıyı kapatma yetkisi kurum politikasında olsun. |
| Yıldız oyları | **Kaçın** | 22A | QGIS oylamayı istismar yüzünden kaldırdı. Etkin kurulum sayısı, sürüm geçmişi ve kimlikli "kötü sürüm bildirimi" (PY-CLOUD-16) yeterli. |
| Bağımlılığın kurmadan önce görünmesi | **Al** | 17, 18 | QGIS bağımlılığı kurulumdan sonra keşfediyor. KentOS'ta eklenti bağımlılıkları da (aşağıda §9) manifestten, kod çalıştırmadan, inceleme ekranında görünmeli. |

---

## 3. Resmî depo (plugins.qgis.org)

### QGIS ne yapıyor

**Hesap ve yükleme yolları.** OSGeo ID ile giriş [D6]. Yükleme web formuyla, eklentiye özgü belirteçle (REST) ya da XML-RPC `plugin.upload` ile yapılır [W3][W9].

**Bugünkü akış (2026'da kademeli kuruldu: zorunlu denetimler Nisan 2026 blog yazısıyla [L1], otomatik onay değişiklikleri site v4.0.0 duyurusuyla [L2]).** Her yeni sürüm `approved=False` ve `validating` durumunda başlar; tarama arka planda koşar; yükleyiciye önce alındı, sonra sonuç e-postası gider. Kritik bulgu varsa sürüm `blocked` olur: indirilemez, onaya sunulamaz; düzeltilmiş **yeni** bir sürüm yüklenmelidir. Temizse sürüm insan onayına gider; güvenilir kullanıcı isterse varsayılan kapalı "Publish immediately after security scan passes" seçeneğiyle doğrudan yayımlar [W3][W4][W9][Q1]. Durumlar: `pending`, `validating`, `validated`, `validated_with_config`, `blocked` [W9]. Site v4.0.0 duyurusu 6 Temmuz 2026'da yapıldı, yayın 13 Temmuz için planlandı [L2]; sitenin bugünkü sürümü 4.4.4 [W1].

**Kim onaylar.** Staff bütün sürümleri onaylar ya da onayı kaldırır; `plugins.can_approve` izni olan ("güvenilir") kullanıcılar yalnız sahibi oldukları eklentilerin sürümlerini onaylayabilir; izni olmayanın yüklediği sürüm onaysız başlar [D6][W9]. Sitedeki `trusted` bilgisi, eklentiyi oluşturanın bu izne sahip olup olmadığıdır, kod denetimi değildir (`created_by.has_perm("plugins.can_approve")`) [W9]. Onaylar gönüllülerce yapılır; hedef her gündür, hafta sonu yapılan yüklemeler büyük olasılıkla pazartesi onaylanır [W3]. Güvenilir statünün nasıl alınacağı belgelerde yetersiz (PSC üyesi Tim Sutton, 27 Ağustos 2026: belgeler *"fall short in explaining HOW to get trusted user status"*) [L9]. Onaylayıcılar sürüme geri bildirim bırakabilir; site geri bildirim bekleyen/alan/tamamlanan eklentiler için ayrı listeler tutar [W9].

**Yükleme denetimleri (kod).** Paket ≤ 25.000.000 bayt; zip adlarında `..`, mutlak yol, ters bölü ve sürücü harfi yok; `.pyc` yok; `__MACOSX`, `.git`, `__pycache__` klasörü yok; `testzip()` ile CRC denetimi; birden çok kök klasör uyarısı; klasör adı `^[A-Za-z][A-Za-z0-9-_]+$` ve yeni eklentide PEP 8 uyumlu; `metadata.txt`'de `[general]` bölümü; zorunlu alanlar `name`, `description`, `version`, `qgisMinimumVersion`, `author`, `email`, `about`, `tracker`, `repository`; `tracker`, `repository`, `homepage` adreslerinin gerçekten erişilebilir olması (iç ağ adresine çözülen ya da yönlenen istek bağlantı anında reddedilir); `LICENSE` dosyası **3 Haziran 2024'ten beri zorunlu** (QEP tartışması #279'a dayanarak); `supportsQt6` görülürse "kullanımdan kalktı" işaretlenir [W9][Q9]. Not: Cookbook `tracker`'ı isteğe bağlı gösteriyor, site ise zorunlu tutuyor [D1][W9].

**Kurallar (yayın ve onay kılavuzu).** Lisans "GPLv2 or later" ile uyumlu olmalı; ikili dosya yok; üretilmiş dosyalar (`ui_*.py`, `resources_rc.py`) kaynak deposunda yok; paket ≤ 25 MB; kısa İngilizce açıklama; dış bağımlılıklar About alanında açıkça yazılmalı, OSGeo4W'de olmayanlar için Windows kurulum talimatı verilmeli [W2]. Onayda: ana sayfa, sorun izleyici ve kaynak deposu bağlantıları çalışmalı ve kaynak kodu herkese açık olmalı; *"Plugins that utilize binaries will not be approved"*; onaylayıcılar eklentinin *"without crashing QGIS"* kurulup çalıştığını rastgele dener; Windows ve Unix'te çalışması beklenir; benzer eklentiler için iş birliği teşvik edilir [W3]. Güncellemelerde değişiklik notu ve yeni sürüm numarası istenir [W3].

**Güvenlik ve kalite taraması.** Bandit ve detect-secrets KRİTİK düzeyde **engelleyici**; Flake8, dosya izinleri ve şüpheli dosya denetimi bilgi amaçlı [W4][Q1]. Kurallar bir kural kaydındadır: kategori (`bandit`, `secrets`, `flake8`, `file_analysis`), önem (`info`, `warning`, `critical`), etkin/pasif (varsayılan pasif), "atlanabilir" işareti; geliştirici atlanabilir bir kuralı yüklemede gerekçesiyle atlar ve kayıt tutulur; pakete `.bandit`, `.secrets.baseline`, `.flake8` koyarak bulgu bastırılabilir, sürüm o zaman "Validated (configured)" olur [W9][W4]. Tarihçe: önce yalnız uyarı veren denetim (site PR 219); QEP 409 (20 Ocak 2026) engelleyici olmasını önerdi; 23 Nisan 2026 blog yazısı mevcut eklentilerin son sürümlerinin geriye dönük tarandığını ve rozet aldığını, masaüstü yöneticinin sonuçları henüz göstermediğini yazdı [Q1][L1]. QEP 409'un gelecek listesi: SPDX lisans başlığı, ikili blob tespiti, GPL uyumu, bağımlılık zafiyet taraması, kötü amaçlı imza taraması [Q1].

**Qt6 denetimi.** Her yüklemede `pyqgis4-checker` koşar; sonuç sürüm sayfasında "Qt6 Check" sekmesindedir ve *"informational only"* — yüklemeyi ya da onayı engellemez [W5][W9].

**Bakıcı iletişimi.** Onaylı eklentilerin e-postası doğrulanır; doğrulanmamışlara aylık hatırlatma, herkese yıllık yeniden doğrulama gönderilir (kod okuması; doğrulanmamanın sonucu kodda görülmedi) [W9]. Yazar e-postası sitede yalnız giriş yapmış kullanıcılara görünür [D1].

**Sayılar (13 Kasım 2026).** Site kategorileri: All 4.521, Stable 3.719, Experimental 1.533, Deprecated 173, QGIS 4 ready 1.665, Server 48, Fresh 236; "Unapproved" listesi 878 kayıt (en az bir onaysız sürümü olan, kaldırılmamış ve kullanımdan kaldırılmamış eklentiler; onay kuyruğunun büyüklüğü demek değildir) [W7]. QEP 409 (Ocak 2026): *"recently surpassed 3,000 plugins, including those currently under review"* [Q1]. XML'den [H1]: 3.44 için 4.461 kayıt (kararlı ve deneysel sürümler ayrı kayıt), 3.927 paket; 2.191 farklı yükleyici, bunların 1.549'u (%71) tek eklentili; `trusted` yükleyicinin paketi 261; kaynak deposu adreslerinin 4.108'i (%92) GitHub. İndirme (`downloads`, eklentinin bütün sürümlerinin toplamı [W8]): paketlerin toplamı 103.174.517; en çok NextGIS QuickMapServices 12.991.577; 13 paket 1 milyonun, 145 paket 100 binin üstünde; medyan 2.348; ilk %1 (39 paket) indirmelerin %51,6'sını, ilk %10 %84,2'sini alıyor.

**Yönetişim ve sorumluluk.** Ana sayfa QGIS kuruluşunun üçüncü taraf eklentilerden sorumluluk almadığını yazar [W1]. Güvenlik sayfası: *"We review submissions manually."* ve *"We do not undertake advanced QA assessments of plugin code."* [L15]. Bir onaylayıcı (Şubat 2026): *"The plugin review process is not focused on code review but does so in limited circumstances."* [L10]. Politika PSC kararları (oylamanın kaldırılması [K14]) ve QEP'lerle değişir [Q1][Q2][Q3]; siteyi Kartoza'daki ekip geliştiriyor [L2][L3]. Yaşam sonu: QEP 426, DB Manager'ı 4.2 ile çekirdekten çıkarıp son sürümünü depoya koymayı, yeni depoyu salt okunur yapıp gönüllü bakıcı çağırmayı öneriyor [Q3]. Buna karşılık bir eklentiyi doğru biçimde emekliye ayırmanın belgesi yok: *"I couldn't find detailed instructions for this."* (Şubat 2026) [L8].

### KentOS planı ne diyor

ADIM 22A: `taslak → gönderildi → denetleniyor → değişiklik isteniyor / onaylandı → yayımlandı → askıda / geri çekildi / arşivlendi`; yükleme yayın değildir; her sürüm yeniden onaydan geçer ve onay paket özetine bağlıdır; yazar kendi genel sürümünü onaylayamaz (PY-CLOUD-07); otomatik denetim (arşiv, şema, bağımlılık, lisans, uyumluluk, test, sır sızıntısı) ve yalıtılmış duman testi (PY-CLOUD-05, 06); ret gerekçe koduyla (PY-CLOUD-11); kötü sürüm bildirimi ve devir (PY-CLOUD-16).

### Öneri

| Konu | Karar | Adım | Somut iş |
|---|---|---|---|
| Otomatik kapı → insan onayı | **Al** | 21, 22A | QGIS'in `validating → blocked/validated → onay` akışı plandaki durum makinesine oturuyor. Engelleyici kural yalnız yüksek güvenli olanlar; bulgu yapısal (dosya, satır, kural kimliği, düzeltme önerisi), aynı kurallar yerelde `extension.verify`'da. |
| Kural kaydı ve gerekçeli atlama | **Al** | 21, 22A | Kural kimliği, önem, atlanabilirlik; yazarın gerekçeli atlaması denetim kaydına düşer; kural değişikliği için açık yol (QGIS: GitHub sorunu). |
| Geriye dönük herkese açık rozet | **Kaçın** | 22A | QGIS'te yanlış pozitifler (`requests.get()` zaman aşımsız, karma değerini sır sanma) herkese açık "security issues" rozetine dönüştü, rozet gizlenmek zorunda kaldı [L4]. Yeni kural önce sahibine gösterilir, süre verilir, sonra yeni yüklemelerde uygulanır. |
| Güvenilir yayımcı hızlı yolu | **Uyarla** | 22A | QGIS'te güvenilir kişi kendi sürümünü yayımlayabiliyor; plan PY-CLOUD-07 bunu yasaklıyor. Öneri (sahip kararı, ECO-DEC-12): genel katalogda ilk sürüm ve yetenek, bağımlılık, ağ hedefi ya da yerel kod farkı getiren her sürüm insan onayı; yalnız kod değişen yama sürümünde "doğrulanmış yayımcı + temiz otomatik kapılar + değişmemiş yetenek kümesi" hızlı yolu. AI ile üretilen eklenti hacminde insan onayı tek kapı olursa ölçeklenmez. |
| Zorunlu bağlantılar ve erişilebilirlik | **Al** | 16, 22A | Genel katalog için kaynak deposu ve sorun izleyici zorunlu, erişilebilirlik denetimi SSRF'e kapalı (QGIS örneği). Yerel ve kurum paketinde isteğe bağlı. |
| Lisans | **Al** (plan zaten `LICENSE` zorunlu) | 16 | Ek olarak SPDX ifadesi (`license: "GPL-2.0-or-later"` gibi) manifestte; katalog süzgeci ve QEP 409'daki SPDX hedefi. Genel katalog lisans politikası sahip kararı. |
| Boyut sınırı | **Al** | 16 | QGIS 25 MB; KentOS değeri altın örnek ve gerçek paketlerle ölçülüp ADR'de sabitlenir. |
| Emeklilik ve devir | **Al** (QGIS'te belgesiz) | 22A | "Kullanımdan kaldır + yerine geçen" ve "son sürüm + salt okunur + bakıcı çağrısı" (QEP 426) yolları baştan belgelensin; kurulu kullanıcıya ne olacağı yazılsın. |
| Tarama sonucunun uygulamada görünmesi | **Uyarla** | 18, 22A | QGIS masaüstü sonuçları göstermiyor [L1]. KentOS'ta inceleme ekranı onay kaydını, doğrulama özetini ve yetenek farkını göstersin. |
| Bakıcı iletişim doğrulaması | **Al** | 22A | Yayımcı iletişiminin doğrulanması ve periyodik yeniden doğrulama; sahipsiz eklentiyi erken görmenin ucuz yolu. |

---

## 4. Python bağımlılıkları

### QGIS ne yapıyor

**Tek yorumlayıcı, ortak paket kümesi.** Bütün eklentiler QGIS'in Python yorumlayıcısında, aynı süreçte çalışır. Matthias Kuhn (19 Mart 2021): *"Plugins in QGIS are unfortunately not 'isolatable' because they all run in one single process/python interpreter and therefore will share one set of dependencies. This is conceptually not perfect (aka. dependency hell), but not avoidable (at least not without a major effort on infrastructure, api and plugin side)."* [Q6].

**Resmî mekanizma yok.** `plugin_dependencies` yalnız eklentiden eklentiye bağımlılıktır; pip paketi kurmaz [Q8]. Yayın kılavuzu dış bağımlılıkların About metninde yazılmasını ve OSGeo4W'de olmayanlar için Windows talimatı verilmesini ister [W2]. Site `external_deps` ("PIP install string") adlı belgelenmemiş bir alanı kabul ediyor, ancak XML'deki 4.461 kaydın hiçbirinde `external_dependencies` dolu değil [W9][H1].

**Önerilip uygulanmayanlar.** QEP 179 (Mayıs 2020) eklenti yönetimini pip ve conda destekli gerçek paket yönetimine çevirmeyi; QEP 202 (Kasım 2020) `metadata.txt`'ye `pip_dependencies` ve kullanıcı profilinde bir sanal ortam eklemeyi önerdi. İkisi de uygulanmadan 31 Aralık 2024'te kapandı [Q7][Q6]. QEP 202 tartışmasındaki engeller KentOS için de kontrol listesidir [Q6]:
- pip'in kararlı bir kütüphane API'si yok; alt süreçle çağrılmak zorunda (elpaso).
- Yerel kütüphane çakışması: pip'ten `pyproj` kurmak QGIS'in derlendiği PROJ'dan farklı bir PROJ getiriyor ve QGIS'i çökertiyor; Nyall Dawson bu yüzden öneriye -1 verdi; Windows'ta conda/OSGeo4W karışımı aynı sorunu doğuruyor (m-kuhn).
- Profildeki sanal ortam, aynı makinedeki birden çok QGIS/Python sürümüne bağlanamıyor (wonder-sk).
- Kurumsal vekil sunucu ve SSL sorunları; ağdan herhangi bir şey kurmadan önce kullanıcı onayı şartı (elpaso).

**Topluluk araçları.** qpip (OPENGIS.ch): eklentinin `requirements.txt`'sini okur, profilde `python/dependencies` altına `pip --prefix` ile kurar, `qgis.utils.loadPlugin`'i değiştirerek (monkeypatch) yükleme öncesi denetim yapar, QGIS'in kendi paketlerini kısıt dosyasıyla sabitler; iki eklenti uyumsuz sürüm isterse kullanıcı birini seçer, *"likely breaking the other ones"* [A1]; 125.618 indirme [H1]. `qgis-plugin-dev-tools` (NLS Finland) bağımlılığı pakete gömüp içe aktarmaları yeniden yazar; yerel kütüphanelerde çalışmayabilir, `sys.path`'e ekleyen seçeneği kendisi "dangerous" diye adlandırır [A2]. QGIS Pip Manager adlı başka bir arayüz eklentisi de var (8.605 indirme) [H1].

**Politika henüz bağlanmadı.** Ekim 2024: *"a plugin downloading anything to be executed or interpreted should be entirely prohibited"* (Greg Troxel); Nyall Dawson böyle eklentinin depodan kaldırılmasını savundu; Matthias Kuhn birden çok eklenti aynı kütüphaneyi gömünce *"there is no isolation between libraries"* uyarısını yaptı [L6]. Mayıs 2026: AequilibraE'nin eklentileri ek bağımlılık indiren kod yüzünden yayından kaldırıldı; Nyall Dawson *"a plugin should NEVER automatically install dependencies like this"* deyip bu tür eklentilerin depoda açıkça engellenmesini önerdi; Julien Cabieces (Oslandia) qpip'in çekirdeğe alınmasını önerdi; eklenti yazarı manuel kurulumun *"a painful task for less technical users"* olduğunu yazdı [L5]. Karar yok. Kullanıcı tarafında, QGIS 4 macOS paketinde artık `pip3` bulunamadığı ve Matplotlib kurulamadığı soruluyor (Mart 2026) [L12].

### KentOS planı ne diyor

ADIM 18A: Python ve uv isteğe bağlı; kişisel analiz, eklenti geliştirme ve değişmez yayın ortamları ayrı (PY-UV-10); yayın ortamının anahtarı eklenti kimliği/sürümü, kilit özeti, Python ABI, işletim sistemi ve ek özellikler (PY-UV-11); iki eklentinin çelişen NumPy'ı aynı ad alanına girmez (PY-UV-12, P05); `pyproject.toml` + `uv.lock` tek kaynak (PY-UV-13); hazırlama ile çalıştırma ayrı, çalıştırmada indirme yok (PY-UV-15–17); kurum aynası, vekil sunucu ve dizin önceliği (PY-UV-25–27); bağımlılık önizlemesi (PY-UV-14).

### Öneri

| Konu | Karar | Adım | Somut iş |
|---|---|---|---|
| Eklenti başına ayrı süreç ve ortam | **Al (plan zaten)** — en değerli karar | 18A, 19 | QGIS'in 2020'den beri çözemediği sorunu kökten çözüyor. Başarım kaygısıyla "aynı süreçte yükle" kısayolu açılmamalı; açılırsa QGIS'in çakışma sınıfı geri gelir. |
| Çalışma anında kod/paket indirme yasağı | **Al** | 21, 22A | Plan yürütücü tarafını kapsıyor (PY-UV-15/16). Eklenti kodunun kendisinin `pip`, `uv`, `subprocess` ile kurulum yapması ya da indirdiği kodu `exec`/`import` etmesi verify Kapı 2'de AST kuralı ve katalogda engelleyici kural olsun; aracı (broker) da buna izin vermesin. |
| Onay ve açıklık | **Al** | 18A | QGIS tartışmalarının tek ortak noktası: kurulacakları kullanıcı görüp onaylar (qpip diyaloğu). Plan PY-UV-14 ve PY-OPT-10 bunu karşılıyor. |
| Yerel tekerlek ve platform sürtünmesi | **Uyarla** | 18A | Ayrı süreç host'u yerel kütüphane çakışmasından korur, ama platform tekerleği bulunamaması, macOS/Windows farkları ve kurumsal vekil sunucu QGIS'teki gibi asıl sürtünme olacak. ECO-QA-04 matrisi bunu ölçmeli; QEP 202'deki "izinli paket listesi" fikri kurum politikasına (PY-OPT-12) bağlanabilir. |
| Disk ve süre maliyeti | **Uyarla** | 18A | Her eklentiye ayrı venv; uv önbellek paylaşımının (PY-UV-12) gerçek disk/süre etkisi ölçülsün (ECO-DEC-04). |
| Serbest metinle bağımlılık anlatma | **Kaçın** | 16 | QGIS'te About metni ve belgelenmemiş `external_deps`. Plan kararı (B4: tek kaynak `pyproject.toml`/`uv.lock`) korunmalı. |

---

## 5. API kararlılığı ve sürüm göçleri

### QGIS ne yapıyor

**Politika.** API yalnız ana sürümler arasında değişebilir; ana sürümden sonra sınıf ve yöntemler yeniden adlandırılmaz, kaldırılmaz, anlamı değişmez; istisnalar ekip kararıyla, büyük etkili değişiklikler sonraki ana sürüme ertelenir [D9]. QEP 425 (Haziran 2026) bunu yazılı politikaya çevirdi: güvence PyQGIS API'si ve `qgis_process` CLI'si içindir; C++ API, uygulama kütüphanesi ve sağlayıcılar kapsam dışıdır; "internal" ve "tech preview" uyarılı yöntemler hariçtir; argüman **adları**, sırası ve dönüş değerleri güvencededir; eskime üç işaretle yapılır (`\deprecated QGIS x.xx, use yyy instead`, `Q_DECL_DEPRECATED`, `SIP_DEPRECATED`), eskiten geliştirici QGIS'in kendi kodundaki bütün çağrıları yenisine taşır ve eskime uyarısı CI'da hata sayılır; istisnai kırılma ancak QEP ile ve en az bir ara sürüm önceden uyarıyla olur [Q2]. Aynı QEP belge biçimlerini (QGS/QGZ, QML, QLR, `.model3` …) geriye dönük uyumlu tutar ve *"QGIS expression functions must remain stable and cannot change behavior between QGIS versions"* der [Q2]. Uygulaması: QEP 427 (Temmuz 2026), 3.8'den beri (7 yıl) eskimiş Processing arayüz API'sini bu istisna maddesiyle kaldırmayı öneriyor [Q4].

**QGIS 2 → 3 (2018).** Python 2→3 ve PyQt4→PyQt5 birlikte geldi; QGIS 2 eklentileri QGIS 3'te çalışmadı, yazarların taşıması gerekti; araçlar `qgis2to3` ve API kırılmalarını arayan `qgis2apifinder`; eklenti klasörü kullanıcı profiline taşındı [K18]. Wiki o zaman şu öneriyi yaptı: Qt'yi doğrudan değil `qgis.PyQt` üzerinden içe aktar ki sonraki Qt göçü kolay olsun [K18].

**QGIS 3 → 4 (Qt5 → Qt6; 4.0.0 6 Mart 2026, 4.2.0 3 Temmuz 2026, 4.4.0 30 Ekim 2026 [L14]).** Değişiklik günlüğü *"retained deprecated APIs where possible"* diyor [L13]. Değişikliklerin büyüğü kapsamlı enum adları: `Qt.UserRole` → `Qt.ItemDataRole.UserRole`, `QgsMapLayer.VectorLayer` → `QgsMapLayer.LayerType.VectorLayer` (ikisi de Qt5'te de çalışır, yani tek kod iki sürümü destekleyebilir) [K17]. PyQt6'da Qt kaynak sistemi desteği ve `pyrcc6` yok [A6]; geliştirici belgesi `rcc -g python`'a geçti [D2]. Qt6 yapısında `PyQt5` içe aktarması QGIS'in içe aktarma kancası tarafından `ImportError` ile durdurulur ve doğru içe aktarma önerilir [K1]. Araçlar: `pyqt5_to_pyqt6.py` betiği (4.0 günlüğünde betiğin bazı içe aktarma biçimlerini kaçırdığı ve bozuk sözdizimi ürettiği hataların düzeltildiği görülüyor) [L13][K17], `pyqgis4-checker` Docker imgesi (kuru koşu ve yerinde düzeltme, CI örnekleri) [K19], sitede her yüklemede bilgi amaçlı Qt6 denetimi [W5].

**Uyumluluk üst verisi karmaşası.** QGIS 4 öncesinde eklentiler Qt6 hazırlığını `supportsQt6=True` ile bildiriyordu. Yükleyici eksik `qgisMaximumVersion`'u "aynı ana sürümün sonu" sayarken depo onu `x.99` diye dolduruyordu; yazarlar alanın zorunlu olduğunu sanıp `3.99` yazdı; Qt6 uyumlu ama üst sınırı 3.99 olan eklentiler QGIS 4'te depodan kurulamaz hale geldi (Borys Jurgiel, 24 Şubat 2026) [L7]. 2–6 Mart 2026'da birleşen iki PR otomatik yükseltmeyi geri aldı ve `supportsQt6`'yı yok saydı; ikincisi QGIS 4.0'ın çıktığı gün birleşti [K15][L13]. Artık iki ana sürümü destekleyen eklenti `qgisMaximumVersion=4.99` yazmak zorunda [W5][K17].

**Ana sürüm geçişinde eklentiler taşınmıyor.** QGIS 4, QGIS 3 profilini kopyalarken `python/plugins` klasörünü bilerek dışarıda bırakır; kod yorumu: *"any plugin folders -- require users to reinstall those, so that we don't copy broken, non-updated 3.x plugins"* [K21]. Kullanıcı eklentilerini QGIS 4 kataloğundan yeniden kurar; kopyalanan ayarların bu yeniden kurulumla nasıl etkileştiği doğrulanmadı.

**Sonuç: göç oranı (13 Kasım 2026, 4.0'dan 8 ay sonra).** QGIS 4.0 kataloğunda 1.654 paket var; 3.44'teki 3.927 paketin %42'si. En çok indirilen 20 paketin 18'i, 50'nin 41'i, 100'ün 76'sı, 500'ün 236'sı hazır; indirme ağırlıklı oran %68,4; yalnız 4.x için açılmış 100 paket var [H2]. Hazır olmayan önemli bir örnek: QGIS Resource Sharing (384.276 indirme, en çok indirilen 40. eklenti, `qgis_maximum_version` 3.99) [H1]. Sitenin "QGIS 4 ready" kategorisinde 1.665 kayıt [W7].

**Belge gecikmesi.** 13 Kasım 2026'da `docs.qgis.org/latest` hâlâ 3.44 belgesine gidiyor; `docs.qgis.org/4.0`, `/4.2`, `/4.4` adresleri "testing"e yönleniyor. PyQGIS API belgesi 4.2 ve 3.44 için var, 4.4 için yok [D10].

### KentOS planı ne diyor

`schemaVersion` tam sayı, bilmediği sürüm "daha yeni bir KentOS gerekir" diye reddedilir, bilinmeyen alan reddedilir (§3.2); `compatibility.sdkProtocol` (§3.2); SDK katalogdan üretilir ve sapma derleme hattında hata (§2, ECO-API-01, 05); uyumluluk/ABI/eskime matrisi keşfedilebilir (ECO-API-11); `kentos.ui` bildirimsel, keyfi HTML/JS/QML yok, Python ikinci bir tasarım dili yaratmaz (PY-UI-12, 14); `.kcad` şema 254'te eklenti verisi yeri (B9).

### Öneri

| Konu | Karar | Adım | Somut iş |
|---|---|---|---|
| Yazılı kararlılık politikası | **Al** (QEP 425) | 21 + ADR | Güvencedeki yüzey: `kentos.ext` Python API'si, manifest/araç/panel şemaları, MCP araç adları ve sonuç şemaları. Argüman adı/sırası/dönüş dahil; eskime işareti + en az bir ara sürüm uyarı; istisna ADR ile. |
| Araç takımından bağımsız UI | **Al (plan zaten)** | 22 | QGIS'te Qt'nin bir ana sürüm değiştirmesi paketlerin %58'ini geride bıraktı. Bildirimsel `kentos.ui` bu maliyeti eklentiye yansıtmaz; "özel bileşen" kaçış yolu (PY-UI-15) de Python'a çizim işaretçisi vermeden kalmalı. |
| Uyumluluk anlamının tek yeri | **Kaçın** (QGIS'te host ve depo ayrı yorumluyordu) | 16 | Uyumluluk hesabı `kentos-extensions` crate'inde tek fonksiyon; masaüstü, CLI, MCP ve bulut kataloğu onu çağırır; anlamı değiştirmek ADR ve sürüm notu ister; çıkış haftasında değiştirilmez. |
| Uygulama sürümü ve protokol | **Uyarla** | 16 | Tek `sdkProtocol` tamsayısı yetmeyebilir: QGIS'in min/max uygulama sürümüne karşılık hem protokol aralığı hem en düşük KentOS sürümü (yeni bir yetenek o sürümle geldiyse). "Üst sınır yazılmamışsa ne olur?" kuralı belgede tek cümleyle sabitlensin; katalog boş alanı kendiliğinden doldurmasın. |
| Göç denetleyicisi | **Al** | 21 | `extension.verify` eski SDK/şema kullanımını sürüme özgü kural kimlikleriyle raporlar, mümkünse otomatik düzeltme önerir (pyqgis4-checker + `pyqt5_to_pyqt6.py` karşılığı); katalog her yüklemede bilgi amaçlı koşturur. |
| Sürümlü belge | **Al (plan zaten)** | 21 | QGIS'te 4.x belgesi 8 ay gecikti. Plan "sürüme bağlı API referansı zorunlu teslim" diyor; belgeler aynı commit'te üretilip sürümle yayımlanmalı. |
| Ana sürüm geçişinde kurulu liste | **Uyarla** | 17, 22A | QGIS kırık kodu taşımamak için eklenti klasörlerini hiç taşımıyor; kullanıcı neyin eksik olduğunu kendisi bulup yeniden kuruyor. KentOS'ta protokol artışında kurulu eklentiler listede kalsın, uyumlu sürüm katalogdan önerilsin, uyumsuz olan nedenli pasif kalsın: kırık kod çalışmaz ama kullanıcı neyi kaybettiğini görür. |
| Göç oranını ölçmek | **Al** | 22A | Her protokol artışında "yeni protokole hazır eklenti oranı" (sayı ve kurulum ağırlıklı) katalogdan izlensin; QGIS'in %42 / %68 farkı ikisinin de gerekli olduğunu gösteriyor. |
| İfade ve denetim davranışının kararlılığı | **Al** | 19, 20 | QEP 425 §2.4.1 ifade fonksiyonlarının davranışını sabitliyor. `@ext.expr` ve `@ext.audit` sonuçları belgeye işlediği için davranış değişikliği sürüm artışı olsun; köken kaydı (betik sürümü + SHA-256, ADIM 20) bu yüzden önemli. |

---

## 6. Geliştirici deneyimi

### QGIS ne yapıyor

**Belge.** PyQGIS Developer Cookbook (eklenti yapısı, İşlemler, sunucu, görevler, yayın) [D1][D3][D4][D7][D6] ve PyQGIS API belgesi [D10]. Cookbook ayrıca arka plan görevleri için açık kurallar koyar: Qt bileşenlerine yalnız ana iş parçacığından dokunulur; `QgsVectorLayer`, `QgsProject` gibi ana iş parçacığı nesnelerini arka plandan kullanmak *"will result in crashes"*; uzun `run()` düzenli olarak `isCanceled()`'a bakmalıdır [D7].

**İskele ve paketleme.** Plugin Builder 3 (152.149 indirme; QGIS 4 için Plugin Builder 4, 3.899 indirme) [H1]; Oslandia'nın Cookiecutter tabanlı "QGIS Plugin Templater"ı (testler, belge, kod yapısı, biçimlendirici, linter) ve onun arayüz eklentisi [A5]; `pb_tool` (derle, dağıt, zip; 3.5.4, 31 Ekim 2026) [A4]; `qgis-plugin-ci` (paketle, resmî depoya yükle, GitHub sürümlerinde özel `plugins.xml` yayımla, Transifex çevirisi, CHANGELOG'dan `changelog`, etiketten `experimental`; 2.11.0, 27 Ekim 2026) [A3]; `qgis-plugin-dev-tools` (bağımlılık gömme, sürüm ve değişiklik notunu CHANGELOG'dan alma, yayımlama) [A2].

**Test.** Resmî `qgis/qgis` Docker imgeleri (839.792 çekme, son güncelleme 12 Kasım 2026; `stable`, `ltr`, sürüm etiketleri) [A7]; imgedeki `qgis_setup.sh` profil klasörlerini kurar, eklentiyi sembolik bağla "kurar", engelleyici diyalogları bastıran bir `startup.py` koyar, eklentiyi etkinleştirir; `qgis_testrunner.sh` testleri başsız koşar [K16]. `pytest-qgis` (5.0.2) [A4]; Qt6 göçü için `pyqgis4-checker` CI örnekleri [K19].

**Çeviri ve kaynaklar.** Qt Linguist zinciri: `.pro` dosyası, `pylupdate5`, `lrelease`; `.ts` dosyası `eklentiadi_dil.ts` biçiminde adlandırılmazsa dil yüklenmez; arayüz değişince ikisi de yeniden üretilir [D1]. Yönetici `name`, `description`, `about`, `tags` alanlarının yerelleştirilmiş çeşitlerini (`name[de]` gibi) okur [K4]. `resources.qrc` 3.44'te `pyrcc5`, testing belgesinde `rcc -g python` ile derlenir; önek başka eklentilerle çakışmamalıdır [D1][D2].

**Hata ayıklama.** Plugin Reloader; eklentilere ayrılmış günlük sekmesi; `qgis.utils.plugins['Eklenti']` ile canlı örneğe erişim [D1]; Geliştirme/Hata Ayıklama panelinde başlangıç (3.14) ve proje yükleme (3.16) profilleri [K12][L16]; bu ihtiyaç için ayrıca bir "Plugin Load Times" eklentisi var (46.929 indirme) [H1].

**Zayıf noktalar.** Belgedeki zorunluluk tablosu ile sitenin denetimi ayrışıyor (`tracker`) [D1][W9]; 4.x için sürümlü belge yok [D10]; denetimler (Bandit, detect-secrets, Flake8, Qt6 denetimi) yüklemeden **sonra** sitede koşuyor; sitenin denetimini yerelde birebir koşturan resmî tek bir araç görmedim, yazar bu araçları ayrı ayrı kurup çalıştırabiliyor.

### KentOS planı ne diyor

ADIM 21: `extension.blueprint` (iskelet, ikonlar, bileşen şemaları, altın örnek), `extension.verify` / `kentos kek verify` (şema ve başvuru, AST ve imza, başsız kuru koşu, yapısal rapor), `extension.hot_reload`, `extension.preview_render`; Rust sözleşmelerinden üretilen `.pyi`, ikon listesi ve JSON Schema'lar; sürümlü API referansı, rehber, çalışan örnekler, çevrimdışı belge paketi; SDK şablonları (ECO-API-10); hata ayıklama raporu (ECO-API-14). ADIM 16: `pack` aynı doğrulayıcıyı kullanır.

### Öneri

| Konu | Karar | Adım | Somut iş |
|---|---|---|---|
| Tek komutlu döngü | **Uyarla** | 21 | QGIS'te iskele, paketleme, denetim ve Qt6 kontrolü ayrı araçlar ve denetim yüklemeden sonra. KentOS'ta `kentos kek new → verify → pack → test` tek CLI, aynı doğrulayıcı yerelde, CI'da ve katalogda. |
| Başsız test imgesi | **Al** | 21 | `qgis_setup.sh` deseni: geçici profil, klasörden kurulum, engelleyici diyalog yok, etkinleştirme, başsız test koşucusu. KentOS için CI şablonu (GitHub Actions) ve `kentos kek test`. |
| Geliştirici klasörü | **Al** | 17, 21 | QGIS'te `QGIS_PLUGINPATH` ya da sembolik bağla paketlemeden çalışma. KentOS'ta "geliştirme kaynağı": paketlenmemiş klasör kayıt defterine "yerel geliştirme" güven durumuyla bağlanır, `hot_reload` onu izler. |
| Günlük ve profil | **Al** | 18, 19 | Eklenti başına günlük kanalı, kayıt/ilk çağrı süresi, son hata; hata ayıklama raporuna (ECO-API-14) girer. |
| Yerelleştirme | **Uyarla** | 16, 22 | `name[tr]` yerine `l10n/<dil>.json` (manifest, panel, form, araç metinleri); verify eksik anahtarı bildirir; katalog kullanıcının dilini seçer. |
| Belge ile denetimin ayrışması | **Kaçın** | 16 | Zorunluluk tek şemadan (Rust tipleri → JSON Schema) üretilir; belgedeki tablo da oradan. |
| AI'nın birincil geliştirici olması | **Uyarla** | 21 | QGIS'in Plugin Builder + linter + checker üçlüsü KentOS'ta blueprint + verify döngüsüdür; ajan hata raporunu tek adımda düzeltebilsin diye kural kimlikleri kalıcı olmalı (plan zaten öyle). |

---

## 7. Güvenlik ve güven modeli

### QGIS ne yapıyor

**Tam yetkiyle, aynı süreçte.** QGIS'in güvenlik sayfası: eklenti yükleyicisi üçüncü taraf eklentileri *"just like web browsers do"* kurar ve Python kodu *"execute arbitrary code with the permissions of the user"*; gönderiler elle incelenir, *"We do not undertake advanced QA assessments of plugin code."*; eklentiler derlenmiş kod içermemeli ki incelenebilsin; hassas ortamlara eklentileri korumalı alanda denetlemeyi, iç depo kurmayı ve genel depoya erişimi kapatmayı önerir [L15]. Korumalı alan yoktur (tek süreç, tek yorumlayıcı) [Q6][K2]. Yükleyicide indirilen paketin özetini ya da imzasını doğrulayan bir adım yoktur: HTTPS ile indirir, geçici klasöre açar, kopyalar [K6]. QEP 336 da kriptografiyi bilerek kapsam dışı bıraktı [Q5].

**Proje içi kod.** Projeler makro, ifade fonksiyonu, eylem ve form başlatma kodu taşıyabilir. QEP 336 (Ocak 2025) "güvenilir proje / güvenilir klasör" önerdi (VS Code benzeri uyarı, kurumsal INI ile önceden tanım); `QgsProjectTrustDialog` QGIS 4.0'da geldi [Q5][D11].

**Depo tarafı.** İkili yasağı ve açık kaynak kodu şartı [W2][W3]; 2026'dan beri engelleyici tarama [W4]; ZIP'ten kurma uyarısı [K7]. Sitedeki `trusted` bilgisi yükleyicinin onay yetkisidir, kodun denetlendiği anlamına gelmez [W9]; masaüstü yöneticinin C++ kodunda bu bilginin gösterildiği bir yer bulamadım [K7].

**Belgelenmiş olaylar ve tartışmalar.**
- **Oy istismarı (Ağustos 2026):** oyların hepsi Cloudflare kenar IP'leriyle kaydedildiği için IP başına sınır işlemedi; incelenen son 100 oyun 100'ü Cloudflare aralığındaydı; giriş zorunluluğu, sahte oyları silme ve masaüstünden oylamayı kaldırma önerildi [L3]. Sonuç: PSC kararıyla oylama kaldırıldı [K14].
- **Geriye dönük rozet (Nisan 2026):** tarama mevcut sürümleri herkese açık "security issues" rozetiyle işaretledi; yanlış pozitifler yüzünden rozet herkesten gizlendi, yalnız sahip ve yöneticilere gösterildi [L4]. Blog yazısı da *"Almost all plugins initially have this badge"* diyor [L1].
- **Bağımlılık indiren eklentilerin kaldırılması (Mayıs 2026):** bkz. §4 [L5].
- **AI ile üretilen eklentiler:** Şubat 2026'da bir onaylayıcı yüklemelerde belirgin artış gördüğünü, bunun güven, inceleme ve yönetişim sorunları getirdiğini yazıp isteğe bağlı bir `ai_derivative` üst veri bayrağını önerdi (kalite yargısı değil, açıklama) [L10]. Haziran 2026'da PSC listesinde LLM kullanan eklentilerin hangi modeli kullandığı ve kullanıcı verisini nasıl işlediği konusunda açıklık istendi; Tim Sutton güvenlik, bakım ve uyum etiketleri ile *"completely vibe coded, vibe coded with human review, ai completion assisted, no LLM used"* gibi LLM beyan kategorileri üzerinde çalışıldığını yazdı [L11]. Karar ya da alan henüz yok.
- **Kötü amaçlı eklenti olayı:** Resmî depoda belgelenmiş bir kötü amaçlı eklenti olayı bulamadım (doğrulanamadı). QEP 409'un gerekçesi genel: *"ensure that malicious or low-quality plugins are not distributed through QGIS infrastructure"* [Q1].

### KentOS planı ne diyor

`requestedCapabilities` ve v1 yetenek sözlüğü (`network`, `files.userSelected`, `document.write` …), istenen yetenek verilmiş yetki değildir (§3.2); `integrity.json` ve içerik özeti, onay özete dışarıdan bağlanır (§3.1, PY-CLOUD-10); güvenilir yerel kip ile işletim sistemi düzeyinde korumalı kip ayrı, "ayrı süreç tek başına korumalı alan değildir" (PY-RUN-19–21); eklenti açıklaması ve README güvenilmeyen veridir (PY-AI-09); güven kökü açık karar (ECO-DEC-03).

### Öneri

| Konu | Karar | Adım | Somut iş |
|---|---|---|---|
| Dürüst güven dili | **Al** | 19, 22A | QGIS'in güvenlik sayfası neyi vaat etmediğini açıkça yazıyor. KentOS'ta da "onaylı", "kurum", "yerel" rozetlerinin neyi garanti ettiği ve etmediği tek sayfada (plan PY-RUN-20 ile aynı ruh). |
| Kurum için kapalı katalog | **Al** | 18A, 22A | QGIS'in "iç depo + genel depoyu kapat" önerisi → kurum politikası yalnız kurum kataloğuna ve onaylı paketlere izin verebilir (PY-OPT-12). |
| İmza ve bütünlük | **Uyarla** | 16, 22A | QGIS'te hiçbiri yok. v1: içerik özeti (`integrity.json`) + katalogun imzaladığı onay kaydı (imzayı yayımcı değil katalog atar) + yerelde doğrulama; yayımcı imzası ECO-DEC-03 ile sonra. |
| Ağ ve LLM açıklığı | **Al** | 16, 22A | QGIS bunu 2026'da tartışmaya başladı. `network` yeteneği hedef alan adlarını (ör. `api.ornek.gov.tr`) taşısın; LLM/harici servis kullanan eklenti bunu ve gönderilen veri türünü manifestte beyan etsin; inceleme ekranında görünsün. Sözlük genişletmesi sürüm artışı olduğu için v1'e girmeli. |
| AI üretim kökeni | **Uyarla** | 21, 22A | KentOS eklentileri zaten AI ürünü; ayırt edici olan "hangi ajan/model, insan inceledi mi, verify raporunun özeti". Bu pakette değil katalog kaydında tutulsun (yeniden paketlemeyle değişmesin). |
| Etkileşim verisine dayalı güven | **Kaçın** | 22A | Anonim oy ve yorum istismara açık; QGIS kaldırdı. |
| Proje içi kod | **Al (plan zaten)** | 17, 22 | QEP 336'nın çözdüğü sorun KentOS'ta tasarımla yok: `.kcad` eklenti verisi kod taşımaz, form/ifade taşınabilir profilde (plan §1.3). Bu sınır gevşetilmemeli. |

---

## 8. Ekosistemin sağlığı

### Ne büyüttü (kanıtla)

1. **Eşik çok düşük:** iki dosyalık klasör [D1]; 2.191 yükleyicinin 1.549'u (%71) tek eklenti yayımlamış [H1]: ekosistemin gövdesi "bir işi çözüp paylaşan" kullanıcılar.
2. **Merkezî depo uygulamanın içinde:** resmî depo önceden tanımlı, listeden silinse de açılışta geri ekleniyor; güncelleme kendiliğinden denetleniyor [K4][D5].
3. **Keşif:** arama, etiket, indirme sıralaması, "New" ve "Upgradeable" sekmeleri [D5][K7]. İndirmeler çok yoğunlaşmış: ilk %1 paket indirmelerin %51,6'sı [H1]; ilk sıralar bütün ekosistemin vitrini.
4. **İşlemler bütünleşmesi:** yayın kılavuzu analiz yapan eklentileri İşlemler alt eklentisi olarak yazmayı öneriyor; gerekçesi toplu çalıştırma, iş akışına (modele) katılma ve ayrı arayüz tasarlama gereğinin kalkması (kılavuzun özeti, birebir alıntı değil) [D6][D3].
5. **Açık kaynak ve devralınabilirlik:** kaynak deposu zorunlu, %92'si GitHub'da [W3][H1]; terk edilen eklenti çatallanıp sürdürülebiliyor (QEP 426'nın bakıcı çağrısı modeli) [Q3].
6. **Şirket destekli eklentiler:** en çok paket yükleyenler arasında NextGIS (26) ve Oslandia (25) [H1]; en çok indirilen 15'in ilki NextGIS'in QuickMapServices'i [H1].
7. **2026 hızlanması:** bugün listelenen paketlerin 1.567'si 2026'da açıldı; aylık yeni paket Ocak'ta 73'ten Ekim'de 213'e çıktı [H3]. Bir onaylayıcı bu dönemde AI etkisini gözlemliyor [L10] (nedensellik kanıtlanmış değil).
8. **İhtiyacın ne olduğu:** en çok indirilen 15 eklentinin en az 5'i veri ve altlık erişimi (QuickMapServices, OpenLayers Plugin, QuickOSM, Google Earth Engine, OSMDownloader), 3'ü dışa aktarma/biçim (qgis2web, Qgis2threejs, KML Tools), 1'i saha senkronu (QField Sync) [H4]. Birincisi QuickMapServices, servis adreslerini elle yazmak yerine topluluğun katkı verdiği bir servis kataloğundan tek tıkla altlık ekliyor [A8]; benim yorumumla en popüler QGIS eklentisi büyük ölçüde bildirimsel bir veri kataloğudur. Python'suz içerik paylaşımı için ayrıca bir eklenti var (QGIS Resource Sharing, 384.276 indirme) [H1].

### Ne yıprattı (kanıtla)

1. **Tek süreç:** Python istisnası mesaj çubuğuyla atlatılıyor ama yerel çökme QGIS'i düşürüyor; bekçi ancak bir sonraki açılışta devreye giriyor [K1][K2].
2. **Temizlik yazara bağlı:** kayıtların kaldırılması unutulunca çökme [K9]; Python–C++ nesne ömrü karışıklığı [Q4].
3. **Bağımlılık cehennemi:** §4 [Q6][A1][L5][L6].
4. **Açılış yavaşlığı:** bütün eklentiler açılışta yükleniyor [K2]; çekirdek geliştirici şüpheliyi eklenti olarak gösteriyor [K12]; ayrıca bir "Plugin Load Times" eklentisine ihtiyaç duyulmuş [H1].
5. **Kalite dağılımı:** inceleme kod incelemesi değil [L10][L15]; ilk taramada *"Almost all plugins initially have this badge"* [L1].
6. **Terk ve bakım:** 173 kullanımdan kaldırılmış eklenti [W7]; 3.44 paketlerinin 616'sının (%15,7) son güncellemesi 2022 ya da öncesinde [H1]; QGIS 4'e geçen %42 [H2].
7. **Ana sürüm göçlerinin bedeli:** 2→3'te her eklenti yeniden taşındı [K18]; 3→4'te araç takımı değişikliği [H2].
8. **Üst veri anlam karmaşası:** `qgisMaximumVersion` / `supportsQt6` [L7][K15].
9. **Sinyal kirliliği:** oy istismarı [L3].
10. **Arayüz kalabalığı:** kural yok, öneri var: cookbook eklentinin kendi menüsünü açmak yerine önceden tanımlı menüleri kullanmayı ve `objectName` vermeyi (kullanıcı özelleştirebilsin diye) öneriyor [D1].

### KentOS planıyla karşılaştırma

Plan, yıpratan etkenlerin 1–4, 7, 8 ve 10'una mimari çözüm getiriyor: ayrı süreç, kiralama, kilitli ortam, tembel başlatma, bildirimsel UI, tek doğrulayıcı ve `utility` eklentinin şerit sekmesi açamaması (EKL-02, arayüz kalabalığına karşı kural). Büyüten etkenlerde ise plan zayıf: 1 (eşik) `pyproject.toml`/`uv.lock`/`integrity.json` gibi zorunluluklar yüzünden elle yazana yüksek, ancak AI blueprint ve `pack` bunları ürettiği için kabul edilebilir; 3 (keşif) için manifestte alan yok; 8 (ihtiyaç) için yedi katkı noktasında veri kaynağı/altlık yok.

### Öneri

| Konu | Karar | Adım | Somut iş |
|---|---|---|---|
| Veri kaynağı/altlık katkısı | **Uyarla** | 18 (+16 sözlük) | Python'suz katkı türü olarak WMS/WMTS/XYZ/vektör karo tanımı, atıf ve lisans metni, kapsama alanı, kimlik bilgisi gerekip gerekmediği. QuickMapServices'in 13 milyon indirmesi bu ihtiyacın büyüklüğü. Yetenek sözlüğüne girecekse v1'de karar verilmeli. |
| Python'suz içerik paketi | **Al (plan zaten)** | 18 | Form, şema, stil, model paylaşımı QGIS'te ayrı bir eklentiye kaldı (Resource Sharing, QGIS 4'e taşınmadı). KentOS'ta `.kek`'in birinci sınıf katkısı olması doğru. |
| Sağlık ölçütleri | **Al** | 22A | Katalog ilk günden şunları tutsun: etkin kurulum, çökme/bekçi olayları, kayıt ve ilk çağrı süresi, son güncelleme, protokol uyumu, bakıcı iletişim doğrulaması, verify sonucu. QGIS'in "son güncelleme 2022 ve öncesi %15,7" türü bulguları ancak böyle görülür. |
| Eşik | **Uyarla** | 16, 21 | Elle yazan kullanıcı için en küçük geçerli `.kek` (Python'suz: `manifest.json` + `LICENSE` + bir form) `pack` ile tek komutta; Python'lu paket blueprint ile. |
| Arayüz kalabalığı | **Al (plan zaten)** | 18 | `vertical`/`utility` ayrımı QGIS'te olmayan bir kural; korunmalı. |

---

## 9. `metadata.txt` → `manifest.json` eşleme tablosu

QGIS sütunu [D1][D2][W9][K4]'ten; KentOS sütunu plan §3.2'den. "Öneri" sütunundaki alan adları önerilerdir, plan kararı değildir.

| QGIS alanı | QGIS'teki anlamı | KentOS v1 karşılığı | Öneri |
|---|---|---|---|
| klasör adı (`package_name`) | Kalıcı kimlik; sitede benzersiz, değiştirilemez; `^[A-Za-z][A-Za-z0-9-_]+$` | `id` (ters alan adı, küçük harf) | KentOS daha iyi: yayımcı ad alanı + katalog doğrulaması. |
| `name` | Zorunlu, görünen ad; değişebilir | `name` | Aynı. Yerelleştirme `l10n/` ile. |
| `name[xx]`, `description[xx]`, `about[xx]`, `tags[xx]` | Yükleyicinin okuduğu yerelleştirilmiş çeşitler | yok | `l10n/<dil>.json` (manifest, UI, form metinleri) ekle (ADIM 16). |
| `description` | Zorunlu, tek satır, HTML yok | `summary` | Aynı; uzunluk sınırı şemada. |
| `about` | Zorunlu (site), uzun açıklama, HTML yok | yok (`docs/` klasörü var) | `description`: uzun düz metin, kod çalıştırmadan katalogda gösterilir (ADIM 16). |
| `version` | Zorunlu, noktalı sürüm; eklentide benzersiz | `version` (anlamsal sürüm) | Aynı; önsürüm eki (`1.2.0-beta.1`) deneysel kanalı da taşır. |
| `qgisMinimumVersion` | Zorunlu; en düşük QGIS | `compatibility.sdkProtocol` | Protokol aralığı + isteğe bağlı en düşük KentOS sürümü (ADIM 16). |
| `qgisMaximumVersion` | İsteğe bağlı; yoksa "aynı ana sürümün sonu"; 2026 karmaşasının kaynağı | yok | Protokol aralığının üst ucu; boşsa anlamı tek cümleyle sabit; katalog doldurmaz. |
| `supportsQt6` | 6 Mart 2026'da kaldırıldı, site "deprecated" işaretler | — | Karşılığı gerekmez: araç takımı manifestte yer almaz. |
| `author` | Zorunlu, serbest metin (eğik çizgi yok) | `publisherId` (paketteki iddia) | `authors[]` (görünen ad) + katalogda doğrulanan `publisherId`. |
| `email` | Zorunlu; sitede yalnız girişli kullanıcıya, yöneticide kurulumdan sonra görünür | yok | Pakete gömmeyin (kişisel veri); katalog hesabında doğrulanmış iletişim (QGIS'in e-posta doğrulaması gibi). |
| `homepage` | İsteğe bağlı; erişilebilirlik denetimi | yok | `links.homepage` (ADIM 16). |
| `repository` | Zorunlu; herkese açık kaynak, erişilebilirlik denetimi | yok | `links.source`; genel katalogda zorunlu (ADIM 16, 22A). |
| `tracker` | Belgede isteğe bağlı, sitede zorunlu | yok | `links.issues`; genel katalogda zorunlu. |
| `changelog` (+ `CHANGELOG` dosyası) | İsteğe bağlı; yönetici kurulu eklentide gösterir; resmî `plugins.xml` bu öğeyi taşımadığı için kurmadan önce görünmez | yok (katalogda "sürüm notları", PY-CLOUD-03) | `changelogFile: "CHANGELOG.md"`; inceleme ve güncelleme ekranında. |
| `tags` | Virgüllü, serbest; arama ve süzgeç | yok | `keywords[]` + önerilen sözlük (Türkçe ve İngilizce). |
| `category` | Raster/Vector/Database/Mesh/Web menü yeri; resmî XML taşımıyor | `kind` + `modules` + `contributions` | KentOS daha iyi; katalog süzgeci `modules`'den. |
| `icon` | PNG/JPEG yolu | `icon` (güvenli SVG alt kümesi) | Aynı. |
| `experimental` | Sürüm başına; yönetici varsayılan gizler, ayrı düğmeyle kurulur | yok | Önsürüm eki ya da `channel: "preview"`; katalog yalnız isteyene gösterir (ADIM 16, 22A). |
| `deprecated` | Eklentinin tamamına; gri, sona itilir | yok (katalog: askıda / geri çekildi / arşivlendi) | Katalog durumu + manifestte isteğe bağlı `deprecation: { "replacedBy": "…", "message": "…" }`. |
| `plugin_dependencies` | 3.8+; görünen adla; yalnız `==`; kurulumdan sonra çözülür | yok | `dependencies.extensions: [{ "id": "tr.ornek.temel", "version": "^1.2" }]`; kimlikle, aralıkla, kurmadan önce görünür (ADIM 16, 17). |
| `external_deps` (yalnız site, belgesiz) | "PIP install string"; XML'de hep boş | `python.projectFile` + `python.lockFile` | Plan doğru (B4); üçüncü bir liste yok. |
| `server` | QGIS Server'ın yükleyeceğini bildirir | `capabilities.runtimes` (araç başına) | Manifestte üst düzey özet `runtimes` (masaüstü / başsız / bulut) katalog süzgeci için. |
| `hasProcessingProvider` | `qgis_process`'in yükleyeceğini bildirir | `entrypoints.processing` var mı | Ayrı bayrak gerekmez; tanımdan türetilir. |
| `LICENSE` dosyası | Yayın için zorunlu (3 Haziran 2024'ten beri) | `licenseFile` + `LICENSE` zorunlu | Ek: SPDX ifadesi `license`. |
| site/XML alanları: `trusted`, `downloads`, `create_date`, `update_date`, `uploaded_by`, `average_vote`, `rating_votes`, `disabled` | Paketin değil deponun bilgisi | yok (katalog kaydı) | Katalogda; oy yok; `trusted` yerine doğrulanmış yayımcı + onay kaydı + AI kökeni. |
| *(QGIS'te yok)* | — | `format`, `schemaVersion`, `integrity.json`, `requestedCapabilities`, `entrypoints`, `contributions`, `python`, `schemaFile` | KentOS'un üstünlüğü; korunmalı. |
| *(QGIS'te tartışılıyor)* | `ai_derivative` önerisi, LLM beyan kategorileri [L10][L11] | `network` yeteneği | `network` için hedef alan adları + "harici servis/LLM kullanır" beyanı (ADIM 16); üretim kökeni katalogda (ADIM 22A). |

---

## 10. En önemli 15 ders (ekosistemin büyümesi ve sağlığına etkisine göre)

1. **Katalog uygulamanın içinde, tek tıkla kurma ve güncelleme, yerel dosyayla da aynı akış.** QGIS'in büyüme motoru budur (önceden tanımlı depo, 4.461 kayıt, ~103 milyon indirme). KentOS: Eklentiler sekmesi ilk sürümden katalogla gelsin; yerel `.kek`, kurum kataloğu ve çevrimdışı katalog aynı inceleme akışından geçsin. (ADIM 18, 22A)
2. **Eşik en düşük, kapı sıkı.** QGIS'te iki dosya yetiyor ve yazarların %71'i tek eklentili; ama denetim yüklemeden sonra. KentOS: blueprint ve `pack` zorunlu dosyaları üretsin, aynı `verify` yazarın makinesinde, CI'da ve katalogda koşsun. (ADIM 16, 21)
3. **Her eklentinin kendi bağımlılıkları, kendi süreci.** QGIS'in 2020'den beri çözülemeyen sorunu (QEP 202 kapandı, Mayıs 2026'da eklentiler kaldırıldı). KentOS'un ayrı süreç + kilitli uv ortamı + çalışırken indirme yok kararı ekosistem büyüdükçe en çok değer üretecek karar. (ADIM 18A, 19)
4. **Temizliği host yapar.** QGIS'te `unload()` yazarın işi ve unutulunca çökme; arayüz belgesi bunu kendisi yazıyor. `ExtensionLease` ve "kaldırınca sıfır kalıntı" testi pazarlık konusu olmamalı. (ADIM 17)
5. **Hata ve çökme yalıtımı + bekçi + kalıcı hata durumu.** QGIS'te Python hatası atlatılıyor, yerel çökme uygulamayı düşürüyor, bekçi ancak sonradan; başarısız eklenti her açılışta yeniden deneniyor. KentOS: ayrı süreç, etkinleştirme bekçisi, art arda çökmede nedenli devre dışı. (ADIM 17, 19)
6. **Yazılı API kararlılığı ve araç takımından bağımsız UI.** QGIS'te Qt değişince paketlerin %58'i geride kaldı (en çok indirilenlerin çoğu geçti). KentOS: QEP 425 benzeri politika, bildirimsel `kentos.ui`, sürüm göçü denetleyicisi. (ADIM 21, 22)
7. **Uyumluluk anlamı tek kodda ve kararlı.** QGIS'te host ile depo boş `qgisMaximumVersion`'u farklı yorumladı ve düzeltme çıkış günü birleşti. KentOS: tek crate, tek fonksiyon, örtük üst sınır yok, değişiklik ADR ile. (ADIM 16)
8. **Keşif üst verisi v1'de.** Uzun açıklama, etiket, bağlantılar, değişiklik notu, yerelleştirme; manifest bilinmeyen alanı reddettiği için sonradan eklemek `schemaVersion` artışı. (ADIM 16)
9. **İhtiyacın büyüğü veri erişimi.** QGIS'te en çok indirilen 15 eklentinin en az 5'i veri/altlık erişimi, birincisi bir servis kataloğu. KentOS'un yedi katkı noktasına bildirimsel veri kaynağı/altlık katkısı eklenmesi değerlendirilsin. (ADIM 18, 16)
10. **Önce otomatik kapı, sonra insan; engelleyen kural az ve kesin.** QGIS'in 2026 akışı doğru; geriye dönük herkese açık rozet ve yanlış pozitif tepkisi kaçınılacak kısmı. Kural kaydı, gerekçeli atlama, sahibe önce gösterme. (ADIM 21, 22A)
11. **Onay kapasitesi ve güven kademeleri açık.** QGIS'te onaylayıcılar gönüllü, hafta sonu bekliyor, güvenilir statünün yolu belgesiz. AI hacminde insan onayı tek kapı olamaz: güvenilir yayımcı hızlı yolu yalnız yetenek/bağımlılık farkı olmayan sürümlerde. (ADIM 22A)
12. **Eklentiler arası bağımlılık kimlikle, aralıkla ve kurmadan önce.** QGIS görünen adla, yalnız `==` ile ve kurulumdan sonra çözüyor. (ADIM 16, 17)
13. **Deneysel kanal, emeklilik ve devir yolu baştan.** QGIS'teki 1.533 deneysel kayıt kanalın gerçekten kullanıldığını gösteriyor; "nasıl emekliye ayırırım?" sorusunun yanıtsız kalması ise belgenin eksik olduğunu. (ADIM 16, 22A)
14. **Açılış maliyeti görünür ve tembel; başsız eşdeğerlik zorunlu.** QGIS hepsini açılışta yüklüyor ve başsız çalışmayı gönüllü bırakıyor. KentOS: manifestten kayıt, ilk çağrıda süreç, eklenti başına süre görünümü; her araç `runtimes` bildirir, verify başsız kuru koşu yapar. (ADIM 17, 19, 21)
15. **Güven sinyalleri dürüst.** `trusted` ≠ kod denetimi; anonim oy istismar edildi; AI/LLM ve ağ kullanımı açıklanmalı. KentOS: onay kaydı + doğrulanmış yayımcı + üretim kökeni + `network` hedefleri; yıldız oyu yok. (ADIM 16, 21, 22A)

---

## 11. Adımlara göre öneri özeti

| Adım | Önerilen ek iş (sahip onayıyla) |
|---|---|
| **16** | Manifest v1'e isteğe bağlı: `description`, `authors`, `links` (`homepage`, `source`, `issues`), `keywords`, `changelogFile`, `license` (SPDX), `dependencies.extensions` (kimlik + aralık), önsürüm/kanal kuralı, `deprecation`, `l10n`, üst düzey `runtimes`, `network` hedefleri ve harici servis/LLM beyanı; uyumluluk anlamının tek fonksiyonu ve "üst sınır yoksa" kuralı; yetenek sözlüğünde veri kaynağı/altlık kararı; en küçük Python'suz `.kek` şablonu. |
| **17** | Etkinleştirme bekçisi ve art arda çökmede nedenli devre dışı; başarısız etkinleştirme kalıcı "hata"; eklenti bağımlılık sırası ve döngü reddi; aynı `id`'nin farklı kaynaklardan gelmesinde açık seçim; geliştirici klasörü kaynağı; kaldırmada ayar/veri kapsamı; protokol artışında kurulu listenin korunması ve uyumlu sürüm önerisi. |
| **18** | Kurulu / Bulunabilir / Güncellenebilir / Yeni / Sorunlu görünümleri, etiket süzgeci, kurmadan önce değişiklik notu ve bağımlılıklar; güven durumu her zaman görünür; eklenti başına günlük ve süre; veri kaynağı/altlık Python'suz katkısı (karar çıkarsa). |
| **18A** | Çalışma anında indirme yasağının eklenti koduna da uzatılması; kurum izinli paket listesi; platform tekerleği ve vekil sunucu matrisinin ölçümü; venv disk/süre ölçümü. |
| **19** | Her aracın `runtimes` bildirimi; Python'a nesne sahipliği verilmemesi (QEP 427 gerekçesi); ayrı süreç çökme sayacı. |
| **20** | Denetim ve ifade davranışının sürümlenmesi (QEP 425 §2.4.1 dersi). |
| **21** | Yazılı SDK kararlılık politikası (ADR); verify'da sürüm göçü kuralları ve kalıcı kural kimlikleri; çalışma anında kurulum/indirme AST kuralı; `kentos kek new/verify/pack/test` ve CI şablonu; başsız test koşucusu; sürümlü belge aynı commit'te. |
| **22** | `l10n` anahtarlarının UI/form metinlerine bağlanması; özel bileşen kaçış yolunun Python'a çizim nesnesi vermemesi. |
| **22A** | Doğrulanıyor/engellendi/doğrulandı durumları; kural kaydı, gerekçeli atlama, yeni kuralın önce sahibe gösterilmesi; güvenilir yayımcı hızlı yolu (yetenek farkı yoksa); yayımcı iletişim doğrulaması; emeklilik, devir ve "yerine geçen"; çevrimdışı/kurum kataloğu; oy yok; sağlık ölçütleri ve protokol göç oranı; AI üretim kökeni kaydı. |

---

## 12. Doğrulanamayanlar ve açık sorular

- **Kötü amaçlı eklenti olayı:** plugins.qgis.org'da belgelenmiş bir kötü amaçlı eklenti olayı bulamadım; aramalarda yalnız §7'deki 2026 tartışmaları çıktı. Doğrulanamadı.
- **Erişilemeyen blog yazıları:** "QGIS is moving to Qt6 and launching QGIS 4.0" (17 Nisan 2025), "Update on QGIS 4.0 Release Schedule and LTR Plans" (7 Ekim 2025) ve "Coming soon: QGIS Plugins Website v4.0.0" (6 Temmuz 2026) adresleri 404 döndü; içerikleri okunamadı. Site v4.0.0'ın 13 Temmuz'da yayımlandığı doğrudan doğrulanamadı (Ağustos iletileri onu yayımlanmış sayıyor, sitenin bugünkü sürümü 4.4.4).
- **QGIS 3 → 4 ayar etkileşimi:** eklenti klasörlerinin taşınmadığı koddan doğrulandı [K21]; kopyalanan `QGIS4.ini`'deki eski etkinlik ayarlarının eklenti yeniden kurulunca nasıl davrandığı doğrulanmadı.
- **Onay kuyruğu büyüklüğü ve süreleri:** "Unapproved" listesindeki 878 kayıt kuyruk değildir; gerçek bekleme süresi yayımlanmıyor.
- **E-posta doğrulanmazsa ne olur:** görevler kodda var; doğrulanmayan eklentiye uygulanan yaptırım görülmedi.
- **`hasProcessingProvider` kullanım oranı:** XML bu alanı taşımadığı için kaç eklentinin başsız çalıştığı ölçülemedi.
- **2026 büyümesinin nedeni:** aylık yeni paket sayısının artışı [H3] gözlem; AI etkisi bir onaylayıcının izlenimi [L10], ölçülmüş değil.
- **`hub.qgis.org`:** erişilemedi; QGIS Hub'ın içerik türleri ve onay süreci okunamadı.
- **Masaüstü yöneticide `trusted` gösterimi:** C++ arayüz kodunda bulamadım; Python tarafında başka bir gösterim olup olmadığını taramadım.

---

## Kaynaklar

Hepsi 13 Kasım 2026'da okundu. Parantez içindeki tarih kaynağın kendi tarihidir.

**QGIS belgeleri**
- [D1] PyQGIS Developer Cookbook 3.44 — Structuring Python Plugins: https://docs.qgis.org/3.44/en/docs/pyqgis_developer_cookbook/plugins/plugins.html
- [D2] Aynı sayfanın "testing" sürümü (Qt6 notu, `rcc -g python`): https://docs.qgis.org/testing/en/docs/pyqgis_developer_cookbook/plugins/plugins.html
- [D3] Writing a Processing plugin (3.44): https://docs.qgis.org/3.44/en/docs/pyqgis_developer_cookbook/processing.html
- [D4] QGIS Server and Python (3.44): https://docs.qgis.org/3.44/en/docs/pyqgis_developer_cookbook/server.html
- [D5] User Manual 3.44 — Plugins: https://docs.qgis.org/3.44/en/docs/user_manual/plugins/plugins.html
- [D6] Releasing your plugin (3.44): https://docs.qgis.org/3.44/en/docs/pyqgis_developer_cookbook/plugins/releasing.html
- [D7] Tasks — doing heavy work in the background (3.44): https://docs.qgis.org/3.44/en/docs/pyqgis_developer_cookbook/tasks.html
- [D8] QGIS Configuration — Deploying QGIS within an organization (3.44): https://docs.qgis.org/3.44/en/docs/user_manual/introduction/qgis_configuration.html
- [D9] Backwards Incompatible Changes (API break): https://api.qgis.org/api/api_break.html
- [D10] Gözlem: `https://docs.qgis.org/{4.0,4.2,4.4}/…` → `testing`'e, `https://docs.qgis.org/latest/…` → `3.44`'e yönleniyor; `https://qgis.org/pyqgis/4.2/` ve `/3.44/` 200, `/4.4/` 404.
- [D11] QgsProjectTrustDialog ("Since QGIS 4.0"): https://api.qgis.org/api/classQgsProjectTrustDialog.html

**QGIS kaynak kodu (`qgis/QGIS`, `master`)**
- [K1] `python/utils.py` (son commit 21 Eylül 2026): https://github.com/qgis/QGIS/blob/master/python/utils.py
- [K2] `src/app/qgspluginregistry.cpp` (17 Eylül 2026): https://github.com/qgis/QGIS/blob/master/src/app/qgspluginregistry.cpp
- [K3] `python/pyplugin_installer/installer.py` (5 Kasım 2026): https://github.com/qgis/QGIS/blob/master/python/pyplugin_installer/installer.py
- [K4] `python/pyplugin_installer/installer_data.py` (5 Kasım 2026): https://github.com/qgis/QGIS/blob/master/python/pyplugin_installer/installer_data.py
- [K5] `python/pyplugin_installer/plugindependencies.py`: https://github.com/qgis/QGIS/blob/master/python/pyplugin_installer/plugindependencies.py
- [K6] `python/pyplugin_installer/qgsplugininstallerinstallingdialog.py`: https://github.com/qgis/QGIS/blob/master/python/pyplugin_installer/qgsplugininstallerinstallingdialog.py
- [K7] `src/app/pluginmanager/qgspluginmanager.cpp`, `qgspluginmanager_texts.cpp`, `qgspluginsortfilterproxymodel.cpp`: https://github.com/qgis/QGIS/tree/master/src/app/pluginmanager
- [K8] `src/process/qgsprocess.cpp`: https://github.com/qgis/QGIS/blob/master/src/process/qgsprocess.cpp
- [K9] `src/gui/qgisinterface.h`: https://github.com/qgis/QGIS/blob/master/src/gui/qgisinterface.h
- [K10] `src/app/main.cpp` (`--noplugins`, `copyProfileNamesFromQgis3`): https://github.com/qgis/QGIS/blob/master/src/app/main.cpp
- [K11] `src/python/qgspythonutilsimpl.cpp` (`QGIS_PLUGINPATH`, bağımlılık sırası): https://github.com/qgis/QGIS/blob/master/src/python/qgspythonutilsimpl.cpp
- [K12] Commit 82c298a "Show startup profile timings in developer/debugging tools panel" (Nyall Dawson, 18 Mayıs 2020): https://github.com/qgis/QGIS/commit/82c298a70300715b59b317ff166fd39ed3fe8b37
- [K13] Bekçi commit'leri: cc650f9 (17 Mart 2016), 150b373 (21 Mayıs 2020), e1de76d (7 Kasım 2024): https://github.com/qgis/QGIS/commit/cc650f9370 · https://github.com/qgis/QGIS/commit/150b373c7a · https://github.com/qgis/QGIS/commit/e1de76d763
- [K14] Oylamanın kaldırılması: sorun #67443 (15 Eylül 2026, PSC 8 Eylül 2026 kararı) https://github.com/qgis/QGIS/issues/67443 · PR #67508 (birleşme 5 Kasım 2026) https://github.com/qgis/QGIS/pull/67508 · 2014 özelliği: commit 3b198d9 https://github.com/qgis/QGIS/commit/3b198d9000
- [K15] PR #65146 (2 Mart 2026) https://github.com/qgis/QGIS/pull/65146 · PR #65168 (6 Mart 2026) https://github.com/qgis/QGIS/pull/65168
- [K16] `.docker/README.md` (`qgis_setup.sh`, `qgis_testrunner.sh`): https://github.com/qgis/QGIS/blob/master/.docker/README.md
- [K17] Wiki — Plugin migration to be compatible with Qt5 and Qt6: https://github.com/qgis/QGIS/wiki/Plugin-migration-to-be-compatible-with-Qt5-and-Qt6
- [K18] Wiki — Plugin migration to QGIS 3: https://github.com/qgis/QGIS/wiki/Plugin-migration-to-QGIS-3
- [K19] `qgis/pyqgis4-checker` README: https://github.com/qgis/pyqgis4-checker
- [K20] Commit fdca6b2 "provide easy access to a settings node for plugins (#65984)" (6 Mayıs 2026): https://github.com/qgis/QGIS/commit/fdca6b2204
- [K21] `src/app/qgsversionmigration.cpp` (`Qgs3To4Migration::runMigration`) ve `src/app/main.cpp` (`copyProfileNamesFromQgis3`): https://github.com/qgis/QGIS/blob/master/src/app/qgsversionmigration.cpp

**Eklenti sitesi**
- [W1] Ana sayfa (sorumluluk notu, site sürümü 4.4.4): https://plugins.qgis.org/
- [W2] How to publish your plugins: https://plugins.qgis.org/docs/publish
- [W3] Plugin approval process: https://plugins.qgis.org/docs/approval
- [W4] Security Scanning: https://plugins.qgis.org/docs/security-scanning
- [W5] Migrate to QGIS 4: https://plugins.qgis.org/docs/migrate-qgis4
- [W6] plugins.xml: https://plugins.qgis.org/plugins/plugins.xml?qgis=3.44 · https://plugins.qgis.org/plugins/plugins.xml?qgis=4.0 (gzip aktarım 2.092.302 bayt, `cache-control: max-age=600`)
- [W7] Kategori sayfalarındaki "records found" sayıları: https://plugins.qgis.org/plugins/ · /plugins/stable/ · /plugins/experimental/ · /plugins/deprecated/ · /plugins/new_qgis_ready/ · /plugins/server/ · /plugins/fresh/ · /plugins/unapproved/
- [W8] Eklenti JSON'u (`downloads` = sürümlerin toplamı): https://plugins.qgis.org/plugins/QuickWKT/json
- [W9] `qgis/QGIS-Plugins-Website` kaynağı: `qgis-app/plugins/validator.py`, `security_scanner.py`, `models.py`, `views.py`, `tasks/trigger_email_confirmation.py`, `tasks/trigger_annual_reverification.py`, `docs/introduction.rst`: https://github.com/qgis/QGIS-Plugins-Website/tree/master/qgis-app/plugins

**QGIS Enhancement Proposals**
- [Q1] QEP 409 — Plugins Security and QA validator (20 Ocak 2026): https://github.com/qgis/QGIS-Enhancement-Proposals/blob/master/qep-409-plugins-security-validator.md
- [Q2] QEP 425 — Stable API policy (4–16 Haziran 2026): https://github.com/qgis/QGIS-Enhancement-Proposals/blob/master/qep-425-stable-api.md
- [Q3] QEP 426 — Demotion of DB Manager plugin to external plugin (23 Haziran 2026): https://github.com/qgis/QGIS-Enhancement-Proposals/blob/master/qep-426-demote_dbmanager.md
- [Q4] QEP 427 — Removal of deprecated Processing GUI API (29 Temmuz 2026): https://github.com/qgis/QGIS-Enhancement-Proposals/blob/master/qep-427-remove-deprecated-processing-gui-api.md
- [Q5] QEP 336 — Trusted Projects and Folders (7 Ocak 2025): https://github.com/qgis/QGIS-Enhancement-Proposals/blob/master/qep-336-trusted-projects-and-folders.md
- [Q6] QEP tartışması #202 — PIP dependencies for Python plugins (2020–2021, 31 Aralık 2024'te kapandı): https://github.com/qgis/QGIS-Enhancement-Proposals/issues/202
- [Q7] QEP tartışması #179 — Turning Plugin Management into Actual Package Management (2020, 31 Aralık 2024'te kapandı): https://github.com/qgis/QGIS-Enhancement-Proposals/issues/179
- [Q8] QEP tartışması #357 — adding external Python package dependencies (Aralık 2025): https://github.com/qgis/QGIS-Enhancement-Proposals/issues/357
- [Q9] QEP tartışması #279 — License requirements for plugins (27 Kasım 2023): https://github.com/qgis/QGIS-Enhancement-Proposals/issues/279

**Listeler, blog, değişiklik günlüğü**
- [L1] Plugin Repository Security Enhancements (Tim Sutton, 23 Nisan 2026): https://blog.qgis.org/2026/04/23/plugin-repository-security-enhancements/
- [L2] Coming soon: QGIS Plugins Website v4.0.0 (6 Temmuz 2026): https://lists.osgeo.org/pipermail/qgis-developer/2026-July/068425.html
- [L3] Rating abuse issue on the QGIS Plugins Website (19 Ağustos 2026): https://lists.osgeo.org/pipermail/qgis-developer/2026-August/068463.html
- [L4] Existing plugin versions should not be marked with "security issues" (23 Nisan 2026): https://lists.osgeo.org/pipermail/qgis-psc/2026-April/010782.html
- [L5] Security issues with plugins (24–27 Mayıs 2026): https://lists.osgeo.org/pipermail/qgis-developer/2026-May/068273.html · https://lists.osgeo.org/pipermail/qgis-developer/2026-May/068283.html
- [L6] How to deal with QGIS plugins which install additional packages (23–29 Ekim 2024): https://lists.osgeo.org/pipermail/qgis-developer/2024-October/067123.html
- [L7] Plugins' Dual QGIS 3 and QGIS 4 Compatibility (Borys Jurgiel, 24 Şubat 2026): https://lists.osgeo.org/pipermail/qgis-developer/2026-February/068089.html
- [L8] How does one deprecate a plugin correctly? (13 Şubat 2026): https://lists.osgeo.org/pipermail/qgis-developer/2026-February/068051.html
- [L9] Obtaining permission to approve plugins (12 ve 27 Ağustos 2026): https://lists.osgeo.org/pipermail/qgis-developer/2026-August/068457.html · https://lists.osgeo.org/pipermail/qgis-developer/2026-August/068500.html
- [L10] QGIS Plugins and AI Generation (Admire Nyakudya, 9 Şubat 2026): https://lists.osgeo.org/pipermail/qgis-developer/2026-February/068038.html
- [L11] Call for transparency — Generative AI plugins (17–18 Haziran 2026): https://lists.osgeo.org/pipermail/qgis-psc/2026-June/010807.html
- [L12] QGIS 4 Mac — Install additional python packages (12 Mart 2026): https://lists.osgeo.org/pipermail/qgis-user/2026-March/056174.html
- [L13] Changelog for QGIS 4.0 (çıkış 6 Mart 2026): https://changelog.qgis.org/en/version/4.0/
- [L14] QGIS Roadmap (4.0.0 6 Mart, 4.2.0 3 Temmuz, 4.4.0 30 Ekim 2026): https://qgis.org/resources/roadmap/
- [L15] QGIS Security: https://qgis.org/resources/support/security/
- [L16] Changelog girdisi — Add project load profile times to debugging tools dock (3.16): https://changelog.qgis.org/en/entry/1685

**Araçlar ve üçüncü taraf**
- [A1] qpip README (son güncelleme 25 Eylül 2026): https://github.com/opengisch/qpip
- [A2] qgis-plugin-dev-tools README (0.13.2, 24 Eylül 2026): https://github.com/nlsfi/qgis-plugin-dev-tools
- [A3] qgis-plugin-ci README ve PyPI (2.11.0, 27 Ekim 2026): https://github.com/qgis/qgis-plugin-ci · https://pypi.org/project/qgis-plugin-ci/
- [A4] pb_tool (3.5.4, 31 Ekim 2026): https://pypi.org/project/pb_tool/ · pytest-qgis (5.0.2, 22 Ekim 2026): https://pypi.org/project/pytest-qgis/
- [A5] Oslandia QGIS Plugin Templater: https://oslandia.gitlab.io/qgis/template-qgis-plugin/
- [A6] Riverbank — Differences Between PyQt6 and PyQt5: https://www.riverbankcomputing.com/static/Docs/PyQt6/pyqt5_differences.html
- [A7] Docker Hub `qgis/qgis` (839.792 çekme, 12 Kasım 2026): https://hub.docker.com/r/qgis/qgis
- [A8] QuickMapServices README: https://github.com/nextgis/quickmapservices

**Benim hesaplarım ([W6] ve [W7] verisinden, 13 Kasım 2026)**
- [H1] `plugins.xml?qgis=3.44`: 4.461 kayıt, 3.927 paket (zip adı köküyle), 1.143 deneysel ve 76 kullanımdan kaldırılmış kayıt; 2.191 farklı `uploaded_by`, 1.549'u tek paketli; 261 paketin `trusted` değeri `True`; `repository` adreslerinin 4.108'i GitHub; paket başına en yüksek `downloads` değerlerinin toplamı 103.174.517, medyan 2.348, ilk %1'in payı %51,6, ilk %10'un %84,2; 13 paket ≥ 1 milyon, 145 paket ≥ 100 bin; sıfırdan farklı `average_vote` yok; `changelog`, `author_email`, `category` ve `plugin_dependencies` öğesi yok, `external_dependencies` hep boş; son `update_date` yılına göre 616 paket (%15,7) 2022 ya da önce, 2.342 paket (%59,6) 2026; sıralama örnekleri: QGIS Resource Sharing 40., Plugin Reloader 44., Plugin Builder 3 95., qpip 119.
- [H2] 3.44 ve 4.0 XML karşılaştırması: 4.0'da 1.732 kayıt / 1.654 paket (3.927'nin %42'si); en çok indirilen ilk 20/50/100/500 paketin 18/41/76/236'sı 4.0'da; indirme ağırlıklı %68,4; yalnız 4.0'da olan 100 paket; ilk 20'de hazır olmayanlar OpenLayers Plugin ve Freehand raster georeferencer.
- [H3] Paket başına en erken `create_date` yılı: 2023 → 259, 2024 → 297, 2025 → 587, 2026 (13 Kasım'a kadar) → 1.567; 2026 aylık: Ocak 73, Şubat 95, Mart 125, Nisan 113, Mayıs 136, Haziran 143, Temmuz 185, Ağustos 201, Eylül 205, Ekim 213. Yalnız 3.44 ile uyumlu paketler sayıldığı için eski yıllar eksik görünür (hayatta kalma yanlılığı).
- [H4] En çok indirilen 15 paketin benim sınıflandırmam: veri/altlık erişimi — NextGIS QuickMapServices, OpenLayers Plugin, QuickOSM, Google Earth Engine, OSMDownloader; dışa aktarma/biçim — qgis2web, Qgis2threejs, KML Tools; saha senkronu — QField Sync; analiz/araç — Semi-Automatic Classification Plugin, HCMGIS, Lat Lon Tools, Profile tool, mmqgis, Street View.
