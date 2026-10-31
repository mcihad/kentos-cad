# KentOS — eklenti, Python API ve yapay zekâ uygulama kararları

İnceleme: **30 Ekim 2026**. Kod tabanı: `dc7b16d6c`.
Plan dalı: `codex/architecture-advanced-plugin-plans`.

Hedef; KentOS'un kendi arayüz bileşenleriyle geliştirilen, mevcut CAD/GIS
yeteneklerini kullanabilen, ağır işi durdurulabilir biçimde çalıştıran ve
yapay zekâyla üretilebilen bir Python eklenti ekosistemidir. Sahnedeki ürün
işlemleri Python, MCP, eklenti ve uygulama içi ajan üzerinden aynı anlamla
yapılabilmelidir. Python bilmek veya bir sağlayıcı kullanmak temel uygulamanın
ön koşulu değildir.

Bu belge kullanıcının son kararlarını **uygulanabilir yedi iş paketinde**
toparlar. [python.md](python.md) ayrıntılı UV/`.kek` ekosistem planı olarak
korunur; oradaki `PY-*` ve `ECO-*` görevleri burada yeniden numaralandırılmaz
ve yapılmış sayılmaz. Eski belgenin 12 Ekim envanteriyle çelişkide bu tarihli
kod tespiti esas alınır. CBS form sözleşmesinin kaynağı
[form.md](form.md) ve [form-kararlar.md](form-kararlar.md)'dir.

**Bu teslimde yalnız plan yazılmıştır.** Uygulama kodu, `CLAUDE.md`,
`TODOS.md`, paketler, kullanıcı ortamı ve yayın sistemi değiştirilmemiştir.
Aşağıda önerilen yeni API/descriptor adları bugün çalışan API değildir.

## 1. Kesin ürün kararları

| Konu | Karar |
| --- | --- |
| Paket adı | KentOS eklentisi **`.kek`** uzantılı sıkıştırılmış pakettir. |
| Ana giriş | Her kurulu eklentinin yönetimi ve ana girişi **Eklentiler** sekmesindedir. Eklenti türüne uygun ek erişimler aynı kayda bağlanır. |
| Arayüz | KentOS UI bileşenlerine Python bağları sağlanır. Ayrı Qt/PySide görünümü veya Python'dan doğrudan Iced pointer'ı verilmez. |
| Türler | Bir eklenti komut, dialog/form, dock/panel, Processing provider, bağlam/menü eylemi ve portable form kaynağı sunabilir; birden fazla tür birlikte olabilir. |
| Analiz gücü | Seçilen Python analitik kütüphaneleri ve mevcut KentOS analizleri kullanılabilir. Kütüphane kurulmuş olması yeni yetki veya otomatik bulut çalıştırma anlamına gelmez. |
| Ağır iş | İş arayüzü kilitlemez; ilerleme, iptal ve kaynak temizleme sözleşmesi vardır. Sonuç belgeye kontrollü ve geri alınabilir uygulanır. |
| Ortam | Python/UV, interpreter ve paket kurulumu isteğe bağlıdır. Kişisel analiz, geliştirme ve yayımlanmış eklenti ortamları ayrıdır. |
| Otomasyon | GUI'deki ürün işlemi ve seçenekleri sürümlü komut/Python/MCP/eklenti yüzeyinde aynı sonucu verir. Salt ekran tıklaması taklidi yeterli değildir. |
| AI | AI ile eklenti üretimi birincil geliştirici akışıdır. Uygulama içi sağlayıcı bağımsız ajan ve dış MCP istemcisi aynı ürün sözleşmesini kullanır. |
| Yayın | Yerel kullanım buluta bağlı değildir. Bulutta yayımlanacak **her sürüm**, değişmez paket özetiyle onaydan geçer. |
| Doküman | Sürüme bağlı Python API referansı, UI/Processing rehberi, çalışan örnekler ve MCP keşif araçları zorunlu teslimdir. |

## 2. Kodda bulunanlar ve eksik bağlantı

Kaynak okumaları bu oturumda test çalıştırıldığı anlamına gelmez. Satır
numaraları inceleme commit'ine aittir.

