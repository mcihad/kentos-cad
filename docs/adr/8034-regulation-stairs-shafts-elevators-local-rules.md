# 8034 — Mevzuat denetimi: merdiven, rampa, korkuluk, ışıklık, asansör ve yerel yönetmelik

Tarih: 2026-11-09. Durum: kabul (M34, ADV-23; plan S23).

## Sorun

- **Merdiven, rampa ve şaftlar yönetmelikle okunmuyordu.** Mevzuat grubu (ADR 8033) piyes, kat ve kapıyı biliyordu.
  Merdivenler ve Erişilebilirlik grupları profil değeriyle uyarıyordu ("mevzuat onayı değildir"); madde yazmıyordu,
  merdivenin ortak mı, daire içi mi olduğunu bilmiyordu. Işıklık, hava bacası ve asansör hiç ölçülmüyordu.
- **Belediye yönetmeliği yoktu.** Sivas'ta ortak merdiven 1,30 m, asansör 3 kattan itibaren; hava bacası ise PAİY'den
  gevşek (0,30 m ve 0,36 m²). Mimar iki metni yan yana açıp hangisinin geçerli olduğunu kendisi çıkarıyordu.
- **Plan M34'ün kabulü:** "Merdiven, rampa, korkuluk, ışıklık, asansör denetimi | Sivas ile PAİY çelişkisi görünür".

## Araştırma (9 Kasım 2026)

- **Planlı Alanlar İmar Yönetmeliği** (mevzuat.gov.tr, `MevzuatMetin/yonetmelik/7.5.23722.pdf`; 23 değişiklik,
  sonuncusu RG 1.7.2026/33297; alıntılar `.run/gui-evidence/mimari-m34/m34-kaynaklar.png`):
  - 30/12: bina girişinde ve içindeki rampanın eğimi kot farkına göre: 15 cm'ye kadar %8, 16–50 cm %7, 51–100 cm %6,
    100 cm üzeri %5. 30/13: rampada kesintisiz 0,90 ve 0,70 m'de iki düzey küpeşte.
  - 31/1-a: ortak merdiven kolu ve sahanlığı konutta 1,20 m, diğer yapılarda 1,50 m; konutta bağımsız bölüm içindeki
    merdiven 1,00 m. 31/1-ç: merdivenin iki yanında korkuluk ve küpeşte.
  - 31/2: rıht asansörsüz binada en çok 0,16 m, asansörlüde 0,18 m; 2a+b = 60–64 cm, basamak en az 0,27 m; dengelenmiş
    merdivende dar uçta 0,10 m. 31/3: son kattaki bölümün çatı piyesine çıkan iç merdivende aranmaz.
  - 32/2: ışıklık 1–6 katlı binada dar kenar 1,50 m ve 4,50 m², 7 ve daha fazla katta 2,00 m ve 9,00 m². 32/3: hava
    bacası her binada en az 0,60 × 0,60 m.
  - 34/1 (1.7.2026 değişikliği): tek bağımsız bölümlü konutlar hariç, bağımsız bölüm, ortak alan (teknik hacimler
    dışında) ya da eklenti bulunan bodrum dahil kat adedi 3 olan binada asansör yeri, 4 ve daha fazlada asansör.
    34/2: kabin dar kenarı 1,20 m ve alanı 1,80 m², kapısı net 0,90 m; kapının açıldığı sahanlık sürgülü kapıda 1,20,
    dışa açılanda 1,50 m; birden çok asansörde yarısı.
  - 38/1: birden fazla basamaklı açık merdivende, rampada ve kotu 0,90 m'den az pencere boşluğunda 1,10 m korkuluk.
  - 69/1–3: il belediyeleri değiştirilemeyen hükümler (1, 2, 6–9. bölümler, 19, 20) dışında Bakanlık onayıyla imar
    yönetmeliği yapar; plan aksini demiyorsa o yönetmelik uygulanır. 28–39. maddeler beşinci bölümdedir.
