# 3038 — Mahal listesi ve grafik doğrama listesi

Tarih: 2026-11-09. Durum: kabul, 1. bölüm (M38; ADV-24). 2. bölüm grafik doğrama listesidir; bu ADR'ye eklenir.

## Sorun

- **Mahal listesi yoktu.** Planlı Alanlar İmar Yönetmeliği 57(29) her mekânın kendi koduyla en az tavan, döşeme ve
  duvar kaplamalarını, kapı-pencere kasa ve kanatlarını, süpürgeliğini, merdiven basamak ve rıhtlarını, küpeşte ve
  korkuluklarını, denizlik ve parapetlerini isteyen bir mahal listesinin projenin ekinde bulunmasını şart koşuyor.
  KentOS'ta yalnız "Mahal kaplama çizelgesi" vardı: döşeme, süpürgelik, duvar, tavan. Denizlik, merdiven ve korkuluk
  için mahalde alan yoktu; mahale hizmet eden kapılar ve pencereler listede görünmüyordu.
- **Süpürgelik Öznitelikler'de yazılamıyordu.** Alan vardı (ADR 0733 §3) ama yalnız çizelgenin hücresinden
  yazılıyordu; `cad.spaces.edit` onu almıyordu.
- **Yanlış sözcük:** Öznitelikler "Zemin kaplaması" diyordu; yönetmelik, Mimarlar Odası şartnamesi ve KentOS'un
  çizelgeleri "Döşeme" diyor. "Zemin" yapının oturduğu toprak ya da zemin kattır.
