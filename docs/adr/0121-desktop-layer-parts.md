# ADR 0121: Masaüstünde büyük stilli katmanın parçaları

- **Durum:** kabul edildi (2026-09-28).
- **Tarih:** 2026-09-28
- **Bağlam belgesi:** ADR 0090 (stilli katmanlar), ADR 0120 (karenin maliyeti, tutulan resim), ADR 0020 (belge sırası); TODOS.md PERF-08.
- **Ölçüm:** [docs/perf/frame-desktop-after-2026-09-28.md](../perf/frame-desktop-after-2026-09-28.md) (önce), [docs/perf/frame-desktop-parts-2026-09-28.md](../perf/frame-desktop-parts-2026-09-28.md) (sonra).

## Bağlam

ADR 0120'den sonra masaüstünde iki pahalı iş kaldı.

- **Düzenleme:** tek nesnenin düzenlenmesi bütün stilli katmanı yeniden kuruyordu. Katman GPU'ya da yeniden yükleniyordu. Tek katmanda 100 000 parselde bir nesneyi silmek arayüz iş parçacığında 120 ms sürüyordu.
- **Kaydırma:** stil çekirdeği bir katmanın aynı stildeki bütün nesnelerini tek partide toplar. Partinin kutusu bütün katmanı kapsar. Yakın görünümde de görünmeyen nesneler köşe gölgelendiricisinden geçer: 100 000 parselde 1:1000'de kaydırmada GPU'nun payı kare başına 29 ms'ydi (geri okumayla).

Web de kirli katmanı bütünüyle kurar; masaüstü aynı çekirdeği kullanır.

## Karar

- **Parçalar** (`apps/desktop/src/style/scene.rs`): 8192 ve daha çok nesneli katman parçalara bölünür. Bir parça, belgedeki yerleri aynı 4096'lık aralığa düşen nesnelerdir (`kentos-domain`'in yeni `Document::by_layer_placed` ve `place` okumaları).
  - Nesne değişince yerini korur; geri alınınca eski yerine döner (ADR 0020). Bu yüzden parçalar düzenlemede kararlıdır.
  - Parçalar belge sırasında birbirini izler.
  - Katman 4096 nesnenin altına inince yeniden bütün kurulur.
- **Düzenleme:** belgenin günlüğü değişen nesneleri söyler. Yalnız nesnenin şimdiki ve önceki parçası yeniden kurulur ve GPU'ya yüklenir; öbür parçalar kimlikleriyle kalır. Nesnesi kalmayan parça gider.
- **Aynı çizim:** stil çekirdeği bir katmanın partilerini düzeye, sonra dolgu, çizgi ve işaret sırasına, sonra ilk rastladığı sıraya göre dizer. Bir partide aynı stilin nesneleri belge sırasındadır.
  - Parçaların partileri bütün katmanın sırasıyla çizilir (`batches::merged_order`). Bir partinin yeri düzeyi, türü ve anahtarının ilk görüldüğü yerdir: ilk parça ve oradaki sırası. Aynı partinin parçaları art arda çizilir.
  - Yerli çözücü her partiye düzeyini ve çekirdeğin birleştirme anahtarının karşılığını verir (`StyledBatch::level`, `key`): türü, stili ve ölçek aralığı. İşaretin anahtarında boyu, yüksekliği ve dönüşü yoktur (`MarkerStyle::key`).
  - Böylece GPU aynı sayıları aynı sırayla çizer.
  - Sahne sırayı (`StyledScene::order`) taşır. Sıra tutulan resmin anahtarına da girer (ADR 0120).
- **Tek parça kalan:** bir ifadesi nesnenin sıradaki yerini (`$sıra`) okuyan katman bölünmez. Yer bütün katmanda sayılır. Parçalardan biri okuyorsa katman hemen bütün kurulur ve stili değişene kadar öyle kalır (`LayerCall::reads_index`).
- **İlk kurulum:** parçalar katmanlar gibi makinenin çekirdeklerinde yan yana kurulur, en büyüğü önce. Tek büyük katman da artık birden çok çekirdekte kurulur.

## Sonuçlar

Aynı makine, aynı çizim, 4× örnekleme; arayüz iş parçacığı (MİB) ve GPU (geri okumayla, boş çizimde ≈12–13 ms) p50:

| Durum | 10 000 parsel | 50 000 parsel | 100 000 parsel |
|---|---|---|---|
| Bir nesneyi silme, MİB | 16,5 → 9,6 ms | 74,5 → 15,3 ms | 120,6 → 7,6 ms |
| Geri al ve yinele, MİB | 11,0 → 7,9 ms | 59,9 → 13,4 ms | 114,5 → 12,8 ms |
| 1:1000'de kaydırma, GPU | 13,3 → 12,9 ms | 18,7 → 11,4 ms | 29,3 → 13,5 ms |

- Düzenleme artık katmanın boyuna bağlı değildir; parçanınkine bağlıdır.
- Yakın görünümde kaydırma, görünmeyen parçaları çizmediği için geri okuma tabanına indi.
- Düzenleme resmi yeniden çizer (ADR 0120). Genel görünümde bunun GPU'daki payı, bütün parselleri çizmek olarak kalır.

- Görüntü resimli işaretler (yazı, SVG) atlasta partinin en büyük işaretine göre çizilir. Parça kendi partisinin en büyüğünü bilir. Boyları nesneden nesneye değişen işaretlerde, bir parçanın küçük işaretleri bütün katmandakinden daha büyük bir resimden küçültülmez; aynı ya da daha keskindir. Başka hiçbir parti bundan etkilenmez.
- Katman başına parça sayısı nesne sayısıyla artar (100 000 nesnede 25). Her parçanın kendi GPU arabelleği ve bağlaması vardır. Partinin çizimi bir bağlama ve bir çizim çağrısıdır.

## Doğrulama

- `cargo test -p kentos-native-style --test batches`: `fixtures/style/v1/batches.json`'ın dokuz durumu bütün ve 1, 2, 3 nesnelik parçalarla kurulur. Parçaların birleşik sırasında partiler bütün katmanın sırasıyla gelir, sayıları bit bit aynıdır.
- `cargo test -p kentos-desktop style::scene_tests` şunları sınar:
  - büyük katman parçalara bölünür, küçük katman bütündür;
  - tek düzenleme yalnız kendi parçasını kurar;
  - boşalan parça gider;
  - `$sıra` okuyan katman bütün kalır;
  - sahnenin çizdiği, bütün kurulan katmanınkiyle sayı sayı aynıdır.
- `cargo test -p kentos-domain --test document` belgedeki yerlerin düzenlemede ve geri almada korunduğunu sınar.
- `KENTOS_PERF_LABEL=parts KENTOS_PERF_OUT=docs/perf cargo test --release -p kentos-desktop perf::frame -- --ignored --nocapture --test-threads=1` ölçümü yineler.
