# ADR 0571: Stil alışverişinin kalanı: degrade, nokta deseni, SVG ve yazı işareti, renklerin veriye bağlı değerleri

- **Durum:** kabul edildi (2026-10-21). `plan/gis.md` CBS-17. ADR 0516 (SLD), 0555 ve 0565 (QML), 0560 (MapLibre)'nin
  “sonraya kalan”larının hepsi: degrade, nokta deseni, SVG ve yazı işareti katmanları, renklerin veriye bağlı değerleri.
  Kapsamı ben belirledim.
- **Bağlam belgesi:**
  - ADR 0186: tarama eklerinde degrade (`gradientFill`) ve nokta deseni (`patternFill`);
  - ADR 0094 ve 0192: sembol katmanları, SVG işareti (`svg`, kitaplığın SVG öğesi), yazı işareti (`text`);
  - ADR 0565: veriye bağlı değerlerin küçük ifade kuralı;
  - ADR 0100: ifade dili (`durum eğer … ise … yoksa … son`).
- **Web:** ertelendi (ADR 0228). Web'de gerekenler TODOS.md §0.2'de.
- **Model:** değişmez. KentOS'un simgesi bunların hepsini zaten taşır; `.kcad` şeması aynı kalır. Burada yalnız üç dosya
  biçimiyle alışveriş eklenir.
