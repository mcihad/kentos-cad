# ADR 8070: Eksik yapı elemanları — rampa, merdiven ve rampa tipleri, baca ve niş, aks ızgarası, kitaplık blokları, kaplama, giydirme cephe, dikme ve kayıt, kaplama nesnesi

- **Durum:** kısmen uygulandı (16 Kasım 2026). Plan: [bim-plan.md](../../plan/bim-plan.md) BIM-09,
  [ifc-uyum.md](../../plan/ifc-uyum.md) IFC-03; uygulama sırasında B9. Bu ADR maddenin bütün adımlarını tutar; her adım
  kendi bölümüdür ve koddaki "docs/adr/8070 §N" atıfları bu bölümleri gösterir: §1 rampanın gövdesi, §2 merdiven ve
  rampanın tipi ve malzemesi, §3 baca ve niş, §4 aks ızgarası, §5 kitaplık blokları, §6 kaplama, §7 giydirme cephe,
  §8 dikme ve kayıt, §9 çizelgeler, dışa aktarmanın sayıları ve ilk kabul, §10 kaplama ve asma tavan nesnesi, §11 kiriş
  ve kaplama çizelgesi. Kararların numarası bölümler boyunca sürer (1–40). Birden çok adımın ortak sınamaları ve sıradaki
  adımlar numarasız son bölümlerdedir (yeni adım numaralı bölüm olarak araya girer).
- **Bağlam:** ADR [8047](8047-ifc-building-export.md) (binanın IFC'si, kayıp listesi), [8042](8042-section-derived-from-a-section-mark.md)
  (kesit modeli), [0705](0705-ramps-by-slope-and-railings.md) ve [0723](0723-ramp-landings-levels-accessibility.md) (rampa),
  [8044](8044-roof-and-roof-plan.md) §6 (çatı bacası), [8045](8045-balanced-stairs-and-special-elements.md) §5 (asansör
  bloğu), [8068](8068-fire-performance-material-declarations.md) (zorunlu alanlar), [8008](8008-ek4-layer-key.md)
  (Mimari katmanları).
- **Dayanak:** Dijital Proje Yönetmeliği (RG 5.8.2026/33331) EK-2 Tablo 2.3 (öğeler); EK-5 Tablo 5.1; EK-6 §6.1.3 ve
  Tablo 6.14 (baca), 6.20 (kaplama), 6.21 (giydirme cephe), 6.35 (mobilya), 6.36 (aks), 6.40 (eleman), 6.49 (rampa), 6.53
  (sıhhi terminal), 6.61 (merdiven), 6.64 (taşıma elemanı), 6.89 (giydirme cephenin isteğe bağlı alanları); metinler
  `mevzuat/bim/ek-2-dosya-isimlendirme.md`, `ek-5-ifc-siniflari-varlik-isimlendirme.md`, `ek-6-1-zorunlu-a-f.md`,
  `ek-6-2-zorunlu-f-z.md` ve `ek-6-3-istege-bagli-a-f.md`.
- **Web:** dokunulmadı (ADR 0228).

## §1. Rampanın gövdesi

**Araştırma.**

- **EK-6 Tablo 6.49 (rampa, PDF s. 89):** tanım: "farklı kotlardaki iki döşeme seviyesini birbirine bağlayan … ara bir
  döşeme plağı olarak bir sahanlık içerebilir."
- **EK-5 Tablo 5.1:** rampa RMP, merdiven MRD; `IfcRampFlight` ve `IfcStairFlight` listede yok: rampa ve merdiven tek
  nesne, gövdesi kolları ve sahanlıklarıyla.

**Karar.**

1. **Rampanın gövdesi** (kesit modeli, `sections::ramp`): her eğimli kol bir eğik plak (çatı yüzünün kesitte
   kullandığı `Sloped`), düzlemi kolun başındaki kottan sonundakine çıkar; yükseklik kollara eğimli uzunluklarıyla
   paylaştırılır (planın oku ve kotları gibi). Her sahanlık kendi kotunda düz bir plak. Kalınlık merdivenin plağı
   gibi: vardığı döşemenin taşıyıcısı, yoksa çıktığı döşemeninki, o da yoksa döşemenin kendi kalınlığı (0,15 m). Zemine
   oturan rampanın altındaki dolgu zemindir, model zemini tutmaz (EK-4 4.30); bu yüzden "yere kadar dolu" çizilmez.
   Kesit, görünüş ve 3B aynı gövdeyi okur; kesit artık "rampa çizilmedi" demez, IFC'nin kayıp listesinde rampa yoktur.

**Sınama.**

- **Uygulama** (`sections::a_ramp_s_runs_slope_its_landing_stands_flat_and_the_model_writes_it`): bölüm binasının
  doğusunda 6 m doğuya, 4 m kuzeye giden, 1,50 m genişliğinde, 0,75 m yükselen L rampa. Beklenenler kesirlerle elle:
  eğimli uzunluk 8,5 m, eğim 3/34, sahanlık 63/136 m; kollar 0,15·√(1 + (3/34)²) dik kalınlıkta; y = 2'den kuzeye bakan
  kesit ikinci kolu x = 35,25…36,75 arasında, üstü 39/68 m'de keser. IFC'de tek `IfcRamp`, QUARTER_TURN_RAMP,
  Betonarme, üç gövde, kayıp listesinde yok.
- **Dijital denetim:** rampa artık "modele girmeyen nesne" değil; sınama örneği yüksekliği olmayan bir duvara geçti.

## §2. Merdiven ve rampanın tipi, malzemesi (K42)

**Araştırma.**

- **EK-6 Tablo 6.49 (rampa, PDF s. 89):** ön tanımlı tip, ad, etiket, malzeme ("Malzeme İsmi_Dayanım"); tip listesi
  STRAIGHT RUN RAMP, TWO STRAIGHT RUN RAMP, QUARTER TURN RAMP, TWO QUARTER TURN RAMP, HALF TURN RAMP, SPIRAL RAMP,
  USERDEFINED, NOTDEFINED.
- **EK-6 Tablo 6.61 (merdiven, PDF s. 102):** aynı dört öznitelik, `Pset_StairCommon` FireRating ve FireExit; tip
  listesi STRAIGHT RUN … TWO CURVED RUN STAIR.
- **IFC 4.3'ün kendi tanımları** (buildingSMART, IFC4.3.x-development deposu, `IfcRampTypeEnum.md`,
  `IfcStairTypeEnum.md`; resmî belge sitesi bu bilgisayardan 403 veriyor): STRAIGHT_RUN_RAMP tek düz kol; TWO_STRAIGHT_RUN
  iki düz kol ve bir sahanlık, dönüşsüz; QUARTER_TURN 90° dönen iki kol ve çeyrek sahanlık; TWO_QUARTER_TURN 180° dönen
  üç kol ve iki çeyrek sahanlık; HALF_TURN 180° dönen iki kol ve yarım sahanlık; SPIRAL çevresinde dönülen boşluk,
  sahanlıksız. Merdivende WINDING türleri **tek kol** sayılır ("one flight with quarter winders"): dengelenmiş basamaklar
  kolun basamaklarıdır. SPIRAL_STAIR bir dikme çevresinde, CURVED_RUN_STAIR tek eğri kol.

**Karar.**

2. **Merdiven ve rampanın tipi** (`kentos_ifc::turns`): yürüme çizgisinin kolları ve iki kol arasındaki her durakta
   (sahanlık ya da balansman) çeyrek dönüş sayısı (+ sola). Rampada 1 kol → STRAIGHT_RUN, 2 kol ve dönüşsüz sahanlık →
   TWO_STRAIGHT_RUN, ±1 → QUARTER_TURN, ±2 (değen iki sahanlık bir sayılır) → HALF_TURN, 3 kol ve aynı yöne iki çeyrek →
   TWO_QUARTER_TURN. Merdivende aynısı ve THREE_QUARTER_TURN; balansmanlı duraklar kol sayılmadan WINDING türlerini
   verir; yaylı merdiven dikmesiyle SPIRAL_STAIR, dikmesiz CURVED_RUN_STAIR. Listede karşılığı olmayan (üç düz kol, dik
   açı dışında dönen çizgi, sahanlık ve balansman birlikte) USERDEFINED'dır ve `ObjectType` ne olduğunu söyler ("3
   kollu, 2 sahanlıklı rampa"): tip tahmin edilmez, NOTDEFINED tip sayılmaz (ADR 8068).
