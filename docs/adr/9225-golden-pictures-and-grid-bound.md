# ADR 9225: Altın görüntüler; ızgaranın sınırı; çizimden çok uzağa uzanan nesne

- **Durum:** kabul edildi (2026-11-25). Plan: [plan/render.md](../../plan/render.md) RM-16, 3. adım.
- **İstek:**
  - planın RM-16 maddesi: “Küçük fikstürler sabit kameralarda çizilip altın görüntülerle karşılaştırılır”;
  - sahibin “kaliteden ödün verilmez” sözü ([ADR 9222](9222-quality-is-not-traded-for-speed.md));
  - 25 Kasım: “Devam et, asla durma, bitene kadar tüm maddeler.”
- **Sözleşmeler:** değişmedi. Kapsam denetiminin “uzak” kuralı genişledi (§2.3); komut, ayar ve `.kcad` aynı.

## §1 Bulgu

### §1.1 İkiz sınaması neyi göremez

[ADR 9224](9224-visual-conformance-edits-and-details.md)'ün ikiz sınaması, bir düzenlemenin resmini aynı programın
baştan çizdiği resimle karşılaştırır. İkisi aynı kodla çizildiği için programın çizdiği resmin kendisinin değişmesini
göremez. Örnekleri:

- eğrinin yeniden kirişe dönmesi;
- çizgi kalınlığı;
- kaybolan bir tarama;
- yazının yüzü.

### §1.2 Izgara sonsuz döngüye giriyordu

İlk altın görüntü çekiminde sözleşme örneğinin çizimi (`fixtures/kcad/v2/drawing.json`) programı düşürdü:
`scene.rs:312`'de “attempt to add with overflow”.

- **Neden:** örnekte uç değerleri sınayan bir çoklu çizgi var. Tepe noktaları en büyük float'a (±1,8·10³⁰⁸) ve
  4,5·10¹⁵'e kadar gidiyor.
- **Tümünü göster** bu nesneyi saydı. Görünümün kutusu sonsuz oldu, ızgara da çizgi sayısını `u32` ile sınırsız saydı.
- **Hata ayıklama derlemesinde** bu taşma paniği. **Yayın derlemesinde** sayaç sarar, döngü hiç bitmez ve her turda
  bir çizgi ekler: program donar, bellek dolar.
