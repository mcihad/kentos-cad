# ADR 8044: Çatı ve çatı planı — saçak çizgisinden çatı yüzleri, mahya, kırma, dere; çatı planı

- **Durum:** sürüyor (10 Kasım 2026); `plan/architecture-advanced.md` ADV-20, M44. 1. bölüm yapıldı: çekirdek
  geometri (§2), çatı nesnesi (§1), plan çizimi (§3) ve Çatı aracı (§4). 2. bölüm yapıldı: kesit ve görünüşte çatı
  (§5). 3. bölüm yapıldı: baca, hava bacası ve çatı penceresi (§6). 4. bölüm yapıldı: çatı planı paftası (§7). Sırada
  oluk ve iniş borusu, kar siperliği ve PAİY 40/42 denetimi.
- **Bağlam belgesi:** ADR 8040 (düşey model: kat yükseklikleri, döşemenin katmanları), ADR 8042–8043 (kesit ve görünüş:
  "çizilmeyen: çatı"), ADR 8008 (EK-4 katman anahtarı), ADR 8011 (ölçeğe göre ayrıntı).

## Bağlam

KentOS'ta çatı nesnesi yok: döşemenin "Teras" rolü düz çatıyı karşılıyor, eğimli çatı ise elle çiziliyor. Kesit ve
görünüş (M42–M43) çatıyı "çizilmeyen: çatı" diye söylüyor; ruhsat takımında çatı planı paftası yok. PAİY 57(2)(a) mimari
projede çatı planını ister.

## Araştırma

### Türkiye mevzuatı

- **PAİY Md. 40** (mevzuat.gov.tr, resmî metin): (2) çatı eğimi idarece, yöresel mimari ve iklime göre belirlenir; (3)
  "Çatı eğimi saçak ucundan hesaplanır. Çatılar parapet üzerine oturtulamaz."; (4) mahya yüksekliği 5,00 m'yi geçemez;
  (5) "Merdiven evleri, ışıklıklar, hava bacaları, alın ve kalkan duvarları … çatı örtüsü düzlemlerini en fazla 0.60
  metre aşabilir"; zorunlu tesisat hacimleri, güneş panelleri ve "çatı pencerelerinin çatı örtüsünü aşmasına" idare
  izin verebilir; (6) teras parapeti 1,10 m; (7) çatı arasında bağımsız bölüm yapılamaz. (Eski özet "baca 0,60 m'yi
  aşamaz" diyordu: maddede geçen hava bacasıdır, duman bacası değil.)
- **Binaların Yangından Korunması Hakkında Yönetmelik Md. 58(2):** "Baca çekişinin azalmaması bakımından, bacaların
  mümkün ise, komşu yüksek binalardan en az 6 m uzaklıkta yapılması ve ait olduğu bina mahyasının en az 0.8 m üzerine
  kadar çıkarılması gerekir." Duman bacası mahyanın 0,80 m üstüne çıkar; hava bacası çatı düzlemini 0,60 m'den çok
  aşamaz.
- **PAİY Md. 42:** saçak en çok 1,00 m (plan aksini söylemiyorsa), parsel sınırını aşamaz; çıkma varsa çıkmadan 0,50 m.
- **Sivas İmar Yönetmeliği:** 40 — mahya 4,00 m'de kaldıkça eğim en çok %40; en az iki yöne eğim (bina derinliği 11,50
  m'ye kadar tek yöne izin); 5 kata kadar kırmızı kiremit; konutta tonoz çatı yasak; çatı penceresi en çok 1,20 × 1,50 m,
  cepheye en az 1 m, birbirine en az 2 m; 42/5 — saçak en çok 1,00 m ve **kar siperliği zorunlu**.
- **Mimarlar Odası, Mimarlık Hizmetleri Şartnamesi:** 14.3.3 (u) ön projede "Çatı planında; eğimler, su toplama yerleri,
  dereler, tesisat ve asansör çıkıntıları, bacalar ve çatıya çıkış delikleri gösterilir. Gerekli kotlandırma ve
  açıklamalar yapılır." 14.5.3 (w) uygulama projesinde "Çatı planı çizilir. Meyiller, su toplama yerleri, dereler ...
  gösterilir ve gerekli kotlar verilir", (x) "Yağmur iniş boruları gerçek boyutlarında çizilir ve ölçüleri yazılır."
- **EK-2 Tablo 2.3** (katman öğeleri): Çatı CATI, Çatı Kaplama CKAP, Çatı Kaplama Tahtası CKTA, Çatı Taşıyıcı CTAS,
  Oluk OLUK, Yağmur İniş Borusu YIBO, Baca BACA, Harpuşta HARP, Çörten CORT.

### Başka programlar (yalnız görünüş için)

- **Revit, Roof by Footprint** (Revit 2024 Get Started, *Revit Roof Sample File*): taslağın her kenarı "Defines slope"
  ya da değil; dört kenar eğim verirse kırma, karşılıklı iki kenar beşik, bir kenar tek eğim, hiçbiri düz çatı
  (`.run/gui-evidence/mimari-m44/referans/revit-cati-turleri.png`). Kenar başına ayrı eğim verilebilir.
- **Revit, Roof Properties** (help.autodesk.com, Revit 2020): Base Level ve Base Offset From Level; "Rafter or Truss":
  "If you choose Rafter, the Plate Offset from Base is measured from the inside of the wall", Truss'ta duvarın dış
  yüzünden: çatının alt yüzü taban kotuna duvarın iç ya da dış yüzünde oturur. "Maximum Ridge Height: The maximum height
  of the top of the roof above the base level": mahya çatının üst yüzünden ölçülür. Rafter Cut: Plumb Cut (saçak alnı
  düşey), Two Cut Plumb, Two Cut Square (Fascia Depth ile).
