# 0934 — KentOS Yol: kurpta kübaj düzeltmesi (Pappus)

Tarih: 2026-11-04. plan/road.md Y30 (ROAD-VOL-03). Durum: Y30.1–Y30.3 kabul.

## Dayanak

- **Pappus'un ağırlık merkezi kuramı:** bir eksen çevresinde dönen düzlem alanın taradığı hacim, alan çarpı ağırlık
  merkezinin yoludur. Ortalama alan yöntemi kesitleri paralel sayar. Kurpta kesitler aksa dik olduğu için yelpaze gibi
  açılır: iç yandaki prizma aralıktan kısa, dış yandaki uzundur.
- Ders kaynaklarındaki düzeltme `(L/2R)(A₁e₁ + A₂e₂)`'dir; e ağırlık merkezinin dışmerkezliğidir. İşaret, e'nin iç ya da
  dış yanda olmasına göre değişir ve kaynaklar işareti farklı tanımlar.
- **Civil 3D:** Compute Materials'ta isteğe bağlı "Curve Correction Tolerance" vardır; yalnız ortalama alan yönteminde
  açılır, prizmoidal ve bileşik yöntemde kapalıdır. Montana DOT'un tercihi kapalıdır (Procedure EWK).
- **OpenRoads Designer:** ortalama alan hacminde kurp düzeltmesi seçeneği yoktur. ODOT'un örneğinde (tek sola kurp,
  iki yan yarma) ortalama alan sonucu 3B ağ hacminden %0,25 büyük çıktı. Aks iki yana da kıvrılıyorsa düzeltme
  gerekmeyebilir ([ODOT Earthwork Volumes](https://www.oregon.gov/odot/EAST/Documents/EarthworkVolumes.pdf)).
- **GEOPAK:** iki ardışık kesitin ağırlık merkezi ötelemeleri ortalanır. Bu ötelemedeki eşmerkezli aks boyu düzeltilmiş
  uzunluktur. CFLHD her projede bunu varsayılan yapar ("Advanced Earthwork with GEOPAK").
- **NETPRO:** çevrimiçi belgesi bulunamadı.

## Karar

### §1 Çekirdek (Y30.1)

- **Eğri boyunca yol:** aksa dik kesitte, eksenden o uzaktaki (sağa artı) bir alan parçası aks ds ilerlerken
  `ds·(1 + κ·o)` yol alır. κ işaretli eğriliktir, sola dönüş artıdır. Sola dönüşte sol yan içtedir ve daha az yol alır.
- **Hacim:** `V = ∫ (A + κ·Q) ds`. Q = ∫ o dA alanın aksa göre birinci momentidir. `A′ = A + κ·Q = A(1 + κ·e)` etkin
  alandır, e ağırlık merkezinin ötelemesidir. Sabit kesit dairede bu tam Pappus'tur.
- **Yöntem:** etkin alanlarla ortalama alan, `L(A′₀ + A′₁)/2`. Klotoidde κ doğrusal değiştiği için ikinci mertebeden
  yaklaşıktır. Ders kaynaklarındaki `(L/2R)(A₁e₁ + A₂e₂)` düzeltmesinin işaretli ve klotoide genişlemiş hâlidir: işaret
  tartışması κ'nın ve o'nun işaretinde çözülür.
- **`template::moments(section, ground, strip)`:** kazı ve dolgunun birinci momentleri. Alanlarla aynı döngüde
  hesaplanır: doğrusal derinlik f için `∫o·f do = (x₁ − x₀)(f₀(2x₀ + x₁) + f₁(x₀ + 2x₁))/6`, f'nin işaret değiştirdiği
  yerde bölünür. Sıyırma kalınlığı alanlarla aynı biçimde uygulanır.
- **`volume::effective`, `end_areas_curved`:** kesit kurbun merkezini geçiyorsa (`1 + κ·o ≤ 0`) etkin alan yoktur ve
  aralık sayılmaz. Bu, Y29'daki kesişen kolların hacimdeki karşılığıdır: o kesit kendisiyle örtüşür.

### §2 Kübaj (Y30.2, KCAD şema 167)

- **`VolumeDef.curve`:** yalnız `endArea` ile olur (Civil 3D gibi). Prizmoidal ve yüzeyler yönteminde kural reddeder.
  Kayıtta 167. şemadadır.
- **Hesap:** her kesitte yolun kazı ve dolgusunun ağırlık merkezi alınır (moment/alan). Eğrilik aksın o km'sindedir:
  eklemde sonraki eleman sayılır. Uzanım kesitin en dış noktalarıdır.
  - Kural ya da üstyapı kazıyı veya dolguyu değiştirirse (formasyon) düzeltme aynı ağırlık merkeziyle ölçeklenir:
    `A(1 + κe)`.
  - Sıyırma, diş kazısı ve tabaka hacimleri düzeltilmez.
- **Merkezi geçen kesit:** o aralık düz ortalama alanla sayılır ve `volume_curve_past_centre` uyarısı verilir. Uygulama
  kuralı çekirdeğin `volume::effective`'inden alır.
- **Rapor:** kazı ve dolgu düzeltilmiş değerlerdir. `curve` düzeltmenin etkisini (Δ kazı, Δ dolgu) ve düz kalan aralık
  sayısını verir. Kurp düzeltmesi kapalıyken çıktı bayt bayt eskisidir.
- **Ortak durumlar:** 7 yeni durum vardır (düzenleme 4, sorgu 3). Referans momentleri §1'in çokgen yöntemiyle, eğriliği
  bağımsız aks referansından alır.

**Ölçüm 1 — gerçeği bilinen deney.** R 60 m'lik 90° sola kurp, arazi kurbun merkezine göre dönel:
z = 100 + 0,2·(r − 60). Dış yan yarma, iç yan dolgudur ve daire boyunca her kesit aynıdır. Kesin hacim Pappus'tan:
kazı 173,80, dolgu 132,49 m³. Daire kısmının (km 140–234,25) KentOS hacimleri ve gerçeğe göre hataları:

| Kurpta örnekleme | Ortalama alan | Ortalama alan + kurp | Yüzey bindirme |
|---|---|---|---|
| 10 m | kazı −%3,8, dolgu +%4,1 | kazı +%0,3, dolgu −%0,5 | kazı −%6,2, dolgu +%6,7 |
| 5 m | kazı −%3,8, dolgu +%4,1 | kazı +%0,3, dolgu −%0,6 | kazı −%1,3, dolgu +%1,3 |
| 2 m | kazı −%3,7, dolgu +%4,0 | kazı +%0,5, dolgu −%0,7 | kazı +%0,3, dolgu −%0,5 |

- Düz ortalama alanın hatası bir **sapmadır**: sıklaştırma onu gidermez.
- Yüzey bindirme sıklaştırdıkça gerçeğe yaklaşır, çünkü kesitleri planda bağlar ve kurbu kendiliğinden içerir.
- Kurp düzeltmesi 10 m'de bile yüzey bindirmenin 2 m'de vardığı yere varır. Ortak kalıntı (+%0,3), dönel arazinin
  2,5 m'lik üçgen ağla yaklaşıklığından gelir.

**Ölçüm 2 — örnek proje** (samples/yol-ornegi.kcad, R 300 m, kesitler aşağı yukarı simetrik): kurp düzeltmesi kazıyı
+5,6 m³ (%0,04) değiştirir. Yöntemlerin birbirinden farkı (ortalama alan, prizmoidal ve yüzeyler arasında %0,2–0,4)
bundan büyüktür. ODOT'un "aks iki yana kıvrılıyorsa ya da kesit simetrikse düzeltme gerekmeyebilir" sözü burada
doğrulanır. Etki, ağırlık merkezinin ötelemesinin yarıçapa oranıdır (κe).

### §3 Kübaj penceresi (Y30.3)

- Tanım bölümünde **Kurp** anahtarı vardır: "Kesitlerin ağırlık merkeziyle düzelt". Yardımı nedenini söyler: kurpta
  kesitler yelpaze gibi açılır, Pappus. Anahtar ortalama alan yönteminde açılır, öbür yöntemlerde devre dışıdır ve
  nedenini söyler.
- **Yöntem değişince düzeltme kapanır.** İlk görüntüde prizmoidale geçince anahtar devre dışı ama açık kalıyordu.
  Kaydet kural gereği reddedilirdi ve kullanıcı bunu değiştiremezdi.
- Sonuçta bir not, düzeltmenin kazıyı ve dolguyu düz ortalama alana göre ne kadar değiştirdiğini söyler. Merkezi
  geçen kesit sorgunun uyarısıyla ayrıca listelenir.
- Ortak çizimde (R 150 m) etki: kazı +0,5 m³, dolgu +3,5 m³.
- Not: anlık görüntü düzeneği çizim ağacını korur ve anahtarın kayma animasyonu orada ilerlemez. Durumu, çizilmiş bir
  anahtarın altında değiştirilen bir görüntü yarı yolda görünür. Görüntü testleri her durumu ayrı bir uygulama
  örneğiyle çeker.

Bekçi: `volume_window_tests::the_curve_correction_is_switched_said_and_saved`.

## Doğrulama

- 17 kesit: tip kesit referansının kesin durumları. Referans kazı ve dolgu bölgelerini çokgen olarak kurar, alan ve
  momenti shoelace formülüyle kesirlerde hesaplar. Alanlar tip kesit referansınınkiyle aynı çıkmak zorundadır; referans
  bunu kendisi de denetler.
- 37 süpürme: sabit kesit R 40, 150, 1000 m'lik daire yayında sola ve sağa döner. Hacim dönel cismin kesin hacmidir
  (θ·∫ r dA). Bir durumda sol kol merkezi geçer; o aralık sayılmaz.
- Örnek: yamaçtaki kesitin kazısı solda, ağırlık merkezi −5,65 m'dedir. R 150 m'lik sola kurpta kazı hacmi %3,8 azalır.

Bekçi: `tests/all/curve_volumes.rs`.

## Açıklar

- Prizmoidal yöntemde kurp düzeltmesi. Civil 3D de vermez.
