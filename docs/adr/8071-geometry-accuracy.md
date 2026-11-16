# ADR 8071: Geometri doğruluğu — gerçek yay, kaynaktan miktar, kapalı gövde

- **Durum:** uygulandı (16 Kasım 2026): BIM-10'un bütün maddeleri ve ilk kabulü; IFC-06'nın kalanı ve ayrıntılar "Sıradaki
  adımlar"da. Plan: [bim-plan.md](../../plan/bim-plan.md) BIM-10,
  [ifc-uyum.md](../../plan/ifc-uyum.md) IFC-06'nın gövde kısmı (§8 "Gövdeler"); uygulama sırasında B10. Bu ADR maddenin
  bütün adımlarını tutar; koddaki "docs/adr/8071 §N" atıfları bölümleri gösterir: §1 gerçek yay, §2 kaynaktan miktar,
  §3 kapalı gövde ve ilk kabul, §4 açıklığı kesilmiş gövde ve iki çıktı (IFC-06), §5 döşemede boşluk ve şaft, §6 eğimli
  döşeme, §7 kapalı mahal hacmi, §8 çatının katmanları, §9 döner merdiven gerçek yayla, §10 içe dönen saçak, §11 çatı
  altındaki mahal. Kararların numarası bölümler boyunca sürer. Ortak sınamalar ve sıradaki adımlar
  numarasız son bölümlerdedir (yeni adım numaralı bölüm olarak araya girer).
