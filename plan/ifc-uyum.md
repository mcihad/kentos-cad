# KentOS IFC uyum ve teslim planı

**Araştırma tarihi** 15 Kasım 2026 · **Kod tabanı** `4c1c0bd0c` · **Durum** Uygulama planı

**Eş belge** [Mimari BIM geliştirme planı](bim-plan.md) · **Mevcut karar** [ADR 8047](../docs/adr/8047-ifc-building-export.md)

## 1. Hedef ve kabul ayrımı

KentOS'un mimari dışa aktarımı, kapsama giren nesneleri ve gerekli veriyi temsil eden, şeması geçerli ve bağımsız araçla okunmuş bir IFC 4.3 teslim dosyası üretir. Bu dosya aynı proje revizyonunun PDF/A paftaları ve EK-9 formuyla izlenebilir. Taslak IFC dışa aktarımı mevcut kullanıcı akışında kullanılmaya devam eder. Ruhsat teslimine hazır durumunu dosya ve kanıt kontrolleri belirler. Müellif ve idare, tasarımın hukuki ve mesleki uygunluğu hakkında bağımsız karar verir.

Beş kabul sonucu ayrı raporlanır. **STEP okunabilirliği, IFC şeması/EXPRESS, Türkiye teslim profili, model–pafta uyumu ve idare kabulü.** Kullanılan noktanın veri kaynağı ve onayı genel sonuçtan görülebilir. [Hazırlama yönetmeliği md. 3–5, 9–15 ve geçici maddeler](https://www.lexpera.com.tr/resmi-gazete/metin/mimarlik-ve-muhendislik-projelerinin-dijital-olarak-hazirlanmasi-hakkinda-yonetmelik-33331).

## 2. Standart, biçim ve teknik sınır

| Konu | Plan kararı |
| --- | --- |
| Şema | Ruhsat yazıcısının hedefi **IFC 4.3 ADD2, 4.3.2.0**, STEP başlığında `IFC4X3_ADD2`. EK-5 §5.4 IFC **4.3**'ü esas alır. ADD2 seçimi yayımlanmış şema ve mevcut KentOS koduyla uyumlu uygulama kararıdır. TS EN ISO 16739-1'in ilgili TSE baskısı ve kurum kabul ettiği şema tanımlayıcısı pilotta kaydedilir. [ISO 16739-1:2024](https://www.iso.org/standard/84123.html) · [Resmî IFC 4.3.2.0 yayını](https://standards.buildingsmart.org/IFC/RELEASE/IFC4_3/) |
| Değişim dosyası | ISO 10303-21 STEP fiziksel dosya, `.ifc`, dolaşan referanslar, türler, karakter kodlaması, sonlu SI sayılar ve başlık doğrulaması. Kalıcı UUID'nin IFC sıkıştırması ile her `IfcRoot` için geçerli ve benzersiz GlobalId. `#n` başvuruları dosya içi ve revizyonlar arası karşılaştırmada geçici. [IFC giriş ve seri hale getirme](https://standards.buildingsmart.org/IFC/RELEASE/IFC4_3/HTML/content/introduction.htm) |
| Görünüm tanımı | Mevcut dosyada `DesignTransferView` başlığı var ve açıklığa `Body` veriliyor. EK-5 ile belirlenen şema ve MVD farklı profillerdir. Yayın hedefinde MVD metnini, seçilen gerçek temsil biçimini ve okuyucu davranışını birlikte doğrula. İdare farklı bir görünüm isterse onu ayrı test profiliyle uygula. Başlıkta yazan adın tek başına sertifikasyon değeri yoktur. [buildingSMART MVD ve IFC yayınları](https://standards.buildingsmart.org/) · [ADR 8047](../docs/adr/8047-ifc-building-export.md) |
| Diğer biçimler/sürümler | IFC4 ve IFC2x3 referans alımı ileriki bir iş paketi. IFCZIP, IFCXML ve diğer dışa aktarım profilleri için ayrı alıcı ihtiyacı ve destek kaydı. Ruhsat düzeyi doğrulayıcı, yanlış sürüm/uzantıyı açık hata olarak bildirir. |
| Kod sahipliği | IFC şeması, okuyucu/yazıcı, eşleme, isim, koordinat, doğrulama ve IDS `kentos-ifc` içinde. `application::arch_ifc` belge verisini uyarlar ve dosya/komut akışını yönetir. `kentos-sheet` pafta üretir. Yolun `IfcAlignment` desteği ve ayrı dosya davranışı aynı çekirdekte korunur. [Çekirdek](../crates/shared/ifc/src/lib.rs) · [Yol kararı](../docs/adr/0908-road-stakeout-sheets-and-exchange.md) |

## 3. Yönetmelik eklerinin uygulanması

| Ek | Kaynak ve kapsam | Uygulama ve çıkış kanıtı |
| --- | --- | --- |
| **EK-1** | Proje kodu. Başvuru yılının iki hanesi + idarenin dört haneli sıra numarası. | Kaynaklı gerçek kodu künye, IFC proje adı ve dosya ailesinde kullan. Yer tutucu ile önizlenebilir, kesin pakette eksik olarak raporlanır. |
| **EK-2** | Tablo 2.1 ve disiplin, öğe/alt disiplin, dosya türü/tipi, konum, sıra, revizyon tabloları. | Tek adlandırma kitaplığı. Model örneği `260123_00_MM_XXXX_MD_XXX_NNN_01_000.ifc`, form örneği `260123_00_MM_XXXX_FR_MKF_NNN_01_000.pdf`. Sıra, bina/konum ve revizyon alanı bağlama duyarlı. Gerçek dosya adı, IFC başlığı ve pafta antedi çapraz kontrol edilir. |
| **EK-3** | 2B mahal no/adı, kat, bölüm/fonksiyon ve sıra. | Kat kodu, bölüm/fonksiyon, sıra ve mahal adı alanlarını kaynakta kimlikten ayrı tut. IFC mahal adını EK-5 ile ortak semantik veri üzerinden üret. Format ve sözlük denetimlerini yalnız varlık kontrolü ile sınırlama. |
| **EK-4** | CAD katman, sembol, ölçü, yazı ve çizgi esasları. | 2B üretim ve DXF/DWG aktarımı katman/ölçek/simge kontrolü. EK-4 CAD öğe kodu ile EK-5 IFC kategori kodu için ayrı sözlükler, ör. `DUVA` ve `DVR`. |
| **EK-5** | §5.4, §5.5, Tablo 5.1–5.3. Şema, sınıflar, isimler, mahal kuralları. | Fıkra ve tablo satırı izli sınıf matrisi. Zorunlu/isteğe bağlı ve disiplin/nesne uygulanabilirliği ayrı alan. Modeldeki sınıf, kategori kodu ve adın eşleşmesi. Sözlükte bulunmayan sınıf için idare yorumu veya belgeli eşleme kararı. |
| **EK-6** | Varlık öznitelikleri, özellik setleri, miktarlar ve ilişkiler. | Zorunlu, koşullu ve isteğe bağlı satır envanteri. Kaynak tip, birim, set adı, IFC veri tipi, izinli değer, kullanım koşulu, sorumlu editör ve IFC doğrulama testi. |
| **EK-7** | Proje, kişi/kuruluş/rol, ortak disiplin bilgisi, CRS, dönüşüm ve temsil bağlamı. | Proje ve disiplin kimliği, aktörler, yatay/düşey datum ve hassasiyet; dışa aktarılan her disiplin için ortak alan karşılaştırması. |
| **EK-8** | Md. 14 model gelişim seviyesi. Sayfa numaralı bağımsız döküm LOD 300 bildiriyor. | Kaynak sayfası teyidi, eleman bazlı geometri ve alfanümerik kabul tablosu, görsel inceleme ve uzman kaydı. |
| **EK-9** | Md. 15, Tablo 9.1 mimari 21 soru. | Formu yazılmış model ve PDF/A belgelerine ilişkilendir. Her yanıt gerçek delile veya kayıtlı müellif incelemesine dayanır. |

Ek kaynakları ve erişim durumu [Resmî Gazete EK-1–9](https://www.resmigazete.gov.tr/eskiler/2026/08/20260805-2-1.pdf) · [Depo ADR 8037](../docs/adr/8037-turkish-title-block-sheet-sets-pdfa.md) · [Depo ADR 8008](../docs/adr/8008-ek4-layer-key.md) · [Depo ADR 8027](../docs/adr/8027-ek3-room-numbers-and-ellipse-tags.md) · [Depo ADR 8047](../docs/adr/8047-ifc-building-export.md) · [Bağımsız sayfa ve tablolar indeksi](https://github.com/BerkantACUN/ifc-ruhsat/blob/main/KAYNAKLAR.md).

### 3.1 Kural kataloğu ve yoruma açık satırlar

- Kural girişi `kaynak/yayım/ek/sayfa/tablo/satır`, disiplin, kapsam koşulu, IFC entity ve alan yolu, şema tipi/birimi, kaynak KentOS alanı, zorunluluk, izinli değer, denetim ve test bağlantısı taşır. Bütün disiplinleri içeren bir EK tablosundaki her sınıfı her binada oluşturmak yerine mevcut/kapsama giren elemanın zorunluluğu değerlendirilir. Her sınıf gereğinin bina ve disiplin kapsamı açıkça tanımlanır.
- Bağımsız indekste EK-5 için ek s. 29–34, EK-6'nın ilk 68 tablosu için s. 36–109, EK-7 için s. 222–230 ve mimari EK-9 için s. 238–239 veriliyor. İlk tabloda 69 sınıf ve 68 zorunlu özellik tablosu bulunması olası sayım farkı olarak kaydedilir. Sınıf kimliği üzerinden eşlenir. Sıra numarasından otomatik atama yapılmaz. EK-6 Tablo 6.69–6.135 ve EK-7'nin son tabloları idare profili için ayrıca incelenir.
- Md. 11/2'nin açıklama bileşenlerindeki tire ifadesi ile EK-5 §5.5/5.5.3'ün bağımsız dökümde alt çizgi olarak aktarılan kuralı bir yorum kalemidir. Mevcut `MM-DVR-Dış_duvar_25` davranışını pilotta iki hüküm ve idarenin yazılı uygulamasıyla doğrula. Sınıf adı, kategori, izinli yerel alt disiplin, Türkçe karakter ve açıklama ayırıcıları ayrı sınanır.
- Tablo 6.37'de bildirilen `IfcGroup.Tag` ile şema alanı farkı, farklı enum boşlukları ve `IfcOrganization.Identification` metin alanındaki sayısal vergi kimliği için kaynak/şema fark listesi açılır. Şema alanı uydurulmaz. Girdi ile dosya temsilinin tipi ayrılır. Belirsiz kural müellife ve gerektiğinde idareye görünür.
- EK-6 §6.1.2–3 için bağımsız döküm, daha özel sınıf ve vekil sınıf kısıtına işaret ediyor. Birincil sayfa doğrulanana kadar profil, vekil sınıfa veya mülkü kaybettiren üst sınıfa sessiz düşüşü engeller. Özel sınıfın Türkiye listesinde olmadığı örnekler (giydirme cephe paneli gibi) mühendislik anlamı, yapıdaki temsil ve yazılı kurum yaklaşımıyla çözülür. [Kaynak/şema farkları](https://github.com/BerkantACUN/ifc-ruhsat/blob/main/KAYNAKLAR.md) · [IFC yayımlanmış şema](https://standards.buildingsmart.org/IFC/RELEASE/IFC4_3/).

## 4. KentOS nesnesinden IFC grafiğine

| KentOS kaynağı | IFC hedefi ve mevcut hat | Kapanması gereken iş |
| --- | --- | --- |
| Proje, parsel, bina, kat | `IfcProject` → `IfcSite` → `IfcBuilding` → `IfcBuildingStorey`, `IfcRelAggregates`. Bugünkü `Building` tek saha/bina veri taşıyor. | Bina ve kat kimliği, ortak bodrum, bir parselde birden çok blok, ayrı modellerin ortak proje alanları, geçerli ve tutarlı placement zinciri. Dosya kapsamını kullanıcı seçer. |
| Duvar ve döşeme | `IfcWall` / `IfcSlab`, gövdeler kesit modelinden. Mevcut duvar malzemesi çekirdek katmana indirgenebilir. | Sırayı, kalınlık/ofset ve malzeme yönünü koruyan katman eşlemesi; uygun yerde `IfcMaterialLayerSet` ve `IfcMaterialLayerSetUsage`. Döşeme tipini işlevden, duvarın dış/taşıyıcı hâlini doğrulanabilir veriden türet. |
| Kolon, kiriş, lento | `IfcColumn`, `IfcBeam` ve ilgili şema enum'ları; mimari modelde temsil var. | Taşıyıcılık ve malzeme sınıfı beyanı ile statik referans bağı. Çelik, ahşap, beton ve bilinmeyen değer ayrı. Bilinmeyeni beton dayanımı ile doldurma. |
| Duvar/döşeme boşluğu ve dolgu | `IfcOpeningElement`, `IfcRelVoidsElement` ile host, `IfcDoor`/`IfcWindow` ve `IfcRelFillsElement` ile dolgu. Kapı/pencere IFC'de ayrı nesne. | Kapalı, niş ve kesen açıklık ayrımı, çok katmanlı/eğri host, doğru kısmi derinlik; açılmanın iki kez düşülmemesi. Uygun olduğu yerde kapı/pencere tipleri ve `IfcRelDefinesByType`; tipte ve örnekte öncelik. |
| Mahal ve bağımsız bölüm | `IfcSpace`, katla ayrıştırma ve mahal miktarı/kaplama. Bağımsız bölüm ve zon semantiği sınıf listesiyle doğrulanacak. | Kapalı mahal hacmi, alan ve hacim kaynakları, net/brüt ve emsal farkları, çakışmayan üst-alt kısımlar. Kullanım amacına göre sınır ilişkisi ve ilgili `IfcZone` / `IfcSpatialZone` / izinli gruplama değerlendirmesi. |
| Merdiven, rampa, korkuluk, çatı | `IfcStair`, `IfcRailing`, `IfcRoof` mevcut. Rampa açık bir kayıp örneği. | `IfcRamp` ve gereken alt parçalar, merdiven kolu/sahanlık ayrıntısı, doğru kat bağı, eğik/kapalı gövde ve kalkış–varış. Tablo kapsamı, alt parça ayrıştırma ve çift sayım sınanır. |
| Aks, şaft, asansör, kaplama, donatı ve koordinasyon nesneleri | Adaylar `IfcGrid`, şaft için amaca göre `IfcSpace` ve/veya açıklık, asansör için `IfcTransportElement`, kaplama için `IfcCovering`. | EK-5 sınıf kapsamı ve ilgili disiplin onayı. Bir CAD blok sembolünden kendiliğinden fiziksel sınıf, kapasite veya yangın rolü çıkarılmaz. Gerçek kayıp ve bilinçli kapsam dışı ayrıdır. |

Yapı elemanının birincil mekânsal sahibi `IfcRelContainedInSpatialStructure` ile kurulur. Çok kata uzanan elemanlarda birincil sahip ve gerekiyorsa ek mekânsal referans değerlendirilir. İhraç edilemeyen her kaynak kayıt kimlik, sınıf, neden, etkilediği teslim ve çözümle listelenir. Hızlı dışa aktarım kayıpları bildirebilir. Ruhsat paketinde gerekli nesne kaybı yayın engelidir. [IFC şema ve kavramları](https://standards.buildingsmart.org/IFC/RELEASE/IFC4_3/HTML/) · [Mevcut eşleme](../crates/shared/ifc/src/arch.rs) · [Model yazıcısı](../crates/shared/ifc/src/building.rs).

### 4.1 Özellik ve miktar örnekleri

| Dayanak | Öncelikli alan | Kaynak ve doğrulama |
| --- | --- | --- |
| EK-6 6.66 / 6.25 / 6.56 / 6.67 / 6.52 | Duvar, kapı, döşeme, pencere ve çatıda ilgili `IsExternal`, `FireRating`; sınıfın `Name`, `Tag`, `PredefinedType` değerleri. | Dış sınır geometrisinden türetilen değer kullanıcı tarafından açıklanabilir. Yangın dayanımı, kapı veya duvar tipi için kayıtlı performans belgesinden ya da uzman beyanından gelir. Kaynağı eksikse alan eksik olarak raporlanır. Karşı örnek olarak kolonla kesilen dış duvar, avlu ve açık balkon. |
| EK-6 6.25, 6.61, 6.16 | Kapı/merdiven çıkış rolü, kolon taşıyıcılığı ve malzeme dayanımı. | Mimari kaçış ağı ile çıkış rolü ve müellif kararı karşılaştırılır. Uygun `Pset_…` ve IFC veri tipi tek tek şablondan doğrulanır. Malzeme adı ve dayanım belgesi kaynaklanır. |
| EK-6 6.58, 6.8 | Mahal net alanı ve kaplamaları, kat alanı ve taşıyıcı döşeme kotu. | `Qto_SpaceBaseQuantities.NetFloorArea`, kaplama Pset'i ve kat için ilgili alan/kot şablonları. Kapalı mahal, döşeme üstü/bitmiş döşeme kotları ve yasal alan hesabı aynı anlamı taşıdığında eşlenir. |
| EK-6 6.7, 6.55, 6.60 | Bina kimliği/kullanımı, adres, parsel/tapu, TAKS, KAKS, çekme, emsal, bina miktarları. | `Pset_BuildingCommon`, `Pset_Address`, `Pset_LandRegistration`, `Qto_BuildingBaseQuantities`, `TREpys_ParselOzellikSeti` ve `TREpys_EmsalOzellikSeti`. Türkçe set ve alan yazımı kaynakta birebir teyit edilir. Hesaplanan ve resmî değerler ayrı izlenir. |
| EK-7 7.1–7.5 | Proje ve müellifler, oda sicili, kuruluş ve rol. | `IfcProject`, kişi, kuruluş, kişi–kuruluş ve aktör/rol ilişkileri. Ad/soyad ayrı veriden girilir, boş yer tutucu dışa verilmez. Ortak proje alanları disiplin dosyaları arasında karşılaştırılır. |

Mevcut kodda yangın dayanımı, çıkış bilgisi ve beton sınıfı açık. Hedefte bilinmeyen, uygulanamaz, gerçek `false`, ölçülmüş sıfır ve eksik değer ayrı olmalı. Nil/boş metin bulunan bir property kaydı gerekli bilgiyi karşılamaz. Standart şablonlar, Türkiye'ye özgü setler ve KentOS çalışma metadata'sı farklı ad alanlarında yönetilir. Avrupa/ulusal ad üst üste gelirse dosyada iki çelişik otorite oluşturulmaz. [ADR 8047 §§4–9](../docs/adr/8047-ifc-building-export.md) · [IFC resmî sürümü ve özellik şablonları](https://standards.buildingsmart.org/) · [EK-6/7 sayfa indeksi](https://github.com/BerkantACUN/ifc-ruhsat/blob/main/KAYNAKLAR.md).

## 5. Koordinat, kot ve kimlik

- EK-7 Tablo 7.6–7.8 için `IfcProjectedCRS`, `IfcMapConversion` ve temsil bağlamı kullanan tek koordinat sözleşmesi. TUREF/TM dilimi için EPSG:5253–5259, merkez meridyeni, metre, düşey datum ve gerçek kuzey ayrı alanlar. `IfcMapConversion` yerel mühendislik düzleminden haritaya ölçek, dönüş ve öteleme uygular. EPSG kodunu kayda yazma işlemi harita dönüşümünü sağlamaz. [IfcMapConversion](https://standards.buildingsmart.org/IFC/RELEASE/IFC4_3/HTML/lexical/IfcMapConversion.htm) · [ADR 8047 §8](../docs/adr/8047-ifc-building-export.md).
- Mevcut uygulama yatayda mutlak çizim koordinatı ve sıfır öteleme kullanıyor. CRS varsa eksik ±0.00 için dönüşümde `0.0` oluşabiliyor, EK-9 eksikliği ayrıca görebiliyor. Tasarım, yerel orijinli model ve haritaya açık dönüşüm seçeneğini ele alır. Seçili yöntemde koordinatı iki kez ötelemeyi, bağıl kat kotuna mutlak kotu iki kez eklemeyi ve bilinmeyen datumda sıfır varsayımını önle.
- Yükseklikte projeye ait ortometrik/ellipsoidal datum, yerel röper, jeoit/dönüşüm parametresi ve EK-7 notu ayrı kaydedilir. TUDKA99/ITRF96 ve kurum yorumu doğrulanır. Geometrik bağlam hassasiyeti, dönüşüm ve kurum için kabul edilen ölçüm toleransı ayrı konfigürasyondur.
- Kabul senaryosu bölgeye dağılmış en az üç yatay kontrol noktası, bağımsız kot röperi, döndürülmüş yerel model ve uzak koordinat, yanal/zemin ve kat doğrulaması. Eksen sırası, birim, ters dönüşüm ve başka disiplin IFC'siyle üst üste bindirme bağımsız okuyucuda ölçülür.
- Proje/saha/bina/kat, kaynak eleman, açıklık, türetilmiş ilişki ve tip kimliği için tek politika. Gerçek nesnede kalıcı KentOS UUID → IFC GUID. Türetilmiş `IfcRoot` kimliğinde proje/bina/disiplin kapsamı ve kararlı ebeveyn/rol, dosya yolu veya nesne sıra numarası kullanılmaz. Türetilmiş kimliğin değiştiği kapsam senaryoları bilinçli test edilir. Aynı nesnenin yer değiştirmesi ve künye/pafta revizyonu kimliğini korur. Kopya, bölme, birleşme, silme/undo ve disiplinler arası çakışma test edilir. [STEP yardımcıları](../crates/shared/ifc/src/step.rs) · [CRS yazıcısı](../crates/shared/ifc/src/crs.rs) · [Uygulama adaptörü](../crates/native/application/src/arch/arch_ifc.rs).

## 6. EK-8 gelişim seviyesi

Md. 14, EK-8'deki gelişim seviyesini ister. İkincil sayfa dökümünde ruhsat modeli için **LOD 300** belirtiliyor. Uygulama, birincil sayfanın teyidinden sonra her nesne grubu için amaç, gerekli geometri ve veri, örnek, sorumlu, makine testi ve görsel inceleme kanıtı kaydeder. Aşağıdaki satırlar KentOS'un önerilen kabul örnekleridir. [Hazırlama md. 14](https://resmi.app/dokuman/33331-html-20260805-2) · [Bağımsız EK-8 dökümü](https://github.com/BerkantACUN/ifc-ruhsat/blob/main/KAYNAKLAR.md).

| Nesne | Geometri kanıtı | Alfanümerik kanıt |
| --- | --- | --- |
| Duvar/döşeme/çatı | Boyut, konum, yön, kalınlık, sınır, kot ve gerçek açıklıklarla plan/kesit kontrolü. | Tür, malzeme/katman, dışlık ve ilgili yük/yangın özelliklerinin kaynağı. |
| Kapı/pencere | Host ve kesilen açıklık, kasa yerleşimi, eşik/parapet, net ve toplam ölçü. | Kimlik/tip, yönde çalışma, çıkış ve dayanım beyanları. |
| Mahal | Hacim kapalı, kotu ve benzersiz yerleşimi belirli, alan hatası ve örtüşme yok. | No ve ad, kullanım, ölçüm/model kökenli miktar ve kaplama. |
| Merdiven/rampa | Başlangıç ve varış, kol ve sahanlık, eğim, temiz geçiş ve kesit. | Tip ve işlev, kaçış/erişilebilirlik ilişkisi ve doğrulayan müellif. |

Dosyada `Body` bulunması, polygon adedi veya bir LOD metin alanı yeterlilik kanıtı sağlamaz. Otomatik geometri ve veri kontrolleri ile ölçekli kesit/görsel inceleme birleştirilir. İlgili bilgi gereksiniminin sınırı ISO 19650 altında teslim amacına bağlanır.

## 7. EK-9 ve yazılmış dosya doğrulaması

[Mevcut `quality::ek9`](../crates/shared/ifc/src/quality.rs) 21 soruyu üretiyor. Bazı sonuçlar bellekte kurulan model veya yazıcının varsayımından türetiliyor. Soru 9 sabit eksikleri bildiriyor, 18 teslim dosyasını görmediğinden kullanıcıya bırakılıyor, 21 kayıp listesinin boş olmasını uyum sayabiliyor. Formun Ruhsat takımında kimliği bugün başlık satırı üzerinden tanınabiliyor. Üretim çalışmasında formun kalıcı kaynak türü, proje/teslim revizyonu ve kanıt bağı olmalı.

| EK-9 soruları | Üretim kanıtı |
| --- | --- |
| **1–2** dosya boyutu/proje bilgisi | Yazılmış dosya bayt boyutu ve yayımlanmış kurum sınırı. EK-7 alanlarının gerçek dosyada varlığı. Sınır yayımlanmamışsa inceleme alanı açık. |
| **3–5** proje, mahal, varlık adı | Dosya adı ve iç başlık, `IfcProject`, mahal biçimi/tekillik ve sınıf kodu. Bayt olarak yazılan ad okunup geri karşılaştırılır. |
| **6–7** koordinat ve gelişim | Harita dönüşümü ve kontrol noktası delili, EK-8 nesne/uzman kontrolü. EPSG etiketinin tek başına yeterliliği varsayılmaz. |
| **8–11** model ve bilgiler | Sınıf kapsamı, zorunlu alanlar, emsal tablosu ve mekânsal hiyerarşi. Emsal, geometri miktarı ile yasal alan kararına bağlı karşılaştırılır. |
| **12–16** disiplin modeli | Mahal varlığı, örtüşme, kapalılık, kat ve yinelenen varlık. Kutu kesişimi hızlı ön filtre, gerçek katı/poligon sınaması karar kaynağı. |
| **17–20** format ve IFC alanları | Gerçek şema/uzantı, sınıf kullanımı, enum ve Pset/Qto. Boş temel özellik ve semantik karşılığı olmayan varsayılan ayrı hata. |
| **21** PDF/A–IFC uyumu | IFC hash ve PDF/A listesi, revizyon/girdi özeti, ilgili plan/kesit katı ve görünen durum, geometri ve künye/metadata. Seçili kritik ölçü ve mahal/elemanlar dosyadan bağımsız yeniden ölçülür. |

Formun resmî Evet/Hayır alanı ve KentOS içindeki kanıt durumu ayrı saklanır. Hazırlık aşamasında eksik inceleme imzalı Evet'e çevrilmez. Bir önceki paketten kalan manuel onay, etkili dosya veya profil değiştiğinde eski olarak görünür. Her bulgu kaynak nesne, GlobalId, IFC sınıfı/alan yolu, fıkra/ek, ölçülen ve istenen değer ve varsa ilgili pafta/düzeltme yolunu verir.

Doğrulama katmanları ve araçlar:

1. **Okuma ve şema.** Fiziksel STEP sözdizimi, başlık, referans, IFC EXPRESS tip/kardinalite/enum ve bağımsız şema denetimi. Mevcut [`arch_ifc_check.py`](../scripts/fixtures/arch_ifc_check.py) ve [ortak vakalar](../scripts/fixtures/arch_ifc_cases.py) üzerine yazılmış dosya kapısı. IfcOpenShell sürümü ve EXPRESS kapsama sınırı raporda. buildingSMART doğrulama hizmetindeki şema/normatif kurallar ayrımı kullanılır. [IfcOpenShell doğrulama](https://docs.ifcopenshell.org/autoapi/ifcopenshell/validate/index.html) · [buildingSMART](https://validate.buildingsmart.org/).
2. **Bilgi gereksinimi.** Sürümlü EK-5/6/7 kataloğundan anlamlı alt küme için IDS 1.0 üret. Entity, öznitelik, özellik, malzeme, sınıflandırma ve uygun ilişki isteklerini ifade et. IDS belgesini şema ve bağımsız denetleyiciyle, çıkan IFC'yi doğrulanmış IDS ile denetle. Mekânsal doğruluk, ayrıntı düzeyi, mevzuat istisnası ve dosyalar arası karşılaştırma ayrı kontrollerde tutulur. [buildingSMART IDS](https://www.buildingsmart.org/standards/bsi-standards/information-delivery-specification-ids/) · [IDS şema ve dokümantasyon](https://github.com/buildingSMART/IDS).
3. **Geometri ve 2B.** Host/void/fill ilişkisi, boşluk hacmi, kat ve yerleşim, kapalılık, çakışma ve alanların dosya üzerinden ölçümü. Bölgesel elipsoit/düşey dönüşüm bağımsız kontrol. PDF/A için mevcut PDF/A-2b, kabul profilindeki bölüm/düzey ve [veraPDF](https://docs.verapdf.org/validation/) ile gerçek baytlarda doğrulanır. Font, çıktı amacı, pafta boyutu, bir dosya bir pafta, vektör içeriği ve pafta–IFC ölçü karşılaştırmaları. [PDF/A kararı ve doğrulama açığı](../docs/adr/8037-turkish-title-block-sheet-sets-pdfa.md).
4. **İnsan ve kurum.** Modelde temsil biçimi, gelişim seviyesi, yangın ve imar istisnaları, kontrol bulgularının çözümü, pilot idare geri bildirimi. Gerçek ruhsat modelleri haricî web doğrulayıcıya otomatik yüklenmez. Bağımsız çevrimdışı doğrulama kullanılır, açık/sentetik örnekler gerekirse dış doğrulamada değerlendirilir.

## 8. İçe alım, karşılaştırma ve federasyon

- İlk alım IFC 4.3 referans görüntüleme ve denetleme. Dosya orijinali, şema, hash, disiplin, teslim revizyonu, dönüşüm ve `GlobalId` haritası tutulur. Yabancı Pset, Qto, sınıflandırma ve placement mümkün olduğunca kayıpsız taşınır. Katalog dışı nesne raporlanır ve desteklenen geometri referans olarak erişilebilir kalır.
- Sonraki adım doğrulanmış sınıfları onayla düzenlenebilir KentOS nesnesine dönüştürür. İçe al–dışa ver turunda iki STEP metninin dizilişi yerine kimlik, sınıf, yer, özellik ve miktar karşılaştırılır. Kayıp defteri reimport ve model güncellemesiyle birlikte oluşur. Kaynak disiplin modeli değişmeden kalır, düzenlenen sonuç yeni belge revizyonudur.
- IFC4 ve IFC2x3 okumaları sürüm eşleme kapısına girer. Kaldırılan sınıflar, Unicode, birimler, geometri biçimleri ve tip serileştirme farkı ayrı test edilir. Disiplin çakışmaları için birleşik görünüm, kaynak kimlikli bulgular ve BCF değerlendirilir. `IfcProject` ortak alanlarını ve GlobalId çakışmalarını federasyon yüklemesinde doğrula.
- Dış dosya okuma salt okunur ve sınırlandırılmış iş olarak çalışır. Bozuk dosya, döngüsel placement ve büyük/yarım modellerde bellek ve süre bütçesi, kaynak sınırı, iptal ve ayrıntılı hata vardır. Depo örneklerini ölçekleyerek nesne sayısı, duvar/açıklık bütünlüğü, yerleşim, bellek, hız ve gerçek pencerede seçim gecikmesi kaydedilir.

## 9. Teslim paketi, regresyon ve üretim kanıtı

| Test/veri | Başarı ölçütü |
| --- | --- |
| `section_building.py`, `arch_ifc_cases.py`, mevcut 2+1 | Bağımsız beklenen sınıf, boşluk ilişkisi, ölçü, kat, proje kodu ve seçili özellikler. Hedef ihracı iki bağımsız okuyucuda aç, PDF/A'yı veraPDF ile doğrula. |
| Kapsama giren ve dışarıda kalan yeni bina, karma kullanım, iki blok, ortak bodrum | Mevzuat grubu ve zamanın doğruluğu, bina/kat kapsamı, ortak alanlar, değişmez teslim girdisi, tekilleştirilmiş kimlik. |
| Eğri duvar, rampa, çatı, şaft, atrium, yangın kapısı ve farklı kot | Alan/hacim ve bağlantı çizimi, net geçiş, kaçış ve uzman beyanı. Temsil geometrisinin ölçü farklarını model odaklı tolerans ve bağımsız kontrol ile göster. |
| Bilerek bozuk IFC, yanlış şema, eksik yangın alanı, bozuk GUID, farklı datum ve pafta revizyonu | Doğru katmanda ve nesneye bağlanan bulgu. Şema temiz olsa bile Türkiye profili veya paket uyumu başarısız olabilmeli. |
| Tekrar dışa verme, yeniden adlandırma, sil/undo, kopya, tip değişikliği | Kaynak kimlik korunumu ve yeni kimlik üretimi, giriş hash'ine bağlı öngörülebilir semantik fark. Dosya zaman damgası sabitlenmiş testlerle kabul karşılaştırması. |
| Yol IFC geriye dönük kabulü | Mevcut `IfcAlignment` şeması, örnek mesafe/kotlar ve başlık davranışı. Ortak CRS/STEP değişikliğinin mimari ile yolu birlikte koruması. [Yol regresyonu](../scripts/fixtures/road_ifc_cases.py) |
| İşlem ve masaüstü akışı | Dışa aktarma normal menü, komut, Python ve MCP üzerinden tutarlı. Geçersiz yol/dosya, üzerine yazma, iptal, karşılanmayan profil ve tekrar deneme. Dosya, EK-9 ve PDF/A hizası gerçek pencere ve proje müellifi üzerinden görülür. |

Paket yönetimi hazırlık dizini → model ve paftalar → bağımsız denetim → EK-9 müellif incelemesi → yayımlanmış revizyon akışını izler. Raporda KentOS commit'i, şema/kurum/IDS sürümü, açık kaynak numarası, araç/okuyucu sürümleri, dosya hash'leri ve inceleme kayıtları vardır. İmza ve idareye gönderim [BIM planındaki e-PYS koşullarına](bim-plan.md#6-bilgi-yönetimi-ve-teslim-işlemi) göre yürür.

## 10. Uygulama sırası ve kabul

| İş | Sahiplik ve bağımlılık | Tamamlanma ölçütü |
| --- | --- | --- |
| **IFC-01 Kaynak ve ölçüm** | IFC + mimar + mevzuat. BIM-01 ile paralel. | EK-1–9 sayfa/satır/kapsam matrisi, şema sürümü, belirsizlikler ve idare teyidi, mevcut bina/yol/PDF/A davranışının tekrar üretilebilir başlangıç raporu. |
| **IFC-02 Nesne grafiği ve alanlar** | IFC/domain + BIM-02/03. | Kaynak–sınıf–ilişki ve tip eşlemesi, kayıp listesinin kimlikli hâli, yangın/katmanlı malzeme/mahal ve niceliklerin kaynaklı yazımı. Negatif fixture'lar ile gerçek STEP ve EXPRESS doğrulaması. |
| **IFC-03 Konum ve proje paylaşımı** | IFC/GIS, BIM-02. | Yatay-düşey kontrol noktaları, ortak proje öznitelikleri, çok bina, aktör rolleri ve kaynağı belirsiz datuma ilişkin olumsuz test. |
| **IFC-04 EK-8, IDS ve EK-9** | IFC/QA + müellif, IFC-01–03. | Dosya tabanlı 21 satır kanıt, LOD incelemesi, bağımsız IDS/şema doğrulama, gerçek dosya boyutu/adı/sürümü ve her sonuçtan kaynak nesnesine dönüş. |
| **IFC-05 IFC + PDF/A pilot paket** | IFC/pafta/ürün, IFC-04. BIM-05 ile ortak pilot. | Bir konut ve bir karma kullanım pilotunda aynı revizyon, en az iki IFC okuyucu ve bağımsız PDF/A denetimi, güncel EK-9 ve idare incelemesi. |
| **IFC-06 Referans okuma ve federasyon** | IFC/koordinasyon. IFC-02–05. | Referans içe alım, dış disiplinlerle hizalama, fark raporu, IFC4/IFC2x3 için ölçülen destek/kayıp listesi ve kaynak revizyon takibi. |

**Araştırma sınırı.** Bu oturumda ana mevzuat metinleri, yayımlanmış IFC şeması ve ilgili resmî teknik belgeler incelendi. Resmî 241 sayfalık ek PDF'sinin doğrudan açılması zaman aşımına uğradı, bağımsız PDF kopyası araç boyut sınırını aştı. Ekler için KentOS ADR'lerinin sayfa okumaları ve [bağımsız sayfa numaralı döküm](https://github.com/BerkantACUN/ifc-ruhsat/blob/main/KAYNAKLAR.md) kullanıldı. EK-8 LOD 300, bütün EK-5/6/7 satırları, adlandırma/şema çelişkileri, TSE baskısı ve resmî e-PYS uygulama kuralları üretim iddiasından önce birincil belgeyle teyit edilecek. ADR'lerdeki test sonuçları geçmiş kanıttır. Bu plan çalışmasında ürün testleri yeniden çalıştırılmadı.
