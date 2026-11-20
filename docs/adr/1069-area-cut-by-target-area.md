# 1069 — İfraz: hedef alanı ayıran, kenara paralel kesim

Tarih: 2026-11-20. usage-reporting-2.md UR2-052. Durum: ilk aşama.

İlgili: [ADR 0191](0191-parallel-shift.md) (Paralel kaydır, hedef alan), [ADR 0065](0065-desktop-area-tools.md) (Alan böl),
[ADR 4220](4220-parcel-register-documents-and-boundaries.md) ve [ADR 4620](4620-subdivision-splits-neighbour-boundary.md)
(parsel kaydı ve ifraz belgesi).

## Dayanak

- **Civil 3D**, Parcel Layout Tools › Slide Line – Create
  ([To Create Parcels by Subdividing](https://help.autodesk.com/cloudhelp/2015/ENU/Civil3D-UserGuide/files/GUID-D9ABB3A9-25E1-4C60-A092-005ECFC6FBBE.htm),
  [Exercise 3: Subdividing a Parcel with a Slide Line](https://help.autodesk.com/cloudhelp/2017/ENU/Civil3D-Tutorials/files/GUID-35AC8372-274B-4EC0-927E-29B2B8C9D614.htm),
  20 Kasım 2026'da okundu): bölünecek parsel ve cephe seçilir, kesim doğrultusu açı ya da iki noktayla verilir, hedef
  alan yazılır; kesim cephe boyunca kayarak hedef alanı ayırır, sonuç kabulden önce gösterilir.
- **KentOS bugün:** web kataloğunda `tool.subdivide` (IFRAZ) “Geliştirme aşamasında” idi. Alan böl kesimi elle alır;
  Paralel kaydır hedef alana kenarı kaydırarak ulaşır ama parseli bölmez.

## Karar

1. **Çekirdek** `kentos_geometry_core::ops::ifraz::cut_for_area(ring, edge, target)`:
   - parselin dış halkası (iki yönde de, kapanış tekrarı atılır) ve referans kenarı (köşe `edge` → sonraki);
   - kesim, kenara paralel, alanın içine doğru `d` uzaklıktaki doğrudur; ayrılan parça halkanın bu doğrunun kenar
     tarafındaki kısmıdır, alanı `d` ile artar;
   - hesap kenarın çerçevesinde yapılır (s kenar boyunca, h kenara dik): büyük harita koordinatı küçük farkı yutmaz;
   - iki köşe yüksekliği arasında kesim uzunluğu doğrusaldır, alan ikinci derecedendir; hedefin düştüğü aralıkta kapalı
     biçimde çözülür (`2c / (w₀ + √(w₀² + 2kc))`);
   - kesim sınırı tam iki yerde keser; daha çoğu ikiden çok parça demektir ve reddedilir;
   - ret: üçten az köşe ya da alansız halka, olmayan kenar, uzunluğu olmayan kenar, sıfırdan büyük ve parsel alanından
     küçük olmayan hedef.
2. **Bağımsız başvuru** `scripts/fixtures/ifraz_cases.py` → `fixtures/ifraz/v1/cases.json` (29 durum): KentOS kodu
   olmadan, mpmath 50 basamakla, kırpılmış alan üzerinde 200 ikiye bölmeyle. Kare, saat yönü, kapanış tekrarı,
   dikdörtgen, yamuk (iki yönden), üçgen, köşe aşan beşgen, eğik dörtgen, kenarda ara köşe, büyük koordinat, çok küçük
   ve toplama çok yakın hedef, içbükey L ve U (iki ve dört kesişim) ile bütün retler. Çekirdek testi
   `crates/shared/geometry-core/tests/all/ifraz.rs`: koordinatta 1 nm ya da büyük koordinatın 2 ulp'i, alanda parsel
   alanının 1e-9'u.
3. **Araç** `tool.subdivide` (IFRAZ, `crates/native/interaction/src/map/ifraz.rs`):
   - referans kenara tıklanır (kilitsiz alan); ayrılacak alan projenin alan biriminde yazılır;
   - Karşı taraf (T) hedefi kenarın karşısındaki parçaya verir;
   - önizleme: parsel, referans kenar, kesim, istenen parça dolu, kalan soluk; etikette istenen, kalan ve kenara uzaklık;
   - Enter kesimi Alan böl'ün komutuyla (`cad.areas.edit`, `AreaWork::Split`) tek geri alma adımında yazar. Kesim
     sınırın iki ucundan 1 cm uzatılır: komut sınırla kesişimi kendisi bulur, köşeler kesim doğrusundadır;
   - yazmadan önce aynı bölme önizlemede yapılır; iki parça çıkmazsa söylenir, yazılmaz.
   Yeni ürün komutu yok: yazma mevcut `cad.areas.edit` ile olur (Python ve MCP onu sarar).
4. **Masaüstü:** Plan sekmesi › Ada ve parsel (Harita CAD). `ported.json`, `equivalents.json` (kısmen).

## Sonra

- Delikli ya da yaylı parsel, çok parçalı alan.
- Doğrultuyla (açı, iki nokta) ya da bir noktadan geçen kesimle bölme; çok parselli dağıtım ve artık kuralı (§23.2).
- Parsel kaydına ifraz belgesi olarak işleme (ADR 4220): bugün parçalar geometridir, numara ve hisse ayrıca işlenir.
