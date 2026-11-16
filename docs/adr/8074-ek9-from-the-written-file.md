# ADR 8074: EK-9'un cevapları yazılmış dosyadan — dosya olguları, Python formu ve katalogdan IDS

- **Durum:** yapıldı (16 Kasım 2026); kalan işler §8'de (son yazılan dosya, beş sonucun raporu, eşlemenin bütçesi,
  Tablo 9.2). Plan: [ifc-uyum.md](../../plan/ifc-uyum.md) IFC-08; [bim-plan.md](../../plan/bim-plan.md)
  sırası B13.
- **Bağlam:** ADR [8047](8047-ifc-building-export.md) §9–§10 (EK-9 formu bugün bellekteki modelden, `kentos_ifc::quality`;
  Mimari denetimin Dijital proje bulguları `digital_audit.rs`), [8062](8062-digital-project-rule-catalog.md) (kural
  kataloğu), [8064](8064-ek3-ek4-2d-checks-python.md) (kararı Python verir, Rust ölçer; bim-plan K10),
  [8067](8067-building-3d-view-and-room-overlaps.md) (mahal örtüşmesi), [8068](8068-fire-performance-material-declarations.md)
  (zorunlu alanlar `kentos_ifc::required`), [8061](8061-ifc-viewer-and-reading-engine.md) §11 ve
  [8073](8073-ifc-viewer-first-slice.md) (okuma yüzü, görüntüleyici).
- **Dayanak:** Dijital Hazırlama Yönetmeliği Md. 15 ve EK-9 Tablo 9.1 (mimari, 21 soru) ve 9.2 (öbür disiplinler, 16 soru)
  ([mevzuat/bim/ek-9-kalite-kontrol-formu.md](../../mevzuat/bim/ek-9-kalite-kontrol-formu.md), PDF s. 238–241); Md. 6,
  10–14; EK-1, EK-2, EK-3, EK-5, EK-6, EK-7.

## §1. Araştırma

