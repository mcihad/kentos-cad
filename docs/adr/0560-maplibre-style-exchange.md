# ADR 0560: Görünüşün MapLibre GL stili olarak alışverişi

- **Durum:** kabul edildi (2026-10-20). `plan/gis.md` CBS-17'nin üçüncü dilimi (ADR 0516 ve 0555'in “sonraya kalan”ındaki
  MapLibre profili). Kapsamı ben belirledim.
- **Bağlam belgesi:** ADR 0516 (SLD: kayıpların söylenmesi, komutların biçimi, süzgeç dilbilgisi), 0555 (QML: aynı
  çekirdeğin ikinci biçimi), 0208 (servis katmanları; `kentos-services` vektör karoları MapLibre stiliyle çizer), 0213
  (`cad.layers.renderer`), 0212 (etiket motoru), 0312 §5 (kâğıdın renkleri), 0100 (ifade dili).
- **Web:** ertelendi (ADR 0228). Web'de gerekenler TODOS.md §0.2'de.
- **Sonraya kalan:**
  - sprite ve simgeler (`icon-image`): daire dışındaki işaret şekilleri, SVG ve yazı işareti;
  - tarama, degrade ve desen dolgusu (`fill-pattern` bir sprite ister);
  - `case` ve öbür ifadeler (veriye bağlı değerler KentOS'un ifadesine çevrilerek);
  - etiketlerin içe alınması (`symbol` katmanları) ve etiketlerin çağrı çizgisi, zemini, gölgesi;
  - `fill-extrusion`, `heatmap`, `raster`, `hillshade` katmanları ve `terrain`;
  - MapLibre'nin `sources`'ından KentOS katmanı açmak (bağlı kaynak olarak; ADR 0208).

## Bağlam

SLD ve QML masaüstü GIS'lerin dilidir. Haritayı web'e koyan, MapLibre GL JS ile gösteren, Maputnik'le düzenleyen ya da
vektör karo sunan biri görünüşü MapLibre'nin stil belirtimiyle (sürüm 8) taşır. Bu biçim ötekilerden üç yönden ayrılır:
- stil bir **haritanın** stilidir: kaynaklar (`sources`) ve onların üstünde sıralı katmanlar; tek bir katmanın değil;
- değerler **ifadedir** (`match`, `step`, `interpolate`, `["get", alan]`); kategorili ya da aralıklı görünüş sınıf listesi
  değil, boya özelliğindeki bir ifadedir;
- boylar kâğıt milimetresi değil CSS pikselidir; metre yoktur.

Plan, dış stilin desteklenen alt kümesinin açık olmasını ve çevrilemeyenin söylenmesini ister.

## Karar

### 1. Stilin biçimi

`kentos_formats::maplibre` stil sürüm 8'i JSON olarak yazar ve okur.

```json
{ "version": 8, "name": "…", "metadata": { "kentos:format": "kentos-maplibre/1" },
  "sources": { "<katman kimliği>": { "type": "geojson", "data": "<ad>.geojson" } },
  "glyphs": "…", "layers": [ … ] }
```

- Her stil katmanının `id`'si `<KentOS katman kimliği>/<sıra>`, `metadata`'sı:
  - `kentos:layer`: KentOS katmanının kimliği (içe alırken eşleme bununla başlar);
  - `kentos:symbol`: `fill`, `line` ya da `marker` (hangi simgenin katmanı; dolgunun kenar çizgisi `fill`);
  - `kentos:class`: sınıfın ya da kuralın adı (sınıf başına katmanda);
  - `kentos:labels`: birleşik katmanda sınıfların adları (kategorilide değer → ad, aralıklıda sırayla).
- **Katmanların sırası:** katman ağacı üstteki katmanı önce sayar, MapLibre'nin ilk katmanı en alttadır; ağaç ters
  çevrilerek yazılır.
- **Kaynak:** GeoJSON dosyası yanında yazılırsa her KentOS katmanı kendi kaynağıdır (kaynağın kimliği katmanın kimliği);
  adres verilirse vektör karo kaynağıdır (`kentos`), her katmanın adı `source-layer` olur (§9).
- **Katman türleri:** alan simgesi `fill` (+ kenar çizgisi için `line`), çizgi simgesi `line`, nokta simgesi `circle`,
  etiket `symbol`. Bir katmanın nesneleri birden çok geometri türündeyse her stil katmanı kendi türünü
  `["==", ["geometry-type"], "Polygon" | "LineString" | "Point"]` ile söyler (MapLibre çember katmanını çokgenin
  köşelerine de çizer; çizgi katmanı çokgenin kenarlarını da çizer).
- **Boş katmanlar:** bütün görünen katmanlar yazılırken nokta, çizgi ya da alanı olmayan katman (yalnız kayıt, yazı,
  ölçü ya da hiçbir şey) yazılmaz ve adı söylenir. Tek katman istenirse boşsa da yazılır.

### 2. Görünüşler

| KentOS | MapLibre |
|---|---|
| tek simge | sabit boya değerleri |
| kategorili (alanın değerleri; Diğerleri) | `["match", ["to-string", ["get", alan]], değer, çıktı, …, yedek]` |
| aralıklı (`alt ≤ değer < üst`, sonuncu `≤ üst`) | `["step", ["to-number", ["get", alan]], ilk, eşik, çıktı, …]` + katmanın süzgeci |
| kurallı (iç içe, Diğerleri, ölçek aralığı) | her kural süzgeçli katman |

**Birleşik ve sınıf başına yazım.** Kategorili ya da aralıklı görünümün sınıfları aynı yapıdaysa her **simge katmanı**
için tek stil katmanı yazılır ve değişen boya değerleri `match`/`step` olur. Aynı yapı:
- her sınıfta aynı sayıda ve aynı türde (`fill`, `line`, `circle`) simge katmanı;
- aynı `layout` (çizgi ucu, köşesi) ve aynı kesik dizisi (`line-dasharray` veriye bağlı olamaz);
- hiçbirinde metre genişliği yok (yakınlaştırma ifadesi, veri ifadesinin çıktısı olamaz);
- kategorilide: değerler birbirinden ayrı, kapalı sınıf yok; aralıklıda: sınıflar artan, bitişik (`üst = sonraki alt`),
  hepsi dolu.
Değilse her sınıf kendi katmanlarıyla, kendi süzgeciyle yazılır (kayıp değildir). Birleşik katmanın süzgeci:
- kategorilide Diğerleri yoksa değerlerin üyeliği (`["match", metin, [değerler], true, false]`); Diğerleri varsa süzgeç yok;
- aralıklıda alanın var olması ve `[ilk alt, son üst]` aralığı (aralığın dışı çizilmez, `step`'in kenar çıktıları sızmaz).

**Süzgeçli yazımda:**
- iç içe kuralın süzgeci üst kuralınkiyle `all`'dır;
- Diğerleri kuralı kardeşlerinden öncekilerin hiçbiri olmadığında çizilir (`!` + `any`); önceki bir kardeş her
  nesneyi alıyorsa Diğerleri yazılmaz;
- kapalı kural ve sınıf yazılmaz;
- kategorilide kapalı sınıfın değeri Diğerleri'nden de dışlanır.

**Ölçek.** Kuralın `minScale`/`maxScale`'i (`min` yakın, `max` uzak) `maxzoom`/`minzoom` olur: `yakınlaştırma =
log2(279 541 132,0144 / payda)`; payda OGC'nin 0,28 mm'lik standart pikselinde, ekvatorda, 512 pikselli karoda zoom 0'ın
paydasıdır (SLD'nin kullandığı piksel; ADR 0516). Yaklaşıktır ve söylenir. Okurken aynı formül tersiyle ve paydalar tam
sayıya yuvarlanır.

### 3. Simgeler

| KentOS | MapLibre | Taşınanlar |
|---|---|---|
| `simpleFill` | `fill` | `fill-color`, `fill-opacity` |
| `simpleLine` | `line` | `line-color`, `line-width`, `line-opacity`, `line-dasharray`, `line-cap`, `line-join` |
| `shape` işaret (daire, halka) | `circle` | `circle-radius`, `circle-color`, `circle-opacity`, `circle-stroke-color`, `circle-stroke-width`, `circle-stroke-opacity` |

- Renk `#rrggbb`, saydamsa `rgba(r,g,b,a)`; tema renkleri (`fg`, `fg-dim`, `ink`, `paper`, `label`) paftanın
  renkleridir (`print_color`, ADR 0312 §5): bir harita açık zeminde okunur.
- Simgenin ve katmanın saydamlığı (Katman çizimi'nin saydamlığı) renklerin yanında `*-opacity`'ye çarpılır. Katmanın
  karışım kipi ve maskesi yazılmaz.
- **Kenar çizgisi:** KentOS dolgunun kenarını ayrı çizgi katmanı çizer; MapLibre'de de ayrı `line` katmanıdır
  (`kentos:symbol: fill`); MapLibre'nin `fill-outline-color`'ı kullanılmaz.
- **Kıl çizgi** (kalınlık 0) 1 pikseldir.
- **Kesikler** MapLibre'de çizginin kalınlığının katıdır: `kesik / kalınlık`. Kalınlık sıfırsa ya da birim metreyse
  sürekli çizgi yazılır ve söylenir.
- **Daire:** `size` çaptır, `circle-radius` yarısı. Halka dolgusuz dairedir. Dolgusuz işarette `circle-opacity` 0'dır.
- Yazılmayanlar, adıyla söylenir:
  - daire ve halka dışındaki şekiller (daire yazılır): MapLibre `circle` katmanında yok, `symbol` ise sprite ister;
  - tarama, degrade, desen ve resim dolgusu, işaretli çizgi, geometri üretici, merkez işareti, SVG ve yazı işareti;
  - veriye göre değer (yedek değeri yazılır), efektler;
  - çizginin paralel kaydırması, dalgası, yumuşak kenarı, gölgesi, kesik başlangıcı, halka seçimi;
  - işaretin kaydırması, deliği, yüksekliği;
  - kitaplıkta olmayan simge.

### 4. Birimler

| KentOS | MapLibre |
|---|---|
| mm (kâğıt) | CSS pikseli, **96 dpi**: 1 mm = 96/25,4 px |
| piksel | piksel |
| m | `["interpolate", ["exponential", 2], ["zoom"], 0, w₀, 24, w₀·2²⁴]`, w₀ = m / 78 271,517 (ekvatorda, 512 pikselli karoda zoom 0'da bir pikselin metresi) |

- Üstel taban 2 arası iki durak 2^zoom'u tam verir: genişlik her yakınlaştırmada metrenin pikseli olur.
- Metre ekvatordaki ölçüdür; MapLibre Mercator'dur, enlem arttıkça yer metreyi aşar: söylenir.
- Okurken piksel piksel (ekran pikseli) kalır, metre ifadesi (iki durak, 2^zoom'a uyan) metre olur; başka zoom eğrisi
  (doğrusal ya da başka tabanlı) söylenir ve ilk durağının değeri alınır. mm piksele giderse milimetreye geri dönmez:
  KentOS'un ekran pikseli ile kâğıt milimetresi ayrı birimlerdir, MapLibre'nin yalnız pikseli vardır.

### 5. Süzgeçler

Kural süzgeci KentOS'un ifadesi (`[Ad] = 'x'`), SLD'nin ve QML'in paylaştığı süzgeç dilbilgisinden (`sld::Filter`)
MapLibre'nin ifadesine çevrilir. Alanın değeri `["get", alan]`'dır; metin `to-string`, sayı `to-number` sarar.

| Süzgeç | MapLibre |
|---|---|
| `[a] = 3`, `<>`, `<`, `<=`, `>`, `>=` | `["all", ["has", a], [op, ["to-number", ["get", a]], 3]]` (metinde `to-string`) |
| `[a] arasında 1 ve 5` | `["all", ["has", a], [">=", sayı, 1], ["<=", sayı, 5]]` |
| `[a] boş` | `["any", ["!", ["has", a]], ["==", ["to-string", ["get", a]], ""]]` |
| `[a] boş değil` | `["all", ["has", a], ["!=", ["to-string", ["get", a]], ""]]` |
| `[a] gibi 'x'` / `'x%'` / `'%x%'` / `'%x'` | `==` / `slice` ile önek / `index-of` ile içerme / `slice` ile sonek |
| `ve`, `veya`, `değil` | `all`, `any`, `!` |

- `has` koruması eksik alanın karşılaştırmasını yanlış yapar (KentOS'ta da öyledir; MapLibre'de `null` sayıya 0 olur).
- `gibi` büyük küçük harfe duyarlıdır (KentOS'un `gibi`'si; `benzer` duyarsızdır, yazılmaz); `%` ya da `_`'nin içeride
  olduğu kalıp çevrilmez: kural alt kurallarıyla yazılmaz ve söylenir.
- Kategorili ve aralıklı görünüşün ifadesi bir alan olmalıdır (`[ada]`); değilse sınıflar yazılmaz ve söylenir.

### 6. Etiketler

Katmanın etiketi (Etiketler penceresi: tek etiket ya da kurallı sınıflar; ADR 0212) `symbol` katmanı olur, geometriye göre:

| | |
|---|---|
| metin | şablon `{alan}` → `concat` + `to-string`; ifade yalnız alan olabilir |
| boy, renk, halo | `text-size` (px), `text-color`, `text-halo-color`, `text-halo-width` |
| yazı tipi | `Noto Sans Regular`, `Bold` (kalınlık ≥ 600), `Italic`; `glyphs` adresi girdide, yoksa MapLibre'nin demo sunucusu |
| nokta | `center` → `text-anchor: center`; çevresinde → `text-variable-anchor` + `text-radial-offset` |
| çizgi | `symbol-placement: line`; tekrar → `symbol-spacing`; üstte/altta → `text-offset` |
| alan | alanın içinde; çevre boyunca etiket yazılmaz ve söylenir |
| öncelik, çakışma | `symbol-sort-key` (10 − öncelik), `text-allow-overlap` (Her zaman) |

Yazılmayanlar, sınıf başına tek cümleyle söylenir: çağrı çizgisi, zemin, gölge, yığma, kısaltma, küçültme, yinelenenleri tek
yazma, çizgileri birleştirme, ölçek sınırları, en küçük nesne, nesneye göre büyüme, dışarıda yazma. Yazı tipleri ve
`glyphs` adresi stile yazılan değerlerdir, çalışan bir sunucuya bağlı olduklarını KentOS sınamaz.

### 7. Okuma

Stil katmanı KentOS katmanının sayılır (sırayla):
1. `metadata["kentos:layer"]` katmanın kimliğiyse (KentOS'un yazdığı stil);
2. yoksa `source-layer`, `id` ya da `id`'nin `/` öncesi katmanın kimliği ya da adıysa;
3. girdide `from` varsa yalnız `source-layer` ya da `id` `from`'a eşit olan.

`layer` verilirse yalnız o katmanın, verilmezse eşleşen her katmanın görünüşü yazılır; hiç eşleşen yoksa ret
(`style_layer_not_found`). Katmanların hepsi tek geri alma adımıdır.

Eşleşen stil katmanlarının `fill`, `line` ve `circle` olanları okunur. Diğerleri adıyla söylenir: `symbol` (etiket),
`background`, `fill-extrusion`, `heatmap`, `raster`, `hillshade`. `visibility: none` kapalı simge katmanıdır.

- **Süzgeç:** ifade biçimi ve eski biçim (`["==", "ad", "x"]`, `["in", "ad", …]`, `["has", …]`, `["!in", …]`);
  `==`, `!=`, `<`…, `in`, `has`, `match` (üyelik), `all`, `any`, `!`; `["geometry-type"]`/`$type` süzgeci hangi simgenin
  olduğunu söyler. Yazıcının koruma `has`'ları sadeleştirilir; yazıcının kalıpları (`boş`, `boş değil`, `arasında`,
  `gibi`) geri tanınır. Çevrilemeyen süzgeçli katman alınmaz ve söylenir.
- **Boya değerleri:** sabit, `["literal", …]`, `rgb`, `rgba`; `match` ve `step` (tek alan); metre `interpolate`'i; CSS
  renkleri (`#abc`, `rgb()`, `rgba()`, `hsl()`, `hsla()`, adlı renkler). Renk okunamazsa katman alınmaz. Başka ifade (`case`,
  `coalesce`, …) söylenir ve değer alınmaz (varsayılanı kalır).
- **Görünüşün türü:** boyada tek bir alanın `match`'i varsa kategorili, `step`'i varsa aralıklı, süzgeçli katmanlar kurallı,
  hiçbiri yoksa tek simge.
  - Kategorili: sınıflar `match`'in değerleri (dizi etiketleri her değer için ayrı sınıf), Diğerleri yedek değeri; katmanın
    süzgeci tam olarak değerlerin üyeliğiyse Diğerleri yoktur.
  - Aralıklı: süzgecin `≥ alt` ve `≤ üst` sınırları ilk ve son aralığın uçlarıdır. Sınır yoksa aralıklar açık uçlu
    kurallar olur.
  - Süzgeçli katmanların bitişik ve aynı süzgeçli olanları bir kuralın simge katmanlarıdır; kurallar tek alanın eşitlikleri
    ya da bitişik aralıklarıysa (en az iki) kategorili ya da aralıklı görünüş olarak tanınır.
- **Simge:** `line` katmanı alan simgesinin kenarıdır (`kentos:symbol: fill`, ya da geometri süzgeci çokgense, ya da aynı
  kuralda bir `fill` varsa), değilse çizgi simgesi; `circle` nokta simgesi. Simgenin saydamlığı: aynı simgenin katmanları aynı
  saydamlıktaysa simgenin kendisi, değilse renklerin saydamlığına katılır.
- **Birimler:** §4'ün tersi. `fill-outline-color` 1 piksellik kenar çizgisi olur.
- Alınmayanlar adıyla söylenir: tanınmayan boya ve yerleşim özellikleri (desen, kaydırma, bulanıklık), başka alana ya da
  türe bağlı ikinci değer, zoom eğrileri, etiketler.

### 8. Kayıplar

Yazarken söylenenler: §3'ün yazılmayanları, §4'ün metre ve ölçek yaklaşıkları, §5'in çevrilemeyen kalıbı, §6'nın yazılmayan
etiket ayarları, katmanın karışım kipi ve maskesi, bir katmanın yazılmadığı (çizilecek bir şeyi yok), nesneleri GeoJSON'a
yazılamayanlar (WGS 84'e çevrilemeyen, adı ve sayısıyla), GeoJSON'un daire, elips ve taramayı çizgi yazması.

Okurken söylenenler: §7'nin alınmayanları ve hiçbir KentOS katmanına denk gelmeyen stil katmanları.

### 9. Komutlar

`file.export.maplibre` ve `file.import.maplibre`, SLD'nin ve QML'in komutlarıyla aynı biçimdedir (ADR 0516 §4, 0555 §5).

**`file.export.maplibre`:**
- girdi: `path` (`.json`), `layer` (yoksa görünen katmanlar), `overwrite`, `sourceUrl`, `sourceKind` (`geojson` ya da
  `vector`; yoksa adreste `{z}` ya da `.pbf`/`.mvt` varsa vektör), `glyphs`;
- `sourceUrl` yoksa her katmanın nesneleri **WGS 84 GeoJSON** (RFC 7946) olarak stilin yanına yazılır: `<ad>.geojson` ya
  da birden çok katmanda `<ad>.<katman kimliği>.geojson`; öznitelikler özellik olur (tipli alanlar JSON sayısı), kaynağın
  `data`'sı göreli dosya adıdır. Projenin koordinat sistemi yoksa (SRID 0, tanım yok) ret (`no_system`): `sourceUrl` verin;
- `sourceUrl` varsa dosya yazılmaz: vektör kaynağında adres `{z}` taşıyorsa `tiles`, değilse TileJSON `url`'i; her katman
  `source-layer` olarak **adını** taşır (vektör karo üreticileri katmanı adıyla adlandırır); GeoJSON adresi tek katmanındır
  (birden çok katmanla `invalid_input`);
- retler sırayla: `invalid_file_path`, `file_exists` (stil ya da GeoJSON var), `layer_not_found`, `not_a_layer`,
  `service_layer`, `no_system`, `nothing_to_export`, `file_unwritable`; uyarı `export_losses`;
- önce veri dosyaları, en son stil yazılır (veri olmadan stil yoktur); her dosya geçici adla yazılıp geri okunur;
- çizimi değiştirmez; çıktı: `layers` (alttan üste), `styleLayers`, `rules`, `bytes`, `sha256`, `data` (yazılan GeoJSON'lar:
  nesne sayısı, bayt, SHA-256), `lost`.

**`file.import.maplibre`:**
- girdi: `layer`, `from`, `path` ya da `text`, `expectedRevision`;
- retler sırayla: `invalid_input`, `file_unreadable`, `invalid_maplibre`, `layer_not_found`, `not_a_layer`, `service_layer`,
  `style_layer_not_found`, `revision_conflict`; uyarı `import_losses`;
- `cad.layers.renderer`'ın yazmasıyla tek geri alma adımıdır (“Katman stili”), kayıtta kendi adıyla; aynı görünüşteyse
  katman için adım yazılmaz (`changed: false`); `plan` dosyayı okur, hiçbir şey yazmaz;
- çıktı: katman başına `layer`, `from` (alınan stil katmanları), `kind`, `rules`, `renderer`, `changed`; `lost`, `revision`.

Komutlar başsız sunucuda, Python'da (`kentos.cad.file.export.maplibre`, `kentos.cad.file.import_.maplibre`) ve MCP'de
çalışır; ikisi de MCP'nin yıkıcı listesindedir.

### 10. Masaüstü

- Katmanlar'ın sağ tık menüsünde (Görünüş ▸), QML'in altında:
  - Görünüşü MapLibre stili olarak kaydet… (o katman);
  - MapLibre stilinden görünüş al… (o katman).
- Veri › Dosya alışverişi'nde (▾ listesinde):
  - MapLibre stili dışa aktar… (görünen bütün katmanlar, tek stil);
  - MapLibre stilinden görünüş al… (stilin hazırlandığı her katman).
- Yol sorulur; sonuç ve söylenen kayıplar komut satırında. Seçenekler (`sourceUrl`, `glyphs`) komutta ve Python'dadır,
  pencerede değil.

## Doğrulama

- `python3 scripts/fixtures/maplibre_cases.py --check`:
  - **33 ortak durum** (`fixtures/commands/v1/file.import.maplibre.json`), başvuru KentOS kodu olmadan yazıldı: okuma
    kuralları ADR'den `json` ile, renk ve süzgeç ayrıştırıcıları betiğin kendisinin; girdi `fixtures/maplibre/v1`'deki
    yirmi elle yazılmış stil (üçüncü tarafın kategorili ve aralıklı stili, eski süzgeç biçimi, CSS renkleri, geometri
    süzgeçleri, kapalı katman, çok katmanlı stil, `from`, KentOS'un yazdığı biçim, kayıplar, retler). Python SDK'sı da
    aynı durumları tipli katmandan çalıştırır.
  - **Resmî doğrulayıcı:** KentOS'un yazdığı altı stil (`KENTOS_WRITE_MAPLIBRE=1 cargo test -p kentos-headless --test
    headless maplibre` yazar, `fixtures/maplibre/v1/out`) `@maplibre/maplibre-gl-style-spec` 26.4.4'ün
    `gl-style-validate`'inden geçer (npm ile `.run/maplibre-spec`'e kurulur, depoya bağımlılık eklenmez); doğrulayıcının
    bozuk bir stili reddettiği de denetlenir.
  - **Resmî ifade değerlendiricisi:** aynı paketin `featureFilter` ve `createPropertyExpression`'ı stilin süzgeçlerini ve
    boya değerlerini örnek nesnelerde (sayı ve metin olarak yazılmış sayılar, eksik alan, aralığın dışı) değerlendirir
    ve KentOS'un görünüşünün çizeceğiyle karşılaştırır: kategorili renkler, aralıklı sınıflar ve dışarıda kalanlar, kuralların
    süzgeci ve Diğerleri'nin olumsuzu iki yakınlaştırma düzeyinde, mm ve metre genişliğinin piksel karşılığı, çemberin yarıçapı.
- `cargo test -p kentos-formats --lib maplibre`: çekirdeğin birim testleri (birleşik kategorili ve aralıklı yazım, süzgeç ve
  ölçek, birimler ve kayıplar, iki dil arası süzgeç, el yazısı stil, etiketler).
- `cargo test -p kentos-headless --test headless maplibre`: gpkg'den alınan katmanlarla beş görünüş yazılır ve geri okunur
  (plan hiçbir şey yazmaz, GeoJSON'un boylam ve enlemi, `file_exists`), bütün harita (katman sırası, bir GeoJSON her
  katman), vektör karo adresi, tek GeoJSON adresinin reddi.
- `pnpm py:test` içinde `python/tests/test_maplibre.py`.
- Masaüstü resimleri: `KENTOS_SHOTS_ONLY=maplibre,menu-gorunus cargo test -p kentos-desktop layering::screens -- --ignored
  --nocapture` → `.run/shots/katman-secimi-maplibre-*`, `…-maplibre-yaz-*`, `…-menu-gorunus-*`.

## Sonuçlar

- MapLibre'ye giden görünüş çoğu zaman tek bir `match`/`step`'li katman çifti olur; sınıfların yapısı ayrıysa katman sayısı
  sınıf sayısıyla büyür. İkisi de geçerli ve aynı çizimi verir (doğrulayıcı ve değerlendirici bunu söyler).
- Veri GeoJSON olarak yanındadır: büyük katmanlar için vektör karo (`sourceUrl`) gerekir; KentOS karo üretmez (ADR'nin
  dışında; GDAL `ogr2ogr -f MVT` ya da tippecanoe, katman adıyla).
- mm → piksel dönüşü tek yönlüdür; metre genişlikleri gidiş-dönüşte korunur.
