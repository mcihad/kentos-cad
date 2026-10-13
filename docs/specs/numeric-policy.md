# Sayısal politika

KentOS'un sayılarının nerede, hangi biçimde ve hangi kesinlikle tutulduğu, nerede yuvarlandığı ve hangi kararın toleransla verildiği.

Kaynaklar:

- CLAUDE.md §23;
- ADR 0004 (kadastral sayısal politika), 0149 (ölçü doğruluğu ve gösterim yuvarlaması), 0157 (uzak karolar);
- ADR 0253 §2 (kanonik JSON), 0256 ve 0257 (birim kuralı);
- [kcad-v2.md](kcad-v2.md).

Bu belge onları birbirine bağlar. Kural değiştiren her iş buraya da yazılır (plan/cad.md `CAD-DATA-24`).

## 1. Dört tür sayı

| Tür | Ne | Biçim | Kural |
|---|---|---|---|
| Kaynak | geometri: koordinat, uzunluk, açı, kot | float64 | yuvarlanmaz; ölçeklenirken tek tam oranla (§3) |
| Gösterim | ekranda, ölçü değerinde, etikette yazılan | metin | `display::fixed` (§4); hesaba geri girmez |
| Karar | çakışma, kenet, topoloji, kesişim | float64 ve tolerans | açık tolerans ya da kesin öngörü (§5) |
| Kesin kullanıcı verisi | tapu alanı, hisse | ondalık metin, kesir | `DecimalString`, `ShareValue`, `NumericPolicy` (§6) |

## 2. Kaynak: float64, metre, projenin düzlemi

- **Birim:** geometri projenin düzleminde metredir (coğrafi sistemde derece). Yerel projenin çizim birimi yalnız okuma ve yazma
  birimidir (ADR 0165 §2, 0245 §3).
- **Dosyalar sayıyı bit bit taşır:**
  - `.kcad` v2 CBOR float64 yazar;
  - kanonik JSON ve DXF en kısa geri dönüşlü ondalığı yazar: Rust'ın `{}` biçimi, `num::dxf_real` (ADR 0253 §2).
- **Telde sayı float64'tür** (JSON sayısı). Python'un `float`'ı ve MCP'nin JSON'u aynı biti taşır. Her sayının birimi kataloğun
  şemasında `x-unit`'tir (ADR 0256 §3).
- **Çizim hattı kaynak koordinatları değiştirmez:**
  - GPU'ya karonun merkezine göre float32 fark gider (65,536 km'lik karolar, ADR 0157). Paketleme float64 kaynağı okur, GPU'nun
    kendi sayılarını yazar; çizimin koordinatlarına dokunmaz.
  - Kamera iki parçalıdır (kaba ve ince).
  - Seçim, kenet ve ölçü hep float64 kaynaktan hesaplanır.
  - Ekrandaki sapma, köşenin karonun merkezinden uzaklığındaki float32 adımı kadardır. Çapadan 12 m'de pikselin ellide biri;
    3 km'de adım 0,24 mm'dir, 1 mm'lik kenar 0,98 mm görünür; 27 km'de adım 2 mm'dir. En derin yakınlıkta milimetre altı çizim
    `REN-16`'da açıktır; bu kaynak koordinatı değil yalnız çizimi etkiler.

## 3. Ölçekleme ve birim çevirisi

- Birimden metreye ve metreden birime çeviri tek tam oranla yapılır (`kentos_contracts::units::Scale`): `v ÷ 1000` (mm),
  `v × 254 ÷ 10000` (inç). Çarpan ve bölen float64'te tam sayılardır; bir çarpma, sonra bir bölme.
- Yazılan değer, DXF'in `$INSUNITS`'i (ADR 0257) ve `cad.units.change` (ADR 0256 §1) aynı yoldan geçer.
- `mm → m → mm` gidiş dönüşü değeri çoğunlukla bit bit geri verir. Vermediği yerde fark, çarpımın ve bölümün ayrı yuvarlanmasından
  gelir ve kaynağın bir ulp'ı kadardır: TM koordinatında (4,4·10⁶ m) yaklaşık 10⁻⁹ m.

## 4. Gösterim

