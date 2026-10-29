# ADR 0742: Büyük çizimde tek düzenleme yalnız dokunduğunu yeniden kurar

- Durum: Kabul edildi (29 Ekim 2026)
- Madde: plan/gis.md CBS-42'nin dilimi
- İlgili: [ADR 0597](0597-budgets-on-large-data.md) (büyük veride bütçeler), [ADR 0121](0121-desktop-layer-parts.md)
  (büyük katman parça parça), [ADR 0120](0120-desktop-frame-cost.md) (tutulan resim), [ADR 0260](0260-object-look.md)
  (nesnenin çizim sırası), [ADR 0351](0351-documents-clipping-and-pdf-underlays.md) (Kırp), [ADR 0144](0144-blocks.md)
  (Bloklar paneli)

## Bağlam

CBS-42'nin ADR 0654'ten kalan işlerinden biri “tek düzenlemenin görünüm süresi”ydi. ADR 0597'nin tablosunda bir nesneyi
silmek 250 000 parselde 62 ms, 500 000'de 118 ms sürüyordu; çizimin boyuyla doğrusal büyüyordu. Tek nesnelik bir
değişikliğin maliyeti çizimin boyundan bağımsız olmalı: ADR 0121 büyük katmanı 4096'lık parçalara bölüp yalnız değişen
parçayı yeniden kurmak için yapıldı.

Karenin parçalara ayrılmış ölçümü (`perf::frame::edit_parts`, 250 000 parsel) maliyetin nereye gittiğini gösterdi:

- **Sahnenin planı, 16,5 ms:** değişen parçayı bulmak ucuzdu; ama her düzenlemede “katmanın kendi çizim sırası ya da
  maskesi olan bir nesnesi var mı” sorusu (ADR 0260 §5: varsa katman parçasız, bütün çizilir) katmanın 250 000 nesnesini
  tek tek okuyarak soruluyordu.
- **Kırpma çerçeveleri, 14,3 ms:** Kırp'ın ekranda gösterilen çerçeveleri (ADR 0351 §1) çizimin her değişikliğinde
  bütün nesneler dolaşılarak yeniden aranıyordu.
- **Bloklar paneli, her karede 11 ms:** panel açıkken blokların yerleştirme sayısı her karede bütün nesneler sayılarak
  bulunuyordu; düzenleme olmasa da.
- **Yazılar, 23 ms:** etiket motoru bir düzenlemeden sonra görünen bütün etiketleri yeniden yerleştiriyor.

