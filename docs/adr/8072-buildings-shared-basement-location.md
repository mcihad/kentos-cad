# ADR 8072: Çok bina, ortak bodrum, konum ve kot

- **Durum:** uygulandı (16 Kasım 2026; §1–§5); iyileştirmeler "Sıradaki adımlar"da. Plan: [bim-plan.md](../../plan/bim-plan.md) BIM-11,
  [ifc-uyum.md](../../plan/ifc-uyum.md) IFC-05; uygulama sırasında B11. Bu ADR maddenin bütün adımlarını tutar; koddaki
  "docs/adr/8072 §N" atıfları bölümleri gösterir: §1 binalar (kod, kullanım, ±0.00, ortak kat), §2 bina başına model, §3 konum
  (yerel sistem, harita dönüşümü, kot, yerleşim zinciri), §4 alanlar ve bina başına çıktılar, §5 parselin plan kararları.
  Kararların numarası
  bölümler boyunca sürer. Ortak sınamalar ve sıradaki adımlar numarasız son bölümlerdedir.
- **Bağlam:** ADR [0773](0773-building-program-units-and-design-brief.md) (bina programı), [0784](0784-building-wing-window-and-common-area-shares.md) §1
  (binaların ve kanatların katları), [8032](8032-site-plan-parcel-setbacks-and-datum.md) (vaziyet planı, saha röperi),
  [8047](8047-ifc-building-export.md) (binanın IFC'si), [8069](8069-room-zone-far-model.md) (emsal bölgeleri).
- **Dayanak:** Dijital Proje Yönetmeliği (RG 5.8.2026/33331) EK-2 §2.3 ve Tablo 2.1 (bina kodu, dosya adı); EK-6 Tablo
  6.7 (`IfcBuilding`), 6.55 (`IfcSite`); EK-7'nin ortak öznitelikleri ve Tablo 7.6–7.8 (koordinat sistemi, harita
  dönüşümü, temsil bağlamı). Metinler `mevzuat/bim/ek-2-…`, `ek-6-1-…`, `ek-6-2-…`, `ek-7-proje-siniflari.md`.
- **Web:** dokunulmadı (ADR 0228).

## §1. Binalar: kod, kullanım, ±0.00, ortak kat

**Araştırma.**

- **EK-2 §2.3:** "'Bina kodu' aynı vaziyet planı içerisinde birden fazla yapının (bina/blok) yer alması ve projelerin
  bina/bloklar için ayrı hazırlanması durumda kullanılan bina kodunu temsil eder. İki karakterli alfanümerik kısaltma ile
  ifade edilir. Projenin tek bir binadan oluştuğu durumda '00' kısaltması kullanılır." **Tablo 2.1:** dosya adı Proje
  Kodu (6) _ Bina Kodu (2) _ Disiplin (2) _ Öge (4) _ Dosya Türü (2) _ Dosya Tipi (3) _ Kat/Konum (3) _ Sıra No (2) _
  Revizyon No (3).
- **EK-6 Tablo 6.7 (`IfcBuilding`, zorunlu):** Uzun İsim, İsim; `Pset_BuildingCommon` BuildingID, IsPermanentID,
  ConstructionMethod, FireProtectionClass, OccupancyType ("Kullanım Türü"), GrossPlannedArea, NetPlannedArea,
  NumberOfStoreys; adres; `Qto_BuildingBaseQuantities` Height, EavesHeight, FootPrintArea. **Tablo 6.55 (`IfcSite`):**
  RefElevation "Deniz seviyesine göre referans (datum) kotudur"; FloorAreaRatio, SiteCoverageRatio, TotalArea.
- **IFC 4.3** (buildingSMART IFC4.3.x-development, `IfcSite.md`): "reference height of site is provided by
  IfcSite.RefElevation … the reference height of each building situated at the site is given against the same height
  datum … the elevations of each storey belonging to each building are given as local height relative to the reference
  height of the building"; `IfcBuildingStorey.Elevation` binanın 0,00'ına göredir (4.3'te `Pset_BuildingStoreyCommon`
  ElevationOfSSLRelative ve ElevationOfFFLRelative önerilir); `IfcBuilding.ElevationOfRefHeight` deniz seviyesine göre
  binanın 0,00'ıdır (4.3'te kullanımdan kalkıyor).
