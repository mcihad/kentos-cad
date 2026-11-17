# ADR 8088: Yangın kompartımanı IFC'de — paketin kompartımanları `IfcZone` olarak

- **Durum:** uygulandı (17 Kasım 2026). Plan: [bim-plan.md](../../plan/bim-plan.md) BIM-08 ("Gruplar: … yangın
  kompartımanı (BIM-13)"), BIM-13; ADR [8086](8086-compartments-across-storeys.md) §6 ("Kompartımanın IFC grubu").
- **Bağlam:** ADR [8081](8081-fire-compartments.md) (kompartımanlar), [8086](8086-compartments-across-storeys.md) (katlar
  boyunca), [8069](8069-room-zone-far-model.md) (bölümlerin `IfcZone` grupları), [8047](8047-ifc-building-export.md)
  (binanın IFC'si), [8073](8073-ifc-viewer-first-slice.md) (IFC modeli), bim-plan K10 (kural Python'da, ölçü Rust'ta).
- **Dayanak:** Binaların Yangından Korunması Hakkında Yönetmelik 4/tt, 24; Mimarlık ve Mühendislik Projelerinin Dijital
  Olarak Hazırlanması Hakkında Yönetmelik (RG 5.8.2026/33331) EK-5 Tablo 5.1 ve §5.5, EK-6 Tablo 6.68 ve 6.135.
- **Web:** TS türleri yeniden üretildi (`ExportIfcBuilding`, `IfcBuildingExported`, `IfcCompartment`); web'e başka iş yok
  (ADR 0228). **`.kcad`:** değişmedi. **Sözleşme:** `cad.arch.ifc` v1 girdisine isteğe bağlı `compartments`, çıktısına
  `compartments` sayısı.

## §1. Araştırma (17 Kasım 2026)

| Kaynak | Ne diyor | KentOS kararı |
|---|---|---|
| EK-6 Tablo 6.68 (`IfcZone`, zorunlu) | Zon mekânların, kısmi mekânların ya da zonların gruplanmış hâlidir; mekânlar bitişik olmayabilir; zonun kendi biçimi yoktur; bir mekân sıfır, bir ya da birden çok zonda olabilir (`IfcRelAssignsToGroup`). Zorunlu: `Name` (EK-5'e göre), `LongName`, `ObjectType` | Kompartıman bir `IfcZone`, mahalleri üyesi; katlar boyunca kompartıman (ADR 8086) bitişik olmayan mahalleri de toplar. Bir mahal bağımsız bölümünün zonunda ve kompartımanınınkinde birlikte durur |
| EK-6 Tablo 6.135 (isteğe bağlı) | `Description`, `Pset_ZoneCommon` (planlanan brüt ve net alan), `Pset_SpaceFireSafetyRequirements` | Yazılmadı: planlanan alan kompartımanın ölçülen alanı değildir; §6 |
| EK-5 Tablo 5.1, §5.5 | `IfcZone`'un kategori kodu ZON; ad `Disiplin-Kod-Açıklama`, açıklamada boşluk ve tire yok | `MM-ZON-Yangın_kompartımanı_N` |
| EK-6 Tablo 6.60 (`IfcSpatialZone`) | Mekânsal zon kendi konumu ve biçimiyle; emsal zonu bununla yazılıyor (ADR 8069) | Kompartıman `IfcSpatialZone` (FIRESAFETY) değil: kompartımanın kendi biçimi yok, mahallerinden oluşur; EK-6'nın zon tanımı gruptur |
| FireBIM modelleme kılavuzu (ITEA, 2026; [itea4.org](https://itea4.org/project/workpackage/deliverable/document/download/1121/firebim-modelling-guideline-snapshot-2026-09-07.html)) | Bir yangın kompartımanının mahalleri bir `IfcZone`'da gruplanır, ayrıntılı tasarımda geometrili mekânsal zon da olabilir; dayanımlar `Pset_*Common.FireRating`'te | Aynı: `IfcZone`; dayanımlar zaten elemanların setinde (ADR 8068) |
| Model denetimi (Solibri'nin kompartıman kuralı; ADR 8087 §1) | Kompartımanlar modelde tanımlıysa onları ayıran duvar, kapı ve pencere denetlenir; tanım yoksa kural çalışmaz | KentOS kompartımanı kendisi bulur ve dosyaya yazar: dosyayı okuyan denetçi kompartımanları tanımlı bulur |
| bim-plan K10 | Kural Python paketinde, ölçü Rust'ta | Kompartımanı paket bulur, `cad.arch.ifc` verileni yazar; kompartımanı Rust'ta yeniden hesaplamak kuralın ikinci kopyası olurdu |

## §2. Karar

- **K110 — Kompartıman bir `IfcZone`'dur** (`kentos_ifc::compartment::add`): üyeleri kompartımanın dosyada yazılan
  mahalleri (`IfcRelAssignsToGroup`); adı `MM-ZON-Yangın_kompartımanı_N`, uzun adı "Yangın kompartımanı N", nesne türü
  "Yangın kompartımanı". GlobalId ilk mahalin kimliğinden: o mahal öne geçtikçe aynı kalır. Bir binanın dosyasında
  (ADR 8072 §2) mahallerinden hiçbiri olmayan kompartıman yazılmaz ve numara almaz.
- **K111 — Komut yazar, paket bulur:** `cad.arch.ifc`'nin `compartments` girdisi (her biri mahallerinin kalıcı
  kimlikleri) verildiği gibi yazılır; boş liste hiçbirini yazmaz. Verilmezse Python çalıştıran ev sahibi paketi sorar:
  masaüstünün IFC dışa aktar (bina) ve kesilmiş gövdeli hâli, başsızın `call_envelope` ve `run`'ı (MCP, komut satırı).
  Çizimin mahali olmayan kimlik reddedilir (`invalid_ifc`, yol `compartments[i].rooms[j]`). Paket yanıt vermezse dosya
  kompartımansız yazılır ve söylenir (başsızda `regulation_rule_unavailable` uyarısı, masaüstünde çıktı satırı); başvuru
  günü 24/7'nin bilinen ilk hâlinden önceyse paket nedenini verir (`export_losses`).
- **K112 — Paketin `kompartimanlar`ı** (`tr.kentos.mimari-mevzuat` 0.12.0): her kompartımanın mahalleri, katları
  (alttan üste) ve Ek-4'ün saydığı alanı; en alt katı, sonra ilk mahali öne. Sınırın süresi 24/7'nin başvuru günündeki
  hâlinden (gün yoksa en yeni hâl). Denetimin kompartımanlarıyla aynı işlev (`kompartiman.kompartimanlar`).
- **K113 — IFC modeli zonu söyler:** seçili nesnenin ayrıntılarında atandığı her grup için bir "Grup" satırı: uzun
  adı, yoksa adı, nesne türü ya da sınıfı (`Model::groups_of`: "Bağımsız bölüm 1", "Yangın kompartımanı 3"); grubu
  olmayan nesnede satır yok. Ağaç zonları zaten listeliyordu.

## §3. Uygulama

- **IFC çekirdeği:** `crates/shared/ifc/src/compartment.rs` (`add`, `OBJECT_TYPE`); okuma `read/details.rs`
  (`groups_of`).
- **Sözleşme:** `IfcCompartment { rooms }`, `ExportIfcBuilding.compartments`, `IfcBuildingExported.compartments`;
  katalog, TS türleri ve Python taslakları yeniden üretildi.
- **Uygulama:** `arch_ifc::check` mahalleri doğrular ve zonları ekler.
- **Başsız:** `regulation::compartments` (ön adım) ve `regulation::warn`; `dispatch::run` ve `call_envelope` sorar.
- **Masaüstü:** `App::fire_compartments` (eşzamanlı, `crate::standards::call`); başarı iletisi "N yangın kompartımanı"
  sayar.
- **Paket:** `kompartiman.gruplar`, `mimari.kompartimanlar`; sürüm 0.12.0.

## §4. Sınamalar

- **IFC çekirdeği:** `each_compartment_is_a_zone_of_the_spaces_the_file_holds` (dosyada mahali olmayan kompartıman
  numara almaz, GlobalId ilk mahalden); `a_space_says_the_zones_it_is_grouped_in` (10 mahalin 5'i bir, 5'i öbür
  kompartımanda).
- **Komut durumları** (`fixtures/commands/v1/cad.arch.ifc.json`, 6 yeni, 33 durum; `scripts/fixtures/arch_ifc_cases.py`
  bölüm binasının tanımından): her kat bir zon (yazma ve planı), boş liste, mahalsiz kompartıman numara almaz,
  bilinmeyen kimlik ve duvarın kimliği reddedilir.
- **IfcOpenShell** (`scripts/fixtures/arch_ifc_check.py fixtures/arch/v1/ifc/kesit-binasi-kompartiman.ifc`): 0
  doğrulayıcı bulgusu; iki `IfcZone`, adları ve uzun adları, üyeleri tam o katın mahalleri; öbür dosyalarda zon yok.
- **Uygulama:** `the_fire_compartments_are_zones_of_their_rooms` (dosyayı yazar). **Başsız:**
  `a_headless_building_ifc_writes_the_package_s_fire_compartments` (bütün duvarlar EI 60: toplantı salonu ile koridor
  bir, merdiven öbür kompartıman; boş listeyle zon yok).
- **Paket:** `Gruplar` (galeri iki katı bir kompartıman yapar, 9000 m²; REI 90 duvar ikiye ayırır; 24/7'den önceki gün
  nedeniyle boş; mahalsiz çizim boş).
- **Masaüstü** (`compartment_tests`): üç kompartımanlı katın dosyasında üç zon, her biri bir mahalli; beyansız duvarlarla
  tek zon. **Resimler:** `.run/gui-evidence/kompartiman-ifc-8088/` (IFC modeli, Kazan dairesi seçili, "Grup" satırı).

## §5. Arayüz

- Yeni pencere yok. IFC dışa aktar (bina) iletisi yazılan kompartımanları sayar; IFC modeli'nin Özellikler panelinde
  "Grup".

## §6. Sıradaki adımlar

- **Zonun alanı ve sınırı:** kompartımanın alanı ve Ek-4 sınırı `Description` ya da bir özellik setinde (EK-6'nın
  isteğe bağlı listesinde uygun alan yok; yerel özellik seti kararı gerek).
- **Kompartımanın çizimde gösterimi:** kat planında kompartımanların renkle boyanması (Denetim'in bulgusundan ayrı bir
  görünüm).
- **Mekânsal zon (FIRESAFETY):** kompartımanın kendi biçimi istenirse `IfcSpatialZone` olarak da.
