# Eklenti ekosistemleri: Blender, VS Code, ArcGIS Pro, JupyterLab ve Obsidian'dan `.kek` için dersler

Tarih: 13 Kasım 2026. Konu: KentOS'un `.kek` eklenti sistemi ([plan/eklenti.md](../../plan/eklenti.md): §1 ilkeler, §3 paket standardı, §4 ADIM 16–22A) tasarlanırken QGIS dışındaki büyük ve sağlıklı eklenti ekosistemlerinden neyin alınacağı, neyin uyarlanacağı ve nelerden kaçınılacağı. QGIS ayrı bir araştırmanın konusudur; burada incelenmedi.

## Yöntem ve sınırlar

- Bütün kaynaklar **13 Kasım 2026**'da okundu. Önce resmî belge okundu; resmî belgenin sustuğu yerde güvenlik araştırması ya da haber kullanıldı ve "ikincil" diye işaretlendi. Kaynak kısaltmaları ([B1], [V7] …) en sondaki listededir.
- Okunan sürümler: Blender kılavuzu **5.3** ("latest"), VS Code belgeleri (sayfa tarihi 11/11/2026), ArcGIS Pro SDK **3.7** (wiki tarihi 20/04/2026, son düzenleme 13 Mayıs 2026), JupyterLab "latest" (4.6→4.7 geçiş rehberi), Obsidian geliştirici belgeleri (GitHub kaynağı; topluluk dizini bölümü 7 Ağustos 2026 commit'i), uv "latest".
- projects.blender.org HTML sayfaları Cloudflare engeli verdiği için Blender kaynak kodu ve takip kayıtları ([B4][B5][B26][B27][B28]) Gitea API'si üzerinden okundu.
- **İki sayımı bu araştırmada biz yaptık** (13 Kasım 2026 anlık görüntüsü): Blender onay kuyruğundaki durum sayıları [B19] ve VS Marketplace'in `RemovedPackages.md` listesinin yıl/tür dağılımı [V14].
- Hiçbir eklenti kurulmadı ya da çalıştırılmadı; davranışlar belgeden yazıldı. Resmî kaynakta bulunamayan her şey "doğrulanamadı" diye işaretlendi ve §8'de toplandı.

## Özet

- **Beyan edilen izin her ekosistemde "bilgi"dir, güvenlik sınırı değil.** Blender izinleri için "not a security-oriented feature" der [B11]; Obsidian eklentileri "cannot reliably restrict" [O4]; VS Code'da izin yoktur, extension host VS Code ile aynı yetkidedir [V7]. KentOS'un aracı (broker) üzerinden geçen işlemlerde yeteneği **gerçekten zorlayabilmesi** bu ekosistemlerin hiçbirinde olmayan bir üstünlüktür; aracıyı atlayan doğrudan dosya/ağ çağrısı için ise dürüst etiket gerekir (plan PY-RUN-19/20 zaten böyle).
- **"Yalnız ilk sürümü incele" modeli sürdürülemedi.** Obsidian'da sonraki sürümler incelenmiyordu ve kuyruk büyüdü; Mayıs 2026'da her sürüm için otomatik taramaya ve "karne"ye geçildi, birkaç günde 2.300'den fazla bekleyen başvuru işlendi [O6]. Blender'da 2024 tasarım kaydına göre yalnız ilk sürüm modere ediliyor, sonraki sürümler "automatically marked as approved" [B26]; 2025'te bu yayın sonrası denetimi "more realistic" yapmak için fark görüntüleyici aranıyor [B12]. KentOS'un "her sürüm onaydan geçer" kararı doğru; insan incelemesi **farka ve riske** odaklanmazsa Obsidian kuyruğu tekrar eder ve AI üretimi bu yükü büyütür.
- **Otomatik tarama şart ama yetmez, üstelik "açık kalmamalı" (fail-closed).** VS Marketplace çok motorlu tarama, kum havuzunda dinamik algılama ve sır taraması yapıyor [V7]; yine de 2026'da (12 Kasım'a dek) 2.140 eklenti kaldırıldı ve en büyük grup **taklit** (1.109) [V14]. Open VSX'in yeni tarayıcısı yük altında taranmamış paketi "PASSED" işaretledi [V26].
- **Bağımlılık paylaşmak sessiz kırılmadır.** Blender bütün eklentilerin tekerleklerini tek `site-packages`'a açar ve aynı paketin en yüksek sürümünü tutar [B4]; ArcGIS Pro eklentileri varsayılan olarak tek `AssemblyLoadContext`'i paylaşır, yan yana yükleme 3.5'ten beri isteğe bağlıdır ve varsayılanı kapalıdır [A2]. KentOS'un eklenti başına uv ortamı bunu baştan çözer; eksik halka, bildirilen platformlar için tekerlek kapsamının kilitte doğrulanması (`required-environments`) ve genel katalogda kaynaktan derlemenin kapatılmasıdır [P1][P4].
- **Tembel etkinleştirme bildirimsel katkılardan türer.** VS Code 1.74'ten beri komut ve dil katkısından etkinleşme olayını kendisi çıkarır, `*`'ı caydırır [V2]; ArcGIS modülleri varsayılan olarak tam zamanında (JIT, `autoLoad="false"`) yüklenir, düğmeler `condition` ile kod yüklenmeden görünür/etkin olur [A1]. KentOS'ta `handler`'ın içe aktarmadan bilinmesi (PY-KEK-11) bunun temelidir.
- **Uyumluluk aralığı, deneysel API kapısı ve yükseltme aracı birlikte gelir.** VS Code `engines.vscode` ile uyumsuz istemciye sürüm vermez, deneysel API yalnız Insiders'ta çalışır ve yayımlanamaz [V5][V6]; ArcGIS ana sürümde kırar, küçük sürümlerde ileri uyumludur [A2][A5]; Blender `blender_version_max`'ın sonradan platformda tanımlanmasına izin verir [B1]; JupyterLab ana sürüm geçişi için yükseltme betiği verir [J3].
- **Kurum katalogları ikinci büyüme motorudur.** Blender statik/dinamik depo, erişim jetonu, engel listesi ve salt okunur "System" deposu sunar [B14][B15][B16][B17]; VS Code `extensions.allowed` ilkesi ve özel market [V12]; JupyterLab izin/engel listeleri [J1]; ArcGIS yönetici klasörleri ve imza düzeyleri [A2][A3]. Belediye BT'si için hepsi doğrudan karşılık bulur.
- **Projeyi açmak kod çalıştırmamalı.** Kötü amaçlı `.blend` dosyaları "Auto Run" açıkken StealC yükledi [B22]; Obsidian'da paylaşılan kasa ve eklenti eşitlemesi PHANTOMPULSE RAT'ına kapı oldu [O10]; VS Code bu yüzden Workspace Trust/Restricted Mode'u kurdu [V8].
- **AI üretimi için en büyük kaldıraç: yerelde çalışan, bulutla aynı doğrulayıcı ve kapalı sözlükler.** Obsidian incelemenin denetimlerini yerelde çalıştıran resmî ESLint eklentisi ve sürüm çıkarmadan "Review branch" taraması sunuyor [O3]; Blender `extension validate` veriyor [B14]; VS Code ajan araçlarını manifestte `modelDescription`/`inputSchema` ile bildirtiyor [V1] ve belgesini `llms.txt` ile sunuyor [V20]. LLM'lerin uydurduğu paket adları (ticari modellerde ortalama en az %5,2, açık modellerde %21,7) bağımlılık denetimini zorunlu kılıyor [P10].

## Karşılaştırma (yalnız kaynakla doğrulanan boyutlar)

| Boyut | Blender | VS Code | ArcGIS Pro | JupyterLab | Obsidian | KentOS planı |
|---|---|---|---|---|---|---|
| Paket | `.zip` + `blender_manifest.toml` [B1] | `.vsix` + `package.json` [V6] | `.esriAddinX` + `Config.daml` [A1][A4] | pip/conda paketinde prebuilt JS [J2] | GitHub sürümünde `main.js` + `manifest.json` [O1] | `.kek` ZIP + `manifest.json` + `integrity.json` |
| Bildirimsel arayüz | Manifestte yok; tipler Python'da kaydedilir [B6] | 38 katkı noktası + `when` [V1] | DAML sekme/grup/düğme + koşul [A1] | incelenmedi | belirtilmemiş | `contributions` + `ui/*.panel.json` |
| Tembel yükleme | belgede bulunamadı | etkinleşme olayları [V2] | JIT modül, `loadOnClick`, koşul [A1] | — | — | PY-RUN-05 |
| Süreç | — | UI'dan ayrı extension host [V4][V9] | — | tarayıcı + sunucu [J1] | — | eklenti sürümü başına süreç |
| İzin | beyan + gerekçe, zorlanmaz [B1][B11] | yok; yayıncı güveni + Workspace Trust [V7][V8] | imza ve yükleme düzeyi [A3] | kurulumda uyarı onayı [J1] | yok; Restricted mode [O4] | yetenek sözlüğü v1 + aracı |
| İnceleme | ilk sürüm insan; sonraki sürümler kendiliğinden onaylı, yayın sonrası denetim [B26][B12] | her sürümde otomatik tarama [V7] | merkezi inceleme yok; kurum dağıtır [A4] | yok (PyPI) | Mayıs 2026'dan beri her sürüm otomatik + karne [O4][O6] | her sürüm insan onayı (22A) |
| Kurum kataloğu | uzak depo + jeton + System [B14][B17] | allowed + özel market [V12] | well-known klasör + kayıt defteri [A2] | izin/engel listesi [J1] | ekip içi özel eklenti planlı [O6] | kiracı kataloğu (PY-CLOUD-02) |
| Python/bağımlılık | platform tekerlekleri, paylaşılan `site-packages` [B3][B4] | — | paylaşılan ALC, yan yana isteğe bağlı [A2] | `sharedPackages` tekilleştirme [J2] | — | eklenti başına uv ortamı (PY-UV-12) |

---

## 1. Blender Extensions (4.2+, extensions.blender.org)

### 1.1 Manifest: `blender_manifest.toml`

Bir eklenti, içinde manifest bulunan bir `.zip` arşividir; türüne göre add-on (`__init__.py` gerekir), tema (`.xml`) ya da 5.3'ten beri varlık kitaplığı (asset library) olabilir [B1]. `schema_version` bugün `"1.0.0"`; şemanın değişiklik günlüğündeki tek yenilik 5.3'te gelen `asset-library` türü, `[asset_library]` tablosu ve `scripts` iznidir [B2].

| Alan | Zorunlu | Kural (kaynak [B1][B2]) |
|---|---|---|
| `schema_version` | evet | Dosya biçiminin iç sürümü, `"1.0.0"` |
| `id` | evet | Eklentinin benzersiz kimliği (`"my_example_extension"`) |
| `version` | evet | Anlamsal sürüm |
| `name` | evet | Tam ad |
| `tagline` | evet | Tek satır, en çok 64 karakter, noktalama ile bitemez |
| `maintainer` | evet | Bakımcı (`"Developer Name <e-posta>"`) |
| `type` | evet | `"add-on"`, `"asset-library"`, `"theme"` |
| `blender_version_min` | evet | En az `4.2.0` |
| `blender_version_max` | hayır | Desteklenmeyen ilk sürüm (dışlayıcı); "can be omitted and defined later on the extensions platform if an issue is found" |
| `license` | evet | SPDX listesi, `"SPDX:GPL-3.0-or-later"` biçiminde |
| `copyright` | hayır | `"Year Name"` ya da `"Year-Year Name"` |
| `website` | hayır | Web adresi; örnek dosyada `support` (hata izleyici) da var ama alan listesinde geçmiyor |
| `tags` | hayır | Türe göre kapalı liste [B21]; `build`/`validate` `--valid-tags` ile denetler [B14] |
| `platforms` | hayır | `windows-x64`, `windows-arm64`, `macos-x64`, `macos-arm64`, `linux-x64`; yoksa bütün işletim sistemleri |
| `wheels` | hayır | Tekerleklerin göreli yolları, ileri bölü ile |
| `[permissions]` | hayır | Add-on: `files`, `network`, `clipboard`, `camera`, `microphone`; varlık kitaplığı: `scripts`; tema: yok. Her izne tek cümlelik, en çok 64 karakterlik, sonda noktası olmayan gerekçe |
| `[asset_library]` | varlık kitaplığında zorunlu | `remote_url`; isteğe bağlı `[asset_library.auth]` (`required`, `auth_method = "TOKEN"`) |
| `[build]` | hayır | `paths` ya da gitignore uyumlu `paths_exclude_pattern`; varsayılan `__pycache__/`, `.git`, `*.zip` |
| `[build.generated]` | ayrılmış | `--split-platforms` derlemesi `platforms` değerini kendisi yazar; kaynak manifestte bulunmaz |

