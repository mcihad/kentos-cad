# İşlem araçları (Processing)

KentOS'un toplu işlem çatısıdır. QGIS Processing'e benzer ama KentOS'un belge
modeline, katmanlarına ve CAD alışkanlıklarına göre kurulmuştur. Bu belge
mimariyi, dizin yapısını, yeni araç yazma tarifini ve ileride gelecek
parçaların (worker, sunucu, PostGIS, modeller) nasıl takılacağını anlatır.
Kod ile bu belge çelişirse önce kodu doğrulayın, sonra belgeyi güncelleyin.

---

## 1. Çizim aracı ile işlem aracı arasındaki fark

| | Çizim aracı (`tools/`) | İşlem aracı (`processing/`) |
|---|---|---|
| Kullanım | Etkileşimli: tıkla, sürükle, yaz | Toplu: bir kez ayarla, çok nesneye uygula |
| Girdi | İmleç, kenet, komut satırı | Tanımlı parametreler (pencere, model, sunucu isteği) |
| Belgeye dokunur mu | Evet, `doc.transact` ile kendisi yazar | **Hayır.** Değişiklik kümesi (`ChangeSet`) döndürür; çalıştırıcı yazar |
| DOM | Yok, `ctx.view` üzerinden | Yok, `ctx` de yok; yalnızca girdiler ve salt okunur belge |
| Nerede çalışır | Tarayıcıda, ana iş parçacığında | Tarayıcı; ileride worker, sunucu, PostGIS |
| Arayüz | Katalogdan üretilen düğme, istem, seçenekler | Tanımdan üretilen pencere, araç kutusu, menü, komut |

Örnekler: köşe noktalarını numaralandırmak, kenar uzunluklarını yazmak, bir
katmandaki bütün parsellerin alanını özniteliğe yazmak, çizgileri
sadeleştirmek, bir ada içindeki parselleri yeniden numaralamak.

## 2. İlkeler

1. **Bildirimsel tanım.** Bir araç ne yaptığını, hangi parametreleri aldığını, hangilerinin zorunlu olduğunu, varsayılanlarını, sınırlarını, çıktılarını ve nerede çalışabileceğini bir nesne olarak söyler. Pencere, araç kutusu, menü, komut satırı takma adları, geçmiş ve modeller bu tanımdan üretilir.
2. **Saf çalışma.** `run(values, ctx, feedback)` belgeyi değiştirmez; `ChangeSet` döndürür. Böylece araç test edilebilir, zincirlenebilir, geri alınabilir (tek adım) ve başka bir çalışma yerine taşınabilir.
3. **Tipler tanımdan çıkar.** `defineTool` parametre listesinden `run`'ın aldığı değerlerin tiplerini türetir; tanım ile kod birbirinden ayrışamaz.
4. **Kullanıcı dili.** Etiket, açıklama, yardım, hata ve özet metinleri Türkçedir ve kullanıcıya yöneliktir. Kimlikler ve kod İngilizcedir.
5. **Katman bağımlılığı.** `processing/` yalnızca `core`, `geo` ve `model` içe aktarır. DOM, `render`, `viewport`, `tools`, `ui` ve `app` bilmez. Bu, aracın worker'da ve (aynı TypeScript ile) sunucuda çalışabilmesinin koşuludur.

## 3. Dizin yapısı

```
apps/web/src/processing/
  types.ts           Sözleşme: parametre türleri, değer tipleri, ProcessingTool, ChangeSet, RunContext, Feedback, defineTool
  parameters.ts      Varsayılanlar, görünürlük, doğrulama (kullanıcı mesajlı), kayıtlı değerleri güvenle geri yükleme
  features.ts        Nesne kapsamını çözme (seçili, görünen, tümü, katman, kimlikler) ve "12 kapalı alan" özetleri
  categories.ts      Araç kutusu kategorileri (üst kategori destekli)
  registry.ts        ProcessingRegistry: kayıt, kategori ağacı, Türkçe katlamalı arama, version sinyali
  runner.ts          ProcessingRunner: doğrula → sayfaya bağlıyı çöz (RunJob) → çalışma yerini seç → çalıştır → tek geri alma adımıyla uygula → geçmiş
  job.ts             RunJob, FeatureRef, Executor arayüzü, materialize, runJob (sayfa ve worker aynı), EntitySnapshot, clientExecutor
  geometry.ts        Çalıştırmanın geometri deposu (ObjectStore) ve araçların sorduğu geometri (RunGeometry), görünümün deposu (DocumentGeometry)
  model.ts           Modeller (akış diyagramı) veri yapısı, tür uyumu, sıralama ve denetim
  modelRunner.ts     Modeli çalıştırma (tek geri alma adımı, hata ve Durdur'da geri alma), modelAsTool
  modelEdit.ts       Model taslağını düzenleme (tasarımcının işlemleri, saf)
  processing.test.ts Birim testleri
  cases.test.ts      İki platformun ortak durumları (fixtures/processing/v1): sayfada ve işçinin yolundan
  (ifade dili model/expression/ altındadır: stil motoru da kullanır)
  worker/
    protocol.ts      Sayfa ↔ worker mesajları
    handleJob.ts     Worker içinde bir işi çalıştırma (sahte worker'la test edilir)
    workerExecutor.ts  Executor: worker'ı başlatır, işi gönderir, Durdur'da sonlandırır
    processingWorker.ts  Worker girişi (yerleşik araçlar)
    worker.test.ts   Sayfa ile worker aynı sonucu verir, Otomatik seçim, hata ve Durdur
  builtin/
    index.ts         BUILTIN_TOOLS listesi
    numbering.ts     Numara biçimi ve adlar (nameCorners); halka yönü, başlangıç köşesi ve ortak köşe çekirdekte
    vertexNumbering.ts   points.numberVertices: Köşe noktalarını numarala
    edgeLengths.ts       annotation.edgeLengths: Kenar uzunluklarını yaz
    calculateField.ts    attributes.calculate: Öznitelik hesapla
    selectByExpression.ts  selection.byExpression: İfadeyle seç
    models.ts        Yerleşik modeller (Parsel ölçü yazıları)

apps/web/src/app/processing.ts         ProcessingService (registry + runner + son değerler), komut kaydı
apps/web/src/ui/processing/
  ToolDialog.ts      Tanımdan üretilen araç penceresi
  paramFields.ts     Parametre türü başına kontrol
  dialogPlan.ts      Pencerenin sayfasız kuralları: form, bölümler, sorunlar, önizleme, alt satır, Nerede çalışır,
                     pencerenin durumu ve gösterdiği (fixtures/processing/v1/dialog.json)
  fieldPlan.ts       Alanların sayfasız kuralları: kapsam, türler, hedef katman, nokta, öznitelik adı, ifade yardımcıları
  dialogTexts.ts     Pencerenin sözleri
  dialogFixture.ts   dialog.json'un oturumlarını oynatan (denetleyici ve kaydedici; uygulama yüklemez)
  ProcessingPanel.ts Sağ doktaki araç kutusu (Modeller dalı dahil) ve geçmiş
  model/             Model tasarımcısı: ModelDesigner, ModelCanvas, modelPalette, modelInspector
apps/web/src/tools/pickPointTool.ts    Nokta parametresinin ve noktalı seçimin "Sahneden seç"i
apps/web/src/tools/pickObjectsTool.ts  Girdi nesnelerinin "Sahneden seç"i (ADR 0088)
apps/web/src/styles/processing.css     Pencere ve panel stilleri
apps/web/src/styles/model.css          Model tasarımcısı stilleri
```

