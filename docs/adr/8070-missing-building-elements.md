# ADR 8070: Eksik yapı elemanları — rampa gövdesi, merdiven ve rampa tipleri, baca ve niş, aks ızgarası, kitaplık blokları

- **Durum:** kısmen uygulandı (16 Kasım 2026). Plan: [bim-plan.md](../../plan/bim-plan.md) BIM-09,
  [ifc-uyum.md](../../plan/ifc-uyum.md) IFC-03; uygulama sırasında B9. Bu ADR maddenin bütün adımlarını tutar; her adım
  kendi bölümüyle eklenir.
- **Bağlam:** ADR [8047](8047-ifc-building-export.md) (binanın IFC'si, kayıp listesi), [8042](8042-section-derived-from-a-section-mark.md)
  (kesit modeli), [0705](0705-ramps-by-slope-and-railings.md) ve [0723](0723-ramp-landings-levels-accessibility.md) (rampa),
  [8044](8044-roof-and-roof-plan.md) §6 (çatı bacası), [8045](8045-balanced-stairs-and-special-elements.md) §5 (asansör
  bloğu), [8068](8068-fire-performance-material-declarations.md) (zorunlu alanlar).
- **Dayanak:** Dijital Proje Yönetmeliği (RG 5.8.2026/33331) EK-5 Tablo 5.1; EK-6 §6.1.3 ve Tablo 6.14 (baca), 6.20
  (kaplama), 6.21 (giydirme cephe), 6.35 (mobilya), 6.36 (aks), 6.40 (eleman), 6.49 (rampa), 6.53 (sıhhi terminal), 6.61
  (merdiven), 6.64 (taşıma elemanı); metinler `mevzuat/bim/ek-6-1-zorunlu-a-f.md` ve `ek-6-2-zorunlu-f-z.md`.
- **Web:** dokunulmadı (ADR 0228).

## §1. Araştırma

- **EK-6 Tablo 6.49 (rampa, PDF s. 89):** ön tanımlı tip, ad, etiket, malzeme ("Malzeme İsmi_Dayanım"); tip listesi
  STRAIGHT RUN RAMP, TWO STRAIGHT RUN RAMP, QUARTER TURN RAMP, TWO QUARTER TURN RAMP, HALF TURN RAMP, SPIRAL RAMP,
  USERDEFINED, NOTDEFINED. Tanım: "farklı kotlardaki iki döşeme seviyesini birbirine bağlayan … ara bir döşeme plağı
  olarak bir sahanlık içerebilir."
- **EK-6 Tablo 6.61 (merdiven, PDF s. 102):** aynı dört öznitelik, `Pset_StairCommon` FireRating ve FireExit; tip
  listesi STRAIGHT RUN … TWO CURVED RUN STAIR.
- **IFC 4.3'ün kendi tanımları** (buildingSMART, IFC4.3.x-development deposu, `IfcRampTypeEnum.md`,
  `IfcStairTypeEnum.md`; resmî belge sitesi bu bilgisayardan 403 veriyor): STRAIGHT_RUN_RAMP tek düz kol; TWO_STRAIGHT_RUN
  iki düz kol ve bir sahanlık, dönüşsüz; QUARTER_TURN 90° dönen iki kol ve çeyrek sahanlık; TWO_QUARTER_TURN 180° dönen
  üç kol ve iki çeyrek sahanlık; HALF_TURN 180° dönen iki kol ve yarım sahanlık; SPIRAL çevresinde dönülen boşluk,
  sahanlıksız. Merdivende WINDING türleri **tek kol** sayılır ("one flight with quarter winders"): dengelenmiş basamaklar
  kolun basamaklarıdır. SPIRAL_STAIR bir dikme çevresinde, CURVED_RUN_STAIR tek eğri kol.
- **EK-5 Tablo 5.1:** rampa RMP, merdiven MRD; `IfcRampFlight` ve `IfcStairFlight` listede yok: rampa ve merdiven tek
  nesne, gövdesi kolları ve sahanlıklarıyla.

## §2. Karar

