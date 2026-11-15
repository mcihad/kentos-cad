<!-- Kaynak: Resmî Gazete 5.8.2026/33331, Dijital Hazırlama Yönetmeliği ekleri (EK-1…EK-9), PDF s. 29–34. Taranmış sayfalardan OCR ile okundu; yöntem, doğrulama kapsamı ve sınırlar için README.md. -->

*[PDF sayfa 29]*

# EK-5 BIM MODELİNDEKİ IFC SINIFLARI VE VARLIK İSİMLENDİRME ESASLARI

**5.1-** Tasarımda bulunması durumunda BIM modelinde tanımlanması zorunlu olan varlıklara ait IFC sınıf listesi Tablo 5.1'de verilmiştir.

**5.2-** BIM modelinde yer alması isteğe bağlı olan varlıklara ait IFC sınıf listesi Tablo 5.2'de verilmiştir.

**5.3-** Söz konusu varlıklar, IFC formatına dönüştürüldüğünde Tablo 5.1 ve Tablo 5.2'de belirtilen IFC sınıfları kullanılarak temsil edilir.

**5.4-** Tablo 5.1 ile Tablo 5.2'de yer alan IFC sınıfları, IFC 4.3 versiyonu esas alınarak düzenlenmiştir.

**5.5-** BIM modelinde yer alan varlıklar Tablo 5.3'de verilen şablona göre isimlendirilir. Şablondaki her bir bilgi alanı orta tire "-" işareti ile ayrılır.

**5.5.1-** "Disiplin", varlığın ait olduğu disiplini temsil eder. İki karakterli alfabetik kısaltma ile ifade edilir. Disiplin adları ve kısaltmaları EK-2 Tablo 2.2'de verilmiştir.

**5.5.2-** "Kategori Kodu" varlığın IFC sınıfının Türkçe karşılığını temsil eder. Tablo 5.1'de ve Tablo 5.2'de verilen üç karakterli alfabetik kısaltma ile ifade edilir.

**5.5.3-** "Açıklama" yapı elemanına ait tanımlayıcı bilgilerin girildiği alandır. Karakter sınırlaması yoktur. Alfanümerik ifadeler kullanılabilir. Alt kategori, fonksiyon, ölçü, malzeme ve benzeri bilgiler buraya yazılabilir. Her yeni bilgi alanı boşluk bırakmaksızın alt tire "\_" işareti ile ayrılır. Bazı IFC sınıflarının "Ön Tanımlı Tipleri" çok sayıda varlık içerdiğinden, bu tiplerin Türkçe karşılıkları verilmez. Gerekli görüldüğü takdirde, bu tip bilgiler "Açıklama" alanında verilebilir. Açıklama kısmında orta tire "-" kullanılmaz.

**5.5.4-** BIM modelinde mahal isimlendirmesi yapılırken şablondaki açıklama bölümünde, EK-3'te tarif edilen mahal numarası ve mahal ismi ayrı ayrı veya beraber kullanılabilir.

## Tablo 5.1 Zorunlu IFC sınıfları İçin Kategori Kodu Listesi

|  | IFC Sınıfı (`IfcClass`) | IFC Sınıfının Türkçe Karşılığı (Bilgi için) | Kategori Kodu |
|---|---|---|---|
| 1 | `IfcAirTerminal` | Havalandırma Menfezi | HVM |
| 2 | `IfcAudioVisualAppliance` | Görsel ve Ses Cihaz | GVS |
| 3 | `IfcBeam` | Kiriş | KRS |
| 4 | `IfcBoiler` | Kazan | KZN |
| 5 | `IfcBridge` | Köprü | KPR |
| 6 | `IfcBridgePart` | Köprü Parçası | KPP |
| 7 | `IfcBuilding` | Bina | BNA |
| 8 | `IfcBuildingStorey` | Bina Katı | BNK |
| 9 | `IfcBurner` | Brülör | BRL |
| 10 | `IfcCableCarrierFitting` | Kablo Taşıyıcı Bağlantı Parçası | KTB |
| 11 | `IfcCableCarrierSegment` | Kablo Taşıyıcı Segment | KTS |
| 12 | `IfcCaissonFoundation` | Keson Temeli | KST |
| 13 | `IfcChiller` | Soğutma Grubu | SGT |

