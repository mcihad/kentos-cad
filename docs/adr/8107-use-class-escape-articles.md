# ADR 8107: Kullanım sınıfına özel kaçış hükümleri — hasta odası, otel ve tek merdivenli konut dışı bina (BYKHY 49, 50, 52)

- **Durum:** uygulandı (17 Kasım 2026), kısmen. Plan: [bim-plan.md](../../plan/bim-plan.md) B22 ("Konut dışı kullanımlar",
  BIM-13; yasal en geç 1.9.2033), ilk dilim. Her yeni kural için geçen ve kalan bağımsız durum, gerçek çizimde Denetim
  (§4); öbür kullanım sınıflarının hükümleri §6'da.
- **Bağlam:** ADR [8078](8078-regulation-rule-set-by-application-date.md) (kural seti başvuru gününe göre; §8 kaçış yolu
  kapıları, yangın sınıfının okunması), [8035](8035-fire-escape-routes-distances-load-exits.md) (kaçış uzaklıkları, Ek-5/A
  kişi yükü, çıkış genişliği ve sayısı kullanıma göre), [8081](8081-fire-compartments.md) (mahaller arası ayrımlar),
  [8068](8068-fire-performance-material-declarations.md) (beyan edilen yangın sınıfı), plan K10 (kurallar Python'da,
  olgular Rust'ta).
- **Web:** dokunulmadı (ADR 0228). **`.kcad`:** değişmedi. **Komut:** değişmedi (`cad.arch.audit`'in bulguları ve kural
  seti satırları çoğaldı). **Kural paketi:** mimari-mevzuat 0.22.0 (`kullanim`). **Olgular:** değişmedi.

## §1. Araştırma (17 Kasım 2026)

Binaların Yangından Korunması Hakkında Yönetmelik, mevzuat.gov.tr derlenmiş metni
(<https://www.mevzuat.gov.tr/MevzuatMetin/21.5.200712937.pdf>, 104 sayfa, SHA-256
`d82d06d2d66410c3a23e86c44605355fb1282c85506a56544611fbe64a89a7d9`: araştırma notu `2026-11-16-tr-mevzuat-denetim-kurallari.md`'nin
okuduğu aynı dosya; son değişiklik RG 1.7.2025/32943). Metin `pdftotext -layout` ile okundu; kural setine yazılan her
alıntı metinde aynen geçer (boşluklar tek boşluğa indirilerek denetlendi).

| Madde | Ne diyor (özet; aynen metni `surumler.json`'da) | KentOS kararı |
|---|---|---|
| 8, 10–18 | Kullanım sınıfları: konut, konaklama (otel, motel, pansiyon, öğrenci yurdu …), kurumsal (eğitim, sağlık: hastane, huzurevi, bakımevi …, cezaevi), büro, ticaret, endüstri, toplanma, depolama, yüksek tehlikeli, karışık. 12/2: başka binanın içindeki büro ana binanın sınıfındadır; 16/2: 50 m²'den küçük depolama bölümü esas binanın parçasıdır | Sınıf bina programının kullanımından; yoksa mahal adlarından, sıkı okumayla: yalnız konut bölümlü bina bunların hiçbiri değil, otelin bürosu ve küçük deposu binayı 52'nin binası yapmaz (K189) |
| 49/1-a (19.12.2007; değişmedi) | Sağlık yapılarında kullanıcı yükü 15 kişiyi aşan hasta yatak odası veya süit oda için birbirinden uzakta 2 kapı | Kişi Ek-5/A'nın Mimari denetimdeki hesabından; kapı sayısı ölçülür; "birbirinden uzakta" bir ölçü değil, satırın notunda idareye (K190) |
| 50/1-a (19.12.2007) | Yatak odaları iç koridordan en az 60 dakika yangına dayanıklı duvarla ayrılır; toplam yatak 20'den fazla veya kat sayısı ikiden fazla otelde her katta en az 2 çıkış; yatak 20'den az ve yapı yüksekliği 15,50 m'den az binada merdiven korunumlu ya da basınçlandırılmışsa tek merdiven yeter | Duvarın beyan edilen sınıfı okunur (beyan yoksa eksik veri); çıkışlar yatak odalı katlarda sayılır; yatak sayısı ve merdivenin korunumu çizimde yok (K191) |
| 50/1-b (19.12.2007) | İç koridora açılan kapılar en az 30 dakika yangına dayanıklı ve kendiliğinden kapatan düzenekli | Kapının beyan edilen sınıfı (TS EN 13501-2 yazımı: dakikası, `C`) (K192) |
| 52 (9.9.2009; 2009/15316 ile değişti) | Fabrika, imalathane, mağaza, dükkân, depo, büro ve ayakta tedavi merkezinde en az 2 bağımsız kaçış merdiveni veya başka çıkış; tek merdiven (a) yapı yüksekliği 21,50 m'den az, (b) bir kattaki kullanıcı 50'den az, (c) kaçış uzaklıkları Ek-5/B'ye uygun, (ç) yanmaz ürünler, (d) kolay alevlenici madde yok, hepsi birlikte tutunca yeter | (a) ve (b) ölçülür; tutmazsa kaldı, tutarsa (c)–(d) çizimden kanıtlanmaz: uzman incelemesi (K193) |
| 51 | Sabit koltuklu salonların sıra arası (Ek-6), ara dolaşım alanları | Koltuklar modelde yok: sonraki iş (§6) |

## §2. Karar

- **K188 — Kural ve olgular:** kurallar mimari-mevzuat'ın `kullanim` modülünde (0.22.0); Rust'ın olgularına
  (`facts.building`) yeni alan eklenmedi: binaların kullanımı, katlar, mahaller (adı, katı, kaçış merdiveni mi), Ek-5/A
  kişileri, kapılar (iki yanı, beyan edilen sınıfı), mahaller arası ayrımların duvarları ve sınıfları, yapı yüksekliği
  yetiyor. Maddelerin hâlleri `data/surumler.json`'da (metni, RG, yürürlüğü, anahtarları: 15 kişi, 60/30 dakika, 20
  yatak, 2 kat, 15,50 m, 21,50 m, 50 kişi).
- **K189 — Kullanım sınıfı:** bina programının kullanımı yazılmışsa ondan (hastane, huzurevi, bakımevi, klinik → sağlık;
  otel, motel, pansiyon, yurt, yatakhane → konaklama; fabrika, imalathane, mağaza, dükkân, depo, büro, ofis, poliklinik,
  dispanser, ayakta tedavi → 52'nin binası). Yazılmamışsa mahal adlarından: bütün bölümleri konut olan bina hiçbiri;
  hasta ya da koğuş adı sağlık, otel, yurt ya da yatakhane adı konaklama; ikisi de yoksa mağaza, dükkân, büro, ofis,
  fabrika, imalathane, ayakta tedavi adı 52'nin binası; depo adı tek başına saymaz. Sınıfı tutmayan madde kapsam dışıdır
  ve satır nedenini söyler ("bina programının kullanımı (“Konut”) sağlık yapısı değil").
- **K190 — 49/1-a:** sağlık yapısında adı Hasta, Koğuş, Süit ya da Yatak odası olan ve kişisi 15'i aşan mahalin kapı
  sayısı 2'den azsa kaldı (`bykhy_patient_room_doors`); kapıların birbirinden uzaklığı idarenin takdiridir, satır söyler.
- **K191 — 50/1-a:** konaklama yapısında yatak odası (Oda, Otel odası, Yatak odası, Süit, Misafir ya da Konuk odası,
  Yatakhane) ile koridor (Koridor, Hol) arasındaki her duvarın beyan edilen sınıfı en az 60 dakika; yetmezse kaldı,
  yoksa eksik veri (`bykhy_hotel_room_wall`). Yatak odalı her katın çıkışları (katın kaçış merdivenleri, zemin katta
  dışarı açılan kapılar da) 2'den azsa: kat sayısı ikiden fazla ve yapı yüksekliği 15,50 m ve üstündeyse kaldı (istisna
  uygulanamaz); öbür hâllerde yatak sayısına ve merdivenin korunumuna bağlı olduğundan eksik veri
  (`bykhy_hotel_storey_exits`).
- **K192 — 50/1-b:** yatak odasıyla koridor arasındaki kapının beyan edilen sınıfı en az 30 dakika ve kendiliğinden
  kapanır (`C`); yetmezse kaldı, yoksa eksik veri (`bykhy_hotel_room_door`).
- **K193 — 52:** 52'nin binasında zemin kattan başka, mahalli her katın kaçış merdiveni 2'den azsa: (a) yapı yüksekliği
  21,50 m ya da (b) herhangi bir katın kişisi 50 tutmazsa kaldı (iletide hangisi); ikisi de tutarsa (c) kaçış
  uzaklıkları, (ç) yanmaz ürünler ve (d) kullanılan maddeler çizimden kanıtlanmaz: uzman incelemesi
  (`bykhy_single_stair`). Tek katlı binada madde kapsam dışı: çıkışları 39/2 ve 32/8 söyler. 9.9.2009'dan önceki başvuru
  için metin kural setinde yok: satır uzman incelemesi.

## §3. Uygulama

- `catalog/standards/mimari-mevzuat/src/mimari_mevzuat/kullanim.py`; `mimari.py`'nin kural sırası, `denetim.py`'nin
  eklenti denetimi yüzü (`kullanim`), `manifest.json`, `data/surumler.json` (dört madde), 0.22.0, CHANGELOG.
- `cad.arch.audit`'in açıklaması yeni kodları sayar (katalog, Python SDK).

## §4. Sınamalar

- **Bağımsız durumlar** (`fixtures/standards/v1/kullanim.json`, 25 durum; `scripts/fixtures/kullanim_cases.py` olguları kurar,
  beklenenler metinden elle): iki merdivenli büro geçer; 28 m'lik tek merdivenli büro (a)'dan, 60 kişilik kat (b)'den kalır;
  koşulları tutan tek merdiven uzman incelemesi; tek katlı mağaza kapsam dışı; programdan depo ve ayakta tedavi merkezi;
  programsız ofis adları büro; konut binasının Depo'su binayı depo yapmaz; 1.6.2009 başvurusu 52'yi bilmez; otelin EI 60
  duvarı ve EI2 30-C5 kapısı geçer, EI 30 duvar, kapanmayan ve 15 dakikalık kapı kalır, beyansız eksik veri; 18 m'lik altı
  katlı tek merdivenli otel kalır, 12 m'lik dört katlı ve iki katlı olanlar eksik veri; öğrenci yurdu; 18 kişilik tek
  kapılı hasta odası kalır, iki kapılı geçer, iki kişilik kapsam dışı; konut binası dört maddede kapsam dışı; tarihsiz
  başvuru en yeni hâlle. `test_kullanim.py` ayrıca sınıfın ve oda adlarının okunmasını sınar; paketin öbür 11 sınaması geçti.
- **Gerçek çizimde Denetim** (`apps/desktop/src/tests/use_class_tests.rs`): komutlarla çizilen üç katlı büro (Ofis 12 × 10
  m, 12 kişi): katlarda tek merdivenle 52 uzman incelemesi (iki bulgu), iki merdivenle geçti. İki katlı otelin 1. katında
  Oda 101, Koridor ve iki merdiven 20 cm duvarlarla, odanın kapısı koridor duvarında: beyansızken 50/1-a ve 50/1-b eksik
  veri; duvar EI 30 iken 50/1-a kaldı ("30 dakika < 60 dakika"); EI 60 ve kapı EI2 30-C5 ile ikisi de geçti. Konut
  binasının kural seti satırı "23 madde kapsam dışı" oldu (`regulation_dates_tests`).
- **Resimler:** `.run/gui-evidence/kullanim-sinifi-8107/` (büronun uzman incelemesi, otelin kalan duvarı; açık ve koyu,
  yakınlaştırılmış).

## §5. Arayüz

Değişmedi: Denetim'in Mevzuat grubunda yeni bulgular, kural seti satırında dört yeni madde.

## §6. Sıradaki adımlar

- **49/1-b ve 49/2:** hastanede 300 m²'yi aşan yatılı katın yatay tahliye kompartımanları (2,8 m²/kişi) ve 2 m'lik koridor
  (koridorun eni olgularda yok: mahalin dar kenarı eklenmeli).
- **50/3 ve 50/6:** otel odasında en uzak noktadan kapıya 15 m (yağmurlamada 20), duman kesiciler (30/45 m).
- **51 ve Ek-6:** sabit koltuklu salonlar (koltuk sırası modeli gerekir).
- **Öbür sınıflar:** eğitim, toplanma (yeme içme, eğlence, ibadet), endüstri ve depolamanın tehlike sınıfı (Ek-1),
  otopark (60), karışık kullanımın 18'i (sıkı olan bütün binaya); yatak sayısının beyanı (otel).
- **Gerçek pencerede kabul** (ADR 8000).
