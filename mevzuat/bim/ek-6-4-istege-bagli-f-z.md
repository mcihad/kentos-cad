<!-- Kaynak: Resmî Gazete 5.8.2026/33331, Dijital Hazırlama Yönetmeliği ekleri (EK-1…EK-9), PDF s. 165–220. Taranmış sayfalardan OCR ile okundu; yöntem, doğrulama kapsamı ve sınırlar için README.md. -->

# EK-6 (devam) — isteğe bağlı, Tablo 6.101–6.135

*[PDF sayfa 165]*

## Tablo 6.101 `IfcFireSuppressionTerminal` Sınıfı İsteğe Bağlı Kullanılabilecek Öznitelik ve Özellik Listesi

| Yangın Söndürme Terminali (`IfcFireSuppressionTerminal`) | Ön Tanımlı Tip Listesi |
|---|---|
| Tanım: Yangın söndürme terminali, bir yangını söndürecek akışkanı (gaz veya sıvı) iletme amacına sahiptir. Yangın söndürme terminali; bir boru sistemiyle bağlantılı olan ve yangını söndürme görevini yerine getirmek üzere tasarlanan tüm sprinkler, dağıtıcı ve diğer terminal biçimlerini kapsar. | BREECHINGI NLET, FIRE HYDRANT, FIRE MONITOR, HOSEREEL, SPRINKLER, USERDEFINED, NOTDEFINED |

| Türkçe Tanım (Bilgi Amaçlı) | Öznitelik | İlişki | Veri Tipi (Bilgi Amaçlı) | Açıklama |
|---|---|---|---|---|
| Nesne Tipi | Attribute: `ObjectType` | - | Metin | Tip, nesneyi daha ayrıntılı tanımlayan özel bir tipi belirtir. Kullanım, örneklenebilir somut alt tiplerin seviyesinde belirlenmelidir. Şu durumlarda kullanıcı tarafından tanımlanan tip bilgisi bu özniteliğe Tanımlı Tip özniteliğinin değeri listeden [[USERDEFINED]] olarak seçilmişse veya 2) somut örneklenmiş varlıkta bir Ön Tanımlı Tip özniteliği yoksa. İkinci durum, bazı istisnai yaprak sınıflarında ve [[IfcBuiltElement]]'i doğrudan örneklendirirken geçerlidir. |
| Açıklama | Attribute: Description | - | Metin | Bilgilendirme amaçlı isteğe bağlı olarak verilen bir tanımdır. |

| Türkçe Tanım (Bilgi Amaçlı) | Özellik | Özellik Seti / İlişki | Veri Tipi (Bilgi Amaçlı) | Açıklama |
|---|---|---|---|---|
|  | `CSBPozNo` | `TREpys_PozOzellikSeti` | Metin | Çevre, Şehircilik ve İklim Değişikliği Bakanlığı İnşaat ve Tesisat Birim Fiyat Poz No'su. |
| Brüt Ağırlık | `GrossWeight` | `Qto_FireSuppressionTerminalBaseQuantities` | Sayı | Nesnenin toplam brüt ağırlığı, ek parçalar olmaksızın ve olası imalat detayları (kesit boşluğu vb.) veya açıklıklar ve girintiler dikkate alınmadan hesaplanır. |

*[PDF sayfa 166]*

## Tablo 6.102 `IfcFooting` Sınıfı İsteğe Bağlı Kullanılabilecek Öznitelik ve Özellik Listesi

| Temel (`IfcFooting`) | Ön Tanımlı Tip Listesi |
|---|---|
| Tanım: Temel pabucu, bir yapının yükünü yayarak toprağa ileten bir yapı parçasıdır. Temel pabucu, yüklerin yüzeye yakın bir alana aktarıldığı sığ temel olarak da tanımlanır. | CAISSON FOUNDATION, FOOTING BEAM, PAD FOOTING, PILE CAP, STRIP FOOTING, USERDEFINED, NOTDEFINED |

| Türkçe Tanım (Bilgi Amaçlı) | Öznitelik | İlişki | Veri Tipi (Bilgi Amaçlı) | Açıklama |
|---|---|---|---|---|
| Nesne Tipi | Attribute: `ObjectType` | - | Metin | Tip, nesneyi daha ayrıntılı tanımlayan özel bir tipi belirtir. Kullanım, örneklenebilir somut alt tiplerin seviyesinde belirlenmelidir. Şu durumlarda kullanıcı tarafından tanımlanan tip bilgisi bu özniteliğe girilir: 1) Ön Tanımlı Tip özniteliğinin değeri listeden [[USERDEFINED]] olarak seçilmişse veya 2) somut örneklenmiş varlıkta bir Ön Tanımlı Tip özniteliği yoksa. İkinci durum, bazı istisnai yaprak sınıflarında ve [[IfcBuiltElement]]'i doğrudan örneklendirirken geçerlidir. |
| Açıklama | Attribute: Description | - | Metin | Bilgilendirme amaçlı isteğe bağlı olarak verilen bir tanımdır. |

| Türkçe Tanım (Bilgi Amaçlı) | Özellik | Özellik Seti / İlişki | Veri Tipi (Bilgi Amaçlı) | Açıklama |
|---|---|---|---|---|
|  | `CSBPozNo` | `TREpys_PozOzellikSeti` | Metin | Çevre, Şehircilik ve İklim Değişikliği Bakanlığı İnşaat ve Tesisat Birim Fiyat Poz No'su. |
| Yük Taşıyan | `LoadBearing` | `Pset_FootingCommon` | Mantıksal | Nesnenin yük taşıma amacıyla tasarlanıp tasarlanmadığını belirtir (DOĞRU) veya tasarlanmadığını (YANLIŞ). |
| Uzunluk | Length | `Qto_FootingBaseQuantities` | Sayı | Nesnenin uzunluğu. Kesit boşlukları gibi imalat özellikleri dikkate alınmaz. Şerit temeller için yol boyunca ölçülür, diğer temeller için ise yatay ölçülerden biridir. Sadece sabitse verilmelidir. |
| Genişlik | Width | `Qto_FootingBaseQuantities` | Sayı | Nesnenin genişliği. Sadece, nesne sabit kalınlığa (prizmatik) sahipse verilir.<br>Şerit temeller için, temel hattına (veya boyuna eksene) dik olarak ölçülür. Diğer temeller için, yatay boyutlardan biridir. Yalnızca sabit olduğu durumda verilmelidir. |
| Yükseklik | Height | `Qto_FootingBaseQuantities` | Sayı | Karakteristik yükseklik.<br>Temelin toplam nominal yüksekliği. Yalnızca sabitse verilmelidir. |

*[PDF sayfa 167]*

*(Tablo önceki sayfadan sürüyor; sütun başlıkları okunabilirlik için yinelendi.)*

| Türkçe Tanım (Bilgi Amaçlı) | Özellik | Özellik Seti / İlişki | Veri Tipi (Bilgi Amaçlı) | Açıklama |
|---|---|---|---|---|
| Brüt Ağırlık | `GrossWeight` | `Qto_FootingBaseQuantities` | Sayı | Nesnenin toplam brüt ağırlığı, ek parçalar olmaksızın ve olası imalat detayları (kesit boşluğu vb.) veya açıklıklar ve girintiler dikkate alınmadan hesaplanır. |
| Net Ağırlık | `NetWeight` | `Qto_FootingBaseQuantities` | Sayı | Nesnenin toplam net ağırlığı, ek parçalar olmaksızın ve olası imalat detayları (kesit boşluğu vb.) veya açıklıklar ve girintiler dikkate alınmadan hesaplanır. |
| Beton Örtüsü | `ConcreteCover` | `Pset_ConcreteElementGeneral` | Sayı | Yerel inşaat yönetmeliklerine göre donatı çubukları üzerindeki beton örtüsü (Türkiye Bina Deprem Yönetmeliğine göre beton örtüsü, boyuna donatı ağırlık merkezi ile en dış beton lifi arasında kalan uzaklıktır) |
| Donatı Dayanım Sınıfı | `ReinforcementStrengthClass` | `Pset_ConcreteElementGeneral` | Metin | Projenin uygulanmakta olduğu beton tasarım standardına göre donatı dayanımının sınıflandırılması. Donatı dayanım sınıfı genellikle dayanım ve deformasyon özelliklerini birleştirir. |
| Kesit Alanı | `CrossSectionArea` | `Qto_FootingBaseQuantities` | Sayı | Nesnenin kesitinin (veya profilinin) toplam alanı. |
| Dış Yüzey Alanı | `OuterSurfaceArea` | `Qto_FootingBaseQuantities` | Sayı | Nesnenin yüzeylerinin toplam alanı (uç kesit alanları dikkate alınmaksızın), genellikle ekstrüzyon durumunda çevre \* uzunluk olarak hesaplanır. |
| Brüt Yüzey Alanı | `GrossSurfaceArea` | `Qto_FootingBaseQuantities` | Sayı | Nesnenin toplam brüt alanı, genellikle çevre \* uzunluk + 2 \* Kesit Alanı olarak hesaplanır. Bu, Dış Yüzey Alanı + (2 x Kesit Alanı) toplamıdır ve yalnızca Dış Yüzey Alanı ve Kesit Alanı ayrı olarak belirlenemediğinde verilecektir. |
| Brüt Hacim | `GrossVolume` | `Qto_FootingBaseQuantities` | Sayı | Nesnenin toplam brüt hacmi. Açıklıklar, girintiler, kapalı nesneler ve çıkıntılar dikkate alınmaz. |
| Net Hacim | `NetVolume` | `Qto_FootingBaseQuantities` | Sayı | Nesnenin toplam net hacmi, olası imalat özellikleri (kesit boşluklarıvb.) veya açıklıklar ve girintiler dikkate alınarak hesaplanır. |

*[PDF sayfa 168]*

## Tablo 6.103 `IfcFurniture` Sınıfı İsteğe Bağlı Kullanılabilecek Öznitelik ve Özellik Listesi

| Mobilya (`IfcFurniture`) | Ön Tanımlı Tip Listesi |
|---|---|
| Tanım: Mobilya, binanın yapısına sabitlenmiş ya da sabitlememiş masa, sandalye, dolap gibi tefriş elemanlarını ifade eder. | BED, CHAIR, DESK, FILE CABINET, SHELF, SOFA, TABLE, TECHNICAL CABINET, USERDEFINED, NOTDEFINED |

| Türkçe Tanım (Bilgi Amaçlı) | Öznitelik | İlişki | Veri Tipi (Bilgi Amaçlı) | Açıklama |
|---|---|---|---|---|
| Nesne Tipi | Attribute: `ObjectType` | - | Metin | Tip, nesneyi daha ayrıntılı tanımlayan özel bir tipi belirtir. Kullanım, örneklenebilir somut alt tiplerin seviyesinde belirlenmelidir. Şu durumlarda kullanıcı tarafından tanımlanan tip bilgisi bu özniteliğe girilir: 1) Ön Tanımlı Tip özniteliğinin değeri listeden [[USERDEFINED]] olarak seçilmişse veya 2) somut örneklenmiş varlıkta bir Ön Tanımlı Tip özniteliği yoksa. İkinci durum, bazı istisnai yaprak sınıflarında ve [[IfcBuiltElement]]'i doğrudan örneklendirirken geçerlidir. |
| Açıklama | Attribute: Description | - | Metin | Bilgilendirme amaçlı isteğe bağlı olarak verilen bir tanımdır. |

| Türkçe Tanım (Bilgi Amaçlı) | Özellik | Özellik Seti / İlişki | Veri Tipi (Bilgi Amaçlı) | Açıklama |
|---|---|---|---|---|
|  | `CSBPozNo` | `TREpys_PozOzellikSeti` | Metin | Çevre, Şehircilik ve İklim Değişikliği Bakanlığı İnşaat ve Tesisat Birim Fiyat Poz No'su. |

*[PDF sayfa 169]*

## Tablo 6.104 `IfcGroup` Sınıfı İsteğe Bağlı Kullanılabilecek Öznitelik ve Özellik Listesi

| Grup (`IfcGroup`) | Ön Tanımlı Tip Listesi |
|---|---|
| Tanım: `IfcGroup`, herhangi bir nesne grubunun genel bir temsilidir. Grup, nesnelerin mantıksal olarak bir araya getirildiği bir topluluktur. Grup nesnesinin kendisine ait bir konumu bulunmaz ve geometrik bir temsil içermez. Bu nedenle grup, geometrik veya topolojik özelliklere dayanmayan bir ilişkilendirme kapsamında değerlendirilir. | - |

| Türkçe Tanım (Bilgi Amaçlı) | Öznitelik | İlişki | Veri Tipi (Bilgi Amaçlı) | Açıklama |
|---|---|---|---|---|
| Nesne Tipi | Attribute: `ObjectType` | - | Metin | Tip, nesneyi daha ayrıntılı tanımlayan özel bir tipi belirtir. Kullanım, örneklenebilir somut alt tiplerin seviyesinde belirlenmelidir. Şu durumlarda kullanıcı tarafından tanımlanan tip bilgisi bu özniteliğe girilir: Ön Tanımlı Tip özniteliğinin değeri listeden [[USERDEFINED]] olarak seçilmişse veya 2) somut örneklenmiş varlıkta bir Ön Tanımlı Tip özniteliği yoksa. İkinci durum, bazı istisnai yaprak sınıflarında ve [[IfcBuiltElement]]'i doğrudan örneklendirirken geçerlidir. |
| Açıklama | Attribute: Description | - | Metin | Bilgilendirme amaçlı isteğe bağlı olarak verilen bir tanımdır. |

*[PDF sayfa 170]*

## Tablo 6.105 `IfcHeatExchanger` Sınıfı İsteğe Bağlı Kullanılabilecek Öznitelik ve Özellik Listesi

| Isı Eşanjörü (`IfcHeatExchanger`) | Ön Tanımlı Tip Listesi |
|---|---|
| Tanım: Isı Eşanjörü, plaka veya gövde-boru tipi eşanjörler gibi, birbirine karışmayan ortamlar arasında ısı transferi sağlamak için kullanılan bir cihazdır. | PLATE, SHELLANDTUBE, USERDEFINED, NOTDEFINED |

| Türkçe Tanım (Bilgi Amaçlı) | Öznitelik | İlişki | Veri Tipi (Bilgi Amaçlı) | Açıklama |
|---|---|---|---|---|
| Nesne Tipi | Attribute: `ObjectType` | - | Metin | Tip, nesneyi daha ayrıntılı tanımlayan özel bir tipi belirtir. Kullanım, örneklenebilir somut alt tiplerin seviyesinde belirlenmelidir. Şu durumlarda kullanıcı tarafından tanımlanan tip bilgisi bu özniteliğe girilir: 1) Ön Tanımlı Tip özniteliğinin değeri listeden [[USERDEFINED]] olarak seçilmişse veya 2) somut örneklenmiş varlıkta bir Ön Tanımlı Tip özniteliği yoksa. İkinci durum, bazı istisnai yaprak sınıflarında ve [[IfcBuiltElement]]'i doğrudan örneklendirirken geçerlidir. |
| Açıklama | Attribute: Description | - | Metin | Bilgilendirme amaçlı isteğe bağlı olarak verilen bir tanımdır. |

| Türkçe Tanım (Bilgi Amaçlı) | Özellik | Özellik Seti / İlişki | Veri Tipi (Bilgi Amaçlı) | Açıklama |
|---|---|---|---|---|
|  | `CSBPozNo` | `TREpys_PozOzellikSeti` | Metin | Çevre, Şehircilik ve İklim Değişikliği Bakanlığı İnşaat ve Tesisat Birim Fiyat Poz No'su. |
| Akış Düzeni | `FlowArrangement` | `Pset_HeatExchangerTypeCommon` | Metin | Isı eşanjörü veya soğutma kulesi için temel akış düzenlemelerini tanımlar: Ters Akış - hava ve su akışı farklı yönlerden girer. Çapraz Akış - hava ve su akışı birbirine dik yönde hareket eder. Paralel Akış - hava ve su akışı aynı yönde girer. Çok Geçişli Akış - birden fazla geçişe sahip ısı eşanjörü düzenlemesidir. Diğer - yukarıda tanımlanmayan diğer ısı eşanjörü akış düzenlemelerini ifade eder. |
| Nominal Yükseklik | `NominalHeight` | `Pset_ElementSize` | Sayı | Nesnenin nominal yüksekliği. Boyut bilgisi, şekil temsiline ve kullanılan geometrik parametrelere ek olarak verilir. Ekli özellik setinde sağlanan geometrik parametreler ile boyut özellikleri arasında tutarsızlık olması durumunda, geometrik parametreler dikkate alınır. |
| Nominal Uzunluk | `NominalLength` | `Pset_ElementSize` | Sayı | Nesnenin toplam nominal uzunluğu. Boyut bilgisi, şekil temsiline ve kullanılan geometrik parametrelere ek olarak verilir. Ekli özellik setinde sağlanan geometrik parametreler ile boyut özellikleri arasında tutarsızlık olması durumunda, geometrik parametreler dikkate alınır. |

*[PDF sayfa 171]*

*(Tablo önceki sayfadan sürüyor; sütun başlıkları okunabilirlik için yinelendi.)*

| Türkçe Tanım (Bilgi Amaçlı) | Özellik | Özellik Seti / İlişki | Veri Tipi (Bilgi Amaçlı) | Açıklama |
|---|---|---|---|---|
| Nominal Genişlik | `NominalWidth` | `Pset_ElementSize` | Sayı | Nesnenin toplam nominal genişliği. Boyut bilgisi, şekil temsiline vel kullanılan geometrik parametrelere ek olarak verilir. Ekli özellik setinde sağlanan geometrik parametreler ile boyut özellikleri arasında tutarsızlık olması durumunda, geometrik parametreler dikkate alınır. |
| Brüt Ağırlık | `GrossWeight` | `Qto_HeatExchangerBaseQuantities` | Sayı | Nesnenin toplam brüt ağırlığı, ek parçalar olmaksızın ve olası imalat alınmadan hesaplanır.<br>detayları (kesit boşluğu vb.) veya açıklıklar ve girintiler dikkate |

*[PDF sayfa 172]*

## Tablo 6.106 `IfcLightFixture` Sınıf İsteğe Bağlı Kullanılabilecek Öznitelik ve Özellik Listesi

| Aydınlatma Armatürü (`IfcLightFixture`) | Ön Tanımlı Tip Listesi |
|---|---|
| Tanım: Aydınlatma Armatürü, bir veya daha fazla lambayı ve isteğe bağlı olarak bu lambaların ışık yayımını kontrol eden, sınırlayan veya değiştiren cihazları barındırmak amacıyla tasarlanmış bir muhafazadır. | DIRECTION SOURCE, POINT SOURCE, SECURITY LIGHTING, USERDEFINED, NOTDEFINED |

| Türkçe Tanım (Bilgi Amaçlı) | Öznitelik | İlişki | Veri Tipi (Bilgi Amaçlı) | Açıklama |
|---|---|---|---|---|
| Nesne Tipi | Attribute: `ObjectType` | - | Metin | Tip, nesneyi daha ayrıntılı tanımlayan özel bir tipi belirtir. Kullanım, örneklenebilir somut alt tiplerin seviyesinde belirlenmelidir. Şu durumlarda kullanıcı tarafından tanımlanan tip bilgisi bu özniteliğe girilir: 1) Ön Tanımlı Tip özniteliğinin değeri listeden [[USERDEFINED]] olarak seçilmişse veya 2) somut örneklenmiş varlıkta bir Ön Tanımlı Tip özniteliği yoksa. İkinci durum, bazı istisnai yaprak sınıflarında ve [[IfcBuiltElement]]'i doğrudan örneklendirirken geçerlidir. |
| Açıklama | Attribute: Description | - | Metin | Bilgilendirme amaçlı isteğe bağlı olarak verilen bir tanımdır. |

