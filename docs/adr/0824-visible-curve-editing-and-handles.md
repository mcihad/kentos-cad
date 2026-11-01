# 0824 — Eğri düzenlemede görünür işlemler ve belgeyi izleyen noktalar

Tarih: 2026-11-01. Durum: masaüstü dilimi uygulandı; CAD-ADV-04 birleşik kabulü açık.

## Karar

Mevcut `cad.curves.edit` komutu yeniden yazılmaz. Etkin aracın ayarları
seçilen eğrinin türünü, derecesini, geçiş/denetim noktası sayılarını ve
uygun işlemleri gösterir. Nokta görünümü yalnız görünümü değiştirir;
geçişten denetime bakmak geometriyi veya Undo geçmişini değiştirmez.
Geçiş noktaları halka, denetim noktaları kare ve kesikli denetim ağıdır.
Görünen noktaya doğrudan tıklamak taşıma adımını başlatır; gizli denetim
noktası yanlışlıkla seçilmez. Esc ve başarısız işlem belgeyi korur.

Nokta taşıma ve uç teğetleri ortak koordinat okuyucusunu kullanır:
mutlak `X,Y`, birimli göreli `@dX,dY`, Orto/kutupsal/koordinat kilitleri
aynı referanstan hesaplanır. İlk ve son teğet aşamalarının önizlemesi
salt okunur aynı anlamsal plandır. Enter ilgili ucu serbest bırakır.
Ağırlık sıfırdan büyük ve sonlu olmalı; geçersiz değer seçili noktayı
kaybettirmez. Dönüşüm toleransı yalnız açıkça seçilen dönüşümde sorulur.

Düğüm ekleme şekli korur; Aç/Kapat mevcut yeniden kurma komutlarıdır.
Denetim/ağırlık/düğüm düzenlemesinde mevcut komutun geçiş verisini
bırakması arayüzde açık yazılır. Uç teğeti yönü G2 eğrilik eşlemesi
değildir; var olmayan bir süreklilik çözücüsü sunulmaz. Bu dilim eski
NURBS→çoklu çizgi yaklaşımını sertifikalı hata hesabına dönüştürmez.

Gerçek kullanımda fare paneldeyken Undo geometriyi güncelliyor, aracın
önbelleğindeki eski halkalar sonraki fare olayına kadar kalıyordu.
Belge kuşağı değişiminde `Tool::document_changed(&Context)` yalnız geçici
gösterimi yeniler; varsayılanı boş, bağlamı salt okunurdur. Yapay fare
olayı gönderilmez ve ikinci yazma/Undo oluşturulmaz. Eğri aracının
ayarları revizyon/seçim/aşama/görünüm değişmedikçe yeniden hesaplanmaz.

Yeni komut, dosya şeması veya Python/MCP sözleşmesi yoktur. Bütün yazmalar
aynı `cad.curves.edit` üzerinden yapılır. Web yüzeyi ADR 0228 gereği ertelidir.

## Araştırma dayanakları

- [AutoCAD SPLINEDIT](https://help.autodesk.com/cloudhelp/2026/ENU/AutoCAD-MAC-Core/files/GUID-216FEC99-80A1-4D1E-BC5B-010690C6FED1.htm):
  geçiş verisi, denetim ağı, ağırlık, uç teğetleri ve açık/kapalı düzenleme.
  KentOS'ta nokta görünümünü değiştirmek, geometri değiştiren bir dönüşüm değildir.
- [BricsCAD SPLINEDIT](https://help.bricsys.com/en-us/document/command-reference/s/splinedit-command?version=V26):
  geçiş/denetim işlemlerinin ve dönüşüm toleransının ayrılması.
- [Rhino Match](https://docs.mcneel.com/rhino/8/help/en-us/commands/match.htm)
  ve [GCon](https://docs.mcneel.com/rhino/8mac/help/en-us/commands/gcon.htm):
  konum, teğet ve eğrilik sürekliliği ayrı yeteneklerdir; yalnız teğet yönü G2 diye adlandırılmaz.

## Doğrulama

- Interaction: 71 kitaplık ve 856 bütünleşme testi geçti (2 ignored).
  Yeni durumlar görünümün salt okunurluğu, doğrudan/gizli nokta seçimi,
  geçersiz ağırlık sonrası düzeltme, anlamsal teğet önizlemesi, Aç/Kapat,
  dönüşüm iptali, mm projede göreli koordinat, Orto ve Undo işaretlerini kapsar.
- Masaüstü: 1.572 test geçti (256 ignored); yeni gerçek App mesajı testi,
  imleç paneldeyken tek Undo/Redo sonrası nokta işaretlerini denetler.
  Desktop/interaction kitaplık Clippy ve üretim uygulama derlemesi geçti.
- Gerçek macOS GUI: geçiş noktasını `3,2`'ye, ağırlıklı eğrinin denetim
  noktasını `0,2.5`'e taşıma; ağırlık `0` reddi ardından yeniden seçmeden
  `2.5`; düğüm ekleme; iki uç teğeti; Aç/Kapat; tek Undo/Redo; açık/koyu
  tema ve 1100 mantıksal piksel genişlikte panel sınandı.
- GUI'den kaydedilen geçiş, denetim, ağırlık, teğet ve Aç/Kapat geometrisi
  Python'un aynı komut planıyla karşılaştırıldı; kimlik ve öznitelikler korundu.
  Ağırlıklı çeyrek daireye GUI düğüm eklemesi, bağımsız rasyonel de Boor
  okuyucusunda 10.001 örnekle sınandı: kaynak farkı en çok
  `8.89e-16 m`, analitik daire yarıçap farkı `4.45e-16 m`.
  Bu yoğun örnek testi genel bir aralık ispatı olarak sunulmaz.
- Ham ekranlar ve veri repo dışında `/tmp/kentos-cad-adv04-2026-11-01/`:
  `31-...45-...png`, `curve-edit-gui-audit.txt` ve kaydedilen KCAD karşılaştırmaları.

Delikli/yaylı alan, bağlı ölçü/tarama ve alt nesne kimliklerinin birleşik
kabulü ayrıca tamamlanacak; güvenli ret tamamlanmış destek sayılmaz.
