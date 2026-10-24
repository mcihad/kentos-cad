# ADR 0630 — Harita sisteminde rasterler, resimler ve nokta bulutları; yerel CRS kapsamı

Durum: kabul edildi (25 Ekim 2026). Kapsam: plan/gis.md `CBS-07`'nin kalanı (ADR 0583'ün “Kalan”ı: harita sisteminde rasterler,
nokta bulutları ve resimler; genel PROJ bağdaştırıcısı; harita sisteminde pafta). Kapsamı ben belirledim. Bağlam belgeleri: ADR
0157 (karolara göre konumlar), 0192 (resim nesnesi), 0204 (raster katmanları), 0207 (nokta bulutu), 0208 §3 (servis karolarının
ağı), 0504 (kutu dönüşümü), 0519 (düşey sistem ve epok), 0583 (katmanın kaynak sistemi ve harita sistemi). Web: ertelendi
(ADR 0228).

## Bağlam

ADR 0583 projenin harita sistemini getirdi. Vektörler stil motorunun toplayıcısında harita sistemine taşınır; kamera imleci,
tıklamayı ve keneti ters dönüşümle projenin noktalarına çevirir. Rasterler, resimler ve nokta bulutları ise harita sistemi
projeninkinden farklıyken çizilmiyordu; durum çubuğu bunu bir kez söylüyordu. Bu üç tür piksellerini ya da noktalarını projenin
sisteminde taşır:

- raster bir afinle karolara bölünür;
- resim çerçevesinin köşesinden, dönüşünden ve boyundan okunur (gölgelendirici her pikselin resim koordinatını çerçeveden hesaplar);
- nokta bulutunun düğümleri merkezlerine göre kayık noktalardır.

Doğru bir harita, piksellerin ve noktaların da çizgiler gibi taşınmasını ister.

Planın tamamlanma ölçütü de ayrıca şunları ister:

- desteklenen yerel Rust CRS kapsamı ile genel PROJ bağdaştırıcısının kapsamının açık olması;
- bilinmeyen sistemin tahmin edilmemesi.

## Karar

### 1. Raster: karolar harita sistemine ağla taşınır

Raster çekirdekte projenin sisteminde kalır: afini, karoları ve önizleme katları değişmez. Harita sistemi farklıyken raster geçişi
(`raster_tiles::RasterQuads::add`) şöyle çalışır:

- **Görünen kutu:** harita sistemindeki kutu, kenarları 16'şar noktada yürünerek ve ortasıyla geri çevrilir
  (`MapProjection::box_back`). Görünen karolar projenin sisteminde bu kutuyla bulunur.
- **Karonun katı:** projenin bir metresinin görünümdeki pikselinden seçilir. Haritanın görünümün ortasındaki doğrusal kısmı
  (`linear_at`, merkezî farklar) metre başına pikseli verir.
- **Karonun ağı:**
  - Her karo n × n hücreli bir ağdır. Düğümleri tek tek harita sistemine taşınır (`MapProjection::grid`).
  - n, 1'den 32'ye kadar ikinin kuvvetidir: karonun iki üçgeninin kenar ortalarının ve ağırlık merkezinin harita sisteminde
    köşelerin ortalamasından sapması, her yarılanmada dörtte bire iner.
  - En küçük n seçilir; sapma toleransı geçmez. Tolerans görünümün yarım aygıt pikselidir, haritanın kendi toleransının onda
    birinden ince değildir (`grid_cells`).
- **Önbellek:** karonun ağı karoyla ve harita sistemiyle önbellektedir. Kare başına dönüşüm yapılmaz; 4096 ağdan sonra önbellek
  boşaltılır.
- **Çizim:** hücreler, raster atlasının dokusuyla raster boru hattında çizilir. Uv'ler karonun dilimi içinde düğüme göre
  doğrusaldır. Kaba karonun parçası kullanılırken (karo hazırlanırken) aynı ağ kullanılır.

Stil toplayıcısı rasterin çerçevesini yalnız partinin kutusu için harita sistemine taşır; afin projenin sisteminde kalır.

### 2. Resim: üçgenleri yarılanarak harita sistemine taşınır

Harita sistemi farklıyken bir resmin dolgusu (`BatchSink::warp_picture`) şöyle kurulur:

- **Ağ:**
  - Çerçeve kırpılmış hâliyle projenin sisteminde üçgenlenir.
  - Her üçgen, haritanın büküldüğü kadar dörde bölünür (`mesh_level`: kenar ortalarının ve ağırlık merkezinin sapması dörtte
    birle iner; tolerans haritanın kendi toleransı, yaklaşık 5 cm ya da 5·10⁻⁷°).
  - Her küçük üçgenin köşeleri tek tek taşınır (`MapProjection::mesh`).
  - Küçük üçgenler en çok 200 000'dir; bölme düzeyi bu sınırla kısıtlanır.
- **Parti:**
  - Partinin sayıları x, y, u, v'dir: köşe harita sisteminde, resim koordinatı projenin noktasından `imageFs`'in kuralıyla
    (dönüş, ayna).
  - Parti `"warped":true` der. Kırpma korunur, çünkü ağ kırpılmış çerçevenin üçgenleridir.
- **Çizim:**
  - wgpu'da resim kendi dokusuyla raster boru hattında çizilir (`FillPaintBatch::Image { warped }`).
  - CPU çizicisi her pikselin resim koordinatını küçük üçgeninde ağırlık merkezli koordinatlarla hesaplar
    (`cpu.rs`'in `warped_picture`'ı).
  - Gölgelendirici değişmez. Web'in donmuş yolu bu partiyi hiç görmez: web'de harita sistemi yoktur.

### 3. Nokta bulutu: düğümler yüklenirken harita sistemine taşınır

Harita sistemi farklıyken noktalar geçişi (`PointsGpu::prepare`) şöyle çalışır:

- **Görünen düğümler:** projenin sistemindeki görünen kutuyla (`box_back`) ve projenin metresinin pikseliyle bulunur. Metre
  boyutlu noktanın boyu da bu piksele göredir.
- **Düğümün taşınması:** düğüm GPU'ya yüklenirken (`on_map`) taşınır:
  - merkezi tam olarak taşınır;
  - kare alanı, haritanın toleransının onda biriyle (5 mm ya da 5·10⁻⁸°) en çok 64 × 64 hücreye bölünüp düğümleri taşınır;
  - her nokta hücresinde çift doğrusal olarak alınır, kotu kalır.
- **Önbellek:** düğümün kimliği harita sistemini de taşır. Taşınan düğüm harita sistemi değişene dek GPU'da kalır.
- **Gölgelendirici:** gölgelendirici ve `points.layout.json` değişmez.

Tek doğrusal kısım yetmez: 2 km'lik kök düğümün köşesinde Web Mercator'da 17 cm sapar (`map_warp_cases.py`'nin `linear`
durumları). Yakından bakılınca kök düğümün noktaları yaprakların noktalarından kayardı.

### 4. Yerel CRS kapsamı; PROJ bağdaştırıcısı yok

KentOS koordinatları kendi Rust motoruyla dönüştürür. Masaüstü, başsız sunucu, Python ve MCP aynı motoru ve aynı kaydı kullanır.

- **Kapsam sayfası:** kapsam `docs/crs/coverage.md`'dedir. Sayfa kayıttan ve çekirdeğin sistem ve datum türlerinden
  `scripts/crs/coverage.py` ile üretilir. Çekirdeğe yeni bir tür gelince denetim düşer.
- **Kapsanan:** coğrafi, Transverse Mercator (UTM dahil), Web Mercator ve yerel sistemler; TUREF, ED50, WGS 84 ve projenin 7
  parametreli datumu; NTv2.
- **Kapsam dışı:** başka izdüşüm yöntemleri, kayıtta olmayan EPSG kodları ve okunamayan WKT ya da PROJ metni adıyla reddedilir
  (`crs_unknown`, `crs_text_unreadable`). Düşey ve epok dönüşümleri ADR 0519'daki gibi reddedilir. Benzer bir sistem tahmin
  edilmez.
- **PROJ:** genel PROJ bağdaştırıcısı eklenmez. PROJ bir C kütüphanesidir ve sahibin onayı olmadan bağımlılık olarak eklenmez.
  Gerekirse ayrı bir karar konusudur.

### 5. Pafta projenin sisteminde basılır

Paftanın haritası projenin sisteminde çizilir; harita sistemi yalnız çizim alanının görünümüdür:

- Paftanın ölçeği gerçek uzunlukların kâğıttaki oranıdır. Harita sisteminin ölçek bozulması (Web Mercator'da 40° enlemde 1,3
  kat) bu oranı bozardı.
- Paftanın PDF'i, atlası ve ölçeği ADR 0583'ten önce ne idiyse odur.