| Türkçe Tanım (Bilgi Amaçlı) | Özellik | Özellik Seti / İlişki | Veri Tipi (Bilgi Amaçlı) | Açıklama |
|---|---|---|---|---|
|  | `CSBPozNo` | `TREpys_PozOzellikSeti` | Metin | Çevre, Şehircilik ve İklim Değişikliği Bakanlığı İnşaat ve Tesisat Birim Fiyat Poz No'su. |
| Toplam Güç | `TotalWattage` | `Pset_LightFixtureTypeCommon` | Number | Tüm ışık kaynakları takılı haldeyken aydınlatma armatürünün toplam güç tüketimi (watt değeri). |
| Nominal Akım | `RatedCurrent` | `Pset_ElectricalDeviceCommon` | Sayı | Cihazın, tasarımına göre taşıyabileceği akım. |
| Nominal Voltaj | `RatedVoltage` | `Pset_ElectricalDeviceCommon` | Sayı | Bir cihazın (çalışmak üzere) sertifikalandırıldığı izin verilen voltaj aralığı. Bu değerin üst sınırı maksimum kabul edilir. |
| Kaynak Sayısı | `NumberOfSources` | `Pset_LightFixtureTypeCommon` | Sayı | Kaynak sayısı. |
| Aydınlatma Armatürü Montaj Tipi | `LightFixtureMountingType` | `Pset_LightFixtureTypeCommon` | Metin | Aydınlatma armatürleri için mevcut montaj tiplerinden, ihtiyaç duyulanın seçilebileceği bir liste. |

*[PDF sayfa 173]*

*(Tablo önceki sayfadan sürüyor; sütun başlıkları okunabilirlik için yinelendi.)*

| Türkçe Tanım (Bilgi Amaçlı) | Özellik | Özellik Seti / İlişki | Veri Tipi (Bilgi Amaçlı) | Açıklama |
|---|---|---|---|---|
| Aydınlatma Armatürü Yerleştirmetipi | `LightFixturePlacingType` | `Pset_LightFixtureTypeCommon` | Metin | Aydınlatma armatürleri için mevcut yerleştirme tiplerinden, ihtiyaç duyulanın seçilebileceği bir liste. |
| Bakım Faktörü | `MaintenanceFactor` | `Pset_LightFixtureTypeCommon` | Tamsayı | Kir, duman veya eskime nedeniyle lambaların ve yansıtıcı ekipmanların ilk değerlerinden düşüşü için yapılan aritmetik düzeltme. |
| Maksimum Plenum Duyulur Isıl Yükü | `MaximumPlenumSensibleLoad` | `Pset_LightFixtureTypeCommon` | Sayı | Aydınlatma armatürünün dönüş havası plenumuna aktardığı maksimum veya pik duyulur ısıl yük. |
| Maksimum Hacim Duyulur Isıl Yükü | `MaximumSpaceSensibleLoad` | `Pset_LightFixtureTypeCommon` | Sayı | Armatürün iklimlendirilmiş mahal içine aktardığı: maksimum veya pik duyulur ısıl yük. |
| Duyulur yükün Işınıma Oranı | `SensibleLoadToRadiant` | `Pset_LightFixtureTypeCommon` | Tamsayı | Duyulur ısıl yükün radyant ısıya olan yüzdesi. |
| Hat Sayısı | `NumberOfPoles` | `Pset_ElectricalDeviceCommon` | Sayı | Nesnenin etkileyeceği kutup sayısı.<br>Cihazın işlemeye/iletmeye yönelik tasarlandığı enerjili hat (faz) sayısı. |
| Koruyucu Topraklama Bağlantısı Mevcut | `HasProtectiveEarth` | `Pset_ElectricalDeviceCommon` | Mantıksal | Nesnenin koruyucu topraklama bağlantısına sahip olduğunu (DOĞRU) veya olmadığını (=YANLIŞ) gösterir. |
| Brüt Ağırlık | `GrossWeight` | `Qto_LightFixtureBaseQuantities` | Sayı | Nesnenin toplam brüt ağırlığı, ek parçalar olmaksızın ve olası imalat detayları (kesit boşluğu vb.) veya açıklıklar ve girintiler dikkate alınmadan hesaplanır. |

*[PDF sayfa 174]*

## Tablo 6.107 `IfcMember` Sınıfı İsteğe Bağlı Kullanılabilecek Öznitelik ve Özellik Listesi

| Eleman (`IfcMember`) | Ön Tanımlı Tip Listesi |
|---|---|
| Tanım: [[`IfcMember`]], destek noktaları arasında veya ötesinde yük taşımak üzere tasarlanmış yapısal bir elemandır. Taşıyıcı olması zorunlu değildir. Elemanın yönelimi (yatay, düşey veya eğimli olması); tanımı açısından önemli değildir ([[`IfcBeam`]] ve [[IfcColumn]]'un aksine). [[`IfcMember`]], mimari veya yapısal modelleme bakış açısından doğrusal bir yapısal elemanı temsil eder ve eğer daha özel; olarak [[`ifcBeam`]] veya [[`IfcColumn`]] olarak ifade edilemiyorsa kullanılmalıdır. | ARCH\_SEGMENT, BRACE, CHORD, COLLAR, MEMBER, MULLION, PLATE, POST, PURLIN, RAFTER, STAY\_CABLE, STIFFENING\_RIB, STRINGER, STRUCTURALCABLE, STRUT, STUD, SUSPENDER, SUSPENSION\_CABLE, TIEBAR, USERDEFINED, NOTDEFINED |

| Türkçe Tanım (Bilgi Amaçlı) | Öznitelik | İlişki | Veri Tipi (Bilgi Amaçlı) | Açıklama |
|---|---|---|---|---|
| Nesne Tipi | Attribute: `ObjectType` | - | Metin | Tip, nesneyi daha ayrıntılı tanımlayan özel bir tipi belirtir. Kullanım, örneklenebilir somut alt tiplerin seviyesinde belirlenmelidir. Şu durumlarda kullanıcı tarafından tanımlanan tip bilgisi bu özniteliğe Tanımlı Tip özniteliğinin değeri listeden [[USERDEFINED]] olarak seçilmişse veya 2) somut örneklenmiş varlıkta bir Ön Tanımlı Tip özniteliği yoksa. İkinci durum, bazı istisnai yaprak sınıflarında ve [[IfcBuiltElement]]'i doğrudan örneklendirirken geçerlidir. |
| Açıklama | Attribute: Description | - | Metin | Bilgilendirme amaçlı isteğe bağlı olarak verilen bir tanımdır. |

| Türkçe Tanım (Bilgi Amaçlı) | Özellik | Özellik Seti / İlişki | Veri Tipi (Bilgi Amaçlı) | Açıklama |
|---|---|---|---|---|
|  | `CSBPozNo` | `TREpys_PozOzellikSeti` | Metin | Çevre, Şehircilik ve İklim Değişikliği Bakanlığı İnşaat ve Tesisat Birim Fiyat Poz No'su. |
| Yangın Dayanım Sınıfı | `FireRating` | `Pset_MemberCommon` | Metin | Bu nesnenin yangın dayanım sınıfı. Ulusal yangın güvenliği sınıflandırmasına göre (Türkiye için: Binaların Yangından Korunması Hakkında Yönetmeliğe göre) verilmiştir. |
| Yük Taşıyan | `LoadBearing` | `Pset_MemberCommon` | Mantıksal | Nesnenin yük taşıma amacıyla tasarlanıp tasarlanmadığını belirtir (DOĞRU) veya tasarlanmadığını (YANLIŞ). |
| Uzunluk | Length | `Qto_MemberBaseQuantities` | Sayı | Nesnenin uzunluğu.<br>Kesitler veya delikler gibi işleme özellikleri dikkate alınmaz. |
| Brüt Ağırlık | `GrossWeight` | `Qto_MemberBaseQuantities` | Sayı | Nesnenin toplam brüt ağırlığı, ek parçalar olmaksızın ve olası imalat detayları (kesit boşluğu vb.) veya açıklıklar ve girintiler dikkate alınmadan hesaplanır. |
| Net Ağırlık | `NetWeight` | `Qto_MemberBaseQuantities` | Sayı | Nesnenin toplam net ağırlığı, ek parçalar olmaksızın ve olası imalat detayları (kesit boşluğu vb.) veya açıklıklar ve girintiler dikkate alınmadan hesaplanır. |

*[PDF sayfa 175]*

*(Tablo önceki sayfadan sürüyor; sütun başlıkları okunabilirlik için yinelendi.)*

| Türkçe Tanım (Bilgi Amaçlı) | Özellik | Özellik Seti / İlişki | Veri Tipi (Bilgi Amaçlı) | Açıklama |
|---|---|---|---|---|
| Eğim | Slope | `Pset_MemberCommon` | Sayı | Eğim açisi yataya göre relatif (0,0 derece). Şekil bilgisi, şekil temsilinin yanı sıra kullanılan geometrik parametrelerle birlikte sağlanır. Geometrik parametreler ile şekil özellikleri arasında tutarsızlık olması durumunda, geometrik parametreler dikkate alınır. Geometri düzenleme uygulamaları (ör. CAD) için bu değer yalnızca "yazılabilir" olmalıdır. |
| Kesit Alanı | `CrossSectionArea` | `Qto_MemberBaseQuantities` | Sayı | Nesnenin kesitinin (veya profilinin) toplam alanı. |
| Dış Yüzey Alanı | `OuterSurfaceArea` | `Qto_MemberBaseQuantities` | Sayı | Nesnenin yüzeylerinin toplam alanı (uç kesit alanları dikkate alınmaksızın), genellikle ekstrüzyon durumunda çevre \* uzunluk olarak hesaplanır. |
| Brüt Yüzey Alanı | `GrossSurfaceArea` | `Qto_MemberBaseQuantities` | Sayı | Nesnenin toplam brüt alanı, genellikle çevre \* uzunluk + 2 \* Kesit Alanı olarak hesaplanır. Bu, Dış Yüzey Alanı - (2 x Kesit Alanı) toplamıdır ve yalnızca Dış Yüzey Alanı ve Kesit Alanı ayrı olarak belirlenemediğinde verilecektir. |
| Net Yüzey Alanı | `NetSurfaceArea` | `Qto_MemberBaseQuantities` | Sayı | Nesnenin net yüzey alanı genellikle, olası imalat özelliklerini (kesikler vb.) veya açıklıkları ve girintileri dikkate alarak., çevre \* uzunluk - 2 \* Kesit Alanı olarak hesaplanır. |
| Brüt Hacim | `GrossVolume` | `Qto_MemberBaseQuantities` | Sayı | Nesnenin toplam brüt hacmi. Açıklıklar, girintiler, kapalı nesneler ve çıkıntılar dikkate alınmaz. |
| Net Hacim | `NetVolume` | `Qto_MemberBaseQuantities` | Sayı | Nesnenin toplam net hacmi, olası imalat özelliklerini (kesit boşlukları vb.) veya açıklıklar ve girintiler dikkate alınarak hesaplanır. |

*[PDF sayfa 176]*

## Tablo 6.108 `IfcOpeningElement` Sınıfı İsteğe Bağlı Kullanılabilecek Öznitelik ve Özellik Listesi

| Açıklık Elemanı (`IfcOpeningElement`) | Ön Tanımlı Tip Listesi |
|---|---|
| Tanım: Açıklık elemanı; açıklık, girinti veya tesisat kanalı gibi boşlukları temsil eder. Fiziksel bir yapı elemanı içinde yer alan bir boşluğu ifade eder. Açıklıklar duvarlara, döşemelere, kirişlere, kolonlara veya diğer elemanlara yerleştirilebilir. | OPENING, RECESS, USERDEFINED, NOTDEFINED |

| Türkçe Tanım (Bilgi Amaçlı) | Öznitelik | İlişki | Veri Tipi (Bilgi Amaçlı) | Açıklama |
|---|---|---|---|---|
| Nesne Tipi | Attribute: `ObjectType` | - | Metin | Tip, nesneyi daha ayrıntılı tanımlayan özel bir tipi belirtir. Kullanım, örneklenebilir somut alt tiplerin seviyesinde belirlenmelidir. Şu durumlarda kullanıcı tarafından tanımlanan tip bilgisi bu özniteliğe girilir: 1) Ön Tanımlı Tip özniteliğinin değeri listeden [[USERDEFINED]] olarak seçilmişse veya 2) somut örneklenmiş varlıkta bir Ön Tanımlı Tip özniteliği yoksa. İkinci durum, bazı istisnai yaprak sınıflarında ve [[IfcBuiltElement]]'i doğrudan örneklendirirken geçerlidir. |
| Etiket | Attribute: Tag | - | Sayı veya Metin | Bir ürünün belirli bir örneğindeki etiket tanımlayıcısı, örneğin seri numarası veya konum numarasıdır. Bu, yerleştirilmiş örnek düzeyindeki tanımlayıcıdır. |
| Açıklama | Attribute: Description | - | Metin | Bilgilendirme amaçlı isteğe bağlı olarak verilen bir tanımdır. |

*[PDF sayfa 177]*

## Tablo 6.109 `IfcOutlet` Sınıfı İsteğe Bağlı Kullanılabilecek Öznitelik ve Özellik Listesi

| Priz (`IfcOutlet`) | Ön Tanımlı Tip Listesi |
|---|---|
| Tanım: Priz, elektrik enerjisi veya iletişim için bir veya birden fazla fişin takılmasını sağlamak amacıyla belirli bir noktaya monte edilen bir cihazdır. | AUDIO VISUAL OUTLET, COMMUNICATIONS OUTLET, DATA OUTLET, POWER OUTLET, TELEPHONE OUTLET, USERDEFINED, NOTDEFINED |

| Türkçe Tanım (Bilgi Amaçlı) | Öznitelik | İlişki | Veri Tipi (Bilgi Amaçlı) | Açıklama |
|---|---|---|---|---|
| Nesne Tipi | Attribute: `ObjectType` | - | Metin | Tip, nesneyi daha ayrıntılı tanımlayan özel bir tipi belirtir. Kullanım, örneklenebilir somut alt tiplerin seviyesinde belirlenmelidir. Şu durumlarda kullanıcı tarafından tanımlanan tip bilgisi bu özniteliğe girilir: 1) Ön Tanımlı Tip özniteliğinin değeri listeden [[USERDEFINED]] olarak seçilmişse veya 2) somut örneklenmiş varlıkta bir Ön Tanımlı Tip özniteliği yoksa. İkinci durum, bazı istisnai yaprak sınıflarında ve [[IfcBuiltElement]]'i doğrudan örneklendirirken geçerlidir. |
| Açıklama | Attribute: Description | - | Metin | Bilgilendirme amaçlı isteğe bağlı olarak verilen bir tanımdır. |

| Türkçe Tanım (Bilgi Amaçlı) | Özellik | Özellik Seti / İlişki | Veri Tipi (Bilgi Amaçlı) | Açıklama |
|---|---|---|---|---|
|  | `CSBPozNo` | `TREpys_PozOzellikSeti` | Metin | Çevre, Şehircilik ve İklim Değişikliği Bakanlığı İnşaat ve Tesisat Birim Fiyat Poz No'su. |
| Nominal Güç Tüketimi | `NominalPowerConsumption` | `Pset_ElectricalDeviceCommon` | Sayı | Nominal toplam güç tüketimi. |
| Nominal Akım | `RatedCurrent` | `Pset_ElectricalDeviceCommon` | Sayı | Cihazın, tasarımına göre taşıyabileceği akım. |
| Nominal Voltaj | `RatedVoltage` | `Pset_ElectricalDeviceCommon` | Sayı | Bir cihazın (çalışmak üzere) sertifikalandırıldığı izin verilen voltaj aralığı. Bu değerin üst sınırı maksimum kabul edilir. |
| Tak-çıkar bağlantı | `IsPluggableOutlet` | `Pset_OutletTypeCommon` | Mantıksal | Prizin tak-çıkar bir fiş bağlantısını kabul ettiğini (= DOĞRU) ya da sabit bağlantı olduğunu (= YANLIŞ) veya bağlantı biçiminin henüz belirlenmediğini (= BİLİNMEYEN) gösterir. |
| Priz Sayısı | `NumberOfSockets` | `Pset_OutletTypeCommon` | Sayı | Bağlanabilecek priz (soket) sayısı. Tutarsızlık durumunda, portlar üzerinde tanımlanan prizler dikkate alınır. |
| Brüt Ağırlık | `GrossWeight` | `Qto_OutletBaseQuantities` | Sayı | Nesnenin toplam brüt ağırlığı, ek parçalar olmaksızın ve olası imalat detayları (kesit boşluğu vb.) veya açıklıklar ve girintiler dikkate alınmadan hesaplanır. |

*[PDF sayfa 178]*

## Tablo 6.110 `IfcPavement` Sınıfı İsteğe Bağlı Kullanılabilecek Öznitelik ve Özellik Listesi

| Asfalt (`IfcPavement`) | Ön Tanımlı Tip Listesi |
|---|---|
| Tanım: Yol veya diğer kaplamalı bir alanda, genellikle birden fazla tabakadan oluşan ve araçlar ya da yayalar tarafından gelen yükleri taşıyarak düzgün bir yüzey sağlayan yapı elemanı türüdür. NOT: ISO 6707-1'e göre tanım: yol, pist veya benzeri, taban zemini üzerinde yer alan yapı. | FLEXIBLE, RIGID, USERDEFINED, NOTDEFINED |

| Türkçe Tanım (Bilgi Amaçlı) | Öznitelik | İlişki | Veri Tipi (Bilgi Amaçlı) | Açıklama |
|---|---|---|---|---|
| Nesne Tipi | Attribute: `ObjectType` | - | Metin | Tip, nesneyi daha ayrıntılı tanımlayan özel bir tipi belirtir. Kullanım, örneklenebilir somut alt tiplerin seviyesinde belirlenmelidir. Şu durumlarda kullanıcı tarafından tanımlanan tip bilgisi bu özniteliğe girilir: 1) Ön Tanımlı Tip özniteliğinin değeri listeden [[USERDEFINED]] olarak seçilmişse veya 2) somut örneklenmiş varlıkta bir Ön Tanımlı Tip özniteliği yoksa. İkinci durum, bazı istisnai yaprak sınıflarında ve [[IfcBuiltElement]]'i doğrudan örneklendirirken geçerlidir. |
| Etiket | Attribute: Tag | - | Sayı veya Metin | Bir ürünün belirli bir örneğindeki etiket tanımlayıcısı, örneğin seri numarası veya konum numarasıdır. Bu, yerleştirilmiş örnek düzeyindeki tanımlayıcıdır. |
| Açıklama | Attribute: Description | - | Metin | Bilgilendirme amaçlı isteğe bağlı olarak verilen bir tanımdır. |

| Türkçe Tanım (Bilgi Amaçlı) | Özellik | Özellik Seti / İlişki | Veri Tipi (Bilgi Amaçlı) | Açıklama |
|---|---|---|---|---|
|  | `CSBPozNo` | `TREpys_PozOzellikSeti` | Metin | Çevre, Şehircilik ve İklim Değişikliği Bakanlığı İnşaat ve Tesisat Birim Fiyat Poz No'su. |

*[PDF sayfa 179]*

## Tablo 6.111 `IfcPile` Sınıfı İsteğe Bağlı Kullanılabilecek Öznitelik ve Özellik Listesi

| Kazık (`IfcPile`)) | Ön Tanımlı Tip Listesi |
|---|---|
| Tanım: Bir kazık, bir yükü desteklemek amacıyla yere dik olarak yerleştirilmiş ince bir ahşap, beton veya çelik yapısal elemandır. Ayrıca, yüklerin daha derin yer altı katmanlarına aktarılmasıyla karakterize edilen derin temele de kazık denir. | BORED, DRIVEN, JETGROUTING, COHESION, FRICTION, SUPPORT, USERDEFINED, NOTDEFINED |

