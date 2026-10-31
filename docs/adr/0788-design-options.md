# ADR 0788: Tasarım seçenekleri

- **Durum:** kabul edildi (2026-10-31).
- **İş:** plan/architecture-advanced.md `ADV-05`'in üçüncü dilimi (ADV-05c): tasarım alternatifleri; ilk dilimler
  [ADR 0775](0775-room-modules-and-controlled-reuse.md), [ADR 0779](0779-room-modules-members-drift-and-flats.md).
- **Web:** web dururken (ADR 0228) yalnız masaüstü; web'e gerekenler aşağıda, “Belgelere eklenecek satırlar”da.
- **`.kcad`:** şema 153 (geçici numara, opt-in; birleşimde verilir). `FORMATS_VERSION` değişmez: yeni alanlar proje
  ayarlarının JSON'undadır, tipli sütunlara girmez.
- **Bağımlılık:** yeni kitaplık yok.

## Bağlam

Planın bir parçası çoğu kez iki üç biçimde düşünülür: mutfak A (tezgâh tek duvarda) ya da B (bölme bir metre sağda,
ada tezgâh). Bugün iki yol vardı: kopyayı yanına çizmek (iki mutfak aynı yerde değil, karşılaştırması gözle) ya da
evre (`arch.phase`) ile gizlemek (evre yapım evresidir; mevcut/yıkım/yeni anlamını bozar). Plan bunu açıkça yasaklar:
“Evre/mevcut-yıkım-yeni alanı alternatif kimliği olarak kullanılmasın. Önce bağımsız alternatif kopyası ve
karşılaştırma yeterliyse, karmaşık otomatik alternatif birleştirme sistemi baştan zorunlu kılınmasın.”

Başvurulan kaynaklar (31 Ekim 2026'da okundu):

- **Autodesk Revit, “Workflow: Design Options”.** Ana model ortak öğeleri tutar; her bölge için bir seçenek kümesi
  (Design Option Set), kümenin bir birincil (primary) seçeneği kendiliğinden kurulur; birincil seçenekler bütün
  görünümlerde görünür, ikincil seçenekler ayrı görünümlerde gösterilip paftaya konur; seçilen seçenek ana modele alınır
  ve küme ile öbür seçenekler silinir.
  <https://help.autodesk.com/cloudhelp/2016/ENU/Revit-Model/files/GUID-D6D4E961-8E43-498D-8405-EDE3BB11A4BD.htm>
- **Autodesk Revit, “Promote a Secondary Option to the Primary Option”.** Make Primary ikincil seçeneği birincil yapar;
  ana modelden eski birincile verilen başvurular kaybolabilir.
  <https://help.autodesk.com/cloudhelp/2016/ENU/Revit-Model/files/GUID-C7089499-2B3E-4514-98DE-62F5FC0E6F5F.htm>
- **Autodesk Revit, “Referencing Elements in Design Options”.** Ana modelin öğesi ikincil seçeneğin öğesini barındıramaz
  ve ona başvuramaz; seçeneğin öğesi ana modele başvurabilir; ana model ile birincil seçenek birbirine başvurabilir.
  <https://help.autodesk.com/cloudhelp/2017/ENU/Revit-Model/files/GUID-1CBD393C-342C-482D-98D7-269E1915DCFE.htm>
- **Graphisoft Archicad, Design Options (AC27–AC29).** Seçenek kümesi, seçenek ve seçenek bileşimi (Combination) ayrıdır;
  bileşim görünüm ayarıdır (her görünüm bir bileşimle çizer). Öğenin Option Status'u ait olduğu seçeneği ya da ana modeli
  söyler; bir seçeneğin öğeleri başka seçeneklerin ve ana modelin öğeleriyle kesişmez. Accept & Merge bir seçeneği ana
  modele katar, kümenin öbür seçenekleri ve küme silinir; bir seferde tek seçenek.
  <https://help.graphisoft.com/AC/28/INT/_AC28_Help/071_DesignOptions/071_DesignOptions-21.htm>,
  <https://help.graphisoft.com/AC/27/INT/_AC27_Help/071_DesignOptions/071_DesignOptions-22.htm>,
  <https://help.graphisoft.com/START/27/INT/_AC27_Help/071_DesignOptions/071_DesignOptions-11.htm>
- Archicad'in seçenekleri karşılaştırma yolu: Model Compare, yan yana görünümler ve Trace & Reference (aynı sayfa).

KentOS ikisinden alır ve ayrılır: Revit'in birincil/ikincil ayrımı ve “ana model ikincil seçeneğe başvuramaz” kuralı
alınır, ama KentOS daha sıkı: ana model hiçbir seçeneğe başvuramaz (birincil değişince başvuru kaybolmasın diye; Revit'in
uyardığı kayıp burada hiç doğmaz). Archicad'in görünüme bağlı bileşimi ayrı bir kavram olarak kurulmaz: adlı görünümün
tarifi (ADR 0774) kümelerin seçimini taşır. Archicad'deki gibi seçeneklerin öğeleri birbirine değmez; ama ana modelle
gösterilen seçenek birleşir (bölme duvarı dış duvara T bağlanır): yoksa birincil seçenek planda kopuk çizilirdi.
Karşılaştırma Archicad'in Model Compare'i gibi plan farkıdır, ama sayılar ve alanlarla birlikte bir komuttur.

## Karar

### 1. Veri: küme, seçenek, üyeler (ayarda)

Projenin seçenek kümeleri `settings.arch.options`'tadır (`OptionSet`: kimlik, ad, birincil seçeneğin kimliği,
seçenekler; `DesignOption`: kimlik, ad, `members` nesnelerin kalıcı kimlikleri). Hiçbir seçenekte olmayan nesne ana
modeldir. Üyelik nesneye yazılmaz: her tür (duvar, açıklık, mahal, donatı, ölçü, etiket, yazı, çizgi) aynı yoldan üye
olur, nesne türlerinin sütunları ve `FORMATS_VERSION` değişmez. Bir nesne yalnız bir seçenekte olur; en çok 50 küme,
kümede 1–12 seçenek, seçeneklerde 20 000 nesne (`option_sets_problem`).

