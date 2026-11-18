# ADR 8120: 3B görünümün çizim kalitesi: kenar yumuşatma, kenar çizgileri, ortam gölgesi ve ayarı

- **Durum:** uygulandı (18 Kasım 2026). Plan: [ifc-uyum.md](../../plan/ifc-uyum.md) IFC-12 (görüntüleyici),
  [bim-plan.md](../../plan/bim-plan.md) BIM-06 (binanın 3B görünümü). Sahibin isteği: görüntüleyicinin çizim kalitesi
  çok düşük; kalite Uygulama ayarlarından değiştirilebilmeli.
- **Bağlam:** ADR [0910](0910-preliminary-route-junction-and-3d.md) §5 (sahnenin çizicisi), [8073](8073-ifc-viewer-first-slice.md) §2–§3
  (görüntüleyici, kesit), [8118](8118-ifc-viewer-layout-refinement.md), [8119](8119-building-3d-in-the-model-viewer.md);
  DESIGN.md §7.20–7.21.
- **Web, `.kcad`, komut:** değişmedi. Yeni ayar: `graphics.scene3dQuality` (bu cihaz, yalnız masaüstü).

## §1. Önceki hâlin gözden geçirmesi

`.run/gui-evidence/3b-kalite-8120/once-*.png` resimleri:

- **Kenar yumuşatma yoktu:** sahne pencerenin hedefine pikselde tek örnekle çiziliyordu. Çatının, kirişin ve pencere
  çerçevesinin eğik kenarları basamaklıydı.
- **Kenar çizgisi yoktu:** aynı tondaki iki gövde (kolon ile duvar, kiriş ile döşeme) birbirine karışıyordu. Bir yüzün
  bittiği yer ancak ışık farkından okunuyordu.
- **Işık tekti:** omuz üstünden gelen bir ışık ve sabit bir ortam payı vardı. Gölgedeki iki yüz aynı koyulukta kalıyordu.
  Çatı duvar kadar griydi.
- **Köşe gölgesi yoktu:** pencere girintisi, duvarın döşemeye oturduğu yer ve iç köşeler düz görünüyordu.

## §2. Araştırma (18 Kasım 2026)

