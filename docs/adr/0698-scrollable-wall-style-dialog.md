# ADR 0698: Dar pencerede Duvar stilleri eylemlerine erişim

- Tarih: 27 Ekim 2026.
- Durum: dar pencere/iptal GUI kontrolleri ve desktop/UI testleri geçti;
  eski stil undo sorunu ve son ortak kapılar açık, nihai özellik kabulü yok.

## Kanıt ve neden

Gerçek native uygulamada 2202×1364 ham görüntü, yaklaşık 1100×682 mantıksal
pencere ve 18px yazıda Duvar stilleri penceresi alt eylemleri göstermiyordu.
Sağ form üzerinde tekerlek kullanmak da içeriği kaydırmıyordu. Tanısal kanıt
`.run/gui-evidence/2026-10-27-wall-dimensions/102-live2-style-manager-light18.png`
ve `103-live2-style-manager-scroll-diagnostic.png` altında; repoya eklenmez.

`wall_styles_view`, uzun gövdeyi `Dialog.push` ile esnek olmayan parça olarak
veriyordu. Bu parça `Fit` düzeninde alt eylemlerden önce kalan yüksekliği
tüketebiliyor; sağ form düz `Column` olduğu için tekerlek işlemiyordu.
Yalnız soldaki stil listesinin kendi kaydırması vardı.

## Dar karar ve resmî dayanak

Yalnız bu pencerenin `.push(body)` çağrısı `.scroll(body)` olur. Mevcut
Dialog/Fit başlık ve eylemleri önce ölçer; uzun gövdeyi kalan alanda kaydırır.
Kaydet/Vazgeç adları, taslak, kayıt/iptal, seçiciler, kesit çizimi ve geniş
penceredeki doğal boy korunur. Yeni ayar, TS, API, dosya şeması veya render
değişikliği yoktur.

