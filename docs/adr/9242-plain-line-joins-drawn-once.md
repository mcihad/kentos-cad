# ADR 9242: Düz çizginin parçaları ve köşeleri bir kez boyanır

- **Durum:** kabul edildi (2026-11-25).
- **Plan:** [plan/render.md](../../plan/render.md) RM-15'in açık maddesi. RM-08'in gerçek pencere doğrulamasında bulundu
  ([ADR 9241](9241-batches-drawn-by-the-cells-a-view-meets.md) §3).
- **İstek:** 24 Kasım: “kaliteden ödün verilmez”.
- **Sözleşmeler:**
  - Paylaşılan `cad2d` düzeni 2 → 3. Sürüm. `SegmentInstance` 36 → 40 bayt; yeni alan `joins` (yer 5, `uint32`).
  - Tarayıcı denetimi (`scripts/wgsl/browser-check.mjs`) düzeni JSON'dan okur ve verilmeyen alana 0 yazar: 0 eskisi
    gibi yuvarlak uç demek. Betik değişmedi. Web uygulaması `cad2d`'yi kullanmıyor.

## §1 Bulgu

**Düz yol uzun kenarı 128 m'lik parçalara böler** (`precision::MAX_SEGMENT_LENGTH`). Böylece yakındaki parçanın iki ucu
da kameraya yakın kalır; en derin yakınlıkta float32 hassasiyeti bunu ister. Düz yolda çizilenler:

- ızgara;
- Mimari altlık planı ([ADR 8101](8101-ifc-underlay-model.md));
- seçim ve üzerinde durma vurguları;
- yapı çizgileri;
- stil kitaplığı olmadan çizim.

**Çizgi gölgelendiricisi her ucu yuvarlak çiziyordu** (`cad2d/line.wgsl`). İki parçanın birleşiminde iki yarım daire üst
üste biniyor; çizgi boyunca ~2 px iki kez boyanıyordu. Bunun sonuçları:

- **Izgara:** çok saydam çiziliyor (ince çizgi %4, kalın çizgi %9), birleşim noktası iki kat koyu.
  - Uzak görünümde parça ekranda ~10 px; ızgara çizgisi boncuk boncuktu: her 10 pikselin 2'si koyu.
  - Yakında birleşimlerin yeri, ızgara önbelleğinin hangi karede yeniden kurulduğuna bağlı. Noktalı, hayalet çizgiler
    belirip kayboluyordu.
- **Altlık planı:** %45 saydam çiziliyor, çizgileri ayrı çiftler olarak geliyor. Her köşesi iki kez boyanıyordu.
- **Tarama noktaları hiç çizilmiyordu.** Desen noktası sıfır uzunlukta bir parça olarak ekleniyor (`segment(p, p)`), ama
  `segment` sıfır uzunluklu parçayı sessizce atıyordu.

## §2 Karar

### §2.1 Örnek birleşimlerini taşır

`SegmentInstance::joins` ADR 9226'nın kodlarını taşır:

- bitler 0–10: başlangıcın kodu;
- bitler 11–21: bitişin kodu;
- kodlama: yarım tur 1024 adım, 1024 dümdüz;
- 0: birleşim yok, uç yuvarlak.

Kodu `layout::join_code` hesaplar. Geriye dönüş (yarım tur) için kod 0'dır.

### §2.2 Her parça açıortayın kendi tarafını çizer

`line_vs` kodlardan açıortayların normallerini kurar, `line_fs` öbür taraftakini atar. Kural stilli çizgininkinin
aynısı (`styled/stroke.wgsl`):

- Önceki parça başlangıç açıortayının gerisini çizer, sonraki parça bitişinkinin ötesini.
- İkisinin birleşimi, çizginin kendisidir.
- İki açıortayını dörtgeninde ayrı tutamayacak kadar kısa bir parça uçlarını eskisi gibi bütün çizer.

**±0,01 px bandı.** Merkezi birleşime 0,01 pikselden yakın olan piksel iki tarafça da çizilir:

- Rasterleştirici dörtgenin köşelerini 1/256 piksele yerleştirir. Bu yüzden iki tarafın birleşime uzaklığı binde birkaç
  piksel ayrışabilir.
