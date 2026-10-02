# ADR 0156: Vektör oturtma (N noktadan dönüşüm)

- **Durum:** kabul edildi (2026-10-02). Sıra sahibin kararıdır: TODOS.md §16.0'ın beşinci işi `HYB-05`. Ayrıntılar bu ADR'nin varsayılanlarıdır. Kauçuk levha (rubbersheet) ve komşu pafta kenar eşlemesi maddenin ikinci yarısıdır; kendi ADR'sinde karara bağlanır (§10).
- **Tarih:** 2026-10-02
- **Bağlam belgesi:** TODOS.md `HYB-05` (ilgili: `GIS-05`, `GIS-08`, `NUM-10`), [araştırma kaydı](../research/2026-10-01-netcad-arcgis-qgis.md); ADR 0037 ve 0047 (`cad.entities.transform`, Hizala), ADR 0070 ve 0075 (Hesap pencereleri ve ölçü tablosu), ADR 0149 (ölçü doğruluğu, eğrilerin 0,1 mm'lik açık sınırı), ADR 0152 (adlı nokta, `#ad`), ADR 0153 (nokta editörü).

## Bağlam

Eski paftalardan sayısallaştırılmış, yerel koordinatlı ya da başka bir ölçümle kaymış çizimler ülke sistemine (ör. TUREF/TM) oturtulur: hem çizimde hem hedef sistemde koordinatı bilinen ortak noktalardan (kontrol noktaları) bir dönüşüm hesaplanır, artıklarına bakılır, kötü noktalar çıkarılır, sonra çizim ya da katman bu dönüşümle taşınır.

KentOS'ta bugün yalnız Hizala vardır (iki nokta çiftiyle benzerlik, ADR 0047): çok noktadan en küçük kareler çözümü, artıklar, m0 ve afin ya da projektif dönüşüm yoktur.

Netcad bunu Dönüşümler (N noktadan Helmert ve Afin) ve XY Yönünde Ölçekle ile, ArcGIS Pro Transform (Similarity, Affine, Projective) ve Calculate Transformation Errors ile, QGIS vektör katmanı için Georeferencer ve Affine transform ile yapar.

## Karar

### 1. Kontrol noktaları

Bir kontrol noktası (eşlenik nokta) bir çifttir: **kaynak** (çizimdeki yeri: Y, X) ve **hedef** (oturtulacağı yer: Y, X). Her çiftin bir adı ve **Kullan** işareti vardır.

- **Kaynak:** çizimden seçilir (Sahneden seç, kenet ile: bir noktaya kenetlenince adı da gelir) ya da yazılır.
- **Hedef:** yazılır, elektronik tablodan yapıştırılır ya da çizimden seçilir (ülke sistemindeki noktalar çizimdeyse).
- **Adla eşleme:** iki katman seçilir (kaynak noktaların ve hedef noktaların katmanı); aynı adı taşıyan noktalar çift olur (adlar baştaki ve sondaki boşluklar atılarak, tam). Bir katmanda aynı ad birden çok noktadaysa o ad eşlenmez ve söylenir.
- Kullan'ı kapalı çift çözüme girmez; artığı yine hesaplanır ve gösterilir (denetim noktası).

### 2. Dönüşüm türleri

Kodda x doğu (Y), y kuzey (X); hepsi float64, projenin biriminde.

| Tür | Denklem | Parametre | En az çift |
|---|---|---|---|
| **Helmert** (benzerlik) | x′ = a·x − b·y + c, y′ = b·x + a·y + d | 4 | 2 |
| **Afin** | x′ = a₁·x + a₂·y + a₃, y′ = b₁·x + b₂·y + b₃ | 6 | 3, bir doğru üstünde olmayan |
| **Projektif** | x′ = (a₁·x + a₂·y + a₃) / (c₁·x + c₂·y + 1), y′ = (b₁·x + b₂·y + b₃) / (c₁·x + c₂·y + 1) | 8 | 4, üçü bir doğru üstünde olmayan |

- **En küçük kareler:** Helmert ve afin, hedef koordinatlarla dönüştürülmüş kaynak arasındaki farkların kareleri toplamını en küçük yapan çözümdür. Projektif, paydasıyla çarpılmış (doğrusallaştırılmış) denklemlerin en küçük kareler çözümüdür: x′·(c₁·x + c₂·y + 1) = a₁·x + a₂·y + a₃ ve y′ için aynısı. Bu doğrusal çözüm geometrik farkların kareler toplamını en küçük yapmaz (dört çiftte ikisi aynıdır, tam geçer); artıklar yine gerçek (geometrik) farklardır.
- **Merkezleme:** denklemler kullanılan çiftlerin kaynak ve hedef ağırlık merkezlerine göre yazılır (x̄ = x − x₀); merkezler ilk çifte göre küçük farklardan bulunur, toplamlar hep merkezlenmiş koordinatlarındır. Büyük koordinatlarda (4 420 000 m) normal denklemler sayı kaybetmez. Helmert ve afinin çözümü merkezlemeden bağımsızdır; projektifin tanımı merkezlenmiş denklemlerdir: bu tanımın çözümü iki sistemin ölçeğine ve dönüklüğüne bağlı değildir, yalnız merkezlemeye bağlıdır.
- **Kapalı biçimler:** Helmert a = Σ(x̄·x̄′ + ȳ·ȳ′) / Σ(x̄² + ȳ²), b = Σ(x̄·ȳ′ − ȳ·x̄′) / Σ(x̄² + ȳ²); afin her eksen için 2×2 normal denklem. Projektif 8×8 normal denklemdir; her sistem kendi en büyük mutlak koordinatıyla ölçeklenir (çözüm değişmez, denklemler iyi koşullu kalır), kısmi pivotlu Gauss yok etmesiyle çözülür.
- **Tekil durum:** çiftler türün gerektirdiğinden azsa, kaynak noktalar üst üste ya da (afin ve projektifte) bir doğru üstündeyse çözüm yoktur ve nedeni söylenir. Sayısal ölçüt: Helmert'te Σ(x̄² + ȳ²) sıfırsa; afinde normal determinant Σx̄²·Σȳ²'nin 1e-12 katından küçükse; projektifte (ölçeklenmiş sistemlerde) bir pivot normal matrisin köşegeninin en büyüğünün 1e-12 katından küçükse. Bağımsız başvuru tam kesirle çalışır; ortak durumlar bu sınırın uzağındadır (tam tekil ya da iyi koşullu).

### 3. Artıklar ve m0

- **Artık:** v = dönüştürülmüş kaynak − hedef; vY, vX ve uzunluğu √(vY² + vX²). Kullan'ı kapalı çiftin artığı da aynı çözümle hesaplanır.
- **m0:** √([vv] / (2n − u)); n kullanılan çift, u parametre sayısı (4, 6, 8). 2n = u iken (en az çift) çözüm tam geçer, m0 “—”dir.
- **Parametreler:** Helmert'te ölçek √(a² + b²) ve dönüklük atan2(b, a) (projenin açı biriminde); afinde X ve Y ölçekleri (sütunların uzunlukları), dönüklük ve kayma (eksenler arasındaki açının dik açıdan farkı); projektifte sekiz sayı.
- **En büyük artık** tabloda vurgulanır; bir çift çıkarılınca (Kullan) çözüm, artıklar ve m0 hemen yeniden hesaplanır.

### 4. Nesnelerin dönüşmesi

Dönüşüm kaynak geometriyi dönüştürür; yuvarlamaz, sadeleştirmez. Benzerlik (Helmert) her türü tam taşır (bugünkü taşı, döndür, ölçekle gibi). Benzerlik olmayan dönüşümde:

| Tür | Afin | Projektif |
|---|---|---|
| Nokta, çizgi, düz kenarlı çoklu çizgi ve alan (delik ve parçalarıyla), yardımcı çizgi | tam | tam (doğru doğruya gider) |
| Daire | tam: elips | 0,1 mm'lik açık sınırla kapalı çoklu çizgi |
| Yay | tam: elips yayı | 0,1 mm ile çoklu çizgi |
| Elips, elips yayı | tam: elips (yayı) | 0,1 mm ile çoklu çizgi |
| Çoklu çizgi ve alanın yaylı kenarları | 0,1 mm ile köşelere açılır (nesne tek kalır) | aynı |
| Eğri (spline) | uydurma noktaları dönüşür; eğri onlardan yeniden geçer | aynı |
| Yazı, öznitelik tanımı, kılavuzun notu | yeri dönüşür; yönü taban çizgisinin görüntüsü, boyu ve genişlik çarpanı kutunun taban uzunluğunu ve alanını korur (§5); kılavuzun notu son köşesindeki türevle | yerindeki türevle aynı kural |
| Blok yerleştirmesi | yeri dönüşür; dönüklüğü x ekseninin görüntüsü, ölçeği √\|det\| | yerindeki türevle aynı kural |
| Ölçü | tanım noktaları dönüşür, değer geometriden yeniden ölçülür; a ile b'nin ortasındaki türevle ofset ve yazı boyu √\|det\| ile çarpılır, doğrusalın doğrultusu türevin görüntüsüdür | aynı |
| Tarama | sınırları dönüşür; desenin açısı kendi doğrultusunun görüntüsü (dış sınırın köşelerinin ortalamasındaki türevle, 0 ile 180° arası), aralığı √\|det\| ile çarpılır | aynı |
| Işın | başlangıcı ve doğrultusu dönüşür | ufuk çizgisini kesen ışın reddedilir |

- **0,1 mm:** ADR 0149'un eğri sınırıdır. Eğri P(t), t₀'dan t₁'e: aralık en çok π/8'lik eşit adımlara bölünür; her adım, ortasının görüntüsü uçlarının görüntülerini birleştiren kirişe 0,1 mm'den uzak oldukça ikiye bölünür (en çok 30 kez); köşeler parametrelerin görüntüleridir, yay kenarının uçları kendi köşeleridir. Tam bir eğrinin son köşesi ilkidir. Yay kenarından açılan yeni köşeler, kenarın iki ucunun kotu varsa onları açıya göre doğrusal alır (ADR 0142 kural 3).
- **Sayılar söylenir:** çoklu çizgiye çevrilen eğriler sayısıyla söylenir: “4 nesnenin eğrileri 0,1 mm'lik köşelere açıldı.”
- **Yaklaşıklar söylenir:** yazı, blok ve tarama deseni biçimlerini korur, kesilmez ve eğilmez; dönüşüm benzerlik değilse sayılarıyla söylenir: “12 yazı ve 2 blok yerinde döndürülüp ölçeklendi; biçimleri eğilmez.”
- **Ufuk:** projektifte payda (c₁·x + c₂·y + 1) kontrol noktalarında pozitiftir. Dönüşecek bir köşede payda sıfıra ya da eksiye inerse (göreli 1e-9) nesneler ufuk çizgisinin ötesindedir; dönüşüm bütünüyle reddedilir.
- **Kot:** dönüşüm düzlemseldir; kotlar (köşe kotu, noktanın Z'si) değişmez.
- **Kilit:** kilitli katmandaki nesne dönüşmez; `cad.entities.transform`'un kuralı (ADR 0037).

### 5. Yazının boyu

Taban çizgisi birim vektörü u, dönüşümün doğrusal kısmı (projektifte yerindeki türevi) L ise:

- yön: L·u'nun yönü; aynalıysa yarım tur eklenir (okunur kalır, bugünkü kural);
- boy: boy · |det L| / |L·u|;
- genişlik çarpanı: çarpan · |L·u|² / |det L|.

Böylece yazının kutusunun taban uzunluğu ve alanı dönüşümün görüntüsününkiyle aynıdır. Benzerlikte boy ölçekle çarpılır, çarpan değişmez.

### 6. Komut

`cad.entities.transform` v1 üç dönüşüm daha alır (`kind`). Üçü de **merkezli** biçimdedir: `from` kaynaktaki merkez, `to` hedefteki merkez, sayılar merkezlenmiş koordinatlar arasındaki dönüşümdür (x̄ = x − from.x, ȳ = y − from.y). Çözüm zaten bu biçimde bulunur; ülke koordinatlarında (4 420 000 m) mutlak biçimin büyük sabitleri sayı kaybettirmez.

- **`similarity`** `{ from, to, a, b }`: x′ = to.x + a·x̄ − b·ȳ, y′ = to.y + b·x̄ + a·ȳ (Helmert).
- **`affine`** `{ from, to, m: [a, b, c, d] }`: x′ = to.x + a·x̄ + c·ȳ, y′ = to.y + b·x̄ + d·ȳ (çekirdeğin `Affine` sırası).
- **`projective`** `{ from, to, h: [a₁, a₂, a₃, b₁, b₂, b₃, c₁, c₂] }`: x′ = to.x + (a₁·x̄ + a₂·ȳ + a₃) / (c₁·x̄ + c₂·ȳ + 1), y′ = to.y + (b₁·x̄ + b₂·ȳ + b₃) / (c₁·x̄ + c₂·ȳ + 1).

Pencere parametreleri kullanıcıya mutlak biçimde de gösterir (Helmert'in a, b, c, d'si, afinin altı sayısı); projektifin mutlak biçimi gösterilmez, merkezleriyle birlikte merkezli sayıları gösterilir.

Retler, var olanlardan sonra: `not_finite` (sayılar), `invalid_transform` (doğrusal kısmın determinantı, sütunlarının uzunlukları çarpımının 1e-12 katından küçük: tekil), `beyond_horizon` (§4). Adım adı “Oturt”. Kopya seçeneği bu türlerde de geçerlidir. Sonucun uyarıları §4'ün sayılarıdır: `warp_curves` (çoklu çizgiye çevrilen eğriler), `warp_shapes` (biçimini koruyan yazılar, öznitelik tanımları, kılavuz notları, bloklar ve tarama desenleri).

### 7. Pencere: Vektör oturtma

Hesap pencereleri gibi (ADR 0070, 0075); yazılanlar oturum boyunca kalır, pencere çizime bakmak için kapatılıp yeniden açılabilir:

- **Üstte:** dönüşüm türü (Helmert, Afin, Projektif; altında en az kaç çift gerektiği) ve **Adla eşle**: kaynak ve hedef katmanı seçilir, Eşle tabloyu aynı adlı nokta çiftleriyle doldurur; eşlenmeyen ve birden çok noktada geçen adlar sayısıyla söylenir.
- **Tablo:** Kullan (onay kutusu), Ad, Kaynak Y, Kaynak X, Hedef Y, Hedef X, vY, vX, v (artıklar mm, değiştirilemez); Enter, ↑/↓, satır ekle ve sil, elektronik tablodan yapıştırma; satırın düğmeleri kaynağı ve hedefi çizimden seçer. Kullanılan çiftlerin en büyük artıklısı uyarı rengiyle vurgulanır; kullanılmayan satır soluk gösterilir, artığı yine hesaplanır.
- **Sonuç:** m0 (kullanılan çift sayısı ve serbestlik derecesiyle), parametreler (Helmert: ölçek ve dönüklük; afin: X ve Y ölçeği, dönüklük, kayma; projektif: merkezleriyle sekiz sayı) ve en büyük artık; çözüm yoksa nedeni.
- **Uygula:** nesneler (Seçili, bir Katman, Tümü) ve Kopya olarak; Uygula `cad.entities.transform` ile tek adımda yazar, iletiyi ve uyarıları söyler, pencere kapanır.
- **Rapor:** sistem panosuna metin olarak (dönüşüm, çiftler, artıklar, m0, parametreler).
- **Parametrelerle:** dönüşüm sayılarla da verilebilir (pencerenin üstündeki Yöntem: Kontrol noktaları | Parametrelerle): taban noktası (adı ya da Y,X; çizimden), Y (sağa) ve X (yukarı) ölçeği, dönüklük (projenin açı biriminde, saat yönünün tersine, özetteki dönüklük gibi) ve öteleme (ΔY, ΔX; metre). Dönüşüm p′ = taban + öteleme + R(dönüklük)·S(Y ölçeği, X ölçeği)·(p − taban)'dır; komuta merkezli afin olarak gider (`from` taban, `to` taban + öteleme, `m` = [sY·cos, sY·sin, −sX·sin, sX·cos], çekirdeğin `ops::fit::scale_turn`'ü). İki ölçek eşitse dönüşüm benzerliktir, her tür tam taşınır (§4). Boş ölçek 1, boş dönüklük ve öteleme 0 sayılır; ölçek sıfır olamaz (eksi ölçek aynalar). Tablo ve Adla eşle gizlenir; özet dönüşümü sözle söyler, ölçekler farklıysa eğrilerin elips olacağını ekler. Dönüklük ve öteleme sıfırken Netcad'in XY Yönünde Ölçekle'sidir.

Afinin özetindeki ölçekler de bu adlarla söylenir: Y ölçeği doğu ekseninin (kodda x), X ölçeği kuzey ekseninin (kodda y) görüntüsünün boyudur (CLAUDE.md §5).

Komut `transform.fit` (Harita › Koordinatlar; takma adlar OTURT, DONUSUM, HELMERT, AFIN), ikonu kendinindir.

### 8. Ortak çekirdek

- **`ops::fit`** (WASM `fitTransform`): çiftler (kaynak, hedef, kullan) ve tür → parametreler, artıklar, m0 ya da çözümsüzlüğün nedeni.
- **`ops::warp`** (WASM `warpShapes`): şekiller, yollarının kotları ve dönüşüm → dönüşmüş şekiller, kotları ve sayılar (§4), ya da ufuk reddiyle reddedilen şeklin sırası. Benzerlik var olan dönüşümle (taşı, döndür, ölçekle gibi), merkezli sistemlerde yapılır.
- **Bağımsız başvurular:** `scripts/fixtures/fit_cases.py` çözümü kesir aritmetiğiyle (float girdilerin tam kesirleri, merkezleme dahil) yazar; `scripts/fixtures/warp_cases.py` nesnelerin dönüşmesini kurallardan yazar (elipsin eksenleri, yazının boyu, ufuk). Ortak durumlar `fixtures/fit/v1`.

### 9. Kapsam dışı

- Koordinat sistemleri arası datum ve projeksiyon dönüşümü (`HYB-12`, `NUM-06`): oturtma bir ölçümü ötekine bağlar; sistem tanımı değildir.
- Kot dönüşümü (3B Helmert, düşey kayma).
- Raster oturtma (`GIS-08`).

### 10. Maddenin ikinci yarısı

Kauçuk levha (kontrol noktalarına göre parça parça dönüşüm, sabit noktalar) ve komşu pafta kenar eşlemesi bu ADR'nin ortak çekirdeğini kullanır; kuralları (üçgenleme, bağların etki alanı, eşleme toleransı) kendi ADR'sinde yazılır.

### 11. İş sırası

1. **Çözüm:** `ops::fit`, WASM, bağımsız başvuru ve ortak durumlar.

   *(2 Ekim: tamam.)*
   - **Çekirdek:** `ops::fit` (`fit`, `FitPair`, `FitKind`, `Fit::map`, `map_centred`); WASM `fitTransform`, web cephesi `model/ops/fit.ts`.
   - **Başvuru:** `scripts/fixtures/fit_cases.py` 15 durumu (Helmert, afin ve projektif; en az çift, gürültülü ülke koordinatları, çıkarılmış ve kaba hatalı çift; dört çözümsüzlük) float girdilerin tam kesirleriyle yazar: `fixtures/fit/v1/solve.json`. Çekirdek (yerli) ve web (WASM) merkezleri, artıkları ve m0'ı 1e-9 m, sayıları ve türetilen değerleri göreli 1e-12 içinde verir; projektifin tek sayısında milyarda birlik sapma yakalanır.
2. **Nesnelerin dönüşmesi:** `ops::warp`, bağımsız başvuru ve ortak durumlar.

   *(2 Ekim: tamam.)*
   - **Çekirdek:** `ops::warp` (`Warp`, `warp_shape`, `warp_shapes`); WASM `warpShapes`, web cephesi `model/ops/warp.ts`.
   - **Başvuru:** `scripts/fixtures/warp_cases.py` kurallardan 6 durum yazar (`fixtures/fit/v1/warp.json`): güçlü bir afin ve aynalı afin altında on beş türün hepsi (daire, yay ve elips tam elips; yaylı kenarlar köşelere; yazı, blok, ölçü, tarama, kılavuz kuralıyla), projektif (eğriler köşelere), ufka koşan ışının ve ufkun ötesindeki noktanın reddi. Çekirdek (yerli) ve web (WASM) koordinatları 1e-9 m, öbür sayıları göreli 1e-12 içinde, köşeleri bire bir verir; kiriş sınırını %10 gevşetmek ya da genişlik çarpanını bozmak yakalanır.
3. **Komut:** `cad.entities.transform`'un üç türü, iki platformda ortak durumlar.

   *(2 Ekim: tamam.)*
   - **Sözleşme:** `Transform`'un `similarity`, `affine`, `projective` türleri (katalog, TS tipleri, Python SDK'sının tipleri yeniden üretildi); retler `invalid_transform`, `beyond_horizon`; uyarılar `warp_curves`, `warp_shapes`; adım “Oturt”.
   - **İşleyiciler:** masaüstü `crates/native/application/src/transform.rs`, web `product/entitiesTransform.ts`: çekirdeğin `warp_shape`'iyle, yolların kotlarıyla (`elevation::assign`, `assignElevations`); türü değişen nesne kendi alanlarını korur.
   - **Ortak durumlar:** `cad.entities.transform.json`'a beş durum (benzerlik her türle; afin, kilitli katman ve iki uyarıyla; afin kopyası; projektif; retler: sonlu olmayan sayı yolu ve sırası, tekil afin, ufuk). Beklenen değerler dönüşümlerin tanımından, aynı işlem sırasıyla çift duyarlıkla; iki platform bit bit geçer. Eğrilerin ve yazıların kuralları `warp.json`'da. `affine_reference.py` artık alanın deliklerini ve parçalarını da taşır (önceki durumlarda delikli alan yoktu, hiçbiri değişmedi).
