# ADR 0248: Katman, stil ve ayar komutları

- **Durum:** kabul edildi (2026-10-12). TODOS.md `CADP-08`: “Katman, stil ve ayar işlemleri katalogda; SDK ve MCP eşit” (plan
  `CAD-CMD-14`, `CAD-CMD-15`; §5.1 `cad.layers.*`, `cad.styles.*`, `cad.settings.*`; `CAD-STYLE-01`, `02`, `04`, `13`, `14`;
  `CAD-PY-03`, `05`, `06`; §24.1 madde 8). Kapsamı ben belirledim; paralel ajanın dalında (`cad-layers`) yapıldı.
- **Bağlam belgesi:** ADR 0072 (katman ve grubu nesneleriyle silme), 0076 (katman ekleme), 0020 (ağacın kendi değişiklikleri), 0177
  (katman yönetimi ekleri: kopya, birleştirme, durumlar, temizlik), 0183 (yazı ve ölçü stilleri), 0199 (katman alanları), 0049 (proje
  ayarları), 0165, 0167, 0168 (proje türü, birim, sistemler), 0205 (açıklamaların ölçeği), 0013 ve 0022 (ürün komutları), 0208–0213
  (var olan `cad.layers.*` komutları), 0244 (CAD işlem manifesti).
- **Web:** ertelendi (ADR 0228); geri dönüşte gerekenler TODOS.md §0.2'de.

## Bağlam

ADR 0244'ün manifesti katman ağacının, stil tablolarının ve proje ayarlarının hiçbir ürün komutu olmadığını gösterdi (`none`):
Katmanlar paneli, Kopyasını oluştur, Katmanları birleştir, Kullanılmayanları temizle, Katman durumları, Alanlar, Yazı ve Ölçü
stilleri, Proje ayarları ve katman araçları (Katmanı gizle, yalıt, kilitle, etkin yap) belgeye doğrudan yazıyordu. Python'un ve
MCP'nin katman ekleyecek, silecek, adlandıracak bir yolu yoktu; bir ofis katman şablonunu kurmak ya da çizim ölçeğini değiştirmek
için ayrı bir mutasyon yolu açmak gerekecekti (CLAUDE.md §0: “yetki ve transaction kurallarını atlayan ayrı mutasyon yolu
kurulmayacak”). Kurallar da kabuktaydı: birleştirme ve kopyanın akışı, katman durumlarının adı ve kimliği masaüstünün
`apps/desktop/src`'sinde yazılıydı (ADR 0228: kabuk incedir).

AutoCAD'de bunların hepsi komuttur (`-LAYER`, `LAYMRG`, `LAYERSTATE`, `STYLE`, `DIMSTYLE`, `PURGE`, `UNITS`, `CANNOSCALE`) ve
betikten çağrılır; QGIS'te `QgsProject.layerTreeRoot()` ve proje ayarları Python'dan değiştirilir.

## Karar

### 1. Kapsam: beş komut

| Komut | Ne yapar | Geri alma |
|---|---|---|
| `cad.layers.edit` | ekle, stil, taşı, sil, kopyala, birleştir, alanlar, temizle | her çağrı tek adım |
| `cad.layers.set` | yeniden adlandır, göster/gizle, kilitle/aç, etkin katman, yalıt, hepsini göster | adım değil (ağacın kendi değişikliği) |
| `cad.layers.states` | adlı katman durumunu kaydet, güncelle, adlandır, sil, uygula | uygulamanın stilleri tek adım; gerisi adım değil |
| `cad.styles.edit` | yazı ve ölçü stilini oluştur, güncelle, adlandır, sil | izleyen nesneler tek adım; tablo adım değil |
| `cad.settings.set` | projenin ayarlarından bir kümesini anahtarla değiştir | adım değil; ölçeğe uyan açıklamalar tek adım |

Komutlar iki aileye bölündü, çünkü bir komutun katalogdaki geri alma bilgisi (`CommandUndo`) tektir: ağacın kendi değişiklikleri
(göz, kilit, ad, etkin katman) ortak belge kurallarında (ADR 0020, `fixtures/document-ops/v1/layers.json`) düzenlemedir ama geri
alma adımı değildir; ekleme, silme, taşıma, birleştirme ise adımdır. İkisini bir komutta toplamak Python'a “bu çağrı geri alınır mı”
sorusunu cevapsız bırakırdı.