| Kaynak | Ne diyor | KentOS kararı |
|---|---|---|
| EK-9 Tablo 9.1 ve 9.2 | Formun her satırı bir Evet/Hayır sorusudur; mimari 21, öbür disiplinler 16 soru | Formun resmî cevabı (Evet, Hayır, boş) ile KentOS'un kanıt durumu (geçti, kaldı, inceleme) ayrı saklanır; inceleme Evet'e çevrilmez |
| buildingSMART IDS 1.0 ([kullanım kılavuzu](https://github.com/buildingSMART/IDS/blob/development/Documentation/UserManual/specifications.md), [şema](https://raw.githubusercontent.com/buildingSMART/IDS/development/Schema/ids.xsd)) | Kök `ids` (ad alanı `http://standards.buildingsmart.org/IDS`): `info`, `specifications`; her `specification`'ın `ifcVersion`'ı (IFC2X3, IFC4, IFC4X3_ADD2), `applicability`'si ve `requirements`'i; altı yön sırayla `entity`, `partOf`, `classification`, `attribute`, `property`, `material`; gereklilikte `cardinality` (required, optional, prohibited); değer `simpleValue` ya da `xs:restriction`. Geometri, çakışma, hesaplanan değer ve dosyalar arası uyum IDS'in dışındadır | Katalogun zorunlu satırlarından IDS üretilir (§5); geometri (kapalılık, örtüşme, kot), dosyalar arası uyum ve adların kalıbı formun kendi denetimlerindedir |
| buildingSMART doğrulama servisi ([kılavuz](https://buildingsmart.github.io/validate/user/index.html)) | Şema ve normatif kurallar; 256 MB sınırı; ülkeye özgü kuralı denetlemez | Gerçek ruhsat modeli dışarı yüklenmez; denetim çevrimdışıdır |
| IfcTester ([PyPI](https://pypi.org/project/ifctester/0.9.0/), 0.9.0, 29.9.2026) | IDS'i okur (IDS 1.0 şemasıyla doğrulayarak), bir IFC'yi ona karşı denetler, raporu JSON, HTML, BCF verir. Öznitelik yönünde boş değer, yön isteğe bağlı olsa da kalır; varlık adı tam eşleşir, alt türleri kapsamaz | Üretilen IDS'in bağımsız denetleyicisi (sahibin 16 Kasım onayıyla indirildi, uv önbelleğinde; ürünle dağıtılmaz); koşullu satır kendi tanımında, ölçekli dönüşüm adıyla (§5) |

## §2. Karar

1. **Ölçen Rust, karar veren Python** (bim-plan K10). `kentos-ifc` yazılmış dosyanın olgularını verir (`read::facts`,
   `kentos.ifcFacts` 1): dosya (adı, uzantısı, boyutu, başlığı, şeması), birimler, konum, her nesne (GlobalId, sınıf,
   öznitelikleri, ağaçtaki yeri, katı, malzemesi, setleri), EK-7'nin kaynak kayıtları (kişi, kuruluş, rol, koordinat
   sistemi, harita dönüşümü, bağlam), okuma bulguları, katalogun zorunlu satırlarının dosyada karşılanmayanları
   (`gaps`), sınıf kodları (EK-5), gövde ölçüleri (mahallerin kapalılığı ve örtüşmesi, elemanların kot aralığı).
   Formun 21 cevabı `dijital_proje.form`'dadır.
2. **Kaynak modelin beklentisi ev sahibinden.** Uygulama, dosyayı yazdığı modelin nesnelerini (kalıcı kimlik, GlobalId,
   sınıf) ve dışa aktarımın bıraktıklarını olgulara ekler (`expected`, `export`). EK-9/8 (zorunlu sınıflar) ve 17
   (doğru sınıf) dosyayı bu beklentiyle karşılaştırır: tamamen eksik nesne IDS geçse de bulgudur.
3. **Yol:** masaüstünde Mimari denetimin standart paketi çağrısı (`standards::call`, ADR 8064): çizimin olgularının yanında
   ikinci girdi dosyanın olguları. Uzantı sisteminin olgu sözleşmesine (`kentos.auditFacts`) dokunulmaz: o dosyalar başka
   bir işin elindedir. Dosya bellekte yazılıp okunur; diske yazılmaz.
4. **Bulgu:** her biri nesneyi (GlobalId ve kalıcı kimlik), sınıfı, alan yolunu, maddeyi ve eki, ölçülen ve istenen değeri
   ve düzeltmenin yerini söyler. Planda ve 3B görünümde nesneyi seçer; IFC modeli penceresi açıksa boyar.
5. **Taşıma:** `digital_audit.rs`'in bulguları (`digital_project_info_missing`, `digital_project_code_missing`,
   `digital_space_unnamed`, `digital_system_wrong`, `digital_no_spaces`, `digital_model_loss`) Python'a geçer, aynı
   kodlarla; `cad.arch.audit` artık onları vermez.

## §3. Uygulama

1. **Dosyanın olguları** (`kentos-ifc`, `read::facts`, `read::file_facts`; `geometry` özelliği gövdeler için): dosya
   (ad, uzantı, bayt, başlık, şema), birimler, konum, her nesne (GlobalId, sınıf, ad, uzun ad, açıklama, nesne tipi, ön
   tanımlı tip, etiket, ağaçtaki yeri, katı, kotu, malzemeleri, kendi ve tipinin setleri), EK-7'nin kaynak kayıtları
   (`IfcPerson`, `IfcPersonAndOrganization`, `IfcOrganization`, `IfcActorRole`, `IfcPostalAddress`, `IfcProjectedCRS`,
   `IfcMapConversion` — ölçekli dönüşüm de, kendi sınıfıyla —, `IfcGeometricRepresentationContext`), EK-7'nin katalog
   satırları, katalogun zorunlu satırlarının dosyada karşılanmayanları (`gaps`, `kentos_ifc::required` ile aynı kural),
   IFC 4.3 sözlüğüne göre set denetimi (`setChecks`), EK-5 kodları (`classCodes`), formun andığı ek satırlarının kaynak
   yazısı (`sources`: EK-1 m. 1.1.1, EK-2 Tablo 2.1, EK-3 Tablo 3.1, EK-5 m. 5.1, 5.4, 5.5, EK-7 Tablo 7.1, 7.6, 7.7 ve
   EK-9'un 21 sorusu; kataloglardan, sayfalarıyla), okuma bulguları ve gövdeler (`bodies`: her gövdenin kot aralığı,
   plandaki kutusu ve float32 sınırı; mahallerin alanı, kapalılığı ve aynı kattaki örtüşmeleri).
2. **Kaynağın beklentisi** (`kentos_ifc::expected`): yazılacak binanın KentOS kimliği olan her nesnesi, yazılacağı GlobalId,
   gösterdiği KentOS nesnesi (bacanın çatısı gibi) ve IFC sınıfı. Dosyanın tohumundan kimlik alan parçalar (kimliksiz
   proje, saha, bina) listede yoktur: çizimde adlandırılacak nesneleri yoktur.
3. **Masaüstü** (`apps/desktop/src/arch/ifc_facts.rs`): modelin eşlemesi pencerenin iş parçacığında
   (`arch_ifc::building`), dosyanın bellekte yazılması, geri okunması, gövdeleri ve olguları kuralın iş parçacığında.
   Olgulara ev sahibinin bildikleri eklenir: `expected` (kalıcı kimlik, GlobalId, sınıf), `export` (`left`: modele
   girmeyenler nedenleriyle; `crsWhy`: konum yazılmadıysa nedeni; `memory`: dosya bellekte yazıldı), `at` (nesnesiz
   bulgunun çizimdeki yeri: binanın ilk duvarı). Duvarsız çizimin olgusu yoktur.
4. **Mimari denetim:** `dijital_proje.mimari.denetle(cizim, dosya)`; dosya verilince EK-9'un soruları dosyadan, çizimin
   örtüşme ve zorunlu alan kuralları (`kalite`, `zorunlu`) yinelenmez. Pakete ait bulgular `digital_` ile başlar.
   Denetim'de seçilen bulgunun nesneleri IFC modeli penceresinde (açıksa ve dosya çizimin dosyasıysa) bulgu rengine
   boyanır; 3B görünüm onları zaten boyuyordu.
5. **Çizimdeki form** (Model kalite kontrol formu, `arch.qualityForm`; Ruhsat takımı'nın FR/MKF paftası): satırları
   `dijital_proje.form.tablo`'dan: başlık (`kentos_ifc::quality::FORM_HEADER`, paketin `BASLIK`'ı; bir test ikisini
   karşılaştırır), grup satırları, 21 soru, resmî cevap ve KentOS'un notu. Yerleştirme pencerenin iş parçacığını
   bekletmez (satırlar gelince imleçle yerleşir); Ruhsat takımı tabloları yenilerken satırları çağıran verir
   (`calc_tables::ensure`'un `form_rows` girdisi).
6. **Taşınanlar:** `digital_audit.rs` ve testleri, `kentos_ifc::quality`'nin 21 cevabı (yalnız tablo başlığı kaldı),
   `arch_ifc::form_rows`. `cad.arch.audit` artık `digital_*` bulgusu vermez; katalog özeti ve sözleşme notu bunu söyler.
   Kodlar (`digital_project_info_missing` …) aynıdır.
7. **Yazıcının EK-7'si** (dosya bu sorularla sınanınca görülen eksikler):
   - kişi, kuruluş ve kişi-kuruluş bağı rollerini taşır (`Roles`: `IfcActorRole` ARCHITECT, açıklaması unvan); rol kaydı
     önce yazılır;
   - `IfcProject.Description` künyenin yeni kutusundan: **Proje açıklaması (IFC modelinde)** (`proje_aciklama`, Ruhsat
     takımı › Dijital teslim (EK-1, EK-2, EK-7)); boşsa yazılmaz, soru Hayır kalır;
   - `Pset_ProjectCommon.ProjectType` künyenin Dijital teslim kapsamı › Yapı'sından (K50): yeni bina `NEWBUILD`, ilave
     ve tadilat `MODIFICATION`; seçilmemişse yazılmaz. Künyenin "yeni bina" sözü `ConstructionMethod`'a da artık "Yeni
     yapı" diye geçer (önce olduğu gibi geçiyordu).

## §4. Soru soru

Kanıt: g geçti, k kaldı, i inceleme (resmî cevap Evet, Hayır, boş). Kodlar paketin; her bulgu nesnesini GlobalId'si ve
çizimdeki kimliğiyle verir.

| No | Dosyadan okunan | Kanıt | Bulgu kodu |
|---|---|---|---|
| 1 | Dosyanın boyutu; Bakanlığın sınırı yayımlanmadı | i | — |
| 2 | EK-7 Tablo 7.1–7.5'in zorunlu öznitelikleri (proje, kişi, kuruluş, rol); eksiğin künyedeki kutusu | g/k | `digital_project_info_missing` |
| 3 | Dosya adı EK-2 Tablo 2.1'in dokuz alanında, proje kodu verilmiş, başlıktaki ad dosyanın adı | g/k | `digital_project_code_missing`, `digital_file_name_wrong` |
| 4 | Mahal numarası ve adı, adlar tekil, EK-3 Tablo 3.2'nin yazımı | g/k | `digital_space_unnamed`, `digital_space_name_duplicate`, `digital_ek3_room_name` |
| 5 | Ad kalıbı MM-<kod>-<açıklama>; açıklaması boş ad inceleme (K49) | g/i/k | `digital_entity_name_wrong`, `digital_entity_name_no_description` (uyarı) |
| 6 | TUREF 3° dilimi, EK-7 Tablo 7.6–7.7'nin alanları; yerel sistemin dönüşü, ölçeği, ±0.00 | g/k | `digital_system_wrong` |
| 7 | EK-8'in gelişim seviyesi müellifin | i | — |
| 8 | Kaynağın her nesnesi dosyada; vekil sınıf yok | g/k (kaynak yoksa i) | `digital_model_object_missing`, `digital_proxy_class` |
| 9 | EK-6 ve EK-7'nin zorunlu satırları; projenin kendi alanları 2. sorunun | g/k | `digital_required_field` |
| 10 | Emsal zonları (`IfcSpatialZone`) ve durumları; tabloyla karşılaştırma müellifin | k/i | `digital_emsal_zones_missing` |
| 11 | Proje, saha, bina, en az bir kat | g/k | `digital_structure_missing` |
| 12 | Mahal var | g/k | `digital_no_spaces` |
| 13 | Aynı kattaki örtüşme (0,01 m² eşiği), mahalin tabanı katının aralığında | g/k | `digital_quality_room_overlap`, `digital_space_wrong_storey` |
| 14 | Mahal gövdesi kapalı, üstü üst katın kotunu aşmıyor | g/k | `digital_space_open`, `digital_space_top_wrong` |
| 15 | Her nesnenin katı var, gövdesi katının aralığında | g/k | `digital_entity_no_storey`, `digital_entity_wrong_storey` |
| 16 | Yinelenen GlobalId; aynı sınıf, ad ve katta aynı yerde iki gövde | g/k | `digital_duplicate_global_id`, `digital_duplicate_entity` |
| 17 | Sınıf kaynağın eşlemesiyle aynı, vekil yok | g/k (kaynak yoksa i) | `digital_class_wrong`, `digital_proxy_class` |
| 18 | Şema IFC4X3_ADD2, uzantı .ifc; bellekte yazılan dosyada teslim edilen müellifin | g/i/k | `digital_version_wrong` |
| 19 | NOTDEFINED tip yok; USERDEFINED tipin adı var | g/k | `digital_predefined_notdefined`, `digital_predefined_userdefined_unnamed` |
| 20 | Standart setler IFC 4.3'te, sınıflarına uygun, özellikleri setlerinin | g/k (başka sürümde i) | `digital_pset_wrong` |
| 21 | Dışa aktarımın modele almadığı nesne yok | g/k (kayıt yoksa i) | `digital_model_loss` |

Bulgunun iletisi maddeyle biter ("— Dijital Proje Yön. 13"); kaynağı madde, Resmî Gazete, ek satırları ve formun sorusu
sayfalarıyla: "Dijital Proje Yön. 6 · RG 5.8.2026/33331 · EK-1 m. 1.1.1 · PDF s. 1 · EK-2 Tablo 2.1 · PDF s. 3 · EK-9
Tablo 9.1 satır 3 · PDF s. 238". Onarımlar menünün gerçek yolunu söyler (Mimari › Proje › Ruhsat takımı'nın künyesi;
Dosya › Proje ayarları › Koordinat sistemi; Öznitelikler › Yangın dayanımı …).

## §5. Katalogdan IDS

`kentos_ifc::ids::write` profilin zorunlu satırlarını IDS 1.0 olarak yazar: crates/shared/ifc/data/temel-2026.ids (77
tanım, 356 gereklilik; bir test dosyayı güncel tutar, `KENTOS_WRITE_IDS=1` yazar). Her EK-6 ve EK-7 tablosu kendi sınıfı
için bir tanımdır (`ifcVersion` IFC4X3_ADD2; `applicability` sınıf, en az 0: sınıf tasarımda varsa sorulur, EK-5 §5.1);
satırlar:

- öznitelik `attribute` (gerekli); ön tanımlı tip, sınıfın sayımının NOTDEFINED dışındakilerinden biri;
- set özelliği ve miktarı `property` (gerekli), veri tipi IFC 4.3 şablonunun (miktarda türünün ölçüsü: Q_LENGTH →
  IFCLENGTHMEASURE …); ulusal setlerde (`TREpys_…`) veri tipi yazılmaz, şemanın değildir;
- malzeme `material` (gerekli);
- `IfcRelAssignsToGroup>IfcSystem` bir `partOf`: grup IfcSystem ya da alt türü.

- **Koşullu satır:** kullanıcı tanımlı rol (Tablo 7.5) kendi tanımındadır: yalnız `Role`'ü USERDEFINED olan rollere
  uygulanır ve orada gereklidir (K48; IFC'nin WR1'i). IDS 1.0'da öznitelik yönünün "isteğe bağlı"sı bunu söyleyemez:
  denetleyici boş özniteliği isteğe bağlı yönde de kaldırır.
- **Her modelde olan sınıflar** (K51): EK-7 Tablo 7.1–7.8'in sınıfları (proje, kişi, kişi-kuruluş, kuruluş, rol,
  koordinat sistemi, harita dönüşümü, bağlam) en az bir kez bulunur (`minOccurs="1"`); 7.9'un ölçüm notu
  (`IfcAnnotation` SURVEY) ve EK-6'nın sınıfları yalnız varsa sorulur.
- **Ölçekli harita dönüşümü:** IDS varlığı adıyla eşler, alt türünü kapsamaz; harita dönüşümünün tanımı yerel sistemin
  ölçekli dönüşümüne (`IfcMapConversionScaled`, ADR 8072 §3) de uygulanır. Öbür alt türler katılmaz: bağlamın alt
  bağlamları alanlarını üst bağlamdan türetir.

Her gerekliliğin `instructions`'ı satırın Türkçe adı, tablosu ve satırıdır. scripts/fixtures/ids_check.py (yalnız Python'un
kendi XML okuyucusu) dosyanın IDS 1.0 yapısını (info, specifications, yönlerin sırası, cardinality, dataType kalıbı)
denetler ve gerekliliklerini KentOS'un dört dosyasının olgularına uygular: eksikler KentOS'un kuralınınkiyle satır ve
nesne düzeyinde aynıdır (kesit binası 45, künyeli 30, yerel sistem 44, kapısı eksik 45).

**Bağımsız denetleyici** (scripts/fixtures/ids_ifctester_check.py; IfcTester 0.9.0, IfcOpenShell 0.9.0.post1): IDS'i
IDS 1.0 şemasıyla doğrulayarak açar (78 tanım) ve dört dosyada koşar. Bulduğu her eksik, KentOS'un kuralının satırı
ve nesnesiyle aynıdır; EK-7 kayıtlarının eksikleri (2., 6. ve 9. soruların okuduğu) ve hiç bulunmayan EK-7 sınıfları da
aynıdır; KentOS'un ölçmediği satırlarda (grup, yerleşim gibi öznitelikler) IfcTester'ın kaldırdığı bir şey yoktur.

| Dosya | Eksik satır (EK-6, EK-7) | EK-7 kaydında eksik | Hiç bulunmayan EK-7 sınıfı | Ayrışan |
|---|---|---|---|---|
| Kesit binası | 45 | 1 (bağlamın gerçek kuzeyi) | 6 | 0 |
| Künyeli kesit binası | 30 | 1 | 0 | 0 |
| Yerel sistem | 44 | 1 | 4 | 0 |
| Kapısı eksik | 45 | 1 | 6 | 0 |

İlk koşuda ayrışan üç satır vardı, üçü de düzeltildi:

- **Arazi kimliği ve tapu kimliği** (`Pset_LandRegistration.LandID`, `LandTitleID`, Tablo 6.55): IFC 4.3'ün şablonu
  `IfcIdentifier` ister, KentOS `IfcLabel` yazıyordu. Yazıcı düzeltildi; `catalog_schema::mistyped` yazılacak binanın her
  standart özelliğini ve miktarını şablonun türüyle karşılaştırır, uygulamanın testi kesit binasında boş liste ister.
- **Kullanıcı tanımlı rol:** IDS satırı olduğu gibi zorunlu sayıyordu; K48 koşullu tanımla yazıldı.
- **Bağlamın gerçek kuzeyi** (Tablo 7.8 `TrueNorth`) iki tarafta da eksikti ama formun hiçbir sorusu söylemiyordu: 9. soru
  artık bağlamın kayıttaki zorunlu alanlarını da sayar ("KentOS bu alanı IFC'ye henüz yazmıyor").

IfcTester'ın satır ve nesne sonuçları fixtures/ifc/v1/ids-sonuc/'dadır; paketin testi KentOS'un olgularını onlarla
karşılaştırır (IfcTester olmadan da koşar).

## §6. Ölçümler

Testler (16 Kasım 2026, bu dal): paketin testleri (form 27, öbür altı dosya), `kentos-ifc` (olgular, beklenti, IDS, okuma 26),
uygulamanın IFC ve denetim testleri (29), `cad.ifc.inspect`'in 37 durumu, masaüstünde formun, Ruhsat takımının,
denetimin, 3B görünümün, IFC modelinin ve kesişen filtrelerin testleri; IfcOpenShell'in künyeli dosyayı okuması ve
denetleyicisi (0 bulgu).

Taban (Apple M5, release, Python 3.15.1; ölçüm testi crates/native/headless/tests/headless/ifc_timing.rs; kabul akışının
2+1 dairesi onarlı satırlarla çoğaltılmış, çizelgeler bir kez):

| Daire | Nesne | Duvar | Mahal | Model eşlemesi ms | Yazma ms | Dosya MB | Okuma + gövde + olgu ms | Tepe bellek MB | Olgu MB | Form ms |
|---|---|---|---|---|---|---|---|---|---|---|
| 1 | 44 | 5 | 5 | 1,6 | 0,2 | 0,04 | 18,3 | 34 | 0,04 | 30 |
| 20 | 842 | 100 | 100 | 82,1 | 3,9 | 0,75 | 76,8 | 54 | 0,50 | 34 |
| 100 | 4 202 | 500 | 500 | 3 108,8 | 19,4 | 3,85 | 323,5 | 158 | 2,43 | 54 |

Formun ilk yazılışı 100 dairede 421 ms idi: katlar her bantta yeniden sıralanıyor, yinelenen gövde her kümeyle
karşılaştırılıyordu. Katlar bir kez sıralanır, kümeler sınırdan küçük olmayan bir ızgarada aranır: 15 ms (süreç
başlangıcıyla 54 ms), cevaplar aynı. Model eşlemesi (`arch_ifc::building`, B13'ten önce de vardı) 20'den 100 daireye 38
kat büyür; Denetle her seferinde iki kez kurar (çizimin olguları ve dosyanın olguları; önce çizimin olguları ve Rust
`digital_audit`): sayı aynı, bütçe ve iş parçacığından çıkarma sıradaki iştir (§8).

## §7. Kabul

| İlk kabul (IFC-08) | Kanıt |
|---|---|
| 21 sorunun her biri için geçen ve kalan örnek | 18 soru iki yönüyle (gerçek dosyalar ve tek değişiklikli olgular, test_form.py); 1 ve 7 doğası gereği inceleme; 10 kalan ya da inceleme (karşılaştırma müellifin) |
| Üç kapıdan biri ve başvuruları çıkarılır; şema geçerli, IDS olumlu iken EK-9/8 eksik kapıyı kaynak kimliğiyle bulur | scripts/fixtures/ifc_missing_door_case.py IfcOpenShell'le K02 kapısını ve yalnız onun andığı 18 kaydı çıkarır; denetleyici (EXPRESS kurallarıyla) 0 bulgu; kalan nesnelerin eksikleri (IDS'in okuduğu) aynı; EK-9/8 kaldı, bulgunun nesnesi `0192f5a1-4242-7000-8000-00000000000e`. Kesit binasının kapılarında yangın dayanımı ve acil çıkış yok: IDS'in kapı satırları olumlu değil, değişmedi |
| Bulgu planda ve 3B'de nesneyi seçer | Denetim'in bulgusu nesnelerini seçer; 3B görünüm ve IFC modeli onları boyar (test: duvarların yangın dayanımı bulgusu IFC modelinde yalnız o duvarları boyar) |
| İkinci bağımsız denetleyiciyle ayrışan satır kalmaz | IDS denetleyicisi: IfcTester dört dosyada KentOS'un kuralıyla satır ve nesne düzeyinde aynı (§5; ilk koşuda bulduğu yazıcı hatası düzeltildi). İkinci okuyucu: scripts/fixtures/ifc_form_reference.py IfcOpenShell'le dört dosyanın olgularını kendi yoluyla kurar (nesneler, kayıtlar, katalog verisinden zorunlu alan kuralı, IfcOpenShell'in IFC 4.3 şablonlarıyla set denetimi, kendi üçgenlemesinden gövdeler, üçgen üçgen örtüşme); form iki okuyucuda 21 satırın kanıtında, bulguların kodunda ve nesnelerinde aynı (kesit binasında 9. sorunun 43 bulgusu, 127 nesne; test_form.py). |

Resimler: .run/gui-evidence/ek9-dosyadan-8074/ (açık ve koyu): Denetim › Mevzuat'ta formun bulgusu; IFC modelinde
bulgunun duvarları; çizimdeki form iki yarısıyla; Ruhsat takımı'nın yeni kutusu.

## §8. Açık işler

- **Gerçek kuzey** (EK-7 Tablo 7.8 `TrueNorth`): yazıcı yazmıyor; kuzey okundan ve projenin sisteminin meridyen
  yakınsamasından (IFC-02).
- **Son yazılan dosya:** Denetim'in bellekteki dosya yerine diske yazılan ya da teslim edilecek dosyayı okuması (K4'ün
  "diske yazılan"ı); bulut işinin aynı betiği çalıştırması.
- **Beş sonucun ayrı raporu** (ifc-uyum §1: okunuyor, şema, Türkiye profili, model–pafta, idare): ruhsat paketinde (B14).
- **Model eşlemesinin bütçesi:** 100 dairede 3,1 s, pencerenin iş parçacığında; ölçüm §6'da.
- **Birden çok binalı program:** form bütün projenin dosyasını okur; binanın kendi dosyası (ADR 8072 §2) için bina başına
  form B14 ile.
- **EK-9 Tablo 9.2** (öbür disiplinlerin 16 sorusu): disiplin modelleri geldiğinde.

## §9. Kararlar

- **K48 — Kullanıcı tanımlı rol yalnız rol USERDEFINED iken aranır.** EK-7 Tablo 7.5 `UserDefinedRole`'ü zorunlu satır
  olarak sayar; IFC 4.3'te öznitelik yalnız `Role` USERDEFINED iken verilir ve o zaman verilmek zorundadır (`IfcActorRole`
  WR1; [buildingSMART IFC4.3.x-development, IfcActorRole](https://github.com/buildingSMART/IFC4.3.x-development/blob/master/docs/schemas/resource/IfcActorResource/Entities/IfcActorRole.md)).
  KentOS ARCHITECT yazar; 2. soru bu alanı ancak rol USERDEFINED ise ister.
- **K49 — Açıklaması boş ad inceleme konusudur.** EK-5 Tablo 5.3 şablonun üç alanını sayar, 5.5.3 açıklamanın karakter
  sınırı olmadığını söyler; alanın boş kalıp kalamayacağı yazılı değil. KentOS stilsiz duvarı `MM-DVR` yazar (ADR 8047 §3;
  IfcOpenShell denetimi açıklamayı isteğe bağlı okur). 5. soru disiplini ya da kodu yanlış adı kaldı sayar; yalnız
  açıklaması boş adı inceleme sayar ve nesnelerini uyarıyla listeler. İdarenin yazılı cevabı gelince karar veridir.
- **K50 — Projenin tipi yapı türünden.** EK-7 Tablo 7.1 `Pset_ProjectCommon.ProjectType`'ı ("bir projenin ek tip
  tanımlaması") zorunlu sayar; IFC 4.3'te sayımlı değerdir (`PEnum_ProjectType`: NEWBUILD "yeni bir tesisin geliştirilmesi",
  MODIFICATION "bir tesisin değiştirilmesi", RENOVATION, REPAIR, OPERATIONMAINTENANCE;
  [buildingSMART IFC4.3.x-development, PEnum_ProjectType](https://github.com/buildingSMART/IFC4.3.x-development/blob/master/docs/schemas/core/IfcKernel/PropertyEnumerations/PEnum_ProjectType.md)).
  KentOS künyenin Yapı'sından yazar: yeni bina NEWBUILD; ilave ve tadilat mevcut yapının değiştirilmesidir, MODIFICATION.
  Yenileme (RENOVATION) ve onarım (REPAIR) ruhsat türü olarak künyede yok; seçilmemiş yapı türünde değer yazılmaz.
- **K51 — EK-7'nin sınıfları her modelde.** EK-7 7.1–7.4 projenin, kişinin, kuruluşun, rolün, koordinat sisteminin ve
  harita dönüşümünün zorunlu sınıflar olduğunu söyler; bağlam her modelde vardır. IDS bu sınıfları en az bir kez ister
  (`minOccurs="1"`). Tablo 7.9'un `IfcAnnotation`'ı (ön tanımlı tipi yalnız SURVEY: ölçüm noktası, kot eğrisi) ancak
  modelde ölçüm notu varsa sorulur. Yerel koordinatlı modelde koordinat sistemi ile harita dönüşümünün yokluğu IDS'te de
  bulgudur; 6. sorunun Hayır'ıyla aynı.
