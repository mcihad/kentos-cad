# ADR 9243: İşaretler sıralarını koruyarak ayıklanır

- **Durum:** kabul edildi (2026-11-25).
- **Plan:** [plan/render.md](../../plan/render.md) RM-08'in açık maddesi
  ([ADR 9241](9241-batches-drawn-by-the-cells-a-view-meets.md) “Kalan”).
- **İstek:**
  - kabul ölçütü: “Kamera değişikliğinden onu gösteren kareye ≤ 16 ms”;
  - 24 Kasım: “kaliteden ödün verilmez”.
- **Sözleşmeler:** değişmedi.
  - Gölgelendiriciler aynı.
  - İşaretlerin sayıları yerinde kalır.

## §1 Bulgu

ADR 9241'den sonra NCZ'nin yakın görünümü hâlâ 264 735 öğe çiziyordu. 176 bini işaretti: işaretler gözlere göre
sıralanmıyor, çünkü üst üste binen iki işaretin hangisinin üstte olduğu görünür.

Bir sonda ile toplu çizimlerin sırasına bakıldı (commit edilmedi):

- 42 toplu çizimde 222 664 işaret var.
- Belge sırasında art arda gelen işaretler, 64 × 64 bir ızgarada ortalama 4,6'lık koşularla aynı gözde kalıyor.
  Rastgele bir sırada bu 1 olurdu.
- Şehrin ortasında, genişliğin ellide biri kadar bir pencerede 886 işaret görünüyor. Bunlar 170 koşuya dağılıyor ve
  koşuların kapsadığı sıra 13 400 öğe, 222 664'ün yerine.

Yani çizimin sırası uzamsal olarak kümeli. Görünen gözlerin öğeleri, toplu çizimin kendi sırasında az sayıda koşu
olarak çizilebilir.

## §2 Karar

### §2.1 İşaretin gözleri öğelerini sırasıyla tutar

`cells::add` 512'den çok işaretli bölümlere de göz kurar, ama sayılarını yerinden oynatmaz:

- Her göz, öğelerinin sıra numaralarını kendi sıralarıyla tutar (`CellPart::order`).
- Bir işaretin kutusu yeridir. Simgenin boyu toplu çizimin eriminde (`reach`).

### §2.2 Görünüm, gözlerinin öğelerini koşu olarak çizer

Her karede görünen gözlerin öğeleri, bölümün üstünde bir bit dizisine işaretlenir (`runs`). Dizi baştan sona taranır
ve toplu çizimin sırasında koşular çıkar.

- Görünen gözler bölümün yarısından çoğunu tutuyorsa bölüm bütün çizilir.
- 16'dan az öğe arayla gelen koşular birleşir (`JOIN_GAP`). Aradaki birkaç öğenin GPU'ya maliyeti, bir çizim çağrısının
  CPU'ya maliyetinden azdır. Bu kural çizgi ve dolgu bölümlerinin aralıklarına da uygulanır.
- İşaret bölümü en çok 32 çizim alır (`MOST_RUNS`). Fazlası en küçük boşluklarından birleştirilir; bu boşluklar tek
  geçişte boylarına göre seçilir (`select_nth_unstable`). Eskiden birleştirme her adımda en küçük boşluğu yeniden
  arıyordu.

**Resim neden değişmez:**

- Her çizim toplu çizimin sırasında bir koşudur ve çizimler artan sırayla gelir.
- Birleştirilen koşuların arasındaki öğeler de çizilir; bunlar görünümün dışındadır.

**Rastgele sıralı bir toplu çizim** için koşular bütün bölüme yayılır ve 32'ye inerken birleşir. Görünen her öğe
yine çizilir; bütünden fazlası çizilmez, ama kazanç azalır (sınamada 29 görünen işaret için 20 000'in 7 395'i).

Gölgelendiricinin örneği bir dizin listesinden okuması bu durumu da çözerdi. Gerçek çizimlerin sırası kümeli olduğu
için şimdilik yapılmadı (§3).

## §3 Kanıt

**Birim sınamaları** (20 000 işaret, 10 km):

| Sıra | Görünen | Çizilen | Koşu |
|---|---|---|---|
| Blok blok (500 m), çizimdeki gibi | 29 | 283 | 2 |
| Blok blok | 184 | 440 | 4 |
| Rastgele | 29 | 7 395 | 32 |
| Rastgele | 184 | 12 898 | 32 |

Her durumda görünen her işaret çizildi ve sayılar yerinde kaldı.

**GPU** (UHD 770, 1 ve 4 örnek): 20 000 daire, 6 m. Turuncu dolgu ile mavi çizgi, ikisi de yarı saydam. 250 m'lik
bloklar hâlinde, üst üste biniyorlar.

| Görünüm | Çizilen | Fark |
|---|---|---|
| 4 px/m | 20 000 → 596 | 0 piksel |
| 12 px/m | 20 000 → 632 | 0 piksel |
| Bütün çizim | 20 000 → 20 000 | 0 piksel |

İşaretler gözlere göre sıralanınca 4 px/m'de bile 5 piksel farklı çıktı. Yani sınama sırayı yakalıyor.

**NCZ, yakın görünüm** (`perf::files`, `near` evresi, 8 px/m, hata ayıklama derlemesi, UHD 770):

| | Çizilen öğe | Kare p50 | Kare p95 | Çizimin payı p50 | Çizimin payı p95 |
|---|---|---|---|---|---|
| ADR 9241 | 264 735 | 18,8 ms | 27,0 ms | 6,1 ms | 13,3 ms |
| Şimdi | 87 207 | 17,9 ms | 21,5 ms | 5,2 ms | 7,8 ms |

**Harness'ın kare süresi GPU zaman damgası değil.** `renderer.screenshot`'ın süresi ölçülüyor: bütün pencere ekran
dışında çiziliyor ve 1440 × 900 görüntü CPU'ya okunuyor. Neredeyse boş bir çizimde (`minimal.kcad`) bu taban
12,7 ms (p50), 13,7 ms (p95). “Çizimin payı” sütunu tabanı çıkarır.

**Gerçek pencere** (Xvfb ve lavapipe): NCZ'nin açılışı ile 8, 16 ve 20 çentik yakını, öncekiyle birebir aynı.

**Paketler:** native stil ve render-wgpu'nun bütün sınamaları (GPU dahil) geçti. Masaüstünde 120 sınama geçti.

## Kalan

- **Örnekli toplu çizimler ayıklanmıyor:** blok yerleştirmeleri ve vektör SVG simgeleri. Her yerleştirme tanımının
  hepsini çiziyor. Blok yoğun çizimlerde (AVM DWG) yerleştirmeler gözlere göre ayıklanmalı.
- **Gerçek GPU süresi için zaman damgası sorgusu** (RM-01'in eksiği). Harness'ın tabanı ölçümü örtüyor.

## Uygulama

- `crates/native/style/src/cells.rs`:
  - `CellPart::order`, `runs`, `fewer`;
  - `item_layout` (işaret: 5 sayı, sırasıyla);
  - `MOST_RUNS`, `JOIN_GAP`.
- Sınamalar:
  - `cells.rs`: `a_marker_batch_keeps_its_order_and_a_view_draws_the_runs_it_meets`;
  - `crates/render/wgpu/tests/gpu/cells.rs`: `markers_keep_their_order`.
