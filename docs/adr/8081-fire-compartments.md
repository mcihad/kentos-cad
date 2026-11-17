# ADR 8081: Yangın kompartımanları — ayrımlar, tehlikeli alanlar, Ek-4 alanı ve açıklıklar

- **Durum:** kısmen uygulandı (17 Kasım 2026). Plan: [bim-plan.md](../../plan/bim-plan.md) BIM-13 ("yangın
  kompartımanı": kompartıman sınırı ve üstündeki açıklık). Katlar arası açıklıklar, 24/4 ve kompartımanın IFC grubu kaldı
  (§6).
- **Bağlam:** ADR [8068](8068-fire-performance-material-declarations.md) (beyan edilen yangın sınıfları: nesnenin
  kendisi, duvar stili, kapı ve pencere tipi), [8035](8035-fire-escape-routes-distances-load-exits.md) (kaçış yolları),
  [8078](8078-regulation-rule-set-by-application-date.md) (kural seti, kapılar §8), bim-plan K10.
- **Dayanak:** Binaların Yangından Korunması Hakkında Yönetmelik 4/tt, 24/2, 24/6, 24/7, 26/1 ve Ek-4 (RG
  19.12.2007/26735; Ek-4 2009/15316 sayılı BKK ile değişik, RG 9.9.2009/27344).
- **Web:** dokunulmadı (ADR 0228). **`.kcad`:** değişmedi. **Sözleşme:** `kentos.auditFacts` 1'in `building` bölümüne
  isteğe bağlı `spaces`, `separations`, `sprinklers`.

## §1. Araştırma

Yönetmeliğin metni deponun derlenmiş kopyasından (`bykhy.pdf`, pdftotext ile; tablo PDF'in 94. sayfasından resim
olarak) okundu:

- **4/tt:** "Yangın kompartımanı: Bir bina içerisinde, tavan ve taban döşemesi dâhil olmak üzere, her yanı en az 60
  dakika yangına karşı dayanıklı yapı elemanları ile duman ve ısı geçirmez alanlara ayrılmış bölge."
- **24/2:** "… kazan dairesi, otopark, ana elektrik dağıtım odaları, yapı içindeki trafo merkezleri, orta gerilim
  merkezleri, jeneratör grubu odaları ve benzeri yangın tehlikesi olan kapalı alanların duvarları ve döşemeleri
  kompartıman duvarı özelliğinde olur."
- **24/6:** en fazla kompartıman alanı Ek-4'te. **24/7:** "kompartımanlar arasında yangına dayanıksız açıklıklar
  bulunamaz."
- **26/1:** "Bütün döşemelerin yangın duvarı niteliğinde olması gerekir."
- **Ek-4** (2009/15316, RG 9.9.2009/27344):

  | Kullanım | En fazla alan (m²) |
  |---|---|
  | Konutlar | sınırsız |
  | Konaklama | 4000 (1) |
  | Sağlık hizmeti | 1500 (1) |
  | Eğitim | 6000 (2) |
  | Büro | 8000 (1) |
  | Ticaret | 2000 (2) |
  | Yeme içme, eğlence, müze ve sergi (tek hücre) | 4000 (2) |
  | Diğer toplanma | 6000 (2) |
  | Endüstriyel, depo | tehlike sınıfına göre (Ek-1) |
  | Kapalı otopark | sınırlama yok |

  - Not (1): uygun yangın kontrol sistemi (otomatik algılama, yağmurlama, duman tahliye …) varsa alan iki katı.
  - Not (2): varsa sınırsız.
  - Not (3): tek katlıysa ya da sistem varsa sınırsız.
- **Okuma dersi:** pdftotext'in düz metni yeme içmeyi 2000 m²'nin satırına düşürüyordu; tablonun resmi üç kullanımın
  tek hücrede 4000 olduğunu gösterdi. Değer metinden değil tablodan alındı.
- **Ek-3/B:** kompartıman duvarı REI 60 ya da Ek-3/C (büyük olanı); Ek-3/C'nin yükseklik ve kullanıma göre süreleri
  sıradaki maddededir (beyan edilen dayanımın öbür elemanlarda karşılaştırılması).
- **Değişiklik tarihleri:** 2009/15316 RG 9.9.2009/27344, 2015/7401 RG 9.7.2015/29411. 24/2, 24/6 ve 24/7'de değişiklik
  işareti yok; Ek-4 2009'da değişti.

## §2. Karar

- **K74 — Ayrımlar ölçülür** (`Document::separations`, `crates/native/domain/src/separations.rs`):
  - Bir katın iki mahali arasında ortak sınır boyunca ne olduğu: duvarlar, açıklıklar (kapı, pencere, boşluk; niş ve
    baca duvarı delmez) ve ikisi de olmayan açık boy (mahal ayırıcı, boşluk).
  - Her mahalin eksen konturunda (çizilmiş mahalde kendi konturunda) 25 cm'de bir nokta alınır, köşelerin 30 cm'si
    dışarıda bırakılır. Noktadan dışarı 2 cm adımlarla en çok 1 m yürünür; ilk girilen başka mahal komşudur. Yürüyüşün
    ortasındaki eleman ayrımın o parçasıdır. İki nokta ayrıysa geçiş ikiye bölünerek 0,1 mm'ye bulunur ve açıklığın
    kasa noktaları halka olarak sıralanır: boylar kesindir ([ADR 8087](8087-shafts-elevators-roofs.md) §3).
  - Duvara kadar çizilen iki mahal birbirine değmez; yürüyüş duvarı geçer.
- **K75 — Kompartıman paketin kuralı** (`tr.kentos.mimari-mevzuat` 0.5.0, `kompartiman`; K10):
  - Beyanı okuyan `yangin_sinifi.ayirir`: E ve I harfleri, en az 60 dakika (4/tt: duman ve ısı geçirmez). E 60, EW 60,
    R 90 ayırmaz.
  - İki mahal, aralarındaki her duvar ve açıklık en az EI 60 ve açık boy yoksa ayrıdır; değilse aynı kompartımandadır.
  - Bütün döşemeler ayırır (26/1): kompartıman katın içindedir. Galeri, atrium ve açık merdivenin katları birleştirmesi
    okunmaz (§6).
- **K76 — 24/2:** adı kazan, otopark, garaj, elektrik, trafo, orta gerilim ya da jeneratör geçen mahalin başka bir
  mahalle her ayrımı kompartıman sınırı olmalı.
  - Yetmeyen sınıf ya da açık boy kaldı. Yalnız beyansız eleman eksik veri.
  - "Ana elektrik dağıtım odası" adla ayrılamadığı için elektrik geçen her mahal sayılır; idarenin teyidi beklenir.
- **K77 — 24/6, Ek-4:** kompartımanın alanı mahallerinin yönetmelik brüt alanları toplamıdır (4/1-ğ-1 konturu).
  - Sınıf binaların kullanımından okunur (`building.uses`: "Büro", "Otel" …). Kullanım yoksa ve bütün bölümler konutsa
    konut sayılır; yoksa eksik veri. Birden çok sınıfta en sıkı sınır.
  - Yağmurlama notları uygular. Sanayi ve depo tehlike sınıfına bağlı: uzman incelemesi.
- **K78 — 24/7:** duvarları en az EI 60 beyan edilmiş bir ayrımda beyansız açıklık eksik veri; yetmeyen açıklık ya da
  açık boy kaldı.
- **Kural seti:** 24/2 ve 24/7 19.12.2007, 24/6 9.9.2009 hâliyle. Daha eski gün uzman incelemesi (K61).

## §3. Uygulama

- **Belge:** `separations.rs` (`Separation`, `Document::separations`).
- **Olgular:** `SpaceFacts` (kimlik, kat, ad, yönetmelik brüt alanı), `SeparationFacts` (`walls`, `openings`:
  `SeparatingElement` kimlik, tür, beyan, boy; `open`), `sprinklers`. Sınıf `arch_performance::fire_rating_of`'tan
  (nesnenin kendisi, yoksa tipi). `factsSchema` ve `fixtures/audit-facts/v1/mimari.json` yeniden yazıldı (planın 17
  ayrımı: kapılar ve duvarlar doğru mahaller arasında, açık boy yok).
- **Paket:** `yangin_sinifi.py` (kapıların sınıf okuması buraya taşındı, `ayirir`), `kompartiman.py`;
  `data/surumler.json`'a BYKHY 24/2, 24/6 (Ek-4'ün sınıfları veri olarak), 24/7; manifestte `kompartiman` denetimi.