3. **Malzeme (K42):** merdiven ve rampa, kesitin çizdiği gibi betonarmedir; IFC'ye "Betonarme" yazılır (kolon ve
   kirişin ADR 8047 kararıyla aynı). Projenin Malzemeler'inde Betonarme'nin dayanımı yazılıysa ad "Betonarme_C30" olur.
   Nesneye kendi malzemesi alanı geldiğinde o yazılır. İdarenin teyidi beklenir.

**Sınama.**

- **`kentos-ifc`** (`turns`, 4): düz çizgi, L'nin çeyrek dönüşü, sahanlıkları değen U'nun yarım dönüşü, ortası eğimli
  U'nun iki çeyreği, düz çizgide sahanlık, 45°'lik dönüşün reddi, üç düz kol (USERDEFINED, "3 kollu, 2 sahanlıklı
  rampa"), sola ve sağa dönen balansmanlar, uçlara uzanan balansman (tek kol), sahanlıkla balansman birlikte (yok).
- **Denetim olguları** (`fixtures/audit-facts/v1/mimari.json`): merdivenin ön tanımlı tip ve malzeme eksikleri düştü
  (yalnız bu iki satır elle çıkarıldı; kaçış yolunun önceden bilinen 11,55 / 4,45 farkı görünür bırakıldı).

## §3. Baca ve niş (K43)

**Karar.**

4. **Niş ve duvardaki baca kesitte** (`sections::recess_ring`, `Recess::band`): duvardan kalınlığının bir kısmında
   çıkarılan kutu; niş yüzünden derinliği kadar, baca iç yüzünden `inset` (yoksa ortada) ve derinliği kadar; denizliğinden
   (yoksa döşeme) başlığına (yoksa duvarın tepesi). Çizgi kestiği yerde boşluk görünür, yüzü görülmez (açıklık gibi).
   Kesit artık "niş ve baca kesilmedi" demez.
5. **Baca (`IfcChimney`, EK-6 Tablo 6.14):** duvardaki bacanın boşluğu duvarını boydan boya boşaltan bir açıklıktır
   (`IfcOpeningElement` OPENING, kimliği bacanın kimliğinden türetilir) ve bacası onu doldurur (`IfcRelFillsElement`).
   Çatının duman ve hava bacası (ADR 8044 §6) çatının oturduğu kattan tepesine bir gövdedir; artık çatının gövdesinde
   değil kendi nesnesindedir. **Tek yapı elemanı:** biri ötekinin planındaki dış hattının içine ortası düşen parçalar
   (alt alta katların bacaları ve üstündeki çatı bacası) bir bacadır; katı en alttaki parçanınki, gövdeleri o katın
   döşemesine göre. Ön tanımlı tip listesi yalnız USERDEFINED ve NOTDEFINED olduğundan USERDEFINED, `ObjectType` "Duman
   bacası" ya da "Hava bacası" (çatı bacası yoksa "Baca"). GlobalId en alttaki duvar bacasının kimliği; yalnız çatı
   bacasıysa çatının kimliği ve parça sırasından türetilir, 3B'de tıklanınca çatı seçilir (`Element::object`).
6. **Bacanın malzemesi (K43):** en alttaki bacanın duvarının çekirdek malzemesi; duvar bacası yoksa, kesitin tuğla
   taramasıyla çizdiği gibi "Tuğla" (ADR 8044 §6). İdarenin teyidi beklenir.
7. **Niş (`IfcOpeningElement` RECESS):** duvarını boşaltır, dolduranı yoktur; GlobalId nişin kimliği. Niş ve baca
   kayıp listesinden çıktı.

**Sınama.**

- **Uygulama** (`roof_sections::a_flue_in_its_walls_and_the_chimney_over_it_are_one_ifc_chimney_and_a_niche_a_recess`):
  bölüm binasının çatısında (4,5; 8,0)'de 40 × 40 cm duman bacası, altındaki batı hol duvarında (İç duvar 15) iki katta
  7 cm'lik ortada baca, doğu hol duvarında 0,90–1,50 m arasında 5 cm'lik niş. Kesit modelinde baca duvarın her
  katmanından 4,465 ≤ x ≤ 4,535, 7,9 ≤ y ≤ 8,1 kutusu olarak çıkar (döşemeden). IFC'de tek `IfcChimney` (USERDEFINED
  "Duman bacası", MM-BAC-…, GlobalId zemin katın bacası), üç gövde (0,00; +3,00; +6,00'dan), iki OPENING onu dolduruyor,
  niş RECESS ve dolduransız; hiçbiri kayıp listesinde değil; dosyada IFCCHIMNEY ve her dolan açıklık için bir
  IFCRELFILLSELEMENT.

## §4. Aks ızgarası

**Karar.**

8. **Aks ızgarası (`IfcGrid`, `IfcGridAxis`; EK-6 Tablo 6.36):** çizimin aksları katlarına göre (katı olmayan, her katta
   görünen aks binanındır) bir ızgara olur. İlk düz aksa paralel olanlar U, ötekiler V ekseni; her takım kendi içinde
   paralel ve ikisi birbirine dik ise RECTANGULAR, değilse IRREGULAR (RADIAL ayrıca denetlenmedikçe yazılmaz). Her eksen
   etiketi (`AxisTag`: A, 1) ve çizgisiyle; yay aks `IfcIndexedPolyCurve`'ün yay parçasıyla (başı, ortası, sonu), tam.
   Izgaranın planda izi (`FootPrint`, `GeometricCurveSet`) yalnız ızgara varken açılan kendi bağlamında. IFC 4.3
   (IFC4X3_ADD2) U ve V'nin ikisini de zorunlu tuttuğundan bir katın hep aynı yöndeki aksları ızgara olamaz: kayıp
   listesine "aks" olarak, nedeniyle girer (`Why::OneWay`). Etiketsiz aks zorunlu alan eksiği olarak denetlenir.

**Sınama.**

- **Uygulama** (`arch_ifc::the_grids_are_one_rectangular_ifc_grid_and_one_direction_alone_is_left_out`): bölüm
  binasının katsız A–D ve 1–3 aksları binanın tek RECTANGULAR ızgarası (U: A–D, V: 1–3, 7 eksen, B (4,5; −2)–(4,5; 11));
  dosyada 7 IFCGRIDAXIS, FootPrint; hiçbir eksenin etiketi eksik değil. Yalnız A–D kalınca ızgara yok, 4 aks kayıpta.

## §5. Kitaplık blokları yapı elemanı olarak

**Karar.**

9. **Kitaplık blokları yapı elemanı olarak** (EK-5 Tablo 5.1; EK-6 Tablo 6.35, 6.53, 6.64): blok kartı neyin yapı elemanı
   olduğunu söyler (`BlockCard.element`: sınıfı `furniture`, `sanitaryTerminal` ya da `transportElement`; IFC 4.3'ün tip
   listesinden tipi; gövdesinin yüksekliği; alt kotu). Kartında sınıfı olmayan blok (işaret, antet, kapı ve pencere
   sembolü, otopark yeri, ev aletleri) yapı elemanı değildir ve modele girmez. Tip sınıfının listesinden olmalı
   (`BlockElement::problem`); USERDEFINED ise bloğun adı tipini söyler (`ObjectType`); tip yoksa NOTDEFINED yazılır ve
   zorunlu alan denetimi bunu eksik sayar. Gövde, bloğun o yerleştirmedeki değerleriyle çizdiğinin kutusu
   (`definition_box`), yerleştirme gibi taşınmış, döndürülmüş, ölçeklenmiş, aynalanmış; alt kotundan yüksekliği kadar
   (`sections::fixture_outline`). Yüksekliği yoksa gövdesiz yazılır. Kesit modelinde de aynı kutu, sınıfının EK-2
   öğesinde ince çizgiyle (Tefriş'in Mobilya ve Vitrifiye'si, Asansör).
10. **Kitaplığın değerleri** (`scripts/blocks/library.py`, 25 blok): lavabonun (85 cm), klozetin (42 cm, oturağı
    olmadan çanağın üstü) ve pisuvarın (70 cm, ön kenarı) üst kotu bir vitrifiye üreticisinin montaj rehberinden (Geberit,
    "The perfect installation height": lavabo 85–90 cm, klozet 41–42 cm, pisuvar ön kenarı 65–70 cm); ötekiler (masa
    0,75, sandalye 0,85, yatak 0,50, gardırop 2,20, kitaplık 2,00, mutfak alt dolabı 0,90, üst dolap 1,45'ten 0,70, duş
    teknesi 0,10, küvet 0,55, asansör kabini 2,20 m) ADR 0702'deki plan ölçüleri gibi KentOS'un kendi yaygın ölçüleridir;
    bir yönetmeliğin istediği söylenmez. Gövde döşemeden başlar; duvara asılı lavabo ve klozetin altındaki boşluk
    modellenmez (üst dolap dışında). Asansör kabininin EN 81-20'deki yüksekliği okunamadı (standart metni ücretli).
11. **Asansörün kapasitesi** (EK-6 Tablo 6.64, `Pset_TransportElementCommon`): kitaplığın asansör bloğuna gizli iki
    öznitelik: `KISI` (kişi) ve `YUK` (kg); her yerleştirme kendi değerini yazar. Boşsa model yazmaz, zorunlu alan
    denetimi eksik der; alan kabin alanından türetilmez (beyan edilen değerdir, ADR 8068). Kütle yazılan dosyaya
    kilogram birimi (`IFCSIUNIT(*,.MASSUNIT.,.KILO.,.GRAM.)`) eklenir; kütlesiz dosya olduğu gibi kalır.
12. **Sürümler:** `.kcad`'de kartın yapı elemanı mimari verisi sürüm 2'dir (`ARCH_WITH_ELEMENTS`; sürüm 1'in verisini de
    içerir): yazan `modules.arch = 2` der, sürüm 1 diyen dosyadaki yapı elemanı reddedilir; belge şeması değişmez. Blok
    kitaplığı dosyası (`kentos.blocks`) sürüm 4'tür (`BLOCKS_ELEMENT_VERSION`): eski okuyucu sınıfı düşürmek yerine
    dosyayı reddeder; kitaplık okuyucusu ve yayın yükseltmesi 1–4'ü okur.
13. **Komut ve pencere:** `cad.blocks.edit` `card`'ı `element` alır (yoksa kalır, `null` kaldırır); Blok kartı
    penceresinde "Yapı elemanı" (Yok, Mobilya, Sıhhi tesisat ucu, Taşıma elemanı), "Tip", "Yükseklik (m)", "Alt kot
    (m)". Var olan çizimlerdeki kitaplık blokları eski kartlarıyla kalır; kitaplıktan yeniden alınca ya da kartından
    sınıflanır. 3B görünümde bu sınıfların kendi rengi var ("Donatı"; döşemenin grisinde seçilmiyordu).

**Sınama.**

- **Uygulama** (`arch_ifc::the_library_s_fixtures_are_their_ifc_classes_with_their_bodies`): kitaplığın Asansör'ü (KISI
  8, YUK 630), Lavabo'su, Masa'sı ve Dolap'ı bölüm binasında IfcTransportElement ELEVATOR (MM-TSM-Asansör; kapasitesi
  `Pset_TransportElementCommon`'da 8 ve 630 kg), IfcSanitaryTerminal WASHHANDBASIN (0,85), IfcFurniture TABLE (0,75) ve
  USERDEFINED "Dolap"; asansörün gövdesi bloğun çizdiğinin kutusu (x 0,17…1,43, y 0,015…1,96), 2,20 m. Kot işareti
  yapı elemanı değil. Kesit modelinde kutuları MM-ASAN-GENEL, MM-TEFR-VITRIFIYE, MM-TEFR-MOBILYA'da; dosyada kilogram
  ve kapasite.
