# ADR 9236: Belgedeki nesne kendi boyu kadar yer tutar

- **Durum:** kabul edildi (2026-11-25). Plan: [plan/render.md](../../plan/render.md) RM-19, 2. adım.
- **İstek:**
  - planın RM-19 maddesi: “Büyük NCZ açılırken en yüksek kullanım 3,8 GB; 1,5 GB'ın altına inecek”;
  - 25 Kasım: “Devam et, asla durma, bitene kadar tüm maddeler.”
- **Sözleşmeler:** değişmedi.
  - `.kcad`, JSON, komut ve ayar biçimleri aynı.
  - TypeScript tiplerinin üretimi bayt bayt aynı.
  - Nesnelerin JSON şeması aynı.

## §1 Bulgu

[ADR 9235](9235-styled-numbers-on-the-gpu-and-freed-memory-given-back.md) sonrasında NCZ'nin (454 651 nesne) belgesi
706 MB tutuyordu.

- **Belge her nesneyi kendi `Arc`'ında saklıyor ve `Entity` 1 264 bayt.** Bir enum en büyük türü kadar yer tutar:
  mekân 1 224, duvar 1 096, kolon 1 064 bayt.
- **Bu türler NCZ'de hiç yok.** NCZ'nin nesneleri:
  - 187 202 çizgi (gereken 384 bayt);
  - 112 127 yazı (616);
  - 108 114 çoklu çizgi ve alan (976);
  - geri kalanı yay, daire, nokta.
- **Şişiren parçalar, çoğu nesnede boş ya da seyrek:**
  - 512 baytlık `ArchInfo` (kat, evre, durum, yangın): her Mimari türün alanlarında, blok referansında
    (`InsertEntity::arch`) ve çoklu çizginin oda ayırıcı rolünde (`space_separator`) satır içinde duruyordu.
  - Tablonun kaynağı, ölçünün görünüşü, taramanın deseni ve ilişkisi, yazının paragrafı, yazı yüzü ve yolu,
    kılavuzun ucu.
- **Okuma tepesinde iki kez sayılıyordu.** Okuyucunun `Vec<Entity>`'si ile belgenin her nesneye ayırdığı kopya,
  ikisi de 1 264 bayt, yan yana duruyordu.

## §2 Karar

Nesnenin seyrek ya da iri parçaları nesnenin dışında, bir işaretçinin arkasında tutulur. `Entity` 480 bayt olur:
çoklu çizginin (472) boyu.

### §2.1 `Lean<T>`: çoğu nesnede varsayılan kalan değer

`kentos_contracts::Lean<T>` (`core/lean.rs`) değeri bir `Box`'ta tutar ve varsayılan olduğu sürece hiç tutmaz.

- **Okuma:** değerin kendisi gibi okunur (`Deref`). Yoksa varsayılan okunur; varsayılanı tür bir kez kurar
  (`lean_default!`).
- **Yazma** (`DerefMut`) değeri var eder.
- **JSON, TypeScript, şema:** değerin kendisininki. Dosyadan okunan varsayılan değer yok sayılır.

Kullanıldığı yerler:

- `InsertEntity::arch` (`ArchInfo`);
- `TextEntity::paragraph` (`Paragraph`), `TextEntity::face` (`TextFace`).

### §2.2 `Box`: seyrek ya da hep dolu ama iri olan

- **Mimari ve CBS türlerinin alanları:** duvar, açıklık, mekân, döşeme, ızgara, kolon, merdiven, rampa, kiriş, çatı,
  korkuluk, görünüş işareti, raster, nokta bulutu.
  - Bunların hepsi `{ base, alanlar }` biçiminde; alan yapısı (`WallFields`…) kutuya alındı.
  - Nesne 296 bayt kaldı; `ArchInfo` alanlarla birlikte yığına gitti.
- **Seçenekli parçalar `Option<Box<T>>`:**
  - çoklu çizginin `space_separator`'ı;
  - yazının `path`'i;
  - taramanın `assoc`'u;
  - kılavuzun `tip`'i.
- **Hep dolu olanlar `Box<T>`:** taramanın `pattern`'i.
- **Kutuya alınan varyantlar:** `Entity::Table` ve `Entity::Dimension`. İkisi de seyrek ve iri (952 ve 800 bayt);
  kendi alanlarında tek bir iri parça yok.

Okuyan kod değişmedi; alanlar kutunun içinden okunuyor. Değiştirilen yerler kurulumlar, bir değeri dışarı alanlar ve
örüntüyle açanlar.

## §3 Kanıt

**Biçimin aynılığı:**

