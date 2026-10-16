# KentOS CAD — Profesyonel 2D mimari çizim geliştirme planı

İnceleme tarihi: **12 Ekim 2026**. İnceleme sırasında görülen dal: `main`, HEAD: `5e1830e1`. Çalışma ağacında başka geliştirme çalışmaları devam etmektedir; bu belge, incelenen kaynakların bu tarihteki durumunu esas alır.

**Amaç:** KentOS'un CAD tarafını, AutoCAD'in profesyonel 2D çizim ve mimari dokümantasyon iş akışlarını karşılayabilen bir ürüne dönüştürmek. Arayüzden yapılan her ürün işlemi komut sistemi, Python API ve AI yüzeyinden de aynı kurallar, aynı doğruluk ve aynı sonuçlarla yapılabilmelidir.

**Bu teslimin kapsamı:** araştırma, mevcut kodun değerlendirilmesi ve uygulanabilir iş planı. Uygulama kodu değiştirilmemiştir. Bu inceleme sırasında uygulama testleri, AutoCAD üzerinde karşılaştırmalı çizim testleri ve performans ölçümleri çalıştırılmamıştır. Aşağıdaki kabul koşulları gelecek geliştirmede doğrulanacak hedeflerdir.

## 1. Ürün hedefi ve kapsam sınırı

- Profesyonel kullanıcı, gerçek ölçekte kat planı, yerleşim, kesit, görünüş, tavan planı, uygulama detayı, mahal/kapı/pencere çizelgesi ve pafta takımı hazırlayabilmelidir.
- Karşılaştırmanın referans sürümü AutoCAD 2026 ürün/geliştirici yardımıdır; bazı konu başlıklarında 2025 resmi anlatımları da kullanılmıştır. Temel CAD, LT, Mac ve Architecture sürümleri arasındaki farklar kaynak bağlamıyla değerlendirilmiştir.
- Günlük üretim; hassas çizim, hızlı düzeltme, blok ve referans kullanımı, ölçülendirme, tarama, çıktı, revizyon ve dosya teslimi boyunca kesintisiz olmalıdır.
- Genel 2D CAD temeli ile mimari nesneler birlikte geliştirilmelidir. Çizgiler ve bloklarla çalışan kullanıcı da duvar, kapı ve mahal nesneleri kullanan kullanıcı da desteklenmelidir.
- AutoCAD'in temel 2D CAD yetenekleri ile **AutoCAD Architecture toolset** yetenekleri ayrı kıyaslanmalıdır. Duvar–kapı ilişkisi, mahal ve mimari çizelgeler Architecture tarafının örnekleridir; temel AutoCAD'de bunların hepsi aynı biçimde bulunuyormuş gibi değerlendirme yapılmamalıdır.
- İşlev ve iş akışı eşdeğerliği hedeflenmelidir. AutoCAD'in arayüzünü, kodunu veya lisanslı içerik kütüphanesini kopyalamak bu hedefin gereği değildir.
- DWG/DXF uyumluluğu, mevcut `.kcad` kaynak modelini ortadan kaldırmamalıdır. Bir dış biçimin desteklenmesi, o biçimin bütün özel nesnelerinin düzenlenebilir olarak desteklendiği anlamına gelmemelidir.
- CAD/GIS ortaklığında geometri, kimlik, sayısal politika, dosya ve yetki altyapısı korunmalıdır. Mimari çizim için GIS algoritmalarının yeniden geliştirilmesi bu planın önceliği değildir.
- Tam BIM, 3D katı modelleme, render, yapısal hesap ve otomatik mevzuat uygunluk kararı ayrı kapsamdır. 2D nesnelerde kat/kot/yükseklik gibi geleceğe yararlı alanlar bulunabilir; bunlar tek başına 3D model veya mevzuat doğrulaması sayılmamalıdır.
- Kesit ve görünüşün **elle 2D çizilmesi** temel hedefe dahildir. Kat planından tam otomatik kesit/görünüş üretimi, gerekli düşey veriler tanımlandıktan sonra ayrı ve açık bir ileri teslim olmalıdır.
- Web ve masaüstünün yayımlanmış CAD yetenekleri aynı davranış sözleşmesini izlemelidir. Tarayıcıya özgü dosya/işletim sistemi sınırları açık capability olarak bildirilmeli; desteklenen veri işlemleri için Python/AI eşdeğerliği sağlanmalıdır.

## 2. İnceleme yöntemi, mevcut temel ve kanıtlar

### 2.1 Nasıl okunmalı?

- **Var:** ilgili sözleşme, hesap veya uygulama yolu kaynakta bulunmuştur. Bu ifade bu oturumda üretim kabulü geçtiği anlamına gelmez.
- **Kısmi:** temel vardır; hedef iş akışının bazı semantik, platform, kalıcılık veya otomasyon parçaları eksiktir.
- **Eksik:** incelenen temel model veya giriş yollarında hedef yeteneğin karşılığı bulunamamıştır; bazıları ayrıca kodda açıkça reddedilmektedir.
- **Yeniden doğrulanmalı:** eski belge/üretilmiş envanter ile güncel kaynak arasında fark vardır veya gerçek dosyalarla çalışma henüz sınanmamıştır.
- Bir araç adının bulunması bütün alt seçeneklerin API'de bulunduğunu kanıtlamaz. Bir API'nin son geometriyi yazabilmesi de o geometrinin nasıl hesaplanacağını kullanıcının veya AI'ın üstlenmesine gerek kalmadığını kanıtlamaz.

### 2.2 İncelenen ana kaynaklar