[Apple Scroll views](https://developer.apple.com/design/human-interface-guidelines/scroll-views)
görünümden uzun içerikte alışılmış kaydırmayı ve kaydırılabilir alanın
anlaşılmasını önerir; aynı yönde iç içe kaydırmayı sakıncalı bulur.
[Microsoft Windows app best practices](https://learn.microsoft.com/en-us/windows/apps/get-started/best-practices)
küçük pencere ve farklı ölçeklerde bütün denetimlerin erişilebilir kalmasını
ve kaydırma girişleriyle sınanmasını önerir.

Bu dar düzeltmede mevcut sol liste kaydırması korunur; genel pencere tasarımı
yenilenmez. İç liste ve dış gövdenin olay paylaşımı bu nedenle özellikle
sınanmalıdır. Bunları ayrı esnek bölmelere dönüştürmek, gerektiği somut GUI
kanıtıyla gösterilirse ayrı bir düzenleme olur; kaynaklardan kusursuz iç içe
kaydırma veya bütün platformlarda erişilebilirlik garantisi çıkarılmaz.

## Kabul kapıları

- İki gerçek widget testi: 1100×650 ve 1100×682/18px uzun stil listesi ve
  16 bileşenli form; iç listede gerçek tekerlek iç alanı kaydırır, dış alan
  sabit kalır. Sağ formda tekerlek dış gövdeyi kaydırır; son bileşen ve kesit
  başlığına erişilir. Kaydet/Vazgeç konumu pencere içinde sabittir.
- 1440×900/13px dört bileşenli doğal görünüm uzatılmaz; gereksiz dış kaydırma
  yoktur. Gerçek Vazgeç tıklaması taslağı kapatır; gezinme ve iptal çizim
  modelini, dar senaryoda seçimi de değiştirmez.
- Otomatik test ve native fare/klavye sonuçları aşağıda ayrı kaydedilir.
  Otomasyon gerçek GUI kabulünün yerine geçmez; bütün tema/pencere/yazı
  kombinasyonlarının veya stil düzenlemenin bütün geçmiş akışının kabulü değildir.

## Gerçek GUI sonucu ve ayrı başarısızlık

[QA kaydı](../baseline/2026-10-27-architecture-wall-dimensions/README.md)
102/103 tanısını Live-3 tekrarıyla ayırır. 126/127 açık tema/18px dar pencerede
Kaydet/Vazgeç görünürdür ve sağ form gerçek tekerlekle kesit önizlemesine
kaydırılır. 128'de gerçek Vazgeç sonrasında tam model 125 ile aynıdır.
162 yerleşik stil görünümünü, 163 düzenlenebilir tam formu geniş/13px'de alt
eylemler ve kesitle gösterir. Stili taslağa alma/düzenleme ve iptal sonrası
164 tam belge 156 == 155 == 153 belgesine eşittir; bu taslak kaydı değildir.

130'da açık stil adını `Dış duvar 25 QA` yapıp tek Kaydet ile dosyaya yazma
yalnız önceden belirlenmiş ad farkını üretir. **131–135 Undo/Redo kabulü
başarısızdır:** ad eskiye dönmemiştir; toolbar ile ayrı deneme de tanı olarak
korunur. 137 eski adın **elle** geri konmasıdır ve tam 125 belgesine eşittir;
geri alma başarısı sayılmaz. Bu davranış tabanda da vardır: duvar stili tablosu
`set_settings` ile undo dışında yazılır; yalnız ad değişince entity follower
operasyonu oluşmaz ve boş grup geçmişe eklenmez. Footer/scroll düzeltmesi
bu semantik sorunu getirmedi ve çözmüş sayılmaz. Stil tablosu ile kaynak
duvarları tek geri alınabilir adımda tutmak ayrı açık iştir.

Son varsayılan paralel desktop turu **1341 geçti / 0 başarısız / 228 ignored**,
UI turu **289 geçti / 0 başarısız**; üç uygulama derlemesi başarılıdır.
Son Python/API, clippy ve WASM kapıları henüz açık olduğundan genel
özellik/yayın kabulü verilmez. Yeni pencere ve iptal kanıtları stil undo/redo
başarısızlığını, ekran okuyucuyu veya bütün tema/boyut matrisini örtmez.

## Paralel testlerde yazı ayarı notu

İlk tam paralel desktop turunda iki dar pencere testi tekerlek konumunda
başarısız oldu; aynı üretim koduyla iki testin tek sıra tanısal turu geçti
(`layout-target-20261027T000149Z.log`). Bu sonuç tek başına ilk koşunun
nedenini kanıtlamaz. Kaynakta `App::boot` global yazı ayarını okuyup kendi
varsayılanını `apply_appearance` ile tekrar yazabilirken, metin ölçen
testlerin kilidine katılmıyordu. Fail anındaki yazı ayarı kaydedilmemişti.

Dar test koruması: yalnız dev-dependency olarak mevcut kilit dosyasındaki
`parking_lot = "=0.12.5"`; mevcut `TYPOGRAPHY` kilidi yeniden girilebilir
olur ve `apply_appearance` yalnız `cfg(test)` altında aynı kilidi alır.
[ReentrantMutex belgesi](https://docs.rs/parking_lot/0.12.5/parking_lot/type.ReentrantMutex.html)
aynı iş parçacığının iç içe kilit almasını destekler;
[Cargo geliştirme bağımlılıkları](https://doc.rust-lang.org/cargo/reference/specifying-dependencies.html#development-dependencies)
bu bağımlılığı normal üretim derlemesinden ayırır. Menü ve Duvar stilleri
testleri yerleşim/olay boyunca global ayarın kendi `App.typography` değeriyle
aynı kaldığını ayrıca denetler. Tekerlek eşikleri ve GUI kabulü gevşetilmez.

Kilidi tutan test başka bir iş parçacığının görünüm ayarı uygulamasını
beklememelidir; bu durumda iş parçacıkları birbirini bekleyebilir. Mevcut
kilitli testlerde bu desen bulunmadı. Dış kilit almayan bütün UI okuyucuları
bu değişiklikle izole edilmiş sayılmaz. Ardından yeni tam paralel desktop
turunda 1341/0/228 sonucu ve yukarıdaki native pencere/iptal vakaları alındı.
Fail anındaki yazı ayarı önceden ölçülmediğinden ilk başarısızlığın tek nedeni
kesinleşti denmez; koruma artık iki testte olay/yerleşim boyunca ayrıca sınanır.
