# ADR 8131: Görüntüleyicide kesit yüzü (kesilen gövdenin kapağı)

- **Durum:** uygulandı (18 Kasım 2026). Plan: [ifc-uyum.md](../../plan/ifc-uyum.md) IFC-12 (Kalanlar: "kesilen yüzün
  kapağı"), [bim-plan.md](../../plan/bim-plan.md) B12b.
- **Bağlam:** ADR [8073](8073-ifc-viewer-first-slice.md) §3 (kesit düzlemi ve kutusu, çizicide kırpma),
  [8120](8120-3d-render-quality.md) (kalite: örnek sayısı, kenar çizgileri), [8061](8061-ifc-viewer-and-reading-engine.md)
  §3 (gevşek bağ; saf çekirdek yerel ve wasm32); kılavuz §6.2 (sıcak yol), §23.3 (tessellation nihai ölçü değildir).
- **Web, `.kcad`, komut:** değişmedi. Kapak bir görünümdür; çizimi değiştirmez, ürün komutu değildir. Gölgelendiricinin
  sözleşmesi sürüm 4'tür (TODOS.md §0.2).

## §1. Sorun

Kesit düzlemi ve kutusu gövdeleri yalnız kırpıyordu: kesilen duvar, döşeme ve kolon içi boş kabuk gibi görünüyordu,
kesitten içeri bakınca gövdenin arka yüzleri ve öbür yanındaki odalar görünüyordu. Çizilmiş bir kesitte kesilen eleman
doldurulur (poşe); görüntüleyicide neyin kesildiği, kalınlığı ve katmanların nerede birleştiği okunmuyordu.

## §2. Araştırma (18 Kasım 2026)

| Kaynak | Ne diyor | KentOS kararı |
|---|---|---|
| xeokit SDK, [SectionCaps](https://xeokit.github.io/xeokit-sdk/docs/class/src/viewer/scene/sectionCaps/SectionCaps.js~SectionCaps.html) ve [Hatching sliced objects](https://xeokit.io/blog/hatching-sliced-objects-using-xeokit-sdk); yeni SDK'da [buildSectionCaps](https://xeokit.github.io/sdk/docs/api/functions/presentations.sectionCaps.buildSectionCaps.html) | Kapak gövdenin saklanan geometrisinden kurulur (`readableGeometryEnabled`); düzlemi kesen üçgenlerin parçaları; yalnız kapalı (solid) gövde ve `capMaterial`'i olan nesne kapak alır | Kapak işlemcide, gövdenin f64 kaynak ağından kurulur; yalnız kapalı ve saydam olmayan gövde (K306) |
| three.js, `webgl_clipping_stencil` örneği; forumda [stencil capping](https://discourse.threejs.org/t/clipping-plane-stencil-capping/74018) ve [birleşik yüzlerde kayma](https://discourse.threejs.org/t/clipping-geometry-made-from-merged-faces-stenciled-cap-face-not-aligning-properly/19280) | Kapak şablon (stencil) tamponuyla: her düzlem için ayrı geçişler, gövdeler iki yüzlü ve kapalı olmalı; birleşik ve çok gövdeli modelde sorunlar bildiriliyor | Alınmadı: kutunun altı düzlemi için her karede geçiş; kapak gövdesine ait değil (seçilemez, ayrı renk alamaz) |
| Solibri, [Sectioning Settings](https://help.solibri.com/hc/en-us/articles/1500004219142-Sectioning-Settings) | Kesilen bileşenler doldurulur: bileşenin kendi rengiyle ya da tek özel renkle; dolgunun saydamlığı (varsayılan 50) | Tek renk "Kesit yüzü", opak; lejantta (K307). Bileşen renginde ve taramalı kapak sonraya (§6) |
| Revit (kaba poşe rengi), Archicad ("Uniform Surface - Color Fill") | Kesilen elemanların hepsi tek renkle gösterilebilir | Aynı: bütün kapaklar bir parça, bir renk |
| WebGPU CTS, [`multisample_info.ts`](https://chromium.googlesource.com/external/github.com/gpuweb/cts/+/HEAD/src/webgpu/multisample_info.ts) (D3D11'in standart deseni) | 4 örneğin yerleri pikselin sol üstünden (0,375; 0,125), (0,875; 0,375), (0,125; 0,625), (0,625; 0,875) | Gölgelendirici düzlem uzaklığını bu yerlere taşır (K309) |
| WGSL, `@builtin(sample_mask)` çıktısı | Parçacığın yazdığı örneklerin maskesi, örtüyle VE'lenir | Kesilen yüz yalnız kesitin tuttuğu örnekleri yazar (K309) |

Kapağın ölçüsü ve gösterimi ayrı tutulur: kapak gösterilen üçgenlerden kurulur, alan ya da hacim kaynağı değildir
(kılavuz §23.3); miktarlar dosyadaki setlerden okunur (IFC-12).

## §3. Karar

- **K306 — Kapak:** kesitin her düzlemi, kestiği her kapalı ve saydam olmayan gövdeyi o düzlemde doldurur (saf çekirdek
  `kentos_model_view::caps`, yerel ve wasm32). Düzlemi kesen her üçgen bir parça verir; parçalar noktalarından (1e-7 m)
  halkalara zincirlenir, kapanmayan zincir bırakılır (açık kabuğun kapağı yok). Kutunun öbür düzlemleri halkayı kırpar.
  İç içe halkalar tek-çift kuralıyla dolar (bir halkanın içindeki halka delik; deliğin dışı onu tutan en küçük halka);
  ortak kulak kırpıcı (`kentos-geometry-core::triangulate`) üçgenler. Mahal, açıklık, cam ve saydam parça kapak almaz.
  Kapak gövdesinin nesnesini taşır; kapağa tıklayan ışın kesilen yanı atlar ve arkasındaki gövdeye varır, gövde seçilir
  (kapak seçimde de koyu kalır). Ağaçta gizlenen gövdenin kapağı yoktur.
- **K307 — Görünüş:** kapakların parçası "Kesit yüzü": tek koyu gri, opak; kesit varken lejantın son satırı. Düzleminin
  normaliyle yumuşak gölgelenir (bütün kapak bir ton), kenar çizgisi çizilmez. Kesilen tarafa döner (oradan görülür) ve
  o tarafa 0,5 mm kayık durur (`LIFT`): düzlemin tam üstündeki bir yüz (kesitin tam ucuna düştüğü duvar) onunla derinlik
  için yarışmaz.
- **K308 — Çatlaksız:** aynı düzlemdeki kapaklar birbirinin kenarında duran köşelerini alır (0,1 mm içinde; modelin
  gövdeleri float32 üçgenlerdir, birleşmesi gereken köşeler yüzde birkaç milimetre ayrık olabilir). Halkanın yalnız
  köşeleri üçgenlenir; doğru bir kenar boyunca dizilen noktalar (komşunun köşesi, bir yüzün bölünmesi) o kenara dayanan
  üçgenin ortasından yelpazelenir. Böylece komşu kapaklar kenarlarının bütün noktalarını paylaşır, T kavşağı açılmaz.
- **K309 — Çizici:** kapağın parçası kesilmez: stilinin alfası gölgelendiriciye 2 artırılmış gelir (`Style.cut`); kapak
  bir kenar çizgisi kadar yakına çizilir, hemen arkasındaki kenar arkada kalır. Kesilen yüz örnek başına kesilir: her
  düzlemin uzaklığı pikselin ortasından örneğin yerine konumun türevleriyle taşınır, `sample_mask` yalnız kesitin
  tuttuğu örnekleri yazar; görünümün `edge.w`'si pikselin örnek sayısıdır. Piksel başına kesmede ortası tutulan pikselin
  örnekleri düzlemi yarım piksel geçiyordu: Dengeli ve Kaliteli'de ince döşeme bandının kapağına sıra sıra noktalar
  düşüyor, kapağın dış kenarında açık renkli bir saçak kalıyordu. Kapağı büyük kaydırmak (5 cm) noktaları siliyordu ama
  kapağı düzlemden ayırırdı; reddedildi.
- **K310 — Ne zaman:** kapaklar kesit kurulunca, Öbür yanı'nda, kutu kurulunca ve kaydırıcı bırakılınca bir kez
  hesaplanır; kaydırıcı sürüklenirken yalnız kırpılır (akıcı), bırakınca kapak gelir. Ölçüm (M5, release): 9 600 gövde,
  115 200 üçgen; yatay kesit 1 600 gövdeyi 19,8 ms'de, düşey kesit 240'ı 4,9 ms'de, kutu 959 kapağı 25,8 ms'de kapatır.
  Arayüz iş parçacığında kabul edildi; çok büyük model için §6.

## §4. Uygulama

- `crates/shared/model-view/src/caps.rs` (yeni: `caps`, `LIFT`, halka zinciri, birleştirme, doldurma, kırpma);
  `parts.rs` (`CAP`, "Kesit yüzü"); `Cargo.toml` (`kentos-geometry-core`).
- `crates/model-view-ui/src/viewer.rs` (kapaklar `rebuild`'de, `cutting`, `Message::AtDone`, `swatches`);
  `viewer_view.rs` (kaydırıcının bırakılışı, lejant).
- `crates/render/scene3d/src/lib.rs` (`Style { color, cut }`, `Style::of`; stillerin köşe evresinde de okunması;
  `edge.w`); `shaders/wgsl/scene3d/mesh.wgsl` ve `scene3d.layout.json` (sürüm 4: `cuts`, `colour`, `nearer`,
  `keptSamples`, `sample_mask`).
- `apps/desktop/src/shell/scene3d_window.rs`, `crates/render/wgpu/tests/gpu/scene3d.rs` (`Style::of`).

## §5. Sınamalar

- **Çekirdek** (`caps::tests`): 4 × 2 × 3 m blok 1,25'te yatay kesilince kapağı 8 m², 1,25 + `LIFT`'te, yukarı bakar;
  kutuyla kesilince kapakları 2 × 2 + 2 × (2 × 1,25) = 9 m². Uzun bloğun kapağı yanındaki iki kısanın birleştiği köşeyi
  kenarına alır (alanlar korunur). Halka biçimli şaft gövdesinin kapağı 9 − 4 = 5 m² (iç halka delik); cam, saydam parça
  ve açık kabuk kapak almaz. `timing` (elle, release): §3 K310'daki ölçüm.
- **Bileşen** (`viewer::tests::a_section_fills_what_it_cuts_and_waits_while_dragged`): yatay kesitte iki duvarın kapağı
  (sahipleri duvarlar), lejantın son satırı "Kesit yüzü"; sürüklerken kapak yok, bırakınca geri gelir; kesit kalkınca
  kapak ve lejant satırı gider.
- **Gerçek GPU** (`the_3d_scene_on_a_real_gpu` › `a_cut_ends_at_its_samples_and_a_cap_is_not_cut`): 8 m'lik kare
  256 piksel, düzlem 128. sütunun ortasının çeyrek piksel doğusunda: Hızlı'da sütun tam gri, sonraki boş; Dengeli'de
  sütun dörtte üç gri (dört örneğin üçü tutulur; piksel başına kesmede tam gri olurdu), sonraki boş; düzlemin kestiği
  yanda kapak stilli kare çizilir. Gölgelendirici sözleşmesi (`styled_contract`): sürüm 4, View 208 bayt.
- **Masaüstü:** `ifc_viewer_tests`, `scene3d`, `module_tests`, `ribbon_tests`, `history::text`, `catalog`, `settings`
  süzgeçleri yeşil (164 sınama).
- **Resimler:** `.run/gui-evidence/kesit-kapagi-8131/` (`ifc_viewer_tests::cap_screens`): bölüm binasının dosyası
  yüksekliğinin üçte birinde yatay kesik (duvarlar, kolonlar, merdivenin sahanlığı dolu; sahanlıkla duvar arasındaki
  2,5 cm boşluk modelindir), ortasından kuzey–güney kesiti (Kaliteli ve Hızlı), seçilen duvarın çevresinde kutu; açık
  ve koyu. Büyütülerek bakıldı: kesit bantlarında nokta ve dış kenarda saçak yok.

## §6. Kalanlar

- Çok büyük modelde (milyon üçgen düzeyi) kapağı arayüz iş parçacığının dışında hesaplamak ve sürüklerken de göstermek.
- Kapağın bileşen ya da malzeme rengiyle ve taramayla gösterilmesi (ayar); kesit çizgisinin kalınlığı.
- Kesitin tutamakları (düzlemi yüze tıklayarak kurmak ve sahnede sürüklemek; kutunun yüzleri), IFC-12'nin öbür kalanları.
