# ADR 9246: Uzun çizgi ve işaret koşuları tek örnekli çizilip bindirilir

- **Durum:** kabul edildi (2026-11-25).
- **Plan:** [plan/render.md](../../plan/render.md) RM-10, resmi değiştirmeyen hız.
- **İstek:**
  - kabul ölçütü: “Kamera değişikliğinden onu gösteren kareye ≤ 16 ms”;
  - 24 Kasım: “kaliteden ödün verilmez”.
- **Sözleşmeler:**
  - Paylaşılan WGSL değişmedi.
  - `RenderSettings::single_runs`, öntanımlı açık.
  - `FrameStats::single_runs`: çizilen tek örnekli koşu sayısı.
  - Ölçüm anahtarı `KENTOS_DIAG_NO_SINGLE`.

## §1 Bulgu

Uzak karenin bedeli 4× MSAA ([plan](../../plan/render.md) RM-10, `tests/gpu/far_cost.rs`, UHD 770):

| 1 M piksel altı çizgi | Kare |
|---|---|
| 4× MSAA | 51,6 ms |
| 1× | 17,9 ms |

- Bedel ilkel başına, alan başına değil: çizginin dörtgenini %37 küçültmek yalnız %6 kazandırdı.
- NCZ'nin açılış karesinin öğelerinin %77'si çizgi, %20'si işaret.
- 24 Kasım'ın uzak düzeyleri ve hareketteki tek örnek bu bedeli düşürüyordu, ama resmi değiştirdikleri için kalktılar
  ([ADR 9222](9222-quality-is-not-traded-for-speed.md)). release'te uzak tekerlek karesi sivas.kcad'de 39 → 102 ms,
  NCZ'de 40 → 90 ms oldu.

## §2 Karar

### §2.1 Resim neden aynı

Çizgi ve işaret kenarını kendi gölgelendiricisinde yumuşatır:

- dörtgeni kaplamanın bittiği yerden bir piksel öteye uzanır;
- en uzak örnek piksel merkezinden ~0,53 px uzaktadır.

Dolayısıyla atılmayan her parçası pikselinin dört örneğini de kaplar ve dördüne aynı rengi yazar.

- Örneklere tek tek karıştırılan aynı renk, ortalamada bir kez karıştırılmış renktir: “üstüne” birleştirilebilir,
  ortalama hedefe göre doğrusaldır.
- Bu yüzden bir koşunun çizgileri tek örnekli bir resme önceden çarpılmış olarak toplanıp her örneğe bindirilince,
  resim aynı kalır.
- Fark yalnız yuvarlamada: örnekli resim her karıştırmayı 8 bite yuvarlar, toplanan resim bir kez yuvarlanır.

Dolgu için bu doğru değil: kenarını örnekler yumuşatır ve kenarda örnekleri farklıdır. Dolgular örneklerde kalır.

### §2.2 Ne zaman

Şu koşulların hepsi tutmalı:

- **Kare bütün çiziliyor:** tekerlek ya da durma karesi. Kaydırmada taşınan resim
  ([ADR 1067](1067-moved-picture-while-panning.md)) ve tekerlekte ölçeklenen resim
  ([ADR 1068](1068-zoomed-picture-while-the-wheel-turns.md)) eskisi gibi; şeritleri örneklerde çizilir.
- **Hedef çok örnekli:** tek örnekte kazanılacak bir şey yok.
- **Koşu:** çizim sırasında başka türden toplu çizimlerin arasında kalan, çizilen çizgiler, işaretler ve blokların
  çizgileri. Efekti olmamalı ve en az 100 000 öğe çizmeli (`SINGLE_FROM`). Bir bindirme tümleşik GPU'da ~1 ms, bir
  milyon çizgi ~32 ms kazandırıyor.

### §2.3 Nasıl

Birleştirici ([ADR 0226](0226-symbol-layer-extras.md)) iki geçiş ekler:

- **`Pass::Single`:** koşuyu tek örnekli, yarım kayan noktalı bir resme çizer (`Rgba16Float`). Renk ve alfa
  bindirilene kadar yuvarlanmaz.
- **`Pass::Layer`:** resmi opaklık 1 ile çizimin üstüne bindirir (katmanın bindirme geçişi).

