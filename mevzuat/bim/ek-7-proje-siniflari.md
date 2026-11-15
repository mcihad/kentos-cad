<!-- Kaynak: Resmî Gazete 5.8.2026/33331, Dijital Hazırlama Yönetmeliği ekleri (EK-1…EK-9), PDF s. 221–236. Taranmış sayfalardan OCR ile okundu; yöntem, doğrulama kapsamı ve sınırlar için README.md. -->

*[PDF sayfa 221]*

# EK-7 PROJE İLE İLGİLİ IFC SINIFLARININ ÖZNİTELİK TANIMLARI

**7.1-** BIM modelinde projeye ait bilgilerin temsilinde kullanılacak zorunlu IFC sınıfları Tablo 7.1 ile Tablo 7.12 arasında verilmiştir.

**7.1.1-** Tablo 7.1 ile Tablo 7.12 arasında yer alan IFC sınıfları, IFC 4.3 versiyonu esas alınarak düzenlenmiştir

**7.1.2-** Zorunlu IFC sınıflarında yer alması gereken zorunlu öznitelikler ve özellikler Tablo 7.1 ile Tablo 7.9 arasında yer alan tablolarda verilmiştir.

**7.1.3-** Zorunlu IFC sınıflarında yer alması isteğe bağlı olan öznitelikler ve özellikler Tablo 7.10, Tablo 7.11 ve Tablo 7.12 tablolarında referans olarak verilmiştir ve ilgili idaresince talep edilmesi halinde ilgili IFC sınıfında yer alır.

**7.2-** Projeye ait temel bilgiler zorunlu "`IfcProject`" sınıfında tanımlanır. Bu sınıfa ait zorunlu öznitelikler ve özellikler Tablo 7.1'de verilmiştir.

**7.2.1-** Her bir disiplin modelinin "`IfcProject`" sınıfının ortak öznitelikleri aynı olmalıdır.

**7.3-** `IfcProject` sınıfı içerisinde tariflenen `IfcPerson` sınıfı zorunlu öznitelik listesi Tablo 7.2'de, `IfcPersonAndOrganization` sınıfı zorunlu öznitelik listesi Tablo 7.3'te, `IfcOrganization` sınıfı zorunlu öznitelik listesi Tablo 7.4'te ve `IfcActorRole` sınıfı zorunlu öznitelik listesi Tablo 7.5'te verilmiştir.

**7.4-** `IfcProjectedCRS`, `IfcMapConversion`, `IfcGeometricRepresentationContext` ve `IfcAnnotation` sınıflarının zorunlu öznitelik listesi sırasıyla Tablo 7.6, Tablo 7.7 ve Tablo 7.8 ve Tablo 7.9'da verilmiştir.

*[PDF sayfa 222]*

## Tablo 7.1 `IfcProject` Sınıfı Zorunlu Öznitelik ve Özellik Listesi

**Zorunlu Öznitelik ve Özellik Listesi**

| Proje (`IfcProject`) | Ön Tanımlı Tip Listesi |
|---|---|
| Tanım: IfcProject'in bir veri alışverişi yapısındaki temel amacı, tüm diğer bilgi öğeleri için kök örneği (root instance) ve bağlamı (context) sağlamaktır. | - |