- **`.kcad`** (`arch_elements`, 2; kodlayıcının birim sınaması): masa bloğunun yapı elemanı mimari veri 2 ile gidip
  gelir; üst dolabın alt kotu ve USERDEFINED tipi de; sınıfı olmayan blok sürümü yükseltmez; mobilyaya lavabo tipi ve
  sıfır yükseklik yazılmaz; sürüm 1 diyen dosyadaki yapı elemanı reddedilir.
- **Ortak durumlar:** `cad.blocks.edit.attributes` 3 yeni durum (verilir, başka alan değişince kalır, `null` kaldırır;
  başka sınıfın tipi, sıfır yükseklik, eksi alt kot reddedilir; kartı olmayan bloğa yalnız yapı elemanı). Kitaplığın
  kartları değiştiği için mutfak dizisi durumları (`cad.casework.*`) yeniden yazıldı: fark yalnız kartlara eklenen yapı
  elemanı. `cad.blocks.edit`'te bilinmeyen kitaplık sürümü örneği 5'e geçti.
- **Masaüstü** (203 süzgeçli sınama; `block_card::the_building_element_is_chosen_typed_and_taken_away`): sınıf ve tip
  seçilir, sınıf değişince öbür sınıfın tipi düşer, sayı olmayan yükseklik komuttan önce reddedilir, "Yok" kaldırır, her
  biri bir geri alma.
- **Resimler:** `.run/shots/arac-blok-karti-yapi-elemani-*` (Blok kartı: Mobilya, TABLE, 0,75); 3B
  `.run/gui-evidence/eksik-elemanlar-8070/` (çatı, baca, rampa; zemin katta asansör, lavabo, masa; üstten). Çok katlı baca
  en alttaki parçasının katında olduğu için yalnız zemin kat gösterilince bütün olarak görünür.

## §6. Kaplama (K44)

**Karar.**

14. **Kaplama (K44; EK-6 Tablo 6.20 dipnotu):** kaplamanın malzemesi ve kalınlığı "(1) IfcSpace içindeki tanımlardan
    faydalanılarak ya da (2) IfcCovering'de … doğrudan modelleme ile" verilir, "bu iki yöntemden biri zorunludur".
    KentOS birinci yolu tutar: mahalin `Pset_SpaceCoveringRequirements`'i kaplama adlarına (oda listesinin, ADR 8038)
    kalınlıklarını da ekler: döşeme kaplamasınınki altındaki döşemenin en üst katmanı kaplama ise onun kalınlığı,
    tavanınki üstündeki döşemenin altındaki sıva (`Document::room_coverings`, ADR 8040 §2). Asma tavanlı mahalin tavan
    kalınlığı yazılmaz (asma tavanın katmanları çizilmiyor); duvar kaplamasının kalınlığı henüz yok (mahale bakan yüzün
    katmanı). Döşemenin kaplaması ve tavanın sıvası zaten `IfcSlab`'ın gövdesinde olduğundan ayrıca `IfcCovering`
    yazılmaz (gövdeyi ikilerdi). Elle çizilen kaplama ve asma tavanın kendi gövdesi sonraki adımdır.

**Sınama.**

- **Uygulama** (`arch_ifc::a_room_carries_its_floor_finish_and_ceiling_plaster_thicknesses`): bölüm binasının mahalleri
  döşeme kaplaması 0,02 (zemin ve 1. katın döşemesinin üst katmanı) ve tavan sıvası 0,02 taşır
  (`IFCPOSITIVELENGTHMEASURE`); 2,60 m'de asma tavanlı mahalin tavan kalınlığı yok. Denetim olgularının çiziminde
  döşeme katmanı olmadığından eksikleri değişmedi.

## §7. Giydirme cephe (K45)

**Araştırma.**

- **EK-6 Tablo 6.21 (giydirme cephe, PDF s. 61):** "zemine oturmaktan ziyade döşeme/çatı strüktürünün kenarına asılmış
  bileşenlerin bir araya gelmesiyle oluşturulan bir bina duvarı"; bir bina elemanı bileşimi olarak `IfcRelAggregates`
  ile kurulur; iç giydirme cephe `Pset_CurtainWallCommon.IsExternal` ile söylenir. Tip listesi yalnız USERDEFINED ve
  NOTDEFINED. Zorunlular: ön tanımlı tip, ad (EK-5), etiket, malzeme ("Malzeme İsmi_Dayanım"), `Pset_CurtainWallCommon`
  FireRating. Tablo 6.89 (isteğe bağlı): `Qto_CurtainWallQuantities` Width ve Height.
- **EK-6 Tablo 6.40 (eleman, `IfcMember`):** tip listesinde MULLION, POST, PLATE…; zorunlular ön tanımlı tip, ad,
  etiket, malzeme ve `Pset_MemberCommon.IsExternal`. IFC 4.3'te MULLION giydirme cephede panelleri birbirine bağlayan
  doğrusal elemandır; dikme ve kayıt ikisi de MULLION'dır.
- **EK-5 Tablo 5.1:** giydirme cephe GDC, eleman ELM; `IfcPlate` listede yok, yani panelin kendi sınıfı yok.
  **EK-2 Tablo 2.3:** Giydirme Cephe öğesi GIYC.
