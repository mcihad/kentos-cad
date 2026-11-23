# ADR 9201: Render motoru — gerçek dosyalarla ölçüm ve taban çizgisi

- **Durum:** kabul edildi (2026-11-23). Plan: [plan/render.md](../../plan/render.md) RM-01.
- **İstek:** [ADR 9200](9200-render-engine-research.md). Sahibin sözü: “performans iyileştirmeleri optimizasyonlar
  sağlamamız gerekiyor”.
- **Sözleşmeler:** yok. Yalnız ölçüm kodu (`#[ignore]` sınamaları) ve gerçek pencere sürücüsü değişti.
- **Bağımlılık:** yok.

## §1 Sorun

Render motorunun her adımı “önce” ve “sonra”yla ölçülecek. Var olan ölçüm (`perf::large`, ADR 1062) yalnız kodun
ürettiği karma çizimi ölçüyordu. Sahibin sıkıntısı ise gerçek dosyalarda: bir milyon nesnelik kent projesi, belediyenin
imar planı NCZ'si, büyük bir mimari DWG.

Gerçek pencere sürücüsünün tekerleği de her çentikten sonra 120 ms bekliyordu. Hızlı bir elin on çentiği arka arkaya
çevirmesi ölçülemiyordu.

## §2 Karar

1. **`perf::files` (`apps/desktop/src/shell/perf/files.rs`):** `KENTOS_PERF_FILES`'taki her dosyayı uygulamanın kendi
   açılışıyla, kendi sürecinde açar. `perf::large`'ın izini aynen kullanır: imleç, tekerlek, kaydırma; kare başına UI
   parçaları ve GPU, p50/p95. Buna eklenenler:
   - **Patlama:** on dört çentik tek karede, içeri ve dışarı. Pencere, geç kaldığı karede hızlı bir elin çentiklerini
     böyle verir. Ardından çizim alanının resmi değişmeyene kadar kareler çizilir; son değişimin anı yazılır.
   - **Tümü:** “Tümünü göster”in karesi ve gösterdiği kutu. 0,0'ın içinde olup olmadığı da yazılır.
   - **Seçim:** ortaya bir tık. O karede `App::view`'ın payı, yani arayüzün yeniden kurulması.
   - **Bellek:** açılıştan sonra ve en yüksek.
2. **Gerçek GPU, pencere açmadan.** `perf::large`'ın donanımı (`Harness`): iced'in ekransız wgpu çizicisi, kare çizilip
   geri okunur. Sahibin masaüstünde pencere açılmaz.
3. **Gerçek pencerede `burst`** (`scripts/usage/real/driver.py`):
   - N çentik verilen arayla, beklemeden.
   - Çizim alanı ilk çentikten önce, çentikler arasında ve sonra izlenir.
   - Her değişimin resmi kaydedilir.
   - Bölge 400 ms durgun ve uygulama boşta olunca durur.
   - Çıktı: ilk değişimin ilk çentikten, son değişimin son çentikten kaç ms sonra geldiği.

## §3 Taban çizgisi

Ortam: Intel Core i5-13500, UHD Graphics 770 (Mesa 26.0.8, Vulkan), release. Sayılar ve ham sonuç
[docs/perf/files-desktop-once-2026-11-23.md](../perf/files-desktop-once-2026-11-23.md)'dedir. Kararı belirleyenler:

1. **Tekerlek çentiği pahalı ve süre `App::view`'da gidiyor.**

   | Dosya | Görünüm (p95) | GPU (p50) | Bütçeyi aşan |
   |---|---|---|---|
   | sivas.kcad | 99,5 ms | 108 ms | 32 karenin 16'sı |
   | NCZ | 116 ms | 38–154 ms | 32 karenin 32'si |

   Görünüm kurulurken sahne yakınlık bandına göre yeniden yapılıyor (bütün çizimin eğrileri) ve yazılar yeniden
   yerleşiyor. İmleç ve kaydırma kareleri ise 1 ms UI ve 12–31 ms GPU. Yani pahalı olan kamera değil, kamera
   değişince yeniden üretilen sahne. Bu, ADR 9200 §3'ün çekirdek kararını (yeniden çiz ile yeniden üret ayrı) ölçüyle
   doğruluyor.
2. **Seçim arayüzü yeniden kuruyor.** DWG'de bir tık 106 ms UI tutuyor, bunun 65 ms'i `App::view`. Sahibin “obje
   seçtiğimde arayüz de yenileniyor” gözleminin ölçüsü bu.
3. **“Tümünü göster” çizimi bir noktaya indiriyor.**
   - NCZ'de kutu x'te −4,1 M ile +4,7 M arasına uzanıyor. Açılışta sığdırılan kutu ise 36 km'lik kent.
   - DWG'de de “Tümünü göster” 0,0'ı içine alıyor.

   Çizimde kentten binlerce kilometre uzakta duran nesneler var (RM-03).
4. **DWG açılışı 22,6 s, en yüksek bellek 3,7 GB.** DXF metin köprüsünün payı RM-21'de ölçülecek.
5. **Gerçek pencerede büyük NCZ** (lavapipe, hata ayıklama derlemesi), on dört çentik 30 ms arayla:

   | Yön | İlk değişim | Son değişim | Gösterilen ayrı resim |
   |---|---|---|---|
   | İçeri | ilk çentikten 1 074 ms sonra | son çentikten 1 704 ms sonra | 3 |
   | Dışarı | ilk çentikten 260 ms sonra | son çentikten 2 574 ms sonra | 4 |

## §4 Planda olmayan kararlarım

1. **GPU süresi zaman damgası sorgusuyla değil, geri okumayla ölçüldü.** Aygıtı iced açıyor ve özelliklerini ölçüm için
   değiştiremiyoruz. Geri okuma GPU'nun bitmesini bekler. Boş pencerenin geri okuma tabanı `perf::large`'da ayrıca
   ölçülüyor. Zaman damgası sorgusu, tuval kendi dokularına çizdiğinde (RM-07) eklenecek.
2. **Patlamanın kareleri ayrıca sayıldı.** Patlama tek karede verildiğinde son değişim, karenin süresi artı yazıların
   oturması (150 ms) oluyor. Bu, kullanıcının elinden ekrana geçen gerçek süredir.
3. **Ölçüm sahibin ayarlarıyla koştu:** gerçek boyutlu yazılar, çizgi kalınlıkları, MSAA 1. Sınama süreci sahibin
   ayar ve durum dosyalarını değiştirmedi (önce ve sonra özetleri aynı).

## Uygulama

- **Ölçüm:**
  - `apps/desktop/src/shell/perf/files.rs` (yeni).
  - `perf/large.rs`: `wheel`, `stats`, `one`, `adapter` ve `trace` modüle açıldı.
  - `perf/mod.rs`.
- **Gerçek pencere:**
  - `scripts/usage/real/driver.py`: `grab`, `burst`.
  - `count.py`: `burst` tekerlek sayılır.
  - `README.md`.
- **Tekrar üretmek:**

  ```text
  KENTOS_PERF_FILES=<dosyalar> KENTOS_PERF_LABEL=once KENTOS_PERF_OUT=docs/perf \
    cargo test --release -p kentos-desktop --lib perf::files::files -- --ignored --nocapture --test-threads=1
  ```
