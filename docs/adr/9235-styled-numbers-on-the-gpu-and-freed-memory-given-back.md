# ADR 9235: Stilli katmanların sayıları GPU'da yaşar, açılışın bıraktığı bellek geri verilir

- **Durum:** kabul edildi (2026-11-25). Plan: [plan/render.md](../../plan/render.md) RM-19, 1. adım.
- **İstek:**
  - planın RM-19 maddesi: “Büyük NCZ açılırken en yüksek kullanım 3,8 GB; 1,5 GB'ın altına inecek”;
  - sahibin “LOD ve performans” isteği;
  - 25 Kasım: “Devam et, asla durma, bitene kadar tüm maddeler.”
- **Sözleşmeler:** değişmedi. `.kcad`, komut, ayar ve stil topluluklarının biçimi aynı; GPU aynı resmi çiziyor.

## §1 Bulgu

**Ölçüm aracı:** `perf::memory::file_parts` (yeni).

- Dosyayı uygulamanın açtığı gibi açar.
- Sonra belleği tutan parçaları tek tek bırakır: çizim alanının sahnesi, yazılar, geometri deposu, belge, GPU.
- Her adımda okur:
  - yerleşik bellek (glibc'nin boş sayfaları `malloc_trim` ile geri verildikten sonra);
  - tepe;
  - Rust yığınında yaşayan bayt: `heap-count` özelliğiyle derlenen sayan ayırıcı (yalnız ölçümün).
- `KENTOS_PERF_STAGES=1` açılışı aşama aşama ölçer.

Kent planının NCZ'si: 454 651 nesne, hata ayıklama derlemesi, UHD 770. Açık çizimde 1 870 MB Rust yığını vardı:

- **Stilli katmanların sayıları: 644 MB.**
  - 322 MB float32 veri, 322 MB onun artığı ([ADR 0878](0878-styled-geometry-low-part-stream.md)).
  - Aynısı GPU'da da duruyordu: çizici her parçanın `Arc`'ını yükledikten sonra da tutuyor, çizim alanının
    önbelleği de tutuyordu.
  - Diziler toplu toplu büyütülerek yazıldığı için ayrılan alan gerçek uzunluğun 2,6 katıydı.
- **Belge: 678 MB. Geometri deposu: 424 MB.** Bu adımın dışında (§4).
- **Tepe 2 878 MB.**
  - Dosyayı belgeye okuyan iş parçacığının tepesi 1 998 MB; belge 1 055 MB'ını tutuyor.
  - Kalan yaklaşık 940 MB boş ama glibc onu o iş parçacığının alanında tutuyor.
  - İlk karenin stil kurulumu bunun üstüne geliyordu.

## §2 Karar

### §2.1 Stilli katmanların sayıları GPU'da yaşar

`kentos-render-wgpu` `StyledLayerPart`, katmanın sayılarını (`data`, `low`) toplularından ayrı tutar:

- **`StyledLayerPart::on_gpu`:** bir görünüm yükledikten sonra CPU sayıları bırakır. Çizim alanının iki stil önbelleği
  (çizim ve derin görünüm) parçalarını böyle kurar.
- **İstisna:** toplularından biri çizilirken CPU'da okunan parça sayılarını tutar. Bunlar işaretlerdir (maskenin
  kutuları) ve vektörleriyle çizilen sembollerdir.
- **`new` ile kurulan parça** sayılarını hep tutar: hizmet, veri katmanı ve sembol resimleri.

GPU'daki kopya tek kopya olduğu için:

- **Görünüm, uygulama parçayı tuttuğu sürece onun tamponlarını tutar:**
  - sahnede olmayan parça (gizli katman, derin görünümün altındaki çizim) bekletilir;
  - çizilmeyen görünüm (paftanın altında kalan çizim alanı) bırakılmaz;
  - uygulama parçayı bırakınca tamponlar da gider.
- **Tamponları olmayan bir çizici** (yeniden kurulmuş bir çizici) o parça için bir şey çizmez ve `parts_lost` der.
  Çizim alanı da o kare için stilli katmanlarını bir kez yeniden kurar.

Çizicinin sayıları okuyan yerleri (yükleme, kesikli çizginin ilerlemesi, maske kutuları) artık parçanın sayılarını
ayrıca alıyor. `batches::merged_order` ve `svg_marks::boxes` toplularla ve sayılarla çalışıyor, katmanın kendisiyle
değil.

### §2.2 Sayılar tam boyunda

Stil çekirdeğinin `BatchSink::finish` adımı:

- katmanın sayılarının adedini önce sayar ve diziyi tam o boyda ayırır;
- her toplunun kendi f64 sayılarını katmana geçtikçe bırakır.

### §2.3 Açılışın bıraktığı bellek geri verilir

Masaüstündeki `give_back` modülü glibc'nin `malloc_trim` çağrısını yapar:

- **Açılışın iş parçacığı:** belgeyi uygulamaya vermeden önce kendi bıraktığını geri verir.
- **Çizim gösterilince ve 250 ms'yi geçen bir stil kurulumundan sonra:** 2 saniye sonra kendi iş parçacığında bir kez
  daha geri verir. Arada gelen istekler tek bir geri vermede birleşir.
- **Yalnız Linux glibc'de.** Windows'un yığını büyük blokları bırakılır bırakılmaz geri veriyor.

## §3 Kanıt

**Sınamalar:**

