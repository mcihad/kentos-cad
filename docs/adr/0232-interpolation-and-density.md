# ADR 0232: İnterpolasyon ve yoğunluk

- **Durum:** kabul edildi (2026-10-09). Kapsam sahibin sözleridir (9 Ekim): “yeni bir branch açarak GIS-31 ve GIS-36 aralığını
  yapacağız”; “yüksek performans ilk önceliğimiz”; MADDE-TARIFI.md'nin sırası. Madde tek parçada biter. Dal `gis-31-36-raster-analysis`
  (ADR numaraları 0231–0236 bu aralık için ayrıldı). İki platformda.
- **Bağlam belgesi:** TODOS.md `GIS-32`, ADR 0231 (raster çözümleme altyapısı: çıktının karolu GeoTIFF'i, `Out`, `par`, sonucun yeri,
  yeni katmanın konması, Raster sekmesi), ADR 0204 (raster nesnesi, afin, rampalar), ADR 0142 (köşe kotu), ADR 0200 §4 (sayıların
  kuralı `kentos.statistics/1`), ADR 0149 (gösterim kuralı), ADR 0084 ve 0200 (İşlemler araçları, tablo çıktısı).

## Bağlam

Harita mühendisi sahada nokta ölçer; yüzeyi o noktalardan kurar: kotlu noktalardan sayısal yükseklik modeli, kuyu ölçümlerinden yer
altı suyu yüzeyi, örnek noktalardan kirlilik haritası. Netcad (Yoğunluk Analizi; NETSURF'ün üçgenlemesi), ArcGIS (IDW, Kriging, Natural
Neighbor, Spline, Kernel Density, Line Density) ve QGIS (IDW ve TIN interpolasyonu, Heatmap, Line Density, GDAL Grid) bunları noktadan
rastere araçlar olarak verir. KentOS'ta raster çözümleme altyapısı var (ADR 0231) ama girdisi rasterdir; noktadan yüzey ve yoğunluk
yoktur. Bu ADR noktalardan (ve çizgilerden) yeni bir 32 bit raster üreten yedi aracı ve onların ortak parçalarını (noktaların toplanması,
ızgara, Delaunay üçgenlemesi, komşu arama, çapraz doğrulama) tanımlar.

## Karar

### 1. Kapsam

İşlemler'in iki yeni kategorisinde yedi araç:

| Araç | Kimlik | Kategori | Çıktı |
|---|---|---|---|
| Ters uzaklık (IDW) | `interpolation.idw` | İnterpolasyon | 32 bit raster |
| Doğal komşu | `interpolation.naturalNeighbor` | İnterpolasyon | 32 bit raster |
| Spline | `interpolation.spline` | İnterpolasyon | 32 bit raster |
| Kriging | `interpolation.kriging` | İnterpolasyon | 32 bit raster; isteğe bağlı ikinci bant standart hata |
| TIN'den raster | `interpolation.tin` | İnterpolasyon | 32 bit raster |
| Çekirdek yoğunluğu | `density.kernel` | Yoğunluk | 32 bit raster |
| Çizgi yoğunluğu | `density.line` | Yoğunluk | 32 bit raster |

Beş interpolasyon aracında **Çapraz doğrulama** (birini dışarıda bırak) seçeneği ve tablosu; Kriging'de **hata yüzeyi** (standart hata).

### 2. Girdi noktaları

- **İnterpolasyonun noktaları:** Noktalar parametresindeki nokta (çok noktalı nesnenin bütün noktaları), çizgi, çoklu çizgi ve alan
  nesnelerinin köşeleri (alanın delikleri ve parçaları dahil; yayların ortası alınmaz, yalnız köşeler). Nesneler girdinin sırasıyla,
  köşeler nesnenin kendi sırasıyla (dış halka, delikleri, sonraki parçalar ve onların delikleri) alınır.
