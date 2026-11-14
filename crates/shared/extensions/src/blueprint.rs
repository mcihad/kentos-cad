//! `extension.blueprint` (plan/eklenti.md ADIM 21; docs/adr/8057 §19): a new extension's working folder, valid by
//! construction (`verify` passes it without a problem or a warning), with one small example of each contribution
//! asked for: a command that reads the layers, an İşlemler tool with its descriptor, an audit rule with a fix, an
//! expression function and a map tool. An agent or a person grows it from there.
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
}

impl Contribution {
    /// Its name in an agent's call (`command`, `processing`, `audit`, `expression`, `mapTool`, `schema`).
    pub fn parse(name: &str) -> Option<Contribution> {
        match name {
            "command" => Some(Contribution::Command),
            "processing" => Some(Contribution::Processing),
            "audit" => Some(Contribution::Audit),
            "expression" => Some(Contribution::Expression),
            "mapTool" => Some(Contribution::MapTool),
            "schema" => Some(Contribution::Schema),
            _ => None,
        }
    }

    pub const ALL: [Contribution; 6] = [
        Contribution::Command,
        Contribution::Processing,
        Contribution::Audit,
        Contribution::Expression,
        Contribution::MapTool,
        Contribution::Schema,
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
    asked.sort();
    asked.dedup();
    if asked.is_empty() {
        return Err(
            "En az bir katkı seçin: command, processing, audit, expression, mapTool.".to_owned(),
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
            "capabilities": { "runtimes": ["desktop-python"] }
        });
        files.insert("tools/alan-ozeti.tool.json".to_owned(), pretty(&descriptor));
        python.push(PROCESSING.to_owned());
    }
    if has(Contribution::Audit) {
        entrypoints.insert(
            "audits".into(),
            json!([{ "id": "adsiz", "title": "Adsız nesneler", "domain": module, "handler": format!("{package}.logic:adsiz") }]),
        );
        capabilities.push(json!({ "id": "audit.provider", "scope": "plugin-owned",
                                  "reason": "Adı olmayan nesneleri bulur ve ad yazan bir düzeltme önerir." }));
        items.push(json!({ "audit": "adsiz", "size": "large", "icon": "assets/eklenti.svg" }));
        python.push(AUDIT.to_owned());
    }
    if has(Contribution::Expression) {
        entrypoints.insert(
            "expressions".into(),
            json!([{ "id": "buyuk-harf", "title": "Türkçe büyük harf", "handler": format!("{package}.logic:buyuk_harf") }]),
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
        files.insert("schema.json".to_owned(), pretty(&layer_schema));
        python.push(SCHEMA.to_owned());
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
        "runtimes": ["desktop"],
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
         eklentisinin işlevleri\"\nrequires-python = \">=3.12,<3.14\"\ndependencies = []\n\n[tool.uv]\nenvironments = \
         [\n{ENVIRONMENTS}]\nrequired-environments = [\n{ENVIRONMENTS}]\n"
    )
}

/// uv's lock for a project without dependencies (uv 0.11, the three systems).
fn lock(project: &str) -> String {
    let markers = "    \"platform_machine == 'AMD64' and sys_platform == 'win32'\",\n    \"platform_machine == 'arm64' and \
                   sys_platform == 'darwin'\",\n    \"platform_machine == 'x86_64' and sys_platform == 'linux'\",\n";
    format!(
        "version = 1\nrevision = 3\nrequires-python = \">=3.12, <3.14\"\nresolution-markers = [\n{markers}]\n\
         supported-markers = [\n{markers}]\nrequired-markers = [\n{markers}]\n\n[[package]]\nname = \"{project}\"\n\
         version = \"0.1.0\"\nsource = {{ virtual = \".\" }}\n"
    )
}

const ICON: &str = "<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 20 20\" fill=\"none\" \
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

const SCHEMA: &str = r#"

@ext.command("katmanlari-kur")
def katmanlari_kur(ctx: ext.CommandContext) -> ext.CommandResult:
    """schema.json'daki grubu, değer listesini ve katmanları açık çizime kurar: var olana dokunmaz, yalnız eksiği ekler
    (ikinci çalıştırma bir şey değiştirmez); tek geri alma adımıdır."""
    return ctx.apply_declarative_schema("schema.json")
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
        assert_eq!(Contribution::parse("panel"), None);
    }
}