Yeni bir araç ailesi büyüdükçe `builtin/` altında alt klasör açılır
(`builtin/cadastre/…`). Geometri araç dosyasında hesaplanmaz: Rust
çekirdeğindedir (`crates/shared/geometry-core`; işlem araçlarına özgü olanlar
`apps/web/src/processing/` altında: köşe numaralama, kenar ölçüleri) ve araç onu
`ctx.geometry` ile nesne kimliğinden sorar (ADR 0008 S4). Araç dosyasında
metin, sayaç, süzgeç ve akış kalır.

## 4. Sözleşme

### 4.1 Araç

```ts
export const vertexNumbering = defineTool({
  id: 'points.numberVertices',          // alan.eylem, İngilizce, değişmez
  label: 'Köşe noktalarını numarala',   // pencere başlığı, menü, araç kutusu
  category: 'points',                   // categories.ts kimliği
  icon: 'numberVertices',               // ui/icons.ts
  description: 'Tek cümle: ne yapar.',
  help: 'Paragraflar boş satırla ayrılır.',
  keywords: ['numara', 'köşe', 'vertex'],   // arama
  aliases: ['KOSENUMARA', 'KNUM'],          // komut satırı
  targets: ['client', 'worker'],            // tercih sırasıyla
  parameters: [ … ] as const,               // `as const` şart
  outputs: [{ name: 'points', label: 'Numaralı noktalar', type: 'features' }],
  validate: (v) => … ?? null,               // parametreler arası denetim
  preview: (v) => 'P00001, P00002 …',       // pencerede canlı önizleme
  run: (v, ctx, feedback) => ({ changes: { add }, outputs: { count }, summary: '…' }),
});
```

- `parameters` dizisi `as const` ile yazılır; `defineTool<const Ds>` bu listeden `run` ve `validate` için değer tiplerini çıkarır.
- **Tanım içindeki ok fonksiyonlarının argümanı tiplenir:** `visibleWhen: (v: Shown) => …`, `default: (c: DefaultsContext) => …`. Tipsiz ok fonksiyonu TypeScript'in çıkarımını durdurur ve bütün değerler birleşim tipine düşer.
- `summary` geçmişte, günlükte ve pencerenin alt çubuğunda görünen tek satırdır: "68 nesnede 126 köşe numaralandı: P00001 – P00126."

### 4.2 Parametre türleri

| Tür | Pencerede değer (`ParamValues`) | `run`'da değer (`ResolvedValues`) | Seçenekler |
|---|---|---|---|
| `features` | `{ scope: 'selection' \| 'visible' \| 'all' }`, `{ scope: 'layer', layerId }`, `{ scope: 'ids', ids }` | `FeatureSet { entities, description }` | `kinds` (uygun nesne türleri), `scopes` (sunulan kapsamlar), `writes` (araç bu nesneleri değiştirir) |
| `number` | `number` | aynı | `min`, `max`, `integer`, `unit` |
| `string` | `string` | aynı | `placeholder`, `maxLength`, `allowEmpty` |
| `boolean` | `boolean` | aynı | — |
| `enum` | seçenek değeri (dar tip) | aynı | `options: { value, label, hint? }[]` |
| `layer` | `{ layerId }` ya da `{ newName }` | `TargetLayer { id, name, isNew }` | `newLayerStyle` |
| `point` | `Vec2 \| null` | aynı | — |
| `expression` | ifade metni | `CompiledExpression` (isteğe bağlı ve boşsa `null`) | `returns: 'condition' \| 'value'`, `of` (okuduğu `features` parametresi), `placeholder` |
| `field` | alan adı (`multiple`: virgülle ayrılmış adlar) | aynı (kırpılmış) | `of` (alanları sunulan `features` ya da `file` parametresi; bir liste ise görünen ilki), `allowNew` (yeni alan adı yazılabilir), `multiple` (liste adları işaretler) |
| `file` | `{ name, rows }` (masaüstünde `path` de) ya da `null` | aynı | `accept` (sunulan uzantılar). Dosya seçilince Tablo ekle'nin okuyucusuyla okunur (ilk sayfa, ilk satır sütun adları; ADR 0200 §7); son değerlerde yalnız adı (masaüstünde yolu) kalır, pencere yeniden açılınca dosya yeniden seçilir (masaüstünde yolundan okunur). Model girdisi olamaz, model adımında seçilmez |

Ortak alanlar: `name` (değer anahtarı), `label`, `description`, `optional`,
`advanced` ("Gelişmiş ayarlar" altında), `visibleWhen` (yalnızca koşul
sağlanınca gösterilir ve denetlenir), `default` (sabit ya da
`(c: DefaultsContext) => …` ile proje ayarından; ör. ondalık basamak).