*[PDF sayfa 30]*

*(Tablo önceki sayfadan sürüyor; sütun başlıkları okunabilirlik için yinelendi.)*

|  | IFC Sınıfı (`IfcClass`) | IFC Sınıfının Türkçe Karşılığı (Bilgi için) | Kategori Kodu |
|---|---|---|---|
| 14 | `IfcChimney` | Baca | BAC |
| 15 | `IfcCoil` | Serpantin | SRP |
| 16 | `IfcColumn` | Kolon | KLN |
| 17 | `IfcCommunicationsAppliance` | İletişim Cihazı | ILT |
| 18 | `IfcCondenser` | Kondenser | KND |
| 19 | `IfcCoolingTower` | Soğutma Kulesi | SKL |
| 20 | `IfcCovering` | Kaplama | KPL |
| 21 | `IfcCurtainWall` | Giydirme Cephe | GDC |
| 22 | `IfcDamper` | Damper | DMP |
| 23 | `IfcDistributionBoard` | Dağıtım Panosu | DPN |
| 24 | `IfcDistributionSystem` | Dağıtım Sistemi | DGS |
| 25 | `IfcDoor` | Kapı | KAP |
| 26 | `IfcDuctFitting` | Kanal Bağlantı Parçası | KBP |
| 27 | `IfcDuctSegment` | Kanal Segmenti | KNS |
| 28 | `IfcDuctSilencer` | Kanal Susturucusu | KSS |
| 29 | `IfcElectricAppliance` | Elektrikli Cihaz | ECZ |
| 30 | `IfcElectricGenerator` | Elektrik Jeneratörü | EJN |
| 31 | `IfcEvaporator` | Evaporatör | EVP |
| 32 | `IfcFan` | Fan | FAN |
| 33 | `IfcFireSuppressionTerminal` | Yangın Söndürme Terminali | YST |
| 34 | `IfcFooting` | Temel | TML |
| 35 | `IfcFurniture` | Mobilya | MBL |
| 36 | `IfcGridAxis` | Aks | AKS |
| 37 | `IfcGroup` | Grup | GRP |
| 38 | `IfcHeatExchanger` | Isı Eşanjörü | IEJ |
| 39 | `IfcLightFixture` | Aydınlatma Armatürü | AYN |
| 40 | `IfcMember` | Eleman | ELM |
| 41 | `IfcOpeningElement` | Açıklık Elemanı | ACL |
| 42 | `IfcOutlet` | Priz | PRZ |
| 43 | `IfcPavement` | Asfalt | ASF |
| 44 | `IfcPile` | Kazık | KZK |
| 45 | `IfcPipeFitting` | Boru Bağlantı Parçası | BBP |

*[PDF sayfa 31]*

*(Tablo önceki sayfadan sürüyor; sütun başlıkları okunabilirlik için yinelendi.)*

|  | IFC Sınıfı (`IfcClass`) | IFC Sınıfının Türkçe Karşılığı (Bilgi için) | Kategori Kodu |
|---|---|---|---|
| 46 | `IfcPipeSegment` | Boru Segmenti | BRS |
| 47 | `IfcPump` | Pompa | PMP |
| 48 | `IfcRailing` | Korkuluk | KRK |
| 49 | `IfcRamp` | Rampa | RMP |
| 50 | `IfcRoad` | Yol | YOL |
| 51 | `IfcRoadPart` | Yol Parçası | YLP |
| 52 | `IfcRoof` | Çatı | CAT |
| 53 | `IfcSanitaryTerminal` | Sıhhi Terminal | STT |
| 54 | `IfcSign` | Tabela | TBL |
| 55 | `IfcSite` | Saha | SAH |
| 56 | `IfcSlab` | Döşeme | DSM |
| 57 | `IfcSolarDevice` | Güneş Enerjisi Cihazı | GEC |
| 58 | `IfcSpace` | Mahal | MHL |
| 59 | `IfcSpaceHeater` | Mekan Isıtıcısı | MIS |
| 60 | `IfcSpatialZone` | Mekansal Zon | MZN |
| 61 | `IfcStair` | Merdiven | MRD |
| 62 | `IfcTank` | Tank | TNK |
| 63 | `IfcTransformer` | Transformatör | TRF |
| 64 | `IfcTransportElement` | Taşıma Elemanı | TSM |
| 65 | `IfcValve` | Vana | VNA |
| 66 | `IfcWall` | Duvar | DVR |
| 67 | `IfcWindow` | Pencere | PNC |
| 68 | `IfcZone` | Zon | ZON |
| 69 | `IfcProject` | Proje | - |