- **Archicad, Floor Plan Display** (help.graphisoft.com, Archicad 29): çatıların varsayılanı "Projected with
  Overhead"; "the part of the element that is above the Floor Plan Cut Plane" overhead çizgileriyle, ayarlarda
  "Overhead Lines: Hidden Line". Kenar kesimleri (edge trims): vertical, perpendicular, horizontal, custom angle.
- Görüntüler: `.run/gui-evidence/mimari-m44/referans/cati-kesit-referans.png`.
- **Archicad, Placing and Moving a Skylight** (help.graphisoft.com, Archicad 29): çatı penceresi "Roof or Shell" üstüne
  tıklanarak, planda ya da 3B'de, Info Box'taki dayanak noktasıyla konur; yüzün eğimine oturur, başka bir çatı yüzüne
  sürüklenince "will automatically adjust itself to the new location"
  (`.run/gui-evidence/mimari-m44/referans/archicad-cati-penceresi-koy.png`, `…-tasi.png`).

## Karar

### 1. Çatı nesnesi (`Entity::Roof`)

- **Alanları:** saçak çizgisi (`pts`, kapalı, en az üç nokta), her kenarın eğimi (`slopes`, yüzde; `null` kalkan), kot
  farkı (`offset`), kalınlık (`thickness`, yoksa 0,25 m: kiremit, çıta, tahta ve mertek), saçak (`overhang`: saçağın
  duvarın dış yüzünden uzaklığı, yalnız bilgi), kaplama (`material`, yoksa Kiremit) ve mimari bilgi (kat, evre, durum).
- **Yüksekliği:** saçak katının **üstündedir**: katın yüksekliği verilmişse taban kotu + yükseklik, yoksa bir üst katın
  taban kotu; kendisinden yukarıda kat yoksa kendi taban kotu. Böylece çatı iki yerden çizilebilir: en üst katta (duvarlar
  oradadır, saçak bir üst katın tabanında) ya da projede "Çatı" diye ayrı bir kat varsa onun üzerinde (saçak o katın
  tabanında). Revit'in "Base Level"ı çatıyı katın kotuna oturtur; KentOS'ta duvarın tepesi çoğunlukla katın tepesi
  olduğundan en üst katta çizilen çatı kendiliğinden duvarların üstüne oturur. Kot farkı bunun üstüne eklenir.
- **Tür adı** eğimlerden okunur, ayrı alan değildir: her kenar eğimliyse "Kırma çatı", ikisi "Beşik çatı", biri "Tek
  eğimli çatı", hiçbiri "Düz çatı". Öznitelikler bu adı ve Eğim, Kalkan, Saçak kotu, Mahya kotu, Mahya yüksekliği, Saçak,
  Kaplama, Çevre ve Alan satırlarını salt okunur gösterir (düzenleme sonraki adımda).
- **Dosya:** `.kcad` şema 237, nesne türü 32 `roof`; şema 236'da çatı okunmaz (`broken/roofs-in-schema-236.kcad`).
  Sunucu tablosu `0039_feature_roofs.sql`. DXF'e saçak ve çatı hatları çizgi olarak, GeoJSON, KML, GPX ve CBS türevine
  (`cati`) saçak çizgisi alan olarak gider.
- **Komut** `cad.roofs.create` v1 ("Çatı"): katmana bir ya da birden çok çatı, tek geri alma adımı. Retler sırasıyla
  `no_objects`, `not_finite`, `invalid_roof` (nokta sayısı, eğim sayısı kenar sayısından farklı, eğim 0 ya da %500'den
  büyük, alansız ya da içe dönen saçak çizgisi …), `level_not_found` ve her oluşturan komutun ortak retleri. Ortak
  durumlar `fixtures/commands/v1/cad.roofs.create.json` (36 durum), bağımsız başvurusu `scripts/fixtures/arch_command_cases.py`.
- **Her yerde bir tür:** seçim süzgecinde "Çatı" (en sonda; kayıtlı süzgeçlerin bitleri kaymaz), Kat ver çatıya kat
  verir ve çatının adını verdiği kat silinemez, Denetim katsız çatıyı söyler, kalem tablosunda "Döşeme ve çatılar".
- **Katmanı** (EK-2 Tablo 2.3 Çatı = CATI, EK-4 4.14.3 kesilen öğenin malzemesi): kaplamasıyla `MM-CATI-KIREMIT`;
  kaplamasız `MM-CATI-GENEL`.

### 2. Çekirdek: saçak çizgisinden yüzler (`geom::roof`)

