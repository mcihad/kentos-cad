# ADR 0238: Mekânsal istatistik

- **Durum:** kabul edildi (2026-10-10). Kapsam sahibin sözleridir (10 Ekim): “GIS-37, 38, 42, 43 maddelerini tamamla”, “Yeni branch
  içinde yap bunları”; önceki kararları: “yüksek performans ilk önceliğimiz”, simgeler sorulmadan seçilir, iki platform. Madde tek
  parçada biter; dal `gis-37-38-42-43`.
- **Bağlam belgesi:** TODOS.md `GIS-38`, ADR 0200 (sayıların kuralı `kentos.statistics/1`, Özet istatistik'in grupları, İşlemler'in tablo
  çıktısı, nesnenin merkezi), ADR 0201 (geometri işlemlerinin araçları ve yeni katmanı), ADR 0237 (tablo veren aracın tablosu).

## Bağlam

Olayların, tesislerin ve değerlerin mekânda nasıl dağıldığı sorusu (suç ve kaza noktaları, hizmet alanları, emlak değerleri, salgın,
yangın, heyelan) mekânsal istatistiğin işidir: dağılımın merkezi ve yayılımı, rastgele mi kümelenmiş mi olduğu, değerlerin komşularına
benzeyip benzemediği, sıcak ve soğuk noktalar, kümeler. ArcGIS Pro'nun Spatial Statistics araç kutusunda Mean Center, Median Center,
Standard Distance, Directional Distribution, Average Nearest Neighbor, Spatial Autocorrelation (Global Moran's I), Hot Spot Analysis
(Getis-Ord Gi*) ve Density-based Clustering; QGIS'te Mean coordinate(s), Nearest neighbour analysis, DBSCAN clustering ve K-means
clustering vardır; Netcad'in Analist'inde lojistik regresyon. KentOS'ta bugün Özet istatistik ve Bilgi al vardır (ADR 0200): mekânsal
dağılımın ölçüleri yoktur.

Araştırmada bulunanlar (10 Ekim):
- **Merkezler ve yayılım** (Bachi 1963; Lefever 1926; Yuill 1971): ortalama merkez ağırlıklı ortalamadır; ortanca merkez uzaklıkların
  toplamını en küçük yapan noktadır (geometrik ortanca), Weiszfeld'in yinelemesiyle, bir veri noktasına düşerse Vardi ve Zhang'ın
  (2000) düzeltmesiyle bulunur; standart uzaklık merkezden uzaklıkların karelerinin ortalamasının köküdür; yön dağılımı elipsi
  koordinatların kovaryansının özvektörleridir; ArcGIS eksenleri √2 düzeltmesiyle yazar.
- **En yakın komşu** (Clark ve Evans 1954): gözlenen ortalama en yakın komşu uzaklığı, aynı yoğunlukta rastgele dağılımın beklenen
  0,5/√(n/A)'sıyla karşılaştırılır; standart hata 0,26136/√(n²/A); ArcGIS ve QGIS böyle hesaplar. QGIS alanı noktaların kutusundan alır.
- **Moran I** (Moran 1950; Cliff ve Ord 1981): değerlerin komşularıyla benzerliği; beklenen −1/(n−1); varyans rastgele dağıtım
  (randomization) varsayımıyla, basıklık b₂ ile; ArcGIS'in z-puanı budur.
- **Getis-Ord Gi\*** (Ord ve Getis 1995): bir nesnenin ve komşularının toplamının beklenenden sapması, kendisi bir z-puanı; nesne kendi
  komşusudur. ArcGIS sıcak ve soğuk noktaları p-değerine göre %90, %95, %99 güvenle sınıflar.
- **Komşuluk:** sabit uzaklık bandı, ters uzaklık (bant içinde), k en yakın komşu. ArcGIS'in varsayılan bandı her nesnenin en az bir
  komşusu olacak uzaklıktır.
- **Kümeleme:** DBSCAN (Ester ve arkadaşları 1996; çekirdek noktalar, sınır noktaları, gürültü), k-ortalamalar (Lloyd 1982).

## Karar

### 1. Kapsam

İşlemler'in yeni Mekânsal istatistik kategorisinde dokuz araç:

| Araç | Kimlik | Çıktı |
|---|---|---|
| Ortalama merkez | `stats.meanCenter` | nokta, grup başına |
| Ortanca merkez | `stats.medianCenter` | nokta, grup başına |
| Standart uzaklık | `stats.standardDistance` | daire, grup başına |
| Yön dağılımı | `stats.directionalDistribution` | elips, grup başına |
| En yakın komşu | `stats.nearestNeighbor` | tablo ve özet: oran, z, p, desen |
| Moran I (mekânsal otokorelasyon) | `stats.moransI` | tablo ve özet: I, beklenen, varyans, z, p, desen |
| Sıcak nokta (Gi\*) | `stats.hotSpot` | nesnelerin kopyaları z, p ve güven sınıfıyla, sınıfın rengiyle |
| DBSCAN kümeleme (yoğunluğa göre) | `stats.dbscan` | nesnelerin kopyaları küme numarasıyla, kümenin rengiyle |
| k-ortalamalar kümeleme | `stats.kMeans` | nesnelerin kopyaları küme numarasıyla, kümenin rengiyle |

**Kapsam dışı:** lojistik ve coğrafi ağırlıklı regresyon (Netcad'in lojistik regresyonu ayrı bir araç ailesidir), Anselin'in yerel Moran'ı
(LISA), en uygun sıcak nokta (bandın kendiliğinden aranması), yanlış keşif oranı (FDR) düzeltmesi, kenar (contiguity) komşuluğu,
uzay-zaman küpleri, HDBSCAN ve OPTICS, Ripley'in K'si, üç boyutlu merkezler, Gi\*'da ters uzaklık (nesnenin kendi ağırlığı tanımsız).

### 2. Ortak kurallar

- **Girdi:** alanlar, yollar ve noktalar (geometri işlemlerinin türleri, ADR 0201 §1); kapsamlar katman, seçim, görünen, hepsi.
- **Nesnenin yeri:** ifadelerin `$merkez_y`, `$merkez_x`'i (`shape_centroid`, ADR 0200 §1): noktanın kendisi (çok noktalının noktalarının
  ortalaması), alanın, dairenin ve bütün elipsin ağırlık merkezi, öbürlerinin dayanağı (çizginin ortası, yolun ortadaki köşesi). Yeri
  bulunamayan nesne alınmaz, sayısı söylenir.
- **Coğrafi proje:** koordinatlar derece olduğundan uzaklık ve alan metre değildir: dokuz araç da reddeder (“Mekânsal istatistik
  projeksiyonlu koordinat ister: projenin sistemi coğrafi.”).
- **Sayılar:** alanın değeri ADR 0200 §4'ün kuralıyla okunur (`kentos.statistics/1`'in ondalık sayısı) ve en yakın float64'e çevrilir;
  okunamayan ya da boş değerli nesne o hesaba alınmaz, sayısı söylenir. Ağırlık 0 ya da artı olmalıdır (eksi ağırlıklı nesne alınmaz,
  söylenir).
