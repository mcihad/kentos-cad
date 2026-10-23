# 0614 — Fiziksel pafta kalibrasyonu

Tarih: 23 Ekim 2026. Kapsam: CAD-SHEET-24.

## Karar

Sistem galerisindeki **Baskı kalibrasyonu** şablonu sabit 100 mm yatay ve düşey
cetveller taşır; 5 mm işaretler, 50 mm ara ve 100 mm uç göstergeleri vardır.
Konum kısıtları boyu ölçeklemez: A4, A3 ve A0'da cetvel yine 100 mm'dir.

Görünüm penceresi kilitlidir ve ölçeği standart bir `map.scale = 50` bağıyla
sabitlenir. Galeri veya script oluşturma seçenekleri başka proje ölçeği verse de
baskı kamerası 1:50 kalır; şablon kimliğine özel bir renderer davranışı yoktur.
İstenirse kullanıcı normal pafta düzenleme işlemleriyle bağı ve kilidi değiştirebilir.

Çizimin 5000 mm (iç modelde 5 m) çizgisi 1:50'de 100 mm'dir. Kağıt cetveli bir
model nesnesi değildir; modelin kaynak geometrisi ve birim tercihi değiştirilmez.
Şablon mevcut çizimi gösterir; kendiliğinden modele test çizgisi eklemez.

## Toleranslar

- Kaydedilmiş PDF'in vektör koordinatlarında ölçüm kabul sınırı **±0,002 mm**.
- Ekran veya raster örneğinde konumun ayrıca bir piksel örnekleme belirsizliği
  vardır; ekran cetvelle fiziksel kontrol için kullanılmaz.
- PNG artık istenen dpi'yi metre başına piksel (`pHYs`) olarak taşır. Piksel
  boyutları yukarı yuvarlanır; dpi metadatası en yakın tam piksel/metredir. Bu
  nicemleme PDF vektör toleransına eşitmiş gibi sunulmaz.
- Fiziksel yazıcıdaki sapma uygulamanın PDF toleransından ayrıdır. Gerçek boyut /
  %100, Sığdır kapalı seçilir; yatay ve düşey 100 mm işaretler kağıtta ölçülür.
  Yazıcının besleme ve sürücü davranışı cihazda ayrıca doğrulanır.

## Doğrulama

Bağımsız Python başvurusu kesirlerle 20 farklı kaynak uzunluğu/ölçek olgusu kurar.
Birim tercihleri mm/cm/m, kameralar düz/90°, koordinatlar yerel ve büyük proje
koordinatıdır. Her PDF'te model çizgisi ve kağıt cetveli ayrı renklerinden okunur;
her ikisi de 100 mm olmalı, kaynak snapshot değişmemelidir. Özellikle 1:50'deki
kaynak uzunluğu 5000 mm'dir. Şablon A4/A3/A0'da ve oluşturma ölçeği 200 verilerek
ayrıca ölçülür. PNG çözünürlük kaydı beş dpi değeriyle kontrol edilir.

Mevcut `cad.sheets.create` ve `cad.sheets.edit` işlemleri, Python ve MCP aynı
şablonu kullanır; ek bir model yazma veya undo yolu yoktur. Arayüz ve çıktı
resimleri açılarak cetvellerin, açıklamaların ve sayfa sığmasının kontrolü yapılır.
Web uygulaması ADR 0228 gereği ertelidir.

Kabul: 71 ortak entegrasyon testi, 8 şablon/matris testi, 3 native fiziksel/dpi
testi (20 ortak olgu dahil), 242 Python testi, MCP oluşturma/ölçüm ve masaüstü
görüntü sınaması geçti. Pafta, önizleme ve gerçek PDF görüntüleri açıldı. SDK,
clippy, 47 crate/61 hedef bağımlılık denetimi ve ortak WASM smoke geçti. Galeri
sıralamasının elle yazılmış referansına yeni şablonun CAD/CBS/ortak sırası eklendi.
