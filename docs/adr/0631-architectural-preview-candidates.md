# 0631 — Büyük belgede mimari önizlemenin aday araması

24–25 Ekim 2026 · ARCH-PERF-01 · Durum: aday dizini uygulandı; ARCH-PERF-01 tam kabulü sürüyor.

## Ölçülen sorun

`walls::neighbourhood` her duvar önizlemesinde tüm belge nesnelerini geziyor;
ilgisiz çizgiler de okunuyor, her duvarın erişim kutusu yeniden hesaplanıyor.
Önceki debug/FFI dikdörtgen denemesi üretim hızının ölçümü değildi. Yeni
`scripts/perf/architecture_preview.py` sabitlenmiş release native modülünde,
her biri ayrı süreçte 24 durum ölçüyor: 1/10/100 bin duvar veya çizgi,
ayrık veya 12 yerel duvarla kalabalık çevre, sıfır ve uzak koordinat.
32 imleç konumu için geometri ve uyarı parmak izleri saklanıyor. Yeni ve
mevcut duvar komutlarının eşitliği ve belgenin yazılmadığı da doğrulanıyor.

Aynı son koşucuda 20 soğuk / 60 sıcak örnekle, 100 bin duvar / uzak koordinatta
sıcak plan medyanı ayrıkta 2,263→0,015 ms, kalabalıkta 2,379→0,156 ms.
Soğuk medyan yaklaşık 3,04→6,32 ms ve 3,19→6,38 ms: ilk kurulum maliyeti saklanmaz.
Bunlar Python/JSON/FFI/plan/yanıt süreleridir; GPU, pencere olay gecikmesi veya FPS
sonucu değildir. Ham veriler ve bellek sınırları
[ölçüm kaydında](../baseline/2026-10-24-architecture-preview/README.md).

## Tasarım sınırı

- Mevcut ortak `PackedTree` ile yalnız duvarın mevcut komşuluk erişim kutusunu
  dizinle; FP64 ve eski genişletme/kesişme hesabını koru. Bu dilimde duvar
  geometrisini, yay/birleşim toleransını veya render motorunu yeniden tanımlama.
- Dizin belge deposunun geçici türetilmiş verisi olur; KCAD'e, schema'ya veya
  veritabanına yazılmaz. İlk gereksinimde kurulur. Soğuk kurulum ayrı ölçülür.
- `Store::put/remove` yolunda duvar ekleme/değiştirme/silme dizini geçersiz kılar;
  undo/redo, işlem iptali, dış değişiklik ve türetilen duvar düzenlemeleri aynı
  yolu kullanır. Kalıcı çıktı ve kopyalanan belge geçmişi değişmez.
- Okuma kopyaları değişmez dizini paylaşabilir; her deponun kendi `OnceLock`'ı
  vardır. Kopyadaki yazma asıl belgenin veya başka kopyanın dizinini değiştirmez.
- Yalnız aday kutuları saklanır. Duvarın güncel stil/evre/kat bilgisi ve gerçek
  birleşim sonucu yeniden okunur/hesaplanır. Eski komut kilitli/gizli duvarı
  dışlamıyorsa dizin onu sessizce dışlayamaz. Adaylar belge sırasına döndürülür;
  uyarı ve birleşim sırası değişmez. Birden çok sorgu kutusunun tekrarları silinir.
- Geçersiz/sonlu olmayan programatik kutu, geçerli kutu isteyen ağaca sokulmaz;
  eski kesişme koşuluyla savunma yolu korunur. Bu, geçersiz dosyayı kabul etme
  veya eski yay desteğini genişletme anlamına gelmez.

## Açık kabul işleri

Önce/sonra aynı release derleme, aynı girdi ve tüm çıktı parmak izleri;
soğuk/sıcak p50/p95 ve süreç belleği. Sil/taşı/genişlik/ofset/katman/evre,
undo/redo/iptal, okuma kopyası dallanması ve mevcut tüm duvar/açıklık/mahal
regresyonları. Python/MCP/GUI ve gerçek fare/sahne ekranları.

`perf::architecture::moving_preview` mevcut Iced kare koşucusunu kullanır:
fare olayı, uygulama, yerleşim, çizim ve GPU geri okuması ayrı raporlanır.
Bu pencere açmadan çalışma zamanı ölçümüdür; GPU alanı geri okumayı da içerir,
gerçek pencere sunum zamanı veya GPU timestamp ölçümü diye sunulamaz.
Gerçek pencere kabulü ve bu ölçümün sınırları ayrıca kaydedilmeden madde kapanmaz.
