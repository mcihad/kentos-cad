# ADR 0841 — Tam elipsin bağlı noktasını eksen düzenlemesinde koru

2026-11-02. CAD-ADV-05 kapsamında sınırlı doğruluk düzeltmesi.

## Sorun ve araştırma

`geometry-core/src/ops/grips.rs` tam elipsin dört eksen tutamacında
`ellipse_from_center` sonucunun sıfır başlangıç/bitiş açısını yazıyordu.
İçe alınmış tam elipsin `t0=t1=0.6` olduğu durumda ekseni 4→6 m uzatmak,
Start'a bağlı çizgiyi `(6 cos(0.6), 2 sin(0.6))` yerine `(6, 0)` konumuna
sıçratıyordu. Çözücü iki durumda da bağı tutuyor diyebildiği için yalnız
yeşil kısıt işaretlerine bakmak bu geometri kaybını yakalamaz.

[AutoCAD elips açı tanımı](https://help.autodesk.com/cloudhelp/2026/ENU/OARX-ManagedRefGuide/files/OARX-ManagedRefGuide-Autodesk_AutoCAD_Geometry_EllipticalArc3d_Set_Point3d_Vector3d_Vector3d_double_double_double_double.html)
parametrik açıyı elipsin eksenlerine göre tanımlar. KentOS çekirdeğinin
`ellipse_point` fonksiyonu da bu yerel koordinat tanımını kullanır.
[ConstrainedEllipse MinorRadius](https://help.autodesk.com/cloudhelp/2026/ENU/OARX-ManagedRefGuide/files/OARX-ManagedRefGuide-Autodesk_AutoCAD_DatabaseServices_ConstrainedEllipse_MinorRadius.html)
çözücü eksen kimliği ile geometrinin major/minor normal biçiminin aynı
şey olmadığına ayrıca örnektir. KentOS kendi çekirdek cebirini kullanır;
yabancı çözücü kodu veya yeni bağımlılık eklenmedi.

## Karar

- Eksenler aynı sırada kaldığında kaynak `t0/t1` korunur. Tam süpürmenin
  eşit açılarla veya 2π farkıyla saklanması değiştirilmez.
- Diğer yarıçap verilen eksenden uzunsa mevcut kurucu eksen bazını +90°
  döndürür. Aynı fiziksel noktayı tutmak için iki parametreden π/2 çıkarılır:
  yeni yerel `(b v, -a u)` bazı, `t−π/2` ile eski `a u cos(t)+b v sin(t)`
  noktasını verir. Karşılaştırma kurucuya verilen gerçek dünya ekseninden
  yapılır; özellikle negatif tutamaçta büyük koordinat yuvarlaması dikkate alınır.
- Kısmi elipsin Start/End kaydırması ve merkez taşıma yolu değişmez.
  Metre tabanlı f64 geometri, mevcut tolerans ve KCAD şeması korunur.
- Web/WASM ve masaüstü aynı `move_grip` çekirdeğini kullanır. Python/MCP
  ortak `cad.entities.edit operation=grip` ile aynı hedef geometriyi uygular;
  ayrı bir çözüm yolu veya ekran pikselinden geometri hesabı yoktur.

## Doğrulama

- Yeni regresyonlar düzeltme öncesinde yanlış noktayla başarısız oldu.
  Dört tutamaç, üç eksen uzunluğu, üç dönme açısı, iki dünya orijini,
  iki tam-süpürme gösterimi; sıfır yer değiştirme, yeniden yöneltme ve
  sıfır eksen reddi çekirdek testinde sınanır.
- Native komut testi dört tutamaç ve eksen sırası değişiminde bağlı çizgiyi,
  planın salt okunurluğunu, UID/öznitelikleri, iki geçerli bağı, beş
  serbestliği, dosyayı yeniden açmayı ve tek Undo'yu denetler.
- Python hedefi bağımsız eksen cebiriyle kurar; MCP eksen sırası değişimini
  plan/execute/query/save/Undo üzerinden sınar.

- Temiz çekirdek: 593 lib, 314 integration ve iki API çağrı testi geçti
  (mevcut 6+21 ignored). Native application 314, MCP 47, Python mevcut
  kısıt dosyalarıyla birlikte 15 test geçti. All-targets çekirdek/application/
  MCP/masaüstü Clippy ve masaüstü derlemesi geçti. Masaüstü kütüphanesinin
  1582 testi geçti (mevcut 256 ignored).
- Gerçek Mac arayüzünde büyük koordinatlı elipste `@2,0` ile 4→6 m,
  `@-2.5,0` ile 4→1.5 m eksen değişimi tek Enter ile uygulandı. İlkinde
  dikiş 0.6 kaldı; ikincisinde −0.9707963267948966 oldu. Bağlı çizginin
  bağımsız formülle beklenen dünya noktasına farkı iki durumda da 0 m.
  İki kalıcı bağ ve beş serbestlik korundu.
- Her iki arayüz işleminin tek Undo kaydı başlangıç dosyasıyla baytları
  tam aynı; GUI hedefini aynı başlangıca uygulayan Python dosyası GUI ile
  tam aynı. Daraltılmış dosya gerçek uygulamada yeniden açıldı; doğru
  geometri, iki yeşil bağ ve beş serbestlik tekrar görüldü.
- Ekranlar `132-full-ellipse-extended-seam-retained.png` –
  `135-full-ellipse-reopened-geometry.png`; sayısal kanıt
  `ellipse-seam-gui-acceptance.json`. Bunlar yalnız
  `/tmp/kentos-cad-adv04-2026-11-01/` altında, repo dışında tutulur.

Tam elipsin bağımsız kalıcı eksen başvuruları ve birleşik CAD-ADV-05
teknik plaka kabulü bu düzeltmeyle tamamlanmış sayılmaz.