1. **Rampanın gövdesi** (kesit modeli, `sections::ramp`): her eğimli kol bir eğik plak (çatı yüzünün kesitte
   kullandığı `Sloped`), düzlemi kolun başındaki kottan sonundakine çıkar; yükseklik kollara eğimli uzunluklarıyla
   paylaştırılır (planın oku ve kotları gibi). Her sahanlık kendi kotunda düz bir plak. Kalınlık merdivenin plağı
   gibi: vardığı döşemenin taşıyıcısı, yoksa çıktığı döşemeninki, o da yoksa döşemenin kendi kalınlığı (0,15 m). Zemine
   oturan rampanın altındaki dolgu zemindir, model zemini tutmaz (EK-4 4.30); bu yüzden "yere kadar dolu" çizilmez.
   Kesit, görünüş ve 3B aynı gövdeyi okur; kesit artık "rampa çizilmedi" demez, IFC'nin kayıp listesinde rampa yoktur.
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
8. **Aks ızgarası (`IfcGrid`, `IfcGridAxis`; EK-6 Tablo 6.36):** çizimin aksları katlarına göre (katı olmayan, her katta
   görünen aks binanındır) bir ızgara olur. İlk düz aksa paralel olanlar U, ötekiler V ekseni; her takım kendi içinde
   paralel ve ikisi birbirine dik ise RECTANGULAR, değilse IRREGULAR (RADIAL ayrıca denetlenmedikçe yazılmaz). Her eksen
   etiketi (`AxisTag`: A, 1) ve çizgisiyle; yay aks `IfcIndexedPolyCurve`'ün yay parçasıyla (başı, ortası, sonu), tam.
   Izgaranın planda izi (`FootPrint`, `GeometricCurveSet`) yalnız ızgara varken açılan kendi bağlamında. IFC 4.3
   (IFC4X3_ADD2) U ve V'nin ikisini de zorunlu tuttuğundan bir katın hep aynı yöndeki aksları ızgara olamaz: kayıp
   listesine "aks" olarak, nedeniyle girer (`Why::OneWay`). Etiketsiz aks zorunlu alan eksiği olarak denetlenir.
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
14. **Kaplama (K44; EK-6 Tablo 6.20 dipnotu):** kaplamanın malzemesi ve kalınlığı "(1) IfcSpace içindeki tanımlardan
    faydalanılarak ya da (2) IfcCovering'de … doğrudan modelleme ile" verilir, "bu iki yöntemden biri zorunludur".
    KentOS birinci yolu tutar: mahalin `Pset_SpaceCoveringRequirements`'i kaplama adlarına (oda listesinin, ADR 8038)
    kalınlıklarını da ekler: döşeme kaplamasınınki altındaki döşemenin en üst katmanı kaplama ise onun kalınlığı,
    tavanınki üstündeki döşemenin altındaki sıva (`Document::room_coverings`, ADR 8040 §2). Asma tavanlı mahalin tavan
    kalınlığı yazılmaz (asma tavanın katmanları çizilmiyor); duvar kaplamasının kalınlığı henüz yok (mahale bakan yüzün
    katmanı). Döşemenin kaplaması ve tavanın sıvası zaten `IfcSlab`'ın gövdesinde olduğundan ayrıca `IfcCovering`
    yazılmaz (gövdeyi ikilerdi). Elle çizilen kaplama ve asma tavanın kendi gövdesi sonraki adımdır.

## §3. Sınamalar

