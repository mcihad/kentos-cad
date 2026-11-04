# 0924 — KentOS Yol: en küçük taşıma momentiyle taşıma planı

Tarih: 2026-11-04. plan/road.md Y23 (ROAD-HAUL-02, ROAD-HAUL-07). Durum: kabul, açıklar aşağıda.

## Dayanak

- **Civil 3D:** Brüknerin dengeleme seçeneklerinde serbest taşıma uzaklığı verilir. Yol boyunca ödünç ve döküm
  alanları km'leri ve hacimleriyle (capacity) eklenir. Eğri bu alanlarla dengelenir; alanların hacmi hacim olarak
  yazılır ([Balancing Options Tab](https://help.autodesk.com/cloudhelp/2024/ENG/Civil3D-UserGuide/files/GUID-9B2D8FA7-1C06-45BD-9B4B-2B1CCD7FEB06.htm),
  [Exercise 2: Balancing Mass Haul Volumes](https://help.autodesk.com/cloudhelp/2016/ENU/Civil3D-Tutorials/files/GUID-A23CD8A8-7C49-489F-91FD-439CAD91B4FB.htm);
  4 Kasım 2026'da okundu). Planı Brüknerin döngüleri verir; en küçük maliyetli dağıtım yapılmaz.
- **Trimble Business Center:** koridor ve saha kütle taşıması kesilen malzemenin, ödünç alanlarının ve döküm
  alanlarının boş hacminin en verimli kullanımını arar. “Respect earthwork site capacities” işaretliyse alanların
  hacmi sınırdır ([Create a Corridor Mass Haul Analysis](https://help.fieldsystems.trimble.com/tbc/10433.htm),
  [Understanding Earthworks Sites](https://help.fieldsystems.trimble.com/tbc/19935.htm)).
- **Softree Optimal:** toprak işinin en düşük maliyetli dağıtımını matematiksel eniyilemeyle bulur
  ([Mass haul diagram](https://support.softree.com/knowledge-base/optimization/optimal-haul/mass-haul-diagram)).
  Toprak işi dağıtımı yazında doğrusal programdır (ulaştırma problemi).
- **OpenRoads Designer:** Bentley'nin çevrimiçi belgesinde Brükner diyagramının tanımı vardır (OpenRail Designer
  yardımı). Eniyileme belgesi bulunamadı.
- **Netcad:** NETPRO'nun Brükner'i ADR 0907'de. Çevrimiçi resmî eniyileme belgesi bulunamadı.

## Karar

### §1 Hesap

`kentos_road::haul_plan`. Kütle ordinatının her aralığının net hacmi aralığın ortasına konur: yarma fazlası arz,
dolgu fazlası talep. Brüknerin ödünç ve döküm alanları km'lerinde durur; hacimleri kapasitedir. Toprak yol boyunca
komşu yerler arasında akar, metreküpü gittiği uzaklık kadar tutar.

Her yarma yerleşmeli, her dolgu karşılanmalıdır. Ödünç alanı en çok kapasitesi kadar verir, döküm alanı en çok
kapasitesi kadar alır. Toplamlar yetmezse plan kurulamaz. Eksik dolgu (`shortage`) ya da artan yarma (`surplus`)
söylenir; hiçbir şey taşınmaz.

En küçük maliyetli akış ardışık en kısa yollarla tam bulunur: kalan ağda Bellman–Ford. Arzı yerleştirmek ve talebi
karşılamak her taşımadan çok kazandırır (−(2·boy+1)). Böylece kurulabilen plan her şeyi yerleştirir; bunların içinde
moment (Σ|akış|·uzaklık, m³·m) en küçüğüdür. Bir doğruda parçaların akışı yerlerin verdiği ve aldığından çıkar.

Serbest taşıma, maliyet ve yönlü taşıma bu planda yoktur. Brüknerin döngüleri (ADR 0907) eğriyi sürekli okur; plan
her aralığın hacmini ortasında toplar. Bu yüzden iki moment aralık boyu kadar ayrılabilir.

### §2 Komut

`road.hauls.plan` v1 yalnız okur. Girdi: Brükner (kimlik ya da ad). Cevap:

- planın kurulup kurulmadığı;
- yarma ve dolgu fazlası;
- taşınan hacim (yarma ve kullanılan ödünç);
- moment ve ortalama uzaklık (moment / taşınan);
- her alanın kullanımı;
- komşu yerler arası akışlar (ileri artı);
- eksik dolgu ve artan yarma.

Ret: `not_a_road_project`, `unknown_haul`, `invalid_haul` (kübaj hesaplanamıyor, hacimli aralık yok, alan
kübajın km'leri dışında). Kübajı olmayan Brükner proje ayarlarında yazılamaz. Uyarı: `haul_plan_infeasible`, ayrıca
kübajın `corridor_stale` ve `volume_gap` uyarıları.

### §3 Masaüstü

Koridor sekmesinin Brükner panelinde Taşıma planı penceresi bulunur. Akış yol boyunca bant olarak çizilir: ileri taşınan
mavi, geri taşınan turuncu. Ödünç alanı yeşil, döküm alanı kırmızı üçgendir. Yanda toplamlar, moment, ortalama ve
alanların kullanımı (kullanılan / kapasite) listelenir. Plan kurulamazsa nedeni söylenir.

## Doğrulama

26 ortak durum (`scripts/fixtures/road_haul_plan_cases.py`). Çizim, koridorlar, kübajlar ve Brüknerler
road_haul_command_cases.py'nindir; dolgu fazlası olan üçüncü kübaj, kesme katsayısı 0,1 ve dolgu katsayısı 5'tir. Plan
HiGHS (scipy `linprog`) doğrusal programıdır, KentOS kodu kullanılmaz. Her alanın kullanımı aynı momentteki
planların en azı ve en çoğuyla tek olduğu denetlenir. Sınırdaki değer sınıra oturtulur, kalanı dengeden bulunur.
Akışlar kesirlerle çıkar.

Durumlar:

- kurulamayan planlar: yarma fazlası, dolgu fazlası, 0,004 m³ eksik alan;
- tek, iki ve sınıra oturan döküm alanı;
- ödünç ile döküm birlikte, kullanılmayan uzak ödünç;
- kesitsiz aralıklı ve prizmoidal kübaj;
- plan ve doğrulama;
- dört ret.

Rust ve Python'da oynatılır. Masaüstü penceresinin testi ve resimleri `.run/gui-evidence/yol-y23-tasima-*`'dir.

## Açıklar

- ROAD-HAUL-02'nin kalanı: bölgeler, işleme alanı, engel, taşıma yolu ve yön. Yol dışı taşıma yolu ağı (Trimble'ın
  haul road'u) ve köprü ya da kapalı yol (ROAD-HAUL-05, 06).
- Serbest taşımalı ve fiyatlı maliyet amacı: fazla taşıma birim fiyatı ve ödünç ile döküm fiyatıyla en küçük maliyet.
- Malzeme türü, uygunluk ve dönüşüm verimi (ROAD-HAUL-04, 09).
- İş takvimi ve eskime (ROAD-HAUL-08).
- Planın Brükner penceresinde ve çıktılarda kullanılması (ROAD-HAUL-11, 12).
- ROAD-HAUL-07'nin raporu: tam algoritmanın adı ve sürümü cevapta yazılmaz; plan tam en küçüktür, sezgisel değildir.
