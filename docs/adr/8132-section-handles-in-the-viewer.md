# ADR 8132: Görüntüleyicide kesitin tutamakları ve yüzden kesit

- **Durum:** uygulandı (18 Kasım 2026). Plan: [ifc-uyum.md](../../plan/ifc-uyum.md) IFC-12 (Kalanlar: "kesit düzlemini
  yüze tıklayarak kurmak ve sürüklemek, kutunun tutamakları"), [bim-plan.md](../../plan/bim-plan.md) B12b.
- **Bağlam:** ADR [8073](8073-ifc-viewer-first-slice.md) §3 (üç düzlem ve seçimin çevresinde kutu, kartın kaydırıcısı),
  [8121](8121-viewer-measuring-and-folding-panes.md) (sahnenin işaret katmanı, ölçerken tıklama),
  [8131](8131-section-caps-in-the-viewer.md) (kesit yüzü; sürüklerken beklemesi).
- **Web, `.kcad`, komut:** değişmedi. Kesit bir görünümdür; çizimi değiştirmez, ürün komutu değildir (ADR 8121 gibi).

## §1. Sorun

Kesit yalnız üç eksen düzlemiyle kurulabiliyor, düzlem kartın kaydırıcısıyla kaydırılıyordu: eksene göre dönük bir
duvara ya da eğik bir çatıya paralel kesit alınamıyordu; sahnedeki düzlem tutulup çekilemiyordu; seçimin çevresindeki
kutunun boyu değiştirilemiyordu (kutu seçimin sınırına göre bir kez kuruluyordu).

## §2. Araştırma (18 Kasım 2026)

| Kaynak | Ne diyor | KentOS kararı |
|---|---|---|
| BIMcollab Zoom, [Creating, moving and selecting section planes or box](https://helpcenter.bimcollab.com/portal/en/kb/articles/creating-moving-and-selecting-section-planes-or-box) | "Add section plane" ile düzlem tıklanan yüzün üstüne kurulur; seçilip ekseni boyunca sürüklenir (Ctrl/Cmd ile ince); kutu seçimin çevresinde (Ctrl/Cmd+B), Ctrl/Cmd+tekerlekle büyür, küçülür; kenarına yakın Ctrl/Cmd+sürükleme döndürür | Yüzden kesit: tıklanan yüze paralel düzlem (K311); düzlem okundan ekseni boyunca sürüklenir (K313). İnce adım ve döndürme sonraya (§6) |
| Solibri, [The Sectioning Tool](https://help.solibri.com/hc/en-us/articles/1500003939981-The-Sectioning-Tool) ve [Central Innovation ipucu](https://centralinnovation.com/technical-resources/tips-and-tricks/solibri-section-tool/) | Kesit aracı açıkken bir yüze tıklanınca düzlem orada belirir | Aynı akış: menüden Yüzden kesit, sonra yüze tek tıklama |
| That Open Engine, [Clipper](https://docs.thatopen.com/Tutorials/Components/Core/Clipper) | Tıklamanın altında bir yüz varsa düzlem ışının yüzü kestiği yere konur | Düzlem tıklanan noktadan geçer (K311) |
| Autodesk Navisworks, [Move and Rotate Section Planes](https://help.autodesk.com/cloudhelp/2025/ENU/Navisworks/files/GUID-952107F0-E65A-4CDC-A928-6EF869BD677C.htm); Autodesk Viewer'ın kesit aracı ([VueOps rehberi](https://info.vueops.com/knowledge/section-a-model)) | Düzleme dik ok düzlemi taşır; beyaz ok sürüklenince düzlem kayar | Düzlemin ortasında, kesilen yana bakan ok (K312) |
| PlanRadar, [Section planes and section boxes](https://helpcenter-prd.planradar.com/en/work-with-section-planes-section-boxes/); Blender [Section Toolbox](https://extensions.blender.org/add-ons/stb-section-toolbox/) | Kutunun bir yüzü tutulup sürüklenince kutu büyür ya da küçülür; her yüzün ortasında tutamak | Kutunun altı yüzünün ortasında dışarı bakan ok (K312, K313) |
| xeokit SDK, [SectionPlanesPlugin](https://xeokit.github.io/xeokit-sdk/docs/class/src/plugins/SectionPlanesPlugin/SectionPlanesPlugin.js~SectionPlanesPlugin.html) | Düzlem bir nokta ve yönle kurulur (`pos`, `dir`); düzenleme aracı taşır ve döndürür | Düzlemin verisi tıklanan nokta ve yüzün normali (K311) |

## §3. Karar

- **K311 — Yüzden kesit:** Kesit ▾ › Yüzden kesit; sonraki tıklamanın vurduğu yüze paralel düzlem, tıklanan noktadan
  1 mm önde (görüldüğü yana doğru), arkası tutulur: yüzün kendisi kesilmez, önündekiler kalkar. Kesitin kaldırdığı
  yan tıklamada vurulmaz (ADR 8073 §3). Eksene göre yatışı, modelin köşegeni boyunca o 1 mm'nin yarısını aşmayan yüz o
  eksenin düzlemini verir (Yatay kesit, Kuzey–güney, Doğu–batı; kartta Kot, Y, X); öbürü "Yüze paralel kesit"tir, kartta
  "Derinlik" (yüzden modelin içine doğru metre). Boşluğa tıklama yüz bekler. Ölç ile Yüzden kesit sırayla: biri açılınca
  öbürü kapanır. Kartta "Düzlemin paralel olacağı yüze tıklayın.", durum çubuğunda "Bir yüze tıklayın: kesit o yüze
  paralel kurulur."
- **K312 — Tutamaklar:** düzlemin tutamağı modelin ortasının düzlemdeki yerinde, kesilen yana bakan ok; kutunun altı
  tutamağı yüzlerinin ortasında, dışarı bakan oklar. Sahnenin işaret katmanında (gövdelerin arkasında kalmaz), vurgu
  renginde beyaz kenarlı; işaretçinin altındaki ya da sürüklenen dolu ve büyük. İşaretçi üstündeyken el imleci. Ölçerken
  ve yüz beklerken tutamak yok (tıklama ölçer ya da yüz seçer).
- **K313 — Sürükleme:** tutamağa basılınca sürükleme yörünge değil düzlemdir. İşaretçinin hareketinin okun ekrandaki
  doğrultusundaki payı, ok boyunca bir metrenin tutamağın yerinde kapladığı piksele bölünür (perspektifte o uzaklıkta):
  düzlem işaretçiyi izler; okun ekrana dik durduğu (göze baktığı) tutamak kıpırdamaz. Düzlem modelin kutusunda kalır;
  kutunun yüzü kutuyu 1 cm'den inceltmez, dışarı serbest. Kartın kaydırıcısı birlikte yürür; kesit yüzü bırakılınca gelir
  (ADR 8131 K310).
- **K314 — Yazılar:** Kesit ▾'nin ipucu "…; kesilen gövdeler koyu dolgulu. Düzlem bir yüze paralel de kurulur;
  sahnedeki okundan sürüklenir." (ADR 8131'de unutulan "Kesilen yüzler kapatılmaz" cümlesinin yerine); Fare yardımına
  "Kesitin okunu sürükleyin: düzlemi ya da kutunun yüzünü kaydırır."
- **K315 — Kapsam:** kesit görünümdür: komut, Python, MCP ve kısayol yok (ADR 8121'in ölçmesi gibi). Yeni renk ve yazı
  yok: vurgu, alan ve beyaz (ADR 8121 K265). Döndürme, ince adım ve çok düzlem sonraya (§6).

## §4. Uygulama

- Çekirdek `crates/shared/model-view/src/section.rs`: `Section::Face { origin, normal, depth, flip }`,
  `Section::from_face`, `place`/`set_place`, `range`, `handles`, `moved`; `Handle`; `normal_of`; `FACE_GAP` (1 mm),
  `THINNEST_BOX` (1 cm).
- Bileşen `crates/model-view-ui/src/handles.rs` (yeni: ekrandaki yer, `at` ile vuruş, `along` ile metre, çizim);
  `scene.rs` (`Gesture::Handle`, `Gesture::HandleDone`, `SceneView.handles`/`active`, basışta tutamak, imleç);
  `viewer.rs` (`Message::FromFace`, `facing`, `gripped`, `face_click`, `range`, `handles`); `viewer_view.rs` (kart,
  menü, durum çubuğu, yardım).
- Uygulama `apps/desktop/src/shell/scene3d_view.rs`: yolun sahnesinde tutamak yok; tutamak hareketleri binanın
  görüntüleyicisine gider.

## §5. Sınamalar

- **Çekirdek** (`section::tests`): 30 m'lik modelde döşemenin üstüne yukarıdan tıklama 1 mm üstte yatay kesit (alt
  tutulur); duvarın batı yüzüne batıdan tıklama 1 mm batıda kuzey–güney kesiti (doğu tutulur); 45° dönük yüz ona paralel,
  1 mm önde, yüz ve arkası tutulur; eksenden milyonda bir radyan yatan yüz eksen düzlemi, binde bir yatan değil; alansız
  üçgen düzlem vermez. Yüze paralel düzlem ±5√2 aralığında kayar, tutamağı görülen yana bakar, 2 m geri çekilince 2 m
  derinleşir, uzak köşede durur; ters çevrilince öbür yan tutulur, ok tersine döner. Yatay kesitin tutamağı modelin
  ortasında, yukarı bakar, modelde kalır; kutunun altı tutamağı yüzlerin ortasında, doğu yüzü 2 m dışarı, batı yüzü
  doğunun 1 cm berisinde durur.
- **Bileşen** (`handles::tests`): yukarıdan 10 piksel/metre görünüşte yatay kesitin tutamağı göze bakar (ok yok, hareket
  0); kuzey–güney kesitininki doğuya 34 piksellik ok; okun üstüne basış onu alır, yanına değil; 25 piksel sağa 2,5 m,
  25 piksel yukarı 0. (`viewer::tests::a_click_on_a_face_makes_a_plane_and_its_handles_drag_the_section`): Yüzden
  kesit ile küpün üstüne tıklama hiçbir şey seçmez, 1 mm üstte yatay kesit, tutamağı modelin ortasında; yarım metre aşağı
  sürükleyince iki küp kesilir, kesit yüzü bırakılınca gelir; kuzey yüzüne tıklama 1 mm kuzeyde doğu–batı kesiti (güney
  tutulur); Ölç ve Yüzden kesit sırayla; kutunun altı tutamağı, doğu oku 1 m dışarı.
- **Masaüstü:** `ifc_viewer_tests`, `scene3d`, `module_tests`, `ribbon_tests`, `history::text`, `catalog`, `settings`
  süzgeçleri yeşil (164); çekirdeğin wasm32 derlemesi yeşil.
- **Resimler:** `.run/gui-evidence/kesit-tutamaklari-8132/` (`ifc_viewer_tests::handle_screens`): bölüm binasında yatay
  kesitin oku sürüklenirken (dolu, kesit yüzü bekliyor) ve bırakılınca; seçilen duvarın kutusu altı okuyla, doğu yüzü
  1,5 m dışarı; Yüzden kesit'in bekleyen kartı; eğri binanın yay duvarına paralel kesit, 2,5 m içeri; açık ve koyu. Oklar
  büyütülerek bakıldı.

## §6. Kalanlar

- Düzlemi ve kutuyu döndürmek; ince adım (bir tuşla yavaş sürükleme) ve klavyeyle kaydırma.
- Aynı anda birden çok düzlem; kesitin görünüm olarak saklanması (BCF bakış noktasına bağlanabilir).
- Gerçek pencerede, gerçek fare ve klavyeyle kabul (IFC-12'nin ortak kalanı).
