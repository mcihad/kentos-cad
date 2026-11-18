# KentOS IFC uyum ve teslim planı

İnceleme: **15 Kasım 2026** (ikinci tur; ilki aynı gün, `8bc6bba77`). Kod tabanı: `8bc6bba77`. Dal: `codex/bim-ifc-uyum`.

**Eş belge:** [Mimari BIM geliştirme planı](bim-plan.md) (sıra ve kapılar orada, §6–§7) · **Dayanak:**
[araştırma notu](../docs/research/2026-11-15-bim-ifc-yonetmelik.md) · **Mevcut karar:** [ADR 8047](../docs/adr/8047-ifc-building-export.md)
· **Görüntüleyici ve okuma motoru:** [ADR 8061](../docs/adr/8061-ifc-viewer-and-reading-engine.md) (karar; deneme
bekliyor), [notu](../docs/research/2026-11-15-ifc-goruntuleyici.md).

**İlerleme (15 Kasım 2026):** IFC-01 `[x]` ([ADR 8062](../docs/adr/8062-digital-project-rule-catalog.md)): EK-5…7 ve 9
`kentos-ifc`'te veri; IFC 4.3 sözlüğüyle karşılaştırma (`catalog_schema`), her farkın kararı var; başlangıç ölçümü dosyadan
(§5'in bugün sütunu ona göre düzeltildi).

Bu tur **araştırma ve plandır**; uygulama kodu değişmedi. İlk taslağa göre değişenler: bugünkü çıktı yönetmeliğin sınıf ve
alan listesine karşı kodda ölçüldü (§4, §5); kontrol formunun 21 sorusu için bugünkü ve hedef kanıt yazıldı (§9); okuyucu
ayrı bir paket oldu ve doğrulamanın önüne alındı (IFC-07); "69 sınıf, 68 tablo" sorusu kapandı (69. sınıf `IfcProject`,
tablosu EK-7'de).

**Görüntüleyici (15 Kasım, sahibin isteği ve kararı):** IFC görüntüleyici plana girdi (IFC-12). Uygulamanın içinde gelir
ama gevşek bağlıdır: gerektiğinde hızlıca ayrı bir program olabilir ya da WASM ile web'de çalışabilir. Okuma ve üçgenleme
motoru olarak IFClite'ın Rust çekirdeği seçildi; kabulü bir denemeye bağlıdır. Bu karar IFC-07'yi ve IFC-09'u değiştirdi.

## 1. Hedef ve kabul ayrımı

KentOS'un mimari dışa aktarımı, modeldeki her yapı nesnesini yönetmeliğin istediği sınıfta ve alanlarla taşıyan, şeması
geçerli, bağımsız araçlarla okunmuş bir IFC 4.3 dosyası üretir. Dosya aynı model revizyonunun PDF/A paftaları ve model
kalite kontrol formuyla izlenebilir. Hızlı dışa aktarma (bugünkü `cad.arch.ifc`) kayıplarını söyleyerek çalışmayı sürdürür;
**ruhsat paketinde** gerekli bir nesnenin ya da alanın eksikliği yayını durdurur.

Beş sonuç ayrı raporlanır: **dosya okunuyor (STEP), şema geçerli, Türkiye teslim profili, model ile pafta uyumu, idare
kabulü.** Tasarımın hukuki ve mesleki uygunluğuna müellif ve idare karar verir.

## 2. Standart, biçim ve teknik sınır

| Konu | Karar |
|---|---|
| Şema | Hedef **IFC 4.3** (ISO 16739-1:2024), başlıkta `IFC4X3_ADD2`. EK-5 §5.4 "IFC 4.3 esas alınarak" der; buildingSMART'ın doğrulama servisi bu adı kabul eder; Finlandiya'nın ruhsat modeli de 4.3.2.0 ister. Şema adı kural kataloğunun bir satırıdır: TS EN ISO 16739-1'in yeni baskısı ya da idarenin yazısı kod değiştirmeden uygulanır. [ISO 16739-1:2024](https://www.iso.org/standard/84123.html) · [IFC 4.3](https://standards.buildingsmart.org/IFC/RELEASE/IFC4_3/) |
| Değişim dosyası | ISO 10303-21 STEP dosyası, `.ifc`, sıkıştırılmamış. Dosya içi `#n` başvuruları geçicidir; revizyonlar `GlobalId` ile karşılaştırılır |
| Görünüm tanımı | Eklerde geçmiyor. Bugünkü başlık `DesignTransferView`: duvar kesilmemiş gövde, açıklığı okuyucu çıkarıyor. Hedef iki çıktı profili (§8); ruhsat profilinin hangisi olacağı ek sayfası ve idarenin cevabıyla kesinleşir. Başlıkta yazan ad tek başına sertifika değildir |
| Öbür biçim ve sürümler | IFC4 ve IFC2x3 yalnız **okunur** (IFC-11). `.ifczip` ve `.ifcxml` yazılmaz; ruhsat denetimi yanlış sürümü ve uzantıyı açık hata sayar |
| Kod sahipliği | IFC'ye ait her şey `crates/shared/ifc` (`kentos-ifc`) içindedir: şema sözlüğü, okuyucu, yazıcı, eşleme, adlar, koordinat, katalog, doğrulama, IDS. Uygulamada yalnız bağdaştırıcı kalır (`application::arch_ifc`). Yolun `IfcAlignment` çıktısı aynı çekirdektedir ve her değişiklikte sınanır ([ADR 0908](../docs/adr/0908-road-stakeout-sheets-and-exchange.md)) |
| Okuma ve geometri motoru | Dosyayı okuyan ve gövdeleri üçgene çeviren motor IFClite'ın Rust çekirdeğidir (MPL-2.0). Yalnız `kentos-ifc`'nin içinde, isteğe bağlı özelliğin arkasında, sabit sürümle durur; türleri dışarı çıkmaz. Kabulü denemeye bağlıdır; geçemezse kendi okuyucumuz yazılır. Üçgen ağ gösterimdir, ölçü kaynağı değildir. Ürünün motoru bağımsız başvuru sayılmaz ([ADR 8061](../docs/adr/8061-ifc-viewer-and-reading-engine.md)) |
| Görüntüleyici | Uygulamanın içinde, gevşek bağlı: saf çekirdek yerel ve wasm32 için derlenir, arayüz bileşeni kendi paketindedir, uygulamayla dar bir yüzden konuşur. Ayrı program ve web gerektiğinde çıkarılır; şimdi yapılmaz (ADR 8061 §3, §6) |
| Sertifika | buildingSMART'ın yazılım sertifikası IFC 4.3 için de veriliyor. Başvuru kararı pilottan sonra ADR ile (IFC-11) |

## 3. Ekler ve kural kataloğu

| Ek | İçerik | Bugün | Kalan |
|---|---|---|---|
| **EK-1** | Proje kodu: yıl (2) + idarenin sıra numarası (4) | Künyede; `IfcProject.Name`; yer tutucu `000000` kimlik sayılmıyor | Kesin pakette eksik kod yayını durdurur |
| **EK-2** | Dosya adı ve sözlükleri (disiplin, alt disiplin, dosya türü ve tipi, kat, sıra, revizyon) | Model `…_MM_XXXX_MD_XXX_NNN_01_000.ifc`, form `…_FR_MKF_…` (ADR 8047 §7, §10) | Sözlüklerin tamamı katalogda; dosya adı, başlık ve pafta anteti çapraz denetlenir |
| **EK-3** | 2B mahal numarası ve adı | Numara var (ADR 8027) | Ad sözlüğü (Tablo 3.2), biçim denetimi (BIM-03) |
| **EK-4** | CAD katmanı, sembol, çizgi | Katman anahtarı var (ADR 8008) | Denetimi (BIM-03). EK-4'ün CAD öğe kodu ile EK-5'in kategori kodu ayrı sözlüklerdir (`DUVA` ve `DVR`) |
| **EK-5** | 69 zorunlu, 67 isteğe bağlı sınıf; kategori kodları; ad kalıbı | 15 sınıf yazılıyor | §4 |
| **EK-6** | Zorunlu sınıfların zorunlu alanları (Tablo 6.1–6.68); isteğe bağlılar (6.69–6.135); vekil sınıf yasağı | Çizimin bildikleri yazılıyor | §5 |
| **EK-7** | Proje, kişi, kuruluş, rol, koordinat sistemi, harita dönüşümü, gösterim bağlamı | ADR 8047 §7–§8 | §5, §7 |
| **EK-8** | Model gelişim seviyesi | Ölçülmüyor | §8 |
| **EK-9** | 21 soruluk form | Bellekteki modelden cevaplanıyor | §9 |

**Kural kataloğu.** Her satır şunları taşır: kaynak (yayım, ek, sayfa, tablo, satır); disiplin; kapsam koşulu; IFC sınıfı ve
alan yolu; şemadaki türü ve birimi; KentOS'taki kaynağı; zorunluluk (zorunlu, koşullu, isteğe bağlı); izinli değerler; denetim
kimliği; test bağlantısı.

- Zorunluluk **modelde bulunan** eleman için aranır: tablodaki her sınıf her binada oluşturulmaz.
- Katmanlar: temel profil (yönetmelik) → kurum profili (idarenin ek alt disiplini, ek alanları, yazılı yorumları) → proje.
  Üst katman alt katmanı kaynağını göstererek daraltır ya da genişletir.
- Her sınıf, öznitelik ve özellik seti adı IFC 4.3 şemasına ve resmî özellik şablonlarına karşı sınanır. Uyuşmayanlar
  (Tablo 6.37'nin `Tag`'i, boşluklu yazılmış ön tanımlı tipler, sayı denen metin alanı) fark listesindedir; şemada olmayan
  alan uydurulmaz.
- Özel sınıfın listede olmadığı durumda (giydirme cephe paneli gibi) vekil sınıfa ya da üst sınıfa sessizce düşülmez; karar
  kaynağıyla kurum profiline yazılır.

## 4. Sınıf kapsamı (EK-5 Tablo 5.1; mimari modelde karşılaşılan 29 sınıf)

✓ yazılıyor · ◐ yazılıyor, eksikli · ✗ yazılmıyor.

| IFC sınıfı | Kod | KentOS'taki kaynağı | Bugün | Kalan | Paket |
|---|---|---|---|---|---|
| IfcProject | — | Künye | ◐ | açıklama, proje tipi | IFC-02 |
| IfcSite | SAH | Parsel, imar durumu, vaziyet planı | ✓ nesne tipi Parsel, toplam alan (ADR 8076) | — | IFC-02 |
| IfcBuilding | BNA | Çizim; programın binaları (ADR 8072) | ✓ bina başına dosya, kendi ±0.00'ı ve alanlarıyla | eksik alanlar | IFC-02 |
| IfcBuildingStorey | BNK | Katlar | ✓ taşıyıcı döşeme kotu, brüt/net yükseklik döşemelerden (ADR 8076) | baskın kotu olmayan kat | IFC-02 |
| IfcSpace | MHL | Mahal | ◐ hep `INTERNAL`, düz prizma | türü, kapalı hacmi, kaplama kalınlıkları | IFC-02, IFC-06 |
| IfcSpatialZone | MZN | Emsal alanı (BIM-08) | ✓ (ADR 8069) | DIGER | IFC-03 |
| IfcZone | ZON | Bağımsız bölüm, eklenti, ortak alan; yangın kompartımanı | ✓ (ADR 8069, 8088) | — | IFC-03 |
| IfcWall | DVR | Duvar | ◐ hep `STANDARD` | türü; yangın; katmanları | IFC-02, IFC-04 |
| IfcCurtainWall | GDC | Giydirme cephe (BIM-09) | ✓ gövdesi panelleri, USERDEFINED, dikme ve kayıtları parçaları (K45, ADR 8070) | panel yerine kapı ve pencere | IFC-03 |
| IfcSlab | DSM | Döşeme, balkon, teras | ◐ hep `FLOOR` | türü; yangın | IFC-02 |
| IfcRoof | CAT | Çatı | ◐ tipi `NOTDEFINED` | türü; yangın | IFC-02 |
| IfcColumn | KLN | Kolon | ◐ | yangın; dayanım | IFC-02 |
| IfcBeam | KRS | Kiriş, lento, hatıl | ◐ | yangın; dayanım | IFC-02 |
| IfcMember | ELM | Dikme, cephe kaydı (BIM-09) | ✓ dikme (POST) ve kayıt (USERDEFINED "Kayıt") kolonun ve kirişin eleman rolüyle; giydirme cephenin çerçeveleri MULLION (K46, ADR 8070) | eğimli eleman | IFC-03 |
| IfcDoor | KAP | Kapı | ◐ | yangın; acil çıkış | IFC-02 |
| IfcWindow | PNC | Pencere | ✓ | — | — |
| IfcOpeningElement | ACL | Açıklık, niş, döşeme deliği ve boşluk mahalinin parçası (ADR 8071 §5) | ✓ | — | IFC-06 |
| IfcStair | MRD | Merdiven | ✓ tipi kollarından, Betonarme (ADR 8070) | yangın; acil çıkış beyanla (ADR 8068) | IFC-02 |
| IfcRamp | RMP | Rampa | ✓ eğik kollar ve sahanlıklar, tipi kollarından (ADR 8070) | korkulukları | IFC-03 |
| IfcRailing | KRK | Korkuluk | ✓ | — | — |
| IfcCovering | KPL | Kaplama, asma tavan (BIM-09) | ✓ döşemenin kaplama rolü (mahale bağlı ya da elle, tipi ve gövdesiyle; K47) ve mahalin seti adları ve kalınlıklarıyla (K44, ADR 8070) | duvar kalınlığı; `IfcRelCoversSpaces`; katman takımı | IFC-03 |
| IfcChimney | BAC | Çatı bacası, duvardaki baca | ✓ tek eleman, duvarın boşluğunu doldurur (ADR 8070) | — | IFC-03 |
| IfcGridAxis | AKS | Aks | ✓ `IfcGrid` U/V (ADR 8070); tek yönlü takım kayıpta | RADIAL | IFC-03 |
| IfcGroup | GRP | — | ✗ | kataloğun ne için istediği belli olunca | IFC-03 |
| IfcFurniture | MBL | Kitaplık bloğu (mobilya) | ✓ kartındaki sınıf ve gövdeyle (ADR 8070) | gerçek biçim | IFC-03 |
| IfcSanitaryTerminal | STT | Kitaplık bloğu (vitrifiye) | ✓ aynı | gerçek biçim | IFC-03 |
| IfcTransportElement | TSM | Asansör bloğu | ✓ kapasitesi KISI/YUK özniteliklerinden (ADR 8070) | — | IFC-03 |
| IfcFooting | TML | — (statik disiplininin) | ✗ | yazılmaz; statik modelden altlık | — |
| IfcSign | TBL | — | ✗ | gerek çıkınca | — |

`building.rs`'in `Class` listesindeki taşıma elemanı, mobilya, sıhhi tesisat ucu, baca, rampa, giydirme cephe ve onun
dikme ve kayıtları, dikme ve kayıt (eleman) ve kaplama artık kuruluyor (ADR 8070).

**Mekânsal yapı.** `IfcProject` → `IfcSite` → `IfcBuilding` → `IfcBuildingStorey` (`IfcRelAggregates`); eleman katında
(`IfcRelContainedInSpatialStructure`); mahal katının parçası. Çok kata uzanan elemanın birincil katı bellidir, öbür katlar
başvuruyla bağlanır. Dışa aktarılamayan her nesne kimliği, sınıfı, sebebi ve çözümüyle listelenir.

## 5. Alan kapsamı (EK-6, EK-7)

Bugün sütunu önce koddan okundu; B1'de künyeli bölüm binasının dosyasından ölçüldü (ADR 8062 §10) ve ayrışan satırlar
düzeltildi. Bir alan "verisi çizimde olduğunda" yazılıyorsa örnekte yoksa ◐ sayıldı.

| Sınıf | Zorunlu alan | Bugün | KentOS'taki kaynağı | Paket |
|---|---|---|---|---|
| Bütün yapı elemanları | `Name` (EK-5 kalıbı) | ✓ | — | — |
| | `Tag` | ◐ kapı ve pencerede numarası; ötekilerde nesnenin kimliği | elemanın numarası ya da tip kodu (S1, K101); yoksa kimliği | IFC-02 |
| | `PredefinedType` | ◐ çatı ve merdivende `NOTDEFINED`; duvar, döşeme, mahal tek değer | nesnenin türünden; tanımlı özel tipte şemanın izin verdiği `USERDEFINED` ve tip adı | IFC-02 |
| | Malzeme, "Ad_Dayanım" | ◐ ad var, dayanım yok; çatı, merdiven, korkuluk malzemesiz | malzeme ve dayanımı (BIM-07) | IFC-02 |
| Duvar, döşeme, kolon, kiriş, çatı, kapı, merdiven, kaplama, giydirme cephe | `FireRating` | ✗ | tipin ya da örneğin beyanı (BIM-07) | IFC-02 |
| Kapı, merdiven | `FireExit` | ✗ | müellifin onayladığı çıkış rolü (BIM-07) | IFC-02 |
| Duvar, döşeme, çatı, kapı, pencere | `IsExternal` | ✓ | dış çizgi kuralı; örnekte değiştirilebilir | — |
| Kolon | `LoadBearing` | ◐ hep doğru | malzemesi ve rolü; mimari dikme ayrımı | IFC-02 |
| Mahal | zemin, duvar, tavan kaplaması | ✓ | mahal listesi (ADR 8038) | — |
| | üç kaplamanın kalınlığı | ✓ döşeme ve tavan döşemelerin katmanından (ADR 8070 §6), duvar yüzün kaplamasından (ADR 8076 K57) | mahalin kaplamaları (BIM-07) | IFC-02 |
| | türü | ◐ | kullanımı (BIM-08) | IFC-02 |
| Kat | net alan | ✓ | mahallerinin toplamı | — |
| | taşıyıcı döşemenin kotu | ✓ mahallerin alanının yarısından çoğuyla (ADR 8076 K54) | katın kotu ve döşeme katmanları (ADR 8040) | IFC-02 |
| Bina | kimlik, kalıcı kimlik değil, kat sayısı, adres, net kat alanı | ✓ | künye | — |
| | kullanım, planlanan brüt alan, taban alanı, brüt kat alanı | ◐ verisi çizimde olduğunda; künyeli bölüm binasında yazılmadı | program, imar durumu | IFC-02 |
| | `Pset_Address.PostalBox` | ✓ künyenin kutusu (ADR 8076 K56) | künye | IFC-02 |
| | yapım yöntemi, yangın koruma sınıfı, planlanan net alan | ✗ | künye (BIM-07) | IFC-02 |
| | yükseklik | ✓ katların brüt yüksekliği, yalnız eşitken (ADR 8076 K55) | bina yüksekliği | IFC-02 |
| | saçak yüksekliği (`EavesHeight`) | ✓ tanımı net kat yüksekliği, yalnız eşitken (ADR 8076 K55) | döşeme katmanları | IFC-02 |
| Saha | uzun ad, ±0.00'ın kotu, emsal, TAKS, adres, ada/parsel, tapu, `TREpys_ParselOzellikSeti` | ✓ | künye, imar durumu, vaziyet planı | — |
| | nesne tipi | ✓ Parsel (ADR 8076 K52) | katalog (B1) | IFC-02 |
| | parsel alanı, `Pset_SiteCommon.TotalArea` | ✓ (ADR 8076 K53) | imar durumu | IFC-02 |
| Proje | adı (proje kodu) | ✓ | künye | — |
| | açıklama; `Pset_ProjectCommon.ProjectType` | ✗ | künye | IFC-02 |
| Kişi, kuruluş, rol | sicil, ad, soyad, vergi no, adres, rol | ✓ (ADR 8047 §7) | ad ve soyad ayrı alanlardan (bugün son kelimeden ayrılıyor) | IFC-02 |
| | `IfcPerson.Roles`, `IfcOrganization.Roles`, `IfcActorRole.UserDefinedRole` | ✗ (rol yalnız kişi-kuruluşta) | künye | IFC-02 |
| Gösterim bağlamı (EK-7 7.8) | `TrueNorth` | ✓ sistemin meridyeninden, yoksa kuzey okundan (ADR 8076 K58) | kuzey oku, projenin kuzeyi | IFC-02 |
| Ölçüm anotasyonu (EK-7 7.9) | `IfcAnnotation`: `Name`, `Description`, `PredefinedType = SURVEY` | ✗ | Ölçüm ve vaziyet bilgisi. Uygulanabilirlik B1’de kaynak ve kurum profiliyle belirlenir | IFC-02 |
| Koordinat sistemi | TUREF dilimi, datum, projeksiyon, dönüşüm | ✓ (ADR 8047 §8; yerel sistem döndürülüp ölçeklenerek ADR 8072 §3) | `TrueNorth`; kontrol noktalarının projede saklanması | IFC-05 |

**Değer durumları.** Bilinmiyor, uygulanmaz, hayır, sıfır ve eksik ayrıdır. Boş metinle yazılmış bir özellik gerekli bilgiyi
karşılamaz; değeri olmayan alan yazılmaz ve "eksik" diye raporlanır. Standart şablonlar, Türkiye'nin `TREpys_` setleri ve
KentOS'un çalışma bilgisi ayrı ad alanlarındadır; aynı bilgi için dosyada iki çelişen kaynak bulunmaz.

## 6. Adlar

| Ne | Kalıp | Bugün | Kalan |
|---|---|---|---|
| Model dosyası | `ProjeKodu_BinaKodu_MM_AltDisiplin_MD_Tip_Kat_Sıra_Revizyon.ifc` (EK-2) | ✓ `naming::model_file`; çok binada bina kodu (ADR 8072 §2) | Alt disiplin ve tip sözlükleri katalogdan |
| Proje | `IfcProject.Name` = proje kodu (EK-1) | ✓ | — |
| Varlık | `Disiplin-KategoriKodu-Açıklama` (EK-5 §5.5) | ✓ `MM-DVR-Dış_duvar_25` | Ayırıcı kuralı kalıbın verisi olur: Md. 11/2 "orta tire", EK-5 "alt çizgi". İdarenin yazılı yorumu profile girer |
| Mahal | Md. 10/2: EK-5'e göre | `MM-MHL-<no>_<ad>`; uzun ad mahalin adı | Kapandı (ADR 8062 K02): EK-5 5.5.4 numarayı ve adı açıklamada ayrı ya da birlikte kullandırır; KentOS ikisini birlikte yazar |
| İdarenin alt disiplini | Dört karakter, EK-2 ile çakışmaz (Md. 11/3) | — | Kurum profili |

Sınıf adı, kategori kodu, Türkçe harfler ve açıklama ayırıcıları ayrı ayrı sınanır. Bayt olarak yazılan ad geri okunup
karşılaştırılır.

## 7. Konum, kot ve kimlik

- **Sistem (EK-7 Tablo 7.6–7.8).** `IfcProjectedCRS`: TUREF'in 3° dilimleri EPSG:5253–5259, merkez meridyeni, metre;
  `IfcMapConversion`: doğu, kuzey, yükseklik, X ekseninin doğrultusu, ölçek. EPSG kodunu yazmak dönüşüm yapmak değildir
  (kılavuz §5). [IfcMapConversion](https://standards.buildingsmart.org/IFC/RELEASE/IFC4_3/HTML/lexical/IfcMapConversion.htm)
- **Bugün (ADR 8072 §3).** Çizim projenin sistemindeyse dönüşüm düzlemde sıfır; yerel sistemdeyse taban sistem ve yerel
  düzlem döndürülüp ölçeklenerek (`IfcMapConversionScaled`); yükseklikte ±0.00'ın kotu. ±0.00'ın kotu bilinmiyorsa konum
  yazılmaz ve dışa aktarma ile form nedenini söyler.
- **Hedef.** İki seçenek: yerel orijinli model ve haritaya açık dönüşüm (öteleme, dönüş, ölçek), ya da bugünkü mutlak
  koordinat. Koordinat iki kez ötelenmez; bağıl kat kotuna mutlak kot iki kez eklenmez; bilinmeyen datumda sıfır varsayılmaz.
- **Yükseklik.** Projenin datumu (ortometrik ya da elipsoit), yerel röper, dönüşüm parametresi ve EK-7'nin notu ayrı
  kaydedilir. EK-7'nin düşey datum notu (ITRF96; gerektiğinde TUDKA99'a dönüşüm) sayfadan yeniden okunur.
- **Hassasiyet** (gösterim bağlamı), dönüşümün doğruluğu ve idarenin kabul ettiği ölçüm toleransı ayrı ayarlardır.
- **Kabul senaryosu.** Bölgeye yayılmış en az üç yatay kontrol noktası, bağımsız bir kot röperi, döndürülmüş yerel model.
  Eksen sırası, birim, ters dönüşüm ve başka bir disiplinin modeliyle üst üste binme bağımsız okuyucuda ölçülür.
- **Kimlik.** Gerçek nesnede kalıcı KentOS kimliği `GlobalId` olur (bugün böyle). Türetilen kök nesnelerde (açıklık,
  ilişki, tip, zon, grup) kimlik, sahibinin kimliğinden ve rolünden türeyen kararlı bir değerdir; dosya yolu ve sıra
  numarası kullanılmaz. **Yapıldı (18 Kasım 2026, [ADR 8117](../docs/adr/8117-persistent-global-ids.md), K228–K233):**
  binanın dosyasında kat, açıklık, emsal zonu, bağımsız bölüm, tek dosyalık proje, saha ve bina, setler ve bütün ilişkiler;
  sürüm 8 UUID; aynı çizim her gün aynı kimlikleri verir, uzaktaki bir değişiklik öbürlerini korur. Kalan: yol hizalaması
  dosyası, kopya, bölme, birleştirme ve geri alma sınamaları. Taşıma, yeniden adlandırma ve revizyon kimliği korur; kopya yeni kimlik alır; bölme, birleştirme ve
  silip geri alma sınanır. Disiplinler arası çakışan kimlik bulgudur.

## 8. Geometri, görünüm tanımı ve gelişim seviyesi

- **İki çıktı profili:**
  1. **Kesilmiş gövde:** duvarın ve döşemenin gövdesi açıklıkları çıkarılmış olarak yazılır; açıklık nesnesi ilişkisiyle
     durur, okuyucu fark almak zorunda kalmaz. Reference View'ın düzenidir; Finlandiya'nın kararnamesinin bunu istediği
     ikincil kaynaktandır (araştırma notu §6).
  2. **Tasarım aktarımı:** bugünkü çıktı; gövde bütün, açıklığı okuyucu çıkarır.

  Başlıktaki görünüm adı dosyanın gerçek içeriğini söyler. Ruhsat profilinin varsayılanı B1'in sayfa okuması ve pilot
  idarenin cevabıyla seçilir; o güne kadar ikisi de sınanır.
- **Gövdeler** kesit modelinden gelir (plan, kesit, 3B ve IFC aynı gövde; ADR 8042, 8047). Eğri duvar ve döner merdiven
  gerçek yayla; rampa eğik gövdeyle; mahal kapalı hacimle (BIM-10). Her gövde kapalı ve kendini kesmeyen bir katıdır.
- **Gelişim seviyesi (Md. 14, EK-8 §8.1–8.3).** Ruhsat modeli için LOD 300, şekil, boyut, konum ve yönelimin
  doğrudan ölçülebilir olmasını gerektirir. Alfanümerik bilgi EK-6 ve EK-7'den gelir. İdarenin farklı proje aşamaları
  için belirlediği bilgi gereksinimi, kaynağı ve aşamasıyla kurum profiline girer. Aşağıdaki tablo KentOS'un kabul
  örnekleridir. Makine denetimleri, müellifin 3B ve ölçekli kesit incelemesiyle tamamlanır.
  [EK-8, PDF s. 237](../mevzuat/bim/ek-8-gelisim-seviyesi.md)

| Nesne | Geometri kanıtı | Bilgi kanıtı |
|---|---|---|
| Duvar, döşeme, çatı | Boyut, konum, yön, kalınlık, sınır, kot; açıklıklar yerinde; plan ve kesitle aynı | Türü, malzemesi ve katmanları, dışarıda oluşu, yangın dayanımı ve kaynağı |
| Kapı, pencere | Duvarı ve kesilen açıklığı, kasanın yeri, eşik ve parapet, kaba ve net ölçü | Kodu ve tipi, açılımı, acil çıkış ve dayanım beyanı |
| Mahal | Hacmi kapalı, kotu belli, başka mahalle örtüşmüyor | Numarası ve adı, kullanımı, alanı, kaplamaları |
| Merdiven, rampa | Başlangıcı ve varışı, kolu ve sahanlığı, eğimi, temiz geçişi | Türü, kaçış ve erişilebilirlik rolü, doğrulayan müellif |

## 9. Doğrulama katmanları ve kontrol formu

1. **Okuma ve şema.** STEP sözdizimi, başlık, başvurular; şemanın tür, sayı ve WHERE kuralları. `kentos-ifc`'nin okuma
   yüzü (IFC-07; motoru ADR 8061'in denemesiyle seçilir) ve bağımsız okuyucu
   ([`arch_ifc_check.py`](../scripts/fixtures/arch_ifc_check.py), IfcOpenShell). Okuyucunun sürümü ve kapsamının sınırı
   raporda.
2. **Normatif kurallar.** buildingSMART'ın açık kural takımı, açık ve sentetik örneklerde. Servisin sınırı 256 MB'tır; ülkeye
   ve kuruma özgü kuralı denetlemez. [Kılavuz](https://buildingsmart.github.io/validate/user/index.html)
3. **Türkiye profili.** Katalogdan çalışan Python denetim betiği (ADIM 20'nin `catalog/standards/`'ı; bim-plan K10), yazılmış dosyanın
   olgularıyla (`kentos-ifc`'in okuma yüzü çıkarır). Aynı katalogdan **IDS 1.0**
   üretilir (sınıf, öznitelik, özellik, malzeme, parçası olma); IDS'in ifade edemediği geometri, kapalılık ve dosyalar arası
   uyum ayrı denetimlerdedir. Örnekler ikinci bir bağımsız denetleyiciyle de okunur; ayrışan her satır "katalog yanlış",
   "öbür araç yanlış" ya da "yorum" diye kapanır.
   [IDS](https://www.buildingsmart.org/standards/bsi-standards/information-delivery-specification-ids/)
4. **Geometri ve 2B.** Duvar, açıklık ve dolgu ilişkisi; kapalı gövde; örtüşme; kat ve kot; alanların dosyadan ölçümü.
   PDF/A, üretilen baytlarda bağımsız doğrulayıcıyla ([veraPDF](https://docs.verapdf.org/validation/)); pafta ile model
   arasında seçilmiş ölçüler ve sayılar karşılaştırılır.
5. **İnsan ve kurum.** Gelişim seviyesi, yangın ve imar istisnaları, bulguların çözümü, pilot idarenin cevabı.

Gerçek ruhsat modeli dış doğrulayıcıya yüklenmez; 1–4 çevrimdışı çalışır.

**Kontrol formu (EK-9 Tablo 9.1).** Bugün `kentos_ifc::quality` 21 soruyu bellekteki modelden cevaplıyor.

| Soru | Bugün | Hedef kanıt (yazılmış dosyadan) |
|---|---|---|
| 1 Dosya boyutu | Boş (sınır yayımlanmadı) | Dosyanın boyutu ve yayımlanan sınır; sınır yoksa "inceleme" |
| 2 Proje bilgileri | Künyeden | EK-7 alanları dosyada dolu |
| 3 Proje ve dosya adı | Künyeden | Dosya adı, başlık ve `IfcProject.Name` birbirine eşit ve kalıpta |
| 4 Mahal no ve adı | Yalnız varlığı | Kalıp, teklik, sözlük |
| 5 Varlık adları | Yapılışıyla evet | Her kök nesnenin adı kalıpta; disiplin ve kategori kodu sınıfıyla uyumlu |
| 6 Koordinat sistemi | Projenin ayarından | Sistem ve dönüşüm dosyada; kontrol noktası kanıtı |
| 7 Gelişim seviyesi | Elle | Makine denetimleri + müellifin 3B incelemesi (kayıtlı) |
| 8 Zorunlu sınıflar | Yapılışıyla evet | Kaynakta beklenen nesneler, IFC kimliği ve sınıfı üzerinden dosyada karşılanıyor; vekil sınıf yok |
| 9 Zorunlu alanlar | Hayır | Katalog satırları dolu; eksikler nesnesiyle |
| 10 Emsal | Elle | Aynı revizyonda mahal ve zon alanlarıyla tablo, kat ve `DAHIL` / `HARIC` / `DIGER` bazında eşleşir |
| 11 Proje, saha, bina, kat | Yapılışıyla evet | Tek proje; saha, bina, en az bir kat |
| 12 Mahal var | Modelden | Dosyada |
| 13 Örtüşme, doğru kat | Elle | Gerçek çokgenle örtüşme; katın kot aralığı |
| 14 Kapalılık, üst sınır | Yapılışıyla evet | Kapalı katı; üstü döşemede ya da çatıda |
| 15 Doğru kat ve kot | Elle | Her elemanın katı ve gövdesinin kot aralığı |
| 16 Yinelenen varlık | Elle | Yinelenen `GlobalId`; aynı sınıf, ad ve yerleşim |
| 17 Sınıflar doğru | Yapılışıyla evet | Sınıf ile kaynağın eşlemesi; vekil yok |
| 18 Sürüm ve uzantı | Müellife bırakılıyor | Başlıktaki şema adı ve dosya uzantısı |
| 19 Ön tanımlı tip | Yapılışıyla evet (çatı ve merdiven `NOTDEFINED`) | Tip, kaynak nesneye ve IFC şemasına uygun. Belirlenebilen tip yerine `NOTDEFINED` kullanılırsa bulgu; gerekçeli belirsizlik incelemeye gider. `USERDEFINED` için şemanın istediği tip adı aranır |
| 20 Özellik setleri | Yapılışıyla evet | Her özellik katalogdaki setinde |
| 21 PDF ile model | Modele girmeyen nesne yoksa evet | BIM-04'ün kaydı: aynı revizyon, güncel türevler, modelden gelmeyen çizim listesi |

Formun resmî Evet/Hayır'ı ile KentOS'un kanıt durumu ayrı saklanır; eksik inceleme "evet"e çevrilmez. Her bulgu nesneyi,
`GlobalId`'yi, sınıfı ve alan yolunu, maddeyi ve eki, ölçülen ve istenen değeri, düzeltmenin yerini söyler.

## 10. İş paketleri

Sıra ve kapılar [BIM planında](bim-plan.md#6-uygulama-sırası-b1b23). `[ ]` başlanmadı, `[~]` sürüyor, `[x]` bütün kabulü
sağlandı.

#### IFC-01 — Kaynak kataloğu ve başlangıç ölçümü [x]

**Yapıldı (15 Kasım 2026, [ADR 8062](../docs/adr/8062-digital-project-rule-catalog.md)):** katalog `kentos-ifc`'te
(`data/temel-2026.json`: 136 sınıf, 147 tablo, 1 146 satır, 37 soru; `data/kararlar-2026.json`); temel ve kurum profili
katmanları `kentos-digital-project`'te. IFC 4.3 sözlüğü (`data/ifc4x3-sozluk.json`, IfcOpenShell 0.9.0.post1'den) ile
karşılaştırma: 235 fark, her biri kararlı; bilerek yanlış yazılan `FireRatng` tek yeni bulgu olarak düşer. Başlangıç
ölçümü IfcOpenShell'le dosyadan: künyeli bölüm binasında 81 / 121. Kaynak–IFC kimlik eşlemesi (IFC-08'in beklentisi) bu
dilimde yok; IFC-08 ile gelir.

**Kullanıcı sonucu:** Dışa aktarılan her alan ve her denetim, yönetmeliğin sayfasını gösterir. [BIM-01](bim-plan.md#5-iş-paketleri)
ile aynı dilimdir.

- Katalog `kentos-ifc`'te veri olarak durur (§3'ün satır alanları); temel profil ve kurum profili katmanları.
- Şema karşılaştırması: katalogdaki her sınıf, öznitelik ve özellik adı IFC 4.3 şemasına ve resmî şablonlara karşı sınanır;
  fark listesi üretilir.
- Başlangıç ölçümü: bölüm binasının ve 2+1’in bugünkü dosyası katalog satırlarına karşı ölçülür. Kaynak modelden,
  seçilen bina/evre ve teslim profiline göre beklenen nesnelerin IFC kimliği ve sınıfı çıkarılır. Bölünen veya birleşen
  temsillerde kaynak–IFC eşlemesi açık kaydedilir. IFC-08 bu beklentiyi yazıcıdan bağımsız olarak dosyayla karşılaştırır.

**İlk kabul:** her satırın sayfası var; bilerek yanlış yazılan bir özellik adı şema karşılaştırmasında düşer; ölçüm raporu
ADR'de.

#### IFC-02 — Zorunlu alanların tamamlanması [~]

**Yapıldı (16 Kasım 2026, [ADR 8068](../docs/adr/8068-fire-performance-material-declarations.md)):** `kentos_ifc::required`
katalogdaki EK-6 ve EK-7 zorunlu satırlarını yazılacak modelle karşılaştırır (öznitelik, özellik ve miktar seti, malzeme;
`NOTDEFINED` tip değil, boş söz değer değil); eksikler `cad.modelGaps` olgusu, `dijital_proje.zorunlu` nesneleriyle hata;
EK-9/9 modelden. Yangın sınıfı, acil çıkış, yapım eylemi, yangın koruma sınıfı, net planlanan alan ve `Ad_Dayanım` yazılıyor.
Projenin açıklaması (künyenin "Proje açıklaması (IFC modelinde)" kutusu), tipi (`Pset_ProjectCommon.ProjectType`, yapı
türünden, K50) ve aktör rolleri 16 Kasım'da ([ADR 8074](../docs/adr/8074-ek9-from-the-written-file.md) §3).
**Yapıldı (17 Kasım 2026, [ADR 8076](../docs/adr/8076-ifc-mandatory-fields-complete.md), ilk kabul tuttu):** sahanın
tipi "Parsel" (K52) ve toplam alanı (K53); katın taşıyıcı döşeme kotu ve brüt/net yüksekliği döşemelerin taşıyıcısından,
mahallerin alanının yarısından çoğuyla (K54, K55); binanın `Height` ve `EavesHeight`'ı yalnız katlarınki aynıyken, koşul
zorunlu ölçüde, ikinci okuyucuda ve IDS'in sözünde (K55); künyenin posta kutusu (K56); mahalin duvar kaplamasının kalınlığı
duvar yüzünün kaplamasından (K57); bağlamın gerçek kuzeyi projenin sisteminden, yoksa kuzey okundan (K58); kullanım türü
program yoksa künyeden. Yazar gibi doldurulmuş künyeli kesit binasında zorunlu satırlardan eksik yok; IfcOpenShell 0
bulgu, IfcTester ile ayrışma yok; gerçek BIM paketi gerçek kuralla yayımlanıyor (ADR 8076 §5).
**Kalan:** `Tag` politikası, kişinin adı ve soyadı ayrı alanlardan, EK-7 Tablo 7.9 ölçüm anotasyonu; baskın kotu olmayan
katın ve çatı altındaki son katın koşulu; duvardan ayrı çizilmiş mahalin duvar kaplaması (ADR 8076 §7).

**Kullanıcı sonucu:** Yazılan her sınıf, yönetmeliğin o sınıf için istediği bütün alanları taşır; taşıyamadığını nesnesiyle
söyler. Verisi BIM-07 ve BIM-08'den gelir.

- §5'in tablosundaki ✗ ve ◐ satırlar: yangın dayanımı, acil çıkış, dayanım, ön tanımlı tipler, kaplama kalınlıkları, taşıyıcı
  döşemenin kotu, binanın ve sahanın eksik alanları, projenin açıklaması ve tipi.
- Değer durumları (§5): girilmemiş değer yazılmaz ve eksik sayılır; varsayılan doldurulmaz.
- `Tag` politikası: elemanın numarası ya da tip kodu; yoksa kimliği.
- Kişide ad ve soyad ayrı alanlardan.
- EK-7 Tablo 7.9’daki `IfcAnnotation` / `SURVEY` için uygulanabilirlik, kaynak ölçüm verisi ve zorunlu alanlar
  katalogda ayrı satırdır. Uygulanan durumda anotasyon gerçek ölçüm kaynağına bağlanır.
  [EK-7, PDF s. 221 ve 233](../mevzuat/bim/ek-7-proje-siniflari.md)

**İlk kabul:** künyeli bölüm binasında katalogdaki zorunlu satırların hepsi dolu; bilerek boş bırakılan her alan bulguda
nesnesiyle; bağımsız okuyucu şema kurallarında 0 bulgu; çatının ve merdivenin tipi dolu.

#### IFC-03 — Eksik sınıflar ve zonlar [~]

**Yapıldı (16 Kasım 2026, [ADR 8069](../docs/adr/8069-room-zone-far-model.md)):** emsal zonları (`IfcSpatialZone`,
`TREpys_EmsalOzellikSeti.EmsalDurumu`, katına `IfcRelReferencedInSpatialStructure` ile) ve bina programının bölümleri
(`IfcZone`, mahalleri `IfcRelAssignsToGroup` ile). **Yapıldı ([ADR 8070](../docs/adr/8070-missing-building-elements.md)):**
rampa, baca, niş, aks ızgarası, asansör, vitrifiye ve mobilya sınıfları; merdivenin tipi; kaplama mahal setiyle (K44);
bağımsız okuyucu (IfcOpenShell) bölüm binasının dosyasında 0 bulgu; giydirme cephe (`IfcCurtainWall`, gövdesi panelleri:
K45) ve dikme ile kayıtları (`IfcMember` MULLION, `IfcRelAggregates`), doğrulayıcıda 0 bulgu; dikme ve kayıt (`IfcMember`
POST ve USERDEFINED, K46), doğrulayıcıda 0 bulgu; rampalı, giydirme cepheli, asansörlü örnek binada kayıp listesi boş,
`cad.arch.ifc` yeni sınıfları da sayar (ADR 8070 §9). **Kalan:** DIGER durumu.

**Kullanıcı sonucu:** Modeldeki her yapı nesnesi kendi sınıfıyla dışa gider; emsal alanları ve bağımsız bölümler dosyadadır.
Nesneleri BIM-08 ve BIM-09'dan gelir.

- **Emsal:** `IfcSpatialZone`, `TREpys_EmsalOzellikSeti.EmsalDurumu` (DAHIL, HARIC, DIGER), açıklaması (dayandığı madde) ve
  alanı; katına ve binasına şemanın izin verdiği ilişkiyle bağlı. EK-9/10 için mahal ve zon alanları kat ve emsal
  sınıfı bazında tabloyla doğrulanır. `IfcSpace` net alanı EK-6 Tablo 6.125’te isteğe bağlıdır. EK-9/10 kanıtı,
  uygun miktar kaydı veya IFC geometrisinden doğrulanabilir hesapla sağlanır.
- **Gruplar:** `IfcZone` (bağımsız bölüm, eklenti, ortak alan), mahallerini gruplar; adı numarası, uzun adı adı, nesne tipi
  türü. Yangın kompartımanı da bir `IfcZone`: standart paketin bulduğu kompartımanın mahalleri, adı
  `MM-ZON-Yangın_kompartımanı_N`, nesne tipi "Yangın kompartımanı" ([ADR 8088](../docs/adr/8088-fire-compartments-as-ifc-zones.md)).
- **Elemanlar:** rampa, giydirme cephe ve dikmeleri, kaplama, baca, aks ızgarası ve eksenleri, asansör, vitrifiye, mobilya.
- Vekil sınıf hiçbir durumda yazılmaz (EK-6 §6.1.3); karşılığı olmayan nesne kayıp listesindedir.

**İlk kabul:** rampalı, giydirme cepheli, asansörlü örnekte her yeni sınıf dosyada ve sayıları çizimle eşit; emsale dahil
zonların alanı kat ve sınıf bazında Emsal ve TAKS tablosuyla uyumlu. İki katta birbirini dengeleyen alan farkı genel
toplam eşit kalsa da bulgu verir. Bağımsız okuyucu 0 bulgu verir ve kayıp listesi boştur.

#### IFC-04 — Tipler, katmanlı malzeme ve ilişkiler [~]

**Kullanıcı sonucu:** Karşı programda duvar stili bir tip, katmanları sırası ve kalınlığıyla görünür. Verisi BIM-12'den gelir;
uyum için şart değildir.

- Tip nesneleri ve tipe bağlama; tipte ve örnekte hangi değerin geçerli olduğu.
- Katmanlı malzeme takımı ve kullanımı (duvar, döşeme, çatı, kaplama); katman uygulanmayan yerde bileşen takımı.
- Çok kata uzanan elemanın öbür katlara başvurusu.

**İlk kabul:** bir duvar stili tek tip olarak yazılır ve bütün duvarları ona bağlıdır; katmanlar sırasıyla ve kalınlığıyla
bağımsız okuyucuda okunur.

**Durum (17 Kasım 2026, [ADR 8105](../docs/adr/8105-wall-style-types-and-layer-sets.md)):** ilk kabul sağlandı: her duvar
stili bir `IfcWallType`, duvarları `IfcRelDefinesByType` ile ona bağlı, tipler `IfcRelDeclares` ile projede; her duvarın
katman takımı (sol yüzden sağa, adı, malzemesi, kalınlığı), aynı takım paylaşılır. IfcOpenShell kesit binasında iki tipi,
8'er duvarı ve katmanları sırasıyla okur. **Kalan:** kolon, döşeme, kiriş, çatı tipleri (BIM-12), katman takımının
kullanımı (duvarlar kendi eksen çerçevesinde yazılınca), çok kata uzanan elemanın başvuruları.

**Durum (17 Kasım 2026, [ADR 8106](../docs/adr/8106-element-types-and-their-exceptions.md)):** eleman tipleri de IFC
tipidir: `IfcColumnType` (dikmede `IfcMemberType`), `IfcBeamType`, `IfcSlabType` (kaplamada `IfcCoveringType`),
`IfcRoofType`, `IfcRailingType`, `IfcStairType`; elemanları ona bağlı, tipler projede bildirilmiş. IfcOpenShell kesit
binasında 16 kolonu, 8 kirişi, döşemeleri ve merdiveni tiplerinde okur. **Kalan:** kolon ve kiriş tipinin profil takımı
(`IfcMaterialProfileSet` ve kullanımı), katman takımının kullanımı, çok kata uzanan eleman.

#### IFC-05 — Konum, kot, çok bina ve projenin ortak alanları [x]

**Yapılan (16 Kasım 2026, [ADR 8072](../docs/adr/8072-buildings-shared-basement-location.md) §1–§4, BIM-11 ile):** bina
başına model dosyası (`cad.arch.ifc` `building`; masaüstünde her bina EK-2 adlı ayrı dosya); proje ve saha her dosyada
aynı kimlikle, başka ortak kimlik yok; her bina kendi ±0.00'ında, katları ona göre, nesneler katlarına göre (çift
öteleme yok); ortak bodrum sayıldığı binada; IfcOpenShell iki dosyada katların mutlak kotlarını yerleşim zincirinden doğru
okur. Konum (§3): yerel sistemli projede IFC'nin sistemi taban (TUREF dilimi), dönüşüm yerel düzlem: öteleme yerel
sıfırın harita koordinatı (proje baz noktası ile ölçüm noktası çakışık), X ekseni dönüşün doğrultusu, ölçek IFC 4.3'ün
`IfcMapConversionScaled`'ının FactorX/FactorY'si (`Scale` birim içindir; yükseklik ölçeksiz); bina ve yol aynı işlevden
(`ifc_system`); kaydıran ya da aynalayan afin, coğrafi sistem yazılmaz ve söylenir. ±0.00'ın kotu bilinmiyorsa dönüşüm 0
yüksekliğe yazılmaz: konum yazılmaz, `export_losses` ve EK-9'un 6. sorusu nedenini söyler. **İlk kabul:** döndürülmüş
yerel modelde (23,5°, ölçek 0,99989, gürültülü anket) üç kontrol noktası IfcOpenShell'de anketteki yerine oturtmanın
kendi artığı içinde (1e-6 m; en çok 0,45 mm) düşer ve geri döner; katlar haritada ±0.00'ın kotunda ve 3 m üstünde;
yerleşim zinciri (döngü, yabancı yerleşim) bütün örnek dosyalarda denetlenir, çift öteleme mutlak kotlardan. Ortak
bodrum (§4): her binanın dosyasında binanın kendi alanları (GrossPlannedArea, GrossFloorArea, FootprintArea), iki
dosyanınki toplamı projeninki, bodrum yalnız A'da; taban alanı iki bloğun zemin katından. İlk kabulün dört koşulu
sağlandı. **İyileştirmeler:** `TrueNorth`; kontrol noktalarının projede saklanması; idarenin ölçüm toleransı kurum
profiliyle; gelen IFC'nin zinciri okuyucuyla (IFC-07).

**Kullanıcı sonucu:** Model, karşı programda doğru yerde ve doğru kotta açılır; pilotta iki blok iki dosya olur ve ortak proje bilgileri
ikisinde aynıdır. Nesneleri BIM-11'den gelir.

- §7'nin hedefleri: yerel orijin ve harita dönüşümü; kontrol noktaları; düşey datum; sıfır varsayımı yok.
- İlk çok bloklu pilotta bina başına model dosyası kullanılır (EK-2 §2.3: ayrı hazırlanan bloklarda bina kodu).
  Ortak proje ve saha alanları eşleşir. Her bloğun ±0.00’ı ortak saha röperine ayrı bağlanır. Teslim kapsamı kurum
  profilinden gelir. [EK-2 §2.3](../mevzuat/bim/ek-2-dosya-isimlendirme.md)
- Yerleşim zinciri: saha → bina → kat → eleman; döngü ve çift öteleme denetimi.

**İlk kabul:** döndürülmüş yerel modelde üç kontrol noktası bağımsız okuyucuda harita koordinatını yazılı tolerans içinde
verir. Giriş kotları farklı iki blok ve ortak bodrum pilot dosyalarında doğru açılır; ortak bodrum alanı bir kez
sayılır. Kotu bilinmeyen ±0.00 eksik olarak bildirilir.

#### IFC-06 — Geometri ve görünüm tanımı [~]

**Yapılan (16 Kasım 2026, [ADR 8071](../docs/adr/8071-geometry-accuracy.md), BIM-10 ile):** yay profilde gerçek yay
(`IfcIndexedPolyCurve`, `IfcArcIndex`); duvar, döşeme, kolon, kiriş ve kaplamanın miktar setleri kaynaktan; kapalılık
denetimi (`ifc_open_body` uyarısı); **iki çıktı profili:** Tasarım aktarımı (gövde bütün, açıklık 'Body', başlık
`DesignTransferView`) ve kesilmiş gövde (Reference View: gövde bantlarla kesik, açıklık 'Reference', başlık
`ReferenceView`), `cad.arch.ifc`'nin `view`'ı ve Dosya › Dışa aktar'da iki eylem; eğri cepheli örnekte iki dosya da
IfcOpenShell'de 0 bulgu, eleman sayıları eşit, hacimler kapalı formlarla aynı (5,8·10⁻⁷ ve 5,2·10⁻⁷). **Kalan:** gelişim
seviyesi kanıtının müellif incelemesi kaydı (IFC-08/BIM-14 ile); açık kural takımı denetimi (servis ya da kural takımı
sahibin onayıyla); ruhsat profilinin seçimi (§8).

**Kullanıcı sonucu:** İki çıktı profili (§8) iki bağımsız okuyucuda aynı sayıları ve hacimleri verir; başlıktaki görünüm adı
dosyanın içeriğiyle uyuşur. Gövdeleri BIM-10'dan gelir.

- Kesilmiş gövde profili; tasarım aktarımı profili.
- Yay, eğik gövde, kapalı mahal hacmi, döşeme boşluğu.
- Gelişim seviyesi kanıtı (§8'in tablosu): makine denetimleri ve müellifin incelemesinin kaydı.

**İlk kabul:** iki profilde eleman sayıları eşit; açıklıklı duvarın hacmi iki profilde yazılı tolerans içinde aynı; bütün
gövdeler kapalı; sentetik örnek açık kural takımından uyarısız geçer.

#### IFC-07 — Okuma yüzü ve motor denemesi [x]

**Kullanıcı sonucu:** KentOS yazdığı dosyayı geri okur; bozuk ya da çok büyük bir dosya uygulamayı düşürmez, sebebini satırıyla
söyler. Doğrulamanın (IFC-08), görüntüleyicinin (IFC-12), altlık modelin (IFC-09) ve idare incelemesinin (BIM-18) ortak
temelidir. IFC-01'i beklemez; kataloğa bağlı olan IFC-08'dir.

- **Motor denemesi önce** ([ADR 8061](../docs/adr/8061-ifc-viewer-and-reading-engine.md) §7): IFClite'ın Rust çekirdeği
  sabit sürümle yalnız `kentos-ifc`'e bağlanır. Derleme (yerel ve wasm32), kendi dosyalarımız, Türkçe harfler, açık
  örnekler, bozuk girdi, iptal, büyük koordinat, süre ve bellek ölçülür. Bağımlılık sahibin onayıyla eklenir (kılavuz §3).
- **Okuma yüzü** (`kentos-ifc`, `read` özelliği): özet (şema, başlık, birimler, koordinat sistemi ve harita dönüşümü),
  nesneler (`GlobalId`, sınıf, tip, ad), yapı ağacı, ters ilişki dizini, istenince özellik ve miktar setleri, bulgular.
  Girdi bayttır; motorun türleri yüzde görünmez.
- **Şema sözlüğü:** sınıflar, öznitelik adları ve sıraları motorun üretilmiş tablosundan gelir ve resmî IFC 4.3 şemasına
  karşı sınanır. Tür ve sayı denetimine yetmediği yerde tablo resmî şemadan üretilir.
- **Sınırlar:** boyut, derinlik, süre, bellek; iptal; döngüsel başvuru ve yerleşim bulgudur. Kullanıcı girdisinde panik
  üretilmez (kılavuz §14); motorun çağrıları sınırda sarılır.
- **Gövde ağları** (`geometry` özelliği; `mesh` adı `kentos_ifc::mesh`'in, ADR 8061 §11.5) bu pakette yalnız denenir;
  ürüne görüntüleyiciyle girer (IFC-12).
- **Deneme geçemezse** kendi ISO 10303-21 ayrıştırıcımız ve şemadan üretilen sözlük yazılır (ADR 8061 §8); yüz değişmez.

**İlk kabul:** ADR 8061 §7'nin tablosu ölçülmüş ve karar ADR'ye yazılmış; yazılan her örnek geri okununca aynı nesne
çizgesini verir; bilerek bozulan dosyalar (kesik satır, olmayan başvuru, döngü, yanlış şema adı) hata verir, panik vermez;
açık IFC 4.3 örnekleri okunur; ölçeklenmiş modelde süre ve bellek tabanı yazılır.

**Durum (16 Kasım 2026, [ADR 8061](../docs/adr/8061-ifc-viewer-and-reading-engine.md) §11):**

- Yapılanlar:
  - Okuma yüzü `kentos_ifc::read` (`read` özelliği): özet, nesneler, ağaç, ters ilişki dizini, istenince setler, satırıyla
    bulgular, sınırlar, iptal ve panik sınırı. Gövde denemesi `geometry` özelliğinde.
  - KentOS'un yazdığı 11 dosya IfcOpenShell'in okumasıyla nesne nesne ve set set aynı. Başka bir yazıcının IFC4 ve IFC 4.3
    dosyaları da aynı (`fixtures/ifc/v1/`).
  - Motorun IFC 4.3 tablosu yayımlanmış şemayla, KentOS'un kullandığı 152 sınıfta aynı.
  - Bozuk dosyaların hepsi hata ya da bulgu verdi, panik vermedi.
  - TUREF koordinatında köşe hatası 1,05e-7 m.
  - Taban: 135 MB'lık dosyada yüz 0,72 s, gövdeler 8,39 s
    ([ölçüm](../docs/perf/ifc-okuma-denemesi-2026-11-16.md)).
  - Gerçek dünyadan 35 dosya (buildingSMART'ın sertifika veri setleri, CC BY 4.0) iki okuyucuda aynı okundu; 7'si
    `fixtures/ifc/v1/disaridan/` altında her sınamada okunur (ADR 8061 §11.7).
- Kararlar (ADR 8061 §11.6): boyut artışı kabul edildi; örnekler yalnız lisansı okunmuş kaynaktan alındı; profil
  yayları gösterim sınırı olarak kabul edildi.
- Kalan yok. Okuma yüzünü ürüne görüntüleyici (IFC-12) ve dosyadan doğrulama (IFC-08) bağlar.

#### IFC-08 — Yazılmış dosyadan doğrulama, IDS ve kontrol formu [~]

**Yapıldı (16 Kasım 2026, ilk dilim, [ADR 8074](../docs/adr/8074-ek9-from-the-written-file.md)):** `kentos-ifc` dosyanın
olgularını verir (`read::file_facts`, `kentos.ifcFacts` 1; gövdelerle), kaynağın beklentisini `expected` verir; formun 21
sorusu `dijital_proje.form`'dadır (resmî cevap ile kanıt ayrı; her bulgu nesnesi, maddesi, ek satırı ve sayfasıyla).
Masaüstü dosyayı bellekte yazıp okur: Mimari denetim, çizimdeki form ve Ruhsat takımı aynı olgulardan. Bugünkü
`kentos_ifc::quality` ve `digital_audit.rs` taşındı. Katalogdan IDS 1.0 (`crates/shared/ifc/data/temel-2026.ids`, 78 tanım,
356 gereklilik; yapısı ve eksikleri KentOS'un kendi okumasında kuralla aynı). Eksik kapı durumu: IfcOpenShell K02'yi ve
başvurularını çıkarır, şema 0 bulgu, EK-9/8 kapıyı kaynak kimliğiyle bulur. İkinci okuyucu (IfcOpenShell) dört dosyada formun
21 satırını, bulgularını ve nesnelerini aynı verir. Bağımsız IDS denetleyicisi (IfcTester 0.9.0) IDS'i şemasıyla açar, dört
dosyada eksikleri KentOS'un kuralıyla satır ve nesne düzeyinde aynı bulur; ilk koşuda bulduğu yazıcı hatası (arazi ve tapu
kimliği `IfcLabel` yazılıyordu) düzeltildi. Bulgu IFC modelinde boyanır. 100 dairede taban (§6 ölçümü). Kararlar K48–K51.
**Yapıldı (18 Kasım 2026, [ADR 8114](../docs/adr/8114-ek9-table-9-2-other-disciplines.md), K216–K218):** EK-9 Tablo 9.2:
öbür disiplinlerin (statik, mekanik, elektrik, iç mimari …) modeli dosyanın adının EK-2 disiplin alanından tanınır, 16
soru 9.1'deki aynı soruların kanıtıyla, onarımlar programdan bağımsız; paket incelemesi modelin tablosunu verir, başka
tablonun formunu söyler. **Yapıldı (18 Kasım 2026, [ADR 8115](../docs/adr/8115-model-mapping-off-the-window-thread.md),
K219–K221):** model eşlemesi Denetim'de ve kalite formunda okuma kopyasından kuralın iş parçacığında; emsal ölçümü mahali
yalnız komşularıyla keser: 100 dairede 3,1 s → 216 ms; bütçe. **Yapıldı (18 Kasım 2026,
[ADR 8116](../docs/adr/8116-ek9-from-the-delivered-model.md), K222–K227):** Denetim EK-9 formunu son teslim edilen modelden
okuyabilir: teslim kaydının son modelli paketi klasöründen, teslim edildiği gibi (liste, model, form kaydı ve taşınan çizim
özetleriyle); tablonun üstündeki satır son teslimi ve çizimin ondan farkını (GlobalId'yle eklenen, silinen, değişen nesne)
söyler, "Teslim edileni denetle" ile "Şimdiki çizimi denetle" arasında geçilir; 18. soru teslim edilen dosyadan cevaplanır;
kaynak listesi taşınan çizimden, yoksa aynı modelde şimdiki çizimden, yoksa verilmez (üç soru incelemeye). **Kalan:** bulut
işi; disiplin modelinin formunun yazımı; çizimdeki formun ve Ruhsat takımının teslim edilen modelden okunması. (Gerçek kuzey ADR 8076 K58 ile yazılıyor; beş sonuç ADR 8075'ten beri ruhsat
paketinin `results`'ında.)

**Kullanıcı sonucu:** Model kalite kontrol formunun her cevabı diske yazılmış dosyadan gelir; her "hayır" nesnesini seçer.
Şeması temiz bir dosyanın Türkiye profilinden kaldığı görülebilir.

- EK-9'un cevapları ve Türkiye profili Python betiğidir (bim-plan K10). `kentos-ifc` yazılmış dosyanın olgularını
  deterministik verir: nesne, sınıf, ad, ön tanımlı tip, setler, malzeme, mekânsal yapı. Betik kataloğu ve kararları
  veri olarak okur. Bugünkü `kentos_ifc::quality` ve `digital_audit.rs` betiğe taşınır. Beş sonuç (§1) ayrı raporlanır.
- Kaynak modelden beklenen nesne listesi (kimlik ve IFC sınıfı; bölünen ya da birleşen temsillerde eşlemesi): IFC-01'in
  başlangıç ölçümü sınıf ve satır düzeyinde yapıldı (ADR 8062 §10), nesne düzeyindeki beklenti bu pakette kurulur.
- §9'un tablosundaki 21 kanıt; formun resmî cevabı ile kanıt durumu ayrı.
- Katalogdan IDS üretimi; üretilen IDS bağımsız bir denetleyicide açılır. IFC-01’de kaynak modelden çıkarılan nesne
  listesi, okunan dosyanın kimlik ve sınıflarıyla karşılaştırılır. Kalan nesnelerin IDS sonucu olumlu olsa da
  tamamen eksik bir nesne EK-9/8 bulgusudur. [IDS kapsam ve kardinalite](https://github.com/buildingSMART/IDS/blob/development/Documentation/UserManual/specifications.md)
- Denetim sekmesi (ADR 8057 §17) aynı betiği son yazılan dosyada ya da geçici bir dışa aktarmada koşar; bulut işi de
  aynı betiği çalıştırır.
- Bağımsız okuyucular başvuru düzeneğindedir; ürünle dağıtılmaz. Ürünün okuma motoru (IFC-07) bağımsız başvuru sayılmaz.
- Bulgular görüntüleyicide (IFC-12) nesnesini boyar ve seçer.

**İlk kabul:** 21 sorunun her biri için geçen ve kalan kanıt örneği. Üç kapıdan biri ve ilgili başvurular IFC’den
çıkarılır. Geçerli şemaya ve kalan kapıların olumlu IDS sonucuna rağmen EK-9/8 eksik kapıyı kaynak kimliğiyle bulur. Bulgu
planda ve 3B'de nesneyi seçer; ikinci bağımsız denetleyiciyle ayrışan satır kalmaz.

#### IFC-09 — Altlık model: projede saklama ve gösterme [~]

**Kullanıcı sonucu:** Başka bir yazılımın IFC 4.3 modeli KentOS projesinde salt okunur altlık olur: planda katında
kesilmiş, 3B'de gövdesiyle, projenin koordinatında; tıklanan nesnenin sınıfı, adı ve özellikleri görünür.

- Okuma, gövdeler, yapı ağacı ve özellik paneli görüntüleyicinindir (IFC-12); bu paket onları projeye bağlar
  (ADR 8061 §4'ün "altlık olarak ekle" isteği).
- **Yerleştirme:** dosyanın koordinat sistemi ve harita dönüşümü projeninkiyle karşılaştırılır. Uyuşmazlık bulgudur;
  sessiz dönüşüm yapılmaz (kılavuz §5).
- **Planda gösterim:** katın kesme yüksekliğinde gövdelerin kesiti ve altındaki izdüşümü.
- Gösterilemeyen gösterim türü listelenir; nesne sınır kutusuyla ve işaretiyle durur.
- Dosya olduğu gibi saklanır (özeti, şeması, disiplini, revizyonu); yabancı özellik seti ve sınıflandırma kaybolmaz.
- `GlobalId` haritası; yeni revizyon gelince fark (IFC-10).
- Onayla düzenlenebilir KentOS nesnesine dönüştürme bu paketin dışındadır; ayrı ADR.

**İlk kabul:** KentOS'un kendi dosyası geri alındığında sayılar ve sınır kutuları aynı, gövdeler çizimin üstüne oturur;
açık bir mimari IFC 4.3 örneği, gösterilen nesne oranı ve gösterilemeyenlerin listesiyle açılır; koordinat sistemi farklı
bir dosya yerleştirilmeden bulgu verir; süre ve bellek ölçülür.

**Yapıldı (17 Kasım 2026, [ADR 8101](../docs/adr/8101-ifc-underlay-model.md), K157–K162):** altlık kaydı çizimde (Mimari
verisi 18; dosya yolu, SHA-256'sı, şeması, disiplini, revizyonu, yerleşimi), model kopyalanmaz. Yerleştirme dosyanın harita
dönüşümüyle projenin sistemine (`kentos_ifc::underlay::place`); başka sistem, dönüşümsüz dosya, sistemsiz proje, farklı
datum ve birimin açıklamadığı ölçek nedeniyle reddedilir, başlangıç noktasına yalnız istenince. Plan etkin katın kesme
yüksekliğinde kesit ve katın tabanına kadar kırılan kenarlar, çizimin altında soluk; 3B görünümde gövdeler; planda ve 3B'de
tıklanan nesnenin ayrıntıları; değişen dosya söylenir. `cad.underlay.edit` (Python, MCP, 32 ortak durum); Mimari › Proje ▾ ›
Altlık modeller. İlk kabulün dört adımı masaüstünde elle sınandı (kendi modeli 16/16 gövde, kutu 10 × 5 × 7,60 m; IFC 4.3 ve
IFC2X3 örnekleri 14/14; süre ve bellek ölçüldü). **Kalan:** gösterilemeyen nesnenin planda ve 3B'de işareti ve kutusu;
bulut projesinde dosyanın proje varlığı olarak saklanması; büyük modelde planın arka planda hesaplanması.

#### IFC-10 — Federasyon, karşılaştırma ve sorun değişimi [~]

**Kullanıcı sonucu:** İki revizyon ya da iki disiplin karşılaştırılır; fark ve çakışma nesnesiyle listelenir; sorun, açık bir
biçimde karşı tarafa gider ve aynı nesneyi gösterir.

- Hizalama denetimi: sistem, dönüşüm, ±0.00, aks, kat; projenin ortak alanları; çakışan `GlobalId`.
- Revizyon farkı: eklenen, silinen, taşınan, özelliği değişen; iki STEP metninin dizilişi değil, kimlik, sınıf, yer, özellik
  ve miktar karşılaştırılır.
- Sorun değişimi: BCF (bakış, nesne kimlikleri, durum, yorum); içe ve dışa.

**İlk kabul:** bölüm binasının iki revizyonunda taşınan bir duvar ve değişen bir yangın sınıfı raporda; yazılan sorun
bağımsız bir görüntüleyicide aynı nesneyi gösterir.

**Durum (17 Kasım 2026):** hizalama denetimi yapıldı ([ADR 8102](../docs/adr/8102-underlay-alignment-and-clash.md),
BIM-17): sistem, dönüşüm ve datum geometriden önce; katlar, akslar, aynı yerde duran ve çakışan elemanlar. Revizyon farkı
ve BCF yapıldı ([ADR 8103](../docs/adr/8103-ifc-revisions-and-bcf.md)): `cad.ifc.compare` (yalnız GlobalId; sınıf,
öznitelik, kat, tip, gövde: taşınan ve boyu değişen, setlerin değerleri), `cad.bcf.write` (BCF 3.0, nesneleri seçen ve onlara
bakan bakış), `cad.bcf.read` (3.0 ve 2.1; nesneler modelde ve çizimde); 28 + 29 + 20 bağımsız ortak durum; IFC
revizyonlarını karşılaştır ve BCF sorunları pencereleri. **İlk kabul sağlandı:** kesit binasının iki revizyonunda taşınan iç
duvar ve doğu duvarının REI 60 → REI 90'ı raporun tamamı; yazılan sorunu IfcOpenShell'in BCF kitaplığı okur, bakışı aynı
duvarı seçer ve kamerası ona bakar. **Kalan:** bir masaüstü BCF görüntüleyicisinde elle açılış, çakışmanın BCF'si,
sorunların inceleme notu olarak saklanması, bakışın görüntüsü, kutu içindeki biçim değişikliği. **Kimlik (18 Kasım 2026,
[ADR 8117](../docs/adr/8117-persistent-global-ids.md)):** farklı günlerde yazılmış iki teslim artık doğru karşılaştırılır;
türetilen parçaların GlobalId'si kalıcıdır (önceden katlar, açıklıklar ve ilişkiler eklenmiş ve silinmiş görünürdü).

#### IFC-11 — Pilot, sürümler ve kurum kabulü [ ]

**Kullanıcı sonucu:** Bir konut ve bir karma kullanım projesinin paketi idarenin yazılı cevabını alır; KentOS eski sürüm
dosyaları kaybını söyleyerek okur. [BIM-16](bim-plan.md#5-iş-paketleri) ile ortak pilot.

- Türkiye genelindeki kapsam grupları için aynı çekirdek ve farklı idare profilleri sınanır. IFC teknik ön kontrolü
  yasal geçiş tarihinden önce pilotlanabilir. Resmî e-PYS aktarımı yayımlanan uygulama kararına bağlıdır.
- Paket raporu: KentOS commit'i; şema, katalog ve kurum profili sürümleri; kullanılan okuyucuların sürümleri; dosya özetleri;
  inceleme kayıtları.
- IFC4 ve IFC2x3 okuma: kaldırılan sınıflar, harf kodlaması, birim ve gövde farkları; kayıp listesi.
- Sürüm takibi: TS EN ISO 16739-1'in yeni baskısı, Bakanlığın kılavuzu, şemanın düzeltmeleri.
- buildingSMART yazılım sertifikasına başvuru kararı (ADR).

**İlk kabul:** iki pilot paketi en az iki bağımsız model okuyucusundan ve bağımsız PDF/A doğrulayıcısından geçer; idarenin
cevabındaki her madde kataloğa ya da bir pakete bağlanır.

#### IFC-12 — IFC görüntüleyici [~]

**Kullanıcı sonucu:** Bir IFC dosyası KentOS'un içinde, kendi penceresinde açılır: model döndürülür, yapı ağacından kat ve
nesne seçilir, tıklanan nesnenin özellikleri okunur, kesit düzlemiyle içine bakılır. Mimar dışa aktardığı dosyayı tek
tıkla görür. Karar ve kurallar [ADR 8061](../docs/adr/8061-ifc-viewer-and-reading-engine.md); sıra B12b; IFC-07'ye bağlıdır.

- **Gevşek bağ** (ADR 8061 §3): saf çekirdek (`kentos-model-view`; yerel ve wasm32), Iced bileşeni kendi paketinde,
  uygulamada tek bağdaştırıcı. Uygulamayla yalnız bağlantı yüzünden konuşur (ADR 8061 §4). Bağımlılık yönü
  `scripts/arch/deps.mjs`'e yazılır.
- **Sürekli iki sınama:** çekirdeğin ve `kentos-ifc`'in wasm32 derlemesi; bileşeni uygulama olmadan açan tek pencere
  örneği ve resmi. Ayrı program ve web bu paketin dışındadır (ADR 8061 §6); bu iki sınama o yolları açık tutar.
- **Gövde ağları** (`kentos-ifc`, `geometry` özelliği): nesne başına float64 köşeler, üçgenler ve renk; gösterilemeyen
  gösterim türü sebebiyle listede, nesne sınır kutusuyla.
- **Pencere** (ADR 8061 §5): yapı ve sınıf ağacı, sahne, özellikler; gizle, yalnız bırak, hepsini göster; mahal ve açıklık
  ayrı açılıp kapanır; kesit düzlemi ve kesit kutusu (çizicide kırpma yeni iştir); hazır bakışlar; ilerleme ve iptal.
- **Mimari ve Yol ile bağ:** IFC dışa aktarımının sonucunda "Görüntüle"; görüntüleyicide seçilen nesne, dosyayı KentOS
  yazdıysa, planda seçilir.
- **Komutlar:** `ifc.view` (görünüm) ve salt okunur `ifc.inspect` (katalog, ≥ 20 bağımsız durum, Python, MCP). Şeritteki
  yer resimle sahibe gönderilir; yeni tuşlar kısayol listesine girer.
- **Ortak parçalar:** binanın 3B görünümü ([BIM-06](bim-plan.md#5-iş-paketleri)) aynı çekirdeği ve pencere parçalarını
  kullanır; hangisi önce yapılırsa parçaları o kurar. **BIM-06 kurdu (16 Kasım 2026, ADR 8067):** `kentos_ifc::mesh`
  (binadan nesne başına f64 köşe ve üçgen), 3B penceresinin `Source` girdisi (parça, ad, nesne kimliği), kat süzgeci,
  mahalleri aç ve kapa, tıkla ve planda seç, bulgunun nesnesini boyamak. Görüntüleyici okunan dosyayı aynı girdiye
  dolduracak; kesit kutusu ikisinin ortak yeni işi.
- **Eski sürümler:** motorun okuduğu IFC4 ve IFC2x3 dosyaları şema adı gösterilerek açılır; kayıp listesi ve denetimi
  IFC-11'dedir.
- Üçgen ağdan ölçü, alan ya da hacim gösterilmez; miktarlar dosyadaki setlerden okunur (kılavuz §23.3).

**İlk kabul:** bölüm binasının dosyası açılır: gövde sayısı dışa aktarılanla eşit, ağaçta katlar, tıklanan duvarın
özellikleri dosyadakiyle aynı. Dışa aktarımdan tek tıkla açılır ve seçilen duvar planda seçilir. Açık bir IFC 4.3 örneği
gösterilemeyenlerin listesiyle açılır; bozuk dosya açık iletiyle reddedilir; iptal edilen okuma pencereye yazmaz; pencere
kapanınca bellek ve GPU kaynağı bırakılır. wasm32 derlemesi ve tek pencere örneği yeşil. Gerçek pencerede, gerçek fare ve
klavyeyle kabul; resimler.

**Durum (16 Kasım 2026, [ADR 8073](../docs/adr/8073-ifc-viewer-first-slice.md)):**

- Yapılanlar:
  - Paketler ADR 8061 §3.2'deki gibi: çekirdek `kentos-model-view` (yerel ve wasm32), bileşen `kentos-model-view-ui` ve
    tek pencere örneği, 3B çizici kendi paketinde (`kentos-render-scene3d`), uygulamada tek bağdaştırıcı (`ifc_viewer.rs`).
    Binanın ve yolun 3B görünümü aynı sahneyi, kamerayı ve görünüşü kullanıyor. `deps.mjs`'e iki grup yazıldı.
  - Pencere: yapı ve sınıf ağacı, gösterilemeyenler sekmesi, sahne, özellikler (öznitelikler, tip, kat, malzeme, setler
    dosyadaki gibi); gizle, yalnız bırak, hepsini göster, ağaçta göz; mahaller ve açıklıklar ayrı; saydam; yatay ve iki
    düşey kesit düzlemi, seçimin çevresinde kutu (çizicide kırpma, seçme de uyuyor); dört bakış; okuma evresi ve İptal.
  - Kendi işletim sistemi penceresinde açılır; Mimari › Proje › IFC modeli ve Dosya › İçe aktar › Bina modeli; dışa
    aktarımın bildiriminde Görüntüle; KentOS'un yazdığı dosyada seçim planla birlikte.
  - `cad.ifc.inspect`: katalog, 37 bağımsız durum (IfcOpenShell'den), Python, MCP (salt okunur).
  - İlk kabulün maddeleri ADR 8073 §8'de: gerçek pencerede kabul dışında hepsi sağlandı; wasm32 derlemesi ve tek pencere
    örneği yeşil.
- **Yerleşim (18 Kasım 2026, [ADR 8118](../docs/adr/8118-ifc-viewer-layout-refinement.md), K236–K241):** kesitin kartı
  sahnenin sol üstünde (şerit kesitle uzamaz); ağacın üstünde arama (ad, sınıf, GlobalId; Türkçe i katlaması; bulunanlar
  katıyla); yan bölmeler sürüklenir; ölçüler projenin basamaklarıyla, dosyadaki değer ipucunda; anahtarlar simgeli.
- **Binanın 3B görünümünde (18 Kasım 2026, [ADR 8119](../docs/adr/8119-building-3d-in-the-model-viewer.md), K242–K251):**
  mimari çizimin 3B görünümü görüntüleyicinin kendisidir; girdisi dışa aktarımın bellekte yazılıp okunan modeli, altlık
  modeller yanında (`Input::append`, menekşe). Yeniden okunan modelde görünüm anahtarlarla kalır (`Viewer::reload`).
  Başvurulan emsal zonu katının altında (`read::view`); sahne boyutunu bildirir, sığdırma dar sahnede kırpmaz.
- **Çizim kalitesi (18 Kasım 2026, [ADR 8120](../docs/adr/8120-3d-render-quality.md), K252–K257):** Uygulama
  ayarları › Çizim motoru › 3B görünüm kalitesi (Hızlı, Dengeli, Kaliteli; bu cihaz, varsayılan Kaliteli): 4× kenar
  yumuşatma, gövdelerin kenar çizgileri (25° kıvrım), ortam gölgesi (SAO, camın altında kalkar); binada gök ve dolgu
  ışığı. Sahne kendi hedefine çizilip bindirilir; gerçek GPU denetimi üç düzeyi ölçer.
- **Ölçme ve katlanan bölmeler (18 Kasım 2026, [ADR 8121](../docs/adr/8121-viewer-measuring-and-folding-panes.md),
  K258–K265):** şeritte Ölç; Uzaklık (eksen bileşenleriyle), Açı, Alan (düz yüzün bütünü); köşe, kenar ortası, kenar,
  yüz yakalaması (f64 kaynak köşeleri); sahnenin sağ altında katlanan kart; yan bölmeler kenar şeridine katlanır.
  Çekirdek `kentos_model_view::measure` (saf, wasm32).
- Kalanlar (ADR 8073 §9, 8118 §6): gerçek pencerede kabul; kesit düzlemini yüze tıklayarak kurmak ve sürüklemek, kutunun
  tutamakları, kesilen yüzün kapağı; gösterilemeyen nesnenin kutusu; ilk gövdelerle sahne; seçimde yeniden kurmadan boyama;
  denetim bulgusundan “Modelde göster” (B13 ile); bir değerden “benzerini göster”; bölme genişliklerinin saklanması.

## 11. Örnekler ve regresyon

| Örnek | Başarı ölçütü |
|---|---|
| Bölüm binası (`section_building.py`), 2+1 daire, `arch_ifc_cases.py` | Bağımsız beklenen sınıf, ilişki, ölçü, kat, proje kodu ve özellikler; iki bağımsız okuyucu |
| Rampalı, giydirme cepheli, asansörlü bina; eğri cephe; kırma çatı altında mahal; farklı kot | Kayıp listesi boş; hacim ve alan bağımsız başvuruya eşit; gövdeler kapalı |
| Karma kullanım, giriş kotları farklı iki blok, ortak bodrum | Pilot profilde bina başına dosya; ortak saha röperi ve ayrı blok kotları; alan tekrarsız; kimlikler tek |
| Kapsam içinde ve dışında kalan binalar | Kapsam kararı doğru madde ve tarihle (BIM-02) |
| Bilerek bozuk dosyalar: yanlış şema, eksik yangın alanı, bozuk kimlik, farklı datum, farklı revizyonlu pafta | Bulgu doğru katmanda ve nesnesinde; şeması temiz dosya profilden kalabiliyor |
| Yeniden dışa aktarma, yeniden adlandırma, sil ve geri al, kopya, tip değişikliği | Kimlik korunur ya da yenisi üretilir; fark öngörülebilir; zaman damgası sabitlenmiş karşılaştırma |
| Yolun IFC'si (`road_ifc_cases.py`) | `IfcAlignment` şeması, yerler ve kotlar; ortak koordinat ve STEP değişikliği yolu bozmaz |
| Komut ve masaüstü akışı | Menü, komut, Python ve MCP aynı sonucu verir; geçersiz yol, üzerine yazma, iptal, karşılanmayan profil; gerçek pencerede kabul |
| Görüntüleyici: kendi dosyalarımız, başka yazılımların açık örnekleri, bozuk ve çok büyük dosyalar | Sayılar yazılanla aynı; gösterilemeyenler listede; panik ve asılma yok; iptal temiz; wasm32 derlemesi ve tek pencere örneği yeşil (IFC-07, IFC-12) |

**Araştırma sınırı.** İki yönetmeliğin ana metni tam okundu. Ekler ilk turda indirilmemişti; 15 Kasım 2026'da OCR ile
okunup [mevzuat/bim/](../mevzuat/bim/README.md) altına yazıldı. EK-8 (nesne tablosu yok, LOD 300), EK-5 5.5.4'ün mahal
kuralı ve EK-7'nin düşey datum notu sayfadan okundu (araştırma notu §4.1). TSE metinleri ve e-PYS'nin uygulama kuralları
hâlâ okunmadı; EK-6 ve EK-7'nin uzun açıklamalarına dayanan kurallar üretim iddiasından önce PDF sayfasıyla
karşılaştırılır (B1). ADR'lerdeki test sonuçları geçmiş kanıttır; bu plan çalışmasında ürün testleri
yeniden çalıştırılmadı. Görüntüleyicinin ve okuma motorunun kaynakları
[ayrı nottadır](../docs/research/2026-11-15-ifc-goruntuleyici.md): IFClite'ın paketleri derlenmedi ve ölçülmedi; benzer
görüntüleyiciler belgelerinden okundu, çalıştırılmadı.
