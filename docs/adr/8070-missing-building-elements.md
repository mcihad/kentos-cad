# ADR 8070: Eksik yapı elemanları — rampa, merdiven ve rampa tipleri, baca ve niş, aks ızgarası, kitaplık blokları, kaplama, giydirme cephe

- **Durum:** kısmen uygulandı (16 Kasım 2026). Plan: [bim-plan.md](../../plan/bim-plan.md) BIM-09,
  [ifc-uyum.md](../../plan/ifc-uyum.md) IFC-03; uygulama sırasında B9. Bu ADR maddenin bütün adımlarını tutar; her adım
  kendi bölümüdür ve koddaki "docs/adr/8070 §N" atıfları bu bölümleri gösterir: §1 rampanın gövdesi, §2 merdiven ve
  rampanın tipi ve malzemesi, §3 baca ve niş, §4 aks ızgarası, §5 kitaplık blokları, §6 kaplama, §7 giydirme cephe.
  Kararların numarası bölümler boyunca sürer (1–21). Birden çok adımın ortak sınamaları §8'de, sıradaki adımlar §9'da.
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

## §8. Ortak sınamalar

- **Bağımsız okuyucu** (`scripts/fixtures/arch_ifc_check.py`, IfcOpenShell 0.9.0.post1, KentOS kodu olmadan): yeniden
  yazılan `fixtures/arch/v1/ifc/kesit-binasi.ifc` ve künyeli eşi IFC4X3_ADD2, doğrulayıcı (öznitelikler, EXPRESS
  kuralları) 0 bulgu, 68 gövde. Betiğe eklenen denetimler: ızgara binanın, eksenleri tanımın aksları adları ve iki
  noktasıyla, RECTANGULAR; U merdiven HALF_TURN_STAIR ve Betonarme. Bu dosyalar ADR 8047'den beri yeniden yazılmamıştı:
  B7 ve B8'in alanları (yangın sınıfları, emsal zonları, bölüm grupları) da artık içlerinde ve doğrulayıcıdan geçiyor.
- **Ortak durumlar:** `cad.arch.ifc` durumları: rampa yazılır ve bir şey söylenmez; tek yönde akslar ızgara olmaz ve
  söylenir; "bina yok" iletisi rampayı da sayar. §1–§6'da dokunulan komutların 365 durumu geçti.

## §9. Sıradaki adımlar

- Tek başına dikme ve kayıt (pergola dikmesi, cephe kayıtı) `IfcMember` olarak.
- Giydirme cephenin panel yerine kapısı ve penceresi; doğrama listesinde yeri; köşede iki giydirme cephenin ortak
  dikmesi; panelin kendi malzeme katmanları.
- Elle çizilen kaplama, asma tavanın gövdesi (`IfcCovering`), duvar kaplamasının kalınlığı.
- Rampanın korkulukları (merdivenin de) kesit modelinde yok.
- Kitaplık bloklarının gövdesi kutudur; gerçek biçim (lavabo çanağı, asılı klozetin altı) ve EN 81-20'nin kabin
  yüksekliği kaynağı gelince.
- `IfcBuildingExported` sınıf sayıları (rampa, baca, aks, donatı, giydirme cephe) çıktıda yok; dosyada var.
