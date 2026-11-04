# 0936 — KentOS Yol: taşıma filosu — çevrim süresi, enerji ve yükleyici kuyruğu

Tarih: 2026-11-04. plan/road.md Y32 (ROAD-HAUL-10). Durum: Y32.1 kabul; Y32.2–Y32.3 bu belgeyle ilerler.

## Dayanak

- **Caterpillar Performance Handbook'un yöntemi** (ders kitapları, Caterpillar'ın taşıma yolu sunumu ve Datamine,
  MineSched belgeleri aynı yöntemi anlatır):
  - Toplam direnç yuvarlanma direnci ile eğim direncinin toplamıdır. Yuvarlanma direncinin her %1'i eğime %1 ekler.
    Bakımlı kalıcı taşıma yolunda en az %1,5–2 alınması önerilir.
  - Hız, çekiş kuvveti (rimpull) eğrisinden okunur. Kullanılabilir çekiş, tahrik dingilindeki ağırlık ile çekiş
    katsayısının çarpımını aşamaz.
- **Carlson, Haul Cycle Analysis** (Surface Mining):
  - Çevrim = yükleme + yüklü gidiş + kuyruk + boşaltma + boş dönüş. Gidiş süreleri hesaplanır; yükleme, boşaltma ve
    kuyruk elle girilir.
  - Her kesimin süresi Newton'un ikinci yasasıyla bulunur: çekiş, yuvarlanma ve eğim kuvvetleri. Hız sınırı yolun,
    taşıtın ve iniş tablosunun en küçüğüdür. Kesimin sonunda durulacaksa fren tepki süresi ve en büyük yavaşlamayla
    durulur.
  - Çıktılar: üretim, gereken filo, dakika/üretim saati, doldurma verimi.
- **Uyum katsayısı (match factor)** `N·t_yükleme/t_çevrim` filo büyüklüğünü yükleyiciye eşler. Kaynaklar
  (Sembakutti, Kumral ve Sasmito 2017; IIETA 2020) onun belirlenimci olduğunu söyler. Çevrim, yükleme ve bekleme
  sürelerinin değişkenliğini görmez. Sonlu kaynaklı kuyruk modeli ("machine repair model", Koenigsberg) filo
  büyüklüğü için kullanılır.
- **Yakıt ve karbon:**
  - Dizelin yanmadan doğan CO₂'si 2,68–2,69 kg/L'dir (EPA 10,15–10,21 kg/galon; UNFCCC CDM 2,69, yoğunluk
    0,85 kg/L ile). Kuyudan tanka dahil değerler (3,16 kg CO₂e/L) ayrı bir tanımdır.
  - Yakıt motor işinden özgül yakıt tüketimiyle hesaplanır.
- **Civil 3D, OpenRoads, NETPRO:** Brükner'den kamyon çevrimine giden bir hesap bulunamadı. Civil 3D'nin kütle
  taşıma diyagramında serbest ve fazla taşıma vardır; kamyon ve çevrim yoktur.
- Türkçe kaynaklar (DergiPark, İTÜ tezleri): eğim ve yuvarlanma direnci kamyon taşımacılığının maliyetini belirler.
  Ekskavatör ile kamyon eşlemesinde bekleme olasılıkları hesaba katılır.

## Karar

### §1 Çekirdek (`kentos_road::haul_fleet`, Y32.1)

- **Değerlerin hiçbiri KentOS'tan gelmez:** kütleler, güç, verim, çekiş, hızlar, fren, yuvarlanma direnci, sabit
  süreler.
- **Gidiş:**
  - Eğim açısı θ'da direnç `R = m·g·(c_r·cos θ + sin θ)`'dır. Çekiş `F(v) = min(η·P/v, μ·D·m·g·cos θ)`'dır; tekerleğe
    varan güç, tahrik tekerleklerinin tutunmasından fazla olamaz.
  - Eğim uzunluğu u boyunca `m·v·dv/du = F − R`, w = v² için Heun yöntemiyle çözülür. Adım en çok 0,25 m'dir ve v²'yi
    en çok %2 değiştirir.
  - Hız üç sınırın altında tutulur: en yüksek hız, yokuş aşağı itilen yerde iniş hızı ve sonda durduran fren eğrisi
    `v² = 2b(L − u)`.
  - Adımın süresi `2h/(v₀ + v₁)`'dir. Sabit ivmede kesindir; bu yüzden 1/v'nin sınırsız olduğu kalkış da kesin
    toplanır.
