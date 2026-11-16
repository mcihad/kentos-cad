# ADR 8073: IFC görüntüleyicinin ilk dilimi — çekirdek, bileşen, bağdaştırıcı, kesit ve `cad.ifc.inspect`

- **Durum:** uygulandı (16 Kasım 2026). Plan: [ifc-uyum.md](../../plan/ifc-uyum.md) IFC-12; [bim-plan.md](../../plan/bim-plan.md)
  sırası B12b. İlk kabulün maddeleri §8'de tek tek; gerçek pencerede, işletim sisteminin faresiyle kabul bu Mac'te
  koşturulmadı.
- **Bağlam:** ADR [8061](8061-ifc-viewer-and-reading-engine.md) (gevşek bağ: katmanlar §3, bağlantı yüzü §4, ilk kapsam §5;
  okuma yüzü ve motor denemesi §11), [8060](8060-secondary-os-windows.md) (işletim sisteminin ayrı pencereleri, §10'un
  sözleşmesi), [8067](8067-building-3d-view-and-room-overlaps.md) (binanın 3B görünümü), [0910](0910-preliminary-route-junction-and-3d.md)
  (ortak 3B sahne çekirdeği ve çizicisi).
- **Web:** dokunulmadı (ADR 0228); yeni sözleşme türlerinin TypeScript karşılıkları üretildi, web'in işleyicisi TODOS.md
  §0.2'dedir. **`.kcad`:** değişmedi. **Sözleşme:** yeni ürün komutu `cad.ifc.inspect` 1; yeni kodlar `ifc_unreadable`,
  `ifc_object_not_found`, `ifc_read_findings`. **Arayüz komutu:** `ifc.view`. **Paket:** yeni `kentos-model-view`
  (crates/shared/model-view), `kentos-model-view-ui` (crates/model-view-ui), `kentos-render-scene3d` (crates/render/scene3d,
  taşındı). Yeni dış bağımlılık yok.

## §1. Araştırma

ADR 8061'in araştırma notu ([2026-11-15](../research/2026-11-15-ifc-goruntuleyici.md)) akışı kurmuştu: yapı ve sınıf
ağacı, sahne, özellikler, görünürlük, kesit. Bu dilimde kesitin ve panellerin biçimine bakıldı:

