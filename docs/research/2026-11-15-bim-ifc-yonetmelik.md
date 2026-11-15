# BIM yönetmeliği ve IFC uyumu — araştırma notu

*KentOS CAD · Mimari modülü ve `kentos-ifc` için · Kaynaklara erişim: 15 Kasım 2026 · Kod tabanı `8bc6bba77`*

Bu not [plan/bim-plan.md](../../plan/bim-plan.md) ile [plan/ifc-uyum.md](../../plan/ifc-uyum.md)'nin dayanağıdır. Plana giren
her iddianın nereden, nasıl okunduğu burada yazılıdır. Önceki notlar: [W4 mevzuat](2026-11-07-mimari/W4-mevzuat-standart.md)
(§2.17), [W2 BIM yazılımları](2026-11-07-mimari/W2-bim-mimari.md); önceki sayfa okumaları ADR
[8008](../adr/8008-ek4-layer-key.md), [8027](../adr/8027-ek3-room-numbers-and-ellipse-tags.md),
[8037](../adr/8037-turkish-title-block-sheet-sets-pdfa.md), [8047](../adr/8047-ifc-building-export.md).

## 0. Yöntem, erişim ve işaretler

- ✔ tam metni bu oturumda okundu · ◐ ikincil kaynaktan ya da arama özetinden · ⚠ kaynaklar çelişiyor ya da hüküm yoruma
  açık · ✖ doğrulanamadı.

