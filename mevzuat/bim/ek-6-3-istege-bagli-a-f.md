<!-- Kaynak: Resmî Gazete 5.8.2026/33331, Dijital Hazırlama Yönetmeliği ekleri (EK-1…EK-9), PDF s. 110–164. Taranmış sayfalardan OCR ile okundu; yöntem, doğrulama kapsamı ve sınırlar için README.md. -->

# EK-6 (devam) — isteğe bağlı, Tablo 6.69–6.100

*[PDF sayfa 110]*

## Tablo 6.69 `IfcAirTerminal` Sınıfı İsteğe Bağlı Öznitelik ve Özellik Listesi

| Havalandırma Menfezi (`IfcAirTerminal`) | Ön Tanımlı Tip Listesi |
|---|---|
| Tanım: Bir havalandırma menfezi, dağıtım sistem(ler)i ile bir veya birden fazla mahal arasındaki hava transferinin başlangıç veya bitiş noktasıdır. Ayrıca, bitişik mekanlar arasında hava transferi için de kullanılabilir. | DIFFUSER, GRILLE, LOUVRE, REGISTER, USERDEFINED, NOTDEFINED |

| Türkçe Tanım (Bilgi Amaçlı) | Öznitelik | İlişki | Veri Tipi (Bilgi Amaçlı) | Açıklama |
|---|---|---|---|---|
| Nesne Tipi | Attribute: `ObjectType` | - | Metin | Tip, nesneyi daha ayrıntılı tanımlayan özel bir tipi belirtir. Kullanım, örneklenebilir somut alt tiplerin seviyesinde belirlenmelidir. Şu durumlarda kullanıcı tarafından tanımlanan tip bilgisi bu özniteliğe girilir: 1) Ön Tanımlı Tip özniteliğinin değeri listeden [[USERDEFINED]] olarak seçilmişse veya 2) somut örneklenmiş varlıkta bir Ön Tanımlı Tip özniteliği yoksa . İkinci durum, bazı istisnai yaprak sınıflarında ve [[IfcBuiltElement]]'i doğrudan örneklendirirken geçerlidir. |
| Açıklama | Attribute: Description | - | Metin | Bilgilendirici yorumlar paylaşmak için isteğe bağlı olarak verilen bir tanımdır. |

| Türkçe Tanım (Bilgi Amaçlı) | Özellik | Özellik Seti / İlişki | Veri Tipi (Bilgi Amaçlı) | Açıklama |
|---|---|---|---|---|
|  | `CSBPozNo` | `TREpys_PozOzellikSeti` | Metin | Çevre, Şehircilik ve İklim Değişikliği Bakanlığı İnşaat ve Tesisat Birim Fiyat Poz No'su. |
| Üfleme Yönü | `DischargeDirection` | `Pset_AirTerminalTypeCommon` | Metin | Havalandırma menfezinin üfleme yönü. Paralel: montaj yüzeyine paralel şekilde üfleme yapar, akış yüzeye yapışacak şekilde tasarlanmıştır. Dik: montaj yüzeyinden uzağa doğru üfleme yapar. Ayarlanabilir: hem paralel hem de dik üfleme yapılabilir. |
| Efektif Alan | `EffectiveArea` | `Pset_AirTerminalTypeCommon` | Sayı | Havalandırma menfezinin efektif üfleme alanı. |
| Havalandırma Menfezi Şekli | `AirTerminalShape` | `Pset_AirTerminalTypeCommon` | Metin | Havalandırma menfezinin şekli. Slot, genellikle en-boy oranı 10:1'den büyük olan uzun ve dar bir hava çıkış cihazıdır. |
| Yüzey Tipi | `FaceType` | `Pset_AirTerminalTypeCommon` | Metin | Havalandırma menfezinin ön yüzeyinin nasıl imal edildiğini belirtir. |
| Slot Genişliği | `SlotWidth` | `Pset_AirTerminalTypeCommon` | Sayı | Slot genişliği. |
| Slot Uzunluğu | `SlotLength` | `Pset_AirTerminalTypeCommon` | Sayı | Slot uzunluğu. |
| Slot Sayısı | `NumberOfSlots` | `Pset_AirTerminalTypeCommon` | Sayı | Slot sayısını belirtir. |

*[PDF sayfa 111]*

*(Tablo önceki sayfadan sürüyor; sütun başlıkları okunabilirlik için yinelendi.)*

| Türkçe Tanım (Bilgi Amaçlı) | Özellik | Özellik Seti / İlişki | Veri Tipi (Bilgi Amaçlı) | Açıklama |
|---|---|---|---|---|
| Akış Paterni | `FlowPattern` | `Pset_AirTerminalTypeCommon` | Metin | Akış paterni. |
| Sıcaklık Aralığı | `TemperatureRange` | `Pset_AirTerminalTypeCommon` | Sayı | İzin verilen maksimum ve minimum sıcaklık. Havalandırma menfezinin kullanım için tasarlandığı sıcaklık aralığı. |
| Hava Atış Mesafesi | `ThrowLength` | `Pset_AirTerminalTypeCommon` | Sayı | akımının, Hava Menfezi'nden çıktıktan `AirFlowRateRange` in üst değerinde ve izotermal koşullar altında, maksimum akış hızının belirli bir menfez hızına düşmesine kadar yatay veya düşey eksende kat ettiği mesafe. Verilen koşullar altında, bir cismin iki noktası arasındaki mesafenin (1). termodinamik sıcaklık (T) ile değişimini karakterize eden büyüklük (IEC 113-04-27). Bu oran Kelvin başına tanımlanır. |
| Hava Dağılım Performans İndeksi | `AirDiffusionPerformanceIndex` | `Pset_AirTerminalTypeCommon` | Sayı | Hava Dağılımı Performans İndeksi (ADPI), soğutma modu koşullarında kullanılır. Bir mekanın kullanılan zonunda hava hızı ve hava sıcaklığına ilişkin birden fazla ölçüm yapıldığında ADPI, bu ölçüm noktalarının efektif cereyan sıcaklığı ve hava hızı şartlarını karşılama yüzdesini ifade eder. |
| Kaplama Tipi | `FinishType` | `Pset_AirTerminalTypeCommon` | Metin | Havalandırma menfezinin kaplama tipi. |
| Kaplama Rengi | `FinishColour` | `Pset_AirTerminalTypeCommon` | Metin | Nesnenin kaplama rengi. |
| Havalandırma Menfezi Montaj Tipi | `AirTerminalMountingType` | `Pset_AirTerminalTypeCommon` | Metin | Havalandırma menfezinin tavana, duvara vb. nasıl monte edildiği. Yüzey: Bir yüzeye (ör. duvar. kanal) monte edilir. Düz ve gömme: Yüzeye düz ve gömme monte edilir. Asma tavan montajı: Asma tavan sistemine monte edilir (Asma tavan taşıyıcı sistemi). |
| Çekirdek Tipi | `CoreType` | `Pset_AirTerminalTypeCommon` | Metin | Havalandırma menfezinin iç çekirdeğinin nasıl imal edildiğini belirtir. |
| Kanat Yatay Açısı | `CoreSetHorizontal` | `Pset_AirTerminalTypeCommon` | Sayı | Yatay (`LocalPlacement` X ekseninde) kanat açısı. |
| Kanat Dikey Açısı | `CoreSetVertical` | `Pset_AirTerminalTypeCommon` | Sayı | Dikey (`LocalPlacement` Y ekseninde) kanat açısı. |
| Sıcaklikkontroludur | `HasIntegralControl` | `Pset_AirTerminalTypeCommon` | Mantıksal | DOĞRU ise, kendi enerjisini sağlayan sıcaklık kontrolü içerir. |
| Akış Kontrol Tipi | `FlowControlType` | `Pset_AirTerminalTypeCommon` | Metin | Havalandırma menfezinin entegre edilmiş olabilecek akış kontrol elemanının tipi. |
| Susturuculudur | `HasSoundAttenuator` | `Pset_AirTerminalTypeCommon` | Mantıksal | DOĞRU ise, nesne ses yalıtımı içerir. |
| Isı Yalıtımlıdır | `HasThermalInsulation` | `Pset_AirTerminalTypeCommon` | Mantıksal | DOĞRU ise, havalandıma menfezi ısı yalıtımı içerir. |
| Boğaz Alanı | `NeckArea` | `Pset_AirTerminalTypeCommon` | Sayı | Havalandırma menfezinin boğaz alanı. |

*[PDF sayfa 112]*

*(Tablo önceki sayfadan sürüyor; sütun başlıkları okunabilirlik için yinelendi.)*

| Türkçe Tanım (Bilgi Amaçlı) | Özellik | Özellik Seti / İlişki | Veri Tipi (Bilgi Amaçlı) | Açıklama |
|---|---|---|---|---|
| Hava Akış Hızı ve Akış Kontrol Elemanı | `AirFlowrateVersusFlowControlElement` | `Pset_AirTerminalTypeCommon` | Sayı | Nominal basınç düşümünde, hava debisinin akış kontrol elemanı konumuna göre değişimi. Nominal basınç farkında, hava terminalinden geçen hava debisinin akış kontrol elemanı konumuna göre değişim karakteristiği. |
| Brüt Ağırlık | `GrossWeight` | `Qto_AirTerminalBaseQuantities` | Sayı | Nesnenin toplam brüt ağırlığı, ek parçalar olmaksızın ve olası imalat detayları (kesit boşluğu vb.) veya açıklıklar ve girintiler dikkate alınmadan hesaplanır. |
| Çevre | Perimeter | `Qto_AirTerminalBaseQuantities` | Sayı | Elemanın çevresi. |
| Toplam Yüzey Alanı | `TotalSurfaceArea` | `Qto_AirTerminalBaseQuantities` | Sayı | Elemanın toplam yüzey alanı. lava menfezinin ön plakası le ilgilidir.<br>Hava menfezinin ön plakası ile ilgilidir. |
| Hava Akış Tipi | `AirFlowType` | `Pset_AirTerminalOccurrence` | Metin | Açıklama mevcut değil. |
| Hava Debisi | `AirFlowRate` | `Pset_AirTerminalOccurrence` | Sayı | Hava debisi. Tasarlandığı şekliyle gerçekleşen hava akış miktarı. |
| Havalandırma Menfezinin Lokasyonu | `AirTerminalLocation` | `Pset_AirTerminalOccurrence` | Metin | Konum (tek bir difüzör tipi birden fazla konumda kullanılabilir); yüksek, tavan yakını olduğunu ifade eder |

*[PDF sayfa 113]*

## Tablo 6.70 `IfcAudioVisualAppliance` Sınıfı İsteğe Bağlı Kullanılabilecek Öznitelik ve Özellik Listesi

| Görüntü ve Ses Cihaz (`IfcAudioVisualAppliance`) | Ön Tanımlı Tip Listesi |
|---|---|
| Tanım: Bir görüntü ve ses cihazı veya görüntüyü gösteren, kaydeden, ileten veya alan bir aygıttır. | AMPLIFIER, CAMERA, DISPLAY, MICROPHONE, PLAYER, PROJECTOR, RECEIVER, SPEAKER, SWITCHER, TELEPHONE, TUNER, USERDEFINED, NOTDEFINED |

| Türkçe Tanım (Bilgi Amaçlı) | Öznitelik | İlişki | Veri Tipi (Bilgi Amaçlı) | Açıklama |
|---|---|---|---|---|
| Nesne Tipi | Attribute: `ObjectType` | - | Metin | Tip, nesneyi daha ayrıntılı tanımlayan özel bir tipi belirtir. Kullanım, örneklenebilir somut alt tiplerin seviyesinde belirlenmelidir. Şu durumlarda kullanıcı tarafından tanımlanan tip bilgisi bu özniteliğe girilir: 1) Ön Tanımlı Tip özniteliğinin değeri listeden [[USERDEFINED]] olarak seçilmişse veya 2) somut örneklenmiş varlıkta bir Ön Tanımlı Tip özniteliği yoksa. İkinci durum, bazı istisnai yaprak sınıflarında ve [[IfcBuiltElement]]'i doğrudan örneklendirirken geçerlidir. |
| Açıklama | Attribute: Description | - | Metin | Bilgilendirime amaçlı isteğe bağlı olarak verilen bir tanımdır. |

| Türkçe Tanım (Bilgi Amaçlı) | Özellik | Özellik Seti / İlişki | Veri Tipi (Bilgi Amaçlı) | Açıklama |
|---|---|---|---|---|
|  | `CSBPozNo` | `TREpys_PozOzellikScti` | Metin | Çevre, Şehircilik ve İklim Değişikliği Bakanlığı İnşaat ve Tesisat Birim Fiyat Poz No'su. |
| Nominal Güç Tüketimi | `NominalPowerConsumption` | `Pset_ElectricalDeviceCommon` | Sayı | Nominal toplam güç tüketimi. |
| Nominal Akım | `RatedCurrent` | `Pset_ElectricalDeviceCommon` | Sayı | Cihazın, tasarımına göre taşıyabileceği akım. |
| Nominal Voltaj | `RatedVoltage` | `Pset_ElectricalDeviceCommon` | Sayı | Bir cihazın (çalışmak üzere) sertifikalandırıldığı izin verilen voltaj aralığı. Bu değerin üst sınırı maksimum kabul edilir. |
| Brüt Ağırlık | `GrossWeight` | `Qto_AudioVisualApplianceBaseQuantities` | Sayı | Nesnenin toplam brüt ağırlığı, ek parçalar olmaksızın ve olası imalat detayları (kesit boşluğu vb.) veya açıklıklar ve oyuklar dikkate alınmadan hesaplanır. |

*[PDF sayfa 114]*

## Tablo 6.71 `IfcBeam` Sınıfı İsteğe Bağlı Kullanılabilecek Öznitelik ve Özellik Listesi

| Kiriş (`IfcBeam`) | Ön Tanımlı Tip Listesi |
|---|---|
| Tanım: Bir Kiriş, genellikle yatay veya neredeyse yatay bir yapısal elemandır ve yükleri esas olarak eğilme taşıma gücü ile karşılar. Mimari açıdan böyle bir elemanı da temsil edebilir. Yük taşıyıcı olması gerekmez | BEAM, JOIST, HOLLOWCORE, LINTEL, SPANDREL, T\_BEAM, GIRDER\_SEGMENT, DIAPHRAGM, PIERCAP, HATSTONE, CORNICE, EDGEBEAM, USERDEFINED, NOTDEFINED |

| Türkçe Tanım (Bilgi Amaçlı) | Öznitelik | İlişki | Veri Tipi (Bilgi Amaçlı) | Açıklama |
|---|---|---|---|---|
| Nesne Tipi | Attribute: `ObjectType` | - | Metin | Tip, nesneyi daha ayrıntılı tanımlayan özel bir tipi belirtir. Kullanım, örneklenebilir somut alt tiplerin seviyesinde belirlenmelidir. Şu durumlarda kullanıcı tarafından tanımlanan tip bilgisi bu özniteliğe girilir: 1) Ön Tanımlı Tip özniteliğinin değeri listeden [[USERDEFINED]] olarak seçilmişse veya 2) somut örneklenmiş varlıkta bir Ön Tanımlı Tip özniteliği yoksa. İkinci durum, bazı istisnai yaprak sınıflarında ve [[IfcBuiltElement]]'i doğrudan örneklendirirken geçerlidir. |
| Açıklama | Attribute: Description | - | Metin | Bilgilendime amaçlı isteğe bağlı olarak verilen bir tanımdır. |

| Türkçe Tanım (Bilgi Amaçlı) | Özellik | Özellik Seti / İlişki | Veri Tipi (Bilgi Amaçlı) | Açıklama |
|---|---|---|---|---|
|  | `CSBPozNo` | `TREpys_PozOzellikSeti` | Metin | Çevre, Şehircilik ve İklim Değişikliği Bakanlığı İnşaat ve Tesisat Birim Fiyat Poz No'su. |
| Taşıyıcı | `LoadBearing` | `Pset_BeamCommon` | Mantıksal | Nesnenin yük taşıma amacıyla tasarlandığını (DOĞRU) veya 'tasarlanmadığını (YANLIŞ) belirtir. |
| Uzunluk | Length | `Qto_BeamBaseQuantities` | Sayı | Nesnenin uzunluğu. Kesit boşluğu gibi imalat özellikleri dikkate alınmaz. |
| Brüt Ağırlık | `GrossWeight` | `Qto_BeamBaseQuantities` | Sayı | Nesnenin toplam brüt ağırlığı, ek parçalar olmaksızın ve olası imalat detayları (kesit boşluğu vb.) veya açıklıklar ve girintiler dikkate alınmadan hesaplanır. |
| Net Ağırlık | `NetWeight` | `Qto_BeamBaseQuantities` | Sayı | Nesnenin toplam net ağırlığı, ek parçalar olmaksızın ve olası imalat detayları (kesit boşluğu vb.) veya açıklıklar ve girintiler dikkate alınmadan hesaplanır. |
| Beton Örtüsü | `ConcreteCover` | `Pset_ConcreteElementGeneral` | Sayı | Yerel inşaat yönetmeliklerine göre donatı çubukları üzerindeki beton örtüsü (Türkiye Bina Deprem Yönetmeliğine göre beton örtüsü, boyuna donatı ağırlık merkezi ile en dış beton lifi arasında kalan uzaklıktır) |

•

*[PDF sayfa 115]*

*(Tablo önceki sayfadan sürüyor; sütun başlıkları okunabilirlik için yinelendi.)*

| Türkçe Tanım (Bilgi Amaçlı) | Özellik | Özellik Seti / İlişki | Veri Tipi (Bilgi Amaçlı) | Açıklama |
|---|---|---|---|---|
| Donatı Dayanım Sınıfı | `ReinforcementStrengthClass` | `Pset_ConcreteElementGeneral` | Metin | Projenin uygulanmakta olduğu beton tasarım standardına göre donatı dayanımının sınıflandırılması. Donatı dayanım sınıfı genellikle dayanım ve deformasyon özelliklerini birleştirir. |
| Beton Dayanım Sınıfı | `StrengthClass` | `Pset_ConcreteElementGeneral` | Metin | Projede uygulanan beton tasarım standardına göre beton dayanımının sınıflandırması. |
| Eğim | Slope | `Pset_BeamCommon` | Sayı | Eğim açısı – yataya göre relatif (0,0 derece). Şekil bilgisi, şekil temsilinin yanı sıra kullanılan geometrik parametrelerle birlikte sağlanır. Geometrik parametreler ile şekil özellikleri arasında tutarsızlık olması durumunda, geometrik parametreler dikkate alınır. Geometri düzenleme uygulamaları (ör. CAD) için bu değer yalnızca "yazılabilir" olmalıdır. |
| Dışarıda | `IsExternal` | `Pset_BeamCommon` | Mantıksal | Yapı elemanının dışarıda kullanıma yönelik tasarlandığını (DOĞRU) veya tasarlanmadığını (YANLIŞ) gösterir. Eğer (DOĞRU) ise, bu bir dış eleman olup binanın dışına bakmaktadır. |
| Kesit A lanı | `CrossSectionArea` | `Qto_BeamBaseQuantities` | Sayı | Nesnenin kesitinin (veya profilinin) toplam alanı. |
| Dış Yüzey Alanı | `OuterSurfaceArea` | `Qto_BeamBaseQuantities` | Sayı | Nesnenin yüzeylerinin toplam alanı (uç kesit alanları dikkate alınmaksızın), genellikle ekstrüzyon durumunda çevre \* uzunluk olarak hesaplanır. |
| Brüt Yüzey Alanı | `GrossSurfaceArea` | `Qto_BeamBaseQuantities` | Sayı | Nesnenin toplam brüt alanı, genellikle çevre \* uzunluk + 2 \* Kesit Alanı olarak hesaplanır. Bu, Dış Yüzey Alanı + (2 x Kesit Alanı) toplamıdır ve yalnızca Dış Yüzey Alanı ve Kesit Alanı ayrı olarak belirlenemediğinde verilecektir. |
| Net Yüzey Alanı | `NetSurfaceArea` | `Qto_BeamBaseQuantities` | Sayı | Nesnenin net yüzey alanı genellikle, olası imalat özelliklerini (kesit boşluğu vb.) veya açıklıkları ve girintileri dikkate alarak., çevre \* uzunluk + 2 \* Kesit Alanı olarak hesaplanır. |
| Brüt Hacim | `GrossVolume` | `Qto_BeamBaseQuantities` | Sayı | Nesnenin toplam brüt hacmi. Açıklıklar, girintiler, kapalı nesneler ve çıkıntılar dikkate alınmaz. |
| Net Hacim | `NetVolume` | `Qto_BeamBaseQuantities` | Sayı | Nesnenin toplam net hacmi, olası imalat özellikleri (kesit boşlukları vb.) veya açıklıklar ve girintiler dikkate alınarak hesaplanır. |

*[PDF sayfa 116]*

## Tablo 6.72 `IfcBoiler` Sınıfı İsteğe Bağlı Kullanılabilecek Öznitelik ve Özellik Listesi

| Kazan (`IfcBoiler`) | Ön Tanımlı Tip Listesi |
|---|---|
| Tanım: Kazan, doğal gaz, fuel-oil veya elektrik gibi bir enerji kaynağı kullanılarak suyun veya başka bir akışkanın ısıtıldığı, basınca dayanıklı kapalı bir basınçlı kaptır. Kazandaki akışkan daha sonra çeşitli süreçlerde veya ısıtma uygulamalarında kullanılmak üzere kazanın dışına sirküle edilir. | WATER, STEAM, USERDEFINED, NOTDEFINED |

