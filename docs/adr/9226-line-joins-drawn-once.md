# ADR 9226: Çizginin köşeleri bir kez çizilir: yarı saydam çizgide düğüm yok

- **Durum:** kabul edildi (2026-11-25). Plan: [plan/render.md](../../plan/render.md) RM-15, 1. adım.
- **İstek:**
  - planın RM-15 maddesi: “Çizgi kalınlığı ekranda çizilir: en az 1 piksel, kenar yumuşatmalı”;
  - sahibin “kaliteden ödün verilmez” sözü ([ADR 9222](9222-quality-is-not-traded-for-speed.md));
  - 25 Kasım: “Devam et, asla durma, bitene kadar tüm maddeler.”
- **Sözleşmeler:** stilli çizimin çizgi segmentinin son sayısı (`mt.y`) genişledi. Uç bitleri (1 yolun başı, 2 sonu)
  aynı. Üstüne iki köşenin kodu eklendi (`styled.layout.json`). Biçim ve adım (6 sayı) aynı. Yalnız `ends & 1` ve
  `ends & 2`'yi okuyan bir çizici eskisi gibi çizer (web'in WebGL2 gölgelendiricisi böyle).

## §1 Bulgu

### §1.1 Planın iki maddesi zaten yapılmış

Stilli yolun gölgelendiricisi (`stroke.wgsl`) planın istediklerinin ikisini zaten yapıyor:

- **Ekranda en az 1 piksel:** `max(genişlik, 1)` ([ADR 0793](0793-model-pen-widths-and-copied-links.md) §1).
- **Bir piksellik kenar yumuşatma.**
- **Çizgi tipi gölgelendiricide**, yol boyu birikimli evreyle ([ADR 1122](1122-dash-phase-along-the-source-curve.md)).
- **Taramalar gölgelendiricide** ([ADR 9208](9208-svg-symbols-as-vectors.md)'e göre denetlendi).

### §1.2 Asıl kusur: köşeler iki kez çiziliyordu

Çoklu çizginin her kenarı ayrı bir örnek. Her örnek iki ucunda yuvarlak uç çiziyor. Köşede iki kenarın yuvarlak uçları
üst üste biniyordu:

- **Opak çizgide:** yalnız kenar yumuşatması iki kez karışıyordu, köşenin kenarı biraz koyulaşıyordu.
- **Yarı saydam çizgide:** her köşe bir düğüm oluyordu.
  - %50 saydam kırmızı çoklu çizginin gövdesi (125, 49, 55), köşesi (180, 63, 69).
  - Kısa kirişli bir daire, kirişlerinin uçları baştan sona üst üste bindiği için neredeyse opak ve boncuklu
    görünüyordu.

**Kimi etkiliyor:**

- **Saydamlık** varsayılan olarak açık. Nesnenin saydamlığı, sembol katmanının opaklığı ve Tadilat görünümünün soluk
  nesneleri (`PHASE_FADED`) bu yoldan çiziliyor.
- **Opak tarafta** ADR 9222 §2.1 aynı etkiyi ölçmüştü: kısa kirişli daireler “kalın ve parlak”.
- **Sahibin ayarında** Saydamlık kapalı; o yalnız opak taraftaki küçük farkı görüyordu.

## §2 Karar

**Köşe, açıortayıyla bölünür.** Her segment yalnız kendi tarafını çizer: önceki segment başlangıç açıortayının
gerisini, sonraki segment bitiş açıortayının ötesini. Sonuç iki kapsülün birleşimidir, üst üste binme yoktur.

**Kanıt, kısaca:** açıortayın bir yanındaki her noktanın o yandaki segmente uzaklığı öbürününkinden küçük ya da eşit.
Bu yüzden her pikselin kaplaması, o yanın segmentinin kendi kaplamasıdır. Hesap için
`p = a(d+d′) + b(d−d′)`: o yanda `a ≥ 0`, gerisinde `b < 0` ve `|a+b| ≤ |a−b|`.

### §2.1 Kodlama

Segmentin son sayısı:

> `uçlar + 4 × başlangıç kodu + 8 192 × bitiş kodu`

- 24 bit; bir float32 bunu tam tutar.
- **Kod:** 1024 + dönüş (yarım turun 1024'te biri, saat yönünün tersi artı).
- 90° ve 45° tam adımdır.
- 0 “kod yok” demek (yolun ucu ya da kendi üstüne dönen 180°'lik köşe). Kod yoksa eski çizim sürer.

**Ortak çekirdek** (`style-core` `batch.rs`) kodları yazar:

- `turn_code` ve `joins` çizilen segmentlerden hesaplar; dejenere kenarları atlar.
- Kapalı yolun ilk ve son segmenti de birbirine bağlanır.

### §2.2 Gölgelendirici

`stroke.wgsl` kodları çözer, köşenin iki yanının normalini segmentin kendi eksenlerinde parçacık aşamasına verir.
Parçacık kendi tarafında olmayan pikseli atar.

- **Ortak şerit:** açıortayın 0,01 piksellik şeridini iki segment de çizer. Kayan noktanın ayrı yollarından gelen
  farkla hiçbir piksel dışarıda kalmaz.
- **Kısa segment bölünmez:** iki açıortayı bant içinde kesişecek kadar kısa segment (boy ≤ bant × (|tan(θ₁/2)| +
  |tan(θ₂/2)|) + 1) bölünmez, eskisi gibi bütün ucunu çizer. Böyle bir segmentin komşusu bölse bile en kötü sonuç
  eskisi gibi üst üste binmedir, delik değil. Uzak görünümde birkaç piksellik kirişler bu yüzden yine üst üste biner;
  onlar için eğri düzeyleri ([ADR 1063](1063-instanced-blocks-and-curve-detail-levels.md) §3) yerinde kalır.
- **Örnekli çizgiler (blok tanımları):** aynalı bir yerleştirmede (doğrusal parçanın determinantı eksi) kodlar ters
  çevrilir (`mirroredJoins`).
- **CPU ikizi** (`kentos-native-style` `cpu.rs`) aynısını yapar. Resminin y'si dünyanınkine ters olduğunda dönüş
  ters okunur.

### §2.3 Dondurulmuş yanıtlar

Ortak çekirdeğin iki dondurulmuş dosyası web'in kaydıdır; web durdu (ADR 0228). İkisi Rust sınamalarının yazma
kipiyle yeniden yazıldı, yalnız değişen sayılar, dosyanın geri kalanı bayt bayt aynı:

| Dosya | Yazma kipi | Değişen |
|---|---|---|
| `fixtures/style/v1/cases.json` | `KENTOS_WRITE_STYLE_CASES=1` | 2 993 sayı |
| `fixtures/style/v1/batches.json` | `KENTOS_WRITE_STYLE_BATCHES=1` | 483 sayı |

Hepsinin segmentin 6. sayısı olduğu ve uç bitlerinin değişmediği denetlendi.

**Altın görüntüler** ([ADR 9225](9225-golden-pictures-and-grid-bound.md)): dördü izni aştı:

| Resim | Eşsiz piksel |
|---|---|
| cizgi-kalinliklari | 25 |
| cizim | 96 |
| egriler | 29 |
| merdiven-yaylari | 31 |

Farklar kirişlerin köşelerindeki kenar pikselleri. 12 ve 7 kat büyütülmüş resimlerde delik yok, kesikli elips ve
merdiven yayları gözle aynı (`.run/gui-evidence/render/rm15/altin/`). Altın görüntüler yeniden yazıldı. Yazılım
Vulkan'ında en çok 4 eşsiz piksel.

## §3 Kanıt

**Sınamalar:**

- `kentos-render-wgpu` `gpu::joins_on_a_real_gpu`. Gerçek GPU'da 12 piksel genişliğinde bir zikzak (126,9° ve 138,5°'lik
  köşeler). Her piksel, kapsüllerin analitik birleşimine göre beklenen kaplamayla karşılaştırılıyor:
  - yarı saydamda (0,5) ve opakta en çok 3 düzey fark;
  - kodsuz (eski) yazımda köşelerde 40'tan fazla.

  Renderer'ın bütün sınamaları GPU'yla geçti.
- **Ortak çekirdek:** `kentos-style-core` `a_pattern_runs_on_along_a_path_past_its_vertices`:
  - açık yolun köşe kodu iki segmentte de 1234 (36,87° sola);
  - kapalı yolda üç köşe, kodlar komşularda aynı, dönüşlerin toplamı tam tur.

  Çekirdeğin bütün sınamaları geçti (85 + 9).
- `kentos-native-style` bütün sınamalar (52 + 52 + 2 + 2).
- **Masaüstü:**
  - altın görüntüler (yeniden yazıldıktan sonra);
  - CAD ve Mimari uygunluk sınamaları (ADR 9224);
  - stilli sahne sınamaları.

**Gerçek pencere** (`.run/gui-evidence/render/rm15/`, Saydamlık açık bir ayar kopyası, açık tema):

- **Çizim:** %50 saydam kırmızı zikzak, mavi dikdörtgen ve yeşil daire, 2 mm çizgi.
- `once-sonra-saydam.png`:
  - önce her köşe koyu bir düğüm, daire neredeyse opak ve boncuklu;
  - sonra hepsi tek renk.

## Uygulama

- `crates/shared/style-core/src/style/batch.rs`: `TURN_STEPS`, `turn_code`, `joins`, `segment_meta`; `push_segments` ve
  çizgi yolu kodları yazar.
- `shaders/wgsl/styled/stroke.wgsl`: `turnOf`, köşelerin normalleri (`joins`), parçacıkta yanların ayrımı.
- `crates/render/wgpu/src/styled/instances.wgsl`: `mirroredJoins`.
- `crates/native/style/src/cpu.rs`: `joins_of`, CPU ikizi.
- `shaders/wgsl/styled.layout.json`: `mt`'nin açıklaması.
- `crates/render/wgpu/tests/gpu/joins.rs`, `crates/render/wgpu/tests/gpu.rs`: GPU sınaması.
- `crates/shared/style-core/src/style/tests.rs`: kodların sınaması.
- `crates/shared/style-core/tests/all/style.rs`, `crates/native/style/tests/batches.rs`: yazma kipleri.
- `fixtures/style/v1/cases.json`, `fixtures/style/v1/batches.json`, `fixtures/render/golden/*.png`: yeniden yazıldı.