4. **Pencere:** Vektör oturtma iki platformda; resimler. İki parçada:
   1. tablo (Kullan sütunuyla; Hesap pencerelerinin tablosuna onay kutusu sütunu eklenir), Adla eşle, canlı çözüm ve artıklar, Uygula (kapsam, Kopya), rapor;

      *(2 Ekim: tamam.)*
      - **Tablo:** Hesap pencerelerinin tablosu onay kutusu sütunu (`check`; yapıştırma ona yazmaz) ve satır işareti (`mark`: kullanılmayan soluk, en büyük artıklı uyarı rengiyle) aldı: web `ui/calc/common.ts` (`Grid.refresh` girdileri yeniden kurmadan değerleri ve işaretleri yeniler), masaüstü `calc/grid.rs` (`Table::check`, `Table::mark`).
      - **Pencere:** web `ui/calc/FitDialog.ts`, masaüstü `calc/fit/`; komut `transform.fit` (Harita › Koordinatlar, ikonu `vectorFit`). Çözüm her değişiklikte çekirdekten; türetilen değerler (ölçek, dönüklük, kayma) çekirdeğin `Fit::derived`'ından, iki platformda aynı. Satırın iki düğmesi kaynağı ve hedefi çizimden seçer (noktaya kenetlenirse ad boşsa adı da gelir); Adla eşle aynı adlı noktaları çift yapar, bir katmanda birden çok geçen adı sayarak dışarıda bırakır; Uygula Seçili, Katman ya da Tümü'nü, istenirse kopyalarını tek adımda (Oturt) yazar, kopyaları seçer; ret pencerede kalır.
      - **Sahne:** `fixtures/interaction/v1/vector-fit.kcad` (yerel ölçü ve aynı altı noktanın TUREF ölçüsü, P5'te 15 cm kaba hata). Masaüstü `calc::fit::tests` (Adla eşle, P5'in en büyük artığı, P5 çıkınca m0'ın 48,5 mm'den 3,6 mm'ye inmesi, Uygula'nın tek adımı, kopyalar, kilitli katman, satırdan çizimden seçme, yapıştırma); web `shots.mjs vectorfit` aynı adımları denetler. İki platform aynı sayıları verir (m0 ±3,6 mm, ölçek 1,00015444, dönüklük 22,2845 g).
   2. Parametrelerle.

Her adım iki platformda, ortak fixture'larla, kendi commit'inde ilerler.

## Sonuçlar

- Yerel ya da kaymış çizimler ülke sistemine çok noktadan, artıkları görülerek oturtulur.
- Afin ve projektif dönüşüm benzerlik olmadığından eğrileri ve yazıları açık kurallarla taşır; yaklaşıklar sayılarıyla söylenir.
- Yeni bir komut yoktur; `cad.entities.transform` üç dönüşüm daha alır.

## Doğrulama

- **Çözüm:** parametreler, artıklar ve m0 kesir aritmetiğiyle yazılan bağımsız başvuruya göre iki platformda; büyük koordinatlar, en az çift, tekil durumlar.
- **Nesneler:** elips eksenleri, yaylar, yazı boyu, ufuk reddi bağımsız başvuruyla iki platformda.
- **Arayüz:** resimler iki temada, 1440×900 ve 1100×650.
