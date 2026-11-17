# ADR 8110: Sağlık yapıları — büyük yatılan katın kompartımanları (BYKHY 49/1-b), hastane koridoru (49/2) ve koridorun en dar yeri

- **Durum:** uygulandı (18 Kasım 2026). Plan: [bim-plan.md](../../plan/bim-plan.md) B22 ("Konut dışı kullanımlar", BIM-13),
  üçüncü dilim. Her madde için geçen, kalan, eksik veri ve uzman incelemesi durumları bağımsız durumlarla; koridorun en
  dar yeri bağımsız bir geometri referansıyla; gerçek çizimde Denetim (§5).
- **Bağlam:** ADR [8107](8107-use-class-escape-articles.md) (kullanım sınıfının okunması, 49/1-a), [8081](8081-fire-compartments.md)
  ve [8086](8086-compartments-across-storeys.md) (kompartımanlar ve katlar arası açıklıklar), [8033](8033-regulation-rules-rooms-storeys-doors.md)
  (dar kenar: net halkanın içindeki en büyük daire; "L biçimli koridorda iç daire köşede biraz büyür, ilk sürümde böyle
  kabul edildi"), [8108](8108-sprinklers-required-by-use.md) (alanlar dış konturdan), plan K10.
- **Web:** dokunulmadı (ADR 0228). **`.kcad`:** değişmedi. **Komut:** değişmedi (`cad.arch.audit`'in açıklaması iki yeni
  kodu sayar). **Kural paketi:** mimari-mevzuat 0.24.0 (`saglik`). **Olgular:** bina olgularının mahallerine `least`
  (hol ve koridorun en dar yeri). **Ortak çekirdek:** `geom::neck` (yeni hesap).

## §1. Araştırma (17–18 Kasım 2026)

### Yönetmelik

Binaların Yangından Korunması Hakkında Yönetmelik, ADR 8107 §1'deki aynı derlenmiş metin (mevzuat.gov.tr, SHA-256
`d82d06d2…a7d9`), 49 uncu madde. Kural setine yazılan metin `pdftotext` çıktısında aynen geçer (boşluklar tek boşluğa
indirilerek denetlendi).

| Bent | Ne diyor | KentOS kararı |
|---|---|---|
| 49/1 (giriş) | Sağlık yapıları kapsamında olan hastanelerde, yaşlılar için dinlenme ve bakım evlerinde, bedensel ve zihinsel engelliler için bakım evlerinde | Sağlık sınıfı ADR 8107'nin okumasıyla (hastane, huzurevi, bakımevi, klinik, rehabilitasyon; programsızda hasta ya da koğuş adlı mahal) |
| 49/1-b (değişik 10.8.2009, 2009/15316; RG 9.9.2009/27344) | Hastanelerin ve bakımevlerinin 300 m²'den büyük yatılan katlarının her biri en az yarısı büyüklüğünde iki ya da daha fazla yangın kompartımanına ayrılır ya da korunumlu yatay tahliye alanları kurulur; yatay tahliye alanında kullanıcı yükü 2,8 m²/kişi | Ölçülen: katın alanı, yatılan mahal, kompartıman sayısı. "En az yarısı büyüklüğünde" tek anlamla okunmuyor (her birinin katın yarısı olması ikiden çok kompartımanla olanaksız); 2,8 m²/kişi hangi kullanıcıların sığınacağına bağlı. Önceki metin kural setinde yok (§2) |
| 49/2 (ilk metin) | Hastanelerde koridor genişlikleri 2 m'den az olamaz | Hastane; adı Koridor olan mahal; genişliği en dar yeri (§1 Geometri) |
| 4/s | Kademeli yatay tahliye: kullanıcıların yangından uzaklaşarak aynı kattaki yangın geçirimsiz komşu kompartımana sığınması | Yatay tahliye alanı kompartıman gibi okunur |
| 4/tt | Yangın kompartımanı: her yanı en az 60 dakika yangına dayanıklı yapı elemanlarıyla duman ve ısı geçirmez ayrılmış bölge | Kompartıman sınırı ADR 8081'in (24/7'nin hâlinin süresi, 60 dakika) |
| 35/7 | 60 dakika dayanıklı ve duman geçişi önlenmiş yatay tahliye alanı sağlanan hastane gibi yerlerde kaçış uzaklığı o alana götüren koridorun çıkış kapısına kadardır | Yatay tahliye alanı çizimde EI 60 ile ayrılmış bir mahaldir: kompartıman sayılır |

