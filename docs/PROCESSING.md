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
  ProcessingPanel.ts Sağ doktaki araç kutusu (Modeller dalı dahil) ve geçmiş
  model/             Model tasarımcısı: ModelDesigner, ModelCanvas, modelPalette, modelInspector
apps/web/src/tools/pickPointTool.ts    Nokta parametresi için "Haritadan göster"
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
| `features` | `{ scope: 'selection' \| 'visible' \| 'all' }`, `{ scope: 'layer', layerId }`, `{ scope: 'ids', ids }` | `FeatureSet { entities, description }` | `kinds` (uygun nesne türleri), `scopes` (sunulan kapsamlar) |
| `number` | `number` | aynı | `min`, `max`, `integer`, `unit` |
| `string` | `string` | aynı | `placeholder`, `maxLength`, `allowEmpty` |
| `boolean` | `boolean` | aynı | — |
| `enum` | seçenek değeri (dar tip) | aynı | `options: { value, label, hint? }[]` |
| `layer` | `{ layerId }` ya da `{ newName }` | `TargetLayer { id, name, isNew }` | `newLayerStyle` |
| `point` | `Vec2 \| null` | aynı | — |
| `expression` | ifade metni | `CompiledExpression` (isteğe bağlı ve boşsa `null`) | `returns: 'condition' \| 'value'`, `of` (okuduğu `features` parametresi), `placeholder` |
| `field` | alan adı | aynı (kırpılmış) | `of` (alanları sunulan `features` parametresi), `allowNew` (yeni alan adı yazılabilir) |

Ortak alanlar: `name` (değer anahtarı), `label`, `description`, `optional`,
`advanced` ("Gelişmiş ayarlar" altında), `visibleWhen` (yalnızca koşul
sağlanınca gösterilir ve denetlenir), `default` (sabit ya da
`(c: DefaultsContext) => …` ile proje ayarından; ör. ondalık basamak).

- **Zorunluluk:** parametreler varsayılan olarak zorunludur. `optional: true` olan boş (`null`) bırakılabilir ve pencerede "isteğe bağlı" yazar. Metinde boş değer ayrıca `allowEmpty` ister (boş önek gibi).
- **Nesne türü süzgeci:** `features` değeri isteğe bağlı `kinds` taşır. Kapsamda aracın alabildiği iki ya da daha çok tür varsa pencere her tür için sayılı bir düğme gösterir ("Kapalı alan 118"); kullanıcı bu çalıştırmada yalnızca bazı türleri alabilir (örneğin yalnızca kapalı alanların kenarlarını yazmak). Hiç tür kalmazsa araç çalışmaz.
- **Kapsamlar:** "Seçili" seçimdeki nesneler; "Görünen" kutusu ekrandaki görünür alanla kesişen, görünür katmanlardaki nesneler (yardımcı çizgiler hariç; kutu testini görünümün geometri deposu yapar); "Tümü" görünür katmanlardaki bütün nesneler; "Katman" bir katman ya da grubun altındaki bütün katmanlar (gizli olsa bile); "ids" modellerde önceki adımın çıktısıdır ve pencerede sunulmaz. `kinds` dışındaki nesneler sessizce elenir; pencere ne kadar nesne okunacağını canlı gösterir.
- **Boş girdi:** zorunlu bir `features` parametresi hiç nesneye çözülmezse çalıştırıcı aracı çalıştırmaz ve alanın altına yönlendiren bir mesaj yazar ("Önce nesneleri seçin ya da kapsamı değiştirin"). Model içinde (`ids`) boş çıktı hata değildir.
- **Hedef katman:** `{ newName }` aynı adlı bir katman varsa onu kullanır (araç ikinci kez çalışınca aynı "Köşe noktaları" katmanına yazar); yoksa katman yalnızca araç gerçekten ona yazarsa oluşturulur. Kilitli katman seçilemez; kilitli katmana düşen değişiklikler atlanır ve sayısı bildirilir.

### 4.3 Çalışma bağlamı ve değişiklik kümesi

