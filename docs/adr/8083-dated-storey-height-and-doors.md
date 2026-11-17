# ADR 8083: Kat yüksekliği ve kapılar başvuru gününün hâliyle — PAİY 28/1-c, 39/1

- **Durum:** uygulandı (17 Kasım 2026). Plan: [bim-plan.md](../../plan/bim-plan.md) BIM-13 ("28/1-c ve 39/1'in tarihli
  hâlleri"); ADR [8078](8078-regulation-rule-set-by-application-date.md)'in sıradaki adımı ("Öbür tarihli maddeler:
  28/1-c (12.8.2023) ve 39/1 (13.8.2025) Rust'ta tek hâlle okunuyor; geçiş hükümleri okunmadan hâlleri ayrılmaz").
- **Bağlam:** ADR [8033](8033-regulation-rules-rooms-storeys-doors.md) (Rust'taki ilk kurallar), [8034](8034-regulation-stairs-shafts-elevators-local-rules.md)
  (Sivas katmanı, çelişki), [8078](8078-regulation-rule-set-by-application-date.md) (K60–K62: kural seti başvuru gününe
  göre, gün yoksa tahmin yok), bim-plan K10 (kural dokunuldukça Rust'tan pakete taşınır).
- **Dayanak:** Planlı Alanlar İmar Yönetmeliği 28/1-c ve 39/1 (RG 3.7.2017/30113; 30.9.2017/30196; 12.8.2023/32277;
  13.8.2025/32985); Sivas Belediyesi İmar Yönetmeliği 28/1-c ve 39/1 (RG 10.11.2022/32009).
- **Web:** dokunulmadı (ADR 0228). **`.kcad`:** değişmedi. **Sözleşme:** `kentos.auditFacts` 1'in `building`
  bölümüne isteğe bağlı `commons`; katlara `height`, `bedrooms`; kapılara `clear`, `height`, `sides`, `toilet`,
  `balcony`. `paiy_storey_height_over`, `paiy_door_height_below`, `paiy_door_width_below` kodları aynı kaldı; onları
  artık paket verir.

## §1. Araştırma (17 Kasım 2026)

Metinler WebFetch ile okundu (mevzuat.gov.tr ve Resmî Gazete bu bilgisayarın tarayıcısında sertifika hatası veriyor,
ADR 8078 §1): Lexpera'nın RG 3.7.2017/30113 metni, alomaliye'nin değişiklik yönetmelikleri, Resmî Gazete'nin eski
sayısı (Sivas) ve mevzuat.gov.tr'nin derlenmiş PAİY metni (değişiklik işaretleri için).

| Kaynak | Ne diyor | KentOS kararı |
|---|---|---|
| RG 3.7.2017/30113, 28/1 (Lexpera) | "Kat yükseklikleri uygulama imar planında daha fazla belirlenmemiş ise döşeme üst kotundan döşeme üst kotuna olmak üzere en fazla; … c) Konut bölgelerinde zemin ve normal katlarda 3.60 metre, … kabul edilerek uygulama yapılabilir." 71 inci madde: 1.10.2017'de yürürlüğe girer | 28/1-c'nin 1.10.2017 hâli: 3,60 m |
| RG 12.8.2023/32277 (alomaliye) | 2 nci madde: (c) bendine "metre," ibaresinden sonra "bodrum hariç 3 katı geçmeyen binalarda ise 4.00 metre," eklendi; (ç)'deki "asma katlı zemin katlarda 5.5 metre," kaldırıldı. 3 üncü madde Geçici 6'yı ekler (ofislerin konuta dönüşümü). 4 üncü madde: yayımı tarihinde yürürlüğe girer | 28/1-c'nin 12.8.2023 hâli; geçiş hükmü 28/1-c'yi saymaz |
| Derlenmiş PAİY (mevzuat.gov.tr) | 28/1-c'de yalnız "(Ek ibare:RG-12/8/2023-32277)"; 28/1-d (RG 9.10.2021/31623) konut bölgesinde geleneksel kesme taş yığma, zemin dahil 3 katı geçmeyen binada 4,50 m | 2017 ile 2023 arasında (c) değişmedi; (d) çizimde okunmaz (§6) |
| RG 3.7.2017/30113, 39/1 (Lexpera) | "Bütün yapılarda; a) Kapı yükseklikleri 2.10 metreden, b) Kapı net (temiz) genişlikleri bina giriş kapılarında ve yangın merdivenlerine açılan kapılarda 1.50 metreden, kapıların çift kanatlı olması halinde bir kanat 1.00 metreden, c) Bağımsız bölüm giriş kapılarında 1.00 metreden, diğer mahallerin kapılarında 0.90 metreden, az olamaz. Balkon ve tuvalet kapıları 0.80 metreye düşürülebilir." | Metin 2017'de de net (temiz) genişlik diyordu |
| RG 30.9.2017/30196, 7 nci madde (alomaliye) | (b) bendi: "bina giriş kapılarında 1.50 metreden, kapıların çift kanatlı olması halinde bir kanat 1.00 metreden," ("ve yangın merdivenlerine açılan kapılarda" çıktı). 15 inci madde: 14 üncü madde dışındakiler 1.10.2017'de yürürlüğe girer | 39/1'in 1.10.2017 hâli bu (b) ile; yangın merdiveni kapısının 1,50'si hiç yürürlüğe girmedi |
| RG 13.8.2025/32985 (alomaliye) | 6 ncı madde 39/1'i yeniden yazar: a) 2.10, b) 1.50 ve çift kanatta bir kanat 1.00, c) bağımsız bölüm girişinde 1.00, diğer mahallerin kapılarında 0.90 ("az olamaz" her bentte); ç) döner kapı; d) yaşlılar için bina giriş kapıları cepheden ayırt edici malzeme, renk ya da desenle belirgin. Balkon ve tuvaletin 0,80'i yok. 8 inci madde Geçici 3/9'u ekler: yalnız 19/1-f-5 ve 6 için. 9 uncu madde: yayımı tarihinde yürürlüğe girer | 39/1'in 13.8.2025 hâli; geçiş hükmü 39/1'i saymaz |
| Sivas İmar Yön. (RG 10.11.2022/32009) | 28/1-c: konut bölgelerinde zemin ve normal katlarda 3,60 m (PAİY'nin 2023'te eklediği 4,00 yok). 39/1: PAİY'nin 2017 hâliyle aynı, balkon ve tuvalet kapıları 0,80 m. Geçici 3: mevcut başvurularda PAİY'nin geçici 3 üncü maddesi | Yerel katman 10.11.2022'den, ADR 8034'teki gibi |

- **Önceki okumanın düzeltmesi:** ADR 8033 "13.8.2025 değişikliği kapı genişliklerini net temiz geçişe bağladı"
  diyordu; 2017 metni de "net (temiz)" der. Değişiklik balkon ve tuvalet kapılarının 0,80'ini kaldırdı ve (d)'yi ekledi.
- **Geçiş hükmü yok:** iki değişiklik de yayımı günü yürürlüğe girdi ve geçici maddeleri bu maddeleri saymıyor. KentOS
  kural setini K60'taki gibi başvuru gününe göre seçer: başvuru gününde yürürlükteki hâl okunur. İdarenin ruhsat
  gününde yürürlükteki hâli uygulaması ayrı bir yorumdur; bulgu maddenin hâlini ve gününü söyler.

## §2. Karar

- **K81 — İki madde pakete taşındı** (K10: dokunulan kural taşınır). Rust'ın Mevzuat grubundan 28/1-c'nin ve 39/1'in
  kuralı çıktı (`regulation_audit.rs`); kodlar aynı, paketin `kat_yuksekligi` ve `kapi_olculeri` kuralları verir.
  `regulation_audit::CODES` onları artık saymaz: paketin bulgusuna verilen istisna Rust'ın grubuna bayat görünmez.
- **K82 — Olgular ne çizildiğini söyler:**
  - Katın kattan kata yüksekliği: kendi yüksekliği, yoksa üstündeki katın kotuna kadar (Rust'ın okuduğu gibi); ikisi de
    yoksa yok.
  - Katın yatak odaları (`kind_of`); programın ortak alanları (`commons`).
  - Kapının net genişliği (`OpeningSizes::clear`), yüksekliği (üst kot eksi eşik), iki yanında ne olduğu (`sides`:
    bölümün, ortak alanın, eklentinin ya da dış alanın mahali, grupsuz mahal, boşluk, hiçbir şey) ve bir yanının
    tuvalet ya da balkon olup olmadığı. Sabit kapı okunmaz.
  - Yükseklik ve genişlik olguların 1 mm'siyle yuvarlanır (`LENGTH`): sınırda 0,5 mm'nin altındaki fark okunmaz.
- **K83 — 28/1-c'nin okuması** (`kat_yuksekligi.py`):
  - Bölge türü projede henüz yok: kural konut projesinde konut bölgesi sayar ve iletide yazar ("(konut bölgesi, 2
    kat)"). Konut projesi: programında konut nitelikli ya da niteliği yazılmamış bağımsız bölümü olan (29/1'in okuması
    gibi; ADR 8033 niteliği yazılmamışı saymıyordu), programsızsa yatak odası çizilmiş proje.
  - "Zemin ve normal katlar" ±0,00 ve üstündeki katlardır; bodrum okunmaz (Rust bodrumu da ölçüyordu). "Bodrum hariç 3
    katı geçmeyen": ±0,00 ve üstündeki kat sayısı.
  - Yüksekliği bilinmeyen kat eksik veridir (Rust onu sessizce atlıyordu).
- **K84 — 39/1'in okuması** (`kapi_olculeri.py`):
  - Yükseklik 2,10 m (a), her hâlde aynı.
  - Dışarı açılan kapı (bir yanı hiçbir şey ya da programın dış alanı): binada ortak alan ya da birden çok bölüm
    varsa ortak alanın ve grupsuz mahalin dış kapısı bina girişi (b, 1,50 m), bölümün mahalininki o bölümün girişi (c,
    1,00 m; Rust bunu da bina girişi okuyordu); eklentininki diğer mahal kapısı. Tek bölümlü, ortak alansız binada dış
    kapı bölümün girişidir.
  - Bölümün mahali ile ortak alanın mahali arasındaki kapı bölüm girişi (c, 1,00 m). Öbürleri diğer mahallerin kapısı
    (c, 0,90 m); bir yanı tuvalet ya da balkon olan kapı 1.10.2017 hâlinde ve Sivas'ta 0,80 m.
  - Balkonu mahal olarak çizilmemiş balkon kapısı dışarı açılır, bölüm girişi okunur: balkon mahalini çizmek okumayı
    düzeltir. Hiçbir yanında binanın mahali olmayan kapı okunmaz.
- **K85 — Gün yoksa parça parça** (`yururluk.parcalarla`, `birlesik`): K61 kuralın bütününe uygulanınca güne bağlı tek
  bir tuvalet kapısı bütün kapıların bulgusunu tek "eksik veri" iletisine saklardı. Her kat ve her kapı ayrı denenir:
  sonucu güne göre değişmeyenin bulgusu en yeni hâlle kalır, değişeninki o parçanın tek "eksik veri" bulgusudur ve onu
  seçer. Maddenin satırı parçaların birleşimidir: biri güne bağlıysa eksik veri, değilse biri kaldıysa kaldı.
- **K86 — İki katmanın ortak yargısı** (`katman.py`): Rust'ın `judge` ve `said`'inin Python'u (ADR 8034): yalnız yerele
  uymayan ölçüde yerel bağlar, ikisine de uymayanda sıkı olan, eşitse PAİY; yerele uyup PAİY'ye uymayan çelişki. İleti
  Rust'ınkiyle aynı biçimde ("… 0.86 m: Sivas 39/1-c en az 0.80 m ister, uyuyor; PAİY 39/1-c (13.8.2025) en az 0.90 m
  ister, uymuyor — çelişkide idare karar verir"). Maddenin bendi iletide ve `rule`'da (`Hal.kisa(bent)`).

## §3. Uygulama

- **Olgular:** `kentos_extensions::audit_facts::building` (`StoreyHolds.height`, `bedrooms`; `BuildingFacts.commons`;
  `DoorFacts.clear`, `height`, `sides`, `toilet`, `balcony`), `kentos_processing::audit_facts::building::measure`.
  `factsSchema` (SDK) ve `fixtures/audit-facts/v1/mimari.json` yeniden yazıldı.
- **Paket** (`tr.kentos.mimari-mevzuat` 0.7.0): `kat_yuksekligi.py`, `kapi_olculeri.py`, `katman.py`;
  `yururluk.parcalarla`, `birlesik`, `Hal.kisa(bent)`, `Hal.kural(bent)`; `data/surumler.json`'a PAİY 28/1-c (1.10.2017,
  12.8.2023), PAİY 39/1 (1.10.2017, 13.8.2025), Sivas 28/1-c ve 39/1 (10.11.2022); manifestte `kat-yuksekligi` ve
  `kapi-olculeri` denetimleri.
- **Rust:** `regulation_audit.rs`'ten kat yüksekliği ve kapı ölçüsü kuralları, maddeleri (`STOREY_ARTICLE`,
  `DOOR_*_ARTICLE`, Sivas'ınkiler) ve `Place`'in tuvalet ve balkon alanları çıktı; asansör sahanlığı (34/2) ve pencere
  korkuluğu (38/1) aynı döngüde kaldı. Komutun belgesi ve kataloğun özeti yeniden yazıldı (`python/kentos/cad` stub'ları
  da).

## §4. Sınamalar

- **Paket** (bağımsız durumlar, metinlerden elle):
  - `fixtures/standards/v1/kat-yuksekligi.json`, 16 durum: iki kat 3,80 m 2026'da geçer, 2022'de kalır; dört kat 3,60
    ister; bodrumlu üç kat 4,00'ı tutar; bodrumun yüksekliği okunmaz; tek kat 4,20 m tarihsiz eksik veri (istenen
    değişiyor), dört kat 3,80 tarihsiz en yeni hâlle; Sivas bağlar, 2021'de Sivas yok, Sivas tarihsiz; büro ve yatak
    odasız programsız çizim kapsam dışı; yüksekliği yazılmamış kat eksik veri; tam 3,60 geçer; 2016 başvurusu uzman
    incelemesi.
  - `fixtures/standards/v1/kapi-olculeri.json`, 25 durum: iç kapı 0,76 ve 0,90; bina girişi (ortak hol, grupsuz hol,
    bahçeye açılan); bölüm girişi (ortak hole açılan, müstakil ev, zemin kat dairesinin bahçe kapısı); eklentinin dış
    kapısı; tuvalet 0,86 m 2026'da kalır, 2024'te geçer, tarihsiz eksik veri; balkon 0,76 m 2024'te 0,80'e kalır; dış
    alandaki balkon; iç kapının tarihsiz bulgusu yanında tuvaletin eksik verisi; Sivas çelişkisi, 2024'te geçer, eşit
    sınır; yükseklik 2,00 m; üst kotsuz, mahalsiz ve boşluğa açılan kapı; kapısız bina; 2016 başvurusu.
  - `python3 -I catalog/standards/tests/mimari-mevzuat/test_kat_yuksekligi.py` ve `test_kapi_olculeri.py`; asansör
    sınamasının durumları yükseklik yazmadığı için kat kuralının bulgularını süzer.
- **Masaüstü:**
  - `dated_sizes_tests` (elle, iki katlı ev): olgular (3,80 m katlar, yatak odası, WC kapısının iki yanı bölüm, tuvalet,
    0,86 m; giriş 1,06 m); 1.8.2023'te iki kat 3,60'a kalır, 14.8.2023'te geçer, 14.8.2025'te WC kapısı 0,90'a kalır,
    tarihsiz iki kat ve kapı eksik veri, Sivas'la 3,60 bağlar ve kapı çelişki; maddelerin satırları ve hâlleri.
  - `regulation_audit_tests`: kabul dairesinin beş kapısı paketten (tarihsiz, iki hâl de 0,90 ve 1,00 istediği için
    en yeni hâlle); küçük konutun 4,20 m katı tarihsiz eksik veri, 2.7.2026'da 4,00'a kalır; dört katlı binanın WC kapısı
    tarihsiz eksik veri, Sivas'la 2.7.2026'da çelişki. `regulation_dates_tests`: kural setinin satırına "PAİY 28/1-c
    (12.8.2023) geçti" girdi, kapı yok, 10 madde kapsam dışı.
- **Resimler:** `.run/gui-evidence/kat-kapi-8083/` (`dated_sizes_tests::screens`): 1.8.2023 kat bulgusu, 14.8.2025 WC
  kapısı; açık ve koyu.

## §5. Arayüz

- Yeni pencere yok. Bulgular Denetim'in Mevzuat grubunda; maddeler kural setinin "PAİY 28/1-c", "PAİY 39/1" (Sivas
  seçiliyse "Sivas 28/1-c", "Sivas 39/1") satırlarında. Bulgu kat için katın mahallerini, kapı için kapıyı seçer.

## §6. Sıradaki adımlar

- **Bölge türü** (plan "bölge türü"): 28/1-a, b, ç (ticaret, karma, zemini ticaretli konut bölgesi) ve 28/1-c'nin konut
  bölgesi varsayımı projenin imar durumundan okunsun.
- **28/1-d:** geleneksel kesme taş yığma bina 4,50 m (taşıyıcı sistem beyanı modelde yok).
- **39/1-b'nin bir kanadı:** çift kanatlı bina girişinde bir kanadın 1,00 m'si (`OpeningSizes.clear_first` ilk kanadı
  ölçer; geniş kanadın tek başına net geçişi gerekir). 39/1-ç döner kapının yanındaki normal kapı; 39/1-d okunmaz.
- **`regulation_conflict`** hem Rust'ın grubunun hem paketin kodudur: paketin çelişkisine verilen istisna Rust'ın
  `stale` listesinde görünebilir (34/8'in çelişkisinde de vardı).
- 39/5 (asansörlü eşya taşıma için 3–10 katlı binada bir balkon kapısı 0,90 m) ayrı kural.