| Türkçe Tanım (Bilgi Amaçlı) | Öznitelik | İlişki | Veri Tipi (Bilgi Amaçlı) | Açıklama |
|---|---|---|---|---|
| Nesne Tipi | Attribute: `ObjectType` | - | Metin | Tip, nesneyi daha ayrıntılı tanımlayan özel bir tipi belirtir. Kullanım, örneklenebilir somut alt tiplerin seviyesinde belirlenmelidir. Şu durumlarda kullanıcı tarafından tanımlanan tip bilgisi bu özniteliğe girilir: Ön Tanımlı Tip özniteliğinin değeri listeden [[USERDEFINED]] olarak seçilmişse veya 2) somut örneklenmiş varlıkta bir Ön Tanımlı Tip özniteliği yoksa. İkinci durum, bazı istisnai yaprak sınıflarında ve [[IfcBuiltElement]]'i doğrudan örneklendirirken geçerlidir. |
| Açıklama | Attribute: Description | - | Metin | Bilgilendirme amaçlı isteğe bağlı olarak verilen bir tanımdır. |

| Türkçe Tanım (Bilgi Amaçlı) | Özellik | Özellik Seti / İlişki | Veri Tipi (Bilgi Amaçlı) | Açıklama |
|---|---|---|---|---|
|  | `CSBPozNo` | `TREpys_PozOzellikSeti` | Metin | Çevre, Şehircilik ve İklim Değişikliği Bakanlığı İnşaat ve Tesisat Birim Fiyat Poz No'su. |
| Enerji Kaynağı | `EnergySource` | `Pset_BoilerTypeCommon` | Metin | Yakıt kaynağını veya yakılan enerjiyi tanımlayan liste |
| Su Depolama Isıtıcısı | `IsWaterStorageHeater` | `Pset_BoilerTypeCommon` | Mantıksal | Kazanın depolama kapasitesi olup olmadığını belirlemek için kullanılır. (DOĞRU) ise vardır, (YANLIŞ) ise, kazanda dahili bir depolama kapasitesi yoktur; örneğin anlık sıcak su ısıtıcısı gibi. |
| Su Girişi Sıcaklık Aralığı | `WaterInletTemperatureRange` | `Pset_BoilerTypeCommon` | Metin | İzin verilen su giriş sıcaklığı aralığı. |
| Su Depolama Kapasitesi | `WaterStorageCapacity` | `Pset_BoilerTypeCommon` | Sayı | Su depolama kapasitesi |
| Nominal Güç Tüketimi | `NominalPowerConsumption` | `Pset_ElectricalDeviceCommon` | Sayı | Nominal toplam güç tüketimi. |
| Nominal Akım | `RatedCurrent` | `Pset_ElectricalDeviceCommon` | Metin | Cihazın, tasarımına göre taşıyabileceği akım. |

*[PDF sayfa 117]*

*(Tablo önceki sayfadan sürüyor; sütun başlıkları okunabilirlik için yinelendi.)*

| Türkçe Tanım (Bilgi Amaçlı) | Özellik | Özellik Seti / İlişki | Veri Tipi (Bilgi Amaçlı) | Açıklama |
|---|---|---|---|---|
| Nominal Voltaj | `RatedVoltage` | `Pset_ElectricalDeviceCommon` | Metin | Bir cihazın (çalışmak üzere) sertifikalandırıldığı izin verilen voltaj aralığı. Bu değerin üst sınırı maksimum kabul edilir. |
| Isı Transfer Yüzey Alanı | `HeatTransferSurfaceArea` | `Pset_BoilerTypeCommon` | Sayı | Kabın toplam Isı transfer alanı. |
| Nominal Kısmi Yük Oranı | `NominalPartLoadRatio` | `Pset_BoilerTypeCommon` | Sayı | İzin verilen kısmi yük oranı aralığı. |
| Çalışma Modu | `OperatingMode` | `Pset_BoilerTypeCommon` | Metin | Kazanın çalışma modunu tanımlar. |
| Çıkış Sıcaklık Aralığı | `OutletTemperatureRange` | `Pset_BoilerTypeCommon` | Sayı | Su veya buhar için izin verilen çıkış sıcaklığı. |
| Kısmi Yük Verimlilik Eğrileri | `PartialLoadEfficiencyCurves` | `Pset_BoilerTypeCommon` | Sayı | Kazan verimliliği, kısmi yük faktörünün bir fonksiyonu olarak; E = f(partialLoadFactor). |
| Basınç Sınıfı / Basınç Dayanımı | `PressureRating` | `Pset_BoilerTypeCommon` | Sayı | Nesnenin basınç dayanımı. Kazanın, yetkili kurum tarafından belirlenen nominal basınç sınıfıdır. |
| Brüt Ağırlık | `GrossWeight` | `Qto_BoilerBaseQuantities` | Sayı | Nesnenin toplam brüt ağırlığı, ek parçalar olmaksızın ve olası imalat detayları (kesit boşluğu vb.) veya açıklıklar ve girintiler dikkate alınmadan hesaplanır. İçerdiği sıvı miktarı dahil değildir. |
| Net Ağırlık | `NetWeight` | `Qto_BoilerBaseQuantities` | Sayı | Nesnenin toplam net ağırlığı, ek parçalar olmaksızın ve olası imalat detayları (kesit boşluğu vb.) veya açıklıklar ve girintiler dikkate alınmadan hesaplanır.<br>Tasarlandığı şekilde içerdiği sıvı dahil olmak üzere yapı elemanın ağırlığı. |
| Toplam Yüzey Alanı | `TotalSurfaceArea` | `Qto_BoilerBaseQuantities` | Sayı | Elemanın toplam yüzey alanı. |

*[PDF sayfa 118]*

## Tablo 6.73 `IfcBridge` Sınıfı İsteğe Bağlı Kullanılabilecek Öznitelik ve Özellik Listesi

| Köprü (`IfcBridge`)) | Ön Tanımlı Tip Listesi |
|---|---|
| Tanım: Köprü, yayaların, hayvanların, araçların ve altyapı hizmetlerinin engellerin üzerinden veya yerden yüksekte bulunan iki nokta arasında geçişini sağlayan bir inşaat mühendisliği yapısıdır. veya | ARCHED, CABLE STAYED, CANTILEVER, CULVERT, FRAMEWORK, GIRDER, SUSPENSION, TRUSS, USERDEFINED, NOTDEFINED |

| Türkçe Tanım (Bilgi Amaçlı) | Öznitelik | İlişki | Veri Tipi (Bilgi Amaçlı) | Açıklama |
|---|---|---|---|---|
| Nesne Tipi | Attribute: `ObjectType` | - | Metin | Tip, nesneyi daha ayrıntılı tanımlayan özel bir tipi belirtir. Kullanım, örneklenebilir somut alt tiplerin seviyesinde belirlenmelidir. Şu durumlarda kullanıcı tarafından tanımlanan tip bilgisi bu özniteliğe girilir: 1) Ön Tanımlı Tip özniteliğinin değeri listeden [[USERDEFINED]] olarak seçilmişse veya 2) somut örneklenmiş varlıkta bir Ön Tanımlı Tip özniteliği yoksa. İkinci durum, bazı istisnai yaprak sınıflarında ve [[IfcBuiltElement]]'i doğrudan örneklendirirken geçerlidir. |
| Açıklama | Attribute: Description | - | Metin | Bilgilendirme amaçlı isteğe bağlı olarak verilen bir tanımdır. |

| Türkçe Tanım (Bilgi Amaçlı) | Özellik | Özellik Seti / İlişki | Veri Tipi (Bilgi Amaçlı) | Açıklama |
|---|---|---|---|---|
|  | `CSBPozNo` | `TREpys_PozOzellikSeti` | Metin | Çevre, Şehircilik ve İklim Değişikliği Bakanlığı İnşaat ve Tesisat Birim Fiyat Poz No'su. |
| Beton Örtüsü | `ConcreteCover` | `Pset_ConcreteElementGeneral` | Sayı | Yerel inşaat yönetmeliklerine göre donatı çubukları üzerindeki beton örtüsü (Türkiye Bina Deprem Yönetmeliğine göre beton örtüsü, boyuna donatı ağırlık merkezi ile en dış beton lifi arasında kalan uzaklıktır) |
| Donatı Dayanım Sınıfı | `ReinforcementStrengthClass` | `Pset_ConcreteElementGeneral` | Metin | Projenin uygulanmakta olduğu beton tasarım standardına göre donatı dayanımının sınıflandırılması. Donatı dayanım sınıfı genellikle dayanım ve deformasyon özelliklerini birleştirir. |

*[PDF sayfa 119]*

## Tablo 6.74 `IfcBridgePart` Sınıfı İsteğe Bağlı Kullanılabilecek Öznitelik ve Özellik Listesi

| Köprü Parçası (`IfcBridgePart`) | Ön Tanımlı Tip Listesi |
|---|---|
| Tanım: Bir köprünün parçası. | ABUTMENT, DECK, DECK SEGMENT, FOUNDATION, PIER, PIER SEGMENT, PYLON, SUB STRUCTURE, SUPER STRUCTURE, SURFACE STRUCTURE, USERDEFINED, NOTDEFINED |

| Türkçe Tanım (Bilgi Amaçlı) | Öznitelik | İlişki | Veri Tipi (Bilgi Amaçlı) | Açıklama |
|---|---|---|---|---|
| Nesne Tipi | Attribute: `ObjectType` | - | Metin | Tip, nesneyi daha ayrıntılı tanımlayan özel bir tipi belirtir. Kullanım, örneklenebilir somut alt tiplerin seviyesinde belirlenmelidir. Şu durumlarda kullanıcı tarafından tanımlanan tip bilgisi bu özniteliğe girilir: 1) Ön Tanımlı Tip özniteliğinin değeri listeden [[USERDEFINED]] olarak seçilmişse veya 2) somut örneklenmiş varlıkta bir Ön Tanımlı Tip özniteliği yoksa. İkinci durum, bazı istisnai yaprak sınıflarında ve [[IfcBuiltElement]]'i doğrudan örneklendirirken geçerlidir. |
| Açıklama | Attribute: Description | - | Metin | Bilgilendirici açıklamalar paylaşmak için isteğe bağlı olarak verilen bir' tanımdır. |

| Türkçe Tanım (Bilgi Amaçlı) | Özellik | Özellik Seti / İlişki | Veri Tipi (Bilgi Amaçlı) | Açıklama |
|---|---|---|---|---|
|  | `CSBPozNo` | `TREpys_PozOzellikSeti` | Metin | Çevre, Şehircilik ve İklim Değişikliği Bakanlığı İnşaat ve Tesisat Birim Fiyat Poz No'su. |
| Beton Örtüsü | `ConcreteCover` | `Pset_ConcreteElementGeneral` | Sayı | Yerel inşaat yönetmeliklerine göre donatı çubukları üzerindeki beton örtüsü (Türkiye Bina Deprem Yönetmeliğine göre beton örtüsü. boyuna donatı ağırlık merkezi ile en dış beton lifi arasında kalan uzaklıktır) |
| Donatı Dayanım Sınıfı | `ReinforcementStrengthClass` | `Pset_ConcreteElementGeneral` | Metin | Projenin uygulanmakta olduğu beton tasarım standardına göre donatı dayanımının sınıflandırılması. Donatı dayanım sınıfı genellikle dayanım ve deformasyon özelliklerini birleştirir. |

*[PDF sayfa 120]*

## Tablo 6.75 `IfcBuilding` Sınıfı İsteğe Bağlı Kullanılabilecek Öznitelik ve Özellik Listesi

| Bina (`IfcBuilding`) | Ön Tanımlı Tip Listesi |
|---|---|
| Tanım: Bir bina, içindekilere veya içeriğine barınak sağlayan ve sabit bir konumda duran bir yapıyı temsil eder. Bina aynı zamanda, bir bina projesinin bileşenleri için (arsa, kat ve mahal ile birlikte) mekânsal yapı hiyerarşisinde temel bir eleman olarak kullanılır. | - |

| Türkçe Tanım (Bilgi Amaçlı) | Öznitelik | İlişki | Veri Tipi (Bilgi Amaçlı) | Açıklama |
|---|---|---|---|---|
| Nesne Tipi | Attribute: `ObjectType` | - | Metin | Tip, nesneyi daha ayrıntılı tanımlayan özel bir tipi belirtir. Kullanım, örneklenebilir somut alt tiplerin seviyesinde belirlenmelidir. Şu durumlarda kullanıcı tarafından tanımlanan tip bilgisi bu özniteliğe girilir: 1) Ön Tanımlı Tip özniteliğinin değeri listeden [[USERDEFINED]] olarak seçilmişse veya 2) somut örneklenmiş varlıkta bir Ön Tanımlı Tip özniteliği yoksa. İkinci durum, bazı istisnai yaprak sınıflarında ve [[IfcBuiltElement]]'i doğrudan örneklendirirken geçerlidir. |
| Açıklama | Attribute: Description | - | Metin | Bilgilendirici açıklamalar paylaşmak için isteğe bağlı olarak verilen bir tanımdır. |

| Türkçe Tanım (Bilgi Amaçlı) | Özellik | Özellik Seti / İlişki | Veri Tipi (Bilgi Amaçlı) | Açıklama |
|---|---|---|---|---|
| Net Hacim | `NetVolume` | `Qto_BuildingBaseQuantities` | Sayı | Nesnenin toplam net hacmi, olası imalat özellikleri (kesit boşluklarıvb.) veya açıklıklar ve girintiler dikkate alınarak hesaplanır. Binanın kapsadığı tüm mahallerin net hacimlerinin toplamıdır. Binadaki yapı elemanlarının hacimlerini içermez. Mekân metrajlarına ek olarak verilebilir. Tutarsızlık durumunda mekânların bireysel miktarları önceliklidir. |
| Sprinkler Sistemi Koruması | `SprinklerProtection` | `Pset_BuildingCommon` | Mantıksal | Bu nesnenin yağmurlama sistemiyle korunduğunu (DOĞRU) veya korunmadığını (YANLIŞ) belirtir. . |
| Sprinkler Sistemi Koruması Otomatiktir | `SprinklerProtectionAutomatic` | `Pset_BuildingCommon` | Mantıksal | Bu nesnenin otomatik yağmurlama sistemiyle korunduğunu (DOĞRU) veya korunmadığını (YANLIŞ) belirtir. Yalnızca "`SprinklerProtection`" özelliği DOĞRU ise belirtilmelidir. |
| Yapım Yılı | `YearOfConstruction` | `Pset_BuildingCommon` | Mantıksal | Binanın inşa yılı, tamamlanması beklenen yılı da dahil eder. |
| Son Yenileme Yıl | `YearOfLastRefurbishment` | `Pset_BuildingCommon` | Mantıksal | Binanın son büyük yenileme veya yeniden yapım yılı (yeniden inşa işlemleri için geçerlidir). |

*[PDF sayfa 121]*

*(Tablo önceki sayfadan sürüyor; sütun başlıkları okunabilirlik için yinelendi.)*

| Türkçe Tanım (Bilgi Amaçlı) | Özellik | Özellik Seti / İlişki | Veri Tipi (Bilgi Amaçlı) | Açıklama |
|---|---|---|---|---|
| Tescilli Yapı Durumu | `IsLandmarked` | `Pset_BuildingCommon` | Mantıksal | Bu binanın tescilli bir tarihi yapı olduğunu (DOĞRU), olmadığını (YANLIŞ) veya bilinmediğini belirtir. |
| Referans Kotu | `ElevationOfRefHeight` | `Pset_BuildingCommon` | Sayı | Tüm kat kotu ölçümleri için kullanılan referans yüksekliğin deniz seviyesine göre kotu; yükseklik 0.0'a eşittir. Genellikle zemin kat seviyesidir. |
| Arazi Kotu | `ElevationOfTerrain` | `Pset_BuildingCommon` | Sayı | Binanın oturum alanı çevresindeki en düşük arazi seviyesinin deniz seviyesine göre yüksekliğidir. |
| İç Konum | `InternalLocation` | `Pset_Address` | Metin | Bir organizasyon tarafından, kurum içi posta teslimatı amacıyla tanımlanan adres. |
| Brüt Hacim | `GrossVolume` | `Qto_BuildingBaseQuantities` | Sayı | Nesnenin toplam brüt hacmi. Açıklıklar, girintiler, kapalı nesneler ve çıkıntılar dikkate alınmaz. Kapsanan tüm mahallerin brüt hacimlerinin toplamıdır. Eleman içindeki yapı elemanlarının hacimlerini içerir. Mekân ve yapı elemanı miktarlarına ek olarak verilebilir. Tutarsızlık durumunda mekân ve yapı elemanlarının bireysel miktarları önceliklidir. |
| Pazar Kategorisi | `MarketCategory` | `Pset_BuildingUse` | Metin | Kullanım kategorisi; örneğin konut, ticari, rekreasyon vb. |
| Pazar Alt Kategorisi | `MarketSubCategory` | `Pset_BuildingUse` | Metin | Kullanım kategorisinin alt kümesi; örneğin çok aileli/birimli, 2 odalı, alçak katlı vb. |
| Posta Kodu | `PostalCode` | `Pset_Address` | Metin | Ülkenin posta servisi tarafından kullanılan kod. |

*[PDF sayfa 122]*

## Tablo 6.76 `IfcBuildingStorey` Sınıfı İsteğe Bağlı Kullanılabilecek Öznitelik ve Özellik Listesi

| Bina Katı (`IfcBuildingStorey`) | Ön Tanımlı Tip Listesi |
|---|---|
| Tanım: Bina katı bir kota sahiptir ve genellikle düşeyde sınırlanmış, (neredeyse) yatay bir mahal grubunu temsil eder. | - |

| Türkçe Tanım (Bilgi Amaçlı) | Öznitelik | İlişki | Veri Tipi (Bilgi Amaçlı) | Açıklama |
|---|---|---|---|---|
| Uzun İsim | Attribute: `LongName` | - | Metin | Referans amaçlı olarak kullanılan bağlamın uzun adı. |
| Bileşim Türü | Attribute: `CompositionType` | - | Metin | Önceden tanımlı mekânsal yapı elemanının kendisini, veya bir bütünü (kompleksi) yoksa bir parçasını mı temsil ettiğini belirtir. Yorum, her mekânsal yapı elemanı alt türü için ayrı ayrı verilir. Eğer bu değer girilmemişse, varsayılan değer "ELEMENT" olarak kabul edilir. |
| Açıklama | Attribute: Description | - | Metin | Bilgilendirme amaçlı isteğe bağlı olarak verilen bir tanımdır. |
| Nesne Tipi | Attribute: `ObjectType` | - | Metin | Tip, nesneyi daha ayrıntılı tanımlayan özel bir tipi belirtir. Kullanım, örneklenebilir somut alt tiplerin seviyesinde belirlenmelidir. Şu durumlarda kullanıcı tarafından tanımlanan tip bilgisi bu özniteliğe girilir: 1) Ön Tanımlı Tip özniteliğinin değeri listeden [[USERDEFINED]] olarak seçilmişse veya 2) somut örneklenmiş varlıkta bir Ön Tanımlı Tip özniteliği yoksa. İkinci durum, bazı istisnai yaprak sınıflarında ve [[IfcBuiltElement]]'i doğrudan örneklendirirken geçerlidir. |

| Türkçe Tanım (Bilgi Amaçlı) | Özellik | Özellik Seti / İlişki | Veri Tipi (Bilgi Amaçlı) | Açıklama |
|---|---|---|---|---|
| Brüt Yükseklik | `GrossHeight` | `Qto_BuildingStoreyBaseQuantities` | Sayı | Bu katın standart brüt yüksekliği, inşaat döşemesinin üst yüzeyinden bir üstteki döşemenin veya çatının üst yüzeyine kadar olan yüksekliktir. Sadece yüksekliğin sabit olduğu durumda belirtilir. |
| Net Yükseklik | `NetHeight` | `Qto_BuildingStoreyBaseQuantities` | Sayı | Bu katın standart net yüksekliği, inşaat döşemesinin üst yüzeyinden bir üstteki döşemenin veya çatının alt yüzeyine kadar olan yüksekliktir. Sadece yüksekliğin sabit olduğu durumda belirtilir. |
| Brüt Kat Alan: | `GrossFloorArea` | `Qto_BuildingStoreyBaseQuantities` | Sayı | Mekânsal yapı elemanı içindeki tüm brüt kat alanlarının toplamıdır. Bina katındaki yapı elemanlarının alanlarını içerir. Katla ilişkilendirilmiş mekân ve yapı elemanlarının miktarlarına ek olarak belirtilebilir. Tutarsızlık durumunda, mekân ve yapı elemanlarının bireysel miktarları geçerlidir. |
| Bitmiş Döşeme Üst Kotu | `ElevationOfFFLRelative` | `Pset_BuildingStoreyCommon` | Sayı | Bitmiş döşeme seviyesinin üst yüzey kotu, yerel sıfır yüksekliğine göre verilen kot değeridir. Seviye değişkenlik gösteriyorsa ve belirgin bir baskın kot bulunmuyorsa, bu özellik atlanabilir. Üst yüzeyin geometrik konumlandırmasıyla herhangi bir tutarsızlık olması durumunda, geometrik gösterim dikkate alınır. |

l

*[PDF sayfa 123]*

