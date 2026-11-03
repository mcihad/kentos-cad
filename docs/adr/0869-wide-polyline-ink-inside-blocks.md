# ADR 0869 — Geniş çoklu çizgi blok içinde de kendi rengiyle dolu çizilir

Tarih: 2026-11-03. CAD-ADV-07 görünüş koruma önkoşuludur.

## Dayanak ve gerçek hata

[Autodesk FILLMODE](https://help.autodesk.com/cloudhelp/2026/ENU/AutoCAD-Core/files/GUID-FC385D70-45AA-4B9A-848A-CA3906C36124.htm)
geniş çoklu çizgi dolgusunu tanımlar;
[blok yerleştirme belgesi](https://help.autodesk.com/cloudhelp/2027/ENU/AutoCAD-LT/files/GUID-BC0FD3C1-3BFC-4C5D-AB9A-BF480D5084BE.htm)
tanım içindeki nesne özelliklerinin korunmasını açıklar. KentOS kararı:
aynı yaylı/genişlikli çizgiyi bloğa almak modeldeki genişlik bandını sıradan
katman poligonu haline getirmemelidir.

Gerçek GUI kontrolünde kökte dolu görünen değişken genişlikli çizgi, blok
alt nesnesi olunca yalnız kenarlarla çizildi. KCAD dosyası ve blok tanımı
genişlikleri koruyordu; geometri kaybı değildi. `native/style::program`
kökteki geniş çizgiye kendi renginde dolgu veriyor, geniş blok parçasını ise
katmanın isteğe bağlı alan dolgusu üzerinden geçiriyordu. Katmanda dolgu
yoksa bant boş, varsa farklı renkte görünüyordu.

## Karar

Mevcut alt nesne stil seçiminde, geniş çoklu çizgi de kökte kullanılan
öz dolgu politikasını alır: model genişliği kendi rengiyle çizilir. Parçanın
renk/katman izleme ve saydamlık çözümü korunur; iç içe blok, ayna ve ölçek
aynı çözülmüş şekil üzerinden gider. Normal ince çizgi, alan, tarama ve
önceden aynı yolu kullanan kılavuz oku davranışı değiştirilmez.

Geometri, birim, eşleştirme toleransı veya GPU koordinat sistemi değişmez.
Yeni dolgu ayarı eklenmez. Var olan alan dolgularını gizleme ayarında geniş
kök çizginin mevcut davranışı korunur; bu değişiklik genel FILLMODE
uyumluluğunun tamamlandığı iddiası değildir.

## Kabul

Yaylı ve daralan bir çizginin blokta/tek başına çizimi; iç içe tanım,
ayna, ölçek, katmanda farklı dolgu var/yok, alan dolgusu görünümü ve fiziksel
çıktı hassasiyet yolu dahil 32 durumda aynı stil JSON'u ve float32 çizim
verisini üretir. Renk ve saydamlık da karşılaştırmadadır. Regresyon önce
eski kodda başarısız, düzeltmeden sonra başarılıdır. Stil paketinin bütün
77 testi (3 isteğe bağlı atlanan) ve masaüstünün 1 628 testi geçti.
Derleme/Clippy doğrulaması ayrıca geçirilir.

Gerçek son derlemede seçili/seçimsiz blok, yakınlaştırma ve bağlı 2 000 mm
ölçü incelendi. Arayüzden kaydedilen KCAD, düzeltme öncesi aynı çizim
verisiyle bayt bayt aynıdır. Ham görüntüler ve SHA kayıtları depo dışındadır.
Bu test genel tekrarları bloklaştırma aracını veya bütün PDF çıktılarının
ayrı görsel kabulünü tamamlamaz.
