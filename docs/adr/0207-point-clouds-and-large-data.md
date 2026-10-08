# ADR 0207: Nokta bulutu, COG ve büyük veri sağlayıcıları

- **Durum:** kabul edildi (2026-10-08). Kapsam sahibin kararlarıdır (8 Ekim, dördü de önerilen seçenek): LAS, LAZ, COPC ve XYZ;
  LAZ'ın sıkıştırması `laz` crate'inden (laz-rs, Apache-2.0; sahibin onayı), LAS ve COPC'nin okuyucusu, dizini ve yazıcısı bizim;
  2B plan görünümü, kat kat ve renk kipleriyle; dört işlem grubunun hepsi (XYZ ve alan sorgusu; seyreltme, zemin süzgeci ve
  yüksekliğe göre sınıflama; kırp, birleştir, karola; rasterleştirme ve sınır); dosya, URL (HTTP Range) ve sanal bulut. İlke
  “Performance First”. Madde tek parçada biter.
- **Ek (8 Ekim, sahibin kararı): yalnız masaüstü.** “Nokta bulutu çok ağır bir iş o yüzden bunun web tarafı olmasın şimdilik. Sadece
  masaüstü ile kalalım”; “masaüstü tarafını çok yüksek kalitede yapmam gerekiyor ve yüksek performans olması gerekiyor”; “Web tarafına
  yine butonların aynı şekilde koy ribon üzerine ama bu işlem sadece masaüstü tarafında web hazırlanıyor diye mesaj ver”. Bu ADR'nin web'e
  ait kısımları (OPFS'teki dizin, nokta bulutu işçisi, WebGPU ve WebGL2'de noktalar, indirme, web'de Kaynağı yeniden seç, web'in .vpc
  üyeleri) **ertelendi**; yalnız masaüstünde yapıldı. Web'de aynı düğmeler aynı yerlerde (CBS'de Veri, CAD'de Ekle; Raster'ın yanında
  Nokta bulutu paneli, İşlemler'in araçları ▾'de) “Yalnız masaüstünde; web için hazırlanıyor” notuyla soluk durur, çalıştırılınca işin
  masaüstünde olduğu söylenir (`app/pointCloudCommands.ts`). Web `.kcad`'in nokta bulutunu (şema 31) ve rasterin adresini okur, korur ve
  yazar (sütunlar, belge, kurallar), çizmez; taşıma ve dizi orada da `pointcloud_fixed` ile reddeder. (Sahip aynı gün bunu da değiştirdi:
  aksini söyleyene dek her modül iki platformda, gerektiğinde bulut tarafıyla yapılır; nokta bulutu yalnız masaüstünde kalır.)
- **Bağlam belgesi:** TODOS.md `GIS-09` (ve araştırma notu), ADR 0204 (raster: kaynak, piramit, karolar, raster atlası, sistemin
  kuralı), ADR 0192 (gömülü varlık), ADR 0157 (uzak karolar), ADR 0084 ve 0200 (İşlemler, dosya parametresi, tablo çıktısı),
  ADR 0201 (sonuç yeni katmana), ADR 0143 (çok parçalı alan), ADR 0046 (dosyanın sistemi), ADR 0025 (`.kcad` v2), ADR 0009 (dosya
  biçimleri: bozuk girdide panik yok, sınırlar).

## Bağlam

Hava ve yer LiDAR'ı, fotogrametrik ve drone nokta bulutları yüz milyonlarca noktadır; tek dosya gigabaytlarca, bir proje yüzlerce
dosyadır. Netcad (Nokta Bulutu, Seyreltme ve Yüzey Filtresi, Nokta Bulutu XYZ ve Alan Sor), ArcGIS (LAS dataset, Classify LAS
Ground ve By Height, LAS Dataset To Raster) ve QGIS (nokta bulutu katmanı, sanal nokta bulutu, PDAL araçları) bulutu dizinli,
kat kat okunan bir katman olarak tutar. GIS-08'in rasteri dosyanın yalnız gereken parçalarını okur ama yalnız yerel dosyayı ve
gömülü varlığı bilir; uzaktaki COG'u (Cloud Optimized GeoTIFF) okuyamaz. Bu ADR KentOS'un nokta bulutu nesnesini, okuyucularını,
dizinini, görünüşünü, işlemlerini ve büyük verinin sağlayıcılarını (dosya, URL, sanal bulut) tanımlar.

## Karar

### 1. Sağlayıcı ve yetenekler

Büyük bir kaynak her zaman **sağlayıcı** üzerinden okunur: sağlayıcı kaynağın baytlarını parça parça verir (yerel dosya, gömülü
varlık ya da URL) ve kaynağın yeteneklerini söyler. Üç yetenek birbirinden ayrıdır:

| Kaynak | Önizleme (görünüm) | Tam okuma (analiz) | Dışa aktarma |
|---|---|---|---|
| COPC (dosya, URL, gömülü) | dosyanın kendi dizini | düğüm düğüm, bütün noktalar | var |
| LAS, LAZ (dosya, URL, gömülü) | dizin hazırlanınca (önbellekte COPC) | dosyanın sırasıyla, parça parça | var |
| XYZ, PTS, TXT, CSV (dosya) | dizin hazırlanınca | satır satır | var |
| Sanal bulut | üyelerinin önizlemeleriyle | üye üye | var |
| Raster: GeoTIFF, COG (dosya, URL) | iç önizlemeler ya da piramit (ADR 0204 §3) | 0. kat blok blok | — |