Hepsi yalnız masaüstünde çalışır (`hosts: desktop`, ADR 0228), başsız sunucuda, Python'da (`kentos.cad.layers.edit`, `.set`,
`.states`, `kentos.cad.styles.edit`, `kentos.cad.settings.set`) ve MCP'de (aynı adlı araçlar, `destructiveHint` açık).

Kapsam dışı (Sonuçlar'da): çizgi tipi kütüphanesi ve LIN desenleri (`CAD-STYLE-08`), viewport başına katman ayarı (`CAD-STYLE-12`),
dondurma ve çizdirme bayrakları (`freeze`, `plot`; sözleşmede alanları yok), katman durumlarının dışa ve içe aktarımı, stil
kitaplığının (sembol, varlık, şablon) komutları (`CAD-STYLE-14`), tanımlı koordinat sistemi, datum seçimleri, Ölçme, Değişkenler,
topoloji ve açıklama yükseklikleri ayarları, kullanıcı ve cihaz ayarları (`CAD-PY-06`'nın kalanı), ortak changeset (`CAD-CMD-14`).

### 2. `cad.layers.edit` v1

Girdi `LayersEdit` (`crates/shared/contracts/src/cad_layer_tree.rs`): `operation` ve işleme göre `layer`, `layers`, `blocks`, `kind`,
`name`, `parent`, `index`, `color`, `lineType`, `lineWeight`, `visible`, `locked`, `activate`, `fields`, `renames`, `expectedRevision`.

- `add`: yeni katman ya da grup. Ad verilmezse “Yeni katman” ya da “Yeni grup”, ağaçta bir tane olacak biçimde (“Yeni katman 2”).
  `parent` bir grupsa içine, katmansa onun grubuna, yoksa en üste; `index` gruptaki yeri (üstte 0), verilmezse sona. Girdiği grup
  açılır (görünüm; geri almada açık kalır). Varsayılan stil `fg`, sürekli, 0,18 mm. `activate` yeni katmanı aynı adımda etkin yapar;
  geri alma eskisini geri verir (ADR 0076). Adım “Katman ekle” ya da “Grup ekle”. Aynı adlı ikinci katman reddedilmez: ad belgenin
  kuralında tekil değildir.
- `style`: renk (`#rrggbb` ya da `fg`, `fg-dim`, `ink`, `paper`), çizgi tipi, kalınlık (0–100 mm). Kilitli düğüm de alır (görünüştür,
  `cad.layers.renderer` gibi). Adım tek alan değiştiyse onun adıyla (“Katman rengi”, “Çizgi tipi”, “Çizgi kalınlığı”), birden
  çoksa “Katman stili”: Katmanlar panelinin adları.
- `move`: düğüm altındakilerle bir gruba (`parent`; verilmezse en üst) ve yerine (`index`, düğüm yerinden çıktıktan sonraki sıra;
  verilmezse sona). Nesneler katmanlarında, etkin katman yerinde kalır. Adım “Katmanı taşı”. Belgeye yeni bir geri alma işlemi
  eklendi (`Op::LayerMove`, `Document::move_layer`): eski yer ve yeni yer; geri alma eski yere koyar.
- `remove`: ADR 0072'nin kuralı ve sözleriyle (son katman, etkin katman, etkin katmanı içeren grup, kilitli düğüm reddedilir).
- `duplicate`: ADR 0177 §3: aynı gruba, sona, “<ad> kopyası” (tekil), stili, görünürlüğü ve alanlarıyla, kilitsiz; nesnelerin
  kopyaları `cad.entities.create` ile aynı adımda. Kopya gizli katmana yazılırsa `layer_hidden` uyarısı geçer.
- `merge`: hedef `layer`, kaynaklar `layers` (hedefin kendisi ve tekrarlar atlanır). Kaynakların nesneleri `cad.entities.set` ile
  hedefe, kaynaklar silinir; tek adım “Katmanları birleştir”. Etkin katman kaynaksa hedef, adımın dışında etkin olur (ADR 0177 §3).
- `fields`: ADR 0199 §3 (`Document::set_layer_fields`); `renames` nesnelerin anahtarlarını aynı adımda değiştirir.
- `purge`: ADR 0177 §5'in kuralı katmanlara, gruplara ve bloklara (`layers`: katman ve grup kimlikleri, `blocks`: blok kimlikleri;
  verilmezse bulunanların hepsi, kilitli boş katman hariç). Kalanların kullandığı kalır ve `kept`'te sayılır. Projenin sembolleri ve
  varlıkları kitaplıktadır, geri alma adımı değildir (ADR 0092): Kullanılmayanları temizle penceresi onları kendisi siler.
  **Kural düzeltmesi:** servisten çizilen katman (altlık) nesne tutmaz ama kullanılır: artık temizlenecekler arasında değildir
  (`layer_purge::removable`, bağımsız başvuru `layer_purge_cases.py`'ye yeni durum). Önceden altlıklar “Boş katmanlar” arasında
  listeleniyordu (ADR 0177 servis katmanlarından önceydi).

Denetimlerin sırası (`CommandError.code`): `no_layer`, `no_layers`, `empty_name`, `nothing_to_set`, `invalid_color`,
`invalid_line_weight`, `invalid_fields`; `invalid_revision`, `revision_conflict`; `layer_not_found` (`layer`, `parent`, `layers[i]`),
`not_a_group`, `move_into_itself`, `not_a_layer`, `service_layer`, `layer_locked`, `unknown_block`, `layer_refused` (belgenin
reddi, kendi sözleriyle). Hiçbir şey değişmezse yazılmaz (`changed` false). Çıktı `LayersEdited`: `layer` (eklenen, kopyalanan,
değişen, birleşmenin hedefi), `changed`, `objects` (silinen, kopyalanan, taşınan), `removed`, `blocks`, `kept`, `revision`.

### 3. `cad.layers.set` v1

`rename` (`layer`, `name` kırpılır), `visible` ve `locked` (`layer`, `value`), `active` (`layer`: nesne alan katman; grup ve servis
katmanı reddedilir), `isolate` (`layers`: bu düğümler, üstlerindeki gruplar ve altlarındakiler görünür, gerisi gizlenir), `showAll`.
Düzenlemedir (çizim kirlenir, sürüm değişir), geri alma adımı değildir (ADR 0020); etkin katman dosyada saklanır ama düzenleme
değildir. Belgeye `set_layer_locked` eklendi (önceden yalnız `toggle_layer_locked` vardı). Retler: `no_layer`, `no_layers`,
`empty_name`, `no_value`; sürüm; `layer_not_found`, `not_a_layer`, `service_layer`. Çıktı `LayersChanged`: `changed`, `active`,
`revision`. **Karar (`CAD-STYLE-02`):** göz, kilit ve etkin katman bugün projenin verisidir (dosyada saklanır); ayrı bir “geçici
görünüm” katmanı yoktur. Dondurma ve çizdirme bayrakları ayrı bir sözleşme değişikliğidir.

### 4. `cad.layers.states` v1

ADR 0177 §4'ün kuralları (`kentos_domain::layer_states`): `save` (`name` verilmezse ilk boş “Durum n”, kimliği ilk boş `durum-n`;
`locks` verilmezse kapalı, `styles` açık), `update` (verilmeyen parçalar durumun sakladığı), `rename`, `remove`, `apply`. Adlar
baştaki ve sondaki boşluklar sayılmadan tekildir. Durumlar proje ayarıdır: kaydetme, güncelleme, adlandırma, silme düzenlemedir,
adım değildir. Uygulama görünürlüğü ve kilitleri ağacın kendi değişikliği olarak, stilleri tek adımda (“Katman durumu: <ad>”) yazar.
Masaüstündeki ad kuralı ve kimlik sayacı belgeye taşındı (`next_state_name`, `next_state_id`, `name_refused`). Retler: `no_state`,
`empty_name`; sürüm; `state_not_found`, `duplicate_name`. Çıktı `LayerStatesDone`: `state`, `changed`, `visible`, `locked`, `styles`,
`missing`, `revision`.

### 5. `cad.styles.edit` v1 ve geri alma politikası (`CAD-CMD-15`)

`kind` (`text`, `dimension`) ve `operation`: `create` (tanımın kimliği boşsa yeni bir UUIDv7; verilen kimlik tabloda olmamalı),
`update` (tanımın kimliği stili adlandırır; yeri korunur), `rename` (`style`, `name`), `remove` (`style`). Sonuçtaki tablo ADR
0183'ün kuralıyla denetlenir (`text_styles_problem`, `dimension_styles_problem`: adlar büyük küçük harf ayırmadan tekil, Standart
ayrılmış, değerler sınırlarında). Yazma Yazı ve Ölçü stilleri pencerelerinin yoludur (`style_tables::save_text_styles`,
`save_dimension_styles`; `crates/native/interaction`'dan `crates/native/application`'a taşındı): tablo projenin ayarı olur, stilin
nesneleri eski değeri hâlâ taşıyan alanlarında yenisini alır, silinen stilin nesneleri görünüşünü korur ve bağını yitirir; kilitli
katmandakiler yazılmaz ve `locked`'da sayılır. **Düzeltme:** değişmeyecek nesne artık yazılmaz; önceden yeniden adlandırma bile
stilin bütün nesneleriyle boş bir “Yazı stili” adımı bırakıyordu.

