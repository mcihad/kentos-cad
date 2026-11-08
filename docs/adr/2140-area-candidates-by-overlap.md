# 2140 — Alan adaylarının örtüşmeyle uzlaştırılması (`map.features.reconcile`)

Tarih: 2026-11-08. plan/map-cad.md MAP-04, üçüncü tur. Durum: taslak. Plan satırları:

- “Yol kenarı, bordür, bina izi … gibi belediye varlıkları”;
- “Aynı varlık birden çok kaynakta görüldüğünde eşleme kimliği ve uyuşmazlık kaydı olsun.”

İlgili:

- [ADR 1760](1760-asset-candidate-reconciliation.md): iki kaynaktan aday uzlaştırma. Bu ADR onun “Sonra”sındaki
  “alan sınıflarının (bina izi) örtüşmeyle uzlaştırılması” maddesini kapatır;
- [ADR 1360](1360-field-codes-and-kodla-ciz.md): Harita varlıkları profili. Alan sınıfları bina izi (BNA) ve yeşil alan
  (YSL).

## Dayanak

- **PASCAL VOC** (Everingham, Van Gool, Williams, Winn, Zisserman, “The PASCAL Visual Object Classes (VOC) Challenge”,
  IJCV 88(2), 2010, §4.4, formül (3); PDF'ten okundu, 8 Kasım 2026):
  - doğru tespit için öngörülen ile gerçek kutunun örtüşme oranı aₒ = alan(Bp ∩ Bgt) / alan(Bp ∪ Bgt) “0,5'i
    aşmalı”;
  - yazarlara göre eşik, gerçek verideki belirsizlik için bilerek düşük tutuldu.
- **Esri Compute Accuracy For Object Detection**
  ([ArcGIS Pro](https://doc.esri.com/en/arcgis-pro/latest/tool-reference/image-analyst/compute-accuracy-for-object-detection.html)):
  - IoU, öngörülen nesnenin çevresindeki kutu ile başvuru kutusunun örtüşmesidir;
  - doğru ya da yanlış pozitifin eşiğidir, 0 ile 1 arasındadır;
  - sayfa öntanımlı değer vermiyor, örnekleri 0,5 kullanıyor.
- **Esri Detect Feature Changes** (ADR 1760): çizgiler için arama mesafesi. Alanların örtüşmeyle eşlenmesinin
  dayanağı yukarıdakilerdir.
- **KentOS bugün** (ADR 1760): nokta ve açık çoklu çizgi adayları yarıçapla eşlenir. Alan sınıfı adayı `wrongKind`
  sayılır, bina izleri uzlaştırılamaz.

## Karar

1. **Alan adayı:** bir kaynağın katmanındaki kapalı alandır: en az üç köşe, alanı 0'dan büyük. Alan sınıfının adayı
   alandır. Alan sınıfı kodlu nokta ya da nokta sınıfı kodlu alan `wrongKind`'dır.
2. **Örtüşme oranı:** iki alanın kesişiminin alanı bölü birleşiminin alanıdır. KentOS bunu çekirdeğin alan cebiriyle
   hesaplar (`intersect_area_sets`, `net_area`); kutuları ayrı iki alanın oranı 0'dır. İki alan adayı, oranları
   `overlap`'ı aştığında bir gruptur, zincirle; yarıçap alanlara uygulanmaz.

   `overlap` 0 ile 1 arasında olmalıdır, uçlar hariç (`invalid_input`). Verilmezse PASCAL VOC'un ölçütü olan 0,5'tir.
3. **Grup:**
   - Temsilci, yineleme ve durumlar (uyuşan, tek, çelişen, var olan) ADR 1760'taki gibidir.
   - Alan grubunun izi en güvenilir temsilcininkidir (eşitlikte önceki kaynak), çizgi grubununki gibi.
   - Yeri o izin dış halkasının ağırlık merkezidir; çizimin koordinatları büyük olduğu için ilk köşeden hesaplanır.
   - Yayılım, temsilcilerin kapalı dış halkaları arasındaki en büyük Hausdorff'tur (m).
   - `overlap`, temsilciler arasındaki en küçük orandır (tek temsilcide yok).
   - **Var olan:** sınıfın katmanındaki aday olmayan bir alan bir temsilciyle oranı aşacak kadar örtüşüyorsa grup
     `existing` olur.
4. **Yazma:** uyuşan alan grubu sınıfın katmanına en güvenilir izle yazılır. Köken özniteliklerine Kaynaklar, Adaylar
   ve Yayılım'ın yanında **Örtüşme** (üç basamak) eklenir; adayınki kopyalanmaz. Tek geri alma adımıdır.
5. **Masaüstü** (Adayları uzlaştır, yalnız Harita):
   - formda **Yarıçap, örtüşme** satırı (nokta ve çizgi yarıçapla, alan örtüşmeyle) ve ayrı **Gözden geçiren** satırı;
   - tabloda **Örtüşme** sütunu.
6. **Kapsam işareti:** ADR 1760'taki gibi **Harita** (pencere ve yazma); komut Python ve MCP'de.

## Kanıt

- **Ortak durumlar:** `scripts/fixtures/map_features_reconcile_cases.py`; alanlar shapely (GEOS) ile, KentOS kodu yok.
  30 durum (25'ten). Yeni durumlar bina izleri ve yeşil alanlardır:
  - iki kaynakta 0,87 örtüşen bina (yineleneniyle), 0,25 örtüşen iki iz (ayrı tekler);
  - var olan binanın üstündeki aday, yeşil alan/bina çelişkisi (0,84), nokta çizilmiş bina, alan çizilmiş direk;
  - uzlaştırma ve tek adımda geri alma; 0,2 eşikle birleşen iki iz;
  - 0 ve 1 eşikleri için ret.

  Rust ve Python geçer.
- **MCP** (`crates/native/mcp/tests/reconcile_areas.rs`): durumun çizimi `.kcad` olarak açılır. Planın grupları,
  oranları, ağırlık merkezleri ve yayılımı durumunkilerle 10⁻⁹ içinde aynıdır; 1 eşiği durumun iletisiyle reddedilir.
- **Masaüstü** (`features_reconcile_areas_tests.rs`):
  - plan durumunkidir, oran tabloda;
  - 0,2 eşik iki izi birleştirir;
  - Uzlaştır en güvenilir izi Örtüşme ile Bina izleri'ne yazar; tek geri alma.

  Çapraz masaüstü denetimleri 135 geçer. Resimler `.run/gui-evidence/aday-alanlar-2140/`.

## Sonra

- Yaylı alan ve çizgi adaylarının yay uzaklığı; delikli alanın Hausdorff'u (bugün dış halka).
- Büyük aday kümesinde ızgara dizini; inceleme kaydının ayrı tutulması; pencerede eksik alan girişi (ADR 1760).
- Çıkarımın kendisi (AI modeli, MAP-04'ün kalanı).
