# 0830 — Kısıt başarısının görünür kalması ve parametre birimleri

2026-11-02. Durum: uygulandı; otomatik ve gerçek GUI kabulü doğrulandı.
CAD-ADV-05 açık.

Gerçek GUI elips kabulünde Sabit eklendiği halde aynı imleç noktasındaki
sonraki önizleme hemen aynı bağı yeniden eklemeyi deneyip kırmızı
“tekrarlıyor” gösteriyordu. Başarı durumuyla yeni bir işlem önizlemesi
üst üste geliyordu.

Başarılı kısıttan sonra son tıklamanın dört mantıksal piksel çevresinde
yeni önizleme başlatılmaz. İmleç oradan ayrılınca yeni bağ önizlemesi
normal çalışır. Aynı noktaya bilerek tekrar tıklamak halen gerçek
komuta gider ve tekrarlama reddini söyler. Belge generation değişirse
(Undo dahil) bekletme geçersizdir. Hover hiçbir bağ eklemez, silmez;
başarısız işlemin hata gösterimi gizlenmez. Yöntem değişimi de bekletmeyi
sıfırlar. Çözücü ve komutlar değiştirilmedi.

Parametre sonuç sütunu uzunluk/açı adını tekrarlamak yerine gerçekten
kullanılan birimi yazar: projenin mm/cm/m/in/ft işareti, açı için °/gon.
Sayı parametresi boyutsuz kalır. Değer parametrenin kendi birimindedir;
bir daha ölçek uygulanmaz. Dört ondalığa zorunlu yuvarlama kaldırılır:
sıfır olmayan küçük değerler sıfır gibi gösterilmez. Çok büyük/küçük
sonuçlar bilimsel gösterimle, diğerleri on iki anlamlı basamakla
yazılır; örneğin 30deg → 33.3333333333 gon. İfadenin kendisi ve saklanan
f64 değer değiştirilmez; çizim/Python/MCP
sayısal çıktıları ve dosya şeması aynıdır.

İki etkileşim regresyonu: başarı sonrası aynı konumda hata önizlemesi
olmaması; yeni hover/bilinçli tekrarın normal reddi; aynı imleçte Undo
sonrası yeni geçerli önizlemenin açılması ve geri alınan bağın kendiliğinden
yeniden oluşmaması.

[BricsCAD Parameters Manager](https://helpcenter.bricsys.com/en-us/document/panels/parameters-manager-panel?version=V25)
ifade/değer/tür incelemesini ayırır.
[AutoCAD dimensional constraints](https://help.autodesk.com/cloudhelp/2026/ENU/AutoCAD-Core/files/GUID-F7ED2587-8F25-4770-BD13-C7C95456E707.htm)
sürücü ile yalnız ölçen referansı ayırır. KentOS bu görünürlük işini
kendi komutlarını ve sayısal anlamını koruyarak uygular.


Gerçek masaüstü kabulü: Sabit kısıtı merkezde bir kez eklendi ve durağan
imleçte başarı görünür kaldı; bilinçli ikinci tıklama yine tekrarı reddetti.
Parametreler arayüzden eklendi: mm/grad projesinde `120cm` → `1200 mm`,
`30deg` → `33.3333333333 gon`, sayı `0.000000001` → `1e-9 · sayı`.
KCAD v2 kaydedilip yeni derlemede açıldı; 1440 ve 1100 mantıksal piksel
pencerede sonuçlar taşmadan okunuyor. Görseller 71, 73, 74 repo dışında
`/tmp/kentos-cad-adv04-2026-11-01/` altında tutulur.

Etkileşim bütün hedefleri: 71 birim + 865 bütünleşik başarılı;
masaüstü: 1577 başarılı. Son gösterim değişiminden sonra parametrelerin
beş kabul testi, tüm masaüstü hedefleri Clippy ve uygulama derlemesi de
başarılı. Bu dilim çözücünün elips dönüş sınırını kapatmaz.