Üreticilerin yolu (resmî belgeler, 29 Ekim 2026'da okundu):

- QGIS API, `QgsMapRendererCache`: çizilen resim katman başına önbellekte tutulur; katmanlardan biri yeniden çizim
  isteyince o katmanın resmi düşer (`invalidateCacheForLayer`). Bir katmanda değişen tek nesne o katmanın bütününü
  yeniden çizdirir. <https://api.qgis.org/api/classQgsMapRendererCache.html>

KentOS daha ince gider: ADR 0121'in parçaları bir katmanın içinde de yalnız değişeni yeniden kurar. Bu dilim o kuralın
çevresindeki, çizimin bütününü dolaşan soruları aynı ölçüye getirir.

## Karar

### 1. Kendi sırası olan nesneler parça parça bilinir

Katmanın önbelleği (`style/scene.rs`'in `Layer`'ı) kendi çizim sırası ya da maskesi olan nesne tutan parçaların kümesini
(`ordered`, parçanın anahtarı yerin 4096'ya bölümü) saklar. Taze bir katmanda düzenleme yalnız dokunduğu parçalara bakar
(`by_layer_placed` ile o parçanın nesneleri); yeni, bayat ya da başka görünüşle kurulacak katman bütün nesnelerine bakar
(zaten bütünüyle kurulur). Katman küme boşken parçalı, değilken bütün çizilir: ADR 0260 §5'in kuralı değişmez.

### 2. Kırpma çerçeveleri değişiklik günlüğüyle izlenir

`clip_frames.rs` çerçeveleri yerleştirmenin yuvasıyla tutar (katmanı ve halkası) ve belgenin günlüğünü
(`changes_since`) okur: değişiklik yalnız dokunduğu yuvalara yeniden bakar. Blok tanımları değişmişse (belge değişmeyen
tanımın `Arc`'ını korur), oturum başkaysa ya da günlük o kadar geriye uzanmıyorsa bütün nesneler bir kez daha dolaşılır.
Katmanın görünürlüğü çerçeveler verilirken sorulur.

### 3. Bloklar panelinin sayımı değişiklikte bir kez

Yerleştirme sayıları çizimin oturumu ve kuşağıyla (`generation`) panelin durumunda tutulur; aynı çizimin sonraki kareleri
saymaz.

### 4. Ölçüm kalıcı

`perf::frame::edit_parts` bir silmeyi, üç geri alma ve yinelemeyi, düzenlemeden sonra yazıları ve çizim alanını ayrı
ayrı, kamera kaymasının GPU payını yazar. [Ölçüm belgesi](../perf/edit-2026-10-29.md).

## Doğrulama

- `cargo test -p kentos-desktop --bin kentos-cad`: masaüstünün bütün testleri; yeni
  `style::scene_tests::an_object_drawn_in_its_own_order_builds_its_layer_whole_and_back_in_parts` (bir parçadaki nesneye
  sıra verilince katman bütün, başka parçadaki düzenlemede bütün kalır, sıra kalkınca yeniden parçalı) ve
  `clip_frames::tests::a_frame_follows_its_insert_its_layer_and_the_steps` (çerçeve gösterilir, yazdırılır, katmanı
  gizlenir, geri alma ve yinelemeyle izlenir).
- `cargo clippy -p kentos-desktop --bin kentos-cad --tests -- -D warnings`.
- Ölçüm (release, aynı makine, ADR 0597'nin testi):

| Durum | 250 000 önce | sonra | 500 000 önce | sonra |
|---|---:|---:|---:|---:|
| Bir nesneyi silme, MİB | 62,4 | 29,7 | 118,5 | 53,0 |
| Sahnenin planı | 16,5 | 0,55 | – | – |
| Kırpma çerçeveleri | 14,3 | 0,02 | – | – |
| Bloklar paneli açıkken her kare | 11,1 | 0,01 | – | – |

## Sonuçlar

- Tek düzenlemenin sahne ve çerçeve payı çizimin boyundan bağımsız: değişen parça kadar.
- Yeni bağımlılık, dosya alanı, komut ya da arayüz değişikliği yok.

## Kalan

- **Yazılar:** bir düzenlemeden sonra görünen bütün etiketler yeniden yerleşir: 250 000'de 23 ms, 500 000'de 47 ms.
  Yalnız değişen nesnelerin etiketlerini yerleştirmek ya da yerleştirmeyi iş parçacığına almak etiket motorunda iş
  ister. 500 000'de silme 53 ms bu yüzden aday bütçenin (33 ms) üstünde kalır.
- **GPU:** bütün çizim ekrandayken her kamera hareketi sahnenin bütününü yeniden çizer: kaydırmanın GPU payı
  250 000'de 498 ms, 500 000'de 1 044 ms (başsız, geri okuma dahil). ADR 0597'nin kaydırma sonucu yalnız arayüz iş
  parçacığıydı; ekranın yenilenme hızı bununla sınırlıdır. Yoğun görünümde çizim payının düşürülmesi ayrı dilim.
- **Seçimin bırakılması (Esc) 384 / 802 ms, Hepsini seç 736 / 1 467 ms, Hepsini taşıma 2,9 / 5,8 s:** çok nesneli
  seçimin işleri hâlâ çizimle doğrusal ve uzun.
- Dışarıdaki satırların dizini, görünür kümeyi izleyen bağ, soğuk önbellek, yavaş ağ, 20 GB raster ve bulut stresi
  (ADR 0654'ün Kalan'ı).
