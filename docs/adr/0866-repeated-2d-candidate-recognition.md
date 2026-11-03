# ADR 0866 — Tekrarlanan 2B bileşenleri önce yazısız ve incelemeli bul

Tarih: 2026-11-03. CAD-ADV-07'nin ilk dilimi; bütün madde tamamlanmış değildir.
Kapsam: ortak analitik çekirdek, native komut/masaüstü, Python ve MCP.
Takip teslimi [ADR 0867](0867-analytic-polyline-repeat-recognition.md)
tek parçalı 2B çoklu çizgileri de aynı sorguya ekler; aşağıdaki üç ilkel tür
ilk teslimin tarihsel sınırıdır. Güncel kullanım sınırı kullanım/API belgesindedir.

## Kaynaklar ve mevcut kod

[BricsCAD V27 BLOCKIFY](https://help.bricsys.com/en-us/document/command-reference/b/blockify-command?version=V27)
kaynak ve arama kapsamını ayırır, geometriye göre karşılaştırmayı açık seçenek
yapar ve bulunan kümeleri kullanıcıya seçtirir. Parametrik ilişkileri koruma
ayrı bir politikadır. [V27 ARRAYDETECT](https://help.bricsys.com/en-us/document/command-reference/a/arraydetect-command?version=V27)
bulunan örüntülerden kullanıcının seçtiklerini diziye dönüştürür; kaynak bir
veya çok nesne olabilir. KentOS tasarım çıkarımı: tanımak, semantik eşitlik
veya kayıpsız dönüştürme yetkisi değildir. Önce aday ve kimlik eşlemesi gerekir.

[AutoCAD for Mac 2027 Detect and Convert](https://help.autodesk.com/cloudhelp/2027/ENU/AutoCAD-MAC-WhatsNew/files/GUID-0876F8BC-9ED5-4EBB-A19C-FF019BAA3510.htm)
da ana örneği değiştirme, üyelerini düzenleme, bulunan örnekler arasında
gezinme ve seçilenleri ayrı onayla dönüştürme adımlarını ayırır.
[AutoCAD 2026 Search and Convert](https://help.autodesk.com/cloudhelp/2026/ENU/AutoCAD-WhatsNew/files/GUID-3B3A51B6-FC61-40AB-9EEA-597426339131.htm)
metin farklılıklarını ayrıca ele alır. KentOS'ta bu metin varyantı henüz
uygulanmış sayılmaz; ilk dilim yerel ve deterministik analitik tanımadır.

KentOS'ta `AssocArray`/`ArrayItem`, `array_assoc`, `arrays_edit` ve domain
`follow_arrays` zaten var. Silinmiş yer `uid=None`, elle değiştirilmiş öğe
`kept` ile saklanıyor; yol değişimi ve tek Undo mevcut. İkinci bir dizi modeli
kurulmaz. `blocks_define::copies` tanım içi kimlik üretir ve bazı bağları
bırakır; bu yol tanınan bütün kümelere kontrolsüz tekrarlanamaz.
`copy_links::links_to` bağlı kök ölçü/tarama/tablo/yazı/etiketi okuyabilir;
bu dilim onu yalnız etki sayımı için kullanır, bağ taşıma diye sunmaz.

## İlk teslimin sınırı ve hesap

`cad.blocks.find` yalnız okur; ne belgeyi ne seçimi değiştirir. Kaynak en çok
128 öğe, arama kapsamı en çok 50 000 nesnedir. Şimdilik analitik çizgi,
çember ve yay desteklenir. Çoklu çizgi, metin, blok, NURBS veya başka bir
kaynak öğesi sessizce atlanmaz: bütün kaynak `recognition_unsupported` ile
reddedilir. Sonraki türler aynı doğruluk kapısından geçerek eklenir.

Döndürme varsayılan açık; ayna ve tekdüze ölçek eşleme varsayılan kapalıdır.
Katman, görünüş ve nesne verisi varsayılan olarak aynıdır. Bunları yok saymak
ayrı `geometryOnly` seçeneği ve raporlanan uyarıdır. Bu seçim bir veri
silme/birleştirme politikası oluşturmaz. Varsayılan tolerans kanonik düzlemde
0.000001 m (0.001 mm), mutlak uzaklıktır. Dünya orijinine uzaklıkla büyümez.
Hesap float64'tür; kaynak önce kendi çerçevesine alınır, çizim/GPU birimi
veya proje ölçeği değiştirilmez.

Yönlü çizgi, yay yarıçapı veya yalnız çemberlerde merkezler aday dönüşümleri
verir. Merkezler üzerinde paketli Hilbert R-tree karşılaştırmayı daraltır.
Son karar analitik geometri ve bütün üyelerin bire bir eşlemesidir; kutu
örtüşmesi veya örneklenmiş görsel benzerlik yeterli değildir. Yayın yarıçapı,
merkezi, uçları ve yay boyu/spanı sınanır. Yinelenen geometri çokluğu bir
üyeye düşürülemez. Örtüşen adaylar `overlaps` ile görünür olur.

`matches[].members`, `source` sırasındaki her UUID'nin karşılığıdır.
`origin`, `at`, `rotation` (radyan), `scale`, `mirror` dönüşümü anlatır:
`p' = at + scale · R(rotation) · M · (p-origin)`; M x eksenine aynadır.
Sıralı girdiyle aynı eşleşmeler ve sayaçlar üretilir. Sonuç sayısı veya
sabit iş bütçesi dolarsa `truncated=true` ve `recognition_truncated` döner;
“hepsini buldum” denmez. Kullanıcı kapsamı daraltabilir. İptal döngülerde
kontrol edilir; iptal sonuçları belgeye veya seçime uygulanmaz.

## Arayüz ve ortak API

Bloklar panelindeki etiketli **Tekrarlanan bileşenleri bul…** eylemi ve Ekle ›
Blok menüsü aynı ayarları açar. Küçük ayar penceresi kaynak seçimini tutar;
Bul aramayı başlatır ve sonuçlar dock'ta görünür. Sahne kullanılabilir kalır.
Her tuşta arama başlatılmaz. Tolerans KentOS `NumberInput` ile mm olarak
gösterilir; ortak birim/ifade çözümleyicisi kullanılır.

Adaylar ayrı incelenir, işaretlenir ve açık eylemle sahnede seçilir. Göster
adaya yakınlaşır. Bağlı nesnelerin sayısı ve kısmi tarama uyarısı görünürdür.
Durdur ve paneli kapatma arka plan işini iptal eder. Eski işin token'ı,
çizim oturumu ve geometri nesli kontrol edilir; eski belgeye ait sonuç yeni
belgenin nesnelerini seçemez. Ayar iptali kaynak çizimi değiştirmez.

Komut kataloğu input/output, birimler, desteklenen türler, ret ve uyarıları
taşır. Python SDK bu sözleşmeden üretilir; MCP aynı native işleyiciyi çağırır.
`plan`/`execute` aynı sorgu çıktısıdır. Python/MCP işi arka planda başlatıp
iptal edebilir. Masaüstü ortak salt okunur `inspect` yolunu kendi iptal
bayrağıyla kullanır. Python kodu çıktısı ayarı açıksa GUI sorgusu da aynı
public komut olarak bir kez yazılır; sahne seçimi mevcut seçim çıktısını
kullanır. Komut günlüğüne yapay bir belge değişikliği eklenmez.

Bu kabulde Python `JobStatus` okuyucusunun başarılı iş sonucunu `None`
çıktı tipiyle düşürdüğü bulundu. İşler yalnız execute çalıştırdığı için
okuyucu komut kataloğundaki `output_type` ile sonucu çözer. Hem mevcut
dönüştürme işi hem yeni sorgunun arka plan sonucu tipli çıktı olarak sınanır;
`validate` yolunun boş çıktı anlamı değiştirilmez.

## Kabul ve kalan iş

Çekirdek testleri 40 dönük/aynalanmış teknik sembolü, farklı yarıçaplı
benzerin reddini, açık ölçek/politika seçimini, büyük harita koordinatlarını,
yinelenen öğe çokluğunu ve iptal/sınırı sınar. Bağımsız Python üreteciyle
ortak komut durumları yazılır; Rust/Python/MCP ve gerçek GUI kabulü ayrıca
planın ilerleme kaydında bildirilir. Ham görüntüler Git'e eklenmez.

Yoğun örtüşme kabulünde, sabit aday ankrajını bulmak için tekrar bütün yakın
kimliklerin tarandığı görüldü. Ankraj üyesi artık yalnız o aday kimliğiyle
karşılaştırılır; analitik geometri ve katman/veri koşulları yine sınanır.
10 000 üst üste çember, ayrı kaynak ve farklı yarıçaplı benzer içeren aynı
dosyada önce 2 000 001 karşılaştırmayla 199 aday dönerken şimdi 2 002
karşılaştırmayla 1 000 aday döner. Sonuç sınırı korunur, kısmi sonuç açık
bildirilir. Süre makine yüküne bağlı olduğundan kabul ölçütü duvar saati
yerine karşılaştırma sayısı, doğru kimlikler ve değişmeyen çizimdir.
Kısmi sonuç ve çizim değişiminden sonra geçersiz kalan sonuç doğrudan
panel başlığında belirtilir. Seçilen aday sayısı eylem düğmesinde gösterilir;
dört haneli sayılar küçük panelde ayar düğmeleriyle üst üste binmez.

Bu dilimde bloklaştırma, alt nesneye bağlı ölçü/field/tarama/grup/tablo
ilişkilerinin taşınması ve tanınan grid/kutupsal/yol dizisinin benimsenmesi
**yapılmış sayılmaz**. Kaynak ID→yeni alt nesne eşlemesi, çakışma incelemesi,
mevcut `ArrayItem` ile eksik/elle değişen öğeler ve tek Undo kabulü sonraki
dilimlerdir. CAD-ADV-07 bunlar tamamlanmadan kapanmaz. Web yürütücüsü ve
arayüzünün eşitliği de bu native teslimin dışında izlenir.
