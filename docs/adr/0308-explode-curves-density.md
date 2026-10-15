# ADR 0308: Patlat'ta eğriler ve nokta sıklığı

- **Durum:** kabul edildi (2026-10-15). Sahibin bildirimi (15 Ekim): “Patlat bir çok obje için çalışmıyor, ayrıca nokta sıklığı
  artırılıp azaltılabilir olmalıdır. Mesela bir çemberi patlattım diyelim noktaları azaltıp artırabilmeliyim, veya diğerleri bu cbs
  ve Cad her ikisinde de olmalı.” Kapsamı ben belirledim.
- **Bağlam belgesi:** ADR 0302 (anlamsal Patlat), 0299 (Böl ve Ölç'ün yerleri), 0149 (eğrilerin 0,1 mm'lik kirişleri), 0258
  (kapalı çoklu çizgi).
- **Web:** ertelendi (ADR 0228); TODOS.md §0.2.

## Bağlam

Patlat (`cad.entities.explode`) daireyi ve elipsi reddediyordu (“parçalamak için Kır kullanın”). Yay temel nesne sayılıyordu. Eğri
ve NURBS eğrisi sabit 0,1 mm'lik kirişlerle tek bir çoklu çizgiye dönüyordu; köşelerin sıklığı seçilemiyordu.

AutoCAD'in EXPLODE'u daireyi patlatmaz. Harita ve kadastro çiziminde ise eğriyi köşeli bir çizgiye çevirmek gündelik bir iştir
(Netcad'in eğriyi noktalara ayırması, bir CBS'ye ya da aletlere giden veri). Sıklığın kullanıcıda olması gerekir.

## Karar

### 1. Eğriler

Daire, yay, elips, eliptik yay, eğri (Catmull–Rom) ve NURBS eğrisi Patlat'ta **tek bir çoklu çizgi** olur:

- Köşeleri eğrinin üstündedir ve bölümler eğri boyunca **eşit uzunluktadır**. Yerler Böl ve Ölç'ün `division_places`'iyle bulunur
  (ADR 0299): daire ve yayda tam, elipste gerçek eğri boyunca Newton adımlarıyla, eğri ve NURBS'te 0,1 mm'lik kirişlerin üstünde.
- Kapalı eğri (daire, tam elips, kapalı eğri) **kapalı çoklu çizgi** olur: n köşe, ilki eğrinin başında. Açık eğri n + 1 köşelidir,
  uçları eğrinin uçlarıdır.
- Çoklu çizgi eğrinin katmanını, görünüşünü ve verisini (öznitelikler, etiket, ekler) alır: aynı şeydir, başka çizilmiştir.

