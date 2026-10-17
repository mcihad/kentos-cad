# ADR 0328: Taramanın kaynak sınırı ve yüzün bağı

- **Durum:** kabul edildi (2026-10-17). [plan/cad.md](../../plan/cad.md) §12 `CAD-HATCH-01`, `02` (sahibin sözü: “CAD-HATCH
  kısmına geç”; 2–3 maddelik gruplar). Kapsamı ben belirledim. `CAD-HATCH-05` (sınırı yeniden oluştur, yeniden seç, ada ekle ve
  çıkar) kendi anlamsal komutuyla sonraki grubundadır.
- **Bağlam belgesi:** ADR 0062 ve 0186 (tarama, ilişkili tarama), 0254 (`cad.entities.hatch`), 0264 (ileri CAD nesnelerinin
  kaynak modeli), 0249 ve 0263 (alt nesne başvurusu), 0259 (NURBS), 0149 §5.3 (eğrinin 0,1 mm'lik kirişleri), 0265 (yeni şemanın
  altı adımı).
- **Web:** ertelendi (ADR 0228); gerekenler TODOS.md §0.2'de.

## Bağlam

Tarama yalnız bir nokta halkası (`ring`) ve delikler tutuyordu. Kapalı alanın yayları, elips ve eğri adalar taramaya
kirişleriyle geçiyordu (alanda yay `bulges` ile kesin iken); DXF'in HATCH'indeki yay, elips yayı ve spline kenarları
72 parça/tur kesilip “parçalı alındı” notuyla geliyordu; yazarken tarama yine kiriş yollu çoklu çizgi olarak gidiyordu. Çizilen
halka ile kaynak sınır aynı alandı: kaynağın kesinliği bir kez kaybolunca geri gelmiyordu.

İlişkili tarama (ADR 0186 §6) yalnız “kapalı nesne” yolunda vardı. Çizgilerin kapattığı yüze (AutoCAD'in “pick point”i:
duvarlar ayrı çizgiler) atılan tarama hiçbir şeyi izlemiyordu; duvar taşınınca tarama yerinde kalıyordu.

## Karar

### 1. Kaynak sınır (`boundary`) ve çizilen halka (`CAD-HATCH-01`)

Taramanın yeni `boundary`'si kaynağın sınırıdır: halkalar (ilki dış, ötekiler delikler), her halka `pts`, isteğe bağlı
`bulges` ve `curves`. Kenar `i` `pts[i]`'den sonrakine gider: düz, `bulges[i]`'lik dairesel yay (alanın bükümü gibi) ya da
`curves[i]`: elips yayı (`c`, `major`, `ratio`, `t0`'dan `t1`'e; `t1 > t0` saat yönünün tersine, küçükse saat yönünde, tam elips
`±2π`) ya da baştan sona NURBS (`degree`, `ctrl`, `knots`, `weights`). Tek noktalı halka kapalı bir eğridir. Biçim kuralı
sözleşmede (`kentos_contracts::boundary_problem`), eğrinin uçlarının noktalara 1 µm içinde oturması çekirdekte
(`geom::hatch_boundary::problem`).

`ring` ve `holes` kalır; ama artık **sınırdan çizilir** (`hatch_boundary::rings`): yaylar alanın halkası gibi, elips ve eğri
0,1 mm'lik kirişlerle (`curve_outline`). Çizim, seçme, kenet, ölçü ve PDF çizilen halkayı okumaya devam eder. Belge kuralı
(`kentos_domain::hatch_boundaries`, adımın kaydından önce, izleyenlerin ilki):

- adım sınırı yeni yazdı ya da değiştirdiyse halka ondan çizilir; uçları oturmayan sınır düşer, halka yazıldığı gibi kalır;
- adım halkayı kendisi değiştirdi, sınır aynı kaldıysa (köşe tablosu) sınır düşer: tarama artık yalnız çizilen halkadır.

