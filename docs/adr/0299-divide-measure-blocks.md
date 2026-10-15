# ADR 0299: Böl ve Ölç'te blok, hizalama, ilk uzaklık ve uçlar

- **Durum:** kabul edildi (2026-10-15). [plan/cad.md](../../plan/cad.md) §8 `CAD-EDIT-13`: “Divide/measure boyunca nokta veya blok
  yerleştirme; aralık/sayı, başlangıç, hizalama ve son nokta politikasıyla açık komut olsun.” Kapsamı ben belirledim.
- **Bağlam belgesi:** ADR 0057 (Böl aracı), 0144 (blok ve yerleştirme), 0254 §5 (anlamsal `cad.entities.divide`), 0298 (elips yayının
  uzunluğu).
- **Web:** ertelendi (ADR 0228); TODOS.md §0.2.

## Bağlam

`cad.entities.divide` (ADR 0254 §5) yalnız nokta koyuyordu. Parça sayısı ya da aralık ve hangi uçtan ölçüleceği girdideydi. Planın
istediklerinden üçü eksikti:

1. **Blok:** AutoCAD'in DIVIDE ve MEASURE'ı noktaların yerine bir bloğun yerleştirmelerini koyar ve onları nesneye hizalayıp
   hizalamayacağını sorar. Direk, ağaç, rögar ya da km taşı dizmek bu yoldan yapılır.
