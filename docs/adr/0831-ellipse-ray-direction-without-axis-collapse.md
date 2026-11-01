# 0831 — Elips merkez ışınını ekseni çökertmeden yönlendirme

2026-11-02. Durum: uygulandı; otomatik ve gerçek GUI kabulü doğrulandı. CAD-ADV-05 açık.

## Gerçek sorun ve kanıt

ADR 0829'un gerçek masaüstü testinde yatay tam elipsin merkezi ile dikiş
noktasına Düşey eklemek mümkün olduğu halde ret üretti. Bağımsız çekirdek
regresyonu bunu büyük koordinatta da tekrar etti: merkez (3e6, 1e7), yarı
eksenler 4/2, sabit merkez ve 4 m merkez–dikiş mesafesiyle Düşey,
`Inconsistent`, conflicting=[2], iterations=1 döndü; geometri korunmuştu.

Eski düşey denklemi Δx=0. Yatay başlangıçta Δx=a·cos(θ) için θ türevi
sıfır; çözücü a'yı sıfıra yöneltiyor. Pozitif yarı eksen ve sabit uzaklık
bunu doğru olarak reddediyor. Ret güvenli olsa da geometrik problem çözülebilir.

## Karar ve sınır

Yalnız **aynı elipsin Center ile Start/End** nokta çiftindeki Yatay/Düşey
için sıfır kümesi eşdeğer bir yön denklemi kullanılır. Sıfır olmayan
merkez→uç vektörünün dual `atan2` açısı, yatay için 0, düşey için π/2
hedefine π periyoduyla en yakın doğrultuda bağlanır. Artık, çizimin mevcut
ölçeği `s` ile çarpılır; toleranslar, rank testi ve ortak Gauss–Newton
çözücüsü değişmez. Seçim sırası ışın yönünü değiştirmez. Tam çeyrek dönüş
iki eşdeğer çözümün sınırıdır; yarı açık açı aralığının deterministik dalı
kullanılır. Yakın doğrultularda ışın karşı yarı düzleme atlamaz.

Bu eşdeğerlik elipsin pozitif eksenlerinden dolayı sıfır olmayan kendi
merkez ışınına özgüdür. Genel iki nokta, çizgi ve bağımsız daire/yay
Yatay/Düşey denklemlerine dokunulmaz. Değişken elips yay ucunun gerçek
parametresi kullanılır; eksen noktası gibi yorumlanmaz. Geçersiz/sıfır
ışın geçerli çözüm sayılmaz. Mevcut dal ve pozitif eksen korumaları sürer.

Kamu komutu, kalıcı başvuru ve dosya şeması değişmez. UI/Python/MCP aynı
`cad.constraints.edit`/`query` yolunu kullanır. Önizleme salt okunurdur;
başarısız işlem son geçerli modeli değiştirmez; başarı tek Undo adımıdır.

## Kaynaklar ve tasarım çıkarımı

[AutoCAD GEOMCONSTRAINT](https://help.autodesk.com/cloudhelp/2022/ENG/AutoCAD-Core/files/GUID-D79452FB-DDA7-44C0-A9EA-F4EA656022D2.htm)
elipsin merkez/eksen noktalarını ve yatay/düşey kısıt türlerini belgeler.
[FreeCAD Constrain horizontal](https://github.com/FreeCAD/FreeCAD-documentation/blob/main/wiki/Sketcher_ConstrainHorizontal.md)
çizgi veya iki nokta seçimini ve aracı sürdürme davranışını açıklar.
[FreeCAD GCS arayüzü](https://github.com/FreeCAD/FreeCAD/blob/main/src/Mod/Sketcher/App/planegcs/GCS.h)
mevcut noktalar ve elipsler için ortak geometrik kısıt API'si örneğidir.
Yukarıdaki normalize açı denklemi bu kaynaklardan kopyalanmış uygulama
iddiası değildir; KentOS'un ölçülmüş Jacobian sorunu için kendi matematiksel
çıkarımıdır. Başka çözücü veya bağımlılık eklenmez.


## Kabul kanıtı

- Çekirdek: 0/90/180/−90° başlangıç, iki seçim sırası, büyük dünya
  koordinatı, 4 m yarı eksen ve 2 m küçük eksenin korunması; gerçek
  eliptik yay End ve en yakın yarı düzlem; çelişkili yönler ve tutulmuş
  elipste değişmeden ret. Bütün çekirdek hedefleri: 891 başarılı.
- Yerel ürün: plan salt okunur, dönen uçtaki çizgi izler, dört bağın
  teşhisi tutar, kalıcı kimlikler ve v2 roundtrip, tek Undo; sonradan
  yatay eklemek atomik reddedilir. Yerel bütünleşik testler: 287 başarılı.
- Etkileşim: çeyrek dönüş hover'ı yazmaz, iki tıklama tek bağ ekler,
  tek Undo geri getirir. Bütün etkileşim: 71 + 866 başarılı.
- Python: iki kabul testi; MCP: aynı geometri, plan, query ve Undo.
  Masaüstü: 1577 başarılı. İlgili bütün hedefler Clippy, masaüstü
  derlemesi ve bağımlılık yönü denetimi temiz (51 crate/67 hedef).
- Gerçek masaüstü: önceden fareyle kurulmuş Sabit + d1=4 m + Çakışık
  setindeki yatay bağ açıkça kaldırılmış başlangıç dosyasında Düşey aracı
  seçilip merkez/dikiş tıklandı. Elips major=(2.4492935982947064e−16,4),
  ratio=0.5, merkez=(0,0), tam kapalı; çizgi başlangıcı=(0,4), bitişi=(7,0).
  Parametre penceresinde dört bağ tutuyor, üç serbestlik kalıyor.
  Kaydedilmiş geometri, nesne kimlikleri, parametreler, bağ kuralları ve
  query Python'un aynı başlangıca uyguladığı komutla birebir eşleşti.
  Bağımsız işlemlerin yeni bağ UUID'si doğal olarak ayrıdır. Tek görünür
  Undo elipsi, çizgiyi ve üç eski bağı geri getirdi.

Ekranlar 75–77 ve `ellipse-turn-gui-acceptance.json` repo dışında
`/tmp/kentos-cad-adv04-2026-11-01/` altında. Bu dilim yaylı path bulge,
boyut cebiri, elips grip hedefi veya genel NURBS desteğini tamamlamaz.