## Tablo 5.2 İsteğe Bağlı IFC Sınıfları İçin Kategori Kodu Listesi

|  | IFC Sınıf İsmi (`IfcClass`) | IFC Sınıfının Türkçe Karşılığı | İsimlendirme (Kategori Kodu) |
|---|---|---|---|
| 1 | `IfcActuator` | Aktüatör | AKT |
| 2 | `IfcAirTerminalBox` | Hava Terminal Kutusu | HTK |
| 3 | `IfcAirToAirHeatRecovery` | Hava-Hava Isı Geri Kazanım Cihazı | HIG |
| 4 | `IfcAlarm` | Alarm | ALR |

*[PDF sayfa 32]*

*(Tablo önceki sayfadan sürüyor; sütun başlıkları okunabilirlik için yinelendi.)*

|  | IFC Sınıfı (`IfcClass`) | IFC Sınıfının Türkçe Karşılığı (Bilgi için) | Kategori Kodu |
|---|---|---|---|
| 5 | `IfcCableFitting` | Kablo Elemanı (Uç) | KLU |
| 6 | `IfcCableSegment` | Kablo Segmenti | KBS |
| 7 | `IfcCompressor` | Kompresör | KMP |
| 8 | `IfcController` | Kontrol Cihazı | KNT |
| 9 | `IfcConveyorSegment` | Konveyör Segmenti | KVS |
| 10 | `IfcCooledBeam` | Soğutmalı Kiriş | SKR |
| 11 | `IfcDiscreteAccessory` | Küçük Donatı Elemanı | KDE |
| 12 | `IfcDistributionChamberElement` | Dağıtım Odası Elemanı | DOE |
| 13 | `IfcDistributionCircuit` | Dağıtım Devresi | DDV |
| 14 | `IfcDistributionPort` | Dağıtım Portu | DPT |
| 15 | `IfcEarthworksCut` | Kazı İşleri | KZI |
| 16 | `IfcEarthworksFill` | Dolgu İşleri | DGI |
| 17 | `IfcElectricFlowStorageDevice` | Elektrik Akış Depolama Cihazı | EAD |
| 18 | `IfcElectricFlowTreatmentDevice` | Elektrik Akış İşleme Cihazı | EAI |
| 19 | `IfcElectricMotor` | Elektrik Motoru | EMO |
| 20 | `IfcElectricTimeControl` | Elektrik Zaman Kontrol Cihazı | EZK |
| 21 | `IfcElementAssembly` | Eleman Montajı | EMN |
| 22 | `IfcEngine` | Motor (Engine) | MTR |
| 23 | `IfcEvaporativeCooler` | Buharlaşmalı Soğutucu | BSC |
| 24 | `IfcExternalSpatialElement` | Dış Mekan Elemanı | DME |
| 25 | `IfcFacilityPartCommon` | Tesis Ortak Bölümü | TOB |
| 26 | `IfcFilter` | Filtre | FLT |
| 27 | `IfcFlowInstrument` | Akış Ölçer Cihazı | AOC |
| 28 | `IfcFlowMeter` | Akış Sayacı | ASY |
| 29 | `IfcGeographicElement` | Coğrafi Eleman | COG |
| 30 | `IfcGeomodel` | Coğrafi Model | CGM |
| 31 | `IfcGeoslice` | Coğrafi Dilim | CDL |
| 32 | `IfcGeotechnicalStratum` | Jeoteknik Katman | JTK |
| 33 | `IfcHumidifier` | Nemlendirici | NML |
| 34 | `IfcImpactProtectionDevice` | Darbe Koruma Cihazı | DKC |
| 35 | `IfcInterceptor` | Tutucu / Ayırıcı (Interceptor) | TTA |
| 36 | `IfcJunctionBox` | Bağlantı Kutusu | BKT |