## §4. Sınamalar

- **Paket** (`fixtures/standards/v1/kompartiman.json`, 22 durum, metinden ve tablodan elle;
  `catalog/standards/tests/mimari-mevzuat/test_kompartiman.py`):
  - 24/6: konut sınırsız; büro 9200 m² (kalır), yağmurlamalı 16000; ikiye bölünmüş büro; ticaret 2500 (kalır),
    yağmurlamalı sınırsız; otel 5000 (kalır), yağmurlamalı 8000; lokanta 3500 (geçer: 4000); karma en sıkı; kullanım
    yok eksik veri; fabrika uzman incelemesi.
  - 24/2: kazan dairesi EI 30 duvar (kalır), beyansız kapı (eksik veri), REI 120 ve EI2 90-C5 (geçer); jeneratör
    odasına açık 2 m (kalır); orta gerilim beyansız (eksik veri).
  - 24/7: EI2 30-C kapı (kalır), beyansız pencere (eksik veri), E 60 duvar kompartıman duvarı değil.
  - Tarih: tarihsiz; 2008 başvurusunda Ek-4 uzman incelemesi.
- **Masaüstü** (`compartment_tests`, 20 × 10 m ofis katı, duvarlara kadar çizilmiş üç mahal, elle):
  - Olgular: ofisle arşiv arasında REI 90 duvar ve beyansız 1,00 m kapı (köşeler dışında 9,20 m), arşivle kazan dairesi
    arasında duvar.
  - Denetim: kazan dairesinin EI 30 duvarı 24/2 kaldı; kapı 24/7 eksik veri; tek kompartıman 200 m² 24/6 geçti.
  - REI 120 ve EI2 60-S200C5 beyan edilince ikisi de geçer.

## §5. Arayüz

- Yeni pencere yok. Bulgular Denetim'in Mevzuat grubunda, maddeler kural setinin satırlarında; bulgu mahalleri ve
  elemanları seçer.

## §6. Sıradaki adımlar

- **Katlar arası açıklıklar:** galeri, atrium (24/5) ve açık merdivenle birleşen katlar; 24/4 (yüksek binada en çok
  üç kat bir kompartıman): [ADR 8086](8086-compartments-across-storeys.md)'da yapıldı.
- **Kompartımanın IFC grubu** (bim-plan "Gruplar: … yangın kompartımanı"): `IfcZone`, EK-6'nın satırıyla.
- **Döşemelerin sınıfı** ve Ek-3/B, Ek-3/C'nin yüksekliğe ve kullanıma göre süreleri: sıradaki madde.