**Bağ kuralı** (`option_leaning`): ana modelin bir nesnesi bir seçeneğin nesnesine (açıklığın duvarı, etiketin nesnesi,
ölçünün noktaları, mahalin kaynakları, bağlı yazının nesnesi) ya da bir seçeneğin nesnesi başka bir seçeneğinkine
bağlanamaz; seçeneğin nesnesi ana modele bağlanabilir. Tanımla, Seçeneğe ekle ve Seçenekten çıkar bunu yazmadan önce
çizimin bütün başvurularına bakar; ilk ihlal çizim sırasıyla söylenir. Böylece kabul ve silme hiçbir bağı asılı
bırakmaz.

### 2. Komutlar

`cad.options.edit` (her yazma tek geri alma adımı “Tasarım seçeneği”; plan aynı hesabı yazmadan verir):

- **define** (`name`, isteğe bağlı `id`, `option`, `uids`): seçilen nesneler yeni kümenin birincil “A” seçeneği olur,
  yerlerinde kalır. Kimlik adın katlanmışı (kapı tipinin kuralı), çakışırsa numaralı.
- **copy** (`set`, `option`, isteğe bağlı `name`, `primary`): seçeneğin nesnelerinin bağımsız kopyası aynı yerde yeni
  seçenek olur (ilk boş harf, adı büyük harfi); kopyaların birbirine bağları kopyalara çevrilir, ana modele bağları
  kalır. Kilitli katmanda nesne `layer_locked`; 12 seçenekte `option_set_full`.
- **addMembers**, **removeMembers**: ana modelden seçeneğe, seçenekten ana modele; bağ kuralıyla.
- **primary**: çizim, paftalar, metraj ve çizelgeler o seçeneği gösterir.
- **accept** (isteğe bağlı `option`, yoksa birincil): seçenek ana model olur, kümenin öbür seçeneklerinin nesneleri
  silinir, küme kalkar. **Otomatik birleştirme yoktur**: seçenek bütünüyle kabul edilir ya da edilmez.
