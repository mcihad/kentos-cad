# ADR 0500: Bindirme işlemlerinin doğruluk korpusu ve GEO-01

- **Durum:** kabul edildi (2026-10-17). `plan/gis.md` CBS-13; TODOS.md `GEO-01`.
- **Bağlam belgesi:** ADR 0201 (geometri işlemleri), 0209 §12 (hizmet alanı), 0221 §10 (kesişim çekirdeğinin sağlamlık
  kuralları), `docs/specs/numeric-policy.md`.
- **Web:** ertelendi (ADR 0228); çekirdek ortak, web'in tarafı yok.

## Bağlam

GEO-01: bindirmenin birleşimi kendi sonucunu, sınırına değen tamponlarla yeniden birleştirince alanı iki kez sayıyordu;
kanıtı `network.rs`'in yok sayılan testiydi. GIS planı yeni analizden önce bu açığın kapanmasını ve zincirlenen
işlemlerin bağımsız bir korpusla sınanmasını istiyor.

## Karar

### 1. GEO-01 kapandı, test açıldı

ADR 0221 §10'un kuralları (TOL içindeki çemberler bir çember, yakın kesişim üçüncü kenarı da keser, köşelerin dengesi)
hatayı kapatmış: `geo01_a_union_joined_again_with_touching_circles` artık yok sayılmıyor ve geçiyor.

### 2. Korpus

`fixtures/overlay/v1/cases.json` (30 durum), `scripts/fixtures/overlay_cases.py`'den:

- **Düz kenarlılar:** GEOS ile (GDAL'ın OGR'ı). Bu gruptaki durumlar:
  - örtüşen, ortak kenarlı ve kısmen ortak kenarlı kareler; köşede değen kareler;
  - içteki kare, deliği dolduran, deliğe giren ve deliğe taşan kare;
  - 0,1 mm'lik şerit ve boşluk, neredeyse paralel kenarlar;
  - aynı kare, çok parçalı alan, tarak, 5 × 5 ızgara ve deliği;
  - kenara değenlerin zinciri;
  - TM koordinatlarında (500 000, 4 400 000) örtüşme, ortak kenar ve 1 mm'lik şerit.
- **Yaylılar:** kapalı formüllerle, mpmath ile 50 basamakta. Örtüşen, dıştan ve içten teğet daireler, karenin iç teğet
  dairesi, farklı yarıçaplar, teğet tampon zinciri, örtüşen daire zinciri ve TM koordinatında mercek.

Her durumda birleşim, kesişim ve iki fark ölçülür; alan ve delik sayısı da başvuruyla aynı olmalıdır. Rust tarafı
`crates/shared/geometry-core/tests/all/overlay_cases.rs`'tir. Ek olarak şunları denetler:

- her sonuç alanı geçerlidir (`validity::problems` boş);
- birleşim idempotenttir (kendisi ve `a` ile yeniden birleşince aynıdır);
- |A∪B| = |A| + |B| − |A∩B| ve |A−B| + |A∩B| = |A|;
- `chain` kümeleri birbiri ardına birleşince hepsinin birden birleşimiyle aynı ölçülür (GEO-01'in biçimi).

Pay düz kenarlarda 10⁻⁷ m², TM koordinatında 10⁻⁶ m², yaylarda 10⁻⁹ m²'dir.

### 3. Çekirdeğin toleransı açık

Kesişim çekirdeği 10⁻⁶ m (`arrangement::TOL`) içindeki kenarları bir sayar. Bu yüzden 1 µm kalınlığındaki şerit
(2,5·10⁻⁵ m²) kaybolur; GEOS onu tutar. Korpus bunu gizlemez:

- `toleransta paralel` durumunda şeridin gitmesi beklenen sonuç olarak yazılıdır; payı tolerans × kenarın 100 m'si,
  yani 10⁻⁴ m²'dir.
- 10⁻⁴ m'lik `neredeyse paralel` durumu GEOS'la kesin eşleşir.

Toleransı büyütmek ya da sonucu sessizce onarmak yoktur.

### 4. Bulunan başvuru hatası

İçten teğet dairenin farkı tek alandır; deliği dış halkaya tek noktada değer (OGC'de geçerli). KentOS böyle verir; ilk
başvuru deliği saymamıştı, düzeltildi.

## Sonuçlar

- GEO-01 kapandı; zincirlenen birleşim korpusla sınanıyor.
- GEO-02 (değen tamponların süresi) ayrı bir performans maddesi olarak açık kalır.
- GEOS'u ayrı bir hesap arka ucu olarak denemek gerekmedi.