- Çatı saçak çizgisinden (kapalı, en az üç nokta) ve **her kenarın eğiminden** (yüzde; yok: kalkan) kurulur, Revit'in
  "Defines slope"u gibi. Eğimli kenarın düzlemi kenarın çizgisinden içeri doğru o eğimle yükselir; bir noktada çatı,
  oradaki eğimli düzlemlerin en alçağıdır (alt zarf). Dışbükey saçak çizgisinde bu, düz iskeletin (straight skeleton)
  çatısıdır.
- **Yüz:** her eğimli kenar için, düzleminin hiçbir öbüründen yüksek olmadığı yer (alanı varsa). **Hatlar:** kenarlar
  verildiği sırayla, eğimliyse saçak, değilse kalkan; sonra yüzlerin saçakta olmayan kenarları bir kez, yüksek ucundan:
  alçak ucu saçağın dışa dönen köşesindeyse kırma, içe dönendeyse dere, değilse mahya (yatay ya da iki iç nokta arasında
  eğik). Mahya yüksekliği yüzlerin köşelerinin en yükseği.
- Saat yönünde verilen saçak çizgisi ters çevrilir, kenarlar eğimlerini korur.
- **Henüz yok:** içe dönen köşesi olan saçak çizgisi (L, T): düz iskeleti bölünme olayları ister; reddedilir (§9).
- **Bağımsız başvuru:** `scripts/fixtures/roof_cases.py` (KentOS kodu olmadan, tam kesirlerle) 9 durum: 12 × 9 m kırma
  (%33: mahya 3 m, 1,485 m yüksek), beşik (%40: mahya kalkandan kalkana, 1,80 m), tek eğim (%10: 0,90 m), düz, ön %30 arka
  %45 (mahya dik yüze kayar), saat yönünde beşik, kare kırma (mahyasız tepe), 6-8-10 kenarlı beşgen (iki eğik mahya, beş
  kırma), üçgen. İlk beşgen seçimi bir iç teğet çemberli beşgendi: bütün kırmalar tek tepe noktasında buluşuyor, iç
  noktaları birleştiren hattı sınamıyordu; değiştirildi.

### 3. Plan çizimi

- Mimarlık Hizmetleri Şartnamesi 14.3.3 (u) ve 14.5.3 (w) "meyiller, su toplama yerleri, dereler … gösterilir ve gerekli
  kotlar verilir": saçak ve kalkan nesnenin kalemiyle, mahya, kırma ve dere yarı kalınlıkta (ISO 128-20'nin ince çizgisi);
  her yüzün ortasında saçağa doğru inen ok ve yanında eğimi (`%33`, kağıtta 12 mm ok, 2,5 mm yazı); mahyanın kotu
  (`+4.65`) en yüksek mahyanın ortasında, mahya yoksa tepe noktasında. Yazılar ölçekle büyür, alttan ya da sağdan okunur.
- Ekran, pafta, PDF ve DXF aynı parçaları çizer; düz çatıda yalnız saçak çizgisi vardır.
- **Kendi katının planında yalnız saçak, kesikli.** Çatı katının üstünde durur, o katın planının kesit düzleminin
  üstündedir; Mimarlık Hizmetleri Şartnamesi 14.5.3 (u): "Planın geçtiği düzlem ile tavan arasında kalan imalat nokta
  nokta işlenir. (Saçak ara kat çıkma vb.)". Kat görünümü (`level_view::looks`) böyle çatıları `roofs` listesine koyar:
  saçak çizgisi kesikli ve ince (üst katın döşemesi gibi, ISO 128'in 02.1'i), ama döşemeden farklı olarak **seçilir**:
  çatı o katın kendi nesnesidir. Saçağının durduğu üst katın planında (projede "Çatı" katı varsa) ve Bütün katlar'da
  bütünü çizilir: çatı planı odur. Katının yüksekliği yok ve üstünde kat yoksa çatı kendi tabanındadır, kendi planında
  bütünüyle çizilir.
  - İlk gerçek pencere denemesinde çatı 1. kat planının içine bütünüyle çiziliyordu: mahya kotu "+8.19" Merdiven holü'nün
    etiketinin üstüne biniyor, oklar iç duvarın taramasıyla çakışıyordu. Kural bunu da çözdü: kat planında yalnız saçak
    var.
  - Pafta (kat planı haritası) aynı kuralı izler (`plot::layers::shows_from_above`).
  - Gerçek pencerede bulunan ve düzeltilen eski hata: Kat görünümü değişince ekrandaki yazılar yeniden
    yerleştirilmiyordu. Bütün katlar'dan Yalnız etkin kat'a geçince çatının "%33"leri ve gizlenen Zemin kat'ın mahal
    numaraları ("005") ekranda kalıyordu. Yazıların anahtarı kat görünümünün sürümünü de tutuyor (`labels.rs`).

### 4. Çatı aracı (Mimari › Yerleştir › Çatı, `CT`)

- **Sınır (S):** *Binadan*: binanın içine tıklanır; duvarların dış yüzü Döşeme'nin *Dış yüzler*'i gibi bulunur ve saçak
  kadar dışarı ötelenir. Çatının katında duvar yoksa (ayrı "Çatı" katı) altındaki katın duvarlarına bakılır. Dış konturda
  dönmeyen köşeler (iki duvar parçasının düz birleştiği yer) atılır: aynı doğrultuda aynı eğimli iki kenar aynı yüzü iki
  kez verirdi. Eğri ya da avlulu kontur henüz çizilmez ve araç bunu söyler. *Köşeleri çiz*: saçağın köşeleri, Enter
  kapatır; duvar bilinmediği için saçak yazılmaz.