| Kaynak | Nasıl okundu | Durum |
|---|---|---|
| Dijital Hazırlama Yönetmeliği, ana metin (RG 5.8.2026/33331) | Yayımlanmış tam metin kopyası (alomaliye.com); W4'ün ve iki plan dosyasının okuduğu kopyalarla (lexpera) madde madde aynı | ✔ |
| e-PYS Yönetmeliği, ana metin (RG 5.8.2026/33331) | Aynı yolla | ✔ |
| Resmî Gazete'nin kendi sitesi | Bu bilgisayarın tarayıcısı sitenin sertifikasını tanımıyor (`ERR_CERT_AUTHORITY_INVALID`); denetim kapatılmadı, site okunmadı. Önceki oturumlardaki "alan adı çözülemedi" ve "zaman aşımı" notlarının olası sebebi budur | ✖ |
| Yönetmeliğin ekleri (EK-1…9; 241 sayfa, taranmış görüntü, metin katmanı yok, yaklaşık 22,8 MB) | Bu oturumda indirilmedi. Depodaki sayfa okumaları (ADR 8008, 8027, 8037, 8047) + bağımsız bir açık kaynak dökümü ([ifc-ruhsat](https://github.com/BerkantACUN/ifc-ruhsat), MIT; tabloları sayfa numarasıyla JSON'a geçirmiş) | ◐ |
| IFC 4.3 şeması ve belgeleri, doğrulama servisi kılavuzu | buildingSMART'ın kendi sayfaları | ✔ |
| Finlandiya'nın model tabanlı ruhsatı | Bir belediyenin resmî sayfası (Tuusula) tam okundu; bakanlık kararnamesi arama özetinden | ✔ / ◐ |
| Estonya, Singapur | Arama özetleri (konferans sunumu, kurum genelgesi) | ◐ |
| TKGM'nin üç boyutlu sayısal yapı modeli | Genelgenin adresi artık yok (404); TKGM sayfası çerez onayı istiyor, onay verilmedi; arama özeti ve 2025 basın haberi | ◐ |
| TSE metinleri (TS EN ISO 16739-1, 19650-1/2, TS 9111 …) | Ücretli; okunmadı | ✖ |

**İki okumanın karşılaştırması.** Depodaki ADR okumaları ile bağımsız döküm örtüştüğü her yerde aynı sonucu veriyor:
kategori kodları (DVR, DSM, KLN, KRS, CAT, KRK, MRD, KAP, PNC, MHL, ACL, BNK, BNA, SAH), EK-6'da duvar, kapı, döşeme,
pencere, çatı, kolon ve merdivenin zorunlu özellikleri, EK-7'nin tabloları, EK-9'un 21 sorusu, EK-3'ün `Kat_Bölüm_Sıra`
kalıbı, EK-2 Tablo 2.14'ün kat kodları. Ayrıştıkları yerler §4'tedir. Bağımsız döküm birincil kaynak değildir; kural
kataloğuna giren her satır sayfa görüntüsünden bir kez daha okunur (plan: BIM-01, IFC-01).

## 1. Ana metinler ne istiyor

### 1.1 Dijital Hazırlama Yönetmeliği ✔

| Madde | Hüküm (kısaltılmış) | Yazılıma etkisi |
|---|---|---|
| 1 | Kapsam: uygulama imar planı olan alanlarda **yeni yapılacak binalar**. Hariç: bodrum dışında en çok iki katlı ve toplam yapı inşaat alanı 200 m²'yi geçmeyen müstakil yapılar; entegre tesis olmayan tarım ve hayvancılık yapıları | Kapsam kararı projenin verisinden çıkar; metin yalnız yeni binayı sayıyor |
| 4/2–3 | 2B çizim dosyaları ve çizim dışı belgeler **PDF/A** | Ruhsat takımı (var, ADR 8037) |
| 4/4 | 3B sayısal yapı modeli TS EN ISO 19650-1 ve -2'ye göre BIM tabanlı | Bilgi yönetimi: durum, revizyon, sorumluluk |
| 4/5 | BIM modeli TS EN ISO 16739-1'deki **IFC** biçiminde teslim | `kentos-ifc` |
| 5 | Açık kalan konuda önce Türk, yoksa Avrupa standardı, yoksa uluslararası kabul gören belge | Yorum sırası |
| 6 | Proje kodu EK-1, dosya adları EK-2 | Adlandırma kitaplığı (var) |
| 7 | CAD'de katman, sembol, ölçü, yazı, çizgi tipi EK-4; mahal numarası ve adı EK-3 | EK-4 katman anahtarı, EK-3 numarası (var); denetimi eksik |
| 8 | A4 ve katları; **her çizim ve belge ayrı dosya**; paftalar tek dosyada birleştirilmez | Pafta başına dosya (var) |
| 9/1 | Her disiplin ayrı BIM modeli, EK-5/6/7'ye göre, IFC olarak teslim. "Ayrıca, BIM modelinden CAD çizim dosyaları da üretilerek PDF/A formatında teslim edilir. **CAD çizim dosyaları, BIM modelinden üretilir.** BIM modeli ile CAD çizim dosyası PDF/A nüshasının birbiriyle uyumlu olması zorunludur" | Paftanın modelden üretildiği kanıtlanabilmeli |
| 9/2 | Ölçüler SI | Var |
| 10 | Model adı Md. 6'ya göre; **modeldeki mahallerin adı EK-5'e göre** | Mahalin IFC adı (§4/2) |
| 11 | Varlık adı disiplin, kategori, açıklama. Açıklamadaki bilgiler "boşluk bırakılmaksızın orta tire" ile ayrılır. İdare dört karakterli ek alt disiplin tanımlayabilir | Ad kalıbı veri olmalı (§4/1) |
| 12 | Varlıklar EK-5'in zorunlu ve isteğe bağlı sınıflarında. Zorunlu sınıfların zorunlu ve isteğe bağlı öznitelikleri EK-6'da. **İsteğe bağlı sınıfların özniteliklerini idare belirleyebilir** | Kural kataloğu idareye göre genişleyebilmeli |
| 13 | EK-7'nin zorunlu proje öznitelikleri; idare isterse isteğe bağlılar | Aynı |
| 14 | Model gelişim seviyesi EK-8 | Geometri ve bilgi yeterliği kanıtı |
| 15 | Teslim edilen her model için EK-9 model kalite kontrol formu | Form (var); kanıtı eksik |
| 16 | Yürürlük **1.9.2027** | |
| Geçici 1 | Kademeli geçiş (§2) | Tarihli kapsam kararı |
| Geçici 2 | Uygulama tarihinden önce ruhsat başvurusu yapılmış ya da kamuda ihale kararı/tarihi alınmış yapılara, o tarihle başlayan hüküm uygulanmaz | Başvuru ve ihale tarihi proje verisi |

### 1.2 e-PYS Yönetmeliği ✔

| Madde | Hüküm | Yazılıma etkisi |
|---|---|---|
| 1 | Kapsam aynı; ayrıca 3194'ün 4, 26 ve 27. maddelerinde sayılanlar hariç | Ayrı kapsam kararı |
| 4/1–2 | Teslim, inceleme, eksik bildirimi, onay ve tebligat Bakanlığın ya da yetkilendirdiği idarenin işlettiği e-PYS'de. Yüklenen her dosya Hazırlama Yönetmeliğine uygun olmak zorunda | Paket yüklemeden önce denetlenir |
| 4/3, 7/7 | İdarelerin kullandığı yazılım platformları ile e-PYS arasında dosya ve bilgi paylaşılır; bu platformlarda yapılan işlemlerin e-PYS'ye aktarılması ve orada kayda alınması zorunlu | **Belediyenin kendi yazılımı e-PYS ile konuşacak** |
| 4/4 | Hangi bina türleri, ölçütler, **yükleme biçimi** ve tarihler: Bakanlık belirler | Biçim ve tarih veri olmalı; kod sabiti olmamalı |
| 4/5 | İdare veriyi üçüncü kişiye veremez, müellifin izni olmadan çoğaltamaz, değiştiremez | Yetki, kayıt, saklama |
| 5/1–2 | Dosyayı müellif kaydeder; doğruluğundan o sorumlu. Dosyalar Hazırlama Yönetmeliğine göre incelenir; eksik ya da hatalı dosya yeniden yüklenir | Müellifin ön denetimi iadeyi önler |
| 5/3 | İdare ve yapı denetim kuruluşu projenin mevzuata uygunluğunu e-PYS'de inceler; eksikleri gerekçesiyle bildirir | İdare tarafında model inceleme aracı |
| 5/4 | Kimlik doğrulama: kullanıcı adı ve şifre, güvenli e-imza ya da Bakanlığın belirleyeceği yöntem | İmza yöntemi henüz belli değil |
| 5/6 | Yöntem, kılavuz ve rehberler Bakanlığın sitesinde yayımlanır | İzlenecek; 15 Kasım 2026'da böyle bir kılavuz bulunamadı ✖ |
| 6 | E-imzasız PDF/A ya da IFC tesliminde ayrıca basılı teslim; e-imzalıysa dijital yeter; dijital ile basılı aynı olmalı; **taranmış PDF kullanılamaz** | Vektör PDF/A; paketin özeti imzaya hazır |
| 8 | Yürürlük **1.9.2028** | |

## 2. Takvim (Hazırlama Yön. Geçici Md. 1; e-PYS Md. 8) ✔

| Tarih | Ne başlar | Kimde |
|---|---|---|
| 1.9.2027 | Yönetmelik yürürlükte; PDF/A (4/2–3) | İl nüfusu > 2.000.000, belediye nüfusu > 600.000 ve toplam yapı inşaat alanı > 5.000 m² |
| 1.9.2028 | e-PYS Yönetmeliği yürürlükte | Fiilî kanal, biçim ve tarih için Bakanlığın kararı ayrıca aranır (4/4) |
| 1.3.2029 | PDF/A | Kapsamdaki öbür bütün binalar |
| 1.9.2029 | IFC (4/5), 1. grup | İl > 2.000.000, belediye > 600.000, alan > 10.000 m² |
| 1.9.2030 | IFC, 2. grup | İl > 2.000.000, belediye > 400.000, alan > 10.000 m² |
| 1.9.2031 | IFC, 3. grup | İl > 1.000.000, belediye > 300.000, alan > 10.000 m² |
| 1.9.2032 | IFC | Kalan bütün binalar |
| 1.9.2033 | Yönetmeliğin tamamı | Konut ve ticaret+konut karma dışındaki binalar |

- ⚠ Geçici 1/1-b'nin son cümlesi: IFC hükmü uygulanmaya başladıktan sonra o yapılar için 4/2–3 (PDF/A) uygulanmaz. Md. 9/1
  ise modelden üretilen çizimlerin PDF/A teslimini ayrıca ister. İkisi birlikte izlenir; IFC başlayınca PDF/A'yı
  kendiliğinden kapatan tek bir bayrak kullanılmaz.
- *Çıkarım (nüfus TÜİK'ten doğrulanmadı):* Sivas ilinin nüfusu 1 milyonun altında. Öyleyse Sivas'ta konut için PDF/A
  1.3.2029'dan, IFC 1.9.2032'den başlar. KentOS'un başka illerdeki kullanıcıları için ilk IFC tarihi 1.9.2029'dur.

## 3. Ekler

### 3.1 Eklerin içeriği

| Ek | İçerik | Sayfa (ek PDF'si) | Okuma |
|---|---|---|---|
| EK-1 | Proje kodu: başvuru yılının iki hanesi + idarenin dört haneli sıra numarası | — | ADR 8037 ◐ |
| EK-2 | Dosya adı: `ProjeKodu_BinaKodu_Disiplin_AltDisiplin_DosyaTürü_DosyaTipi_Kat_Sıra_Revizyon`; Tablo 2.2 12 disiplin kodu, 2.12 dosya türü (MD model, FR form …), 2.13 dosya tipi (MKF model kontrol formu …), 2.14 kat kodları (B01…, K00, K01…, NNN) | s. 3… | ADR 8037, 8047; bağımsız döküm ◐ |
| EK-3 | 2B mahal numarası `Kat(3)_Bölüm/Fonksiyon(3)_Sıra(3)`; sıra her kat ve bölüm için 001'den, saat yönünde; Tablo 3.2 yaklaşık 150 mahal adı ve kısaltması (SLN, YTO, MUT, BAN, WCC …) | s. 14–19 | ADR 8027 ◐ |
| EK-4 | CAD katmanı, sembol, ölçü, yazı, çizgi tipi | — | ADR 8008 ◐ |
| EK-5 | §5.4: sınıflar IFC 4.3 esas alınarak; Tablo 5.1 **69 zorunlu sınıf** ve üç harfli kategori kodu; Tablo 5.2 **67 isteğe bağlı sınıf**; §5.5 ve Tablo 5.3 ad kalıbı | s. 29–34 | ADR 8047; bağımsız döküm ◐ |
| EK-6 | Tablo 6.1–6.68: zorunlu sınıfların zorunlu öznitelik, özellik seti ve miktarları (sıra Tablo 5.1 ile aynı); Tablo 6.69–6.135 isteğe bağlılar. §6.1.2 üst sınıf yerine alt sınıf; **§6.1.3 vekil sınıf (IfcProxy, IfcBuildingElementProxy) kullanılmaz**; §6.5 `TREpys_` önekli Türkiye setleri | s. 36–109 (zorunlu) | ADR 8047; bağımsız döküm ◐ |
| EK-7 | Tablo 7.1 IfcProject, 7.2 IfcPerson, 7.3 IfcPersonAndOrganization, 7.4 IfcOrganization, 7.5 IfcActorRole, 7.6 IfcProjectedCRS, 7.7 IfcMapConversion, 7.8 gösterim bağlamı, 7.9–7.12 isteğe bağlı. "Her bir disiplin modelinin IfcProject sınıfının ortak öznitelikleri aynı olmalıdır" | s. 221–230 | ADR 8047; bağımsız döküm ◐ |
| EK-8 | Model gelişim seviyesi; bağımsız döküm ruhsat modeli için **LOD 300** diyor | — | ◐ ⚠ nesne bazlı tablo okunmadı |
| EK-9 | Tablo 9.1: mimari model kalite kontrol formu, 21 Evet/Hayır sorusu | s. 238–239 | ADR 8047; bağımsız döküm ◐ |

69. zorunlu sınıf `IfcProject`'tir; tablosu EK-6'da değil EK-7'dedir (7.1). "69 sınıf, 68 tablo" bir sayım farkı değildir.

### 3.2 Mimari modelde karşılaşılan zorunlu sınıflar (EK-5 Tablo 5.1; EK-6) ◐

`M` metin, `S` sayı, `B` mantıksal. Hepsinde ayrıca `Name` (EK-5 kalıbıyla). "Malzeme" = `IfcRelAssociatesMaterial` ile
`IfcMaterial.Name`, biçimi "Malzeme İsmi_Dayanım" (Beton_C25, Çelik_S355).

| Sınıf | Kod | EK-6 | Zorunlu alanlar |
|---|---|---|---|
| IfcProject | — | 7.1 | Description, `Pset_ProjectCommon.ProjectType`; adı proje kodu (EK-1) |
| IfcSite | SAH | 6.55 | ObjectType, LongName, RefElevation; `Pset_SiteCommon` FloorAreaRatio, SiteCoverageRatio, TotalArea; `Pset_Address` AddressLines, Town, Region; `Pset_LandRegistration` LandID, LandTitleID; `TREpys_ParselOzellikSeti` TAKS, KAKS, CekmeOn, CekmeYan, CekmeArka |
| IfcBuilding | BNA | 6.7 | LongName; `Pset_BuildingCommon` BuildingID, IsPermanentID, ConstructionMethod, FireProtectionClass, OccupancyType, GrossPlannedArea, NetPlannedArea, NumberOfStoreys; `Pset_Address`; `Qto_BuildingBaseQuantities` Height, EavesHeight, taban alanı, GrossFloorArea, NetFloorArea |
| IfcBuildingStorey | BNK | 6.8 | `Qto_BuildingStoreyBaseQuantities.NetFloorArea`; `Pset_BuildingStoreyCommon.ElevationOfSSLRelative` (taşıyıcı döşemenin kotu) |
| IfcSpace | MHL | 6.58 | PredefinedType; `Pset_SpaceCoveringRequirements` FloorCovering, WallCovering, CeilingCovering ve üçünün kalınlığı. Dipnot: kaplama bilgisi ya mahalde bu setle ya da IfcCovering'in katmanlarıyla verilir |
| IfcSpatialZone | MZN | 6.60 | Description, LongName; `TREpys_EmsalOzellikSeti.EmsalDurumu` (DAHIL, HARIC, DIGER) |
| IfcZone | ZON | 6.68 | LongName, ObjectType |
| IfcWall | DVR | 6.66 | PredefinedType, Tag, malzeme; `Pset_WallCommon` IsExternal, FireRating |
| IfcCurtainWall | GDC | 6.21 | PredefinedType, Tag, malzeme; `Pset_CurtainWallCommon.FireRating` |
| IfcSlab | DSM | 6.56 | PredefinedType, Tag, malzeme; `Pset_SlabCommon` IsExternal, FireRating |
| IfcRoof | CAT | 6.52 | PredefinedType, Tag; `Pset_RoofCommon` IsExternal, FireRating |
| IfcColumn | KLN | 6.16 | PredefinedType, Tag, malzeme; `Pset_ColumnCommon` FireRating, LoadBearing |
| IfcBeam | KRS | 6.3 | PredefinedType, Tag, malzeme; `Pset_BeamCommon.FireRating` |
| IfcMember | ELM | 6.40 | PredefinedType, Tag, malzeme; `Pset_MemberCommon.IsExternal` |
| IfcDoor | KAP | 6.25 | PredefinedType, Tag; `Pset_DoorCommon` IsExternal, FireRating, FireExit |
| IfcWindow | PNC | 6.67 | PredefinedType, Tag; `Pset_WindowCommon.IsExternal` |
| IfcOpeningElement | ACL | 6.41 | PredefinedType |
| IfcStair | MRD | 6.61 | PredefinedType, Tag, malzeme; `Pset_StairCommon` FireRating, FireExit |
| IfcRamp | RMP | 6.49 | PredefinedType, Tag, malzeme |
| IfcRailing | KRK | 6.48 | PredefinedType, Tag |
| IfcCovering | KPL | 6.20 | PredefinedType, Tag, malzeme; katmanın malzemesi ve kalınlığı; `Pset_CoveringCommon.FireRating` |
| IfcChimney | BAC | 6.14 | PredefinedType, Tag, malzeme |
| IfcGridAxis | AKS | 6.36 | AxisTag |
| IfcGroup | GRP | 6.37 | Name (tabloda Tag da yazıyor ⚠) |
| IfcFurniture | MBL | 6.35 | PredefinedType, Tag |
| IfcSanitaryTerminal | STT | 6.53 | PredefinedType, Tag |
| IfcTransportElement | TSM | 6.64 | PredefinedType, Tag; `Pset_TransportElementCommon` CapacityPeople, CapacityWeight |
| IfcFooting | TML | 6.34 | PredefinedType, Tag, malzeme (statik disiplininin) |
| IfcSign | TBL | 6.54 | PredefinedType, Tag |

Tablo 5.1'in öbür 40 sınıfı tesisat, elektrik ve altyapınındır (menfez, kanal, boru, pano, kazan, köprü, yol …).
İsteğe bağlı listede mimariye değenler: IfcStairFlight (MRK), IfcRampFlight (RMK), IfcShadingDevice (GNK),
IfcElementAssembly (EMN), IfcExternalSpatialElement (DME), IfcGeographicElement (COG), IfcVirtualElement (SNE),
IfcSystemFurnitureElement (SME), IfcVoidingFeature (BOS).

### 3.3 EK-9'un 21 sorusu ◐

1 dosya boyutu (Bakanlığın sınırı) · 2 proje bilgileri (Md. 13) · 3 proje ve dosya adı (Md. 6) · 4 mahal no ve adı
(Md. 10) · 5 varlık adları (Md. 11) · 6 koordinat sistemi · 7 gelişim seviyesi (Md. 14) · 8 doğru zorunlu sınıflar ·
9 zorunlu öznitelik ve özellikler · **10 "Mahal (IfcSpace) ve Mekansal Zon (IfcSpatialZone) alanları emsal hesap
tablolarıyla uyumlu mu?"** · 11 proje, saha, bina ve en az bir kat · 12 mimari modelde mahal · 13 mahaller örtüşmüyor ve
doğru katta · 14 mahaller kapalı, üst sınırı doğru · 15 varlıklar doğru katta ya da kotta · 16 yinelenen varlık yok ·
17 sınıflar doğru atanmış · 18 IFC sürümü ve `.ifc` uzantısı · 19 doğru ön tanımlı tip · 20 özellikler doğru setlerde ·
**21 "Teslim edilen PDF paftaları BIM modelinden üretildi mi?"**

## 4. Yoruma açık ve çelişkili noktalar

| # | Konu | Durum | KentOS'ta bugün |
|---|---|---|---|
| 1 | Açıklamadaki ayırıcı: Md. 11/2 "orta tire"; EK-5 §5.5 alanlar "-", açıklamanın içi "_", "açıklamada orta tire kullanılmaz" ⚠ | Ek daha ayrıntılı; iki okuma da EK-5'i uyguluyor | `MM-DVR-Dış_duvar_25` (EK-5) |
| 2 | Mahalin IFC adı ⚠: ADR 8047 EK-5 §5.5.4'ü "numarası ve adı açıklamada" okudu (`MM-MHL-…`); bağımsız döküm `IfcSpace.Name`'i yalın EK-3 numarası (`K00_SLN_001`), adı `LongName` sayıyor | Sayfadan yeniden okunacak | `MM-MHL-<no>_<ad>`, LongName ad |
| 3 | EK-8: nesne bazlı asgari seviye tablosu okunmadı; "LOD 300" ikincil ✖ | Sayfadan okunacak | — |
| 4 | Görünüm tanımı (MVD): eklerde geçmiyor (ADR 8047 taradı) | İdare ya da Bakanlık kılavuzu beklenir | Başlık `DesignTransferView` |
| 5 | Ön tanımlı tip: EK-9/19 boş ya da `NOTDEFINED` değeri uygun saymıyor (bağımsız dökümün okuması) | Sayfadan okunacak | Çatı ve merdiven `NOTDEFINED` |
| 6 | Tablo 6.37 `IfcGroup.Tag`: şemada böyle bir öznitelik yok ⚠ | Şema üstün; fark listesine | Yazılmıyor |
| 7 | `IfcOrganization.Identification` "sayı" (vergi no); şemada metin ⚠ | Rakamlardan oluşan metin | Metin |
| 8 | Ön tanımlı tip yazımları (SKIRTING BOARD gibi boşluklu) şemadan farklı ⚠ | Şemadaki yazım | — |
| 9 | Giydirme cephe paneli (`IfcPlate`) iki listede de yok ⚠ | `IfcCurtainWall` içinde; idareyle teyit | Giydirme cephe nesnesi yok |
| 10 | Aks: liste `IfcGridAxis` diyor; şemada kök nesne `IfcGrid`'dir, eksenleri `IfcGridAxis` | İkisi birlikte yazılır | Aks IFC'ye yazılmıyor |
| 11 | Düşey datum: EK-7 notu ITRF96, gerektiğinde TUDKA99'a dönüşüm; bağımsız döküm TUDKA99 ⚠ | Sayfadan okunacak | Projenin söylediği (ADR 8047 §8) |
| 12 | Dosya boyutu sınırı (EK-9/1): Bakanlık yayımlamadı ✖ | Elle | Boş bırakılıyor |
| 13 | PDF/A'nın bölümü ve düzeyi: metin yalnız "PDF/A" diyor | İdare ya da kılavuz | PDF/A-2b |
| 14 | Mahalde net alan (6.58): ADR 8047 var dedi, bağımsız dökümde yok ⚠ | Sayfadan okunacak | `Qto_SpaceBaseQuantities.NetFloorArea` yazılıyor |
| 15 | `IfcZone`'un neyi temsil edeceği (bağımsız bölüm, yangın kompartımanı …) ✖ | Sayfadan okunacak | Yazılmıyor |
| 16 | e-PYS'nin teknik arayüzü, imza yöntemi, yükleme biçimi ✖ | Bakanlık (e-PYS 4/4, 5/4, 5/6) | — |
| 17 | Bakanlığın Kasım 2025 taslağı nihai metinden farklı (tablo numaraları, ayırıcılar, zorunlu/isteğe bağlı) | Taslak kullanılmaz | — |

## 5. KentOS'un bugünkü IFC çıktısı (kodda ölçüldü, `crates/shared/ifc`)

- **Yazılan sınıflar (15):** IfcProject, IfcSite, IfcBuilding, IfcBuildingStorey, IfcSpace, IfcWall, IfcSlab, IfcColumn,
  IfcBeam, IfcRoof, IfcRailing, IfcStair, IfcDoor, IfcWindow, IfcOpeningElement (`arch.rs`).
- **Listede olup yazılmayan (14):** IfcSpatialZone, IfcZone, IfcCurtainWall, IfcMember, IfcRamp, IfcCovering, IfcChimney,
  IfcGridAxis, IfcGroup, IfcFurniture, IfcSanitaryTerminal, IfcTransportElement, IfcFooting, IfcSign. `building.rs`'in
  `Class` listesinde yedisinin adı ve kodu duruyor; `arch.rs` hiçbirini kurmuyor. Rampa "henüz aktarılmıyor" uyarısıyla
  dışarıda kalıyor.
- **Yazılan sınıflarda eksik zorunlu alanlar:**
  - yangın dayanım sınıfı (`FireRating`: duvar, döşeme, kolon, kiriş, çatı, kapı, merdiven) ve acil çıkış (`FireExit`:
    kapı, merdiven) — çizim bu bilgileri tutmuyor;
  - malzemede dayanım ("Beton_C25"); çatı, merdiven ve korkulukta malzeme yok;
  - çatı ve merdivenin ön tanımlı tipi `NOTDEFINED`; duvar hep `STANDARD`, döşeme hep `FLOOR`, mahal hep `INTERNAL`;
  - mahalde kaplama kalınlıkları;
  - katta taşıyıcı döşemenin kotu; binada yapım yöntemi, yangın koruma sınıfı, planlanan net alan, saçak yüksekliği;
  - sahada `ObjectType` ve `Pset_SiteCommon.TotalArea` (parsel alanı `Qto_SiteBaseQuantities.GrossArea`'da);
  - projede `Description` ve `Pset_ProjectCommon.ProjectType`.
- **Yapı:** tek saha, tek bina; tip nesnesi (`IfcWallType` …), katmanlı malzeme takımı, sınıflandırma ve mahal sınırı
  ilişkisi yok; eğri duvar ve merdiven düz parçalarla; açıklık gövdeden kesilmemiş (okuyucu çıkarır).
- **EK-9:** 21 soru bellekteki modelden cevaplanıyor (`quality.rs`); yazılmış dosya yeniden okunmuyor. Depoda IFC okuyucu
  yok.
- **Bağımsız okuyucu:** `scripts/fixtures/arch_ifc_check.py` (IfcOpenShell) bölüm binasında 0 bulgu veriyor; bu, şemanın
  kurallarını sınar, Türkiye profilini sınamaz.

## 6. Dünyadaki uygulama

| Ülke | Ne yapıyor | Ders |
|---|---|---|
| **Finlandiya** ✔/◐ | 1.1.2026'dan beri ruhsat başvurusu tasarım modeliyle ya da makinece okunur veriyle yapılabiliyor. Model **IFC 4.3.2.0 ya da üstü**, ana çizim düzeyinde, bakanlık kararnamesinin (1340/2025) eklerindeki çekirdek veriyle. **Ana çizimler her durumda PDF** olarak da veriliyor. Uzmanlık projelerini IFC vermek zorunlu değil. Belediye, 2026 başında sistemlerin tam hazır olmadığını yazıyor. Kararnamenin Reference View ve STEP dosyası istediği arama özetinden ◐ | Şema hedefi Türkiye'ninkiyle aynı (IFC 4.3). Model + PDF birlikte. İdarenin araçları geride kaldı |
| **Estonya** ◐ | Yapı sicilinin (EHR) içinde çalışan, IFC'yi doğrudan alan sistem; 47 mevzuat ölçütünü kendiliğinden denetliyor; ölçütler etki ve emek matrisiyle seçilmiş | Yazılımlar denetimin istediği veriyi IFC'ye yazmakta zorlanıyor; denetimin istediği her veri model gereklerinde yok; gerekler sürekli güncelleniyor |
| **Singapur** ◐ | CORENET X; 1.10.2026'dan beri brüt alanı 5.000 m² ve üstü yeni projelerde zorunlu; biçim IFC+SG (IFC'nin yerel gereklerle genişletilmişi) | Ulusal özellik setleri (bizde `TREpys_`) ve alan eşiğiyle kademeli geçiş aynı desen |

Kaynaklar: [Tuusula belediyesi](https://tuusula.fi/en/home/housing-and-environment/building-control-and-building-permits/information-model-based-permit-processing/),
[Finlandiya Çevre Bakanlığı duyurusu](https://valtioneuvosto.fi/-/1410903/ymparistoministerio-antoi-asetuksen-rakentamisen-suunnitelmamallien-ja-viranomaiskatselmusten-sisallosta),
[Estonya, BIM tabanlı ruhsat (kitap bölümü)](https://www.taylorfrancis.com/chapters/oa-edit/10.1201/9781003650287-22/bim-based-building-permit-christopher-robin-raitviir-pille-riin-peet-irene-lill-raido-puust),
[URA genelgesi DC26-08](https://www.ura.gov.sg/guidelines/circulars/dc26-08/).

## 7. buildingSMART'ın araçları

- **IFC 4.3** ISO 16739-1:2024'tür; yayımlanan şemanın adı `IFC4X3_ADD2` (4.3.2.x). ✔
  [Şema ve belgeler](https://standards.buildingsmart.org/IFC/RELEASE/IFC4_3/)
- **Doğrulama servisi** ✔ ([kılavuz](https://buildingsmart.github.io/validate/user/index.html)): en çok 256 MB, sıkıştırılmamış
  `.ifc`; kabul ettiği şema adları `IFC2X3`, `IFC4`, `IFC4X3_ADD2`. Sırasıyla STEP sözdizimi, şema (öznitelik sayısı ve
  türü, ters ilişkiler, WHERE kuralları, genel kurallar), normatif kurallar (uygulayıcı anlaşmaları, gayriresmî
  önermeler) ve hata saymadığı yaygın uygulama kuralları. Proje, ülke ya da kuruma özgü kuralları **denetlemez**. Kurallar
  açık kaynak (`ifc-gherkin-rules`). 2026 sürümünde Reference View'ın dışladığı sınıfları uyaran kurallar eklendi ◐.
- **IDS 1.0** (Information Delivery Specification): sınıf, öznitelik, özellik, malzeme, sınıflandırma ve parçası olma
  isteklerini makinece ifade eder; **geometriyi, mesafeyi, kapalılığı, dosyalar arası uyumu ifade etmez** ◐.
  [buildingSMART IDS](https://www.buildingsmart.org/standards/bsi-standards/information-delivery-specification-ids/)
- **Yazılım sertifikası** ◐: IFC 2x3, 4 ve 4.3 için içe ve dışa aktarma sertifikası veriliyor; sürüme ve araca özel;
  geçersiz dosya üretince geri alınıyor. [Program](https://www.buildingsmart.org/compliance/software-certification/)
- **Görünüm tanımları:** IFC 4.3 için Reference View, Alignment Based View ve Design Transfer View.
  [MVD politikası](https://www.buildingsmart.org/wp-content/uploads/2021/05/20210425_MVD-policy_IFC4.x.pdf)

## 8. Bağımsız araç: `ifc-ruhsat`

MIT lisanslı, IfcOpenShell üstünde; yönetmeliğin eklerini JSON kurala çevirmiş, EK-9 formunu dolduruyor, EK-6 ve EK-7'yi IDS
olarak da veriyor. Halka açık üç modelde (başka yazılımların çıktıları) tipik eksikler: IFC 4.3 değil IFC4; vekil sınıflar;
ad kalıbı dışı adlar; yangın özellikleri yok; mahal yok; koordinat sistemi yok ya da Türkiye dilimi değil; `TREpys_` setleri
yok. KentOS için değeri: **üçüncü bir bağımsız okuyucu** ve satır satır karşılaştırılacak ikinci bir ek dökümü. Birincil
kaynak ya da resmî doğrulayıcı değildir.

## 9. Komşu yükümlülük: TKGM'nin üç boyutlu sayısal yapı modeli ◐

Kat irtifakı ve kat mülkiyeti kurulurken mimari projeden üretilen 3B sayısal yapı modeli TKGM'ye özgü CityGML biçiminde
(`.gml`, ülke koordinat sisteminde) veriliyor ve kendiliğinden doğrulanıyor; modeli harita mühendisi hazırlıyor. Bu, 33331
sayılı yönetmelikten ayrı bir yükümlülüktür; aynı bina modelinden beslenebilir. Genelgenin kendisi bu oturumda okunamadı:
biçim, ayrıntı düzeyi ve sorumluluk birincil metinden doğrulanmadan iş açılmaz (plan: BIM-19).
[2025 basın haberi](https://www.memurlar.net/haber/1127428/tapu-islemlerinde-dijital-donem-basliyor.html).

## 10. Kaynaklar

- Yönetmelikler: [Hazırlama, tam metin](https://www.alomaliye.com/2026/08/05/mimarlik-ve-muhendislik-projelerinin-dijital-olarak-hazirlanmasi-hakkinda-yonetmelik/) ·
  [e-PYS, tam metin](https://www.alomaliye.com/2026/08/05/mimarlik-ve-muhendislik-projelerinin-elektronik-ortamda-teslimi-ve-yonetilmesi-hakkinda-yonetmelik/) ·
  [Hazırlama (lexpera)](https://www.lexpera.com.tr/resmi-gazete/metin/mimarlik-ve-muhendislik-projelerinin-dijital-olarak-hazirlanmasi-hakkinda-yonetmelik-33331) ·
  Resmî Gazete: `resmigazete.gov.tr/eskiler/2026/08/20260805-2.htm`, `-3.htm`, ekler `20260805-2-1.pdf`.
- Ek dökümü: [ifc-ruhsat, KAYNAKLAR.md](https://github.com/BerkantACUN/ifc-ruhsat/blob/main/KAYNAKLAR.md) ve
  `src/ifc_ruhsat/ekler/*.json`.
- IFC: [IFC 4.3](https://standards.buildingsmart.org/IFC/RELEASE/IFC4_3/) ·
  [IfcSpatialZone](https://standards.buildingsmart.org/IFC/RELEASE/IFC4_3/HTML/lexical/IfcSpatialZone.htm) ·
  [IfcZone](https://standards.buildingsmart.org/IFC/RELEASE/IFC4_3/HTML/lexical/IfcZone.htm) ·
  [IfcGrid](https://standards.buildingsmart.org/IFC/RELEASE/IFC4_3/HTML/lexical/IfcGrid.htm) ·
  [IfcMapConversion](https://standards.buildingsmart.org/IFC/RELEASE/IFC4_3/HTML/lexical/IfcMapConversion.htm) ·
  [ISO 16739-1:2024](https://www.iso.org/standard/84123.html) · [ISO 19650-1](https://www.iso.org/standard/68078.html) ·
  [ISO 19650-2](https://www.iso.org/standard/68080.html).
- Doğrulama: [buildingSMART doğrulama servisi](https://validate.buildingsmart.org/) ·
  [IfcOpenShell doğrulama](https://docs.ifcopenshell.org/autoapi/ifcopenshell/validate/index.html) ·
  [veraPDF](https://docs.verapdf.org/validation/).