| Alan | Somut kaynak | Bugünkü durum / sınır |
| --- | --- | --- |
| Tipli SDK | [sdk.py](../scripts/python/sdk.py), 1–19, 989–1051; [Python paketi](../python/pyproject.toml) | Katalogdan namespace, tip ve docstring üretimi; yazmadan sapma kontrolü var. Tam geliştirici portalı ve örnek kabulüyle aynı şey değil. |
| Python native bağı | [lib.rs](../crates/native/python/src/lib.rs), 1–12, 55–59, 99–115 | PyO3/headless Session ve ortak komut yolu var; Rust çalışırken GIL bırakılıyor. Bu genel UI bağlaması değildir. |
| Konsol süreci | [host.rs](../apps/desktop/src/python/host.rs), 37–54, 64–142 | Ayrı Python süreci, pipe okuma/yazma thread'leri, süreç sonlandırma var. Interpreter seçimi UV ürün yöneticisi değildir. |
| Undo/stop | [python/mod.rs](../apps/desktop/src/python/mod.rs), 10–16, 620–633 | Scriptin yazımları gruplanır, hata/durdurmada rollback ve generation kontrolü var; çalışma boyunca başka belge düzenlemeleri engellenebilir. |
| Donma riski | Aynı [mod.rs](../apps/desktop/src/python/mod.rs), 479–533 | Konsol RPC'si `call_from(..., None, ...)` ile masaüstünde yürür. Ayrı Python süreci, gönderdiği bütün ağır ürün çağrılarını kendiliğinden arka plana taşımaz. |
| Python işlem yankısı | [echo.rs](../apps/desktop/src/python/echo.rs), 1–15, 28–89 | `drafting.pythonEcho` ile komut/seçim ve undo/redo yankısı, gizli alan redaksiyonu mevcut. Yeniden icat yerine seçenek/işlem kapsamı denetlenir. |
| Masaüstü ajanı ve işler | [agents.rs](../apps/desktop/src/python/agents.rs), 24–63, 97–188; [rpc.rs](../crates/native/headless/src/rpc.rs), 38–122 | Açık belge tutamacı, modül bağlantısı, busy/kapalı belge kontrolü, job start/status/wait/cancel temeli var. |
| Native Processing | [types.rs](../crates/native/processing/src/types.rs), 493–520, 618–736; [registry.rs](../crates/native/processing/src/registry.rs), 38–47, 87–118 | Rust function pointer executor, internal Document/Slot bağlamı, Feedback, changeset ve builtins/kullanıcı modelleri var. Dinamik Python provider adaptörü eksik halkadır. |
| Processing arka planı | [background.rs](../apps/desktop/src/processing/background.rs), 1–14, 96–112, 185–218 | Okuma kopyası/thread, progress ve stop bayrağı var. Thread'i zorla öldürmek değil, işbirlikli iptal ve cevabı uygulamama davranışı sağlar. |
| Sunucu işleri | [jobs.rs](../crates/server/application/src/jobs.rs), 1–25; [jobrun](../apps/jobrun/src/main.rs) | Lease/fence/heartbeat, staged artifact, revision/yetki kontrollü commit ve ayrı hesap süreci temeli mevcut. Sıfırdan ikinci job sistemi kurulmaz. |
| MCP | [tools.rs](../crates/native/mcp/src/tools.rs), 341–387, 577–594; [server.rs](../crates/native/mcp/src/server.rs), 16–19, 77–85 | Katalogdan araç üretiliyor; modern `2026-07-28` ve üç legacy sürüm var. Araç listesi başlangıçta hazırlanıyor; plugin kaynaklı dinamik yaşam döngüsü ayrıca tasarlanmalı. |
| KentOS UI | [widget/mod.rs](../crates/ui/src/widget/mod.rs), 95–169; [form.rs](../crates/ui/src/widget/form.rs), 41–55, 85–99 | Zengin Rust/Iced bileşenleri var. Rust Element/Message kabul eden API, dış Python UI descriptor'ı değildir. |
| CBS form kaynağı | [forms.rs](../crates/shared/contracts/src/forms.rs), 557–578, 679–753, 1212–1262; [building.py](../python/examples/forms/building.py), 32–63 | Sekme/grup, 15 widget, action ve Processing bağı; Python form örneği var. Genel eklenti arayüzü için ikinci domain/ACL sistemi gerekmiyor. |
| AI Surface | [ADR0154](../docs/adr/0154-ai-surfaces.md), 10–18, 64–67 | Sağlayıcı bağımsız UI/durum bileşenleri var; bu, model çağıran ve araç yürüten genel ajan host'unun tamamlandığı anlamına gelmez. |
| Gerçek yeni ürün parçaları | Yukarıdaki ürün yolları ve [python.md](python.md) karşılaştırması | İncelenen yollarda tam `.kek` yöneticisi, UV ürün akışı, `kentos.ui` Python katmanı ve dinamik native Python provider yaşam döngüsü bulunmadı. |

Form ilişkileri ve çıktı işleri gibi alanlarda eski Python planının bazı
“yok” kayıtları artık güncel değildir. Yeni iş başlamadan ilgili `PY-*`
maddesinin kod/ADR karşılığı yeniden okunmalıdır; eski tabloya bakılarak çalışan
altyapı değiştirilmemelidir.

## 3. Uçtan uca çalışma modeli

Kullanıcı akışı: Eklentiler → eklentinin türüne uygun arayüz/araç → doğrulama ve
önizleme → gerekiyorsa arka plan işi → sonuç inceleme → ortak komutla uygulama.