## Tablo 6.77 `IfcBurner` Sınıfı İsteğe Bağlı Kullanılabilecek Öznitelik ve Özellik Listesi

| Brülör (`IfcBurner`) | Ön Tanımlı Tip Listesi |
|---|---|
| Tanım: Brülör, yakıtı yanma yoluyla ısıya dönüştüren bir cihazdır. Gaz, yağ ve odun brülörlerini içerir. | USERDEFINED, NOTDEFINED |

| Türkçe Tanım (Bilgi Amaçlı) | Öznitelik | İlişki | Veri Tipi (Bilgi Amaçlı) | Açıklama |
|---|---|---|---|---|
| Nesne Tipi | Attribute: `ObjectType` | - | Metin | Tip, nesneyi daha ayrıntılı tanımlayan özel bir tipi belirtir. Kullanım, örneklenebilir somut alt tiplerin seviyesinde belirlenmelidir. Şu durumlarda kullanıcı tarafından tanımlanan tip bilgisi bu özniteliğe girilir: 1) Ön Tanımlı Tip özniteliğinin değeri listeden [[USERDEFINED]] olarak seçilmişse veya 2) somut örneklenmiş varlıkta bir Ön Tanımlı Tip özniteliği yoksa. İkinci durum, bazı istisnai yaprak sınıflarında ve [[IfcBuiltElement]]'i doğrudan örneklendirirken geçerlidir. |
| Açıklama | Attribute: Description | - | Metin | Bilgilendirme amaçlı isteğe bağlı olarak verilen bir tanımdır. |

| Türkçe Tanım (Bilgi Amaçlı) | Özellik | Özellik Seti / İlişki | Veri Tipi (Bilgi Amaçlı) | Açıklama |
|---|---|---|---|---|
|  | `CSBPozNo` | `TREpys_PozOzellikSeti` | Metin | Çevre, Şehircilik ve İklim Değişikliği Bakanlığı İnşaat ve Tesisat Birim Fiyat Poz No'su. |
| Enerji Kaynağı | `EnergySource` | `Pset_BurnerTypeCommon` | Metin | Yakıt kaynağını veya yakılan enerjiyi tanımlayan liste. |
| Nominal Akım | `RatedCurrent` | `Pset_ElectricalDeviceCommon` | Sayı | Cihazın, tasarımına göre taşıyabileceği akım. |
| Nominal Voltaj | `RatedVoltage` | `Pset_ElectricalDeviceCommon` | Sayı | Bir cihazın (çalışmak üzere) sertifikalandırıldığı izin verilen voltaj aralığı. Bu değerin üst sınırı maksimum kabul edilir. |
| Brüt Ağırlık | `GrossWeight` | `Qto_BurnerBaseQuantities` | Sayı | Nesnenin toplam brüt ağırlığı, ek parçalar olmaksızın ve olası imalat detayları (kesit boşluğu vb.) veya açıklıklar ve girintiler dikkate alınmadan hesaplanır. |

*[PDF sayfa 124]*

## Tablo 6.78 `IfcCableCarrierFitting` Sınıfı İsteğe Bağlı Kullanılabilecek Öznitelik ve Özellik Listesi

| Kablo Taşıyıcı Bağlantı Parçası (`IfcCableCarrierFitting`) | Ön Tanımlı Tip Listesi |
|---|---|
| Tanım: Bir kablo taşıyıcı bağlantı parçası, kablo taşıyıcı sisteminde bir bağlantı noktasına veya geçiş yerine yerleştirilen bağlantı elemanıdır. | BEND, CONNECTOR, JUNCTION, TRANSITION, USERDEFINED, NOTDEFINED |

| Türkçe Tanım (Bilgi Amaçlı) | Öznitelik | İlişki | Veri Tipi (Bilgi Amaçlı) | Açıklama |
|---|---|---|---|---|
| Nesne Tipi | Attribute: `ObjectType` | - | Metin | Tip, nesneyi daha ayrıntılı tanımlayan özel bir tipi belirtir. Kullanım, örneklenebilir somut alt tiplerin seviyesinde belirlenmelidir. Şu durumlarda kullanıcı tarafından tanımlanan tip bilgisi bu özniteliğe girilir: 1) Ön Tanımlı Tip özniteliğinin değeri listeden [[USERDEFINED]] olarak seçilmişse veya 2) somut örneklenmiş varlıkta bir Ön Tanımlı Tip özniteliği yoksa. İkinci durum, bazı istisnai yaprak sınıflarında ve [[IfcBuiltElement]]'i doğrudan örneklendirirken geçerlidir. |
| Açıklama | Attribute: Description | - | Metin | Bilgilendirme amaçlı isteğe bağlı olarak verilen bir tanımdır. |

| Türkçe Tanım (Bilgi Amaçlı) | Özellik | Özellik Seti / İlişki | Veri Tipi (Bilgi Amaçlı) | Açıklama |
|---|---|---|---|---|
|  | `CSBPozNo` | `TREpys_PozOzellikSeti` | Metin | Çevre, Şehircilik ve İklim Değişikliği Bakanlığı İnşaat ve Tesisat Birim Fiyat Poz No'su. |
| Brüt Ağırlık | `GrossWeight` | `Qto_CableCarrierFittingBaseQuantities` | Sayı | Nesnenin toplam brüt ağırlığı, ek parçalar olmaksızın ve olası imalat detayları (kesit boşluğu vb.) veya açıklıklar ve girintiler dikkate alınmadan hesaplanır. |
| Bükülme Açısı | `BendAngle` | `Pset_FittingBend` | Sayı | Akış yönündeki değişim |
| Bükülme Yarıçapı | `BendRadius` | `Pset_FittingBend` | Sayı | Eğer dairesel yay ise bükülme yarıçapı, keskin dönüş ise sıfır. |
| Bağlantı Tipi | `JunctionType` | `Pset_FittingJunction` | Metin | Bağlantı tipi. TEE = 3 çıkışlı. CROSS = 4 çıkışlı. |
| Sol Bağlantı Açısı | `JunctionLeftAngle` | `Pset_FittingJunction` | Sayı | Sol bağlantı için akış yönündeki değişim. |
| Sol Bağlantı Yarıçapı | `JunctionLeftRadius` | `Pset_FittingJunction` | Sayı | Sol bağlantı için bükülme yarıçapı. |
| Sağ Bağlantı Açısı | `JunctionRightAngle` | `Pset_FittingJunction` | Sayı | Sağ bağlantı için akış yönündeki değişim (0 değeri düz segmenti gösterir). |

*[PDF sayfa 125]*

*(Tablo önceki sayfadan sürüyor; sütun başlıkları okunabilirlik için yinelendi.)*

| Türkçe Tanım (Bilgi Amaçlı) | Özellik | Özellik Seti / İlişki | Veri Tipi (Bilgi Amaçlı) | Açıklama |
|---|---|---|---|---|
| Sağ Bağlantı Yarıçapı | `JunctionRightRadius` | `Pset_FittingJunction` | Sayı | Sağ bağlantı için bükülme yarıçapı (0 değeri keskin dönüşü gösterir). |
| Nominal Uzunluk | `NominalLength` | `Pset_FittingTransition` | Sayı | Nesnenin toplam nominal uzunluğu. Boyut bilgisi, şekil temsiline ve kullanılan geometrik parametrelere ek olarak verilir. Ekli özellik setinde sağlanan geometrik parametreler ile boyut özellikleri arasında tutarsızlık olması durumunda, geometrik parametreler dikkate alınır. |
| Y Ekseninde Eksantriklik | `EccentricityInY` | `Pset_FittingTransition` | Sayı | Nokta bağlantısında yer alan iki nokta (veya köşe noktaları) arasındaki y yönündeki mesafe. |
| Z Ekseninde Eksantriklik | `EccentricityInZ` | `Pset_FittingTransition` | Sayı | Nokta bağlantısında yer alan iki nokta (veya köşe noktaları) arasındaki z yönündeki mesafe. |

*[PDF sayfa 126]*

## Tablo 6.79 `IfcCableCarrierSegment` Sınıfı İsteğe Bağlı Kullanılabilecek Öznitelik ve Özellik Listesi

| Kablo Taşıyıcı Segment (`IfcCableCarrierSegment`) | Ön Tanımlı Tip Listesi |
|---|---|
| Tanım: Bir kablo taşıyıcı segmenti, özellikle kabloları taşımak ve desteklemek için kullanılan bir akış segmentidir. | CABLE BRACKET, CABLE LADDER SEGMENT, CABLE TRAY SEGMENT, CABLE TRUNKING SEGMENT, CATENARY WIRE, CONDUIT SEGMENT, DROPPER, USERDEFINED, NOTDEFINED |

| Türkçe Tanım (Bilgi Amaçlı) | Öznitelik | İlişki | Veri Tipi (Bilgi Amaçlı) | Açıklama |
|---|---|---|---|---|
| Nesne Tipi | Attribute: `ObjectType` | - | Metin | Tip, nesneyi daha ayrıntılı tanımlayan özel bir tipi belirtir. Kullanım, örneklenebilir somut alt tiplerin seviyesinde belirlenmelidir. Şu durumlarda kullanıcı tarafından tanımlanan tip bilgisi bu özniteliğe girilir: 1) Ön Tanımlı Tip özniteliğinin değeri listeden [[USERDEFINED]] olarak seçilmişse veya 2) somut örneklenmiş varlıkta bir Ön Tanımlı Tip özniteliği yoksa. İkinci durum, bazı istisnai yaprak sınıflarında ve [[IfcBuiltElement]]'i doğrudan örneklendirirken geçerlidir. |
| Açıklama | Attribute: Description | - | Metin | Bilgilendirme amaçlı isteğe bağlı olarak verilen bir tanımdır. |

| Türkçe Tanım (Bilgi Amaçlı) | Özellik | Özellik Seti / İlişki | Veri Tipi (Bilgi Amaçlı) | Açıklama |
|---|---|---|---|---|
|  | `CSBPozNo` | `TREpys_PozOzellikSeti` | Metin | Çevre, Şehircilik ve İklim Değişikliği Bakanlığı İnşaat ve Tesisat Birim Fiyat Poz No'su. |
| Uzunluk | Length | `Qto_CableCarrierSegmentBaseQuantities` | Sayı | Nesnenin uzunluğu. Kesit ortasında hesaplanır, giriş ve çıkış portları arasındaki mesafeye eşittir. |
| Nominal Genişlik | `NominalWidth` | `Pset_CableCarrierSegmentTypeConduitSegment` | Sayı | Nesnenin toplam nominal genişliği. Boyut bilgisi. şekil temsiline ve kullanılan geometrik parametrelere ek olarak verilir. Ekli özellik setinde sağlanan geometrik parametreler ile boyut özellikleri arasında tutarsızlık olması durumunda, geometrik parametreler dikkate alınır. |
| Nominal Yükseklik | `NominalHeight` | `Pset_CableCarrierSegmentTypeConduitSegment` | Sayı | Nesnenin nominal yüksekliği. Boyut bilgisi, şekil temsiline ve kullanılan geometrik parametrelere ek olarak verilir. Ekli özellik setinde sağlanan geometrik parametreler ile boyut özellikleri arasında tutarsızlık olması durumunda, geometrik parametreler dikkate alınır. |
| Nominal Çap | `NominalDiameter` | `Pset_CableCarrierSegmentTypeConduitSegment` | Sayı | Nesnenin nominal çapı veya genişliği. |

*[PDF sayfa 127]*

*(Tablo önceki sayfadan sürüyor; sütun başlıkları okunabilirlik için yinelendi.)*

| Türkçe Tanım (Bilgi Amaçlı) | Özellik | Özellik Seti / İlişki | Veri Tipi (Bilgi Amaçlı) | Açıklama |
|---|---|---|---|---|
| Brüt Ağırlık | `GrossWeight` | `Qto_CableCarrierSegmentBaseQuantities` | Sayı | Nesnenin toplam brüt ağırlığı, ek parçalar olmaksızın ve olası imalat detayları (kesit boşluğu vb.) veya açıklıklar ve girintiler dikkate alınmadan hesaplanır. |
| Kesit Alanı | `CrossSectionArea` | `Qto_CableCarrierSegmentBaseQuantities` | Sayı | Nesnenin kesitinin (veya profilinin) toplam alanı. |
| Dış Yüzey Alanı | `OuterSurfaceArea` | `Qto_CableCarrierSegmentBaseQuantities` | Sayı | Nesnenin yüzeylerinin toplam alanı (uç kesit alanları dikkate alınmaksızın), genellikle ekstrüzyon durumunda çevre \* uzunluk olarak hesaplanır. |
| İç Çap | `InnerDiameter` | `Qto_ConduitSegmentBaseQuantities` | Sayı | Nesnenin iç çapı. |
| Diş Çap | `OuterDiameter` | `Qto_ConduitSegmentBaseQuantities` | Sayı | Nesnenin dış çapı. |
| Kanal Şekil Tipi | `ConduitShapeType` | `Pset_CableCarrierSegmentTypeConduitSegment` | Metin | Kanal segmentinin şekli. |
| Rijittir | `IsRigid` | `Pset_CableCarrierSegmentTypeConduitSegment` | Mantıksal | Kanalın rijit (= DOĞRU) mi yoksa esnek (= YANLIŞ) mi olduğunu gösterir |
| Akım Taşıma Kapasitesi | `CurrentCarryingCapacity` | `Pset_CableCarrierSegmentTypeDropper` | Sayı | Belirtilen koşullar altında, bir iletkenin, cihazın veya ekipmanın sürekli olarak taşıyabileceği maksimum elektrik akımı; sabit sıcaklığı verilen sınır değeri aşmayacak şekilde. IEC60826-11-13'e dayanır. NOT: Belirtilen sıcaklık, maksimum tasarım ortam sıcaklığıdır. |
| Çekme Dayanımı | `TensileStrength` | `Pset_CableCarrierSegmentTypeDropper` | Sayı | Uygulanan kuvvet altında kırılmaya karşı dayanıklılığını gösterir. |
| Kurulum Yönergesi | `AssemblyInstruction` | `Pset_CableCarrierSegmentTypeDropper` | Metin | Sistem / ekipman / tesisin nasıl monte edildiğini tanımlayan talimatlar. |
| Maksimum Çekme Dayanımı | `UltimateTensileStrength` | `Pset_CableCarrierSegmentTypeDropper` | Sayı | Bir malzemenin veya elemanın gerilme veya çekme sırasında kırılmadan önce dayanabileceği maksimum gerilme. |
| Ayarlanabilirdir | `IsAdjustable` | `Pset_CableCarrierSegmentTypeDropper` | Mantıksal | Elemanın ayarlanabilir olup olmadığını gösterir. |
| Akım Taşıyordur | `IsCurrentCarrying` | `Pset_CableCarrierSegmentTypeDropper` | Mantıksal | Akımın düşey iletkenden geçip geçmeyeceğini belirtir. |
| Nominal Yük | `NominalLoad` | `Pset_CableCarrierSegmentTypeDropper` | Sayı | Bir bileşenin taşıyabileceği nominal yük. |
| Merdiven Konfigürasyonu | `LadderConfiguration` | `Pset_CableCarrierSegmentTypeCableLadderSegment` | Metin | Kullanılan merdiven yapısının konfigürasyonunun açıklaması. |

*[PDF sayfa 128]*

*(Tablo önceki sayfadan sürüyor; sütun başlıkları okunabilirlik için yinelendi.)*

| Türkçe Tanım (Bilgi Amaçlı) | Özellik | Özellik Seti / İlişki | Veri Tipi (Bilgi Amaçlı) | Açıklama |
|---|---|---|---|---|
| Bölme Sayısı | `NumberOfCompartments` | `Pset_CableCarrierSegmentTypeCableTrunkingSegment` | Sayı | Kanal gövdesi (trunking) içindeki ayrı iç bölmelerin sayısı. |
| AC Direnci | `ACResistance` | `Pset_CableCarrierSegmentTypeCatenaryWire` | Metin | AC altındaki direnç. |
| Katenary Tel Türü | `CatenaryWireType` | `Pset_CableCarrierSegmentTypeCatenaryWire` | Sayı | Katenary (askı) telinin tipini belirtir. |
| Isıl Genleşme Katsayısı | `ThermalExpansionCoefficient` | `Pset_CableCarrierSegmentTypeCatenaryWire` | Sayı | Bir gövdenin iki noktası arasındaki 1 mesafesinin, verilen koşullar altında (IEC 113-04-27) termodinamik sıcaklık T ile değişimini tanımlayan nicelik. Oran, Kelvin başına tanımlanır. |
| DC Direnci | `DCResistance` | `Pset_CableCarrierSegmentTypeCatenaryWire` | Sayı | Doğru akım (DC) altında ve 20 °C sıcaklıkta direnç. |
| Büküm Oranı | `LayRatio` | `Pset_CableCarrierSegmentTypeCatenaryWire` | Sayı | Büküm uzunluğu ile tek iletken çapı arasındaki oran. |
| Uzunluk Başına Kütle | `MassPerLength` | `Pset_CableCarrierSegmentTypeCatenaryWire` | Sayı | Uzunluk başına kütle; yani birim uzunluktaki ekstrüzyonun kütlesi (ör. kg/m cinsinden ölçülür). |
| Mekanik Gerilme | `MechanicalTension` | `Pset_CableCarrierSegmentTypeCatenaryWire` | Sayı | Bir akış segmentine uygulanan nominal mekanik kuvvet. |
| Fiziksel Tanım Referansı | `PhysicalDescriptionReference` | `Pset_CableCarrierSegmentTypeCatenaryWire` | Metin | Ekipmanın dış İsteğe Bağlı fiziksel tanımı; örneğin ağırlık. şekil, model, uzunluk, yükseklik, çap. |
| Büküm Yöntemi | `StrandingMethod` | `Pset_CableCarrierSegmentTypeCatenaryWire` | Metin | Kablonun büküm yöntemini belirtir. Büküm (stranding), belirli sayıda tel elemanının ortak bir eksen etrafında sarılarak birleştirilmesi işlemidir. |
| Young Modülü | `YoungModulus` | `Pset_CableCarrierSegmentTypeCatenaryWire` | Sayı | Malzemenin Young modülü elastisite ölçüsü. |

*[PDF sayfa 129]*

## Tablo 6.80 `IfcCaissonFoundation` Sınıfı İsteğe Bağlı Kullanılabilecek Öznitelik ve Özellik Listesi

| Keson Temeli (`IfcCaissonFoundation`) | Ön Tanımlı Tip Listesi |
|---|---|
| Tanım: Keson temeli esasen açık veya kapalı olabilen boş bir kutudur. | WELL, CAISSON, USERDEFINED, NOTDEFINED |

| Türkçe Tanım (Bilgi Amaçlı) | Öznitelik | İlişki | Veri Tipi (Bilgi Amaçlı) | Açıklama |
|---|---|---|---|---|
| Nesne Tipi | Attribute: `ObjectType` | - | Metin | Tip, nesneyi daha ayrıntılı tanımlayan özel bir tipi belirtir. Kullanım, örneklenebilir somut alt tiplerin seviyesinde belirlenmelidir. Şu durumlarda kullanıcı tarafından tanımlanan tip bilgisi bu özniteliğe girilir: 1) Ön Tanımlı Tip özniteliğinin değeri listeden [[USERDEFINED]] olarak seçilmişse veya 2) somut örneklenmiş varlıkta bir Ön Tanımlı Tip özniteliği yoksa. İkinci durum, bazı istisnai yaprak sınıflarında ve [[IfcBuiltElement]]'i doğrudan örneklendirirken geçerlidir. |
| Açıklama | Attribute: Description | - | Metin | Bilgilendirme amaçlı isteğe bağlı olarak verilen bir tanımdır. |

| Türkçe Tanım (Bilgi Amaçlı) | Özellik | Özellik Seti / İlişki | Veri Tipi (Bilgi Amaçlı) | Açıklama |
|---|---|---|---|---|
|  | `CSBPozNo` | `TREpys_PozOzellikSeti` | Metin | Çevre, Şehircilik ve İklim Değişikliği Bakanlığı İnşaat ve Tesisat Birim Fiyat Poz No'su. |

*[PDF sayfa 130]*

## Tablo 6.81 `IfcChiller` Sınıfı İsteğe Bağlı Kullanılabilecek Öznitelik ve Özellik Listesi

| Soğutma Grubu (`IfcChiller`) | Ön Tanımlı Tür |
|---|---|
| Tanım: Soğutma grubu, bir sıvıdaki ısıyı uzaklaştırmak için buhar sıkıştırmalı veya absorbsiyonlu soğutma çevrimini kullanan bir cihazdır. Genellikle su veya su-glikol karışımı olan akışkanı soğutmak için kullanılır. Soğutulmuş akışkan daha sonra bir binadaki havayı soğutmak ve nemini gidermek için kullanılır. | AIR COOLED, WATER COOLED, HEAT RECOVERY, USERDEFINED, NOTDEFINED |

| Türkçe Tanım (Bilgi Amaçlı) | Öznitelik | İlişki | Veri Tipi (Bilgi Amaçlı) | Açıklama |
|---|---|---|---|---|
| Nesne Tipi | Attribute: `ObjectType` | - | Metin | Tip, nesneyi daha ayrıntılı tanımlayan özel bir tipi belirtir. Kullanım, örneklenebilir somut alt tiplerin seviyesinde belirlenmelidir. Şu durumlarda kullanıcı tarafından tanımlanan tip bilgisi bu özniteliğe<br>1) Ön Tanımlı Tip özniteliğinin değeri listeden [[USERDEFINED]] olarak seçilmişse veya 2) somut örneklenmiş varlıkta bir Ön Tanımlı Tip özniteliği yoksa. İkinci durum, bazı istisnai yaprak sınıflarında ve [[IfcBuiltElement]]'i doğrudan örneklendirirken geçerlidir. |
| Açıklama | Attribute: Description | - | Metin | Bilgilendirme amaçlı isteğe bağlı olarak verilen bir tanımdır. |

