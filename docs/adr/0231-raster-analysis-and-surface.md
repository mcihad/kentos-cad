# ADR 0231: Raster çözümleme altyapısı ve DEM yüzey analizi

- **Durum:** kabul edildi (2026-10-09). Kapsam sahibin sözleridir (9 Ekim): “yeni bir branch açarak GIS-31 ve GIS-36 aralığını
  yapacağız”; “yüksek performans ilk önceliğimiz”; MADDE-TARIFI.md'nin sırası. Madde tek parçada biter. Dal `gis-31-36-raster-analysis`
  (paralel çalışma: ana ajan GIS maddelerini başka makinede `main`'de yürütüyor; ADR numaraları 0231–0236 bu aralık için ayrıldı, `main`
  0208'deydi). İki platformda (ADR 0207'nin eki: nokta bulutundan başka her modül masaüstünde ve web'de).
- **Bağlam belgesi:** TODOS.md `GIS-31`, ADR 0204 (raster nesnesi, okuyucu, piramit, karolu GeoTIFF yazıcısı, gdaldem'in gölgeli
  kabartması), ADR 0207 §7 (İşlemler'in dosyaları, Rasterleştir), ADR 0084, 0200 ve 0201 (İşlemler araçları, sonuç yeni katmana),
  ADR 0142 (köşe kotu), ADR 0149 (gösterim kuralı).

## Bağlam

Sayısal yükseklik modeli (DEM) harita mühendisliğinin ve planlamanın her gün kullandığı veridir: eğim haritası imar ve yol için,
bakı ve güneşlenme yerleşim ve güneş santrali için, gölgeli kabartma ve renkli kabartma altlık için, eş yükselti eğrileri her paftada.
Netcad (Analist › Yüzey Analizleri: Eğim, Bakı, Rölyef, Eğrisellik, Pürüzlülük, Yıllık Güneş Radyasyonu), ArcGIS (Slope, Aspect,
Hillshade, Curvature, Contour, Area Solar Radiation) ve QGIS (GDAL'ın gdaldem'i ve gdal_contour'u, Ruggedness index) bunları raster
işlem araçları olarak verir. KentOS GIS-08'le DEM'i okuyup çiziyor (ADR 0204) ama ondan yeni bir raster ya da eğri üretemiyor; web'de
bekleyen iki komut (`analysis.slope`, `map.contours`) bunun yerini tutuyor.

GIS-31'den GIS-36'ya altı madde (yüzey, interpolasyon, harita cebiri, raster ile vektör, hidroloji, maliyet) aynı işi yapar: bir ya da
birkaç rasteri okuyup hücre hücre hesaplar ve yeni bir raster ya da vektör yazar. Bu ADR o ortak altyapıyı ve onun ilk kullanıcısı
olan yüzey analizini tanımlar.

## Karar

### 1. Kapsam

İşlemler'in yeni **Yüzey analizi** kategorisinde sekiz araç:

| Araç | Kimlik | Çıktı |
|---|---|---|
| Eğim | `surface.slope` | 32 bit raster: derece ya da yüzde |
| Bakı | `surface.aspect` | 32 bit raster: kuzeyden saat yönünde derece, düzlük −1 |
| Gölgeli kabartma | `surface.hillshade` | 8 bit raster: 1–255, nodata 0 |
| Renkli kabartma | `surface.colorRelief` | 8 bit RGBA raster |
| Eğrilik | `surface.curvature` | 32 bit raster: toplam, profil ya da plan eğriliği |
| Pürüzlülük | `surface.ruggedness` | 32 bit raster: TRI (Riley ya da Wilson), TPI ya da engebe |
| Güneşlenme | `surface.insolation` | 32 bit raster: dönemin doğrudan güneş enerjisi, kWh/m² |
| Eş yükselti eğrileri | `surface.contours` | yeni katmanda çoklu çizgiler: kotlu köşeler, Kot ve Tür |

**TWI** (topografik nemlilik indisi) akış birikimini ister: hidrolojiyle birlikte GIS-35'tedir (ADR 0235).

### 2. Çözümleme işi (altı maddenin altyapısı)

- **Girdi:** tek bir raster nesnesi (Sahneden seç) ve bandı (1'den). Hep 0. kat (dosyanın kendisi) okunur. **Etkin nodata** rasterin
  görünüşündeki `nodata`'dır, yoksa dosyanınki (ADR 0204 §4); NaN da nodata'dır.
- **Şeritler:** çıktı 256 satırlık şeritlerle (karo boyu) üretilir; girdi şeridi araç neyi isterse o kadar kenar payıyla okunur (3 × 3
  pencere için 1). Okuyucunun blok istekleri ev sahibine gider (masaüstünde dosya, gömülü bayt ya da HTTP aralığı; web'de dosyanın
  dilimi). Bellek şerit kadardır, rasterin boyundan bağımsızdır.
- **Pencerenin kuralı:** rasterin dışında kalan komşu en yakın kenar hücresinin değerini alır; nodata komşu merkezin değerini alır;
  merkez nodata ise sonuç nodata'dır.
- **Çıktı dosyası:** karolu (256 × 256), Deflate'li GeoTIFF, kaynağın afini ve sistemiyle (rasterin `srid`'i 0'dan büyükse EPSG kodu).
  İçinde önizleme katları vardır, ayrıca piramit geçişi gerekmez. k. katın boyu ⌈w / 2ᵏ⌉ × ⌈h / 2ᵏ⌉, son kat tek karoya sığandır. Her
  kat bir üstünün 2 × 2 ortalamasıdır, nodata ve NaN dışarıda (ADR 0204'ün okuyucusunun kendi hesabı). 32 bitlik çıktıda nodata NaN'dır
  (`GDAL_NODATA` “nan”).
- **Yer:** masaüstünde seçilen yol (uzantısı `.tif` yapılır), yoksa bağlı kaynağın yanına `<ad>-<ek>.tif`; gömülü ya da adresteki
  kaynakta çizimin klasörü (ADR 0207'nin `Files::output_path`'i; `<ad>` dosyanın, adresin ya da gömülü varlığın kimliğinin adı,
  klasörü, sorgusu ve bilinen uzantısı atılarak). Dosya önce `.yaziliyor` ekiyle yazılır, bitince adlandırılır: durdurulan iş yarım
  dosya bırakmaz. Web'de Çıktı dosyası sonucun adıdır (aynı kuralla); sonuç 32 MB'a kadarsa projeye gömülür (ADR 0204 §8'in kuralı,
  kitaplığın düzenlemesi, geri alma adımı değil), büyüğü oturumun bu adlı bağlı dosyası olur ve indirilir. Adresteki raster web'de
  çözümlenmez (söylenir).
- **Sonuç nesnesi:** Çizime ekle açıksa (varsayılan) yeni katmanda raster, aracın görünüşüyle (§10), çalıştırmanın tek adımında.
  Eğriler aynı adımda yeni katmana yazılır. Yeni katman kaynağın katmanının hemen üstüne konur (aynı grupta, ondan önceki yere;
  sonuç altta kalsaydı kaynak onu örterdi): katman parametresinin `above`'u bir nesne parametresini adlandırır, çalıştırıcı hazırlarken
  onun ilk nesnesinin katmanını bulur ve yeni katmanı uygularken onun hemen üstüne koyar, iki platformda; öbür araçların yeni katmanları
  eskisi gibi en alta gider.
- **Sınırlar:** genişlik en çok 65 536 hücre, hücre sayısı en çok 2³¹; eğrilerin toplamı en çok 5 milyon köşe. Aşan iş nedeni ve
  çözümüyle söylenerek reddedilir (“aralığı büyütün”).
- **Sistemin ölçüsü:** afinin doğrusal kısmı J = [[a, b], [c, d]] (ADR 0204 §2: x = x₀ + a·i + b·j, y = y₀ + c·i + d·j) tersinmeli.
  Rasterin sistemi coğrafiyse (derece) türevler satırın enleminde (orta sütunundaki hücre merkezinin y'si) metreye çevrilir: GRS80
  elipsoidinde (a = 6 378 137 m, f = 1 / 298,257222101) doğu yönünde N(φ)·cos φ·π/180, kuzey yönünde M(φ)·π/180 metre bir derecedir;
  M(φ) = a(1 − e²) / (1 − e² sin²φ)^{3/2}, N(φ) = a / (1 − e² sin²φ)^{1/2}, e² = f(2 − f).

### 3. Türevler

3 × 3 pencere üstten alta, soldan sağa `z1 … z9` (z5 merkez; üst satır rasterin bir önceki satırı). Piksel ekseninde:

- **Horn** (gdaldem ve ArcGIS'in varsayılanı): pᵢ = ((z3 + 2·z6 + z9) − (z1 + 2·z4 + z7)) / 8, pⱼ = ((z7 + 2·z8 + z9) − (z1 + 2·z2 + z3)) / 8.
- **Zevenbergen ve Thorne:** pᵢ = (z6 − z4) / 2, pⱼ = (z8 − z2) / 2.

Dünyadaki eğim vektörü (doğu, kuzey) g = J⁻ᵀ·(pᵢ, pⱼ): det = a·d − b·c, gₓ = (d·pᵢ − c·pⱼ) / det, gᵧ = (a·pⱼ − b·pᵢ) / det (kuzeye
bakan rasterde gₓ = pᵢ / a, gᵧ = pⱼ / d, d < 0). Coğrafi sistemde gₓ ve gᵧ §2'deki ölçülere bölünür. **Z çarpanı** (z, varsayılan 1)
yükseklikleri ölçekler: gₓ ve gᵧ z ile çarpılır.

### 4. Eğim, bakı ve gölgeli kabartma

- **Eğim:** derece = atan(|g|)·180/π; yüzde = 100·|g|.
- **Bakı:** g = 0 ise −1 (düzlük); değilse atan2(−gₓ, −gᵧ)·180/π, eksi ise 360 eklenir (en dik inişin kuzeyden saat yönündeki açısı).
- **Gölgeli kabartma** (ADR 0204'ün gdaldem'le aynı kabartması, dünya eğimiyle): ışığın kuzeyden saat yönünde açısı A (315), yüksekliği
  h (45): c = (sin h − cos h·(gₓ·sin A + gᵧ·cos A)) / √(1 + gₓ² + gᵧ²); değer = 1 + 254·c, c ≤ 0 ise 1; 8 bite ⌊değer + 0,5⌋. Nodata 0.

### 5. Renkli kabartma

Yüksekliği renge çeviren tablo iki biçimdedir:

- **Rampa:** ADR 0204'ün altı rampasından biri (durakları eşit aralıklı), isteğe bağlı ters; değer aralığı bandın en küçüğünden en büyüğüne
  (0. katın bütünü, ilk geçişte) ya da elle verilen iki değer.
- **Tablo:** satır satır `değer #RRGGBB` ya da `değer #RRGGBBAA` (gdaldem color-relief'in metni gibi); değerler artan sıraya konur, aynı
  değerden ilki kalır. En az iki satır.

Tablonun satırları satır başıyla ya da noktalı virgülle ayrılır (tek satırlık alan “100 #2E7D32; 500 #FFF59D” yazar). Rampada en büyük
değer en küçükten büyük değilse (tek değerli bant) bütün hücreler rampanın (ters ise çevrilmiş rampanın) ilk rengini alır.

Eşleme: tablonun ilk değerinden küçük değer ilk rengi, son değerinden büyük değer son rengi alır. Arada **Doğrusal** (varsayılan) iki
komşu satırın arasında kanal kanal doğrusal; kanal ⌊v + 0,5⌋. **En yakın** iki satırdan değere yakın olanın rengi (eşitlikte küçük
değerin). Nodata saydamdır (alfa 0), öbür hücrelerin alfası tablonun alfası (yoksa 255).

### 6. Eğrilik

Zevenbergen ve Thorne'un yüzeyi, piksel ekseni boyunca (doğu yerine sütun, kuzey yerine satırın tersi): Lₓ = |(a, c)| ve Lᵧ = |(b, d)|
piksel kenarlarının metre boyu (coğrafi sistemde §2'nin ölçüleriyle), eksenler dik olmalı (|a·b + c·d| ≤ 10⁻⁹·Lₓ·Lᵧ; değilse ret).

- D = ((z4 + z6) / 2 − z5) / Lₓ², E = ((z2 + z8) / 2 − z5) / Lᵧ², F = (z3 − z1 + z7 − z9) / (4·Lₓ·Lᵧ), G = (z6 − z4) / (2·Lₓ),
  H = (z2 − z8) / (2·Lᵧ); yükseklikler z çarpanıyla.
- **Toplam:** −200·(D + E). Artı: yüzey yukarı doğru dışbükey (tepe), eksi: içbükey (çukur).
- **Profil** (eğim yönünde): 200·(D·G² + E·H² + F·G·H) / (G² + H²). Eksi: yukarı doğru dışbükey, akış yavaşlar; artı: içbükey.
- **Plan** (eğriler boyunca): −200·(D·H² + E·G² − F·G·H) / (G² + H²). Artı: yanal dışbükey (sırt), eksi: yanal içbükey (dere).
- G = H = 0 (düzlük) ise profil ve plan 0'dır. Birim, ArcGIS'teki gibi yükseklik biriminin yüzde biri (1/100 m⁻¹).

### 7. Pürüzlülük

Sekiz komşu n₁ … n₈ ve merkez z5; yükseklikler z çarpanıyla çarpılmaz (gdaldem gibi):

- **TRI (Riley, varsayılan):** √Σ (nₖ − z5)². **TRI (Wilson):** Σ |nₖ − z5| / 8.
- **TPI:** z5 − Σ nₖ / 8.
- **Engebe:** penceredeki (merkez dahil) en büyük ile en küçük değerin farkı.

### 8. Güneşlenme

Dönemde eğimli yüzeye gelen **doğrudan** güneş enerjisi, açık gökte (ArcGIS'in Area Solar Radiation'ının doğrudan kısmı gibi):

- **Dönem:** Yıl (1–365. günler), tarih aralığı (başlangıç ve bitiş günü ve ayı, aynı yıl, ikisi dahil) ya da tek gün; 365 günlük yıl.
  **Gün aralığı** s (varsayılan 14; Tek gün'de yoktur): dönem başlangıçtan s'er günlük parçalara bölünür (sonuncusu kısa olabilir);
  her parçayı ortadaki günü temsil eder (başlangıç + ⌊(uzunluk − 1) / 2⌋), ağırlığı parçanın gün sayısı. **Saat aralığı** Δ (varsayılan
  0,5 saat): güneş zamanında 0'dan 24'e Δ'lık dilimlerin ortaları tₖ = (k + ½)·Δ.
- **Güneşin yeri** (gün n, Γ = 2π(n − 1)/365; Spencer 1971): eğiklik δ = 0,006918 − 0,399912 cos Γ + 0,070257 sin Γ − 0,006758 cos 2Γ +
  0,000907 sin 2Γ − 0,002697 cos 3Γ + 0,00148 sin 3Γ; uzaklık çarpanı E₀ = 1,000110 + 0,034221 cos Γ + 0,001280 sin Γ + 0,000719 cos 2Γ +
  0,000077 sin 2Γ. Saat açısı ω = 15°·(t − 12). Güneşe doğru birim vektör (doğu, kuzey, yukarı): (−cos δ sin ω, sin δ cos φ −
  cos δ sin φ cos ω, sin φ sin δ + cos φ cos δ cos ω); yukarısı (sin h) ≤ 0 ise katkı yoktur.
- **Enlem φ:** satırın orta sütunundaki hücre merkezinin coğrafi enlemi (rasterin sisteminden, ADR 0167'nin dönüşümüyle); sistemi
  olmayan (yerel) rasterde araca yazılan Enlem.
- **Hava kütlesi** (Kasten ve Young 1989): m = 1 / (sin h + 0,50572·(h° + 6,07995)^−1,6364). Rakımın basınç çarpanı yoktur: geçirgenlik
  bütün rasterde aynıdır (sonucu hücre başına üs almadan hesaplatır).
- **Işınım:** I = 1367·E₀·τ^m·max(0, n · s) W/m²; τ geçirgenlik (varsayılan 0,5), n yüzeyin birim normali (−gₓ, −gᵧ, 1) / √(1 + |g|²),
  s güneş vektörü. Arazinin gölgesi yoktur, yalnız yüzeyin kendi gölgesi (n · s ≤ 0).
- **Enerji:** Σ parçalar Σ dilimler I·Δ·ağırlık / 1000 → kWh/m². Toplama sırası: güneş başına K = 1367·E₀·τ^m·Δ·ağırlık / 1000
  satır başına bir kez; hücrede Σ K·max(0, −gₓ·sₓ − gᵧ·sᵧ + s_z), en sonda √(1 + |g|²)'ye bölünür.

### 9. Eş yükselti eğrileri

- **Düzeyler:** Lₖ = taban + k·aralık, geçerli hücrelerin en küçüğü ≤ Lₖ ≤ en büyüğü olan bütün tam sayı k'lar için; aralık > 0, en çok
  10 000 düzey (tek bir kare 10 000'den çok düzey kaplıyorsa da ret: çoğu zaman bildirilmemiş bir nodata değeridir, ileti onu söyler). **Ana eğri** k'si **Ana eğri her** sayısının (m, varsayılan 5) katı olandır (k mod m = 0), öbürleri **Ara eğri**.
- **Kareler:** komşu dört hücre merkezi (sol üst (i, j), sağ üst, sağ alt, sol alt) bir karedir; birinin değeri nodata ise kare atlanır
  (eğri orada kesilir). Köşe z ≥ L ise içeridedir. Kenarda içerisi ve dışarısı ayrılan iki köşe arasında kesişme noktası t =
  (L − z_p) / (z_q − z_p) ile piksel koordinatında (hücre merkezi (i + ½, j + ½)), sonra afinle dünyaya.
- **Durumlar:** iki karşı köşe içerideyse (eyer) karenin ortası, dört köşenin ortalaması, L'den küçük değilse içerdeki köşeler birleşik
  sayılır (dışarıdakiler ayrı kalır), değilse içerdeki köşeler ayrı. Her parça yüksek taraf solunda kalacak yönde çizilir.
- **Birleştirme:** bir parçanın sonu öbürünün başıdır; bir düzeyin parçaları başı başka parçanın sonu olmayanlardan başlayarak (açık
  eğriler, karelerin satır satır sırasıyla), sonra kalanlardan (kapalı eğriler, ilk parçanın karesinin sırasıyla) zincirlenir. Art arda
  aynı noktalar teke iner; iki noktadan az kalan eğri düşer. Kapalı eğrinin son noktası ilkidir.
- **Sadeleştir** (isteğe bağlı tolerans, metre; varsayılan 0, kapalı): Douglas-Peucker, uçlar kalır, en uzak nokta (eşitlikte ilki)
  toleransı aşıyorsa bölünür; kapalı eğri ilk noktasından ikiye ayrılmış gibi.
- **Nesneler:** her eğri bir çoklu çizgi, köşelerinin kotu L (`zs`), öznitelikleri `Kot` (L, gösterim kuralıyla) ve `Tür` (Ana ya da
  Ara); ana eğrinin kalınlığı 0,35 mm. Sıra: düzeyler artan, düzeyin içinde zincirlenme sırası. Kot'un basamağı aralığın ve tabanın
  tam yazıldığı en az basamaktır (en çok 6; |x·10ᵈ − round(x·10ᵈ)| ≤ 10⁻⁹·max(|x·10ᵈ|, 1)): 2,5 ve 1,25 iki basamak verir.

### 10. Arayüz

- **Araçlar** İşlemler'de, iki platformda aynı adlar, parametreler ve varsayılanlarla: Raster (Seçili ya da Katman, Sahneden seç),
  Bant; aracın kendi parametreleri; Çıktı dosyası (masaüstünde yol, web'de ad; boşsa kaynağın adı ve aracın eki: `-egim`, `-baki`,
  `-golge`, `-renkli`, `-egrilik`, `-puruzluluk`, `-gunes`), Çizime ekle, Çıktı katmanı (varsayılan aracın adı). Eş yükselti
  eğrileri'nde Aralık (5 m), Taban (0), Ana eğri her (5), Sadeleştir (0) ve Çıktı katmanı. Güneşlenme'de Dönem (Yıl, Tarih aralığı,
  Tek gün), başlangıç ve bitiş günü ve ayı (21 Haziran, 21 Eylül), Gün aralığı (14), Saat aralığı (15 dakika, 30 dakika, 1 saat,
  2 saat), Geçirgenlik (0,5), Enlem (boş: projenin sisteminden). Renkli kabartma'da Renkler (Rampa, Renk tablosu), Rampa (Arazi),
  Ters, Değer aralığı (bandın en küçüğü–en büyüğü ya da Elle: En küçük, En büyük), Renk tablosu, Ara renkler. Takma adlar
  EGIMANALIZI, EGIMHARITASI, BAKI, GOLGEURET, KABARTMAURET, RENKLIKABARTMA, HIPSOMETRI, EGRILIK, EGRISELLIK, PURUZLULUK, TRI, TPI,
  GUNESLENME, GUNESRADYASYONU, ESYUKSELTI, KONTUR, EGRIURET (EGIM, HILLSHADE ve GOLGELIKABARTMA başka komutlarındır).
- **Görünüşler:** Eğim Spektral (düz mavi, dik kırmızı), Bakı Spektral, Gölgeli kabartma gri, Renkli kabartma RGBA, Eğrilik
  Mavi-kırmızı, Pürüzlülük Viridis, Güneşlenme Sıcaklık; gerdirme en küçük–en büyük, Eğrilik'te %2–98 (birkaç keskin hücre öbürlerini
  soldurmasın); Raster stili ile değişir. Kural çekirdekte (`Job::style`), iki platform aynısını alır.
- **Komutlar ve şerit:** her aracın `processing.run.surface.<ad>` komutu; web'in bekleyen `analysis.slope` komutu Eğim'i, `map.contours`
  Eş yükselti eğrileri'ni açar (ikisi menülerden kalkar: araçlar Yüzey analizi panelindedir). CBS şeridinin yeni **Raster** sekmesi
  (Analiz'den sonra, harfi R; QGIS'in Raster menüsü gibi): rasterlerin paneli (Raster ekle, Raster stili, Raster oturt; Veri'deki de kalır)
  ve İşlemler'in raster çözümleme kategorileri, her biri bir panel, şimdilik **Yüzey analizi** (sekiz araç). İşlemler'in her kategorisi
  Analiz'de bir panel olduğundan Yüzey analizi Analiz'i 1100 px'ten taşırırdı, GIS-32–GIS-36'nın beş kategorisi daha gelecek: Analiz
  menünün `except`'iyle onları dışarıda bırakır (`RASTER_ANALYSIS`, `app/ribbon.ts`). Masaüstünde şerit envanterden; sekmeler ve harf
  ipuçları ortak `fixtures/shell/v1/ribbon.json`'da (`ribbon_cases.py`).
- **İkonlar:** Eğim `slope` ve Eş yükselti eğrileri `contours` (var olanlar); yeni: Bakı `aspect` (pusula gülü, kuzey işareti ve iniş
  oku), Gölgeli kabartma `hillshade` (kuzeybatıdan ışıklı, öbür yamacı gölgeli tepe; kategorinin de ikonu), Renkli kabartma
  `colorRelief` (renk bantlı tepe), Eğrilik `curvature` (dışbükey ve içbükey kesit, eğrilik daireleriyle), Pürüzlülük `ruggedness`
  (kırık arazi çizgisi), Güneşlenme `insolation` (güneş ve ışınların vurduğu eğik yüzey).
- **İlerleme:** masaüstünde İşlemler'in iş parçacığı ve ilerleme çubuğu, Durdur; web'de çözümleme kendi işçisinde (çizimin karolarını
  hazırlayan raster işçilerini bekletmez), ilerleme İşlemler penceresinde.

### 11. Performans

Sahibin ilkesi: “yüksek performans ilk önceliğimiz”.

- Hesap ve sıkıştırma arayüzün iş parçacığında yapılmaz: masaüstünde İşlemler'in iş parçacığı ve onun içinde şeridin satırları ve
  karoların sıkıştırması çekirdek sayısının bir eksiği (en az 1, en çok 8) iş parçacığında; web'de iş başına bir çözümleme işçisi
  (`raster-wasm`, yalnız iş başlayınca yüklenir).
- Piksel verisi tipli dizilerdir; JSON'a yalnız parametreler ve eğriler girer. Şeridin tamponları iş boyunca yeniden kullanılır.
- Güneşlenmede güneşin konumları ve τ^m satır başına bir kez hesaplanır; hücrede yalnız iç çarpım toplanır.
- **Bütçeler** (release, geliştirme makinesi, Deflate'li 32 bit kaynak, katlarıyla yazılan çıktı): 4096 × 4096 DEM'in eğimi masaüstünde
  ≤ 1,5 s, web'de ≤ 6 s; aynı DEM'den 100 düzeyli eğriler ≤ 1,5 s; 2048 × 2048 DEM'in yıllık güneşlenmesi (14 gün, 0,5 saat) ≤ 3 s.
  Ölçüler Doğrulama'dadır.

## Kapsam dışı

TWI (GIS-35); arazinin gölgesi ve dağınık güneşlenme (görünür alan çözümlemesiyle); çok yönlü ve birleşik gölgeli kabartma; eğrilerin
yumuşatılması ve yazıları; TIN'den eğim ve eğri (`CIVIL-02`); başka sistemdeki rasterin yeniden izdüşümü (ADR 0204).

## Uygulama

- **Çekirdek** `crates/shared/raster` (`kentos-raster`, saf Rust, libm): `frame` (afin, J⁻ᵀ, coğrafi satırın GRS80 metresi),
  `terrain` (pencere, Horn ve Zevenbergen-Thorne türevleri, eğim, bakı, gölgeli kabartma, eğrilik, pürüzlülük; satır çekirdekleri),
  `relief` (rampa ve renk tablosu, `;` ya da satır başıyla), `insolation` (dönemin parçaları, Spencer, Kasten ve Young, `energy`),
  `contours` (kareler, eyer, zincirleme, Douglas-Peucker, `Spec::level_text`), `par` (iş parçacıkları; wasm'da tek), `out` (katlı
  GeoTIFF'in tek geçişte yazımı, karoların kodlaması paralel), `job` (`Spec`, `Tool`, `Job`: `needs`, `put`, `put_all` bloklar iş
  parçacıklarında çözülür, `step`, `share`, `finish`, `result`, `style`, `contours`). Biçim çekirdeğinde karonun kodlaması ayrıldı
  (`raster::write::code`, `coded`); geometri çekirdeğine `crs::System::geographic_of` ve `is_geographic`.
- **WASM** `crates/wasm/raster-wasm` (`AnalysisOpening`, `Analysis`): TIFF'in başlığı dilim dilim, PNG çözülerek, JPEG tarayıcının
  pikselleriyle; sonuç GeoTIFF'in parçaları ya da eğriler tipli dizilerle ve Kot yazılarıyla. `pnpm rust:wasm:raster`, paket
  `apps/web/src/io/raster/pkg` (`scripts/wasm/ensure.mjs`'te, `.gitignore`'da), yalnız çözümleme işçisinde yüklenir.
- **İşlemler (Rust)** `kentos_processing::builtin::surface` (`mod.rs`: parametreler, işin ayarı, işi ev sahibinin dosyalarıyla sürme,
  raster nesnesi, eğrilerin çoklu çizgileri; `tools.rs`: sekiz araç), kategori `surface` (Yüzey analizi). `files.rs`: `Beside`
  (sonucun yanına gideceği kaynak: bulut ya da raster, adın kuralı), `RasterOpen` ve `Files::open_raster`, `with_extension`,
  `NO_RASTER_FILES`, `RASTER_READER_BUDGET`. Katman parametresinin `above`'u (`ParamKind::Layer`), çalıştırıcının yeni katman planı
  (`NewLayerPlan`) ve yeni katmanı oraya koyması (`add_layer_at`).
- **Masaüstü:** `pointclouds/files.rs`'te `DesktopFiles::open_raster` (her çalıştırmanın kendi okuyucusu; bağlı dosya, kitaplığın
  `raster-…` varlıkları, adres), `rasters/tiles.rs`'in `open_reader`'ı (sahnenin ve çözümlemenin ortak açılışı, kendi bütçesiyle);
  `processing/mod.rs`: `map.contours` ve `analysis.slope` araçları açar; `catalog.rs` `PORTED`, `ported.json`, `equivalents.json`.
  Testler `processing/surface_tests.rs`, resimler `surface_scenes.rs` (`tools_screens`'in `yuzey-*`'ı).
- **Web:** `io/rasterAnalysis.ts` (iş başına bir işçi, Durdur işçiyi sonlandırır), `io/rasterAnalysisWorker.ts`,
  `io/rasterAnalysisProtocol.ts`; `processing/rasterHost.ts` (araçların ev sahibi) ve sayfanınki `app/rasterAnalysis.ts` (rasterin
  baytları raster hizmetinden, gömme ya da oturumun dosyası ve indirme); araçlar `processing/builtin/surface/shared.ts`, `tools.ts`;
  katman parametresinin `above`'u (`LayerParam`) ve çalıştırıcının yerleştirmesi (`processing/runner.ts`); `map.contours` ve `analysis.slope` `app/processing.ts`'te (bekleyenlerden çıktı,
  menülerden kalktı; CAD'in gizlediği komutlardan `analysis.slope` düştü); kategori `processing/categories.ts`; ikonlar `ui/icons.ts`.
  Testler `processing/cases.test.ts` (yüzey durumları), `processing/surfaceTesting.ts`, `io/raster.wasm.test.ts`; ölçüm
  `scripts/perf/raster.mjs`; resimler `shots.mjs`'in `surface` grubu.
- **İkonlar** (sorulmadan seçildi): `aspect`, `hillshade` (kategorinin de), `colorRelief`, `curvature`, `ruggedness`, `insolation`;
  `slope` ve `contours` var olanlar.
- **Sonraya kalan:** TWI GIS-35'te; arazinin gölgesi görünür alanla (GIS-36'nın yanında); büyük raster için web'de çok iş parçacıklı
  WASM (SharedArrayBuffer ve izolasyon gerektirir).

## Doğrulama

- **Bağımsız başvurular** (KentOS kodu yok):
  - `scripts/fixtures/terrain_cases.py --check`: 5 DEM (düz, dönük, coğrafi, int16 şeritli, 600 × 530 Deflate'li ve tahmin
    çarpanlı), 43 durum, tanımlar Python'un float'larında ADR'nin sırasıyla. gdaldem'le çapraz denetim (tepe, int16, geniş; iç
    hücreler): eğim Horn ve Zevenbergen-Thorne, derece ve yüzde 10⁻³°; bakı eğimi 1°'yi aşan hücrelerde 5·10⁻²° (gdaldem 32 bitte
    hesaplar); gölgeli kabartma iki ışıkla en çok bir adım; TRI Riley ve Wilson, TPI, engebe 10⁻³; renkli kabartma kanal başına bir
    adım. Güneşlenmenin satır enlemleri pyproj'dan. Dönük ve coğrafi DEM'de gdaldem başka okur (çevirir, tek ölçek alır): yalnız
    tanımdan.
  - `scripts/fixtures/contour_cases.py --check`: 7 durum (eyerler, düzeye eşit değerler, nodata, dönük afin, int16, geniş); Kot
    yazıları ADR 0149'un kuralıyla (decimal). gdal_contour'la çapraz denetim: genişte her düzeyin eğrilerinin toplam boyu göreli
    10⁻⁹ (deliği doldurulmuş, merkezlerin dikdörtgenine kırpılmış kopyada; gdal_contour nodata boyunca ve dış kenara uzatır).
  - `scripts/fixtures/surface_processing_cases.py --check`: İşlemler'in 15 ortak durumu (`fixtures/processing/v1/surface.json`,
    çizim `surface.kcad`): araçların adları, özetleri, katmanları ve nesneleri kuraldan; yazılan dosyanın 0. katı yüzey başvurusunun
    adı verilen durumunun değerleri (`rasterOf`), eğriler eğri başvurusununkiler (`contoursOf`), yeni katman kaynağın hemen üstünde
    (`layerAbove`); retler (iki raster, raster yok, olmayan bant, eğik pikselli rasterde eğrilik, 30 Şubat).
- **Testler:** `kentos-raster` 7 (43 yüzey durumu f32'de en çok 1 ulp, baytta birebir, katların 2 × 2 ortalaması, yer ve sistem; 7
  eğri durumu bit bit ve Kot'ları; kapalı eğride yüksek taraf solda; düzey sınırı; retler; yazılan enlemle güneşlenme; her sonucun
  görünüşü). `kentos-processing` ortak durumlar: yüzeyin 15'i masaüstünün çalıştırdığı gibi (çizimin okuma kopyasında, ev sahibinin
  dosyalarıyla) ve dosyasız ev sahibinde ret; araçların varsayılanları. Masaüstü `processing::surface_tests` 4 (vadi DEM'inde
  pencereden arka planda: yazılan yer ve `.yaziliyor`'un kalkması, nesne ve geri alma; eğrilerin kotları, Tür'ü ve kalınlığı;
  `map.contours` ve `analysis.slope`; dosyası olmayan raster). Web: `processing/cases.test.ts`'in yüzey bloğu 17 (15 durum, sayfanın
  çalıştırıcısıyla), `io/raster.wasm.test.ts` 50 (43 yüzey ve 7 eğri durumu WASM'da tek iş parçacığıyla), işlem penceresinin
  formları (`dialog.json`'a sekiz form eklendi; başka satır değişmedi).
- **Tarayıcıda uçtan uca:** `shots.mjs surface` her sahnede aracı İşlemler penceresinden çalıştırır: iş kendi işçisinde, sonuç
  gömülür ve çizilir (12 sahne, iki tema, iki boy).
- **Görsel incelemede bulunup düzeltilenler:** sonucun yeni katmanı en alta eklendiği için kaynak onu örtüyordu (katman parametresinin `above`'u); eğrilik en
  küçük–en büyük gerdirmeyle solgundu (%2–98); İşlemler'in kategorisi zaten panel olduğu için şeride ayrıca konan Yüzey analizi
  paneli kaldırıldı; takma adlar EGIM (ölçü ve hesaplayıcı), HILLSHADE ve GOLGELIKABARTMA (Raster stili) ile çakışıyordu; eğrilik
  ikonundaki çentik 16 pikselde “U” okunuyordu, eğrilik daireleri kondu.
- **Süreler** (9 Ekim 2026; Intel Core i5-13500, 20 iş parçacığı, Linux; 5 m'lik tepeli DEM, Deflate'li 32 bit kaynak, katlarıyla
  yazılan çıktı; üç koşunun ortancası):

  | İş | Masaüstü (8 iş parçacığı) | Masaüstü (1) | Web (Chrome, işçi, release WASM, 1) | Bütçe |
  |---|---|---|---|---|
  | Eğim 4096² | 0,383 s | 0,905 s | 1,667 s | masaüstü 1,5 s, web 6 s |
  | Bakı 4096² | 0,405 s | | | |
  | Gölgeli kabartma 4096² | 0,291 s | | | |
  | Eğrilik (profil) 4096² | 0,362 s | | | |
  | TRI 4096² | 0,334 s | | | |
  | Renkli kabartma 4096² (iki geçiş) | 0,545 s | | | |
  | Eş yükselti 4096², 5 m (~100 düzey) | 0,506 s | | 0,970 s | 1,5 s |
  | Güneşlenme 2048², yıl, 14 gün, 0,5 saat | 0,442 s | | 2,953 s | 3 s |

  Masaüstü: `cargo test --release -p kentos-raster --test all timing -- --ignored --nocapture --test-threads=1`; web:
  `node scripts/perf/raster.mjs` (işçinin bütün işi: başlaması, modül, dosyanın blokları, iş ve kodlama).
- **Ölçülmeyenler:** p99 (yalnız üç koşu); 4096²'den büyük rasterin web'deki belleği; masaüstünde adresteki (HTTP aralıklarıyla)
  rasterin çözümlenmesi gerçek bir sunucuyla denenmedi (yol bulut ve sahneyle ortak, `Bytes::Remote`); JPEG sıkıştırmalı DEM (yol
  var, örnek yok).
