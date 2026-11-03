# ADR 0880 — Mevcut kılavuzda sabit aralıklı tekrarları tanıma

Tarih: 3 Kasım 2026. Durum: uygulandı; native, Python, MCP ve gerçek masaüstü kabulü doğrulandı.
CAD-ADV-07 dilimi. Kaynak türü genişletmesini veya bütün 07'yi kapatmaz.

## Karar ve dayanak

[AutoCAD ARRAYPATH](https://help.autodesk.com/cloudhelp/2027/ENU/AutoCAD-Core/files/GUID-D36C46CD-4E17-4A16-A387-C0B158EA5A9E.htm)
kılavuz, başlangıç/taban, sabit aralık ve teğete göre hizalamayı ayrı ele alır.
[BricsCAD ARRAYDETECT](https://help.bricsys.com/en-us/document/command-reference/a/arraydetect-command?version=V27)
bulunan örüntüyü kullanıcıya ayrı inceletip seçilen dönüşümü sunar.
KentOS bunlardan hareketle mevcut `ArrayLayout::Path` ve metadata-only
benimsetmeyi kullanır; ikinci bir yol yerleşim motoru kurmaz.

`cad.arrays.find(kind="path", path=<mevcut UUID>)` açık kılavuzla çalışır.
Kılavuzun kayıtlı başlangıcı ve yönü kullanılır. Kaynak Insert taban noktası
başlangıçta, verilen mutlak tolerans içinde olmalıdır. Kaynak kendiliğinden
başlangıca taşınmaz, kılavuz çevrilmez. Bu sürüm bilinmeyen bir yolu
noktalardan uydurmaz; ters yön, gelişigüzel başlangıç kayması veya ofseti
çıkarmaz. Kılavuz tek parçalı 2B çizgi, yay, çember veya en çok 4096 köşeli
çoklu çizgidir. Kapanış ve analitik bulge'lar korunur; Z/M, mahal rolü,
delik ve ek parça sessizce atlanmaz.

## Yöntem ve güvence

- Ortak statik şablon denetimi; aynı blok/görünüş/ayna/ölçek. Yönler önce
  dışlanmaz: hem yönü koruma hem kılavuzun teğetine göre dönüş denenir.
- Mevcut analitik `path_of`, `nearest_s` ve `point_at_s` ile yay uzunluğu
  istasyonları bulunur. Dünya X/Y kareleri veya render tessellation'ı
  üzerinden aralık çıkarılmaz. Hesap Float64'tür; tolerans metreyle mutlak
  kalır, birim/ölçek değiştirilmez.
- Gözlenen komşu istasyon farkları aralık önerir; gözlenen tam açıklıkla
  yeniden denetlenir. En az üç normal öğe ve gerçekten dolu bir komşu çift
  gerekir. Her ikinci yer doluysa görünmeyen yarım aralık uydurulmaz.
- Son kabul gerçek `item_placements` ve `arrays_adopt::inspect` üzerindedir.
  Konum, açı, ölçek ve ayna aynı mevcut kapıdan geçer. Bir yere birden fazla
  uygun UUID düşmesi belirsizliktir; rastgele seçim yapılmaz. Yanlış yöndeki
  kök eşleşmez, kendiliğinden korunmuş sayılmaz. Son doğrulanmış dolu yerden
  sonrası varsayılmaz; yanlış yönlü uç, kuyrukta sahte boş yer üretemez.
- Öneri `spacing=Some(...)` ile sabit aralıktır. Kılavuz uzayınca otomatik
  yeni öğe ekleme veya adede göre bütün yolu yeniden bölme niyeti tahmin
  edilmez. Bu davranışlar mevcut açık dizi düzenlemelerinden ayrı karardır.
- En çok 2048 kapsam nesnesi, 100 sonuç, 10000 yol yeri ve ortak iki milyon
  iş bütçesi. Analitik kenar projeksiyonu ve yerleşim üretimi bütçeye dahildir.
  Sonuç sınırı kısmi olarak görünür; iptal, geç iş ve revizyon koruması ortak
  native Job'dadır. Arama ve inceleme çizime yazmaz.

## Arayüz, Python ve MCP

Bloklar panelinde aynı **Dizileri bul… / DIZIBUL** penceresi kullanılır.
Bir kaynak blok ve bir kılavuz birlikte seçilmişse yol ailesi ve kılavuz
hazır açılır. Yalnız blok seçilmişse açık **Kılavuz yol** listesi vardır;
ilk 2048 görünür kılavuz sınırı açıklanır. Kart dolu/boş sayılarını, sabit
aralığı ve yönü açıklar. İncelemede kılavuz, kayıtlı başlangıç/yön ve yer
sırası; açık koruma ataması ve tek Undo gösterilir. Hata panelinde de yeni
kaynağa/ayarlara dönülebilir. Yeni ribbon düğmesi veya dosya şeması yoktur.

Python `ArrayFindKind.PATH`, `ArraysFind.path`, `PathArrayLayout` ve
`Document.start` aynı uygulamayı kullanır. MCP aynı sorgu ve korunan
plan/adopt akışıdır. Katalog açıklamasındaki eski kutupsal kapsam metni de
ortak kaynaktan düzeltildi; iki yeni ret kodu yetenek kataloğuna eklendi.

## Kabul kayıtları

Native uygulamanın tam ana entegrasyon koşusunda 415 test geçti, 37 elle
çalıştırılan test atlandı. Son koruyucu kontrollerle 29 tanıma testi ayrıca
geçti. 47 bağımsız ortak durum native/Python koşucularında, 15 Python dizi,
altı ilgili MCP, 12 panel, dört katalog ve 1050 TS bağlama üretimi testi
başarılıdır. SDK/bağımsız veri/946 satırlı yüzey manifesti tutarlıdır.
Native uygulama/MCP/Python/masaüstünün tüm hedeflerinde Clippy başarılıdır.
İlk filtreli tam koşudaki boş masaüstü fixture seçimi ve eksik yeni ret
listesi hatası saklanmaz; filtre kaldırılıp katalog düzeltilince tam koşu
geçmiştir. Son görünür düzeltmeler ayrıca tekrar denetlenir.

Gerçek 2200×1364 pencerede bir kaynak ve kılavuz Shift ile birlikte seçildi;
yol ayarı hazır açıldı. 2000 mm aralık, altı dolu/iki boş yer ve köşe teğeti
aynıdır. Kaymış yedinci öğe açıkça boş yere korunmuş olarak bağlandı.
Vazgeçme/bağlama Undo ve Redo dosyaları ilgili başlangıç/son dosyalarına
bayt bayt eşittir. Bağımsız KCAD okuyucusunda yalnız dizi kaydı eklenir;
sekiz kök, UUID, tanım, sıra ve ayar aynıdır.

Gerçek **Diziyi düzenle → Aralık** ile 2000→1500 mm değişti; kök kimlikleri,
boş yer ve korunan öğe konumu kaldı. Kılavuz köşesi gerçek fareyle sürüklendi;
normal köklerin yeni konum ve açıları bağımsız `math` hesabıyla doğrulandı.
Kılavuz ile bağlı yerleşimler tek Undo'dur. Aralık ve kılavuz Undo/Redo'nun
dört dosya çifti birebirdir. Başlangıçta olmayan kaynak gerçek son programda
okunur gerekçeyle reddedildi. Ham ekranlar 425 ve sonrası ile
`path-gui-acceptance.json` depo dışında `/tmp/kentos-cad-adv04-2026-11-01`
altındadır; repoya eklenmez.

Son teslim derlemesinde hata → **Yeni kaynak** → başarılı inceleme akışı
432–433 ekranlarıyla tekrarlandı. Yön şeması ve görünür kılavuz listesi sınırı
doğrulandı. Aynı programdan yeniden bağlama/Undo/Redo dosyaları bağımsız
okuyucuda doğrulandı: yalnız dizi metaverisi eklenir, iki boş yer kalır;
ret kaydı ve Undo başlangıçla, Redo bağlanmış kayıtla bayt bayt aynıdır.
Son arayüz düzeltmesinin 12 panel testi, derlemesi ve Clippy'si de geçti.
