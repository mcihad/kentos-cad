# ADR 0147: Yeni ölçü türleri

- **Durum:** kabul edildi (2026-10-01). Yön sahibin kararıdır: biçim değişiklikleri özellikleriyle gelir; sıra köşe kotu, çok parçalı alan, blok, yazı ekleri, kılavuz, yeni ölçü türleri. Sahibin istekleri (1 Ekim): “Açı ölçüsü de ekle”, “Başka ne ölçüler olabilirse ekle”, yeni ölçüler şeritte. Hangi ölçülerin geldiği ve ayrıntılar bu ADR'nin varsayılanlarıdır.
- **Tarih:** 2026-10-01
- **Bağlam belgesi:** ADR 0061 (ölçülendirme), ADR 0140 (zincir ve baz ölçü; “sözleşmede yeni nesne türü isteyenler”), ADR 0142 (köşe kotu), ADR 0145 (yazının zemini; “ölçü yazısının hizası ve zemini yeni ölçü türlerinin adımında”), ADR 0146 (şema 8), ADR 0025 (KCAD v2), ADR 0066 (Öznitelikler), ADR 0009 (DXF); `docs/specs/kcad-v2.md` §6.6.

## Bağlam

KentOS'ta beş ölçü vardır (ADR 0061):
- hizalı;
- doğrusal (ΔY yatay ya da ΔX düşey);
- açı (iki kenardan, ya da köşe ve iki noktadan);
- yarıçap;
- çap.

Zincir ve baz ölçü bunlardan yeni ölçüler dizer (ADR 0140). Harita, kadastro, yol ve imar çizimlerinde başka ölçüler de gerekir; bugün kullanıcı onları çizgi ve yazıyla elle çizer, nesne değişince yazı eskisini söyler:

- **Koordinat ölçüsü:** bir noktanın Y'si ya da X'i, noktadan çıkan bir çizginin üstünde (yapı köşesi, rögar, sınır taşı). AutoCAD'de DIMORDINATE, Türkçesiyle “Koordinat”; Netcad'de “Koordinat Yaz” bunun yazı olarak yapılanıdır.
- **Yay uzunluğu ölçüsü:** bir yayın boyu, yayla eş merkezli bir ölçü yayının üstünde (yol kurbu, parsel köşesindeki yuvarlatma). AutoCAD'de DIMARC, “Yay Uzunluğu”; Netcad'de “Paralel Ölçülendir (Yay)”.
- **Kırıklı yarıçap ölçüsü:** merkezi çizimin dışında kalan büyük yarıçap (yol kurbu R = 300 m). Ölçü çizgisi yakına konan bir merkezden başlar, kırılarak yaya varır. AutoCAD'de DIMJOGGED, “Kırıklı”.
- **Semt ölçüsü:** bir kenarın kuzeyden saat yönünde doğrultusu (t), projenin açı biriminde, ok yönüyle. Poligon ve aplikasyon krokilerinde yazılır. AutoCAD'de karşılığı yoktur.
- **Eğim ölçüsü:** iki kotlu nokta arasındaki eğim, yüzde olarak, iniş yönünü gösteren okla (yol, kanal, şev). Köşe kotu (ADR 0142) bunu mümkün kılar.
- **Açı ölçüsünün eksik yolları:** bir yayın merkez açısı, bir dairenin iki noktası arasındaki açı (AutoCAD'in DIMANGULAR'ı yaya ve daireye tıklayınca bunları ölçer).
- **Döndürülmüş doğrusal ölçü:** doğrusal ölçü bugün yalnız yatay ya da düşeydir; AutoCAD'in DIMROTATED'ı yazılan açıda ölçer.
- **Hızlı ölçü:** seçili nesnelerin bütün kenarlarını tek seferde ölçülendirmek (parsel kenar ölçüleri). AutoCAD'de QDIM, “Hızlı Ölçü”.

ADR 0140 koordinat ve yay uzunluğu ölçüsünü “sözleşmede yeni nesne türü isteyenler” arasında bekletti. ADR 0145 ölçü değerinin zeminini bu adıma bıraktı: değer bir taramanın ya da çizginin üstünde okunmaz kalıyor.

DXF'te üçünün karşılığı vardır: koordinat ölçüsü DIMENSION'ın 6. türü, yay uzunluğu ARC_DIMENSION, kırıklı yarıçap LARGE_RADIAL_DIMENSION. Okuyucu bugün üçünü de yalnız bloğundan, çizgi ve yazı olarak alır.

