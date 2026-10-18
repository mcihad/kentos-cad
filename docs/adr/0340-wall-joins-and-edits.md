# ADR 0340: Duvar birleşimleri ve düzenlemeleri

- **Durum:** kabul edildi (2026-10-18). [plan/cad.md](../../plan/cad.md) §14 `CAD-ARCH-06`, `07`, `08`, `10`, `11` (sahibin sözü:
  “devam”; yedi grubun ikincisi). Kapsamı ben belirledim.
- **Bağlam belgesi:** ADR 0339 (duvarın kaynak modeli), 0249 ve 0263 (alt nesne yerleri), 0328 (taramanın yüz bağı), 0253 (komut
  protokolü), 0265 (yeni şemanın altı adımı), 0228 (web'e ara), CLAUDE.md §9.2 (araç = komut).
- **Web:** ertelendi (ADR 0228); gerekenler TODOS.md §0.2'de.

## Bağlam

ADR 0339'un duvarı tek başına doğru çizilir, ama iki duvar buluştuğunda her biri kendi ucunu ve yüzlerini çizer: köşede uç çizgileri
üst üste biner, T'de bağlanan duvarın yüzleri konağın içine girer. Mimari çizimde birleşim duvarın kendisinin parçasıdır (AutoCAD
Architecture'ın duvar temizliği, Revit'in katman öncelikleri). Birleşim saklanmamalı, duvarlardan türemelidir: duvar oynayınca
birleşim kendiliğinden izlemeli, kullanıcı yalnız istisnayı söylemelidir.

## Karar

### 1. Model (`.kcad` şema 76, `FORMATS_VERSION` 83)

- **Bileşenin önceliği** `WallComponent.priority` (1–9; küçük olan güçlü; yoksa 3). KentOS'un stillerinde çekirdek (tuğla, gazbeton,
  betonarme) 1, ısı yalıtımı 3, bölmenin yalıtımı 2, sıva ve alçı levha 5.
- **Uçların birleşimi** `WallFields.joins` (`start`, `end`: `auto`, `none`, `miter`, `butt`, `through`; ikisi de `auto` iken yazılmaz):
  Otomatik, Yok (bu uç birleşmez, uç çizgisi çizilir), Gönye, Alın (bu duvar diğerinin yüzünde durur), Geçen (diğeri bunun yüzünde durur).
- **Temizlik** `WallFields.cleanup` (yalnız `false` iken yazılır): kapalıyken bileşenler öncelikle geçmez, duvarlar yalnız yüzleriyle
  buluşur.
- Eski okuyucular bu alanları taşıyan dosyayı reddeder; şema 75'te `priority` bilinmeyen alandır (`unknown_field`).

### 2. Birleşimler (`CAD-ARCH-06`; çekirdek `geom::wall_join`)

Birleşim saklanmaz: duvarların yollarından, uç birleşimlerinden ve önceliklerinden çözülür (`solve`). Kurallar:

- **Uçlar buluşursa** (yolların uçları 1 mm içinde): açıyla iki uç **köşe**: iki duvarın bütün çizgileri, birincinin en soldaki
  çizgisinin (içine giden yöne göre) ikincinin en sağdakiyle ve birincinin en sağdakinin ikincinin en soldakiyle buluştuğu iki noktadan
  geçen gönye doğrusuna kesilir ya da uzatılır; köşede uç çizgisi yoktur. Bir doğru üzerinde (1° içinde ters) iki uç **düz devam**:
  ikisi de noktadan dik kesilir; yüzleri aynı uzaklıktaysa uç çizgisi kalkar. 3° içinde aynı yöne bakan iki uç **üst üste**dir; 15°'den
  dar, ikisi de Otomatik köşe **dar köşe**dir: ikisi de birleştirilmez, söylenir. Bir uç Alın (ya da diğeri Geçen) ise **alın köşe**:
  geçen duvar alın duvarının uzak yüzüne dek uzar (uç çizgisi köşenin dışını kapatır), alın duvarı ona T ile bağlanır. Üç ya da daha çok
  uç: ters iki uç düz devam eder, diğerleri ikisine T ile bağlanır; böyle bir çift yoksa söylenir.
- **Uç başka duvarın içindeyse** (yoluna dik, iki yüzünün arasında, açık konağın ucunda değil): **T**. Bağlanan duvarın her bileşeni,
  yakın yüzden başlayarak konağın bileşenlerini öncelikleri kendisininkinden büyük (zayıf) oldukça geçer, ilk eşit ya da güçlüde durur;
  bağlanan duvarın her çizgisi iki yanındaki bileşenlerin derin olanınca (yüz kendi bileşeninin) konağın o derinlikteki çizgisine
  kesilir. Konağın yakın yüzü bağlanan duvarın yüzleri arasında kesintiye uğrar; iç çizgileri, altından geçen ya da eşit öncelikle
  orada duran bileşenler boyunca; bandı, içinden geçen bileşenler boyunca kesilir. Bağlanan duvar konağa 3° içinde paralelse üst üstedir.
