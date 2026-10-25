# Kapalı konturdan duvar — ilk uygulama ve kabul

25 Ekim 2026 · ARCH-DRAW-03 [~] · ADR 0637 · `codex/architecture-2d-workflows`.
Taban `24f93bb5e`. Bu kayıt yalnız aşağıda gerçekten
çalıştırılmış kapsamı kanıtlar. Ortak duvar çözümü, farklı kat semantiği ve
§10'un tam gerçek pencere/font/tema matrisi açık kalır.

## Davranış

Mevcut Çizgiden duvar aracı ve `cad.walls.convert` komutunda isteğe bağlı
Kapalı kontur modu. Dış halka/delik yönleri düzenlenir, ilk köşe ve analitik
yaylar korunur. İç net / Dış ölçü / Eksen; stil/kalınlık; kaynak Koru/Sil
aynı planın girdileridir. Önizleme yazmaz, uygulama tek undo/redo'dur.
Eski Tek tek/Çiftler, varsayılan yöntem ve Ç kısayolu korunur.

Bu dilim değen/kesişen konturların ortak duvarını birleştirmez; hiçbirini
yazmadan nedenini bildirir. Aynı katmanda aynı analitik konturun mevcut
kapalı duvarı varsa ikinci kez üretmez. Açık/geçersiz kaynak, kilitli katman,
uyumsuz hedef türü, oda/avlu kapanması ve desteklenmeyen kot/M girdisi de
atomik ret verir. Görünmez katmandaki açıkça verilen kaynağa erişim korunur.

Yeni KCAD şeması/nesne türü/komut yoktur. Model metre, çizim ölçeği 1:100,
FP64/import/kamera/render kuralları korunur. Python/MCP aynı native komutu
kullanır. Web UI'ye özellik eklenmedi; yalnız mevcut WallsFromMode TS türü,
ortak katalog ve Python SDK çıktıları güncellenir.

## Bağımsız başvuru ve testler

- Önceki **38** convert vakası JSON olarak birebir aynı önektir; **31** yeni
  bağımsız Python vakası eklendi. L/avlu, ters yön/büyük koordinat, üç referans,
  Koru/Sil, daire, kapalı çoklu çizgi ve üç geçersiz kaynak durumu.
  `fixture-prefix-check.json` kanıtı. Eski çeyrek dairenin tanjantı referansta
  Decimal sqrt(2)-1 ile hesaplanır; platform libm'nin bir ULP farkı giderilir,
  ürün matematiği değiştirilmez.
- Son kaynak hedef application: **5 geçti**. SI ölçüler m/mm/cm/in/ft gösterim
  biriminden bağımsız; L iç alanları 18 / 13.76 / 15.84 m²; avlu net alanları
  76 / 67.2 / 71.6 m²; daire πr² ile bağımsız kontrol. Kilitli/gizli/uyumsuz
  katman, salt okunur politika, kaynak nitelik/renk, tek undo/redo, tekrar,
  ortak kenar ve daralan oda retleri de kapsanır.
- Son kaynak hedef interaction: **3 geçti**. Görünür Türkçe ayarlar, 250 mm
  giriş, kaynak Koru/Sil, negatif genişlik reddi, eski Çiftler kısayolu ve
  değişmiş belge revision'ında ilk onayın reddi/yeniden açık onay.
- Son kaynak MCP: **1 geçti**. Gerçek headless hostun keşif şeması ve aynı
  semantic komutla yazmasız plan, avlu için iki duvar, tek undo/redo ve tekrar
  üretimin atomik reddi. İlk test enum dizisi varsaydığından, mevcut açıklamalı oneOf şemasını
  okuyacak şekilde düzeltildi; ürün şeması buna göre daraltılmadı.
- Ortak sözleşme **799** ve ortak 69 convert fixture testi önceki kaynak
  aşamasında geçti; son katman türü/kot kontrolünün tam küme kabulü değildir.
  Son kaynak tam küme kontrolleri aşağıda eklenecektir.
- SDK/işlem manifesti ve bağımsız fixture drift kontrolü, 47 crate/61 hedef
  bağımlılık yönü temiz. Web'in donmuş test kümesi çalıştırılmadı.

## Gerçek uygulama kabulü

Ayrılmış Ribbon QA uygulaması ve kullanıcı çizimlerinden ayrı dört kaynaklı
çizim hazırlandı: L oda, avlulu alan, ters oda ve açık çizgi. m / 1:100.
CUA ile gerçek fare/klavye kullanıldı, ham JPEG görüntüleri `shots/` içindedir.
İlk pencere 2880×1720 fiziksel / 1440×828 iç alan, koyu tema ve 18 px'tir.
Bu boyut 1440×900 kabulü sayılmaz. Köşeden gerçek sürüklemeyle son pencere
2200×1364 fiziksel / **1100×650 iç alan** boyutuna ayarlandı.

