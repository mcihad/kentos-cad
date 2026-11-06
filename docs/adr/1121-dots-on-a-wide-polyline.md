# ADR 1121: Geniş çoklu çizgide nokta öğesi

- **Durum:** kabul edildi (2026-11-06). Kapsam: plan/cad-advanced.md `CAD-ADV-11` açığı “noktalı bant”: sıfır uzunluklu
  tire (nokta) içeren desen geometrik genişlikli çoklu çizgide şimdiye kadar `linetype_unsupported` ile
  reddediliyordu (ADR 0930 “Geniş çoklu çizginin gerçek desenli bandı”).
- **Bağlam belgesi:** ADR 0930 (`style-core/style/cad_band.rs`), ADR 1122 (eğride dash fazı).

## 1. Araştırma

- Autodesk [PLINEGEN](https://help.autodesk.com/cloudhelp/2020/ENU/AutoCAD-Core/files/GUID-352844E4-FBEA-44E0-B8F3-41C4187AD9C7.htm):
  0 desenin her tepe noktasında tireyle başlayıp bittiğini, 1 kesintisiz sürdüğünü, değişkenin daraltılmış
  (tapered) kenarlı çoklu çizgiye uygulanmadığını söyler. Noktanın geniş çizgide nasıl çizildiği yazılı değildir.
- Autodesk'in [polyline](https://help.autodesk.com/cloudhelp/2025/ENU/AutoCAD-Core/files/GUID-392BF13C-D9E7-47A8-8E07-435296332279.htm)
  yardımı geometrik genişliği çizgi kalınlığından ayırır; nokta için kural vermez. Autodesk destek makalesi
  [Dash line in Polyline is not even](https://www.autodesk.com/support/technical/article/caas/sfdcarticles/sfdcarticles/Dash-line-in-Polyline-apprear-not-evenly.html)
  yalnız PLINEGEN'i anlatır. CAD Forum ipucu noktalı desenlerin tireyle başlaması gerektiğini söyler, genişlik
  için bir şey söylemez.
- Bir açık kaynak 2B CAD projesinin değişikliği (triya-cad-2d PR #127) noktayı genişlik boyunca kesit olarak
  çizer ve bunun AutoCAD'e karşı doğrulanmadığını kendisi yazar. AutoCAD çıktısı elde değil; değer uydurulmadı.

Sonuç: AutoCAD'in davranışı belgelenmemiş. Aşağıdaki kural KentOS kararıdır ve AutoCAD eşdeğerliği iddia etmez.

## 2. Karar

- Geniş bant, dolu bir şeklin desen aralıklarına bölünmesidir; sıfır uzunluklu mürekkep aralığının limiti bandın o
  noktadaki **kesitidir**: noktanın yerinde, yolun teğetine dik, o noktadaki genişlik boyunda bir çizgi. Kesit
  nesnenin çizgi kalınlığıyla (dash'sız) çizilir; bandın genişliksiz kenarlarının ince çizgisiyle aynı stil.
- Noktaların yeri desenden analitik hesaplanır (ADR 0930'un kesintisiz faz kuralı): yay kenarında gerçek yay
  uzunluğu ve teğet, genişlik özgün kenarın parametresinden; tepe noktasına düşen nokta çıkan kenarın teğetini ve
  genişliğini alır (işaret yerleştiricisiyle aynı); kapalı yolun dikişinde tek nokta; açık yolun son noktası
  ince çizginin `dash_pieces` kuralı gibi sayılır.
- Genişliğin **sıfır** olduğu yere düşen nokta için kesit yoktur: çıktı açık `linetype_unsupported` ile reddedilir
  (“Nokta öğesi geniş çizginin sıfır genişlikli bir yerine düşüyor…”), boşluk olarak geçilmez.
- Kesitler 2 nokta olarak mevcut bant iş yükü bütçesine sayılır (20 000 geçiş, 500 000 nokta); sınır aşımı
  eskisi gibi açık bulgu. Ekran (GPU), CPU önizlemesi ve PDF aynı `Sink::stroke` kesitlerini alır.

## 3. Kanıt

- `style-core` `cad_band`: 8/8 (4 yeni: uçlar ve faz, tire/boşluk/nokta karışımı, tepe noktası ve kapalı dikiş,
  yayda gerçek yay uzunluğu ve daralan genişlik, saat yönü, sıfır genişlikte ret).
- `kentos-plot` `curve_dashes::a_dotted_wide_polyline_prints_its_cross_sections`: 1:1/1:20/1:100'de 20 mm'lik bantta
  2 mm'de bir 11 kesit, x ve ±w/2 1e-12 içinde; PDF üretildi; sıfıra daralan bantta `linetype_unsupported`.
  `line_types::geometric_width_patterns_are_inspected_before_pdf_delivery`'nin eski “dot engellenir” beklentisi bu
  kural değişikliğiyle “dot çizilir, dot_zero engellenir” oldu (bilinçli kural değişikliği).
- Resimler (repoya alınmaz): `.run/shots/arac-noktali-bant-1440x900.png` (GPU; 2 m'de bir kesit 69,8 px,
  34,9 px/m; kesit 52 px = 1,49 m, bant 1,5 m), `.run/gui-evidence/adr1122-egri-1-{20,100}-pdf-1.png` ve
  `-cpu.png` (PDF'in Poppler resmi ve CPU önizlemesi).

## 4. Sınırlar

- Kesitin AutoCAD çıktısıyla karşılaştırılması yapılmadı. PLINEGEN=0 (her tepe noktasında yeniden başlayan desen)
  KentOS'ta yok; kesintisiz kural geçerli.
- Sıfır genişlikli noktanın bir “nokta” olarak çizilmesi ayrı karar ister; şimdilik açık ret.