Manifestteki her değer dolu olmalıdır: boş metin ya da boş liste yazmak yerine alan hiç yazılmaz [B1]. Bu katı kural ajan için iyidir: "varsayılanı bırakılmış isteğe bağlı alan" moderasyonun eskiden sık gördüğü bir sorundu [B8].

### 1.2 Python bağımlılıkları: platform başına tekerlek

- Kılavuz tekerlekleri `./wheels/` altında ister; tekerlek "PyPI'den değiştirilmeden" alınmalı ve kendi bağımlılıklarını içermelidir. Derlenmiş içerikli paketlerde her platform için ayrı tekerlek indirilir (`pip download pillow --dest ./wheels --only-binary=:all: --python-version=3.13 --platform=manylinux_2_28_x86_64` vb.) ve manifestin `wheels` listesine yazılır [B3].
- Paket boyunu küçültmek için `blender --command extension build --split-platforms` her platforma ayrı zip üretir (`…-windows_x64.zip`); `platforms` alanında olmayan platforma zip çıkmaz [B3].
- **Kurulumda ne olduğu (koddan):** `main` dalındaki `wheel_manager.py`'ye göre bütün eklentilerin tekerlekleri tek bir yere açılır (`~/.config/blender/4.2/extensions/.local/lib/python3.11/site-packages` örneği). Aynı taban adlı tekerleklerden **eski sürüm atlanır, yenisi açılır**; kod yorumu bunun "not fool-proof" olduğunu ve tekerlek içi dosya çakışmalarının mümkün olduğunu yazar [B4]. 2024'teki takip kaydında plan "çakışan sürüm kurulumu hata verir" idi [B5]; bugünkü kod hata vermek yerine yeni sürümü seçiyor. Sonuç (koddan çıkarım): eski sürümü bekleyen eklenti **sessizce** başka sürümle çalışır.
- Kılavuz, çakışmadan kaçınmanın yolu olarak saf Python bağımlılığı alt modül olarak gömmeyi (vendorize) de anar [B6]. Hizmet koşulları ise "Source code of the extension must be fully written in Python. Depending on external, pre-compiled packages is not allowed." der [B10]; PyPI tekerleklerine verilen izinle bu madde arasındaki sınır metinde açık değil (doğrulanamadı).

### 1.3 İzinler ve çevrimiçi erişim

- İzin, "geliştiricinin niyetini şeffaf kılma" aracıdır, "not a security-oriented feature" [B11]. Blender'ın genel "Allow Online Access" ayarı (`bpy.app.online_access`, `--offline-mode`/`--online-mode`) vardır, ama kılavuzun kendi cümlesiyle "Blender cannot prevent third-party add-ons from violating this rule" [B6]. "Get Extensions" ilk açıldığında Blender çevrimiçi erişim izni sorar [B20].
- Hizmet koşulları: ağa bağlanan eklenti `network` iznini gerekçesiyle ister, kullanıcı ayarına uyar, kullanıcının izni olmadan veri göndermez, uzaktan kod yükleyip çalıştırmaz, başka eklentiye bağımlı işlev sunmaz (5.3) [B10].
- **Arayüzde gösterim (koddan):** izin gösterimi 4.2 beta görev listesindeki 18 Mayıs 2024 commit'iyle (`e0b47f78e8`, "Extensions: show permissions, validate permission values") geldi [B28]. Bugünkü `main` kodu izinleri eklentinin ayrıntı panelinde yalnız **adlarıyla** yazar ("Files, Network"; yoksa "No permissions specified"); gerekçe metni henüz gösterilmez ("we will want to show the dictionary values eventually"). Güncellemenin yeni izin getirdiğini kullanıcıya göstermek kodda açık bir iş olarak duruyor: "users need to be aware when upgrading an extension results in it having additional permissions … TODO(@ideasman42): handle permissions on upgrade" [B27]. Web sitesindeki gösterim doğrulanamadı.

### 1.4 İnceleme ve moderasyon

- Yükleme Blender ID ister; eklenti incelemeye alınır ve moderatör onayından sonra yayımlanır [B1]. Kuyruk durumları: Awaiting Review, Awaiting Changes, Approved, Declined [B13][B19]. Durum kendiliğinden değişmez, yazar "Awaiting Review"a kendisi çeker [B13].
- Moderatör denetim listesi kod okumaya dayanır: hizmet koşulları, eklentinin gerçekten çalışması ("Most often the add-on is just not working, since it was poorly converted into an extension without testing"), izinlerin doğruluğu, kendi güncelleyicisi olmaması, pip yerine tekerlek, `bpy.app.online_access` denetimi, `__file__` ile `sys.path`'e ekleme ya da eklenti dizinine yazma, `sys` modülünü değiştirme, `bl_ext` metni, arka plan kipinde `keyconfigs.addon`'un `None` olabilmesi, `requests`/`urllib` içe aktarma ve `http` metinleri ile `ruff check` [B8]. Ret ve düzeltme istekleri hazır yanıtlarla yazılır (düşük değer, bozuk eklenti, çevrimiçi ayarın yok sayılması, güncelleyici, dış bileşen, `__pycache__`, ters bölü yollar, sabit veri adları, `__name__` ile tercih, modül yükleme manipülasyonu, `threading` kaynaklı çökme) [B9].
- **Sonraki sürümler:** Platformun 2024 tasarım kaydı bunu "accepted by design" diye yazar: "moderation is required only for an initial version, all subsequent versions are automatically marked as approved and become listed"; yayındaki sorun kullanıcıların "abuse report"uyla bildirilir; dosya doğrulamasının şüpheli bulduğu yükleme moderatöre görünür olmalıdır [B26]. 24 Nisan 2025 notu durumun sürdüğünü gösterir: "Once an extension is 'Approved', no actions on the author's side will be required" ve "post-publish moderation of subsequent uploads"ı daha gerçekçi yapmak için fark görüntüleyici aranır. Aynı not iki ölçüm verir: eklentilerin üçte biri ilk incelemeden sonra ikinci incelemeyi istemiyor; birçok yazar ilk moderatör yorumundan sonra dönmüyor; şikâyet kutusu yazara gitmesi gereken hata bildirimleriyle doluyor [B12]. Kullanıcıya dönük resmî belgede bu kural yazılı değil.
- **13 Kasım 2026 kuyruk sayımı** [B19]: toplam 2.396 kayıt; Awaiting Review 7, Awaiting Changes 426, Approved 1.516, Declined 447. İnceleme bekleyen sayısı düşük, düzeltme bekleyen sayısı yüksek: 2025 notundaki "yazar dönmüyor" gözlemiyle uyumlu.
- Blender çekirdeği için AI katkı politikası, inceleyici zamanını korumak için yazarın tam sorumluluk almasını ister; politika açıkça "It does not apply to Blender extensions" der [B25].

### 1.5 Depolar ve kurum katalogları

- Varsayılan kurulum bir uzak depo (resmî platform) ve iki yerel depoyla gelir; uzak depo eklenebilir, "Check for Updates on Startup" ve erişim jetonu ayarları vardır. "Install from Disk" ile kurulan paket yerel depoya girer ve güncelleme almaz [B17]. Eklentiler kendiliğinden güncellenmez; kullanıcı denetler [B20].
- **Statik depo:** `blender --command extension server-generate --repo-dir=/yol` klasördeki zip'lerden `index.json`, `--html` ile de göz atılabilir bir `index.html` üretir; indirme adresine `repository`, `platforms`, `blender_version_min` sorgu bilgileri eklenebilir [B15].
- **Dinamik depo:** Blender listeyi isterken `platform` ve `blender_version` gönderir, sunucu yalnız uyumlu son sürümleri döner; jeton `Authorization: Bearer` başlığıyla gider. Listede `"blocklist": []` alanı vardır [B16]; `blender_repo.toml` içinde `[[blocklist]]` (`id`, `reason`) ile engellenen eklentiler tanımlanır [B14].
- **System deposu:** Blender dışında yönetilen salt okunur depo; çevrimdışı ya da denetimli ortamda bütün kullanıcılara eklenti seti dağıtmak için [B14][B17][B23]. Eklentiler bu yüzden kendi dizinine yazamaz, kullanıcı verisi için `bpy.utils.extension_path_user(__package__, create=True)` kullanır; bu dizin güncellemede korunur, kaldırmada silinir [B6][B7].
- Kurum içi ya da kapalı grup eklentileri platforma kabul edilmez (hizmet koşulları 3.4) [B10]; ticari eklentiler için kendi deposunu kurma yolu açıktır [B11].

### 1.6 Araçlar

`blender --command extension` alt komutları: paket için `list`, `sync`, `update`, `install`, `install-file`, `remove`; depo için `repo-list`, `repo-info`, `repo-add` (`--url`, `--access-token`, `--source USER|SYSTEM`), `repo-remove`; üretim için `build` (`--split-platforms`, `--valid-tags`), `validate` (klasör ya da açılmamış zip) ve `server-generate` [B14]. Platforma CI'dan yükleme tek `curl` çağrısıdır (`POST /api/v1/extensions/$EXTENSION/versions/upload/`, taşıyıcı jeton) [B18]. Hazır yanıtlardan biri `__pycache__` gibi baytkod artıklarını önlemek için `build` kullanmayı önerir [B9].

### 1.7 Eski eklentilerin göçü (4.2)

Eski yöntem "deprecated" sayıldı ama "Install legacy Add-on" ile kurulabiliyor. Göç adımları: manifest yaz, `bl_info`'yu kaldır, modül adı yerine `__package__` kullan, içe aktarmaları göreli yap, bağımlılıkları tekerlekle paketle. Farklı depolardaki aynı adlı eklentiler çakışmasın diye modül adı `bl_ext.{repository_module_name}.{id}` oldu; tercihlere `__name__` ile erişen eklentiler kırıldı [B6]. 4.1'e kadar Blender'la gelen eklentiler platforma taşındı; bakımcısı olmayan popüler olanlar projects.blender.org/extensions altında tutuluyor [B24].

### 1.8 Olay: belge içinde gömülü kod

Kasım 2025'te Morphisec, CGTrader gibi ücretsiz model sitelerine konan `.blend` dosyalarının "Auto Run Python Scripts" açıkken gömülü `Rig_Ui.py` betiğiyle StealC V2 yüklediğini raporladı; öneri Auto Run'ı güvenilmeyen dosyada kapalı tutmak [B22] (ikincil). Bu bir eklenti olayı değil, ama "dosyayı açmak kod çalıştırır" sınıfının ders kitabı örneğidir.

**Blender'ı ne büyütüyor, ne koruyor:** uygulama içinde arama ve web'den sürükle-bırak kurulum [B17][B20]; kısa ve katı bir manifest ile `build`/`validate`/`server-generate` araçları [B14]; hazır yanıtlı insan moderasyonu [B9]; kendi deposunu kurma kolaylığı [B15][B16]. Zayıf yanları: zorlanmayan izinler, paylaşılan `site-packages`, sonraki sürümlerin büyük olasılıkla yayın sonrası denetimi ve ilk yorumdan sonra kaybolan yazarlar [B4][B11][B12].

---

## 2. Visual Studio Code

### 2.1 Bildirimsel katkılar

`package.json` içindeki `contributes` alanında 38 katkı noktası listelenir: `commands`, `menus`, `submenus`, `views`, `viewsContainers`, `viewsWelcome`, `configuration`, `keybindings`, `walkthroughs`, `icons`, `themes`, `languages`, `languageModelTools`, `chatSkills` vb. [V1]. Menü öğeleri `when` koşuluyla görünür; komutun `enablement` koşulu bütün menülere ve kısayollara uygulanır; komutlar varsayılan olarak Komut Paleti'nde çıkar [V1]. Extension host, eklentilerin "Impacting startup performance", "Slowing down UI operations" ve "Modifying the UI" yapmasını engeller: eklenti arayüzü ancak katkı noktaları ve API ile değiştirebilir [V4].

### 2.2 Tembel etkinleştirme ve başlangıç başarımı