| Türkçe Tanım (Bilgi Amaçlı) | Öznitelik | İlişki | Veri Tipi (Bilgi Amaçlı) | Açıklama |
|---|---|---|---|---|
| Nesne Tipi | Attribute: `ObjectType` | - | Metin | Tip, nesneyi daha ayrıntılı tanımlayan özel bir tipi belirtir. Kullanım, örneklenebilir somut alt tiplerin seviyesinde belirlenmelidir. Şu durumlarda kullanıcı tarafından tanımlanan tip bilgisi bu özniteliğe girilir: 1) Ön Tanımlı Tip özniteliğinin değeri listeden [[USERDEFINED]] olarak seçilmişse veya 2) somut örneklenmiş varlıkta bir Ön Tanımlı Tip özniteliği yoksa. İkinci durum, bazı istisnai yaprak sınıflarında ve [[IfcBuiltElement]]'i doğrudan örneklendirirken geçerlidir. |
| Açıklama | Attribute: Description | - | Metin | Bilgilendirme amaçlı isteğe bağlı olarak verilen bir tanımdır. |

| Türkçe Tanım (Bilgi Amaçlı) | Özellik | Özellik Seti / İlişki | Veri Tipi (Bilgi Amaçlı) | Açıklama |
|---|---|---|---|---|
|  | `CSBPozNo` | `TREpys_PozOzellikSeti` | Metin | Çevre, Şehircilik ve İklim Değişikliği Bakanlığı İnşaat ve Tesisat Birim Fiyat Poz No'su. |
| Taşıyıcı | `LoadBearing` | `Pset_PileCommon` | Mantıksal | Nesnenin yük taşıma amacıyla tasarlandığını (DOĞRU) veya tasarlanmadığını (YANLIŞ) belirtir . |
| Uzunluk | Length | `Qto_PileBaseQuantities` | Sayı | Nesnenin uzunluğu. Kesitler veya kesit boşlukları gibi işleme özellikleri dikkate alınmaz. |
| Brüt Ağırlık | `GrossWeight` | `Qto_PileBaseQuantities` | Sayı | Nesnenin toplam brüt ağırlığı, ek parçalar olmaksızın ve olası imalat detayları (kesit boşluğu vb.) veya açıklıklar ve girintiler dikkate alınmadan hesaplanır. |
| Net Ağırlık | `NetWeight` | `Qto_PileBaseQuantities` | Sayı | Nesnenin toplam net ağırlığı, ek parçalar olmaksızın ve olası imalat detayları (kesit boşluğu vb.) veya açıklıklar ve girintiler dikkate alınmadan hesaplanır. |
| Beton Örtüsü | `ConcreteCover` | `Pset_ConcreteElementGeneral` | Sayı | Yerel inşaat yönetmeliklerine göre donatı çubukları üzerindeki beton örtüsü (Türkiye Bina Deprem Yönetmeliğine göre beton örtüsü, boyuna donatı ağırlık merkezi ile en dış beton lifi arasında kalan uzaklıktır) |
| Donatı Dayanım Sınıfı | `ReinforcementStrengthClass` | `Pset_ConcreteElementGeneral` | Metin | Projenin uygulanmakta olduğu beton tasarım standardına göre donatı dayanımının sınıflandırılması. Donatı dayanım sınıfı genellikle dayanım ve deformasyon özelliklerini birleştirir. |

*[PDF sayfa 180]*

*(Tablo önceki sayfadan sürüyor; sütun başlıkları okunabilirlik için yinelendi.)*

| Türkçe Tanım (Bilgi Amaçlı) | Özellik | Özellik Seti / İlişki | Veri Tipi (Bilgi Amaçlı) | Açıklama |
|---|---|---|---|---|
| Kesit Alanı | `CrossSectionArea` | `Qto_PileBaseQuantities` | Sayı | Nesnenin kesitinin (veya profilinin) toplam alanı. |
| Dış Yüzey Alanı | `OuterSurfaceArea` | `Qto_PileBaseQuantities` | Sayı | Nesnenin yüzeylerinin toplam alanı (uç kesit alanları dikkate alınmaksızın), genellikle ekstrüzyon durumunda çevre \* uzunluk olarak hesaplanır. |
| Brüt Yüzey Alanı | `GrossSurfaceArea` | `Qto_PileBaseQuantities` | Sayı | Nesnenin toplam brüt alanı, genellikle çevre \* uzunluk + 2 \* Kesit Alanı olarak hesaplanır. Bu, Dış Yüzey Alanı + (2 x Kesit Alanı) toplamıdır ve yalnızca Dış Yüzey Alanı ve Kesit Alanı ayrı olarak belirlenemediğinde verilecektir. |
| Brüt Hacim | `GrossVolume` | `Qto_PileBaseQuantities` | Sayı | Nesnenin toplam brüt hacmi. Açıklıklar, girintiler, kapalı nesneler ve çıkıntılar dikkate alınmaz. |
| Net Hacim | `NetVolume` | `Qto_PileBaseQuantities` | Sayı | Nesnenin toplam net hacmi, olası imalat özellikleri (kesit boşlukları vb.) veya açıklıklar ve girintiler dikkate alınarak hesaplanır. |

*[PDF sayfa 181]*

## Tablo 6.112 `IfcPipeFitting` Sınıfı İsteğe Bağlı Kullanılabilecek Öznitelik ve Özellik Listesi

| Boru Bağlantı Parçası (`IfcPipeFitting`) | Ön Tanımlı Tip Listesi |
|---|---|
| Tanım: Boru bağlantı parçası, borulu akış dağıtım sisteminde bir bağlantı veya geçiş elemanıdır; boru segmentlerini birleştirmek için kullanılır ve akışkanın yönü veya debisi gibi akış özelliklerinde değişikliklere neden olur. | BEND, CONNECTOR, ENTRY, EXIT, JUNCTION, OBSTRUCTION, TRANSITION, USERDEFINED, NOTDEFINED |

| Türkçe Tanım (Bilgi Amaçlı) | Öznitelik | İlişki | Veri Tipi (Bilgi Amaçlı) | Açıklama |
|---|---|---|---|---|
| Nesne Tipi | Attribute: `ObjectType` | - | Metin | Tip, nesneyi daha ayrıntılı tanımlayan özel bir tipi belirtir. Kullanım, örneklenebilir somut alt tiplerin seviyesinde belirlenmelidir. Şu durumlarda kullanıcı tarafından tanımlanan tip bilgisi bu özniteliğe girilir: 1) Ön Tanımlı Tip özniteliğinin değeri listeden [[USERDEFINED]] olarak seçilmişse veya 2) somut örneklenmiş varlıkta bir Ön Tanımlı Tip özniteliği yoksa. İkinci durum. bazı istisnai yaprak sınıflarında ve [[IfcBuiltElement]]'i doğrudan örneklendirirken geçerlidir. |
| Malzeme | Attribute (`IfcRelAssociatesMaterial>IfcMaterial`): Name\* | - | Metin | [`IfcMaterial`], yapı elemanlarını (fiziksel ürünler veya bileşenleri) oluşturmak için kullanılabilecek homojen veya heterojen maddelerdir. |
| Açıklama | Attribute: Description | - | Metin | Bilgilendirme amaçlı isteğe bağlı olarak verilen bir tanımdır. |

| Türkçe Tanım (Bilgi Amaçlı) | Özellik | Özellik Seti / İlişki | Veri Tipi (Bilgi Amaçlı) | Açıklama |
|---|---|---|---|---|
|  | `CSBPozNo` | `TREpys_PozOzellikSeti` | Metin | Çevre, Şehircilik ve İklim Değişikliği Bakanlığı İnşaat ve Tesisat Birim Fiyat Poz No'su. |
| Renk | Colour | `Pset_PipeFittingOccurrence` | Metin | Bu nesnenin rengi. |
| Basınç Sınıfı | `PressureClass` | `Pset_PipeFittingTypeCommon` | Sayı | Nesnenin nominal basınç sınıfı.<br>Bağlantı parçasının test veya nominal basınç sınıflandırması. |
| Basınç Aralığı | `PressureRange` | `Pset_PipeFittingTypeCommon` | Sayı | İzin verilen maksimum ve minimum çalışma basıncı (ortam basıncına göre). |
| Sıcaklık Aralığı | `TemperatureRange` | `Pset_PipeFittingTypeCommon` | Metin | İzin verilen maksimum ve minimum sıcaklık. |
| Bağlantı Elemanı Kayıp Faktörü | `FittingLossFactor` | `Pset_PipeFittingTypeCommon` | Sayı | Bağlantı parçası boyunca sürtünmeden kaynaklanan basınç kaybını belirleyen bir katsayı. |
| Brüt Kesit Alanı | `GrossCrossSectionArea` | `Qto_PipeFittingBaseQuantities` | Sayı | Kesit alanı.<br>Boru bağlantı elemanının kendisini ve iç akış alanını da içerir. |
| Brüt Ağırlık | `GrossWeight` | `Qto_PipeFittingBaseQuantities` | Sayı | Nesnenin toplam brüt ağırlığı, ek parçalar olmaksızın ve olası imalat detayları (kesit boşluğu vb.) veya açıklıklar ve girintiler dikkate alınmadan hesaplanır. |

*[PDF sayfa 182]*

*(Tablo önceki sayfadan sürüyor; sütun başlıkları okunabilirlik için yinelendi.)*

| Türkçe Tanım (Bilgi Amaçlı) | Özellik | Özellik Seti / İlişki | Veri Tipi (Bilgi Amaçlı) | Açıklama |
|---|---|---|---|---|
| Uzunluk | Length | `Qto_PipeFittingBaseQuantities` | Sayı | Nesnenin uzunluğu.<br>Kesitin orta noktasından hesaplanır ve bağlantı ağzı girişinden bağlantı ağzı çıkışına kadar olan akış yolu boyunca olan mesafeye eşittir. Bağlantı parçaları için, en uzun akış yolunun uzunluğunu gösterir. |
| Net Kesit Alanı | `NetCrossSectionArea` | `Qto_PipeFittingBaseQuantities` | Sayı | Nesnenin kesit alanı.<br>Boru bağlantı parçaları dahil, iç akış alanı hariç. |
| Net Ağırlık | `NetWeight` | `Qto_PipeFittingBaseQuantities` | Sayı | Nesnenin toplam net ağırlığı, ek parçalardan bağımsız olarak ve olası imalat özellikleri (kesit boşlukları vb.) veya açıklıklar ve girintiler dikkate alınmadan hesaplanır. Tasarlandığı şekilde içerdiği sıvı dahil olmak üzere boru bağlantı parçasının ağırlığı. |
| Dış Yüzey Alanı | `OuterSurfaceArea` | `Qto_PipeFittingBaseQuantities` | Sayı | Nesnenin yüzeylerinin toplam alanı (uç kesit alanları dikkate alınmaksızın), genellikle ekstrüzyon durumunda çevre \* uzunluk olarak hesaplanır. |

\*Attribute (`IfcRelAssociatesMaterial>IfcMaterial`): Name\* bilgisi giriliken: "Malzeme İsmi\_Dayanım" formatı kullanılmalıdır. (Örnek:Çelik\_S355 Beton\_C25)

*[PDF sayfa 183]*

## Tablo 6.113 `IfcPipeSegment` Sınıfı İsteğe Bağlı Kullanılabilecek Öznitelik ve Özellik Listesi

| Boru Segmenti (`IfcPipeSegment`) | Ön Tanımlı Tip Listesi |
|---|---|
| Tanım: Boru segmenti, tipik olarak bir boru şebekesinin iki bölümünü birleştirmek için kullanılır. | CULVERT, FLEXIBLESEGMENT, RIGIDSEGMENT, GUTTER, SPOOL, USERDEFINED, NOTDEFINED |

| Türkçe Tanım (Bilgi Amaçlı) | Öznitelik | İlişki | Veri Tipi (Bilgi Amaçlı) | Açıklama |
|---|---|---|---|---|
| Nesne Tipi | Attribute: `ObjectType` | - | Metin | Tip, nesneyi daha ayrıntılı tanımlayan özel bir tipi belirtir. Kullanım, örneklenebilir somut alt tiplerin seviyesinde belirlenmelidir. Şu durumlarda kullanıcı tarafından tanımlanan tip bilgisi bu özniteliğe girilir: 1) Ön Tanımlı Tip özniteliğinin değeri listeden [[USERDEFINED]] olarak seçilmişse veya 2) somut örneklenmiş varlıkta bir Ön Tanımlı Tip özniteliği yoksa. İkinci durum, bazı istisnai yaprak sınıflarında ve [[IfcBuiltElement]]'i doğrudan örneklendirirken geçerlidir. |
| Açıklama | Attribute: Description | - | Metin | Bilgilendirme amaçlı isteğe bağlı olarak verilen bir tanımdır. |
| Malzeme | Attribute(IfcRelAssociatesMaterial>IfcMaterial): Name\* | - | Metin | [`IfcMaterial`], yapı elemanlarını (fiziksel ürünler veya bileşenleri) oluşturmak için kullanılabilecek homojen veya heterojen maddelerdir. |

| Türkçe Tanım (Bilgi Amaçlı) | Özellik | Özellik Seti / İlişki | Veri Tipi (Bilgi Amaçlı) | Açıklama |
|---|---|---|---|---|
|  | `CSBPozNo` | `TREpys_PozOzellikSeti` | Metin | Çevre, Şehircilik ve İklim Değişikliği Bakanlığı İnşaat ve Tesisat Birim Fiyat Poz No'su. |
| Uzunluk | Length | `Pset_PipeSegmentTypeCommon` | Sayı | Nesnenin uzunluğu. |
| İç Çap | `InnerDiameter` | `Pset_PipeSegmentTypeCommon` | Sayı | Nesnenin iç çapı. |
| Diş Çap | `OuterDiameter` | `Pset_PipeSegmentTypeCommon` | Sayı | Nesnenin dış çapı. |
| Basınç Aralığı | `PressureRange` | `Pset_PipeSegmentTypeCommon` | Sayı | İzin verilen maksimum ve minimum çalışma basıncı (ortam basıncına göre). |
| Sıcaklık Aralığı | `TemperatureRange` | `Pset_PipeFittingTypeCommon` | Metin | İzin verilen maksimum ve minimum sıcaklık. |
| Çalışma Basıncı | `WorkingPressure` | `Pset_PipeSegmentTypeCommon` | Sayı | Çalışma basıncı. |
| Renk | Colour | `Pset_PipeSegmentOccurrence` | Metin | Bu nesnenin rengi. |
| Eğim | Gradient | `Pset_PipeSegmentOccurrence` | Sayı | Boru segmentinin eğimi. |
| Boru Alt Kotu | `InvertElevation` | `Pset_PipeSegmentOccurrence` | Sayı | Proje için belirlenen datuma göre taban kotu. |

*[PDF sayfa 184]*

|  |  |
|---|---|
|  | \*Attribute (`IfcRelAssociatesMaterial>IfcMaterial`): Name\* bilgisi giriliken: "Malzeme İsmi\_Dayanım" formatı kullanılmalıdır. (Örnek:Çelik\_S355 Beton\_C25) |

*[PDF sayfa 185]*

## Tablo 6.114 `IfcPump` Sınıfı İsteğe Bağlı Kullanılabilecek Öznitelik ve Özellik Listesi

| Pompa (`IfcPump`) | Ön Tanımlı Tip Listesi |
|---|---|
| Tanım: Pompa, akışkanlara veya çamurlara mekanik enerji aktararak bunların bir kanal veya boru sistemi içinde taşınmasını sağlayan ekipmandır. Pompanın tipik bir kullanımı, bina dağıtım sisteminde soğutulmuş su veya ısıtma sıcak suyunun dolaştırılmasıdır. | CIRCULATOR, END SUCTION, SPLIT CASE, SUBMERSIBLE PUMP, SUMP PUMP, VERTICAL INLINE, VERTICAL TURBINE, USERDEFINED, NOTDEFINED |

| Türkçe Tanım (Bilgi Amaçlı) | Öznitelik | İlişki | Veri Tipi (Bilgi Amaçlı) | Açıklama |
|---|---|---|---|---|
| Nesne Tipi | Attribute: `ObjectType` | - | Metin | Tip, nesneyi daha ayrıntılı tanımlayan özel bir tipi belirtir. Kullanım, örneklenebilir somut alt tiplerin seviyesinde belirlenmelidir. Şu durumlarda kullanıcı tarafından tanımlanan tip bilgisi bu özniteliğe girilir: 1) Ön Tanımlı Tip özniteliğinin değeri listeden [[USERDEFINED]] olarak seçilmişse veya<br>2) somut örneklenmiş varlıkta bir Ön Tanımlı Tip özniteliği yoksa. İkinci durum, bazı istisnai yaprak sınıflarında ve [[IfcBuiltElement]]'i doğrudan örneklendirirken geçerlidir. |
| Açıklama | Attribute: Description | - | Metin | Bilgilendirme amaçlı isteğe bağlı olarak verilen bir tanımdır. |

| Türkçe Tanım (Bilgi Amaçlı) | Özellik | Özellik Seti / İlişki | Veri Tipi (Bilgi Amaçlı) | Açıklama |
|---|---|---|---|---|
|  | `CSBPozNo` | `TREpys_PozOzellikSeti` | Metin | Çevre, Şehircilik ve İklim Değişikliği Bakanlığı İnşaat ve Tesisat Birim Fiyat Poz No'su. |
| Bağlantı Ölçüsü | `ConnectionSize` | `Pset_PumpTypeCommon` | Sayı | Nesnenin bağlantı boyutu. |
| Akış Direnci Aralığı | `FlowResistanceRange` | `Pset_PumpTypeCommon` | Metin | Akışkanın pompalanmakta olduğu izin verilen sürtünme direnci aralığı. |
| Nominal Akım | `RatedCurrent` | `Pset_ElectricalDeviceCommon` | Sayı | Cihazın, tasarımına göre taşıyabileceği akım. |
| Nominal Voltaj | `RatedVoltage` | `Pset_ElectricalDeviceCommon` | Sayı | Bir cihazın (çalışmak üzere) sertifikalandırıldığı izin verilen voltaj aralığı. Bu değerin üst sınırı maksimum kabul edilir. |
| Brüt Ağırlık | `GrossWeight` | `Qto_PumpBaseQuantities` | Sayı | Nesnenin toplam brüt ağırlığı, ek parçalar olmaksızın ve olası imalat detayları (kesit boşluğu vb.) veya açıklıklar ve girintiler dikkate alınmadan hesaplanır. |
| Net Pozitif Emme Yüksekliği | `NetPositiveSuctionHead` | `Pset_PumpTypeCommon` | Sayı | Kavitasyonu önlemek için pompa girişinde gerekli minimum sıvı basıncı. |
| Nominal Dönme Hızı | `NominalRotationSpeed` | `Pset_PumpTypeCommon` | Sayı | Nominal koşullar altında nesnenin dönme hızı.<br>Nominal koşullar altında pompanın dönme hızı. |
| Sıcaklık Aralığı | `TemperatureRange` | `Pset_PumpTypeCommon` | Metin  | İzin verilen maksimum ve minimum sıcaklık.<br>Akışkan sıcaklığının izin verilen çalışma aralığı. |

*[PDF sayfa 186]*

*(Tablo önceki sayfadan sürüyor; sütun başlıkları okunabilirlik için yinelendi.)*

| Türkçe Tanım (Bilgi Amaçlı) | Özellik | Özellik Seti / İlişki | Veri Tipi (Bilgi Amaçlı) | Açıklama |
|---|---|---|---|---|
| Pervane Çapı | `ImpellerDiameter` | `Pset_PumpOccurrence` | Sayı | esnenin çapı - geometrik olarak benzer nesnelerin performansını ceklendirmek için kullanılı |

