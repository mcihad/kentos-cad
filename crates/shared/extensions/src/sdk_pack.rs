//! `kentos://sdk/ai-kek-pack` (plan/eklenti.md ADIM 21; docs/adr/8057 §20): everything an agent needs to write a
//! KentOS extension, in one read: how the SDK works, the capabilities and what each covers, the contributions with
//! their decorators and manifest keys, an İşlemler tool descriptor's parameter and output types, what `verify`'s codes
//! mean, the steps from an ask to an installed package, and a blueprint with every contribution as the working example.
//! Built from the code it describes (the capabilities, the modules, the blueprint), so it does not drift from it.

use serde_json::{Value, json};

use crate::blueprint::{Ask, Contribution, blueprint};
use crate::manifest::{Capability, Module};

/// The SDK as an agent reads it first (`kentos.ext`).
pub const GUIDE: &str = r#"# KentOS eklentisi yazmak (`kentos.ext`)

Bir eklenti `.kek` paketidir: `manifest.json`, `src/` altında Python, bir İşlemler aracının tanımı (`tools/*.tool.json`),
`pyproject.toml` ve `uv.lock`. KentOS kodu içe aktarmadan tanır; bir işlev yalnız çağrıldığında, eklenti sürümünün kendi
sürecinde ve kendi Python ortamında çalışır. Ayrı süreç bir korumalı alan değildir: eklenti kullanıcının haklarıyla
çalışır, KentOS yalnız çizimden ne verdiğini sınırlar.

`from kentos import ext` (çizime yazan komut ayrıca `from kentos import cad`); bütün imzalar `api.stub`'da.

## İşaretler (her biri manifestteki bir giriş noktasının yerel kimliğiyle)

- `@ext.command("kimlik")` → `entrypoints.commands`: `def f(ctx: ext.CommandContext) -> ext.CommandResult`.
  `ctx.doc` açık çizimdir (`document.read`/`document.write` verildiyse; yoksa ya da çizim açık değilse `None`):
  `ctx.doc.layers()`,
  `ctx.doc.entities(layer=…)`; yazmak `kentos.cad` komutlarıyla (`cad.entities.set(ctx.doc, uids=[…],
  operation="attributes", attrs={…})`); `ctx.doc.select(uidler)` bulduğunu seçili bırakır. Bir çalıştırmanın bütün
  yazmaları tek geri alma adımıdır; hata veren ya da durdurulan çalıştırma geri alınır.
- `@ext.processing("kimlik")` → `entrypoints.processing` + tanım dosyası: `def f(ctx: ext.ProcessingContext,
  <parametreler adlarıyla>) -> dict`. Nesne girdileri `ext.FeatureInput`'tur (`ctx.read_features(girdi)` →
  `ext.Feature`: `uid`/`id`, `kind`, `layer`, `attrs`, `geometry` GeoJSON, `__geo_interface__`). Çizim yoktur; çıktılar
  tanımdaki adlarıyla bir sözlükte döner (`features`: `ext.NewFeature(attrs=…, geometry=…)` ya da girdinin şekliyle
  `ext.NewFeature(source=f, attrs=…)` listesi; `table`: satır sözlükleri ya da DataFrame; `number`; `string`). Şekil
  `ext.point/polyline/polygon`, GeoJSON ya da `__geo_interface__` taşıyan her şeydir (shapely, GeoPandas).
  `ctx.summary(metin)` geçmişteki tek satırı, `ctx.select(nesneler)` bitince seçili kalanları söyler.
- `@ext.audit("kimlik")` → `entrypoints.audits`: `def f(facts: ext.GisAuditFacts) -> list[ext.AuditFinding]`.
  `facts.layers()`, `facts.features(katman_kimliği_ya_da_adı)`, `facts.all_features()`, `facts.group_related(katman,
  foreign_key=…)`, `facts.cut`. Kural yazamaz; bulgu `ext.AuditFinding(message, severity="error"|"warning"|"info",
  feature_id, fix=ext.SemanticFix(title, command, args))`. Düzeltmeyi kullanıcı uygular (tek geri alma adımı; yalnız
  `document.write` izinli eklentide).
- `@ext.expr("kimlik")` → `entrypoints.expressions`: düz işlev, değer alır değer döndürür. İfadelerde eklenti
  kimliği ve yerel kimlikle çağrılır: `ext.tr.ornek.vitrin.ada-parsel(ada, parsel)` (etiket, stil kuralı, süzgeç, form
  hesabı, İşlemler). Çizim beklemez: yanıt gelince yeniden çizilir.
- `@ext.map_tool("kimlik")` → `entrypoints.mapTools` (`capture`: point, line, polygon, rectangle,
  orientedRectangle, circle): `def f(ctx: ext.MapToolContext, sekil: ext.Shape) -> ext.CommandResult`; `ctx.add_feature(katman, sekil,
  öznitelikler)`. Şekli KentOS çizer (kenetleme, yazılan koordinat), kod yerleştirir.

## Katman şeması (`schema.json`)

Eklentinin katmanları `schema.json`'da bildirilir (manifestte `"schemaFile": "schema.json"`) ve bir komut onları açık
çizime kurar: `@ext.command("katmanlari-kur") def f(ctx): return ctx.apply_declarative_schema("schema.json")`
(`document.write` ister). Biçim `kentos.layerSchema` 1, sözcükler KentOS'un:

- `layers`: her biri `name` (çizimdeki **kimliği**, `layer.id`: küçük harf, rakam, `_`; harita aracı ve denetim
  katmanı bununla bulur: `ctx.add_feature("mezar_yeri", …)`, `facts.features("mezar_yeri")`; bir komut
  `ctx.doc.layer("mezar_yeri")` ile, nesnelerini `ctx.doc.entities(layer="mezar_yeri")` ile okur), `title` (görünen
  adı, `layer.name`: "Mezar yeri"; katman ağacında adla aramak kimliği bulmaz), `geometry` (point, line, area,
  annotation, mixed; verilmezse her tür), `fields`: `{name, kind, alias, required, default, length, min, max, domain}`
  (kind: text, integer, decimal, date, boolean, datetime, time, bigint, instant, uuid, json, binary); isteğe bağlı `form`:
  paketteki `.kform` (manifestin `entrypoints.forms`'unda bildirilir), kurulumda katmana katmanın adıyla bağlanır.
- Katmanın isteğe bağlı `subtypes`'ı (alt türler): `{field, types: [{code, label, fields: [{field, values, default,
  required}]}]}`: `field` değer listeli, tek değerli bir alan; her alt tür onun bir kodu ve öbür alanları o türün
  kayıtları için daraltır (sunulan kodlar, yeni kaydın varsayılanı, zorunluluk). `contingents`'ı (değer bileşimleri):
  `[{name, fields: [2–8 değer listeli alan], restrictive, rows: [{cells: [{kind: "code", code} | {kind: "any"} |
  {kind: "empty"}], subtype}]}]`: birlikte geçerli değerler; kısıtlayıcı grupta başka bileşim reddedilir. KentOS
  kuralları her yazmada uygular, formun ve tablonun seçenekleri daralır. Grubun bir alanı doluyken öbür hücreler de
  bir satırca karşılanmalı: alanlar tek tek doldurulacaksa boş hücreli satırlar ekleyin (`[{kind: "any"}, {kind:
  "empty"}]` ve tersi), yoksa öznitelik tablosunda ilk yazılan alan reddedilir (inceleme bunu uyarır).
- `group` (isteğe bağlı): `{id, name}`, katmanlar altına girer. `domains`: `{id, name, kind, values: [{code, label}]}` ya
  da `min`/`max`. `relations`: `{id, name, parent, child, field, onDelete}`: çocuğun alanı ebeveynin kalıcı kimliğini
  tutar (onDelete: keep, unlink, cascade, restrict).
- Var olana dokunmaz, yalnız eksiği ekler (katmanın eksik alanlarını da); ikinci çalıştırma bir şey değiştirmez. Şema
  paket incelemesinde denetlenir: her sorun JSON işaretçisiyle (`layer_schema`).

## Panel (`ui/<id>.panel.json`)

Panel bildirimseldir; kodu bir arayüz nesnesi almaz. Manifestte `entrypoints.panels` (`id`, `title`, `side`: right
Katmanlar'ın yanında, left haritanın solunda, bottom altında; kullanıcının taşıdığı yer korunur, `definition`),
şeritte `{"panel": "<id>"}`, yetenek `ui.panel`. Biçim `kentos.ui.panel` 1: `title` ve `body`
bileşenleri: `text` (`text`), `textField` (`id`, `label`, `placeholder`, `value`), `numberField` (`id`, `label`,
`min`, `max`, `unit`, `value`), `dateField` (`id`, `label`, `value`, `min`, `max`: YYYY-AA-GG; kullanıcı GG.AA.YYYY ya
da YYYY-AA-GG yazar ya da takvimden seçer, değeri ISO tarih metni ya da boşsa None), `timeField` (`id`, `label`,
`value`, `min`, `max`: SS:DD; kullanıcı 14:30, 14.30 ya da 9:05 yazar ya da saat seçicisinden seçer, değeri SS:DD ya da
None), `dateTimeField` (`id`, `label`, `value`, `min`, `max`: YYYY-AA-GGTSS:DD, yerel saat; kullanıcı 15.11.2026 14:30
yazar ya da takvim ve saatten seçer, değeri 2026-11-15T14:30 ya da None), `select` (`id`, `label`,
`options: [{value, label}]`, `value`), `multiSelect` (`id`, `label`, `options: [{value, label}]`, `value`: başta
seçili değerlerin listesi; her seçenek bir çip, basınca seçilir ya da bırakılır; değeri seçilenlerin listesi, seçenek
sırasıyla, hiçbiri seçilmezse boş liste), `checkbox` (`id`,
`label`, `value`), `layerPicker` (`id`, `label`, `geometry`: point, line, area, annotation; açık çizimin katmanları,
değeri katmanın kimliği ya da seçilmezse None), `fieldPicker` (`id`, `label`, `layer`: bu paneldeki bir `layerPicker`'ın
kimliği, `kinds`: alan türleri, ör. `["integer", "decimal"]`; orada seçilen katmanın alanları, değeri alanın adı ya da
None), `featurePicker` (`id`, `label`, `layer`: bir `layerPicker`'ın kimliği, `geometry`, `multiple`: varsayılan true;
kullanıcı "Seçimi al"a basınca çizimdeki seçimden o katmanın ve türün nesnelerini alır, değeri kalıcı kimliklerin
listesi, `multiple: false` ise tek kimlik ya da None; kod her birini `ctx.doc.entity(uid)` ile okur; katman değişince
alan ve nesne seçimi temizlenir), `button` (`id`, `label`, `command`: bir `entrypoints.commands` kimliği, `table`,
`primary`), `table` (`id`, `label`), `tree` (`id`, `label`, `group`: 1–3 sütun adı; düğmenin satırlarını bu sütunlara
göre açılıp kapanan gruplara dizer, her grubun sayısını yazar; yaprağın adı satırın ilk diğer sütunu, `uid`'li yaprak
tıklanınca nesneyi gösterir; düğmenin `table`'ı bir ağacı da gösterebilir), `card` (`id`, `label`, `title`, `tag`:
sütun adları; her satır bir kart: `title` sütunu başlığında (yoksa ilk sütun), `tag` sütunu yanında, diğer sütunlar
altında adlarıyla; dar yerde sığmayan çok sütunlu tablonun yerine; `uid`'li kart tıklanınca nesneyi gösterir;
düğmenin `table`'ı bir kartı da gösterebilir), `stats` (`id`, `label`, `title`, `value`, `unit`, `note`: sütun adları;
her satır bir gösterge kartı: `title` sütunu adı (yoksa ilk sütun), `value` sütunu sayısı (yoksa ikinci), `unit` birimi,
`note` altındaki notu; kartlar yan yana dizilir; özet sayıları için), `chart` (`id`, `label`, `kind`: bar (varsayılan),
pie, histogram, line; `category`: gruplanacak ya da yatay eksendeki sütun, `value`: sayı sütunu, `aggregate`: count
(varsayılan; `value` verilirse sum), sum, mean, min, max, median; çubuk ve pasta `category` ister, histogram `value`,
çizgi ikisini; satırları düğmenin `table`'ından ya da `of` ile bu paneldeki bir tablonun, ağacın, kartın ya da
göstergenin satırlarından; KentOS'un grafik motoru çizer). Metin, sayı, tarih, saat ve an alanları, çoklu seçim ve katman/alan/nesne seçicileri `required: true` alır: boş
bırakılan komut çalışmadan adıyla söylenir, sözler alanın altında ("“Önek” boş bırakılamaz"), adının yanında yıldız.
`select`, `multiSelect`, `layerPicker`, `fieldPicker`, `featurePicker`, `dateField`, `timeField`, `dateTimeField` (seçiciden seçilince ya da Enter'la) ve `checkbox` isteğe
bağlı `onChange` alır: değer değişince o komut panelin değerleriyle çalışır, satırları panelin ilk tablosuna gelir
(çalışırken gelen değişiklik bitince bir kez yeniden çalıştırılır). Düğme komutu çalıştırır: panelin değerleri `ctx.params`'tadır (kimlikleriyle: metin, sayı ya
da boşsa None, seçimin değeri, onay true/false); komutun `ctx.report(satırlar, message=…)` sonucu panelin tablosunu
doldurur (boş liste de bir cevaptır: tablo boşalır; `ctx.message` tabloya dokunmaz) (`ctx.doc.entities(layer=…)` kayıt verir: `kayit.uid` kalıcı kimlik, öznitelikler `kayit.entity.attrs`'ta,
kayıtta değil). Bir satırda `uid` (nesnenin kalıcı kimliği, `kayit.uid`) varsa satıra tıklamak nesneyi seçip yakınlaştırır;
`uid` sütun olarak görünmez. Panel Python'suz açılır; ortam hazır değilse düğmeler nedenini söyler. İskelette
`panel` katkısı bir örnek verir. Bileşen kuralı Python'suz çalışır, formun kurallarının ifade diliyle panelin
girdilerini kimlikleriyle okur: `visibleWhen` (her bileşen), `requiredWhen` (zorunlu olabilen girdi), `enabledWhen`
(düğme ve metin, sayı, tarih, saat alanları), ör. `"visibleWhen": "içerir(bilgiler, 'tarih')"`; tireli kimlik köşeli
parantezle (`[en-sik]`). Hesaplı değer de Python'suz: `calculate` (metin; metin, sayı, tarih, saat, an alanı) değeri
okuduğu girdiler değiştikçe yeniden hesaplar, alan yazılmaz ve kod onu değiştiremez, metinde `text` hesap boşken görünür;
`defaultExpr` (alanlar) ilk değeri panel açılırken hesaplar (`"defaultExpr": "bugün()"`); ikisi `bugün()` ve `şimdi()`'yi
okur, birbirini döngüyle okuyan hesaplar Kapı 1'de reddedilir. Katmanın sağ tık menüsünden (`contextMenus`, `target: "layer"`) çalışan komut tıklanan
katmanın kimliğini `ctx.params["layer"]`'da alır. Panelden çalışan komut panelini çalışırken değiştirir (`ctx.panel`;
panelden çalışmadıysa None; başka bir panelini `ctx.open_panel(id)` açıp verir): `set(id, value=…, options=…, enabled=…, visible=…, text=…)` (seçenekleri veriden doldurmak, düğmeyi
kapatmak, alanı gizlemek), `rows(id, satırlar)`, `say(metin, kind)`, `focus(id)`, `close()`; yanlış kimlik ya da değer
`kentos.HostError`, kullanıcının o sırada değiştirdiği alan onun kalır (False), kodun koyduğu değer `onChange`'i
çalıştırmaz.

## İkonlar

Bir düğmenin, komutun ya da harita aracının ikonu yerleşik bir addır, tire biçiminde (`space-number`, `layer-fields`;
bütün adlar `icons` bölümünde), ya da paketteki 20×20 çizgi SVG'si (`assets/x.svg`: `path`, `rect`, `circle`,
`ellipse`; renk yok, tema rengiyle çizilir). Bilinmeyen ad uyarıdır: düğme düz simgeyle görünür.

## Bağlamın ortak yöntemleri

`ctx.progress(n, toplam, metin)`, `ctx.check()` (Durdur'a basıldıysa `ext.Stopped` atar: uzun döngülerde çağırın),
`ctx.is_stopped()`, `ctx.log(metin, level="info"|"warning"|"error")`, `ctx.report(satırlar, title=…, message=…)`,
`ctx.message(metin)`, `ctx.cancelled()`; kullanıcıya sormak için `ctx.dialog(başlık, alanlar, ok=…, cancel=…)` (alanlar
bir panelin girdileri: textField, numberField, dateField, timeField, dateTimeField, select, multiSelect, checkbox,
layerPicker, fieldPicker, text; düğme ve komut yok; Tamam değerleri kimlikleriyle, İptal None verir, çalışma yanıtı
bekler) ve `ctx.confirm(metin)` (evet: True). Alanları ve seçenekleri elle JSON yerine `from kentos import ui_panel
as ui` ile yazın: bileşen başına bir işlev, gerekli anahtarlar sırayla, kalanı adıyla (`ui.text_field("onek", "Önek",
value="Y-")`, `ui.number_field("ilk", "İlk numara", min=1, required=True)`, `ui.select("tur", "Tür", ["A", "B"])`,
`ui.choice("-", "Seçmeyin")`, bütün panel `ui.panel(başlık, bileşenler)`); panelin şemasından üretilir, yanlış anahtar
`TypeError`. `ctx.params` ve `ctx.settings` sözlüklerdir. Bu KentOS'un yapamadığı bir şey
`ext.NotAvailable` atar: KentOS onu eklentinin hatası değil eksik bir yetenek olarak söyler.

## Kurallar

- Her bildirilen kimliğin tek, doğru türde işaretli işlevi olur; `kentos kek verify` (ajanlarda `extension.verify`)
  her adımda bunu ve paketin kurallarını denetler. İşlemler aracının tanımını (parametreler, çıktılar; yeni nesne veren
  çıktının `sinkParameter`'ı bir `layer` parametresini adlandırır) kurulumdaki gibi tam denetleyen
  `extension.verify`'dır; komut satırındaki `kek verify` yalnız biçimine bakar ve bunu söyler.
- Bir katkı yeteneğini ister ve nedenini yazar (`requestedCapabilities`): kullanıcı kurarken her birini görüp onaylar.
  Ağa çıkan kod `network` ister; işletim sistemi komutu ve `eval`/`exec` uyarıdır.
- Kütüphane `uv add <paket>` ile eklenir (`pyproject.toml` ve `uv.lock` birlikte yazılır); KentOS ortamı kilitten kurar.
- Bir hata eklentinin adıyla söylenir ve iz kaydıyla eklentinin günlüğüne yazılır; kullanıcıya bozuk bir çizim kalmaz.
- Kurma, yetki verme ve yayımlama kullanıcınındır: ajan bunları sessizce geçmez, kurulu paketi ya da izni uydurmaz.
"#;

/// `kentos.ext`'s public surface as a Python stub: every name `__all__` gives, in the file's order; each class with its
/// fields and its public methods and properties, each with its doc's first sentence. Generated from
/// `python/kentos/ext.py` by this module's tests (`KENTOS_WRITE_SDK=1`), which fail when the two differ.
pub const EXT_STUB: &str = include_str!("../sdk/kentos_ext.pyi");

/// `kentos.ui.panel` 1's JSON Schema, made from the panel's types (`ui_panel::json_schema`) and committed beside the stub:
/// a part of its own, as the icons are (the pack stays under what the desktop's Asistan gives a tool's result).
pub const PANEL_SCHEMA: &str = include_str!("../sdk/kentos-ui-panel.schema.json");

/// The manifest v1's JSON Schema, made from the manifest's types (`manifest::json_schema`) and committed beside the
/// stub: a part of its own (docs/adr/8057 §28).
pub const MANIFEST_SCHEMA: &str = include_str!("../sdk/kentos-extension-manifest.schema.json");

/// `schema.json`'s (`kentos.layerSchema` 1) JSON Schema, built from the check's own key lists
/// (`layer_schema::json_schema`) and committed beside the stub: a part of its own (docs/adr/8057 §28).
pub const LAYER_SCHEMA: &str = include_str!("../sdk/kentos-layer-schema.schema.json");

/// An İşlemler tool descriptor's (`kentos.processingTool` 1) JSON Schema, built beside the reader from its vocabularies
/// (`kentos_processing::external::descriptor_schema`, whose test writes it here): a part of its own (docs/adr/8057 §28).
pub const TOOL_SCHEMA: &str = include_str!("../sdk/kentos-processing-tool.schema.json");

/// The pack's parts an agent may ask for alone (`extension.sdk`'s `part`, `kentos kek sdk --part`).
pub const PARTS: [&str; 15] = [
    "guide",
    "api",
    "capabilities",
    "modules",
    "contributions",
    "toolDescriptor",
    "verifyCodes",
    "workflow",
    "example",
    "icons",
    "panelSchema",
    "panelBuilders",
    "manifestSchema",
    "layerSchema",
    "toolSchema",
];

/// One part of the pack; none for a name the pack has not. `icons` is the whole list, which the pack only counts: its
/// 19 KB would take the pack past what the desktop's Asistan gives a tool's result (48 KB).
pub fn sdk_part(part: &str) -> Option<Value> {
    if part == "icons" {
        return Some(json!(crate::icon_names::names().collect::<Vec<_>>()));
    }
    if part == "panelSchema" {
        return serde_json::from_str(PANEL_SCHEMA).ok();
    }
    // `kentos.ui_panel`'s source, made from the schema as the SDK's module is (16 KB, a part of its own).
    #[cfg(feature = "schema")]
    if part == "panelBuilders" {
        return Some(Value::String(crate::ui_panel::python::python_builders()));
    }
    if part == "manifestSchema" {
        return serde_json::from_str(MANIFEST_SCHEMA).ok();
    }
    if part == "layerSchema" {
        return serde_json::from_str(LAYER_SCHEMA).ok();
    }
    if part == "toolSchema" {
        return serde_json::from_str(TOOL_SCHEMA).ok();
    }
    if part == "example" {
        return serde_json::to_value(example()).ok();
    }
    if !PARTS.contains(&part) {
        return None;
    }
    sdk_pack().get_mut(part).map(Value::take)
}

/// The working example: a blueprint with every contribution, a package that verifies without a finding. A part of its
/// own since 15 November 2026 (17 KB): with it the pack went past what the desktop's Asistan gives a tool's result.
fn example() -> std::collections::BTreeMap<String, String> {
    blueprint(&Ask {
        id: "tr.ornek.ornek-eklenti".into(),
        name: "Örnek eklenti".into(),
        summary: "Her katkı türünün küçük, çalışan örneği.".into(),
        vertical: true,
        module: Module::Cbs,
        contributions: Contribution::ALL.to_vec(),
    })
    .unwrap_or_default()
}

/// The pack, as the MCP resource and `extension.sdk` give it.
pub fn sdk_pack() -> Value {
    let example = json!({
        "files": example().keys().collect::<Vec<_>>(),
        "how": "Her katkı türünün küçük, çalışan örneği (manifest, Python, panel, araç tanımı, form, katman şeması), \
                doğrulamadan sorunsuz geçen bir paket: extension.sdk part=example ya da kentos kek sdk --part example. \
                İsteğe göre iskelet için extension.blueprint.",
    });
    let modules: Vec<&str> = [Module::Cbs, Module::Harita, Module::Mimari, Module::Yol]
        .iter()
        .map(|m| m.id())
        .collect();
    json!({
        "format": "kentos.ai-kek-pack",
        "version": 1,
        "guide": GUIDE,
        "api": { "module": "kentos.ext", "stub": EXT_STUB },
        "capabilities": Capability::ALL.iter().map(|c| json!({
            "id": c.id(),
            "title": c.title(),
            "scopes": c.scopes().iter().map(|s| s.id()).collect::<Vec<_>>(),
        })).collect::<Vec<_>>(),
        "modules": modules,
        "contributions": [
            { "kind": "command", "decorator": "@ext.command", "manifest": "entrypoints.commands",
              "capabilities": ["document.read ya da document.write (çizime erişirse)"] },
            { "kind": "processing", "decorator": "@ext.processing", "manifest": "entrypoints.processing",
              "capabilities": ["processing.provider"], "descriptor": "tools/<kimlik>.tool.json" },
            { "kind": "audit", "decorator": "@ext.audit", "manifest": "entrypoints.audits",
              "capabilities": ["audit.provider"] },
            { "kind": "expression", "decorator": "@ext.expr", "manifest": "entrypoints.expressions",
              "capabilities": ["expression.provider"] },
            { "kind": "mapTool", "decorator": "@ext.map_tool", "manifest": "entrypoints.mapTools",
              "capabilities": ["mapTool", "document.write"] },
            { "kind": "panel", "manifest": "entrypoints.panels", "file": "ui/<kimlik>.panel.json",
              "capabilities": ["ui.panel"],
              "note": "Bildirimsel bileşenler; düğmesi bir komutu panelin değerleriyle (ctx.params) çalıştırır, satırlar tabloya." },
            { "kind": "schema", "manifest": "schemaFile", "file": "schema.json",
              "capabilities": ["document.write (kuran komut için)"],
              "note": "Katmanlar, alanları, değer listeleri, ilişkiler; bir komut ctx.apply_declarative_schema(\"schema.json\") ile kurar." },
            { "kind": "ribbonTab", "manifest": "contributions.ribbonTabs", "capabilities": ["ui.ribbonTab"],
              "note": "Yalnız dikey eklenti (kind: vertical); öğeleri command, processing, audit, mapTool gösterir." },
            { "kind": "contextMenu", "manifest": "contributions.contextMenus", "capabilities": ["ui.contextMenu"] },
            { "kind": "dataSource", "manifest": "contributions.dataSources", "capabilities": ["dataSource"],
              "note": "Python'suz; ağdan okuyorsa network da gerekir." }
        ],
        "toolDescriptor": {
            "format": "kentos.processingTool",
            "schemaVersion": 1,
            "parameterTypes": ["features", "number", "string", "boolean", "enum", "field", "point", "layer"],
            "outputTypes": ["features", "table", "number", "string"],
            "fields": "id, version, label, description, modelDescription (yapay zekâ için, en çok 2000 karakter), help, \
                       keywords, parameters, outputs, feedback, capabilities.runtimes, timeoutSeconds",
            "notes": "features parametresi default: {scope: \"layer\", layerId: \"<katman kimliği>\"} ile eklentinin kendi \
                      katmanında açılır (verilmezse etkin katmanla); yeni nesne veren features çıktısı sinkParameter ile \
                      bir layer parametresini adlandırır (o parametre default: {newName: \"…\"} ile yeni katman önerir)."
        },
        "verifyCodes": {
            "python_function_missing": "Manifestin işleyicisindeki işlev modülde yok (en yakını önerilir).",
            "python_decorator_missing": "İşlev işaretsiz: üstüne türünün işaretini yazın.",
            "python_decorator_kind": "İşaretin türü manifestteki giriş noktasıyla uyuşmuyor.",
            "python_decorator_id": "İşaretin kimliği manifestteki yerel kimlikle uyuşmuyor.",
            "python_decorator_twice": "Bir işlevde birden çok @ext işareti.",
            "python_id_twice": "Aynı kimlik iki işlevde.",
            "python_network_undeclared": "Uyarı: network yeteneği istemeden ağ kitaplığı.",
            "python_os_command": "Uyarı: işletim sistemi komutu.",
            "python_eval": "Uyarı: eval/exec.",
            "handler_missing": "İşleyicinin modülü pakette yok (paketin kendi denetimi).",
            "manifest_icon": "İkon yerleşik bir ad (küçük harf, tire) ya da paketteki bir .svg yolu değil.",
            "manifest_icon_unknown": "Uyarı: bu KentOS'ta böyle bir yerleşik ikon yok; en yakınları önerilir.",
            "layer_schema": "schema.json'da bir kural bozuk; işaretçi yeri, ileti kuralı söyler.",
            "ui_panel": "Panel tanımında bir kural bozuk (bileşen türü, kimlik, düğmenin komutu …); işaretçi yeri söyler.",
            "layer_schema_outside": "Uyarı: ilişki şemanın kurmadığı bir katmanı adlandırıyor; projede bulunmalı."
        },
        "workflow": [
            "İsteği ve parametrelerini netleştirin; kurulu eklentileri ve yetenekleri kentos://extensions/installed'dan okuyun.",
            "extension.blueprint ile iskeleti alın.",
            "Dosyaları yazın: atölyesi olan ajan (KentOS'un Asistan'ı, MCP) extension.workshop.write ile kendi atölyesine; \
             kendi dosya araçları olan ajan boş bir klasöre. Kullanıcıya dosya yazdırmayın.",
            "Örnekleri isteğe göre değiştirin; her adımda extension.verify (atölyede id, klasörde path ile); \
             doğrulanınca yanıtının dryRun'ı paketi geçici bir kayıt defterinde bağlar: eklenen katkılar, atlananlar \
             ve nedenleri, kaldırınca kalıntı (kod çalışmaz).",
            "Kütüphane gerekiyorsa klasörde `uv add <paket>`; atölyede uv çalışmaz: Python'un kendi kitaplığıyla ve \
             kentos ile yazın, kütüphane gerekiyorsa kullanıcıya söyleyin.",
            "Paket: atölyede extension.workshop.review paketleyip kullanıcının Gözden geçir ve kur penceresine açar; \
             klasörde `kentos kek pack <klasör>`. Kurulum, yetki ve yayın kullanıcının incelemesiyle olur.",
            "Kullanıcı atölyeden kurduktan sonra: değişiklikleri extension.workshop.reload uygulama kapanmadan yükler \
             (aynı sürüm, kullanıcının verdiği yeteneklerle; yeni yetenek isteyen yeniden review'a gider); \
             extension.workshop.preview kullanıcının gördüğünü (sekme, paneller, araçlar, kapalı düğmenin nedeni) \
             yazıyla verir; extension.workshop.trial denetim kurallarını ve İşlemler araçlarını verdiğiniz örnek \
             nesnelerden kurulan sentetik çizimde çalıştırır (bulgular nesnenin sırasıyla, aracın yazdıkları, hata ve \
             iz kaydı; kullanıcının çizimine dokunmaz). Masaüstünün Asistan'ında ve masaüstüne bağlı ajanda (MCP'de \
             desktop.attach)."
        ],
        "example": example,
        "icons": {
            "count": crate::icon_names::names().count(),
            "how": "Yerleşik ikonlar tire biçiminde yazılır (zoom-extents, space-number, layer-fields). Tamamı: \
                    extension.sdk part=icons ya da kentos kek sdk --part icons. Yoksa paketteki 20×20 çizgi .svg.",
        },
        "panelSchema": {
            "how": "kentos.ui.panel 1'in JSON Schema'sı (bileşenler, anahtarları ve türleri): extension.sdk \
                    part=panelSchema ya da kentos kek sdk --part panelSchema. Kesin denetim yine extension.verify'da \
                    (düğmenin komutu, seçicinin katman seçicisi, kimliklerin tekliği şemada söylenemez).",
        },
        "panelBuilders": {
            "how": "kentos.ui_panel'in kaynağı: panel bileşenlerinin Python işlevleri (ctx.dialog alanları, \
                    set(options=…), ui.panel ile bütün panel), panelin şemasından üretilir: extension.sdk \
                    part=panelBuilders ya da kentos kek sdk --part panelBuilders.",
        },
        "manifestSchema": {
            "how": "manifest.json'un (kentos.extension 1) JSON Schema'sı: anahtarlar, türleri, kapalı sözlükler (yetenekler, \
                    kapsamlar, programlar, katkılar): extension.sdk part=manifestSchema ya da kentos kek sdk --part \
                    manifestSchema. Kesin denetim extension.verify'da (yeteneğin kapsamı, paketteki dosyalar, işleyici).",
        },
        "layerSchema": {
            "how": "Dikey eklentinin schema.json'u (kentos.layerSchema 1: katmanlar, alanları, değer listeleri, alt türler, \
                    değer grupları, ilişkiler) için JSON Schema: extension.sdk part=layerSchema ya da kentos kek sdk \
                    --part layerSchema. Alanın listesi, alt türün kodu, ilişkinin katmanları extension.verify'da.",
        },
        "toolSchema": {
            "how": "İşlemler aracı tanımının (tools/<id>.tool.json, kentos.processingTool 1) JSON Schema'sı: parametre ve \
                    çıktı türleri, anahtarları: extension.sdk part=toolSchema ya da kentos kek sdk --part toolSchema. \
                    Parametre adlarının ve sinkParameter'ın tutarlılığı extension.verify'da.",
        },
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A docstring's first sentence, on one line, at most 400 characters.
    fn first_sentence(doc: &str) -> String {
        let text = doc.split_whitespace().collect::<Vec<_>>().join(" ");
        let end = text
            .match_indices(". ")
            .map(|(i, _)| i + 1)
            .find(|&i| !text[..i].ends_with("e.g.") && !text[..i].ends_with("i.e."))
            .unwrap_or(text.len());
        let sentence = &text[..end];
        match sentence.char_indices().nth(400) {
            Some((i, _)) => format!("{}…", &sentence[..i]),
            None => sentence.to_owned(),
        }
    }

    /// The docstring starting at `lines[at]` (if one does) and the index after it.
    fn docstring(lines: &[&str], at: usize) -> (Option<String>, usize) {
        let Some(first) = lines.get(at).map(|l| l.trim_start()) else {
            return (None, at);
        };
        let Some(rest) = first.strip_prefix("\"\"\"") else {
            return (None, at);
        };
        if let Some(end) = rest.find("\"\"\"") {
            return (Some(first_sentence(&rest[..end])), at + 1);
        }
        let mut text = rest.to_owned();
        let mut i = at + 1;
        while let Some(line) = lines.get(i) {
            i += 1;
            if let Some(end) = line.find("\"\"\"") {
                text.push(' ');
                text.push_str(&line[..end]);
                break;
            }
            text.push(' ');
            text.push_str(line);
        }
        (Some(first_sentence(&text)), i)
    }

    /// A signature from `lines[at]` to the line ending it, on one line, and the index after it.
    fn signature(lines: &[&str], at: usize) -> (String, usize) {
        let mut depth = 0i32;
        let mut parts = Vec::new();
        let mut i = at;
        while let Some(line) = lines.get(i) {
            i += 1;
            parts.push(line.trim());
            depth +=
                line.matches(['(', '[']).count() as i32 - line.matches([')', ']']).count() as i32;
            if depth <= 0 && line.trim_end().ends_with(':') {
                break;
            }
        }
        let joined = parts
            .join(" ")
            .replace("( ", "(")
            .replace(", )", ")")
            .replace(" )", ")");
        (stub_defaults(joined.trim_end_matches(':')), i)
    }

    /// A default that is the module's private sentinel (`value: Any = _KEEP`, "not given") written as a stub writes
    /// one it does not name: `...` (the stub has no `_KEEP`; a checker reading it would not find it).
    fn stub_defaults(sig: &str) -> String {
        let mut out = String::with_capacity(sig.len());
        let mut rest = sig;
        while let Some(at) = rest.find("= _") {
            out.push_str(&rest[..at]);
            let after = &rest[at + 2..];
            let name = after
                .find(|c: char| !(c.is_ascii_alphanumeric() || c == '_'))
                .unwrap_or(after.len());
            out.push_str("= ...");
            rest = &after[name..];
        }
        out.push_str(rest);
        out
    }

    /// `kentos.ext`'s stub from its source (see [`EXT_STUB`]).
    fn ext_stub(source: &str) -> String {
        let lines: Vec<&str> = source.lines().collect();
        let start = source.find("__all__ = [").expect("__all__");
        let end = start + source[start..].find(']').expect("__all__'s end");
        let public: Vec<&str> = source[start..end].split('"').skip(1).step_by(2).collect();
        let mut out = String::from(
            "# kentos.ext: what an extension's Python uses (`from kentos import ext`). Generated from \
             python/kentos/ext.py by\n# `KENTOS_WRITE_SDK=1 cargo test -p kentos-extensions --lib -- sdk_pack`; do not \
             edit.\nfrom dataclasses import dataclass, field\nfrom typing import Any, Callable, Iterable, Iterator, \
             Sequence, TypeVar\n\nF = TypeVar(\"F\", bound=Callable[..., Any])\n",
        );
        let doc = |out: &mut String, indent: &str, text: Option<String>| {
            if let Some(text) = text {
                out.push_str(&format!("{indent}\"\"\"{text}\"\"\"\n"));
            }
        };
        let mut decorators: Vec<&str> = Vec::new();
        let mut i = 0;
        while i < lines.len() {
            let line = lines[i];
            if line.starts_with('@') {
                decorators.push(line);
                i += 1;
                continue;
            }
            let name = |rest: &str| {
                rest.split(|c: char| !(c.is_alphanumeric() || c == '_'))
                    .next()
                    .unwrap_or_default()
                    .to_owned()
            };
            if let Some(rest) = line.strip_prefix("def ")
                && public.contains(&name(rest).as_str())
            {
                let (sig, next) = signature(&lines, i);
                let (text, next) = docstring(&lines, next);
                out.push_str(&format!("\n\n{sig}:\n"));
                doc(&mut out, "    ", text.clone());
                if text.is_none() {
                    out.push_str("    ...\n");
                }
                i = next;
            } else if let Some(rest) = line.strip_prefix("class ")
                && public.contains(&name(rest).as_str())
            {
                out.push_str("\n\n");
                for d in &decorators {
                    out.push_str(&format!("{d}\n"));
                }
                out.push_str(&format!("{line}\n"));
                let (text, mut j) = docstring(&lines, i + 1);
                doc(&mut out, "    ", text);
                let mut inner: Vec<&str> = Vec::new();
                let mut in_init = false;
                let mut attr_doc: Option<String> = None;
                while j < lines.len() && (lines[j].is_empty() || lines[j].starts_with(' ')) {
                    let l = lines[j];
                    if let Some(rest) = l.strip_prefix("    def ") {
                        let method = name(rest);
                        in_init = method == "__init__";
                        let (sig, next) = signature(&lines, j);
                        let (text, next) = docstring(&lines, next);
                        // `to_json` is KentOS's own reading of a result, not something an extension calls.
                        let shown = (!method.starts_with('_') || method == "__geo_interface__")
                            && method != "to_json";
                        if shown && !inner.contains(&"    @overload") {
                            for d in &inner {
                                out.push_str(&format!("{d}\n"));
                            }
                            out.push_str(&format!("    {sig}:\n"));
                            doc(&mut out, "        ", text.clone());
                            if text.is_none() {
                                out.push_str("        ...\n");
                            }
                        }
                        inner.clear();
                        j = next;
                        continue;
                    }
                    if l.starts_with("    @") {
                        inner.push(l);
                    } else if in_init && let Some(rest) = l.strip_prefix("        #: ") {
                        attr_doc = Some(
                            attr_doc.map_or_else(|| rest.to_owned(), |d| format!("{d} {rest}")),
                        );
                    } else if in_init && let Some(rest) = l.strip_prefix("        self.") {
                        let field = name(rest);
                        if !field.starts_with('_') {
                            let kind = rest[field.len()..]
                                .strip_prefix(": ")
                                .and_then(|t| t.split(" = ").next())
                                .unwrap_or("Any");
                            if let Some(d) = attr_doc.take() {
                                out.push_str(&format!("    #: {}\n", first_sentence(&d)));
                            }
                            out.push_str(&format!("    {field}: {kind}\n"));
                        }
                        attr_doc = None;
                    } else if !in_init
                        && let Some(rest) = l.strip_prefix("    ")
                        && !rest.starts_with([' ', '#', '"', '_'])
                        && rest.contains(": ")
                        && !rest.starts_with("def ")
                    {
                        // A dataclass's field.
                        out.push_str(&format!("    {rest}\n"));
                    }
                    j += 1;
                }
                i = j;
            } else {
                i += 1;
            }
            if !line.starts_with('@') {
                decorators.clear();
            }
        }
        out
    }

    /// The guide, the SDK's reference, names every panel component type Kapı 1 knows, as ADR 8057 §25's coverage
    /// report does: the two show the same version (plan/eklenti.md ADIM 22's acceptance).
    #[test]
    fn the_guide_names_every_panel_component() {
        for kind in crate::ui_panel::component_types() {
            assert!(
                GUIDE.contains(&format!("`{kind}`")),
                "the guide's Panel section does not name `{kind}`"
            );
        }
    }

    /// The committed panel schema is the one the panel's types make: a component added is written there by
    /// `KENTOS_WRITE_SDK=1 cargo test -p kentos-extensions --lib -- sdk_pack`.
    #[test]
    fn the_panel_schema_is_the_panel_types() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        let made = serde_json::to_string_pretty(&crate::ui_panel::json_schema()).unwrap() + "\n";
        if std::env::var_os("KENTOS_WRITE_SDK").is_some() {
            std::fs::write(root.join("sdk/kentos-ui-panel.schema.json"), &made)
                .expect("the schema");
            return;
        }
        assert!(
            PANEL_SCHEMA == made,
            "the panel's types changed: KENTOS_WRITE_SDK=1 cargo test -p kentos-extensions --lib -- sdk_pack"
        );
        // Every component type is in it, as the guide and ADR 8057 §25 name them.
        for kind in crate::ui_panel::component_types() {
            assert!(PANEL_SCHEMA.contains(&format!("\"{kind}\"")), "{kind}");
        }
        assert!(
            sdk_part("panelSchema")
                .is_some_and(|s| s["properties"]["format"]["const"] == "kentos.ui.panel")
        );
    }

    /// The committed manifest schema is the one the manifest's types make (a key added is written there by
    /// `KENTOS_WRITE_SDK=1 cargo test -p kentos-extensions --lib -- sdk_pack`), and it takes the manifests KentOS ships:
    /// the fixtures, the Vitrin's and a blueprint's with every contribution; a key the reader refuses, it refuses.
    #[test]
    fn the_manifest_schema_is_the_manifest_types_and_takes_real_manifests() {
        use crate::schema_check::errors;
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        let made = serde_json::to_string_pretty(&crate::manifest::json_schema()).unwrap() + "\n";
        if std::env::var_os("KENTOS_WRITE_SDK").is_some() {
            std::fs::write(
                root.join("sdk/kentos-extension-manifest.schema.json"),
                &made,
            )
            .expect("the schema");
            return;
        }
        assert!(
            MANIFEST_SCHEMA == made,
            "the manifest's types changed: KENTOS_WRITE_SDK=1 cargo test -p kentos-extensions --lib -- sdk_pack"
        );
        for capability in crate::manifest::Capability::ALL {
            assert!(
                MANIFEST_SCHEMA.contains(&format!("\"{}\"", capability.id())),
                "{}",
                capability.id()
            );
        }
        let schema = sdk_part("manifestSchema").expect("the part");
        assert_eq!(schema["properties"]["format"]["const"], "kentos.extension");
        let repo = root.join("../../..");
        let mut manifests: Vec<(String, Value)> = [
            "fixtures/extensions/v1/ornek-altlik",
            "fixtures/extensions/v1/ornek-imar",
            "fixtures/extensions/v1/ornek-kent",
            "fixtures/extensions/v1/mebis",
            "samples/eklenti-vitrini/eklenti",
        ]
        .iter()
        .map(|dir| {
            let text = std::fs::read_to_string(repo.join(dir).join("manifest.json")).expect(dir);
            ((*dir).to_owned(), serde_json::from_str(&text).expect(dir))
        })
        .collect();
        let files = crate::blueprint::blueprint(&crate::blueprint::Ask {
            id: "tr.ornek.sema".into(),
            name: "Şema".into(),
            summary: "Bütün katkılarıyla bir iskelet.".into(),
            vertical: true,
            module: crate::manifest::Module::Cbs,
            contributions: crate::blueprint::Contribution::ALL.to_vec(),
        })
        .expect("a blueprint");
        manifests.push((
            "blueprint".into(),
            serde_json::from_str(&files["manifest.json"]).expect("its manifest"),
        ));
        for (name, manifest) in &manifests {
            let found = errors(&schema, manifest);
            assert!(found.is_empty(), "{name}: {found:#?}");
        }
        // As strict as the reader: a key it does not know, a capability not in the vocabulary.
        let vitrin = &manifests[4].1;
        let mut unknown = vitrin.clone();
        unknown["entrypoints"]["commands"][0]["hanlder"] = json!("vitrin.logic:x");
        assert!(
            errors(&schema, &unknown)
                .iter()
                .any(|e| e.ends_with("/hanlder: bilinmeyen anahtar")),
            "{:?}",
            errors(&schema, &unknown)
        );
        let mut invented = vitrin.clone();
        invented["requestedCapabilities"][0]["id"] = json!("filesystem.write");
        assert!(!errors(&schema, &invented).is_empty());
    }

    /// The committed layer schema is the one the check's key lists make, and it takes the layer schemas KentOS ships
    /// (the Vitrin's, the fixtures' and a blueprint's); a key the check refuses, it refuses.
    #[test]
    fn the_layer_schema_is_the_checks_and_takes_real_schemas() {
        use crate::schema_check::errors;
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        let made =
            serde_json::to_string_pretty(&crate::layer_schema::json_schema()).unwrap() + "\n";
        if std::env::var_os("KENTOS_WRITE_SDK").is_some() {
            std::fs::write(root.join("sdk/kentos-layer-schema.schema.json"), &made)
                .expect("the schema");
            return;
        }
        assert!(
            LAYER_SCHEMA == made,
            "the layer schema's keys changed: KENTOS_WRITE_SDK=1 cargo test -p kentos-extensions --lib -- sdk_pack"
        );
        let schema = sdk_part("layerSchema").expect("the part");
        let repo = root.join("../../..");
        let mut schemas: Vec<(String, Value)> = [
            "samples/eklenti-vitrini/eklenti",
            "fixtures/extensions/v1/ornek-kent",
            "fixtures/extensions/v1/mebis",
        ]
        .iter()
        .map(|dir| {
            let text = std::fs::read_to_string(repo.join(dir).join("schema.json")).expect(dir);
            ((*dir).to_owned(), serde_json::from_str(&text).expect(dir))
        })
        .collect();
        let files = crate::blueprint::blueprint(&crate::blueprint::Ask {
            id: "tr.ornek.katman".into(),
            name: "Katman".into(),
            summary: "Katman şemalı bir iskelet.".into(),
            vertical: true,
            module: crate::manifest::Module::Cbs,
            contributions: crate::blueprint::Contribution::ALL.to_vec(),
        })
        .expect("a blueprint");
        if let Some(text) = files.get("schema.json") {
            schemas.push((
                "blueprint".into(),
                serde_json::from_str(text).expect("its schema"),
            ));
        }
        assert_eq!(schemas.len(), 4, "the blueprint makes a layer schema too");
        for (name, value) in &schemas {
            let found = errors(&schema, value);
            assert!(found.is_empty(), "{name}: {found:#?}");
            // The check agrees: these are files KentOS installs.
            let (problems, _) = crate::layer_schema::check("schema.json", value, &[]);
            assert!(
                problems
                    .iter()
                    .all(|p| p.message.contains("entrypoints.forms")),
                "{name}: {problems:#?}"
            );
        }
        let vitrin = &schemas[0].1;
        let mut unknown = vitrin.clone();
        unknown["layers"][0]["fields"][0]["tip"] = json!("text");
        assert!(
            errors(&schema, &unknown)
                .iter()
                .any(|e| e.ends_with("/tip: bilinmeyen anahtar")),
            "{:?}",
            errors(&schema, &unknown)
        );
        let mut kind = vitrin.clone();
        kind["layers"][0]["fields"][0]["kind"] = json!("string");
        assert!(!errors(&schema, &kind).is_empty(), "a kind KentOS has not");
        let mut empty = vitrin.clone();
        empty["layers"] = json!([]);
        assert!(!errors(&schema, &empty).is_empty(), "at least one layer");
    }

    /// The tool descriptor schema takes the descriptors KentOS ships (the Vitrin's, the fixtures', a blueprint's); a key
    /// written wrong and a parameter kind an extension's tool does not take, it refuses.
    #[test]
    fn the_tool_schema_takes_real_descriptors() {
        use crate::schema_check::errors;
        let schema = sdk_part("toolSchema").expect("the part");
        assert_eq!(
            schema["properties"]["format"]["const"],
            "kentos.processingTool"
        );
        let repo = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../..");
        let mut descriptors: Vec<(String, Value)> = [
            "samples/eklenti-vitrini/eklenti/tools/taks.tool.json",
            "fixtures/extensions/v1/ornek-imar/tools/taks.tool.json",
            "fixtures/extensions/v1/mebis/tools/occupancy-by-island.tool.json",
            "fixtures/extensions/v1/mebis/tools/empty-graves.tool.json",
        ]
        .iter()
        .map(|file| {
            let text = std::fs::read_to_string(repo.join(file)).expect(file);
            ((*file).to_owned(), serde_json::from_str(&text).expect(file))
        })
        .collect();
        let files = crate::blueprint::blueprint(&crate::blueprint::Ask {
            id: "tr.ornek.arac".into(),
            name: "Araç".into(),
            summary: "Araçlı bir iskelet.".into(),
            vertical: true,
            module: crate::manifest::Module::Cbs,
            contributions: crate::blueprint::Contribution::ALL.to_vec(),
        })
        .expect("a blueprint");
        for (path, text) in files.iter().filter(|(p, _)| p.ends_with(".tool.json")) {
            descriptors.push((
                path.clone(),
                serde_json::from_str(text).expect("a descriptor"),
            ));
        }
        assert_eq!(descriptors.len(), 5, "the blueprint makes a tool too");
        for (name, descriptor) in &descriptors {
            let found = errors(&schema, descriptor);
            assert!(found.is_empty(), "{name}: {found:#?}");
        }
        let vitrin = &descriptors[0].1;
        let mut typo = vitrin.clone();
        typo["parameters"][0]["lable"] = json!("Parseller");
        assert!(
            !errors(&schema, &typo).is_empty(),
            "a key written wrong is said where it was written"
        );
        let mut raster = vitrin.clone();
        raster["parameters"][0]["type"] = json!("rasterValues");
        assert!(
            !errors(&schema, &raster).is_empty(),
            "a kind İşlemler has but an extension's tool does not take"
        );
        let mut sink = vitrin.clone();
        sink["outputs"] = json!([{ "name": "yeni", "type": "features" }]);
        assert!(
            !errors(&schema, &sink).is_empty(),
            "new objects name the layer they go to"
        );
    }

    /// The panel schema takes the Vitrin's panels; a component of a type KentOS does not draw, it refuses.
    #[test]
    fn the_panel_schema_takes_the_vitrins_panels() {
        use crate::schema_check::errors;
        let schema = sdk_part("panelSchema").expect("the part");
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../../samples/eklenti-vitrini/eklenti/ui");
        let mut read = 0;
        for entry in std::fs::read_dir(&dir).expect("the Vitrin's panels") {
            let path = entry.expect("an entry").path();
            let panel: Value =
                serde_json::from_str(&std::fs::read_to_string(&path).expect("a panel"))
                    .expect("JSON");
            let found = errors(&schema, &panel);
            assert!(found.is_empty(), "{}: {found:#?}", path.display());
            read += 1;
            let mut odd = panel.clone();
            odd["body"][0]["type"] = json!("saat");
            assert!(!errors(&schema, &odd).is_empty(), "{}", path.display());
        }
        assert_eq!(read, 3, "the Vitrin's three panels");
    }

    #[test]
    fn the_stub_is_the_sdk_s() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        let source = std::fs::read_to_string(root.join("../../../python/kentos/ext.py"))
            .expect("python/kentos/ext.py");
        let stub = ext_stub(&source);
        if std::env::var_os("KENTOS_WRITE_SDK").is_some() {
            std::fs::write(root.join("sdk/kentos_ext.pyi"), &stub).expect("the stub");
            return;
        }
        assert!(
            EXT_STUB == stub,
            "python/kentos/ext.py changed: KENTOS_WRITE_SDK=1 cargo test -p kentos-extensions --lib -- sdk_pack"
        );
        // Every name `__all__` gives is in it, and what the guide shows.
        for name in [
            "class CommandContext",
            "def report(",
            "def read_features(",
            "def add_feature(",
            "class AuditFinding",
        ] {
            assert!(stub.contains(name), "{name}");
        }
    }

    #[test]
    fn the_pack_holds_every_capability_and_a_verifying_example() {
        let pack = sdk_pack();
        assert_eq!(pack["format"], "kentos.ai-kek-pack");
        assert_eq!(
            pack["capabilities"].as_array().map(Vec::len),
            Some(Capability::ALL.len())
        );
        let example: std::collections::BTreeMap<String, Vec<u8>> = sdk_part("example")
            .expect("the example")
            .as_object()
            .expect("the example's files")
            .iter()
            .map(|(k, v)| {
                (
                    k.clone(),
                    v.as_str().unwrap_or_default().as_bytes().to_vec(),
                )
            })
            .collect();
        let v = crate::verify(&example, &crate::Limits::default());
        assert!(
            v.problems.is_empty() && v.warnings.is_empty(),
            "{:#?}",
            v.problems
        );
        // Its parts are the ones it says, each there.
        let keys: Vec<&str> = pack
            .as_object()
            .unwrap()
            .keys()
            .map(String::as_str)
            .collect();
        for part in PARTS {
            assert!(sdk_part(part).is_some_and(|v| !v.is_null()), "{part}");
        }
        assert_eq!(keys.len(), PARTS.len() + 2, "{keys:?}");
        assert_eq!(
            sdk_part("icons").and_then(|v| v.as_array().map(Vec::len)),
            pack["icons"]["count"].as_u64().map(|n| n as usize)
        );
        // The pack fits what the desktop's Asistan gives a tool's result.
        assert!(
            pack.to_string().len() < 48_000,
            "{}",
            pack.to_string().len()
        );
        assert!(sdk_part("format").is_none());
        // The map tool's shapes are the manifest's.
        use crate::manifest::Capture;
        for capture in [
            Capture::Point,
            Capture::Line,
            Capture::Polygon,
            Capture::Rectangle,
            Capture::OrientedRectangle,
            Capture::Circle,
        ] {
            let name = serde_json::to_value(capture).unwrap();
            assert!(
                GUIDE.contains(&format!(" {}", name.as_str().unwrap())),
                "{name}"
            );
        }
        // Every decorator the guide names is one the SDK has.
        for decorator in [
            "@ext.command",
            "@ext.processing",
            "@ext.audit",
            "@ext.expr",
            "@ext.map_tool",
        ] {
            assert!(GUIDE.contains(decorator), "{decorator}");
        }
    }
}