- **Gerçek pencerede yeniden üretildi** (main'in ucu, `drawing.kcad`): açılış penceresi “Sonra” ile kapatılıp Home'a
  basılınca program çöktü.

### §1.3 Uzak nesne kuralı uzanan nesneyi görmüyordu

[ADR 9203](9203-show-everything-without-far-objects.md) Tümünü göster'in kutusuna, çizimin çekirdeğinden çok uzakta
duran nesneleri almıyordu. Kural, nesnenin kutusunun **bütünüyle** erişimin ötesinde olmasını istiyordu.

Ama çekirdekten çok uzağa **uzanan** bir nesne de kutuyu büyütür ve çizimi bir noktaya indirir: bir ucu 0,0'a düşmüş
çizgi, bir tepe noktası uç değerde çoklu çizgi. Bunların kutusu çekirdeği de içine aldığı için kural onları
saymıyordu.

## §2 Karar

### §2.1 Altın görüntüler (masaüstü, GPU)

`render_golden_tests.rs` deponun küçük çizimlerini uygulamanın kendi penceresinde, sabit kameralarda çizer. Resim
yalnız çizim alanıdır (kenarlarından ikişer piksel içeride). Pencere 1 440 × 900, uygulamanın kendi ayarları geçerli:
koyu tema, ızgara, çizgi kalınlıkları. Resim `fixtures/render/golden/` altındaki altın görüntüyle karşılaştırılır.

**Eşleşme:** bir piksel, altın görüntüde bir piksel yakınında her kanalı en çok 40 ayrı bir renk varsa eşleşmiş
sayılır. En çok **16 eşsiz piksele** izin var.

- Altın görüntüler sahibin Intel UHD 770'inde çekildi; orada fark 0.
- Mesa'nın yazılım Vulkan'ında (lavapipe) en çok 4 eşsiz piksel. Fark, kenar yumuşatmasından.
- RM-09'un yuvarlaması kapatılıp eğriler 72 kirişe döndürülünce iki resim kırmızı: 21 ve 38 eşsiz piksel.

**15 resim:**

| Ad | Fikstür | Ne gösterir |
|---|---|---|
| acikliklar | openings | duvar, kapı, pencere, sahibi gitmiş açıklık |
| acikliklar-derin | openings | kapının sövesi, kanadı ve yayı, derin görünümde (520 px/m) |
| duvarlar | walls | taramalı duvar, ince duvar, daire duvar |
| mahaller | spaces | mahal etiketleri, kayıp mahal işareti |
| taramalar | hatches | taramalar, degrade dolgular |
| cizgi-tipleri | look | kesikli daire, çizgi tipleri |
| cizgi-kalinliklari | line-weights | kalınlıklar, daire |
| yazilar | texts | hizalar, döndürme, sıkıştırma, genişletme ve zemin |
| notlar | annotation | çağrılar, ölçüler |
| cizim | drawing | sözleşme örneğinin her türü; uç değerli çoklu çizgi dışarıda (§2.3) |
| olculer | dimensions | ölçüler |
| egriler | nurbs | NURBS ve spline |
| maskeler | wipeouts | maskeler, etiket |
| bloklar | blocks | blok örnekleri |
| merdiven-yaylari | stair-arcs | döner merdivenler |

**Yazmak:** çizimi bilerek değiştiren iş `KENTOS_GOLDEN_WRITE=1` ile altın görüntüleri yeniden yazar ve ADR'sinde
önce/sonra resimleriyle söyler. Kırmızı bir durum resmini, altın görüntüyü ve farkı
`.run/gui-evidence/render/rm16-altin/`'e yazar.

### §2.2 Izgara sayabildiği kadar çizer

`build_grid` ve `build_grid_framed` çizgileri artık sayarak çizer (`grid_steps`). Şu durumlarda ızgara boş döner:

- görünümün kutusu, aralık ya da çerçeve sonlu değilse;
- çizgi sayısı 20 000'i geçerse;
- aralık sayıların kesinliğinin altındaysa (x0 + i·aralık kıpırdamıyorsa).

Döngünün sınırı, döngüden önce bilinir.

### §2.3 Çizimden çok uzağa uzanan nesne de uzaktır

Kapsam denetiminin kuralı (`Store::extent_outliers`, `extent_outliers_of`) artık şu nesneleri uzak sayar:

- kutusu erişimin ötesine **uzanan** (bir kenarı çekirdeğin 10 katı ötesinde) nesneyi;
- kutusu sonlu olmayan nesneyi.

Uzak nesneler kutuya girmez, sayısı söylenir, Kapsam denetimi onları seçer. Bu, ADR 9203'ün kuralının
genişlemesidir.

- **İleti aynı kaldı:** “1 nesne çizimin geri kalanından çok uzakta; görünüme alınmadı. Kapsam denetimi onları seçer.”
- **Bilinen bedeli:** küçük bir kasabanın çiziminde onun 10 katından uzun bir yol ekseni de artık Tümünü göster'e
  girmez ve söylenir. Bir noktaya inen çizimden iyidir; uzak nesnenin kendisi Kapsam denetimiyle bulunur.

## §3 Kanıt

**Sınamalar:**

- `kentos-desktop` `render_golden_tests::the_drawings_draw_as_their_golden_pictures`. 15 resim: Intel'de 0 fark,
  lavapipe'ta en çok 4 eşsiz piksel. Eğriler kirişe döndürülünce kırmızı.
- `kentos-render-wgpu` `scene::a_grid_over_a_box_it_cannot_count_is_empty`:
  - sonsuz, NaN ve 2·10³⁰⁰ genişliğinde kutularda düz ve döndürülmüş ızgara boş;
  - 4,5·10¹⁵'te 0,1'lik aralık boş;
  - 19 990 çizgi çizilir, 20 010 çizilmez.

  Renderer'ın bütün sınamaları GPU'yla geçti.
- `kentos-geometry-core` `store::select::tests::far_objects_are_found_and_the_drawing_itself_is_not`:
  - ucu 0,0'a düşen çizgi ve en büyük float'a uzanan çoklu çizgi uzak;
  - kümenin içinden geçen çizgi uzak değil.

  Çekirdeğin bütün sınamaları geçti (785 + 373 + …).
- **Masaüstü:** gezinme, kapsam, seçim, sorgu ve uygunluk sınamalarından 55; `kentos-scene-query` 17.

**Gerçek pencere** (`.run/gui-evidence/render/rm16/izgara/`, `drawing.kcad`, “Sonra”, Home):

- **Önce:** program `scene.rs:312`'de taşma paniğiyle kapandı (`o/app.log`).
- **Sonra:** çizimin tamamı görünüyor, durum çubuğunda “1 nesne çizimin geri kalanından çok uzakta” (`s/c2-tumu.png`).

## §4 Bu işte görülüp sonraya kalan

Sahibin ayarlarıyla (Saydamlık kapalı) aynı katmanda, alandan **sonra** yazılmış yazı alanın dolgusunun **altında**
kalıyor (`drawing.kcad`, “Ada 101 / Parsel 7”). Main'in ucunda da böyle. Çizim sırası işi RM-15'in ilk maddesi
oldu.

## Uygulama

- `crates/render/wgpu/src/scene.rs`: `MOST_GRID_LINES`, `grid_steps`, `build_grid`, `build_grid_framed`.
- `crates/render/wgpu/tests/scene.rs`: sınama.
- `crates/shared/geometry-core/src/store/select.rs`: `far_from_rest`, belge, sınama.
- `apps/desktop/src/shell/render_testing.rs`: `compare_in`, `crop`.
- `apps/desktop/src/tests/render_golden_tests.rs`: altın görüntüler.
- `fixtures/render/golden/*.png`: 15 resim, 740 KB.
- [ADR 9203](9203-show-everything-without-far-objects.md): genişleme notu.
