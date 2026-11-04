# 0934 — KentOS Yol: kurpta kübaj düzeltmesi (Pappus)

Tarih: 2026-11-04. plan/road.md Y30 (ROAD-VOL-03). Durum: Y30.1 kabul; Y30.2–Y30.3 bu belgeyle ilerler.

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
