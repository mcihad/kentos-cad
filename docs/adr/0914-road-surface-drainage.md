# 0914 — KentOS Yol: yüzey drenajı

Tarih: 2026-11-04. plan/road.md Y16 (ROAD-DRAIN-03, 05; menfez hidroliği ROAD-DRAIN-02'nin kalanı). Durum: taslak,
aşamalar aşağıda.

## Dayanak

- **Civil 3D:** Water Drop bir noktadan yüzeyde suyun yolunu çizer; Create Catchment From Surface bir boşaltım noktası
  ister ve havzayı, boşaltım noktasından hidrolik olarak en uzak noktaya akış yolunu birlikte kurar; akış yolu sekmesinde
  geçiş süresi TR-55 ya da kullanıcının değeri, havzada Rasyonel yöntemin akış katsayısı (0,01–1,00; Civil 3D kullanmaz,
  havzayla dışarı verir). Küçük bir yüzey düzensizliği ya da “düz kenar” komutu durdurabilir; önce Water Drop ile ilgilenilen
  yere yakın bir yol bulup boşaltım noktasını onun üstünde seçmek önerilir. Havza yüzey değişince kendiliğinden
  güncellenmez
  ([To Create Catchments From Surfaces](https://help.autodesk.com/cloudhelp/2024/ENG/Civil3D-UserGuide/files/GUID-C3F42F30-3022-41C4-AAB4-C4F1E862AB78.htm),
  [Create Catchment From Surface Dialog Box](https://help.autodesk.com/cloudhelp/2027/ENG/Civil3D-UserGuide/files/GUID-21BC8ABE-8B63-40B8-9D0A-BEEA09D4009E.htm),
  [To Create a Flow Path From a Surface](https://help.autodesk.com/cloudhelp/2024/ENG/Civil3D-UserGuide/files/GUID-6A28106A-4498-4337-AF33-D586975F4392.htm),
  4 Kasım 2026'da okundu).
- **OpenRoads Designer:** Terrain › Analysis'te Analyze Trace Slope (Maximum Slope: en dik iniş, düşük noktada ya da
  modelin kenarında biter; Constant Slope; yukarı ve aşağı), Minimum Depth sığ çukurları ayıklar (0 kullanılmaz); Analyze
  Pond bir düşük noktanın göletini (hacim, alan, en büyük derinlik) verir; Analyze Catchment aynı grupta
  ([Analyze Trace Slope](https://docs.bentley.com/LiveContent/web/OpenRoads%20Designer-v2024.0.2/Help/en/topics/404806/GUID-98E9D3AC-1588-4EE2-9D5C-43B4689776B2.html),
  [CTDOT, Drainage and Utility Modeling](https://portal.ct.gov/DOT/Engineering-Applications/CTDOT-CONNECT-DDE-eBook-Volume-005---OpenRoads-Designer-Drainage-and-Utility-Modeling/Section-1---Introduction)).
- **Netcad:** Netpro'nun drenaj hesabı için çevrimiçi resmî kılavuz bulunamadı; menfez tasarımı eğitim videolarında
  çizim akışı olarak görünür. Türkiye'de küçük havzalarda Rasyonel yöntem yaygındır.
- **Matematik:** üçgenin düzlemi z = a x + b y + c, en dik iniş −∇z = (−a, −b) yönündedir ve üçgen içinde sabittir;
  yol üçgenden çıktığı kenarda karşıdaki üçgenin iniş yönü kenardan uzaklaşıyorsa onda sürer, kenara geri dönüyorsa
  (vadi) kenar boyunca alçak ucuna iner; köşede en dik çıkış ya iniş yönü köşenin açısı içinde kalan üçgen ya da alçalan
  kenardır. Gölet seviyesi kenardan öncelikli taşkınla bulunur (Barnes'ın priority flood'ı: köşenin seviyesi kenara giden
  yollardan geçilen en yüksek köşenin en küçüğü). Bir üçgende su derinliği doğrusal olduğundan seviyeyle kırpılan
  çokgenin hacmi alanı çarpı ağırlık merkezindeki derinliktir. Manning: Q = A R^(2/3) S^(1/2) / n (SI). Rasyonel yöntem:
  Q = C i A / 3,6 (A km², i mm/sa, Q m³/s). Pürüzlülük, akış katsayısı ve yağış şiddeti kullanıcınındır; KentOS değer
  vermez.

## Karar

1. **Çekirdek `kentos_road::drainage` (Y16.1).** `Tin` (tutulan üçgenlerin komşuları, köşelerin üçgenleri ve komşu
   köşeleri, yüzeyin kenarındaki köşeler); `water_drop` (yol, plan boyu, bitişi: düşük nokta, düz, kenar, yüzeyin dışı);
   `spill_levels` ve `ponds` (dip, seviye, taştığı köşe, derinlik, alan, hacim, köşe sayısı; en az derinlikle); `catchment`
   (bir ızgaranın hücrelerinden su damlası boşaltım noktasının eriminden geçenler: hücre sayısı, alan ızgaranın
   çözünürlüğünde, dış hatları, en uzak hücreden akış yolu, boyu ve düşüsü); `manning` (dolu boru, dolu kutu, yamuk
   hendek) ve `rational`.
2. **Komut (Y16.2).** `road.drainage.query`: bir yüzeyde göletler, verilen noktalardan su damlaları, bir boşaltım
   noktasının (ya da menfezin girişinin) havzası ve Rasyonel debisi, menfezin dolu akış kapasitesiyle karşılaştırması;
   `road.drainage.draw`: havzanın sınırı, akış yolu ve göletlerin dipleri bir katmana.
3. **Masaüstü (Y16.3).** Drenaj penceresi: yüzey, boşaltım noktası ya da menfez, ızgara aralığı ve erim, C ve i; planda
   havza, akış yolu ve göletler; debi ile kapasite.

Açık: hendeğin boyuna kapasitesi (koridorun tip kesitinden yamuk ve boykesitin eğimi), geçiş süresi yöntemleri (TR-55),
dış hidrolik çözücü bağlantısı (HEC-RAS, SWMM; ROAD-DRAIN-05…07), taşkın kapanma kuralı (ROAD-DRAIN-08), yeraltı ağı
çakışması (ROAD-DRAIN-09).

## Aşamalar

Y16.1 (yapıldı, çekirdek): §1. `crates/shared/road/src/drainage.rs`, testleri `tests/all/drainage.rs`: düzlemde damla
eğimin tersine kenara (yön ve boy kapalı biçimle), V vadisinde vadiye ve boyunca batı ucuna, 400 hücrenin her birinin
damlası kapalı biçimin söylediği yerde biter (vadiye 4 x₀ ≥ |y₀ − 5| ile, değilse kenarda |y₀ − 5| − 4 x₀ ötede; üçgen
kenarındaki başlangıçlar dahil), koni çanakta gölet kenara kadar (dip 0, seviye 5), kare çukurun hacmi her üçgenin alanı
çarpı ağırlık merkezindeki derinlikle aynı, vadinin çıkışının havzası kapalı biçimin hücreleri, Manning ve Rasyonel elle.
Testlerin bulduğu: başlangıcı bir üçgenin kenarında olan ve en dik inişi komşuya giden damla “düz” diye duruyordu; kenarı
geçer, köşedeyse köşeden sürer.

Y16.2 (yapıldı, komutlar): §2. `road.drainage.query` (`crates/native/application/src/road_drainage.rs`): göletler,
damlalar (yol, son yer, boy, bitiş), havza (boşaltım noktası `at` ya da menfezin girişi, hücreler, alan, dış hatlar, akış
yolu, boy, düşü, ortalama eğim; C ile i verilince Rasyonel debi; menfezle her gözün ya da borunun dolu akış kapasitesi
Manning'le, eğim menfezin, pürüzlülük menfezin; debinin kapasiteye oranı); uyarılar `drainage_catchment_empty`,
`culvert_no_roughness`, `culvert_over_capacity`. `road.drainage.draw` havzanın dış hatlarını, akış yolunu, göletlerin
diplerini ve isteğe bağlı yazıları tek adımda (“Drenaj çiz”) çizer; çizilecek bir şey yoksa `drainage_nothing`. Yeni
birimler `discharge` (m³/s), `velocity` (m/s), `rainfall` (mm/sa). 28 + 24 ortak durum
(`scripts/fixtures/road_drainage_cases.py`: menfezin çizimine eklenen üç kafes arazi, her karesi düzlem: V vadide damlanın
yeri ve boyu, havzanın hücreleri ve en uzak akış yolu kapalı biçimle; elmas çukurda gölet 5 m derin, 50 m², 250/3 m³;
düzlemde kenara; Manning ve Rasyonel ADR'nin formülleriyle; menfezlerin ölçüleri road_culvert_cases.py'nin; KentOS kodu
yok). Python `kentos.cad.road.drainage`, MCP.
