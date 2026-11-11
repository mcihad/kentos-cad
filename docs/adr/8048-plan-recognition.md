# ADR 8048: Planı tanı — patlatılmış DWG/DXF planından duvar ve açıklık

- **Durum:** 1. ve 2. bölüm, 3. bölümün kolon, blok, duvar üstü açıklık ve duvar ucundaki kapı adımları yapıldı (11
  Kasım 2026); `plan/architecture-advanced.md` ADV-25, M48. Tanıma çekirdekte
  (`kentos_geometry_core::ops::plan_recognition`, §1), komutu `cad.arch.recognize` v1 (§2), masaüstü aracı Mimari ›
  Duvar ▾ › Planı tanı (§4); bölüm binasının ofis planı tanımla birebir okunuyor, gerçek pencerede kabul edildi
  (Doğrulama); kolon katmanının dikdörtgeni ve dairesi kolon olur (§5); bloklar yerinde okunur (§6). Açık: L ve
  düzensiz kolon, duvar stilinin genişlikten seçilmesi, yüzleri kesilmeden çizilmiş pencere, eğri
  duvar, duvar ucundaki kapı, bloklar (BLOCKIFY benzeri) ve gerçek bir ofis DWG'siyle tadilat örneği (§3).
- **Bağlam belgesi:** ADR 0339 §5 (Çizgiden duvar, `cad.walls.convert`), ADR 0340 §2 (duvar birleşimleri), ADR 0341
  (açıklıklar), ADR 0772 (açıklık türleri), ADR 8039 (tadilat durumu), ADR 8046 (DWG/DXF katman profili), ADR 8047
  (binanın IFC'si).

## Bağlam

Plan ADV-25: "Eski DWG planından tanıma: patlatılmış plandan duvar, açıklık ve donatı tanınır"; ilk kabul "Gerçek bir
ofis DWG'siyle tadilat örneği yapılır." Belediyenin arşivindeki onaylı projeler ve ofislerin eski işleri DWG'dir:
duvar iki çizgi, kapı bir kanat ve yay, pencere birkaç paralel çizgi, kolon taralı bir dikdörtgen; çoğu blok
patlatılmıştır (AutoCAD EXPLODE). KentOS'ta bu çizim yalnız çizgidir: ne IFC modeli ne mahal ne denetim ondan çıkar.

Mevzuat: Mimarlık ve Mühendislik Projelerinin Dijital Olarak Hazırlanması Hakkında Yönetmelik (RG 5.8.2026/33331)
Md. 4(4)–(5) yapı ruhsatı başvurusunda BIM modelini (IFC) ister; Geçici Md. 1 bunu 2029–2033 arasında şehrin
büyüklüğüne göre zorunlu kılar, Geçici Md. 2 o tarihten önce başvurusu yapılmış yapıları dışarıda bırakır. Yani o
tarihten sonraki bir tadilat projesi, yapısı eskiden yalnız DWG olarak onaylanmış olsa da modelle gelir: eski plan
modele dönmelidir. Md. 9 model ile CAD çiziminin uyumlu olmasını ister: tanınmayan çizgi sessizce kaybolmamalı, çizimde
kalmalı ve söylenmelidir. Tadilatta mevcut duvar Mevcut durumdadır (ADR 8039; PAİY'nin tadilat ruhsatı).

Başka programlar (görünüş ve iş akışı için okundu; ayrıca `docs/research/2026-11-07-mimari/W1-autocad-dwg.md`):
- **Chief Architect, Convert CAD to Walls:** duvar, pencere, kapı ve korkuluk için katman seçilir; duvar türleri
  ölçülen kalınlığa 25 mm içinde eşlenir; kapı yaydan bulunur.
- **BricsCAD BIMIFY / QuickDraw:** kalınlık paralel çizgilerin aralığından; BLOCKIFY aynı çizgi kümelerini bloğa çevirir.
- **Archicad Magic Wand:** tek tek izler; forumlarda kapı ve pencere boşluğunda kopuyor.
- **Vectorworks Create Objects from Shapes:** seçilen şekillerden nesne, katman eşlemesiyle.

Ortak ders: rol katmandan gelir (ofisler duvar, kapı, pencere ve kolonu ayrı katmanda çizer), geometri yalnız kalan
belirsizliği çözer.

## Karar

### 1. Tanıma (`kentos_geometry_core::ops::plan_recognition`)

Girdi: rolleriyle düz çizgiler (çizgi, çoklu çizginin düz kenarı), yaylar (yay, büküm), kapalı çevreler (kolon).
Uzunluklar metre; ayarlar en ince duvar 0,05 m, en kalın 0,60 m, en geniş açıklık 3,00 m, açı 1°, tolerans 5 mm.

- **Bir doğru üzerindeki çizgiler tek çizgidir.** Duvar çizgileri doğrultularına göre sınıflanır (1° içinde; sınıfın
  yönü en uzun çizgisinin, `t` doğuya ya da kuzeye artar); bir doğru üzerinde 5 mm içinde kalan ve örtüşen ya da uç uca
  gelen çizgiler bir "koşu"dur. Parça parça, iki kez ya da ters çizilmiş yüz bir kez çizilmiş gibi okunur.
- **Duvar bir banttır.** Aynı koşuların sürdüğü her aralıkta paralel koşular enine sırasıyla gruplanır: grup en çok
  0,60 m geniş, en az 0,05 m; en çok bir aralığı 0,10 m'den geniştir (çekirdek), dar aralıklar bileşenin çizgisidir
  (yalıtım, sıva). Böylece 0,05 m yalıtım çizgili 0,25 m duvar tek 0,30 m duvar, 0,40 m şaftın iki yanındaki 0,15 m
  duvarlar iki duvardır. Bandın çizgi boyunca sürdüğü yer bir parçadır; parçaları toplamı genişliğinden kısa bant duvar
  değildir (dar bir boşluğun iki pervazı, çizgiyle çizilmiş kolon).
- **Bandın kesildiği yer**, sırayla: bir yüz sürüyorsa birleşimdir (öbür yandan saplanan duvar); pervazın çevresinde
  boşluk kadar (0,35–1,15 katı) bir yay kapıdır, iki pervazda yarım genişlikte iki yay iki kanatlı kapı — menteşe yayın
  merkezinin olduğu pervaz, açıldığı yan yayın ortasının yanı, açısı yayınki (90°'den 1°'den çok farklıysa);
  boşluk boyunca, yüzlerin 0,25 m içinde ve boşluğun en az yarısını örten pencere çizgileri penceredir; kolon ya da
  kesen/saplanan başka duvar birleşimdir; içi boş ve en geniş açıklıktan dar aralık Boşluk'tur (0,30 m'den darı çizim
  hatası, duvar sürer); daha geniş aralık duvarı bitirir. Kapı kolonun yanındaysa kolonun yüzünden açılır.
- **Duvarın üstüne çizilmiş açıklık.** Ofis blokları çoğu zaman duvarı kesmez, silme maskesiyle (wipeout) örter: yüzler
  açıklığın altında sürer. Böyle bir duvarda, açıklığı olmayan yerde, yüzlerin arasında birbiriyle örtüşen **iki ya da
  daha çok pencere çizgisi** (0,30 m'den uzun, en geniş açıklıktan kısa) o uzunlukta bir penceredir; tek çizgi pencere
  değildir (denizlik, lento işareti). Duvarın bandındaki bir noktanın çevresinde dönen **kapı yayı**, kanadı duvar
  boyunca kapanıyorsa menteşeden kanadın ucuna, oradaki kasalara (duvarın içindeki kapı çizgileri) dek bir kapıdır.
  Pervaz çiftinin bandı (dar boşlukta) birleşim sayılmaz: duvarın yüzlerinin dışına taşmayan bant bir duvar değildir.
- **Duvarlar buluşur.** Duvar ucu başka bir duvarın eksenine yarı kalınlıklarının toplamı (ya da durduğu kolonun
  ötesi) kadar yakınsa ona uzar: T'de eksenine, köşede ikisi de kesişime (15°'den geniş açıda); birleşimi ADR 0340 §2
  çözer. Duvar kolondan geçer (bölüm binasında olduğu gibi).
- **Duvar ucundaki kapı.** Hiçbir duvara ulaşmayan bir uç, doğrultusunda ilk karşılaştığı duvarın yüzüne 0,30 m ile en
  geniş açıklık arasında bir boşluk bırakıyorsa ve o boşlukta kapı yayı ya da pencere çizgileri varsa (koridor
  köşesindeki kapı) o duvarın eksenine uzar (T), açıklık uzadığı kısma konur: KentOS'ta açıklık duvarının içindedir.
  Kanıt yoksa uç olduğu yerde kalır (duvarla duvar arasındaki kapısız geçiş).
- **Alınan kaynak:** duvar çizgisi bir duvarın bandında (uçta en kalın duvarın yarısı kadar ötesiyle: köşenin dış
  yüzü), kapı ve pencere çizgisi ve yayı açıklığının bölgesinde (tek katmanda çizilmiş planda duvar katmanındaki kanat
  da); kaynağın bütün parçaları alındıysa kaynak alınmıştır. Kolon §5'te.

Çıktı: duvarlar (eksen, genişlik, ilk çizgisi) ilk çizgilerinin sırasıyla; açıklıklar duvarına ve duvar boyunca;
sayılar nanometreye yuvarlanır (çizim milimetreye çizilir, toplamları daha incesini taşımaz).

### 2. Komut `cad.arch.recognize` v1

- **Roller katmandan.** `roles` verilen katmana; verilmeyenin adından: duvar, perde, bölme, wall → duvar; kapı, door →
  kapı; pencere, window, glaz, camekan, cam → pencere; kolon, column, col → kolon; tarama, ölçü, yazı, aks, merdiven,
  mobilya, eşya, korkuluk, kesit, pafta, antet … → yok sayılır (DUVAR-TARAMA duvar değildir). Hiçbir katman duvar
  değilse adı bir şey söylemeyen katman ("0", "Çizim") duvardır: tek katmanda çizilmiş plan. Türkçe harfler ve büyük
  küçük harf bir sayılır.
- **Yazılanlar.** Duvar düz, ortası eksende, okunan genişlikte, stilsiz; `arch` verilirse (kat, tadilat durumu) hepsi
  onu alır. Açıklık KentOS türüyle: kapı genişliğine göre Kapı 80 (0,85 m altı) ya da Kapı 90, iki kanatlıysa İki
  kanatlı kapı 140; pencere Pencere 120 (kanatları türün); boşluk Boşluk 100; her biri okunduğu genişlikte, ortası
  boşluğun ortasında. Menteşe, yan ve açı yalnız kapıda okunur. Duvarlar `wallLayerId`'ye ya da ilk çizgisinin
  katmanına; kapı ve pencere `openingLayerId`'ye ya da rolündeki ilk katmana; boşluk duvarının katmanına. Tek geri
  alma adımı "Planı tanı".
- **Kaynaklar.** Bütünüyle okunan kaynak `keepSources` verilmezse silinir; kalanlar çizimde kalır ve
  `plan_unrecognized` uyarısı nedenleriyle sayar ("23 nesne yok sayılan katmanlarda (Aks, Merdiven, MOBILYA, YAZI);
  16 kolon çizgisi (kolon henüz kurulmuyor)"). Kilitli katmandaki kaynak atlanır (`keepSources` ile okunur).
- **Retler,** sırayla: `no_entities`, `invalid_uid`, `entity_not_found`, `layer_locked` (hepsi kilitli),
  `layer_not_found` (rolün katmanı), `not_finite`, `recognize_option` (genişlik sıfırdan büyük, en ince en kalından
  ince, açıklık sıfırdan büyük, açı 0–45°), `level_not_found` (`arch.level`), `plan_unrecognized` (hiç duvar yok),
  yazılan katmanların `layer_not_found`, `not_a_layer`, `layer_locked`, sonra sürüm. Plan yazmadan duvarları,
  açıklıkları (konağı geçici kimlikle), silinecek ve kalacak kaynakları gösterir.

### 4. Masaüstü aracı: Mimari › Duvar ▾ › Planı tanı (`tool.planRecognize`)

Seçim önce, değiştirme araçları gibi: plan pencereyle seçilir (seçim yoksa araç seçtirir). Araç ayarlarında Kat (L),
Kaynak çizgiler (K: Koru/Sil) ve **seçimin her katmanı bir satır**: adı ve rolü (Duvar, Kapı, Pencere, Kolon, Yok
say), adından gelen değeriyle; değiştirilen rol komuta `roles` olarak gider, adı bir şey söylemeyen katmanın rolü
gösterilir (duvar katmanı yoksa Duvar). Komutun ve aracın rolü aynı işlevden okunur
(`arch_recognize::layer_roles`). Önizleme komutun planıdır: duvarlar Duvar aracının önizlemesi gibi, açıklıklar
pervazlarından çizilen dörtgenle; imlecin yanında "8 duvar, 3 kapı, 4 pencere, 0 boşluk; 88 nesne silinir; 39 nesne
kalır". Enter, Uygula ya da hızlı sağ tık yazar; yazılmayan rol (duvar katmanı Yok say) sebebini söyler, hiçbir şey
yazmaz. Kat ve tadilat durumu Çizgiden duvar'daki gibi (`cx.phased`): mevcut binayı tanırken etkin durum Mevcut ise
duvarlar Mevcut gelir. 24'ten çok katmanda kalanlar adlarının rolüyle okunur ve söylenir.

### 5. Kolonlar

Kolon katmanının kapalı çevresi (çoklu çizgi, alan) dört dik köşeliyse **dikdörtgen kolon** olur: ortası dört köşenin
ortası, eni doğu–batıya yakın kenarı (±45°), boyu öbürü (eşitse yazılmaz), dönüşü o kenarın açısı; dairesi **yuvarlak
kolon** (çapı kadar). Kolon dolgulu (betonarme kesit), katmanı kendi katmanı, `arch` verilmişse onu alır; malzemesi
yazılmaz (Betonarme varsayılanı, ADR 8041 §2). Ortası bir kolonun içinde kalan kolon katmanı taraması kolonla gider.
L ya da düzensiz çevre kolon olmaz, kalır ve söylenir ("1 kolon nesnesi kolona çevrilemedi (dikdörtgen ya da daire
değil)"). Kolon, duvarın tanınmasında yine köprüdür (§1). Çıktıda `columns`, planda kolon nesneleri; araç önizlemede
çevrelerini çizer ve sözüne "8 kolon" ekler. Sayılar nanometreye yuvarlanır (30° dönüş 0,523598776).

### 6. Bloklar yerinde okunur

Ofisler kapıyı, pencereyi ve kolonu çoğunlukla blok olarak tutar (KAPI-90-15, PENCERE-120-25, KOLON-30); planı
patlatmadan getirdiklerinde bu nesneler INSERT'tür. Seçimdeki her blok **çizimi değiştirmeden**, Patlat'ın açtığı gibi
açılır (`Blocks::explode`, iç içe blok sekiz kata dek): parça kendi katmanında, katmanı olmayan (bloğun "0" katmanı)
parça yerleştirmenin katmanında okunur, her biri o katmanın rolüyle. Döndürülmüş ve aynalanmış yerleştirme parçalarını
yerlerinde verir. Kapı bloğunun yayı ve kanadı kapı, pencere bloğunun çizgileri pencere, kolon bloğunun çevresi kolon
olur; bütün parçaları okunan blok silinir. Parçaları yok sayılan katmanlarda kalan blok (bir mobilya) yok sayılan
katmanda sayılır; açılamayan blok "N blok açılamadı" diye söylenir.

## Doğrulama

- Çekirdek: 15 test (oda, kapı, dışa açılan kapı, iki kanat, pencere, boşluk ve pervazları, kalem kayması, geniş
  aralık, T bölme, kolonlar, kolon yanındaki kapı, yalıtım çizgisi, şaft, parçalı/çift/ters yüz, eğik duvar, ilgisiz
  çizgiler).
- **Bölüm binasının ofis planı** (`scripts/fixtures/arch_recognize_plan.py` → `fixtures/arch/v1/recognize/
  kesit-binasi-zemin.dxf`, KentOS kodu olmadan, santimetreyle): duvarların dış hattı DUVAR'da (köşelerde ve T'lerde tek
  hat, kolonlarda duruyor, açıklıklarda pervazlı), 8 kolon KOLON'da taramalı, 3 kapı KAPI'da (kanat, yay, iki kasa),
  4 pencere PENCERE'de (iki yüz, iki cam, dış denizlik), aks, merdiven basamakları (28 cm arayla), oda adları ve bir
  yatak. DXF içe aktar + Planı tanı: **8 duvar ve 7 açıklık tanımla mikrometresine kadar aynı** (eksenler,
  kalınlıklar, açıklıkların yeri, genişliği, türü, kapıların menteşesi ve yanı); 88 kaynak silindi, 39 kaldı ve
  söylendi; tek geri alma adımı. Aynı çizimin IFC modelinde 8 duvar, 3 kapı, 4 pencere (Md. 9). Kolon adımından
  sonra: 8 kolon tanımla aynı (ortaları, 30/30, dolgulu), IFC'de 8 kolon; 104 kaynak silinir, 23 kalır (yalnız yok
  sayılan katmanlar).
