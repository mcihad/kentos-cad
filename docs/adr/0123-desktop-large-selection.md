# ADR 0123: Masaüstünde büyük seçim: vurgu çekirdeklerde kurulur

- **Durum:** kabul edildi (2026-09-28).
- **Tarih:** 2026-09-28
- **Bağlam belgesi:** ADR 0029 (seçim ve üzerine gelme vurguları), ADR 0120 (karenin maliyeti), ADR 0121 (katman parçaları); TODOS.md PERF-08.
- **Ölçüm:** [docs/perf/frame-desktop-parts-2026-09-28.md](../perf/frame-desktop-parts-2026-09-28.md) (önce), [docs/perf/frame-desktop-select-2026-09-28.md](../perf/frame-desktop-select-2026-09-28.md) (sonra).

## Bağlam

100 000 parselde Ctrl+A (Hepsini seç) arayüz iş parçacığında 242 ms sürüyordu. Pencere seçimi de büyük bir bölgeyi aynı yoldan seçer. İki payı vardı:

- **Seçim vurgusu:** seçilen her nesnenin çizgileri, alan dolguları ve işaretleri tek bir sahne parçasında, sırayla kuruluyordu (ADR 0029).
- **Katman görünürlüğü:** komut her nesne için katmanının ve üst gruplarının görünürlüğünü ağaçta yeniden arıyordu (`is_visible`).

## Karar

- **Paralel vurgu:** `scene::build_highlight_parallel` (`kentos-render-wgpu`) nesne listesini sıralı parçalara böler. Parçaları makinenin çekirdeklerinde (en çok 8) yan yana kurar, sonra sırayla birleştirir.
  - Dolgular bütün üçgenler, çizgi ve işaretler kendi başına örneklerdir. Birleştirme yalnız ardı ardına eklemektir.
  - Parça, sırayla kurulanın aynısıdır: katmanlar, çizgiler, dolgular ve işaretler bit bit eşittir (test, 1–n iş parçacığı).
- **Masaüstü:** 2048 ve daha çok nesneli seçimin vurgusu böyle kurulur (`viewport/highlight.rs`). Küçük seçim ve üzerine gelinen nesne eskisi gibidir.
- **Hepsini seç ve Seçimi ters çevir:** katmanın görünürlüğünü katman başına bir kez arar (`selecting.rs`, `Shown`).
- **Öznitelikler'in çok nesneli özeti:** kilitli katmanı da katman başına bir kez arar (`properties/rows.rs`).
- **Ölçüm:** `perf::frame`'e yeni bir durum eklendi: çizimi açma (ilk kare). Dosya okunup çözüldükten sonra çizimin ekrana geldiği kareyi ölçer: depo, stilli katmanların ilk kurulumu, ilk yükleme ve resim.

## Sonuçlar

Aynı makine, aynı çizim; arayüz iş parçacığı p50:

| Hepsini seç | 10 000 parsel | 50 000 parsel | 100 000 parsel |
|---|---|---|---|
| önce → sonra | 28,2 → 23,7 ms | 137,7 → 92,4 ms | 241,9 → 171,1 ms |

- 100 000 parselde kalan 150 ms'nin iki büyük payı vardır. Biri, parçaların arayüz iş parçacığında sırayla birleştirilmesidir: 2 milyon çizgi örneği ve dolgular, ≈130 MB kopya. Öbürü, Öznitelikler'in 100 000 nesnelik ilk özetidir: toplam uzunluk ve alan, ortak değerler.
- Seçimin GPU'daki ilk karesi (vurgunun yüklenmesi ve resmin yeniden çizilmesi) değişmedi.
- Yeni "çizimi açma" durumu 100 000 parselde 105 ms'dir (depo 63 ms, stilli katmanlar 41 ms). Önceki kaydı yoktur; bundan sonrakilerin tabanıdır.

## Doğrulama

- `cargo test -p kentos-render-wgpu --test scene`: örnek çizimin her türü 300 kez; 1, 2, 3, 7, 8 ve nesne sayısından çok iş parçacığıyla kurulan vurgu, sırayla kurulanın aynısıdır.
- `cargo test -p kentos-desktop` masaüstünün bütün testlerini çalıştırır: seçim, vurgunun önbelleği, Hepsini seç.
- `KENTOS_PERF_LABEL=select KENTOS_PERF_OUT=docs/perf cargo test --release -p kentos-desktop perf::frame -- --ignored --nocapture --test-threads=1` ölçümü yineler.
