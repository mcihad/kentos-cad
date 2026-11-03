# KentOS Road — 3D yol projelendirme, arazi, nokta bulutu ve kazı/dolgu

Araştırma ve kaynak inceleme tarihi: **13 Ekim 2026**, gözden geçirme **2 Kasım 2026**. Belge sürümü: **taslak 0.2**. Bu belge mevcut CAD/GIS'in üzerine kurulacak yol mühendisliği ürününün araştırma, mimari ve uygulama planıdır. Uygulama yapılmış, GPU benchmark'ı alınmış veya rakip ürünlerde karşılaştırmalı test koşulmuş değildir.

**2 Kasım 2026 gözden geçirmesi (sahibin kararları):**

- **Ayrı modül ve program.** Mimari CAD, Harita CAD ve GIS gibi yol da ayrı bir KentOS modülüdür: **KentOS Yol** (`kentos-yol`, `--modul yol`). Kendi şeridi, kendi komut kümesi, kendi pencere kimliği ve Launcher'da şimdiki “Yakında” kartının açılmasıyla gelir ([ADR 0800](../docs/adr/0800-kentos-launcher.md), [ADR 0801](../docs/adr/0801-kentos-modules-and-their-programs.md), [ADR 0802](../docs/adr/0802-strict-module-separation.md)). Yol projesi bir CAD çizimi değil, kendi kaynak grafı olan projedir; modülün komutları başka modülde aranmaz ve çalışmaz (§2a).
- **Ortak çekirdek.** Hesap (arazi yüzeyi, güzergâh, istasyon, profil, kesit, hacim) modülün arayüzünden ayrı, ortak Rust çekirdeğinde kurulur. İleride Harita CAD'deki imar planı çizimi (imar yolu ekseni, km, enkesit, kot) ve Mimari'nin vaziyet planı aynı hesabı kullanabilir; çekirdek yol modülünün ekranlarına bağlanmaz.
- **Çalışma düzeni.** Her madde için önce güncel internet araştırması yapılır (üreticinin resmî belgesi önce), madde aşamalara bölünür, her aşamadan sonra masaüstünün gerçek ekran görüntüsü alınıp incelenir ve sahibe gönderilir. Alt ajan yoktur; işi tek ajan yapar. Bu dalda ADR numaraları 0900'den başlar; `.kcad` şema numarası birleştirmede ana dalın son numarasına göre yeniden verilir.
- **Standart kaynağı.** KGM'nin *Karayolu Tasarım El Kitabı* (KTEK) 2022 baskısı ve *Karayolları Teknik Şartnamesi* 2023 baskısı ücretli basılı yayınlardır ([KGM Yayınlar](https://www.kgm.gov.tr/Sayfalar/KGM/SiteTr/Yayinlar/Yayinlar.aspx), 2 Kasım 2026'da görüldü); serbestçe indirilebilen resmî metin bulunamadı. İkincil ders notu ve web tablolarından sınır değeri alınmaz. Standart profili (hız–yarıçap, dever, klotoid A aralıkları, görüş mesafeleri, kurp genişlemesi, şev eğimleri) resmî kitaptan, sahibin sağladığı kopyadan ve onayıyla girilir; o zamana dek denetimler “standart profili yok” der, uydurma sayı kullanmaz (ROAD-DEC-01).

**Hedef:** Ölçme ve nokta bulutundan doğrulanmış arazi üret; yatay/düşey güzergâh, dever, tip enkesit ve koridor tasarla; kazı/dolgu, malzeme ve taşıma hesabını izlenebilir yöntemlerle yap; aynı kaynaktan 2D plan/profil/kesit, gerçek 3D sahne, aplikasyon ve teslim dosyaları üret. Görselleştirme ve uygun büyük veri hesapları GPU'dan yararlanacak; mühendislik doğruluğu ayrıca ölçülecek.

**Belge sahipliği:** Ortak 3D sahne, FP32 hassasiyet, kamera, streaming, GPU kaynakları, Digital Twin ve simülasyon altyapısının tek planı [3d.md](3d.md)'dir. Buradaki arazi/yol/hacim modeli o altyapıyı kullanır. [cad.md](cad.md) CAD komut/pafta temelini, [python.md](python.md) isteğe bağlı `.kek`/Python ortamlarını, [form.md](form.md) yetkili saha formunu tanımlar. Formun ilk özellik olarak çıkarılabileceği önceki sıra korunur; bu araştırma CAD/GIS geliştirmesini durduran bir 3D göç talimatı değildir.

`ROAD-*` iş kimlikleri bu belgeye aittir. `[ ]` yapılacak iş, **mevcut** kaynakta görülen temel, **öneri** KentOS kararı, **doğrulanmadı** kamuya açık belgeyle veya bu oturumla kanıtlanamayan noktadır. GPU/FPS ve hata eşikleri aşağıda başlangıç hedefi veya araştırma kapısıdır; ölçülmüş sonuç değildir.

## 1. Ürün kapsamı ve tamamlanma anlamı

- Yol mühendisliği nesnesi, genişletilmiş bir polyline veya görsel mesh ile sınırlanmaz: analitik güzergâh, profil, istasyon, kesit şablonu, hedefler ve kaynak bağımlılıkları kalıcıdır.
- Arazi TIN/DEM'i, render mesh'i ve fizik/simülasyon ağı ayrı kaynaklardır. Her birinin doğruluk, çözünürlük, revision ve üretim yöntemi bellidir.
- Aynı tasarım UI, komut, Python ve AI/MCP ile aynı parametre, hata, plan ve sonuç semantiği üzerinden yapılır. İşlem hesabını API kullanıcısına yeniden yazdırma.
- Uygulama plan, boykesit, enkesit ve perspektif 3D görünüşlerini aynı mühendislik kaynağına bağlar. Bir görünüşte değişiklik ötekilerin kaynağını tutarlı yeniler.
- Büyük veri diskte/ağda parçalı, RAM ve GPU'da bütçeli tutulur. Bütün nokta bulutunun yüklenmesi ilk görüntü veya tek kesit için önkoşul değildir.
- Düşey datum, metre/ayak, yatay CRS, grid/ground uzaklık, ölçme epoch'u ve kaynak kalite bilgisi hesap sözleşmesinde bulunur.
- Tam GPU desteği; render, paralel veri hazırlama ve uygun analiz çekirdeklerinin donanım hızlandırmalı yollarını kapsar. Topolojik karar ve kesinleşmiş hakediş hesabının doğruluğu için gerektiğinde robust CPU/FP64 doğrulama aynı ürüne dahildir.
- Önizleme/LOD/seyreltilmiş veri ile hesaplanan tahmini sonuç etiketlenir; kaynak çözünürlükte doğrulanmış miktarla karışmaz.
- Donanım veya veri yeteneği yoksa hangi işin desteklenmediği açıklanır; GPU'ya gönderilmiş olması bir işlemin doğru veya hızlı olduğunun kanıtı değildir.
- İlk odak karayoludur. Demiryolu, tünel, köprü ve ileri kavşak bileşenleri sürümlü genişletmelerdir; bütün uzman disiplinlerini ilk dilime zorunlu bağlama.
- **Dinamik revizyon temel davranıştır.** Netpro 12'nin ana yeniliği gibi ([Netcad NETPRO](https://wiki.netcad.com.tr/display/HELP/NETPRO)) arazi, yatay güzergâh (eleman, yön, ripaj, km eşitliği), düşey güzergâh, kesit, şevli kotlu plan, sıyırma/diş kazısı ve kübaj kuralları değişince bağlı her şey yeniden üretilir. KentOS'ta bu bağımlılık grafı (§5) ve etkilenen km aralığıyla yapılır; onaylanmış miktar sessizce değişmez, yeni hesap ve fark üretilir (ROAD-VOL-17).
- **2B önce, 3B sonra.** İlk aşamalar plan, boykesit ve enkesit görünüşleriyle tamamlanır; gerçek 3B sahne [3d.md](3d.md)'nin kamerası ve derinlik hattı geldiğinde bağlanır. 3B'nin olmaması yüzey, güzergâh, kesit ve kübajın doğruluğunu ve kabulünü bekletmez.

## 2. KentOS’ta mevcut temel ve gerçek boşluklar