*[PDF sayfa 187]*

## Tablo 6.115 `IfcRailing` Sınıfı İsteğe Bağlı Kullanılabilecek Öznitelik ve Özellik Listesi

| Korkuluk (`IfcRailing`) | Ön Tanımlı Tip Listesi |
|---|---|
| Tanım: Korkuluk, insan veya araç dolaşım alanlarının bitişiğinde ya da bazı mekân sınırlarında, duvarların yerine veya duvarları tamamlayıcı bir eleman olarak kullanılan çerçeve biçimindeki bir yapıdır. İnsanlara fiziksel destek sağlamak veya düşme ya da çarpışma sonucu oluşabilecek yaralanma ya da hasarları önlemek amacıyla tasarlanmıştır. | HAND RAIL, GUARD RAIL, BALUSTRADE, USERDEFINED, NOTDEFINED |

| Türkçe Tanım (Bilgi Amaçlı) | Öznitelik | İlişki | Veri Tipi (Bilgi Amaçlı) | Açıklama |
|---|---|---|---|---|
| Nesne Tipi | Attribute: `ObjectType` | - | Metin | Tip, nesneyi daha ayrıntılı tanımlayan özel bir tipi belirtir. Kullanım, örneklenebilir somut alt tiplerin seviyesinde belirlenmelidir. Şu durumlarda kullanıcı tarafından tanımlanan tip bilgisi bu özniteliğe girilir: 1) Ön Tanımlı Tip özniteliğinin değeri listeden [[USERDEFINED]] olarak seçilmişse veya 2) somut örneklenmiş varlıkta bir Ön Tanımlı Tip özniteliği yoksa. İkinci durum, bazı istisnai yaprak sınıflarında ve [[IfcBuiltElement]]'i doğrudan örneklendirirken geçerlidir. |
| Açıklama | Attribute: Description | - | Metin | Bilgilendirme amaçlı isteğe bağlı olarak verilen bir tanımdır. |
| Malzeme | Attribute(IfcRelAssociatesMaterial>IfcMaterial): Name\* | - | Metin | [`IfcMaterial`], yapı elemanlarını (fiziksel ürünler veya bileşenleri) oluşturmak için kullanılabilecek homojen veya heterojen maddelerdir. |

| Türkçe Tanım (Bilgi Amaçlı) | Özellik | Özellik Seti / İlişki | Veri Tipi (Bilgi Amaçlı) | Açıklama |
|---|---|---|---|---|
|  | `CSBPozNo` | `TREpys_PozOzellikSeti` | Metin | Çevre, Şehircilik ve İklim Değişikliği Bakanlığı İnşaat ve Tesisat Birim Fiyat Poz No'su. |
| Yükseklik | Height | `Pset_RailingCommon` | Sayı | Karakteristik yükseklik.<br>Zeminin veya merdivenin üzerindeki korkuluğun üst yüksekliğidir.<br>Boyut bilgisi, şekil gösterimine ve içinde kullanılan geometrik parametrelere ek olarak verilir. Ekli özellik setinde (property set) belirtilen boyut özellikleri ile geometrik parametreler arasında bir tutarsızlık olması durumunda, geometrik parametreler dikkate alınır. |

\*Attribute (`IfcRelAssociatesMaterial>IfcMaterial`): Name\* bilgisi giriliken: "Malzeme İsmi\_Dayanım" formatı kullanılmalıdır. (Örnek:Çelik\_S355 Beton\_C25)

*[PDF sayfa 188]*

## Tablo 6.116 `IfcRamp` Sınıfı İsteğe Bağlı Kullanılabilecek Öznitelik ve Özellik Listesi

| Rampa (`IfcRamp`) | Ön Tanımlı Tip Listesi |
|---|---|
| Tanım: Rampa, farklı kotlardaki iki döşeme seviyesini birbirine bağlayan dikey bir geçiş yoludur ve insan dolaşımını sağlar. Ara bir döşeme plağı olarak bir sahanlık içerebilir. Bir rampa normalde basamak içermez. | STRAIGHT RUN RAMP, TWO STRAIGHT RUN RAMP, QUARTER TURN RAMP, TWO QUARTER TURN RAMP, HALF TURN RAMP, SPIRAL RAMP, USERDEFINED, NOTDEFINED |

| Türkçe Tanım (Bilgi Amaçlı) | Öznitelik | İlişki | Veri Tipi (Bilgi Amaçlı) | Açıklama |
|---|---|---|---|---|
| Nesne Tipi | Attribute: `ObjectType` | - | Metin | Tip, nesneyi daha ayrıntılı tanımlayan özel bir tipi belirtir. Kullanım, örneklenebilir somut alt tiplerin seviyesinde belirlenmelidir. Şu durumlarda kullanıcı tarafından tanımlanan tip bilgisi bu özniteliğe girilir: 1) Ön Tanımlı Tip özniteliğinin değeri listeden [[USERDEFINED]] olarak seçilmişse veya 2) somut örneklenmiş varlıkta bir Ön Tanımlı Tip özniteliği yoksa. İkinci durum, bazı istisnai yaprak sınıflarında ve [[IfcBuiltElement]]'i doğrudan örneklendirirken geçerlidir. |
| Açıklama | Attribute: Description | - | Metin | Bilgilendirici açıklamalar paylaşmak için isteğe bağlı olarak verilen bir tanımdır. |

| Türkçe Tanım (Bilgi Amaçlı) | Özellik | Özellik Seti / İlişki | Veri Tipi (Bilgi Amaçlı) | Açıklama |
|---|---|---|---|---|
|  | `CSBPozNo` | `TREpys_PozOzellikSeti` | Metin | Çevre, Şehircilik ve İklim Değişikliği Bakanlığı İnşaat ve Tesisat Birim Fiyat Poz No'su. |
| Gerekli Eğim | `RequiredSlope` | `Pset_RampCommon` | Sayı | Nesnenin yataya göre gerekli eğim açısı (0,0 derece). Geçerli yapı yönetmeliğine veya ek gerekliliklere göre geçit için gerekli maksimum eğim. |

*[PDF sayfa 189]*

## Tablo 6.117 `IfcRoad` Sınıfı İsteğe Bağlı Kullanılabilecek Öznitelik ve Özellik Listesi

| Yol (`IfcRoad`) | Ön Tanımlı Tip Listesi |
|---|---|
| Tanım: Karayolları, caddeler, bisiklet ve yaya yolları dâhil ancak demiryolları hariç olmak üzere, bir yerden başka bir yere seyahati sağlamak amacıyla kara üzerinde inşa edilen bir güzergâhtır. Bir tesis türü olarak yol, yol projesinin bileşenleri için (örneğin tasarım, inşaat veya bakım gibi herhangi bir çalışma) proje yapısı hiyerarşisinde temel unsuru oluşturur. | BICYCLE CROSSING, BUS STOP, CARRIAGEWAY, CENTRAL ISLAND, CENTRAL RESERVE, HARD SHOULDER, INTERSECTION, LAY-BY, PARKING BAY, PASSING BAY, PEDESTRIAN CROSSING, REFUGE ISLAND, ROAD SEGMENT, ROAD SIDE, ROAD SIDE PART, ROADWAY PLATEAU, ROUNDABOUT, SHOULDER, SIDEWALK, SOFT SHOULDER, TOLL PLAZA, TRAFFIC ISLAND, TRAFFIC LANE, USERDEFINED, NOTDEFINED |

| Türkçe Tanım (Bilgi Amaçlı) | Öznitelik | İlişki | Veri Tipi (Bilgi Amaçlı) | Açıklama |
|---|---|---|---|---|
| Nesne Tipi | Attribute: `ObjectType` | - | Metin | Tip, nesneyi daha ayrıntılı tanımlayan özel bir tipi belirtir. Kullanım, örneklenebilir somut alt tiplerin seviyesinde belirlenmelidir. Şu durumlarda kullanıcı tarafından tanımlanan tip bilgisi bu özniteliğe girilir: 1) Ön Tanımlı Tip özniteliğinin değeri listeden [[USERDEFINED]] olarak seçilmişse veya 2) somut örneklenmiş varlıkta bir Ön Tanımlı Tip özniteliği yoksa. İkinci durum, bazı istisnai yaprak sınıflarında ve [[IfcBuiltElement]]'i doğrudan örneklendirirken geçerlidir. |
| Açıklama | Attribute: Description | - | Metin | Bilgilendirme amaçlı isteğe bağlı olarak verilen bir tanımdır. |

| Türkçe Tanım (Bilgi Amaçlı) | Özellik | Özellik Seti / İlişki | Veri Tipi (Bilgi Amaçlı) | Açıklama |
|---|---|---|---|---|
|  | `CSBPozNo` | `TREpys_PozOzellikSeti` | Metin | Çevre, Şehircilik ve İklim Değişikliği Bakanlığı İnşaat ve Tesisat Birim Fiyat Poz No'su. |

*[PDF sayfa 190]*

## Tablo 6.118 `IfcRoadPart` Sınıfı İsteğe Bağlı Kullanılabilecek Öznitelik ve Özellik Listesi

| Yol Parçası (`IfcRoadPart`) | Ön Tanımlı Tip Listesi |
|---|---|
| Tanım: Bir yolun parçası. | BICYCLE CROSSING, BUS STOP, CARRIAGEWAY, CENTRAL ISLAND, CENTRAL RESERVE, HARD SHOULDER, INTERSECTION, LAY-BY, PARKING BAY, PASSING BAY, PEDESTRIAN CROSSING, REFUGE ISLAND, ROAD SEGMENT, ROAD SIDE, ROAD SIDE PART, ROADWAY PLATEAU, ROUNDABOUT, SHOULDER, SIDEWALK, SOFT SHOULDER, TOLL PLAZA, TRAFFIC ISLAND, TRAFFIC LANE, USERDEFINED, NOTDEFINED |

| Türkçe Tanım (Bilgi Amaçlı) | Öznitelik | İlişki | Veri Tipi (Bilgi Amaçlı) | Açıklama |
|---|---|---|---|---|
| Nesne Tipi | Attribute: `ObjectType` | - | Metin | Tip, nesneyi daha ayrıntılı tanımlayan özel bir tipi belirtir. Kullanım, örneklenebilir somut ait tiplerin seviyesinde belirlenmelidir. Şu durumlarda kullanıcı tarafından tanımlanan tip bilgisi bu özniteliğe girilir: 1) Ön Tanımlı Tip özniteliğinin değeri listeden [[USERDEFINED]] olarak seçilmişse veya 2) somut örneklenmiş varlıkta bir Ön Tanımlı Tip özniteliği yoksa. İkinci durum, bazı istisnai yaprak sınıflarında ve [[IfcBuiltElement]]'i doğrudan örneklendirirken geçerlidir. |
| Açıklama | Attribute: Description | - | Metin | Bilgilendirme amaçlı isteğe bağlı olarak verilen bir tanımdır. |

| Türkçe Tanım (Bilgi Amaçlı) | Özellik | Özellik Seti / İlişki | Veri Tipi (Bilgi Amaçlı) | Açıklama |
|---|---|---|---|---|
|  | `CSBPozNo` | `TREpys_PozOzellikSeti` | Metin | Çevre, Şehircilik ve İklim Değişikliği Bakanlığı İnşaat ve Tesisat Birim Fiyat Poz No'su. |

*[PDF sayfa 191]*

## Tablo 6.119 `IfcRoof` Sınıfı İsteğe Bağlı Kullanılabilecek Öznitelik ve Özellik Listesi

| Çatı (`IfcRoof`) | Ön Tanımlı Tip Listesi |
|---|---|
| Tanım: Çatı, bir binanın en üst kısmını örten yapı elemanıdı; binayı hava koşullarının etkilerine karşı korur. | FLAT ROOF, SHED ROOF, GABLE ROOF, HIP ROOF, HIPPED GABLE ROOF, GAMBREL ROOF, MANSARD ROOF, BARREL ROOF, RAINBOW ROOF, BUTTERFLY ROOF, PAVILION ROOF, DOME ROOF, FREEFORM, USERDEFINED, NOTDEFINED |

| Türkçe Tanım (Bilgi Amaçlı) | Öznitelik | İlişki | Veri Tipi (Bilgi Amaçlı) | Açıklama |
|---|---|---|---|---|
| Nesne Tipi | Attribute: `ObjectType` | - | Metin | Tip, nesneyi daha ayrıntılı tanımlayan özel bir tipi belirtir. Kullanım, örneklenebilir somut alt tiplerin seviyesinde belirlenmelidir. Şu durumlarda kullanıcı tarafından tanımlanan tip bilgisi bu özniteliğe girilir: gı) Ön Tanımlı Tip özniteliğinin değeri listeden [[USERDEFINED]] olarak seçilmişse veya 2) somut örneklenmiş varlıkta bir Ön Tanımlı Tip özniteliği yoksa. İkinci durum, bazı istisnai yaprak sınıflarında ve [[IfcBuiltElement]]'i doğrudan örneklendirirken geçerlidir. |
| Açıklama | Attribute: Description | - | Metin | Bilgilendirme amaçlı isteğe bağlı olarak verilen bir tanımdır. |

| Türkçe Tanım (Bilgi Amaçlı) | Özellik | Özellik Seti / İlişki | Veri Tipi (Bilgi Amaçlı) | Açıklama |
|---|---|---|---|---|
|  | `CSBPozNo` | `TREpys_PozOzellikSeti` | Metin | Çevre, Şehircilik ve İklim Değişikliği Bakanlığı İnşaat ve Tesisat Birim Fiyat Poz No'su. |
| Isıl İletkenlik | `ThermalTransmittance` | `Pset_RoofCommon` | Sayı | Bir bileşenin, ısı akışı yönünde (tüm malzemeleri içeren) ısı iletkenlik katsayısı (U-Value). |
| Taşıyıcı | `LoadBearing` | `Pset_RoofCommon` | Mantıksal | Nesnenin yük taşıma amacıyla tasarlandığını (DOĞRU) veya tasarlanmadığını (YANLIŞ) belirtir. |

*[PDF sayfa 192]*

## Tablo 6.120 `IfcSanitaryTerminal` Sınıfı İsteğe Bağlı Kullanılabilecek Öznitelik ve Özellik Listesi

| Sıhhi Terminal (`IfcSanitaryTerminal`) | Ön Tanımlı Tip Listesi |
|---|---|
| Tanım: Sıhhi terminal, genellikle su ile beslenen ve içme, temizlik veya atık suyun uzaklaştırılması için kullanılan sabit bir cihaz veya terminaldir ya da böyle bir cihaz veya terminal ile doğrudan kullanılan bir ekipman öğesidir. | BATH, BIDET, CISTERN, SHOWER, SINK, SANITARY FOUNTAIN, TOILET PAN, URINAL, WASH HAND BASIN, WC SEAT, USERDEFINED, NOTDEFINED |

| Türkçe Tanım (Bilgi Amaçlı) | Öznitelik | İlişki | Veri Tipi (Bilgi Amaçlı) | Açıklama |
|---|---|---|---|---|
| Nesne Tipi | Attribute: `ObjectType` | - | Metin | Tip, nesneyi daha ayrıntılı tanımlayan özel bir tipi belirtir. Kullanım, örneklenebilir somut alt tiplerin seviyesinde belirlenmelidir. Şu durumlarda kullanıcı tarafından tanımlanan tip bilgisi bu özniteliğe girilir: 1) Ön Tanımlı Tip özniteliğinin değeri listeden [[USERDEFINED]] olarak seçilmişse veya<br>2) somut örneklenmiş varlıkta bir Ön Tanımlı Tip özniteliği yoksa. İkinci durum, bazı istisnai yaprak sınıflarında ve [[IfcBuiltElement]]'i doğrudan örneklendirirken geçerlidir. |
| Açıklama | Attribute: Description | - | Metin | Bilgilendirme amaçlı isteğe bağlı olarak verilen bir tanımdır. |

| Türkçe Tanım (Bilgi Amaçlı) | Özellik | Özellik Seti / İlişki | Veri Tipi (Bilgi Amaçlı) | Açıklama |
|---|---|---|---|---|
|  | `CSBPozNo` | `TREpys_PozOzellikSeti` | Metin | Çevre, Şehircilik ve İklim Değişikliği Bakanlığı İnşaat ve Tesisat Birim Fiyat Poz No'su. |
| Nominal Derinlik | `NominalDepth` | `Pset_SanitaryTerminalTypeCommon` | Sayı | Nesnenin nominal derinliği. |
| Nominal Uzunluk | `NominalLength` | `Pset_SanitaryTerminalTypeCommon` | Sayı | Nesnenin toplam nominal uzunluğu. Boyut bilgisi, şekil temsiline ve kullanılan geometrik parametrelere ek olarak verilir. Ekli özellik setinde sağlanan geometrik parametreler ile boyut özellikleri arasında tutarsızlık olması durumunda, geometrik parametreler dikkate alınır. |
| Nominal Genişlik | `NominalWidth` | `Pset_SanitaryTerminalTypeCommon` | Sayı | Nesnenin toplam nominal genişliği. Boyut bilgisi, şekil temsiline ve kullanılan geometrik parametrelere ek olarak verilir. Ekli özellik setinde sağlanan geometrik parametreler ile boyut özellikleri arasında tutarsızlık olması durumunda, geometrik parametreler dikkate alınır. |
| Renk | Colour | `Pset_SanitaryTerminalTypeCommon` | Metin | Bu nesnenin rengi. |
| Brüt Ağırlık | `GrossWeight` | `Qto_SanitaryTerminalBaseQuantities` | Sayı | Nesnenin toplam brüt ağırlığı, ek parçalar olmaksızın ve olası imalat detayları (kesit boşluğu vb.) veya açıklıklar ve girintiler dikkate alınmadan hesaplanır. |

*[PDF sayfa 193]*

## Tablo 6.121 `IfcSign` Sınıfı İsteğe Bağlı Kullanılabilecek Öznitelik ve Özellik Listesi

| Tabela (`IfcSign`) | Ön Tanımlı Tip Listesi |
|---|---|
| Tanım: Levha (tabela), yazılı, sembolik veya başka bir biçimde bilgi ya da talimat veren görsel bir bildirimdir. Tabelalar pasiftir; en yaygın biçimi görsel panodur. `IfcSign` örneği, duvar gibi bir yüzeye uygulanan bir panelin yerleştirilmiş veya bir Sinyal Bileşimi içinde toplanan birden fazla tabela örneği yerleştirilmesini ve ilgili taşıyıcı yapısal elemanlarını ifade eder. | MARKER, PICTORAL, MIRROR, USERDEFINED, NOTDEFINED |

| Türkçe Tanım (Bilgi Amaçlı) | Öznitelik | İlişki | Veri Tipi (Bilgi Amaçlı) | Açıklama |
|---|---|---|---|---|
| Nesne Tipi | Attribute: `ObjectType` | - | Metin | Tip, nesneyi daha ayrıntılı tanımlayan özel bir tipi belirtir. Kullanım, örneklenebilir somut alt tiplerin seviyesinde belirlenmelidir. Şu durumlarda kullanıcı tarafından tanımlanan tip bilgisi bu özniteliğe girilir: 1) Ön Tanımlı Tip özniteliğinin değeri listeden [[USERDEFINED]] olarak seçilmişse veya 2) somut örneklenmiş varlıkta bir Ön Tanımlı Tip özniteliği yoksa. İkinci durum, bazı istisnai yaprak sınıflarında ve [[IfcBuiltElement]]'i doğrudan örneklendirirken geçerlidir. |
| Açıklama | Attribute: Description | - | Metin | Bilgilendirme amaçlı isteğe bağlı olarak verilen bir tanımdır. |