- `cargo test -p kentos-contracts` TypeScript tiplerini yeniden yazdı: fark yok.
- Sözleşmenin 1 675 sınaması (şema dahil) geçti.
- `.kcad` (2 124 + 281), DXF ve biçimler (221 + 143), komut, domain, NCZ ve DWG sınamaları geçti.
- `Lean`'in kendi sınamaları: varsayılan değer yok sayılır ve varsayılan okunur; boyu bir işaretçi; JSON'u değerin
  kendisininki.

**Davranışın aynılığı:**

- Uygulama, etkileşim, sahne sorgusu, çizim çıktısı ve işleme paketlerinde 2 847 sınama geçti. Kalan altı kırmızının
  altısı da bu değişiklikten önce main'de aynı (ayrı bir çalışma ağacında ölçüldü):
  - `file_digest` ve çizim çıktısının bandı: CBS projesinde bağlı katman kuralı;
  - modül özelliklerinin listesi;
  - IFC/BCF'nin iki yazımı;
  - komut fikstürünün blok kitaplığı sayısı (40 → 61);
  - mimari olgu fikstüründe bir kaçış yolunun başlangıcı (4,45 → 11,55).
- Masaüstünün nesne, stil, çizim alanı ve GPU sınamaları (607) geçti. Mimari paketinin 205, harita paketinin 195
  sınaması (GPU uygunluğu dahil) geçti.
- Bütün çalışma alanı derleniyor (wasm paketleri dahil); dokunulan dosyalarda lint uyarısı yok.

**Ölçüm** (`perf::memory::file_parts`, hata ayıklama, NCZ, açılış uygulamanın yaptığı gibi):

| | 1. adımdan sonra | Bu adımdan sonra |
|---|---|---|
| `Entity` | 1 264 bayt | 480 bayt |
| Belge | 706 MB | 341 MB |
| Açık çizimde Rust yığını | 1 274 MB | 941 MB |
| Açılışın tepesi | 1 996 MB | 1 712 MB |
| Açık çizimde yerleşik (boş sayfalar verildikten sonra) | 1 772 MB | 1 486 MB |

**Gerçek pencere** (Xvfb ve lavapipe, hata ayıklama derlemeleri, oturumlar sırayla, ikişer kez; NCZ açılıp oturdu ve
yakınlaşıldı):

| | RM-19 öncesi | 1. adım | 2. adım |
|---|---|---|---|
| Yerleşik | 3 706 / 3 680 MB | 2 780 / 2 784 MB | 2 549 / 2 550 MB |
| Tepe | 4 440 / 4 423 MB | 3 507 / 3 513 MB | 2 946 / 2 960 MB |

- Öncesi ile 2. adım arasında çizim alanında farklı piksel 0 (iki görünüm).
- Görüntü: `.run/gui-evidence/render/rm19/once-adim1-adim2.png`.
- **Ölçüm notu:** iki oturum aynı anda koşunca (iki ayrı Xvfb'de) sayılar birkaç yüz MB oynadı. Tablodakiler sırayla
  koşan oturumların.

## §4 Ayrıca

**Dosya adı olmayan IMAGE** ([ADR 9227](9227-dwg-regions-wipeouts-and-images.md) ek):

- Boyu varsa artık çerçevesiyle alınıyor ve rapor dosyasının adlandırılmadığını söylüyor. Önce çerçevesiyle birlikte
  dışarıda kalıyordu.
- Boyu olmayan (yalnız yerleştirme noktası olan) yine alınmıyor.
- Masaüstünün içe aktarma sınaması bu yüzden kırmızıydı: alınmayanların uyarısı içe aktarmanın özetinden sonra
  geliyor. Sınama artık özeti son üç iletide arıyor.

**Kalan (RM-19'un sonraki adımları):**

- Geometri deposu 449 MB: `Item` 472 bayt, bunun 384'ü `Shape`; aynı sebep.
- Okumanın tepesi.

## Uygulama

**Sözleşme:**

- `crates/shared/contracts/src/core/lean.rs` (yeni), `lib.rs`.
- `entity.rs`, `annotation.rs`, `arch/*.rs`, `gis/raster.rs`, `gis/pointcloud.rs`: alan ve varyant tipleri.

**Kullanan paketler:** kurulumlar, dışarı alınan değerler ve örüntülerle açılanlar.

- `kcad`, `formats`, `domain`, `scene-query`, `application`, `interaction`, `processing`, `plot`, `headless`;
- masaüstü, Mimari ve harita uygulamaları ile sınamaları.

**Bu adımla gelenler:**

- `crates/shared/formats/src/dxf/emit.rs`: adı olmayan görüntünün çerçevesi; sınaması `tests/all/dxf.rs`.
- `apps/desktop/src/shell/exchange/tests.rs`: içe aktarmanın özeti.
- `crates/native/processing/src/files.rs`: kutulanmış raster alanlarından `Beside`.
