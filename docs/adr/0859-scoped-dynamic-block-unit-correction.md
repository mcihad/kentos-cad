# ADR 0859 — Dinamik blokta sayıları koruyan birim düzeltmesi

Tarih: 2026-11-03. CAD-ADV-06; ADR 0315, 0318 ve 0858'in devamı.

## Sorun ve araştırma

Gerçek masaüstünde `Birim › Sayıları koru: milimetre`, metre cinsinden
kapının parçalarını küçültürken doğrusal parametresini, tutamaç konumunu,
katalog satırlarını ve yerleştirmelerdeki genişlikleri eski değerlerinde
bırakıyordu. Görsel seçim ve özellik paneli aynı kapıyı farklı boylarda
anlatıyordu. `blocks_edit::rescaled` yalnız statik parçaları ve iç
INSERT çarpanlarını ölçekliyordu.

AutoCAD'in [blok birimleri ve ekleme ölçeği açıklaması](https://help.autodesk.com/cloudhelp/2025/ENU/AutoCAD-Core/files/GUID-6C46049D-8636-442D-8BAC-CF4FD515FDC0.htm)
ekleme birimini ek ölçekten ayırır. Autodesk'in [dinamik bloklarda birim
değiştirme sorununa ilişkin açıklaması](https://www.autodesk.com/support/technical/article/caas/sfdcarticles/sfdcarticles/How-to-change-or-convert-the-units-of-a-drawing-that-contains-dynamic-blocks-in-AutoCAD.html)
`INSUNITS` desteğinin dinamik bloklara doğrudan genellenemediğini belirtir.
[BricsCAD INSERT](https://help.bricsys.com/en-us/document/command-reference/i/insert-command?version=V27)
birim kaynaklı ölçek ile yerleştirme parametrelerini ayrı tutar. KentOS'un
metre cinsinden kanonik koordinat kararı korunur; bu kaynaklardan
KentOS formüllerinin veya iç veri yapısının aynı olduğu sonucu çıkarılmaz.

## Karar

- `cad.blocks.edit(operation="units", rescale=false)` fiziksel boyları
  korur; yalnız birim etiketi ve mevcut yerel geometri sayacı davranışı
  değişir. Kullanıcı birim etiketini değiştirdi diye geometri dönüştürülmez.
- `rescale=true` eski/yeni birimin tam sayı oranını kullanır. Aynı birimde
  oran 1'dir: formül yeniden sarılmaz, kayıtlı değerler yeniden yazılmaz.
- Taban, statik parçalar, öznitelik tanımları, doğrusal/kutupsal uzunluklar,
  nokta/çevirme/açı merkezleri, sınır/adım/listeler, katalog uzunlukları,
  germe alanları ve dizi aralıkları ortak `units` yordamlarıyla dönüşür.
  Açılar, görünüş adları, bayraklar ve parametre grupları korunur.
- Uzunluk formülleri mevcut gerçek ifade lexer'ıyla `s*f(length/s, angle)`
  biçiminde dönüşür. Alıntılar veya benzer adlı parametreler metin
  değiştirmeyle bozulmaz. Açı çıktısına uzunluk çarpanı uygulanmaz.
- Bu tanımı kullanan kök ve başka tanımlardaki iç yerleştirmelerin
  uzunluk/nokta girdileri, taşınmış öznitelik yerleri ve kırpma sınırları
  tanımın yerel koordinatlarıyla birlikte dönüşür. Türetilmiş değerler
  girdi sayılmaz; yeni formüllerden yeniden hesaplanır.
- Üst bloğun kendi tabanı, dünyadaki ekleme noktası, dönüşü ve ölçeği
  değiştirilmez. Ölçeklenen tanımın **içinde** yerleştirilmiş başka bir
  blokta ise çocuğun tanımı/girdileri/yerel öznitelik yerleri korunur;
  yalnız üst tanımdaki ekleme noktası ve yerleştirme çarpanı dönüşür.
- Kilitli katmandaki doğrudan veya dolaylı kullanım boyut değişikliğinden
  etkilenecekse işlem bütünüyle reddedilir (`layer_locked`). İlgisiz kilitli
  nesne dönüşümü engellemez. Boyları koruyan etiket değişimi kullanılabilir.
- Geçici tanım ve nesneler tamamlanır; bütün blok grafiği ve sahne birlikte
  doğrulanır. Temsil edilemeyen sayılar, bozuk parametre/formüller ve
  dosya sözleşmesinin altına küçülen kırpma sınırı hiçbir yazı yapılmadan
  reddedilir. Dönüşüm varsayılan değere sıfırlayarak veri kaybını gizlemez.
- Tanım ve etkilenen üst tanımlar `merge_blocks` ile aynı işlemde yazılır;
  kök kayıtlar da aynı işlem içindedir. Tek Undo/Redo bütün değişikliği
  kapsar. Plan, değişen üst tanımları da `changed` içinde gösterir.
- Yeni komut/sürüm veya KCAD şeması gerekmez. Masaüstü, Python, MCP ve
  iç ajanlar aynı native komutu kullanır. Web'in ayrı uygulama handler'ı
  bu teslimle eşitlenmiş sayılmaz.

## Kabul

Native testler statik geometri, dinamik sınır/katalog değerleri, kök/iç
girdiler, formüller, öznitelik yerleri, kırpma, kilitli üst kullanım,
ilgisiz kilitli nesne, birimsiz dönüşüm, taşma, planın yazısızlığı ve tek
Undo/Redo'yu kapsar. Python/MCP gerçek korumalı plan–çalıştır yolunu ve
KCAD baytlarıyla geri almayı sınar. Gerçek masaüstünde menüyle dönüşüm, tek Undo/Redo,
ölçü düzenleme ve yeniden açma yapıldı. İlk örnekte 1→0.001 m; yanlış
yorumlanmış 900 birimlik kapı örneğinde varsayılan 900→0.9 m, override
1000→1 m ve ardından 1→1.05 m düzenleme kabul edildi. Gerçek GUI ile
korumalı Python komutunun dosyaları iki örnekte tam aynı; birim/ölçü Undo
kayıtları ve yeniden açılıp kaydedilen dosya beklenen baytlarla tam aynı.
Sınır dışı 0.5 m girişi gerçek GUI’de reddedildi; kayıtlı çizim baytları
değişmedi. 330 application, 1612 masaüstü, 61 MCP ve 459 Python testi,
Clippy ve derleme geçti. Bağımsız okuyucu kanıt dosyalarını açtı.
Görüntüler ve kayıtlar depo dışında tutulur.
