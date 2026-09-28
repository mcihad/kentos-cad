# ADR 0120: Masaüstünde karenin maliyeti: çizimin tutulan resmi, yazıların ve Öznitelikler'in önbelleği

- **Durum:** kabul edildi (2026-09-28).
- **Tarih:** 2026-09-28
- **Bağlam belgesi:** ADR 0005 (performans hedefleri, taslak), ADR 0019 (masaüstünün wgpu çizim alanı), ADR 0023 (kalite ayarları), ADR 0029 (seçim ve üzerine gelme vurguları), ADR 0055 (çizimin yazıları), ADR 0063 (Öznitelikler), ADR 0090 (stilli katmanlar); TODOS.md PERF-03, PERF-08.
- **Ölçüm:** [docs/perf/frame-desktop-before-2026-09-28.md](../perf/frame-desktop-before-2026-09-28.md), [docs/perf/frame-desktop-after-2026-09-28.md](../perf/frame-desktop-after-2026-09-28.md).

## Bağlam

Sahibi, web'deki işler masaüstüne taşındıktan sonra masaüstünün performansına geçilmesini istedi. Kalite ayarları yalnız çizim alanını düşürür; arayüz her zaman tam kalitededir (ADR 0023'ün eki).

Masaüstünün bir karesi hiç ölçülmemişti. Yeni ölçüm (`perf::frame`) arayüzü pencere açmadan, Iced'in çalışma zamanının sırasıyla sürer. Bir girdiyi parçalarına ayırır: bileşenlerin olayı alması, uygulamanın mesajları, `App::view`, Iced'in ağacı karşılaştırıp yerleştirmesi, bileşenlerin kareyi kaydetmesi ve GPU'nun payı. Çizim parsel başına 20 köşeli alanlardır, pencere 1440×900'dür.

Önceki durumda, 100 000 parselde (2 milyon kenar) bütün çizim görünürken:

- **İmleç hareketi:** GPU'nun payı kare başına 174 ms'ydi (≈6 kare/sn), arayüz iş parçacığınınki 5,6 ms.
- **Hepsi seçiliyken imleç:** arayüz iş parçacığı kare başına 63 ms harcıyordu.

Üç neden vardı:

1. **GPU:** Iced her olayda bütün pencereyi yeniden çizer: imleç hareketinde, bir düğmenin üzerine gelince, yazı yazarken. Çizim alanı da her karede bütün sahneyi çiziyordu. Çizim değişmemiş olsa da, değişen yalnız imleç, kenet işareti ya da şeridin bir düğmesi olsa da böyleydi. Web'de imleç, kenet işareti ve önizleme ayrı bir 2B tuvaldedir. Çizimin WebGL katmanları imleçle yeniden çizilmez; yalnız üzerine gelme vurgusu değişince çizilir.
2. **Yazılar:** çizimin yazıları (ADR 0055) her `view`'da depodan sorulurdu. Resim önbellekteyken de öyleydi. Depo görünümdeki her nesneyi gezer; genel görünümde bu bütün çizimdir.
3. **Öznitelikler:** birden çok nesne seçiliyken panel bütün seçimi her `view`'da iki kez geziyordu: başlıktaki sayı için ve gövde için. Türler, ortak değerler ve toplamlar her seferinde yeniden bulunuyordu.

## Karar

- **Tutulan resim** (`kentos-render-wgpu`, `renderer.rs`):
  - Yeni `FrameInput::keep_picture` ile istenir. Görünüm tek örneklemeli ve tam çözünürlükte de kendi hedeflerine çizer.
  - Çözülmüş resmin neyi gösterdiği bir anahtarda tutulur (`Targets::holds`, `Targets::keep`). Anahtar şunlardan oluşur:
    - sahne parçalarının kimlikleri;
    - karenin tekdüzesi (kamera, boyut, ölçek, ayarlar);
    - zeminin rengi;
    - stilli katmanların katmanları, görünen parçaları ve imgelerinin atlastaki yerleri (`ViewStyled::key`).
  - Anahtarı aynı olan kare sahneyi çizmez; tutulan resmi birleştirir.
  - Hedefler yeniden yapılınca resim tutulmamış sayılır.
  - Karenin istatistiği bunu söyler: `FrameStats::picture_kept`.
- **Üst katmanlar:** imleçle değişen parçalar (`FrameInput::overlays`; masaüstünde üzerine gelinen nesnenin vurgusu, son parça) resme girmez. Her karede resmin üstüne, doğrudan Iced'in karesine çizilir; web de vurguyu çizimin katmanlarından ayrı çizer.
  - Üst katmanlar pencerenin çözünürlüğünde ve tek örneklemeyle çizilir. Çizgi ve işaret gölgelendiricileri kenarlarını kendileri yumuşatır (uzaklıktan örtme). Bu yüzden 1× ve 4× örneklemede bütün çizilen kareyle piksel piksel aynıdır (GPU testi).
  - HiDPI kapalıyken de üst katman tam çözünürlüktedir: kaplamalar her zaman tam kalitededir (sahibin kuralı).
