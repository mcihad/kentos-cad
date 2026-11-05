# 0955 — Taşıma ağı: yol dışındaki alanlar ve gerçek bağlantı

Tarih: 2026-11-05. Durum: kabul (Y49). Kapsam ROAD-HAUL-06; ADR 0936'nın "alanlara yoldan ayrılan erişim yolu" açığı.

## Sorun

Ödünç ve döküm alanları aksın bir km'sinde duruyordu (ADR 0907, 0924). Gerçekte ocak ya da döküm sahası yoldan uzakta
olur ve ona bir erişim yoluyla gidilir; o yolun boyu her m³'ün taşıma uzaklığına eklenir. ROAD-HAUL-06 ayrıca şunu
istiyor: "Haul network bağlantısı yalnız plan çizgilerinin ekranda kesişmesi sayılmasın; gerçek source/destination/
topology gereksinimleri denetlensin."

## Araştırma

- **Trimble Business Center:** Taşıma yolları çizgiden dönüştürülür ve alanları koridora bağlayan bir ağ kurar.
  - İki taşıma yolu, biri ötekine bağlantı toleransı içinde başlıyor ya da bitiyorsa birleşmiş sayılır ("Any haul
    roads that start or finish within this distance to other haul roads will be considered joined").
  - Ödünç alanı yalnız bir taşıma yoluyla koridora bağlıysa hesaba girer. Alanın yola uzaklığı için ayrı bir "site
    connection tolerance" vardır.
  - Yolların maliyeti ve yönü (tek yön) vardır.
  - Kaynaklar: TBC yardımı, "Create and Edit Haul Roads for Mass Haul", "Create Earthwork Sites", "Workflow: Perform
    a Corridor Mass Haul".
- **Civil 3D:** Ödünç ve döküm alanlarını yalnız km ve kapasiteyle alır; erişim yolu yoktur.

## Karar

### Model (KCAD şema 173)

- **Brükner'in `network`'ü:** `layers` (yolların katmanları), `connect` (bağlantı toleransı, m) ve isteğe bağlı
  `reach` (aksa erişim uzaklığı, m; yoksa `connect`).
- **Alanın `at`'ı:** yol dışındaki konumu (`x` doğu, `y` kuzey). Konumu olan alan ağ ister.

Katmanlardaki çizgi ve çoklu çizgiler yoldur. Çok parçalı çoklu çizgide her parça ayrı bir yoldur. Yaylar en çok
0,01 mm sehimli kirişlerle düzleştirilir; uzunluk hatası L·s/(3R)'dir (R = 20 m, L = 100 m'de 1,7·10⁻⁵ m).

### Bağlantı (`kentos_road::haul_network`)

TBC'nin kuralıdır, dört parçası var:

1. **Yolun yola bağlanması:** Bir yolun ucu başka bir yola `connect` içindeyse ikisi birleşir.
   - Ucu ötekinin ucuna değiyorsa iki uç birleşir.
   - Ucu ötekinin gövdesine değiyorsa T birleşimidir; öteki o noktada bölünür.
2. **Kesişme bağlantı değildir:** Gövdeleri yalnız kesişen iki yol birleşmez ve bu `crossing` olarak söylenir.
3. **Yolun aksa bağlanması:** Ucu aksın orta çizgisine `reach` içinde olan yol, ayağının km'sinde aksa bağlanır.
   Aksı gövdesiyle kesmek bağlamaz ve `crossesAlignment` olarak söylenir.
4. **Alanın yola bağlanması:** Alan, `connect` içindeki en yakın yola bağlanır; yol o noktada bölünür.

Bunun dışında hiçbir şeye bağlanmayan uç, en yakın yola ya da aksa ne kadar uzak olduğuyla söylenir (`loose`, `gap`).
Böylece toleransı biraz aşan bir açıklık kapatılabilir.

### Erişim

Bir alanın erişimi, birleşmiş yollar boyunca aksa giden en kısa yoldur (Dijkstra). Eşit uzunlukta iki yol varsa
küçük km'de buluşan seçilir. Bulunan yol şunları verir:

- **Alanın km'si:** erişimin aksla buluştuğu km.
- **Erişim boyu:** her m³'ün yolda gittiğine ek olarak gittiği uzaklık.

Üç çözücüde (moment, maliyet, ayrılmış malzeme; ADR 0924, 0925, 0944) erişim boyu, alanın verdiği ya da aldığı her m³
için ek uzaklıktır:

- **Moment ve ayrılmış malzeme:** Alan kenarının birim maliyeti, erişim boyudur.
- **Maliyet:** Kaynak–hedef yayının uzaklığına iki ucun erişimi eklenir. Fazla taşıma bu toplamın serbest taşımayı
  aşan kısmıdır.

Moment, yol boyu akışlar ile Σ kullanım × erişim'in toplamıdır. Plan erişim momentini ayrıca verir (`accessMoment`).

### Bağlanmayan alan

Ağın ulaşmadığı alan hesaba katılmaz: kapasitesi 0'dır ve `haul_site_unconnected` uyarısı nedenini söyler. Bu,
TBC'nin bağlı olmayan ödünç alanını dışarıda bırakmasıyla aynıdır. Neden şunlardan biridir:

- en yakın yol tolerans dışında;
- katmanda yol yok;
- bağlandığı yollar aksa ulaşmıyor.

### Aynı yerleşim her yerde

Brükner diyagramı (`road.hauls.query`), taşıma planı (`road.hauls.plan`), onay (ADR 0954), Planı yaz ve
`road.hauls.network` alanı aynı yardımcıdan (`road_hauls_network::placed`) okur.

### Komutlar

- **`road.hauls.network` (yeni, salt okur):** yollar ve uzunlukları; aksa bağlandıkları km'ler; birleşmelerin
  sayısı; birleşmeyen yerler; her konumlu alanın erişimi, aksa bağlandığı km ve kayma, yolları ya da neden
  bağlanmadığı.
- **`road.hauls.edit`:** `network` alır (null kaldırır). Katmanın çizimde olduğunu denetler.

### Pencere ve çıktı

- **Brükner:** Taşıma ağı katmanı, bağlantı toleransı ve aksa erişim alanları; alan tablosunda Doğu ve Kuzey.
- **Taşıma planı:** "Resim: Akış | Ağ".
  - Ağ görünüşü düzlemde aynı ölçekle çizilir: aks, yollar (alanların yolları koyu), aksa bağlantılar ve km'leri,
    alanlar (bağlanmayan içi boş), birleşmeyen kesişme (×) ve boşta uç (halka).
  - Altında birleşmeyen yerler ve erişim tablosu durur.
  - Alanlar tablosunda erişim sütunu, figürlerde "Erişim yolu" vardır.
- **Planı yaz:** Alanlar sayfasına erişim, erişim momenti ve ağ durumu; Özet'e erişim yollarının momenti eklenir.

### Gösterge sırası (`kentos_ui::widget::StatRow`, DESIGN.md §7.19)

Bu işte ortak bileşen olarak eklendi. Değer kutuları büyük sayılarda alt satıra taşıyordu: 1100 px pencerede
"1 974 258 m³·m" dört satırdı.

- Değer tek satırdır ve kartına sığan en büyük basamağı alır (figür, 18, 16, başlık, alt başlık, gövde).
- Birim değerden ayrılıp küçük yazılır.
- Başlık tek satırdır; sığmazsa sonu "…" ile kısalır (`Elided`).
- Kartlar eşit genişlikte ve eşit yüksekliktedir.

## Ölçüm

### Bağımsız referans

`scripts/fixtures/road_haul_network_cases.py`, KentOS kodu kullanmaz:

- yollar aksın mpmath referansıyla km ve kaymaya kurulur;
- uzaklıklar ve kesişmeler GEOS (shapely) ile hesaplanır;
- yayın boyu R·|sweep| olarak tam hesaplanır;
- en kısa yol için Dijkstra kullanılır.

| Komut | Durum |
|---|---|
| `road.hauls.network` | 21 |
| `road.hauls.plan` | 9 yeni, toplam 65 |
| `road.hauls.query` | 1 yeni |
| `road.hauls.edit` | 5 yeni |

Rust'ta ve Python'da hepsi geçer.

### Bulunan erişimler

- **Dere döküm:** km 1+100'deki 74 m'lik yol üzerinden.
- **Tepe ödünç:** iki yoldan biri yay. km 1+450'ye giden yol 142,5 m, km 1+520'ye T yolundan giden 125,5 m; ikincisi
  seçilir.
- **0,8 m açıkta kalan uç:** 0,5 m toleransta `loose` (0,80 m), 1 m toleransta birleşir.

### Erişimin plana etkisi

28 116,2 m³ yarma tek döküm alanına gidiyor:

| Alan | Moment |
|---|---|
| Yol üstünde | 8 143 051,8 m³·m |
| 74 m erişimle | 10 223 662,3 m³·m |

Erişim momenti 2 080 604,1 m³·m'dir, yani 74,0 × 28 116,2. Aradaki 6 m³·m, alanın km 100'de değil ağın 99,9996'sında
durmasından gelir.

### Sayısal tolerans

HiGHS, 10⁷ m³·m'lik momentte ~10⁻⁹ göreli tolerans bırakıyor; bir alanın kullanımı bu yüzden ~7·10⁻³ m³ oynuyor.
Teklik denetimi 1e-3 + 1e-6·|değer| olarak göreli yapıldı; iki durum göreli toleransla karşılaştırılıyor.

## Açık

- **Yol özellikleri:** TBC'deki gibi tek yönlü yol ve yolun kendi maliyeti yok.
- **Erişimin kotu:** Kamyon filosu (ADR 0936) erişim yolunun kotunu ve eğimini henüz yürümüyor; erişim yalnız
  uzaklık olarak giriyor.
- **Birden çok bağlantı:** Bir alanın aksa birden çok bağlantısı olduğunda en kısa olanı kullanılıyor. Akışa göre
  bağlantı seçmek, alanı paylaşılan kapasiteli sanal alanlara bölmeyi gerektirir.