- İlk koşu bir hata buldu: duvar katmanı yok sayılınca "adı bir şey söylemeyen katman duvardır" kuralı MERDIVEN'i duvar
  saydı ve basamaklar 5 duvar oldu. Merdiven, aks, mobilya gibi adlar artık "yok sayılır" rolündedir; yalnız adı bir
  şey söylemeyen katman duvar sayılır.
- 42 ortak durum (kolon adımıyla: yuvarlak, 30° dönük 0,50×0,30 ve taramalı kolon, kalan L çevre)
  `scripts/fixtures/arch_recognize_cases.py` (KentOS kodu olmadan) masaüstü işleyicisinde ve Python
  SDK'da geçiyor.
- **Bloklu ofis planı** (aynı üretici, `kesit-binasi-zemin-bloklu.dxf`: her kapı, pencere ve kolon kendi bloğu,
  duvarın doğrultusunda döndürülmüş, dış tarafa göre aynalanmış pencere, parçalar 0 katmanında; DXF içe aktar blokları
  blok olarak getirir): tanım yine mikrometresine kadar aynı (8 duvar, 7 açıklık, menteşe ve yanlar, 8 kolon); 56 duvar
  çizgisi ve 15 blok silinir, 23 nesne kalır. 43 ortak durumun biri bloklar (yarım dönüp aynalanmış pencere bloğu).
