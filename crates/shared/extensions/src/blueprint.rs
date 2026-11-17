//! `extension.blueprint` (plan/eklenti.md ADIM 21; docs/adr/8057 §19): a new extension's working folder, valid by
//! construction (`verify` passes it without a problem or a warning), with one small example of each contribution
//! asked for: a command that reads the layers, an İşlemler tool with its descriptor and an example, an audit rule with
//! a fix and its examples (`kentos kek test` runs both, §48, §75), an expression function, a map tool, a layer schema,
//! a panel and a portable form bound to the schema's layer (§112). An agent or a person grows it from there.
//!
//! KentOS gives the files; the caller writes them (an agent's read-only tool writes nothing). The Python project has
//! no dependency and its lock is uv's own for that (uv 0.11): adding a library is `uv add` in the folder, which writes
//! both files again.

use std::collections::BTreeMap;

use serde_json::{Value, json};

use crate::manifest::{Module, is_extension_id};

/// What the new extension is.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Ask {
    /// Its reverse-domain id (`tr.belediye.mezarlik`).
    pub id: String,
    /// Its name for people.
    pub name: String,
    /// One sentence: what it does.
    pub summary: String,
    /// A vertical extension opens its own ribbon tab; a utility adds tools to KentOS's.
    pub vertical: bool,
    /// The program it is for.
    pub module: Module,
    pub contributions: Vec<Contribution>,
}

/// A contribution the blueprint gives an example of.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Contribution {
    Command,
    Processing,
    Audit,
    Expression,
    MapTool,
    /// A layer schema (`schema.json`, docs/adr/8057 §23) and the command that sets it up on the drawing.
    Schema,
    /// A panel (`ui/nesne-ara.panel.json`, docs/adr/8057 §24) whose button runs a search command.
    Panel,
    /// A portable form (`forms/alan.kform`, docs/adr/8057 §112) the schema's layer opens its objects with: the layer's
    /// fields, its list of values from the schema's domain. Asked alone, it brings the schema.
    Form,
}

impl Contribution {
    /// Its name in an agent's call (`command`, `processing`, `audit`, `expression`, `mapTool`, `schema`, `panel`,
    /// `form`).
    pub fn parse(name: &str) -> Option<Contribution> {
        match name {
            "command" => Some(Contribution::Command),
            "processing" => Some(Contribution::Processing),
            "audit" => Some(Contribution::Audit),
            "expression" => Some(Contribution::Expression),
            "mapTool" => Some(Contribution::MapTool),
            "schema" => Some(Contribution::Schema),
            "panel" => Some(Contribution::Panel),
            "form" => Some(Contribution::Form),
            _ => None,
        }
    }

    pub const ALL: [Contribution; 8] = [
        Contribution::Command,
        Contribution::Processing,
        Contribution::Audit,
        Contribution::Expression,
        Contribution::MapTool,
        Contribution::Schema,
        Contribution::Panel,
        Contribution::Form,
    ];
}