| Türkçe Tanım (Bilgi Amaçlı) | Özellik | Özellik Seti / İlişki | Veri Tipi (Bilgi Amaçlı) | Açıklama |
|---|---|---|---|---|
|  | `CSBPozNo` | `TREpys_PozOzellikSeti` | Metin | Çevre, Şehircilik ve İklim Değişikliği Bakanlığı İnşaat ve Tesisat Birim Fiyat Poz No'su. |
| Dışarıda | `IsExternal` | `Pset_SignCommon` | Mantıksal | Yapı elemanının dışarıda kullanıma yönelik tasarlandığını (DOĞRU) veya tasarlanmadığını (YANLIŞ) gösterir. Eğer (DOĞRU) ise, bu bir dış eleman olup binanın dışına bakmaktadır. |
| Yükseklik | Height | `Qto_SignBaseQuantities` | Sayı | Karakteristik yükseklik. |

*[PDF sayfa 194]*

*(Tablo önceki sayfadan sürüyor; sütun başlıkları okunabilirlik için yinelendi.)*

| Türkçe Tanım (Bilgi Amaçlı) | Özellik | Özellik Seti / İlişki | Veri Tipi (Bilgi Amaçlı) | Açıklama |
|---|---|---|---|---|
| Genişlik | Width | `Qto_SignBaseQuantities` | Sayı | Nesnenin genişliği. Yalnızca nesne sabit kalınlığa (prizmatik) sahipse verilir. |
| Kategori | Category | `Pset_SignCommon` | Metin | Popülasyondaki aktörlerin dahil olduğu kategorinin tanımlanması |
| Dokunsal İşaretleme | `TactileMarking` | `Pset_SignCommon` | Metin | Elemanın Dokunsal İşaretleme türü. |
| Kalınlık | Thickness | `Qto_SignBaseQuantities` | Sayı | Nesnenin geometrik kalınlığı. |
| Ağırlık | Weight | `Qto_SignBaseQuantities` | Sayı | Nesnenin toplam ağırlığı. |

*[PDF sayfa 195]*

## Tablo 6.122 `IfcSite` Sınıfı İsteğe Bağlı Kullanılabilecek Öznitelik ve Özellik Listesi

| Saha (`IfcSite`) | Ön Tanımlı Tip Listesi |
|---|---|
| Tanım: Saha, projenin inşaatının gerçekleştirileceği, suyla kaplı olabilecek sınırları belirli bir kara alanıdır. Bir saha, bina inşa etmek, mevcut yapıları yenilemek veya yıkmak ya da diğer inşaatla ilgili geliştirme faaliyetleri için kullanılabilir. | - |

| Türkçe Tanım (Bilgi Amaçlı) | Öznitelik | İlişki | Veri Tipi (Bilgi Amaçlı) | Açıklama |
|---|---|---|---|---|
| Bileşim Türü | Attribute: `CompositionType` | - | Metin | Önceden tanımlı mekânsal yapı elemanının kendisini mi, bir bütününü (kompleksi) mü yoksa bir parçasını mı temsil ettiğini belirtir. Bu değerlendirme her mekânsal yapı elemanı alt tipi için ayrı ayrı yapılır. Eğer bu değer girilmemişse, varsayılan değer "ELEMENT" olarak uygulanır. |
| Açıklama | Attribute: Description | - | Metin | Bilgilendirme amaçlı isteğe bağlı olarak verilen bir tanımdır. |

| Türkçe Tanım (Bilgi Amaçlı) | Özellik | Özellik Seti / İlişki | Veri Tipi (Bilgi Amaçlı) | Açıklama |
|---|---|---|---|---|
| Brüt Alan | `GrossArea` | `Qto_SiteBaseQuantities` | Sayı | Nesnenin brüt alanı. Boşluklar, girintiler, çıkıntılar ve kesit boşlukları dikkate alınmaz. |
| İç Konum | `InternalLocation` | `Pset_Address` | Metin | Bir organizasyon tarafından, kurum içi posta teslimatı amacıyla tanımlanan adres. |
| Yapılaşabilir Alan | `BuildableArea` | `Pset_SiteCommon` | Sayı | Yerel imar yönetmeliklerine (Planlı Alanlar İmar Yönetmeliğine) göre ifade edilen, arsa kullanım alanının azami değeridir. |
| Bina Yükseklik Sınırı | `BuildingHeightLimit` | `Pset_SiteCommon` | Sayı | Bu arsa üzerindeki binalar için yerel imar yönetmeliklerine (Planlı Alanlar İmar Yönetmeliğine) göre izin verilen azami bina yüksekliğidir. |
| Brüt Çevre | `GrossPerimeter` | `Qto_SiteBaseQuantities` | Sayı | Nesnenin dış sınırı boyunca ölçülen brüt çevresidir. |
| Açıklama | Description | `Pset_Address` | Metin | Nesnenin açıklamasıdır. |
| Posta Kutusu | `PostalBox` | `Pset_Address` | Metin | Tanımlı bir posta kutusuyla ilişkilendirilen adrestir. |
| Posta Kodu | `PostalCode` | `Pset_Address` | Metin | Ülkenin posta servisi tarafından kullanılan koddur. |

*[PDF sayfa 196]*

## Tablo 6.123 `IfcSlab` Sınıfı İsteğe Bağlı Kullanılabilecek Öznitelik ve Özellik Listesi

| Döşeme (`IfcSlab`) | Ön Tanımlı Tip Listesi |
|---|---|
| Tanım: Döşeme, düşey olarak bir hacmi kapatabilen yapı bileşenidir. Döşeme, alt taşıyıcıyı (kat döşemesi) veya üst katı (çatı döşemesi) sağlayabilir ve bir binadaki herhangi bir mekânda yer alabilir. | APPROACH SLAB, BASE SLAB, FLOOR, LANDING, PAVING, ROOF, SIDEWALK, TRACK SLAB, WEARING, USERDEFINED, NOTDEFINED |

| Türkçe Tanım (Bilgi Amaçlı) | Öznitelik | İlişki | Veri Tipi (Bilgi Amaçlı) | Açıklama |
|---|---|---|---|---|
| Nesne Tipi | Attribute: `ObjectType` | - | Metin | Tip, nesneyi daha ayrıntılı tanımlayan özel bir tipi belirtir. Kullanım, örneklenebilir somut alt tiplerin seviyesinde belirlenmelidir. Şu durumlarda kullanıcı tarafından tanımlanan tip bilgisi bu özniteliğe girilir: Ön Tanımlı Tip özniteliğinin değeri listeden [[USERDEFINED]] olarak seçilmişse veya 2) somut örneklenmiş varlıkta bir Ön Tanımlı Tip özniteliği yoksa. İkinci durum, bazı istisnai yaprak sınıflarında ve [[IfcBuiltElement]]'i doğrudan örneklendirirken geçerlidir. |
| Açıklama | Attribute: Description | - | Metin | Bilgilendirme amaçlı isteğe bağlı olarak verilen bir tanımdır. |

| Türkçe Tanım (Bilgi Amaçlı) | Özellik | Özellik Seti / İlişki | Veri Tipi (Bilgi Amaçlı) | Açıklama |
|---|---|---|---|---|
|  | `CSBPozNo` | `TREpys_PozOzellikSeti` | Metin | Çevre, Şehircilik ve İklim Değişikliği Bakanlığı İnşaat ve Tesisat Birim Fiyat Poz No'su. |
| Taşıyıcı | `LoadBearing` | `Pset_SlabCommon` | Mantıksal | Nesnenin yük taşıma amacıyla tasarlanıp tasarlanmadığını belirtir (DOĞRU) veya tasarlanmadığını (YANLIŞ). |
| Uzunluk | Length | `Qto_SlabBaseQuantities` | Sayı | Nesnenin uzunluğu. Sadece dikdörtgen olması durumunda girilir. |
| Genişlik | Width | `Qto_SlabBaseQuantities` | Sayı | Nesnenin genişliği. Sadece, nesne sabit kalınlığa (prizmatik) sahipse verilir. |
| Derinlik | Depth | `Qto_SlabBaseQuantities` | Sayı | Nesnenin derinliği. Döşemenin derinliği (döşemenin izdüşümsüz - projeksiyonu alınmamış taban alanının bir doğrultudaki derinliği). Yalnızca döşemenin dikdörtgen biçiminde olduğu durum için verilmelidir. |
| Net Alan | `NetArea` | `Qto_SlabBaseQuantities` | Sayı | Nesnenin toplam net alanı. Açıklıklar, girintiler ve kesit boşlukları çıkarılır; Çıkıntılar ise eklenir. Nesnenin çıkıntılı alanını belirtir. Sadece prizmatik olduğunda verilir. |

*[PDF sayfa 197]*

*(Tablo önceki sayfadan sürüyor; sütun başlıkları okunabilirlik için yinelendi.)*

| Türkçe Tanım (Bilgi Amaçlı) | Özellik | Özellik Seti / İlişki | Veri Tipi (Bilgi Amaçlı) | Açıklama |
|---|---|---|---|---|
| Brüt Ağırlık | `GrossWeight` | `Qto_SlabBaseQuantities` | Sayı | Nesnenin toplam brüt ağırlığı, ek parçalar olmaksızın ve olası imalat detayları (kesit boşluğu vb.) veya açıklıklar ve girintiler dikkate alınmadan hesaplanır. |
| Net Ağırlık | `NetWeight` | `Qto_SlabBaseQuantities` | Sayı | Nesnenin toplam net ağırlığı, ek parçalardan olmaksızın ve olası imalat detayları (kesit boşluğu vb.) veya açıklıklar ve girintiler dikkate alınmadan hesaplanır. |
| Beton Örtüsü | `ConcreteCover` | `Pset_ConcreteElementGeneral` | Sayı | Yerel inşaat yönetmeliklerine göre donatı çubukları üzerindeki beton örtüsü (Türkiye Bina Deprem Yönetmeliğine göre beton örtüsü, boyuna donatı ağırlık merkezi ile en dış beton lifi arasında kalan uzaklıktır) |
| Donatı Dayanım Sınıfı | `ReinforcementStrengthClass` | `Pset_ConcreteElementGeneral` | Metin | Projenin uygulanmakta olduğu beton tasarım standardına göre donatı dayanımının sınıflandırılması. Donatı dayanım sınıfı genellikle dayanım ve deformasyon özelliklerini birleştirir. |
| Eğim Açısı | `PitchAngle` | `Pset_SlabCommon` | Sayı | Döşeme, çatı bileşeni olarak kullanıldığında yataya göre eğim açısı (çatı bileşeni olarak kullanılmayan durumlarda 0 derece olarak belirtilir veya belirtilmez). Şekil bilgisi, şekil gösterimine ve kullanılan geometrik parametrelere ek olarak sağlanır. Geometrik parametreler ve şekil özellikleri arasında tutarsızlık durumunda. ekli özellikte sağlanan geometrik parametreler dikkate alınır. |
| Brüt Alan | `GrossArea` | `Qto_SlabBaseQuantities` | Sayı | Nesnenin brüt alanı. Açıklıklar, girintiler, çıkıntılar ve kesit boşlukları dikkate alınmaz. Elemanın ekstrüzyon (uzatılmış) alanını belirtir. Yalnızca elemanın prizmatik olduğu durumda verilir. |
| Brüt Hacim | `GrossVolume` | `Qto_SlabBaseQuantities` | Sayı | Nesnenin toplam brüt hacmi. Açıklıklar, girintiler, kapalı nesneler ve çıkıntılar dikkate alınmaz. |
| Net Hacim | `NetVolume` | `Qto_SlabBaseQuantities` | Sayı | Nesnenin toplam net hacmi, olası imalat özellikleri (kesit boşlukları vb.) veya açıklıklar ve girintiler dikkate alınarak hesaplanır. |

*[PDF sayfa 198]*

## Tablo 6.124 `IfcSolarDevice` Sınıfı İsteğe Bağlı Kullanılabilecek Öznitelik ve Özellik Listesi

| Güneş Enerjisi Cihazı (`IfcSolarDevice`) | Ön Tanımlı Tip Listesi |
|---|---|
| Tanım: Güneş enerjisi cihazı, güneş radyasyonunu elektrik akımı veya ısıl enerji gibi diğer enerji türlerine dönüştürür. | SOLAR COLLECTOR, SOLAR PANEL, USERDEFINED, NOTDEFINED |

| Türkçe Tanım (Bilgi Amaçlı) | Öznitelik | İlişki | Veri Tipi (Bilgi Amaçlı) | Açıklama |
|---|---|---|---|---|
| Nesne Tipi | Attribute: `ObjectType` | - | Metin | Tip, nesneyi daha ayrıntılı tanımlayan özel bir tipi belirtir. Kullanım, örneklenebilir somut alt tiplerin seviyesinde belirlenmelidir. Şu durumlarda kullanıcı tarafından tanımlanan tip bilgisi bu özniteliğe girilir: 1) Ön Tanımlı Tip özniteliğinin değeri listeden [[USERDEFINED]] olarak seçilmişse veya 2) somut örneklenmiş varlıkta bir Ön Tanımlı Tip özniteliği yoksa. İkinci durum, bazı istisnai yaprak sınıflarında ve [[IfcBuiltElement]]'i doğrudan örneklendirirken geçerlidir. |
| Açıklama | Attribute: Description | - | Metin | Bilgilendirme amaçlı isteğe bağlı olarak verilen bir tanımdır. |

| Türkçe Tanım (Bilgi Amaçlı) | Özellik | Özellik Seti / İlişki | Veri Tipi (Bilgi Amaçlı) | Açıklama |
|---|---|---|---|---|
|  | `CSBPozNo` | `TREpys_PozOzellikSeti` | Metin | Çevre, Şehircilik ve İklim Değişikliği Bakanlığı İnşaat ve Tesisat Birim Fiyat Poz No'su. |
| Brüt Alan | `GrossArea` | `Qto_SolarDeviceBaseQuantities` | Sayı | Nesnenin brüt alanı. Açıklıklar, girintiler, çıkıntılar ve kesit boşlukları dikkate alınmaz. Dış çerçeveyi de kapsar. |
| Brüt Ağırlık | `GrossWeight` | `Qto_SolarDeviceBaseQuantities` | Sayı | Nesnenin toplam brüt ağırlığı, ek parçalar olmaksızın ve olası imalat detayları (kesit boşluğu vb.) veya açıklıklar ve girintiler dikkate alınmadan hesaplanır. |

*[PDF sayfa 199]*

## Tablo 6.125 `IfcSpace` Sınıfı İsteğe Bağlı Kullanılabilecek Öznitelik ve Özellik Listesi

| Mahal (`IfcSpace`) | Ön Tanımlı Tip Listesi |
|---|---|
| Tanım: Bir mahal, gerçek veya varsayımsal olarak sınırlandırılmış bir alanı veya hacmi temsil eder. Mekanlar, bir bina içinde belirli işlevleri yerine getiren alanlar veya hacimlerdir. | SPACE, PARKING, GFA, INTERNAL, EXTERNAL, USERDEFINED, NOTDEFINED |

| Türkçe Tanım (Bilgi Amaçlı) | Öznitelik | İlişki | Veri Tipi (Bilgi Amaçlı) | Açıklama |
|---|---|---|---|---|
| Açıklama | Attribute: Description | - | Metin | Bilgilendirme amaçlı isteğe bağlı olarak verilen bir tanımdır (EK-3 Tablo 3.1 ve 3.2'de verilen isimler birleştirilerek kullanılabilir) |
| Bileşim Türü | Attribute:CompositionType | - | Metin | Önceden tanımlı mahal yapı elemanının kendisini mi. bir bütününü (kompleksi) mü yoksa bir parçasını mı temsil ettiğini belirtir. Bu değerlendirme her mahal yapı elemanı alt tipi için ayrı ayrı yapılır. Eğer bu değer girilmemişse, varsayılan değer "ELEMENT" olarak uygulanır. |
| Uzun İsim | Attribute: `LongName` | - | Metin | Referans amaçlı olarak kullanılan bağlamın uzun adı (EK-3 Tablo 3.2'de verilen uzun isimler kullanılabilir) |
| Nesne Tipi | Attribute: `ObjectType` | - | Metin | Tip, nesneyi daha ayrıntılı tanımlayan özel bir tipi belirtir. Kullanım, örneklenebilir somut alt tiplerin seviyesinde belirlenmelidir. Şu durumlarda kullanıcı tarafından tanımlanan tip bilgisi bu özniteliğe girilir: 1) Ön Tanımlı Tip özniteliğinin değeri [[USERDEFINED]] olarak seçilmişse veya 2) somut örneklenmiş varlıkta bir Ön Tanımlı Tip özniteliği yoksa . İkinci durun, bazı istisnai yaprak sınıflarında ve [[IfcBuiltElement]]'i doğrudan örneklendirirken geçerlidir. |

| Türkçe Tanım (Bilgi Amaçlı) | Özellik | Özellik Seti / İlişki | Veri Tipi (Bilgi Amaçlı) | Açıklama |
|---|---|---|---|---|
| Yükseklik | Height | `Qto_SpaceBaseQuantities` | Sayı | Karakteristik yükseklik. |
| Çıta | Molding\*\* | `Pset_SpaceCoveringRequirements` | Metin | Mahal tavanı çevresindeki çıtanın malzemesini veya yapımını belirtmek amacıyla kullanılan etiket. Etiket, mahal listesi bilgileri için kullanılır. Malzeme bilgisi, kendi şekil gösterimine ve malzeme atamasına sahip bir `IfcCovering` (type=MOLDING) nesnesinin bulunmaması durumunda sağlanır. Tutarsızlık halinde `IfcCovering` elemanlarına atanmış malzeme dikkate alınır. |
| Çıta Yüksekliği | `MoldingHeight`\*\* | `Pset_SpaceCoveringRequirements` | Sayı | Çıtanın yüksekliği. Yükseklik bilgisi, kendi şekil gösterimine ve malzeme atamasına sahip bir `IfcCovering` (type=MOLDING) nesnesinin bulunmaması durumunda sağlanır. Tutarsızlık halinde `IfcCovering` elemanlarına atanmış yükseklik dikkate alınır. |

*[PDF sayfa 200]*

*(Tablo önceki sayfadan sürüyor; sütun başlıkları okunabilirlik için yinelendi.)*