| Türkçe Tanım (Bilgi Amaçlı) | Özellik | Özellik Seti / İlişki | Veri Tipi (Bilgi Amaçlı) | Açıklama |
|---|---|---|---|---|
|  | `CSBPozNo` | `TREpys_PozOzellikSeti` | Metin | Çevre, Şehircilik ve İklim değişiklii bakanlığı İnşaat ve Tesisat Birin fiyatl Poz No'sudur. |
| Nominal Yoğuşma Sıcaklığı | `NominalCondensingTemperature` | `Pset_ChillerTypeCommon` | Sayı | Soğutucunun yoğuşma sıcaklığı. |
| Nominal Buharlaşma Sıcaklığı | `NominalEvaporatingTemperature` | `Pset_ChillerTypeCommon` | Sayı | Soğutucunun buharlaşma sıcaklığı. |
| Nominal Akım | `RatedCurrent` | `Pset_ElectricalDeviceCommon` | Sayı | Cihazın, tasarımına göre taşıyabileceği akım. |
| Nominal Voltaj | `RatedVoltage` | `Pset_ElectricalDeviceCommon` | Sayı | Bir cihazın (çalışmak üzere) sertifikalandırıldığı izin verilen voltaj aralığı. Bu değerin üst sınırı maksimum kabul edilir. |
| Kapasite Eğrisi | `CapacityCurve` | `Pset_ChillerTypeCommon` | Sayı | Chiller soğutma kapasitesi, yoğuşma sıcaklığı ve buharlaşma sıcaklığının bir fonksiyonudur; veriler tablo formundadır. Kapasite = f (`TempCon`. `TempEvp`), kapasite =a1+b1\*Tei+c1\*Tei\\^2+d1\*Tci+e1\*Tci\\^2+f1\*Tei\*Tci. Bu tablo birden |

*[PDF sayfa 131]*

*(Tablo önceki sayfadan sürüyor; sütun başlıkları okunabilirlik için yinelendi.)*

| Türkçe Tanım (Bilgi Amaçlı) | Özellik | Özellik Seti / İlişki | Veri Tipi (Bilgi Amaçlı) | Açıklama |
|---|---|---|---|---|
|  |  |  |  | fazla giriş değişkeni kullanır; bunu temsil etmek için hem Defining Values hem de `DefinedValues` listeleri null'dur ve `IfcTable`, `IfcResourceConstraintRelationship` ve `IfcMetric` kullanılarak iliştirilir. Sütunlar aşağıdaki sırayla tanımlanmıştır: 1. `IfcPowerMeasure`: Capacity<br>2. `IfcThermodynamicTemperatureMeasure`: `CondensingTemperature` 3. `IfcThermodynamicTemperatureMeasure`: `EvaporatingTemperature` |
| Performans Katsayısı Eğrisi | `CoefficientOfPerformanceCurve` | `Pset_ChillerTypeCommon` | Sayı | Chiller performans katsayısı (COP), yoğuşma sıcaklığı ve buharlaşma sıcaklığının bir fonksiyonudur; veriler tablo formundadır. COP= f (`TempCon`, `TempEvp`), a2+b2\*Tei+c2\*Tei\\^2+d2\*Tci+e2\*Tci\\^2+f2\*Tei\*Tci. Bu tablo birden fazla giriş değişkeni kullanır: bunu temsil etmek için hem Defining Values hem de `DefinedValues` listeleri null'dur ve `IfcTable`, `IfcResourceConstraintRelationship` ve `IfcMetric` kullanılarak iliştirilir. Sütunlar aşağıdaki sırayla tanımlanmıştır: 1. `IfcPositiveRatioMeasure`: `CoefficientOfPerformance` 2. `IfcThermodynamicTemperatureMeasure`: `CondensingTemperature` 3. `IfcThermodynamicTemperatureMeasure`: Evaporating Temperature |
| Tam Yük Orani Tam Yük Oranı Eğrisi | `FullLoadRatioCurve` | `Pset_ChillerTypeCommon` | Sayı | Belirli bir yoğuşma ve buharlaşma sıcaklığında, gerçek güç ile tam yük gücü oranı; kısmi yükün kuadratik bir fonksiyonu olarak. `FraeFullLoadPower` = f(PartLoadRatio). |
| Nominal Verimlilik | `NominalEfficiency` | `Pset_ChillerTypeCommon` | Sayı | Nominal koşullar altında nesnenin nominal verimliliği. |
| Nominal Isı İkame Oranı | `NominalHeatRejectionRate` | `Pset_ChillerTypeCommon` | Sayı | Soğutma etkisinin ve kompresöre gelen güç girdisinin ısıl eşdeğerinin toplamı. |
| Brüt Ağırlık | `GrossWeight` | `Qto_ChillerBaseQuantities` | Sayı | Nesnenin toplam brüt ağırlığı, ek parçalar olmaksızın ve olası imalat detayları (kesit boşluğu vb.) veya açıklıklar ve girintiler dikkate alınmadan hesaplanır. |

*[PDF sayfa 132]*

## Tablo 6.82 `IfcChimney` Sınıfı İsteğe Bağlı Kullanılabilecek Öznitelik ve Özellik Listesi

| Baca (`IfcChimney`) | Ön Tanımlı Tip Listesi |
|---|---|
| Tanım: Bacalar genellikle bir binanın yapısının dikey ya da dikeye yakın bölümleridir ve bina kabuğunun bir parçasını oluştururlar. Genellikle prefabrik veya yerinde dökme betondan inşa edilirler: günümüzde tuğla ile yapılmaları ise nadirdir. | USERDEFINED, NOTDEFINED |

| Türkçe Tanım (Bilgi Amaçlı) | Öznitelik | İlişki | Veri Tipi (Bilgi Amaçlı) | Açıklama |
|---|---|---|---|---|
| Nesne Tipi | Attribute: `ObjectType` | - | Metin | Tip, nesneyi daha ayrıntılı tanımlayan özel bir tipi belirtir. Kullanım, örneklenebilir somut alt tiplerin seviyesinde belirlenmelidir. Şu durumlarda kullanıcı tarafından tanımlanan tip bilgisi bu özniteliğe girilir: 1) Ön Tanımlı Tip özniteliğinin değeri listeden [[USERDEFINED]] olarak seçilmişse veya 2) somut örneklenmiş varlıkta bir Ön Tanımlı Tip özniteliği yoksa. İkinci durum, bazı istisnai yaprak sınıflarında ve [[IfcBuiltElement]]'i doğrudan örneklendirirken geçerlidir. |
| Açıklama | Attribute: Description | - | Metin | Bilgilendirme amaçlı isteğe bağlı olarak verilen bir tanımdır. |
| : Türkçe Tanım (Bilgi Amaçlı) | Özellik | Özellik Seti / İlişki | Veri Tipi (Bilgi Amaçlı) | Açıklama |
| Yangın Dayanım Sınıfı | `FireRating` | `Pset_ChimneyCommon` | Metin | Bu nesnenin yangın dayanım sınıfı. Ulusal yangın güvenliği sınıflandırmasına göre (Türkiye için: Binaların Yangından Korunması Hakkında Yönetmeliğe göre) verilmiştir. |
| Taşıyıcı | `LoadBearing` | `Pset_ChimneyCommon` | Mantıksal | Nesnenin yük taşıma amacıyla tasarlandığını (DOĞRU) veya tasarlanmadığını (YANLIŞ) belirtir. |
| Uzunluk | Length | `Qto_ChimneyBaseQuantities` | Sayı | Nesnenin uzunluğu. Temelden (veya başlangıçtan) en üst noktaya kadar, hiçbir kesit boşluğu veya diğer imalat özelliği dikkate alınmaz. |
| Dışarıda | `IsExternal` | `Pset_ChimneyCommon` | Mantıksal | Yapı elemanının dışarıda kullanıma yönelik tasarlandığını (DOĞRU) veya tasarlanmadığını (YANLIŞ) gösterir. Eğer (DOĞRU) ise, bu bir dış eleman olup binanın dışına bakmaktadır. |
| Baca Kanalı Sayısı | `NumberOfDrafts` | `Pset_ChimneyCommon` | Sayı | Tek bir bacada, hava geçişinin olduğu kesintisiz boşluklar olan baca kanallarının sayısı. |

*[PDF sayfa 133]*

## Tablo 6.83 `IfcCoil` Sınıfı İsteğe Bağlı Kullanılabilecek Öznitelik ve Özellik Listesi

| Serpantin (`IfcCoil`) | Ön Tanımlı Tip Listesi |
|---|---|
| Tanım: Serpantin, birbirine karışmayan akışkanlar arasında ısı transferi sağlamak için kullanılan bir cihazdır. Yaygın bir örnek, yüzeyi boyunca hareket eden havadan ısıyı uzaklaştırmak için soğutulmuş su, antifriz veya soğutucu akışkan dolaştırılan kanatlı bir serpantinin kullanıldığı soğutma serpantinidir. Bir serpantin, bir ısıtma veya soğutma akışkanı taşıyan bir dizi borunun (serpantin) bir hava akımına yerleştirilmesiyle ısıtma veya soğutma amaçları için kullanılabilir. Serpantin, serpantin formunda demetlenmiş borulardan veya genişletilmiş bir ısı transfer yüzeyi sağlayan kanatlı borulardan yapılabilir. | DXCOOLING COIL, ELECTRIC HEATING COIL, GAS HEATING COIL, HYDRONIC COIL, STEAM HEATING COIL, WATER COOLING COIL, WATER HEATING COIL, USERDEFINED, NOTDEFINED |

| Türkçe Tanım (Bilgi Amaçlı) | Öznitelik | İlişki | Veri Tipi (Bilgi Amaçlı) | Açıklama |
|---|---|---|---|---|
| Nesne Tipi | Attribute: `ObjectType` | - | Metin | Tip, nesneyi daha ayrıntılı tanımlayan özel bir tipi belirtir. Kullanım, örneklenebilir somut alt tiplerin seviyesinde belirlenmelidir. Şu durumlarda kullanıcı tarafından tanımlanan tip bilgisi bu özniteliğe girilir: 1) Ön Tanımlı Tip özniteliğinin değeri listeden [[USERDEFINED]] olarak seçilmişse veya 2) somut örneklenmiş varlıkta bir Ön Tanımlı Tip özniteliği yoksa. İkinci durum, bazı istisnai yaprak sınıflarında ve [[IfcBuiltElement]]'i doğrudan örneklendirirken geçerlidir. |
| Açıklama | Attribute: Description | - | Metin | Bilgilendirme amaçlı isteğe bağlı olarak verilen bir tanımdır. |

| Türkçe Tanım (Bilgi Amaçlı) | Özellik | Özellik Seti / İlişki | Veri Tipi (Bilgi Amaçlı) | Açıklama |
|---|---|---|---|---|
|  | `CSBPozNo` | `TREpys_PozOzellikSeti` | Metin | Çevre, Şehircilik ve İklim Değişikliği Bakanlığı İnşaat ve Tesisat Birim Fiyat Poz No'su. |
| Hava Akış Debisi Aralığı | `AirFlowRateRange` | `Pset_CoilTypeCommon` | Sayı | Sağlanabilecek hava debisinin olası aralığı. Serpantin boyunca hava akışının olmadığı durumlarda (örneğin, döşeme plağına gömülü elektrik serpantini gibi), değer sıfırdır. |
| Serpantin Yerleşimi | `CoilPlacement` | `Pset_CoilTypeCommon` | Metin | Serpantinin yerleşimini belirtir. FLOOR, serpantin tipi WATERHEATINGCOIL veya ELECTRICHEATINGCOIL ise, yerden Isıtıcıyı ifade eder. CEILING, serpantin tipi WATERCOOLINGCOIL ise, soğutma tavanını ifade eder. UNIT, serpantinin soğutulmuş kiriş vb. gibi bir soğutma veya ısıtma ünitesinin parçası olduğunu belirtir. |
| Nominal UA | `NominalUA` | `Pset_CoilTypeCommon` | Sayı | Nominal UA değeri (toplam ısı transfer katsayısı × yüzey alanı). |
| Çalışma Sıcaklık Aralığı | `OperationalTemperatureRange` | `Pset_CoilTypeCommon` | Sayı | İzin verilen çalışma hava sıcaklığı aralığı. |

i

*[PDF sayfa 134]*

*(Tablo önceki sayfadan sürüyor; sütun başlıkları okunabilirlik için yinelendi.)*

| Türkçe Tanım (Bilgi Amaçlı) | Özellik | Özellik Seti / İlişki | Veri Tipi (Bilgi Amaçlı) | Açıklama |
|---|---|---|---|---|
| Brüt Ağırlık | `GrossWeight` | `Qto_CoilBaseQuantities` | Sayı | Nesnenin toplam brüt ağırlığı, ek parçalar olmaksızın ve olası imalat detayları (kesit boşluğu vb.) veya açıklıklar ve girintiler dikkate alınmadan hesaplanır. |

*[PDF sayfa 135]*

## Tablo 6.84 `IfcColumn` Sınıfı İsteğe Bağlı Kullanılabilecek Öznitelik ve Özellik Listesi

| Kolon (`IfcColumn`) | Ön Tanımlı Tip Listesi |
|---|---|
| Tanım: Kolon genellikle statik akslarla hizalanan, düşey yapısal veya mimari bir elemandır. Çoğu durumda, basınç yoluyla üst yapıdaki yükü alttaki diğer yapısal elemanlara aktaran düşey ya da düşeye yakın konumlu bir yapısal elemanı temsil eder. Mimari bakış açısından da böyle bir elemanı temsil edebilir; bu durumda yük taşımayan bir eleman olabilir. Bir kolonun yapısal olarak yük taşıyan veya yük taşımayan bir eleman olup olmadığı [[`Pset_ColumnCommon]]-[[LoadBearing`]] özelliğiyle belirlenir. | COLUMN, PIERSTEM, PIERSTEM\_SEGMENT, PILASTER, STANDCOLUMN, USERDEFINED, NOTDEFINED |

| Türkçe Tanım (Bilgi Amaçlı) | Öznitelik | İlişki | Veri Tipi (Bilgi Amaçlı) | Açıklama |
|---|---|---|---|---|
| Nesne Tipi | Attribute: `ObjectType` | - | Metin | Tip, nesneyi daha ayrıntılı tanımlayan özel bir tipi belirtir. Kullanım, örneklenebilir somut alt tiplerin seviyesinde belirlenmelidir. Şu durumlarda kullanıcı tarafından tanımlanan tip bilgisi bu özniteliğe girilir: 1) Ön Tanımlı Tip özniteliğinin değeri listeden [[USERDEFINED]] olarak seçilmişse veya 2) somut örneklenmiş varlıkta bir Ön Tanımlı Tip özniteliği yoksa. İkinci durum, bazı istisnai yaprak sınıflarında ve [[IfcBuiltElement]]'i doğrudan örneklendirirken geçerlidir. |
| Açıklama | Attribute: Description | - | Metin | Bilgilendirme amaçlı isteğe bağlı olarak verilen bir tanımdır. |

| Türkçe Tanım (Bilgi Amaçlı) | Özellik | Özellik Seti / İlişki | Veri Tipi (Bilgi Amaçlı) | Açıklama |
|---|---|---|---|---|
|  | `CSBPozNo` | `TREpys_PozOzellikSeti` | Metin | Çevre, Şehircilik ve İklim Değişikliği Bakanlığı İnşaat ve Tesisat Birim Fiyat Poz No'su. |
| Isıl İletkenlik Katsayısı | `ThermalTransmittance` | `Pset_ColumnCommon` | Sayı | Bir bileşenin, ısı akışı yönünde (tüm malzemeleri içeren) ısı iletkenlik katsayısı (U-Value). |
| Uzunluk | Length | `Qto_ColumnBaseQuantities` | Sayı | Nesnenin uzunluğu. Kesitler veya kesit boşlukları gibi imalat özellikleri dikkate alınmaz. |
| Brüt Ağırlık | `GrossWeight` | `Qto_ColumnBaseQuantities` | Sayı | Nesnenin toplam brüt ağırlığı, ek parçalar olmaksızın ve olası imalat detayları (kesit boşluğu vb.) veya açıklıklar ve girintiler dikkate alınmadan hesaplanır. |
| Net Ağırlık | `NetWeight` | `Qto_ColumnBaseQuantities` | Sayı | Nesnenin toplam net ağırlığı, ek parçalar olmaksızın ve olası imalat detayları (kesit boşluğu vb.) veya açıklıklar ve girintiler dikkate alınmadan hesaplanır. |

*[PDF sayfa 136]*

*(Tablo önceki sayfadan sürüyor; sütun başlıkları okunabilirlik için yinelendi.)*

| Türkçe Tanım (Bilgi Amaçlı) | Özellik | Özellik Seti / İlişki | Veri Tipi (Bilgi Amaçlı) | Açıklama |
|---|---|---|---|---|
| Beton Örtüsü | `ConcreteCover` | `Pset_ConcreteElementGeneral` | Sayı | Yerel inşaat yönetmeliklerine göre donatı çubukları üzerindeki beton örtüsü (Türkiye Bina Deprem Yönetmeliğine göre beton örtüsü, boyuna donatı ağırlık merkezi ile en dış beton lifi arasında kalan uzaklıktır) |
| Donatı Dayanım Sınıfı | `ReinforcementStrengthClass` | `Pset_ConcreteElementGeneral` | Metin | Projenin uygulanmakta olduğu beton tasarım standardına göre donatı dayanımının sınıflandırılması. Donatı dayanım sınıfı genellikle dayanım ve deformasyon özelliklerini birleştirir. |
| Beton Dayanım Sınıfı | `StrengthClass` | `Pset_ConcreteElementGeneral` | Metin | Projede uygulanan beton tasarım standardına göre beton dayanımının sınıflandırması. |
| Eğim | Slope | `Pset_ColumnCommon` | Sayı | Eğim açısı yataya göre relatif (0.0 derece). Şekil bilgisi, şekil temsilinin yanı sıra kullanılan geometrik parametrelerle birlikte sağlanır. Geometrik parametreler ile şekil özellikleri arasında tutarsızlık olması durumunda, geometrik parametreler dikkate alınır. Geometri düzenleme uygulamaları (ör. CAD) için bu değer yalnızca "yazılabilir" olmalıdır. |
| Dışarıda | `IsExternal` | `Pset_ColumnCommon` | Mantıksal | Yapı elemanının dışarıda kullanıma yönelik tasarlandığını (DOĞRU) veya tasarlanmadığını (YANLIŞ) gösterir. Eğer (DOĞRU) ise, bu bir dış eleman olup binanın dışına bakmaktadır. |
| Kesit Alanı | `CrossSectionArea` | `Qto_ColumnBaseQuantities` | Sayı | Nesnenin kesitinin (veya profilinin) toplam alanı. |
| Dış Yüzey Alanı | `OuterSurfaceArea` | `Qto_ColumnBaseQuantities` | Sayı | Nesnenin yüzeylerinin toplam alanı (kolon başlarının alanları dikkate alınmaksızın), genellikle ekstrüzyon durumunda çevre \* uzunluk olarak hesaplanır. |
| Brüt Yüzey Alanı | `GrossSurfaceArea` | `Qto_ColumnBaseQuantities` | Sayı | Nesnenin toplam brüt alanı, genellikle çevre \* uzunluk + 2 \* Kesit Alanı olarak hesaplanır. Bu, Dış Yüzey Alanı + (2 x Kesit Alanı) toplamıdır ve yalnızca Dış Yüzey Alanı ve Kesit Alanı ayrı olarak belirlenemediğinde verilecektir. |
| Net Yüzey Alanı | `NetSurfaceArea` | `Qto_ColumnBaseQuantities` | Sayı | Nesnenin net yüzey alanı genellikle, olası imalat özelliklerini (kesit boşlukları vb.) veya açıklıkları ve girintileri dikkate alarak., çevre \* uzunluk + 2 \* Kesit Alanı olarak hesaplanır. |
| Brüt Hacim | `GrossVolume` | `Qto_ColumnBaseQuantities` | Sayı | Nesnenin toplam brüt hacmi. Açıklıklar, girintiler, kapalı nesneler ve çıkıntılar dikkate alınmaz. |
| Net Hacim | `NetVolume` | `Qto_ColumnBaseQuantities` | Sayı | Nesnenin toplam net hacmi, olası imalat özelliklerini (kesit boşlukları vb.) veya açıklıklar ve girintiler dikkate alınarak hesaplanır. |

*[PDF sayfa 137]*

## Tablo 6.85 `IfcCommunicationsAppliance` Sınıfı İsteğe Bağlı Kullanılabilecek Öznitelik ve Özellik Listesi

| İletişim Cihazı (`IfcCommunicationsAppliance`) | Ön Tanımlı Tip Listesi |
|---|---|
| Tanım: İletişim cihazı, elektronik veya dijital bilgiyi veri ya da ses olarak ileten ve alan bir aygıttır. | ANTENNA, COMPUTER, FAX, GATEWAY, MODEM, NETWORK APPLIANCE, NETWORK BRIDGE, NETWORK HUB, PRINTER, REPEATER, ROUTER, SCANNER, USERDEFINED, NOTDEFINED |