Yalnız düz kenarlı sınır halkanın kendisidir; böyle taramaya sınır yazılmaz (`hatch_boundary::curved`). Değişikliğin dosyaları
ve testleri bu yüzden düz taramalarda aynıdır.

**Sınır nereden gelir:**

- **Tarama ve Çoklu tara** bölgeyi alan işlemleriyle keser (yaylar `bulges` ile kesin). `hatch_boundary::of_area` bölgenin
  halkalarında, kesildiği kapalı elipsin, kapalı eğrinin (Catmull–Rom, B-spline olarak kesin) ya da NURBS'ün çekirdekçe çıkarılan
  kiriş köşelerinin (bit bit aynı) art arda gelişini bulur ve o eğriyi geri koyar: bütünse tek noktalı halka, kesilmişse kesimin
  bıraktığı iki köşe arasındaki eğri parçası (NURBS'te `sub`, dikiş yeri bölünür). Kesimin yeni köşesine giden kiriş parçası
  0,1 mm içindedir.
- **İlişkili tarama** nesneleri değişince aynı yoldan yeniden kesilir (`hatch_ties`); eliptik ada kesin kalır.
- **Halka** (Halka aracı) iki çemberini iki yarım yay olarak yazar.
- **DXF okuma:** HATCH'in çoklu çizgi yolu nokta ve bükümleri, kenar yolu çizgiler ve yaylar (büküm; saat yönündeki yayın
  açıları eksiyle), elips yayları ve spline'lar (eğri) olarak, OCS dönüşümüyle; bir kenar öncekinin bittiği yerde
  (1 µm + boyun milyarda biri) başlamıyorsa o tarama eskisi gibi parçalı alınır ve söylenir.
- **DXF yazma:** sınırı olan tarama yayları büküm olarak çoklu çizgi yolu, eğrileri kenar yolu (elips yayı 3, spline 4;
  AC1021: uydurma noktası yok) olarak yazar. Okuma–yazma turu yayları, saat yönündeki elipsi ve ağırlıklı spline'ı nanometre
  içinde geri verir (`a_hatch_boundary_goes_out_and_comes_back_exact`).

**Dönüşümler sınırı taşır:** benzerlik (Taşı, Döndür, Ölçekle, Aynala) yayları korur, aynada bükümün işareti döner; benzerlik
olmayan afin (Vektör oturtma, blokun Y ölçeği) her yayı elips yayına çevirir, elips görüntüsünün elipsi olur, NURBS'ün denetim
noktaları taşınır (`hatch_boundary::mapped`); projektif dönüşüm ve kauçuk levha sınırı düşürür. Esnet penceredeki noktaları
taşır; iki ucu birden taşınmayan eğri sınırı düşürür. Birimi değiştir sınırı ölçekler. Tutamaçlar sınırın dış köşeleridir
(yayların kirişleri değil); bir köşe taşınınca bükümler kalır, ona dokunan eğri kirişlerine döner (`moved_point`).

### 2. Yüzün bağı (`bounds`, `CAD-HATCH-02`)

İlişki iki biçimdedir: kapalı nesne (`outer`) ya da **yüzü kapatan çizgi işi** (`bounds`); tam biri. Tarama “Sınır: çizgiler”
yolunda İlişkili açıkken (varsayılan) çizimin o yüzünü kapatan nesneleri bağlar: yüzün dış halkasındaki her kenarın ortası hangi
nesnenin kenarı üstündeyse (1 µm) o nesne `bounds`'a, deliklerininkiler `islands`'a (`ops::hatch_region::bounding`), boş
bırakılan yazılar `cutouts`'a; sıra çizimin sırasıdır. Nesneler kalıcı kimlikleriyle anılır: bir duvarın parçalanması değil,
kendisi izlenir.