/// The folder's files (path → text), or why the ask cannot be made one.
pub fn blueprint(ask: &Ask) -> Result<BTreeMap<String, String>, String> {
    if !is_extension_id(&ask.id) {
        return Err(format!(
            "“{}” bir eklenti kimliği değil: ters alan adı, küçük harf, en az iki parça (tr.belediye.mezarlik).",
            ask.id
        ));
    }
    let name = ask.name.trim();
    if name.is_empty() || name.chars().count() > 80 {
        return Err("Eklentinin adı boş olamaz ve en çok 80 karakterdir.".to_owned());
    }
    let summary = ask.summary.trim();
    if summary.is_empty() || summary.chars().count() > 200 {
        return Err("Özet boş olamaz ve en çok 200 karakterdir (tek cümle).".to_owned());
    }
    let mut asked = ask.contributions.clone();
    // A form opens the schema's layer's objects: it brings the schema.
    if asked.contains(&Contribution::Form) {
        asked.push(Contribution::Schema);
    }
    asked.sort();
    asked.dedup();
    if asked.is_empty() {
        return Err(
            "En az bir katkı seçin: command, processing, audit, expression, mapTool, schema, panel, form.".to_owned(),
        );
    }
    let last = ask.id.rsplit('.').next().unwrap_or("eklenti");
    let package = last.replace('-', "_");
    let has = |c: Contribution| asked.contains(&c);
    let module = ask.module.id();

    let mut entrypoints = serde_json::Map::new();
    let mut capabilities = Vec::new();
    let mut items = Vec::new();
    let mut commands = Vec::new();
    let mut python = vec![HEADER.replace("{NAME}", name)];
    let mut files = BTreeMap::new();
    let schema = has(Contribution::Schema);
    // The schema's layer: its id in the drawing, what the map tool draws into when there is one.
    let layer = format!("{package}_alan");
    if has(Contribution::Command) {
        commands.push(
            json!({ "id": "katman-listesi", "title": "Katman listesi", "handler": format!("{package}.logic:katman_listesi") }),
        );
        capabilities.push(json!({ "id": "document.read", "scope": "current-project",
                                  "reason": "Açık çizimin katmanlarını ve nesne sayılarını okur." }));
        items.push(
            json!({ "command": "katman-listesi", "size": "large", "icon": "assets/eklenti.svg" }),
        );
        python.push(COMMAND.to_owned());
    }
    if has(Contribution::Processing) {
        entrypoints.insert(
            "processing".into(),
            json!([{ "id": "alan-ozeti", "descriptor": "tools/alan-ozeti.tool.json", "handler": format!("{package}.logic:alan_ozeti") }]),
        );
        capabilities.push(
            json!({ "id": "processing.provider", "scope": "plugin-owned",
                                  "reason": "Alan özetini İşlemler'e ekler." }),
        );
        items.push(json!({ "processing": "alan-ozeti", "label": "Alan özeti", "size": "large", "icon": "assets/eklenti.svg" }));
        let descriptor = json!({
            "format": "kentos.processingTool",
            "schemaVersion": 1,
            "id": "alan-ozeti",
            "version": "0.1.0",
            "label": "Alan özeti",
            "description": "Seçilen kapalı alanların sayısını ve toplam alanını verir.",
            "modelDescription": "Kapalı alanların (parsel, yapı, ada …) sayısını ve toplam alanını m² olarak öğrenmek \
                                 için kullanın. alanlar: kapalı alanlar. Tablo (her alanın sırası ve alanı) ve toplam \
                                 döner; çizime bir şey yazmaz.",
            "keywords": ["alan", "özet"],
            "parameters": [{
                "name": "alanlar",
                "label": "Alanlar",
                "type": "features",
                "kinds": ["polygon"],
                "scopes": ["layer", "selection", "visible", "all"],
                "description": "Alanı toplanacak kapalı alanlar."
            }],
            "outputs": [
                { "name": "tablo", "label": "Alan tablosu", "type": "table" },
                { "name": "toplam", "label": "Toplam alan (m²)", "type": "number" }
            ],
            "feedback": { "progress": true, "cancel": true },
            // On the desktop and in the headless hosts (MCP, Python) alike (docs/adr/8057 §70).
            "capabilities": { "runtimes": ["desktop-python", "headless-python"] },
            // What it gives on objects of its own (docs/adr/8057 §48): Örnekleri çalıştır and kentos kek test check it.
            "examples": [{
                "title": "İki kare alan",
                "features": [
                    { "layer": "alan", "geometry": { "type": "Polygon",
                      "coordinates": [[[0.0, 0.0], [10.0, 0.0], [10.0, 10.0], [0.0, 10.0], [0.0, 0.0]]] } },
                    { "layer": "alan", "geometry": { "type": "Polygon",
                      "coordinates": [[[20.0, 0.0], [40.0, 0.0], [40.0, 20.0], [20.0, 20.0], [20.0, 0.0]]] } }
                ],
                "params": { "alanlar": { "scope": "layer", "layerId": "alan" } },
                "expect": { "outputs": { "toplam": 500.0 }, "summary": "2 alan" }
            }]
        });
        let mut descriptor = descriptor;
        // With a schema, the tool opens on the extension's own layer (not the active one).
        if schema {
            descriptor["parameters"][0]["default"] = json!({ "scope": "layer", "layerId": layer });
        }
        files.insert("tools/alan-ozeti.tool.json".to_owned(), pretty(&descriptor));
        python.push(PROCESSING.to_owned());
    }
    if has(Contribution::Audit) {
        entrypoints.insert(
            "audits".into(),
            json!([{ "id": "adsiz", "title": "Adsız nesneler", "domain": module, "handler": format!("{package}.logic:adsiz"),
                     "examples": "audits/adsiz.examples.json" }]),
        );
        // Its examples (docs/adr/8057 §75): what it is given and the findings it must give; kentos kek test runs them.
        files.insert(
            "audits/adsiz.examples.json".to_owned(),
            pretty(&json!({
                "format": "kentos.auditExamples", "schemaVersion": 1,
                "examples": [{
                    "title": "Adsız nesne bulunur, düzeltmesi ad yazar",
                    "features": [
                        { "layer": "nesne", "geometry": { "type": "Point", "coordinates": [0.0, 0.0] },
                          "attrs": { "ad": "Çeşme" } },
                        { "layer": "nesne", "geometry": { "type": "Point", "coordinates": [10.0, 0.0] } }
                    ],
                    "expect": { "count": 1, "has": [{ "severity": "info", "feature": 1, "fix": "cad.entities.set" }] }
                }]
            })),
        );
        capabilities.push(json!({ "id": "audit.provider", "scope": "plugin-owned",
                                  "reason": "Adı olmayan nesneleri bulur ve ad yazan bir düzeltme önerir." }));
        items.push(json!({ "audit": "adsiz", "size": "large", "icon": "assets/eklenti.svg" }));
        python.push(AUDIT.to_owned());
    }
    if has(Contribution::Expression) {
        entrypoints.insert(
            "expressions".into(),
            json!([{ "id": "buyuk-harf", "title": "Türkçe büyük harf", "handler": format!("{package}.logic:buyuk_harf"),
                     // A call and its value (docs/adr/8057 §78): kentos kek test runs them.
                     "examples": [{ "args": ["istanbul"], "expect": "İSTANBUL" },
                                  { "args": ["ılık"], "expect": "ILIK" },
                                  { "args": [null], "expect": null }] }]),
        );
        capabilities.push(
            json!({ "id": "expression.provider", "scope": "plugin-owned",
                                  "reason": "İfadelere Türkçe büyük harf işlevini ekler." }),
        );
        python.push(EXPRESSION.replace("{ID}", &ask.id));
    }
    if schema {
        // The layers it needs, set up by its own command (docs/adr/8057 §23).
        commands.push(
            json!({ "id": "katmanlari-kur", "title": "Katmanları kur", "handler": format!("{package}.logic:katmanlari_kur") }),
        );
        items.push(json!({ "command": "katmanlari-kur", "size": "large", "icon": "layer-add" }));
        let layer_schema = json!({
            "format": crate::layer_schema::FORMAT,
            "version": crate::layer_schema::VERSION,
            "group": { "id": package, "name": name },
            "domains": [{
                "id": format!("{package}_durum"),
                "name": "Durum",
                "kind": "text",
                "values": [{ "code": "ETKIN", "label": "Etkin" }, { "code": "KAPALI", "label": "Kapalı" }]
            }],
            "layers": [{
                "name": layer,
                "title": format!("{name} alanı"),
                "geometry": "area",
                "fields": [
                    { "name": "ad", "kind": "text", "alias": "Ad" },
                    { "name": "durum", "kind": "text", "alias": "Durum", "domain": format!("{package}_durum") }
                ]
            }]
        });
        let mut layer_schema = layer_schema;
        if has(Contribution::Form) {
            // The layer opens its objects with the form; installing binds it to the layer by its name.
            layer_schema["layers"][0]["form"] = json!(FORM_FILE);
            entrypoints.insert(
                "forms".into(),
                json!([{ "id": "alan", "definition": FORM_FILE }]),
            );
            files.insert(FORM_FILE.to_owned(), pretty(&form_package(name, &package)));
        }
        files.insert("schema.json".to_owned(), pretty(&layer_schema));
        python.push(SCHEMA.to_owned());
    }
    let panel = has(Contribution::Panel);
    if panel {
        // A declared panel whose button runs a command with the panel's values (docs/adr/8057 §24).
        commands.push(
            json!({ "id": "nesne-ara", "title": "Nesne ara", "handler": format!("{package}.logic:nesne_ara") }),
        );
        entrypoints.insert(
            "panels".into(),
            json!([{ "id": "nesne-paneli", "title": "Nesne ara", "side": "right", "definition": "ui/nesne-ara.panel.json" }]),
        );
        capabilities.push(json!({ "id": "ui.panel", "scope": "plugin-owned",
                                  "reason": "Nesne arama panelini sağdaki Eklenti sekmesine ekler." }));
        if !capabilities.iter().any(|c| c["id"] == "document.read") {
            capabilities.push(json!({ "id": "document.read", "scope": "current-project",
                                      "reason": "Açık çizimin nesnelerini okur (panelin araması)." }));
        }
        let definition = json!({
            "format": crate::ui_panel::FORMAT,
            "schemaVersion": crate::ui_panel::VERSION,
            "title": "Nesne ara",
            "body": [
                { "type": "text", "text": "Bir özniteliğinde aranan geçen nesneleri listeler; satıra tıklamak nesneyi gösterir." },
                { "type": "textField", "id": "sorgu", "label": "Aranan", "placeholder": "ad, no …" },
                { "type": "layerPicker", "id": "katman", "label": "Katman" },
                { "type": "fieldPicker", "id": "alan", "label": "Alan", "layer": "katman" },
                { "type": "button", "id": "ara", "label": "Ara", "command": "nesne-ara", "table": "sonuc", "primary": true },
                { "type": "table", "id": "sonuc", "label": "Bulunanlar" }
            ]
        });
        files.insert("ui/nesne-ara.panel.json".to_owned(), pretty(&definition));
        python.push(PANEL.to_owned());
    }
    if !commands.is_empty() {
        entrypoints.insert("commands".into(), Value::Array(commands));
    }
    let map_tool = has(Contribution::MapTool);
    if map_tool {
        // With a schema, the tool draws an area into the schema's layer; without one, a point into the template's.
        let (id, title, capture, function) = if schema {
            ("alan-ciz", "Alan çiz", "rectangle", "alan_ciz")
        } else {
            ("isaret-koy", "İşaret koy", "point", "isaret_koy")
        };
        entrypoints.insert(
            "mapTools".into(),
            json!([{ "id": id, "title": title, "capture": capture, "handler": format!("{package}.logic:{function}") }]),
        );
        capabilities.push(json!({ "id": "mapTool", "scope": "plugin-owned",
                                  "reason": format!("Haritaya “{title}” aracını ekler.") }));
        items.push(json!({ "mapTool": id, "size": "large", "icon": "assets/eklenti.svg" }));
        python.push(if schema {
            MAP_TOOL_SCHEMA.replace("{LAYER}", &layer)
        } else {
            MAP_TOOL.to_owned()
        });
    }
    if panel {
        items.push(json!({ "panel": "nesne-paneli", "size": "large", "icon": "search" }));
    }
    // Reading and writing the drawing: one capability, the stronger when anything writes.
    if map_tool || schema {
        let what = match (map_tool, schema) {
            (true, true) => "eklentinin katmanlarını kurar ve harita aracıyla çizilen alanı yazar",
            (false, true) => "eklentinin katmanlarını kurar",
            _ => "harita aracıyla gösterilen yere işaret yazar",
        };
        capabilities.retain(|c| c["id"] != "document.read");
        capabilities.insert(
            0,
            json!({ "id": "document.write", "scope": "current-project",
                    "reason": format!("Açık çizimi okur; {what}.") }),
        );
    }
    let mut contributions = serde_json::Map::new();
    if ask.vertical && !items.is_empty() {
        capabilities.push(json!({ "id": "ui.ribbonTab", "scope": "plugin-owned",
                                  "reason": format!("{name} sekmesini açar.") }));
        contributions.insert(
            "ribbonTabs".into(),
            json!([{ "id": last, "label": name, "module": module, "panels": [{ "label": name, "items": items }] }]),
        );
    }
    // Its tool's and its expression function's code runs in MCP and the Python library too (docs/adr/8057 §70, §80);
    // a command, a map tool, a panel need the desktop.
    let runtimes = if has(Contribution::Processing) || has(Contribution::Expression) {
        json!(["desktop", "headless"])
    } else {
        json!(["desktop"])
    };
    let mut manifest = json!({
        "format": "kentos.extension",
        "schemaVersion": 1,
        "id": ask.id,
        "version": "0.1.0",
        "name": name,
        "icon": "assets/eklenti.svg",
        "summary": summary,
        "authors": [name],
        "license": "MIT",
        "licenseFile": "LICENSE",
        "changelogFile": "CHANGELOG.md",
        "kind": if ask.vertical { "vertical" } else { "utility" },
        "modules": [module],
        "runtimes": runtimes,
        "compatibility": {
            "sdkProtocol": { "min": 1, "max": 1 },
            "kentos": { "min": "0.1.0" },
            "operatingSystems": ["windows", "macos", "linux"]
        },
        "python": { "projectFile": "pyproject.toml", "lockFile": "uv.lock" },
        "entrypoints": Value::Object(entrypoints),
        "requestedCapabilities": capabilities,
    });
    if !contributions.is_empty() {
        manifest["contributions"] = Value::Object(contributions);
    }
    if schema {
        manifest["schemaFile"] = json!("schema.json");
    }
    files.insert("manifest.json".to_owned(), pretty(&manifest));
    files.insert(format!("src/{package}/__init__.py"), String::new());
    files.insert(format!("src/{package}/logic.py"), python.concat());
    // Its own tests (docs/adr/8057 §87): `kentos kek test`, Örnekleri çalıştır and an agent's trial run them with
    // Python's unittest in its environment.
    files.insert(
        "tests/test_logic.py".to_owned(),
        tests(&package, has(Contribution::Expression)),
    );
    files.insert("pyproject.toml".to_owned(), pyproject(last, name));
    files.insert("uv.lock".to_owned(), lock(last));
    files.insert("assets/eklenti.svg".to_owned(), ICON.to_owned());
    files.insert(
        "CHANGELOG.md".to_owned(),
        format!("# Değişiklikler\n\n## 0.1.0\n\n- İlk sürüm: {summary}\n"),
    );
    files.insert("LICENSE".to_owned(), mit(name));
    Ok(files)
}