- `activationEvents`: `onCommand`, `onLanguage`, `onView`, `workspaceContains`, `onUri`, `onLanguageModelTool`, `onStartupFinished`, `*` vb. **1.74'ten beri** eklentinin katkıda bulunduğu komut ve diller için ayrıca etkinleşme olayı yazmak gerekmez. `*` yalnız başka hiçbir bileşim işe yaramıyorsa önerilir; `onStartupFinished` "will not slow down VS Code startup" [V2].
- Teşhis araçları: "Developer: Show Running Extensions" (extension host CPU profili), "Startup Performance" zamanlayıcıları, `code --prof-startup` ve "Help: Start Extension Bisect" [V10]. Bisect, eklentileri yarıya bölerek sorunluyu `log2(N)+1` adımda bulur; 24 eklentide 4–5 adım [V9].
- Paketleme önerisi başarım içindir: "Loading 100 small files is much slower than loading one large file" [V19].
- Eklenti başına sayısal başlangıç bütçesi (ör. "X ms'yi aşan eklenti uyarılır"): okunan belgelerde **bulunamadı**.

### 2.3 Extension Host: neyi korur, neyi korumaz

- Host'lar konuma ve çalışma zamanına göre ayrılır: local (Node.js), web (tarayıcı WebWorker), remote; `extensionKind` tercih edilen yeri bildirir [V4]. Belge eklenti başına süreç tanımlamaz; aynı host'taki eklentiler süreç düzeyinde birbirinden ayrılmaz (belgeden çıkarım).
- Korunan şey arayüzdür: "we designed VS Code with 'bad' extensions in mind and therefore run them in a separate process. This isolation guarantees that VS Code keeps running, the cursor always blinks, and that you can always save your work." [V9]
- Korunmayan şey kullanıcının sistemidir: "The extension host has the same permissions as VS Code itself" — dosya okuyup yazabilir, ağ isteği yapabilir, dış süreç çalıştırabilir [V7].

### 2.4 API kararlılığı ve uyumluluk

- `engines.vscode` (`"^1.8.0"`) eklentinin yalnız ihtiyaç duyduğu API'yi içeren istemcilere kurulmasını sağlar; Insiders için tarih eki (`^1.56.0-20210428`) desteklenir [V6].
- **Proposed API:** yalnız Insiders dağıtımında, `package.json`'da `"enabledApiProposals"` ile ve ilgili `vscode.proposed.<ad>.d.ts` dosyası projeye kopyalanarak kullanılır; "should not be used in published extensions"; paylaşmak için VSIX + `--enable-proposed-api=<kimlik>` gerekir. Gerekçe: "once we introduce an API, we cannot easily change it anymore" [V5].
- Ön sürüm kanalı: `vsce publish --pre-release`; semver ön sürüm etiketi desteklenmediği için kararlı sürümde çift, ön sürümde tek ara sayı önerilir [V6]. Kullanıcı "Install Another Version" ile belirli sürüme dönebilir [V11].

### 2.5 Marketplace: yayıncı, tarama, imza, engel listesi