| Kanıt | Kaynak ve incelemenin konusu |
|---|---|
| E01 | [Cargo.toml](../Cargo.toml), [CLAUDE.md](../CLAUDE.md): Rust çekirdekleri, ayrı TS web ve Rust masaüstü, platform sınırları |
| E02 | [Web envanteri](../docs/inventory/web.md), [envanter yöntemi](../docs/inventory/README.md), [web.json](../docs/inventory/web.json): arayüz komutları, araçlar ve durumlar |
| E03 | [Ürün kataloğu](../crates/shared/contracts/src/catalog.rs), [üretilmiş katalog](../apps/web/src/contracts/generated/commandCatalog.json): sürümlü ürün komutlarının gerçek kapsamı |
| E04 | [Web ürün kaydı](../apps/web/src/product/registry.ts), [yerel dispatcher](../crates/native/headless/src/dispatch.rs): işleyici ve headless erişim |
| E05 | [Entity sözleşmeleri](../crates/shared/contracts/src/entity.rs): eğri, blok yerleştirme, ölçü, tarama ve kılavuz modelleri |
| E06 | [Proje sözleşmesi](../crates/shared/contracts/src/document.rs), [yeni proje](../crates/native/project/src/new_project.rs), [ADR 0165](../docs/adr/0165-project-types-cad-gis.md): yerel CAD, birimler ve CAD/GIS eksenleri |
| E07 | [Düzenleme sözleşmesi](../crates/shared/contracts/src/cad_edit.rs), [oluşturma sözleşmesi](../crates/shared/contracts/src/cad_create.rs), [kenar araçları](../apps/web/src/tools/edgeTools.ts): hesaplanan geometriyi komuta verme yaklaşımı |
| E08 | [Dizi sözleşmesi](../crates/shared/contracts/src/cad_array.rs): dikdörtgen, kutupsal ve yol boyunca kopyalar |
| E09 | [Spline hesabı](../crates/shared/geometry-core/src/geom/spline.rs): fit noktalarından Catmull–Rom ve Bézier karşılığı |
| E10 | [Katman sözleşmesi](../crates/shared/contracts/src/layer.rs): CAD çizgi türleri ve katman görünüşü |
| E11 | [DXF okuyucu](../crates/shared/formats/src/dxf/mod.rs), [DXF dönüştürme](../crates/shared/formats/src/dxf/emit.rs), [DXF yazıcı](../crates/shared/formats/src/dxf/writer/mod.rs): desteklenen veri ve bildirilen kayıplar |
| E12 | [Python Document](../python/kentos/cad/document.py), [yerel Python bağlayıcısı](../crates/native/python/src/lib.rs), [SDK üreticisi](../scripts/python/sdk.py): gerçek Python girişleri ve üretilmiş tipler |
| E13 | [Headless oturum](../crates/native/headless/src/session.rs), [RPC](../crates/native/headless/src/rpc.rs), [sorgu](../crates/native/headless/src/query.rs): dosya, sayfalama, ölçme, grup ve undo |
| E14 | [MCP araçları](../crates/native/mcp/src/tools.rs), [masaüstü bağlantısı ADR](../docs/adr/0137-agents-on-the-desktop.md): dış ajan erişimi |
| E15 | [AI Surface ADR](../docs/adr/0154-ai-surfaces.md), [showcase AI](../apps/ui-showcase/src/ai.rs): yerel AI bileşenleri ve örnek sağlayıcı |
| E16 | [Web pafta deposu](../apps/web/src/product/sheet/store.ts), [masaüstü paftaları](../apps/desktop/src/sheets.rs), [pafta tasarımı](../docs/sheet/design.md): pafta kalıcılığı ve uygulama bağlantısı |
| E17 | [Pafta çekirdeği](../crates/shared/sheet/src/lib.rs), [masaüstü PDF](../apps/desktop/src/sheet_pdf.rs), [web pafta komutları](../apps/web/src/app/sheet/commands.ts): mevcut düzenleme ve çıktı altyapısı |
| E18 | [Yazı/ölçü stilleri ADR](../docs/adr/0183-text-and-dimension-styles.md), [annotation ölçek hesabı](../crates/shared/contracts/src/annotation_scale.rs): stiller, override ve proje ölçeğini izleme |
| E19 | [Python konsolu ADR](../docs/adr/0132-desktop-python-console.md), [Python yüzeyi ADR](../docs/adr/0150-python-surfaces.md): ayrı süreç, durdurma, editör ve tamamlama |
| E20 | [Mevcut yol haritası](../TODOS.md), [performans kayıtları](../docs/perf/README.md), [komut fixture'ları](../fixtures/commands/README.md): önceki işler ve doğrulama altyapısı |

### 2.3 Mevcut durumun sayısal özeti

- İncelenen üretilmiş web envanteri **595 arayüz komutu**, **131 araç**, **137 işlem aracı** ve **143 pencere/panel** içeriyor. Web durumunda 572 komut `implemented`, 23 komut `pending` olarak işaretli. Bunlar CAD ile GIS'in birlikte sayılarıdır; yalnız mimari CAD kapsamının sayıları değildir. [E02]
- İncelenen ortak ürün kataloğu **45 komut** içeriyor: **24 web/masaüstü çizim komutu**, **21 sunucu/proje komutu**. Genel `cad.entities.create/edit/transform` gibi komutlar çok sayıda alt işlemi taşıyor. [E03–E04]
- Dolayısıyla `595 − 45` hesabı “550 eksik API” şeklinde kullanılamaz. Gerçek kapsam, **işlem + seçenek + nesne türü + host + davranış** seviyesinde ölçülmelidir.
- Eski ADR'lerde geçen 33/35 Python komutu veya 12/14 MCP çizim komutu sayıları tarihsel kayıttır; güncel kapsamı değerlendirmek için üretilmiş katalog ve dispatcher esas alınmalıdır.
- Envanterde çok sayıda `sheet.*` masaüstü karşılığı eksik görünmesine rağmen güncel kaynakta `kentos-sheet-ui`, masaüstü pafta bağlantısı ve PDF üretimi bulunuyor. “Masaüstünde pafta/PDF yok” sonucu çıkarılmamalı; envanterin bu alanı gerçek kaynak ve çalıştırılacak kabul testleriyle yenilenmelidir. [E02, E16–E17]

### 2.4 Korunacak mevcut yetenekler

- Çizgi, çoklu çizgi, kapalı alan, nokta, daire, yay, elips, spline, yardımcı çizgi/ışın, dikdörtgen, düzgün çokgen, halka ve revizyon bulutu araçları mevcut.
- Taşıma, kopyalama, döndürme, ölçekleme, aynalama, öteleme, budama, uzatma, köşe yuvarlama, pah, kırma, birleştirme, patlatma, esnetme ve tutamaç düzenleme altyapıları mevcut.
- Dizi, kutupsal dizi ve yol boyunca dizi mevcut; bunlar aşağıda hedeflenen kalıcı ilişkili dizi ile karıştırılmamalı.
- Çok sayıda ölçü türü, zincir/baz/hızlı ölçü, kılavuz, çok satırlı biçimli yazı, yazı ve ölçü stilleri, CAD tabloları mevcut.
- Desenli/degradeli ve sınır nesnelerini izleyebilen tarama; blok tanımları, yerleştirmeleri ve öznitelikleri; katman durumları ve gelişmiş seçim araçları mevcut.
- Yerel `SRID 0` CAD projeleri, mm/cm/m gösterimi, CAD/GIS'e göre eksen ve açı yazımı mevcut. Bunları sıfırdan kurulacak yetenek gibi planlamamak gerekir.
- GPU çizim, ortak Rust geometri, kalıcı UUID, transaction/undo, `.kcad` doğrulamalı kayıt, bulut ve çevrimdışı temel mevcut.
- Python SDK, masaüstü Python editörü/konsolu, headless host ve MCP üzerinden açık çizime erişim mevcut.

### 2.5 AutoCAD karşılaştırmalı eksik/iyileştirme matrisi

| Alan | KentOS'ta görülen durum | Tamamlanması gereken ana fark | Öncelik |
|---|---|---|---|
| Temel 2D çizim/düzenleme | Var [E05, E07] | Alt seçeneklerin tutarlı olması; gerçek mimari çizimlerle kullanım kabulü | P1 |
| Arayüz–Python–AI eşdeğerliği | Kısmi [E03, E07, E12–E15] | Bütün ürün işlemleri, semantik hesap, ayarlar ve oturum işlemleri | P0 |
| Ötele/buda gibi semantik API | Geometri yaması yazılabiliyor [E07] | Kaynak ID + işlem parametrelerinden sonucu ortak hesaplama | P0 |
| Birimler | Yerel mm/cm/m var [E06] | Python/MCP yeni proje girişlerinde birim; bütün sonuçlarda açık birim | P0 |
| UCS / yerel çalışma ekseni | Temel CAD/GIS eksen düzeni var [E06] | Kaydırılmış/döndürülmüş adlı 2D UCS ve açık API bağlamı | P1 |
| Genel NURBS spline | Fit noktası eğrisi var [E05, E09] | Derece, knot, kontrol noktası, ağırlık ve periodic veri | P1 |
| Çoklu çizgi genişliği | Temel yol/bulge var [E05, E11] | Sabit/değişken segment genişliği; DXF kaybının kapanması | P1 |
| CAD görünüş özellikleri | Katman/renk/kalınlık var [E05, E10] | BYBLOCK/BYLAYER, nesne linetype/scale, tam LIN deseni, draw order | P1 |
| Statik bloklar | Var [E05] | Blok içinde katman semantiği, bağımsız X/Y ölçek, blok düzenleme kipi | P1 |
| Dinamik/parametrik bloklar | Temel sözleşmede yok [E05] | Parametre, eylem, görünürlük, flip/stretch ve instance değerleri | P2 |
| İlişkili ölçüler | Ölçü koordinatlarını taşıyor [E05] | Nesne/alt geometri referansı, kopuk bağ uyarısı, yeniden bağlama | P0 |
| Çok ölçekli annotation | Proje ölçeğini izleme var [E18] | Aynı nesnenin farklı viewportlarda ölçek/konum temsili | P1 |
| Yazı/font | Biçimli yazı ve 7 font ailesi var [E05, E18] | Harici font çözümleme, SHX stratejisi, fields ve ileri MTEXT | P1 |
| Tarama | Desen/degrade/ilişki var [E05] | Eğri sınır doğruluğu, PAT, origin, boşluk toleransı ve ölçek | P1 |
| Kılavuz | Tek çizgi yolu/not modeli var [E05] | Çoklu oklar, rich content, blok içerik, bağlı uçlar | P1 |
| Geometrik/ölçüsel kısıtlar | Entity modelinde karşılık bulunmadı [E05] | Kısıt grafı, çözücü, parametreler ve teşhis | P2 |
| Mimari duvar/kapı/pencere/mahal | Temel Entity türlerinde yok [E05] | Kalıcı mimari nesne ve ilişkiler | P1–P2 |
| Xref | DXF'te açıkça atlanıyor [E11] | Bağlı belge, attach/overlay, reload/bind/clip ve çözümleme | P1 |
| DWG | Okuyucu DWG'yi açıkça reddediyor [E11] | SDK kararı, codec adapter, sürüm ve doğruluk matrisi | P0 karar / P1 teslim |
| DXF model/kâğıt uzayı | Model işleniyor, kâğıt uzayı atlanıyor [E11] | Layout/viewport ve gerçek pafta verisinin iki yönde aktarımı | P1 |
| Pafta/PDF | Ortak çekirdek ve iki host yolu var [E16–E17] | Ana projede taşınabilirlik, CAD viewport, API ve çıktı kabulü | P0 kalıcılık / P1 üretim |
| CTB/STB ve baskı | PDF temeli var [E17] | CAD plot style, sayfa ayarları, toplu publish ve ölçek güvencesi | P1 |
| PDF altlık/içe alma | Resim/raster var; CAD PDF akışı ayrıca doğrulanmalı | Sayfa seçimi, vektör altlık ve kontrollü nesneye dönüştürme | P2 |
| Çoklu belge | Genel yol haritasında açık [E20] | Belge sekmeleri, izolasyon, kopyalama ve otomasyon handle'ları | P1 |
| AI Surface | Bileşen ve showcase var [E15] | Gerçek sağlayıcı/uygulama host'u, bağlam, plan, yürütme ve eval | P0–P1 |
| Web Python | Mevcut ADR'lerde gelecek kapsam [E19–E20] | Worker transport ve aynı SDK semantiği | P1–P2 |

## 3. Tamlık tanımı ve değişmez mimari kurallar

### 3.1 “Tam ve kusursuz” nasıl ölçülecek?

- Yayımlanmış her CAD veri işleminin arayüz, komut satırı, Python ve AI karşılığı bulunmalıdır; alt seçenekler dahil kapsam **%100** olmalıdır.
- Aynı başlangıç belgesi, aynı açık bağlam ve aynı parametreler, bütün desteklenen host'larda aynı semantik belge sonucunu üretmelidir.
- ID üretimi gibi kontrollü farklar normalleştirilerek karşılaştırılmalı; koordinat, geometri, birim, ilişki, katman, stil, çizim sırası ve uyarı farkları gizlenmemelidir.
- Okuma, doğrulama ve önizleme; belgeyi, revision'ı, dirty durumunu ve undo geçmişini değiştirmemelidir.
- Başarısız/iptal edilen bir işlem yarım kalmış ilişkiler veya yarım çizim bırakmamalıdır.
- Bir işlemin undo/redo sonucu bütün bağlı nesneler, stiller ve ilgili proje verisi bakımından tutarlı olmalıdır.
- Dosya dönüşüm kayıpları açık raporlanmalıdır; desteklenen içerikte sessiz kayıp kabul edilmemelidir.
- “AI her zaman doğru anlar” gibi sınırsız garanti verilmemelidir. API kapsamı, geometri doğruluğu ve AI'ın doğal dilde doğru iş planlama başarısı ayrı ölçülmelidir.

### 3.2 Her arayüz eyleminin eşdeğeri

| Eylem sınıfı | Python/komut/AI karşılığı | Headless davranışı |
|---|---|---|
| Veri işlemi | Aynı semantik ürün komutu | Tam çalışır |
| Proje/stil/katman/pafta değişikliği | Tipli, yetkili ve sürümlü komut | Tam çalışır |
| Görünüm/seçim/drafting durumu | Açık oturum kimliğiyle session komutu | Sanal oturum veya somut koordinat/ID eşdeğeri |
| Etkileşimli araç | Araç oturumu + tipli adım girdileri; ayrıca doğrudan işlem API'si | Fare gerektirmeyen parametreli yol |
| Panel/dialog açma | Host UI komutu + aynı veri işleminin parametreli yolu | `requires_ui` yalnız pencere gösterme için |
| Dosya/yazıcı seçme | Kullanıcı seçimi veya path/asset/printer parametresi | Dosya/asset ile çalışır; fiziksel yazıcı host'a bağlı |

**Kural:** `requires_ui`, ölçü oluşturma, katman ekleme, pafta düzenleme veya PDF üretme gibi veri işlemlerinin API'siz bırakılmasını meşrulaştırmamalıdır. Her düğmenin pikselde tıklanmasını yeniden üretmek yerine yaptığı ürün işi erişilebilir olmalıdır; gerçek UI durumunu değiştiren eylemler de bağlı oturumdan yönetilebilmelidir.

### 3.3 Korunacak teknik yaklaşım

- Hesap tek Rust kaynağından gelmeli; web dar WASM adapter'larını, masaüstü/headless native adapter'ları kullanmalıdır.
- Web'in TS belge ve etkileşim katmanı korunmalıdır; web'e bütün masaüstü runtime'ı taşınmamalıdır.
- Native `Document` ile web `CadDocument` ayrı uygulamalar olarak ortak sözleşme ve fixture'larla aynı davranışı sağlamalıdır.
- Kalıcı mutation, ürün komutları ve belgenin transaction yolundan geçmelidir. Python/AI için alternatif bir iş kuralı veya doğrudan SQL/DOM yazma yolu açılmamalıdır.
- UI birim, seçim, görünüm ve araç belleğini komut girdisinde somutlaştırmalıdır. Komut hangi nesnenin değişeceğini etkin seçimden tahmin etmemelidir.
- Kullanıcı girdisi kaynak geometriden işlenmeli; ekran tessellation'ı veya ekran görüntüsü ölçünün otoritesi olmamalıdır.
- Proje verisi, kullanıcı tercihi, cihaz ayarı ve oturum durumu farklı sahiplerde kalmalıdır; Python/AI her birinin kapsamını keşfedebilmelidir.
- Yeni bağımlılık ve SDK seçimi; lisans, dağıtım, platform, performans ve bakım incelemesiyle yapılmalıdır. Bu plan herhangi bir kütüphaneyi satın alma veya uygulamaya ekleme kararı vermez.

## 4. CAD-AUD — Kapsam envanteri ve gerçek boşlukların ölçülmesi — P0

- [x] **CAD-AUD-01:** Mevcut envantere CAD kapsam etiketi ekle; GIS, CAD ve ortak altyapı ayrı filtrelenebilsin. — **Bitti (12 Ekim; CADP-01, ADR 0244).**
- [x] **CAD-AUD-02:** Her araç, pencere, özellik satırı, menü ve kısayol için yaptığı ürün işlemini çıkar; yalnız komut ID listesine dayanma. — **Bitti (12 Ekim; CADP-01, ADR 0244).**
- [x] **CAD-AUD-03:** Araç yöntemlerini ve seçeneklerini ayrıca say: örneğin daire merkez–yarıçap/iki nokta/üç nokta/teğet yöntemleri ayrı davranış satırları olsun. — **Bitti (12 Ekim; CADP-01, ADR 0244).**
- [x] **CAD-AUD-04:** Üretilen capability manifestinde `operationId`, command/version, nesne türleri, seçenekler, input/output/plan, birim ve hata kodları bulunsun. — **Bitti (12 Ekim; CADP-01, ADR 0244).**
- [x] **CAD-AUD-05:** Her satırda UI, yazılı komut, Python, AI/MCP, web, desktop, headless, test kanıtı ve eksik nedenini göster. — **Bitti (12 Ekim; CADP-01, ADR 0244).**
- [x] **CAD-AUD-06:** `create/edit` gibi genel komutların son geometri yazma desteği ile yüksek seviyeli semantik işlem desteğini farklı sütunlarda göster. — **Bitti (12 Ekim; CADP-01, ADR 0244).**
- [ ] **CAD-AUD-07:** Katman oluşturma/silme/adlandırma, stil tanımlama, proje ayarı, pafta ve dışa aktarma işlemlerini özellikle uçtan uca denetle. — *Kısmen (12 Ekim; CADP-08, ADR 0248: katman, stil ve proje ayarı komutla; pafta ve dışa aktarma açık).*
- [x] **CAD-AUD-08:** Güncel pafta masaüstü yollarını envanterle karşılaştır; eski `none/pending` etiketlerini kaynak ve davranış kanıtına göre düzelt. — **Bitti (12 Ekim; CADP-02, ADR 0247).**
- [x] **CAD-AUD-09:** Python ve MCP yeni proje oluşturma akışında `drawingUnit` eksikliğini doğrula. Python bağlayıcısında bu alanın `None` verilmesi ve MCP girdisinde olmaması ilk düzeltme listesine girsin. [E12, E14] — **Bitti (12 Ekim; CADP-03, ADR 0245).**
- [ ] **CAD-AUD-10:** Arayüzde mevcut ama doğrudan belge/proje ayarı yazan yolları listele; ortak command yoluna taşıma planı oluştur.
- [x] **CAD-AUD-11:** Kayıtlı olup sessiz no-op çalışan handler, tamamlanmamış seçenek ve ulaşılamayan komut için otomatik denetim ekle. — **Bitti (12 Ekim; CADP-01, ADR 0244).**
- [x] **CAD-AUD-12:** Kaynak, üretilmiş katalog, Python wrapper ve MCP schema arasındaki sapmayı CI'da hata yap. — **Bitti (12 Ekim; CADP-01, ADR 0244).**
- [ ] **CAD-AUD-13:** Gerçek bir mimari ofis çiziminin üretim gününü görev listesine çevir; çizerken yapılan bütün eylemleri manifest satırlarına eşle.
- [ ] **CAD-AUD-14:** Her yeni CAD işi için “hangi mevcut modül genişletilecek?” bilgisini zorunlu kıl; aynı yeteneğin ikinci bir uygulaması açılmasın.

**Kabul:** CAD arayüzündeki her yayımlanmış işin manifest satırı vardır. “Arayüzde var / Python veya AI'da yok” satırları otomatik raporlanır. Ürünün tam kapsam kabulünde açıklamasız eksik satır kalmaz.

## 5. CAD-CMD — Semantik komutlar, transaction ve tekrar oynatma — P0

Bugün `cad.entities.edit` girdisindeki `operation`, büyük ölçüde undo adını belirliyor; asıl değişiklikler `changes` içindeki önceden hesaplanmış geometrilerdir. UI ortak geometri hesabını çağırıp sonucu bu komuta veriyor. Örneğin Python'un “200 mm ötele” yapması için son geometriyi kendisinin üretmesi gerekebilir. Bu, mutation eşdeğerliği sağlar; hedeflenen **işlem eşdeğerliğini** tek başına sağlamaz. [E07]

- [x] **CAD-CMD-01:** Mevcut düşük seviyeli geometri yaması komutlarını koru; üzerlerine kaynak ID ve işlem parametreleri alan tipli semantik komutlar ekle. — **Bitti (13 Ekim; ADR 0246, 0250, 0254, 0255: Ötele, Buda, Uzat, Köşe yuvarla, Pah, Esnet, Kır, Birleştir, Böl, Alan oluştur, Tarama ve Ölçülendirme anlamsal; düşük düzey işlemler kalır).**
- [x] **CAD-CMD-02:** Öteleme girdisi; kaynaklar, mesafe/birim, taraf noktası veya yön, noktadan geçme, iki yana, kaynak silme ve hedef katmanı açıkça taşısın. — **Bitti (12 Ekim; CADP-04, ADR 0246).**
- [x] **CAD-CMD-03:** Buda/uzat girdisi; hedef alt kenar, kesici/sınır nesneleri, seçim bölgesi, izdüşüm/uzantı politikası ve toleransı taşısın. — **Bitti (13 Ekim; CADP-05, ADR 0250).**
- [x] **CAD-CMD-04:** Fillet/chamfer girdisi; hedef alt kenarlar, seçilen taraf, yarıçap veya iki mesafe/açı, trim politikası ve tüm köşeler seçeneğini taşısın. — **Bitti (13 Ekim; CADP-05, ADR 0250).**
- [x] **CAD-CMD-05:** Stretch, join, break, divide, boundary, hatch ve ölçü oluşturma işlemlerinde aynı semantik girdi yaklaşımını tamamla. — **Bitti (13 Ekim; ADR 0254, 0255: Esnet, Birleştir, Kır, Böl, Alan oluştur, Tarama ve Ölçülendirme'nin on türü; Zincir, Baz ve Hızlı ölçü türeten araçlar olarak TODOS.md'de).**
- [x] **CAD-CMD-06:** UI araçları bu semantik komutların plan/execute yolunu kullansın; hesap UI'da ve Python'da farklı şekillerde tekrar edilmesin. — **Bitti (13 Ekim; ADR 0246, 0250, 0254, 0255: bu komutların araçları önizlemeyi planla, yazmayı komutla yapar; kullanım senaryosu `usage-semantic-tools` resimleriyle).**
- [x] **CAD-CMD-07:** İşlem descriptor'larında desteklenen nesne türü/yöntem kombinasyonlarını ve açık red nedenlerini keşfedilebilir yap. — **Bitti (13 Ekim; ADR 0253 §7: kataloğun `capabilities`'i (hedef türler, ret ve uyarı kodları), kodların tek tablosu ve tür denemeleriyle doğruluk testi).**
- [x] **CAD-CMD-08:** `discover → validate → plan → execute → progress/result` akışını veri, proje, dosya ve uzun işlerde tutarlı uygula. — **Bitti (13 Ekim; ADR 0253 §4, §9, §10: belge, proje ayarı ve dosya komutları ile uzun işler aynı akışta; sunucunun `project.*` komutları kendi protokolünde (§11)).**
- [x] **CAD-CMD-09:** Plan; dokunulan nesneler, bağımlı nesneler, eklenen/silinen/değişen sayıları, bbox, kayıp/uyarı ve kaynak revision'larını dönsün. — **Bitti (13 Ekim; ADR 0253 §5: planın özeti eklenen, değişen, silinen ve izleyen nesneleri, kutuyu, uyarıları ve önceki/sonraki damgaları verir).**
- [x] **CAD-CMD-10:** Preview/execute aynı algoritma ve numeric policy sürümünü kullansın; önizlenen sonuç ile commit edilen sonuç değişmesin. — **Bitti (13 Ekim; ADR 0253 §5: plan kimliği ve parmak izi; planla yazma tam o planı yazar, araçların önizlemesi komutun planı).**
- [x] **CAD-CMD-11:** `expectedRevision` yanında gerekli nesne/tanım/asset revision'larını doğrula; xref veya blok değişikliği eski planı geçersiz kılsın. — **Bitti (13 Ekim; ADR 0253 §2: nesne, blok, katman ağacı, ayar, stil ve pafta damgaları; blok değişince plan geçersiz; dış başvurular gelince aynı kuralla).**
- [x] **CAD-CMD-12:** Ağ veya süreç retry'ında mutating çağrının iki kez uygulanmasını önleyen idempotency sözleşmesi kur; yalnız client request ID taşımayı yeterli sayma. — **Bitti (13 Ekim; ADR 0253 §6: istek kimliği parmak iziyle `.kcad` şema 45'in defterinde; aynı istek ikinci kez uygulanmaz, farklı girdiyle reddedilir).**
- [x] **CAD-CMD-13:** İlişkili geometri güncellemeleri ve mimari türevler aynı transaction içinde yazılsın; commit sırasında yeniden doğrulansın. — **Bitti (13 Ekim; ADR 0253 §1, §8: ilişkili ölçü, tarama ve bağlı yazılar aynı adımda izler, bütünlük yazmadan önce denetlenir; mimari türevler gelince aynı adımda).**
- [x] **CAD-CMD-14:** Nesneler, bloklar, stiller, proje ayarları ve paftaların birlikte değiştiği iş için ortak changeset tasarla. — **Bitti (13 Ekim; ADR 0253 §8: `cad.changeset.apply` nesneleri, blokları, katmanları, stilleri, ayarları ve paftaları tek adımda, başvurularla).**
- [x] **CAD-CMD-15:** Mevcut stil değişikliğinde stil tablosunun undo dışında, nesne güncellemelerinin undo içinde olması gibi ayrık davranışları incele; yeni bütüncül undo politikası için ADR ve göç kararı yaz. [E18] — **Bitti (12 Ekim; CADP-08, ADR 0248: ayrık davranış korundu, bütüncül geri alma için göç yolu ADR'de).**
- [x] **CAD-CMD-16:** Uzun hesap snapshot üzerinde çalışsın; UI/document lock'u boyunca kullanıcı girdisi veya network cevabı beklenmesin. — **Bitti (13 Ekim; ADR 0253 §9: iş belgenin okuma kopyasında, ayrı iş parçacığında; belge tutulmaz).**
- [x] **CAD-CMD-17:** İptal; hesap, staged sonuç ve commit aşamalarında tanımlı olsun. Commit tamamlandıktan sonraki “Durdur”, sonucu belirsiz bırakmasın. — **Bitti (13 Ekim; ADR 0253 §9: hesapta, `staged`'de ve kesinleşmede iptalin anlamı; bittikten sonra `already_committed`).**
- [x] **CAD-CMD-18:** Replay kaydı; command/version, normalized input, kaynak revision, birim/UCS, algoritma sürümü, seed, uyarı ve çıktı ID eşlemesini taşısın. — **Bitti (13 Ekim; ADR 0253 §3, §10: `kentos.calls/1` kaydı ve tekrar oynatma dört yoldan; dosya komutlarının baytları karşılaştırılır).**
- [x] **CAD-CMD-19:** Hatalar kararlı code, alan yolu, nesne ID, revision ve retry bilgisi taşısın; Türkçe kullanıcı mesajından hata mantığı çıkarılmasın. — **Bitti (13 Ekim; ADR 0253 §7: `CommandError`'ın kodu, yolu, `uids`'i, revizyonu ve `retryable`'ı; Python'un hata sınıfları koddan).**
- [x] **CAD-CMD-20:** Dışa aktarma gibi yan etkiler için staged dosya + atomik yer değiştirme kullan; belge undo'sunun dış dosyaları geri alıp almadığını açıkça tanımla. — **Bitti (13 Ekim; ADR 0253 §10: DXF ve PDF geçici dosya, geri okuma ve atomik yer değiştirmeyle; belgenin geri alması dosyayı geri almaz, katalog söyler).**

**Kabul:** UI, Python ve MCP aynı kaynak nesne için aynı semantik ötele/buda/fillet komutunu verir; normalleştirilmiş sonuçlar, ilişkiler, uyarılar ve undo davranışı eşittir. Python/AI'ın çizim algoritmasını yeniden yazması gerekmez.

### 5.1 Hedef katalog aileleri ve giriş sözleşmesi

Aşağıdaki adlar **önerilen yeni API taslağıdır**, mevcut uygulamada çalıştığı iddia edilen komutlar değildir. Son ad/sürüm kararı ilgili ADR'de verilmeli; mevcut düşük seviyeli komutlar geriye uyumlulukla korunmalıdır. Tek komutta birden çok operation kullanılması da mümkündür, fakat her alt işlemin şeması ve kapsamı görünür olmalıdır.

| Önerilen aile / örnek komut | Asıl girdi; AI/Python'dan beklenen bilgi |
|---|---|
| `cad.entities.offset` | UUID/subentity, mesafe + birim, side/through, copy/delete source, target layer |
| `cad.entities.trim` / `extend` | Target refs, sınır refs, pick/region, edge policy, tolerance |
| `cad.entities.fillet` / `chamfer` | İki alt kenar, radius veya distances/angle, trim, selected side |
| `cad.dimensions.create` / `reassociate` | Measure refs veya free points, dimension kind, placement, style, scale |
| `cad.hatches.create` / `edit` | Boundary refs veya seed point + kapsam, pattern, islands, association policy |
| `cad.layers.*` | Layer/group ID, ad/yer, flag/property/state değişiklikleri |
| `cad.styles.*` | Style türü/ID, typed definition/override, kullanıcılara uygulama politikası |
| `cad.settings.*` | Scope ve owner, key/schema, requested/effective değer, reset |
| `cad.references.*` | Source asset/path/document-revision, attach/overlay, transform/unit, clip |
| `cad.constraints.*` | Entity/subentity refs, relation/parameter, expression/unit, solve policy |
| `cad.arch.walls.*` / `openings.*` | Source path/style, host wall ref, anchor/position/width, type |
| `cad.arch.spaces.*` / `schedules.*` | Boundary kaynakları, ölçme policy, field/filter/group ve table placement |
| `cad.sheets.*` / `viewports.*` | Book/sheet/item/view ID, paper geometry, model ref, unit/scale, layer overrides |
| `cad.files.*` / `plot.*` | Source snapshot revision, path/asset, format/version, loss policy/page setup |
| `cad.session.*` | Session/drawing ID, seçim/camera/UCS/drafting/tool state ve tipli adım |
| `processing.*` / `jobs.*` | Tool/model + sürüm, parametreler, input snapshot, output ve job handle |

Her mutating istekte uygun `expectedRevision`, idempotency ve actor/scope bilgisi bulunmalıdır. Sonuç entity/ref kimlikleri, revision, uyarı/kayıp ve gerektiğinde artifact/job/undo bilgisi taşır. Bir UI işleminin API girdisi yalnız “uygulanacak final koordinatlar” ise yüksek seviyeli işlem kapsamı hâlâ eksiktir.

## 6. CAD-DATA — Birimler, hassasiyet ve kalıcı CAD modeli — P0/P1

### 6.1 Birimler, koordinatlar ve 2D UCS

- [x] **CAD-DATA-01:** Mevcut iç koordinat/birim politikasını bütün SDK'larda açıklaştır: yerel CAD geometrisi metrede saklanıyor, mm/cm/m giriş/gösterim sınırında çevriliyor. Yeni kolay API birimli miktar alabilir; mevcut metre tabanlı wire değerlerini sessizce farklı yorumlamasın. [E06] — **Bitti (12 Ekim; CADP-03, ADR 0245).**
- [x] **CAD-DATA-02:** Yeni proje komutuna `workspace`, yerel/gerçek koordinat seçimi, `drawingUnit`, plot scale ve font dahil sihirbazdaki bütün ürün parametrelerini ekle. — **Bitti (12 Ekim; CADP-03, ADR 0245).**
- [x] **CAD-DATA-03:** `Document.new`, yerel bağlayıcı ve `drawing.new` MCP şeması aynı birim parametrelerini desteklesin. — **Bitti (12 Ekim; CADP-03, ADR 0245).**
- [x] **CAD-DATA-04:** Birim değiştirirken “yalnız gösterimi değiştir” ile “geometriyi ölçekle” farklı işlemler olsun; açık kaynak/hedef birimle planlansın. — **Bitti (13 Ekim; ADR 0256 §1, §2: `cad.units.change`, `from`/`to`/`geometry: keep | scale`, birim ve çizim tek adım; Proje ayarları sorar).**
- [x] **CAD-DATA-05:** Uzunluk, alan, açı, çizgi kalınlığı, yazı yüksekliği ve kağıt boyları şemada birimleriyle belgelensin. — **Bitti (13 Ekim; ADR 0256 §3: her şema sayısının `x-unit`'i, kataloğun `units` sözlüğü, birimsiz sayıyı düşüren test, Python belgelerinde birim).**
- [x] **CAD-DATA-06:** Toplu ölçü sonuçları birim/CRS/UCS/metot bilgisi taşısın; yalnız sayı döndürmesin. — **Bitti (13 Ekim; ADR 0256 §4: `basis` srid, sistem, tür, `frame`, `method` plane/ellipsoid/ground; `measure_many`, MCP `uids`).**
- [x] **CAD-DATA-07:** İçe alma `INSUNITS`, blok birimi ve hedef proje birimini birlikte değerlendirsin; belirsiz dosyada sessiz ölçekleme yapmasın. — **Bitti (13 Ekim; ADR 0257: tek kural `chosen`/`declared`/`metres`/`project`, `UnitDecision` ve `unit_assumed`, `unit_ignored`, `block_units_differ`; `BLOCK_RECORD` 70; pencerede “Dosyanın birimi”; `file.import.dxf` komutu Python ve MCP'de, `CAD-IO-25`/`26`'nın DXF kısmı).**
- [x] **CAD-DATA-08:** CAD X/Y ile GIS doğu/kuzey gösterimi korunmalı; API kanonik eksenleri, UI etiketleri ve polar açı başlangıcı açıklanmalı. — **Bitti (13 Ekim; ADR 0256 §5: özetin `conventions`'ı, `Document.conventions`, MCP talimatı).**
- [x] **CAD-DATA-09:** Kaydırılabilir/döndürülebilir 2D UCS; world, önceki, nesneye hizalı, iki nokta ve adlı UCS işlemlerini desteklesin. — *13 Ekim; [ADR 0261](../docs/adr/0261-user-coordinate-system.md) §1–§3: projenin `ucs`'i ve `namedUcs`'i (KCAD şema 49), `cad.ucs.set` (dünya, çerçeve, orijin, döndür, iki nokta, nesne, adlı, kaydet, sil), Önceki oturumun geçmişi; Görünüm › Koordinatlar.*
- [x] **CAD-DATA-10:** UCS koordinat girişini, ortho/polar/grid ve ilgili ölçü/tarama yönünü etkilesin; mevcut kaynak geometriyi sessizce taşımasın. — *13 Ekim; ADR 0261 §4: yazılan nokta, Orto, kutupsal izleme, açı kilidi, ızgara, Karelaj, Doğrusal ölçü ve yeni taramanın açısı UCS'te; durum çubuğu ve Koordinat oku UCS'te, eksen simgesi döner; nesneler değişmez.*
- [x] **CAD-DATA-11:** Python/AI point girdisi world/UCS bağlamını açık alsın; görünümün ekran dönüşü UCS dönüşü gibi kullanılmasın. — *13 Ekim; ADR 0261 §5: tel dünyadır; Python `Document.ucs`, `to_world`, `to_ucs`, MCP `drawing.toWorld`, `drawing.toUcs`, özetin `conventions.ucs`'i; kameranın dönüşü yoktur, UCS ondan ayrıdır.*
- [x] **CAD-DATA-12:** Ayak/inç ve mimari kesirli uzunluk gösterimini uluslararası uyumluluk için ayrı genişletme olarak ekle; metrik ilk kabulü geciktirmesin. — *14 Ekim; [ADR 0262](../docs/adr/0262-feet-and-inches.md): yerel projenin `in` ve `ft` birimleri ve `lengthStyle` (mimari, mühendislik, kesirli; KCAD şema 50), yazılan uzunlukta ayak ve inç işaretleri, DXF `$INSUNITS` 1 ve 2; metrik projeler değişmez.*

### 6.2 Kaynak geometri ve referans kimliği

- [x] **CAD-DATA-13:** Entity UUID'lerine ek olarak köşe, segment, eğri aralığı ve blok içi alt nesne için kararlı referans tasarla. — *14 Ekim; ADR 0249 (köşe, kenar ortası, uç, merkez, düğüm) ve [ADR 0263](../docs/adr/0263-places-along-and-inside-blocks.md): kenar ve eğri boyunca yer (`along`), eğri aralığı (`SubRange`, `range_length`), blok içi parça (`path`); KCAD şema 51.*
- [x] **CAD-DATA-14:** Referanslar vertex index'e körü körüne bağlı olmasın; köşe ekle/sil ve kenar bölme sırasında eşleme/invalidasyon politikası tanımlansın. — **Bitti (12 Ekim; CADP-06/07, ADR 0249).**
- [x] **CAD-DATA-15:** Daire/yay/elips kaynakları analitik kalsın; ölçme, snap ve trim çizilen kırık çizgi üzerinden yapılmasın. — *13 Ekim; [ADR 0259](../docs/adr/0259-nurbs-and-curves.md) §1: denetlendi, kenet ve kesme eğrinin kendisinde; uzunluklar OpenCASCADE'le (`curves_occt.py`).*
- [x] **CAD-DATA-16:** Genel NURBS için derece, knot dizisi, kontrol noktaları, ağırlıklar, periodic/closed, fit/tangent ve tolerans alanlarını ekle. — *13 Ekim; [ADR 0259](../docs/adr/0259-nurbs-and-curves.md) §2–§4, §6: `nurbs` türü, `cad.nurbs.create`, NURBS eğrisi aracı, KCAD şema 47, DXF SPLINE tam.*
- [x] **CAD-DATA-17:** Mevcut Catmull–Rom eğrisi yeni NURBS modeline tahminsiz göç etsin veya açık ayrı subtype olarak korunsun; eski çizimin biçimi değişmesin. — *13 Ekim; [ADR 0259](../docs/adr/0259-nurbs-and-curves.md) §5: ayrı tür olarak korunur, baytları aynı.*
- [x] **CAD-DATA-18:** Polyline sabit genişlik ve segment başlangıç/bitiş genişliği ekle; width ile kâğıt lineweight aynı alan olmasın. — *13 Ekim; [ADR 0258](../docs/adr/0258-closed-and-wide-polylines.md) §2: köşe başına başlangıç ve bitiş genişliği (`widths`), çizim uzunluğunda; şerit olarak çizilir; düzenlemelerde kotun kurallarıyla taşınır; KCAD şema 46, DXF 43 ve 40/41.*
- [x] **CAD-DATA-19:** Açık/kapalı polyline ile polygon/region ayrımını koru; CAD polyline'ın kapalı olması onu zorunlu GIS alanı yapmasın. — *13 Ekim; ADR 0258 §1: kapalı çoklu çizgi (`closed`) alan değildir; DXF'in kapalı LWPOLYLINE'ı CAD projesinde kapalı çizgi, CBS'de alan (`closedPolylines`); kapalı eğrinin Patlat'ı web'le birlikte (§5).*
- [x] **CAD-DATA-20:** CAD solid fill, region, wipeout, center mark/centerline, multileader ve ileri nesneler için önce kaynak model ve düzenleme semantiğini tanımla. — *14 Ekim; [ADR 0264](../docs/adr/0264-advanced-cad-objects-source-model.md): dolu alan alan ve dolgusu, bölge alan, wipeout alanın maskesi (çizim sırasıyla), merkez işareti ve eksen çizgisi başvurulu iki yeni açıklama türü, çok kollu kılavuz `leader`'ın kolları, MLINE ve TOLERANCE var olan türlere; uygulaması `CAD-EDIT-23`, `CAD-HATCH-09`, `CAD-ANNO-11`, §11.3.*
- [x] **CAD-DATA-21:** Nesne görünüşünde ByLayer/ByBlock/explicit ayrımını, linetype, linetype scale, transparency, plot ve draw order alanlarını tasarla. — *13 Ekim; [ADR 0260](../docs/adr/0260-object-look.md): görünüşün kaynağı (kendi, katmana göre, bloğa göre; blokta `byLayer`), nesnenin çizgi tipi, ölçeği ve saydamlığı, katmanın `plot`'u, katman içi çizim sırası (`cad.entities.order`, Öne getir, Arkaya gönder); KCAD şema 48, DXF 6/48/440/290, sunucuda `feature.look`.*
- [x] **CAD-DATA-22:** Nesne/alt nesne/ilişki modelinin `.kcad`, tipli sütunlar, WASM, cloud kaynak tanımı ve Python tiplerine birlikte taşınmasını sağla. — *14 Ekim; [ADR 0265](../docs/adr/0265-model-carriage-and-schema-changes.md) §1: taşıma matrisi ve bekçileri; yeni testler bütün `.kcad` örneklerini bulutun kaynağından (`tests/carriage.rs`) ve Python'un tipli katmanından (`test_carriage.py`) geçirir; sunucunun sayı sınırı verinin sayılarını (raster zamanı, nokta sayısı) artık reddetmiyor. WASM web dururken yalnız derlenir.*
- [x] **CAD-DATA-23:** Yeni format şeması; eski okuyucunun açık reddi, yeni okuyucunun eski dosyayı açması, bağımsız doğrulama ve geri dönüş yolu içersin. — *14 Ekim; ADR 0265 §2–§3 ve `docs/specs/kcad-v2.md`'nin kuralı: altı adım, `schemas` testi her şemanın örneğini ve bir öncekinde reddini zorlar (2–51 tamam); geri dönüş en eski şema kuralıyla, numara yeniden kullanılmaz.*
- [x] **CAD-DATA-24:** Sayısal politika; float64 kaynak, kontrollü gösterim yuvarlaması, toleranslı geometri kararı ve kesin ondalık kullanıcı verisi arasındaki sınırı belgele. — **Bitti (13 Ekim; [docs/specs/numeric-policy.md](../docs/specs/numeric-policy.md): dört tür sayı, birim çevirisi, gösterim, toleranslar tablosu, kesin veri ve politikası, doğrulama).**
- [x] **CAD-DATA-25:** Küçük mimari detay ve büyük gerçek koordinat birlikte sınansın; GPU origin/rebase çözümü kaynak koordinatları değiştirmesin. — **Bitti (13 Ekim): TM33'te 1 mm'lik ayrıntının ölçüsü, Buda'sı, `.kcad` ve DXF gidiş dönüşü bit bit (başsız sunucunun testi); çizimde paketleme kaynağı yalnız okur, sapma konumun float32 adımı kadar (`styled_precision`); çapadan uzakta milimetre altı çizim `REN-16`'da.**

**Kabul:** mm ile başlayan 4000 × 3000 mm oda, kaynak hesabında 12 m² verir; UI/Python/AI aynı birimi gösterir. Birim veya UCS değişikliği eski geometriyi beklenmedik biçimde ölçeklemez. NURBS ve genişlikli polyline kaydedilip yeniden açıldığında kaynak parametrelerini korur.

## 7. CAD-INPUT — Hassas çizim, seçim ve komut deneyimi — P1

AutoCAD'in hassas çizim referansı; mutlak/göreli/kutupsal giriş, doğrudan mesafe, açı kilidi, polar izleme, grid ve object snap iş akışlarıdır. KentOS'taki mevcut dinamik giriş ve snap altyapısı bunlara göre **tamamlanıp sınanmalıdır**. [Autodesk hassas çizim](https://help.autodesk.com/cloudhelp/2026/ENU/AutoCAD-OnBoarding/files/ACD_FOUNDATIONS_MAIN6.html)

- [x] **CAD-INPUT-01:** Mutlak/göreli koordinat, mesafe<açı, doğrudan mesafe, açı kilidi, birimli giriş ve hesap ifadesini bütün çizim/düzenleme araçlarında tutarlı yap. — *14 Ekim; [ADR 0266](../docs/adr/0266-typed-units-and-sums.md): metrik birimli uzunluk (`2.5m`, `250cm`) ve parantezli toplam (`(12.5+3*2)`) ortak dilbilgisinde; araçların uzunluk alanları tek yardımcıdan (`plain_value`, `parse_length_pair`), ayak-inç da.*
- [x] **CAD-INPUT-02:** İlk yazılan karakter, Enter/Space/Esc, Tab ile alan değiştirme, negatif değer ve Türkçe klavye davranışlarını ortak etkileşim fixture'larıyla doğrula. — *14 Ekim: ortak izler `polygon-keys` (Esc, Backspace, Tab, Boşluk, Enter, odak), `polygon-signs` (eksi ve artı ile başlayan değer), `command-name` (harfin komut satırını açması) ve yeni `value-entry` (`(` ile açılış, toplam, birim harfleri, Tab ile birimli kilit, kapanmayan parantez; ADR 0266); masaüstü her izi ABD ve Türkçe Q klavyesiyle ve HiDPI'da oynatır (`traces::keyboard`'a `(`, `)`, `*` eklendi).*
- [x] **CAD-INPUT-03:** Dinamik girişte mesafe/açı/X/Y alanları ayrı kilitlenebilsin; kilidin hangi eksen/UCS'e göre çalıştığı görünür olsun. — *14 Ekim; [ADR 0267](../docs/adr/0267-coordinate-locks-and-lock-frame.md): X ve Y kilitleri (`draft.lock.east`, `.north`; Tab öbür koordinata), uzunlukla birlikte çemberle doğrunun kesişimi, doğrultu kilidiyle birlikte tutulmaz; UCS'te kilitler UCS'in eksenlerinde, çipler “(UCS)” der, değer alanının Enter'ı önizlemeyle aynı kuralla; ortak iz `lock-coordinates`, bağımsız başvuru `lock_coordinate_cases.py`.*
- [x] **CAD-INPUT-04:** Ortho/polar tracking ve geçici override önceliğini tanımla; geçici kapatma aracın diğer ayarlarını kaybettirmesin. — *14 Ekim; [ADR 0268](../docs/adr/0268-drafting-aid-priority-and-overrides.md): tek öncelik kuralı (koordinat kilidi, doğrultu kilidi, Dik açı, kenet ve izleme, Orto, kutupsal; `Aid`, `ruling_aid`), Orto ve kutupsal birbirini kapatmaz, Orto önce gelir; Shift ve F8/F10 önizlemeyi hemen yeniler, durum çubuğunda Orto'nun geçici hâli ve kutupsalın askısı; ortak iz `aid-priority` (`hold`, `aid`).*
- [x] **CAD-INPUT-05:** Endpoint/midpoint/center/quadrant/intersection/tangent/perpendicular/nearest/insertion/geometric center gibi snap türlerinin mevcut kapsamını tek tek doğrula. — *14 Ekim; [ADR 0269](../docs/adr/0269-snap-kind-coverage.md): kapsam matrisi bağımsız başvuruyla (`snap_coverage_cases.py`, 42 durum); bulunan eksikler kapandı: yeni Ekleme noktası türü (yazı, blok, tablo; `snap.insertion`, `draft.snap.insertion`), yayın ve yaylı kenarın çeyrekleri, tek seferlik Merkez'de dar yayın merkezi eğrinin üstünden.*
- [x] **CAD-INPUT-06:** Extension, parallel, iki noktanın ortası ve tracking acquire gibi ek snap yöntemlerini mevcut destek üzerinden tamamla; eksik yöntemleri semantik API'ye ekle. — *14 Ekim; [ADR 0270](../docs/adr/0270-point-constructions.md): salt okunur `cad.points.construct` (iki nokta ortası, uzantı, paralel, izleme, kutupsal, hat üzerinde; noktalar koordinat ya da nesnenin yeri), Python ve MCP'de; Geçici izleme noktası (`draft.trackPoint`, Nesne izleme kapalıyken de), tek seferlik menüde Referans noktası ve İki nokta ortası; ortak durumlar `point_construct_cases.py` (30), iz `track-point`.*
- [x] **CAD-INPUT-07:** Kalabalık çizimde snap adayına cycling, tür/geometri ipucu, öncelik ve piksel aperture davranışını deterministik yap. — *14 Ekim; [ADR 0271](../docs/adr/0271-snap-cycling-and-determinism.md): çekirdeğin `snap_candidates`'i (ağırlıklı uzaklık, eşitlikte çizimin sırası, aynı nokta bir kez; ilki kenetin seçtiği), Tab ve Shift+Tab ile dolaşma, işarette nesnenin türü ve “2/3 (Tab)”; açıklık mantıksal piksel; bağımsız başvuru `candidates`, iz `snap-cycle`.*
- [x] **CAD-INPUT-08:** Snap sonucu; nesne/alt nesne referansı, kaynak noktası, türü ve revision bilgisiyle okunabilsin; ilişkili ölçü oluşturmak için referans kullanılabilsin. — *14 Ekim; [ADR 0272](../docs/adr/0272-snap-references-as-a-command.md): salt okunur `cad.snap.find` (tür, nokta, kimlik, yer, blok yolu, sürüm; `all` ile adaylar), cevabı `cad.dimensions.create`'in `place` noktasına olduğu gibi verilir; Ekleme noktası `node` yeri; Koordinat oku kenetin başvurusunu yazar; ortak durumlar `snap_find_cases.py` (21), Python testi bağlamayı yapar.*
- [x] **CAD-INPUT-09:** Pointer commit anında yeniden snap hesaplama kuralını koru; büyük çizim ve hızlı fare hareketinde eski aday commit edilmesin. — *14 Ekim; [ADR 0273](../docs/adr/0273-snap-at-commit.md): basma ve bırakma kenedi kendi yerinde ve o anki çizimde alır; Tab'ın adayları çizimin sürümüne bağlı (değişince biter); testler hareketsiz basma, kaldırılan nesne, değişen çizimde Tab ve 20 000 nesneli çizim.*
- [x] **CAD-INPUT-10:** Blok, xref ve mimari nesnelerin içeriklerine snap; görünürlük, kilit ve nested transform kurallarına uysun. — *14 Ekim; [ADR 0274](../docs/adr/0274-snap-inside-blocks.md): blok parçaları iç içe, dönük, ölçekli ve aynalı yerleştirmede kendi nesneleri gibi (bağımsız başvuru `snap_block_cases.py`, 30 durum), kapalı parçanın ağırlık merkezi eklendi, ekleme noktası en dıştakinin, karar yerleştirmenin katmanının (gizli değil, kilitli olur, kendi türleri), `cad.snap.find` parçanın yolu ve yeriyle; xref (`CAD-REF-13`) ve mimari nesneler (`CAD-ARCH-02`) henüz yok, gelince aynı kurala uyar.*
- [x] **CAD-INPUT-11:** Selection window/crossing, fence, polygon, circle, önceki seçim, benzerini seç ve filtre davranışlarını aynı açık sorgu modeline bağla. — *14 Ekim; [ADR 0275](../docs/adr/0275-selection-as-a-query.md): salt okunur `cad.entities.select` (on bir bölge, türler ve katmanlar, dışarıda kalanların sayısı, sürüm); seçim araçları, Tümünü seç, Ters çevir ve Önceki seçim aynı sorguyla (Ters çevir ve Önceki seçim artık katman süzgecine ve zaman penceresine de uyar); ortak durumlar `entities_select_cases.py` (39), Python `test_select.py`, MCP'de salt okunur.*
- [x] **CAD-INPUT-12:** Üst üste nesnede seçim cycling ve subentity selection; UUID/alt nesne kimliğine dönüştürülebilsin. — *14 Ekim; [ADR 0276](../docs/adr/0276-sub-object-selection.md): alt nesne kuralı (`place_near`, `place_near_in`; başvuru `subref_cases.py`), salt okunur `cad.entities.pick` (23 ortak durum), seçim aracında Ctrl/⌘ + tık (halka, günlük, çipte her adayın alt nesnesi), kısayol penceresinde Ctrl + tık ve Çizerken grubu.*
- [x] **CAD-INPUT-13:** Sürükleyerek seçim ile tutamaç hareketi ve kamera pan davranışının çakışmalarını gerçek fare izleriyle test et. — *14 Ekim; [ADR 0277](../docs/adr/0277-pointer-conflicts.md): izlere ham fare adımı (`mouse`, çizim alanının kendi işleyicisinden), `pointer-conflicts` izi; bulunan hata düzeldi: kutunun ilk köşesi kaydırma ve yakınlaştırmada çizimde kalır, pencere kesişene dönmez (seçim aracı, değiştirme araçlarının seçmesi, Özellik kopyala, etiket araçları, pencerelerin nesne seçmesi).*
- [x] **CAD-INPUT-14:** Selection filter, kilitli/gizli katman ve xref filtreleri bütün araçlarda aynı kurala uysun; atlananlar sayısıyla raporlansın. — *14 Ekim; [ADR 0278](../docs/adr/0278-one-rule-for-locked-hidden-and-filter.md): seçim süzgeci kutuyla ve kenarla seçen her araçta, kilitliler seçilir ama küme komutları (`set`, `order` dahil) onları sayıyla atlar, ilişki kuran düzenlemeler bütün ya da hiç, tıklamada neden söylenir; iz `locked-and-filter`. Xref henüz yok (`CAD-REF-*`), gelince aynı kurala uyar.*
- [x] **CAD-INPUT-15:** Tutamaç menüleri nesneye göre move/stretch/rotate/scale/mirror, vertex add/remove ve ilgili semantik işlemleri sunsun. — *14 Ekim; [ADR 0279](../docs/adr/0279-grip-menus-and-the-vertex-command.md): `cad.entities.vertex` (köşe sil, köşe ekle: kenarın ortası ya da üzerindeki yer, düz kenar, yaya dönüştür; topoloji komşularıyla), tutamaç menüsü her tutamakta Esnet, Taşı, Döndür, Ölçekle, Aynala ile; tutamaç menüsünün köşe işleri ve Köşe ekle/sil aracı komutla yazar.*
- [x] **CAD-INPUT-16:** Birden çok tutamacı etkinleştirme, referansla ölçek/döndürme, base point değiştirme ve copy modu desteklensin. — *14 Ekim; [ADR 0280](../docs/adr/0280-grip-modes-and-several-hot-grips.md): Shift ile birden çok sıcak tutamaç, tutamaç kipleri (E Esnet, T Taşı, D Döndür, L Ölçekle, A Aynala), B taban noktası, R referans, K kopya; Esnet `grip`, öbürleri `cad.entities.transform` ile.*
- [x] **CAD-INPUT-17:** Araç seçenekleri fare, klavye ve yazılı komut yoluyla erişilebilir olsun; belirsiz kısaltmalar ve alias çakışmaları yakalansın. — *14 Ekim; [ADR 0281](../docs/adr/0281-typed-names-and-options.md): seçenek adıyla ya da adının başıyla (belirsizse adaylar söylenir), komut satırında bağlama göre tam ad, sonra tekil kısaltma, belirsiz ad ve kısaltma çalıştırılmaz; takma ad ve seçenek harfi çakışmaları testte.*
- [x] **CAD-INPUT-18:** İngilizce AutoCAD alışkanlıkları için desteklenen alias profili sun; KentOS'un Türkçe adları ve klavye sözleşmesi korunmalı. — *14 Ekim; [ADR 0282](../docs/adr/0282-autocad-alias-profile.md): Uygulama ayarları › Görünüm › Komut satırı'nda kapalı başlayan “AutoCAD takma adları” (`drafting.autocadAliases`); profil `catalog/v1/aliases-autocad.json` (57 acad.pgp adı, yöntemle `JOG`), KentOS'un adı, yöntem adı ya da tek tuş kısayolu olan ad dışarıda kalır (B, CP, D, I, P, POLYGON, S, T); komut satırının listesinde ve F1'de.*
- [x] **CAD-INPUT-19:** Şeffaf pan/zoom/ölçme ve nokta hesaplayıcıdan dönünce aktif aracın parametreleri ve preview'ı korunsun. — *14 Ekim; [ADR 0283](../docs/adr/0283-transparent-commands-keep-the-tool.md): Kaydır, Pencere yakınlaştır, Mesafe ölç, Açı ölç ve Koordinat oku çalışan komutun ya da sıcak tutamacın üstünde çalışır (`Session::start_over`); komut noktaları, seçenekleri, istemi, kilitleri ve referansıyla bekler, Esc ya da Enter ile kaldığı yerden sürer; askıdakiler yığın (hesaplayıcı şeffaf ölçmenin üstünde de); görünüm değişince ya da komut dönünce önizleme imlecin altında (`follow_view`); iz `transparent.json`.*
- [x] **CAD-INPUT-20:** Komut geçmişi; kullanıcıya okunabilir işlem, seçenekler, hata ve uyarıyı taşısın; istenirse karşılık gelen Python kodu ve replay girdisi alınabilsin. — *14 Ekim; [ADR 0284](../docs/adr/0284-command-history-calls.md): Komut geçmişi çizimin kaydını (`kentos.calls/1`) okur: komutun Türkçe başlığı, düz Türkçe girdisi, yazdığı, uyarıları kodlarıyla, aynı geri alma adımı birlikte, geri alınan soluk; yazmadan dönen çağrı (`Document::note_refusal`) hata koduyla; her çağrının Python kodu (`python_call`) gösterilir ve kopyalanır, tekrar oynatma girdisi (`kentos.replay/1`) kopyalanır ya da kaydedilir.*
- [x] **CAD-INPUT-21:** Zoom extent/window/previous/selection, named view, döndürülmüş görünüm ve birden çok model viewport iş akışlarını tamamla. — *14 Ekim; [ADR 0285](../docs/adr/0285-views-twist-and-split.md): adlı görünümler projede (`.kcad` şema 52, `cad.views.edit`, Görünümler penceresi, Python, MCP), Görünümü döndür (iki nokta, açı, UCS'e göre, Kuzey yukarı; kamera dönüşü çizim hattında, dönmüş görünümde seçim kutusu), Görünümü böl (yan yana iki model görünümü); 2×2 düzen ve görünüme göre katman görünürlüğü açık.*
- [x] **CAD-INPUT-22:** Büyük seçimde property paneli “çeşitli” değerleri, toplu değişikliği ve kilitli nesne sonuçlarını gecikmeden ve açık biçimde sunsun. — *14 Ekim; [ADR 0286](../docs/adr/0286-many-selected-properties.md): karışık seçimde kilitsizler düzenlenir, kilitliler atlanır ve sayılır (ilk satırda not), Katman ortak ya da Çeşitli, Ortak öznitelikler (değer ya da “Çeşitli (k/n nesnede)”, bütün seçime tek adımda); 100 000 nesnede panel 48 ms.*

**Kabul:** Aynı örnek kat planı mouse/klavye, komut satırı ve koordinat/ID parametreleriyle üretilebilir. Yanlış snap ve odak kaybından kaynaklı koordinat hatası oluşmaz; iptal preview nesnelerini belgede bırakmaz.

## 8. CAD-EDIT — Çizim ve düzenleme araçlarının profesyonel tamamlanması — P1

- [x] **CAD-EDIT-01:** Her primitive'in bütün yöntemlerini envanterle karşılaştır; eksik yöntem/yöntem seçeneği için mevcut araç ailesini genişlet. — *14 Ekim; [ADR 0287](../docs/adr/0287-drawing-tool-methods.md): AutoCAD'le karşılaştırma tablosu (var, eklendi, yapılmadı ve nedeni); şeritte Yay'ın 11 yöntemi, Daire'nin Merkez, çap'ı, Elips ve Revizyon bulutu yöntem düğmeleri, Düzgün çokgen'in çember içinde, dışında ve kenardan yöntemleri; yeni seçenekler: Dikdörtgen'in Alan'ı (A) ve Boyutlarla, Alanla yöntemleri, Yardımcı çizgi'nin Paralel'i (P, Noktadan), Revizyon bulutu'nun Nesne'si (N); yöntemler `catalog/v1/additions.json`'un `methods`'unda, 24 yeni ikonla; hesap çekirdekte (`rect_side_for_area`, `xline_offset`, `cloud_around`), yazma araçların kendi komutlarıyla.*
- [x] **CAD-EDIT-02:** Line/polyline üzerinde arc–line geçişi, tangent devam, close/undo, width, eğri düzenleme ve segment conversion desteğini tamamla. — *14 Ekim; [ADR 0288](../docs/adr/0288-polyline-arcs-widths-and-segment-edits.md): AutoCAD PLINE ve PEDIT'le karşılaştırma tablosu; Çoklu çizgi'de yay ikilileri (Açı+Merkez/Yarıçap, Merkez+Açı/Uzunluk, Yarıçap+Açı), yay kipinde Genişlik (E) ve teğet yayla Kapat (K), kenar başına başlangıç ve bitiş genişliği; `cad.entities.vertex`'in Yaya dönüştür'üne `through` ve `radius`, yeni Kenar genişliği (`width`, `widths`; `cad.entities.edit`'in `EdgeWidth`'i), `invalid_arc_edge`, `not_a_polyline`; tutamaç menüsünde Noktadan ya da yarıçapla yay… ve Kenar genişliği… (Kenar düzenle aracı); 84 ortak durum; fit ve spline CAD-EDIT-17'de.*
- [x] **CAD-EDIT-03:** Daire/yay/elips araçlarında iki/üç nokta, merkez, açı/uzunluk, tangent ve radius/diameter yöntemlerini doğrula. — *14 Ekim; [ADR 0289](../docs/adr/0289-curve-methods-verified.md): yöntem ve doğrulama tablosu; salt okunur `cad.curves.construct` (2N, 3N, TTY, TTT; on bir yay yöntemi ve Devam; elips ve eliptik yay), yedi yeni ret kodu; araçlar eğriyi komutun hesabından alır; düzeltilenler: paralel iki doğruya TTY, ölçekten bağımsız aynı doğru, başlangıç doğrultusundaki bitiş ve tam tur açı reddi, saat yönünde verilen yaydan Devam son noktadan, kapalı çoklu çizginin ucu; bağımsız başvuru `curve_method_cases.py` (81 komut, 37 araç durumu, `within`); Python ve MCP.*
- [x] **CAD-EDIT-04:** Rectangle üzerinde boyut/alan/rotation/fillet/chamfer seçenekleri; polygon üzerinde inscribed/circumscribed ve kenar yöntemlerini tamamla. — *14 Ekim; [ADR 0290](../docs/adr/0290-rectangle-and-regular-polygon-commands.md): anlamsal `cad.rectangle.create` (iki köşe, boyut, alan, kenar; dönüş; keskin, yuvarlak, pah) ve `cad.polygon.regular` (çember içinde, dışında, kenardan); Dikdörtgen, Döndürülmüş dikdörtgen ve Düzgün çokgen önizlemeyi komutun hesabından alır ve komutla yazar (Çakışma ve Topoloji alanı değiştirirse alanın yolu); 62 ortak durum, Python, MCP, manifestte semantic.*
- [x] **CAD-EDIT-05:** Move/copy/rotate/scale/mirror; numeric input, reference, multiple copy ve başlangıç noktası kurallarıyla bütün nesne türlerinde çalışsın. — *14 Ekim; [ADR 0291](../docs/adr/0291-move-copy-rotate-scale-mirror-rules.md): AutoCAD MOVE/COPY/ROTATE/SCALE/MIRROR karşılaştırma tablosu; Yer değiştirme (Y ya da temel noktada Enter, ikinci Enter son yer değiştirme), Kopyala'nın Kip'i (Tek/Çoklu) ve Dizi'si (adet, Sığdır; `cad.entities.array`'in `line` yerleşimi), Döndür'ün açısı projenin düzeniyle (CAD derece saat yönü tersine, CBS grad saat yönünde, Referans'ta doğrultu/semt) ve Noktalar, Ölçekle'nin Noktalar'ı; her türün dört dönüşümü `transform_kinds_cases.py`'den (31 durum), dolu taramanın deseni artık dönmüyor; yazının aynası CAD-EDIT-06'da.*
- [x] **CAD-EDIT-06:** Mirror text/attribute politikasını ayrı seçenek yap; kapı yönü, metin okunurluğu ve blok attribute değerleri rastlantısal değişmesin. — *14 Ekim; [ADR 0292](../docs/adr/0292-mirror-text-policy.md): `cad.entities.transform`'un aynasına `text` (readable varsayılan, mirrored; AutoCAD'in MIRRTEXT'i), çekirdekte `TextPolicy`/`transform_shape_with`; okunur yazı aynadaki yerinde, baş aşağı olmadan, hizası o yeri dolduracak biçimde (yatay eksende baş aşağı çıkan yazı, kılavuz notu, tablo ve blok içindeki öznitelik yazısı düzeldi); bloğun geometrisi aynalanır (kapının açılışı döner), yazıları okunur, öznitelik değerleri değişmez; Aynala'da ve tutamaç kipinde Yazılar (Y); başvuru `affine_reference.text_frame` ve `transform_kinds_cases.py`; harfleri ters yazan yazı yapılmadı.*
- [x] **CAD-EDIT-07:** Offset; line/arc/polyline/ellipse/NURBS/region için destek ve yaklaşık sonuç politikası belli olsun; yarıçap çökmesi ve self-intersection raporlansın. — *14 Ekim; [ADR 0293](../docs/adr/0293-offset-regions-curves-and-reports.md): kesinlik tablosu (çizgi, yol, alan, daire, yay kesin; elips, fit-point eğrisi ve NURBS yaklaşık, `approximate_offset` milimetreyle); delikli ve çok parçalı alanın delikleri öbür yana (önceden sessizce düşüyordu); yarıçap ve alan genişliği çökmesi `radius_collapse` (tersine dönen halka artık yazılmıyor), kendini kesen kopya `self_intersecting` uyarısıyla; çekirdekte `offset_noted`, web'in kayıtları değişmedi; 36 ortak durum.*
- [x] **CAD-EDIT-08:** Trim/extend; seçili sınırlarla ve hızlı tüm-sınırlar moduyla, fence/crossing ve edge extension seçenekleriyle çalışsın. — *14 Ekim; [ADR 0294](../docs/adr/0294-trim-extend-modes.md): AutoCAD TRIM/EXTEND karşılaştırması; komutta Çit (`fence`), Kesişen (`crossing`), Uzantılar (`edgeMode`: çizgi doğrusuyla, yay çemberiyle, açık çoklu çizginin uç kenarları), hızlı kipte kesilmeyen nesnenin silinmesi (`eraseUncut`), kalan parça bütünüyle gider; araçta Kesişen (K, boş yere tıklama), Uzantılar (U), Sil (R); 52 ortak durum (`trim_modes_cases.py`), iz `trim-modes.json`; tutamaç kipinin Döndür'ü projenin açı düzeniyle.*
- [x] **CAD-EDIT-09:** Fillet/chamfer; iki nesne, polyline, all corners, radius 0, no-trim ve geçersiz köşe durumlarını kapsasın. — *14 Ekim; [ADR 0295](../docs/adr/0295-fillet-chamfer-two-objects-and-lines.md): AutoCAD FILLET/CHAMFER karşılaştırması; iki nesnenin köşesi çizgi ya da açık çoklu çizginin ilk veya son düz kenarı (yolun serbest ucu yayın ya da pahın ucuna gelir), aynı yolun iki ucu yolu kapatır (kapanan kenar yay ya da pah, sıfırda son köşe düşer); Tüm köşeleri yuvarla ve Tüm köşelere pah uçları buluşan ayrı çizgilerde de (sahibin bildirimi: çizgilerle çalışmıyordu), sığmayan köşe çifti atlanır; iç ve yay kenarı, kapalı yol, iç köşeye düşen köşe, aynı uç retleri; Kırp kapalıyken yayın açıları [0, 2π); 52 ortak durum (`corner_sides_cases.py`); yay ve daireyle köşe kalan.*
- [x] **CAD-EDIT-10:** Break at point / iki nokta, join toleransı, polyline birleştirme ve direction reverse işlemleri metadata ve ilişki politikasını korusun. — *14 Ekim; [ADR 0296](../docs/adr/0296-break-join-reverse-carry-policy.md): AutoCAD BREAK/JOIN/REVERSE karşılaştırması ve taşınanlar tablosu; Kır (tek ve iki nokta) ve Birleştir parçalara katmanı, görünüşü, öznitelikleri, kotları, ölçüleri, genişlikleri ve ekleri taşır, ilişkili ölçüler, bağlı yazılar ve taramalar izler ya da nedeniyle kopar; Birleştir'de tolerans, ayrışan öznitelikler ve birleşmeyenler uyarıyla; Yönü çevir anlamsal `cad.entities.reverse` (kotlar, ölçüler, genişlikler tersine); ortak durumlar `break_join_policy_cases.py`; ajan başlattı, ben bitirdim.*
- [x] **CAD-EDIT-11:** Stretch seçim penceresinin yalnız kapsadığı köşeleri düzenlesin; bağlı ölçü, açıklık, tarama ve dizi etkileri önizlemede görülsün. — *15 Ekim; [ADR 0297](../docs/adr/0297-stretch-vertices-and-followers.md): AutoCAD STRETCH karşılaştırması; yayın köşeleri uçlarıdır (yalnız ortası ya da merkezi pencerede olan yay değişmez, bir ucu taşınan yay kiriş yüksekliğini ve yanını korur); bağlı olduğu nesneyi izleyen nesne kendisi esnemez (ilişkili tarama, bağlı yazı, ölçünün bağlı noktası; listelenirse `follows_object`); plan ve sonuç izleyenleri verir (`dependents`, belgenin adım sonu kuralı bir kopyada: `Document::followers`, aracın tuttuğu kopyayla `followers_with`); Esnet önizlemesinde izleyenler noktalı; 26 ortak durum (`stretch_policy_cases.py`); ilişkili dizi ve mimari açıklık henüz yok (kalanlar).*
- [x] **CAD-EDIT-12:** Lengthen; delta, yüzde, total ve dynamic yöntemlerini analitik path üzerinde desteklesin. — *15 Ekim; [ADR 0298](../docs/adr/0298-lengthen-command.md): AutoCAD LENGTHEN karşılaştırması; anlamsal `cad.entities.lengthen` (uç, Fark, Yüzde, Toplam, Dinamik; yayda açıyla), çekirdeğin `ops::lengthen::lengthen`'i çizgi, yay, çoklu çizginin yay kenarı ve elips yayında (kendi elipsinde), uzayan ucun kotu ve ölçüsü uç kenar boyunca sürer, kısaltma kesilen kenarınkini uzunlukla alır; araç komutun planıyla önizler ve yazar; 38 ortak durum bağımsız başvurudan (mpmath), Python ve MCP; 100 000 nesnede önizleme en çok 0,37 ms. Eğri ve NURBS sonraya.*
- [x] **CAD-EDIT-13:** Divide/measure boyunca nokta veya blok yerleştirme; aralık/sayı, başlangıç, hizalama ve son nokta politikasıyla açık komut olsun. — *15 Ekim; [ADR 0299](../docs/adr/0299-divide-measure-blocks.md): AutoCAD DIVIDE/MEASURE karşılaştırması; `cad.entities.divide`'a `block` (adıyla, hizalı, ölçek, dönüş), `start` (aralığın ilk uzaklığı) ve `ends` (açık nesnenin uçları); çekirdeğin `division_places`'i yerleri ve doğrultuyu verir, elips yayı gerçek yay uzunluğuyla; Böl aracında Blok, Nokta, Hizala, Uçlar, İlk ve plandan önizleme; 27 ortak durum bağımsız başvurudan; 10 000 hizalı bloğun planı 7,7 ms.*
- [x] **CAD-EDIT-14:** İlişkili dikdörtgen/kutupsal/yol dizisi modeli ekle; kaynak, yerleşim ve instance override'ları kalıcı kalsın. — *15 Ekim; [ADR 0300](../docs/adr/0300-associative-arrays.md): belgenin `arrays`'i (kaynak blok, ilk yer, ızgara, kutupsal, doğru ya da yol yerleşimi, yer başına öğe: elle silinen boş, elle değişen `kept`; `.kcad` şema 53); `cad.entities.array`'in `associative`'i nesneleri “Dizi n” bloğu yapar ve yerleştirmelerini koyar; adımın sonunda silinen öğe boşalır, değişen korunur, yol dizisi yolunu izler; veritabanı projesinde meta veri (göç 0019); 21 ortak durum bağımsız başvurudan; 10 000 öğe 26 ms, 2 000 öğenin yolu izlemesi 1 ms.*
- [x] **CAD-EDIT-15:** Dizi satır/sütun/sayı/aralık/yol değişikliği bütün elemanları yenilesin; tek eleman değiştirme/silme ve reset desteklensin. — *15 Ekim; [ADR 0301](../docs/adr/0301-array-edit.md): AutoCAD ARRAYEDIT karşılaştırması; anlamsal `cad.arrays.edit` (yerleşim: öğeler numaralarını, ızgarada satır ve sütunlarını korur, yeni yerlere yerleştirme eklenir, kalkanlar silinir, silinen ve korunan kalır; ilk yer; Değiştir; Sil; Sıfırla); Diziyi düzenle aracı öğeye tıklanınca dizinin seçenekleriyle, Taban'da plandan önizleme; 33 ortak durum bağımsız başvurudan; 100 × 100 ızgarada en uzunu 14 ms.*
- [x] **CAD-EDIT-16:** Mevcut bağımsız kopya dizileri korunmalı; “ilişkili” açık seçenek olsun ve explode ilişkili diziyi bağımsız nesnelere dönüştürsün. — *15 Ekim; [ADR 0302](../docs/adr/0302-associative-option-and-explode.md): Dizi, Kutupsal dizi ve Yol boyunca dizi'de İlişkili (İ), kapalı başlar, Kopyala'nın Dizi'si bağımsız; anlamsal Patlat `cad.entities.explode` (parçalar çekirdekten, bloğun nesneleri kendi katman ve görünüşleriyle), ilişkili dizinin öğesi bütün diziyi patlatır ve dizi aynı adımda kalkar; 23 ortak durum bağımsız başvurudan; 30 000 parça 90 ms.*
- [x] **CAD-EDIT-17:** PEDIT/spline edit eşdeğeri; control/fit point, knot, tangent, close/open, curve-to-polyline toleransını kapsasın. — *15 Ekim; [ADR 0303](../docs/adr/0303-curve-edit.md): AutoCAD PEDIT ve SPLINEDIT karşılaştırması; anlamsal `cad.curves.edit`: çoklu çizgide Kapat, Aç, Düzleştir, Eğriye çevir (geçen ya da denetim noktalı), geçiş noktalı eğride nokta ekle, taşı, sil ve uç teğetleri (Piegl ve Tiller 9.2.2, çekirdeğin `through_tangents`'ı), her eğride denetim noktası taşı, ağırlık, sil, düğüm ekle (Boehm), kapat ve aç, toleransla çoklu çizgiye çevir; yedi yeni kod; Eğriyi düzenle aracı komutun planıyla önizler ve yazar; 37 ortak durum IEEE ikiziyle bit bit, mpmath denetimiyle; Python ve MCP; 2 000 noktalı eğride önizleme 0,19 ms.*
- [x] **CAD-EDIT-18:** Match properties; aktarılacak katman, renk, linetype, kalınlık, stil ve annotation özelliklerini seçebilsin. — *15 Ekim; [ADR 0304](../docs/adr/0304-match-properties.md): AutoCAD MATCHPROP karşılaştırması; anlamsal `cad.entities.match` (kaynak, hedefler, seçilen özellikler: katman, renk, çizgi tipi ve ölçeği, kalınlık yalnız çizgiyle çizilenler arasında, saydamlık, sembol; aynı türe yazının yüzü, yüksekliği ve genişliği, ölçünün görünüşü, tarama deseni, kılavuzun oku), katmana taşıma `cad.entities.set`'in kurallarıyla, kilitli hedef uyarıyla, `no_properties`; Özellik kopyala komutla yazar, istemde on anahtar oturumda kalır; 32 ortak durum bağımsız başvurudan; Python ve MCP; 10 000 parselin yazılması 128 ms.*
- [x] **CAD-EDIT-19:** Overkill/cleanup; yinelenen, üst üste, sıfır uzunluk ve birleşebilir geometriyi öneri/plan olarak sunsun; toleransla düzeltme kaybı raporlansın. — *15 Ekim; [ADR 0305](../docs/adr/0305-cleanup-overkill.md): AutoCAD OVERKILL karşılaştırması; anlamsal `cad.entities.cleanup` (kapsam, tolerans, yok sayılan özellikler; boş nesneler, türe göre tolerans içinde yinelenenler, verisi farklı olan yinelenen sayılmaz; örtüşen ve istenirse uç uca eş doğrultulu çizgiler ilkine katılır; yolun tekrarlanan ve doğrudaki köşeleri, kotlu ve genişliği değişen köşe kalır), rapor sayılar ve kayıpla (gidenin kalandan en uzak noktası); çekirdek `ops::overkill`; Çizimi temizle komutun planıyla önizler, beş seçenek oturumda; 37 ortak durum bağımsız başvurudan; Python ve MCP; 100 000 nesnede plan 293 ms.*
- [x] **CAD-EDIT-20:** Audit/recover/purge; bozuk ilişki, kayıp tanım ve kullanılmayan named resource bulsun; kullanılan içerik temizlenmesin. — *15 Ekim; [ADR 0306](../docs/adr/0306-audit-and-purge.md): AutoCAD AUDIT, RECOVER ve PURGE karşılaştırması (okuyucu olmayan katmanı ve bloğu zaten reddeder, KCAD açılışı ve kurtarma kopyası RECOVER'dır); anlamsal `cad.drawing.audit`: nesnesi olmayan bağlı yazı, ilişkili tarama ve ölçü bağı, projede olmayan yazı ve ölçü stili bulunur ve istenirse tek adımda onarılır (bağ kopar, stil Standart olur, kilitli katmandaki söylenir), kullanılmayan katman, grup, blok ve stiller raporlanır, stiller istenirse temizlenir; kullanılan hiçbir şey temizlenmez; Denetle aracı komutun planıyla önizler ve yazar; 20 ortak durum bağımsız başvurudan; Python (`kentos.cad.drawing`) ve MCP; 100 000 nesnede plan 149 ms, yazma 86 ms.*
- [x] **CAD-EDIT-21:** Holes ve multipart üzerinde mevcut açık redleri incele; destek genişlerken boşluklar, attribute, Z/M ve kalıcı ID kaybedilmesin. — *15 Ekim; [ADR 0307](../docs/adr/0307-holes-and-parts-in-edits.md): düzenleme komutlarının delikli alan, çok parçalı alan, çok parçalı çizgi ve çok noktalı nesnedeki davranışının envanteri; Kır, Buda ve Uzat çok parçalı çizgide tıklanan parçada çalışır, parçalar aynı nesnede kalır (kimlik, öznitelik, görünüş, kot ve ölçü korunur; Buda'nın “kesildi” bayrağı parça başına); çok parçalı ve delikli alan gerekçesiyle reddedilir; Uzat-kısalt, Paralel kaydır, Birleştir, Biçim değiştir ve Eğriyi düzenle kalanlar olarak yazıldı; 60 ortak durum bağımsız başvurudan.*
- [x] **CAD-EDIT-22:** COPYBASE, PASTECLIP/PASTEORIG/PASTEBLOCK eşdeğerlerinde belge birimi, kaynak katman/stil/blok ad çakışmasını planlı çöz. — *15 Ekim; [ADR 0309](../docs/adr/0309-clipboard-commands.md): `cad.entities.copy` kopyayı katman yolları, blokları, stilleri, kitaplığı, birimi ve taban noktasıyla verir; `cad.entities.paste` katman yoluna yazar, olmayan yolu açar, aynı adlı blok ve stilde bu çizimin tanımını kullanır, kilitli katmanda bütünüyle reddeder, başka birimi söyler (boylar metrede korunur), `asBlock` ile tek blok yapar; Taban noktasıyla kopyala ve Blok olarak yapıştır araçları; Python ve MCP; 100 000 nesnede kopyalama 120, yapıştırma 470–500 ms.*
- [x] **CAD-EDIT-23:** Drawing order, bring front/back, text above hatch, wipeout ve boundary görünürlüğünü tutarlı yap. — *15 Ekim; [ADR 0310](../docs/adr/0310-draw-order-and-wipeout.md): `cad.entities.order`'a Nesnenin üstüne ve altına (başvuru, öbür yan yer açmak için kayar), Yazıları öne getir ve Taramaları arkaya gönder; alanın maskesi `wipeout` (`.kcad` şema 54): zemin renginde, kendinden önce çizilenleri örter, yalnız sınırından seçilir, çerçevesi açılıp kapanır; Maske aracı (Çerçeve, Nesne), Öznitelikler'de Maske; Python ve MCP; 100 000 nesnede sıra 9–172 ms. Yazıları örtmek ve DXF'in WIPEOUT'u TODOS'ta.*

**Kabul:** Gerçek kat planında duvar çizgisi öteleme/budama/esnetme, detay çoğaltma, köşe yuvarlama ve blok yerleştirme; API'ye özgü el hesabı olmadan yapılır. İlişkili dizi aralığı değiştirilip undo edildiğinde kaynak ve bütün instance'lar geri gelir.

## 9. CAD-STYLE — Katmanlar, çizgi türleri ve görünüş — P1

Mevcut katman stilinin CAD `LineType` alanı dört temel türle sınırlıdır. GIS sembol motorunun daha geniş olması, DXF LIN deseninin veya nesne başına CAD linetype semantiğinin zaten tam olduğu anlamına gelmez. [E05, E10–E11]

- [x] **CAD-STYLE-01:** Katman/group oluştur, adlandır, taşı, sil, birleştir ve purge işlemlerini ortak ürün kataloğuna al. — *15 Ekim; [ADR 0311](../docs/adr/0311-layer-flags-states-find-translate.md) §6: ADR 0248'in `cad.layers.edit` ve `.set`'i (ekle, adlandır, taşı, sil, kopyala, birleştir, temizle) katalogda, Python'da ve MCP'de; eksikler dondurma, yalıtımı kaldırma ve çevirmeyle tamamlandı.*
- [x] **CAD-STYLE-02:** On/off, freeze/thaw, lock/unlock, plot/no-plot ve isolate/unisolate anlamlarını ayır; geçici görünüm ile kalıcı proje değişimi açık olsun. — *15 Ekim; [ADR 0311](../docs/adr/0311-layer-flags-states-find-translate.md) §1–§2: katmanın `frozen`'ı (`.kcad` şema 55; çizilmez, seçilmez, kenetlenmez, kapsama ve paftaya girmez; Hepsini göster çözmez; etkin katman dondurulmaz), `cad.layers.set`'in `frozen`, `thawAll`, `unisolate`'i; göz, kilit, Yazdır, Dondur ve ad projenin, Yalıt'ın gizledikleri oturumun; Katmanı dondur aracı, Dondur / Çöz, kar tanesi; DXF'in 70 grubunun 1. biti dondurma.*
- [x] **CAD-STYLE-03:** Katman özellikleri ve adlandırma şablonlarına mimari ofis profili ekle: duvar, doğrama, mobilya, eksen, ölçü, yazı, tarama ve revizyon. — *15 Ekim; [ADR 0312](../docs/adr/0312-drawing-standard-look-sources-print-colours.md) §6: yerleşik `mimari-ofis` standardı (`catalog/v1/standards/`): on A- katmanı (duvar, kapı, pencere, mobilya, eksen, ölçü, yazı, tarama, revizyon, yardımcı) rengi, çizgi tipi, kalınlığı, Yazdır'ı ve topladığı eski adlarla; `A-*` ad örüntüsü, Barlow ve Arimo, A-Yazı, A-Başlık, A-Ölçü; Standardı uygula ile çizime gelir.*
- [x] **CAD-STYLE-04:** Adlı layer states; capture/apply/update/import/export ve viewport kapsamını Python/AI'dan yönetilebilir yap. — *15 Ekim; [ADR 0311](../docs/adr/0311-layer-flags-states-find-translate.md) §3: durumlar dondurmayı ve yazdırmayı da saklar; `export` (`kentos.layer-states` dosyası, düğümler yollarıyla) ve `import` (kimlikle ya da yolla eşleme, aynı adlı atlanır ya da `replace`); Katman durumları'nda Dışa aktar… ve İçe aktar…; Python ve MCP. Görünüm (viewport) kapsamı `CAD-STYLE-12` ile.*
- [x] **CAD-STYLE-05:** Katman filtreleri, wildcard, property filtre ve layer translator eşdeğeri sun; toplu işlemin önizlemesi olsun. — *15 Ekim; [ADR 0311](../docs/adr/0311-layer-flags-states-find-translate.md) §4–§5: salt okunur `cad.layers.find` (AutoCAD'in `wcmatch` örüntüsü, renk, çizgi tipi, kalınlık, görünür, dondurulmuş, kilitli, yazdırılan, nesnesiz) ve `cad.layers.translate` (LAYTRANS; kurallar, standart katman eklenir ya da stillenir, kaynaklar birleşir, tek adım; plan önizleme); Katmanlar'da örüntülü arama ve Süzgeç ▾, Katmanları çevir penceresi; 28 + 28 ortak durum.*
- [x] **CAD-STYLE-06:** ByLayer/ByBlock/explicit değerleri renk, çizgi türü, kalınlık ve transparency için tutarlı çözümle. — *15 Ekim; [ADR 0312](../docs/adr/0312-drawing-standard-look-sources-print-colours.md) §4: tek kural `resolve_look` (kendi değeri, blokta `byLayer` parçası kendi katmanından, öbürleri yerleştirmeden BYBLOCK, çizimde katmandan); blok parçası çizgi tipini, ölçeğini ve saydamlığını da taşır; DXF yazıcısı tanımda BYBLOCK çizgi tipi ve saydamlık yazar, okuyucu tanımdaki BYLAYER çizgi tipini katmanınki sayar; masaüstünde bloktaki `byLayer`'ın hiç çalışmaması düzeldi; 22 ortak durum.*
- [x] **CAD-STYLE-07:** Theme token ile CAD palette/truecolor ayrımını koru; dışa aktarım ve plot rengi tema değişiminden etkilenmesin. — *15 Ekim; [ADR 0312](../docs/adr/0312-drawing-standard-look-sources-print-colours.md) §5: kâğıdın renkleri tek tabloda (`print_color`, `paper`): pafta, PDF ve lejant aynı; lejantta `fg-dim` ve `paper`'ın siyah basılması düzeldi; DXF'te ACI ve truecolor; hiçbir çıktı arayüzün temasını okumaz; 16 ortak durum.*
- [x] **CAD-STYLE-08:** LIN desenleri; dash/gap/dot dizisi, pattern length, continuous phase, text/shape içeriği ve dosya kökenini korusun. — *15 Ekim; [ADR 0313](../docs/adr/0313-line-type-patterns.md) §1–§2: `LineTypeDef` (öğeler, birim, kaynak), projenin `lineTypes`'ı, katmanın ve nesnenin `linePattern`'ı (KCAD şema 56), LIN okuyucu ve yazıcı, `cad.linetypes.edit` (load, add, remove, scale, export; 34 ortak durum), DXF LTYPE desenleriyle iki yönde, Çizgi tipleri penceresi.*
- [x] **CAD-STYLE-09:** Complex linetype ve SHX shape desteği için font/shape çözümleme kararı ver; desteklenmeyen desenin kaybını açık raporla. — *15 Ekim; [ADR 0313](../docs/adr/0313-line-type-patterns.md) §3: yazı ve şekil öğeleri saklanır ve geri yazılır, çizilmez (yerleri boşluk); pencere “yazı ve şekil: boşluk” der, DXF yazıcısı şekli raporla atlar; SHX çizimi ayrı iş.*
- [x] **CAD-STYLE-10:** Global/entity/polyline/paper-space linetype scale ilişkisini tanımla; viewport ölçeği değişiminde dash boyu tutarlı kalsın. — *15 Ekim; [ADR 0313](../docs/adr/0313-line-type-patterns.md) §4: boy = desen (kâğıt mm × ölçek/1000 ya da çizim birimi) × LTSCALE × nesnenin ölçeği; paftada kâğıtta aynı (PSLTSCALE=1 gibi); çoklu çizgide PLINEGEN; `$LTSCALE` DXF'te iki yönde.*
- [x] **CAD-STYLE-11:** Kesikli yay/polyline çizgilerinde faz devamı ve corner davranışı bütün renderer'larda eşit olsun. — *15 Ekim; [ADR 0313](../docs/adr/0313-line-type-patterns.md) §5: parçalar yolun başından uzaklığı taşır, üç çizim hattı ve PDF aynı topluluklardan okur; köşede desen sürer, başlık yalnız yolun uçlarında; testi `a_pattern_runs_on_along_a_path_past_its_vertices`; sınır 8 sayı.*
- [x] **CAD-STYLE-12:** Viewport bazında freeze, renk, linetype, lineweight ve plot override ekle; modeldeki kaynak stil değişmesin. — *16 Ekim; [ADR 0314](../docs/adr/0314-viewport-overrides-effects-library-impact.md) §1: paftanın haritasında katman durumu ve katman başına geçersiz kılma (`layerOverrides`: dondur, yazdır, renk, çizgi tipi ya da desen, kalınlık), tek uygulama yeri `kentos_plot::layers::map_layers` (pafta kipi ve PDF), model değişmez; `cad.sheets.edit` ile yazılır, pafta çekirdeği denetler; Denetçi'nin Katmanlar bölümü; 6 ortak durum.*
- [x] **CAD-STYLE-13:** Görünüş ve kaynak stil değişikliği için dirty/undo/cloud davranışı tek sözleşmede belirlensin. — *16 Ekim; [ADR 0314](../docs/adr/0314-viewport-overrides-effects-library-impact.md) §2: `CHANGE_EFFECTS` (16 satır: komut, adım / dışında / tercih, kirlilik, parçalar ve bulutta gidişleri), kataloğun `effects`'i; uygulamanın testi her satırın komutunu çalıştırıp tabloyu belgeyle, bulut testi `ProjectPatch` alanlarıyla denetler.*
- [x] **CAD-STYLE-14:** Style library ve block/library öğelerinin create/update/duplicate/delete/import/export/assign işlerini API'ye aç. — *16 Ekim; [ADR 0314](../docs/adr/0314-viewport-overrides-effects-library-impact.md) §3: `cad.library.edit` (create, update, duplicate, rename, move, remove, import, export, assign; 33 ortak durum), `cad.blocks.edit`'in `duplicate`, `export`, `import`'u (`kentos.blocks`, `.kblok`; 6 durum), kanonik metinler; `.kstil` kuralları `kentos-style-library`'de; Bloklar panelinde Kopyasını oluştur, Dışa aktar…, Blokları al…; Python ve MCP.*
- [x] **CAD-STYLE-15:** Stil değişikliği bağlı nesne, blok içi nesne ve xref override bakımından açık sonuç raporu versin. — *16 Ekim; [ADR 0314](../docs/adr/0314-viewport-overrides-effects-library-impact.md) §4: salt okunur `cad.styles.impact` (katman, çizgi tipi, yazı ve ölçü stili: izleyenler, kendi değerini koruyanlar özelliğe göre, kilitliler, blok parçaları ve yerleştirmeler, eski görünüşü saklayan durumlar, geçersiz kılan haritalar, dış başvuru 0); Değişikliğin etkisi penceresi (Katmanlar'ın sağ tıkı, Çizgi tipleri); 20 ortak durum; 100 000 nesnede 5–17 ms.*
- [x] **CAD-STYLE-16:** Drawing standard checker; yanlış layer adı, font, dimstyle, linetype ve plot profile'ı raporlasın; düzeltme ayrı planlı işlem olsun. — *15 Ekim; [ADR 0312](../docs/adr/0312-drawing-standard-look-sources-print-colours.md) §1–§3: çizim standardı veri olarak; salt okunur `cad.standards.check` (katmanlar, adlar, yazı tipleri, stiller, çizgi tipleri, kendi görünüşlü nesneler; plot profili katmanın Yazdır'ı ve kalınlığıdır) ve ayrı, planlı `cad.standards.apply` (birleştir, ekle, düzelt, stiller); Çizim standardı penceresi; 25 + 23 ortak durum.*

**Kabul:** Bir ofis katman/stil şablonu UI, Python ve AI ile aynı biçimde kurulabilir. Farklı ölçekli paftalarda çizgi tipi ve plot kalınlığı beklenen fiziksel boyutları korur.

## 10. CAD-BLOCK — Statik, iç içe ve parametrik bloklar — P1/P2

Mevcut blok modeli tek uniform `scale` ve `mirror` taşıyor. Tanım içindeki nesne katmanları saklansa da yerleştirmenin katmanında çizilme yaklaşımı mevcut; AutoCAD blok içi gerçek katman, layer 0 ve ByBlock davranışıyla eşdeğerlik ayrıca geliştirilmelidir. Xref, dinamik blok ve blok düzenleme kipi mevcut blok tesliminin açık kapsam dışı alanlarıdır. [E05, E11, E20]

- [x] **CAD-BLOCK-01:** Blok tanımı/instance ID, nested path ve alt nesne referanslarını ölçü/snap/selection için kararlı yap. — *16 Ekim; [ADR 0315](../docs/adr/0315-block-ids-layers-scales-units.md) §1: blok içi yol tanım değişince parçanın yerel kimliğiyle izlenir (`Blocks::path_in`, `ops::subref::follow_path`), ölçünün `path`'i güncellenir; `subref_cases.py`'nin iki yeni durumu.*
- [x] **CAD-BLOCK-02:** Layer 0 kalıtımı ile gerçek katmandaki blok içi nesnenin kendi katmanını ayrı çözümle; ByBlock ve ByLayer doğrulama çizimleri hazırla. — *16 Ekim; [ADR 0315](../docs/adr/0315-block-ids-layers-scales-units.md) §2: 0'daki parçanın `byLayer`'ı yerleştirmenin katmanından, kendi katmanındaki parça o katman görünürken görünür (iç içe de), Blok oluştur parçaları çizildiği gibi (`byLayer`), DXF'in BYLAYER'ı `byLayer`; `look_cases.py` 28 durum, doğrulama çizimi `block_scenes.rs`.*
- [x] **CAD-BLOCK-03:** Nonuniform X/Y scale, negatif scale/mirror ve nested transform için kaynak modeli genişlet; daire→elips gibi dönüşümler analitik yönetilsin. — *16 Ekim; [ADR 0315](../docs/adr/0315-block-ids-layers-scales-units.md) §3: `InsertEntity.yScale` (`.kcad` şema 57), parçalar afin bükmeyle (daire, yay, elips kesin elips; yolun yayları 0,1 mm), iç içe bileşir, Patlat eğilen iç bloğu reddeder; DXF 41/42; Blok ekle'nin Y ölçeği, Öznitelikler'in X/Y ölçeği; `block_scale_cases.py`.*
- [x] **CAD-BLOCK-04:** Blok birimi ve insert unit conversion ekle; mm bloğu metre çizime yerleştirme kullanıcıya sürpriz yaratmasın. — *16 Ekim; [ADR 0315](../docs/adr/0315-block-ids-layers-scales-units.md) §4: `BlockDefinition.units` (birimli gerçek boyutlu, birimsiz çizim birimiyle), anlamsal `cad.blocks.insert` (26 durum; `insert_units_differ`), `cad.blocks.edit`'in `units`'i (`rescale`), DXF `BLOCK_RECORD` 70; Bloklar panelinde Birim ▸.*
- [x] **CAD-BLOCK-05:** Yerinde blok düzenleme / ayrı block editor; giriş, çalışma kopyası, scope, save/discard ve return akışlarını tanımla. — *16 Ekim; [ADR 0316](../docs/adr/0316-block-editor-files-and-size.md) §1: Blok düzenleyici (`block.edit`, BEDIT; Bloklar panelinde Bloğu düzenle): tanımın nesneleri yerel kimlikleriyle ayrı bir çizimde, “0 (blok)” katmanı, kendi geri alması; açıkken dosya, bulut ve pafta komutları bekler; Kaydet `cad.blocks.edit`'in `update`'iyle tek adım, Vazgeç dokunmaz, ikisi de görünüme ve seçime döner.*
- [x] **CAD-BLOCK-06:** Blok düzenleme commit'i bütün instance'ları, ölçü bağlarını, snap deposunu, bbox ve render cache'i doğru invalid etsin. — *16 Ekim; [ADR 0316](../docs/adr/0316-block-editor-files-and-size.md) §2: `update` bloğun kimliğini ve parçaların yerel kimliklerini korur; ölçünün blok içi bağı kimlikle izler; masaüstünde depo ve stilli çizim belgenin günlüğünden yeniden kurulur, Geri al tek adım (`block_editor::tests`).*
- [x] **CAD-BLOCK-07:** WBLOCK / dosyaya blok çıkarma, dosyadan blok insert ve başka projeden yeniden tanımlama mevcut alışveriş yolunun semantik API'si olsun. — *16 Ekim; [ADR 0316](../docs/adr/0316-block-editor-files-and-size.md) §3: `file.export.block` (WBLOCK: blok ya da nesneler, taban başlangıçta, aynı blok aynı bayt) ve `file.import.block` (`drawing`: dosya tek blok; `blocks`: adlarıyla, `replace` yeniden tanımlar; çakışan kimlik yenilenir); Dosyadan blok ekle, Dosyaya yaz…, Başka çizimden yeniden tanımla… bunlarla; 25 ortak durum.*
- [ ] **CAD-BLOCK-08:** Blok attribute; visible/invisible, constant, preset, verify, locked position, multiline ve field seçeneklerini veri modeliyle tamamla.
- [ ] **CAD-BLOCK-09:** Attribute tanımı değişince instance değerini koruma/sync politikası olsun; eksik/yeni/değişen tag'ler raporlansın.
- [ ] **CAD-BLOCK-10:** Blok replace, count, select instances, find by attribute, export schedule ve purge kullanılabilir olsun.
- [x] **CAD-BLOCK-11:** Nested/self-referential bloklarda döngü, derinlik, kaynak bütçesi ve eksik tanım redlerini bütün host'larda eşit uygula. — *16 Ekim; [ADR 0316](../docs/adr/0316-block-editor-files-and-size.md) §4: `block_too_large` (bir milyon parça, öznitelikler dahil, doymalı sayım `blocks::sizes`) döngü ve 16 düzeyin yanında `kentos_contracts::blocks`'ta; `.kcad` okuyucusu, Python okuyucusu, iki belge, komutlar ve sunucu aynı kuraldan.*
- [ ] **CAD-BLOCK-12:** Kütüphane kartları; thumbnail, kategori, arama, ölçü/birim, parametreler, tag ve sürüm taşısın.
- [ ] **CAD-BLOCK-13:** Kapı, pencere, mobilya, ıslak hacim, ölçü işareti, kesit ve antet için KentOS'a ait veya dağıtım izni açık örnek içerik hazırla.
- [ ] **CAD-BLOCK-14:** Dynamic block için parameter/action modeli tanımla: linear, polar, angle, point, flip, visibility ve lookup.
- [ ] **CAD-BLOCK-15:** Move/stretch/rotate/scale/array/flip action'ları kaynak geometri üzerinde deterministik çalışsın; instance parametreleri kalıcı olsun.
- [ ] **CAD-BLOCK-16:** Parametre aralığı, adım, değer listesi, tip/birim, expression ve lookup table doğrulansın.
- [ ] **CAD-BLOCK-17:** Görünürlük durumları ve grip davranışları renderer/snap/selection/plot'ta aynı kapsamı kullansın.
- [ ] **CAD-BLOCK-18:** Parametrik blok düzenleme UI'sı ile `parameters.get/set/reset`, validation ve preview API'leri aynı modele bağlı olsun.
- [ ] **CAD-BLOCK-19:** Dynamic block için kaynak parametreli düzenleme, proxy görünüş ve bağımsız geometriye çevirme farklı capability'ler olarak yayımlansın.
- [ ] **CAD-BLOCK-20:** AutoCAD dynamic block verisinin DWG/DXF ile aktarımını gerçek dosya korpusu üzerinde sınamadan “tam uyumlu” olarak etiketleme.

**Kabul:** Tek kapı bloğunun genişliği, açılımı, handedness ve visibility değeri değiştirilebilir; instance attribute'ları korunur. İç içe blok ve farklı layer renkleri UI/PDF/DXF'te aynı görünür.

Araştırma dayanağı: Dynamic block, aynı tanımın farklı boyut ve görünüşle kullanılmasına imkân verir. KentOS'un yukarıdaki iş listesi bu iş akışının kendi kaynak modeline uygulanması için öneridir. [Autodesk Dynamic Blocks](https://help.autodesk.com/view/ACD/2026/ENU/?caas=caas%2Fdocumentation%2FCIV3D%2F2014%2FENU%2FfilesACD%2FGUID-3C2FB982-3AF6-437B-987F-4EDF81EA0662-htm.html)

## 11. CAD-ANNO — İlişkili ölçüler, yazı, kılavuz ve tablolar — P0/P1

### 11.1 İlişkili ölçü ve annotation grafı

AutoCAD'de ilişkili ölçü, ölçtüğü geometri değiştiğinde konum/yön/değerini izler; kopan ilişki ayrıca görülebilir ve yeniden bağlanabilir. KentOS `DimensionEntity` şu anda koordinatları ve görünüşü taşıyor, ölçülen entity/alt geometri için kalıcı bir referans alanı taşımıyor. [E05; Autodesk ilişkili ölçüler](https://help.autodesk.com/cloudhelp/2026/ENU/AutoCAD-MAC-Core/files/GUID-3559D5D2-AD15-4167-9BD9-EE5FCEB4F8CE.htm)

- [x] **CAD-ANNO-01:** Ölçü uçları/merkezi/kenarı için `EntityRef/SubentityRef` tabanlı ilişki modeli ekle; derived koordinatlar kaynak bağdan hesaplanabilsin. — **Bitti (12 Ekim; CADP-06/07, ADR 0249: nokta yöntemlerinin uçları).**
- [x] **CAD-ANNO-02:** Free point, ilişkili, kısmen ilişkili ve kopuk durumları açıkça sakla; son görünen koordinat kaybolmasın. — **Bitti (12 Ekim; CADP-06/07, ADR 0249).**
- [ ] **CAD-ANNO-03:** Move/stretch/trim/join/vertex edit, blok redef ve xref reload sırasında referans eşleme/invalidasyon politikasını uygula. — *Kısmen (12 Ekim; CADP-06/07, ADR 0249: taşıma, esnetme ve köşe düzenlemesi izler; budama ve birleştirmede eşleme, blok ve xref açık).*
- [ ] **CAD-ANNO-04:** Kopuk ölçü rozeti, bağlı geometri vurgusu, reason code ve yeniden bağlama komutu olsun. — *Kısmen (12 Ekim; CADP-06/07, ADR 0249: kopuk işareti, nedeni ve Yeniden bağla; bağlı geometri vurgusu açık).*
- [x] **CAD-ANNO-05:** Yeniden bağlama önizlemesi ölçü değerinin nasıl değişeceğini gösterip tek adımda uygulanabilsin. — **Bitti (12 Ekim; CADP-06/07, ADR 0249).**
- [x] **CAD-ANNO-06:** Kaynak değişikliği ile bağlı ölçü güncellemesi aynı changeset ve undo grubunda olsun. — **Bitti (12 Ekim; CADP-06/07, ADR 0249).**
- [ ] **CAD-ANNO-07:** Zincir, baz, açı, radius/diameter, ordinate, arc length ve jogged ölçü yöntemlerinin ilişkili karşılıklarını tamamla. — *Kısmen (12 Ekim; CADP-06/07, ADR 0249: Koordinat bağlanır; zincir, baz, açı, yarıçap, çap, yay uzunluğu ve kırıklı açık).*
- [ ] **CAD-ANNO-08:** DIMSPACE/DIMBREAK eşdeğeri; spacing, çizgi kesintisi, text move, inside/outside ve fit politikası sun.
- [ ] **CAD-ANNO-09:** Birincil/alternatif unit, tolerance, limit, fraction, rounding increment, zero suppression ve açık ölçü value override destekle.
- [ ] **CAD-ANNO-10:** Hesaplanan ölçü ile kullanıcı metni/override ayrımı belli olsun; yanlış yazılmış override geometrinin gerçek ölçüsü gibi raporlanmasın.
- [ ] **CAD-ANNO-11:** Center mark/centerline ve datum/elevation işaretleri bağlı kaynak nesneyi izlesin.
- [ ] **CAD-ANNO-12:** Annotation nesnesine scale representations ekle: katıldığı ölçekler, görünürlük, konum/rotation override ve reset.
- [ ] **CAD-ANNO-13:** Aynı yazı/ölçü/tag 1:50 ve 1:100 viewportlarda istenen kâğıt yüksekliğinde çizilsin; bir viewport değişikliği öbür temsili bozmasın.
- [ ] **CAD-ANNO-14:** Projenin mevcut plot scale izleme davranışını çok ölçekli annotation modeline açık göçle bağla; eski nesnelerin boyu değişmesin.
- [ ] **CAD-ANNO-15:** Paper-space ölçüsünün model viewport nesnesine bağlanma politikasını tanımla; ölçek çarpanı double uygulanmasın.

### 11.2 Yazı, fields ve font

- [ ] **CAD-ANNO-16:** Yazı/ölçü/kılavuz/table style create/edit/delete/import/export için ortak komutlar ve tipli API sun.
- [ ] **CAD-ANNO-17:** Stilin explicit override alanlarını değer eşitliğine dayanarak tahmin etmek yerine kaynak/override bayrağıyla temsil etmeyi değerlendir; eski dosya davranışını göç testleriyle koru.
- [ ] **CAD-ANNO-18:** Unicode, Türkçe karakter, sembol, satır kırma, rich text ve width/oblique davranışını web/native/PDF/DXF'te karşılaştır.
- [ ] **CAD-ANNO-19:** MTEXT paragraph hizası, indent, tabs, listeler, columns ve stacked fractions için eksikleri sırala ve tamamla.
- [ ] **CAD-ANNO-20:** Harici TTF/OTF font çözümleme; font file/family eşleme, fallback, missing-font raporu ve PDF embedding/subsetting politikasını kur.
- [ ] **CAD-ANNO-21:** SHX ve bigfont için okuma/render/vektöre dönüştürme stratejisi belirle; tahmini font değişimini sessiz yapma.
- [ ] **CAD-ANNO-22:** Field nesneleri; alan/uzunluk, block attribute, proje/pafta adı/numarası, viewport scale ve çapraz referans değerlerini bağlı kaynaklardan alsın.
- [ ] **CAD-ANNO-23:** Fields için typed expression, format/unit, cache, dirty/recompute, döngü ve kopuk kaynak durumlarını tanımla.
- [ ] **CAD-ANNO-24:** TEXT/MTEXT çevirme, numaralandırma, bul/değiştir ve spell/check akışlarını mevcut altyapı üzerinden API'ye aç.

### 11.3 Multileader ve CAD tabloları

- [ ] **CAD-ANNO-25:** Bir kılavuz nesnesinde birden çok ok yolu, ortak landing, MTEXT veya block content ve named style desteği ekle.
- [ ] **CAD-ANNO-26:** Kılavuz uçlarını nesne/alt geometriye bağla; taşınan kapı/mahal etiketi ilişkiyi korusun.
- [ ] **CAD-ANNO-27:** Leader alignment, collect, content edit, arrow add/remove ve style override işlemlerini semantik komutlara al.
- [ ] **CAD-ANNO-28:** CAD table için cell format/style, merge, formula, field, satır/sütun büyütme ve source update davranışını tamamla.
- [ ] **CAD-ANNO-29:** Dosya bağlantılı çizelgede path/asset, source digest, refresh, kayıp kaynak ve kullanıcı override politikası olsun.
- [ ] **CAD-ANNO-30:** Table, dimension ve multileader'ı dışa aktarırken native entity / block görünüş / KentOS metadata seçenekleri ayrı raporlansın.

**Kabul:** 4000 mm duvar 4500 mm'ye esnetildiğinde bağlı ölçü 4500 olur; kaynak ve ölçü tek undo'da geri gelir. 1:50 ve 1:100 viewportlarda 2.5 mm annotation aynı fiziksel boydadır. Bir kapı sayısı değişince field ve çizelge güncellenir.

## 12. CAD-HATCH — Tarama, sınır, dolgu ve maskeleme — P1

Mevcut desen/degrade ve ilişkili tarama kodu korunmalıdır. Hedef yeni bir tarama aracı açmak değil, kaynak sınır doğruluğunu ve profesyonel düzenleme/aktarımı tamamlamaktır. [E05, E11]

- [ ] **CAD-HATCH-01:** Line/arc/ellipse/NURBS kaynak boundary edge'lerini kayıpsız temsil et; render ring ile kaynak boundary birbirine karışmasın.
- [ ] **CAD-HATCH-02:** Associative hatch kaynaklarını kararlı alt geometri referanslarıyla bağla; boundary değişikliği ilgili taramayı aynı transaction'da güncellesin.
- [ ] **CAD-HATCH-03:** Pick point / select boundaries / çoklu bölge / island detection yöntemlerinin sayısal ve UI kapsamını doğrula.
- [ ] **CAD-HATCH-04:** Gap tolerance ve boundary repair; kapanan boşluk ve değişen alanı plan raporunda göstersin.
- [ ] **CAD-HATCH-05:** Boundary oluşturma, boundary yeniden seçme, add/remove island ve ilişkiyi koparma işlemleri API'de olsun.
- [ ] **CAD-HATCH-06:** PAT import/export ve kullanıcı desenleri; pattern name, family, dash ve origin bilgilerini korusun.
- [ ] **CAD-HATCH-07:** Pattern scale/angle/origin, match hatch properties ve align across objects seçeneklerini tamamla.
- [ ] **CAD-HATCH-08:** Çok ölçekli/annotative pattern ile kaynak world ölçeğinde pattern açık ayrı seçenekler olsun.
- [ ] **CAD-HATCH-09:** Solid/gradient transparency ve ön/arka çizim sırası tutarlı olsun; text background mask ve wipeout ayrı nesne/özellikler olarak ele alınsın.
- [ ] **CAD-HATCH-10:** Bağlı metinleri boş bırakma davranışı metin taşındığında da güncellensin; döngü ve missing text durumu raporlansın.
- [ ] **CAD-HATCH-11:** Çok büyük/dense hatch için bütçe ve cancellation olsun; desenin ekranda sadeleşmesi kaynak ölçü/plot doğruluğunu değiştirmesin.
- [ ] **CAD-HATCH-12:** Render ve PDF'de çizgi kalınlığı, dash, kesişim ve clip kuralları aynı görünüşü üretsin.
- [ ] **CAD-HATCH-13:** DXF HATCH boundary loops, source references, gradients ve custom pattern aktarımı için bağımsız dosyalarla round-trip test et.

**Kabul:** Yaylı duvar ve adalı mahal sınırı değiştirildiğinde tarama doğru alanı izler; hiçbir delik kapanmaz. UI/Python/AI aynı pick-point veya boundary-ID girdisinden aynı taramayı oluşturur.

## 13. CAD-PARAM — Geometrik ve ölçüsel kısıtlar — P2

AutoCAD'de geometrik kısıtlar 2D nesne ilişkilerini, ölçüsel kısıtlar ise değerleri korumaya yarar. KentOS'un pafta yerleşimindeki `constraint` altyapısı veya settings constraint alanları, CAD geometri çözücüsüyle karıştırılmamalıdır. [E05; Autodesk kısıtlar](https://help.autodesk.com/cloudhelp/2026/ENU/AutoCAD-LT-MAC/files/GUID-7D93999A-6CAF-4F54-B7E5-DD2AF29B1564.htm)

- [ ] **CAD-PARAM-01:** CAD constraint graph, parameter table, kaynak ID ve subentity bağlarını yeni sözleşme olarak tasarla.
- [ ] **CAD-PARAM-02:** Coincident, horizontal, vertical, parallel, perpendicular, tangent, concentric, collinear, equal, symmetric ve fix kısıtlarını önceliklendirilmiş sırayla ekle.
- [ ] **CAD-PARAM-03:** Distance, angle, radius ve diameter kısıtlarında driving ile reference/read-only ölçüyü ayır.
- [ ] **CAD-PARAM-04:** Birimli numeric parametreler, ifade, bağımlılık grafı, değer aralığı ve döngü teşhisini uygula.
- [ ] **CAD-PARAM-05:** Çözücü seçimi için kendi implementasyon/mevcut kütüphane seçeneklerini benchmark ve lisansla karşılaştır; bu plan belirli bir çözücüyü zorunlu tutmasın.
- [ ] **CAD-PARAM-06:** Çözücü under/fully/over-constrained, inconsistent, singular ve non-convergent sonuçlarını yapılandırılmış dönsün.
- [ ] **CAD-PARAM-07:** Alternatif çözümler ve yön/handedness korunması politikası olsun; küçük değişiklikte şekil beklenmedik ters dönmesin.
- [ ] **CAD-PARAM-08:** Grip ve move/stretch gibi düzenlemelerde kısıtlar korunsun; geçici relax açık seçenek olsun.
- [ ] **CAD-PARAM-09:** Trim/join/break/copy sırasında kısıt kaybı veya kopyalanması açık plan/uyarı olarak sunulsun.
- [ ] **CAD-PARAM-10:** Constraint ekleme/silme, parametre değiştirme ve çözülen bütün geometriler tek transaction'da yazılsın.
- [ ] **CAD-PARAM-11:** UI constraint glyph, bağlı geometri vurgusu, parametre tablosu ve conflict çözüm akışı olsun.
- [ ] **CAD-PARAM-12:** Python/AI create/list/update/remove/solve/diagnose çağrıları aynı çözücüyü kullansın.
- [ ] **CAD-PARAM-13:** Constraint çözümü worker/native hesapta çalışsın; timeout/cancel staged değişikliği bıraksın.
- [ ] **CAD-PARAM-14:** Architectural host ilişkileri ile genel constraint graph'ın otoritesini belirle; kapı duvara hem iki ayrı sistemden taşınmasın.
- [ ] **CAD-PARAM-15:** DWG/DXF constraint aktarımının destek sınırını ayrıca belirt; KentOS'ta kısıt olması dışarıda düzenlenebilir kısıt olarak korunmasını otomatik kanıtlamaz.

**Kabul:** Dikdörtgenin genişliği değiştirildiğinde karşı kenarlar paralel ve köşeler dik kalır. Çelişkili değer bütün host'larda aynı teşhisi üretir ve hiçbir geometri yazılmaz.

## 14. CAD-ARCH — 2D mimari üretim araçları — P1/P2

Architecture toolset'te duvarlar katmanlı bileşenler taşıyabilir; kapı/pencere duvara bağlanır, duvar açıklığına uyum sağlar ve bağlı öğe taşınan duvarı izler. KentOS için aşağıdaki **2D ürün tasarımı**, bu iş akışından çıkarılan gereksinimlerin mevcut CAD kaynak modeline uyarlanmasıdır. [Autodesk mimari iş akışı](https://help.autodesk.com/cloudhelp/2026/ENU/AutoCAD-Architecture/files/GUID-508C88DB-2CB7-4D0C-A60D-8D4CCC7CC5A6.htm), [Autodesk Walls](https://help.autodesk.com/cloudhelp/2025/ENU/AutoCAD-Architecture/files/GUID-E7595DBB-BB79-4006-ADE5-17E4BD676110.htm)

### 14.1 Mimari nesne modeli ve duvar

- [ ] **CAD-ARCH-01:** `Wall`, `Opening`, `Door`, `Window`, `Space`, `Grid`, `Stair2D` ve mimari tag/schedule kaynak modellerini tanımla; anlamsız attribute yığınlarıyla ilişkiyi taklit etme.
- [ ] **CAD-ARCH-02:** Kaynak nesne ile çizim primitive'lerini ayır; duvarın çizilmiş iki kenarı duvarın kimliği ve otoritesi yerine geçmesin.
- [ ] **CAD-ARCH-03:** Kat, kot, discipline, phase/renovation ve construction status bilgisi; açık typed alanlar ve ofis şablonlarıyla taşınsın.
- [ ] **CAD-ARCH-04:** Wall style; component/material, kalınlık, offset, çizgi, tarama, endcap ve ölçeğe göre detay seviyesi taşısın.
- [ ] **CAD-ARCH-05:** Duvar çizimi; line/arc/polyline yol, width, justify left/center/right, base line offset ve direction seçeneklerini kapsasın.
- [ ] **CAD-ARCH-06:** L/T/X ve eğik duvar birleşimlerinde öncelik/material cleanup kuralları olsun; başarısız birleşim tanımlı uyarı ve repair önerisi versin.
- [ ] **CAD-ARCH-07:** Duvar uçları, birleşimler, köşe detayı ve component temizliği kullanıcı tarafından override edilebilsin; reset otomatik kurala dönsün.
- [ ] **CAD-ARCH-08:** Düz/yaylı duvarda kalınlık değişikliği bağlı açıklıklar, mahal sınırları, tarama ve ölçülerle aynı changeset'te çözülsün.
- [ ] **CAD-ARCH-09:** Çizgi/polyline'dan duvara dönüştürme; kaynak çizgileri koru/sil, eşleşme, tolerans ve önerilen style ile önizlemeli çalışsın.
- [ ] **CAD-ARCH-10:** Wall split/join/extend/trim/flip/style replace için doğrudan semantik komutlar olsun.
- [ ] **CAD-ARCH-11:** Explode/export-to-basic-CAD; duvarı görünüş primitive'lerine çevirirken kaybedilen semantik ilişkiyi raporlasın.
- [ ] **CAD-ARCH-12:** Mimari görünüş; 1:20 detaylı, 1:50 uygulama, 1:100 şematik gibi profillerle kaynak nesneyi değiştirmeden türetilebilsin.

### 14.2 Açıklık, kapı ve pencere

- [ ] **CAD-ARCH-13:** Açıklık; host wall ID, wall path position, width, anchor/reference side ve gerekiyorsa sill/head/elevation verisini taşısın.
- [ ] **CAD-ARCH-14:** Door/window type; kasa/kanat, leaf count, width, handedness, swing/slide, frame/reveal ve sembol profilini taşısın.
- [ ] **CAD-ARCH-15:** Insert sırasında duvarı bulma, köşeden mesafe, duvar ortası, duvar yönüne göre flip ve sayısal konum girişini destekle.
- [ ] **CAD-ARCH-16:** Kapı kanadı ve açılım yayı genişliğe bağlı analitik geometri olsun; swing açısı ve show/hide seçeneği bulunsun.
- [ ] **CAD-ARCH-17:** Taşınan duvar açıklığı taşısın; silinen/çıkarılan kapının boşluğu belirlenen kuralla kapanabilsin.
- [ ] **CAD-ARCH-18:** Açıklık uçları duvarı aşarsa, iki açıklık çakışırsa veya host silinirse reason code ve kullanıcı çözümü sun; sessiz rehost yapma.
- [ ] **CAD-ARCH-19:** Açıklık moving/copy/stretch/flip/rehost/type replace, batch numbering ve width edit bütün yüzeylerde çalışsın.
- [ ] **CAD-ARCH-20:** Kapı/pencere tag'i kaynak nesneye bağlı kalsın; duplicate numara, eksik type ve kopuk host raporlansın.
- [ ] **CAD-ARCH-21:** Kapı/pencere çizelgesi filtre, sort, type grouping, sayı, boy ve kullanıcı alanlarıyla kaynağı izlesin.
- [ ] **CAD-ARCH-22:** Çizelge hücresinde düzenlenebilir kaynak property ile yalnız computed değer ayrımı açık olsun.

### 14.3 Mahal, alan ve metraj

- [ ] **CAD-ARCH-23:** Space oluşturma; kullanıcı sınırı, kapalı geometri veya wall topology üzerinden çalışsın; delikler ve bağımsız parçalar tanımlı olsun.
- [ ] **CAD-ARCH-24:** Space boundary; net/brüt/duvar orta hattı gibi ölçme yöntemini açık policy alanında taşısın; “alan” tek belirsiz sayı olmasın.
- [ ] **CAD-ARCH-25:** Mahal kodu, adı, kullanım, kat, finish ve özel alanlar; typed şemalarla düzenlensin.
- [ ] **CAD-ARCH-26:** Duvar/açıklık değişince mahal sınırı, alanı, etiketi ve çizelgesi aynı bağımlılık grafında güncellensin.
- [ ] **CAD-ARCH-27:** Mahal merge/split ve lost enclosure; eski kimliği nasıl koruduğunu ve yeni kimlikleri nasıl verdiğini raporlasın.
- [ ] **CAD-ARCH-28:** Net/brüt alan, çevre ve duvar/açıklık metrajı kaynak geometriden hesaplanıp provenance ile dönsün.
- [ ] **CAD-ARCH-29:** Mahal ve metraj tabloları floor/zone/type/phase filtreleriyle üretilebilsin; PDF/CSV/XLSX veya CAD table sonucu aynı değerleri taşısın.
- [ ] **CAD-ARCH-30:** Ölçme yöntemi ofis/kurum profili olabilir; güncel mevzuata otomatik uygunluk iddiası ayrı doğrulanmış bir ürün işi olarak ele alınsın.

### 14.4 Aks, merdiven ve mimari semboller

- [ ] **CAD-ARCH-31:** Aks sistemi; düz/yaylı aks, aralık dizisi, alphabetic/numeric numbering, bubble ve uzatma/clip sınırı taşısın.
- [ ] **CAD-ARCH-32:** Aks etiketi ve aks ölçüsü kaynak aksa bağlı kalsın; numara değişince çapraz referanslar izlesin.
- [ ] **CAD-ARCH-33:** Kolon/taşıyıcı sembolleri; aks konumu, ölçü, rotation, type ve tag ile 2D olarak üretilebilsin.
- [ ] **CAD-ARCH-34:** Merdiven 2D; straight/L/U, landing, width, riser/tread/count, direction arrow, cut line ve numbering parametreleriyle çizilsin.
- [ ] **CAD-ARCH-35:** Parametre uyuşmazlığı ve geometrik sığmama hesaplanıp raporlansın; merdiven tasarımının mevzuat onayı gibi sunulmasın.
- [ ] **CAD-ARCH-36:** Rampa 2D; yön, width, run/rise/slope, landing ve kot etiketi için açık parametreler taşısın.
- [ ] **CAD-ARCH-37:** Kesit/görünüş/detail callout ve kot işaretleri; hedef görünüm/pafta referanslarını kaynak olarak taşısın.
- [ ] **CAD-ARCH-38:** Pafta numarası veya görünüm adı değiştiğinde callout field'ları yenilensin.
- [ ] **CAD-ARCH-39:** Tavan grid, floor finish pattern, furniture/fixture ve room tag yerleştirme; mevcut block/hatch/style temellerini kullansın.
- [ ] **CAD-ARCH-40:** Kullanıcının çizdiği 2D kesit/görünüşleri named view ve sheet view olarak düzenle; kaynak plan–detay referansları yönetilebilsin.

### 14.5 Mimari üretim paketi

- [ ] **CAD-ARCH-41:** Mimari çalışma şablonu; mm veya cm kullanıcı birimi, layer/style/block, annotation scale, plot profile ve pafta şablonlarıyla bir ürün paketi olsun.
- [ ] **CAD-ARCH-42:** Mevcut/yıkılacak/yeni phase gösterimi; görünürlük, linetype/hatch, tag ve schedule filtreleriyle çalışsın.
- [ ] **CAD-ARCH-43:** Kat planları ve detay dosyalarını drawing set içinde organize et; değişen kaynaklardan etkilenen paftalar bulunabilsin.
- [ ] **CAD-ARCH-44:** Mimari nesnelerin basic DXF/DWG görünüşü ile KentOS semantic metadata'sı ayrı doğrulansın.
- [ ] **CAD-ARCH-45:** Tam otomatik section/elevation için gerekli yükseklik/katman/material modeli ve doğruluk sınırları ayrıca tasarlansın; bu teslim için elle 2D üretim zorunlu kalsın.

**Kabul:** Duvar–kapı–mahal içeren bir kat planında duvarı 500 mm taşımak kapıyı, ilişkili ölçüyü, mahal alanını ve çizelgeyi tutarlı günceller. Aynı iş Python betiği ve AI araç çağrılarıyla da yapılır; tek undo kaynak ve türevleri geri getirir.

## 15. CAD-REF — Çoklu belge, dış referanslar ve altlıklar — P1

AutoCAD xref, kaynak dosya değişimini reload ile yansıtabilen bağlı bir çizimdir; normal blok insert veya çizimden tek seferlik veri alma ile aynı iş değildir. KentOS DXF akışında xref açıkça atlanıyor. [E11; Autodesk Xrefs](https://help.autodesk.com/view/ACDLT/2026/ENU/?caas=caas%2Fdocumentation%2FACD%2F2014%2FENU%2Ffiles%2FGUID-A987D2FF-45BD-474E-99C1-E6316A42F667-htm.html)

- [ ] **CAD-REF-01:** Çoklu drawing/document handle modeli; belge sekmeleri, aktif belge ve her belgenin ayrı undo/selection/view/tool durumunu tanımla.
- [ ] **CAD-REF-02:** Belge kapanınca query, subscription, GPU buffer, worker, Python handle, AI context ve geç sonuçları invalid et.
- [ ] **CAD-REF-03:** Belge A'da çalışan Python/AI işi sırasında B'ye geçiş hedefi değiştirmesin; target document ID bütün çağrılarda bulunsun.
- [ ] **CAD-REF-04:** Belgeler arası copy/paste ve style/block transferinde birim ve ad çakışması çözümlensin.
- [ ] **CAD-REF-05:** `ExternalReference` kaynak modeli; path/asset/project-revision, attach/overlay, transform, birim, revision/digest ve yüklenme durumu taşısın.
- [ ] **CAD-REF-06:** Attach, detach, reload, unload, repath, bind ve insert-as-copy ayrı komutlar olsun.
- [ ] **CAD-REF-07:** Attach/overlay nested propagation kurallarını test et; döngülü referans zinciri açık reddedilsin.
- [ ] **CAD-REF-08:** Relative/absolute/project asset yolları, search paths ve missing reference için güvenilir resolver kur.
- [ ] **CAD-REF-09:** Referans değişince bildirim ve etkilenen view/sheet listesi sun; uygulama kullanıcı çalışmasını sessizce değiştirip kaydetmesin.
- [ ] **CAD-REF-10:** Reload kontrollü snapshot/revision değişikliği olsun; eski plan/ölçü bağı/snap sonucu gerektiğinde stale sayılsın.
- [ ] **CAD-REF-11:** Reference layer namespace, görünürlük, fade ve override retention kuralları tanımlansın.
- [ ] **CAD-REF-12:** Rectangle/polygon/inverted xref clipping; frame display ve plot/no-plot ayrı seçenekler olsun.
- [ ] **CAD-REF-13:** Nested xref snap ve selection; kaynak document ID, ref path ve entity/subentity ID'yi döndürsün.
- [ ] **CAD-REF-14:** Bind işleminde kaynak nesne, layer/style/block isimleri ve bağlı ölçülerin nasıl dönüştüğü açık raporlansın.
- [ ] **CAD-REF-15:** Kaynağı yerinde düzenleme; salt okunur referans ve kaynak dosyayı değiştiren iş ayrı yetki/undo sınırı taşısın.
- [ ] **CAD-REF-16:** Resim altlığı için mevcut embed/link/clip temelini koru; reload, missing source ve portable asset paketini tamamla.
- [ ] **CAD-REF-17:** PDF underlay; page, crop, insertion, scale, rotation, contrast/fade ve display/plot durumlarını taşısın.
- [ ] **CAD-REF-18:** PDF vector snap kaynak dönüşümle tutarlı olsun; görüntüden alınan ölçünün doğruluğu kanıtlanmış CAD geometry gibi sunulmasın.
- [ ] **CAD-REF-19:** Kaynak dosya değişikliği, relative path ve offline missing-asset durumu UI/Python/AI'da aynı structured result olsun.
- [ ] **CAD-REF-20:** eTransmit eşdeğeri paket; drawing set, xref, raster/PDF, font, style ve plot kaynaklarını manifest/hash/loss report ile toparlasın.

**Kabul:** Mimarın planı, taşıyıcı proje ve detay dosyaları attach/overlay ile bağlanır. Kaynak reload sonrası bağlı ölçüler kontrollü güncellenir. Teslim paketi temiz başka makinede referans kaybetmeden açılır.

## 16. CAD-IO — DWG, DXF, PDF ve taşınabilir dosya uyumluluğu — P0 karar / P1–P2

### 16.1 DWG stratejisi

DWG desteği şu anda kodda reddediliyor; üretim uyumluluğunda büyük bir boşluk. Autodesk RealDWG, DWG/DXF erişimi için bir SDK sunuyor; ODA Drawings de DWG/DXF ve farklı platform seçenekleri sunuyor. Bu bilgi belirli bir SDK'nın KentOS modeli için otomatik veya tam semantik uyum sağladığını kanıtlamaz. [E11; Autodesk RealDWG](https://aps.autodesk.com/developer/overview/realdwg-api), [ODA Drawings](https://www.opendesign.com/products/drawings), [ODA platformları](https://www.opendesign.com/supported-platforms/drawings)

- [ ] **CAD-IO-01:** DWG seçenekleri için karar matrisi hazırla: RealDWG, ODA, bağımsız reader/converter ve yetkili uzak dönüşüm.
- [ ] **CAD-IO-02:** Matriste Windows/macOS/Linux/ARM64/web, rust FFI, server/headless kullanım, offline, dağıtım hakkı, maliyet, lisans ve bakım yer alsın.
- [ ] **CAD-IO-03:** Platform/sürüm/özellik desteğini sağlayıcının güncel dokümanı ve deneme korpusuyla doğrula; eski tanıtım tablosunu son teknik garanti sayma.
- [ ] **CAD-IO-04:** Ayrı codec adapter sınırı kur; SDK tipleri domain ve shared geometry katmanlarına yayılmasın.
- [ ] **CAD-IO-05:** Önce model-space 2D temel entity/layer/style/block içe alma ve dışa aktarma için dikey dilim geliştir.
- [ ] **CAD-IO-06:** Sonraki kabul; annotation, xref, layout/viewport, dynamic block, constraint ve AEC/proxy içerik için ayrı capability satırları içersin.
- [ ] **CAD-IO-07:** Kaynak sürüm ile yazılacak DWG format sürümü ayrı raporlansın; AutoCAD ürün yılı doğrudan binary format yılı sayılmasın.
- [ ] **CAD-IO-08:** Desteklenmeyen/proxy nesneyi koruma, görünüşünü kullanma, düzenlemeyi engelleme ve basic entity'ye dönüştürme seçeneklerini tasarla.
- [ ] **CAD-IO-09:** Format açma/kaydetme atomik olsun; bozuk dosya açık belgeyi değiştirmesin ve başarısız yazma eski hedefi korusun.
- [ ] **CAD-IO-10:** Kullanıcı seçmedikçe dosya dönüşümü için uzak sunucuya upload yapma; offline yol ve yetki capability'leri açık olsun.

### 16.2 DXF doğruluğu ve round-trip

- [ ] **CAD-IO-11:** Mevcut DXF kayıp raporunu entity/property/line/source ID düzeyine genişlet; sayı ve önem sınıfı taşısın.
- [ ] **CAD-IO-12:** Paper-space entity, LAYOUT ve VIEWPORT verisini gerçek KentOS pafta/view modeline iki yönde aktar.
- [ ] **CAD-IO-13:** Xref block kayıtları/path/transform ve attach/overlay semantiğini koru; mevcut “önce BIND yapın” zorunluluğu kaldırılabilsin.
- [ ] **CAD-IO-14:** Foreign NURBS'in degree/knot/weights/control verisini koru; yalnız KentOS'un kendi curve metadata'sını round-trip etmek yeterli sayılmasın.
- [ ] **CAD-IO-15:** LWPOLYLINE/POLYLINE width, bulge, close, elevation, extrusion/OCS ve block nonuniform scale kayıplarını kapat.
- [ ] **CAD-IO-16:** LTYPE/LIN, STYLE, DIMSTYLE, MLEADERSTYLE, HATCH/PAT, TABLE ve object overrides için kapsam matrisi oluştur.
- [ ] **CAD-IO-17:** ACI/truecolor, ByLayer/ByBlock, transparency, plot/no-plot ve draw order bilgilerini koru.
- [ ] **CAD-IO-18:** Handle/reference/xdata/extension dictionary ve bilinmeyen metadata'nın güvenli opaque preservation stratejisini belirle.
- [ ] **CAD-IO-19:** ASCII/binary ve hedeflenen DXF sürümlerini açık listele; destek olmayan sürüm/encoding için structured refusal veya loss report ver.
- [ ] **CAD-IO-20:** Kendi yazıcını kendi okuyucunla sınamanın yanına bağımsız reader ve AutoCAD'de gerçek görsel/düzenleme kontrolü ekle.

### 16.3 PDF içe alma ve çıktı ilişkisi

- [ ] **CAD-IO-21:** PDF import; page/region, layer, text/vector/fill/raster seçenekleri ve target unit/scale ile çalışsın.
- [ ] **CAD-IO-22:** PDF'nin CAD kaynak doğruluğunu ve nesne ilişkilerini taşımayabileceğini raporla; ölçü/kılavuz/table'ın parçalanmış görünüşünü kaynak semantic entity gibi tahmin etme.
- [ ] **CAD-IO-23:** Bézier/line dönüşümü için açıklanan tolerans ve output geometry tipi sun; raster sayfa OCR/tracing ayrı isteğe bağlı iş olsun.
- [ ] **CAD-IO-24:** PDF text/font ve SHX-as-geometry için açık sonuç/uyarı ver; tahmini text recovery kullanıcı tarafından gözden geçirilebilsin.
- [ ] **CAD-IO-25:** İçe alma planında unit, bbox, target layer/style eşlemesi ve beklenen kayıplar görülsün; işlem tek undo olsun.
- [ ] **CAD-IO-26:** Basic CAD dosya ve paket işlemleri Python/AI/headless'te dosya veya asset handle girdisiyle aynı davranışı sağlasın.

**Kabul:** Gerçek ofis dosyalarından oluşturulmuş korpusta desteklenen 2D entity, birim, layer, block, annotation ve layout kayıpsız korunur. Korunmayan semantik alanlar raporda görünür. AutoCAD'de açılan teslim çizimi ölçülebilir ve düzenlenebilir kalır; yalnız benzer ekran görüntüsü kabul kanıtı değildir.

Araştırma: DXF SPLINE derece/knot/kontrol noktası/ağırlık verileri taşır; PDF import ise kaynak CAD semantiğinin kayıpsız dönüşü değildir. [Autodesk SPLINE DXF](https://help.autodesk.com/cloudhelp/2026/ENU/AutoCAD-DXF/files/GUID-E1F884F8-AA90-4864-A215-3182D47A9C74.htm), [Autodesk PDF import](https://help.autodesk.com/cloudhelp/2026/ENU/AutoCAD-Core/files/GUID-1202CC8A-364F-4E93-8E86-6F476CD83C72.htm)

## 17. CAD-SHEET — Model/kâğıt uzayı, pafta ve baskı — P0 kalıcılık / P1

### 17.1 Mevcut pafta altyapısının tamamlanması

Pafta çekirdeği, düzenleyici, PDF/SVG ve değişken/şablon altyapısı zaten var. Ancak web kitapları IndexedDB'de, masaüstü kitapları cihazın veri klasöründe tutuluyor; `DocumentSnapshotV2` incelenen halinde pafta kitabını taşımıyor. Taşınabilir mimari proje için bu kopukluk öncelikle kapanmalıdır. [E05, E16–E17]

- [x] **CAD-SHEET-01:** Pafta kitabı, page/view/item kimlikleri, bağlı kaynaklar ve asset'leri ana proje snapshot kapsamına al.
- [x] **CAD-SHEET-02:** Cihazdaki mevcut pafta kitapları için project-key/digest ile güvenli migration tasarla; bulunamayan kitabı boş kitapla ezme.
- [x] **CAD-SHEET-03:** `.kcad`, cloud file snapshot, database project, duplicate, import/export ve restore bütün pafta verisini taşısın.
- [x] **CAD-SHEET-04:** `.kpafta` bağımsız template/exchange yolu olarak korunmalı; ana proje kaydı için kullanıcıya ek dosya taşıma zorunluluğu kalmasın. — ✅ **Bitti (13 Ekim; CADP-09, ADR 0251):** kitap `.kcad` şema 44'te, cihazdaki kitaplar projeye alınır ve kaydedilince kenara ayrılır, veritabanı projesinde `project.sheets`; `.kpafta` şablon ve alışveriş dosyası olarak kalır.
- [ ] **CAD-SHEET-05:** Model ve paper-space veri sahipliğini, farklı undo geçmişlerini ve birleştirilmiş script transaction davranışını ADR ile netleştir. — ◐ **Kısmen (ADR 0251 §3):** kitap projenin, kâğıdın geri alması modelinkinden ayrı; birlikte yazan betik işlemi açık.
- [ ] **CAD-SHEET-06:** Ortak pafta `ops`, variables, templates, assets ve export işlemlerini ürün kataloğu/headless host'a aç. — ◐ **Kısmen (ADR 0252):** `cad.sheets.edit` çekirdeğin işlemleriyle, başsız PDF (haritaların yazıları olmadan) ve DXF; kalan: başsız haritaların yazıları ve resim olarak haritalar, asset baytları.
- [ ] **CAD-SHEET-07:** Sheet create/rename/delete/duplicate/order, item create/update/group/align, page setup ve source binding UI/Python/AI'da aynı iş olsun. — ◐ **Kısmen (ADR 0252):** işlemler `cad.sheets.edit` ile aynı çekirdekten; kalan: kullanışlı sarmalayıcılar (şablondan pafta, haritayı nesneye sığdır).
- [ ] **CAD-SHEET-08:** Paper-space çizim/annotation için mevcut pafta item modelinin yeterliliğini değerlendir; CAD entity gerekiyorsa kaynak bağlantısı açık yeni model ekle.

### 17.2 CAD viewport ve çok ölçekli üretim

- [ ] **CAD-SHEET-09:** CAD viewport; drawing/view kaynağı, bbox/center, gerçek scale ratio, rotation, clip, lock ve layer override taşısın.
- [ ] **CAD-SHEET-10:** Model kaynak 1:1 kalsın, kağıt mm ile yönetilsin; unit conversion ile viewport scale tek açık hesap yolu kullansın.
- [ ] **CAD-SHEET-11:** 1:20/1:50/1:100 gibi standart ölçekler ve özel oranlar tam saklansın; scale değeri yalnız UI yuvarlamasıyla bozulmasın.
- [ ] **CAD-SHEET-12:** Rectangle/polygon viewport clip, frame plot/no-plot ve lock/unlock seçenekleri desteklensin.
- [ ] **CAD-SHEET-13:** Viewport içinden model pan/zoom/edit; paper/model aktif scope'u anlaşılır olsun ve API explicit scope alsın.
- [ ] **CAD-SHEET-14:** Named model view ve sheet view bağları; view değişikliği, callout ve annotation scale etkilerini taşısın.
- [ ] **CAD-SHEET-15:** Viewport bazlı layer freeze/style override ve annotative representation seçimi birlikte çalışsın.
- [ ] **CAD-SHEET-16:** Aynı modelin farklı viewportlarda gösterimi kaynağı çoğaltmasın; selection/snap/measure doğru model kimliğini kullansın.
- [ ] **CAD-SHEET-17:** Kat planı, görünüş, kesit ve detay view'larıyla birden çok sayfalı pafta takımı hazırlanabilsin.

### 17.3 Plot profile, PDF ve toplu publish

- [ ] **CAD-SHEET-18:** Named page setup; paper size/orientation/margins, print area, scale, plot profile ve device seçimini taşısın.
- [ ] **CAD-SHEET-19:** CTB/STB semantiği için native plot profile modeli ve import/export stratejisi oluştur; renk ve adlı plot style birbirine karışmasın.
- [ ] **CAD-SHEET-20:** Plot color, grayscale/screening, lineweight, linetype, transparency ve line cap/join override'larını uygula.
- [ ] **CAD-SHEET-21:** Monochrome baskı, raster/PDF altlık, wipeout, text mask ve hatch draw order için karşılaştırmalı paftalar hazırla.
- [ ] **CAD-SHEET-22:** Vektör PDF'de font embedding, searchable/selectable text, eğri/tarama, raster resolution ve sayfa ölçülerini doğrula.
- [ ] **CAD-SHEET-23:** UI print preview, oluşturulan PDF ve headless export aynı kaynak scene/style/scale hesaplarını kullansın.
- [ ] **CAD-SHEET-24:** Gerçek boyutta kontrol işaretleri ekle; 1:50'de 5000 mm çizgi PDF'de 100 mm olsun, ölçüm toleransı açık olsun.
- [ ] **CAD-SHEET-25:** Toplu publish; selected/all sheets, sıra, tek/çok PDF, naming, progress, cancel ve missing-source raporu sun.
- [ ] **CAD-SHEET-26:** Drawing set; pafta numarası/başlığı/revision/status, subset, view index ve antet field'larını yönetebilsin.
- [ ] **CAD-SHEET-27:** Drawing set export/package; xref/font/plot kaynakları ve revision manifestiyle eksiksiz teslim üretsin.
- [ ] **CAD-SHEET-28:** Preflight; missing font/reference, kopuk annotation, viewport scale unlock, dışarı taşan item, düşük raster çözünürlüğü ve sığmayan yazıyı raporlasın.
- [ ] **CAD-SHEET-29:** Export başlamadan kaynak revision sabitlensin; çizim değişirse eski snapshot'tan üretilen dosya bunu provenance'ında söylesin.
- [ ] **CAD-SHEET-30:** Browser fiziksel print dialog'u ve desktop yazıcısı host işlemi olabilir; PDF üretimi her desteklenen otomasyon host'unda tam çalışsın.

**Kabul:** `.kcad` başka cihazda açıldığında paftalar, view'lar, antet ve kaynak bağları gelir. Aynı belge UI ve Python ile 1:50/1:100 viewportlu PDF üretir; geometri, font, sayfa ölçüsü ve baskı sırası karşılaştırmada eşittir.

Araştırma: AutoCAD layout'ları birden çok viewport, viewport ölçeği ve annotation temsili kullanabilir; CTB/STB baskı görünüşünü kontrol eder, sheet set paftaları toplu yönetir. [Layout Viewports](https://help.autodesk.com/view/ACD/2026/ENU/?caas=caas%2Fdocumentation%2FCIV3D%2F2014%2FENU%2FfilesACD%2FGUID-2B5D404A-DCAB-4AF6-A5C1-51593B38F519-htm.html), [Annotation scale](https://help.autodesk.com/cloudhelp/2026/ENU/AutoCAD-LT/files/GUID-4F448A62-A99E-4AB5-AE50-9EAAC0485283.htm), [Scale representations](https://help.autodesk.com/cloudhelp/2026/ENU/AutoCAD-Architecture/files/GUID-4335105B-F200-4FC7-A5B7-EA9205C2BC4E.htm), [Plot Style Tables](https://help.autodesk.com/view/ACDLT/2026/ENU/?caas=caas%2Fdocumentation%2FACD%2F2014%2FENU%2Ffiles%2FGUID-2FD01085-DDD8-49D2-A910-E63EA45A7FED-htm.html), [Sheet Sets](https://help.autodesk.com/view/ACDLT/2026/ENU/?caas=caas%2Fdocumentation%2FACD%2F2014%2FENU%2Ffiles%2FGUID-34D889BC-19AD-4CD1-ADB1-F359D9B515FB-htm.html)

## 18. CAD-PY — Eksiksiz Python API ve betik sistemi — P0/P1/P2

Mevcut `kentos.cad` SDK'sı, üretilmiş wrapper/tipler, `.validate/.plan/.run`, `Document/Connection`, ayrı süreç konsolu ve script editor korunmalıdır. Hedef, yalnız yeni primitive fonksiyonları eklemek değil, **arayüzdeki bütün işlerin aynı SDK'da erişilebilir olmasıdır**. AutoCAD'in resmi programlama belgeleri de çizim ortamı ve veritabanına birden çok API ile erişim yaklaşımını gösterir; KentOS için ana otomasyon dili Python'dur. [E12–E13, E19; Autodesk programlama arayüzleri](https://help.autodesk.com/cloudhelp/2026/ENU/AutoCAD-Customization/files/GUID-E6429154-36DF-4D84-8ABC-9FCA15B66158.htm)

**Kapsam notu:** Tam yüzey şartı yalnız yeni CAD özelliklerine uygulanmamalıdır. Mevcut GIS, processing, stil kütüphanesi, ayarlar, dosya/bulut ve paylaşım arayüzlerinin de Python/AI karşılığı manifestte yer almalıdır. Yeni GIS analiz araçları geliştirmek bu CAD planının kapsamı dışındadır; mevcut işlerin erişilebilirliğini tamamlamak ortak altyapı teslimidir.

### 18.1 Tam ürün erişimi ve ergonomi

- [ ] **CAD-PY-01:** Katalogdan generated Python types, wrappers, `.pyi`, docstring, capability ve örnek üretimini bütün yeni komutlara genişlet.
- [ ] **CAD-PY-02:** Geometri create/edit yerine offset/trim/fillet/hatch/dimension gibi semantik çağrıları doğrudan sun; kullanıcı final geometry hesaplamasın.
- [ ] **CAD-PY-03:** Layer/group CRUD, state/filter/freeze/plot ve active layer yönetimini aç.
- [ ] **CAD-PY-04:** Block definition/instance/attribute CRUD, editor commit ve parametrik değerleri aç.
- [ ] **CAD-PY-05:** Text/dimension/leader/table/linetype/hatch/plot style CRUD ve library işlemlerini aç.
- [ ] **CAD-PY-06:** Proje/user/device/session settings için schema/get/effective/set/reset/import/export destekle; doğru scope zorunlu olsun.
- [ ] **CAD-PY-07:** Selection query/set/clear, view/camera/named view/UCS/drafting/active tool için bağlı session API sun.
- [ ] **CAD-PY-08:** Etkileşimli tool start/step/option/finish/cancel; nokta/entity/mesafe girdilerini tipli işle. Script mouse olayına bağımlı kalmasın.
- [ ] **CAD-PY-09:** Sheet/book/page/viewport/item/variable/template ve publish/export/preflight işlerinin tümünü aç.
- [ ] **CAD-PY-10:** Xref/PDF/image attach/reload/clip/bind ve asset yönetimini aç.
- [ ] **CAD-PY-11:** Mimari wall/opening/door/window/space/grid/stair/tag/schedule komutları SDK'da tipli olsun.
- [ ] **CAD-PY-12:** Constraint/parameter create/edit/solve/diagnose ve ilişkili annotation inspect/reassociate/reset destekle.
- [ ] **CAD-PY-13:** Processing tool/model discover/validate/plan/run, parametre şeması ve result layer/table/artifact yollarını ortak API'ye al.
- [ ] **CAD-PY-14:** I/O; supported formats, import plan, loss report, save/save-as/export/package ve hedef revision bilgisi sunsun.
- [ ] **CAD-PY-15:** Cloud catalog/open/sync/revisions/checkpoints/conflicts/sharing/invitations; mevcut yetki modeliyle tam çalışsın.
- [ ] **CAD-PY-16:** Geometry query için bbox dışında typed expression, geometric predicate, field projection, related entities ve aggregate ölçümler ekle.
- [ ] **CAD-PY-17:** EntityRef ve SubentityRef, closed document, stale revision ve deleted object durumlarını açık tip/hata olarak taşısın.
- [ ] **CAD-PY-18:** Query pagination kararlı revision/snapshot'a bağlı olsun; sayfa arasında değişiklikte sessiz atlama/tekrar yerine stale policy uygula.
- [ ] **CAD-PY-19:** `UNSET`/`None`, enum, tagged union, unit, integer/revision ve exact decimal alanlarını kayıpsız koru.
- [ ] **CAD-PY-20:** Sync headless/native kolay yolu ve async remote/job yolu birlikte sun; çalışan event loop'u gizlice bloke etme.
- [ ] **CAD-PY-21:** `Job` handle status/progress/log/result/cancel/wait ve artifact erişimi sağlasın; kopan istemci işi sahipsiz bırakmasın.
- [ ] **CAD-PY-22:** Batch read/write ve numaralandırma/style apply/selection işlerinde entity başına RPC yerine toplu değişiklik kullan.
- [ ] **CAD-PY-23:** NumPy/Arrow/buffer aktarımını ölçümle ihtiyaç varsa ekle; zero-copy lifetime/mutability garantisini kanıtla.
- [ ] **CAD-PY-24:** Üst seviye ergonomic çağrılar mevcut `kentos.cad` isim düzenine uysun; düşük seviyeli typed `run` kaçış yolu katalogu atlamasın.

### 18.2 Runtime, editör ve dağıtım

- [ ] **CAD-PY-25:** Masaüstü konsolunun mevcut ayrı süreç yaklaşımını koru; desteklenen Python/OS/architecture ve paket ABI matrisini yayımla.
- [ ] **CAD-PY-26:** Kullanıcının sistem Python'una tesadüfen bağlı kalmayan dağıtım tasarla: bundled/managed runtime, wheel ve güncelleme politikası.
- [ ] **CAD-PY-27:** Windows/macOS/Linux için paketleme ve açık çizime transport desteğini doğrula. MCP masaüstü adapter'ının mevcut `cfg(not(unix))` reddi kapanacak platform açığı olarak kaydedilsin. [E14]
- [ ] **CAD-PY-28:** Script çalıştırması; stdout/stderr, traceback, source line, result artifact ve command history'yi tutarlı göstersin.
- [ ] **CAD-PY-29:** Stop/restart/crash/disconnect sırasında açık script grubunu geri al; geç request/result yeni run'a uygulanmasın.
- [ ] **CAD-PY-30:** Başarılı script bir undo grubu olabilir; uzun işlerde tek grup ile kademeli commit seçeneği açık ve belgeli olsun.
- [ ] **CAD-PY-31:** Group/savepoint/nested transaction politikasını belge düzeyinde tutarlılaştır; network await boyunca DB transaction açık kalmasın.
- [ ] **CAD-PY-32:** Script editörü; dosya, seçili kod, history, debugger/traceback navigation, autosave taslak ve command completion iş akışlarını tamamla.
- [ ] **CAD-PY-33:** Tamamlama yeni generated SDK ve kullanıcı import/alias'larını anlasın; property getter veya kullanıcı kodu tamamlama için çalıştırılmasın.
- [ ] **CAD-PY-34:** Ortam/paket yönetimi; lock, offline cache ve kurulu paket görünürlüğü sağlasın; proje açılışında otomatik paket kurma/kod çalıştırma olmasın.
- [ ] **CAD-PY-35:** Script dosya/ağ/secret erişim kapsamları ile CAD command yetkilerini ayır; ayrı Python sürecini güvenlik sandbox'ı gibi tanıtma.

### 18.3 Web Python ve host eşdeğerliği

- [ ] **CAD-PY-36:** Browser Python için worker tabanlı interpreter/transport kararı ver; mevcut Pyodide yönünü bundle, cancel, paket ve lisans açısından ölçerek doğrula.
- [ ] **CAD-PY-37:** Interpreter yalnız Python paneli veya betik gerektiğinde yüklensin; normal 2D açılış maliyetine eklenmesin.
- [ ] **CAD-PY-38:** Pure Python facade; native, desktop-session, browser-worker ve remote transport'ları aynı typed SDK arkasında ayırsın.
- [ ] **CAD-PY-39:** Worker RPC request/run/job ID, typed error, transfer bütçesi, timeout/cancel ve belge revision taşısın.
- [ ] **CAD-PY-40:** Browser stop/worker termination staged changeset'i iptal etsin; UI belgesi yarım iş almasın.
- [ ] **CAD-PY-41:** Native wheel/socket/OS gerektiren paketlerde açık capability hatası ver; izinli remote alternatif varsa kullanıcı seçsin.
- [ ] **CAD-PY-42:** Ortak SDK senaryolarını headless, açık desktop ve web transport üzerinde çalıştır; desteklenen ürün işlerinde sonuç eşit olsun.
- [ ] **CAD-PY-43:** Örnekler; ofis şablonu kur, kat planı çiz, tüm kapı genişliğini değiştir, ölçüleri bağla, mahal çizelgesi üret ve toplu PDF/DXF çıkar.

**Kabul:** Kullanıcı aynı kat planı iş akışını UI, masaüstü Python editörü ve bağımsız Python betiğinde tamamlayabilir. Stil/katman/pafta/PDF için UI'a dönme zorunluluğu yoktur. Web'de desteklenen aynı işler aynı facade üzerinden çalışır; platform sınırı açık biçimde bildirilir.

## 19. CAD-AI — Gerçek AI Surface ve eksiksiz ajan erişimi — P0/P1

`kentos-ui` içinde konuşma, prompt, stream, activity, question ve tool call bileşenleri var. Showcase'ın kendi açıklaması deterministic provider simulator olduğunu söylüyor; ADR de uzak model sağlayıcısının bağlı olmadığını belirtiyor. Dış ajan MCP erişimi mevcut olsa da bu, uygulama içi AI Surface'ın uçtan uca tamamlandığı anlamına gelmez. [E14–E15]

### 19.1 Uygulama içi AI Surface

- [ ] **CAD-AI-01:** Gerçek AI host'u oluştur; mevcut AI bileşenleri command/session/query/job servislerine bağlansın.
- [ ] **CAD-AI-02:** Sağlayıcı adapter arayüzü; request/stream/tool call/question/error/cancel/retry akışını UI/domain'den ayırsın.
- [ ] **CAD-AI-03:** Desktop ve web paneli; yeni konuşma, geçmiş, attachment, stop/retry, tool progress ve sonuç artifact akışını sunsun.
- [ ] **CAD-AI-04:** Aktif drawing/document/session/revision açık bağlam olsun; kullanıcı belge değiştirirse eski istek yeni belgeye yazamasın.
- [ ] **CAD-AI-05:** Kullanıcının yazdığı bir sonraki taslak, iptal, ağ hatası ve kısmi stream durumunda korunsun; geç parçalar request ID ile reddedilsin.
- [ ] **CAD-AI-06:** Provider/endpoint/credential/maliyet ayarları doğru kullanıcı/kurum kapsamına bağlı olsun; sırlar proje dosyasına veya log'a yazılmasın.
- [ ] **CAD-AI-07:** Modelden saklı düşünce isteme/gösterme yerine gösterilebilir işlem özeti, plan ve gerçek tool progress göster.
- [ ] **CAD-AI-08:** Chat yanıtı başarı iddiasını gerçek command/job sonucundan türetsin; simüle ilerleme ve başarılı görünen no-op ürün akışında bulunmasın.
- [ ] **CAD-AI-09:** Soru/eksik parametre `needs_input` şemasından üretilebilsin; birim, hedef belge ve seçim gibi kritik bilgiler uydurulmasın.
- [ ] **CAD-AI-10:** Basit geri alınabilir ve yetkili CAD işleri sürekli modal istemeden çalışsın; geniş silme/overwrite/paylaşım gibi işlemlerde kapsamı belli plan/onay politikası uygulansın.
- [ ] **CAD-AI-11:** Onay yalnız incelenen plan/input/revision/izin kapsamına bağlı olsun; kullanıcı veri veya alıcı değiştirirse eski onay kullanılamasın.
- [ ] **CAD-AI-12:** “Yapılanları göster / geri al / Python olarak al / ilgili nesneleri seç” eylemleri doğrudan gerçek sonuçlara bağlansın.

### 19.2 Tam araç yüzeyi ve çizim bağlamı

- [ ] **CAD-AI-13:** Her yayımlanmış command/operation için açıklama, schema, birim, capability, örnek, error ve plan/result metadata katalogdan üretilsin.
- [ ] **CAD-AI-14:** UI, Python ve AI kapsamında aynı operation ID kullanılsın; ayrı elle tutulan AI command listesi oluşmasın.
- [ ] **CAD-AI-15:** Tool discovery sorgulanabilir ve kategoriye göre yüklenebilir olsun; çok büyük katalog her prompt'a bütünüyle doldurulmasın.
- [ ] **CAD-AI-16:** Drawing summary; units/CRS/UCS, revision, layer/style/block/annotation/xref/sheet ve active scope bilgisi taşısın.
- [ ] **CAD-AI-17:** Selection ve visible bbox, source IDs, locked/hidden durumu ile izinli session context olarak sunulsun.
- [ ] **CAD-AI-18:** Query; typed filters, spatial predicates, projection, limit/cursor/snapshot ve aggregate ölçümler sunsun.
- [ ] **CAD-AI-19:** “Bu duvar” gibi referanslar seçili object ID veya kullanıcı pick akışıyla çözülsün; yakın görsel tahmin tek kaynak olmasın.
- [ ] **CAD-AI-20:** Viewport image; camera transform, world↔screen eşlemesi, device scale, görünür katmanlar ve kaynak revision ile gelsin.
- [ ] **CAD-AI-21:** Görsel bağlam UI okumaya yardım etsin; ölçü/alan/geometry kaynak geometri tool'undan doğrulansın.
- [ ] **CAD-AI-22:** Tool result; source/computed ayrımı, entities/subentities, units/CRS, affected bbox, warnings, losses, artifact ve undo/group bilgisi taşısın.
- [ ] **CAD-AI-23:** Plan; bağımlı ölçü/hatch/mahal/schedule güncellemeleri ve locked/skipped nesneler dahil sonucu göstersin.
- [ ] **CAD-AI-24:** Batch/group çağrıları ile tek doğal dil işi tek tutarlı changeset'e dönüşebilsin; birden çok tool arasında yarım iş kalmasın.
- [ ] **CAD-AI-25:** Uzun iş submit/status/progress/result/cancel ve devam eden iş listesiyle yönetilsin; stream bağlantısı kapanınca sonuç bilinmez olmasın.
- [ ] **CAD-AI-26:** File/cloud/format/plot işlemleri aynı yetkili katalog yolundan yürüsün; ekran dialog'larını otomasyonun tek yolu yapma.
- [ ] **CAD-AI-27:** MCP tools/resources/prompts ayrımını koru; protocol negotiation, transport, schema validation ve hata sonuçlarını desteklenen istemcilerle sınayıp sürümünü sabitle.
- [ ] **CAD-AI-28:** Desktop attach dışında web/session attachment ve yetkili remote headless akışı tasarla; host capability gerçek destekten gelsin.
- [ ] **CAD-AI-29:** Session save/undo/group kısıtlarını açık yayımla; mevcut desktop attach'in kullanıcıya ayrılmış işlemleri için güvenilir host adapter tamamla.

### 19.3 Yetki, veri güveni ve değerlendirme

- [ ] **CAD-AI-30:** AI actor'u kullanıcının/servis hesabının yetkisini devralsın; schema annotation'ları yetkilendirme yerine kullanılmasın.
- [ ] **CAD-AI-31:** Belge/tenant/proje/scope yetkisi her yazma ve commit'te yeniden doğrulansın.
- [ ] **CAD-AI-32:** Layer adı, attribute, metin, PDF notu ve xref içeriği veri olarak ele alınsın; içlerindeki talimatlar izin/alıcı/endpoint'i değiştiremesin.
- [ ] **CAD-AI-33:** Provider'a gönderilen geometri/görüntü/öznitelik kapsamı seçilebilir olsun; hassas alanları çıkarma ve yerel/kurumsal endpoint desteği sun.
- [ ] **CAD-AI-34:** AI-generated Python ayrı yürütme yetkisi taşısın; normal CAD tool yetkisi otomatik dosya/ağ/secret yetkisine yükselmesin.
- [ ] **CAD-AI-35:** Audit; actor, command/version, input digest, source revision, onay kapsamı, result ve undo referansını sır içermeden tutsun.
- [ ] **CAD-AI-36:** Retry, stale revision, missing entity, auth change, timeout ve cancel senaryoları yapılandırılmış düzeltme/yeniden planlama akışına girsin.
- [ ] **CAD-AI-37:** Eval korpusu; Türkçe kısa komut, birimli ölçü, belirsiz seçim, çok belgeli bağlam, kat planı değişikliği ve uzun çok adımlı iş içersin.
- [ ] **CAD-AI-38:** Eval; doğru hedef, doğru birim, doğru command/parametre, geometri doğruluğu, yan etki, undo, süre ve kullanıcı müdahalesini ayrı ölçsün.
- [ ] **CAD-AI-39:** Deterministik tool/host testleri ile modelin doğal dil başarısı ayrı raporlansın; sağlayıcı değişince eval tekrar çalışsın.
- [ ] **CAD-AI-40:** Kullanıcıya açıklanan sonuç ve gerçek belge diff'i karşılaştırılsın; yapılmayan işi “tamamlandı” gösteren yanıt kabul hatası olsun.

**Kabul:** “Seçili kapıları 900 mm yap, etiketlerini ve çizelgesini güncelle, 1:50 paftayı PDF çıkar” isteği gerçek sorgu/plan/command/job zinciriyle tamamlanır. Kullanıcı değişen nesneleri, Python karşılığını ve çıktı dosyasını görebilir. Belirsiz veya yetkisiz girdi hiçbir yanlış belgeye yazmaz.

## 20. CAD-CLI — Yazılı komutlar, script ve dış otomasyon — P1

- [ ] **CAD-CLI-01:** UI command line, headless CLI ve RPC aynı semantic catalog'u kullansın; her biri kendi komut listesini tutmasın.
- [ ] **CAD-CLI-02:** Command completion/help; yöntem, seçenek, birim, aktif scope ve eksik parametreyi şemadan üretsin.
- [ ] **CAD-CLI-03:** Etkileşimli prompt komutu ile bütün parametreleri verilen batch komutu ayrı kullanım yolu olarak desteklensin.
- [ ] **CAD-CLI-04:** Terminal CLI; open/new/query/plan/execute/run-script/save/export/package/preflight işlerini JSON girdi/çıktıyla sunsun.
- [ ] **CAD-CLI-05:** Stable exit code, stderr/stdout ayrımı, structured error, timeout/cancel ve progress kanalı tanımlansın.
- [ ] **CAD-CLI-06:** CLI birim/drawing/UCS/source revision parametreleri Python ve MCP ile eşit olsun.
- [ ] **CAD-CLI-07:** UI command recorder; semantic çağrıları kaydedip düzenlenebilir Python betiğine ve replay JSON'una dönüştürebilsin.
- [ ] **CAD-CLI-08:** Recorder'a birim, scope ve kaynak referanslarını ekle; yalnız ekranda görülen metni veya tıklama koordinatını kaydetme.
- [ ] **CAD-CLI-09:** Kullanıcı-defined command/alias/macro registration; ad çakışması, module lifecycle ve izin kurallarıyla çalışsın.
- [ ] **CAD-CLI-10:** Headless batch; çoklu dosya, selected sheets ve format dönüşümünde failure policy ve dosya başına rapor sunsun.

**Kabul:** Aynı operasyon JSON'u CLI/Python/MCP'den çalıştırılabilir; sonuç ve error code eşittir. Kayıtlı UI işi, fare gerektirmeyen Python betiği olarak tekrar oynatılır.

## 21. CAD-LIFE — Kalıcılık, bulut, iş yaşam döngüsü ve kurtarma — P0/P1

- [ ] **CAD-LIFE-01:** Yeni ilişki/mimari/pafta/constraint kaynakları file snapshot ve database kaynak tanımında eşdeğer saklansın; GIS projection yalnız türev olsun.
- [ ] **CAD-LIFE-02:** Project changes; nesne, tanım, style, relation ve sheet atomikliğini veya güvenilir staged commit protokolünü kapsasın.
- [ ] **CAD-LIFE-03:** Undo/redo ve external sync çakışmasında ilişkili graph tutarlılığı denetlensin; başkasının değişikliğini geri alan yanlış inverse üretilmesin.
- [ ] **CAD-LIFE-04:** Autosave; kaydedilen revision sonrası yapılan işi temiz göstermesin. Pafta ve style dirty durumu proje dirty'sinden kopuk kalmasın.
- [ ] **CAD-LIFE-05:** Crash recovery snapshot'ı bütün kaynak veriyi içersin; yeniden açıldığında ölçü/mahal/sheet bağları korunabilsin.
- [ ] **CAD-LIFE-06:** Offline edit; xref/cache/source revision ve pending job sonuçlarıyla birlikte tutarlı geri bağlansın.
- [ ] **CAD-LIFE-07:** Conflict çözümü; yalnız iki entity değil, duvar–açıklık–mahal ve tanım–instance ilişkilerini de açıklasın.
- [ ] **CAD-LIFE-08:** Project duplicate/checkpoint/restore/import sırasında UUID referanslarını kaynaklara göre yeniden eşle; orphan relation bırakma.
- [ ] **CAD-LIFE-09:** Ağ kesintisi/süreç kapanması/uygulama çıkışı sırasında job durumu ve staged artifact temizliği tanımlansın.
- [ ] **CAD-LIFE-10:** Dosya overwrite/export sonuçları atomik ve provenance'lı olsun; kullanıcı işi kaydettiğini sanarken sadece cihazdaki pafta deposu güncellenmesin.

**Kabul:** Duvarı ve bağlı paftayı değiştiren script yarıda durdurulduğunda tutarlı geri dönüş olur. Kaydedilmiş proje farklı host'ta bütün ilişkileriyle açılır; cloud restore eksik style/xref/sheet bırakmaz.

## 22. CAD-PERF — CAD render kalitesi ve performans — P1

Mevcut performance kayıtları korunmalı. Aşağıdaki rakamlar **yeni başlangıç hedefi önerileridir**, bu oturumda ölçülmüş sonuç veya kesin donanım garantisi değildir. Önce sabit reference hardware, build, viewport, MSAA ve veri korpusu belirlenmeli; mevcut [performans hedefi ADR'sinin](../docs/adr/0005-performance-acceptance-targets.md) durumuyla birleştirilmelidir.

- [ ] **CAD-PERF-01:** Mimari veri setleri oluştur: küçük daire, kat planı, çok katlı drawing set, 100 bin/1 milyon karma entity ve büyük xref planı.
- [ ] **CAD-PERF-02:** Line/arc/ellipse/NURBS, geniş polyline, hatches, rich text, nested blocks ve annotative viewport karışımı gerçek korpusta ölçülsün.
- [ ] **CAD-PERF-03:** Rapor p50/p95/p99, memory peak, first usable frame, job duration ve cancel latency içersin; yalnız ortalama FPS olmasın.
- [ ] **CAD-PERF-04:** Referans makinede rutin plan için pointer/snap/preview p95 hedefini 16.7 ms, büyük çizimde 33.3 ms olarak başlangıçta değerlendir.
- [ ] **CAD-PERF-05:** Rutin pan/zoom 60 FPS, büyük karma çizimde 30 FPS başlangıç hedefi; minimum kabul donanımıyla kalibre et.
- [ ] **CAD-PERF-06:** UI'da 50 ms üzeri blocking hesaplar için worker/job yolunu değerlendirme eşiği belirle; uzun işte görünür progress ve hızlı cancel olsun.
- [ ] **CAD-PERF-07:** Query/snap/index ve styled scene; yalnız değişen entity/alt geometri/dependency bölgelerini invalid etsin.
- [ ] **CAD-PERF-08:** Nested block/xref instance caching ve dizi instancing; kaynak tanım değişimini doğru izlesin.
- [ ] **CAD-PERF-09:** Hatches/text/dash/selection overlays ve label cache için görünürlük/ölçek bazlı bütçe uygula; source geometry aynı kalsın.
- [ ] **CAD-PERF-10:** PDF plot kullanımı için display LOD sadeleştirmesi uygulanmasın veya çıktı kalite politikası açıkça ayrı olsun.
- [ ] **CAD-PERF-11:** HiDPI, MSAA, line joins/caps, subpixel thin lines, dash phase, text metrics ve seçili highlight görsel regresyonla denetlensin.
- [ ] **CAD-PERF-12:** Çok sayıda pafta/viewport export memory peak'i sınırlı olsun; streaming/staged artifact üretimini mevcut PDF çekirdeğine uygun genişlet.
- [ ] **CAD-PERF-13:** Python batch/RPC ve AI query context için aktarım boyutu/süre limitleri ölçülsün; bütün çizim varsayılan prompt payload'ı olmasın.
- [ ] **CAD-PERF-14:** Desktop/web renderer farklılıkları golden'la tek başına değil, scene geometri/ölçü doğruluğuyla birlikte kontrol edilsin.

**Kabul:** Performans raporu sabit donanım ve korpusla yeniden üretilebilir. Kaynak doğruluğunu veya görünür annotation'ı gizleyerek performans hedefi geçmiş sayılmaz.

## 23. CAD-QA — Ortak doğrulama, gerçek dosyalar ve tamlık kapıları — P0/P1

- [ ] **CAD-QA-01:** Operation/options/entity/host kapsam testlerini capability manifestinden üret; schema ile dispatcher uyuşmazsa CI düşsün.
- [ ] **CAD-QA-02:** Shared fixtures; web product, native product, headless, typed Python ve MCP transport üzerinden aynı senaryoyu çalıştırsın.
- [ ] **CAD-QA-03:** UI/session davranışları için gerçek klavye/fare izlerini koru; yalnız doğrudan handler testi UX kabulü sayılmasın.
- [ ] **CAD-QA-04:** Toleranslı geometry karşılaştırmasının gerekçesini açıkla; kaynak aynı algoritma ise gereksiz geniş toleransla sapma gizleme.
- [ ] **CAD-QA-05:** Numeric edge cases; çok küçük/büyük koordinat, nearly parallel/tangent, degenerate/zero-length, self-intersection ve çökük offset içersin.
- [ ] **CAD-QA-06:** İlişki fixture'ları; split/join/redefine/delete/undo/redo/reload ve missing source durumlarını kapsasın.
- [ ] **CAD-QA-07:** `.kcad` migration, corrupt input, unsupported version ve independent writer/reader doğrulaması yeni şemalarla genişletilsin.
- [ ] **CAD-QA-08:** DWG/DXF/PDF korpusu kaynak program, sürüm, birim, beklenen kayıp ve redistribution izniyle kayıtlı olsun.
- [ ] **CAD-QA-09:** Gerçek AutoCAD round-trip; import→edit→export→AutoCAD reopen→measure/plot kontrolüyle doğrulansın; erişim yoksa kabul bekliyor işaretlensin.
- [ ] **CAD-QA-10:** Geometri/field/table değeri bağımsız hesap veya elle doğrulanmış analitik örnekle sınansın; kendi çıktını golden yapıp doğruluk iddia etme.
- [ ] **CAD-QA-11:** Plot testleri; gerçek sayfa boyu, fiziksel ölçek, font, hatch, lineweight, clip ve draw order'ı ayrı doğrulasın.
- [ ] **CAD-QA-12:** Transaction testleri; validate/plan no-write, failed no-write, cancel rollback, tek undo, redo ve stale revision kapsasın.
- [ ] **CAD-QA-13:** Runtime testleri; Python crash/stop, browser worker termination, desktop attachment lost ve multi-document target isolation içersin.
- [ ] **CAD-QA-14:** Yetki/prompt injection/idempotency testleri AI/Python/CLI host adapter'larında ortak çalışsın.
- [ ] **CAD-QA-15:** Kapsam, correctness, performance, visual ve doğal dil eval raporlarını ayrı sun; birinin başarısı diğerini tamamlanmış göstermesin.
- [ ] **CAD-QA-16:** Kabul hataları P0/P1/P2 olarak sınıflansın; veri kaybı, yanlış ölçü/ölçek, yanlış belgeye yazma ve yetki aşımı P0 kapısı olsun.

### 23.1 Zorunlu uçtan uca kabul senaryoları

Her senaryo ilgili UI, yazılı command, Python ve AI yolundan çalıştırılmalıdır. ID/çıktı zamanı gibi kontrollü farklar dışında kaynak belge ve iş sonucu eşdeğer olmalıdır.

✅ **İlk kabul çizimi (13 Ekim; CADP-12, ADR 0252):** K01–K04, K11 ve K12 tek senaryoda (`fixtures/acceptance/v1/room.json`): arayüz (iz `acceptance-room`), yazılı komut ve masaüstünün ajan bağlantısı, Python ve MCP aynı sonucu verir; DXF ve PDF bağımsız okunur. AI panelinin yolu CADP-10'la eklenir; K11'in başsız PDF'inde haritaların yazıları yok (ADR 0252 §3).

| ID | Senaryo | Beklenen kanıt |
|---|---|---|
| K01 | Yerel CAD/mm oluştur; 4000 × 3000 mm kapalı oda çiz | 12 m² alan, 14 m çevre; kaydet/aç sonrası aynı kaynak değerleri |
| K02 | Oda çizgilerini 200 mm ötele; köşeleri buda/birleştir | Semantik parametrelerle aynı geometri; AI/Python final koordinat hesabı yapmaz |
| K03 | 4000 mm duvara ilişkili ölçü ekle; uç 4500 mm olsun | Ölçü 4500; undo duvarı ve ölçüyü 4000'e döndürür |
| K04 | Ölçülen polyline'a vertex ekle; ilgili segmenti böl | Bağ doğru eşlenir veya açık kopuk duruma geçer; yanlış segmente sessiz bağlanmaz |
| K05 | Duvar, 900 mm kapı, mahal ve bağlı çizelge kur; duvarı taşı | Kapı host'u, mahal sınırı/alanı, ölçü ve çizelge tutarlı güncellenir |
| K06 | İki kapının tip/genişliğini değiştir; birini aynala | Numara/attribute/handedness doğru; schedule ve tag bağlı kalır |
| K07 | Parametrik kapı bloğunu büyüt, flip/visibility değiştir | Nested source, instance değerleri ve grip sonuçları eşdeğer |
| K08 | İlişkili dizi aralığını/sayısını değiştir, kaynak nesneyi düzenle | Bütün türevler ve override/reset doğru; tek undo |
| K09 | Yaylı ve delikli sınırda associative hatch; boundary değiştir | Delikler korunur, alan doğru, PDF ve DXF'te biçim eşdeğer |
| K10 | Aynı annotation'ı 1:50 ve 1:100 viewportta göster | Her ikisinde 2.5 mm kâğıt yüksekliği; bağımsız temsil konumu |
| K11 | 5000 mm kaynak çizgiyi 1:50 A3 paftada PDF çıkar | PDF'de 100 mm; sayfa 420 × 297 mm; çıktı toleransı kaydedilmiş |
| K12 | İki paftayı kaydet; başka cihaz/host'ta `.kcad` aç | Pafta kitabı, viewport, antet, kaynak ve asset'ler eksiksiz |
| K13 | Xref attach/overlay/clip; kaynak değiştir ve reload | Doğru nested propagation, snap kimlikleri, bağlı annotation ve missing-source raporu |
| K14 | Foreign NURBS, geniş polyline, nested block ve layout dosyasını round-trip et | Kontrol/knot/width/style/space verisi veya açık kayıp raporu; AutoCAD yeniden açma |
| K15 | Birim mm→cm gösterim değiştir; farklı birimli blok yerleştir | Kaynak şekil beklenmedik ölçeklenmez; gerçek uzunluk/alan korunur |
| K16 | UCS döndür, yerel koordinatlarla çiz; world API'den ölç | Beklenen dünya geometrisi ve aynı ölçü; view rotation ile UCS karışmaz |
| K17 | Script birkaç değişiklikten sonra hata versin veya durdurulsun | Hiç yarım entity/relation/sheet kalmaz; tek grubun doğru rollback'i |
| K18 | Önizlemeden sonra belge/xref/izin değişsin; execute/retry | Structured conflict; eski onayla yeni veri yazılmaz, retry duplicate üretmez |
| K19 | Belge A hedefli AI/Python işi çalışırken B'ye geç | B değişmez; result A'nın source revision ve ID'lerine bağlıdır |
| K20 | Katman adı/PDF metni içine kötü niyetli araç talimatı koy | İzin, endpoint, paylaşım alıcısı ve command kapsamı değişmez |
| K21 | Çizelgedeki net alanı field ile antete bağla; kaynak sınırı değiştir | Aynı geometri kaynaklı field/table/PDF sayısı; computed override ayrımı açık |
| K22 | Standart ve bozuk drawing set için headless preflight/publish/package | Doğru bulgu, progress/cancel, tam dosya manifesti ve temiz makinede açma |
| K23 | Mevcut processing aracı + CAD düzenleme + PDF işini tek betikle yap | Bütün işler katalogdan keşfedilir; result/artifact ve değişiklikler izlenir |
| K24 | Büyük kat planında yoğun hatch/nested xref ile snap/edit/export | Kaynak doğruluk + kalibre edilmiş p95/memory bütçeleri; cancel çalışır |

### 23.2 Tamamlandı sayılma kapısı

- [ ] İşin source contract ve model göçü tamamlandı.
- [ ] Rust hesap, web adapter, native handler ve headless semantic yol aynı fixture'ı geçti.
- [ ] UI tool/command line/Python/AI bütün desteklenen seçenekleri sundu.
- [ ] Read/validate/plan no-write, execute/undo/redo ve fail/cancel senaryoları geçti.
- [ ] `.kcad`/cloud/asset/ilişki kalıcılığı doğrulandı.
- [ ] İlgili DXF/DWG/PDF kayıp ve round-trip raporları hazır.
- [ ] İlgili gerçek UI, görüntü, ölçek ve performans kabulü geçti.
- [ ] Generated catalog/SDK/manifest/help ve örnekler güncel.
- [ ] Desteklenmeyen capability açıklaması ve kalan iş görünür; P0 veri kaybı/doğruluk sorunu yok.

## 24. Geliştirme sırası ve bağımlılıklar

Bu sıralama takvim/sprint sözü değildir. Süre tahmini, DWG/çözücü denemeleri ve gerçek kapsam manifesti tamamlandıktan sonra kişi/kapasite bilgisiyle yapılmalıdır. Öncelik tanımları: **P0** doğruluk/kalıcılık/ortak yüzey temeli, **P1** günlük mimari üretim, **P2** ileri parametrik ve uzman üretkenlik.

| Faz | Somut teslim | Ana işler / bağımlılık | Çıkış kapısı |
|---|---|---|---|
| F0 — Gerçek kapsam | CAD operation manifesti, kaynak–envanter farkları, örnek kat planı korpusu | AUD; mevcut kodun yeniden doğrulanması | Her UI işinin yüzey/host karşılığı görünür |
| F1 — Ortak kaynak ve işlem | Birim eşdeğerliği, semantik komutlar, subentity referansları, changeset/undo, pafta kalıcılığı ve göç | CMD, DATA 01–15/21–25, ANNO 01–06, SHEET 01–08, LIFE | K01–K04/K12/K15/K17–K19 temel kabulü |
| F2 — Günlük CAD güveni | INPUT/EDIT/STYLE, NURBS/width, statik blok, ilişkili ölçü/hatch, çok ölçekli annotation | F1 + DATA/INPUT/EDIT/STYLE/BLOCK/ANNO/HATCH | Kat planı düzeltme ve detay üretimi |
| F3 — Dosya ve pafta teslimi | Çoklu belge/xref, DWG ilk dilimi, DXF layout, CAD viewport/plot/publish | F1; DWG karar/POC F0'da başlar, uygulama F2 ile koordineli | K10–K14/K22; gerçek AutoCAD round-trip |
| F4 — Mimari 2D nesneler | Duvar/açıklık/kapı/pencere/mahal/aks/merdiven/tag/schedule | F1 referans/dependency grafı + F2 annotation/style/block | K05–K06/K21; üretim kat planı |
| F5 — İleri parametrik | Genel constraints, dynamic blocks, associative arrays ileri edit, PDF import | F1 graph/model + F2–F4 temel; solver POC erkenden | K07–K08; ileri düzenleme ve teşhis |
| F6 — Üretim kabulü | Gerçek ofis drawing set, dış format teslimi, tam yüzey ve platform kabulü | Bütün fazlarda oluşan QA/PERF/eval kanıtı | %100 yayımlanmış işlem kapsamı + P0 hata yok |

**Python/AI/CLI ayrı sona bırakılacak faz değildir.** F1'den itibaren her yeni capability'nin SDK ve AI karşılığı aynı teslimde çıkar. Gerçek AI Surface host'u ve Python runtime/transport altyapısı F1–F2'de kurulur. Web Python ve platform paketleme açık kalan işleri F6 öncesi kapatılır. F6, önceden yapılmış işleri bütün ürün olarak doğrular.

Bağımlılık omurgası:

1. `AUD → CMD/DATA`.
2. `Kalıcı subentity referansı → dependency graph → ilişkili ölçü/hatch/field`.
3. `Dependency graph + changeset → duvar–açıklık–mahal → tag/schedule`.
4. `Unit/style/font + annotation scale → CAD viewport → güvenilir plot/publish`.
5. `Çoklu belge + resolver + revision → xref → portable package`.
6. `NURBS/width/block/space kaynak modeli + codec kararı → gerçek dış format uyumu`.
7. `Bütün semantic command'lar → generated Python/CLI/MCP → gerçek AI Surface/eval`.

### 24.1 İlk uygulanacak 12 somut iş

1. CAD operasyon/alt seçenek manifestini çıkar; UI–Python–AI kapsam farklarını üret. — ✅ **Bitti (12 Ekim; CADP-01, ADR 0244).**
2. Pafta kaynakları ile eski envanter farkını düzelt; gerçek var/eksik durumunu belgele. — ✅ **Bitti (12 Ekim; CADP-02, ADR 0247).**
3. Python/MCP `new drawing` birim parametrelerini ve mevcut API birim açıklamalarını eşitle. — ✅ **Bitti (12 Ekim; CADP-03, ADR 0245).**
4. Semantik `offset` dikey dilimini UI/native/web/headless/Python/MCP ile kur. — ✅ **Bitti (12 Ekim; CADP-04, ADR 0246; web ADR 0228 gereği TODOS §0.2'de).**
5. Aynı kalıpla `trim/extend/fillet` ve ilişkili ölçü oluşturmayı tamamla. — ✅ **Bitti (13 Ekim; CADP-05, ADR 0250).**
6. Subentity ID ve reference migration/invalidasyon tasarımını yaz; bağımsız testlerle uygula. — ✅ **Bitti (12 Ekim; CADP-06/07, ADR 0249).**
7. Dimension associativity, annotation monitor ve reassociate ilk dilimini çıkar. — ✅ **Bitti (12 Ekim; CADP-06/07, ADR 0249).**
8. Katman/stil/settings CRUD işlemlerini kataloga al; SDK/MCP kapsamını eşitle. — ✅ **Bitti (12 Ekim; CADP-08, ADR 0248).**
9. Pafta kitabını `.kcad` kaynak kapsamına alma ve cihaz kitaplarından göç yolunu uygula. — ✅ **Bitti (13 Ekim; CADP-09, ADR 0251).**
10. Gerçek AI Surface host ve çizim context/query/plan/execute zincirini bağla.
11. DWG karar matrisi ve gerçek ofis dosyalarından codec POC yap; seçim kararını ayrı ADR'ye yaz.
12. İlk uçtan uca kabul çizimini üret: oda → düzeltme → ölçü/tarama → 1:50 pafta → PDF/DXF → başka host'ta yeniden açma. — ✅ **Bitti (13 Ekim; CADP-12, ADR 0252; UI, yazılı komut, Python ve MCP; AI panelinin yolu CADP-10'la).**

## 25. Uygulama öncesi çözülmesi gereken tasarım kararları

Bu maddeler mevcut planı yazmak için kullanıcı yanıtı gerektiren sorular değildir; geliştirmenin ilgili aşamasında somut deneme/ADR ile kesinleştirilecek seçeneklerdir.

- [ ] **DEC-01:** DWG codec; platform, lisans/dağıtım, offline/remote ve destek seviyesi kararını POC sonrası ver.
- [ ] **DEC-02:** Genel constraints çözücüsü; robust geometry, deterministic result, lisans ve WASM/native performans kararı ver.
- [ ] **DEC-03:** Subentity reference; ID graph, split/join eşlemesi ve blok/xref nested path biçimini seç. — *Kısmen (12 Ekim; CADP-06/07, ADR 0249: yer ve başvuru biçimi seçildi; parçalanma ve birleşmede eşleme ile blok ve xref yolu açık).*
- [ ] **DEC-04:** Style/settings/sheet undo; mevcut ayrık davranış ile yeni composite changeset'i göç planıyla uzlaştır.
- [ ] **DEC-05:** Kağıt uzayı; mevcut sheet item modelinin CAD entity/viewport gereksinimleriyle entegrasyonunu belirle.
- [ ] **DEC-06:** Mimari nesne; kaynak parametre ve generated display sahipliği, cleanup ve dependency güncelleme sırasını belirle.
- [ ] **DEC-07:** SHX/harici font; okuma/render/export/paketleme ve platform fallback çözümünü seç.
- [ ] **DEC-08:** Web Python; interpreter/transport ve durdurma stratejisini benchmark ile seç.
- [ ] **DEC-09:** Masaüstü Python/MCP bağlantısı; Windows named pipe/uygun IPC ve Unix/macOS yaşam döngüsünü eşitle.
- [ ] **DEC-10:** AutoCAD uyumluluk iddiası; hedef dosya sürümleri, entity/property kapsamı ve kabul korpusunu sabitle.
- [ ] **DEC-11:** Ürün kalite kapıları; minimum donanım, performance toleransı ve AI eval hedeflerini ölçüm sonrası kalibre et.
- [ ] **DEC-12:** Uluslararası units/architecture fraction desteği; metrik temel kabul sonrası dağıtım önceliğini belirle.

## 26. Geliştiren ajana uygulama notları

- Önce mevcut dosyayı ve ilgili ADR'yi oku; mevcut CAD/GIS çekirdeklerini veya UI widget'larını gereksiz tekrar yazma.
- Her dilim sözleşme/model → ortak hesap → web/native → semantic host → Python/AI → format/UI → kabul sırasıyla somut tamamlanmalı.
- Generated dosyalar elle düzenlenmemeli; kaynak sözleşme ve üreticiler güncellenmeli.
- Yeni UI özelliği Python/AI karşılığı olmadan “tamamlandı” işaretlenmemeli. Tam ürün tesliminde `requires_ui` yalnız gerçek host sunumu için kullanılmalı.
- Mevcut API'nin hazır geometri yazabilmesini yüksek seviyeli işlemin tamamlandığı kanıtı sayma.
- Eski envanter veya eski ADR sayısına göre özellik silme/yeniden yazma kararı verme; kaynak ve gerçek davranışı doğrula.
- Genel GIS/topoloji/raster işlerini bu mimari CAD planını tamamlamak adına genişletme; CAD/GIS ortak verisini yeni CAD modellerinde koru.
- Sürmekte olan başka geliştirmelerin dosyalarını bu planı uygularken gelişigüzel değiştirme; görev/sahiplik ve dependency sınırını belirle.
- [TODOS.md](../TODOS.md) `CMD-`, `PY-`, `AI-`, `DOM-08` ve CAD maddeleriyle eşleştirme yap; aynı iş iki ayrı backlog'da farklı şekilde tamamlanmış görünmesin.
- Kabul kanıtında **çalıştırılan test**, **kaynakta bulunan test** ve **henüz hedef olan senaryo** ayrı yazılsın.

## 27. Araştırma kaynakları ve ne için kullanıldıkları

Kaynaklar 12 Ekim 2026'da araştırıldı. Öncelik resmi Autodesk ürün/geliştirici belgeleri ve SDK sağlayıcılarının kendi sayfalarıdır. Yardım adresinde eski belge tarihi görünmesi, özelliğin güncel ürün yardımında bulunmadığı anlamına gelmez; kaynak sürümü ayrıca belirtilmiştir. SDK pazarlama ifadeleri ürünümüzün uyumluluk kabulü yerine kullanılmamıştır. Bu belgedeki iş paketleri KentOS'a ait tasarım önerileridir; kaynakların birebir ürün vaadi değildir.

| Kaynak | İncelenen konu |
|---|---|
| [AutoCAD 2026 — Create Objects with Precision and Accuracy](https://help.autodesk.com/cloudhelp/2026/ENU/AutoCAD-OnBoarding/files/ACD_FOUNDATIONS_MAIN6.html) | Koordinat, mesafe, açı kilidi, polar, grid ve snap |
| [AutoCAD 2026 — Associative Dimensions](https://help.autodesk.com/cloudhelp/2026/ENU/AutoCAD-MAC-Core/files/GUID-3559D5D2-AD15-4167-9BD9-EE5FCEB4F8CE.htm) | Kaynak geometriyi izleyen ölçü, kopuk bağ, reassociate |
| [AutoCAD 2026 — Annotation Scale](https://help.autodesk.com/cloudhelp/2026/ENU/AutoCAD-LT/files/GUID-4F448A62-A99E-4AB5-AE50-9EAAC0485283.htm) | Model/paper annotation yüksekliği ve viewport ölçeği |
| [Architecture 2026 — Annotation Scales and Viewports](https://help.autodesk.com/cloudhelp/2026/ENU/AutoCAD-Architecture/files/GUID-4335105B-F200-4FC7-A5B7-EA9205C2BC4E.htm) | Aynı annotation'ın çok ölçekli temsilleri |
| [AutoCAD — Layout Viewports, 2026 yardım girişi](https://help.autodesk.com/view/ACD/2026/ENU/?caas=caas%2Fdocumentation%2FCIV3D%2F2014%2FENU%2FfilesACD%2FGUID-2B5D404A-DCAB-4AF6-A5C1-51593B38F519-htm.html) | Çoklu viewport ve scale/layout |
| [AutoCAD — Plot Style Tables, 2026 yardım girişi](https://help.autodesk.com/view/ACDLT/2026/ENU/?caas=caas%2Fdocumentation%2FACD%2F2014%2FENU%2Ffiles%2FGUID-2FD01085-DDD8-49D2-A910-E63EA45A7FED-htm.html) | CTB/STB |
| [AutoCAD — Plot Styles, 2026 yardım girişi](https://help.autodesk.com/view/ACD/2026/ENU/?caas=caas%2Fdocumentation%2FACDLT%2F2014%2FENU%2Ffiles%2FGUID-929FE8EC-EFE3-43BB-A79F-4FF509A91D5A-htm.html) | Plot görünüşü, kalınlık ve transparency |
| [AutoCAD — Dynamic Blocks, 2026 yardım girişi](https://help.autodesk.com/view/ACD/2026/ENU/?caas=caas%2Fdocumentation%2FCIV3D%2F2014%2FENU%2FfilesACD%2FGUID-3C2FB982-3AF6-437B-987F-4EDF81EA0662-htm.html) | Parametre/action ve instance davranışı |
| [AutoCAD — Xrefs, 2026 yardım girişi](https://help.autodesk.com/view/ACDLT/2026/ENU/?caas=caas%2Fdocumentation%2FACD%2F2014%2FENU%2Ffiles%2FGUID-A987D2FF-45BD-474E-99C1-E6316A42F667-htm.html) | Attach/detach/reload/bind, namespace ve clipping |
| [AutoCAD 2025 — Associative Arrays](https://help.autodesk.com/cloudhelp/2025/ENU/AutoCAD-DidYouKnow/files/GUID-11F6539E-DFA2-4DC9-8146-50C95799675C.htm) | Kaynak ve yerleşimin ilişkili kalması |
| [AutoCAD — Geometric Constraints, 2026 yardım](https://help.autodesk.com/cloudhelp/2026/ENU/AutoCAD-LT-MAC/files/GUID-7D93999A-6CAF-4F54-B7E5-DD2AF29B1564.htm) | 2D geometrik ilişkiler; ürün/platform farkları ayrıca doğrulanmalı |
| [AutoCAD — Constraints ve Dynamic Blocks platform farkları](https://help.autodesk.com/view/ACD/2026/ENU/?caas=caas%2Fdocumentation%2FCIV3D%2F2014%2FENU%2FfilesACD%2FGUID-97C5344F-CF59-4B02-9EA2-22247820D239-htm.html) | AutoCAD ile LT oluşturma/düzenleme sınırları |
| [AutoCAD 2025 — UCS](https://help.autodesk.com/cloudhelp/2025/ENU/AutoCAD-Core/files/GUID-0BE49DA1-B323-4758-B49B-4C497D194C7A.htm) | Yerel çalışma ekseni, origin/orientation ve adlı UCS |
| [Architecture 2026 — Architectural Objects Workflow](https://help.autodesk.com/cloudhelp/2026/ENU/AutoCAD-Architecture/files/GUID-508C88DB-2CB7-4D0C-A60D-8D4CCC7CC5A6.htm) | Duvar, açıklık, kapı, pencere, mahal ve cleanup |
| [Architecture 2025 — Walls](https://help.autodesk.com/cloudhelp/2025/ENU/AutoCAD-Architecture/files/GUID-E7595DBB-BB79-4006-ADE5-17E4BD676110.htm) | Component/style ve hosted opening davranışı |
| [Architecture toolset ürün sayfası](https://www.autodesk.com/in/products/autocad/included-toolsets/autocad-architecture) | Genel mimari araç kapsamı; pazarlama hız oranları plana kabul hedefi yapılmadı |
| [AutoCAD 2026 — Supported Programming Interfaces](https://help.autodesk.com/cloudhelp/2026/ENU/AutoCAD-Customization/files/GUID-E6429154-36DF-4D84-8ABC-9FCA15B66158.htm) | Programlama yüzeyi ve platform matrisi |
| [AutoCAD 2026 Developer and ObjectARX Help](https://help.autodesk.com/view/OARX/2026/ENU/) | Resmi API ve DXF başvuru merkezi |
| [AutoCAD 2026 DXF — SPLINE](https://help.autodesk.com/cloudhelp/2026/ENU/AutoCAD-DXF/files/GUID-E1F884F8-AA90-4864-A215-3182D47A9C74.htm) | Degree/knot/control/weight verisi |
| [AutoCAD 2026 — Importing PDF Files](https://help.autodesk.com/cloudhelp/2026/ENU/AutoCAD-Core/files/GUID-1202CC8A-364F-4E93-8E86-6F476CD83C72.htm) | PDF vector/text/raster dönüşümü ve semantik kayıplar |
| [AutoCAD 2025 — Import PDF Dialog](https://help.autodesk.com/cloudhelp/2025/ENU/AutoCAD-Core/files/GUID-F94D7059-5B4A-4396-97FE-765B1D037D56.htm) | Page/layer/object conversion seçenekleri |
| [AutoCAD — Sheet Sets, 2026 yardım girişi](https://help.autodesk.com/view/ACDLT/2026/ENU/?caas=caas%2Fdocumentation%2FACD%2F2014%2FENU%2Ffiles%2FGUID-34D889BC-19AD-4CD1-ADB1-F359D9B515FB-htm.html) | Drawing set ve toplu pafta yönetimi |
| [Architecture 2026 — Sheet Set Fields](https://help.autodesk.com/cloudhelp/2026/ENU/AutoCAD-Architecture/files/GUID-79BE068E-7B22-4C77-91C6-16C77E3BCE8A.htm) | Sheet/view/title/scale kaynaklı field ve callout |
| [AutoCAD 2025 — Package Drawings for Transmittal](https://help.autodesk.com/cloudhelp/2025/ENU/AutoCAD-DidYouKnow/files/GUID-DD451786-5D5A-42A5-92D5-943CE9D66016.htm) | Drawing dependency ve teslim paketi |
| [AutoCAD Mac 2026 — Package Drawing](https://help.autodesk.com/cloudhelp/2026/ENU/AutoCAD-MAC-Core/files/GUID-DE1AF506-0197-4258-B0F4-0A6C38F794FC.htm) | Paket kaynakları ve missing font/ref raporu |
| [Autodesk RealDWG](https://aps.autodesk.com/developer/overview/realdwg-api) | DWG/DXF SDK seçeneği |
| [ODA Drawings SDK](https://www.opendesign.com/products/drawings) | DWG/DXF/native/web SDK seçenekleri |
| [ODA Drawings supported platforms](https://www.opendesign.com/supported-platforms/drawings) | Sağlayıcı platform/compiler matrisi |

**Kapanış ölçütü:** Profesyonel bir 2D mimari projenin çizimden revizyona ve dosya/pafta teslimine kadar tamamı KentOS içinde yapılabilmeli; aynı ürün işleri komut, Python ve AI yolundan tekrar üretilebilmelidir. Tamamlanma, bu belgede tanımlanan kapsam ve kabul kanıtlarıyla gösterilir.