- **Kat sütunu alfabetik sıralanıyordu:** kata göre sıralanan bir çizelgede "1. kat" "Zemin kat"ın önüne düşüyordu.
- **Takımda mahal listesi paftası yoktu** (ADR 3037'nin "Kalanlar"ı).

## Araştırma (9 Kasım 2026)

### Türkiye mevzuatı

- **Planlı Alanlar İmar Yönetmeliği 57(29)** (Ek: RG 25.2.2022/31761): mahal listesi mekân kodlamasına göredir; en az
  tavan, döşeme, duvar kaplamaları, kapı-pencere kasa ve kanatları, süpürgelik, merdiven basamak ve rıhtları,
  küpeşte ve korkuluklar, denizlik ve parapetler; musluk, batarya, lavabo, duy, anahtar, armatür gibi sıva ve
  kaplama üzeri malzemeler ise "ilgisine göre mimari, elektrik ve mekanik tesisat proje müellifleri" tarafından.
  Kaplama bilgisi malzemenin özelliklerini mevzuata ve standartlara uygun olarak söyler.
- **57(29)'un 59(1) ile ilişkisi:** taşıyıcı sistemi ve yangın güvenliğini etkilemeyen mahal listesi değişikliği
  ruhsat gerektirmez; liste yaşayan bir belgedir.
- **Mimarlar Odası Mimarlık Hizmetleri Şartnamesi 14.8:** dış ve iç duvar, döşeme, tavan kaplamaları, asma tavanlar,
  kapılar, pencereler, kasalar, süpürgelikler, denizlik ve harpuştalar, çatı kaplaması; "ilgili yönetmelik, Türk
  Standartları Enstitüsü ve diğer standart ve norm poz noları yazılır."
- **Dijital proje yönetmeliği (RG 5.8.2026/33331) EK-2:** Tablo 2.12 Liste = **LS**; Tablo 2.13 "tüm disiplinlerde
  olabilecek dosyalar" Mahal Listesi = **MHL**, Malzeme Listesi = MLZ; Tablo 2.14 kattan bağımsız = **NNN**. Dosya adı
  `…_LS_MHL_NNN_01_000`. 4.18.2: mimari projede pencere, kapı ve tefriş blok olarak gösterilir; Tablo 4.4 blok adı
  disiplin (2) + öğe (4) + malzeme/tip (≤ 15) + ebat (≤ 10) (2. bölümün doğrama görünüşleri için).
- **TS 825:2024** (Tebliğ MHGM-2025/1, RG 20.2.2025/32819, yürürlük 1.4.2025): pencere Up ≤ 1,8 W/m²K, bütün iklim
  bölgelerinde; Sivas merkez 5. bölge (İZODER il sayfası). 2. bölümün U değeri sütunu ve denetimi bunu okur.

### Başka programlar (yalnız görünüş için)

- Revit'in oda çizelgesi oda alanlarını ve "From/To Room" ile kapıları okur; KentOS M28'de bu kuralın Türk
  uygulamasına uyarlanmış hâlini (geçiş mahalleri) kurdu ve aynısını burada kullanır.

## Karar

### 1. Mahalin kaplamaları

- `SpaceFinishes` üç alan aldı (KCAD şema 216, yalnız kullanılınca): `sill` (Denizlik ve parapet), `stair` (Merdiven
  basamak ve rıht), `railing` (Küpeşte ve korkuluk). Metin; mahalin kendi alanı, çizelgenin hücresinden de yazılır.
- `cad.spaces.edit`'in `set`'i `base`, `sill`, `stair`, `railing` alır (null kaldırır).
- **Öznitelikler:** Döşeme kaplaması (eski "Zemin kaplaması"), Süpürgelik, Duvar kaplaması, Tavan kaplaması,
  Denizlik ve parapet, Basamak ve rıht, Küpeşte ve korkuluk.

### 2. Mahalin kapıları ve pencereleri

- `kentos_domain::room_openings`: her kapı ve pencere hizmet ettiği mahale, Doğrama kodları'nın kuralıyla (ADR 3028):
  oda ile geçiş mahali (koridor, hol, antre, giriş, sahanlık, merdiven, geçit) arasındaki kapı odanın; iki oda
  arasındaki açıldığı odanın; tek yanında mahal olan onun. Pencere iki yanından birindeki mahalin, önce sol.
- Mahal satırında doğrama kodu (K01), kodu olmayan tipte ölçü (90/210); birden çoksa sayısıyla: "K01 (2), K02".
- Kural tek yerde: `opening_codes`'un `sides` ve `passage`'ı belgeden okur; "Mahale göre numara" ile liste aynı
  sonucu verir.
- Boşluk, niş, baca ve tasarım seçeneklerinin gösterilmeyenleri sayılmaz.

### 3. Mahal listesi çizelgesi

- Yeni çizelge alanları (`ScheduleField`, şema 216): `sillFinish`, `stairFinish`, `railingFinish` (düzenlenir),
  `doors`, `windows` (hesaplanır).
- **Çizelge ▾ › Mahal listesi** (`tool.scheduleRoomList`): Kat (başlık satırıyla), No, Ad, Döşeme, Süpürgelik, Duvar
  kaplaması, Tavan, Kapı, Pencere, Denizlik ve parapet, Basamak ve rıht, Küpeşte ve korkuluk; toplam satırı yok.
- Menüde altı satır sınırı var (DESIGN.md §7.3.1). Mahal listesi menüde Mahal kaplama çizelgesinin yerini aldı:
  onun bütün kaplama sütunları listede var. Kaplama çizelgesi Çizelge aracının Tür seçiminde ve komut aramada durur.
- **Kat sırası:** çizelgede Kat sütununa göre sıralama katların kotuna göredir (Bodrum, Zemin, 1. kat), adına göre
  değil. Bu bütün çizelgeler için geçerli; ortak örneklerin başvurusu (`schedule_extras_cases.py`) aynı kuralı yazar.
- **Liste çizimi izler:** mahal ve açıklık değişince olduğu gibi, kapı-pencere tiplerinin değişmesinde de (adı, kodu)
  liste aynı adımda yenilenir. Kapı ve pencere çizelgesi de tip değişikliğinde artık yenilenir (önceden tipin adı ya da
  kodu değişince eski kalıyordu).

### 4. Ruhsat takımında mahal listesi paftası

- `calc_tables::ensure` çizimin mahal listesini bulur, yoksa öbür tabloların yanına kurar (`CalcTable.list` = MHL).
- Takımda planlardan sonra, hesap raporundan önce (MO sırası: mahal listesi, otopark hesabı): **"Mahal listesi"**
  paftası (`Page::list`; EK-2 LS/MHL/NNN). Kendi kâğıdı: listenin sığdığı en küçük A kâğıdı; takımın öbür paftaları
  kendi kâğıdında kalır. Listede ölçek yoktur: antette "—", tablonun üstünde ölçek yazısı yok.
- Hiçbir kâğıda (A0) sığmayan liste kurulmaz, durum satırı söyler.
- Tablo paftalarında her tablonun görünüm penceresi tablodan kâğıtta 1,5 mm büyüktür: pencere tablonun kendi sınırına
  oturunca tablonun sağ kenar çizgisi kırpılıyordu (M37'nin hesap paftası da).

### 5. Doğrama tipinin kasa ve kanadı ve U değeri (2. bölümün ilk adımı)

- `OpeningType.material` (Kasa ve kanat; tek satır, en çok 200 harf) ve `uValue` (W/m²K, 0 < U ≤ 10), KCAD şema 217.
  Tipin kendi alanlarıdır: açıklıklar tutmaz; yalnız bunların değişmesi tipin sürümünü artırmaz, açıklıklara dokunmaz.
- `cad.openingtypes.edit` `update`'in `change`'i `material` ve `uValue` alır (null kaldırır). Kopya tip bunları
  taşır (kodunu taşımaz, ADR 3028).
- Kapı ve pencere tipleri penceresi: "Kasa ve kanat" ve "U değeri" (pencerede "W/m²K · TS 825: en çok 1,8").
- **Mevzuat:** çizimin kullandığı bir pencere tipinin U değeri 1,8'i aşarsa `window_u_high`, tip başına bir bulgu,
  pencereleriyle: "P01 (1 pencere): U 2,4 W/m²K > 1,8 W/m²K — TS 825 tavsiye edilen U değerleri". Kaynak "TS
  825:2024 … · RG 20.2.2025/32819, Tebliğ MHGM-2025/1". U değeri yazılmamış tip yargılanmaz (üreticinin sayısı
  çizimden tahmin edilmez). Kapılar TS 825:2024'ün çizelgesinde yok; yargılanmaz.
- Birim kataloğuna `W/m2K` eklendi.

## Ölçüm

- **Testler (2. bölümün ilk adımı):**
  - `opening_types_tests::a_window_type_says_its_frame_and_u_value_and_ts_825_judges_it`: okunamayan U ("iki")
    söylenir; P01'e "PVC, 3 odacıklı; çift camlı" ve 2,4 → bulgu ve kaynağı; 1,8 → bulgu yok; pencere tipin
    değerlerini yeniden gösterir.
  - Ortak örnekler (`opening_types_cases.py`): "Kasa ve kanat ile U değeri: yalnız tip yazılır, sürüm aynı", "Ret: U
    değeri sıfır".
  - KCAD: `opening-specs.kcad` (şema 217) ve kırık `opening-specs-in-schema-216`; 248 geçerli dosya.
- **Testler (1. bölüm):**
  - `room_list_tests` (kabul dairesi): Doğrama kodları'ndan sonra liste Kat, No, Ad … Küpeşte ve korkuluk; "Zemin kat"
    başlık satırı; Salon "Lamine parke … K01, P01 (2), Mermer denizlik, 3 cm"; Yatak odası 1 "K01, P01"; Yatak odası 2
    "—, P01" (kapısı banyoya açılır, banyonun); Koridor "GK01"; Banyo "K01 (2), P02". Hücreden döşeme ve denizlik
    mahale yazılır. İç kapı tipinin kodu K10 olunca liste aynı adımda "K10", "K10 (2)" yazar. Dört geri alma.
  - `permit_set_tests`: takım 6 pafta (UP-05 Mahal listesi), 6 PDF/A dosyası; `…_LS_MHL_NNN_01_000.pdf`; mahal
    listesi paftasında ölçek yazısı yok.
  - `kentos_sheet` `a_list_is_a_sheet_of_its_own_paper_without_a_caption_or_a_scale`: takım A2'de, liste A3'te;
    LS/MHL, `olcek` "—", 15 × 10 m tablo 1/100'de 153 × 103 mm.
  - Ortak örnekler: `cad.schedules.create.extras` 23 durum (yeni: Mahal listesi), merdiven mahali eklendi; 5 durum
    kat sırasıyla yeniden yazıldı; 19 281 durumun hepsi geçiyor.
  - KCAD: `room-list.kcad` (şema 216; denizlikli salon, basamak ve korkuluklu merdiven, mahal listesi) ve kırık
    `room-list-in-schema-215`; 247 geçerli dosya. Python başvurusu (`tools/kcad`) aynı kuralı yazar.
- **Gerçek pencere** (`fixtures/interaction/real/mahal-listesi.txt`; kabul dairesi, 1920 × 1080, sahibin
  ayarlarının kopyası):
  - Önce: Öznitelikler'de yalnız Zemin, Duvar, Tavan kaplaması; Çizelge ▾ içinde Mahal kaplama çizelgesi.
  - Sonra: Doğrama kodları; Salon seçilip beş kaplama Öznitelikler'den yazıldı; Çizelge ▾ › Mahal listesi (0,-12)'ye
    yerleşti; Ruhsat takımı UP-04 Mahal listesi'ni A2'de kurdu (öbürleri A3).
  - Bu dairede İmar durumu olmadığı için UP-01'in bilgi tablosu "@arsa_alani" bekledi (ADR 3037'nin kuralı); UP-01
    çıkarılınca 4 pafta yayınlandı, `260457_00_MM_XXXX_LS_MHL_NNN_01_000.pdf` PDF/A-2b, A2.
  - Resimler: `.run/gui-evidence/mimari-m38/` (Öznitelikler ve menü önce/sonra, listenin çizimdeki ve paftadaki hâli,
    yayın, simge).
- **Simge** `scheduleRoomList`: çizelge ailesinin düzeninde; solda kapılı oda ve altında iki kaplama katmanı, sağda
  liste.

## Sonraki adımlar

- 2. bölüm: grafik doğrama listesi (tip görünüşü Türkiye'deki açılım çizgisiyle, kaba ölçü, açılım, kasa ve kanat
  malzemesi, U değeri, adet), takımda paftası ve TS 825:2024 pencere U denetimi.
- Poz numarası ve TS numarası kaplama metninde yazılıyor; malzeme kitaplığından okunması sonraki iştir.
- Dış cephe ve çatı kaplaması (MO 14.8) bina düzeyinde bir satır olarak; mahal listesinde başlıklı grupta Kat
  sütununun tekrarlanmaması.
