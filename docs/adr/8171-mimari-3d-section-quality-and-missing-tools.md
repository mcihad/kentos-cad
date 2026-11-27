# ADR 8171: Mimari'nin 3B görünümü, kesiti ve eksik araçları

- **Durum:** kabul edildi (2026-11-27). Plan: [plan/mimari-bim.md](../../plan/mimari-bim.md). Araştırma:
  [docs/research/2026-11-27-mimari-3b-kesit-ve-araclar.md](../research/2026-11-27-mimari-3b-kesit-ve-araclar.md).
- **İstek** (sahip, 27 Kasım 2026):
  - "3d bina çizimlerde ciddi problemler var … kolon kirişler düzgün render edilmiyorlar. Kesit esnasında ciddi
    bozulmalar oluyor";
  - "Kesit değiştirirken ve döndürürken üçgenlerde ve mesh yapılarda titremeler ve kayıplar oluyor";
  - mimari projede eksik araçlar; piyasadaki mimari çizim ve BIM araçlarının incelenmesi.
- **Değiştirdikleri:** ADR 8131 K310 (kesit sürüklenirken kapak yok), ADR 8041 §1'in kirişinin ve kolonunun 3B ile IFC
  gövdesi. Kirişin ve kolonun ölçüleri ve kotları değişmez.

## §1 Bulgu

Araştırma raporunun §3'ü. Özetle:

- **B1:** Kolon, kiriş ve döşeme aynı betonu tutuyor. Aynı kottaki yüzler derinlikte yarışıyor (titreme). IFC hacimleri
  birleşimleri iki ya da üç kez sayıyor.
- **B2:** Kesit sürüklenirken kapak bilerek kaldırılıyor; gövdeler içi boş görünüyor.
- **B3:** Kapak halkası 1e-7 m ile zincirleniyor; float32 gövdede kopup kapak hiç çizilmeyebiliyor.
- **B4:** Kenar çizgisi köşeleri bitleri birebir aynıysa birleştiriyor; ayrık köşeli yüzün köşegeni kenar sayılabilir.
  (Gerçek pencerede çatıda görülen noktalı köşegen bu değil: Xvfb'nin yazılımsal Vulkan'ının izi, MB-03.)
- **B5:** Her değişiklikte bütün sahne yeni kimliklerle yeniden yükleniyor; eski parçalar ekran kartında kalıyor.
- **B6:** Keskin dönen kirişin gönyesi bandı kendini kestiriyor; 3B'de uç kayboluyor.

## §2 Karar

### §2.1 Birleşim kuralı (MB-01)

Kesit modeli gövdeleri **birleşim önceliğiyle** ayırır. Hiçbir iki gövde aynı hacmi tutmaz:

1. **Döşemenin taşıyıcısı içindekini tutar.** Kolonun ve kirişin tepesi döşemenin betonarme üstüdür (ADR 8041 §1;
   kiriş yüksekliği döşemeyi içine alır, TBDY 2018 7.4.1.1). Gövdeleri, döşemenin örttüğü yerde onun betonarmesinin
   altında biter. Bu, KentOS'un kendi metraj kuralıdır (ADR 8041 §3: "Kirişin hacmi döşemenin altında kalan
   kısmıdır … Döşemenin içindeki kısmı döşemenin hacmindedir, iki kez sayılmaz").
   - Döşemenin kenarından taşan parça (2,5 cm dışarıda kalan kolon yüzü, döşemesiz yere uzanan kiriş) tam yüksekliğini
     korur.
   - Döşemenin içinden geçen kolon (iki kata yayılan, ADR 8168) döşemenin üstünde sürer.
   - Kirişin ve kolonun ölçüsü, kotu ve "Üst kot"u değişmez; değişen yalnız gövdedir.
2. **Kolon kirişi keser.** Kirişin bandı, düşeyde çakışan kolonların ayak izinde kesilir. Revit'in betonarme birleşim
   tablosu da kolonu baskın sayar.