- Daha dar bir bant piksel atlatırdı: eksik piksel, çift boyamadan kötüdür.
- Rastgele bir konumda, bir birleşimin tek sütunu %2 olasılıkla iki kez boyanır. Önceden her birleşimin ~2 pikseli
  iki kez boyanıyordu.

### §2.3 Kurucu kodları yazar

- Kesilen ve derin kutuda bölünen parçalar birbirine dümdüz (1024) bağlanır.
- Bir yolun ardışık kenarları, ancak gerçekten buluştukları köşede (bitişin noktası, sonrakinin başlangıcı)
  dönüşlerinin kodunu alır. Kapalı halka ve başına dönen yol başlangıç noktasında da buluşur. Sayı olmayan bir noktayla
  kopan yol birleşmez.
- Buluşmayan uç 0 alır, yani yuvarlak kalır.
- `build_lines` uç uca eklenen çizgileri tek yol olarak çizer (altlık planı).
- Tarama deseninin noktası sıfır uzunlukta bir parça olarak çizilir: çizgi genişliğinde yuvarlak bir nokta
  (`Builder::dot`).

## §3 Kanıt

**GPU** (UHD 770, 1 ve 4 örnek): yarı saydam (%50), 12 px'lik kırmızı çizgi, kapsüllerin analitik birleşimiyle
karşılaştırıldı.

| Durum | En büyük fark |
|---|---|
| Zikzak köşeleri (126,9° ve 138,5°) | 0,6/255 |
| 2 km'lik çizgi, 16 parça; birleşimler piksel merkezinde değil | 0/255 |
| Aynı çizgi, iki birleşim tam piksel merkezinde | Yalnız o iki sütun bir kat (64/255); geri kalanı 0 |
| Eski yol, her uç yuvarlak | 40/255'ten çok |

**CPU sınamaları:**

- 1 km'lik kare halka: parçalar arası 1024, köşeler 1536 (sola çeyrek tur), kapanış köşesi dahil.
- Uç uca çizgiler köşede birleşiyor, araya sıfır uzunluklu çizgi girse de; ayrı çizgi ve sayı olmayan noktayla kopan yol
  0 alıyor.
- Tarama noktaları çiziliyor. Düzeltmesiz kodda 0 nokta çıktı.

**Paketler:**

- render-wgpu'nun bütün sınamaları (GPU dahil) ve `wgsl_contract` (düzen 3) geçti;
- masaüstünde 119 sınama geçti (altın görüntü, uygunluk, ızgara, altlık, vurgu, etiket, derin görünüm);
- Mimari'nin render uygunluk ve altlık sınamaları: 6'sı geçti.

**Gerçek pencere** (Xvfb ve lavapipe, NCZ). Eski, RM-08 sonrası ikili:

| Görüntü | 2/255'ten çok farklı | Izgara pikseli | Önce daha koyu |
|---|---|---|---|
| Açılış | 36 347 | 35 647 | hepsi |
| 8 çentik | 1 948 | 1 623 | hepsi |
| 16 çentik | 264 | 249 | hepsi |
| 20 çentik | 59 | 56 | hepsi |

- Değişen hiçbir piksel zemine dönmedi; hepsi ızgaranın kendi tonunda (228–244). Boşluk açılmadı, yalnız çift boyama
  kalktı.
- Izgara olmayan farklı pikseller, saydam dolguların altından görünen ızgara.

## Uygulama

- `shaders/wgsl/cad2d/line.wgsl`: `line_turn`, açıortaylar, atma.
- `shaders/wgsl/cad2d.layout.json`, `common/frame.wgsl`, `crates/render/wgpu/src/layout.rs`: düzen 3,
  `SegmentInstance::joins`, `join_code`, `joins`.
- `crates/render/wgpu/src/scene.rs`: `Builder::{joined, cut, piece, dot, path}`, `build_lines` zincirleri.
- Sınamalar:
  - `tests/gpu/plain_joins.rs`;
  - `tests/scene.rs`: `a_rings_pieces_and_corners_carry_their_joins`, `plain_lines_that_follow_one_another_meet`,
    `a_hatch_patterns_dots_are_drawn`;
  - `tests/wgsl_contract.rs`.