- **Yayıncı güveni:** 1.97'den beri üçüncü taraf yayıncının eklentisi ilk kurulurken yayıncıya güven onayı istenir; eklenti paketine ya da bağımlılıklara güvenmek onların yayıncılarına da güvenmektir; komut satırından kurulumda yayıncı kendiliğinden güvenilir olmaz [V7].
- **Doğrulanmış yayıncı:** alan adına DNS TXT kaydı; yayıncının en az 6 aydır eklentisi olmalı, alan adı en az 6 aylık olmalı; alt alan adı kabul edilmez; HTTPS ve HEAD isteğine 200 gerekir; ekip 5 iş günü içinde inceler; görünen ad değişince rozet düşer [V6].
- **Korumalar** [V7]: her yeni eklenti ve her güncelleme birkaç antivirüs motoruyla taranır, temiz çıkana kadar yayımlanmaz; kum havuzunda (clean room VM) dinamik algılama; olağandışı kullanım izleme; resmî yayıncı ve popüler eklenti adlarının çalınmasını önleme; doğrulanan kötü amaçlı eklenti engel listesine girer ve kuruluysa VS Code onu **kendiliğinden kaldırır**; Marketplace her paketi imzalar, VS Code kurulumda doğrular (`extensions.verifySignature`) [V11]; sır taraması (ör. Azure DevOps PAT) yayını engeller, `vsce` paketlemede `.env` dosyalarını tarar. Şikâyete ilk yanıt bir iş günü içinde.
- Microsoft'un 11 Haziran 2025 yazısı: yayından kısa süre sonra yeniden tarama ve dönemsel toplu tarama; tarayıcı ya da dinamik algılamanın işaretlediği paketler yanlış pozitif olmasın diye güvenlik mühendislerince elle incelenir; 2025'te o güne dek 136 eklenti incelenip 110'u kaldırıldı; topluluk 100'den fazla eklenti bildirdi; yol haritasında üçüncü taraf yayıncı imzası ve "obfuscation, remote code execution" gibi riskli davranışların kullanıcıya gösterilmesi var [V13].
- **Kamuya açık kaldırma kaydı** (`RemovedPackages.md`, 26 Şubat 2025'te başlıyor) [V14] — bizim sayımımız: 2025'te 409 kaldırma (Malware 259, Impersonation 120, Impersonation+Malware 5, diğer 25); 2026'da 12 Kasım'a dek **2.140** kaldırma: Impersonation **1.109**, Malware 538, Untrustworthy 329, Spam 142, diğer 22.
- `vsce` kullanıcı SVG'li simgeyi ve güvenilmeyen SVG rozetleri reddeder; README görselleri `https` olmalıdır [V6].
- Yayından kaldırma iki türlüdür: "Unpublish" (istatistik korunur) ve "Remove" (geri alınamaz; ad, asıl yayıncı için bile **kalıcı olarak ayrılır**). Tek sürüm silinebilir ama sürüm numarası tekrar kullanılamaz. "Deprecated" eklentiye alternatif eklenti ya da ayar gösterilir, VS Code "Migrate" düğmesi sunar [V6].
- **Yayın kimliği:** Azure DevOps genel PAT'leri 1 Aralık 2026'da emekliye ayrılıyor; GitHub Actions için OIDC tabanlı "Trusted Publishing" (`vsce publish --oidc`; Marketplace kimliği 15 dakikada sona erer, her OIDC jetonu bir kez takas edilir) ya da Azure Pipelines için Entra ID [V6].
- **Gecikmeli otomatik güncelleme:** 1.123 (3 Haziran 2026) yeni sürümün otomatik kurulumunu yayından iki saat sonraya erteledi, "problematic or potentially compromised releases"a karşı; Microsoft, GitHub, OpenAI gibi güvenilir yayıncılara uygulanmaz [V15]. 1.125 (17 Haziran 2026) `extensions.autoUpdateDelay` ayarını getirdi, varsayılan iki saat [V16]. Güncel belge (11/11/2026) varsayılanı **12 saat** yazıyor ve kurumların ilkeyle yönetebildiğini söylüyor [V11]; varsayılanın hangi sürümde değiştiği doğrulanamadı.

### 2.6 Bağımlılık, paket kümesi, platform paketi

`extensionDependencies` işlevsel bağımlılık, `extensionPack` ise birlikte kurulan küme içindir; paket kümesinin üyelerine işlevsel bağımlılığı olmamalıdır [V3]. Yerel ikili içeren eklentiler platforma özel VSIX olarak yayımlanır (`vsce publish --target win32-x64 …`; hedefler `win32-x64`, `win32-arm64`, `linux-x64`, `linux-arm64`, `linux-armhf`, `alpine-x64`, `alpine-arm64`, `darwin-x64`, `darwin-arm64`, `web`); hedefsiz paket yedek olarak kullanılır [V6].

### 2.7 Workspace Trust ve Restricted Mode

Amaç, bir klasörü açmanın istem dışı kod çalıştırmasını önlemek. Eklenti `capabilities.untrustedWorkspaces` ile `{ supported: true }`, `{ supported: false, description }` ya da `{ supported: 'limited', description, restrictedConfigurations }` bildirir. **Hiç bildirmeyen eklenti Restricted Mode'da devre dışıdır.** API: `workspace.isTrusted`, `onDidGrantWorkspaceTrust`; `when` bağlam anahtarı `isWorkspaceTrusted`; çalışma alanının tanımladığı tehlikeli ayarlar `restrictedConfigurations` ile güvenilene dek okunmaz [V8].

### 2.8 Kurum: izin listesi ve özel market

`extensions.allowed` (1.96+) yayıncıya, eklenti kimliğine, tek tek sürümlere (`"3.0.0"`, aralık yok), platforma (`"5.0.0@win32-x64"`) ya da `"stable"`e göre izin verir/engeller; engellenen kurulu eklenti devre dışı kalır; `AllowedExtensions` ilkesi kullanıcı ayarını ezer. Belge, yerel VSIX'te bu kuralların paketin kaynağını doğrulamadığını ve çalışma zamanı yetkisini kısmadığını açıkça yazar. Özel market: kendi altyapısında barındırma, genel Marketplace'ten süzülmüş "upstreaming", havasız ağ için "rehosting", durumsuz Docker kabı; bugün GitHub Enterprise müşterilerine açık. Windows'ta `bootstrap\extensions` ile önceden kurulum [V12].

### 2.9 Araçlar ve ajana uygunluk

`yo code` iskele üreticisi ve F5 ile açılan "Extension Development Host" [V17]; `@vscode/test-cli` + `@vscode/test-electron` ile entegrasyon testleri [V18]; `vsce package/publish` [V6]. Ajan araçları **manifestte** bildirilir: `contributes.languageModelTools` içinde `name` (`{verb}_{noun}`, eklenti önekli), `displayName`, `modelDescription` ("explain what the tool does, what it returns, and when it should or should not be used"), `inputSchema`, `when`; uygulama `vscode.lm.registerTool` ile etkinleşmede bağlanır [V1]. Belgeler `llms.txt` ve her sayfanın `.md` sürümüyle makine okur hâlde sunuluyor [V20].

### 2.10 Belgelenmiş olaylar

- **Ocak 2023 (Aqua):** Prettier'ı taklit eden deneme eklentisi 48 saatten kısa sürede binden fazla kuruluma ulaştı; `displayName` benzersiz değildi; mavi tik yalnız alan adı sahipliğini gösteriyordu ve alan adı satın alınabiliyordu [V21].
- **Haziran 2024 (Assaraf, Kruk, Dardikman; ikincil):** bilinen kötü amaçlı kod içeren 1.283 eklenti (229 milyon kurulum); "Darcula" deneme eklentisi satın alınmış `darculatheme.com` ile doğrulanmış yayıncı oldu. Okur yorumları sayıların bir kısmına itiraz ediyor [V22].
- **Şubat–Mart 2025, yanlış pozitif:** Material Theme eklentileri "heavily obfuscated code" gerekçesiyle kaldırıldı ve engel listesiyle kullanıcılardan zorla kaldırıldı; 12 Mart 2025'te Microsoft özür diledi: "False positives suck … In the interest of safety, we moved fast and we messed up … We will clarify our policy on obfuscated code and we will update our scanners and investigation process" [V23].
- **Ekim 2025 (Wiz):** iki markette 500'den fazla eklentide 550'den fazla geçerli sır; 100'den fazla geçerli VS Marketplace PAT'i (85.000+ kurulum) ve 30'dan fazla Open VSX jetonu (100.000+ kurulum) — yani otomatik güncellemeyle zararlı sürüm dağıtma imkânı. En büyük kaynak nokta dosyalar (`.env`), artan kaynak AI yapılandırmaları (`mcp.json`, `.cursorrules`, `config.json`). Microsoft sızan PAT'leri iptal etti ve yayın öncesi sır taramasını getirdi; Open VSX jetonlarına tanınabilir `ovsxp_` öneki ekledi [V24].
- **Ekim 2025, GlassWorm (Koi):** kod Unicode değişken seçicileriyle görünmez yapılmıştı, editörde ve farkta boşluk görünüyordu; çalınan kimlik bilgileriyle başka eklentilere yayıldı; Open VSX'te 17 Ekim'de 7 eklenti, toplam 35.800 kurulum, Marketplace'te de bir eklenti [V25].
- **Şubat 2026, Open VSX "Open Sesame":** yeni yayın öncesi tarama hattı (kötü amaçlı yazılım, sır, ikili analizi, ad çalma) yük altında veritabanı bağlantı havuzu tükenince iş kuyruğa alınamadı; "tarayıcı yok" ile "kuyruğa alınamadı" aynı `false` değerini döndürdüğü için eklenti **taranmadan "PASSED"** yayına girdi; 8 Şubat'ta bildirildi, 11 Şubat'ta düzeltildi [V26].

**VS Code'u ne büyütüyor, ne koruyor:** zengin ama kapalı katkı sözlüğü, bildirimsel tembel etkinleşme, sağlam iskele/test/yayın araçları, kararlı API'yi koruyan deneysel API kapısı [V1][V2][V5][V17]. Sağlık, yayıncı güveni, çok katmanlı otomatik tarama, imza, engel listesi ve şeffaf kaldırma kaydıyla korunuyor [V7][V13][V14]. Zayıf yanı: çalışma zamanı izni olmadığı için güven tamamen yayıncıya ve markete yükleniyor; taklit en büyük kaldırma nedeni [V7][V14].

---

## 3. ArcGIS Pro add-in ve configuration (Esri)

### 3.1 `Config.daml`: bildirimsel şerit

Add-in, Esri'nin XML dili DAML ile bildirilen kısım ve .NET'teki davranış kısmından oluşur; Visual Studio sihirbazı iskeleti üretir [A1]. Kök `ArcGIS` öğesi `xsi:schemaLocation` ile `ArcGIS.Desktop.Framework.xsd` şemasına bağlanır; `AddInInfo` (`id` GUID, `version`, `desktopVersion`, `Name`, `Description`, `Image`, `Company`, `Date`, `Subject`) ilk çocuktur. `modules/insertModule` (`id`, `className`, `autoLoad`, `caption`) altında `tabs`, `groups`, `controls` tanımlanır; düğmede `loadOnClick`, `condition`, `keytip`, `smallImage`/`largeImage`, `tooltip` vardır [A1]. Katkı noktaları: şerit sekmesi/bağlamsal sekme, grup, düğme, araç, palet, araç çubuğu, galeri, açılır kutu, menü, dinamik menü, özel denetim, pane, dock pane, özellik sayfası, backstage sekmesi, ProWindow [A1].

DAML işlemleri ekleme, güncelleme ve silmedir: bir add-in başka bir add-in'in ya da Esri'nin grubuna düğme ekleyebilir (`updateModule` + `insertButton insert="before" placeWith=…`), başlığını değiştirebilir (`updateButton`), silebilir (`deleteButton`). Çerçeve bütün DAML'ları çalışma anında tek bellek içi modele birleştirir; `ArcGISPro.exe /dumpcombineddaml` birleşik sonucu döker [A1]. Başka add-in'in arayüzünü güncellemek için yükleme sırası `<dependencies><dependency name="{GUID}"/>` ile verilir [A2].

### 3.2 Koşul, durum ve JIT yükleme

"State" adlandırılmış bir mantıksal değer, "condition" bunların `and`/`or`/`not` bileşimidir; koşul DAML'da kod olmadan bildirilir ve sekme, dock pane, düğme ve araçların görünürlük/etkinliğini yönetir. Örtük durumlar: etkin pane, etkin sekme, etkin araç, yüklenmiş modül; açık durumlar `State.Activate(...)` ile koddan verilir; durum tabloları uygulama ve pane düzeyindedir [A1]. **Denetim (düğme vb.) koşulu ilk kez sağlanana kadar oluşturulmaz**; modül bir koşul gerçekleşince yüklenebilir; modüller varsayılan olarak JIT yüklenir ve "In almost all cases, autoLoad should be set to false." [A1]

### 3.3 Paket ve bağımlılık yükleme

`.esriAddinX` tek sıkıştırılmış dosyadır [A4]; üçüncü taraf derlemeler arşivin "Install" klasörüne girer ve başlangıçta GUID'li bir `AssemblyCache` alt klasörüne kopyalanır. Derlemeler varsayılan olarak `AssemblyLoadContext.Default`'a yüklenir; yani bütün add-in'ler ortak bağımlılık sürümünü paylaşır. **3.5**'te kayıt defteri anahtarıyla, **3.6**'da yönetici ayarı `UseSideBySideAddins` (varsayılan `false`) ve Add-In Manager onay kutusuyla, **3.7**'de DAML'daki `AssemblyLoadContextID` ile her add-in kendi bağlamına alınabiliyor [A2]. Yardım sayfası yan yana yüklemenin davranışı daha kararlı yaptığını ama bazen yerleşik davranışı bozabildiğini söyler [A4].

### 3.4 Sürümleme

`desktopVersion` gereken en düşük Pro sürümüdür; add-in'ler **küçük sürümlerde ileri uyumlu**, ana sürümlerde değil, geriye hiç değil. Aynı add-in'in birden çok sürümü kuruluysa en yüksek `version` yüklenir [A2]. 3.0, .NET 6'ya geçiş yüzünden kıran sürümdü: 2.x'te derlenmiş add-in, configuration ve plugin 3.x'te yüklenmez, Add-In Manager'da devre dışı görünür. 3.x içinde .NET 8 (3.3) ve .NET 10 (3.7) geçişleri oldu [A5].

### 3.5 Dijital imza ve güvenlik düzeyleri

Add-in, configuration ve plugin arşivleri X.509 sertifikasıyla `ArcGISSignAddIn.exe` üzerinden imzalanır; imza içeriğin değişmediğini kanıtlar, "does not signify that the archive is free of program errors". Ticari CA sertifikası 1 Haziran 2023'ten beri HSM'de tutulmalıdır. İmza durumu (None, Un-trusted, Invalid, Authenticated) Add-In Manager'da ve kurulum aracında gösterilir. Yönetici kısıtları, en gevşekten başlayarak: imzasız dahil hepsini yükle; yalnız güvenilir CA ile imzalı; yalnız Esri; yönetici klasörleri + Esri; hiç yükleme; yalnız yönetici klasörleri. Kullanıcı seçenekleri: yalnız Esri / güvenilir yayıncı imzası / kısıtsız; yönetici olmayan kullanıcı yöneticinin koyduğundan gevşek ayar seçemez [A3]. **Varsayılan "load all add-ins without restrictions"** [A4]. Sorunlu kurulumda `ArcGISPro.exe /disableaddins` ile eklentisiz başlatılır [A4].

### 3.6 Kurumsal dağıtım

Kurulum aracı (RegisterAddin.exe) add-in'i `Documents\ArcGIS\AddIns\ArcGISPro`'ya kopyalar ve varsa EULA gösterir [A1][A4]. "Well-known" klasörler sırası: configuration'da tanımlı klasörler, yönetici klasörleri (`HKLM\SOFTWARE\ESRI\ArcGISPro\Settings\Add-In Folders`), kullanıcı klasörleri (HKCU), varsayılan klasör; bu klasörlerden yüklenenler "Shared Add-ins"dir ve yöneticiden silinemez [A2][A4]. Ağ paylaşımındaki klasör başlangıcı yavaşlatabilir [A4]. "Configuration" daha ileri bir özelleştirmedir: açılış ekranı, başlık, denetimleri yeniden düzenleme ya da kaldırma, rol bazlı arayüz, **hangi add-in'lerin yükleneceğini denetleme**, komut süzme [A1]. Genel dağıtım ArcGIS Online öğesi (`.esriAddinX` yüklenir, paylaşım düzeyi seçilir) ya da iş ortakları için ArcGIS Marketplace üzerinden yapılır [A6].

### 3.7 Python ortamı (kısa)

ArcGIS Pro'nun varsayılan olarak tek conda ortamı (`arcgispro-py3`) vardır; değiştirilmesi önerilmez, paket eklemek için klonlanır (`conda create --clone arcgispro-py3 --name my_env --pinned`) ve `proswap` ile bütün Pro oturumlarının varsayılanı yapılır [A7]. Yani araçlar tek bir etkin ortamı paylaşır.

**ArcGIS'i ne büyütüyor, ne koruyor:** kurumsal Windows dağıtımına oturan klasör/kayıt defteri modeli ve imza düzeyleri; DAML'ın koşul/JIT modeli. Zayıf yanları: varsayılanın imzasız yükleme olması, paylaşılan yükleme bağlamı, bir add-in'in başkasının arayüzünü değiştirip silebilmesi ve ana sürümde bütün ekosistemin yeniden derlenmesi [A1][A2][A4][A5].

---

## 4. Kısa: JupyterLab, Obsidian ve Python bağımlılık yalıtımı

### 4.1 JupyterLab

- Bütün uygulama eklentilerden oluşur ve çekirdek eklentiler "no more powerful or privileged than any custom extension" [J1]. Tercih edilen tür **prebuilt** eklentidir: pip/conda paketiyle gelir, JupyterLab'i yeniden derlemeyi ve Node.js'i gerektirmez; kaynak eklenti türü "DEPRECATED" [J2].
- Eklenti Yöneticisi'nde kurulum düğmesi, "Installing an extension allows it to execute arbitrary code on the server, kernel, and the browser" uyarısı onaylanana dek görünmez [J1]. Yönetici `blocked_extensions_uris` ya da `allowed_extensions_uris` ile engel/izin listesi kipine geçer; listeden çıkarılan kurulu eklenti kırmızı işaretlenir; `--LabApp.extension_manager=readonly` kurulumu kapatır; `jupyter labextension lock` ile eklentiler kullanıcı arayüzünden açılıp kapanamaz hâle gelir. Belge, listelerin sunucu ortamını kullanıcı çekirdeklerinden yalıtmanın yerini tutmadığını yazar [J1].
- Bağımlılık tekilleştirme: `jupyterlab.sharedPackages` (`bundled`, `singleton`, `strictVersion`) ile Rspack modül federasyonu; servis jetonları paylaşılmak zorunda olduğundan tekil olmalı [J2]. Bir eklenti `jupyterlab.disabledExtensions` ile çekirdek eklentiyi kapatabilir [J2].
- Araçlar: copier tabanlı `extension-template`, `jupyter-builder build`/`develop`, `@jupyterlab/testutils`; keşif için PyPI sınıflandırıcıları (`Framework :: Jupyter :: JupyterLab :: Extensions :: Prebuilt` vb.) [J1][J2].
- Kırılma: "Your extensions may break with new releases of JupyterLab"; yazara JupyterLab sürümüne üst sınır koyması önerilir [J2]. 3.x→4.x için her JupyterLab ana sürümüne eklentinin ayrı ana sürümü önerilir ve `python -m jupyterlab.upgrade_extension .` yükseltme betiği verilir [J3].

### 4.2 Obsidian

- **Eski model:** ilk gönderim küçük ekipçe elle inceleniyor, "subsequent versions were not reviewed"; kuyruk uzadı; yeniden incelemede güncel kurallara uymayan eski eklentilere geçici istisna verildi [O6].
- **Yeni model (12 Mayıs 2026):** her sürüm güvenlik, kod kalitesi ve kötü amaçlı yazılım için otomatik taranır; sonuç karnede gösterilir; elle inceleme popüler, öne çıkan ve işaretlenen eklentilerde sürer [O4][O6]. Sonuçlar genelde birkaç dakikada gelir; geçen eklenti 24 saatte yayına girer; yeni sürümü incelemeden geçemeyen eklenti 24 saat içinde aramadan düşer; birkaç günde 2.300'den fazla bekleyen başvuru işlendi; 4.000'den fazla eklenti/tema ve 120 milyon indirme var [O6]. Kapalı kaynak: Mayıs blogu yeni kapalı kaynak eklentiyi "for now" kabul etmediğini yazıyordu [O6]; güncel politika bunu "case by case" açıklamaya bağlıyor ve SSS kaynak kodun ayrı özel depoda tutulabileceğini söylüyor [O2][O3], yani kural Mayıs'tan sonra gevşemiş görünüyor.
- İnceleme bölümleri: Manifest, Releases, Source code, Build verification; sonuçlar Error/Warning/Recommendation/Pass. Tarayıcı `build` betiğini çalıştırır. **Yerel denetim için resmî ESLint eklentisi**, sürüm çıkarmadan dal/etiket/commit üzerinde "Review branch" taraması [O3]. Karne: sağlık (README/lisans/katkı rehberi, bakım, yanıt verme, benimsenme) ve inceleme (bilinen açıklı bağımlılık yok, karartılmış kod yok, doğrulanmış GitHub artifact attestation, kullanılan Obsidian API'leri ör. Vault Read/Write), açıklamalar (pano erişimi, dış alan adına istek) [O5]. Eklentinin kendi beyan edeceği, kurulum öncesi gösterilecek erişim açıklamaları ve "verified authors" planlı [O6].
- Çalışma zamanı: varsayılan **Restricted mode** üçüncü taraf kodunu çalıştırmaz; eklentiler Obsidian'ın erişimini devralır (dosya, internet, program kurma) [O4].
- Politikalar: karartma, internetten yüklenen reklam, **istemci tarafı telemetri**, kendini ya da bağımlılıklarını kurma/güncelleme yasak; ödeme, hesap, ağ kullanımı, kasa dışı dosya erişimi, sunucu tarafı telemetri README'de açıklanmalı; çatallar ancak asıl yazarın izniyle ya da 6 ay sessizlik + 30 gün bildirimle; ihlal ve bakımsız/ağır bozuk eklenti dizinden kaldırılır [O2]. Manifest: `id` küçük harf ve tire, "obsidian" içeremez; `minAppVersion` zorunlu; Node/Electron kullanan `isDesktopOnly: true` [O7]; komut kimliklerine eklenti kimliği kendiliğinden önek olarak eklenir [O8]. Yayıncı organizasyonları üye/yönetici rolleriyle eklenti sahibi olabilir [O9]. İlk gönderimden sonra yeni GitHub sürümü yayınlamak güncelleme için yeterli [O1][O3].
- **Olay (Nisan 2026, ikincil):** Elastic'in REF6598 olarak izlediği kampanyada kurban paylaşılan bir bulut kasasına bağlandı, "Installed community plugins" eşitlemesini açmaya ikna edildi; meşru Shell Commands eklentisi PowerShell ile yükleyici indirdi, Hider eklentisi arayüz öğelerini gizledi. Eşitleme varsayılan kapalı ve uzaktan açılamıyor [O10].

### 4.3 Python bağımlılık yalıtımı: bugünkü durum

- **Paylaşılan yorumlayıcı + paylaşılan paketler:** Blender (tek `site-packages`, en yüksek sürüm kazanır) [B4]; ArcGIS Pro (tek etkin conda ortamı) [A7]. Kurulum kolay, çakışma sessiz.
- **Eklenti başına ortam (uv):** `uv.lock` evrensel çözümle üretilir ve platformlar arası taşınabilir; ancak kaynak dağıtımı olmayan paketler bazı platformlarda kurulamaz. `required-environments` listelenen her ortam için tekerlek bulunmasını zorunlu kılar (yoksa çözüm başarısız); `minimum-libc-version` (ön izleme) glibc/musl tabanını sabitler [P1]. `no-build` kaynaktan derlemeyi kapatır — uv belgesi derleme arka uçlarının "may run arbitrary Python code" olduğunu yazar [P4]. `exclude-newer` süreyle verilirse ("1 week", "P7D") **bağımlılık soğuma süresi** olur: "delaying package updates until the community has had the opportunity to vet new versions"; zaman damgası kilide yazılır [P1]. Önbellek ortamlara Linux/macOS'ta kopyala-yaz klonla, Windows'ta sabit bağla bağlanır; önbellek ortamla aynı dosya sisteminde olmalı [P4][P5]. `find-links`, `no-index`, `offline` yerel tekerlek deposuyla çevrimdışı kurulumu mümkün kılar [P4]. `uv export` kilidi `requirements.txt`, `pylock.toml` ve CycloneDX SBOM'a dönüştürür [P3][P9].
- **PEP 723 (satır içi betik üst verisi):** tek dosyalık betiğin başındaki `# /// script` bloğunda `dependencies`, `requires-python` ve `[tool]`; Ekim 2023'te PEP 723 ile koşullu kabul, Ocak 2024'te son biçim ve geçici statüden çıkış [P6]. uv bu betikleri kilitleyebilir (`uv lock --script example.py` → `example.py.lock`) [P2].
- **PEP 751 (`pylock.toml`):** "Final", çözüm tarihi 31 Mart 2025; özetler zorunlu, tek dosyada çok platform, kurulumda çözücü gerektirmez [P7].
- **Alt yorumlayıcılar (Python 3.14 `concurrent.interpreters`):** belge, aynı süreçteki yorumlayıcıların bellek düzeyinde kesin yalıtılamayacağını ve "do not use multiple interpreters in security-sensitive situations" der; "not all PyPI packages support use in multiple interpreters yet" [P8]. Eklenti yalıtımı için ayrı süreç hâlâ doğru araçtır.
- **AI ve paket adları:** 16 kod modeliyle 576.000 örnekte uydurulmuş paket oranı ticari modellerde ortalama en az %5,2, açık modellerde %21,7; 205.474 benzersiz uydurma ad. Yazarlar bunu yeni bir "package confusion" saldırısı olarak tanımlar [P10].

---

## 5. Platform tekerleği mi, uv kilidi mi?

Kanıt iki yöne de işaret ediyor:

- **Pakete gömülü platform tekerleği** (Blender [B3], VS Code `--target` [V6]): kurulumda ağ ve dizin güveni gerekmez, inceleyici kurulacak ikiliyi görür; ama yazar her platform için elle indirir, paket büyür (`--split-platforms` bu yüzden var) ve Blender örneğinde paylaşılan `site-packages` sessiz sürüm değişimi doğurur [B4].
- **Kilit + kurulumda çözüm** (KentOS v1: `pyproject.toml` + `uv.lock`): metin, küçük, farkı okunur, platformlar arası tek kaynak [P1]; ama kurulum anında dizine güvenir ve bir platformda tekerlek eksikliği ancak kullanıcı tarafında ortaya çıkar.

**Öneri (uyarla, ikisi birlikte):** `.kek` kaynak paketi plandaki gibi kalır (v1'de ikili yok, PY-KEK-06). `extension.verify`, manifestteki `compatibility.operatingSystems` ile `pyproject.toml`'daki `[tool.uv] required-environments`'ın birbirini karşıladığını denetler [P1]. Genel/kiracı katalog onay sırasında kilitten **platform başına tekerlek deposunu kendisi üretir**, ikilileri tarar, özetlerini inceleme kaydına bağlar (PY-CLOUD-04/10) ve masaüstü "hazırlama" adımı katalogdan gelen bu depodan, dizine düşmeden kurar (`no-index` + `find-links`/`offline` ailesi; bayraklar ADIM 18A'da ölçülerek seçilir) [P4]. Genel katalogda kaynaktan derleme kapalıdır (`no-build`, gerekçeli istisna). Yerel ve geliştirici paketleri bugünkü PY-UV-15 akışında kalır. Böylece Blender'ın "kurulumda ağ yok, inceleyici ikiliyi gördü" üstünlüğü, yazara elle tekerlek indirtmeden ve paylaşılan `site-packages` olmadan alınır.

---

## 6. KentOS için öneriler tablosu

"Karar" sütunu: **Aynen al** (uygulamayı olduğu gibi), **Uyarla** (fikri KentOS'a göre değiştirerek), **Kaçın** (bilerek yapma).

| # | Uygulama | Ekosistem (kaynak) | Karar | ADIM | Neden |
|---|---|---|---|---|---|
| Ö1 | Her izin için tek cümlelik gerekçe (≤64 karakter) | Blender `[permissions]` [B1][B2] | **Uyarla**: `requestedCapabilities[]`'e zorunlu kısa Türkçe `reason`; kurulum ve güncelleme ekranında aynen gösterilir | 16, 17, 18 | Moderatörün ilk baktığı yer izinler [B8]; Blender arayüzü gerekçeyi ve güncellemedeki izin artışını henüz göstermiyor [B27] |
| Ö2 | İznin yalnız beyan olması | Blender [B11], Obsidian [O4] | **Kaçın**: aracıdan geçen her işlem yetenekle zorlanır; aracıyı atlayan doğrudan dosya/ağ erişimi "güvenilir yerel Python" diye açıkça etiketlenir | 19 | Zorlanmayan izin yanıltır; PY-RUN-19/20 ile aynı yön |
| Ö3 | Statik "beyan dışı kullanım" tespiti ve karnede gösterme | Obsidian karnesi [O5], Blender `requests`/`urllib`/`http` araması [B8] | **Aynen al**: `extension.verify` Kapı 2 AST'den ağ, dosya, alt süreç, pano, `sys.path` kullanımını çıkarır; beyan edilmemiş yetenek kullanımı ret | 21, 22A | Ajanın unuttuğu ya da uydurduğu yetenek beyanı makineyle yakalanır |
| Ö4 | Yayıncı güveni diyaloğu (bağımlılıkları kapsar) | VS Code 1.97 [V7] | **Uyarla**: ilk kurulumda yayıncı + güven durumu + yetenek özeti tek ekranda | 17, 18 | VS Code'da izin olmadığı için yalnız yayıncıya güveniliyor; KentOS ikisini birden gösterebilir |
| Ö5 | İmza/güven durumunun yöneticide ve kurulumda görünmesi | ArcGIS None/Un-trusted/Invalid/Authenticated [A3] | **Aynen al**: "yerel / kurum / onaylı genel / bütünlüğü bozuk" durumları (PY-KEK-12) | 17, 18 | Güven durumu tek bakışta okunur |
| Ö6 | Kod yüklemeden değerlendirilen koşullar (`when`, `enablement`, DAML `condition`) | VS Code [V1], ArcGIS [A1] | **Aynen al**: kapalı koşul sözlüğü (etkin modül, seçim türü, katman şema kimliği, belge durumu) manifestte | 16, 18 | Python'suz kullanıcıda bile şerit doğru görünür; tembel yüklemenin ön şartı |
| Ö7 | Katkıdan türeyen etkinleşme; `*` caydırılır; JIT modül | VS Code [V2], ArcGIS `autoLoad="false"`, `loadOnClick` [A1] | **Aynen al**: tetikleyiciler manifestten türetilir (komut, MapTool, İşlemler, denetim, ifade, panel); v1'de "başlangıçta çalış" yok | 17, 19 | PY-RUN-05'in somut biçimi; başlangıç maliyeti eklenti sayısıyla büyümez |
| Ö8 | Çalışan eklentiler profili, başlangıç zamanlayıcıları, ikiye bölme, eklentisiz başlatma | VS Code [V9][V10], ArcGIS `/disableaddins` [A4] | **Aynen al**: Eklenti Yöneticisi'nde eklenti başına etkinleşme süresi/bellek/CPU; "Eklentisiz başlat" ve "Sorunlu eklentiyi bul" | 17, 18 | Eşik uydurmadan (ECO-QA-14) teşhis; destek yükünü düşürür |
| Ö9 | UI'dan ayrı süreç; süreç ≠ yetki sınırı | VS Code [V4][V7][V9] | **Uyarla**: eklenti sürümü başına süreç (PY-RUN-04) + yanıt vermeyeni sonlandır/yeniden başlat + boşta kapat; arayüzde "ayrı süreç kum havuzu değildir" | 19 | VS Code'da aynı host'taki eklentiler süreç paylaşıyor (belgeden çıkarım); KentOS daha güçlü yalıtım veriyor ama bellek maliyetini ölçmeli |
| Ö10 | Başka eklentinin ya da yerleşiğin arayüzünü güncelleme/silme | ArcGIS `updateModule`/`deleteButton` [A1], JupyterLab `disabledExtensions` [J2] | **Kaçın**: katkı yalnız ekler; yerleşiği ve başka eklentiyi gizleyemez, silemez; kısıtlamayı yalnız kurum ilkesi yapar | 16, 18 | Kiralamayla kalıntısız kaldırma ve "yerleşik gölgelenemez" kuralı (§3.2) |
| Ö11 | Uyumluluk aralığı: min zorunlu, max dışlayıcı, katalogdan sonradan daraltılabilir | Blender [B1], VS Code `engines.vscode` [V6], ArcGIS `desktopVersion` [A2], Obsidian `minAppVersion` [O7] | **Uyarla**: `compatibility`'ye KentOS sürüm aralığı (`sdkProtocol` yanında); bilinen kırılmada katalog yeni yükleme istemeden `max`'ı daraltır | 16, 22A | Yazar dönmese de kullanıcı uyumsuz sürüm almaz |
| Ö12 | Deneysel API kapısı | VS Code proposed API [V5] | **Uyarla**: deneysel SDK yüzeyleri yalnız geliştirici ortamında açılır; genel ve kiracı katalog bu paketi reddeder | 21, 22A | Ajanlarla API denenir, kararlı yüzey kırılmaz |
| Ö13 | Ana sürüm kırılması + yükseltme betiği | ArcGIS 3.0 [A5], JupyterLab 4 [J3] | **Uyarla**: `schemaVersion`/`sdkProtocol` artışında `kentos kek upgrade` (otomatik dönüştürme) ve ajana geçiş notu | 16, 21 | Kırılmada yazar kaybedilmez |
| Ö14 | Platform başına paket | Blender `--split-platforms` [B3], VS Code `--target` [V6] | **Uyarla**: §5 — kaynak `.kek` tek; katalog kilitten platform tekerlek deposu üretir ve özetle bağlar | 18A, 22A | Kurulumda ağ/derleme yok, inceleyici ikiliyi görür |
| Ö15 | Bütün eklentilere tek `site-packages`, en yüksek sürüm kazanır | Blender [B4] | **Kaçın**: eklenti başına ortam (PY-UV-12) korunur | 18A | Eski sürüm bekleyen eklenti sessizce başka sürümle çalışır |
| Ö16 | Paylaşımlı yükleme bağlamı varsayılan, yalıtım isteğe bağlı | ArcGIS ALC [A2][A4] | **Kaçın**: yalıtım varsayılandır, ortak ortam yoktur | 18A, 19 | ArcGIS 3.7'de bile yan yana yükleme varsayılan kapalı |
| Ö17 | `required-environments` (+ `minimum-libc-version`) | uv [P1] | **Aynen al**: `operatingSystems` ile tutarlılık `inspect`/`verify`'da | 16, 18A, 21 | "Linux'ta çalışıyor, Windows'ta tekerlek yok" paketleme anında yakalanır |
| Ö18 | Kaynaktan derlemeyi kapatma | uv `no-build` [P4] | **Uyarla**: genel katalogda yalnız tekerlek; istisna gerekçeli inceleme ile | 18A, 22A | Derleme arka ucu kod çalıştırır ve platforma göre başarısız olur (PY-UV-21'i sertleştirir) |
| Ö19 | Bağımlılık soğuma süresi | uv `exclude-newer` süresi [P1] | **Uyarla**: ajanın ürettiği kilitlerde soğuma; zaman damgası inceleme kaydında | 21, 22A | Yeni yayımlanmış kötü sürüme ve uydurulmuş paket adına karşı ilk savunma [P10] |
| Ö20 | Ajanın bağımlılık adlarını uydurması | LLM paket halüsinasyonu [P10] | **Uyarla**: `extension.blueprint` izinli/yaygın paket listesini verir; `verify` dizinde var olmayan, çok yeni ya da adı popüler pakete benzeyen bağımlılığı işaretler | 21 | Uydurma adın saldırganca kaydı bilinen bir tehdit |
| Ö21 | Standart kilit ve SBOM çıktısı | PEP 751 [P7], uv export [P3][P9] | **Uyarla**: `uv.lock` kaynak kalır; inceleme kaydına türetilmiş `pylock.toml` ve CycloneDX SBOM eklenir | 22A | PY-CLOUD-04 "kilit + SBOM" için standart biçim |
| Ö22 | Satır içi betik üst verisi | PEP 723 [P6], uv [P2] | **Uyarla** kişisel betikte; `.kek`'te **Kaçın** | 18A | `.kek`'te ikinci gereksinim kaynağı PY-UV-13'ü bozar; kişisel betikte kendi kilidiyle iyi |
| Ö23 | Alt yorumlayıcıyla yalıtım | Python 3.14 [P8] | **Kaçın** | 19 | Belge güvenlik için önermiyor; birçok PyPI paketi desteklemiyor |
| Ö24 | Her sürümde çok motorlu tarama, dinamik algılama, dönemsel yeniden tarama | VS Marketplace [V7][V13], Obsidian [O4] | **Uyarla**: ADIM 20'nin ağsız/sırsız işçisinde `verify` kapıları + bağımlılık açığı + sır taraması; onaylı sürümler dönemsel yeniden taranır | 20, 22A | Tek seferlik tarama yeni desenleri kaçırır |
| Ö25 | Sır taraması ve nokta dosya reddi | VS Code + `vsce` [V7], Wiz [V24] | **Aynen al**: `inspect` yasaklı listesine `.env`'in yanında `mcp.json`, `.cursorrules` gibi AI yapılandırmaları ve sır desenleri | 16, 22A | Sızıntıların çoğu nokta dosyalardan, artan kısmı AI yapılandırmalarından |
| Ö26 | Görünmez Unicode ile gizlenmiş kod | GlassWorm [V25] | **Aynen al**: `.py` ve JSON'da görünmez biçim karakterleri (değişken seçici, sıfır genişlik, yön denetimi) ret; Türkçe harfler serbest | 16, 21 | İnsan inceleyici boşluk görür, yalnız makine yakalar |
| Ö27 | Tarama hata verince "geçti" sayma | Open VSX "Open Sesame" [V26] | **Kaçın**: "taranmadı", "tarama hata verdi", "geçti" ayrı durumlar; yalnız sonuncusu yayına götürür | 22A | Yük altında saldırgan taramayı atlattı |
| Ö28 | Doğrulanmış yayıncı + taklit engeli | VS Marketplace [V6][V7], Aqua/Darcula [V21][V22] | **Uyarla**: ters alan adı `id` öneki doğrulanmış alan adına bağlı; benzer ad/simge/görünen ad denetimi; kiracı için ayrı güven kökü (ECO-DEC-03) | 16, 22A | 2026'da kaldırılan 2.140 eklentinin 1.109'u taklit [V14]; ucuz alan adıyla rozet alınabiliyordu |
| Ö29 | Kısa ömürlü kimlikle yayın, uzun ömürlü jetonun emekliliği | VS Code Trusted Publishing [V6], Wiz [V24] | **Aynen al**: CI'dan yayın OIDC/kısa ömürlü kimlikle; yayın jetonları önekli ve süreli | 22A | Sızan yayın jetonu bütün kurulumlara zararlı güncelleme demek |
| Ö30 | Engel listesi, kurulu kopyayı durdurma, kamuya açık kaldırma kaydı | VS Code [V7][V14], Blender `blocklist` [B14][B16], JupyterLab [J1] | **Uyarla**: imzalı geri çekme listesi; çevrimiçi eşitlemede eklenti nedeni yazılarak devre dışı kalır, kullanıcı verisi silinmez; kayıt kamuya açık | 17, 22A | PY-CLOUD-15 ile uyumlu: inen bayt geri alınamaz, çalıştırma durdurulabilir |
| Ö31 | Yanlış pozitif ve itiraz | Material Theme [V23] | **Aynen al**: "şüpheli" ile "kesin" ayrımı; yayıncıyı toptan yasaklamadan insan incelemesi; itiraz yolu ve kamuya düzeltme | 22A | Microsoft özür diledi ve süreç değiştirdi |
| Ö32 | Otomatik güncellemede gecikme | VS Code 1.123 / 1.125 / güncel belge [V15][V16][V11] | **Uyarla**: katalog güncellemesi politika süresi dolunca kurulur, güvenlik düzeltmesi hariç; süre ölçümle seçilir | 17, 22A | Ele geçirilmiş sürümün etki penceresi daralır |
| Ö33 | Ön sürüm kanalı, başka sürüm kurma | VS Code [V6][V11] | **Uyarla**: beta kanalı + sürüm seçme/geri dönme (PY-KEK-14) | 17, 22A | Kurumlar kararlı kanalda kalır |
| Ö34 | Statik/dinamik depo, erişim jetonu, salt okunur sistem deposu | Blender [B14][B15][B16][B17] | **Aynen al**: `kentos kek index` ile statik katalog (+HTML); dinamik uçta platform/sürüm süzgeci; kiracı jetonu; BT'nin salt okunur sistem dizini | 17, 22A | İnternetsiz belediye ağı ve kurum içi eklentiler |
| Ö35 | Kurum izin listesi ilkesi, özel market | VS Code [V12], JupyterLab [J1], ArcGIS güvenlik düzeyleri [A3] | **Uyarla**: yayıncı/kimlik/sürüm/platform ilkesi; kullanıcı ilkeyi gevşetemez (PY-OPT-12); güvensiz varsayılan yok | 17, 22A | ArcGIS'in varsayılanı "kısıtsız yükle" [A4]; KentOS bunu tekrarlamamalı |
| Ö36 | Sunucuyla aynı incelemeyi yerelde çalıştırma | Obsidian ESLint + "Review branch" [O3], Blender `validate` [B14] | **Aynen al**: `kentos kek verify` bulut incelemesinin otomatik kısmının aynısı (aynı sürüm, aynı kod) | 16, 21, 22A | Blender'da yazarların üçte biri ikinci incelemeyi istemiyor [B12]; yerel geri bildirim döngüyü kısaltır |
| Ö37 | İskele üretici ve test koşucusu | VS Code `yo code`, `@vscode/test-cli` [V17][V18]; JupyterLab copier şablonu [J2] | **Aynen al**: `kentos kek new` (= `extension.blueprint`) ve başsız test koşucusu | 21 | Plan ECO-API-10 |
| Ö38 | Birleşik katkı dökümü | ArcGIS `/dumpcombineddaml` [A1] | **Aynen al**: bütün etkin katkıların birleşik görünümü (`kentos kek dump` ya da MCP kaynağı) | 18, 21 | Ajan "sekmem nereye düştü?" sorusunu kanıtla cevaplar |
| Ö39 | Ajan aracı manifestte (`modelDescription`, `inputSchema`, `when`) | VS Code `languageModelTools` [V1] | **Aynen al**: araç tanımına model için ayrı açıklama alanı; MCP listesi kod yüklemeden | 19, 21 | Kullanıcı etiketi ile model talimatı ayrı amaçtır |
| Ö40 | Makine okur belge (`llms.txt`, `.md`) | VS Code [V20] | **Aynen al**: sürümlü SDK belgesinin düz metni + `kentos://sdk/ai-kek-pack` | 21 | Uydurma API azalır |
| Ö41 | Kendi güncelleyicisi, çalışma anında paket kurma, `sys.path` oynama yasağı | Blender [B7][B8][B9], Obsidian [O2] | **Aynen al**: Kapı 2 AST kuralları | 21 | Ortamı yalnız host yönetir (PY-UV-23) |
| Ö42 | Paket dizini salt okunur, kullanıcı verisi ayrı dizinde | Blender `extension_path_user` [B6][B7] | **Aynen al** | 17 | Güncelleme veriyi silmez (PY-RUN-18) |
| Ö43 | Genel çevrimiçi erişim anahtarı | Blender `bpy.app.online_access` [B6] | **Aynen al**: anahtar kapalıyken `network` yeteneği aracıda reddedilir | 18A, 19 | Kullanıcı ağı tek yerden kapatır; Blender bunu zorlayamıyor, KentOS aracıda zorlayabilir |
| Ö44 | Eklentiler arası kod bağımlılığı | Blender yasak [B10]; VS Code `extensionDependencies`/`extensionPack` [V3] | **Uyarla**: v1'de kod bağımlılığı yok, yalnız birlikte kurulan "paket kümesi" | 17, 22A | AI'ın ürettiği küçük ve bağımsız paketler birbirini kırmaz (PY-RUN-15 sonraya) |
| Ö45 | Karne: sağlık + inceleme + açıklamalar | Obsidian [O5] | **Uyarla**: katalog karnesi (geçen kapılar, yetenekler, tespit edilen kullanım, bakım) | 22A | Keşif ve güven aynı ekranda |
| Ö46 | İstemci tarafı telemetri yasağı, açıklama zorunluluğu | Obsidian [O2], Blender ToS 4.5 [B10] | **Aynen al** | 22A | Belediye ve vatandaş verisi |
| Ö47 | Proje/çalışma alanı güveni | VS Code [V8], `.blend` Auto Run [B22], Obsidian kasa [O10] | **Aynen al**: dışarıdan gelen `.kcad` açılınca eklenti ifadesi/denetimi "kısıtlı kip"te; eksik eklenti kurdurulmaz (PY-TOOL-20) | 17, 19, 22 | Belge açmak kod çalıştırma izni değildir (§1.3) |
| Ö48 | Organizasyon sahipliği, "Official" rozeti, çatal politikası | Obsidian [O2][O5][O9], Blender ToS 3.8 [B10] | **Uyarla** | 22A | Yazar devri (PY-CLOUD-16) ve AI'ın ürettiği kopyaların ayıklanması |

---

## 7. Etkiye göre ilk 15 öneri

Sıra, ekosistemin büyümesine ve sağlığına etkiye göredir. Her maddenin altında §6'daki satırlar ve plan adımları yazılıdır.

1. **Kurulumdan önce gösterilen, aracıda zorlanan ve koddan doğrulanan yetenekler.** Manifestteki her yetenek kısa bir Türkçe gerekçeyle kurulumda ve her güncellemede **farkıyla** gösterilir; aracıdan geçen işlemler yetenekle zorlanır; `verify` AST'den gerçek kullanımı çıkarır ve beyan dışı kullanımı reddeder. İncelenen beş ekosistemin hiçbiri çalışma zamanında izni zorlamıyor [B11][O4][V7][J1][A4]; Blender'ın kendi arayüzü gerekçeyi göstermiyor ve güncellemedeki izin artışını göstermek orada hâlâ açık iş [B27]. KentOS'un en belirgin güven farkı burada. (Ö1–Ö4, Ö43 · ADIM 16, 17, 18, 19, 21, 22A)
2. **Ajanın ürettiği paketi yerelde, bulutla aynı doğrulayıcıyla, kapalı sözlüklere karşı denetlemek.** Tek `kentos kek verify` (= `extension.verify`) bulut incelemesinin otomatik kısmının aynısıdır; hatalar dosya + JSON işaretçisi + Türkçe çözümle döner; sözlükler (yetenek, koşul, ikon, modül, platform) kapalıdır; ajan araçları manifestte `modelDescription`/`inputSchema` taşır; SDK belgesi düz metin verilir. Obsidian'ın ESLint eklentisi ve "Review branch"ı, Blender'ın `validate`'i ve VS Code'un `llms.txt`'i aynı ihtiyaca cevap [O3][B14][V1][V20]. (Ö3, Ö36, Ö39, Ö40 · ADIM 16, 21)
3. **Her sürüm için otomatik ve "açık kalmayan" kapılar.** Şema/AST/kuru koşu kapılarına sır taraması (AI yapılandırma dosyaları dahil), görünmez Unicode reddi, bağımlılık açığı taraması ve dönemsel yeniden tarama eklenir; "taranmadı" hiçbir zaman "geçti" sayılmaz. Wiz sızıntıları, GlassWorm ve "Open Sesame" bu üç boşluğun her birinin gerçekten kullanıldığını gösterdi [V24][V25][V26]. (Ö24–Ö27 · ADIM 16, 20, 21, 22A)
4. **İnsan onayını farka ve riske bağlamak; kapasiteyi baştan planlamak.** "Her sürüm onaydan geçer" kalır (PY-CLOUD-08), ama inceleyici tam paketi değil önceki onaylı sürüme göre farkı (yetenek, bağımlılık, kod, ikili) görür; yeni yetenek ya da yeni bağımlılık zorunlu derin incelemedir, yalnız metin/ikon değişikliği hızlı yol. Blender fark görüntüleyici arıyor [B12]; Obsidian'ın yalnız-ilk-sürüm modelinde ekip başvurulara yetişemedi, kuyruk sürekli uzadı ve geçişte 2.300'den fazla bekleyen başvuru vardı [O6]. AI üretimi başvuru sayısını artıracağı için ret gerekçeleri hazır yanıt kodlarıyla ve ajanın tek adımda düzelteceği biçimde verilir [B9]. (Ö36, Ö45 · ADIM 21, 22A)
5. **Yayıncı ve ad alanı doğrulama, taklide karşı ilk savunma.** Ters alan adı kimliğin öneki doğrulanmış alan adına bağlanır; görünen ad, simge ve kimlik benzerliği yayın öncesi denetlenir; yayın kısa ömürlü kimlikle (OIDC) yapılır, jetonlar önekli ve sürelidir. 2026'da VS Marketplace'ten kaldırılanların yarıdan fazlası taklit [V14]; ucuz alan adıyla doğrulanmış rozet alınabildi [V21][V22]. (Ö28, Ö29 · ADIM 16, 22A; ECO-DEC-03)
6. **Eklenti başına ortam + platform kapsamı doğrulanmış kilit + yalnız tekerlek + soğuma süresi.** PY-UV-12 korunur; `required-environments` manifestteki işletim sistemleriyle tutarlı olmalıdır; genel katalogda kaynaktan derleme kapalıdır; ajanın kilitlediği paketler soğuma süresinden geçer ve katalog onaylı kilitten platform tekerlek deposu üretir (§5). Blender'ın paylaşılan `site-packages`'ı ve ArcGIS'in paylaşılan yükleme bağlamı, paylaşmanın sessiz kırılma olduğunu gösteriyor [B4][A2]. (Ö14–Ö21 · ADIM 18A, 21, 22A)
7. **Süreç yalıtımı, dürüst etiketle.** Eklenti sürümü başına süreç arayüzü ve belgeyi korur (VS Code'un extension host'unun kanıtladığı şey [V9]), yanıt vermeyen süreç sonlandırılıp yeniden başlatılır; ama aynı kaynağın uyardığı gibi süreç yetki sınırı değildir [V7]. OS düzeyinde kum havuzu ayrı bir kip olarak kalır ve desteklenmediği yerde "yok" denir (ECO-DEC-01). Alt yorumlayıcılar seçenek değildir [P8]. Süreç başına bellek ilk dilimde ölçülür. (Ö9, Ö23 · ADIM 19)
8. **Tembel etkinleştirme ve teşhis araçları, uydurma bütçe olmadan.** Tetikleyiciler manifestten türetilir, koşullar kod yüklenmeden değerlendirilir, v1'de başlangıçta çalışan eklenti yoktur; Eklenti Yöneticisi eklenti başına etkinleşme süresi/bellek/CPU gösterir, "Eklentisiz başlat" ve "Sorunlu eklentiyi bul" (ikiye bölme) sunar. Eşikler ölçümden sonra konur (ECO-QA-14) [V2][V9][V10][A1][A4]. (Ö6–Ö8 · ADIM 17, 18, 19)
9. **Uyumluluk aralığı, deneysel API kapısı, kullanımdan kaldırma ve yükseltme aracı.** Manifest KentOS sürüm aralığını (min zorunlu, max dışlayıcı) taşır ve katalog max'ı sonradan daraltabilir; deneysel SDK yüzeyleri yalnız geliştirici ortamında açılır ve genel katalog onları reddeder; protokol artışında `kentos kek upgrade` ve ajana geçiş notu verilir [B1][V5][V6][A2][J3]. (Ö11–Ö13 · ADIM 16, 21, 22A)
10. **Kurum katalogları ve ilke.** Kiracı kataloğu, statik katalog üretimi (`kentos kek index`, internetsiz ağ için), erişim jetonu, engel listesi ve BT'nin salt okunur sistem dizini; yayıncı/kimlik/sürüm/platform bazlı izin listesi ilkesi ve kullanıcı ilkeyi gevşetemez. Varsayılan güvenli: ArcGIS'in "kısıtsız yükle" varsayılanı tekrarlanmaz [B14][B15][B16][B17][V12][J1][A3][A4]. (Ö34, Ö35 · ADIM 17, 22A)
11. **Geliştirici araç zinciri, tek doğrulayıcı üzerinde.** `kentos kek new | verify | pack | test | serve | index | dump | upgrade`: iskele (blueprint), doğrulama, deterministik paketleme, başsız test, yerel katalog sunucusu, birleşik katkı dökümü ve yükseltme. Blender'ın `build/validate/server-generate`, VS Code'un `yo code`/`vsce`/`test-cli`, ArcGIS'in `/dumpcombineddaml` ve JupyterLab'in şablon + yükseltme betiği bu setin her parçasının ekosistem büyütmede işe yaradığını gösteriyor [B14][V6][V17][V18][A1][J2][J3]. (Ö36–Ö38 · ADIM 16, 21)
12. **Geri çekme, zorunlu devre dışı bırakma, şeffaf kayıt ve itiraz.** İmzalı geri çekme listesi çevrimiçi eşitlemede eklentiyi nedeniyle devre dışı bırakır, kullanıcı verisini silmez; kaldırmalar kamuya açık bir kayda düşer; "şüpheli" ile "kesin" ayrılır ve yayıncı toptan yasaklanmadan önce insan bakar [V7][V14][V23][B14][J1]. (Ö30, Ö31 · ADIM 17, 22A)
13. **Proje güveni: `.kcad` açmak eklenti kodu çalıştırmaz.** Dışarıdan gelen projede eklenti ifadeleri ve denetimleri kullanıcı onaylayana dek kısıtlı kipte kalır (etiket/stil yer tutucuyla çizilir); eksik eklenti kurdurulmaz; ileride senkron/paylaşım özellikleri eklentiyi beraberinde getirmez. `.blend` Auto Run, Obsidian kasa eşitlemesi ve VS Code Workspace Trust aynı dersi veriyor [B22][O10][V8]. (Ö47 · ADIM 17, 19, 22)
14. **Gecikmeli güncelleme, kanallar ve geri dönüş.** Katalog güncellemeleri politika süresi dolunca kurulur (güvenlik düzeltmesi hariç), beta ve kararlı kanal ayrıdır, kullanıcı ve kurum belirli sürüme dönebilir. VS Code iki saatlik gecikmeyle başladı ve ayarlanabilir yaptı [V15][V16][V11]. (Ö32, Ö33 · ADIM 17, 22A)
15. **Keşfedilebilirlik ve sağlık karnesi.** Katalog sayfasında yetenekler ve gerekçeleri, geçen kapılar, tespit edilen kullanım, bakım ve yanıt verme göstergeleri; kurumsal "paket kümeleri" (birlikte kurulacak dikey eklentiler); orijinal yazar rozeti ve çatal politikası; istemci telemetrisi yasağı. Obsidian'ın karnesi, VS Code'un paket kümesi ve Blender'ın uygulama içi keşfi büyümenin görünür tarafı [O5][V3][B17]. (Ö44–Ö46, Ö48 · ADIM 22A)

---

## 8. Doğrulanamayanlar ve açık sorular

- **Blender, onaylı eklentinin yeni sürümü:** 2024 tasarım kaydı sonraki sürümlerin kendiliğinden onaylandığını yazıyor [B26], 2025 notu yayın sonrası denetimin sürdüğünü gösteriyor [B12]; ama kullanıcıya dönük resmî belgede yazılı değil ve 13 Kasım 2026 itibarıyla değişip değişmediği doğrulanamadı.
- **Blender izinlerinin web sitesinde ve kurulum onayında nasıl gösterildiği:** doğrulanamadı; yalnız Blender içindeki ayrıntı paneli koddan okundu [B27].
- **Blender hizmet koşulları 3.6** ("pre-compiled packages") ile PyPI tekerlek izni arasındaki sınır metinde açık değil.
- **Blender tekerlek çakışması:** 2024 takip kaydı "hata verir" diyor [B5], `main` kodu yenisini seçiyor [B4]; hangi sürümde değiştiği doğrulanamadı.
- **VS Code eklenti başına sayısal etkinleşme bütçesi:** belgede bulunamadı. Aynı host'taki eklentilerin tek süreç paylaşması belgedeki host tablosundan çıkarımdır [V4].
- **VS Code `extensions.autoUpdateDelay` varsayılanı:** 1.125 notu iki saat [V16], güncel belge 12 saat [V11]; değişikliğin sürümü doğrulanamadı.
- **ArcGIS 3.3 (.NET 8) ve 3.7 (.NET 10) geçişlerinde** 3.0–3.2'de derlenmiş add-in'lerin yeniden derleme gerektirip gerektirmediği: Esri küçük sürümde ileri uyum diyor [A2]; ayrı yükseltme rehberleri okunamadı (sayfa ana sayfaya yönlendi).
- **Obsidian "Build verification"ın ayrıntısı** ve planlanan erişim açıklamalarının zorlanıp zorlanmayacağı: blog açıklamaların beyana dayalı olacağını söylüyor, zorlama planından söz etmiyor [O6].
- **Haziran 2024 VS Code sayıları** (1.283 eklenti / 229 milyon kurulum) ikincil kaynaktan; okur yorumları bir kısmına itiraz ediyor [V22]. Open VSX'in pre-publish uygulama takvimi yalnız ikincil kaynaklarda görüldü; burada yalnız birincil "Open Sesame" yazısındaki boru hattı tanımı kullanıldı [V26].
- **NumPy ve benzeri derlenmiş paketlerin alt yorumlayıcı desteği:** güncel durum doğrulanamadı; sonuç yalnız Python belgesinin genel uyarısına dayanıyor [P8].
- **Blender için ticari depo örneği** (ör. Superhive): yalnız üçüncü taraf belgede geçtiği için kullanılmadı.

---

## Kaynaklar

Hepsi 13 Kasım 2026'da okundu; sayfanın kendi tarihi varsa yanında yazılı.

**Blender**

- [B1] Blender Manual 5.3 — How to Create Extensions — https://docs.blender.org/manual/en/latest/advanced/extensions/getting_started.html
- [B2] Blender Developer Docs — Manifest Schema 1.0.0 — https://developer.blender.org/docs/features/extensions/schema/1.0.0/
- [B3] Blender Manual 5.3 — Python Wheels — https://docs.blender.org/manual/en/latest/advanced/extensions/python_wheels.html
- [B4] Blender kaynak kodu, `scripts/modules/_bpy_internal/extensions/wheel_manager.py` (`main`, Gitea API ile) — https://projects.blender.org/blender/blender/src/branch/main/scripts/modules/_bpy_internal/extensions/wheel_manager.py
- [B5] Blender issue #119681 "Extensions Platform: support bundling wheels (.whl)" (açılış 20 Mart 2024, kapanış 8 Mayıs 2024; Gitea API ile) — https://projects.blender.org/blender/blender/issues/119681
- [B6] Blender Manual 5.3 — Add-ons — https://docs.blender.org/manual/en/latest/advanced/extensions/addons.html
- [B7] Blender Developer Handbook — Add-on Guidelines — https://developer.blender.org/docs/handbook/extensions/addon_guidelines/
- [B8] Blender Developer Docs — Moderation / Guidelines — https://developer.blender.org/docs/features/extensions/moderation/guidelines/
- [B9] Blender Developer Docs — Moderation / Canned Responses — https://developer.blender.org/docs/features/extensions/moderation/canned_responses/
- [B10] Blender Extensions — Terms of Service (son güncelleme 10 Ağustos 2026) — https://extensions.blender.org/terms-of-service/
- [B11] F. Siddi, "Extensions Platform Beta Release", Blender Developers Blog, 15 Mayıs 2024 — https://code.blender.org/2024/05/extensions-platform-beta-release/
- [B12] "2025-04-24 Extensions Platform" toplantı notu (yayın 25 Nisan 2025) — https://forum.blender.org/t/2025-04-24-extensions-platform/40185
- [B13] "2024-04-30 Extensions Platform Backend" toplantı notu — https://forum.blender.org/t/2024-04-30-extensions-platform-backend/34413
- [B14] Blender Manual 5.3 — Extensions Command Line Arguments — https://docs.blender.org/manual/en/latest/advanced/command_line/extension_arguments.html
- [B15] Blender Manual 5.3 — Creating a Static Extensions Repository — https://docs.blender.org/manual/en/latest/advanced/extensions/creating_repository/static_repository.html
- [B16] Blender Manual 5.3 — Creating a Dynamic Extensions Repository — https://docs.blender.org/manual/en/latest/advanced/extensions/creating_repository/dynamic_repository.html
- [B17] Blender Manual 5.3 — Get Extensions (Preferences) — https://docs.blender.org/manual/en/latest/editors/preferences/extensions.html
- [B18] Blender Developer Docs — Extensions CI/CD — https://developer.blender.org/docs/features/extensions/ci_cd/
- [B19] Blender Extensions — Approval Queue, durum süzgeçleriyle sayım (13 Kasım 2026) — https://extensions.blender.org/approval-queue/
- [B20] Blender Extensions — About — https://extensions.blender.org/about/
- [B21] Blender Manual 5.3 — Extensions Tags ve Extension Licenses — https://docs.blender.org/manual/en/latest/advanced/extensions/tags.html · https://docs.blender.org/manual/en/latest/advanced/extensions/licenses.html
- [B22] The Hacker News, "Hackers Hijack Blender 3D Assets to Deploy StealC V2", 25 Kasım 2025 (ikincil; Morphisec raporu) — https://thehackernews.com/2025/11/hackers-hijack-blender-3d-assets-to.html
- [B23] Blender Developer Docs — Blender 4.2 LTS: Extensions — https://developer.blender.org/docs/release_notes/4.2/extensions/
- [B24] Blender Developer Handbook — Extension Hosting on blender.org — https://developer.blender.org/docs/handbook/extensions/hosted/
- [B25] Blender Developer Handbook — AI Contributions Policy (sayfada tarih yok) — https://developer.blender.org/docs/handbook/contributing/ai_contributions/
- [B26] extensions-website issue #89 "Moderation and approval queue: review data model and state transitions" (açılış 23 Nisan 2024, kapanış 10 Mayıs 2024; Gitea API ile) — https://projects.blender.org/infrastructure/extensions-website/issues/89
- [B27] Blender kaynak kodu, `scripts/addons_core/bl_pkg/bl_extension_ui.py` (`main`, Gitea API ile) — https://projects.blender.org/blender/blender/src/branch/main/scripts/addons_core/bl_pkg/bl_extension_ui.py
- [B28] Blender issue #119521 "Extensions Platform: beta launch" (kapanış 18 Temmuz 2024) ve commit `e0b47f78e8aaf9eefd816441eaa7086ce0acf259` (18 Mayıs 2024; Gitea API ile) — https://projects.blender.org/blender/blender/issues/119521

**Visual Studio Code**

- [V1] Contribution Points (sayfa tarihi 11/11/2026) — https://code.visualstudio.com/api/references/contribution-points
- [V2] Activation Events — https://code.visualstudio.com/api/references/activation-events
- [V3] Extension Manifest — https://code.visualstudio.com/api/references/extension-manifest
- [V4] Extension Host (11/11/2026) — https://code.visualstudio.com/api/advanced-topics/extension-host
- [V5] Using Proposed API (11/11/2026) — https://code.visualstudio.com/api/advanced-topics/using-proposed-api
- [V6] Publishing Extensions — https://code.visualstudio.com/api/working-with-extensions/publishing-extension
- [V7] Extension runtime security (11/11/2026) — https://code.visualstudio.com/docs/configure/extensions/extension-runtime-security
- [V8] Workspace Trust Extension Guide (11/11/2026) — https://code.visualstudio.com/api/extension-guides/workspace-trust
- [V9] J. Rieken, "Resolving extension issues with bisect", 16 Şubat 2021 — https://code.visualstudio.com/blogs/2021/02/16/extension-bisect
- [V10] VS Code wiki — Performance Issues — https://github.com/microsoft/vscode/wiki/Performance-Issues
- [V11] Extension Marketplace — https://code.visualstudio.com/docs/configure/extensions/extension-marketplace
- [V12] Manage extensions in enterprise environments (11/11/2026) — https://code.visualstudio.com/docs/enterprise/extensions
- [V13] "Security and Trust in Visual Studio Marketplace", Microsoft, 11 Haziran 2025 — https://developer.microsoft.com/blog/security-and-trust-in-visual-studio-marketplace/
- [V14] microsoft/vsmarketplace — `RemovedPackages.md` (26 Şubat 2025'ten 12 Kasım 2026'ya; sayım bizim) — https://github.com/microsoft/vsmarketplace/blob/main/RemovedPackages.md
- [V15] VS Code 1.123 sürüm notları, 3 Haziran 2026 — https://code.visualstudio.com/updates/v1_123
- [V16] VS Code 1.125 sürüm notları, 17 Haziran 2026 — https://code.visualstudio.com/updates/v1_125
- [V17] Your First Extension — https://code.visualstudio.com/api/get-started/your-first-extension
- [V18] Testing Extensions — https://code.visualstudio.com/api/working-with-extensions/testing-extension
- [V19] Bundling Extensions — https://code.visualstudio.com/api/working-with-extensions/bundling-extension
- [V20] VS Code `llms.txt` — https://code.visualstudio.com/llms.txt
- [V21] I. Goldman, Y. Kadkoda (Aqua Security), "Can You Trust Your VSCode Extensions?", 6 Ocak 2023 — https://www.aquasec.com/blog/can-you-trust-your-vscode-extensions/
- [V22] B. Toulas, BleepingComputer, "Malicious VSCode extensions with 229M installs found on Microsoft marketplace", 9 Haziran 2024 (ikincil) — https://www.bleepingcomputer.com/news/security/malicious-vscode-extensions-with-229m-installs-found-on-microsoft-marketplace/
- [V23] microsoft/vsmarketplace issue #1173, S. Hanselman yanıtı 12 Mart 2025 — https://github.com/microsoft/vsmarketplace/issues/1173
- [V24] R. McCarthy (Wiz Research), "Dismantling a Critical Supply Chain Risk in VSCode Extension Marketplaces", 15 Ekim 2025 — https://www.wiz.io/blog/supply-chain-risk-in-vscode-extension-marketplaces
- [V25] I. Dardikman (Koi Security / Palo Alto Networks), "GlassWorm: First Self-Propagating Worm Using Invisible Code Hits OpenVSX Marketplace", 18 Ekim 2025 — https://www.paloaltonetworks.com/blog/security-operations/glassworm-first-self-propagating-worm-using-invisible-code-hits-openvsx-marketplace/
- [V26] O. Simhony (Palo Alto Networks), "Open Sesame: How a Fail-Open Bug in Open VSX's New Scanner Let Malware Walk Right In", 27 Mart 2026 — https://www.paloaltonetworks.com/blog/security-operations/open-sesame-fail-open-bug-open-vsx-scanner/

**ArcGIS Pro (Esri)**

- [A1] ProConcepts Framework (ArcGIS Pro 3.7, tarih 20/04/2026) — https://github.com/Esri/arcgis-pro-sdk/wiki/ProConcepts-Framework
- [A2] ProConcepts Advanced Topics (3.7, 20/04/2026) — https://github.com/Esri/arcgis-pro-sdk/wiki/ProConcepts-Advanced-Topics
- [A3] ProGuide Digitally signed add-ins and configurations (3.7, 20/04/2026) — https://github.com/Esri/arcgis-pro-sdk/wiki/ProGuide-Digitally-signed-add-ins-and-configurations
- [A4] ArcGIS Pro yardım — Manage add-ins — https://pro.arcgis.com/en/pro-app/latest/get-started/manage-add-ins.htm
- [A5] ProConcepts 3.0 Migration Guide (3.7, 20/04/2026) — https://github.com/Esri/arcgis-pro-sdk/wiki/ProConcepts-3.0-Migration-Guide
- [A6] ProConcepts Distributing Add-Ins Online (3.7, 20/04/2026) — https://github.com/Esri/arcgis-pro-sdk/wiki/ProConcepts-Distributing-Add-Ins-Online
- [A7] ArcGIS Pro yardım — Package management with conda — https://pro.arcgis.com/en/pro-app/latest/arcpy/get-started/what-is-conda.htm

**JupyterLab**

- [J1] JupyterLab — Extensions (kullanıcı belgesi) — https://jupyterlab.readthedocs.io/en/latest/user/extensions.html
- [J2] JupyterLab — Develop Extensions — https://jupyterlab.readthedocs.io/en/latest/extension/extension_dev.html
- [J3] JupyterLab — Extension Migration Guide — https://jupyterlab.readthedocs.io/en/latest/extension/extension_migration.html

**Obsidian**

- [O1] Submit your plugin (kaynak: obsidianmd/obsidian-developer-docs) — https://docs.obsidian.md/plugins/releasing/submit-plugin
- [O2] Developer policies — https://docs.obsidian.md/community-directory/developer-policies
- [O3] Community directory — Frequently asked questions — https://docs.obsidian.md/community-directory/faq
- [O4] Obsidian Help — Plugin security (inceleme bölümü 14 Mayıs 2026'da güncellendi; kaynak: obsidianmd/obsidian-help) — https://help.obsidian.md/plugin-security
- [O5] Obsidian Help — Community directory (listeleme sayfası ve karne) — https://help.obsidian.md/community-directory
- [O6] kepano, Obsidian blog yazısı, 12 Mayıs 2026 — https://obsidian.md/blog/future-of-plugins/
- [O7] Manifest — https://docs.obsidian.md/Reference/Manifest
- [O8] Submission requirements for plugins — https://docs.obsidian.md/community-directory/submission-requirements-for-plugins
- [O9] Organizations — https://docs.obsidian.md/community-directory/organizations
- [O10] R. Lakshmanan, The Hacker News, "Obsidian Plugin Abuse Delivers PHANTOMPULSE RAT", 16 Nisan 2026 (ikincil; Elastic Security Labs REF6598) — https://thehackernews.com/2026/04/obsidian-plugin-abuse-delivers.html

**Python bağımlılık yalıtımı**

- [P1] uv — Resolution (universal resolution, `required-environments`, `minimum-libc-version`, `exclude-newer`, dependency cooldowns) — https://docs.astral.sh/uv/concepts/resolution/
- [P2] uv — Running scripts (PEP 723, `uv lock --script`) — https://docs.astral.sh/uv/guides/scripts/
- [P3] uv — Exporting a lockfile (`pylock.toml`) — https://docs.astral.sh/uv/concepts/projects/export/
- [P4] uv — Settings reference (`no-build`, `link-mode`, `find-links`, `no-index`, `offline`) — https://docs.astral.sh/uv/reference/settings/
- [P5] uv — Caching — https://docs.astral.sh/uv/concepts/cache/
- [P6] PyPA — Inline script metadata — https://packaging.python.org/en/latest/specifications/inline-script-metadata/
- [P7] PEP 751 — A file format to record Python dependencies for installation reproducibility (Final, 31 Mart 2025) — https://peps.python.org/pep-0751/
- [P8] Python 3.14 belgesi — `concurrent.interpreters` — https://docs.python.org/3/library/concurrent.interpreters.html
- [P9] uv — Locking and syncing (dışa aktarma biçimleri, CycloneDX) — https://docs.astral.sh/uv/concepts/projects/sync/
- [P10] J. Spracklen vd., "We Have a Package for You! A Comprehensive Analysis of Package Hallucinations by Code Generating LLMs", USENIX Security 2025 (arXiv v3) — https://arxiv.org/abs/2406.10279
