# 0910 — KentOS Yol: ön güzergâh, kavşak ve 3B

Tarih: 2026-11-03. plan/road.md Y12 (ROAD-ALIGN-13; ROAD-JUNC; 3d.md bağlantısı). Durum: taslak, aşamalar aşağıda.

## Dayanak

- **Koridor analizi:** Netpro'nun Koridor Analizi ve Bentley OpenRoads ConceptStation'ın ön güzergâh araçları aks
  tasarımından önce arazide en düşük maliyetli geçişi arar; Esri'nin Cost Path ve Optimal Path aracı aynı ilkeyle
  (birikimli maliyet yüzeyi ve geri izleme) çalışır. KentOS'un raster uzaklık ve maliyet çözümlemesi
  ([ADR 0236](0236-distance-and-cost.md)) ve uygunluk analizi ([ADR 0237](0237-multi-criteria-suitability.md)) vardır;
  yol projesinde arazi bir TIN yüzeyidir ([ADR 0900](0900-road-terrain-surface.md)), maliyet onun eğiminden gelir.
- **Bulunan yol aks değildir:** plan ROAD-ALIGN-13 “ortaya çıkan raster path teknik aks olarak ayrıca fit/QA/onaydan
  geçsin” der. KentOS'ta bu, çizgiden aks oturtmadır ([ADR 0908](0908-road-stakeout-sheets-and-exchange.md) §12,
  `road.alignments.fit`).
