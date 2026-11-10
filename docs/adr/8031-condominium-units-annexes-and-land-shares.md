# 3031 — Kat irtifakı: bağımsız bölüm, eklenti, ortak yer ve arsa payı

Tarih: 2026-11-09. Durum: kabul (M31, ADV-22; plan S22).

## Sorun

- **Tapuya verilecek projenin paylaşım tablosu yoktu.** Kat Mülkiyeti Kanunu 12/a projede bölümlerin niteliğini, 1'den
  sırayla numarasını, eklentilerini, arsa paylarını ve inşaat alanını ister; PAİY 57/2-a "kat irtifakına ve kat
  mülkiyetine esas paylaşım tablosu"nu ruhsat projesinin parçası sayar. Mimar bu tabloyu elle yazıyordu.
- **Bina programında eklenti yoktu.** Bölümün dışındaki deposu, garajı, kömürlüğü (KMK 6) ayrı bir bölüm ya da ortak alan
  gibi giriliyordu; bölümün toplam brütüne (PAİY 4/ğ-4) katılmıyordu. M29 bu yüzden "toplam brüt brüttür" demişti.
- **Bölümün niteliği ve arsa payı yoktu.** "Daire", "dükkân", "iş bürosu" ve arsa payının yazılacağı yer yoktu.
- **Plan M31'in kabulü:** "Bağımsız bölüm, eklenti, ortak yer, kat irtifakı | Paylaşım tablosu".

## Araştırma (9 Kasım 2026)

- **Kat Mülkiyeti Kanunu (634)** (mevzuat.gov.tr, `MevzuatMetin/1.5.634.pdf`; kesitler
  `.run/gui-evidence/mimari-m31/m31-once-sonra.png` altında):
  - 2/a: eklenti "bir bağımsız bölümün dışında olup, doğrudan doğruya o bölüme tahsis edilmiş olan yerler"; 2/b ortak
    yer; 2/d arsa payı.
  - 3/2: kat irtifakı "bağımsız bölümlerinden her birinin konum ve büyüklüklerine göre hesaplanan değerleri ile oranlı
    olarak projesinde tahsis edilen arsa payının … açıkça gösterilmesi suretiyle kurulur". Oransız paylar mahkemede
    düzeltilir.
  - 4: her hâlde ortak yer sayılanlar: merdiven, asansör, sahanlık, koridor, kapıcı dairesi, kalorifer dairesi,
    sığınak, genel su deposu, çatı …
  - 5: anagayrimenkulde bölüme bağlanmamış arsa payı bırakılamaz.
  - 6: kömürlük, su deposu, garaj, saat yuvaları, tuvalet gibi eklentiler ait oldukları bölümün bütünleyici parçasıdır.
  - 12/a: projede "bağımsız bölüm, eklenti, ortak yerlerinin ölçüleri ve … değerleriyle oranlı arsa payları, kat,
    daire, iş bürosu gibi nevi ile bunların birden başlayıp sırayla giden numarası ve bağımsız bölümlerin yapı inşaat
    alanı" açıkça gösterilir.
- **PAİY 4/ğ-4:** BB toplam brüt alanı = BB brüt + eklentilerin brüt alanı; 4/ğ-2: genel brüt toplam brütün üstüne
  ortak alan payıdır. **57/3:** projede eklenti net ve brüt alanı, BB genel ve toplam brüt alanı gösterilir.
- **W4 §2.1 ve §4/3** (`docs/research/2026-11-07-mimari/W4-mevzuat-standart.md`): eklenti → BB bağlantısı, nitelik,
  arsa payı girişi ve "kat irtifakı / BB listesi" çizelgesi.
- **Uygulamada arsa payı:** büyüklüğe göre oransal paylaşım, kat, cephe ve manzara farkı katsayıyla (şerefiye);
  tapudaki payda çoğu kez parsel alanıdır (450 m² parselde 45/450 …).
- Başka programlarda (İdeCAD dahil) bu tabloyu anlatan resmî bir belge bulunamadı; TKGM'nin kat irtifakı kılavuzları
  indirilemedi (site tarayıcı denetimiyle döndü).

## Karar

- **Eklenti** (`UnitKind::Annex`, "Eklenti"; `ProgramUnit.annexOf`): mahalleri olan bir grup, ait olduğu bağımsız
  bölümün kimliğiyle.
  - Kendi grubu olarak ölçülür (M29 konturu); brütü ve neti bölümüne toplanır (`PaiyGroup::annex_gross`, `annex_net`).
  - Ortak alan payı almaz, pay havuzu da değildir; Bina programı denetiminin toplamında bölümlerle sayılır.
  - Seçilmemiş ya da bağımsız bölüm olmayan bir grubu gösteren eklenti reddedilir; `annexOf` yalnız eklentide yazılır.
- **BB toplam brüt alanı** = brüt + eklentilerin brütü; **genel brüt** = toplam brüt + pay (PAİY 4/ğ-2). Ortak alan
  payının "net" kuralı da toplam brütü tartar. Eklentisiz projede M29'un değerleri değişmez.
- **Niteliği** (`ProgramUnit.nature`): bölümün ve eklentinin tapudaki adı (Daire, Dükkân, İş bürosu; Depo, Garaj). Ortak
  alanda gösterilmez.
