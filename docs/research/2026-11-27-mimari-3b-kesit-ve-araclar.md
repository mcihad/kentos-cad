# Mimari: 3B görünüm, kesit ve eksik araçlar — araştırma raporu

27 Kasım 2026. Dal `mimari-bim` (main `82b6c8ef1`'den). Plan: [plan/mimari-bim.md](../../plan/mimari-bim.md). Karar:
[ADR 8171](../adr/8171-mimari-3d-section-quality-and-missing-tools.md).

## 1. İstek

Sahibin sözleri (27 Kasım 2026):

- "3d bina çizimlerde ciddi problemler var onları düzeltelim mesela kolon kirişler düzgün render edilmiyorlar. Kesit
  esnasında ciddi bozulmalar oluyor, ayrıca mimari proje çizimlerinde birçok eksik aracımız var çizimle ilgili çok
  ciddi sıkıntılarımız var piyasadaki özellikle mimari çizim ve BİM araçlarını incele iyi bir rapor …"
- İş sürerken: "Kesit değiştirirken ve döndürürken üçgenlerde ve mesh yapılarda titremeler ve kayıplar oluyor ayrıca"
  (ileti "ayrıca"da bitiyor; devamı gelince plana eklenecek).

## 2. Nasıl bakıldı

- **Çizim:** bölüm binası (`fixtures/arch/v1/section-building.kcad`: 16 duvar, 16 kolon, 8 kiriş, 3 döşeme, merdiven).
  Yanına komutlarla 2 × 2 açıklıklı bir çerçeve eklendi:
  - iki katta dikdörtgen, 90° dönük, yuvarlak (Ø50) ve L kolonlar;
  - kolon ortasından kolon ortasına 25/50 kirişler;
  - 1. katta keskin dönen (≈16°) üç noktalı bir kiriş;
  - zemin katın yarısının üstünde döşeme, öbür yarısında ve 1. katın üstünde döşeme yok.
- **Ekran dışı resim** (`KENTOS_SNAPSHOT_BACKEND=wgpu`): 3B görünümün birkaç yönü, yatay ve iki düşey kesit.
- **Gerçek pencere** (Xvfb, sahibin ayarlarının kopyası): 3B pencere fareyle döndürüldü ve kesit kaydırıcısı
  sürüklendi. Hareket sırasında değişen her kare kaydedildi. Bunun için gerçek pencere sürücüsüne `down`, `up` ve
  `drift` eklendi (`scripts/usage/real/driver.py`).
- Resimler: `.run/gui-evidence/mimari-bim/once/`.

## 3. Bulgular: 3B görünüm ve kesit

3B görünüm, çizimin bellekte yazılıp okunan IFC'sini gösterir (ADR 8119). Gövdeleri kesit modeli verir
(`crates/native/application/src/arch/sections.rs`). Görüntüleyici gövdeleri parçalara (döşemelere, “tile”) böler ve
ekran kartına yükler (`crates/model-view-ui`, `crates/render/scene3d`).

### B1. Kolon, kiriş ve döşeme aynı hacmi paylaşıyor

- **Kural bugün** (ADR 8041 §1, `sections.rs` `column`, `beam`):
  - kolonun tepesi üstündeki döşemenin betonarme üstüdür;
  - kirişin tepesi de oradadır (TBDY 2018 7.4.1.1: kiriş yüksekliği döşemeyi içine alır);
  - kiriş kolonun ortasından ortasına uzanır;
  - döşeme bunların üstünde kesintisizdir.
- **Sonuç:** aynı beton iki ya da üç gövdede yer alıyor. Aynı kotta üst üste binen yüzler, derinlik tamponunda hangisinin
  önde olduğuna her karede başka karar veriyor ("z-fighting"). Döndürürken titreşen yerler:
  - döşemesiz katta kolon başı ile kiriş üstü (birleşimlerde açık renkli kareler, `birlesim-titremesi.png`);
  - cephede döşemenin alnı ile kirişin dış yüzü;
  - iki kirişin kesiştiği yer.
- **IFC'de:** kolonun, kirişin ve döşemenin `GrossVolume`'u kendi prizmasıdır (`kentos_ifc::quantities::column`,
  `beam`). Birleşimlerdeki beton iki ya da üç kez sayılır.

### B2. Kesit sürüklenirken kesit yüzü kalkıyor

- Bu bilinçli bir karar (ADR 8131 K310): kaydırıcı sürüklenirken kapak hesaplanmıyor, bırakınca geri geliyor.
- Sürükleme boyunca kesilen her duvar, kolon, kiriş ve döşeme içi boş bir kabuk gibi görünüyor. Kesitin önünden
  gövdelerin iç yüzleri ve onların üçgen çizgileri (X biçimli köşegenler) görünüyor (`kesit-surukleme-kareleri.png`).
- Sahibin "kesit değiştirirken kayıplar" dediği bu.

### B3. Kesit yüzünün halkası küçük bir aralıkta kopuyor

- Kapak, düzlemin her gövdeyi kestiği parçaların halkaya zincirlenmesiyle kurulur. Zincirde iki nokta 1e-7 m'den
  yakınsa birdir (`caps.rs` `JOIN`).
- IFC'den dönen gövdeler float32'dir. Aynı dosyanın `MEET` notu, birleşmesi gereken köşelerin yüzde birkaç milimetre
  ayrık olabildiğini söylüyor.
- Halka kapanmazsa o gövdenin kapağı hiç çizilmez ("açık kabuğun kapağı yok"). Gövde kesitte içi boş kalır.

### B4. Kenar çizgisi köşeleri bit eşitliğiyle birleştiriyor

- Kenar çizgileri köşeleri yalnız float32 bitleri birebir aynıysa birleştiriyordu (`crates/render/scene3d/src/edges.rs`).
  Bir yüzün iki üçgeninin köşeleri son bitte ayrıksa köşegen iki ayrı "açık kenar" sayılıp çizilir.
- **Düzeltme (MB-03'te ölçüldü):** gerçek pencerede düz çatıda görülen noktalı köşegen (`cati-kosegeni.png`) bu değil.
  Kenar çizgisi ve ortam gölgesi olmayan Hızlı kalitede de görünüyor. Ekran kartıyla çekilen ekran dışı resimde ise
  yok. Gerçek pencere sınaması Xvfb'de yazılımsal Vulkan'la (lavapipe) çiziyor; çizgi onun izi. Kenar kaynağı yine
  toleranslı yapıldı, çünkü IFC'den dönen gövdelerde ayrık köşe olabiliyor (`caps.rs`'in `MEET` notu).

### B5. Her değişiklikte bütün sahne yeniden yükleniyor

- Şunlardan biri olunca bütün sahne yeni parça kimlikleriyle baştan kuruluyor (`Scene::of`, `NEXT_CONTENT`):
  - seçim;
  - kesitin başlaması ve bırakılması;
  - gizle ya da göster;
  - Denetim bulgusu;
  - çizimdeki her düzenleme.
- Bunun bedeli:
  - Bütün parçalar ekran kartına yeniden yükleniyor ve kenar çizgileri yeniden hesaplanıyor.
  - Eski parçalar ekran kartında kalıyor; yalnız 1 GiB'lik bütçe dolunca bırakılıyor.
  - Bir karede en çok 64 MiB yükleniyor. Daha büyük modelde kalan parçaların yeri bir sonraki kareye dek boş kalıyor:
    seçince ya da kesiti bırakınca modelin parçaları bir an kayboluyor.

### B6. Keskin dönen kirişin ucu (tanı düzeltildi)

İlk bakışta üç noktalı kirişin 16°'lik dönüşünde iki kolu 3B'de birleşmeden bitiyor gibi görünmüştü
(`keskin-kiris.png`). MB-05'te ölçüldü:
- kirişin 3B gövdesi eksiksiz: üst yüzü 3,69 m², doğuda gönye ucuna dek (x 29,88) uzanıyor;
- tepeden yakın çekimde iki kol doğuda sivri gönyeyle birleşiyor.

Resimde ayrık görünen uçlar kirişin batı uçlarıydı; onlar tasarım gereği ayrık. Kusur yok.

### Ekran dışı resimde neden görünmedi

Sınama düzeneği her resimden önce iki kare bekliyor ve tek bir kareyi kaydediyor. Derinlik yarışı da kare kare değişen
bir görüntü verir. Bu yüzden B1, B4 ve B5 ancak gerçek pencerede, hareket sırasında görüldü.

## 4. Piyasa: birleşim ve kesit

| Uygulama | Birleşim kuralı | Kaynak |
|---|---|---|
| Revit | Betonarme birleşimde kiriş–kolon: kolon baskın. Kiriş–kiriş: önce çizilen baskın. "Switch Join Order" sırayı çevirir. Taşıyıcı elemanlar duvar, döşeme ve çatıyı keser. | [Dominant Elements in a Concrete Join](https://help.autodesk.com/cloudhelp/2016/ENU/Revit-Model/files/GUID-C158F662-0A35-48A8-9B1E-F502DC060C3B.htm), [Revit 2025 Beam/Column Join](https://ukcommunity.arkance.world/hc/en-us/articles/25722374187922-Revit-2025-Structural-Framing-The-Beam-Column-Join-Tool) |
| Archicad | Yapı malzemesinin kesişim önceliği. Yüksek öncelikli eleman düşüğünü keser; duvar, kiriş, kolon, döşeme, çatı, kabuk ve "morph" için geçerli. Elemanlar gerçekten çakışmalı. Katman kesişim grupları farklı olan elemanlar hiç kesişmez. | [Basic Intersection Principles (AC 29)](https://help.graphisoft.com/AC/29/INT/_AC29_Help/040_ElementsVB/040_ElementsVB-271.htm) |
| Archicad forumu | Kiriş ve kolonun önceliği döşemeninkinden yüksek ve tepeleri aynıysa kiriş döşemeyi deler. Çözüm döşemenin önceliğini artırmak. | [Intersecting horizontal and vertical elements in 3D](https://community.graphisoft.com/t5/Modeling/Intersecting-horizontal-and-vertical-elements-in-3D/td-p/293214) |
| xeokit, three.js (ADR 8131) | Kesit kapağı gövdenin kendi geometrisinden kurulur. Yalnız kapalı gövde kapak alır. Birleşik gövdelerde kayma sorunları bildiriliyor. | ADR 8131 §2 |

**Sonuç:** Hiçbir uygulama iki elemanın aynı hacmi tutmasına izin vermiyor. Birinin baskın olduğu bir birleşim kuralı
gövdeleri ayırıyor. Kesitte aynı malzemeden gelen parçalar tek dolgudur. KentOS'ta bu, 2B kesitte zaten böyle
(ADR 8041 §2: perde ile kolon kesintisiz). 3B gövdelerde ve IFC'de böyle değil.

## 5. Piyasa: araç takımları ve KentOS

Kaynaklar: üreticilerin 2026 sürüm notları ve yardım sayfaları:

- [Revit 2026](https://www.autodesk.com/blogs/aec/2025/04/02/whats-new-in-revit-2026/), Revit duvar
  [silmesi](https://help.autodesk.com/cloudhelp/2023/ENU/Revit-ArchDesign/files/GUID-02B3706B-3638-4377-AE67-4EEBF1958004.htm)
  ve [oyuğu](https://help.autodesk.com/cloudhelp/2023/ENU/Revit-ArchDesign/files/GUID-D843AE7A-457D-4860-9EAE-19FAF171723C.htm);
- [Archicad 29](https://help.graphisoft.com/AC/29/INT/_AC29_Help/005_NewFeatures/005_NewFeatures-1.htm);
- [Vectorworks Architect 2026](https://www.vectorworks.net/newsroom/architecture-features-architect-2026);
- [Allplan 2026](https://www.allplan.com/system/releasenotes/2026/allplan-2026/);
- [BricsCAD V26](https://www.bricsys.com/news/octave-bricscad-v26-2-release-introduces-30-new-and-enhanced-features);
- [ideCAD mimari komut listesi](https://help.idecad.com/ideCAD/list-of-idecad-architectural-commands).

"Var" yalnız KentOS'un şeridinde ya da komut kataloğunda bulunanı gösterir (`catalog/v1/ribbon.json`,
`crates/native/interaction/src/arch/`).

| Araç | Piyasada | KentOS |
|---|---|---|
| Duvar, oda, kapı, pencere, boşluk, niş | hepsinde | var |
| Kolon, kiriş, döşeme, kaplama, asma tavan | hepsinde | var |
| Çatı (kırma, beşik, tek eğimli), çatı penceresi, oluk | hepsinde | var |
| Merdiven, döner merdiven, rampa, korkuluk | hepsinde | var |
| Giydirme cephe | hepsinde | var |
| Akslar, kat yönetimi, tadilat durumu | hepsinde | var |
| Kesit, görünüş, iç görünüş, detay işareti | hepsinde | var |
| Çizelge, metraj, alan cetveli, IFC, BCF | hepsinde | var |
| **Temel** (tekil, sürekli, radye) | Revit, Archicad (Allplan "foundation slab"), ideCAD (statikle) | **yok** |
| **Duvar profili** (parapet, harpuşta, söve, denizlik, silme, oyuk) | Revit "Wall Sweep/Reveal", Archicad karmaşık profil | **yok** |
| **Arazi ve kazı-dolgu** (bina oturumu, tesviye) | Revit Toposolid, Archicad Mesh, Vectorworks DTM | Yol'da arazi var, Mimari'de **yok** |
| **Serbest biçimli eleman** (kütle, morph, kabuk) | Revit kütle, Archicad Morph ve Shell | **yok** |
| **3B görünüm paftada** (aksonometri, perspektif, kesit perspektifi) | hepsinde | **yok** (paftada yalnız 2B görünüm) |
| **Mahal renk planı** (kullanım, daire, yangın bölmesi renkleri ve lejandı) | Revit "Color Fill", Archicad zon renkleri | **yok** |
| **Güneş ve gölge** (konum ve tarih) | hepsinde | **yok** |
| **Yürüyen merdiven ve bant** | Revit ve Archicad nesnesi | **yok** (asansör blok olarak var) |
| Kat tekrarı | hepsinde "kopyala ve yapıştır" ya da bağlı kat | oda modülünün kat tekrarı ile var |
| Asma tavan planı | hepsinde | var (`ceiling_plan`) |

## 6. Türkiye mevzuatı

- **Mimarlık ve Mühendislik Projelerinin Dijital Olarak Hazırlanması Hakkında Yönetmelik** (RG 5.8.2026/33331):
  - EK-5 Tablo 5.1'in zorunlu sınıfları arasında `IfcFooting` (Temel, TML, sıra 34) ve `IfcPile` (Kazık, KZK, sıra 44)
    var (`mevzuat/bim/ek-5-ifc-siniflari-varlik-isimlendirme.md`). KentOS ikisini de yazmıyor.
  - Yürüyen merdiven `IfcTransportElement`'tir (TSM, sıra 64); asansör bugün blok olarak yazılıyor.
- **PAİY 57/2-a-6:** "bir tanesi ortak merdivenden geçmek üzere en az iki adet kesit" ve yeteri sayıda görünüş
  (ADR 8042, 8043). Temeli olmayan bir model kesitte temeli de gösteremez.
- **TBDY 2018 7.4.1.1:** kiriş yüksekliği döşeme kalınlığını içine alır (ADR 8041). Bu ölçü nesnenin değeri olarak
  kalır; gövdeler birleşim kuralıyla ayrılır.
- **TS EN ISO 128-3** (ADR 8041 §2): aynı parçanın kesit kısımları aynı taramayla gösterilir. Birlikte dökülen
  betonarme kolon, kiriş ve döşeme kesitte tek parçadır; birleşimden yeni çizgi doğmamalı.
- **Metraj:** Kolonun, kirişin ve döşemenin birleşimdeki hacminin hangisine yazılacağına dair bağlayıcı bir metin
  bulunamadı (Çevre, Şehircilik ve İklim Değişikliği Bakanlığı'nın birim fiyat tarifleri bu turda okunamadı). Karar
  ADR 8171'de: Revit'in kuralı ve TBDY'nin ölçüsü. Toplam beton hacmi her durumda kesindir: iki kez sayılmaz.

## 7. Öneri

Plan [plan/mimari-bim.md](../../plan/mimari-bim.md):

- **Faz A: 3B ve kesit, sahibin bildirdikleri.**
  - MB-01: birleşim kuralı.
  - MB-02: hareket sırasında da kesit yüzü ve kopmayan halka.
  - MB-03: kenar çizgisi.
  - MB-04: değişmeyen parça yeniden yüklenmez.
  - MB-05: keskin köşe (ölçümde kusur çıkmadı).
  - MB-06: saydam yüzler (ölçümde kusur çıkmadı).
- **Faz B: eksik araçlar,** Türkiye'deki ruhsat ve uygulama projesinin gereğine göre sıralı.
  - MB-10: temel.
  - MB-11: duvar profili.
  - MB-12: arazi ve kazı-dolgu.
  - MB-13: 3B görünüm paftada.
  - MB-14: mahal renk planı.
  - MB-15: güneş ve gölge.
  - MB-16: serbest biçimli eleman.
  - MB-17: yürüyen merdiven.
  - MB-18: kazık.