- **delete**: birincil olmayan seçenek nesneleriyle silinir (birincil `option_primary`).
- **rename**: küme ya da seçenek.

Çizimden silinmiş üye bir sonraki seçenek yazmasında seçenekten düşer; okuma onu `missing` sayar ve
`option_member_gone` der.

`cad.options.query` (salt okunur): kümeler; her seçeneğin nesne sayısı, türlere göre sayıları, mahallerinin alanı
(çizim biriminin karesi), duvarlarının boyu, eksikleri; `uid` ile nesnenin kümesi ve seçeneği; `compare` (`set`, `a`,
`b`, `search`, `tolerance`) iki seçeneği Veri karşılaştır'ın kuralıyla (`ops::compare`, ADR 0179, konumdan eşleme)
karşılaştırır: aynı sayısı, yalnız b'de (`added`), yalnız a'da (`removed`), değişen çiftler (`geometry`, `attributes`,
`both`), alan ve duvar boyu farkı (b − a), sayısı farklı türler. Öntanımlı arama 0,5 m, tolerans 1 mm (çizim biriminde).

### 3. Görünüm: birincil, tarif

Bir “seçenek görünümü” her küme için gösterilen seçeneği söyler: görünüm bir küme için seçmediyse birincildir
(`OptionSet::shown`, `option_hidden_uids`). Gösterilmeyen seçeneklerin nesneleri:

- çizimde çizilmez (`kentos_plot::layers::phasing_of`), seçilmez ve kenetlenmez (deponun `phase_hidden`'ı,
  `Spatial::sync_phases`, `geometry_store::store_of`, `viewport_scope::hidden_entities`);
- paftada çizilmez (`plot_phasing_of`); adlı görünümün tarifi `options: [{set, option}]` ile bağlı haritada başka bir
  seçeneği gösterir (`ViewRecipe.options` → `MapPrim.options` → `map_phasing_of`'un `options`'ı; harita kendi deposunu
  kurar, `Spatial::set_option_view`, masaüstü `SheetMaps::scaled` ve başsız kitap `MapDrawer`). `cad.views.edit` tarifin
  kümesini ve seçeneğini projede arar (`option_set_not_found`, `option_not_found`); bir küme iki kez ya da kimlik olmayan
  ad `invalid_recipe`.
- Görünüş şablonuyla (ADR 0783): gösterilen seçenek kat ve kırpma gibi görünümündür. Şablona bağlı tarif kendi
  `options`'ını tutar (`resolved_recipe` onu şablondan almaz; masaüstünde şablon seçilip bırakılınca da kalır); şablonun
  tarifi seçenek taşımaz (`template_problem`, `invalid_recipe`). Gerekçe: iki görünüm aynı “Uygulama 1:50” şablonunu
  izlerken biri mutfak A'yı, öbürü B'yi gösterebilmeli; Revit'te tasarım seçeneği görünürlüğü şablona girer, ama
  KentOS'ta şablon çizim biçimidir, hangi alternatifin gösterildiği görünümün işidir.

### 4. Seçenekler birbirini görmez

- **Duvar birleşimi:** deponun `set_option_out`'u gösterilmeyen seçeneklerin duvarlarını birleşim çözümünden çıkarır
  (kendi başlarına çizilir) ve açıklıklarını duvar boşluğundan çıkarır. Ana model gösterilen seçenekle birleşir. Haritanın
  kendi deposu kendi seçimiyle çözer; iki seçeneğin duvarı hiçbir depoda buluşmaz.
- **Açıklık çakışması:** aynı kümenin iki seçeneğindeki açıklıklar çakışmaz (`options_meet`); ana modelin duvarındaki
  bir kapının A ve B kopyaları `overlap` sayılmaz.