- **Yollar uçlardan uzakta kesişirse**: **X**; güçlü olan (en güçlü bileşeni, eşitse genişi, eşitse önce çizileni) konaktır; diğeri
  iki yandan iki T'dir: çizgileri ve bantları konağın içinde, derinlikleri arasında kesintiye uğrar.
- `cleanup` iki duvardan birinde kapalıysa bileşen geçmez; yakın yüz yine kesintiye uğrar. `none` olan uç hiç birleşmez.
- **Birleşmeyenler** kodla söylenir: `wall_join_overlap`, `wall_join_crowded`, `wall_join_acute`, `wall_join_unresolved` (bir çizgi
  kesilemedi: duvar birleşimden kısa ya da konak birleşimden önce bitiyor); her birinin onarımı bir uç birleşimidir (`Fix`: üst üste ve
  kesilemeyende o uç Yok, dar köşede Alın, kalabalıkta en kısa duvarın ucu Yok).

Çözüm çizgileri yaylarıyla keser (`ops::path::sub_path`); yay ucunun uzatılması teğeti boyunca düz kenardır. Duvarın çizimi
(`geom::wall::Drawn`: çizgilerin parçaları, serbest uçların çizgileri, bileşen bantlarının ve dolgunun parçaları) görünüşten bağımsızdır;
`pieces_of` onu bir ayrıntıda parçalara çevirir.

### 3. Depo

Depo duvar ekleyip silerken eski ve yeni kutusunu işaretler; bir sonraki sorgudan önce (`maybe_rebuild`) o kutulara değen duvarları,
onlara değenlerle birlikte, belgenin sırasıyla çözer ve yalnız onları yeniden açar (`store/wall_joins.rs`); 64'ten çok değişiklikte
bütün duvarlar tek geçişte çözülür. Her duvarın çizimi saklanır: görünüş değişince yeniden çözülmez, yalnız parçalanır. Birleşmeyenler
`Store::wall_issues`'tadır.

### 4. Komutlar (`CAD-ARCH-07`, `10`)

- `cad.walls.edit`'in yeni işlemleri: `split` (Böl: noktaya en yakın yerde; nokta yüzlerin arasında olmalı, uçta olmaz; kapalı duvar
  orada açılır), `join` (Birleştir: uçları 1 mm içinde buluşan, alanları aynı duvarlar; ters yöndeki gövdesi yerinde çevrilerek; uçları
  buluşursa kapanır), `extend` (Uzat: uç son kenarının doğrultusunda diğer duvarın yoluna), `trim` (Buda: diğer duvarın yolunu kestiği
  yerde, `keep` noktasının parçası ya da uzun olanı), `flip` (Çevir: yol ters döner, gövde yolun diğer yanına aynalanır, uçların
  birleşimi yer değiştirir), `replaceStyle` (Stili değiştir: kimlik verilmezse stilin bütün duvarları), `joins` ve `resetJoins`.
  Yeni retler `point_off_wall`, `split_at_end`, `walls_not_touching`, `walls_differ`, `wall_closed`, `wall_misses`, `style_not_used`;
  çıktıda `added` (Böl'ün yeni parçaları) ve `removed` (Birleştir'in giden kimlikleri).
