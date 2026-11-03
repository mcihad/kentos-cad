# ADR 0868 — Yeni ölçü bağını işlemin yeni geometrisinden çöz

Tarih: 2026-11-03. CAD-ADV-07 bloklaştırmasının ilişki koruma önkoşulu;
adayları bloklaştıran yeni araç henüz teslim edilmiş sayılmaz.

## Dayanak ve hata

[BricsCAD V27 BLOCKIFY](https://help.bricsys.com/en-us/document/command-reference/b/blockify-command?version=V27)
parametrik ilişkilerin değiştiren bloğun alt nesnelerine eşlenmesini ayrı
bir koruma davranışı olarak tanımlar.
[AutoCAD 2027 blok yerleştirme](https://help.autodesk.com/cloudhelp/2027/ENU/AutoCAD-LT/files/GUID-BC0FD3C1-3BFC-4C5D-AB9A-BF480D5084BE.htm)
ayrı yerleştirmelerin aynı tanımı konum, dönüş ve ölçekle kullanmasını açıklar.
KentOS çıkarımı: kök nesne bağı → yerleştirme/alt nesne bağı, bir eski
geometrinin kendiliğinden takibi değildir; komutun açıkça verdiği yeni bağdır.

`Document::follow_dimensions` yeni bağın yolunu eski blok tanımları ve
eski kök nesne üzerinden izliyordu. Aynı dış işlemde blok tanımlama,
kök nesneyi yerleştirmeyle değiştirme ve `cad.dimensions.reassociate`
başarıyla bitse de işlem kapanınca geçerli bağ `no_such_place` oluyordu.
Hata hem domain regresyonunda hem gerçek masaüstü Python konsolunda
aynı çağrı zinciriyle düzeltme öncesinde tekrarlandı.

## Karar

- Bir ölçü noktasının bağı işlem tarafından açıkça değiştirilmişse yeni
  UID/yol/yer, işlemin son geometrisinden çözülür. Aynı UID'de tür değişimi
  ve ayrı UID ile yeni yerleştirme oluşturma aynı kurala uyar.
- İşlemde yeni eklenmiş ölçünün hedefi veya blok tanımı da değişmişse
  yeni bağı eski geometri üzerinden izlenmez.
- Geçersiz yeni hedef/alt nesne/yer başarı sayılmaz; uygun kopma nedeni
  yazılır, noktanın konumu korunur. Otomatik ölçü tarifinin sahiplik istisnası
  ve eğri/kenar/çift/kesit türetme kuralları korunur.
- Değiştirilmemiş bağın olağan takip, kopma ve ölçüyü tek başına sürükleyince
  serbest bırakma davranışı değişmez. Takip düzeltmeleri aynı Undo adımındadır.
- Yeni komut veya dosya şeması eklenmez. Mevcut public komutların birlikte
  çalışmasını düzeltir; GUI/Python/MCP aynı domain işlemini kullanır.

## Kabul

Domain testleri aynı kimlikte değiştirme ve yeni yerleştirme kimliğiyle
aktarma, işlemde yeni ölçü/yeni tanım, geçersiz köşe, sonraki taşıma ve
tek Undo/Redo ile tam snapshot eşitliğini sınar. Native uygulama testi public
blok tanımla, nesne değiştir ve ölçü yeniden bağla komutlarını aynı dış
transaksiyonda çalıştırır. Tüm domain testleri ve uygulama testleri geçirilir.

Gerçek GUI'de aynı script önce eski sürümde kopuk, yeni sürümde canlı bağ
üretmiştir. Sonraki 5 m/3 m taşıma ölçüyü taşımış, 2 000 mm boy kalmıştır.
Taşımayı tek Undo, blok+bağ işlemini tek Undo ve Redo sonrasında arayüzden
kaydedilen dosyalar ilgili başlangıç/son dosyalarıyla bayt bayt aynıdır.
Ham görüntüler, dosyalar ve SHA kayıtları depo dışındadır.

Bu kabul genel bloklaştırmanın bütün bağımlılıklarını veya görünüş
korumasını tamamlamaz. GUI'de geniş çoklu çizginin blok içinde farklı dolgu
ile çizildiği ayrıca görüldü; dönüşüm araçları açılmadan ayrıca giderilmelidir.
