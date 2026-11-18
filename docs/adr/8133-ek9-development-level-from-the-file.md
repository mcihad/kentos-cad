# ADR 8133: EK-9'un 7. sorusu, gelişim seviyesi, yazılmış dosyadan

- **Durum:** uygulandı (18 Kasım 2026). Plan: [bim-plan.md](../../plan/bim-plan.md) BIM-14 (Kalan: "7. sorunun gelişim
  seviyesi ölçülmüyor"), R10 (Md. 14, EK-8); [ifc-uyum.md](../../plan/ifc-uyum.md) IFC-06 (Kalan: "gelişim seviyesi
  kanıtının müellif incelemesi kaydı"), IFC-08.
- **Bağlam:** ADR [8074](8074-ek9-from-the-written-file.md) (formun 21 sorusu dosyanın olgularından; 7. soru incelemeye
  bırakılmıştı), [8075](8075-bim-permit-package.md) ve [8077](8077-ek9-author-answers-in-the-drawing.md) (paket ve
  müellifin cevapları), [8070](8070-missing-building-elements.md) (kitaplık bloklarının gövdesi kutu; rampanın gövdesi),
  [8071](8071-geometry-accuracy.md) (gövdeler, iki görünüm). Kurallar Python'dadır, Rust olguları verir (bim-plan K10).
- **Web, `.kcad`, komut:** değişmedi; `cad.form.answer`'ın örneği 7. soru yerine emsal hesabını (10) cevaplar.
  Olgular (`kentos.ifcFacts` 1) alan ekledi, eskisini değiştirmedi.

## §1. Sorun

Formun 7. sorusu ("Modelde yer alan varlıklar Yönetmeliğin 14 üncü maddesinde verilen gelişim seviyesine uygun olarak
modellendi mi?") her dosyada müellifin incelemesine kalıyordu: KentOS hiçbir şey ölçmüyor, yayın her pakette bu soruya
müellifin Evet'ini bekliyordu. Sorunun ölçülebilir yarısı dosyadadır: elemanların gövdesi var mı, gövde gerçek biçim mi
yoksa bir sınır kutusu mu, katı mı.

## §2. Araştırma (18 Kasım 2026)

| Kaynak | Ne diyor | KentOS kararı |
|---|---|---|
| Hazırlama Yönetmeliği Md. 14 ([lexpera](https://www.lexpera.com.tr/resmi-gazete/metin/mimarlik-ve-muhendislik-projelerinin-dijital-olarak-hazirlanmasi-hakkinda-yonetmelik-33331)) | "BIM modeli, EK-8'de verilen model gelişim seviyesine göre düzenlenir." | Soru EK-8'e göre ölçülür |
| EK-8 8.1, 8.2 (`mevzuat/bim/ek-8-gelisim-seviyesi.md`, PDF s. 237) | Ruhsat eki modellerde varlıklar "Detaylı Modelleme" seviyesinde: şekil, boyut, konum ve yönelim net, temsilden doğrudan ölçülebilir; LOD 300'e karşılık gelir. Alfanümerik bilgi EK-6 ve EK-7'den (8.2). Nesne tablosu yok | Geometrik yarı 7. sorunun; alfanümerik yarı 9. sorunun (zorunlu alanlar), yinelenmez (K316) |
| BIMForum LOD Specification, LOD 300 (aktaran [BD+C](https://bdcnetwork.com/level-development-will-new-standard-bring-clarity-bim-model-detail)) | Eleman, belirli bir sistem, nesne ya da topluluk olarak miktarı, boyutu, biçimi, konumu ve yönelimiyle grafikle gösterilir; LOD 200 yaklaşık miktar, boyut ve biçimdir | Gerçek biçim ister: bir sınır kutusu LOD 300 değildir (K316) |
| IFC4, `IfcShapeRepresentation` ([IFC4 belgesinin aynası](https://bsi-infraroom.github.io/ifc4steelfabrication-documentation/em11v3/schema/ifcrepresentationresource/lexical/ifcshaperepresentation.htm)); IFC4'ün eklediği türler | `RepresentationType`: SweptSolid, AdvancedSweptSolid, Brep, CSG, Clipping, SolidModel (katılar); SurfaceModel (yüzey); BoundingBox "simplistic 3D representation by a bounding box"; MappedRepresentation (eşlenen gösterim); IFC4 ile Tessellation ve AdvancedBrep. Bir gösterimin öğeleri türüne uyar | Dosyanın kendi beyanı okunur; Tessellation yüz setleri `Closed` diyorsa katıdır (K316, K319) |
| ISO 7817-1:2024 (bilgi gereksinim düzeyi), içindekiler ([AS ISO 7817.1](https://store.standards.org.au/product/as-iso-7817-1-2024)) | Geometrik bilgi beş yönle tanımlanır: ayrıntı, boyutluluk, konum, görünüş, parametrik davranış | Boyutluluk (3B katı) ve konum (6. ve 15. sorular) ölçülür; ayrıntının yeterliliği ölçülmez, müellifin (K318) |

## §3. Karar

- **K316 — Ne ölçülür:** dosyanın her yapı elemanı (`IfcElement`'in alt sınıfı; açıklık `IfcFeatureElement` ve sanal
  sınır `IfcVirtualElement` dışında). Gövde gösterimi (kimliği `Body`, IFC4'ün yedeği `Body-FallBack`): yoksa eleman
  gövdesizdir, ancak parçaları gövdeli bir topluluksa (`IfcRelAggregates`, `IfcRelNests`) değildir; yalnız sınır kutusu
  (`BoundingBox`) gerçek biçim değildir; katı türleri SweptSolid, AdvancedSweptSolid, Brep, AdvancedBrep, CSG, Clipping,
  SolidModel ve kapalı diyen Tessellation; eşlenen gösterim eşlediğinin türüyle. Mahallerin kapalılığı 14. sorunun,
  alfanümerik bilgi 9.'nun.
- **K317 — Okuyucunun üçgen ağı ölçüt değil:** motorun ağlarında kendi duvarlarımızın 16'sı da, döşemelerin hepsi de
  köşeleri birebir kaynaşmadığı için "açık" çıkıyor (Boole kesimi). Ölçüt dosyanın gösterim türüdür; ağ görüntüdür,
  ölçü kaynağı değildir (kılavuz §23.3). Okuyucunun çıkaramadığı gövde ölçülmez, nedeniyle incelemeye kalır.
- **K318 — Kutu yerine kitaplık bloğu:** KentOS'un kitaplık bloklarının (asansör, vitrifiye, mobilya) gövdesi
  çizilmeyen biçimlerinin yerine taban izinin yüksekliğe kaldırılmış kutusudur (ADR 8070). Dışa aktarım bunu bilir ve
  söyler (`Element.stand_in`, beklenen nesnenin `standIn`'i): soru o binada müellife kalır, kaç blok olduğu notta.
- **K319 — Sonuç:** gövdesiz ya da yalnız sınır kutulu eleman varsa kaldı (`digital_lod_no_body`,
  `digital_lod_bounding_box`); katı olmayan (`digital_lod_not_solid`), okunamayan (`digital_lod_unread`) ya da kutu
  yerine (`digital_lod_stand_in`) varsa inceleme, bulgularıyla; hiçbiri yoksa kaynağı verilen dosyada (KentOS'un yazdığı:
  beklenen nesneler var) geçti, notunda türlerin sayısıyla ("58 yapı elemanının hepsi katı gövdeli (SweptSolid 58),
  çizimden ölçüsünde yazıldı."); kaynağı verilmeyen dosyada (başka bir programın) ölçülen not ile inceleme: ayrıntının
  ve ölçünün gerçekliği incelemecinindir. Tablo 9.2'nin 6. satırı (öbür disiplinler) aynı kuralı okur.
- **K320 — Yazıcı:** bir gövdede yüzler (eğik kol, eğik korkuluk) ile prizmalar (sahanlık, dikme) birlikteyse prizmalar
  da kapalı yüz setleri olarak yazılır: üstü delikleri boşluklu (`IfcIndexedPolygonalFaceWithVoids`), altı ters, yanları
  dışa; yayları çokgenleriyle (`flat_rings`). IFC'nin gösterim türleri karışmaz: 'Tessellation' yalnız mozaik öğe tutar.
  Önceden böyle bir gövde 'Tessellation' diye yazılıp içine `IfcExtrudedAreaSolid` konuyordu; ölçüm bunu buldu (rampa
  "katı değil" görünüyordu). Yalnız prizmalı gövde eskisi gibi SweptSolid'dir, yaylar yay kalır.

## §4. Uygulama

- `crates/shared/ifc/src/read/shapes.rs` (yeni: `ShapeKind`, `Model::shape_kinds`: kimlik, tür, eşlenenin türü, yüz
  setlerinin `Closed`'ı); `read/facts.rs` (nesneye `element`, `shapes`); `read/facts_bodies.rs` (`notShown`).
- `crates/shared/ifc/src/building.rs` (`Element.stand_in`; `Writer::prism_faces`, karışık gövde); `arch.rs` (kitaplık
  bloğu kutu yerine); `expected.rs` (`Expected.stand_in`); `crates/native/application/src/arch/ifc_facts.rs`
  (`standIn`).
- `catalog/standards/dijital-proje/src/dijital_proje/form.py` (`s7`, `GOVDE`, `KATI`).
- `scripts/fixtures/ifc_form_reference.py` (bağımsız okuyucu: `element`, `shapes`, `parent`, `notShown`).
- `crates/shared/commands/src/catalog/arch_project.rs` (`cad.form.answer`'ın örneği); `catalog/v1/additions.json`
  (`arch.qualityForm`'un açıklaması).

## §5. Sınamalar

- **Rust** (`kentos-ifc --features view`, 102): `a_body_of_faces_and_a_prism_is_one_tessellation` (1 m küp yüzleri ile
  2 m karede 1 m delikli 0,5 m prizma: tek 'Tessellation', `IfcExtrudedAreaSolid` yok, iki boşluklu yüz; motorla geri
  okunan hacim 2,5 m³, yani bütün yüzler kapalı ve dışa bakar; olgularda `closed: true`); olguların altı dosyası yeniden
  yazıldı (yalnız eklenen alanlar).
- **Python** (`test_form.py`, 42): kaynağı verilen bölüm binası geçer, verilmeyen incelemedir; eksen gösterimli duvar
  `digital_lod_no_body` ile kalır, parçası gövdeli topluluk kalmaz; sınır kutusu kalır; kapalı demeyen üçgen ağ incelemedir,
  kapalı diyen geçer; okunamayan gövde ve kutu yerine kitaplık bloğu incelemedir, cevap boş kalır. `test_bim.py`: tam
  paketin 7. sorusu artık dosyadan Evet; müellife 1. ve 10. kalır.
- **Bağımsız okuyucu** (IfcOpenShell 0.9.0.post1, çevrimdışı): beş dosyanın formu KentOS'unkiyle aynı (`--check`); ayrıca
  476 nesnenin eleman bayrağı, gösterim türleri ve üst nesnesi iki okuyucuda bire bir aynı (tek seferlik karşılaştırma).
- **Masaüstü** (`bim_permit_tests`): bölüm binasının paketinde 7. satır "geçti / Evet", müellife yalnız 1.; form, ruhsat,
  BIM, IFC, 3B ve ortak süzgeçler yeşil (178). Ortak durumlar (`cad.arch.ifc`, 33) değişmedi.
- **Resimler:** `.run/gui-evidence/gelisim-seviyesi-8133/` (`ifc_viewer_tests::lod_screens`): çizimdeki formun notlar
  yarısı; bölüm binasında 7. satır Evet ve ölçülen not; kitaplık bloklu binada (rampa, asansör, lavabo, masa) boş cevap
  ve "64 yapı elemanından 3 kitaplık bloğu kutu; öbürleri katı gövdeli…"; açık ve koyu.

## §6. Kalanlar

- Formun kalıcı kaynak türü (`TableSource`): mimari verisinin yeni sürümünü ister, bağımsız `.kcad` okuyucusu mimari
  verisi okumadıkça bekler (TODOS.md'de ayrı iş).
- Ayrıntının yeterliliği: EK-8'de eleman tablosu yok; idare bilgi gereksinimini tanımlarsa (8.2) eleman sınıfı başına
  ölçüt eklenir.
- Kitaplık bloklarının gerçek biçimi (IFC-03'ün kalanı): gelince 7. soru o binalarda da dosyadan cevaplanır.