Retler: `no_definition`, `no_style`, `empty_name`; sürüm; `style_not_found`, `duplicate_style`, `invalid_style`. Çıktı
`StylesEdited`: `style`, `changed`, `followed`, `locked`, `revision`.

**Geri alma politikası.** Bugün iki belge de (web ve masaüstü) proje ayarlarını ve stil tablolarını geri alma adımına koymaz;
nesnelerin izlemesi ise adımdır. Bu ayrık davranış bu maddede **korundu**:

- ayarın geri alınabilir olması, iki belgenin geçmişine yeni bir işlem türü (ayarların önceki ve sonraki hâli) ve ortak belge
  durumlarına (`fixtures/document-ops/v1/project.json`) yeni beklentiler ister; web dururken (ADR 0228) yalnız masaüstünde yapmak
  iki belgenin davranışını ayırırdı;
- bulutta proje ayarları `project.changes`'in ayrı bir parçasıdır; bir adımda ayar ve nesne birlikte geri alınınca sunucuya giden
  değişikliğin biçimi (`CAD-CMD-14`'ün ortak changeset'i) henüz tasarlanmadı.

Göç yolu (ayrı bir maddede): belgeye `Op::Settings { before, after }`; ayar komutları ve stil komutu tek adım yazar (tablo ve
izleyen nesneler birlikte); `CommandUndo::Step`; ortak belge durumlarında `setSettings` adım olur; web'in `CadDocument`'i aynı
işlemi alır; `.kcad` değişmez (geçmiş dosyaya yazılmaz). O zamana dek komutların açıklamaları ve çıktıları hangi kısmın adım
olduğunu söyler.

