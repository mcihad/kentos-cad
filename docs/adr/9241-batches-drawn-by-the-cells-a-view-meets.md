# ADR 9241: Toplu çizim, görünümün değdiği gözlerden çizilir

- **Durum:** kabul edildi (2026-11-25). Plan: [plan/render.md](../../plan/render.md) RM-08 (dünya döşemeleri ve görünüm
  ayıklaması).
- **İstek:**
  - planın RM-08 maddesi: “Dünya döşemeleri, ayıklama ve çizim sırasının korunması”;
  - kabul ölçütü: “Kamera değişikliğinden onu gösteren kareye ≤ 16 ms”;
  - 24 Kasım: “kaliteden ödün verilmez”.
- **Sözleşmeler:** değişmedi.
  - Stil çekirdeği ve paylaşılan WGSL aynı.
  - Kâğıt ve PDF yolları aynı: gözler yalnız model ekranında (`hold_weights`) kurulur.

## §1 Bulgu

**Toplu çizim, katmanın bir stilini bütün katman boyunca toplar.** Bir imar planının parsel çizgileri yüz binlerce
parçalık birkaç toplu çizimdir. Görünüm yalnız toplu çizimin bütün kutusuna bakıyordu (`StyledBatch::in_view`).

`KENTOS_DIAG_ITEMS` ile NCZ (UHD 770, sürüm):

| Görünüm | Çizilen öğe |
|---|---|
| Açılış (bütün şehir) | 1 159 737 |
| Sekiz çentik yakında (bir mahalle) | 1 223 608 |

Öğelerin çoğu “eğri d3/d2” toplu çizimlerinde, ama bunlar eğri kirişi değil. Eğri düzeyi olan bir toplu çizimin her
düzeyi, düz parçaların hepsini de taşıyor. 670 bini parsel çizgisi. Yani analitik eğri (RM-09) bu kareleri hızlandırmaz;
ayıklama hızlandırır.

## §2 Karar

### §2.1 Model ekranında büyük toplu çizim gözlere göre sıralanır