| Türkçe Tanım (Bilgi Amaçlı) | Öznitelik | İlişki | Veri Tipi (Bilgi Amaçlı) | Açıklama |
|---|---|---|---|---|
| Nesne Tipi | Attribute: `ObjectType` | - | Metin | Tip, nesneyi daha ayrıntılı tanımlayan özel bir tipi belirtir. Kullanım, örneklenebilir somut alt tiplerin seviyesinde belirlenmelidir. Şu durumlarda kullanıcı tarafından tanımlanan tip bilgisi bu özniteliğe<br>1) Ön Tanımlı Tip özniteliğinin değeri listeden [[USERDEFINED]] olarak seçilmişse veya 2) somut örneklenmiş varlıkta bir Ön Tanımlı Tip özniteliği yoksa. İkinci durum, bazı istisnai yaprak sınıflarında ve [[IfcBuiltElement]]'i doğrudan örneklendirirken geçerlidir. |
| Açıklama | Attribute: Description | - | Metin | Bilgilendirme amaçlı isteğe bağlı olarak verilen bir tanımdır. |

| Türkçe Tanım (Bilgi Amaçlı) | Özellik | Özellik Seti / İlişki | Veri Tipi (Bilgi Amaçlı) | Açıklama |
|---|---|---|---|---|
|  | `CSBPozNo` | `TREpys_PozOzellikSeti` | Metin | Çevre, Şehircilik ve İklim Değişikliği Bakanlığı İnşaat ve Tesisat Birim Fiyat Poz No'su. |
| Nominal Güç Tüketimi | `NominalPowerConsumption` | `Pset_ElectricalDeviceCommon` | Sayı | Nominal toplam güç tüketimi. |
| Nominal Akım | `RatedCurrent` | `Pset_ElectricalDeviceCommon` | Sayı | Cihazın, tasarımına göre taşıyabileceği akım. |
| Nominal Voltaj | `RatedVoltage` | `Pset_ElectricalDeviceCommon` | Sayı | Bir cihazın (çalışmak üzere) sertifikalandırıldığı izin verilen voltaj aralığı. Bu değerin üst sınırı maksimum kabul edilir. |
| Brüt Ağırlık | `GrossWeight` | `Qto_CommunicationsApplianceBaseQuantities` | Sayı | Nesnenin toplam brüt ağırlığı, ek parçalar olmaksızın ve olası imalat detayları (kesit boşluğu vb.) veya açıklıklar ve girintiler dikkate alınmadan hesaplanır. |

*[PDF sayfa 138]*

## Tablo 6.86 `IfcCondenser` Sınıfı İsteğe Bağlı Kullanılabilecek Öznitelik ve Özellik Listesi

| Kondenser (`IfcCondenser`) | Ön Tanımlı Tip Listesi |
|---|---|
| Tanım: Kondenser, genellikle bir soğutucuyu (refrigerant) gaz halinden sıvı hale yoğunlaştırarak ısıyı dağıtmak için kullanılan bir cihazdır. | AIR COOLED, EVAPORATIVE COOLED, WATER COOLED, WATER COOLED BRAZEDPLATE, WATER COOLED SHELL COIL, WATER COOLED SHELL TUBE, WATER COOLED TUBE IN TUBE, USERDEFINED, NOTDEFINED |

| Türkçe Tanım (Bilgi Amaçlı) | Öznitelik | İlişki | Veri Tipi (Bilgi Amaçlı) | Açıklama |
|---|---|---|---|---|
| Nesne Tipi | Attribute: `ObjectType` | - | Metin | Tip, nesneyi daha ayrıntılı tanımlayan özel bir tipi belirtir. Kullanım, örneklenebilir somut alt tiplerin seviyesinde belirlenmelidir. Şu durumlarda kullanıcı tarafından tanımlanan tip bilgisi bu özniteliğe girilir: Ön Tanımlı Tip özniteliğinin değeri listeden [[USERDEFINED]] olarak seçilmişse veya 2) somut örneklenmiş varlıkta bir Ön Tanımlı Tip özniteliği yoksa. İkinci durum, bazı istisnai yaprak sınıflarında ve [[IfcBuiltElement]]'i doğrudan örneklendirirken geçerlidir. |
| Açıklama | Attribute: Description | - | Metin | Bilgilendirme amaçlı isteğe bağlı olarak verilen bir tanımdır. |

| Türkçe Tanım (Bilgi Amaçlı) | Özellik | Özellik Seti / İlişki | Veri Tipi (Bilgi Amaçlı) | Açıklama |
|---|---|---|---|---|
|  | `CSBPozNo` | `TREpys_PozOzellikSeti` | Metin | Çevre, Şehircilik ve İklim Değişikliği Bakanlığı İnşaat ve Tesisat Birim Fiyat Poz No'su. |
| Nominal Akım | `RatedCurrent` | `Pset_ElectricalDeviceCommon` | Sayı | Cihazın, tasarımına göre taşıyabileceği akım. |
| Nominal Voltaj | `RatedVoltage` | `Pset_ElectricalDeviceCommon` | Sayı | Bir cihazın (çalışmak üzere) sertifikalandırıldığı izin verilen voltaj aralığı. Bu değerin üst sınırı maksimum kabul edilir. |
| Dış Yüzey Alanı | `ExternalSurfaceArea` | `Pset_CondenserTypeCommon` | Sayı | Dış yüzey alanı (birincil ve ikincil alan). |
| İç Soğutucu  | `InternalRefrigerantVolume` | `Pset_CondenserTypeCommon` | Sayı | Nesnenin iç hacmi (soğutucu akışkan tarafı). |
| İç Yüzey Alanı | `InternalSurfaceArea` | `Pset_CondenserTypeCommon` | Sayı | İç yüzey alanı. |
| İç Su Hacmi | `InternalWaterVolume` | `Pset_CondenserTypeCommon` | Sayı | Nesnenin iç hacmi (su tarafı). |
| Nominal Isı Transfer Alan: | `NominalHeatTransferArea` | `Pset_CondenserTypeCommon` | Sayı  | Nominal toplam ısı transfer katsayısı ile ilişkili nominal ısı transfer yüzey alanı. |

*[PDF sayfa 139]*

*(Tablo önceki sayfadan sürüyor; sütun başlıkları okunabilirlik için yinelendi.)*

| Türkçe Tanım (Bilgi Amaçlı) | Özellik | Özellik Seti / İlişki | Veri Tipi (Bilgi Amaçlı) | Açıklama |
|---|---|---|---|---|
| Nominal Isı Transfer Katsayısı | `NominalHeatTransferCoefficient` | `Pset_CondenserTypeCommon` | Sayı | Nominal ısı transfer alanı ile ilişkili nominal toplam ısı transfer katsayısı. |
| Soğutucu Akışkan Sınıfı | `RefrigerantClass` | `Pset_CondenserTypeCommon` | Metin | Nesne tarafından kullanılan soğutucu akışkan sınıfı.; Kloroflorokarbonlar.; Hidrokloroflorokarbonlar.; Hidroflorokarbonlar. |
| Brüt Ağırlık | `GrossWeight` | `Qto_CondenserBaseQuantities` | Sayı | Nesnenin toplam brüt ağırlığı, ek parçalar olmaksızın ve olası imalat detayları (kesit boşluğu vb.) veya açıklıklar ve girintiler dikkate alınmadan hesaplanır. |

*[PDF sayfa 140]*

## Tablo 6.87 `IfcCoolingTower` Sınıfı İsteğe Bağlı Kullanılabilecek Öznitelik ve Özellik Listesi

| Soğutma Kulesi (`IfcCoolingTower`) | Ön Tanımlı Tip Listesi |
|---|---|
| Tanım: Soğutma kulesi, su gibi bir akışkanı dolaştırarak sıcaklığını kısmi buharlaşma yoluyla düşürmek için ısıyı ortam havasına atan bir cihazdır. | NATURAL DRAFT, MECHANICAL INDUCED DRAFT, MECHANICAL FORCED DRAFT, USERDEFINED, NOTDEFINED |

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
| Dış Ortam Tasarım Kuru Termometre Sıcaklığı | `AmbientDesignDryBulbTemperature` | `Pset_CoolingTowerTypeCommon` | Sayı | Soğutma kulesinin seçimi için kullanılan dış ortam tasarım kuru termometre sıcaklığı. |
| Dış Ortam Tasarım Yaş Termometre Sıcaklığı | `AmbientDesignWetBulbTemperature` | `Pset_CoolingTowerTypeCommon` | Sayı | Soğutma kulesinin seçimi için kullanılan dış ortam tasarım yaş termometre sıcaklığı. |

*[PDF sayfa 141]*

*(Tablo önceki sayfadan sürüyor; sütun başlıkları okunabilirlik için yinelendi.)*

| Türkçe Tanım (Bilgi Amaçlı) | Özellik | Özellik Seti / İlişki | Veri Tipi (Bilgi Amaçlı) | Açıklama |
|---|---|---|---|---|
| Hazne Rezerv Hami | `BasinReserveVolume` | `Pset_CoolingTowerTypeCommon` | Sayı | Soğutma kulesi haznesinde işletme ve taşma seviyeleri arasındaki hacim. |
| Kapasite Kontrolü | `CapacityControl` | `Pset_CoolingTowerTypeCommon` | Metin | Görevi kontrol etmek için fan açılıp kapatılır.; Görevi kontrol etmek için fan düşük ve yüksek hız arasında değiştirilir.; Görevi kontrol etmek için fan hızı değiştirilir.; Görevi kontrol etmek için damperler hava akışını düzenler.; Görevi kontrol etmek için baypas vanası su akışını düzenler.; Görevi kontrol etmek için birden fazla seri pompa açılıp kapatılır.; Görevi kontrol etmek için pompa hızı yüksek/düşük arasında değiştirilir.; Görevi kontrol etmek için pompa hızı değiştirilir. |
| Dolaşım Tipi | `CircuitType` | `Pset_CoolingTowerTypeCommon` | Metin | 'Açık Dolaşım (`OpenCircuit`): Su doğrudan soğutma atmosferine maruz bırakılır.; Kapalı Dolaşım (`CloseCircuit`): Akışkan atmosferden bir 151 eşanjörü ile ayrılır.; Islak (Wet): Hava akımı veya ısı değişim yüzeyi buharlaşarak soğutulur.; Kuru (Dry): Hava akımına buharlaşma olmaz.: Kuru Islak (`DryWet`): Kuru kule ile ıslak kulenin birleşimi. |
| Kontrol Stratejisi | `ControlStrategy` | `Pset_CoolingTowerTypeCommon` | Metin | `FixedExitingWaterTemp`: Kapasite, sabit bir çıkış suyu sıcaklığını koruyacak şekilde kontrol edilir. `WetBulbTempReset`: Ayarlanan hedef değer (set noktası), yaş termometre sıcaklığına göre yeniden ayarlanır. |
| Akış Düzeni | `FlowArrangement` | `Pset_CoolingTowerTypeCommon` | Metin | Isı eşanjörü veya soğutma kulesi için temel akış düzenlemelerini tanımlar: Ters Akış — hava ve su akışı farklı yönlerden girer. Çapraz Akış - hava ve su akışı birbirine dik yönde hareket eder. Paralel Akış — hava ve su akışı aynı yönde girer. Çok Geçişli Akış — birden fazla geçişe sahip ısı eşanjörü düzenlemesidir. Diğer — yukarıda tanımlanmayan diğer ısı eşanjörü akış düzenlemelerini ifade eder. |
| Asansör Yükseklik Farkı Kule içi Su Kot Farkı | `LiftElevationDifference` | `Pset_CoolingTowerTypeCommon` | Sayı | Soğutma kulesi çukuru ile kule tepesinin kot farkı. |
| Hücre Sayısı | `NumberofCells` | `Pset_CoolingTowerTypeCommon` | Sayı | Soğutma kulesi ünitesindeki bağımsız soğutma hücrelerinin sayısı. |
| Çalışma Sıcaklık Aralığı | `OperationTemperatureRange` | `Pset_CoolingTowerTypeCommon` | Sayı | İzin verilen çalışma ortam hava sıcaklığ: aralığı. |
| Püskürtme Tipi | `SprayType` | `Pset_CoolingTowerTypeCommon` | Metin | `SprayFilled`: Su, hava akışı içine püskürtülerek dağıtılır., `SplashTypeFill`: Su, ardışık sıçratma çubukları üzerinden kademeli olarak aşağıya doğru akarak soğutulur., `FilmTypeFill`: Su, birbirine yakın levhalar üzerinde ince bir film tabakası halinde akar. |

*[PDF sayfa 142]*

*(Tablo önceki sayfadan sürüyor; sütun başlıkları okunabilirlik için yinelendi.)*

| Türkçe Tanım (Bilgi Amaçlı) | Özellik | Özellik Seti / İlişki | Veri Tipi (Bilgi Amaçlı) | Açıklama |
|---|---|---|---|---|
| Su Gereksinimi | `WaterRequirement` | `Pset_CoolingTowerTypeCommon` | Sayı | Besleme suyu gereksinimi. |
| Brüt Ağırlık | `GrossWeight` | `Qto_CoolingTowerBaseQuantities` | Sayı | Nesnenin toplam brüt ağırlığı, ek parçalar olmaksızın ve olası imalat detayları (kesit boşluğu vb.) veya açıklıklar ve girintiler dikkate alınmadan hesaplanır. |

*[PDF sayfa 143]*

## Tablo 6.88 `IfcCovering` Sınıfı İsteğe Bağlı Kullanılabilecek Öznitelik ve Özellik Listesi

| Kaplama (`IfcCovering`) | Ön Tanımlı Tip Listesi |
|---|---|
| Tanım: Kaplama, başka bir elemanın bir kısmını örten ve tamamen o elemana bağlı olan bir elemandır. `IfcCovering`, bir kaplama türünün oluşumunu tanımlar ve (varsa) bu tür `IfcCoveringType` tarafından ifade edilir. | CEILING, CLADDING, COPING, FLOORING, INSULATION, MEMBRANE, MOLDING, ROOFING, SKIRTING BOARD, SLEEVING, TOPPING, WRAPPING, USERDEFINED, NOTDEFINED |

| Türkçe Tanım (Bilgi Amaçlı) | Öznitelik | İlişki | Veri Tipi (Bilgi Amaçlı) | Açıklama |
|---|---|---|---|---|
| Nesne Tipi | Attribute: `ObjectType` | - | Metin | Tip, nesneyi daha ayrıntılı tanımlayan özel bir tipi belirtir. Kullanım, örneklenebilir somut alt tiplerin seviyesinde belirlenmelidir. Şu durumlarda kullanıcı tarafından tanımlanan tip bilgisi bu özniteliğe girilir: 1) Ön Tanımlı Tip özniteliğinin değeri listeden [[USERDEFINED]] olarak seçilmişse veya 2) somut örneklenmiş varlıkta bir Ön Tanımlı Tip özniteliği yoksa. İkinci durum, bazı istisnai yaprak sınıflarında ve [[IfcBuiltElement]]'i doğrudan örneklendirirken geçerlidir. |
| Açıklama | Attribute: Description | - | Metin | Bilgilendirme amaçlı isteğe bağlı olarak verilen bir tanımdır. |

| Türkçe Tanım (Bilgi Amaçlı) | Özellik | Özellik Seti / İlişki | Veri Tipi (Bilgi Amaçlı) | Açıklama |
|---|---|---|---|---|
|  | `CSBPozNo` | `TREpys_PozOzellikSeti` | Metin | Çevre, Şehircilik ve İklim Değişikliği Bakanlığı İnşaat ve Tesisat Birim Fiyat Poz No'su. |
| Brüt Alan | `GrossArea` | `Qto_CoveringBaseQuantities` | Sayı | Nesnenin brüt alanı. Boşluklar, girintiler, çıkıntılar ve kesit boşlukları dikkate alınmaz. Mekâna bakan kaplamanın tüm brüt alanlarının toplamıdır. |
| Net Alan | `NetArea` | `Qto_CoveringBaseQuantities` | Sayı | Nesnenin toplam net alanı. Açıklıklar, girintiler ve kesit boşlukları çıkarılır; çıkıntılar ise eklenir. Mekâna bakan kaplamanın tüm net alanlarının toplamıdır. |

*[PDF sayfa 144]*

## Tablo 6.89 `IfcCurtainWall` Sınıfı İsteğe Bağlı Kullanılabilecek Öznitelik ve Özellik Listesi

| Giydirme Cephe (`IfcCurtainWall`) | Ön Tanımlı Tip Listesi |
|---|---|
| Tanım: Bir giydirme cephe, zemine oturmaktan ziyade döşeme/çatı strüktürünün kenarına asılmış bileşenlerin bir araya gelmesiyle oluşturulan bir bina duvarıdır. Giydirme cephe. bir bina eleman: bileşimi olarak temsil edilir ve [[`IfcBuiltElement`]] alt tipi olarak `IfcRelAggregates` ilişkisini kullanarak uygulanır. Giydirme cephe genellikle dış cephe olur, ancak [[`Pset_CurtainWallCommon]].[[IsExternal`]] kullanılarak iç mekân giydirme cepheleri de tanımlanabilir. | USERDEFINED, NOTDEFINED |

| Türkçe Tanım (Bilgi Amaçlı) | Öznitelik | İlişki | Veri Tipi (Bilgi Amaçlı) | Açıklama |
|---|---|---|---|---|
| Nesne Tipi | Attribute: `ObjectType` | - | Metin | Tip, nesneyi daha ayrıntılı tanımlayan özel bir tipi belirtir. Kullanım, örneklenebilir somut alt tiplerin seviyesinde belirlenmelidir. Şu durumlarda kullanıcı tarafından tanımlanan tip bilgisi bu özniteliğe girilir: 1) Ön Tanımlı Tip özniteliğinin değeri listeden [[USERDEFINED]] olarak seçilmişse veya 2) somut örneklenmiş varlıkta bir Ön Tanımlı Tip özniteliği yoksa. İkinci durum, bazı istisnai yaprak sınıflarında ve [[IfcBuiltElement]]'i doğrudan örneklendirirken geçerlidir. |
| Açıklama | Attribute: Description | - | Metin | Bilgilendirme amaçlı isteğe bağlı olarak verilen bir tanımdır. |

| Türkçe Tanım (Bilgi Amaçlı) | Özellik | Özellik Seti / İlişki | Veri Tipi (Bilgi Amaçlı) | Açıklama |
|---|---|---|---|---|
|  | `CSBPozNo` | `TREpys_PozOzellikSeti` | Metin | Çevre, Şehircilik ve İklim Değişikliği Bakanlığı İnşaat ve Tesisat Birim Fiyat Poz No'su. |
| Isıl İletkenlik Katsayısı | `ThermalTransmittance` | `Pset_CurtainWallCommon` | Sayı | Bir bileşenin, ısı akışı yönünde (tüm malzemeleri içeren) ısı iletkenlik katsayısı (U-Value). |
| Dışarıda | `IsExternal` | `Pset_CurtainWallCommon` | Mantıksal | Öğenin dışarıda kullanıma yönelik tasarlandığını (DOĞRU) veya tasarlanmadığını (YANLIŞ) gösterir. Eğer (DOĞRU) ise, bu bir dış eleman olup binanın dışına bakmaktadır. |
| Uzunluk | Length | `Qto_CurtainWallQuantities` | Sayı | Nesnenin uzunluğu.<br>Orta hat/eksen boyunca (duvar hattından farklı olsa bile). |
| Genişlik | Width | `Qto_CurtainWallQuantities` | Sayı | Nesnenin genişliği. Sadece, nesne sabit kalınlığa sahipse (prizmatik) verilir. Yalnızca giydirme cephe hattı boyunca sabit olduğu durumda verilmelidir. |
| Yükseklik | Height | `Qto_CurtainWallQuantities` | Sayı | Karakteristik yükseklik.<br>Giydirme cephenin toplam yüksekliği. Yalnızca giydirme cephe boyunca sabitse verilmelidir. |

*[PDF sayfa 145]*

*(Tablo önceki sayfadan sürüyor; sütun başlıkları okunabilirlik için yinelendi.)*

| Türkçe Tanım (Bilgi Amaçlı) | Özellik | Özellik Seti / İlişki | Veri Tipi (Bilgi Amaçlı) | Açıklama |
|---|---|---|---|---|
| Akustik Performans Sınıfı | `AcousticRating` | `Pset_CurtainWallCommon` | Metin | Bu nesne için akustik performans sınıfı.; Ulusal yapı yönetmeliğine göre verilmiştir. Bu nesnenin ses iletim direncini, tam ses yutum değerleri vermek yerine bir indeks oranıyla gösterir. |

*[PDF sayfa 146]*

## Tablo 6.90 `IfcDamper` Sınıfı İsteğe Bağlı Kullanılabilecek Öznitelik ve Özellik Listesi

| Damper (`IfcDamper`) | Ön Tanımlı Tip Listesi |
|---|---|
| Tanım: Bir damper, tipik olarak bir HVAC kanal dağıtım sisteminde yer alır ve hava akışını kontrol etmek veya düzenlemek için kullanılır. | BACK DRAFT DAMPER, BALANCING DAMPER, BLAST DAMPER, CONTROL DAMPER, FIRE DAMPER, FIRE SMOKE DAMPER, FUME HOOD EXHAUST, GRAVITY DAMPER, GRAVITY RELIEF DAMPER, RELIEF DAMPER, SMOKE DAMPER, USERDEFINED, NOTDEFINED |