### 6. `cad.settings.set` v1

Yalnız projenin kendi ayarları, anahtarla; verilmeyen kalır; bilinmeyen anahtar şemada yoktur (`deny_unknown_fields`) ve reddedilir:
tema, grafik, kısayol kullanıcının ve cihazındır.

| Anahtar | Kural |
|---|---|
| `name` | çizimin adı, kırpılır, boş olamaz |
| `projectType` | `cad` ya da `gis` (`plan3d`, `disaster` henüz seçilemez) |
| `srid` | kayıttaki EPSG kodu ya da 0 (yerel); **atanır, koordinatlar dönüştürülmez** (CLAUDE.md §5); tanımlı sistemin yerini alır; sistem birimi metre yapar |
| `secondSrid` | kayıttaki, projeninkinden başka bir kod ya da 0 (yok); tanımlı ikinci sistemin yerini alır |
| `drawingUnit` | yalnız sistemsiz yerel projede `mm`, `cm`; `m` yazılmaz |
| `lengthDecimals`, `areaDecimals` | 0–4 |
| `areaUnit`, `angleUnit` | `m2`, `donum`, `ha`; `grad`, `deg` |
| `plotScale` | 1–1 000 000 tam sayı payda; eski genel yükseklikteki açıklamalar yenisini tek adımda alır (“Yazı yüksekliklerini uydur”, ADR 0205 §3) |
| `drawingFont` | çizimin yazı tipi |