- **Arsa payı** (`kentos_contracts::land_shares`, kural `kentos.land/1`):
  - Bölümün değeri = toplam brüt (yüzde bir m² adımla) × değer katsayısı (`ProgramUnit.landFactor`, tam okunan ondalık;
    yoksa 1).
  - Payda: programın `landTotal`'ı; yoksa parselin alanı (M30'un imar durumundan, tam m²); yoksa değerlerin tam m²
    toplamı.
  - Paylar en büyük kalanla dağılır: herkes tabanını alır, kalan birimler kalanı en büyük olanlara (eşitlikte programda
    önce gelene) gider. Toplam paydaya tam eşittir.
  - Alanı olmayan bölüm, bölüm sayısından küçük payda ya da bir bölüme pay bırakmayan payda reddedilir (KMK 5).
- **Bölüm çizelgesinin altı yeni sütunu:** Niteliği, Eklenti net alanı, Eklenti brüt alanı, Eklentiler (her eklentinin
  niteliği ya da adı ve brütü: "Depo 30.00 m²"), BB toplam brüt alanı, Arsa payı ("214/450"). Eklentisi olmayan
  bölümde eklenti alanları boştur. Niteliği çizelgeden düzenlenir (programa yazılır).
- **Kat irtifakı tablosu** (`tool.scheduleLandShares`; Mimari › Belgele › Metraj ▾; KATIRTIFAKI, ARSAPAYI,
  PAYLASIMTABLOSU):
  - Yalnız bağımsız bölümler; sütunlar No, Kat, Niteliği, BB net, BB brüt, Eklentiler, Eklenti net, BB toplam brüt, BB
    genel brüt, Arsa payı ve toplam satırı.
  - Çizelge olduğu için her adımda yenilenir: mahal, duvar ya da program değişince (payda değişince de).
  - Yerleşince söyler: paydayı, arsa payının verilemediği sebebi, numaraların 1'den sırayla gitmediğini (KMK 12/a) ve
    niteliği olmayan bölümleri.
- **Bina programı penceresi:** Tür'de "Eklenti"; satırda Niteliği, eklentide Bölümü (ilk bağımsız bölüm önerilir),
  bağımsız bölümde Değer katsayısı; üstte Arsa payı paydası (boş: parsel alanı).
- **Yeni ikon `scheduleLandShares`:** pay dilimi gösteren bir daire, tablo satırları ve altında arsa çizgisi.
- **KCAD şema 223:** eklenti, nitelik, değer katsayısı, arsa payı paydası ya da bu sütunlardan biri şema 223 ister;
  eski sürüm `UnknownField` ile reddeder. Python okuyucusu aynı kuralları uygular.
- KentOS bir onay vermez: arsa payı mimarın girdileriyle hesaplanır; maliklerin anlaşması ve yönetim planı ayrıdır.

## Ölçüm

- **Test** (`land_share::tests`): 60, 30, 30 m² ve 450 m² parsel → 225, 113, 112 (yarım önce gelene); üçüncüde 1,2
  katsayı → 214, 107, 129. Parselsiz 45,4 ve 44,6 m² → payda 90; verilen payda 1000 tam dağılır. Alansız bölüm, 1 payda
  ve bir bölüme pay bırakmayan 3 payda reddedilir.
- **Test** (`paiy_area_tests::a_kat_irtifaki_table_lists_the_units_with_their_annexes_and_land_shares`): M29'un iki
  katlı binası; üst kattaki doğu oda 1 numaralı dairenin eklentisi (Depo), 3 numaralı daire 1,2; merdiven evi eşit
  paylaşılır; parsel 450 m².
  - Eklentisi seçilmemiş eklenti programda reddedilir.
  - Tablo: 1 · Zemin kat · Daire · 25.71 · 30.00 · Depo 30.00 m² · 25.71 · 60.00 · 80.00 · 214/450; 2 ve 3: 30.00,
    50.00, 107/450 ve 129/450; toplam 77.14 / 90.00 / 25.71 / 120.00 / 180.00.
  - D1, D2, D3 numaraları ve niteliksiz D2 söylenir.
- **Gerçek pencere** (`fixtures/interaction/real/kat-irtifaki.txt`, 1440 × 900; testin sakladığı `irtifak.kcad`):
  - Bina programında yeni alanlar.
  - Metraj ▾'de dördüncü satır; tablo imleçten yerleşti; değerler testle aynı.
  - Payda 1000 yazılıp kaydedilince tablo 476/1000, 238/1000, 286/1000 oldu.
  - Resim: `.run/gui-evidence/mimari-m31/m31-once-sonra.png`.
- **KCAD:** `land-shares.kcad`; kırık `land-shares-in-schema-222` ve `land-shares-annex-without-unit`. 242 geçerli
  dosya; 1704 dosya Python okuyucusuyla tutarlı.

## Sonraki adımlar

- Ortak yer sözlüğü (KMK 4): adı "Merdiven", "Kapıcı dairesi", "Sığınak" olan atanmamış mahalleri ortak alana önermek.
- Elle verilen arsa payları (maliklerin anlaşması) ve paydanın parsel alanıyla tutarlılık uyarısı.
- Bölümleri kat ve konum sırasıyla 1, 2, 3 … numaralayan araç.
- Bina programı Durum sekmesinde arsa paylarının gösterilmesi.
- Eklentinin vaziyet planında gösterilmesi (bahçe eklentisi, KMK 6/2; M32).
