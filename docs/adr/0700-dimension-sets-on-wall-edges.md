# ADR 0700: Kapalı ve çok kenarlı duvarda ölçü dizileri, bütün kenarlar

- **Durum:** kabul edildi (27 Ekim 2026).
- **Bağlam:** `plan/architecture.md` ARCH-DIM-01; ADR 0695'in ilk dilimi yalnız iki uçlu düz duvarı ölçüyordu.
- **Web:** ertelendi (ADR 0228); TODOS.md §0.2.

## Bağlam

ADR 0695 kalıcı ölçü dizisini tek bir iki uçlu düz duvarın bir yüzüne bağladı. Kullanıcı odayı ya da yapıyı çoğunlukla
**Dikdörtgen duvar** (ADR 0627) ya da kapalı konturdan duvar (ADR 0637) ile çizer: bunlar dört (ya da daha çok) kenarlı **tek
kapalı duvardır**. Duvar ölçüleri aracı bu duvarları reddediyordu; mimari çizimin en sık ölçüsü olan dış ölçü zinciri bu yüzden
kurulamıyordu.

Araştırma (üretici belgeleri; yerelde çalıştırılmadı):

- [Archicad 29 — Automatic Exterior Dimensioning](https://help.graphisoft.com/AC/29/INT/_AC29_Help/070_Documentation/070_Documentation-49.htm):
  seçilen duvarlar için toplam, yapı ve açıklık zincirleri; açıklıkta merkez ya da uçlar; zincirler arası mesafe; **“Place
  Dimensions on Four Sides”** seçimin dört yanına zincir koyar; yön bir eleman kenarına tıklanarak verilir.
- [Vectorworks 2026 — Dimensioning exterior walls](https://app-help.vectorworks.net/2026/eng/VW2026_Guide/Dimensions/Dimensioning_exterior_walls.htm):
  dış duvar ölçüleri duvara bağlıdır; yüz/merkez ve açıklık uçları/merkezi ayrı seçilir (ADR 0695'te de dayanak).

KentOS kararı: dört yan için sınır kutusu yerine **duvarın kendi kenarları** kullanılır; böylece dönük ya da çokgen yapıda da
her zincir kendi kenarına paralel olur ve kaynak bağı korunur.

## Karar

### 1. Model: `edge`

`DimensionSet` isteğe bağlı `edge` alır: dizinin ölçtüğü duvar yolunun düz kenarı (köşe `edge` → `edge + 1`; kapalı duvarda
son kenar ilk köşeye döner). Yoksa iki uçlu duvarın tek kenarıdır: eski diziler ve dosyalar aynı kalır.

- Kaynak: açık ya da kapalı, en az iki köşeli duvar. Seçilen kenar düz olmalıdır (büküm 0); öteki kenarlar yay olabilir.
- Yüz, eskisi gibi yol yönüne göre `left`/`right` gerçek yüzdür. Yüz doğrusu, duvarın yüz çizgisinin (`wall::lines`) o kenara
  düşen parçasıdır: köşelerde birleşen (gönyeli) yüz köşeleri arasında. Kapalı dikdörtgende dış yüzün toplamı dış boydur
  (eksen boyu artı iki yarım kalınlık).
- Açıklıklar: pervazlarının ortası kenar boyunca yüz doğrusunun iki ucu arasında ve yüzden en çok duvar kalınlığı kadar
  içeride kalan açıklıklar; başka kenarın açıklığı o kenarın zincirine aittir. Kaynak yerleri (`Vertex{part: yüz, index:
  köşe}`) kenarın köşe numaralarıyla yazılır; ADR 0695'in kimlik ve üyelik kuralları aynen geçerlidir.
- Yüz doğrusunun köşeleri yolun köşeleriyle birebir olmalıdır: köşesi pahlanan (gönye sınırını aşan sivri köşe) ya da
  sıfır uzunluklu kenarı olan yüz `unsupportedWall` ile reddedilir, başka köşeye yanlış bağ kurulmaz.
- **Kenarın izi:** duvarın köşe sayısı değişirse (köşe ekleme/silme) kenar, eski kenarın iki ucu yeni yolda aynı iki ardışık
  köşe olarak **bit bit** bulunursa yeni numarasını alır; bulunmazsa dizi `unsupportedWall` ile kopar ve son geometrisi görünür
  uyarıyla kalır (sessiz başka kenar ölçmek yok).
- Dosya: `edge` taşıyan dizi KCAD **şema 114** ister; yoksa yazıcı eski şemayı seçer, eski dosyaların baytları değişmez.

### 2. Komut: `cad.dimensionsets.create` v1'in yeni alanları

- `edge` (isteğe bağlı): birden çok kenarlı duvarda zorunludur; iki uçlu duvarda yalnız `0` olabilir. Düz olmayan ya da duvarda
  olmayan kenar `invalid_dimension_set` (`edge`) ile reddedilir.
- `around` (isteğe bağlı, varsayılan yok): **Bütün kenarlar.** Duvarın her düz kenarına birer dizi, hepsi tek adımda.
  - Kapalı duvarda yüz **dış yüzdür** (yolun dönüş yönünden, işaretli alanla); `face` yok sayılır. Açık duvarda `face`
    bütün kenarlara uygulanır.
  - `at`, `edge`'in (yoksa ilk düz kenarın) yüzünden ilk sıranın uzaklığını verir (mutlak değer); her kenara aynı
    uzaklık, kenarın yüz doğrusunun ortasından yüzün dışına doğru uygulanır. İmleç odanın içindeyken de sıralar dışarıya
    konur.
  - Yay kenarlar atlanır; düz kenarı olmayan duvar reddedilir.
- Plan: bütün dizilerin ölçüleri ve segmentleri kenar sırasıyla. Çıktı `DimensionSetsChanged`'a `sets` eklenir:
  `around`'un yazdığı bütün diziler, kenar sırasıyla (`set` ilkidir); tek dizi yazan çağrıda alan yazılmaz, eski çıktılar
  bayt bayt aynıdır. Bütün diziler tek geri alma adımıdır (“Otomatik duvar ölçüleri”): biri yazılamazsa hiçbiri yazılmaz.
- İki uçlu açık duvarda `edge: 0` saklanmaz (eski tarif ve şema 113 korunur).
- Kenarsız eski çağrılar ve iki uçlu duvarın sonuçları değişmez.

### 3. Sunucu ve ortak hesap

Yüz doğrusunun kenar parçası ve kenardaki açıklıkların seçimi ortak çekirdekte tek yerdedir
(`ops::dimension_set::edge_face`, `on_edge`); masaüstü belgesi, başsız sunucu ve `project.changes`'in tutarlılık denetimi onu
kullanır. Sunucu, kenarlı dizinin üyelerini aynı hesapla doğrular.

### 4. Masaüstü aracı

Duvar ölçüleri, birden çok kenarlı duvarda imlece en yakın düz kenarı ve imlecin bulunduğu yüzü seçer; kapalı duvar dış
yüzüyle başlar ve yüzler “Dış yüz” / “İç yüz” adlarıyla görünür. Ölçülen yüz kenarı kesikli duvarın üstünde kalın çizgiyle
vurgulanır. Görünür ayarlara **Kenarlar: Bu kenar / Bütün kenarlar** eklenir (kullanıcının son seçimi hatırlanır, tek düz
kenarlı duvarda gösterilmez); Bütün kenarlar önizlemesi bütün zincirleri ve “n ölçü · k kenar · tek adım” etiketini
gösterir, kapalı duvarda yüz satırı “Dış yüz (bütün kenarlar)” olarak salt okunurdur. Tek tıklama/Enter/sağ tuş Onayla tek
adımda yazar, Esc yazmaz.

### 5. Belge izleme

Duvarın köşe sayısı ya da kapalılığı değişirse dizinin kenarı, eski kenarın iki ucu yeni yolda **bit bit** aynı iki ardışık
köşe olarak aranarak yeni numarasını alır (iki uçlu duvara köşe eklenince numarasız tarif de numara alır). Bulunamazsa dizi
`unsupportedWall` ile kopar, üyelerin son geometrisi ve kopuk bağ işaretleri kalır; geri al aynı adımda eski hâli getirir.

## Doğrulama

- Bağımsız başvuru `scripts/fixtures/dimension_sets_command_cases.py`: kapalı dikdörtgenin gönyeli yüz köşeleri ve kenardaki
  açıklıkların aralıkları elle hesaplanır (KentOS kodu yok); dört kenar, iç ve dış yüz, dönük ve büyük koordinat, yay kenar
  reddi, kenarsız çok kenarlı duvar reddi, `around`'un dört dizisi, tek geri al/yinele.
- Belge: `crates/native/domain/tests/all/dimension_sets.rs` köşe eklenince kenarın yeni numarasını, ucu silinince kopmayı ve
  geri almayı sınar. Araç: `crates/native/interaction/tests/all/wall_dimensions.rs` kapalı odada imlecin altındaki kenarı ve
  Bütün kenarlar'ın tek onay/tek geri al davranışını sınar.
- KCAD: `dimension-set-edges.kcad` ve `broken/dimension-set-edges-in-schema-113.kcad` bağımsız yazıcıyla
  (`scripts/fixtures/kcad_v2_reference.py`), Rust testi `crates/shared/kcad/tests/all/dimension_sets.rs`.
- Masaüstü: Dikdörtgen duvarla oda, kapı ve pencere, Duvar ölçüleri › Bütün kenarlar, tek onay ve geri al (resimler:
  `wall_scenes.rs`'in `olcu-*` sahneleri).
