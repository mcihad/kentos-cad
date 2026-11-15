# ADR 8062: Dijital proje kural kataloğu — ekler sayfa sayfa veride, şemayla ve bağımsız dökümle karşılaştırılmış; başlangıç ölçümü

- **Durum:** uygulandı (15 Kasım 2026). Plan: [bim-plan.md](../../plan/bim-plan.md) BIM-01 ve
  [ifc-uyum.md](../../plan/ifc-uyum.md) IFC-01; uygulama sırasında B1. İki paketin ilk kabulü sağlandı (§12). Eksik kalanlar
  §13'te.
- **Bağlam:** ADR [8047](8047-ifc-building-export.md) (IFC yazımı, EK-9 formu bellekteki modelden),
  [8061](8061-ifc-viewer-and-reading-engine.md) (okuma motoru denemesi bekliyor), [8008](8008-ek4-layer-key.md),
  [8027](8027-ek3-room-numbers-and-ellipse-tags.md), [8037](8037-turkish-title-block-sheet-sets-pdfa.md),
  [8033](8033-regulation-rules-rooms-storeys-doors.md) (kural motoru).
- **Araştırma:** [BIM yönetmeliği notu](../research/2026-11-15-bim-ifc-yonetmelik.md) §4 (17 nokta); eklerin metni
  [mevzuat/bim/](../../mevzuat/bim/README.md) (15 Kasım OCR'ı; bu işte 14 satırdaki okuma hatası düzeltildi, §2.3).
- **Web:** dokunulmadı (ADR 0228). İki crate de saf ve wasm32 için derlenir. Web dönüşünde pencerenin karşılığı
  TODOS.md §0.2'ye yazıldı.
- **`.kcad`:** değişmedi. Kurum ve proje katmanlarının projede saklanması bu işte yok (§5).
- **Bağımlılık:** dış bağımlılık eklenmedi. Çalışma alanına yeni bir crate geldi: `kentos-digital-project`
  (`crates/shared/digital-project`; serde ve serde_json, ikisi de çalışma alanında zaten var). Başvuru araçları:
  IfcOpenShell 0.9.0.post1 (yalnız betiklerde, `uv run --with ifcopenshell`) ve poppler'ın `pdftoppm`'i (sayfa
  görüntüsü). İkisi de ürünle dağıtılmaz.

## §1. Karar

1. **Kural veridir (plan K2).** Yönetmeliğin eklerindeki her satır KentOS'ta bir veri satırıdır ve nereden okunduğunu
   taşır: ek, tablo ya da madde, satır, PDF sayfası. Bir bulgu ya da dışa aktarılan bir alan kaynağını bu satırdan
   söyler.
2. **Ekler iki crate'tedir (§3).** IFC'ye ait olanlar (EK-5, 6, 7, 9) `kentos-ifc`'te; 2B olanlar (EK-1…4, 8), ortak
   veri modeli (kaynak, zorunluluk, karar, profil katmanı) ve genel kararlar yeni `kentos-digital-project`'te.
   Bağımlılık tek yönlüdür: `kentos-ifc` → `kentos-digital-project`.
3. **Katalog transkripsiyondan üretilir, elle yazılmaz.** `scripts/mevzuat/katalog.py`, `mevzuat/bim/*.md`'yi okur ve
   iki JSON dosyasını yazar. `--check` metinden sapmayı yakalar. Okuyamadığı bir tablo satırında durur; hiçbir satır
   sessizce atlanmaz.
4. **Şema, tablonun yazımından üstündür; kaynağın yazımı yine de saklanır.** Katalog ekin yazdığını korur. IFC 4.3
   şemasıyla karşılaştırma (§7) her farkı bulur, her fark bir kararla kapanır. Kararı olmayan yeni bir fark testi
   durdurur.
5. **Açık nokta açık kalır (plan K3).** İdarenin ya da Bakanlığın cevabını bekleyen bir parametrenin değeri boştur.
   KentOS'un varsayılanı (PDF/A-2b gibi) "KentOS'un seçimi" diye ayrıca söylenir.
6. **Pencere yalnız okur.** Dijital proje kuralları penceresi (Mimari › Proje ▾) kataloğu, kararları ve şema
   farklarını gösterir; çizimi ve projeyi değiştirmez. Bir çizimin modelinin ne taşıdığı yazılmış dosyadan okunur
   (IFC-08), bu listeden değil.

## §2. Kaynak ve okuma zinciri

### §2.1 Zincir

```text
Resmî Gazete ekleri PDF'i (241 s., SHA-256 7dc88971…)        hukuki metin; depoda yok
  └─ mevzuat/bim/*.md (OCR, her sayfa işaretli)               okunabilir metin; düzeltmeler §2.3
       └─ scripts/mevzuat/katalog.py [--check]                tablo ve madde ayrıştırıcısı
            ├─ crates/shared/digital-project/data/temel-2026.json   EK-1…4, 8
            └─ crates/shared/ifc/data/temel-2026.json               EK-5…7, 9
  IfcOpenShell (IFC4X3_ADD2 + IFC 4.3 set şablonları)
       └─ scripts/mevzuat/ifc_sozluk.py [--check] → crates/shared/ifc/data/ifc4x3-sozluk.json
  Elle yazılan kararlar: crates/shared/{digital-project,ifc}/data/kararlar-2026.json
```

### §2.2 Ayrıştırıcının kuralları

- Her tablo kendi `## Tablo N.M` başlığının altındadır. Sayfa kırılmasından sonra yinelenen başlık aynı tabloya
  eklenir. Her satır kendi sayfasını taşır.
- Sayfa kırılmasından sonra süren madde iki sayfayı da taşır (EK-4 4.12.3: s. 20–21). Satır da sürer: EK-9'un 9.
  sorusu s. 238–239'dur, Tablo 6.134'ün "Akustik / Performans Sınıfı" adı iki satıra bölünmüştür.
- Blok içinde yinelenen başlık (Tablo 6.82), başlığı kaybolmuş blok (Tablo 6.7 s. 44, 6.113, 6.134) ve özellik
  bloğundaki öznitelik (Tablo 6.93'ün `OverallWidth`'i) hücrelerinden okunur. Kaplamanın katman öznitelikleri yollarıyla
  okunur (Tablo 6.20, 6.43: `IfcRelAssociatesMaterial>IfcMaterialLayerSetUsage>IfcMaterialLayerSet>IfcMaterialLayer`).
- Satır kimlikleri kararlıdır: tablo satırı `T6.66/Pset_WallCommon.FireRating`, `T2.2/MM`, `T5.1/IfcWall`,
  `T9.1/21`; madde `M4.11`; şekil `S4.7/3`.

### §2.3 Bu işte düzeltilen okuma hataları

Şema karşılaştırması (§7) ve bağımsız döküm (§8) transkripsiyonda 14 satırda hata buldu. Her biri PDF'in kendi sayfasının
görüntüsüyle (300–400 dpi) doğrulandı ve `mevzuat/bim`'de düzeltildi:

| Metinde | PDF'te | Sayfa | Dosya |
|---|---|---|---|
| `CSBPoZNo` (3 kez) | `CSBPozNo` | 154, 202, 217 | ek-6-3, ek-6-4 |
| `### USERDEFINED, NOTDEFINED` | `USERDEFINED, NOTDEFINED` | 51 | ek-6-1 |
| `TWO QUARTER TURN RAMP: HALF TURN RAMP` | virgülle iki değer | 88 | ek-6-2 |
| `IfcReiAssignsToGroup` | `IfcRelAssignsToGroup` | 62 | ek-6-1 |
| `IfcRe)AssociatesMaterial` | `IfcRelAssociatesMaterial` | 79 | ek-6-2 |
| `InsulationStandardCIInsulationStandardClass`, Türkçe adı iki kez | `InsulationStandardClass`, "Yalıtım Standardı Sınıfı" | 149 | ek-6-3 |
| `FrictionLossI` | `FrictionLoss` | 151 | ek-6-3 |
| `FinishColor` | `FinishColour` | 111 | ek-6-3 |
| `LossCoefficientCurve` | `LossCoefficentCurve` (şemanın yazımıyla aynı) | 147 | ek-6-3 |
| `ValvePatter` | `ValvePattern` | 214 | ek-6-4 |
| `Qto_BuildingBaseQuantity` (2 kez) | `Qto_BuildingBaseQuantities` | 43 | ek-6-1 |

Son iki satır dikkat ister. OCR'ın sözlük adımı iki yerde basılı yazımı "düzeltmişti": `FinishColour` →
`FinishColor`, `LossCoefficentCurve` → `LossCoefficientCurve`. `Qto_BuildingBaseQuantity` ise önce kaynağın yazım hatası
sanıldı. Bağımsız döküm farklı okuyunca sayfaya yeniden bakıldı: PDF'te üç satırda da hücreye sarmış
`Qto_BuildingBaseQuantiti es` yazıyor. O yanlış karar kaldırıldı.

Basılı metnin kendi yazım hataları düzeltilmedi; kararlarla çözüldü (§6.3): `IfcDistrubitionSystem` (s. 150),
`TREpys_PozOzellikScti` (s. 113, görüntüde öyle), `NumberofCells` (s. 141).

## §3. Crate ayrımı

Sahibin sözü (15 Kasım): "ayrı crate olması gerekenler muhakkak ayrı crate olacak".

| Crate | İçinde | Neden ayrı |
|---|---|---|
| `kentos-digital-project` (yeni, `crates/shared/digital-project`) | `source` (Source, Level), `catalog` (EK-1…4, 8), `decision` (Decision, Param, Replace), `profile` (katmanlar, çözüm, ret) | 2B kurallar IFC değildir: dosya adı (PDF/A paftaları da kullanır), mahal adı, CAD katmanı. 2B denetim (BIM-03), Ruhsat takımı, kapsam kararı (BIM-02), sunucu (idare incelemesi) ve ileride web aynı veriyi IFC'yi derlemeden okur. Saf; yerel ve wasm32 |
| `kentos-ifc` (var) | `catalog` (EK-5…7, 9), `catalog_schema` (sözlük, karşılaştırma), `rules` (iki kataloğun tek listesi) | Kılavuz §14 kesin kuralı: IFC'ye ait her şey buradadır (sınıf, özellik seti, denetim) |

Uygulamada yalnız bağdaştırıcı kaldı. `kentos-native-application` (`mod-arch` özelliğiyle) ve masaüstü iki crate'i de
kullanır. `scripts/arch/deps.mjs` yeni crate'i `crates/shared/` grubunda kendiliğinden denetler: hedefleri yerel ve
wasm32, yalnız ortak kitaplıklara bağlanabilir.

Kural motoruna (ADR 8033, `kentos-native-application`) konmadı. O motor çizimi ölçer; katalog ise çizimden bağımsız
veridir ve saf kalmalıdır. Tarih ve kapsam kararı (BIM-02) da veriye dayanacağı için yeri `kentos-digital-project`
olacak; plan K2'deki "mevcut kural motorunda" ifadesi bu ADR'yle değişti.

## §4. Katalog

| Ek | Satırlar |
|---|---|
| EK-1 | 3 madde; Tablo 1.1'in 7 alanı |
| EK-2 | 16 madde; Tablo 2.1 şablonu; Tablo 2.2–2.14: 253 kod (12 disiplin, 53 mimari öge, 66 iç mimari öge, statik/mekanik/elektrik/altyapı/peyzaj alt disiplinleri, 13 harita ve 5 zemin ögesi, 12 dosya türü, 30 dosya tipi, 9 kat) ve Tablo 2.14'ün iki "…" dizisi |
| EK-3 | 7 madde; Tablo 3.1 şablonu; Tablo 3.2'nin 152 mahal adı |
| EK-4 | 68 madde; Tablo 4.1–4.6 şablonları; Tablo 4.7'nin 5 yazı tipi; Şekil 4.1–4.7 ve Şekil 4.7'nin 9 işaret ölçüsü |
| EK-5 | 9 madde; 69 zorunlu ve 67 isteğe bağlı sınıf, kodlarıyla; Tablo 5.3 şablonu |
| EK-6, EK-7 | 17 madde; 147 tablo (6.1–6.135, 7.1–7.12): tanım, ön tanımlı tipler; 1 146 öznitelik, özellik, miktar, malzeme ve ilişki satırı (356 zorunlu, 790 isteğe bağlı) |
| EK-8 | 3 madde (LOD 300) |
| EK-9 | 21 + 16 soru |

Toplam 1 870 satır (madde, şablon, kod, yazı tipi, işaret ölçüsü, sınıf, öznitelik ve özellik satırı, soru). Pencerenin
listesi bunlara 147 tablonun başlığını, 7 şekli, EK-1'in bilgi tablosunu ve EK-5'in şablonunu da ekler: 2 026 satır.
Her satırın sayfası var; test bunu satır satır denetliyor.

## §5. Profil katmanları

Sıra: temel profil (yönetmelik ve KentOS'un kararları) → idare profilleri → proje (plan ifc-uyum §3). Bir katman
şunları yapabilir: listeye kod eklemek (Md. 11/3: idarenin dört karakterli alt disiplini; EK-3 3.3: projenin mahali),
bir satırın zorunluluğunu değiştirmek (Md. 12/2) ya da bir parametreyi ayarlamak (görünüm tanımı, PDF/A bölümü, dosya
sınırı, düşey datum). Her değer onu kimin ayarladığını taşır.

Şu durumlarda katman bütünüyle reddedilir, ret iletisi nedeni ve çözümü söyler: EK-2 ya da EK-3'te zaten olan bir kod,
listenin biçimine uymayan kod (öge ve alt disiplinde dört büyük harf, Türkçe harfsiz; mahalde üç harf), katalogda
olmayan bir satır ya da parametre, sırası yanlış katman.

Katmanların projede ve kurum dosyası olarak saklanması bu işte yok. İlk gerçek idare profiliyle birlikte gelir (BIM-16,
IFC-11).

## §6. Kararlar

27 karar var, 6'sı açık. Kimlikler kalıcıdır; pencere ve bu ADR onları anar. Türler: **sayfa** (sayfadan okundu),
**şema** (şemadan çözüldü), **idare** (idare ya da Bakanlık cevaplar; parametre boş), **KentOS** (KentOS'un yorumu,
idarenin teyidi beklenir).

### §6.1 Araştırma notunun 17 noktası

| # | Konu | Karar | Tür | Sonuç |
|---|---|---|---|---|
| 1 | Varlık adındaki ayırıcı | K01 | sayfa | EK-5 5.5 ve 5.5.3 (s. 29): alanlar "-", açıklamanın içi "_"; `MM-DVR-Dış_duvar_25` |
| 2 | Mahalin modeldeki adı | K02 | sayfa | EK-5 5.5.4 (s. 29): numara ve ad açıklamada, ayrı ayrı ya da birlikte; KentOS ikisini birlikte yazar |
| 3 | EK-8 | K03 | sayfa | Nesne tablosu yok; LOD 300 (s. 237) |
| 4 | Görünüm tanımı | K04 | idare | Eklerde yok. Parametre `ifc.view` boş; KentOS'un bugünkü çıktısı DesignTransferView |
| 5 | Ön tanımlı tip | K05 | KentOS | EK-9/19 NOTDEFINED'ı yasaklamaz. KentOS kuralı: belirlenebilen tip yerine NOTDEFINED bulgudur |
| 6 | `IfcGroup.Tag` | K06 | şema | Tablo 6.37 ister (s. 76), şemada yok: yazılmaz, fark listesinde durur |
| 7 | Vergi no "Sayı" | K07 | şema | Tablo 7.4 (s. 225); şemada `IfcIdentifier`: rakamlardan oluşan metin |
| 8 | Boşluklu tip yazımları | K08 | şema | Şemanın yazımı (`ELEMENTEDWALL`, `FLAT_ROOF`); katalog basılı yazımı saklar |
| 9 | Giydirme cephe paneli | K09 | idare | İki listede de yok; vekil sınıfa düşülmez |
| 10 | Aks | K10 | şema | `IfcGrid` ve eksenleri; `AxisTag` aksın etiketi |
| 11 | Düşey datum | K11 | idare | EK-7 7.6 ITRF96 (+TUDKA99) ile EK-4 4.31.4 Helmert ortometrik çelişiyor; `height.datum` boş |
| 12 | Dosya boyutu sınırı | K12 | idare | Yayımlanmadı; `file.limit` boş, form sorusu incelemeye kalır |
| 13 | PDF/A bölümü | K13 | idare | `pdfa.part` boş; KentOS'un seçimi PDF/A-2b |
| 14 | Mahalde net alan | K14 | sayfa | Zorunlu 6.58'de yok, isteğe bağlı 6.125'te (s. 200) |
| 15 | `IfcZone` | K15 | KentOS | Tablo 6.68 genel (s. 109); KentOS bağımsız bölüm, eklenti, ortak alan için kullanır |
| 16 | e-PYS arayüzü | K16 | idare | Bakanlık (e-PYS Md. 4/4, 5/4, 5/6); `epys.interface` boş |
| 17 | Taslak metin | K17 | KentOS | Yalnız RG 5.8.2026/33331 |

### §6.2 Eklerden çıkan 2B kararlar

- **K18:** EK-2 2.5 dört karakter der, Tablo 2.11'in zemin kodları üç harflidir (s. 2, 11). Basılı kodlar geçerli
  sayılır; yeni kod dört harflidir.
- **K19:** Alt disiplin alanı Tablo 4.2'de dört, Tablo 4.5'te üç karakterdir. Her şablon kendi uzunluğuyla denetlenir.
- **K21:** Proje kodu: 1.1.1 hem "altı haneli sayı" hem "alanlar alt tireyle" der; Tablo 2.1 altı karakter verir. KentOS
  alt tiresiz altı rakam okur (`^[0-9]{6}$`); `000000` kimlik sayılmaz.
- **K22:** Tablo 2.14'ün "…" satırları dizinin sürdüğünü gösterir (B01…, K01…). Asma kat ve benzerleri KXX (2.9.2),
  kesit ve görünüş konum kodu XXX (2.9.3); `storey.pattern`.

### §6.3 Şema farklarının kararları

- **K30:** `IfcDistrubitionSystem` → `IfcDistributionSystem` (Tablo 6.92 başlığı, s. 150).
- **K32:** `TREpys_PozOzellikScti` → `TREpys_PozOzellikSeti` (Tablo 6.70, s. 113; öbür bütün tablolar Seti der).
- **K33:** `NumberofCells` → `NumberOfCells` (Tablo 6.87, s. 141).
- **K34:** Tablo 6.50 ve 6.117, IfcRoad'un tipi olarak yol parçalarının tiplerini sayar; IFC 4.3'te bunlar
  `IfcRoadPartTypeEnum`'undur.
- **K35:** Veri tipi sütunu "Bilgi Amaçlı"dır; ayrıştığında şemanın türü geçerlidir (sıcaklık aralığı "Metin",
  yapım yılı "Mantıksal" gibi satırlar).
- **K36:** Şablonun uygulanmadığı sınıfa verilen set yazılmaz (mekânsal zona `Pset_ZoneCommon`, boru parçasına
  bağlantı parçasının seti …). İsteğe bağlı oldukları için idare isteyene kadar yazılmaz.

## §7. Şema karşılaştırması

- **Sözlük:** IfcOpenShell 0.9.0.post1'in `IFC4X3_ADD2` şeması ve IFC 4.3 set şablonları (buildingSMART'ın dosyaları).
  İçinde kataloğun andığı 152 sınıf (öznitelikleri sırasıyla, türleri, isteğe bağlılığı, üst sınıf zinciri, ön tanımlı
  tip sayımı), 128 tip sayımı, kataloğun andığı 125 setin şablonu ve bütün set adları var (yanlış yazılmış bir setin en
  yakın adı aranabilsin diye).
- **Karşılaştırma (`kentos_ifc::catalog_schema::compare`):** bilinmeyen sınıf, öznitelik, set ve özellik; sınıfa
  uygulanmayan şablon; ön tanımlı tipin yazımı (harf ve rakamla karşılaştırılır, `FLAT ROOF` = `FLAT_ROOF`) ya da
  sayımda olmayışı; veri tipi türü; ulusal setin yazımı (`TREpys_` setleri kendi aralarında karşılaştırılır). Aynı
  anahtarlı bulgular birleşir. Yanlış yazılmış adın şemadaki en yakın adı söylenir.
- **Sonuç:** İlk koşunun bulguları arasında 14 satırdaki okuma hatası çıktı ve düzeltildi (§2.3). Kalan her bulgunun bir
  kararı var. `every_finding_is_settled_and_every_decision_still_finds_its_finding` testi iki yönü denetler: kararı
  olmayan bulgu ve bulgusu kalmamış karar. `a_misspelled_property_is_found` kabul durumudur: duvarın `FireRating`'i
  `FireRatng` yazılınca tek bir yeni bulgu çıkar: "IFC 4.3 şablonunda Pset_WallCommon setinin FireRatng özelliği yok;
  şemadaki en yakın ad FireRating".

## §8. Bağımsız dökümle karşılaştırma

- **Döküm:** ifc-ruhsat (MIT; commit `cd6d3ce5`; `src/ifc_ruhsat/ekler/` altındaki beş JSON dosyası). Sahibin izniyle
  karalama klasörüne indirildi; depoya konmadı. EK-2'nin disiplinlerini, EK-5'i, EK-6'nın 68 zorunlu tablosunu, EK-7'nin
  zorunlu tablolarını ve EK-9'un 21 sorusunu kapsıyor.
- **Betik:** `scripts/mevzuat/karsilastir.py DİZİN`. KentOS kodu kullanmaz, kataloğu veri olarak okur.
- **Sonuç:** 244 satır ve tablo karşılaştırıldı. Son durumda 16 fark var, ilk koşuda 18'di:

| Fark | Sayı | Kapanış |
|---|---|---|
| `Qto_BuildingBaseQuantity` satırları (Tablo 6.7) | 2 | Katalog yanlıştı (OCR); sayfa s. 43; düzeltildi |
| Ön tanımlı tip listeleri | 11 | Yorum: öbür döküm listeleri ekten değil şemadan alıyor (kendi notu); katalog ekin basılı listesini saklar |
| Veri tipi (6.20 LayerThickness, 6.47 FlowRateRange) | 2 | Öbürü yanlış: sayfalar (s. 58, 86) "Metin" der; şema türü K35 |
| `IfcGroup.Tag` (6.37) | 1 | Yorum: öbürü satırı atmış; katalog basılı satırı saklar, K06 çözer |
| Soru 6 ("öznitelikleri") ve 17 ("sınıfları") | 2 | Öbürü yanlış: s. 238–239 |

## §9. Rastgele 20 satır

`scripts/mevzuat/ornekle.py` 1 870 satırdan 20 satır seçti (tohum 8062). Her satır PDF'in kendi sayfasının görüntüsüyle
(100 dpi, gerektiğinde 300) hücre hücre karşılaştırıldı. İki sayfaya yayılan tablolarda satır numarası önceki sayfa
sayılarak denetlendi. **20 satırın 20'si birebir tuttu:**

T2.4/ISYA (s. 6), T2.4/TASI (7), T2.8/DOSB (10), T2.9/DIGR (10), T5.1/IfcRoof (31), T6.26/Tag (65),
T6.79/Qto_CableCarrierSegmentBaseQuantities.Length (126), T6.92/Pset_DistributionSystemTypeVentilation.MinimumHeight
(151; s. 150'deki 4 satırla 19. satır), T6.96/Pset_DuctSilencerTypeCommon.WorkingPressureRange (158), T6.99/Description
(161), T6.102/Qto_FootingBaseQuantities.Height ve Pset_FootingCommon.LoadBearing (166), T6.104/Description (169),
T6.111/Pset_ConcreteElementGeneral.ConcreteCover ve Description (179), T6.111/Qto_PileBaseQuantities.GrossSurfaceArea
(180), T6.114/Pset_PumpTypeCommon.ConnectionSize (185), T6.123/Qto_SlabBaseQuantities.GrossArea (197),
T6.129/ObjectType (207), T6.130/TREpys_PozOzellikSeti.CSBPozNo (208; Türkçe adı kaynakta da boş).

## §10. Başlangıç ölçümü

- **Yöntem:** `scripts/mevzuat/olcum.py`, yazılmış IFC dosyasını IfcOpenShell'le okur, KentOS kodunu kullanmaz. EK-6 ve
  EK-7'nin zorunlu satırlarından dosyada sınıfı bulunanların her biri için değeri olan nesneleri sayar. Bir kararın
  `replaces`'ı varsa ad yerine kullanılan ad ölçülür. Sonuç `fixtures/arch/v1/ifc/katalog-olcum.json`'dadır, her dosyanın
  SHA-256'sıyla.
- **Dosyalar:** bölüm binasının iki çıktısı. Yazıcının bugünkü çıktısı test yeniden yazdığında baytı baytına aynıydı.
  Planın andığı 2+1 depoda model dosyası olarak yok: yalnız gerçek pencere senaryosu var (ADR 8052). `konut-3-1` ve
  `Konut100m2` örnekleri düz CAD çizimidir (duvar yok). 2+1 ölçümü örnek proje gelince eklenecek (§13).

| Dosya | Uygulanan zorunlu satır | Değeri var | Yok |
|---|---|---|---|
| `kesit-binasi.ifc` (künyesiz) | 95 | 46 | 49 |
| `kesit-binasi-kunye.ifc` | 121 | 81 | 40 |

**Künyeli dosyada yok olanlar:**

- **Yangın:** `FireRating`, hiçbir elemanda yok (duvar, döşeme, kolon, kiriş, kapı, merdiven); kapı ve merdivende
  `FireExit` yok.
- **Malzeme:** adlar "Malzeme İsmi_Dayanım" biçiminde değil (Tuğla, Betonarme); merdivende malzeme yok.
- **Ön tanımlı tip:** merdivende NOTDEFINED.
- **Bina:** `ConstructionMethod`, `FireProtectionClass`, `OccupancyType`, `GrossPlannedArea`, `NetPlannedArea`,
  `EavesHeight`, `FootPrintArea`, `GrossFloorArea`, `Pset_Address.PostalBox`.
- **Kat:** `ElevationOfSSLRelative`.
- **Saha:** `ObjectType`, `FloorAreaRatio`, `SiteCoverageRatio`, `TotalArea`, `TREpys_ParselOzellikSeti`'nin beş alanı.
  Örnekte imar durumu yok.
- **Mahal:** kaplama setinin altı alanı. Örnekte kaplama yok.
- **Proje:** `Description` ve `ProjectType`. **Kişi ve kuruluş:** `IfcPerson.Roles`, `IfcOrganization.Roles`,
  `IfcActorRole.UserDefinedRole`. **Gösterim bağlamı:** `TrueNorth`.

**Kod okumasıyla ayrışan:** araştırma notu §5 ve plan ifc-uyum §5 bazı bina alanlarını kod okumasından "✓" saymıştı.
Bu örnekte dosyaya yazılmadılar (kullanım, planlanan brüt alan, taban alanı, brüt kat alanı). Sebep, verilerinin çizimde
olmamasıdır (program ve imar durumu yok). Kesin söz dosyadan ölçülenindir (plan K4). Planın bugün sütunu bu ölçüme göre
düzeltildi.

## §11. Arayüz

- **Dijital proje kuralları penceresi** (`apps/desktop/src/arch/digital_rules.rs`, `arch.digitalRules`, ikon
  `digitalRules`, seçenek A). Yeri Mimari › Proje ▾, Model kalite kontrol formunun ardı. Yol pencerelerinin iskeleti
  kullanılır:
  - solda profil (Temel profil 2026.1, RG 5.8.2026/33331, ekler) ve süzgeç (Ek, Zorunluluk, "Yalnız mimari modelin
    kuralları", Ara);
  - sağda sayılar (kural, gösterilen, karar, açık karar, şema farkı), açık kararların notu ve "Kurallar | Kararlar |
    Şema farkları" sekmeli, satırları seçilen tablo;
  - altta seçili satırın metni, kaynağı ve ona dokunan kararlar.

  Arama Türkçe harfleri katlar ve sınıfın Türkçe adını da arar ("duvar yangın" → `T6.66/Pset_WallCommon.FireRating`).
- **Mimari denetim › Mevzuat** dijital proje bulgularının kuralı, maddenin ardından ekin satırını ve formun sorusunu
  sayfasıyla söyler. Sayfalar katalogdan gelir, kodda sabit yazılmaz. Örnek: "Dijital Proje Yön. 6 · RG 5.8.2026/33331
  · EK-1 m. 1.1.1 · PDF s. 1 · EK-9 Tablo 9.1 satır 3 · PDF s. 238".
- **Araç = komut:** pencere çizimi değiştirmediği için bu kural uygulanmaz (`operations.json`: `host`). Python ve MCP
  için salt okuyan bir sorgu komutu (bir sınıfın zorunlu satırları, bir kodun anlamı) sonraki iştir (§13).

## §12. Kabul

| Kabul (plan) | Kanıt |
|---|---|
| BIM-01: katalogdaki her satırın sayfası var | `every_row_has_its_page_and_a_unique_id` (iki crate), `every_rule_is_listed_once_with_its_page` |
| BIM-01: rastgele 20 satır sayfa görüntüsüyle birebir | §9, 20/20 |
| BIM-01: 17 nokta karara bağlı | §6.1 |
| BIM-01: bağımsız dökümle satır satır, fark listesi ADR'de | §8 |
| BIM-01, IFC-01: ölçüm raporu ADR'de | §10, `katalog-olcum.json` |
| IFC-01: bilerek yanlış yazılan özellik adı şema karşılaştırmasında düşer | `a_misspelled_property_is_found` |
| IFC-01: temel profil ve kurum profili katmanları | §5, `profile::tests` |

## §13. Kalanlar ve sonraki işler

- **2+1 ölçümü:** model dosyası yok. Örnek proje (BIM-20) gelince `olcum.py` ona da koşulur.
- **Python ve MCP sorgusu:** bir sınıfın zorunlu satırlarını, bir kodun anlamını, bir kararın durumunu veren salt
  okuyan komut (katalog, ≥ 20 bağımsız durum, `kentos.cad`, MCP).
- **Katalogdan beslenme:** yazıcı (IFC-02), 2B denetim (BIM-03) ve kapsam kararı (BIM-02) kendi sabitleri yerine
  katalogdan okuyacak. Bu ADR katalog ile yazıcının adlarını yalnız testte eşledi (`every_class_kentos_writes_has_ek_5_s_code`).
- **EK-6 ve EK-7'nin uzun açıklamaları:** yalnız kurala dayanan satırlar sayfasıyla karşılaştırıldı (K07, K11, K14, ön
  tanımlı tipler ve §8'deki satırlar). Geri kalanı bilgi amaçlıdır.
- **Şerit üreticisi:** `scripts/ui/ribbon.py` depodaki `catalog/v1/ribbon.json`'u üretmiyor. Yeniden çalıştırılınca
  sonradan elle eklenmiş 249 satırlık düğmeyi siliyor; bu işten önce de böyleydi. Yeni düğme `ribbon.json`'a elle,
  `ribbon.py`'ye de eklendi. Üreticinin onarımı ayrı bir iştir.