- **Önizleme** yalnız çizim içindir: kaba katlardan başlar, görünüm yaklaştıkça incelir; düşük çözünürlüklü bir örnek hiçbir zaman
  analizin girdisi olmaz.
- **Tam okuma** kaynağın bütün noktalarını özgün değerleriyle (nokta biçimi, ölçek ve öteleme, ek baytlar) verir; işlemler (§7) yalnız
  tam okumayla çalışır.
- **Dışa aktarma** işlemlerin sonuçlarını yazar (LAS, LAZ, COPC, GeoTIFF; §7).
- Pencereler ve İşlemler, kaynağın yeteneği yoksa düğmeyi kapalı gösterir ve nedenini söyler (ör. “dizin hazırlanıyor”).

**URL** (HTTP ve HTTPS): kaynak önce boyu ve sürümü için sorulur (`Range: bytes=0-0`; yanıtın `Content-Range`'i boyu, `ETag` ya da
`Last-Modified`'ı sürümü verir), sonra yalnız gereken baytlar `Range` istekleriyle okunur. Sunucu `206 Partial Content` vermezse kaynak
nedeni söylenerek reddedilir (“sunucu parça parça okumaya izin vermiyor”). Yan yana istekler birleştirilir; okunan parçalar 64 KB'lık
bloklar olarak sınırlı bir bellekte (kaynak başına 64 MB) tutulur. Geçici hatalar (bağlantı, 5xx, zaman aşımı 30 s) üç kez artan
aralıkla yeniden denenir. Kaynağın sürümü oturumda değişirse (başka `ETag`) kaynak kapanır ve yeniden açılması söylenir. Web'de (ertelendi)
sunucunun CORS'ta `Range` başlığına ve `Content-Range`'i okumaya izin vermesi gerekecek; vermiyorsa bu söylenir. Kimlik doğrulama,
vekil sunucu ve servis protokolleri (WMS, WFS, …) `GIS-10`'dadır; burada yalnız herkese açık adresler (adresin kendi sorgu
dizesindeki belirteçle birlikte) okunur. Raster de URL'den eklenebilir: COG iç önizlemeleriyle hemen, önizlemesiz büyük GeoTIFF
piramidi bir kez akıtılarak (ADR 0204 §3).

### 2. Biçimler

- **LAS 1.0–1.4** (ASPRS R15): başlık, VLR'ler ve EVLR'ler; nokta biçimleri 0–10. Dalga biçimi paketleri (4, 5, 9, 10'un alanları)
  okunur ama kullanılmaz; ek baytlar (extra bytes) kayıtta durur, dışa aktarmada taşınır. Koordinatlar `X·ölçek + öteleme`dir; tam
  sayılar özgün olarak tutulur, hiçbir zaman yuvarlanmaz.
- **LAZ** (LASzip): LAS'ın nokta kayıtları `laz` crate'inin çözücüsüyle, parça (chunk) parça; parça tablosu okunur, parçalar birbirinden
  bağımsız ve aynı anda çözülür. 1.0–1.3'ün sıralı (v2) ve 1.4'ün katmanlı (v3) sıkıştırmaları.
- **COPC 1.0** (copc.io): LAZ 1.4, nokta biçimi 6, 7 ya da 8; ilk VLR `copc`/1 (160 bayt: kübün merkezi, yarı kenarı, kökün nokta
  aralığı, hiyerarşinin ilk sayfası, GPS zamanının aralığı); hiyerarşi `copc`/1000 EVLR'sinde, 32 baytlık girdilerle (anahtar d, x, y, z;
  öteleme; bayt boyu; nokta sayısı: > 0 veri parçası, −1 alt sayfa, 0 boş). Her düğüm tek bir LAZ parçasıdır.
