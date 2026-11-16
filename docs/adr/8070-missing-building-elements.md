# ADR 8070: Eksik yapı elemanları — rampa gövdesi, merdiven ve rampa tipleri, baca, aks, kitaplık blokları

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
- **Dijital denetim:** rampa artık "modele girmeyen nesne" değil; sınama örneği yüksekliği olmayan bir duvara geçti.
- **Denetim olguları** (`fixtures/audit-facts/v1/mimari.json`): merdivenin ön tanımlı tip ve malzeme eksikleri düştü
  (yalnız bu iki satır elle çıkarıldı; kaçış yolunun önceden bilinen 11,55 / 4,45 farkı görünür bırakıldı).

## §4. Sıradaki adımlar

- Aks ızgarası (`IfcGrid`, `IfcGridAxis`).
- Kitaplık bloklarının yapı sınıfı ve gövdesi (asansör, vitrifiye, mobilya).
- Kaplama ve asma tavan (`IfcCovering`), giydirme cephe (`IfcCurtainWall`), dikme ve kayıt (`IfcMember`).
- Rampanın korkulukları (merdivenin de) kesit modelinde yok.
