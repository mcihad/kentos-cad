# ADR 0233: Raster işlemleri

- **Durum:** kabul edildi (2026-10-09). Kapsam sahibin sözleridir (9 Ekim): “yeni bir branch açarak GIS-31 ve GIS-36 aralığını
  yapacağız”; “yüksek performans ilk önceliğimiz”; MADDE-TARIFI.md'nin sırası. Madde tek parçada biter. Dal `gis-31-36-raster-analysis`
  (ADR numaraları 0231–0236 bu aralık için ayrıldı). İki platformda.
- **Bağlam belgesi:** TODOS.md `GIS-33`, ADR 0231 (raster çözümleme altyapısı: şeritler, `Out`, `par`, sonucun yeri, Raster sekmesi),
  ADR 0232 (noktalardan raster, ızgara, katman parametresinin `below`'u), ADR 0204 (raster nesnesi, afin, nodata, alfa, rampalar),
  ADR 0100 (ifade dili ve sütun motoru), ADR 0200 (İşlemler'in tablo çıktısı, `İçindekinden bilgi al`'ın yazma kuralı), ADR 0149
  (gösterim kuralı).

## Bağlam

Raster çözümlemenin günlük işleri hesap ve düzenlemedir: iki bandın oranı (NDVI), bir eşiğin üstü, eğim sınıfları, bir parselin
içinde kalan kısım, komşu paftaların birleşmesi, hücre boyunun değişmesi, her parselin ortalama kotu, bir pencerede ortalama ya da
en büyük değer, yılların ortalaması. Netcad (Analist › Raster Analizler: Band Aritmetiği, Sınıflandır, Mozaik, Filtre), ArcGIS
(Raster Calculator, Reclassify, Extract by Mask, Mosaic To New Raster, Resample, Zonal Statistics, Focal Statistics, Cell Statistics)
ve QGIS (Raster calculator, Reclassify by table, Clip raster by mask layer, Merge, Warp, Zonal statistics, Raster layer histogram,
r.neighbors, Cell statistics) bunları ayrı araçlar olarak verir. KentOS'ta raster çözümleme altyapısı var (ADR 0231) ama yalnız tek
rasterden pencere işleri yapar. Bu ADR dokuz aracı ve ortak parçalarını tanımlar: birden çok rasteri aynı ızgarada okuma, alanları
hücrelere çevirme, istatistiklerin sayı kuralı.

## Karar

### 1. Kapsam

İşlemler'in iki yeni kategorisinde dokuz araç:

| Araç | Kimlik | Kategori | Çıktı |
|---|---|---|---|
| Raster hesaplayıcı | `raster.calculator` | Raster işlemleri | 32 ya da 64 bit raster |
| Yeniden sınıflandır | `raster.reclassify` | Raster işlemleri | 32 bit, tam sayı ya da bayt raster |
| Maskeyle kırp | `raster.clipByMask` | Raster işlemleri | kaynağın türünde raster |
| Mozaik | `raster.mosaic` | Raster işlemleri | girdilerin türünde raster |
| Yeniden örnekle | `raster.resample` | Raster işlemleri | kaynağın türünde raster |
| Bölgesel istatistik | `raster.zonalStatistics` | Raster istatistiği | alanlara öznitelik ve tablo |
| Histogram | `raster.histogram` | Raster istatistiği | tablo |
| Komşuluk istatistiği | `raster.focalStatistics` | Raster istatistiği | 32 bit raster (64 bitlikten 64 bit) |
| Hücre istatistiği | `raster.cellStatistics` | Raster istatistiği | 32 bit raster (64 bitlikten 64 bit) |

İfade diline harita cebirinin matematik işlevleri eklenir (§3).

### 2. Ortak kurallar

- **Rasterin değeri:** bir bandın örneği; değersiz (nodata): örnek NaN ise ya da rasterin nodata'sına (görünüşün nodata'sı, yoksa
  dosyanınki) eşitse. Alfa bandı olan rasterde (ADR 0204: son bant alfa) alfası 0 olan pikselin bütün bantları değersizdir.
- **Yer:** rasterin yeri nesnesinin afinidir (Raster oturt değiştirmiş olabilir; ADR 0204 §2: x = x₀ + a·u + b·v, y = y₀ + c·u + d·v).
  Bütün rasterler çizimin koordinatlarındadır; hiçbiri yeniden izdüşürülmez.
- **Hücrenin merkezi:** (i + ½, j + ½) afinle. Bir rasterin bir noktadaki ızgara yeri ters afinle: dx = x − x₀, dy = y − y₀,
  D = a·d − b·c, u = (d·dx − b·dy)/D, v = (a·dy − c·dx)/D (bu sırayla, float64).