- **Sonraya kalan:** işlevli ve karşılaştırmalı ifadeler (`round`, `if`, `CASE`); QGIS'in öbür simge katmanları (`Raster
  image`, `Shapeburst`, `Geometry generator` …); SLD'de `ogc:Function`'ların `Recode` ve `Categorize` dışındakileri; etiketler,
  Katman çizimi, tematik işleyiciler.

## Bağlam

Dış stil kullanıcısının en sık kullandığı dört katman dışarıda kalmıştı: degradeli dolgular, nokta desenleri (orman,
bataklık), SVG simgeler (kapı, ağaç, durak) ve yazı tipinden simgeler (ok, sembol yazı tipleri). Bir de renklerin alana
bağlı olması (alanın değeri rengi söyler). Üç biçimin hangisinin ne söyleyebildiği ayrıdır; her kayıp adıyla söylenir.

## Karar

### 1. Degrade

| KentOS | QGIS `GradientFill` | SLD | MapLibre |
|---|---|---|---|
| `gradientFill` doğrusal | `type` 0, ilk ve ikinci nokta, `angle` | yok | yok |
| küre | `type` 1 (dairesel) | yok | yok |
| silindir | `type` 0, `spread` 1 (yansıtma), yarım kutu | yok | yok |

QML (QGIS 4.2'de çizdirilerek ve özellikleri okunarak denetlendi):
- KentOS'un doğrusal degradesi kutunun batısından doğusuna olan çifttir (`0,0.5` → `1,0.5`), `angle` ikisinde de saat yönünün
  tersine ölçülür ve çevrilmez; yazılırken `angle` değişmeden, okunurken QGIS'in iki noktasının doğrultusu açıya eklenir;
- küre: QGIS'te dairesel degrade merkezden (`0.5,0.5`) köşeye (`1,1`) gider; ilk renk merkezdedir. KentOS'un küresinde
  `color` merkezdir; bu yüzden iki renk yer değiştirir. `inverted` renkleri değiştirir;
- silindir: yarım kutuyla (`0,0.5` → `0.5,0.5`) yansıtma;
- okumada konik degrade doğrusal alınır ve söylenir; renk rampası (`color_type` 1: QGIS 4 `color1`, `color2`'yi katmanın
  kendi özelliklerinde, QGIS 3 `<colorramp>` öğesinde yazar) ilk ve son rengiyle alınır, ara durakları söylenir; görünüm
  kutusuna göre koordinat, kaydırma, yansıtma ve yineleme söylenir.

Söylenen, yazmada: QGIS noktaları nesnenin kutusunun kesirleridir, KentOS'un açılı degradesi kutunun doğrultudaki
izdüşümüdür. 0, 90, 180 ve 270 dışındaki açıda iki ucun yeri farklıdır, renk geçişi biraz kısalır. QGIS'ten gelen uçlar da
(kesir çifti) standart değilse aynısı söylenir; yön alınır.

SLD ve MapLibre: degrade yoktur. Yazılmaz, “degrade dolgu”nun adıyla söylenir. SLD'de karşılığı yoktur (SE'nin dolgusu
tek renk ya da resim); MapLibre'nin `fill-color`'ı tek renktir.

### 2. Nokta deseni

| KentOS `patternFill` | QGIS `PointPatternFill` | SLD | MapLibre |
|---|---|---|---|
| işaret (`marker`) | alt simge | `GraphicFill`'in `Graphic`'i (ilk işaret) | `fill-pattern` ister |
| `spacingX`, `spacingY` | `distance_x`, `distance_y` | yok | |
| `stagger` | `displacement_x` = aralığın yarısı | yok | |
| `angle` | `angle`, saat yönünde (eksi) | yok | |
| `offset` | `offset_x`, `offset_y` (y aşağı: eksi) | yok | |

QML: açı ve kaydırma işaretleri QGIS 4.2'nin çizdirilmesiyle denetlendi. `displacement_x`'in aralığın yarısı olması KentOS'un
`stagger`'ıdır; başka kayma, y kayması ve rastgele sapma okumada söylenir. Yazmada söylenir: rastgele sapma, doluluk ve tohum
(QGIS'in deseninde sabit dizilim vardır); ayrıca dizilimin başlangıcı (QGIS nesnenin kutusundan, KentOS çizimin başlangıç
noktasından başlar; işaretler bir hücre kayabilir).

SLD: `GraphicFill`, grafiğini kendi boyunda döşer; aralık ve kayma yoktur. Yazılan desen yalnız ilk işareti taşır; aralığın, her
ikinci satırın kaymasının, açının, kaymanın ve sapmanın yazılmadığı söylenir. Okunan `GraphicFill` bir nokta desenidir; aralığı
işaretin boyudur (söylenir). QGIS'in SLD'si `GraphicFill`'e aralığı yazmaz (yalnız `graphic-margin` satıcı seçeneğini yazar).

MapLibre: `fill-pattern` bir sprite ister; bir stil dosyası sprite taşımaz (sprite adresi ayrı bir kaynak). Yazılmaz, söylenir.

### 3. SVG işareti

KentOS'un SVG işareti kitaplığın SVG öğesine kimlikle bağlıdır (`asset`). Dosyalar bu kimliği bilmez, çizimi taşır:
- yazmada çizim simgenin içine konur (`svgData`; masaüstü uygulama katmanında, `inline_assets`, kitaplığın simgeleri de);
  biçim çekirdeğinin işlevlerinin imzası değişmez;
- QML: `SvgMarker`, `name` olarak `base64:` ve çizim (QGIS bu yazımı kendi de okur ve yazar);
- SLD: iki `ExternalGraphic`. Önce `OnlineResource`'ta `base64:` adresi (QGIS yalnız bunu okur; `InlineContent`'i önce
  koyunca yüklemez), sonra SE'nin `InlineContent`'i (`encoding="base64"`). Renk değiştirme parametreleri yazılmaz, söylenir
  (çizimin kendi renkleri kalır);
- MapLibre: `icon-image` bir sprite ister; yazılmaz, söylenir.

Okumada `base64:` ve `data:image/svg+xml;base64,` adresleri alınır; QGIS'in `?fill=…` ile parametreli adresi önce gelirse
okunmaz, sonraki düz adres alınır. Çizim projenin kitaplığına eklenir (kimlik `qml-` ya da `sld-` ve çiziminin 64 bitlik özeti:
aynı çizim iki kez bir öğe olur); bu bir kitaplık düzenlemesidir, geri alma adımı değildir (kitaplık geri alma tutmaz; ADR
0314). Bir dosya yoluna bağlı SVG (`/yol/ad.svg`, `ad.svg`) alınmaz, söylenir: KentOS çizimin içindeki SVG'yi alır, dosyayı
okumaz. Resim dosyası (PNG) alınmaz.

### 4. Yazı işareti

KentOS'un `text` işareti: bir karakter (ya da metin), yazı tipi, kalın ve eğik, renk, hale, dönüş, boy.

- QML `FontMarker`: `chr`, `font`, `font_style` (`Bold`, `Italic`), `color`, kenar rengi ve kalınlığı hale olur, `angle` (saat
  yönünde: çevrilir), `size`. Karakter ve renk veriye bağlı olabilir.
- SLD: QGIS'in okuduğu biçim `Mark` içinde `OnlineResource` `ttf://Yazı tipi` ile `MarkIndex` (karakterin kodu) ve `Format`
  `ttf`'dir; yazılan budur. Okumada `WellKnownName`'de `ttf://Yazı tipi#0x41` (ondalık ya da karakterin kendisi de) de alınır.
  SE'nin işareti tek karakterdir, kalın ve eğik yoktur: çok karakterli metnin ilk karakteri yazılır, kalın ve eğik söylenir.
  Hale işaretin `Stroke`'udur.
- MapLibre: işaret `symbol` katmanıdır (`text-field`, `text-size`, `text-rotate`, `text-color`, `text-halo-*`). Yazı tipi
  harf sunucusunun (Noto Sans) yığınıdır; işaretin yazı tipinin adı katmanın `metadata`'sında (`kentos:font`) gider ve
  okumada geri gelir. Harf sunucusunda olmayan yazı tipi ve simge yazı tipleri çizilmeyebilir: her yazmada söylenir. Kalın ve
  eğik birlikte olamaz (yığın kalın), söylenir. Okumada `symbol` katmanı yalnız yazının yazdığı (`kentos:symbol`
  `marker`) ya da tek karakterli, simgesiz ve etiket olmayan katmansa işarettir; öbürü etiket katmanıdır, eskisi gibi alınmaz.

### 5. Renklerin veriye bağlı değerleri

ADR 0565'in kuralı aynen: bir ifade alanlar, sayılar ve dört işlemse iki yazılımın anlamı aynıdır. Burada renkler için:

- QML: dolgunun, çizginin, işaretin dolgu ve çizgi rengi, yazı işaretinin rengi (`fillColor`, `outlineColor`, `color2`);
  alan olarak (`type` 2) ya da ifadeyle. Sabit renk KentOS'ta `fallback` olur. KentOS'ta alan ve ifade `{ "expr", "fallback" }`'tır.
  Çevrilemeyen ifade (`if(…)`, `CASE`) adıyla söylenir, sabit renk alınır.
- SLD: renk parametresinde `ogc:PropertyName` (alanın değeri renktir). Yazmada yalnız tek alanın ifadesi yazılabilir; yedek
  değer SLD'de yoktur, söylenir. Bu da söylenir: **QGIS SLD'deki alana bağlı rengi okumaz** (QGIS 4.2'de denendi: gri alır;
  GeoServer okur). Okumada `PropertyName` bir alandır; `ogc:Function` olarak yalnız `Recode` (bir alanın değerlerine renkler)
  ve `Categorize` (bir alanın sınırlarına renkler) alınır: KentOS'un `durum eğer [alan] = değer ise 'renk' … yoksa 'renk' son`
  ifadesine çevrilir. Başka işlevler adıyla söylenir, sabit renk alınır.
- MapLibre: `["to-color", ["get", "alan"], yedek]` yazılır, okunur; yalnız `["get", "alan"]` da okunur (yedeksiz). Yazmada yedek
  yoksa siyah yazılır ve söylenir. `match` ve `step`'in kategorili ve aralıklı görünüş olarak okunması (ADR 0560) değişmez.

## Doğrulama

- `python3 scripts/fixtures/qml_cases.py --check`: bağımsız okuyucuya degrade, nokta deseni, SVG ve yazı işareti ve renklerin kuralları
  eklendi; QGIS 4.2'nin yazdığı sekiz yeni QML (degradeler, desenler, SVG, yazı, renkler, katmanlar üst üste); 37 durum.
  KentOS'un yazdığı beş yeni QML (degrade, desen, SVG, yazı, renkler) QGIS'te `loadNamedStyle` ile açılır: degradenin türü,
  renkleri, uçları, yayılımı ve açısı, desenin aralığı, kayması, açısı ve kayması, SVG'nin adresi ve boyu ve açısı, yazı
  işaretinin yazı tipi, karakteri, stili ve rengi, ifadeler.
- `python3 scripts/fixtures/sld_cases.py --check`: QGIS 4.2'nin yazdığı dört yeni SLD, iki elle yazılmış (SE'nin işlevleri; yazı
  işaretleri, satır içi SVG, resim); 30 durum. KentOS'un yazdığı üç yeni SLD QGIS'te açılır: nokta deseni (`GraphicFill`),
  SVG'nin `base64:` adresi ve yazı işareti ve açıları, ve alanın rengi (QGIS'te gri: söylenen kayıp doğrulandı).
- `python3 scripts/fixtures/maplibre_cases.py --check`: iki yeni elle yazılmış stil; 35 durum. KentOS'un yazdığı iki yeni stil
  MapLibre'nin resmî doğrulayıcısından ve ifade değerlendiricisinden geçer: `to-color`'ın değerleri örnek nesnelerde, yazı
  işaretinin `symbol` katmanı.
- `KENTOS_WRITE_QML=1`, `KENTOS_WRITE_SLD=1`, `KENTOS_WRITE_MAPLIBRE=1` ile başsız testler (`layer_looks_go_out_as_qml`,
  `layer_looks_go_out_as_sld`, `maplibre`): her görünüş yazılır, geri okunur, SVG çizimi kitaplığa girer.
- `cargo test -p kentos-formats --lib`: çekirdeğin birim testleri.
- `KENTOS_CASES_ONLY=file.import cargo test -p kentos-native-application --test all every_command_case`: ortak durumlar.
- `KENTOS_SHOTS_ONLY=stil-simgeler cargo test -p kentos-desktop tools_screens -- --ignored --nocapture`: QML'in degrade, nokta deseni
  ve kenarı iki parselde, SVG ve yazı işaretleri dört durakta, SLD'nin nokta deseni üçüncüde (`.run/shots/arac-stil-simgeler-*`).

## Sonuçlar

- Üç biçimin hepsi dört katmanı ve renklerin alana bağlılığını söyleyebildiği kadar taşır; söyleyemediğini adıyla söyler, hiçbir
  şeyi sessizce atmaz.
- SVG içe aktarması kitaplığa yazar: bir QML ya da SLD'nin içe alınması geri alınınca simge eski görünüşe döner, kitaplıktaki
  SVG öğesi kalır (kitaplık geri alma tutmaz).
- Yeni bağımlılık yok. QGIS'in SLD'si degradeyi, nokta desenini aralığıyla ve alana bağlı rengi yazmadığı için bu üçü SLD'den
  gidiş-dönüşte QGIS'e göre kayıplıdır; SLD gerekmeyen yerde QML kullanılır.