- **Üretici belgeleri** (yalnız iş akışı ve ızgara kuralı için; değer alınmadı): Revit'in giydirme cephe tip
  özellikleri (Autodesk yardımı, "Curtain Wall Type Properties", 2020, GUID-F6A2FCBB-E0AD-4E33-9572-F605FCDD7E8A): her
  yönde ızgara düzeni "Fixed Distance", "Maximum Spacing" (duvar boyunca eşit aralıklarla, verilen aralığı aşmadan) ya
  da "Fixed Number"; dikme her ızgara çizgisine konur, kenar ve iç dikmenin tipi ayrı; "Join Condition" hangi yönün
  kesintisiz süreceğini seçer; panel tipin panelidir, tek tek değiştirilebilir. Archicad'in giydirme cephe çerçeveleri
  (Graphisoft yardımı, "Curtain Wall Frames", AC 27, 040_ElementsVB-151): ızgara şemadan; çerçeve sınıfları köşe,
  sınır (kenarda her zaman), dikme (sütunlarda) ve kayıt (satırlarda); kesişimde önceliği yüksek çerçeve ötekini keser.

**Karar.**

15. **Izgara** (`CurtainGrid`, `geom::curtain`): giydirme cephe bir duvardır ve ızgarasını taşır (`WallFields.curtain`:
    `spacing`, `rows`, isteğe bağlı `frame`, `panel`, `frames`). Yolunun her düz parçası, hiçbiri `spacing`'den geniş
    olmayan en az sayıda eşit bölmeye ayrılır (en büyük aralık kuralı: bir parçanın panelleri eşittir, ucunda artık
    parça kalmaz); yüksekliği aynı kuralla `rows`'tan eşit sıralara. Izgaranın her çizgisinde bir çerçeve vardır, uçlar,
    alt ve üst de içinde (kenar çerçevesi her zaman). Dikmeler alttan üste kesintisizdir, kayıtlar dikmeler arasında
    kesilir. Uçtaki çerçeve parçanın içinde, içtekiler çizgisinin ortasında durur; paneller çerçeve yüzleri arasındadır.
    Aralıklar 0,3–10 m; çerçeve yüzü sıfırdan büyük ve aralıkların yarısından küçük (bir panele yer kalır); yaylı kenar,
    bileşen ve boş malzeme adı reddedilir (`CurtainGrid::problem`, `WallFields::problem`).
16. **Ölçüler:** verilmeyen çerçeve yüzü 0,05 m; panel 0,024 m kalınlığında, çerçeve derinliğinin ortasında; aracın
    ilk aralıkları 1,50 m (dikme) ve 1,20 m (kayıt). Bunlar KentOS'un kendi yaygın ölçüleridir; bir yönetmeliğin ya da
    üreticinin istediği söylenmez. Duvarın genişliği (Kalınlık) çerçevenin derinliğidir. Araç paneli "Cam", çerçeveyi
    "Alüminyum" yazar; ikisi de değiştirilir.
17. **Plan, kesit, görünüş ve 3B:** planda yüzleri ve uçları duvarınki gibi, cam ortada bir çizgi, dikmeler yüz
    genişliğinde ve duvarın derinliğinde; tarama yok (`geom::wall::curtain_pieces`). Kesit modelinde dikme, kayıt ve
    panel ayrı katılardır (`sections::curtain`, `Part::Mullion/Transom/Panel`); EK-2'nin Giydirme Cephe öğesinde,
    taramasız: çerçeveler MM-GIYC-CERCEVE, cam MM-GIYC-CAM. Kesit kestiğini keser, görünüş yüzlerini görür; 3B'de
    paneller pencere camının, çerçeveler kirişin renginde.
18. **IFC (K45):** `IfcCurtainWall` USERDEFINED, `ObjectType` "Giydirme cephe" (listede başka öğe yok); adı EK-5'e göre
    (GDC ve duvar stilinin adı, stil yoksa "Giydirme cephe"), etiketi kimliği, malzemesi panelinki. **Panellerin kendi
    nesnesi yazılmaz, giydirme cephenin gövdesidir:** EK-5'te panel sınıfı (`IfcPlate`) olmadığından ayrı panel nesnesi
    listede olmayan bir sınıf olurdu. Dikme ve kayıtlar `IfcMember` MULLION (adı "ELM Dikme" / "ELM Kayıt", malzemesi
    çerçeveninki, GlobalId duvarın kimliği ve sırasından türetilir); `IfcRelAggregates` ile giydirme cephenin
    parçalarıdır ve katta ayrıca yer almazlar. Bir çerçevesi katın dış hattına değen giydirme cephe dışarıdadır
    (`Pset_CurtainWallCommon.IsExternal`), çerçeveleri de onunla (`Pset_MemberCommon.IsExternal`). FireRating
    duvarlarınki gibi beyandan (ADR 8068). İdarenin teyidi beklenir.
19. **Açıklık:** giydirme cepheye kapı ve pencere açıklığı konmaz (`not_a_wall`, "Giydirme cepheye açıklık konmaz:
    kapısı ve penceresi paneli olarak gelecek; açıklığı bir duvara koyun."). Panelin yerine kapı ve pencere sonraki
    adımdır.
20. **Sürüm:** `.kcad`'de giydirme cephenin ızgarası mimari veri sürüm 3'tür (`ARCH_WITH_CURTAIN_WALLS`; 1 ve 2'nin
    verisini de içerir): yazan `modules.arch = 3` der, daha küçük sürüm diyen dosyadaki ızgara reddedilir; belge şeması
    değişmez.
21. **Komut, araç ve pencere:** `cad.walls.create` duvarın `curtain`'ını alır (stilin bileşenleri ve dolgusu gelmez,
    genişliği gelir); `cad.walls.edit` de alır: verilince duvar giydirme cephe olur, bileşenleri ve dolgusu düşer;
    `null` onu düz duvara çevirir. Araç `tool.curtainWall` "Giydirme cephe" (kısa adı Giydirme, yazılı adı GC):
    Duvar'ın yolu, seçenekleri Kalınlık (E), Hiza (H), Öteleme (Ö), Panel eni (P), Panel boyu (B); Stil yok. Nesnesi
    Mimari katman anahtarıyla EK-2'nin Giydirme Cephe katmanına (MM-GIYC-GENEL) gider (ADR 8008). Şeritte Mimari › Duvar
    panelinde, Ölçülü bölme'den sonra; ikonu çerçeve içinde 3 × 3 ızgara ve camın üç yansıma çizgisi (seçenek
    sayfasından C). Öznitelikler'de "Tür" (Duvar, Giydirme cephe), Panel eni, Panel boyu, Çerçeve, Panel malzemesi,
    Çerçeve malzemesi; her biri bir geri alma; Tür'den giydirme cepheye çevrilen duvar aracın ilk ızgarasını alır.

**Sınama.**

- **Çekirdek** (`geom::curtain`, 2; `geom::wall::a_curtain_wall_draws_its_faces_its_glass_and_its_mullions`): 6 m 1,5'ta
  dört eşit bölme, 6,1 m beş tane 1,22 m; 1 m tek bölme. Planda 6 m'lik, 0,15 m'lik duvarın iki yüzü, ortada cam
  çizgisi (y 0), beş dikme (x 0–0,05; 1,475–1,525; 2,975–3,025; 4,475–4,525; 5,95–6; y ±0,075), tarama yok.
- **Uygulama** (`sections::a_curtain_wall_is_its_mullions_transoms_and_panels`): bölüm binasının doğusunda 6 m, 0,15 m
  derinliğinde, 3 m yüksekliğinde, 1,50 / 1,20 m aralıklı giydirme cephe. Elle: 6 / 1,5 = 4 eşit bölme (5 dikme), 3 /
  1,2 = 2,5 → 3 eşit sıra (1,00 m, 4 kayıt çizgisi). Beş dikme beklenen x'lerde; 16 kayıt (4 bölme × 4); 12 panel,
  ilki x 20,05–21,475, y ±0,012, z 0,05–0,975. İlk bölmeyi kesen kesit 4 kayıt ve 3 panel keser. IFC'de bir
  `IfcCurtainWall` (USERDEFINED, "Giydirme cephe", 12 panel gövdesi, Cam), 21 `IfcMember` MULLION (Alüminyum, hepsi
  onun parçası), ikisi de dışarıda; dosyada 1 IFCCURTAINWALL ve 21 IFCMEMBER; kayıp listesinde değil; kapı açıklığı
  `not_a_wall` ile reddedilir.
- **Bağımsız okuyucu:** bu sınamanın yazdığı dosya (`KENTOS_WRITE_IFC_CURTAIN`) IfcOpenShell 0.9.0.post1'in
  doğrulayıcısından 0 bulguyla geçti.