| Kanıt | Kaynak ve gözlenen temel | Planın genişleteceği yer |
|---|---|---|
| R-E01 | [Plan yolu ADR](../docs/adr/0198-plan-roads.md), [yol çekirdeği](../crates/shared/geometry-core/src/ops/road.rs), [araç](../crates/native/interaction/src/plan_road.rs) | 2D yol/kaldırım/refüj alanı ve kavşak temizliği var; kalıcı analitik 3D yol projesi farklı modeldir |
| R-E02 | [Delaunay](../crates/shared/geometry-core/src/geom/delaunay.rs), [robust predicates](../crates/shared/geometry-core/src/predicates.rs) | Mevcut CPU Delaunay ve exact-sign kararları korunur; constrained breakline, yüzey modeli ve büyük veri üretimi genişler |
| R-E03 | [Dolgu üçgenleme](../crates/shared/geometry-core/src/triangulate.rs) | Ear-clipping polygon fill vardır; mühendislik TIN'i veya 3D hacim mesher'ı olarak sunulmaz |
| R-E04 | [İnterpolasyon ADR](../docs/adr/0232-interpolation-and-density.md), [TIN'den raster](../crates/native/processing/src/builtin/interpolation/tools.rs), [nokta raster hesabı](../crates/shared/raster/src/from_points.rs) | Noktalardan yüzey örnekleme ve çapraz doğrulama temeli var; persistent TIN/breakline/volume kaynak lifecycle'i gerekir |
| R-E05 | [Nokta bulutu ADR](../docs/adr/0207-point-clouds-and-large-data.md), [ortak okuyucu](../crates/shared/pointcloud/src/lib.rs) | LAS/LAZ/COPC/text, parçalı okuma, indeks ve VPC temeli var |
| R-E06 | [Nokta node hazırlama](../crates/shared/pointcloud/src/nodes.rs), [zemin filtresi](../crates/shared/pointcloud/src/ops/ground.rs) | Node merkezine göre FP32 render noktaları; CPU tabanlı iki geçişli zemin sınıflandırma vardır |
| R-E07 | [Raster arazi](../crates/shared/raster/src/terrain.rs), [yüzey araçları](../crates/native/processing/src/builtin/surface/tools.rs) | Eğim/aspect/hillshade ve raster worker temelini GPU/3D adaptörleriyle genişlet |
| R-E08 | [FP32 hassasiyet](../crates/render/wgpu/src/precision.rs), [2D kamera](../crates/render/wgpu/src/camera.rs), [renderer](../crates/render/wgpu/src/renderer.rs) | Dünya FP64, GPU hi/lo FP32 mevcut; kamera 2D, renderer depth buffer kullanmıyor. Gerçek 3D kamera/depth/mesh pipeline gerekir |
| R-E09 | [Sahne](../crates/render/wgpu/src/scene.rs), [layout](../crates/render/wgpu/src/layout.rs) | 2D fill/segment/marker ve origin-relative cache; 3D vertex/normal/terrain node yeni sözleşmelerdir |
| R-E10 | [Mesh/çok boyutlu ADR](../docs/adr/0243-mesh-and-multidimensional-data.md), [mesh okuyucu](../crates/shared/formats/src/multidim/mesh.rs) | UGRID/2DM ve zamanlı veri temeli var; mesh görünüşü sanal rasterdır, tam 3D fizik motoru değildir |
| R-E11 | [LRS ADR](../docs/adr/0224-linear-referencing.md), [grid/yer/elipsoid ADR](../docs/adr/0171-ground-ellipsoid-plane.md) | Measure/route ve ölçme politikaları var; yol chainage ve station equation bunlarla açık eşlenir |
| R-E12 | [Processing tipleri](../crates/native/processing/src/types.rs), [runner](../crates/native/processing/src/runner.rs) | Salt okunur girdi → changeset/worker modeli; uzun civil/GPU job ve artifact lifecycle'i eklenir |
| R-E13 | [Sözleşme kataloğu](../crates/shared/contracts/src/catalog.rs), [headless](../crates/native/headless/src/dispatch.rs) | UI/SDK/AI işlemleri tek katalog yaklaşımına eklenir |
| R-E14 | [Genel yol haritası](../TODOS.md) `CIVIL-01…14` | Arazi, aks/profil, koridor, hacim ve LandXML/IFC işleri açık; bu belge ayrıntılı karşılığıdır |

| R-E15 | [Launcher](../apps/launcher/src/registry.rs), [modüller](../crates/native/launch/src/module.rs), [modül yüzeyleri](../apps/desktop/src) (ADR 0800–0802) | `kentos-yol` kartı “Yakında” durumunda; `Module` yalnız Mimari, Harita, GIS. Yol modülü buraya dördüncü modül olarak eklenir |
| R-E16 | [Km yaz](../docs/adr/0189-stationing.md), [orta hat](../docs/adr/0190-centerline.md), [nokta hesaplayıcının km'si](../docs/adr/0188-point-calculator-extras.md) | Çizgi boyunca km ve sapma var; analitik aks ve km eşitliğiyle değil, çizilmiş yolla çalışır. Aks gelince bu araçlar aksı da kaynak olarak alır |
| R-E17 | [Uzaklık ve maliyet](../docs/adr/0236-distance-and-cost.md), [hidroloji](../docs/adr/0235-hydrology.md) | Netpro'nun Koridor Analizi'nin karşılığı olacak ön güzergâh (en düşük maliyetli yol, yasak alanlar) ve drenajın raster temeli var (ROAD-ALIGN-13, ROAD-DRAIN-03) |
| R-E18 | [Ağ dengelemesi](../docs/adr/0203-network-adjustment.md), [saha verisi](../docs/adr/0169-field-data.md), [zemin–düzlem](../docs/adr/0171-ground-ellipsoid-plane.md), [geoit](../docs/adr/0631-geoid-velocity-and-vertical-statements.md) | Karne, GSI/SDR/JobXML, GNSS, zemin ve düzlem uzunluğu, geoit ızgarası ve cihaza gönderme var; aplikasyon ve ölçüm kaynağı bunlara bağlanır |

Bu envanter ürün dosyalarına bakılarak hazırlanmıştır. Mühendislik koridorunun, doğrulanmış kazı/dolgu sisteminin veya 3D viewport'un bugün tamam olduğu sonucuna varılmaz. Geliştirmeye başlamadan önce devam eden ajanın son değişiklikleriyle kaynak envanteri yeniden eşleştirilmelidir.

## 2a. ROAD-MOD — KentOS Yol modülü, programı ve çalışma yüzeyi — P0

Yol, Mimari ve Harita'dan ayrı bir iştir; kullanıcı yol programını açtığında yalnız yol işinin araçlarını görür. Modülün sınırı ADR 0802'nin kurallarıyla aynıdır: komut arama, komut satırı, kısayollar, F1, menüler, paneller ve ayar bölümleri modüle göre süzülür; başka modülün komutu `not_offered` ile reddedilir. Çekirdek ortak kalır (§1, 2 Kasım kararı).

**Netpro ve OpenRoads'dan alınan çalışma yüzeyi.** Netpro tek pencereden yönetilen editörlerle (Güzergah Editörü, profil, Tip Kesit Editörü, kesit editörü, otokorkuluk ve duvar editörleri) ve projenin dosyalarını (*.KTB, *.KSE, *.KSP, *.PRJ, *.EKT) gösteren proje ekranıyla çalışır; editör açıkken çizim sürer. OpenRoads, çalışma alanında araziyi, geometriyi, şablonu ve koridoru “sivil veri” olarak tutar ve yapılan değişiklik bağlı modele yansır. KentOS'ta karşılığı: proje ağacı (ayrı dosyalar değil, tek projenin kalıcı kaynakları), yan yana plan–boykesit–enkesit görünüşleri ve seçili kaynağın düzenleyicisi.

- [x] **ROAD-MOD-01:** `Module::Road` (`yol`), `kentos-yol` program bağlantısı (`scripts/desktop/module_links.py`), kendi pencere kimliği, `.desktop` kaydı, ajan soketi (`masaustu-yol.sock`), yerleşim dosyası (`yerlesim-yol.json`); Launcher'da `kentos-yol` kartı “Yakında”dan çıkar ve modülü başlatır.
- [~] **ROAD-MOD-02:** Yol projesinin türü: yeni proje sihirbazında koordinat sistemi (yerel değil; il ve dilim), düşey datum (ortometrik, geoit modeli), birim (metre) ve standart profili sorulur. `.kcad`'de projenin işi `road` (ADR 0800'in `cadWork`'ü gibi opt-in alan); başka modülün dosyası o modülün programına devredilir.
- [~] **ROAD-MOD-03:** Şerit: **Dosya, Arazi, Güzergâh, Boykesit, Enkesit, Koridor, Kübaj, Aplikasyon, Çıktı, Görünüm**. Her düğme 1440 px'te yazılı, 1100 px'te kademeli daralır; bir komut ana sekmelerde bir kez geçer; seçili kaynağa göre bağlamsal sekme (Yüzey, Aks, Profil, Tip kesit, Koridor) açılır (ADR 0622'nin kuralları; `catalog/v1/ribbon.json`'da modülün `modules`'ü).
- [ ] **ROAD-MOD-04:** Proje ağacı paneli: Arazi yüzeyleri, Ölçüm ve bulutlar, Güzergâhlar (her birinin altında profilleri, kesit hattı takımları, dever çizelgesi), Tip kesitler, Koridorlar, Kübaj hesapları, Paftalar. Her satır kaynağın durumunu söyler: güncel, eskimiş (kaynağı değişti), çözülemeyen aralık var, onaylı. Satırdan seç, göster, düzenleyiciyi aç, yeniden üret.
- [ ] **ROAD-MOD-05:** Görünüşler: plan (ana çizim alanı), boykesit ve enkesit görünüşü aynı pencerede bölünmüş görünüm olarak (ADR 0523/0637'nin görünüm yerleri); km kaydırıcısı seçili istasyonu üç görünüşte birlikte gösterir; boykesitte düşey abartma yalnız görünüştür (ROAD-PROFILE-06).
- [ ] **ROAD-MOD-06:** Dinamik revizyonun kullanıcıya görünen yüzü: değişiklikten sonra etkilenen km aralıkları, kesitler, şevli kotlu plan ve kübaj “eskidi” olur; “Yeniden üret” tek geri alma adımıdır; isteğe bağlı “Kendiliğinden yeniden üret” (Netpro 12'deki seçenek gibi) yalnız taslak hesaplarda çalışır, onaylı kübajı yeni hesap ve fark olmadan değiştirmez.
- [ ] **ROAD-MOD-07:** Modülün komutları `road.*` ailesindedir (§21); her biri ürün komutu, Python (`kentos.road`) ve MCP aracıdır; manifestte (`catalog/v1/operations.json`) `semantic` düzeyindedir (CLAUDE.md §9.2).
- [ ] **ROAD-MOD-08:** Ortak çekirdek sınırı (sahibin 2 Kasım kararı: ayrı olabilecek iş ayrı crate'tir): yol hesabı kendi crate'i `crates/shared/road` (`kentos-road`; yüzey modeli, güzergâh, istasyon, profil, kesit, hacim), `kentos-geometry-core`'un öngörüleri ve Delaunay'ı üstünde; ürün komutları `crates/native/application`'da onu çağırır, WASM bağlayıcısı gerekirse ayrı `crates/wasm/road-wasm`. Crate DOM, Iced ve modül bilmez; Harita CAD (imar yolu ekseni ve kesiti) ile Mimari (vaziyet planı kotu) aynı çekirdeği komut üzerinden kullanabilir. Modülün arayüzü `apps/desktop` içinde kendi klasöründe (`apps/desktop/src/road/`) durur.

**Kabul:** `kentos-yol` açılır, yalnız yol şeridini ve komutlarını sunar; Mimari ve Harita'nın komutları aramada çıkmaz; Launcher kartı programı başlatır; boş yol projesi kaydedilip açılır; dar (1100×650) ve geniş pencerede, açık ve koyu temada şerit yazıları kesilmez.

## 3. Netcad, Bentley ve diğer yazılımlar: doğrulanan yaklaşım

| Sistem | Resmi belgede görülen yaklaşım | KentOS için teknik karşılık |
|---|---|---|
| Netcad Netpro | Yatay/düşey güzergâh, geçiş eğrileri, en/boykesit, farklı kübaj yöntemleri, kabarma/sıkışma ve dinamik proje çıktıları | Türkiye iş akışı; sürümlü standart, güzergâh bağımlılıkları, hesap yöntemi ve rapor provenance'ı |
| Bentley MicroStation | CAD/3D üretim ve altyapı modelleme platformu | Ortak sahne, model/reference, çizim/pafta ve kaynak organizasyonu |
| Bentley OpenRoads Designer | MicroStation temeli üzerinde survey/terrain, template corridor, dever, kesit ve triangulated/end-area earthworks | Analitik model → şablon/target → koridor → ilişkili kesit/rapor/pafta zinciri |
| Autodesk Civil 3D | Corridor surface ve material/sample-line tabanlı miktarlar; sectional AEA/prismoidal/composite seçenekleri | Görsel mesh'ten ayrı surface ve material model; hesap yöntemini açık seçme |
| 12d Model | Yatay/düşey super alignment, string/template modifiers ve bağlı tasarımın yeniden çözülmesi | Station aralıklı parametreler, referanslı geometri ve lokal recalculation |
| Trimble Business Center | Corridor/site mass-haul; malzeme, taşıma yolu, engel, borrow/waste kapasite ve maliyetleri | Brükner grafiğinden ileri, kısıtlı malzeme taşıma ve saha/as-built iş akışı |

Kaynaklar: [Netpro](https://www.netcad.com/tr/urunler/netpro), [MicroStation](https://www.bentley.com/products/microstation), [OpenRoads](https://www.bentley.com/en/products/openroads-designer/), [Civil 3D miktar eğitimi](https://www.autodesk.com/learn/ondemand/module/create-preliminary-road-quantities-and-schedules-in-civil-3d), [Civil 3D hacim yöntemleri](https://help.autodesk.com/cloudhelp/2018/ENU/Civil3D-UserGuide/files/GUID-1E3A9CD9-DFD6-4455-9EB1-D8E46721A0B4.htm), [12d alignment](https://12d.com/product/12d_model_alignment.html), [Trimble mass-haul](https://help.fieldsystems.trimble.com/tbc/10433.htm).

**Önemli araştırma sınırı:** Netpro ve MicroStation/OpenRoads'un kapalı render motorlarında hangi shader hesabının FP32/FP64 olduğu, bütün hacim/üçgenleme işlemlerinin GPU'da çalıştığı veya private bellek/cache düzeni bu kaynaklardan doğrulanmadı. Pazarlama performans ifadesinden algoritma çıkarılmaz. KentOS için aşağıdaki GPU mimarisi kendi önerimizdir. Bentley'nin açık [iTwin.js display belgesi](https://www.itwinjs.org/learning/display/) backend tile üretimi ve frontend tile renderer ayrımını doğrular; iTwin.js renderer'ı MicroStation'ın renderer'ıyla aynı kabul edilmez.

### 3.1 2 Kasım 2026'da doğrulananlar

**Netcad Netpro 12** (resmî [genel özellikler](https://wiki.netcad.com.tr/pages/viewpage.action?pageId=275350930), son düzenleme 26 Eylül 2025, ve [yenilikler sayfası](https://wiki.netcad.com.tr/display/HELP/NETPRO)):

- Kapsam yalnız karayolu değil: orman yolu, kent geçişi, demiryolu, kavşak, tünel, alt geçit, havaalanı, liman; gölet, baraj, taşkın koruma, dere ıslahı; açık maden ve hafriyat. KentOS'un ilk teslimi karayoludur, kaynak modeli bu kapsamı kapatmayacak biçimde genel koridor olarak kurulur.
- Yatay güzergâh: doğru, yay, spiral; klotoid, ters kurp ve ove spiral; paralel güzergâh; RL değerlerine göre sabitlenen yay ve doğrular; kesişen en kesit kollarının dar kurpta kırpılması.
- Kesit alma: güzergâha dik ya da km esaslı; **sabit aralıklarla** ya da **üçgen kenarlarını kesen noktalarla**; kurp ve spiralde sıklaştırma; kesit km'lerinde kırmızı kotun düşey güzergâhtan hesabı.
- Düşey güzergâh: parabolik ve dairesel düşey kurp; kurp boyu bisektris, açık/kapalı kurp katsayısı, Duruş Görüş Mesafesi (DGM) ve Geçiş Görüş Mesafesi (GGM) ölçütleriyle; km ve offset sorgusu.
- Kübaj yöntemleri adlı kurallardır: **Yol–Zemin, Formasyon–Zemin, Yol–Bitkisel Toprak, Formasyon–Bitkisel Toprak, Sıyırma Kazısı**; kabarma ve sıkışma katsayıları; Excel'e aktarma. Sıyırma ve **diş kazısı** kesitleri ayrı tanımlanır.
- Şevli kotlu plan: şev taramaları kullanıcı aralığıyla, yarma/dolgu renklendirmesi, eğim ve akış yönü; “3 boyutlu kırmızı kot hesabı” ile en az kazı/dolguyu veren düşey güzergâh önerisi.
- Dinamik proje (Netpro 12): arazi, yatay ve düşey güzergâh, **ripaj** ve **km eşitliği**, sıyırma/diş kazısı, kesit ve profil çizimleri, şevli kotlu plan ve kübaj kuralları değişince güncellenir; bu bir seçenektir. Yatay ve düşey düzenleme fare hareketiyle eşzamanlı önizlenir.
- KTEK 2022: tanımlı standartlar, dever uygulamaları, yatay ve düşey kurp oluşturma ve denetimi, fmax sürtünme katsayısı, ağırlık duvarı kalınlıkları ve kurp genişleme katsayıları güncellenmiş; TCK ve AASHTO profilleri; Brükner dengelemesi.
- Sanat yapıları ve güvenlik: menfez, kutu menfez, istinat duvarı, palye hendeği; duvar türleri (betonarme iksa, betonarme istinat, toprakarme, harçlı istinat ve iksa, banket altı); metrajın KGM pozlarıyla raporu; **otokorkuluk** aralıklarının dolgu yüksekliğine göre bulunması.
- Koridor analizi: arazi modeli üzerinde eğim ölçütüyle en uygun güzergâh ve alternatifleri, tünel/köprü/viyadük, yasak alanlar (askerî, sit), maliyet–eğim–uzunluk karşılaştırması.
- Proje dosyaları sürümlenir (numara, açıklama); yatay, düşey ve kesit bilgisi ölçüm cihazlarına (Leica, Topcon, Trimble, Nikon) tek işlemle aktarılır; DWG/DGN ve LandXML alışverişi.

**Bentley OpenRoads Designer** ([ürün sayfası](https://www.bentley.com/software/openroads-designer/), [CONNECT yardımı](https://docs.bentley.com/LiveContent/web/OpenRoads%20Designer%20CONNECT-v11/en/GUID-44A992C7-08A8-4C17-9BF0-496E675D0315.html), FHWA ve eyalet DOT eğitim belgeleri): ham ölçme biçimleri, karmaşık ve fark (delta) arazi modelleri; yatay geometri yay ya da kiriş tanımlı, düşey parabolik ya da dairesel; dever (superelevation/cant) yatay geometriden kural dosyasıyla hesaplanır ve koridora atanır, şablonda dever alacak kaplama noktaları işaretlenir; şablon nokta ve bileşenlerden oluşan enkesittir ve kitaplıkta saklanır; koridor yatay + düşey + şablon ister; dinamik enkesitler, kesit ve plan–profil paftası üretimi; üçgenlemeli ve uç alan (end-area) hacim, özelleştirilebilir kübaj raporu; menfez tasarımı ve hidrolik analiz; nokta bulutu bağlama ve ölçeklenebilir arazi; iTwin'e yayın. [FHWA şablon kitaplığı](https://highways.dot.gov/sites/fhwa.dot.gov/files/docs/federal-lands/cadd-support/ord-user-manual/13331/ord-manual08-template-library_2.pdf), [FHWA dever](https://highways.fhwa.dot.gov/sites/fhwa.dot.gov/files/docs/federal-lands/cadd-support/ord-user-manual/13266/ord-manual10-superelevation.pdf)

**Autodesk Civil 3D 2026.2** ([yenilikler](https://help.autodesk.com/cloudhelp/2026/ENG/Civil3D-WhatsNew/files/GUID-1777B37B-2DDA-459B-8CDF-AD17B9331C75.htm)): LandXML dışa aktarmada desteklenmeyen eğri türü için uyarı ve spiral örnekleme ayarı eklendi. KentOS için ders: desteklenmeyen spirali sessizce örneklemek yerine ROAD-ALIGN-14'teki kayıp raporu doğru yoldur. [Geçiş eğrisi türleri](https://help.autodesk.com/cloudhelp/2024/ENG/Civil3D-UserGuide/files/GUID-DD7C0EA1-8465-45BA-9A39-FC05106FD822.htm): klotoid, Bloss, kübik parabol, sinüs eğrisi; Trimble'ın [geçiş türleri](https://help.fieldsystems.trimble.com/trimble-access/2023.10/en/RXLroad-transitions.htm) sahadaki karşılıklarıdır.

**IFC 4.3** ([IfcAlignment](https://standards.buildingsmart.org/IFC/RELEASE/IFC4_3/HTML/lexical/IfcAlignment.htm), [güzergâh geometrisi kavramı](https://ifc43-docs.standards.buildingsmart.org/IFC/RELEASE/IFC4x3/HTML/concepts/Product_Shape/Product_Geometric_Representation/Alignment_Geometry/content.html)): yatay düzen bileşik eğri (`IfcCompositeCurve`), yatay+düşey 3B güzergâh `IfcGradientCurve`, dever/cant ise ona göre tanımlı `IfcSegmentedReferenceCurve` ile temsil edilir; semantik düzen (`IfcAlignmentHorizontal`, `…Vertical`, `…Cant` segmentleri) geometriden ayrıdır. KentOS'un kaynak modeli de semantik segment ile çizim geometrisini ayırır (§5), bu eşleme ROAD-IO-06'nın temelidir.

**LandXML 1.2:** `Alignment` içinde `StaEquation`, `CoordGeom` (Line, Curve, `Spiral spiType="clothoid"` …) ve `Profile/ProfAlign` (PVI); Japonya NILIM profili ve DOT örnek dosyaları ([NILIM LandXML 1.2 tabanlı şema](https://www.nilim.go.jp/lab/qbg/researchfields/pdf/h28_basedLandXML1.2.v1.0_English.pdf)) bağımsız sınama girdisi olarak kullanılabilir.

**Ulusal standart:** KTEK 2022 ve KTŞ 2023 ücretli basılıdır (başlıktaki not). Ders notlarında geçen ifadeler (klotoid için A² = R·L; A için R/3 ≤ A ≤ R aralığı; KGM dever bağıntısı ve %8 üst sınır) yalnız formül ve terim kaynağıdır; sayısal sınır olarak kullanılmaz, resmî kitapla doğrulanır.

### 3.2 Terimler (arayüz Türkçesi)

| Arayüz | Anlamı ve KentOS kaynağı |
|---|---|
| Güzergâh, aks | Yatay güzergâh (`HorizontalAlignment`); elemanları **aliyman** (doğru), **kurp** (dairesel yay), **geçiş eğrisi** (klotoid; ove spiral iki kurp arası, ters kurp zıt yönlü ardışık kurplar) |
| Klotoid parametresi A | A² = R·L; eğrilik boy boyunca doğrusal değişir |
| Km, km eşitliği | İstasyon (0+000,00 biçimi); geri/ileri km eşitliği (station equation) |
| Ripaj | Güzergâhın yatay kaydırılması/yeniden konumlandırılması; dinamik revizyon olayı |
| Düşey güzergâh, düşey some, düşey kurp | Profil; some = PVI; parabolik ya da dairesel kurp; bisektris (some ile kurp arası düşey uzaklık) |
| Kırmızı kot, siyah kot | Proje (tasarım) kotu ve arazi kotu |
| Dever, dever rakortmanı, kurp genişlemesi | Enine eğim (superelevation), geçiş boyu (runoff), kurpta şerit genişletmesi |
| Tip enkesit, platform, formasyon | Şablon (template); platform tasarım yüzeyi, formasyon üst yapı altındaki taban (subgrade) yüzeyi |
| Şev, yarma, dolgu, banket, hendek | Yan eğimler; yarma kazı tarafı; daylight hedefi araziye çarpan şev |
| Sıyırma kazısı, diş kazısı, bitkisel toprak | Bitkisel toprağın sıyrılması; dolgunun eğimli zemine oturması için basamaklı kazı |
| Kübaj, kabarma, sıkışma | Hacim hesabı; gevşek ve sıkıştırılmış hacim katsayıları (bank/loose/compacted) |
| Brükner diyagramı | Birikimli kütle (mass-haul) eğrisi |
| Şevli kotlu plan | Şev taramalı, kotlu, yarma/dolgu renkli plan paftası |
| Aplikasyon | Saha işaretleme (stakeout) |
| DGM, GGM | Duruş görüş mesafesi, geçiş görüş mesafesi |
| Otokorkuluk, menfez, istinat duvarı | Güvenlik bariyeri; kutu/borulu menfez (culvert); dayanma duvarı |

KentOS'un hedeflediği farklar: kaynak kalitesiyle birlikte doğruluk raporu; donanıma göre bütçeli 3D; değişen station aralığını yeniden hesaplama; ortak UI/Python/AI; offline/merkezi saha bağlantısı ve Digital Twin'e kalıcı kimliklerle geçiş. Bunlar hedeflerdir, mevcut ürüne veya rakiplere karşı ölçülmüş üstünlük iddiası değildir.

## 4. ROAD-NUM — Koordinat, birim, datum ve sayısal politika — P0

Render origin/hi-lo/depth politikasının ayrıntısı [3d.md](3d.md)'dedir. Yol hesabında kaynak FP64 kalır; görünüş mesh'inin quantization ve LOD hatası raporlanan mühendislik hacmine taşınmaz. KGM/AASHTO gibi limit tabloları sabit rastgele değerler yerine **kaynağı, baskısı ve yürürlük kapsamı belirli DesignStandard** ile seçilir. [KGM yayın listesi](https://www.kgm.gov.tr/Sayfalar/KGM/SiteTr/Yayinlar/Yayinlar.aspx)

- [ ] **ROAD-NUM-01:** Canonical yatay eksen x=doğu, y=kuzey, z=yukarı olsun; eski CAD X/Y gösterim etiketleri açık presentation mapping ile korunmalı.
- [ ] **ROAD-NUM-02:** Her Survey/Terrain/Alignment kaynakta yatay CRS, düşey CRS/datum, metre/ayak, epoch, geoid dönüşüm referansı ve dönüşüm zinciri taşısın.
- [ ] **ROAD-NUM-03:** Yükseklik h ellipsoidal ile H ortometrik ayrımı açık olsun; geoid N için H=h−N dönüşümü yalnız doğru model/datum/epoch ile uygulansın.
- [ ] **ROAD-NUM-04:** Bilinmeyen düşey datum veya unit olduğunda karşılaştırmalı hacmi bloke et ya da açık taslak varsayım profiline bağla; sessiz sıfır kot veya geoid tahmini yapma.
- [ ] **ROAD-NUM-05:** Grid chainage, ground mesafe ve gerçek 3D uzunluk ayrı ölçü türleri olsun; scale factor ve ellipsoid/geoid hesabı nerede kullanıldığıyla kaydedilsin.
- [ ] **ROAD-NUM-06:** Analitik aks, profile, TIN ve final miktar kaynakları FP64/robust topoloji kullansın; GPU render kopyası kaynağa FP32 değer yazmasın.
- [ ] **ROAD-NUM-07:** Projeyi ortalayarak yapılan sayısal normalizasyonun dönüşümünü sakla; EPSG/datum ya da ölçü kaynak değerleri değişmiş gibi kaydetme.
- [ ] **ROAD-NUM-08:** Hata bütçesi survey, CRS/vertical transform, modelleme, örnekleme, GPU encoding ve integration olarak ayrışsın; ekran hassasiyeti kaynak ölçme kalitesi sayılmasın.
- [ ] **ROAD-NUM-09:** Final miktarda reduction sırası ve compensated/pairwise sum tanımlansın; native/WASM/worker farkı tolerans ve algoritma sürümüyle ölçülsün.
- [ ] **ROAD-NUM-10:** Non-finite, aşırı kapsam, çok kısa eleman, eş/çok yakın XY ve tutarsız Z girişlerini alan/record path ile reddet veya açık çözüm politikası iste.
- [ ] **ROAD-NUM-11:** Station/offset/elevation, angle, slope, crossfall ve volume wire birimleri schema'da bulunsun; yüzde ile oran, derece ile radyan karışmasın.
- [ ] **ROAD-NUM-12:** Yol/3D precision korpusunu projeksiyonlu büyük koordinat, negatif koordinat, yerel küçük detay ve kilometre uzunluğunda aksla kur.

## 5. Kalıcı kaynak modeli ve bağımlılık grafı

| Kaynak | Asgari alanlar |
|---|---|
| RoadProject / DesignAlternative | ID/revision, workspace, coordinate reference, standart profili, source graph, scenarios, owners |
| SurveyDataset / CloudSource | Digest, source records, ölçek/offset, CRS/datum, ölçme zamanı, class/quality/coverage, index refs |
| TerrainSurface | Surface ID/revision, 2.5D/DEM türü, vertices/triangles/topology, breaklines, boundary/holes, provenance, quality mask |
| HorizontalAlignment | Analitik element IDs, line/arc/spiral parametreleri, direction, base station, station equations, continuity |
| VerticalAlignment | Horizontal ref, tangent grades, vertical curves, constraints, minimum/maximum grade profili |
| RoadTemplate / Component | Point/link/shape codes, material, constraints, side, target capability, param expressions, version |
| CorridorDefinition | Baselines, regions, template drops, target refs, crossfall/dever/widening, station policy, source revisions |
| CorridorResult | Build revision, solved stations, feature strings, surface/material meshes, warnings, lineage, error bounds |
| SectionSet / ProfileView | Sources, sample stations/offsets, exact intersection results, labels, view/paper configuration |
| QuantityRun / HaulPlan | Source digests, AOI, yöntem/tolerans, materials/states/factors, gross/net amounts, costs, approval |
| Job / Artifact | Algorithm/schema version, backend/capability, snapshot, inputs, plan, cancel/progress, reproducible outputs |

Önerilen kaynak zinciri: `survey/cloud → approved terrain → horizontal+vertical+template+targets → corridor → section/surface/material → quantity/haul → sheets/stakeout/twin`.

Kamera, LOD, GPU buffer ve geçici triangle index bu kalıcı grafın kimliği değildir. Terrain revision değişince triangle ref geçersizleşmesi/lineage açıktır. Aksın segment ID'si taşınır; aynı station text'i farklı station-equation dalını temsil edebileceği için kimlik olamaz.

## 6. ROAD-SURVEY — Ölçme, kaynak QA ve proje hazırlığı — P0/P1

- [ ] **ROAD-SURVEY-01:** Ölçü noktası, feature code, breakline, control point, kalite ve survey epoch'u kalıcı typed kaynağa al.
- [ ] **ROAD-SURVEY-02:** Yerel/gerçek CRS seçimi ve yatay/düşey dönüşüm önizlemesi control residual ve coverage raporuyla sunulsun.
- [ ] **ROAD-SURVEY-03:** LandXML/CSV/XYZ/LAS girişlerinde unit, eksen sırası, delimiter, precision ve class mapping inspect aşamasında doğrulansın.
- [ ] **ROAD-SURVEY-04:** Tekrarlı XY, aykırı Z, noise, flight-strip farkı ve eksik coverage raporlarını ayrı QA job yap; ham kayıt korunmalı.
- [ ] **ROAD-SURVEY-05:** Sert zemin ve bitki kaplı alan için uygun checkpoint sınıfları/istatistikleri ayrı göster; yalnız tek RMSE sayısına dayanma.
- [ ] **ROAD-SURVEY-06:** Ölçülmüş breakline ile algoritmanın çıkardığı öneri ayrışsın; kullanıcı onayından sonra civil source'a bağlansın.
- [ ] **ROAD-SURVEY-07:** Kontrol ağı ve koordinat dönüşümü sonrasında original→normalized kaynak eşlemesi ve dönüşüm grid digest'i korunsun.
- [ ] **ROAD-SURVEY-08:** Proje sınırı, survey acceptance mask, no-data bölgeleri ve tahmini/ölçülmüş veri işaretleri açık olsun.
- [ ] **ROAD-SURVEY-09:** Saha ölçüsü revisyonu mevcut terrain'i sessiz değiştirip onaylı miktarı yenilemesin; impact plan ve yeni run üret.
- [ ] **ROAD-SURVEY-10:** Survey kontrol, ölçüm kayıtları ve revizyon girişi için [form.md](form.md) field/row/task yetkilerini kullan.

**Kabul:** İki datumdaki bulut/ölçü ayrı görünür; onaylı dönüşüm ve kontrol raporu olmadan ortak kazı/dolgu çalışmaz. Survey doğruluğu kontrol noktalarıyla değerlendirilir. [USGS geometrik doğruluk çalışması](https://pubs.usgs.gov/publication/ofr20251017/full)

## 7. ROAD-PC — Nokta bulutu mühendislik iş akışı — P0/P1

COPC 1.0, LAZ 1.4 verisini octree hierarchy ve erişilebilir chunk'larla düzenler; parçalı tüketim için doğal bir temeldir. [COPC şartnamesi](https://copc.io/). Zemin algoritması seçimi ayrı doğruluk işidir; [PDAL SMRF](https://pdal.org/en/stable/stages/filters.smrf.html) ve [zemin iş akışı](https://pdal.org/en/stable/tutorial/ground-filters.html) kıyas kaynağıdır. Bu plan PDAL kurulumunu veya kullanıcıya Python zorunluluğunu varsaymaz.

- [ ] **ROAD-PC-01:** Mevcut LAS/LAZ/COPC/text/VPC reader ve Need/range adapter'larını genişlet; tüm dosyayı parse eden ikinci bağımsız reader kurma.
- [ ] **ROAD-PC-02:** COPC node hierarchy, source digest/ETag, range cache ve gerekli dimensions seçimini ortak 3D streaming scheduler'a bağla.
- [ ] **ROAD-PC-03:** Ham LAS integer scale/offset ve source record identity korunmalı; ekran FP32 noktalarından LAS export veya survey edit üretme.
- [ ] **ROAD-PC-04:** Ground/classification/return/intensity/RGB/scan-angle/time için typed filtre ve histogram/query arayüzü kur.
- [ ] **ROAD-PC-05:** Mevcut zemin filtresini referans CPU yolu olarak koru; GPU grid/min/morphology adaylarını aynı girdiler ve halo sınırlarıyla karşılaştır.
- [ ] **ROAD-PC-06:** PMF/SMRF, progressive TIN densification ve cloth-based adaylarını ayrı profiller olarak dene; yöntem adını mevcut filtreden farklıysa doğru koy.
- [ ] **ROAD-PC-07:** Şerit uyumsuzluğu, outlier, duvar/bitki ve köprü tabliyesi durumlarını kontrol edilebilir QA katmanı olarak üret.
- [ ] **ROAD-PC-08:** Classification değişikliklerini original point ID→class patch olarak sakla; kullanıcı düzeltmesi ve original class geçmişini koru.
- [ ] **ROAD-PC-09:** Kesit bandı sorgusu radius/width/CRS/station kapsamını alsın; yalnız ekranda resident node'ları tarayıp tamamlandı demesin.
- [ ] **ROAD-PC-10:** Arazi için ground subset çıkarırken density/coverage ve thinning hatasını kaydet; bridge/overpass noktalarını yer yüzeyiyle birleştirme.
- [ ] **ROAD-PC-11:** Adaptive thinning breakline, curb ve sharp slope çevresinde ayrıntıyı korusun; voxel size/density yalnız görüntü LOD'u olmasın.
- [ ] **ROAD-PC-12:** Point→DEM/TIN, contours, height-above-ground ve as-built değişim araçları built-in Processing descriptor kullansın.
- [ ] **ROAD-PC-13:** Bulk point edit için undo delta, chunk provenance, staged write ve cancel/recovery sınırı tanımla.
- [ ] **ROAD-PC-14:** Milyarlarca kayıtta full-source job bounded RAM ile çalışsın; node/partition sınırında görünmeyen gap ve duplicated point üretmesin.
- [ ] **ROAD-PC-15:** Render point pick önce aday olur; exact source point/cluster ve uncertainty ile refine et. Tam kayıt hazır değilse pending göster.
- [ ] **ROAD-PC-16:** Nokta bulutu yetkisi ve private class/attribute projection'ı metadata/range/export/AI'da da uygulansın.

## 8. ROAD-TIN — Mühendislik yüzeyi ve constrained üçgenleme — P0/P1

Constrained Delaunay, sıradan Delaunay'dan farklı olarak korunan kenarları modele dahil eder. Robust karar ve intersection handling ayrı konulardır. [CGAL 2D triangulations](https://doc.cgal.org/latest/Triangulation_2/index.html), [Shewchuk robust predicates](https://www.cs.cmu.edu/~quake/robust.html). KentOS'un mevcut exact-sign çekirdeği genişletilmelidir; harici kütüphane seçimi bağımlılık/lisans/prototip kararıdır.

- [ ] **ROAD-TIN-01:** Mevcut Delaunay/predicates'i persistent TerrainSurface, topology ve elevation query sözleşmesine bağla.
- [ ] **ROAD-TIN-02:** Delaunay, constrained Delaunay ve fill ear-clipping ayrı algorithm IDs olsun; yanlış algoritma adıyla sonuç yayınlama.
- [ ] **ROAD-TIN-03:** Breakline hard/soft, boundary/hole, island ve maximum edge length kuralları modelde sürümlensin.
- [ ] **ROAD-TIN-04:** Intersecting breakline için split/ret/priority ve çelişkili Z politikası açık olsun; yatay kesişimi olan farklı kotlu bridge çizgileri otomatik birleştirilmesin.
- [ ] **ROAD-TIN-05:** Aynı XY ve farklı Z tek-valued terrain için ambiguity'dir; onaylı selection/aggregation veya ayrı surface patch/3D mesh gerekir.
- [ ] **ROAD-TIN-06:** Constraint endpoints ve korunan terrain feature IDs kararlı olsun; tasarım revision'ı triangle index'e kör bağlanmasın.
- [ ] **ROAD-TIN-07:** Nearly collinear/cocircular/duplicate noktaları mevcut adaptive exact-sign kararlarıyla sınayarak topology üret.
- [ ] **ROAD-TIN-08:** Exact predicate yalnız determinantın işaretini doğrular; intersection coordinates/areas için ayrıca construction error ve degeneracy politikası tanımla.
- [ ] **ROAD-TIN-09:** Point insert/remove/move, edge flip, triangle remove, hole edit ve breakline ekleme local rebuild/impact plan kullansın.
- [ ] **ROAD-TIN-10:** Elevation query result z, triangle/source revision, interpolation method, no-data/extrapolation ve quality taşısın.
- [ ] **ROAD-TIN-11:** Extrapolation varsayılan kapalı olsun; dış kabuk veya hole içinde nokta için tahmini kot ancak açık parametreyle üret.
- [ ] **ROAD-TIN-12:** TIN tiles için shared border ownership ve seam reconciliation kur; sıradan sabit halo'nun global Delaunay garantisi vermediğini hesaba kat.
- [ ] **ROAD-TIN-13:** Partition sınırında empty-circle/constraint etkileşimi için proven merge veya adaptive genişleme kullan; aynı kaynak monolithic referansla karşılaştırılsın.
- [ ] **ROAD-TIN-14:** Analiz topology ve display simplification ayrı olsun; decimation sonucu final volume veya aplikasyon query'si olmasın.
- [ ] **ROAD-TIN-15:** Kontur üretimi vertex/edge eşitlik kuralı, plateau, hole ve snapping toleransıyla deterministic olsun.
- [ ] **ROAD-TIN-16:** Slope/aspect ve surface roughness değerlerini yüzey türü/metot/units ile döndür; DEM raster yaklaşımıyla TIN'in farklı sonucunu açıklayabilsin.
- [ ] **ROAD-TIN-17:** Surface smoothing/repair rollback ve diff sunsun; tasarım noktası veya breakline otomatik kaydırılmasın.
- [ ] **ROAD-TIN-18:** İleri hedefte GPU broad phase/sorting/triangle candidates kullan; uncertified FP32 incircle ile authoritative topology yayınlama.

**Kabul:** Breakline triangle kenarı olarak korunur; hole kapanmaz; tile sınırındaki kotlar birleşir; yüzey editinden sonra revision-aware query ve volume önceki sonuçları güncelmiş gibi göstermez.

## 9. ROAD-ALIGN — Yatay güzergâh ve istasyon — P0/P1

OpenRoads'un kamuya açık yetenek listesi birden çok spiral türü ve yatay/düşey geometriyi; 12d ise referansla bağlı alignment tasarımını gösterir. KentOS v1'de line/arc/clothoid'i bağımsız doğrular, diğer spiral türlerini capability matrisiyle genişletir. [OpenRoads](https://www.bentley.com/en/products/openroads-designer/), [12d](https://12d.com/product/12d_model_alignment.html)

- [ ] **ROAD-ALIGN-01:** Alignment ID/revision ve line/arc/spiral analitik segment IDs tanımla; görsel spline kaynak tasarım geometrisi olmasın.
- [ ] **ROAD-ALIGN-02:** IP/PI tabanlı ve fixed/floating/free element tabanlı düzenlemeyi aynı analitik kaynakla ilişkilendir.
- [ ] **ROAD-ALIGN-03:** Line, circular arc ve clothoid'in point/tangent/curvature/length query'lerini FP64 çekirdekte kur.
- [ ] **ROAD-ALIGN-04:** Clothoid için k(s)=k0+(k1−k0)s/L ve theta(s)=theta0+k0s+(k1−k0)s²/(2L) tanımını kullan; Fresnel veya hata sınırlı integration yöntemi ADR'de seçilsin.
- [ ] **ROAD-ALIGN-05:** Doğru→spiral→kurp→spiral→doğru zincirinde konum/teğet/eğrilik süreklilik koşulları ve kalan residual raporlansın.
- [ ] **ROAD-ALIGN-06:** Station, başlangıçtan analitik yatay uzunluk olsun; 3D uzunluk ve kullanıcıya gösterilen km label'ı ayrı query olsun.
- [ ] **ROAD-ALIGN-07:** Station equations back/ahead, direction ve label discontinuity'yi taşısın; aynı label birden fazla yere karşılık gelirse branch/element ID istenmeli.
- [ ] **ROAD-ALIGN-08:** Station→point ve point→station/offset dönüşümünde çoklu yakın çözüm, eğri üzerinde minimum ve tie-break politikası açık olsun.
- [ ] **ROAD-ALIGN-09:** Offset alignment'ı kaynak aksla kalıcı ilişkilendir; keskin/ters kurpta cusp/self-intersection detect ve ret/explicit çözüm sun.
- [ ] **ROAD-ALIGN-10:** Alignment edit ve reverse için segment/station mapping üret; ölçü, profil, corridor region ve stakeout refs'e impact raporu ver.
- [ ] **ROAD-ALIGN-11:** DesignStandard speed/radius/spiral length/visibility kontrolünü sürümlü kurallar olarak uygula; kendi ilk sayılarımızı mevzuat limiti sayma.
- [ ] **ROAD-ALIGN-12:** Alternative alignment ve compare için parent/source digest ve değişen kilometre aralığı korunsun.
- [ ] **ROAD-ALIGN-13:** Ön tasarım için mevcut GIS suitability/cost path kullanılabilir; ortaya çıkan raster path teknik aks olarak ayrıca fit/QA/onaydan geçsin.
- [ ] **ROAD-ALIGN-14:** LandXML/IFC'den gelen unsupported spiral türlerini analitik kayıpla düz spline'a sessiz dönüştürme; source preserve ve loss report üret.
- [ ] **ROAD-ALIGN-15:** Ove spiral (iki kurp arası klotoid parçası), ters kurp (zıt yönlü ardışık kurplar, araya aliyman konmadan) ve kesişen kurpların çakışma denetimi ayrı eleman ve kural olsun; Netpro'nun AASHTO hız–yarıçap ile kurp çakışmasını kaldırması gibi öneri ayrı taslaktır, kaynak aksı sessiz değiştirmez.
- [ ] **ROAD-ALIGN-16:** Ripaj (aksı yatay kaydırma/yeniden konumlandırma) ve km eşitliği ekleme kayıtlı düzenleme olayıdır; profil, kesit hattı takımı, dever çizelgesi ve kübaj km'leri eşleme tablosuyla izler ya da çözülemeyen aralık olarak işaretlenir (ROAD-MOD-06).

## 10. ROAD-PROFILE — Düşey güzergâh ve boykesit — P1

- [ ] **ROAD-PROFILE-01:** VerticalAlignment, horizontal revision'a bağlansın; PVI, tangent grades, parabolic crest/sag ve ileride circular curve türlerini saklasın.
- [ ] **ROAD-PROFILE-02:** Yatay station t boyunca parabolic kurbu z(t)=z0+g1·t+(g2−g1)t²/(2L) ile tanımla; g oran, L yatay metre olsun.
- [ ] **ROAD-PROFILE-03:** PVI/kurp uzunluğu, başlangıç/son station, kot ve grade constraint solver çıktısını residual ile doğrula.
- [ ] **ROAD-PROFILE-04:** K=L/|g2−g1| raporunda grade yüzde mi oran mı açık olsun; standarttaki K birimiyle doğru çevir.
- [ ] **ROAD-PROFILE-05:** Mevcut arazi profili TIN triangle edge geçişlerinde exact parçalı kesit olarak çıkarılsın; yalnız sabit aralıklı ekran örneklemesi olmasın.
- [ ] **ROAD-PROFILE-06:** Boykesitte yatay/düşey çizim ölçekleri ayrı; exaggeration veri kotunu, slope veya hacmi değiştirmesin.
- [ ] **ROAD-PROFILE-07:** Bridge/tunnel/culvert/utility crossing ve clearance constraints profile'a referansla bağlansın.
- [ ] **ROAD-PROFILE-08:** Görüş mesafesi kontrolü araç/engel/göz yüksekliği, standart, terrain/corridor revision ve örnekleme hatasını açık alsın.
- [ ] **ROAD-PROFILE-09:** Aks profilini değiştirirken yeniden üretilecek corridor/quantity/section aralıklarını göster; stale derived kaynaklar işaretlensin.
- [ ] **ROAD-PROFILE-10:** Terrain missing region, gap, override ve measured/proposed profile ayrı görünüş/provenance taşısın.
- [ ] **ROAD-PROFILE-11:** Profil edit/drag için responsive GPU preview ile authoritative solve farklı durum olsun; final save exact validator'dan geçsin.
- [ ] **ROAD-PROFILE-12:** Analitik point/grade/curvature query, profile report ve sheet üretimi aynı komutlardan çalışsın.
- [ ] **ROAD-PROFILE-13:** Düşey kurp boyu bisektris, açık/kapalı kurp katsayısı, DGM ve GGM ölçütlerinden önerilsin; ölçütün kaynağı standart profilidir (ROAD-DEC-01), profil yoksa öneri yapılmaz, yalnız elle boy girilir.
- [ ] **ROAD-PROFILE-14:** Kırmızı kot önerisi (en az kazı/dolgu ya da dengeli kütle veren düşey güzergâh) ayrı taslak alternatif olsun; amaç işlevi, kısıtlar (en büyük/en küçük eğim, kontrol kotları, köprü/menfez açıklığı) ve çözücünün durumu raporlansın (ROAD-GRADE-08/09 ile aynı kural).

## 11. ROAD-DEVER — Dever, enine eğim, genişletme ve geçişler — P1

- [ ] **ROAD-DEVER-01:** Lane/side ID, normal crossfall, maximum/minimum e, transition rules ve pivot/rotation axis tanımla.
- [ ] **ROAD-DEVER-02:** Dever schedule station aralıkları, interpolation, station equations ve source curve refs taşısın.
- [ ] **ROAD-DEVER-03:** Sol/sağ ve forward/reverse sign convention tek sözleşmede olsun; alignment reverse planı schedule'ı doğru remap etsin.
- [ ] **ROAD-DEVER-04:** Crown removal, runoff/runout ve kesintisiz geçiş için standard-profile rules ve kullanıcı override reason saklansın.
- [ ] **ROAD-DEVER-05:** Widening lane/component bazlı width function olsun; otomatik öneri design vehicle/speed/radius standardına bağlansın.
- [ ] **ROAD-DEVER-06:** Aynı station'da width/crossfall/height modifier çakışması priority/merge ile çözülsün; sessiz son satır kazanmasın.
- [ ] **ROAD-DEVER-07:** Pivot reference değişimi, widening target ve median split durumları corridor sampling event listesine eklensin.
- [ ] **ROAD-DEVER-08:** Grade/crossfall kaynakları oran biriminde canonical olsun; UI yüzde gösterimi açık çevrilsin.
- [ ] **ROAD-DEVER-09:** Drainage için reverse crossfall, zero-slope pockets ve low-point warnings ortak corridor modelinden üret.
- [ ] **ROAD-DEVER-10:** Dever profile editor, tables, curves ve 3D section sweep aynı kaynak schedule'ı gösterip değiştirir.
- [ ] **ROAD-DEVER-11:** Limit kontrolünün sonucu standard ID/version, checked parameters ve override actor/reason taşısın.
- [ ] **ROAD-DEVER-12:** Çok kısa ters kurp/transition overlap ve station-equation sınırları bağımsız kabul korpusunda olsun.

## 12. ROAD-COR — Tip enkesit, parametrik bileşen ve koridor — P1

- [ ] **ROAD-COR-01:** Template point/link/closed-shape kodlarını stable ID ile tanımla; lane, shoulder, median, sidewalk, ditch, cut/fill slope ve pavement layer bileşenleri olsun.
- [ ] **ROAD-COR-02:** Bileşen geometry/constraint ile material/cost/quantity code ve style ayrı fields olsun; yalnız çizgi rengi malzeme kimliği olmasın.
- [ ] **ROAD-COR-03:** Parametric dependency graph unit/type checked expressions kullansın; raw Python callback temel template için zorunlu olmasın.
- [ ] **ROAD-COR-04:** Width/slope/elevation/target station fonksiyonları ve right/left mirroring semantiği açık olsun.
- [ ] **ROAD-COR-05:** Component libraries owner/version/compatibility ve import/remap taşısın; template güncellemesi approved corridor'u sessiz değiştirmesin.
- [ ] **ROAD-COR-06:** Corridor baselines, regions ve template drops gap/overlap/priority kontrolüyle saklansın.
- [ ] **ROAD-COR-07:** Target alias; alignment, terrain, feature string, offset/elevation function ve intersection control kaynaklarını typed referansla alsın.
- [ ] **ROAD-COR-08:** Station sample listesi başlangıç/son, geometry events, template/target/dever değişimi ve kullanıcı critical station'ları kapsasın.
- [ ] **ROAD-COR-09:** Adaptive sampling position/tangent/crossfall/surface sag toleransını doğrulasın; yalnız sabit 20 m aralık tasarım garantisi sayılmasın.
- [ ] **ROAD-COR-10:** Daylight solver hangi terrain branch/sidewall aradığını açık alsın; no-hit, multiple-hit, iteration limit ve missing terrain ayrı sonuç olsun.
- [ ] **ROAD-COR-11:** İmalat feature strings ve top/datum/subgrade/material surfaces stable lineage ile oluşsun.
- [ ] **ROAD-COR-12:** Bridge/tunnel/retaining wall gibi çok katlı/overhang geometriyi 2.5D TIN'e zorla bastırma; 3D mesh/solid capability gerekir.
- [ ] **ROAD-COR-13:** Koridor self-intersection, region seam, inverted triangles ve material shape closure preflight'ı çalışsın.
- [ ] **ROAD-COR-14:** Preview GPU buffers ve source solve result revision/generation ile bağlansın; geç sonuç yeni tasarımı ezmesin.
- [ ] **ROAD-COR-15:** Küçük edit yalnız etkilenen station/region/terrain tiles ve halo'sunu rebuild etsin; downstream quantities stale işaretlensin.
- [ ] **ROAD-COR-16:** Build result solved/unsolved ranges, source refs, quality/error bound, generated feature/material IDs ve warnings taşısın.
- [ ] **ROAD-COR-17:** Create/update/rebuild/delete corridor ortak semantic komut ve tek commit/undo politikası kullansın.
- [ ] **ROAD-COR-18:** İleri aşamada 3D constructive solids ve intersection templates ayrı profille eklenir; v1 shape-code miktarlarını eksik solid gibi sunma.

**Kabul:** Profilin 100 m'lik parçası değiştiğinde ilgili koridor/kesit/miktar aralığı yeniden oluşur. Template, source terrain ve çözülemeyen daylight bölgeleri görülebilir; eski sonucu yeni revision sonucu diye yazmaz.

## 13. ROAD-SEC — Enkesit, boykesit ve aplikasyon — P1

- [ ] **ROAD-SEC-01:** SampleLineSet station/direction/skew/left-right offsets ve source list taşısın; station label yerine alignment identity kullan.
- [ ] **ROAD-SEC-02:** TIN section triangle-edge kesişimleriyle exact parçalı doğru olsun; breakline, hole ve no-data sınırlarını koru.
- [ ] **ROAD-SEC-03:** Cloud strip section full-resolution query ile paralel çalışsın; çizim amacı için decimation ayrıca belirtilsin.
- [ ] **ROAD-SEC-04:** Mevcut/tasarım/material/geology sections ayrı çizgiler ve source revisions ile aynı istasyonda gösterilsin.
- [ ] **ROAD-SEC-05:** Oblique/skew sections ile normal sections farklı tip olsun; komşu kesit arası hacimde geometric footprint açık hesaplansın.
- [ ] **ROAD-SEC-06:** Yatay/düşey view scales, graph offsets ve labels gerçek hesap koordinatlarını değiştirmesin.
- [ ] **ROAD-SEC-07:** Cross-section edit source mı override mı olduğu açık komut olsun; tek görünüşte sessiz bağımsız geometri kopması oluşmasın.
- [ ] **ROAD-SEC-08:** Missing surface range ve extrapolated z gösterimi actual cut/fill area'ya onaysız alınmasın.
- [ ] **ROAD-SEC-09:** Stakeout table station/offset/x/y/z/side/component/code ve grid/ground policy ile export edilsin.
- [ ] **ROAD-SEC-10:** Machine control/as-built çıktısında valid vertical datum, project localization ve quality acceptance şartları olsun.
- [ ] **ROAD-SEC-11:** Plan/profile/section sheet set [cad.md](cad.md) pafta motoruna bağlı kaynaklarla üretilsin; cache görseli yerine kaynak revision saklansın.
- [ ] **ROAD-SEC-12:** Form/twin'den gelen saha ölçüsünü station/source point lineage ile eşle; yanlış aks branch'ına otomatik kabul yapma.
- [ ] **ROAD-SEC-13:** Kesit hattı takımı üç yöntemle üretilsin: sabit aralık, km esaslı liste ve **üçgen kenarlarını kesen noktalar** (arazinin her kırığında kesit); kurp, spiral ve tasarım olaylarında sıklaştırma; dar kurpta komşu kesit kollarının kesişmesi kırpma ya da çarpık kesitle açıkça çözülsün.

## 14. ROAD-GRADE — Şev, platform, saha ve arazi düzenleme — P1/P2

- [ ] **ROAD-GRADE-01:** Platform boundary, target grade, cut/fill slope, bench, ditch ve retaining feature kaynaklarını ayrı tanımla.
- [ ] **ROAD-GRADE-02:** Şev daylight rule'u soil/material/geotechnical profile'dan gelsin; örnek bir şev oranı gerçek saha kararı gibi varsayılmasın.
- [ ] **ROAD-GRADE-03:** Cut/fill bench ve slope transitions bölgesel hedef/surface karşılaşma koşullarıyla üret.
- [ ] **ROAD-GRADE-04:** Platform/road surface merge için overlap ownership ve seam/breakline policy iste; ilk surface sessiz kazanmasın.
- [ ] **ROAD-GRADE-05:** Water drainage basin, low point, ponding ve slope-direction görselleştirmesi verified terrain sürümünden gelsin.
- [ ] **ROAD-GRADE-06:** Boundary/hole ve protected/no-earthwork bölgeleri terrain/quantity clip'e birlikte yansısın.
- [ ] **ROAD-GRADE-07:** 3D grading edit aynı exact source constraint ile plan/profil/section görünüşünü yenilesin.
- [ ] **ROAD-GRADE-08:** Optimization objective cut/fill/cost/land-use kısıtlarını ve bound'ları açık alsın; öneri ayrı alternative draft olsun.
- [ ] **ROAD-GRADE-09:** Aykırı solution, bounds ihlali ve optimizer convergence state kullanıcıya görünsün; minimum bulundu diye global optimum iddia etme.
- [ ] **ROAD-GRADE-10:** Platform, trench ve road grading volume sonuçları aynı QuantityRun standardını kullansın.

## 15. ROAD-VOL — Kazı/dolgu, malzeme hacmi ve hesap sertifikası — P0/P1

**Tanım:** Aynı düşey datumda `d(x,y)=z_design−z_existing`. `V_fill=∫max(d,0)dA`, `V_cut=∫max(−d,0)dA`; `dA` yatay plan alanıdır. Source TIN'lerin parça başına lineer yüzeylerini ortak bölüp sıfır çizgisinde ayırınca her alt üçgende `V_signed=A(d0+d1+d2)/3` kullanılır. Bu yöntem, yüzey modelinin temsil ettiği miktarın integralidir; gerçek zeminin ölçülmemiş ayrıntısını exact hale getirmez.

Kesit yöntemleri: `V_AEA=L(A0+A1)/2`; gerçek orta kesit alınırsa `V_prismoidal=L(A0+4Am+A1)/6`. Sabit aralıkta bu formülleri kullanmak genel arazi için otomatik exact sonuç garantisi değildir. Raster yöntemi resolution, cell coverage ve refinement convergence raporu taşımalıdır. [Civil 3D yöntem ayrımı](https://help.autodesk.com/cloudhelp/2018/ENU/Civil3D-UserGuide/files/GUID-1E3A9CD9-DFD6-4455-9EB1-D8E46721A0B4.htm)

**Kontrol örneği:** 10×10 m AOI'de `d=x−5` ise 125 m³ fill ve 125 m³ cut vardır; net sıfırdır. Neti veya `abs(net)`i toplam kazı/dolgu diye kullanmak yanlıştır.

- [ ] **ROAD-VOL-01:** QuantityRun source original/design surfaces, AOI, overlap/no-data coverage, method, tolerance, algorithm version ve units taşısın.
- [ ] **ROAD-VOL-02:** Cut ve fill gross ayrı hesaplansın; net=fill−cut tek başına hakediş veya taşınacak malzeme miktarı olmasın.
- [ ] **ROAD-VOL-03:** AEA, prismoidal, TIN overlay ve raster integration ayrı methods olsun; rapor hangi yaklaşım ve örnekleme adımını kullandığını söylesin.
- [ ] **ROAD-VOL-04:** TIN overlay iki source triangulation kenarlarını ve AOI/hole sınırlarını ortak planar subdivision'a bölsün.
- [ ] **ROAD-VOL-05:** Height difference sıfır çizgisinde alt poligonları split et; işaret değiştiren üçgende signed average'ın mutlak değeri gross miktar değildir.
- [ ] **ROAD-VOL-06:** Adaptive exact predicates ve construction error controls sliver/near-coincident edge/zero-area durumlarını güvenle çözsün.
- [ ] **ROAD-VOL-07:** Her overlay alt üçgende integral signed area·(d0+d1+d2)/3 olarak hesapla; aynı sign region'ları cut/fill'e ayrı ekle.
- [ ] **ROAD-VOL-08:** Tam ortak domain dışında veri olmayan alanı sıfır kazı/dolgu sayma; excluded area/coverage ve blocked final durumunu raporla.
- [ ] **ROAD-VOL-09:** Material layers ve geology strata için karşılaştırılan top/bottom/shape'ın ownership ve units'ini kontrol et; aynı excavation iki malzemeye çift yazılmasın.
- [ ] **ROAD-VOL-10:** Bank/in-situ, loose ve compacted hacimler açık volume-state conversion ile taşınsın; swell/shrink/compaction faktörleri kaynak ve onay taşısın.
- [ ] **ROAD-VOL-11:** Solid/overhang yapı miktarı watertight oriented topology gerektirsin; kapalı olmayan mesh'e güvenilir solid volume verme.
- [ ] **ROAD-VOL-12:** GPU candidate intersection/clip/reduction ile preview hesap üret; final commit error budget/CPU robust doğrulama kapısından geçsin.
- [ ] **ROAD-VOL-13:** Final miktar için sabit reduction order veya deterministic CPU sum kullan; nondeterministic float atomics nedeniyle farklı cihazda farklı hakediş çıkmasın.
- [ ] **ROAD-VOL-14:** Quantity report numeric/mesh/measurement error bileşenleri ve kabul edilen bounds taşısın; FP64 hesap survey kalitesini artırmış sayılmasın.
- [ ] **ROAD-VOL-15:** Alternative/as-built/period quantity kaynak digests ve önceki approved run'a referansla karşılaştırılsın.
- [ ] **ROAD-VOL-16:** Raporu oluştururken yuvarlama sona bırakılsın; kesit başına gösterim yuvarlamasını tekrar toplayıp final hacim yapma.
- [ ] **ROAD-VOL-17:** Approved quantity revision immutable olsun; kaynak değişince yeni run ve diff üret, önceki hesap/audit silinmesin.
- [ ] **ROAD-VOL-18:** Hesap kaynak parçaları ve contribution ledger drill-down ile incelenebilsin; station/material/AOI bazında büyük toplamın nedeni görülsün.
- [ ] **ROAD-VOL-19:** Kübaj kuralları adlı ve sürümlü olsun: Yol–Zemin, Formasyon–Zemin, Yol–Bitkisel Toprak, Formasyon–Bitkisel Toprak, Sıyırma Kazısı (Netpro'nun adları); her kural hangi iki yüzeyi, hangi sınırları (yatay ve düşey) ve hangi malzeme durumunu kullandığını söylesin.
- [ ] **ROAD-VOL-20:** Sıyırma kazısı (bitkisel toprak kalınlığı) ve diş kazısı (eğimli zeminde dolgu oturma basamakları) ayrı kesit kaynakları olsun; kalınlık, basamak boyu ve eşik eğim parametreleri ölçme ya da zemin raporundan gelir, örnek değer saha kararı sayılmaz.
- [ ] **ROAD-VOL-21:** Km aralıklarına göre kabarma ve sıkışma katsayıları tablo olarak verilebilsin; hangi katsayının hangi aralıkta hangi hacme uygulandığı defterde görünsün; XLSX/ODS/CSV kübaj cetveli ADR 0223'ün yazıcısıyla.

**Kabul:** Düz prizma, eğik düzlem, işaret değiştiren yüzey, hole/no-data ve material katman örnekleri bağımsız analitik sonuçlarla geçer; kamera/LOD/GPU tier değişimi final miktarı değiştirmez.

## 16. ROAD-HAUL — Brükner, malzeme dengeleme ve taşıma maliyeti — P1/P2

Trimble'ın resmi mass-haul iş akışı borrow/waste kapasitesi, malzeme ve engel/taşıma kısıtlarını birlikte ele alır. KentOS için öneri, basit net hacim grafiğinin üzerinde ayrı kısıtlı optimizasyon modelidir. [Mass-haul](https://help.fieldsystems.trimble.com/tbc/10433.htm), [haul roads](https://help.fieldsystems.trimble.com/tbc/9956_1.htm)

- [ ] **ROAD-HAUL-01:** Station boyunca cumulative cut/fill grafiği gross malzeme türü ve bank/loose/compacted bazını açık alsın.
- [ ] **ROAD-HAUL-02:** Mass-haul network zone, borrow/waste/processing site, capacity, barrier, haul road ve direction typed kaynakları taşısın.
- [ ] **ROAD-HAUL-03:** Haul cost birimi currency/(volume-state·distance) olsun; kullanılacak volume state ve kilometre/metre açık çevrilsin.
- [ ] **ROAD-HAUL-04:** Geology/material suitability ve contamination/unsuitable malzeme fill'e otomatik transfer edilmesin.
- [ ] **ROAD-HAUL-05:** Barriers, köprü geçişi, temporary road closure ve erişilebilirlik constraints taşıma planını ayırsın.
- [ ] **ROAD-HAUL-06:** Haul network bağlantısı yalnız plan çizgilerinin ekranda kesişmesi sayılmasın; gerçek source/destination/topology gereksinimleri denetlensin.
- [ ] **ROAD-HAUL-07:** Min-cost-flow veya seçilen optimizer feasibility/optimality gap ve solver version ile raporlansın; heuristic optimum diye sunulmasın.
- [ ] **ROAD-HAUL-08:** Borrow/waste kapasite ve iş takvimi değişince etkilenen taşıma planı stale olsun.
- [ ] **ROAD-HAUL-09:** Kısıtlı kapasite, one-way taşıma, malzeme dönüşüm verimi ve yeniden işleme cost'u alternatif analizde olsun.
- [ ] **ROAD-HAUL-10:** Maliyet/karbon/truck trip hesabı katsayı, taşıt payload ve varsayımlarını görünür tutsun; otomatik mevzuat/şantiye onayı üretmesin.
- [ ] **ROAD-HAUL-11:** Brükner grafiği, plan ağı, station tables ve 3D materyal bölgeleri aynı HaulPlan'ı kullansın.
- [ ] **ROAD-HAUL-12:** CSV/XLSX/report çıktısı approved quantity/haul revisions ve aktarım birimlerini taşısın.

## 17. ROAD-JUNC — Kavşak, bağlantı, köprü/tünel ve araç zarfı — P1/P2

- [ ] **ROAD-JUNC-01:** Kavşak source alignment refs, branch station ve shared control geometry ile kalıcı model olsun; 2D polygon union yetmez.
- [ ] **ROAD-JUNC-02:** At-grade junction ve grade-separated crossing farklı connectivity taşısın; bridge altında kalan yol graph'ta bağlanmasın.
- [ ] **ROAD-JUNC-03:** Intersection approach profiles, crowns ve superelevation continuity target constraints olarak çözülsün.
- [ ] **ROAD-JUNC-04:** Curb returns, roundabout, channelized islands ve ramp transitions reusable civil templates olsun.
- [ ] **ROAD-JUNC-05:** Yol birleşiminde corridor ownership/clip priorities ve material overlap/double volume kontrolü uygulanmalı.
- [ ] **ROAD-JUNC-06:** Design vehicle wheelbase/steering/speed ve articulated configuration kaynaklarını parametreli zarf analizine ver.
- [ ] **ROAD-JUNC-07:** Swept-path ile basit bounding box farklı validation yöntemleri olsun; clearance sonucu kullanılan kinematic model ve sampling tolerance taşısın.
- [ ] **ROAD-JUNC-08:** Bridge deck, retaining wall, tunnel envelope ve underground utilities ayrı 3D structure/source refs taşısın.
- [ ] **ROAD-JUNC-09:** Cover/clearance/checks ana terrain datum ve ilgili corridor/utility revisions ile bağlı olsun.
- [ ] **ROAD-JUNC-10:** Kavşak editinden sonra bağlı approach bölgelerini incremental rebuild et; bütün ağı gerekmeden yeniden çözme.
- [ ] **ROAD-JUNC-11:** İleri traffic capacity/operational simulation ayrı provider ve modeldir; yol geometrisinin doğru görünmesi trafik analizinin doğruluğu sayılmasın.
- [ ] **ROAD-JUNC-12:** Otokorkuluk: dolgu yüksekliği, şev ve engel mesafesine göre giriş–çıkış uzunlukları ve korkuluklar arası en az aralık standart profilinden hesaplansın; sonuç km aralıkları tablosu, şevli kotlu plana ve enkesite işlenen bileşen olsun.

## 18. ROAD-DRAIN — Drenaj, altyapı ve afet bağlantısı — P1/P2

- [ ] **ROAD-DRAIN-01:** Ditch/gutter/inlet/pipe/manhole/culvert asset IDs ve topology'yi corridor point/link code'larına bağla.
- [ ] **ROAD-DRAIN-02:** Invert/crown/ground elevation, diameter/section, roughness, material ve cover/crossing constraints typed olsun.
- [ ] **ROAD-DRAIN-03:** Surface watershed/flow accumulation kaynak çözünürlükte çalışsın; terrain hole/no-data ve conditioning işlemi ayrı provenance taşısın.
- [ ] **ROAD-DRAIN-04:** Road embankment, bridge/culvert ve drainage links sonraki flood domain hazırlığında barrier/opening olarak export edilebilsin.
- [ ] **ROAD-DRAIN-05:** Hydraulic solver adapter girdi/sonuç units, CRS, time, boundary conditions ve solver version taşısın.
- [ ] **ROAD-DRAIN-06:** İlk aşamada HEC-RAS/SWMM gibi harici doğrulanmış iş akışlarının input/output bağlantısını araştır; renderer'ın su göstermesini analiz sayma.
- [ ] **ROAD-DRAIN-07:** Dynamic flood results için [3d.md](3d.md) SimulationRun/FieldDataset sözleşmesini kullan; model mesh'i render mesh'iyle karıştırma.
- [ ] **ROAD-DRAIN-08:** Taşkın nedeniyle yol kapanması karar kuralı depth/velocity/time/vehicle class ve uncertainty'yi açık alsın.
- [ ] **ROAD-DRAIN-09:** Subsurface network clash için gerçek 3D nearest/clearance query kullan; plan kesişimini çakışma kabul etme.
- [ ] **ROAD-DRAIN-10:** Saha bakım, culvert kontrol ve as-built ölçülerini [form.md](form.md) task/asset/row izinleriyle ilişkilendir.
- [ ] **ROAD-DRAIN-11:** Standart menfez tipleri (borulu, kutu, palye hendeği) ve istinat/iksa duvarı türleri parametreli bileşen kitaplığı olsun; boyutlandırma ve metraj kuralı (KGM pozları) sürümlü ve kaynaklı tablodan gelir; menfezin değişmesi bağlı kesit ve metrajı eskitir.

Hidrolik model kapsamı ve gerekli fiziksel denklemler ayrı doğrulanır. [HEC-RAS equation selection](https://www.hec.usace.army.mil/confluence/rasdocs/r2dum/6.4/running-a-model-with-2d-flow-areas/shallow-water-or-diffusion-wave-equations), [EPA SWMM](https://www.epa.gov/water-research/storm-water-management-model-swmm)

## 19. ROAD-GPU — GPU hesap adayları, precision ve backend kapıları — P0/P1/P2

| İş | Önerilen GPU kapsamı | Kesinleşme kapısı |
|---|---|---|
| 3D terrain/cloud/corridor | Render/culling/LOD/visibility/selection | Görüntü ve picking precision korpusu |
| Cloud→grid/filter | Paralel bins/min/reduce, morphological passes | Source-scale CPU reference ve tile halo equivalence |
| TIN | Sort/candidate spatial search; ileri triangulation deneyi | Robust topology, constraints, monolithic/seam doğrulaması |
| Section/corridor | Batch sampling ve türev render geometry | Analitik FP64 source/event validation |
| Cut/fill | Pair candidates, clip/reduce preview | Exact source domain ve kontrollü final integral |
| Flood fields | Interpolation/contours/advection view; ileri solver | Ayrı fizik/accuracy/stability testleri |

Taşınabilir shader sayısal zemini FP32'dir; `f16` ayrı yetenek ve doğruluk profilidir. [WGSL, 21 Eylül 2026 taslağı](https://www.w3.org/TR/2026/CRD-WGSL-20260921/). GPU mimarisi ve hi/lo sınırlarının tam sözleşmesi [3d.md](3d.md)'dedir.

- [ ] **ROAD-GPU-01:** İşlem descriptor'ı render/compute/authoritative certification ve CPU/GPU backend kapsamını ayrı bildirsin.
- [ ] **ROAD-GPU-02:** Point/class filter, histogram, grid minima, raster/slope, bounds ve corridor tessellation adaylarını profile ile seç; önce maliyet ve transfer ölç.
- [ ] **ROAD-GPU-03:** GPU broad-phase TIN/volume intersection adayları üretirken conservative bounds kullansın; false negative final geometriyi düşürmesin.
- [ ] **ROAD-GPU-04:** GPU sort/scan/reduction için bounded buffers, workgroup/dispatch limits, multi-pass ve overflow ret politikası oluştur.
- [ ] **ROAD-GPU-05:** Terrain/corridor GPU payload tile-local FP32/quantized olsun; large world matrix multiply'dan önce origin/camera subtraction yapılsın.
- [ ] **ROAD-GPU-06:** Hassas predicate ve final volume doğrulama için CPU exact-sign/FP64 baseline'ı koru; GPU double mevcut değil diye kararlar epsilon'a indirgenmesin.
- [ ] **ROAD-GPU-07:** Native FP64/64-bit atomics/subgroup gibi hız yolları explicit feature olarak keşfedilsin; portable WGSL FP32 yoluna zorunlu bağlanmasın.
- [ ] **ROAD-GPU-08:** Uzun compute job render'a time slice bıraksın; display ve compute aynı VRAM bütçesini scheduler'da paylaşsın.
- [ ] **ROAD-GPU-09:** Job cancel chunk/dispatch sınırlarında gözlensin; stale source veya denied grant sonuçları commit'te yeniden kontrol edilsin.
- [ ] **ROAD-GPU-10:** Preview reduction nondeterminism/error band görünür olsun; final receipt seçilmiş deterministic algorithm/backend'i kaydetsin.
- [ ] **ROAD-GPU-11:** GPU unavailable/device lost/OOM ve fallback sonuçları typed olsun; kaynak değişmeden staged artifacts temizlenebilsin.
- [ ] **ROAD-GPU-12:** GPU kernel A/B prototipleri küçük transfer-heavy işte CPU'nun daha hızlı çıkabileceğini ölçsün; her primitive'ı GPU'ya taşımayı başarı ölçüsü yapma.

## 20. ROAD-UX — Plan/profil/kesit/3D çalışma deneyimi — P1

- [ ] **ROAD-UX-01:** Senkron plan, profile, cross-section ve 3D viewport aynı selection/engineering source ID'lerini kullansın.
- [ ] **ROAD-UX-02:** Station scrubber ve cross-section plane görünüşle bağımsız state taşısın; data mutation ayrı komut olsun.
- [ ] **ROAD-UX-03:** Aks/template/dever edits drag sırasında düşük gecikmeli preview; pending/solved/invalid/stale state açık olsun.
- [ ] **ROAD-UX-04:** 3D orbit/pan/fly, cut plane, isolate, x-ray ve exaggeration [3d.md](3d.md) camera/presentation API'sinden gelsin.
- [ ] **ROAD-UX-05:** Nokta/TIN/koridor ölçümünde snap source type, coordinate frame, unit ve accuracy bilgisi gösterilsin.
- [ ] **ROAD-UX-06:** Değişiklik planı etkilenen stations/surfaces/materials/sheets ve final quantity invalidation'ı anlaşılır sunsun.
- [ ] **ROAD-UX-07:** Büyük cloud veya terrain eksikken gösterilen yüzeyin LOD/coverage bilgisi görülsün; unresolved region normal boş arazi gibi sunulmasın.
- [ ] **ROAD-UX-08:** Geometri, metraj, malzeme, source revision ve hesap ledger'ı aynı nesneden açılabilsin.
- [ ] **ROAD-UX-09:** View preset/style/theme ve GPU settings kaynak tasarım/volume sonucunu değiştirmesin.
- [ ] **ROAD-UX-10:** Tasarım standardı ve kullanıcı override farkını inspector/rule summary'de göster; debug implementation ayrıntısını normal kullanıcı iş akışına taşıma.

## 21. ROAD-API — UI/komut/Python/AI ve Processing tamlığı — P0/P1

Önerilen aileler bugün çalışan API iddiası değildir: `civil.surveys.*`, `civil.terrains.*`, `civil.alignments.*`, `civil.profiles.*`, `civil.templates.*`, `civil.corridors.*`, `civil.sections.*`, `civil.quantities.*`, `civil.haul.*`, `civil.stakeout.*`, `jobs.*`. Son adlar mevcut catalog/SDK ADR'sinde kesinleştirilir.

- [ ] **ROAD-API-01:** Survey/source inspect, terrain build/edit/query, alignment/profile, template/dever/corridor, section, volume/haul ve export lifecycle komutlarını aynı kataloğa ekle.
- [ ] **ROAD-API-02:** Her UI yöntem/seçenek için operationId/version, semantic input, output/plan/error, units, supported entity/profile ve backend capability manifest üret.
- [ ] **ROAD-API-03:** Source digests, expected revisions, algorithm/profile version ve idempotency mutating/job komutlarında zorunlu olsun.
- [ ] **ROAD-API-04:** Discover→validate→plan→run→progress/result→commit akışı aynı UI/SDK/MCP business rules ile çalışsın.
- [ ] **ROAD-API-05:** Python generated typed wrappers ve AI schema aynı contracts kaynağından çıksın; UI'daki hesap için final triangle/coord üretmek kullanıcıya kalmasın.
- [ ] **ROAD-API-06:** High-volume input typed binary batches/source handles ile taşınsın; milyonlarca nokta tek JSON/MCP cevap veya AI prompt'una yüklenmesin.
- [ ] **ROAD-API-07:** Job outputs layer/table/surface/material/source artifact/file ve sink commit/provenance bilgisi taşısın.
- [ ] **ROAD-API-08:** Plan/validate no-write olsun; draft preview CPU/GPU kaynaklarına erişirken production mutation yapılmasın.
- [ ] **ROAD-API-09:** Trusted local script ve cloud scoped worker izinleri ayrı; source read/download/export ile project write grants ayrı doğrulansın.
- [ ] **ROAD-API-10:** AI natural-language tasarımı parametreli draft/plan üretsin; tasarım standardı, source quality, units ve unknown datum'u uydurmasın.
- [ ] **ROAD-API-11:** AI metraj cevabı gerçek approved QuantityRun/ledger'dan gelsin; ekrandaki tahmini cloud mesh hacmi finalmiş gibi anlatılmasın.
- [ ] **ROAD-API-12:** Hazır Rust civil araçları Python kurulmadan UI/command/AI'da kullanılabilsin; custom [kek eklentileri](python.md) aynı descriptors'a bağlansın.
- [ ] **ROAD-API-13:** Headless GPU/CPU capabilities ve offline source coverage makinece keşfedilebilir olsun; eksik runtime için structured refusal üret.
- [ ] **ROAD-API-14:** Coverage ve behavior conformance fixture'ları CI'da source/schema/SDK/MCP drift'i zorunlu hata yapacak şekilde bağlansın.

**Örnek job isteği (öneri):** `mode=plan` production no-write'tır. Source refs gerçek immutable kaynaklara çözülür; actor/grant doğrulanmış oturumdan gelir. `validated-overlap` yalnız ortak domain'i hesaplarken excluded/no-data alanını raporlar; proje kalite politikası eksik coverage'ı final için ayrıca bloke edebilir. Kamera veya render LOD parametresi bu miktar isteğinde yoktur.

```json
{
  "format": "kentos.civilJob",
  "schemaVersion": 1,
  "operation": "civil.quantities.compute",
  "operationVersion": 1,
  "mode": "plan",
  "projectRef": "project:example",
  "expectedProjectRevision": "42",
  "idempotencyKey": "example-earthworks-run-001",
  "input": {
    "existingSurfaceRef": "terrain:existing@7",
    "designSurfaceRef": "corridor:design-surface@3",
    "areaRef": "area:excavation@1",
    "method": "tin-overlay",
    "lengthUnit": "m",
    "volumeUnit": "m3",
    "volumeBasis": "in-situ",
    "sourceResolution": "full",
    "domainPolicy": "validated-overlap",
    "errorPolicyRef": "project.quantity-error-policy"
  }
}
```

## 22. ROAD-IO — Kalıcılık, bulut ve mühendislik dosya alışverişi — P0/P1

IFC 4.3 resmi şeması yol ve alignment nesnelerini kapsar; reader/exporter profilinin gerçek desteği yine ayrı yapılmalıdır. [IfcRoadDomain](https://standards.buildingsmart.org/IFC/RELEASE/IFC4_3/HTML/ifcroaddomain/content.html), [IfcAlignment](https://standards.buildingsmart.org/IFC/RELEASE/IFC4_3/HTML/lexical/IfcAlignment.htm)

- [ ] **ROAD-IO-01:** Civil sources/relations/standards ve result refs için `.kcad` manifest sürümü ve eski reader ret/yeni reader migration politikası yaz.
- [ ] **ROAD-IO-02:** Çok büyük point/TIN/corridor verileri chunked asset store'da; projede digest/source/coverage manifest'leri olsun. Portable bundle ve external reference mode ayrı seçilsin.
- [ ] **ROAD-IO-03:** Save/export/duplicate/checkpoint/restore aynı kaynak grafını, CRS/datum ve material standard refs'i korusun.
- [ ] **ROAD-IO-04:** Cloud worker digest/tenant/grant ile yalnız ilgili source subsets'i alsın; private raw survey gereksiz paylaşılmasın.
- [ ] **ROAD-IO-05:** LandXML round-trip'te line/arc/spiral/profile/surface/units destek matrisi ve bağımsız tüketici kabulü kur.
- [ ] **ROAD-IO-06:** IFC 4.3 alignment/road ve coordinate referencing adaylarını metadata/segment loss report ile dene; yalnız mesh export'u parametrik road exchange sayma.
- [ ] **ROAD-IO-07:** DGN/DWG/native Netcad proje biçimlerini lisans ve publicly documented codec durumuyla araştır; destek verilmemiş formatı mevcut ithalat gibi listeleme.
- [ ] **ROAD-IO-08:** LAS/LAZ/COPC export original dimensions, scale/offset, CRS ve class edits'i korusun; density/display LOD yalnız explicit export seçeneği olsun.
- [ ] **ROAD-IO-09:** DEM/COG/mesh/UGRID result export units/nodata/time/domain metadata'yı taşısın; 3D display exaggeration kaynağa yazılmasın.
- [ ] **ROAD-IO-10:** Asset dependency ledger ve missing-source repair/remap akışı olsun; proje adı/dosya adı gerçek source digest'in yerine geçmesin.
- [ ] **ROAD-IO-11:** Staged files, atomic replace, job checkpoints ve resume source-revision kontrolüyle kurtarma yap.
- [ ] **ROAD-IO-12:** Şantiye/aplikasyon raporunda template/alignment/terrain/quantity revisions ve üretim zamanı bulunmalı.
- [ ] **ROAD-IO-13:** Aplikasyon için güzergâh, profil ve kesit bilgisini ölçüm cihazlarına (Leica, Topcon, Trimble, Nikon) yazmak mevcut Cihaza gönder yolunu (ADR 0169) genişletsin; her biçimin desteklediği eleman türleri belgeden çıkarılsın, taşınamayan eleman (ör. ove spiral) neden ile söylensin.
- [ ] **ROAD-IO-14:** DWG/DGN'deki yol projelerinin (polyline aks, profil çizimi) KentOS aksına dönüştürülmesi ayrı araçtır: aliyman ve kurplar uydurularak çıkarılır, sapma raporlanır; Netpro'nun bu yönde bir aracı vardır, KentOS'ta kaynak çizgi korunur.

## 23. ROAD-QA — Bağımsız doğruluk ve performans kapıları — P0/P1

- [ ] **ROAD-QA-01:** Analitik line/arc/clothoid/parabolic profile, station equation ve station↔point query referanslarını KentOS algoritmasını kopyalamayan yüksek doğruluk hesabıyla kur.
- [ ] **ROAD-QA-02:** TIN topology/constraint/holes/seams için exact-sign ve bağımsız geometric invariant tests oluştur; tek golden mesh görselini kanıt sayma.
- [ ] **ROAD-QA-03:** Known prism/ramp/mixed-sign volumes ve farklı station sampling convergence korpusunu oluştur.
- [ ] **ROAD-QA-04:** Ground filtering/classification bağımsız checkpoints ve farklı yoğunluk/terrain/vegetation örnekleriyle doğrulansın.
- [ ] **ROAD-QA-05:** GPU/CPU conformance mutlak/bağıl tolerans, allowed nondeterminism ve certificate metadata bakımından test edilsin.
- [ ] **ROAD-QA-06:** Native/web/headless desteklenen profiller ortak fixtures'e girsin; unavailable host işaretli capability olsun.
- [ ] **ROAD-QA-07:** Cancel/crash/stale result/device lost/OOM/idempotent retry ve staged export cleanup fault injection ile sınansın.
- [ ] **ROAD-QA-08:** Bir delta ile changed station ranges ve bütün downstream invalidation doğru olsun; açık derived result yeni onaylıymış gibi görünmesin.
- [ ] **ROAD-QA-09:** Gerçek yol ofisinin survey→terrain→alignment→profile→dever→corridor→volume→sheets/stakeout günü kabul korpusuna çevrilsin.
- [ ] **ROAD-QA-10:** İthal/dışa aktarılan dosyalar bağımsız uygun okuyucular ve engineering metadata karşılaştırmasıyla sınansın.
- [ ] **ROAD-QA-11:** Benchmark source point/triangle counts, density, class mix, viewport resolution, camera path, hardware/driver/backend, RAM/VRAM ve cold/warm cache bilgisi taşısın.
- [ ] **ROAD-QA-12:** FPS yanında p50/p95/p99 frame time, input latency, first coherent view, bytes read/upload, job throughput ve final error raporlansın.
- [ ] **ROAD-QA-13:** İlk hedef interaktif 60 Hz profili olabilir; 16.7 ms kare bütçesi hedef olarak ölçülür. Donanım/veri profili seçilmeden milyar nokta/FPS garantisi yazma.
- [ ] **ROAD-QA-14:** Mühendislik onay kapısı data loss, yanlış datum/unit, yanlış source revision, hatalı gross volume veya izin aşımı gördüğünde durmalı.

## 24. Zorunlu kabul korpusu

| ID | Senaryo | Beklenen kanıt |
|---|---|---|
| R01 | TM büyük koordinatta küçük arazi detayı | Source FP64 korunur; 3D/pick precision profili geçer |
| R02 | İki bulut farklı vertical datum'dadır | Onaylı dönüşüm olmadan ortak volume finalleşmez |
| R03 | Ground/noise/bridge/vegetation sınıfları | QA/coverage ve checkpoint sonucu; bridge deck ground'a karışmaz |
| R04 | Breakline ve hole içeren TIN | Constraints korunur; gap/nodata alanı doldurulmaz |
| R05 | Tile sınırında ince hendek ve breakline | Seam ve monolithic reference aynı kaynak anlamını verir |
| R06 | Near-collinear/cocircular/duplicate XY | Kararlı topology; çelişkili Z açık karar |
| R07 | Line–clothoid–arc güzergâhı | Point/tangent/curvature ve station doğru |
| R08 | Station equation aynı km label'ını iki kere verir | Branch-aware query; yanlış nokta seçilmez |
| R09 | Crest/sag profil ve farklı çizim ölçekleri | Kot/eğim değişmez; independent profile reference |
| R10 | Dever + widening + template transition | Sürekli kaynak geometry, doğru sign ve component lineage |
| R11 | Terrain daylight hit yok/çoklu hit | Çözülemeyen aralık ve typed warning; sahte mesh yok |
| R12 | Profilin 100 m parçası değişir | Lokal rebuild; ilgili quantity/sheets stale olur |
| R13 | 10×10 m AOI, d=x−5 | Fill=125 m³, cut=125 m³, net=0 |
| R14 | TIN edges farklı, hole/no-data vardır | Exact domain overlay; excluded coverage raporu |
| R15 | AEA/prismoidal/raster/TIN kıyası | Yöntem ve sampling error açıklanır; yanlış eşdeğerlik iddiası yok |
| R16 | Material bank/loose/compacted | Dönüşüm bazları/factors doğru; double counting yok |
| R17 | Borrow capacity + one-way haul + barrier | Feasibility/cost ve malzeme conservation ledger |
| R18 | Grade-separated crossing | Aks/traffic/utility graph yanlış bağlanmaz |
| R19 | Sadece cloud LOD resident'tır | Full-source kesit/volume pending veya tamamı okunur; eksik analiz hazır sayılmaz |
| R20 | GPU FP32 preview cihazlar arası farklıdır | Error bound; final QuantityRun doğruluğu ve provenance korunur |
| R21 | Job cancel/OOM/crash/stale revision | Belge tutarlı; artifacts ve retry kontrollü |
| R22 | UI/Python/MCP aynı proje | Aynı semantic kaynak/validation/izin/sonuç |
| R23 | LandXML/IFC/portable `.kcad` yeni host | Desteklenen metadata/geometry korunur; kayıp raporu görünür |
| R24 | Road culvert flood modeline gider | Datum/domain/boundary/topology doğru; render mesh solver ağı yerine kullanılmaz |

## 25. Geliştirme sırası ve ilk dikey dilim

| Faz | Teslim | Bağımlılık ve çıkış |
|---|---|---|
| RD0 — Araştırma/contracts | Kaynak envanteri, numeric/source contracts, bağımsız korpus ve 3D bağlantısı | CAD/GIS/form devam eder; cihaz/standart/korpus kararları |
| RD1 — Arazi + ilk 3D | Persistent TIN, known DEM/cloud, elevation query, terrain 3D ve exact picking | 3d.md D0–D2; R01–R06 |
| RD2 — Analitik yol | Line/arc/clothoid + profile + station equations; synchronized views | R07–R09; aynı SDK/AI |
| RD3 — Koridor | Template, targets, dever/widening, daylight, adaptive sampling, incremental build | R10–R12, R18 |
| RD4 — Miktar + pafta | Verified volumes/materials, section sets, sheets/stakeout, interchange | R13–R16, R22–R23 |
| RD5 — Büyük veri/GPU | Node streaming, compute prototypes, chunked terrain jobs, bounded resources | R19–R21; precision/performance gates |
| RD6 — Saha + ileri civil | Haul, junction/structures/drainage, as-built ve merkezi workflow | R17–R18; yerel mühendislik doğrulama |
| RD7 — Twin/afet entegrasyonu | Asset identity, temporal observations, solver source export ve verified result view | 3d.md twin/simulation fazları; R24 |

GPU render ve source precision RD1'den itibaren vardır; RD5 hızlandırma genişlemesidir. Robust CPU baseline'ı olmadan speculative GPU TIN veya final volume ilk teslimin önkoşulu yapılmaz.

### 25.1 Uygulama sırası (2 Kasım 2026)

Yukarıdaki fazlar hedef kapsamdır. Uygulama, 3B sahne beklenmeden, her biri kullanıcının masaüstünde gördüğü bir sonuçla biten maddelerle yürür. Her madde ayrı ADR'dir (0900'den), kendi aşamalarına bölünür, her aşama kendi commit'i ve ekran görüntüsüyle biter. Madde başlamadan o konunun güncel resmî belgeleri yeniden araştırılır ve bu tabloya not düşülür.

| Sıra | Madde | Kapsadığı işler | Görünen sonuç |
|---|---|---|---|
| Y1 | Modül ve proje | ROAD-MOD-01…05, 08; ROAD-NUM-01…04 ve 11'in sözleşmesi | `kentos-yol` açılır, kendi şeridi ve proje ağacıyla; yeni yol projesi sihirbazı (sistem, düşey datum, standart profili yok/var); Launcher kartı |
| Y2 | Arazi yüzeyi | ROAD-TIN-01…11, 15, 16; ROAD-SURVEY-01, 03, 08; ROAD-NUM-10 | Noktalardan, kırık hatlardan, sınır ve boşluklardan kısıtlı Delaunay yüzey; nokta kotu sorgusu, eş yükselti eğrileri, eğim; LandXML/CSV/XYZ/nokta bulutundan girdi |
| Y3 | Yatay güzergâh | ROAD-ALIGN-01…10, 15, 16; ROAD-NUM-05 | Aliyman–klotoid–kurp zinciri, PI'dan ya da elemandan; km, km eşitliği, km ve offset sorgusu, paralel aks; Güzergah editörü ve planda km yazısı |
| Y4 | Düşey güzergâh ve boykesit | ROAD-PROFILE-01…06, 09, 10, 12, 13 | Arazi boykesiti (üçgen kenarlarıyla kesin), some'lerle parabolik ve dairesel düşey kurplar, kırmızı/siyah kot, boykesit görünüşü |
| Y5 | Enkesit ve tip kesit | ROAD-SEC-01…08, 13; ROAD-COR-01…04, 10 | Kesit hattı takımı (üç yöntem), arazi ve platform kesiti, şerit–banket–hendek–şev tip kesiti, daylight; enkesit görünüşü |
| Y6 | Dever ve genişleme | ROAD-DEVER-01…12 | Dever çizelgesi ve diyagramı, kesitlerde enine eğim, kurp genişlemesi (standart profili varsa önerili) |
| Y7 | Koridor | ROAD-COR-05…17; ROAD-MOD-06 | Koridor, platform ve formasyon yüzeyi, şevli kotlu plan, dinamik yeniden üretim |
| Y8 | Kübaj | ROAD-VOL-01…21 | Kübaj kuralları, kesit (AEA/prismoidal) ve yüzey (TIN bindirme) hacmi, sıyırma ve diş kazısı, kabarma/sıkışma, kübaj cetveli |
| Y9 | Brükner ve taşıma | ROAD-HAUL-01…12 | Brükner diyagramı, ödünç/döküm alanları, taşıma planı |
| Y10 | Pafta ve alışveriş | ROAD-SEC-09…11; ROAD-IO-05, 06, 13, 14 | Plan–profil ve enkesit paftaları, aplikasyon cetveli, LandXML ve IFC 4.3 güzergâh, cihaza gönderme |
| Y11 | Sanat yapıları ve güvenlik | ROAD-DRAIN-01, 02, 11; ROAD-JUNC-12 | Menfez ve duvar bileşenleri, otokorkuluk aralıkları, metraj |
| Y12 | Ön güzergâh, kavşak, 3B | ROAD-ALIGN-13; ROAD-JUNC; 3d.md bağlantısı | Koridor analizi (yasak alanlar), kavşak, 3B sahne |

**Y1'in aşamaları** (araştırma 2 Kasım 2026: OpenRoads'un iş akışı sekmeleri Home, Terrain, Geometry, Site, Corridors, Model Detailing, Drawing Production soldan sağa iş sırasıyla dizilir, [Bentley şerit yardımı](https://docs.bentley.com/LiveContent/web/OpenRoads%20Designer-v2024.0.2/Help/en/topics/1296840/GUID-40CF9525-AB56-47BF-AF51-63D75333BB61.html), [CTDOT giriş](https://portal.ct.gov/dot/engineering-applications/ctdot-connect-dde-ebook-volume-003---openroads-designer-roadway-modeling/section-1---introduction); Netpro editörleri tek pencereden ve proje ekranından yönetir):

1. **Y1.1 Modül:** `CadWork::Road` (`.kcad`'de opt-in), `Module::Road` (`yol`, KentOS Yol, `kentos-yol`), bulut kataloğunun `road` türü Harita'dan Yol'a geçer, Launcher kartı açılır, sihirbazda “Yol projesi” (metre, 1:1000, ilin sisteminde). Ekran: Launcher ve boş `kentos-yol` penceresi.
2. **Y1.2 Şerit:** CAD'in ortak sekmeleri (Dosya, Giriş, Ekle, Açıklama, Değiştir, Görünüm, Yönet, Çıktı) ve yolun ilk sekmesi Arazi (ölçüm, karne, GNSS, nokta bulutu, raster altlık); Harita'nın Plan sekmesi ve Mimari'nin sekmesi yolda yoktur; Ölçme Harita ile ortak. Yolun öbür sekmeleri kendi maddesiyle gelir; boş sekme ya da sessiz düğme konmaz. Ekran: dar ve geniş, açık ve koyu.
3. **Y1.3 Yol projesinin ayarları:** sihirbazda ve Proje ayarları'nda düşey datum (mevcut `heights` ayarı, ADR 0519/0631) ve standart profili (yok ya da kataloğun profili); profil yokken “standart denetimi yok” sözü. Ekran: sihirbaz ve ayarlar.

Proje ağacı (ROAD-MOD-04) ve bölünmüş plan–boykesit–enkesit görünüşü (ROAD-MOD-05) gösterecek kaynak oluşunca Y2–Y5'te gelir.

**Y1'in durumu (2 Kasım 2026, dal `road-01-yuzey`):** Y1.1 `20b252b2e` (`CadWork::Road`, `.kcad` şema 158, `Module::Road`, Launcher kartı, sihirbazın Yol projesi kartı, bulut kataloğunun `road` türü Yol'da), Y1.2 `dec3c4e54` (şerit: Arazi sekmesi, Ölçme ve altlıklar Harita ile ortak, seçim sekmelerinde arazi işlemleri, hidroloji ve uzaklık/maliyet), Y1.3 `417bd05b2` (yeni yol projesi TUDKA-99 ortometrik kotlar ve TG-20 geoidiyle; sihirbazda Kotlar; Python `cad_work`, MCP `drawing.new` `cadWork`). ROAD-MOD-02'nin standart profili ROAD-DEC-01'i (resmî KTEK kopyası) bekler; ROAD-MOD-03'ün öbür sekmeleri kendi maddeleriyle gelir. Ajan bağlantısı testleri (`every_program_opens_a_link_of_its_own`, MCP `desktop_attach_names_the_program_whose_link_it_takes`) macOS'ta ana dalda da düşer: geçici soket yolu SUN_LEN'den uzun.

**Y2'nin aşamaları** ([ADR 0900](../docs/adr/0900-road-terrain-surface.md); araştırma 2 Kasım 2026: OpenRoads'un nokta, kırık hat, sınır, boşluk ve kenar yöntemi; Civil 3D'nin kırık hat türleri; Netcad'in üçgen model akışı; Shewchuk–Brown ve Sloan'ın kısıt ekleme yöntemi):

1. **Y2.1 Çekirdek:** `kentos-road` crate'i; kısıtlı Delaunay (noktalar, kırık hatlar, drape sınır ve boşluklar, en uzun dış kenar), aynı yer ve kot çelişkisi retleri, kot sorgusu; Triangle ve kesirli Python denetleyicisiyle ortak durumlar. Ekran: örnek yüzeylerin resmi.
2. **Y2.2 Kayıt:** yüzeyin kaynakları (katmanlar, nesneler, ayarlar) ve sonucu `.kcad`'de; kaynak değişince yüzey “eskidi”.
3. **Y2.3 Komutlar:** `road.terrains.create`, `.edit`, `.query` (kot, kesit), Python `kentos.road`, MCP; manifest.
4. **Y2.4 Masaüstü:** Arazi sekmesinde Yüzey oluştur aracı, yüzeyin çizimi (üçgenler, kırık hatlar, sınır), Öznitelikler ve proje ağacının ilk hâli (ROAD-MOD-04).
5. **Y2.5 Türevler:** TIN'den eş yükselti eğrileri (ROAD-TIN-15), eğim ve bakı (ROAD-TIN-16), LandXML yüzeyi okuma.

**Y2'nin durumu:** Y2.1 `e5dacbad0`: `kentos-road` crate'i, `kentos.cdt/1`; 14 ortak durum Triangle ve kesirlerle, 400 rastgele durum Triangle'la üçgen üçgen aynı; 104 000 köşe 98 ms, 100 000 profil kotu 1,4 ms (release). Y2.2 (bu commit): `kentos_contracts::RoadSettings` (`road.surfaces`), `.kcad` şema 158'de (yalnız yol projesinde; Python okuyucusu ve başvuru yazıcısı, `broken/road-settings-of-a-map-project.kcad`), `kentos_road::sources::gather` (kaynakların okunması, kullanılamayanların nedeni, özet ve güncel/eskidi/yeni durumu; ADR 0900 §10). Y2.3: `road.terrains.create`, `.edit`, `.query` (ADR 0900 §11), 69 ortak durum Triangle ve kesirlerle, Python `kentos.road`, MCP. Y2.4: Arazi sekmesinde Yüzeyler ve Yüzey oluştur, Arazi yüzeyleri penceresi, yüzeyin çizimde üçgenleri ve kırık hatları (ADR 0900 §12; proje ağacı ROAD-MOD-04 açık). Y2.5: `road.terrains.contours` (ADR 0900 §13), sorguda eğim ve bakı, pencerede Eş yükselti eğrileri; `file.import.landxml` (ADR 0900 §14, 26 durum), pencerede LandXML al…; Y2 tamam. Y2.2 kararı: yüzeyin tanımı (kaynak katmanlar ve nesneler, sınır, boşluklar, tolerans, kenar yöntemi) projede saklanır, üçgenleri kaynaklardan açılışta yeniden kurulur (kesin ve hızlı); kaynakların özeti tutulur, özet değişince yüzey “eskidi”; `.kcad` şema 158 (henüz birleşmedi) yol projesinin bu ayarlarını da kapsar.

**Y3'ün aşamaları ve durumu** ([ADR 0901](../docs/adr/0901-road-horizontal-alignment.md)): Y3.1 `cd14a2331`: `kentos_road::alignment` (aliyman, kurp ve klotoid tek analitik biçimde, `kentos.alignment/1`; PI'dan eşit olmayan klotoidler, km ve km eşitlikleri, noktadan km ve öteleme bütün ayaklarıyla, paralel aks ve sivri ucu, düzenlemenin km eşlemesi), 26 durum mpmath'le 40 basamakta. Y3.2 `e2b2ee023`: `road.alignments.create`, `.edit` (set, reverse, remove), `.query` (km'de yer, km yazısından yer, yuvarlak km, nokta, zemin boyu ROAD-NUM-05); akslar geri alma geçmişinde tutulan proje ayarı (`Op::RoadAlignments`), `.kcad` yol ayarlarında; 33 + 27 + 20 durum, Python, MCP. Y3.3 `1dfea2ae8`: Güzergâh sekmesi ve penceresi (PI tablosu, km eşitlikleri, paralel aks, yazıldıkça kurulan rapor, noktanın km'si), planda aks, km işaretleri ve kurp noktaları. Y3.4 `7277fec5e`: `road.alignments.label` (km işaretleri, km yazıları, uçlar ve kurp noktaları çizime nesne olarak, paftaya ve DXF'e gider; 27 durum), pencerede Km'yi çizime yaz. Açık: elemanla düzenleme (sabit, yüzen, serbest), ove klotoidin PI'dan kurulması, LandXML aksları (ROAD-ALIGN-14), tasarım standardı (ROAD-ALIGN-11), 3B uzunluk (Y4'te boykesitle).

**Y4'ün aşamaları ve durumu** ([ADR 0902](../docs/adr/0902-road-vertical-alignment-and-profile.md)): Y4.1 `2672e0a1d`: aksın bir yüzeydeki arazi boykesiti (`kentos.profile/1`): üçgen kenarlarını kestiği her yer kesin, kurp boyunca doğrusal okumada 1 mm sınırı, yüzeyin dışı boşluk; 6 durum Triangle ve mpmath'le. Y4.2 `b26e5e8f1`: düşey güzergâh (PVI'ler arasında eğimler, simetrik ve asimetrik parabol, km–kot düzleminde gerçek çember; K, tepe ya da çukur, en yüksek ya da alçak nokta); 21 durum kesirler ve mpmath'le. Y4.3 `35e736a69`: `road.profiles.create`, `.edit` (set, rebind, remove), `.query` (rapor, kırmızı ve siyah kot, fark, eğim, arazi boykesiti); boykesit aksının revizyonunu tutar, aks değişince eskir; geri alma geçmişinde (`Op::RoadProfiles`), `.kcad` yol ayarlarında; boykesiti olan aks silinmez; 34 + 24 + 23 durum, Python, MCP. Y4.4 `315e31dcc`: Boykesit penceresi (Güzergâh sekmesinde Boykesit paneli): PVI tablosu, arazi ve proje çizgisi düşey abartmayla, K değerleri, km/mesafe/siyah/kırmızı/fark/eğim bantları, Aksa yeniden bağla. Y4.5 `d6c6704a3`: `road.profiles.draw` (boykesit çizime nesne olarak, paftaya ve DXF'e gider; 27 durum), pencerede Çizime yaz. Açık: görüş mesafesi (ROAD-PROFILE-08), köprü ve menfez kısıtları (07), kırmızı kot önerisi (14), standart profilinden kurp boyu önerisi (13, sahibin KGM kopyasını bekler), grafikte PVI'yi fareyle sürükleme, 3B uzunluk.

**Y5'in aşamaları ve durumu** ([ADR 0903](../docs/adr/0903-road-cross-sections-and-templates.md)): Y5.1 `cdb49f504`: kesit hattı (aksın km'sinde aksa dik, sol ve sağ şerit genişliği) ve arazi kesiti (üçgen kenarlarında kesin, yüzeyin dışı boşluk); 4 durum Triangle ve mpmath'le. Y5.2 `7a31bd368`: tip kesit (şerit, banket, yarma ve dolgu şevi, yarmada hendek), menteşede yarma ya da dolgu, kesin daylight, açık yan, yarma ve dolgu alanı; 20 durum kesirlerle. Y5.3 `6d89c25be`: `road.templates.edit`, `road.sections.edit` (range, stations, lines; set, rebind, remove), `road.sections.query`; takımın kullandığı tip kesit, boykesit ve aks silinmez; 32 + 36 + 23 durum, `.kcad` şema 160 (main 159'u aldı), Python, MCP. Y5.4 `b6386ddc6`, `3e3258e5f`: `road.sections.draw` (görünüşler ızgarada, 31 durum) ve Enkesit penceresi (Güzergâh sekmesinde Enkesit paneli). Açık: tip kesitte bordür, duvar ve çok katmanlı üstyapı, şevde basamak, dever (Y6), kübaj (Y8), kesit hattının başka aksa kenetli genişliği, enkesit görünüşünde bantlar.

**Y6'nın aşamaları ve durumu** ([ADR 0904](../docs/adr/0904-road-superelevation-and-widening.md)): Y6.1 `6bd5c46dd`: `kentos_road::superelevation` (orta eksen etrafında döndürme, NC, LC, RC, FS, iç yan genişlemesi, km satırları); 17 durum kesirlerle. Y6.2 `8ff3497d7`: `road.superelevations.edit` ve `.query`, kesit takımının `superelevation`'ı (şeritler yanın eğimini, son şerit genişlemeyi alır); 33 + 20 durum, `.kcad` şema 160, Python, MCP. Y6.3: Dever penceresi (çizelge, diyagram yazdıkça, kurulmayanın nedeni, Bütün kurplar, Aksa yeniden bağla; Güzergâh sekmesinde Dever paneli) ve Enkesit'te takımın Dever seçimi. Açık: iç ya da dış kenar etrafında dönme, banketin kırılma kuralı, standart profilinden öneri, ters ve birleşik kurpta çakışmanın çözümü, bölünmüş yol.

**Y7'nin aşamaları ve durumu** ([ADR 0905](../docs/adr/0905-road-corridors.md)): Y7.1 `4ad9fad85`: `kentos_road::corridor` (bölgeler, km örneklemesi: aliyman aralığının katları, kurp ve klotoidin aralık ya da kiriş sapmasıyla eşit bölünmesi, geometri, profil, dever kritik ve kullanıcı km'leri; öznitelik çizgileri; şerit üçgenlemesiyle üst yüzey; çözülmeyen ve katlanan aralıklar); 26 durum. Y7.2 `b4e6aad8b`: `road.corridors.edit`, `.query`; sonuç her okumada kurulur, kaynak değişince rapor eskir (ROAD-MOD-06'nın ilk yüzü); 35 + 22 durum, `.kcad` şema 160, Python, MCP. Y7.3 `5ece6c106`: `road.corridors.draw` şevli kotlu plan; 26 durum. Y7.4: Koridor penceresi ve Koridor sekmesi. Açık: datum ve formasyon yüzeyi (üstyapı katmanları), hedefe bağlı genişlik ve kot, artımlı kurma ve kendiliğinden yeniden üretim, birden çok taban çizgisi, 3B katı model.

**Y8'in aşamaları ve durumu** ([ADR 0906](../docs/adr/0906-road-earthwork-volumes.md)): Y8.1 `f0fb6b328`: `kentos_road::volume` (ortalama alan, prizmoidal, iki yüzey arası hacim: üçgenler planda üst üste, `d`'nin işaret değiştirdiği doğruda bölünür, örtülmeyen alan ayrı); 14 yüzey durumu ve kesit durumları kesirlerle. Y8.2 `290e9594c`: `road.volumes.edit` ve `.query` (kübaj cetveli: satırlar, katsayılar, birikimli düzeltilmiş toplamlar, kütle ordinatı, plan alanları, hacimsiz aralıklar); kübajın kullandığı koridor silinmez; koridorun şerit üçgenlemesinde eşit köşegen 1 nm içinde; 30 + 22 durum, `.kcad` şema 161'in `volumes`'ı, Python, MCP. Y8.3: Kübaj penceresi (`volume_window.rs`; `volume.editor`, `volume.new`, `volume.export`; Koridor sekmesinde Kübaj paneli): kübaj yazdıkça hesaplanır, kütle diyagramı, toplamlar, uyarılar, cetvel; Cetveli yaz XLSX, ODS ya da CSV (ADR 0223). Y8.4: sıyırma (zemin bitkisel toprak kalınlığı kadar indirilir, bitkisel toprak ayrı kalem) ve diş kazısı (eşikten dik zeminde dolgunun altında basamak, genişlik × h/2), pencerede ve cetvelde. Y8.5: `road.volumes.approve` (onaylı hâl tanımda; onaylı kübaj değişmez, kaynakları değişince fark ve `volume_changed`), pencerede Onayla ve Onayı kaldır. Açık: bileşik kesit hacmi ve kurp düzeltmesi, yüzeyler arası yöntemde sıyırma, diş kazısının kütle dengesine katılması, onayın kim tarafından yapıldığı (bulut kimliği).

**Y9'un aşamaları ve durumu** ([ADR 0907](../docs/adr/0907-road-mass-haul.md)): Y9.1 `dc6da68f7`: `kentos_road::haul::diagram` (kütle eğrisi katman katman: denge noktaları, döngüler, dengelenen hacim, taşıma momenti, serbest ve fazla taşıma, dengesiz uçlar döküm ya da ödünç); 21 durum kesirlerle. Y9.2: `road.hauls.edit` ve `.query` (kübajın Brükneri, denge çizgisi, serbest taşıma, ödünç ve döküm alanları basamak olarak), kübajın kullandığı Brükner silinmez; 28 + 22 durum, `.kcad` şema 161'in `hauls`'u, Python, MCP. Y9.3: Brükner penceresi (Koridor sekmesinde Brükner paneli; eğri, döngüler, denge noktaları, alanların basamakları, döngüler tablosu). Açık: maliyet ve taşıma planı (Y9.4).

Her maddenin kabulünde CLAUDE.md §9.2 geçerlidir: anlamsal komut, ortak durumlar (KentOS kodu olmadan yazılmış bağımsız başvuru), Python (`kentos.road`) ve MCP, manifest; aracın yazması kayıtta komutun adıyla; dar ve geniş pencerede, iki temada gerçek masaüstü görüntüleri (repo dışında `.run/gui-evidence/`).

İlk 12 iş (taslak 0.1; 25.1'in Y1–Y10'u bunların yerini alır):

1. Mevcut Delaunay/predicates/cloud/raster/renderer kaynaklarına ADR ve source ownership eşlemesi çıkar.
2. Coordinate/vertical datum/quality ve Surface/Alignment/Job source contracts'ını kesinleştir.
3. Analitik doğruluk ve FP32 büyük koordinat corpus'unu hazırla.
4. 3d.md'deki orbit/orthographic camera, terrain depth ve precision ilk dilimini prototiple.
5. Küçük persistent TIN + elevation/section query ve schema/persist yolunu tamamla.
6. Ground cloud subset ve breakline'lı yüzey üretimini kalite raporuyla bağla.
7. Analitik line/arc/clothoid ve station equations'i bağımsız sınayarak SDK'ya aç.
8. Parabolic profil ve synchronous plan/profile/3D görünüşünü ekle.
9. Basit iki şerit + banket + ditch/daylight template koridoru üret.
10. Flat/ramp/mixed-sign AOI hacmini verified TIN overlay ile raporla.
11. Plan/profil/enkesit paftası ve LandXML/portable proje kabulünü yap.
12. Ölçülen darboğaza göre ilk GPU compute kernel'ini seç; benchmark/error raporunu source/model doğruluğuyla birlikte yayınla.

## 26. ROAD-DEC — Açık kararlar ve araştırma denemeleri

- [ ] **ROAD-DEC-01:** MVP veri türleri ve karayolu kapsamı; ilk yerel DesignStandard kaynağı/baskısı/onay sahibi seçilsin. — **2 Kasım 2026:** ilk kapsam karayolu; ilk standart kaynağı KGM KTEK 2022 (ve KTŞ 2023), ücretli basılı olduğu için değerler sahibin sağladığı resmî kopyadan girilir. Standart profili dosyası `catalog/v1/road-standards/` altında kaynak, baskı, sayfa ve onaylayanla tutulur; profil yokken araçlar çalışır, standart denetimi “profil yok” der.
- [ ] **ROAD-DEC-02:** Persistent terrain asset/topology/block storage ve constrained triangulation genişletme yöntemi/lisansı kararlaştırılsın.
- [ ] **ROAD-DEC-03:** 2.5D terrain ile multi-surface/solid ayrımı ve bridge/vertical-wall source politikası seçilsin.
- [ ] **ROAD-DEC-04:** Clothoid/numeric integration ve station-equation mapping policy bağımsız prototiple doğrulansın.
- [ ] **ROAD-DEC-05:** Template expression/constraint dili ve component version/target adapter politikası seçilsin.
- [ ] **ROAD-DEC-06:** Final volume error tolerance, source survey kabulü ve deterministic certification yolu mühendislik kararıyla belirlensin.
- [ ] **ROAD-DEC-07:** Native GPU tier ve portable WebGPU compute baseline, optional features ve fallback sözleşmesi seçilsin.
- [ ] **ROAD-DEC-08:** Chunked TIN partition/merge/seam algoritmasının correctness proof/validation planı yazılsın.
- [ ] **ROAD-DEC-09:** İlk reference cihazlar, dataset corpus, 60 Hz/latency ve job throughput hedefleri ölçümle sabitlensin.
- [ ] **ROAD-DEC-10:** LandXML/IFC/DGN/native exchange supported profile ve loss report stratejisi ayrı ADR olsun.
- [ ] **ROAD-DEC-11:** Material conversion factors, quantity approval ve haul optimizer objective/constraints yetkili ürün modeliyle seçilsin.
- [ ] **ROAD-DEC-12:** Simülasyonla yol domain coupling ve source ownership, solver bağımlılıkları ve data upload politikası [3d.md](3d.md) ile ortaklaştırılsın.

**Tamamlandı kapısı:** Kaynak sözleşme ve migration, bütün UI/command/Python/AI seçenekleri, izin/transaction/kalıcılık, bağımsız numerical tests, desteklenen host/backends, büyük veri ve gerçek ofis kabulü birlikte raporlanır. Mevcut veya hedef olan test, gerçekten çalıştırılan testten ayrı yazılır. Araştırma dokümanındaki işaretler uygulama kanıtı olmadan `[x]` yapılmaz.

## 27. Araştırma kaynakları ve sonraki güncelleme

Aşağıdaki resmi kaynaklar 13 Ekim 2026'da incelendi; ürün destekleri/modül/lisans/sürümler uygulama öncesi yeniden kontrol edilir. Önceki bölümdeki karşılaştırma ürünlerin kamuya açık iş akışıdır; özel shader, exact predicate veya hacim algoritmalarının kodunu görmüş olduğumuz anlamına gelmez.

| Kaynak | Kullanım |
|---|---|
| [Netcad Netpro](https://www.netcad.com/tr/urunler/netpro) | Yerel civil iş akışı, dinamik tasarım/kesit/çıktı |
| [Netcad NETPRO yardım ve yenilikler](https://wiki.netcad.com.tr/display/HELP/NETPRO), [Netpro genel özellikler](https://wiki.netcad.com.tr/pages/viewpage.action?pageId=275350930) | Netpro 12: KTEK 2022, dinamik revizyon, kübaj kuralları, sıyırma/diş kazısı, otokorkuluk, menfez ve duvar, koridor analizi (2 Kasım 2026) |
| [KGM Yayınlar](https://www.kgm.gov.tr/Sayfalar/KGM/SiteTr/Yayinlar/Yayinlar.aspx) | KTEK 2022 ve KTŞ 2023 ücretli basılı yayın (2 Kasım 2026) |
| [Civil 3D 2026 yenilikler](https://help.autodesk.com/cloudhelp/2026/ENG/Civil3D-WhatsNew/files/GUID-1777B37B-2DDA-459B-8CDF-AD17B9331C75.htm), [geçiş eğrileri](https://help.autodesk.com/cloudhelp/2024/ENG/Civil3D-UserGuide/files/GUID-DD7C0EA1-8465-45BA-9A39-FC05106FD822.htm) | LandXML'de desteklenmeyen eğri uyarısı; spiral türleri |
| [FHWA ORD şablon kitaplığı](https://highways.dot.gov/sites/fhwa.dot.gov/files/docs/federal-lands/cadd-support/ord-user-manual/13331/ord-manual08-template-library_2.pdf), [FHWA ORD dever](https://highways.fhwa.dot.gov/sites/fhwa.dot.gov/files/docs/federal-lands/cadd-support/ord-user-manual/13266/ord-manual10-superelevation.pdf) | OpenRoads'un şablon ve dever iş akışı |
| [IFC 4.3 güzergâh geometrisi](https://ifc43-docs.standards.buildingsmart.org/IFC/RELEASE/IFC4x3/HTML/concepts/Product_Shape/Product_Geometric_Representation/Alignment_Geometry/content.html) | Yatay bileşik eğri, IfcGradientCurve, IfcSegmentedReferenceCurve (cant) |
| [Trimble Access geçiş türleri](https://help.fieldsystems.trimble.com/trimble-access/2023.10/en/RXLroad-transitions.htm), [NILIM LandXML 1.2](https://www.nilim.go.jp/lab/qbg/researchfields/pdf/h28_basedLandXML1.2.v1.0_English.pdf) | Saha cihazının spiral türleri; LandXML örnek yapısı |
| [Netcad nokta bulutu](https://www.netcad.com/tr/nokta-bulutu) | Netcad ekosisteminde cloud/3D/ölçme uygulamaları |
| [Bentley MicroStation](https://www.bentley.com/products/microstation) | CAD/3D platform rolü |
| [Bentley OpenRoads](https://www.bentley.com/en/products/openroads-designer/) | Terrain/template/corridor/dever/earthworks kapsamı |
| [OpenRoads öğrenme kılavuzu](https://www.bentley.com/wp-content/uploads/OpenRoads-Designer-Learn-Guide.pdf) | Geometry/terrain/template çalışma bölümleri |
| [Civil 3D corridor/miktar](https://www.autodesk.com/learn/ondemand/module/create-preliminary-road-quantities-and-schedules-in-civil-3d) | Source surface ve material üretimi |
| [Civil 3D hacim yöntemleri](https://help.autodesk.com/cloudhelp/2018/ENU/Civil3D-UserGuide/files/GUID-1E3A9CD9-DFD6-4455-9EB1-D8E46721A0B4.htm) | AEA/prismoidal/composite ayrımı; eski sürüm belgesi, güncel davranış tekrar denenecek |
| [12d alignment](https://12d.com/product/12d_model_alignment.html) | Parametrik kaynak/string/modifier ilişkileri |
| [Trimble mass-haul](https://help.fieldsystems.trimble.com/tbc/10433.htm) | Materyal/kapasite/taşıma kısıtları |
| [COPC 1.0](https://copc.io/) | Range/chunk/hierarchy ve source dimensions |
| [PDAL SMRF](https://pdal.org/en/stable/stages/filters.smrf.html) | Sınıflandırma algoritması ve parametre kıyası |
| [CGAL triangulation](https://doc.cgal.org/latest/Triangulation_2/index.html) | Constrained/robust topology ve intersection ayrımı |
| [Shewchuk predicates](https://www.cs.cmu.edu/~quake/robust.html) | Adaptive exact-sign hesap |
| [USGS doğruluk değerlendirmesi](https://pubs.usgs.gov/publication/ofr20251017/full) | Survey/ground checkpoint bağımsız doğrulama |
| [KGM yayın listesi](https://www.kgm.gov.tr/Sayfalar/KGM/SiteTr/Yayinlar/Yayinlar.aspx) | Geçerli tasarım kriterleri için seçilecek resmi yayın; bu belge limit tablosu çıkarmaz |
| [IFC road](https://standards.buildingsmart.org/IFC/RELEASE/IFC4_3/HTML/ifcroaddomain/content.html), [alignment](https://standards.buildingsmart.org/IFC/RELEASE/IFC4_3/HTML/lexical/IfcAlignment.htm) | Infrastructure exchange semantiği |
| [WGSL 2026-09-21](https://www.w3.org/TR/2026/CRD-WGSL-20260921/) | Portable shader precision tabanı |
| [iTwin.js display](https://www.itwinjs.org/learning/display/) | Backend tile üretimi/frontend render ayrımı |

Sonraki revizyonlar source/ürün sürümü, benchmark corpus, karar ADR'si ve değişen kabul kapsamını bir revizyon günlüğünde kaydeder. Teknik prototip sonucu ürün claim'i veya mevzuat uygunluğu sayılmaz; doğrulama kapısı ve yetkili standard seçimi ayrıca tamamlanır.