- **Mahal sınırı:** mahal sınırını ana modelin ve kendi seçeneğinin duvarlarında bulur, öbür kümelerde birincilinkilerde
  (`SpaceSearch::found_as`, `Document::option_view_of`); ana modelin mahali birincillerinkilerde. Birincil değişince ya
  da nesne seçeneğe girip çıkınca (`Op::ArchOptions`) değen mahaller aynı adımda yeniden bulunur, açıklıklar yeniden
  denetlenir, bütün çizelgeler yeniden yazılır.

### 5. Metraj, çizelge, denetim, dışa aktarma

Gösterilmeyen seçenekler yapılmayacaktır: Metraj (`cad.spaces.quantities`), çizelgeler (`schedule_rows`) ve Mimari
denetim (`cad.arch.audit`, birincil görünümün okuma kopyasında: `options_edit::primary_view`) onları saymaz; iki seçeneğin
üst üste duvarı denetimde yinelenen duvar çıkmaz. DXF yalnız çizileni yazar; gösterilmeyen seçeneklerin nesneleri
yazılmaz ve raporda “Birincil olmayan tasarım seçeneği” denir; kümeler yalnız KCAD'dedir.

### 6. `.kcad` şema 153 (geçici)

`settings.arch.options` ve `namedViews[].recipe.options` şema 153'ten beri yazılır; seçeneği olmayan proje eski
şemasıyla aynı baytlardır. Okuyucu küme ve seçenek kimliklerini, adların tekliğini, birincilin kümede olmasını, bir
nesnenin tek seçenekte olmasını denetler (`bad_value`); önceki şemada `unknown_field`. Örnek `options.kcad`, bozuk
örnekler `broken/options-in-schema-152.kcad`, `broken/options-object-twice.kcad`, `broken/options-primary-unknown.kcad`;
aralık dışı `broken/schema-version-154.kcad`. Bağımsız okuyucu `tools/kcad/kcad.py`, yazıcı
`scripts/fixtures/kcad_v2_reference.py`, spesifikasyon `docs/specs/kcad-v2.md`.

### 7. Masaüstü

Mimari › Proje ▾'da **Tasarım seçenekleri** (`arch.options`, ikon `designOptions`; `catalog/v1/additions.json`,
düzeyi `operations.json`'da `semantic`). Pencere: kümeler; seçilen kümenin seçenekleri (birincil, nesne sayısı, mahal
alanı, duvar boyu, silinmiş); iki seçeneğin karşılaştırması (aynı, değişmiş, yalnız A'da, yalnız B'de, alan ve duvar
farkı, tür sayıları); Kabul et'in planı (kaç nesne ana model olur, kaç nesne silinir); Seçimi ekle, Seçimi çıkar;
Seçimden tanımla, Seçeneği kopyala, Birincil yap, Seçeneği sil, Kapat, Kabul et. Görünümler penceresinin Görünüş
tarifinde her küme için **Tasarım seçeneği** seçimi (Birincil ya da bir seçenek). Her yazma komutla, önizlemeler komutun
planından.

## Doğrulama