- **Değer:** Değer alanı boşsa her köşenin kendi kotu (ADR 0142: noktanın `z`'si, çizginin `za`, `zb`'si, yolun `zs`'i); kotu olmayan
  köşe alınmaz ve sayılır. Değer alanı seçiliyse nesnenin o alandaki değeri `kentos.statistics/1`'in kuralıyla (ADR 0200 §4) okunur ve
  bütün köşelerine verilir; okunamayan nesne alınmaz ve sayılır. Yazı, ölçü, blok, raster gibi öbür türler parametreye girmez.
- **Çakışan noktalar:** x ve y'si aynı (eşit float64) noktalar tek nokta olur; değeri değerlerinin girdinin sırasıyla toplamının sayısına
  bölümüdür, sırası (kimliği) ilkinin sırasıdır. Birleşen nokta sayısı söylenir. Üçgenleme ve Kriging'in denklemleri çakışan noktayı
  kaldıramaz; IDW'de de sonuç aynıdır.
- **Sınırlar:** en çok 10 milyon nokta (birleşmeden önce); en az bir değerli nokta (TIN'den raster ve Doğal komşu'da doğru üzerinde
  olmayan en az üç).
- **Yoğunluğun girdileri:** Çekirdek yoğunluğu'nda noktalar (nokta nesneleri ve çok noktalı nesnenin noktaları); çakışanlar birleşmez,
  her biri sayılır. Çizgi yoğunluğu'nda çizgi, çoklu çizgi, yay, daire, elips, eğri ve alanın sınırları (delikleri ve parçaları dahil):
  doğru parçaları ve yaylar (yaylı kenar, yay, daire) olduğu gibi, elips ve eğri 0,1 mm içinde doğru parçalarıyla (ADR 0149'un hesap
  kuralı, geometri çekirdeğinin `entity_edges`'i). Ağırlık alanı seçiliyse her nesnenin ağırlığı o alandan aynı kuralla okunur; okunamayan
  ya da eksi ağırlıklı nesne alınmaz ve sayılır; seçilmemişse ağırlık 1'dir.

### 3. Izgara

- **Hücre merkezi:** (i + ½, j + ½) afinle (ADR 0204 §2: x = x₀ + a·u + b·v, y = y₀ + c·u + d·v). Değer hücre merkezinde hesaplanır.
- **Noktaların kutusu** (varsayılan Kapsam): noktaların en küçük ve en büyük x, y'si; yoğunlukta her yöne yarıçap kadar büyütülür
  (çekirdekler kesilmesin). Hücre boyu s verilmişse o, değilse (0) **otomatik**: kutunun kısa kenarı / 250 (kısa kenar 0 ise uzun
  kenarı / 250; ikisi de 0 ise ret), aşağıya doğru en yakın {1, 2, 2,5, 5} × 10ᵏ değerine yuvarlanır (ArcGIS'in kuralı, yuvarlanarak:
  aynı boyla yapılan iki çözümleme aynı ızgaraya oturur). Izgara s'nin katlarına oturur: x₀ = ⌊xmin / s⌋·s, üst kenar
  y₁ = ⌈ymax / s⌉·s, genişlik = max(1, ⌈(xmax − x₀) / s⌉), yükseklik = max(1, ⌈(y₁ − ymin) / s⌉); afin [x₀, s, 0, y₁, 0, −s].
  Bölmeler float64'te, bu sırayla (iki platform ve başvuru aynı sonucu bulur).
- **Rasterin ızgarası:** seçilen raster nesnesinin afini, genişliği ve yüksekliği aynen (dönük ve eğik de); iki sonuç hücre hücre üst
  üste gelir (GIS-33'ün harita cebiri için).
- **Sınırlar:** ADR 0231 §2'ninkiler (genişlik en çok 65 536, hücre sayısı en çok 2³¹). Sistem: projenin EPSG'si (sistemi varsa).
- Değeri olmayan hücre (kapsamın dışı, yetmeyen komşu, çözümsüz denklem) NaN'dır (ADR 0231'in 32 bit nodata'sı).

### 4. Delaunay üçgenlemesi

TIN'den raster ve Doğal komşu'nun temeli (CIVIL-02'nin TIN'i de kullanacak): geometri çekirdeğinde `geom::delaunay`.

- **Yöntem:** süpürme kabuğu (Sinclair 2010'un s-hull'u, Delaunator'ın yolu): kutunun ortasına en yakın nokta, ona en yakın nokta ve
  çevrel çemberi en küçük üçüncü nokta tohum üçgeni; öbür noktalar tohumun çevrel merkezine uzaklıklarının artan sırasıyla (eşitlikte
  girdinin sırası) dışbükey kabuğa eklenir, görünen kabuk kenarlarına üçgen bağlanır ve kenarlar çevrel çember sınamasıyla çevrilir.
- **Kesin sınamalar:** yön `orient2d` (Shewchuk, var olan) ve çevrel çember `incircle` (yeni: önce hata sınırlı float64, sınır işareti
  kanıtlamazsa tam genişleme aritmetiği). Eş çemberli noktalarda (düzgün ızgara) çevirme yapılmaz: üçgenleme tek değildir, KentOS'unki
  girdinin sırasına bağlıdır ve iki platformda aynıdır.
- **Retler:** üçten az farklı nokta ya da hepsi bir doğru üzerinde: “Üçgenleme için doğru üzerinde olmayan en az üç nokta gerekir.”
- **Nokta bulma:** görünürlük yürüyüşü (Delaunay'da her zaman biter): üçgenin kenarlarından noktanın sağında kaldığı ilkinin komşusuna
  geçilir; kabuk kenarının sağı kabuğun dışıdır. Satırın hücreleri bir öncekinin üçgeninden yürür.

### 5. Ters uzaklık (IDW)

- z(q) = Σ wᵢ zᵢ / Σ wᵢ, wᵢ = 1 / dᵢ^p (dᵢ² = Δx² + Δy², dᵢ^p = (dᵢ²)^(p/2)); toplamlar komşuların uzaklık sırasıyla.
- **Komşular:** en yakın k nokta (Nokta sayısı, varsayılan 12, 1–64), Arama yarıçapı R > 0 ise yalnız dᵢ ≤ R olanlar (0: sınırsız);
  sıralama (dᵢ², sıra) ile: eşit uzaklıkta önce sırası küçük olan. En az nokta m'den (varsayılan 1) az komşu varsa hücre NaN.
- Bir komşu q'nun tam üstündeyse (dᵢ = 0) değer onunkidir.
- Üs p varsayılan 2 (0,1–10). GDAL'ın `invdistnn`'iyle aynı tanım.

### 6. Doğal komşu

- **Sibson koordinatları** (Sibson 1981): q noktalara eklenince Voronoi hücresinin her komşudan aldığı alanın hücrenin alanına oranı
  λᵢ; z(q) = Σ λᵢ zᵢ.
- **Hesap** (Watson 1992): q'yu içeren üçgenden başlayıp çevrel çemberi q'yu içeren (kesin `incircle`) üçgenler bulunur (boşluk); boşluğun
  sınır köşeleri v₀ … v_{m−1} (q çevresinde saat yönünün tersine) doğal komşulardır. vᵢ'nin payı, gᵢ = çevrel merkez(q, vᵢ, vᵢ₊₁),
  boşluğun vᵢ'ye değen üçgenlerinin çevrel merkezleri (vᵢ çevresinde sırayla) ve gᵢ₋₁'in oluşturduğu çokgenin alanıdır (ayakkabı bağı
  formülü). Merkezler q'ya göre bağıl koordinatlarla hesaplanır.
- **Kapsam:** q dışbükey kabuğun dışındaysa NaN (Sibson tanımsız). Kabuk kenarının üstündeyse kenarın iki ucu arasında doğrusal (Sibson'ın
  sınırdaki limiti), bir noktanın üstündeyse onun değeri.

### 7. TIN'den raster

- q'yu içeren Delaunay üçgeninde doğrusal: z(q) = λₐzₐ + λ_b z_b + λ_c z_c, λₐ = orient(q, b, c) / orient(a, b, c) (öbürleri benzer;
  `orient` a'ya göre bağıl koordinatlarla düz float64). Kabuğun dışı NaN. GDAL'ın `gdal_grid -a linear`'ıyla aynı tanım.

### 8. Spline

ArcGIS'in Spline'ı (Mitas ve Mitasova 1988), her hücrede en yakın k noktayla (Nokta sayısı, varsayılan 12, 3–64; §5'in sıralaması):

- z(q) = T(q) + Σ λⱼ R(rⱼ); λ'lar komşuların değerlerinden (denklem: Σⱼ λⱼ R(|pᵢ − pⱼ|) + T(pᵢ) = zᵢ ve T'nin kısıtları).
- **Düzenlemeli** (varsayılan): T = a₁ + a₂x + a₃y, kısıtlar Σλ = Σλx = Σλy = 0;
  R(r) = (1/2π)·[(r²/4)(ln(r/2τ) + c − 1) + τ²(K₀(r/τ) + c + ln(r/2τ))], τ² = Ağırlık (varsayılan 0,1), c = 0,577215664901532…
  (Euler sabiti), K₀ değiştirilmiş Bessel işlevi; R(0) = 0. (ArcGIS'in belgesinde son terim ln(r/2π)'dir; fark R'ye eklenen bir
  sabittir ve Σλ = 0 kısıtı onu yok eder: yüzey aynıdır.)
- **Gerilimli:** T = a₁, kısıt Σλ = 0; R(r) = −(1/(2πφ²))·[ln(rφ/2) + c + K₀(rφ)], φ² = Ağırlık (varsayılan 0,1); R(0) = 0.
- **Ağırlık:** düzenlemelide 0 ile 5 arası (0: ince plaka), gerilimlide 0'dan büyük, en çok 100.
- **Denklem:** koordinatlar komşuların ilkine (sırası en küçük) göre bağıl; kısmi pivotlu LU. Pivot, matrisin en büyük öğesinin 10⁻¹³'ünden
  küçükse (doğru üzerindeki komşular) hücre NaN. Aynı komşu kümesi için λ'lar bir kez çözülür (iş parçacığı başına önbellek).
- **K₀:** x ≤ 2'de seri (K₀ + γ + ln(x/2) biçiminde, sıfır yakınında basamak yitirmeden); x > 2'de e^x·√x·K₀(x)'in u = 4/x − 1'deki 30
  terimli Chebyshev açılımı (Cephes'in yolu; katsayılar mpmath'in `besselk`'iyle 60 basamakta 96 düğümden, atılanlar 10⁻²⁰'nin altında:
  `scripts/fixtures/bessel_k0.py`), Clenshaw'la; x > 700'de 0. x > 40'ta K₀(x) γ'nın ulp'unun yarısından küçüktür: eklenmez, bitler
  aynıdır. Başvuru mpmath'in `besselk`'idir.

### 9. Kriging

Sıradan kriging, her hücrede en yakın k nokta (varsayılan 12, 1–64) ve isteğe bağlı Arama yarıçapı ile:

- **Variogram** γ(h): h = 0'da 0; h > 0'da c₀ + s·f(h/a) (c₀ külçe, s kısmi eşik, a erim):
  Küresel f(u) = 1,5u − 0,5u³ (u < 1), 1 (u ≥ 1); Üstel f(u) = 1 − e^(−3u); Gauss f(u) = 1 − e^(−3u²) (pratik erim).
- **Denklem:** [Γ 1; 1ᵀ 0][λ; μ] = [γ₀; 1], Γᵢⱼ = γ(|pᵢ − pⱼ|), γ₀ᵢ = γ(|pᵢ − q|); ẑ = Σ λᵢ zᵢ; varyans σ² = Σ λᵢ γ₀ᵢ + μ; standart hata
  √max(σ², 0). q bir komşunun tam üstündeyse ẑ onun değeri, σ = 0. Aynı komşu kümesinin LU'su bir kez çözülür; hücre başına yalnız sağ
  taraf. §8'in pivot kuralı.
- **Otomatik variogram** (varsayılan): noktalar 4 000'den çoksa her ⌈n / 4 000⌉. nokta (sırayla) alınır. Ampirik variogram: en büyük
  uzaklık L = kutunun köşegeni / 2, Aralık sayısı N (varsayılan 12) eşit aralık w = L / N; i < j sırasıyla her çift için hᵢⱼ ≤ L ise
  k = min(⌊h / w⌋, N − 1). aralığa sayı, Σh ve Σ(zᵢ − zⱼ)²/2 eklenir; boş olmayan aralıkların h̄ₖ ve γ̂ₖ'sı. Uydurma: Σ nₖ(γ̂ₖ − c₀ −
  s·f(h̄ₖ/a))²'yi c₀ ≥ 0, s ≥ 0, a > 0 ile en küçük yapan değerler: a sabitken c₀ ve s doğrusal en küçük kareler (biri eksi çıkarsa
  sınırdaki iki çözümün iyisi); a, w/2 ile 2L arasında logaritmik eşit aralıklı 200 adayın en iyisinin iki komşusu arasında 100 adım
  altın oran aramasıyla. Üçten az dolu aralık: ret (“elle girin”). Bulunan değerler özette söylenir.
- **Elle:** Model, Külçe, Kısmi eşik (> 0), Erim (> 0).
- **Hata yüzeyi** (isteğe bağlı): sonuç iki bantlı olur (1. bant tahmin, 2. bant standart hata) ve iki raster nesnesi (iki katman: Çıktı
  katmanı ve Hata katmanı) aynı dosyayı gösterir, her biri kendi bandıyla.

### 10. Çekirdek yoğunluğu

- Hücrede D(q) = ölçek · Σᵢ wᵢ K(dᵢ / r) / r², dᵢ < r olan noktalar üstünden (dᵢ = r: 0). Çekirdekler düzlemde integralleri 1 olacak
  biçimde: Dörtlü (varsayılan; ArcGIS ve QGIS'in) K = (3/π)(1 − u²)², Üçgen (3/π)(1 − u), Düzgün 1/π, Epanechnikov (2/π)(1 − u²),
  Üçlü ağırlık (4/π)(1 − u²)³. Yoğunluğun ızgaradaki toplamı ağırlıkların toplamına yaklaşır.
- **Birim:** km² başına (varsayılan, ölçek 10⁶), hektar başına (10⁴), dönüm başına (10³), m² başına (1).
- **Yarıçap:** verilmişse o; 0 ise ArcGIS'in Silverman kuralı: ağırlıklı ortalama merkez, standart uzaklık SD = √(Σ w·|p − p̄|² / Σ w),
  ağırlıklı ortanca uzaklık Dm (uzaklıklar artan sırada, birikmiş ağırlığın toplamın yarısına ulaştığı ilk uzaklık), n = Σ w;
  r = 0,9 · min(SD, √(1/ln 2) · Dm) · n^(−0,2). r 0 çıkarsa ret.
- Görünüşte 0 değerli hücreler boş (çizimin altı görünsün): raster nesnesinin nodata'sı 0.

### 11. Çizgi yoğunluğu

- Hücrede L(q) = ölçek · Σ wₗ · |çizgiₗ ∩ daire(q, r)| / (π r²) (ArcGIS'in Line Density'si): çizginin r yarıçaplı dairenin içinde
  kalan uzunluğu, kapalı biçimde: doğru parçasında ikinci dereceden denklemin [0, 1]'e kırpılmış kökleri, yayda yayın çemberinin dairenin
  içindeki açı aralığı (cos(θ − φ) ≥ (R² + d² − r²) / (2Rd)) ile yayın kendi aralığının kesişimi.
- **Birim:** km/km² (varsayılan, ölçek 1 000), m/ha (10⁴), m/m² (1). **Yarıçap:** 0 ise çizgilerin kutusunun kısa kenarı / 30 (ArcGIS'in
  varsayılanı). Kapsam çizgilerin kutusu yarıçap kadar büyütülerek. 0 değerli hücreler görünüşte boş.

### 12. Çapraz doğrulama

- Her nokta sırayla dışarıda bırakılır ve öbürlerinden aynı ayarlarla tahmin edilir. IDW, Spline ve Kriging'de komşular öbür noktalardan;
  TIN'den raster ve Doğal komşu'da noktanın Delaunay komşularının oluşturduğu yıldız çokgeni yeniden üçgenlenir (noktasız
  üçgenlemenin o bölgesi); kabuk üstündeki noktanın tahmini yoktur (sayılır, tabloda boş).
- **Tablo** (İşlemler'in tablo çıktısı, ADR 0200 §7): Sıra, Ad (noktanın etiketi), Y ve X (CAD projesinde X ve Y), Ölçülen, Tahmin,
  Fark (tahmin − ölçülen); Kriging'de Standart hata ve Standart fark (fark / standart hata). Sayılar gösterim kuralıyla, değerler 3
  basamak.
- **Özet:** ortalama fark, karesel ortalama hata (KOH), ortalama mutlak fark; Kriging'de standart farkların ortalaması ve KOH'u (iyi bir
  modelde 0 ve 1'e yakın).

### 13. Arayüz

- **Araçlar** iki platformda aynı adlar, parametreler ve varsayılanlarla:
  - İnterpolasyon: Noktalar (Katman, Seçili, Görünen, Hepsi; nokta, çizgi, çoklu çizgi, alan), Değer alanı (boş: kot); yöntemin
    parametreleri; Hücre boyu (0: otomatik), Kapsam (Noktaların kutusu, Rasterin ızgarası) ve Izgara rasteri; Çapraz doğrulama; Çıktı
    dosyası (boşsa girdinin katmanının adı ve aracın eki: `-idw`, `-dogalkomsu`, `-spline`, `-kriging`, `-tin`), Çizime ekle, Çıktı
    katmanı.
  - IDW: Üs (2), Nokta sayısı (12), Arama yarıçapı (0), En az nokta (1). Spline: Tür (Düzenlemeli, Gerilimli), Ağırlık (0,1), Nokta
    sayısı (12). Kriging: Model (Küresel, Üstel, Gauss), Variogram (Otomatik, Elle), Aralık sayısı (12), Külçe (0), Kısmi eşik (1), Erim
    (100 m), Nokta sayısı (12), Arama yarıçapı (0), Hata yüzeyi ve Hata katmanı.
  - Çekirdek yoğunluğu: Noktalar, Ağırlık alanı, Yarıçap (0: otomatik), Çekirdek, Birim, Hücre boyu, Kapsam; ek `-yogunluk`.
    Çizgi yoğunluğu: Çizgiler, Ağırlık alanı, Yarıçap (0: otomatik), Birim, Hücre boyu, Kapsam; ek `-cizgiyogunlugu`.
- **Sonuç** ADR 0231 §2'deki gibi (masaüstünde yol ya da girdinin katmanının adıyla çizimin klasörü, web'de gömülü ya da oturumun
  dosyası). Yeni katman girdinin katmanının **hemen altına** konur (noktalar yüzeyin üstünde görünsün): katman parametresinin `below`'u;
  aynı katmanın altına giden birden çok yeni katman çalıştırmanın sırasını korur, her biri öncekinin altına (Kriging'in hata katmanı
  tahminin katmanının altında).
- **Görünüşler:** kotla interpolasyon Arazi rampası, alanla ve hata bandı Viridis, yoğunluklar Sıcaklık; en küçük–en büyük gerdirme.
- **Komutlar ve şerit:** `processing.run.interpolation.<ad>`, `processing.run.density.<ad>`; CBS'nin Raster sekmesinde İnterpolasyon ve
  Yoğunluk panelleri (ADR 0231 §10'un `RASTER_ANALYSIS`'i).
- **Takma adlar:** IDW, TERSUZAKLIK, TERSMESAFE; DOGALKOMSU, SIBSON; SPLINEYUZEY, SPLINEINTERPOLASYON (SPLINE Eğri'nindir); KRIGING; TINRASTER, TINDENRASTER; YOGUNLUK,
  CEKIRDEKYOGUNLUGU, ISIHARITASI, HEATMAP; CIZGIYOGUNLUGU.
- **İlerleme ve Durdur:** masaüstünde İşlemler'in iş parçacığı, web'de çözümleme işçisi (ADR 0231 §11); çapraz doğrulama da orada.

### 14. Performans

- Satırlar 256'lık şeritlerle ve iş parçacıklarında; noktalar düzgün kova ızgarasında (kova başına ortalama iki nokta), en yakın k
  komşu halkalar genişletilerek; Delaunay O(n log n); Spline ve Kriging'de komşu kümesinin çözümü önbellekte; yoğunluk noktaları
  y'ye göre sıralanarak satırın şeridine düşenlerle.
- Web'de noktalar işçiye tipli dizilerle değil nesnelerin JSON'uyla (yalnız geometrisi ve değerin metni) gider: noktaların toplanması
  çekirdektedir (tek kaynak).
- **Bütçeler** (release, geliştirme makinesi; 100 000 rastgele noktadan 2048 × 2048 ızgara): masaüstünde IDW ≤ 1 s, TIN'den raster ≤ 0,5 s,
  Doğal komşu ≤ 3 s, Spline ≤ 2 s, Kriging ≤ 2 s, Çekirdek yoğunluğu (yarıçap 20 hücre) ≤ 1 s; 1 milyon noktanın Delaunay'ı ≤ 1,5 s; web'de
  masaüstünün tek iş parçacıklı süresinin en çok 3 katı.

## Kapsam dışı

Evrensel ve eş kriging, yönlü (anizotrop) variogram, kırık çizgili (bariyerli) interpolasyon, Topo to Raster (dere ağıyla), çizgilerin
çekirdek yoğunluğu, kısıtlı Delaunay (kırık hatlarla; CIVIL-02), TIN'in kendisinin nesne olarak yazılması (CIVIL-02).

## Uygulama

- **Geometri çekirdeği:** `predicates::incircle` (Shewchuk'un ilk aşama hata sınırı; sınır yetmezse yığında genişlemelerle tam
  determinant: `exp_diff`, `exp_scaled`, `exp_mul`, `exp_sum`), `geom::delaunay` (`triangulate`: süpürme kabuğu, kesin sınamalar,
  `legalize`'ın sınırsız yığını; `Delaunay::locate` görünürlük yürüyüşü, `out_edges`, `star`; `fill_star` yıldız deliğinin Delaunay
  kulaklarıyla yeniden doldurulması; `circumcenter`).
- **Raster çekirdeği** `kentos-raster`: `points` (`Source`: noktanın, çizginin, yolun yeri ve kotları, web'in JSON'undan serde'yle ya da
  sözleşmenin `Entity`'sinden; `gather`, `gather_weighted`, `gather_lines`; sayılar `ops::statistics::read_number`), `grid` (`Grid::of_box`,
  `Grid::of`, `nice_cell`), `index` (kova ızgarası: `Index::nearest` halka halka kesin, `within`; `EdgeIndex`), `solve` (kısmi pivotlu LU),
  `interp` (`Prepared`: IDW, TIN, Doğal komşu, Spline, Kriging, `at`, `leave_out`; iş parçacığı başına `Scratch` ve komşu kümesinin
  çözümleri karma tabloda; `natural`: boşluk ve Watson'ın yelpaze çokgenleri; `spline`: K₀, `Basis`, `fit`; `kriging`: modeller, `bins`,
  `best_at`, `fit`, `system`, `predict`), `density` (`KernelDensity`, `silverman`, `LineDensity`, `inside`), `from_points` (`PointSpec`,
  `PointTool`, `PointInput`, `PointJob`: şerit şerit, çapraz doğrulama 4 096'lık parçalarla, `Notes`, `CrossRow`, `CrossSummary`, `style`).
- **WASM** `raster-wasm`'ın `PointAnalysis`'i (nesnelerin JSON'u, değer metinleri, ayarlar; şeritler, GeoTIFF'in kuyruğu ve başı, ızgara,
  bantların görünüşü, notlar, çapraz doğrulama tipli dizilerle).
- **İşlemler (Rust)** `builtin::interpolation` (`mod.rs`: parametreler, `run_points`, raster nesneleri, özet ve uyarılar, çapraz doğrulama
  tablosu; `tools.rs`: yedi araç), kategoriler `interpolation` ve `density`. Katman parametresinin `below`'u, çalıştırıcının yerleştirmesi
  (aynı katmanın altına gidenler sırayla), `Beside::named` (girdinin katmanının adı). Ortak durumlar `fixtures/processing/v1/interpolation.json`
  ve `.kcad` (`interpolation_processing_cases.py`; anahtarlar `layerBelow`, `interpolationOf`), oynatıcı `tests/cases/surface.rs`'in
  `raster_cases`'i.
- **Masaüstü:** `catalog.rs`'in `PORTED`'ı, `ported.json`; testler `processing/interpolation_tests.rs`, resimler
  `interpolation_scenes.rs` (`tools_screens`'in `interp-*`, `yogunluk-cizim`, `cizgi-yogunlugu-cizim`).
- **Web:** `io/rasterAnalysisProtocol.ts` (`PointRequest`, `PointResult`), işçinin `points` işi, `io/rasterAnalysis.ts`'in
  `analyzePoints`'i, ev sahibinin `analyzePoints`'i (`processing/rasterHost.ts`, `app/rasterAnalysis.ts`); araçlar
  `processing/builtin/interpolation/shared.ts` ve `tools.ts`; `LayerParam.below` ve çalıştırıcının yerleştirmesi (`processing/runner.ts`);
  RunContext'in `project`'i (SRID ve tür: tablonun eksenleri); kategoriler; şeridin `RASTER_ANALYSIS`'i İnterpolasyon ve Yoğunluk'la;
  ikonlar. Testler `processing/cases.test.ts`'in interpolasyon bloğu, `io/raster.wasm.test.ts`'in 21 interpolasyon durumu,
  `processing/surfaceTesting.ts`'in `analyzePointsHere`'i; ölçüm `scripts/perf/raster.mjs`; resimler `shots.mjs`'in `interpolation` grubu.
- **İkonlar** (sorulmadan seçildi): `idw` (merkeze uzaklıkla soluklaşan bağlar), `naturalNeighbor` (Voronoi hücresi; İnterpolasyon
  kategorisinin değil, onun ikonu `idw`), `splineSurface` (noktalardan geçen yumuşak eğriler; `spline` Eğri aracınındır), `kriging`
  (platoya oturan variogram ve ampirik noktalar), `tinRaster` (ızgaranın üstünde üçgenler), `kernelDensity` (ısı lekeleri; Yoğunluk
  kategorisinin de), `lineDensity` (çizgiler ve arama dairesi).

## Doğrulama

- **Bağımsız başvurular** (KentOS kodu yok):
  - `scripts/fixtures/delaunay_cases.py --check`: qhull'un (matplotlib) dört genel konumlu kümedeki (rastgele, TM'de 300, kümeler, şerit)
    üçgenleri, her biri Python'un `Fraction`'larıyla tam boş çember denetiminden geçmiş; dört eş çemberli ya da yinelenen küme (düzgün ızgara,
    çember, kabukta doğrusal noktalar, eşit noktalar) üçgen ve kabuk sayılarıyla; üç ret.
  - `scripts/fixtures/interpolation_cases.py --check`: noktaların toplanması, ızgaranın kuralı, IDW (mpmath; GDAL'ın `invdistnn`'iyle çapraz
    denetim 10⁻⁹), TIN (qhull üçgenlerinde tam kesirli ağırlıklar; GDAL'ın `linear`'ıyla 10⁻⁹ ve kabuğun dışı), Doğal komşu (Voronoi
    hücrelerinin kesirlerle tam kırpılması), Spline ve Kriging (40 basamaklı mpmath, K₀ mpmath'ten), variogram uydurması (4 000 adaylı sık
    arama ve mpmath'le altın oran), Çekirdek ve Çizgi yoğunluğu (mpmath), her yöntemin çapraz doğrulaması (TIN ve Doğal komşu noktasız
    kümenin qhull'uyla); 40 durum: 21 yüzey ve yoğunluk, 4 variogram uydurması, 13 ızgara kuralı, 2 nokta toplama.
  - `scripts/fixtures/bessel_k0.py --check`: K₀'ın Chebyshev katsayıları mpmath'ten.
  - `scripts/fixtures/interpolation_processing_cases.py --check`: İşlemler'in 16 ortak durumu (12 çalıştırma, 4 ret); yazılan dosyalar
    `interpolationOf` ile başvuruya bağlı, yeni katmanlar `layerBelow`.
- **Sınırlar** (hücrenin değeri, max(1, |başvuru|)'ya göre): ölçülen en büyük sapmalar IDW 5,6·10⁻¹⁶, TIN 4,9·10⁻¹⁶, Doğal komşu 2,9·10⁻¹⁶,
  Spline 1,3·10⁻¹⁵, Kriging 7,3·10⁻¹⁶, uydurulmuş variogramla Kriging 5·10⁻¹⁰ (erim 10⁻⁹ kadar bulunur), çekirdekler 5,2·10⁻¹⁴, çizgiler
  2,1·10⁻¹³ (yayın daireye neredeyse teğet olduğu yerde acos basamak yitirir). Testlerin sınırları 10⁻¹³; uydurulmuş variogramda 10⁻⁸,
  çekirdekte 10⁻¹², çizgide 10⁻¹¹. Yazılan 32 bit dosya başvurunun 32 bite yuvarlanmışından en çok 1 ulp; bir ve dört iş parçacığı aynı
  baytları yazar.
- **Testler:** geometri çekirdeğinde `incircle` 3 (tam tamsayı aritmetiğiyle 10⁵ rastgele ve eş çemberli dörtlü, TM koordinatlarında ulp
  ulp), Delaunay 5 (fixture'lar, rastgele ve dejenere kümelerde kurallar, nokta bulma, yıldız ve yeniden doldurma); `kentos-raster` 10
  interpolasyon testi ve 8 birim testi (kova ızgarası kaba kuvvetle, ızgara, LU, K₀ ve spline, yoğunluğun kesin uzunlukları);
  `kentos-processing` ortak durumların 16'sı ve varsayılanlar; masaüstü `interpolation_tests` 3 (pencereden arka planda: yer, tablo, katman
  sırası, iki bantlı Kriging, yoğunluğun boş sıfırları), araç sayısı 44; web `cases.test.ts`'in 17'si, `raster.wasm.test.ts` 71,
  `workspaces.test.ts` (Raster sekmesinin dört paneli), pencere formları (`dialog.json`'a yedi form eklendi, başka satır değişmedi).
- **Tarayıcıda uçtan uca:** `shots.mjs interpolation` her sahnede aracı İşlemler penceresinden çalıştırır: iş kendi işçisinde, sonuç gömülür
  ve noktaların altında çizilir (11 sahne, iki tema, iki boy).
- **Görsel incelemede düzeltilenler:** TIN ikonunun ızgarası 16 pikselde kalabalıktı (iki çizgiye indi); `SPLINE` takma adı Eğri'nindi.
- **Süreler** (9 Ekim 2026; Intel Core i5-13500, 20 iş parçacığı, Linux; 100 000 rastgele nokta 2 km'de, 2048 × 2048 ızgara (metrede
  bir hücre), katlarıyla yazılan çıktı dahil; masaüstünde üç koşunun ortancası, web'de iki koşu):

  | İş | Masaüstü (8) | Masaüstü (1) | Web (işçi, release WASM, 1) | Bütçe (masaüstü) |
  |---|---|---|---|---|
  | IDW (12 nokta) | 0,433 s | 2,098 s | 2,633 s | 1 s |
  | TIN'den raster | 0,134 s | | | 0,5 s |
  | Doğal komşu | 0,522 s | 2,428 s | 2,664 s | 3 s |
  | Spline (düzenlemeli, 12) | 1,930 s | | | 2 s |
  | Kriging (küresel, 12) | 1,047 s | | 6,594 s | 2 s |
  | Çekirdek yoğunluğu (20 m) | 0,317 s | | | 1 s |
  | Delaunay, 10⁶ nokta | 0,485 s | | | 1,5 s |

  Web'de IDW ve Doğal komşu masaüstünün tek iş parçacıklı süresinin 1,25 katı (bütçe 3 kat). Spline'ı bütçeye üç iyileştirme indirdi: K₀'ın
  yamuk toplamı yerine Chebyshev açılımı (30 s → 4,5 s), simetrik matris ve tek logaritma (2,4 s), komşu kümelerinin karma tablosu (1,93 s).
  Masaüstü: `cargo test --release -p kentos-raster --test all interpolation_timing -- --ignored --nocapture --test-threads=1`; web:
  `node scripts/perf/raster.mjs`.
- **Ölçülmeyenler:** p99 (az koşu); 10⁶'dan çok noktalı interpolasyon; web'de tek iş parçacıklı Spline; coğrafi (derece) sistemde
  mesafeler derecedir (metreye çevrilmez; kapsam dışı).
