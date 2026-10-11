# ADR 0219: Geometri üretimi

- **Durum:** kabul edildi (2026-10-11). Kapsamı ben belirledim (sahibin sözü: “Benim seçmeme gerek yok sen sıradan devam et”). İki
  platform (masaüstü ve web); `.kcad` şeması değişmez; ikonlar sorulmadan seçilir; ilke “Performance First”. Madde tek parçada biter.
- **Bağlam belgesi:** TODOS.md `GIS-23`; ADR 0232 §4 (kesin Delaunay: `geom::delaunay`), ADR 0212 (etiket motorunun ulaşılmazlık kutbu),
  ADR 0201 (geometri işlemleri: Çıktı katmanı, girdi değişmez, eğriler 0,1 mm), ADR 0215 §2.3 (nesnenin adı), ADR 0218 (dönüştürme
  araçları, `toleranceFields`), ADR 0149 §5.3 (eğrilerin 0,1 mm'lik temsili).
- **Araştırma:** QGIS'in Voronoi polygons (Buffer region %), Delaunay triangulation (POINTA, POINTB, POINTC), Convex hull, Concave hull,
  Minimum bounding geometry (Envelope, Minimum oriented rectangle, Minimum enclosing circle, Convex hull; alana göre gruplama, genişlik,
  yükseklik, açı, yarıçap alanları), Oriented minimum bounding box ve Pole of inaccessibility'si; ArcGIS'in Create Thiessen Polygons ve
  Minimum Bounding Geometry'si (RECTANGLE_BY_AREA, CONVEX_HULL, CIRCLE, ENVELOPE; NONE, ALL, LIST gruplama; MBG_ alanları); GEOS'un
  VoronoiDiagramBuilder'ı (üçgenlerin çevrel merkezleri), ConcaveHull'u (JTS'ten: kenar uzunluğu oranıyla aşındırma, delikler),
  MinimumAreaRectangle'ı ve MinimumBoundingCircle'ı; Mapbox polylabel.

## Bağlam

Noktalardan ve alanlardan yeni geometri üretmek CBS'nin gündelik işidir: ölçü noktalarının ya da istasyonların etki alanları (Thiessen),
noktaların üçgen ağı, bir kümenin kapladığı alan (dışbükey ve içbükey zarf), yapıların döndürülmüş kutuları (yön ve boy), bir grubu
çevreleyen en küçük daire, etiketler ve numaralar için alanın en iç noktası. Çekirdekte kesin Delaunay (ADR 0232 §4) ve ulaşılmazlık
kutbu (ADR 0212) vardı; araçları yoktu.

## Karar

### 1. Kapsam

İşlemler'in yeni **Geometri üretimi** kategorisinde beş araç, iki platformda:

1. **Voronoi alanları** (`generate.voronoi`).
2. **Delaunay üçgenleri** (`generate.delaunay`).
3. **Sınırlayıcı geometri** (`generate.bounding`): dışbükey zarf, sınır kutusu, döndürülmüş kutu ya da en küçük daire; her nesnenin,
   hepsinin ya da alanın değerine göre grupların.
4. **İçbükey zarf** (`generate.concaveHull`), isteğe bağlı delikleriyle.
5. **Ulaşılmazlık kutbu** (`generate.pole`).

Ortak kurallar ADR 0201 ve 0218'inkiler: sonuç **Çıktı katmanı**na yeni nesnelerdir (yoksa adıyla açılır, geometri araçlarının
görünüşüyle), girdi değişmez ve yalnız okunur (kilitli katmanınki de); hesaplar çekirdekte (`ops::generate`); alanı olmayan sonuç
yazılmaz, söylenir; sonuç kotsuzdur (Delaunay üçgenlerinin köşeleri dışında), kotlu girdi söylenir. **Kapsam dışı:** Voronoi'nin bir
alanla kırpılması (ardından Kırp, ADR 0201 §3), ağırlıklı Voronoi, kısıtlı Delaunay (sınır kenarlarıyla), alfa şekilleri (içbükey zarfın
oranı yeter), alanların içbükey zarfı (GEOS'un ConcaveHullOfPolygons'ı), en küçük genişlikli kutu, en küçük çevreleyen elips, üç boyutlu
zarf.

### 2. Voronoi alanları

- **Girdi** noktalar (çok noktalı nesnenin her noktası bir merkezdir); **Genişletme** (%, 0–1000, varsayılan 10); **Çıktı** (varsayılan
  yeni “Voronoi”).
- **Çerçeve:** noktaların sınır kutusu her yandan uzun kenarının Genişletme / 100 katı büyütülür (float64'te bu sırayla:
  `büyüme = max(genişlik, yükseklik) · (Genişletme / 100)`).
- **Hücre:** noktanın kesin Delaunay üçgenlemesindeki (ADR 0232 §4) çevresindeki üçgenlerin çevrel merkezleri sırayla; sınırdaki
  noktanın hücresi açıktır: iki sınır kenarının dik ortayları kenarlarının üçgeninin merkezinden dışarı ışındır. Hücre çerçeveye kesilir
  (Sutherland–Hodgman, kenarları sabit sırada). **Komşu hücreler köşelerini bit bit paylaşır:** her üçgenin merkezi bir kez hesaplanır;
  aynı çemberdeki komşu üçgenler (kesin `incircle` sıfır) en küçük numaralısınınkini paylaşır, bu yüzden düzgün ızgarada sıfır boylu
  kenar kalmaz; bir ışının uzak noktası hücrelerin ortak uzaklığıyla bir kez; bir kenarın çerçeveyle kesişimi kenarın çerçeve içindeki
  ucundan hesaplanır, iki hücrede de aynı sayıyla. Merkez `orient2d`'nin yakın büyüklüğüyle hesaplanır: ince üçgenin uzak merkezi
  doğrultusunu korur.
- Bir doğrudaki noktaların hücreleri doğru boyunca komşularıyla şeritlerdir. İki ayrı yerden az nokta reddedilir (“Voronoi alanları için
  en az iki ayrı yerde nokta gerekir.”).
- Hücre noktanın özniteliklerini alır; çok noktalı nesnenin noktaları **Parça** 1, 2 … alır. Başka bir noktayla aynı yerdeki nokta hücre
  vermez (girdide ilk gelen alır), söylenir; çerçevede alanı olmayan hücre (sıfır yükseklikte çerçeve) söylenir.
- Halka saat yönünün tersine, en alttaki (eşitlerde en batıdaki) köşesinden.

### 3. Delaunay üçgenleri

- **Girdi** noktalar; **Ad alanı** (isteğe bağlı); **Çıktı** (varsayılan yeni “Üçgenler”).
- Kesin Delaunay üçgenleri (ADR 0232 §4; aynı çemberdeki dört noktada köşegen girdinin sırasıyla). Her üçgen saat yönünün tersine,
  girdide önce gelen köşesinden; üçgenler köşelerinin girdi sıralarıyla sıralı. Aynı yerdeki noktalardan ilki köşe olur, söylenir.
- Öznitelikler **Nokta 1**, **Nokta 2**, **Nokta 3**: köşelerin adları (ADR 0215 §2.3: Ad alanındaki değer, boşsa etiket, o da yoksa
  girdideki sırası; çok noktalı nesnenin k'ıncı noktası `ad/k`). Noktaların kotları üçgenin köşe kotlarıdır (TIN).
- Üçten az ayrı nokta ya da hepsi bir doğruda reddedilir (“Üçgenleme için doğru üzerinde olmayan en az üç nokta gerekir.”).

### 4. Sınırlayıcı geometri

- **Girdi** her türden nesne (alan, daire, elips, eğri, çizgi, çoklu çizgi, yay, nokta); **Tür** Dışbükey zarf, Sınır kutusu, Döndürülmüş
  kutu ya da Daire (varsayılan Dışbükey zarf); **Gruplama** Her nesne, Hepsi ya da Alana göre (**Alan**; boş değer de bir gruptur, gruplar
  ilk nesnelerinin sırasıyla; Alan boşsa Hepsi); **Çıktı** (varsayılan yeni “Sınırlayıcı”).
- **Nesnenin noktaları:** köşeleri ve noktaları yazıldığı gibi; yaylarının noktaları sapması 0,1 mm'yi geçmeyen adımla (yay başına en çok
  4 096 parça) ve eksenlerdeki uç noktaları (kutu yaylarla kesindir); elipsin ve eğrinin 0,1 mm'lik çizgisi; delikler de.
- **Dışbükey zarf:** Andrew'un monoton zinciri (y, x) sırasıyla, dönüşler kesin (`orient2d`), kenar üstündeki noktalar atılır; halka saat
  yönünün tersine en alttaki noktadan. **Sınır kutusu** eksenlere paraleldir. **Döndürülmüş kutu** en küçük alanlı dikdörtgendir: bir
  kenarı zarfın bir kenarında (Freeman ve Shapira), uç noktalar zarf boyunca dönen kumpasla izlenir; eşit alanlarda zarfta önce gelen kenar.
  **Daire** en küçük çevreleyen dairedir: Welzl'in özyinelemesiz biçimi, noktalar sabit tohumlu bir sıralamayla karıştırılmış (her
  platformda aynı daire), ilk noktaya göre hesaplanır; nesne daire olarak yazılır.