Stilli boru hatlarının bu resme çizen takımı `FLOAT_PICTURE` adıyla tutulur (örnek sayısı 0 onu adlandırır).

**Yalnız koşu için birleşen kare** (`single_only`) içeriğini düz bir kare gibi tutar. Ardından gelen kaydırma resmi
taşıyabilir; birleşen bir kare eskiden resmini tutmuyordu.

## §3 Kanıt

**GPU eşitlik sınaması** (UHD 770, 4×): 300 dolgu, 150 000 yarı saydam çizgi (düz ve kesikli) ve 20 000 dış hatlı
daire, ardından bir dolgu daha. 147 456 piksel.

| Durum | Farklı | 1 adım | 2 adım | 3 adım |
|---|---|---|---|---|
| Uzakça | 103 385 | 97 109 | 6 201 | 75 |
| Yakın | 92 149 | 89 517 | 2 624 | 8 |

- 8 bitlik bir ara resimle en çok 4 adımdı (289'u 3, 8'i 4). Kalan farklar örneklerin karıştırma başına kendi
  yuvarlaması.
- HiDPI kapalı ve 1 örnekte koşu çizilmiyor; koşul kaldırılınca sınama kırılıyor.

**Ölçüm sınaması** (1 M parça, UHD 770):

| | Örneklerde | Tek örnekli koşu | 1× |
|---|---|---|---|
| Düz | 50,9 ms | 18,8 ms | 17,9 ms |
| Kesikli | 51,5 ms | 20,3 ms | |

**NCZ** (hata ayıklama derlemesi, `perf::files`, UHD 770):

| | Koşusuz | Koşulu |
|---|---|---|
| Uzak tekerlek p50 | 96,1 ms | 65,3 ms |
| Uzak tekerlek p95 | 109,4 ms | 75,1 ms |
| Kaydırma | 18,0 ms | 17,8 ms |
| Yakın | 17,1 ms | 17,3 ms |

Harness'ın ~12,7 ms'lik tabanı çıkarılınca uzak karede çizimin payı 83,4 → 52,6 ms (−%37).

**release** (`docs/perf/files-desktop-rm10-tek-2026-11-25.json`, önceki `…-rm08-son-…`, UHD 770). Harness'ın tabanı
~12 ms.

| Dosya | Uzak tekerlek p50 | Uzak tekerlek p95 |
|---|---|---|
| sivas.kcad | 101,6 → 75,2 ms (−%26) | 108,1 → 79,0 ms |
| NCZ | 90,2 → 59,5 ms (−%34) | 103,5 → 70,8 ms |
| AVM DWG | 22,3 → 19,5 ms | 37,6 → 33,9 ms |
| Suşehri DXF | 20,6 → 15,0 ms (−%27) | 21,7 → 16,5 ms |

Kaydırma, yakın görünüm ve bellek değişmedi. sivas.kcad ve NCZ'nin uzak karesi ölçütün hâlâ üstünde.

**Gerçek pencere** (Xvfb ve lavapipe, NCZ, önceki ikiliyle):

| Görüntü | Farklı | 1 adım | 2 adım | 3–4 adım |
|---|---|---|---|---|
| Açılış | 64 504 | 61 131 | 3 220 | 153 |
| 8 çentik | 161 305 | 154 211 | 6 990 | 104 |
| 16 ve 20 çentik | 0 | | | |

**Paketler:** render-wgpu'nun bütün sınamaları (GPU dahil), masaüstünde 320 sınama (birleştirme, maske, efekt,
kararlılık dahil) ve Mimari'nin 6 render uygunluk sınaması geçti.

## Uygulama

- `crates/render/wgpu/src/compose.rs`: `Tex::Single`, `Pass::Single`, `Planner::single`, `DrawBatches::draw_single`.
- `crates/render/wgpu/src/renderer.rs`: karar (`single_runs`, `single_only`), `plain`, tutma.
- `crates/render/wgpu/src/styled/gpu.rs`: `ViewStyled::single_runs`, `whole_pixels`, `work`, `FLOAT_PICTURE`.
- `crates/render/wgpu/src/settings.rs`, `stats.rs`: `single_runs`.
- Sınamalar: `tests/gpu/single.rs`; `tests/gpu/far_cost.rs` (varyantlar).