- **İş:** tekerleğin verdiği kuvvet `m·(w₁ − w₀)/(2h) + R`'dir; sıfırın altına inmez, çünkü frenlemek motorun işi
  değildir. Toplamı tekerlekteki iştir. Kuvvetin sıfır olduğu süre motorun rölantisidir.
- **Durma:** tutunması eğimin direncinden az olan kamyon orada durur (`Stall`).
- **Taşımalar:**
  - Planın kesim akışları doğruda kaynak ve çukur verir. İleri akışlar soldan sağa, geri akışlar sağdan sola süpürülür.
  - İlk giren ilk çıkar: yükler birbirini geçmez. Taşımaların momenti planınkine eşittir.
- **Yükleyici:** N kamyon tek yükleyiciyi paylaşır.
  - Belirlenimci hesap: `min(N/(a + s), 1/s)` yük ve uyum katsayısı.
  - Üstel sürelerle sonlu kaynaklı kuyruk: `p₀ = [Σ N!/(N − n)!·(s/a)ⁿ]⁻¹`, meşguliyet `1 − p₀`, üretim `(1 − p₀)/s`,
    çevrim başına bekleme `N/X − a − s`.
  - Gerçek süreler üstelden az değişir; doğru değer ikisinin arasındadır. İkisi de verilir, hiçbirine "üretim"
    denmez.

**Ölçüm (yöntemin seçimi).** Az güçlü %5 tırmanışın (150 kW, 40 t) kapalı çözümü 160,416 250 s'dir. Sabit 0,25 m
adım 160,493 s verdi: 77 ms geç (5·10⁻⁴), hata adımın karesiyle küçülüyordu. Hata kalkıştaydı ve tutunmanın güce
bıraktığı yerdeydi. v² değişimiyle sınırlanan adım 0,47 ms'ye indi; adım sayısı yalnızca 2402'den 3108'e çıktı.

## Doğrulama

24 ortak durum vardır (`scripts/fixtures/road_haul_fleet_cases.py`):

- **16 gidiş:** düz, %5 ve %8 çıkış, %6 iniş (sınırlı ve sınırsız), kırıklı güzergâh yüklü ve boş, en yüksek hıza
  varmayan kısa yol, uzun yumuşak yol, az güçlü, kaygan kalkış, %8'i çıkamayan kaygan kamyon, sert fren, yavaş üst
  hız.
  - Referans zamanda çözer: (u, v) durumu 1 ms'lik RK4 ile ilerler, hız sınırlara izdüşürülür. Fren eğrisine
    girildiğinde kalan süre v/b'dir.
  - Çekirdekten en büyük göreli süre farkı 6,9·10⁻⁵'tir.
  - Tepe hızı fren eğrisiyle buluşma noktasındadır. Çekirdek onu adımların üstünde birkaç mm/s düşük okur.
- **8 yükleyici:** referans Markov zincirinin denge denklemlerini sayısal çözer (numpy), kapalı çarpım formülünü
  kullanmaz.
- **Fizik:**
  - Düz yolda iş, kinetik enerjiyle frenlemeye kadar olan yuvarlanmanın toplamıdır (‰1 içinde).
  - Tırmanışta iş potansiyel enerjiden büyüktür.
  - Az güçlü tırmanış kapalı çözümüyle 1 ms içinde çıkar.
- **Taşımalar:** moment planınkine eşittir; ilk giren ilk çıkar.

Bekçi: `tests/all/haul_fleet.rs`.

## Açıklar

- Alanlara yoldan ayrılan erişim yolu: alanlar aksın km'sindedir.
- Rimpull eğrisi tablosu, vites; motorun gücü sabit alınır.
- Yüklü ve boşta tahrik dingili payının farkı.
