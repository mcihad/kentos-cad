# 0840 — Parametre boyutları ve eski ifadelerin korunması

2026-11-02. CAD-ADV-05. Durum: ortak komut, Python/MCP ve gerçek Mac arayüz kabulü geçti.

## Araştırma ve mevcut sorun

[Fusion Parameter.expression API](https://help.autodesk.com/cloudhelp/ENU/Fusion-360-API/files/fusion_Parameter_expression.htm)
birimsiz yazılan sayının belge biriminde yorumlanmasını, açık birimin ise
belgeden bağımsız olmasını tanımlar.
[Fusion parametre referansı](https://help.autodesk.com/cloudhelp/ENU/Fusion-Model/files/GUID-76272551-3275-46C4-AE4D-10D58B408C20.htm)
parametrenin türünü, ifadesini ve sonucunu ayırır; matematiksel `PI`
birimsizdir. [FreeCAD ifadeleri](https://github.com/FreeCAD/FreeCAD-documentation/blob/main/wiki/Expressions.md)
işlemlerin birim uyuşmasını ister; çıplak sayının uzunluğa eklenmesinde
KentOS'tan daha katıdır. İncelenen açık kaynak
[PropertyQuantity::setPyObject](https://github.com/FreeCAD/FreeCAD/blob/main/src/App/PropertyUnits.cpp)
birimli girdinin hedef birimle uyuşmasını denetler. Bu kaynaklardan kod
alınmadı; KentOS'un mevcut parser ve sayısal değerlendiricisi kullanılır.

KentOS `ParameterKind` saklıyordu fakat değerleri yalnız `f64` olarak
hesaplıyordu. Böylece `2m + 45deg`, uzunluğa `45deg`, uzunluk yerine
`boy/en` gibi girdiler ölçüyü sürebiliyordu. Diğer yandan mevcut `pi`,
matematiksel birimsiz π değil, proje açı biriminde **yarım tur**dur.
Bu anlamı değiştirmek kaydedilmiş çizimlerin ölçülerini bozardı.

## Boyut kuralları

- `constraint_expr/dimensions.rs`, aynı ayrıştırılmış `ParamExpr` ağacında
  uzunluk ve açı üslerini izler. Sayısal değerlendirme ve birim dönüşümü
  değiştirilmedi. Geometri toleransı genişletilmedi.
- Açık uzunluk/açı birimleri kendi türünü taşır. Adla anılan parametre
  beyan edilmiş Length/Angle/Number türünü taşır; Number açıkça birimsizdir.
- Çıplak sayılar ve tamamen çıplak aritmetik, sonuçta hedefin proje
  birimini alabilir; toplamada diğer boyuta uyabilir, çarpma/bölmede sayı
  katsayısıdır. `boy + 25`, mm projede 25 mm ekler. Adlı Number aynı
  örtük uzunluk sayılmaz; gerekirse `adet * 1mm` açıkça yazılır.
- Toplama/çıkarma ve min/max boyut uyumu ister. Çarpım/bölüm üsleri
  toplar/çıkarır. Ara alan ve ters uzunluk hesaplanabilir; hedef hâlâ
  Length/Angle/Number'dır. Örneğin `sqrt(boy^2 + en^2)` uzunluk,
  `boy/en` sayıdır; `boy*en` doğrudan uzunluk parametresi olamaz.
- `sqrt` üsleri yarılar; `^` birimsiz üs ister. Adlı bir sayı üs olarak
  değiştiğinde önceden geçerli bağımlı ifadelerin boyutu da yeniden
  denetlenir. Üs aritmetiğinin yalnız boyut karşılaştırmasında 1e-12
  mutlak eşiği vardır; bu, çözücünün konum toleransı değildir.
- Sin/cos/tan uzunluk kabul etmez; açı veya mevcut proje açı birimindeki
  birimsiz sayıyı alır. Ters trigonometrik işlevler birimsiz sayı alıp
  açı üretir. Abs/round/floor/ceil girdinin boyutunu korur. Eski çıplak
  `sin(30)` gibi ifadelerin proje birimindeki kullanımı korunur.
- `pi` açı boyutunda yarım turdur. `pi/2` çeyrek tur, `pi/1rad`
  birimsiz matematiksel π'dir. Yeni sabit adı veya yeni gramer eklenmedi.

## Yazma ve uyumluluk

- `cad.parameters.edit` ve `cad.constraints.edit`, ortak native
  `check_dimension_writes` üzerinden yeni/açıkça değiştirilen ifadeyi
  ve önceden geçerli bağımlıların yeni boyutunu kontrol eder. Hata
  `parameter_kind`, `parameters[i].expression` yolu ve Türkçe düzeltme
  açıklaması taşır; validate/plan/execute aynı ret verir.
- Eklenen parametrenin sonra yeniden adlandırılması kontrolü atlatmaz.
  Bir komut içindeki ad değişiklikleri eski parametre kimliğine eşlenir;
  remove/add yeni parametredir. Rename veya yalnız açıklama değiştirme,
  eski hatalı formülü zorunlu bir göçe çeviremez.
- Açılışta eski ifade/sonuç yeniden yorumlanmaz. Önceden uyumsuz olup
  ifadesine dokunulmayan parametre çalışmaya devam eder. Bu istisna,
  daha önce geçerli bir bağımlıda yeni oluşan hatayı örtmez. Eski hatalı
  ifade açıkça yeniden girilirse artık düzeltilmesi gerekir.
- `cad.constraints.query` salt okunur `parameter_units_legacy`
  uyarılarını ortak denetimden üretir. İfade, değer ve dosya değişmez.
  Parametre penceresi uyarı sayısını ve ad seçilince ayrıntıyı gösterir;
  taslakta tuş başına çözücü veya bu analiz çalıştırılmaz.
- Python ve MCP aynı ürün komutunu kullanır. Katalog ret/uyarı listesi,
  açıklamaları ve üretilmiş Python belgeleri güncellendi. KCAD şeması,
  kayıtlı ifade grameri ve komut girdi/çıktı türleri değişmedi.

## Doğrulama

Ortak boyut testleri, native validate/plan/execute atomiklik ve tek Undo,
metrik/imperial uzunluk ile derece/gon matrisi, değişken üs bağımlısı,
yeni ölçüye yanlış tür, eski pi'nin korunması/rename/açık onarımını
kapsar. Python eski dosyayı bağımsız KCAD referans kodlayıcısıyla üretir;
MCP kabulü gerçek dosyayı sorgular, onarır ve Undo ile baytlarına döner.

- Ortak ifade testleri 7, native application tam paket 313 geçti (mevcut
  37 ignored). Yeni boyut kabulü yedi native test içindedir; fiziksel
  sonuçlar beş uzunluk × iki açı biriminde denendi. Masaüstü tam paket
  1582 geçti (mevcut 256 ignored). All-targets contracts/application/
  MCP/desktop Clippy ve masaüstü derlemesi geçti.
- Python yeni boyut dosyası 3 test, mevcut kısıt dosyaları 14 test geçti.
  MCP tam paket 46 test geçti; yaylı kenar, elips ve eski ifade kabulünü içerir. Eski dosya
  sorgusu dosyayı değiştirmez; açık onarım ve tek Undo baytları geri getirir.
- Temiz kaynak tekrarında komut paketinin 1057 testi geçti. Tam katalog
  denetiminin bulduğu seçim/offset raporlarındaki eksik count/index birimleri
  ve isolation örneğinin tip kontrolü tamamlandı; katalog ve Python belgeleri
  yeniden üretildi. SDK tutarlılık kontrolü de geçti.
- Gerçek GUI: büyük koordinatlı, altı bağlı mm çizimde
  `2500mm + 45deg` tek Enter ile açıklamalı reddedildi. Saklanan yarıçap
  2500 mm kaldı; kaydedilen dosya başlangıçla baytları tam aynı.
  `sqrt((3000mm)^2)` tek Enter ile 3000 mm oldu, altı bağ yeşil kaldı;
  teğet çizgi izledi. UI/Python nesneleri ve dosyaları tam aynı.
  Tek Undo, başlangıç baytlarını geri verdi.
- GUI'de eski Number `pi`, 180 sayısal değeriyle ve birim uyarısıyla
  açıldı. Adına tıklayınca açıklaması görüldü. Açık `pi / 1rad` onarımı
  π sonucunu verdi, uyarı kalktı; geometri değişmedi. UI/Python onarılmış
  dosyaları aynı, tek Undo başlangıç baytlarıyla aynı.
- Yaklaşık 1101×682 mantıksal piksel test penceresinde `45deg` reddi,
  tamamı okunabilir hata, tablo kaydırması ve erişilebilir Kapat düğmesi
  doğrulandı; kaydedilen belge yine aynı. Geniş pencerede kaydedilmiş
  çözüm yeniden açıldı: karekök ifadesi, 3000 mm, altı geçerli bağ ve
  sıfır serbestlik korundu.
- Ekranlar `121-parameter-unit-mismatch-retains-model.png` ile
  `127-parameter-units-narrow-table-scroll.png` arasındaki yedi dosya;
  `parameter-units-gui-acceptance.json` sayısal/dosya kanıtıdır. Günlükler
  ve ekranlar yalnız `/tmp/kentos-cad-adv04-2026-11-01/` altındadır,
  görsel repoya eklenmedi.

Tam elips eksen kimliği/grip davranışı ve CAD-ADV-05 birleşik teknik
plaka kabulü bu değişikliğin kapsamında bitmiş sayılmaz.
