# ADR 9247: Çizginin dörtgeni dört köşeli şerit, payı üç çeyrek piksel

- **Durum:** kabul edildi (2026-11-25).
- **Plan:** [plan/render.md](../../plan/render.md) RM-10, resmi değiştirmeyen hız.
- **İstek:**
  - kabul ölçütü: “Kamera değişikliğinden onu gösteren kareye ≤ 16 ms”;
  - 24 Kasım: “kaliteden ödün verilmez”.
- **Sözleşmeler:**
  - Paylaşılan WGSL değişmedi. Değişiklik masaüstünün kendi kaynağında: `styled/shader.rs`'in yaması ve
    `styled/strip.wgsl`.
  - ADR 9246'nın eşitlik sınaması (`tests/gpu/single.rs`) yuvarlamanın kuyruğunu kabul ediyor (§3).

## §1 Bulgu

[ADR 9246](9246-strokes-drawn-one-sample-a-pixel-and-laid-over.md)'dan sonra uzak karenin çizgileri tek örnekte
çiziliyor. Bedel yine ilkel başına (`tests/gpu/far_cost.rs`, UHD 770, 1 M parça):

| | Kare |
|---|---|
| Tek örnekli koşu | 18,8 ms |
| 1× | 17,9 ms |

İki gereksiz iş vardı:

- **Altı köşe:** her çizgi parçası altı köşeli bir liste olarak çiziliyordu. Dörtgen iki üçgen, ama köşe aşaması altı
  kez çalışıyordu.
- **Bir piksellik pay:** dörtgen, kaplamanın bittiği yerden 1 px öteye uzanıyordu (ADR 9246 §2.1, en uzak örnek için).
  Örnekler pikselin içinde durur; en uzağı piksel ortasından yarım köşegen, 0,71 px.

## §2 Karar

### §2.1 Dört köşeli şerit

Düz çizgiler (`Pipe::Stroke`) `strokeVsStrip` ile çizilir:

- listenin 0, 1, 5, 2 köşeleri şerit sırasında aynı dörtgeni iki üçgenle kurar;
- köşe aşaması parça başına 6 yerine 4 kez çalışır;
- kapsanan pikseller ve aradeğerlenen sayılar aynıdır.

Blokların örnekli çizgileri (`strokeVsInstanced`) ve işaretler liste olarak kalır.

### §2.2 Pay üç çeyrek piksel, her boru hattında aynı

Dörtgen kaplamanın bittiği yerden 0,75 px öteye uzanır (eskiden 1):

- Hiçbir örnek düzeninde bir örnek piksel ortasından 0,71 px'ten uzak değil. Yani 16× dahil her örnek sayısında çizilen
  piksel bütün örneklerini kaplar: ADR 9246 §2.1'in koşulu.
- Birleşim denetimi (`len > ext * reach + 1`) eski `ext`'le kalır.

**Tek örnekte 0,5 yeterdi**, çünkü orada yalnız piksel ortaları sayılır. Ama yalnız tek örnekli boru hatlarını
daraltmak resmi değiştirdi (§3). Köşeler farklı olunca çizginin koordinatları kayan noktada bir kıl farklı çıkıyor.
Çizgi gölgelendiricisinde kaplamanın sıçradığı üç sınır var:

- düz uç (kaplama yarımdan tama);
- kesik ucu (yarımdan sıfıra);
- köşe açıortayı (ADR 9226).

Bir sınır tam piksel ortasından geçince o kıl pikseli öbür yana atıyor. Pay bu yüzden her boru hattında aynı.

## §3 Kanıt