- **Grup** (merkezlerde, standart uzaklıkta ve yön dağılımında, isteğe bağlı): Özet istatistik'in kuralı (ADR 0200 §5): alanın metni,
  başındaki ve sonundaki boşluklar atılarak; gruplar adlarının doğal sırasıyla, boş grup “(boş)” en sonda.
- **Sıra ve kesinlik:** nesneler girdinin sırasıyla (belgedeki sırası). Koordinatlar ilk yerin farkı olarak (x − x₀, y − y₀) hesaplanır,
  sonuca eklenir; toplamlar bu sırayla float64'te.
- **Uzaklık:** d = √(Δx·Δx + Δy·Δy) (float64, `sqrt`'in doğru yuvarlaması); bant karşılaştırması d ≤ bant.
- **p-değeri:** iki yanlı, normal dağılımla: p = erfc(|z| / √2).
- **Desen** (En yakın komşu ve Moran I): p < 0,05 ise z'nin işaretine göre Kümelenmiş ya da Dağınık (En yakın komşu'da z < 0, Moran I'da
  z > 0 kümelenmiş), yoksa Rastgele.
- **Yazım:** öznitelik ve tablo değerleri metin, ondalık nokta; çekirdek yazar (Rust'ın biçimi: ikili değerin tam açılımı, yarımlar çifte),
  iki platform aynı metni gösterir.
- **Yeni katman:** sonuçlar aracın yeni katmanında (katman parametresi, ADR 0201'in yeni katmanı), tek adımda; yeni katman girdinin
  katmanının hemen üstündedir (ADR 0231'in `above`'u): sonuçlar geldikleri nesnelerin üstünde. Kopya yazan araçlarda kopya kaynağın
  geometrisiyle (kotlarıyla) ve öznitelikleriyle, yeni alanlar eklenerek; kaynağın kendi rengi, sembolü, etiketi ve kalınlığı kopyaya
  geçmez, rengi sınıfın ya da kümenin olur.
- **Görünüş:** merkezlerin katmanı noktalarını çarpıyla çizer (12 px). Sıcak noktalar ve Kümeler katmanı kategorili görünüşle gelir
  (§9, §10; lejantta sınıflar): katmanın düz görünüşünde dolgu katmanın rengidir, nesnenin rengi yalnız çizgisine geçer.
- **Başvuru:** bağımsız başvurunun 40 basamaklı hesabıyla yerler ve uzaklıklar 10⁻⁶ m, istatistikler (I, z, oranlar) 10⁻⁹ göreli,
  p-değerleri 10⁻¹² mutlak yakın olmalıdır; kümeler, sınıflar, sayılar ve yazılan metinler tam aynı olmalıdır.

### 3. Ortalama ve ortanca merkez

- **Girdi:** nesneler; Ağırlık alanı (isteğe bağlı); Grup alanı (isteğe bağlı).
- **Ortalama merkez:** (Σ wᵢxᵢ / Σ wᵢ, Σ wᵢyᵢ / Σ wᵢ); ağırlık alanı yoksa wᵢ = 1. Ağırlıklarının toplamı 0 olan grup yazılmaz, söylenir.
- **Ortanca merkez:** Σ wᵢ‖p − pᵢ‖'yi en küçük yapan p. Weiszfeld'in yinelemesi ortalama merkezden başlar:
  T(p) = (Σ wᵢpᵢ/dᵢ) / (Σ wᵢ/dᵢ), dᵢ = ‖p − pᵢ‖. p'ye 10⁻¹² m'den yakın veri noktaları (ağırlıklarının toplamı w₀) varsa Vardi ve Zhang'ın
  adımı: öbürleriyle R = Σ wᵢ(pᵢ − p)/dᵢ, r = ‖R‖; r ≤ w₀ ise p sonuçtur; değilse p ← (1 − w₀/r)·T + (w₀/r)·p (T öbürleriyle).
  Adım 10⁻¹⁰ m'den kısa olunca ya da 10 000 adımda durulur. Ağırlığı 0 olan nesne hesaba girmez.
- **Öznitelikler:** Grup (grup alanı varsa), Nesne sayısı (alınan nesne), Ağırlık toplamı (ağırlık alanı varsa; ağırlıkların kesin toplamı,
  ADR 0200 §4).
- **Katmanlar:** Ortalama merkez, Ortanca merkez.

### 4. Standart uzaklık

- **Girdi:** §3'ünkiler ve Standart sapma (1, 2 ya da 3; varsayılan 1).
- **Hesap:** merkez ortalama merkezdir; SD = √(Σ wᵢ((xᵢ − X̄)² + (yᵢ − Ȳ)²) / Σ wᵢ).
- **Sonuç:** merkezde, yarıçapı k·SD olan daire. Öznitelikler: §3'ünkiler, Standart uzaklık (SD, m, 3 ondalık), Kat (k). SD'si 0 olan grup
  (tek yer ya da çakışık yerler) yazılmaz, söylenir.
- **Katman:** Standart uzaklık.

### 5. Yön dağılımı (standart sapma elipsi)

- **Girdi:** §4'ünkiler.
- **Hesap:** x̃ = x − X̄, ỹ = y − Ȳ; sxx = Σ w x̃², syy = Σ w ỹ², sxy = Σ w x̃ỹ, W = Σ w; t = (sxx + syy)/2,
  h = √(((sxx − syy)/2)² + sxy²). Kovaryansın özdeğerleri λ₁ = (t + h)/W, λ₂ = (sxx·syy − sxy²)/(W²·λ₁). Büyük yarı eksen
  a = k·√2·√λ₁, küçük b = k·√2·√λ₂ (ArcGIS'in √2 düzeltmesi). Büyük eksenin doğrultusu φ = ½·atan2(2·sxy, sxx − syy) (x ekseninden saat
  yönünün tersine), (−π/2, π/2]'de (eksi sıfır sxy'nin −π/2'si π/2 sayılır); doğrultu yönün (cos φ, sin φ) atan2'siyle kuzeyden saat yönünde.
- **Sonuç:** elips: merkez (X̄, Ȳ), büyük eksen (a·cos φ, a·sin φ), oran b/a, 0 … 2π. Öznitelikler: §3'ünkiler, Büyük yarı eksen (m),
  Küçük yarı eksen (m) (3 ondalık), Doğrultu (büyük eksenin kuzeyden saat yönünde açısı, [0, 180) derece, 2 ondalık), Kat. λ₂ ≤ 0 olan
  grup (yerler bir doğru üzerinde ya da çakışık) yazılmaz, söylenir.
- **Katman:** Yön dağılımı.

### 6. En yakın komşu

- **Girdi:** nesneler (en az iki); Alan (m², isteğe bağlı; boşsa yerlerin eksenlere paralel kutusunun alanı).
- **Hesap:** dᵢ, yerin öbür yerlere en kısa uzaklığı (çakışık yer 0). D̄ₒ = Σ dᵢ / n; D̄ₑ = 0,5 / √(n / A); oran R = D̄ₒ / D̄ₑ;
  SE = 0,26136 / √(n² / A); z = (D̄ₒ − D̄ₑ) / SE. Alan 0 ise ret (“Yerlerin kutusunun alanı sıfır; Alan'ı yazın.”).
- **Sonuç:** tablo (Ölçü, Değer): Nesne sayısı, Gözlenen ortalama uzaklık (m), Beklenen ortalama uzaklık (m) (3 ondalık), En yakın komşu
  oranı, z (4 ondalık), p (6 ondalık), Alan (m², 2 ondalık), Desen. Özet: “En yakın komşu oranı 0.6213 (z -4.2100, p 0.000026):
  kümelenmiş.” Çıktılar: tablo, oran, z, p.

### 7. Komşuluk (Moran I ve Gi\*)

- **Kavram:** Sabit uzaklık bandı (wᵢⱼ = 1, dᵢⱼ ≤ bant), Ters uzaklık (yalnız Moran I'da; wᵢⱼ = 1 / max(dᵢⱼ, 1 m), dᵢⱼ ≤ bant: 1 m'den
  yakın komşu, çakışık yerler de, 1 m'de sayılır), k en yakın komşu (wᵢⱼ = 1, j yerin en yakın k komşusundan; (d, sıra) küçük olan önce).
- **Bant:** yazılmazsa en büyük en yakın komşu uzaklığı (her nesnenin en az bir komşusu olur; ArcGIS'in varsayılanı). k varsayılan 8; k
  nesne sayısının bir eksiğinden büyükse öbürlerinin hepsi.
- **Moran I'da** wᵢᵢ = 0; Satır standartlaştırma (varsayılan açık) her satırı toplamına böler; komşusu olmayan nesnenin satırı 0 kalır,
  sayısı söylenir.
- **Gi\*'da** wᵢᵢ = 1 (nesne kendi komşusudur), standartlaştırma yoktur.
- **Sınır:** komşu çiftlerinin sayısı 5·10⁷'yi aşarsa ret (“Bant çok geniş: komşu çiftleri 50000000'u aşıyor; bandı küçültün ya da k en
  yakın komşuyu seçin.”).

### 8. Mekânsal otokorelasyon (Moran I)

- **Girdi:** nesneler (en az dört); Değer alanı; Komşuluk, Bant, k, Satır standartlaştırma.
- **Hesap:** zᵢ = xᵢ − x̄; S₀ = ΣΣ wᵢⱼ; I = (n / S₀)·ΣΣ wᵢⱼzᵢzⱼ / Σ zᵢ²; E[I] = −1/(n − 1);
  S₁ = ½ ΣΣ (wᵢⱼ + wⱼᵢ)², S₂ = Σᵢ (Σⱼ wᵢⱼ + Σⱼ wⱼᵢ)², b₂ = n·Σ zᵢ⁴ / (Σ zᵢ²)²;
  E[I²] = (n·((n² − 3n + 3)S₁ − nS₂ + 3S₀²) − b₂·((n² − n)S₁ − 2nS₂ + 6S₀²)) / ((n − 1)(n − 2)(n − 3)S₀²);
  V[I] = E[I²] − E[I]²; z = (I − E[I]) / √V[I]. S₀ = 0 (komşu yok), Σ zᵢ² = 0 (değerlerin hepsi aynı) ya da V ≤ 0 ise ret, nedeniyle.
- **Sonuç:** tablo: Nesne sayısı, Moran I, Beklenen I (6 ondalık), Varyans (8 ondalık), z, p, Desen, Komşuluk (“Sabit uzaklık bandı,
  125.000 m”, “Ters uzaklık, 125.000 m”, “8 en yakın komşu”), Komşusu olmayan. Özet: “Moran I 0.412345 (z 6.1200, p 0.000000): kümelenmiş.”
  Çıktılar: tablo, I, z, p.

### 9. Sıcak nokta (Getis-Ord Gi\*)

- **Girdi:** nesneler (en az üç); Değer alanı; Komşuluk (Sabit uzaklık bandı ya da k en yakın komşu), Bant, k.
- **Hesap:** X̄ = Σ xⱼ / n; S = √(Σ (xⱼ − X̄)² / n);
  Gᵢ\* = (Σⱼ wᵢⱼxⱼ − X̄ Σⱼ wᵢⱼ) / (S·√((n Σⱼ wᵢⱼ² − (Σⱼ wᵢⱼ)²) / (n − 1))). S = 0 ise ret; payda 0 olan nesnenin (bütün nesneler
  komşusu) z'si yoktur: öznitelikleri boş, sınıfı 0, sayısı söylenir.
- **Güven sınıfı:** z'nin işaretiyle 3 (p < 0,01), 2 (p < 0,05), 1 (p < 0,10), yoksa 0.
- **Sonuç:** değeri ve yeri olan her nesnenin kopyası; öznitelikler kaynağınkiler ve z puanı (4 ondalık), p değeri (6 ondalık), Güven
  sınıfı; rengi sınıfın (ColorBrewer'ın RdBu'su: 3 #B2182B, 2 #EF8A62, 1 #FDDBC7, 0 #D9D9D9, −1 #D1E5F0, −2 #67A9CF, −3 #2166AC). Özet
  sıcak ve soğuk noktaları sayar: “120 nesne yazıldı: 14 sıcak, 9 soğuk nokta (%90 ve üstü güvenle).”
- **Görünüş:** katman `[Güven sınıfı]`'na göre kategorili: her sınıf kendi renginde alan (beyaz ince sınırla), çizgi (0,6 mm) ve dolu
  nokta (8 px); etiketleri “Sıcak nokta, %99 güven” … “Anlamlı değil” … “Soğuk nokta, %99 güven”.
- **Katman:** Sıcak noktalar.

### 10. Kümeleme

- **DBSCAN:** Yarıçap ε (m) ve En az nokta (çekirdek olmak için ε içinde kendisi dahil en az bu kadar yer; varsayılan 5); Sınır noktaları
  gürültü (DBSCAN\*, varsayılan kapalı).
  - Yerler sırasıyla gezilir; atanmamış bir çekirdek yer yeni kümeyi (1, 2, …) açar; küme sırayla genişler (kuyruk): kuyruktan alınan
    çekirdek yerin ε komşuları sırayla bakılır; atanmamış olan kümeye girer ve çekirdekse kuyruğa eklenir. Sınır noktası onu ilk ulaşan
    kümeye girer; DBSCAN\*'da çekirdek olmayan yer kümeye girmez.
  - Kümesiz yer gürültüdür (Küme 0).
- **k-ortalamalar:** k (2–100, varsayılan 5).
  - Başlangıç: ilk merkez ortalama merkeze en yakın yer; her sonraki merkez, seçilmiş merkezlerin en yakınına en uzak yer (eşitse küçük
    sıra). Karşılaştırmalar karesel uzaklıkla.
  - Lloyd'un yinelemesi: her yer en yakın merkeze (karesel uzaklık; eşitse küçük sıralı merkez); her merkez üyelerinin ortalaması (üyesi
    yoksa yerinde kalır); atamalar değişmeyince ya da 500 yinelemede durur. Farklı yer sayısı k'dan azsa ret.
- **Sonuç:** yeri olan her nesnenin kopyası; öznitelikler kaynağınkiler ve Küme, Küme boyu (gürültüde boş); rengi kümenin (on renklik sıra,
  küme numarasıyla döner: #1F77B4, #FF7F0E, #2CA02C, #D62728, #9467BD, #8C564B, #E377C2, #17BECF, #BCBD22, #7F7F7F; gürültü #BDBDBD).
  Özet küme ve gürültü sayısını, k-ortalamalarda yineleme sayısını söyler.
- **Görünüş:** katman `([Küme] - 1) % 10`'a göre kategorili (on renk dönerek; JavaScript'in kalanı işareti korur, gürültünün 0'ı −1
  olur, gri): dolu nokta (7 px), alan, çizgi; etiketleri “Küme 1, 11, 21 …” ve “Gürültü”.
- **Katmanlar:** Kümeler (DBSCAN), Kümeler (k-ortalamalar).

### 11. Arayüz

- **Araçlar** iki platformda aynı adlar, parametreler ve varsayılanlarla; **komutlar** `processing.run.<kimlik>`; tablo veren araçların
  tablosu çalıştırmadan sonra görünür olur (ADR 0237). Parçalı seçimlerin adları kısa (“1 kat”, “Sabit bant”, “k en yakın”), tam anlamı
  ipucunda. Bant yalnız bantlı komşulukta, k yalnız k en yakında görünür.
- **Şerit:** CBS'nin Analiz sekmesinde Mekânsal istatistik paneli: Ortalama merkez, Yön dağılımı, En yakın komşu, Moran I, Sıcak nokta ve
  DBSCAN görünür; Ortanca merkez, Standart uzaklık ve k-ortalamalar ▾'de (Analiz geniş pencerede 3200 px'i aşmasın diye).
- **Takma adlar:** ORTALAMAMERKEZ, MEANCENTER; ORTANCAMERKEZ, MEDIANCENTER; STANDARTUZAKLIK, STANDARDDISTANCE; YONDAGILIMI,
  DIRECTIONALDISTRIBUTION; ENYAKINKOMSU, NEARESTNEIGHBOR; MORAN, MORANI; SICAKNOKTA, HOTSPOT, GISTAR; DBSCAN; KORTALAMA, KMEANS.

### 12. Performans

- Hesap geometri çekirdeğinde (`ops::spatial_stats`); masaüstünde İşlemler'in iş parçacığında, web'de 2 000 ve daha çok nesnede işlem
  işçisinde (ADR 0124). Çekirdek yazılan metinleri ve tabloları da verir.
- Komşular dengeli bir k-d ağacıyla bulunur (n log n; aykırı bir nokta ızgaradaki gibi aramayı bozmaz); k en yakında (d², sıra) sırası
  kesindir, bant aramasında ağacın budaması d ≤ bant'ı hiç kaçırmaz. Moran'ın S₁ ve S₂'si seyrek komşuluk listelerinden.
- **Bütçeler** (release, geliştirme makinesi, 100 000 nokta; masaüstü çekirdeği doğrudan, web çağrıyı JSON'uyla, gönderilen WASM'la):

| İş | Masaüstü | Web |
|---|---|---|
| Ortalama ve ortanca merkez, standart uzaklık, yön dağılımı | ≤ 0,2 s | ≤ 0,6 s |
| En yakın komşu | ≤ 0,5 s | ≤ 1,5 s |
| Moran I, sabit bant (ortalama ~10 komşu) | ≤ 1 s | ≤ 3 s |
| Sıcak nokta, sabit bant | ≤ 1 s | ≤ 3 s |
| DBSCAN | ≤ 1 s | ≤ 3 s |
| k-ortalamalar, k = 10 | ≤ 1 s | ≤ 3 s |

## Uygulama

- **Çekirdek** (`crates/shared/geometry-core/src/ops/spatial_stats/`):
  - `kdtree`: dengeli k-d ağacı; `nearest` (d², sıra) sırasıyla, `within` d ≤ r'yi kaçırmayan budamayla.
  - `mod`: nesnenin yeri (`shape_centroid`), ilk yerin farkıyla yerler (`Placed`), sayının okunuşu, p, desen, metinler.
  - `centers` (§3–§5), `nearest` (§6), `weights` (§7), `autocorrelation` (§8, §9), `clusters` (§10).
  - `calls`: web'in çağrıları `statsCenters`, `statsNearest`, `statsMoran`, `statsHotSpots`, `statsDbscan`, `statsKMeans`.
  - Her çalıştırma `StatsRun`'dır: özet, iletiler, tablo, sayılar, yeni nesneler ve kopyalar, metinleri çekirdek yazar.
- **Web:** `model/ops/spatialStats.ts`; araçlar `processing/builtin/stats/` (`tools.ts`, `shared.ts`: kopya, sonuç, `HOT_RENDERER`,
  `CLUSTER_RENDERER`); kategori `spatialStats`; şeritte Mekânsal istatistik paneli (`app/ribbon.ts`); dokuz ikon (`statsMeanCenter`,
  `statsMedianCenter`, `statsStandardDistance`, `statsEllipse`, `statsNearest`, `statsMoran`, `statsHotSpot`, `statsDbscan`, `statsKMeans`).
- **Masaüstü:** `kentos-processing`'in `builtin/stats/` (`mod.rs`: kopya, sonuç, `hot_renderer`, `cluster_renderer`; `tools.rs`); yeni
  katmanın görünüşüne kategorili görünüş (`NewLayerStyle.renderer`); pencere ve resimler `apps/desktop` (`stats_scenes.rs`,
  `processing/stats_tests.rs`).
- **Ortak durumlar:** `fixtures/processing/v1/spatial-stats.json` (18 durum), iki çizim; karşılaştırıcılarda `r`, `major`, `ratio`
  geometri alanı, mekânsal istatistiğin oynatıcılarında `layerAbove`.

## Doğrulama

- **Bağımsız başvuru** `scripts/fixtures/spatial_stats_cases.py` (KentOS kodu yok; yerler ve karşılaştırmalar kesirlerle, kökler, ortanca,
  açılar ve erfc 40 basamaklı mpmath'le; yuvarlama sınırına 10⁻⁷'den yakın değer ve float64'te ayırt edilemeyecek karşılaştırma reddedilir):
  `fixtures/spatial-stats/v1/cases.json`, 68 durum. Çekirdeğin testi (`tests/all/spatial_stats.rs`) hepsini web'in çağrısıyla oynatır:
  metinler tam, yerler ve uzunluklar 10⁻⁶ m, istatistikler 10⁻⁹, p 10⁻¹² içinde. İlk tam koşuda 68'i de geçti.
- **İşlemler'in ortak durumları:** 18 durum iki platformda (web'de sayfada ve işçide, masaüstünde çizimde ve okuma kopyasında); varsayılanlar
  ve pencerenin formları (`dialog.json`, yalnız eklemeler).
- **Masaüstü:** pencerenin dört testi (elipsler gruplara göre ve tek adımda geri alma, Moran'ın tablosu çizimi değiştirmez, her bloğun
  sınıf renginde kopyası, kazaların kümeleri).
- **Süreler** (release, geliştirme makinesi, 100 000 nokta):

| İş | Masaüstü | Web (WASM) |
|---|---|---|
| Ortalama merkez | 0,004 s | 0,096 s |
| Ortanca merkez | 0,017 s | 0,115 s |
| Standart uzaklık | 0,004 s | 0,096 s |
| Yön dağılımı | 0,004 s | 0,095 s |
| En yakın komşu | 0,057 s | 0,164 s |
| Moran I, sabit bant (~10 komşu) | 0,118 s (JSON'la 0,184 s) | 0,205 s |
| Sıcak nokta, sabit bant | 0,139 s | 0,343 s |
| DBSCAN, ε 40 m, 5 nokta | 0,080 s | 0,226 s |
| k-ortalamalar, k = 10 (81 yineleme) | 0,071 s | 0,323 s |

- **Resimler:** masaüstünde `tools_screens`'in `ist-*`'ı, web'de `shots.mjs stats`; iki tema, iki boy.
