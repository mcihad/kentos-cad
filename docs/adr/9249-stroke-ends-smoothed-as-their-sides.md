# ADR 9249: Çizginin uçları yanları gibi yumuşar

- **Durum:** kabul edildi (2026-11-25).
- **Plan:** [plan/render.md](../../plan/render.md) RM-15'in açık maddesi: uçların yarım basamağı
  ([ADR 9247](9247-stroke-quads-as-strips-with-a-narrower-margin.md)'de bulundu).
- **İstek:** 24 Kasım, “kaliteden ödün verilmez”.
- **Sözleşmeler:**
  - Paylaşılan `stroke.wgsl` değişti. Web uygulaması iptal edildi (TODOS.md §0.2 kapandı), web'e erteleme yazılmaz.
  - Stil çekirdeğinin sayıları ve düzeni değişmedi.
  - Taramaların kesikleri (`dashCover`) eskisi gibi.

## §1 Bulgu

Çizginin yanları bir piksellik simetrik bir rampayla yumuşar: kenarın 0,5 px dışında 0, 0,5 px içinde 1. Bu, pikselin
aralığının çizgide kalan payıdır. Uçlar ise tek yanlıydı:

- **Düz uç:** rampa yalnız dışarıdaydı; uçta kaplama 0,5'ten 1'e sıçrıyordu. Uç sekizde bir piksel uzun görünüyordu.
- **Kesik ucu:** rampa yalnız içerideydi; kenarda kaplama 0,5'ten 0'a iniyordu. Kesik her ucunda sekizde bir piksel
  kısa, boşluk o kadar uzun görünüyordu.

Sıçramanın iki sonucu vardı:

- **Kenar piksel ortasına düşünce** kayan noktanın bir kılı pikseli öbür yana atıyor. ADR 9247'nin eşitlik sınamasındaki
  4 ve 5 adımlık iki piksel buydu; orada yanlışlıkla yuvarlamaya yazılmıştı.
- **Kısa boşluklar hareket ederken** ya tam karanlık ya tam boş çıkıyor; görünüm kaydıkça boşluklar piksel piksel atlıyor.

## §2 Karar

Uçlar ve kesikler pikselin aralığıyla kesiştirilir, yanlar gibi.

- **Düz uç:** yolun ucundaki parçanın kaplaması ucun rampasıyla çarpılır:
  - başta `cover(x)`, sonda `cover(len - x)`;
  - iki uç birden düzse `cover(x) + cover(len - x) - 1`: kısa bir parçada da pikselin payı.

  Yuvarlak ve kare başlık ile köşeler değişmedi.
- **Kesik** (`strokeDash`): her kesiğin [0, l] aralığı pikselin [t − ½, t + ½] aralığıyla kesiştirilir. Komşu
  dönemlerin kesikleri de sayılır.
  - Pikselden kısa kesiğin “bir piksel boyar” kuralı ([ADR 0186](0186-hatch-extras.md) §3) aynı.
  - Sıfır boylu noktalar ([ADR 1124](1124-line-type-dots-print-round.md)) aynı.
- **CPU ikizi** (`cpu.rs`) aynısını yapar.
- **Taramalar** (`dashCover`) ilk adımda kendi kurallarında kaldı; aynı akşam onlar da aynı paya geçti (§4).

## §3 Kanıt

**GPU sınaması** (`tests/gpu/ends.rs`, UHD 770):

- Çizim: 5 birim kalın, iki ucu düz bir çizgi; uçları ve kesik kenarları piksel ortalarından uzakta. Bir kez düz, bir
  kez 7,3 / 4,6 kesikli.
- Sonuç: orta satırın her pikseli, pikselin aralığının payına ±2/255 içinde, GPU'da ve CPU ikizinde.
- Eski kodda sınama kırılıyor: düz ucun içteki pikseli 255, olması gereken pay 178,5.

**Eşitlik sınaması** ([ADR 9246](9246-strokes-drawn-one-sample-a-pixel-and-laid-over.md)): en çok fark yeniden 3 adım;
4 ve 5 adımlık pikseller kayboldu.

| Durum | 1 adım | 2 adım | 3 adım |
|---|---|---|---|
| Uzakça | 97 629 | 6 516 | 76 |
| Yakın | 91 870 | 2 983 | 9 |

**Gerçek pencere** (NCZ, sahibin ayarı 1×, ADR 9247'li ikiliye karşı):

| Görüntü | Farklı piksel | 64 adımdan fazla | En çok |
|---|---|---|---|
| Açılış | 15 321 | 119 | 141 |
| 8 çentik | 36 453 | 873 | 148 |
| 16 çentik | 2 760 | 435 | 165 |
| 20 çentik | 196 | 20 | 122 |

Kaplama yalnız uçların ve kesik kenarlarının yarım pikselinde değişti, farklar da orada. Büyütülmüş fark haritaları
tek tek uç piksellerini gösteriyor.

**Altın görüntüler:**

- `cizim` izni aştı: kesikli eğride 50 pikselin yakınında eşi yok.
- Öbür 14'ü izin içindeydi.
- 15'i de yeniden yazıldı (`KENTOS_GOLDEN_WRITE=1`).

**Paketler:** render-wgpu'nun bütün sınamaları (GPU dahil), stil çekirdeği, masaüstünde 320 render sınaması ve
Mimari'nin render uygunluk sınaması geçti.

## §4 Taramaların kesikleri (aynı akşam)

Taramanın kesikli çizgileri (`dashCover`: kısa desenler paylaşılan `common.wgsl`'de, uzunlar yerel `dashes.wgsl`'de)
aynı yarım basamağı taşıyordu. Onlar da pikselin aralığının payına geçti. Pikselden kısa kesiğin ve sıfır boylu
kesiğin bir piksel boyama kuralı taramada aynı (ADR 0186 §3). CPU ikizinde iki kip tek işlevde (`dash_spans`).

- `tests/gpu/ends.rs`'e kesikli bir tarama çizgisi eklendi: GPU ve CPU paya ±2/255 içinde. Eski kodda kırılıyor:
  boşluktaki bir piksel 0, olması gereken pay 51.
- Altın görüntülerden yalnız `taramalar` değişti (261 piksel, 8'inin yakınında eşi yok; izin 16) ve yeniden yazıldı.
- render-wgpu'nun bütün sınamaları, stil çekirdeği, masaüstünde 324 render sınaması ve Mimari'nin uygunluk sınaması
  geçti.

## Uygulama

- `shaders/wgsl/styled/stroke.wgsl`: `strokeFs`'in uçları.
- `shaders/wgsl/styled/common.wgsl`: `dashSpan`, `dashCover` (kısa desenler).
- `crates/render/wgpu/src/styled/dashes.wgsl`: `strokeDash`, `dashCover` (uzun desenler).
- `crates/native/style/src/cpu.rs`: `strokes`, `dash_spans`.
- `fixtures/render/golden/*.png`.
- Sınamalar:
  - `tests/gpu/ends.rs`;
  - `tests/gpu/single.rs`: eşik yeniden en çok 3 adım.
