# ADR 9237: Geometri deposunun nesneleri ve etiketlerin belleği kendi boyları kadar

- **Durum:** kabul edildi (2026-11-25). Plan: [plan/render.md](../../plan/render.md) RM-19, 3. adım.
- **İstek:**
  - planın RM-19 maddesi: “Büyük NCZ açılırken en yüksek kullanım 3,8 GB; 1,5 GB'ın altına inecek”;
  - 25 Kasım: “Devam et, asla durma, bitene kadar tüm maddeler.”
- **Sözleşmeler:** değişmedi.
  - Geometri çekirdeğinin JSON'u (çağrı fikstürleri) aynı.
  - Stil topluluklarının donmuş fikstürleri ve `.kcad` aynı.

## §1 Bulgu

[ADR 9236](9236-an-object-as-large-as-it-is.md) sonrasında, NCZ'yi (454 651 nesne) tutan masaüstünün `Spatial`'ı
451 MB'tı. `perf::memory::file_parts` onu iki parçaya ayırdı: yeni kurulan depo 251 MB, ana görünümün etiket
önbellekleri 196 MB.

**Depo:**

- 251 MB'ın 205'i nesnelerin satır içi kaydı: `Item` 472 bayt, bunun 384'ü `Shape`.
- `Shape` en büyük varyantı kadar. Seyrek türlerin iri parçaları satır içindeydi:
  - tarama 384 bayt (deseni 184, ilişkisi 128);
  - ölçü 368 (görünüşü 192);
  - tablo 336;
  - açıklık 320 (özelliği 280);
  - raster 288;
  - duvar 272 (özelliği 224).

**Etiketlerin belleği** (`LabelMemo`, [ADR 0786](0786-text-layer-object-by-object.md) §1):

- Görüş alanındaki her nesneye bir `Kept` açıyordu: damga, parçalar, iki boş tampon.
- NCZ'nin 454 649 nesnesinden yazısı olan 112 127. Yazısı olmayan 342 522 çizgi ve alan da kaydını taşıyordu.

## §2 Karar

### §2.1 Yazısı olmayan nesne bir damgadan ibaret

`LabelMemo`, sade okunan ve yazı göstermeyen nesneyi ayrı bir tabloda tutar (`Blank`: damga, son soru, parça sayısı;
24 bayt). Sade okunan nesne: blok parçası, örtüsü, tablo kutusu olmayan nesne.

- Damga değişmedikçe nesne yeniden okunmaz, önceden olduğu gibi: `taken` ve `made` sayıları aynı.
- Döndürülen boş parçalar herkesin paylaştığı tek bir boş `Part`.
- Yazı gösteren nesne `Kept` olarak kalır.

### §2.2 `Shape`'in iri parçaları kutuda

En büyük altı varyantın iri alanları `Box`'a alındı:

| Varyant | Kutudaki alanlar |
|---|---|
| Tarama | `pattern`, `assoc` |
| Ölçü | `look` |
| Tablo | `source`, `breaks`, `face` |
| Açıklık | `opening` |
| Raster | `style`, `dataset`, `tiles` |
| Duvar | `wall` |

- Sonuç: `Shape` 248 bayt (en büyüğü artık tablo), `Item` 336 bayt.
- JSON'un düzleştirilmiş parçaları için `Flat`, `Box<T>`'ye iletilir.
- Daha aşağısı için tablonun ve rasterin tümden kutulanması gerekir: 84 kullanım yeri; bu adımın dışında.

## §3 Kanıt

**Sınamalar:**

- Geometri çekirdeği, stil çekirdeği, native stil, sahne sorgusu, biçimler, etkileşim, uygulama ve çizim çıktısında
  3 960 sınama geçti. Çağrı ve stil fikstürleri bayt bayt aynı.
- Kalan beş kırmızı main'de bu işten önce de vardı ([ADR 9236](9236-an-object-as-large-as-it-is.md) §3).
- Masaüstünün nesne, stil, etiket, çizim alanı ve GPU sınamaları (612), Mimari (205) ve harita (195) geçti.

**Ölçüm** (`perf::memory::file_parts`, hata ayıklama, NCZ, açılış uygulamanın yaptığı gibi):

