# ADR 8047: Binanın IFC 4.3'ü — ayrı IFC paketi, EK-5 sınıfları ve adları, EK-6 özellikleri

- **Durum:** 1. ve 2. bölüm yapıldı (11 Kasım 2026); `plan/architecture-advanced.md` ADV-25, M47. Binanın IFC'si
  çekirdekte ve uygulamada hazır, bağımsız okuyucu sınıyor (§1–§5); komutu `cad.arch.ifc` ve Dosya › Dışa aktar › Bina
  modeli › "IFC dışa aktar (bina)…", gerçek pencerede kabul edildi (§6). Sırada: 3. bölüm, EK-7'nin proje ve kişi
  bilgileri, EK-9 kalite formu ve Md. 9 tutarlılık denetimi Denetim'de; başlığın görünüm tanımı (§6).
- **Bağlam belgesi:** ADR 0908 (yolun IFC 4.3 hizalaması), ADR 8042 (kesit modeli), ADR 8040 (düşey model), ADR 8041
  (kiriş, lento, kolon–duvar birleşimi), ADR 0342 (mahal ve net alan), ADR 8030 (İmar durumu), ADR 8032 (Vaziyet planı).

## Bağlam

Plan ADV-25: "2029'dan sonra istenecek IFC modeli aynı çizimden üretilir." İlk kabul: "IFC'de duvar, kapı ve mahal
sayısı çizimle eşit; bağımsız okuyucu sınar." Mimarlık ve Mühendislik Projelerinin Dijital Olarak Hazırlanması Hakkında
Yönetmelik (RG 5.8.2026/33331) IFC teslimini şehrin büyüklüğüne göre 2029–2032 arasında zorunlu kılıyor.

Kullanıcının kuralı (11 Kasım): "IFC işlemleri için ayrı bir crate yap … ileride bunu çok genişleteceğiz"; "IFC ile ilgili
her şey orada birleştirilecek, oradaki kütüphaneler kullanılacak"; "kodlar değiştirilirken ona dokunan her kod oraya
taşınacak, doğrudan oradaki fonksiyonlar, metotlar ve sınıflar kullanılacak." CLAUDE.md'de kesin kural olarak yazılı.

## Araştırma

### Türkiye mevzuatı (yönetmeliğin ekleri, Resmî Gazete'nin taranmış PDF'i, 241 sayfa)