| Kaynak | Ne diyor | KentOS kararı |
|---|---|---|
| Autodesk Viewer, [Viewer Settings](https://help.autodesk.com/cloudhelp/ENU/ADSKVIEWER-Help/files/AutodeskViewerTools/ADSKVIEWER_Help_AutodeskViewerTools_ViewerSettings_html.html) | Görünüş sekmesinde kenar yumuşatma ve ortam gölgesi; yapılandırmada kenarları göster; gölge etkisini kapatmak gezinmeyi hızlandırır | Üç düzey: kenar yumuşatma, kenar çizgileri, ortam gölgesi; yavaş aygıt alt düzeyi seçer |
| Autodesk Revit, [Graphics options](https://help.autodesk.com/cloudhelp/2016/ENU/Revit-Customize/files/GUID-64944142-3544-4CCC-A97B-B306DA8623F7.htm) | Çizgiler kenar yumuşatmayla düzgün; gölgeli görünüşte ortam gölgesi ve kenarları göster seçenekleri | Kenar çizgisi gövdenin kendi kenarıdır, kalite düzeyiyle gelir |
| xeokit, [SAO](https://xeokit.github.io/xeokit-sdk/docs/class/src/viewer/scene/postfx/SAO.js~SAO.html) | Ortam gölgesi kıvrımları, oyukları ve birbirine yakın yüzeyleri koyulaştırır; Scalable Ambient Obscurance makalesine dayanır; yakın nesnelerde maliyet artar | Ortam gölgesi aynı yöntemle; ekrandaki yarıçap 72 px ile sınırlı |
| McGuire, Mara, Luebke, “Scalable Ambient Obscurance” (HPG 2012) | Derinlikten konum ve normal; spiral üzerinde örnekler; (r² − v·v)³ düşüşü; ε = 0,01 | Yöntem bu; ε makaledeki gibi (ilk denemede ε çok küçüktü ve köşeler karardı, §5) |
| WebGPU, çok örneklemli hedefler | Dört örnek renk ve derinlik biçimlerinin hepsinde zorunlu | Dört örnek; aygıt sorgusu gerekmez |
| KentOS kılavuzu §6.2, §8, §20.3 | Sıcak yolda iş azaltılır; sabit renk eklenmez; kaynaklar bırakılır | Kenarlar içerik bir kez yüklenirken bulunur; renkler parçanın renginden; hedefler görünümle gider |

## §3. Karar

- **K252 — Üç düzey:** Uygulama ayarları › Çizim motoru › 3B görünüm › “3B görünüm kalitesi”. Hızlı: pikselde tek
  örnek, yalnız yüzeyler. Dengeli: dört örnek ve gövdelerin kenar çizgileri. Kaliteli: bunlara ek olarak ortam gölgesi.
  Varsayılan Kaliteli. Ayar bu cihazındır (`Device`), yalnız masaüstündedir. Değişince açık görünüm o düzeyde yeniden
  çizilir; içerikler GPU'ya yeniden yüklenir. Binanın 3B görünümü, IFC modeli ve yolun 3B görünümü aynı ayarı izler.
- **K253 — Görünümün kendi hedefleri:** Hızlı'nın üstündeki düzeyde sahne kendi renk ve derinlik hedeflerine çizilir.
  Sonra çözülür ve pencerenin üstüne önceden çarpılmış saydamlıkla bindirilir; gök ev sahibinin çizdiğidir. Pencerenin
  dışına taşan görünüm, hedefinin pencerede kalan parçasını bindirir: izdüşüm eskisi gibi sıkışmaz. Hızlı düzey de aynı
  yoldan geçer.
- **K254 — Kenar çizgileri:** gövdenin iki yüzünün en az 25° döndüğü yerde, açık kenarında ve üç ya da daha çok yüzün
  buluştuğu yerde çizilir. Bir yüzün üçgen köşegeni çizilmez. İki gövde birleşmez, yan yana iki duvar iki çerçeve
  verir. Yumuşak gövdede (arazi) kenar yoktur. Kenarlar içerik yüklenirken bir kez bulunur (`edges::feature_edges`).
  Çizgi, parçanın renginin %62 koyusudur. Yüzünden uzaklığın 0,04'ü kadar öne çizilir: 30 m'de 1 cm. Saydam parçanın
  (mahal, açıklık) kenarı çizilmez; pencerenin camınınki çizilir. Kesit kenarları da keser.
- **K255 — Işık:** omuz üstündeki ışığa iki pay eklendi, ev sahibi verir. Gök yukarı bakanı aydınlatır, aşağı bakanı
  koyulaştırır (binada 0,16). Dolgu ışığın öbür yanından gelir (binada 0,22). İkisi sıfırken ışık eskisi gibidir. Yol
  kendi bakışını korur (ikisi sıfır).
- **K256 — Ortam gölgesi:** opak yüzlerin derinliğinden; yarıçap 0,6 m, güç 0,6, 12 örnek, 4 × 4 dönen desen. Desen
  bindirmede ortalanır. En koyu köşe ışığın %45'ini korur: gölge yalnız ortam ışığını alır. Saydam yüzün altında gölge
  saydamlığı oranında kalkar (yüzlerin ikinci hedefi, “örtü”). Önce camın ardındaki iç mekânın gölgesi camın üstüne
  leke gibi düşüyordu.
- **K257 — Yeni renk yok:** kenar ve gölge parçanın kendi renginden türer (kılavuz §8).

## §4. Uygulama

- `crates/render/scene3d`: `Quality`; `Scene3dRenderer::with_quality`, `ensure_targets`, `place`, `render_offscreen`;
  `edges.rs` (kenarlar), `offscreen.rs` (hedefler, örtü, ortam gölgesi, bindirme); `Frame::{sky, fill}`.
- `shaders/wgsl/scene3d/`: `mesh.wgsl` (ışık, `edgeVs`/`edgeFs`, `meshVeiledFs`), `composite.wgsl`, `occlusion.wgsl`;
  sözleşme `scene3d.layout.json` sürüm 3 (View 208 bayt).
- `crates/model-view-ui`: `SceneView::{quality, sky, fill}`, `Host::quality`; her görünümün çizicisi kalitesi
  değişince yeniden kurulur.
- `crates/shared/contracts`: `graphics.scene3dQuality`; masaüstünde `App::scene3d_quality`, ayar penceresinde
  Çizim motoru › 3B görünüm.

## §5. Sınamalar ve resimler

- Çizici: kutunun 12 kenarı, köşegen yok; yan yana iki kutu 24; 10° kıvrım tek yüzey, 40° kenar; yumuşak gövdede kenar
  yok (`edges`).
- Gerçek GPU (`KENTOS_GPU_TESTS=1`, `the_3d_scene_on_a_real_gpu`): 1 m kutu 4 m döşemede. Hızlı'da yalnız iki gri ve
  beyaz var. Dengeli'de koyu kenar pikselleri ve iki gri arasında ara tonlar var. Kaliteli'de kutunun dibindeki döşeme
  uzaktakinden en az 12 birim koyu, uzaktaki gri değişmez. Ortam gölgesinin ilk denemesinde ε 0,0001 idi; kutunun
  altı ve dibi simsiyah oldu. ε makaledeki 0,01'e çekildi, köşeye %45 taban kondu.
- Gölgelendirici sözleşmesi (`styled_contract`): üç modül naga ile doğrulanır; giriş noktaları ve bağlar sözleşmeninki.
- Masaüstü (`scene3d_quality_tests`): ayar varsayılan Kaliteli; üç değer görünüme ulaşır; adlar Hızlı, Dengeli,
  Kaliteli. 3B, görüntüleyici ve ayar sınamaları yeşil.
- Resimler: `.run/gui-evidence/3b-kalite-8120/` (önce, üç düzey açık ve koyu, GPU denetiminin üç resmi, ayar).

## §6. Sıradaki adımlar

- Kesilen yüzün kapağı (ADR 8073 §9) ve kesitte kenar çizgisi.
- Hareket sırasında düzeyi düşürmek (gezinirken Dengeli, durunca Kaliteli), büyük modelde ölçümle.
- Yarım çözünürlüklü ortam gölgesi ve derinliğe duyarlı bulanıklaştırma.
