# ADR 1008: Eşit olmayan ölçekli bloğun Patlat'ında yayların ve genişliklerin kesin kalması

- **Durum:** kabul edildi (2026-11-06). Kapsam: plan/cad-advanced.md `CAD-ADV-17` açığı “KentOS içi Patlat'ta eşit
  olmayan yay” (ADR 1061 §8).
- **Bağlam belgesi:** ADR 1061 §3 (DXF/DWG okuyucusunda eşit olmayan blok: kesin NURBS ve kesin genişlik), ADR 0315
  §3 (eşit olmayan yerleştirme), ADR 0302 (Patlat).

## 1. Kaynak

Piegl & Tiller, *The NURBS Book* §7.3–7.4: konik yay rasyonel ikinci derece Bézier'dir; afin dönüşüm altında denetim
noktaları dönüşür, ağırlıklar kalır. Autodesk'in [EXPLODE yardımı](https://help.autodesk.com/cloudhelp/2023/ENU/AutoCAD-LT/files/GUID-E98BCEF4-DED6-48A6-87EB-10FE87188083.htm)
eşit olmayan ölçekli bloğun patlatılmasında dairelerin elips, yayların eliptik yay olduğunu söyler; yaylı çoklu
çizgi için ayrı bir kural yazmaz, KentOS'un NURBS seçimi kendi kararıdır; KentOS'ta daire ve yay bu kuralla zaten kesin elipsti
(`warp_shape`). Yaylı çoklu çizgi ise 0,1 mm içinde düz köşelere bölünüyordu ve genişlikler kenar dönmeden
kopyalanıyordu.

## 2. Karar

- ADR 1061 §3'ün iki kuralı DXF okuyucusundan ortak çekirdeğe taşındı:
  `kentos_geometry_core::geom::affine_path::{arcs_as_nurbs, exact_widths}`. Okuyucu artık bunları çağırır (bit
  eşitliği için çekirdek modül `libm::hypot` kullanır; okuyucunun testleri değişmeden geçti).
- `Blocks::explode`, benzerlik olmayan yerleştirmede her parçayı önce `exact_path` ile dener:
  - açık, tek parçalı, deliksiz, genişliksiz ve kotsuz yaylı çoklu çizgi → ikinci derece ağırlıklı NURBS
    (`closed: false`, çift iç düğüm); örnekleme yok;
  - yaysız, genişlikli, tek kotlu (ya da kotsuz) çoklu çizgi → köşeleri dönüşür, her kenarın genişliği
    |det M| / |M·t| ile çarpılır (kapalısı da);
  - başka her şey (kapalı/çok parçalı yaylı yol, genişlikli yaylı yol, kotlu yaylı yol) eski `warp_shape` yolunda
    kalır.
- Yerleştirmenin kendisinin çizimi (patlatılmadan) değişmedi; kural yalnız Patlat'ın yazdığı parçalardadır.

## 3. Kanıt

- Ortak vaka `fixtures/commands/v1/cad.entities.explode.json` (25 vaka): “Eğri yol” bloğu 3 × 0,5 ölçekle
  patlatılır. Beklenen NURBS `scripts/fixtures/explode_command_cases.py`'da yayın merkezi ve açılarından hesaplanır
  (her çeyreğin orta denetim noktası merkez + r / cos(φ/2) yönünde, ağırlık cos(φ/2)); KentOS'un çıkıntı formülleri
  kullanılmaz. Genişlikler 0,2 → 0,1 (x kenarı) ve 0,4/0,2 → 1,2/0,6 (y kenarı). ±1e-12 içinde eşleşir; geri alma
  özgün yerleştirmeyi döndürür.
- `kentos-geometry-core` (9 test ikilisi), `kentos-formats` emit birim ve `cad_` entegrasyon testleri ile Patlat,
  blok ve düzenleme ortak vakaları geçti.
- Resimler: `.run/shots/arac-patlat-esitsiz-*.png` (1 × 1 ve 1 × 2,5 yerleştirme seçili) ve
  `arac-patlat-esitsiz-yazildi-*.png` (Patlat sonrası parçalar seçili): eğri aynı yerde kalır, NURBS denetim
  noktaları elipsin teğet köşelerindedir, yatay kenarın genişliği 2,5 katına çıkar, düşeyinki aynı kalır.

## 4. Sınırlar

- Çoklu çizginin köşe düzenlemesi yerine NURBS düzenlemesi gelir. Ek (6 Kasım 2026): Patlat bunu
  `semantics_lost` uyarısıyla sayarak söyler (“N yaylı çoklu çizgi eşit olmayan ölçek altında kesin NURBS oldu …”);
  sayı `Blocks::explode_counted`'dan gelir, ortak vaka uyarıyı bekler.
- Kapalı yaylı yol, genişlikli yaylı yol ve kotlu yaylı yol hâlâ örneklenir.
- AutoCAD ile karşılaştırma yapılmadı (CAD-IO-20).
