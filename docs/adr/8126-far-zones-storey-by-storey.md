# ADR 8126: Emsal zonları dosyada kat kat ve durum durum emsal tablosuyla (EK-9/10)

- **Durum:** uygulandı (18 Kasım 2026). Plan: [bim-plan.md](../../plan/bim-plan.md) BIM-08'in son kalanı ("katlar arasında
  yer değiştiren alanın denetimi, teslim edilen dosyada"), [ifc-uyum.md](../../plan/ifc-uyum.md) IFC-03, IFC-08.
- **Bağlam:** ADR [8069](8069-room-zone-far-model.md) (emsal zonları), [8074](8074-ek9-from-the-written-file.md) (form
  yazılmış dosyadan), [8116](8116-ek9-from-the-delivered-model.md) (form teslim edilen modelden),
  [8125](8125-room-emsal-state-type-and-storey.md) (DİĞER).
- **Dosya, komut:** değişmedi. **Olgular:** `kentos.ifcFacts` 1'in `bodies`'ine `zones`, dosya olgularına `emsal`
  (çizimin tablosu). **Paket:** dijital-proje 0.13.0'ın `form.s10`'u.

## §1. Araştırma

| Kaynak | Ne diyor | KentOS kararı |
|---|---|---|
| EK-9 Tablo 9.1 satır 10 (PDF s. 239) | "Mahal ve Mekansal Zon alanları emsal hesap tablolarıyla uyumlu mu?" | Kat kat ve durum durum karşılaştırma; genel toplam yetmez (BIM-08: "dahil, hariç ve diğer alanlar birbirini telafi edemez") |
| IfcOpenShell 0.9.0'ın IFC 4.3 şablonları, `Qto_SpatialZoneBaseQuantities` | Yalnız Length, Width, Height | Zonun alanı dosyada miktar değil: gövdesinin izinden ölçülür (K290) |
| IFC 4.3 `IfcRelReferencedInSpatialStructure` | Bir yapının bir zonu içermeden başvurması | KentOS zonları katlarına böyle bağlar; okuyucu bu bağı tutar (K289) |
| ADR 8074 §2 (gövde ölçüleri) | Gövdeler motorun float32 üçgenleridir; her gövdenin float32 sınırı olgudadır | Alanın payı çevresinin iki katı çarpı sınır (K290) |

## §2. Karar

- **K289 — Zonun katı:** okuyucu `IfcRelReferencedInSpatialStructure`'ın yapısını nesnede tutar (`reference`, ilki);
  ağaç değişmez. Bir zonun katı parçası olduğu, içerildiği, yoksa başvurulduğu yapıdan bulunur.
- **K290 — Zonun alanı:** gövdesinin en alt yüzündeki üçgenlerin birleşiminin alanı; payı izin çevresinin iki katı çarpı
  gövdenin float32 sınırı (`bodies.zones`: GlobalId, kat, alan, pay).
- **K291 — Beklenen:** her emsal zonu kendi katının o durumda sayılan mahallerinin tablodaki alanını taşır (mahallerin
  dış çizgilerinin alanı; dosyaya yazılmaz). Dosya olgularına `emsal` (kat GlobalId'si, durum, alan) olarak eklenir:
  çizimin şimdiki tablosu, teslim edilen model okunurken de (ADR 8116'nın fark satırıyla).
- **K292 — Formun 10. sorusu:** beklenen ve gövdeler varsa her kat ve durum için dosyadaki zonların alanı tablonunkiyle
  payı içinde aynı mı: değilse kalır, her farklı kat ve durum nesneleriyle bulgu (`digital_emsal_zone_area`: "Zemin kat,
  DAHIL: dosyadaki zonlar 90,00 m², emsal tablosunda 100,00 m²"); hepsi aynıysa geçer. Beklenen yoksa (başkasının
  dosyası, idarenin paket incelemesi) eskisi gibi incelemedir.

## §3. Uygulama

- `crates/shared/ifc/src/read/{mod.rs, graph.rs}` (`reference`), `facts_bodies.rs` (`zones`, `ring_length`).
- `crates/shared/ifc/src/{building.rs (Zone.table), arch.rs (EmsalArea.table)}`;
  `crates/native/application/src/arch/{arch_ifc.rs (emsal_areas), ifc_facts.rs (emsal)}`.
- `catalog/standards/dijital-proje/src/dijital_proje/form.py` (`s10`).

## §4. Sınamalar ve kalanlar

- **Paket** (`test_form.py`, `EmsalZonlari`, 5): tablo yoksa inceleme; kat kat aynı geçer; genel toplam aynıyken zemin
  kattan birinci kata 10 m² geçmiş alan iki bulguyla kalır; yanlış durum (DAHIL yerine HARIC) kalır; payın içindeki fark
  aynı sayılır.
- **Okuyucu** (`every_emsal_zone_has_its_storey_and_its_footprint`): kesit binasının dosyasında her zonun katı ve alanı,
  her katta zon. Olguların başvuruları (`fixtures/ifc/v1/olgular/*.json`, `KENTOS_WRITE_IFC_FACTS=1`) yeniden yazıldı:
  fark yalnız `bodies.zones` (kesit binasında katta 71,40 m² dahil ve 24,51 m² merdiven holü).
- **Masaüstü** (`the_form_s_10th_row_holds_the_zones_to_the_table_storey_by_storey`): kesit binasının şimdiki modelinde
  10. satır "Evet", "emsal tablosuyla aynı".
- **Değişen beklentiler:** BIM paketinde müellife kalan sorular 1, 7 ve 10'dan 1 ve 7'ye indi
  (`bim_permit_tests`, `kentos-headless` `bim_package`: cevaplanmamış soru 3'ten 2'ye; bayat cevap örneği 10'dan 7'ye).
  B7 ve B8'in önceki dilimlerinin koşturulmamış sınamaları da burada düzeldi: çizimdeki formun künye satırı mimarın
  soyadı kutusunu sayar (`quality_form_tests`; ad ve soyadın kutusu tek: "Mimarın soyadı (IFC modelinde)"); mahallere
  yazılan `IsExternal` bayt bayt karşılaştırılan fikstürleri bozuyordu, kaldırıldı (ADR 8125 K286'nın notu); Proje
  ayarları sınaması formun metinlerini pencere gibi projeden başlatır.
- **Kalan:** ikinci okuyucu (IfcOpenShell, `scripts/fixtures/ifc_form_reference.py`) zonların izini ölçmüyor; başvuru
  dosyalarında beklenen olmadığından iki okuyucu 10. soruda yine aynı (inceleme). Mevzuat paketinin emsal kuralları
  DİĞER'i okumuyor.
