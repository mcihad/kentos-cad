# ADR 9244: Örnekli toplu çizimin yerleştirmeleri sıralarını koruyarak ayıklanır

- **Durum:** kabul edildi (2026-11-25).
- **Plan:** [plan/render.md](../../plan/render.md) RM-08'in açık maddesi
  ([ADR 9243](9243-markers-culled-in-their-order.md) “Kalan”).
- **İstek:** 24 Kasım: “kaliteden ödün verilmez”.
- **Sözleşmeler:** değişmedi. Gölgelendiriciler ve sayılar aynı.

## §1 Bulgu

Örnekli bir toplu çizim (blok yerleştirmeleri, vektörden çizilen SVG simgeleri) her yerleştirmede tanımının hepsini
çizer ([ADR 1063](1063-instanced-blocks-and-curve-detail-levels.md) §2). Görünüm toplu çizimin bütün kutusuna bakıyordu:
yerleştirmelerin hepsi her karede çiziliyordu.

NCZ'de simgeler büyüyünce vektörden çiziliyor ([ADR 9208](9208-svg-symbols-as-vectors.md)). Yakın bir karede:

- sayılan 87 343 öğe, karenin bütün işi 147 722;
- bunun 64 890'ı 7 örnekli toplu çizimin üçgenleri (her birinde birkaç yüz yerleştirme).

AVM DWG'de örnekli iş küçük: yakın görünümde 43 814 öğeye karşı 43 972 iş.

## §2 Karar

### §2.1 Yerleştirmelerin gözleri

`cells::placements` örnekli toplu çizimin yerleştirmelerine göz kurar, sıralarını koruyarak (ADR 9243 gibi):

- **Kutu:** yerleştirmenin kutusu, tanımın kutusunun köşelerinin yerleştirmenin doğrusal kısmı ve ötelemesiyle
  taşınmışıdır.
- **Bölüm:** yerleştirmelerin sayılarıdır (`mesh..mesh + count × 8`, toplu çizimin başından). Sayılar yerinde kalır.
- **Göz büyüklüğü:** bir yerleştirme tanımının hepsini çizer. Bu yüzden göz, tanımın 64 öğesi kadar iş tutar: 200
  kenarlı bir simgede gözü başına bir yerleştirme.
- **Eşik:** yerleştirmelerin toplam işi 512 öğenin altındaysa ya da 8'den az yerleştirme varsa toplu çizim bütün
  çizilir.
- **Kapsam:** yalnız çizicinin yerleştirdiği tanımlar: çizgiler ve düz dolgular (`Placed`).

### §2.2 Çizim

Her karede görünen yerleştirmelerin koşuları hesaplanır (`placed_in`, `GpuBatch::placed_shown`; büyüteç kendi
kutusuyla). Tanımın her aralığı her koşu için çizilir: `draw(tanım, a..z)`.

- Koşular artan sırada geldiği için çizim sırası bütün çizimdekiyle aynı.
- Erim çizgilerinkiyle aynı yoldan hesaplanır (`cell_reach`).

## §3 Kanıt

**Birim sınamaları:**

- 5 000 yerleştirme, 4 m'lik dönük kare: görünüme değen her yerleştirme çiziliyor; yakın görünümde 5 000'in onda
  birinden azı.
- 200 kenarlı bir halkanın 120 yerleştirmesi: en az 64 göz, tek bir yerleştirmeye değen görünüm yalnız onu çiziyor.
  4 yerleştirme bütün çiziliyor.

**GPU** (UHD 770, 1 ve 4 örnek): 6 m'lik kare ile köşegeni, yarı saydam, 20 000 dönük yerleştirme, üst üste
biniyorlar.

| Görünüm | Çizilen yerleştirme | Fark |
|---|---|---|
| 4 px/m | 20 000 → 632 | 0 piksel |
| 12 px/m | 20 000 → 325 | 0 piksel |
| Bütün çizim | 20 000 → 20 000 | 0 piksel |

**NCZ:**

- Simgelerin vektörden çizildiği yakın karede örnekli iş 64 890 → 21 258 üçgen; karenin işi 147 722 → 104 090.
- Harness'ın yakın kare süreleri ölçüm gürültüsü içinde: bu kare tekerlek dizisinin 16 karesinden biri.

**Gerçek pencere:** NCZ'nin dört görüntüsü öncekiyle birebir aynı.

**Paketler:** native stil ve render-wgpu'nun bütün sınamaları (GPU dahil) geçti. Masaüstünde 247 sınama geçti
(blok, simge ve işaret süzgeçleri dahil).

## Uygulama

- `crates/native/style/src/cells.rs`: `placements`, `grid` (göz başına iş), `LEAST_PLACEMENTS`.
- `crates/render/wgpu/src/styled/gpu.rs`:
  - `cell_reach`, `placed_in`, `GpuBatch::placed_shown`;
  - yerleştirme koşularıyla çizim;
  - `DRAWN_INSTANCES` yerleştirmeleri de sayıyor.
- Sınamalar:
  - `cells.rs`: `an_instanced_batch_keeps_its_placements_order_and_a_view_draws_those_it_meets`,
    `a_heavy_definitions_few_placements_have_cells`;
  - `tests/gpu/cells.rs`: `placements_keep_their_order`.