`kentos_native_style::cells` çözmenin sonunda (`decode`, `svg_marks`'tan sonra) çalışır. Kapsam:

- 512'den çok öğeli bölümler (`LEAST_ITEMS`);
- çizgi ve dolgu toplu çizimleri (düz, tarama, gradyan, desen ve karo dolgular);
- örneksiz olanlar.

Her bölüm ayrı sıralanır. Bölüm, ana sayılar ve her ayrıntı düzeyinin sayılarıdır (`Lod::ranges`); düzey yoksa bütün
sayılar.

- Bölümün üstüne bir ızgara kurulur: göz başına ortalama 64 öğe (`ITEMS_A_CELL`), en çok 64 × 64 göz.
- Öğeler orta noktalarının gözüne göre sayarak sıralanır (counting sort). Sıralama kararlı: bir gözün içinde öğeler
  eski sırasını korur.
- Düşük akış (`low`) sayılarıyla birlikte taşınır.
- Her göz, öğelerinin başladığı yeri ve kapladığı kutuyu tutar. Bir öğenin kendi gözünden en çok ne kadar taştığı da
  saklanır (`spill`).

**Eşikler neden bu kadar küçük:** büyük bir katman, belgenin sırasıyla parçalar hâlinde kurulur
([ADR 0121](0121-desktop-layer-parts.md)). Her parçanın toplu çizimleri bütün çizime yayılır. NCZ'de bir parçanın
toplu çizimi 8 000 kadar öğe tutuyor, bir kısmı 4 096'nın altında.

- İlk eşikler (4 096 öğe, göz başına 512) bir mahalle görünümünün 1,24 M öğesinin 1,1 M'unu bütün olarak çizdiriyordu.
- 8 000 öğelik bir bölümde göz başına 512 öğe, şehrin dörtte birini kaplayan 16 göz demekti.

**Resim neden değişmez:**

- Bir toplu çizim tek bir stildir. Aynı rengin iki katı hangi sırayla gelirse gelsin aynı pikseli verir:
  `c·a + c·b − c·a·b`, altında `d·(1−a)(1−b)`.
- Yerden okunan boya (tarama, gradyan, desen, resim) da bir piksele tek renk verir.
- **İşaretlere dokunulmaz:** bir resmin renkleri yer yer değişir, üst üste binen iki işaretin hangisinin üstte olduğu
  görünür.

### §2.2 Görünüm değdiği gözleri çizer

Her karede, görünen her toplu çizim için düzeyinin aralıkları, görünüme değen gözlerin aralıklarına bölünür
(`GpuBatch::shown`).

**Gözler kutularıyla sınanır.** Kutu, erim kadar büyütülür:

- toplu çizimin kendi erimi (`reach`: çizginin genişliği ve bir birim);
- çizginin bulanıklığının yarısı;
- iki aygıt pikseli.

Görünümün kenarındaki gözler her karede sınanır. Çizginin dörtgeni genişliğinin ötesine de yayılır: kenar yumuşatma
için bir piksel, en ince çizginin yarım pikseli, bulanıklığın yarısı (`stroke.wgsl`). Uzakta, çizgiye kendi erimi
pikselden azdır. Pay olmasaydı görünümün hemen dışındaki bir gözün çizgisi, görünümün ilk pikselini boyamaz olurdu.

**Çizim:**

- Yan yana aralıklar birleştirilir. Çizim sayısı bölüm başına en çok 8 (`MOST_RANGES`): daha fazlası en yakın
  aralıkların arası da çizilerek birleştirilir.
- Çizim ve istatistik aynı aralıkları kullanır.
- Büyüteç kendi kutusuyla hesaplar.
- Uzak taramaların tek çizimde birleştirilen dizisine dokunulmaz
  ([ADR 1066](1066-million-far-hatches-select-all-and-first-frame.md) §2).

**Her toplu çizim kendi gözünü alır, görünse de görünmese de.** Gözlerin listesi toplu çizimlerle bire bir yürür. İlk
sürüm listeyi yalnız görünen toplu çizimlerde ilerletiyordu:

- toplu çizimler başkasının gözünü alıyordu ya da hiç almıyordu;
- NCZ'de 674 bin öğe bütün olarak çiziliyordu.

GPU sınamasının katmanı bu yüzden önce görünmeyen küçük bir toplu çizimle başlar. Eski kodla sınama kırılıyor.

## §3 Kanıt

**Birim sınamaları** (`cells`, 4 sınama):

- Her öğe bir kez kalıyor; düşük akış sayısıyla taşınıyor.
- Her öğe kendi gözünün kutusunda; göz içinde sıra korunuyor.
- Görünüme değen hiçbir öğe atlanmıyor (üç görünüm).
- Düzeyli toplu çizimin her bölümü ayrı sıralanıyor.
- Küçük toplu çizim ve işaret olduğu gibi kalıyor.

**GPU sınaması** (UHD 770, 1 ve 4 örnekle). Katman:

- 30 000 kesikli, yarı saydam parça;
- 12 000 yarı saydam üçgen;
- önlerinde görünmeyen küçük bir toplu çizim.

| Görünüm | Çizilen öğe | Resim |
|---|---|---|
| Yakın (8 px/m) | 66 000 → 270 | birebir aynı |
| Daha yakın (40 px/m) | 66 000 → 266 | birebir aynı |
| Çizimin bir kısmı (0,5 px/m) | 66 000 → 2 775 | birebir aynı |
| Bütün çizim (0,06 px/m) | 66 000 → 66 000 | 1 000 pikselde (%1,5) en çok 2/255 |

Bütün çizimdeki fark yalnız sıradan geliyor: pikseli onlarca yarı saydam öğe örtüyor ve 8 bitlik hedef her harmanlamayı
yuvarlıyor.

**Görünüm kenarı:** 32 × 32 dik çizgi, 0,1 px/m. Görünümün sol kenarı, ilk göz sütununun son çizgilerinin 3 m (0,3 px)
sağında. Resim birebir aynı. Payı kaldırınca 96 piksel 27 adıma kadar farklı çıktı.

**NCZ, yakın görünüm** (`perf::files`'ın `near` evresi, 8 px/m, hata ayıklama derlemesi, UHD 770, GPU zaman damgaları):

| | Çizilen öğe | Tekerlek karesi GPU p50 | p95 |
|---|---|---|---|
| Gözsüz | 5 510 082 | 74,1 ms | 77,8 ms |
| Gözlü | 264 735 | 18,8 ms | 27,0 ms |

- Uzak tekerlek değişmedi (p50 ~95 ms). Açılış süresi ve en yüksek bellek de değişmedi.
- Kalan yakın öğelerin çoğu işaret: 176 bin, sıralanmıyor (§2.1).

**Gerçek pencere** (Xvfb ve lavapipe). NCZ'nin açılışı ile 8, 16 ve 20 çentik yakını, eski ve yeni ikiliyle:

| Görüntü | Farklı piksel | En büyük fark |
|---|---|---|
| Açılış | 1 576 | 2/255 |
| 8 çentik | 511 | 2/255 |
| 16 çentik | 949 (587'si 2'den çok) | 19/255 |
| 20 çentik | 0 | 0 |

- Yeni ikili iki kez koşunca dört görüntünün dördü de birebir aynı.
- Yeni ikili gözsüz koşunca (`KENTOS_DIAG_NO_CELLS`), dördü de eski ikiliyle birebir aynı.
- **16 çentikteki fark gözlerden değil, ızgaradan geliyor:**
  - Izgara çizgileri 128 m'lik eşit parçalara bölünüyor (`precision::MAX_SEGMENT_LENGTH`). Parçaların birleştiği piksel
    iki kez boyanıyor.
  - Izgara, görünümü kaplamadığında üç görünüm genişliğinde yeniden kuruluyor. Hangi ara karede kurulduğu kare hızına
    bağlı.
  - Eski ikilide birleşimler 322. ve 805. sütunlarda (483 px arayla); yenisinde 80., 537. ve 994. sütunlarda (457 px
    arayla). Fark yalnız bu sütunlarla ızgara çizgilerinin kesiştiği noktalarda.
  - Birleşim pikselinin iki kez boyanması eskiden beri var; ayrı bir madde olarak düzeltilecek (plan, RM-15).

**Paketler:**

- native stil ve render-wgpu'nun bütün sınamaları (GPU dahil) geçti;
- masaüstünün sahne, altın görüntü, uygunluk, etiket, derin görünüm ve görünüm sınamaları: 97'si geçti. Elle koşulan
  17 ekran görüntüsü ve ölçüm sınaması koşulmadı.

## Kalan

- **İşaretler ayıklanmıyor.** Sırayı koruyarak ayıklamak gerekiyor: görünen gözlerin öğe sayıları her karede artan
  sırayla birleştirilir, gölgelendirici örneği bu listeden okur.
- **Yakın görünümde kabul ölçütü henüz tutmuyor:** GPU p50 18,8 ms. Hedef ≤ 16 ms; işaretler ve RM-10 kaldı.

## Uygulama

- `crates/native/style/src/cells.rs` (yeni): `add`, `Cells`, `CellPart::ranges_in`.
- `crates/native/style/src/batches.rs`: `StyledLayer::cells`; çözmenin sonunda `cells::add`.
- `crates/render/wgpu/src/styled/gpu.rs`: `culled` (erim ve pay), `GpuBatch::shown`, çizimde aralıklar, sayımda
  aralıklar.
- `apps/desktop/src/shell/perf/files.rs`: yakın görünüm evresi (`near`).
- Sınamalar: `crates/render/wgpu/tests/gpu/cells.rs`.