- **Metin bulutu** (XYZ, PTS, TXT, CSV): ayırıcı boşluk, sekme, virgül ya da noktalı virgül (ondalık ayırıcı nokta); sayı olmayan ilk
  satırlar başlık sayılır, PTS'nin ilk satırındaki nokta sayısı atlanır. Sütunlar sayılarına göre: 3 X Y Z; 4 X Y Z yoğunluk; 6 X Y Z R G B;
  7 X Y Z yoğunluk R G B (PTS'nin sırası). Yoğunluk 0–1 ise 65535 ile, renkler 0–255 ise 257 ile çarpılır. Koordinatların ölçeği,
  dosyada görülen en çok ondalık basamaktır (en çok 6); öteleme en küçük değerin ölçeğe yuvarlanmışıdır: yazılan değer kayıpsız tutulur.
  Ölçek ve aralık 32 bitlik tam sayıya sığmıyorsa ölçek bir basamak kabalaşır ve bu söylenir.
- **Koordinat sistemi** LAS'ın WKT VLR'sinden (`LASF_Projection`/2112) ya da GeoTIFF anahtarlarından (34735, 34736, 34737): yatay
  sistemin EPSG kodu (birleşik sistemde yatay parçanınki). Kural rasterinkidir (ADR 0204 §1, `raster::place`): aynı sistem eklenir, başka
  sistem nedeniyle reddedilir, bilinmeyen sistem (metin bulutu, kodsuz dosya) ancak kullanıcı “projenin sisteminde” diye onaylarsa eklenir.
- **Sınırlar:** VLR ve EVLR sayısı en çok 10 000, biri en çok 64 MB; nokta kaydı en çok 1024 bayt; parça tablosunun parça sayısı
  noktalarla tutarlı olmalı; COPC'nin hiyerarşisi en çok 1 000 000 düğüm, sayfaları döngüsüz; metin satırı en çok 4096 harf. Her biçim
  hatası nedeniyle söylenir, panik yok.

### 3. Nesne

Yeni nesne türü `pointcloud`:

- `sources`: bir ya da daha çok üye (birden çoksa **sanal bulut**); her üye `file` (bağlı dosya), `url` ya da `asset` (gömülü, ADR 0192'nin
  kuralıyla, en çok 32 MB) ve `format` (`las`, `laz`, `copc`, `xyz`), `count` (nokta sayısı) ve `bounds` (`[x₁, y₁, z₁, x₂, y₂, z₂]`,
  dosyanın başlığından) taşır. En çok 4096 üye.
- `bounds` ve `count`: bütün üyelerin kapsamı ve nokta sayısı. `srid` (0: bilinmiyordu, kullanıcı projeninkini onayladı).
- `style` (§5) ve `opacity` (0,1–1; yoksa 1). `.kcad` şema 31.

Noktalar `.kcad`'e yazılmaz; yalnız kaynakları. Nokta bulutu kendi katmanında durur: Nokta bulutu ekle dosyanın adıyla yeni katman açar.
Rasterin kaynağına üçüncü seçenek `url` eklenir (§1).

Nokta bulutu taşınmaz, döndürülmez, ölçeklenmez, aynalanmaz, kopyalanmaz (dizide de), patlatılmaz: konumu dosyasındadır, düzenlemeler
nedeniyle reddeder (“Nokta bulutu taşınmaz, döndürülmez, ölçeklenmez, aynalanmaz ve kopyalanmaz: konumu dosyasındadır. Bulutu seçimden
çıkarın.”). Silinir, katmanı ve
görünüşü değişir. Kapsamının dikdörtgeni nesnenin renginde ince çizgiyle çizilir; yalnız bu çerçeveden seçilir ve köşelerine kenetlenir.

### 4. Dizin (önizlemenin katları)

Önizleme COPC'nin sekizli ağacıdır. COPC kaynağın dizini kendisidir; LAS, LAZ ve metin bulutu için dizin **bir kez** hazırlanır ve
cihazın önbelleğine COPC dosyası olarak yazılır (masaüstünde `$XDG_CACHE_HOME/kentos-cad/pointcloud/<anahtar>.copc.laz`; web'de,
ertelendi, tarayıcının özel dosya sisteminde (OPFS) `kentos-pointclouds/<anahtar>.copc.laz`); anahtar kaynağın yolu (web'de adı; URL'de adresi),
boyu ve değişme zamanından (URL'de sürümünden) SHA-256'dır. Önbellek masaüstünde 16 GB, web'de 4 GB'ı aşınca en uzun süredir
açılmayan dizinler silinir. Hazırlık çizimin sağ altındaki panelde ilerlemesiyle görünür ve Durdur'la bırakılabilir; o sürece bulut
yalnız çerçevesiyle çizilir.

Ağaç belirlenimci bir kuralla kurulur (iki platform aynı dosyayı yazar):

- **Küp:** merkezi kaynağın kapsamının merkezi, yarı kenarı kapsamın en büyük yarı kenarı (sıfırsa 1 m). Düğüm (d, x, y, z), kökün
  kenarının 2⁻ᵈ'si boyunda, EPT'nin anahtarıdır.
- **Kökün nokta aralığı:** küpün kenarı / 128; her katta yarıya iner. Bir düğümün ızgarası 128 × 128 × 128 hücredir.
- **Yapraklar:** kaynak bir kez okunurken noktalar (kayıtları nokta biçimi 6, 7 ya da 8'e çevrilmiş, ek baytlarıyla) d₀ katındaki
  kutulara dağıtılır (d₀, kutu başına ortalama 2 milyon noktayı geçmeyecek en küçük kat, en çok 8; kutular masaüstünde önbelleğin geçici
  klasöründe, web'de OPFS'te). Her kutu ayrı işlenir: 100 000'den çok noktası olan düğüm, kat 16'ya ulaşmadıysa sekize bölünür; 8 milyonu
  aşan kutu bir kat daha bölünerek yeniden dağıtılır.
- **Katlar aşağıdan yukarı:** her düğüm kendi noktalarından (yaprakta kutunun noktaları, iç düğümde çocuklarının yukarı verdikleri)
  **ana düğümün ızgarasının her dolu hücresi için o hücrenin merkezine en yakın noktayı** (üç boyutlu uzaklık; eşitlikte kaynakta önce
  gelen) ana düğüme verir; kalanlar düğümün kendisinde kalır ve bir LAZ parçası olarak yazılır. Kök hiçbir şey vermez. Böylece her
  nokta ağaçta tam bir kez bulunur, iç düğümde ızgaranın her hücresinde en çok bir nokta vardır ve kaba katlar bulutun düzgün bir
  örneğidir.
- **Dosya:** LAS 1.4 başlığı (ölçek ve öteleme kaynağın; metin bulutunda §2'ninki), WKT VLR'si (sistem biliniyorsa), COPC bilgi VLR'si,
  LAZ VLR'si (değişken parçalı), parçalar ağacın sırasıyla (önce derinlik), parça tablosu ve hiyerarşi EVLR'si tek sayfa (düğüm 1 000 000'u
  geçmez). Dosya aynı zamanda sıradan bir LAZ'dır.
- Nokta biçimlerinin dönüşümü LAS 1.4 R15'in Ek A'sıdır: 0, 1, 4, 9 → 6; 2, 3, 5 → 7; 10 → 8; eski sınıf (0–31) ve bayrakları,
  dönüş sayıları, tarama açısı (derece → 0,006°'lik adım), kullanıcı verisi, kaynak kimliği, GPS zamanı (yoksa 0), renkler.

### 5. Görünüş

`style`'ın alanları:

- `render`: `rgb` (dosyanın renkleri; 16 bit 8 bite), `classification` (ASPRS sınıfları §5.1'in renkleriyle), `elevation` (Z renk
  rampasıyla), `intensity` (yoğunluk griyle), `returns` (tek, ilk, ara, son dönüş dört renkle) ya da `single` (nesnenin rengi).
  Varsayılan: nokta biçiminde renk varsa `rgb`, yoksa `elevation`.
- `ramp` (ADR 0204'ün rampaları; yoksa Arazi) ve `invert`; `min` ve `max`: `elevation`'ın ve `intensity`'nin aralığı. Nokta bulutu
  eklenirken aralık örnek noktaların (COPC'de kök düğümün, öbürlerinde dosyaya yayılmış örneğin) %2 ve %98'lik değerleri olarak yazılır;
  Nokta bulutu stili'nin **Otomatik**'i aynı kuralla yeniden hesaplar. Görünüş yalnız nesnenin alanlarından çizilir.
- `hidden`: gizlenen sınıflar (0–255).
- `size` (0,5–32, varsayılan 2) ve `sizeUnit`: `px` (aygıt pikselinden bağımsız ekran pikseli) ya da `m` (zeminde metre; en az 1 piksel);
  `shape`: `round` (varsayılan) ya da `square`.

Renkler çekirdekte hesaplanır (iki platform aynı baytları üretir); GPU yalnız noktaları çizer.

#### 5.1 Sınıflar

ASPRS'nin sınıfları, Türkçe adları ve renkleri: 0 Hiç sınıflanmamış, 1 Sınıflanmamış, 2 Zemin, 3 Düşük bitki, 4 Orta bitki, 5 Yüksek
bitki, 6 Bina, 7 Düşük gürültü, 8 Model anahtar noktası, 9 Su, 10 Demiryolu, 11 Yol yüzeyi, 12 Örtüşme, 13 Koruyucu tel, 14 İletken tel,
15 İletim kulesi, 16 Tel bağlantısı, 17 Köprü tabliyesi, 18 Yüksek gürültü, 19 Havai yapı, 20 Yok sayılan zemin, 21 Kar, 22 Zaman dışı;
23–63 ayrılmış, 64–255 kullanıcının. Renkler `pointcloud::classes`'tedir (iki platformda aynı).

### 6. Çizim

- **Katların seçimi** (çekirdek `geom::pointcloud::visible_nodes`, iki platformda aynı): kökten başlanır; küpü görünümle kesişen düğüm
  çizilir, nokta aralığı aygıt pikselinde noktanın boyundan büyükse çocuklarına inilir; en büyük aralıklılar önce, karede en çok 4 milyon
  (web'de 3 milyon) nokta. COPC'de düğümler birbirini tamamlar: inilen her kat ayrıntı ekler, eksik çocuk yalnız seyrek gösterir.
- **Yükleme:** düğümler ev sahibinin iş parçacıklarında çözülür ve renklenir, yalnız görünüşün okuduğu alanlarla: LAS 1.4'ün katmanlı
  sıkıştırmasında konum ve dönüşler her zaman, sınıf yalnız Sınıflar'da ya da gizlenen sınıf varken, yoğunluk yalnız Yoğunluk'ta, renkler
  yalnız Renkler'de açılır; GPS zamanı, açı, kaynak kimliği, bayraklar ve ek baytlar sıkıştırılmış kalır (`nodes::Needs`,
  `chunks::selection_for`). Çözülen düğüm görünüşün gereksinimiyle saklanır; başka gereksinimli görünüş düğümü yeniden çözer; önce görünüm
  ortasındakiler, görünümden çıkan düğümün isteği bırakılır. Çözülmüş düğümler sınırlı bellekte (masaüstünde 512 MB, web'de 256 MB),
  GPU'daki düğümler nokta bütçesinin 1,5 katında en az kullanılan önce bırakılarak tutulur; karede en çok 1 milyon nokta yüklenir.
  Görünüş değişince yalnız renkler yeniden hesaplanır ve yüklenir; konumlar kalır.
- **Derinlik:** her bulut kendi ara dokusuna çizilir (renk ve derinlik): noktalar ekran pikselinde kare ya da yuvarlak dörtgenlerdir,
  derinliği Z'dir (yüksek nokta üstte). Doku sonra belgedeki sırasında, saydamlığıyla çizime basılır. Konumlar GPU'ya düğümün merkezine
  göre float32 farkla gider; düğümün merkezinin kameraya farkı karede float64'ten hesaplanır (ADR 0157'nin ilkesi).
- Noktalar kendi modüllerindedir (`shaders/wgsl/points`, sözleşmesi `points.layout.json`: `pointsVs`, `pointsFs`); ara dokunun çizime
  basılması stilli çizimin 7. sürümüdür (`cloudFs`).

### 7. İşlemler

Hepsi İşlemler'de, **Nokta bulutu** kategorisinde (şimdilik yalnız masaüstü; web'de komutları notuyla bekler), tam okumayla, arka
planda (iş parçacığı), ilerleme çubuğu ve Durdur'la. Nokta yazan işlemler çıktı biçimi olarak LAS, LAZ (varsayılan) ya da COPC sunar; tek girdili
işlem girdinin nokta biçimini, sürümünü, ölçek ve ötelemesini, VLR'lerini ve ek baytlarını korur. Çıktı seçilen yere (yoksa kaynağın
yanına) yazılır; **Çizime ekle** açıksa (varsayılan) sonuç yeni katmanda nesne olarak eklenir (dosyanın adıyla, işlemin adımında).

- **Nokta bulutu XYZ sor**: araç; tıklanan yere ekranda 8 piksel içinde en yakın nokta (düzlemde; tam çözünürlükte, oradaki
  bütün düğümler okunarak): X, Y, Z, sınıf, yoğunluk, renk, dönüş, GPS zamanı, kaynak kimliği; komut satırına ve karta.
- **Nokta bulutu alan sorgusu**: seçili alanlar (ya da çizilen) içindeki noktaların sayısı, yoğunluğu (nokta/m²), Z'nin en küçüğü, en büyüğü,
  ortalaması ve standart sapması, sınıfların sayıları; tablo (Panoya kopyala, CSV; ADR 0200'ün tablo çıktısı).
- **Seyrelt**: Hücreyle (her 3B hücrede merkezine en yakın nokta; eşitlikte önce gelen), Yarıçapla (dosyanın sırasıyla: tutulan bir
  noktaya r'den yakın nokta atılır; 3B uzaklık), Her n'inci (sıradaki her n'inci nokta).
- **Zemin süzgeci** (SMRF, Pingel ve ark. 2013'ün belirlenimci tanımı): hücre (1 m), eğim (0,15), pencere (18 m), eşik (0,5 m), ölçek
  çarpanı (1,25), Yalnız son dönüşler (açık). (1) Son ve tek dönüşlerin her hücrede en küçük Z'si; (2) boş hücreler dolu hücrelerden
  başlayan 8 komşulu dalgayla, her hücre bir önceki dalgadaki komşularının en küçüğünü alarak dolar; (3) düşük aykırılar: −Z'ye 5 m
  pencere ve 1,0 eğimle aşamalı açma (aşağıdaki gibi), işaretlenenler boşaltılıp yeniden doldurulur; (4) r = 1 … ⌈pencere / hücre⌉ için r
  yarıçaplı disk (i² + j² ≤ r²) ile aşındırma ve yayma (açma; ızgaranın dışı yok sayılır): yüzeyin açılmışından farkı eğim · r · hücre'yi
  aşan hücre nesnedir, yüzey açılmışı olur; (5) nesne ve aykırı hücreler boşaltılıp (2) ile doldurulur: zemin yüzeyi; (6) her nokta için
  yüzeyin hücre merkezlerinden çift doğrusal değeri ve hücrenin merkezi farklarla eğimi: |z − yüzey| ≤ eşik + çarpan · eğim ise
  zemin (2); zemin olmayan 2'ler 1 olur, öbürleri değişmez.
- **Yüksekliğe göre sınıfla**: zemin noktalarından (2) hücrenin en küçük Z'si ve (2)'nin dolgusu ile yüzey, her noktanın yüzeyden
  yüksekliği h: taban (0,15 m) ≤ h ≤ 0,5 → 3, ≤ 2 → 4, ≤ 50 → 5 (sınırlar değiştirilebilir); yalnız 0 ve 1 sınıflılar (seçenek: zemin
  olmayan hepsi).
- **Bulutu kırp**: seçili alanların (delikleri ve parçalarıyla; yaylı kenarlar kesin) içinde ya da dışında kalanlar; sınırdaki nokta içeridedir.
- **Bulutları birleştir**: aynı sistemdeki bulutlar tek dosyaya; nokta biçimi gerekenlerin en genişi (6, 7 ya da 8), ölçek en incesi; bütün
  koordinatlar tam sayı olarak kayıpsız taşınabiliyorsa taşınır, taşınamıyorsa yuvarlanan nokta sayısı söylenir; farklı ek baytlar düşer
  ve söylenir.
- **Karola**: T metrelik karolar (köşeleri T'nin katlarında; sınırdaki nokta sağdaki ve üstteki karoya), `<ad>_<x>_<y>` dosyaları ve
  isteğe bağlı sanal bulut (.vpc).
- **Rasterleştir**: hücre (varsayılan noktaların ortalama aralığının iki katı), değer (En düşük, En yüksek, Ortalama, Sayı, IDW:
  yarıçap içindeki noktaların uzaklığın karesinin tersiyle ağırlıklı ortalaması), sınıflar (Hepsi ya da seçilenler; DTM için Zemin);
  ızgara hücrenin katlarına oturur; boş hücre nodata (−9999); çıktı karolu, Deflate'li 32 bit GeoTIFF ve raster nesnesi (ADR 0204;
  görünüşü rampa ve gölge).
- **Sınır çıkar**: hücre ve en az nokta sayısı; dolu hücrelerin birleşimi kesin dik kenarlı halkalarla (delikleriyle), tek çok parçalı
  alan olarak yeni katmana.

### 8. Sanal bulut

Birden çok dosya tek nesnedir (§3). **Nokta bulutu ekle** birden çok dosya seçilince sorar: her biri ayrı nesne mi, tek sanal bulut mu.
**.vpc** (QGIS'in ve PDAL wrench'in biçimi: STAC ItemCollection, her üye bir STAC Item) okunur: üyelerin adresleri `assets.data.href`'ten
(dosyanın klasörüne göre), kapsamları `proj:bbox`'tan, nokta sayıları `pc:count`'tan, sistem `proj:epsg`'den ya da `proj:wkt2`'den; eksik
bilgi üyenin başlığından okunur. **Sanal bulut olarak kaydet** aynı biçimi yazar (`geometry` ve `bbox` WGS 84'te, sistemi EPSG'li değilse
`geometry` boş; `proj:bbox` 3B). Web'de (ertelendi) .vpc'nin üyeleri onunla birlikte seçilecek ya da URL olacak.

### 9. Pencereler ve araçlar

- **Nokta bulutu ekle** (CBS'de Veri › Nokta bulutu, CAD'de Ekle › Nokta bulutu): Dosya ya da Adres; üyeler, biçim, sürüm, nokta biçimi,
  nokta sayısı, kapsam, sistem ve kuralın sonucu, dizin gerekip gerekmediği; Ekle yeni katmanı ve nesneyi tek adımda yazar (“Nokta bulutu
  ekle”) ve buluta yakınlaşır. Masaüstünde Bağlı (varsayılan) ya da Göm (32 MB'a kadar).
- **Raster ekle**'ye Adres (URL) seçeneği: yalnız GeoTIFF (COG; PNG ve JPEG dünya dosyalarıyla dosya olarak eklenir); başlık
  parça parça okunur, çizim adresi tutar (Saklama: Adres), önizlemesi olmayan büyük GeoTIFF'in piramidi adres, boy ve sürümle anahtarlanır.
- İki pencere de komutla hemen açılır, dosya penceresi üstündedir: kapatılınca pencere kalır, Dosya seç… ya da Kaynak'ta Adres seçilir.
- **Nokta bulutu stili** (şeritten ve Öznitelikler'in Görünüş satırından): tür, rampa ve aralık (Otomatik), sınıflar (onay kutuları,
  renkleriyle), boy ve birimi, biçim, saydamlık; çizimde canlı önizleme, Uygula tek adım (“Nokta bulutu stili”), Vazgeç iz bırakmaz.
- **Öznitelikler:** Kaynak (üyeler; bağlı yol ve bulunup bulunmadığı, adres, gömülü ad ve boy), Biçim ve sürüm, Nokta sayısı, Kapsam,
  Yoğunluk (nokta/m², kapsamın alanından), Sistem, Dizin (hazır, hazırlanıyor %, durduruldu), Saydamlık; **Sanal bulut olarak kaydet**.
  (Web'in Kaynağı yeniden seç'i ertelendi.)
- **XYZ sor** aracı (§7) ve İşlemler'in araçları.

### 10. Komutlar

`cad.entities.create`'in `pointcloud` geometrisi, işlem `pointCloud` (adım “Nokta bulutu ekle”); `cad.entities.edit`'in `pointCloudStyle`
işlemi (“Nokta bulutu stili”). Ret kodu `invalid_pointcloud` (üye sayısı, kaynak, biçim, kapsam, sayı, stil, saydamlık); gömülü kaynak
`unknown_asset`; blok tanımında `pointcloud_in_block`; taşıma, döndürme, ölçekleme, aynalama, kopyalama ve dizide `pointcloud_fixed`.
Raster kaynağına `url`. Durumlar şimdilik yalnız masaüstünün: `fixtures/commands/v1/desktop/` (web'in koşucusu okumaz).

### 11. Biçimler

DXF'e ve GeoJSON'a nokta bulutu yazılmaz, raporda söylenir. PostGIS izdüşümü kapsamın dikdörtgenidir.

### 12. Performans

- Çözme, dizin, renk ve işlemler hiçbir zaman arayüzün iş parçacığında yapılmaz: iş parçacığı havuzu (çekirdek sayısının bir eksiği,
  en az 1, en çok 4); web'de (ertelendi) nokta bulutu işçisi. Nokta verisi işçiden aktarılan tipli dizilerdir; JSON'a girmez.
- Karede bellek ayırma yok; düğümün konumları bir kez yüklenir, görünüş değişince yalnız renkler.
- Bütçeler (release, geliştirme makinesi): 50 000 noktalı LAZ düğümünün çözülüp renklenmesi ≤ 15 ms; COPC'nin eklenmesinden ilk
  görüntüye (kök düğüm) ≤ 150 ms; dizin hazırlığı ≥ 1 milyon nokta/s; 4 milyon noktalı karede kaydırma ve yakınlaştırma kareyi
  düşürmez. Ölçüler Doğrulama'dadır.

## Kapsam dışı

3B görünüm ve kesit penceresi (§17, `CIVIL-05`), noktaya kenet ve noktadan sayısallaştırma, E57 ve PLY, bulutun yeniden izdüşümü (başka
sistemdeki bulut), nokta bulutunun düzenlenmesi (nokta silme, elle sınıflama), TIN ve eş yükselti eğrileri (`GIS-31`, `GIS-32`),
kimlik doğrulamalı ve vekilli adresler, servisler (`GIS-10`), bulutun pafta çıktısı (§16.4).

## Uygulama

8 Ekim, tek parçada, **yalnız masaüstünde** (sahibin kararı; web'in kısmı ertelendi, düğmeleri notuyla bekler).

- **Sözleşme:** `kentos_contracts::pointcloud` (`PointCloudEntity`, `PointCloudFields`, `CloudSource`, `PointCloudStyle`, `CloudFormat`,
  `CloudRender`, `PointSizeUnit`, `PointShape`; kurallar `problem`, `url_problem`), rasterin `url`'si ve kaynak kuralı; `CreateOperation::PointCloud`,
  `EditOperation::PointCloudStyle`; kodlar `invalid_pointcloud`, `pointcloud_in_block`, `pointcloud_fixed`. `.kcad` şema 31 (`FORMATS_VERSION` 41):
  Rust kodeği ve sütunlar (`crates/shared/kcad`), spesifikasyon §6.1 ve §6.6, örnek `pointclouds.kcad` ve 21 bozuk dosya; bağımsız Python
  okuyucusu (`tools/kcad/kcad.py`) ve yazıcısı (`scripts/fixtures/kcad_v2_reference.py`). Web sözleşmeyi taşır (tipler, `io/columns.ts`'in 18.
  türü, rasterin adresi), çizmez.
- **Çekirdek** `kentos-pointcloud` (`crates/shared/pointcloud`, ev sahibine bağlı olmayan okuyucular): `source` (aşamalı açılış, `Need`/`Step`),
  `las`, `chunks` (laz-rs'in parça çözücü ve sıkıştırıcısı, seçmeli çözme), `copc` (bilgi ve hiyerarşi), `text` (XYZ, PTS, TXT, CSV; ölçek
  dosyanın ondalıklarından), `crs` (WKT ve GeoTIFF anahtarları), `record` (nokta biçimleri, 6–8'e genişletme), `index` (belirlenimci COPC dizini:
  küp, kutular, kutuların ağaçları, yazıcı), `write` (akan LAS ve LAZ yazıcısı), `vpc`, `look` ve `classes` (renkler), `nodes` (düğümün
  noktaları, `Needs`), `place` (örnek ve sistemin kuralı), `ops` (`query`, `region`, `stats`, `convert`, `grid`, `ground`, `height`, `thin`,
  `raster`, `boundary`, `tile`). Geometri çekirdeğinin `geom::pointcloud`'u (`Octree`, `visible`, `near`) ve `Shape::PointCloud`.
- **Çizim hattı:** `crates/render/wgpu/src/styled/points.rs` (bulutun resmi, düğümler GPU'da en az kullanılan önce bırakılarak, karede
  bellek ayırmadan), `shaders/wgsl/points` ve `points.layout.json`, stilli çizimin 7. sürümü (`cloudFs`).
- **Masaüstü:** `apps/desktop/src/pointclouds/` (`add` Nokta bulutu ekle: Dosya ya da Adres, birden çok dosya, .vpc, Bağlı ya da Göm; `look`
  Nokta bulutu stili; `query` XYZ sor; `vpc` Sanal bulut olarak kaydet; `service` düğümler; `index` önbellekteki dizin; `bytes` dosya, HTTP
  aralıkları ve gömülü baytlar; `files` İşlemler'in dosyaları), rasterin adresi (`rasters/tiles.rs`'in `Origin::Url`'si, `rasters/add.rs`'in Adres'i,
  piramidin adresle anahtarı), Öznitelikler'in satırları (`properties/rows/cloud.rs`), şeritte CAD'in Ekle'si ve CBS'nin Veri'si (envanterden).
  Pencereler önce açılır, dosya penceresi üstünde: kapanınca Adres seçilebilir.
- **İşlemler:** `kentos_processing::files` (`Files`, `CloudRead`, `Sink`), `ParamKind::SaveFile`, `Feedback::files`, Nokta bulutu kategorisi ve
  dokuz araç (`builtin/pointcloud/`: alan sorgusu, Seyrelt, Zemin süzgeci, Yüksekliğe göre sınıfla, Bulutu kırp, Bulutları birleştir, Karola,
  Rasterleştir, Sınır çıkar); masaüstünün `DesktopFiles`'ı (parçalar iş parçacıklarında, `.yaziliyor` ve yeniden adlandırma).
- **Ürün komutları:** `create`, `edit`, `transform`, `array`, `blocks_define` masaüstünde; web'in `entitiesTransform.ts` ve `entitiesArray.ts`'i de
  `pointcloud_fixed` ile reddeder.
- **Web:** `app/pointCloudCommands.ts` (13 komut, `pending` ve not), şeritte aynı panel; ikonlar `ui/icons.ts`'te; boş `crates/wasm/pointcloud-wasm`
  web'in modülünü bekler. Veri güvenliği: `.kcad`'in sütunları ve başı (`io/columns.ts`, `io/kcad.ts`'in `OBJECT_FIELDS`'ı), dosyanın
  denetimi (`model/snapshot.ts`), kurallar (`model/pointCloudRules.ts`; komutlarda `invalid_pointcloud`, gömülü dosyada `unknown_asset`),
  geometri deposunun paketi (`wasm/pack.ts`'in 21. türü ve rasterin adresi; çekirdeğin okuyucusu 21'i alır) ve İşlemler'in tür notu.
- **Başvurular:** `pointcloud_cases.py` (okuyucu, laspy ve LASzip), `pointcloud_index_cases.py` (dizin, `--verify`), `pointcloud_ops_cases.py`
  (işlemler, numpy), `pointcloud_scene.py` (sahne), `pointcloud_command_cases.py` (masaüstünün komut durumları, `fixtures/commands/v1/desktop`).
- **Bağımlılık:** `laz` 0.13.0 (Apache-2.0, `parallel` kapalı; `docs/deps/README.md`).

## Doğrulama

**Bağımsız başvurular** (KentOS kodu olmadan; laspy 2.7 ve LASzip'in kendi C++ çözücüsü, numpy): okuyucu `pointcloud_cases.py`
(LAS 1.2–1.4, biçimler 0–8, sistemler, ek baytlar, çok parçalı LAZ, metin bulutları, bozuk dosyalar; kayıtlar FNV-1a ile), dizin
`pointcloud_index_cases.py` (düğüm düğüm noktalar; `--verify` KentOS'un yazdığı COPC'yi LASzip'le nokta nokta denetler), işlemler
`pointcloud_ops_cases.py` (dokuz aracın sonuçları ADR'nin tanımlarından), sahne `pointcloud_scene.py`. Çekirdeğin testleri bu dosyaları
okur; masaüstünün `pointclouds::ops_tests`'i dokuz aracı İşlemler'in penceresinden uçtan uca çalıştırıp aynı başvuruyla karşılaştırır.
Görünüşe göre çözme beş gereksinimle her düğümde bütün kayıtlardan çözülenle aynıdır (`a_pictures_records_hold_what_the_looks_read`);
paralel sıkıştırılan sonuç `put`'un yazdığıyla bayt bayt aynıdır (`chunks_compressed_together_are_the_same_bytes`).

**`.kcad` şema 31:** bağımsız yazıcı 376 dosya (`kcad_v2_reference.py --check`), Python okuyucusu bozuk 21 dosyanın her birini nedeniyle
reddeder; Rust'ta `pointclouds.rs` (şema 31 yalnız bulut ya da adres varken, gidiş dönüş, yazıcının retleri, okuyucunun hata yerleri,
sütunlar) ve `fixtures.rs` (37 geçerli dosya).

**Komutlar:** masaüstünün 11 durumu (`fixtures/commands/v1/desktop`: ekleme, kurallar sırayla, sonlu olmayan, kitaplıkta olmayan, katman,
stil, taşıma, dizi, blok); paylaşılan bütün durumlar iki platformda (rasterin adresi dahil).

**Adres:** yerel bir aralık sunucusuyla (`range_server.rs`): blok önbelleği ve istek sayısı, aralık vermeyen sunucunun reddi, açıldıktan
sonra değişen sürüm, 404; adresten bulut (yalnız başlık, tablo ve örneğin parçaları, her bayt bir kez) ve GeoTIFF (dosyadakiyle aynı
pikseller) ekleme. Gerçek bir uzak sunucu sınanmadı.

**Pencereler:** Nokta bulutu ekle tek adım (katman ve bulut birlikte geri alınır), adresten ekleme, dosya penceresi kapanınca kalan
pencere, XYZ sor uçtan uca (aracın adı, çatıdaki noktanın sınıfı), Sanal bulut olarak kaydet ve geri okuma; resimler 1440×900 ve
1100×650, iki temada (`tools_screens`'in `bulut-*`'ı, `raster-ekle-adres`).

**Süreler** (release; 11th Gen Intel i5-11300H, 4 çekirdek 8 iş parçacığı, 15 GB; Iris Xe ve RTX 3050 Mobile, wgpu varsayılan
bağdaştırıcısıyla; 8 Ekim). Sentetik hava LiDAR'ı:
4 000 000 nokta, 1 km², LAS 1.2 biçim 3, LAZ 51,6 MB (`perf::clouds`); düğüm süreleri çekirdeğin ölçümü (`kentos-pointcloud`'un `timing`'i).

| Ölçü | Sonuç | Bütçe |
|---|---|---|
| Dizin (COPC) hazırlığı | 1,58 s, 2,53 milyon nokta/s | ≥ 1 milyon nokta/s |
| İlk görüntü (COPC'yi açma, kökü çözme ve renkleme; kökte 25 598 nokta) | 8,6 ms | ≤ 150 ms |
| 50 000 noktalı düğüm, tek iş parçacığı: Sınıflar / Yükseklik / Yoğunluk / Dönüşler | 10,3 / 11,1 / 12,2 / 9,8 ms | ≤ 15 ms |
| aynı, Renkler (LAZ'ın renk katmanı tek başına 15,9 ms) | 16,2 ms | ≤ 15 ms: aşılır |
| aynı, bütün alanlar (İşlemler ve XYZ sor'un okuması) | 23,3 ms | — |
| Sığdırılmış görünümün bütün düğümleri (713 100 nokta) çizilene dek | 0,17 s, 9 kare | — |
| Karenin düğüm seçimi | 0,003 ms | — |
| Tam okuma: LAZ / COPC (parçalar iş parçacıklarında) | 9,76 / 6,66 milyon nokta/s | — |
| LAZ yazma: tek iş parçacığı / İşlemler gibi 4 iş parçacığıyla | 3,12 / 10,63 milyon nokta/s | — |
| Zemin süzgeci (SMRF) çekirdeği, iki geçiş | 1,97 s (2,03 milyon nokta/s) | — |
| Yüksekliğe göre sınıfla / Seyrelt (0,5 m) / Rasterleştir (1 m) / Sınır çıkar / Bulutu kırp | 0,24 / 0,72 / 0,34 / 0,05 / 0,11 s | — |
| Kare (1440×900): kaydırma, yakınlaştırma, 1:500'de kaydırma; MİB p50 | 0,56 / 0,85 / 0,57 ms | kareyi düşürmez |
| aynı, GPU p50 (resmin geri okunması dahil; boş çizimde ~8 ms) | 8,6 / 10,6 / 9,8 ms | — |

Renkler görünüşünde 50 000 noktalı düğüm bütçeyi 1,2 ms aşar: süre laz-rs'in renk katmanının çözmesidir (konum 9,5 ms, konum ve renk
15,9 ms) ve tek düğüm içinde paylaşılamaz; dört işçiyle görünüm saniyede ~12 milyon nokta yükler, sığdırılmış görünüm 0,17 s'de tamamdır.
Bütçenin anlamı (bir düğümün gecikmesi) yerine bu iş hacmi ölçülür. Renkler dışındaki her görünüş bütçenin içindedir. Ölçülmeyenler:
gerçek bir uzak sunucudan okuma, başka GPU'lar ve 100 milyon noktadan büyük bulutlar.