fn pretty(v: &Value) -> String {
    let mut text = serde_json::to_string_pretty(v).unwrap_or_default();
    text.push('\n');
    text
}

/// The three systems the manifest names, as uv resolves for them (`compatibility.operatingSystems`).
const ENVIRONMENTS: &str = "    \"sys_platform == 'win32' and platform_machine == 'AMD64'\",\n    \"sys_platform == 'darwin' \
                            and platform_machine == 'arm64'\",\n    \"sys_platform == 'linux' and platform_machine == \
                            'x86_64'\",\n";

fn pyproject(project: &str, name: &str) -> String {
    format!(
        "# {name} eklentisinin Python tarafı. Kütüphaneleri eklentinin kilidi seçer, KentOS değil: `uv add <paket>` \
         bu dosyayı ve uv.lock'u yazar.\n[project]\nname = \"{project}\"\nversion = \"0.1.0\"\ndescription = \"{name} \
         eklentisinin işlevleri\"\nrequires-python = \">=3.12,<3.14\"\ndependencies = []\n\n[tool.uv]\n# Yeni yayımlanan \
         sürüm kilide bir hafta sonra girer: yeni kaydedilmiş bir taklit paket o arada fark edilir.\nexclude-newer = \
         \"7 days\"\nenvironments = [\n{ENVIRONMENTS}]\nrequired-environments = [\n{ENVIRONMENTS}]\n"
    )
}