- **Öznitelikler:** Her nesne'de nesnenin öznitelikleri, Hepsi'nde **Nesne sayısı**, Alana göre'de alanın değeri ve **Nesne sayısı**;
  ölçüler: zarfta **Zarf alanı**, kutuda **Kutu genişliği** (doğu yönü) ve **Kutu yüksekliği**, döndürülmüş kutuda **Kutu uzunluğu**,
  **Kutu genişliği** ve uzun kenarın **Kutu semti** (yarım dönüşten küçük, projenin açı biriminde 4 basamak), dairede **Daire yarıçapı**.
  Uzunluklar projenin uzunluk, alanlar alan basamağıyla m²'de (ADR 0215'in yazımı). Adlar nesnenin kendi özniteliklerine (Alan, Genişlik
  gibi) çarpmasın diye ön eklidir (ArcGIS'in MBG_'si gibi).
- Alanı olmayan sonuç (bir nokta, bir doğru, sıfır yarıçap) yazılmaz, söylenir. Yaylı ya da eğrili nesneler söylenir.

### 5. İçbükey zarf

- **Girdi** Sınırlayıcı geometri'ninki (noktaları §4'teki gibi); **Oran** 0–1 (varsayılan 0,3); **Delikler** (kapalı); **Gruplama**
  (varsayılan Hepsi); **Çıktı** (varsayılan yeni “İçbükey zarf”).
