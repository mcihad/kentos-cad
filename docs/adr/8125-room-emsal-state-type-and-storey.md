# ADR 8125: Mahalin kendi emsal durumu ve emsal artışı (DİĞER), mahalin türü, doğru katta mahal

- **Durum:** uygulandı (18 Kasım 2026). Plan: [bim-plan.md](../../plan/bim-plan.md) B8 (BIM-08: "elle düzeltilir",
  mahalin türü, kot aralığı denetimi), [ifc-uyum.md](../../plan/ifc-uyum.md) IFC-03 (DIGER durumu).
- **Bağlam:** ADR [8030](8030-emsal-taks-zoning-badge-and-table.md) (emsal sınıflaması), [8069](8069-room-zone-far-model.md) (emsal
  zonları), [8079](8079-emsal-article-22-limits.md) (sınırlı kullanımlar), [8067](8067-building-3d-view-and-room-overlaps.md)
  (EK-9/13 örtüşme), [8124](8124-b7-remaining-declarations-and-ifc-fields.md) (B7).
- **`.kcad`:** mimari verisi 21 (`ARCH_WITH_ROOM_EMSAL`). **Sözleşme:** `SpaceFields.emsal` (`SpaceEmsal`),
  `EmsalCount::Bonus`, `is_parking`; `cad.spaces.edit`'in `set`'ine `emsal`; `kentos.auditFacts` 1'in CAD odalarına
  `floor`, `storeyFrom`, `storeyTo`. **Paket:** dijital-proje 0.13.0 (`kalite`'ye `quality_room_storey`).

## §1. Araştırma (18 Kasım 2026)

| Kaynak | Ne diyor | KentOS kararı |
|---|---|---|
| EK-4 4.14.2 | Emsal katmanlarında "DAHIL", "HARIC" ve "ilgili idarenin özel imar düzenlemelerine bağlı olarak, emsale esas alanın artış yapılabilen alanları" için "DIGER" | Mahalin kendi durumu "Emsal artışı (DİĞER)"; ne katlar alanında ne %30'da, kendi toplamı (K285) |
| EK-6 Tablo 6.60 (`TREpys_EmsalOzellikSeti.EmsalDurumu`) | DAHIL, HARIC, DIGER; "DIGER olması durumunda açıklama Attribute: Description bölümüne girilir" | DİĞER dayanaksız yazılmaz; zonun açıklaması dayanaklardır |
| BIM-08 (plan) | Emsal alanı mahal sınıflamasından türer, "elle düzeltilir" | Mahalin kendi durumu kullanımının ve adının okumasını geçer (K284) |
| EK-6 Tablo 6.58 (`IfcSpace`) | Ön tanımlı tipler SPACE, PARKING, GFA, INTERNAL, EXTERNAL | Otopark PARKING, dış mahal EXTERNAL, öbürü INTERNAL (K286) |
| buildingSMART IFC 4.3, `IfcSpaceTypeEnum` | PARKING: araç park yeri, erişim yolu dahil; INTERNAL ve EXTERNAL kullanımdan kalkıyor: yerine `Pset_SpaceCommon.IsExternal` | Tip EK-6 gibi yazılır; isteğe bağlı `IsExternal` yazılmaz |
| IfcOpenShell 0.9.0'ın IFC 4.3 şablonları, `Qto_SpatialZoneBaseQuantities` | Yalnız Length, Width, Height; alan yok | Zonların kat kat karşılaştırması dosyadaki gövdeden ölçülmeli: sonraki dilim (§4) |
| EK-9 Tablo 9.1 satır 13 (PDF s. 239) | "Mahaller diğer mahallerle örtüşmeyecek şekilde ve doğru katta modellendi mi?" | Döşemesi katının kot aralığında olmayan mahal bulgu (K287) |

## §2. Karar

- **K284 — Mahalin kendi emsal durumu:** `space.emsal` (sayılışı ve dayanağı) kullanımının (programın) ve adının
  okumasını geçer. Öznitelikler'de "Emsal durumu": Kendiliğinden (şimdiki sayılışıyla), Emsale dahil, Emsal harici (%30
  içinde), Emsal harici (%30'a sayılmaz), Emsal artışı (DİĞER); "Emsal dayanağı" yazılınca o anki sayılış dayanağıyla
  mahalin kendi durumu olur. Komut `cad.spaces.edit` `set` `emsal` (null kendiliğinden). Dayanak boş olamaz, en çok 200
  harf, satır sonsuz.
- **K285 — Emsal artışı (DİĞER):** idarenin düzenlemesinin emsale esas alana eklemeye izin verdiği alan. Yalnız mahalin
  kendi durumudur (programın kullanımları üç durumla kalır); dayanaksız yazılmaz. Emsal raporunda kendi toplamı
  (`bonus`, katta ve binada), tabloda "Emsal artışı (DİĞER), idarenin düzenlemesiyle" satırı; katlar alanına ve %30'a
  girmez. Modelde kat başına bir `DIGER` zonu, açıklaması "Emsal artışı: <dayanaklar>". Artışın kendisini hesaplamak
  idarenindir: KentOS alanı ve dayanağı söyler.
- **K286 — Mahalin türü:** adı ya da kullanımı Otopark, Garaj olan PARKING; programdaki kullanımı dış alan olan (balkon,
  teras, açık alan) EXTERNAL; öbürü INTERNAL. EK-6 türü ister; IFC 4.3'ün yerine önerdiği `Pset_SpaceCommon.IsExternal`
  isteğe bağlıdır ve yazılmaz (ilk yazımda her mahale yazılıyordu; ADR 8126 sırasında bütün IFC fikstürlerini anlamsız
  bir özellik için değiştireceği görülünce kaldırıldı).
- **K287 — Doğru katta (EK-9/13):** CAD olgularında mahalin döşemesi (modelin okuduğu: altındaki döşeme, kot farkı) ve
  katının kot aralığı (katın kotundan bir sonraki katın kotuna; en üst katta kotuna yüksekliği eklenerek). Kural
  (`kalite.quality_room_storey`) döşeme aralığın dışındaysa hata der; eşik yoktur (olgular milimetreye yuvarlı). Kademeli
  kat bilerek böyleyse istisna yapılır.
- **K288 — Eğik çatı altında kesilmiş mahal hacmi:** ADR [8071](8071-geometry-accuracy.md) §11 ve
  [8123](8123-far-zone-under-the-roof.md) ile yapılmıştı; BIM-08'in listesinden düşer.

## §3. Uygulama

- **Sözleşme:** `arch.rs` (`EmsalCount::{Bonus, ROOM}`, `SpaceEmsal`, `is_parking`, `folded_words`), `space.rs`
  (`emsal`); `cad_spaces.rs` (`SpaceChange.emsal`).
- **Dosya:** `kentos_kcad::{ARCH_WITH_ROOM_EMSAL, uses_room_emsal}`, `space.rs`, `decode.rs`.
- **Alan:** `zoning.rs` (kendi durum önce; `bonus` raporda, katta, tabloda; `ZoningRoom.basis`).
- **Uygulama:** `spaces_edit` (doğrulama, yazım), `arch_ifc::emsal_areas` (DİĞER zonu ve dayanakları).
- **IFC:** `kentos_ifc::arch` (`EmsalArea.bases`, `emsal_state` DIGER; mahalin tipi ve `IsExternal`).
- **Olgular:** `kentos_processing::audit_facts::cad` (`floor`, `storey_from`, `storey_to`; `storey_range`).
- **Masaüstü:** Öznitelikler (`rows/space.rs`: Emsal durumu, Emsal dayanağı; `Field::SpaceEmsalBasis`).
- **Paket:** dijital-proje 0.13.0 `kalite.py`.

## §4. Sınamalar ve kalanlar

- **Komut durumları** (`fixtures/commands/v1/cad.spaces.edit.json`, 43 → 47; `scripts/fixtures/space_command_cases.py`):
  harici dayanağıyla, DİĞER dayanağıyla, null, dayanaksız DİĞER reddi.
- **Dosya:** `a_room_s_own_emsal_state_round_trips_under_arch_data_21`.
- **Uygulama:** `a_room_s_own_emsal_state_counts_and_an_increase_is_a_diger_zone` (iki mahal: biri harici, biri
  DİĞER; rapor ve tablo; DIGER zonu açıklamasıyla; her mahalde tip ve `IsExternal`).
- **Paket:** `test_kalite.py` (`DogruKat`, 3). B7'den kalan: `test_form.py` künyenin soyadı kutusunu bekler (ADR 8124
  K279; o dilimde koşturulmamıştı).
- **Masaüstü:** `room_emsal_tests` (dayanaksız DİĞER reddi, dayanak yazılınca durum, DİĞER, Kendiliğinden).
- **Kalan (BIM-08):** emsal zonlarının teslim edilen dosyada kat kat ve durum durum emsal tablosuyla karşılaştırılması
  (EK-9/10, "katlar arasında yer değiştiren alan"): okuyucu zonu başvurduğu kata bağlamıyor (yalnız yetim saymıyor),
  zonun alanı dosyada standart bir miktar değil (yalnız boyutlar); gövdenin izinden ölçülecek, ikinci okuyucu da aynısını
  yapacak, beklenen çizimin tablosundan gelecek. Ayrıca mevzuat paketinin emsal kuralları DİĞER'i henüz okumuyor.
  Gerçek pencerede kabul yapılmadı.