- **Tür (T):** Kırma, Beşik, Tek eğimli, Düz. **Ön kenar** Binadan'da tıklanan yere en yakın kenar, çizerken ilk kenardır:
  tek eğimli çatının suyu ön kenardan akar; beşik çatıda ön kenar ve ona en karşı doğrultudaki kenar eğimlidir, mahya ön
  kenara paralel uzanır, öbür kenarlar kalkandır. Böylece aynı binada beşiğin yönü tıklanan yerle seçilir, ayrı bir soru
  sorulmaz.
- **Değerler ve harfler** (ADR 8003): Eğim **M** (Rampa'nın Eğim'i gibi; yüzde, `33` ya da `%33`), Saçak **A**, Kalınlık
  **E**, Kot farkı **F** (boş: çatı duvara oturur, §5), Kaplama **P** (K Kapat'ın), Kat **L**. Varsayılanlar: %33 (18°; kiremitli çatının olağan
  eğimi, Sivas İmar Yönetmeliği'nin %40'ının altında), saçak 50 cm (PAİY 42 en çok 1,00 m), kalınlık 25 cm, Kiremit.
- **Önizleme** yazılacak çatıyı planıyla çizer; etiketi türünü, eğimini, mahyanın kotunu ve yüksekliğini söyler. Mahya
  saçaktan 5,00 m'den yüksekse (PAİY 40/4) etiket kırmızıya döner ve "PAİY 40: mahya en çok 5,00 m yüksek olur" der;
  yazmayı engellemez (denetim Mevzuat paneline gelecek).
- Yazınca: "Kırma çatı çizildi; mahya +4.65. Kat planında saçağı kesikli; bütünü “Çatı” planında ve Bütün katlar'da."
  (üstte kat yoksa yalnız Bütün katlar). Önizleme bütün çatıyı çizer; yazılınca planda kesikli saçak kalır, kullanıcı
  çatısının nereye gittiğini aramasın diye söylenir. Araç açık kalır.
- **Şeritte yeri:** Yerleştir panelinde Kiriş'in yanında küçük düğme. Mimari sekmesi en çok 30 düğme gösterir
  (DESIGN.md §7.3.1, ADR 8015); 31. düğme için **Rampa, Merdiven ▾ ailesine alındı** (Merdiven, Döner merdiven, Rampa):
  Revit'in Circulation paneli gibi bir çıkış ailesi; kodda da merdiven ve rampa tek araç (`FlightTool`).
- **Simgeler** cepheden görünüşle çizildi (çatının biçimi, altında ince duvar): plan simgesi (dikdörtgende mahya ve
  kırmalar) gerçek boyda zarfa (✉) benziyordu. `roof`, `roofGable`, `roofShed`, `roofFlat`.

### 5. Kesit ve görünüşte çatı (2. bölüm)

- **Mevzuat:** Mimarlık Hizmetleri Şartnamesi 14.3.4 (kesitler, ön proje) c) "Giriş saçakları, meyilli çatılar, çatı
  örtüleri kaplamaları belirtilir", g) "Çatı eğimi ve örtü malzemeleri, dereler, yağmur inişleri belirtilir … Dere
  mahya, saçak … bacalar kotlandırılır", h) "Kesit düzlemi arkasında kalan bina görünüşleri çizilir"; 14.5.4 q) "Çatı
  konstrüksiyonu … çizilir … derelerin, mahyaların … bacaların kotları ile çatı eğimi yazılır"; 14.5.3 j) görünüşte
  "Saçaklar … kalkan duvarları, oluk, mahya, baca ve çıkıntılarına kot verilir". PAİY 4: saçak seviyesi "son kat tavan
  döşemesinin üst kotu".