Tek kayıtlı eklenti kimliği; komut, UI kaynağı, Processing provider, yardım ve
MCP keşfini birbirine bağlar. UI ev sahibi KentOS'tur; Python hesap ve callback
süreci ayrı yaşar. Belge değişikliğinin otoritesi mevcut uygulama/domain
komutlarıdır. Eklenti doğrudan canlı Document, SQL bağlantısı veya UI widget
referansı alarak iş kurallarını atlamaz.

Desktop, headless ve bulut yürütücüleri aynı şey değildir. Araç hangi ortamda
çalışabildiğini bildirir. Masaüstü Python eklentisi kendiliğinden web/mobilde
çalışmış sayılmaz; portable tanım ile Python yürütücüsü desteği ayrıdır.
Web uygulama işi mevcut ADR0228 sınırını korur.

## 4. Yedi uygulama paketi

### EKL-01 — Her aracı, her seçeneği ve bütün otomasyon yüzeyini eşleştir [ ]

Eski görevler: `ECO-API`, `PY-AI`, `ECO-QA`. Başlangıç kaynağı
[CAD manifesti](../docs/cad/manifest.md), [operations.json](../catalog/v1/operations.json)
ve gerçek komut/dispatcher yollarıdır. Manifestte yazılı eski sayılar yeni
denetimin sonucu gibi kullanılmaz.

- GUI komut kaydı ile ürün komut kataloğunu ayrı incele. Bir düğmenin kayıtlı
  olması anlamsal API'sinin tamam olduğunu kanıtlamaz. Çizim/değiştirme, seçme,
  sorgulama, stil/kat/pafta, dosya ve eklenti yaşam döngüsü birlikte taransın.
- Her işlem **ve seçenek** için eşleme raporu üret: GUI → ürün komutu veya
  host eylemi → girdi/çıktı/plan şeması → Python → MCP → eklenti → job/iptal
  → undo → yetki → doküman → kabul kanıtı. Eksik parametre ve farklı anlam
  ayrı bulgu olsun. Genel bir “execute” aracının varlığı tamlık sayılmasın.
- Çizimi değiştiren iş semantik komuta gitsin; Python/AI geometriyi yeniden
  hesaplamak zorunda kalmasın. Seçim, kamera, araç başlatma, panel açma gibi
  oturum işleri için ayrı tipli host sözleşmesi olsun. Headless'ta fiziksel
  pencere olmayan iş açık capability/ret ile açıklansın; işin anlamsal
  karşılığı mümkünse GUI açmadan yürüsün.
- Mevcut Python yankısı aynı komut girdilerinden gelsin; açık/kapalı tercihi
  korunsun. Eksik okuma/seçim/düzenleme yolları raporlansın. Gizli alanlar,
  anahtarlar ve yerel kimlikler yankıyla açığa çıkmasın.
- Her yeni özellik bu eşlemeyi aynı teslimde tamamlasın. Belge/SDK/MCP/eklenti
  desteği sonradan yapılacak isteğe bağlı iş olmasın. Eksik olan bölümün
  gerekçesi ve engellediği ürün kapsamı görünür kalsın.

**Kabul:** Aynı konut işlemi GUI/Python/MCP/eklenti üzerinden yürütülsün;
normalize edilmiş model, birimler, ret/uyarı, kalıcı kimlik, tek Undo ve
kaydet–aç sonucu eşdeğer olsun. Yüzeylerin farklı çağrı tutamaçları model
farkı sayılmasın; semantik veri farkı gizlenmesin.

### EKL-02 — Eklenti türlerini ve Eklentiler sekmesini tek kayıtla yönet [ ]

Eski görevler: `PY-RUN`, `PY-UI`, `PY-TOOL`. Modül sınırları için mevcut
[module_surfaces.rs](../apps/desktop/src/module_surfaces.rs) ve komutların
`offers/module_has` davranışı korunur.

| Katkı türü | Kullanıcının erişimi | Ortak yürütme |
| --- | --- | --- |
| Komut/menü eylemi | Eklentiler'de ana giriş; uygun bağlam menüsünde ek erişim | Aynı sürümlü komut kaydı |
| Dialog/form veya dock/panel | Eklentiler → Aç; açık panelin kendi yaşam döngüsü | KentOS UI host + eklenti oturumu |
| Processing provider/algoritma | Eklentiler'de provider ve araçlarına erişim; İşlemler kataloğu/model/batch; uygun nesne bağlamı | Aynı araç kimliği ve runner |
| Portable CBS form kaynağı | Eklentiler'de kaynak yönetimi; veri girişinde normal form akışı | Mevcut FormBody/domain/yayın/izin modeli |
| Etkileşimli çizim aracı | Eklentiler'de başlangıç; açık araç ayarları ve komut istemi | Host giriş/önizleme protokolü + semantik ürün komutu |