İkincil kaynak: Türk mevzuatını hastane yangın güvenliği için yabancı kurallarla karşılaştıran bir DergiPark makalesi 49/1-b'yi
"en az yarısı büyüklüğünde iki veya daha fazla yangın kompartımanı" diye aynen aktarıyor, yorumlamıyor. 2,8 m²/kişi değeri
yabancı kuralların yatağa bağlı hasta başına 30 ft² sığınma alanıyla örtüşüyor; KentOS bu yorumu kurala dayanak yapmadı.

### Geometri: koridorun genişliği nasıl ölçülür

- **Bugünkü ölçü (ADR 8033):** dar kenar, net halkanın içindeki en büyük dairenin çapı. Bir odanın dar kenarı için doğru;
  koridor için değil: en büyük daire koridorun **en geniş** yerini ölçer. L biçimli koridorda daire köşede büyür (kolun
  %17 fazlası: 1,80 m'lik kol 2,11 m okunur), T'de %25, artıda %41; holle birleşen 1,20 m'lik koridor holün genişliğini
  okur. 49/2'nin 2,00 m sınırında bu yanlış "geçti" demek.
- **Model denetim araçlarının yaklaşımı:** Solibri'nin "247 Local Accessible Circulation Rule" yardım sayfası
  (<https://help.solibri.com/hc/en-us/articles/36284303842583>, 18 Kasım 2026 okundu) dolaşım alanının istenen en az
  genişliği taşıyıp taşımadığını, bu genişlikte dolaşılabilen alanı bularak denetler; dar köşe ve çıkıntıların payı için
  dolaşım alanının %90'ının erişilebilir olmasını ve erişilebilir alanın bölünmemesini arar (dar bir yer alanı ikiye bölerse
  kalır). %90 aracın kendi değeri; KentOS değer uydurmaz.
- **KentOS kararı, eşiksiz tanım:** koridorun en dar yeri, kesitin iki yanında da kendi uzunluğu kadar geniş bir daire
  kalan en kısa kesittir (iki ucu aynı halkada). Böyle kesit yoksa mahalin en büyük dairesi. Kapı girintisi (1,00 × 0,15 m)
  ağzından sığ olduğu için boğaz değildir; ağzı kadar derin niş (1,00 × 1,20 m) bir koldur. L, T ve artının en dar yeri
  kollarıdır; holle birleşen koridorunki koridorun kendisi; iki geniş ucu arasında daralan koridorunki belidir.
- **Görmediği (yazıldı):** çıkmaz uca doğru daralan koridorun dar ucu (her kesitin ötesi kesitten dar, köşe gibi) ve
  serbest duran kolonun yanındaki aralık (deliğe varan kesit mahali ikiye bölmez).

## §2. Karar

- **K200 — En dar yer ortak çekirdekte:** `kentos-geometry-core`'da `geom::neck::least_width(rings, precision)`. Kesitler
  her kenardan 1 cm arayla dik olarak ve her içbükey köşeden (iki kenarına dik, her kenarın en yakın noktasına, her
  içbükey köşeye) denenir, ilk değdiği halkaya kadar; kısa olandan başlanır, eşit uzunluklarda en büyük daireye yakın
  olan önce. Bir yanın dairesi "şu genişlikte daire sığar mı" diye erken çıkışlı bir aramayla sorulur (en büyük daireyi
  bulmak uzun koridorda bütün orta çizgiyi dolaşır); eğik duvarda kesitlerin arası yarılanarak bulunur. Halkalar ilk
  noktalarına taşınır: projeksiyon düzleminde uzaktaki oda milimetresini korur. Alan katmanında `room_width::least_of`
  (mahalin net halkası, yayları kirişleriyle; parçalarının en darı).
- **K201 — Olgu:** bina olgularının mahallerinde `least` (m, milimetreye): yalnız hol ve koridor (alanın adla ya da
  kullanımla okuması, `kind_of` geçiş); öbür mahallerin dar kenarı mimari olguların `narrow`'u kalır. Sınırı bulunamayan
  mahalde yok. Mimari denetimin PAİY 29/3 ve BYKHY 33/1 koridor kuralları bu işte değişmedi (§6).
- **K202 — 49/1-b:** sağlık yapısının yatılan katı (hasta odası, koğuş, süit, yatak odası ya da bakımevinin odası adlı
  mahal; koridor, hol, ıslak hacim ve bekleme değil) alanı (katın mahallerinin alanlarının toplamı, dış kontur) 300 m²'yi
  aşıyorsa: katın kompartımanları ADR 8081'in kuralıyla (en az 24/7'nin süresi beyan edilmiş, açık yeri olmayan ayrımlar;
  katlar arası açıklıklar birleştirir), yalnız kaçış merdiveni mahallerinden oluşanı (merdivenin yuvası) sayılmaz. Tek
  kompartımansa kaldı (`bykhy_patient_floor_compartments`); iki ya da daha çoksa bulgu kompartımanların o kattaki
  alanlarıyla uzman incelemesi (büyüklük şartı ve 2,8 m²/kişi idarenin). 9.9.2009'dan önceki başvuruda metin kural setinde
  yok: satır uzman incelemesi.