Sistemin kodu koordinat sistemleri kaydında aranır (`fixtures/crs/v1/registry.json`, proje modelinin `kentos_project::crs`'inin de
okuduğu tek veri dosyası); komutların crate'i proje modeline bağlanmaz, çünkü stil ve çizim hattı komutların crate'ini kullanır ve
proje modelini kullanamaz (`scripts/arch/deps.mjs`). Açıklamaların izlemesi (`annotation_scale`) de etkileşimden komutların
crate'ine taşındı. Retler: `nothing_to_set`, `empty_name`, `invalid_project_type`, `invalid_decimals`, `invalid_scale`,
`unknown_srid`; sürüm; `second_needs_system`, `second_is_own`, `unit_needs_local`. Çıktı `SettingsChanged`: değişen anahtarlar
(belgelenen sırayla), `followed`, `locked`, `revision`.

### 7. Masaüstü: arayüz aynı komutlardan yazar

Düz değişimler (adımların adları ve iletiler aynı):

- Katmanlar paneli: Yanına yeni katman, Yeni katman, Yeni grup (`add`), renk, çizgi tipi, kalınlık (`style`), Sil (`remove`),
  göz, kilit, Yeniden adlandır, Etkin katman yap, çift tık, Yalnızca bunu göster, Hepsini göster (`cad.layers.set`);
  `apps/desktop/src/layer_commands.rs` üç komutu çalıştırır, reddi komutun sözleriyle söyler.
- Kopyasını oluştur ve Katmanları birleştir (`duplicate`, `merge`; akış kabuktan komuta taşındı).
- Kullanılmayanları temizle: katmanlar, gruplar ve bloklar `purge` ile, adlarıyla; sembol ve varlıklar kitaplıktan (değişmedi).
- Katman durumları penceresi ve Katmanlar'ın Katman durumları ▾'i (`cad.layers.states`).
- Alanlar penceresi (`fields`; reddi pencerede).
- Katman araçları (Katmanı gizle, yalıt, kilitle, etkin yap) ve Yalıtımı kaldır (`cad.layers.set`, `kentos_interaction::layer_tools`).
- Şeridin Ölçek'i ve Ölçek yaz… (`cad.settings.set`'in `plotScale`'i).

Pencere yolu korunanlar: Yazı ve Ölçü stilleri penceresi tabloyu bütün olarak kaydeder; komutla aynı işlevi
(`style_tables::save_*`) çağırır. Proje ayarları penceresi taslağın bütününü (tanımlı sistem, datum seçimleri, Ölçme, Değişkenler)
`Document::set_settings` ile yazar; komut aynı işlevle ve aynı izlemeyle yazar. Bulut projesinde ağacı değiştirme yetkisi
(`tree_locked`, ADR 0078) arayüzde denetlenmeye devam eder; başsız sunucu yerel çizimdir.

### 8. Performans