- **En yakın örnek:** noktanın içinde olduğu hücre: i = ⌊u⌋, j = ⌊v⌋; rasterin dışı (i, j ∉ [0, W) × [0, H)) değersiz. Hesaplayıcı,
  Mozaik ve Hücre istatistiği öbür rasterleri sonucun hücre merkezlerinde böyle okur (ArcGIS'in harita cebirinin varsayılanı); aynı
  ızgaradaki raster hücre hücre aynen okunur.
- **Sonuç dosyası:** ADR 0231 §2'deki gibi karolu, Deflate'li, katlı GeoTIFF; şimdi her örnek türünde (8–32 bit tam sayılar, 32 ve 64
  bit ondalık) ve bantta. Katların ortalaması değersizleri dışarıda bırakır (ADR 0231'in kuralı).
- **Sonucun değersizi:** kaynağın nodata'sı varsa o; yoksa ondalıkta NaN; 3 ya da 4 bantlı baytta (RGB, RGBA) alfa bandı (RGB'ye
  eklenir, değersiz piksel alfası 0); öbür tam sayılarda türün ucu (işaretlide en küçüğü, işaretsizde en büyüğü). Hesaplayıcı,
  sınıflandırma ve istatistikler kendi türlerini yazar (§3, §4, §10).
- **Sonuç nesnesi:** raster sonucu yeni bir katmanda, okunan ilk rasterin katmanının hemen üstünde, tek adımda (ADR 0231 §2). Katman
  parametresinin `above`'u girdinin ilk nesnesinin katmanını verir; okunan ilk raster başkaysa araç onu sonucunda söyler
  (`RunResult.above`, iki çalıştırıcıda: yeni katman araç bir katman söylerse onun üstüne). Dosyanın adı (çıktı dosyası boşsa) ve sistemi
  de o rasterin. Masaüstünde dosya kaynağın yanına ya da seçilen yola, web'de gömülü (32 MB'a kadar) ya da oturumun dosyası.
- **Girdilerin sırası:** Katmanlar panelindeki sıra, üstteki önce; aynı katmanda sonra çizilen önce (çizimde üstte görünen önce).
  Mozaik'in Üstteki ve Alttaki'si, ızgarayı seçen eşitlik ve Hücre istatistiği'nin girdileri bu sırayı kullanır.
- **Sınırlar:** ADR 0231 §2'ninkiler (sonucun genişliği en çok 65 536, hücre sayısı en çok 2³¹); en çok 64 girdi raster.

### 3. Raster hesaplayıcı