- `kentos-render-wgpu`, `numbers_on_the_gpu_only` (yeni GPU sınaması, `tests/gpu/on_gpu.rs`, UHD 770). Sınanan:
  - yükledikten sonra CPU sayıları bırakıyor;
  - dört kare gizli kalan parça, bekletilen tamponlarından aynı resmi çiziyor;
  - çizilmeyen görünüm bırakılmıyor;
  - yeni bir çizici o parçayı çizmiyor ve `parts_lost` diyor; eski çizici çiziyor;
  - parça bırakılınca tamponlar ve görünüm gidiyor;
  - işaretli parça ve `new` parça sayılarını tutuyor.
- Aynı kalan sınamalar:
  - `kentos-render-wgpu`'nun bütün sınamaları (GPU dahil);
  - stil çekirdeği ve native stilin sınamaları, donmuş fikstürler dahil;
  - masaüstünün stil, çizim alanı, altın görüntü, GPU uygunluğu, pafta ve veri katmanı sınamaları (208);
  - Mimari GPU uygunluğu (9 durum): tazeden farklı 0 piksel.

**Ölçüm** (`perf::memory::file_parts`, hata ayıklama derlemesi, NCZ, açılış uygulamanın yaptığı gibi):

| | Önce | Sonra |
|---|---|---|
| Açık çizimde Rust yığını | 1 870 MB | 1 274 MB |
| CPU'daki stilli sayılar (ayrılan) | 644 MB | 48 MB (işaret katmanları) |
| Açılışın tepesi | 2 878 MB | 1 996 MB |
| Açık çizimde yerleşik (boş sayfalar verildikten sonra) | 2 128 MB | 1 772 MB |

**Gerçek pencere** (Xvfb ve lavapipe, hata ayıklama derlemeleri, sürücünün yeni `memory` komutu):

- Senaryo: NCZ açılıp oturdu, rapor kapatıldı, sekiz çentik yakınlaşıldı.

  | | Önce | Sonra |
  |---|---|---|
  | Yerleşik | 3 647 MB (yığın 2 813) | 2 595 MB (yığın 1 710) |
  | Tepe | 4 389 MB | 3 327 MB |

- lavapipe GPU tamponlarını sürecin yığınında tutuyor; gerçek GPU'da sayılar daha düşük.
- İki görünümün çizim alanında farklı piksel 0.
- Görüntü: `.run/gui-evidence/render/rm19/once-sonra-bellek.png`.

## §4 Kalan (RM-19'un sonraki adımları)

1,5 GB'a inmek için kalanlar:

- **Belge, 706 MB.**
  - Nesne tipi 1 264 bayt: en büyük türlerin boyu (mekân 1 224, duvar 1 096; ikisinde de 512 baytlık `ArchInfo` satır
    içinde).
  - NCZ'nin nesneleri çizgi (384 bayt), yazı (616) ve çoklu çizgi (976; bunun 512'si yine `ArchInfo`).
- **Geometri deposu, 450 MB.** `Item` 472 bayt, bunun 384'ü `Shape`. Aynı sebep: seyrek türlerin boyu.
- **Okuma tepesi.** Okuyucunun `Vec<Entity>`'si ile belgenin her nesneye ayırdığı kopya yan yana duruyor.

## Uygulama

**`crates/render/wgpu/src/styled/gpu.rs`:**

- `StyledLayerPart::{new, on_gpu, batches, pictures, dropped, numbers, on_gpu_only, to_layer}`, `Numbers`;
- yükleme ve kaybolan parça;
- `ViewStyled::{parked, lost, holds_only_copies}`;
- maske kutuları.

**Çizicinin geri kalanı:**

- `crates/render/wgpu/src/styled/dashes.rs`: `upload_values` toplular ve sayılarla.
- `crates/render/wgpu/src/renderer.rs`: `trim` yalnız kopyayı tutan görünümü bırakmaz; `parts_lost`.
- `crates/render/wgpu/src/stats.rs`: `FrameStats::parts_lost`.
- `crates/render/wgpu/tests/gpu/on_gpu.rs`, `tests/gpu.rs`: yeni sınama; öbür sınamalar `StyledLayerPart::new` ile.

**Stil:**

- `crates/native/style/src/batches.rs`: `merged_order` toplularla; `Picture` yeniden dışa açıldı.
- `crates/native/style/src/svg_marks.rs`: `boxes` sayılarla.
- `crates/shared/style-core/src/style/batch.rs`: `finish` tam boy ayırır; `Entry::drawn`.

**Masaüstü:**

- `apps/desktop/src/cad/style/scene.rs`: parçalar `on_gpu`; büyük kurulumdan sonra geri verme.
- `apps/desktop/src/shell/give_back.rs` (yeni): `soon`, `now`.
- `apps/desktop/src/shell/opening.rs`: açılış iş parçacığı geri veriyor.
- `apps/desktop/src/shell/app/settings.rs`: `show_document` geri veriyor.
- `apps/desktop/src/shell/viewport/{mod,state}.rs`: kaybolan parçada yeniden kurma; ölçümün yardımcıları.
- `apps/desktop/src/shell/perf/memory.rs`: `file_parts`, `heap-count`.
- `apps/desktop/Cargo.toml`: `heap-count` özelliği.

**Gerçek pencere sürücüsü:**

- `scripts/usage/real/driver.py`, `README.md`: `memory AD`.

**Önceki işlerden kalan iki uyarı:**

- `crates/native/dwg/src/regions.rs`;
- `crates/render/wgpu/src/scene.rs`.
