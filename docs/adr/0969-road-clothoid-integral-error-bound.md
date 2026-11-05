# 0969 — Klotoid integralinin hata sınırı

Tarih: 2026-11-05. Durum: kabul (Y63). Kapsam ROAD-ALIGN-04.

## Sorun

ROAD-ALIGN-04: "Clothoid için k(s)=k0+(k1−k0)s/L ve theta(s)=theta0+k0s+(k1−k0)s²/(2L) tanımını kullan; Fresnel veya hata
sınırlı integration yöntemi ADR'de seçilsin."

Tanım ADR 0901 §1'de kullanılıyordu. Yöntem de belliydi: θ'nın en çok 0,2 rad döndüğü eşit parçalarda 10 noktalı
Gauss–Legendre. Ama hatanın sınırı hiçbir yerde yazılı değildi; tek kanıt bir serinin ilk terimlerine karşı 10⁻¹⁰'luk
bir testti.

## Araştırma

İki kabul görmüş yol vardır:

- **Fresnel integralleri:** Kareye tamamlama ile θ = α + (c/2)(t + β)² yazılır; β = k0/c, α = θ0 − k0²/(2c). İntegral
  √(π/|c|)·e^{iα}·[C(v) ± i·S(v)] olur; v = √(|c|/π)(t + β).
  - Sorunu: c → 0'da (neredeyse daire olan ove klotoid) β ve v büyür. C ve S farkları sıfıra yakın sayıların farkıdır;
    çift duyarlıkta basamak kaybedilir.
  - Hesap için ayrıca bir Fresnel uygulaması gerekir (seri, asimptotik açılım ve kesir).
- **Gauss–Legendre:** n noktalı Gauss kuralının hatası, integrand Bernstein elipsi E_ρ'ya analitik uzanıyor ve orada
  |f| ≤ M ise, |I − Iₙ| ≤ (64/15)·M·ρ⁻²ⁿ/(ρ² − 1)'dir. Kaynak: L. N. Trefethen, "Is Gauss quadrature better than
  Clenshaw–Curtis?", SIAM Review 50(1), 2008, Teorem 4.5; ayrıca *Approximation Theory and Approximation Practice*,
  Teorem 19.3. cos θ ve sin θ tam fonksiyonlardır, dolayısıyla koşul her ρ için sağlanır.

## Karar

**Gauss–Legendre kalır.** Hata sınırı aşağıda türetilir. Fresnel bağımsız referanstır (60 basamak, mpmath).

### Türetme

Kodun parça kuralı şudur:

- Dönüş T = (|k0| + ½|c||s|)·|s|; parça sayısı p = ⌈T/0,2⌉; parça boyu h = |s|/p ≤ 0,2/(|k0| + ½|c||s|).

Parçanın ortasında u ∈ [−1, 1] yerel değişkeniyle θ = θ_m + θ′_m·(h/2)·u + (c/2)(h/2)²·u² olur. Katsayılar sınırlıdır:

- |θ′_m| ≤ |k0| + |c||s| ≤ 2(|k0| + ½|c||s|), dolayısıyla |θ′_m|·h/2 ≤ **0,2**.
- |c|·h ≤ 0,2·|c|/(½|c||s|) = 0,4/|s| ve h ≤ |s|, dolayısıyla |c|·h²/8 ≤ **0,05**.

E_ρ'da |Im u| ≤ b = (ρ − ρ⁻¹)/2 ve |Im u²| ≤ 2ab'dir; burada a = (ρ + ρ⁻¹)/2. Bu yüzden |Im θ| ≤ b·(0,2 + 0,1·a)
olur. |cos z| ve |sin z| ≤ cosh(Im z) ≤ e^{|Im z|} olduğundan:

| ρ | |Im θ| sınırı | M | Bir parçada, [−1, 1] ölçeğinde hata |
|---|---|---|---|
| 5 | 1,104 | 3,02 | 5,6·10⁻¹⁵ |
| **10** | 3,490 | 32,8 | **1,4·10⁻²⁰** |
| 20 | 11,995 | 1,6·10⁵ | 1,7·10⁻²³ |

ρ = 10 ile parçanın hatası h/2 ile ölçeklenir. Parçaların toplamı |s|'dir; iki bileşen birleşince (√2) bütün integralin
kesme hatası **≤ 1,0·10⁻²⁰·|s|** olur. 10 km'lik bir klotoidde bu 10⁻¹⁶ m'dir. Kalan hata yalnız kayan noktalı
yuvarlamadır.

- **Parça sayısının üst sınırı:** 2²⁰'dir; T > 2·10⁵ rad'da etkin olur. Yolda böyle bir klotoid yoktur.
- **Aliyman ve kurp:** Kapalı biçimdir (kiriş s·sinc(k·s/2), ortalama doğrultuda); kesme hatası yoktur.

## Ölçüm

`scripts/fixtures/road_clothoid_integral_cases.py`, KentOS kodu olmadan 12 klotoidi Fresnel integralleriyle 60
basamakta yazar. Her değeri mpmath'in kendi integraliyle de sınar: fark < 10⁻⁴⁰, yoksa betik durur. Durumlar:

- düzden kurba ve kurptan düze;
- sağa dönen;
- R 5000/L 500 ve R 30/L 10;
- iki ove (daralan, genişleyen);
- S klotoid;
- neredeyse daire (R 1000 → 1000,1, c = 2·10⁻⁹; Fresnel'in çift duyarlıkta zorlandığı yer);
- 10 rad'lık saç tokası;
- L 3000;
- θ0 = 100 rad.

`crates/shared/road/tests/all/clothoid_integrals.rs` her değerde 10⁻¹²·max(1, s) m'yi ister. En büyük fark
**2,5·10⁻¹³ m**'dir (R 2000/L 3000, s = 1800 m). Bu yuvarlamadır: kesme sınırı orada 1,8·10⁻¹⁷ m'dir.

ADR 0901 §1'in yöntemi değişmedi; yalnız sınırı ve ölçümü yazıldı.

## Komut ve pencere

Bir komutun girdisi ya da çıktısı değişmedi; ekranda görünen bir şey yoktur. Sınır, aks raporunun her sayısında
geçerlidir: konum, `residual`, birleşim farkları (ADR 0967).
