# ADR 0171: Zemin, elipsoit ve projeksiyon düzlemi

- **Durum:** kabul edildi (2026-10-04). `HYB-14`. Sahibin kararları (4 Ekim): elipsoit değerleri GeographicLib'in Rust kapısıyla
  (`geographiclib-rs`); zemin değerleri projenin ortalama elipsoit yüksekliğiyle; Hesap pencerelerinde zemin ile düzlem arasındaki
  çeviri bir proje ayarıyla, kapalı başlar.
- **Bağlam belgesi:** TODOS.md `HYB-14` (ilgili: `NUM-04`, `HYB-13`, `HYB-11`), ADR 0167 (ikinci sistemin düzlemindeki ölçüler;
  elipsoit üstü ölçüler buraya bırakıldı), ADR 0168 (projenin tanımları: başlangıçlı TM, yerel sistem), ADR 0169 §3 (karne
  indirgemesi: yatay uzunluk ve kot farkı; projeksiyon ve yükseklik düzeltmeleri buraya bırakıldı), ADR 0149 (ölçü doğruluğu),
  CLAUDE.md §5 ve §23; Netcad Zemine İndirgeme, ArcGIS ground to grid correction ve Measure (planar, geodesic), QGIS ölçümde elipsoit.

## Bağlam

Mesafe ölç ve Alan hesapla bugün yalnız projenin düzlemindeki değeri verir (ikinci sistemin düzlemindeki de, ADR 0167 §2). Arazide
ölçülen uzunluk ise zemindedir: elipsoitten yükseklik kadar yukarıda, projeksiyonun ölçek bozulması olmadan. TM3'te ölçek
faktörü orta meridyende 1'dir ve dilim kenarında (1,5°) Türkiye'nin enlemlerinde 1,00019 ile 1,00022 arasına çıkar; 1 000 m
yükseklik uzunluğu yaklaşık 157 ppm uzatır. 100 m'lik bir kenarda bunlar 2 cm ve 1,6 cm eder: kadastro aplikasyonunda yok
sayılamaz.

Netcad'in Zemine İndirgeme'si ve ArcGIS'in ground to grid düzeltmesi bu iki çarpanı (projeksiyon ve yükseklik) uygular; ArcGIS
Measure ve QGIS düzlem ile jeodezik (elipsoit üstü) değeri ayrı verir.

## Karar

### 1. Üç değer

Bir uzunluk ya da alan üç düzeyde verilir:

- **Düzlem:** projenin koordinat sistemindeki değer (bugünkü).
- **Elipsoit:** köşeler projenin sisteminden coğrafiye çevrilir (ADR 0167'nin yolu; projenin datumunda kalınır, datum dönüşümü
  yoktur), düz kenar köşeleri arasındaki jeodeziktir (Karney 2013, `geographiclib-rs`). Uzunluk jeodeziklerin toplamı, alan
  jeodeziklerle ve yaylarla sınırlı bölgenin elipsoit üstündeki alanıdır. Yay düzlemde çizilmiştir: uzunluğu 0,1 mm sehimli
  parçalarının (ADR 0167 §2, ADR 0149) jeodeziklerinin toplamı, yayın düzlemdeki kesin uzunluğunun parçaların kirişlerine
  oranıyla; alanı köşelerin jeodezik çokgeninin (GeographicLib'in `PolygonArea`'sı) yanında yay ile kirişi arasındaki bölgedir:
  düzlemin alan ölçeğinin tersi o bölgede Gauss kurallarıyla tümlenir (kirişin ucundan yelpaze üçgenleri, 5°'lik dilimlerin
  kesimleri ağırlık merkezlerinde), düzlemdeki kiriş ile çokgenin aldığı jeodezik kiriş arasındaki ince parça düşülür
  ((2/3)·L·s, s jeodeziğin ortasının kirişe uzaklığı). Yayı 0,1 mm'lik parçalarla `PolygonArea`'ya vermek olmaz: onun alanı
  her kenarın ekvatora kadar olan alanlarının toplamıdır, yuvarlaması kenarla büyür (bir köşede bir ulp 400 m²'lik kareyi
  3·10⁻⁶ m², 1 000 parçalı 20 m'lik daireyi 3·10⁻⁴ m² oynatır).
- **Zemin:** elipsoit değeri projenin ortalama elipsoit yüksekliğine (h) büyütülür: uzunluğun her parçası s·(R_α + h)/R_α, R_α
  parçanın ortasında doğrultusundaki Euler eğrilik yarıçapı (§3); alan A·(M + h)(N + h)/(MN), M ve N alanın orta enleminde
  (en güney ve en kuzey noktasının ortası) meridyen ve birinci düşey eğrilik yarıçapları (yükselen yüzeyin alan çarpanı).

Yerel projede (SRID 0, projenin sistemi yok) yalnız düzlem vardır; başka satır yazılmaz (CAD'de her ölçümde aynı nedeni
söylemek gürültüdür). Projenin sistemi coğrafiyse düzlem değeri yoktur (derece). Yerel sistem tanımı (ADR 0168) tabanının TM'i
üstünden elipsoide gider.

### 2. Ortalama elipsoit yüksekliği proje ayarıdır

`groundHeight` (metre, isteğe bağlı; −500 ile 9 000 arası), Proje ayarları › Ölçme'de “Ortalama elipsoit yüksekliği”.
Ayar aynı zamanda anahtardır: yazılmadıkça Mesafe ölç ve Alan hesapla bugünkü gibi yalnız düzlemi (ve ikinci sistemi) söyler
(§4a). Noktaların kotları kullanılmaz: ortometrik mi elipsoit yüksekliği mi olduğu bilinmez (ADR 0169'un GNSS kuralı gibi),
yerine tahmin konmaz. `.kcad` şema 16: `survey`'in
`groundHeight`'ı ve §4'ün `reduceToGrid`'i (yalnız açıkken yazılır); yazıcı 16'yı yalnız bunlardan biri varken yazar.

### 3. Ölçek çarpanları

- **Noktanın projeksiyon ölçeği** k: TM'de Krüger serilerinin türeviyle (Karney 2011, çekirdeğin `geodesy::tm_scale`'i),
  benzerlikle bağlı yerel sistemde tabanının k'sı bölü düzlemin ölçeği. Coğrafide, Pseudo-Mercator'da (elipsoit üstünde
  meridyen ve paralel boyunca ölçeği ayrıdır) ve afinle bağlı yerel sistemde noktanın tek ölçeği yoktur; ölçülen yol ve alan
  için düzlem ile elipsoit değerlerinin oranı yine verilir.
- **Çizginin ölçeği:** iki ucun ve ortasının k'sıyla Simpson, (k₁ + 4kₘ + k₂)/6.
- **Yükseklik çarpanı:** R/(R + h), R çizginin doğrultusundaki eğrilik yarıçapı (Euler: MN/(N cos²α + M sin²α)).
- Düzlem uzunluğu = zemin uzunluğu × çizginin ölçeği × yükseklik çarpanı; ters yön bölmedir.

### 4. Hesap pencereleri

Proje ayarları › Ölçme'de “Uzunlukları projeksiyona indir” (`reduceToGrid`, kapalı). Açıkken Kutupsal alım ve Poligon hesabı
ölçülen yatay (zemin) uzunlukları düzleme çevirir (istasyonla hedefin çizgisinin ölçeği ve yükseklik çarpanıyla), Aplikasyon
düzlemdeki uzunluğu zemine çevirerek verir; pencereler ve raporları çarpanları yazar. Ortalama yükseklik yoksa ayar açılamaz,
nedeni söylenir. Kapalıyken bugünkü sonuçlar değişmez.

Hesap çekirdekte, iki geçişle: hedefin yeri uzunluğa, uzunluğun çarpanı hedefin yerine bağlıdır. Kutupsal alımda nokta önce
ölçülen uzunlukla bulunur, istasyondan ona çizginin çarpanları (§3) alınır, uzunluk indirilir ve nokta yeniden bulunur; bir
geçiş daha yapılır (ikinci geçiş 1 km'lik uzunluğu dilim kenarında bir mikrometre kadar değiştirir, üçüncüsü ölçülemez). Poligon hesabında bütün
poligon önce ölçülen uzunluklarla hesaplanır, her kenarın çarpanları geçici noktalarından alınır, uzunluklar indirilir ve poligon
yeniden hesaplanır; bir kez daha (ilk geçişin noktaları indirgemenin kendisi kadar, kilometrede birkaç desimetre kayıktır;
dengeleme son geçişte, indirilmiş uzunluklarla). Yönler düzlemdedir (bakılan bilinen noktaların doğrultusu); doğrultuya
yay-kiriş düzeltmesi yapılmaz (1 km'lik kenarda, dilim kenarında 1″'den küçük). Aplikasyon'da her hedef için düzlemdeki uzunluğun
yanında zemindeki verilir: düzlem ÷ (ölçek × yükseklik çarpanı). Çekirdeğin çağrılarına isteğe bağlı `grid` (projenin sistemi ve
ortalama yükseklik) eklenir; yoksa sonuçlar bugünküdür, bit bit.

Ayar, Ölçme'nin Zemin grubunda “Hesap pencereleri” satırındaki anahtardır. Üç koşulla açılır, açılamadığında satırın açıklaması
nedenini söyler: ortalama yükseklik yazılmış (“Ortalama elipsoit yüksekliği yazılınca açılır.”), projenin bir koordinat sistemi
var (yerel projede yok) ve sistemde bir noktanın tek ölçeği var: TM ya da ona benzerlikle bağlı yerel sistem; coğrafi sistem,
Pseudo-Mercator (elipsoit üstünde açı korumaz) ve afinle bağlı yerel sistem doğrultuya göre değişen ölçekleriyle dışarıda kalır
(`has_point_scale`, `crsHasPointScale`). Koşul tutmazken kaydedilen ayar `reduceToGrid`'i yazmaz. Pencereler indirgeme yaptığında
tabloları zemin ve düzlem uzunluklarını yan yana, çarpanla (ölçek × yükseklik çarpanı) gösterir; raporlar ölçeği ve yükseklik
çarpanını ayrı sütunlarda yazar; özet bir satırla yüksekliği ve ayarın yerini söyler (Aplikasyon'da “Arazide zemindekini ölçün.”).
Kural iki platformda aynıdır (`kentos_interaction::ground::{survey_grid, why_not_grid}`, `model/groundMeasures.ts`'in `surveyGrid`,
`whyNotGrid`'i).

### 4a. Mesafe ölç ve Alan hesapla

Projenin sistemi ve ortalama elipsoit yüksekliği varken Mesafe ölç'ün toplamından ve Alan hesapla'nın sonucundan (İçine
tıkla dahil) sonra komut geçmişine, ikinci sistemin satırından (ADR 0167 §2) önce, iki satır yazılır:

- `Elipsoit üstünde: Toplam uzunluk 69.994 m   Ölçek 1.00009093` (alanda `Alan 999.82 m²   Çevre 129.988 m   Ölçek …`);
  coğrafi projede ölçek yoktur.
- `Zeminde (h = 850 m): Toplam uzunluk 70.003 m   Yükseklik çarpanı 0.99986672` (alanda `Alan …   Çevre …   Yükseklik
  çarpanı …`).

Yüksekliği yazılmamış projede bu satırlar gelmez: durum çubuğu son iletiyi gösterir, ölçümün kendi sonucu orada kalmalıdır
(her ölçümde “yükseklik yok” demek de gürültüdür; ayarın kendi açıklaması satırları söyler). Uzunluk ve alan projenin
basamaklarıyla, çarpanlar sekiz basamakla (ADR 0149'un kuralı). Ölçülen yerin bir noktası sisteme ulaşmıyorsa elipsoit satırı
bunu söyler, zemin satırı yazılmaz. Sabit ilk noktalı ölçümün ışınları yalnız düzlemi (ve ikinci
sistemi) söyler; ışın ışın zemin uzunluğu Aplikasyon'undur (§4). Satırların kuralı iki platformda aynıdır
(`kentos_interaction::ground`, `model/groundMeasures.ts`); ortak iz `ground-measures.json`.

### 5. Ortak çekirdek ve başvuru

Hesaplar geometri çekirdeğinde (`crs::ground`; web'e WASM ile), iki platform aynı kodla ve aynı bitlerle. `geographiclib-rs`
Rust'ın kendi kayan nokta yöntemlerini (`x.sin()`, `y.atan2(x)`) çağırır: masaüstünde glibc'ninkiler, tarayıcının WASM'ında
`libm`'in Rust kapısınınkiler. İkisi son bitte ayrılır, jeodezik ve alan bunu büyütür: ilk ölçümde 346 yanıttan 321'i ayrıldı,
alan 6·10⁻⁷ (göreli), uzunluk 10⁻⁷ m kadar. Çekirdeğin kuralı (ADR 0008: `libm` ile her hedefte aynı bitler) için kitaplık,
crates.io'daki 0.2.7 sürümünden yazılan bir kopyayla gelir (`crates/shared/geographiclib-rs`, çalışma alanının üyesi değil):
arşivi Cargo'nun kilitlediği SHA-256 ile denetlenir, aşkın yöntem çağrılarının (51 yer: `sin`, `cos`, `sin_cos`, `atan2`,
`hypot`, `cbrt`, `atanh`, `atan`) her biri `libm`'e giden bir özelliğin yöntemi olur, başka hiçbir şey değişmez
(`scripts/vendor/geographiclib.py`, `--check`). Bunun bedeli Karney'nin resmî sınama kümesinde (GeodTest, 500 000 jeodezik)
ölçüldü: ters problemde en büyük uzunluk hatası yamasız sürümde 7,5 nm, kopyada 11 nm (GeographicLib'in bildirdiği doğruluk
15 nm), 10 km'den kısa jeodeziklerde ikisinde de 2,8 nm; düz problemde enlem 8,7 ve 9,5 nm, boylam ikisinde 12,6 nm. Sürümün
glibc'nin son bitine bağlanmış dört birim testi (`assert_eq!`) kopyada bir ulp farkla düşer. Çekirdeğin yanıtları
`fixtures/geodesy/v1/ground-answers.json`'da dondurulur; web onları bit bit verir. Bağımsız başvuru PROJ ve GeographicLib'in C kitaplığıdır (pyproj: `Geod.inv`, `Geod.fwd`,
`Geod.npts`, `Proj.get_factors`), çarpanlar ve kesin düzlem değerleri için mpmath; alan, sınır sıklaştırılarak (jeodezikler
1 m'de bir) PROJ'un elipsoit üstündeki Lambert eşit alanlı azimutal izdüşümünde hesaplanır. Yaysız alanlarda bu yol
GeographicLib'in köşe çokgeniyle onun yuvarlama payı içinde uyuşur.

## Adımlar

1. Çekirdek: noktanın ve çizginin ölçeği, yükseklik çarpanı, jeodezik uzunluk ve elipsoit alanı, üç değer (`crs::ground`:
   `point_scale`, `line_scale`, `line_factors`, `ground_measures`; çağrılar `crsPointScale`, `crsLineFactors`,
   `crsGroundMeasures`); `geographiclib-rs` (sahibin onayı, 4 Ekim) `libm`'li kopyasıyla (§5); bağımsız başvuru
   `ground_cases.py` (`fixtures/geodesy/v1/ground.json`), dondurulmuş yanıtlar `ground-answers.json`.
2. Mesafe ölç ve Alan hesapla'nın satırları iki platformda (§4a); `groundHeight` proje ayarı ve Ölçme'deki alanı; `.kcad` şema 16
   (`groundHeight`, `reduceToGrid`).
3. Hesap pencereleri: `reduceToGrid`, Kutupsal alım, Poligon hesabı, Aplikasyon. 3a çekirdek: `crs::ground::Grid` ve
   `grid_factor`, `survey::polar` (`PolarInput.grid`, `PolarPoint.grid`, `scale`, `heightFactor`; Aplikasyon'un `ground`'u),
   `survey::traverse` (`TraverseInput.grid`, kenarın `ground`'u ve çarpanları); bağımsız başvuru `ground_survey_cases.py`
   (`fixtures/geodesy/v1/ground-survey.json`: bilinen düzlem noktaları, PROJ ve GeographicLib'in çarpanlarıyla zemin uzunlukları).
   3b: ayar ve pencereler iki platformda: Ölçme'de anahtar ve koşulları, Kutupsal alım, Aplikasyon ve Poligon hesabının tabloları,
   özetleri ve raporları (masaüstü `calc/{polar,stakeout,traverse}.rs`, web `ui/calc/{Polar,Stakeout,Traverse}Dialog.ts`);
   resimler `hesap-*-zemin`, `olcme-ayar-indir` ve web'in `ground` grubu. Bitti (5 Ekim).

Her adım iki platformda, ortak fixture'larla, kendi commit'inde ilerler.

## Doğrulama

- Ölçek çarpanları PROJ'un `get_factors`'ıyla (sayısal türev: 1e-10), jeodezik uzunluk GeographicLib'in C kitaplığıyla (1e-6 m),
  alan eşit alanlı izdüşümdeki sık sınırla (çevrenin her 100 m'si için 1e-6 m² ve köşe çokgeninin yuvarlama payının dört katı:
  her koordinatı bir ulp oynayınca `PolygonArea`'nın en çok oynayışlarının toplamı, `areaNoise`; çekirdeğin enlem ve boylamları
  kendi izdüşümünündür, PROJ'unkinden birkaç ulp uzak); yükseklik çarpanı ve zemin değerleri mpmath'le. 300 rastgele yol ve alanla
  (yaylı; sekiz sistemde ve onlara bağlı yerel sistemlerde) çekirdek bir kez taranır.
- Arayüz: ortak izler ve resimler iki platformda.