/// uv's lock for a project without dependencies (uv 0.11, the three systems, a week's cooling: docs/adr/8057 §83).
fn lock(project: &str) -> String {
    let markers = "    \"platform_machine == 'AMD64' and sys_platform == 'win32'\",\n    \"platform_machine == 'arm64' and \
                   sys_platform == 'darwin'\",\n    \"platform_machine == 'x86_64' and sys_platform == 'linux'\",\n";
    format!(
        "version = 1\nrevision = 3\nrequires-python = \">=3.12, <3.14\"\nresolution-markers = [\n{markers}]\n\
         supported-markers = [\n{markers}]\nrequired-markers = [\n{markers}]\n\n[options]\nexclude-newer = \
         \"0001-01-01T00:00:00Z\" # This has no effect and is included for backwards compatibility when using \
         relative exclude-newer values.\nexclude-newer-span = \"P7D\"\n\n[[package]]\nname = \"{project}\"\n\
         version = \"0.1.0\"\nsource = {{ virtual = \".\" }}\n"
    )
}

pub(crate) const ICON: &str = "<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 20 20\" fill=\"none\" \
                    stroke=\"currentColor\" stroke-width=\"1.4\" stroke-linecap=\"round\" stroke-linejoin=\"round\">\n  \
                    <rect x=\"3\" y=\"3\" width=\"14\" height=\"14\" rx=\"2\"/>\n  <path d=\"M7 10h6M10 7v6\"/>\n</svg>\n";