- Bağımsız başvuru `scripts/fixtures/option_cases.py` (KentOS kodu olmadan: kimlik katlama, bağ kuralı, kopya ve bağların
  çevrilmesi, kabul, silme, ad, alan ve boy, konumdan eşlemeyle karşılaştırma Python'da): `cad.options.edit` 49 durum
  (8'i `cad.views.edit` tarif ve şablon adımlarıyla), `cad.options.query` 23 durum; Rust koşucusunda 72/72.
- Kabul (`crates/native/headless/tests/headless/design_options.rs`): mutfak A, kopyası B, B'nin bölmesi 1 m sağda;
  B'nin mutfağı 1,9 × 3,9, A'nınki 2,9 × 3,9, salon A'yı görür (2,9 × 3,9); depoda B gizli ve birleşim sorunu
  eklenmez; B birincil olunca salon 3,9 × 3,9 olur, depo A'yı gizler; karşılaştırma bölmeyi ve kapısını değişmiş bulur,
  alan farkı −3,9 m²; ana duvardaki kapının iki seçenekteki kopyası çakışmaz; paftanın görünüşü ve tarifin seçimi;
  KCAD'e yazılıp açılınca kümeler aynı; DXF yalnız çizileni yazar ve 6 nesneyi söyler; B'yi kabul etmek A'yı siler, tek
  geri alma her şeyi geri getirir.
- Masaüstü (`apps/desktop/src/design_option_scenes.rs`): pencereden tanımla, kopyala, birincil yap (depo B'yi
  gösterir), kabul et ve geri al; Görünümler'de tarifin seçeneği. Resimler: `secenek-tanimla`, `secenek-pencere`,
  `secenek-plan-a`, `secenek-plan-b`, `secenek-tarif` (iki tema, 1440×900 ve 1100×650).
- KCAD: `kentos-kcad` testleri, `kcad_v2_reference.py --check`.
- Python SDK `kentos.cad.options`, MCP aracı (katalogdan; `cad.options.edit` yıkıcı listede), manifest.
- Çalıştırılanlar: `kentos-native-application` bütün testleri (bütün ortak komut durumları dahil, 266), `kentos-kcad`,
  `kentos-domain`, `kentos-commands`, `kentos-contracts`, `kentos-plot`, `kentos-sheet`, `kentos-headless` bütün testleri,
  masaüstünün pafta, tarif, modül ve seçenek testleri, Python'da seçeneklerin ortak durumları ve taşıma testi; dokunulan
  crate'lerde clippy `-D warnings` (yolda `modules_more.rs` ile `flat_modules.rs`'teki iki eski clippy bulgusu
  düzeltildi). Pafta haritasının ilkelindeki `MapPrim.options` kutulu dilimdir (`Prim`'in
  `clippy::large_enum_variant` izni ADR 0783'ten). `kentos-sheet-ui` testindeki eksik `pens`/`mono` alanları da
  eklendi; MCP'nin `desktop_attach_names_the_program_whose_link_it_takes` testi bu çalışma kopyasının uzun yolunda Unix
  soketinin adı sığmadığı için düşer (seçeneklerle ilgisiz).

## Sınırlar

- Otomatik birleştirme, seçenekler arası nesne eşleme ya da “B'nin değişikliklerini A'ya al” yok: seçenek bütünüyle
  kabul edilir. Seçenek bileşimi (Archicad'in Combination'ı) ayrı kavram değil; görünüş tarifi taşır.
- Ana model hiçbir seçeneğe bağlanamaz (Revit birincile izin verir); ana modelden bağlı nesne (etiket, ölçü) seçeneğe
  birlikte alınmalıdır.
- Kopya aynı yerdedir; seçenekler iki ayrı yerde çizilip yan yana konmaz (yan yana karşılaştırma iki adlı görünümün
  paftasıyla).
- Bağlı haritada gösterilen ikincil seçeneğin mahal alanları ve çizelgeleri birincil görünümün hesabıdır (Revit gibi);
  haritanın kendi deposu yalnız duvar birleşimini ve gizlemeyi kendi seçimiyle yapar.
- Kolonlar mahal düşümünde seçenek ayrımı yapmaz (seçenekteki kolon her mahalde düşülür).
- Çizelge, Metraj ve denetimde “şu seçeneği say” ayarı yok; hep birincil.
- Gerçek pencerede elle denenmedi; resimler pencere açmadan.

## Belgelere eklenecek satırlar

### CLAUDE.md §0

- **Tasarım seçenekleri (31 Ekim 2026; [ADR 0788](docs/adr/0788-design-options.md), ADV-05 [~]).**
  Planın bir parçası için sınırlı alternatifler: `settings.arch.options` (küme, birincil, seçenekler, üyeler kalıcı
  kimlikle; `.kcad` şema 153 geçici, opt-in), hiçbir seçenekte olmayan nesne ana model. `cad.options.edit` (define,
  copy: bağımsız kopya aynı yerde, bağlar kopyalara; addMembers, removeMembers, primary, accept: seçenek ana model olur,
  öbürleri silinir, küme kalkar; delete, rename; tek adım “Tasarım seçeneği”; otomatik birleştirme yok); ana model
  seçeneğe, seçenek başka seçeneğe bağlanamaz `option_leaning`. `cad.options.query` (sayılar, mahal alanı, duvar boyu,
  `compare`: Veri karşılaştır'ın konum kuralıyla aynı/değişen/eklenen/kaldırılan, alan farkı). Çizim, pafta, Metraj,
  çizelge, Mimari denetim ve DXF birincili gösterir; görünüş tarifinin `options`'ı bağlı haritada başka seçeneği
  (`map_phasing_of`, haritanın deposu `Spatial::set_option_view`). Seçenekler birbirini görmez: gösterilmeyenin duvarı
  birleşmez ve açıklığı duvarı kesmez (`Store::set_option_out`), iki seçeneğin açıklığı çakışmaz, mahal sınırını ana
  model ve kendi seçeneğinde bulur (`SpaceSearch::found_as`); birincil değişince mahaller, açıklıklar ve çizelgeler aynı
  adımda izler. Mimari › Proje ▾ Tasarım seçenekleri (`arch.options`), Görünümler'de tarifin Tasarım seçeneği (şablona bağlı
  tarifte de görünümün; şablon seçenek taşımaz). 49 + 23
  ortak durum (`option_cases.py`). Seçenek bileşimi, seçenekler arası eşleme ve gerçek pencerede elle kabul açık.

### CLAUDE.md §2 (çalıştırma)

```bash
python3 scripts/fixtures/option_cases.py --check; KENTOS_CASES_ONLY=cad.options cargo test -p kentos-native-application --test all every_command_case; cargo test -p kentos-headless --test headless design_options; cargo test -p kentos-desktop --lib design_option_scenes   # tasarım seçeneklerinin 49 + 23 ortak durumu KentOS kodu olmadan (8'i görünüş tarifi ve şablonu), mutfak A/B kabulü (birleşim, mahal sınırı, çakışma, birincil, karşılaştırma, KCAD 153, DXF, kabul ve geri alma) ve pencere; resimler KENTOS_SHOTS_ONLY=secenek-tanimla,secenek-pencere,secenek-plan-a,secenek-plan-b,secenek-tarif cargo test -p kentos-desktop --lib tools_screens::screens -- --ignored --nocapture (.run/shots/arac-secenek-*) (ADR 0788)
```

### TODOS.md §0.2 (web'e ertelenenler)

- ADR 0788: `cad.options.edit` ve `cad.options.query` web'in işleyicilerinde; görünüş tarifinin `options`'ı ve bağlı
  haritanın seçimi; Tasarım seçenekleri penceresi, Görünümler'de tarifin Tasarım seçeneği; çizimde ve depoda
  gösterilmeyen seçeneklerin gizlenmesi ve birleşimden çıkarılması; KCAD şema 153'ün okunması ve yazılması
  (`options.kcad`).

### plan/architecture-advanced.md ADV-05 (Durum)

_Durum (31 Ekim 2026, [ADR 0775](../docs/adr/0775-room-modules-and-controlled-reuse.md), [ADR 0779](../docs/adr/0779-room-modules-members-drift-and-flats.md), [ADR 0788](../docs/adr/0788-design-options.md)):_
tek kat, düz duvarlı oda ve bütün daire modülü; tanım ve örnek ayrı, yerleştirme, planlı kaynak güncelleme, yerel
istisna, bağımsızlaştırma, kaynağı silme; üye ekleme ve çıkarma; elle değişen örnek nesnesi; katları tekrarlama.
Tasarım seçenekleri: küme ve birincil seçenek, bağımsız kopya, birincili değiştirme, iki seçeneğin karşılaştırması
(sayı, alan, plan farkı), kabul (öbürleri silinir) ve silme; görünüş tarifiyle başka seçeneği gösteren pafta;
seçenekler birbirini görmez (birleşim, çakışma, mahal sınırı). Dört banyo, iki katta dört daire ve mutfak A/B kabulü
geçti. Açık: seçenek bileşimi, seçenekler arası eşleme/birleştirme, çok katlı tek modül, kattaki modülsüz nesnelerin
tekrarı, gerçek pencerede elle kabul.