- **Çekirdek (`geom::section`, `Height::Sloped`):** çatının her yüzü eğik bir katı: tabanı yüzün halkası, üstü çatının
  düzlemi, altı kalınlık kadar aşağıda (düşey ölçülür). Kesilen yer düzlem kadar eğik bir bant. Görünen: düzlem bakandan
  uzaklaşarak yükseliyorsa üst yüz, bakana doğru yükseliyorsa alt yüz (çatı arasından görünen arka yüzün altı), tabanın
  kenarlarıyla; saçak çizgisi üzerindeki kenarlarda düşey yüz (saçak alnı, kalkanın ucu: Revit'in "Plumb cut"ı,
  Archicad'in "vertical" kenarı), üst ve alt çizgisiyle, köşelerde düşey çizgiyle. Bir çatının yüzlerinin paylaştığı
  kırma ve mahya bir kez çizilir; beşiğin kalkanında iki yüzün aynı düzlemdeki uçlarının tepedeki ek yeri çizilmez
  (8043 §7). Önündeki çatı arkasındakini örter.
- **Bağımsız başvuru:** `scripts/fixtures/section_cases.py` 4 yeni durum (24'e çıktı): 8 × 6 m %50 kırma çatının
  enine kesiti (üç yüzün bandı, arka yüzün altı), önden görünüşü (saçak alnı, kırmalar, mahya; arka yüz görünmez),
  beşiğin kalkanı (tepedeki ek yeri yok), arkadaki duvarın tepesini örtmesi (kırmaların arasında kalan parçası
  görünmez). Elle hesapla da karşılaştırıldı.
- **Model (`sections::roof`):** yüzler `geom::roof::face_planes` ile; kesilen bant TS 5319 SN23 "Ağaç malzeme" ile
  taranır (eğimli çatının ahşap konstrüksiyonu), kaplamasının katmanında (MM-CATI-KIREMIT), kalın çizgiyle. Düz çatı
  saçağın altında kalınlığı kadar bir döşeme. Çatı varsa "çatı (modelde yok)" notu düşer.
- **Kotlar:** görünen her çatının oturduğu katın üstü soldaki kot sütununda "Saçak" adıyla (PAİY 4'ün saçak seviyesi;
  ayrı bir saçak kotu yazmak aynı yerde iki kot üst üste bindirirdi), mahyanın üstü "Mahya" adıyla: "+6.00 Saçak",
  "+7.69 Mahya". Görünüşte de.
- **Eğim:** kesit çizgisi bir yüzün eğim doğrultusunda geçiyorsa (bandın görünen eğimi gerçek eğime eşit) bandın
  üstüne "%33"; yüzü eğik ya da yatay kesiyorsa yazılmaz (görünen eğim gerçeği değildir). 1 m'den kısa kesimlere yazı
  konmaz.
- **Görünüşte kaplama:** kaplama yazısı taraması artık mahya kotuna kadar çıkar; çatının örtüsü ("Kiremit") cephe
  kaplamalarıyla birlikte yazılır (14.3.4 c).
- **Çatının duvara oturması (araç):** Binadan'da kot farkı boş bırakılırsa çatının alt yüzü duvarın dış yüzünde katın
  üstüne oturur (Revit'in "Truss"ı): kot farkı = kalınlık − saçak × eğim (25 cm − 50 cm × %33 = 8,5 cm); köşeler
  çizilince saçak çizgisinde (kot farkı = kalınlık). İlk denemede çatı bandı 50 cm saçakla duvar başlarına 8,5 cm
  gömülüyordu. Kot farkı yazılırsa o kullanılır; boşaltılınca yeniden oturur.
- **Bulunan eski hata:** kesitin sağındaki kat yüksekliği yazıları ("3,00") π/2 ile döndürülüyordu; metnin dönüşü
  derece olduğu için 90° yerine 1,57° dönüktüler (M42–M43). Artık 90°.

### 6. Baca ve çatı penceresi (3. bölüm)

- **Mevzuat:** Binaların Yangından Korunması Hakkında Yönetmelik 58/2: duman bacası "ait olduğu bina mahyasının en az
  0.8 m üzerine kadar" çıkar. PAİY 40/5: hava bacası çatı örtüsünün düzlemini "en fazla 0.60 metre" aşar; çatı
  pencerelerinin örtüyü aşmasına idare izin verir. Sivas İmar Yönetmeliği 40: çatı penceresi en çok 1,20 × 1,50 m,
  cepheye en az 1 m, birbirine en az 2 m. Mimarlık Hizmetleri Şartnamesi 14.3.3 u: çatı planında "bacalar ve çatıya
  çıkış delikleri gösterilir"; 14.3.4 g ve 14.5.3 j: bacalar kotlandırılır.
- **Çatının parçaları (`RoofFields.parts`, `.kcad` şema 238):** duman bacası (`chimney`), hava bacası (`vent`), çatı
  penceresi (`window`). Her biri planda ortası (`at`), eni ve derinliği; baca ve hava bacasının dönüşü; tepesinin
  yüksekliği (`rise`): duman bacası mahyanın, hava bacası çatının o yerdeki en yüksek köşesinin üstünde; verilmezse
  0,80 m ve 0,60 m. Pencerenin yüksekliği verilmez (çatının düzlemindedir); eni yüzün saçağı boyunca, boyu eğim boyunca
  çatının düzleminde ölçülür (Sivas'ın 1,50 m'si böyle ölçülür), planda boy √(1 + eğim²)'ye bölünür; yüzün eğimine göre
  döner. En çok 100 parça, her ölçü en çok 10 m. Şema 237 dosyaya parça yazılmaz, okuyan "şema 238 ister" der.
- **Nereye konur** (`arch_elements::roof_valid`, `cad.roofs.create` ve `cad.arch.edit` ortak): bacanın ortası saçak
  çizgisinin içinde; pencerenin ortası eğimli bir yüzde, dört köşesi aynı yüzde. Kırmaya, mahyaya, dereye ya da saçağa
  taşan, düz çatıdaki pencere `invalid_roof` ile reddedilir; çatı düzenlenip penceresinin yüzü kalmazsa düzenleme de.
- **`cad.arch.edit` çatıyı da düzenler:** eğimler (`slopes`), kot farkı (`offset`, kirişinkiyle aynı alan), kalınlık,
  saçak, kaplama (`material`; null Kiremit), parçalar (`parts`: hepsi birden yazılır, boş liste kaldırır), kat ve tadilat
  durumu. Ret metinleri "aks, kolon, kiriş, merdiven, rampa ya da çatı" oldu. 14 yeni ortak durum (72'ye çıktı).
- **Planda** (`geom::roof::part_pieces`): duman bacası çerçeve ve iki köşegen, hava bacası çerçeve ve bir köşegen, yanında
  tepesinin kotu; pencere çerçeve ve 5 cm kasa. Çatının kendi katının planında (yalnız kesikli saçak) parçalar çizilmez.
- **Kesitte ve görünüşte:** baca ve hava bacası, çatının oturduğu katın üstünden tepesine bir prizma: kesilince ANSI31
  taramalı (tuğla), EK-2'nin Baca öğesinde (MM-BACA-GENEL). Tepesi kot sütununda "+8.49 Baca" / "Hava bacası" (14.3.4
  g). Pencere yüzün üstünde çerçevesiyle görünür. Çatının yüzleri bacayı boşluk (`Void`) olarak taşır: çekirdekte
  eğik bandın kesildiği aralıktan, yükseklikleri bandı içine alan boşluğun aralığı çıkar (`geom::section`, duvardaki
  açıklık gibi); ilk denemede bandın çizgileri bacanın içinden geçiyordu. Bağımsız başvuruya kural ve bir durum eklendi
  (25'e çıktı): beşik çatının kuzey yüzünü delen 50 cm'lik baca, bant iki yanında iki parça.
- **Yakın kotlar:** bir kot bir üsttekine yazısının yüksekliğinin 2,2 katından yakınsa (hava bacasının tepesi mahyanın 16
  cm altında) yazısı kendi genişliği kadar sola alınır, çizgisi binaya kadar uzar: ilk denemede "+7.62 Hava bacası" ile
  "+7.78 Mahya" üst üste basılıyordu. Sağdaki yükseklik zinciri katların, saçağın ve mahyanın kotlarıyla kalır; baca
  tepeleri zincire girince 16 cm'lik bir halka ve üst üste binen "0,80 / 0,16" yazıları çıkıyordu. Baca prizması çatının
  kaplamasını almaz: görünüşün kaplama listesi onu "Kiremit" saymaz.
- **Araçlar (Mimari › Yerleştir › Çatı ▾: Çatı, Çatı bacası, Çatı penceresi):** şeritte Çatı düğmesi bir aileye döndü,
  düğme sayısı değişmedi (30). *Çatı bacası* (`CB`): Tür (T) Duman bacası / Hava bacası, En (E) ve Derinlik (D) 40 ×
  40 cm, Yükseklik (Y) kuralın değeri; en yakın saçağa göre ±45° içinde döner (dik duran binada dönmez). Tür
  değişince yazılan yükseklik düşer, yeni türün kuralı gelir. *Çatı penceresi* (`CPN`): En (E) 78 cm, Boy (Y, Kolon'un harfi) 118 cm,
  olağan çatı penceresi. Kısa adlarda Ç C okunduğu için `ÇP` Kopyala'nın `CP`'siyle çakışırdı; `PN` Pencere'nin adıdır.
  Araçlar tıklanan yerin altındaki çatıyı bulur: görünümün bütün çizdiği önce, sonra saçağı en yüksek; çatının kendi
  katının planında "bacaları ve pencereleri üst katın planında ya da Bütün katlar'da konur" der. Yazılı nokta (5.2,6.5)
  da olur. Önizleme parçayı çizer, tepenin kotunu ve mahyayı söyler; 80 cm'den alçak duman bacası, 60 cm'den yüksek hava
  bacası ve Sivas'ın 1,20 × 1,50 m'sinden büyük pencere kırmızı uyarılır ama yazılır (denetim Mevzuat paneline
  gelecek). Yazım tek geri alma adımı ("Mimari nesneyi düzenle"); "Duman bacası kondu; tepesi +8.58 (mahyanın 0.800 m
  üstünde)."
- **Simgeler:** `roofChimney`, `roofVent`, `roofWindow`, çatı simgeleri gibi cepheden: beşik çatı, üstünde baca, başlıklı
  boru ya da eğimde pencere.
- **Öznitelikler:** her parça bir satır, aynı türden birden çoksa numaralı: "Duman bacası — 0.400 × 0.400 m · tepesi
  +8.58".

### 7. Çatı planı paftası (4. bölüm)

- **Mevzuat:** PAİY 57(2)(a) mimari projede çatı planını ister; Mimarlık Hizmetleri Şartnamesi 14.3.3 u çatı planında
  "eğimler, su toplama yerleri, dereler, tesisat ve asansör çıkıntıları, bacalar ve çatıya çıkış delikleri" ile
  kotları. Dijital proje yönetmeliği EK-2 Tablo 2.13'te çatı planının kendi dosya tipi var: **CAT** ("çatı planı ve
  detayları"); Tablo 2.14'te çatı bir kat değil: yeri **NNN** (kattan bağımsız). Dosya adı `…_PL_CAT_NNN_01_000.pdf`.
- **Çatı planı kuralı** (`level_view::roof_plan`): bütün çatılar bütün hâliyle (kesikli değil), bacaları ve
  pencereleriyle; katsız nesneler (akslar, kesit ve görünüş işaretleri, katsız ölçü ve yazılar) durur; herhangi bir
  katın başka her nesnesi (duvar, kapı, mahal, merdiven, döşeme, katın ölçüleri) çıkar. Revit ve Archicad'de çatı planı
  çatının üstündeki katın planıdır; en üst katın çatısının üstünde kat olmadığı için kural adıyla istenir.
- **Paftanın haritası:** `MapLevel.roof` (isteğe bağlı, yanlışsa yazılmaz; `MapLevelChoice::Level.roof` ile
  `setViewport` da kurar). Pafta kitabı `.kcad`'de olduğu gibi saklanır (şema değişmedi); alanı bilmeyen eski sürüm
  bu kitabı okumaz, sessizce düşürmez. Masaüstündeki pafta haritası, paftanın modelde düzenlenmesi ve PDF çıktısı
  aynı kuralı kullanır (`kentos_plot::layers::level_looks`, `Spatial` paftanın görünümü).
- **Ruhsat takımı** (`permit_set::pages`): bir çatı kendi katının planında kesikli (saçağı katın üstünde) ve üstünde
  planı olan bir kat yoksa (kat yok ya da boş) katların planlarından sonra "Çatı planı" eklenir: planlarla aynı zemin
  ve ölçek (Ruhsat 1/100), haritanın katı bu çatıların en yükseğinin katı. Üstte planı olan kat varsa o planda çatı
  zaten bütün görünür; ayrı pafta açılmaz. "Çatı kendi katının üstünde mi" kuralı tek yerde
  (`roofs::over_its_storey`); kat görünümü ona başvurur.
- **Sonraya:** çatı planında alttaki dış duvarların kesik çizgiyle gösterilmesi (Türk kaynağında doğrulanamadı; Revit
  ve Archicad göstermiyor), saçak ve bina ölçüleri, oluk ve iniş boruları.

## Doğrulama

- `geom::roof` birim testleri 7 (kırma, beşik, tek eğim, düz ve saat yönü, içe dönen köşe reddi, planın okları ve
  yazıları, kendi katının planında kesikli saçak); `tests/all/roofs.rs` başvurunun 9 durumu 1e-7 içinde.
- `cad.roofs.create`: 36 ortak durum (`fixtures/commands/v1/cad.roofs.create.json`, bağımsız başvuru
  `scripts/fixtures/arch_command_cases.py`); KCAD şema 237 örneği ve şema 236'da red; yetenek listesinde `roof` her tür
  denemesinden geçti (Nokta kur'un paraleli çatının saçağını alıyor: listeye eklendi).
- Uygulama: `roofs` 2 (türe göre eğimler ve ön kenar; dönmeyen köşe atılır). Etkileşim: Çatı aracı 6 (Binadan kırma —
  iki parçalı güney duvarının köşesi atılır, 13 × 10 m'de mahya +4.65; beşik yönü tıklamaya göre; köşeleri çiz ve tek
  eğim; dışarıya tıklama; çatı katında duvarsız; kat görünümünde kesikli ve seçilir). Masaüstü: Mimari sekmesi 30 düğme,
  kendi katında kesikli saçak ve görünüm değişince yazıların yeniden yerleşmesi.
- Gerçek pencere (Xvfb 1920 × 1080, gerçek fare ve klavye; `fixtures/interaction/real/cati.txt`, `cati-besik.txt`):
  1. kat etkin, Binadan kırma: saçak duvarın 50 cm dışında, mahya +7.69 (saçak +6.00, 1.691 m); batı duvarına yakın
  tıklamayla beşik: mahya kuzey–güney, +8.19 (2.186 m); Yalnız etkin kat'ta yalnız kesikli saçak, yazılar yeniden
  yerleşti ("105 Mutfak", "%33" yok).

- 2. bölüm (§5): çekirdek ile bağımsız başvuru 24 durumda aynı (4'ü çatı); `roof_sections` 4 (B-B'de iki bant ve
  "%33", "+6.00 Saçak", "+7.69 Mahya", çatı artık "eksik" değil; A-A'da yatay bantta eğim yazılmaz; çatısız binada not
  durur; kuzey görünüşünde 3 m'lik mahya, 13,25 m'lik saçak alnı, iki kırma, "Kiremit"); Çatı aracı 7 (duvara oturma:
  %40'ta kot farkı 5 cm, mahya +5.05; yazılan kot farkı korunur, boşaltılınca yeniden oturur); `face_planes` 1.
- Gerçek pencere (`fixtures/interaction/real/cati-kesit.txt`, `cati-gorunus.txt`): B-B'de iki bant mahyada birleşti,
  saçak alnı düşey, altı duvarın dış yüzünde +6.00'a oturdu, "+7.78 Mahya" (kot farkı 8,5 cm ile), sağdaki "3,00"lar
  düşey; kuzey görünüşünde saçak alnı, kırmalar, mahya.

- 3. bölüm (§6): `geom::roof` parçaların halkası, tepesi, yüzü ve pencerenin yüzde kalması (duman bacası 80 cm ile
  "+8.29"); kesit başvurusu 25 durum (yeni: beşik çatının yüzünü delen baca, bant iki parça). KCAD şema 238 örneği
  (`roof-parts.kcad`), şema 237'de parçanın reddi, şema 239'un reddi; KCAD testleri 244 + 6. Ortak durumlar:
  `cad.roofs.create` 47 (11 yeni: 2 geçerli, 9 ret), `cad.arch.edit` 72 (14 yeni: parçalar, boş liste, eğimler, kot
  farkı, kalınlık, saçak, kaplama, kat; dışarıdaki baca, düzleşen çatının penceresi, eksik eğim, sonsuz değerler,
  başka türün alanı), `cad.arch.edit.distance` 30 (ret metni). Uygulama: `roof_sections` 5 (yeni: B-B bacayı keser,
  MM-BACA-GENEL taraması, "+8.49 Baca", bir bant fazla, hava bacasının kotu mahyanın solunda, zincirde "1,69" var,
  "0,80"/"0,16" yok), kesit, görünüş ve ruhsat takımıyla 22. Etkileşim: Çatı bacası/penceresi 4 (mahya + 80 cm, tek
  geri alma adımı; hava bacası 90 cm'de PAİY 40/5 uyarısı, tür değişince kuralın yüksekliği; pencere yüzde, kırmaya
  taşan reddedilir, 1,60 m boyda Sivas uyarısı; çatı dışında ve kendi katının planında konmaz), Çatı 7. Masaüstü 169
  (Mimari sekmesi 30 düğme; Boy her araçta Y; istem ve panel aynı adla; katalog ve kısa adlar).
- Gerçek pencere (Xvfb 1920 × 1080, gerçek fare ve klavye; `fixtures/interaction/real/cati-baca.txt`,
  `cati-baca-gorunus.txt`): "baca" araması Baca ve Çatı bacası'nı buldu; önizleme "Tepesi +8.58 · mahyanın 0.800 m
  üstünde", "Mahya +7.78"; yazılı noktayla (5.2,6.5) B-B üstüne baca; T ile hava bacası, güney yüzde tepesi +7.62;
  pencereler güney ve batı yüzde (batıdaki yüze göre dönük), kırma yanında kırmızı ret; Öznitelikler dört parçayı
  ölçü ve tepe kotuyla listeledi; B-B'de taramalı baca, bant iki yanında kesik; kuzey görünüşünde baca ve hava bacası.
  Gerçek pencerede bulunan iki çizim kusuru düzeltildi (üst üste binen kotlar, zincirdeki baca halkaları); bir harf
  seçeneği (T) yazılır yazılmaz uygulanır, ardından gelen Enter "Bitir"dir (senaryo buna göre).

- 4. bölüm (§7): pafta `map_levels` (çatı planı bütün yazılır ve geri alınır), Ruhsat takımı'nın pafta testleri 21;
  etkileşim: çatı planı kuralı (çatı bütün, kendi katının duvarı çıkar, katsız çizgi kalır); çıktı: çatı planı
  haritası iki katın mahal adlarını yazmaz, katsız yazı kalır; uygulama: çatılı binanın takımında "Çatı planı" 1. kat
  planından sonra, PL / CAT / NNN, haritası `roof`; çatısız binada yok. Masaüstü: Ruhsat takımı'nın 15 × 6 m binası
  çatı, baca ve pencereyle: UP-01…UP-05, UP-05 Çatı planı; `260457_00_MM_XXXX_PL_CAT_NNN_01_000.pdf` PDF/A-2'ye uyar
  (mahya +7.15, baca +7.95; dosya `.run/gui-evidence/mimari-m44/p4/ruhsat/`).
- Gerçek pencere (`fixtures/interaction/real/cati-plani-paftasi.txt`): takımda UP-05 Çatı planı planlardan sonra;
  paftası A3, 1/100, haritada bütün çatı, baca (+8.58) ve pencere, akslar ve işaretler, duvar ve mahal yok; antette
  "…_PL_CAT_NNN_01_000".
- Bulunan iki hata: görünüm penceresinin yaması katı yalnız `id` ve `below` ile yazıyordu (çatı planı kayboluyordu);
  geri alma yaması eski katta olmayan alana `null` yazıyor ve `bool` alan onu okuyamıyordu (`below` için de
  geçerliydi, testlere hiç düşmemişti): ikisi de `null`ı yanlış okur (`default_on_null`).

## Sonraya kalan

- §9: içe dönen köşeli saçak (L, T) ve dereler.
- Oluk, iniş borusu, kar siperliği; PAİY 40/42 ve Sivas denetimleri Mevzuat panelinde (PAİY 40'ın
  mahya yüksekliği saçak seviyesinden, yani çatının oturduğu katın üstünden ölçülmeli: kot farkı da sayılır; duman
  bacası mahya + 0,80 m, hava bacası 0,60 m, pencerenin cepheye 1 m ve birbirine 2 m uzaklığı); bacanın duvar içi
  bacayla (Baca aracı) bağı; çatının ve parçalarının Öznitelikler'den düzenlenmesi (komut hazır: `cad.arch.edit`);
  çatı katmanlarının (kiremit, çıta, tahta, mertek) ayrı ayrı çizilmesi (14.5.4 q, uygulama projesi).
- `scripts/ui/ribbon.py` depodaki `catalog/v1/ribbon.json`'u artık aynen üretmiyor: sonradan eklenen arazi ve nokta
  bulutu girdilerini ve İmar panelini siliyor. Bu adımda çatının iki girdisi dosyaya elle eklendi; betik ayrıca
  güncellenmeli.
