# ADR 0157: Stilli çizimin uzak karoları

- **Durum:** kabul edildi (2026-10-02).
- **Tarih:** 2026-10-02
- **Bağlam belgesi:** ADR 0156'nın 4.2. adımında bulunan kusur; TODOS.md `REN-07` (float64 → float32, yüksek ve düşük parçalar), ADR 0090 (stilli katmanlar, WGSL sözleşmesi), ADR 0121 (büyük katmanın parçaları), CLAUDE.md §4.9.

## Bağlam

Vektör oturtmanın sahnesinde (`fixtures/interaction/v1/vector-fit.kcad`) yerel ölçü katmanı (1000, 2000) çevresindedir, projenin çapası (`origin`) ise TM'de (487 100, 4 420 200) noktasıdır. Bu katmana yakınlaşınca daire ve yaylar iki platformda da basamaklı çizilir.

Nedeni stilli çizimin konumlarıdır. Ortak stil çekirdeği (`kentos_style_core::style::batch`) her konumu katmanın tek orijinine (çizimin çapası) göre **tek** float32 olarak paketler. Kameranın merkezi de çizim hattına tek float32 gider (`Frame.offset`). Çapadan 4 400 km uzakta float32'nin adımı 0,5 m'dir: köşeler yarım metrelik ızgaraya oturur, kaydırırken görüntü yarım metre zıplar.

Düz çizim hattında (`cad2d`, `REN-07`) bu sorun yoktur: konumlar yüksek ve düşük iki float32 parçayla gider. Masaüstünde her katman stil motorundan çizildiği (ADR 0090) ve web de katmanları aynı yoldan çizdiği için güvence stilli yolda kaybolmuştur.

Bugünkü hata çapaya uzaklıkla büyür:

- ±10 km içinde 0,5 mm'den azdır.
- 46 km'de 2 mm'dir.
- 4 400 km'de 0,25 m'dir.

Yerel koordinatlı bir ölçü TM projesine alındığında (oturtmanın tam girdisi) çizim bozuk görünür.

## Karar

### 1. Karolar

Stil çekirdeği her ilkeli (çizgi yolu, dolgu, işaret) bir **karoya** koyar. Karolar çapaya ortalanmış, kenarı T = 2¹⁶ m = 65,536 km olan bir ızgaradır.

- **Karonun indisi:** ilkelin ilk noktasından alınır (yolun ilk noktası, dolgunun ilk halkasının ilk noktası, işaretin yeri). i = yuvarla((x − çapa.x) / T), j = yuvarla((y − çapa.y) / T).
- **Karonun orijini:** çapaya göre (i·T, j·T) noktasıdır. Bu sayı float32'de tamdır (2¹⁶'nın küçük katı).
- **Topluluk anahtarı:** karo da topluluğun anahtarına girer. Aynı stil farklı karolarda ayrı topluluktur.
- **Veriler:** topluluğun verileri kendi karosunun orijinine göredir.
- **JSON:** karo (0, 0) değilse topluluğun tanımına `"origin": [i·T, j·T]` yazılır.
- **Sınırlar:** kutu (`bounds`) eskisi gibi çapaya göredir; görünürlük denetimi değişmez.

Sonuçlar:

- Çapanın ±32,768 km çevresindeki her ilkel karo (0, 0)'dadır. Veri, topluluk sayısı ve çizim bugünküyle bit bit aynıdır. Web'in görsel karşılaştırması ve masaüstünün stil resimleri değişmez.
- Uzak veri kendi karosunu alır. Hatası karonun içindeki uzaklıkla sınırlıdır, çapaya uzaklıkla değil: karo merkezinden 46 km'de en çok 2 mm, birkaç km içinde 0,5 mm'den az.
- Bir ilkel karonun dışına taşıyorsa (uzun bir yol) hatası kendi boyuyla büyür. 100 km'lik bir çizgide 4 mm'dir.

### 2. Kamera: yüksek ve düşük parça

Çizim hattına kameranın çapaya göre merkezi iki float32 olarak gider: `offset` = f32(m), `offsetLo` = f32(m − offset).

Gölgelendirici konumu `((p − (offset − origin)) − offsetLo) · pxPerM` sırasıyla hesaplar:

- `origin` tamdır ve `offset`'e yakındır, bu yüzden ilk çıkarma tamdır (Sterbenz).
- Kameranın yakınındaki bir konumdan bu fark da tam çıkar ve sonuç küçüktür.
- İnce parça en son çıkar.

Böylece karo çapadan ne kadar uzak olursa olsun kameranın hiçbir adımı kaybolmaz. Kaba ve ince parçayı önce toplamak yanlış olurdu: karonun merkezine göre kamera onlarca kilometre olabilir ve float32 toplamda ince parçayı yutar.

WebGL2'de kameranın karoya göre farkı her çizim çağrısında JavaScript'in float64'ünde hesaplanır. Fark kaba ve ince iki parça olarak gider: `u_cam`, `u_camLo`.

### 3. Desen ve taramaların evresi

Tarama, doku ve desen dolgularının evresi dünyaya bağlıdır: gölgelendirici konumdan çizgi aralığını, döşeme hücresini ve desen satırını hesaplar.