- JTS'in ve GEOS'un kuralı, kesin Delaunay üçgenleri üstünde: kenarların en kısası k, en uzunu u ise en uzun izinli kenar oran · (u − k) +
  k (0'da 0, 1'de 2u). Tek sınır kenarlı üçgenler kuyruktadır, boyları sınır kenarlarının uzunluğu; en uzunu önce (eşitlerde alanı büyük,
  sonra numarası küçük), boyu izinliden kısaysa durulur; üçüncü köşesi sınırdaysa (alan ikiye ayrılırdı) atlanır; giden üçgenin yeni tek
  sınır kenarlı komşuları kuyruğa girer. Uzunluklar GEOS'unki gibi √(dx² + dy²).
- **Delikler:** sınıra değmeyen (hiçbir köşesi sınırda olmayan), en uzun kenarı izinliden kısa olmayan iç üçgenler **büyükten küçüğe**
  (JTS'in sırası; GEOS'un `ConcaveHull.cpp`'si adayları yorumuna ve JTS'e karşın küçükten büyüğe sıralar, ayrı deliklerde sonuç
  değişebilir) delik açar;
  delik aynı kuralla, sınıra ve başka deliğe bir köşeyle değmeden büyür.
- Sonuç tek alan; halkalar saat yönünün tersine dış, deliklerin halkaları saat yönünde, her biri en alttaki köşesinden; delikler en
  alttaki köşelerinin sırasıyla. **Zarf alanı** yazılır; üçgeni olmayan grup (üçten az ayrı nokta, bir doğru) yazılmaz, söylenir.

### 6. Ulaşılmazlık kutbu

- **Girdi** alanlar (alan, daire, bütün elips, kapalı eğri); **Hassasiyet** (m; varsayılan 0,1, en az 0,001); **Çıktı** (varsayılan yeni
  “Kutuplar”).
- Alanın kenarlarından (delikleri de) en uzak iç noktası, Mapbox'ın polylabel'ı (etiket motorunun çekirdeği, ADR 0212) hassasiyet içinde:
  kare hücreler, hücrenin olabileceği en büyük uzaklık (uzaklığı artı yarım köşegeni) büyükten küçüğe, eşitlerde önce yapılan; en iyiyi
  geçemeyecek hücre yapılınca bırakılır. Çok parçalı alanda kutbu en uzak parçanınki (eşitlerde önce gelen); dairede merkezi, uzaklığı
  yarıçapı. Yaylar 0,1 mm içinde noktalarla.
- Nokta alanın özniteliklerini ve **Uzaklık**'ı (kenara uzaklık) alır. Alanı olmayan nesne yazılmaz, söylenir.

### 7. Arayüz