| | 2. adımdan sonra | Bu adımdan sonra |
|---|---|---|
| Etiketlerin önbellekleri | 196 MB | 118 MB |
| Geometri deposu (yeni kurulmuş) | 251 MB | 192 MB |
| Açık çizimde Rust yığını | 941 MB | 815 MB |
| Açılışın tepesi | 1 712 MB | 1 561 MB |
| Açık çizimde yerleşik (boş sayfalar verildikten sonra) | 1 486 MB | 1 324 MB |

- Hata ayıklama derlemesinin yerleşik belleğinin yaklaşık 300 MB'ı ikilinin dosya eşlemesi.
- Sürüm derlemesindeki ölçüm: §4.

## §4 Sürüm derlemesinde: RM-19'un hedefi

**`perf::files`** NCZ'yi gerçek GPU'da (UHD 770) uygulamanın açtığı gibi açtı (`docs/perf/files-desktop-rm19-2026-11-25.json`;
öncesi `files-desktop-rm13-gpu-2026-11-24.json`):

| | 24 Kasım | Şimdi |
|---|---|---|
| Açılış sonrası yerleşik | 2 448 MB | 1 458 MB |
| Bütün koşunun tepesi (yakınlaşma ve patlama denemeleri dahil) | 2 572 MB | 1 504 MB |
| Koşu sonunda yerleşik | 2 572 MB | 1 258 MB |
| Açılış | 3,34 s | 3,08 s |

**`perf::memory::file_parts`** (sürüm, iki kez):

- açılışın tepesi 1 485 / 1 484 MB;
- oturmanın tepesi 1 511–1 524 MB (etiketler);
- açık çizimde yerleşik 1 190 / 1 218 MB.

Planın hedefi (“açılırken 1,5 GB'ın altına”) tuttu.

**GPU'da tutulan bayt 243 → 430 MB çıktı.** Bu adımın işi değil: [ADR 9223](9223-screen-curves-round-at-every-zoom.md)
(eğriler her yakınlıkta çeyrek piksel) çizilen öğeleri 701 712 → 1 159 737'ye çıkardı. Park edilmiş katman yok
(`FrameStats::parked_bytes` = 0).

**Gerçek pencere** (Xvfb ve lavapipe, hata ayıklama, oturumlar sırayla, ikişer kez):

| | 2. adım | Bu adım |
|---|---|---|
| Yerleşik | 2 569 / 2 471 MB | 2 379 / 2 362 MB |
| Tepe | 2 916 / 2 882 MB | 2 773 / 2 773 MB |

- RM-19 öncesinde yerleşik 3 706 MB, tepe 4 440 MB'tı.
- Bütün adımlar boyunca çizim alanında farklı piksel 0.
- Görüntü: `.run/gui-evidence/render/rm19/once-sonra-rm19.png`.

**Kalan tepe** ilk karede: stilli katmanların sayıları GPU'ya yüklenene kadar bir an CPU'da birlikte duruyor (430 MB).
Bunu kesmek parçaları kuruldukça yükleyen bir akış ister: RM-07'nin (ayrı iş parçacığında çizim) işi.

## Uygulama

**Etiketlerin belleği:**

- `crates/shared/geometry-core/src/store/label_memo.rs`: `Blank`, `blank`, ortak boş parça, `held` (ölçümün).

**`Shape` ve okuyanları:**

- `crates/shared/geometry-core/src/entity.rs`: `Shape`'in kutudaki alanları; JSON'un düzleştirilmiş tipleri.
- `crates/shared/geometry-core/src/api/json.rs`: `Flat for Box<T>`.
- Geometri çekirdeğinin `geom/`, `ops/`, `store/` modülleri ve sınamaları.
- `formats`, `style-core`, `scene-query`, `domain`, `application`, `interaction`, `render-wgpu`: kurulumlar ve
  okumalar.

**Ölçüm:**

- `crates/native/scene-query/src/spatial.rs`: `label_memory_report`, `forget_label_memory` (gizli, ölçümün).
- `crates/render/wgpu/src/{stats,renderer}.rs`, `styled/gpu.rs`: `FrameStats::parked_bytes`.
- `apps/desktop/src/shell/perf/memory.rs`: depo ayrıca kurulup ölçülüyor, etiketlerin önbellekleri ayrı; aşamalar
  kırpılmadan (`KENTOS_PERF_STAGES=2`).
- `apps/desktop/src/shell/perf/files.rs`: rapora `parkedBytes`.
- `docs/perf/files-desktop-rm19-2026-11-25.json`: sürüm ölçümü.