- **Bağlam:** ADR [8047](8047-ifc-building-export.md) (binanın IFC'si, kayıp listesi), [8042](8042-section-derived-from-a-section-mark.md)
  (kesit modeli), [0340](0340-wall-joins-and-edits.md) (duvar birleşimleri), [8040](8040-vertical-model.md) (duvarın
  ayağı ve tepesi), [8067](8067-building-3d-view-and-room-overlaps.md) (3B görünüm), [8070](8070-missing-building-elements.md)
  (rampa, baca ve niş).
- **Dayanak:** Dijital Proje Yönetmeliği (RG 5.8.2026/33331) EK-5 §5.4 (IFC 4.3 esas); EK-8 §8.1 (LOD 300); EK-9 satır
  14 (mahaller kapalı); EK-6'nın isteğe bağlı satırları: Tablo 6.133 duvar (PDF s. 216–217), 6.123 döşeme (s. 197–198),
  kiriş (s. 114–115), kolon (s. 135–136), 6.88 kaplama (s. 143–144). Metinler `mevzuat/bim/ek-5-…`, `ek-6-3-…`, `ek-6-4-…`,
  `ek-8-…`, `ek-9-…`. Kılavuz §23.3 (ölçü kaynaktan, görüntüden değil).
- **Web:** dokunulmadı (ADR 0228).

## §1. Gerçek yay

**Araştırma.**

- **EK-8 §8.1:** ruhsat modelinde varlıklar "şekil, boyut, konum ve yönelimi açısından net olarak tanımlanır. Bu
  temsilden doğrudan ölçüm yapılabilir"; LOD 300. Yayı kirişlere bölen gövdeden ölçülen alan ve hacim kirişlerin
  payı kadar eksiktir (5° adımda 6 m yarıçaplı 30 cm duvarda %0,02).
- **IFC 4.3, `IfcIndexedPolyCurve`:** yalnız doğru ve çember yayı parçalarından oluşan sınırlı eğri; `IfcLineIndex`
  ilk ve son noktası arasındaki noktalardan geçen doğrular, `IfcArcIndex` "the first index is the start point of the
  circular arc, the second index is a point on arc, the third index is the end point"; "the three points shall not be
  co-linear". Son parçanın son indisi ilk parçanın ilk indisiyse eğri kapalıdır. Gayriresmî önerme: art arda iki nokta
  aynı yerde olamaz. Kaynak: buildingSMART'ın IFC4.3.x-development deposu (`docs/schemas/resource/IfcGeometryResource`);
  belge sitesi otomatik okuyucuya güvenlik doğrulaması gösteriyor, aşılmadı.
- **Üreticiler (karşılaştırma için):** Archicad'in IFC çevirmeni "Geometry Conversion" seçeneğinde BREP'i "the most
  exact reproduction of element geometry" diye anlatır ama düzenlenemez yapar; parametrik (ekstrüzyon) seçenek
  kalınlığı, yüksekliği, referans çizgisini korur; "Export gross geometry" kapalıyken delikli eleman BREP'e döner
  (help.graphisoft.com, AC27 "Geometry Conversion for IFC Export"). Revit'te eğri profil parçalı yazılır; OSArch'ın
  "Revit and IFC Geometry" sayfası "It is not currently possible to modify the number of facets created in a curve"
  der; IFC4 Reference View üçgenlemeyi esas alır, eğri biçimlerde süpürme katılarını da kapsamına alır. BricsCAD'in
  B-spline'ı üçgenleme anahtarı vardır. KentOS'un kararı: ekstrüzyonun profili gerçek yayla; parça yalnız görüntüde.

**Karar.**

1. **Yayların kaynağı kesit modelidir** (`sections::Model::exact`, her katıya paralel). Döşemenin ve kaplamanın
   halkaları parçalarından bükümleriyle gelir (önceden bükümlü kenar tek kirişti: yalnız köşeleri). Duvarın gövdesi
   birleşim çözücüsünün (wall_join) düz parçalı halkasıdır; yayları geri alınır (`geom::arc_recover::recover`): bir
   duvar çemberinin üstünde, aynı yöne dönen, her biri en çok adım (5°) kadar en az iki kirişlik bir koşu, koşunun orta
   köşesinden geçen tek yaydır; bütün çemberi dolaşan halka iki yaydır; tek kiriş düz kenardır (öbür duvarın eğrisini
   düz kesen uç). Düzleştirme köşeleri çemberin üstüne koyduğundan (10⁻⁹ göreli) geri alma kesindir ve birleşim kodu
   bükümleri taşımak zorunda kalmaz.
2. **Birleşimde yay kendi çemberinde uzar** (`wall_join::cut_end`): yay biten çizgi gönye ya da T hedefine teğetiyle
   değil, kendi çemberi boyunca uzatılır; çember hedefe hiç değmiyorsa eskisi gibi teğetiyle. Önceden eğri duvarın
   köşesinde düz bir parça kalıyor, köşe noktası yüzden 3·10⁻⁴ m dışarı düşüyordu (kabul örneğinin kuzey duvarında
   3,9·10⁻⁵ m³).
3. **IFC:** `Shape::Prism` her halkanın bükümlerini taşır. Yaylı halka `IFCCARTESIANPOINTLIST2D` üstünde
   `IFCINDEXEDPOLYCURVE` olarak yazılır: yay `IFCARCINDEX((baş, orta, son))` (orta nokta bükümden, `segment_mid`), her düz
   koşu tek `IFCLINEINDEX`, eğri ilk noktasında kapanır; düz halka eskisi gibi `IFCPOLYLINE`. Halkaların iç içeliği
   yayların kendi çevresinden kararlaştırılır (düzleştirilmiş halka yalnız bunun için). Mahalin halkaları parçalarının
   yaylarıyla yazılır. Eğri duvardaki açıklığın yüzler boyunca kenarları yüzlerin yaylarıdır
   (`arc_recover::along`: duvarın çemberlerinden birinin iki noktası arasındaki kenar o çemberin kısa yayı); önceden
   kirişti ve okuyucunun farkında 1 m'lik pencerede 2 cm'lik duvar şeridi kalıyordu.
4. **Görüntü parça sayısını kendi seçer:** 3B ağ yayı çekirdeğin adımıyla (5°) böler. Kesit düzleştirilmiş katıyı keser
   ve eğri duvarı ve döşemeyi kendi kaybı olarak yine söyler ("eğri duvar ya da merdiven (düz parçalarla)"). IFC'nin
   kayıp listesi eğri duvarı ve döşemeyi artık anmaz; yalnız eğri merdiveni ve yayı geri alınamayan nesneyi sayar.
5. **Art arda aynı köşe yok:** rampanın sahanlığını kesen kırpma, kesme çizgisine düşen köşeyi her kenar için ayrı
   yazıyordu (aynı nokta üç kez); bir kez yazar. §3'ün denetimi buldu.

**Sınama.** `arc_recover` (çeyrek halka geri gelir, alanı 0,9π'ye 10⁻¹²; tam çember iki yay; kare düz kalır; eğri
duvardan geçen açıklığın iki yayı), `building::tests::a_prism_s_arcs_are_written_as_arc_segments` (dizinler
`(IFCARCINDEX((1,2,3)),IFCLINEINDEX((3,4)),IFCARCINDEX((4,5,6)),IFCLINEINDEX((6,1)))`, orta noktalar çemberde, polyline
yok), `mesh::tests::a_curved_prism_is_a_closed_mesh_of_its_flattened_arcs` (ağın hacmi kirişlerin payı kadar).

## §2. Kaynaktan miktar

**Araştırma.**

- **EK-6 (isteğe bağlı):** duvarda Uzunluk "Orta hat boyunca (duvar yolundan farklı olsa bile)", Genişlik ve Yükseklik
  yalnız sabitse, Net Cephe Alanı "orta düzleminin cephe görünümünden görülen Alan'ı. Açıklıklar gibi tüm nesne
  değişikliklerini hesaba katar", Brüt Hacim "Açıklıklar, girintiler, kapalı nesneler ve çıkıntılar dikkate alınmaz",
  Net Hacim "açıklıklar ve girintiler dikkate alınarak". Döşemede Genişlik (kalınlık, prizmatikse), Brüt Alan
  "Elemanın ekstrüzyon (uzatılmış) alanı", Net Alan "Açıklıklar, girintiler ve kesit boşlukları çıkarılır", Brüt ve Net
  Hacim; Uzunluk ve Derinlik yalnız dikdörtgende. Kolon ve kirişte Uzunluk, Kesit Alanı, Brüt ve Net Hacim; kaplamada
  Brüt ve Net Alan.
- **IFC 4.3 (`reference_schemas/psd/Qto_WallBaseQuantities.xml`, `…Slab…`):** Length "Total nominal length of the wall
  along the wall center line (even if different to the wall path)"; GrossVolume "without taking into account the
  openings and the connection geometry"; NetVolume "after subtracting the openings and after considering the connection
  geometry"; döşemenin GrossArea'sı "Openings, recesses and projections are not taken into account", NetArea'sı
  "Openings and recesses are taken into account by subtraction". EK-6'nın Türkçesi duvarın brüt hacminde birleşim
  geometrisini anmaz; EK-5 §5.4'ün esası IFC 4.3'tür, onun tanımı izlenir.

**Karar.**

6. **Duvar** (`kentos_ifc::quantities::wall`, `Qto_WallBaseQuantities`): Length orta hattın boyu (yolun hizasına ve
   kaydırmasına göre ötelenmiş, yaylar kendi boylarıyla); Width; Height (KentOS duvarında sabit); NetSideArea
   `Length × Height` eksi her açıklığın orta hattı kestiği boy çarpı yükseklik payı (orta düzleme varmayan niş bir şey
   almaz); GrossVolume duvarın kendi gövdesi (iki yüzünün arası, birleşimsiz) çarpı yüksekliği; NetVolume dosyanın
   yazdığı gövde (birleşmiş, kolonlarca kesilmiş) eksi açıklıklar ve girintiler: gövde açıklıkların denizlik ve lento
   kotlarında bantlara ayrılır, her bandın tabanından o bantta duran açıklıklar çekirdeğin örtüşme işlemiyle (yayları
   yay olarak tutar) çıkarılır.
7. **Döşeme** (`Qto_SlabBaseQuantities`): Width katmanlarıyla toplam kalınlık; GrossArea dış halkalar (döşemenin deliği
   onun açıklığıdır: "dikkate alınmaz"); NetArea delikler ve döşemeyi baştan sona geçen açıklıklar çıkmış; GrossVolume
   GrossArea × Width; NetVolume bantlarla. Uzunluk, Derinlik ve Çevre yazılmaz.
8. **Kolon, kiriş, kaplama:** kolon Length (yüksekliği), CrossSectionArea, Gross ve NetVolume; kiriş Length (orta hattı),
   CrossSectionArea (genişlik × yükseklik), Gross ve NetVolume; kaplama GrossArea ve NetArea.
9. **Hiçbir ölçü üçgenden ya da düzleştirilmiş halkadan gelmez.** Halkanın alanı köşelerin ayakkabı bağı toplamı artı
   her yayın çember parçası, `r²/2 (θ − sin θ)`. Mahalin net alanı planın ölçüsüdür (değişmedi); yaylı odada kapalı
   formla aynı.

**Sınama.** `quantities::tests` (eğri duvarın brüt hacmi `(r₁² − r₂²)/2 · π/2 · h`, 30°–40° arası pencereyle net hacmi
10⁻⁹ içinde; düz duvarda pencere orta düzlemden düşer, 10 cm'lik niş düşmez, ikisi de hacimden).

## §3. Kapalı gövde ve ilk kabul

**Karar.**

10. **Kapalılık denetimi** (`kentos_ifc::closed::faults`): prizma, profilinin her halkası basit kapalı eğriyse
    (düzleştirilmiş en az üç köşe, alanı var, hiçbir kenar komşusu dışındakini kesmiyor ya da ona değmiyor, komşusuna
    yalnız ortak köşesinde; yaylar yay olarak) ve halkalar birbirini kesmiyorsa, derinliği de varsa kapalıdır. Yüz
    kümesi, her yönlü kenarı bir kez bu yönde bir kez ters yönde karşılanıyorsa kapalıdır. Kusur sözle söylenir (hangi
    halka, hangi kenarlar, nerede).
11. **İlk kabulün örneği** `fixtures/arch/v1/curved-building.kcad`: [`curved_building.py`](../../scripts/fixtures/curved_building.py)
    KentOS kodu olmadan yazar; kapalı formlarını `curved-building.json`'a koyar. 10 × 6 m plan, kuzey duvarı 2 m dışa
    bombeli yay (büküm 0,4; (5; 0,75) çevresinde 7,25 m); Zemin ±0,00 (3,20 m), 1. kat +3,35 (2,90 m); 30 cm, katmansız,
    ortalı duvarlar köşelerde gönyeyle; güneyde pencere, doğuda kapı, eğri duvarda yarıçaplara dik söveli pencere;
    döşemeler dış yüzlerden, yay kenarı yay (zeminde 5 cm kaplamalı, 1. katta 2 × 1 m delikli, çatıda); her katta iç
    yüzlerin içinde bir oda. Duvarların ayağı ve tepesi ADR 8040 §1–2'nin kuralıyla (zemin katta yapısal döşeme üstü
    −0,05'ten 1. kat döşemesinin altı +3,20'ye).

| Gövde | Kapalı form | Değer |
|---|---|---|
| Zemin kuzey duvarı, brüt | (7,40² − 7,10²)/2 · 4 atan 0,4 · 3,25 | 10,758818 m³ |
| Zemin kuzey duvarı, net | (gönyeli dörtgen + dış parça − iç parça) · 3,25 − pencere (7,40² − 7,10²)/2 · (1,2/7,25) · 1,2 | 10,327506 m³ |
| Zemin doğu duvarı, net | gönyeli dörtgen · 3,25 − kapı 1,0 · 0,30 · 2,1 | 5,219616 m³ |
| 1. kat döşemesi | brüt alan 78,787332 m² (dikdörtgen + dış yayın parçası), net 76,787332 m² | 11,8181 / 11,5181 m³ |
| Oda | iç yüzlerin dörtgeni + iç yayın parçası | 68,876951 m² |

**Sınama.** `arch_ifc::the_curved_building_s_bodies_are_their_closed_forms`: her duvarın ve döşemenin Qto'su, her
gövdenin altı ve üstü (±0,00'a göre), her odanın net alanı kapalı forma 10⁻⁹ göreli içinde; eğri duvarlar ve üç döşeme
yaylı, iki pencere yüzlerin yaylarıyla; bütün gövdeler kapalı; dışa aktarma "düz parçalarla" demiyor.
[`curved_ifc_check.py`](../../scripts/fixtures/curved_ifc_check.py) dosyayı IfcOpenShell 0.9.0'la okur: doğrulayıcı
(EXPRESS kuralları dahil) 0 bulgu; 8 duvar, 3 döşeme ve 2 odanın miktarları kapalı formlar; profillerde `IfcArcIndex`;
IfcOpenShell'in kendi geometrisi (açıklıkları kendi Boole işlemleriyle çıkarır) 13 kapalı yüzey ve hacimleri kapalı
formlara 5,8·10⁻⁷ içinde. B9'un örnek binasında (rampa, giydirme cephe, asansör, dikme, kayıt, asma tavan) ve çatılı
kesit binasında (çatı yüzleri, baca) bütün gövdeler kapalı.

## §4. Açıklığı kesilmiş gövde ve iki çıktı

**Araştırma.**

- **IFC 4.3, `IfcOpeningElement` ve `IfcRelVoidsElement`:** açıklığın 'Body' gösterimi oyduğu elemanın gövdesinden
  çıkarılır (örtük Boole farkı); 'Reference' gösterimi "is not subtracted. It is provided in addition to the hole in
  the Body shape representation of the voided element". buildingSMART'ın "Element Openings" kavramı da Reference
  gösterimini dışa aktarırken boşluğu zaten içeren gövdeyle anlatır.
- **ADR 8047 §6 ve ifc-uyum.md §8:** başlık bugün `DesignTransferView` (gövde bütün, açıklığı okuyucu çıkarır); hedef iki
  çıktı profili, başlıktaki görünüm adı dosyanın içeriğini söyler; ruhsat profilinin hangisi olacağı B1'in sayfa okuması
  ve pilot idarenin cevabıyla seçilir, o güne kadar ikisi de sınanır.
- **Üreticiler (karşılaştırma için):** Archicad'in "Export gross geometry of elements (Export holes separately)"
  seçeneği gövdeyi bütün, deliği `IfcOpeningElement` olarak ayrı yazar; kapalıyken "extruded elements with holes are
  exported as BREP". KentOS'ta kesilmiş gövde de ekstrüzyon kalır: bantlara ayrılmış, her bandın profili yaylarıyla.

**Karar.**

12. **Kesilmiş gövde** (`kentos_ifc::quantities::cut`): gövde açıklıkların denizlik ve lento kotlarında bantlara ayrılır,
    her bandın tabanından o bantta duran açıklıklar çekirdeğin örtüşme işlemiyle çıkarılır (yaylar yay kalır); açıklığın
    durmadığı ardışık bantlar tek prizma kalır. Net hacim (§2), 3B ağ (ADR 8067'nin bant kesimi artık bu işlev; yayları
    ağ kendi adımıyla böler) ve Reference View aynı işlevi kullanır: dört görüntü aynı gövdeyi gösterir.
13. **İki çıktı** (`Building::view`): Tasarım aktarımı (varsayılan) gövdeyi bütün, açıklığı 'Body' yazar, başlık
    `ViewDefinition [DesignTransferView]`; Reference View gövdeyi kesilmiş, açıklığı 'Reference' yazar, başlık
    `ViewDefinition [ReferenceView]`. Nesneler, kimlikler, miktar setleri ve hacimler ikisinde aynıdır.
14. **Komut ve pencere:** `cad.arch.ifc`'nin girdisinde `view` (`designTransfer` | `reference`), çıktısında yazılan
    görünüm; dosyaya yazılan her gövde (ve mahal) kapalılık denetiminden geçer, kapalı olmayan adı ve ilk kusuruyla
    `ifc_open_body` uyarısıdır. Masaüstünde Dosya › Dışa aktar › Bina modeli'nde "IFC dışa aktar (bina, kesilmiş
    gövde)…" (`arch.ifcExportCut`, simgesi seçenek sayfasının A'sı: kutunun yüzünde açıklık) var olan "IFC dışa aktar
    (bina)…"nın yanında; ikincisinin açıklaması gövdelerin bütün yazıldığını söyler.
15. **Ruhsatın profili açık:** varsayılan Tasarım aktarımı kalır; hangisinin ruhsat profili olacağı plan §8'in koşuluyla
    seçilir.

**Sınama.** `quantities::tests::a_curved_wall_s_cut_body_is_its_bands_with_their_arcs` (pencereli çeyrek halka: lentonun
altı ve üstü bütün, pencere bandı iki yaylı parça, hepsi kapalı, toplamları net hacim),
`building::tests::a_reference_view_writes_the_wall_cut_and_its_opening_as_reference` (başlık, 'Reference', kapılı duvarın
üç bandı), `arch_ifc::the_curved_building_s_bodies_are_their_closed_forms` (her duvarın kesilmiş gövdesi kapalı formun net
hacmi; Reference View dosyası `egri-bina-referans.ifc`), `curved_ifc_check.py` iki dosyayı da okur (Reference View'da
IfcOpenShell'in ağları kapalı formlara 5,2·10⁻⁷ içinde, açıklıklar yalnız 'Reference'), masaüstünde
`arch_ifc_tests::the_cut_bodies_go_out_as_a_reference_view`, ortak durumlar `cad.arch.ifc` (26; ikisi Reference View).
Resim: `scene3d_building_tests::ifc_export_menu_screens` (`5-disa-aktar-*.png`).

## §5. Döşemede boşluk ve şaft

**Araştırma.**

- **ADR 0708:** boşluk mahali (şaft, asansör boşluğu, merdiven ve galeri boşluğu) "döşemesi yoktur"; her katta ayrı
  boşluk çizilir. O ADR döşemenin kendisini kapsam dışı bırakmıştı: döşeme boşluğun altında dolu kalıyordu.
- **Duvarın ayağı (ADR 8040 §1–2):** bir katın döşemesi, üst yüzü katın tabanında olan (yarım metre altından 10 cm
  üstüne) döşemedir; boşluğun kestiği döşeme de budur.
- **IFC 4.3 ve EK-6:** döşemenin deliği `IfcOpeningElement`'tir (`IfcRelVoidsElement`), duvarınki gibi; Qto'da brüt
  alan açıklıkları "dikkate almaz", net alan çıkarır (§2). EK-5'in açıklık kodu ACL.

**Karar.**

16. **Boşluk katının döşemelerini keser** (kesit modeli, `sections::slab`), metrajın okuduğu gibi (ADR 0722 §5: bir
    döşeme kendi katının boşluk mahallerini, katsız döşeme hepsini okur): çekirdeğin örtüşme işlemiyle çıkarılır (yaylar
    korunur); metraj, kesit, görünüş, 3B ve IFC aynı döşemeyi gösterir. Boşluğun döşemeye düşen parçası
    `Model::slab_cuts`'ta durur. Kaplama ve eğimli döşeme kesilmez. Üst üste katlarda şaft her katın kendi boşluğudur
    (ADR 0708); katın üstündeki teras da o katın döşemesi olduğundan ışıklık gibi açılır. Üstü kapalı bir şaftın çatısı
    ayrı bir katta çizilir (kat arası şaft ADR 0708'in dışında kaldı).
17. **IFC:** döşemenin gövdesi kendi dış çizgisidir (çizilmiş delikleri ve boşluk kesikleri olmadan); her çizilmiş delik
    "MM-ACL-Döşeme_boşluğu", her boşluğun parçası boşluğun numarası ve adıyla ("MM-ACL-1S1_Şaft") döşemenin açıklığıdır,
    döşemenin bütün kalınlığınca. Tasarım aktarımında okuyucu çıkarır, Reference View'da gövde kesiktir. Dışa aktarmanın
    `openings` sayısı bunları da sayar.

**Sınama.** `arch_ifc::a_shaft_takes_its_floor_out_of_its_storey_s_slab` (kesit binasının 1. katında 1 × 1 m şaft: o katın
döşemesi ve üstündeki teras kesilir, zemininki kesilmez; gövdeleri tek halka; açıklıkları; net alanlar
12,25 × 9,25 − 2,85 × 3,94 − 1 ve 12,25 × 9,25 − 1; kesik gövdeler kapalı ve net hacimleri; iki görünümde de kapalı). Kesit binasının IFC'sinde 15 açıklık (14 duvarda, 1 döşemede),
`arch_ifc_check.py` döşeme açıklıklarını çizimin delik sayısıyla karşılaştırır. Resim: `shaft_screens`
(`6-saft-*.png`, terası çıkarılmış 1. kat üstten).

## §6. Eğimli döşeme

**Araştırma.**

- **EK-6 Tablo 6.123:** Pset_SlabCommon.PitchAngle "Döşeme, çatı bileşeni olarak kullanıldığında yataya göre eğim açısı
  (… kullanılmayan durumlarda 0 derece olarak belirtilir veya belirtilmez)"; Qto'da Genişlik "Sadece, nesne sabit
  kalınlığa (prizmatik) sahipse".
- **Üreticiler (karşılaştırma için):** Revit'in döşeme eğim oku: kuyruğu sınırın üstünde, "Height Offset at Tail" ve
  "Height Offset at Head" ya da eğim (help.autodesk.com, "Create a Sloped Surface Using a Slope Arrow"); Archicad'in
  döşeme aracı eğim almaz, yardım merkezi eğik üst yüz için çatı, değişken eğim için arazi aracını önerir. KentOS'un
  kararı: döşemenin kendi eğim oku.

**Karar.**

18. **Eğim oku** (`SlabFields::slope`, `SlabSlope { from, to, rise }`): üst yüz ok boyunca eşit yükselir; kuyrukta
    döşemenin kot farkı, başta `rise` kadar fazlası; aynı düzlem okun dışına da sürer. Kalınlıklar (yapı ve katmanlar)
    üst yüze dik ölçülür, düşeyde √(1 + eğim²) katı; yanlar düşey. Eğim en çok %100 (45°: daha diki duvar ya da
    çatıdır), yükseliş en çok ±100 m, ok en az 1 mm; delikli döşemeye eğim verilmez (kesitin görünen yüz çizimi eğik
    yüzün deliğini tutmuyor; delik ayrı döşeme parçalarıyla çizilir). `.kcad` mimari verisi 7.
19. **Modelin her yerinde:** kesit modeli her katmanı eğik plak (`Sloped`, yüz kümesi kendinin) olarak kurar; IFC kapalı
    yüz kümesi yazar (`IfcPolygonalFaceSet`), PitchAngle'ı (radyan) ve miktarlarını kaynaktan (Genişlik dik kalınlık,
    alanlar plan alanı × √(1 + eğim²), hacim plan alanı × dik kalınlık × √(1 + eğim²)); metraj alanları eğim boyunca
    okur; altındaki duvar duvarın ortasındaki yapısal alt yüzde biter (ADR 8040 §1'in kuralı, noktasında okunur).
20. **Arayüz:** Öznitelikler'de "Eğim" (yüzde; 0 yatay) ve eğimliyse "Eğim yönü" (projenin açı biriminde, yükseldiği
    yön): ok, döşemenin o yöndeki en alçak noktasından en yükseğine kurulur. `cad.slabs.create` ve `cad.slabs.edit`
    `slope` alır (null: yatay).

**Sınama.** `arch_ifc::a_sloped_slab_is_a_sloped_plate_of_its_closed_forms` (4 × 3 m, 20 cm, 3 m'de 30 cm: eğik plak 3,00
→ 3,30; altındaki duvar 3,00 + 0,15 − 0,2 · √1,01'de biter; kapalı yüz kümesi, hacmi 12 · 0,2 · √1,01; Qto ve PitchAngle;
3B ağı aynı hacim), `curved_ifc_check.py`'nin eğimli bölümü `egimli-doseme.ifc`'yi IfcOpenShell'le okur (0 bulgu, yüzey
kapalı, hacim 10⁻⁹, kotlar), `.kcad`'ın `a_sloped_slab_round_trips_under_arch_data_7`'si, masaüstünde
`a_slab_s_slope_is_typed_in_percent_and_turned_by_its_direction`, ortak durumlar (`cad.slabs.create` 70, `cad.slabs.edit`
53; eğimli olanlar ve retleri). Resimler: `sloped_slab_screens` (`7-…`, `8-…`).

## §7. Kapalı mahal hacmi

**Araştırma.**

- **EK-9 satır 14:** "Mahaller kapalı olarak ve üst sınırları doğru tanımlandı mı?"; bim-plan Z6: mahalin hacmi kat
  yüksekliğinde düz prizma (tavanı bilinse de).
- **EK-6 Tablo 6.125 (isteğe bağlı):** Yükseklik "Karakteristik yükseklik"; Bitmiş Tavan Yüksekliği "döşeme üst kotundan
  asma tavan alt kotuna kadar. Yalnızca mahalde sabit yükseklikte bir asma tavan varsa"; Brüt Çevre "dış sınırı boyunca
  … Döşeme seviyesinde"; Brüt ve Net Kat Alanı; Brüt ve Net Hacim.
- **ADR 8040 §3:** mahalin tabanı (kendi kot farkı, altındaki döşemenin bitmiş üstü, katın tabanı) ve tavanı (asma tavan,
  yazılan iç yükseklik, üstteki döşemenin sıvalı altı, yoksa katın üstü: yalnız bir sınır) zaten bulunuyor; IFC bunu
  kullanmıyordu.

**Karar.**

21. **IFC mahali tabanından tavanına** (`kentos_ifc::arch::RoomLevels`, `Document::room_height`): gövde bitmiş tabandan
    tavana kapalı prizma; tavan bilinmiyorsa (yalnız katın üstü) gövde yine oraya kadar, ama yükseklik ve hacim
    yazılmaz (değer uydurulmaz).
22. **Miktarlar** (Qto_SpaceBaseQuantities): NetFloorArea (planın ölçüsü, değişmedi), GrossFloorArea ve GrossPerimeter
    halkalarından yaylarıyla (çevre dış halkalarınki), tavan biliniyorsa Height (iç yükseklik), GrossVolume ve
    NetVolume (alanlar × yükseklik); asma tavanlı odada FinishCeilingHeight.
23. **Sınır:** çatının altındaki oda çatının eğik alt yüzüyle kesilmez (tavanı yoksa katın üstüne kadar): çatıyla kesilmiş
    mahal hacmi sıradaki adımlarda.

**Sınama.** `arch_ifc::a_room_is_closed_from_its_floor_to_its_ceiling` (kesit binasının zemin odası ±0,00'dan 2,76'ya;
asma tavanla 2,60; bütün mahal gövdeleri kapalı), eğri binanın odaları kapalı formlarıyla (alan, iç yayın boyuyla çevre,
3,20 ve 2,90 yükseklik, hacimler; IfcOpenShell da okur).

## §8. Çatının katmanları

**Araştırma.**

- **EK-2 Tablo 2.3** (`mevzuat/bim/ek-2-dosya-isimlendirme.md`): Çatı CATI, Çatı Kaplama CKAP, Çatı Kaplama Tahtası
  CKTA, Çatı Taşıyıcı CTAS, Isı Yalıtımı ISYA, Su Yalıtımı SUYA. Katmansız çatı tek öğedir (CATI, ADR 8044); katmanlı
  çatının her katmanı kendi öğesindedir.
- **Mimarlık Hizmetleri Şartnamesi 14.5.4 q** (uygulama projesi kesitleri): "Çatı konstrüksiyonu … çizilir"; ADR 8044
  çatı katmanlarının (kiremit, çıta, tahta, mertek) ayrı çizilmesini buraya bırakmıştı.
- **IFC 4.3 `IfcMaterialLayer`** (buildingSMART IFC4.3.x-development, `IfcMaterialLayer.md`): katman sabit kalınlıklıdır;
  Material verilmemişse "hava boşluğu demek değil, malzeme o noktada belirtilmemiş" demektir; LayerThickness'in anlamı
  kullanımına bağlıdır; Category için genel sözcükler 'LoadBearing', 'Insulation', 'Inner finish', 'Outer finish'.
  **`IfcRoof`** (`IfcRoof.md`): "çatı her yüz için döşemelere ayrılabilir"; parçaları varsa çatının kendi gövdesi
  olmaz, gövdesi parçalarının toplamıdır.
- **TS 5319** (ADR 8014): SN55 yalıtım malzemesi, SN23 ağaç malzeme; SN53 seramik kitaplıkta yok.
- **Üreticiler (karşılaştırma için):** Archicad'in bileşik yapısında çatının "üst" yüzü ilk katmandır ve çatının
  kalınlığı katmanların toplamıdır (help.graphisoft.com, AC25 "Composite Structures"); Revit'in bileşik yapısında her
  katmanın işlevi (Structure, Substrate, Finish…), malzemesi ve kalınlığı vardır, zar katmanı 0 kalınlık alabilir
  (help.autodesk.com, "Compound Structure", "Edit Layers in a Compound Structure").
- **Değerler:** kaynaklı hazır katman takımı bulunamadı (birim fiyat pozları yalnız latanın kesitini veriyor; tahtanın ve
  merteğin kalınlığı projeye göre); kalınlıklar yazılır, KentOS önermez.

**Karar.**

24. **Katmanlar** (`RoofFields::layers`, `RoofLayer { kind, thickness, material }`): üstten aşağı kaplama, lata (ADR
    8044'te çıta), su yalıtımı, kaplama tahtası, ısı yalıtımı, mertek sırasıyla, her tür en çok bir kez; her biri EK-2
    öğesinde (CKAP, CKAP, SUYA, CKTA, ISYA, CTAS). Kalınlık çatı yüzüne dik ölçülür (tahtanın, merteğin kendi
    kalınlığı), düşeyde √(1 + eğim²) katı. Katmanlı çatının kalınlığı katmanlarının toplamıdır, ayrıca verilmez;
    katmansız çatı ADR 8044'teki gibi tek kalınlıktır (düşey ölçülür, yoksa 0,25 m). Katman sıfırdan büyük, en çok 1 m
    (zar katmanı yok: kesitin plağı hacim ister; 0 katmanı kaldırır); toplam en çok 2 m; malzeme boş ya da kenarı
    boşluklu olamaz. `.kcad` mimari verisi 8.
25. **Kesitte:** her yüzde her katman bir eğik plak, üstündekinin altında, kendi EK-4 katmanında (MM-CKAP-KIREMIT,
    MM-CKAP-GENEL, MM-CKTA-CAM…: malzemesi, kaplamanınki verilmemişse çatının kaplaması, öbürlerininki GENEL); mertek
    çatının taşıyıcısı olarak tek kalınlıktaki çatı gibi SN23 ile taranır ve kalın çizilir; öbürleri ince çizgiyle
    (kiremidin SN53'ü kitaplıkta yok, kaplama ağaç diye taranmaz). Pencere ve kar siperliği çizgileri ve yüzey adı en üst
    katmandadır.
26. **Kesitte ısı yalıtımı katmana oturur** (`geom::zigzag::fitted`): kesilen her yalıtım bandı TS 5319 SN55'in tek
    zikzağıdır, bandın iki uzun yüzü arasında, adımı kalınlığın yarısı, noktaları iki yüzde sırayla (ADR 8014'ün planda
    duvar katmanı için koyduğu kural); bant, dışbükey zarfının en dar olduğu kenar boyunca uzanır, kalınlığı o
    genişliktir; çizgi bandın çevresine kırpılır; kâğıtta 1 mm'den ince bant simgesiz kalır. Döşemelerin kesilen yalıtımı
    da artık böyledir. Kitaplığın 2 mm'lik döşenmiş satırları ince ve eğik bantta kırık parçalara bölünüyordu (sahibin
    16 Kasım 2026'daki uyarısı: "çizgiler kırık").
27. **IFC'de:** katmanlı çatı tek malzeme yerine bir `IfcMaterialLayerSet` ("Çatı katmanları") taşır, katmanlar üstten
    aşağı: Name türün sözcüğü; Material verilen malzeme (kaplamanınki verilmemişse çatının kaplaması), verilmemişse boş;
    LayerThickness yüze dik kalınlık; Category IFC 4.3'ün sözcükleriyle: kaplama 'Outer finish', ısı yalıtımı
    'Insulation', mertek 'LoadBearing', öbürleri boş. Takım `IfcRoof`'a bağlanır; gövdesi çatının kendisindedir (ADR
    8044), her yüzün katmanları kapalı plaklardır.
28. **Arayüz:** Öznitelikler'de "Çatı katmanları" bölümü: altı tür üstten aşağı, kalınlığı yazılan katman yerine girer
    ya da değişir, boş ya da 0 kaldırır; katmanın hemen altında "Malzeme"; "Toplam kalınlık". Geometri'de "Kalınlık"
    katmansız çatıda yazılır, katmanlıda katmanlarının toplamıdır ("çatı yüzüne dik" notuyla). Her değişiklik
    `cad.arch.edit`'in `layers`'ı ile tek geri alma adımıdır; `cad.roofs.create` de `layers` alır. Yeni komut, araç ya da
    şerit düğmesi yok.

**Sınama.** `geom::zigzag`'ın dört sınaması (yatay, eğik, dik bant; ince bant simgesiz); `kentos-ifc`'nin
`a_layered_roof_is_one_material_layer_set`'i (adlar, boş malzeme, roller, bir malzeme bir kez); `roof_sections::a_layered_roof_is_its_layers_plates_cut_in_the_section_and_one_ifc_layer_set` (kesit binasının
%33'lük kırma çatısı beş katmanla: dört yüzde beşer plak, düşeyde t · √1,1089, her biri üstündekinin altında, EK-4
katmanları; B-B'de yalıtımın zikzağı her dişi √(0,05² + 0,025²) (döşemelerinki kendi kalınlıklarıyla), mertekte SN23,
her katmanın çizgisi; IFC'de takım; gövdeler kapalı); `curved_ifc_check.py`'nin çatı
bölümü `katmanli-cati.ifc`'yi IfcOpenShell'le okur: 0 bulgu, takım beş katmanıyla geri okunur, yüzey kapalı, hacim
13,25 · 10,25 · 0,26 · √1,1089 (10⁻⁹), en alt 6,00 − 0,26 · √1,1089, en üst 6,00 + 0,33 · 10,25 / 2; `.kcad`'ın
`a_layered_roof_round_trips_under_arch_data_8`'i (sırası bozuk ve kalınlığı ayrıca verilen reddedilir); masaüstünde
`a_roofs_layers_are_edited_in_their_section` (altı geri alma adımı); ortak durumlar (`cad.roofs.create` 67,
`cad.arch.edit` 96: katmanlı çatı, retleri, katman verme ve kaldırma). Resimler: `roof_layers_screens`
(`.run/gui-evidence/cati-katmanlari-8071/`: B-B, saçağın yakını, 3B; açık ve koyu) ve `roof_properties_screens`
(`cati-katmanlari.png`).

## §9. Döner merdiven gerçek yayla

**Araştırma.**

- **ADR 0709** (döner merdiven): yürüme çizgisi iki noktalı bir yay, basamak derinliği onun yayı boyunca, rıhtlar
  radyal; **ADR 0724 §3** merkez direği (çapı); **ADR 8045 §2** dengeli merdivenin balansman basamağı kendi gövdesi,
  alt yüzü kademeli (betonun eğik alt yüzü modellenmez). Kesit modeli döner merdiveni hiç kurmuyordu ("düz parçalarla"
  notuyla bırakıyordu); IFC'ye gövdesiz gidiyordu.
- **IFC 4.3 `IfcStairTypeEnum`** (`IfcStairTypeEnum.md`): SPIRAL_STAIR "dairesel bir göbek çevresinde balansmanlı,
  çoğu kez sahanlıksız", yürüme çizgisi dönüşü belirler; CURVED_RUN_STAIR "dönüşsüz, tek eğri kol". KentOS'ta direkli
  döner merdiven ilki, direksiz ikincisidir (B9, `turns::of_stair`).
- **BIM-10:** "parça sayısı görüntünün işidir, kaynağın değil".

**Karar.**

29. **Basamak bir halka parçası** (`sections::curved_stair`): iki rıhtı arasında, yürüme çizgisinin merkezine göre iç ve
    dış kenarı gerçek yay (`Model::exact`; IFC'de `IfcArcIndex`), yanları radyal; plan alanı yayın boyu × genişlik.
    Balansman basamağı gibi durur: üstü kendi kotu, altı bir rıht ve merdiven plağı aşağısı (plak, vardığı ya da
    çıktığı döşemenin taşıyıcısı), katın tabanının altına inmez; plak yoksa tabana kadar dolu. Merkez direği katın
    tabanından merdivenin vardığı kota silindir. Hepsi merdivenin tek gövdesidir (`IfcStair` SPIRAL_STAIR; direksiz
    CURVED_RUN_STAIR). Kesit yayların kirişlerini çizer (eğri duvar gibi "düz parçalarla" notu kalır); IFC dışa
    aktarımı eğri merdiven için artık "düz parçalarla yazıldı" demez.
30. **3B görünümde yarı saydam yüz** (mahal) üzerinde durduğu opak yüzün (duvarın iç yüzü, döşemenin üstü) kılı kadar
    önünde çizilir (`DepthBiasState`, yalnız yarı saydam çizimde): iki yüzün derinliklerinin son bitleri yüzünden
    beliren benekler kalkar, mahalin rengi yüze düzgün biner.

**Sınama.** Eğri cepheli örnek binaya 1. katın 2,4 × 2,4 m boşluğundan çıkan döner merdiven eklendi (`curved_building.py`:
yürüme çizgisi 0,70 m, 19 rıht × 3,35/19, basamak 22 cm, genişlik 80 cm, direk 20 cm; kapalı formlar KentOS kodu
olmadan): `arch_ifc::the_curved_building_s_bodies_are_their_closed_forms` 18 basamağın ve direğin altını, üstünü ve plan
alanını (0,22 · 0,80 ve π · 0,1²) 10⁻⁹ içinde, hacim toplamını, yayların yay olduğunu, gövdelerin kapalılığını sınar;
`curved_ifc_check.py` iki dosyada `IfcStair` SPIRAL_STAIR, 38 yay parçası, IfcOpenShell'in yüzeyi kapalı, hacmi kapalı
formla 1,04·10⁻⁶ içinde. Resim: `curved_screens`'in `5-3b-doner-merdiven-*`'i (zemin kat, 60°'den).

## §10. İçe dönen saçak

**Araştırma.**

- **ADR 8044 §2, §10:** çatının her eğimli kenarı kendi düzlemini kaldırır; dışbükey saçakta yüzler düzlemlerin alt
  zarfıdır (düz iskeletin çatısı); içe dönen (L, T biçimli) saçak bölünme olayları gerektirdiği için reddediliyordu.
- **Aichholzer, Aurenhammer, Alberts, Gärtner (1995), "A novel type of skeleton for polygons":** düz iskelet ve çatı
  modeli: kenarlar içe doğru eşit hızda yürür, bir noktanın yüksekliği yürüyen çizginin ona vardığı andır.
- **Felkel ve Obdržálek (1998), "Straight skeleton implementation":** kenar olayı (kenar sıfırlanır, komşuları
  birleşir) ve bölünme olayı (içe dönen köşe karşı kenara varır, çizgi ikiye ayrılır); algoritmanın bölünme sınaması
  sonraki uygulama çalışmalarında kusurlu bulundu (Huber ve Held, "Theoretical and Practical Results on Straight
  Skeletons of Planar Straight-Line Graphs", 2011; "On Implementing Straight Skeletons: Challenges and Experiences").
- **Biedl, Held, Huber, Kaaser, Palfrader (2015), "Weighted straight skeletons in the plane", CGTA 48(2):** her kenarın
  kendi hızıyla yürüdüğü ağırlıklı düz iskelet; eğimleri farklı yüzlü gerçek çatılar böyledir (Laycock ve Day, Kelly ve
  Wonka); çizgide kendini kesme ve aynı doğruda farklı hızlar tanımı bozar.
- **Bağımsız başvuru:** iki L'nin (kolları eşit ve farklı) yüzleri, alanları ve kotları elle çözüldü; dışbükey saçakta
  iskelet, alt zarf yöntemiyle (başka bir yol) karşılaştırıldı.

**Karar.**

31. **İçe dönen saçakta yüzler ağırlıklı düz iskeletten** (`geom::roof_skeleton`): eğimli kenarın doğrusu çatı her metre
    yükseldiğinde `1 / eğim` metre içeri yürür, kalkan yerinde durur; olaylar en erkenden: kenar sıfırlanır, içe dönen köşe
    karşı kenara varır (çizgi bölünür), alanı kalmayan döngü mahyayla kapanır (iki köşe ya da sırt sırta kenarlar). Her
    olaydan sonra bütün adaylar yeniden hesaplanır (köşe sayısının küpü; bir çatının birkaç düzinesi): bölünme kenarın o
    anki parçasına göre sınanır. Her yüz kendi kenarının düzlemindedir; derenin türü iç köşeden inmesidir; mahya kotu en
    yüksek düğümün. Dışbükey saçak bugünkü alt zarf yöntemiyle kalır (sonuçlar aynı, örnek çizimler değişmez).
32. **Ret:** kendini kesen saçak (tam yön sınamasıyla, `orient2d`); aynı doğrudaki iki kenarın eğimi farklıysa (ya da
    biri kalkansa): çatı orada basamak yapardı. "İçe dönük köşe" reddi kalktı; Öznitelikler'in "henüz hesaplanmıyor"
    satırı da.
33. **Planda ok** yüzün köşelerinin ortalamasında; içe dönen bir yüzde bu nokta dışarıda kalırsa kenarının ortasından
    içeri, yüzün karşı kenarına giden yolun yarısında. Çatı penceresinin yüzü genel içerde-mi sınamasıyla bulunur.

**Sınama.** `geom::roof_skeleton`'ın yedi sınaması (dikdörtgen kırma çatı, kalkanlı beşik, tek eğimli; eşit kollu L'nin
ve kolları farklı L'nin elle çözülmüş yüz alanları 16, 4, 12, 8, 4, 12 ve 17, 4, 8, 11, 9, 15, kotları 2 ve 3; T; L'de
iki eğim ve kalkanlar; her yüzün her noktası kendi düzleminde, yüzler birlikte saçağın alanı); `geom::roof`'ta
`an_eaves_line_turning_in_has_a_valley` (altı yüz, bir dere, iki mahya, beş kırma, 0,66 m mahya, beş noktanın kotu,
her yüzde ok; kendini kesen çizgi reddi) ve `on_a_convex_line_the_skeleton_is_the_lower_envelope` (altı dışbükey çizgi,
karışık eğim ve kalkanlarla: yüz alanları ve köşe kotları alt zarfınkiyle 10⁻⁹ içinde aynı); uygulamada
`roof_sections::an_l_roof_turns_in_with_a_valley_through_the_section_and_the_ifc` (kesit binasında L çatı: altı eğik
plak, dere iç köşeye, B-B çatıyı çizer, IFC gövdesi kapalı ve hacmi L'nin alanı × 0,25); ortak durumlar
(`cad.roofs.create`: L, kanatları kalkanlı T, L'de baca kabul; kendini kesen ve aynı doğruda farklı eğimli saçak
reddi). Resimler: `l_roof_screens` (`.run/gui-evidence/ic-donen-sacak-8071/`: plan, güneybatı, kuzeydoğu).

## §11. Çatı altındaki mahal

**Araştırma.**

- **EK-9 satır 14:** "Mahaller kapalı olarak ve üst sınırları doğru tanımlandı mı?" Üstünde döşeme olmayan, çatının
  altındaki mahalin üst sınırı çatının alt yüzüdür; §7'de bu mahal kattan kata bir prizmaydı, hacmi yazılmıyordu.
- **IFC 4.3 `Qto_SpaceBaseQuantities`** (buildingSMART IFC4.3.x-development, `reference_schemas/psd`): Height "To be
  provided only if the space has a constant height"; FinishCeilingHeight yalnız sabit yükseklikte asma tavanda;
  GrossCeilingArea ve NetCeilingArea eğik tavanda gerçek alan; GrossVolume ve NetVolume mahalin çevrelediği hacim.
- **EK-6 Tablo 6.125:** Yükseklik "Karakteristik yükseklik"; tavan alanları "gerçek alandır (izdüşüm alanı değil; örneğin
  eğimli tavanlarda)".
- **ADR 8040 §3:** mahalin tavanı asma tavan, yazılı iç yükseklik, üstündeki döşemenin sıvalı altı, yoksa katın üstü
  ("tavan bilinmiyor"); **ADR 8044 §1:** çatı katının üstünde (ve kot farkında) durur.

**Karar.**

34. **Tavanı başka yoldan bilinmeyen mahal** (asma tavanı, yazılı yüksekliği, üstünde döşemesi yok) kendi katında duran
    bir çatının altındaysa çatının alt yüzüne kadar yükselir (`kentos_ifc::under_roof`): planı çatının yüzleriyle
    parçalanır (yaylar korunur), her parça tabanından o yüzün alt düzlemine kadar kapalı bir yüz kümesidir. Alt düzlem
    üst düzlemden yapısı kadar aşağıdadır: tek kalınlık düşey ölçülür, katmanlar yüze dik (× √(1 + eğim²)), kesit
    modelinin plakları gibi; düz çatı tek düzlemdir. Mahalin bir kısmı hiçbir yüzün altında değilse ya da bir parçasında
    delik varsa mahal eskisi gibi prizma kalır.
35. **Miktarlar:** Height yazılmaz (sabit değil); GrossVolume her parçanın alanı çarpı alt düzlemin parçanın ağırlık
    merkezindeki tabana yüksekliği (bir düzlemin bir alan üstündeki ortalaması ağırlık merkezindeki değeridir: yaylarla
    birlikte tam); NetVolume net alan brüte eşitse aynı, değilse yazılmaz (düşülen elemanların hangi yükseklikte durduğu
    bilinmiyor; değer uydurulmaz). Tavan alanları henüz yazılmıyor.
36. **Kapsam:** IFC gövdesi ve miktarları ile 3B ağı. Mahalin iç yüksekliği (PAİY 28 denetimi), metraj ve kesit
    değişmez: eğik tavanlı mahalin yükseklik kuralı ayrı karardır, kaynağı bekleniyor.

**Sınama.** `under_roof`'un iki sınaması (4 × 3 m mahal tek eğimli çatı altında 39 m³, iki parça, kapalı; yarım daire
mahal tam yayıyla); `roof_sections::a_room_with_no_slab_over_it_rises_to_the_roof` (kesit binası teras döşemesiz ve
kırma çatılı: 1. katın beş mahalinin GrossVolume'u çatının alt zarfının 1 200 × 1 200 ızgarada tümlenmesiyle 10⁻⁵
içinde, NetVolume aynı, Height yok, gövde parçaları kapalı ve hacmi aynı; zemin kat prizma); `curved_ifc_check.py`'nin
çatı altı bölümü `cati-alti-mahal.ifc`'yi IfcOpenShell'le okur: 0 bulgu, yüzeyler kapalı, GrossVolume Python'un kendi
ızgarasıyla 10⁻⁵ ve IfcOpenShell'in hacmiyle 10⁻⁹ içinde, Height yok. Resim yok: 3B'de mahaller çatının ve duvarların
içinde kalıyor, görünümün kesit kutusu henüz yok (B6'nın kalanı).

## Ortak sınamalar

- `cargo test -p kentos-ifc --lib` (45), `-p kentos-geometry-core` (1 117), `-p kentos-interaction wall` (81),
  `-p kentos-native-application --test all -- sections elevation roof arch_ifc scene mesh slab wall ramp`.
- Ortak durumlar: `KENTOS_CASES_ONLY=cad.walls,cad.sections,cad.modules,cad.spaces,cad.slabs,cad.openings,cad.arch,cad.elevations,cad.roofs,cad.stairs,cad.columns,cad.beams,cad.ramps`
  `every_command_case` 2 995 durum. B9'dan eskimiş iki dosya yenilendi: `cad.sections.edit` (rampa ve niş artık
  çizilir, "çizilmeyenler"de değil) ve `cad.modules.edit.runs` (kitaplık bloklarının kartında IFC sınıfı).
- `scripts/fixtures/arch_ifc_check.py` kesit binasının iki dosyasında yeni miktar setleriyle 0 bulgu.
- Resimler: `scene3d_building_tests::curved_screens` (`.run/gui-evidence/egri-bina-8071/`: plan, 3B, güneybatı, zemin kat
  üstten; açık ve koyu tema).

## Sıradaki adımlar

- **IFC-06'nın kalanı:** gelişim seviyesi kanıtının müellif incelemesi kaydı (EK-9 formuyla, IFC-08 ve BIM-14);
  "sentetik örnek açık kural takımından uyarısız geçer" kabulü buildingSMART'ın doğrulama servisini ya da açık kural
  takımını ister: dosyayı servise yüklemek ya da kural takımını indirmek sahibin onayıyla.
- İçe dönen yüzde çatı penceresi: köşeleri yüzdeyse kabul ediliyor, kenarının dereyi aşıp aşmadığı sınanmıyor.
- **Çatı yüz başına `IfcSlab`:** IFC 4.3'ün "her yüz için döşemelere ayrılabilir" yolu; katman takımı her yüze
  `IfcMaterialLayerSetUsage` ile; çatının kendi gövdesi o zaman kalkar.
- Kiremit kaplamanın TS 5319 SN53 "seramik" taraması (kitaplığa girince); çatının tek kalınlığının da yüze dik ölçülmesi
  (ADR 8044 düşey ölçüyor; örnek çizimler değişir: sahibin kararı).
- Eğimli döşemenin planda eğim oku simgesi; delikli eğimli döşeme (kesitin eğik yüz çizimi deliği tutarsa).
- Döner merdivenin helis alt yüzü (şimdi ADR 8045 §2'nin kademeli alt yüzü) ve kesitte yayının kendi çizimi.
- Mahalin Net Çevre'si (kapı ve sanal sınırlar düşülmüş), duvar ve tavan alanları (çatı altında gerçek alan); eğik
  tavanlı mahalin iç yükseklik kuralı (PAİY 28; kaynak bekleniyor).
- Kapının açıklığı bitmiş döşemeden (±0,00) başlar, duvar yapısal döşemeden; kapının altında kaplama kalınlığında duvar
  kalır (zemin katta 5 cm). Açıklığın alt kotu duvarın ayağına mı inmeli, ayrı karar.