- **`.kcad`** (`arch_elements::a_curtain_wall_round_trips_under_arch_data_3`): ızgara `modules.arch = 3` ile gidip
  gelir; panele yer bırakmayan çerçeve yazılırken reddedilir.
- **Etkileşim** (`curtain_wall_tool::the_curtain_wall_tool_draws_a_wall_with_its_grid`): seçenekler Panel eni ve Panel
  boyu, Stil yok; P 1,25, B 11 reddedilir (eski değer kalır), B 1,5; iki tıklamanın duvarı bileşensiz, ızgarası 1,25 /
  1,5, Cam ve Alüminyum. Katman anahtarının listesine MM-GIYC-GENEL eklendi.
- **Masaüstü** (`properties::tests::arch::a_wall_turns_into_a_curtain_wall_and_its_grid_is_typed`): Tür duvarı aracın
  ilk ızgarasıyla giydirme cepheye çevirir (bir geri alma), yazılan Panel eni `cad.walls.edit` ile gider, Tür Duvar
  ızgarayı kaldırır. Öznitelikler'in sıra sınaması ("Tür", "Kalınlık") güncellendi.
- **Ortak durumlar:** `cad.walls.create`'e 20 giydirme cephe durumu (ızgara olduğu gibi saklanır; stille bileşen ve
  dolgu gelmez; çerçeve ve malzemeler; köşeli ve kapalı yol; kat ve yükseklik; en küçük ve en büyük aralık; düz büküm;
  düz duvarla tek çağrıda; plan; aralığın, çerçevenin, boş malzemenin, yaylı kenarın ve bileşenin reddi),
  `cad.walls.edit`'e 3 durum (verilir, `null` kaldırır, ret). `cad.walls.edit.parts`'ın "değiştirilecek alan yok"
  iletisi ızgarayı da sayar. Dokunulan komutların 698 durumu geçti.
- **Resimler:** `.run/gui-evidence/giydirme-cephe-8070/` (plan, doğu görünüşü, kuzeydoğudan 3B; açık tema) ve ikon
  seçenek sayfası.

## §8. Dikme ve kayıt (K46)

**Araştırma.**

- **EK-6 Tablo 6.40 (eleman, `IfcMember`, PDF s. 79):** "destek noktaları arasında veya ötesinde yük taşımak üzere
  tasarlanmış yapısal bir elemandır. Taşıyıcı olması zorunlu değildir"; `IfcBeam` ya da `IfcColumn` olarak ifade
  edilemiyorsa kullanılır. Zorunlular: ön tanımlı tip, ad, etiket, malzeme ("Malzeme İsmi_Dayanım"),
  `Pset_MemberCommon.IsExternal`. Tip listesi ARCH_SEGMENT … TIEBAR, USERDEFINED, NOTDEFINED.
- **IFC 4.3'ün tip tanımları** (buildingSMART, IFC4.3.x-development deposu, `IfcMemberTypeEnum.md`): POST "a linear
  (usually vertical) member used to support something or to mark a point"; PURLIN çatıda merteği taşıyan yatay eleman;
  RAFTER çatı örtüsünü taşıyan, çoğu eğimli eleman; MULLION giydirme cephede panelleri bağlayan eleman (§7). Bir cephenin
  ya da pergolanın kaydının listede karşılığı yok.
- **EK-5 Tablo 5.1:** eleman ELM. **EK-2 Tablo 2.3:** dikmenin ve kaydın kendi öğesi yok; "Diğer (Diğer ögeler için)"
  DIGR.
- **Üretici belgeleri** (yalnız iş akışı için): Archicad'in yerleşik IFC tip eşlemesi kirişi `IfcBeam`, kolonu
  `IfcColumn` yazar; `IfcMember` elemanın sınıflandırmasından ya da tip eşlemesinden gelir (Graphisoft yardımı, "Built-in
  IFC Type Mapping for ARCHICAD", AC 25, 121_IFC-50). Revit'te ailenin ya da örneğin `IfcExportAs` parametresi sınıfı ve
  ön tanımlı tipi birlikte söyler ("IfcMember.POST" gibi; OSArch wiki, "Revit setup for OpenBIM"; Autodesk yardımı,
  "Specifying IFC Entities for Families", 2014). İkisinde de dikme ve kayıt kolon ve kiriş aracıyla çizilir, sınıfı
  sonradan söylenir.

**Karar.**

22. **Eleman bir roldür (K46):** dikme, eleman olan kolondur (`ColumnFields.member`), kayıt eleman olan kiriştir
    (`BeamFields.member`); ayrı nesne türü kurulmaz: kolonun ve kirişin gövdesi, planı, aksları, katı ve Öznitelikler'i
    zaten var, üreticiler de sınıfı sonradan söyletir. `Member { predefined, name }`: tip IFC 4.3'ün listesinden
    (`Member::TYPES`, 20; NOTDEFINED tip değildir); USERDEFINED'ın ne olduğu adında yazılır (nesne tipi), adsız
    reddedilir; ad boş, kenarı boşluklu, denetim karakterli ya da 120 karakterden uzun olamaz. Yalnız Kiriş türündeki
    kiriş kayıt olur; lento ve hatıl olamaz.
23. **Kolonun kendi yüksekliği** (`ColumnFields.height`): dikmenin (pergola, saçak) yüksekliği; yoksa katın yüksekliği,
    çerçevenin kolonu gibi. Her kolon alabilir, en çok 100 m. Kesit modelinde gövde döşemesinin taşıyıcısının üstünden
    katın tabanı + kendi yüksekliğine çıkar (oraya yakın bir döşeme varsa onun taşıyıcısında durur, kolonun kuralı).
24. **Malzeme ve katman:** malzemesiz eleman, kolon ve kiriş gibi Betonarme'dir (veri anlamı değişmez); araçlar ilk
    malzemeyi "Çelik" yazar (KentOS'un kendi ilk değeri, M ile değişir). Kesitte EK-2'nin Diğer öğesinde, malzemesiyle
    (MM-DIGR-CELIK; Betonarme beton görünüşüyle); planda katman anahtarıyla MM-DIGR-DIKME ve MM-DIGR-KAYIT.
25. **IFC:** `IfcMember`, tipi ve adı EK-5'e göre (ELM ve adı, ad yoksa tipin Türkçe sözcüğü: "MM-ELM-Dikme",
    "MM-ELM-Aşık"), USERDEFINED'ın nesne tipi adı, malzemesi, etiketi kimliği. Katın duvar, kolon ve elemanlarının dış
    hattına değen eleman dışarıdadır (`Pset_MemberCommon.IsExternal`); duvarların arasında kalan değildir.
    `LoadBearing` yazılmaz (çizim söylemiyor; EK-6 da istemiyor); çerçevenin kolonu `IfcColumn` ve LoadBearing olarak
    kalır. FireRating beyandan (ADR 8068, `Pset_MemberCommon`).
26. **Metraj:** kayıt "Kirişler" metraj tablosuna girmez (çerçevenin kirişi değil); elemanların kendi çizelgesi sonraki
    adımdır.
27. **Sürüm:** `.kcad`'de eleman ve kolonun kendi yüksekliği mimari veri sürüm 4'tür (`ARCH_WITH_MEMBERS`; 1–3'ün
    verisini de içerir); daha küçük sürüm diyen dosyadaki eleman reddedilir; belge şeması değişmez.
28. **Komut, araç ve pencere:** `cad.columns.create` `height` ve `member`, `cad.beams.create` `member` alır;
    `cad.arch.edit` `member`'ı (kolon, kiriş; `null` çerçevenin kolonuna ya da kirişine döndürür), kolonun `height`'ını
    ve `heightFromLevel: true`'yu (kolonu katın yüksekliğine döndürür; `height` ile birlikte reddedilir) alır. Araçlar:
    **Dikme** (`tool.post`, yazılı adı DK): Kolon aracının kipi; 10/10, içi doldurulmaz, Çelik; Kesit boyu (O), Yükseklik (Y; boş ya da
    Kat: katın), Malzeme (M); Aks kesişimleri yok. **Kayıt** (`tool.rail`, KY): Kiriş aracının kipi; 5/10, 1,00 m
    kattan yükseklikte, Çelik; Tür yok. Şeritte Mimari › Yerleştir'de Kolon ▾ (Kolon, Dikme) ve Kiriş ▾ (Kiriş, Kayıt);
    ikonları seçenek sayfasından Dikme A (başlığı ve tabanıyla dikme) ve Kayıt B (iki dayanağı üstünde kayıt).
    Öznitelikler'de Tür (Kolon, Dikme; Kiriş, Lento, Hatıl, Kayıt), Eleman tipi ("Dikme (POST)" listesi), Eleman adı,
    kolonun Yükseklik'i (boş: katın); her biri bir geri alma. Tiplerin Türkçe sözcükleri (terim sözlüğü,
    `Member::word`): POST Dikme, PURLIN Aşık, RAFTER Mertek, COLLAR Kuşak, BRACE Payanda, STRUT Basınç çubuğu, STUD Ara
    dikme, PLATE Başlık kaydı, STRINGER Limon, CHORD Başlık çubuğu, MEMBER Çubuk, MULLION Cephe dikmesi; köprü
    elemanları (askı, kablolar, gergi çubuğu, berkitme, kemer parçası) da listede, okunur.