- Her kurulu eklenti Eklentiler'de adı, yayımlayıcı, sürüm, durum, yardım,
  ayar, aç/çalıştır, devre dışı bırak/kaldır ile bulunabilsin. Python kapalı
  veya bağımlılık eksikse yönetim ekranı çalışsın; neden kullanılamadığı söylensin.
- Bir paket birden fazla katkı sunabilir. Tip yalnız bir etiket değildir;
  açılış, parametre toplama, sonuç sunumu ve kapatmanın davranışını belirler.
  Eklenti kendiliğinden yeni üst ribbon sekmeleri ve rastgele araç kalabalığı yaratmasın.
- Topoloji denetimi gibi bir araç, analizi uygunsa Processing algoritması
  olarak; özel inceleme gerekiyorsa aynı komuta bağlı panel/menü eylemi olarak
  sunulabilir. Sekmedeki ve İşlemler'deki iki giriş ayrı hesaplayıcı olmasın.
- Katkılar desteklenen Mimari/Harita/GIS modülünü ve gerekli veri türlerini
  bildirsin. Başka modüle ait araç yanlış aramada/bağlamda sunulmasın; eklenti
  ana kaydı görünür kalıp bu modülde neden çalışmadığını açıklayabilsin.
- Kayıtların sahibi plugin kimliği+sürüm+runtime generation olsun. Disable,
  crash, update ve uninstall; araç, menü, provider, callback ve abonelikleri
  temizlesin. Paket içe aktarmak/metadata okumak kodu import etmek anlamına gelmesin.
  Komut/araç kimlikleri eklenti ad alanında olsun; hazır komutu gölgeleyemesin.

**Kabul:** Aynı eklenti panel ve iki Processing aracı sunsun. Eklentiler,
İşlemler, komut arama ve uygun bağlamdan aynı kimliğe gidilsin. Devre dışı
bırakılınca hayalet komut/callback kalmasın; ilgili olmayan modülün yüzeyi bozulmasın.

### EKL-03 — KentOS UI bileşenlerine tam, sürümlü Python bağları sağla [ ]

Eski görevler: `PY-UI`, ilgili `ECO-API`. Önerilen `kentos.ui` katmanı
Python'da tipli bir tanım üretir; mevcut Rust UI host'u onu kendi bileşenleriyle
oluşturur. Bu isim ve aşağıdaki sözleşme henüz uygulanmış değildir.

- KentOS UI bileşen envanterini çıkar; her bileşenin property/event/action ve
  erişilebilirlik davranışını Python şemasına eşleştir. Metin/sayı/birim,
  seçim/tarih, sekme/grup/grid, tablo/tree, dosya/katman/nesne seçici,
  dialog/dock, progress/log/cancel ve AI bileşenleri kapsansın. Birkaç basit
  widget bağlayıp “tam UI API” denmesin; özel çizim/editör bileşenlerinin
  destek sınırı ayrıca gösterilsin.
- Genel UI tanımı sürümlü olsun: kararlı widget/session/action kimlikleri,
  tipli değer/olay, doğrulama, salt okunur/etkin durum ve düzen bilgisi.
  Python ham Iced nesnesi/lifetime/Message alamaz. Geriye uyum ve desteklenmeyen
  widget için açıklayıcı ret/capability pazarlığı tasarlansın.
- Kullanıcının henüz uygulamadığı değer, odak, seçili metin ve kaydırma yeri
  patch geldiğinde korunsun. Her tuş belge komutu çalıştırmasın. Uygula ile
  giden taslak, önizleme ve kalıcı değişiklik ayrıdır; ağır callback işi başlatır.
- Aynı host'un tema, ölçü/boşluk, klavye, odak, dar pencere, font ve hata
  gösterimi kuralları kullanılsın. Python eklentisi ikinci bir tasarım dili
  yaratmasın. Tablo/listede büyük veriyi sayfalama ve sanallaştırmayla taşı.
- Portable CBS formlarının mevcut schema/domain/ilişki/ACL anlamını tekrar
  yazma. Eklenti panelinde gizlenen alan erişim yetkisi değildir; veri okuma
  ve yazma mevcut yetkili katmandan geçer. Python gerektirmeyen form açılışı
  Python kurulumu başlatmasın.
- Pencere kapanması, eklenti unload'u ve açık belgenin değişmesi geç callback'i
  etkisiz kılsın. Bir kullanıcının taslağı başka belgeye veya yeniden açılmış
  aynı adlı pencereye uygulanmasın. Host'un temizlediği kaynaklar test edilsin.

**Kabul:** Python'dan dialog+dock+tablo+katman seçici üret; açık/koyu tema,
dar pencere ve büyük yazıda gerçek fare/klavyeyle kullan. Yanlış sayı,
Tab/Enter/Esc, giriş sırasında patch, kapat/unload ve geç callback sınansın.
Bileşen kapsam raporu ile üretilen API referansı aynı sürümü göstersin.