| Türkçe Tanım (Bilgi Amaçlı) | Öznitelik | İlişki | Veri Tipi (Bilgi Amaçlı) | Açıklama |
|---|---|---|---|---|
| Nesne Tipi | Attribute: `ObjectType` | - | Metin | Tip, nesneyi daha ayrıntılı tanımlayan özel bir tipi belirtir. Kullanım, örneklenebilir somut alt tiplerin seviyesinde belirlenmelidir. Şu durumlarda kullanıcı tarafından tanımlanan tip bilgisi bu özniteliğe girilir: 1) Ön Tanımlı Tip özniteliğinin değeri listeden [[USERDEFINED]] olarak seçilmişse veya 2) somut örneklenmiş varlıkta bir Ön Tanımlı Tip özniteliği yoksa. İkinci durum, bazı istisnai yaprak sınıflarında ve [[IfcBuiltElement]]'i doğrudan örneklendirirken geçerlidir. |
| Açıklama | Attribute: Description | - | Metin | Bilgilendirme amaçlı isteğe bağlı olarak verilen bir tanımdır. |

| Türkçe Tanım (Bilgi Amaçlı) | Özellik | Özellik Seti / İlişki | Veri Tipi (Bilgi Amaçlı) | Açıklama |
|---|---|---|---|---|
|  | `CSBPozNo` | `TREpys_PozOzellikSeti` | Metin | Çevre, Şehircilik ve İklim Değişikliği Bakanlığı İnşaat ve Tesisat Birim Fiyat Poz No'su. |
| Nominal Hava Debisi | `NominalAirFlowRate` | `Pset_DamperTypeCommon` | Sayı | Nominal hava debisi. |
| Nominal Yükseklik | `NominalHeight` | `Pset_ElementSize` | Sayı | Nesnenin nominal yüksekliği. Boyut bilgisi, şekil temsiline ve kullanılan geometrik parametrelere ek olarak verilir. Ekli özellik setinde sağlanan geometrik parametreler ile boyut özellikleri arasında tutarsızlık olması durumunda, geometrik parametreler dikkate alınır. |
| Nominal Uzunluk | `NominalLength` | `Pset_ElementSize` | Sayı | Nesnenin toplam nominal uzunluğu. Boyut bilgisi, şekil temsiline ve kullanılan geometrik parametrelere ek olarak verilir. Ekli özellik setinde sağlanan geometrik parametreler ile boyut özellikleri arasında tutarsızlık olması durumunda, geometrik parametreler dikkate alınır. |
| Nominal Genişlik | `NominalWidth` | `Pset_ElementSize` | Sayı | Nesnenin toplam nominal genişliği. Boyut bilgisi, şekil temsiline ve kullanılan geometrik parametrelere ek olarak verilir. Ekli özellik setinde sağlanan geometrik parametreler ile boyut özellikleri arasında tutarsızlık olması durumunda, geometrik parametreler dikkate alınır. |
| Çalışma | Operation | `Pset_DamperTypeCommon` | Metin | Damperin çalışması için kullanılan çalışma mekanizması. |

*[PDF sayfa 147]*

*(Tablo önceki sayfadan sürüyor; sütun başlıkları okunabilirlik için yinelendi.)*

| Türkçe Tanım (Bilgi Amaçlı) | Özellik | Özellik Seti / İlişki | Veri Tipi (Bilgi Amaçlı) | Açıklama |
|---|---|---|---|---|
| Yönelim? | Orientation | `Pset_DamperTypeCommon` | Sayı veya Metin | Üretici tarafından belirtilen damperin tasarlanan yönelimi. |
| Kanat Kalınlığı | `BladeThickness` | `Pset_DamperTypeCommon` | Sayı | Damper kanadının kalınlığı. |
| Kanat Hareketi | `BladeAction` | `Pset_DamperTypeCommon` | Metin | Kanat hareketi. |
| Kanat Şekli | `BladeShape` | `Pset_DamperTypeCommon` | Metin | Kanat şekli. Düz, üçlü V-oluk anlamına gelir. |
| Kanat Kenarı | `BladeEdge` | `Pset_DamperTypeCommon` | Metin | Kanat kenarı. |
| Kanat Sayısı | `NumberofBlades` | `Pset_DamperTypeCommon` | Sayı | Kanat sayısı. |
| Yüzey Alanı | `FaceArea` | `Pset_DamperTypeCommon` | Sayı | Hava akımına açık olan yüzey alanı. |
| Maksimum Hava Akış Debisi | `MaximumAirFlowRate` | `Pset_DamperTypeCommon` | Sayı | İzin verilen maksimum hava akış debisi. |
| Sıcaklık Aralığı | `TemperatureRange` | `Pset_DamperTypeCommon` | Sayı | İzin verilen maksimum ve minimum sıcaklık. |
| Maksimum Çalışma Basıncı | `MaximumWorkingPressure` | `Pset_DamperTypeCommon` | Sayı | Nesnenin dayanması için üretilmiş olduğu maksimum basınç. |
| Sıcaklık Sınıflandırması | `TemperatureRating` | `Pset_DamperTypeCommon` | Sayı | Sıcaklık sınıflandırması. |
| Açık Konumda Basınç Düşümü | `OpenPressureDrop` | `Pset_DamperTypeCommon` | Sayı | Damper boyunca toplam basınç düşümü. |
| Tam Kapalı Durumda Sızıntı | `LeakageFullyClosed` | `Pset_DamperTypeCommon` | Sayı | Tamamen kapalıyken sızıntı. |
| Kayıp Katsayısı Eğrisi | `LossCoefficentCurve` | `Pset_DamperTypeCommon` | Sayı | Kayıp katsayısı - kanat pozisyon açısı eğrisi; basınç düşümünün hız basıncına oranı, kanat açısına göre; C = f (kanat açısı). |
| Sızıntı Eğrisi | `LeakageCurve` | `Pset_DamperTypeCommon` | Sayı | Sızıntı - basınç düşümü ilişkisi; Sızıntı = f (basınç). |
| Yeniden Üretilen Ses Eğrisi | `RegeneratedSoundCurve` | `Pset_DamperTypeCommon` | Sayı | Yeniden üretilen ses - hava debisi ilişkisi. |
| Çerçeve Tipi | `FrameType` | `Pset_DamperTypeCommon` | Metin | Damperin kullandığı çerçeve tipi (ör. Standart, Tek Flanş, Tek Ters Flanş, Çift Flanş, vb.). |
| Çerçeve Derinliği | `FrameDepth` | `Pset_DamperTypeCommon` | Sayı | Çerçevenin uzunluğu (veya derinliği).<br>Damper için, damper çerçevesinin uzunluğu (veya derinliği). |
| Çerçeve Kalınlığı | `FrameThickness` | `Pset_DamperTypeCommon` | Sayı | Çerçevenin kalınlığı.<br>Damper için, damper çerçevesi malzemesinin kalınlığı. |
| Kapatma Sınıfı | `CloseOffRating` | `Pset_DamperTypeCommon` | Sayı | Kapatma sınıfı. |
| Brüt Ağırlık | `GrossWeight` | `Qto_DamperBaseQuantities` | Sayı | Nesnenin toplam brüt ağırlığı, ek parçalar olmaksızın ve olası imalat detayları (kesit boşluğu vb.) veya açıklıklar ve girintiler dikkate alınmadan hesaplanır. |

*[PDF sayfa 148]*

## Tablo 6.91 `IfcDistributionBoard` Sınıfı İsteğe Bağlı Kullanılabilecek Öznitelik ve Özellik Listesi

| Dağıtım Panosu (`IfcDistributionBoard`) | Ön Tanımlı Tip Listesi |
|---|---|
| Tanım: Dağıtım panosu, belirli bir amaç doğrultusunda elektriksel veya iletişim cihazlarının bir araya getirildiği enerji dağıtımını ve devrelerin kontrolünü sağlayan merkezi bir akış kontrol cihazıdır. | CONSUMER UNIT, DISTRIBUTION BOARD, MOTOR CONTROL CENTRE, SWITCH BOARD, USERDEFINED, NOTDEFINED |

| Türkçe Tanım (Bilgi Amaçlı) | Öznitelik | İlişki | Veri Tipi (Bilgi Amaçlı) | Açıklama |
|---|---|---|---|---|
| Nesne Tipi | Attribute: `ObjectType` | - | Metin | Tip, nesneyi daha ayrıntılı tanımlayan özel bir tipi belirtir. Kullanım, örneklenebilir somut alt tiplerin seviyesinde belirlenmelidir. Şu durumlarda kullanıcı tarafından tanımlanan tip bilgisi bu özniteliğe girilir: 1) Ön Tanımlı Tip özniteliğinin değeri listeden [[USERDEFINED]] olarak seçilmişse veya<br>2) somut örneklenmiş varlıkta bir Ön Tanımlı Tip özniteliği yoksa. İkinci durum, bazı istisnai yaprak sınıflarında ve [[IfcBuiltElement]]'i doğrudan örneklendirirken geçerlidir. |
| Açıklama | Attribute: Description | - | Metin | Bilgilendirme amaçlı isteğe bağlı olarak verilen bir tanımdır. |

| Türkçe Tanım (Bilgi Amaçlı) | Özellik | Özellik Seti / İlişki | Veri Tipi (Bilgi Amaçlı) | Açıklama |
|---|---|---|---|---|
|  | `CSBPozNo` | `TREpys_PozOzellikSeti` | Metin | Çevre, Şehircilik ve İklim Değişikliği Bakanlığı İnşaat ve Tesisat Birim Fiyat Poz No'su. |
| Nominal Akım | `RatedCurrent` | `Pset_ElectricalDeviceCommon` | Sayı | Cihazın, tasarımına göre taşıyabileceği akım. |
| Nominal Voltaj | `RatedVoltage` | `Pset_ElectricalDeviceCommon` | Sayı | Bir cihazın (çalışmak üzere) sertifikalandırıldığı izin verilen voltaj aralığı. Bu değerin üst sınırı maksimum kabul edilir. |
| Ana Dağıtım Noktasıdır | `IsMain` | `Pset_DistributionBoardOccurrence` | Mantıksal | Geçerli örneğin bir elektrik dağıtım hiyerarşisinde ana dağıtım noktası veya en üst seviye olduğunu (= DOĞRU) ya da alt-ana dağıtım noktası olduğunu (= YANLIŞ) tanımlar. |
| Yetkin Operatör Gerektirir | `IsSkilledOperator` | `Pset_DistributionBoardOccurrence` | Mantıksal | Geçerli örnekteki dağıtım panosunda işlemlerin, yetkin veya talimatlandırılmış bir kişi tarafından yapılmasının gerektiğini (=DOĞRU) ya da uygun beceri veya talimat olmayan bir kişi tarafından yapılabileceğini (-YANLIŞ) tanımlar. |
| Nominal Frekans Aralığı | `NominalFrequencyRange` | `Pset_ElectricalDeviceCommon` | Sayı | Cihazın çalışması için sertifikalandırıldığı frekansın alt ve üst sınırları. |
| Güç Faktörü | `PowerFactor` | `Pset_ElectricalDeviceCommon` | Sayı | Cihazın, tasarımına göre taşıyabileceği akım. |
| İletken Fonksiyonu | `ConductorFunction` | `Pset_ElectricalDeviceCommon` | Metin | Yükün bağlandığı iletkenlerin işlevini gösterir. IEC 60446 standardına göre L1, L2 ve L3 faz hatlarını temsil eder (bazı durumlarda faz hatları renk [Kırmızı, Mavi, Sarı] veya numara [1, 2, 3] ile de ifade edilebilir). Koruyucu |

*[PDF sayfa 149]*

*(Tablo önceki sayfadan sürüyor; sütun başlıkları okunabilirlik için yinelendi.)*

| Türkçe Tanım (Bilgi Amaçlı) | Özellik | Özellik Seti / İlişki | Veri Tipi (Bilgi Amaçlı) | Açıklama |
|---|---|---|---|---|
|  |  |  |  | Topraklama (Protective Earth) bazen CPC veya ortak koruyucu iletken olarak da adlandırılır. Elektriksel bir cihaz için bir grup hat iletken işlevi uygulanabilir. |
| Hat Sayısı | `NumberOfPoles` | `Pset_ElectricalDeviceCommon` | Sayı | Nesnenin etkileyeceği kutup sayısı.<br>Cihazın işlemeye/iletmeye yönelik tasarlandığı enerjili hat (faz) sayısı. |
| Koruyucu Topraklama Bağlantısı Mevcut | `HasProtectiveEarth` | `Pset_ElectricalDeviceCommon` | Mantıksal | Nesnenin koruyucu topraklama bağlantısına sahip olduğunu (=DOĞRU) veya olmadığını (-YALIŞ) gösterir. |
| Yalıtım Standardı Sınıfı | `InsulationStandardClass` | `Pset_ElectricalDeviceCommon` | Metin | Yalıtım standart sınıfları, elektrik çarpmasına karşı temel koruma bilgileri sağlar. Yalıtım seviyelerini, yapısal gereksinimler (yüzeysel mesafe ve boşluk mesafeleri) ve elektriksel gereksinimler (elektrik dayanıklılık testlerine uygunluk) açısından tanımlar. Temel yalıtım, tek arıza koşulları altında kısa devre yapmış olarak kabul edilir. Gereken gerçek değerler, yalıtımın maruz kaldığı çalışma voltajına ve diğer faktörlere bağlıdır. Ayrıca, elektrikli cihazın koruyucu toprak bağlantısına sahip olup olmadığını da belirtir. |
| IP Kodu | IP\_Code | `Pset_ElectricalDeviceCommon` | Metin | IP Kodu (Uluslararası Koruma Sınıflandırması, IEC 60529), dış etkilere karşı sağlanan koruma derecesini sınıflandırır ve derecelendirir. |
| IK Kodu | IK\_Code | `Pset_ElectricalDeviceCommon` | Metin | IEC 62262 (2002)'ye göre IK Kodu, elektrikli ekipman muhafazalarının dış mekanik darbelere karşı sağladığı koruma derecesi için sayısal bir sınıflandırmadır.<br>Not: Daha önceki etiketlemede, ilgili IP Koduna (dış etkilere karşı koruma) üçüncü bir rakam (1..) eklenerek darbe koruma seviyesi gösterilmişti. |
| Topraklama Tipi | `EarthingStyle` | `Pset_ElectricalDeviceCommon` | Metin | Elektrikli cihazın topraklama tipini gösterir. |
| Isı Dağılımı | `HeatDissipation` | `Pset_ElectricalDeviceCommon` | Sayı | Elektrik cihazının güç cinsinden ölçülen ısı yayılımını belirtir. |
| Güç | Power | `Pset_ElectricalDeviceCommon` | Sayı | Gerçek güç ve çalışabilir aralık. |
| Güç Besleme Port Sayısı | `NumberOfPowerSupplyPorts` | `Pset_ElectricalDeviceCommon` | Sayı | Elektrikli cihazın güç besleme portlarının sayısını gösterir. |

*[PDF sayfa 150]*

## Tablo 6.92 `IfcDistrubitionSystem` Sınıfı İsteğe Bağlı Kullanılabilecek Öznitelik ve Özellik Listesi

| Dağıtım Sistemi (`IfcDistributionSystem`) | Ön Tanımlı Tip Listesi |
|---|---|
| Tanım: Bir dağıtım sistemi, bir dağıtım ortamının akışını almak, depolamak, sürdürmek, dağıtmak veya kontrol etmek üzere tasarlanmış bir ağdır. Yaygın bir örnek, bir pompa, bir tank ve sıcak suyu terminallere dağıtmak için birbirine bağlı bir boru sisteminden oluşan bir sıcak su ısıtma sistemidir. | AIRCONDITIONING, AUDIOVISUAL, CATENARY\_SYSTEM, CHEMICAL, CHILLEDWATER, COMMUNICATION, COMPRESSEDAIR, CONDENSERWATER, CONTROL, CONVEYING, DATA, DISPOSAL, DOMESTICCOLDWATER, DOMESTICHOTWATER, DRAINAGE, EARTHING, ELECTRICAL, ELECTROACOUSTIC, EXHAUST, FIREPROTECTION, FIXEDTRANSMISSIONNETWORK, FUEL, GAS, HAZARDOUS, HEATING, LIGHTING, LIGHTNINGPROTECTION, MOBILENETWORK, MONITORINGSYSTEM, MUNICIPALSOLIDWASTE, OIL, OPERATIONAL, OPERATIONALTELEPHONYSYSTEM, OVERHEAD\_CONTACTLINE\_SYSTEM, POWERGENERATION, RAINWATER, REFRIGERATION, RETURN\_CIRCUIT, SECURITY, SEWAGE, SIGNAL, STORMWATER, TELEPHONE, TV, VACUUM, VENT, VENTILATION, WASTEWATER, WATERSUPPLY, USERDEFINED, NOTDEFINED |

| Türkçe Tanım (Bilgi Amaçlı) | Öznitelik | İlişki | Veri Tipi (Bilgi Amaçlı) | Açıklama |
|---|---|---|---|---|
| Açıklama | Attribute: Description | - | Metin | Bilgilendirme amaçlı isteğe bağlı olarak verilen bir tanımdır. |
| Nesne Tipi | Attribute: `ObjectType` | - | Metin | Tip, nesneyi daha ayrıntılı tanımlayan özel bir tipi belirtir. Kullanım, örneklenebilir somut alt tiplerin seviyesinde belirlenmelidir. Şu durumlarda kullanıcı tarafından tanımlanan tip bilgisi bu özniteliğe girilir: 1) Ön Tanımlı Tip özniteliğinin değeri listeden [[USERDEFINED]] olarak seçilmişse veya 2) somut örneklenmiş varlıkta bir Ön Tanımlı Tip özniteliği yoksa. İkinci durum, bazı istisnai yaprak sınıflarında ve [[IfcBuiltElement]]'i doğrudan örneklendirirken geçerlidir. |

| Türkçe Tanım | Özellik | Özellik Seti / İlişki | Veri Tipi | Açıklama |
|---|---|---|---|---|
| Elektrik Sistem Tipi | `ElectricalSystemType` | `Pset_DistributionSystemTypeElectrical` | Metin | Elektrik yönetmelikleri doğrultusunda IEC 60364 standardına göre sistem tiplerini tanımlayan tip belirleyicileridir. Bu belirleyiciler, enerji kaynağının ve tesisatın açıkta kalan iletken kısımlarının toprakla (topraklama sistemiyle) ilişkisine bağlı olarak atanır. TN, TN-C, TN-S, TN-C-S, TT ve IT sistem tiplerini içerir. |
| Elektrik Sistem Kategorisi | `ElectricalSystemCategory` | `Pset_DistributionSystemTypeElectrical` | Metin | Devrenin gerilim aralığını IEC standardına göre belirtir. YÜKSEK GERİLİM: >1000V AC veya >1500V DC; DÜŞÜK GERİLİM: 50-1000V AC veya 120-1500V DC; COK DÜŞÜK GERİLİM: &lt;50V AC veya &lt;120V DC. |

*[PDF sayfa 151]*

*(Tablo önceki sayfadan sürüyor; sütun başlıkları okunabilirlik için yinelendi.)*

| Türkçe Tanım | Özellik | Özellik Seti / İlişki | Veri Tipi | Açıklama |
|---|---|---|---|---|
| Çeşitlilik Oranı | Diversity | `Pset_DistributionSystemTypeElectrical` | Sayı | Belirli bir zaman aralığında, bir grup elektrikli cihazın veya tüketicinin eşzamanlı maksimum talebinin, aynı dönemdeki bireysel maksimum taleplerinin toplamına oranıdır. Genellikle bir devreye bağlı cihaz gruplarındaki eşzamanlı yük tahminini ifade eder. |
| Canlı İletken Sayısı | `NumberOfLiveConductors` | `Pset_DistributionSystemTypeElectrical` | Sayı | Bu devre içinde bulunan faz (canlı) iletkenlerin sayısıdır. Bu özellik kullanılabilir veya `ConductorFunction` özelliği (eğer sadece tek iletken varsa) kullanılabilir. |
| İzin Verilen Maksimum Gerilim Düşümü | `MaximumAllowedVoltageDrop` | `Pset_DistributionSystemTypeElectrical` | Sayı | Devre boyunca izin verilen maksimum gerilim düşümüdür; bu değer aşılmamalıdır. Varsayılan olarak tali devrelerde %1.5, son devrelerde %2.5, toplamda %4 sınırı uygulanabilir. |
| Topraklama Döngü Empedansı | `NetImpedance` | `Pset_DistributionSystemTypeElectrical` | Sayı | Devrenin yukarısındaki maksimum topraklama döngü empedansıdır (genellikle Zs olarak ifade edilir). 55°C (130°F) sıcaklık koşulları için geçerlidir. |
| Nominal Voltaj Aralığı | `RatedVoltageRange` | `Pset_DistributionSystemTypeElectrical` | Sayı | Üretici tarafından beyan edilen alt ve üst sınır gerilim değerleriyle ifade edilen nominal voltaj aralığıdır. |
| Tasarım Adi | `DesignName` | `Pset_DistributionSystemTypeVentilation` | Metin | Tasarım değerleri için verilen ad. |
| Kanal Boyutlandırma Yöntemi | `DuctSizingMethod` | `Pset_DistributionSystemTypeVentilation` | Metin | Sistem bileşenlerinin boyutlandırılmasında kullanılacak yöntemi tanımlayan liste. |
| Basınç Sınıfı | `PressureClass` | `Pset_DistributionSystemTypeVentilation` | Sayı | Nesnenin nominal basınç sınıfı. Sistem bileşenlerinin nominal basınç değerini belirtir. |
| Sızdırmazlık Sınıfı | `LeakageClass` | `Pset_DistributionSystemTypeVentilation` | Sayı | Sistem bileşenleri için nominal sızdırmazlık sınıfı. |
| Sürtünme Kaybı | `FrictionLoss` | `Pset_DistributionSystemTypeVentilation` | Sayı | Birim uzunluk başına sürtünmeden kaynaklanan basınç kaybı. (Veri tipi = Basınç/Uzunluk ölçüsü) |
| Fire Oranı | `ScrapFactor` | `Pset_DistributionSystemTypeVentilation` | Sayı | Sac malzeme fire orani. |
| Kanal Sızdırmazlık Malzemesi | `DuctSealant` | `Pset_DistributionSystemTypeVentilation` | Metin | Kanal ve bağlantı parçalarında kullanılan sızdırmazlık malzemesinin türü. |
| Maksimum Hava Hızı | `MaximumVelocity` | `Pset_DistributionSystemTypeVentilation` | Sayı | Kanal veya bağlantı parçasındaki maksimum tasarım hava hızı. |
| En Boy Orani | `AspectRatio` | `Pset_DistributionSystemTypeVentilation` | Sayı | Varsayılan en-boy oranı. |
| Minimum Yükseklik | `MinimumHeight` | `Pset_DistributionSystemTypeVentilation` | Sayı | Dikdörtgen, oval veya dairesel kanallar için minimum kanal yüksekliği. |
| Minimum Genişlik | `MinimumWidth` | `Pset_DistributionSystemTypeVentilation` | Sayı | Oval veya dikdörtgen kanallar için minimum kanal genişliği. |