Komutlar anlıktır (`CommandCost::Instant`). Plan, okuma kopyasında (`Document::reading_copy`: nesneler paylaşılır, kopyalanmaz)
aynı yazmayı yapar; çizim değişmez. Ağır olan yalnız birleştirme ve kopyadır (nesne sayısıyla doğrusal; `cad.entities.set` ve
`.create`'in kendi yolları); temizlik ADR 0177 §5'in kuralıdır. Bütçe ADR 0177'ninkidir; yeni ölçüm gerekmedi.

## Uygulama

- **Sözleşme:** `cad_layer_tree.rs` (`LayersEdit`, `FieldRename`, `LayersEdited`, `LayersSet`, `LayersChanged`, `LayersStates`,
  `LayerStatesDone`, `layer_color_ok`), `cad_styles.rs` (`StylesEdit`, `AnnotationStyleKind`, `StylesEdited`), `cad_settings.rs`
  (`SettingsSet`, `SettingsChanged`, `MAX_SHOWN_DECIMALS`, `MAX_PLOT_SCALE`); katalogda beş tanım (masaüstü); TS türleri ve
  `commandCatalog.json` üretildi.
- **Belge (`kentos_domain`):** `move_layer` ve `Op::LayerMove` (`LayerTree::move_place`, `move_node`), `set_layer_locked`,
  `layer_purge::object_of`, `source_of` ve servis katmanının kuralı, `layer_states::next_state_name`, `next_state_id`,
  `name_refused`.
- **Komutlar (`kentos_native_application`):** `layers_edit.rs` (ve `new_object`, etkileşim onu yeniden dışa aktarır), `layers_set.rs`,
  `layers_states.rs`, `styles_edit.rs`, `settings_set.rs`; `style_tables.rs` ve `annotation_follow.rs` etkileşimden taşındı (etkileşim
  `style_tables` ve `annotation_scale` adlarıyla yeniden dışa aktarır); `said_of`; yeni kodlar `codes.rs`'te; `DESKTOP_COMMANDS`.
- **Başsız sunucu, Python, MCP:** `dispatch.rs`; `scripts/python/sdk.py` ile `kentos.cad.layers`, `kentos.cad.styles`,
  `kentos.cad.settings`; MCP'de beş araç `destructiveHint` ile, uçtan uca testi (`layers_styles_and_settings_go_through_their_tools`:
  katman ekle, adlandır, kilitle, stil, ölçek; `theme` anahtarının reddi).
- **Masaüstü:** `layer_commands.rs`, `layering.rs`, `layer_tree.rs`, `app.rs`, `layer_merge.rs`, `layer_purge.rs`, `layer_states.rs`,
  `layer_fields.rs`, `annotation_scale.rs`; etkileşimde `layer_tools.rs`.
- **Manifest:** `catalog/v1/operations.json`'da on beş komut ve dört araç `semantic` (`via` ile), `workspace.*` kuralı
  `cad.settings.set`'e; `scripts/cad/manifest.py` katman komutlarının işlemlerini sözleşmeden okur; `docs/cad/manifest.{json,md}`.
- **Ortak durumlar:** `fixtures/commands/v1/cad.layers.edit.json` (59), `cad.layers.set.json` (25), `cad.layers.states.json` (20),
  `cad.styles.edit.json` (25), `cad.settings.set.json` (28); iki koşucuya (masaüstü, Python) üç beklenti eklendi: `activeLayer`,
  `name`, `settings` (yalnız verilen anahtarlar; yazılmayan boş liste `null`).

## Doğrulama

- **Takım** (12 Ekim, Apple M5, macOS): `pnpm rust:test` 3343 test geçti, 65 ayrık (clippy `-D warnings` ve bağımlılık yönü
  temiz); `pnpm rust:test:desktop` 1428 geçti, 213 ayrık (clippy temiz); `pnpm py:test` 66 test (ortak komut durumları dahil);
  `cargo test -p kentos-mcp` 13. Yeni ortak durumlar 157 (59 + 25 + 20 + 25 + 28), ikisi de (masaüstü ve Python) geçer.

- **Bağımsız başvurular**, KentOS kodu olmadan: `scripts/fixtures/layer_tree_command_cases.py` (ağaç, nesneler ve durumlar düz JSON
  olarak; temizliğin kuralı ADR 0177 §5'ten yeniden yazıldı; alanların kuralı `layer_field_cases.py`'den), `style_edit_command_cases.py`
  (tabloların kuralı ve izleme `annotation_style_cases.py`'nin bağımsız başvurusundan), `settings_set_command_cases.py` (kayıt
  `fixtures/crs/v1/registry.json`'dan, veri; açıklamaların genel yüksekliği ADR 0205 §1'den), `layer_purge_cases.py`'ye altlık
  durumu. Hepsi `--check` ile.
- **Testler:** `cargo test -p kentos-native-application` (ortak durumlar dahil), `kentos-domain` (`layer_rules`: temizliğin yeni
  durumu), `kentos-contracts` (katalog), `kentos-interaction`, `kentos-headless`, `kentos-mcp`, `pnpm py:test` (ortak durumlar Python'un
  tipli katmanından), `pnpm rust:test:desktop` (katman pencerelerinin testleri ve izler: `layer-*.json`, `layer-states.json`,
  `layer-purge.json`, `layer-merge.json`, `layer-by-object.json`).
