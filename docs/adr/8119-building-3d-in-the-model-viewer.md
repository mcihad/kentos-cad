# ADR 8119: Binanın 3B görünümü görüntüleyicinin içinde: ağaç, özellikler, kesit; model çizimden canlı

- **Durum:** uygulandı (18 Kasım 2026). Plan: [bim-plan.md](../../plan/bim-plan.md) BIM-06 (3B görünüm),
  [ifc-uyum.md](../../plan/ifc-uyum.md) IFC-12 (görüntüleyici). Sahibin bildirimi: 3B görünümde sağdaki ve soldaki
  bölmeler yoktu. Sahibin seçimi: “3B görünüm'e paneller”. Mimari çizimde 3B görünüm görüntüleyicinin bileşeniyle açılır;
  model çizimden canlı kurulur. Yol çizimlerinin 3B görünümü değişmez.
- **Bağlam:** ADR [8067](8067-building-3d-view-and-room-overlaps.md) §9 (binanın 3B görünümü),
  [8073](8073-ifc-viewer-first-slice.md) (görüntüleyici), [8118](8118-ifc-viewer-layout-refinement.md) (yerleşimi),
  [8101](8101-underlay-models.md) (altlık modeller), [8115](8115-model-mapping-off-the-window-thread.md) (eşleme pencerenin
  iş parçacığı dışında), [8117](8117-persistent-global-ids.md) (kalıcı GlobalId); DESIGN.md §7.20–7.21.
- **Web, `.kcad`, komut:** değişmedi. `scene3d.view` aynı komuttur; yeni ayar yok.

## §1. Sorun

İki ayrı 3B pencere vardı:

- **IFC modeli** bir dosyayı okuyordu. Solda ağaç ve arama, sağda özellikler, sahnede kesitin kartı vardı (ADR 8118).
- **3B görünüm** çizimin binasını gösteriyordu. Yalnız şeridi, sahnesi ve durum çubuğu vardı. Gövdeleri `kentos_ifc::mesh`
  ile, dosya yazılmadan yapılıyordu.

Sahip çizimin 3B görünümünü açınca bölmeleri arıyordu. Bir nesnenin özelliğini görmek için önce IFC dışa aktarmak, sonra
dosyayı IFC modelinde açmak gerekiyordu. Kesit, arama ve gizleme yalnız dosya penceresindeydi.

## §2. Araştırma (18 Kasım 2026)

