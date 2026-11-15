<!-- Kaynak: Resmî Gazete 5.8.2026/33331, Dijital Hazırlama Yönetmeliği ekleri (EK-1…EK-9), PDF s. 35–72. Taranmış sayfalardan OCR ile okundu; yöntem, doğrulama kapsamı ve sınırlar için README.md. -->

*[PDF sayfa 35]*

# EK-6 IFC SINIFLARININ ÖZNİTELİK İLE ÖZELLİK TANIMLARI

**6.1-** Tasarımda yer verilen varlıkların temsilinde kullanılan zorunlu IFC sınıfları EK-5 Tablo 5.1'de ve kullanımı isteğe bağlı IFC sınıfları Tablo 5.2'de verilmiştir.

**6.1.1-** Tablo 6.1 ile Tablo 6.135 arasında yer alan IFC sınıfları, IFC 4.3 versiyonu esas alınarak düzenlenmiştir.

**6.1.2-** BIM modelinde alt sınıfı bulunan bir üst sınıf kullanılmaz. Bu durumda yalnızca alt sınıf modele dahil edilir.

**6.1.3-** BIM modelinde, IFC şemasında özel bir tanımı olmayan veya henüz sınıfı belirlenmemiş nesneleri temsil eden genel bir varlığa (nesne vekili- `IfcProxy` veya bina nesne vekili - `IfcBuildingElementProxy`) yer verilemez.

**6.2-** Uniclass 2015 sınıflandırma kodları (bilgi için verilmiştir): Her varlığa ait belirlenmiş olan öznitelikler ve özellikler ile birlikte Uniclass 2015 sınıflandırma kodları kullanılabilir.

**6.3-** Zorunlu IFC sınıflarında yer alması gereken zorunlu öznitelikler ve özellikler Tablo 6.1 ila Tablo 6.68 tablolarında verilmiştir.

**6.4** Zorunlu IFC sınıflarında yer alması isteğe bağlı olan öznitelikler ve özellikler Tablo 6.69 ila Tablo 6.135 tablolarında verilmiştir ve ilgili idare tarafından talep etmesi halinde ilgili IFC sınıfında yer alır.

**6.5-** Tablo 6.1 ile Tablo 6.135 arasında yer alan `TREpys_` ile başlayan tüm özellik setleri ve altında yer alan özellikler modelde kullanıcı tarafından oluşturulacaktır. Diğer özellikler IFC veri şemasında yer alan özelliklerdir.

**6.6-** EK-5 Tablo 5.2'de yer alan isteğe bağlı IFC sınıflarının öznitelikleri ve özellikleri Yönetmeliğin 5 inci maddesinin birinci fıkrası kapsamında belirlenebilir.

*[PDF sayfa 36]*

## Tablo 6.1 `IfcAirTerminal` Sınıfı Zorunlu Öznitelik ve Özellik Listesi

**Zorunlu Öznitelik ve Özellik Listesi**

| Havalandırma Menfezi (`IfcAirTerminal`) | Ön Tanımlı Tip Listesi |
|---|---|
| Tanım: Bir havalandırma menfezi, dağıtım sistem(ler)i ile bir veya birden fazla mahal arasındaki hava transferinin başlangıç veya bitiş noktasıdır. Ayrıca, bitişik mekanlar arasında hava transferi için de kullanılabilir. | DIFFUSER, GRILLE, LOUVRE, REGISTER, USERDEFINED, NOTDEFINED |