- **`kentos-ifc`** (`turns`, 4): düz çizgi, L'nin çeyrek dönüşü, sahanlıkları değen U'nun yarım dönüşü, ortası eğimli
  U'nun iki çeyreği, düz çizgide sahanlık, 45°'lik dönüşün reddi, üç düz kol (USERDEFINED, "3 kollu, 2 sahanlıklı
  rampa"), sola ve sağa dönen balansmanlar, uçlara uzanan balansman (tek kol), sahanlıkla balansman birlikte (yok).
- **Uygulama** (`sections::a_ramp_s_runs_slope_its_landing_stands_flat_and_the_model_writes_it`): bölüm binasının
  doğusunda 6 m doğuya, 4 m kuzeye giden, 1,50 m genişliğinde, 0,75 m yükselen L rampa. Beklenenler kesirlerle elle:
  eğimli uzunluk 8,5 m, eğim 3/34, sahanlık 63/136 m; kollar 0,15·√(1 + (3/34)²) dik kalınlıkta; y = 2'den kuzeye bakan
  kesit ikinci kolu x = 35,25…36,75 arasında, üstü 39/68 m'de keser. IFC'de tek `IfcRamp`, QUARTER_TURN_RAMP,
  Betonarme, üç gövde, kayıp listesinde yok.
- **Uygulama** (`roof_sections::a_flue_in_its_walls_and_the_chimney_over_it_are_one_ifc_chimney_and_a_niche_a_recess`):
  bölüm binasının çatısında (4,5; 8,0)'de 40 × 40 cm duman bacası, altındaki batı hol duvarında (İç duvar 15) iki katta
  7 cm'lik ortada baca, doğu hol duvarında 0,90–1,50 m arasında 5 cm'lik niş. Kesit modelinde baca duvarın her
  katmanından 4,465 ≤ x ≤ 4,535, 7,9 ≤ y ≤ 8,1 kutusu olarak çıkar (döşemeden). IFC'de tek `IfcChimney` (USERDEFINED
  "Duman bacası", MM-BAC-…, GlobalId zemin katın bacası), üç gövde (0,00; +3,00; +6,00'dan), iki OPENING onu dolduruyor,
  niş RECESS ve dolduransız; hiçbiri kayıp listesinde değil; dosyada IFCCHIMNEY ve her dolan açıklık için bir
  IFCRELFILLSELEMENT.
- **Uygulama** (`arch_ifc::the_grids_are_one_rectangular_ifc_grid_and_one_direction_alone_is_left_out`): bölüm
  binasının katsız A–D ve 1–3 aksları binanın tek RECTANGULAR ızgarası (U: A–D, V: 1–3, 7 eksen, B (4,5; −2)–(4,5; 11));
  dosyada 7 IFCGRIDAXIS, FootPrint; hiçbir eksenin etiketi eksik değil. Yalnız A–D kalınca ızgara yok, 4 aks kayıpta.
- **Bağımsız okuyucu** (`scripts/fixtures/arch_ifc_check.py`, IfcOpenShell 0.9.0.post1, KentOS kodu olmadan): yeniden
  yazılan `fixtures/arch/v1/ifc/kesit-binasi.ifc` ve künyeli eşi IFC4X3_ADD2, doğrulayıcı (öznitelikler, EXPRESS
  kuralları) 0 bulgu, 68 gövde. Betiğe eklenen denetimler: ızgara binanın, eksenleri tanımın aksları adları ve iki
  noktasıyla, RECTANGULAR; U merdiven HALF_TURN_STAIR ve Betonarme. Bu dosyalar ADR 8047'den beri yeniden yazılmamıştı:
  B7 ve B8'in alanları (yangın sınıfları, emsal zonları, bölüm grupları) da artık içlerinde ve doğrulayıcıdan geçiyor.
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
  elemanı. `cad.blocks.edit`'te bilinmeyen kitaplık sürümü örneği 5'e geçti. `cad.arch.ifc` durumları: rampa yazılır ve
  bir şey söylenmez; tek yönde akslar ızgara olmaz ve söylenir; "bina yok" iletisi rampayı da sayar. Dokunulan komutların
  365 durumu geçti.
- **Masaüstü** (203 süzgeçli sınama; `block_card::the_building_element_is_chosen_typed_and_taken_away`): sınıf ve tip
  seçilir, sınıf değişince öbür sınıfın tipi düşer, sayı olmayan yükseklik komuttan önce reddedilir, "Yok" kaldırır, her
  biri bir geri alma.
- **Resimler:** `.run/shots/arac-blok-karti-yapi-elemani-*` (Blok kartı: Mobilya, TABLE, 0,75); 3B
  `.run/gui-evidence/eksik-elemanlar-8070/` (çatı, baca, rampa; zemin katta asansör, lavabo, masa; üstten). Çok katlı baca
  en alttaki parçasının katında olduğu için yalnız zemin kat gösterilince bütün olarak görünür.
- **Uygulama** (`arch_ifc::a_room_carries_its_floor_finish_and_ceiling_plaster_thicknesses`): bölüm binasının mahalleri
  döşeme kaplaması 0,02 (zemin ve 1. katın döşemesinin üst katmanı) ve tavan sıvası 0,02 taşır
  (`IFCPOSITIVELENGTHMEASURE`); 2,60 m'de asma tavanlı mahalin tavan kalınlığı yok. Denetim olgularının çiziminde
  döşeme katmanı olmadığından eksikleri değişmedi.
- **Dijital denetim:** rampa artık "modele girmeyen nesne" değil; sınama örneği yüksekliği olmayan bir duvara geçti.
- **Denetim olguları** (`fixtures/audit-facts/v1/mimari.json`): merdivenin ön tanımlı tip ve malzeme eksikleri düştü
  (yalnız bu iki satır elle çıkarıldı; kaçış yolunun önceden bilinen 11,55 / 4,45 farkı görünür bırakıldı).

## §4. Sıradaki adımlar

- Giydirme cephe (`IfcCurtainWall`), dikme ve kayıt (`IfcMember`).
- Elle çizilen kaplama, asma tavanın gövdesi (`IfcCovering`), duvar kaplamasının kalınlığı.
- Rampanın korkulukları (merdivenin de) kesit modelinde yok.
- Kitaplık bloklarının gövdesi kutudur; gerçek biçim (lavabo çanağı, asılı klozetin altı) ve EN 81-20'nin kabin
  yüksekliği kaynağı gelince.
- `IfcBuildingExported` sınıf sayıları (rampa, baca, aks, donatı) çıktıda yok; dosyada var.