| Kaynak | Ne diyor | KentOS kararı |
|---|---|---|
| Autodesk Revit, [Isolate Elements in a 3D View with a Selection Box](https://help.autodesk.com/cloudhelp/2022/ENU/Revit-DocumentPresent/files/GUID-0BBD875D-0E36-4FA4-A997-D6E3573CA3FF.htm) | Seçilen öğeler 3B görünümde kesit kutusuyla açılır; kutu 3B görünümün Özellikler paletinden açılıp kapanır | Çizimin 3B görünümünde kesit ve “Seçimin çevresinde kutu” görüntüleyicinindir (K242) |
| Graphisoft Archicad, [Info Box](https://help.graphisoft.com/AC/20/INT/AC20Help/Appendix_Settings/Appendix_Settings-21.htm) ve [Navigator Preview](https://helpcenter.graphisoft.com/guides/archicad-21/archicad-21-reference-guide/interaction/navigation/navigation-in-the-3d-window/navigator-preview-3d) | Bilgi kutusu seçilen öğenin ayarlarını gösterir; gezgin ve 3B pencere aynı modelin görünüşleridir | Seçilen nesnenin özellikleri sağda, planla aynı seçim (K246) |
| Bonsai, [Exploring an IFC model](https://docs.bonsaibim.org/studio/quickstart/explore_model.html) | Proje ağacı sahadan kata, mahale; seçilen nesnenin sınıfı, nitelikleri, özellik ve miktar setleri; listelerde arama; kata göre yalnız bırakma ve gizleme | Ağaç, arama, kat gözü ve özellik tablosu görüntüleyicide vardı; çizimin modeline de verildi |
| KentOS kılavuzu §6.2, §8, §23; ADR 8115 K220 | Uzun iş pencerenin iş parçacığında koşmaz; sabit renk ve yazı eklenmez; gösterim kaynağı değiştirmez | Model okuma kopyasından ayrı iş parçacığında kurulur (K244); kitin jetonları |

## §3. Karar

- **K242 — Bina görüntüleyicide:** yol çizimi dışında 3B görünüm görüntüleyici bileşeninin kendisidir: solda ağaç ve arama,
  sağda özellikler, sahnede kesitin kartı, sürüklenen bölmeler, simgeli anahtarlar (DESIGN.md §7.21). Pencere, şeridin
  sonundaki “Katlar ▾” ve ayrıyken “Ana pencereye al” ile onu barındırır. Çizimin üstündeyken genişliği 1280 px'tir; sahne
  iki bölmeye rağmen yolunkiyle aynı yeri bulur. Yol çiziminin 3B görünümü değişmez.
- **K243 — Model dosyanın okunduğu yoldan:** dışa aktarımın o an yazacağı IFC (`arch_ifc::building`) bellekte yazılır.
  IFC modelinin okumasıyla (`ifc_viewer::read_bytes`) gövdeleri ve açıklıklarıyla geri okunur. Böylece 3B'de görünen
  dosyadakidir. Özellik tablosu dosyanın setlerini, tiplerini ve ölçülerini gösterir. Eski yol (`kentos_ifc::mesh`) 3B
  görünümden kalktı. Dosyanın tuttuğu emsal zonları (`IfcSpatialZone`, ADR 8069) mahaller gibi saydam hacimdir.
- **K244 — Ayrı iş parçacığında, canlı:** model okuma kopyasından ayrı bir iş parçacığında kurulur. Pencere açıkken çizim
  (nesil, sürüm) ya da gösterilen altlıklar değişince yeniden kurulur. Kurulurken bir değişiklik daha olursa ardından bir
  kez daha kurulur. Pencere kapanınca iş durur, sonucu gösterilmez. İlk model gelene dek bölmelerin yerinde “Çizimin
  modeli kuruluyor” yazar. Duvarı, döşemesi, kolonu, kirişi, çatısı, merdiveni, rampası ya da korkuluğu olmayan çizim
  pencere açılmadan söylenir. Model kurulduğunda gövdesi yoksa (bütün gövdeler silinmişse) pencere kapanır ve nedenini söyler.
- **K245 — Görünüm yerinde kalır** (`Viewer::reload`, `View::carried`): yeniden kurulan modelde kamera, ağacın açık dalları,
  sekme, arama, kesit, anahtarlar ve bölme genişlikleri kalır. Kesit düzlemi yeni modelin aralığına kısılır. Gizlenen,
  yalnız bırakılan, seçilen ve bulgulu nesneler anahtarlarıyla (GlobalId) kalır. Anahtarlar aynı çizimde aynıdır
  (ADR 8117), bu yüzden düzenleme görünümü dağıtmaz.
- **K246 — Planla birlikte:** planın ilk seçili nesnesi görüntüleyicide seçilir ve özellikleri okunur. Görüntüleyicide
  seçilen planda seçilir. Boşluğa tıklamak planın seçimini boşaltır; eski 3B görünüm de böyleydi. IFC modeli
  boşaltmaz, çünkü dosya çizimin olmayabilir. Denetim'in bulguları kırmızıdır; durum çubuğunda “N nesnede bulgu” yazar,
  ipucu kırmızının ne olduğunu söyler.
- **K247 — Altlıklar yanında, ayrı:** gösterilen altlık modellerin nesneleri görüntüleyicinin girdisine eklenir
  (`Input::append`). Anahtarları `altlik:<kimlik>:` önekini alır; iki dosyanın GlobalId'leri karışmaz. Parçaları
  “Altlık model”dir: ADR 8101'in menekşesi, artık çekirdekte (`parts::UNDERLAY`). Ağaçta projeleri “<ad> (altlık model)”
  diye durur. Tıklanan altlık nesnesi Altlık modeller penceresinde gösterilir; planın seçimi kalır. Okunurken
  yerleştirilmiş gövdeleri kullanılır, yeniden okunmaz.
- **K248 — Katlar ▾ kalır:** şeridin sonundaki menü ağaçtaki katın gözüyle aynı işi yapar. Katın adı çizimin adıdır
  (“Zemin kat”). “Mahaller” görüntüleyicinin anahtarıdır.
- **K249 — Yeni renk ve yazı yok:** kitin jetonları; parça renkleri görüntüleyicinin çekirdeğindedir (kılavuz §8).
- **K250 — Zon başvuran katın altında:** bir yapının içermeden başvurduğu nesne
  (`IfcRelReferencedInSpatialStructure`; katın emsal zonu) başka bir yere bağlı değilse ilk başvuranın altına asılır.
  Ağaçta katın altında durur; kat gizlenince o da gizlenir. Önce ağacın kökündeydi: “Katlar 1/2”de gizlenen katın
  zonları sahnede kalıyordu (ilk resimde görüldü). Kural `kentos_ifc::read::view`'dedir, IFC modeline de geçerlidir.
- **K251 — Sığdırma sahnenin biçimine göre:** sahne bileşeni boyutunu ilk karede ve her değişince bildirir
  (`Gesture::Size`). Eni boyundan kısa sahnede modelin küresi dar açıya sığdırılır (`OrbitCamera::fit_in`). İki bölme
  sahneyi daraltınca bina yanlardan kırpılıyordu (1100 × 700 resmi). Model ilk karden önce yüklenirse o karede bir kez daha
  sığdırılır; kullanıcı önce döndürmüş ya da kaydırmışsa sığdırılmaz. Geniş sahnede sığdırma eskisi gibidir; yolun 3B
  görünümü kendi kamerasını korur.

## §4. Uygulama

- `crates/shared/model-view`: `Input::{append, apart, is_apart}`; `View::carried` (anahtarlarla taşınan durum);
  `parts::UNDERLAY` (“Altlık model”); `OrbitCamera::{fit_in, look_in}`.
- `crates/model-view-ui`: `Viewer::reload`; sahnenin boyutu (`Gesture::Size`) ve ilk sığdırma.
- `crates/shared/ifc/src/read/view.rs`: başvurulan nesne başvuranın altında.
- Masaüstü `shell/scene3d_building.rs` baştan yazıldı: `Building3d` görüntüleyiciyi, modelleri (çizimin ve altlıkların),
  kurulumu, planın seçimini ve bulguları tutar. `shell/scene3d_window.rs`: `Event::{Viewer, Made}`, binanın olayları
  görüntüleyicinin iletilerine; `open_scene3d` ve `follow_scene3d` iş döndürür. `shell/scene3d_view.rs`: binada
  görüntüleyicinin yerleşimi.
- `shell/ifc_viewer.rs`: `read_bytes` (dosya ya da bellek), `viewer_host` (iki pencerenin ev sahibi), `viewer_control`
  (izlerin denetimleri).
- DESIGN.md §7.20.

## §5. Sınamalar, ölçüm ve resimler

- Masaüstü (`scene3d_building_tests`): kesit binası 58 yapı elemanı, 10 mahal ve 4 emsal zonuyla dosyanın tuttuğu 72
  gövdeli nesnenin hepsini gösterir; ağaç ve şema durum çubuğunda. Duvara tıklamak planda seçer, ayrıntısı okunur; boşluk
  seçimi boşaltır; planın seçimi 3B'de görünür. Silinen duvar ve kolonla model yeniden kurulur: kamera, arama, kesit ve gizli
  kat yerinde. Örtüşen iki mahal kırmızı, “2 nesnede bulgu”. Altlık: menekşe parça, ağaçta “(altlık model)”, tıklanan
  nesne Altlık modeller'de.
- Çekirdek (`kentos-model-view`): eklenen modelin anahtarları ve parçası; yeniden okunan modelde görünümün anahtarlarla
  kalması; eni boyunun yarısı olan sahnede eski sığdırmanın kutunun köşelerini kırptığı, yenisinin kırpmadığı, geniş
  sahnede ikisinin aynı olduğu.
- IFC (`kentos-ifc`, `view`): kesit binasının dört emsal zonu katlarının altında, kökte zon yok.
- Ölçüm ([`ifc_timing`](../../crates/native/headless/tests/headless/ifc_timing.rs), release, Apple M5, 18 Kasım 2026): 3B
  görünümün kendi iş parçacığındaki işi model eşlemesi, yazma ve gövdeleriyle geri okumadır. Pencere yalnız okuma
  kopyasını öder.

  | Daire | Nesne | Okuma kopyası | Eşleme | Yazma | Geri okuma ve gövdeler | Toplam | Gövde |
  |---|---|---|---|---|---|---|---|
  | 1 | 44 | 0,04 ms | 4,4 ms | 0,6 ms | 4,4 ms | ≈ 9 ms | 37 |
  | 20 | 842 | 0,03 ms | 31,9 ms | 4,5 ms | 58,3 ms | ≈ 95 ms | 721 |
  | 100 | 4 202 | 0,09 ms | 217,0 ms | 23,3 ms | 286,1 ms | ≈ 0,53 s | 3 601 |

  100 dairede bir düzenlemeden sonra model yarım saniyede yenilenir; bu sürede eski model görünür kalır.
- Resimler: `.run/gui-evidence/bina-3b-8067/`.

## §6. Sıradaki adımlar

- Büyük çizimde yalnız değişen nesneleri yeniden okumak (bugün bütün model yazılıp okunur).
- Plandan seçilen nesnenin ağaçta dallarının açılması (bugün yalnız sahnede tıklanınca açılır).
- Emsal zonlarının ayrı bir anahtarla gösterilmesi (bugün mahallerle birlikte).
- Aksların (`IfcGrid`) çizgi olarak gösterilmesi: bugün “1 gösterilemeyen” olarak sayılır.