2. **Başlangıç:** aralıkla bölmede ilk nokta hep aralık kadar ötedeydi. Yol ve hat işlerinde ilk direk ya da kazık çoğu zaman başka bir
   uzaklıktadır (yaya geçidinden 5 m sonra, her 30 m'de).
3. **Son nokta:** AutoCAD de KentOS da nesnenin uçlarına nokta koymaz. Aralığın kısa kalanından sonra uca bir nokta, ya da iki uca birer
   direk, çizimde sık istenir ve elle eklenirdi.

Ayrıca elips yayının noktaları, uzunluğu ince kirişlerle ölçülen yoldan bulunuyordu. 4 m'lik bir elips yayında gerçek yay uzunluğundan
7 µm sapıyordu. Bağımsız başvuru bunu buldu.

AutoCAD'le karşılaştırma:

| AutoCAD DIVIDE / MEASURE | Önce | Şimdi |
|---|---|---|
| Parça sayısı (DIVIDE), aralık (MEASURE) | var | aynı |
| MEASURE seçilen yere yakın uçtan ölçer | var (`fromEnd`) | aynı |
| Block: blok adı, “Align block with object? [Yes/No]” | yok | `block` (`name`, `align`, `scale`, `rotation`) |
| Uçlara nokta | konmaz | konmaz; `ends` ile baş, son ya da ikisi |
| İlk noktanın uzaklığı | yok | `start` (aralıkla) |
| Elips | kirişlerle | gerçek yay uzunluğuyla |

## Karar

### 1. Yerler

Çekirdeğin `ops::path::division_places`'i (masaüstü, başsız sunucu ve web'in WASM'ı tek kaynaktan) yerleri ve nesnenin oradaki
doğrultusunu verir:

- **Parça sayısı** n: s = L·k/n (k = 1 … n−1; kapalı nesnede k = 0 … n−1).
- **Aralık** d: s = d, 2d, … L'den küçükken. `start` a verilmişse s = a, a + d, … (a = 0: uçta da).
- `fromEnd` ölçmeyi nesnenin sonundan başlatır.
- **Elips yayı:** uzunluk gerçek yay uzunluğudur. Yer, kirişlerin tahmininden başlayıp gerçek uzunlukta Newton adımlarıyla bulunur.

### 2. Uçlar

`ends` (`none`, `start`, `end`, `both`) açık nesnenin kendi başına, sonuna ya da ikisine de bir yer ekler. Orada zaten bir yer varsa
eklemez (1e-9 · max(1, L) içinde). Kapalı nesnenin ucu başlangıcıdır, parçaların yerleri arasında zaten vardır.

Yazılma sırası şöyledir: ölçmenin başladığı uç, bölmenin yerleri, ölçmenin bittiği uç.

### 3. Blok

`block` verilirse her yere noktanın yerine bloğun bir yerleştirmesi konur:

- taban noktası yerin kendisidir; ölçek `scale`, varsayılan 1'dir;
- dönüş `rotation` radyandır; `align` (varsayılan açık) ile nesnenin oradaki doğrultusu eklenir;
- doğrultu nesnenin başından sonuna giden yöndür, `fromEnd` onu çevirmez (AutoCAD gibi);
- blok adıyla bulunur.

Yerleştirmeler `cad.entities.create`'in `insert` geometrisiyle, katmanın ve görünüşün kurallarıyla yazılır.

### 4. Retler

Yeni retler, bölmenin kendi retlerinden sonra şu sırayla:

- `not_finite` ve `invalid_distance` (`start` sıfırdan küçük);
- `start_needs_step` (parça sayısıyla `start`);
- `not_finite` ve `invalid_scale` (bloğun sayıları);
- `unknown_block` (o adla blok yok);
- `no_points` (ilk uzaklık nesnenin boyundan uzun; iletisi başlangıcı söyler).

### 5. Araç

Böl aracı komutun planıyla önizler (noktalar ya da yerleştirmelerin kesikli çizgileri) ve komutun yürütmesiyle yazar. Yeni seçenekler:

| Seçenek | Ne yapar |
|---|---|
| Blok (B) | çizimin sıradaki bloğu; sonuncudan sonra yine nokta |
| Nokta (N) | noktalara döner |
| Hizala (H) | yerleştirmelerin nesneyle dönmesini açar ya da kapatır |
| Uçlar (U) | yok, baş, son, iki uç |
| İlk (İ) | yalnız aralıkta: ilk uzaklığı sorar; Enter tek başına aralığa döndürür |

Seçimler uygulama açık kaldıkça hatırlanır (`Memory::divide_block`, `divide_align`, `divide_ends`, `divide_start`). İleti “9 blok
kondu.” gibidir.

## Uygulama

- **Çekirdek:** `crates/shared/geometry-core/src/ops/path.rs` (`division_places`, `DivisionPlace`, `DivisionEnds`,
  `ellipse_param_at`).
- **Sözleşme:** `cad_divide.rs` (`DivideBlock`, `DivideEnds`, `start`, `ends`, `block`); `codes.rs`'te `START_NEEDS_STEP`; katalog
  açıklaması ve örneği, yetenekler.
- **Komut:** `crates/native/application/src/divide.rs` (`places_of`).
- **Araç:** `crates/native/interaction/src/divide.rs`; `Memory`'nin dört alanı.
- **Python ve MCP:** `kentos.cad` yeniden üretildi; MCP aracı katalogdan gelir (yalnız ekler, yıkıcı değil).
- **Manifest:** `tool.divide` zaten `semantic`; notu güncellendi.
- **Resimler:** `apps/desktop/src/divide_scenes.rs` (`bol-blok`, `bol-blok-yazildi`, `bol-blok-hizasiz`).

## Doğrulama

- **Bağımsız başvuru:** `scripts/fixtures/divide_place_cases.py` KentOS kodu olmadan yazıldı (elips yayının uzunlukları mpmath ile 40
  basamakta). Ortak durumlar `fixtures/commands/v1/cad.entities.divide.places.json`'da (27 durum):
  - çizgi, köşeli çoklu çizgi, yay, daire ve elips yayında hizalı blok; hizasız, dönük ve ölçekli blok;
  - sondan ölçmek;
  - başlangıç 1 ve 0;
  - uçlar: son, iki uç, aralık tam bölerken yinelenmeyen uç, parça sayısıyla, kapalı alanda;
  - sondan ölçerken baş;
  - aralık nesneden uzunken yalnız iki uç;
  - bloğun uçlarda hizalanması;
  - bütün yeni retler.

  Planlar 1e-9 içinde, yazmalar bire bir karşılaştırılır. Eski `cad.entities.divide.json`'ın 22 durumu değişmeden geçer. Yetenek testi
  yeni kodları görür.
- **Komut testi:** `crates/native/application/tests/all/divide.rs`: elips yayının altı parçası gerçek yay uzunluğunda eşittir
  (200 000 örnekle, 1e-7 içinde).
- **Araç:** `crates/native/interaction/tests/all/drawing.rs`: Blok, Hizala, Uçlar, plandan önizleme, “5 blok kondu.”, İlk ve Enter.
- **Resimler:** 1440×900 ve 1100×650, açık ve koyu.
- **Süreler:** release, 100 000 çizgilik çizim; `divide::timing`:

  | İş | Süre |
  |---|---|
  | 10 000 hizalı bloğun planı (10 km'lik çoklu çizgi) | 7,68 ms |
  | Elips yayında 1 000 yer (gerçek uzunlukla) | 12,73 ms |
  | 10 000 bloğun yazılması | 15,80 ms |

## Kalanlar

- Blok öznitelikliyse yerleştirmelerin değerleri sorulmaz (AutoCAD de sormaz); Blok öznitelikleri penceresinden sonra verilir.
- Ölçek ve dönüş araçta sorulmaz (1 ve 0); komutta, Python'da ve MCP'de verilir.
- Web'in aracı ve komutun web işleyicisi: TODOS.md §0.2.