- **Sivas Belediyesi İmar Yönetmeliği** (RG 10.11.2022/32009, resmigazete.gov.tr `eskiler/2022/11/20221110-3.htm`):
  - 28/1-c: konut bölgesinde zemin ve normal katlar 3,60 m. PAİY'nin 12.8.2023'te eklediği "3 katı geçmeyen binada
    4,00 m" bu metinde yoktur.
  - 31/1-a: ortak merdiven konutta 1,30 m; konutlarda ara sahanlık 1,30 m, kat sahanlığı 1,50 m. 31/7: üç kata kadar tek
    bölümlü konutta merdivenin çatıya ve bodruma ulaşması gerekmez.
  - 32/2-a: ışıklık yararlanan kat sayısına göre (değerler PAİY'ninki). 32/2-b: cepheye açılan ışıklıkta yalnız dar
    kenar. 32/3: hava bacası dar kenar 0,30 m ve 0,36 m².
  - 34/1: en çok üç katlı tek bölümlü konut hariç, kat adedi 3 ve daha fazla binada asansör; iskân edilen ve ortak alan
    bulunan bodrum sayılır.
  - 39/1-c: balkon ve tuvalet kapıları 0,80 m'ye düşürülebilir.
  - 30/12–13 rampa, 31/2 basamak, 34/2 kabin ve 38/1 korkuluk PAİY ile aynıdır.
- **W4 §2.7–2.10, 2.21** (`docs/research/2026-11-07-mimari/W4-mevzuat-standart.md`): katmanlar ulusal → belediye →
  plan notu; "çelişkide idare karar verir; KentOS iki değeri kaynaklarıyla gösterir ve sıkı olanı önerir".
- **KentOS'ta parçalar hazırdı:** merdivenin kol, rıht, basamak, sahanlık ve korkuluk alanları; `stair_landings`,
  `stair_measure` (döner merdivenin dar ucu); rampanın eğimli boyu (`ramp_run`); mahal boşluğu (ADR 0708); kapının
  işleyişi (sürme, kanatlı).

## Karar

- **Madde kaynağını taşır** (`Article.source`: `Paiy`, `Sivas`). `rule` "Sivas İmar Yön. 31/1-a · RG
  10.11.2022/32009" der; iletinin sonu "Sivas 31/1-a". `RULE_SET` PAİY'nin son değişikliğini 1.7.2026/33297 olarak
  yazar (M33'teki metin 14.1.2026'da kalmıştı; 34/1 o tarihten sonra değişti).
- **Yerel imar yönetmeliği** (`ArchSettings.local_rules`, `LocalRules::Sivas`; KCAD 225):
  - Proje ayarları › Mimari › İmar durumu › **Yerel imar yönetmeliği**: "Yok (yalnız Planlı Alanlar İmar Yönetmeliği)"
    ya da "Sivas Belediyesi İmar Yönetmeliği (10.11.2022)". Proje dosyasında saklanır.
  - Her ölçü iki sınırla okunur (`judge`):
    - İkisine de uyuyor: bulgu yok.
    - Yerel sınır daha sıkı ve ölçü ona uymuyor: bulgu kendi koduyla, yerel madde bağlar, PAİY'ninki iletide yanında:
      "Ortak merdivenin kolu 1.25 m < 1.30 m — Sivas 31/1-a; PAİY 31/1-a: 1.20 m". Sıkı yerel değer çelişki değildir:
      ikisini birden karşılamak mümkündür ve PAİY 69/3 yerel yönetmeliği uygular.
    - Yerel sınır daha gevşek ve ölçü yalnız ona uyuyor: **`regulation_conflict`**. İleti iki maddeyi ve kimin karar
      verdiğini söyler: "Hava bacasının dar kenarı 0.40 m: Sivas 32/3 en az 0.30 m ister, uyuyor; PAİY 32/3 en az 0.60 m
      ister, uymuyor — çelişkide idare karar verir". `rule` iki maddeyi "yerel ↔ ulusal" diye taşır. Onarım PAİY
      değerine uymanın çelişkiyi kaldırdığını söyler.
    - İkisine de uymuyor: sıkı olan bağlar, öbürü yanında yazılır. Değerler eşitse yalnız PAİY yazılır.
  - Sivas değerleri: ortak merdiven ve ara sahanlık 1,30 m; asansör 3 kattan; konutta kat yüksekliği 3,60 m
    (28/1-c); hava bacası 0,30 m ve 0,36 m²; tuvalet ve balkon kapısı 0,80 m; ışıklık yararlandığı kat sayısıyla
    (ışıklık boşluğunun çizildiği kat sayısı).
- **Merdivenler (31):**
  - Merdivenin yeri, yürüme çizgisinin ortasının durduğu mahalin bölümüdür: konut bölümü (ya da onun eklentisi) içi
    1,00 m; konut olmayan bölümün içi kol ölçüsü aranmaz (yönetmelik yalnız konutun iç merdivenini sayar); ortak
    alan, bölümsüz mahal ya da hiç mahal çizilmemişse ortak merdiven. O katta mahal varken hiçbir mahalde değilse dış
    merdivendir, kol ölçülmez.
  - Ortak merdiven konut binasında 1,20 m (Sivas 1,30), konut dışı binada 1,50 m; ara sahanlık aynı sınırla.
    Bina konut dışıdır yalnız programdaki bütün bölümlerin niteliği konut değilse (ADR 8033: nitelik verilmemiş
    bölüm konuttur).
  - Rıht 0,16 m, binada Asansör adlı bir mahal varsa 0,18 m. 2a+b 0,60–0,64 m, basamak 0,27 m, döner merdivende dar
    uçta 0,10 m. Bunlar her merdivene uygulanır; 31/3'ün çatı piyesi merdiveni ayırt edilemiyor.
  - Korkuluk: ortak merdivende iki yanda (31/1-ç; hangi yanın eksik olduğu söylenir), öbür merdivende birden çok
    basamakta en az bir yanda (38/1).
- **Rampalar (30/12, 38/1):** eğim kot farkı ÷ eğimli boy; korkuluksuz rampa bulgusu. 2,75 m ve daha geniş rampa
  taşıt rampasıdır (Otopark Yönetmeliği 5/1-h: rampa genişliği en az 2,75 m) ve okunmaz.
- **Pencereler (38/1):** zemin katın üstündeki (kotu 0'dan büyük) katlarda denizliği 0,90 m'den alçak pencere "1.10 m
  korkuluk gerekir" der. Zemin kat ve bodrum okunmaz. Pencere korkuluğu nesnesi yok; çizildiyse bulgu istisna yapılır.
- **Işıklık ve hava bacası (32/2, 32/3):** adı ya da kullanımı Işıklık (Aydınlık) olan mahal ışıklık, Hava bacası
  (Havalandırma bacası) olan hava bacasıdır; net dar kenar (en büyük iç dairenin çapı) ve net alan ölçülür. Tesisat
  şaftı okunmaz.
- **Asansör (34):**
  - Adı ya da kullanımı Asansör olan mahal (Asansör holü, önü, lobisi değil) asansör kuyusudur.
  - Kat adedi: kotu ±0,00 ve üstündeki katlar, artı teknik olmayan (kazan dairesi, ısı merkezi, elektrik odası, su
    deposu, jeneratör, iklimlendirme…) bir mahali olan bodrumlar. İleti "bodrum dahil 4 kat" der.
  - Asansör yoksa: PAİY 3 katta "asansör yeri yok", 4 ve daha fazlada "asansör yok"; Sivas 3 kattan "asansör yok"
    ("Sivas 34/1; PAİY 34/1 (1.7.2026): 3 katta asansör yeri"). Programda tek konut bölümü varsa PAİY aramaz; Sivas
    yalnız üç kata kadar aramaz. Programsız projede bölüm sayısı bilinmez: onarım, tek bölümlü konutsa bölümün
    tanımlanmasını söyler.
  - Kuyunun net ölçüsü kabininkinden (1,20 m, 1,80 m²) küçükse kabin sığmaz: `paiy_elevator_small`. Aynı katta birden
    çok kuyuda yarısının (aşağı yuvarlanır) sığması yeter. Kitaplığın Asansör bloğu da asansördür; kuyudaki bloğun
    kabini ve kapısı ayrıca ölçülür, 10 katta sedye asansörü aranır (ADR 8045 §5).
  - Asansör kuyusuna açılan kapının öbür yanındaki mahal sahanlıktır: net eni sürme ya da cebe giren kapıda 1,20 m,
    öbür kapıda 1,50 m (`paiy_elevator_landing_narrow`).
- **Düzeltmeler:** ortak merdivenin kolu ve sahanlığı ("Genişliği 1.30 m yap", "Sahanlığı 1.30 m yap") ve korkuluk
  ("İki yana korkuluk koy") `cad.arch.edit` ile; kilitli katmandaki nesnede düzeltme verilmez.
- **Denetim tablosu:** sütunlar sürüklenerek genişletilir (`ArchAuditPanel.columns`). Mevzuat iletileri uzun ve maddesi
  sonda; 1920 piksel genişlikte bile kesiliyordu.
- **Yeni kodlar** (uyarı): `paiy_stair_width_narrow`, `paiy_stair_landing_short`, `paiy_stair_riser_high`,
  `paiy_stair_step`, `paiy_railing_missing`, `paiy_ramp_steep`, `paiy_window_railing`, `paiy_lightwell_small`,
  `paiy_air_shaft_small`, `paiy_elevator_missing`, `paiy_elevator_small`, `paiy_elevator_landing_narrow`,
  `regulation_conflict`.
- Bulgu yokluğu ruhsat uygunluğu değildir; planın notları ayrıca okunur.

## Ölçüm

- **Test** (`regulation_audit::tests`):
  - Adlar: Asansör ve Asansör kuyusu → asansör, Asansör holü ve önü → yok; Işıklık, AYDINLIK → ışıklık; Hava bacası,
    Havalandırma bacası → hava bacası; Tesisat şaftı → yok. Kazan dairesi, Isı merkezi, Su deposu teknik; Işıklık ve
    Sığınak değil.
  - Katmanlar: 1,25 m (PAİY 1,20, Sivas 1,30) → Sivas bağlar, PAİY yanında; 0,40 m (PAİY 0,60, Sivas 0,30) → çelişki;
    0,20 m → PAİY bağlar, Sivas yanında; eşit değerde yalnız PAİY; kat 3,80 m (PAİY 4,00, Sivas 3,60) → Sivas bağlar.
- **Test** (`regulation_audit_tests::a_four_storey_building_s_stairs_ramp_shafts_and_elevator_are_said`): 4 katlı, iki
  daireli bina. Ortak merdiven 1,25 m, rıht 0,1667, yalnız solda korkuluk; 4 m'de 0,40 m rampa; 0,40 × 0,90 m hava
  bacası; 1,40 × 3,00 m ışıklık; 1. katta denizliği 0,40 m pencere; 1,00 m kaba (0,86 m net) WC kapısı.
  - PAİY: tam olarak on bulgu, sırasıyla: kapı 0,86 < 0,90; rıht 0.167 > 0.16 (asansörsüz); ortak merdivenin sağ yanı;
    rampada korkuluk; rampa %10.00 > %7; pencere denizliği; ışıklık 1.40 < 1.50 ve 4.20 < 4.50 m² (4 katlı bina);
    hava bacası 0.40 < 0.60; "Binada asansör yok (4 kat) — PAİY 34/1 (1.7.2026)".
  - Sivas: merdiven kolu ve ara sahanlığı "1.25 m < 1.30 m — Sivas 31/1-a; PAİY 31/1-a: 1.20 m"; hava bacası ve
    "Tuvalet kapısının net genişliği 0.86 m: Sivas 39/1-c en az 0.80 m ister, uyuyor; PAİY 39/1-c (13.8.2025) en az
    0.90 m ister, uymuyor — çelişkide idare karar verir"; hava bacası artık `paiy_air_shaft_small` değil.
  - 3 kat: Sivas "Binada asansör yok (3 kat) — Sivas 34/1; PAİY 34/1 (1.7.2026): 3 katta asansör yeri"; Sivas
    kapalıyken "Binada asansör yeri yok (3 kat; 3 katta asansör yeri bırakılır)".
  - 1,10 × 1,50 m Asansör eklenince eksik bulgusu ve rıht bulgusu (0,18'e göre) kalkar; "Asansör kuyusunun net dar
    kenarı 1.10 m < 1.20 m" ve "net alanı 1.65 m² < 1.80 m²".
  - Merdiven bulgusunun düzeltmesi "Genişliği 1.30 m yap": çalıştırılınca kol ve sahanlık bulguları kalkar.
- **Gerçek pencere** (`fixtures/interaction/real/mevzuat-merdiven.txt`, 1920 × 1080; testin sakladığı `bina.kcad`):
  - Önce (M33 ikilisi, aynı çizim): Mevzuat'ta tek bulgu, kapı.
  - Sonra: Denetim 14 bulgu, Mevzuat süzgeciyle 10. Proje ayarlarında Sivas seçilip Kaydet, Denetle: 16 bulgu, 12'si
    Mevzuat; Sivas 31/1-a önde, iki çelişki sonda. Bulgu sütunu sürüklenip genişletildi; çelişkiler tam okunuyor.
  - Satır seçilip Düzelt: merdiven 1,300 m (Öznitelikler), kol ve sahanlık bulguları kalktı.
  - Resimler: `.run/gui-evidence/mimari-m34/m34-once-sonra.png`, `m34-ayar-duzeltme.png`, `m34-kaynaklar.png`.
- **Fikstürler:** 19 280 komut vakası değişmedi (grup adıyla istenir). KCAD: `local-rules.kcad`; kırık
  `local-rules-in-schema-224`; 244 geçerli dosya, 1709 dosya Python okuyucusuyla tutarlı.

## Sonraki adımlar

- Kat sahanlığı (Sivas 1,50 m) ve asansör kapısının net genişliği (34/2, 0,90 m) ayrı ölçülsün; baş yüksekliği ve
  sahanlık uzunluğu yangın kuralıyla (BYKHY, M35).
- 31/3 çatı piyesi merdiveni ve 31/7 (Sivas) için merdivenin hangi katlara ulaştığı; 34/8 asansörün bütün katlara
  ulaşması.
- Işıklıktan yararlanan piyes sayısı (32/5) ve cepheye açılan ışıklık (Sivas 32/2-b); pencere korkuluğu nesnesi.
- Projede ruhsat başvuru tarihi ve bölge türü; Sivas 28/1-a, b, ç ticaret ve karma bölge kat yükseklikleri; 29/2
  birleşik ıslak hacimde alanların toplamı; dükkân ve büro 29/9.
- Başka belediyelerin yönetmelikleri (veri olarak); parselin plan notu katmanı.