- **EK-5 §5.4:** "IFC sınıfları IFC 4.3 versiyonu esas alınarak" düzenlenmiştir. Plan "IFC4" diyordu; şema
  `IFC4X3_ADD2` (IFC 4.3'ün yayımlanmış hâli).
- **EK-5 §5.5:** nesnenin adı `Disiplin(2)-KategoriKodu(3)-Açıklama`; alanlar "-" ile, alanın içindeki kelimeler "_" ile
  ayrılır. Mimari "MM". **Tablo 5.1** kodları: duvar DVR, döşeme DSM, kolon KLN, kiriş KRS, çatı CAT, korkuluk KRK, merdiven
  MRD, kapı KAP, pencere PNC, mahal MHL, açıklık ACL, bina katı BNK, bina BNA, saha SAH.
- **EK-6** (sınıf başına zorunlu öznitelik ve özellik tabloları): her yapı elemanında ad (EK-5), etiket (Tag), ön tanımlı
  tip ve malzeme ("Malzeme İsmi_Dayanım", ör. Beton_C25); duvar (Tablo 6.66), döşeme (6.56), kapı (6.25), pencere (6.67)
  ve çatıda (6.52) `IsExternal`; hepsinde `FireRating` (Binaların Yangından Korunması Hakkında Yönetmeliğe göre); kolonda
  `LoadBearing` (6.16); kapı ve merdivende (6.61) `FireExit`; mahalde kaplamalar ve net alan (6.58); katta net alan ve
  taşıyıcı döşemenin kotu (6.8); binada `Pset_BuildingCommon` (kimlik, yapım yöntemi, yangın koruma sınıfı, kullanım
  türü, planlanan alanlar, kat sayısı), adres ve `Qto_BuildingBaseQuantities` (yükseklik — "sadece yüksekliğin sabit
  olduğu durumda", saçak yüksekliği, taban alanı, brüt ve net kat alanı) (6.7); sahada emsal ve TAKS (6.55).
- **EK-7:** projenin bilgileri — `IfcProject` (Tablo 7.1), kişiler (7.2: ad, soyad, oda sicil numarası), kuruluş (7.4:
  vergi numarası), koordinat sistemi ve harita dönüşümü (7.6–7.7). "Her bir disiplin modelinin IfcProject sınıfının ortak
  öznitelikleri aynı olmalıdır." 3. bölümün konusu.
- **EK-9:** mimari model kalite kontrol formu; 3. bölümün konusu.

### Bugünkü durum (ölçüldü)

- KentOS'ta yalnız yolun IFC 4.3 hizalaması vardı (ADR 0908, `formats::ifc_alignment`); bina IFC'si yoktu.
- Kesit modeli (ADR 8042) binanın gövdelerini zaten kuruyor: planın ve kesitin birleşimleriyle duvarlar (katman katman,
  kolonun yüzünde ve kirişin altında biten), açıklıklar, döşemeler, kolonlar, kirişler, çatılar, korkuluklar ve
  merdivenler, ±0.00'a göre prizma, kol ve eğik yüz olarak.

## Karar

### 1. Ayrı paket: `crates/shared/ifc` (`kentos-ifc`)

- IFC'ye ait her şey burada: `step` (ISO 10303-21 dosyası: sayı, yazı, GUID'in 22 karakterlik IFC biçimi, varlık
  yazıcısı), `alignment` (yolun hizalaması, `formats`'tan **aynen** taşındı), `building` (IFC bina modeli ve yazıcısı;
  KentOS çizimini bilmez), `arch` (KentOS Mimari'nin binası → IFC binası), `naming` (EK-5 adları).
- Bağımlılıklar: `kentos-contracts`, `kentos-geometry-core`, `kentos-road`, `libm`. Belgeyi ve uygulamayı bilmez:
  uygulama yalnız bağdaştırıcıdır (`application::arch_ifc`, 46 satır: kesit modeli, nesneler kimlikleriyle, duvar
  stilinin adı, mahalin net alanı, projenin mimari ayarları). Yolun IFC'si (`application::road_ifc`) paketin
  hizalamasını doğrudan kullanır.
- **Plan dışı:** dış duvar kuralı (`geom::wall::exterior`, "gövdesinin bir köşesi bütün gövdelerin dış çizgisinde")
  etkileşimdeki plan ölçülerinden geometri çekirdeğine taşındı; plan ölçüleri ve IFC aynı kuralı kullanır.

### 2. Sınıflar ve yapı

- `IfcProject` → `IfcSite` → `IfcBuilding` → `IfcBuildingStorey` (her kat bir; kotu katın kotu, uzun adı katın adı; katı
  olmayan çizimde ±0.00'da tek "Zemin kat"). Yapı elemanları katında durur (`IfcRelContainedInSpatialStructure`), mahal
  katının parçasıdır (`IfcRelAggregates`).
- Duvar → `IfcWall` STANDARD; döşeme → `IfcSlab` FLOOR; kolon → `IfcColumn` COLUMN; kiriş → `IfcBeam` BEAM, lento LINTEL;
  çatı → `IfcRoof`; korkuluk → `IfcRailing` BALUSTRADE; merdiven → `IfcStair`; mahal → `IfcSpace` INTERNAL.
- Açıklık → duvarını oyan `IfcOpeningElement` (niş RECESS) ve onu dolduran `IfcDoor`/`IfcWindow`: etiketi çizimdeki
  numarası ("K01", "P01"), toplam yüksekliği ve genişliği. Kapı ve pencerenin gövdesi, yapısı kasa derinliğini
  veriyorsa kasanın duvar içindeki bandıdır (`HoleFrame`, planın kasayı çizdiği kural: ortada ya da açıldığı yüzden
  "Kasanın yüzden uzaklığı" kadar); vermiyorsa duvarın bütün kalınlığı — plan da öyle çizer.
- Gövdeler kesit modelinden (ADR 8042): duvar katman katman, kolonun yüzünde ve kirişin altında biter — plan, kesit ve
  model aynı birleşimi gösterir. Prizmalar `IfcExtrudedAreaSolid`, kollar ve eğik yüzler `IfcPolygonalFaceSet`.
- **GlobalId nesnenin KentOS kimliğidir** (UUID'in 128 biti, IFC'nin 22 karakteri): aynı çizim yeniden aktarıldığında
  her duvar, kapı ve mahal aynı GlobalId'yi taşır, karşı programda eşleşir.

### 3. Adlar (EK-5 §5.5)

`MM-<kod>-<açıklama>`; açıklamada boşluk "_" olur, "-" kalmaz. Duvar stilinin adıyla (`MM-DVR-Dış_duvar_25`), kapı ve
pencere numarasıyla (`MM-KAP-K01`, `MM-PNC-P01`), mahal numarası ve adıyla (`MM-MHL-001_Yatak_odası`), kat adıyla
(`MM-BNK-Zemin_kat`), açıklık numarasıyla (`MM-ACL-K01`), bina çizimin adıyla, saha `MM-SAH-Parsel`.

### 4. Özellikler (EK-6): çizimin bildikleri

- **`IsExternal`:** duvarda, gövdesinin bir köşesi katın **duvarları ve kolonları birlikte** dış çizgisindeyse. Kolonlar
  çizgiyi kapatır: kesit modeli duvarları kolonların yüzünde durdurur; yalnız duvarlarla çerçeveli katın dış çizgisi
  parça parça kalıyor ve iç duvarın ucu bir parçanın çizgisine değiyordu (ölçüldü: bölüm binasının **8 iç duvarının 8'i
  dış** çıkıyordu; §Doğrulama). Kapı ve pencerede duvarınınki; döşemede balkon ve teras dışarıda, kat döşemesi değil;
  çatıda dışarıda.
- **`LoadBearing`:** kolonda doğru. **Plan dışı:** KentOS'un kolonu çerçevenin kolonudur (malzemesi söylenmemişse
  Betonarme, ADR 8041); mimari dikme ayrımı yok.
- **Malzeme:** duvarın çekirdek katmanınınki (birleşim önceliği en güçlü — 1 çekirdek, söylenmemiş 3, 5 kaplama —,
  eşitlikte genişliğin kalanını alan katman: bölüm binasında Tuğla), döşemenin kendi malzemesi, kolon ve kirişinki
  (söylenmemişse Betonarme). Çizim beton sınıfını bilmediği için "_Dayanım" yazılmaz.
- **Mahal:** zemin, duvar ve tavan kaplaması (`Pset_SpaceCoveringRequirements`, mahal listesindekiler), net alanı
  (`Qto_SpaceBaseQuantities.NetFloorArea`, planın ve mahal listesinin net alanı).
- **Kat:** kat yüksekliği (`GrossHeight`), net alanı (mahallerinin toplamı).
- **Bina:** kat sayısı; yükseklik bütün katlarda aynıysa o; net alan (bütün mahaller).
- **Saha:** İmar durumundan emsal ve TAKS, parsel alanı (`Qto_SiteBaseQuantities.GrossArea`); ±0.00'ın mutlak kotu
  (`RefElevation`, Vaziyet planı).

### 5. Henüz yazılmayanlar

Çizimin bilmediği ya da sonraki bölümün işi olanlar: yangın dayanım sınıfı (`FireRating`), acil çıkış (`FireExit`), bina
kimliği, yapım yöntemi, yangın koruma sınıfı, kullanım türü, planlanan alanlar, adres, saçak yüksekliği, taban alanı,
brüt kat alanı, taşıyıcı döşemenin kotu, katmanlı malzeme takımı (`IfcMaterialLayerSetUsage`), beton sınıfı; EK-7'nin
kişileri ve kuruluşu, koordinat sistemi. 3. bölümde Denetim bunları "eksik zorunlu bilgi" olarak sayacak.

### 6. Komut ve arayüz (2. bölüm)

- **Başka programlar (yalnız davranış):** BricsCAD'in "Export To IFC" penceresi: ayar profili, IFC sürümü (IFC4X3 dahil),
  tür süzgeci, metraj/doğrulama kutuları ve ayrı bir yazar–kuruluş sekmesi; IFC4 RV profilinde "Subtract openings from
  the host geometry" açık. Archicad: Kaydet penceresinde "hangi elemanlar" ve "çevirmen", IFC Save Options'ta
  ayarların özeti; belgesi aktarılan modelin bir IFC görüntüleyicide denetlenmesini öğütler. Revit'in "IFC Export Setup
  Options" sayfası: IFC sürümü, dosya türü, evre, mahal sınırları, özellik setleri, coğrafi başvuru (ekran görüntüsü
  yok, belgeden okundu).
- **Komut `cad.arch.ifc` v1:** girdi yol (mutlak, `.ifc`; baştaki ve sondaki boşluk kırpılır, büyük harfli uzantı olur),
  `stamp` (yoksa şimdi, UTC) ve `overwrite`; çıktı şema, sınıf sınıf sayılar, bayt ve SHA-256; plan ve doğrulama dosyaya
  dokunmaz. Retler sırasıyla `invalid_file_path`, `file_exists`, `invalid_ifc` (okunmayan saat; yazılacak bina yok).
  Uyarı `export_losses`, sebebine göre: "N duvar … IFC'ye yazılmadı: kesit modeli gövdesini kuramadı", "… duvarı
  yazılmadı" (açıklık), "… alanı yok" (mahal), "N rampa IFC'ye yazılmadı: henüz IFC'ye aktarılmıyor"; eğri duvar ve
  merdiven için "düz parçalarla yazıldı". Yazamadığını çekirdek söyler (`kentos_ifc::arch::Left`, `Why`).
- **Plan dışı — komutun adı:** `file.export.ifc.building` önce düşünüldü; yolun `file.export.ifc`'ünün altına bir ad
  açınca Python SDK'da `ifc` hem işlev hem modül olmak zorunda kalıyordu (katalogda başka önek çifti yok). Mimari'nin
  komutları gibi `cad.arch.ifc` oldu; Python'da `kentos.cad.arch.ifc(doc, path=…)`.
- **Arayüz:** Dosya › Dışa aktar menüsünde mimari işte "Bina modeli" bloğu, eylemi `arch.ifcExport` ("IFC dışa aktar
  (bina)…"): kayıt penceresi "IFC 4.3 (.ifc)" süzgeciyle, çizimin adıyla açılır; uzantısız ada `.ifc` eklenir; sonuç
  durum çubuğunda ve Komut geçmişi'nde "IFC 4.3 yazıldı: 2 kat, 16 duvar, 5 kapı, 9 pencere, 10 mahal, 3 döşeme, 16
  kolon, 8 kiriş, 1 merdiven (…)". Çizim değişmez, geri alma adımı yazılmaz.
- **Başlığın görünüm tanımı `DesignTransferView`:** IFC 4.3'te 'Body' gösterimli açıklık, duvarın gövdesinden okuyucunun
  çıkardığı bir Boole farkıdır (IfcRelVoidsElement); Reference View'da ise duvar zaten kesik, açıklık yalnız 'Reference'
  olur. Bizim duvarlarımız kesilmemiş prizmalar olduğundan 1. bölümün `ReferenceView` başlığı yanlıştı. Yönetmeliğin
  eklerinde görünüm tanımı geçmiyor (ekler tarandı). Açıklığı gövdeden önceden kesen bir Reference View çıktısı 3.
  bölümde değerlendirilecek.
- **Bilinen sınır:** dosyanın saati UTC yazılıyor (yolun IFC'si gibi), bölge işareti yok.

## Doğrulama

- **Bağımsız okuyucu** (`scripts/fixtures/arch_ifc_check.py`): IfcOpenShell, KentOS kodu olmadan, bölüm binasının kendi
  tanımına (`section_building.py`) karşı. Şema `IFC4X3_ADD2`; IfcOpenShell'in doğrulayıcısı (öznitelikler ve EXPRESS
  kuralları) **0 bulgu**; sayılar çizimle eşit: **2 kat, 16 duvar, 5 kapı, 9 pencere, 14 açıklık, 10 mahal, 3 döşeme,
  16 kolon, 8 kiriş, 1 merdiven**; bütün adlar EK-5 kalıbında; GlobalId'ler çizimin kimlikleri; her eleman bir katta,
  her açıklık bir duvarı oyuyor, her kapı ve pencere kendi duvarının açıklığını dolduruyor, etiketleri çizimin
  numaraları; `IsExternal` dış duvar stilinde doğru, iç duvarda yanlış, kapı ve pencerede duvarınınki; mahallerin net
  alanı dikdörtgenlerinin alanı, katların mahallerinin toplamı, binanınki hepsi; binanın yüksekliği 3.00; kolonlar
  taşıyıcı, teras döşemesi dışarıda; malzemeler Tuğla ve Betonarme; **68 gövde** IfcOpenShell'de şekil veriyor.
- Okuyucunun ilk koşusu gerçek bir hata buldu: **8 iç duvarın 8'i `IsExternal = doğru`** (§4). Düzeltmeden sonra temiz.
  Uygulama testi (`arch_ifc`) de artık bunu bekliyor: eski kuralla "MM-DVR-İç_duvar_15: doğru ≠ yanlış" diye düşüyor
  (denendi).
- Yolun IFC'si taşınmadan önceki çıktısıyla **bayt bayt aynı** (`yol.ifc`).
- Testler: `kentos-ifc` 6 (kasanın bandı: ortada 0,09–0,16; yüzden 2 cm, sol ve sağ; duvar kadar derin kasa duvarı
  doldurur), uygulama `arch_ifc` ve IFC dosyaları, plan ölçüleri (birim ve masaüstü
  `plan_dimensions_writes_four_sides…`).
- **2. bölüm:** 22 ortak durum (`scripts/fixtures/arch_ifc_cases.py`, KentOS kodu olmadan bölüm binasının tanımından:
  sayılar, plan, doğrulama, üzerine yazma, rampa uyarısı, boş/göreli/uzantısı yanlış/klasörü olmayan yol, var olan
  dosya, okunmayan saat, binasız çizim, büyük harfli uzantı, kırpılan yol, saatsiz yazım, 1. katı boş bina) masaüstü
  işleyicisinde (`fixtures.rs`) ve Python SDK'da (`test_command_cases.py`) geçiyor. Komut kataloğu, TypeScript tipleri,
  Python SDK ve manifest yeniden üretildi. Masaüstü: `arch_ifc_tests` (2), katalog (9), geçmiş (17).
- **Gerçek pencere** (`.run/gui-evidence/mimari-m47/`): 1920×1080 sanal ekran, gerçek fare ve klavye, ayrı ev klasörü;
  dosya penceresi için uygulamaya ait geçici bir D-Bus oturumu ve GTK portalı (sahibinin oturumuna ve portalına
  dokunulmadı). Dosya › Dışa aktar › "IFC dışa aktar (bina)…" → GTK kayıt penceresi → yol → Kaydet: 123 137 baytlık dosya
  yazıldı, durum çubuğu ve Komut geçmişi sayıları söyledi; pencereden yazılan dosya da `arch_ifc_check.py`'den **0
  bulguyla** geçti. ÖNCE (eski şerit) menüde bina modeli yok.

## Sonuçlar

- IFC'nin bundan sonraki her işi (okuma, başka disiplinler, denetim, IDS) `kentos-ifc`'e gider; başka paket IFC yazmaz.
- Dışa aktarma komut, MCP aracı ve Python işlevi olarak da kullanılır (`cad.arch.ifc`).