Karo (0, 0) dışında, sayfa topluluğu çözerken (decode) evreyi karonun orijiniyle **katlar**. Böylece gölgelendirici küçük sayılarla aynı deseni çizer, komşu karolarda desen kesintisiz sürer.

Dünya birimli boyalar için kurallar:

- **Tarama:** n = (−sin θ, cos θ) olmak üzere `offset' = offset − ((origin·n) mod aralık)`. Kesik taramada çizgi boyunca evre `dash_offset' = dash_offset + ((origin·dir) mod desenin toplamı)` olur; toplam, tek sayılı desen iki kez yazıldıktan sonraki toplamdır (`dash_values`).
- **Doku (tile):** R dönüklük olmak üzere `offset' = offset − ((R·origin) mod boyut)`, bileşen bileşen.
- **Desen:** `offset' = offset − ((R·origin) mod (boyut.x, boyut.y·k))`. Satırlar şaşırtmalıysa k = 2'dir ki satırın tekliği korunsun; değilse k = 1.

Mod her zaman negatif olmayan kalandır (`rem_euclid`). Ekran birimli boyalar evresini ekrandan alır; katlanmaz.

Bilinen sınır: desenin rastgeleliği (`jitter`, `coverage`) hücrenin indisinden gelir. Uzak karoda indisler kaydığından rastgele dizilim karo (0, 0)'dakinden başka olur. Kafes ve evre aynıdır.

### 4. Sözleşme

`styled.layout.json` sürüm 3 olur:

- `Frame`'e `offsetLo: vec2<f32>` eklenir (32 → 40 bayt).
- `SStyle`'a `origin: vec4<f32>` eklenir (xy: karonun orijini, metre; zw: boş; 144 → 160 bayt).
- `toPx(p) = ((p − (frame.offset − st.origin.xy)) − frame.offsetLo) · pxPerM`.

Web'in WebGPU arka ucu ve masaüstünün wgpu çizim hattı aynı WGSL'yi kullanır. WebGL2'nin GLSL ikizi `u_cam`'i çağrı başına alır.

### 5. İki platform

- **Çekirdek:** `style::batch`'te karolar ve `origin` (Rust; masaüstü yerli, web WASM).
- **Sayfa:** masaüstü `kentos_native_style::batches::decode`, web `render/styledBatches.ts`. Her ikisi `origin`'i okur (yoksa 0), evreyi katlar ve `origin`'i topluluğun anahtarına katar (ADR 0121'in `merged_order`'ı için).
- **Çizim hattı:** masaüstü `crates/render/wgpu/src/styled`, web `render/webgpu/styledRenderer.ts` ve `render/webgl2/styledRenderer.ts`.

### 6. Doğrulama

- **Çekirdek testleri:** aynı stil çapada ve 4 400 km ötede iki topluluk olur; uzaktakinin verileri karosuna göredir; karo (0, 0) JSON'u değişmez.
- **Ortak fixture:** `fixtures/style/v1/batches.json`'a uzak karo durumu eklenir: taramalı, desenli ve dokulu alan, kesikli çizgi ve işaret, çapada ve uzakta. Web kaydeder; iki platform çekirdeğin yanıtını ve sayfanın katladığı değerleri aynı bulur.
- **Hassasiyet:** gölgelendiricinin adımları CPU'da float32 ile birebir yürütülür (`tests/styled_precision.rs`). Ölçütler:
  - 4 400 km ötedeki bir köşe her yakınlıkta dünyada 2 mm içinde olmalıdır. Karosunun merkezinden 27 km uzaktadır; float32'si 1 mm'ye iyidir.
  - Uzaktaki 4 m'lik daire 100 px/m'de yuvarlak çizilmelidir. Eskiden köşeleri yarım metrelik ızgaraya oturuyor, 25 px kayıyordu.
  - En derin yakınlıkta 0,01 mm'lik kaydırmalar 0,05 px'lik eşit adımlarla ilerlemelidir.
  - Çapanın karosundaki konumlar eskisiyle aynı float32 olmalıdır.
- **Görüntü:**
  - `vector-fit.kcad`'in yerel katmanına yakınlaşınca daireler ve yaylar iki platformda düzgün çizilmelidir.
  - Web'in `pnpm e2e:visual`'ı ve masaüstünün stil resimleri değişmemelidir (karo 0).
  - Paylaşılan WGSL tarayıcıda derlenmelidir (`scripts/wgsl/browser-check.mjs`).

### 7. Kapsam dışı

- Konumların her köşede yüksek ve düşük parçayla gitmesi (konum belleğini ikiye katlar). Karolar normal çizimde hiçbir şey eklemeden uzak veriyi çözer.
- Kaydıkça orijini kameraya taşımak (yüzer orijin). Uzak sıçramada her şeyi yeniden kurmayı gerektirir.

## Sonuçlar

- Projenin çapasından binlerce kilometre uzaktaki çizim (yerel koordinatlı ölçü) iki platformda tam çizilir; kaydırma zıplamaz.
- Çapanın ±32 km çevresindeki çizimde hiçbir şey değişmez.
- Uzak karolarda rastgele desenlerin dizilimi başka olabilir (§3).