- **K203 — 49/2:** hastane bina programının kullanımından (Hastane), kullanım yazılmamışsa hasta odası ya da koğuş adlı
  mahalden; huzurevi, bakımevi, rehabilitasyon ve klinik hastane değildir. Adı Koridor olan her mahalin `least`'i 2,00
  m'den azsa kaldı (`bykhy_hospital_corridor`; ileti milimetresi varsa milimetreye yazar: 1,999 m 2,00 diye görünmez);
  ölçülemediyse eksik veri. Hol adı koridor değildir.

## §3. Uygulama

- Çekirdek: `crates/shared/geometry-core/src/geom/neck.rs` (`least_width`, `LeastWidth`), birim sınamaları.
- Alan: `crates/native/domain/src/room_width.rs` (`least_of`, halkaların ortak tessellasyonu `rings_of`).
- Olgular: `crates/shared/extensions/src/audit_facts/building.rs` (`SpaceFacts.least`, yuvarlama), `crates/native/processing/
  src/audit_facts/building.rs` (hol ve koridorlar), SDK şeması `kentos-audit-facts.schema.json`; mimari olguların sınama
  dosyası `fixtures/audit-facts/v1/mimari.json` (planın iki koridoru `least: 1.4`).
- Kural paketi: `catalog/standards/mimari-mevzuat/src/mimari_mevzuat/saglik.py`, kural sırası, Denetim yüzü `saglik`
  (kullanım sınıfı yüzü artık yalnız `bykhy_patient_room_*`), `manifest.json`, `data/surumler.json` (49/1-b, 49/2),
  0.24.0, CHANGELOG. `cad.arch.audit`'in açıklaması ve sözleşme yorumu iki kodu sayar.

## §4. Sınamalar