*[PDF sayfa 152]*

## Tablo 6.93 `IfcDoor` Sınıfı Sınıfı İsteğe Bağlı Kullanılabilecek Öznitelik ve Özellik Listesi

| Kapı (`IfcDoor`) | Ön Tanımlı Tip Listesi |
|---|---|
| Tanım: Kapı, esas olarak insanlar, eşyalar, hayvanlar ve araçlar için kontrollü erişim sağlamak üzere kullanılan bir yapı elemanıdır. Menteşeli, eksenel, kayar ve ayrıca döner kapı veya katlanır kapı yapılarını içerir. | BOOM BARRIER, DOOR, GATE, TRAPDOOR, TURNSTILE, USERDEFINED, NOTDEFINED |

| Türkçe Tanım (Bilgi Amaçlı) | Öznitelik | İlişki | Veri Tipi (Bilgi Amaçlı) | Açıklama |
|---|---|---|---|---|
| Nesne Tipi | Attribute: `ObjectType` | - | Metin | Tip, nesneyi daha ayrıntılı tanımlayan özel bir tipi belirtir. Kullanım, örneklenebilir somut alt tiplerin seviyesinde belirlenmelidir. Şu durumlarda kullanıcı tarafından tanımlanan tip bilgisi bu özniteliğe girilir: 1) Ön Tanımlı Tip özniteliğinin değeri listeden [[USERDEFINED]] olarak seçilmişse veya 2) somut örneklenmiş varlıkta bir Ön Tanımlı Tip özniteliği yoksa. İkinci durum, bazı istisnai yaprak sınıflarında ve [[IfcBuiltElement]]'i doğrudan örneklendirirken geçerlidir. |
| Açıklama | Attribute: Description | - | Metin | Bilgilendirme amaçlı isteğe bağlı olarak verilen bir tanımdır. |
| Malzeme | Attribute(IfcRelAssociatesMaterial>IfcMaterial): Name\* | - | Metin | [`IfcMaterial`], yapı elemanlarını (fiziksel ürünler veya bileşenleri) oluşturmak için kullanılabilecek homojen veya heterojen maddelerdir. |

| Türkçe Tanım (Bilgi Amaçlı) | Özellik | Özellik Seti / İlişki | Veri Tipi (Bilgi Amaçlı) | Açıklama |
|---|---|---|---|---|
|  | `CSBPozNo` | `TREpys_PozOzellikSeti` | Metin | Çevre, Şehircilik ve İklim Değişikliği Bakanlığı İnşaat ve Tesisat Birim Fiyat Poz No'su. |
| Isıl İletkenlik Katsayısı | `ThermalTransmittance` | `Pset_DoorCommon` | Sayı | Bir bileşenin, ısı akışı yönünde (tüm malzemeleri içeren) ısı iletkenlik katsayısı (U-Value). |
| Genişlik | Width | `Qto_DoorBaseQuantities` | Sayı | Nesnenin genişliği. Sadece, nesne sabit kalınlığa (prizmatik) sahipse verilir. Kapı kasasının toplam dış genişliği. Yalnızca dikdörtgen kapılar için tanımlanmalıdır. |
| Yükseklik | Height | `Qto_DoorBaseQuantities` | Sayı | Karakteristik yükseklik. kasasının toplam dış yüksekliği. Yalnızca kapı dikdörtgen biçimindeyse verilmelidir. |
| Toplam Genişlik | `OverallWidth` | Attribute | Sayı | Genişliğin genel ölçüsüdür; kapı açıklığını çevreleyen sınırlayıcı kutunun (bounding box) X boyutunu ifade eder. Eğer belirtilmemişse, Toplam Genişlik değeri, kapının yerleştirildiği OpeningElement'ın geometrik temsilinden alınmalıdır. |
| Engelli Erişimine Uygun | `HandicapAccessible` | `Pset_DoorCommon` | Mantıksal | Bu nesnenin engellilerin erişimine uygun olarak tasarlanıp tasarlanmadığını belirtir. Yerel bina yönetmeliklerine göre engelli erişimine uygun olarak |

*[PDF sayfa 153]*

*(Tablo önceki sayfadan sürüyor; sütun başlıkları okunabilirlik için yinelendi.)*

| Türkçe Tanım (Bilgi Amaçlı) | Özellik | Özellik Seti / İlişki | Veri Tipi (Bilgi Amaçlı) | Açıklama |
|---|---|---|---|---|
|  |  |  |  | değerlendiriliyorsa (DOĞRU), değilse (YANLIŞ) olarak girilir. Ulusal bina yönetmeliği gereklerine göre belirlenir. |
| Akustik Performans Sınıfı | `AcousticRating` | `Pset_DoorCommon` | Metin | Bu nesne için akustik performans sınıfı.; Ulusal yapı yönetmeliğine göre sağlanmıştır. Bu nesnenin ses iletim direncini, tam ses yutum değerleri vermek yerine bir indeks oranıyla gösterir. |
| Alan | Area | `Qto_DoorBaseQuantities` | Sayı | Nesne için hesaplanan alan. Kapının dış kasasının toplam alanıdır. |

**"Attribute:": Standart bir IFC elemanının özniteliği**

\*Attribute (`IfcRelAssociatesMaterial>IfcMaterial`): Name\* bilgisi giriliken: "Malzeme İsmi\_Dayanım" formatı kullanılmalıdır. (Örnek:Çelik\_S355 Beton\_C25)

*[PDF sayfa 154]*

## Tablo 6.94 `IfcDuctFitting` Sınıfı İsteğe Bağlı Kullanılabilecek Öznitelik ve Özellik Listesi

| Kanal Bağlantı Parçası (`IfcDuctFitting`) | Ön Tanımlı Tip Listesi |
|---|---|
| Tanım: Kanal bağlantı parçası, kanallı akış dağıtım sisteminde bir bağlantı veya geçiş elemanıdır ya da kanal segmentlerini bağlamak için kullanılır. Bu, akışkanın yönü ve debisi gibi akış özelliklerinde değişikliklere neden olur. | BEND, CONNECTOR, ENTRY, EXIT, JUNCTION, OBSTRUCTION, TRANSITION, USERDEFINED, NOTDEFINED |

| Türkçe Tanım (Bilgi Amaçlı) | Öznitelik | İlişki | Veri Tipi (Bilgi Amaçlı) | Açıklama |
|---|---|---|---|---|
| Nesne Tipi | Attribute: `ObjectType` | - | Metin | Tip, nesneyi daha ayrıntılı tanımlayan özel bir tipi belirtir. Kullanım, örneklenebilir somut alt tiplerin seviyesinde belirlenmelidir. Şu durumlarda kullanıcı tarafından tanımlanan tip bilgisi bu özniteliğe girilir: 1) Ön Tanımlı Tip özniteliğinin değeri listeden [[USERDEFINED]] olarak seçilmişse veya 2) somut örneklenmiş varlıkta bir Ön Tanımlı Tip özniteliği yoksa. İkinci durum, bazı istisnai yaprak sınıflarında ve [[IfcBuiltElement]]'i doğrudan örneklendirirken geçerlidir. |
| Açıklama | Attribute: Description | - | Metin | Bilgilendirme amaçlı isteğe bağlı olarak verilen bir tanımdır. |
| Malzeme | Attribute(IfcRelAssociatesMaterial>IfcMaterial): Name\* | - | Metin | [`IfcMaterial`], yapı elemanlarını (fiziksel ürünler veya bileşenleri) oluşturmak için kullanılabilecek homojen veya heterojen maddelerdir. |

| Türkçe Tanım (Bilgi Amaçlı) | Özellik | Özellik Seti / İlişki | Veri Tipi (Bilgi Amaçlı) | Açıklama |
|---|---|---|---|---|
|  | `CSBPozNo` | `TREpys_PozOzellikSeti` | Metin | Çevre, Şehircilik ve İklim Değişikliği Bakanlığı İnşaat ve Tesisat Birim Fiyat Poz No'su. |
|  | Debi | `TREpys_PozOzellikSeti` | Sayı | Kanal bağlantı parçasından geçen hava akış miktarını ifade eder. Belirli işletme koşullarında kanal içindeki akışkanın hacimsel debisini gösterir. |
| Basınç Sınıfı | `PressureClass` | `Pset_DuctFittingTypeCommon` | Sayı | Nesnenin nominal basınç sınıfı.<br>Yetkili kurum tarafından tanımlanan basınç sınıflandırması (ör.<br>SMACNA, vb.). |
| Basınç Aralığı | `PressureRange` | `Pset_DuctFittingTypeCommon` | Sayı | İzin verilen maksimum ve minimum çalışma basıncı (ortam basıncına göre). |
| Sıcaklık Aralığı | `TemperatureRange` | `Pset_DuctFittingTypeCommon` | Sayı | İzin verilen maksimum ve minimum sıcaklık. |
| Brüt Kesit Alanı | `GrossCrossSectionArea` | `Qto_DuctFittingBaseQuantities` | Sayı | Kesit alanı.<br>Girişte, kanal bağlantı elemanının kendisini ve iç akış alanını da kapsar. |

*[PDF sayfa 155]*

*(Tablo önceki sayfadan sürüyor; sütun başlıkları okunabilirlik için yinelendi.)*

| Türkçe Tanım (Bilgi Amaçlı) | Özellik | Özellik Seti / İlişki | Veri Tipi (Bilgi Amaçlı) | Açıklama |
|---|---|---|---|---|
| Brüt Ağırlık | `GrossWeight` | `Qto_DuctFittingBaseQuantities` | Sayı | Nesnenin toplam brüt ağırlığı, ek parçalar olmaksızın ve olası imalat detayları (kesit boşluğu detayları (kesit boşluğu vb.) veya açıklıklar ve girintiler dikkate alınmadan hesaplanır. |
| Uzunluk | Length | `Qto_DuctFittingBaseQuantities` | Sayı | Nesnenin uzunluğu.<br>Kesitin orta noktasından hesaplanır ve bağlantı ağzı girişinden bağlantı ağzı çıkışına kadar olan akış yolu boyunca olan mesafeye eşittir.<br>Bağlantı parçaları için, en uzun akış yolunun uzunluğunu gösterir. |
| Net Kesit Alanı | `NetCrossSectionArea` | `Qto_DuctFittingBaseQuantities` | Sayı | Nesnenin kesit alanı.<br>Kanal bağlantı parçaları dahil, iç akış alanı hariç. |
| Dış Yüzey Alanı | `OuterSurfaceArea` | `Qto_DuctFittingBaseQuantities` | Sayı | Nesnenin yüzeylerinin toplam alanı (uç kesit alanları dikkate alınmaksızın), genellikle ekstrüzyon durumunda çevre \* uzunluk olarak hesaplanır. |

\*Attribute (`IfcRelAssociatesMaterial>IfcMaterial`): Name\* bilgisi giriliken: "Malzeme İsmi\_Dayanım" formatı kullanılmalıdır. (Örnek:Çelik\_S355 Beton\_C25)

*[PDF sayfa 156]*

## Tablo 6.95 `IfcDuctSegment` Sınıfı İsteğe Bağlı Kullanılabilecek Öznitelik ve Özellik Listesi

| Kanal Segmenti (`IfcDuctSegment`) | Ön Tanımlı Tip Listesi |
|---|---|
| Tanım: Kanal segmenti, tipik olarak kanal ağı şebekesinin iki bölümünü birleştirmek için kullanılır. | FLEXIBLE SEGMENT, RIGID SEGMENT, USERDEFINED, NOTDEFINED |

| Türkçe Tanım (Bilgi Amaçlı) | Öznitelik | İlişki | Veri Tipi (Bilgi Amaçlı) | Açıklama |
|---|---|---|---|---|
| Nesne Tipi | Attribute: `ObjectType` | - | Metin | Tip, nesneyi daha ayrıntılı tanımlayan özel bir tipi belirtir. Kullanım, örneklenebilir somut alt tiplerin seviyesinde belirlenmelidir. Şu durumlarda kullanıcı tarafından tanımlanan tip bilgisi bu özniteliğe girilir: Tanımlı Tip özniteliğinin değeri listeden [[USERDEFINED]] olarak seçilmişse veya 2) somut örneklenmiş varlıkta bir Ön Tanımlı Tip özniteliği yoksa. İkinci durum, bazı istisnai yaprak sınıflarında ve [[IfcBuiltElement]]'i doğrudan örneklendirirken geçerlidir. |
| Açıklama | Attribute: Description | - | Metin | Bilgilendirme amaçlı isteğe bağlı olarak verilen bir tanımdır. |
| Malzeme | Attribute(IfcRelAssociatesMaterial>IfcMaterial): Name\* | - | Metin | [`IfcMaterial`], yapı elemanlarını (fiziksel ürünler veya bileşenleri) oluşturmak için kullanılabilecek homojen veya heterojen maddelerdir. |

| Türkçe Tanım (Bilgi Amaçlı) | Özellik | Özellik Seti / İlişki | Veri Tipi (Bilgi Amaçlı) | Açıklama |
|---|---|---|---|---|
|  | `CSBPozNo` | `TREpys_PozOzellikSeti` | Metin | Çevre, Şehircilik ve İklim Değişikliği Bakanlığı İnşaat ve Tesisat Birim Fiyat Poz No'su. |
|  | Debi | `TREpys_PozOzellikSeti` | Sayı | Kanal bağlantı parçasından geçen hava akış miktarını ifade eder. Belirli işletme koşullarında kanal içindeki akışkanın hacimsel debisini gösterir. |
| Nominal Çap Veya Genişlik | `NominalDiameterOrWidth` | `Pset_DuctSegmentTypeCommon` | Sayı | Kanal segmentinin nominal çapı veya genişliği. |
| Nominal Yükseklik | `NominalHeight` | `Pset_DuctSegmentTypeCommon` | Sayı | Nesnenin nominal yüksekliği. Boyut bilgisi, şekil temsiline ve kullanılan geometrik parametrelere ek olarak verilir. Ekli özellik setinde sağlanan geometrik parametreler ile boyut özellikleri arasında tutarsızlık olması durumunda, geometrik parametreler dikkate alınır. |
| Uzunluk | Length | `Qto_DuctSegmentBaseQuantities` | Sayı | Nesnenin uzunluğu.<br>Kesit ortasında hesaplanır, giriş ve çıkış portları arasındaki mesafeye eşittir. |
| İç Pürüzlülük Katsayısı | `InteriorRoughnessCoefficient` | `Pset_DuctSegmentOccurrence` | Sayı | Nesnenin malzemesinin iç pürüzlülüğü. |

*[PDF sayfa 157]*

*(Tablo önceki sayfadan sürüyor; sütun başlıkları okunabilirlik için yinelendi.)*

| Türkçe Tanım (Bilgi Amaçlı) | Özellik | Özellik Seti / İlişki | Veri Tipi (Bilgi Amaçlı) | Açıklama |
|---|---|---|---|---|
| İç Kaplamalıdır | `HasLiner` | `Pset_DuctSegmentOccurrence` | Mantıksal | İç kısmında kanal yalıtım kaplaması varsa DOGRU, yoksa YANLIŞ. |
| Renk | Colour | `Pset_DuctSegmentOccurrence` | Metin | Bu nesnenin rengi. |
| Çalışma Basıncı | `WorkingPressure` | `Pset_DuctSegmentTypeCommon` | Sayı | Çalışma basıncı.<br>Yetkili kurum tarafından tanımlanan basınç sınıflandırması (örn.<br>SMACNA, vb.). |
| Güçlendirme Türü | Reinforcement | `Pset_DuctSegmentTypeCommon` | Metin | Kanal segmenti için kullanılan güçlendirme türü (varsa). |
| Basınç Aralığı | `PressureRange` | `Pset_DuctSegmentTypeCommon` | Sayı | İzin verilen maksimum ve minimum çalışma basıncı (ortam basıncına göre). |
| Sıcaklık Aralığı | `TemperatureRange` | `Pset_DuctSegmentTypeCommon` | Sayı | İzin verilen maksimum ve minimum sıcaklık. |
| Kesit Şekli | `CrossSectionShape` | `Pset_DuctSegmentTypeCommon` | Metin | Kesit şekli. Bu şeklin segmentin uzunluğu boyunca aynı olduğunu unutmayın. Değişken şekiller için bunun yerine bir geçiş bağlantı parçası kullanılmalıdır. |
| Boyuna Kenet | `LongitudinalSeam` | `Pset_DuctSegmentTypeCommon` | Metin | Kanal segmentinin boyuna ekseni boyunca kullanılacak dikiş (birleştirme)/kenet tipidir. |
| Güçlendirme Mesafesi | `ReinforcementSpacing` | `Pset_DuctSegmentTypeCommon` | Sayı | Güçlendirme elemanları arasındaki mesafe. |
| Brüt Kesit Alanı | `GrossCrossSectionArea` | `Qto_DuctSegmentBaseQuantities` | Sayı | Kesit alanı.<br>Kanalın kendisini ve iç akış alanını da kapsar. |
| Brüt Ağırlık | `GrossWeight` | `Qto_DuctSegmentBaseQuantities` | Sayı | Nesnenin toplam brüt ağırlığı, ek parçalar olmaksızın ve olası imalat detayları (kesit boşluğu vb.) veya açıklıklar ve girintiler dikkate alınmadan hesaplanır. |
| Net Kesit Alanı | `NetCrossSectionArea` | `Qto_DuctSegmentBaseQuantities` | Sayı | Nesnenin kesit alanı.<br>İç akış alanı hariç. |
| Dış Yüzey Alanı | `OuterSurfaceArea` | `Qto_DuctSegmentBaseQuantities` | Sayı | Nesnenin yüzeylerinin toplam alanı (uç kesit alanları dikkate alınmaksızın), genellikle ekstrüzyon durumunda çevre \* uzunluk olarak hesaplanır. |

\*Attribute (`IfcRelAssociatesMaterial>IfcMaterial`): Name\* bilgisi giriliken: "Malzeme İsmi\_Dayanım" formatı kullanılmalıdır. (Örnek:Çelik\_S355 Beton\_C25)

*[PDF sayfa 158]*

## Tablo 6.96 `IfcDuctSilencer` Sınıfı İsteğe Bağlı Kullanılabilecek Öznitelik ve Özellik Listesi

| Kanal Susturucusu (`IfcDuctSilencer`) | Ön Tanımlı Tip Listesi |
|---|---|
| Tanım: Kanal susturucusu, tipik olarak bir kanal dağıtım sistemine, hava hareketi, fan gürültüsü vb. kaynaklı gürültü seviyelerini bitişik alanda veya susturucu cihazın aşağı akışında azaltmak amacıyla monte edilen bir cihazdır. | FLATOVAL, RECTANGULAR, ROUND, USERDEFINED, NOTDEFINED |

| Türkçe Tanım (Bilgi Amaçlı) | Öznitelik | İlişki | Veri Tipi (Bilgi Amaçlı) | Açıklama |
|---|---|---|---|---|
| Nesne Tipi | Attribute: `ObjectType` | - | Metin | Tip, nesneyi daha ayrıntılı tanımlayan özel bir tipi belirtir. Kullanım, örneklenebilir somut alt tiplerin seviyesinde belirlenmelidir. Şu durumlarda kullanıcı tarafından tanımlanan tip bilgisi bu özniteliğe girilir: 1) Ön Tanımlı Tip özniteliğinin değeri listeden [[USERDEFINED]] olarak seçilmişse veya 2) somut örneklenmiş varlıkta bir Ön Tanımlı Tip özniteliği yoksa. İkinci durum, bazı istisnai yaprak sınıflarında ve [[IfcBuiltElement]]'i doğrudan örneklendirirken geçerlidir. |
| Açıklama | Attribute: Description | - | Metin | Bilgilendirme amaçlı isteğe bağlı olarak verilen bir tanımdır. |