| Türkçe Tanım (Bilgi Amaçlı) | Özellik | Özellik Seti / İlişki | Veri Tipi (Bilgi Amaçlı) | Açıklama |
|---|---|---|---|---|
| Bitmiş Tavan Yüksekliği | `FinishCeilingHeight` | `Qto_SpaceBaseQuantities` | Sayı | Askı tavan yüksekliği. Bitmiş tavan yüksekliği (döşeme üst kotundan asma tavan alt kotuna kadar). Yalnızca mahalde sabit yükseklikte bir asma tavan varsa girilir. |
| Zemin Kaplama Yüksekliği | `FinishFloorHeight` | `Qto_SpaceBaseQuantities` | Sayı | Döşeme yüksekliği (kaplamasız temel döşeme üstünden bitmiş döşeme kotuna kadar). Yalnızca mahalde sabit döşeme yüksekliği varsa girilir. |
| Brüt Çevre | `GrossPerimeter` | `Qto_SpaceBaseQuantities` | Sayı | Nesnenin dış sınırı boyunca ölçülen brüt çevresidir. Döşeme seviyesinde ölçülür; sanal sınırlar ve açıklıklar (örneğin kapılar) tarafından oluşturulan çevre bölümleri de dâhil olmak üzere mahalin tüm kenarları dikkate alınır. |
| Net Çevre | `NetPerimeter` | `Qto_SpaceBaseQuantities` | Sayı | Bu mekânın döşeme seviyesindeki net çevre uzunluğu. Sanal sınırlar ve açıklıklar (örneğin kapılar) tarafından oluşturulan çevre bölümlerini hariç tutar. Süpürgelik hesaplarında kullanılan ölçüdür ve kolon gibi iç sabit elemanların çevresini içerebilir. |
| Brüt Kat Alanı | `GrossFloorArea` | `Qto_SpaceBaseQuantities` | Sayı | Mahal yapı elemanı içindeki tüm brüt kat alanlarının toplamıdır. Bina katındaki yapı elemanlarının alanlarını içerir. Katla ilişkilendirilmiş mekân ve yapı elemanlarının miktarlarına ek olarak belirtilebilir. Tutarsızlık durumunda, mekân ve yapı elemanlarının bireysel miktarları geçerlidir. |
| Net Kat Alanı | `NetFloorArea` | `Qto_SpaceBaseQuantities` | Sayı | Tüm net kullanılabilir döşeme alanlarının toplamıdır. Mahal içindeki elemanların (kolonlar, iç duvarlar, sabit mobilyalar vb.), döşeme boşluklarının veya diğer çıkıntılı elemanların kapladığı alanı hariç tutar. Değişken yükseklikler dikkate alınmaz (örneğin minimum kafa kurtarma yüksekliği altında kalan alanlar için eksiltme yapılmaz). |
| Brüt Duvar Alanı | `GrossWallArea` | `Qto_SpaceBaseQuantities` | Sayı | Mekân tarafından sınırlandırılan tüm duvar (ve kolon gibi diğer düşey Sınır elemanları) alanlarının toplamı. Duvar alanı içindeki elemanların (kapılar, pencereler, diğer açıklıklar vb.) kapladığı alanı içerir. |
| Net Duvar Alanı | `NetWallArea` | `Qto_SpaceBaseQuantities` | Sayı | Mahal tarafından sınırlandırılan tüm duvar (ve kolon gibi diğer düşey Sınır elemanları) alanlarının toplamı. Duvar alanı içindeki elemanların (kapılar. pencereler, diğer açıklıklar vb.) kapladığı alanı hariç tutar. |
| Brüt Tavan Alanı | `GrossCeilingArea` | `Qto_SpaceBaseQuantities` | Sayı | Mekânın tüm tavan alanlarının toplamı. Mekân icindeki elemanların (kolonlar, iç duvarlar vb.) kapladığı alanı içerir. Tavan alanı gerçek alandır (izdüşüm alanı değil; örneğin eğimli tavanlarda). |
| Net Tavan Alanı | `NetCeilingArea` | `Qto_SpaceBaseQuantities` | Sayı | Mahal tüm tavan alanlarının toplamı. Mahal içindeki elemanların (kolonlar, iç duvarlar vb.) kapladığı alanı hariç tutar. Tavan alanı gerçek alandır (izdüşüm alanı değil; örneğin eğimli tavanlarda). |

*[PDF sayfa 201]*

*(Tablo önceki sayfadan sürüyor; sütun başlıkları okunabilirlik için yinelendi.)*

| Türkçe Tanım (Bilgi Amaçlı) | Özellik | Özellik Seti / İlişki | Veri Tipi (Bilgi Amaçlı) | Açıklama |
|---|---|---|---|---|
| Brüt Hacim | `GrossVolume` | `Qto_SpaceBaseQuantities` | Sayı | Nesnenin toplam brüt hacmi. Açıklıklar, girintiler, kapalı nesneler ve çıkıntılar dikkate alınmaz. |
| Net Hacim | `NetVolume` | `Qto_SpaceBaseQuantities` | Sayı | Nesnenin toplam net hacmi, olası imalat özellikleri (kesit boşlukları vb.) veya açıklıklar ve girintiler dikkate alınarak hesaplanır. Mahal tarafından çevrelenen net hacmi ifade eder; mahal içindeki yapı elemanlarının hacmi hariç tutulur. |
| Park Birimi Sayısı | `ParkingUnits` | `Pset_SpaceParking` | Sayı | `ParkingUse` özelliğinde belirtilen türdeki ulaşım araçlarının mekânda barındırılabilecek sayısını belirtir. 'Genellikle bu değer varsayılan olarak 1 birimdir. Ancak park alanı motosiklet veya bisiklet içinse, aynı mahalde birden fazla araç için düzenleme yapılabilir. |

\*\* Kaplamalarla ilgili bu özellikler: (1) `IfcSpace` içindeki tanımlardan faydalanılarak yada (2) IfcCovering'de belirtilen ilişkiler kurularak doğrudan modelleme ile tanımlanacak. Bu nedenle belirtilen bu iki yöntemden biri zorunludur.

\*Attribute (`IfcRelAssociatesMaterial>IfcMaterial`): Name\* bilgisi giriliken: "Malzeme İsmi\_Dayanım" formatı kullanılmalıdır. (Örnek:Çelik\_S355 Beton\_C25)

*[PDF sayfa 202]*

## Tablo 6.126 `IfcSpaceHeater` Sınıfı İsteğe Bağlı Kullanılabilecek Öznitelik ve Özellik Listesi

| Mekan Isıtıcısı (`IfcSpaceHeater`) | Ön Tanımlı Tip Listesi |
|---|---|
| Tanım: Mekân ısıtıcıları, sınırlı bir mekânı veya alanı ısıtmak için elektrik, buhar veya sıcak su gibi bir ısıtma kaynağı kullanarak ışınım (radyasyon) ve/veya doğal konveksiyon kombinasyonundan yararlanır. Mekân ısıtıcılarına örnek olarak radyatörler, konvektörler, süpürgelik tipi ısıtıcılar ve kanatlı borulu ısıtıcılar verilebilir. | CONVECTOR, RADIATOR, USERDEFINED, NOTDEFINED |

| Türkçe Tanım (Bilgi Amaçlı) | Öznitelik | İlişki | Veri Tipi (Bilgi Amaçlı) | Açıklama |
|---|---|---|---|---|
| Nesne Tipi | Attribute: `ObjectType` | - | Metin | Tip, nesneyi daha ayrıntılı tanımlayan özel bir tipi belirtir. Kullanım, örneklenebilir somut alt tiplerin seviyesinde belirlenmelidir. Şu durumlarda kullanıcı tarafından tanımlanan tip bilgisi bu özniteliğe<br>1) Ön Tanımlı Tip özniteliğinin değeri listeden [[USERDEFINED]] olarak seçilmişse veya 2) somut örneklenmiş varlıkta bir Ön Tanımlı Tip özniteliği yoksa. İkinci durum, bazı istisnai yaprak sınıflarında ve [[IfcBuiltElement]]'i doğrudan örneklendirirken geçerlidir. |
| Açıklama | Attribute: Description | - | Metin | Bilgilendirme amaçlı isteğe bağlı olarak verilen bir tanımdır. |

| Türkçe Tanım (Bilgi Amaçlı) | Özellik | Özellik Seti / İlişki | Veri Tipi (Bilgi Amaçlı) | Açıklama |
|---|---|---|---|---|
|  | `CSBPozNo` | `TREpys_PozOzellikSeti` | Metin | Çevre, Şehircilik ve İklim Değişikliği Bakanlığı İnşaat ve Tesisat Birim Fiyat Poz No'su. |
| Uzunluk | Length | `Qto_SpaceHeaterBaseQuantities` | Sayı | Nesnenin uzunluğu. |
| Brüt Ağırlık | `GrossWeight` | `Qto_SpaceHeaterBaseQuantities` | Sayı | Nesnenin toplam brüt ağırlığı, ek parçalar olmaksızın ve olası imalat detayları (kesit boşluğu vb.) veya açıklıklar ve girintiler dikkate alınmadan hesaplanır. |
| Net Ağırlık | `NetWeight` | `Qto_SpaceHeaterBaseQuantities` | Sayı | Nesnenin toplam net ağırlığı, ek parçalar olmaksızın ve olası imalatl detayları (kesit boşluğu vb.) veya açıklıklar ve girintiler dikkate alınmadan hesaplanır. Tasarlandığı şekilde içerdiği sıvı dahil olmak üzere yapı elemanın ağırlığı. |
| Mekan Isıtıcısı Konumu | `SpaceHeaterPlacement` | `Pset_SpaceHeaterTypeCommon` | Metin | Mekan ısıtıcısının nasıl tasarıma göre nasıl konumlandırılacağını belirtir. |
| Sıcaklık Sınıflandırması | `TemperatureClassification` | `Pset_SpaceHeaterTypeCommon` | Metin | Mekân ısıtıcısının yüzey sıcaklığına göre sıcaklık sınıflandırmasını tanımlayan listeden aldığı değer. Düşük sıcaklık: Yüzey sıcaklığı nispeten düşüktür; genellikle sıcak su |

*[PDF sayfa 203]*

*(Tablo önceki sayfadan sürüyor; sütun başlıkları okunabilirlik için yinelendi.)*

| Türkçe Tanım (Bilgi Amaçlı) | Özellik | Özellik Seti / İlişki | Veri Tipi (Bilgi Amaçlı) | Açıklama |
|---|---|---|---|---|
|  |  |  |  | elektrikle ısıtılır. Yüksek sıcaklık: Yüzey sıcaklığı nispeten yüksektir; genellikle gaz veya buharla ısıtılır. |
| Isı Transferi Boyutu | `HeatTransferDimension` | `Pset_SpaceHeaterTypeCommon` | Metin | Mekân ısıtıcısının şekline bağlı olarak ısının nasıl iletildiğini gösterir. |
| Isı Transfer Ortamı | `HeatTransferMedium` | `Pset_SpaceHeaterTypeCommon` | Metin | Varsa, ısı transfer tanımlayan liste değeri. |
| Enerji Kaynağı | `EnergySource` | `Pset_SpaceHeaterTypeCommon` | Metin | kaynağını veya yakılan enerjiyi tanımlayan liste. Not: Hidronik ısıtıcılar için "Tanımsız\*, hem hidronik hem elektrikli çalışan ısıtıcılar için "Elektrik" değeri kullanılmalıdır. |
| Gövde Kütlesi | `BodyMass` | `Pset_SpaceHeaterTypeCommon` | Sayı | Isıtıcının toplam gövde kütlesi. |
| Termal Kütle Isı Kapasitesi | `ThermalMassHeatCapacity` | `Pset_SpaceHeaterTypeCommon` | Sayı | Bileşen kütlesi ile özgül ısısının çarpımıdır. |
| Termal Verimlilik | `ThermalEfficiency` | `Pset_SpaceHeaterTypeCommon` | Sayı | Genel Isıl Verimi (Overall Thermal Efficiency), bir Isı transfer cihazının brüt enerji çıktısının, enerji girdisine oranı olarak tanımlanır. |
| Panel Sayısı | `NumberOfPanels` | `Pset_SpaceHeaterTypeCommon` | Sayı | Panel sayısı. |
| Bölüm Sayısı | `NumberOfSections` | `Pset_SpaceHeaterTypeCommon` | Sayı | Bölüm sayısı.<br>Akış yönünde ölçülen dikey bölüm sayısı. |

*[PDF sayfa 204]*

## Tablo 6.127 `IfcSpatialZone` Sınıfı İsteğe Bağlı Kullanılabilecek Öznitelik ve Özellik Listesi

| Mekansal Zon (`IfcSpatialZone`) | Ön Tanımlı Tip Listesi |
|---|---|
| Tanım: Mekansal zon, belirli bir işlevsel amaca göre projenin hiyerarşik olmayan ve potansiyel olarak örtüşebilen bir alt bölümlemedir. Mekânsal zonlar örneğin bir ısı bölgesi, inşaat bölgesi, aydınlatma bölgesi veya kullanılabilir alanı temsil etmek için kullanılabilir. Bir mekansal zon, kendi konumlandırmasına ve biçim (geometri) temsiline sahip olabilir. | - |

| Türkçe Tanım (Bilgi Amaçlı) | Öznitelik | İlişki | Veri Tipi (Bilgi Amaçlı) | Açıklama |
|---|---|---|---|---|
| Nesne Tipi | Attribute: `ObjectType` | - | Metin | Tip, nesneyi daha ayrıntılı tanımlayan özel bir tipi belirtir. Kullanım, örneklenebilir somut alt tiplerin seviyesinde belirlenmelidir. Şu durumlarda kullanıcı tarafından tanımlanan tip bilgisi bu özniteliğe girilir: 1) Ön Tanımlı Tip özniteliğinin değeri listeden [[USERDEFINED]] olarak seçilmişse veya 2) somut örneklenmiş varlıkta bir Ön Tanımlı Tip özniteliği yoksa . İkinci durum, bazı istisnai yaprak sınıflarında ve [[IfcBuiltElement]]'i doğrudan örneklendirirken geçerlidir. |

| Türkçe Tanım (Bilgi Amaçlı) | Özellik | Özellik Seti / İlişki | Veri Tipi (Bilgi Amaçlı) | Açıklama |
|---|---|---|---|---|
| Brüt Planlanan Alan | `GrossPlannedArea` | `Pset_ZoneCommon` | Sayı | Mekânsal yapı elemanına ait toplam planlanan brüt alandır. Mekânsal yapı elemanının programlanması amacıyla kullanılır. |
| Net Planlanan Alan | `NetPlannedArea` | `Pset_ZoneCommon` | Sayı | Nesneye ait toplam planlanan net alandır. Nesnenin programlanması amacıyla kullanılır. |
| Toplam Hava Akışı | `TotalAirFlow` | `Pset_AirSideSystemInformation` | Sayı | Açıklama verilmemiştir. |
| Toplam Enerji Kazancı | `EnergyGainTotal` | `Pset_AirSideSystemInformation` | Sayı | Sistem tarafından hizmet verilen mahallerin maksimum soğutma koşullarındaki toplam\*enerji kazançları ile sistem düzeyindeki tüm enerji kazançlarının toplamıdır. |
| Enerji Kaybı | `EnergyLoss` | `Pset_AirSideSystemInformation` | Sayı | Sistem tarafından hizmet verilen mahallerin maksimum ısıtma koşullarındaki toplam enerji kayıplarının toplamıdır. |
| Isıtma Sıcaklık Farkı | `HeatingTemperatureDelta` | `Pset_AirSideSystemInformation` | Sayı | Mekan hava debilerini hesaplamak için kullanılan ısıtma sıcaklık farkıdır. |
| Soğutma Sıcaklık Farkı | `CoolingTemperatureDelta` | `Pset_AirSideSystemInformation` | Sayı | Mekan hava debilerini hesaplamak için kullanılan soğutma sıcaklık farkıdır. |
| Sıcaklık Ayar Noktası | `TemperatureSetPoint` | `Pset_SpaceHVACDesign` | Sayı | Sıcaklık ayar noktası aralığı ve varsayılan ayar noktasıdır. |

*[PDF sayfa 205]*

*(Tablo önceki sayfadan sürüyor; sütun başlıkları okunabilirlik için yinelendi.)*

| Türkçe Tanım (Bilgi Amaçlı) | Özellik | Özellik Seti / İlişki | Veri Tipi (Bilgi Amaçlı) | Açıklama |
|---|---|---|---|---|
| Nem Ayar Noktası | `HumiditySetPoint` | `Pset_SpaceHVACDesign` | Sayı | Kullanıcı veya tasarımcı bakış açısından mekan veya zon için gerekli olan nem oranıdır. Eğer yaz veya kış dönemi için özel bir nem gereksinimi belirtilmemişse, tüm yıl için geçerlidir; aksi halde geçiş dönemine uygulanır. Nem aralığı (Min-Maks) mevcut değilse bu özellik belirtilmelidir. |
| Hava İşleme Adı | `AirHandlingName` | `Pset_SpaceHVACDesign` | Metin | Hava tarafı sisteminin adıdır. Doğru `AirHandlingSystem` (IfcSystem)'e referans vermek için `IfcRelServicesBuildings` kullanılmalıdır. |
| Acil Çıkış | `FireExit` | `Pset_SpaceFireSafetyRequirements` | Mantıksal | Bu nesnenin yangın durumunda çıkış olarak kullanılmak üzere tasarlandığını (DOĞRU) veya tasarlanmadığını (YANLIŞ) belirtir. Burada, bir mekanın (örneğin bir koridor gibi) yangın kaçışı için hizmet verecek bir çıkış alanı olarak tasarlanıp tasarlanmadığını ifade eder |

*[PDF sayfa 206]*

## Tablo 6.128 `IfcStair` Sınıfı İsteğe Bağlı Kullanılabilecek Öznitelik ve Özellik Listesi

| Merdiven (`IfcStair`) | Ön Tanımlı Tip Listesi |
|---|---|
| Tanım: Merdiven, kullanıcıların bir döşeme seviyesinden diğerine yürüyerek (adım atarak) çıkmasına/ inmesine imkân tanıyan düşey bir geçiş yoludur. Ara kat döşemesi gibi bir sahanlık içerebilir. | STRAIGHT RUN STAIR, TWO STRAIGHT RUN STAIR, QUARTER WINDING STAIR, QUARTER TURN STAIR, HALF WINDING STAIR, HALF TURN STAIR, TWO QUARTER WINDING STAIR, TWO QUARTER TURN STAIR, THREE QUARTER WINDING STAIR, THREE QUARTER TURN STAIR, SPIRAL STAIR, DOUBLE RETURN STAIR, CURVED RUN STAIR, TWO CURVED RUN STAIR, USERDEFINED, NOTDEFINED |

| Türkçe Tanım (Bilgi Amaçlı) | Öznitelik | İlişki | Veri Tipi (Bilgi Amaçlı) | Açıklama |
|---|---|---|---|---|
| Nesne Tipi | Attribute: `ObjectType` | - | Metin | Tip, nesneyi daha ayrıntılı tanımlayan özel bir tipi belirtir. Kullanım, örneklenebilir somut alt tiplerin seviyesinde belirlenmelidir. Şu durumlarda kullanıcı tarafından tanımlanan tip bilgisi bu özniteliğe girilir: 1) Ön Tanımlı Tip özniteliğinin değeri listeden [[USERDEFINED]] olarak seçilmişse veya 2) somut örneklenmiş varlıkta bir Ön Tanımlı Tip özniteliği yoksa. İkinci durum, bazı istisnai yaprak sınıflarında ve [[IfcBuiltElement]]'i doğrudan örneklendirirken geçerlidir. |
| Açıklama | Attribute: Description | - | Metin | Bilgilendirme amaçlı isteğe bağlı olarak verilen bir tanımdır. |

| Türkçe Tanım (Bilgi Amaçlı) | Özellik | Özellik Seti / İlişki | Veri Tipi (Bilgi Amaçlı) | Açıklama |
|---|---|---|---|---|
|  | `CSBPozNo` | `TREpys_PozOzellikSeti` | Metin | Çevre, Şehircilik ve İklim Değişikliği Bakanlığı İnşaat ve Tesisat Birim Fiyat Poz No'su. |
| Dışarıda | `IsExternal` | `Pset_StairCommon` | Mantıksal | Yapı elemanının dışarıda kullanıma yönelik tasarlandığını (DOĞRU) veya tasarlanmadığını (YANLIŞ) gösterir. Eğer (DOĞRU) ise, bu bir dış eleman olup binanın dışına bakmaktadır. |
| Beton Örtüsü | `ConcreteCover` | `Pset_ConcreteElementGeneral` | Sayı | Yerel inşaat yönetmeliklerine göre donatı çubukları üzerindeki beton örtüsü (Türkiye Bina Deprem Yönetmeliğine göre beton örtüsü, boyuna donatı ağırlık merkezi ile en dış beton lifi arasında kalan uzaklıktır) |
| Donatı Dayanım Sınıfı | `ReinforcementStrengthClass` | `Pset_ConcreteElementGeneral` | Metin | Projenin uygulanmakta olduğu beton tasarım standardına göre donatı dayanımının sınıflandırılması. Donatı dayanım sınıfı genellikle dayanım ve deformasyon özelliklerini birleştirir. |
| Rıht Sayısı | `NumberOfRiser` | `Pset_StairCommon` | Number | Merdiven veya merdiven kolunda bulunan toplam rıht sayısı. |
| Basamak Sayısı | `NumberOfTreads` | `Pset_StairCommon` | Number | Merdiven veya merdiven kolunda bulunan toplam basamak sayısı. |