- **Zorunluluk:** parametreler varsayılan olarak zorunludur. `optional: true` olan boş (`null`) bırakılabilir ve pencerede "isteğe bağlı" yazar. Metinde boş değer ayrıca `allowEmpty` ister (boş önek gibi).
- **Nesne türü süzgeci:** `features` değeri isteğe bağlı `kinds` taşır. Kapsamda aracın alabildiği iki ya da daha çok tür varsa pencere her tür için sayılı bir düğme gösterir ("Kapalı alan 118"); kullanıcı bu çalıştırmada yalnızca bazı türleri alabilir (örneğin yalnızca kapalı alanların kenarlarını yazmak). Hiç tür kalmazsa araç çalışmaz.
- **Kapsamlar:** "Seçili" seçimdeki nesneler; "Görünen" kutusu ekrandaki görünür alanla kesişen, görünür katmanlardaki nesneler (yardımcı çizgiler hariç; kutu testini görünümün geometri deposu yapar); "Tümü" görünür katmanlardaki bütün nesneler; "Katman" bir katman ya da grubun altındaki bütün katmanlar (gizli olsa bile); "ids" modellerde önceki adımın çıktısıdır ve pencerede sunulmaz. `kinds` dışındaki nesneler sessizce elenir; pencere ne kadar nesne okunacağını canlı gösterir.
- **Boş girdi:** zorunlu bir `features` parametresi hiç nesneye çözülmezse çalıştırıcı aracı çalıştırmaz ve alanın altına yönlendiren bir mesaj yazar ("Önce nesneleri seçin ya da kapsamı değiştirin"). Model içinde (`ids`) boş çıktı hata değildir.
- **Değiştirilen girdi (`writes`):** araç girdisinin nesnelerini değiştiriyorsa (Öznitelik hesapla) kilitli katmandaki nesneler girdiye alınmaz ve çalıştırma başında söylenir: "“Nesneler”: 1 nesne kilitli katmanda olduğu için işleme alınmadı." Özet, çıktılar ve modelin sonraki adımı yalnız yazılanı sayar. Nesnelerin hepsi kilitliyse araç çalışmaz: "“Nesneler”: seçili nesnelerin hepsi kilitli katmanda. Kilidi Katmanlar panelinden açın." (kapsamın sözüyle: görünen alandaki, görünen katmanlardaki, bu katmandaki). Uygulamadaki kilitli katman atlaması güvenlik ağı olarak kalır.
- **Hedef katman:** `{ newName }` aynı adlı bir katman varsa onu kullanır (araç ikinci kez çalışınca aynı "Köşe noktaları" katmanına yazar); adlar kırpılmış ve Türkçe harfler katlanarak karşılaştırılır ("kose noktalari" = "Köşe noktaları", `sameNamedLayer`), pencere de "(mevcut)" demeyi bu kuralla, var olan katmanın adıyla söyler. Yoksa katman yalnızca araç gerçekten ona yazarsa oluşturulur. Kilitli katman seçilemez; kilitli katmana düşen değişiklikler atlanır ve sayısı bildirilir.

### 4.3 Çalışma bağlamı ve değişiklik kümesi

```ts
run(values: ResolvedValues<Ds>, ctx: RunContext, feedback: Feedback): RunResult | Promise<RunResult>

RunContext { doc: DocumentSnapshot /* get, all, byLayer; salt okunur */, units: DefaultsContext,
             layerName(id): string, selection: readonly number[] /* çalıştırma başındaki seçim */,
             geometry: RunGeometry /* girdilerin geometrisi, kimlikten: measures, numberCorners, cornerTexts, edgeLengths, relatePairs */,
             field(layerId, name): LayerField | undefined /* katmanın alanı (ADR 0199 §1) */,
             crs: ProjectCrs | null /* projenin sistemi: srid, code, system, choices (ADR 0201 §8); masaüstünde ctx.doc.settings() */ }
Feedback   { progress(fraction, label?), info(m), warn(m), canceled, yield() }
ChangeSet  { add?: NewEntity[], update?: { id, patch }[], remove?: number[] }
RunResult  { changes?, select?: readonly number[] /* çalıştırmadan sonraki seçim */, outputs?, summary?,
             refused?: string /* araç reddeder: çizim değişmez, ileti olduğu gibi */ }
```

