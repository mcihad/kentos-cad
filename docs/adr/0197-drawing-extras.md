# ADR 0197: Çizim ekleri

- **Durum:** kabul edildi (2026-10-07). Sıra sahibin kararıdır: TODOS.md §16.1, `CAD-34`'ün ardından `CAD-35`; madde tek parçada biter ve
  sahibin 6 Ekim gecesi sözüyle CAD-36'ya dek ara verilmez. Ayrıntılar bu ADR'nin varsayılanlarıdır. Örnekler Netcad'in 2 Daireye Teğet
  Doğru ve 4. Köşeyi Oluştur'u, ArcGIS'in Create range rings'idir.
- **Bağlam belgesi:** TODOS.md `CAD-35`; ADR 0057 (çizim araçları, Daire'nin teğet yöntemleri), ADR 0140 (yapım noktaları), ADR 0149
  (ölçü doğruluğu).

## Bağlam

İki boru ya da iki ağacın çevresine teğet geçen sınır, kayış ya da hat çizmek; üç köşesi ölçülmüş bir paralelkenarın (bina, parsel) eksik
köşesini bulmak; bir noktadan eşit aralıklı menzil (etki, güvenlik, hizmet) halkaları çizmek bugün elle yapılıyor.

## Karar

### 1. İki daireye teğet doğru

- İki daire (daire ya da yay; yayda dairesi) arasındaki ortak teğetler: **dış teğetler** (iki daire doğrunun aynı yanında) ve **iç
  teğetler** (çapraz). C₁, C₂ merkezleri, r₁, r₂ yarıçapları, d = |C₂ − C₁|, u = (C₂ − C₁)/d, n sol dik (−u.y, u.x):
  - dış: d > |r₁ − r₂| iken; c = (r₁ − r₂)/d; k ∈ {+1, −1}: m = c·u + k·√(1 − c²)·n; teğet noktaları A = C₁ + r₁·m, B = C₂ + r₂·m;
  - iç: d > r₁ + r₂ iken; c = (r₁ + r₂)/d; m aynı biçimde; A = C₁ + r₁·m, B = C₂ − r₂·m.
  - Sıra: dış sol (k = +1), dış sağ, iç sol, iç sağ; olmayan (daire içinde daire, kesişen dairelerde iç teğetler, değen dairelerde
    eşitlikler) yoktur. Yayda teğet noktası yayın üstünde değilse o çözüm yoktur (yayın açısı içinde, uçlar dahil, 1e−9 içinde).
- **Seçim:** dört çözümden, tıklamaların yakınındaki: birinci dairede tıklanan yer P₁, ikincide P₂ (önizlemede imleç) için
  |P₁ − A| + |P₂ − B| en küçük olan (eşitse sıradaki ilk). AutoCAD'in “ertelenmiş teğet” kenetinin kuralıdır.
- Yazılan: A'dan B'ye çizgi, etkin katmanda, `cad.entities.create` ile; adım “İki daireye teğet”.

### 2. Dördüncü köşe

- Paralelkenarın üç köşesi A, B, C verilir (B ortadaki, A ile C'ye komşu); dördüncüsü D = A + C − B (B'nin karşısı). Üç köşe bir doğru
  üzerindeyse paralelkenar yoktur, söylenir.
- Yazılan: D'de nokta ya da **Alan** seçeneğiyle A, B, C, D kapalı alanı; `cad.entities.create` ile, adım “Dördüncü köşe”.

### 3. Menzil halkaları

- Merkez M, aralık s (metre, sıfırdan büyük), sayı n (1–100): yarıçapı k·s olan daireler, k = 1 … n.
- Işın sayısı m (0–360; 0: ışın yok): M'den en dış halkaya, kuzeyden saat yönünde eşit açılı çizgiler: j. ışının ucu
  M + n·s·(sin θⱼ, cos θⱼ), θⱼ = 2π·j/m, j = 0 … m − 1.
- Yazılan: daireler ve ışınlar tek adımda (`cad.entities.create`, adım “Menzil halkaları”).

### 4. Araçlar

- **İki daireye teğet** (`tangentLine`): birinci daireye ya da yaya, teğetin değeceği yerin yakınından tıklanır; ikincinin üstünde
  önizleme seçilecek teğeti parlak, ötekileri kesikli gösterir, imlecin yanında türü (Dış teğet, İç teğet) ve uzunluğu; tıklama yazar.
  Çözüm yoksa söylenir, birinci seçili kalır; Esc onu bırakır. Yardımcı çizgi ▾ ailesinde.
- **Dördüncü köşe** (`fourthCorner`): üç köşeye tıklanır (kenet çalışır, yazılabilir); önizleme paralelkenarı kesikli, dördüncü köşeyi
  halkalı ve koordinatıyla çizer. Çıktı (Ç): Nokta ya da Alan, oturum boyu hatırlanır.
- **Menzil halkaları** (`rangeRings`): merkeze tıklanır (kenet çalışır, yazılabilir); önizleme halkaları imleçle taşır. Aralık (A),
  Sayı (S) ve Işın (I) yazılır, oturum boyu hatırlanır; ilk değerler 10 m, 5, 0.
- Üç araç da proje türünden bağımsızdır: CAD'de Giriş › Çizim ▾, CBS'de Düzenle'nin çizim bölümlerinde (Yardımcı, Eğri, Nokta).

## Kapsam dışı

Halkaların uzaklık yazıları, elips ve eğriye teğet doğru, nokta ile daire arası teğet (Çizgi'nin Teğet kenetiyle yapılır).

## Uygulama

- **Sözleşme:** `CreateOperation`'ın `tangentLine`, `fourthCorner`, `rangeRings` adımları (`crates/shared/contracts/src/cad_create.rs`),
  adları iki işleyicide (`crates/native/application/src/create.rs`, `apps/web/src/product/entitiesCreate.ts`); katalog, TypeScript ve
  Python tipleri yeniden üretildi.
- **Çekirdek:** `crates/shared/geometry-core/src/tools/drawing_extras.rs`: `Round` (daire ya da yay), `common_tangents`,
  `chosen_tangent`, `fourth_corner`, `range_rings`; web'e `commonTangents`, `chosenTangent`, `fourthCorner`, `rangeRings` işlemleri
  (`apps/web/src/model/drawingExtras.ts`).
- **Masaüstü:** `crates/native/interaction/src/drawing_extras.rs` (`TangentLine`, `FourthCorner`, `RangeRings`); bellekte
  `fourth_area`, `ring_spacing`, `ring_count`, `ring_rays`. Eğri boyunca yazı'nın imleci de web'inki gibi nesne isteyen imleç oldu.
- **Web:** `apps/web/src/tools/drawingExtrasTools.ts`; katalogda üç araç (`tools/catalog.ts`), ikonlar `ui/icons.ts`'te.
- **İkonlar:** sahip uyurken seçenek sayfasının önerilenleri alındı: iki daire ve dış teğeti; üç köşesi tutamaç, dördüncüsü halkalı
  paralelkenar; menzil halkaları önce tam halkalar ve iki ışındı, 16 px'te Halka'ya ve Sahneden seç'in hedefine benzediği için tutamaçtan
  çıkan iki ışın arasında üç çeyrek halka (yelpaze) oldu.
- **İkon turu:** `cizim-paneli` sahnesi CAD'in Giriş › Çizim ▾ listesini (üç yeni araç ve ikonları) resimler.

## Doğrulama

- Bağımsız başvuru `scripts/fixtures/drawing_extras_cases.py` (ADR'den, KentOS kodu olmadan): sekiz teğet durumu (eşit ve farklı
  yarıçap, kesişen, dıştan değen, iç içe, uzak koordinat, yay ve iki yay), beş seçim, üç dördüncü köşe, üç halka takımı;
  `fixtures/drawing-extras/v1/cases.json`. Çekirdek aynı dosyayı yerli (`tests/all/drawing_extras.rs`) ve WASM üzerinden
  (`apps/web/src/tools/drawingExtras.wasm.test.ts`) 1e−9 içinde geçer; sınır dışı değerler reddedilir.
- Komut durumları: üç adımın adı, geri alma ve yineleme (`create_command_cases.py`, `fixtures/commands/v1/cad.entities.create.json`) iki
  işleyicide.
- Ortak iz `fixtures/interaction/v1/drawing-extras.json` üç klavye düzeninde iki platformda geçer; resimler 1440×900 ve 1100×650'de,
  koyu ve açık temada iki platformda aynı (`kentos-cad kullan drawing-extras`, `pnpm -C apps/web e2e:use drawing-extras`).