- **Kavşak:** Autodesk Civil 3D'nin kavşak sihirbazı
  ([To Create an Intersection](https://help.autodesk.com/cloudhelp/2023/ENU/Civil3D-UserGuide/files/GUID-6C3D8C2B-890B-4A78-90D6-DC71AC82FDC3.htm))
  iki aksın kesişiminde kurulur; kenarlar akslara ötelenmiş hatlardır, bordür dönüşü çeyrek başına yarıçaplı daire yayı
  (ya da pah, üç merkezli yay), “Copy These To All Quadrants” bir değeri bütün çeyreklere verir. KentOS'un ilk kavşağı
  daire yayıdır; pah ve üç merkezli yay, kenarın kendi hattı (ötelenmiş aks) ve kavşak boykesitleri sonraya.
- **3B sahne:** glTF 2.0 ([Khronos belirtimi](https://registry.khronos.org/glTF/specs/2.0/glTF-2.0.html), 3 Kasım
  2026'da okundu) açık, ISO/IEC 12113:2022 olarak standartlaşmış 3B iletim biçimidir: uzunluklar metre, sağ el, +Y
  yukarı; GLB 12 baytlık başlık (`glTF`, sürüm 2, boy) ve dört bayta hizalı iki parça (JSON boşlukla, BIN sıfırla
  doldurulur); POSITION erişicisi `min` ve `max` taşımak zorundadır; normal verilmezse okuyucu düz normal kurar.
  Konumlar float32'dir; plan/3d.md SCENE-PREC-04 bu yüzden her yükün FP64 kökenle ve sınırlı yerel konumlarla
  taşınmasını ister. Düğümün `translation`'ı JSON sayısıdır (f64), kök düğüm kökeni tam taşır. Civil 3D ve OpenRoads
  koridoru 3B modele (katı ya da yüzey) çevirip dışa verir; KentOS'un ilk 3B çıktısı arazi ve koridor üst yüzeyidir.
- **Ortak 3B çekirdek (sahibin isteği, 3 Kasım 2026: “3B sahneyi yüksek performanslı yaz, ileride birçok sahnede ve
  işte kullanacağız, LOD sistemi de olacak”):** plan/3d.md'nin D1'i FP64 kamera, karo yerel FP32, ters derinlik ve
  kaynağa dönen seçimi ister; TileDescriptor sınırlar, FP64 köken, geometrik ve kodlama hatası, çocuklar ve inceltme
  taşır (§8). [3D Tiles 1.1](https://github.com/CesiumGS/3d-tiles/blob/1.1/specification/README.adoc): geometrik hata
  karonun sadeleştirilmiş gösteriminin metre cinsinden hatasıdır; REPLACE inceltmede çocuklar karonun yerine, ADD'de
  onunla çizilir; çocuğun içeriği ebeveynin hacmindedir; dönüşüm geometrik hatayı en büyük ölçeğiyle büyütür.
  CesiumJS'in kaynağı (`Cesium3DTile.getScreenSpaceError`, 3 Kasım 2026'da okundu): perspektifte
  `hata · yükseklik / (uzaklık · 2·tan(fov/2))`, uzaklık en az 1e-7; ortografikte `hata / piksel boyu`. Ters Z
  ([Reed, Depth Precision Visualized](https://www.reedbeta.com/blog/depth-precision-visualized/), NVIDIA'nın yeniden
  yayımı): float32 derinlikle ters Z ve sonsuz uzak düzlem karşılaştırma hatasını en aza indirir. Büyük koordinatta
  Cesium'un göze göre çizimi ve Unreal'ın LWC'si (3d.md §3): büyük sayı GPU'ya varmadan f64'te çıkarılır.
- **Sınırlar:** en dik eğim ve tırmanmanın ağırlığı kurumun ve projenin seçimidir; KentOS değer vermez
  (plan/road.md ROAD-DEC-01).

## Karar

1. **Ön güzergâh (ROAD-ALIGN-13).** `road.routes.find` arazi yüzeyinde iki yer arasında en düşük maliyetli yolu bulur,
   hiçbir şey yazmaz; `road.routes.draw` onu bir katmana kotlu çoklu çizgi olarak çizer (tek adım “Ön güzergâh”). Izgara
   iki yerin kutusu ve çevresinde `margin`, `cell` aralıklıdır (en çok iki milyon düğüm). Düğümün kotu yüzeyden
   okunur ve mikrometreye yuvarlanır (aynı arazi her yerde aynı yolu versin diye); yüzeyin dışındaki ve `avoid`
   katmanlarının alanlarındaki (dış halkanın içi, deliklerin dışı, köşeler düz birleşir, yaylar kirişleriyle)
   düğümlere girilmez. Bir düğüm 16 komşusuna bağlıdır (düz, çapraz ve at hamlesi; yol 45°'lik dönüşlere
   bağlanmasın diye); adım plan boyu artı `climb` × aştığı kot farkı tutar; `grade`'ten (yüzde) dik adım atılmaz.
   En küçük maliyet önce, eşitlikte küçük düğüm önce yerleşir; maliyet yalnız kesin küçülünce değişir, komşular sabit
   sırada denenir. Yolun başı ve sonu sorulan yerlerdir, arası düğümler; `tolerance` ile Douglas–Peucker (planda).
   Cevap: noktalar, plan boyu, toplam kot farkı, en dik bölüm, maliyet, düğüm ve yerleşen düğüm sayısı. At hamlesi
   uçları denetler; tek düğüm kalınlığındaki yasak şeritten atlayabilir: alan en az iki hücre geniş olmalıdır.
   Retler: `invalid_route`, `route_blocked` (baş ya da son girilemeyen düğümde), `route_unreached` (eğim yetmiyor).
   Masaüstünde Güzergâh › Ön güzergâh: yüzey, iki yer (Y, X), hücre, pay, en dik eğim, tırmanma ağırlığı,
   sadeleştirme, kaçınılacak katmanlar; yazıldıkça plan önizlemesi (kaçınılan alanlarla) ve özet; Planda çiz.

2. **Kavşak (ROAD-JUNC-01, 02, 04).** Kavşak yol ayarlarında `junctions[]`'tır: kalıcı kimlik, iki kol (`legs`:
   her biri PI'larıyla kurulan bir aks, aksın kurulduğu sürüm, `left` ve `right` kenarların aksa uzaklığı), hangi
   kesişim (`at`, birinci kolun km sırasıyla), tür (`atGrade` hemzemin: kollar bağlı, dört çeyrekte bordür dönüş yayı;
   `separated` katlı: kollar bağlı değil, yay yok; plan ROAD-JUNC-02: köprünün altındaki yol grafikte bağlanmaz) ve
   hemzeminde `radius` (her çeyrek) ile isteğe bağlı `radii` (dört çeyrek: birinci kolun sağı ile ikincinin sağı, sağ ile
   sol, sol ile sol, sol ile sağ). Kesişim iki aksın her metrede örneklenmesi ve kesişen parçaların iki km üstünde
   Newton yöntemiyle inceltilmesiyle bulunur. Kollar kesişimde teğetleri boyunca düz kabul edilir: her çeyrekte yay iki
   kenara teğet ve ikisinin dışında kalan daire yayıdır (merkez iki kenarın yarıçap kadar dışa ötelenmiş hatlarının
   kesişimi, teğet noktaları merkezin kenarlara izdüşümü), teğet noktalarının kollar boyunca km'si kesişimin km'si artı
   teğet boyunca uzaklık. Bir kol yayların eriminde kurptaysa `junction_on_curve` (yay kolu düz kabul eder); bir kolun
   aksı değişince `junction_stale`, `rebind` aksların bugünkü sürümünü alır. Kesişmeyen kollar, olmayan kesişim ve
   kesişimde paralel kollar ret (`invalid_junction`). Komutlar `road.junctions.edit` (tek adım “Kavşak”),
   `road.junctions.query`, `road.junctions.draw` (hemzeminde dört yay; `labels` ile kesişimde ad ve açı; katlıda ret).
   Masaüstünde Güzergâh › Kavşak.
3. **3B sahne (3d.md'nin ilk yol sahnesi).** Sahne yol projesinin arazi yüzeylerinin üçgenleri ve koridorlarının üst
   yüzeyidir; her biri kaynağından okunduğu anda kurulur, kaynak koordinatında (x doğu, y kuzey, z yukarı) indeksli ağdır
   (koridorun üçgen çorbasında aynı bitlerdeki köşe bir kez). Sahnenin kökeni bütün köşelerin sınırlarının ortasıdır,
   tam metreye yuvarlanır; konumlar kökene göre float32'dir, `error` her köşenin kökene göre koordinatıyla float32'si
   arasındaki en büyük eksen farkıdır (600 m'lik sahnede 1,4e-5 m; 10 km'lik TM sahnesinde 2,5e-4 m'nin altında).
   `file.export.gltf` sahneyi GLB'ye yazar (`kentos_formats::gltf`): kök düğüm projenin adıyla ve kökenle, altında
   her ağ bir düğüm (`extras`'ta tür ve kaynağın kimliği); glTF'in ekseni (doğu, yukarı, −kuzey), dönme olduğundan
   yukarıdan saat yönü tersine üçgen +Y'den önyüzdür; normal yazılmaz; malzemeler Arazi ve Koridor (iki yüzlü).
   Yüzey ve koridor kimlik ya da adla (yoksa hepsi, boş liste hiçbiri); adla seçilen kurulamazsa ya da koridorun üst
   yüzeyi yoksa ret (`invalid_gltf`), seçilmeyen kurulamazsa `export_losses`. Bütün yazılır ya da hiç; çizim değişmez.
   Düşey abartma yalnız görünüştür (SCENE-CAM-14), dosyaya girmez. Masaüstünde Koridor › 3B › glTF ver.

4. **Ortak 3B sahne çekirdeği (`kentos-scene3d`, plan/3d.md D1).** GPU, pencere, dosya bilmez; her 3B görünüşün
   (yol, bina, nokta bulutu, ikiz) CPU tarafıdır, wasm'a da derlenir.
   - *Kamera:* göz, hedef ve yukarı f64'te, kaynağın koordinatında (gösterilen uzayda); perspektif (`fov_y`, `near`,
     uzak düzlem sonsuzda) ya da ortografik (`height`, `depth`). GPU'ya ötelemesiz `P · R` gider (küçük sayılar) ve
     karo başına `gösterilen(köken) − göz` f64'te hesaplanıp float32 yapılır; vertex gölgelendiricisi karonun yerel
     float32'sini (z abartmayla çarpılmış) ona ekler: büyük sayı hiç yuvarlanmaz (SCENE-PREC-05, 08). Derinlik ters:
     perspektifte `z/w = near / derinlik`, ortografikte `1 − derinlik / depth`, `Greater`, temizleme 0 (SCENE-CAM-05,
     06). Düşey abartma yalnız görünüştür: `datum + k · (z − datum)`; kamera gösterilen uzayda yaşar, karonun hatası
     `max(1, |k|)` ile büyür (3D Tiles'ın ölçek kuralı).
   - *Frustum:* yakın düzlem ve dört yan (ortografikte uzak da) `n · (p − göz) + d ≥ 0`; kutu p-köşe sınamasıyla
     dışarıda, kesişen ya da içeride (muhafazakâr: dışarıda denen dışarıdadır).
   - *Karo:* anahtar, kaynak koordinatında kutu, metre cinsinden geometrik hata, REPLACE ya da ADD, isteğe bağlı içerik
     (kimlik, GPU'daki bayt), çocuklar; hiyerarşinin kuralları (tek ebeveyn, döngü yok, kökten ulaşılır, çocuğun hatası
     ebeveyninkini aşmaz, kutusu içindedir, 1 µm pay).
   - *Seçim:* görünmeyen atlanır; içeriği olmayan karo hep iner; hatası sınırın altında ya da çocuksuz karo içeriği
     hazırsa çizilir, değilse istenir; REPLACE yalnız görünen her çocuk şimdi kendi payını örtebiliyorsa iner, yoksa
     ebeveyn çizilir ve çocuklar istenir (boşluk yok, ebeveyn ve çocuk birlikte çizilmez: SCENE-STREAM-04, 05); ADD
     karo çocuklarıyla çizilir; istekler hataya göre büyükten küçüğe, sonra anahtara göre (SCENE-STREAM-08).
   - *Bölme:* büyük üçgen ağları üçgenlerin ağırlık merkezleri üstünde ikili ağaçla, merkezlerin kutusunun en uzun
     yanında medyandan (anahtar merkez, parça, sıra; alt yarı `n/2` en küçük) karo en çok `max_triangles` ve en çok
     `max_extent` metre olana dek bölünür (öntanımlı 32 768 ve 1 000 m: 500 m içindeki float32 yarım ULP 0,03 mm).
     Her yaprak bir içerik: köşeleri bir kez, kendi f64 kökenine (kutusunun ortası) göre float32; üçgenleri parçaya göre
     aralıklarda (bir parça bir stil); her üçgenin kaynağı (parça, sıra) seçimin kaynağa dönmesi için (SCENE-STREAM-17);
     kodlama hatası raporda. İç karoların içeriği yok, hatası kutularının köşegeni; yaprağın hatası 0 (kaynağın
     kendisi). Numaralama derinlik önce, önce alt yarı; büyük ağın üst dört düzeyi iş parçacıklarında kurulur (tarayıcıda
     sırayla), sonuç aynıdır. Sadeleştirilmiş ebeveyn içerikleri (gerçek LOD) aynı hiyerarşiye sonra eklenir; seçim onlar
     için şimdiden hazırdır.

## Aşamalar

Y12.1 (yapıldı, ön güzergâh): `road.routes.find` 26, `.draw` 22 ortak durum (`road_route_cases.py`; arazinin kotları
road_profile_cases.py'nin TIN'inden, arama §1'den; KentOS kodu yok; sınırlar durumlarındır). Çekirdek
`kentos_road::route`. Python, MCP (`tests/road_routes.rs`: bul, yasak alandan dolaş, çiz ve aks olarak oturt),
masaüstünde Güzergâh › Ön güzergâh. Ayarı yazılmaz, `.kcad` değişmez.

Y12.2 (yapıldı, kavşak): `road.junctions.edit` 29, `.query` 20, `.draw` 21 ortak durum (`road_junction_cases.py`;
akslar road_alignment_cases.py'nin 40 basamaklı başvurusundan, kesişim 40 basamakta Newton'la, yaylar §2'den; KentOS
kodu yok). Çekirdek `kentos_road::junction`, KCAD şema 161'in `junctions`'ı bağımsız okuyucuda
(`broken/road-junction-on-parallel-axis.kcad`; örnek `road-work.kcad`'e bağlantı yolu aksı ve kavşağı eklendi). Python,
MCP (`tests/road_routes.rs`: örneğin kavşağı sorulur, genişletilir, çizilir, katlıya çevrilir), masaüstünde Güzergâh ›
Kavşak: kollar, kenarlar, tür, yarıçap ve çeyrek yarıçapları; yazıldıkça kesişimin çizimi, km'ler, açı ve çeyreklerin
tablosu; Kaydet, Sil, Planda çiz.

Y12.3a (yapıldı, glTF): `file.export.gltf` 21 ortak durum (`road_scene_cases.py`; arazi profil başvurusunun Triangle
üçgenlemesi, koridorlar road_corridor_cases.py'nin başvurusundan, float32 farkı numpy'ın float32'siyle; KentOS kodu
yok). `--written` KentOS'un yazdığı `yol.glb`'yi kendi GLB okuyucusuyla (başlık, parçalar, doldurma, min ve max,
üçgenler başvuruyla, yönler) ve trimesh'le okur; Khronos glTF-Validator kuruluysa onu da koşar. Python, MCP
(`tests/road_routes.rs`), masaüstünde Koridor › 3B › glTF ver.

Y12.3b (yapıldı, sahne çekirdeği): `kentos-scene3d` (§4). Ortak durumlar `fixtures/scene3d/v1/cases.json`
(`scene3d_cases.py`; kurallar ADR'den, numpy'ın float32'siyle, KentOS kodu olmadan): 8 kamera (matris, frustum
kararları, piksel ve derinlik, ışın, karonun göze göre kökeni; TM koordinatında ve abartmalı), 23 seçim (dörtlü ağaç,
eksik çocuk, ADD, içeriksiz kök, ortografik, abartma, dar pencere, göz karoda, ufka bakış, bölünmüş arazi), 8 bölme
(sınırlar, parçalar, TM'de 2,4 km, aynı merkezli ve bozuk üçgen); float32 konumlar bit bit aynı. Süreler (release, M5):
4 milyon üçgenlik 2 km × 1 km TM arazisi 175 ms'de 255 karo, 128 içerik (80,5 MB), en büyük float32 farkı 2,4e-7 m;
bir karenin seçimi 0,004–0,011 ms; 21 845 karolu dörtlü ağaç üstünde uçuşta 0,003 ms. Bağımlılık yönü temiz, wasm32
derlenir.

## Sonra

Arazinin koridorla kesilmesi (yarmada araziyi koridorun altından çıkarmak), koridorun katı modeli (üstyapı
katmanları), arazinin hatlarla ve ortofotoyla kaplanması, glTF'te `KHR_mesh_quantization`, büyük sahnede karolara
bölünmüş kökenler (SCENE-PREC-11) ve 3D Tiles; maliyetin arazi örtüsü ve kamulaştırma rasterleriyle çok ölçütlü bileşimi (ADR 0237'nin uygunluk yüzeyi
maliyet olarak), birden çok seçenek yolun karşılaştırılması.