- Bir sayı yazılırken önce yedi ondalığa yuvarlanır (gürültü, 0,1 µm), sonra gösterilen basamağa. İkisinde de yarım sıfırdan uzağa
  gider (`kentos_geometry_core::display::fixed`, ADR 0149). Sıfıra yuvarlanan değer eksi işaretsiz yazılır.
- Yazılan değer hesaba geri girmez.
- Gösterilen basamak proje ayarıdır (`lengthDecimals`, `areaDecimals`; 0–4). Kaynağı değiştirmez.
- `toFixed`, `Math.round`, SQL'in `round`'u iş kuralı değildir (CLAUDE.md §23.2).
- Durumları `fixtures/numeric/v1/display.json`'dadır (`scripts/fixtures/numeric_display.py`).

## 5. Karar ve tolerans

| Tolerans | Değer | Nerede |
|---|---|---|
| Eğrinin kirişleri | 0,1 mm (`CURVE_TOL`) | elips ve eğri: kenet, seçim, kesişim, budama sınırı, Böl, alan işlemleri (ADR 0149 §5.3); uzunluk kirişten değil, kesin |
| Kesişim çekirdeği | 10⁻⁶ m (`arrangement::TOL`) | alan işlemleri, örtüşmeler, bölgeler (ADR 0201, 0221) |
| Mekânsal sorgu | 1 mm (`spatial_query::TOLERANCE`) | Konuma göre seç'in ilişkileri (ADR 0200) |
| Topoloji | proje ayarı, 1 mm varsayılan | topoloji kuralları, topolojik temizlik (ADR 0148, 0202) |
| Kenet ve seçim | ekranda piksel | yalnız neyin tutulduğunu seçer; yeri kaynaktan gelir |
| Kayıtlı ölçü denetimi | proje ayarı | ADR 0180 |

- Yön, içerme ve boş çember gibi kararlar kesin öngörülerle verilir (`predicates::orient2d`, `incircle`).
- Tolerans büyütülerek hata örtülmez. Eşiği kesen belirsiz durum `needs_review` ya da ret olur (CLAUDE.md §23.3).

## 6. Kesin kullanıcı verisi

- Tapu alanı, hisse ve resmî değerler float64 değildir:
  - ondalık metin (`DecimalString`: üssü olmayan ondalık yazı);
  - kesin kesir (`ShareValue`: pay ve payda tam sayı yazısı).
- Yuvarlama yalnız sürümlü bir `NumericPolicy` ile yapılır: niceliği, birimi, basamağı, yuvarlama yönü (`RoundingMode`) ve artık
  kuralı (`RemainderRule`) açıktır. Taslak politika önizler, kesinleştiremez (`PolicyStatus`, ADR 0004).
- Geometrik alan, kayıtlı alan ve gösterim ayrı alanlardır, birbirini ezmez.
- Komut kaydının satırı hesabın politikasını söyler (`AlgorithmInfo.policy`: `kentos.numeric/1`, ADR 0253 §3).

## 7. Doğrulama

- Her hesap bağımsız başvuruyla sınanır (mpmath 50–60 basamak, `fractions`, PROJ, GeographicLib, GEOS); native ve WASM eşitliği
  ayrıca.
- Gösterim kuralının durumları `fixtures/numeric/v1/display.json`'da, yuvarlama ve dağıtımınkiler `rounding.json`,
  `distribution.json`, `shares.json`'dadır.
- Küçük ayrıntı ve büyük koordinat birlikte sınanır (`CAD-DATA-25`):
  - TUREF / TM33'te, 4 412 km kuzeyde 1 mm'lik kare ve 1 mm'lik iki çizgi (başsız sunucunun
    `a_millimetre_detail_on_tm_coordinates_survives_measure_edits_and_files` testi):
    - alan ve çevre köşelerin float64 farklarından hesaplananla aynı (10⁻¹⁸ m², 10⁻¹⁵ m);
    - Buda'nın kesişimi 1 nm içinde;
    - `.kcad` ve DXF gidiş dönüşünde her sayı bit bit aynı.
  - Çizimin konumları (çizim hattının `styled_precision` testleri, ADR 0157;
    `a_millimetre_detail_is_off_by_no_more_than_its_float32_step_on_screen`): 1 mm'lik kenar en derin yakınlıkta (1 mm = 10 px)
    çapanın yakınında boyunu pikselin ellide biri içinde korur, uzakta en çok konumun float32 adımı kadar sapar.