### EKL-04 — Processing ve ağır analizleri gerçekten durdurulabilir çalıştır [ ]

Eski görevler: `PY-RUN`, `PY-TOOL`, `ECO-QA`. Mevcut
[Processing sözleşmesi](../docs/PROCESSING.md), snapshot/changeset ve job
altyapısı genişletilir; ayrı Python konsoluna bağlanmak tek başına çözüm sayılmaz.

- Native Tool'daki Rust function pointer ile dış Python executor ayrıştırılsın.
  Portable descriptor; kimlik/sürüm/kategori, yardım, parametreler, kaynak
  türleri, kapsam, birim/CRS, validate/preview, yürütücü ve typed çıktıları
  bildirsin. Dış API'de internal Slot/Document referansı yerine UUID, snapshot
  ve kontrollü veri erişimi olsun.
- Girdi; katman/seçim/nesne/kapsam ve kayıt sürümüyle belirlenir. Çıktı;
  yeni katman, mevcut katmana kontrollü değişiklik, geçici sonuç, tablo/sayı,
  dosya/asset veya rapor olabilir. Hedef, overwrite ve sahneye ekleme davranışı
  başlamadan bilinsin. Model/batch ve eklenti aynı sonuç sözleşmesini kullansın.
- Python analizi ayrı süreçte; hazır Rust hesapları uygun mevcut job yolunda
  çalışsın. NumPy ve benzeri seçilmiş paketlerin hesap gücü kullanılabilsin.
  Büyük diziler için ölçülmüş toplu/binary aktarım, sınırlı kuyruk ve bellek
  bütçesi kullan; nesne başına sınırsız JSON veya UI thread'inde uzun RPC yürütme.
- İş durumu/aşama, ölçülebilir ilerleme ve belirsiz bekleme ayrıştırılsın.
  İptal önce cooperative token; yanıt vermeyen Python/native eklenti hesabı
  kendi süreç ağacında watchdog ile sonlandırılabilsin. Başlatılan KentOS
  alt işleri de kendi token'larıyla iptal edilsin. Python'u öldürmek başka
  Rust thread'ini veya sunucu işini bitirmiş kabul edilmesin.
- Sonuç staging'de tutulsun. Uygularken belge/session, revision, yetki ve
  kilitler tekrar doğrulansın; tek changeset/Undo ile yazılsın. İptal, çökme,
  belge kapanması ve geç cevap yarım katman bırakmasın. Commit kazanmışsa
  “iptal edildi” yalanı söylenmesin; mevcut `already_committed` anlamı korunsun.
  Canlı belge işlemi Python callback'i veya uzun hesap boyunca açık tutulmasın;
  değişiklik kısa, atomik uygulama aşamasında gerçekleşsin.
- Uzun hesap boyunca görüntüleme/pan/zoom/iptal çalışsın. Düzenleme güvenle
  desteklenmiyorsa yalnız ilgili belge için nedenli kısıt koy; bütün uygulamayı
  kilitleme. İleride revision çatışmasıyla eşzamanlı düzenlemeyi desteklemek
  ayrı bir kabul olsun.

**Kabul:** Büyük topoloji taraması, Python CPU döngüsü, uzun native NumPy
çağrısı, takılan ağ, yoğun log ve alt süreç vakalarında durdur. İlk mühendislik
hedefi iptal isteğinin 100 ms içinde görünmesi, cooperative hesabın 1 s içinde
durması, yanıt vermeyen plugin sürecinin en çok 5 s içinde sonlandırılmasıdır;
ölçülmüş garanti değildir. Platform/veriyle doğrulanamayan paket hazır sayılmaz;
kaynak temizleme süresi ayrıca raporlanır. Aynı işi GUI/Python/MCP/modelden
iptal ederek belge ve dosya bütünlüğünü karşılaştır.

### EKL-05 — UV, analitik ortamlar ve `.kek` kurulumunu kullanıcı seçimine bağla [ ]

Eski görevler: `PY-OPT`, `PY-UV`, `PY-KEK`, `PY-RUN`.

- Python/UV yokken CAD/GIS açılışı ve hazır Rust araçları aynı şekilde çalışsın.
  UV algılama; PATH, kullanıcının seçtiği yol, sürüm, OS/CPU ve GUI'nin shell'den
  farklı ortamını hesaba katsın. Bulunamazsa Windows/macOS/Linux için resmî
  kurulum seçenekleri ve yeniden algılama sunulsun; kurulum kullanıcıya bağlı olsun.
- Kişisel analitik ortam, plugin geliştirme ortamı ve yayımlanmış sürümün
  çalışma ortamı ayrıdır. Kullanıcı NumPy'ı kişisel ortamına kolayca ekleyip
  deneyebilsin; bu deneme başka eklentinin kilitli ortamını değiştirmesin.
  Çelişen paketler aynı interpreter namespace'ine sokulmasın.
