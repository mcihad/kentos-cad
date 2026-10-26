# 0673 — CAD GPU konum hesaplarının tutarlı derlenmesi

25 Ekim 2026. Aşağıdaki dar düzeltme Apple M5 / Metal ve gerçek masaüstü
penceresinde doğrulandı. ADR 0672'ün kolon özelliğinin bütün kabulü ayrıca
izlenir; diğer GPU/sürücü ve performans matrisi tamamlandı sayılmaz.

## Gerçek hata ve bağımsız ayırma

- Aynı çizim 500000 / 4400000 m çevresindedir; dosya origin'i sıfırdır.
  Mahalın önizleme çizgisi ve yuvarlak kolonun gerçek çizgisi yaklaşık 15 piksel
  ayrılır. Kamera durduktan sonra da fark kalır. Ham ekranlar ADR 0672 kabul
  klasöründe 05, 06 ve 08'dir; görüntü düzenlenmemiştir.
- Kaydedilen mahalın dış/delik halkalarını bağımsız Python okuyucu ve Decimal
  alan hesabı doğrular: 75.1746018366… m². Bu durumda sınırı kaydırmak veya
  birim/ölçek değiştirerek görüntüyü denk getirmek doğru çözüm değildir.
- Yeni shader düzenlemesinden **önce**, mevcut `on_a_real_gpu` testi gerçekten
  `KENTOS_GPU_TESTS=1` ile çalıştırıldı. Apple M5 / Metal'de daha ilk
  alt piksel kaydırmada hata: çizgi 0 piksel hareket etti, beklenen −0.05 piksel.
  Bu eski testin origin'i veriye yakındır; yalnız sıfır origin sorunu değildir.

## Araştırma ve dar değişiklik

[WGSL `invariant` tanımı](https://gpuweb.github.io/gpuweb/wgsl/#invariant-attr)
aynı konum hesaplarının farklı giriş noktalarında tutarlılığını ister.
[Apple `preserveInvariance` belgesi](https://developer.apple.com/documentation/metal/mtlcompileoptions/preserveinvariance),
konum çıktısında `[[invariant]]` da bulunduğunda vertex hesaplarının korumalı
derleneceğini söyler. [Apple silicon taşıma kılavuzu](https://developer.apple.com/documentation/apple-silicon/porting-your-metal-code-to-apple-silicon)
konum ifadelerinin etrafındaki kodun agresif optimizasyonla farklı sonuçlar
üretebileceğini ve iki ayarın birlikte gerektiğini açıklar.

Kurulu wgpu-hal 27.0.4 Metal kodu, desteklenen aygıtta
`options.set_preserve_invariance(true)` verir (`metal/device.rs`). CAD ve
stil shader'larının konum çıktılarında `@invariant` yoktur. Bu yüzden ilk
aday düzeltme bu çıktılara açık `@invariant` koymaktır. Test sonucu olmadan
bunun bütün sürücülerin sayısal doğruluğunu garanti ettiği söylenmez.

- Kaynak koordinatlar, kamera matematiği, origin, hi/lo alanları ve GPU buffer
  düzeni aynı kalır. Yeni layout alanı veya veri şeması gerekmez.
- CAD dolgu/çizgi/nokta ve stil alan/raster/çizgi/sembol vertex konumları
  kapsanır. Paylaşılan WGSL, web arayüzüne yeni özellik ekleme değildir.
- GPU testi iki ekseni, yakın/sıfır/uzak origin'leri ayrı ölçer. Referans
  kamera FP64 hesabıdır; GPU çizgisinin kapsama ağırlıklı merkezi gerçek
  render hedefinden okunur. 8 bit kapsama için mevcut 0.02 piksel kabul
  sınırı korunur; tolerans büyütülmez.

## Kabul kapıları

- [x] Önceki gerçek GPU hatası yeni shader ile geçsin; üç origin × iki eksen,
  0.01 mm kaydırma adımları ve en yakın görünüm.
- [x] GPU'nun mevcut MSAA/HiDPI/katman bileşimi ve bütün pipeline kontrolleri.
- [x] WGSL/Naga sözleşme ve CPU hassasiyet testleri; konum invariant kontrolü.
- [x] Yeni masaüstü build'inde aynı dosya ve kamera çevresinde önizleme ile
  gerçek çizginin hizası; birkaç zoom/pan, ham ekran ve kayıt korunumu.
- [x] İlgili statik kontroller. Performans ve diğer sürücüler ayrı ölçülmeden
  evrensel hız veya tüm donanım kabulü iddiası yapılmaz.

Gerçek GPU sonucu: Apple M5 / Metal, üç origin × iki eksende en büyük
kapsama merkezi sapması **0,0029 piksel**, mevcut 0,02 piksel sınırının altında.
Aynı çalışmada bütün mevcut pipeline, MSAA/HiDPI ve katman bileşimi testleri
geçti. CPU hassasiyetinin yedi, WGSL/Naga sözleşmesinin yedi testi geçti.
Kanıtlar: `docs/baseline/2026-10-25-architecture-space-columns/gpu-before.log`
ve `gpu-invariant.log`. İkinci masaüstü build'inin gerçek penceresinde
13–19 görüntüleriyle sınır hizası, zoom ve gerçek Kaydır sürüklemesi doğrulandı;
iptal/kaydet dosyası bağımsız okuyucuda başlangıç modeliyle tamamen eşit.
İlgili on crate'in bütün hedeflerinde `clippy -D warnings` geçti (16 dk 54 sn).
Dar penceredeki yeni mahal açıklamasının kırpılması ADR 0672'te ayrı izlenir;
GPU hizası kabulünün yerine bu metin kontrolü kullanılmaz.
