# ADR 8068: Yangın, performans ve malzeme bilgisi — beyan, dayanak, modelin zorunlu alanları

- **Durum:** uygulandı (16 Kasım 2026). Plan: [bim-plan.md](../../plan/bim-plan.md) BIM-07, [ifc-uyum.md](../../plan/ifc-uyum.md)
  IFC-02; uygulama sırasında B7. İlk kabulün ikisi sağlandı; formun 9. sorusu artık modelden cevaplanıyor ama kesit
  binasında IFC-02'nin öbür satırları eksik olduğu için "hayır" (§6, §8).
- **Bağlam:** ADR [8047](8047-ifc-building-export.md) (binanın IFC'si), [8062](8062-digital-project-rule-catalog.md) (kural
  kataloğu; K10: kararı Python verir), [8064](8064-ek3-ek4-2d-checks-python.md) (`cad` olguları),
  [8035](8035-fire-escape-routes-distances-load-exits.md) (kaçış yolları), [8067](8067-building-3d-view-and-room-overlaps.md).
- **Dayanak:** Hazırlama Yönetmeliği Md. 12/1, 13; EK-6 Tablo 6.3 (kiriş), 6.7 (bina), 6.16 (kolon), 6.25 (kapı), 6.52
  (çatı), 6.56 (döşeme), 6.61 (merdiven), 6.66 (duvar); EK-6'nın malzeme notu ("Malzeme İsmi_Dayanım": Çelik_S355,
  Beton_C25); EK-9 Tablo 9.1 satır 9; Binaların Yangından Korunması Hakkında Yönetmelik (sınıf TS EN 13501-2 ile).
- **Web:** dokunulmadı (ADR 0228); TS sözleşmeleri üretildi, web işleyicisi TODOS.md §0.2'de. **`.kcad`:** mimari verisi
  sürüm 1 (`modules.arch = 1`, şema değişmez; ADR 8054 §10). **Sözleşme:** yeni alanlar (§2) ve `cad.arch.performance` v1;
  `kentos.auditFacts` 1'in `cad` bölümüne isteğe bağlı `modelGaps`. **Paket:** `dijital-proje` 0.6.0 (`zorunlu`).

## §1. Araştırma

| Kaynak | Ne diyor | KentOS kararı |
|---|---|---|
| EK-6 Tablo 6.7 (`mevzuat/bim/ek-6-1-zorunlu-a-f.md`, PDF s. 42–44) | `ConstructionMethod`: "Nesneye ait inşaat eyleminin türü; örneğin yeni inşaat, yenileme, tadilat"; `FireProtectionClass`: ulusal yönetmeliğin sınıflandırma tablosuna göre binanın ana yangın koruma sınıfı | Planın "yapım yöntemi" okuması düzeltildi: `ConstructionMethod` künyenin yapı türünden (yeni yapı, ilave, tadilat) yazılır, beyan edilmez; yangın koruma sınıfı binanın beyanıdır |
| EK-6 Tablo 6.52, 6.56, 6.61, 6.66 | `FireRating`: "Ulusal yangın güvenliği sınıflandırmasına göre (Türkiye için: Binaların Yangından Korunması Hakkında Yönetmeliğe göre)" | Değer müellifin sözüdür; biçimini Python kuralı okur (K10) |
| EK-6'nın malzeme notu (Tablo 6.3, 6.16, 6.56, 6.61 …) | IfcMaterial'ın adı "Malzeme İsmi_Dayanım" (Çelik_S355, Beton_C25) | Dayanım malzemenin alanıdır; model adı `Ad_Dayanım` yazar |
| TS EN 13501-2 ve BYKHY (dergipark makaleleri; lexpera, aile.gov.tr yayımlanmış metinler) | Sınıf ölçütleri R, E, I …; süre sınıfları 15, 20, 30, 45, 60, 90, 120, 180, 240, 360 dk; "REI 120", "EI 60" yazımı | Arayüzde örnek bu yazımdır; KentOS sınıfı reddetmez, yalnız boş, uzun ya da satır sonlu sözü reddeder |

## §2. Karar

1. **Beyan ve dayanağı** (`kentos_contracts::performance`): `Declared { value, basis }`, `DeclaredExit { exit, basis }`;
   dayanak `DeclaredBasis`: müellif beyanı ya da belge (adı ve isteğe bağlı günü). Türetilen değer (dışarıda, taşıyıcı,
   yapım eylemi) saklanmaz, yazıldığı yerde hesaplanır. Varsayılan yoktur: null değeri kaldırır, eksik sayılır; boş söz,
   başında ya da sonunda boşluk olan, 64 karakteri aşan ya da satır sonlu söz reddedilir.