- Pencereler tanımdan (ADR 0084). Araç kutusunda Geometri üretimi kategorisi (Dönüştürme'den sonra). CBS'de Analiz sekmesinin
  Geometri panelinin ▾'inde beş araç (sekme 1100 pikselde dolu; panel aynı adla birleşir, `app/ribbon.ts`'in `GENERATE_UNDER`'ı);
  envanterin `ribbonByMode`'u masaüstünün şerididir. Sayı alanlarının birimlerine `%` eklendi (Genişletme).

### 8. Performans

Bütçeler (release, bu makine; denetim, araç ve tek adım): 100 000 noktanın Voronoi'si masaüstünde ≤ 500 ms, web'de ≤ 1500 ms; 100 000
noktanın Delaunay üçgenleri (200 000 üçgen) ≤ 800 / 1500 ms; 100 000 noktanın içbükey zarfı ≤ 500 / 1500 ms; 100 000 parselin
döndürülmüş kutusu ≤ 400 / 1200 ms; 100 000 parselin kutbu (0,1 m) ≤ 1000 / 2500 ms. Web'de çekirdeğe yalnız geometri gider
(`entityGeometry`: öznitelikler okunup atılmak için geçmez); Her nesne'de çekirdek nesne başına grup listesi kurmaz.

## Uygulama

- **Çekirdek:** `ops::generate`: `voronoi.rs` (`frame_of`, `distinct`, `centre`, cocircular grupların merkezi `centres`, çerçeveye kesme
  `clip`, `normalised`, `voronoi` ve `Cell`, `delaunay_triangles`), `bounding.rs` (`convex_hull`, `axis_box`, `min_area_box`,
  `min_circle`, `bound`, `Kind`, `Bound`), `concave.rs` (`target_length`, `concave_hull`), `pole.rs` (`pole`, etiket motorunun
  `labels::geom::pole`'ü), `points.rs` (`outline_points`, `is_curved`, `CURVE_TOL`), `mod.rs` (`bounding_run`, `concave_run`,
  `bound_shape`, `hull_area`); web için toplu çağrılar `calls.rs` (`generateVoronoi`, `generateDelaunay`, `generateBounding`,
  `generateConcave`, `generatePoles`). 11 birim testi kaba kuvvetle karşılaştırır (hücreler her öbür noktanın dik ortayıyla, zarflar her
  kenarın yanıyla, kutular her zarf kenarıyla, daireler her ikili ve üçlüyle) ve komşu hücrelerin köşelerini bit bit paylaştığını sınar;
  `bounding_parts_timing` kutunun çekirdekteki parçalarını ölçer.
- **Web:** `model/ops/generate.ts`, `processing/builtin/generate/` (`shared.ts`: kapsamlar, noktalar, gruplama ve notlar;
  `triangulation.ts`, `bounding.ts`, `pole.ts`), kategori `generation` (`processing/categories.ts`), birim `%` (`processing/types.ts`).
- **Masaüstü:** `kentos-processing`'in `builtin/generate/`'u (`mod.rs`, `triangulation.rs`, `bounding.rs`, `pole.rs`), kategori
  (`categories.rs`), katalog (`apps/desktop/src/catalog.rs`, `ported.json`).
- **İkonlar** (sorulmadan seçildi): kategori `generation` (noktaların içinde kaldığı alan ve parıltı), Voronoi alanları `voronoi` (çerçevede
  hücreler ve merkezleri), Delaunay üçgenleri `delaunay` (noktaları birleştiren üçgenler), Sınırlayıcı geometri `boundingGeometry`
  (noktaları saran döndürülmüş kutu), İçbükey zarf `concaveHull` (içe oyulmuş zarf ve noktaları), Ulaşılmazlık kutbu
  `poleOfInaccessibility` (L biçimli alan, en geniş dairesi ve merkezi).

## Doğrulama

- **Bağımsız başvuru:** `scripts/fixtures/generate_cases.py` (KentOS kodu olmadan ADR'den): Voronoi hücreleri kesirlerle tam (çerçeve her
  öbür noktanın dik ortayıyla kesilir, üçgenleme yok) ve GEOS'la (shapely 2.1, GEOS 3.13: `voronoi_polygons`) çapraz; Delaunay üçgenleri
  2³⁴ katı tam sayılarla her üçlünün çemberi boş mu diye ve GEOS'un `delaunay_triangles`'ıyla; zarflar tam sayılarla, döndürülmüş kutular
  her zarf kenarında 50 basamakta (GEOS'un `oriented_envelope` alanıyla), daireler kesirlerle Welzl'le (GEOS'un
  `minimum_bounding_radius`'uyla); içbükey zarf ADR'nin kuralıyla kesin üçgenleme üstünde, GEOS'un `concave_hull`'uyla (delikli durumda
  da: tek deliğin bölgesinde iki sıra aynı sonucu verir); kutup polylabel'ın tanımından, GEOS'un `maximum_inscribed_circle`'ıyla
  hassasiyet içinde. Noktaların genel konumu (üçü bir doğruda, dördü bir çemberde değil) kesin olarak denetlenir. 20 durum: Voronoi
  (çok noktalı nesne, aynı yer, kotlar; Genişletme 0; bir doğrudaki noktalar; ret), Delaunay (adlı, adsız, ret), sınırlayıcı (her
  parselin zarfı, bütün türlerin kutusu, adalara göre döndürülmüş kutular, daireler, kilitli katman), içbükey zarf (C, O'nun deliği, O
  deliksiz, türlere göre, bir doğru), kutup (parseller, ince L 1 mm, daire); `fixtures/processing/v1/generate.json` ve `generate.kcad`.
  Daire yarıçapları ve kotlar dosyanın `toleranceFields`'ıyla 10 nm içinde. İki platformun oynatıcıları (`cases.test.ts` sayfada ve
  işçide, `cases.rs`'in `the_generation_cases_do_what_they_say`'i çizimde ve okuma kopyasında) 20'sini ilk koşuda geçti.
- **Pencere:** masaüstünde `processing::generate_tests` (20 durum pencereden).
- **Resimler:** masaüstünde `cargo test -p kentos-desktop processing::generate_tests::screens -- --ignored --nocapture
  --test-threads=1` (`.run/shots/islem-uret-*`), web'de `(cd apps/web && node scripts/e2e/shots.mjs generate)`; yerleşim
  `(cd apps/web && node scripts/e2e/layout.mjs --only tab-analysis,processing-generate-voronoi,processing-generate-bounding,processing-generate-concave)`.
- **Süreler** (11 Ekim, bu makine, release; p50 / p95, denetim, araç ve tek adım):

  | İş | Masaüstü | Web | Bütçe |
  |---|---|---|---|
  | Voronoi alanları, 100 000 nokta | 279,8 / 281,0 ms | 762,2 / 810,4 ms | 500 / 1500 ms |
  | Delaunay üçgenleri, 100 000 nokta, 200 000 üçgen | 385,8 / 404,1 ms | 703,7 / 776,9 ms | 800 / 1500 ms |
  | İçbükey zarf, 100 000 nokta, oran 0,3 | 125,9 / 127,7 ms | 197,9 / 199,3 ms | 500 / 1500 ms |
  | Sınırlayıcı geometri, 100 000 parsel, döndürülmüş kutu | 299,3 / 312,3 ms | 803,6 / 863,6 ms | 400 / 1200 ms |
  | Ulaşılmazlık kutbu, 100 000 parsel, 0,1 m | 425,2 / 478,8 ms | 749,8 / 805,1 ms | 1000 / 2500 ms |

  Masaüstünde parçaları (hazırlık, araç, uygulama): Voronoi 30,5 / 186,6 / 57,3 ms, Delaunay 32,6 / 235,5 / 123,5 ms, içbükey zarf
  22,9 / 101,2 / 0,1 ms, kutu 31,5 / 210,3 / 56,4 ms, kutup 30,6 / 332,6 / 59,4 ms. Kutunun çekirdekteki payı 55 ms (noktalar 9, zarflar
  12, kutular 19, şekiller 15 ms); kalanı nesnelerin öznitelikleri ve yazılması. İlk ölçümde kutu 330,5 ms'ydi: öznitelikler iki kez
  kopyalanıyor, Her nesne'de nesne başına grup listesi kuruluyordu. Web'de çekirdeğe öznitelikleriyle giden nesneler kutuyu 938,8,
  kutbu 901,1, içbükey zarfı 291,8 ms tutuyordu; yalnız geometri gidince 803,6, 749,8, 197,9 ms. Komutlar: `cargo test --release -p
  kentos-processing --test cases generate_timing -- --ignored --nocapture`; web'de `KENTOS_WASM_PROFILE=wasm pnpm -s rust:wasm;
  GENERATE_BENCH=1 pnpm -C apps/web exec vitest run scripts/perf/generate.test.ts --disable-console-intercept; rm
  apps/web/src/wasm/pkg/.stamp`.