- **ADR 0784 §1:** programın binaları ve kanatları katlarını kaplar; bir kat başka binanın da olabilir (ortak bodrum),
  ama hangi binanın sayılacağı yazılmıyordu.
- **Üreticiler (karşılaştırma için):** Archicad bir projede tek `IfcBuilding` tutar; bir sahayı paylaşan ayrı
  projelerde aynı "Project ID" ve "Site ID", farklı "Building ID" verilmesini önerir (help.graphisoft.com, "IFC Data
  Types"). KentOS'ta binalar tek projede durur; dışa aktarma bina başına model yazacaktır (§2).

**Karar.**

1. **Binanın yeri programdadır** (`ProgramBuilding`): `code` (EK-2 §2.3 bina kodu: iki büyük harf ya da rakam, her
   binada ayrı; "00" yalnız tek binalı projede), `usage` (kullanım türü, EK-6'nın OccupancyType'ı), `zero` (±0.00'ı,
   projenin ±0.00'ına göre metre; en çok ±100 m). Binanın mutlak ±0.00'ı saha röperi (ADR 8032) artı bu farktır; tek
   binada fark verilmezse projeninkidir.
2. **Ortak kat** (`BuildingProgram::shared`, `SharedLevel { level, building }`): birden çok binanın kapladığı kat
   (ortak bodrum) seçilen binanın katı sayılır, alanı bir kez. Seçim kapsayan binalardan biri olmalı, her kat bir kez;
   tek binanın katı için seçim reddedilir. Seçilmeyen ortak kat kimsenin değildir (`unshared_levels`).
3. **Her nesnenin binası katından okunur** (`BuildingProgram::building_of_level`): katı kaplayan tek bina, birden
   çoksa ortak katın seçtiği; hiçbiri ya da seçilmemişse yok.
4. `.kcad` mimari verisi 9 (`ARCH_WITH_BUILDING_PLACES`); `cad.program.edit` yeni alanları programla birlikte yazar ve
   doğrular. Bina programı › Binalar: her binanın Kodu (büyük harfe çevrilir), Kullanım türü ve ±0.00'ı; "Ortak katlar"
   altında her ortak katın Sayıldığı bina; seçilmeyen için "Seçilmedi" notu.

**Sınama.** `program::tests::a_shared_basement_counts_in_the_building_chosen` (seçimle ve seçimsiz ortak kat,
kodun biçimi, tekrarı, "00", seçimin katı ve binası, ±0.00'ın sınırı); `.kcad`'ın
`building_places_round_trip_under_arch_data_9`'u; ortak durumlar (`cad.program.edit` 40: iki blok ve tek binanın 00'ı
kabul, sekiz ret); masaüstünde `two_blocks_get_codes_zeros_and_a_shared_level_through_the_window` (pencereden yazılır,
"00" reddedilir, hiçbir şey yazılmaz). Resim: `bina-ortak-kat` (`.run/shots/arac-bina-ortak-kat-*`).

## §2. Bina başına model

**Araştırma.**

- **EK-2 §2.3:** projeler bina/bloklar için ayrı hazırlanınca dosya adında bina kodu; **BIM-11 ve IFC-05:** "İlk çok bloklu
  pilotta bina başına model dosyası … Ortak proje ve saha alanları eşleşir. Her bloğun ±0.00'ı ortak saha röperine ayrı
  bağlanır"; "Yerleşim zinciri: saha → bina → kat → eleman; döngü ve çift öteleme denetimi".
- **IFC 4.3** (`IfcSite.md`, `IfcBuildingStorey.md`): her binanın referans kotu sahanın datumuna göre, katların kotu
  binanın referans kotuna göre; katın yerleşimi `ObjectPlacement`'tır, Elevation bilgi içindir.
- **Archicad** (help.graphisoft.com, "IFC Data Types"): bir sahayı paylaşan ayrı projelerde aynı Project ID ve Site ID,
  farklı Building ID (karşılaştırma için).

**Karar.**

5. **`cad.arch.ifc`'nin `building`'i** programın bir binasını yazar: katları (ortak kat sayıldığı binada) ve onlardaki
   nesneler; duvarı yazılan açıklık; katı olmayan aks ızgarası her binada. Katı olmayan ya da ortak katının binası
   seçilmemiş yapı nesnesi hiçbir binanın dosyasına girmez, `export_losses` ile söylenir; programda olmayan bina
   `invalid_ifc` (`building`).
6. **Kot:** bina sahada kendi ±0.00'ında durur (`Building::zero`, yerleşimin z'si); katların Elevation'ı ve yerleşimi
   binanın ±0.00'ına göre (kat kotu eksi bina farkı); nesneler katlarına göre eskisi gibi: çift öteleme yok. 3B ağı
   binanın farkını ekler.
7. **Kimlik ve ad:** bina adı ve uzun adı programdaki ad (EK-5: MM-BNA-…); BuildingID künyenin proje kodu ve binanın
   kodu; OccupancyType binanınki (yoksa künyeninki); künyenin proje toplamı olan alanları (GrossPlannedArea,
   FootprintArea, GrossFloorArea) birden çok binada yazılmaz. Proje ve sahanın GlobalId'si projenin tohumundan, her
   binanın dosyasında aynı; binanınki ve öbür bütün kimlikler binanın dosya adından (EK-2 adı; proje kodu yoksa çizimin
   adı ve bina kodu): iki dosya birlikte okunduğunda yalnız proje ve saha ortaktır; katı olmayan akslar her binada
   kendi kimliğiyle.
8. **Masaüstü:** programında birden çok bina olan projede IFC dışa aktar (bina) her bina için bir dosya yazar: ilki
   kullanıcının seçtiği yere, öbürleri yanına EK-2 adlarıyla; ileti her binanın kat ve duvar sayısını söyler.

**Sınama.** `arch_ifc::two_blocks_are_two_models_with_their_own_zero_and_the_basement_once` (kesit binası A Blok, 20 m
doğusunda 1,20 m yukarıdan girilen B Blok, ortak bodrum A'da: A'nın katları −3,00/±0,00/+3,00, B'ninki kendi ±0,00'ında
0 ve bina 1,20 yukarıda, B'nin duvarları +1,20'den, bodrum döşemesi yalnız A'da, saha ve proje aynı, adlar ayrı);
`curved_ifc_check.py`'nin bloklar bölümü `blok-a.ifc` ve `blok-b.ifc`'yi IfcOpenShell'le okur: 0 bulgu, aynı saha ve
proje, başka ortak kimlik yok, katların yerleşim zincirinden mutlak kotları −3,00/±0,00/+3,00 ve +1,20, B'nin katının
Elevation'ı 0, döşemeler A'da 4 B'de 0, B'nin duvarları +1,20'de; masaüstünde `two_blocks_go_out_as_two_files`; ortak
durum (`cad.arch.ifc` 27: programda olmayan bina reddi).

## §3. Konum: yerel sistem, harita dönüşümü, kot

**Araştırma.**

- **EK-7 Tablo 7.6 (`IfcProjectedCRS`, PDF s. 227–228):** ad TUREF'in 3° dilimine karşılık gelen EPSG kodu (5253–5259);
  jeodezik datum TUREF; düşey datum "Yükseklik referans sistemi olarak ITRF96 kullanılır. Gerektiğinde yerel deniz
  seviyesi referansına (TUDKA99) dönüşüm parametreleri ayrıca belirtilir"; projeksiyon Transverse Mercator; zon merkez
  meridyeni; birim METRE. **Tablo 7.7 (`IfcMapConversion`, s. 229–230):** doğu, kuzey, düşey yükseklik; X ekseninin
  doğu ve kuzey bileşeni "yerel X ekseninin harita koordinat sisteminin yatay düzlemi içindeki yönünü" verir; ölçek
  "Koordinat referans sisteminin (CRS) birimleri ile mühendislik koordinat sisteminin birimleri aynı olmadığında
  kullanılacak ölçek değeri", verilmezse 1. **Tablo 7.8 (s. 231–232):** `WorldCoordinateSystem` yerel sıfır ile
  dönüşümün ötelemesinin bağlı olduğu nokta arasındaki ötelemedir; "Tercih edilen uygulamada her iki nokta (genellikle
  'proje baz noktası' ve 'ölçüm noktası' olarak adlandırılır) çakışık olmalıdır"; `TrueNorth`, harita dönüşümü varken
  bilgi içindir, tutarsızlıkta dönüşüm geçerlidir.
- **IFC 4.3** (buildingSMART IFC4.3.x-development, `IfcMapConversion.md`): eksenler önce `Scale` ile ölçeklenir ("one scale
  is applied equally to x, y and z, to convert units"), sonra θ = atan(XAxisOrdinate / XAxisAbscissa) kadar saat
  yönünün tersine döner, sonra Eastings, Northings, OrthogonalHeight kadar ötelenir; yerel z ekseni haritanınkine hep
  paraleldir. `IfcMapConversionScaled.md`: "additional different factors multiply x, y and z, to scale coordinates - not
  units" (FactorX, FactorY, FactorZ). Şema (IfcOpenShell 0.9.0'ın IFC4X3_ADD2'si): OrthogonalHeight zorunlu;
  XAxisAbscissa, XAxisOrdinate, Scale seçimli; ölçekli dönüşümün üç katsayısı zorunlu. `IfcLocalPlacement.md`: eleman
  kapsayan yapının (kapsama ilişkisindekiyle aynı) yerleşimine göre, açıklık deldiği elemana, parça bütününe, kapı ve
  pencere doldurduğu açıklığa göre; saha mutlak; "Rules to prevent cyclic relative placements have to be introduced on
  the application level".
- **Okuyucu:** IfcOpenShell 0.9.0'ın `ifcopenshell.util.geolocation`'u `Scale`'i birim ölçeği sayar ve x, y ile birlikte
  z'ye de uygular; ölçekli dönüşümün katsayılarını eksen başına okur.
- **Üreticiler (karşılaştırma için):** Revit'in IFC dışa aktarıcısı projenin ölçek katsayısını ("scale factor * surface
  distance = grid distance") `IfcMapConversion.Scale`'e yazıyor (OSArch wiki, topluluk kaynağı; Autodesk'in ve
  Graphisoft'un kendi sayfaları otomatik okumaya 403 verdi). IFC 4.3'e göre okuyan program bu katsayıyla yükseklikleri de
  ölçekler: 0,99983'te 30 m'lik bina 5 mm kısalır.
- **KentOS'ta olan:** projenin yerel sistemi (ADR 0168 §1: `CrsSystem::Local`, taban ve düzlem: saat yönünün tersine dönüş,
  ölçek, öteleme ya da afin) ve Özel koordinat sistemi'nin Ortak noktalardan hesapla'sı (en küçük kareler, artıklar, m0;
  ADR 0168 §6). Yerel orijin ve kontrol noktaları buydu; IFC ise yerel sistemin adını dönüşümsüz yazıyordu, ±0.00'ın
  kotu bilinmezse yüksekliğe 0 yazıyordu.

**Karar.**

9. **Bina ve yol aynı yerden** (`ifc_system::of`): projenin sistemi listenin düzlem bir sistemiyse çizim onun içindedir,
   dönüşüm düzlemde sıfır. Yerel sistem düzlem bir tabanda (listenin sistemi ya da projenin TM tanımı) ise IFC'nin sistemi
   tabandır (EPSG:5256 bütün alanlarıyla), dönüşüm yerel düzlemdir: Eastings ve Northings ötelemesi (yerel sıfırın
   harita koordinatı: proje baz noktası ile ölçüm noktası çakışık, EK-7 7.8), X ekseni (cos θ, sin θ), `Scale` 1
   (birimler aynı); yerel ölçek IFC 4.3'ün `IfcMapConversionScaled`'ının FactorX ve FactorY'sine, FactorZ 1 (yükseklik
   ölçeklenmez). Ölçeksiz yerel sistem düz `IfcMapConversion`'dır. Eksenleri dik afin her eksende kendi sütun boyuyla
   yazılır; kaydıran ya da aynalayan afin, coğrafi sistem, düzlem olmayan taban yazılmaz ve nedeni söylenir: yerine
   benzerlik oturtulmaz.