- `cad.walls.create` ve `cad.walls.edit` yazdıklarının çevresiyle birleşmeyenleri uyarı olarak söyler (`walls::join_warnings`).
- Salt okunur **`cad.walls.joins`**: verilen duvarların (verilmezse hepsinin) birleşimleri (tür, yer, duvarlar uçlarıyla) ve
  birleşmeyenleri (kod, söz, onarım, yer, duvarlar ve `fix`: olduğu gibi `cad.walls.edit`'e verilecek girdi); x, y ve türe göre sıralı.
- Python `kentos.cad.walls.joins`, MCP aracı kataloğdan; manifestte bütün araçlar `semantic`, Birleşimleri denetle `query`.

### 5. İzleyenler ve kayıp (`CAD-ARCH-08`, `11`)

- **Duvarın yerleri:** alt nesne yerlerinden `Vertex { part, ring: 0, index }` ve `Mid` duvarda “`part`. çizgi (0 sol yüz, son sağ yüz,
  aralarında bileşen sınırları), yolun `index`. köşesi” demektir (`ops::subref`); sözleşme ve dosya değişmez. Kalınlık, hiza ya da stil
  değişince yapı aynı kaldıkça yer aynıdır: duvarın yüzüne bağlı ölçü aynı adımda izler. Açık duvarın birleşen ucu çizgisinin kendi
  ucudur (birleşimin köşesi değil): birleşmiş köşeye kenetlenen ölçü bağlanmaz, serbest kalır.
- **Tarama:** odanın içine çizgilerle tıklanan ilişkili tarama (ADR 0328'in yüz bağı) duvarın iç yüzünü izler; kalınlık değişimiyle aynı
  adımda yeniden kesilir.
- Açıklık ve mahal sınırı 3. ve 4. grupla gelecek; onların izlemesi kendi ADR'lerinde.
- **Patlat:** duvar, çizimin ayrıntısında, çevresiyle birleşmiş çizgilerine ve taramalarına açılır; `semantics_lost` uyarısı stilinin,
  bileşenlerinin, katının, yüksekliğinin ve birleşimlerinin artık taşınmadığını söyler. **DXF:** duvarlar birleşimleriyle yazılır;
  rapor aynı kaybı söyler.

### 6. Masaüstü

Mimari sekmesinde **Duvar düzenle** paneli: Duvar birleşimi (uca tıklanır; Otomatik O, Yok Y, Gönye G, Alın A, Geçen E, Temizlik T,
Sıfırla R), Duvarı böl, Duvarları birleştir, Duvarı uzat, Duvarı buda, Duvarı çevir, Stili değiştir (Stil S, Tümü T) ve Birleşimleri
denetle (Göster, Öneriyi uygula). Araçlar yalnız tıklananı söyler, hesap ve yazma `cad.walls.edit`'tir. Çizimde birleşmeyen her yerde
uyarı üçgeni. Öznitelikler'de Başlangıç birleşimi, Son birleşimi, Temizlik; Duvar stilleri'nde bileşenin Öncelik sütunu. İkonlar benim
seçimim: tıklanan ucu gösteren L, ayrık ve birleşen duvar parçaları, sınıra uzayan ve kesilen duvar, aynalanan duvar, uyarılı köşe; uç
birleşimleri için gönyeli, alın ve geçen köşeler.

## Sonuçlar

- Duvarlar kendiliğinden birleşir; kullanıcı yalnız istisnayı (uç birleşimi, temizlik, öncelik) söyler, geri al birleşimi duvarla
  birlikte döndürür.
- Birleşimin çözümü duvar değiştikçe yalnız çevresinde yapılır; büyük çizimin açılışı tek geçiştir.
- Açık duvarın birleşmiş köşesine bağlı ölçü yoktur: köşe iki nesnenin ortak noktasıdır, alt nesne başvurusu tek nesneyi anar.
  Odanın tek kapalı duvarla çizilmesi köşeleri bağlanabilir kılar.
- Kapı ve pencere (3. grup) duvarın yolunda yer tutacak; birleşimin kesintileri onların boşluklarına da uygulanacak.

## Doğrulama

- Çekirdek: `python3 scripts/fixtures/wall_join_cases.py --check` (31 durum, kesirlerle, KentOS kodu olmadan) ve
  `cargo test -p kentos-geometry-core --test all wall_joins`; deponun izlemesi `cargo test -p kentos-geometry-core --lib store::walls`;
  yolların düzenlenmesi `--lib wall_edit`; duvarın yerleri `--test all subref`.
- Komutlar: `python3 scripts/fixtures/wall_command_cases.py --check` (`cad.walls.create` 38, `.edit` 62, `.joins` 35 durum; birleşim
  uyarıları `wall_join_cases.py`'nin çözücüsünden), `python3 scripts/fixtures/explode_command_cases.py --check`;
  `KENTOS_CASES_ONLY=cad.walls cargo test -p kentos-native-application --test all fixtures`; izleyenler `--test all wall_follow`.
- `.kcad`: `python3 scripts/fixtures/kcad_v2_reference.py --check` (`wall-joins.kcad` ve altı bozuk dosya), `cargo test -p kentos-kcad`,
  taşıma `cargo test -p kentos-application --test carriage`.
- İz: `KENTOS_TRACES_ONLY=wall-edit cargo test -p kentos-desktop every_trace_passes`.
- Resimler: `KENTOS_SHOTS_ONLY=duvar-birlesim,duvar-birlesim-ayrintili,duvar-birlesim-x,duvar-duzenle-serit,duvar-birlesim-araci,duvar-birlesim-denetle,duvar-birlesim-oznitelikler,duvar-bol
  cargo test -p kentos-desktop tools_screens -- --ignored --nocapture` (`.run/shots/arac-duvar-*`).