## Karar

### 1. Veri

Yeni nesne türü yoktur. Ölçünün (`dimension`) biçimine (`style`) beş değer, ölçüye üç alan eklenir. Öbür ölçülerin anlamı değişmez.

| Biçim | `a` | `b` | `c` | `offset` | `angle` | `za`, `zb` | Değer |
|---|---|---|---|---|---|---|---|
| `ordinate` (Koordinat) | nokta | çizginin ucu | – | kullanılmaz, 0 | 0 Y, 90 X | – | noktanın Y'si ya da X'i |
| `arcLength` (Yay uzunluğu) | yayın başı | yayın sonu | yayın merkezi | ölçü yayının yaydan uzaklığı (+ dışarı) | – | – | yarıçap × açı |
| `jogged` (Kırıklı yarıçap) | yayın gerçek merkezi | yayın üstünde nokta | gösterilen merkez | kırığın gösterilen merkezden uzaklığı | – | – | yarıçap |
| `azimuth` (Semt) | kenarın başı | kenarın sonu | – | yazının kenardan işaretli uzaklığı (+ sol) | – | – | `a`'dan `b`'ye semt |
| `slope` (Eğim) | birinci nokta | ikinci nokta | – | yazının kenardan işaretli uzaklığı (+ sol) | – | iki noktanın kotu | eğim, yüzde |

