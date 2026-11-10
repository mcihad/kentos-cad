# ADR 8044: Çatı ve çatı planı — saçak çizgisinden çatı yüzleri, mahya, kırma, dere; çatı planı

- **Durum:** sürüyor (10 Kasım 2026); `plan/architecture-advanced.md` ADV-20, M44. 1. bölüm yapıldı: çekirdek
  geometri (§2), çatı nesnesi (§1), plan çizimi (§3) ve Çatı aracı (§4). Sırada kesit ve görünüşte çatı, baca, çatı
  penceresi, oluk ve iniş borusu, kar siperliği, çatı planı paftası ve PAİY 40/42 denetimi.
- **Bağlam belgesi:** ADR 8040 (düşey model: kat yükseklikleri, döşemenin katmanları), ADR 8042–8043 (kesit ve görünüş:
  "çizilmeyen: çatı"), ADR 8008 (EK-4 katman anahtarı), ADR 8011 (ölçeğe göre ayrıntı).

## Bağlam

KentOS'ta çatı nesnesi yok: döşemenin "Teras" rolü düz çatıyı karşılıyor, eğimli çatı ise elle çiziliyor. Kesit ve
görünüş (M42–M43) çatıyı "çizilmeyen: çatı" diye söylüyor; ruhsat takımında çatı planı paftası yok. PAİY 57(2)(a) mimari
projede çatı planını ister.

## Araştırma

### Türkiye mevzuatı

- **PAİY Md. 40:** (2) çatı eğimi idarece, yöresel mimari ve iklime göre belirlenir; (4) mahya yüksekliği en çok 5,00 m;
  (5) merdiven evi, ışıklık ve baca çatı örtüsünü en çok 0,60 m aşabilir; (6) düz çatı parapeti en çok 1,10 m, kullanılan
  terasta 1,10 m parapet ya da korkuluk; (7) çatı arasında bağımsız bölüm yapılamaz.
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
  **E**, Kot farkı **F**, Kaplama **P** (K Kapat'ın), Kat **L**. Varsayılanlar: %33 (18°; kiremitli çatının olağan
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

## Sonraya kalan

- §9: içe dönen köşeli saçak (L, T) ve dereler.
- Kesit ve görünüşte çatı (notu şimdilik "çatı (kesit ve görünüşte henüz çizilmiyor)"); baca, çatı penceresi, oluk, iniş
  borusu, kar siperliği; çatı planı paftası; PAİY 40/42 ve Sivas denetimleri; çatının Öznitelikler'den düzenlenmesi
  (kenar eğimi, saçak, kaplama).
- `scripts/ui/ribbon.py` depodaki `catalog/v1/ribbon.json`'u artık aynen üretmiyor: sonradan eklenen arazi ve nokta
  bulutu girdilerini ve İmar panelini siliyor. Bu adımda çatının iki girdisi dosyaya elle eklendi; betik ayrıca
  güncellenmeli.