fn mit(holder: &str) -> String {
    format!(
        "MIT License\n\nCopyright (c) {holder}\n\nPermission is hereby granted, free of charge, to any person obtaining \
         a copy of this software and associated documentation files (the \"Software\"), to deal in the Software \
         without restriction, including without limitation the rights to use, copy, modify, merge, publish, \
         distribute, sublicense, and/or sell copies of the Software, and to permit persons to whom the Software is \
         furnished to do so, subject to the following conditions:\n\nThe above copyright notice and this permission \
         notice shall be included in all copies or substantial portions of the Software.\n\nTHE SOFTWARE IS PROVIDED \
         \"AS IS\", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF \
         MERCHANTABILITY, FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE AUTHORS OR \
         COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT \
         OR OTHERWISE, ARISING FROM, OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE \
         SOFTWARE.\n"
    )
}

const HEADER: &str = r#""""{NAME} eklentisinin işlevleri (KentOS eklenti iskeleti).

Her işlev manifestteki yerel kimliğiyle işaretlidir: KentOS onu yalnız çağrıldığında, eklentinin kendi sürecinde ve
kendi Python ortamında çalıştırır. Örnekleri isteğinize göre değiştirin; her adımda `kentos kek verify` ile doğrulayın.
"""

from kentos import ext
"#;

const COMMAND: &str = r#"

@ext.command("katman-listesi")
def katman_listesi(ctx: ext.CommandContext) -> ext.CommandResult:
    """Nesnesi olan her katman ve nesne sayısı: çizimi okuyan bir komutun örneği."""
    if ctx.doc is None:
        return ctx.message("Açık çizim yok.")
    satirlar = []
    yigin = list(ctx.doc.layers())
    while yigin:
        dugum = yigin.pop(0)
        yigin.extend(dugum.children)
        if dugum.type != "layer":
            continue
        sayi = len(list(ctx.doc.entities(layer=dugum.id)))
        if sayi:
            satirlar.append({"katman": dugum.name, "nesne": sayi})
    return ctx.report(satirlar, title="Katman listesi", message=f"{len(satirlar)} katmanda nesne var.")
"#;

const PROCESSING: &str = r#"

def _alan(halka: list) -> float:
    """Bir halkanın alanı (shoelace), metrekare."""
    toplam = 0.0
    for (x1, y1), (x2, y2) in zip(halka, halka[1:] + halka[:1]):
        toplam += x1 * y2 - x2 * y1
    return abs(toplam) / 2


@ext.processing("alan-ozeti")
def alan_ozeti(ctx: ext.ProcessingContext, alanlar: ext.FeatureInput) -> dict:
    """Kapalı alanların tablosu ve toplamı: bir İşlemler aracının örneği (girdiler veri olarak gelir, çıktılar
    adlarıyla döner)."""
    tablo = []
    for sira, alan in enumerate(ctx.read_features(alanlar), start=1):
        sekil = alan.geometry or {}
        halka = sekil.get("coordinates", [[]])[0] if sekil.get("type") == "Polygon" else []
        tablo.append({"sıra": sira, "alan (m²)": round(_alan([tuple(p) for p in halka]), 2)})
    toplam = round(sum(s["alan (m²)"] for s in tablo), 2)
    ctx.summary(f"{len(tablo)} alan, toplam {toplam} m².")
    return {"tablo": tablo, "toplam": toplam}
"#;

const AUDIT: &str = r#"

@ext.audit("adsiz")
def adsiz(facts: ext.GisAuditFacts) -> list[ext.AuditFinding]:
    """"ad" özniteliği boş her nesne bir bilgi bulgusu; düzeltmesi bir ad yazar: bir denetim kuralının örneği
    (katmanlar veri olarak gelir, kural yazamaz)."""
    bulgular = []
    for nesne in facts.all_features():
        if nesne.attrs.get("ad"):
            continue
        bulgular.append(
            ext.AuditFinding(
                message=f"{nesne.layer} katmanında adsız bir nesne.",
                severity="info",
                feature_id=nesne.id,
                fix=ext.SemanticFix(
                    title="Ad yaz",
                    command="cad.entities.set",
                    args={"uids": [nesne.id], "operation": "attributes", "attrs": {"ad": "adsız"}},
                ),
            )
        )
    return bulgular
"#;

const EXPRESSION: &str = r#"

@ext.expr("buyuk-harf")
def buyuk_harf(metin) -> str | None:
    """Metnin Türkçe büyük harfi (i → İ, ı → I): bir ifade işlevinin örneği; etikette, stil kuralında, süzgeçte
    ext.{ID}.buyuk-harf(ad)."""
    if metin is None:
        return None
    return str(metin).replace("i", "İ").replace("ı", "I").upper()
"#;

/// The package's own tests (docs/adr/8057 §87): its code imports; with an expression function, the function's answers.
fn tests(package: &str, expression: bool) -> String {
    let mut out = format!(
        "\"\"\"Eklentinin kendi sınamaları: `kentos kek test`, Örnekleri çalıştır ve ajanın denemesi onları eklentinin \
         ortamında Python'un\nunittest'iyle çalıştırır (kod src'den). Sınamalar unittest.TestCase sınıfında yazılır.\"\"\"\n\n\
         import unittest\n\nfrom {package} import logic\n\n\nclass Mantik(unittest.TestCase):\n    def test_kod_yuklenir(self):\n        \
         self.assertTrue(hasattr(logic, \"ext\"))\n"
    );
    if expression {
        out.push_str(
            "\n    def test_buyuk_harf(self):\n        self.assertEqual(logic.buyuk_harf(\"istanbul\"), \"İSTANBUL\")\n        \
             self.assertEqual(logic.buyuk_harf(\"ılık\"), \"ILIK\")\n        self.assertIsNone(logic.buyuk_harf(None))\n",
        );
    }
    out.push_str("\n\nif __name__ == \"__main__\":\n    unittest.main()\n");
    out
}

const SCHEMA: &str = r#"

@ext.command("katmanlari-kur")
def katmanlari_kur(ctx: ext.CommandContext) -> ext.CommandResult:
    """schema.json'daki grubu, değer listesini ve katmanları açık çizime kurar: var olana dokunmaz, yalnız eksiği ekler
    (ikinci çalıştırma bir şey değiştirmez); tek geri alma adımıdır."""
    return ctx.apply_declarative_schema("schema.json")
"#;

const PANEL: &str = r#"

@ext.command("nesne-ara")
def nesne_ara(ctx: ext.CommandContext) -> ext.CommandResult:
    """"Nesne ara" panelinin düğmesi: bir özniteliğinde aranan geçen nesneleri listeler (en çok 500). Panelin değerleri
    ctx.params'tadır: sorgu, katman (seçilen katmanın kimliği; None: bütün katmanlar) ve alan (o katmanın seçilen
    alanının adı; None: bütün öznitelikler). Satırın uid'i, satıra tıklanınca nesneyi gösterir (sütun olarak
    görünmez)."""
    if ctx.doc is None:
        return ctx.message("Açık çizim yok.")
    sorgu = str(ctx.params.get("sorgu") or "").strip().casefold()
    katman, alan = ctx.params.get("katman"), ctx.params.get("alan")
    satirlar = []
    for kayit in ctx.doc.entities(layer=katman) if katman else ctx.doc.entities():
        ctx.check()
        oznitelikler = dict(getattr(kayit.entity, "attrs", None) or {})
        bakilan = [oznitelikler.get(alan, "")] if alan else list(oznitelikler.values())
        if sorgu and not any(sorgu in str(deger).casefold() for deger in bakilan):
            continue
        yazi = ", ".join(f"{ad}={deger}" for ad, deger in oznitelikler.items())
        satirlar.append({"uid": kayit.uid, "tür": kayit.entity.kind, "öznitelikler": yazi or "—"})
        if len(satirlar) == 500:
            break
    return ctx.report(satirlar, title="Nesne ara", message=f"{len(satirlar)} nesne bulundu.")
"#;

const MAP_TOOL_SCHEMA: &str = r#"

@ext.map_tool("alan-ciz")
def alan_ciz(ctx: ext.MapToolContext, sekil: ext.Shape) -> ext.CommandResult:
    """Kullanıcının çizdiği dikdörtgeni eklentinin katmanına bir alan olarak yazar (önce Katmanları kur)."""
    return ctx.add_feature("{LAYER}", sekil, {"ad": "yeni", "durum": "ETKIN"})
"#;

const MAP_TOOL: &str = r#"

@ext.map_tool("isaret-koy")
def isaret_koy(ctx: ext.MapToolContext, sekil: ext.Shape) -> ext.CommandResult:
    """Kullanıcının gösterdiği noktaya bir işaret yazar: bir harita aracının örneği (şekli KentOS çizer, kod
    yerleştirir)."""
    return ctx.add_feature("taslak", sekil, {"ad": "işaret"})
"#;

/// The blueprint's form in the package.
const FORM_FILE: &str = "forms/alan.kform";

/// The portable form of the schema's layer (`kentos.form-package` 1): its two fields, the name required, the state
/// from the schema's domain, so the form and the layer say the same thing (docs/adr/8057 §112).
fn form_package(name: &str, package: &str) -> Value {
    let values =
        json!([{ "code": "ETKIN", "label": "Etkin" }, { "code": "KAPALI", "label": "Kapalı" }]);
    let domain = format!("{package}_durum");
    json!({
        "format": "kentos.form-package",
        "version": 1,
        "name": format!("{name} alanı"),
        "body": {
            "revision": 1,
            "items": [
                { "type": "field", "id": "ad", "field": "ad", "required": true },
                { "type": "field", "id": "durum", "field": "durum" }
            ]
        },
        "fields": [
            { "name": "ad", "kind": "text" },
            { "name": "durum", "kind": "text", "values": values, "domain": domain }
        ],
        "domains": [{ "id": domain, "name": "Durum", "kind": "text", "values": values }]
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::archive::Limits;

    fn ask(vertical: bool, contributions: &[Contribution]) -> Ask {
        Ask {
            id: "tr.belediye.mezarlik-bilgi".into(),
            name: "Mezarlık bilgi".into(),
            summary: "Mezarlık adalarını ve mezar yerlerini yönetir.".into(),
            vertical,
            module: Module::Cbs,
            contributions: contributions.to_vec(),
        }
    }

    fn bytes(files: &BTreeMap<String, String>) -> BTreeMap<String, Vec<u8>> {
        files
            .iter()
            .map(|(k, v)| (k.clone(), v.as_bytes().to_vec()))
            .collect()
    }

    #[test]
    fn every_blueprint_passes_verify_without_a_problem_or_a_warning() {
        let mut asks: Vec<Ask> = Contribution::ALL
            .iter()
            .flat_map(|c| [ask(true, &[*c]), ask(false, &[*c])])
            .collect();
        asks.push(ask(true, &Contribution::ALL));
        asks.push(ask(false, &Contribution::ALL));
        for a in asks {
            let files = blueprint(&a).expect("a blueprint");
            let v = crate::verify(&bytes(&files), &Limits::default());
            assert!(
                v.problems.is_empty(),
                "{:?}: {:#?}",
                a.contributions,
                v.problems
            );
            assert!(
                v.warnings.is_empty(),
                "{:?}: {:#?}",
                a.contributions,
                v.warnings
            );
            let m = v.manifest.expect("its manifest");
            assert_eq!(m.id, "tr.belediye.mezarlik-bilgi");
            assert!(files.contains_key("src/mezarlik_bilgi/logic.py"));
        }
    }

    #[test]
    fn its_python_parses() {
        // The Python is checked by Python's own parser where a python3 is at hand (the SDK's tests run it).
        let files = blueprint(&ask(true, &Contribution::ALL)).unwrap();
        let logic = &files["src/mezarlik_bilgi/logic.py"];
        let Ok(out) = std::process::Command::new("python3")
            .args(["-I", "-c", "import ast, sys; ast.parse(sys.stdin.read())"])
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .and_then(|mut child| {
                use std::io::Write;
                child.stdin.take().unwrap().write_all(logic.as_bytes())?;
                child.wait_with_output()
            })
        else {
            return;
        };
        assert!(
            out.status.success(),
            "{}",
            String::from_utf8_lossy(&out.stderr)
        );
        assert!(logic.contains("ext.tr.belediye.mezarlik-bilgi.buyuk-harf(ad)"));
    }

    #[test]
    fn a_wrong_ask_is_said() {
        let mut a = ask(true, &[Contribution::Command]);
        a.id = "Mezarlik".into();
        assert!(blueprint(&a).unwrap_err().contains("eklenti kimliği değil"));
        let mut a = ask(true, &[]);
        a.contributions.clear();
        assert!(blueprint(&a).unwrap_err().contains("En az bir katkı"));
        assert_eq!(Contribution::parse("mapTool"), Some(Contribution::MapTool));
        assert_eq!(Contribution::parse("panel"), Some(Contribution::Panel));
        assert_eq!(Contribution::parse("dialog"), None);
    }
}