**Sınama.**

- **Sözleşme** (`member::tests`): dikme ve kayıt geçerli; "Dikme" adı, kaydın nesne tipi "Kayıt"; NOTDEFINED, adsız
  USERDEFINED ve kenarı boşluklu ad reddedilir.
- **Uygulama** (`arch_ifc::a_post_and_a_rail_are_ifc_members_of_their_types`): bölüm binasının doğusunda 10/10 Çelik
  dikme (kendi yüksekliği 2,40, katınki 3,00 değil) ve y = 0'da x 20–26 arasında, alt yüzü 1,00 m'de 5/10 Çelik kayıt.
  Elle: dikme 0–2,40, kayıt 1,00–1,10, ikisi de MM-DIGR-CELIK. IFC'de ikisi de `IfcMember` (POST; USERDEFINED "Kayıt"),
  Çelik, dışarıda, `Pset_ColumnCommon` yok; kayıp listesi boş, çerçevenin kolon ve kiriş sayısı değişmedi; dosyada 2
  IFCMEMBER.
- **Bağımsız okuyucu:** bu sınamanın yazdığı dosya (`KENTOS_WRITE_IFC_MEMBERS`) IfcOpenShell 0.9.0.post1'in
  doğrulayıcısından 0 bulguyla geçti; iki eleman da zemin katta, IsExternal doğru, malzemesi Çelik, gövdeleri kuruluyor.
- **`.kcad`** (`arch_elements::a_post_and_a_rail_round_trip_under_arch_data_4`; kodlayıcının birim sınaması): dikme ve
  kayıt `modules.arch = 4` ile gidip gelir; yalnız kendi yüksekliği de sürüm 4 ister; NOTDEFINED ve adsız USERDEFINED
  yazılırken reddedilir; sürüm 3 diyen dosyadaki dikme okunurken reddedilir.
- **Etkileşim** (`member_tools`, 2): Dikme'nin seçenekleri Yükseklik ve Malzeme, Aks kesişimleri yok; ilk dikme
  katın yüksekliğinde, 10/10 Çelik, dolgusuz, MM-DIGR-DIKME'de, "1 dikme konuldu."; 2,4 ve Ahşap yazılır, 0 reddedilir;
  Kat katın yüksekliğine döndürür. Kayıt'ın seçenekleri Kattan yükseklik ve Malzeme, Tür ve Kot farkı yok; iki tıklama
  ve Enter 5/10, 0,90 m'de, Çelik, MM-DIGR-KAYIT'ta kayıt yazar, "Kayıt çizildi: 5/10.". Katman türleri listesine
  MM-DIGR-DIKME ve MM-DIGR-KAYIT eklendi.
- **Masaüstü** (`properties::tests::arch::a_column_becomes_a_post_and_a_beam_a_rail`): kolonun Tür'ü Dikme yapınca
  Eleman tipi "Dikme (POST)" ve Eleman adı satırları gelir; Yükseklik 2,4 yazılır, boşaltınca katınki; Eleman adı tipi
  korur; kirişin Tür'ü Kayıt, sonra çerçevenin kirişi. Kesişen süzgeçler (şerit, modüller, geçmiş metinleri, katalog,
  ayarlar, Öznitelikler, 3B) 239 sınamayla geçti.
- **Ortak durumlar:** 31 yeni durum, başvuruları `scripts/fixtures/arch_command_cases.py`'nin ADR'den yazılmış
  kurallarından: `cad.columns.create`'e 15 (POST, adıyla USERDEFINED, STUD, kendi yüksekliği, katlı ve akslara bağlı
  dikme; NOTDEFINED, küçük harfli tip, adsız USERDEFINED, boş, kenarı boşluklu ve 121 karakterlik ad, sıfır, 101 m ve
  sonsuz yükseklik reddi), `cad.beams.create`'e 7 (kayıt, aşık, asılı kayıt; lento ve hatıl kayıt olamaz, listede olmayan
  tip, adsız USERDEFINED), `cad.arch.edit`'e 9 (kolon dikme olur ve geri döner, tipi ve adı, kendi yüksekliği ve
  katınkine dönüşü, dikmenin yüksekliği, ikisi birlikte reddi, adsız USERDEFINED reddi, kiriş kayıt olur ve geri döner,
  kayıt lentoya çevrilemez, merdivene eleman verilmez). Üç komutun 263 durumu geçti; üreticiye bağlı sekiz üretici
  başvurularıyla aynı.
- **Resimler:** `.run/gui-evidence/dikme-kayit-8070/` (görünüş binasının doğusunda pergola: dört dikme 2,60 m, üstünde
  iki kayıt ve dört ahşap mertek; plan, doğu görünüşü, kuzeydoğudan 3B; açık ve koyu) ve
  `.run/gui-evidence/ikon-dikme-kayit/secenekler.png`.

## §9. Çizelgede, dışa aktarmanın sayıları ve ilk kabul

**Araştırma.**

- **Planın ilk kabulü** (bim-plan BIM-09): "rampalı, giydirme cepheli, asansörlü örnek binada dışa aktarmanın kayıp
  listesi boş. Her yeni eleman planda, kesitte, 3B'de ve çizelgede"; giydirme cephe "doğrama listesinde".