Çizgi, nokta, yardımcı çizgi, ışın ve tek satırlık yazı zaten temel nesnedir; uyarıyla atlanır. Çok satırlı yazının satır satır
ayrılması (AutoCAD'in MTEXT patlatması) ayrı bir iştir.

### 2. Nokta sıklığı

`cad.entities.explode`'un yeni alanı `density`:

| Tür | Anlamı |
|---|---|
| `count` (`segments`) | eşit bölüm sayısı; kapalı eğri en az 3 |
| `step` (`length`, m) | bölümler bu uzunluktan uzun olmaz: n = ⌈L / aralık⌉ |
| `deviation` (`tolerance`, m) | kirişler eğriye bu uzaklıktan yakın: θ = 2·acos(1 − t/ρ), n = ⌈L / (ρθ)⌉ |

ρ eğrinin en küçük eğrilik yarıçapıdır: dairede ve yayda yarıçap, elipste b²/a, eğri ve NURBS'te 0,1 mm'lik kirişlerin ardışık
üç köşesinin en küçük çevrel yarıçapı. Bölümler eşit uzunlukta olduğu için en kısa yarıçapın kuralı her bölümü tolerans içinde tutar.

`density` verilmezse daire, yay ve elips 0,01 m sapmayla (`EXPLODE_DEVIATION`) patlar. Eğri ve NURBS ise eskisi gibi 0,1 mm'lik
kirişleriyle patlar; kayıtlı çağrılar değişmez.

**Sınırlar:**

- Bölüm sayısı 1 ile 100 000 arasında olmalı; aralık ve sapma sıfırdan büyük ve sonlu olmalı. Değilse ret: `invalid_density`, yolu
  alanın kendisi.
- Bir eğri 100 000 köşeden çok olacaksa patlatılmaz; nedeni söylenir (`not_explodable`).

### 3. Araç

Seçimde eğri varsa Patlat hemen yazmaz, açık kalır:

- **Önizleme:** gelecek çoklu çizgiler ve köşelerindeki halkalar.
- **İstem:** nesne ve köşe sayısı; seçenekler Patlat (Enter), Sayı (S), Aralık (A) ve Sapma (P), seçili olanın yanında ●.
- **Seçenekler:** harf yolu seçer; yazılan sayı onun değeridir (Aralık ve Sapma projenin uzunluk biriminde). Seçenekler oturumda
  kalır (`Memory::explode_way`, `explode_count`, `explode_step`, `explode_deviation`); ilk değerler sapma 0,01 m, 24 bölüm, 1 m.
- **Yazma:** Enter tek adımda yazar (“Patlat”); yeni nesneler seçili kalır.

Eğrisiz seçimde araç eskisi gibi hemen yazar. CAD'de Değiştir'de, CBS'de Düzenle › Nesne'de aynı araçtır.

## Uygulama

- **Çekirdek:** `crates/shared/geometry-core/src/ops/explode.rs` (`Density`, `curve_polyline`, `explode_with`). Web'in çağırdığı
  `explode_entity` (`explodeEntity`) web dururken eski cevabını korur: daireyi ve elipsi reddeder, yayı temel sayar; donmuş çağrı
  kayıtları değişmez (ADR 0228). Web dönünce `explode_with`'e geçer (TODOS.md §0.2).
- **Sözleşme:** `crates/shared/contracts/src/cad_explode.rs` (`ExplodeDensity`, `EXPLODE_DEVIATION`, `MAX_EXPLODE_VERTICES`);
  `invalid_density`; katalog ve yetenekler (hedef türlere daire, yay, elips, NURBS).
- **Komut:** `crates/native/application/src/explode.rs` (`density`, eğrinin verisi korunur, `plan_in`).
- **Araç:** `crates/native/interaction/src/object.rs` (Patlat'ın sıklık aşaması).
- **Python ve MCP:** `kentos.cad.entities.explode`'un `density`'si (SDK yeniden üretildi); MCP aracı katalogdan.
- **Resimler:** `apps/desktop/src/explode_scenes.rs`.

## Doğrulama

- **Bağımsız başvuru:** `scripts/fixtures/explode_curve_cases.py` KentOS kodu olmadan yazıldı (mpmath, 40 basamak; elipsin yay
  uzunluğu sayısal integralle, yerleri kök bularak). Ortak durumlar `fixtures/commands/v1/cad.entities.explode.curves.json`'dadır
  (25 durum):
  - daire, yay, elips ve eliptik yay için üç sıklık türü ve varsayılan;
  - kapalı eğride en az 3 köşe; birlikte patlayan eğriler; temel nesnenin uyarısı;
  - eğrinin bölüm sayısı (köşeleri çekirdeğin kirişlerinden, birim testinde);
  - retler (sıfır ve sınır dışı sayı, sıfır aralık, eksi sapma, sayı olmayan aralık, çok sık, kilitli katman);
  - plan, geri alma.

  Köşeler planın nesnelerinde ±1e-9 m ile denetlenir. Eski başvurunun (`explode_command_cases.py`) daireyi bekleyen iki durumu
  noktaya çevrildi.
- **Çekirdek testleri:** `ops::explode`'un birim testi (eğrinin sayısı ve sapmayla sıklığı).
- **Araç:** `crates/native/interaction/tests/all/explode_density.rs` (açık kalan araç, istem, önizlemenin halkaları, Sayı ve Aralık,
  sınır dışı sayı, geri alma).
- **Resimler:** CAD'de daire ve yay (varsayılan, 12 bölüm, yazılmış), CBS'de elips (2 m aralık); 1440×900 ve 1100×650, açık ve
  koyu.