- **Geometri, bağımsız referans** (`fixtures/geometry/v1/least-width.json`, 24 durum; `scripts/fixtures/least_width_cases.py`:
  Shapely 2.2.0 ve GEOS ile kaba kuvvet, KentOS kodu yok — kenarlardan 2 mm arayla dik, her noktadan çeyrek derecede bir
  ışın, `split` ve `maximum_inscribed_circle`): koridor, kısa oda, kare, L (eşit ve eşit olmayan kollar), T, artı, kapı
  girintisi, derin niş, çıkıntı, bel, çıkmaza daralan, holle koridor, çapraz köşeler, iki hol arasında yay koridor,
  uzaktaki L, saat yönünde T, döndürülmüş koridor, pahlı L, Z, iki girintili ve hollü koridor, eğik bel, serbest kolon, U
  ayağın cebi. Hepsi referansla aynı (boğazda kesin, dairede iki kat hassasiyet içinde); hata ayıklama derlemesinde 24 durum
  toplam 0,16 s (koridor başına 2–40 ms).
- **Birim sınamaları** (`geom::neck`, 9): uzun koridor, kısa oda, L/T/artı, girinti ve niş, çıkıntı, bel, çıkmaza
  daralan (görmediği yer, yazılı), uzak koordinat, ters yön.
- **Kural, bağımsız durumlar** (`fixtures/standards/v1/saglik.json`, 25 durum; `scripts/fixtures/saglik_cases.py`): tek
  kompartımanlı 360 m² kat kalır; ikiye bölünmüş uzman incelemesi; 280 ve tam 300 m² kapsam dışı; yatılmayan kat kapsam
  dışı; huzurevi kalır, 49/2'si kapsam dışı; merdiven yuvası sayılmaz; 12 m² elektrik odası ve yatay tahliye alanı ikinci
  kompartıman (uzman incelemesi, alanlarıyla); konut ve otel kapsam dışı; programsız binada adlardan; 1.6.2008 başvurusu
  49/1-b uzman incelemesi; tarihsiz en yeni hâl; iki kat (biri kalır, biri uzman); döşeme boşluğu iki yarıyı birleştirir;
  1,80 m koridor kalır, tam 2,00 geçer, 1,999 kalır ("1,999 m"); ölçülemeyen eksik veri; Hol kapsam dışı; bakımevi
  koridoru kapsam dışı; "Hasta koridoru" kalır; klinik; hastane ve konut karma.
- **Gerçek çizimde Denetim** (`hospital_tests::an_l_shaped_corridor_is_as_wide_as_its_arms_not_its_corner`): komutlarla
  çizilen hastanenin 1. katı, kolları 1,80 m L koridor ve dokuz hasta odası (329 m²): olgunun `least`'i 1,80, en büyük
  daire 2,11 m; Denetim'de 49/1-b ve 49/2 kaldı; kollar 2,20 m iken 49/2 geçti. Konut binasının kural seti satırı "26
  madde kapsam dışı" (`regulation_dates_tests`).
- **Resimler** (`.run/gui-evidence/saglik-8110/`): dar koridor ve tek kompartıman bulguları (açık ve koyu), eski ve yeni
  ölçünün şekli.

## §5. Arayüz

Değişmedi: Denetim'in Mevzuat grubunda iki yeni bulgu, kural setinde iki yeni madde, Eklenti denetiminde yeni yüz (Sağlık
yapılarının yatılan katları ve hastane koridorları).

## §6. Sıradaki adımlar

- Mimari denetimin koridor kuralları (PAİY 29/3 1,20 m, BYKHY 33/1 1,10 m ve 33/2 1,20 m) en dar yere geçti: ADR
  [8112](8112-corridor-rules-by-narrowest-neck.md).
- Çıkmaza daralan koridorun dar ucu ve serbest kolonun yanındaki aralık.
- Yatay tahliye alanının beyanı ve 2,8 m²/kişi kapasitesi (hangi kompartımanın kullanıcıları sığınır).
- B22'nin kalanı: 50/3, 50/6, 51 ve Ek-6, eğitim, toplanma, endüstri ve depolama, 60 otopark, 18 karma kullanım.