```ts
run(values: ResolvedValues<Ds>, ctx: RunContext, feedback: Feedback): RunResult | Promise<RunResult>

RunContext { doc: DocumentSnapshot /* get, all, byLayer; salt okunur */, units: DefaultsContext,
             layerName(id): string, selection: readonly number[] /* çalıştırma başındaki seçim */,
             geometry: RunGeometry /* girdilerin geometrisi, kimlikten: measures, numberCorners, cornerTexts, edgeLengths */ }
Feedback   { progress(fraction, label?), info(m), warn(m), canceled, yield() }
ChangeSet  { add?: NewEntity[], update?: { id, patch }[], remove?: number[] }
RunResult  { changes?, select?: readonly number[] /* çalıştırmadan sonraki seçim */, outputs?, summary? }
```

- Uzun döngülerde `await feedback.yield()` sayfanın donmasını önler (16 ms'de bir gerçekten bekler) ve `feedback.canceled` denetlenir. İptal edilen çalıştırmanın değişiklikleri uygulanmaz.
- `ctx.units.plotScale` kâğıt ölçüsünü dünyaya çevirir: 2 mm yazı, 1:1000'de 2 m'dir.
- Araç `NewEntity` üretirken `layerId` olarak hedef katmanın `id`'sini kullanır; yeni katman o anda henüz yoktur, çalıştırıcı uygularken kurar.
- **Seçim üreten araçlar** (İfadeyle seç) belgeyi değiştirmez, `select` döndürür; çalıştırıcı seçimi uygular (`FeatureHost.select`). Geri alınacak bir şey yoktur. Mevcut seçimle birleştirme (ekle, çıkar, içinde ara) aracın işidir, `ctx.selection` ile yapılır.
- **Geometri çekirdekten gelir** (`ctx.geometry`, `processing/geometry.ts`): çalıştırma, features girdilerinin nesnelerini kendi geometri deposuna paketler (ilk soruda; bitince bırakılır). Araç kimlikle sorar: ifadelerin geometri değerleri (`measures`, `measuredOf` ile bütün nesneler için bir kez), köşe numaralama (`numberCorners`: her köşenin yeri, dışa bakan yönü ve kimin numarasını aldığı; adları `nameCorners` verir), köşe yazısının yeri (`cornerTexts`), kenar ölçüsü yazıları ve ortak kenar testi (`edgeLengths`).
- **Öznitelik değiştiren araçlar** `update` içinde `attrs` alanının tamamını verir (`{ ...e.attrs, [alan]: değer }`); yalnızca öznitelik değişirse belge `attrs` olayı yayar ve GPU tamponu kurulmaz.
- **`features` çıktıları:** `outputs[ad]` bir kimlik dizisiyse o kullanılır (seçilenler, değişenler); yoksa çalıştırmanın eklediği nesneler çıktıdır. Modeller bu kimlikleri sonraki adıma `{ scope: 'ids' }` olarak verir.

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

Koşul ve değer parametreleri (`expression`) küçük, güvenli bir ifade dili kullanır (`model/expression/expression.ts`, `expressionLib.ts`; stil motoru da aynı dili kullanır). `eval` yoktur: metin sözcüklere ayrılır, öncelik tırmanmasıyla ayrıştırılır ve closure'lara derlenir. Hata mesajı yerini söyler: "15. karakterde: İfade yarım kalmış: sonunda bir değer eksik."

```
Nitelik = 'Arsa' ve $alan > 500
'P' || doldur($sıra, 5)
yuvarla([Tapu alanı (m²)] - $alan, 2)
eğer(boş(Parsel), 'numarasız', Ada || '/' || Parsel)
```

- **Alanlar:** düz ad (`Parsel`) ya da boşluk ve işaret içerenler için köşeli parantez (`[Tapu alanı (m²)]`). Olmayan alan boş (`null`) verir; pencere ifadenin okuduğu ama nesnelerde olmayan alanları önizlemede söyler.
- **Değerler:** sayı (ondalık ayırıcı nokta), metin (`'…'` ya da `"…"`, içte çift tırnak bir tırnaktır), `doğru`/`true`, `yanlış`/`false`, `boş`/`null`.
- **İşleçler:** `ve`/`and`, `veya`/`or`, `değil`/`not`; `= != <> < <= > >=`; `+ - * / %`; `||` metin birleştirir. `+` iki taraf da sayıysa toplar, değilse birleştirir.
- **Tür kuralları:** öznitelikler metindir; aritmetik ve karşılaştırma metindeki sayıyı okur ("472.27" → 472.27). Boş bir değerle aritmetik boş verir, karşılaştırma yanlış verir; `= boş` yalnızca boş için doğrudur. Sıfıra bölme boştur. Metin karşılaştırması Türkçe sıralamayla ve büyük/küçük harfe duyarlıdır; `içerir`, `başlar`, `biter` harf farkı gözetmez.
- **Değişkenler:** `$alan`, `$uzunluk` (`$çevre`), `$köşe`, `$tür`, `$katman`, `$etiket`, `$y` (sağa), `$x` (yukarı), `$sıra` (bu çalıştırmadaki sıra, 1'den), `$id`; yalnızca sembol çizilirken `$ölçek` (çizim ölçeğinin paydası; işlem araçlarında boştur).
- **İşlevler** (Türkçe adı ve QGIS'teki İngilizce adıyla): `yuvarla/round`, `metin/to_string` (sabit ondalık), `sayı/to_real`, `tamsayı/int`, `mutlak/abs`, `min`, `max`, `büyük/upper`, `küçük/lower`, `kırp/trim`, `uzunluk/length`, `parça/substr`, `doldur/lpad`, `değiştir/replace`, `içerir/contains`, `başlar/starts_with`, `biter/ends_with`, `eğer/if`, `boş/is_empty`, `varsayılan/coalesce`.
- **Adlar Türkçe harf farkı gözetmez:** `YUVARLA` = `yuvarla`, `$cevre` = `$çevre`, `DEGIL` = `değil`.
- Yazılan değer metne `toText` ile çevrilir: tam sayılar ondalıksız, ondalıklar kayan nokta gürültüsü atılarak (0.1 + 0.2 → "0.3"), doğru/yanlış olarak.

Pencerede ifade alanı tek satırdır (komut satırı gibi eşaralıklı yazıyla). Altında girdi nesnelerinin alanları düğme olarak (tıklayınca imlecin yerine eklenir), "Değişkenler" ve "İşlevler" menüleri (her biri ne yaptığını söyler) ve canlı bir satır bulunur: koşulda "16 / 340 nesne koşulu sağlıyor.", değerde "İlk nesnede (10): “472.26”."

Yeni işlev eklemek için `EXPR_FUNCTIONS` listesine ad, İngilizce karşılık, değer sayısı, kullanım, açıklama ve `call` ekleyin ve `model/expression/expression.test.ts`'e bir satır yazın. Menü ve belge buradan beslenir.

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

- **Seçim:** kullanıcı pencerenin sağ panelindeki "Nerede çalışır" listesinden seçer ve seçim araç başına hatırlanır (`kentos.processing.v1`). **Otomatik** (varsayılan), girdiler `WORKER_THRESHOLD` (2 000) nesne ve üstündeyse worker'ı, değilse aracın ilk tercihini kullanır; pencere o anki kararı yazar ("şimdi: bu tarayıcıda"). Aracın bildirdiği ama bu ortamda olmayan yerler "yakında" diye, seçilemez olarak listelenir.
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

`ui/processing/model/`: `ModelDesigner` (pencere, taslak, kendi geri alma yığını, kaydetme), `ModelCanvas` (diyagram), `modelPalette` (sol), `modelInspector` (sağ).

- **Sol:** "Girdi ekle" (Nesneler, Sayı, Metin, Evet/hayır, Katman, Nokta) ve aranabilir araç listesi. Bir araca tıklamak seçili kutunun sağına ekler ve ilk uygun girdisini seçili kutuya bağlar; tuvale sürüklemek bırakılan yere koyar.
- **Orta:** girdiler (mavi kenarlı) ve adımlar kutu, bağlantılar eğridir; eğri ortasında hangi parametreyi beslediği yazar. Kutular sürüklenir (10 px ızgaraya), boş alan sürüklenince tuval kayar, tekerlek yakınlaştırır, çift tık hepsini gösterir. Bir kutunun sağındaki noktadan sürükleyip bir adımın üstüne bırakmak, o adımın uygun girdilerini (ve adımın çıktılarını) bir menüde sorar. Sorunlu adım kesik turuncu kenarla ve ilk sorunuyla görünür.
- **Sağ:** hiçbir şey seçili değilken modelin adı, kategorisi, açıklaması, çıktıları ve sorunları; girdi seçiliyken etiketi, açıklaması, isteğe bağlılığı ve türüne göre varsayılanı; adım seçiliyken başlığı ve her parametre için **kaynak** (aracın varsayılanı, sabit değer, girdi, önceki adımın çıktısı ya da "Yeni model girdisi yap"). Sabit değer, araç penceresindeki denetimin aynısıyla girilir.
- **Alt:** "Düzenle" (sütunlara diz), durum (sorun sayısı ve ilki ya da "Model çalışmaya hazır"), Kapat, "Kaydet ve çalıştır…", Kaydet. Kaydedilmemiş değişiklikle kapatmak alt çubukta sorar. Ctrl+Z / Ctrl+Y taslakta geri alır, Ctrl+S kaydeder, Delete seçili kutuyu siler. Sorunlu model kaydedilebilir ama çalışmaz.

## 8. Arayüz

- **Araç kutusu:** sağ dokun üst yuvasında "Katmanlar | İşlemler" sekmeleri. İşlemler sekmesinde arama (Türkçe harfler katlanır: "kose" = "köşe"), kategori ağacı (katlama durumu `ui.processingFolded`) ve "Araçlar | Geçmiş" seçicisi vardır. Araç satırına **tek tık** pencereyi açar.
- **Menü:** üst menüde **İşlemler**: İşlem araç kutusu, İşlem geçmişi, **Modeller** (kitaplıktaki modeller ve "Yeni model…"; `'@models'`) ve her kategori için bir alt menü (`'@processing'`); hepsi kayıttan üretilir (`app/menus.ts`).
- **Araç kutusunda Modeller dalı** en üsttedir: her model bir satır (tıklayınca çalıştırma penceresi; kalem düğmesi tasarımcıyı açar, yerleşik modelde kopyasını) ve "Yeni model…".
- **Komutlar:** her araç `processing.run.<id>` komutudur; takma adları komut satırından yazılabilir (`KOSENUMARA`). `processing.toolbox`, `processing.history` ve eski `map.edgeLengths` (Harita menüsü) de komuttur.
- **Pencere** (`ui/processing/ToolDialog.ts`): solda Girdi, Ayarlar, Çıktı ve katlanır "Gelişmiş ayarlar"; sağda kategori, açıklama, yardım, canlı önizleme, çalışma yerleri ve komut satırı takma adları; altta Varsayılanlar, durum, Kapat ve Çalıştır. Hatalar dokunulan alanda anında, Çalıştır'dan sonra hepsi görünür. Enter (metin alanında) ya da Ctrl+Enter çalıştırır. Başarılı çalıştırmada "Sonuçları seç" ve "Geri al" sunulur; pencere açık kalır, değer değiştirip yeniden çalıştırılabilir.
- **Nokta parametresi:** "Haritadan göster" pencereyi kapatır, `PickPointTool` ile tek nokta ister (kenet ve `Y,X` yazımı çalışır; Esc vazgeçer) ve pencereyi değerlerle yeniden açar.
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
5. `processing.test.ts`'e (ya da aracın yanına `*.test.ts`) saf çekirdek ve `ProcessingRunner` üzerinde belgeyle bir test ekleyin: değişiklik, tek geri alma adımı, sınır durumları.
6. Yeni bir kullanıcı akışıysa `apps/web/scripts/e2e/smoke.mjs`'e bir kontrol ekleyin.

Pencere, araç kutusu satırı, menü öğesi, komut ve takma adlar kendiliğinden
oluşur. Arayüz kodu yazmak gerekmez; gerekiyorsa bu, yeni bir parametre
türünün işaretidir (bkz. §10).

## 10. Genişletme noktaları

- **Yeni parametre türü:** `types.ts` (tanım + `ValueOf` + gerekirse `ResolvedOf`), `parameters.ts` (`defaultValue`, `fits`, `checkParam`), `runner.ts` (çözme), `ui/processing/paramFields.ts` (kontrol). Planlananlar: çoklu seçim, dosya, CRS, mesafe (birimli), renk, tablo (satır listesi).
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