3. **Kiriş–kiriş:** önce çizilen baskındır (Revit'in kuralı). Sonra çizilenin bandı öbürünün bandında kesilir.

**Kapsam:** Kesit modelini kullanan her şey aynı gövdeyi kullanır: 3B görünüm, IFC'nin iki profili, IFC'nin miktar
setleri ve 2B kesit. Döşemenin gövdesi değişmez. Eğimli kiriş ve döşeme ile bir nesnenin parçası (baca, giydirme
cephenin çerçevesi) birleştirilmez.

**2B kesit:** Kesilen parçalar malzeme malzeme birleştirilip tek dolgu çizilir (`union_areas`, ADR 8041 §2,
TS EN ISO 128-3). Döşeme ile altında biten kiriş yine tek betonarme kesittir; yeni çizgi doğmaz.

**IFC miktarları:** Kolonun ve kirişin `GrossVolume`'u kendi gövdesidir; birleşimdeki beton döşemenindir.
Döşemenin altında kalan ve kenarından taşan parçasıyla iki yükseklikli kolonun `Qto_ColumnBaseQuantities`'i:
- `Length` en alt ayağından en üst tepesine;
- `CrossSectionArea` ayağında duran parçaların toplamı, yani kolonun bütün kesiti.

### §2.2 Kesit yüzü (MB-02)

- Kapak sürüklenirken de çizilir. K310'un "akıcılık için kapaksız" kararı kaldırıldı: sahibin gördüğü kayıp buydu.
- Kapak, modelin parçalarından ayrı küçük bir katmandır. Kaydırıcı her adımda yalnız onu yeniden kurar; model
  parçaları yeniden yüklenmez.
- Halka zinciri ve komşu kapakların köşe alması gövdenin float32 aralığında birleşir. Bu aralık bugünkü
  `MEET` (0,1 mm) mertebesindedir; çizimin hassasiyetinin çok altındadır.

### §2.3 Kenar çizgisi (MB-03)

Köşeler, parçanın kendi kökenine göre 0,05 mm'lik ızgaraya yuvarlanarak birleştirilir. İki gövde yine hiç
birleştirilmez (ADR 8120).

### §2.4 Kararlı parçalar (MB-04)

- Parçanın kimliği içeriğinden gelir. Aynı içerik aynı kimliği alır ve yeniden yüklenmez; kenarları yeniden
  hesaplanmaz.
- Önceki sahnede olup yenisinde olmayan parça hemen ekran kartından bırakılır.

### §2.5 Keskin köşe (MB-05)

Kiriş bandının gönyesi sınırlanır. Sınırı aşan köşe pahlanır; bant kendini kesmez. Plan, kesit ve IFC aynı bandı
kullanır.

### §2.6 Eksik araçlar (Faz B)

Sıra plandadır: temel, duvar profili, arazi ve kazı-dolgu, 3B görünüm paftada, mahal renk planı, güneş ve gölge,
serbest biçimli eleman, yürüyen merdiven, kazık. Her biri kendi ADR'siyle, kendi araştırmasıyla gelir.

### §2.7 Modüller ve kanallar

| | Var mı | Gerekçe |
|---|---|---|
| Mimari | var | 3B görünüm, IFC ve kesit Mimari'nin. |
| Harita, CBS | yok | Bina modeli yok; IFC görüntüleyicisi (`ifc.view`) aynı görüntüleyiciyi kullandığından MB-02…MB-04 onda da geçerli. |
| Yol | kısmen | Yolun 3B görünümü aynı çiziciyi kullanır: MB-03 ve MB-04 orada da geçerli. Birleşim kuralı yolu ilgilendirmez. |
| MCP, Python, eklenti, yapay zekâ ajanı | var | `cad.arch.ifc` aynı kesit modelini yazar; birleşim kuralı onlarda da geçerli. Görüntüleyici bir görünümdür; kanalların ayrı yolu yok. |

## §3 Kanıt

Her madde kendi bölümünü ekler: önce ve sonra resimleri, sınamalar, ölçüler.

### MB-01 (27 Kasım 2026)

- **Bekçi sınamaları** (`crates/native/application/tests/all/frame_joins.rs`):
  - Birleşen her iki gövde (kolon, kiriş, döşemenin taşıyıcısı) düşeyde çakışıyorsa planda ortak alanları < 1e-9 m².
    Elle kurulan çerçevede ve bölüm binasında denetlenir.
  - Hacimler elle hesaplananla aynı (1e-9 m³):
    - kolon 0,09 × 2,85;
    - kolonlar arasındaki kiriş 4,70 × 0,25 × 0,35;
    - döşeme kenarından taşan kiriş 2,85 × 0,25 × 0,50;
    - sonra çizilen ve kesişen kiriş 2 × 0,875 × 0,25 × 0,35;
    - kenarda yarısı döşeme altında kalan kiriş 5 × 0,125 × (0,35 + 0,50);
    - döşemesiz katta yuvarlak kolona dayanan kiriş, çemberin iki noktası arasında.
  - İlk koşuda bir açık yakalandı: kenardaki kolonun içine düşen kiriş parçası kesilmiyordu ("hiçbir şey kalmazsa
    gövdeyi koru" parça başına uygulanıyordu). Kural eleman başına alındı.
- **Bölüm binasının IFC'si** (`fixtures/arch/v1/ifc/kesit-binasi-tipli.ifc`, `KENTOS_WRITE_IFC_TYPES=1` ile yeniden
  yazıldı):

  | Sınıf | Önce | Sonra |
  |---|---|---|
  | Kiriş | 10,50 m³ | 6,93 m³ |
  | Kolon | 4,29 m³ | 4,10 m³ |
  | Döşeme | 97,45 m³ | 97,45 m³ |
  | Toplam | 112,24 m³ | 108,48 m³ |

  İki kez sayılan 3,76 m³ gitti. Elle sağlama: 84 m kiriş × 0,25 × 0,35 = 7,35 m³, eksi 16 kolon kesimi × 0,30 m ×
  0,25 × 0,35 = 0,42 m³ → 6,93.
- **IfcOpenShell 0.9.0** yeni dosyayı açtı: türler, katman takımları ve kat yerleşimi "uyuyor"
  (`ifc_types_check.py`, `slab_layers_check.py`, `storey_spans_check.py`).
- **Sınamalar:**
  - `kentos-native-application`: kesit, IFC, metraj, kolon ve mahal sınamaları ile `cad.arch.ifc`, `cad.sections`,
    `cad.elevations`, `cad.spaces.quantities` ortak durumları geçti; 2B kesit sınamalarında değişen yok.
  - `kentos-ifc`'in 77 sınaması ve Mimari masaüstünün 3B ile IFC görüntüleyicisi sınamaları (21) geçti.
- **Resim:** `.run/gui-evidence/mimari-bim/sonra/mb01-birlesim.png`. Gerçek pencerede döndürürken birleşimlerdeki
  titreyen kareler yok, yuvarlak kolonun tepesi görünüyor, döşeme kenarındaki çift çizgiler gitti.
- **Uygulama:**
  - `crates/native/application/src/arch/sections/frame.rs`: kolon ve kiriş gövdeleri `sections.rs`'ten taşındı
    (dosya sınırının üstündeydi, 3334 → 3240 satır), `joins`.
  - `sections.rs`: `Model::structure` (döşemelerin taşıyıcı plakaları), `joins`'in çağrısı.
  - `crates/shared/ifc/src/quantities.rs`: kolonun iki yükseklikli gövdesinin miktarları.

### MB-02 (27 Kasım 2026)

- **Kesit yüzü kendi katmanında:** görüntüleyicinin `caps` sahnesi (`crates/model-view-ui/src/viewer.rs` `recap`).
  - Kesit kurulunca, kaydırıcının her adımında, tutamaç sürüklenirken, Öbür yanı'nda ve kesit kalkınca yalnız bu katman
    yeniden kurulur. Model sahnesi kesitle hiç yeniden kurulmaz; önceden kesitin başında ve bırakılışında bütün model
    yeniden yükleniyordu.
  - Çizici iki sahneyi birlikte seçer ve çizer (`crates/model-view-ui/src/scene.rs` `Frame3d::prepare`).
  - Görünümün önceki sahnesinin parçaları ekran kartından hemen bırakılır (`Pipeline::drawn`); sürüklerken her adımın
    eski kapakları birikmez.
- **Halka zinciri toleranslı** (`crates/shared/model-view/src/caps.rs`): segment uçları 0,05 mm içinde aynı düğümdür.
  Düğüm, komşu hücreler aranarak bulunur; ızgara çizgisi iki ucun arasına düşse de birleşir. Önceden 1e-7 m'lik
  yuvarlama yapılıyordu.
- **Sınamalar:**
  - `caps::tests::a_body_whose_corners_lie_a_hair_apart_is_capped_whole`: yüzlerinin köşeleri 0,02 mm'ye dek kaymış
    blok tam kapaklanır (8 m²).
  - `viewer::tests::a_section_fills_what_it_cuts_also_while_dragged_and_leaves_the_scene_as_it_is`: sürüklerken her
    adımda iki kapak, sahne aynı nesne (`Arc::ptr_eq`), sahnede kapak yok.
  - Tutamaç sınaması sürüklerken iki kapak bekler.
  - `kentos-model-view` 24, `kentos-model-view-ui` 8, Mimari masaüstünün 3B ve IFC görüntüleyici sınamaları (16) ve
    yolun 3B görünümü (3) geçti.
- **Resim:** `.run/gui-evidence/mimari-bim/sonra/mb02-kesit-surukleme.png`. Gerçek pencere, yatay kesit sürüklenirken
  aynı anlar: önce 2,96 m'de döşemeler içi boş ve iç üçgenlerin X çizgileri görünüyor; sonra kesit yüzü dolu.
- **Bedel:** kapak her adımda hesaplanır. ADR 8131 K310'un ölçüsü (release, 9 600 gövde): yatay kesit 19,8 ms. Bina
  ölçeğinde gövde sayısı yüzlerle olduğundan adım başına birkaç milisaniye. Çok büyük modelde sürükleme bu kadar
  yavaşlar; kalite için kabul edildi ([kalite hızdan önce](9222-quality-is-not-traded-for-speed.md)).

### MB-03 (27 Kasım 2026)

- **Kenar kaynağı** (`crates/render/scene3d/src/edges.rs`, `WELD` 0,05 mm): bir gövdenin köşeleri bu uzaklıkta birdir.
  Komşu hücreler aranır, ızgara çizgisi iki köşenin arasına düşse de birleşir. İki gövde yine hiç birleşmez.
- **Sınama:** `a_face_whose_triangles_part_by_a_hair_draws_no_diagonal`: her yüzünün iki üçgeni kendi köşeli ve
  0,02 mm'ye dek ayrık kutu yine 12 kenar. Önceki dört kenar sınaması da aynen geçti.
- **Tanının düzeltilmesi:** gerçek pencerede düz çatıdaki noktalı köşegen kaynaktan sonra da duruyordu. Üç kalitede
  aynı açıdan çekildi; Hızlı'da (kenar yok, ortam gölgesi yok, tek örnek) da var. Ekran kartıyla çekilen ekran dışı
  resimde yok (`once/01-kuzeydogu.png`). Gerçek pencere sınaması Xvfb'de lavapipe ile çiziyor; çizgi onun izi.
  Model tarafında da bakıldı: çatının üst yüzü tek gövdede iki üçgen. O kotta yüzü olan öbür gövdeler yalnız emsal
  zonları; onlar öntanımlı olarak gizli.
- **Kalan:** gerçek ekran kartında gerçek pencere bu makinede çekilemiyor (Xvfb'de DRI3 yok). Sahip kendi ekranında
  köşegen görürse ayrıca bakılır.

### MB-04 (27 Kasım 2026)

- **İçerikten kimlik** (`crates/model-view-ui/src/scene.rs`, `Scene::after`).
  - Her döşeme parçasının ekran kartına giden verisinin (köken, konumlar, parçalar, normaller, indeksler, aralıklar)
    özeti çıkarılır.
  - Yeni sahnede önceki sahnedekiyle bayt bayt aynı olan parça onun kimliğini alır. Çizici onu yüklü bulur, yeniden
    yüklemez, kenarını yeniden hesaplamaz.
  - Parça kimlikle bulunur (`index`). Görüntüleyici modeli ve kesit yüzünü önceki sahnesinden sonra kurar.
- **Bırakma:** çizici, görünümün sahnesi değişince yalnız artık hiçbir sahnede olmayan kimlikleri ekran kartından
  bırakır (`Pipeline::drawn`).
- **Ölçüm** (`tests/timing.rs`, `KENTOS_IFC_LARGE=.run/ifc-buyuk/kesit-binasi-x120.ifc`, hata ayıklama derlemesi):
  - 11 400 nesne, 8 160 gövde, 368 640 üçgen, 16 parça.
  - Bir duvar seçilince 16 parçadan 12'si kimliğini korudu, 4'ü yeniden yüklenir. Önceden 16'sının hepsi yeni
    kimlikle yükleniyordu.
- **Sınama:** `a_scene_made_again_keeps_the_ids_of_the_tiles_it_did_not_change`. 2 km arayla iki kutu, her biri bir
  parça. Aynısı yeniden kurulunca iki kimlik korunur; ikincisi seçilince yalnız onun parçası yeni kimlik alır. Her
  parça kimliğiyle bulunur.