- **Doğrama listesi:** Türkiye'de planda doğrama kodu yazılır, doğrama listesi kapıları, pencereleri ve öbür doğramaları
  kodlarıyla bir arada sayar (ADR 8028'in araştırması); KentOS'un doğrama listesi Kapı ve pencere çizelgesidir (ADR
  0344). Yönetmeliğin eklerinde doğrama listesi yalnız EK-2'nin Doğrama (DOGR) öğesi olarak geçer.

**Karar.**

29. **Giydirme cephe doğrama listesinde:** Kapı ve pencere çizelgesi giydirme cepheleri de satır olarak alır, çizimin
    sırasıyla açıklıkların arasında: Biçim "Giydirme cephe", Tür stilinin adı (stilsiz: "Giydirme cephe"), Genişlik boyu;
    Yükseklik, Alan, Kat, Tadilat, Adet ve öznitelikler duvarınki; açıklığın kendi alanları (No, Denizlik, Lento, Duvar,
    kod, ölçüler) boş (`Document::curtain_value`). Satırı salt okunurdur: hücrelerinden hiçbiri açıklığın alanı değil
    (`cell_computed`). Biçim süzgeci verilince alınmaz (biçimlerden biri değil). Duvar çizelgesinde stilsiz giydirme
    cephenin türü de "Giydirme cephe".
30. **Dikme Kolon çizelgesinde,** türü kendi türü yoksa elemanın adı ("Dikme"). Kayıt bir çizelgede değildir: KentOS'un
    kiriş çizelgesi yok, metrajın Kirişler tablosu da kaydı saymaz (karar 26). Rampa metrajın Rampalar tablosunda,
    kitaplığın asansörü, vitrifiyesi ve mobilyası Donatı çizelgesinde; baca ve aks bir çizelgenin nesnesi değildir.
31. **Dışa aktarmanın sayıları:** `IfcBuildingExported` rampa, baca (duvarın bacaları ve üstündeki çatı bacası bir),
    giydirme cephe, eleman (dikme, kayıt, giydirme cephenin dikme ve kayıtları), donatı ve aks sayılarını da verir;
    duvar, kolon ve kiriş sayıları giydirme cepheyi, dikmeyi ve kaydı saymaz (kendi sınıflarıdır). Masaüstünün iletisi
    sıfır olmayanları ekler. Komutun `export_losses` anlatımındaki "rampa henüz yazılmıyor" düştü; tek yönlü akslar
    eklendi.

**Sınama.**

- **İlk kabul** (`arch_ifc::the_sample_building_with_a_ramp_a_curtain_wall_and_a_lift_leaves_nothing_out`): bölüm
  binasına L rampa, 6 m'lik giydirme cephe, kitaplığın Asansör'ü (8 kişi, 630 kg), bir dikme ve bir kayıt. Her birinin
  kesit modelinde gövdesi var (kesit, görünüş ve 3B onu çizer); IFC'de IfcRamp, IfcCurtainWall, IfcTransportElement ve
  iki IfcMember; kayıp listesi boş. Giydirme cephe Kapı ve pencere çizelgesinde, asansör Donatı çizelgesinde, dikme Kolon
  çizelgesinde.
- **Ortak durumlar** (başvuru `scripts/fixtures/tag_command_cases.py`, KentOS kodu olmadan): `cad.schedules.create`'e 6
  (doğrama listesinde giydirme cephe: biçimi ve türü Giydirme cephe, genişliği 6,000, yüksekliği 3,000; türe göre grupta
  18,00 m² ve adedi; biçim süzgeci almaz; kat süzgecinde kendi katı; Duvar çizelgesinde türü; Kolon çizelgesinde dikme),
  `cad.schedules.edit`'e 2 (giydirme cephe satırının genişliği ve numarası yazılmaz). `cad.arch.ifc`'ye 1 (giydirme cephe,
  dikme ve kayıt: 1 giydirme cephe, 23 eleman; duvar, kolon ve kiriş sayıları değişmez) ve bütün durumlarda yeni sayılar
  (bölüm binasında 7 aks, tek yönlü takımda 0); başvurunun kuralları (`arch_ifc_cases.py`: giydirme cephenin çerçeve
  sayısı, aksların iki yön kuralı) KentOS kodu olmadan. Etiket ve çizelge komutlarının bütün durumları ve `cad.arch.ifc`'nin
  24 durumu geçti.
- **Masaüstü:** kesişen süzgeçler ve IFC, çizelge sınamaları 246 sınamayla geçti.

## §10. Kaplama ve asma tavan (K47)

**Araştırma.**

- **EK-6 Tablo 6.20 (kaplama, `IfcCovering`, PDF s. 59):** "başka bir elemanın bir kısmını örten ve tamamen o elemana
  bağlı olan bir eleman"; tip listesi CEILING, CLADDING, COPING, FLOORING, INSULATION, MEMBRANE, MOLDING, ROOFING,
  SKIRTING BOARD, SLEEVING, TOPPING, WRAPPING, USERDEFINED, NOTDEFINED. Zorunlular ön tanımlı tip, ad (EK-5), etiket,
  malzeme; kaplamanın malzemesi ve kalınlığı iki yoldan biriyle (mahalin seti ya da `IfcCovering`'in katmanı, §6).
- **IFC 4.3'ün tip tanımları** (IFC4.3.x-development deposu, `IfcCoveringTypeEnum.md`): CEILING tavanı, FLOORING döşeme
  kaplamasını, ROOFING çatı örtüsünü, TOPPING "a layer of material used for leveling or flattening a surface" temsil
  eder.
- **EK-5 Tablo 5.1:** kaplama KPL. **EK-2 Tablo 2.3:** Asma Tavan ASTV, Zemin Kaplama ZMNK, Duvar Kaplama DUVK, Çatı
  Kaplama CKAP, Isı Yalıtımı ISYA, Su Yalıtımı SUYA, Kartonpiyer KARP, Süpürgelik SUPR, Harpuşta HARP, Tesviye Betonu TSVY.
- **Başka uygulamalar:** asma tavan mahalden ayrı bir nesnedir (ADR 8040'ın araştırması).
- **Asma tavanın ilk kalınlığı:** alçı levha üreticilerinin veri sayfaları yaygın kalınlıkları 9,5, 12,5 ve 15 mm verir
  (ör. GYPFOR'un ürün broşürü); NHBC'nin iç duvar kılavuzu 600 mm aralık için 12,5 mm ve üstünü önerir, bir yapı
  malzemesi satıcısının notu tavanlarda 9,5–12,5 mm der. EN 520'nin kendi metni okunamadı (ücretli): KentOS'un ilk
  değeri olarak 12,5 mm, bir standardın istediği söylenmez.

**Karar.**

32. **Kaplama döşemenin bir roldür (K47):** `SlabFields.covering` (`Covering { predefined, name }`: IFC 4.3'ün 13 tipi,
    NOTDEFINED tip değil, USERDEFINED'ın ne olduğu adında); döşemenin sınırı, kalınlığı, kotu, malzemesi ve kaynağa bağlı
    sınırı kullanılır, ayrı nesne türü kurulmaz (dikme ve kayıt gibi, §8). **Mahalden türeyen nesne:** mahale bağlı
    kaplamanın sınırı mahalin halkalarıdır ve mahal değişince yeniden bulunur (ADR 0771 §4); elle de köşeleriyle
    çizilir. Kaplama katmansız, kenarsız ve türü Döşeme'dir (`SlabFields::problem`).
33. **Yapı değildir:** duvarın yüksekliği, kolonun ve kirişin gövdesi, mahalin tavanı ve döşemesi onu görmez (yapı dizini
    `structure_index`, kesit modelinin döşeme listesi); metrajın döşeme satırları ve korkuluk denetimi onu saymaz.
    Mahalin kendi asma tavanı (`suspendedCeiling`, ADR 8040 §3) yüksekliği söyler; kaplama onun gövdesidir.
34. **Kesit, plan ve 3B:** kesitte tipinin EK-2 öğesinde, malzemesiyle (MM-ASTV-ALCIPAN; malzemesiz GENEL), ince ve
    taramasız bir plaka; planda katman anahtarıyla tipinin öğesi (MM-ASTV-GENEL, MM-ZMNK-GENEL); 3B'de döşemenin
    renginde.
35. **IFC:** `IfcCovering`, tipi, adı EK-5'e göre (KPL ve adı, yoksa tipin Türkçe sözcüğü: MM-KPL-Asma_tavan,
    MM-KPL-Zemin_kaplaması), USERDEFINED'ın nesne tipi adı, malzemesi, etiketi kimliği; `Pset_SlabCommon` yazılmaz,
    FireRating beyandan (`Pset_CoveringCommon`). Katman takımı (`IfcMaterialLayerSet`, LayerThickness) yazılmaz, kalınlık
    gövdededir (katmanlı malzeme IFC-04'ün işi); mahale bağ (`IfcRelCoversSpaces`) sonraki adım. Dışa aktarmanın
    sayılarında `coverings`; `slabs` kaplamayı saymaz.
36. **Sürüm:** `.kcad`'de kaplama mimari veri sürüm 5'tir (`ARCH_WITH_COVERINGS`; 1–4'ün verisini de içerir).
37. **Komut, araç ve pencere:** `cad.slabs.create` `covering`'i, `cad.slabs.edit` nullable `covering`'i alır. Araç
    **Kaplama** (`tool.covering`, yazılı adı KM; şeritte Mimari › Mahal'de Döşeme ▾ (Döşeme, Kaplama); ikonu seçenek
    sayfasından A, katmanlı plakalar): Sınır (S: Mahal tıklanan mahale bağlar, Köşeleri çiz), Tip (P: Asma tavan ile
    başlar), Kalınlık (E: asma tavan 12,5 mm, öbürleri 2 cm, ADR 8040'ın döşeme dizilişindeki kaplama), Kot farkı (F; Oto:
    tavan mahalin tavanında, öbürleri tabanında, üstü kalınlığı kadar yukarıda), Malzeme (M). Öznitelikler'de Tür
    (Döşeme, Kaplama: kaplamaya çevirince katmanlar ve kenar kalkar, türü Döşeme olur), Kaplama tipi, Kaplama adı;
    kaplamada Döşeme türü satırı yok. Tiplerin Türkçe sözcükleri (`Covering::word`): CEILING Tavan kaplaması (araç ve
    pencere "Asma tavan" der), FLOORING Zemin kaplaması, CLADDING Duvar kaplaması, ROOFING Çatı kaplaması, INSULATION Isı
    yalıtımı, MEMBRANE Su yalıtımı, MOLDING Kartonpiyer, SKIRTINGBOARD Süpürgelik, COPING Harpuşta, TOPPING Tesviye
    betonu, SLEEVING Kılıf, WRAPPING Sargı.

**Sınama.**

- **Sözleşme** (`covering::tests`): asma tavan geçerli, adı "Asma tavan"; FLOORING'in sözcüğü; CEILING ASTV'de, SLEEVING
  Diğer'de; NOTDEFINED ve adsız USERDEFINED reddedilir.
- **Uygulama** (`arch_ifc::a_ceiling_from_a_room_and_a_floor_covering_are_ifc_coverings`): bölüm binasının zemin kattaki
  bir mahaline bağlı 12,5 mm Alçıpan asma tavan (üstü 2,60) ve elle çizilmiş zemin kaplaması. Elle: plaka 2,5875–2,60,
  MM-ASTV-ALCIPAN, ince. Duvarların yükseklikleri ve mahalin tavanı değişmez. IFC'de IfcCovering CEILING
  (MM-KPL-Asma_tavan, Alçıpan) ve FLOORING (MM-KPL-Zemin_kaplaması); IfcSlab sayısı aynı; kayıp listesi boş.
- **Bağımsız okuyucu:** yazılan dosya (`KENTOS_WRITE_IFC_COVERINGS`) IfcOpenShell 0.9.0.post1'in doğrulayıcısından 0
  bulguyla geçti; iki kaplama zemin katta, gövdeleri kuruluyor.
- **`.kcad`** (`arch_elements::a_covering_round_trips_under_arch_data_5`): kaplama `modules.arch = 5` ile gidip gelir;
  katmanlı kaplama yazılırken reddedilir.
- **Etkileşim** (`covering_tool::the_covering_tool_draws_a_ceiling_and_a_floor_covering`): seçenekler Sınır, Tip,
  Kalınlık, Kot farkı, Malzeme; köşelerle 12,5 mm Alçıpan asma tavan (Kot farkı 2,6) MM-ASTV-GENEL'de; Tip FLOORING'de
  2 cm, Oto kotla tabanında (0,02), MM-ZMNK-GENEL'de; sıfır kalınlık reddedilir.
- **Masaüstü** (`properties::tests::arch::a_slab_becomes_a_covering_and_back`): Tür Kaplama asma tavan yapar, Kaplama
  tipi "Asma tavan (CEILING)" ve Kaplama adı gelir, Döşeme türü gider; ad yazılır, tip kalır; Tür Döşeme geri döndürür.
  Kesişen süzgeçler 251 sınamayla geçti.
- **Ortak durumlar** (başvuru `scripts/fixtures/slab_cases.py`, metrajın kuralı `space_command_cases.py`, KentOS kodu
  olmadan): `cad.slabs.create`'e 14 (asma tavan, zemin kaplaması, adıyla kullanıcı tanımlı, katlı çatı kaplaması, metraj
  dışı ısı yalıtımı, her biri metrajda döşeme sayılmadan; NOTDEFINED, küçük harfli tip, adsız USERDEFINED, boş ve 121
  karakterlik ad, katmanlı, korkuluklu, balkon ve teras kaplama reddi), `cad.slabs.edit`'e 8 (döşeme asma tavan olur ve
  metrajdan çıkar, kaplama döşemeye döner ve metraja girer, tipi ve adı, kalınlığı ve kotu; katmanlı döşeme kaplama
  olamaz, kaplamaya katman ve korkuluk verilmez, adsız USERDEFINED). "Değiştirilecek alan yok" iletisi kaplamayı da sayar.
  Döşeme, bağlı döşeme, metraj ve dışa aktarma komutlarının bütün durumları geçti.
- **Resimler:** `.run/gui-evidence/kaplama-8070/` (görünüş binasının zemin katındaki her mahalde asma tavan: A-A kesiti,
  yalnız zemin kat 3B güneybatıdan ve üstten; açık ve koyu) ve `.run/gui-evidence/ikon-kaplama/secenekler.png`.

## §11. Kiriş ve kaplama çizelgesi

**Araştırma.** Planın ilk kabulü her yeni elemanın bir çizelgede olmasını ister (§9). §9'dan sonra kayıt (bir kiriş,
§8) ve kaplama (bir döşeme, §10) hiçbir çizelgede değildi: KentOS'un açıklık, mahal, kolon, duvar, donatı ve bölüm
çizelgesi vardı, kiriş ve döşeme çizelgesi yoktu.

**Karar.**

38. **Kiriş çizelgesi** (`ScheduleOf::Beams`): kirişler, lentolar, hatıllar ve kayıtlar; sütunları Etiket, Tür (kaydın
    adı, yoksa türü: Kiriş, Lento, Hatıl), Genişlik, Yükseklik, Uzunluk, Kat; Hacim de okunur. Numarası çizelgeden
    yazılır, öbürleri hesaplanır (türü kirişin rolü ve elemanıdır, Öznitelikler'den değişir). Gruplu: tür ve ölçüye
    göre adet ve boy.
39. **Kaplama çizelgesi** (`ScheduleOf::Coverings`): yalnız kaplama olan döşemeler, binanın döşemeleri değil; sütunları
    Tür (kaplamanın adı), Kalınlık, Alan, Kat; Hacim de okunur. Hücreleri hesaplanır (öznitelikler dışında). Gruplu:
    tür ve kalınlığa göre adet ve alan. Döşeme çizelgesi yok: döşemeler metrajın Döşemeler tablosundadır.
40. **Sürüm ve araç:** böyle bir çizelge `.kcad`'de mimari veri sürüm 6'dır (`ARCH_WITH_ELEMENT_SCHEDULES`); Çizelge
    aracının Tür (T) seçimi Kiriş çizelgesi ve Kaplama çizelgesini de verir; tablonun başlığındaki ikonları kirişin ve
    kaplamanınki.

**Sınama.**

- **Uygulama** (`arch_ifc::beam_and_covering_schedules_list_their_own_and_round_trip_under_arch_data_6`): bölüm
  binasında kayıt kiriş çizelgesinde, binanın kirişleriyle; kaplama çizelgesinde yalnız kaplama; çizelgeli çizim sürüm
  6 ile gidip gelir. **İlk kabul** sınaması artık asma tavanı da kurar: kayıt Kiriş, asma tavan Kaplama çizelgesinde.
- **Ortak durumlar** (başvuru `tag_command_cases.py`): `cad.schedules.create`'e 8 (kiriş, lento ve kayıt; türe göre grup
  ve boy toplamı; hacim; yalnız kaplamalar; türe göre grup ve alan toplamı; hacim ve kat süzgeci; kiriş çizelgesinde
  Konum ve kaplama çizelgesinde Genişlik okunmaz), `cad.schedules.edit`'e 2 (kirişin numarası yazılır, türü yazılmaz).
- **Masaüstü:** kesişen süzgeçler, tablo ve çizelge sınamaları 261 sınamayla geçti.
- **Resim:** `.run/gui-evidence/eleman-cizelgeleri-8070/` (pergolalı binada iki çizelge; açık ve koyu).

## Ortak sınamalar

- **Bağımsız okuyucu** (`scripts/fixtures/arch_ifc_check.py`, IfcOpenShell 0.9.0.post1, KentOS kodu olmadan): yeniden
  yazılan `fixtures/arch/v1/ifc/kesit-binasi.ifc` ve künyeli eşi IFC4X3_ADD2, doğrulayıcı (öznitelikler, EXPRESS
  kuralları) 0 bulgu, 68 gövde. Betiğe eklenen denetimler: ızgara binanın, eksenleri tanımın aksları adları ve iki
  noktasıyla, RECTANGULAR; U merdiven HALF_TURN_STAIR ve Betonarme. Bu dosyalar ADR 8047'den beri yeniden yazılmamıştı:
  B7 ve B8'in alanları (yangın sınıfları, emsal zonları, bölüm grupları) da artık içlerinde ve doğrulayıcıdan geçiyor.
- **Ortak durumlar:** `cad.arch.ifc` durumları: rampa yazılır ve bir şey söylenmez; tek yönde akslar ızgara olmaz ve
  söylenir; "bina yok" iletisi rampayı da sayar. §1–§6'da dokunulan komutların 365 durumu geçti.

## Sıradaki adımlar

- Eğimli eleman (mertek, payanda): kirişin ekseni düzlemde; eğik gövde sonraki adım. Elemanların kendi çizelgesi ve
  metrajı; aks kesişimlerine dikme.
- Giydirme cephenin panel yerine kapısı ve penceresi; doğrama kodu (GC1); köşede iki giydirme cephenin ortak dikmesi;
  panelin kendi malzeme katmanları.
- Duvar kaplamasının kalınlığı; kaplamanın mahale bağı (`IfcRelCoversSpaces`) ve katman takımı; mahale bağlı asma
  tavanın kotunun mahalin asma tavan yüksekliğini izlemesi.
- Rampanın korkulukları (merdivenin de) kesit modelinde yok.
- Kitaplık bloklarının gövdesi kutudur; gerçek biçim (lavabo çanağı, asılı klozetin altı) ve EN 81-20'nin kabin
  yüksekliği kaynağı gelince.
- Baca ve aksın çizelgesi gerekirse.