| Görüntü | Gerçek işlem ve gözlenen sonuç |
| --- | --- |
| 01 | Dört kaynak, başlangıç m / 1:100. |
| 02 | L alanı Kapalı kontur / İç net / Koru önizlemesi, belge yazılmadı. |
| 03 | 250 mm girilip 0.250 m gösterildi; tek sağ tuş Onayla menüsü. |
| 04 | Onayla'ya bir kez tıklama L duvarını oluşturdu; tek Undo kaynak çizime döndü. Uygulama dosyası `l-inner-keep.kcad`, geri alma `l-undo.kcad`. |
| 05–06 | Redo sonrası avlulu alan; Dış ölçü / Koru / 0.250 m önizlemesi, Enter ile iki normal kapalı duvar. |
| 07 | Seçili özgün avludan tekrar üretim kırmızı açıklamayla reddedildi; Enter da yazmadı. Escape ardından dosya aynı. |
| 08–09 | Ters oda Eksen / Sil; tek Enter duvar ekleyip alanı kaldırdı. Tek Undo özgün alanı geri getirdi; Redo tekrar duvarı getirdi. |
| 10–11 | Gerçek teker yakınlaştırma, Home, Görünüm/Kaydır ile fare sürüklemesi. Geometri ve nesne sayısı korunur; kare süresi veya titreme ölçümü yapılmadı. |
| 12 | Kapat/aç ve Home: 7 nesne, normal duvarlar ve 1:100. |
| 15 | Son ikiliyle açık kaynağın aynı okunur kırmızı ret mesajı; Enter yazmaz, Escape çıkar. |
| 13–14 | Gerçek 1100×650 / koyu 18 px: Mimari araçlar yazıyla erişilebilir; beş ayar satırı görünür. Açık çizgi Kapalı kontur modunda açık kırmızı ret verir; Enter yazmaz, Escape çıkar. |

Bağımsız `verify.py` gerçek GUI dosyalarını KCAD şartnamesi okuyucusuyla okur.
Decimal ile ortogonal yüzleri ayrıca hesaplar: L 18 m²; 0.25 m Dış ölçü avlu
65 m²; ters dikdörtgen Eksen 21.5625 m². İlk dört kaynağın bütün semantik
alanları/UUID'leri (Sil seçilen kaynak dışında), renk/nitelik/katman, model
birimi ve proje ayarları korunur. Dosya yerel sıralı sayısal ID'ler kaynak
silinince sıkışır; kalıcı UUID eşlemesiyle karşılaştırılır. Undo ve tekrar ret
sonrası çizim aynı, yeniden açılışta duvarlar/nitelikler aynı.

İlk gerçek test ikilisinin SHA-256 değeri `gui-binary.json` içindedir. Sonraki
clippy denetimi yeni önizleme satırlarında gereksiz bir unwrap buldu; güvenli
Option eşlemesine dönüştürüldü. Son hedef interaction testleri tekrar geçti;
önceki tam native/desktop kümeleri bu küçük sunum korumasından öncedir.
Son ikili (`bc28d4e4…`) tekrar açıldı; 15. görüntüde aynı açık kaynak
ret mesajının satırları ve Enter/Escape davranışı doğrulandı. Bu son pencere
1440×830 iç alandır; 1440×900 diye sayılmaz. Tam 1440×900,
13 px/açık tema ve bütün özellik düzenleme matrisi açık kalır.

## Son kaynak denetimleri

- Native application: **51 birim + 173 entegrasyon geçti**, 22 entegrasyon
  atlandı. Interaction: **63 birim + 659 entegrasyon geçti**, 1 entegrasyon
  ve 5 resim/ölçüm testi atlandı. Genel Modify ayar aktarımıyla eski araçların
  bu kümesi de geçti. Son unwrap korumasıyla hedef interaction **3 geçti**.
- Desktop: **1280 geçti, 221 atlandı**; font testleri seri, ayrılmış XDG
  ayar/veri dizinleri. Bu otomatik küme gerçek GUI'nin yerine sayılmaz.
- Application/interaction/desktop/MCP/Python bütün hedeflerde son clippy geçti;
  ortak geometri WASM derlemesi geçti.
- Son native Python modülü tekrar derlendi; tam Python kümesi **277 geçti**
  (219.920 s). İki yeni kontur testi; SI/gösterim birimi, idempotency, kaynak
  kimlik/nitelik, KCAD tekrar açma, avlu/sil/undo ve tekrar atomik ret dahil.

Ham test günlükleri `logs/` altındadır. Atlananlar geçti sayılmaz. Tam GUI
matrisi, ortak kenar birleştirme ve farklı kat entegrasyonu açık kaldığından
ana ARCH-DRAW-03 maddesi tamamlandı diye işaretlenmedi.
