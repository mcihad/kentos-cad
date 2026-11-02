# 0833 — Blok yerleştirme noktasının kısıtlarla düzenlenmesi

2026-11-02. Durum: sınırlı yerleştirme adaptörü kabul edildi. CAD-ADV-05 açık.

## Kodda bulunan boşluk ve araştırma

`SubPlace::Node { part: 0 }` ve ortak `ops/subref` zaten Insert'in
yerleştirme noktasını kalıcı olarak tanımlar. Mekânsal Store, görünür
parçalar uzakta olsa da yerleştirme noktasını genişletilmiş kutuya dahil
eder. Kısıt aracının nokta seçicisi de bu noktayı bulabilir. Fakat
`constraints/model.rs` Insert'e bilinmeyen ayırmadığı için ürün komutu
bu mevcut başvuruyu reddediyordu. Önce yazılan üç çekirdek kabulü bunu
gösterdi: bağlar bloğu hedefe götürmüyor ve izleyen nokta hareket etmiyordu.

[AutoCAD GEOMCONSTRAINT](https://help.autodesk.com/cloudhelp/2024/ENU/AutoCAD-Core/files/GUID-D79452FB-DDA7-44C0-A9EA-F4EA656022D2.htm)
bloklarda yerleştirme noktası ve iç nesneleri farklı kısıt noktaları
olarak listeler.
[Bloklarla kısıt kullanımı](https://help.autodesk.com/cloudhelp/2027/ENU/AutoCAD-Core/files/GUID-899E008D-B422-4DF2-AC8D-1A4F5701ED4E.htm)
blok başvurularının kısıt sonucu hareket edebileceğini veya dönebileceğini
açıklar. KentOS'un bu aşaması yalnız yerleştirme noktasıdır; iç parça
kısıtını veya dönme bilinmeyenini uygulanmış gibi göstermez.

## Karar

- Mevcut Point/Node(0) kullanılır; yeni başvuru, dosya şeması, komut veya
  ikinci çözücü eklenmez. Yalnız p.x/p.y bilinmeyendir.
- Bloğun tanım kimliği, ayna, dönüş, tekil/ayrı X/Y ölçekleri, öznitelik
  değerleri ve taşınmış öznitelik yerleri korunur. İç geometri yeniden
  yazılmaz, patlatılmaz; mevcut blok genişletme yolu yeni konumu çizer.
- Sabit, Çakışık, iki nokta Yatay/Düşey, mesafe ve nokta simetrisi ortak
  Point denklemleridir. Node(1), Start, Center, Segment veya Circle bloğun
  içindeki ilk parça gibi yorumlanmaz.
- Yerleştirme grip'i yalnız iki koordinatı geçici tutar. Bağlı nesneler
  bu hedefe çözülür; aynı işleme ikinci bir düzeltme komutu eklenmez.
- Gerçek grip kabulü, mevcut genel `cad.entities.edit update` yolunda
  taşınmış öznitelik konumlarının silindiğini yakaladı. Aynı tanımın
  geometri güncellemesi artık öznitelik konumlarını, dinamik değerleri ve
  yerel kırpmayı korur. Başka tanıma bu yerel değerler taşınmaz; nesnenin
  mimari faz/kat bilgisi korunur. Bu bilgiler EntityGeometry içinde
  bulunmadığı için ham geometriyle örneği yeniden oluşturup sıfırlanmaz.
- Kilitli katmandaki referans yerinde kalır; serbest blok ona göre
  düzenlenebilir. Hedefi tutan başka bağ veya kilit varsa mevcut atomik
  hata yolu son geçerli modeli korur. Tolerans değiştirilmez.
- Dizi/ilişki tarafından yönetilen örneklerin mevcut `doc.governed`
  denetimi kaldırılmaz. Bu adaptör ilişki otoritesinin önüne geçmez.
- Araç, blok noktası seçildikten sonra yalnız yerleşimin değişeceğini
  söyler. Destek dışı iç parçayı tıklamak açıklamalı ret verir. Parametre
  penceresi blokların açı/ölçek/iç parça serbestliklerinin bu modeldeki
  sayıya dahil olmadığını açıklar. Undo ile bu kapsam notu güncellenir.

## Ortak API

`cad.constraints.edit`, `cad.parameters.edit` ve `cad.constraints.query`
UI/Python/MCP için tek otoritedir. `op: plan` yazmaz; çalıştırma bir Undo
adımıdır. [Python/MCP örneği](../cad/block-anchor-constraints-python.md).

## Kabul

Ortak ham örnek: `fixtures/interaction/v1/constraint-block-anchors.kcad`.
30° dönük, aynalı, X=1.5/Y=0.75 ölçekli asimetrik bileşen, taban noktası
(1,1), kendi özniteliği ve öznitelik konumu vardır. Yerleştirme noktası
görünür parçalardan uzaktadır. Sabit referans, Yatay, 6m mesafe ve izleyen
çizgi; parametre 8m yapılırken yalnız blok konumu ve çizgi başlangıcı
değişmelidir. Kimlik, tanım ve bütün yerleştirme sabitleri korunmalıdır.
Dosya roundtrip, tek Undo, kilitli referans, kilitli hedef, kapsam dışı
parça ve kısmi kısıtlı grip testi aynı sözleşmeye tabidir.

- Çekirdek: 897 test geçti. Native application: 292 geçti / 37 bilinen
  ignored. Interaction: 71 lib + 873 entegrasyon geçti; 2 bilinen ignored
  ve 5 performans testi çalıştırılmadı. Yeni MCP kabulü ve iki Python
  kabulü geçti. Masaüstü: 1579 geçti / 256 bilinen ignored.
- İlgili crate'lerde all-targets clippy `-D warnings`, masaüstü derlemesi,
  bağımlılık denetimi (51 crate / 67 hedef) ve diff kontrolü geçti.
- Gerçek Mac arayüzünde dört bağ fareyle kuruldu; mm projesinde 6→8 m
  ifadesi 8000 mm göründü. Dönük/aynalı/asimetrik blok ve izleyen çizgi
  taşındı. Dar ve geniş parametre penceresinde kapsam notu okunabildi.
- Bağlar açıkça başvuru/serbest hale getirildikten sonra gerçek fareyle
  yerleştirme grip'i sürüklendi; bağlı çizgi ve taşınmış öznitelik izledi.
  Aynı grip'e `@2000,2000` yazılarak kesin (8,2) metre hedefi verildi.
  Başvuru ölçüsü 8246.211 mm hesaplandı. Her iki sonucun nesneleri,
  kimlikleri ve query teşhisi Python komutuyla eşleşti. Tek Undo kaynak
  belgeye döndü. Query'nin oturum revision alanı bu karşılaştırmaya dahil değil.
- Çakışık hover'ında genişletilmiş gerçek bloğun kesikli hayaleti görüldü;
  Escape ve iç parçaya açıklamalı ret, kaydedilmiş belgeyi değiştirmedi.
- Ekran görüntüleri 85–94 ve sayısal karşılaştırma raporu repo dışında
  `/tmp/kentos-cad-adv04-2026-11-01/` altında tutuldu; binary eklenmedi.

## Ayrı bulunan teşhis düzeltmesi

Hover/execute `hold=[ilk seçilen UID]` ile çalışır; dosyadan query ise bu
geçici tutuş olmadan çalışır. Önceden Fix ile sabitlenmiş ilk nokta,
geçici hold altında gereksiz bağ raporlanır. Aynı açık hold girdisiyle
Python da aynı Over/2 sonucunu verdi; hold olmadan Under/2 verdi.
Dolayısıyla geometri veya SDK ayrışması değildir, fakat kullanıcıya
kalıcı çizimde olmayan bir gereksizlik gösterilir. Geçici hedef ve kalıcı
teşhisin ayrılması CAD-ADV-05 içinde sonraki düzeltmedir. Bu adaptörün
kabulü bütün parametrik maddenin kapandığı anlamına gelmez.