| Türkçe Tanım (Bilgi Amaçlı) | Öznitelik | İlişki | Veri Tipi (Bilgi Amaçlı) | Açıklama |
|---|---|---|---|---|
| Ön Tanımlı Tip | `PredefinedType` | - | Metin | Ön Tanımlı Tip. |
| İsim | Attribute: Name | - | Metin | IFC uyumlu yazılım sistemleri veya kullanıcılar tarafından kullanılmak üzere isteğe bağlı bir ad. [[`IfcRoot`]] sınıfının bazı alt tipleri için İsim niteliğinin eklenmesi gerekebilir. Bu, bir "where" kuralı ile zorunlu hale getirilecektir. (EK-5'e göre isimlendirilecektir) |
| Etiket | Attribute: Tag | - | Sayı veya Metin | Bir ürünün belirli bir örneğindeki etiket tanımlayıcısı, örneğin seri numarası veya konum numarasıdır. Bu, yerleştirilmiş örnek düzeyindeki tanımlayıcıdır. |
| Sistem | `IfcSystem` | `IfcRelAssignsToGroup>IfcSystem` | Metin | Bir sistem, Mimarlık, Mühendislik ve Yapım ürünü içerisinde, ortak bir amaç, işlev ya da hizmet sunmak üzere bir araya getirilmiş ilişkili parçalardan oluşan düzenli bir kombinasyondur. Sistem esasen, işlevsel olarak ilişkili ürünler grubudur. Bir veya birden fazla `IfcProduct` örneğinin (sistem üyelerinin) gruplandırma ilişkisi, `IfcRelAssignsToGroup` aracılığıyla yönetilir. |

| Türkçe Tanım (Bilgi Amaçlı) | Özellik | Özellik Seti / İlişki | Veri Tipi (Bilgi Amaçlı) | Açıklama |
|---|---|---|---|---|
| Hava Akış Debisi Aralığı | `AirFlowRateRange` | `Pset_AirTerminalTypeCommon` | Sayı | Sağlanabilecek hava debisinin olası aralığı.<br>Hava terminalinin çalışmak üzere tasarlandığı hava debisi aralığı. |

*Not: Bu parametreler IFC'de ilişkiler yoluyla kurgulanmaktadır.*

*[PDF sayfa 37]*

## Tablo 6.2 `IfcAudioVisualAppliance` Sınıfı Zorunlu Öznitelik ve Özellik Listesi

**Zorunlu Öznitelik ve Özellik Listesi**

| Görüntü ve Ses Cihaz (`IfcAudioVisualAppliance`) | Ön Tanımlı Tip Listesi |
|---|---|
| Tanım: Bir görüntü ve ses cihazı veya görüntüyü gösteren, kaydeden, ileten veya alan bir aygıttır. | AMPLIFIER, CAMERA, DISPLAY, MICROPHONE, PLAYER, PROJECTOR, RECEIVER, SPEAKER, SWITCHER, TELEPHONE, TUNER, USERDEFINED, NOTDEFINED |

| Türkçe Tanım (Bilgi Amaçlı) | Öznitelik | İlişki | Veri Tipi (Bilgi Amaçlı) | Açıklama |
|---|---|---|---|---|
| Ön Tanımlı Tip | `PredefinedType` | - | Metin | Ön Tanımlı Tip. |
| İsim | Attribute: Name | - | Metin | IFC uyumlu yazılım sistemleri veya kullanıcılar tarafından kullanılmak üzere isteğe bağlı bir ad. [[`IfcRoot`]] sınıfının bazı alt tipleri için İsim niteliğinin eklenmesi gerekebilir. Bu, bir "where" kuralı ile zorunlu hale getirilecektir. (EK-5'e göre isimlendirilecektir) |
| Etiket | Attribute: Tag | - | Sayı veya Metin | Bir ürünün belirli bir örneğindeki etiket tanımlayıcısı, örneğin seri numarası veya konum numarasıdır. Bu, yerleştirilmiş örnek düzeyindeki tanımlayıcıdır. |

*[PDF sayfa 38]*

## Tablo 6.3 `IfcBeam` Sınıfı Zorunlu Öznitelik ve Özellik Listesi

**Zorunlu Öznitelik ve Özellik Listesi**

| Kiriş (`IfcBeam`) | Ön Tanımlı Tip Listesi |
|---|---|
| Tanım: Bir Kiriş, genellikle yatay veya neredeyse yatay bir yapısal elemandır ve yükleri esas olarak eğilme taşıma gücü ile karşılar. Mimari açıdan böyle bir elemanı da temsil edebilir. Yük taşıyıcı olması gerekmez. | BEAM, JOIST, HOLLOWCORE, LINTEL, SPANDREL, T\_BEAM, GIRDER\_SEGMENT, DIAPHRAGM, PIERCAP, HATSTONE, CORNICE, EDGEBEAM, USERDEFINED, NOTDEFINED |

| Türkçe Tanım (Bilgi Amaçlı) | Öznitelik | İlişki | Veri Tipi (Bilgi Amaçlı) | Açıklama |
|---|---|---|---|---|
| Ön Tanımlı Tip | `PredefinedType` | - | Metin | Ön Tanımlı Tip. |
| İsim | Attribute: Name | - | Metin | IFC uyumlu yazılım sistemleri veya kullanıcılar tarafından kullanılmak üzere isteğe bağlı bir ad. [[`IfcRoot`]] sınıfının bazı alt tipleri için İsim niteliğinin eklenmesi gerekebilir. Bu, bir "where" kuralı ile zorunlu hale getirilecektir. (EK-5'e göre isimlendirilecektir) |
| Etiket | Attribute: Tag | - | Sayı veya Metin | Bir ürünün belirli bir örneğindeki etiket tanımlayıcısı, örneğin seri numarası veya konum numarasıdır. Bu, yerleştirilmiş örnek düzeyindeki tanımlayıcıdır. |
| Malzeme | Attribute(IfcRelAssociatesMaterial>IfcMaterial): Name\* | - | Metin | [`IfcMaterial`], yap: elemanlarını (fiziksel ürünler veya bileşenleri) oluşturmak için kullanılabilecek homojen veya heterojen maddelerdir. |

| Türkçe Tanım (Bilgi Amaçlı) | Özellik | Özellik Seti / İlişki | Veri Tipi (Bilgi Amaçlı) | Açıklama |
|---|---|---|---|---|
| Yangın Dayanım Sınıfı | `FireRating` | `Pset_BeamCommon` | Metin | Bu nesnenin yangın dayanım sınıfı. Ulusal yangın güvenliği sınıflandırmasına göre (Türkiye için: Binaların Yangından Korunması Hakkında Yönetmeliğe göre) verilmiştir. |

\*Attribute (`IfcRelAssociatesMaterial>IfcMaterial`): Name\* bilgisi giriliken: "Malzeme İsmi\_Dayanım" formatı kullanılmalıdır. (Örnek:Çelik\_S355 Beton\_C25)

,

*[PDF sayfa 39]*

## Tablo 6.4 `IfcBoiler` Sınıfı Zorunlu Öznitelik ve Özellik Listesi

**Zorunlu Öznitelik ve Özellik Listesi**

| Kazan (`IfcBoiler`) | Ön Tanımlı Tip Listesi |
|---|---|
| Tanım: Kazan, doğal gaz, fuel-oil veya elektrik gibi bir enerji kaynağı kullanılarak suyun veya başka bir akışkanın ısıtıldığı, basınca dayanıklı kapalı bir basınçlı kaptır. Kazandaki akışkan daha sonra çeşitli süreçlerde veya ısıtma uygulamalarında kullanılmak üzere kazanın dışına sirküle edilir. | WATER, STEAM, USERDEFINED, NOTDEFINED |

| Türkçe Tanım (Bilgi Amaçlı) | Öznitelik | İlişki | Veri Tipi (Bilgi Amaçlı) | Açıklama |
|---|---|---|---|---|
| Ön Tanımlı Tip | `PredefinedType` | - | Metin | Ön Tanımlı Tip. |
| İsim | Attribute: Name | - | Metin | IFC uyumlu yazılım sistemleri veya kullanıcılar tarafından kullanılmak üzere isteğe bağlı bir ad. [[`IfcRoot`]] sınıfının bazı alt tipleri için İsim niteliğinin eklenmesi gerekebilir. Bu, bir "where" kuralı ile zorunlu hale getirilecektir. (EK-5'e göre isimlendirilecektir) |
| Etiket | Attribute: Tag | - | Sayı veya Metin  | Bir ürünün belirli bir örneğindeki etiket tanımlayıcısı, örneğin seri numarası veya konum numarasıdır. Bu, yerleştirilmiş örnek düzeyindeki tanımlayıcıdır. |
| Sistem | `IfcSystem` | `IfcRelAssignsToGroup>IfcSystem` | Metin | Bir sistem, Mimarlık, Mühendislik ve Yapım ürünü içerisinde, ortak bir amaç, işlev ya da hizmet sunmak üzere bir araya getirilmiş ilişkili parçalardan oluşan düzenli bir kombinasyondur. Sistem esasen, işlevsel olarak ilişkili ürünler grubudur. Bir veya birden fazla `IfcProduct` örneğinin (sistem üyelerinin) gruplandırma ilişkisi, `IfcRelAssignsToGroup` aracılığıyla yönetilir. |

| Türkçe Tanım (Bilgi Amaçlı) | Özellik | Özellik Seti / İlişki | Veri Tipi (Bilgi Amaçlı) | Açıklama |
|---|---|---|---|---|
| Nominal Enerji Tüketimi | `NominalEnergyConsumption` | `Pset_BoilerTypeCommon` | Sayı | Toplam kazan ısı gücünü sağlamak için gerekli nominal yakıt tüketiml debisi. |

*Not: Bu parametreler IFC'de ilişkiler yoluyla kurgulanmaktadır.*

*[PDF sayfa 40]*

## Tablo 6.5 `IfcBridge` Zorunlu Öznitelik ve Özellik Listesi

**Zorunlu Öznitelik ve Özellik Listesi**

| Köprü (`IfcBridge`)) | Ön Tanımlı Tip Listesi |
|---|---|
| Tanım: Köprü, yayaların, hayvanların, araçların ve altyapı hizmetlerinin engellerin üzerinden veya yerden yüksekte bulunan iki nokta arasında geçişini sağlayan bir inşaat mühendisliği yapısıdır. | ARCHED, CABLE STAYED, CANTILEVER, CULVERT, FRAMEWORK, GIRDER, SUSPENSION, TRUSS, USERDEFINED, NOTDEFINED |

| Türkçe Tanım (Bilgi Amaçlı) | Öznitelik | İlişki | Veri Tipi (Bilgi Amaçlı) | Açıklama |
|---|---|---|---|---|
| Ön Tanımlı Tip | `PredefinedType` | - | Metin | Ön Tanımlı Tip. |
| İsim | Attribute: Name | - | Metin | IFC uyumlu yazılım sistemleri veya kullanıcılar tarafından kullanılmak üzere isteğe bağlı bir ad. [[`IfcRoot`]] sınıfının bazı alt tipleri için İsim niteliğinin eklenmesi gerekebilir. Bu, bir "where" kuralı ile zorunlu hale getirilecektir. (EK-5'e göre isimlendirilecektir) |
| Malzeme | Attribute (`IfcRelAssociatesMaterial>IfcMaterial`): Name\* | - | Metin | [`IfcMaterial`], yapı elemanlarını (fiziksel ürünler veya bileşenleri) oluşturmak için kullanılabilecek homojen veya heterojen maddelerdir. |

\*Attribute (`IfcRelAssociatesMaterial>IfcMaterial`): Name\* bilgisi giriliken: "Malzeme İsmi\_Dayanım" formatı kullanılmalıdır. (Örnek:Çelik\_S355 Beton\_C25)

*[PDF sayfa 41]*

## Tablo 6.6 `IfcBridgePart` Sınıfı Zorunlu Öznitelik ve Özellik Listesi

**Zorunlu Öznitelik ve Özellik Listesi**

| Köprü Parçası (`IfcBridgePart`) | Ön Tanımlı Tip Listesi |
|---|---|
| 'Tanım: Bir köprünün parçası. | ABUTMENT, DECK, DECK SEGMENT, FOUNDATION, PIER, PIER SEGMENT, PYLON, SUB STRUCTURE, SUPER STRUCTURE, SURFACE STRUCTURE, USERDEFINED, NOTDEFINED |

| Türkçe Tanım (Bilgi Amaçlı) | Öznitelik | İlişki | Veri Tipi (Bilgi Amaçlı) | Açıklama |
|---|---|---|---|---|
| Ön Tanımlı Tip | `PredefinedType` | - | Metin | Ön Tanımlı Tip. |
| İsim | Attribute: Name | - | Metin | IFC uyumlu yazılım sistemleri veya kullanıcılar tarafından kullanılmak üzere isteğe bağlı bir ad. [[`IfcRoot`]] sınıfının bazı alt tipleri için İsim niteliğinin eklenmesi gerekebilir. Bu, bir "where" kuralı ile zorunlu hale getirilecektir. (EK-5'e göre isimlendirilecektir) |
| Malzeme | Attribute(IfcRelAssociatesMaterial>IfcMaterial): Name\* | - | Metin | [`IfcMaterial`], yapı elemanlarını (fiziksel ürünler veya bileşenleri) oluşturmak için kullanılabilecek homojen veya heterojen maddelerdir. |

\*Attribute (`IfcRelAssociatesMaterial>IfcMaterial`): Name\* bilgisi giriliken: "Malzeme İsmi\_Dayanım" formatı kullanılmalıdır. (Örnek:Çelik\_S355 Beton\_C25)

*[PDF sayfa 42]*

## Tablo 6.7 `IfcBuilding` Sınıfı Zorunlu Öznitelik ve Özellik Listesi

**Zorunlu Öznitelik ve Özellik Listesi**

| Bina (`IfcBuilding`) | Ön Tanımlı Tip Listesi |
|---|---|
| Tanım: Bir bina. içindekilere veya içeriğine barınak sağlayan ve sabit bir konumda duran bir yapıyı temsil eder. Bina aynı zamanda, bir bina projesinin bileşenleri için (arsa, kat ve mahal ile birlikte) mekânsal yapı hiyerarşisinde temel bir eleman olarak kullanılır. | - |

| Türkçe Tanım (Bilgi Amaçlı) | Öznitelik | İlişki | Veri Tipi (Bilgi Amaçlı) | Açıklama |
|---|---|---|---|---|
| Uzun İsim | Attribute:LongName | - | Metin | Referans amaçlı olarak kullanılan bağlamın uzun adı. |
| İsim | Attribute:Name | - | Metin | IFC uyumlu yazılım sistemleri veya kullanıcılar tarafından kullanılmak üzere isteğe bağlı bir ad. [[`IfcRoot`]] sınıfının bazı alt tipleri için İsim niteliğinin eklenmesi gerekebilir. Bu, bir "where" kuralı ile zorunlu hale getirilecektir. (EK-5'e göre isimlendirilecektir) |

| Türkçe Tanım (Bilgi Amaçlı) | Özellik | Özellik Seti / İlişki | Veri Tipi (Bilgi Amaçlı) | Açıklama |
|---|---|---|---|---|
| Bina Kimliği | `BuildingID` | `Pset_BuildingCommon` | Metin | Bir binaya atanan benzersiz bir tanımlayıcıdır. Planlama başvurusu yapıldığı anda geçici bir kimlik atanır. Bu geçici kimlik, bina yasal binalar ve taşınmazlar veri tabanına kaydedildiğinde kalıcı bir kimliğe dönüştürülür. |
| Kalıcı Kimlik | `IsPermanentID` | `Pset_BuildingCommon` | Mantıksal | Nesneye atanan kimliğin kalıcı (= DOĞRU) mı yoksa geçici (= YANLIŞ) mi olduğunu belirtir. |
| Yapım Yöntemi | `ConstructionMethod` | `Pset_BuildingCommon` | Metin | Nesneye ait inşaat eyleminin türü; örneğin yeni inşaat, yenileme, tadilat vb. |
| Yangın Koruma Sınıfı | `FireProtectionClass` | `Pset_BuildingCommon` | Metin | İlgili ulusal bina yönetmeliğinde verilen yangın koruma sınıflandırma tablosuna göre binaya atanan ana yangın koruma sınıfı. |
| Kullanım Türü | `OccupancyType` | `Pset_BuildingCommon` | Metin | Bu nesneye ait kullanım türü; yürürlükteki ulusal bina yönetmeliğine göre tanımlanır. |
| Brüt Planlanan Alan | `GrossPlannedArea` | `Pset_BuildingCommon` | Sayı | Mekânsal yapı elemanının toplam planlanan brüt alanı. Mekânsal yapı elemanının planlanmasında kullanılır. |
| Net Planlanan Alan | `NetPlannedArea` | `Pset_BuildingCommon` | Sayı | Nesnenin toplam planlanan net alanı. Nesnenin planlanmasında kullanılır. |

*[PDF sayfa 43]*

*(Tablo önceki sayfadan sürüyor; sütun başlıkları okunabilirlik için yinelendi.)*

| Türkçe Tanım (Bilgi Amaçlı) | Özellik | Özellik Seti / İlişki | Veri Tipi (Bilgi Amaçlı) | Açıklama |
|---|---|---|---|---|
| Kat Sayısı | `NumberOfStoreys` | `Pset_BuildingCommon` | Sayı | Binadaki kat sayısı. `IfcBuildingStorey` varlığının kullanılmadığı durumlarda belirtilir. `IfcBuildingStorey` kullanılmışsa, kat sayısı oradan belirlenmelidir. |
| Adres Satırları | `AddressLines` | `Pset_Address` | Metin | Posta adresidir. NOT: Bir posta adresi kaydedilirken birden fazla satır (veya öğe) içerebilir. Normal kullanımda, aşağıdaki adres kavramlarının ilgili unsurlarını içermesi beklenir: Bina içindeki konum (ör. 3. Kat) Bina adı (ör. Interoperability House) Bina numarası (ör. 6400) Sokak adı (ör. Alliance Boulevard) Adres satırlarının tipik içeriği ülkeden ülkeye değişiklik gösterebilir. |
| iI / Kasaba | Town | `Pset_Address` | Metin | Bir kasaba veya şehrin adıdır. |
| Bölge / İlçe | Region | `Pset_Address` | Metin | Bir bölgenin adıdır. |
| Yükseklik | Height | `Qto_BuildingBaseQuantities` | Sayı | Karakteristik yükseklik. Bu binanın standart brüt yüksekliği; inşaat döşemesinin üst yüzeyinden bir üst döşemenin veya çatının üst yüzeyine kadar olan yükseklik. Sadece yüksekliğin sabit olduğu durumda belirtilir. |
| Saçak Yüksekliği | `EavesHeight` | `Qto_BuildingBaseQuantity` | Sayı | Bu katın standart net yüksekliği: inşaat döşemesinin üst yüzeyinden bir üst döşemenin veya çatının alt yüzeyine kadar olan yükseklik. Sadece yüksekliğin sabit olduğu durumda belirtilir. |
| Taban Alanı | `FootPrintArea` | `Qto_BuildingBaseQuantity` | Sayı | Binanın veya binaların kapladığı arsanın brüt alanı. |

*[PDF sayfa 44]*

|  |  |  |  |
|---|---|---|---|
| Brüt Kat Alanı | `GrossFloorArea` | `Qto_BuildingBaseQuantities` | Sayı |
| Net Kat Alanı | `NetFloorArea` | `Qto_BuildingBaseQuantities` | Sayı |
| Posta Kutusu | `PostalBox` | `Pset_Address` | Metin |

Mekânsal yapı elemanı içindeki tüm brüt kat alanlarının toplamıdır. Bina katındaki yapı elemanlarının alanlarını içerir. Katla ilişkilendirilmiş mekân ve yapı elemanlarının miktarlarına ek olarak belirtilebilir. Tutarsızlık durumunda, mekân ve yapı elemanlarının bireysel miktarları geçerlidir.

Tüm net kullanılabilir kat alanlarının toplamıdır. Binadaki yapı elemanlarının alanlarını içermez. Binaya atanmış mekân metrajma ek olarak belirtilebilir. Tutarsızlık durumunda, mekânların bireysel miktarları önceliklidir.

Tanımlı bir posta kutusuyla ilişkilendirilen adrestir.

*[PDF sayfa 45]*

## Tablo 6.8 `IfcBuildingStorey` Sınıfı Zorunlu Öznitelik ve Özellik Listesi

**Zorunlu Öznitelik ve Özellik Listesi**

| Bina Katı (`IfcBuildingStorey`) | Ön Tammlı Tip Listesi |
|---|---|
| Tanım: Bina katı bir kota sahiptir ve genellikle düşeyde sınırlanmış, (neredeyse) yatay bir mahal grubunu temsil eder. | - |

| Türkçe Tanım (Bilgi Amaçlı) | Öznitelik | İlişki | Veri Tipi (Bilgi Amaçlı) | Açıklama |
|---|---|---|---|---|
| İsim | Attribute: Name | - | Metin | IFC uyumlu yazılım sistemleri veya kullanıcılar tarafından kullanılmak üzere isteğe bağlı bir ad. [[`IfcRoot`]] sınıfının bazı alt tipleri için İsim niteliğinin eklenmesi gerekebilir. Bu, bir "where" kuralı ile zorunlu halel getirilecektir. (EK-5'e göre isimlendirilecektir) |

| Türkçe Tanım (Bilgi Amaçlı) | Özellik | Özellik Seti / İlişki | Veri Tipi (Bilgi Amaçlı) | Açıklama |
|---|---|---|---|---|
| Net Kat Alanı | `NetFloorArea` | `Qto_BuildingStoreyBaseQuantities` | Sayı | yı Tüm net kullanılabilir kat alanlarını toplamıdır. Binadaki yapı elemanlarının alanlarını içermez. Binaya atanmış mekân metrajına ek olarak belirtilebilir. Tutarsızlık durumunda, mekânların bireysel miktarları önceliklidir. |
| Taşıyıcı Döşeme Üst Kotu | `ElevationOfSSLRelative` | `Pset_BuildingStoreyCommon` | Sayı | Taşıyıcı döşeme seviyesinin üst yüzey kotu, yerel sıfır yüksekliğine göre verilen kot değeridir. Seviye değişkenlik gösteriyorsa ve belirgin biri baskın kot bulunmuyorsa, bu özellik girilmeyebilir. Üst yüzeyin geometrik konumlandırmasıyla herhangi bir tutarsızlık olması durumunda, geometrik gösterim dikkate alınır. |

*[PDF sayfa 46]*

## Tablo 6.9 `IfcBurner` Sınıfı Zorunlu Öznitelik ve Özellik Listesi

**Zorunlu Öznitelik ve Özellik Listesi**

| Brülör (`IfcBurner`) | Ön Tanımlı Tip Listesi |
|---|---|
| Tanım: Brülör, yakıtı yanma yoluyla ısıya dönüştüren bir cihazdır. Gaz, yağ ve odun brülörlerini içerir. | USERDEFINED, NOTDEFINED |

| Türkçe Tanım (Bilgi Amaçlı) | Öznitelik | İlişki | Veri Tipi (Bilgi Amaçlı) | Açıklama |
|---|---|---|---|---|
| Ön Tanımlı Tip | `PredefinedType` | - | Metin | Ön Tanımlı Tip. |
| İsim | Attribute: Name | - | Metin | IFC uyumlu yazılım sistemleri veya kullanıcılar tarafından kullanılmak üzere isteğe bağlı bir ad. [[`IfcRoot`]] sınıfının bazı alt tipleri için İsim niteliğinin eklenmesi gerekebilir. Bu, bir "where" kuralı ile zorunlu hale getirilecektir. (EK-5'e göre isimlendirilecektir) |
| Etiket | Attribute: Tag | - | Sayı veya Metin | Bir ürünün belirli bir örneğindeki etiket tanımlayıcısı, örneğin seri numarası veya konum numarasıdır. Bu, yerleştirilmiş örnek düzeyindeki tanımlayıcıdır. |
| Sistem | `IfcSystem` | `IfcRelAssignsToGroup>IfcSystem` | Metin | Bir sistem, Mimarlık, Mühendislik ve Yapım ürünü içerisinde, ortak bir amaç, işlev ya da hizmet sunmak üzere bir araya getirilmiş ilişkili parçalardan oluşan düzenli bir kombinasyondur. Sistem esasen, işlevsel olarak ilişkili ürünler grubudur. Bir veya birden fazla `IfcProduct` örneğinin (sistem üyelerinin) gruplandırma `IfcRelAssignsToGroup` aracılığıyla yönetilir. |

| Türkçe Tanım (Bilgi Amaçlı) | Özellik | Özellik Seti / İlişki | Veri Tipi (Bilgi Amaçlı) | Açıklama |
|---|---|---|---|---|
| Nominal Güç Tüketimi | `NominalPowerConsumption` | `Pset_ElectricalDeviceCommon` | Sayı | Nominal toplam güç tüketimi. |

*Not: Bu parametreler IFC'de ilişkiler yoluyla kurgulanmaktadır.*

*[PDF sayfa 47]*

## Tablo 6.10 `IfcCableCarrierFitting` Sınıfı Zorunlu Öznitelik ve Özellik Listesi

**Zorunlu Öznitelik ve Özellik**

**Kablo Taşıyıcı Bağlantı Parçası (`IfcCableCarrierFitting`)**

Tanım: Bir kablo taşıyıcı bağlantı parçası, kablo taşıyıcı sisteminde bir bağlantı noktasına veya geçiş yerine yerleştirilen bağlantı elemanıdır.

| Türkçe Tanım (Bilgi Amaçlı) | Öznitelik | İlişki | Veri Tipi (Bilgi Amaçlı) |
|---|---|---|---|
| Ön Tanımlı Tip | `PredefinedType` | - | Metin |
| İsim | Attribute: Name | - | Metin |
| Etiket | Attribute: Tag | - | Sayı veya Metin |

Listesi

Ön Tanımlı Tip Listesi

BEND, CONNECTOR, JUNCTION, TRANSITION, USERDEFINED, NOTDEFINED

Açıklama

Ön Tanımlı Tip.

IFC uyumlu yazılım sistemleri veya kullanıcılar tarafından kullanılmak üzere isteğe bağlı bir ad. [[`IfcRoot`]] sınıfının bazı alt tipleri için İsim niteliğinin eklenmesi gerekebilir. Bu, bir "where" kuralı ile zorunlu hale getirilecektir. (EK-5'e göre isimlendirilecektir)

Bir ürünün belirli bir örneğindeki etiket tanımlayıcısı, örneğin seri numarası veya konum numarasıdır. Bu, yerleştirilmiş örnek düzeyindeki tanımlayıcıdır.

*[PDF sayfa 48]*

## Tablo 6.11 `IfcCableCarrierSegment` Sınıfı Zorunlu Öznitelik ve Özellik Listesi

**Zorunlu Öznitelik ve Özellik Listesi**

| Kablo Taşıyıcı Segment (`IfcCableCarrierSegment`) | Ön Tanımlı Tip Listesi |
|---|---|
| Tanım: Bir kablo taşıyıcı segmenti, özellikle kabloları taşımak ve desteklemek için kullanılan bir akış segmentidir. | CABLE BRACKET, CABLE LADDER SEGMENT, CABLE TRAY SEGMENT, CABLE TRUNKING SEGMENT, CATENARY WIRE, CONDUIT SEGMENT, DROPPER, USERDEFINED, NOTDEFINED |

| Türkçe Tanım (Bilgi Amaçlı) | Öznitelik | İlişki | Veri Tipi (Bilgi Amaçlı) | Açıklama |
|---|---|---|---|---|
| Ön Tanımlı Tip | `PredefinedType` | - | Metin | Ön Tanımlı Tip. |
| İsim | Attribute: Name | - | Metin | IFC uyumlu yazılım sistemleri veya kullanıcılar tarafından kullanılmak üzere isteğe bağlı bir ad. [[`IfcRoot`]] sınıfının bazı alt tipleri için İsim niteliğinin eklenmesi gerekebilir. Bu, bir "where" kuralı ile zorunlu hale getirilecektir. (EK-5'e göre isimlendirilecektir) |
| Etiket | Attribute: Tag | - | Sayı veya Metin | Bir ürünün belirli bir örneğindeki etiket tanımlayıcısı, örneğin seri numarası veya konum numarasıdır. Bu, yerleştirilmiş örnek düzeyindeki tanımlayıcıdır. |
| Sistem | `IfcSystem` | `IfcRelAssignsToGroup>IfcSystem` | Metin | Bir sistem, Mimarlık, Mühendislik ve Yapım ürünü içerisinde, ortak bir amaç, işlev ya da hizmet sunmak üzere bir araya getirilmiş ilişkili parçalardan oluşan düzenli bir kombinasyondur. Sistem esasen, işlevsel olarak ilişkili ürünler grubudur. Bir veya birden fazla `IfcProduct` örneğinin (sistem üyelerinin) gruplandırma ilişkisi, `IfcRelAssignsToGroup` aracılığıyla yönetilir. |

*Not: Bu parametreler IFC'de ilişkiler yoluyla kurgulanmaktadır.*

*[PDF sayfa 49]*

## Tablo 6.12 `IfcCaissonFoundation` Sınıfı Zorunlu Öznitelik ve Özellik Listesi

**Zorunlu Öznitelik ve Özellik Listesi**

| Keson Temeli (`IfcCaissonFoundation`) | Ön Tanımlı Tip Listesi |
|---|---|
| Tanım: Keson temeli esasen açık veya kapalı olabilen boş bir kutudur. | WELL, CAISSON, USERDEFINED, NOTDEFINED |

| Türkçe Tanım (Bilgi Amaçlı) | Öznitelik | ilişki | Veri Tipi (Bilgi Amaçlı) | Açıklama |
|---|---|---|---|---|
| Ön Tanımlı Tip | `PredefinedType` | - | Metin | Ön Tanımlı Tip. |
| İsim | Attribute: Name | - | Metin | IFC uyumlu yazılım sistemleri veya kullanıcılar tarafından kullanılmak üzere isteğe bağlı bir ad. [[`IfcRoot`]] sınıfının bazı alt tipleri için İsim niteliğinin eklenmesi gerekebilir. Bu, bir "where" kuralı ile zorunlu hale getirilecektir. (EK-5'e göre isimlendirilecektir) |
| Etiket | Attribute: Tag | - | Sayı veya Metin | Bir ürünün belirli bir örneğindeki etiket tanımlayıcısı, örneğin seri numarası veya konum numarasıdır. Bu, yerleştirilmiş örnek düzeyindeki tanımlayıcıdır. |
| Malzeme | Attribute (`IfcRelAssociatesMaterial>IfcMaterial`): Name\* | - | Metin | [`IfcMaterial`], yapı elemanlarını (fiziksel ürünler veya bileşenleri) oluşturmak için kullanılabilecek homojen veya heterojen maddelerdir. |

\*Attribute (`IfcRelAssociatesMaterial>IfcMaterial`): Name\* bilgisi giriliken: "Malzeme İsmi\_Dayanım" formatı kullanılmalıdır. (Örnek:Çelik\_S355 Beton\_C25)

*[PDF sayfa 50]*

## Tablo 6.13 `IfcChiller` Sınıfı Zorunlu Öznitelik ve Özellik Listesi

**Zorunlu Öznitelik ve Özellik Listesi**

| Soğutma Grubu (`IfcChiller`) | Ön Tanımlı Tür |
|---|---|
| Tanım: Soğutma grubu, bir sıvıdaki ısıyı uzaklaştırmak için buhar sıkıştırmalı veya absorbsiyonlu soğutma çevrimini kullanan bir cihazdır. Genellikle su veya su-glikol karışımı olan akışkanı soğutmak için kullanılır. Soğutulmuş akışkan daha sonra bir binadaki havayı soğutmak ve nemini gidermek için kullanılır. | AIR COOLED, WATER COOLED, HEAT RECOVERY, USERDEFINED, NOTDEFINED |

| Türkçe Tanım (Bilgi Amaçlı) | Öznitelik | İlişki | Veri Tipi (Bilgi Amaçlı) | Açıklama |
|---|---|---|---|---|
| Ön Tanımlı Tip | `PredefinedType` | - | Metin | Ön Tanımlı Tip. |
| İsim | Attribute: Name | - | Metin | IFC uyumlu yazılım sistemleri veya kullanıcılar tarafından kullanılmak üzere isteğe bağlı bir ad. [[`IfcRoot`]] sınıfının bazı alt tipleri için İsim niteliğinin eklenmesi gerekebilir. Bu, bir "where" kuralı ile zorunlu hale getirilecektir. (EK-5'e göre isimlendirilecektir) |
| Etiket | Attribute: Tag | - | Sayı veya Metin | Bir ürünün belirli bir örneğindeki etiket tanımlayıcısı, örneğin seri numarası veya konum numarasıdır. Bu, yerleştirilmiş örnek düzeyindeki tanımlayıcıdır. |
| Sistem | `IfcSystem` | `IfcRelAssignsToGroup>IfcSystem` | Metin | Bir sistem, Mimarlık, Mühendislik ve Yapım ürünü içerisinde, ortak bir amaç, işlev ya da hizmet sunmak üzere bir araya getirilmiş ilişkili parçalardan oluşan düzenli bir kombinasyondur. Sistem esasen, işlevsel olarak ilişkili ürünler grubudur. Bir veya birden fazla `IfcProduct` örneğinin (sistem üyelerinin) gruplandırma `IfcRelAssignsToGroup` aracılığıyla yönetilir. |

| Türkçe Tanım (Bilgi Amaçlı) | Özellik | Özellik Seti / İlişki | Veri Tipi (Bilgi Amaçlı) | Açıklama |
|---|---|---|---|---|
| Soğutucu Kapasitesi | `ChillerCapacity` | `Pset_ChillerTypeCommon` | Sayı | Yetkili kurum tarafından tanımlanan standart koşullarda, soğutucunun nominal soğutma kapasitesi. |
| Nominal Güç Tüketimi | `NominalPowerConsumption` | `Pset_ElectricalDeviceCommon` | Sayı | Nominal toplam güç tüketimi. |

*Not: Bu parametreler IFC'de ilişkiler yoluyla kurgulanmaktadır.*

*[PDF sayfa 51]*

## Tablo 6.14 `IfcChimney` Sınıfı Zorunlu Öznitelik ve Özellik Listesi

**Zorunlu Öznitelik ve Özellik**

**Baca (`IfcChimney`)**

Tanım: Bacalar genellikle bir binanın yapısının dikey ya da dikeye yakın bölümleridir ve bina kabuğunun bir parçasını oluştururlar. Genellikle prefabrik veya yerinde dökme betondan inşa edilirler; günümüzde tuğla ile yapılmaları ise nadirdir.

| Türkçe Tanım (Bilgi Amaçlı) | Öznitelik | İlişki | Veri Tipi (Bilgi Amaçlı) |
|---|---|---|---|
| Ön Tanımlı Tip | `PredefinedType` | - | Metin |
| İsim | Attribute: Name | - | Metin |
| Etiket | Attribute: Tag | - | Sayı veya Metin |
| Malzeme | Attribute(IfcRelAssociatesMaterial>IfcMaterial): Name\* | - | Metin |

Listesi

Ön Tanımlı Tip Listesi

### USERDEFINED, NOTDEFINED

Açıklama

Ön Tanımlı Tip.

IFC uyumlu yazılım sistemleri veya kullanıcılar tarafından kullanılmak üzere isteğe bağlı bir ad. [[`IfcRoot`]] sınıfım hazı alt tipleri için İsim niteliğinin eklenmesi gerekebilir. Bu, bir "where" kuralı ile zorunlu hale getirilecektir. (EK-5'e göre isimlendirilecektir)

Bir ürünün belirli bir örneğindeki etiket tanımlayıcısı, örneğin seril numarası veya konum numarasıdır. Bu, yerleştirilmiş örnek düzeyindeki tanımlayıcıdır.

[`IfcMaterial`]. yapı elemanlarını (fiziksel ürünler veya bileşenleri) oluşturmak için kullanılabilecek homojen veya heterojen maddelerdir. \*Attribute (`IfcRelAssociatesMaterial>IfcMaterial`): Name\* bilgisi giriliken: "Malzeme İsmi\_Dayanım" formatı kullanılmalıdır. (Örnek:Çelik\_S355 Beton\_(25)

*[PDF sayfa 52]*

## Tablo 6.15 `IfcCoil` Sınıfı Zorunlu Öznitelik ve Özellik Listesi

Zorunlu Öznitelik ve Özellik Listesi

| Serpantin (`IfcCoil`) | Ön Tanımlı Tip Listesi |
|---|---|
| Tanım: Serpantin, birbirine karışmayan akışkanlar arasında ısı transferi sağlamak için kullanılan bir cihazdır. Yaygın bir örnek, yüzeyi boyunca hareket eden havadan ısıyı uzaklaştırmak için soğutulmuş su, antifriz veya soğutucu akışkan dolaştırılan kanatlı bir serpantinin kullanıldığı soğutma serpantinidir. Bir serpantin, bir ısıtma veya soğutma akışkanı taşıyan bir dizi borunun (serpantin) bir hava akımına yerleştirilmesiyle ısıtma veya soğutma amaçları için kullanılabilir. Serpantin, serpantin formunda demetlenmiş borulardan veya genişletilmiş bir ısı transfer yüzeyi sağlayan kanatlı borulardan yapılabilir. | DXCOOLING COIL, ELECTRIC HEATING COIL, GAS HEATING COIL, HYDRONIC COIL, STEAM HEATING COIL, WATER COOLING COIL, WATER HEATING COIL, USERDEFINED, NOTDEFINED |

| Türkçe Tanım (Bilgi Amaçlı) | Öznitelik | İlişki | Veri Tipi (Bilgi Amaçlı) | Açıklama |
|---|---|---|---|---|
| Ön Tanımlı Tip | `PredefinedType` | - | Metin | Ön Tanımlı Tip. |
| İsim | Attribute: Name | - | Metin | IFC uyumlu yazılım sistemleri veya kullanıcılar tarafından kullanılmak üzere isteğe bağlı bir ad. [[`IfcRoot`]] sınıfının bazı alt tipleri için İsim niteliğinin eklenmesi gerekebilir. Bu, bir "where" kuralı ile zorunlu hale getirilecektir. (EK-5'e göre isimlendirilecektir) |
| Etiket | Attribute: Tag | - | Sayı veya Metin | Bir ürünün belirli bir örneğindeki etiket tanımlayıcısı, örneğin seri numarası veya konum numarasıdır. Bu, yerleştirilmiş örnek düzeyindeki tanımlayıcıdır. |
| Sistem | `IfcSystem` | `IfcRelAssignsToGroup>IfcSystem` | Metin | Bir sistem, Mimarlık, Mühendislik ve Yapım ürünü içerisinde, ortak bir amaç, işlev ya da hizmet sunmak üzere bir araya getirilmiş ilişkili parçalardan oluşan düzenli bir kombinasyondur. Sistem esasen, işlevsel olarak ilişkili ürünler grubudur. Bir veya birden fazla `IfcProduct` örneğinin (sistem üyelerinin) gruplandırma ilişkisi, `IfcRelAssignsToGroup` aracılığıyla yönetilir. |

| Türkçe Tanım (Bilgi Amaçlı) | Özellik | Özellik Seti / İlişki | Veri Tipi (Bilgi Amaçlı) | Açıklama |
|---|---|---|---|---|
| Nominal Gizli Kapasite | `NominalLatentCapacity` | `Pset_CoilTypeCommon` | Sayı | Nominal gizli kapasite. |
| Nominal Hissedilir Kapasite | `NominalSensibleCapacity` | `Pset_CoilTypeCommon` | Sayı | Nominal hissedilir kapasite. |

*Not: Bu parametreler IFC'de ilişkiler yoluyla kurgulanmaktadır.*

*[PDF sayfa 53]*

## Tablo 6.16 `IfcColumn` Sınıfı Zorunlu Öznitelik ve Özellik Listesi

**Zorunlu Öznitelik ve Özellik Listesi**

| Kolon (`IfcColumn`) | Ön Tanımlı Tip Listesi |
|---|---|
| Tanım: Kolon genellikle statik akslarla hizalanan, düşey yapısal veya mimari bir elemandır. Çoğu durumda. basınç yoluyla üst yapıdaki yükü alttaki diğer yapısal elemanlara aktaran düşey ya da düşeye yakın konumlu bir yapısal elemanı temsil eder. Mimari bakış açısından da böyle bir elemanı temsil edebilir; bu durumda yük taşımayan bir eleman olabilir. Bir kolonun yapısal olarak yük taşıyan veya yük taşımayan bir eleman olup olmadığı [[`Pset_ColumnCommon]]-[[LoadBearing`]] özelliğiyle belirlenir. | COLUMN, PIERSTEM, PIERSTEM\_SEGMENT, PILASTER, STANDCOLUMN, USERDEFINED, NOTDEFINED |

| Türkçe Tanım (Bilgi Amaçlı) | Öznitelik | İlişki | Veri Tipi (Bilgi Amaçlı) | Açıklama |
|---|---|---|---|---|
| Ön Tanımlı Tip | `PredefinedType` | - | Metin | Ön Tanımlı Tip. |
| İsim | Attribute: Name | - | Metin | IFC uyumlu yazılım sistemleri veya kullanıcılar tarafından kullanılmak üzere isteğe bağlı bir ad. [[`IfcRoot`]] sınıfının bazı alt tipleri için İsim niteliğinin eklenmesi gerekebilir. Bu, bir "where" kuralı ile zorunlu hale getirilecektir. (EK-5'e göre isimlendirilecektir) |
| Etiket | Attribute: Tag | - | Sayı veya Metin | Bir ürünün belirli bir örneğindeki etiket tanımlayıcısı, örneğin seri numarası veya konum numarasıdır. Bu, yerleştirilmiş örnek düzeyindeki tanımlayıcıdır. |
| Malzeme | Attribute (`IfcRelAssociatesMaterial>IfcMaterial`): Name\* | - | Metin | [`IfcMaterial`], yapı elemanlarını (fiziksel ürünler veya bileşenleri) oluşturmak için kullanılabilecek homojen veya heterojen maddelerdir. |

*[PDF sayfa 54]*

| Türkçe Tanım (Bilgi Amaçlı) | Özellik | Özellik Seti / İlişki | Veri Tipi (Bilgi Amaçlı) | Açıklama |
|---|---|---|---|---|
| Yangın Dayanım Sınıfı | `FireRating` | `Pset_ColumnCommon` | Metin | Bu nesnenin yangın dayanım sınıfı. Ulusal yangın güvenliği sınıflandırmasına göre (Türkiye için: Binaların Yangından Korunması Hakkında Yönetmeliğe göre) verilmiştir. |
| Taşıyıcı | `LoadBearing` | `Pset_ColumnCommon` | Mantıksal | Nesnenin yük taşıma amacıyla tasarlandığını (DOĞRU) veya tasarlanmadığını (YANLIŞ) belirtir . |

\*Attribute (`IfcRelAssociatesMaterial>IfcMaterial`): Name\* bilgisi giriliken: "Malzeme İsmi\_Dayanım" formatı kullanılmalıdır. (Örnek:Çelik\_S355 Beton\_C25)

*[PDF sayfa 55]*

## Tablo 6.17 `IfcCommunicationsAppliance` Sınıfı Zorunlu Öznitelik ve Özellik Listesi

**Zorunlu Öznitelik ve Özellik Listesi**

| İletişim Cihazı (`IfcCommunicationsAppliance`) | Ön Tanımlı Tip Listesi |
|---|---|
| Tanım: İletişim cihazı, elektronik veya dijital bilgiyi veri ya da ses olarak ileten ve alan bir aygıttır. | ANTENNA, COMPUTER, FAX, GATEWAY, MODEM, NETWORK APPLIANCE, NETWORK BRIDGE, NETWORK HUB, PRINTER, REPEATER, ROUTER, SCANNER, USERDEFINED, NOTDEFINED |

| Türkçe Tanım (Bilgi Amaçlı) | Öznitelik | İlişki | Veri Tipi (Bilgi Amaçlı) | Açıklama |
|---|---|---|---|---|
| Ön Tanımlı Tip | `PredefinedType` | - | Metin | Ön Tanımlı Tip. |
| İsim | Attribute: Name | - | Metin | IFC uyumlu yazılım sistemleri veya kullanıcılar tarafından kullanılmak üzere isteğe bağlı bir ad. [[`IfcRoot`]] sınıfının bazı alt tipleri için İsim niteliğinin eklenmesi gerekebilir. Bu, bir "where" kuralı ile zorunlu hale getirilecektir. (EK-5'e göre isimlendirilecektir) |
| Etiket | Attribute: Tag | - | Sayı veya Metin | Bir ürünün belirli bir örneğindeki etiket tanımlayıcısı, örneğin seri numarası veya konum numarasıdır. Bu, yerleştirilmiş örnek düzeyindeki tanımlayıcıdır. |

*[PDF sayfa 56]*

## Tablo 6.18 `IfcCondenser` Sınıfı Zorunlu Öznitelik ve Özellik Listesi

**Zorunlu Öznitelik ve Özellik Listesi**

| Kondenser (`IfcCondenser`) | Ön Tanımlı Tip Listesi |
|---|---|
| Tanım: Kondenser, genellikle bir soğutucuyu (refrigerant) gaz halinden sıvı hale yoğunlaştırarak ısıyı dağıtmak için kullanılan bir cihazdır. | AIR COOLED, EVAPORATIVE COOLED, WATER COOLED, WATER COOLED BRAZEDPLATE, WATER COOLED SHELL COIL, WATER COOLED SHELL TUBE, WATER COOLED TUBE IN TUBE, USERDEFINED, NOTDEFINED |

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

*[PDF sayfa 57]*

## Tablo 6.19 `IfcCoolingTower` Sınıfı Zorunlu Öznitelik ve Özellik Listesi

**Zorunlu Öznitelik ve Özellik Listesi**

| Soğutma Kulesi (`IfcCoolingTower`) | Ön Tanımlı Tip Listesi |
|---|---|
| Tanım: Soğutma kulesi, su gibi bir akışkanı dolaştırarak sıcaklığını kısmi buharlaşma yoluyla düşürmek için ısıyı ortam havasına atan bir cihazdır. | NATURAL DRAFT, MECHANICAL INDUCED DRAFT, MECHANICAL FORCED DRAFT, USERDEFINED, NOTDEFINED |

| Türkçe Tanım (Bilgi Amaçlı) | Öznitelik | İlişki | Veri Tipi (Bilgi Amaçlı) | Açıklama |
|---|---|---|---|---|
| Ön Tanımlı Tip | `PredefinedType` | - | Metin | Ön Tanımlı Tip. |
| İsim | Attribute: Name | - | Metin | IFC uyumlu yazılım sistemleri veya kullanıcılar tarafından kullanılmak üzere isteğe bağlı bir ad. [[`IfcRoot`]] sınıfının bazı alt tipleri için İsim niteliğinin eklenmesi gerekebilir. Bu, bir "where" kuralı ile zorunlu hale getirilecektir. (EK-5'e göre isimlendirilecektir) |
| Etiket | Attribute: Tag | - | Sayı veya Metin | Bir ürünün belirli bir örneğindeki etiket tanımlayıcısı, örneğin seri numarası veya konum numarasıdır. Bu, yerleştirilmiş örnek düzeyindeki tanımlayıcıdır. |
| Sistem | `IfcSystem` | `IfcRelAssignsToGroup>IfcSystem` | Metin | Bir sistem, Mimarlık, Mühendislik ve Yapım ürünü içerisinde, ortak bir amaç, işlev ya da hizmet sunmak üzere bir araya getirilmiş ilişkili parçalardan oluşan düzenli bir kombinasyondur. Sistem esasen, işlevsel olarak ilişkili ürünler grubudur. Bir veya birden fazla `IfcProduct` örneğinin (sistem üyelerinin) gruplandırma ilişkisi, `IfcRelAssignsToGroup` aracılığıyla yönetilir. |

| Türkçe Tanım (Bilgi Amaçlı) | Özellik | Özellik Seti / İlişki | Veri Tipi (Bilgi Amaçlı) | Açıklama |
|---|---|---|---|---|
| Nominal Kapasite | `NominalCapacity` | `Pset_CoolingTowerTypeCommon` | Sayı | Kulenin toplam nominal veya hacimsel soğutma kapasitesi. |

*Not: Bu parametreler IFC'de ilişkiler yoluyla kurgulanmaktadır.*

*[PDF sayfa 58]*

## Tablo 6.20 `IfcCovering` Sınıfı Zorunlu Öznitelik ve Özellik Listesi

**Zorunlu Öznitelik ve Özellik Listesi**

| Kaplama (`IfcCovering`) | Ön Tanımlı Tip Listesi |
|---|---|
| Tanım: Kaplama, başka bir elemanın bir kısmını örten ve tamamen o elemana bağlı olan bir elemandır. `IfcCovering`, bir kaplama türünün oluşumunu tanımlar ve (varsa) bu tür `IfcCoveringType` tarafından ifade edilir. | CEILING, CLADDING, COPING, FLOORING, INSULATION, MEMBRANE, MOLDING, ROOFING, SKIRTING BOARD, SLEEVING, TOPPING, WRAPPING, USERDEFINED, NOTDEFINED |

| Türkçe Tanım (Bilgi Amaçlı) | Öznitelik | İlişki | Veri Tipi (Bilgi Amaçlı) | Açıklama |
|---|---|---|---|---|
| Ön Tanımlı Tip | `PredefinedType` | - | Metin | Ön Tanımlı Tip. |
| İsim | Attribute: Name | - | Metin | IFC uyumlu yazılım sistemleri veya kullanıcılar tarafından kullanılmak üzere isteğe bağlı bir ad. [[`IfcRoot`]] sınıfının bazı alt tipleri için İsim niteliğinin eklenmesi gerekebilir. Bu, bir "where" kuralı ile zorunlu hale getirilecektir. (EK-5'e göre isimlendirilecektir) |
| Etiket | Attribute: Tag | - | Sayı veya Metin | Bir ürünün belirli bir örneğindeki etiket tanımlayıcısı, örneğin seri numarası veya konum numarasıdır. Bu, yerleştirilmiş örnek düzeyindeki tanımlayıcıdır. |
| Malzeme | Attribute (`IfcRelAssociatesMaterial>IfcMaterial`): Name\* | - | Metin | [`IfcMaterial`], yapı elemanlarını (fiziksel ürünler veya bileşenleri) oluşturmak için kullanılabilecek homojen veya heterojen maddelerdir. |
| Kaplama Malzemesi | Attribute: Material | Attribute(IfcRelAssociatesMaterial>IfcMaterialLayerSetUsage>IfcMaterialLayerSet>IfcMaterialLayer): Material\*\* | Metin | Katmanın oluşturulduğu malzemeye yönelik isteğe bağlı referans. Bu değerin boş bırakılması, katmanın malzeme içermediğini (örneğin hava boşluğu olduğunu) ifade etmez; yalnızca ilgili aşamada malzeme bilgisinin tanımlanmadığını gösterir. |
| Kaplama Kalınlığı | Attribute: `LayerThickness` | Attribute(IfcRelAssociatesMaterial>IfcMaterialLayerSetUsage>IfcMaterialLayerSet>IfcMaterialLayer): `LayerThickness`\*\* | Metin | Malzeme katmanının kalınlığıdır. "Kalınlık" teriminin anlamı, kullanımına bağlı olarak değişir. `IfcMaterialLayerSetUsage` kullanan yapı elemanlarında, bu boyut `IfcMaterialLayerSetUsage` içinde belirtilen pozitif `LayerSetDirection` doğrultusunda ölçülür. NOT: Zar (membran) gibi kalınlığı sıfıra çok yakın olan malzemeler için bu öznitelik değeri 0 olabilir. Kalınlığı 0 olan malzeme katmanları geometrik gösterimde görüntülenmeyebilir. |

| Türkçe Tanım (Bilgi Amaçlı) | Özellik | Özellik Seti / İlişki | Veri Tipi (Bilgi Amaçlı) | Açıklama |
|---|---|---|---|---|

*[PDF sayfa 59]*

*(Tablo önceki sayfadan sürüyor; sütun başlıkları okunabilirlik için yinelendi.)*

| Türkçe Tanım (Bilgi Amaçlı) | Özellik | Özellik Seti / İlişki | Veri Tipi (Bilgi Amaçlı) | Açıklama |
|---|---|---|---|---|
| Yangın Dayanım Sınıfı | `FireRating` | `Pset_CoveringCommon` | Metin | Bu nesnenin yangın dayanım sınıfı. Ulusal yangın güvenliği sınıflandırmasına göre (Türkiye için: Binaların Yangından Korunması Hakkında Yönetmeliğe göre) verilmiştir. |

\*\* Kaplamalarla ilgili bu özellikler: (1) `IfcSpace` içindeki tanımlardan faydalanılarak yada (2) IfcCovering'de belirtilen ilişkiler kurularak doğrudan modelleme ile tanımlanacak. Bu nedenle belirtilen bu iki yöntemden biri zorunludur.

*Not: Bu parametreler IFC'de ilişkiler yoluyla kurgulanmaktadır.*

\*Attribute (`IfcRelAssociatesMaterial>IfcMaterial`): Name\* bilgisi giriliken: "Malzeme İsmi\_Dayanım" formatı kullanılmalıdır. (Örnek:Çelik\_S355 Beton\_C25)

*[PDF sayfa 60]*

## Tablo 6.21 `IfcCurtainWall` Sınıfı Zorunlu Öznitelik ve Özellik Listesi

**Zorunlu Öznitelik ve Özellik Listesi**

| Giydirme Cephe (`IfcCurtainWall`) | Ön Tanımlı Tip Listesi |
|---|---|
| Tanım: Bir giydirme cephe, zemine oturmaktan ziyade döşeme/çatı strüktürünün kenarına asılmış bileşenlerin bir araya gelmesiyle oluşturulan bir bina duvarıdır. Giydirme cephe, bir bina elemanı bileşimi olarak temsil edilir ve [[`IfcBuiltElement`]] alt tipi olarak `IfcRelAggregates` ilişkisini kullanarak uygulanır. Giydirme cephe genellikle dış cephe olur, ancak [[`Pset_CurtainWallCommon]]-[[IsExternal`]] kullanılarak iç mekân giydirme cepheleri de tanımlanabilir. | USERDEFINED, NOTDEFINED |

| Türkçe Tanım (Bilgi Amaçlı) | Öznitelik | İlişki | Veri Tipi (Bilgi Amaçlı) | Açıklama |
|---|---|---|---|---|
| Ön Tanımlı Tip | `PredefinedType` | - | Metin | Ön Tanımlı Tip. |
| İsim | Attribute: Name | - | Metin | IFC uyumlu yazılım sistemleri veya kullanıcılar tarafından kullanılmak üzere isteğe bağlı bir ad. [[`IfcRoot`]] sınıfının bazı alt tipleri için İsim niteliğinin eklenmesi gerekebilir. Bu, bir "where" kuralı ile zorunlu hale getirilecektir. (EK-5'e göre isimlendirilecektir) |
| Etiket | Attribute: Tag | - | Sayı veya Metin | Bir ürünün belirli bir örneğindeki etiket tanımlayıcısı, örneğin seri numarası veya konum numarasıdır. Bu, yerleştirilmiş örnek düzeyindeki tanımlayıcıdır. |
| Malzeme | Attribute(IfcRelAssociatesMaterial>IfcMaterial): Name\* | - | Metin | [`IfcMaterial`], yapı elemanlarını (fiziksel ürünler veya bileşenleri) oluşturmak için kullanılabilecek homojen veya heterojen maddelerdir. |

| Türkçe Tanım (Bilgi Amaçlı) | Özellik | Özellik Seti / İlişki | Veri Tipi (Bilgi Amaçlı) | Açıklama |
|---|---|---|---|---|
| Yangın Dayanım Sınıfı | `FireRating` | `Pset_CurtainWallCommon` | Metin | Bu nesnenin yangın dayanım sınıfı. Ulusal yangın güvenliği sınıflandırmasına göre (Türkiye için: Binaların Yangından Korunması Hakkında Yönetmeliğe göre) verilmiştir. |

\*Attribute (`IfcRelAssociatesMaterial>IfcMaterial`): Name\* bilgisi giriliken: "Malzeme İsmi\_Dayanım" formatı kullanılmalıdır. (Örnek:Çelik\_S355 Beton\_C25)

*[PDF sayfa 61]*

## Tablo 6.22 `IfcDamper` Sınıfı Zorunlu Öznitelik ve Özellik Listesi

**Zorunlu Öznitelik ve Özellik Listesi**

| Damper (`IfcDamper`) | Ön Tanımlı Tip Listesi |
|---|---|
| Tanım: Bir damper, tipik olarak bir HVAC kanal dağıtım sisteminde yer alır ve hava akışını kontrol etmek veya düzenlemek için kullanılır. | BACK DRAFT DAMPER, BALANCING DAMPER, BLAST DAMPER, CONTROL DAMPER, FIRE DAMPER, FIRE SMOKE DAMPER, FUME HOOD EXHAUST, GRAVITY DAMPER, GRAVITY RELIEF DAMPER, RELIEF DAMPER, SMOKE DAMPER, USERDEFINED, NOTDEFINED |

| Türkçe Tanım (Bilgi Amaçlı) | Öznitelik | İlişki | Veri Tipi (Bilgi Amaçlı) | Açıklama |
|---|---|---|---|---|
| Ön Tanımlı Tip | `PredefinedType` | - | Metin | Ön Tanımlı Tip. |
| İsim | Attribute: Name | - | Metin | IFC uyumlu yazılım sistemleri veya kullanıcılar tarafından kullanılmak üzere isteğe bağlı bir ad. [[`IfcRoot`]] sınıfının bazı alt tipleri için İsim niteliğinin eklenmesi gerekebilir. Bu, bir "where" kuralı ile zorunlu hale  getirilecektir. (EK-5'e göre isimlendirilecektir) |
| Etiket | Attribute: Tag | - | Sayı veya Metin | Bir ürünün belirli bir örneğindeki etiket tanımlayıcısı, örneğin seri numarası veya konum numarasıdır. Bu, yerleştirilmiş örnek düzeyindeki tanımlayıcıdır. |
| Sistem | `IfcSystem` | `IfcRelAssignsToGroup>IfcSystem` | Metin | Bir sistem, Mimarlık, Mühendislik ve Yapım ürünü içerisinde, ortak bir amaç, işlev ya da hizmet sunmak üzere bir araya getirilmiş ilişkili parçalardan oluşan düzenli bir kombinasyondur. Sistem esasen, işlevsel olarak ilişkili ürünler grubudur. Bir veya birden fazla `IfcProduct` Örneğinin (sistem üyelerinin) gruplandırma ilişkisi, `IfcRelAssignsToGroup` aracılığıyla yönetilir. |

*Not: Bu parametreler IFC'de ilişkiler yoluyla kurgulanmaktadır.*

*[PDF sayfa 62]*

## Tablo 6.23 `IfcDistributionBoard` Sınıfı Zorunlu Öznitelik ve Özellik Listesi

**Zorunlu Öznitelik ve Özellik Listesi**

| Dağıtım Panosu (`IfcDistributionBoard`) | Ön Tanımlı Tip Listesi |
|---|---|
| Tanım: Dağıtım panosu, belirli bir amaç doğrultusunda elektriksel veya iletişim cihazlarının bir araya getirildiği enerji dağıtımını ve devrelerin kontrolünü sağlayan merkezi bir akış kontrol cihazıdır. | CONSUMER UNIT, DISTRIBUTION BOARD, MOTOR CONTROL CENTRE, SWITCH BOARD, USERDEFINED, NOTDEFINED |

| Türkçe Tanım (Bilgi Amaçlı) | Öznitelik | İlişki | Veri Tipi (Bilgi Amaçlı) | Açıklama |
|---|---|---|---|---|
| Ön Tanımlı Tip | `PredefinedType` | - | Metin | Ön Tanımlı Tip. |
| İsim | Attribute: Name | - | Metin | IFC uyumlu yazılım sistemleri veya kullanıcılar tarafından kullanılmak üzere isteğe bağlı bir ad. [[`IfcRoot`]] sınıfının bazı alt tipleri için İsim niteliğinin eklenmesi gerekebilir. Bu, bir "where" kuralı ile zorunlu hale getirilecektir. (EK-5'e göre isimlendirilecektir) |
| Etiket | Attribute: Tag | - | Sayı veya Metin | Bir ürünün belirli bir örneğindeki etiket tanımlayıcısı, örneğin seri numarası veya konum numarasıdır. Bu, yerleştirilmiş örnek düzeyindeki tanımlayıcıdır. |
| Sistem | `IfcSystem` | `IfcReiAssignsToGroup>IfcSystem` | Metin | Bir sistem, Mimarlık, Mühendislik ve Yapın ürünü içerisinde, ortak bir amaç, işlev ya da hizmet sunmak üzere bir araya getirilmiş ilişkili parçalardan oluşan düzenli bir kombinasyondur. Sistem esasen, işlevsel olarak ilişkili ürünler grubudur. Bir veya birden fazla `IfcProduct` örneğinin (sistem üyelerinin) gruplandırma ilişkisi, `IfcRelAssignsToGroup` aracılığıyla yönetilir. |

| Türkçe Tanım (Bilgi Amaçlı) | Özellik | Özellik Seti / İlişki | Veri Tipi (Bilgi Amaçlı) | Açıklama |
|---|---|---|---|---|
| Nominal Güç Tüketimi | `NominalPowerConsumption` | `Pset_ElectricalDeviceCommon` | Sayı | Nominal toplam güç tüketimi. |

*Not: Bu parametreler IFC'de ilişkiler yoluyla kurgulanmaktadır.*

*[PDF sayfa 63]*

## Tablo 6.24 `IfcDistributionSystem` Sınıfı Zorunlu Öznitelik ve Özellik Listesi

**Zorunlu Öznitelik ve Özellik Listesi**

| Dağıtım Sistemi (`IfcDistributionSystem`) | Ön Tanımlı Tip Listesi |
|---|---|
| Tanım: Bir dağıtım sistemi, bir dağıtım ortamının akışını almak, depolamak, sürdürmek, dağıtmak veya kontrol etmek üzere tasarlanmış bir ağdır. Yaygın bir örnek, bir pompa, bir tank ve sıcak suyu terminallere dağıtmak için birbirine bağlı bir boru sisteminden oluşan bir sıcak su ısıtma sistemidir. | AIRCONDITIONING, AUDIOVISUAL, CATENARY\_SYSTEM, CHEMICAL, CHILLEDWATER, COMMUNICATION, COMPRESSEDAIR, CONDENSERWATER, CONTROL, CONVEYING, DATA, DISPOSAL, DOMESTICCOLDWATER, DOMESTICHOTWATER, DRAINAGE, EARTHING, ELECTRICAL, ELECTROACOUSTIC, EXHAUST, FIREPROTECTION, FIXEDTRANSMISSIONNETWORK, FUEL, GAS, HAZARDOUS, HEATING, LIGHTING, LIGHTNINGPROTECTION, MOBILENETWORK, MONITORINGSYSTEM, MUNICIPALSOLIDWASTE, OIL, OPERATIONAL, OPERATIONALTELEPHONYSYSTEM, OVERHEAD\_CONTACTLINE\_SYSTEM, POWERGENERATION, RAINWATER, REFRIGERATION, RETURN\_CIRCUIT, SECURITY, SEWAGE, SIGNAL, STORMWATER, TELEPHONE, TV, VACUUM, VENT, VENTILATION, WASTEWATER, WATERSUPPLY, USERDEFINED, NOTDEFINED |

| Türkçe Tanım (Bilgi Amaçlı) | Öznitelik | İlişki | Veri Tipi (Bilgi Amaçlı) | Açıklama |
|---|---|---|---|---|
| Ön Tanımlı Tür | `PredefinedType` | - | Metin | Ön Tanımlı Tür. |
| Uzun İsim | Attribute: `LongName` | - | Metin | Referans amaçlı olarak kullanılan bağlamın uzun adı. |
| İsim | Attribute: Name | - | Metin | IFC uyumlu yazılım sistemleri veya kullanıcılar tarafından kullanılmak üzere isteğe bağlı bir ad. [[`IfcRoot`]] sınıfının bazı alt tipleri için İsim niteliğinin eklenmesi gerekebilir. Bu, bir "where" kuralı ile zorunlu hale getirilecektir. (EK-5'e göre isimlendirilecektir) |

*[PDF sayfa 64]*

## Tablo 6.25 `IfcDoor` Sınıfı Zorunlu Öznitelik ve Özellik Listesi

**Zorunlu Öznitelik ve Özellik Listesi**

| Kapı (`IfcDoor`) | Ön Tanımlı Tip Listesi |
|---|---|
| Tanım: Kapı, esas olarak insanlar, eşyalar, hayvanlar ve araçlar için kontrollü erişim sağlamak üzere kullanılan bir yapı elemanıdır. Menteşeli, eksenel. kayar ve ayrıca döner kapı veya katlanır kapı yapılarını içerir. | BOOM BARRIER, DOOR, GATE, TRAPDOOR, TURNSTILE, USERDEFINED, NOTDEFINED |

| Türkçe Tanım (Bilgi Amaçlı) | Öznitelik | İlişki | Veri Tipi (Bilgi Amaçlı) | Açıklama |
|---|---|---|---|---|
| Ön Tanımlı Tip | `PredefinedType` | - | Metin | Ön Tanımlı Tip. |
| İsim | Attribute: Name | - | Metin | IFC uyumlu yazılım sistemleri veya kullanıcılar tarafından kullanılmak üzere isteğe bağlı bir ad. [[`IfcRoot`]] sınıfının bazı alt tipleri için İsim niteliğinin eklenmesi gerekebilir. Bu, bir "where" kuralı ile zorunlu hale getirilecektir. (EK-5'e göre isimlendirilecektir) |
| Etiket | Attribute: Tag | - | Sayı veya Metin | Bir ürünün belirli bir örneğindeki etiket tanımlayıcısı, örneğin seri numarası veya konum numarasıdır. Bu, yerleştirilmiş örnek düzeyindeki tanımlayıcıdır. |

| Türkçe Tanım (Bilgi Amaçlı) | Özellik | Özellik Seti / İlişki | Veri Tipi (Bilgi Amaçlı) | Açıklama |
|---|---|---|---|---|
| Dışarıda | `IsExternal` | `Pset_DoorCommon` | Mantıksal | Yapı elemanının dışarıda kullanıma yönelik tasarlandığını (DOĞRU) veya tasarlanmadığını (YANLIŞ) gösterir. Eğer (DOĞRU) ise, bu bir dış eleman olup binanın dışına bakmaktadır. |
| Yangın Dayanım Sınıfı | `FireRating` | `Pset_DoorCommon` | Metin | Bu nesnenin yangın dayanım sınıfı. Ulusal yangın güvenliği sınıflandırmasına göre (Türkiye için: Binaların Yangından Korunması Hakkında Yönetmeliğe göre) verilmiştir. |
| Acil Çıkış | `FireExit` | `Pset_DoorCommon` | Mantıksal | Bu nesnenin yangın durumunda çıkış olarak kullanılmak üzere tasarlandığını (DOĞRU) veya tasarlanmadığını (YANLIŞ) belirtir. Burada. çıkış kapısı ulusal bina yönetmeliğine uygun olarak tanımlanmaktadır. |

*[PDF sayfa 65]*

## Tablo 6.26 `IfcDuctFitting` Sınıfı Zorunlu Öznitelik ve Özellik Listesi

**Zorunlu Öznitelik ve Özellik Listesi**

| Kanal Bağlantı Parçası (`IfcDuctFitting`) | Ön Tanımlı Tip Listesi |
|---|---|
| Tanım: Kanal bağlantı parçası, kanallı akış dağıtım sisteminde bir bağlantı veya geçiş elemanıdır ya da kanal segmentlerini bağlamak için kullanılır. Bu, akışkanın yönü ve debisi gibi akış özelliklerinde değişikliklere neden olur. | BEND, CONNECTOR, ENTRY, EXIT, JUNCTION, OBSTRUCTION, TRANSITION, USERDEFINED, NOTDEFINED |

| Türkçe Tanım (Bilgi Amaçlı) | Öznitelik | İlişki | Veri Tipi (Bilgi Amaçlı) | Açıklama |
|---|---|---|---|---|
| Ön Tanımlı Tip | `PredefinedType` | - | Metin | Ön Tanımlı Tip. |
| İsim | Attribute: Name | - | Metin | IFC uyumlu yazılım sistemleri veya kullanıcılar tarafından kullanılmak üzere isteğe bağlı bir ad. [[`IfcRoot`]] sınıfının bazı alt tipleri için İsim niteliğinin eklenmesi gerekebilir. Bu, bir "where" kuralı ile zorunlu hale getirilecektir. (EK-5'e göre isimlendirilecektir) |
| Etiket | Attribute: Tag | - | Sayı veya Metin | Bir ürünün belirli bir örneğindeki etiket tanımlayıcısı, örneğin seri numarası veya konum numarasıdır. Bu, yerleştirilmiş örnek düzeyindeki tanımlayıcıdır. |
| Sistem | `IfcSystem` | `IfcRelAssignsToGroup>IfcSystem` | Metin | Bir sistem, Mimarlık, Mühendislik ve Yapım ürünü içerisinde, ortak bir amaç, işlev ya da hizmet sunmak üzere bir araya getirilmiş ilişkili parçalardan oluşan düzenli bir kombinasyondur. Sistem esasen, işlevsel olarak ilişkili ürünler grubudur. Bir veya birden fazla `IfcProduct` örneğinin (sistem üyelerinin) gruplandırma ilişkisi, `IfcRelAssignsToGroup` aracılığıyla yönetilir. |

*Not: Bu parametreler IFC'de ilişkiler yoluyla kurgulanmaktadır.*

*[PDF sayfa 66]*

## Tablo 6.27 `IfcDuctSegment` Sınıfı Zorunlu Öznitelik ve Özellik Listesi

**Zorunlu Öznitelik ve Özellik Listesi**

| Kanal Segmenti (`IfcDuctSegment`) | Ön Tanımlı Tip Listesi |
|---|---|
| Tanım: Kanal segmenti, tipik olarak kanal ağı şebekesinin iki bölümünü birleştirmek için kullanılır. | FLEXIBLE SEGMENT, RIGID SEGMENT, USERDEFINED, NOTDEFINED |

| Türkçe Tanım (Bilgi Amaçlı) | Öznitelik | İlişki | Veri Tipi (Bilgi Amaçlı) | Açıklama |
|---|---|---|---|---|
| Ön Tanımlı Tip | `PredefinedType` | - | Metin | Ön Tanımlı Tip. |
| İsim | Attribute: Name | - | Metin | IFC uyumlu yazılım sistemleri veya kullanıcılar tarafından kullanılmak üzere isteğe bağlı bir ad. [[`IfcRoot`]] sınıfının bazı alt tipleri için İsim niteliğinin eklenmesi gerekebilir. Bu, bir "where" kuralı ile zorunlu hale getirilecektir. (EK-5'e göre isimlendirilecektir) |
| Etiket | Attribute: Tag | - | Sayı veya Metin | Bir ürünün belirli bir örneğindeki etiket tanımlayıcısı, örneğin seri numarası veya konum numarasıdır. Bu, yerleştirilmiş örnek düzeyindeki tanımlayıcıdır. |
| Sistem | `IfcSystem` | `IfcRelAssignsToGroup>IfcSystem` | Metin | Bir sistem, Mimarlık, Mühendislik ve Yapım ürünü içerisinde, ortak bir amaç, işlev ya da hizmet sunmak üzere bir araya getirilmiş ilişkili parçalardan oluşan düzenli bir kombinasyondur. Sistem esasen, işlevsel olarak ilişkili ürünler grubudur. Bir veya birden fazla `IfcProduct` örneğinin (sistem üyelerinin) gruplandırma ilişkisi, `IfcRelAssignsToGroup` aracılığıyla yönetilir. |

*Not: Bu parametreler IFC'de ilişkiler yoluyla kurgulanmaktadır.*

*[PDF sayfa 67]*

## Tablo 6.28 `IfcDuctSilencer` Sınıfı Zorunlu Öznitelik ve Özellik Listesi

**Zorunlu Öznitelik ve Özellik Listesi**

| Kanal Susturucusu (`IfcDuctSilencer`) | Ön Tanımlı Tip Listesi |
|---|---|
| Tanım: Kanal susturucusu, tipik olarak bir kanal dağıtım sistemine, hava hareketi, fan gürültüsü vb. kaynaklı gürültü seviyelerini bitişik alanda veya susturucu cihazın aşağı akışında azaltmak amacıyla monte edilen bir cihazdır. | FLATOVAL, RECTANGULAR, ROUND, USERDEFINED, NOTDEFINED |

| Türkçe Tanım (Bilgi Amaçlı) | Öznitelik | İlişki | Veri Tipi (Bilgi Amaçlı) | Açıklama |
|---|---|---|---|---|
| Ön Tanımlı Tip | `PredefinedType` | - | Metin | Ön Tanımlı Tip. |
| İsim | Attribute: Name | - | Metin | IFC uyumlu yazılım sistemleri veya kullanıcılar tarafından kullanılmak üzere isteğe bağlı bir ad. [[`IfcRoot`]] sınıfının bazı alt tipleri için İsim niteliğinin eklenmesi gerekebilir. Bu, bir "where" kuralı ile zorunlu hale getirilecektir. (EK-5'e göre isimlendirilecektir) |
| Etiket | Attribute: Tag | - | Sayı veya Metin | Bir ürünün belirli bir örneğindeki etiket tanımlayıcısı, örneğin seri numarası veya konum numarasıdır. Bu, yerleştirilmiş örnek düzeyindeki tanımlayıcıdır. |
| Sistem | `IfcSystem` | `IfcRelAssignsToGroup>IfcSystem` | Metin | Bir sistem, Mimarlık, Mühendislik ve Yapım ürünü içerisinde, ortak bir amaç, işlev ya da hizmet sunmak üzere bir araya getirilmiş ilişkili parçalardan oluşan düzenli bir kombinasyondur. Sistem esasen, işlevsel olarak ilişkili ürünler grubudur. Bir veya birden fazla `IfcProduct` örneğinin (sistem üyelerinin) gruplandırma ilişkisi, `IfcRelAssignsToGroup` aracılığıyla yönetilir. |

*Not: Bu parametreler IFC'de ilişkiler yoluyla kurgulanmaktadır.*

*[PDF sayfa 68]*

## Tablo 6.29 `IfcElectricAppliance` Sınıfı Zorunlu Öznitelik ve Özellik Listesi

**Zorunlu Öznitelik ve Özellik Listesi**

| Elektrikli Cihaz (`IfcElectricAppliance`) | Ön Tanımlı Tip Listesi |
|---|---|
| Tanım: Elektrikli cihaz, elektrikle çalışan ve tüketici kullanımına yönelik bir aygıttır. | DISH WASHER, ELECTRIC COOKER, FREE STANDING ELECTRIC HEATER, FREE STANDING FAN, FREE STANDING WATER COOLER, FREE STANDING WATER HEATER, FREEZER, FRIDGE FREEZER, HAND DRYER, KITCHEN MACHINE, MICROWAVE, PHOTOCOPIER, REFRIGERATOR, TUMBLE DRYER, VENDING MACHINE, WASHING MACHINE, USERDEFINED, NOTDEFINED |

| Türkçe Tanım (Bilgi Amaçlı) | Öznitelik | İlişki | Veri Tipi (Bilgi Amaçlı) | Açıklama |
|---|---|---|---|---|
| Ön Tanımlı Tip | `PredefinedType` | - | Metin | Ön Tanımlı Tip. |
| İsim | Attribute: Name | - | Metin | IFC uyumlu yazılım sistemleri veya kullanıcılar tarafından kullanılmak üzere isteğe bağlı bir ad. [[`IfcRoot`]] sınıfının bazı alt tipleri için İsim niteliğinin eklenmesi gerekebilir. Bu, bir "where" kuralı ile zorunlu hale getirilecektir. (EK-5'e göre isimlendirilecektir) |
| Etiket | Attribute: Tag | - | Sayı veya Metin | Bir ürünün belirli bir örneğindeki etiket tanımlayıcısı, örneğin seri numarası veya konum numarasıdır. Bu, yerleştirilmiş örnek düzeyindeki tanımlayıcıdır. |

*[PDF sayfa 69]*

## Tablo 6.30 `IfcElectricGenerator` Sınıfı Zorunlu Öznitelik ve Özellik Listesi

**Zorunlu Öznitelik ve Özellik Listesi**

| Elektrik Jeneratörü (`IfcElectricGenerator`) | Ön Tanımlı Tip Listesi |
|---|---|
| Tanım: Elektrik jeneratörü, mekanik enerjiyi elektrik enerjisine dönüştüren bir makine olan bir motordur. | CHP, ENGINEGENERATOR, STANDALONE, USERDEFINED, NOTDEFINED |

| Türkçe Tanım (Bilgi Amaçlı) | Öznitelik | İlişki | Veri Tipi (Bilgi Amaçlı) | Açıklama |
|---|---|---|---|---|
| Ön Tanımlı Tip | `PredefinedType` | - | Metin | Ön Tanımlı Tip. |
| İsim | Attribute: Name | - | Metin | IFC uyumlu yazılım sistemleri veya kullanıcılar tarafından kullanılmak üzere isteğe bağlı bir ad. [[`IfcRoot`]] sınıfının bazı alt tipleri için İsim niteliğinin eklenmesi gerekebilir. Bu, bir "where" kuralı ile zorunlu hale getirilecektir. (EK-5'e göre isimlendirilecektir) |
| Etiket | Attribute: Tag | - | Sayı veya Metin | Bir ürünün belirli bir örneğindeki etiket tanımlayıcısı, örneğin seri numarası veya konum numarasıdır. Bu, yerleştirilmiş örnek düzeyindeki tanımlayıcıdır. |
| Sistem | `IfcSystem` | `IfcRelAssignsToGroup>IfcSystem` | Metin | Bir sistem, Mimarlık, Mühendislik ve Yapım ürünü içerisinde, ortak bir amaç, işlev ya da hizmet sunmak üzere bir araya getirilmiş ilişkili parçalardan oluşan düzenli bir kombinasyondur. Sistem esasen, işlevsel olarak ilişkili ürünler grubudur. Bir veya birden fazla `IfcProduct` örneğinin (sistem üyelerinin) gruplandırma ilişkisi, `IfcRelAssignsToGroup` aracılığıyla yönetilir. |

*Not: Bu parametreler IFC'de ilişkiler yoluyla kurgulanmaktadır.*

*[PDF sayfa 70]*

## Tablo 6.31 `IfcEvaporator` Sınıfı Zorunlu Öznitelik ve Özellik Listesi

**Zorunlu Öznitelik ve Özellik Listesi**

| Evaporatör (`IfcEvaporator`) | Ön Tanımlı Tip Listesi |
|---|---|
| Tanım: Evaporatör, sıvı haldeki bir soğutucu akışkanın buharlaştığı ve çevresindeki akışkandan ısı emdiği bir cihazdır. | DIRECT EXPANSION, DIRECT EXPANSION SHELL AND TUBE, DIRECT EXPANSION TUBE IN TUBE, DIRECT EXPANSION BRAZED PLATE, FLOODED SHELL AND TUBE, SHELL AND COIL, USERDEFINED, NOTDEFINED |

| Türkçe Tanım (Bilgi Amaçlı) | Öznitelik | İlişki | Veri Tipi (Bilgi Amaçlı) | Açıklama |
|---|---|---|---|---|
| Ön Tanımlı Tip | `PredefinedType` | - | Metin | Ön Tanımlı Tip. |
| İsim | Attribute: Name | - | Metin | IFC uyumlu yazılı sistemleri veya kullanıcılar tarafından kullanılmak üzere isteğe bağlı bir ad. [[`IfcRoot`]] sınıfının bazı alt tipleri için İsim niteliğinin eklenmesi gerekebilir. Bu, bir "where" kuralı ile zorunlu hale getirilecektir. (EK-5'e göre isimlendirilecektir) |
| Etiket | Attribute: Tag | - | Sayı veya Metin | Bir ürünün belirli bir örneğindeki etiket tanımlayıcısı, örneğin seri numarası veya konun numarasıdır. Bu, yerleştirilmiş örnek düzeyindeki tanımlayıcıdır. |
| Sistem | `IfcSystem` | `IfcRelAssignsToGroup>IfcSystem` | Metin | Bir sistem, Mimarlık, Mühendislik ve Yapım ürünü içerisinde, ortak bir amaç, işlev ya da hizmet sunmak üzere bir araya getirilmiş ilişkili parçalardan oluşan düzenli bir kombinasyondur. Sistem esasen, işlevsel olarak ilişkili ürünler grubudur. Bir veya birden fazla `IfcProduct` örneğinin (sistem üyelerinin) gruplandırma ilişkisi, `IfcRelAssignsToGroup` aracılığıyla yönetilir. |

| Türkçe Tanım (Bilgi Amaçlı) | Özellik | Özellik Seti / İlişki | Veri Tipi (Bilgi Amaçlı) | Açıklama |
|---|---|---|---|---|
| Nominal Güç Tüketimi | `NominalPowerConsumption` | `Pset_ElectricalDeviceCommon` | Sayı | Nominal toplam güç tüketimi. |

*Not: Bu parametreler IFC'de ilişkiler yoluyla kurgulanmaktadır.*

*[PDF sayfa 71]*

## Tablo 6.32 `IfcFan` Sınıfı Zorunlu Öznitelik ve Özellik Listesi

**Zorunlu Öznitelik ve Özellik Listesi**

| Fan (`IfcFan`) | Ön Tanımlı Tip Listesi |
|---|---|
| Tanım: Fan, bir gaza mekanik iş uygulayan bir cihazdır. Bir fanın tipik kullanımı, bina servisleri hava dağıtım sisteminde hava akışı sağlamaktır. | CENTRIFUGAL AIR FOIL, CENTRIFUGAL BACKWARDI NCLINED CURVED, CENTRIFUGAL FORWARD CURVED, CENTRIFUGAL RADIAL, PROPELLOR AXIAL, TUBE AXIAL, VANE AXIAL, USERDEFINED, NOTDEFINED |

| Türkçe Tanım (Bilgi Amaçlı) | Öznitelik | İlişki | Veri Tipi (Bilgi Amaçlı) | Açıklama |
|---|---|---|---|---|
| Ön Tanımlı Tip | `PredefinedType` | - | Metin | Ön Tanımlı Tip. |
| İsim | Attribute: Name | - | Metin | IFC uyumlu yazılım sistemleri veya kullanıcılar tarafından kullanılmak üzere isteğe bağlı bir ad. [[`IfcRoot`]] sınıfının bazı alt tipleri için İsim niteliğinin eklenmesi gerekebilir. Bu, bir "where" kuralı ile zorunlu hale getirilecektir. (EK- 5'e göre isimlendirilecektir) |
| Etiket | Attribute: Tag | - | Sayı veya Metin | Bir ürünün belirli bir örneğindeki etiket tanımlayıcısı, örneğin seri numarası veya konum numarasıdır. Bu, yerleştirilmiş örnek düzeyindeki tanımlayıcıdır. |
| Sistem | `IfcSystem` | `IfcRelAssignsToGroup>IfcSystem` | Metin | Bir sistem, Mimarlık, Mühendislik ve Yapım ürünü içerisinde, ortak bir amaç, işlev ya da hizmet sunmak üzere bir araya getirilmiş ilişkili parçalardan oluşan düzenli bir kombinasyondur. Sistem esasen, işlevsel olarak ilişkili ürünler grubudur. Bir veya birden fazla `IfcProduct` örneğinin (sistem üyelerinin) gruplandırma ilişkisi, `IfcRelAssignsToGroup` aracılığıyla yönetilir. |

| Türkçe Tanım (Bilgi Amaçlı) | Özellik | Özellik Seti / İlişki | Veri Tipi (Bilgi Amaçlı) | Açıklama |
|---|---|---|---|---|
| Nominal Toplam Basınç | `NominalTotalPressure` | `Pset_FanTypeCommon` | Sayı | Fan boyunca nominal toplam basınç artışı. |
| Nominal Hava Debisi | `NominalAirFlowRate` | `Pset_FanTypeCommon` | Sayı | Nominal hava debisi. |

*Not: Bu parametreler IFC'de ilişkiler yoluyla kurgulanmaktadır.*

*[PDF sayfa 72]*

## Tablo 6.33 `IfcFireSuppressionTerminal` Sınıfı Zorunlu Öznitelik ve Özellik Listesi

**Zorunlu Öznitelik ve Özellik Listesi**

| Yangın Söndürme Terminali (`IfcFireSuppressionTerminal`) | Ön Tanımlı Tip Listesi |
|---|---|
| Tanım: Yangın söndürme terminali, bir yangını söndürecek akışkanı (gaz veya sıvı) iletme amacına sahiptir. Yangın söndürme terminali; bir boru sistemiyle bağlantılı olan ve yangını söndürme görevini yerine getirmek üzere tasarlanan tüm sprinkler, dağıtıcı ve diğer terminal biçimlerini kapsar. | BREECHINGI NLET, FIRE HYDRANT, FIRE MONITOR, HOSEREEL, SPRINKLER, USERDEFINED, NOTDEFINED |

| Türkçe Tanım (Bilgi Amaçlı) | Öznitelik | İlişki | Veri Tipi (Bilgi Amaçlı) | Açıklama |
|---|---|---|---|---|
| Ön Tanımlı Tip | `PredefinedType` | - | Metin | Ön Tanımlı Tip. |
| İsim | Attribute: Name | - | Metin | IFC uyumlu yazılım sistemleri veya kullanıcılar tarafından kullanılmak üzere isteğe bağlı bir ad. [[`IfcRoot`]] sınıfının bazı alt tipleri için İsim niteliğinin eklenmesi gerekebilir. Bu, bir "where" kuralı ile zorunlu hale getirilecektir. (EK-5'e göre isimlendirilecektir) |
| Etiket | Attribute: Tag | - | Sayı veya Metin | Bir ürünün belirli bir örneğindeki etiket tanımlayıcısı, örneğin seri numarası veya konum numarasıdır. Bu, yerleştirilmiş örnek düzeyindeki tanımlayıcıdır. |
| Sistem | `IfcSystem` | `IfcRelAssignsToGroup>IfcSystem` | Metin | Bir sistem, Mimarlık, Mühendislik ve Yapım ürünü içerisinde, ortak bir amaç, işlev ya da hizmet sunmak üzere bir araya getirilmiş ilişkili parçalardan oluşan düzenli bir kombinasyondur. Sistem esasen, işlevsel olarak ilişkili ürünler grubudur. Bir veya birden fazla `IfcProduct` örneğinin (sistem üyelerinin) gruplandırma ilişkisi, `IfcRelAssignsToGroup` aracılığıyla yönetilir. |

*Not: Bu parametreler IFC'de ilişkiler yoluyla kurgulanmaktadır.*
