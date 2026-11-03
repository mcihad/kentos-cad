# ADR 0881 — Tekrarlardaki yazı değerleri ve iç blok bağları

Tarih: 3 Kasım 2026. CAD-ADV-07 kaynak genişletmesi.

## Dayanak ve karar

[AutoCAD 2026 Search and Convert](https://help.autodesk.com/cloudhelp/2026/ENU/AutoCAD-WhatsNew/files/GUID-3B3A51B6-FC61-40AB-9EEA-597426339131.htm)
yazı çeşitlerini ayrı filtreyle arar ve değerleri blok özniteliklerine taşır.
[BricsCAD BLOCKIFY](https://help.bricsys.com/en-us/document/command-reference/b/blockify-command?version=V26)
kaynağı, kapsamı ve dönüştürülecek eşleri açıkça seçtirir. KentOS'ta benzer
geometri, verinin silinmesi için gerekçe değildir. Arama ile veri eşlemesi
iki ayrı kullanıcı kararıdır; mevcut blok ve öznitelik modeline yazılır.

## Tanıma ve sınırlar

`cad.blocks.find` tek satırlı, en çok 4096 baytlık düz yazıları ve aynı
statik tanımın tekdüze ölçekli yerleştirmelerini destekler. Yazının yönü,
yüksekliği, hizalaması, genişlik katsayısı ve biçimi eşitlikte kalır.
Dünya koordinatına küçük yön vektörü ekleyip çıkarmak yerine kayıtlı açı
ve boyut oranı kullanılır; büyük koordinatta kısa yazı için açı hassasiyeti
korunur. Çekirdek FP64'tür; render, proje ölçeği ve birimler değişmez.

`textVariants` yalnız sözcük farkına; `attributeVariants` yalnız tanımda
bildirilmiş blok etiketlerinin değer farkına açık izin verir. Her ikisi
varsayılan kapalıdır ve açıkken ayrı yoluyla uyarı raporlanır. Diğer kök
verileri varsayılan katman/görünüş denetiminde kalır. Aynı görünüşteki
farklı blok tanımları sırf benziyor diye birleştirilmez. `geometryOnly`
bu veri kaybı kapısını dönüşüm için açamaz.

Etiket/alan/annotative/eğri yazısı, çok satır, zengin metin, ayrı Y ölçeği,
kırpma, dinamik parametre ve mimari kayıt sessizce atlanmaz. İç tanımlar
16 düzey, döngüsüz ve statik olmalıdır. Her iç parça aynı destek denetiminden
geçer. Açılımın çoklu kullanımı, yazı/çoklu çizgi boyutu ve önbellekteki
tanımlar toplam 200000 iş birimiyle sınırlandırılır; iptal denetimi vardır.
Kapsam boş olsa da kaynak tanımı denetlenir. Aday sınırı yerleştirme
noktasının yanında gerçek açılmış parçaları ve yazıları da kapsar.

## Dönüştürme ve ilişki koruması

- `textToAttributes` ayrı ve varsayılan kapalıdır. Açılırsa kaynak çizim
  sırasının sonundaki temsil edilebilir yazılar `TEXT_1`, `TEXT_2`… etiketleri
  olur. Her yerleştirme kendi gerçek sözcüğünü saklar. Tanımın sonunda
  çizilen öznitelikler ara geometriyi örtemeyeceği için araya geometri
  girmiş yazı sırası, özel font/maske/görünüş ve kök verisi atomik ret alır.
- İnceleme gerçek yeni Insert'i açar; her öznitelik yazısının dünya konumu,
  yönü, yüksekliği, içeriği ve biçimini eski Text ile tekrar karşılaştırır.
  “Yaklaşık aynı” görünüş veya kaybolmuş değer kabul edilmez.
- `mapping.attribute` hedef etikettir; bu satırın `path=[]` olması sahte bir
  blok alt nesnesi yolu anlamına gelmez. Normal parçaların sıfır tabanlı
  yolları eskisi gibidir. Özniteliğe çevrilecek yazıya bağlı ölçü, mevcut
  SubRef modeli bu yaprağı temsil etmediği için açıkça reddedilir.
- Statik iç blok kendi tanımına referans olarak korunur; tanımlar patlatılmaz
  veya değiştirilmez. Tanımlı öznitelik değerleri eşitse iç Insert'te aynen
  kalır. Farklı değerler aramada görülebilir ama bu dönüşümde bir değere
  indirgenmez: yeniden doğrulama reddeder. Genel kök verisi taşınmış sayılmaz.
- İç yaprağa bağlı sıradan ölçüde `yeniParçaYolu + eskiİçYol` kullanılır.
  Eski/yeni gerçek yaprak ve nokta denetlenir; kayıtlı koordinatı güncel
  değilse ölçü sessizce düzeltilmez. Field, tarama, grup, tablo ve diğer
  temsil edilemeyen kök bağları mevcut kapsam taramasından gerekçeli ret
  alır; hiçbir bağ kopartılmaz.

## Yüzeyler ve kabul

Aynı native komut masaüstü arka plan işi, Python SDK, Job ve MCP tarafından
kullanılır. Arama penceresinde iki görünür varyant seçeneği; dönüşüm
penceresinde ayrı değerleri öznitelik olarak koru seçeneği vardır. Seçenek
veya ad değişince önceki inceleme uygulanamaz. Detaylarda öznitelik etiketi
ile normal parça yolu ayrılır. Katalog, TS ve Python tipleri ortak kaynaktan
üretilir. Yeni komut veya KCAD şeması yoktur; tek Undo, revizyon ve korunan
plan protokolü devam eder.

Tam native uygulama koşusunda 420 test geçti, 37 elle çalıştırılan test
atlandı; çekirdek 16, tüm MCP 76, ilgili panel 8 test geçti. Kaynak ve dönüşüm
63 bağımsız ortak durumda native/Python üzerinden doğrulandı. SDK üzerinde
12 doğrudan test ve ortak durum koşucusu başarılıdır; katalog 4, TS üretimi
1050 test ve 946 satırlı yüzey manifesti tutarlıdır. Native/MCP/masaüstünün
tüm hedeflerinde Clippy ve teslim derlemesi başarılıdır. Geçerli Arimo kalın
yazı ile kayıpsız ret ayrıca sınandı; ilgili native alt küme 19 testtir.
İlk tam koşuda yetenek kataloğunun yeni Text/Insert türlerini bildirmediği
bulundu; katalog ve biçim örneği düzeltilerek tam koşu tekrar geçmiştir.

Gerçek 2200×1364 pencerede dört döndürülmüş çizgi/yazı çifti varsayılan
aramada tek, açık yazı varyantıyla dört eş verdi. Öznitelik seçeneği kapalı
ret, açıkken inceleme, parça/etiket eşlemesi, dönüşüm ve mevcut inspector'da
yalnız A01→A99 düzenlemesi fare/klavyeyle sınandı. A00–A03 konum/yönleri,
kök UUID'leri ve 2000 mm ölçü bağı bağımsız KCAD okuyucusu + `math` ile
doğrulandı. Vazgeçme/Undo başlangıçla, Redo son kayıtla bayt bayt aynıdır.
Değer düzenlemesinin Undo'su da önceki bloklu kayıtla birebirdir.

Ayrı iki statik iç blokta N-01 değerleri, özgün tanımlar ve 4000 mm ölçü
korundu; iç ölçü yolu [0,0]→[0,0,0], eski/yeni dünya noktaları aynıdır.
Aynı programda Undo/Redo dosyaları birebirdir. Adayın Göster eylemi gerçek
içerik sınırlarına yakınlaşır. Kısa dock'ta ikinci, ilgisiz blok sayacı satırı
arama sırasında kaldırılarak Yeni kaynak/Ayarlar kontrolleri görünür kaldı.

Son teslim programında her iki dönüşüm ve Undo/Redo tekrarlandı. Geçerli
kalın Arimo yazının temsil edilemeyen biçimi ve N-01/N-02 iç değerlerinin
birleştirilmesi ret aldı; kaydedilen iki dosya kendi başlangıçlarıyla bayt
bayt aynıdır. Eski belgeden kalan sonuçlar yeni çizimde uygulanamadı;
Bloklara dön/Yeni kaynak akışı kullanıldı. `source-delivery-gui-acceptance.json`,
`text-gui-acceptance.json`, `nested-gui-acceptance.json` ve 434–453 ham ekranlar
`/tmp/kentos-cad-adv04-2026-11-01` altında, depo dışındadır.