- **Koordinat:** `angle` doğrusal ölçününki gibidir: 0 Y (sağa, noktanın doğusu), 90 X (yukarı, kuzeyi). Yokken 0'dır. Değer, projenin koordinat sisteminde mutlak koordinattır; önüne `Y=` ya da `X=` yazılır.
- **Yay uzunluğu:** yay `a`'dan `b`'ye saat yönünün tersinedir. Yarıçap `|a − c|`'dir; `b` yalnız ucun doğrultusunu verir (açı ölçüsündeki gibi). Ölçü yayının yarıçapı `|a − c| + offset`'tir, sıfırdan büyük olmalıdır.
- **Kırıklı yarıçap:** yarıçap `|b − a|`'dır. Ölçü çizgisi `c`'den başlar; `offset` kırığın `c`'den uzaklığıdır.
- **Semt:** değer `t = atan2(Δx, Δy)`, 0 ile tam tur arasında; projenin açı biriminde, önüne `t=` yazılır. Ok `a`'dan `b`'ye gider.
- **Eğim:** değer `|zb − za| / |b − a| × 100`, iki ondalıkla, önüne `%` yazılır. Ok yüksekten alçağa, iniş yönünde gider; iki kot eşitse ok başı yoktur.
- **`mask` — ölçü değerinin zemini:** bütün ölçülerde, yazınınki gibi (ADR 0145). Değer, çizimin zemin rengiyle örtülü bir kutunun üstünde çizilir. Yalnız `true` yazılır.
- **`za`, `zb`:** eğim ölçüsünün iki kotu, metre (ADR 0142'nin çizgisindeki adlar). Öbür biçimlerde yazılmaz.
- **Adlar:** `DimensionStyle::{Ordinate, ArcLength, Jogged, Azimuth, Slope}`; sözleşmede ve TypeScript'te `ordinate`, `arcLength`, `jogged`, `azimuth`, `slope`. `DimensionEntity`'nin yeni alanları `mask`, `za`, `zb`.

### 2. Ölçüler ve yerleşim

Her ölçü değerin yüksekliği `h` cinsindendir, öbür ölçülerdeki gibi: boşluk `h/2`, çentik `0,6h`, değer çizginin `0,35h` üstünde.

- **Koordinat:**
  - Çizgi ölçülen eksene diktir: Y ölçüsünün çizgisi düşey, X ölçüsününki yataydır; `b`'ye doğru gider.
  - Çizgi noktadan `h/2` uzakta başlar, uzatma çizgileri gibi.
  - `b` noktanın hizasında değilse çizgi kırılır, AutoCAD'deki gibi: `h` boyunca eksene dik, `h` boyunca çapraz, sonra yine eksene dik `b`'ye. Kırığa yer yoksa çizgi noktadan `b`'ye düz gider.
  - Değer son parçanın ortasında, üstündedir.
- **Yay uzunluğu:**
  - Ölçü yayı ölçülen yayla eş merkezli, aynı açıdadır; uçlarında çentik vardır.
  - Uzatma çizgileri yayın uçlarından, merkezden dışarı ya da içeri, ölçü yayına dek ve `h/2` ötesine uzanır. Ölçü yayı yaya `h/2`'den yakınsa çizilmezler.
  - Değer ölçü yayının ortasında, teğet doğrultusunda, okunur yönde ve ölçülen yaydan uzak yanındadır: okuma yönünde üstü yaya bakıyorsa (yayın alt yarısı gibi) değer, simgesiyle birlikte, çizginin öbür yanına üstteki boşluğu kadar geçer; yayın üstüne binmez.
  - Değerin üstünde küçük bir yay simgesi vardır (AutoCAD'in “yazının üstünde” seçeneği): 0,6h genişliğinde yarım daire, merkezi tabanın 1,3h üstünde, zeminin üstünde kalır. Simge yazı tipine bağlı değildir, ölçünün çizgileri gibi çizilir.
- **Kırıklı yarıçap:**
  - Çizgi gösterilen merkezden yarıçap doğrultusunda başlar, `offset`'te 45°'lik bir kırıkla gerçek yarıçap çizgisine geçer, yaya varır; yayda çentik vardır.
  - Değer yaya varan parçanın üstündedir; o parça kısaysa gösterilen merkezden başlayanın.
- **Semt ve eğim:**
  - Uzatma çizgisi ve çentik yoktur. Kenarın ortasının `offset` kadar yanında, kenara paralel, `3h` boyunda bir ok vardır, açık ok başlı.
  - Değer okun kenardan uzak yanındadır (ok kenarın üstündeyse okun üstünde); okuma yönünde üstü kenara bakıyorsa okun altına, üstteki boşluğu kadar geçer.
- **Zemin:** değerin kutusu yazınınki gibi ölçülür (tabanın 1,15h üstünden 0,23h altına); zemin onun çevresinde, yanlarda 0,1h payla, üstte ve altta paysızdır: ölçü çizgisi ve yay simgesi açıkta kalır.
- **Kutu ve çizgi:** değerin kutusu çizgiden iki yanda da 0,12h uzaktır: üstte taban çizginin 0,35h üstünde, altta kutunun tepesi çizginin 0,12h altında (simgeli yay uzunluğunda taban 1,72h, öbürlerinde 1,27h altta).

### 3. `.kcad`: belge şeması 9

- **Yenilikler:** beş biçim ile `mask`, `za`, `zb` alanları şema 9'dadır. Yazıcı 9'u yalnız bunlardan biri kullanılınca yazar; öbür çizimler eski şemalarıyla, bayt bayt aynı kalır. Şema 9 şema 8'i kapsar.
- **Eski şema:** şema 8 ve öncesinde yeni türler bilinmeyen değer (`bad_value`), yeni alanlar bilinmeyen alandır (`unknown_field`), yeriyle. Eski okuyucu ölçüyü sessizce başka bir ölçüye çevirmez, dosyayı açmaz.
- **Okuyucu reddi:**
  - yay uzunluğu ve kırıklı yarıçapta `c`, eğimde `za` ya da `zb` yoksa `missing_field`;
  - öbür biçimlerde `za` ya da `zb` varsa `bad_value`;
  - koordinatta `angle` 0 ya da 90 değilse `bad_value`;
  - `mask: false` `bad_value`.
  - Yazıcı aynılarını yazmaz, yeriyle reddeder.
- **Birlikte değişenler (ADR 0025'in kuralı):**
  - spesifikasyon ve şema 9'un paragrafı;
  - Rust kodeği ve tipli sütunlar (`FORMATS_VERSION` artar);
  - bağımsız Python okuyucusu ve yazıcısı;
  - örnek dosyalar (`fixtures/kcad/v2`): her yeni biçim, zeminli ölçü, blok tanımında ölçü; şema 8'de reddedilen değerler.
- **Sunucu:** ölçü zaten `cad_definition` olarak saklanır; okuyucunun kurallarıyla denetlenir. İzdüşümü ölçülen noktaların LineString'idir.

### 4. Hesap (ortak çekirdek)

- **Yerleşim:** `geom::dimension`'ın `layout_dimension`'ı yeni biçimlerin çizgilerini, değerin yerini ve dönüşünü, değeri, birimini ve önekini verir. Yeni birimler yüzde ve koordinattır: koordinat ölçüsünün değeri uzunluk gibi yazılır ama uzunluk sayılmaz (`$uzunluk`, Öznitelikler'in toplamları). Birim ve önek biçimden de bulunur (`dimension_measure`; bloktaki ölçünün etiketi böyle yazılır). Bağımsız Python başvurusuyla sınanır (`fixtures/dimension/v1/layout.json`, `scripts/fixtures/dimension_cases.py`).
- **Depo:** seçme, kenet ve kapsam öbür ölçülerin yolundandır; kırıklı yarıçapın kapsamı gerçek merkezi içermez (çizilmez, yüzlerce metre uzakta olabilir), semt ve eğiminki kenarın uçlarını içerir (tutamaçları ve kenet noktaları oradadır). Kenet noktaları:
  - koordinatta nokta ve çizginin ucu;
  - yay uzunluğunda yayın uçları ve merkezi;
  - kırıklı yarıçapta yaydaki nokta ve gösterilen merkez;
  - semt ve eğimde kenarın uçları.
- **Tutamaçlar:**
  - koordinatta çizginin ucu (nokta yerinde kalır);
  - yay uzunluğunda yayın uçları (yarıçap korunur) ve ölçü yayı;
  - kırıklı yarıçapta gösterilen merkez, kırık ve yaydaki nokta;
  - semt ve eğimde uçlar ve ok.
- **Dönüşümler:** noktalar taşınır; ölçek yüksekliği ve uzaklığı çarpar.
  - Koordinat ölçüsünün ekseni dünyanındır, döndürmede değişmez; çizgi yine eksene dik kurulur.
  - Semt yeni doğrultuyu, eğim aynı kotları gösterir (ölçeklenen bloktaki eğim de: yatay uzaklık büyür, kotlar aynı kalır).
  - Yansımada yay uzunluğu aynı yayı ölçer (uçları yer değiştirir, ölçü yayı aynı yanda kalır); kırıklı yarıçapın kırığı yarıçap boyuncadır, yansımada değişmez.
- **Patlat:** ölçü çizgilerine ve değerine ayrılır; yay simgesi ve oklar çizgi olur.

### 5. Çizim

- **Çizgiler** sahnenin parçasıdır, öbür ölçülerinki gibi.
- **Değer** etiket kaydıyla çizilir (`LABEL_DIMENSION`). Yeni önekler (`Y=`, `X=`, `t=`, `%`) ve yüzde birimi kayıttaki kodlarıyla gelir; zemin kaydın boş sayısındadır. İki çizici zemini değerin ölçülen kutusundan çizer.

### 6. Komutlar

- **`cad.entities.create`:** ölçü geometrisi yeni biçimleri ve alanları alır. Denetimler, `cad.entities.edit`'in geometri kurallarıyla ve onların sırasında (sonlu sayılardan sonra, yarıçaptan önce), hepsi `invalid_dimension`, düzeltilecek alanın yoluyla:
  - kot yalnız eğimde (`za`, `zb`);
  - koordinatın ekseni (`angle`) 0 ya da 90;
  - sonra çekirdeğin çizemediği ölçü (`dimension_fault`; yerleşim yalnız o zaman yoktur): koordinatın çizgisi noktadan eksene dik yarım yazı yüksekliğinden kısa (`b`); yay uzunluğunun merkezi yok (`c`), yarıçapı (`a`), açısı (`b`) sıfır, ölçü yayı merkeze ulaşıyor (`offset`); kırıklı yarıçapın gösterilen merkezi yok (`c`), yarıçapı sıfır (`b`), gösterilen merkez yarıçap boyunca yaydaki noktadan geride değil ya da yarıçaptan uzaklığı bu geriliği aşıyor (`c`); semt ve eğimin iki ucu aynı (`b`); eğimin bir kotu yok (`za` ya da `zb`).
  - Kotlar da sonlu sayıdır (`not_finite`). `mask: false` ve null kot alan değildir.
- **`cad.entities.edit`:** `properties` işlemi koordinatı (Y ya da X), kotları ve zemini değiştirir; tutamaçlar `grip`'le yazar; ikisi de aynı kurallarla denetlenir.
- **`cad.entities.transform`:** §4'ün dönüşüm kurallarıyla; bağımsız dönüşüm başvurusu (`affine_reference.py`) yeni türleri bilir.
- **Ortak durumlar:** bağımsız Python üreticileriyle, üç koşucuda.

### 7. Araç ve arayüz

Bütün ölçüler şeritte, Çizim › Açıklama › **Ölçülendirme ▾** listesindedir. İki platformda aynı katalogdan gelir; web'de klasik araç kutusu ve menüler de oradan beslenir. Her biri komut satırının seçeneği ve takma adıyla da açılır.

- **Ölçülendirme'nin yeni yöntemleri:**
  - **Koordinat** (O): önce nokta, sonra çizginin ucu. Ekseni imleç seçer: imleç noktadan daha çok düşey uzaklaştıysa Y, yatay uzaklaştıysa X. Y (Y) ve X (X) ekseni sabitler. Takma adlar: `KOORDINATOLCU`, `DIMORDINATE`, `DOR`.
  - **Yay uzunluğu** (U): önce yay (yay nesnesi, çoklu çizginin ya da alanın yaylı kenarı), sonra ölçü yayının yeri. Kısmi (K): yayın üstünde iki nokta. Takma adlar: `YAYUZUNLUGU`, `DIMARC`, `DAR`.
  - **Kırıklı yarıçap** (I): önce yay ya da daire, sonra gösterilen merkez, yaydaki nokta, kırığın yeri. Takma adlar: `KIRIKLI`, `DIMJOGGED`, `DJO`.
  - **Semt** (T): kenarın iki ucu ya da bir kenara tıklama, sonra yazının yeri. Takma adlar: `SEMT`, `SEMTOLCU`.
  - **Eğim** (E): iki nokta, sonra yazının yeri. Kot kenetlenen köşeden alınır; kotsuz noktada komut satırı kotu sorar. Takma adlar: `EGIM`, `EGIMOLCU`, `SLOPE`.
- **Açı'nın yeni yolları:**
  - **Yaydan:** yaya tıklayınca merkez açısı.
  - **Daireden:** daireye tıklanan noktadan ikinci noktaya. 180°'den büyük açı yayın yerine göredir, bugünkü gibi.
- **Doğrusal'ın Açı (A) seçeneği:** ölçme doğrultusu projenin açı biriminde yazılır (döndürülmüş doğrusal ölçü).
- **Zemin (Z):** bütün yöntemlerde seçenek; uygulama boyunca kalır, Yazı'nınki gibi.
- **Hızlı ölçü:** yeni araç, Ölçülendirme ▾ listesinde. Takma adlar: `QDIM`, `HIZLIOLCU`.
  - Seçili çizgi, çoklu çizgi ve alanların her düz kenarına hizalı, her yaylı kenarına yay uzunluğu ölçüsü koyar.
  - Ölçüler kapalı alanda dışarıda, açık çizgide imlecin tarafındadır; uzaklık imleçle ya da yazılarak verilir.
  - İki nesnenin ortak kenarı bir kez ölçülür. Hepsi tek adımda yazılır.
- **Öznitelikler:**
  - ölçü bölümünde Zemin;
  - koordinatta Koordinat (Y ya da X);
  - yay uzunluğunda Yarıçap ve Açı (salt okunur; kırıklı yarıçapın yarıçapı zaten “Ölçülen yarıçap” satırındadır, açısı yoktur);
  - eğimde iki kot.
  - Çoklu seçimde farklı değer “Çeşitli”dir; her değişiklik tek “Değiştir” adımıdır.

### 8. Değişim biçimleri

- **DXF yazma:**
  - **Koordinat:** 6. türden bir DIMENSION; 10 başlangıç (0, 0), 13 nokta, 14 çizginin ucu; Y ölçüsünde 70'in 64 biti (AutoCAD'in X türü).
  - **Yay uzunluğu:** bir ARC_DIMENSION; 13, 14 yayın uçları, 15 merkez, 10 ölçü yayının üstü, 40, 41 açılar.
  - **Kırıklı yarıçap:** bir LARGE_RADIAL_DIMENSION; 10 gerçek merkez, 15 yaydaki nokta, 13 gösterilen merkez, 14 kırık.
  - **Semt ve eğim:** DXF'te karşılıkları yoktur. Kendi bloğuyla çizilen hizalı bir DIMENSION olarak yazılırlar (değer yazısıyla); başka programlar KentOS'un çizdiğini gösterir.
  - Her ölçünün çizimi kendi bloğundadır; KentOS verisi ölçüyü aynen geri verir.
  - **Zemin:** ölçünün boyut stili değişikliğinde DIMTFILL 1; başka programlar da gösterir.
- **DXF okuma:**
  - 6. türden DIMENSION, başlangıcı (10) 0, 0 iken koordinat ölçüsüdür. Başka bir başlangıcın değeri göreli olduğundan bloğuyla alınır; rapor söyler.
  - ARC_DIMENSION yay uzunluğu, LARGE_RADIAL_DIMENSION kırıklı yarıçap ölçüsüdür, kendi gruplarından.
  - DIMTFILL 1 (nesnenin ya da stilin) zemindir.
- **GeoJSON:** ölçü yazılmaz, öbür ölçüler gibi; rapor söyler.

### 9. Kapsam dışı

- Ölçü değerinin çizgi boyunca yeri ve hizası (AutoCAD'in DIMTAD ve DIMJUST'u).
- Koordinat ölçüsünde başka bir başlangıç noktası (AutoCAD'in UCS'si); KentOS'un koordinatları mutlaktır.
- Köşelere toplu koordinat yazımı (Netcad'in “Koordinat Yaz”ı, `N-15`): ayrı bir işlem aracıdır.
- Ölçülerde ok başı türleri; KentOS ölçüleri eğik çentik kullanır.
- Merkez işareti, geometrik tolerans, ölçü kesmesi ve ölçü aralığı.

### 10. İş sırası

1. **Sözleşme ve `.kcad` şema 9.** Beş biçim, `mask`, `za`, `zb`; kodek ve tipli sütunlar, belirtim, Python okuyucusu ve yazıcısı, örnekler; iki belge; sunucu. *(1 Ekim: tamam. Sözleşmede `DimensionStyle`'ın beş yeni değeri (`ALL`, `name`, `from_name`, `is_schema_9`; adlar artık camelCase, eski beşi aynı) ve `DimensionEntity`'nin `mask`, `za`, `zb`'si; komutların geometrisi (`EntityGeometry::Dimension`) ve çekirdeğin şekli (`Shape::Dimension`) bunları taşır, dönüşüm, tutamaç, esnetme ve blok yolları düşürmez; deponun paket kaydı (iki tarafta) üç sayı aldı. KCAD şema 9 yalnız bunlar kullanılınca yazılır; okuyucu ve yazıcı aynı kuralları yeriyle uygular (`missing_field`, `bad_value`, eski şemada `bad_value` ve `unknown_field`). Tipli sütunlarda yeni bayraklar (`FORMATS_VERSION` 18). Bağımsız Python yazıcısı ve okuyucusuyla `dimensions.kcad` (beş tür, zeminli ölçü, blok tanımında eğim) ve sekiz bozuk örnek; desteklenmeyen şema örneği artık 10. Web açılışta aynı kuralları denetler. İki belge yeni ölçüleri taşır (`document-ops` senaryosu). Sunucu denetler ve saklar. Python SDK'sının tipleri yeniden üretildi. Bilinen ara durum: yerleşim 2. adıma dek yeni türleri hizalı ölçü gibi çizer; DXF yazıcısı 5. adıma dek onları KentOS verisi olmadan, çizgileri ve değeriyle yazar ve söyler.)*
2. **Çekirdek ve çizim.** Beş yerleşim bağımsız başvurusuyla; depo, tutamaçlar, dönüşümler, Patlat; iki çizicide değer, önek, yüzde ve zemin. *(1 Ekim: tamam. Çekirdekte `geom/dimension/kinds.rs`: koordinat (çizgi eksene dik, yer varsa AutoCAD'in kırığıyla), yay uzunluğu (ölçü yayı, uzatma çizgileri, çizgi olarak yay simgesi), kırıklı yarıçap (45°'lik kırık, değer yaya varan parçada ya da ilkinde), semt ve eğim (3h'lik açık oklu ok; eğimde ok inişe, düzde oksuz). Bağımsız başvuru `scripts/fixtures/dimension_cases.py` 25 örneği `fixtures/dimension/v1/layout.json`'a yazar; çekirdek (`tests/dimensions.rs`) ve web WASM'ı (`dimension.test.ts`) 1e-9 m içinde aynı. Birim ve önek biçimden de bulunur (`dimension_measure`, web'de `dimensionMeasure`, ikisi her biçim için sınanır). Tutamaçlar türe göre (koordinatta yalnız çizginin ucu; yay uzunluğunun uçları yayda, kırıklı yarıçapın noktası çemberde kalır; yeni tür çizilemeyeceği yere gitmez), kenet ve kapsam §4'teki gibi; dönüşümlerde yay uzunluğu açı, kırıklı yarıçap ve koordinat yarıçap gibidir. Etiket kaydının birim (uzunluk, açı, yüzde, koordinat), önek ve zemin kodları; bloktaki ölçünün etiketi biçiminden. İki çizici değeri yeni birimlerle yazar (yüzde iki ondalıkla) ve zeminini kutusundan, yanlarda paylı çizer. Öznitelikler'de “Ölçülen …” satırı türe göre, ötelenmenin adı (kırıklı yarıçapta “Kırık uzaklığı”, koordinatta yok). Resim sahnesi `fixtures/interaction/v1/dimensions.kcad`: masaüstü `labels::dimension_screens`, web `shots.mjs dimensions`.)*
3. **Komutlar.** `create` ve `edit`; ortak durumlar üç koşucuda. *(1 Ekim: tamam. Çekirdekte `dimension_fault` (`DimensionFault`, WASM'da `dimensionFault`): yeni bir ölçü neden çizilemez; yerleşimle birebir (ortak örnekler, elle seçilmiş hatalar ve 20 000 rastgele ölçü). İki işleyicide aynı denetimler ve iletiler (masaüstü `dimension.rs`, web `product/dimension.ts`); masaüstünün sonluluk denetimi kotları da kapsar (web zaten kapsıyordu); web `mask: false`'u ve null kotları alan olarak yazmaz. Ortak durumlar: `create` beş türü ve zemini yazar, on beş reddi yoluyla verir; `edit` Öznitelikler'den ekseni, kotları ve zemini yazar, tutamaçla koordinatın ucunu, kuralları ve kilitli katmanı denetler; `transform` dört yeni türü aynada, döndürmede ve ölçekte bağımsız başvuruyla (`affine_reference.py`). Web, masaüstü ve Python SDK'sı hepsini geçer.)*
4. **Araç ve arayüz.** İki platformda izler ve resimlerle, dört parça:
   1. Koordinat ve Yay uzunluğu; *(1 Ekim: tamam. Ölçülendirme'nin yöntemleri kataloğun `methods`'u olarak şeritte, Ölçülendirme ▾ listesinde (Hizalı, Doğrusal, Açı, Yarıçap, Çap, Koordinat, Yay uzunluğu, sonra Zincir ve Baz ölçü), iki platformda aynı envanterden. Yöntemin kendi takma adları (`ToolMethod.aliases`: DOR, DIMORDINATE, KOORDINATOLCU; DAR, DIMARC, YAYUZUNLUGU; eski yöntemlere AutoCAD'in DAL, DLI, DAN, DRA, DDI'si ve DIMLIN, DIMANG, DIMRAD, DIMDIA artık yöntemiyle) komut satırında aracı o yöntemle başlatır (web `tools/methods.ts`, masaüstü `Catalog::method_by_alias`). Koordinat: nokta, sonra çizginin ucu ya da yazılan boyu (imlecin doğrultusunda, her nokta aracındaki gibi); eksen imleçten (çekirdeğin `ordinate_axis_for`'u), Y koordinatı (Y), X koordinatı (X), Eksen (O) imlece geri verir. Yay uzunluğu: yay ya da çoklu çizginin ya da alanın yaylı kenarı (daire değil), Kısmi (K) ile yayın üstünde iki nokta (çekirdeğin `arc_length_ends`'i: uçları saat yönünün tersine, yay dışındaki nokta yakın uca), sonra ölçü yayının yeri ya da yazılan uzaklık; Kısmi'de seçilen yay ve parçası vurgulanır, imlecin yanında parçanın uzunluğu. Ctrl+Z önce noktaları, sonra yayı bırakır. Ortak iz `dimension-kinds.json`, araç testleri iki platformda, resimler `dimension_scenes` ve `shots.mjs dimensions`.)*
   2. Kırıklı yarıçap, Semt ve Eğim; *(1 Ekim: tamam. Kırıklı yarıçap (I): daire, yay ya da yaylı kenar, sonra gösterilen merkez, yaydaki nokta (yaya konur; kırığa yer bırakmayan nokta söylenerek alınmaz), sonra kırığın yeri ya da yazılan uzaklığı; yaydaki nokta seçilirken ölçü önizlenir. Semt (T) ve Eğim (E): iki nokta ya da Kenardan (K) ile bir düz kenarın iki ucu, sonra okun yeri ya da yazılan uzaklık. Eğim her noktanın kotunu kenetlendiği noktadan ya da köşeden alır (Koordinat oku'nun yolu), kotsuzda komut satırı sorar; sorarken kenet ve imleçteki çizgi yoktur, tıklama söylenerek alınmaz. Takma adlar: KIRIKLI, DIMJOGGED, DJO; SEMT, SEMTOLCU; EGIM, EGIMOLCU, SLOPE. Ctrl+Z noktaları (Kenardan'ın iki ucunu birlikte), sonra daireyi bırakır. Ortak iz `dimension-more-kinds.json` (belgesi `dimension-ground.kcad`), araç testleri iki platformda, resimler.)*
   3. Açı'nın yolları, döndürülmüş doğrusal, Zemin ve Öznitelikler; *(1 Ekim: tamam. Açı'nın ilk seçimi düz kenar, yay ya da dairedir: yay (yaylı kenar da) kendi merkez açısını verir, uçları saat yönünün tersine (`arc_length_ends`), imleç yalnız yayın yarıçapını seçer; daireye tıklanan nokta daireye konur, ikinci nokta gösterilir, yay ikisinin arasında imlecin tarafındadır (Köşeden'in `vertex_arms`'ı); daire seçilince birinci kol vurgulu, ikincisi imlece çizilir. İlk seçimde boş yer “düz bir kenara, yaya ya da daireye” der; ikinci seçim yalnız düz kenardır. Ctrl+Z ikinci noktayı, sonra daireyi ya da yayı bırakır; noktalar varken Enter baştan başlar. Doğrusal'ın Açı (A) seçeneği ölçme doğrultusunu projenin açı biriminde, doğudan saatin tersine sorar; yazılan açı Yön'de görünür ve Y, X kilitleri gibi kalır; iki platform dereceyi aynı işlemle bulur (`(rad · 180) / π`, Rust'ın `to_degrees`'i değil), saklanan açı aynı bitlerdir. Zemin (Z) seçilecek bir şey yokken ve ölçü yerleştirilirken seçenektir (komut satırında yöntemlerden önce), uygulama boyunca kalır; ölçü `mask` ile yazılır. Öznitelikler'in ölçü satırları (web `ui/properties/dimensionRows.ts`, masaüstü `properties/rows/dimension.rs`, yazması `change_dimensions`): Zemin; hepsi koordinatsa Koordinat (Y, X); hepsi eğimse Birinci ve İkinci kot; hepsi yay uzunluğuysa Yarıçap ve Açı (salt okunur). Tek ölçüde Geometri'de, çoklu seçimde “Ölçü” ya da “Ölçüler (n)” bölümünde; farklı değer “Çeşitli”, her değişiklik tek “Değiştir” adımıdır, değeri olan atlanır. İzler: `dimensions.json`'a Daireden, Zemin ve yazılan doğrultu, `dimension-more-kinds.json`'a Yaydan; iki iz oynatıcısı ölçünün zeminini (`mask`) ve doğrultusunu (`angle`, 1e-9 içinde) da karşılaştırır. Web'in araç test düzeneği ölçü değerini artık uygulamanın biçimiyle (önek ve birimle) yazar. Testler iki platformda; resimler `dimension_scenes` ve `shots.mjs dimensions`.)*
   4. Hızlı ölçü.
5. **Biçimler.** DXF okuma ve yazma; örnek dosyalar ve bağımsız denetim.

Her adım iki platformda, ortak fixture'larla, kendi commit'inde ilerler.

## Sonuçlar

- Yeni nesne türü yoktur; ölçünün yolu (araç, depo, çizim, DXF) genişler.
- Koordinat, semt ve eğim noktalarına bağlıdır: noktalar ya da kotlar değişince değer kendiliğinden doğrudur.
- Ölçü zemini yazınınkiyle aynı çizim yolunu kullanır.

## Doğrulama

- **Şema:** örnek dosyalar bağımsız Python yazıcısıyla; Rust ve Python okuyucusu aynı redleri verir.
- **Yerleşim:** `fixtures/dimension/v1` bağımsız başvurudan; çekirdek ve web WASM'ı 1e-9 m içinde aynı.
- **Komutlar:** ortak durumlar üç koşucuda.
- **Araç:** ortak izler iki platformda; resimler iki temada, 1440×900 ve 1100×650.
- **Biçimler:** DXF'in koordinat, yay uzunluğu ve kırıklı yarıçap örnekleri elle hesaplı beklentilerle; yazıcının örneği bağımsız denetimle (`dxf_write_reference.py`).
