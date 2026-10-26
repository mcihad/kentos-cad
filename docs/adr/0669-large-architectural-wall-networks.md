# 0669 — Büyük mimari kontur ağları ve tam önizleme

Tarih: 25 Ekim 2026. Durum: uygulandı; ARCH-DRAW-03'ün bu diliminin kabulü
[kayıtta](../baseline/2026-10-25-architecture-wall-networks/README.md).
Genel mimari yol haritası ve tam gerçek pencere matrisi açıktır.

## Sorun

ADR 0667'in ağ yolu her kenar parçasında bütün kaynaklar için `Vec<f64>`
tutuyordu. Bellek kenar × kaynak sayısıyla büyüyordu. İlk yoğun tabloyu
8 milyon hücreyle sınırlayan koruma 1.600 basit odada hem bitişik hem
ayrık geçerli girdiyi reddediyordu. Bu sınırı yalnız yükseltmek uygun değil.

Kontur önizlemesi ayrıca genel değiştirme araçlarının 400 hayalet nesne
sınırını kullanıyordu; kabul edilen büyük planın son odaları görünmüyordu.
Bu, hem ortak sınır çözümündeki hem bağımsız kapalı duvar yolundaki sorundu.

## Karar

- Arrangement'ın noding, analitik yay, kesişme, köşe temsilcisi, tolerans ve
  sıralama hesabı tek kalır. `Piece<D>`/`Built<D>` yalnız kaynak sahipliği
  deposunu parametreleştirir. Varsayılan `Vec<f64>` ve `build` mevcut
  overlay tüketicilerinin yoğun davranışını korur.
  Sınırsız yoğun yolun hata tipi `Infallible`'dır; kullanıcı verisine açık
  bir `expect`/panic eklenmez. Seyrek yolun aynı denetimleri `Budget` döndürür.
- Duvar ağı `build_sparse` ile kaynak kimliğine göre sıralı `(indis, sayı)`
  çiftlerini tutar. İptal olmuş sıfır hücreler saklanır; toplama sırası ve
  sıfır testi yoğun yolla aynıdır. Aynı çekirdekte iki ayrı geometri hesabı yoktur.
- Kaynak köşe sayısı kopyalamadan önce en fazla 200.000; çıktı parçası
  200.000 ve sahiplik hücresi 400.000 sınırındadır. Limit aşımı `Budget`
  döndürür; kısmi ağ veya yaklaşık geometri dönmez. Bunlar sayım
  kotalarıdır, kesin byte/RSS veya süre garantisi değildir. Noding'in geçici
  kesişme/kesme yapıları bu iki çıktı kotasıyla sınırlanmış sayılmaz.
- Farklı kaynaklarda aynı taraftan geçen çakışık kenar mevcut overlap
  hesabıyla ön kontrolde reddedilir. Binlerce yinelenen odayı önce düğümlemek
  gerekmez. Birden fazla hata içeren girdide ret gerekçesi önceliği değişebilir;
  ürünün `invalid_wall` ve atomik ret davranışı aynıdır.
- Ortak kontur önizlemesinde 400 parça kesmesi kaldırılır. Ayrık konturlar
  komutun mevcut `MAX_WALLS` sınırına kadar bütünüyle görünür. Tek tek/Çiftler
  ve diğer değiştirme araçlarının eski hayalet sınırı korunur.
- Yeni komut, sözleşme, SDK türü, KCAD şeması, ölçek, birim, kamera ya da
  render matematiği eklenmez. Python/MCP mevcut `cad.walls.convert` yolunu kullanır.

## Doğrulama ve sınır

- Yoğun/seyrek arrangement köşeleri, tam kenarları, sahiplik sayıları;
  yaylar, ters yön iptali ve kesici çizgilerle birebir karşılaştırılır.
- 1.600 bitişik oda ve 10.000 ayrık oda eski bütçede başarısızken yeni
  yolda geçer. 2.000 yinelenen oda açıkça reddedilir. Kotanın atomik dönüşü sınanır.
- Bağımsız ölçülü avlu/dolgu, çok parçalı L oda, kısmi ortak kenar, daire,
  yaylı poligon ve kapalı çoklu çizgi: 48 birleşimde 9 duvar, 5 ortak parça,
  9 m ortak sınır, toplam `115 + 8π` m; renk/öznitelik/kat/kaynak ve undo/redo.
- 40×40 native ve gerçek GUI: 3.276 normal duvar; 3.280 temel kenarın her
  biri bir kez, toplam 11.480 m. Önizleme yazmaz; tek onay/undo/redo/kayıt-aç.
- Ayrık ve bitişik 501 odanın önizlemesi son odaya ulaşır. Aynı test eski
  kodda 400'üncü odada kesilerek başarısız olur; yeni kodda geçer.
- Çekirdek release süre/bellek ölçümü ile gerçek arayüzün işlevsel kabulü
  ayrı raporlanır. Çekirdek ölçümü GUI gecikmesi, GPU veya tüm native komut
  performansı değildir. Tam uçtan uca performans ve tema/font/boyut matrisi açık kalır.