*[PDF sayfa 33]*

*(Tablo önceki sayfadan sürüyor; sütun başlıkları okunabilirlik için yinelendi.)*

|  | IFC Sınıfı (`IfcClass`) | IFC Sınıfının Türkçe Karşılığı (Bilgi için) | Kategori Kodu |
|---|---|---|---|
| 37 | `IfcKerb` | Bordür | BRD |
| 38 | `IfcLamp` | Lamba | LMB |
| 39 | `IfcLiquidTerminal` | Sıvı Terminali | STM |
| 40 | `IfcMechanicalFastener` | Mekanik Bağlantı Elemanı | MBE |
| 41 | `IfcMedicalDevice` | Medikal Cihaz | MDC |
| 42 | `IfcMobileTelecommunicationsAppliance` | Mobil Telekomünikasyon Cihazı | MTC |
| 43 | `IfcMooringDevice` | Halat / Bağlama Cihazı | HBC |
| 44 | `IfcMotorConnection` | Motor Bağlantısı | MBA |
| 45 | `IfcNavigationElement` | Seyir Elemanı | SEY |
| 46 | `IfcProjectionElement` | Projeksiyon Elemanı | PJE |
| 47 | `IfcProtectiveDevice` | Koruma Cihazı | KRC |
| 48 | `IfcProtectiveDeviceTrippingUnit` | Koruma Cihazı Açma Ünitesi | KAU |
| 49 | `IfcRail` | Ray | RAY |
| 50 | `IfcRampFlight` | Rampa Kolu | RMK |
| 51 | `IfcReinforcedSoil` | Donatılı Zemin | DNZ |
| 52 | `IfcSensor` | Sensör | SNS |
| 53 | `IfcShadingDevice` | Güneş Kırıcı | GNK |
| 54 | `IfcSignal` | Sinyal | SNY |
| 55 | `IfcStackTerminal` | Yığın Terminali | YTN |
| 56 | `IfcStairFlight` | Merdiven Kolu | MRK |
| 57 | `IfcSwitchingDevice` | Anahtarlama Cihazı | AHC |
| 58 | `IfcSystemFurnitureElement` | Sistem Mobilya Elemanı | SME |
| 59 | `IfcTubeBundle` | Boru Demeti | BDM |
| 60 | `IfcUnitaryControlElement` | Üniter Kontrol Elemanı | UKE |
| 61 | `IfcUnitaryEquipment` | Üniter Ekipman | UEK |
| 62 | `IfcVibrationDamper` | Titreşim Sönümleyici | TSD |
| 63 | `IfcVibrationIsolator` | Titreşim İzolatörü | TIZ |
| 64 | `IfcVirtualElement` | Sanal Eleman | SNE |
| 65 | `IfcVehicle` | Araç | ARC |
| 66 | `IfcVoidingFeature` | Boşaltma Özelliği | BOS |
| 67 | `IfcWasteTerminal` | Atık Terminali | ATK |

*[PDF sayfa 34]*

## Tablo 5.3 BIM Modelinde Varlık İsimlendirme Şablonu\*

| Bilgi Alanı | Disiplin | Kategori Kodu | Açıklama |
|---|---|---|---|
| Karakter Sayısı | 2 | 3 | - |
| Veri Türü | Alfabetik | Alfabetik | Alfanümerik |

\* sadece ilgili varlığın isimlendirmesinde kullanılır