*[PDF sayfa 207]*

## Tablo 6.129 `IfcTank` Sınıfı İsteğe Bağlı Kullanılabilecek Öznitelik ve Özellik Listesi

**Tank (`IfcTank`)**

Tanım: Tank, sıvı veya gazın daha sonra kullanılmak üzere depolandığı bir kap ya da haznedir.

| Türkçe Tanım (Bilgi Amaçlı) | Öznitelik | İlişki | Veri Tipi (Bilgi Amaçlı) |
|---|---|---|---|
| Nesne Tipi | Attribute: `ObjectType` | - | Metin |
| Açıklama | Attribute: Description | - | Metin |

| Türkçe Tanım (Bilgi Amaçlı) | Özellik | Veri Tipi Özellik Seti / İlişki (Bilgi Amaçlı) |
|---|---|---|

|  |  |  |  |
|---|---|---|---|
|  | `CSBPozNo` | `TREpys_PozOzellikSeti` | Metin |
| Depo Türü | `StorageType` | `Pset_TankTypeCommon` | Metin |
| Etkin Kapasite | `EffectiveCapacity` | `Pset_TankTypeCommon` | Sayı |

Ön Tanımlı Tip Listesi

BASIN, BREAK PRESSURE, EXPANSION, FEED AND EXPANSION, OILRETENTION TRAY, PRESSURE VESSEL, STORAGE, VESSEL, USERDEFINED, NOTDEFINED Açıklama

Tip, nesneyi daha ayrıntılı tanımlayan özel bir tipi belirtir. Kullanım, örneklenebilir somut alt tiplerin seviyesinde belirlenmelidir. Şu durumlarda kullanıcı tarafından tanımlanan tip bilgisi bu özniteliğe girilir: 1) Ön Tanımlı Tip özniteliğinin değeri listeden [[USERDEFINED]] olarak seçilmişse veya 2) somut örneklenmiş varlıkta bir Ön Tanımlı Tip özniteliği yoksa. İkinci durum, bazı istisnai yaprak sınıflarında ve [[IfcBuiltElement]]'i doğrudan örneklendirirken geçerlidir.

Bilgilendirme amaçlı isteğe bağlı olarak verilen bir tanımdır.

Açıklama

Çevre, Şehircilik ve İklim Değişikliği Bakanlığı İnşaat ve Tesisat Birim Fiyat Poz No'su.

Depolanması amaçlanan genel malzeme kategorisini tanımlar. Tankın toplam efektif veya gerçek hacim kapasitesini belirtir.

*[PDF sayfa 208]*

## Tablo 6.130 `IfcTransformer` Sınıfı İsteğe Bağlı Kullanılabilecek Öznitelik ve Özellik Listesi

| Transformatör (`IfcTransformer`) | Ön Tanımlı Tip Listesi |
|---|---|
| Tanım: Transformatör, elektrik enerjisini bir devreden başka bir devreye aktaran, endüktif ve sabit bir cihazdır. | CURRENT, FREQUENCY, INVERTER, RECTIFIER, VOLTAGE, USERDEFINED, NOTDEFINED |

| Türkçe Tanım (Bilgi Amaçlı) | Öznitelik | İlişki | Veri Tipi (Bilgi Amaçlı) | Açıklama |
|---|---|---|---|---|
| Nesne Tipi | Attribute: `ObjectType` | - | Metin | Tip, nesneyi daha ayrıntılı tanımlayan özel bir tipi belirtir. Kullanım, örneklenebilir somut alt tiplerin seviyesinde belirlenmelidir. Şu durumlarda kullanıcı tarafından tanımlanan tip bilgisi bu özniteliğe girilir: 1) Ön Tanımlı Tip özniteliğinin değeri listeden [[USERDEFINED]] olarak seçilmişse veya 2) somut örneklenmiş varlıkta bir Ön Tanımlı Tip özniteliği yoksa. İkinci durum, bazı istisnai yaprak sınıflarında ve [[IfcBuiltElement]]'i doğrudan örneklendirirken geçerlidir. |
| Açıklama | Attribute: Description | - | Metin | Bilgilendirme amaçlı isteğe bağlı olarak verilen bir tanımdır. |

| Türkçe Tanım (Bilgi Amaçlı) | Özellik | Özellik Seti / İlişki | Veri Tipi (Bilgi Amaçlı) | Açıklama |
|---|---|---|---|---|
|  | `CSBPozNo` | `TREpys_PozOzellikSeti` | Metin | Çevre, Şehircilik ve İklim Değişikliği Bakanlığı İnşaat ve Tesisat Birim Fiyat Poz No'su. |
| Primer Görünen Güç | `PrimaryApparentPower` | `Pset_TransformerTypeCommon` | Sayı | Primer tarafta transformatöre giren ve dönüştürülmüş olan VA (volt- amper) cinsinden güç. |
| Primer Akım | `PrimaryCurrent` | `Pset_TransformerTypeCommon` | Sayı | Dönüştürülmek üzere transformatörün primer tarafına giren akım. |
| Primer Frekans | `PrimaryFrequency` | `Pset_TransformerTypeCommon` | Sayı | Dönüştürülmek üzere transformatörün primer tarafına giren frekans. |
| Primer Gerilim | `PrimaryVoltage` | `Pset_TransformerTypeCommon` | Sayı | Dönüştürülmek üzere transformatörün primer tarafına uygulanan gerilim. |
| Sekonder Görünen Güç | `SecondaryApparentPower` | `Pset_TransformerTypeCommon` | Sayı | Transformatörün sekonder tarafında dönüştürülmüş ve çıkışta verilen VA (volt-amper) cinsinden güç. |
| Sekonder Akım | `SecondaryCurrent` | `Pset_TransformerTypeCommon` | Sayı | Dönüştürülmüş ve transformatörün sekonder tarafından çıkan akım. |
| Sekonder Akım Türü | `SecondaryCurrentType` | `Pset_TransformerTypeCommon` | Metin | Transformatör çıkışından elde edilebilecek sekonder akım tiplerinin listesi. |
| Sekonder Frekans | `SecondaryFrequency` | `Pset_TransformerTypeCommon` | Sayı | Dönüştürülmüş ve transformatörün sekonder tarafından çıkan frekans. |

*[PDF sayfa 209]*

*(Tablo önceki sayfadan sürüyor; sütun başlıkları okunabilirlik için yinelendi.)*

| Türkçe Tanım (Bilgi Amaçlı) | Özellik | Özellik Seti / İlişki | Veri Tipi (Bilgi Amaçlı) | Açıklama |
|---|---|---|---|---|
| Sekonder Gerilim | `SecondaryVoltage` | `Pset_TransformerTypeCommon` | Sayı | Dönüştürülmüş ve transformatörün sekonder tarafından çıkan gerilim. |
| Transformatör Vektör Grubu | `TransformerVectorGroup` | `Pset_TransformerTypeCommon` | Metin | Transformatör için gerekli olanın ayarlanabileceği olası vektör gruplarının Bu enum listesindeki değerler uluslararası standart bir koda göre belirlenmiştir: İlk harf birincil sargıların nasıl bağlandığını, ikinci harf ikincil sargıların nasıl bağlandığını ve rakamlar birincil taraftan ikincil tarafa voltaj ve akımların dönüş açisini (30 derece katlarıyla) tanımlar. D: sargıların üçgen (delta) bağlı olduğunu belirtir. Y: sargıların yıldız (star) bağlı olduğunu belirtir. Z: sargıların zikzak (zig-zag) bağlı olduğunu belirtir (düşük reaktans sağlayan özel bir yıldız bağlantı biçimi). Bağlantı tipi yalnızca üç fazlı transformatörler için geçerlidir. |
| Brüt Ağırlık | `GrossWeight` | `Qto_TransformerBaseQuantities` | Sayı | Nesnenin toplam brüt ağırlığı, ek parçalar olmaksızın ve olası imalat detayları (kesit boşluğu vb.) veya açıklıklar ve girintiler dikkate alınmadan hesaplanır. |
| Koruyucu Topraklama Bağlantısı Mevcut | `HasProtectiveEarth` | `Pset_ElectricalDeviceCommon` | Mantıksal | Nesnenin koruyucu topraklama bağlantısına sahip olduğunu (=DOĞRU) veya olmadığını (=YANLIŞ) gösterir. |
| Yalıtım Standardı Sınıfı | `InsulationStandardClass` | `Pset_ElectricalDeviceCommon` | Metin | Yalıtım standart sınıfları, elektrik çarpmasına karşı temel koruma bilgileri sağlar. Yalıtım seviyelerini, yapısal gereksinimler (yüzeysel mesafe ve boşluk mesafeleri) ve elektriksel gereksinimler (elektrik dayanıklılık testlerine uygunluk) açısından tanımlar. Temel yalıtım, tek arıza koşulları altında kısa devre yapmış olarak kabul edilir. Gereken gerçek değerler, yalıtımın maruz kaldığı çalışma voltajına ve diğer faktörlere bağlıdır. Ayrıca, elektrikli cihazın koruyucu toprak bağlantısına sahip olup olmadığını da belirtir. |
| Hat Sayısı | `NumberOfPoles` | `Pset_ElectricalDeviceCommon` | Sayı | etkileyeceği kutup sayısı. Cihazın işlemeye/iletmeye yönelik tasarlandığı enerjili hat (faz) sayısı. |
| Sanal Empedans Oranı | `ImaginaryImpedanceRatio` | `Pset_TransformerTypeCommon` | Sayı | Sıfır dizisi empedansının sanal (imajiner) kısmı ile pozitif empedansın sanal kısmı (yani kısa devre geriliminin sanal kısmı) arasındaki orandır. N iletkenini (nötr iletken) içeren üç fazlı transformatörlerde kullanılır. |
| Birincil Nötr Terminal Mevcut | `IsNeutralPrimaryTerminalAvailable` | `Pset_TransformerTypeCommon` | Mantıksal | Birincil sargının nötr noktasının terminal olarak mevcut olduğunu (DOĞRU) veya olmadığını (YANLIŞ) gösterir. |
| İkincil Nötr Terminal Mevcut | `IsNeutralSecondaryTerminalAvailable` | `Pset_TransformerTypeCommon` | Mantıksal | İkincil sargının nötr noktasının terminal olarak mevcut olduğunu (DOĞRU) veya olmadığını (YANLIŞ) gösterir. |

•

*[PDF sayfa 210]*

*(Tablo önceki sayfadan sürüyor; sütun başlıkları okunabilirlik için yinelendi.)*

| Türkçe Tanım (Bilgi Amaçlı) | Özellik | Özellik Seti / İlişki | Veri Tipi (Bilgi Amaçlı) | Açıklama |
|---|---|---|---|---|
| Gerçek Empedans Oranı | `RealImpedanceRatio` | `Pset_TransformerTypeCommon` | Sayı | Sıfır dizi empedansının gerçek kısmı ile transformatörün pozitif empedansının gerçek kısmı (yani kısa devre geriliminin gerçek kısmı) arasındaki oran. N iletkeni içeren üç fazlı transformatörler için kullanılır. |
| Kısa Devre Gerilimi | `ShortCircuitVoltage` | `Pset_TransformerTypeCommon` | Sayı | Transformatörün nominal akımında kısa devre geriliminin gerçek ve sanal bileşenlerini yüzde (%) cinsinden belirten karmaşık sayı. |

*[PDF sayfa 211]*

## Tablo 6.131 `IfcTransportElement` Sınıfı İsteğe Bağlı Kullanılabilecek Öznitelik ve Özellik Listesi

| Taşıma Elemanı (`IfcTransportElement`) | Ön Tanımlı Tip Listesi |
|---|---|
| Tanım: Taşıma elemanı, bir bina veya bina kompleksi içinde insanları, hayvanları veya eşyaları taşıyan tüm ulaşım ile ilgili nesnelerin genelleştirilmiş halidir. Taşıma elemanı, (varsa) `IfcTransportElementType` tarafından tanımlanan bir taşıma elemanının yerleştirilmiş örneğini ifade eder. | ELEVATOR, ESCALATOR, MOVING WALKWAY, CRANEWAY, LIFTING GEAR, USERDEFINED, NOTDEFINED |

| Türkçe Tanım (Bilgi Amaçlı) | Öznitelik | İlişki | Veri Tipi (Bilgi Amaçlı) | Açıklama |
|---|---|---|---|---|
| Nesne Tipi | Attribute: `ObjectType` | - | Metin | Tip, nesneyi daha ayrıntılı tanımlayan özel bir tipi belirtir. Kullanım, örneklenebilir somut alt tiplerin seviyesinde belirlenmelidir. Şu durumlarda kullanıcı tarafından tanımlanan tip bilgisi bu özniteliğe girilir: 1) Ön Tanımlı Tip özniteliğinin değeri listeden [[USERDEFINED]] olarak seçilmişse veya 2) somut örneklenmiş varlıkta bir Ön Tanımlı Tip özniteliği yoksa. İkinci durum, bazı istisnai yaprak sınıflarında ve [[IfcBuiltElement]]'i doğrudan örneklendirirken geçerlidir. |
| Açıklama | Attribute: Description | - | Metin | Bilgilendirme amaçlı isteğe bağlı olarak verilen bir tanımdır. |

| Türkçe Tanım (Bilgi Amaçlı) | Özellik | Özellik Seti / İlişki | Veri Tipi (Bilgi Amaçlı) | Açıklama |
|---|---|---|---|---|
|  | `CSBPozNo` | `TREpys_PozOzellikSeti` | Metin | Çevre, Şehircilik ve İklim Değişikliği Bakanlığı İnşaat ve Tesisat Birim Fiyat Poz No'su. |
| Acil Çıkış | `FireExit` | `Pset_TransportElementCommon` | Mantıksal | Bu nesnenin yangın durumunda çıkış olarak kullanılmak üzere tasarlandığını (DOĞRU) veya tasarlanmadığını (YANLIŞ) belirtir. Örneğin bir asansör gibi taşıma elemanının, yangın tahliyesi amacıyla kullanılacak bir yangın çıkışı olarak tasarlanıp tasarlanmadığını gösterir. |

*[PDF sayfa 212]*

## Tablo 6.132 `IfcValve` Sınıfı İsteğe Bağlı Kullanılabilecek Öznitelik ve Özellik Listesi

| Vana (`IfcValve`) | Ön Tanımlı Tip Listesi |
|---|---|
| Tanım: Vana, bina tesisatı boru dağıtım sisteminde akışkanın akışını kontrol etmek veya düzenlemek' için kullanılır. | AIR RELEASE, ANTI VACUUM, CHANGE OVER, CHECK, COMMISSIONING, DIVERTING, DRAW OFF COCK, DOUBLE CHECK, DOUBLE REGULATING, FAUCET, FLUSHING, GAS COCK, GAS TAP, ISOLATING, MIXING, PRESSURE REDUCING, PRESSURE RELIEF, REGULATING, SAFETY CUT OFF, STEAM TRAP, STOP COCK, USERDEFINED, NOTDEFINED |

| Türkçe Tanım (Bilgi Amaçlı) | Öznitelik | İlişki | Veri Tipi (Bilgi Amaçlı) | Açıklama |
|---|---|---|---|---|
| Nesne Tipi | Attribute: `ObjectType` | - | Metin | Tip, nesneyi daha ayrıntılı tanımlayan özel bir tipi belirtir. Kullanım, örneklenebilir somut alt tiplerin seviyesinde belirlenmelidir. Şu durumlarda kullanıcı tarafından tanımlanan tip bilgisi bu özniteliğe<br>1) Ön Tanımlı Tip özniteliğinin değeri listeden [[USERDEFINED]] olarak seçilmişse veya 2) somut örneklenmiş varlıkta bir Ön Tanımlı Tip özniteliği yoksa. İkinci durum, bazı istisnai yaprak sınıflarında ve [[IfcBuiltElement]]'i doğrudan örneklendirirken geçerlidir. |
| Açıklama | Attribute: Description | - | Metin | Bilgilendirme amaçlı isteğe bağlı olarak verilen bir tanımdır. |

| Türkçe Tanım (Bilgi Amaçlı) | Özellik | Özellik Seti / İlişki | Veri Tipi (Bilgi Amaçlı) | Açıklama |
|---|---|---|---|---|
|  | `CSBPozNo` | `TREpys_PozOzellikSeti` | Metin | Çevre, Şehircilik ve İklim Değişikliği Bakanlığı İnşaat ve Tesisat Birim Fiyat Poz No'su. |
| Çalışma Basıncı | `WorkingPressure` | `Pset_ValveTypeCommon` | Sayı | Calışma basıncı.<br>Vananın normalde beklenen maksimum çalışma basıncı. |
| Kapatma Sınıfı | `CloseOffRating` | `Pset_ValveTypeCommon` | Sayı | Kapatma sınıfı. |
| Akış Katsayısı | `FlowCoefficient` | `Pset_ValveTypeCommon` | Sayı | Debi katsayısı (tamamen açık bir vanadan, birim basınç düşümünde geçen akışkan miktarı), genellikle vananın Kv veya Cv değeri olarak ifade edilir. |
| Test Basıncı | `TestPressure` | `Pset_ValveTypeCommon` | Sayı | Test sırasında vananın maruz kaldığı maksimum basınç. |
| Vana Mekanizması | `ValveMechanism` | `Pset_ValveTypeCommon` | Metin | Vana fonksiyonunun gerçekleştirildiği mekanizma türü: Küre: Gövde oturma portlarına göre döndürülebilen delikli bir küreye sahip vana.<br>Kelebek: Akış ekseni boyunca dönebilen aerodinamik diske sahip vana. |

,

*[PDF sayfa 213]*

*(Tablo önceki sayfadan sürüyor; sütun başlıkları okunabilirlik için yinelendi.)*

| Türkçe Tanım (Bilgi Amaçlı) | Özellik | Özellik Seti / İlişki | Veri Tipi (Bilgi Amaçlı) | Açıklama |
|---|---|---|---|---|
|  |  |  |  | Biçimlendirilmiş Kapak: Kapatma kapısı özel biçimlendirilmiş olan ve basınç ile akış değişimini daha hassas kontrol eden vida kontrollü vana. Salmastra: Konik oturma yüzeyine sahip. döner tapası salmastra ve salmastra keçesiyle tutulan vana.<br>Küresel: Küresel gövdeye sahip vida kontrollü vana.<br>Yağlanmış Tapa: Tapası ile gövde arasına basınç altında yağlayıcı enjekte edilen tapa tipi vana.<br>İğne: Akış yönü boyunca hareket eden ince koni biçimli bir elemanın sabit konik oturma yüzeyine kapanarak akışı düzenlediği vana.<br>Paralel Kayar: İşlenmiş bir plakanın şekillendirilmiş oluklar içinde kayarak sızdırmazlık sağladığı vida kontrollü vana.<br>Tapa: Gövde oturma portlarına göre döndürülebilen delikli tapaya sahip vana.<br>Kamalı Kapak: Kamalı biçimdeki plakanın konik kılavuzlara oturarak sızdırmazlık sağladığı vida kontrollü vana. |
| Vana İşletim Yöntemi | `ValveOperation` | `Pset_ValveTypeCommon` | Metin | Vana işletim yöntemini tanımlar: Ağırlık Düşürmeli: Ağırlıklı bir kolun serbest bırakılmasıyla kapanan vana; ağırlık genellikle bir tel ile tutulur, tel üzerindeki eriyebilir bağlantının ısıyla etkilenmesi sonucu kapanma gerçekleşir.<br>Şamandıralı: Su seviyesiyle birlikte yükselip alçalan bir şamandıra tarafından açılıp kapanan vana. Şamandıra bir kola veya başka bir mekanizmaya bağlı bir küre olabilir.<br>Hidrolik: Hidrolik tahrikle açılıp kapanan vana.<br>Kollu: Vana içindeki kapıyı döndüren bir kolun hareketiyle açılıp kapanan vana.<br>Kilit Korumalı: Açma ve kapama işlemi için özel bir kilit anahtarı gerektiren ve normal çalışmada mekanizması bir koruyucu kapakla örtülü olan vana.<br>Motorlu: Elektrik motoru tarafından bir aktüatör aracılığıyla açılıp kapanan vana.<br>Pnömatik: Pnömatik tahrikle açılıp kapanan vana.<br>Manyetik Bobinli: Manyetik alan oluşturan bir bobin akımıyla açık tutulan, ancak akım kesildiğinde hemen kapanan vana.<br>Yaylı: Bir yay basıncıyla konumda tutulan, ancak akışkan basıncı yay kuvvetini yendiğinde açılan vana.<br>Termostatik: Portları, istenen önceden belirlenmiş sıcaklığı korumak için açılıp kapanan vana.<br>Tekerlekli: Vana içindeki kapıyı bir tekerlek aracılığıyla hareket ettirerek açıp kapatan vana. |

