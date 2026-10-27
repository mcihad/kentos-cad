# ADR 0704: Yükseklikten tasarlanan 2B merdiven

- **Durum:** kabul edildi (27 Ekim 2026); ARCH-STAIR-01'in ilk dilimi.
- **Bağlam:** `plan/architecture.md` ARCH-STAIR-01; ADR 0343 (aks, kolon, merdiven ve rampa: yürüme çizgisi, rıht
  istasyonları, sahanlık, kesme, numaralar, denetimler), ADR 0628 ve 0657 (etkin aracın görünür ayarları).
- **Web:** ertelendi (ADR 0228); TODOS.md §0.2'ye satırı yazıldı.

## Bağlam

ADR 0343'ün merdiveni bir yürüme çizgisi (iki nokta düz, üç L, dört U) ve verilen rıht sayısı, rıht yüksekliği ve
basamakla çizilir. Gerçek GUI'de üç kusur görüldü:

1. Araç 18 rıht × 0,17 m ile başlıyordu; kat yüksekliğinden rıht sayısını kullanıcı elle hesaplıyor, yol kısa ya da
   uzun çizilince yalnız uyarı (`stair_short`, `stair_long`) alıyordu. Önizleme yalnız merdivenin bandını çiziyordu;
   rıhtlar, kesme ve ok yazılmadan görünmüyordu.
2. U merdivende kollar arası dar olunca (kol eni 1,20 m, kol eksenleri arası 1,30 m) iki dönüşün sahanlıkları ayrı
   kalıyor, aralarındaki 10 cm'ye bir rıht düşüyordu: plan yanlış, rıht sayısı bir eksik.
3. Kesme çizgisinin üstündeki rıhtlar hiç çizilmiyordu; plan geleneği onları kesikli gösterir.

Araştırma (üretici belgeleri; yerelde çalıştırılmadı):

