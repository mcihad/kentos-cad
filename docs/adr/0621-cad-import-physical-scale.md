# 0621 — CAD içe aktarmada fiziksel birim ve bilinen ölçü

Tarih: 23 Ekim 2026. Kapsam: DWG/DXF içe aktarma, blok tekrar kullanımı ve model önizlemesi.

## Sorun ve karar

Native içe aktarma koordinat sistemi olan projelerde `unit: None` gönderiyor,
CAD dosyasının bildirdiği milimetreyi ham metre olarak okuyabiliyordu. Artık
CAD/CBS ayrımı olmadan kaynak birimi uygulanır. Birim bildirmeyen kaynak proje
birimini alır; koordinat sistemli projede bu metredir. Kullanıcının `fileUnit`
seçimi kaynak değerlerin yorumunu değiştirir; proje gösterim birimini değiştirmez.
Düşük seviye codec'in açık `DxfReadOptions.unit: None` ham metre kipi uyumluluk
amacıyla korunur. Bu kip native pencerenin otomatik seçimi değildir.

Geometri ortak çekirdekte metre tutulur. ADR 0589'daki blok kuralı korunur:
blok tanımı kendi fiziksel biriminde normalleştirilir, mevcut yerleştirmenin
çarpanı model/blok birim oranıyla telafi edilir. Kaynak birimin değiştirilmesi
bloğun tanımına ikinci kez uygulanmaz. Yeni ölçek 1 yerleştirmesi tanımın
fiziksel boyunu kullanır. Kaynakta ölçek 0,1 olan eski bir örneğin yeni ölçek 1
örnekle aynı boyda olması kendiliğinden garanti edilemez. Rapor bu farkı,
seçilmiş örneklerin X/Y çarpanlarıyla açıklar; kaynak geometriyi tahminle düzeltmez.

## Bilinen ölçü sözleşmesi

`file.import.dxf` ve `file.import.dwg` v1'e isteğe bağlı `scaleReference` eklenir:
`{ "sourceLength": 1250, "realLength": 12.5 }`. İlk sayı dosyadaki ham uzunluk,
ikincisi metredir. Model dönüşüm oranı ikincinin birinciye bölümüdür. İki değer
ve sonuç pozitif ve sonlu olmalıdır; `fileUnit` ile birlikte verilmesi
`invalid_scale` ile herhangi bir dosya okuma/yazmasından önce reddedilir.
Standart birime tam denk gelen oran aynı rasyonel birim dönüşümünü kullanır;
karşılaştırma toleransı genişletilmez. Sonuçtaki `UnitDecision` referansı taşır,
komut günlüğü girdiyi saklar. Bu ek alan eski girdiler için zorunlu değildir.

Masaüstü yalnız bu sözleşmeye girdi toplar; aynı ortak okuma hesabından önizleme
üretip aynı native komutla yazar. Referans düzenlenince eski önizleme geçersiz
olur, uygulanmadan içe aktarılamaz. Normal birim seçmek referansı kaldırır.
Python SDK ve MCP plan/execute aynı alanı kullanır. İçe aktarma tek undo'dur;
kayıt/yeniden açma fiziksel geometriyi korur. Kağıt boyutları model uzunluğu
olarak yeniden ölçeklenmez.

## Önizleme ve görünüş

Eski tanımlayıcı kutu yayı tam daire kadar sayıyor, yerleştirilmemiş bloklar ve
kağıt nesneleri model kapsamını büyütebiliyordu. CAD model kapsamı artık gerçek
yay aralığı ve yerleştirilmiş bloklardan ortak hesaplanır. Blokların tabanı,
dönüşü, ayna ve farklı X/Y oranları kullanılır; yerel kutular önbelleğe alınır,
her yerleştirmede bütün blok patlatılmaz. İç içe döndürülmüş kutular muhafazakâr
olabilir; yazı glif taşmaları ölçülmez. Pencere bu değeri **yaklaşık model boyutu**
olarak metreyle gösterir; doğrulanmış bina boyu olarak sunmaz.

İçe aktarılanlara yakınlaşma da aynı kutuları kullanır. Sabit en az 20 m kapsam
ölçülebilir parçalar için kaldırılır; tek nokta proje biriminde bağlam alır.
Mevcut kameranın 5000 mantıksal piksel/metre üst sınırı korunur; 4 mm parça
sınırda 20 piksel görünür, ekrana sınırsız yakınlaşma iddiası yoktur.

## Gerçek kullanıcı dosyasının belirsizliği

Yerel Downloads `2-1-ev-kat-plani.dwg`, `$INSUNITS=4` (mm) bildirir.
`g.don1050x2100` tanımı da mm'dir: fiziksel boyu 2,1 m, mevcut ham
INSERT çarpanı 0,1'dir. Dosyanın gerçek fiziksel birimini kullanıcı bilmiyor.
Bu nedenle başlık veya bina adı üzerinden sessiz santimetre tahmini yapılmaz.