- Uzun döngülerde `await feedback.yield()` sayfanın donmasını önler (16 ms'de bir gerçekten bekler) ve `feedback.canceled` denetlenir. İptal edilen çalıştırmanın değişiklikleri uygulanmaz.
- `ctx.units.plotScale` kâğıt ölçüsünü dünyaya çevirir: 2 mm yazı, 1:1000'de 2 m'dir.
- Araç `NewEntity` üretirken `layerId` olarak hedef katmanın `id`'sini kullanır; yeni katman o anda henüz yoktur, çalıştırıcı uygularken kurar.
- **Seçim üreten araçlar** (İfadeyle seç) belgeyi değiştirmez, `select` döndürür; çalıştırıcı seçimi uygular (`FeatureHost.select`). Geri alınacak bir şey yoktur. Mevcut seçimle birleştirme (ekle, çıkar, içinde ara) aracın işidir, `ctx.selection` ile yapılır.
- **Geometri çekirdekten gelir** (`ctx.geometry`, `processing/geometry.ts`): çalıştırma, features girdilerinin nesnelerini kendi geometri deposuna paketler (ilk soruda; bitince bırakılır). Araç kimlikle sorar: ifadelerin geometri değerleri (`measures`, `measuredOf` ile bütün nesneler için bir kez), köşe numaralama (`numberCorners`: her köşenin yeri, dışa bakan yönü ve kimin numarasını aldığı; adları `nameCorners` verir), köşe yazısının yeri (`cornerTexts`), kenar ölçüsü yazıları ve ortak kenar testi (`edgeLengths`).
- **Öznitelik değiştiren araçlar** `update` içinde `attrs` alanının tamamını verir (`{ ...e.attrs, [alan]: değer }`); yalnızca öznitelik değişirse belge `attrs` olayı yayar ve GPU tamponu kurulmaz.
- **`features` çıktıları:** `outputs[ad]` bir kimlik dizisiyse o kullanılır (seçilenler, değişenler); yoksa çalıştırmanın eklediği nesneler çıktıdır. Modeller bu kimlikleri sonraki adıma `{ scope: 'ids' }` olarak verir.
- **`table` çıktısı** (`{ columns, rows }`, metinler; Özet istatistik, Geçerliliği denetle, Onar, Sadeleştir): pencere çalıştırmadan sonra formun altında gösterir (Panoya kopyala, CSV olarak kaydet); modelde sonraki adıma geçmez.
- **Geometri işlemleri** (ADR 0201): hesap çekirdeğin `ops::geoprocess`'idir (web `model/ops/geoprocess.ts` ile nesneleri bütün verir, masaüstü işlevleri şekillerle çağırır); araç sonucu kendi Çıktı katmanına yeni nesne olarak yazar, girdi değişmez. Ortak notlar (`builtin/geometry/shared.ts`, `builtin/geometry/mod.rs`): elips ve eğrinin 0,1 mm'lik doğru parçalarıyla girdiği, kotların taşınmadığı, sonucu boş kalan nesneler.
- **Katmanın alanları:** çalıştırıcı değişiklik kümesini uygulamadan önce öznitelik yazmalarını katmanın alanlarının kuralıyla denetler (`processing/writeCheck.ts`, masaüstünde `kentos_processing::writes`; ADR 0199 §1, 0200 §3): alana yazılan değer alanın tek biçimine çevrilir, uymayan ilk değer bütün çalıştırmayı reddeder ("Öznitelik yazılamadı (#id): …"). Araçlar değeri yazarken `ctx.field` ile aynı biçime çevirir; aynı kalan değeri yeniden yazmaz.

### 4.4 Çalıştırma akışı (`ProcessingRunner.run`)

```
değerler ─► validateValues (parametre + araç düzeyi) ── sorun varsa ─► { status: 'invalid', issues }
        ─► girdileri çöz: features → FeatureSet, layer → TargetLayer   (boş girdi → invalid)
        ─► executorFor(tool): targets sırasıyla ilk uygun Executor
        ─► executor.execute(tool, çözülmüş değerler, { doc, units }, feedback)
        ─► iptal edildiyse değişiklik yok
        ─► apply: tek doc.transact (tek geri alma adımı), yeni katmanlar, kilitli katman atlama
        ─► RunRecord geçmişe (en çok 100), { status: 'ok', result, added, record }
```

`running` sinyali ilerlemeyi, `history` sinyali geçmişi yayınlar. Pencere ve
panel bunlara abone olur.

## 5. İfade dili

Koşul ve değer parametreleri (`expression`) küçük, güvenli bir ifade dili kullanır; stil motoru da aynı dili kullanır. Dil Rust'tadır, `crates/shared/expression` (`kentos-expression`, ADR 0100); web ona `model/expression/expression.ts` ve `expressionLib.ts` üzerinden WASM'la, masaüstü doğrudan ulaşır. `eval` yoktur: metin sözcüklere ayrılır, öncelik tırmanmasıyla ayrıştırılır ve değerlendirilir. Hata mesajı yerini söyler: "15. karakterde: İfade yarım kalmış: sonunda bir değer eksik."

```
Nitelik = 'Arsa' ve $alan > 500
'P' || doldur($sıra, 5)
yuvarla([Tapu alanı (m²)] - $alan, 2)
eğer(boş(Parsel), 'numarasız', Ada || '/' || Parsel)
```

- **Alanlar:** düz ad (`Parsel`) ya da boşluk ve işaret içerenler için köşeli parantez (`[Tapu alanı (m²)]`). Olmayan alan boş (`null`) verir; pencere ifadenin okuduğu ama nesnelerde olmayan alanları önizlemede söyler.
- **Değerler:** sayı (ondalık ayırıcı nokta), metin (`'…'` ya da `"…"`, içte çift tırnak bir tırnaktır), `doğru`/`true`, `yanlış`/`false`, `boş`/`null`.
- **İşleçler:** `ve`/`and`, `veya`/`or`, `değil`/`not`; `= != <> < <= > >=`; `+ - * / %`; `^` üs alır (işaretten önce, sağdan sola: `-2 ^ 2` = −4, `2 ^ 3 ^ 2` = 512); `||` metin birleştirir. `+` iki taraf da sayıysa toplar, değilse birleştirir.
- **Koşul sözleri** (ADR 0100 §4; QGIS'teki karşılığı takma addır):
  - `durum eğer koşul ise değer … [yoksa değer] son` (`CASE WHEN … THEN … ELSE … END`): ilk doğru koşulun değeri; seçilmeyen dallar hesaplanmaz.
  - `x [değil] içinde ('Arsa', 'Tarla')` (`IN`), `x [değil] arasında 3 ve 5` (`BETWEEN`).
  - `x [değil] gibi 'P___12'` (`LIKE`: `%` herhangi bir metin, `_` tek karakter, `\` ardındakini olduğu gibi alır), `x benzer 'çın%'` (`ILIKE`: büyük/küçük ve Türkçe harf farkı gözetilmez).
  - `x boş`, `x boş değil` (`IS NULL`, `IS NOT NULL`).
  - Bu sözler yalnız durabilecekleri yerde sözdür: Durum, Son, Gibi adlı alanlar alandır. Dil iki değerlidir: boş bir değerle `arasında`, `gibi`, `içinde` yanlış, `değil`'li biçimleri doğrudur.
- **Tür kuralları:** öznitelikler metindir; aritmetik ve karşılaştırma metindeki sayıyı okur ("472.27" → 472.27). Boş bir değerle aritmetik boş verir, karşılaştırma yanlış verir; `= boş` yalnızca boş için doğrudur. Sıfıra bölme boştur. Metin karşılaştırması Türkçe sıralamayla ve büyük/küçük harfe duyarlıdır; `içerir`, `başlar`, `biter` harf farkı gözetmez.
- **Değişkenler:** `$alan`, `$uzunluk` (`$çevre`), `$köşe`, `$tür`, `$katman`, `$etiket`, `$y` (sağa), `$x` (yukarı), `$sıra` (bu çalıştırmadaki sıra, 1'den), `$id`; yalnızca sembol çizilirken `$ölçek` (çizim ölçeğinin paydası; işlem araçlarında boştur).
  - Geometriden doğrudan okunanlar (ADR 0100 §3): `$merkez_y` ve `$merkez_x` (ağırlık merkezi: kapalı alanda, taramada, dairede ve tam elipste alanın merkezi, delikler düşülür; öbür nesnelerde yer noktası); `$min_y`, `$max_y`, `$min_x`, `$max_x` (sınır kutusu; Y sağa, X yukarı); `$genişlik`, `$yükseklik`.
  - Web'de bu yeniler ifade geometri deposunda değerlendirildiğinde dolar (`ExprObjects.geometry`); ölçü kaydıyla giden yolda boştur.
- **Tipli alanlar:** çağıran bir şema verirse (`compile_with`), şemadaki kullanıcı alanları türleriyle okunur: sayı alanı sayıdır (metinden okunmaz), doğru/yanlış alanı doğru/yanlıştır, tarih ISO metnidir (YYYY-AA-GG; metin olarak karşılaştırılır). Şemada olmayan ad bugünkü gibi metin özniteliğidir.
- **İşlevler** (Türkçe adı ve QGIS'teki İngilizce adıyla): `yuvarla/round`, `metin/to_string` (sabit ondalık), `sayı/to_real`, `tamsayı/int`, `mutlak/abs`, `min`, `max`, `büyük/upper`, `küçük/lower`, `kırp/trim`, `uzunluk/length`, `parça/substr`, `doldur/lpad`, `değiştir/replace`, `içerir/contains`, `başlar/starts_with`, `biter/ends_with`, `eğer/if`, `boş/is_empty`, `varsayılan/coalesce`; `kök/sqrt`, `tavan/ceil`, `taban/floor`, `pi`, `sol/left`, `sağ/right`, `bul/strpos` (1'den; yoksa 0), `birleştir/concat` (boş değer bir şey eklemez), `sağdoldur/rpad`; harita cebirinin (ADR 0233 §3) `ln`, `log10`, `log(taban, sayı)`, `üstel/exp`, `sin`, `cos`, `tan`, `asin`, `acos`, `atan`, `atan2(y, x)` (açılar radyan), `derece/degrees`, `radyan/radians` (libm'den; sonlu olmayan sonuç boş).
- **Adlar Türkçe harf farkı gözetmez:** `YUVARLA` = `yuvarla`, `$cevre` = `$çevre`, `DEGIL` = `değil`.
- Yazılan değer metne `toText` ile çevrilir: tam sayılar ondalıksız, ondalıklar kayan nokta gürültüsü atılarak (0.1 + 0.2 → "0.3"), doğru/yanlış olarak.

Pencerede ifade alanı tek satırdır (komut satırı gibi eşaralıklı yazıyla). Altında girdi nesnelerinin alanları düğme olarak (tıklayınca imlecin yerine eklenir), "Değişkenler" ve "İşlevler" menüleri (her biri ne yaptığını söyler) ve canlı bir satır bulunur: koşulda "16 / 340 nesne koşulu sağlıyor.", değerde "İlk nesnede (10): “472.26”."

Yeni işlev eklemek için `crates/shared/expression/src/library.rs`'teki `FUNCTIONS` tablosuna ad, İngilizce (QGIS) karşılık, değer sayısı, grup, argümanlar, örnekler ve açıklama, `functions.rs`'teki `call`'a hesabını ekleyin (sütun motoru `kernels.rs`'te aynı kuralı çağırır); `src/tests.rs`'e ve `fixtures/expression/v2/language.json`'un bağımsız başvurusuna (`scripts/fixtures/expression_language.py`) durum yazın. Menüler, İfade oluşturucunun ağacı ve yardımı bu tablodan beslenir.

## 6. Çalışma yerleri (client, worker, server, postgis)

Her araç `targets` ile nerelerde çalışabileceğini tercih sırasıyla bildirir.
Bir çalıştırma iki yarıya ayrılır:

1. **Sayfada, çalıştırıcı** (`runner.ts`) sayfaya bağlı olanı çözer: `features` değerlerini nesne kimliklerine (`FeatureRef { ids, description }`), `layer` değerlerini hedef katmana (`TargetLayer`). Sonuç, kopyalanabilir bir veridir (`RunJob`, `job.ts`): araç kimliği, değerler, birimler, çalıştırma başındaki seçim, katman adları.
2. **Çalışma yerinde, `Executor`** işi alır, elindeki belgeye göre `materialize` eder (kimlikler → nesneler, ifadeler → derlenmiş), aracı çalıştırır ve `RunResult` döndürür. Sonucu her zaman sayfadaki çalıştırıcı uygular (tek geri alma adımı).

```ts
interface Executor {
  readonly target: 'client' | 'worker' | 'server' | 'postgis';
  available(): boolean;
  supports?(tool): boolean;                 // worker yalnızca kendi içindeki araçları bilir
  execute(tool, job: RunJob, doc: DocumentSnapshot, feedback): Promise<RunResult>;
}
```

| Yer | Durum | Nasıl |
|---|---|---|
| `client` | Var (`clientExecutor`) | Araç sayfada, canlı belge üzerinde çalışır. |
| `worker` | Var (`worker/workerExecutor.ts`) | Web Worker ilk kullanımda başlar ve açık kalır (`worker/processingWorker.ts`, yerleşik araçları taşır). Belge, nesnelerinin kopyasıyla gider (`EntitySnapshot`); iş sayfadaki gibi `runJob` ile çalışır: ifadeler worker'da derlenir, okunan nesneler worker'ın kendi geometri deposuna konur. İlerleme ve günlük satırları mesajla gelir. **Durdur** worker'ı sonlandırır (sıkı döngüdeki bir araca rica edilemez); sonraki iş yeni bir worker açar. Worker çökerse iş hata olarak biter. Mesaj alışverişi `worker/protocol.ts`'te, iş yürütme `worker/handleJob.ts`'te durur (testler sahte bir worker'la sürer). |
| `server` | Planlı | Aynı `RunJob` KentOS servisine gider (belge sürümüyle); ilerleme bir akıştan (SSE/WebSocket) gelir. Sonuç yine `ChangeSet`'tir. Aynı TypeScript araçları Node'da çalışabilir (`handleJob` sunucuda da kullanılabilir). |
| `postgis` | Planlı | Araç, `run`'a ek olarak bir `sql` üreticisi verir; sunucu bunu PostGIS'te parametreli sorgu olarak çalıştırır. Sonuç `ChangeSet`'e çevrilir ya da veritabanında kalır. |

- **Seçim:** kullanıcı pencerenin sağ panelindeki "Nerede çalışır" listesinden seçer ve seçim araç başına hatırlanır (`kentos.processing.v1`). **Otomatik** (varsayılan), girdiler `WORKER_THRESHOLD` (2 000) nesne ve üstündeyse worker'ı, değilse aracın ilk tercihini kullanır; pencere o anki kararı yazar ("şimdi: bu tarayıcıda"). Aracın bildirdiği ama bu ortamda olmayan yerler "yakında" diye, seçilemez olarak listelenir. Yalnız bir yer varsa Otomatik sunulmaz, o yer "bu çalıştırmada" notuyla işaretlidir; saklanan seçim bu ortamda yoksa çalıştırma Otomatik'le (tek yerde o yerle) yapılır.
- **Kural:** `run` DOM'a, `ctx` dışındaki servislere ve modül düzeyinde değişen duruma dokunmaz; sonuç `RunResult` yapılandırılmış kopyayla taşınabilir olmalıdır (işlev, sınıf örneği yok). Yalnızca yerleşik araçlar worker'dadır; eklenti araçları `targets`'ta `worker` bildirse de worker onları bilmedikçe (`supports`) sayfada çalışır.
- Geçmiş, her çalıştırmanın nerede çalıştığını saklar ve gösterir ("130 ms, arka planda").

## 7. Modeller (akış diyagramları)

Araçlar birbirine bağlanarak modeller kurulur (QGIS Model Designer gibi). Bir model, adımları ve girdileri olan bir veridir; tek başına bir araç gibi çalıştırılır, araç kutusunda ve menüde görünür.

```ts
ValueSource = { kind: 'value', value } | { kind: 'input', name } | { kind: 'output', step, output }
ModelStep   { id, tool, values: Record<string, ValueSource>, position?, caption? }
ProcessingModel { id, label, category, description, inputs: ParamDef[], steps, outputs, inputPositions? }
```

- **Bağlantılar:** bir adımın parametresi sabit bir değer, modelin kendi girdisi, önceki bir adımın çıktısı ya da (değer verilmemişse) aracın varsayılanıdır. `features` çıktısı sonraki adıma `{ scope: 'ids', ids }` olarak geçer; adımlar birbirinin ürettiği, seçtiği ya da değiştirdiği nesneler üzerinden zincirlenir.
- **Tür uyumu** (`canFeed`): aynı tür; sayı metne; metin ifadeye ve alan adına. `checkModel` bilinmeyen aracı, uyumsuz bağlantıyı, olmayan girdi ya da çıktıyı, değeri gereken ama bağlanmamış parametreyi (görünürlük koşuluyla) ve döngüyü kullanıcı diliyle bildirir. `orderSteps` bağımlılık sırasını verir.
- **Çalıştırma** (`modelRunner.ts`, `runModel`): girdiler doğrulanır, model denetlenir, adımlar sırayla `runner.run(..., { silent: true })` ile çalışır. Bütün model **tek geri alma adımıdır**: `CadDocument.beginGroup` farklı zamanlarda (await arasında) yapılan işlemleri bir araya toplar. Bir adım çalışmazsa ya da Durdur'a basılırsa grup iptal edilir (`cancel`), önceki adımların yaptıkları geri alınır; ileti hangi adımın neden çalışmadığını söyler. Geçmişe model bir kayıt olarak girer (`model:<id>`). Adımlar çalışma yerlerini kendileri seçer (Otomatik); kullanıcı "Arka planda" seçerse bunu destekleyen adımlar worker'da çalışır.
- **Pencere:** `modelAsTool` modeli bir aracın biçimine koyar (girdileri parametre olur); aynı `ToolDialog` açılır, sağ panelde adımlar sırasıyla ve "Modeli düzenle" düğmesi durur.
- **Kitaplık:** yerleşik modeller (`builtin/models.ts`) değiştirilemez; düzenlenirse kopyası açılır. Kullanıcının modelleri bu tarayıcıda saklanır (`kentos.processing.v1` → `models`). Proje dosyası ve sunucu gelince modeller projeye ve paylaşılan kitaplığa da yazılabilecek. Her model bir komuttur (`processing.model.<id>`); kitaplık değişince komutlar yenilenir.
- **Düzenleme işlemleri** (`modelEdit.ts`, saf ve testli): girdi ekle/sil, adım ekle (seçili kutuya bağlanarak)/sil, kaynak ata, uygun kaynakları listele (döngü oluşturacak adımları dışarıda bırakır), bağlantıları kenar olarak çıkar, sütunlara diz, kopyala.

### 7.1 Model tasarımcısı

`ui/processing/model/`: `ModelDesigner` (pencere, taslak, kendi geri alma yığını, kaydetme), `ModelCanvas` (diyagram), `modelPalette` (sol), `modelInspector` (sağ). Sözler, kurallar ve diyagramın geometrisi `designerPlan.ts`'te, modelin düzenlemeleri `processing/modelEdit.ts`'tedir; `fixtures/processing/v1/designer.json` ikisini sabitler. Davranışın ayrıntısı, web kodunu görmemiş okuyucu için: [specs/model-designer.md](specs/model-designer.md).

- **Sol:** "Girdi ekle" (Nesneler, Sayı, Metin, Evet/hayır, Katman, Nokta) ve aranabilir araç listesi. Bir araca tıklamak seçili kutunun sağına ekler ve ilk uygun girdisini seçili kutuya bağlar; tuvale sürüklemek bırakılan yere koyar.
- **Orta:** girdiler (mavi kenarlı) ve adımlar kutu, bağlantılar eğridir; eğri ortasında hangi parametreyi beslediği yazar. Kutular sürüklenir (10 px ızgaraya), boş alan sürüklenince tuval kayar, tekerlek yakınlaştırır, çift tık hepsini gösterir. Bir kutunun sağındaki noktadan sürükleyip bir adımın üstüne bırakmak, o adımın uygun girdilerini (ve adımın çıktılarını) bir menüde sorar. Sorunlu adım kesik turuncu kenarla ve ilk sorunuyla görünür.
- **Sağ:** hiçbir şey seçili değilken modelin adı, kategorisi, açıklaması, çıktıları ve sorunları; girdi seçiliyken etiketi, açıklaması, isteğe bağlılığı ve türüne göre varsayılanı; adım seçiliyken başlığı ve her parametre için **kaynak** (aracın varsayılanı, sabit değer, girdi, önceki adımın çıktısı ya da "Yeni model girdisi yap"). Sabit değer, araç penceresindeki denetimin aynısıyla girilir.
- **Alt:** "Düzenle" (sütunlara diz), durum (sorun sayısı ve ilki ya da "Model çalışmaya hazır"), Kapat, "Kaydet ve çalıştır…", Kaydet. Kaydedilmemiş değişiklikle kapatmak pencerenin üstünde sorar (Kaydetmeden kapat, Vazgeç, Kaydet ve kapat). Ctrl+Z / Ctrl+Y taslakta geri alır, Ctrl+S kaydeder, Delete seçili kutuyu siler. Sorunlu model kaydedilebilir ama çalışmaz.

## 8. Arayüz

- **Araç kutusu:** sağ dokun üst yuvasında "Katmanlar | İşlemler" sekmeleri. İşlemler sekmesinde arama (Türkçe harfler katlanır: "kose" = "köşe"), kategori ağacı (katlama durumu `ui.processingFolded`) ve "Araçlar | Geçmiş" seçicisi vardır. Araç satırına **tek tık** pencereyi açar.
- **Menü:** üst menüde **İşlemler**: İşlem araç kutusu, İşlem geçmişi, **Modeller** (kitaplıktaki modeller ve "Yeni model…"; `'@models'`) ve her kategori için bir alt menü (`'@processing'`); hepsi kayıttan üretilir (`app/menus.ts`).
- **Araç kutusunda Modeller dalı** en üsttedir: her model bir satır (tıklayınca çalıştırma penceresi; kalem düğmesi tasarımcıyı açar, yerleşik modelde kopyasını) ve "Yeni model…".
- **Komutlar:** her araç `processing.run.<id>` komutudur; takma adları komut satırından yazılabilir (`KOSENUMARA`). `processing.toolbox`, `processing.history` ve eski `map.edgeLengths` (Harita menüsü) de komuttur.
- **Pencere** (`ui/processing/ToolDialog.ts`): solda Girdi, Ayarlar, Çıktı ve katlanır "Gelişmiş ayarlar"; sağda kategori, açıklama, yardım, canlı önizleme, çalışma yerleri ve komut satırı takma adları; altta Varsayılanlar, durum, Kapat ve Çalıştır. Hatalar dokunulan alanda anında, Çalıştır'dan sonra hepsi görünür. Enter (metin alanında) ya da Ctrl+Enter çalıştırır. Başarılı çalıştırmada "Sonuçları seç" ve "Geri al" sunulur; pencere açık kalır, değer değiştirip yeniden çalıştırılabilir. Gelişmiş ayarlar, görünen değerlerinden biri varsayılanından farklıysa (son çalıştırmadan ya da geçmişten) açık başlar; seçili olanı yeniden seçmek hiçbir şeyi değiştirmez; çalışırken Varsayılanlar kapalıdır. Kurallar `dialogPlan.ts`, `fieldPlan.ts` ve `dialogTexts.ts`'tedir; `fixtures/processing/v1/dialog.json` onları sabitler; masaüstünün penceresi aynı dosyayı oynatır ([fixtures/processing/README.md](../fixtures/processing/README.md)).
- **Sahneden seç** ([ADR 0088](adr/0088-pick-from-the-scene.md)), hedef ikonlu düğme:
  - **Nokta parametresi:** "Sahneden seç" (nokta varken "Yeniden seç"; yokken alan "Henüz seçilmedi" der) pencereyi kapatır, `PickPointTool` ile tek nokta ister (kenet ve `Y,X` yazımı çalışır; Esc vazgeçer) ve pencereyi olduğu gibi (dokunulan alanlar, çalıştırma denemesi, açık bölümler) noktayla geri getirir.
  - **Bir seçeneği çizimdeki nokta olan seçim:** tanımda `picks: { option, point }` (numaralamanın Başlangıç köşesi: `point`, `startPoint`). Seçimin yanında ikon düğme durur, o seçenek seçiliyken vurgu rengindedir; gösterilen nokta nokta parametresine yazılır ve seçim o seçeneğe geçer. Model tasarımcısında adımın sabit değerleri için de vardır.
  - **Girdi nesneleri:** kapsam düğmelerinin yanında "Sahneden seç". Pencere kenara çekilir, seçim saklanıp boşalır; `PickObjectsTool` alanın türlerinden (tür çipleriyle daraltıldıysa onlar) nesneleri tıklamayla (seçime girer ya da çıkar; imlecin altındaki nesne başka türdense alınan türlerden kenarı en yakın olan, erimde kenar da yoksa içinde tıklanan en küçük alan) ya da kutuyla (soldan sağa pencere, sağdan sola kesişim) seçer; kenet yoktur. Enter, Boşluk ya da hızlı sağ tık: alan Seçili olur, günlüğe "n nesne seçildi." yazılır. Esc ya da hiçbir şey seçmeden bitirmek: pencere olduğu gibi, önceki seçim geri gelir.
- **Geçmiş:** her çalıştırmanın durumu, saati, süresi ve özeti; "Yeniden aç" aynı değerlerle pencereyi açar, "n nesneyi seç" çalıştırmanın eklediği ve hâlâ var olan nesneleri seçip yakınlaştırır.

### 8.1 Durum kapsamları

| Durum | Kapsam | Yer |
|---|---|---|
| Her aracın ve modelin son değerleri, araç başına çalışma yeri seçimi, kullanıcının modelleri | Uygulama ayarı (bu tarayıcı) | `localStorage` `kentos.processing.v1` (`lastValues`, `targets`, `models`) |
| Dok sekmesi, İşlemler görünümü, katlanan kategoriler | Çalışma alanı yerleşimi | `kentos.ui.v1` (`dockTab`, `processingTab`, `processingFolded`) |
| Çalıştırma geçmişi | Oturum | `runner.history` (bellekte, en çok 100) |
| Araçların ürettiği nesneler ve katmanlar | Belge verisi | `CadDocument` (tek geri alma adımı) |

Kayıtlı değerler `restoreValues` ile geri yüklenir: artık uymayan (araç
değişmiş, katman silinmiş) değerler varsayılana döner.

## 9. Yeni işlem aracı tarifi

1. Geometriyi Rust çekirdeğine yazın ve test edin (`crates/shared/geometry-core`; araca özgüyse `apps/web/src/processing/` altına, bir depo sorgusu ve `ObjectStore`'da bir yöntemle); TS'te metin ve akış kalır. Taşıma yöntemi ADR 0008'dedir.
2. `processing/builtin/<ad>.ts` içinde `defineTool({...})` ile tanımı yazın: kimlik, etiket, kategori, simge, açıklama, yardım, anahtar kelimeler, takma adlar, `targets`, `parameters` (`as const`), `outputs`, gerekirse `validate` ve `preview`, `run`.
3. `processing/builtin/index.ts` içindeki `BUILTIN_TOOLS` listesine ekleyin. Kategori yoksa `categories.ts`'e ekleyin.
4. Simge yoksa `ui/icons.ts`'e çizin (DESIGN.md §6).
5. `processing.test.ts`'e (ya da aracın yanına `*.test.ts`) saf çekirdek ve `ProcessingRunner` üzerinde belgeyle bir test ekleyin: değişiklik, tek geri alma adımı, sınır durumları. Yerleşik araç ve model `fixtures/processing/v1/cases.json`'a en az bir başarılı ve bir ret durumuyla girer; masaüstü (`kentos-processing`) aynı durumları oynatır ([fixtures/processing/README.md](../fixtures/processing/README.md)).
6. Yeni bir kullanıcı akışıysa `apps/web/scripts/e2e/smoke.mjs`'e bir kontrol ekleyin.

Pencere, araç kutusu satırı, menü öğesi, komut ve takma adlar kendiliğinden
oluşur. Arayüz kodu yazmak gerekmez; gerekiyorsa bu, yeni bir parametre
türünün işaretidir (bkz. §10).

## 10. Genişletme noktaları

- **Yeni parametre türü:** `types.ts` (tanım + `ValueOf` + gerekirse `ResolvedOf`), `parameters.ts` (`defaultValue`, `fits`, `checkParam`), `runner.ts` (çözme), `ui/processing/paramFields.ts` (kontrol), `dialogPlan.ts` ve `fieldPlan.ts` (form ve görünüş); masaüstünde `kentos_processing::types`, `parameters`, `web_param` ve `apps/desktop/src/processing/fields.rs`. Planlananlar: CRS, mesafe (birimli), renk.
- **Yeni çalışma yeri:** bir `Executor` yazıp `app/processing.ts` içindeki `createExecutors` listesine ekleyin. İşi `RunJob` olarak alır; `materialize` ve `jobContext` ile aracı çalıştırır ya da işi uzağa gönderir.
- **Eklenti araçları:** `registry.register(tool)` bir `Disposable` döndürür; eklenti kaldırılınca araç ve menü öğeleri kaybolur (`version` sinyali).

## 11. Yerleşik araçlar

| Kimlik | Ad | Ne yapar |
|---|---|---|
| `builtin.parcelSheet` (model) | Parsel ölçü yazıları | Üç adım: köşeleri numaralar (önek bir model girdisi), kenar uzunluklarını yazar, hesaplanan alanı "Hesap alanı" alanına yazar. Tek geri alma adımı. |
| `points.numberVertices` | Köşe noktalarını numarala | Alanların (ve çoklu çizgilerin) köşelerine biçimli numaralı nokta ve/veya yazı koyar. Biçim: önek + doldurma karakteri + sayı, toplam uzunluk sabit (`P` + `00001` = 6). Yön saat yönünde ya da tersine; başlangıç kuzeybatı, en kuzey, ilk çizilen ya da gösterilen noktaya en yakın köşe. Delikli alanlarda önce dış halka. Komşu alanların ortak köşesi tek numara alır (tolerans ayarlı); hedef katmandaki aynı biçimli numaralar korunur ve numara kaldığı yerden devam eder. |
| `attributes.calculate` | Öznitelik hesapla | Seçilen alana (var olan ya da yeni) her nesne için bir ifadenin değerini yazar; varsayılan `metin($alan, <proje alan hassasiyeti>)`. İsteğe bağlı koşulla yalnızca bazı nesnelere yazar; sonuç boşsa alana dokunmaz ya da boşaltır. Etiket alanın eski değerini gösteriyorsa yeni değeri gösterir. Tek geri alma adımı. |
| `selection.byExpression` | İfadeyle seç | Koşulu sağlayan nesneleri seçer: yeni seçim, seçime ekle, seçimden çıkar ya da seçim içinde ara. Belgeyi değiştirmez. |
| `annotation.edgeLengths` | Kenar uzunluklarını yaz | Alan, çoklu çizgi ve çizgilerin her kenarına uzunluğunu, kenar ortasına ve okunur açıyla, dışa ya da içe yazar. Yay kenarında yay boyu yazılır. Ortak kenarlar bir kez yazılır; ondalık basamak varsayılanı proje ayarından gelir; önek, sonek ve en kısa kenar süzgeci gelişmiş ayarlardadır. |
| `selection.byLocation` | Konuma göre seç | Başvuru nesnelerinden biriyle (Ayrık: hiçbiriyle) ilişkisi olan nesneleri seçer: Kesişen, İçeren, İçinde kalan, Ayrık, Uzaklıkta, Merkezi içinde (ADR 0200 §1–§2; ilişkiler çekirdeğin `ops::spatial_query`'si). Seçim biçimleri İfadeyle seç'inki. |
| `attributes.fromInside` | İçindekinden bilgi al | Her alana içindeki (ya da ona değen, merkezi içindeki) kaynak nesnelerin sayısını ya da bir alanlarının toplamını, ortalamasını, en azını, en çoğunu ya da ilk değerini yazar (sayıların kuralı `kentos.statistics/1`, ADR 0200 §4). |
| `attributes.fromEnclosing` | Çevreleyenden bilgi al | Her nesneye merkezinin içinde kaldığı alanın bir alanını yazar; birden çok alan: çizim sırasıyla ilki, hiçbiri: değişmez; ikisi de söylenir. |
| `statistics.summary` | Özet istatistik | Bir alanın sayı, toplam, ortalama, en az, en çok ve standart sapmasını, isteğe bağlı bir alana göre gruplayarak tablo olarak verir; çizimi değiştirmez. |
| `attributes.joinByField` | Anahtarla birleştir | Ortak anahtarla başka katmandan ya da CSV, TXT, XLSX dosyasından alan aktarır (sayı anahtarlar sayı olarak; kaynakta tekrarlanan anahtarın ilk satırı); önek, Üzerine yaz ya da Yalnız boşlara. |
| `geometry.buffer` | Tampon | Nesnelerin çevresinde uzaklıktaki alan: noktada daire, çizgide kapsüller, alanda dışa ya da eksi uzaklıkla içe; Sol, Sağ (tek yanlı, uçları düz), Halka sayısı (k. halka (k − 1)·d ile k·d arası; “Uzaklık”, “Halka”), Birleştir, Uzaklık alanı. Yaylar kesin (ADR 0201 §2). |
| `geometry.clip` | Kırp | Nesnelerin kesen alanların (birleşmiş) içinde ya da sınırında kalan kısmı, öznitelikleriyle (§3). |
| `geometry.dissolve` | Gruplayarak birleştir | Bir alana göre grup grup birleştirme: alanların ortak sınırları kalkar, çizgi ve noktalar grubun çok parçalı nesnesi; “Nesne sayısı” ve Toplanacak alanların kesin toplamları; Çok parçalı ya da parça parça (§4). |
| `geometry.intersection` | Kesişim | Her nesnenin kestiği her alanla parçası, iki tarafın öznitelikleri (Önek, aynı ad “ad (2)”); Alan oranıyla paylaştır (§5). |
| `geometry.difference` | Fark | Nesnelerden çıkarılacak alanların kapladığı kısım atılır (§5). |
| `geometry.symDifference` | Simetrik fark | İki alan kümesinin yalnız birinin kapladığı parçalar, kendi taraflarının öznitelikleriyle (§5). |
| `geometry.union` | Birleşim | Ortak parçalar iki tarafın, kalanlar kendi tarafının öznitelikleriyle; Alan oranıyla paylaştır (§5). |
| `geometry.validity` | Geçerliliği denetle | Yinelenen köşe, alanı sıfır halka, kendini kesen ya da kendine değen halka ve yol, taşan ve örtüşen delik; tablo (Nesne, Katman, Sorun, Doğu, Kuzey) ve sorunlular seçilir; çizim değişmez (§6). |
| `geometry.repair` | Onar | Alan kendi halkalarından yeniden kurulur, yinelenen köşeler düşer; sorunsuz nesne olduğu gibi; rapor tablosu (§6). |
| `geometry.simplify` | Sadeleştir | Çizimin Sadeleştir kuralı yeni katmana; köşe, alan değişimi ve en büyük sapma tablosu (§7). |
| `geometry.reproject` | Koordinat sistemine dönüştür | Kayıttaki bir sistemden projenin sistemine, projenin datum seçimleriyle; yay ve daireler 1 mm içinde doğru parçaları (§8). |

## 12. Masaüstü

Masaüstü aynı araçları kendi çekirdeğiyle çalıştırır ([ADR 0084](adr/0084-desktop-processing.md)): `crates/native/processing` (`kentos-processing`) bu belgenin sözleşmesinin yerli karşılığıdır (parametreler, kapsamlar, çalıştırıcı, modeller, yerleşik araçlar); `fixtures/processing/v1`'in bütün durumlarını geçer. Pencere, komutlar ve takma adlar `apps/desktop/src/processing/`'dedir; her aracın son değerleri `$XDG_STATE_HOME/kentos-cad/islemler.json`'dadır. Yeni bir yerleşik araç iki platformda birlikte ve ortak durumlarıyla eklenir.

Çalıştırıcı web'in `RunJob` modeliyle üç adımdır ([ADR 0124](adr/0124-desktop-processing-background.md)): `prepare` girdileri ve katmanları sahipli bir işe çözer, `compute` aracın hesabıdır (çizimi yalnız okur, her iş parçacığında çalışır), `finish` sonucu tek geri alma adımında yazar. Masaüstünün "Nerede çalışır"ı web'in planıdır: Otomatik, 2 000 ve daha çok nesneli işi arka plandaki bir iş parçacığına gönderir (`apps/desktop/src/processing/background.rs`); hesap çizimin okuma kopyasında (`Document::reading_copy`, nesneler paylaşılır) yapılır, pencere ilerlemeyi gösterir, Durdur işi hemen bitirir. Bir model arka planda bütünüyle kopyada çalışır, adımları kaydedilir (`record_model`) ve çizimde, kopyada gittikleri gibi tek geri alma adımında yeniden oynatılır (`replay_model`); çizim arada değiştiyse model çizimin şimdiki hâlinde burada yeniden çalışır ([ADR 0125](adr/0125-desktop-models-background.md)).
