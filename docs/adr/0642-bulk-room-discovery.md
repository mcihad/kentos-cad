# 0642 — Önizlemeli toplu mahal bulma

Tarih: 25 Ekim 2026. ARCH-SPACE-01 sürer; kolon ve sanal sınır gibi kalanlar
bu dilimin varlığıyla tamamlanmış sayılmaz.

## Ürün davranışı

`cad.spaces.discover` ile bir açık katın veya **Tüm katlar** duvar ağındaki
kapalı boşluklar bulunur. Kat verilmezse mevcut Mahal/follower'ın bütün
çizim kuralı kullanılır; eşit kot farklı kat kimliklerini birleştirmez.
`bounds` kaynak duvarları kesmez: doğrulanmış iç temsil noktası dikdörtgenin
içinde kalan odaları önerir. Temsil noktası etiket yerleşiminden alınır,
analitik içeride testiyle doğrulanır; gerekirse analitik yüz probu kullanılır.
Sınır bulma ve duvar gövdesi sınıflandırması yay örneklemesine dayanmaz.

Plan her adayın sınırını, aynı yüzü kullanan mevcut mahal UUID'lerini ve
kısmen çakışan elle çizilmiş/kopuk mahal UUID'lerini ayrı verir. Mevcut
mahallerin adı/numarası/kullanımı değiştirilmez; isim eşitliği oda kimliği
değildir. Seed'i aynı boşlukta olan mahal mevcuttur; farklı yöntemin brüt
sınırlarının duvar içinde örtüşmesi ikinci oda çakışması sayılmaz. Elle
çizilen sınır analitik fark/kesişim ile karşılaştırılır: eşit sınır korunur,
kısmi çakışma bütün yazmayı engeller. Plan çakışmayı göstermeye devam eder.

Execute yalnız yeni adayları mevcut `cad.spaces.create` yazıcısıyla tek
**Mahal** adımında ekler; komut günlüğünde tek `cad.spaces.discover` çağrısı
vardır. Tek undo/redo bütün yeni mahalleri kapsar. Tekrar çalıştırmak yeni
nesne veya boş undo adımı üretmez. `expectedRevision` eski önizlemeden
yazmayı engeller. Hedef katmanın kilidi ve tipli zorunlu alanları denetlenir.

Gizli/kilitli kaynak duvarlar da ağın parçasıdır; kamera veya katman
görünürlüğü değişimi odanın kaynağını değiştirmez. Kapı/pencere açıklıkları
mevcut duvar kuralıyla kapalı sayılır, fiziksel duvar kopukluğu yeni bir
gap toleransıyla kapatılmaz. Bir çağrıda kaynak duvar ve oda sayısı ayrı
ayrı 10.000 ile sınırlıdır; bu sayı kesin RAM/süre garantisi değildir.

## Ortak geometri ve sorgu ömrü

`space_index::BoundaryIndex` aynı analitik joined duvar yüzleriyle FaceIndex'i
bir kez kurar. `faces_with_probes` her kapalı yüzün gerçek analitik probunu
verir. Sol yüz ters, sağ yüz düz ve uçları kapanacak yönde alınarak duvar
malzemesinin yönlü sınırı kurulur; yerel koordinatta winding sayısı sıfır
olmayan hücreler mahal değildir. Render için örneklenmiş `Drawn.fill`
sınıflandırmada kullanılmaz. Çözülemeyen kaynak veya uç yönü, sessizce oda
üretmek yerine açık ret verir. Net/Eksen/Brüt mevcut `space::by_method` yolu
kullanır; yöntem hesabının standarda uygunluğu konusunda yeni iddia yoktur.

`SpaceSearch` tek immutable belge ödüncüne bağlı, geçici bir sorgu oturumudur.
Kaynaklar değişirken yaşayamaz. Aynı sıralı kaynak Slot kümesi aynı dizini
kullanır; farklı yakın kümeler birleştirilmez. Eski yakın arama/tam arama
önceliği ve büyütme sırası korunur. Eski tek seed sorguları malzeme dizini
oluşturmaz. Çoklu oluşturma ve follower geçişi bu oturumu paylaşır; follower
önce bütün sonuçları hesaplayıp aynı adıma uygular. Mahal düzenlemesi duvar
geometrisini değiştirmediğinden sonuçlar birbirinin kaynağı değildir.

Toplu keşif ağı bir kez kurar; komutun doğrulaması/yazması ve follower'ın
farklı yakın kümeleri ayrıca iş yapar. Çekirdek sorgu hızını uçtan uca
masaüstü gecikmesi veya GPU süresi gibi raporlamayın.

## Arayüz ve diğer yüzeyler

Mimari/Çiz içindeki ayrı **Toplu mahal bul** aracı mevcut tek mahal aracını
değiştirmez. Kapsam, kat, yöntem ve Yeni/Var/Çakışan sayıları görünürdür.
Yeni sınır vurgu rengi, mevcut sınır kesik yardımcı rengi, çakışan sınır
kırmızı gösterilir. Bölge iki köşeyle seçilir; ikinci köşe yazmaz. Enter veya
mevcut sağ tuş onayı aynı komutu uygular; Escape belgeye dokunmadan çıkar.
Önizleme revision/hedef katman değişmedikçe fare hareketiyle tekrar çözülmez.
Python SDK ve MCP aynı plan/execute sözleşmesini kullanır. KCAD mahal
şeması, SI metre politikası, import ve render motoru değiştirilmez.

## Üretici kaynakları ve çıkarım

Autodesk [associative space üretimini](https://help.autodesk.com/cloudhelp/2022/ENU/AutoCAD-Architecture/files/GUID-3479D10A-3594-4D0F-9193-9F2851D0AAA7.htm)
tek iç nokta veya toplu üretim olarak açıklar; var olan odalarda yeni oda
üretmek ayrıca açılan bir seçimdir. [Güncelleme belgesi](https://help.autodesk.com/cloudhelp/2025/ENU/AutoCAD-Architecture/files/GUID-F51C80FE-3B37-43E8-8A20-2850D07AFDD9.htm)
sınır nesnelerinin kalıcı rolünü ve sürümler arası filtre farkını belirtir.
[Revit oda belgesi](https://help.autodesk.com/cloudhelp/2025/ENU/Revit-ArchDesign/files/GUID-65668F88-4429-4992-AA0D-1C4FBDCFA650.htm)
kapalı bölge ve yinelenen oda uyarısını ayrı tutar.
KentOS çıkarımı: kaynağı, mevcut oda ve çakışma durumunu yazmadan göstermek;
kısmi kaynak filtresini yalnız ilk çalıştırmada kullanıp follower'da kaybetmemek.
KentOS burada bu ürünlerin bütün alan standartlarını veya 3D oda modelini
uygulamış değildir.

## Kabul ve kalanlar

[Kabul kaydı](../baseline/2026-10-25-architecture-space-discovery/README.md)
34 ortak komut senaryosunu, ham koşuları, gerçek görüntüleri, bağımsız ölçü/korunum ve performans
ölçümlerini tutar; geçmeyen veya yapılmayan kapı orada açık kalır.

Seçilmiş alt kaynak kümesi veya yalnız atanmamış duvar kapsamı için kalıcı
sınır-kaynak modeli gereklidir; bu komut o süzgeci ekleyip sonradan kaybetmez.
Kolon çıkarımı, kalıcı sanal sınır, kapanmayan sınırı kaynak uçlarıyla sahnede
tanıma, tam GUI matrisi ve diğer ARCH-SPACE-01 koşulları ayrıca açıktır.