- Başlığa göre mm: model yaklaşık 1,419 × 1,153 m; eski yatak 0,21 m,
  yeni ölçek 1 yatak 2,1 m.
- Açık cm seçimi veya **yalnız bir doğrulama örneği olarak** 1250 → 12,5 m:
  model yaklaşık 14,189 × 11,527 m; eski ve yeni yatak 2,1 m.
- Açık m seçimi: eski yatak 210 m, yeni yatak 2,1 m.

Bu karşılaştırma cm yorumunun blokları eşitlediğini kanıtlar; evin gerçek
ölçüsünü kanıtlamaz. Yanlış birim bildirimi ancak güvenilir bilinen uzunluk
veya kaynak belgenin gerçek birim bilgisiyle kesinleştirilebilir. Özgün DWG
üzerine yazılmaz; önceden yanlış alınmış belge sessizce yeniden ölçeklenmez.

## Başvuru

- [Autodesk INSUNITS](https://help.autodesk.com/cloudhelp/2025/ENU/AutoCAD-Core/files/GUID-A58A87BB-482B-4042-A00A-EEF55A2B4FD8.htm)
- [Autodesk blok birimleri](https://help.autodesk.com/cloudhelp/2025/ENU/AutoCAD-LT/files/GUID-6C46049D-8636-442D-8BAC-CF4FD515FDC0.htm)
- [Autodesk farklı boyda eklenen bloklar ve bilinen uzunluk](https://www.autodesk.com/support/technical/article/caas/sfdcarticles/sfdcarticles/Blocks-xrefs-or-raster-images-are-scaled-when-inserted.html)

## Kabul kanıtları

Bağımsız Python başvurusu 64 DXF ve 51 DWG komut durumu içerir. Yeni 20
referans durumu mm/cm/m/in/ft ve dört proje türünü çaprazlar; dört yeni ret
sınırı vardır. Kaynak blok tabanı, iç içe blok, farklı X/Y, mevcut örneklerin
korunması ve kalibrasyon varyantları çekirdek kabulüne dahildir. Ayrı regresyon
yay aralığını, kullanılmayan tanımı ve kağıt/model ayrımını denetler.

Native application takımında 135, formats takımında 111 test geçti; sırasıyla
22 ve 7 mevcut özel performans/platform denemesi atlandı. MCP plan/execute,
undo ve çakışan kalibrasyon reddi geçti. Gerçek DWG penceresi açık/koyu ve
18 puntoyla görüntülendi; Ölçüyü uygula gerçek fare tıklamasıyla çalıştırıldı.
Üretilen pencere ve çizim görüntüleri açılıp incelendi.

Son kapılar: iki masaüstü ölçek regresyonu, native clippy, katalog (3), TS
sözleşme üretimi (876), SDK/manifest ve bağımlılık yönü geçti. Tam Python takımı
249 testle geçti; son eklenen `Document.import_dxf(scale_reference=...)`
kolay yolunu ve tipli planı karşılaştıran üç test ayrıca yeniden geçti.

Gerçek DWG, CAD mm/CAD cm/CBS olmak üzere üç projede otomatik/mm/cm/m/bilinen
ölçü seçenekleriyle toplam 15 kez alındı. Her birinde yeni yatak eklendi,
kaydedilen belge yeniden açıldı, DXF bağımsız ezdxf 1.4.5 okuyucusuyla çözüldü
ve eski/yeni bloğun görünüş geometrisinin uzun kenarı metreye çevrilip ölçüldü.
DXF çıktısı proje birimini kullanabildiğinden okuyucunun `$INSUNITS` dönüşümü
hesaba katıldı. Yukarıdaki 0,21/2,1/210 m değerleri doğrulandı; gösterim birimi,
undo/redo ve kaynak dosyanın SHA-256'sı korundu. Bu, bağımsız DWG codec veya
gerçek AutoCAD'de düzenleme testi yerine geçmez (CAD-IO-20 kullanıcı kararıyla
bu oturumun kapsamı dışındadır).

Optimize formats/dxf/sheet WASM derlemeleri ve mevcut pafta WASM smoke takımı
da geçti. Web arayüzü/test/build çalışmaları ADR 0228 uyarınca ertelenmiştir;
yalnız ortak TS sözleşmeleri ve katalog yenilendi.

Şerit erişimi de eklendi: CAD Ekle / İçe aktar grubunda DWG düğmesi, her iki
proje türünün Dosya / İçe aktar ve CBS Veri / Dosya alışverişi / İçe aktar
menülerinde DXF'in önünde DWG. Hepsi mevcut `file.import.dwg` komutunu kullanır.
Katalog ve 1100 px uyarlanır yerleşim için 9 mevcut test, masaüstü derlemesi
ve clippy geçti. Açık/koyu tema, 18 punto ve açılmış menülerin son ekran
görüntüleri incelendi. Donmuş web envanteri değiştirilmedi; masaüstü ported
listesindeki mevcut `sheet.drawingSet` üretim eksikliği de yenilendi.