| Türkçe Tanım (Bilgi Amaçlı) | Öznitelik | İlişki | Veri Tipi (Bilgi Amaçlı) | Açıklama |
|---|---|---|---|---|
| Açıklama | Attribute: Description | - | Metin | Bilgilendirme amaçlı isteğe bağlı olarak verilen bir tanımdır. |
| İsim | Attribute: Name | - | Metin | IFC uyumlu yazılım sistemleri veya kullanıcılar tarafından kullanılmak üzere isteğe bağlı bir ad. [[`IfcRoot`]] sınıfının bazı alt tipleri için İsim niteliğinin eklenmesi gerekebilir. Bu, bir "where" kuralı ile zorunlu hale getirilecektir. (EK-5'e göre isimlendirilecektir) |

| Türkçe Tanım (Bilgi Amaçlı) | Özellik | Özellik Seti / İlişki | Veri Tipi (Bilgi Amaçlı) | Açıklama |
|---|---|---|---|---|
| Proje Tipi | `ProjectType` | `Pset_ProjectCommon` | Metin | Bir projenin ek tip tanımlaması |

*[PDF sayfa 223]*

## Tablo 7.2 `IfcPerson` Sınıfı Zorunlu Öznitelik ve Özellik Listesi (`IfcProject` içerisinde tariflenir)

| Kişi (`IfcPerson`) | Ön Tanımlı Tip Listesi |
|---|---|
| Tanım: Bu varlık, bir bireyi temsil eder. NOT: Birçok ülkede, veri tabanlarında kişilerin tanımlanmasına ilişkin yasal düzenlemeler bulunmaktadır. IFC Modeli'nin amacı veri değişimi ve paylaşımı için bir tanımlama standardı olarak hizmet etmek olsa da bazı durumlarda bir IFC dosyası bu tür mevzuatlar kapsamında belirli bir kişiyi tanımlamaya olanak sağlayan bir veri tabanı olarak kabul edilebilir. Kullanıcılar, IFC dosyalarının kullanıldığı yerlerde geçerli olabilecek yasal kısıtlamaların farkında olmalıdır. | - |

| Türkçe Tanım (Bilgi Amaçlı) | Öznitelik | İlişki | Veri Tipi (Bilgi Amaçlı) | Açıklama |
|---|---|---|---|---|
| Kimlik No | Attribute: Identification | - | Metin | (Oda Sicil numarası). Kişinin tanımlanması. |
| Aile Adı | Attribute: `FamilyName` | - | Metin | Kişinin aile kimliğinin tanınmasını sağlayan addır. NOT: Coğrafi konuma ve kültüre bağlı olarak, soyad bir ismin ilk veya son kısmında yer alabilir. |
| Ad | Attribute: `GivenName` | - | Metin | Kişinin aile içinde bilindiği ve tanındığı addır. NOT: Coğrafi konuma ve kültüre bağlı olarak, ad bir ismin ilk veya son kısmında yer alabilir. |
| Rol | Attribute: Roles | - | Metin | Kişinin üstlendiği roller. |

*[PDF sayfa 224]*

## Tablo 7.3 `IfcPersonAndOrganization` Sınıfı Zorunlu Öznitelik ve Özellik Listesi (`IfcProject` içerisinde tariflenir)

| Kişi ve Organizasyon (`IfcPersonAndOrganization`) | Ön Tanımlı Tip Listesi |
|---|---|
| Bu varlık, bir kuruluş adına hareket eden bir kişiyi temsil eder. NOT: Bu varlık, ISO 10303-41'de tanımlanan person\_and\_organization kavramından uyarlanmıştır. | - |

| Türkçe Tanım (Bilgi Amaçlı) | Öznitelik | İlişki | Veri Tipi (Bilgi Amaçlı) | Açıklama |
|---|---|---|---|---|
| Kişi | Attribute: `ThePerson` | - | Metin | Kuruluş ile ilişkili olan kişi. |
| Organizasyon | Attribute: `TheOrganization` | - | Metin | Kişinin ilişkili olduğu kuruluş. |

*[PDF sayfa 225]*

## Tablo 7.4 `IfcOrganization` Sınıfı Zorunlu Öznitelik ve Özellik Listesi (`IfcProject` içerisinde tariflenir)

| Organizasyon (`IfcOrganization`) | Ön Tanımlı Tip Listesi |
|---|---|
| Tanım: Kurumsal bir kimliğe sahip, adlandırılmış ve yapılandırılmış bir gruplamadır. NOT: Bir şirket içindeki departmanlar arasındaki ilişkilerde olduğu gibi, `IfcOrganization` varlıkları arasındaki ilişkiler, nesneleştirilmiş ilişki olan `IfcOrganizationRelationship` aracılığıyla ifade edilebilir. | - |

| Türkçe Tanım (Bilgi Amaçlı) | Öznitelik | İlişki | Veri Tipi (Bilgi Amaçlı) | Açıklama |
|---|---|---|---|---|
| Firma Vergi No | Attribute: Identification | - | Sayı | Kuruluşun tanımlanması (Vergi Kimlik Numarası). |
| İsim | Attribute: Name | - | Metin | IFC uyumlu yazılım sistemleri veya kullanıcılar tarafından kullanılmak üzere isteğe bağlı bir ad. [[`IfcRoot`]] sınıfının bazı alt tipleri için İsim niteliğinin eklenmesi gerekebilir. Bu, bir "where" kuralı ile zorunlu hale getirilecektir. |
| Rol | Attribute:Roles>ifcActorRole | - | Metin | Kuruluşun üstlendiği roller. |
| Adres | Attribute: Addresses >ifcPostalAddress, `ifcTelecomAddress` | - | Metin | Bir kuruluşun posta ve telekomünikasyon adresleri. NOT: Bir kuruluşa ait birden fazla adres bulunabilir. |

*[PDF sayfa 226]*

## Tablo 7.5 `IfcActorRole` Sınıfı Zorunlu Öznitelik Listesi (`IfcProject` içerisinde tariflenir)

| Rol (`IfcActorRole`) | Ön Tanımlı Tip Listesi |
|---|---|
| Tanım: Kurumsal kimliğe sahip, adlandırılmış ve yapılandırılmış bir gruplama. NOT: IfcOrganization'lar arasındaki ilişkiler (örneğin bir şirket içindeki departmanlar arasındaki ilişkiler), nesneleştirilmiş ilişki `IfcOrganizationRelationship` kullanılarak ifade edilebilir. | - |

| Türkçe Tanım (Bilgi Amaçlı) | Öznitelik | İlişki | Veri Tipi (Bilgi Amaçlı) | Açıklama |
|---|---|---|---|---|
| Rol | Attribute: Role | - | Metin | Bir aktörün üstlendiği rolün adıdır. Eğer Rol değeri USERDEFINED ise, kullanıcı tarafından tanımlanan rol, `UserDefinedRole` özniteliğinin değeri olarak verilmelidir. |
| Kullanıcı Tanımlı Rol | Attribute: `UserDefinedRole` | - | Metin | `IfcRoleEnum` türündeki Role özniteliğinde tanımlanan listelenmiş değerlerin ötesinde, kullanıcı tanımlı rollerin belirtilmesine izin verir. `UserDefinedRole` özniteliğine bir değer verildiğinde, Role özniteliğinin numaralandırılmış değeri USERDEFINED olmalıdır. |
| Açıklama | Attribute: Description | - | Metin | Bilgilendirme amaçlı isteğe bağlı olarak verilen bir tanımdır. |

*[PDF sayfa 227]*

## Tablo 7.6 `IfcProjectedCRS` Sınıfı Zorunlu Öznitelik Listesi

| Yansıtılmış Koordinat Referans Sistemi (`IfcProjectedCRS`) | Ön Tanımlı Tip Listesi |
|---|---|
| Tanım: `IfcProjectedCRS`, bir inşaat veya tesis mühendisliği projesinin yerel mühendislik koordinat sisteminin harita koordinat sistemine dönüştürüldüğü koordinat referans sistemidir (CRS). Yansıtılmış (projected) koordinat referans sisteminin 2B veya 3B, sağ el kuralına göre tanımlanmış Kartezyen bir koordinat sistemi olduğu varsayılır. Haritada kullanılan uzunluk birimini belirtmek için isteğe bağlı `MapUnit` özniteliği kullanılabilir. | - |

| Türkçe Tanım (Bilgi Amaçlı) | Öznitelik | İlişki | Veri Tipi (Bilgi Amaçlı) | Açıklama |
|---|---|---|---|---|
| İsim | Attribute: Name | - | Metin | Koordinat referans sisteminin tanımlandı isimdir. Not: Modelin bulunduğu TUREF (Türkiye Ulusal Temel GPS Ağı - ITRF96) 3° dilime karşılık gelen EPSG kodu kullanılır: EPSG:5253, EPSG:5254 , EPSG:5255, EPSG:5256, EPSG:5257, EPSG:5258, veya EPSG:5259 |
| Açıklama | Attribute: Description | - | Metin | Bu koordinat referans sisteminin gayriresmî açıklaması. |
| Jeodezik Datum | Attribute: `GeodeticDatum` | - | Metin | Bu datumun tanımlandığı isim. Jeodezik datum, koordinat referans sistemi ile ilişkilidir ve dönel elipsoidin şekli ile boyutunu ve bu elipsoidin gerçek dünya / yerküre ile olan bağlantısını ve yönelimini gösterir. Türkiye Ulusal Temel GPS Ağı — ITRF96 çerçevesinin Türkiye'deki gerçeklemesidir. GRS80 elipsoidi kullanılır. |
| Yansıtılmış Koordinat Referans Sistemi (Düşey Datum) | Attribute: `VerticalDatum` | - | Metin | tanımlamak için kullanılan Düşey datum, koordinat referans sisteminin yükseklik ekseni ile ilişkilidir ve bir yükseklik sisteminin başlangıç düzlemini ve temel referans noktasını tanımlar.<br>Not: Yükseklik referans sistemi olarak ITRF96 kullanılır. Gerektiğinde yerel deniz seviyesi referansına (TUDKA99) dönüşüm parametreleri ayrıca belirtilir. |
| Yansıtılmış Koordinat Referans Sistemi (Harita Projeksiyonu) | Attribute: `MapProjection` | - | Metin | Harita projeksiyonunun tanımlandığı addır. Not: Transverse Mercator (3° dilimlere bölünmüş Gauss-Krüger tipi Enine Merkator projeksiyonu. Her dilim 3°'lik meridyen genişliğine sahiptir.) |
| Yansıtılmış Koordinat Referans Sistemi (Harita Zonu) | Attribute: `MapZone` | - | Metin | Merkez meridyeni değeridir (derece cinsinden). Modelin coğrafi konumuna göre ilgili zona ait deger seçilir ve açıkça belirtilir. Not: 27, 30, 33, 36, 39, 42, veya 45 |

*[PDF sayfa 228]*

*(Tablo önceki sayfadan sürüyor; sütun başlıkları okunabilirlik için yinelendi.)*

| Türkçe Tanım (Bilgi Amaçlı) | Öznitelik | İlişki | Veri Tipi (Bilgi Amaçlı) | Açıklama |
|---|---|---|---|---|
| Yansıtılmış Koordinat Referans Sistemi (Harita Birimi) | Attribute: `MapUnit` | - | Metin | Harita koordinat sistemini oluşturan koordinat eksenlerinin birimidir. Not: Yalnızca uzunluk ölçülerini kapsar ve harita koordinat sisteminin iki veya üç ekseninin tamamı aynı uzunluk birimine sahip olmalıdır. Not: METRE kullamlacaktır |

*[PDF sayfa 229]*

## Tablo 7.7 `IfcMapConversion` Sınıfı Zorunlu Öznitelik Listesi

| Harita Dönüşümü (`IfcMapConversion`) | Ön Tanımlı Tip Listesi |
|---|---|
| Tanım: Harita dönüşümü, yerel mühendislik koordinat sisteminin -genellikle dünya koordinat sistemi (world coordinate system) olarak adlandırılır- temel alınan haritanın koordinat referans sistemine dönüştürülmesiyle ilgilenir. NOT: `IfcMapConversion`, haritanın jeodezik koordinat referans sisteminden projeksiyonunu gerçekleştirmez. | - |

| Türkçe Tanım (Bilgi Amaçlı) | Öznitelik | İlişki | Veri Tipi (Bilgi Amaçlı) | Açıklama |
|---|---|---|---|---|
| Harita Dönüşümü (Doğu) | Attribute: Eastings | - | Sayı | Hedef harita koordinat referans sisteminin doğu (easting) ekseni boyunca konumu belirtir. NOT: Sağ el kuralına göre tanımlanmış Kartezyen koordinat sistemlerinde bu değer, x ekseni üzerindeki konumu ifade eder. |
| Harita Dönüşümü (Kuzey) | Attribute: Northings | - | Sayı | Hedef harita koordinat referans sisteminin kuzey (northing) ekseni boyunca belirtir. NOT: Sağ el kuralına göre tanımlanmış Kartezyen koordinat sistemlerinde bu değer, y ekseni üzerindeki konumu ifade eder. |
| Harita Dönüşümü (Düşey Yükseklik) | Attribute: `OrthogonalHeight` | - | Sayı | Belirtilen düşey datuma göre düşey (orthogonal) yüksekliği tanımlar. NOT: Sağ el kuralına göre tanımlanmış Kartezyen koordinat sistemlerinde bu değer, z ekseni üzerindeki konumu ifade eder. |
| Harita Dönüşümü (Yerel X Ekseninin Doğu Bileşeni) | Attribute: `XAxisAbscissa` | - | Sayı | Yerel mühendislik koordinat referans sisteminin yerel X ekseninin yönünü gösteren vektörün uç noktasının, hedef harita koordinat referans sisteminin doğu (easting) ekseni üzerindeki degerini tanımlar. NOT: Sağ el kuralına göre tanımlanmış Kartezyen koordinat sistemlerinde bu değer, x ekseni üzerindeki konumu ifade eder. NOT: Bu değer, `XAxisOrdinate` ile birlikte kullanıldığında, yerel X ekseninin harita koordinat sisteminin yatay düzlemi içindeki yönünü tanımlar. |
| Harita Dönüşümü (Yerel X Ekseninin Kuzey Bileşeni) | Attribute: `XAxisOrdinate` | - | Sayı | Yerel mühendislik koordinat referans sisteminin yerel X ekseninin yönünü gösteren vektörün uç noktasının, hedef harita koordinat referans sisteminin kuzey (northing) ekseni üzerindeki degerini tanımlar. NOT: Sağ el kuralına göre tanımlanmış Kartezyen koordinat sistemlerinde bu değer, y ekseni üzerindeki konumu ifade eder. NOT: Bu değer, `XAxisAbscissa` ile birlikte kullanıldığında, yerel X ekseninin harita koordinat sisteminin yatay düzlemi içindeki yönünü |

*[PDF sayfa 230]*

*(Tablo önceki sayfadan sürüyor; sütun başlıkları okunabilirlik için yinelendi.)*

| Türkçe Tanım (Bilgi Amaçlı) | Öznitelik | İlişki | Veri Tipi (Bilgi Amaçlı) | Açıklama |
|---|---|---|---|---|
| Harita Dönüşümü (Ölçek Değeri) | Attribute: Scale | - | Sayı | Koordinat referans sisteminin (CRS) birimleri ile mühendislik koordinat sisteminin birimleri aynı olmadığında kullanılacak ölçek değerini belirtir. Eğer belirtilmezse varsayılan değer 1.0 olarak kabul edilir. |

*[PDF sayfa 231]*

## Tablo 7.8 `IfcGeometricRepresentationContext` Sınıfı Zorunlu Öznitelik Listesi

| Geometrik Temsil Bağlamı (`IfcGeometricRepresentationContext`) | Ön Tanımlı Tip Listesi |
|---|---|
| Tanım: `IfcGeometricRepresentationContext`, bir proje içindeki ürünlerin çeşitli şekil (geometri) temsillerine uygulanan geometrik bağlamı tanımlar. Bu bağlam, şekil temsilinin içinde tanımlandığı bağlam türünü ve bu bağlamdaki geometrik temsil öğeleri için geçerli olan sayısal hassasiyet (numeric precision) değerini belirtir. Buna ek olarak, `WorldCoordinateSystem` özniteliği kullanılarak projenin koordinat sistemi, küresel bir referans noktasından öteleme (offset) verilerek tanımlanabilir. | - |

| Türkçe Tanım (Bilgi Amaçlı) | Öznitelik | İlişki | Veri Tipi (Bilgi Amaçlı) | Açıklama |
|---|---|---|---|---|
| Geometrik Temsil Bağlamı (Koordinat Uzayı Boyutu) | Attribute: `CoordinateSpaceDimension` | - | Tam Sayı | Geometrik temsil bağlamında modellenen koordinat uzayının boyut sayısının tamsayı olarak ifade edilmesidir. |
| Geometrik Temsil Bağlamı (Hassasiyet) | Attribute: Precision | - | Sayı | Geometrik modeller için model hassasiyetinin değeridir. Bu değer, genellikle 1E-5 ile 1E-8 aralığında olan, iki noktanın özdeş kabul edildiği toleransı gösteren bir çift değeridir (REAL). Bu değer örneğin brep modellerinde bir kenar eğrisinin alttaki yüzey geometrisine olan en fazla uzaklığını belirlemek için kullanılabilir. |
| Geometrik Temsil Bağlamı (Dünya Koordinat Sistemi) | Attribute: `WorldCoordinateSystem` | - | - | Projede kullanılan tüm temsil bağlamları için mühendislik koordinat sisteminin (CAD'de genellikle dünya koordinat sistemi olarak adlandırılır) oluşturulmasıdır. NOT Bu, binaların konumu orijinden çok uzakta olduğunda daha iyi sayısal kararlılık sağlamak için kullanılabilir. Ancak çoğu durumda orijin (0.,0.,0.) ve yönler x(1.,0.,0.), y(0.,1.,0.), z(0.,0.,1.) olarak ayarlanır.<br>Eğer coğrafi yerleşim `IfcMapConversion` kullanılarak sağlanmışsa, `WorldCoordinateSystem` özniteliği, yerel mühendislik koordinat sisteminin sıfır noktası ile `IfcMapConversion` ötelemesinin ilişkili olduğu coğrafi referans noktası arasındaki ötelemeyi tanımlamak için kullanılır. Tercih edilen uygulamada her iki nokta (genellikle "proje baz noktası" ve "ölçüm noktası" olarak adlandırılır) çakışık olmalıdır. Ancak coğrafi referans noktasının yerel sıfır noktasından ötelenmesi mümkündür. |
| Geometrik Temsil Bağlamı (Gerçek Kuzey Yönü) | Attribute: `TrueNorth` | - | Sayı | Gerçek kuzey yönü (true north) veya coğrafi kuzey yönü, projenin kendi koordinat sistemine göre tanımlanır. Bu yön, proje koordinat sisteminin XY düzlemi içinde verilen iki |

*[PDF sayfa 232]*

*(Tablo önceki sayfadan sürüyor; sütun başlıkları okunabilirlik için yinelendi.)*

| Türkçe Tanım (Bilgi Amaçlı) | Öznitelik | İlişki | Veri Tipi (Bilgi Amaçlı) | Açıklama |
|---|---|---|---|---|
|  |  |  |  | boyutlu bir yön vektörü ile Eğer belirtilmemişse varsayılan değer 0., 1. olarak alınır; bu da proje koordinat sisteminin pozitif Y ekseninin coğrafi kuzey yönüne karşılık geldiği anlamına gelir. NOT: Eğer coğrafi yerleşim `IfcMapConversion` kullanılarak sağlanmışsa, True North yalnızca bilgilendirme amaçlıdır. Herhangi bir tutarsızlık durumunda, `IfcMapConversion` ile verilen değer geçerli kabul edilir ve önceliğe sahiptir. |

*[PDF sayfa 233]*

## Tablo 7.9 `IfcAnnotation` Sınıfı Zorunlu Öznitelik Listesi

| Anotasyon (`IfcAnnotation`) | Ön Tanımlı Tip Listesi |
|---|---|
| Anotasyon, bir projenin geometrik (ve mekânsal) bağlamı içerisinde yer alan ve proje modelini oluşturan nesnelere açıklama veya anlam katan bir bilgi öğesidir. Anotasyonlar; ek noktaları, eğrileri, metinleri, ölçülendirmeleri, taramaları ve diğer grafiksel not biçimlerini içerir. Ayrıca ürünleri veya mekânsal yapıları temsil etmeyen, ancak modele ait ek bileşenlerin sanal ya da sembolik gösterimlerini de kapsar; örneğin ölçüm/nirengi noktaları ve çizgileri, eş yükselti eğrileri (kot eğrileri) veya benzeri gösterimler buna dahildir. | SURVEY |

| Türkçe Tanım (Bilgi Amaçlı) | Öznitelik | İlişki | Veri Tipi (Bilgi Amaçlı) | Açıklama |
|---|---|---|---|---|
| İsim | Attribute: Name | - | Metin | IFC uyumlu yazılım sistemleri veya kullanıcılar tarafından kullanılmak üzere isteğe bağlı bir ad. [[`IfcRoot`]] sınıfının bazı alt tipleri için isim niteliğinin eklenmesi gerekebilir. Bu, bir "where" kuralı ile zorunlu hale getirilecektir. |
| Açıklama | Attribute: Description | - | Metin | Bilgilendirme amaçlı isteğe bağlı olarak verilen bir tanımdır. |
| Ön Tanımlı Tip | `PredefinedType` | - | Metin | Ön Tanımlı Tip. |

*[PDF sayfa 234]*

## Tablo 7.10 `IfcProject` Sınıfı İsteğe Bağlı Kullanılabilecek Öznitelik Listesi

| Proje (`IfcProject`) | Ön Tanımlı Tip Listesi |
|---|---|
| Tanım: IfcProject'in bir veri değişim yapısındaki temel amacı, tüm diğer bilgi öğeleri için kök örneği (root instance) ve bağlamı (context) sağlamaktır. | - |

| Türkçe Tanım (Bilgi Amaçlı) | Öznitelik | İlişki | Veri Tipi (Bilgi Amaçlı) | Açıklama |
|---|---|---|---|---|
| Nesne Tipi | Attribute: `ObjectType` | - | Metin | Tip, nesneyi daha ayrıntılı tanımlayan özel bir tipi belirtir. Kullanım, örneklenebilir somut alt tiplerin seviyesinde belirlenmelidir. Not: `IfcContext` alt türleri `PredefinedType` niteliğini içermez. Bu nedenle, `ObjectType` kullanımının `PredefinedType` enum değerleri arasında USERDEFINED seçilmesine bağlı olması gerekmez. |
| Uzun İsim | Attribute: `LongName` | - | Metin | Referans amaçlı olarak kullanılan bağlamın uzun adı. |
| Aşama | Attribute: Phase | - | Metin | Mevcut proje aşaması veya bu projenin yaşam döngüsü aşaması. Uygun aşamalar, görünüm tanımları veya uygulayıcı sözleşmeleri üzerinden belirlenmelidir. |

*[PDF sayfa 235]*

## Tablo 7.11 `IfcPerson` Sınıfı İsteğe Bağlı Kullanılabilecek Öznitelik Listesi (`IfcProject` içerisinde tariflenir)

| Kişi (`IfcPerson`) | Ön Tanımlı Tip Listesi |
|---|---|

Tanım: Bu varlık, bir bireyi temsil eder. NOT: Birçok ülkede, veri tabanlarında kişilerin tanımlanmasına ilişkin yasal düzenlemeler bulunmaktadır. IFC Modeli'nin amacı veri değişimi ve paylaşımı için bir tanımlama standardı olarak hizmet etmek olsa da, bazı durumlarda bir IFC dosyası bu tür mevzuatlar kapsamında belirli bir kişiyi tanımlamaya olanak sağlayan bir veri tabanı olarak kabul edilebilir. Kullanıcılar, IFC dosyalarının kullanıldığı yerlerde geçerli olabilecek yasal kısıtlamaların farkında olmalıdır.

| Türkçe Tanım (Bilgi Amaçlı) | Öznitelik | İlişki | Veri Tipi (Bilgi Amaçlı) | Açıklama |
|---|---|---|---|---|
| Orta Ad | Attribute: `MiddleNames` | - | Metin | Aynı veya benzer soyadı ve adı taşıyan diğer kişilerden ayırt edilmesini sağlayan, kişiye verilen ek adlardır. NOT: İkinci adlar genellikle gündelik iletişimde kullanılmaz, ancak gerekirse belirli bir kişiyi tanımlamak için ek tanımlayıcı olarak kullanılabilir. Bu adlar, özellikle kişinin soyadının bulunduğu bölgede çok yaygın olarak kullanıldığı durumlarda yararlıdır. |

*[PDF sayfa 236]*

## Tablo 7.12 `IfcOrganization` Sınıfı İsteğe Bağlı Kullanılabilecek Öznitelik Listesi (`IfcProject` içerisinde tariflenir)

| Organizasyon (`IfcOrganization`) | Ön Tanımlı Tip Listesi |
|---|---|
| Tanım: Kurumsal bir kimliğe sahip, adlandırılmış ve yapılandırılmış bir gruplamadır. NOT: Bir şirket içindeki departmanlar arasındaki ilişkilerde olduğu gibi, `IfcOrganization` varlıkları arasındaki ilişkiler, nesneleştirilmiş ilişki olan `IfcOrganizationRelationship` aracılığıyla ifade edilebilir. | - |

| Türkçe Tanım (Bilgi Amaçlı) | Öznitelik | İlişki | Veri Tipi (Bilgi Amaçlı) | Açıklama |
|---|---|---|---|---|
| Açıklama | Attribute: Description | - | Metin | Bilgilendirme amaçlı isteğe bağlı olarak verilen bir tanımdır. |