**GPU eşitlik sınaması** (ADR 9246'nınki, UHD 770, 4×, 147 456 piksel): örneklerde çizim ile tek örnekli koşu.

| Durum | 1 adım | 2 adım | 3 adım | 4 adım | 5+ adım |
|---|---|---|---|---|---|
| Uzakça | 96 959 | 6 336 | 64 | 1 | 0 |
| Yakın | 89 776 | 2 588 | 6 | 0 | 1 |
| Uzakça, yalnız tek örnekte 0,5 px pay | 97 075 | 6 302 | 109 | 10 | 45 (en çok 45) |

- **4 ve 5 adımlık birer piksel.** Önce yuvarlamanın kuyruğu sandım: örnekli resim her karıştırmayı 8 bite
  yuvarlıyor ve kesikler kaldırılınca yakın durumdaki piksel aynı yerde 4 adımda kalıyordu.
  - **Düzeltme ([ADR 9249](9249-stroke-ends-smoothed-as-their-sides.md)):** yuvarlama değildi. Sınamanın çizgilerinin
    hepsi düz uçlu. Düz uçta kaplama yarımdan tama sıçrıyordu; köşeler değişince kayan noktanın bir kılı pikseli
    öbür yana attı.
  - Uçlar yumuşayınca iki piksel de kayboldu; en çok fark yeniden 3 adım.
- **Sınamanın eşiği:** bu ADR'de en çok 8 adıma gevşetilmişti. ADR 9249'la yeniden en çok 3 adım.
  - 3 adım binde birden az;
  - 2 adım onda birden az.

  Kaybolan ya da kayan bir çizgi pikseli bu eşiği geçer: 0,5'lik deneme 55 pikselde geçti.

**Ölçüm sınaması** (`far_cost`, release, UHD 770, 1 M parça):

| | ADR 9246 | Yalnız şerit | Şerit ve 0,75 pay |
|---|---|---|---|
| 4×, örneklerde | 50,9 ms | | 50,5 ms |
| Tek örnekli koşu, düz | 18,8 ms | 17,1 ms | 15,4 ms (−%18) |
| Tek örnekli koşu, kesikli | 20,3 ms | 19,1 ms | 17,0 ms (−%16) |
| 1× | 17,9 ms | 16,4 ms | 14,7 ms (−%18) |

0,5 pay 0,75'ten fazla kazandırmadı (1× 14,6 ms).

**Dosyalar** (hata ayıklama derlemesi, `perf::files`, UHD 770; harness'ın tabanı ~12,7 ms). Uzak tekerlek karesi:

| | 4× (öntanımlı) | 1× (sahibin ayarı) |
|---|---|---|
| NCZ p50 | 65,0 → 60,4 ms | 61,8 → 56,9 ms |
| NCZ p95 | 77,1 → 71,6 ms | 70,2 → 66,1 ms |
| sivas.kcad p50 | 80,7 → 76,4 ms | 63,4 → 59,5 ms |
| sivas.kcad p95 | 89,0 → 84,4 ms | 72,9 → 67,2 ms |

- Kaydırma ve yakın görünüm değişmedi.
- sivas.kcad ve NCZ'nin uzak karesi ölçütün hâlâ üstünde.

**Gerçek pencere** (Xvfb ve lavapipe, NCZ; işlenmiş ikiliye karşı):

| Görüntü | 1× | 4× |
|---|---|---|
| Açılış | 1 009 px, en çok 4 adım | 1 033 px, en çok 4 |
| 8 çentik | 3 161 px; 2'si 5 adımın üstünde, en çok 14 | 4 429 px; 3'ü 5 adımın üstünde, en çok 19 |
| 16 çentik | 654 px; 1'i 5 adımın üstünde, en çok 12 | 728 px, en çok 3 |
| 20 çentik | 186 px, en çok 2 | 178 px, en çok 2 |

Her görüntüde farkların en az %96'sı 1 adım. 5 adımı geçen altı piksel 14 kat büyütülerek incelendi: hepsi ince çizgilerin keskin
sınırlarında. Kaybolan ya da eklenen çizgi yok.

**Paketler:** render-wgpu'nun bütün sınamaları (GPU dahil), masaüstünde 320 sınama (birleştirme, maske, efekt,
kararlılık dahil) ve Mimari'nin render uygunluk sınaması geçti.

## §4 ADR 9246'nın gerçek pencere kanıtı düzeltildi

Sahibin ayarında `graphics.msaa` 1, öntanımlı 4. Gerçek pencere oturumu sahibin ayarlarının bir kopyasıyla açılıyor,
yani 1×'te çiziyor. ADR 9246 1× hedefte koşu çizmiyor: orada kazanılacak bir şey yok.

- ADR 9246'nın gerçek pencere tablosu, bu denetim eklenmeden önceki bir ara ikiliyle alınmıştı. O ikili 1× hedefte de
  koşu çiziyordu.
- İşlenmiş ikiliyle 1×'te resim koşulu da koşusuz da birebir aynı (0 piksel).
- Aynı karşılaştırma 4× ayarlı bir kopyayla, bu ADR'nin koduyla:
  - açılış 64 524 piksel (61 118 / 3 253 / 153 piksel 1 / 2 / 3–4 adım);
  - 8 çentik 161 694 piksel (154 552 / 7 044 / 96; 2'si 5 adımın üstünde);
  - 16 ve 20 çentik 0.

  Eski tablonun sayılarıyla aynı büyüklükte.
- ADR 9246'nın release ölçümleri doğru: `perf::files` öntanımlı ayarla, yani 4×'te ölçüyor. Sahibin 1× karesi bu
  ADR'nin tablosunda.

## Bilinen: düz uç ve kesik ucunun yarım basamağı

Çizginin yanları kenarda bir piksellik simetrik rampayla yumuşar: 0,5 px dışarıda 0, 0,5 px içeride 1. Uçlarda rampa
yalnız bir yanda:

- **Düz uç:** dışarıda yarım rampa var, kenarda 0,5'ten 1'e sıçrıyor. Uç sekizde bir piksel uzun görünür.
- **Kesik ucu:** içeride yarım rampa var, kenarda 0,5'ten 0'a iniyor. Kesik her ucunda sekizde bir piksel kısa
  görünür.

Paylaşılan WGSL'de böyle; web ve CPU ikizi de aynı. Düzeltmek resmi değiştirir ve kendi maddesi var (RM-15).

## Uygulama

- `crates/render/wgpu/src/styled/strip.wgsl`: `strokeVsStrip`.
- `crates/render/wgpu/src/styled/shader.rs`: dörtgenin payı (`let quad = halfW + 0.5 * blur + 0.75;`).
- `crates/render/wgpu/src/styled/gpu.rs`: `pipe_as` (şerit), `corners`.
- Sınamalar:
  - `shader.rs`: `the_stroke_quad_reaches_three_quarters_of_a_pixel_past_its_smoothing`;
  - `tests/gpu/single.rs`: eşik.
