# ADR 0874 — Mevcut bloklarda yazısız doğrusal örüntü önerileri

Tarih: 3 Kasım 2026. CAD-ADV-07'nin doğrusal aday bulma dilimi.

## Ürün dayanağı ve sınır

[BricsCAD V27 ARRAYDETECT](https://help.bricsys.com/en-us/document/command-reference/a/arraydetect-command?version=V27)
bulunan örüntülerin ayrı ayrı incelenip seçilmesini sağlar.
[AutoCAD for Mac 2027 Detect and Convert](https://help.autodesk.com/cloudhelp/2027/ENU/AutoCAD-MAC-WhatsNew/files/GUID-0876F8BC-9ED5-4EBB-A19C-FF019BAA3510.htm)
bulunan kümeleri, birincil örneği ve incelemeyi ayrı adımlar olarak sunar.
Bunlar kullanıcı akışının dayanağıdır; kapalı kaynak geometrik algoritmaları
incelenmiş veya KentOS'a aktarılmış değildir.
BricsCAD'in [resmi panel görseli](https://static.bricsys.com/help/images/V27/en_US/CMD_ARRAYDETECT_01.png)
tarayıcıda görsel olarak da incelendi: küçük örüntü resmi, satır/sütun/öğe
sayısı, aday seçimi ve ayrı dönüştürme eylemi aynı panelde gösteriliyor.

KentOS'un ADR0873 bağlama komutu incelenmiş yerleşimi kabul eder, örüntü
bulmaz. `cad.arrays.find` bu boşluğun önce doğrusal kısmını doldurur:
aynı statik blok tanımı, katman/görünüş, yön, ayna ve tekdüze ölçeğe sahip
verisiz yerleştirmeler arasındaki doğrusal düzenleri önerir. Tanımların
geometrik benzerliğini veya anlamca eşitliğini varsaymaz. Grid, kutupsal
ve yol bulma ile grafik inceleme paneli bu API dilimiyle tamamlanmaz.

## Açık sözleşme

- `source` mevcut bir yerleştirme UUID'sidir; bulunan sıranın ortasında
  olabilir. `uids` yoksa belge, boşsa yalnız kaynak aranır. Kaynak her
  durumda kapsama dahil edilir; toplam sınır 2 048 kök nesnedir.
- Statik kaynak denetimi ADR0873 ile aynı işlevden gelir. Dinamik/xref,
  mimari kayıt/nesne verisi, kırpma ve öznitelik tanımı gizlice atlanmaz.
  Kaynak zaten bir dizinin üyesiyse bütün istek gerekçeli ret alır.
- Başka kökler için `excluded` UUID→neden haritası döner: `not_insert`,
  `template_differs`, `layer_locked`, `array_member`. Uyumlu `eligible`
  listesi UUID sırasındadır. Uyumlu olup adaya girmeyen öğeler eşleşmemiştir;
  kendiliğinden `kept` yapılmaz veya çizimden silinmez.
- Konum toleransı model metresiyle, varsayılan 0,000001; üst sınır 0,001.
  Bağlama komutunun bağımsız açı/ölçek toleransları aynen geçerlidir.
- Her öneri `adoption: ArraysAdopt`, dolu/eksik sayıları ve doğrulanan en
  büyük konum sapmasını verir. Yer sırası ilk dolu uçtan son dolu uca gider;
  boş yerler açıkça boş kalır. Çakışan öneriler `overlaps` ile işaretlenir;
  aynı nesne iki ayrı diziye bağlanamaz.
- Sorgu hiçbir kök, seçim, blok veya dizi kaydı ve Undo adımı üretmez.
  Kullanıcı aday girdisini inceler; uygulama ayrı `cad.arrays.adopt` çağrısıdır.
  Girdideki `expectedRevision` bulunma anının revizyonudur; eski öneri yeni
  çizime yazılmaz. Daha ayrıntılı incelemede bağlama komutunun `planId`
  beklentisi kullanılır.

## Geometrik yaklaşım ve belirsizlik

Hesap Float64 ile kaynağa göre yerel farklardan yapılır. Kaynak ile diğer
uyumlu yerleştirmeler yön önerir. Yön çevresindeki noktalar dik sapmayla
gruplanır; izdüşüm sırasındaki gerçek komşu uzaklıkları olası aralıklardır.
Her nokta tam sayıdaki bir yere mutlak model toleransıyla yerleşmelidir.
Önerilen bütün pozlar mevcut `domain::arrays::placements` motoru ve
`arrays_adopt::inspect` ile tekrar doğrulanır. Toleransa yakın olduğu için
hiçbir mevcut nesne taşınmaz.

En az üç dolu yer ve gözlenmiş bir komşu çift gerekir. Yalnız 0, 4, 8, 12
metre gözleniyorsa 4 metre aralık önerilir; 2 metre aralıklı, her ikinci
yeri eksik bir sıra gözlemden çıkarılamaz. İki nesne tek başına güçlü bir
örüntü sayılmaz. Aynı yere iki UUID düşerse biri rastgele seçilmez; aday
reddedilir. Tam kapsayıcı bir öneri varken onun öz alt kümeleri bastırılır;
kesişen yönlerdeki alternatifler kullanıcıya bırakılır.

## İş sınırları ve yüzeyler

İş iki milyon ölçüm/karşılaştırma denemesini veya istenen 1..100 aday
sınırını aşınca `truncated` ve `recognition_truncated` uyarısı verir.
Kısmi tarama bütün çizimi bulmuş gibi sunulmaz. Döngüler iptali kontrol
eder; native Job, Python ve MCP aynı handler'ı kullanır. Büyük kapsamın
GUI iş parçacığında eşzamanlı taranması kabul edilmez.

## Doğrusal dizinin görünür aralık düzenlemesi

Mevcut **Diziyi düzenle** aracı doğru düzeninde yalnız Adet sunuyordu.
**Aralık (A)** aynı `cad.arrays.edit` yazma yoluna eklenir. Girilen pozitif
uzunluk, dizinin güncel yönünde iki komşu yer arasındaki gerçek uzaklıktır;
`dx`/`dy` yönü değiştirilmez. `fit=false` için vektörün boyu, `fit=true`
için toplam vektörün boyu `aralık × (adet − 1)` olur. Mevcut fit anlamı
sessizce değiştirilmez. Birim, aracın mevcut birimli sayı okuyucusundan
gelir. Sıfır/negatif aralık yazılmaz; sonlu poz ve ortak yerleşim denetimi
yerindedir. Geçerli değişim aynı öğe UUID'leriyle tek Undo/Redo'dur.

## Kabul kapısı

İki boş yer, elle kaymış eşleşmeyen öğe, iç kaynak, büyük koordinatta
küçük aralık, çakışan UUID, kesişen alternatifler, belirsiz temel aralık,
ters kapsam sırası, boş kapsam, retler ve iptal sınanır. Her dönen öneri
bağlama komutundan geçmelidir; sorgu öncesi/sonrası dosya aynı olmalıdır.
Gerçek uygulama kontrolü Python konsoluyla sorgu/inceleme/bağlama ve
mevcut Diziyi düzenle aracı üzerinden yapılır. Bu kontrol grafik aday
paneli teslim edilmiş sayılmaz.