| Kaynak | Ne diyor | KentOS kararı |
|---|---|---|
| BIMcollab Zoom, kesit düzlemi ve kutusu ([yardım](https://helpcenter.bimcollab.com/portal/en/kb/articles/creating-moving-and-selecting-section-planes-or-box), [kutu ve hızlı yaklaşma](https://helpcenter.bimcollab.com/portal/en/kb/articles/quick-zoom-and-section-box)); BIMcollab Model WebViewer ([araç çubukları](https://helpcenter.bimcollab.com/portal/en/kb/articles/toolbars-in-bimcollab-model-webviewer)) | Kesit kutusu seçimin çevresinde altı düzlemdir; düzlemler modelin içinden görünüm verir; web görüntüleyicide en çok altı düzlem ve bir kutu | Kesit bir düzlem ya da seçimin çevresinde altı düzlemli kutu; çizici en çok altı düzlem tutar (`MAX_PLANES`) |
| xeokit SDK, [FaceAlignedSectionPlanesPlugin](https://xeokit.github.io/xeokit-sdk/docs/class/src/plugins/FaceAlignedSectionPlanesPlugin/FaceAlignedSectionPlanesPlugin.js~FaceAlignedSectionPlanesPlugin.html), [kesilen nesneyi taramak](https://xeokit.io/blog/hatching-sliced-objects-using-xeokit-sdk) | Düzlem konumu ve yönüyle kurulur; yüze göre hizalı düzlem sürükleme tutamağıyla taşınır; kesilen yüzün taranması ayrı bir iştir | İlk dilimde düzlem üç eksenden birine diktir, yeri kaydırıcıyla verilir; yüze tıklayıp kurmak ve sürüklemek, kesilen yüzün kapağı sonraki iştir (§9) |
| That Open Engine, [Clipper](https://docs.thatopen.com/api/@thatopen/components/classes/Clipper), [mekân ağacı](https://docs.thatopen.com/Tutorials/UserInterface/OBC/SpatialTree), [IfcPropertiesManager](https://docs.thatopen.com/api/@thatopen/components/classes/IfcPropertiesManager) | Kırpma düzlemleri ayrı bir bileşenin işidir; ağaç modelin mekân ilişkilerinden kurulur; özellik setleri bileşenin okuduğu IFC'den | Kesit çekirdekte saf bir türdür (`Section`), çizici ve seçme aynı düzlemleri kullanır; ağaç okuma yüzünün `parent`/`container` ilişkilerinden, açıklık oyduğu gövdenin altında; özellikler okuma yüzünden |

Değer alınmadı: kutunun payı ve renkler KentOS'un kararıdır (§3).

## §2. Paketler, kurulduğu gibi

ADR 8061 §3.2'nin tablosu uygulandı:

| Paket | İçi |
|---|---|
| `kentos-model-view` | `Input` (nesneler: anahtar, sınıf, etiket, kategori, ağaçtaki üstü, katı; gövdeler f64 köşelerle; gösterilemeyenler nedenleriyle), `View` (gizli, yalnız bırakılan, mahaller, açıklıklar, seçilen, bulgulu; ağacın satırları yapıya ya da sınıfa göre, yalnız açık dallar gezilir; sahnenin kaynak ağları parça renkleriyle; kutu, lejant, sayılar), `OrbitCamera` (yörünge, kaydırma, yaklaşma, sığdırma, dört bakış, kuzey oku, ışık), `Section` (düzlem ya da kutu, düzlemler, ışının korunan aralığı), `Details` (seçilenin ad-değer grupları). IFC adı geçmez; yerel ve wasm32. |
| `kentos-ifc`, `view` özelliği | `read::view::input` (sınıftan kategori, şemanın kalıtımıyla; açıklık oyduğu gövdenin altında; mekân bölgesi saydam hacim), `Model::details` (öznitelikler, tip, kat, malzeme, setler; değer dosyadaki gibi, birim kendisinin ya da projenin), `Model::opening_bodies` (açıklıkların oyulmamış hacimleri), `step::unguid` (GlobalId'den 128 bitlik sayı). |
| `kentos-model-view-ui` | `scene` (karolara bölünmüş sahne, seçme, Iced'in gölgelendirici bileşeni; hareketler ev sahibine `Gesture`), `look` (gök, kuzey oku, lejant, bakışlar menüsü), `viewer` ve görünümü (araç şeridi, ağaç, sahne, özellikler, durum çubuğu; `Host` ev sahibinin verdikleri), tek pencere örneği. Uygulamayı, belgeyi, komutu bilmez. |
| `kentos-render-scene3d` | 3B çizici `kentos-render-wgpu`'dan taşındı: o paket stil motoruyla çizim belgesini getiriyordu (`cargo tree`, ADR 8061 §3.2'nin ölçütü). `kentos_render_wgpu::scene3d` aynı türleri yeniden verir. |
| `kentos_ui::widget::strip` | Araç şeridi masaüstünden kite taşındı; masaüstü yeniden verir. |
| bağdaştırıcı | `apps/desktop/src/shell/ifc_viewer.rs` (§5). |

**3B görünüm de aynı parçalardadır.** Binanın ve yolun 3B görünümü (`scene3d_window.rs`) artık sahneyi, gölgelendirici
bileşeni, kuzey okunu, lejantı, gökü ve yörünge kamerasını bileşenden ve çekirdekten alır; kendinde yalnız penceresi, ne
gösterdiği ve olayları kalır. Görüntüleyici ile 3B görünüm arasında ikinci bir sahne kodu yoktur (ADR 8061 §3).
`scripts/arch/deps.mjs`'e iki grup eklendi: `model-view-ui` (ortak, kit ve 3B çizici; belge, komut, araç oturumu yok) ve
`render3d` (yalnız ortak çekirdekler). Denetim bu iki grup için temiz; bilinen tek sorun önceden var olan
`crates/vendor/block`'tur.

## §3. Kesit

- **Tür:** `Section::Plane { cut, at, flip }` üç yönden birinde: yatay (kot), kuzey–güney doğrultusunda düşey (doğu
  koordinatında), doğu–batı doğrultusunda düşey (kuzey koordinatında); alt yan korunur, `flip` öbürünü korur.
  `Section::Box(kutu)` seçilen nesnenin ve altındakilerin kutusu, köşegeninin yirmide biri kadar payla (seçilenin kendi
  yüzleri kesilmesin; görünüm seçimidir, ölçü değildir).
- **Çizici:** kare düzlemleri gösterilen uzayda alır (düşey abartma varsa `section::shown` çevirir). Her düzlem gözün
  yerine göre yazılır, f64'te: a·(göz + r) + d = a·r + (a·göz + d); sabiti gözün düzleme uzaklığıdır, gölgelendiriciye
  yalnız küçük sayılar gider. Parça gölgelendiricisi türevleri aldıktan sonra kesilen yandaki parçayı atar
  (`discard`). Kesilen yüzün kapağı çizilmez (ADR 8061 §5).
- **Seçme:** ışının düzlemlerin koruduğu parçası `[t0, t1]` bulunur (dışbükey bölge, tek aralık); ışın `t0`'dan başlar,
  `t1`'den sonraki değme kesilmiştir. Kesilip atılan bir nesne tıklanınca seçilmez.
- **Sınama:** GPU sınaması (`the_3d_scene_on_a_real_gpu`) TM koordinatlarında 100 m'lik kareyi düzlemle ve kutuyla keser;
  kalan yer kameranın izdüşürdüğüyle bir piksel içinde. Bileşenin sınaması tıklamanın kesilen tarafı seçmediğini tutar.

## §4. Sürekli iki sınama

- **wasm32:** `cargo build -p kentos-model-view -p kentos-ifc --features kentos-ifc/view --target wasm32-unknown-unknown`
  uyarısız derleniyor (16 Kasım 2026).
- **Tek pencere örneği:** `cargo run -p kentos-model-view-ui --example viewer -- <dosya.ifc>` bileşeni uygulama olmadan açar;
  resim sınaması `KENTOS_SNAPSHOT_BACKEND=wgpu cargo test -p kentos-model-view-ui --test screens -- --ignored`
  (kesit binası tıklanan duvarıyla açık ve koyu, yatay kesit, katın kutusu, gerçek bir IFC 4.3 dosyası, okunuyor,
  reddedildi; `.run/gui-evidence/model-view-ui/`).
- **Vitrin:** bileşenler vitrininde (`apps/ui-showcase`) Galeri › CBS ve CAD › “Model görüntüleyici” sayfası aynı
  bileşeni kesit binasının dosyasıyla gösterir (sahibin 16 Kasım isteği); dosya sayfa ilk açılınca okunur, vitrinin
  açılışı onu beklemez. İkinci girdi kuzey okunu dört bakışta ve lejantı gösterir. Resmi:
  `kentos-ui-showcase snapshot … --senaryo galeri --sayfa model` (gerçek tıklamayla `--tikla x,y`).

## §5. Uygulamadaki bağdaştırıcı

- **Pencere:** `Surface::IfcViewer`, ADR 8060 §10'un sözleşmesiyle (`IfcView: OwnWindow`): kendiliğinden ayrı pencerede
  açılır (1400 × 860, en küçüğü 900 × 560), “Ana pencereye al” ile çizimin üstüne gelir; seçim `ayri-pencereler.json`'da
  `ifc-modeli` adıyla saklanır. Esc kapatır.
- **Okuma:** dosya kendi iş parçacığında okunur (`iced_runtime::task::blocking`): okuma yüzü, gövdeler, açıklıkların
  hacimleri, girdi. Her okumanın bir numarası ve durdurma bayrağı vardır; yeni bir dosya öncekini durdurur, İptal ve
  pencerenin kapanması durdurur; durdurulan ya da numarası eskimiş okumanın sonucu pencereye yazılmaz (kılavuz §20.2).
  Okunurken evresi yazar (“Nesneler ve ilişkiler okunuyor”, “Gövdeler üçgenleniyor”); bozuk dosya okuma yüzünün
  iletisiyle reddedilir (nedeni ve çözümü).
- **Planla birlikte seçim:** KentOS bir nesnenin GlobalId'sini kalıcı kimliğinden yazar (`step::guid`); görüntüleyicide
  seçilen nesnenin GlobalId'si `step::unguid` ile kimliğe döner ve açık çizimde o nesne varsa planda seçilir. Plandaki
  seçim de her iletiden sonra görüntüleyicide seçilir (`follow_ifc_viewer`). Başka bir programın dosyası planda bir şey
  seçmez. Görüntüleyici çizimi değiştirmez.
- **Yeri:** Mimari › Proje › “IFC modeli” (3B görünümün altında) ve Dosya › Alışveriş › İçe aktar › “Bina modeli” bloğu;
  `ifc.view` dosyayı sorar. IFC dışa aktarımı bitince sağ alttaki bildirim “Görüntüle” düğmesiyle dosyayı açar.
- **Kaynaklar:** pencere kapanınca okuma durur, model ve sahne bırakılır. Kendi penceresi çiziciyle birlikte gider;
  çizimin üstündeyken gölgelendiricinin durumu düşerken görünümün numarasını bırakır, sonraki karede o görünümün çizicisi
  ve GPU tamponları silinir (`RETIRED`).

## §6. `cad.ifc.inspect`

Salt okunur ürün komutu (ADR 8061 §5 “Komutlar”; adı kataloğun `cad.<alan>.<eylem>` düzeninde kesinleşti): dosyanın
şeması, kayıt ve nesne sayıları, sınıflara göre nesneler, projenin birimleri (önek ve ad yazıldığı gibi), katlar dosyadaki
sırasıyla ve doğrudan içerdikleri, okumayı durdurmayan bozukluklar (satır sırasıyla); `object` ile bir nesnenin
öznitelikleri, tipinin ve katının adı ve setleri (kendi setleri ilişkilerinin kayıt sırasıyla, sonra tipininkiler), her
değer türüyle (`text`, `number` ve ölçü türü, `integer`, `logical` (bilinmiyorsa `null`), `enumeration`, `list`,
`bounded`, `reference`, `empty`). Retler `invalid_file_path`, `ifc_unreadable`, `ifc_object_not_found`; uyarı
`ifc_read_findings`. Çizimi okumaz, yazmaz (`effect: query`); araç sunucusunda salt okunur ipucuyla gelir.

**Ortak durumlar:** `fixtures/commands/v1/cad.ifc.inspect.json`, 37 durum, `scripts/fixtures/ifc_inspect_cases.py`
IfcOpenShell 0.9.0.post1'in okuyuşundan yazar (KentOS kodu ve KentOS'un okuma motoru olmadan): KentOS'un 11, başka bir
programın 2, gerçek dünyanın 7 dosyasının özeti; duvar, mahal, kat, başka programın ve gerçek örneğin duvarı, tipinden
set alan nesne; plan ve denetle; dört yol hatası; olmayan nesne; STEP olmayan ve şeması bilinmeyen dosya; olmayan kayda
başvuru ve iki nesnede aynı GlobalId (bu iki dosya betikte sağlam bir örnekten yapılır; bulgunun kaydı ve satırı
yapılışından bilinir). Masaüstünün işleyicisi ve Python'un tipli katmanı 37'sini de geçer; değiştirilen bir beklenen
değeri ikisi de yakalar (denendi). Python SDK yeniden üretildi (`kentos.cad.ifc`).

## §7. Ölçüm

Büyük model (`scripts/fixtures/ifc_large_case.py 700`: 63 700 nesne, 49 000 gövde, açıklıklarla 2 150 400 üçgen), Apple
M5, release, `crates/model-view-ui/tests/timing.rs`:

| Adım | Süre |
|---|---|
| Okuma yüzü | 0,74 s |
| Gövdeler ve açıklık hacimleri | 8,59 s |
| Görüntüleyicinin girdisi | 0,13 s |
| İlk sahne (karolara bölme) | 0,12 s |
| Seçimle sahnenin yeniden kurulması | 0,12 s |

Seçim ve gizleme sahneyi yeniden kurar: parça renkleri karoların içindedir. 2 milyon üçgende bu release'te 0,12 s'dir;
nesne başına renk tamponu sonraki iştir (§9). Hedef ADR 8061 §7'deki gibi bu ölçümden sonra konur.

## §8. İlk kabul (plan IFC-12)

| Ölçüt | Durum |
|---|---|
| Bölüm binasının dosyası açılır; gövde sayısı dosyadakiyle eşit | ✓ `ifc_viewer_tests`: dosyanın metninden sayılan şekilli ürünler (yapı elemanları, mahaller, mekân bölgeleri) = görüntüleyicinin gövdeleri (70) |
| Ağaçta katlar | ✓ iki kat, açık |
| Tıklanan duvarın özellikleri dosyadakiyle aynı | ✓ ayrıntıların genişliği ve boyu çizimdeki duvarın; setler okuma yüzüyle aynı (okuma yüzü B12'de IfcOpenShell'le set set aynı) |
| Dışa aktarımdan tek tıkla açılır, seçilen duvar planda seçilir | ✓ bildirimin “Görüntüle”si; tıklanan duvar planda, plandaki duvar görüntüleyicide |
| Açık bir IFC 4.3 örneği gösterilemeyenlerin listesiyle açılır | ✓ gerçek IFC 4.3 binası; “Gösterilemeyen” sekmesi (bu örnekte boş; kesit binasında aks) |
| Bozuk dosya açık iletiyle reddedilir | ✓ |
| İptal edilen okuma pencereye yazmaz | ✓ İptal ve yeni dosya |
| Pencere kapanınca bellek ve GPU kaynağı bırakılır | ✓ model ve okuma; GPU: kendi penceresiyle, çizimin üstündeyken sonraki karede |
| wasm32 derlemesi ve tek pencere örneği yeşil | ✓ |
| Gerçek pencerede, gerçek fare ve klavyeyle kabul; resimler | Resimler ✓ (`.run/gui-evidence/ifc-modeli-8073/`); gerçek pencerede kabul koşturulmadı |
| `ifc.inspect`: katalog, ≥ 20 bağımsız durum, Python, MCP | ✓ `cad.ifc.inspect`, 37 durum |

## §9. Açık kalanlar

- Kesit düzlemini yüze tıklayarak kurmak ve ekseni boyunca sahnede sürüklemek; kutunun tutamakları; kesilen yüzün kapağı.
- Gösterilemeyen nesnenin sınır kutusu ve işareti: motor gövdesini çıkaramadığı nesneye kutu da vermiyor; liste var, kutu
  yok.
- İlk gövdeler gelince sahnenin çizilmesi (okuma parça parça): bugün sahne bütün gövdeler bitince kurulur, evre yazar.
- Seçimde sahneyi yeniden kurmadan boyamak (nesne başına renk).
- Mimari denetimin bulgusundan “Modelde göster” (ADR 8061 §5): bulguları dosyadan doğrulama B13'tür (IFC-08).
- Eski sürümlerin (IFC4, IFC2x3) kayıp listesi IFC-11'dedir; şema adı durum çubuğunda yazar.

## §10. Sonuçlar

- Görüntüleyici ve 3B görünüm tek sahne, tek kamera, tek görünüş dilindedir; birinde düzelen öbüründe de düzelir.
- Ayrı program ya da web gerektiğinde bileşen ve çekirdek hazırdır; iki sınama (wasm32, tek pencere) bu yolları açık
  tutar. Okuma ayrı süreçte koşturulacaksa iletiler aynıdır (ADR 8061 §4).
- Ölçü, alan, hacim üçgen ağdan gösterilmez; miktarlar dosyanın setlerinden okunur (kılavuz §23.3).