- **Girdi:** rasterler (Görünen, Katman, Seçili, Tümü; varsayılan Görünen). Yalnız ifadenin andığı rasterler okunur; öbürleri açılmaz
  da, sayılmaz da (64 girdi sınırı okunanlara uygulanır): hangilerinin okunacağı rasterler açılmadan, adlarından çekirdekte bulunur
  (`OpsSpec::reads`; web'de çözümleme işçisinde `opsReads`), ev sahibi yalnız onları açar. Dosyası verilmemiş ya da okunamayan bir
  raster yalnız okunuyorsa söylenir. Okunanların sırası ifadenin andığı sıradır; ilki sonucun ızgarasını, adını ve yerini verir.
- **Adlar:** her raster katmanının adıyla anılır; aynı katmanda birden çok girdi raster varsa ikincisi ve sonrakiler `Katman (2)`,
  `Katman (3)` (girdilerin sırasıyla). Bant `@` ile: `[Ortofoto@3]`; `[DEM]` 1. banttır. Ad dilin alan yazımıyla: köşeli parantezde
  (sözcükse çıplak da).
- **İfade:** KentOS'un ifade dili (ADR 0100): aritmetik, karşılaştırma (doğru 1, yanlış 0), `ve`, `veya`, `değil`, `durum eğer … ise …
  yoksa … son`, `içinde`, `arasında`, `boş`, işlevler. `$y` ve `$x` hücrenin merkezi (doğu, kuzey), `$alan` hücrenin alanı.
- **Matematik işlevleri** (dile eklenir, bütün ifadelerde): `ln` (doğal logaritma; 0 ve eksi sayıda boş), `log10`, `log(taban, sayı)`,
  `üstel` (eˣ, takma adı `exp`), `sin`, `cos`, `tan`, `asin`, `acos`, `atan` (radyan), `atan2(y, x)`, `derece` (radyandan, `degrees`),
  `radyan` (dereceden, `radians`). Değerleri libm'den (iki platformda aynı bitler); sonlu olmayan sonuç boş.
- **Izgara:** ifadenin metinde ilk andığı rasterin ızgarası; öbürleri hücre merkezlerinde en yakın örnekle (§2).
- **Değeri olmayan hücreler:** “Değersiz kalır” (varsayılan; QGIS, gdal_calc, ArcGIS): ifadenin okuduğu rasterlerden birinin değeri
  yoksa sonuç değersiz. “İfade karar verir”: ifade değersizi `boş` olarak görür (aritmetik boş verir, karşılaştırma yanlış; `boş`,
  `eğer`, `varsayılan` onu karşılar).
- **Sonuç:** sayı olduğu gibi; doğru/yanlış 1/0; boş, sayı okunamayan metin, NaN ve ±∞ değersiz. Tür: Ondalık 32 bit (varsayılan) ya da
  Ondalık 64 bit; değersiz NaN. Görünüş Viridis, en küçük–en büyük gerdirme.
- **Hesap:** ifade bir kez derlenir (bantlar şemanın sayı alanları); satır satır sütun motoruyla (256 hücrelik partiler), satırlar
  işin iş parçacıklarında.

### 4. Yeniden sınıflandır

- **Girdi:** bir raster, bir bant.
- **Tablo:** satır başına bir kural (satırlar yeni satır ya da `;` ile; sayılar boşlukla ayrılır, ondalık `.` ya da `,`):
  `alt üst yeni` aralık, `değer yeni` tek değer, `boş yeni` değersiz hücreler; açık uç `*` (`* 0 0`: 0'a kadar). `yeni` `boş` olabilir
  (değersiz yapar).
- **Sınırlar:** “alt < değer ≤ üst” (varsayılan; QGIS'in varsayılanı) ya da “alt ≤ değer < üst”. Tek değerli kural eşitliktir.
- **Sıra:** tablonun sırasıyla ilk tutan kural.
- **Tabloda olmayanlar:** “Olduğu gibi kalır” (varsayılan; QGIS, ArcGIS) ya da “Değersiz olur”. Değersiz hücre `boş` kuralı yoksa
  değersiz kalır.
- **Tür:** Ondalık 32 bit (varsayılan, değersiz NaN), Tam sayı 32 bit (değersiz −2 147 483 648), Bayt (0–254, değersiz 255). Tam
  sayıda değer yarımlar sıfırdan uzağa yuvarlanır; kuralın yeni değeri türe sığmıyorsa ya da değersizin kendisiyse ret.
- **Görünüş:** Spektral, en küçük–en büyük.

### 5. Maskeyle kırp

- **Girdi:** bir raster (bütün bantları); Maske: kapalı alanlar (kapalı alan, daire, elips, kapalı eğri, tarama; parçaları ve delikleriyle).
- **Hücre maskenin içinde:** merkezi en az bir maske nesnesinin içindeyse (nesnelerin birleşimi; delik dışarıdır). İçerisi her nesnede
  çift-tek kuralıyla, hücre merkezlerinin satırı boyunca (§6'nın alanları hücrelere çevirme kuralı).
- **Kırpma:** “Maskenin kutusuna kırp” (varsayılan açık): sonuç kaynağın ızgarasının maskeye düşen hücrelerinin satır ve sütunları
  (kaynağın hücreleri aynen, yeniden örnekleme yok); kapalıysa kaynağın bütün ızgarası. Maskenin içinde hücre yoksa ret.
- **Dışarıdaki hücreler:** değersiz (§2'nin sonucun değersizi kuralı). Tür ve bantlar kaynağın (alfa eklenirse bir fazla). Görünüş
  kaynağınki.

### 6. Alanları hücrelere çevirme

- **Kenarlar:** nesnenin bütün halkaları (dış, delikler, parçalar): doğru parçaları ve yaylar (daire, yaylı kenar) olduğu gibi; elips
  ve eğri 0,1 mm içinde doğru parçalarıyla (ADR 0149, geometri çekirdeğinin `entity_edges`'i).
- **Satır:** j. satırın hücre merkezleri afinle bir doğrudur: q(t) = (x₀ + a·t + b·(j + ½), y₀ + c·t + d·(j + ½)). Kenarın doğruyu
  kestiği yerler t olarak bulunur (yayda yay parçalarına bölünerek); her kenar yarı açık kuralla sayılır (ucu doğrunun bir yanında ya da
  üstünde): doğrunun normali n = (−c, a) yönünde f(p) = n·(p − q(0)) > 0 olan uç “üstte”, öbürü “altta”; uçları farklı yanlarda olan
  kenar bir kez keser.
- **İçerisi:** t'ler sıralanır; hücre i, merkezi t = i + ½ kendisinden küçük ya da eşit tek sayıda kesişim varsa içeridedir.
- **Hız:** kenarlar satır aralıklarına göre sıralanır; her şeritte yalnız şeride değen kenarlar denenir.

### 7. Mozaik

- **Girdi:** en az iki raster (Seçili, Görünen, Katman, Tümü; varsayılan Seçili). Hepsinin bant sayısı ve örnek türü aynı olmalı
  (değilse ret, nedeniyle).
- **Izgara:** en ince hücreli girdinin (hücre alanı |a·d − b·c| en küçük; eşitse sıradaki ilk) eksenleri ve hücre boyu; kafesi
  girdilerin hepsini kapsayacak kadar büyütülür: girdilerin dört köşesinin o ızgaradaki yerleri (u, v), sütunlar ⌊u_min + 10⁻⁹⌋'den
  ⌈u_max − 10⁻⁹⌉'e, satırlar ⌊v_min + 10⁻⁹⌋'den ⌈v_max − 10⁻⁹⌉'e (kafese oturan köşe gürültüyle bir hücre büyütmez).
- **Örnekleme:** En yakın (varsayılan), Çift doğrusal, Kübik (§8'in nokta yöntemleri), her girdi kendi değerleriyle.
- **Çakışanlar:** Üstteki (varsayılan: çizimde üstte görünen), Alttaki, Ortalama, En küçük, En büyük. Her bant ayrı; değeri olmayan girdi
  sayılmaz; hiçbirinin değeri yoksa değersiz.
- **Tür ve görünüş:** girdilerinki (sonucun değersizi §2); görünüş sıradaki ilk girdininki.

### 8. Yeniden örnekle

- **Girdi:** bir raster (bütün bantları); Hücre boyu (> 0, metre).
- **Izgara:** sol üst köşe ve eksenler kaynağınki; hücreler eksenler boyunca yeni boyda: r_u = s/|a⃗|, r_v = s/|b⃗| (eksen x ya da y
  boyuncaysa uzunluğu tam |a|, |d|), a⃗' = a⃗·r_u, b⃗' = b⃗·r_v; genişlik ⌈W/r_u − 10⁻⁹⌉, yükseklik ⌈H/r_v − 10⁻⁹⌉, en az 1 (kaynağı
  bütünüyle kapsar; kayan noktanın gürültüsü bir sütun eklemez).
- **Nokta yöntemleri** (hücre merkezinin kaynaktaki yeri u, v):
  - En yakın (varsayılan): §2.
  - Çift doğrusal: (u − ½, v − ½) çevresindeki dört hücre merkezi, ağırlıklar (1 − fx)(1 − fy) …
  - Kübik: on altı hücre, Keys'in kübik evrişimi (a = −½; GDAL'ın `cubic`'i).
  - Değeri olmayan ya da rasterin dışındaki hücreler dışarıda kalır, kalanların ağırlıkları toplamlarına bölünür; toplam 10⁻⁶'dan
    küçükse değersiz.
- **Alan yöntemleri** (sonuç hücresinin kaynaktaki dikdörtgeni [u₀, u₁) × [v₀, v₁); eksenler aynı olduğundan eksenlere paralel):
  her kaynak hücresinin örtüşmesi iki eksendeki örtüşmelerin çarpımı; bir eksende hücrenin 10⁻⁹'undan küçük örtüşme sayılmaz.
  Ortalama (örtüşmeyle ağırlıklı), Çoğunluk (toplam örtüşmesi en büyük değer; eşitse küçüğü), En küçük, En büyük.
- **Tür:** kaynağınki; tam sayıda yarımlar sıfırdan uzağa yuvarlanır. Alfa bandı maskedir: sonuçta değer çıkan pikselin alfası 255.
- **Görünüş:** kaynağınki.

### 9. Sayıların kuralı (`kentos.rasterstats/1`)

Bölgesel istatistik, Komşuluk istatistiği ve Hücre istatistiği'nin sayıları (değeri olan hücrelerin değerleri, x₁ … xₙ):

- **Sayı** n; **En küçük**, **En büyük** kesin; **Aralık** en büyük − en küçük (bir yuvarlama).
- **Toplam** ve **Ortalama:** toplam çift-çift (double-double, 106 bit) toplanır, ortalama toplamın n'ye bölümü; ikisi de en sonda bir
  kez float64'e yuvarlanır.
- **Standart sapma** (örneklem, n − 1; ADR 0200 §4 gibi): S₁ = Σx ve S₂ = Σx² (her kare çift-çift, kesin) çift-çift toplanır;
  D = S₂ − S₁²/n çift-çift, varyans D/(n − 1), karekökü float64. n < 2 ise değersiz.
- **Ortanca:** sıralanmış değerlerin ortadaki (çiftte ortadaki ikisinin ortalaması, bir yuvarlama). **Çoğunluk** en sık değer, **Azınlık**
  en seyrek değer (eşitse küçüğü); **Çeşit** farklı değer sayısı.
- **Belirlenimcilik:** bölgelerin toplamları satır satır, satırların sırasıyla birleştirilir; sonuç iş parçacığı sayısından bağımsızdır,
  iki platformda aynıdır.
- **Doğruluk:** kesin değerden en çok 1 ulp + 2⁻⁹⁰·Σ|xᵢ| (kayan pencerede sütunların toplamları 32 satırda, satırınki 256 sütunda bir
  yeniden kurulur).

### 10. Bölgesel istatistik

- **Girdi:** raster ve bant; Bölgeler: kapalı alanlar (§5'in türleri; kilitli katmandakiler alınmaz).
- **Bölgenin hücreleri:** merkezi alanın içinde olanlar (§6); bir hücre örtüşen bölgelerin hepsinde sayılır; değeri olmayan hücre
  sayılmaz.
- **İstatistik** (yazılan): Sayı, Toplam, Ortalama (varsayılan), En küçük, En büyük, Aralık, Standart sapma, Ortanca, Çoğunluk,
  Azınlık, Çeşit, Alan (değeri olan hücrelerin alanlarının toplamı, m²).
- **Yazma:** İçindekinden bilgi al gibi (ADR 0200 §3): seçilen istatistik her alanın Yazılacak alan'ına (var olan ya da yeni); hücresi
  olmayan bölgeye Sayı 0 ve Alan 0, öbürleri alanı boşaltır. Metin gösterim kuralıyla Basamak (varsayılan 3) basamak; Sayı ve Çeşit
  tam sayı; katmanın tipli alanında alanın kuralıyla (ADR 0199).
- **Tablo:** her bölge bir satır: Nesne (etiketi, yoksa #numarası), Sayı, Toplam, Ortalama, En küçük, En büyük, Standart sapma ve
  bunlardan değilse yazılan istatistik. Panoya kopyala, CSV.

### 11. Histogram

- **Girdi:** raster ve bant; Aralık sayısı (varsayılan 20, 1–1000); En küçük ve En büyük (boşsa bandın en küçüğü ve en büyüğü).
- **Aralık:** k = ⌊(x − alt) / (üst − alt) · n⌋, [0, n − 1]'e sıkıştırılır (float64'te bu sırayla); alt = üst ise tek aralık. Sınırların
  dışındaki değerler ayrıca sayılır.
- **Tablo:** Aralık, Alt sınır, Üst sınır, Sayı, Oran (%), Birikimli (%). Özet: değeri olan ve olmayan hücreler, sınırlar, dışarıda
  kalanlar. Çizim değişmez.

### 12. Komşuluk istatistiği

- **Girdi:** raster ve bant.
- **Komşuluk:** Dikdörtgen (Genişlik × Yükseklik hücre, tek sayılar, 1–255; varsayılan 3 × 3), Daire (Yarıçap r hücre, 1–127: merkezinin
  merkeze uzaklığı r'yi geçmeyen hücreler, di² + dj² ≤ r²), Halka (İç yarıçap r₁ < Dış yarıçap r₂: r₁² < di² + dj² ≤ r₂²).
- **İstatistik:** Ortalama (varsayılan), Toplam, En küçük, En büyük, Aralık, Standart sapma, Ortanca, Çoğunluk, Azınlık, Çeşit (§9).
- **Değersizleri yok say** (varsayılan açık; ArcGIS'in varsayılanı): pencerenin değeri olan hücreleri; hiçbiri yoksa değersiz (değersiz
  merkezin çevresinde değer varsa değer alır). Kapalıysa pencerede değeri olmayan hücre varsa değersiz. Rasterin dışı pencerede değildir.
- **Hesap:** Dikdörtgende toplamlar ayrılabilir kayan pencereyle (önce sütunlar, sonra satır; çift-çift; sütunlar 32 satırda, satır 256
  sütunda bir yeniden kurulur: hata sınırlı kalır, satır grupları iş parçacıklarına dağılır), en
  küçük ve en büyük van Herk–Gil–Werman ile (hücre başına sabit iş); Dairede ve Halkada pencerenin her satırı bir aralıktır (satır önek
  toplamları, satırın kayan en küçüğü). Sıra istatistikleri (Ortanca, Çoğunluk, Azınlık, Çeşit) pencerenin değerlerini toplayıp seçer
  (hücre başına pencere boyunda iş).
- **Tür:** Ondalık 32 bit (kaynak 64 bitse 64 bit); görünüş Viridis.

### 13. Hücre istatistiği

- **Girdi:** en az iki raster ve bant (hepsinde aynı numara, varsayılan 1).
- **Izgara:** Mozaik'inki (§7); girdiler en yakın örnekle okunur.
- **İstatistik:** Ortalama (varsayılan), Toplam, En küçük, En büyük, Aralık, Standart sapma, Ortanca, Çoğunluk, Azınlık, Çeşit, Sayı
  (§9; girdilerin sırasıyla).
- **Değersizleri yok say** (varsayılan açık): değeri olan girdiler; kapalıysa birinin değeri yoksa değersiz.
- **Tür:** Ondalık 32 bit (girdilerden biri 64 bitse 64 bit); görünüş Viridis.

### 14. Arayüz

- **Araçlar** iki platformda aynı adlar, parametreler ve varsayılanlarla. Rasterlerden oluşan girdinin alanları rasterlerin adlarıdır:
  her rasterin adı ve ilkinden sonraki bantları `Ad@b` (girdinin özeti, `summarizeFeatures`, `summarize_features`; her biri bir
  nesnede). Hesaplayıcının ifade alanının çipleri ve ε'unun açtığı İfade oluşturucu'nun alanları bunlardır. Rasterlerde ifadenin
  önizleme satırı ve oluşturucunun nesne önizlemesi yoktur (ifade hücre hücre çekirdekte çalışır; bir nesnenin değeri anlamsızdır).
- **Komutlar:** `processing.run.raster.<ad>`. **Şerit:** CBS'nin Raster sekmesinde Raster işlemleri ve Raster istatistiği panelleri.
- **Takma adlar:** RASTERHESAP, HARITACEBIRI, BANDARITMETIGI; YENIDENSINIFLA, RECLASS; MASKEKIRP, RASTERKIRP; MOZAIK; YENIDENORNEKLE,
  RESAMPLE; BOLGESELISTATISTIK, ZONAL; HISTOGRAM; KOMSULUK, ODAKISTATISTIK, FOCAL; HUCREISTATISTIK, CELLSTATS.
- **İlerleme ve Durdur:** masaüstünde İşlemler'in iş parçacığı, web'de çözümleme işçisi (iş başına bir işçi; girdiler Blob, maske ve
  bölgeler JSON).

### 15. Performans

- Her araç ADR 0231'in şerit işidir: sonucun 256 satırlık şeridi için her girdinin gereken blokları; satırlar iş parçacıklarında; aynı
  ızgaradaki girdi kopyalanır, öbürleri şeridin 256 × 256'lık parçalarıyla okunur (dönük girdide de bellek sınırlı).
- **Bütçeler** (release, geliştirme makinesi; 4096 × 4096 Float32):

| İş | Masaüstü | Web (işçi) |
|---|---|---|
| Hesaplayıcı, iki raster, `[A] * 2 + [B]` | ≤ 1 s | ≤ 4 s |
| Yeniden sınıflandır, 10 kural | ≤ 0,6 s | ≤ 3 s |
| Maskeyle kırp, 10 000 parsel | ≤ 1 s | |
| Mozaik, dört 2048² → 4096² | ≤ 1,5 s | |
| Yeniden örnekle, Ortalama 2× ve Çift doğrusal ½× | ≤ 1,5 s | |
| Bölgesel istatistik, 10 000 parsel | ≤ 1,5 s | |
| Histogram | ≤ 0,6 s | |
| Komşuluk, 5 × 5 Ortalama; 15'lik Daire En büyük; 5 × 5 Ortanca | ≤ 1 s; ≤ 2 s; ≤ 3 s | |
| Hücre istatistiği, beş raster, Ortalama | ≤ 1,5 s | |

## Uygulama

- **Raster çekirdeği** `kentos-raster`: `dd` (çift-çift: `two_sum`, Veltkamp'lı `two_prod`, toplama, çarpma, bölme, kare), `stats`
  (`Stat`, `Moments`: sayı, çift-çift S₁ ve S₂, en küçük, en büyük, birleştirme; `order_stat`: ortanca, çoğunluk, azınlık, çeşit),
  `inputs` (`Input`: okuyucu, afin, bantlar, alfa, tür, nodata; `place_in`, `Mapping`: aynı kafeste kaydırma, değilse afin; `View`,
  `region_for`, nokta yöntemleri ve Keys'in çekirdeği, `empty_of`), `areas` (`Areas`: hücre uzayında kenarlar, yayların v'ye göre tekdüze
  parçaları, kesin `orient2d`'yle yarı açık kural, şerit şerit kenar listesi, nesne başına çift-tek aralıklar, `union`), `calc` (`Calc`:
  bantlar şemanın sayı alanları, `named`: adlardan okunan girdiler, `row`: sütun motoru), `reclass` (tablonun okunması ve kurallar), `focal`
  (`Window`: dikdörtgen, daire, halka; ayrılabilir kayan toplamlar 32 satır ve 256 sütunda bir yeniden kurularak, satır aralıklarıyla
  daire ve halka, van Herk–Gil–Werman'ın `running`'i, sıra istatistikleri), `resample` (`grid_of`, `reach`, alan yöntemlerinin
  örtüşmeleri), `ops` (`OpsSpec`: araç, girdiler, `reads`, `reading`; `OpsJob`: `needs`, `put_all`, `put_pixels`, `step`, `share`,
  `finish`; birleşim ızgarası, şeridin tipli `Samples`'a paralel kurulması, hücre istatistiğinin özel yolları, histogramın aralığı,
  görünüş kuralları), `out` (`Rows`: her örnek türü; katların karoları ve Deflate'i `par::map`'te). İfade diline matematik işlevleri
  (`library`, `functions`, `kernels`, akışın türleri, tamamlama; değerler geometri çekirdeğinin `jsmath`'inden, libm).
- **WASM** `raster-wasm`: `OpsOpening` (girdiler: GeoTIFF, PNG, tarayıcının çözdüğü JPEG), `start` (okunanlarla kurulur), `OpsAnalysis`
  (bloklar, şeritler, sonuç dosyasının kuyruğu ve başı, ızgara, tür, görünüş, notlar, bölgelerin yedişer sayısı, histogram), `opsReads`.
- **İşlemler (Rust)** `builtin::raster_ops` (`mod.rs`: girdilerin sırası ve adları `features::raster_order`, `raster_names`; ayarlar,
  okunanların açılması, `drive`, raster nesnesi, `above`, özetler; Bölgesel istatistik'in yazması ve tablosu, histogramın tablosu;
  `tools.rs`: dokuz araç), kategoriler `rasterOps` ve `rasterStats`. Girdinin özetinde rasterlerin adları (`summarize_features`),
  önizlemesiz rasterler (`runner::only_rasters`), `RunResult.above` ve çalıştırıcının yerleştirmesi. Ortak durumlar
  `fixtures/processing/v1/raster-ops.json` ve `.kcad`, rasterleri `raster-ops/*.tif` (`raster_ops_processing_cases.py`; anahtarlar
  `rasterOpsOf`, `layerAbove`, `inputFields`), oynatıcı `tests/cases/surface.rs`'in `raster_cases`'i ve
  `a_raster_input_offers_its_bands_names`.
- **Masaüstü:** `catalog.rs`'in `PORTED`'ı, `ported.json`; İfade oluşturucu rasterlerde nesne önizlemesi açmaz; testler
  `processing/raster_ops_tests.rs`, resimler `raster_ops_scenes.rs` (`tools_screens`'in `ops-*`).
- **Web:** `io/rasterAnalysisProtocol.ts` (`OpsRequest`: girdilerin dosyaları ya da okunamama nedenleri; `OpsResult`: okunanlar, raster,
  bölgeler, histogram), işçinin `ops` işi (`opsReads`, yalnız okunanlar açılır), `io/rasterAnalysis.ts`'in `analyzeOps`'u, ev sahibinin
  `analyzeOps`'u; araçlar `processing/builtin/rasterOps/shared.ts` ve `tools.ts`; `features.ts`'in `rasterRun`'ı (sıra ve adlar; aynı
  katmandaki rasterler için yalnız o katmanın nesneleri sorulur) ve özetin adları; `RunContext.layerIndex`; `RunResult.above`; önizlemesiz
  rasterler (`runner.ts`); kategoriler; şeridin `RASTER_ANALYSIS`'i Raster işlemleri ve Raster istatistiği'yle; ikonlar. Testler
  `processing/cases.test.ts`'in raster işlemleri bloğu, `io/raster.wasm.test.ts`'in raster işlemleri (başvurunun girdileri test içinde
  sıkıştırmasız TIFF olarak yazılır), `processing/surfaceTesting.ts`'in `analyzeOpsHere`'i; ölçüm `scripts/perf/raster.mjs`; resimler
  `shots.mjs`'in `rasterops` grubu.
- **İkonlar** (sorulmadan seçildi): `rasterCalculator` (tuşları raster hücreleri olan hesap makinesi; Raster işlemleri kategorisinin de),
  `reclassify` (tonlardan iki sınıfa ok), `clipRaster` (raster ızgarasını kesen alan), `mosaic` (örtüşen iki pafta), `resample` (ince ve
  kaba ızgara bir arada), `zonalStats` (çubuklarını taşıyan alan; Raster istatistiği kategorisinin de), `histogram` (taban çizgisinde
  çubuklar), `focalStats` (ızgarada bir hücrenin 3 × 3 penceresi), `cellStats` (raster yığını ve bir hücrenin sütunu).

## Doğrulama

- **Bağımsız başvurular** (KentOS kodu yok):
  - `scripts/fixtures/raster_ops_cases.py --check`: dokuz aracın kuralları ADR'den Python'la (kesirler ve mpmath): hesaplayıcı (13 durum:
    iki raster, kaydırılmış, yarım hücre ve dönük ızgara, koşul, boş, karşılaştırma, merkez ve alan, işlevler, bantlar, 64 bit, ilk anılanın
    ızgarası, ret), sınıflandırma (4), maskeyle kırpma (8: kare, sınırda, delikli ve parçalı, daire, yaylı, dönük raster, RGB'nin alfası,
    dışarıda), mozaik (8), yeniden örnekleme (10), bölgesel istatistik (12), histogram (2), komşuluk (11), hücre istatistiği (6); 74 durum.
    Yeniden örnekleme GDAL'ın `gdalwarp`'ıyla (iç hücrelerde aynı kural) 358 hücrede çapraz denetlenir.
  - `scripts/fixtures/raster_ops_processing_cases.py --check`: İşlemler'in 15 ortak durumu (10 çalıştırma, 5 ret) ve girdinin adları
    (`inputFields`); rasterleri GDAL yazar, okunup başvurunun girdisiyle karşılaştırılır. “Bütün rasterlerde” hesaplayıcı durumu yalnız
    andığı iki rasteri açabilir (ortofotoların dosyası yok); sonuç ilk anılanın ızgarasında, adıyla ve katmanının üstünde.
  - `scripts/fixtures/expression_language.py --check`: yeni matematik işlevleri 129 durumun içinde, 50 basamaklı mpmath'ten; doğru
    yuvarlanmış değerden ulp olarak uzaklık (libm'in kendi sınırı: tek argümanlılar 1, `atan2` ve `pow` 2; tabanlı `log` iki logaritmanın
    bölümü olduğundan 3).
- **Kurallar:** “exact” bit bit; “sum” toplamla bulunan değer sonucun türünde en çok bir son basamak birimi, bölgenin float64 sayısında
  iki (ADR'nin §9'u). Bir ve üç iş parçacığı aynı baytları yazar.
- **Testler:** `kentos-raster` 10 raster işlemi testi (başvurunun 74 durumu, 5 000'den çok değer; okunan girdiler) ve birim testleri
  (çift-çift, istatistikler, alanlar: kesin daire sayısı, kayan pencereler, tablo); `kentos-expression` dil durumları ve oluşturucunun
  yanıtları; `kentos-processing` ortak durumların 15'i, varsayılanlar, girdinin adları; masaüstü `raster_ops_tests` 4 (bütün rasterlerde
  hesaplayıcı, ifade alanının adları ve önizlemesi, bölgesel istatistik, histogram), araç sayısı 53; web `cases.test.ts`'in 17'si,
  `raster.wasm.test.ts`'in 75'i, `processing.test.ts` (kategori ağacında iki yeni kategori), `workspaces.test.ts` (Raster sekmesinin
  altı paneli), pencere formları (`dialog.json`'a dokuz form eklendi, başka satır değişmedi).
- **Tarayıcıda uçtan uca:** `shots.mjs rasterops` her sahnede aracı İşlemler penceresinden çalıştırır (9 sahne, iki tema, iki boy).
- **Görsel incelemede düzeltilenler:** Yeniden örnekle ikonu 16 pikselde sıkışıktı (iki ızgara yan yana yerine bir ızgarada ince ve kaba
  hücreler); Bölgesel istatistik ve Histogram tablolarını bildirmiyordu (pencere tabloyu göstermiyordu); Raster hesaplayıcı ifadenin
  anmadığı rasterleri de açıyordu (dosyası eksik ilgisiz bir raster çalıştırmayı durduruyordu) ve sonucu girdinin ilk nesnesinin
  katmanının üstüne koyuyordu.
- **Süreler** (9 Ekim 2026; Intel Core i5-13500, 20 iş parçacığı, Linux; 4096 × 4096 Float32, karolu ve Deflate'li girdiler, sonucun
  katlarıyla yazılması dahil; masaüstünde 8 iş parçacığı, web'de işçi, release WASM, tek iş parçacığı; üç koşunun ortancası):

  | İş | Masaüstü (8) | Bütçe | Web (işçi, 1) | Bütçe |
  |---|---|---|---|---|
  | Hesaplayıcı, iki raster, `[A] * 2 + [B]` | 0,433 s | 1 s | 1,844 s | 4 s |
  | Yeniden sınıflandır (masaüstünde 10, web'de 4 kural) | 0,218 s | 0,6 s | 0,764 s | 3 s |
  | Maskeyle kırp, 10 000 parsel | 0,289 s | 1 s | | |
  | Mozaik, dört 2048² → 4096² | 0,254 s | 1,5 s | | |
  | Yeniden örnekle, Ortalama 2× | 0,149 s | 1,5 s | 0,732 s | |
  | Yeniden örnekle, Çift doğrusal ½× (8192² sonuç) | 1,160 s | 1,5 s | | |
  | Bölgesel istatistik, 10 000 parsel | 0,121 s | 1,5 s | 0,579 s | |
  | Histogram | 0,134 s | 0,6 s | 0,535 s | |
  | Komşuluk, 5 × 5 Ortalama | 0,440 s | 1 s | 1,915 s | |
  | Komşuluk, Daire (15) En büyük | 0,897 s | 2 s | | |
  | Komşuluk, 5 × 5 Ortanca | 0,538 s | 3 s | | |
  | Hücre istatistiği, beş raster, Ortalama | 0,929 s | 1,5 s | | |

  Ölçümle yapılan iyileştirmeler: şeridin girdi bloklarının tipli örneklere iş parçacıklarında açılması (her blok kendi satırlarına),
  sonucun karolarının ve Deflate'inin, katın yarılanmasının iş parçacıklarında yapılması, aynı kafesteki girdinin kaydırmayla (afinsiz)
  okunması, kayan pencerelerin tamponlarının yeniden kullanılması, hücre istatistiğinin istatistiğe göre ayrı yolları. Kalan payın çoğu
  Deflate'tedir (miniz); değiştirmek yeni bağımlılık ister, yapılmadı. Masaüstü:
  `cargo test --release -p kentos-raster --test all ops_timing -- --ignored --nocapture --test-threads=1`; web:
  `node scripts/perf/raster.mjs --only ops`.
- **Ölçülmeyenler:** p99 (az koşu); web'de kırpma, mozaik, ½× çift doğrusal, daire ve ortanca, hücre istatistiği; 64 girdiye yakın
  işler; dönük girdilerin şeritlerinin süresi.

## Kapsam dışı

- Raster bölgeler (bölge rasteriyle bölgesel istatistik), bölgesel histogram, ağırlıklı ve özel çekirdekli komşuluk, metre birimli
  pencere, koşullu işlevlerin ArcGIS adları (`Con`, `SetNull`; dilin `durum`'u ve `eğer`'i karşılar), çok bantlı sonuç veren ifade.
- Mozaiğin renk dengelemesi ve kesim çizgisi; yeniden izdüşüm (sistemler arası dönüşüm).
- Rasterleştirme ve rasterden vektör (`GIS-34`).