*[PDF sayfa 214]*

*(Tablo önceki sayfadan sürüyor; sütun başlıkları okunabilirlik için yinelendi.)*

| Türkçe Tanım (Bilgi Amaçlı) | Özellik | Özellik Seti / İlişki | Veri Tipi (Bilgi Amaçlı) | Açıklama |
|---|---|---|---|---|
| Vana Düzeni | `ValvePattern` | `Pset_ValveTypeCommon` | Metin | Vana potlarının, akışkanın vana içindeki doğrusal akış yönüne veya port sayısına göre düzeni: Tek Portlu: Hizmet ettiği sistemden tek giriş portuna sahip, çıkış portu çevre ortamına olan vana.<br>Açılı İki Portlu: Akış yönünün 90 derece değiştiği vana.<br>Doğrusal İki Portlu: Akışın doğrusal olarak geçtiği vana.<br>Doğrusal Üç Portlu: Üç ayrı porta sahip vana.<br>Çapraz Dört Portlu: Dört ayrı porta sahip vana. |
| Brüt Ağırlık | `GrossWeight` | `Qto_ValveBaseQuantities` | Sayı | Nesnenin toplam brüt ağırlığı, ek parçalar olmaksızın ve olası imalat detayları (kesit boşluğu vb.) veya açıklıklar ve girintiler dikkate alınmadan hesaplanır. |

*[PDF sayfa 215]*

## Tablo 6.133 `IfcWall` Sınıfı İsteğe Bağlı Kullanılabilecek Öznitelik ve Özellik Listesi

| Duvar (`IfcWall`) | Ön Tanımlı Tip Listesi |
|---|---|
| Tanım: Duvar, mekanları sınırlayabilen veya bölebilen düşey bir yapısal bileşendir. Duvarlar genellikle düşey veya düşeye yakın düzlemsel elemanlardır ve yapısal yükleri taşımak üzere tasarlanabilir. Ancak, taşıyıcı olmak zorunda değildir. | ELEMENTED WALL, MOVABLE, PARAPET, PARTITIONING, PLUMBING WALL, POLYGONAL, RETAINING WALL, SHEAR, SOLIDWALL, STANDARD, WAVEWALL, USERDEFINED, NOTDEFINED |

| Türkçe Tanım (Bilgi Amaçlı) | Öznitelik | İlişki | Veri Tipi (Bilgi Amaçlı) | Açıklama |
|---|---|---|---|---|
| Nesne Tipi | Attribute: `ObjectType` | - | Metin | Tip, nesneyi daha ayrıntılı tanımlayan özel bir tipi belirtir. Kullanım, örneklenebilir somut alt tiplerin seviyesinde belirlenmelidir. Şu durumlarda kullanıcı tarafından tanımlanan tip bilgisi bu özniteliğe girilir: 1) Ön Tanımlı Tip özniteliğinin değeri listeden [[USERDEFINED]] olarak seçilmişse veya 2) somut örneklenmiş varlıkta bir Ön Tanımlı Tip özniteliği yoksa. İkinci durum, bazı istisnai yaprak sınıflarında ve [[IfcBuiltElement]]'i doğrudan örneklendirirken geçerlidir. |
| Açıklama | Attribute: Description | - | Metin | Bilgilendirme amaçlı isteğe bağlı olarak verilen bir tanımdır. |

| Türkçe Tanım (Bilgi Amaçlı) | Özellik | Özellik Seti / İlişki | Veri Tipi (Bilgi Amaçlı) | Açıklama |
|---|---|---|---|---|
|  | `CSBPozNo` | `TREpys_PozOzellikSeti` | Metin | Çevre, Şehircilik ve İklim Değişikliği Bakanlığı İnşaat ve Tesisat Birim Fiyat Poz No'su. |
| Isıl İletkenlik Katsayısı | `ThermalTransmittance` | `Pset_WallCommon` | Sayı | Bir bileşenin, ısı akışı yönünde (tüm malzemeleri içeren) ısı iletkenlik katsayısı (U-Value). |
| Taşıyıcı | `LoadBearing` | `Pset_WallCommon` | Mantıksal | Nesnenin yük taşıma amacıyla tasarlandığını (DOĞRU) veya tasarlanmadığını (YANLIŞ) belirtir . |
| Uzunluk | Length | `Qto_WallBaseQuantities` | Sayı | Nesnenin uzunluğu.<br>Orta hat boyunca (duvar yolundan farklı olsa bile). |
| Genişlik | Width | `Qto_WallBaseQuantities` | Sayı | Nesnenin genişliği. Sadece, nesne sabit kalınlığa (prizmatik) sahipse verilir.<br>Duvar hattına dik olarak ölçülür. Yalnızca duvar hattı boyunca sabit olduğu durumda verilmelidir. |
| Yükseklik | Height | `Qto_WallBaseQuantities` | Sayı | Karakteristik yükseklik.<br>Duvarın toplam nominal yüksekliği. Yalnızca duvar boyunca sabitse verilmelidir. |

*[PDF sayfa 216]*

*(Tablo önceki sayfadan sürüyor; sütun başlıkları okunabilirlik için yinelendi.)*

| Türkçe Tanım (Bilgi Amaçlı) | Özellik | Özellik Seti / İlişki | Veri Tipi (Bilgi Amaçlı) | Açıklama |
|---|---|---|---|---|
| Brüt Ağırlık | `GrossWeight` | `Qto_WallBaseQuantities` | Sayı | Nesnenin toplam brüt ağırlığı, ek parçalar olmaksızın ve olası imalat detayları (kesit boşluğu vb.) veya açıklıklar ve girintiler dikkate alınmadan hesaplanır. |
| Net Ağırlık | `NetWeight` | `Qto_WallBaseQuantities` | Sayı | Nesnenin toplam net ağırlığı, ek parçalar olmaksızın ve olası imalat detayları (kesit boşluğu vb.) veya açıklıklar ve girintiler dikkate alınmadan hesaplanır. |
| Beton Örtüsü | `ConcreteCover` | `Pset_ConcreteElementGeneral` | Sayı | Yerel inşaat yönetmeliklerine göre donatı çubukları üzerindeki beton örtüsü (Türkiye Bina Deprem Yönetmeliğine göre beton örtüsü, boyuna donatı ağırlık merkezi ile en dış beton lifi arasında kalan uzaklıktır) |
| Donatı Dayanım Sınıfı | `ReinforcementStrengthClass` | `Pset_ConcreteElementGeneral` | Metin | Projenin uygulanmakta olduğu beton tasarım standardına göre donatı dayanımının sınıflandırılması. Donatı dayamım sınıfı genellikle dayanım ve deformasyon özelliklerini birleştirir. |
| Akustik Performans Sınıfı | `AcousticRating` | `Pset_WallCommon` | Metin | Bu nesne için akustik performans sınıfı.; Ulusal yapı yönetmeliğine göre sağlanmıştır. Bu nesnenin ses iletim direncini, tam ses yutum değerleri vermek yerine bir indeks oranıyla gösterir. |
| Net Cephe Alanı | `NetSideArea` | `Qto_WallBaseQuantities` | Sayı | Nesnenin, orta düzleminin cephe görünümünden görülen Alan'ı. Açıklıklar gibi tüm nesne değişikliklerini hesaba katar. |
| Brüt Hacim | `GrossVolume` | `Qto_WallBaseQuantities` | Sayı | Nesnenin toplam brüt hacmi. Açıklıklar, girintiler, kapalı nesneler ve çıkıntılar dikkate alınmaz. |
| Net Hacim | `NetVolume` | `Qto_WallBaseQuantities` | Sayı | Nesnenin toplam net hacmi, olası imalat özellikleri (kesit boşlukları vb.) veya açıklıklar ve girintiler dikkate alınarak hesaplanır. |

*[PDF sayfa 217]*

## Tablo 6.134 `IfcWindow` Sınıfı İsteğe Bağlı Kullanılabilecek Öznitelik ve Özellik Listesi

| Pencere (`IfcWindow`) | Ön Tanımlı Tip Listesi |
|---|---|
| Tanım: Pencere, esas olarak doğal ışık ve temiz hava sağlamak için kullanılan bir yapı elemanıdır. Düşey açıklıkları içerdiği gibi çatı pencereleri veya ışıklıklar gibi yatay açıklıkları da kapsar. Açılır, döner, kayar, döner kanatlı veya sabit panellerden oluşan yapıları içerebilir. Bir pencere, bir kasadan ve bir veya birkaç panelden oluşur. | WINDOW, SKYLIGHT, LIGHTDOME, USERDEFINED, NOTDEFINED |

| Türkçe Tanım (Bilgi Amaçlı) | Öznitelik | İlişki | Veri Tipi (Bilgi Amaçlı) | Açıklama |
|---|---|---|---|---|
| Nesne Tipi | Attribute: `ObjectType` | - | Metin | Tip, nesneyi daha ayrıntılı tanımlayan özel bir tipi belirtir. Kullanım, örneklenebilir somut alt tiplerin seviyesinde belirlenmelidir. Şu durumlarda kullanıcı tarafından tanımlanan tip bilgisi bu özniteliğe girilir: 1) Ön Tanımlı Tip özniteliğinin değeri listeden [[USERDEFINED]] olarak seçilmişse veya 2) somut örneklenmiş varlıkta bir Ön Tanımlı Tip özniteliği yoksa. İkinci durum, bazı istisnai yaprak sınıflarında ve [[IfcBuiltElement]]'i doğrudan örneklendirirken geçerlidir. |
| Açıklama | Attribute: Description | - | Metin | Bilgilendirme amaçlı isteğe bağlı olarak verilen bir tanımdır. |
| Malzeme | Attribute (`IfcRelAssociatesMaterial>IfcMaterial`): Name\* | - | Metin | [`IfcMaterial`], yapı elemanlarını (fiziksel ürünler veya bileşenleri) oluşturmak için kullanılabilecek homojen veya heterojen maddelerdir. |

| Türkçe Tanım (Bilgi Amaçlı) | Özellik | Özellik Seti / İlişki | Veri Tipi (Bilgi Amaçlı) | Açıklama |
|---|---|---|---|---|
|  | `CSBPozNo` | `TREpys_PozOzellikSeti` | Metin | Çevre, Şehircilik ve İklim Değişikliği Bakanlığı İnşaat ve Tesisat Birim Fiyat Poz No'su. |
| Isıl İletkenlik Katsayısı | `ThermalTransmittance` | `Pset_WindowCommon` | Sayı | Bir bileşenin, 1sı akışı yönünde (tüm malzemeleri içeren) ısı iletkenlik katsayısı (U-Value). |
| Acil Çıkış | `FireExit` | `Pset_WindowCommon` | Mantıksal | Bu nesnenin yangın durumunda çıkış olarak kullanılmak üzere tasarlandığını (DOĞRU) veya tasarlanmadığını (YANLIŞ) belirtir. Burada, ulusal bina yönetmeliğine uygun olarak bir çıkış penceresini tanımlanmaktadır. |
| Genişlik | Width | `Qto_WindowBaseQuantities` | Sayı | Nesnenin genişliği. Sadece, nesne sabit kalınlığa (prizmatik) sahipse verilir. |

*[PDF sayfa 218]*

*(Tablo önceki sayfadan sürüyor; sütun başlıkları okunabilirlik için yinelendi.)*

| Türkçe Tanım (Bilgi Amaçlı) | Özellik | Özellik Seti / İlişki | Veri Tipi (Bilgi Amaçlı) | Açıklama |
|---|---|---|---|---|
|  |  |  |  | Pencere kasasının toplam dış genişliği. Yalnızca dikdörtgen pencereler için sağlanmalıdır. |
| Yükseklik | Height | `Qto_WindowBaseQuantities` | Sayı | Karakteristik yükseklik.<br>Pencere kasasının toplam dış yüksekliği. Yalnızca pencere dikdörtgen biçimindeyse verilmelidir. |
| Alan | Area | `Qto_WindowBaseQuantities` | Sayı | Nesne için hesaplanan alan.<br>Pencerenin dış kasasının toplam alanıdır. |
| Yangın Dayanın Sınıfı | `FireRating` | `Pset_WindowCommon` | Metin | Bu nesnenin yangın dayanım sınıfı. Ulusal yangın güvenliği sınıflandırmasına göre (Türkiye için: Binaların Yangından Korunması Hakkında Yönetmeliğe göre) verilmiştir. |
| Akustik | `AcousticRating` | `Pset_WindowCommon` | Metin | Bu nesne için akustik performans sınıfı.; Ulusal yapı yönetmeliğine göre sağlanmıştır. Bu nesnenin ses iletim direncini, tam ses yutum değerleri vermek yerine bir indeks oranıyla gösterir. |
| Performans Sınıfı |  |  |  |  |
| Cam Panel Alan Oranı | `GlazingAreaFraction` | `Pset_WindowCommon` | Sayı | Panel toplam alanına göre cam panel alanının oram.: Eğer cam panel alanı dolgu elemanı içindeki tüm paneller için ayrı ayrı verilmemişse kullanılmalıdır. |
| Güneş Geçirgenliği | `SolarTransmittance` | `Pset_DoorWindowGlazingType` | Sayı | Gelen güneş radyasyonunun doğrudan bir sistemden geçen oranı (te olarak da adlandırılır). Aşağıdaki eşitliğe dikkat edilmelidir: Asol + Rsol + Tsol = 1 |
| Görünür Işık Geçirgenliği | `VisibleLightTransmittance` | `Pset_DoorWindowGlazingType` | Sayı | Normal geliş açısında nesneden geçen görünür ışığın oranıdır. Bu, birimsiz bir değerdir. |

\*Attribute (`IfcRelAssociatesMaterial>IfcMaterial`): Name\* bilgisi giriliken: "Malzeme İsmi\_Dayanım" formatı kullanılmalıdır. (Örnek:Çelik\_S355 Beton\_C25)

*[PDF sayfa 219]*

## Tablo 6.135 `IfcZone` Sınıfı İsteğe Bağlı Kullanılabilecek Öznitelik ve Özellik Listesi

| Zon (`IfcZone`) | Ön Tanımlı Tip Listesi |
|---|---|
| Tanım: Zon, mekanların, kısmi mekanların veya diğer zonların gruplanmış halini ifade eder. Bu mekanlar bitişik veya bitişik olmayabilir. Bir zonun kendi geometrik biçim temsili yoktur. Zon yapıları hiyerarşik olmayabilir (bir projenin mekânsal yapısının aksine bkz. `SpatialStructureElement`). Yani, tek bir mekân (Space), sıfır, bir veya birden fazla `IfcZone` ile ilişkilendirilebilir. Mekânlar, üst tür olan Group'ta tanımlanan `IfcRelAssignsToGroup` ilişkisi kullanılarak bir `IfcZone` içinde gruplanır. Örneğin, zon bir daireyi temsil mekanları gruplandırarak temsil etmek için kullanılabilir. | - |

| Türkçe Tanım (Bilgi Amaçlı) | Öznitelik | İlişki | Veri Tipi (Bilgi Amaçlı) | Açıklama |
|---|---|---|---|---|
| Açıklama | Attribute:Description | - | Metin | Bilgilendirme amaçlı isteğe bağlı olarak verilen bir tanımdır. |

| Türkçe Tanım (Bilgi Amaçlı) | Özellik | Özellik Seti / İlişki | Veri Tipi (Bilgi Amaçlı) | Açıklama |
|---|---|---|---|---|
| Brüt Planlanan Alan | `GrossPlannedArea` | `Pset_ZoneCommon` | Sayı | Mekânsal yapı elemanına ait toplam planlanan brüt alandır. Mekânsal yapı elemanının programlanması amacıyla kullanılır. |
| Net Planlanan Alan | `NetPlannedArea` | `Pset_ZoneCommon` | Sayı | Nesneye ait toplam planlanan net alandır. Nesnenin programlanması amacıyla kullanılır. |
| Toplam Hava Akışı | `TotalAirFlow` | `Pset_AirSideSystemInformation` | Sayı | Açıklama verilmemiştir. |
| Toplam Enerji Kazancı | `EnergyGainTotal` | `Pset_AirSideSystemInformation` | Sayı | Sistem tarafından hizmet verilen mahallerin maksimum soğutma koşullarındaki toplam enerji kazançları ile sistem düzeyindeki tüm enerji kazançlarının toplamıdır. |
| Enerji Kaybı | `EnergyLoss` | `Pset_AirSideSystemInformation` | Sayı | Sistem tarafından hizmet verilen mahallerin maksimum ısıtma koşullarındaki toplam enerji kayıplarının toplamıdır. |
| Isıtma Sıcaklık Farkı | `HeatingTemperatureDelta` | `Pset_AirSideSystemInformation` | Sayı | Mekan hava debilerini hesaplamak için kullanılan ısıtma sıcaklık farkıdır. |
| Soğutma Sıcaklık Farkı | `CoolingTemperatureDelta` | `Pset_AirSideSystemInformation` | Sayı | Mekan hava debilerini hesaplamak için kullanılan soğutma sıcaklık farkıdır. |
| Sıcaklık Ayar Noktası | `TemperatureSetPoint` | `Pset_SpaceHVACDesign` | Sayı | Sıcaklık ayar noktası aralığı ve varsayılan ayar noktasıdır. |
| Nem Ayar Noktası | `HumiditySetPoint` | `Pset_SpaceHVACDesign` | Sayı | Kullanıcı veya tasarımcı bakış açısından, mahal veya bölge için gerekli olan nem oranıdır. Eğer yaz veya kış dönemi için özel bir nem |

*[PDF sayfa 220]*

*(Tablo önceki sayfadan sürüyor; sütun başlıkları okunabilirlik için yinelendi.)*

| Türkçe Tanım (Bilgi Amaçlı) | Özellik | Özellik Seti / İlişki | Veri Tipi (Bilgi Amaçlı) | Açıklama |
|---|---|---|---|---|
|  |  |  |  | gereksinimi belirtilmemişse, tüm yıl için geçerlidir; aksi halde geçiş dönemine uygulanır. Nem aralığı (Min-Maks) mevcut değilse bu özellik belirtilmelidir. |
| Hava İşleme Adı | `AirHandlingName` | `Pset_SpaceHVACDesign` | Metin | Hava tarafı sisteminin adidir. Doğru `AirHandlingSystem` (IfcSystem)'e referans vermek için `IfcRelServicesBuildings` kullanılmalıdır. |
| Acil Çıkış | `FireExit` | `Pset_SpaceFireSafetyRequirements` | Mantıksal | Bu nesnenin yangın durumunda çıkış olarak kullanılmak üzere tasarlandığını (DOĞRU) veya tasarlanmadığını (YANLIŞ) belirtir. Burada, bir mekanın (örneğin bir koridor gibi) yangın kaçışı için hizmet verecek bir çıkış alanı olarak tasarlanıp tasarlanmadığını ifade eder. |