- Eklenti Python sürümü, doğrudan/geçişli bağımlılık, seçilen extras,
  paket kaynağı, native/wheel/platform gereksinimi ve hesaplanabilen indirme/
  disk tahminini kurulumdan önce göstersin. Bilinmeyen boyut veya desteklenmeyen
  wheel açık söylensin. Reddetme eklentiyi nedenli pasif bırakır; açılışta otomatik indirme yapmaz.
- Ortam hazırlama ile çalıştırmayı ayır. Release `pyproject.toml` ve
  `uv.lock` tutarlılığı doğrulansın; çalışma aşaması sessiz lock güncellemesi,
  sync veya interpreter indirmesi yapmasın. UV belgelerine göre `--locked`
  güncelliği denetler, `--frozen` bu denetimi atlar, `--no-sync` ortam
  senkronizasyonunu atlar; bunlar birbirinin eşdeğeri değildir.
- `.kek` sürümlü, deterministik sıkıştırılmış paket olsun: manifest, Python
  kaynakları, UI/tool/form tanımları, kaynaklar, yardım, `pyproject.toml`,
  lock ve içerik özetleri. Kimlik/sürüm/yayımlayıcı, minimum KentOS API/UI/tool
  sürümleri, modüller, izinler ve entrypoint'ler açık bildirilsin. `.venv`,
  API key, kişisel dosya ve mutlak kullanıcı yolu pakete girmez.
- Paket doğrulama/import, bağımlılık hazırlama, enable ve run ayrı durumlardır.
  Arşiv çıkarma yol/boyut sınırlarıyla yapılır; metadata okumak kod çalıştırmaz.
  Ortam anahtarı kullanıcı+plugin sürümü+lock özeti+Python ABI+OS/CPU+extras
  içerir. Universal lock, her platformda native paket bulunacağının garantisi değildir.

**Kabul:** UV olmayan kullanıcı kurmayı reddedip normal çalışır; isteyen
kullanıcı NumPy kurar, sonra aynı bağımlılığı `.kek` ile başka kullanıcıya
bildirir. İki çelişen sürümlü eklenti birbirini bozmaz. Offline, yarım indirme,
disk yetersizliği, uyumsuz wheel ve iptal durumunda eski çalışan ortam kalır.
Bu incelemede hiçbir kurulum yapılmamıştır.

### EKL-06 — AI ile eklenti üretimini gerçek API dokümanı ve keşifle destekle [ ]

Eski görevler: `PY-AI`, `ECO-API`, `ECO-QA`. AI Surface görünümü, model
sağlayıcısı ve araç yürütücüsü ayrı sorumluluklardır.

- Katalog/şemadan sürüme bağlı Python API referansı, tip stubları ve keşif
  indeksi üret. Her API'nin parametre/birim/varsayılan, çıktı, hata/uyarı,
  validate/plan/execute, undo, job/iptal ve host/headless/platform kapsamı
  açıklansın. Var olan docstring üretimi bu sitenin kaynağı olarak genişletilsin.
- Kavramsal başlangıç, KentOS UI cookbook, Processing provider, seçim/katman
  erişimi, paketleme, hata ayıklama, sürüm geçişi ve çalışır örnek eklentiler
  ayrıca yazılsın. Örnekler yalnız ekran metni değil, ilan edilen SDK sürümüyle
  test edilen kaynaklar olsun; offline belge paketi de sağlansın.
- AI akışı: kurulu capability/API/doc keşfi → istek/parametre açıklığı →
  eklenti taslağı → bağımlılık/izin planı → doğrulama/test → `.kek` üret →
  kullanıcı incelemesi/yerel etkinleştirme → istenirse yayına gönderme.
  AI kurulmamış API uydurduğunda doğrulama bunu yakalasın; eksik özellik
  geometriyi gizlice başka yoldan yazdırarak geçilmesin.
- Uygulama içi ajan; sağlayıcı/model adapter'ı, kullanıcı tarafından girilen
  API key veya sağlayıcının desteklediği kimlik yoluyla yapılandırılsın.
  Anahtarlar OS secret store'da; proje, `.kek`, konsol ve log dışında tutulsun.
  Sağlayıcı bağımsızlığı her sağlayıcının aynı özellikleri sunduğu iddiası
  değildir; tool calling/streaming/context capability'si açık olsun.
- Dış MCP istemcisiyle bağlanma ikinci bağımsız ürün yoludur; yerleşik ajan
  zorunlu değildir. İki yol aynı komut, belge kapsamı, gizli alan redaksiyonu,
  preview ve izinleri kullanır. Çalışan işler, maliyet/istek bütçesi ve
  gönderilecek çizim verisinin kapsamı kullanıcıya anlaşılır gösterilsin.