- [Revit 2026 — Create Stairs and Railings (video)](https://help.autodesk.com/cloudhelp/2026/ENU/Revit-GetStarted/files/GUID-842A4DB6-4CA1-4ECD-9009-962769FFB19C.htm):
  rıht sayısı iki kat arasındaki yükseklikten ve tipin hesap kurallarından gelir; kol çizilirken imleç ilerledikçe
  yaratılan ve kalan rıht sayısı görünür; kolun ucu tıklanınca hesaplanan bütün rıhtlar oluşur.
- [Revit — Stair Component Instance Properties](https://help.autodesk.com/cloudhelp/2014/ENU/Revit/files/GUID-541EB756-8376-4522-BFA1-141C22346ED8.htm):
  istenen rıht sayısı katlar arası yükseklikten hesaplanır; gerçek rıht yüksekliği tipin en büyük rıht yüksekliğine
  eşit ya da ondan küçüktür.
- [Revit 2025 — Modify Stair Dimensions](https://help.autodesk.com/cloudhelp/2025/ENU/Revit-ArchDesign/files/GUID-7EA40FA9-3128-45B2-B794-8E33036E9D9B.htm):
  rıht sayısı en büyük rıht yüksekliğiyle değişir; yükseklik değişince gerçek rıht yüksekliği uyar, rıht sayısı ve
  basamak değişmez.
- [Archicad 29 — Stair Settings: Geometry and Positioning](https://help.graphisoft.com/AC/29/INT/_AC29_Help/040_ElementsVB/040_ElementsVB-187.htm):
  rıht sayısı ve rıht yüksekliği birbirine bağlı, Kurallar ve Standartlar aralıklarıyla sınırlı; basamak derinliği
  esnek (çizimden) ya da sabit; yürüme çizgisi normalde ortada, genişliğin yarısı ötede.
- [Archicad 24 — Stair Settings: Rules and Standards](https://help.graphisoft.com/AC/24/INT/_AC24_Help/040_ElementsVB/040_ElementsVB-195.htm):
  rıht ve basamak için tercih aralıkları; girdi kuralla çelişirse seçenekler önerilir.
- [Vectorworks 2025 — Stair settings: Geometry tab](https://app-help.vectorworks.net/2025/eng/VW2025_Guide/Stairs/Stair_settings_Geometry_tab.htm):
  toplam yükseklik rıht sayısı × rıht yüksekliğidir; kilitli olmayan değer yeniden hesaplanır; adım uzunluğu 2R + G.
- [Vectorworks 2026 — Drawing a stair with Vertex mode](https://app-help.vectorworks.net/2026/eng/VW2026_Guide/Stairs/Drawing_a_stair_with_Vertex_mode.htm):
  düz merdiven iki, L üç, U dört köşeyle çizilir.

KentOS kararı: Revit'in “yükseklikten ve en yüksek rıhttan sayı” kuralı ve çizerken görünen sayı, Archicad'in ortadaki
yürüme çizgisi ve sabit basamağı, Vectorworks'ün köşe sayısıyla düz/L/U'su alınır. Alınmayanlar: tipe bağlı kural
tabloları, balans basamak ve dönen basamaklar, otomatik yönetmelik uygunluğu. En fazla rıht (0,175 m), basamak
(0,28 m) ve plan kesme yüksekliği (1,10 m) KentOS'un öntanımlı değerleridir; araç ve ADR bunları yönetmelik değeri diye
sunmaz.

## Karar

### 1. Komut girdisi: `cad.stairs.create` v1, geriye uyumlu

Merdivenin (`NewStair`) yeni, isteğe bağlı alanları `height`, `maxRiser`, `cutHeight` ve `fit`'tir; `risers` ve
`riserHeight` artık yazılmayabilir (0 ya da yok). Eski girdiler aynı sonucu verir; saklanan nesne aynıdır (KCAD şeması
değişmez): noktaları, rıht sayısı, rıht yüksekliği ve kesmesi.

- **Yükseklikten:** `height` verilince rıht sayısı verilmişse odur, yoksa ⌈height / maxRiser − 10⁻⁹⌉ (maxRiser yoksa
  `DEFAULT_MAX_RISER` = 0,175 m); rıht yüksekliği height / rıht sayısıdır. 10⁻⁹ payı 2,80 / 0,175 gibi tam bölünen
  yükseklikte kayan noktanın fazladan rıht eklemesini önler.
- **Retler (`invalid_stair`, sırasıyla):** yükseklikle birlikte rıht yüksekliği (`riserHeight`), yüksekliksiz
  `maxRiser`, sıfır ya da 100'den büyük yükseklik ya da en fazla rıht, 400'den çok rıht isteyen yükseklik
  (hesaplanan sayı için; verilen sayı ADR 0343'ün sınırıyla), kesme rıhtı ile kesme yüksekliği birlikte, sıfır ya da
  eksi kesme yüksekliği, rıhtlara uydurulamayan son kol. Sonlu olmayan değerler önce `not_finite`'tir.
- **Uyarı:** verilen rıht sayısıyla rıht verilen `maxRiser`'dan yüksekse `stair_riser_high` (yazılır).
- **Kesme yüksekliği:** kesme, altında bütünüyle kalan son rıhttır: ⌊cutHeight / rıht + 10⁻⁹⌋, en az 1, en çok rıht
  sayısı.
- **Son kol rıhtlara göre (`fit: true`):** son nokta son kolun doğrultusunda taşınır, yürüme çizgisi son rıhtta biter
  (kısa çizgi uzar, uzun çizgi kısalır); önceki noktalar verildiği gibidir. Rıhtlar son kolun başına varmadan
  bitiyorsa (ya da tam başında) ret: “Son dönüşü başa yaklaştırın, rıht ekleyin ya da son kolu tıklanan yerde
  bitirin.” Çekirdek `geom::arch::stair_fit_end`: son kolu bütün rıhtlara ve sahanlıklara yetecek kadar uzatıp son
  rıhtın istasyonunu bulur, uç o uzaklıktadır.

### 2. Sahanlıklar

Ardışık iki dönüşün sahanlıkları arasında bir basamak (`tread`) sığmıyorsa tek sahanlıktır
(`geom::arch::stair_landings`); rıht aralarına düşmez. Kolları bir eni kadar aralıklı U merdivenin iki dönüşü böylece
iki kolu da kesen tek sahanlıktır. Arada bir basamak ya da daha fazla kalıyorsa iki sahanlık ve aradaki kısa kol ADR
0343'teki gibidir. Rampalar değişmez (ARCH-STAIR-02). Bu geometriye sahip eski merdivenlerin planından sahte rıht
kalkar, sonraki rıhtlar sahanlığın sonundan başlar; saklanan alanları yeniden yazılmaz.

### 3. Plan gösterimi

Kesme çizgisinden sonraki rıhtlar çizilmemek yerine kesiklidir (parçanın çizgi tipi `dashed`); ok başlangıçtan kesmeye,
numaralar kesmenin altındaki rıhtlardadır (ADR 0343). DXF ve Patlat parçaların çizgi tipini taşır.

### 4. Araç

Merdiven aracı yükseklikten çalışır: Yükseklik (Y, 3,00 m), En fazla rıht (R, 0,175 m), Rıht sayısı (S, Hesaplanır; 0
yeniden hesaplatır), Basamak (D, 0,28 m), Genişlik (G), Sahanlık (H, Genişlik kadar), Kesme (K, 1,10 m; 0 ya da Yok
kapatır), Numaralar (N) ve Son kol (U: Rıhtlara göre biter / Tıklanan yerde biter). Aynı değerler Öznitelikler'in
“Etkin aracın ayarları”ndadır; satırların altında planlanan tasarım (rıht × yükseklik, basamak, 2R+G, yürüme
çizgisi) ve öntanımlıların yönetmelik değeri olmadığını söyleyen not durur.

Bir tıklama ve imlecin yönü yeter: önizleme komutun planıdır ve rıhtları, sahanlıkları, kesmeyi, oku ve numaraları
çizer; son kol imleç ne kadar ileride olursa olsun son rıhtta biter. Dönüşe tıklanınca L, iki dönüşle U olur;
Enter ya da dördüncü nokta yazar (tek geri alma adımı “Merdiven”). Uydurulamayan durumda etiket kırmızıdır ve nedeni
söyler; hiçbir şey yazılmaz. Rampa aracının davranışı aynıdır; görünür ayarları (Genişlik, Yükseklik farkı, Eğim,
Sahanlık) eklendi.

### 5. Python ve MCP

Aynı komut ve aynı alanlar; `kentos.cad.stairs.create` katalogdan yeniden üretildi. Yeni komut yoktur; manifestte
Merdiven aracı `semantic`, `cad.stairs.create` üzerindendir.

## Sınırlar

- Kat görünümü (aynı merdivenin üst katta kesmenin üstüyle gösterimi), dairesel ve döner merdiven, balans basamak,
  korkuluk ve merdiven denetimi (ARCH-DOC-03'e eklenecek) açıktır.
- Merdivenin yüksekliği katın yüksekliğine bağlanmaz: kat yüksekliği değişince merdiven izlemez, `stair_rise_level`
  uyarısı sürer.
- Kurum ya da yönetmelik profili (rıht/basamak aralıkları) ayrı, sürümlü bir profil olarak sonraki dilimdir.

## Doğrulama

- `scripts/fixtures/arch_cases.py --check`: çekirdeğin merdivenleri 14 durum (birleşen ve ayrı kalan sahanlıklar,
  uydurulmuş uç, kesikli rıht sayısı; kesirlerle). `cargo test -p kentos-geometry-core --test all arch`.
- `scripts/fixtures/arch_command_cases.py --check`: `cad.stairs.create.design.json` 30 yeni durum (yükseklikten sayı,
  verilen sayı, `stair_riser_high`, tam bölünen yükseklik, kesme yüksekliği ve sınırları, L, tek sahanlıklı U, uzayan
  ve ters yöne çizilen kol, eğik kol, uydurmasız kısa yol, kat yüksekliği, plan, on bir ret); uydurulmuş uçlar kesirli
  başvurudan, saklanan noktalar çekirdeğin kayan nokta adımlarıyla bit bit. Eski merdiven, rampa, aks, kolon ve
  düzenleme dosyaları değişmedi. Rust ve Python ortak durum koşucuları geçer.
- `cargo test -p kentos-interaction --test all stair_design`: bir tıklama ve Enter ile 18 rıhtlı, son rıhtta biten,
  kesmesi 6. rıhtta merdiven ve tek geri alma; verilen sayı ve tıklanan uç; uydurulamayan durumda kırmızı neden ve
  yazılmama.
- Gerçek GUI resimleri (`KENTOS_SHOTS_ONLY=merdiven-yukseklik,merdiven-l,merdiven-u,merdiven-u-yazildi,merdiven-sigmaz
  cargo test -p kentos-desktop --bin kentos-cad tools_screens::screens -- --ignored --nocapture`): koyu ve açık tema,
  1440×900 ve 1100×650; `.run/shots/arac-merdiven-*`.
