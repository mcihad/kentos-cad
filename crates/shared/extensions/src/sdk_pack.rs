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

## İkonlar

Bir düğmenin, komutun ya da harita aracının ikonu yerleşik bir addır, tire biçiminde (`space-number`, `layer-fields`;
bütün adlar `icons` bölümünde), ya da paketteki 20×20 çizgi SVG'si (`assets/x.svg`: `path`, `rect`, `circle`,
`ellipse`; renk yok, tema rengiyle çizilir). Bilinmeyen ad uyarıdır: düğme düz simgeyle görünür.

## Bağlamın ortak yöntemleri

`ctx.progress(n, toplam, metin)`, `ctx.check()` (Durdur'a basıldıysa `ext.Stopped` atar: uzun döngülerde çağırın),
`ctx.is_stopped()`, `ctx.log(metin, level="info"|"warning"|"error")`, `ctx.report(satırlar, title=…, message=…)`,
`ctx.message(metin)`, `ctx.cancelled()`; `ctx.params` ve `ctx.settings` sözlüklerdir. Bu KentOS'un yapamadığı bir şey
`ext.NotAvailable` atar: KentOS onu eklentinin hatası değil eksik bir yetenek olarak söyler.

## Kurallar

- Her bildirilen kimliğin tek, doğru türde işaretli işlevi olur; `kentos kek verify` (ajanlarda `extension.verify`)
  her adımda bunu ve paketin kurallarını denetler.
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

/// The pack's parts an agent may ask for alone (`extension.sdk`'s `part`, `kentos kek sdk --part`).
pub const PARTS: [&str; 10] = [
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
];

/// One part of the pack; none for a name the pack has not. `icons` is the whole list, which the pack only counts: its
/// 19 KB would take the pack past what the desktop's Asistan gives a tool's result (48 KB).
pub fn sdk_part(part: &str) -> Option<Value> {
    if part == "icons" {
        return Some(json!(crate::icon_names::names().collect::<Vec<_>>()));
    }
    if !PARTS.contains(&part) {
        return None;
    }
    sdk_pack().get_mut(part).map(Value::take)
}

/// The pack, as the MCP resource and `extension.sdk` give it.
pub fn sdk_pack() -> Value {
    let example = blueprint(&Ask {
        id: "tr.ornek.ornek-eklenti".into(),
        name: "Örnek eklenti".into(),
        summary: "Her katkı türünün küçük, çalışan örneği.".into(),
        vertical: true,
        module: Module::Cbs,
        contributions: Contribution::ALL.to_vec(),
    })
    .unwrap_or_default();
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
                       keywords, parameters, outputs, feedback, capabilities.runtimes, timeoutSeconds"
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
            "manifest_icon_unknown": "Uyarı: bu KentOS'ta böyle bir yerleşik ikon yok; en yakınları önerilir."
        },
        "workflow": [
            "İsteği ve parametrelerini netleştirin; kurulu eklentileri ve yetenekleri kentos://extensions/installed'dan okuyun.",
            "extension.blueprint ile iskeleti alın, boş bir klasöre yazın.",
            "Örnekleri isteğe göre değiştirin; her adımda extension.verify.",
            "Kütüphane gerekiyorsa klasörde `uv add <paket>`.",
            "`kentos kek pack <klasör>` paketler; kurulum, yetki ve yayın kullanıcının incelemesiyle olur."
        ],
        "example": example,
        "icons": {
            "count": crate::icon_names::names().count(),
            "how": "Yerleşik ikonlar tire biçiminde yazılır (zoom-extents, space-number, layer-fields). Tamamı: \
                    extension.sdk part=icons ya da kentos kek sdk --part icons. Yoksa paketteki 20×20 çizgi .svg.",
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
        (joined.trim_end_matches(':').to_owned(), i)
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
        let example: std::collections::BTreeMap<String, Vec<u8>> = pack["example"]
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
