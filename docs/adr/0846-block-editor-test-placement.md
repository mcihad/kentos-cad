# ADR 0846 — Kaydetmeden blok test yerleştirmesi

Tarih: 2026-11-02. CAD-ADV-06, masaüstü blok düzenleyicisi.

## Dayanak ve sınır

[AutoCAD'in Test Block belgesi](https://help.autodesk.com/cloudhelp/2026/ENU/AutoCAD-Core/files/GUID-B2732F5A-121D-4C33-8429-A53342003A53.htm)
testte kaydedilmiş eski bloğu değil düzenleyicideki güncel tanımı gösterir.
[Dinamik blok rehberi](https://help.autodesk.com/cloudhelp/2026/ENU/AutoCAD-LT-DidYouKnow/files/GUID-4005A6BB-388A-4EC7-BF17-555F1E6C137C.htm)
genişlik/tutamaç denemelerini ve sınır/adım kuralını test akışına dahil eder.
KentOS'ta mevcut düzenleyici ayrı taslak belgeye sahipti, fakat bu taslağın
parametreli yerleştirmesini kaydetmeden deneme yolu yoktu.

## Davranış

- Düzenleyicinin alt çubuğundaki **Test et**, güncel taslağı mevcut
  `cad.blocks.edit/update` doğrulamasından geçirir. Geçersiz öz başvuru
  gibi hatada taslakta kalır; çizime hiçbir şey yazılmaz.
- Geçerli güncel geometri, bütün dinamik/öznitelik/birim bilgileri korunarak
  ayrı belgedeki tanıma konur; örnek ortak `cad.blocks.insert` ile yerleştirilir.
  Varsayılan yerleştirme ölçeği ve birimsiz blok kuralı burada da aynıdır.
- Test örneği seçilidir. Kullanıcı normal tutamaç ve Öznitelikler
  kontrolleriyle genişlik, açı, görünürlük, lookup ve diğer değerleri dener.
- Testin ayrı geçmişi vardır. İlk test Undo'su kurulum örneğini silmez;
  yalnız kullanıcının testte yaptığı işlemleri geri alır.
- Testte **Kaydet** yoktur; yanlışlıkla gönderilen Save olayı da reddedilir.
  Mevcut file/cloud/project/sheet ve kurtarma engelleri yürürlükte kalır.
- **Düzenleyiciye dön**, test değişikliklerini bırakır; taslağın nesnelerini,
  geçmişini, seçimini ve kamera merkez/zoom durumunu geri getirir; testte
  pencere boyutu değiştiyse kameranın boyutu güncel çizim alanına uyar. Tekrar Test et yeni
  varsayılan örnekle başlar. Sonraki normal Kaydet önceki gibi asıl çizimde
  tek Undo adımıdır; Vazgeç asıl çizimi değiştirmez.
- Parametre panelinde Tanım varsayılanı / Örneğe özel / Hesaplanan ayrımı,
  eşleşen katalog satırı durumu ve güncel proje biriminde min/max/adım
  görünür. Uzun sayısal listede ilk altı değer ve kalan değer sayısı
  gösterilir; kuralın tamamı belge/API'de korunur. Hesaplanan ifade
  salt okunur. Bu bilgiler doğrulamanın yerine geçmez.

## Komut, Python ve AI

Bu bir belgeye kalıcı yeni nesne türü eklemez; test belgesi geçicidir.
Testte kullanılan değiştirme/yerleştirme/parametre komutları mevcut ortak
API'dir. Python'da `Document.from_bytes(original.to_bytes())` ile ayrılan
belgede aynı komutlar çalışır; MCP'de ayrı çizim oturumunda aynı
`cad.blocks.edit`, `cad.blocks.insert`, `cad.blocks.parameters` kullanılır.
Test oturumunun nesne kimlikleri asıl çizimin kimlikleri diye kullanılmaz.
İlgili örnek: [blok test rehberi](../cad/block-test-placement.md).

Tür/sürüm yükseltme, iç içe etki önizlemesi, kullanıcı tanımlı parametre
grupları ve web düzenleyicisinin eşitliği bu dilimin dışında, planda açıktır.

## Kabul kaydı

Son temiz adayda 1591 masaüstü testi, derleme ve Clippy geçti. Yeni kabul
öz başvuru reddi, kaydedilmemiş geometri, test Undo kurulumu, parametre ve
görünürlük, tekrar testin varsayılanları, taslak geçmişi/seçimi ve testte
pencere boyutu değişimini içerir. Belgeli Python örneği çalıştırıldı.

Gerçek pencerede 60,450→300,450 mm kol çizildi; Test et güncel kolu gösterdi,
1050 mm ve Sade denendi. Tam genişlikli sınır notu, dar pencere, testte
Save engeli ve geri dönüş görüldü. Normal Kaydet sonucu Python update
sonucuyla bayt bayt aynı; GUI tek Undo dosyası ve iptal dosyası başlangıçla
tam aynı. Son kamera düzeltmesinde küçült→dön→ikinci test doğru çerçevelendi.
Görüntüler Git dışında `/tmp/kentos-cad-adv04-2026-11-01/` altında
`163`–`168`; ölçülebilir dosya karşılaştırması `block-editor-test-gui-acceptance.json`.