- Duvar ucundaki kapı: bir çekirdek testi (2,0 m'de biten 0,10 m bölme kuzey duvarın eksenine, 3,0 m'ye uzar, kapı
  2,00–2,90 m) ve bir ortak durum (45).
- Duvarın üstüne çizilmiş açıklık: 2 çekirdek testi (dört pencere çizgisi pencere, tek çizgi değil; kasalı kapı yayı
  1,00–1,90 m kapı, menteşe başta, sola) ve bir ortak durum (44).
- Üretici iki dosyayı her koşuda aynı yazar: ezdxf'in CLASSES sırası koşunun karma tohumuyla değişiyordu (adlarına
  göre sıralanır), sabit üst veri seçeneği de ilk belgeden önce açılır.
- Araç: 2 etkileşim testi (önizlemenin sözü, katman satırları ve rolleri, Enter'ın tek adımı ve geri alınması; Duvar
  rolü Yok say'a çekilince ret ve hiçbir şey yazılmaması).
- **Gerçek pencere** (1920×1080, Xvfb, gerçek fare ve klavye, release ikilisi; `.run/gui-evidence/mimari-m48/`): ofis
  planı açıldı (127 nesne), Ctrl+A, Mimari › Duvar ▾ › Planı tanı: önizleme 8 duvarı ve 7 açıklığı çizdi, imlecin yanında
  "8 duvar, 3 kapı, 4 pencere, 0 boşluk; 88 nesne silinir; 39 nesne kalır"; Araç ayarlarında Aks, Merdiven, MOBILYA,
  YAZI Yok say, DUVAR Duvar, KAPI Kapı, PENCERE Pencere, Kolon Kolon. Enter: DUVAR katmanı 56 çizgiden 8 duvara, KAPI 12
  parçadan 3 kapıya indi; seçilen güney duvarı 0,250 m, 12,000 m, Zemin kat. Aynı çizimde Mahal ▾ › Toplu mahal bul
  tanınan duvarlardan **5 mahal** buldu (5 yeni, 0 çakışan): köşe ve T birleşimleri odaları kapatıyor. Çizim 127
  nesneden 59'a indi (8 duvar, 7 açıklık, 5 mahal, 39 kalan).
- Masaüstünün simge testi M47'nin bir kaçağını buldu: Model kalite kontrol formu (EK-9) Mimari denetim'in simgesini
  taşıyordu (Mimari'de iki komut bir simgeyi paylaşamaz); forma kendi simgesi verildi (satırlarında onay işaretli bir
  form).

## 3. Açık kalanlar

- L ve düzensiz kolon (ADR 0725 §3'ün L kolonu), duvar stilinin ölçülen kalınlıktan seçilmesi (Chief Architect'in
  25 mm'si) ve bileşen çizgilerinden katmanlar.
- Eğri duvar (eş merkezli yaylar), blok niteliğindeki kapı numarasının açıklığa geçmesi, merdiven ve donatı.
- Gerçek bir ofis DWG'siyle tadilat örneği (ADV-25'in ilk kabulü).