- Plugin komutu/UI/tool descriptor ve doküman sürüm/özet ile bağlansın.
  Yükleme/kapatmada MCP keşfi güncellensin; sayfalama ve liste değişikliği,
  uygulanan modern ve legacy protokol sürümlerinin kendi kurallarına uysun.
  Salt bir araç adı değil tipli sonuç ve anlamlı hata da AI'a dönsün.

**Kabul:** Yerleşik sağlayıcı adapter'ı ve bağımsız bir dış MCP istemcisi
aynı küçük eklentiyi kurulu dokümandan üretip doğrulasın. Uydurulmuş API,
eksik dependency, eski şema ve kapalı belge açıklanabilir biçimde reddedilsin.
Eklenti README'si veya çizim metni, kod yürütme/kurulum/yayın yetkisi olmasın.

### EKL-07 — Yerel yaşam döngüsü, onaylı bulut yayını ve güncelleme [ ]

Eski görevler: `PY-CLOUD`, `PY-KEK`, `PY-RUN`, `ECO-QA`.

- Eklentinin keşfedildi, gereksinim bekliyor, hazırlanıyor, hazır, çalışıyor,
  durduruluyor, hata verdi, devre dışı ve güncelleme bekliyor durumları
  tutarlı olsun. Her durumda yardım, log ve çözüm yolu bulunsun; durum değişikliği
  UI/Python/MCP'de aynı anlamla görülsün.
- Yerel `.kek` kullanılabilsin; bulut hesabı/yayın zorunlu olmasın. Dağıtılan
  kaynakların lisans ve bağımlılık bilgisi paketle taşınsın. İmzalı veya
  incelenmiş paket “hiç risk yok” ya da “OS sandbox var” anlamına gelmez.
- Bulut yayın akışı: taslak yükle → içerik/dependency/platform testi →
  inceleme → kabul/ret ve gerekçe → yayımlanmış değişmez sürüm. Her yeni
  sürümün içerik özeti onaylanır; onaylı paketin aynı sürüm altında değiştirilmesi
  engellenir. Kullanıcı isteği olmadan yerel paket buluta gönderilmez.
- Uyumluluk ve istenen erişimler yüklemeden önce gösterilsin. Kurum politikası,
  kullanıcı seçimi ve eklentinin talebi birlikte değerlendirilir. Ayrı süreç
  ve venv bir güvenlik sandbox'ı değildir; güçlü dosya/ağ izolasyonu vaadi
  verilecekse gerçek OS düzeyindeki uygulaması ve platform testi ayrı teslimdir.
- Güncelleme yeni ortamda hazırlanıp smoke testten sonra atomik etkinleştirilsin.
  Çalışan işler eski sürüm lease'inde kalsın veya açıkça durdurulsun. Hata
  durumunda önceki paket/ortam geri alınabilsin. Kaldırma; komut/provider/UI
  kayıtlarını temizlesin, kullanıcının çıktı verisini sessiz silmesin.
- Yetki iptali, paket geri çekme, crash ve belgenin kapanması geç cevabı veya
  eski callback'i yeniden etkinleştirmesin. Ağır işin bitmesiyle sonuç commit'i
  ayrı durumlar olsun; hangi sürüm/parametre/veriyle üretildiği kaydedilsin.

**Kabul:** Yerel yükle–çalıştır–durdur–disable–enable–update–rollback–uninstall
akışı gerçek UI'de geçsin. Bulut testinde onaysız sürüm kurulabilir genel
kataloğa çıkmasın; kabulden sonraki içerik değişikliği yeni inceleme istesin.
Yarım güncelleme çalışan eski eklentiyi bozmasın.

## 5. Önce hangi dilim teslim edilecek?

| Sıra | Somut çıktı | Tamamlanma kapısı |
| --- | --- | --- |
| 0 | EKL-01 araç/parametre haritası ve destek matrisi; UI/tool/broker şema kararı | Mevcut yollar ve açıklar tek raporda; bugün olmayan API bugün varmış gibi belgelenmez. |
| 1 | Tek kullanıcıda yerel `.kek`; NumPy kullanan tek Processing algoritması ve KentOS parametre dialog'u | UV seçimi, dependency açıklığı, ayrı süreç, gerçek iptal, typed sonuç, yeni katman, tek Undo; Python/MCP eşdeğerliği. |
| 2 | Aynı pakete dock/panel ve etkileşimli olmayan ikinci komut; kayıt/unload | Eklentiler/İşlemler/bağlam tek kayıt; crash ve geç callback temizliği; GUI kalite matrisi. |
| 3 | UI bileşen aileleri ve mevcut araçların eksik API seçenekleri | Coverage raporu ad ve parametre düzeyinde tamam; her dilimin API/doküman/AI erişimi birlikte kabul. |
| 4 | Dokümandan AI eklenti üretimi ve sağlayıcı adapter'ı; dış MCP | Çalışan örnekler, sürüm uyumu, yetki/veri kapsamı, keşif güncellemesi. |
| 5 | Onaylı bulut katalog/yayın/güncelleme | Yerel runtime kabulünü takiben sürüm/digest incelemesi ve rollback. |

