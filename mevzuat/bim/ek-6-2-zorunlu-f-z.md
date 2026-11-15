<!-- Kaynak: Resmî Gazete 5.8.2026/33331, Dijital Hazırlama Yönetmeliği ekleri (EK-1…EK-9), PDF s. 73–109. Taranmış sayfalardan OCR ile okundu; yöntem, doğrulama kapsamı ve sınırlar için README.md. -->

# EK-6 (devam) — zorunlu, Tablo 6.34–6.68

*[PDF sayfa 73]*

## Tablo 6.34 `IfcFooting` Sınıfı Zorunlu Öznitelik ve Özellik Listesi

**Zorunlu Öznitelik ve Özellik Listesi**

| Temel (`IfcFooting`) | Ön Tanımlı Tip Listesi |
|---|---|
| Tanım: Temel pabucu, bir yapının yükünü yayarak toprağa ileten bir yapı parçasıdır. Temel pabucu, yüklerin yüzeye yakın bir alana aktarıldığı sığ temel olarak da tanımlanır. | CAISSON FOUNDATION, FOOTING BEAM, PAD FOOTING, PILE CAP, STRIP FOOTING, USERDEFINED, NOTDEFINED |

| Türkçe Tanım (Bilgi Amaçlı) | Öznitelik | İlişki | Veri Tipi (Bilgi Amaçlı) | Açıklama |
|---|---|---|---|---|
| Ön Tanımlı Tip | `PredefinedType` | - | Metin | Ön Tanımlı Tip. |
| İsim | Attribute: Name | - | Metin | IFC uyumlu yazılım sistemleri veya kullanıcılar tarafından kullanılmak üzere isteğe bağlı bir ad. [[`IfcRoot`]] sınıfının baz: alt tipleri için İsim niteliğinin eklenmesi gerekebilir. Bu, bir "where" kuralı ile zorunlu hale getirilecektir. (EK-5'e göre isimlendirilecektir) |
| Etiket | Attribute: Tag | - | Sayı veya Metin | Bir ürünün belirli bir örneğindeki etiket tanımlayıcısı, örneğin seri numarası veya konum numarasıdır. Bu, yerleştirilmiş örnek düzeyindeki tanımlayıcıdır. |
| Malzeme | Attribute (`IfcRelAssociatesMaterial>IfcMaterial`): Name\* | - | Metin | [`IfcMaterial`], yapı elemanlarını (fiziksel ürünler veya bileşenleri) oluşturmak için kullanılabilecek homojen veya heterojen maddelerdir. |

\*Attribute (`IfcRelAssociatesMaterial>IfcMaterial`): Name\* bilgisi giriliken: "Malzeme İsmi\_Dayanım" formatı kullanılmalıdır. (Örnek:Çelik\_S355 Beton\_C25)

*[PDF sayfa 74]*

## Tablo 6.35 `IfcFurniture` Sınıfı Zorunlu Öznitelik ve Özellik Listesi

**Zorunlu Öznitelik ve Özellik Listesi**

| Mobilya (`IfcFurniture`) | Ön Tanımlı Tip Listesi |
|---|---|
| Tanım: Mobilya, binanın yapısına sabitlenmiş ya da sabitlenmemiş masa, sandalye, dolap gibi tefriş elemanlarım ifade eder. | BED, CHAIR, DESK, FILE CABINET, SHELF, SOFA, TABLE, TECHNICAL CABINET, USERDEFINED, NOTDEFINED |

| Türkçe Tanım (Bilgi Amaçlı) | Öznitelik | İlişki | Veri Tipi (Bilgi Amaçlı) | Açıklama |
|---|---|---|---|---|
| Ön Tanımlı Tip | `PredefinedType` | - | Metin | Ön Tanımlı Tip. |
| İsim | Attribute: Name | - | Metin | IFC uyumlu yazılım sistemleri veya kullanıcılar tarafından kullanılmak üzere isteğe bağlı bir ad. [[`IfcRoot`]] sınıfının bazı alt tipleri için İsim niteliğinin eklenmesi gerekebilir. Bu, bir "where" kuralı ile zorunlu hale getirilecektir. (EK-5'e göre isimlendirilecektir) |
| Etiket | Attribute: Tag | - | Sayı veya Metin | Bir ürünün belirli bir örneğindeki etiket tanımlayıcısı, örneğin seri numarası veya konum numarasıdır. Bu, yerleştirilmiş örnek düzeyindeki tanımlayıcıdır. |

*[PDF sayfa 75]*

## Tablo 6.36 `IfcGridAxis` Sınıfı Zorunlu Öznitelik ve Özellik Listesi

**Zorunlu Öznitelik ve Özellik Listesi**

| Aks (`IfcGridAxis`) | Ön Tanımlı Tip Listesi |
|---|---|
| Tanım: Tekil bir aks, `IfcGridAxis`, bir tasarım aks sistemi bağlamında tanımlanır. Aks tanımı, iki boyutlu (2D) bir eğriye dayanır. Grid aksı, `IfcGrid` (aks sistemi) tarafından tanımlanan konum koordinat sisteminin XY düzlemi içerisinde yer alır. | - |

| Türkçe Tanım (Bilgi Amaçlı) | Öznitelik | İlişki | Veri Tipi (Bilgi Amaçlı) | Açıklama |
|---|---|---|---|---|
| Aks Etiketi | Attribute: `AxisTag` | - | Sayı veya Metin | Bu aks eksenine ait etiket veya isim. |

*[PDF sayfa 76]*

## Tablo 6.37 `IfcGroup` Sınıfı Zorunlu Öznitelik ve Özellik Listesi

**Zorunlu Öznitelik ve Özellik Listesi**

| Grup (`IfcGroup`) | Ön Tanımlı Tip Listesi |
|---|---|
| Tanım: `IfcGroup`, herhangi bir nesne grubunun genel bir temsilidir. Grup, nesnelerin mantıksal olarak bir araya getirildiği bir topluluktur. Grup nesnesinin kendisine ait bir konumu bulunmaz ve geometrik bir temsil içermez.. Bu nedenle grup, geometrik veya topolojik özelliklere dayanmayan bir ilişkilendirme kapsamında değerlendirilir. | - |

| Türkçe Tanım (Bilgi Amaçlı) | Öznitelik | İlişki | Veri Tipi (Bilgi Amaçlı) | Açıklama |
|---|---|---|---|---|
| İsim | Attribute: Name | - | Metin | IFC uyumlu yazılım sistemleri veya kullanıcılar tarafından kullanılmak üzere isteğe bağlı bir ad. [[`IfcRoot`]] sınıfının bazı alt tipleri için İsim niteliğinin eklenmesi gerekebilir. Bu, bir "where" kuralı ile zorunlu hale getirilecektir. |
| Etiket | Attribute: Tag | - | Sayı veya Metin | Bir ürünün belirli bir örneğindeki etiket tanımlayıcısı, örneğin seri numarası veya konum numarasıdır. Bu, yerleştirilmiş örnek düzeyindeki tanımlayıcıdır. |

*[PDF sayfa 77]*

## Tablo 6.38 `IfcHeatExchanger` Sınıfı Zorunlu Öznitelik ve Özellik Listesi

**Zorunlu Öznitelik ve Özellik Listesi**

| Isı Eşanjörü (`IfcHeatExchanger`) | Ön Tanımlı Tip Listesi |
|---|---|
| Tanım: Isı Eşanjörü, plaka veya gövde-boru tipi eşanjörler gibi, birbirine karışmayan ortamlar arasında ısı transferi sağlamak için kullanılan bir cihazdır. | PLATE, SHELLANDTUBE, USERDEFINED, NOTDEFINED |

| Türkçe Tanım (Bilgi Amaçlı) | Öznitelik | İlişki | Veri Tipi (Bilgi Amaçlı) | Açıklama |
|---|---|---|---|---|
| Ön Tanımlı Tip | `PredefinedType` | - | Metin | Ön Tanımlı Tip. |
| İsim | Attribute: Name | - | Metin | IFC uyumlu yazılım sistemleri veya kullanıcılar tarafından kullanılmak üzere isteğe bağlı bir ad. [[`IfcRoot`]] sınıfının bazı alt tipleri için İsim niteliğinin eklenmesi gerekebilir. Bu, bir "where" kuralı ile zorunlu hale getirilecektir. (EK-5'e göre isimlendirilecektir) |
| Etiket | Attribute: Tag | - | Sayı veya Metin | Bir ürünün belirli bir örneğindeki etiket tanımlayıcısı, örneğin seri numarası veya konum numarasıdır. Bu, yerleştirilmiş örnek düzeyindeki tanımlayıcıdır. |
| Sistem | `IfcSystem` | `IfcRelAssignsToGroup>IfcSystem` | Metin | Bir sistem, Mimarlık, Mühendislik ve Yapım ürünü içerisinde, ortak bir amaç, işlev ya da hizmet sunmak üzere bir araya getirilmiş ilişkili parçalardan oluşan düzenli bir kombinasyondur. Sistem esasen, işlevsel olarak ilişkili ürünler grubudur. Bir veya birden fazla `IfcProduct` örneğinin (sistem üyelerinin) gruplandırma ilişkisi, `IfcRelAssignsToGroup` aracılığıyla yönetilir. |

*Not: Bu parametreler IFC'de ilişkiler yoluyla kurgulanmaktadır.*

*[PDF sayfa 78]*

## Tablo 6.39 `IfcLightFixture` Sınıfı Zorunlu Öznitelik ve Özellik Listesi

**Zorunlu Öznitelik ve Özellik Listesi**

| Aydınlatma Armatürü (`IfcLightFixture`) | Ön Tanımlı Tip Listesi |
|---|---|
| Tanım: Aydınlatma Armatürü, bir veya daha fazla lambayı ve isteğe bağlı olarak bu lambaların ışık yayımını kontrol eden, sınırlayan veya değiştiren cihazları barındırmak amacıyla tasarlanmış bir muhafazadır. | DIRECTION SOURCE, POINT SOURCE, SECURITY LIGHTING, USERDEFINED, NOTDEFINED |

| Türkçe Tanım (Bilgi Amaçlı) | Öznitelik | İlişki | Veri Tipi (Bilgi Amaçlı) | Açıklama |
|---|---|---|---|---|
| Ön Tanımlı Tip | `PredefinedType` | - | Metin | Ön Tanımlı Tip. |
| İsim | Attribute: Name | - | Metin | IFC uyumlu yazılım sistemleri veya kullanıcılar tarafından kullanılmak üzere isteğe bağlı bir ad. [[`IfcRoot`]] sınıfının bazı alt tipleri için İsim niteliğinin eklenmesi gerekebilir. Bu, bir "where" kuralı ile zorunlu hale getirilecektir. (EK-5'e göre isimlendirilecektir) |
| Etiket | Attribute: Tag | - | Sayı veya Metin | Bir ürünün belirli bir örneğindeki etiket tanımlayıcısı, örneğin seri numarası veya konum numarasıdır. Bu, yerleştirilmiş örnek düzeyindeki tanımlayıcıdır. |
| Sistem | `IfcSystem` | `IfcRelAssignsToGroup>IfcSystem` | Metin | Bir sistem, Mimarlık, Mühendislik ve Yapım ürünü içerisinde, ortak bir amaç, işlev ya da hizmet sunmak üzere bir araya getirilmiş ilişkili parçalardan oluşan düzenli bir kombinasyondur. Sistem esasen, işlevsel olarak ilişkili ürünler grubudur. Bir veya birden fazla `IfcProduct` örneğinin (sistem üyelerinin) gruplandırma ilişkisi, `IfcRelAssignsToGroup` aracılığıyla yönetilir. |

| Türkçe Tanım (Bilgi Amaçlı) | Özellik | Özellik Seti / İlişki | Veri Tipi (Bilgi Amaçlı) | Açıklama |
|---|---|---|---|---|
| Nominal Güç Tüketimi | `NominalPowerConsumption` | `Pset_ElectricalDeviceCommon` | Sayı | Nominal toplam güç tüketimi. |

*Not: Bu parametreler IFC'de ilişkiler yoluyla kurgulanmaktadır.*

*[PDF sayfa 79]*

## Tablo 6.40 `IfcMember` Sınıfı Zorunlu Öznitelik ve Özellik Listesi

**Zorunlu Öznitelik ve Özellik Listesi**

| Eleman (`IfcMember`) | Ön Tanımlı Tip Listesi |
|---|---|
| Tanım: [[`IfcMember`]], destek noktaları arasında veya ötesinde yük taşımak üzere tasarlanmış yapısal bir elemandır. Taşıyıcı olması zorunlu değildir. Elemanın yönelimi (yatay, düşey veya eğimli olması) tanımı açısından önemli değildir ( [[`IfcBeam`]] ve [[IfcColumn]]'un aksine). [[`IfcMember`]], mimari veya yapısal modelleme bakış açısından doğrusal bir yapısal elemanı temsil eder ve eğer daha özel olarak [[`IfcBeam`]] veya [[`IfcColumn`]] olarak ifade edilemiyorsa kullanılmalıdır. | ARCH\_SEGMENT, BRACE, CHORD, COLLAR, MEMBER, MULLION, PLATE, POST, PURLIN, RAFTER, STAY\_CABLE, STIFFENING\_RIB, STRINGER, STRUCTURALCABLE, STRUT, STUD, SUSPENDER, SUSPENSION\_CABLE, TIEBAR, USERDEFINED, NOTDEFINED |

| Türkçe Tanım (Bilgi Amaçlı) | Öznitelik | İlişki | Veri Tipi (Bilgi Amaçlı) | Açıklama |
|---|---|---|---|---|
| Ön Tanımlı Tip | `PredefinedType` | - | Metin | Ön Tanımlı Tip. |
| İsim | Attribute: Name | - | Metin | IFC uyumlu yazılım sistemleri veya kullanıcılar tarafından kullanılmak üzere isteğe bağlı bir ad. [[`IfcRoot`]] sınıfının bazı alt tipleri için İsim niteliğinin eklenmesi gerekebilir. Bu, bir "where" kuralı ile zorunlu hale getirilecektir. |
| Etiket | Attribute: Tag | - | Sayı veya Metin | Bir ürünün belirli bir örneğindeki etiket tanımlayıcısı, örneğin seri numarası veya konum numarasıdır. Bu, yerleştirilmiş örnek düzeyindeki tanımlayıcıdır. |
| Malzeme | Attribute(IfcRe)AssociatesMaterial>IfcMaterial): Name\* | - | Metin | [`IfcMaterial`], yapı elemanlarını (fiziksel ürünler veya bileşenleri) oluşturmak için kullanılabilecek homojen veya heterojen maddelerdir. |

| Türkçe Tanım (Bilgi Amaçlı) | Özellik | İlişki | Veri Tipi (Bilgi Amaçlı) | Açıklama |
|---|---|---|---|---|
| Dışarıda | `IsExternal` | `Pset_MemberCommon` | Mantıksal | Yapı elemanının dışarıda kullanıma yönelik tasarlandığını (DOĞRU) veya tasarlanmadığını (YANLIŞ) gösterir. Eğer (DOĞRU) ise, bu bir dış eleman olup binanın dışına bakmaktadır. |

**"Attribute:": Standart bir IFC elemanının özniteliği**

\*Attribute (`IfcRelAssociatesMaterial>IfcMaterial`): Name\* bilgisi giriliken: "Malzeme İsmi\_Dayanım" formatı kullanılmalıdır. (Örnek:Çelik\_S355 Beton\_C25)

*[PDF sayfa 80]*

## Tablo 6.41 `IfcOpeningElement` Sınıfı Zorunlu Öznitelik ve Özellik Listesi

**Zorunlu Öznitelik ve Özellik Listesi**

| Açıklık Elemanı (`IfcOpeningElement`) | Ön Tanımlı Tip Listesi |
|---|---|
| Tanım: Açıklık elemanı; açıklık, girinti veya tesisat kanalı gibi boşlukları temsil eder. Fiziksel bir yapı elemanı içinde yer alan bir boşluğu ifade eder. Açıklıklar duvarlara, döşemelere, kirişlere, kolonlara veya diğer elemanlara yerleştirilebilir. | OPENING, RECESS, USERDEFINED, NOTDEFINED |

| Türkçe Tanım (Bilgi Amaçlı) | Öznitelik | İlişki | Veri Tipi (Bilgi Amaçlı) | Açıklama |
|---|---|---|---|---|
| Ön Tanımlı Tip | `PredefinedType` | - | Metin | Ön Tanımlı Tip. |
| İsim | Attribute: Name | - | Metin | IFC uyumlu yazılım sistemleri veya kullanıcılar tarafından kullanılmak üzere isteğe bağlı bir ad. [[`IfcRoot`]] sınıfının bazı alt tipleri için İsim niteliğinin eklenmesi gerekebilir. Bu, bir "where" kuralı ile zorunlu hale getirilecektir. (EK-5'e göre isimlendirilecektir) |

*[PDF sayfa 81]*

## Tablo 6.42 `IfcOutlet` Sınıfı Zorunlu Öznitelik ve Özellik Listesi

**Zorunlu Öznitelik ve Özellik Listesi**

| Priz (`IfcOutlet`) | Ön Tanımlı Tip Listesi |
|---|---|
| Tanım: Priz, elektrik enerjisi veya iletişim için bir veya birden fazla fişin takılmasını sağlamak amacıyla belirli bir noktaya monte edilen bir cihazdır. | AUDIO VISUAL OUTLET, COMMUNICATIONS OUTLET, DATA OUTLET, POWER OUTLET, TELEPHONE OUTLET, USERDEFINED, NOTDEFINED |

| Türkçe Tanım (Bilgi Amaçlı) | Öznitelik | İlişki | Veri Tipi (Bilgi Amaçlı) | Açıklama |
|---|---|---|---|---|
| Ön Tanımlı Tip | `PredefinedType` | - | Metin | Ön Tanımlı Tip. |
| İsim | Attribute: Name | - | Metin | IFC uyumlu yazılım sistemleri veya kullanıcılar tarafından kullanılmak üzere isteğe bağlı bir ad. [[`IfcRoot`]] sınıfının bazı alt tipleri için İsim niteliğinin eklenmesi gerekebilir. Bu, bir "where" kuralı ile zorunlu hale getirilecektir. (EK-5'e göre isimlendirilecektir) |
| Etiket | Attribute: Tag | - | Sayı veya Metin | Bir ürünün belirli bir örneğindeki etiket tanımlayıcısı, örneğin seri numarası veya konum numarasıdır. Bu, yerleştirilmiş örnek düzeyindeki tanımlayıcıdır. |
| Sistem | `IfcSystem` | `IfcRelAssignsToGroup>IfcSystem` | Metin | Bir sistem, Mimarlık, Mühendislik ve Yapım ürünü içerisinde, ortak bir amaç, işlev ya da hizmet sunmak üzere bir araya getirilmiş ilişkili parçalardan oluşan düzenli bir kombinasyondur. Sistem esasen, işlevsel olarak ilişkili ürünler grubudur. Bir veya birden fazla `IfcProduct` örneğinin (sistem üyelerinin) gruplandırma ilişkisi, `IfcRelAssignsToGroup` aracılığıyla yönetilir. |

*Not: Bu parametreler IFC'de ilişkiler yoluyla kurgulanmaktadır.*

*[PDF sayfa 82]*

## Tablo 6.43 `IfcPavement` Sınıfı Zorunlu Öznitelik ve Özellik Listesi

**Zorunlu Öznitelik ve Özellik Listesi**

| Asfalt (`IfcPavement`) | Ön Tanımlı Tip Listesi |
|---|---|
| Tanım: Yol veya diğer kaplamalı bir alanda, genellikle birden fazla tabakadan oluşan ve araçlar ya da yayalar tarafından gelen yükleri taşıyarak düzgün bir yüzey sağlayan yapı elemanı türüdür. NOT: ISO 6707-1'e göre tanım: yol, pist veya benzeri, taban zemini üzerinde yer alan yapı. | FLEXIBLE, RIGID, USERDEFINED, NOTDEFINED |

| Türkçe Tanım (Bilgi Amaçlı) | Öznitelik | İlişki | Veri Tipi (Bilgi Amaçlı) | Açıklama |
|---|---|---|---|---|
| Ön Tanımlı Tip | `PredefinedType` | - | Metin | Ön Tanımlı Tip. |
| İsim | Attribute: Name | - | Metin | IFC uyumlu yazılım sistemleri veya kullanıcılar tarafından kullanılmak üzere isteğe bağlı bir ad. [[`IfcRoot`]] sınıfının bazı alt tipleri için İsim niteliğinin eklenmesi gerekebilir. Bu, bir "where" kuralı ile zorunlu hale getirilecektir. (EK-5'e göre isimlendirilecektir) |
| Malzeme | Attribute (`IfcRelAssociatesMaterial>IfcMaterial`): Name\* | - | Metin | [`IfcMaterial`], yapı elemanlarını (fiziksel ürünler veya bileşenleri) oluşturmak için kullanılabilecek homojen veya heterojen maddelerdir. |
| Kaplama Malzemesi | Attribute: Material | Attribute(IfcRelAssociates Material>IfcMaterialLayerSetUsage >IfcMaterialLayerSet>IfcMaterialLayer): Material\* | Metin | Katmanın oluşturulduğu malzemeye yönelik isteğe bağlı referans. Bu değerin boş bırakılması, katmanın malzeme içermediğini (örneğin hava boşluğu olduğunu) ifade etmez; yalnızca ilgili aşamada malzeme bilgisinin tanımlanmadığını gösterir. |
| Kaplama Kalınlığı | Attribute: `LayerThickness` | Attribute(IfcRelAssociates Material>IfcMaterialLayerSetUsage>IfcMaterialLayerSet>IfcMaterialLayer): `LayerThickness`\* | Metin | Malzeme katmanının kalınlığıdır. "Kalınlık" teriminin anlamı, kullanımına bağlı olarak değişir. `IfcMaterialLayerSetUsage` kullanan yapı elemanlarında, bu boyut `IfcMaterialLayerSetUsage` içinde belirtilen pozitif `LayerSetDirection` doğrultusunda ölçülür. NOT: Zar (membran) gibi kalınlığı sıfıra çok yakın olan malzemeler için bu öznitelik değeri 0 olabilir. Kalınlığı 0 olan malzeme katmanları geometrik gösterimde görüntülenmeyebilir. |

*Not: Bu parametreler IFC'de ilişkiler yoluyla kurgulanmaktadır.*

**"Attribute:": Standart bir IFC elemanının özniteliği**

\*Attribute (`IfcRelAssociatesMaterial>IfcMaterial`): Name\* bilgisi giriliken: "Malzeme İsmi\_Dayanım" formatı kullanılmalıdır. (Örnek:Çelik\_S355 Beton\_C25)

*[PDF sayfa 83]*

## Tablo 6.44 `IfcPile` Sınıfı Zorunlu Öznitelik ve Özellik Listesi

**Zorunlu Öznitelik ve Özellik Listesi**

| Kazık (`IfcPile`)) | Ön Tanımlı Tip Listesi |
|---|---|
| Tanım: Bir kazık, bir yükü desteklemek amacıyla yere dik olarak yerleştirilmiş ince bir ahşap, beton veya çelik yapısal elemandır. Ayrıca, yüklerin daha derin yer altı katmanlarına aktarılmasıyla karakterize edilen derin temele de kazık denir. | BORED, DRIVEN, JETGROUTING, COHESION, FRICTION, SUPPORT, USERDEFINED, NOTDEFINED |

| Türkçe Tanım (Bilgi Amaçlı) | Öznitelik | İlişki | Veri Tipi (Bilgi Amaçlı) | Açıklama |
|---|---|---|---|---|
| Ön Tanımlı Tip | `PredefinedType` | - | Metin | Ön Tanımlı Tip. |
| İsim | Attribute: Name | - | Metin | IFC uyumlu yazılım sistemleri veya kullanıcılar tarafından kullanılmak üzere isteğe bağlı bir ad. [[`IfcRoot`]] sınıfının bazı alt tipleri için İsim niteliğinin eklenmesi gerekebilir. Bu, bir "where" kuralı ile zorunlu hale getirilecektir. (EK-5'e göre isimlendirilecektir) |
| Etiket | Attribute: Tag | - | Sayı veya Metin | Bir ürünün belirli bir örneğindeki etiket tanımlayıcısı, örneğin seri numarası veya konum numarasıdır. Bu, yerleştirilmiş örnek düzeyindeki tanımlayıcıdır. |
| Malzeme | Attribute(IfcRelAssociatesMaterial>IfcMaterial): Name\* | - | Metin | [`IfcMaterial`], yapı elemanlarını (fiziksel ürünler veya bileşenleri) oluşturmak için kullanılabilecek homojen veya heterojen maddelerdir. |

**"Attribute:": Standart bir IFC elemanının özniteliği**

\*Attribute (`IfcRelAssociatesMaterial>IfcMaterial`): Name\* bilgisi giriliken: "Malzeme İsmi\_Dayanım" formatı kullanılmalıdır. (Örnek:Çelik\_S355 Beton\_C25)

*[PDF sayfa 84]*

## Tablo 6.45 `IfcPipeFitting` Sınıfı Zorunlu Öznitelik ve Özellik Listesi

**Zorunlu Öznitelik ve Özellik Listesi**

| Boru Bağlantı Parçası (`IfcPipeFitting`) | Ön Tanımlı Tip Listesi |
|---|---|
| Tanım: Boru bağlantı parçası, borulu akış dağıtım sisteminde bir bağlantı veya geçiş elemanıdır; boru segmentlerini birleştirmek için kullanılır ve akışkanın yönü veya debisi gibi akış özelliklerinde değişikliklere neden olur. | BEND, CONNECTOR, ENTRY, EXIT, JUNCTION, OBSTRUCTION, TRANSITION, USERDEFINED, NOTDEFINED |

| Türkçe Tanım (Bilgi Amaçlı) | Öznitelik | İlişki | Veri Tipi (Bilgi Amaçlı) | Açıklama |
|---|---|---|---|---|
| Ön Tanımlı Tip | `PredefinedType` | - | Metin | Ön Tanımlı Tip. |
| İsim | Attribute: Name | - | Metin | IFC uyumlu yazılım sistemleri veya kullanıcılar tarafından kullanılmak üzere isteğe bağlı bir ad. [[`IfcRoot`]] sınıfının bazı alt tipleri için İsim niteliğinin eklenmesi gerekebilir. Bu, bir "where" kuralı ile zorunlu hale getirilecektir. (EK-5'e göre isimlendirilecektir) |
| Etiket | Attribute: Tag | - | Sayı veya Metin | Bir ürünün belirli bir örneğindeki etiket tanımlayıcısı, örneğin seri numarası veya konum numarasıdır. Bu, yerleştirilmiş örnek düzeyindeki tanımlayıcıdır. |
| Sistem | `IfcSystem` | `IfcRelAssignsToGroup>IfcSystem` | Metin | Bir sistem, Mimarlık, Mühendislik ve Yapım ürünü içerisinde, ortak bir amaç, işlev ya da hizmet sunmak üzere bir araya getirilmiş ilişkili parçalardan oluşan düzenli bir kombinasyondur. Sistem esasen, işlevsel olarak ilişkili ürünler grubudur. Bir veya birden fazla `IfcProduct` örneğinin (sistem üyelerinin) gruplandırma ilişkisi, `IfcRelAssignsToGroup` aracılığıyla yönetilir. |

*Not: Bu parametreler IFC'de ilişkiler yoluyla kurgulanmaktadır.*

*[PDF sayfa 85]*

## Tablo 6.46 `IfcPipeSegment` Zorunlu Öznitelik ve Özellik Listesi

**Zorunlu Öznitelik ve Özellik Listesi**

| Boru Segmenti (`IfcPipeSegment`) | Ön Tanımlı Tip Listesi |
|---|---|
| Tanım: Boru segmenti, tipik olarak bir boru şebekesinin iki bölümünü birleştirmek için kullanılır. | CULVERT, FLEXIBLESEGMENT, RIGIDSEGMENT, GUTTER, SPOOL, USERDEFINED, NOTDEFINED |

| Türkçe Tanım (Bilgi Amaçlı) | Öznitelik | İlişki | Veri Tipi (Bilgi Amaçlı) | Açıklama |
|---|---|---|---|---|
| Ön Tanımlı Tip | `PredefinedType` | - | Metin | Ön Tanımlı Tip. |
| İsim | Attribute: Name | - | Metin | IFC uyumlu yazılım sistemleri veya kullanıcılar tarafından kullanılmak üzere isteğe bağlı bir ad. [[`IfcRoot`]] sınıfının bazı alt tipleri için İsim niteliğinin eklenmesi gerekebilir. Bu, bir "where" kuralı ile zorunlu hale getirilecektir. (EK-5'e göre isimlendirilecektir) |
| Etiket | Attribute: Tag | - | Sayı veya Metin | Bir ürünün belirli bir örneğindeki etiket tanımlayıcısı, örneğin seri numarası veya konum numarasıdır. Bu, yerleştirilmiş örnek düzeyindeki tanımlayıcıdır. |
| "Sistem | `IfcSystem` | `IfcRelAssignsToGroup>IfcSystem` | Metin | Bir sistem, Mimarlık, Mühendislik ve Yapım ürünü içerisinde, ortak bir amaç, işlev ya da hizmet sunmak üzere bir araya getirilmiş ilişkili parçalardan oluşan düzenli bir kombinasyondur. Sistem esasen, işlevsel olarak ilişkili ürünler grubudur. Bir veya birden fazla `IfcProduct` örneğinin ( sistem üyelerinin) gruplandırma ilişkisi, `IfcRelAssignsToGroup` aracılığıyla yönetilir. |

| Türkçe Tanım (Bilgi Amaçlı) | Özellik | Özellik Seti / İlişki | Veri Tipi (Bilgi Amaçlı) | Açıklama |
|---|---|---|---|---|
| Nominal Çap | `NominalDiameter` | `Pset_PipeSegmentTypeCommon` | Sayı | Nesnenin nominal çapı veya genişliği. |

*Not: Bu parametreler IFC'de ilişkiler yoluyla kurgulanmaktadır.*

*[PDF sayfa 86]*

## Tablo 6.47 `IfcPump` Sınıfı Zorunlu Öznitelik ve Özellik Listesi

**Zorunlu Öznitelik ve Özellik Listesi**

| Pompa (`IfcPump`) | Ön Tanımlı Tip Listesi |
|---|---|
| Tanım: Pompa, akışkanlara veya çamurlara mekanik enerji aktararak bunların bir kanal veya boru sistemi içinde taşınmasını sağlayan ekipmandır. Pompanın tipik bir kullanımı, bina dağıtım sisteminde soğutulmuş su veya ısıtma sıcak suyunun dolaştırılmasıdır. | CIRCULATOR, END SUCTION, SPLIT CASE, SUBMERSIBLE PUMP, SUMP PUMP, VERTICAL INLINE, VERTICAL TURBINE, USERDEFINED, NOTDEFINED |

| Türkçe Tanım (Bilgi Amaçlı) | Öznitelik | İlişki | Veri Tipi (Bilgi Amaçlı) | Açıklama |
|---|---|---|---|---|
| Ön Tanımlı Tip | `PredefinedType` | - | Metin | Ön Tanımlı Tip. |
| İsim | Attribute: Name | - | Metin | IFC uyumlu yazılım sistemleri veya kullanıcılar tarafından kullanılmak üzere isteğe bağlı bir ad. [[`IfcRoot`]] sınıfının bazı alt tipleri için İsim niteliğinin eklenmesi gerekebilir. Bu, bir "where" kuralı ile zorunlu hale getirilecektir. (EK-5'e göre isimlendirilecektir) |
| Etiket | Attribute: Tag | - | Sayı veya Metin | Bir ürünün belirli bir örneğindeki etiket tanımlayıcısı, örneğin seri numarası veya konum numarasıdır. Bu, yerleştirilmiş örnek düzeyindeki tanımlayıcıdır. |
| Sistem | `IfcSystem` | `IfcRelAssignsToGroup>IfcSystem` | Metin | Bir sistem, Mimarlık, Mühendislik ve Yapım ürünü içerisinde, ortak bir amaç, işlev ya da hizmet sunmak üzere bir araya getirilmiş ilişkili parçalardan oluşan düzenli bir kombinasyondur. Sistem esasen, işlevsel olarak ilişkili ürünler grubudur. Bir veya birden fazla `IfcProduct` örneğinin (sistem üyelerinin) gruplandırma ilişkisi, `IfcRelAssignsToGroup` aracılığıyla yönetilir. |

| Türkçe Tanım (Bilgi Amaçlı) | Özellik | Özellik Seti / İlişki | Veri Tipi (Bilgi Amaçlı) | Açıklama |
|---|---|---|---|---|
| Akış Debisi Aralığı | `FlowRateRange` | `Pset_PumpTypeCommon` | Metin | Belirtilen dirence karşı pompalanan akışkanın izin verilen hacim aralığı. |
| Nominal Güç Tüketimi | `NominalPowerConsumption` | `Pset_ElectricalDeviceCommon` | Sayı | Nominal toplam güç tüketimi. |

*Not: Bu parametreler IFC'de ilişkiler yoluyla kurgulanmaktadır.*

*[PDF sayfa 87]*

## Tablo 6.48 `IfcRailing` Sınıfı Zorunlu Öznitelik ve Özellik Listesi

**Zorunlu Öznitelik ve Özellik Listesi**

| Korkuluk (`IfcRailing`) | Ön Tanımlı Tip Listesi |
|---|---|
| Tanım: Korkuluk, insan veya araç dolaşım alanlarının bitişiğinde ya da bazı mekân sınırlarında, duvarların yerine veya duvarları tamamlayıcı bir eleman olarak kullanılan çerçeve biçimindeki bir yapıdır. İnsanlara fiziksel destek sağlamak veya düşme ya da çarpışma sonucu oluşabilecek yaralanma ya da hasarları önlemek amacıyla tasarlanmıştır. | HAND RAIL, GUARD RAIL, BALUSTRADE, USERDEFINED, NOTDEFINED |

| Türkçe Tanım (Bilgi Amaçlı) | Öznitelik | İlişki | Veri Tipi (Bilgi Amaçlı) | Açıklama |
|---|---|---|---|---|
| Ön Tanımlı Tip | `PredefinedType` | - | Metin | Ön Tanımlı Tip. |
| İsim | Attribute: Name | - | Metin | IFC uyumlu yazılım sistemleri veya kullanıcılar tarafından kullanılmak üzere isteğe bağlı bir ad. [[`IfcRoot`]] sınıfının bazı alt tipleri için İsim niteliğinin eklenmesi gerekebilir. Bu, bir "where" kuralı ile zorunlu hale getirilecektir. (EK-5'e göre isimlendirilecektir) |
| Etiket | Attribute: Tag | - | Sayı veya Metin | Bir ürünün belirli bir örneğindeki etiket tanımlayıcısı, örneğin seri numarası veya konum numarasıdır. Bu, yerleştirilmiş örnek düzeyindeki tanımlayıcıdır. |

*[PDF sayfa 88]*

## Tablo 6.49 `IfcRamp` Sınıfı Zorunlu Öznitelik ve Özellik Listesi

**Zorunlu Öznitelik ve Özellik Listesi**

| Rampa (`IfcRamp`) | Ön Tanımlı Tip Listesi |
|---|---|
| Tanım: Rampa, farklı kotlardaki iki döşeme seviyesini birbirine bağlayan dikey bir geçiş yoludur ve insan dolaşımını sağlar. Ara bir döşeme plağı olarak bir sahanlık içerebilir. Bir rampa normalde basamak içermez. | STRAIGHT RUN RAMP, TWO STRAIGHT RUN RAMP, QUARTER TURN RAMP, TWO QUARTER TURN RAMP: HALF TURN RAMP, SPIRAL RAMP, USERDEFINED, NOTDEFINED |

| Türkçe Tanım (Bilgi Amaçlı) | Öznitelik | İlişki | Veri Tipi (Bilgi Amaçlı) | Açıklama |
|---|---|---|---|---|
| Ön Tanımlı Tip | `PredefinedType` | - | Metin | Ön Tanımlı Tip. |
| İsim | Attribute: Name | - | Metin | IFC uyumlu yazılım sistemleri veya kullanıcılar tarafından kullanılmak üzere isteğe bağlı bir ad. [[`IfcRoot`]] sınıfının bazı alt tipleri için İsim niteliğinin eklenmesi gerekebilir. Bu, bir "where" kuralı ile zorunlu hale getirilecektir. (EK-5'e göre isimlendirilecektir) |
| Etiket | Attribute: Tag | - | Sayı veya Metin | Bir ürünün belirli bir örneğindeki etiket tanımlayıcısı, örneğin seri numarası veya konum numarasıdır. Bu, yerleştirilmiş örnek düzeyindeki tanımlayıcıdır. |
| Malzeme | Attribute(IfcRelAssociatesMaterial>IfcMaterial): Name\* | - | Metin | [`IfcMaterial`], yapı elemanlarını (fiziksel ürünler veya bileşenleri) oluşturmak için kullanılabilecek homojen veya heterojen maddelerdir. |

**"Attribute:": Standart bir IFC elemanının özniteliği**

\*Attribute (`IfcRelAssociatesMaterial>IfcMaterial`): Name\* bilgisi giriliken: "Malzeme İsmi\_Dayanım" formatı kullanılmalıdır. (Örnek:Çelik\_S355 Beton\_C25)

*[PDF sayfa 89]*

## Tablo 6.50 `IfcRoad` Sınıfı Zorunlu Öznitelik ve Özellik Listesi

**Zorunlu Öznitelik ve Özellik Listesi**

| Yol (`IfcRoad`) | Ön Tanımlı Tip Listesi |
|---|---|
| Tanım: Karayolları, caddeler, bisiklet ve yaya yolları dâhil ancak demiryolları hariç olmak üzere, bir yerden başka bir yere seyahati sağlamak amacıyla kara üzerinde inşa edilen bir güzergâhtır. Bir tesis türü olarak yol, yol projesinin bileşenleri için (örneğin tasarım, inşaat veya bakım gibi herhangi bir çalışma) proje yapısı hiyerarşisinde temel unsuru oluşturur. | BICYCLE CROSSING, BUS STOP, CARRIAGEWAY, CENTRAL ISLAND, CENTRAL RESERVE, HARD SHOULDER, INTERSECTION, LAY-BY, PARKING BAY, PASSING BAY, PEDESTRIAN CROSSING, REFUGE ISLAND, ROAD SEGMENT, ROAD SIDE, ROAD SIDE PART, ROADWAY PLATEAU, ROUNDABOUT, SHOULDER, SIDEWALK, SOFT SHOULDER, TOLL PLAZA, TRAFFIC ISLAND, TRAFFIC LANE, USERDEFINED, NOTDEFINED |

| Türkçe Tanım (Bilgi Amaçlı) | Öznitelik | İlişki | Veri Tipi (Bilgi Amaçlı) | Açıklama |
|---|---|---|---|---|
| Ön Tanımlı Tip | `PredefinedType` | - | Metin | Ön Tanımlı Tip. |
| İsim | Attribute: Name | - | Metin | IFC uyumlu yazılım sistemleri veya kullanıcılar tarafından kullanılmak üzere isteğe bağlı bir ad. [[`IfcRoot`]] sınıfının bazı alt tipleri için İsim niteliğinin eklenmesi gerekebilir. Bu, bir "where" kuralı ile zorunlu hale getirilecektir. (EK-5'e göre isimlendirilecektir) |
| Malzeme | Attribute(IfcRelAssociatesMaterial>IfcMaterial): Name\* | - | Metin | [`IfcMaterial`], yapı elemanlarını (fiziksel ürünler veya bileşenleri) oluşturmak için kullanılabilecek homojen veya heterojen maddelerdir. |

**"Attribute:": Standart bir IFC elemanının özniteliği**

\*Attribute (`IfcRelAssociatesMaterial>IfcMaterial`): Name\* bilgisi giriliken: "Malzeme İsmi\_Dayanım" formatı kullanılmalıdır. (Örnek:Çelik\_S355 Beton\_C25)

*[PDF sayfa 90]*

## Tablo 6.51 `IfcRoadPart` Sınıfı Zorunlu Öznitelik ve Özellik Listesi

**Zorunlu Öznitelik ve Özellik Listesi**

| Yol Parçası (`IfcRoadPart`) | Ön Tanımlı Tip Listesi |
|---|---|
| Tanım: Bir yolun parçası. | BICYCLE CROSSING, BUS STOP, CARRIAGEWAY, CENTRAL ISLAND, CENTRAL RESERVE, HARD SHOULDER, INTERSECTION, LAY-BY, PARKING BAY, PASSING BAY, PEDESTRIAN CROSSING, REFUGE ISLAND, ROAD SEGMENT, ROAD SIDE, ROAD SIDE PART, ROADWAY PLATEAU, ROUNDABOUT, SHOULDER, SIDEWALK, SOFT SHOULDER, TOLL PLAZA, TRAFFIC ISLAND, TRAFFIC LANE, USERDEFINED, NOTDEFINED |

| Türkçe Tanım (Bilgi Amaçlı) | Öznitelik | İlişki | Veri Tipi (Bilgi Amaçlı) | Açıklama |
|---|---|---|---|---|
| Ön Tanımlı Tip | `PredefinedType` | - | Metin | Ön Tanımlı Tip. |
| İsim | Attribute: Name | - | Metin | IFC uyumlu yazılım sistemleri veya kullanıcılar tarafından kullanılmak üzere isteğe bağlı bir ad. [[`IfcRoot`]] sınıfının bazı alt tipleri için İsim niteliğinin eklenmesi gerekebilir. Bu, bir "where" kuralı ile zorunlu hale getirilecektir. (EK-5'e göre isimlendirilecektir) |
| Malzeme | Attribute(IfcRelAssociatesMaterial>IfcMaterial): Name\* | - | Metin | [`IfcMaterial`], yapı elemanlarını (fiziksel ürünler veya bileşenleri) oluşturmak için kullanılabilecek homojen veya heterojen maddelerdir. |

**"Attribute:": Standart bir IFC elemanının özniteliği**

\*Attribute (`IfcRelAssociatesMaterial>IfcMaterial`): Name\* bilgisi giriliken: "Malzeme İsmi\_Dayanım" formatı kullanılmalıdır. (Örnek:Çelik\_S355 Beton\_C25)

*[PDF sayfa 91]*

## Tablo 6.52 `IfcRoof` Sınıfı Zorunlu Öznitelik ve Özellik Listesi

**Zorunlu Öznitelik ve Özellik Listesi**

| Çatı (`IfcRoof`) | Ön Tanımlı Tip Listesi |
|---|---|
| Tanım: Çatı, bir binanın en üst kısmını örten yapı elemanıdı; binayı hava koşullarının etkilerine karşı korur. | FLAT ROOF, SHED ROOF, GABLE ROOF, HIP ROOF, HIPPED GABLE ROOF, GAMBREL ROOF, MANSARD ROOF, BARREL ROOF, RAINBOW ROOF, BUTTERFLY ROOF, PAVILION ROOF, DOME ROOF, FREEFORM, USERDEFINED, NOTDEFINED |

| Türkçe Tanım (Bilgi Amaçlı) | Öznitelik | İlişki | Veri Tipi (Bilgi Amaçlı) | Açıklama |
|---|---|---|---|---|
| Ön Tanımlı Tip | `PredefinedType` | - | Metin | Ön Tanımlı Tip. |
| İsim | Attribute: Name | - | Metin | IFC uyumlu yazılım sistemleri veya kullanıcılar tarafından kullanılmak üzere isteğe bağlı bir ad. [[`IfcRoot`]] sınıfının bazı alt tipleri için İsim niteliğinin eklenmesi gerekebilir. Bu, bir "where" kuralı ile zorunlu hale getirilecektir. (EK-5'e göre isimlendirilecektir) |
| Etiket | Attribute: Tag | - | Sayı veya Metin | Bir ürünün belirli bir örneğindeki etiket tanımlayıcısı, örneğin seri numarası veya konum numarasıdır. Bu, yerleştirilmiş örnek düzeyindeki tanımlayıcıdır. |

| Türkçe Tanım (Bilgi Amaçlı) | Özellik | Özellik Seti / İlişki | Veri Tipi (Bilgi Amaçlı) | Açıklama |
|---|---|---|---|---|
| Dışarıda | `IsExternal` | `Pset_RoofCommon` | Mantıksal | Yapı elemanının dışarıda kullanıma yönelik tasarlandığını (DOĞRU) veya tasarlanmadığını (YANLIŞ) gösterir. Eğer (DOĞRU) ise, bu bir dış eleman olup binanın dışına bakmaktadır. |
| Yangın Dayanım Sınıfı | `FireRating` | `Pset_RoofCommon` | Metin | Bu nesnenin yangın dayanım sınıfı. Ulusal yangın güvenliği sınıflandırmasına göre (Türkiye için: Binaların Yangından Korunması Hakkında Yönetmeliğe göre) verilmiştir. |

*[PDF sayfa 92]*

## Tablo 6.53 `IfcSanitaryTerminal` Sınıfı Zorunlu Öznitelik ve Özellik Listesi

**Zorunlu Öznitelik ve Özellik Listesi**

| Sıhhi Terminal (`IfcSanitaryTerminal`) | Ön Tanımlı Tip Listesi |
|---|---|
| Tanım: Sıhhi terminal, genellikle su ile beslenen ve içme, temizlik veya atık suyun uzaklaştırılması için kullanılan sabit bir cihaz veya terminaldir ya da böyle bir cihaz veya terminal ile doğrudan kullanılan bir ekipman öğesidir. | BATH, BIDET, CISTERN, SHOWER, SINK, SANITARY FOUNTAIN, TOILET PAN, URINAL, WASH HAND BASIN, WC SEAT, USERDEFINED, NOTDEFINED |

| Türkçe Tanım (Bilgi Amaçlı) | Öznitelik | İlişki | Veri Tipi (Bilgi Amaçlı) | Açıklama |
|---|---|---|---|---|
| Ön Tanımlı Tip | `PredefinedType` | - | Metin | Ön Tanımlı Tip. |
| İsim | Attribute: Name | - |  Metin | IFC uyumlu yazılım sistemleri veya kullanıcılar tarafından kullanılmak üzere isteğe bağlı bir ad. [[`IfcRoot`]] sınıfının bazı alt tipleri için İsim niteliğinin eklenmesi gerekebilir. Bu, bir "where" kuralı ile zorunlu hale getirilecektir. (EK-5'e göre isimlendirilecektir) |
| Etiket | Attribute: Tag | - | Sayı veya Metin | Bir ürünün belirli bir örneğindeki etiket tanımlayıcısı, örneğin seri numarası veya konum numarasıdır. Bu, yerleştirilmiş örnek düzeyindeki tanımlayıcıdır. |

*[PDF sayfa 93]*

## Tablo 6.54 `IfcSign` Sınıfı Zorunlu Öznitelik ve Özellik Listesi

**Zorunlu Öznitelik ve Özellik Listesi**

| Tabela (`IfcSign`) | Ön Tanımlı Tip Listesi |
|---|---|
| Tanım: Levha (tabela), yazılı, sembolik veya başka bir biçimde bilgi ya da talimat veren görsel bir bildirimdir. Tabelalar pasiftir; en yaygın biçimi görsel panodur. `IfcSign` örneği, duvar gibi bir yüzeye uygulanan bir panelin yerleştirilmiş veya bir Sinyal Bileşimi içinde toplanan birden fazla tabela örneği yerleştirilmesini ve ilgili taşıyıcı yapısal elemanlarını ifade eder. | MARKER, PICTORAL, MIRROR, USERDEFINED, NOTDEFINED |

| Türkçe Tanım (Bilgi Amaçlı) | Öznitelik | İlişki | Veri Tipi (Bilgi Amaçlı) | Açıklama |
|---|---|---|---|---|
| Ön Tanımlı Tip | `PredefinedType` | - | Metin | Ön Tanımlı Tip. |
| İsim | Attribute: Name | - | Metin | IFC uyumlu yazılım sistemleri veya kullanıcılar tarafından kullanılmak üzere isteğe bağlı bir ad. [[`IfcRoot`]] sınıfının bazı alt tipleri için İsim niteliğinin eklenmesi gerekebilir. Bu, bir "where" kuralı ile zorunlu hale getirilecektir. (EK-5'e göre isimlendirilecektir) |
| Etiket | Attribute: Tag | - | Sayı veya Metin | Bir ürünün belirli bir örneğindeki etiket tanımlayıcısı, örneğin seri numarası veya konum numarasıdır. Bu, yerleştirilmiş örnek düzeyindeki tanımlayıcıdır. |

*[PDF sayfa 94]*

## Tablo 6.55 `IfcSite` Sınıfı Zorunlu Öznitelik ve Özellik Listesi

**Zorunlu Öznitelik ve Özellik Listesi**

| Saha (`IfcSite`) | Ön Tanımlı Tip Listesi |
|---|---|
| Tanım: Saha, projenin inşaatının gerçekleştirileceği, suyla kaplı olabilecek sınırları belirli bir kara alanıdır. Bir saha, bina inşa etmek, mevcut yapıları yenilemek veya yıkmak ya da diğer inşaatla ilgili geliştirme faaliyetleri için kullanılabilir. | - |

| Türkçe Tanım (Bilgi Amaçlı) | Öznitelik | İlişki | Veri Tipi (Bilgi Amaçlı) | Açıklama |
|---|---|---|---|---|
| Nesne Tipi | Attribute: `ObjectType` | - | Metin | Tip, nesneyi daha ayrıntılı tanımlayan özel bir tipi belirtir. Kullanım, örneklenebilir somut alt tiplerin seviyesinde belirlenmelidir. Şu durumlarda kullanıcı tarafından tanımlanan tip bilgisi bu özniteliğe girilir: 1) Ön Tanımlı Tip özniteliğinin değeri listeden [[USERDEFINED]] olarak seçilmişse veya 2) somut örneklenmiş varlıkta bir Ön Tanımlı Tip özniteliği yoksa. İkinci durum, bazı istisnai yaprak sınıflarında ve [[IfcBuiltElement]]'i doğrudan örneklendirirken geçerlidir. |
| Uzun İsim | Attribute: `LongName` | - | Metin | Referans amaçlı olarak kullanılan bağlamın uzun adı. |
| İsim | Attribute: Name | - | Metin | IFC uyumlu yazılım sistemleri veya kullanıcılar tarafından kullanılmak üzere isteğe bağlı bir ad. [[`IfcRoot`]] sınıfının bazı alt tipleri için İsim niteliğinin eklenmesi gerekebilir. Bu, bir "where" kuralı ile zorunlu hale getirilecektir. (EK-5'e göre isimlendirilecektir) |
| Referans Kotu | Attribute:RefElevation | - | Sayı | Deniz seviyesine göre referans (datum) kotudur. |

| Türkçe Tanım (Bilgi Amaçlı) | Özellik | Özellik Seti / İlişki | Veri Tipi (Bilgi Amaçlı) | Açıklama |
|---|---|---|---|---|
| Kat Alanı Oranı | `FloorAreaRatio` | `Pset_SiteCommon` | Sayı | Yerel imar yönetmeliklerine (Planlı Alanlar İmar Yönetmeliğine) göre belirlenen. toplam kat alanının yapılaşabilir alana oranı olarak ifade edilen azami kat alanı kullanım oranıdır. |
| Saha Kapsama Oranı | `SiteCoverageRatio` | `Pset_SiteCommon` | Sayı | Kullanım oranı, [[`ToplamAlan`]] / [[YapılaşabilirAlan]] biçiminde azami bir değer olarak ifade edilir. Bu oran değeri, [[YapılaşabilirAlan]]'ın türetilmesinde kullanılabilir. |
| Toplam Alan | `TotalArea` | `Pset_SiteCommon` | Sayı | Arsaya ilişkin toplam planlanan alandır. Arsa alanının programlanması amacıyla kullanılır. |
| Adres Satırları | `AddressLines` | `Pset_Address` | Metin | Posta adresidir. NOT: Bir posta adresi kaydedilirken birden fazla satır (veya öğe) içerebilir. Normal kullanımda, aşağıdaki adres kavramlarının ilgili unsurlarını |

*[PDF sayfa 95]*

*(Tablo önceki sayfadan sürüyor; sütun başlıkları okunabilirlik için yinelendi.)*

| Türkçe Tanım (Bilgi Amaçlı) | Özellik | Özellik Seti / İlişki | Veri Tipi (Bilgi Amaçlı) | Açıklama |
|---|---|---|---|---|
|  |  |  |  | içermesi beklenir: Bina içindeki konum (ör. 3. Kat) Bina adı (ör. Interoperability House) Bina numarası (ör. 6400) Sokak adı (ör. Alliance Boulevard) Adres satırlarının tipik içeriği ülkeden ülkeye değişiklik gösterebilir. |
| I1 / Kasaba | Town | `Pset_Address` | Metin | Bir kasaba veya şehrin adıdır. |
| Bölge / İlçe | Region | `Pset_Address` | Metin | Bir bölge veya ilçenin adıdır. |
| Parsel Numarası | `LandID` | `Pset_LandRegistration` | Metin | Yasal tescil makamı tarafından bir taşınmaz parseline atanan kimlik numarasıdır. |
| Tapu Numarası | `LandTitleID` | `Pset_LandRegistration` | Metin | Yasal tescil makamı tarafından bir taşınmaz parseline ait tapuya atanan kimlik numarasıdır. |
|  | TAKS | `TREpys_ParselOzellikSeti` | Sayı | Taban Alanı Kat Sayısı'nı ifade eder. |
|  | KAKS | `TREpys_ParselOzellikSeti` | Sayı | Kat Alanı Kat Sayısı'nı ifade eder. |
|  | `CekmeOn` | `TREpys_ParselOzellikSeti` | Sayı | Ön bahçe mesafesi: Yapı ile parselin ön sınırı arasındaki yatay uzaklıktır. |
|  | `CekmeYan` | `TREpys_ParselOzellikSeti` | Sayı | Yan bahçe mesafesi: Yapı ile parselin yan sınırları arasındaki yatay uzaklıktır. |
|  | `CekmeArka` | `TREpys_ParselOzellikSeti` | Sayı | Arka bahçe mesafesi: Yapı ile parselin arka sınırı arasındaki yatay uzaklıktır. |

*[PDF sayfa 96]*

## Tablo 6.56 `IfcSlab` Sınıfı Zorunlu Öznitelik ve Özellik Listesi

**Zorunlu Öznitelik ve Özellik Listesi**

| Döşeme (`IfcSlab`) | Ön Tanımlı Tip Listesi |
|---|---|
| Tanım: Döşeme, düşey olarak bir hacmi kapatabilen yapı bileşenidir. Döşeme, alt taşıyıcıyı (kat döşemesi) veya üst katı (çatı döşemesi) sağlayabilir ve bir binadaki herhangi bir mekânda yer alabilir. | APPROACH SLAB, BASE SLAB, FLOOR, LANDING, PAVING, ROOF, SIDEWALK, TRACK SLAB, WEARING, USERDEFINED, NOTDEFINED |

| Türkçe Tanım (Bilgi Amaçlı) | Öznitelik | İlişki | Veri Tipi (Bilgi Amaçlı) | Açıklama |
|---|---|---|---|---|
| Ön Tanımlı Tip | `PredefinedType` | - | Metin | Ön Tanımlı Tip. |
| İsim | Attribute: Name | - | Metin | IFC uyumlu yazılım sistemleri veya kullanıcılar tarafından kullanılmak üzere isteğe bağlı bir ad. [[`IfcRoot`]] sınıfının bazı alt tipleri için İsim niteliğinin eklenmesi gerekebilir. Bu, bir "where" kuralı ile zorunlu hale getirilecektir. (EK-5'e göre isimlendirilecektir) |
| Etiket | Attribute: Tag | - | Sayı veya Metin | Bir ürünün belirli bir örneğindeki etiket tanımlayıcısı, örneğin seri numarası veya konum numarasıdır. Bu, yerleştirilmiş örnek düzeyindeki tanımlayıcıdır. |
| Malzeme | Attribute(IfcRelAssociatesMaterial>IfcMaterial): Name\* | - | Metin | [[`IfcMaterial`]], Elemanlar (fiziksel ürünler veya bileşenleri) oluşturmak için kullanılabilecek homojen veya heterojen maddelerdir. |

| Türkçe Tanım (Bilgi Amaçlı) | Özellik | Özellik Seti / İlişki | Veri Tipi (Bilgi Amaçlı) | Açıklama |
|---|---|---|---|---|
| Dışarıda | `IsExternal` | `Pset_SlabCommon` | Mantıksal | Yapı elemanının dışarıda kullanıma yönelik tasarlandığını (DOĞRU) veya tasarlanmadığını (YANLIŞ) gösterir. Eğer (DOĞRU) ise, bu bir dış eleman olup binanın dışına bakmaktadır. |
| Yangın Dayanım Sınıfı | `FireRating` | `Pset_SlabCommon` | Metin | Bu nesnenin yangın dayanım sınıfı. Ulusal yangın güvenliği sınıflandırmasına göre (Türkiye için: Binaların Yangından Korunması Hakkında Yönetmeliğe göre) verilmiştir. |

\*Attribute (`IfcRelAssociatesMaterial>IfcMaterial`): Name\* bilgisi giriliken: "Malzeme İsmi\_Dayanım" formatı kullanılmalıdır. (Örnek:Çelik\_S355 Beton\_(25)

*[PDF sayfa 97]*

## Tablo 6.57 `IfcSolarDevice` Sınıfı Zorunlu Öznitelik ve Özellik Listesi

**Zorunlu Öznitelik ve Özellik Listesi**

| Güneş Enerjisi Cihazı (`IfcSolarDevice`) | Ön Tanımlı Tip Listesi |
|---|---|
| Tanım: Güneş enerjisi cihazı, güneş radyasyonunu elektrik akımı veya ısıl enerji gibi diğer enerji türlerine dönüştürür. | SOLAR COLLECTOR, SOLAR PANEL, USERDEFINED, NOTDEFINED |

| Türkçe Tanım (Bilgi Amaçlı) | Öznitelik | İlişki | Veri Tipi (Bilgi Amaçlı) | Açıklama |
|---|---|---|---|---|
| Ön Tanımlı Tip | `PredefinedType` | - | Metin | Ön Tanımlı Tip. |
| İsim | Attribute: Name | - | Metin | IFC uyumlu yazılım sistemleri veya kullanıcılar tarafından kullanılmak üzere isteğe bağlı bir ad. [[`IfcRoot`]] sınıfının bazı alt tipleri için İsim niteliğinin eklenmesi gerekebilir. Bu, bir "where" kuralı ile zorunlu hale getirilecektir. (EK-5'e göre isimlendirilecektir) |
| Etiket | Attribute: Tag | - | Sayı veya Metin | Bir ürünün belirli bir örneğindeki etiket tanımlayıcısı, örneğin seri numarası veya konum numarasıdır. Bu, yerleştirilmiş örnek düzeyindeki tanımlayıcıdır. |
| Sistem | `IfcSystem` | `IfcRelAssignsToGroup>IfcSystem` | Metin | Bir sistem, Mimarlık, Mühendislik ve Yapım ürünü içerisinde, ortak bir amaç, işlev ya da hizmet sunmak üzere bir araya getirilmiş ilişkili parçalardan oluşan düzenli bir kombinasyondur. Sistem esasen, işlevsel olarak ilişkili ürünler grubudur. Bir veya birden fazla `IfcProduct` örneğinin (sistem üyelerinin) gruplandırma ilişkisi, `IfcRelAssignsToGroup` aracılığıyla yönetilir. |

*Not: Bu parametreler IFC'de ilişkiler yoluyla kurgulanmaktadır.*

*[PDF sayfa 98]*

## Tablo 6.58 `IfcSpace` Sınıfı Zorunlu Öznitelik ve Özellik Listesi

**Zorunlu Öznitelik ve Özellik Listesi**

| Mahal (`IfcSpace`) | Ön Tanımlı Tip Listesi |
|---|---|
| Tanım: Bir mahal, gerçek veya varsayımsal olarak sınırlandırılmış bir alanı veya hacmi temsil eder. Mekanlar, bir bina içinde belirli işlevleri yerine getiren alanlar veya hacimlerdir. | SPACE, PARKING, GFA, INTERNAL, EXTERNAL, USERDEFINED, NOTDEFINED |

| Türkçe Tanım (Bilgi Amaçlı) | Öznitelik | İlişki | Veri Tipi (Bilgi Amaçlı) | Açıklama |
|---|---|---|---|---|
| Ön Tanımlı Tip | `PredefinedType` | - | Metin | Ön Tanımlı Tip. |
| İsim | Attribute: Name | - | Metin | IFC uyumlu yazılım sistemleri veya kullanıcılar tarafından kullanılmak üzere isteğe bağlı bir ad. [[`IfcRoot`]] sınıfının bazı alt tipleri için İsim niteliğinin eklenmesi gerekebilir. Bu, bir "where" kuralı ile zorunlu hale getirilecektir. (EK-5'e göre isimlendirilecektir) |

| Türkçe Tanım (Bilgi Amaçlı) | Özellik | Özellik Seti / İlişki | Veri Tipi (Bilgi Amaçlı) | Açıklama |
|---|---|---|---|---|
| Zemin Kaplaması | `FloorCovering`\*\* | `Pset_SpaceCoveringRequirements` | Metin | Mahal döşemesi için malzemeyi veya bitişi belirtmek amacıyla kullanılan etiket. Etiket, mahal listesi (room book) bilgileri için kullanılır ve genellikle mahal kaşesinde gösterilir. Malzeme bilgisi, kendi şekil gösterimine ve malzeme atamasına sahip bir `IfcCovering` (type=FLOORING) nesnesinin bulunmaması durumunda sağlanır. Tutarsızlık halinde `IfcCovering` elemanlarına atanmış malzeme dikkate alınır. |
| Zemin Kaplama Kalınlığı | `FloorCoveringThickness`\*\* | `Pset_SpaceCoveringRequirements` | Sayı | Mahal döşemesine ait malzeme katman(lar)ının kalınlığı. Kalınlık bilgisi, kendi şekil gösterimine sahip bir `IfcCovering` (type=FLOORING) nesnesinin bulunmaması durumunda sağlanır. Atanmış bir IfcCovering'in geometrik parametreleri ile bu özelliğe bağlı değer arasında tutarsızlık olması durumunda geometrik parametreler dikkate alınır. |
| Duvar Kaplaması | `WallCovering`\*\* | `Pset_SpaceCoveringRequirements` | Metin | Mahal duvar kaplaması için malzeme veya bitişi belirtmek amacıyla kullanılan etiket. Etiket, mahal listesi bilgileri için kullanılır ve genellikle mahal kaşesinde gösterilir. Malzeme bilgisi, kendi şekil gösterimine ve malzeme atamasına sahip bir `IfcCovering` (type=CLADDING) nesnesinin bulunmaması durumunda sağlanır. Tutarsızlık halinde `IfcCovering` elemanlarına atanmış malzeme dikkate alınır. |

*[PDF sayfa 99]*

*(Tablo önceki sayfadan sürüyor; sütun başlıkları okunabilirlik için yinelendi.)*

| Türkçe Tanım (Bilgi Amaçlı) | Özellik | Özellik Seti / İlişki | Veri Tipi (Bilgi Amaçlı) | Açıklama |
|---|---|---|---|---|
| Duvar Kaplama Kalınlığı | `WallCoveringThickness`\*\* | `Pset_SpaceCoveringRequirements` | Sayı | Mahal duvar kaplamasına ait malzeme katman(lar)ının kalınlığı. Kalınlık bilgisi, kendi şekil gösterimine sahip bir `IfcCovering` (type=CLADDING) nesnesinin bulunmaması durumunda sağlanır. Atanmış bir IfcCovering'in geometrik parametreleri ile bu özelliğe bağlı değer arasında tutarsızlık olması durumunda geometrik parametreler dikkate alınır. |
| Tavan Kaplaması | `CeilingCovering`\*\* | `Pset_SpaceCoveringRequirements` | Metin | Mahal tavanı için malzeme veya bitişi belirtmek amacıyla kullanılan etiket. Etiket, mahal listesi bilgileri için kullanılır ve genellikle mahal kaşesinde gösterilir. Malzeme bilgisi, kendi şekil gösterimine ve malzeme atamasına sahip bir `IfcCovering` (type=CEILING) nesnesinin bulunmaması durumunda sağlanır. Tutarsızlık halinde `IfcCovering` elemanlarına atanmış malzeme dikkate alınır. |
| Tavan Kaplama Kalınlığı | `CeilingCoveringThickness` | `Pset_SpaceCoveringRequirements` | Sayı | Mahal tavanına ait malzeme katman(lar)ının kalınlığı. Kalınlık bilgisi, kendi şekil gösterimine sahip bir `IfcCovering` (type=CEILING) nesnesinin bulunmaması durumunda sağlanır. Atanmış bir IfcCovering'in geometrik parametreleri ile bu özelliğe bağlı değer arasında tutarsızlık olması durumunda geometrik parametreler dikkate alınır.. |

\*\* Kaplamalarla ilgili bu özellikler: (1) `IfcSpace` içindeki tanımlardan faydalanılarak yada (2) IfcCovering'de belirtilen ilişkiler kurularak doğrudan modelleme ile tanımlanacak. Bu nedenle belirtilen bu iki yöntemden biri zorunludur.

**"Attribute:": Standart bir IFC elemanının özniteliği**

\*Attribute (`IfcRelAssociatesMaterial>IfcMaterial`): Name\* bilgisi giriliken: "Malzeme İsmi\_Dayanım" formatı kullanılmalıdır. (Örnek:Çelik\_S355 Beton\_C25)

*[PDF sayfa 100]*

## Tablo 6.59 `IfcSpaceHeater` Sınıfı Zorunlu Öznitelik ve Özellik Listesi

**Zorunlu Öznitelik ve Özellik Listesi**

| Mekan Isıtıcısı (`IfcSpaceHeater`) | Ön Tanımlı Tip Listesi |
|---|---|
| Tanım: Mekân ısıtıcıları, sınırlı bir mekânı veya alanı ısıtmak için elektrik, buhar veya sıcak su gibi bir ısıtma kaynağı kullanarak ışınım (radyasyon) ve/veya doğal konveksiyon kombinasyonundan yararlanır. Mekân ısıtıcılarına örnek olarak radyatörler, konvektörler, süpürgelik tipi ısıtıcılar ve kanatlı borulu ısıtıcılar verilebilir. | CONVECTOR, RADIATOR, USERDEFINED, NOTDEFINED |

| Türkçe Tanım (Bilgi Amaçlı) | Öznitelik | İlişki | Veri Tipi (Bilgi Amaçlı) | Açıklama |
|---|---|---|---|---|
| Ön Tanımlı Tip | `PredefinedType` | - | Metin | Ön Tanımlı Tip. |
| İsim | Attribute: Name | - | Metin | IFC uyumlu yazılım sistemleri veya kullanıcılar tarafından kullanılmak üzere isteğe bağlı bir ad. [[`IfcRoot`]] sınıfının bazı alt tipleri için İsim niteliğinin eklenmesi gerekebilir. Bu, bir "where" kuralı ile zorunlu hale getirilecektir. (EK-5'e göre isimlendirilecektir) |
| Etiket | Attribute: Tag | - | Sayı veya Metin | Bir ürünün belirli bir örneğindeki etiket tanımlayıcısı, örneğin seri numarası veya konum numarasıdır. Bu, yerleştirilmiş örnek düzeyindeki tanımlayıcıdır. |
| Sistem | `IfcSystem` | `IfcRelAssignsToGroup>IfcSystem` | Metin | Bir sistem, Mimarlık, Mühendislik ve Yapım ürünü içerisinde, ortak bir amaç, işlev ya da hizmet sunmak üzere bir araya getirilmiş ilişkili parçalardan oluşan düzenli bir kombinasyondur. Sistem esasen, işlevsel olarak ilişkili ürünler grubudur. Bir veya birden fazla `IfcProduct` örneğinin (sistem üyelerinin) gruplandırma ilişkisi, `IfcRelAssignsToGroup` aracılığıyla yönetilir. |

| Türkçe Tanım (Bilgi Amaçlı) | Özellik | Özellik Seti / İlişki | Veri Tipi (Bilgi Amaçlı) | Açıklama |
|---|---|---|---|---|
| Çıkış Kapasitesi | `OutputCapacity` | `Pset_SpaceHeaterTypeCommon` | Sayı | Üretici tarafından listelenen toplam nominal ısı çıkışı. |

*Not: Bu parametreler IFC'de ilişkiler yoluyla kurgulanmaktadır.*

*[PDF sayfa 101]*

## Tablo 6.60 `IfcSpatialZone` Sınıfı Zorunlu Öznitelik ve Özellik Listesi

**Zorunlu Öznitelik ve Özellik Listesi**

| Mekansal Zon (`IfcSpatialZone`) | Ön Tanımlı Tip Listesi |
|---|---|
| Tanım: Mekansal zon, belirli bir işlevsel amaca göre projenin hiyerarşik olmayan ve potansiyel olarak örtüşebilen bir alt bölümlemedir. Mekânsal zonlar örneğin bir ısı bölgesi. inşaat bölgesi. aydınlatma bölgesi veya kullanılabilir alanı temsil etmek için kullanılabilir. Bir mekansal zon, kendi konumlandırmasına ve biçim (geometri) temsiline sahip olabilir. | - |

| Türkçe Tanım (Bilgi Amaçlı) | Öznitelik | İlişki | Veri Tipi (Bilgi Amaçlı) | Açıklama |
|---|---|---|---|---|
| Açıklama | Attribute: Description | - | Metin | Bilgilendirme amaçlı isteğe bağlı olarak verilen bir tanımdır. |
| Uzun İsim | Attribute: `LongName` | - | Metin | Referans amaçlı olarak kullanılan bağlamın uzun adı. |
| İsim | Attribute: Name | - | Metin | IFC uyumlu yazılım sistemleri veya kullanıcılar tarafından kullanılmak üzere isteğe bağlı bir ad. [[`IfcRoot`]] sınıfının bazı alt tipleri için İsim niteliğinin eklenmesi gerekebilir. Bu, bir "where" kuralı ile zorunlu hale getirilecektir. (EK-5'e göre isimlendirilecektir) |

| Türkçe Tanım (Bilgi Amaçlı) | Özellik | Özellik Seti / İlişki | Veri Tipi (Bilgi Amaçlı) | Açıklama |
|---|---|---|---|---|
|  | `EmsalDurumu` | `TREpys_EmsalOzellikSeti` | Metin | İlgili mekansal zon, emsale dahil alanı temsil etmesi halinde 'DAHIL', emsal dışı alanı temsil etmesi halinde 'HARIC', ilgili idarenin özel imar düzenlemelerine bağlı olarak, emsal artışı alanı temsil etmesi halinde 'DIGER' girilir. DIGER olması durumunda açıklama "Attribute: Description" bölümüne girilir. |

*[PDF sayfa 102]*

## Tablo 6.61 `IfcStair` Sınıfı Zorunlu Öznitelik ve Özellik Listesi

**Zorunlu Öznitelik ve Özellik Listesi**

| Merdiven (`IfcStair`) | Ön Tanımlı Tip Listesi |
|---|---|
| Tanım: Merdiven, kullanıcıların bir döşeme seviyesinden diğerine yürüyerek (adım atarak) çıkmasına/ inmesine imkân tanıyan düşey bir geçiş yoludur. Ara kat döşemesi gibi bir sahanlık içerebilir. | STRAIGHT RUN STAIR, TWO STRAIGHT RUN STAIR, QUARTER WINDING STAIR, QUARTER TURN STAIR, HALF WINDING STAIR, HALF TURN STAIR, TWO QUARTER WINDING STAIR, TWO QUARTER TURN STAIR, THREE QUARTER WINDING STAIR, THREE QUARTER TURN STAIR, SPIRAL STAIR, DOUBLE RETURN STAIR, CURVED RUN STAIR, TWO CURVED RUN STAIR, USERDEFINED, NOTDEFINED |

| Türkçe Tanım (Bilgi Amaçlı) | Öznitelik | İlişki | Veri Tipi (Bilgi Amaçlı) | Açıklama |
|---|---|---|---|---|
| Ön Tanımlı Tip | `PredefinedType` | - | Metin | Ön Tanımlı Tip. |
| İsim | Attribute: Name | - | Metin | IFC uyumlu yazılım sistemleri veya kullanıcılar tarafından kullanılmak üzere isteğe bağlı bir ad. [[`IfcRoot`]] sınıfının bazı alt tipleri için İsim niteliğinin eklenmesi gerekebilir. Bu, bir "where" kuralı ile zorunlu hale getirilecektir. (EK-5'e göre isimlendirilecektir) |
| Etiket | Attribute: Tag | - | Sayı veya Metin | Bir ürünün belirli bir örneğindeki etiket tanımlayıcısı, örneğin seri numarası veya konum numarasıdır. Bu, yerleştirilmiş örnek düzeyindeki tanımlayıcıdır. |
| Malzeme | Attribute(IfcRelAssociatesMaterial>IfcMaterial): Name\* | - | Metin | [`IfcMaterial`], yapı elemanlarını (fiziksel ürünler veya bileşenleri) oluşturmak için kullanılabilecek homojen veya heterojen maddelerdir. |

| Türkçe Tanım (Bilgi Amaçlı) | Özellik | Özellik Seti / İlişki | Veri Tipi (Bilgi Amaçlı) | Açıklama |
|---|---|---|---|---|
| Yangın Dayanım Sınıfı | `FireRating` | `Pset_StairCommon` | Metin | Bu nesnenin yangın dayanım sınıfı. Ulusal yangın güvenliği sınıflandırmasına göre (Türkiye için: Binaların Yangından Korunması Hakkında Yönetmeliğe göre) verilmiştir. |
| Acil Çıkış | `FireExit` | `Pset_StairCommon` | Mantıksal | Bu nesnenin yangın durumunda çıkış olarak kullanılmak üzere tasarlandığını (DOĞRU) veya tasarlanmadığını (YANLIŞ) belirtir. Burada, ulusal bina yönetmeliğine uygun olarak bir acil çıkış merdiveni tanımlanmaktadır. |

**"Attribute:": Standart bir IFC elemanının özniteliği**

\*Attribute (`IfcRelAssociatesMaterial>IfcMaterial`): Name\* bilgisi giriliken: "Malzeme İsmi\_Dayanım" formatı kullanılmalıdır. (Örnek:Çelik\_S355 Beton\_C25)

*[PDF sayfa 103]*

## Tablo 6.62 `IfcTank` Sınıfı Zorunlu Öznitelik ve Özellik Listesi

**Zorunlu Öznitelik ve Özellik Listesi**

| Tank (`IfcTank`) | Ön Tanımlı Tip Listesi |
|---|---|
| Tanım: Tank, sıvı veya gazın daha sonra kullanılmak üzere depolandığı bir kap ya da haznedir. | BASIN, BREAK PRESSURE, EXPANSION, FEED AND EXPANSION, OILRETENTION TRAY, PRESSURE VESSEL, STORAGE, VESSEL, USERDEFINED, NOTDEFINED |

| Türkçe Tanım (Bilgi Amaçlı) | Öznitelik | İlişki | Veri Tipi (Bilgi Amaçlı) | Açıklama |
|---|---|---|---|---|
| Ön Tanımlı Tip | `PredefinedType` | - | Metin | Ön Tanımlı Tip. |
| İsim | Attribute: Name | - | Metin | IFC uyumlu yazılım sistemleri veya kullanıcılar tarafından kullanılmak üzere isteğe bağlı bir ad. [[`IfcRoot`]] sınıfının bazı alt tipleri için İsim niteliğinin eklenmesi gerekebilir. Bu, bir "where" kuralı ile zorunlu hale getirilecektir. (EK-5'e göre isimlendirilecektir) |
| Etiket | Attribute: Tag | - | Sayı veya Metin | Bir ürünün belirli bir örneğindeki etiket tanımlayıcısı, örneğin seri numarası veya konum numarasıdır. Bu, yerleştirilmiş örnek düzeyindeki tanımlayıcıdır. |

| Türkçe Tanım (Bilgi Amaçlı) | Özellik | Özellik Seti / İlişki | Veri Tipi (Bilgi Amaçlı) | Açıklama |
|---|---|---|---|---|
| Tank Nominal Kapasitesi | `TankNominalCapacity` | `Pset_TankTypeCommon` | Sayı | Tankın toplam nominal veya tasarım hacmi. |

*[PDF sayfa 104]*

## Tablo 6.63 `IfcTransformer` Sınıfı Zorunlu Öznitelik ve Özellik Listesi

**Zorunlu Öznitelik ve Özellik Listesi**

| Transformatör (`IfcTransformer`) | Ön Tanımlı Tip Listesi |
|---|---|
| Tanım: Transformatör, elektrik enerjisini bir devreden başka bir devreye aktaran, endüktif ve sabit bir cihazdır. | CURRENT, FREQUENCY, INVERTER, RECTIFIER, VOLTAGE, USERDEFINED, NOTDEFINED |

| Türkçe Tanım (Bilgi Amaçlı) | Öznitelik | İlişki | Veri Tipi (Bilgi Amaçlı) | Açıklama |
|---|---|---|---|---|
| Ön Tanımlı Tip | `PredefinedType` | - | Metin | Ön Tanımlı Tip. |
| İsim | Attribute: Name | - | Metin | IFC uyumlu yazılım sistemleri veya kullanıcılar tarafından kullanılmak üzere isteğe bağlı bir ad. [[`IfcRoot`]] sınıfının bazı alt tipleri için isim niteliğinin eklenmesi gerekebilir. Bu, bir "where" kuralı ile zorunlu hale getirilecektir. (EK-5'e göre isimlendirilecektir) |
| Etiket | Attribute: Tag | - | Sayı veya Metin | Bir ürünün belirli bir örneğindeki etiket tanımlayıcısı, örneğin seri numarası veya konum numarasıdır. Bu, yerleştirilmiş örnek düzeyindeki tanımlayıcıdır. |

| Türkçe Tanım (Bilgi Amaçlı) | Özellik | Özellik Seti / İlişki | Veri Tipi (Bilgi Amaçlı) | Açıklama |
|---|---|---|---|---|
| Maksimum Görünen Güç | `MaximumApparentPower` | `Pset_TransformerTypeCommon` | Sayı | Transformatörün maksimum görünen gücü (veya kapasitesi), VA (volt- amper) cinsinden ifade edilir. |

*[PDF sayfa 105]*

## Tablo 6.64 `IfcTransportElement` Sınıfı Zorunlu Öznitelik ve Özellik Listesi

**Zorunlu Öznitelik ve Özellik Listesi**

| Taşıma Elemanı (`IfcTransportElement`) | Ön Tanımlı Tip Listesi |
|---|---|
| Tanım: Taşıma elemanı, bir bina veya bina kompleksi içinde insanları, hayvanları veya eşyaları taşıyan tüm ulaşım ile ilgili nesnelerin genelleştirilmiş halidir. Taşıma elemanı, (varsa) `IfcTransportElementType` tarafından tanımlanan bir taşıma elemanının yerleştirilmiş örneğini ifade eder. | ELEVATOR, ESCALATOR, MOVING WALKWAY, CRANEWAY, LIFTING GEAR, USERDEFINED, NOTDEFINED |

| Türkçe Tanım (Bilgi Amaçlı) | Öznitelik | İlişki | Veri Tipi (Bilgi Amaçlı) | Açıklama |
|---|---|---|---|---|
| Ön Tanımlı Tip | `PredefinedType` | - | Metin | Ön Tanımlı Tip. |
| İsim | Attribute: Name | - | Metin | IFC uyumlu yazılım sistemleri veya kullanıcılar tarafından kullanılmak üzere isteğe bağlı bir ad. [[`IfcRoot`]] sınıfının bazı alt tipleri için İsim niteliğinin eklenmesi gerekebilir. Bu, bir "where" kuralı ile zorunlu hale getirilecektir. (EK-5'e göre isimlendirilecektir) |
| Etiket | Attribute: Tag | - | Sayı veya Metin | Bir ürünün belirli bir örneğindeki etiket tanımlayıcısı, örneğin seri numarası veya konum numarasıdır. Bu, yerleştirilmiş örnek düzeyindeki tanımlayıcıdır. |

| Türkçe Tanım (Bilgi Amaçlı) | Özellik | Özellik Seti / İlişki | Veri Tipi (Bilgi Amaçlı) | Açıklama |
|---|---|---|---|---|
| İnsan Kapasitesi | `CapacityPeople` | `Pset_TransportElementCommon` | Sayı | Taşıma elemanının kapasitesi, kişi sayısı cinsinden ölçülür. |
| Ağırlık Kapasitesi | `CapacityWeight` | `Pset_TransportElementCommon` | Sayı | Taşıma elemanının kapasitesi, ağırlık cinsinden ölçülür. |

i

*[PDF sayfa 106]*

## Tablo 6.65 `IfcValve` Sınıfı Zorunlu Öznitelik ve Özellik Listesi

**Zorunlu Öznitelik ve Özellik Listesi**

| Vana (`IfcValve`) | Ön Tanımlı Tip Listesi |
|---|---|
| Tanım: Vana, bina tesisatı boru dağıtım sisteminde akışkanın akışını kontrol etmek veya düzenlemek için kullanılır. | AIR RELEASE, ANTI VACUUM, CHANGE OVER, CHECK, COMMISSIONING, DIVERTING, DRAW OFF COCK, DOUBLE CHECK, DOUBLE REGULATING, FAUCET, FLUSHING, GAS COCK, GAS TAP, ISOLATING, MIXING, PRESSURE REDUCING, PRESSURE RELIEF, REGULATING, SAFETY CUT OFF, STEAM TRAP, STOP COCK, USERDEFINED, NOTDEFINED |

| Türkçe Tanım (Bilgi Amaçlı) | Öznitelik | İlişki | Veri Tipi (Bilgi Amaçlı) | Açıklama |
|---|---|---|---|---|
| Ön Tanımlı Tip | `PredefinedType` | - | Metin | Ön Tanımlı Tip. |
| İsim | Attribute: Name | - | Metin | IFC uyumlu yazılım sistemleri veya kullanıcılar tarafından kullanılmak üzere isteğe bağlı bir ad. [[`IfcRoot`]] sınıfının bazı alt tipleri için İsim niteliğinin eklenmesi gerekebilir. Bu, bir "where" kuralı ile zorunlu hale getirilecektir. (EK-5'e göre isimlendirilecektir) |
| Etiket | Attribute: Tag | - | Sayı veya Metin | Bir ürünün belirli bir örneğindeki etiket tanımlayıcısı, örneğin seri numarası veya konum numarasıdır. Bu, yerleştirilmiş örnek düzeyindeki tanımlayıcıdır. |
| Sistem | `IfcSystem` | `IfcRelAssignsToGroup>IfcSystem` | Metin | Bir sistem, Mimarlık, Mühendislik ve Yapım ürünü içerisinde, ortak bir amaç, işlev ya da hizmet sunmak üzere bir araya getirilmiş ilişkili parçalardan oluşan düzenli bir kombinasyondur. Sistem esasen, işlevsel olarak ilişkili ürünler grubudur. Bir veya birden fazla `IfcProduct` örneğinin (sistem üyelerinin) gruplandırma ilişkisi, `IfcRelAssignsToGroup` aracılığıyla yönetilir. |

| Türkçe Tanım (Bilgi Amaçlı) | Özellik | Özellik Seti / İlişki | Veri Tipi (Bilgi Amaçlı) | Açıklama |
|---|---|---|---|---|
| Boyut | Size | `Pset_ValveTypeCommon` | Sayı | Vananın bağlantı boyutu (veya musluklar, karışım vanaları vb. için her bir bağlantının boyutu). |

*Not: Bu parametreler IFC'de ilişkiler yoluyla kurgulanmaktadır.*

*[PDF sayfa 107]*

## Tablo 6.66 `IfcWall` Sınıfı Zorunlu Öznitelik ve Özellik Listesi

**Zorunlu Öznitelik ve Özellik Listesi**

| Duvar (`IfcWall`) | Ön Tanımlı Tip Listesi |
|---|---|
| Tanım: Duvar, mekanları sınırlayabilen veya bölebilen düşey bir yapısal bileşendir. Duvarlar genellikle düşey veya düşeye yakın düzlemsel elemanlardır ve yapısal yükleri taşımak üzere tasarlanabilir. Ancak, taşıyıcı olmak zorunda değildir. | ELEMENTED WALL, MOVABLE, PARAPET, PARTITIONING, PLUMBING WALL, POLYGONAL, RETAINING WALL, SHEAR, SOLIDWALL, STANDARD, WAVEWALL, USERDEFINED, NOTDEFINED |

| Türkçe Tanım (Bilgi Amaçlı) | Öznitelik | İlişki | Veri Tipi (Bilgi Amaçlı) | Açıklama |
|---|---|---|---|---|
| Ön Tanımlı Tip | `PredefinedType` | - | Metin | Ön Tanımlı Tip. |
| İsim | Attribute: Name | - | Metin | IFC uyumlu yazılım sistemleri veya kullanıcılar tarafından kullanılmak üzere isteğe bağlı bir ad. [[`IfcRoot`]] sınıfının bazı alt tipleri için İsim niteliğinin eklenmesi gerekebilir. Bu, bir "where" kuralı ile zorunlu hale getirilecektir. (EK-5'e göre isimlendirilecektir) |
| Etiket | Attribute: Tag | - | Sayı veya Metin | Bir ürünün belirli bir örneğindeki etiket tanımlayıcısı, örneğin seri numarası veya konum numarasıdır. Bu, yerleştirilmiş örnek düzeyindeki tanımlayıcıdır. |
| Malzeme | Attribute(IfcRelAssociatesMaterial>IfcMaterial): Name\* | - | Metin | [`IfcMaterial`], yapı elemanlarını (fiziksel ürünler veya bileşenleri) oluşturmak için kullanılabilecek homojen veya heterojen maddelerdir. |

| Türkçe Tanım (Bilgi Amaçlı) | Özellik | Özellik Seti / İlişki | Veri Tipi (Bilgi Amaçlı) | Açıklama |
|---|---|---|---|---|
| Dışarıda | `IsExternal` | `Pset_WallCommon` | Mantıksal | Yapı elemanının dışarıda kullanıma yönelik tasarlandığını (DOĞRU) veya tasarlanmadığını (YANLIŞ) gösterir. Eğer (DOĞRU) ise, bu bir dış eleman olup binanın dışına bakmaktadır. |
| Yangın Dayanım Sınıfı | `FireRating` | `Pset_WallCommon` | Metin | Bu nesnenin yangın dayanım sınıfı. Ulusal yangın güvenliği sınıflandırmasına göre (Türkiye için: Binaların Yangından Korunması Hakkında Yönetmeliğe göre) verilmiştir. |

**"Attribute:": Standart bir IFC elemanının attribute (öznitelik) listesindeki özellikler**

\*Attribute (`IfcRelAssociatesMaterial>IfcMaterial`): Name\* bilgisi giriliken: "Malzeme İsmi\_Dayanım" formatı kullanılmalıdır. (Örnek:Çelik\_S355 Beton\_C25)

*[PDF sayfa 108]*

## Tablo 6.67 `IfcWindow` Sınıfı Zorunlu Öznitelik ve Özellik Listesi

**Zorunlu Öznitelik ve Özellik Listesi**

| Pencere (`IfcWindow`) | Ön Tanımlı Tip Listesi |
|---|---|
| Tanım: Pencere, esas olarak doğal ışık ve temiz hava sağlamak için kullanılan bir yapı elemanıdır. Düşey açıklıkları içerdiği gibi çatı pencereleri veya ışıklıklar gibi yatay açıklıkları da kapsar. Açılır, döner, kayar, döner kanatlı veya sabit panellerden oluşan yapıları içerebilir. Bir pencere, bir kasadan ve bir veya birkaç panelden oluşur. | WINDOW, SKYLIGHT, LIGHTDOME, USERDEFINED, NOTDEFINED |

| Türkçe Tanım (Bilgi Amaçlı) | Öznitelik | İlişki | Veri Tipi (Bilgi Amaçlı) | Açıklama |
|---|---|---|---|---|
| Ön Tanımlı Tip | `PredefinedType` | - | Metin | Ön Tanımlı Tip. |
| İsim | Attribute: Name | - | Metin | IFC uyumlu yazılım sistemleri veya kullanıcılar tarafından kullanılmak üzere isteğe bağlı bir ad. [[`IfcRoot`]] sınıfının bazı alt tipleri için İsim niteliğinin eklenmesi gerekebilir. Bu, bir "where" kuralı ile zorunlu hale getirilecektir. (EK-5'e göre isimlendirilecektir) |
| Etiket | Attribute: Tag | - | Sayı veya Metin | Bir ürünün belirli bir örneğindeki etiket tanımlayıcısı, örneğin seri numarası veya konum numarasıdır. Bu, yerleştirilmiş örnek düzeyindeki tanımlayıcıdır. |

| Türkçe Tanım (Bilgi Amaçlı) | Özellik | İlişki | Veri Tipi (Bilgi Amaçlı) | Açıklama |
|---|---|---|---|---|
| Dışarıda | `IsExternal` | `Pset_WindowCommon` | Mantıksal | Yapı elemanının dışarıda kullanıma yönelik tasarlandığını (DOĞRU) veya tasarlanmadığını (YANLIŞ) gösterir. Eğer (DOĞRU) ise, bu bir dış eleman olup binanın dışına bakmaktadır. |

*[PDF sayfa 109]*

## Tablo 6.68 `IfcZone` Sınıfı Zorunlu Öznitelik ve Özellik Listesi

**Zorunlu Öznitelik ve Özellik Listesi**

| Zon (`IfcZone`) | Ön Tanımlı Tip Listesi |
|---|---|
| Tanım: Zon, mekanların, kısmi mekanların veya diğer zonların gruplanmış halini ifade eder. Bu mekanlar bitişik veya bitişik olmayabilir. Bir zonun kendi geometrik biçim temsili yoktur. Zon yapıları hiyerarşik olmayabilir (bir projenin mekânsal yapısının aksine - bkz. `SpatialStructureElement`). Yani, tek bir mekân (Space), sıfır, bir veya birden fazla `IfcZone` ile ilişkilendirilebilir. Mekânlar, üst tür olan Group'ta tanımlanan `IfcRelAssignsToGroup` ilişkisi kullanılarak bir `IfcZone` içinde gruplanır. Örneğin, zon bir daireyi temsil mekanları gruplandırarak temsil etmek için kullanılabilir. | - |

| Türkçe Tanım (Bilgi Amaçlı) | Öznitelik | İlişki | Veri Tipi (Bilgi Amaçlı) | Açıklama |
|---|---|---|---|---|
| Uzun İsim | Attribute: `LongName` | - | Metin | Referans amaçlı olarak kullanılan bağlamın uzun adı. |
| İsim | Attribute: Name | - | Metin | IFC uyumlu yazılım sistemleri veya kullanıcılar tarafından kullanılmak üzere isteğe bağlı bir ad. [[`IfcRoot`]] sınıfının bazı alt tipleri için İsim niteliğinin eklenmesi gerekebilir. Bu, bir "where" kuralı ile zorunlu hale getirilecektir. (EK-5'e göre isimlendirilecektir) |
| Nesne Tipi | Attribute: `ObjectType` | - | Metin | Tip, nesneyi daha ayrıntılı tanımlayan özel bir tipi belirtir. Kullanım, örneklenebilir somut alt tiplerin seviyesinde belirlenmelidir. Şu durumlarda kullanıcı tarafından tanımlanan tip bilgisi bu özniteliğe girilir: 1) Ön Tanımlı Tip özniteliğinin değeri listeden [[USERDEFINED]] olarak seçilmişse veya 2) somut örneklenmiş varlıkta bir Ön Tanımlı Tip özniteliği yoksa . İkinci durum, bazı istisnai yaprak sınıflarında ve [[IfcBuiltElement]]'i doğrudan örneklendirirken geçerlidir. |