10. **Kot:** OrthogonalHeight dosyanın sıfırının yüksekliğidir: binada ±0.00'ın mutlak kotu (Proje ayarları › Mimari ›
    İmar durumu'ndaki ±0,00 kotu), yolda 0 (yükseklikleri projenin kendi kotları). Bilinmeyen kot 0 sayılmaz: alan zorunlu
    olduğundan dönüşüm de sistem de yazılmaz; sistemi olan projede `export_losses` ("IFC'ye konum … yazılmadı: ±0,00'ın
    mutlak kotu bilinmiyor …") nedenini ve yerini söyler, EK-9'un 6. sorusu nedeniyle "hayır" der. Sistemi hiç olmayan
    proje dışa aktarmada eskisi gibi sessizdir (form söyler); yolun dosyası yerini korur.
11. **Yerleşim zinciri:** saha dünyanın sıfırında mutlak, bina sahaya (kendi ±0.00'ıyla, §2), kat binaya, eleman katına,
    açıklık deldiği elemana, kapı ve pencere açıklığına göre yerleşir; yazıcı hep önce yazılmış varlığa başvurduğundan
    döngü kurulamaz. Bağımsız denetim (`curved_ifc_check.py`'nin `chain_faults`'u, bütün örnek dosyalarda): her ürün
    kendisininkilerden birine göre (IFC'nin kuralınca üst düzeydeki yapı da olur), saha mutlak, döngü yok. Çift öteleme
    mutlak kotlardan ölçülür: katlar haritada, duvarlar katlarının taşıyıcı döşemesinde.
12. **Form ve denetim:** EK-9'un 6. sorusunun "evet"i yerel sistemin dönüşünü, ölçeğini ve başlangıcını söyler;
    `digital_system_wrong`'un onarımı Proje ayarları › Koordinat sistemi'ni (TUREF dilimi, ya da Özel sistem… ile dilimin
    üstünde yerel sistem) ve İmar durumu'nun ±0,00 kotunu gösterir (eskiden ±0,00'ı olmayan Parsel ve bahçe mesafeleri'ni
    gösteriyordu).

**Sınama.** `kentos-ifc`'in `crs::tests`'i (döndürülmüş ve ölçekli sistemde üç nokta projenin kendi düzlem dönüşümünün
koyduğu yerde, 1e-8 m; yazılan satır; ölçeksiz düz dönüşüm; dik eksenli afin; kayma, ayna, doğruya indirme ve sonsuz
değer reddi) ve `quality::tests::a_system_must_be_a_turef_zone_with_its_elevation` (dışa aktarmanın nedeni, dönüş
sözleri). Uygulamada `ifc_place::a_local_model_is_turned_and_scaled_onto_its_base` (üç kontrol noktasından KentOS'un
oturttuğu düzlem bağımsız çözümle aynı; dosyanın sistemi EPSG:5256 TUREF/TM36, Antalya yükseklikleri; her kontrol noktası
yazılan değerlerle anketteki yerine oturtmanın kendi artığı içinde, 1e-6 m; EK-9'un 6. sorusu; yolun dosyası aynı
düzlemde, sıfırı 0'da; uyarı yok) ve `what_the_file_cannot_say_is_said_and_nothing_is_assumed` (±0,00'sız: sistem yok,
dönüşüm yok, tek uyarı, form "hayır", yol yerinde; kaydıran afin reddi; sistemsiz çizim sessiz). Bağımsız durum
`scripts/fixtures/location_cases.py` (sentetik anket: 23,5°, 0,99989, milimetreye yuvarlanmış ve az gürültülü üç nokta;
düzlem `crs_definition_fit_cases.py`'nin tam kesirli çözümüyle) `fixtures/arch/v1/ifc/yerel-sistem.json`'ı yazar;
`curved_ifc_check.py`'nin konum bölümü `yerel-sistem.ifc`'yi IfcOpenShell'le okur: 0 bulgu; `IfcMapConversionScaled`
oturtulan değerlerle (dönüş 23,504951°, ölçek 0,999830907, iki eksende; yükseklik ölçeksiz, 1285,40'tan); üç kontrol
noktası `xyz2enh` ile anketteki yerine artıkları içinde (en çok 0,45 mm) ve `enh2xyz` ile geri; doğu ve kuzey yer
değiştirmemiş; katlar haritada 1285,40 ve 1288,40; duvarlar katlarının taşıyıcı döşemesinde (−0,13 ve 2,93); zincir
sağlam. Zincir denetimi bozulmuş bir kopyada döngüyü ve duvara göre yerleşen katı bulur. Masaüstü ortak sınamaları
(modül, şerit, geçmiş, katalog, ayarlar, IFC, özel sistem). Resim: `ozel-crs-konum-*` (Şantiye, TM36 tabanı, üç kontrol
noktası, m0 ±0,5 mm).

## §4. Ortak bodrum bir kez: emsal, TAKS, metraj, ruhsat takımı

**Araştırma.**

- **PAİY** (Planlı Alanlar İmar Yönetmeliği, 1 Temmuz 2026 tarihli birleştirilmiş metin) 4/1-bbbb: "Taban alanı: Bahçede
  yapılan eklenti ve müştemilatı dâhil **yapıların** tabii zemin veya tesviye edilmiş zemin üzerinde kalan kısmının …
  parseldeki izdüşümünün kapladığı alanı"; 4/1-cccc TAKS taban alanının imar parseline oranı; 20/4: kısmen ya da tamamen
  açığa çıkan bodrumların oturumu taban alanını geçemez. Taban alanı parseldeki bütün yapılarındır.
- **KentOS'ta olan:** emsal ve yapı inşaat alanı mahal mahal, katlarıyla sayılır (ADR 8030): ortak bodrum tek kattır,
  bir kez sayılır. Ama taban alanı adında "zemin" geçen ilk katın (yoksa ±0.00'a en yakın katın) mahallerinden okunuyordu:
  iki blokta B'nin zemini sayılmıyordu. Metrajın kat süzgeci vardı, bina süzgeci yoktu: bir binanın katlarını elle seçen
  ortak bodrumu iki binada sayabilirdi. Ruhsat takımı projenin bütün katlarını çiziyor, bina kodunu künyeden ("00")
  okuyordu. Bina başına IFC'ye (§2) projenin alanları yazılmıyordu, binanınkiler de yazılmıyordu.
- **EK-2 §2.3:** bloklar için ayrı hazırlanan projelerde dosya adında bina kodu.

**Karar.**

13. **Binanın katları** (`BuildingProgram::counted_levels`): binanın kendi katları; birden çok binanın kapladığı kat
    yalnız seçildiği binada. **Binanın zemin katı** (`ground_of`): saydığı katlardan adında "Zemin" geçen, yoksa kendi
    ±0.00'ına en yakın olan.
14. **TAKS:** programda bina varsa taban alanı her binanın zemin katının sayılan mahallerinden; birden çok zemin katta
    oturumların birleşiminin alanı (üst üste binen bir kez), tek zemin katta eskisi gibi mahallerin toplamı.
    `ZoningReport::grounds` her zemin katın kendi alanını tutar.
15. **Binanın payı** (`ZoningReport::of_building`): saydığı katların yapı inşaat alanı ve zemin katının taban alanı;
    binalarınki birlikte projeninkidir, ortak bodrum bir kez. Bina başına IFC'de (§2) GrossPlannedArea,
    GrossFloorArea ve FootprintArea binanın kendi değerleridir (imar durumu verilmişse).
16. **Metraj** (`cad.spaces.quantities` `building`): binanın saydığı katlar; `levels` ile ikisinin kesişimi;
    programda olmayan bina `building_not_found`. Masaüstünde Metraj'ın listesinde, birden çok binada, katlardan önce
    "A Blok (binanın katları)".
17. **Ruhsat takımı** (`PermitSet::building`): birden çok binalı projede pencerede Bina (ilk bina seçili; "Bütün proje"
    de olur): takım binanın saydığı katlarını çizer, kat kodları (EK-2 Tablo 2.14) binanın ±0.00'ından, her pafta kendi
    `bina_kodu`'nu ve `blok_no`'sunu taşır (pafta değişkeni çizimin değişkeninden önce okunur: antet ve dosya adı); yayın
    klasörü bina koduyla ayrı. Bilgi paftası, vaziyet planı ve hesap tabloları parselindir; vaziyet planının haritası
    binanın zemin katını çizer.

**Sınama.** `program::tests::a_building_counts_its_levels_and_stands_on_its_ground` (ortak katın binası, adla ve ±0.00'la
zemin); uygulamada `blocks_areas` (iki blok, ortak bodrumda 11 × 8 m otopark, 2 000 m² parsel):
`the_ratios_count_both_blocks_and_the_basement_once` (zemin katlar A'nınki ve B'nin +1,20'deki katı; taban alanı ikisinin
toplamı; binaların yapı inşaat alanları toplamı projeninki, bodrum yalnız A'da), `each_block_s_model_carries_its_own_areas`
(iki IFC'nin GrossFloorArea ve FootprintArea toplamları projeninki), `metraj_by_building_partitions_the_project_s_rooms`
(her mahal tek binanın metrajında; otopark A'da, B'de yalnız dükkân; `building_not_found`),
`a_block_s_permit_set_is_its_own_storeys_with_its_code` (B'nin takımında yalnız B zemin kat planı, K00, her paftada B1;
programda olmayan bina reddi). Ortak durumlar: `cad.spaces.quantities` 30 (bina süzgeci dört durum, programsız ret;
`space_command_cases.py`'nin başvurusu programın binasının katlarını kendisi sayar). Resimler: `bina-metraj`,
`bina-ruhsat`.

## §5. Parselin plan kararları

**Araştırma.**

- **BIM-11:** "parseli etkileyen plan kararları (fonksiyon, nizam, yaklaşma sınırı, emsal, TAKS, yükseklik) CBS
  tarafından kaynağı, onay tarihi ve plan notuyla gelir. Plan gösterimi ya da bir kesişim tek başına hukuki hesap kuralı
  olmaz; çelişen iki karar kullanıcıya gösterilir."
- **PAİY 4/1-ll:** "İmar durum belgesi: Bir parselin; Kanun, uygulama imar planı, plan notlarını ve bu Yönetmelikte yer
  alan kullanım kararlarını ve yapılaşma şartlarını içeren, yapının inşa edileceği imar parselini sınır ve boyutlarıyla
  gösteren belgeyi"; 55/2 ve 55/4: belgeyi idare verir. **4/1-mm (kademe hattı):** bir imar parseline birden fazla kat
  adedi, yükseklik, yoğunluk, nizam ya da kullanım kararı isabet edince farklı kararların sınırı: iki karara düşen parsel
  kendiliğinden çelişki değildir.
- **KentOS'ta olan:** harita tarafının `imar-plani` profili (ADR 1420, 2020, 5040): plan sınırı (PlanKimlik, PlanAdi,
  Seviye, Durum, Sürüm, OnayTarihi, Dayanak), karar alanı (Kullanim, Nizam, TAKS, KAKS, KatAdedi, Hmax, çekmeler,
  Belirtilmeyen, PlanNotlari), plan notu (NotNo, Metin, Kapsam); İmar planı denetimi (`cad.zoning.check`) ve parçanın
  ortalama genişlik kuralı (ADR 0202 §2). Mimari tarafta İmar durumu elle yazılır (ADR 8030), vaziyet planı parseli tutar
  (ADR 8032).

**Karar.**

18. **`cad.zoning.parcel` v1** (salt okunur, mimari modülü): bir parselin (yoksa vaziyet planınınki) imar planı
    katmanlarıyla kesişimi. Her karar alanının payı, ortak parçaları ortalama genişliği 2A/P toleransı (0,01 m) geçince
    sayılır; değerleri yazıldığı gibi (sayı olarak okunmaz, §23.1); planı (kimlik, ad, seviye, durum, sürüm, onay tarihi,
    dayanak) ve notları (andığı numaralar, sonra planın genel notları). Hiçbir kararın altında kalmayan alan; aynı parça
    üstünde farklı yazan iki karar (çelişki: alan, iki kimlik, iki değer; sayılar sayı olarak karşılaştırılır, "1,20" ile
    "1.2" aynı); imar durumunun emsali ya da TAKS'ı plan kararınınkinden farklıysa ikisi. Notlar: karar katmanı yok,
    birden çok karar (kademe hattı, PAİY 4/1-mm), yürürlükte olmayan, durumu ya da onay tarihi yazılmamış plan, planı
    yazılmamış ya da çizilmemiş karar, çizilmemiş plan notu. Retler `parcel_missing` (yeni), `entity_not_found`,
    `not_a_parcel`, `invalid_input`.
19. **Hiçbir şey yazılmaz:** kesişim hukuki karar değildir; imar durumunun alanları belgeden elle yazılır. Masaüstünde
    Proje ayarları › Mimari › İmar durumu'nun altında "Parselin plan kararları": her karar, kaynağı ve notları; kararsız
    alan, çelişkiler ve imar durumuyla farklar uyarı renginde; altında "kesişim hukuki karar değildir" notu. Komut
    çizimin sürümüne göre bir kez okunur. Şerit düğmesi yok (Python, MCP ve katalogdan çağrılır).
20. Rozet özetinde boş toplamın −0'ı "0,00" yazılır.

**Sınama.** Ortak durumlar `cad.zoning.parcel` 21 (`scripts/fixtures/zoning_parcel_cases.py`, KentOS kodu olmadan,
eksenlere paralel dikdörtgenlerle tam): vaziyet planının parseli iki karara (kaynak, notlar, imar durumuyla fark), planı,
tek karar, üst üste çizilmiş iki kararın çelişkisi, askıdaki ve onay tarihsiz plan ile planın dışı, 5 mm şerit
(sayılmaz; 1 mm toleransla sayılır), katmansız çizim, imar durumsuz ve yalnız emsalli çizim, planı yazılmamış ve
çizilmemiş karar, durumu yazılmamış plan, kapalı çizgi parsel, planın dışındaki parsel, doğrulama, dört ret.
Masaüstünde `zoning_parcel_tests::imar_durumu_lists_the_parcel_s_decisions_with_their_source` (satırlar, kaynak,
uyarı rengindeki fark, alanlar değişmez). Resim `.run/gui-evidence/plan-kararlari-8072/`.

## Ortak sınamalar

- `cargo test -p kentos-contracts --lib program`, `-p kentos-kcad --test all -- arch_elements`,
  `KENTOS_CASES_ONLY=cad.program … every_command_case`, `-p kentos-desktop --lib -- program_scenes`.
- `KENTOS_CASES_ONLY=cad.zoning.parcel … every_command_case capabilities`; `python3 scripts/fixtures/zoning_parcel_cases.py
  --check`; `-p kentos-desktop --lib -- zoning_parcel_tests history::text`.
- `cargo test -p kentos-ifc --lib`, `-p kentos-native-application --test all -- ifc_place arch_ifc blocks_areas`,
  `KENTOS_CASES_ONLY=cad.arch.ifc,cad.arch.audit,file.export.ifc … every_command_case`; `uv run --offline --no-project
  --with mpmath python3 scripts/fixtures/location_cases.py --check`; `curved_ifc_check.py` (IfcOpenShell 0.9.0).

## Sıradaki adımlar

- **Konumun kalanı:** `TrueNorth` (EK-7 7.8; dönüşüm varken bilgi içindir, meridyen yakınsamasını projeksiyondan ister);
  kontrol noktalarının projede saklanması (pencere düzlemi hesaplar, noktalar yazıldıkları sürece durur) ve dışa
  aktarmada yeniden denetlenmesi; idarenin ölçüm toleransı kurum profiliyle; gelen IFC'nin zinciri okuyucuyla (IFC-07).
- **Alanların kalanı:** vaziyet planının haritası birden çok binanın zemin katını birlikte çizmez (harita tek kat
  çizer); bilgi paftasındaki PAİY 57/10 tablosu binanın değil parselin; açığa çıkan bodrumun taban alanına katılması
  (TAKS'ın notu, 20/4) arazi kotunu ister.
- **Plan kararlarının kalanı:** imar durumuna bir kararın değerlerini kaynağıyla (plan, onay tarihi) almak, kaynağı
  imar durumunda saklamak — yapıldı: ADR [8140](8140-plan-decision-values-into-zoning.md) (kararın altında Değerlerini al,
  emsalin ve TAKS'ın kaynağı İmar durumu'nda, Kaydet onaylar); plan notlarının metninden kural çıkarmak yok (müellifin ve
  idarenin işi).
- Denetim bulguları: seçilmeyen ortak kat, birden çok binada kodsuz ya da ±0.00'ı verilmeyen bina (bugün Bina programı
  "Seçilmedi" der, IFC dışa aktarma binası belli olmayan nesneyi söyler; Mimari denetimin program grubu mahal konumlu
  bulgu verir, bina düzeyinde bulgu yeri ister).