Belge bağı aynı adımda izler (`hatch_ties`): bağlı nesnelerden biri değişince çizgi işi yeniden yüzlere ayrılır
(`face_region`) ve tohum noktasının yüzü taranır. Bütün çizgiler birlikte taşındıysa tohum yüzün dışında kalır: o zaman tohum,
taşınan ilk bağlı nesnenin ilk köşesinin kaydığı kadar kaydırılıp yeniden aranır ve bulunursa yeni yeriyle saklanır. Yüz
açılırsa (nokta artık kapalı bir yüzde değil) tarama yerinde kalır, bağı kopar. Tarama aracında İlişkili (İ) iki yolda da
sunulur; Öznitelikler “İlişkili: Evet (n nesne)” der ve İlişkiyi kopar'ı sunar.

### 3. KCAD şema 69

Taramanın `boundary`'si (JSON'u) ve ilişkinin `bounds`'u; `bounds` varken `outer` yazılmaz. Sütunlarda taramanın 6. seçeneği
(tek JSON nesnesi), yüzün ilişkisinde boş `outer` kimliği. `FORMATS_VERSION` 78. Bağımsız Python okuyucusu (`tools/kcad/kcad.py`)
ve yazıcısı (`kcad_v2_reference.py`), örnek `hatch-boundaries.kcad`, bozuk dosyalar (`hatch-boundary-in-schema-68`, boş sınır,
eğrisiz tek nokta, eğri sayısı, düğümler, oran, `outer` ile `bounds` birlikte) ve `schema-version-70`. Sunucu sınırın biçimini
denetler; sınır nesnenin kaynağında (`cad_definition`) taşınır, PostGIS geometrisi çizilen halkadır.

### 4. Komutlar

Yeni komut yok. `cad.entities.hatch` İlişkili'yi çizgi yolunda da uygular (`associative` yokken `true`), yazdığı taramaya sınırı
koyar; `cad.entities.create` ve `.edit`'in tarama geometrisi `boundary` alır (`ring` o zaman boş olabilir, sınırdan çizilir;
geçersiz sınır `invalid_hatch`).

## Doğrulama

- Çekirdek: `geom::hatch_boundary` testleri (tam elips ve yarısı, yaylar, afin ve ayna, tutamaç, kapalı eğrinin NURBS'ü).
- Belge: `crates/native/domain/tests/all/hatch_boundaries.rs` (sınır halkayı çizer, oturmayan düşer, elle halka sınırı
  düşürür, yüz çizgisini izler, bütünüyle taşınan yüz tohumunu götürür, açılan yüz bağı koparır, eliptik ada kesin kalır).
- Ortak durumlar: `cad.entities.hatch.json` yüzün bağıyla (bağımsız başvuru `region_command_cases.py`: kenar ortası çizgi işinin
  üstünde, kesirlerle), 35 durum.
- DXF: `a_hatch_boundary_goes_out_and_comes_back_exact`; var olan DXF testleri değişmeden.
- KCAD: `kcad_v2_reference.py --check` (824 dosya), kodek testleri.
- İz: `hatches.json` (çizgi yolunda İ seçeneği).
- Resimler: `KENTOS_SHOTS_ONLY=tarama-yuz-bagi,tarama-kesin-sinir … tools_screens`.

## Sonuçlar

- Düz kenarlı taramalar eskisi gibidir: dosyaları ve çıktıları bayt bayt aynı.
- Kapalı eğrinin kirişleri bit bit aynı çıkmazsa (başka bir kesim yolu) eğri geri bulunmaz, kirişler kalır: kayıp 0,1 mm'dir,
  sessiz değil, ölçülebilir.
- Blokların içindeki çizgi işi yüz bağının nesnesi değildir (yerleştirmenin kendi kenarı yok); blok içi parçanın yolu
  (ADR 0263) gerekirse sonraki işe kalır.
- Web: taramanın sınırını okuyup yazmalı, Tarama'nın çizgi yolunda İlişkili'yi sunmalı (TODOS.md §0.2).