- **Seçim vurgusu** resmin içindedir: seçim değişince resim bir kez yeniden çizilir.
- **Yazılar** (`labels.rs`, `Spots`): depodan sorulan yazılar bir anahtarla tutulur. Anahtar çizimin kimliği, değişiklikleri (`generation`: başkalarının değişiklikleri de sayılır), görünüm ve yerinde düzenlenen yazıdır.
  - Görünüm ve çizim aynı kaldıkça kare depoya sormaz.
  - Resmin anahtarı da artık `revision` değil `generation`'dır. Buluttan gelen değişiklik de yazıları yeniden çizer.
- **Öznitelikler** (`properties/`, `PanelCache`): birden çok nesnenin paneli açık çizimin kimliği, değişiklikleri ve seçimin sürümüyle tutulur. Tek nesne ve boş seçim her karede yeniden bulunur. Bunlar ucuzdur ve çizimin değişikliği olmayanları gösterir: dosya adı, etkin katman.
- **Ölçüm:** `perf::frame` (`apps/desktop/src/perf/frame.rs`) on bir durumu dört çizim boyutunda (0, 10 000, 50 000, 100 000 parsel) ölçer. Durumlar şunlardır:
  - boş kare, imleç, tekerlek, kaydırma, 1:1000'de kaydırma;
  - çizim aracıyla imleç;
  - bir nesneyi silme, geri al ve yinele;
  - hepsini seç, hepsi seçiliyken imleç, seçimi bırakma.

  Sonuç `docs/perf/frame-desktop-<etiket>-<tarih>.{json,md}`'ye yazılır. Klavye olayları uygulamaya, pencerede olduğu gibi, aboneliğin işleviyle (`keys::key_event`) verilir.

## Sonuçlar

Aynı makine, aynı çizim, aynı kalite (4× örnekleme varsayılan), 100 000 parsel; ayrıntı ölçüm dosyalarındadır:

| Durum | Arayüz iş parçacığı, önce → sonra | GPU (geri okumayla), önce → sonra |
|---|---|---|
| Boş kare | 5,77 → 0,62 ms | 175,6 → 12,9 ms |
| İmleç çizimin üstünde (seçim aracı) | 5,64 → 0,52 ms | 173,8 → 12,4 ms |
| Çoklu çizgi çizerken imleç (kenet açık) | 5,75 → 0,68 ms | 173,9 → 13,1 ms |
| İmleç, hepsi seçili | 63,18 → 1,14 ms | 320,8 → 12,1 ms |
| Orta tuşla kaydırma (genel görünüm) | 6,16 → 5,87 ms | 169,5 → 172,5 ms |
| Bir nesneyi silme | 111,1 → 120,6 ms | 195,5 → 193,1 ms |
| Hepsini seç | 275,8 → 243,6 ms | 450,6 → 451,0 ms |

- İmleç, üzerine gelme ve arayüzün kendi olayları artık çizimin boyutundan bağımsızdır. 10 000 parselde de aynıdır: imleç 1,45 → 0,48 ms, GPU 28,3 → 12,1 ms.
- Kamerayı değiştiren kareler (kaydırma, yakınlaştırma), düzenleme ve seçim değişikliği resmi yeniden çizer; onlarda değişiklik beklenmiyordu.

- GPU sütunu resmin geri okunmasını da içerir (boş çizimde ≈13 ms). Pencerede geri okuma yoktur; imleç karesinin GPU'daki gerçek payı bundan da küçüktür.
- **Kalanlar** (TODOS.md PERF-08):
  - Tek nesnenin düzenlenmesi bütün stilli katmanı yeniden kurar: tek katmanda 100 000 parselde ≈110 ms ve katmanın GPU'ya yeniden yüklenmesi.
  - Kaydırma ve yakınlaştırma kamerayı değiştirir, resim yeniden çizilir: 100 000 parselin genel görünümünde GPU'da kare başına ≈170 ms. Stilli katmanın parçalarının kutusu bütün katmanı kapsar; yakın görünümde de görünmeyen nesneler köşe gölgelendiricisinden geçer.
  - Hepsini seç, seçimin vurgusunu bir kez kurar: ≈250 ms.

  Bunlar sonraki dilimlerdir. Katmanları mekânsal parçalara bölmek hem düzenlemeyi hem yakın görünümü küçültür; parçaların çizim sırası stil motorunun sembol düzeyi sırasını korumalıdır.

## Doğrulama

- `KENTOS_GPU_TESTS=1 cargo test -p kentos-render-wgpu --test gpu` gerçek GPU'da şunları sınar (`a_kept_picture_is_composed_again_with_the_hover_over_it`):
  - aynı kare tutulan resmi birleştirir;
  - başka üzerine gelme resmi tutar ve yeni vurguyu üstüne çizer;
  - kaydırma sahneyi yeniden çizer;
  - her durumda pikseller bütün çizilen kareninkilerdir (1× ve 4×).
- `cargo test -p kentos-desktop` masaüstünün bütün testlerini çalıştırır.
- `KENTOS_PERF_LABEL=after KENTOS_PERF_OUT=docs/perf cargo test --release -p kentos-desktop perf::frame -- --ignored --nocapture --test-threads=1` ölçümü yineler.