Harita sisteminde basılmış bir pafta isteği olursa paftanın kendi izdüşümü ayrı bir karar konusudur.

### 6. Durum çubuğu

ADR 0583'ün uyarısı kalktı: harita sistemi seçilince resimler, rasterler ve nokta bulutları da çizilir. “Çizim dönüştürülmedi”
iletisi kalır.

## Doğrulama

- **`scripts/fixtures/map_warp_cases.py`:** 55 ortak durum, KentOS kodu olmadan, PROJ'la (pyproj 3.7.2, PROJ 9.7.1) ve
  `crs_transform_cases.py`'nin EPSG yollarıyla. Bir TUREF / TM36 projesi beş harita sisteminde görülür: Web Mercator, WGS 84,
  TUREF / TM30, WGS 84 / UTM 36N, ED50 / TM30.
  - Raster karoları: 100 m, 5 km ve 80 km, dönük.
  - Resimler: 60 × 40 m'lik fotoğraf ve 6 × 4 km'lik pafta.
  - Doğrusal kısım: 20 m ve 2 km.
  - Bulut düğümleri: 20 m ve 2 km.
  - Görünen kutular: 2 km ve 200 km.
  - Betik her ağın gerçek sapmasını PROJ'a karşı hücre başına 6 × 6 ya da 7 × 7 noktada ölçer. En büyüğü toleransın 0,51
    katıdır; 1,5 katı aşan durum betiği düşürür.
  - Bir eşiğin yanındaki durum da betiği düşürür: KentOS ile PROJ'un TUREF'teki 0,1 mm'lik farkı hücre sayısını değiştirmesin.
- **`kentos-project`'in `map_warp_tests`'i:** aynı durumlar çekirdekte. Hücre sayıları ve düzeyler tam; noktalar 1 mm ve 10⁻⁸°
  içinde.
- **Stil çekirdeği:** `a_picture_on_another_map_is_a_mesh_with_its_coordinates` testi. Web Mercator'da 6 km'lik paftanın 128
  üçgeni var; her köşenin resim koordinatı, PROJ'un taşıdığı yere 1 cm içinde düşer.
- **Masaüstü resimleri** (`.run/shots/arac-*-harita-*`):
  - `raster-harita-cografi`, `raster-harita-mercator`, `bulut-harita`: wgpu'yla.
  - `resim-harita`: wgpu'yla ve CPU çizicisiyle.
  - Parseller rasterin, bulutun ve resmin üstüne oturur.

  ```text
  KENTOS_SNAPSHOT_BACKEND=wgpu KENTOS_SHOTS_ONLY=raster-harita-cografi,raster-harita-mercator,bulut-harita,resim-harita cargo test -p kentos-desktop --bin kentos-cad tools_screens::screens -- --ignored --nocapture
  ```

- **`scripts/crs/coverage.py --check`:** kapsam sayfası kayıtla ve çekirdekle aynı.
- **Süreler** (release, bu makine; `cargo test --release -p kentos-project --lib map_warp_timing -- --ignored --nocapture`):

  | İş | Web Mercator | WGS 84 |
  |---|---|---|
  | Görünümün 40 karosunun ağı (ilk kez, sonra önbellekte) | 0,61 ms | 0,48 ms |
  | 6 × 4 km paftanın ağı (128 üçgen; yapılırken bir kez) | 0,10 ms | 0,07 ms |
  | 2 km düğüm, 100 000 nokta (8 × 8 hücre; yüklenirken bir kez) | 1,38 ms | 1,36 ms |
  | Görünümün kutusunun geri çevrilmesi (kare başına) | 0,048 ms | 0,046 ms |

## Sonuçlar

- `CBS-07` kapanır. Bir TUREF / TM36 projesinde WGS 84, Web Mercator ve TM kaynakları aynı harita sisteminde hizalanır:
  - vektörler ADR 0583'le, rasterler, resimler ve bulutlar bu kararla taşınır;
  - harita sistemini değiştirmek kaynak koordinatlarını değiştirmez;
  - tıklama ve koordinatla sorgu aynı nesneyi bulur (`map_crs_tests`);
  - bilinmeyen sistem tahmin edilmez;
  - eksen sırası ve Y, X gösterimi ADR 0583 §4'teki ayrı durumlarla denetlenir.
- Açık kalanlar ayrı madde olur:
  - harita sisteminde pafta (paftanın kendi izdüşümü);
  - genel PROJ bağdaştırıcısı (sahibin onayıyla);
  - web'in harita sistemi (TODOS.md §0.2).