2. **Nerede:** nesnenin kendi sınıfı `ArchInfo.fireRating` (duvar, döşeme, kolon, kiriş, çatı, korkuluk, merdiven, rampa)
   ve açıklığın `fireRating`'i; merdivenin `arch.exit`'i, kapının `exit`'i; duvar stilinin ve kapı-pencere tipinin
   `fireRating`'i; malzemenin `strength`'i; binanın `ArchSettings.performance.fireProtectionClass`'ı. **Nesnenin kendi
   değeri tipinkini geçer**; kendi değeri olmayan duvar stilininkini, açıklık tipininkini taşır.
3. **Komut** `cad.arch.performance` v1 (§9.2): `objects`, `wallStyle` (KentOS'un stili verilirse aynı kimlikli bir proje
   kopyası sınıfı tutar, Duvar stilleri penceresinin bir KentOS stilini alışı gibi), `openingType`, `material`,
   `building`. Tek geri alma adımı "Yangın ve performans"; geometri değişmez. Ret kodları `field_not_for_kind`,
   `nothing_to_set`, `style_not_found`, `opening_type_not_found`, `material_not_found`, `invalid_declared` (yeni),
   `invalid_strength` (yeni), sürüm ve kilitli katman. Binanın beyanı kendi geri alma işlemiyle (`ArchOp::Performance`).
   Stilin, tipin ve malzemenin doğrulaması (`problem`) da beyanı denetler: hiçbir yazma yolu boş söz bırakamaz.
4. **IFC:** `Pset_<Sınıf>Common.FireRating` (kendi, yoksa stilin ya da tipin), kapıda ve merdivende `FireExit`,
   `Pset_BuildingCommon.ConstructionMethod` (künyenin `yapi_turu`'su), `FireProtectionClass`, `NetPlannedArea`
   (mahallerin net alanları), malzemenin adı `Ad_Dayanım`. Boşluktan ibaret söz yazılmaz.
5. **Modelin zorunlu alanları** (`kentos_ifc::required`): katalogdaki EK-6 ve EK-7 zorunlu satırları, yazılacak modelin
   nesneleriyle karşılaştırılır (öznitelik, özellik seti, miktar seti, malzeme adı); bir satır sınıfının her nesnesinde
   değerliyse taşınır. `NOTDEFINED` tip değildir, boş söz değer değildir; ilişki satırları ve bağlamın kuzeyi burada
   okunmaz. Olgu `cad.modelGaps` (sınıf ve Türkçe adı, yol, satırın adı, tablo, sayfa, değeri olmayan nesneler, sınıfın
   nesne sayısı). Karar Python'da: `dijital_proje.zorunlu` her satırı nesneleriyle bir hata yapar, onarımı alanına göre
   söyler; Mimari denetimin Mevzuat grubunda (`digital_required_field`).
6. **EK-9'un 9. sorusu** artık sabit "hayır" değil: eksik satır yoksa evet, varsa kaç satır ve ilk dördü.
7. **Arayüz:** Öznitelikler'de öğenin ana satırlarında "Yangın dayanımı" (kendi sınıfı; altında "Duvar stilinden: REI
   120", "Tipinden: …", "Dayanak: …" ya da "Yok: modelde eksik") ve kapıda ve merdivende "Acil çıkış" (Evet, Hayır,
   Bilinmiyor). Duvar stilleri penceresinde "Yangın dayanımı" alanı taslağa yazılır, Kaydet ile gider.
8. **`.kcad`:** mimari verisi sürüm 1; yazıcı bu veri varken `modules.arch = 1` yazar, sürüm 1'i adlandırmayan dosyada bu
   veri reddedilir. Şema numarası değişmez.

## §3. Ortak durumlar

`fixtures/commands/v1/cad.arch.performance.json`, 42 durum, `scripts/fixtures/arch_performance_cases.py`'nin ADR'den yazılmış
kurallarından, KentOS kodu olmadan: duvarın, kapının, merdivenin, pencerenin, kolonun sınıfı; null'un stile dönüşü; yalnız
acil çıkış; stilin, tipin adıyla ve kimliğiyle sınıfı (kendi sınıfı olan nesne sayılmaz); dayanım ve kaldırılışı;
binanın sınıfı ve kaldırılışı; her ret kodu; geri al ve yinele.

## §4. Sınamalar ve kanıt

- **`kentos-contracts`** (3): beyanın ve dayanağın sorunları (boş, boşluk, 65 karakter, adsız belge, 29 Şubat), dayanımın
  yazımı, etiketli JSON.
- **`kentos-ifc`** (`required`, 2): stili olmayan duvarın sınıfı eksik ve kimliğiyle; boş söz eksik; `NOTDEFINED` tip
  değil; modelde olmayan sınıf sorulmaz. Bütün kasa 31 geçti.