| Türkçe Tanım (Bilgi Amaçlı) | Özellik | Özellik Seti / İlişki | Veri Tipi (Bilgi Amaçlı) | Açıklama |
|---|---|---|---|---|
|  | `CSBPozNo` | `TREpys_PozOzellikSeti` | Metin | Çevre, Şehircilik ve İklim Değişikliği Bakanlığı İnşaat ve Tesisat Birim Fiyat Poz No'su. |
| Hidrolik Çap | `HydraulicDiameter` | `Pset_DuctSilencerTypeCommon` | Sayı | Hidrolik çap. |
| Uzunluk | Length | `Pset_DuctSilencerTypeCommon` | Sayı | Nesnenin uzunluğu. |
| Hava Akış Debisi Aralığı | `AirFlowRateRange` | `Pset_DuctSilencerTypeCommon` | Sayı | Sağlanabilecek hava debisinin olası aralığı |
| Çalışma Basıncı Aralığı | `WorkingPressureRange` | `Pset_DuctSilencerTypeCommon` | Sayı | İzin verilen minimum ve maksimum çalışma basıncı (ortam basıncına göre). |
| Brüt Ağırlık | `GrossWeight` | `Qto_DuctSilencerBaseQuantities` | Sayı | Nesnenin toplam brüt ağırlığı, ek parçalar olmaksızın ve olası imalat detayları (kesit boşluğu vb.) veya açıklıklar ve girintiler dikkate alınmadan hesaplanır. |
| Ağırlık | Weight | `Pset_DuctSilencerTypeCommon` | Sayı | Nesnenin toplam ağırlığı. |
| Sıcaklık Aralığı | `TemperatureRange` | `Pset_DuctSilencerTypeCommon` | Sayı | İzin verilen maksimum ve minimum sıcaklık. |
| Dış Yalıtımlı | `HasExteriorInsulation` | `Pset_DuctSilencerTypeCommon` | Mantıksal | Susturucunun dış yalıtımı varsa DOĞRU, yoksa YANLIŞ. |

*[PDF sayfa 159]*

## Tablo 6.97 `IfcElectricAppliance` Sınıfı İsteğe Bağlı Kullanılabilecek Öznitelik ve Özellik Listesi

| Elektrikli Cihaz (`IfcElectricAppliance`) | Ön Tanımlı Tip Listesi |
|---|---|
| Tanım: Elektrikli cihaz, elektrikle çalışan ve tüketici kullanımına yönelik bir aygıttır. | DISH WASHER, ELECTRIC COOKER, FREE STANDING ELECTRIC HEATER, FREE STANDING FAN, FREE STANDING WATER COOLER, FREE STANDING WATER HEATER, FREEZER, FRIDGE FREEZER, HAND DRYER, KITCHEN MACHINE, MICROWAVE, PHOTOCOPIER, REFRIGERATOR, TUMBLE DRYER, VENDING MACHINE, WASHING MACHINE, USERDEFINED, NOTDEFINED |

| Türkçe Tanım (Bilgi Amaçlı) | Öznitelik | İlişki | Veri Tipi (Bilgi Amaçlı) | Açıklama |
|---|---|---|---|---|
| Nesne Tipi | Attribute: `ObjectType` | - | Metin | Tip, nesneyi daha ayrıntılı tanımlayan özel bir tipi belirtir. Kullanım, örneklenebilir somut alt tiplerin seviyesinde belirlenmelidir. Şu durumlarda kullanıcı tarafından tanımlanan tip bilgisi bu özniteliğe girilir: 1) Ön Tanımlı Tip özniteliğinin değeri listeden [[USERDEFINED]] olarak seçilmişse veya<br>2) somut örneklenmiş varlıkta bir Ön Tanımlı Tip özniteliği yoksa. İkinci durum, bazı istisnai yaprak sınıflarında ve [[IfcBuiltElement]]'i doğrudan örneklendirirken geçerlidir. |
| Açıklama | Attribute: Description | - | Metin | Bilgilendirme amaçlı isteğe bağlı olarak verilen bir tanımdır. |

| Türkçe Tanım (Bilgi Amaçlı) | Özellik | Özellik Seti / İlişki | Veri Tipi (Bilgi Amaçlı) | Açıklama |
|---|---|---|---|---|
|  | `CSBPozNo` | `TREpys_PozOzellikSeti` | Metin | Çevre, Şehircilik ve İklim Değişikliği Bakanlığı İnşaat ve Tesisat Birim Fiyat Poz No'su. |
| Nominal Güç Tüketimi | `NominalPowerConsumption` | `Pset_ElectricalDeviceCommon` | Sayı | Nominal toplam güç tüketimi. |
| Nominal Akım | `RatedCurrent` | `Pset_ElectricalDeviceCommon` | Metin | Cihazın, tasarımına göre taşıyabileceği akım. |
| Nominal Voltaj | `RatedVoltage` | `Pset_ElectricalDeviceCommon` | Metin | Bir cihazın (çalışmak üzere) sertifikalandırıldığı izin verilen voltaj aralığı. Bu değerin üst sınırı maksimum kabul edilir. |
| Brüt Ağırlık | `GrossWeight` | `Qto_ElectricApplianceBaseQuantities` | Sayı | Nesnenin toplam brüt ağırlığı, ek parçalar olmaksızın ve olası imalat detayları (kesit boşluğu vb.) veya açıklıklar ve girintiler dikkate alınmadan hesaplanır. |

*[PDF sayfa 160]*

## Tablo 6.98 `IfcElectricGenerator` Sınıfı İsteğe Bağlı Kullanılabilecek Öznitelik ve Özellik Listesi

| Elektrik Jeneratörü (`IfcElectricGenerator`) | Ön Tanımlı Tip Listesi |
|---|---|
| Tanım: Elektrik jeneratörü, mekanik enerjiyi elektrik enerjisine dönüştüren bir makine olan bir motordur. | CHP, ENGINEGENERATOR, STANDALONE, USERDEFINED, NOTDEFINED |

| Türkçe Tanım (Bilgi Amaçlı) | Öznitelik | İlişki | Veri Tipi (Bilgi Amaçlı) | Açıklama |
|---|---|---|---|---|
| Nesne Tipi | Attribute: `ObjectType` | - | Metin | Tip, nesneyi daha ayrıntılı tanımlayan özel bir tipi belirtir. Kullanım, örneklenebilir somut alt tiplerin seviyesinde belirlenmelidir. Şu durumlarda kullanıcı tarafından tanımlanan tip bilgisi bu özniteliğe girilir: 1) Ön Tanımlı Tip özniteliğinin değeri listeden [[USERDEFINED]] olarak seçilmişse veya 2) somut örneklenmiş varlıkta bir Ön Tanımlı Tip özniteliği yoksa. İkinci durum, bazı istisnai yaprak sınıflarında ve [[IfcBuiltElement]]'i doğrudan örneklendirirken geçerlidir. |
| Açıklama | Attribute: Description | - | Metin | Bilgilendirme amaçlı isteğe bağlı olarak verilen bir tanımdır. |

| Türkçe Tanım (Bilgi Amaçlı) | Özellik | Özellik Seti / İlişki | Veri Tipi (Bilgi Amaçlı) | Açıklama |
|---|---|---|---|---|
|  | `CSBPozNo` | `TREpys_PozOzellikSeti` | Metin | Çevre, Şehircilik ve İklim Değişikliği Bakanlığı İnşaat ve Tesisat Birim Fiyat Poz No'su. |
| Nominal Voltaj | `RatedVoltage` | `Pset_ElectricalDeviceCommon` | Sayı | Bir cihazın (çalışmak üzere) sertifikalandırıldığı izin verilen voltaj aralığı. Bu değerin üst sınırı maksimum kabul edilir. |
| Elektrik Jeneratörü Verimi | `ElectricGeneratorEfficiency` | `Pset_ElectricGeneratorTypeCommon` | Sayı | Çıkış kapasitesinin giriş kapasitesine oranı |
| Başlangıç Akım Faktörü | `StartCurrentFactor` | `Pset_ElectricGeneratorTypeCommon` | Sayı | IEC: Başlangıç akım faktörü, motorun çalıştırılması sırasında oluşacak tepe başlangıç akımının büyüklüğünü tanımlar. `StartCurrentFactor`, `NominalCurrent` (nominal akım) ile çarpılarak başlangıç akımı elde edilir. |
| Brüt Ağırlık | `GrossWeight` | `Qto_ElectricGeneratorBaseQuantities` | Sayı | Nesnenin toplam brüt ağırlığı, ek parçalar olmaksızın ve olası imalat detayları (kesit boşluğu vb.) veya açıklıklar ve girintiler dikkate alınmadan hesaplanır. |

*[PDF sayfa 161]*

## Tablo 6.99 `IfcEvaporator` Sınıfı İsteğe Bağlı Kullanılabilecek Öznitelik ve Özellik Listesi

| Evaporatör (`IfcEvaporator`) | Ön Tanımlı Tip Listesi |
|---|---|
| Tanım: Evaporatör, sıvı haldeki bir soğutucu akışkanın buharlaştığı ve çevresindeki akışkandan 151 emdiği bir cihazdır. | DIRECT EXPANSION, DIRECT EXPANSION SHELL AND TUBE, DIRECT EXPANSION TUBE IN TUBE, DIRECT EXPANSION BRAZED PLATE, FLOODED SHELL AND TUBE, SHELL AND COIL, USERDEFINED, NOTDEFINED |

| Türkçe Tanım (Bilgi Amaçlı) | Öznitelik | İlişki | Veri Tipi (Bilgi Amaçlı) | Açıklama |
|---|---|---|---|---|
| Nesne Tipi | Attribute: `ObjectType` | - | Metin | Tip, nesneyi daha ayrıntılı tanımlayan özel bir tipi belirtir. Kullanım, örneklenebilir somut alt tiplerin seviyesinde belirlenmelidir. Şu durumlarda kullanıcı tarafından tanımlanan tip bilgisi bu özniteliğe girilir: 1) Ön Tanımlı Tip özniteliğinin değeri listeden [[USERDEFINED]] olarak seçilmişse veya 2) somut örneklenmiş varlıkta bir Ön Tanımlı Tip özniteliği yoksa. İkinci durum, bazı istisnai yaprak sınıflarında ve [[IfcBuiltElement]]'i doğrudan örneklendirirken geçerlidir. |
| Açıklama | Attribute: Description | - | Metin | Bilgilendirme amaçlı isteğe bağlı olarak verilen bir tanımdır. |

| Türkçe Tanım (Bilgi Amaçlı) | Özellik | Özellik Seti / İlişki | Veri Tipi (Bilgi Amaçlı) | Açıklama |
|---|---|---|---|---|
|  | `CSBPozNo` | `TREpys_PozOzellikSeti` | Metin | Çevre, Şehircilik ve İklim Değişikliği Bakanlığı İnşaat ve Tesisat Birim Fiyat Poz No'su. |
| Nominal Akım | `RatedCurrent` | `Pset_ElectricalDeviceCommon` | Sayı | Cihazın, tasarımına göre taşıyabileceği akım. |
| Nominal Voltaj | `RatedVoltage` | `Pset_ElectricalDeviceCommon` | Sayı | Bir cihazın (çalışmak üzere) sertifikalandırıldığı izin verilen voltaj aralığı. Bu değerin üst sınırı maksimum kabul edilir. |
| Evaporatör Soğutucusu | `EvaporatorCoolant` | `Pset_EvaporatorTypeCommon` | Metin | Evaporatörde soğutucu olarak kullanılan akışkan. |
| Evaporatör Ortam Türü | `EvaporatorMediumType` | `Pset_EvaporatorTypeCommon` | Metin | Evaporatör, soğutucu akışkanla ısı alışverişi yapmak için sıvı tipinde bir akışkan kullanır.; Evaporatör, soğutucu akışkanla ısı alışverişi yapmak için hava kullanır. |
| Dış Yüzey Alanı | `ExternalSurfaceArea` | `Pset_EvaporatorTypeCommon` | Sayı | Dış yüzey alanı (birincil ve ikincil alan). |

*[PDF sayfa 162]*

*(Tablo önceki sayfadan sürüyor; sütun başlıkları okunabilirlik için yinelendi.)*

| Türkçe Tanım (Bilgi Amaçlı) | Özellik | Özellik Seti / İlişki | Veri Tipi (Bilgi Amaçlı) | Açıklama |
|---|---|---|---|---|
| Le Soğutucu Hacmi | `InternalRefrigerantVolume` | `Pset_EvaporatorTypeCommon` | Sayı | Nesnenin iç hacmi (soğutucu akışkan tarafı). |
| İç Yüzey Alanı | `InternalSurfaceArea` | `Pset_EvaporatorTypeCommon` | Sayı | Evaporatörün iç yüzey alanı. |
| İç Su Hacmi | `InternalWaterVolume` | `Pset_EvaporatorTypeCommon` | Sayı | Nesnenin iç hacmi (su tarafı). |
| Nominal Isı Transfer Alanı | `NominalHeatTransferArea` | `Pset_EvaporatorTypeCommon` | Sayı | Nominal toplam ısı transfer katsayısı ile ilişkili nominal ısı transfer yüzey alanı. |
| Nominal Isı Transfer Katsayısı | `NominalHeatTransferCoefficient` | `Pset_EvaporatorTypeCommon` | Sayı | Nominal ısı transfer alanı ile ilişkili nominal toplam ısı transfer katsayısı. |
| Soğutucu Akışkan Sınıfı | `RefrigerantClass` | `Pset_EvaporatorTypeCommon` | Metin | Nesne tarafından kullanılan soğutucu akışkan sınıfı.; Kloroflorokarbonlar.; Hidrokloroflorokarbonlar.; Hidroflorokarbonlar. |
| Brüt Ağırlık | `GrossWeight` | `Qto_EvaporatorBaseQuantities` | Sayı | Nesnenin toplam brüt ağırlığı, ek parçalar olmaksızın ve olası imalat detayları (kesit boşluğu vb.) veya açıklıklar ve girintiler dikkate alınmadan hesaplanır. |

*[PDF sayfa 163]*

## Tablo 6.100 `IfcFan` Sınıfı İsteğe Bağlı Kullanılabilecek Öznitelik ve Özellik Listesi

| Fan (`IfcFan`) | Ön Tanımlı Tip Listesi |
|---|---|
| Tanım: Fan, bir gaza mekanik iş uygulayan bir cihazdır. Bir fanın tipik kullanımı, bina servisleri hava dağıtım sisteminde hava akışı sağlamaktır. | CENTRIFUGAL AIR FOIL, CENTRIFUGAL BACKWARDI NCLINED CURVED, CENTRIFUGAL FORWARD CURVED, CENTRIFUGAL RADIAL, PROPELLOR AXIAL, TUBE AXIAL, VANE AXIAL, USERDEFINED, NOTDEFINED |

| Türkçe Tanım (Bilgi Amaçlı) | Öznitelik | İlişki | Veri Tipi (Bilgi Amaçlı) | Açıklama |
|---|---|---|---|---|
| Nesne Tipi | Attribute: `ObjectType` | - | Metin | Tip, nesneyi daha ayrıntılı tanımlayan özel bir tipi belirtir. Kullanım, örneklenebilir somut alt tiplerin seviyesinde belirlenmelidir. Şu durumlarda kullanıcı tarafından tanımlanan tip bilgisi bu özniteliğe girilir: Ön Tanımlı Tip özniteliğinin değeri listeden [[USERDEFINED]] olarak seçilmişse veya 2) somut örneklenmiş varlıkta bir Ön Tanımlı Tip özniteliği yoksa. İkinci durum, bazı istisnai yaprak sınıflarında ve [[IfcBuiltElement]]'i doğrudan örneklendirirken geçerlidir. |
| Açıklama | Attribute: Description | - | Metin | Bilgilendirme amaçlı isteğe bağlı olarak verilen bir tanımdır. |

| Türkçe Tanım (Bilgi Amaçlı) | Özellik | Özellik Seti / İlişki | Veri Tipi (Bilgi Amaçlı) | Açıklama |
|---|---|---|---|---|
|  | `CSBPozNo` | `TREpys_PozOzellikSeti` | Metin | Çevre, Şehircilik ve İklim Değişikliği Bakanlığı İnşaat ve Tesisat Birim Fiyat Poz No'su. |
| Nominal Statik Basınç | `NominalStaticPressure` | `Pset_FanTypeCommon` | Sayı | Hava akımı içindeki statik basınç, fanın tasarlanan hava dolaşımını sağlamak için aşması gereken basınçtır. |
| Nominal Güç Oranı | `NominalPowerRate` | `Pset_FanTypeCommon` | Sayı | Nominal fan güç oranı. |
| Nominal Güç Tüketimi | `NominalPowerConsumption` | `Pset_ElectricalDeviceCommon` | Sayı | Nominal toplam güç tüketimi. |
| Nominal Akım | `RatedCurrent` | `Pset_ElectricalDeviceCommon` | Sayı | Cihazın, tasarımına göre taşıyabileceği akım. |
| Nominal Voltaj | `RatedVoltage` | `Pset_ElectricalDeviceCommon` | Sayı | Bir cihazın (çalışmak üzere) sertifikalandırıldığı izin verilen voltaj aralığı. Bu değerin üst sınırı maksimum kabul edilir. |
| Çalışma Sıcaklık Aralığı | `OperationTemperatureRange` | `Pset_FanTypeCommon` | Sayı | İzin verilen çalışma ortam hava sıcaklığı aralığı. |

*[PDF sayfa 164]*

*(Tablo önceki sayfadan sürüyor; sütun başlıkları okunabilirlik için yinelendi.)*

| Türkçe Tanım (Bilgi Amaçlı) | Özellik | Özellik Seti / İlişki | Veri Tipi (Bilgi Amaçlı) | Açıklama |
|---|---|---|---|---|
| Hava Çıkış Türü | `DischargeType` | `Pset_FanOccurrence` | Metin | Fan çıkışındaki bağlantı tipini tanımlar.<br>Duct: Kanal tesisatına çıkış. Screen: Izgara çıkışına çıkış. Louver: Panjur çıkışına çıkış. Damper: Damper çıkışına çıkış. |
| Motor Konumu | `MotorPosition` | `Pset_FanOccurrence` | Metin | Motorun hava akımına göre konumunu tanımlar.<br>`InAirStream` (Havaakımında): Fan motoru hava akımı içindedir, `OutOfAirStream` (Havaakınıdışında): Fan motoru hava akımı dışındadır. |
| Motor Tahrik Tipi | `MotorDriveType` | `Pset_FanTypeCommon` | Metin | Doğrudan tahrik, Kayış tahriki, Bağlantı, Diğer motor tahrik tipi, Bilinmeyen motor tahrik tipi. |
| Kapasite Kontrol Türü | `CapacityControlType` | `Pset_FanTypeCommon` | Metin | Giriş kanadı ayarı ile kontrol, Değişken hızlı sürücü ile kontrol, Kanat açı ayarı ile kontrol, Yüksek ve düşük hız arasında geçiş ile kontrol. Çıkış damperi modülasyonu ile kontrol. |
| Nominal Dönme Hızı | `NominalRotationSpeed` | `Pset_FanTypeCommon` | Sayı | Nominal koşullar altında nesnenin dönme hızı.<br>Nominal koşullar altında fan çarkı hızı. |
| Çalışma Kriteri | `OperationalCriteria` | `Pset_FanTypeCommon` | Sayı veya Metin | Maksimum çalışma ortam sıcaklığında çalışma süresi. |
| Basınç Eğrisi | `PressureCurve` | `Pset_FanTypeCommon` | Sayı | Basınç artışı = f (debi). |
| Verim Eğrisi | `EfficiencyCurve` | `Pset_FanTypeCommon` | Sayı | Fan verimliliği = f (debi). |
| Brüt Ağırlık | `GrossWeight` | `Qto_FanBaseQuantities` | Sayı | Nesnenin toplam brüt ağırlığı, ek parçalar olmaksızın ve olası imalat detayları (kesit boşluğu vb.) veya açıklıklar ve girintiler dikkate almadan hesaplanır. |
| Fan Uygulama Türü | `ApplicationOfFan` | `Pset_FanOccurrence` | Metin | Fanın fonksiyonel uygulaması.<br>Besleme havası fanı, Dönüş havası fanı, Egzoz havası fanı, Yukarıda tanımlanmamış diğer uygulama türü. |
| Serpantin Pozisyonu | `CoilPosition` | `Pset_FanOccurrence` | Metin | Bir fan ile serpantin arasındaki ilişkiyi tanımlar. Fan, serpantinin aşağı akışında bulunur, Fan, serpantinin yukarı akışında bulunur. |
| Fan Montaj Türü | `FanMountingType` | `Pset_FanOccurrence` | Metin | Fanın binadaki montaj yöntemini tanımlar. |
| Motor Isısının Hava Akımına Oranı | `FractionOfMotorHeatToAirStream` | `Pset_FanOccurrence` | Tamsayı | Motor ısısının akışkana salınan kısmı. |
| Pervane Çapı | `ImpellerDiameter` | `Pset_FanOccurrence` | Tamsayı | Nesnenin çapı - geometrik olarak benzer nesnelerin performansını ölçeklendirmek için kullanılır. |