- **Resimler:** `cargo test -p kentos-desktop layering::screens -- --ignored --nocapture`'ın yeni iki sahnesi `komutlar` (Yanına
  yeni katman, renk, kesikli, 0,5 mm, Duvar adı, kilit, Yeni grup; hepsi komutlardan) ve `durumlar` (iki durum kaydedilip biri
  uygulanmış Katman durumları penceresi), 1440×900 ve 1100×650, koyu ve açık: `.run/shots/katman-secimi-komutlar-*`,
  `katman-secimi-durumlar-*`.
- **Ölçülmeyen:** yeni süre ölçümü yok (§8).

## Sonuçlar

- Python ve MCP arayüzün katman, stil ve ayar işlerini aynı kurallarla ve aynı geri alma adımlarıyla yapar; manifestin `none`
  satırları 108'den 86'ya indi, CAD tarafının boşlukları 243'ten 225'e.
- Kalan işler: ayarların ve stil tablolarının geri alınabilmesi (§5'in göç yolu, `CAD-CMD-15`'in uygulaması, `CAD-STYLE-13`) ve
  ortak changeset (`CAD-CMD-14`); dondurma, çizdirme ve viewport başına katman bayrakları (`CAD-STYLE-02`'nin kalanı, `CAD-STYLE-12`);
  katman durumlarının dışa ve içe aktarımı (`CAD-STYLE-04`'ün kalanı); stil kitaplığının komutları (`CAD-STYLE-14`); kullanıcı ve
  cihaz ayarlarının ayrı bir komutu ve tanımlı sistem, datum seçimleri, Ölçme, Değişkenler, topoloji ve açıklama yükseklikleri
  anahtarları (`CAD-PY-06`'nın kalanı); Proje ayarları penceresinin anahtar anahtar komuttan yazması.
- Web dönüşte beş komutun işleyicilerini, iki koşucunun yeni beklentilerini ve temizlik kuralının düzeltmesini alır (TODOS.md §0.2).

## Ek (19 Ekim 2026): katmanların sırası ve sağ tık menüsü

Sahibin bildirimi: katmanın sağ tık menüsü çok uzamıştı ve katmanlar taşınamıyordu.

- **Menü ilgisine göre alt menülerde:** üstte sık işler (Etkin katman yap, Gizle, Kilitle, Yalnızca bunu göster, Nesnelerini
  seç, Katmana yakınlaştır), sonra **Görünürlük ▸** (Tüm katmanları göster, Dondur, Bütün katmanları çöz, Yazdır), **Görünüş ▸**
  (renk, çizgi tipi, kalınlık, Katman stili, Etiketler, SLD, Değişikliğin etkisi), **Veri ▸** (Alanlar, Formlar, Bilgi kartı,
  Süzgeç, Zaman ayarları, İstatistik, Grafik), Kaynak ▸, Kenet ▸, Senaryo ▸, **Sıra ▸**; altta Yeniden adlandır, yeni katman,
  Kopyasını oluştur, birleştir, Sil.
- **Sıra ▸:** Yukarı taşı, Aşağı taşı, En üste taşı, En alta taşı, Gruptan çıkar (grubun hemen ardına). Her biri
  `cad.layers.edit`'in `move`'u, tek geri alma adımı; kilitli düğüm komutun sözüyle reddedilir (`layer_order.rs`).
- **Sürükleme:** Katmanlar ağacında satır bir satırın önüne, ardına ya da (grupsa) içine bırakılır; sanal satırlı ağaçta da
  (KentOS UI `TreeView`'in tembel satırları, kenarda kendiliğinden kaydırma). Bırakılan yer `move`'un dizinine çevrilir.
- Doğrulama: `cargo test -p kentos-desktop layer_order`; resimler `KENTOS_SHOTS_ONLY=katman-menu-sira,katman-surukle,katman-tasindi
  cargo test -p kentos-desktop tools_screens -- --ignored --nocapture`.
