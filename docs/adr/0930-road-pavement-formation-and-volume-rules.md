# 0930 — KentOS Yol: üstyapı tabakaları, formasyon ve kübaj kuralları

Tarih: 2026-11-04. plan/road.md Y26 (ROAD-COR-01, 11; ROAD-VOL-09, 19). Durum: Y26.1 kabul; Y26.2–Y26.4 bu belgeyle
ilerler, açıklar aşağıda.

## Dayanak

- **Civil 3D:** LaneSuperelevationAOR şeridi bitmiş yüzün altına dört tabaka koyar (Pave1, Pave2, Base, SubBase);
  kalınlıklar ayrı parametredir, sıfır tabakayı kaldırır. Tabakalar bitmiş yüze paraleldir, şeridin iki ucunda düşey
  bağlar şekli kapatır. SubBase'in altı `Datum` kodludur ([LaneSuperelevationAOR](https://help.autodesk.com/cloudhelp/2025/ENU/Civil3D-Subassembly-Ref/files/GUID-98C93726-64C9-42D9-8213-9CBCD95CBFA9.htm),
  4 Kasım 2026'da okundu; resimleri incelendi). ShoulderExtendAll banketin altında tabakaları sürdürür. Kenar ya
  “Hold Slope” ile banket menteşesinden belirli şevle inen çizgidir, tabakalar ona kadar uzar, ya da “Hold Width” ile
  belirli genişliktir ([ShoulderExtendAll](https://help.autodesk.com/cloudhelp/2025/ENU/Civil3D-Subassembly-Ref/files/GUID-904838B2-5B98-42CB-A605-75D1747792CB.htm)).
  Hacim Compute Materials ile hesaplanır: Earthworks ölçütünde EG ile koridorun `Datum` yüzeyi karşılaştırılır.
  Tabakalar Material List ölçütünde malzeme malzeme sayılır ([Creating a Material List](https://help.autodesk.com/cloudhelp/2026/ENG/Civil3D-Tutorials/files/GUID-DAC90690-3DC4-400E-8A5C-104205DE7C30.htm)).
- **OpenRoads Designer:** tip kesitte üstyapı “simple component”lerdir. Şev “end condition”dır. Miktarlar özellik
  tanımına göre gruplanır. Her asfalt karışımı ayrı tanımda olmazsa raporda ayrılmaz (FHWA ORD kılavuzu, bölüm 20).
  Kazı/dolgu ağ yüzeyinin hacim seçenekleriyle yapılır (Design, Existing, Subgrade, Cut, Fill, Unsuitable). Üstyapı
  hacmi için bazı kurumlar hesabı iki kez yapar, üst yüzeye ve formasyona, sonra farkı alır (NCDOT Pavement Structure
  Volume). Bentley belgelerinin bir kısmı bu oturumda açılamadı (403, bağlantı koptu).
- **Netcad NETPRO:** kübaj kuralları adlıdır: Yol–Zemin, Formasyon–Zemin, Yol–Bitkisel Toprak, Formasyon–Bitkisel
  Toprak, Sıyırma Kazısı (plan/road.md §3, 2 Kasım araştırması). NETPRO 5 kitabında enkesit makrolarında FORMASYON
  makrosu vardır. Alttemel yüzeyini taşıyan FORMAS.KSP dosyası kübajda kullanılır. Netcad wiki'sinin sertifika zinciri
  bu oturumda doğrulanamadı. Sayfalara doğrulamasız bağlanılmadı.
- **KGM:** Karayolu Teknik Şartnamesi üstyapı tabakalarını (aşınma, binder, bitümlü temel, plentmiks temel, alttemel)
  tip enkesitte gösterir. Kalınlıklar projenin ve standart profilinindir (ROAD-DEC-01). KentOS sayı vermez; örneklerdeki
  kalınlıklar örnektir.

## Karar

### §1 Çekirdek

Kod `kentos_road::section::pavement` içindedir. Üstyapı tip kesitin bir parçasıdır. Tabakaların her birinin kalınlığı
vardır (yukarıdan aşağı) ve iki yanda aynıdır (aynalı). Her tabaka, bitmiş yüzün üstündeki tabakaların derinliği ile
kendi derinliği kadar indirilmiş hâlleri arasındaki bölgedir.

Kenar iki türlüdür:

- `Vertical`: menteşede düşey çizgi.
- `Slope(h)`: tabakalar son bileşenin eğimiyle dışa sürer ve menteşeden 1 düşeye h yatay inen çizgiye kadar uzar.
  h dolgu şevine eşitse üstyapının kenarı dolgunun yüzüdür.

Kurallar:

- Üstyapı bitmiş çizginin üstüne çıkmaz. Hendeğin ön şevi kenardan hızlı iniyorsa tabakaları keser.
- Üstyapı kesitin izinin (bitmiş çizginin araziyle buluştuğu yerin) dışına taşmaz. Orada kübaj ölçülmez. Ölçülseydi
  bitkisel toprak sıyrılmış yarmada dolgu eksiye düşerdi (referansın yakaladığı durum).

Formasyon son tabakanın tabanıdır. Kenarlar, uçlarından menteşelere çıkan çizgilerdir.

Her alan, ofset boyunca parça parça doğrusal fonksiyonların min ve max'ı arasındaki farkın integralidir. Köşeler
arasındaki her aralıkta zarflar doğrusal parçalıdır. İki doğrunun kesiştiği ve farkın sıfırlandığı yerlerde bölünür,
bu yüzden sonuç tamdır.

Formasyondan kübaj (Formasyon–Zemin) yolunkinden çıkar:

- kazı = yolun kazısı + üstyapının zeminin altındaki alanı;
- dolgu = yolun dolgusu − üstyapının zeminin üstündeki alanı.

Üstyapı bitmiş çizginin altında ve izin içinde olduğundan bu iki eşitlik tamdır. Bitkisel topraklı kurallarda zemin,
sıyırma kalınlığı kadar indirilmiş zemindir (ADR 0906 §6).

Retler:

- tabaka yok ya da kalınlık sıfır ve altı (`Layer`);
- kenar şevi sıfır ve altı ya da son bileşenin eğiminden yatık, tabakalar onu hiç kesmez (`Edge`);
- bileşensiz yan (`NoPlatform`).

### §2 Kayıt ve komut (Y26.2)

- Tip kesit tanımının `pavement` alanı: `layers[]` (ad, kalınlık m, isteğe bağlı malzeme kodu) ve `edge`
  (`vertical` | `slope` + `h`). `.kcad`'de alan isteğe bağlıdır, üstyapısız tip kesit değişmez.
- `road.templates.edit` alanı yazar; `road.sections.query` kesit başına tabaka alanlarını ve formasyonu verir.
- Kübajın `rule` alanı NETPRO'nun adlarını taşır: `roadGround`, `formationGround`, `roadTopsoil`, `formationTopsoil`.
  Sıyırma kazısı Y8.4'teki gibi ayrı kalemdir. Bitkisel topraklı kural sıyırma tanımı ister. Kuralın adı, karşılaştırdığı
  iki çizgi ve zemin durumu cevapta yazılır.
- Kübaj cetveline tabaka tabaka hacim sütunları gelir (ortalama alan, kesitlerin tabaka alanlarından).

### §3 Masaüstü (Y26.3)

- Tip kesit penceresinde Üstyapı bölümü: tabaka tablosu (ad, kalınlık, malzeme), Kenar (Dik | Şevli, h). Pencere
  Y26.0'ın ortak düzenindedir.
- Enkesit görünüşünde tabakalar ince çizgiyle, formasyon kalın kesikliyle çizilir.
- Kübaj penceresinde Kübaj kuralı seçimi ve tabaka hacimleri.

### §4 Pencere düzeni (Y26.0)

Sahibin isteği: “Dialog form tasarımları çok karmaşık oluyor, onları da daha derli toplu hale getir.” Yol pencereleri
`road_dialog::RoadDialog` iskeletini kullanır (DESIGN.md §7.19):

- solda gruplu form;
- sağda çizim, en çok dört özet sayı, notlar ve tek tablo, hangi listenin gösterileceği parçalı seçimle;
- alt çubukta durum, Kapat ve tek birincil eylem.

Yalnız okuyan pencere değer değişince kendiliğinden yeniden okur, “Önizle” yoktur. İlk taşınan pencere Kırmızı kot
önerisi'dir. Öbürleri sırayla taşınır.

## Doğrulama

19 ortak durum (`scripts/fixtures/road_pavement_cases.py` → `fixtures/road/v1/pavements.json`). Referans kesirlerledir;
kesiti road_template_cases.py'nin başvurusu kurar. İntegral bütün parçaların ikişer ikişer kesişimleri kaba kuvvetle
kırılma noktası alınarak hesaplanır (zarf kullanılmaz). KentOS kodu kullanılmaz.

Durumlar:

- dolgu ve yarmada dik ve şevli kenar;
- hendeğin kestiği ve kesmediği kenar;
- arazinin üstyapıyı kestiği yamaç ve arazinin üstyapının içinden geçtiği durum;
- dever, boşluklu arazi, asimetrik kesit, büyük kotlar, tek tabaka;
- sıyrılmış zeminde Formasyon–Bitkisel Toprak;
- beş ret.

Rust testi `crates/shared/road/tests/all/pavements.rs`'tir.

## Açıklar

- Kenarın “Hold Width”i (belirli genişlik), yanlara göre farklı üstyapı, şeritte ve bankette farklı tabaka kümesi
  (banket altında yalnız temel ve alttemel).
- Tabakaların koridor yüzeyleri (her tabaka tabanının üçgen yüzeyi) ve yüzeyden kübaj (TIN bindirme) Formasyon
  kurallarıyla.
- Malzeme kodunun poz defterine (ADR 0909, Y11.4) bağlanması ve tabaka metrajının ton/m² birimleri.