Birinci dilim “üç widget'lı demo” ile kapanmaz: ikinci kullanıcıya kurulabilen,
iptal edilen, gerçek çıktı üreten, tekrar çalıştırılan ve kaldırılabilen
paket olmalıdır. Sonraki zengin bileşen kapsamı ayrıca açık kalır.

Her teslimde ilgili Rust/Python/komut ve fixture testleri, hata/iptal/
revizyon/yetki/Undo, gerçek fare/klavye ve ekran görüntüsü kontrolü yapılır.
Uzun iş sırasında UI yanıt süresi, iptal gecikmesi, CPU/bellek ve aktarım
boyutu ölçülür. Ham görüntüler repoya eklenmez. Sadece plan tesliminde kod
testleri çalıştırılmış gibi raporlanmaz.

## 6. Daha sonra kurallara taşınacak kısa bakım ilkesi

Kullanıcı kararı gereği **`CLAUDE.md` bu çalışmada değiştirilmeyecektir**.
İleride ayrı bakım işinde yapılacak düzenleme burada kayıtlıdır:

- Dosyada yalnız kalıcı geliştirme kuralları, güncel kapsam sınırı ve kısa
  yönlendirmeler kalsın. Uzun tamamlanan iş anlatıları ADR/baseline kaydına;
  yapılacak işler ilgili plan/TODOS başlığına taşınsın.
- Kod/ADR/plan arasında tek kaynak ve bağlantılar korunsun; aynı özellik için
  çelişen üç yapılacaklar listesi oluşturulmasın. Kullanıcının geçerli kararları
  ve veri/izin kuralları kısaltma uğruna kaybolmasın.
- Yeni işin kısa tamamlanma kuralı: **anlamsal komut + Python + MCP/AI +
  eklentiden erişim + sürümlü doküman + ilgili test/GUI kanıtı**. Host'a özgü
  davranışlar açık capability ile ayrılır; veri işlemleri sessizce hariç tutulmaz.

Bu maddeler bugün etkin bir dosya taşıma/kural değişikliği değildir; sonraki
uygulama için planlanmıştır. Mevcut görevde yalnız bu iki yeni plan dosyası
teslim edilir.

## 7. Resmî araştırma dayanakları

- [QGIS 3.44 — Processing plugin](https://docs.qgis.org/3.44/en/docs/pyqgis_developer_cookbook/processing.html):
  provider kayıt/temizliği ve toolbox/model/batch entegrasyonu. KentOS'un tek
  registry kararına dayanak; Qt sınıflarını doğrudan taşımak önerilmez.
- [QGIS 3.44 — Processing scripts](https://docs.qgis.org/3.44/en/docs/user_manual/processing/scripts.html):
  tipli parametre/çıktı, yardım ve iptal edilen algoritma davranışı.
- [QGIS 3.44 — Tasks](https://docs.qgis.org/3.44/en/docs/pyqgis_developer_cookbook/tasks.html):
  arka plan işinde ana thread nesneleri ve kaynak yaşam döngüsü sınırı.
- [UV — kurulum](https://docs.astral.sh/uv/getting-started/installation/),
  [Python sürümleri](https://docs.astral.sh/uv/concepts/python-versions/),
  [lock/sync](https://docs.astral.sh/uv/concepts/projects/sync/),
  [resolution](https://docs.astral.sh/uv/concepts/resolution/): isteğe bağlı
  ortam hazırlama ve kilitli çalışma ayrımı; evrensel çözüm ile platform
  uygunluğunun ayrı doğrulanması.
- [MCP 2026-07-28 — tools](https://modelcontextprotocol.io/specification/2026-07-28/server/tools):
  güncel araç keşfi, şema, sayfalama ve değişiklik bildirimi. Depodaki modern
  sürüm budur; legacy uyumluluk kaldırılmaz.
- [MCP 2025-11-25 — cancellation](https://modelcontextprotocol.io/specification/2025-11-25/basic/utilities/cancellation):
  iptal bildirimi ve geç sonuç yarışı; task iptaliyle istek iptalinin ayrımı.
  Bu kaynak legacy sürüme aittir; modern protokol adaptörü uygulanırken modern
  sürümün kuralları ayrıca izlenir. Protokol bildirimi işletim sistemi sürecini
  kendiliğinden sonlandıran bir mekanizma değildir.

Araştırma yöntemi: güncel kod ve mevcut plan karşılaştırması, resmî teknik
belgeler. Bu belgeler KentOS'un henüz yazılmamış eklenti katmanının çalıştığına
veya performans hedeflerinin ölçüldüğüne kanıt değildir.