- **`kentos-kcad`:** beyanlar mimari verisi sürüm 1 ile yazılıp aynen okunur; beyansız çizim değişmez.
- **`kentos-native-application`:** 42 ortak durum; yetenek, katalog, IFC sınamaları.
- **Paket:** `test_zorunlu.py` (6) ve öbür beşi; `kentos kek verify` 0 uyarı, 5 katkı.
- **Masaüstü** (`performance_tests`, 3): kesit binasında her duvar zorunlu alan bulgusunda; stillere REI 120 verilince bulgu
  kalkar, dosyada `IFCLABEL('REI 120')`; bir stilin sınıfı kaldırılınca yalnız o stilin duvarları bulguda, dosyada boş söz
  yok; kapının belgeli EI 60'ı ve FireExit'i dosyada. Öznitelikler'den " REI 90 " yazılır (kırpılır, tek adım "Yangın ve
  performans"), boş bırakılınca kalkar; kapının Acil çıkış'ı Evet. EK-9/9 "hayır" ve eksik satırları sayar.
- **Ortak süzgeçler:** `ribbon_tests`, `module_tests`, `history::text`, `catalog`, `settings`, `ek_rules`, `permit_`,
  `scene3d`, `opening_types`, `wall_styles`, `materials`, `properties`: 261 geçti; `typing_tests::project_settings_scale…`
  bu yük altında bir kez düştü, tek başına geçti (tuş zamanlaması; bu işle ilgisiz).
- **Bilinen:** `kentos-processing`'in `mimari.json` altın farkı (kaçış yolu 11.55 ↔ 4.45) ana daldan; altın dosyaya yalnız
  yeni `modelGaps` bölümü eklendi, o fark görünür kaldı (CLAUDE.md §23.4).
- **Resimler** `.run/gui-evidence/yangin-performans-8068/` (açık 1440×1180, koyu 1280×1100): Duvar stilleri'nde sınıf,
  duvarın stilinden taşıdığı sınıf, kapının belgeli sınıfı ve acil çıkışı, Denetim › Mevzuat'ta eksik zorunlu alanlar.

## §5. Kesit binasının eksikleri (EK-9/9)

Beyansız kesit binasında 48 zorunlu satır eksik: kiriş, kolon, döşeme, duvar, kapı ve merdivenin yangın dayanım sınıfı;
kapının ve merdivenin acil çıkışı; merdivenin tipi ve malzemesi; binanın kimliği, yapım eylemi, yangın koruma sınıfı,
kullanım türü, alanları; sahanın, katların, mahal kaplamalarının ve projenin satırları. Künyeli binada ve yangın beyanları
dolunca kalanlar IFC-02'nin işidir (§8).

## §6. Kabul (plan: BIM-07 ilk kabul)

| Ölçüt | Durum |
|---|---|
| Bölüm binasında bütün zorunlu yangın alanları dolunca formun 9. sorusu evet | **kısmen:** 9. soru modelden; yangın alanları dolunca o satırlar düşer, ama IFC-02'nin öbür satırları kaldığı için "hayır" |
| Bir duvar stilinin sınıfı silinince o stilin duvarları bulguda | sağlandı |
| Girilmemiş değer dosyaya boş metin olarak yazılmaz; eksik sayılır | sağlandı (yazıcı ve doğrulama) |

## §7. Gelecek

- **IFC-02'nin öbür satırları:** sahanın TAKS/KAKS'ı, çekme mesafeleri ve toplam alanı (imar durumundan), katların
  `ElevationOfSSLRelative`'i, mahal kaplamaları ve kalınlıkları, merdivenin tipi ve malzemesi, projenin açıklaması ve tipi,
  aktör rolleri, `TrueNorth`, `PostalBox`, `FootPrintArea` yazımı (EK-6'nın büyük P'si ile IFC'ninki).
- **Acil çıkış önerisi:** kaçış yollarının geçtiği kapı ve merdivenlerin aday gösterilmesi, beyan ile rota uyuşmazlığının
  bulgusu (ADR 8035'in olguları).

## §8. Eksik kalanlar

- **Pencereler:** Kapı ve pencere tipleri penceresinde, Malzemeler penceresinde ve Mimari ayarları'nda alan yok; bu üç
  değer komutla (Python, MCP) yazılır. Öznitelikler'de dayanak belgesi seçilmez (arayüz müellif beyanı yazar).
- **Türetilenin açıklaması:** dışarıda ve taşıyıcı değerinin gerekçesi ve örnekte değiştirilmesi yok.
- **Kaplama kalınlıkları, asansörün kişi ve yük kapasitesi** (planın BIM-07 listesindeki) yok.
- **Python yerel modülü:** `scripts/python/sdk.py` katmanı yazdı ve denetimi geçiyor; `pnpm py:test` (maturin ile yerel
  modül) koşturulmadı.
- **`.kcad` örnek dosyası:** `fixtures/kcad/v2`'de mimari verisi sürüm 1 örneği yok; gidiş-dönüş sınaması var.
- **Gerçek pencerede kabul** (B6'daki gibi) yapılmadı; resimler uygulamanın görünüm ağacından.
